# Submission drafts (NOT sent; user approves push / X post / form)

## X post draft
An NFT that IS a wallet. Sell the NFT, and its NEAR + tokens move to the new owner automatically. The AI agent inside can only spend within a daily limit + allowlist, and an off-chain rug/honeypot check refuses risky targets first.
Built on NEAR testnet for @NEARProtocol #NEARHackathon (Tracks: NFTs + Security)
Repo: https://github.com/neqro/near-agent-nft  Demo tx log: docs/TESTNET.md  [attach screen recording of scripts/day3_guard_demo.sh]

## Form text
Project: Agent NFT. Each NFT mints a keyless sub-account running a wallet contract (NEP-591 global code) that checks the current NFT owner via nft_token on every call. A guard layer gives an agent a daily NEAR limit and contract/method allowlist, bound to the owner (reset on sale), plus an off-chain risk check (missing code, blocklist, full-access keys, tiny code, zero-supply token). Tracks: 10 NFTs, 06 Security. Testnet only. Limits: heuristic risk check, daily limit counts NEAR only.

## Demo script (60-90 s)
1. Mint bot token; send NEAR + wNEAR to wallet. 2. Transfer NFT to Bob; Alice withdraw rejected, Bob succeeds. 3. Agent: honeypot refused, wrap passes, over-limit rejected. 4. Sale resets agent policy.
