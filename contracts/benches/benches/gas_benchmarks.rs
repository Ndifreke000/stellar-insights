//! Gas (CPU instruction / memory / ledger I/O) benchmarks using the Soroban
//! host budget and invocation metering.
//!
//! Run with `cargo bench -p contract-benches --bench gas_benchmarks`.
//!
//! Measures **every public entrypoint** of the `payraider` contract once,
//! against realistic pre-populated state, and prints one machine-readable line
//! per entrypoint (#2787):
//!
//! ```text
//! GAS payraider::submit_snapshot cpu=123456 mem=7890 read_entries=3 write_entries=2 read_bytes=0 write_bytes=812
//! ```
//!
//! Host budget figures are deterministic (no wall-clock noise), so CI can hold
//! each entrypoint to a tight threshold. `scripts/check_gas_regression.py`
//! compares these lines against the baseline in `docs/GAS_COSTS.md` using the
//! thresholds in `contracts/benches/gas-thresholds.toml`, and writes a JSON
//! result file and a markdown diff table.
//!
//! `read_entries` counts every ledger entry the invocation touched (disk and
//! in-memory), since live Soroban state is in memory and disk reads are almost
//! always zero in the test host. `read_bytes` is the disk-read figure.
//!
//! Set `GAS_MAX_CPU` to also enforce an absolute per-invocation CPU instruction ceiling.

use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, BytesN, Env};

use payraider::{
    PayRaiderContract, PayRaiderContractClient, Role, DEFAULT_UPGRADE_DELAY_LEDGERS,
};

/// Snapshots stored before measuring, so map-backed reads and writes are
/// costed at a realistic size rather than against empty storage.
const HISTORY: u64 = 50;

/// Every public entrypoint of the contract. The benchmark refuses to report if
/// one of these was not measured, so a new entrypoint must be added here and
/// measured below (and given a threshold in gas-thresholds.toml).
const ENTRYPOINTS: &[&str] = &[
    "initialize",
    "get_version",
    "submit_snapshot",
    "get_snapshot",
    "latest_snapshot",
    "get_admin",
    "propose_admin",
    "accept_admin",
    "cancel_admin_transfer",
    "get_pending_admin",
    "get_latest_epoch",
    "pause",
    "unpause",
    "schedule_upgrade",
    "execute_upgrade",
    "cancel_upgrade",
    "get_pending_upgrade",
    "get_upgrade_delay",
    "set_upgrade_delay",
    "get_storage_version",
    "migrate",
    "set_governance",
    "get_governance",
    "set_admin_by_governance",
    "set_paused_by_governance",
    "is_paused",
    "add_submitter",
    "remove_submitter",
    "rotate_submitter",
    "get_submitters",
    "bump_ttl",
    "get_role_holders",
    "grant_role",
    "revoke_role",
    "has_role",
    "get_metadata",
    "get_contract_info",
];

struct Cost {
    cpu: u64,
    mem: u64,
    read_entries: u64,
    write_entries: u64,
    read_bytes: u64,
    write_bytes: u64,
}

/// Run `f` (exactly one top-level contract invocation) and record its cost.
fn measure(env: &Env, f: impl FnOnce()) -> Cost {
    let mut budget = env.cost_estimate().budget();
    budget.reset_unlimited();
    f();
    let res = env.cost_estimate().resources();
    Cost {
        cpu: budget.cpu_instruction_cost(),
        mem: budget.memory_bytes_cost(),
        read_entries: u64::from(res.disk_read_entries) + u64::from(res.memory_read_entries),
        write_entries: u64::from(res.write_entries),
        read_bytes: u64::from(res.disk_read_bytes),
        write_bytes: u64::from(res.write_bytes),
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
    governance: Address,
}

impl Fixture {
    fn client(&self) -> PayRaiderContractClient<'_> {
        PayRaiderContractClient::new(&self.env, &self.id)
    }
}

/// Initialised contract with one authorised submitter, a governance address
/// and `HISTORY` snapshots.
fn fixture() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(PayRaiderContract, ());
    let client = PayRaiderContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    let submitter = Address::generate(&env);
    let governance = Address::generate(&env);
    client.initialize(&admin);
    client.add_submitter(&admin, &submitter);
    client.set_governance(&admin, &governance);
    for epoch in 1..=HISTORY {
        client.submit_snapshot(&epoch, &hash(&env, epoch), &submitter);
    }
    Fixture {
        env,
        id,
        admin,
        submitter,
        governance,
    }
}

