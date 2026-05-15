#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct DriverPayoutContract;

#[contractimpl]
impl DriverPayoutContract {
    // TODO: implement driver payout logic
    pub fn hello(env: Env) -> soroban_sdk::Symbol {
        soroban_sdk::Symbol::new(&env, "driver_payout")
    }
}
