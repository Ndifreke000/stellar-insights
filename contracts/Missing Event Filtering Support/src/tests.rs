#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::*, Address, BytesN, Env};

    use crate::SnapshotContractClient;

    #[test]
    fn test_submit_snapshot_emits_filtered_event() {
        let env = Env::default();
        let contract_id = env.register_contract(None, crate::SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let submitter = Address::random(&env);
        let epoch: u64 = 1;
        let hash = BytesN::<32>::random(&env);

        // Submit snapshot - this emits an event with topics for filtering
        let timestamp = client.submit_snapshot(&epoch, &hash, &submitter);

        assert!(timestamp.is_ok());

        // Verify events were emitted
        let events = env.events().all();
        assert!(events.len() > 0);
    }

    #[test]
    fn test_multiple_snapshots_with_different_epochs() {
        let env = Env::default();
        let contract_id = env.register_contract(None, crate::SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let submitter = Address::random(&env);

        for epoch in 1..5 {
            let hash = BytesN::<32>::random(&env);
            let result = client.submit_snapshot(&(epoch as u64), &hash, &submitter);
            assert!(result.is_ok());
        }

        // All events should have different epoch topics for filtering
        let events = env.events().all();
        assert!(events.len() >= 4);
    }
}
