#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, String,
    Symbol, Vec,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ~30 days at 5 s/ledger
const LEDGERS_TO_EXTEND: u32 = 518_400;
const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    Unauthorized = 1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum Role {
    SuperAdmin,
    Admin,
    Operator,
    Viewer,
}

#[derive(Clone)]
#[contracttype]
pub struct Permission {
    pub role: Role,
    pub function: Symbol,
}

#[contracttype]
pub enum DataKey {
    Roles(Address),
    Permissions(Role),
    Version,
}

// ---------------------------------------------------------------------------
// Event types — emitted on every access-control mutation for audit trails
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleGrantedEvent {
    pub admin: Address,
    pub user: Address,
    pub role: Role,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleRevokedEvent {
    pub admin: Address,
    pub user: Address,
    pub role: Role,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermissionGrantedEvent {
    pub admin: Address,
    pub role: Role,
    pub function: Symbol,
}

/// Extended contract metadata for public disclosure
#[contracttype]
#[derive(Clone, Debug)]
pub struct PublicMetadata {
    pub name: soroban_sdk::String,
    pub version: soroban_sdk::String,
    pub author: soroban_sdk::String,
    pub description: soroban_sdk::String,
    pub repository: soroban_sdk::String,
    pub license: soroban_sdk::String,
}

/// Contract info combining metadata with runtime state
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractInfo {
    pub metadata: PublicMetadata,
    pub initialized: bool,
    pub total_roles: u32,
}

#[contract]
pub struct AccessControlContract;

#[contractimpl]
impl AccessControlContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        let mut roles = Vec::new(&env);
        roles.push_back(Role::SuperAdmin);
        env.storage()
            .persistent()
            .set(&DataKey::Roles(admin.clone()), &roles);
        env.storage().persistent().extend_ttl(
            &DataKey::Roles(admin),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        env.storage()
            .instance()
            .set(&DataKey::Version, &String::from_str(&env, VERSION));
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
    }

    pub fn get_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or_else(|| String::from_str(&env, VERSION))
    }

    pub fn grant_role(env: Env, caller: Address, user: Address, role: Role) -> Result<(), Error> {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::Admin)?;

        let mut roles = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Role>>(&DataKey::Roles(user.clone()))
            .unwrap_or(Vec::new(&env));
        roles.push_back(role.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Roles(user.clone()), &roles);
        env.storage().persistent().extend_ttl(
            &DataKey::Roles(user.clone()),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        bump_instance(&env);

        env.events().publish(
            (symbol_short!("role_grnt"), user.clone()),
            RoleGrantedEvent {
                admin: caller,
                user,
                role,
            },
        );
        Ok(())
    }

    pub fn revoke_role(env: Env, caller: Address, user: Address, role: Role) -> Result<(), Error> {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::Admin)?;

        if let Some(roles) = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Role>>(&DataKey::Roles(user.clone()))
        {
            let mut new_roles = Vec::new(&env);
            for r in roles.iter() {
                if !roles_equal(&r, &role) {
                    new_roles.push_back(r);
                }
            }
            env.storage()
                .persistent()
                .set(&DataKey::Roles(user.clone()), &new_roles);
            env.storage().persistent().extend_ttl(
                &DataKey::Roles(user.clone()),
                LEDGERS_TO_EXTEND,
                LEDGERS_TO_EXTEND,
            );
            bump_instance(&env);

            env.events().publish(
                (symbol_short!("role_rvk"), user.clone()),
                RoleRevokedEvent {
                    admin: caller,
                    user,
                    role,
                },
            );
        }
        Ok(())
    }

    pub fn has_role(env: Env, user: Address, role: Role) -> bool {
        if let Some(roles) = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Role>>(&DataKey::Roles(user))
        {
            for r in roles.iter() {
                if roles_equal(&r, &role) {
                    return true;
                }
            }
        }
        false
    }

    pub fn grant_permission(
        env: Env,
        caller: Address,
        role: Role,
        function: Symbol,
    ) -> Result<(), Error> {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::Admin)?;

        let mut perms = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Symbol>>(&DataKey::Permissions(role.clone()))
            .unwrap_or(Vec::new(&env));
        perms.push_back(function.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Permissions(role.clone()), &perms);
        env.storage().persistent().extend_ttl(
            &DataKey::Permissions(role.clone()),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        bump_instance(&env);

        env.events().publish(
            (symbol_short!("perm_grnt"), role.clone()),
            PermissionGrantedEvent {
                admin: caller,
                role,
                function,
            },
        );
        Ok(())
    }

    /// Check if a user has permission for a given function.
    /// SuperAdmin and Admin have all permissions (bypass).
    /// For other roles, checks the user's roles and inherited lower-role permissions.
    pub fn check_permission(env: Env, user: Address, function: Symbol) -> bool {
        if let Some(roles) = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Role>>(&DataKey::Roles(user))
        {
            // Find the highest role level the user has
            let mut max_level = 0u32;
            for r in roles.iter() {
                let level = role_level(&r);
                if level > max_level {
                    max_level = level;
                }
            }

            // SuperAdmin and Admin have all permissions
            if max_level >= role_level(&Role::Admin) {
                return true;
            }

            // Check permissions for all roles at or below the user's highest level.
            // This implements role inheritance: e.g. an Operator inherits Viewer permissions.
            let checkable = [Role::Operator, Role::Viewer];
            for check_role in checkable.iter() {
                if role_level(check_role) <= max_level {
                    if let Some(perms) = env
                        .storage()
                        .persistent()
                        .get::<DataKey, Vec<Symbol>>(&DataKey::Permissions(check_role.clone()))
                    {
                        for perm in perms.iter() {
                            if perm == function {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn require_role(env: &Env, user: &Address, role: Role) -> Result<(), Error> {
        let required_level = role_level(&role);
        if let Some(roles) = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Role>>(&DataKey::Roles(user.clone()))
        {
            for r in roles.iter() {
                if role_level(&r) >= required_level {
                    return Ok(());
                }
            }
        }
        Err(Error::Unauthorized)
    }

    /// Get public contract metadata
    pub fn get_metadata(env: Env) -> PublicMetadata {
        PublicMetadata {
            name: soroban_sdk::String::from_str(&env, "Stellar Insights Access Control"),
            version: soroban_sdk::String::from_str(&env, VERSION),
            author: soroban_sdk::String::from_str(&env, "Stellar Insights Team"),
            description: soroban_sdk::String::from_str(
                &env,
                "Role-based access control contract for Stellar Insights",
            ),
            repository: soroban_sdk::String::from_str(
                &env,
                "https://github.com/stellar-insights/contracts",
            ),
            license: soroban_sdk::String::from_str(&env, "MIT"),
        }
    }

    /// Get comprehensive contract information
    pub fn get_contract_info(env: Env) -> ContractInfo {
        // Check if contract is initialized by looking for the version key
        let initialized = env.storage().instance().has(&DataKey::Version);

        ContractInfo {
            metadata: Self::get_metadata(env),
            initialized,
            total_roles: 0,
        }
    }
}

fn roles_equal(a: &Role, b: &Role) -> bool {
    matches!(
        (a, b),
        (Role::SuperAdmin, Role::SuperAdmin)
            | (Role::Admin, Role::Admin)
            | (Role::Operator, Role::Operator)
            | (Role::Viewer, Role::Viewer)
    )
}

fn role_level(role: &Role) -> u32 {
    match role {
        Role::Viewer => 1,
        Role::Operator => 2,
        Role::Admin => 3,
        Role::SuperAdmin => 4,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Events},
        Env, Val, Vec,
    };

    macro_rules! setup {
        ($env:ident, $client:ident, $admin:ident) => {
            let $env = Env::default();
            let contract_id = $env.register_contract(None, AccessControlContract);
            let $client = AccessControlContractClient::new(&$env, &contract_id);
            let $admin = Address::generate(&$env);
            $env.mock_all_auths();
            $client.initialize(&$admin);
        };
    }

    // =========================================================================
    // initialize
    // =========================================================================

    #[test]
    fn test_initialize_grants_super_admin_role() {
        setup!(env, client, admin);
        assert!(client.has_role(&admin, &Role::SuperAdmin));
    }

    #[test]
    fn test_initialize_does_not_grant_admin_to_initializer() {
        setup!(env, client, admin);
        assert!(!client.has_role(&admin, &Role::Admin));
    }

    #[test]
    fn test_initialize_does_not_grant_operator_to_admin() {
        setup!(env, client, admin);
        assert!(!client.has_role(&admin, &Role::Operator));
    }

    #[test]
    fn test_initialize_does_not_grant_viewer_to_admin() {
        setup!(env, client, admin);
        assert!(!client.has_role(&admin, &Role::Viewer));
    }

    // =========================================================================
    // grant_role
    // =========================================================================

    #[test]
    fn test_grant_operator_role() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);
        assert!(client.has_role(&user, &Role::Operator));
    }

    #[test]
    fn test_grant_viewer_role() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Viewer);
        assert!(client.has_role(&user, &Role::Viewer));
    }

    #[test]
    fn test_grant_admin_role_to_user() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Admin);
        assert!(client.has_role(&user, &Role::Admin));
    }

    #[test]
    fn test_grant_multiple_roles_to_same_user() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);
        client.grant_role(&admin, &user, &Role::Viewer);
        assert!(client.has_role(&user, &Role::Operator));
        assert!(client.has_role(&user, &Role::Viewer));
    }

    #[test]
    fn test_grant_role_unauthorized() {
        setup!(env, client, _admin);
        let user = Address::generate(&env);
        let unauthorized = Address::generate(&env);
        let result = client.try_grant_role(&unauthorized, &user, &Role::Operator);
        assert!(result.is_err());
    }

    #[test]
    fn test_grant_role_by_operator_fails() {
        setup!(env, client, admin);
        let operator = Address::generate(&env);
        let target = Address::generate(&env);
        client.grant_role(&admin, &operator, &Role::Operator);
        let result = client.try_grant_role(&operator, &target, &Role::Viewer);
        assert!(result.is_err());
    }

    // =========================================================================
    // role hierarchy — require_role uses has_permission
    // =========================================================================

    #[test]
    fn test_role_hierarchy_super_admin_can_grant_any_role() {
        setup!(env, client, admin);
        let u1 = Address::generate(&env);
        let u2 = Address::generate(&env);
        let u3 = Address::generate(&env);
        let u4 = Address::generate(&env);
        client.grant_role(&admin, &u1, &Role::SuperAdmin);
        client.grant_role(&admin, &u2, &Role::Admin);
        client.grant_role(&admin, &u3, &Role::Operator);
        client.grant_role(&admin, &u4, &Role::Viewer);
        assert!(client.has_role(&u1, &Role::SuperAdmin));
        assert!(client.has_role(&u2, &Role::Admin));
        assert!(client.has_role(&u3, &Role::Operator));
        assert!(client.has_role(&u4, &Role::Viewer));
    }

    #[test]
    fn test_role_hierarchy_admin_can_grant_roles() {
        setup!(env, client, admin);
        let admin_user = Address::generate(&env);
        let target = Address::generate(&env);
        client.grant_role(&admin, &admin_user, &Role::Admin);
        // Admin satisfies the Admin requirement via hierarchy
        client.grant_role(&admin_user, &target, &Role::Operator);
        assert!(client.has_role(&target, &Role::Operator));
    }

    #[test]
    fn test_role_hierarchy_non_admin_cannot_grant() {
        setup!(env, client, admin);
        let viewer = Address::generate(&env);
        let target = Address::generate(&env);
        client.grant_role(&admin, &viewer, &Role::Viewer);
        let result = client.try_grant_role(&viewer, &target, &Role::Viewer);
        assert!(result.is_err());
    }

    #[test]
    fn test_role_hierarchy_operator_cannot_grant() {
        setup!(env, client, admin);
        let operator = Address::generate(&env);
        let target = Address::generate(&env);
        client.grant_role(&admin, &operator, &Role::Operator);
        let result = client.try_grant_role(&operator, &target, &Role::Viewer);
        assert!(result.is_err());
    }

    #[test]
    fn test_role_hierarchy_new_admin_can_grant() {
        setup!(env, client, admin);
        let new_admin = Address::generate(&env);
        let user = Address::generate(&env);
        client.grant_role(&admin, &new_admin, &Role::Admin);
        client.grant_role(&new_admin, &user, &Role::Operator);
        assert!(client.has_role(&user, &Role::Operator));
    }

    #[test]
    fn test_role_hierarchy_super_admin_satisfies_all() {
        setup!(env, client, admin);
        // SuperAdmin (the initializer) can call grant_role which requires Admin
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Viewer);
        assert!(client.has_role(&user, &Role::Viewer));
    }

    #[test]
    fn test_role_hierarchy_admin_satisfies_admin_and_below() {
        setup!(env, client, admin);
        let admin_user = Address::generate(&env);
        client.grant_role(&admin, &admin_user, &Role::Admin);
        // Admin can revoke roles (requires Admin level)
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);
        client.revoke_role(&admin_user, &user, &Role::Operator);
        assert!(!client.has_role(&user, &Role::Operator));
    }

    // =========================================================================
    // role inheritance — higher roles inherit lower role permissions
    // =========================================================================

    #[test]
    fn test_role_inheritance_operator_inherits_viewer_permissions() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        let func = symbol_short!("read");
        client.grant_role(&admin, &user, &Role::Operator);
        // Grant permission only to Viewer role
        client.grant_permission(&admin, &Role::Viewer, &func);
        // Operator should inherit Viewer permissions
        assert!(client.check_permission(&user, &func));
    }

    #[test]
    fn test_role_inheritance_viewer_does_not_inherit_operator_permissions() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        let func = symbol_short!("submit");
        client.grant_role(&admin, &user, &Role::Viewer);
        client.grant_permission(&admin, &Role::Operator, &func);
        // Viewer should NOT have Operator permissions
        assert!(!client.check_permission(&user, &func));
    }

    #[test]
    fn test_role_inheritance_admin_has_all_permissions() {
        setup!(env, client, admin);
        let admin_user = Address::generate(&env);
        client.grant_role(&admin, &admin_user, &Role::Admin);
        let func = symbol_short!("anything");
        // Admin has all permissions (bypass)
        assert!(client.check_permission(&admin_user, &func));
    }

    #[test]
    fn test_role_inheritance_super_admin_has_all_permissions() {
        setup!(env, client, admin);
        let func = symbol_short!("anything");
        // SuperAdmin (initializer) has all permissions
        assert!(client.check_permission(&admin, &func));
    }

    // =========================================================================
    // revoke_role
    // =========================================================================

    #[test]
    fn test_revoke_role() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);
        assert!(client.has_role(&user, &Role::Operator));
        client.revoke_role(&admin, &user, &Role::Operator);
        assert!(!client.has_role(&user, &Role::Operator));
    }

    #[test]
    fn test_revoke_one_role_preserves_others() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);
        client.grant_role(&admin, &user, &Role::Viewer);
        client.revoke_role(&admin, &user, &Role::Operator);
        assert!(!client.has_role(&user, &Role::Operator));
        assert!(client.has_role(&user, &Role::Viewer));
    }

    #[test]
    fn test_revoke_nonexistent_role_is_noop() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.revoke_role(&admin, &user, &Role::Operator);
        assert!(!client.has_role(&user, &Role::Operator));
    }

    #[test]
    fn test_revoke_role_unauthorized() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        let attacker = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);
        let result = client.try_revoke_role(&attacker, &user, &Role::Operator);
        assert!(result.is_err());
        assert!(client.has_role(&user, &Role::Operator));
    }

    // =========================================================================
    // has_role (exact match)
    // =========================================================================

    #[test]
    fn test_has_role_returns_false_for_unknown_user() {
        setup!(env, client, _admin);
        let stranger = Address::generate(&env);
        assert!(!client.has_role(&stranger, &Role::SuperAdmin));
        assert!(!client.has_role(&stranger, &Role::Admin));
        assert!(!client.has_role(&stranger, &Role::Operator));
        assert!(!client.has_role(&stranger, &Role::Viewer));
    }

    #[test]
    fn test_has_role_does_not_cross_contaminate() {
        setup!(env, client, admin);
        let u1 = Address::generate(&env);
        let u2 = Address::generate(&env);
        client.grant_role(&admin, &u1, &Role::Operator);
        assert!(client.has_role(&u1, &Role::Operator));
        assert!(!client.has_role(&u2, &Role::Operator));
    }

    #[test]
    fn test_has_role_is_exact_match() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);
        // has_role is exact match — Operator is not Admin
        assert!(!client.has_role(&user, &Role::Admin));
        assert!(!client.has_role(&user, &Role::SuperAdmin));
        assert!(client.has_role(&user, &Role::Operator));
    }

    // =========================================================================
    // grant_permission
    // =========================================================================

    #[test]
    fn test_grant_permission_to_operator() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        let func = symbol_short!("transfer");
        client.grant_role(&admin, &user, &Role::Operator);
        client.grant_permission(&admin, &Role::Operator, &func);
        assert!(client.check_permission(&user, &func));
    }

    #[test]
    fn test_grant_permission_unauthorized() {
        setup!(env, client, admin);
        let operator = Address::generate(&env);
        client.grant_role(&admin, &operator, &Role::Operator);
        let func = symbol_short!("transfer");
        let result = client.try_grant_permission(&operator, &Role::Viewer, &func);
        assert!(result.is_err());
    }

    #[test]
    fn test_grant_multiple_permissions_to_role() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);
        let f1 = symbol_short!("transfer");
        let f2 = symbol_short!("deposit");
        client.grant_permission(&admin, &Role::Operator, &f1);
        client.grant_permission(&admin, &Role::Operator, &f2);
        assert!(client.check_permission(&user, &f1));
        assert!(client.check_permission(&user, &f2));
    }

    // =========================================================================
    // check_permission
    // =========================================================================

    #[test]
    fn test_check_permission_super_admin_has_all() {
        setup!(env, client, admin);
        let func = symbol_short!("anything");
        assert!(client.check_permission(&admin, &func));
    }

    #[test]
    fn test_check_permission_admin_has_all() {
        setup!(env, client, admin);
        let admin_user = Address::generate(&env);
        client.grant_role(&admin, &admin_user, &Role::Admin);
        let func = symbol_short!("anything");
        assert!(client.check_permission(&admin_user, &func));
    }

    #[test]
    fn test_check_permission_user_without_role_denied() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        let func = symbol_short!("transfer");
        client.grant_permission(&admin, &Role::Operator, &func);
        assert!(!client.check_permission(&user, &func));
    }

    #[test]
    fn test_check_permission_role_without_permission_denied() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);
        let func = symbol_short!("transfer");
        assert!(!client.check_permission(&user, &func));
    }

    #[test]
    fn test_check_permission_after_role_revoked() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        let func = symbol_short!("transfer");
        client.grant_role(&admin, &user, &Role::Operator);
        client.grant_permission(&admin, &Role::Operator, &func);
        assert!(client.check_permission(&user, &func));
        client.revoke_role(&admin, &user, &Role::Operator);
        assert!(!client.check_permission(&user, &func));
    }

    #[test]
    fn test_check_permission_viewer_role() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        let func = symbol_short!("read");
        client.grant_role(&admin, &user, &Role::Viewer);
        client.grant_permission(&admin, &Role::Viewer, &func);
        assert!(client.check_permission(&user, &func));
    }

    #[test]
    fn test_check_permission_does_not_bleed_across_roles() {
        setup!(env, client, admin);
        let operator = Address::generate(&env);
        let viewer = Address::generate(&env);
        let func = symbol_short!("transfer");
        client.grant_role(&admin, &operator, &Role::Operator);
        client.grant_role(&admin, &viewer, &Role::Viewer);
        client.grant_permission(&admin, &Role::Operator, &func);
        assert!(client.check_permission(&operator, &func));
        assert!(!client.check_permission(&viewer, &func));
    }

    // =========================================================================
    // event emission
    // =========================================================================

    #[test]
    fn test_grant_role_emits_event() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);

        let events = env.events().all();
        assert!(!events.is_empty());
        // The last event should be the role_grnt event for the user grant
        // (initialize emits nothing, so only the grant_role event is present)
        let (topics, data): (soroban_sdk::Vec<Val>, RoleGrantedEvent) = events
            .last()
            .map(|(_, t, d)| (t, soroban_sdk::FromVal::from_val(&env, &d)))
            .unwrap();
        assert_eq!(data.user, user);
        assert_eq!(data.admin, admin);
        assert!(matches!(data.role, Role::Operator));
        // First topic is the symbol "role_grnt"
        let topic0: Symbol = soroban_sdk::FromVal::from_val(&env, &topics.get(0).unwrap());
        assert_eq!(topic0, symbol_short!("role_grnt"));
    }

    #[test]
    fn test_revoke_role_emits_event() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        client.grant_role(&admin, &user, &Role::Operator);
        env.events().all(); // clear snapshot reference point

        client.revoke_role(&admin, &user, &Role::Operator);

        let events = env.events().all();
        let revoke_event = events.iter().find(|(_, topics, _)| {
            if topics.is_empty() {
                return false;
            }
            let t: Symbol = soroban_sdk::FromVal::from_val(&env, &topics.get(0).unwrap());
            t == symbol_short!("role_rvk")
        });
        assert!(revoke_event.is_some(), "expected role_rvk event");
        let (_, _, data_val) = revoke_event.unwrap();
        let data: RoleRevokedEvent = soroban_sdk::FromVal::from_val(&env, &data_val);
        assert_eq!(data.user, user);
        assert_eq!(data.admin, admin);
        assert!(matches!(data.role, Role::Operator));
    }

    #[test]
    fn test_revoke_nonexistent_role_emits_no_event() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        // revoke a role the user never had — should be a no-op with no event
        client.revoke_role(&admin, &user, &Role::Operator);

        let events = env.events().all();
        let revoke_event = events.iter().find(|(_, topics, _)| {
            if topics.is_empty() {
                return false;
            }
            let t: Symbol = soroban_sdk::FromVal::from_val(&env, &topics.get(0).unwrap());
            t == symbol_short!("role_rvk")
        });
        assert!(revoke_event.is_none(), "no event expected for no-op revoke");
    }

    #[test]
    fn test_grant_permission_emits_event() {
        setup!(env, client, admin);
        let func = symbol_short!("transfer");
        client.grant_permission(&admin, &Role::Operator, &func);

        let events = env.events().all();
        let perm_event = events.iter().find(|(_, topics, _)| {
            if topics.is_empty() {
                return false;
            }
            let t: Symbol = soroban_sdk::FromVal::from_val(&env, &topics.get(0).unwrap());
            t == symbol_short!("perm_grnt")
        });
        assert!(perm_event.is_some(), "expected perm_grnt event");
        let (_, _, data_val) = perm_event.unwrap();
        let data: PermissionGrantedEvent = soroban_sdk::FromVal::from_val(&env, &data_val);
        assert_eq!(data.admin, admin);
        assert_eq!(data.function, func);
        assert!(matches!(data.role, Role::Operator));
    }

    #[test]
    fn test_grant_role_unauthorized_issue_689() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AccessControlContract);
        let client = AccessControlContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let unauthorized = Address::generate(&env);

        env.mock_all_auths();
        client.initialize(&admin);

        // Should fail - unauthorized user trying to grant role
        let result = client.try_grant_role(&unauthorized, &user, &Role::Admin);
        assert!(result.is_err());
    }

    #[test]
    fn test_role_hierarchy_issue_689() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        let target = Address::generate(&env);

        // Test that admin can grant any role
        client.grant_role(&admin, &user, &Role::Admin);
        assert!(client.has_role(&user, &Role::Admin));

        // Test that non-admin (Viewer) cannot grant roles
        let viewer = Address::generate(&env);
        client.grant_role(&admin, &viewer, &Role::Viewer);
        let result = client.try_grant_role(&viewer, &target, &Role::Operator);
        assert!(result.is_err());
    }

    #[test]
    fn test_revoke_role_issue_689() {
        setup!(env, client, admin);
        let user = Address::generate(&env);

        // Grant and then revoke
        client.grant_role(&admin, &user, &Role::Operator);
        assert!(client.has_role(&user, &Role::Operator));

        client.revoke_role(&admin, &user, &Role::Operator);
        assert!(!client.has_role(&user, &Role::Operator));
    }

    #[test]
    fn test_check_permission_issue_689() {
        setup!(env, client, admin);
        let user = Address::generate(&env);
        let func = symbol_short!("execute");

        client.grant_role(&admin, &user, &Role::Operator);
        client.grant_permission(&admin, &Role::Operator, &func);

        // Test permission checking
        assert!(client.check_permission(&user, &func));

        // Test unauthorized permission checking
        let stranger = Address::generate(&env);
        assert!(!client.check_permission(&stranger, &func));
    }
}
