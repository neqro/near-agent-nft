//! Keyless per-token wallet: `<token_id>.<collection>`.
//!
//! The account has NO access keys. Its only code is this contract, which never stores an owner:
//! every privileged call asks the collection `nft_token(token_id)` and acts in the callback only if
//! the caller is the CURRENT owner. Transfer the NFT and control of the NEAR / NEP-141 balance moves with it.
//!
//! Flow: withdraw_near / withdraw_ft (1 yocto, sets `busy`) -> collection.nft_token -> on_owner
//!       on_owner: clears `busy`, checks owner, then executes (or logs a rejection, never panics so the lock is freed).
use near_sdk::json_types::U128;
use near_sdk::{
    env, ext_contract, log, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise,
    PromiseError, PromiseOrValue,
};

const GAS_NFT_TOKEN: Gas = Gas::from_tgas(5);
const GAS_ON_OWNER: Gas = Gas::from_tgas(25);
const GAS_FT_TRANSFER: Gas = Gas::from_tgas(10);
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
    /// One privileged action at a time (threat model #4: no overlapping withdrawals around a sale).
    pub busy: bool,
}

#[near]
impl Wallet {
    #[init]
    pub fn new(collection: AccountId, token_id: String) -> Self {
        // Only the parent collection can init (same batch as create_account): blocks front-running init.
        require!(env::predecessor_account_id() == collection, "only collection can init");
        Self { collection, token_id, busy: false }
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
        self.busy = false; // always release the lock; this callback never panics
        let owner = match token {
            Ok(Some(t)) => t.owner_id,
            _ => {
                log!("REJECTED: could not read owner of {}", self.token_id);
                return PromiseOrValue::Value(false);
            }
        };
        if owner != caller {
            log!("REJECTED: {} is not the owner of {} (owner is {})", caller, self.token_id, owner);
            return PromiseOrValue::Value(false);
        }
        match action {
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
    pub fn wallet_info(&self) -> WalletInfo {
        WalletInfo {
            collection: self.collection.clone(),
            token_id: self.token_id.clone(),
            busy: self.busy,
            available_near: U128(self.available().as_yoctonear()),
        }
    }
}

impl Wallet {
    fn start(&mut self, action: Action) -> Promise {
        // 1 yocto = full-access-key signature required (function-call keys cannot attach deposits).
        require!(env::attached_deposit() == NearToken::from_yoctonear(1), "attach exactly 1 yoctoNEAR");
        require!(!self.busy, "another action is pending");
        self.busy = true;
        ext_nft::ext(self.collection.clone())
            .with_static_gas(GAS_NFT_TOKEN)
            .nft_token(self.token_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_ON_OWNER)
                    .on_owner(env::predecessor_account_id(), action),
            )
    }

    /// Spendable NEAR: balance minus storage stake minus a small reserve.
    fn available(&self) -> NearToken {
        let stake = env::storage_byte_cost().saturating_mul(env::storage_usage() as u128);
        env::account_balance().saturating_sub(stake).saturating_sub(STATE_RESERVE)
    }
}
