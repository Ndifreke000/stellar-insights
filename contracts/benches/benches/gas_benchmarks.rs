//! Gas (CPU instruction / memory) benchmarks using the Soroban host budget.
//!
//! Run with `cargo bench -p contract-benches --bench gas_benchmarks`.
//!
//! Measures every state-changing and read entrypoint of the `payraider`
//! contract once, against realistic pre-populated state, and prints one
//! machine-readable line per entrypoint:
//!
//! ```text
//! GAS payraider::submit_snapshot cpu=123456 mem=7890
//! ```
//!
//! Host budget figures are deterministic (no wall-clock noise), so CI can hold
//! each entrypoint to a tight per-entrypoint threshold. `scripts/check_gas_regression.py`
//! compares these lines against the baseline in `docs/GAS_COSTS.md` using the
//! thresholds in `contracts/benches/gas-thresholds.toml`.
//!
//! Set `GAS_MAX_CPU` to also enforce an absolute per-invocation CPU instruction ceiling.

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

use payraider::{PayRaiderContract, PayRaiderContractClient, Role};

/// Snapshots stored before measuring, so map-backed reads and writes are
/// costed at a realistic size rather than against empty storage.
const HISTORY: u64 = 50;

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

struct Fixture {
    env: Env,
    id: Address,
    admin: Address,
    submitter: Address,
}

/// Initialised contract with one authorised submitter and `HISTORY` snapshots.
fn fixture() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(PayRaiderContract, ());
    let client = PayRaiderContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    let submitter = Address::generate(&env);
    client.initialize(&admin);
    client.add_submitter(&admin, &submitter);
    for epoch in 1..=HISTORY {
        client.submit_snapshot(&epoch, &hash(&env, epoch), &submitter);
    }
    Fixture {
        env,
        id,
        admin,
        submitter,
    }
}

fn main() {
    let mut rows: std::vec::Vec<(&str, Cost)> = std::vec::Vec::new();

    // ── initialize ──────────────────────────────────────────────────────────
    {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(PayRaiderContract, ());
        let client = PayRaiderContractClient::new(&env, &id);
        let admin = Address::generate(&env);
        rows.push((
            "payraider::initialize",
            measure(&env, || client.initialize(&admin)),
        ));
    }

    // ── snapshot write / read path ──────────────────────────────────────────
    {
        let f = fixture();
        let client = PayRaiderContractClient::new(&f.env, &f.id);
        let next = HISTORY + 1;
        rows.push((
            "payraider::submit_snapshot",
            measure(&f.env, || {
                client
                    .submit_snapshot(&next, &hash(&f.env, next), &f.submitter);
            }),
        ));
        rows.push((
            "payraider::get_snapshot",
            measure(&f.env, || {
                client.get_snapshot(&(HISTORY / 2));
            }),
        ));
        rows.push((
            "payraider::latest_snapshot",
            measure(&f.env, || {
                client.latest_snapshot();
            }),
        ));
        rows.push((
            "payraider::bump_ttl",
            measure(&f.env, || client.bump_ttl()),
        ));
    }

    // ── submitter set (#2600) ───────────────────────────────────────────────
    {
        let f = fixture();
        let client = PayRaiderContractClient::new(&f.env, &f.id);
        let added = Address::generate(&f.env);
        let rotated_in = Address::generate(&f.env);
        rows.push((
            "payraider::add_submitter",
            measure(&f.env, || client.add_submitter(&f.admin, &added)),
        ));
        rows.push((
            "payraider::rotate_submitter",
            measure(&f.env, || {
                client
                    .rotate_submitter(&f.admin, &f.submitter, &rotated_in)
            }),
        ));
        rows.push((
            "payraider::remove_submitter",
            measure(&f.env, || client.remove_submitter(&f.admin, &added)),
        ));
    }

    // ── admin / roles / pause ───────────────────────────────────────────────
    {
        let f = fixture();
        let client = PayRaiderContractClient::new(&f.env, &f.id);
        let user = Address::generate(&f.env);
        rows.push((
            "payraider::grant_role",
            measure(&f.env, || {
                client.grant_role(&f.admin, &user, &Role::PauseManager)
            }),
        ));
        rows.push((
            "payraider::revoke_role",
            measure(&f.env, || {
                client.revoke_role(&f.admin, &user, &Role::PauseManager)
            }),
        ));
        rows.push((
            "payraider::pause",
            measure(&f.env, || client.pause(&f.admin)),
        ));
        rows.push((
            "payraider::unpause",
            measure(&f.env, || client.unpause(&f.admin)),
        ));
        let new_admin = Address::generate(&f.env);
        rows.push((
            "payraider::set_admin",
            measure(&f.env, || client.set_admin(&f.admin, &new_admin)),
        ));
    }

    // ── Report ──────────────────────────────────────────────────────────────
    let max_cpu: Option<u64> = std::env::var("GAS_MAX_CPU")
        .ok()
        .and_then(|v| v.parse().ok());
    let mut failures: std::vec::Vec<std::string::String> = std::vec::Vec::new();

    for (name, cost) in &rows {
        println!("GAS {name} cpu={} mem={}", cost.cpu, cost.mem);
        if let Some(max) = max_cpu {
            if cost.cpu > max {
                failures.push(format!(
                    "{name} = {} cpu exceeds GAS_MAX_CPU={max}",
                    cost.cpu
                ));
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
