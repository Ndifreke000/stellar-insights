use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

#[derive(Clone, Debug)]
#[contracttype]
pub struct AdminTransferEvent {
    pub previous_admin: Address,
    pub new_admin: Address,
    pub transferred_by: Address,
    pub timestamp: u64,
    pub ledger_sequence: u32,
}

#[derive(Clone, Debug)]
#[contracttype]
pub enum DataKey {
    Admin,
}

#[derive(Clone, Debug)]
#[contracttype]
pub enum Error {
    AdminNotSet,
    Unauthorized,
}

#[contract]
pub struct AnalyticsContract;

#[contractimpl]
impl AnalyticsContract {
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
        caller.require_auth();
        
        let previous_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        
        if caller != previous_admin {
            return Err(Error::Unauthorized);
        }
        
        // Update admin
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        
        // ✅ EMIT DETAILED EVENT
        env.events().publish(
            (symbol_short!("admin"), new_admin.clone()),
            AdminTransferEvent {
                previous_admin,
                new_admin: new_admin.clone(),
                transferred_by: caller,
                timestamp: env.ledger().timestamp(),
                ledger_sequence: env.ledger().sequence(),
            },
        );
        
        Ok(())
    }
    
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)
    }
    
    pub fn init_admin(env: Env, initial_admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &initial_admin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    #[test]
    fn test_admin_transfer_event() {
        let env = Env::default();
        env.mock_all_auths();
        
        // Set up test addresses
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        
        // Initialize with first admin
        AnalyticsContract::init_admin(env.clone(), admin1.clone());
        
        // Transfer admin and verify the event is emitted
        let result = AnalyticsContract::set_admin(env.clone(), admin1.clone(), admin2.clone());
        
        assert!(result.is_ok(), "Admin transfer should succeed");
        
        // Verify new admin is set
        let current_admin = AnalyticsContract::get_admin(env.clone());
        assert!(current_admin.is_ok(), "Should be able to get admin");
        assert_eq!(current_admin.unwrap(), admin2, "Admin should be updated to new_admin");
        
        // Verify events were emitted
        let events = env.events().all();
        assert!(!events.is_empty(), "Events should have been published");
        
        // Check the event contains correct data
        let (topics, event_data) = &events[0];
        assert_eq!(topics.len(), 2, "Event should have 2 topics");
    }
    
    #[test]
    fn test_unauthorized_admin_transfer() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        let unauthorized = Address::generate(&env);
        
        // Initialize with first admin
        AnalyticsContract::init_admin(env.clone(), admin1.clone());
        
        // Try to transfer from unauthorized address
        let result = AnalyticsContract::set_admin(env.clone(), unauthorized.clone(), admin2.clone());
        
        assert!(result.is_err(), "Unauthorized address should not be able to transfer admin");
        assert_eq!(result.unwrap_err(), Error::Unauthorized, "Should return Unauthorized error");
    }
    
    #[test]
    fn test_admin_not_set() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        
        // Try to get admin when not set
        let result = AnalyticsContract::get_admin(env.clone());
        
        assert!(result.is_err(), "Should fail when admin not set");
        assert_eq!(result.unwrap_err(), Error::AdminNotSet, "Should return AdminNotSet error");
    }
}
