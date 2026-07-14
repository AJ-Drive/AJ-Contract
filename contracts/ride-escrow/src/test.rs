#![cfg(test)]

use super::*;
use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as _,
    token, Address, Env, Symbol,
};

// Stateful mock for DriverPayoutContract
#[contract]
pub struct MockDriverPayout;

#[contractimpl]
impl MockDriverPayout {
    pub fn record_payout(
        env: Env,
        _caller: Address,
        driver: Address,
        token: Address,
        amount: i128,
        ride_id: Symbol,
    ) {
        env.storage().instance().set(&Symbol::new(&env, "recorded"), &true);
        env.storage().instance().set(&Symbol::new(&env, "driver"), &driver);
        env.storage().instance().set(&Symbol::new(&env, "token"), &token);
        env.storage().instance().set(&Symbol::new(&env, "amount"), &amount);
        env.storage().instance().set(&Symbol::new(&env, "ride_id"), &ride_id);
    }
}

// Stateful mock for DisputeResolutionContract
#[contract]
pub struct MockDisputeResolution;

#[contractimpl]
impl MockDisputeResolution {
    pub fn dummy(_env: Env) {}
}

#[test]
fn test_deposit_and_release() {
    let env = Env::default();
    env.mock_all_auths();

    // Generate addresses
    let admin = Address::generate(&env);
    let passenger = Address::generate(&env);
    let driver = Address::generate(&env);

    // Register token
    let sac_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(sac_admin);
    let token_address = sac.address();
    let token_client = token::Client::new(&env, &token_address);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    // Mint tokens to passenger
    token_admin_client.mint(&passenger, &1000);
    assert_eq!(token_client.balance(&passenger), 1000);

    // Register contracts
    let escrow_id = env.register_contract(None, RideEscrowContract);
    let escrow_client = RideEscrowContractClient::new(&env, &escrow_id);

    let payout_id = env.register_contract(None, MockDriverPayout);
    let dispute_id = env.register_contract(None, MockDisputeResolution);

    // Initialize escrow contract
    escrow_client.initialize(&admin, &payout_id, &dispute_id);

    let ride_id = Symbol::new(&env, "ride_1");

    // Deposit 500 tokens into escrow
    escrow_client.deposit(&ride_id, &passenger, &driver, &token_address, &500);

    // Verify escrow contract holds 500 tokens
    assert_eq!(token_client.balance(&escrow_id), 500);
    assert_eq!(token_client.balance(&passenger), 500);

    // Verify escrow record in storage
    let escrow = escrow_client.get_escrow(&ride_id);
    assert_eq!(escrow.ride_id, ride_id);
    assert_eq!(escrow.passenger, passenger);
    assert_eq!(escrow.driver, driver);
    assert_eq!(escrow.token, token_address);
    assert_eq!(escrow.amount, 500);
    assert!(matches!(escrow.status, RideStatus::Funded));

    // Release escrow (called by driver/admin)
    escrow_client.release(&ride_id, &driver);

    // Verify funds were transferred to payout contract
    assert_eq!(token_client.balance(&escrow_id), 0);
    assert_eq!(token_client.balance(&payout_id), 500);

    // Verify payout contract recorded the payment details
    let payout_recorded: bool = env.as_contract(&payout_id, || {
        env.storage().instance().get(&Symbol::new(&env, "recorded")).unwrap_or(false)
    });
    assert!(payout_recorded);

    let recorded_driver: Address = env.as_contract(&payout_id, || {
        env.storage().instance().get(&Symbol::new(&env, "driver")).unwrap()
    });
    assert_eq!(recorded_driver, driver);

    let recorded_amount: i128 = env.as_contract(&payout_id, || {
        env.storage().instance().get(&Symbol::new(&env, "amount")).unwrap()
    });
    assert_eq!(recorded_amount, 500);

    // Check updated escrow status is Completed
    let updated_escrow = escrow_client.get_escrow(&ride_id);
    assert!(matches!(updated_escrow.status, RideStatus::Completed));
}

