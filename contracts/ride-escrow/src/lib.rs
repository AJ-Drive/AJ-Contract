#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct RideEscrowContract;

#[contractimpl]
impl RideEscrowContract {
    // TODO: implement escrow logic
    pub fn hello(env: Env) -> soroban_sdk::Symbol {
        soroban_sdk::Symbol::new(&env, "ride_escrow")
    }
}
