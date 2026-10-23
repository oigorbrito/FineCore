#!/usr/bin/env python3
"""Fail-closed, local-only Ollama tool-call eligibility probe. No remote fallback."""
import argparse
import ipaddress
import json
import urllib.parse
import urllib.request
from pathlib import Path

def local_url(value):
    u = urllib.parse.urlsplit(value)
    if u.scheme != "http" or u.username or u.password or u.query or u.fragment or u.path not in ("", "/"):
        raise ValueError("Only HTTP Ollama loopback root allowed")
    try:
        ip = ipaddress.ip_address(u.hostname)
    except (ValueError, TypeError) as exc:
        raise ValueError("Require literal loopback IP, not a DNS hostname") from exc
    if not ip.is_loopback:
        raise ValueError("Nonlocal endpoint is forbidden")
    return value.rstrip("/")

def request(base, route, payload=None):
    data = None if payload is None else json.dumps(payload).encode()
    req = urllib.request.Request(base + route, data=data, headers={"Content-Type":"application/json"})
    with urllib.request.urlopen(req, timeout=120 if data else 5) as resp:
        return json.load(resp)

def preflight(endpoint, model, attempts=3):
    base = local_url(endpoint)
    models = request(base, "/api/tags").get("models", [])
    found = [m for m in models if m.get("name") == model or m.get("model") == model]
    if len(found) != 1 or not found[0].get("digest"):
        raise RuntimeError("Model not installed uniquely or digest missing: no inference performed")
    schema = {"type":"function","function":{"name":"write_file","description":"Write a file","parameters":{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}}}
    rows = []
    for i in range(attempts):
        answer = request(base, "/api/chat", {"model":model,"stream":False,"messages":[{"role":"user","content":"Call write_file to write hello.txt containing exactly hello. No plain-text response."}],"tools":[schema],"options":{"temperature":0}})
        calls = answer.get("message", {}).get("tool_calls") or []
        valid = len(calls) == 1 and calls[0].get("function", {}).get("name") == "write_file"
        args = calls[0].get("function", {}).get("arguments", {}) if valid else {}
        if isinstance(args, str):
            try:
                args = json.loads(args)
            except json.JSONDecodeError:
                args = None
        valid = bool(valid and isinstance(args, dict) and args.get("path") == "hello.txt" and args.get("content") == "hello")
        rows.append({"attempt":i+1,"valid_structured_call":valid,"prompt_eval_count":answer.get("prompt_eval_count"),"eval_count":answer.get("eval_count")})
    return {"gate":"LOCAL_TOOL_CALL_PREFLIGHT","endpoint":base,"model":model,"digest":found[0]["digest"],"attempts":attempts,"results":rows,"eligible":all(r["valid_structured_call"] for r in rows)}

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--endpoint",default="http://127.0.0.1:11434")
    ap.add_argument("--model",required=True)
    ap.add_argument("--attempts",type=int,default=3)
    ap.add_argument("--output",type=Path,required=True)
    args = ap.parse_args()
    if args.attempts != 3:
        ap.error("Exactly three structured-call trials required")
    result = preflight(args.endpoint,args.model,args.attempts)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps({"gate":result["gate"],"eligible":result["eligible"],"digest":result["digest"]}))
    raise SystemExit(0 if result["eligible"] else 3)
if __name__ == "__main__":
    main()
