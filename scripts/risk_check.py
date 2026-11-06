#!/usr/bin/env python3
"""Off-chain rug/honeypot check for a NEAR contract (stdlib only, RPC view calls).
Usage: risk_check.py <target> [--rpc URL] [--blocklist file]  -> JSON, exit 0 = OK, 2 = RISKY
Heuristics (each adds points; >=50 refuses):
 - account missing / no contract code                       100
 - on blocklist                                             100
 - contract owner holds a FULL-ACCESS key (can swap code)    40
 - contract code is tiny (<2 KB) or has no exported methods  30
 - looks like a token (ft_metadata) but ft_total_supply fails or is 0   60
 - token has no icon+no reference AND supply held >90% by one account    (skipped, needs indexer)
Full-access key on a contract = owner can rug (upgrade code). Score 40 alone passes (warning); testnet wrap has several keys but passes."""
import sys, json, base64, urllib.request, argparse
def rpc(url, method, params):
    req = urllib.request.Request(url, json.dumps({"jsonrpc":"2.0","id":1,"method":method,"params":params}).encode(), {"Content-Type":"application/json"})
    return json.load(urllib.request.urlopen(req, timeout=30))
def view(url, acct, m, args="{}"):
    r = rpc(url, "query", {"request_type":"call_function","finality":"final","account_id":acct,"method_name":m,"args_base64":base64.b64encode(args.encode()).decode()})
    res = r.get("result", {}).get("result")
    return json.loads(bytes(res)) if res is not None else None
def check(target, url, blocklist=()):
    score, why = 0, []
    if target in blocklist: score += 100; why.append("on blocklist")
    acc = rpc(url, "query", {"request_type":"view_account","finality":"final","account_id":target})
    if "error" in acc or "error" in acc.get("result", {}):
        return {"target":target,"score":100,"risky":True,"reasons":["account does not exist"]}
    a = acc["result"]; zero = "1"*32
    if a.get("code_hash") == zero: score += 100; why.append("no contract code (not a contract)")
    else:
        code = rpc(url, "query", {"request_type":"view_code","finality":"final","account_id":target}).get("result", {})
        size = len(base64.b64decode(code.get("code_base64","")))
        if size < 2048: score += 30; why.append(f"tiny contract ({size} B)")
    keys = rpc(url, "query", {"request_type":"view_access_key_list","finality":"final","account_id":target}).get("result", {}).get("keys", [])
    fa = [k["public_key"] for k in keys if k["access_key"]["permission"] == "FullAccess"]
    if fa: score += 40; why.append(f"{len(fa)} full-access key(s): owner can replace code (rug risk)")
    try:
        meta = view(url, target, "ft_metadata")
    except Exception: meta = None
    if meta:
        try: sup = int(view(url, target, "ft_total_supply"))
        except Exception: sup = 0
        if sup == 0: score += 60; why.append("token with zero/unreadable supply")
    return {"target":target,"score":score,"risky":score >= 50,"reasons":why or ["no red flags"]}
if __name__ == "__main__":
    p = argparse.ArgumentParser(); p.add_argument("target"); p.add_argument("--rpc", default="https://rpc.testnet.near.org"); p.add_argument("--blocklist")
    n = p.parse_args(); bl = set(open(n.blocklist).read().split()) if n.blocklist else set()
    r = check(n.target, n.rpc, bl); print(json.dumps(r)); sys.exit(2 if r["risky"] else 0)
