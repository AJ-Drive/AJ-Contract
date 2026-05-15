#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String, Symbol, token,
};

// ── Storage keys ─────────────────────────────────────────────────────────────

const ADMIN: Symbol = symbol_short!("ADMIN");

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
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialized");
        }
        env.storage().instance().set(&ADMIN, &admin);
    }

    /// Either the passenger or driver raises a dispute.
    /// The escrowed funds must already be held by the escrow contract;
    /// this contract records the dispute and the admin resolves it.
    pub fn raise_dispute(
        env:             Env,
        dispute_id:      Symbol,
        ride_id:         Symbol,
        raised_by:       Address,
        passenger:       Address,
        driver:          Address,
        token:           Address,
        escrowed_amount: i128,
        reason:          DisputeReason,
        description:     String,
    ) {
        raised_by.require_auth();

        if raised_by != passenger && raised_by != driver {
            panic!("only ride participants can raise a dispute");
        }

        if env.storage().persistent().has(&dispute_id) {
            panic!("dispute already exists");
        }

        if escrowed_amount <= 0 {
            panic!("escrowed amount must be positive");
        }

        let dispute = Dispute {
            dispute_id:       dispute_id.clone(),
            ride_id,
            passenger,
            driver,
            token,
            escrowed_amount,
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
            escrowed_amount,
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

        let token_client = token::Client::new(&env, &dispute.token);

        // Pay passenger their share
        if passenger_amount > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &dispute.passenger,
                &passenger_amount,
            );
        }

        // Pay driver their share
        if driver_amount > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &dispute.driver,
                &driver_amount,
            );
        }

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
}
