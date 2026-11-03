# Agent NFT: an NFT that is a wallet, with a guard on what its agent can spend

NEAR Hackathon 2026 · Track 10 (NFTs That Do Something) + Track 06 (Security)

Every NFT in the collection **is** a NEAR sub-account (`<token-id>.<collection>`).
That account has **no keys**; it runs a tiny wallet contract that obeys only whoever owns the NFT *right now*.
Sell the NFT and the wallet, its NEAR and its tokens move with it. An AI agent can spend from the wallet,
but only inside a guard policy (daily limit, allowed contracts/methods) and only after an off-chain rug/honeypot check.

> Status: work in progress (day 1 of 5). See `docs/PLAN.md`.

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

## Try it (testnet) - filled in on day 5
1. Do: ... See: ...
