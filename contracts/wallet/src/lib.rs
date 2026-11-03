//! Gün 2'de doldurulacak: tokene bağlı anahtarsız cüzdan.
//! Şimdilik yalnız init + görünümler; koleksiyon kontratının mint akışını test etmek için yeterli.
use near_sdk::{env, near, AccountId, PanicOnDefault};

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Wallet {
    pub collection: AccountId,
    pub token_id: String,
}

#[near]
impl Wallet {
    #[init]
    pub fn new(collection: AccountId, token_id: String) -> Self {
        // Yalnız üst kontrat (koleksiyon) başlatabilir: front-running init engeli.
        assert_eq!(env::predecessor_account_id(), collection, "only collection can init");
        Self { collection, token_id }
    }

    pub fn wallet_info(&self) -> (AccountId, String) {
        (self.collection.clone(), self.token_id.clone())
    }
}
