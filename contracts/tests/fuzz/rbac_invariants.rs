//! Model-based RBAC invariant fuzzing for the payraider contract (#2785).
//!
//! Random sequences of admin / role / submitter / pause / submit calls, from
//! random callers drawn from a small address pool, are applied both to the
//! contract and to a reference model. After **every** step the harness checks:
//!
//! 1. **Exactly one admin.** `get_admin()` matches the model, and among all
//!    pool addresses only that one answers `has_role(_, Admin)`.
//! 2. **Revoked submitters can never submit.** Every address outside the
//!    model's live submitter set (set ∪ unexpired overlap keys) that is not
//!    the admin is refused by `submit_snapshot`, checked on every step.
//! 3. **Paused ⇒ no snapshot state change.** While paused, `latest epoch`
//!    and the chain head never move; only `unpause` (or governance) resumes.
//!    Role and submitter-set management stays allowed while paused, by design
//!    (incident response), so it is not part of this invariant.
//! 4. **Model agreement.** Success/failure of each call, the submitter set,
//!    non-admin roles, pause flag and latest epoch all equal the model.
//!
//! Generation is a seeded xorshift PRNG rather than `proptest`, so the harness
//! adds no dependency and each case is identified by one `u64` seed. A
//! failure prints its seed; add it to `REGRESSION_SEEDS` to pin it forever.
//!
//! Budget: `RBAC_FUZZ_CASES` (default 64) cases of `RBAC_FUZZ_STEPS`
//! (default 60) steps. CI raises both in `.github/workflows/contract-fuzzing.yml`.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};

use payraider::{
    DataKey, PayRaiderContract, PayRaiderContractClient, Role, RotationReason,
    MAX_ROTATION_OVERLAP, MAX_SUBMITTERS,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env, Vec as SVec,
};

/// Seeds that once found a bug (or exercise a tricky path). Never remove.
pub const REGRESSION_SEEDS: &[u64] = &[
    // grant_role(Admin) to a second address produced two admins (fixed #2785).
    0x0000_0000_0000_2785,
    // rotate onto a key still inside its overlap window.
    0x0000_0000_0000_2786,
    0xDEAD_BEEF_CAFE_F00D,
];

const POOL: usize = 6;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick(&mut self) -> usize {
        self.below(POOL as u64) as usize
    }
}

#[derive(Debug, Clone)]
enum Op {
    Submit {
        caller: usize,
        delta: u64,
    },
    AddSubmitter {
        caller: usize,
        who: usize,
    },
    RemoveSubmitter {
        caller: usize,
        who: usize,
    },
    Rotate {
        caller: usize,
        old: usize,
        new: usize,
        overlap: u32,
        compromise: bool,
    },
    Grant {
        caller: usize,
        user: usize,
        role: Role,
    },
    Revoke {
        caller: usize,
        user: usize,
        role: Role,
    },
    Pause {
        caller: usize,
    },
    Unpause {
        caller: usize,
    },
    SetAdmin {
        caller: usize,
        new: usize,
    },
    Advance {
        ledgers: u32,
    },
}

fn role_of(n: u64) -> Role {
    match n % 4 {
        0 => Role::Admin,
        1 => Role::SnapshotSubmitter,
        2 => Role::PauseManager,
        _ => Role::UpgradeManager,
    }
}

fn gen_op(rng: &mut Rng) -> Op {
    match rng.below(10) {
        0 => Op::Submit {
            caller: rng.pick(),
            delta: rng.below(3),
        },
        1 => Op::AddSubmitter {
            caller: rng.pick(),
            who: rng.pick(),
        },
        2 => Op::RemoveSubmitter {
            caller: rng.pick(),
            who: rng.pick(),
        },
        3 => Op::Rotate {
            caller: rng.pick(),
            old: rng.pick(),
            new: rng.pick(),
            overlap: [0u32, 5, 20, MAX_ROTATION_OVERLAP + 1][rng.below(4) as usize],
            compromise: rng.below(4) == 0,
        },
        4 => Op::Grant {
            caller: rng.pick(),
            user: rng.pick(),
            role: role_of(rng.next()),
        },
        5 => Op::Revoke {
            caller: rng.pick(),
            user: rng.pick(),
            role: role_of(rng.next()),
        },
        6 => Op::Pause { caller: rng.pick() },
        7 => Op::Unpause { caller: rng.pick() },
        8 => Op::SetAdmin {
            caller: rng.pick(),
            new: rng.pick(),
        },
        _ => Op::Advance {
            ledgers: 1 + rng.below(15) as u32,
        },
    }
}

