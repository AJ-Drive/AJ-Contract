#![cfg(test)]

use super::*;
use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as _,
    token, Address, Env, Symbol,
};

#[contract]
pub struct DummyEscrow;

#[contractimpl]
impl DummyEscrow {
    pub fn dummy(_env: Env) {}
}

#[test]
fn test_record_payout_from_escrow_and_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let escrow = env.register_contract(None, DummyEscrow);
    let driver = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token_address = sac.address();
    let token_client = token::Client::new(&env, &token_address);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let payout_id = env.register_contract(None, DriverPayoutContract);
    let payout_client = DriverPayoutContractClient::new(&env, &payout_id);

    payout_client.initialize(&admin, &escrow);

    // Verify initialization
    assert_eq!(payout_client.get_escrow_contract(), escrow);

    // Escrow completes ride, transfers 500 tokens to payout contract
    // (Simulating RideEscrow behavior)
    token_admin_client.mint(&payout_id, &500);
    assert_eq!(token_client.balance(&payout_id), 500);

    let ride_id = Symbol::new(&env, "ride_1");

    // Escrow contract calls record_payout
    env.as_contract(&escrow, || {
        payout_client.record_payout(&escrow, &driver, &token_address, &500, &ride_id);
    });

    // Check earnings summary
    let earnings = payout_client.get_earnings(&driver);
    assert_eq!(earnings.total_earned, 500);
    assert_eq!(earnings.total_trips, 1);
    assert_eq!(earnings.pending, 500);

    // Driver withdraws funds
    let withdrawn = payout_client.withdraw(&driver, &token_address);
    assert_eq!(withdrawn, 500);

    // Verify token balance of driver and contract
    assert_eq!(token_client.balance(&driver), 500);
    assert_eq!(token_client.balance(&payout_id), 0);

    // Verify pending balance is reset
    let updated_earnings = payout_client.get_earnings(&driver);
    assert_eq!(updated_earnings.pending, 0);
}

#[test]
fn test_record_payout_from_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let escrow = Address::generate(&env);
    let driver = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token_address = sac.address();
    let token_client = token::Client::new(&env, &token_address);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let payout_id = env.register_contract(None, DriverPayoutContract);
    let payout_client = DriverPayoutContractClient::new(&env, &payout_id);

    payout_client.initialize(&admin, &escrow);

    // Admin has 1000 tokens
    token_admin_client.mint(&admin, &1000);

    let ride_id = Symbol::new(&env, "ride_2");

    // Admin records payout (which should pull funds from admin)
    payout_client.record_payout(&admin, &driver, &token_address, &300, &ride_id);

    // Verify token balance
    assert_eq!(token_client.balance(&admin), 700);
    assert_eq!(token_client.balance(&payout_id), 300);

    // Check earnings
    let earnings = payout_client.get_earnings(&driver);
    assert_eq!(earnings.pending, 300);
}

#[test]
fn test_instant_payout() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let escrow = Address::generate(&env);
    let driver = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token_address = sac.address();
    let token_client = token::Client::new(&env, &token_address);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let payout_id = env.register_contract(None, DriverPayoutContract);
    let payout_client = DriverPayoutContractClient::new(&env, &payout_id);

    payout_client.initialize(&admin, &escrow);

    // Admin has 1000 tokens
    token_admin_client.mint(&admin, &1000);

    // Admin performs instant payout of 400 tokens (pulls from admin directly to driver)
    payout_client.instant_payout(&admin, &driver, &token_address, &400);

    // Verify balances
    assert_eq!(token_client.balance(&admin), 600);
    assert_eq!(token_client.balance(&driver), 400);
    assert_eq!(token_client.balance(&payout_id), 0);

    // Earnings summary total earned should increase, but pending remains 0
    let earnings = payout_client.get_earnings(&driver);
    assert_eq!(earnings.total_earned, 400);
    assert_eq!(earnings.pending, 0);
}
