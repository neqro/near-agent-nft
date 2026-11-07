# Agent NFT: an NFT that is a wallet, with a guard on what its agent can spend

NEAR Hackathon 2026 · Track 10 (NFTs That Do Something) + Track 06 (Security)

Every NFT in the collection **is** a NEAR sub-account (`<token-id>.<collection>`).
That account has **no keys**; it runs a tiny wallet contract that obeys only whoever owns the NFT *right now*.
Sell the NFT and the wallet, its NEAR and its tokens move with it. An AI agent can spend from the wallet,
but only inside a guard policy (daily limit, allowed contracts/methods) and only after an off-chain rug/honeypot check.

> Status: all 5 days done. Everything below ran on NEAR testnet; tx ids in `docs/TESTNET.md`.

## How control passes on transfer
See [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md).

## Layout
- `contracts/collection`: NEP-171/177/178/181 collection; `nft_mint` creates `<id>.<collection>`, attaches wallet code (NEP-591 global contract), inits it, then mints.
- `contracts/wallet`: per-token wallet (day 2), guard policy (day 3).
- `scripts/`: testnet deploy + demo + agent (day 2-4).

## Build
```bash
cargo near build non-reproducible-wasm --no-abi   # in each contracts/* folder
```

## Try it (testnet, no real money)
Needs `near` CLI, `python3`. Collection: `agentnft-11040702.testnet`. Logged tx ids: `docs/TESTNET.md`.
1. **Do:** `bash scripts/day2_transfer_demo.sh` (mint token, send NEAR + wNEAR into its wallet, transfer NFT).
   **See:** old owner's `withdraw_near` is REJECTED, new owner's succeeds; the balance moved with the NFT.
2. **Do:** `bash scripts/day3_guard_demo.sh` (owner sets agent policy: only `wrap.testnet.near_deposit`, 0.5 NEAR/day).
   **See:** agent wrap passes; `honeypot.testnet.swap` is REJECTED (not in allowlist); a call over the daily limit is REJECTED; after the NFT is sold the old policy no longer works.
3. **Do:** `bash scripts/agent.sh <agent> <wallet> <target> <method> <args-json> <deposit-yocto>` (runs `scripts/risk_check.py` first).
   **See:** `honeypot.testnet` is REFUSED off-chain (no code, risk 140, no tx sent); `wrap.testnet.near_deposit` (risk 40) goes through.
4. **Do:** open `https://testnet.nearblocks.io/txns/<tx>` for any tx in `docs/TESTNET.md`.
   **See:** the same results on-chain.
