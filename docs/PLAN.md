# Plan (deadline 2026-11-14 23:59 UTC)
- [x] Day 1 (2026-11-04): verify NEAR docs (sub-account from contract, NEP-591 global contracts, cross-contract), repo skeleton, collection contract (builds to wasm), threat model.
- [ ] Day 2: wallet contract (owner check via nft_token callback, NEAR + NEP-141 withdraw) + testnet deploy + transfer demo.
- [ ] Day 3: guard policy (daily limit, contract/method allowlist, per-owner agent rights).
- [ ] Day 4: off-chain risk check (rug/honeypot) + agent script.
- [ ] Day 5 (<= 2026-11-12): README "do X, see Y", demo script, X post draft, form text.

## Open decisions
- Global contract cost: 10 NEAR per 100 KB is burned. Wallet wasm is ~91 KB now (would burn ~9 NEAR). Shrink it (no serde_json, opt-level z) or embed code per wallet (~0.9 NEAR locked each, refundable). Decide on day 2.
- Native `cargo test` fails in the sandbox (aws-lc-sys/secp256k1 C build); test on testnet / near-sandbox instead.
