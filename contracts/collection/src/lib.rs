//! Agent NFT collection (NEP-171 + NEP-177 + NEP-181).
//!
//! Every token `<id>` owns a NEAR sub-account `<id>.<collection>`.
//! The sub-account gets NO access key; it only runs the wallet contract
//! (attached via a NEP-591 global contract when configured, else embedded wasm),
//! so the only way to move its funds is through wallet code that asks this
//! collection `nft_token(id)` for the CURRENT owner. Transfer the NFT => control moves.
//!
//! Mint flow (atomic from the minter's view):
//!   nft_mint -> [create sub-account + fund + attach code + init(new)] -> on_wallet_created
//!   success: token is minted to receiver; failure: token id released, deposit refunded.
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::{NonFungibleToken, Token, TokenId};
use near_contract_standards::non_fungible_token::core::{NonFungibleTokenCore, NonFungibleTokenResolver};
use near_contract_standards::non_fungible_token::enumeration::NonFungibleTokenEnumeration;
use near_sdk::collections::LazyOption;
use near_sdk::json_types::{Base58CryptoHash, U128};
use near_sdk::store::IterableSet;
use near_sdk::{
    env, log, near, require, AccountId, BorshStorageKey, CryptoHash, Gas, NearToken,
    PanicOnDefault, Promise, PromiseOrValue,
};

/// NEAR sent to each wallet sub-account to cover its own storage.
pub const WALLET_STORAGE_DEPOSIT: NearToken = NearToken::from_millinear(100); // 0.1 NEAR (global code ref)
/// Extra the minter must cover for the NFT record itself.
pub const TOKEN_STORAGE_DEPOSIT: NearToken = NearToken::from_millinear(20); // 0.02 NEAR
const GAS_WALLET_INIT: Gas = Gas::from_tgas(10);
const GAS_CALLBACK: Gas = Gas::from_tgas(30);

#[derive(BorshStorageKey)]
#[near]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
    Pending,
}

/// Where the wallet code comes from.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub enum WalletCode {
    /// NEP-591 immutable global contract, referenced by code hash (recommended: holders can audit it never changes).
    GlobalHash(Base58CryptoHash),
    /// NEP-591 global contract published under an account (upgradable by that account).
    GlobalAccount(AccountId),
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Collection {
    owner_id: AccountId,
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    wallet_code: WalletCode,
    /// token ids whose wallet account is being created (blocks duplicate mints mid-flight)
    pending: IterableSet<TokenId>,
}

#[near]
impl Collection {
    #[init]
    pub fn new(owner_id: AccountId, wallet_code: WalletCode, metadata: Option<NFTContractMetadata>) -> Self {
        let metadata = metadata.unwrap_or(NFTContractMetadata {
            spec: NFT_METADATA_SPEC.to_string(),
            name: "Agent NFT".to_string(),
            symbol: "AGNT".to_string(),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        });
        metadata.assert_valid();
        Self {
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id.clone(),
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            owner_id,
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            wallet_code,
            pending: IterableSet::new(StorageKey::Pending),
        }
    }

    /// Mint `token_id` to `receiver_id` and create its wallet account `<token_id>.<this>`.
    /// Attach at least `mint_cost()`. Open mint (anyone can mint) for the demo.
    #[payable]
    pub fn nft_mint(&mut self, token_id: TokenId, receiver_id: AccountId, token_metadata: TokenMetadata) -> Promise {
        assert_valid_token_id(&token_id);
        require!(self.tokens.owner_by_id.get(&token_id).is_none(), "token already exists");
        require!(self.pending.insert(token_id.clone()), "mint already in progress");
        let deposit = env::attached_deposit();
        require!(deposit >= self.mint_cost(), format!("attach at least {} yocto", self.mint_cost().as_yoctonear()));

        let wallet_id = self.wallet_id_of(&token_id);
        let init_args = near_sdk::serde_json::json!({
            "collection": env::current_account_id(),
            "token_id": token_id,
        })
        .to_string()
        .into_bytes();

        let create = Promise::new(wallet_id.clone()).create_account().transfer(WALLET_STORAGE_DEPOSIT);
        let create = match &self.wallet_code {
            WalletCode::GlobalHash(h) => {
                let hash: CryptoHash = (*h).into();
                create.use_global_contract(hash)
            }
            WalletCode::GlobalAccount(a) => create.use_global_contract_by_account_id(a.clone()),
        };
        create
            .function_call("new".to_string(), init_args, NearToken::from_yoctonear(0), GAS_WALLET_INIT)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_CALLBACK)
                    .on_wallet_created(token_id, receiver_id, token_metadata, env::predecessor_account_id(), U128(deposit.as_yoctonear())),
            )
    }

    #[private]
    pub fn on_wallet_created(
        &mut self,
        token_id: TokenId,
        receiver_id: AccountId,
        token_metadata: TokenMetadata,
        payer: AccountId,
        deposit: U128,
    ) -> Option<Token> {
        self.pending.remove(&token_id);
        let ok = env::promise_result_checked(0, 1024).is_ok();
        if !ok {
            // Sub-account creation/init failed: release id and refund. (Create failures revert the whole batch,
            // so the 0.1 NEAR wallet deposit came back to us as a refund receipt as well.)
            log!("wallet creation failed for {}; refunding {}", token_id, deposit.0);
            Promise::new(payer).transfer(NearToken::from_yoctonear(deposit.0)).detach();
            return None;
        }
        // Mint without the library's own refund (refund_id = None); we measure storage and refund the surplus ourselves.
        let before = env::storage_usage();
        let token = self.tokens.internal_mint_with_refund(token_id.clone(), receiver_id, Some(token_metadata), None);
        let used = env::storage_byte_cost().saturating_mul((env::storage_usage() - before) as u128);
        let spent = WALLET_STORAGE_DEPOSIT.saturating_add(used);
        let surplus = deposit.0.saturating_sub(spent.as_yoctonear());
        if surplus > 0 {
            Promise::new(payer).transfer(NearToken::from_yoctonear(surplus)).detach();
        }
        log!("minted {} with wallet {}", token_id, self.wallet_id_of(&token_id));
        Some(token)
    }

    // ---------- views ----------
    pub fn mint_cost(&self) -> NearToken {
        WALLET_STORAGE_DEPOSIT.saturating_add(TOKEN_STORAGE_DEPOSIT)
    }

    /// The wallet sub-account bound to `token_id` (deterministic, exists only after a successful mint).
    pub fn wallet_of(&self, token_id: TokenId) -> AccountId {
        self.wallet_id_of(&token_id)
    }

    pub fn wallet_code(&self) -> WalletCode {
        self.wallet_code.clone()
    }

    /// Owner-only: point future mints to new wallet code (existing wallets keep theirs if by hash).
    pub fn set_wallet_code(&mut self, wallet_code: WalletCode) {
        require!(env::predecessor_account_id() == self.owner_id, "owner only");
        self.wallet_code = wallet_code;
    }
}

