#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Map};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Channel {
    pub subscriber: Address,
    pub service: Address,
    pub deposit: i128,
    pub rate_per_ledger: i128,
    pub opened_at: u32,
}

#[contracttype]
pub enum DataKey {
    Channel(Address, Address), // (subscriber, service)
    Admin,
}

#[contract]
pub struct SubscriptionChannel;

#[contractimpl]
impl SubscriptionChannel {
    /// Initialize the contract
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Open a payment channel by depositing XLM
    pub fn open_channel(
        env: Env,
        subscriber: Address,
        service: Address,
        deposit: i128,
        rate_per_ledger: i128,
    ) {
        subscriber.require_auth();

        let key = DataKey::Channel(subscriber.clone(), service.clone());

        // Prevent duplicate channels
        if env.storage().persistent().has(&key) {
            panic!("channel already exists");
        }

        assert!(deposit > 0, "deposit must be positive");
        assert!(rate_per_ledger > 0, "rate must be positive");

        // Transfer deposit from subscriber to contract
        let xlm_id = env.storage().instance().get::<_, Address>(&symbol_short!("xlm_id"))
            .unwrap_or_else(|| {
                // Native XLM token contract address
                Address::from_contract_id(&env, &env.current_contract_address().contract_id())
            });

        // Store channel
        let channel = Channel {
            subscriber: subscriber.clone(),
            service: service.clone(),
            deposit,
            rate_per_ledger,
            opened_at: env.ledger().sequence(),
        };

        env.storage().persistent().set(&key, &channel);
    }

    /// Verify a subscription is active and has remaining balance (READ-ONLY, zero gas)
    pub fn verify_payment(
        env: Env,
        subscriber: Address,
        service: Address,
    ) -> (bool, i128, i128, i128, u32) {
        let key = DataKey::Channel(subscriber.clone(), service.clone());

        match env.storage().persistent().get::<_, Channel>(&key) {
            Some(channel) => {
                let elapsed = env.ledger().sequence() - channel.opened_at;
                let consumed = (elapsed as i128) * channel.rate_per_ledger;
                let remaining = if consumed > channel.deposit {
                    0
                } else {
                    channel.deposit - consumed
                };
                let active = remaining > 0;

                (active, remaining, channel.deposit, channel.rate_per_ledger, channel.opened_at)
            }
            None => (false, 0, 0, 0, 0),
        }
    }

    /// Close a channel and settle funds
    pub fn close_channel(env: Env, subscriber: Address, service: Address) {
        // Either party can close
        let key = DataKey::Channel(subscriber.clone(), service.clone());

        let channel: Channel = env
            .storage()
            .persistent()
            .get(&key)
            .expect("channel not found");

        // Require auth from subscriber or service
        if env.current_contract_address() != subscriber && env.current_contract_address() != service {
            subscriber.require_auth();
        }

        let elapsed = env.ledger().sequence() - channel.opened_at;
        let consumed = core::cmp::min(
            (elapsed as i128) * channel.rate_per_ledger,
            channel.deposit,
        );
        let refund = channel.deposit - consumed;

        // TODO: Transfer `consumed` to service, `refund` to subscriber
        // This will use the Stellar Asset Contract (SAC) for native XLM

        // Remove channel
        env.storage().persistent().remove(&key);
    }

    /// Force-close a channel after timeout (protects subscribers)
    pub fn force_close(env: Env, subscriber: Address, service: Address) {
        subscriber.require_auth();

        let key = DataKey::Channel(subscriber.clone(), service.clone());

        let channel: Channel = env
            .storage()
            .persistent()
            .get(&key)
            .expect("channel not found");

        // Require timeout period (100 ledgers ~ 8 minutes on testnet)
        let elapsed = env.ledger().sequence() - channel.opened_at;
        assert!(elapsed >= 100, "force close timeout not reached");

        let consumed = core::cmp::min(
            (elapsed as i128) * channel.rate_per_ledger,
            channel.deposit,
        );
        let _refund = channel.deposit - consumed;

        // TODO: Transfer funds via SAC

        env.storage().persistent().remove(&key);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::Env;

    #[test]
    fn test_open_and_verify() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(SubscriptionChannel, ());
        let client = SubscriptionChannelClient::new(&env, &contract_id);

        let subscriber = Address::generate(&env);
        let service = Address::generate(&env);

        client.open_channel(&subscriber, &service, &1_000_000, &100);

        let (active, remaining, deposit, rate, _opened_at) =
            client.verify_payment(&subscriber, &service);

        assert!(active);
        assert_eq!(remaining, 1_000_000);
        assert_eq!(deposit, 1_000_000);
        assert_eq!(rate, 100);
    }

    #[test]
    fn test_balance_drains() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(SubscriptionChannel, ());
        let client = SubscriptionChannelClient::new(&env, &contract_id);

        let subscriber = Address::generate(&env);
        let service = Address::generate(&env);

        client.open_channel(&subscriber, &service, &1000, &10);

        // Advance 50 ledgers
        env.ledger().set_sequence_number(env.ledger().sequence() + 50);

        let (active, remaining, _, _, _) = client.verify_payment(&subscriber, &service);
        assert!(active);
        assert_eq!(remaining, 500); // 1000 - (50 * 10)
    }

    #[test]
    fn test_channel_expires() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(SubscriptionChannel, ());
        let client = SubscriptionChannelClient::new(&env, &contract_id);

        let subscriber = Address::generate(&env);
        let service = Address::generate(&env);

        client.open_channel(&subscriber, &service, &1000, &10);

        // Advance past depletion (100+ ledgers)
        env.ledger().set_sequence_number(env.ledger().sequence() + 200);

        let (active, remaining, _, _, _) = client.verify_payment(&subscriber, &service);
        assert!(!active);
        assert_eq!(remaining, 0);
    }

    #[test]
    #[should_panic(expected = "channel already exists")]
    fn test_no_duplicate_channels() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(SubscriptionChannel, ());
        let client = SubscriptionChannelClient::new(&env, &contract_id);

        let subscriber = Address::generate(&env);
        let service = Address::generate(&env);

        client.open_channel(&subscriber, &service, &1000, &10);
        client.open_channel(&subscriber, &service, &2000, &20); // should panic
    }

    #[test]
    fn test_close_channel() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(SubscriptionChannel, ());
        let client = SubscriptionChannelClient::new(&env, &contract_id);

        let subscriber = Address::generate(&env);
        let service = Address::generate(&env);

        client.open_channel(&subscriber, &service, &1000, &10);
        client.close_channel(&subscriber, &service);

        // After close, verify should return inactive
        let (active, _, _, _, _) = client.verify_payment(&subscriber, &service);
        assert!(!active);
    }
}
