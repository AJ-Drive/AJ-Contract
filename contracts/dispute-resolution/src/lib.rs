#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct DisputeResolutionContract;

#[contractimpl]
impl DisputeResolutionContract {
    // TODO: implement dispute resolution logic
    pub fn hello(env: Env) -> soroban_sdk::Symbol {
        soroban_sdk::Symbol::new(&env, "dispute_resolution")
    }
}