impl Collection {
    fn wallet_id_of(&self, token_id: &str) -> AccountId {
        format!("{}.{}", token_id, env::current_account_id()).parse().expect("invalid wallet id")
    }
}

/// Token ids become account-id prefixes: lowercase a-z 0-9 '-' '_' , 2..=32 chars, no dots.
fn assert_valid_token_id(id: &str) {
    require!(id.len() >= 2 && id.len() <= 32, "token_id length 2..=32");
    require!(
        id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'),
        "token_id must be [a-z0-9_-]"
    );
    let first = id.as_bytes()[0];
    let last = id.as_bytes()[id.len() - 1];
    require!(first.is_ascii_alphanumeric() && last.is_ascii_alphanumeric(), "token_id must start/end alphanumeric");
}

// ---------- NEP-171 / 177 / 181 plumbing ----------
#[near]
impl NonFungibleTokenCore for Collection {
    #[payable]
    fn nft_transfer(&mut self, receiver_id: AccountId, token_id: TokenId, approval_id: Option<u64>, memo: Option<String>) {
        self.tokens.nft_transfer(receiver_id, token_id, approval_id, memo)
    }

    #[payable]
    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<bool> {
        self.tokens.nft_transfer_call(receiver_id, token_id, approval_id, memo, msg)
    }

    fn nft_token(&self, token_id: TokenId) -> Option<Token> {
        self.tokens.nft_token(token_id)
    }
}

#[near]
impl NonFungibleTokenResolver for Collection {
    #[private]
    fn nft_resolve_transfer(
        &mut self,
        previous_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        approved_account_ids: Option<std::collections::HashMap<AccountId, u64>>,
    ) -> bool {
        self.tokens.nft_resolve_transfer(previous_owner_id, receiver_id, token_id, approved_account_ids)
    }
}

#[near]
impl NonFungibleTokenEnumeration for Collection {
    fn nft_total_supply(&self) -> U128 {
        self.tokens.nft_total_supply()
    }
    fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
        self.tokens.nft_tokens(from_index, limit)
    }
    fn nft_supply_for_owner(&self, account_id: AccountId) -> U128 {
        self.tokens.nft_supply_for_owner(account_id)
    }
    fn nft_tokens_for_owner(&self, account_id: AccountId, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
        self.tokens.nft_tokens_for_owner(account_id, from_index, limit)
    }
}

near_contract_standards::impl_non_fungible_token_approval!(Collection, tokens);

#[near]
impl NonFungibleTokenMetadataProvider for Collection {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn acc(s: &str) -> AccountId {
        s.parse().unwrap()
    }

    fn setup() -> Collection {
        let ctx = VMContextBuilder::new().current_account_id(acc("agents.testnet")).build();
        testing_env!(ctx);
        Collection::new(acc("owner.testnet"), WalletCode::GlobalAccount(acc("wallet-code.testnet")), None)
    }

    #[test]
    fn wallet_id_is_subaccount() {
        let c = setup();
        assert_eq!(c.wallet_of("bot-1".into()), acc("bot-1.agents.testnet"));
    }

    #[test]
    #[should_panic(expected = "token_id must be [a-z0-9_-]")]
    fn rejects_dotted_token_id() {
        assert_valid_token_id("evil.attacker");
    }

    #[test]
    #[should_panic(expected = "attach at least")]
    fn mint_requires_deposit() {
        let mut c = setup();
        c.nft_mint("bot-1".into(), acc("alice.testnet"), TokenMetadata::default());
    }

    #[test]
    fn only_owner_changes_code() {
        let mut c = setup();
        testing_env!(VMContextBuilder::new()
            .current_account_id(acc("agents.testnet"))
            .predecessor_account_id(acc("owner.testnet"))
            .build());
        c.set_wallet_code(WalletCode::GlobalAccount(acc("v2.testnet")));
        assert_eq!(c.wallet_code(), WalletCode::GlobalAccount(acc("v2.testnet")));
    }
}
