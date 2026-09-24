//! Gas (CPU instruction / memory) benchmarks using the Soroban host budget.
//!
//! Run with `cargo bench -p contract-benches --bench gas_benchmarks`.
//! Prints per-operation costs and exits non-zero if a scaling check fails:
//! - batching N snapshots must be cheaper than N single submissions
//! - voting cost must not grow with the number of existing voters
//! - finalizing a proposal must not grow with the number of existing proposals
//!
//! Set `GAS_MAX_CPU` to also enforce an absolute per-invocation CPU instruction ceiling.

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env, String, Vec,
};

use analytics::{AnalyticsContract, AnalyticsContractClient};
use governance::{GovernanceContract, GovernanceContractClient, VoteChoice};

struct Cost {
    cpu: u64,
    mem: u64,
}

fn measure(env: &Env, f: impl FnOnce()) -> Cost {
    let mut budget = env.cost_estimate().budget();
    budget.reset_unlimited();
    f();
    Cost {
        cpu: budget.cpu_instruction_cost(),
        mem: budget.memory_bytes_cost(),
    }
}

fn hash(env: &Env, seed: u64) -> BytesN<32> {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&seed.to_be_bytes());
    bytes[31] = 1; // never all-zero
    BytesN::from_array(env, &bytes)
}

fn analytics(env: &Env) -> (AnalyticsContractClient, Address) {
    let id = env.register(AnalyticsContract, ());
    let client = AnalyticsContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin, &None);
    (client, admin)
}

fn governance(env: &Env) -> (GovernanceContractClient, Address) {
    let id = env.register(GovernanceContract, ());
    let client = GovernanceContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin, &1, &3600);
    (client, admin)
}

fn main() {
    let mut rows: std::vec::Vec<(&str, Cost)> = std::vec::Vec::new();
    let mut failures: std::vec::Vec<std::string::String> = std::vec::Vec::new();

    // ── Analytics: single vs batch submission ──────────────────────────────
    const BATCH: u64 = 10;
    let single = {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = analytics(&env);
        // Pre-populate history so map size is realistic
        let mut pre = Vec::new(&env);
        for e in 1..=50u64 {
            pre.push_back((e, hash(&env, e)));
        }
        client.batch_submit_snapshots(&admin, &pre);
        measure(&env, || {
            client.submit_snapshot(&51, &hash(&env, 51), &admin);
        })
    };
    let batch = {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = analytics(&env);
        let mut pre = Vec::new(&env);
        for e in 1..=50u64 {
            pre.push_back((e, hash(&env, e)));
        }
        client.batch_submit_snapshots(&admin, &pre);
        let mut input = Vec::new(&env);
        for e in 51..51 + BATCH {
            input.push_back((e, hash(&env, e)));
        }
        measure(&env, || {
            client.batch_submit_snapshots(&admin, &input);
        })
    };
    if batch.cpu >= single.cpu * BATCH {
        failures.push(format!(
            "batch_submit_snapshots({BATCH}) = {} cpu is not cheaper than {BATCH} x submit_snapshot = {}",
            batch.cpu,
            single.cpu * BATCH
        ));
    }
    rows.push(("analytics::submit_snapshot", single));
    rows.push(("analytics::batch_submit_snapshots(10)", batch));

    // ── Governance: vote / finalize cost vs existing state ─────────────────
    let vote_cost = |existing_voters: u32, existing_proposals: u32| -> (Cost, Cost) {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = governance(&env);
        let target = Address::generate(&env);
        let title = String::from_str(&env, "Upgrade");
        for i in 0..existing_proposals {
            client.create_proposal(&admin, &title, &target, &hash(&env, u64::from(i)));
        }
        let id = client.create_proposal(&admin, &title, &target, &hash(&env, 999));
        for _ in 0..existing_voters {
            client.vote(&Address::generate(&env), &id, &VoteChoice::For);
        }
        let voter = Address::generate(&env);
        let vote = measure(&env, || {
            client.vote(&voter, &id, &VoteChoice::For);
        });
        env.ledger().with_mut(|l| l.timestamp += 3601);
        let finalize = measure(&env, || {
            client.finalize(&id, &1_000);
        });
        (vote, finalize)
    };

    let (vote_small, finalize_small) = vote_cost(1, 1);
    let (vote_large, finalize_large) = vote_cost(50, 50);
    // Allow 50% slack for key/encoding size noise; O(n) storage would be far above this.
    if vote_large.cpu * 2 > vote_small.cpu * 3 {
        failures.push(format!(
            "governance::vote grows with voter count: {} -> {} cpu",
            vote_small.cpu, vote_large.cpu
        ));
    }
    if finalize_large.cpu * 2 > finalize_small.cpu * 3 {
        failures.push(format!(
            "governance::finalize grows with proposal count: {} -> {} cpu",
            finalize_small.cpu, finalize_large.cpu
        ));
    }
    rows.push(("governance::vote (1 prior voter)", vote_small));
    rows.push(("governance::vote (50 prior voters)", vote_large));
    rows.push(("governance::finalize (1 proposal)", finalize_small));
    rows.push(("governance::finalize (50 proposals)", finalize_large));

    // ── Report ──────────────────────────────────────────────────────────────
    let max_cpu: Option<u64> = std::env::var("GAS_MAX_CPU").ok().and_then(|v| v.parse().ok());
    println!("{:<42} {:>14} {:>14}", "operation", "cpu_insns", "mem_bytes");
    for (name, cost) in &rows {
        println!("{name:<42} {:>14} {:>14}", cost.cpu, cost.mem);
        if let Some(max) = max_cpu {
            if cost.cpu > max {
                failures.push(format!("{name} = {} cpu exceeds GAS_MAX_CPU={max}", cost.cpu));
            }
        }
    }

    if !failures.is_empty() {
        eprintln!("\nGas benchmark failures:");
        for f in &failures {
            eprintln!("  - {f}");
        }
        std::process::exit(1);
    }
}
