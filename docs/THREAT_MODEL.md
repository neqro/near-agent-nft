# Threat model: NFT transfer (seller tries to keep the money)

Each token `<id>` owns wallet account `<id>.<collection>`. Assets: NEAR balance, NEP-141 tokens, access rights.

| # | Attack by old owner / third party | Mitigation |
|---|---|---|
| 1 | Keeps a full-access key on the wallet account | Wallet is created by the collection contract with **no access keys at all** (locked account). Nobody, including the deployer, can sign for it. |
| 2 | Adds a key later via the wallet contract | Wallet code exposes **no** `add_key` / `deploy` / `delete_account` method. Code is attached by immutable global hash. |
| 3 | Cached ownership: wallet trusts an old owner value | Wallet never stores the owner; every privileged call does a cross-contract `nft_token(id)` and acts only in the callback, if `predecessor == current owner`. |
| 4 | Race: seller starts a withdrawal, then sells (withdraw in flight during transfer) | Ownership is checked in the callback right before the action (same receipt as the transfer-out). A withdrawal whose check ran before the sale settles is the seller's legit pre-sale spend; buyer sees the post-settlement balance. Marketplaces should list `wallet_of(id)` balance as informational only. (Day 2: add per-token `nonce`/lock while an action is pending.) |
| 5 | NFT approval left to the seller's agent | Standard NEP-178: approvals are cleared on every transfer. Agent rights (Day 3 guard policy) are stored per owner and reset when owner changes. |
| 6 | Agent key / allowance survives the sale | Agent policy is keyed by `(owner, agent)`; a new owner starts with an empty policy. |
| 7 | Front-run wallet `new()` init | `create_account + use_global_contract + new()` are one batch from the collection; `new()` also asserts `predecessor == collection`. |
| 8 | Collection owner upgrades wallet code to steal | Wallets reference code **by hash** (immutable). `set_wallet_code` only affects future mints. |
| 9 | Fake token id with dots creates `x.evil.<collection>` | `token_id` restricted to `[a-z0-9_-]`, 2..=32 chars. |
| 10 | Mint failure leaves a paid but tokenless state | Token is minted only in the callback after the wallet account exists; on failure the id is released and the deposit refunded. |
