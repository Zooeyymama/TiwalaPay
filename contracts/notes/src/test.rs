#[cfg(test)]
mod tests {
    use soroban_sdk::{
        testutils::Address as _,
        token, Address, Env,
    };
    use crate::{SafeHatidContract, SafeHatidContractClient, EscrowStatus};

    fn setup() -> (Env, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();

        let token_client = token::StellarAssetClient::new(&env, &token_id);

        let buyer = Address::generate(&env);
        let seller = Address::generate(&env);

        // Mint 1000 USDC to the buyer
        token_client.mint(&buyer, &1000);

        (env, buyer, seller, token_id, token_admin)
    }

    /// Test 1 (Happy path): full lock → ship → confirm → release flow
    #[test]
    fn test_full_happy_path() {
        let (env, buyer, seller, token_id, _) = setup();
        let contract_id = env.register_contract(None, SafeHatidContract);
        let client = SafeHatidContractClient::new(&env, &contract_id);
        let token_client = token::Client::new(&env, &token_id);

        let order_id = client.lock_funds(&buyer, &seller, &500, &token_id);
        assert_eq!(token_client.balance(&contract_id), 500);
        assert_eq!(token_client.balance(&buyer), 500);

        client.confirm_shipment(&seller, &order_id);
        client.confirm_delivery(&buyer, &order_id);

        assert_eq!(token_client.balance(&seller), 500);
        assert_eq!(token_client.balance(&contract_id), 0);

        let order = client.get_order(&order_id);
        assert_eq!(order.status, EscrowStatus::Released);
    }

    /// Test 2 (Edge case): second confirm_delivery panics — already Released
    #[test]
    #[should_panic]
    fn test_double_release_rejected() {
        let (env, buyer, seller, token_id, _) = setup();
        let contract_id = env.register_contract(None, SafeHatidContract);
        let client = SafeHatidContractClient::new(&env, &contract_id);

        let order_id = client.lock_funds(&buyer, &seller, &500, &token_id);
        client.confirm_delivery(&buyer, &order_id);
        client.confirm_delivery(&buyer, &order_id); // should panic here
    }

    /// Test 3 (State verification): storage reflects correct state after lock
    #[test]
    fn test_storage_state_after_lock() {
        let (env, buyer, seller, token_id, _) = setup();
        let contract_id = env.register_contract(None, SafeHatidContract);
        let client = SafeHatidContractClient::new(&env, &contract_id);

        let order_id = client.lock_funds(&buyer, &seller, &300, &token_id);
        let order = client.get_order(&order_id);

        assert_eq!(order.buyer, buyer);
        assert_eq!(order.seller, seller);
        assert_eq!(order.amount, 300);
        assert_eq!(order.token, token_id);
        assert_eq!(order.status, EscrowStatus::Locked);
    }
}