#[test]
fn test_deposit_and_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let passenger = Address::generate(&env);
    let driver = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token_address = sac.address();
    let token_client = token::Client::new(&env, &token_address);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    token_admin_client.mint(&passenger, &1000);

    let escrow_id = env.register_contract(None, RideEscrowContract);
    let escrow_client = RideEscrowContractClient::new(&env, &escrow_id);
    let payout_id = env.register_contract(None, MockDriverPayout);
    let dispute_id = env.register_contract(None, MockDisputeResolution);

    escrow_client.initialize(&admin, &payout_id, &dispute_id);

    let ride_id = Symbol::new(&env, "ride_2");
    escrow_client.deposit(&ride_id, &passenger, &driver, &token_address, &400);

    // Refund
    escrow_client.refund(&ride_id, &passenger);

    // Verify passenger got refunded
    assert_eq!(token_client.balance(&escrow_id), 0);
    assert_eq!(token_client.balance(&passenger), 1000);

    let escrow = escrow_client.get_escrow(&ride_id);
    assert!(matches!(escrow.status, RideStatus::Refunded));
}

#[test]
fn test_dispute_and_resolve() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let passenger = Address::generate(&env);
    let driver = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token_address = sac.address();
    let token_client = token::Client::new(&env, &token_address);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    token_admin_client.mint(&passenger, &1000);

    let escrow_id = env.register_contract(None, RideEscrowContract);
    let escrow_client = RideEscrowContractClient::new(&env, &escrow_id);
    let payout_id = env.register_contract(None, MockDriverPayout);
    let dispute_id = env.register_contract(None, MockDisputeResolution);

    escrow_client.initialize(&admin, &payout_id, &dispute_id);

    let ride_id = Symbol::new(&env, "ride_3");
    escrow_client.deposit(&ride_id, &passenger, &driver, &token_address, &600);

    // Raise dispute
    escrow_client.dispute(&ride_id, &passenger);

    let escrow = escrow_client.get_escrow(&ride_id);
    assert!(matches!(escrow.status, RideStatus::Disputed));

    // Resolve dispute with 200 to passenger and 400 to driver
    // This call must be made by the dispute contract
    env.as_contract(&dispute_id, || {
        escrow_client.resolve_dispute(&dispute_id, &ride_id, &200, &400);
    });

    // Check balances
    assert_eq!(token_client.balance(&escrow_id), 0);
    assert_eq!(token_client.balance(&passenger), 600); // 400 leftover + 200 resolved
    assert_eq!(token_client.balance(&payout_id), 400); // 400 went to payout contract

    // Check payout contract recorded the driver's portion
    let recorded_amount: i128 = env.as_contract(&payout_id, || {
        env.storage().instance().get(&Symbol::new(&env, "amount")).unwrap()
    });
    assert_eq!(recorded_amount, 400);

    let escrow = escrow_client.get_escrow(&ride_id);
    assert!(matches!(escrow.status, RideStatus::Completed));
}

#[test]
fn test_dispute_and_dismiss() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let passenger = Address::generate(&env);
    let driver = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token_address = sac.address();
    let _token_client = token::Client::new(&env, &token_address);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    token_admin_client.mint(&passenger, &1000);

    let escrow_id = env.register_contract(None, RideEscrowContract);
    let escrow_client = RideEscrowContractClient::new(&env, &escrow_id);
    let payout_id = env.register_contract(None, MockDriverPayout);
    let dispute_id = env.register_contract(None, MockDisputeResolution);

    escrow_client.initialize(&admin, &payout_id, &dispute_id);

    let ride_id = Symbol::new(&env, "ride_4");
    escrow_client.deposit(&ride_id, &passenger, &driver, &token_address, &600);

    // Raise dispute
    escrow_client.dispute(&ride_id, &passenger);

    // Dismiss dispute (called by dispute contract)
    env.as_contract(&dispute_id, || {
        escrow_client.dismiss_dispute(&dispute_id, &ride_id);
    });

    // Check escrow status goes back to Funded
    let escrow = escrow_client.get_escrow(&ride_id);
    assert!(matches!(escrow.status, RideStatus::Funded));
}
