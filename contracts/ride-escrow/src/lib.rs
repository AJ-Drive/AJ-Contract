#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, Symbol, token,
};

// ── Storage keys ────────────────────────────────────────────────────────────

const ADMIN: Symbol = symbol_short!("ADMIN");

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

    /// One-time initialisation — sets the platform admin address.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialized");
        }
        env.storage().instance().set(&ADMIN, &admin);
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

        // Transfer funds to driver
        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.driver,
            &escrow.amount,
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
}
