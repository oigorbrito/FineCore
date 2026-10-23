#!/usr/bin/env python3
"""CORE-06: one-shot local admission/launch; never runs models except via explicit operator command.
Run on Windows where the existing SWE-bench runner is installed. No background daemon.
"""
import argparse
import json
import os
import pathlib
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timezone

def ollama_running(url="http://127.0.0.1:11434/api/ps", timeout=3):
    with urllib.request.urlopen(url, timeout=timeout) as response:
        if response.status != 200:
            raise RuntimeError(f"Ollama returned HTTP {response.status}")
        payload=json.load(response)
    if not isinstance(payload, dict) or not isinstance(payload.get("models"),list):
        raise ValueError("Ollama /api/ps response missing models list")
    return [m.get("name", "<unknown>") for m in payload["models"]]

def sample_host(samples=12, interval=1, max_cpu=55, min_available_gb=3):
    import psutil
    readings=[]
    for i in range(samples):
        cpu=psutil.cpu_percent(interval=interval)
        available=psutil.virtual_memory().available
        readings.append({"cpu_percent":cpu,"available_bytes":available})
    mean=sum(r["cpu_percent"] for r in readings)/len(readings)
    peak=max(r["cpu_percent"] for r in readings)
    floor=min(r["available_bytes"] for r in readings)
    return {"readings":readings,"mean_cpu":mean,"max_cpu":peak,"min_available_bytes":floor,
            "eligible":mean<=max_cpu and peak<=max_cpu*1.4 and floor>=int(min_available_gb*1024**3)}

def stop_process_tree(process, grace_seconds=3):
    """Stop descendants and direct child, then reap the owned direct child."""
    import psutil
    try:
        parent=psutil.Process(process.pid)
        descendants=parent.children(recursive=True)
    except psutil.NoSuchProcess:
        descendants=[]
    for child in descendants:
        try:
            child.terminate()
        except psutil.NoSuchProcess:
            pass
    try:
        process.terminate()
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=grace_seconds)
    except subprocess.TimeoutExpired:
        try:
            process.kill()
        except ProcessLookupError:
            pass
        process.wait()
    alive=psutil.wait_procs(descendants,timeout=grace_seconds)[1]
    for child in alive:
        try:
            child.kill()
        except psutil.NoSuchProcess:
            pass
    psutil.wait_procs(alive,timeout=grace_seconds)
    return {"descendant_count":len(descendants),"descendants_remaining":[p.pid for p in alive if p.is_running()]}

def run_one(command, cwd, timeout, logfile):
    start=time.monotonic_ns()
    with open(logfile,"w",encoding="utf-8") as stream:
        # New process group/session prevents ordinary shell signals from hitting the runner.
        kwargs={"creationflags":subprocess.CREATE_NEW_PROCESS_GROUP} if os.name=="nt" else {"start_new_session":True}
        process=subprocess.Popen(command,cwd=cwd,stdout=stream,stderr=subprocess.STDOUT,shell=False,**kwargs)
        try:
            code=process.wait(timeout=timeout)
            return {"status":"FINISHED","exit_code":code,"wall_ms":(time.monotonic_ns()-start)/1e6}
        except subprocess.TimeoutExpired:
            cleanup=stop_process_tree(process)
            return {"status":"TIMEOUT","exit_code":None,"wall_ms":(time.monotonic_ns()-start)/1e6,"cleanup":cleanup}

def main(argv=None):
    p=argparse.ArgumentParser(description="Single controlled attempt; never repeats automatically")
    p.add_argument("--results-dir",required=True,type=pathlib.Path)
    p.add_argument("--cwd",required=True,type=pathlib.Path)
    p.add_argument("--timeout",type=int,default=1200)
    p.add_argument("--samples",type=int,default=12)
    p.add_argument("--sample-interval",type=float,default=1)
    p.add_argument("--max-cpu",type=float,default=55)
    p.add_argument("--min-available-gb",type=float,default=3)
    p.add_argument("--allow-loaded-model",action="store_true",help="Only if the model is loaded but provably idle; does not override CPU/memory gate")
    args,command=p.parse_known_args(argv)
    if command[:1]==["--"]:command=command[1:]
    if not command:p.error("Provide existing trial runner command after --")
    if args.timeout<1 or args.samples<2 or args.sample_interval<=0:p.error("Invalid gate or deadline")
    if not args.cwd.is_dir():p.error("Missing --cwd")
    results=args.results_dir.resolve()
    results.mkdir(parents=True,exist_ok=True)
    manifest=results/"admission.json"
    if manifest.exists():p.error("Refusing to overwrite a prior admission/run; use a fresh results directory")
    record={"schema_version":1,"started_at":datetime.now(timezone.utc).isoformat(),
            "command":command,"cwd":str(args.cwd.resolve()),"timeout_seconds":args.timeout,
            "status":"PRECHECK"}
    def save():
        manifest.write_text(json.dumps(record,indent=2,ensure_ascii=False)+"\n",encoding="utf-8")
    save()
    try:
        loaded=ollama_running()
        record["ollama_loaded_models"]=loaded
        if loaded and not args.allow_loaded_model:
            record["status"]="BLOCKED_LOADED_MODEL";save();return 3
        baseline=sample_host(args.samples,args.sample_interval,args.max_cpu,args.min_available_gb)
        record["host_baseline"]=baseline
        if not baseline["eligible"]:
            record["status"]="BLOCKED_RESOURCE_CONTENTION";save();return 3
        # Check load again after monitoring; no model is started by this gate.
        now=ollama_running()
        record["ollama_loaded_models_final"]=now
        if now and not args.allow_loaded_model:
            record["status"]="BLOCKED_LOADED_MODEL";save();return 3
        record.update(run_one(command,args.cwd,args.timeout,results/"trial-runner.log"))
        save()
        if record["status"]!="FINISHED" or record["exit_code"]!=0:return 4
        # Analysis belongs to the caller's real runner output; this tool does not manufacture data.
        return 0
    except Exception as error:
        record["status"]="BLOCKED_PREFLIGHT_ERROR"
        record["error"]=str(error)
        save()
        return 3

if __name__=="__main__":raise SystemExit(main())