/// Reference model of the contract's access-control state.
#[derive(Debug, Clone)]
struct Model {
    admin: usize,
    set: std::vec::Vec<usize>,
    /// outgoing key -> ledger at which it stops being accepted
    outgoing: BTreeMap<usize, u32>,
    /// non-submitter roles other than Admin
    roles: BTreeMap<usize, BTreeSet<u32>>,
    paused: bool,
    latest: u64,
    ledger: u32,
}

impl Model {
    fn new() -> Self {
        Model {
            admin: 0,
            set: std::vec::Vec::new(),
            outgoing: BTreeMap::new(),
            roles: BTreeMap::new(),
            paused: false,
            latest: 0,
            ledger: 0,
        }
    }
    fn prune(&mut self) {
        let now = self.ledger;
        self.outgoing.retain(|_, until| now < *until);
    }
    fn live_submitter(&self, a: usize) -> bool {
        self.set.contains(&a) || self.outgoing.get(&a).is_some_and(|u| self.ledger < *u)
    }
    fn has(&self, a: usize, role: Role) -> bool {
        match role {
            Role::Admin => a == self.admin,
            Role::SnapshotSubmitter => self.live_submitter(a),
            r => self.roles.get(&a).is_some_and(|s| s.contains(&(r as u32))),
        }
    }
    fn insert(&mut self, who: usize) -> bool {
        self.prune();
        if self.set.contains(&who) || self.set.len() as u32 >= MAX_SUBMITTERS {
            return false;
        }
        self.outgoing.remove(&who);
        self.set.push(who);
        true
    }
    fn delete(&mut self, who: usize) -> bool {
        self.prune();
        if let Some(i) = self.set.iter().position(|x| *x == who) {
            self.set.remove(i);
            return true;
        }
        self.outgoing.remove(&who).is_some()
    }

    /// Apply `op`; returns whether the contract call is expected to succeed.
    fn apply(&mut self, op: &Op) -> bool {
        match *op {
            Op::Submit { caller, delta } => {
                if self.paused || !(self.live_submitter(caller) || caller == self.admin) {
                    return false;
                }
                if delta == 0 {
                    return false;
                }
                self.latest += delta;
                true
            }
            Op::AddSubmitter { caller, who } => caller == self.admin && self.insert(who),
            Op::RemoveSubmitter { caller, who } => caller == self.admin && self.delete(who),
            Op::Rotate {
                caller,
                old,
                new,
                overlap,
                compromise,
            } => {
                if caller != self.admin {
                    return false;
                }
                if overlap > MAX_ROTATION_OVERLAP || (compromise && overlap > 0) {
                    return false;
                }
                self.prune();
                let Some(idx) = self.set.iter().position(|x| *x == old) else {
                    return false;
                };
                if new == old
                    || self.set.contains(&new)
                    || new == self.admin
                    || self.outgoing.contains_key(&new)
                {
                    return false;
                }
                if overlap > 0 && self.outgoing.len() as u32 >= MAX_SUBMITTERS {
                    return false;
                }
                self.set[idx] = new;
                if overlap > 0 {
                    self.outgoing.insert(old, self.ledger + overlap);
                }
                true
            }
            Op::Grant { caller, user, role } => {
                if caller != self.admin || role == Role::Admin {
                    return false;
                }
                if role == Role::SnapshotSubmitter {
                    self.prune();
                    if self.set.contains(&user) {
                        return true;
                    }
                    return self.insert(user);
                }
                self.roles.entry(user).or_default().insert(role as u32);
                true
            }
            Op::Revoke { caller, user, role } => {
                if caller != self.admin || (role == Role::Admin && user == self.admin) {
                    return false;
                }
                match role {
                    Role::SnapshotSubmitter => {
                        self.delete(user);
                    }
                    Role::Admin => {}
                    r => {
                        if let Some(s) = self.roles.get_mut(&user) {
                            s.remove(&(r as u32));
                        }
                    }
                }
                true
            }
            Op::Pause { caller } | Op::Unpause { caller } => {
                if !(caller == self.admin || self.has(caller, Role::PauseManager)) {
                    return false;
                }
                self.paused = matches!(op, Op::Pause { .. });
                true
            }
            Op::SetAdmin { caller, new } => {
                if self.paused || caller != self.admin {
                    return false;
                }
                // Contract replaces both role entries wholesale.
                self.roles.remove(&self.admin);
                self.roles.remove(&new);
                self.admin = new;
                true
            }
            Op::Advance { ledgers } => {
                self.ledger += ledgers;
                true
            }
        }
    }
}

