#!/usr/bin/env python3
"""Pre-register deterministic SWE-bench Verified candidate order. No agent inference.

Input is an official Parquet snapshot downloaded by the operator. Qualification
with the official Docker evaluator is separate; this script never labels a
candidate QUALIFIED. It avoids cherry-picking by recording the full sorted
population before execution.
"""
import argparse
import hashlib
import json
from pathlib import Path

REQUIRED = ("instance_id", "repo", "base_commit", "problem_statement",
            "test_patch", "FAIL_TO_PASS", "PASS_TO_PASS")

def select_rows(rows, count=3):
    seen = set()
    accepted, rejected = [], []
    for row in sorted(rows, key=lambda r: str(r.get("instance_id", ""))):
        instance = str(row.get("instance_id",""))
        reason = next((f"missing_{k}" for k in REQUIRED if not row.get(k)), None)
        if not instance or instance in seen:
            reason = "duplicate_or_missing_instance_id"
        seen.add(instance)
        if reason:
            rejected.append({"instance_id":instance,"reason":reason})
        else:
            accepted.append({"instance_id":instance,"repo":row["repo"],
                             "base_commit":row["base_commit"],"qualification":"PENDING_OFFICIAL_EVALUATOR"})
    return {"candidate_count":len(accepted),"rejected":rejected,
            "candidate_order":accepted,"proposed_first":accepted[:count],
            "qualification":"NOT_EXECUTED"}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parquet",type=Path)
    parser.add_argument("--snapshot-revision",required=True)
    parser.add_argument("--count",type=int,default=3)
    parser.add_argument("--output",type=Path,required=True)
    args=parser.parse_args()
    if args.count < 1: parser.error("count must be positive")
    import pyarrow.parquet as pq
    binary=args.parquet.read_bytes()
    table=pq.read_table(args.parquet,columns=list(REQUIRED))
    selection=select_rows(table.to_pylist(),args.count)
    selection.update({"source":"SWE-bench/SWE-bench_Verified",
        "snapshot_revision":args.snapshot_revision,
        "parquet_sha256":hashlib.sha256(binary).hexdigest(),
        "schema_version":1})
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(selection,indent=2,ensure_ascii=False)+"\n",encoding="utf-8")
    print(json.dumps({"proposed":len(selection["proposed_first"]),
                      "qualified":0,"source_sha256":selection["parquet_sha256"]}))
if __name__ == "__main__": main()
