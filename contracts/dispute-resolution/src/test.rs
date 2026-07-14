#![cfg(test)]

use super::*;
use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as _,
    Address, Env, String, Symbol,
};

// Stateful mock for RideEscrowContract
#[contract]
pub struct MockRideEscrow;

#[contractimpl]
impl MockRideEscrow {
    pub fn setup_mock(
        env: Env,
        passenger: Address,
        driver: Address,
        token: Address,
        amount: i128,
        status: RideStatus,
    ) {
        env.storage().instance().set(&Symbol::new(&env, "passenger"), &passenger);
        env.storage().instance().set(&Symbol::new(&env, "driver"), &driver);
        env.storage().instance().set(&Symbol::new(&env, "token"), &token);
        env.storage().instance().set(&Symbol::new(&env, "amount"), &amount);
        env.storage().instance().set(&Symbol::new(&env, "status"), &status);
    }

    pub fn get_escrow(env: Env, ride_id: Symbol) -> RideEscrow {
        let passenger = env.storage().instance().get(&Symbol::new(&env, "passenger")).unwrap();
        let driver = env.storage().instance().get(&Symbol::new(&env, "driver")).unwrap();
        let token = env.storage().instance().get(&Symbol::new(&env, "token")).unwrap();
        let amount = env.storage().instance().get(&Symbol::new(&env, "amount")).unwrap();
        let status = env.storage().instance().get(&Symbol::new(&env, "status")).unwrap();
        RideEscrow {
            ride_id,
            passenger,
            driver,
            token,
            amount,
            status,
        }
    }

    pub fn resolve_dispute(
        env: Env,
        _caller: Address,
        _ride_id: Symbol,
        passenger_amount: i128,
        driver_amount: i128,
    ) {
        env.storage().instance().set(&Symbol::new(&env, "resolved"), &true);
        env.storage().instance().set(&Symbol::new(&env, "passenger_amount"), &passenger_amount);
        env.storage().instance().set(&Symbol::new(&env, "driver_amount"), &driver_amount);
    }

    pub fn dismiss_dispute(env: Env, _caller: Address, _ride_id: Symbol) {
        env.storage().instance().set(&Symbol::new(&env, "dismissed"), &true);
    }
}

#[test]
fn test_raise_and_resolve_dispute() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let passenger = Address::generate(&env);
    let driver = Address::generate(&env);
    let token = Address::generate(&env);

    let escrow_id = env.register_contract(None, MockRideEscrow);
    let escrow_client = MockRideEscrowClient::new(&env, &escrow_id);

    // Setup mock escrow parameters with Disputed status
    escrow_client.setup_mock(&passenger, &driver, &token, &1000, &RideStatus::Disputed);

    let dispute_id = env.register_contract(None, DisputeResolutionContract);
    let dispute_client = DisputeResolutionContractClient::new(&env, &dispute_id);

    dispute_client.initialize(&admin, &escrow_id);

    // Verify initialization
    assert_eq!(dispute_client.get_escrow_contract(), escrow_id);

    let dispute_key = Symbol::new(&env, "disp_1");
    let ride_key = Symbol::new(&env, "ride_1");

    // Passenger raises dispute
    dispute_client.raise_dispute(
        &dispute_key,
        &ride_key,
        &passenger,
        &DisputeReason::DriverNoShow,
        &String::from_str(&env, "Driver didn't show up"),
    );

    // Check dispute state
    let dispute = dispute_client.get_dispute(&dispute_key);
    assert_eq!(dispute.ride_id, ride_key);
    assert_eq!(dispute.passenger, passenger);
    assert_eq!(dispute.driver, driver);
    assert_eq!(dispute.escrowed_amount, 1000);
    assert!(matches!(dispute.status, DisputeStatus::Open));

    // Admin resolves dispute (split 400 to passenger and 600 to driver)
    dispute_client.resolve(&admin, &dispute_key, &400, &600);

    // Verify dispute state updated
    let dispute = dispute_client.get_dispute(&dispute_key);
    assert!(matches!(dispute.status, DisputeStatus::SplitResolution));
    assert_eq!(dispute.passenger_amount, 400);
    assert_eq!(dispute.driver_amount, 600);

    // Verify resolve_dispute was called on MockRideEscrow with correct params
    let resolved: bool = env.as_contract(&escrow_id, || {
        env.storage().instance().get(&Symbol::new(&env, "resolved")).unwrap_or(false)
    });
    assert!(resolved);

    let passenger_amount: i128 = env.as_contract(&escrow_id, || {
        env.storage().instance().get(&Symbol::new(&env, "passenger_amount")).unwrap()
    });
    assert_eq!(passenger_amount, 400);

    let driver_amount: i128 = env.as_contract(&escrow_id, || {
        env.storage().instance().get(&Symbol::new(&env, "driver_amount")).unwrap()
    });
    assert_eq!(driver_amount, 600);
}

#[test]
fn test_raise_and_dismiss_dispute() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let passenger = Address::generate(&env);
    let driver = Address::generate(&env);
    let token = Address::generate(&env);

    let escrow_id = env.register_contract(None, MockRideEscrow);
    let escrow_client = MockRideEscrowClient::new(&env, &escrow_id);

    // Setup mock escrow parameters with Disputed status
    escrow_client.setup_mock(&passenger, &driver, &token, &1000, &RideStatus::Disputed);

    let dispute_id = env.register_contract(None, DisputeResolutionContract);
    let dispute_client = DisputeResolutionContractClient::new(&env, &dispute_id);

    dispute_client.initialize(&admin, &escrow_id);

    let dispute_key = Symbol::new(&env, "disp_2");
    let ride_key = Symbol::new(&env, "ride_2");

    // Passenger raises dispute
    dispute_client.raise_dispute(
        &dispute_key,
        &ride_key,
        &passenger,
        &DisputeReason::SafetyConcern,
        &String::from_str(&env, "Safety issue raised"),
    );

    // Admin dismisses dispute
    dispute_client.dismiss(&admin, &dispute_key);

    // Verify dispute state is Dismissed
    let dispute = dispute_client.get_dispute(&dispute_key);
    assert!(matches!(dispute.status, DisputeStatus::Dismissed));

    // Verify dismiss_dispute was called on MockRideEscrow
    let dismissed: bool = env.as_contract(&escrow_id, || {
        env.storage().instance().get(&Symbol::new(&env, "dismissed")).unwrap_or(false)
    });
    assert!(dismissed);
}
