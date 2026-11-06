#!/usr/bin/env bash
# Agent: risk-check target off-chain, then (only if OK) call wallet.agent_call. Usage: agent.sh <agent> <wallet> <target> <method> <args-json> <deposit-yocto>
set -uo pipefail; D=$(dirname "$0")
A=$1 W=$2 T=$3 M=$4 ARGS=$5 DEP=$6
R=$(python3 "$D/risk_check.py" "$T"); rc=$?; echo "risk: $R"
if [ $rc -ne 0 ]; then echo "AGENT REFUSES: $T flagged risky, no transaction sent"; exit 2; fi
ARGS_ESC=$(python3 -c 'import json,sys;print(json.dumps(sys.argv[1]))' "$ARGS")
near contract call-function as-transaction "$W" agent_call json-args "{\"target\":\"$T\",\"method\":\"$M\",\"args\":$ARGS_ESC,\"deposit\":\"$DEP\"}" prepaid-gas '150 Tgas' attached-deposit '0 NEAR' sign-as "$A" network-config testnet sign-with-legacy-keychain send 2>&1 | grep -E "Transaction ID|REJECTED|rror:"
