#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String, Symbol,
};

// ── Storage keys ─────────────────────────────────────────────────────────────

const ADMIN: Symbol = symbol_short!("ADMIN");
const ESCROW_CONTRACT: Symbol = symbol_short!("ESCROW");

// ── Client interfaces ────────────────────────────────────────────────────────

#[soroban_sdk::contracttype]
#[derive(Clone, PartialEq)]
pub enum RideStatus {
    Pending,
    Funded,
    Completed,
    Refunded,
    Disputed,
}

#[soroban_sdk::contracttype]
#[derive(Clone)]
pub struct RideEscrow {
    pub ride_id:   Symbol,
    pub passenger: Address,
    pub driver:    Address,
    pub token:     Address,
    pub amount:    i128,
    pub status:    RideStatus,
}

#[soroban_sdk::contractclient(name = "RideEscrowClient")]
pub trait RideEscrowContract {
    fn get_escrow(env: Env, ride_id: Symbol) -> RideEscrow;
    fn resolve_dispute(
        env: Env,
        caller: Address,
        ride_id: Symbol,
        passenger_amount: i128,
        driver_amount: i128,
    );
    fn dismiss_dispute(env: Env, caller: Address, ride_id: Symbol);
}

// ── Data types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum DisputeStatus {
    Open,
    ResolvedForPassenger,
    ResolvedForDriver,
    SplitResolution,
    Dismissed,
}

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum DisputeReason {
    DriverNoShow,
    PassengerNoShow,
    RouteDeviation,
    SafetyConcern,
    PaymentIssue,
    Other,
}

#[contracttype]
#[derive(Clone)]
pub struct Dispute {
    pub dispute_id:        Symbol,
    pub ride_id:           Symbol,
    pub passenger:         Address,
    pub driver:            Address,
    pub token:             Address,
    pub escrowed_amount:   i128,
    pub reason:            DisputeReason,
    pub description:       String,
    pub status:            DisputeStatus,
    pub raised_by:         Address,
    pub raised_at:         u64,
    pub resolved_at:       u64,
    pub passenger_amount:  i128,   // how much passenger gets on resolution
    pub driver_amount:     i128,   // how much driver gets on resolution
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct DisputeResolutionContract;

#[contractimpl]
impl DisputeResolutionContract {

