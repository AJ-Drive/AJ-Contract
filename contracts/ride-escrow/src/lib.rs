#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, Symbol, token,
};

// ── Storage keys ────────────────────────────────────────────────────────────

const ADMIN: Symbol = symbol_short!("ADMIN");
const PAYOUT_CONTRACT: Symbol = symbol_short!("PAYOUT");
const DISPUTE_CONTRACT: Symbol = symbol_short!("DISPUTE");

// ── Client interfaces ────────────────────────────────────────────────────────

#[soroban_sdk::contractclient(name = "DriverPayoutClient")]
pub trait DriverPayout {
    fn record_payout(
        env: Env,
        caller: Address,
        driver: Address,
        token: Address,
        amount: i128,
        ride_id: Symbol,
    );
}

// ── Data types ───────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum RideStatus {
    Pending,
    Funded,
    Completed,
    Refunded,
    Disputed,
}

#[contracttype]
#[derive(Clone)]
pub struct RideEscrow {
    pub ride_id:   Symbol,
    pub passenger: Address,
    pub driver:    Address,
    pub token:     Address,   // XLM or USDC token contract
    pub amount:    i128,
    pub status:    RideStatus,
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct RideEscrowContract;

#[contractimpl]
impl RideEscrowContract {

    /// One-time initialisation — sets the platform admin address and linked contracts.
    pub fn initialize(
        env: Env,
        admin: Address,
        payout_contract: Address,
        dispute_contract: Address,
    ) {
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialized");
        }
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&PAYOUT_CONTRACT, &payout_contract);
        env.storage().instance().set(&DISPUTE_CONTRACT, &dispute_contract);
    }

    /// Passenger deposits funds into escrow when a ride is booked.
    /// The passenger must have approved this contract to spend `amount` tokens.
    pub fn deposit(
        env:       Env,
        ride_id:   Symbol,
        passenger: Address,
        driver:    Address,
        token:     Address,
        amount:    i128,
    ) {
        passenger.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }

        // Ensure no duplicate ride
        if env.storage().persistent().has(&ride_id) {
            panic!("ride already exists");
        }

        // Pull tokens from passenger into this contract
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&passenger, &env.current_contract_address(), &amount);

        let escrow = RideEscrow {
            ride_id: ride_id.clone(),
            passenger,
            driver,
            token,
            amount,
            status: RideStatus::Funded,
        };

        env.storage().persistent().set(&ride_id, &escrow);

        env.events().publish(
            (symbol_short!("deposit"), ride_id),
            amount,
        );
    }

    /// Driver calls this after completing the ride — releases funds to driver.
    /// In production the backend (or a multisig) would call this after
    /// both parties confirm completion.
    pub fn release(env: Env, ride_id: Symbol, caller: Address) {
        caller.require_auth();

        let mut escrow: RideEscrow = env
            .storage()
            .persistent()
            .get(&ride_id)
            .expect("ride not found");

        if escrow.status != RideStatus::Funded {
            panic!("ride is not in funded state");
        }

        // Only the driver or admin may release
        let admin: Address = env.storage().instance().get(&ADMIN).expect("not initialized");
        if caller != escrow.driver && caller != admin {
            panic!("unauthorized: only driver or admin can release");
        }

        let payout_contract: Address = env.storage().instance().get(&PAYOUT_CONTRACT).expect("not initialized");

        // Transfer funds to driver payout contract
        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &payout_contract,
            &escrow.amount,
        );

        // Record payout for the driver
        let payout_client = DriverPayoutClient::new(&env, &payout_contract);
        payout_client.record_payout(
            &env.current_contract_address(),
            &escrow.driver,
            &escrow.token,
            &escrow.amount,
            &ride_id,
        );

        escrow.status = RideStatus::Completed;
        env.storage().persistent().set(&ride_id, &escrow);

        env.events().publish(
            (symbol_short!("release"), ride_id),
            escrow.amount,
        );
    }

    /// Passenger or admin can refund if the ride was cancelled before starting.
    pub fn refund(env: Env, ride_id: Symbol, caller: Address) {
        caller.require_auth();

        let mut escrow: RideEscrow = env
            .storage()
            .persistent()
            .get(&ride_id)
            .expect("ride not found");

        if escrow.status != RideStatus::Funded {
            panic!("ride is not in funded state");
        }

        let admin: Address = env.storage().instance().get(&ADMIN).expect("not initialized");
        if caller != escrow.passenger && caller != admin {
            panic!("unauthorized: only passenger or admin can refund");
        }

        // Return funds to passenger
        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.passenger,
            &escrow.amount,
        );

        escrow.status = RideStatus::Refunded;
        env.storage().persistent().set(&ride_id, &escrow);

        env.events().publish(
            (symbol_short!("refund"), ride_id),
            escrow.amount,
        );
    }

    /// Mark a ride as disputed — freezes funds until admin resolves.
    pub fn dispute(env: Env, ride_id: Symbol, caller: Address) {
        caller.require_auth();

        let mut escrow: RideEscrow = env
            .storage()
            .persistent()
            .get(&ride_id)
            .expect("ride not found");

        if escrow.status != RideStatus::Funded {
            panic!("can only dispute a funded ride");
        }

        if caller != escrow.passenger && caller != escrow.driver {
            panic!("only ride participants can raise a dispute");
        }

        escrow.status = RideStatus::Disputed;
        env.storage().persistent().set(&ride_id, &escrow);

        env.events().publish(
            (symbol_short!("dispute"), ride_id),
            (),
        );
    }

    /// Read the current state of an escrow.
    pub fn get_escrow(env: Env, ride_id: Symbol) -> RideEscrow {
        env.storage()
            .persistent()
            .get(&ride_id)
            .expect("ride not found")
    }

    /// Returns the admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&ADMIN).expect("not initialized")
    }

    /// Returns the payout contract address.
    pub fn get_payout_contract(env: Env) -> Address {
        env.storage().instance().get(&PAYOUT_CONTRACT).expect("not initialized")
    }

    /// Returns the dispute contract address.
    pub fn get_dispute_contract(env: Env) -> Address {
        env.storage().instance().get(&DISPUTE_CONTRACT).expect("not initialized")
    }

    /// Resolves a dispute on an escrow by splitting the funds.
    /// Only callable by the dispute resolution contract.
    pub fn resolve_dispute(
        env: Env,
        caller: Address,
        ride_id: Symbol,
        passenger_amount: i128,
        driver_amount: i128,
    ) {
        caller.require_auth();

        let dispute_contract: Address = env.storage().instance().get(&DISPUTE_CONTRACT).expect("not initialized");
        if caller != dispute_contract {
            panic!("unauthorized: only dispute resolution contract can resolve");
        }

        let mut escrow: RideEscrow = env
            .storage()
            .persistent()
            .get(&ride_id)
            .expect("ride not found");

        if escrow.status != RideStatus::Disputed {
            panic!("escrow is not in disputed state");
        }

        if passenger_amount + driver_amount != escrow.amount {
            panic!("passenger_amount + driver_amount must equal escrowed_amount");
        }

        let token_client = token::Client::new(&env, &escrow.token);

        // 1. Pay passenger their share directly
        if passenger_amount > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &escrow.passenger,
                &passenger_amount,
            );
        }

        // 2. Pay driver their share through the payout contract
        if driver_amount > 0 {
            let payout_contract: Address = env.storage().instance().get(&PAYOUT_CONTRACT).expect("not initialized");
            
            // Transfer to payout contract
            token_client.transfer(
                &env.current_contract_address(),
                &payout_contract,
                &driver_amount,
            );

            // Record driver's payout
            let payout_client = DriverPayoutClient::new(&env, &payout_contract);
            payout_client.record_payout(
                &env.current_contract_address(),
                &escrow.driver,
                &escrow.token,
                &driver_amount,
                &ride_id,
            );
        }

        escrow.status = RideStatus::Completed;
        env.storage().persistent().set(&ride_id, &escrow);

        env.events().publish(
            (symbol_short!("resolved"), ride_id),
            (passenger_amount, driver_amount),
        );
    }

    /// Dismisses a dispute, reverting the escrow status back to Funded.
    /// Only callable by the dispute resolution contract.
    pub fn dismiss_dispute(env: Env, caller: Address, ride_id: Symbol) {
        caller.require_auth();

        let dispute_contract: Address = env.storage().instance().get(&DISPUTE_CONTRACT).expect("not initialized");
        if caller != dispute_contract {
            panic!("unauthorized: only dispute resolution contract can dismiss");
        }

        let mut escrow: RideEscrow = env
            .storage()
            .persistent()
            .get(&ride_id)
            .expect("ride not found");

        if escrow.status != RideStatus::Disputed {
            panic!("escrow is not in disputed state");
        }

        escrow.status = RideStatus::Funded;
        env.storage().persistent().set(&ride_id, &escrow);

        env.events().publish(
            (symbol_short!("dismissed"), ride_id),
            (),
        );
    }
}

#[cfg(test)]
mod test;
