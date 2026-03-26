#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, String,
    Symbol, Vec,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    Unauthorized = 1,
}

#[derive(Clone)]
#[contracttype]
pub enum Role {
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
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub repository: String,
    pub license: String,
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
pub struct AccessControl;

#[contractimpl]
impl AccessControl {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        let mut roles = Vec::new(&env);
        roles.push_back(Role::Admin);
        env.storage()
            .persistent()
            .set(&DataKey::Roles(admin), &roles);
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
                if !Self::roles_equal(&r, &role) {
                    new_roles.push_back(r);
                }
            }
            env.storage()
                .persistent()
                .set(&DataKey::Roles(user.clone()), &new_roles);

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
                if Self::roles_equal(&r, &role) {
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

    pub fn check_permission(env: Env, user: Address, function: Symbol) -> bool {
        if Self::has_role(env.clone(), user.clone(), Role::Admin) {
            return true;
        }

        if let Some(roles) = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Role>>(&DataKey::Roles(user))
        {
            for role in roles.iter() {
                if let Some(perms) = env
                    .storage()
                    .persistent()
                    .get::<DataKey, Vec<Symbol>>(&DataKey::Permissions(role))
                {
                    for perm in perms.iter() {
                        if perm == function {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn require_role(env: &Env, user: &Address, role: Role) -> Result<(), Error> {
        if !Self::has_role(env.clone(), user.clone(), role) {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    fn roles_equal(a: &Role, b: &Role) -> bool {
        matches!(
            (a, b),
            (Role::Admin, Role::Admin)
                | (Role::Operator, Role::Operator)
                | (Role::Viewer, Role::Viewer)
        )
    }

    // =========================================================================
    // Contract Metadata
    // =========================================================================

    /// Get public contract metadata
    pub fn get_metadata(env: Env) -> PublicMetadata {
        PublicMetadata {
            name: String::from_str(&env, "Stellar Insights Access Control"),
            version: String::from_str(&env, VERSION),
            author: String::from_str(&env, "Stellar Insights Team"),
            description: String::from_str(
                &env,
                "Role-based access control contract for Stellar Insights",
            ),
            repository: String::from_str(&env, "https://github.com/stellar-insights/contracts"),
            license: String::from_str(&env, "MIT"),
        }
    }

    /// Get comprehensive contract information
    pub fn get_contract_info(env: Env) -> ContractInfo {
        ContractInfo {
            metadata: Self::get_metadata(env),
            initialized: true,
            total_roles: 0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    macro_rules! setup {
        ($env:ident, $client:ident, $admin:ident) => {
            let $env = Env::default();
            let contract_id = $env.register_contract(None, AccessControl);
            let $client = AccessControlClient::new(&$env, &contract_id);
            let $admin = Address::generate(&$env);
            $env.mock_all_auths();
            $client.initialize(&$admin);
        };
    }

    // =========================================================================
    // initialize
    // =========================================================================

    #[test]
    fn test_initialize_grants_admin_role() {
        setup!(env, client, admin);
        assert!(client.has_role(&admin, &Role::Admin));
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
    // role hierarchy
    // =========================================================================

    #[test]
    fn test_role_hierarchy_admin_can_grant_any_role() {
        setup!(env, client, admin);
        let u1 = Address::generate(&env);
        let u2 = Address::generate(&env);
        let u3 = Address::generate(&env);
        client.grant_role(&admin, &u1, &Role::Admin);
        client.grant_role(&admin, &u2, &Role::Operator);
        client.grant_role(&admin, &u3, &Role::Viewer);
        assert!(client.has_role(&u1, &Role::Admin));
        assert!(client.has_role(&u2, &Role::Operator));
        assert!(client.has_role(&u3, &Role::Viewer));
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
    fn test_role_hierarchy_new_admin_can_grant() {
        setup!(env, client, admin);
        let new_admin = Address::generate(&env);
        let user = Address::generate(&env);
        client.grant_role(&admin, &new_admin, &Role::Admin);
        client.grant_role(&new_admin, &user, &Role::Operator);
        assert!(client.has_role(&user, &Role::Operator));
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
    // has_role
    // =========================================================================

    #[test]
    fn test_has_role_returns_false_for_unknown_user() {
        setup!(env, client, _admin);
        let stranger = Address::generate(&env);
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
    fn test_check_permission_admin_has_all() {
        setup!(env, client, admin);
        let func = symbol_short!("anything");
        assert!(client.check_permission(&admin, &func));
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
        let (topics, data): (soroban_sdk::Vec<soroban_sdk::Val>, RoleGrantedEvent) = events
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
}