struct Harness {
    env: Env,
    client: PayRaiderContractClient<'static>,
    pool: std::vec::Vec<Address>,
}

impl Harness {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| li.sequence_number = 0);
        let id = env.register_contract(None, PayRaiderContract);
        let client: PayRaiderContractClient<'static> = PayRaiderContractClient::new(&env, &id);
        let pool: std::vec::Vec<Address> = (0..POOL).map(|_| Address::generate(&env)).collect();
        client.initialize(&pool[0]);
        Harness { env, client, pool }
    }

    fn hash(&self, n: u64) -> BytesN<32> {
        let mut b = [0u8; 32];
        b[..8].copy_from_slice(&n.to_be_bytes());
        b[31] = 1;
        BytesN::from_array(&self.env, &b)
    }

    /// Run `op` against the contract; returns whether it succeeded.
    fn run(&self, op: &Op, latest: u64) -> bool {
        let p = &self.pool;
        let c = &self.client;
        match *op {
            Op::Submit { caller, delta } => {
                let e = latest + delta;
                c.try_submit_snapshot(&e, &self.hash(e), &p[caller]).is_ok()
            }
            Op::AddSubmitter { caller, who } => c.try_add_submitter(&p[caller], &p[who]).is_ok(),
            Op::RemoveSubmitter { caller, who } => {
                c.try_remove_submitter(&p[caller], &p[who]).is_ok()
            }
            Op::Rotate {
                caller,
                old,
                new,
                overlap,
                compromise,
            } => {
                let reason = if compromise {
                    RotationReason::Compromise
                } else {
                    RotationReason::Scheduled
                };
                c.try_rotate_submitter_with_policy(&p[caller], &p[old], &p[new], &overlap, &reason)
                    .is_ok()
            }
            Op::Grant { caller, user, role } => {
                c.try_grant_role(&p[caller], &p[user], &role).is_ok()
            }
            Op::Revoke { caller, user, role } => {
                c.try_revoke_role(&p[caller], &p[user], &role).is_ok()
            }
            Op::Pause { caller } => c.try_pause(&p[caller]).is_ok(),
            Op::Unpause { caller } => c.try_unpause(&p[caller]).is_ok(),
            Op::SetAdmin { caller, new } => {
                c.try_propose_admin(&p[caller], &p[new]).is_ok()
                    && c.try_accept_admin(&p[new]).is_ok()
            }
            Op::Advance { ledgers } => {
                self.env
                    .ledger()
                    .with_mut(|li| li.sequence_number += ledgers);
                true
            }
        }
    }

    /// Check every invariant against the model. `Err` describes the first
    /// violation found.
    fn check(&self, m: &Model, before: &(u64, BytesN<32>)) -> Result<(), std::string::String> {
        let c = &self.client;
        let admin = c.get_admin();
        if admin != self.pool[m.admin] {
            return Err(format!("admin mismatch: model={}", m.admin));
        }
        // 1. exactly one admin
        let admins: std::vec::Vec<usize> = (0..POOL)
            .filter(|i| c.has_role(&self.pool[*i], &Role::Admin))
            .collect();
        if admins != [m.admin] {
            return Err(format!(
                "expected exactly one admin {}, found {admins:?}",
                m.admin
            ));
        }
        // 4. model agreement
        if c.is_paused() != m.paused {
            return Err("pause flag mismatch".into());
        }
        if c.get_latest_epoch() != m.latest {
            return Err(format!(
                "latest epoch {} != model {}",
                c.get_latest_epoch(),
                m.latest
            ));
        }
        let set: std::vec::Vec<Address> = c.get_submitters().iter().collect();
        let want: std::vec::Vec<Address> = m.set.iter().map(|i| self.pool[*i].clone()).collect();
        if set != want {
            return Err(format!("submitter set mismatch, model {:?}", m.set));
        }
        for i in 0..POOL {
            for r in [
                Role::SnapshotSubmitter,
                Role::PauseManager,
                Role::UpgradeManager,
            ] {
                if c.has_role(&self.pool[i], &r) != m.has(i, r) {
                    return Err(format!("has_role({i}, {r:?}) disagrees with model"));
                }
            }
        }
        // 3. paused ⇒ snapshot history frozen
        if m.paused && c.get_chain_head() != *before {
            return Err("snapshot state changed while paused".into());
        }
        // 2. non-submitters can never submit (probe, must not change state)
        let probe = m.latest + 1;
        for i in 0..POOL {
            if i != m.admin && !m.live_submitter(i) {
                let r = c.try_submit_snapshot(&probe, &self.hash(probe), &self.pool[i]);
                if r.is_ok() {
                    return Err(format!("revoked/unknown address {i} was able to submit"));
                }
            }
        }
        Ok(())
    }
}

