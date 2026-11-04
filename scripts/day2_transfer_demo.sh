#!/usr/bin/env bash
# Day 2 demo (testnet): money in the NFT's wallet follows the NFT.
# Usage: COLL=<collection>.testnet ALICE=<a>.testnet BOB=<b>.testnet TOKEN=bot-1 ./scripts/day2_transfer_demo.sh
# Needs near-cli-rs (`near`) with ALICE and BOB keys in the legacy keychain (~/.near-credentials).
set -uo pipefail
: "${COLL:?}" "${ALICE:?}" "${BOB:?}"; TOKEN=${TOKEN:-bot-1}; FT=${FT:-wrap.testnet}
NET="network-config testnet"
call() { # signer contract method json deposit
  near contract call-function as-transaction "$2" "$3" json-args "$4" prepaid-gas '150 Tgas' attached-deposit "$5" \
    sign-as "$1" $NET sign-with-legacy-keychain send 2>&1 | grep -E "Transaction ID|REJECTED|owner .* sends|minted|Error|rror:" ; }
view() { near contract call-function as-read-only "$1" "$2" json-args "$3" $NET now 2>&1 | sed -n '/^{/,/^}/p;/^"/p;/^null/p;/^\[/p' ; }
WALLET="$TOKEN.$COLL"
echo "1) Alice mints $TOKEN; collection creates keyless account $WALLET"
call "$ALICE" "$COLL" nft_mint "{\"token_id\":\"$TOKEN\",\"receiver_id\":\"$ALICE\",\"token_metadata\":{\"title\":\"Agent $TOKEN\"}}" '0.13 NEAR'
view "$WALLET" wallet_info '{}'
near account list-keys "$WALLET" $NET now 2>&1 | grep -iE "key|no access" | head -3
echo "2) Alice funds the NFT: 1 NEAR + 0.3 wNEAR"
near tokens "$ALICE" send-near "$WALLET" '1 NEAR' $NET sign-with-legacy-keychain send 2>&1 | grep "Transaction ID"
call "$ALICE" "$FT" storage_deposit "{\"account_id\":\"$WALLET\"}" '0.00125 NEAR'
call "$ALICE" "$FT" storage_deposit "{\"account_id\":\"$ALICE\"}" '0.00125 NEAR'
call "$ALICE" "$FT" near_deposit '{}' '0.3 NEAR'
call "$ALICE" "$FT" ft_transfer "{\"receiver_id\":\"$WALLET\",\"amount\":\"300000000000000000000000\"}" '1 yoctoNEAR'
echo "3) Alice sells/transfers the NFT to Bob"
call "$ALICE" "$COLL" nft_transfer "{\"receiver_id\":\"$BOB\",\"token_id\":\"$TOKEN\"}" '1 yoctoNEAR'
echo "4) Alice (old owner) tries to drain the wallet -> must be REJECTED"
call "$ALICE" "$WALLET" withdraw_near "{\"receiver_id\":\"$ALICE\",\"amount\":\"500000000000000000000000\"}" '1 yoctoNEAR'
echo "5) Bob (new owner) withdraws 0.5 NEAR and 0.3 wNEAR"
call "$BOB" "$WALLET" withdraw_near "{\"receiver_id\":\"$BOB\",\"amount\":\"500000000000000000000000\"}" '1 yoctoNEAR'
call "$BOB" "$FT" storage_deposit "{\"account_id\":\"$BOB\"}" '0.00125 NEAR'
call "$BOB" "$WALLET" withdraw_ft "{\"ft_contract\":\"$FT\",\"receiver_id\":\"$BOB\",\"amount\":\"300000000000000000000000\"}" '1 yoctoNEAR'
view "$WALLET" wallet_info '{}'
view "$FT" ft_balance_of "{\"account_id\":\"$BOB\"}"
