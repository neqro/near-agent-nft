//! Keyless per-token wallet: `<token_id>.<collection>`.
//!
//! The account has NO access keys. Its only code is this contract, which never stores an owner:
//! every privileged call asks the collection `nft_token(token_id)` and acts in the callback only if
//! the caller is the CURRENT owner. Transfer the NFT and control of the NEAR / NEP-141 balance moves with it.
//!
//! Flow: withdraw_near / withdraw_ft (1 yocto, sets `busy`) -> collection.nft_token -> on_owner
//!       on_owner: clears `busy`, checks owner, then executes (or logs a rejection, never panics so the lock is freed).
use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{
    env, ext_contract, log, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise,
    PromiseError, PromiseOrValue,
};

const GAS_NFT_TOKEN: Gas = Gas::from_tgas(5);
const GAS_ON_OWNER: Gas = Gas::from_tgas(25);
const GAS_FT_TRANSFER: Gas = Gas::from_tgas(10);
const GAS_AGENT_CALL: Gas = Gas::from_tgas(30);
/// Lock expires after this many blocks so a failed callback can never freeze the wallet.
const LOCK_BLOCKS: u64 = 20;
const DAY_NS: u64 = 86_400_000_000_000;
/// Kept on the account on top of its storage stake so it can always pay for its own state.
const STATE_RESERVE: NearToken = NearToken::from_millinear(10);

/// The only field of NEP-171 `Token` we need (serde ignores the rest).
#[near(serializers = [json])]
pub struct TokenOwner {
    pub owner_id: AccountId,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub enum Action {
    Near { receiver_id: AccountId, amount: U128 },
    Ft { ft_contract: AccountId, receiver_id: AccountId, amount: U128 },
    /// Owner grants an agent account limited rights.
    SetAgent { agent: AccountId, daily_limit: U128, rules: Vec<Rule> },
    RevokeAgent { agent: AccountId },
    /// Agent (not owner) calls an allowlisted contract/method, spending at most the daily limit of NEAR.
    AgentCall { contract: AccountId, method: String, args: String, deposit: U128 },
}

/// Allowlist entry: contract + allowed method names (empty list = no methods allowed).
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub struct Rule {
    pub contract: AccountId,
    pub methods: Vec<String>,
}

/// Guard policy, keyed by agent. Bound to the owner who set it: if the NFT changes hands it stops applying.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub struct Policy {
    pub owner: AccountId,
    pub daily_limit: U128,
    pub rules: Vec<Rule>,
    pub day: u64,
    pub spent: U128,
}

#[near(serializers = [json])]
pub struct WalletInfo {
    pub collection: AccountId,
    pub token_id: String,
    pub busy: bool,
    pub available_near: U128,
}

#[ext_contract(ext_nft)]
#[allow(dead_code)]
trait Nft {
    fn nft_token(&self, token_id: String) -> Option<TokenOwner>;
}

#[ext_contract(ext_ft)]
#[allow(dead_code)]
trait Ft {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Wallet {
    pub collection: AccountId,
    pub token_id: String,
    /// One privileged action at a time (threat model #4). Block height until which the lock holds.
    pub busy_until: u64,
    pub policies: LookupMap<AccountId, Policy>,
}

#[near]
impl Wallet {
    #[init]
    pub fn new(collection: AccountId, token_id: String) -> Self {
        // Only the parent collection can init (same batch as create_account): blocks front-running init.
        require!(env::predecessor_account_id() == collection, "only collection can init");
        Self { collection, token_id, busy_until: 0, policies: LookupMap::new(b"p") }
    }

    /// Send NEAR out of the wallet. Caller must be the current NFT owner. Attach exactly 1 yoctoNEAR.
    #[payable]
    pub fn withdraw_near(&mut self, receiver_id: AccountId, amount: U128) -> Promise {
        self.start(Action::Near { receiver_id, amount })
    }

    /// Send a NEP-141 token held by the wallet. Caller must be the current NFT owner. Attach exactly 1 yoctoNEAR.
    #[payable]
    pub fn withdraw_ft(&mut self, ft_contract: AccountId, receiver_id: AccountId, amount: U128) -> Promise {
        self.start(Action::Ft { ft_contract, receiver_id, amount })
    }

    /// Owner: let `agent` act within a policy. Attach 1 yoctoNEAR. Replaces any existing policy of that agent.
    #[payable]
    pub fn set_agent(&mut self, agent: AccountId, daily_limit: U128, rules: Vec<Rule>) -> Promise {
        self.start(Action::SetAgent { agent, daily_limit, rules })
    }

    /// Owner: remove an agent. Attach 1 yoctoNEAR.
    #[payable]
    pub fn revoke_agent(&mut self, agent: AccountId) -> Promise {
        self.start(Action::RevokeAgent { agent })
    }

    /// Agent: call an allowlisted contract/method with up to the daily NEAR limit. `args` is a JSON string.
    /// Needs no deposit; the agent's own account can sign it (even a function-call key).
    pub fn agent_call(&mut self, target: AccountId, method: String, args: String, deposit: U128) -> Promise {
        self.start_checked(Action::AgentCall { contract: target, method, args, deposit }, false)
    }

    /// Accept `ft_transfer_call` deposits (keep everything).
    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> U128 {
        let _ = msg;
        log!("received {} of {} from {}", amount.0, env::predecessor_account_id(), sender_id);
        U128(0)
    }