struct Rows(std::vec::Vec<(&'static str, Cost)>);

impl Rows {
    fn push(&mut self, entrypoint: &'static str, cost: Cost) {
        self.0.push((entrypoint, cost));
    }
}

fn main() {
    let mut rows = Rows(std::vec::Vec::new());

    // ── initialize ──────────────────────────────────────────────────────────
    {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(PayRaiderContract, ());
        let client = PayRaiderContractClient::new(&env, &id);
        let admin = Address::generate(&env);
        rows.push("initialize", measure(&env, || client.initialize(&admin)));
    }

    // ── snapshot write / read path ──────────────────────────────────────────
    {
        let f = fixture();
        let client = f.client();
        let next = HISTORY + 1;
        rows.push(
            "submit_snapshot",
            measure(&f.env, || {
                client.submit_snapshot(&next, &hash(&f.env, next), &f.submitter);
            }),
        );
        rows.push(
            "get_snapshot",
            measure(&f.env, || {
                client.get_snapshot(&(HISTORY / 2));
            }),
        );
        rows.push(
            "latest_snapshot",
            measure(&f.env, || {
                client.latest_snapshot();
            }),
        );
        rows.push("bump_ttl", measure(&f.env, || client.bump_ttl()));
    }

    // ── views ───────────────────────────────────────────────────────────────
    {
        let f = fixture();
        let client = f.client();
        rows.push("get_version", measure(&f.env, || {
                let _ = client.get_version();
            }));
        rows.push("get_admin", measure(&f.env, || {
                let _ = client.get_admin();
            }));
        rows.push(
            "get_pending_admin",
            measure(&f.env, || {
                let _ = client.get_pending_admin();
            }),
        );
        rows.push(
            "get_latest_epoch",
            measure(&f.env, || {
                let _ = client.get_latest_epoch();
            }),
        );
        rows.push(
            "get_pending_upgrade",
            measure(&f.env, || {
                let _ = client.get_pending_upgrade();
            }),
        );
        rows.push(
            "get_upgrade_delay",
            measure(&f.env, || {
                let _ = client.get_upgrade_delay();
            }),
        );
        rows.push(
            "get_storage_version",
            measure(&f.env, || {
                let _ = client.get_storage_version();
            }),
        );
        rows.push(
            "get_governance",
            measure(&f.env, || {
                let _ = client.get_governance();
            }),
        );
        rows.push("is_paused", measure(&f.env, || {
                let _ = client.is_paused();
            }));
        rows.push(
            "get_submitters",
            measure(&f.env, || {
                let _ = client.get_submitters();
            }),
        );
        rows.push(
            "get_role_holders",
            measure(&f.env, || {
                let _ = client.get_role_holders();
            }),
        );
        rows.push(
            "has_role",
            measure(&f.env, || {
                let _ = client.has_role(&f.admin, &Role::Admin);
            }),
        );
        rows.push("get_metadata", measure(&f.env, || {
                let _ = client.get_metadata();
            }));
        rows.push(
            "get_contract_info",
            measure(&f.env, || {
                let _ = client.get_contract_info();
            }),
        );
    }

    // ── submitter set (#2600) ───────────────────────────────────────────────
    {
        let f = fixture();
        let client = f.client();
        let added = Address::generate(&f.env);
        let rotated_in = Address::generate(&f.env);
        rows.push(
            "add_submitter",
            measure(&f.env, || client.add_submitter(&f.admin, &added)),
        );
        rows.push(
            "rotate_submitter",
            measure(&f.env, || {
                client.rotate_submitter(&f.admin, &f.submitter, &rotated_in)
            }),
        );
        rows.push(
            "remove_submitter",
            measure(&f.env, || client.remove_submitter(&f.admin, &added)),
        );
    }

    // ── roles / pause ───────────────────────────────────────────────────────
    {
        let f = fixture();
        let client = f.client();
        let user = Address::generate(&f.env);
        rows.push(
            "grant_role",
            measure(&f.env, || {
                client.grant_role(&f.admin, &user, &Role::PauseManager)
            }),
        );
        rows.push(
            "revoke_role",
            measure(&f.env, || {
                client.revoke_role(&f.admin, &user, &Role::PauseManager)
            }),
        );
        rows.push("pause", measure(&f.env, || client.pause(&f.admin)));
        rows.push("unpause", measure(&f.env, || client.unpause(&f.admin)));
    }

    // ── two-step admin transfer (#2783) ─────────────────────────────────────
    {
        let f = fixture();
        let client = f.client();
        let cancelled = Address::generate(&f.env);
        client.propose_admin(&f.admin, &cancelled);
        rows.push(
            "cancel_admin_transfer",
            measure(&f.env, || client.cancel_admin_transfer(&f.admin)),
        );
        let next = Address::generate(&f.env);
        rows.push(
            "propose_admin",
            measure(&f.env, || client.propose_admin(&f.admin, &next)),
        );
        rows.push(
            "accept_admin",
            measure(&f.env, || client.accept_admin(&next)),
        );
    }

    // ── timelocked upgrades (#2783) ─────────────────────────────────────────
    {
        let f = fixture();
        let client = f.client();
        rows.push(
            "set_upgrade_delay",
            measure(&f.env, || {
                client.set_upgrade_delay(&f.admin, &(DEFAULT_UPGRADE_DELAY_LEDGERS * 2))
            }),
        );
        client.schedule_upgrade(&f.admin, &hash(&f.env, 1));
        rows.push(
            "cancel_upgrade",
            measure(&f.env, || client.cancel_upgrade(&f.admin)),
        );
        rows.push(
            "schedule_upgrade",
            measure(&f.env, || {
                let _ = client.schedule_upgrade(&f.admin, &hash(&f.env, 2));
            }),
        );
        let delay = client.get_upgrade_delay();
        f.env
            .ledger()
            .with_mut(|li| li.sequence_number += delay);
        // No real Wasm is uploaded in the bench host, so the final install is
        // rejected by the host. Everything the contract itself does — auth,
        // timelock check, state clear, events — is still measured, and the
        // figure is deterministic.
        rows.push(
            "execute_upgrade",
            measure(&f.env, || {
                let _ = client.try_execute_upgrade(&f.admin);
            }),
        );
    }

    // ── governance and migration ────────────────────────────────────────────
    {
        let f = fixture();
        let client = f.client();
        rows.push(
            "set_governance",
            measure(&f.env, || client.set_governance(&f.admin, &f.governance)),
        );
        rows.push(
            "set_paused_by_governance",
            measure(&f.env, || {
                client.set_paused_by_governance(&f.governance, &false)
            }),
        );
        // Migration from pre-versioning (v0) storage.
        f.env.as_contract(&f.id, || {
            f.env
                .storage()
                .instance()
                .remove(&payraider::DataKey::StorageVersion);
        });
        rows.push(
            "migrate",
            measure(&f.env, || {
                let _ = client.migrate(&f.admin);
            }),
        );
        let voted = Address::generate(&f.env);
        rows.push(
            "set_admin_by_governance",
            measure(&f.env, || {
                client.set_admin_by_governance(&f.governance, &voted)
            }),
        );
    }

    // ── Coverage: every public entrypoint measured exactly once ─────────────
    let mut problems: std::vec::Vec<std::string::String> = std::vec::Vec::new();
    for name in ENTRYPOINTS {
        let n = rows.0.iter().filter(|(r, _)| r == name).count();
        if n != 1 {
            problems.push(format!("payraider::{name} measured {n} times (expected 1)"));
        }
    }
    for (name, _) in &rows.0 {
        if !ENTRYPOINTS.contains(name) {
            problems.push(format!("payraider::{name} measured but not in ENTRYPOINTS"));
        }
    }

    // ── Report ──────────────────────────────────────────────────────────────
    let max_cpu: Option<u64> = std::env::var("GAS_MAX_CPU")
        .ok()
        .and_then(|v| v.parse().ok());

    for (name, c) in &rows.0 {
        println!(
            "GAS payraider::{name} cpu={} mem={} read_entries={} write_entries={} read_bytes={} write_bytes={}",
            c.cpu, c.mem, c.read_entries, c.write_entries, c.read_bytes, c.write_bytes
        );
        if let Some(max) = max_cpu {
            if c.cpu > max {
                problems.push(format!(
                    "payraider::{name} = {} cpu exceeds GAS_MAX_CPU={max}",
                    c.cpu
                ));
            }
        }
    }

    if !problems.is_empty() {
        eprintln!("\nGas benchmark failures:");
        for p in &problems {
            eprintln!("  - {p}");
        }
        std::process::exit(1);
    }
}
