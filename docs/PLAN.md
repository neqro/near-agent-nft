# Plan (deadline 2026-11-14 23:59 UTC)
- [x] Day 1 (2026-11-04): verify NEAR docs (sub-account from contract, NEP-591 global contracts, cross-contract), repo skeleton, collection contract (builds to wasm), threat model.
- [x] Day 2 (2026-11-04): wallet contract (owner check via nft_token callback, NEAR + NEP-141 withdraw, busy lock) + testnet deploy + transfer demo. See docs/TESTNET.md.
- [x] Day 3 (2026-11-05): guard policy (daily NEAR limit, contract/method allowlist, policy bound to owner, lock expiry 20 blocks). Wallet v2 global hash 65z5Ucf53V1Wt2hakfLzWiUhFMhD2ULU9PAzbSYaVfnX (159 KB). See docs/TESTNET.md.
- [x] Day 4 (2026-11-06): off-chain risk check (scripts/risk_check.py: missing code, blocklist, full-access keys, tiny code, zero-supply token) + scripts/agent.sh (check first, then agent_call). Tested on testnet, see docs/TESTNET.md.
- [x] Day 5 (2026-11-07): README try-it steps, docs/SUBMISSION.md (X post, form text, demo script). Needs user approval: push, X post, form.

## Open decisions
- DECIDED (day 2): global contract by hash. Wallet wasm is 123 KB -> ~12.3 NEAR burned once (testnet: pooled two faucet accounts). Each wallet then needs only 0.1 NEAR instead of ~1.2 NEAR locked for embedded code, so it pays off after ~11 mints. Guard code on day 3 grows the wasm: needs a NEW global hash + `set_wallet_code` (existing bot-1 keeps old code) and ~15 NEAR more testnet funds (faucet 10 NEAR/account; create + delete-account into the deployer).
- DONE day 3: lock now expires after 20 blocks. Limitation: daily limit counts NEAR deposits only, not NEP-141 amounts (agent ft_transfer must be allowlisted deliberately). Day 4 adds the off-chain rug/honeypot check before the agent calls agent_call.
- Faucet (helper.nearprotocol.com) failed on 2026-11-05; funds came from bob's balance. Testnet balances are getting low (collection ~4.8 NEAR, alice ~6, bob ~7).
- Native `cargo test` fails in the sandbox (aws-lc-sys/secp256k1 C build); test on testnet / near-sandbox instead.
- Day 4 limits: heuristics only (no indexer holder-concentration, no swap simulation); two layers: off-chain check then on-chain allowlist.
