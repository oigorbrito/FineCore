import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';

const analyzer = path.resolve('bench/real-task/analyze.mjs');
const root = fs.mkdtempSync(path.join(os.tmpdir(),'core-real-test-'));
function row(harness, overrides={}) {
  return {task_id:'fixed-retry',trial_id:'1',harness,model_digest:digest,fixture_sha256:'fixture-fixed',oracle_sha256:'oracle-fixed',endpoint:'http://127.0.0.1:11434',status:'completed',oracle_correct:true,wall_ms:harness==='pi'?100:90,usage:{prompt_tokens:100,completion_tokens:10},...overrides};
}
const digest='a'.repeat(64);
const parityBase={task_id:'fixed-retry',repeat:1,model_id:'test-model',model_digest:digest,context_tokens:4096,output_tokens_limit:2048,temperature:0,thinking:'off',tool_schema_sha256:digest,system_prompt_sha256:digest,step_limit:12,deadline_ms:120000,transport:'ollama',oracle_id:'test-oracle',oracle_version:'1',tool_choice:'auto',exit_status:'completed',oracle_pass:true,usage:{input:100,output:10}};
function write(a,b) {
  for (const f of fs.readdirSync(root)) fs.unlinkSync(path.join(root,f));
  fs.writeFileSync(path.join(root,'pi.json'),JSON.stringify(a));
  if(b) fs.writeFileSync(path.join(root,'orca.json'),JSON.stringify(b));
  fs.writeFileSync(path.join(root,'parity-metadata.json'),JSON.stringify({trials:[a,b].filter(Boolean).map(r=>({...parityBase,core:r.harness,model_digest:r.model_digest,oracle_pass:r.oracle_correct,usage:r.usage?{input:r.usage.prompt_tokens,output:r.usage.completion_tokens}:null}))}));
}
function run(gate) {return spawnSync(process.execPath,[analyzer,gate,root],{encoding:'utf8'});}
try {
  write(row('pi'),row('orca'));
  for (const gate of ['consumption','speed','efficacy']) {
    const p=run(gate);
    assert.equal(p.status,0,p.stderr);
    const report=JSON.parse(p.stdout);
    assert.equal(report.verdict,'PILOT_INCONCLUSIVE');
    assert.equal(report.paired_trials,1);
  }
  write(row('pi'),null);
  assert.notEqual(run('efficacy').status,0,'unpaired data must fail');
  write(row('pi'),row('orca',{model_digest:'sha256:different'}));
  assert.notEqual(run('speed').status,0,'different model must fail');
  write(row('pi'),row('orca',{endpoint:'https://remote.example/v1'}));
  assert.notEqual(run('consumption').status,0,'remote endpoint must fail');
  write(row('pi',{usage:null}),row('orca',{usage:null}));
  const missing=run('consumption');
  assert.notEqual(missing.status,0,'unknown usage must fail the strict metadata gate rather than be imputed');
  console.log('CORE real-task analyzer contract tests: PASS');
} finally {fs.rmSync(root,{recursive:true,force:true});}