    #[private]
    pub fn on_owner(
        &mut self,
        caller: AccountId,
        action: Action,
        #[callback_result] token: Result<Option<TokenOwner>, PromiseError>,
    ) -> PromiseOrValue<bool> {
        self.busy_until = 0; // always release the lock; this callback never panics
        let owner = match token {
            Ok(Some(t)) => t.owner_id,
            _ => {
                log!("REJECTED: could not read owner of {}", self.token_id);
                return PromiseOrValue::Value(false);
            }
        };
        if let Action::AgentCall { contract, method, args, deposit } = action {
            return self.run_agent(caller, owner, contract, method, args, deposit);
        }
        if owner != caller {
            log!("REJECTED: {} is not the owner of {} (owner is {})", caller, self.token_id, owner);
            return PromiseOrValue::Value(false);
        }
        match action {
            Action::SetAgent { agent, daily_limit, rules } => {
                log!("owner {} sets agent {} limit {}", owner, agent, daily_limit.0);
                self.policies.insert(agent, Policy { owner, daily_limit, rules, day: 0, spent: U128(0) });
                PromiseOrValue::Value(true)
            }
            Action::RevokeAgent { agent } => {
                self.policies.remove(&agent);
                log!("owner {} revoked agent {}", owner, agent);
                PromiseOrValue::Value(true)
            }
            Action::AgentCall { .. } => PromiseOrValue::Value(false),
            Action::Near { receiver_id, amount } => {
                if amount.0 == 0 || amount.0 > self.available().as_yoctonear() {
                    log!("REJECTED: amount {} exceeds available {}", amount.0, self.available().as_yoctonear());
                    return PromiseOrValue::Value(false);
                }
                log!("owner {} sends {} yoctoNEAR to {}", owner, amount.0, receiver_id);
                PromiseOrValue::Promise(Promise::new(receiver_id).transfer(NearToken::from_yoctonear(amount.0)))
            }
            Action::Ft { ft_contract, receiver_id, amount } => {
                log!("owner {} sends {} of {} to {}", owner, amount.0, ft_contract, receiver_id);
                PromiseOrValue::Promise(
                    ext_ft::ext(ft_contract)
                        .with_attached_deposit(NearToken::from_yoctonear(1))
                        .with_static_gas(GAS_FT_TRANSFER)
                        .ft_transfer(receiver_id, amount, Some(format!("agent-nft wallet {}", self.token_id))),
                )
            }
        }
    }

    // ---------- views ----------
    pub fn agent_policy(&self, agent: AccountId) -> Option<Policy> {
        self.policies.get(&agent).cloned()
    }

    pub fn wallet_info(&self) -> WalletInfo {
        WalletInfo {
            collection: self.collection.clone(),
            token_id: self.token_id.clone(),
            busy: env::block_height() < self.busy_until,
            available_near: U128(self.available().as_yoctonear()),
        }
    }
}

impl Wallet {
    fn start(&mut self, action: Action) -> Promise {
        // 1 yocto = full-access-key signature required (function-call keys cannot attach deposits).
        self.start_checked(action, true)
    }

    fn start_checked(&mut self, action: Action, owner_call: bool) -> Promise {
        // 1 yocto = full-access-key signature required for owner actions.
        if owner_call {
            require!(env::attached_deposit() == NearToken::from_yoctonear(1), "attach exactly 1 yoctoNEAR");
        } else {
            require!(env::attached_deposit().is_zero(), "no deposit");
        }
        require!(env::block_height() >= self.busy_until, "another action is pending");
        self.busy_until = env::block_height() + LOCK_BLOCKS;
        ext_nft::ext(self.collection.clone())
            .with_static_gas(GAS_NFT_TOKEN)
            .nft_token(self.token_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_ON_OWNER)
                    .on_owner(env::predecessor_account_id(), action),
            )
    }

    fn run_agent(&mut self, agent: AccountId, owner: AccountId, contract: AccountId, method: String, args: String, deposit: U128) -> PromiseOrValue<bool> {
        let mut p = match self.policies.get(&agent) {
            Some(p) => p.clone(),
            None => {
                log!("REJECTED: {} has no policy", agent);
                return PromiseOrValue::Value(false);
            }
        };
        if p.owner != owner {
            log!("REJECTED: policy of {} was set by previous owner {}", agent, p.owner);
            self.policies.remove(&agent);
            return PromiseOrValue::Value(false);
        }
        if !p.rules.iter().any(|r| r.contract == contract && r.methods.contains(&method)) {
            log!("REJECTED: {}.{} not in allowlist", contract, method);
            return PromiseOrValue::Value(false);
        }
        let today = env::block_timestamp() / DAY_NS;
        if p.day != today {
            p.day = today;
            p.spent = U128(0);
        }
        let new_spent = p.spent.0.saturating_add(deposit.0);
        if new_spent > p.daily_limit.0 || deposit.0 > self.available().as_yoctonear() {
            log!("REJECTED: daily limit {} exceeded (spent {}, asked {})", p.daily_limit.0, p.spent.0, deposit.0);
            return PromiseOrValue::Value(false);
        }
        p.spent = U128(new_spent);
        self.policies.insert(agent.clone(), p);
        log!("agent {} calls {}.{} deposit {}", agent, contract, method, deposit.0);
        PromiseOrValue::Promise(Promise::new(contract).function_call(
            method,
            args.into_bytes(),
            NearToken::from_yoctonear(deposit.0),
            GAS_AGENT_CALL,
        ))
    }

    /// Spendable NEAR: balance minus storage stake minus a small reserve.
    fn available(&self) -> NearToken {
        let stake = env::storage_byte_cost().saturating_mul(env::storage_usage() as u128);
        env::account_balance().saturating_sub(stake).saturating_sub(STATE_RESERVE)
    }
}
