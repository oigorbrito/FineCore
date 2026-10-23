#!/usr/bin/env python3
"""Qualify frozen SWE-bench instances via official gold-patch evaluator.

This driver *never* records agent success and keeps gold material outside any
agent workspace. Qualification requires the official report's resolved flag.
"""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

def evaluate(selection, evaluator, output_dir, dataset_name="SWE-bench/SWE-bench_Verified"):
    data=json.loads(Path(selection).read_text(encoding="utf-8"))
    if data.get("qualification")!="NOT_EXECUTED" or not data.get("snapshot_revision") or not data.get("parquet_sha256"):
        raise ValueError("Unfrozen or already modified selection manifest")
    ids=[x["instance_id"] for x in data["proposed_first"]]
    if not ids or len(ids)!=len(set(ids)):
        raise ValueError("No unique candidates")
    output_dir=Path(output_dir)
    output_dir.mkdir(parents=True,exist_ok=True)
    command=[sys.executable,"-m","swebench.harness.run_evaluation",
      "--dataset_name",dataset_name,"--split","test","--instance_ids",*ids,
      "--predictions_path","gold","--max_workers","1","--run_id","core_gold_qualification"]
    event={"schema_version":1,"status":"QUALIFICATION_PENDING","kind":"OFFICIAL_GOLD_ORACLE",
           "instance_ids":ids,"snapshot_revision":data["snapshot_revision"],
           "parquet_sha256":data["parquet_sha256"],"command":command,
           "warning":"gold reference patches are for qualification only; never expose to agents"}
    (output_dir/"qualification-manifest.json").write_text(json.dumps(event,indent=2)+"\n",encoding="utf-8")
    completed=subprocess.run(command,cwd=evaluator,text=True,stdout=subprocess.PIPE,
                             stderr=subprocess.STDOUT,check=False)
    (output_dir/"evaluator.log").write_text(completed.stdout,encoding="utf-8")
    event["returncode"]=completed.returncode
    event["log_sha256"]=hashlib.sha256(completed.stdout.encode()).hexdigest()
    event["status"]="AWAIT_OFFICIAL_REPORT_INSPECTION" if completed.returncode==0 else "OFFICIAL_EVALUATOR_FAILED"
    (output_dir/"qualification-manifest.json").write_text(json.dumps(event,indent=2)+"\n",encoding="utf-8")
    print(json.dumps({"status":event["status"],"attempted_instances":len(ids),
                      "validated_instances":0,"returncode":completed.returncode}))
    return completed.returncode

def main():
    p=argparse.ArgumentParser()
    p.add_argument("--selection",type=Path,required=True)
    p.add_argument("--evaluator",type=Path,required=True,help="Pinned official SWE-bench source checkout")
    p.add_argument("--output-dir",type=Path,required=True)
    p.add_argument("--dataset-name",default="SWE-bench/SWE-bench_Verified")
    a=p.parse_args()
    if not (a.evaluator/"swebench/harness/run_evaluation.py").is_file():
        p.error("Not the official source checkout")
    raise SystemExit(evaluate(a.selection,a.evaluator,a.output_dir,a.dataset_name))
if __name__=="__main__":main()
