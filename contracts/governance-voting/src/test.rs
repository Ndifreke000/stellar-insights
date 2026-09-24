#[cfg(test)]
mod tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env, String,
    };

    use crate::{GovernanceVotingContract, GovernanceVotingContractClient, ProposalStatus, VoteChoice};

    fn setup() -> (
        Env,
        GovernanceVotingContractClient<'static>,
        Address,
        token::StellarAssetClient<'static>,
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_addr = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        let token_client = token::StellarAssetClient::new(&env, &token_addr);

        let contract_id = env.register(GovernanceVotingContract, ());
        let client = GovernanceVotingContractClient::new(&env, &contract_id);

        // quorum = 10 weight units, voting_period = 3600 seconds
        client.initialize(&admin, &10, &3600, &token_addr);

        (env, client, admin, token_client)
    }

    fn register_voter_with_balance(
        client: &GovernanceVotingContractClient,
        token_client: &token::StellarAssetClient,
        admin: &Address,
        voter: &Address,
        balance: i128,
    ) {
        token_client.mint(voter, &balance);
        client.register_voter(admin, voter);
    }

    fn advance_time(env: &Env, seconds: u64) {
        env.ledger().with_mut(|li| {
            li.timestamp += seconds;
        });
    }

    #[test]
    fn test_initialize() {
        let (_, client, admin, _) = setup();
        assert_eq!(client.get_admin(), admin);
        let (cfg_admin, quorum, period, count) = client.get_config();
        assert_eq!(cfg_admin, admin);
        assert_eq!(quorum, 10);
        assert_eq!(period, 3600);
        assert_eq!(count, 0);
    }

    #[test]
    #[should_panic]
    fn test_double_initialize_fails() {
        let (_, client, admin, _) = setup();
        let token = Address::generate(&client.env);
        client.initialize(&admin, &10, &3600, &token);
    }

    #[test]
    fn test_register_voter() {
        let (_, client, admin, token_client) = setup();
        let voter = Address::generate(&client.env);

        assert_eq!(client.get_voter_weight(&voter), 0);
        register_voter_with_balance(&client, &token_client, &admin, &voter, 5);
        assert_eq!(client.get_voter_weight(&voter), 5);
    }

    #[test]
    fn test_deregister_voter() {
        let (_, client, admin, token_client) = setup();
        let voter = Address::generate(&client.env);

        register_voter_with_balance(&client, &token_client, &admin, &voter, 5);
        assert_eq!(client.get_voter_weight(&voter), 5);
        client.deregister_voter(&admin, &voter);
        assert_eq!(client.get_voter_weight(&voter), 0);
    }

    #[test]
    fn test_create_proposal_as_admin() {
        let (_, client, admin, _) = setup();
        let title = String::from_str(&client.env, "Test Proposal");
        let desc = String::from_str(&client.env, "A test proposal description");

        let id = client.create_proposal(&admin, &title, &desc);
        assert_eq!(id, 1);

        let proposal = client.get_proposal(&id);
        assert_eq!(proposal.proposer, admin);
        assert_eq!(proposal.status, ProposalStatus::Active);
    }

    #[test]
    fn test_create_proposal_as_voter() {
        let (_, client, admin, token_client) = setup();
        let voter = Address::generate(&client.env);
        register_voter_with_balance(&client, &token_client, &admin, &voter, 5);

        let title = String::from_str(&client.env, "Voter Proposal");
        let desc = String::from_str(&client.env, "Proposed by a voter");
        let id = client.create_proposal(&voter, &title, &desc);
        assert_eq!(id, 1);
    }

    #[test]
    #[should_panic]
    fn test_unregistered_cannot_create_proposal() {
        let (_, client, _, _) = setup();
        let stranger = Address::generate(&client.env);
        let title = String::from_str(&client.env, "Unauthorized");
        let desc = String::from_str(&client.env, "Should fail");
        client.create_proposal(&stranger, &title, &desc);
    }

    #[test]
    fn test_vote_and_finalize_passed() {
        let (env, client, admin, token_client) = setup();

        // Register voters with weights totaling > quorum (10)
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        register_voter_with_balance(&client, &token_client, &admin, &alice, 7);
        register_voter_with_balance(&client, &token_client, &admin, &bob, 5);

        let title = String::from_str(&env, "Upgrade Protocol");
        let desc = String::from_str(&env, "Upgrade to v2");
        let id = client.create_proposal(&admin, &title, &desc);

        client.vote(&alice, &id, &VoteChoice::For);
        client.vote(&bob, &id, &VoteChoice::For);

        // Advance past voting period
        advance_time(&env, 3601);

        let status = client.finalize_proposal(&id);
        assert_eq!(status, ProposalStatus::Passed);

        let proposal = client.get_proposal(&id);
        assert_eq!(proposal.votes_for, 12);
        assert_eq!(proposal.voter_count, 2);
    }

    #[test]
    fn test_vote_and_finalize_failed_quorum() {
        let (env, client, admin, token_client) = setup();

        let voter = Address::generate(&env);
        register_voter_with_balance(&client, &token_client, &admin, &voter, 3); // weight 3 < quorum 10

        let title = String::from_str(&env, "Low Quorum");
        let desc = String::from_str(&env, "Won't reach quorum");
        let id = client.create_proposal(&admin, &title, &desc);

        client.vote(&voter, &id, &VoteChoice::For);

        advance_time(&env, 3601);

        let status = client.finalize_proposal(&id);
        assert_eq!(status, ProposalStatus::Failed);
    }

    #[test]
    fn test_vote_and_finalize_failed_more_against() {
        let (env, client, admin, token_client) = setup();

        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        register_voter_with_balance(&client, &token_client, &admin, &alice, 6);
        register_voter_with_balance(&client, &token_client, &admin, &bob, 8);

        let title = String::from_str(&env, "Controversial");
        let desc = String::from_str(&env, "More against than for");
        let id = client.create_proposal(&admin, &title, &desc);

        client.vote(&alice, &id, &VoteChoice::For);
        client.vote(&bob, &id, &VoteChoice::Against);

        advance_time(&env, 3601);

        let status = client.finalize_proposal(&id);
        assert_eq!(status, ProposalStatus::Failed);
    }

    #[test]
    #[should_panic]
    fn test_double_vote_fails() {
        let (env, client, admin, token_client) = setup();
        let voter = Address::generate(&env);
        register_voter_with_balance(&client, &token_client, &admin, &voter, 5);

        let title = String::from_str(&env, "Proposal");
        let desc = String::from_str(&env, "Desc");
        let id = client.create_proposal(&admin, &title, &desc);

        client.vote(&voter, &id, &VoteChoice::For);
        client.vote(&voter, &id, &VoteChoice::Against); // should panic
    }

    #[test]
    #[should_panic]
    fn test_vote_after_deadline_fails() {
        let (env, client, admin, token_client) = setup();
        let voter = Address::generate(&env);
        register_voter_with_balance(&client, &token_client, &admin, &voter, 5);

        let title = String::from_str(&env, "Proposal");
        let desc = String::from_str(&env, "Desc");
        let id = client.create_proposal(&admin, &title, &desc);

        advance_time(&env, 3601);
        client.vote(&voter, &id, &VoteChoice::For); // should panic
    }

    #[test]
    fn test_has_voted() {
        let (env, client, admin, token_client) = setup();
        let voter = Address::generate(&env);
        register_voter_with_balance(&client, &token_client, &admin, &voter, 5);

        let title = String::from_str(&env, "Proposal");
        let desc = String::from_str(&env, "Desc");
        let id = client.create_proposal(&admin, &title, &desc);

        assert!(!client.has_voted(&id, &voter));
        client.vote(&voter, &id, &VoteChoice::Abstain);
        assert!(client.has_voted(&id, &voter));
    }

    #[test]
    fn test_update_quorum_and_voting_period() {
        let (_, client, admin, _) = setup();
        client.update_quorum(&admin, &20);
        client.update_voting_period(&admin, &7200);

        let (_, quorum, period, _) = client.get_config();
        assert_eq!(quorum, 20);
        assert_eq!(period, 7200);
    }

    #[test]
    fn test_set_admin() {
        let (env, client, admin, _) = setup();
        let new_admin = Address::generate(&env);

        client.set_admin(&admin, &new_admin);
        assert_eq!(client.get_admin(), new_admin);
    }

    #[test]
    #[should_panic]
    fn test_non_admin_cannot_register_voter() {
        let (env, client, _, token_client) = setup();
        let stranger = Address::generate(&env);
        let voter = Address::generate(&env);
        token_client.mint(&voter, &5);
        client.register_voter(&stranger, &voter);
    }

    #[test]
    #[should_panic]
    fn test_register_voter_zero_balance_fails() {
        let (_, client, admin, _) = setup();
        let voter = Address::generate(&client.env);
        client.register_voter(&admin, &voter);
    }
}
