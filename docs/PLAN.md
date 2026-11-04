# Plan (deadline 2026-11-14 23:59 UTC)
- [x] Day 1 (2026-11-04): verify NEAR docs (sub-account from contract, NEP-591 global contracts, cross-contract), repo skeleton, collection contract (builds to wasm), threat model.
- [x] Day 2 (2026-11-04): wallet contract (owner check via nft_token callback, NEAR + NEP-141 withdraw, busy lock) + testnet deploy + transfer demo. See docs/TESTNET.md.
- [ ] Day 3: guard policy (daily limit, contract/method allowlist, per-owner agent rights).
- [ ] Day 4: off-chain risk check (rug/honeypot) + agent script.
- [ ] Day 5 (<= 2026-11-12): README "do X, see Y", demo script, X post draft, form text.

## Open decisions
- DECIDED (day 2): global contract by hash. Wallet wasm is 123 KB -> ~12.3 NEAR burned once (testnet: pooled two faucet accounts). Each wallet then needs only 0.1 NEAR instead of ~1.2 NEAR locked for embedded code, so it pays off after ~11 mints. Guard code on day 3 grows the wasm: needs a NEW global hash + `set_wallet_code` (existing bot-1 keeps old code) and ~15 NEAR more testnet funds (faucet 10 NEAR/account; create + delete-account into the deployer).
- Open: if a wallet's `on_owner` callback ever runs out of gas, `busy` stays true (no unlock path). Day 3: add an expiry (block height) to the lock.
- Native `cargo test` fails in the sandbox (aws-lc-sys/secp256k1 C build); test on testnet / near-sandbox instead.