    /// One-time initialisation.
    pub fn initialize(env: Env, admin: Address, escrow_contract: Address) {
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialized");
        }
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&ESCROW_CONTRACT, &escrow_contract);
    }

    /// Either the passenger or driver raises a dispute.
    /// The escrowed funds must already be held by the escrow contract;
    /// this contract records the dispute and the admin resolves it.
    pub fn raise_dispute(
        env:             Env,
        dispute_id:      Symbol,
        ride_id:         Symbol,
        raised_by:       Address,
        reason:          DisputeReason,
        description:     String,
    ) {
        raised_by.require_auth();

        if env.storage().persistent().has(&dispute_id) {
            panic!("dispute already exists");
        }

        // Fetch escrow details from RideEscrowContract
        let escrow_contract: Address = env.storage().instance().get(&ESCROW_CONTRACT).expect("not initialized");
        let escrow_client = RideEscrowClient::new(&env, &escrow_contract);
        let escrow = escrow_client.get_escrow(&ride_id);

        if raised_by != escrow.passenger && raised_by != escrow.driver {
            panic!("only ride participants can raise a dispute");
        }

        if escrow.status != RideStatus::Disputed {
            panic!("associated escrow is not in disputed state");
        }

        let dispute = Dispute {
            dispute_id:       dispute_id.clone(),
            ride_id,
            passenger:        escrow.passenger,
            driver:           escrow.driver,
            token:            escrow.token,
            escrowed_amount:  escrow.amount,
            reason,
            description,
            status:           DisputeStatus::Open,
            raised_by,
            raised_at:        env.ledger().timestamp(),
            resolved_at:      0,
            passenger_amount: 0,
            driver_amount:    0,
        };

        env.storage().persistent().set(&dispute_id, &dispute);

        env.events().publish(
            (symbol_short!("raised"), dispute_id),
            escrow.amount,
        );
    }

    /// Admin resolves the dispute by specifying how to split the escrowed funds.
    /// passenger_amount + driver_amount must equal escrowed_amount.
    pub fn resolve(
        env:              Env,
        admin:            Address,
        dispute_id:       Symbol,
        passenger_amount: i128,
        driver_amount:    i128,
    ) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&ADMIN).expect("not initialized");
        if admin != stored_admin {
            panic!("only admin can resolve disputes");
        }

        let mut dispute: Dispute = env
            .storage()
            .persistent()
            .get(&dispute_id)
            .expect("dispute not found");

        if dispute.status != DisputeStatus::Open {
            panic!("dispute is not open");
        }

        if passenger_amount + driver_amount != dispute.escrowed_amount {
            panic!("passenger_amount + driver_amount must equal escrowed_amount");
        }

        if passenger_amount < 0 || driver_amount < 0 {
            panic!("amounts cannot be negative");
        }

        // Call resolve_dispute on the RideEscrowContract
        let escrow_contract: Address = env.storage().instance().get(&ESCROW_CONTRACT).expect("not initialized");
        let escrow_client = RideEscrowClient::new(&env, &escrow_contract);
        escrow_client.resolve_dispute(
            &env.current_contract_address(),
            &dispute.ride_id,
            &passenger_amount,
            &driver_amount,
        );

        // Determine resolution type
        dispute.status = if driver_amount == 0 {
            DisputeStatus::ResolvedForPassenger
        } else if passenger_amount == 0 {
            DisputeStatus::ResolvedForDriver
        } else {
            DisputeStatus::SplitResolution
        };

        dispute.passenger_amount = passenger_amount;
        dispute.driver_amount    = driver_amount;
        dispute.resolved_at      = env.ledger().timestamp();

        env.storage().persistent().set(&dispute_id, &dispute);

        env.events().publish(
            (symbol_short!("resolved"), dispute_id),
            (passenger_amount, driver_amount),
        );
    }

    /// Admin can dismiss a dispute (e.g. duplicate or bad-faith filing).
    /// Funds are returned to the escrow contract — no token movement here.
    pub fn dismiss(env: Env, admin: Address, dispute_id: Symbol) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&ADMIN).expect("not initialized");
        if admin != stored_admin {
            panic!("only admin can dismiss disputes");
        }

        let mut dispute: Dispute = env
            .storage()
            .persistent()
            .get(&dispute_id)
            .expect("dispute not found");

        if dispute.status != DisputeStatus::Open {
            panic!("dispute is not open");
        }

        // Revert escrow status back to Funded
        let escrow_contract: Address = env.storage().instance().get(&ESCROW_CONTRACT).expect("not initialized");
        let escrow_client = RideEscrowClient::new(&env, &escrow_contract);
        escrow_client.dismiss_dispute(&env.current_contract_address(), &dispute.ride_id);

        dispute.status      = DisputeStatus::Dismissed;
        dispute.resolved_at = env.ledger().timestamp();

        env.storage().persistent().set(&dispute_id, &dispute);

        env.events().publish(
            (symbol_short!("dismissed"), dispute_id),
            (),
        );
    }

    /// Read a dispute record.
    pub fn get_dispute(env: Env, dispute_id: Symbol) -> Dispute {
        env.storage()
            .persistent()
            .get(&dispute_id)
            .expect("dispute not found")
    }

    /// Returns the admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&ADMIN).expect("not initialized")
    }

    /// Returns the escrow contract address.
    pub fn get_escrow_contract(env: Env) -> Address {
        env.storage().instance().get(&ESCROW_CONTRACT).expect("not initialized")
    }
}

#[cfg(test)]
mod test;
