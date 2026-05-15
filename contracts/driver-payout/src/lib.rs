#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, Symbol, token,
};

// ── Storage keys ─────────────────────────────────────────────────────────────

const ADMIN: Symbol = symbol_short!("ADMIN");

// ── Data types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct PayoutRecord {
    pub driver:    Address,
    pub token:     Address,
    pub amount:    i128,
    pub ride_id:   Symbol,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct DriverEarnings {
    pub driver:        Address,
    pub total_earned:  i128,
    pub total_trips:   u32,
    pub pending:       i128,   // accumulated but not yet withdrawn
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct DriverPayoutContract;

#[contractimpl]
impl DriverPayoutContract {

    /// One-time initialisation.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialized");
        }
        env.storage().instance().set(&ADMIN, &admin);
    }

    /// Called by the escrow contract (or admin) after a ride completes.
    /// Adds the fare to the driver's pending balance.
    pub fn record_payout(
        env:     Env,
        caller:  Address,
        driver:  Address,
        token:   Address,
        amount:  i128,
        ride_id: Symbol,
    ) {
        caller.require_auth();

        let admin: Address = env.storage().instance().get(&ADMIN).expect("not initialized");
        if caller != admin {
            panic!("only admin can record payouts");
        }

        if amount <= 0 {
            panic!("amount must be positive");
        }

        // Update driver earnings record
        let mut earnings = Self::_get_or_create_earnings(&env, &driver);
        earnings.total_earned += amount;
        earnings.total_trips  += 1;
        earnings.pending      += amount;
        env.storage().persistent().set(&driver, &earnings);

        // Store payout record keyed by ride_id
        let record = PayoutRecord {
            driver:    driver.clone(),
            token,
            amount,
            ride_id:   ride_id.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&ride_id, &record);

        env.events().publish(
            (symbol_short!("recorded"), ride_id),
            amount,
        );
    }

    /// Driver withdraws their accumulated pending balance.
    pub fn withdraw(
        env:    Env,
        driver: Address,
        token:  Address,
    ) -> i128 {
        driver.require_auth();

        let mut earnings = Self::_get_or_create_earnings(&env, &driver);

        if earnings.pending <= 0 {
            panic!("no pending balance to withdraw");
        }

        let amount = earnings.pending;
        earnings.pending = 0;
        env.storage().persistent().set(&driver, &earnings);

        // Transfer from contract to driver
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(
            &env.current_contract_address(),
            &driver,
            &amount,
        );

        env.events().publish(
            (symbol_short!("withdraw"), driver),
            amount,
        );

        amount
    }

    /// Admin can do an immediate direct payout (bypasses pending accumulation).
    pub fn instant_payout(
        env:    Env,
        admin:  Address,
        driver: Address,
        token:  Address,
        amount: i128,
    ) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&ADMIN).expect("not initialized");
        if admin != stored_admin {
            panic!("only admin can do instant payouts");
        }

        if amount <= 0 {
            panic!("amount must be positive");
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(
            &env.current_contract_address(),
            &driver,
            &amount,
        );

        // Update earnings record
        let mut earnings = Self::_get_or_create_earnings(&env, &driver);
        earnings.total_earned += amount;
        earnings.total_trips  += 1;
        env.storage().persistent().set(&driver, &earnings);

        env.events().publish(
            (symbol_short!("instant"), driver),
            amount,
        );
    }

    /// Get a driver's earnings summary.
    pub fn get_earnings(env: Env, driver: Address) -> DriverEarnings {
        Self::_get_or_create_earnings(&env, &driver)
    }

    /// Get a specific payout record by ride_id.
    pub fn get_payout_record(env: Env, ride_id: Symbol) -> PayoutRecord {
        env.storage()
            .persistent()
            .get(&ride_id)
            .expect("payout record not found")
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn _get_or_create_earnings(env: &Env, driver: &Address) -> DriverEarnings {
        env.storage()
            .persistent()
            .get(driver)
            .unwrap_or(DriverEarnings {
                driver:       driver.clone(),
                total_earned: 0,
                total_trips:  0,
                pending:      0,
            })
    }
}