fn run_case(seed: u64, steps: usize) -> Result<(), std::string::String> {
    let mut rng = Rng(seed | 1);
    let h = Harness::new();
    let mut m = Model::new();
    h.check(&m, &h.client.get_chain_head())?;
    for step in 0..steps {
        let op = gen_op(&mut rng);
        let before = h.client.get_chain_head();
        let latest = m.latest;
        let expected = m.clone().apply(&op);
        let got = h.run(&op, latest);
        if got != expected {
            return Err(format!(
                "seed {seed:#x} step {step}: {op:?} contract ok={got} model ok={expected}"
            ));
        }
        if got {
            m.apply(&op);
        }
        h.check(&m, &before)
            .map_err(|e| format!("seed {seed:#x} step {step} after {op:?}: {e}"))?;
    }
    Ok(())
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[test]
fn rbac_invariants_hold_for_regression_seeds() {
    for seed in REGRESSION_SEEDS {
        run_case(*seed, 80).unwrap_or_else(|e| panic!("{e}"));
    }
}

#[test]
fn rbac_invariants_hold_for_random_sequences() {
    let cases = env_usize("RBAC_FUZZ_CASES", 64);
    let steps = env_usize("RBAC_FUZZ_STEPS", 60);
    let base: u64 = std::env::var("RBAC_FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0x5EED_0000_0000_0001);
    for i in 0..cases as u64 {
        let seed = base.wrapping_add(i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        if let Err(e) = run_case(seed, steps) {
            panic!("{e}\nadd this seed to REGRESSION_SEEDS to keep it as a regression");
        }
    }
}

/// The harness must catch a broken RBAC check. Simulate the classic bug — a
/// second address holding `Role::Admin` (what `grant_role(Admin)` used to
/// allow) — by writing it straight into storage, and assert the invariant
/// check reports it.
#[test]
fn harness_detects_injected_second_admin() {
    let h = Harness::new();
    let m = Model::new();
    let id = h.client.address.clone();
    let intruder = h.pool[3].clone();
    h.env.as_contract(&id, || {
        let mut roles = SVec::new(&h.env);
        roles.push_back(Role::Admin);
        h.env
            .storage()
            .persistent()
            .set(&DataKey::Roles(intruder.clone()), &roles);
    });
    let err = h.check(&m, &h.client.get_chain_head()).unwrap_err();
    assert!(err.contains("exactly one admin"), "{err}");
}

/// Injected bug: a revoked submitter left in storage must be caught.
#[test]
fn harness_detects_injected_stale_submitter() {
    let h = Harness::new();
    let m = Model::new();
    let id = h.client.address.clone();
    let stale = h.pool[2].clone();
    h.env.as_contract(&id, || {
        let mut set = SVec::new(&h.env);
        set.push_back(stale.clone());
        h.env.storage().instance().set(&DataKey::Submitters, &set);
    });
    assert!(h.check(&m, &h.client.get_chain_head()).is_err());
}
