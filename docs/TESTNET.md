# Testnet deployment log

## Day 2 (2026-11-04): wallet follows the NFT
- Wallet code published as NEP-591 global contract **by hash** `5jyWjrJeZWUDTH5qWQ1m9kLJnBYgg7eszFZNysTEpYQz` (123 KB, ~12.3 NEAR burned once) — tx `HdTeFVkZPaMSz1NpXVodi1PR6Dyu5QmPt8cQagtAum1K`
- Collection `agentnft-11040702.testnet` deployed + `new` — tx `C61kartfZ6kWxaMjbSYNAhwHd4C7qNwZnXYtRDZb9chZ`
- Demo (`scripts/day2_transfer_demo.sh`), token `bot-1`, wallet `bot-1.agentnft-11040702.testnet` (no access keys):

| Step | Result | Tx |
|---|---|---|
| Alice mints bot-1 (wallet account created in same flow) | minted | `5qkMmET1EKCvAi7mnFY7b3uyzYpF1bTnQBdGd7EzDdyv` |
| Alice sends 1 NEAR to the wallet | ok | `A9JpEpuNvAmhrRBbMoniUEGWuGBQfvtutDXRFhiXguyV` |
| Alice sends 0.29875 wNEAR to the wallet | ok | `2eaEtSQsATkMwoHsNPUkxrg1qhxdnZsHs1J3BoobjFj4` |
| Alice transfers NFT to Bob | ok | `87fMdJnnkYssyfzS7paZuedr4rKBqXU3KwkTwLmQKDq6` |
| Alice (old owner) withdraw_near 0.5 | **REJECTED** (not owner) | `r5r1PyHXo3uejf5B8P5yZYhGefJMoScav58GHZ5BxWw` |
| Bob withdraw_near 0.5 | ok | `5xeNZ3qTs7zCysk4djmigHpC2azem94jLW6ZnxTgAG4i` |
| Bob withdraw_ft 0.29875 wNEAR | ok, Bob balance 0.29875 wNEAR | `8CaXy1dg4XxpyRHQEXWe2Vdk2Stp82kvvv3vVLXFR8B` |

Explorer: https://testnet.nearblocks.io/txns/<tx>

## Day 3 (2026-11-05): guard policy
- Wallet v2 global hash `65z5Ucf53V1Wt2hakfLzWiUhFMhD2ULU9PAzbSYaVfnX` (159 KB) — deploy tx `BeBhKrVtCz1898Tqn1wrqXKpSTZ1NA8XyjBqrBR95scy`; `set_wallet_code` tx `ApR6bAjjEzKEW7REcqwFo1dCR7K7Fyks9xvJXQGntYpt` (bot-1 keeps v1).
- Agent account `agent-11050702.alice-agnt-11040702.testnet`; token `bot-2` (`scripts/day3_guard_demo.sh`):

| Step | Result | Tx |
|---|---|---|
| Owner set_agent (only wrap.testnet.near_deposit, 0.5 NEAR/day) | ok | `AMQ4s6KNnWQZd4AZm1rFftFWufmTgfidaPbsAwryj7DY` |
| Agent wraps 0.2 NEAR | ok | `CS333Ab6QUTcsF8eGCBCkRC2RVt67YSJ4ESxFKr5uaYi` |
| Agent calls honeypot.testnet.swap | **REJECTED** not in allowlist | `7n8jcrZMnCgeLrdwckrRZpHJXDsyXhPihyq43agXFU6D` |
| Agent wraps 0.4 more (over daily limit) | **REJECTED** | `3zxfFMYfRXGhdevuv8GPUFdJmLq6Ln6MP8wfJJRAXYs7` |
| NFT sold to Bob, agent retries | **REJECTED** (policy of previous owner) | `2hbnVau4hr2GuUyLCmXrddvxau1365uGQ8onCtKH66NH` |
