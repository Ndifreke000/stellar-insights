const VERSION: &str = env!("CARGO_PKG_VERSION");

#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String, Symbol, Vec};

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
    Version,
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
        env.storage().instance().set(&DataKey::Version, &VERSION);
    }

    pub fn grant_role(env: Env, caller: Address, user: Address, role: Role) {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::Admin);

        let mut roles = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Role>>(&DataKey::Roles(user.clone()))
            .unwrap_or(Vec::new(&env));
        roles.push_back(role);
        env.storage()
            .persistent()
            .set(&DataKey::Roles(user), &roles);
    }

    pub fn revoke_role(env: Env, caller: Address, user: Address, role: Role) {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::Admin);

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
                .set(&DataKey::Roles(user), &new_roles);
        }
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

    pub fn grant_permission(env: Env, caller: Address, role: Role, function: Symbol) {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::Admin);

        let mut perms = env
            .storage()
            .persistent()
            .get::<DataKey, Vec<Symbol>>(&DataKey::Permissions(role.clone()))
            .unwrap_or(Vec::new(&env));
        perms.push_back(function);
        env.storage()
            .persistent()
            .set(&DataKey::Permissions(role), &perms);
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

    pub fn getversion(env: Env) -> String {
        String::from_str(&env, VERSION)
    }

    fn require_role(env: &Env, user: &Address, role: Role) {
        if !Self::has_role(env.clone(), user.clone(), role) {
            panic!("Unauthorized: missing required role");
        }
    }

    fn roles_equal(a: &Role, b: &Role) -> bool {
        matches!(
            (a, b),
            (Role::Admin, Role::Admin)
                | (Role::Operator, Role::Operator)
                | (Role::Viewer, Role::Viewer)
        )
    }
}

#[cfg(test)]
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
}
