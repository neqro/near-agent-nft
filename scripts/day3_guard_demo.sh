#!/usr/bin/env bash
# Day 3 demo (testnet): guard policy. Owner grants an agent: allowlist wrap.testnet.near_deposit, 0.5 NEAR/day.
# Usage: COLL= ALICE= BOB= AGENT= TOKEN=bot-2 ./scripts/day3_guard_demo.sh  (keys in legacy keychain; HOME for near CLI)
set -uo pipefail
: "${COLL:?}" "${ALICE:?}" "${BOB:?}" "${AGENT:?}"; TOKEN=${TOKEN:-bot-2}; FT=wrap.testnet; W="$TOKEN.$COLL"
NET="network-config testnet"
call() { near contract call-function as-transaction "$2" "$3" json-args "$4" prepaid-gas '150 Tgas' attached-deposit "$5" \
  sign-as "$1" $NET sign-with-legacy-keychain send 2>&1 | grep -E "Transaction ID|REJECTED|agent .* calls|sets agent|revoked|rror:" ; }
echo "1) Alice mints $TOKEN and funds the wallet with 2 NEAR"
call "$ALICE" "$COLL" nft_mint "{\"token_id\":\"$TOKEN\",\"receiver_id\":\"$ALICE\",\"token_metadata\":{\"title\":\"Agent $TOKEN\"}}" '0.13 NEAR'
near tokens "$ALICE" send-near "$W" '2 NEAR' $NET sign-with-legacy-keychain send 2>&1 | grep "Transaction ID"
echo "2) Alice (owner) sets policy for $AGENT: only $FT.near_deposit, 0.5 NEAR/day"
call "$ALICE" "$W" set_agent "{\"agent\":\"$AGENT\",\"daily_limit\":\"500000000000000000000000\",\"rules\":[{\"contract\":\"$FT\",\"methods\":[\"near_deposit\"]}]}" '1 yoctoNEAR'
echo "3) Agent wraps 0.2 NEAR (inside policy) -> OK"
call "$AGENT" "$W" agent_call "{\"target\":\"$FT\",\"method\":\"near_deposit\",\"args\":\"{}\",\"deposit\":\"200000000000000000000000\"}" '0 NEAR'
echo "4) Agent calls a non-allowlisted 'honeypot' contract -> REJECTED"
call "$AGENT" "$W" agent_call "{\"target\":\"honeypot.testnet\",\"method\":\"swap\",\"args\":\"{}\",\"deposit\":\"100000000000000000000000\"}" '0 NEAR'
echo "5) Agent wraps 0.4 more (0.6 > 0.5/day) -> REJECTED"
call "$AGENT" "$W" agent_call "{\"target\":\"$FT\",\"method\":\"near_deposit\",\"args\":\"{}\",\"deposit\":\"400000000000000000000000\"}" '0 NEAR'
echo "6) Alice sells NFT to Bob; agent tries again -> REJECTED (policy belonged to old owner)"
call "$ALICE" "$COLL" nft_transfer "{\"receiver_id\":\"$BOB\",\"token_id\":\"$TOKEN\"}" '1 yoctoNEAR'
call "$AGENT" "$W" agent_call "{\"target\":\"$FT\",\"method\":\"near_deposit\",\"args\":\"{}\",\"deposit\":\"100000000000000000000000\"}" '0 NEAR'
