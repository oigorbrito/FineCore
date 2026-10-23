#!/usr/bin/env node
// CORE-DECISION-003 — Verify paired efficacy records using the two real
// repository contracts. No inference, no synthetic trials, fail closed.
import fs from 'node:fs';
import path from 'node:path';
import {audit} from './audit-parity.mjs';
export function verifyDirectory(dir){
 const files=fs.readdirSync(dir).filter(n=>n.endsWith('.json')&&n!=='manifest.json'&&n!=='parity-metadata.json').sort();
 const diagnostics=[];
 if(!files.length)return {eligible:false,reason:'EMPTY_DATASET',diagnostics:['No trial records']};
 const rows=[];
 for(const name of files){
  let r;
  try{r=JSON.parse(fs.readFileSync(path.join(dir,name),'utf8'))}catch(e){diagnostics.push(`${name}: invalid JSON ${e.message}`);continue}
  for(const field of ['task_id','trial_id','harness','model_digest','fixture_sha256','oracle_sha256','endpoint','status','oracle_correct','wall_ms','usage']){
   if(r[field]===undefined||r[field]===null)diagnostics.push(`${name}: missing ${field}`);
  }
  if(!['pi','orca'].includes(r.harness))diagnostics.push(`${name}: invalid harness`);
  if(typeof r.oracle_correct!=='boolean')diagnostics.push(`${name}: oracle_correct must be boolean`);
  if(!Number.isFinite(r.wall_ms)||r.wall_ms<0)diagnostics.push(`${name}: invalid wall_ms`);
  if(typeof r.endpoint!=='string'||!/^http:\/\/(127\.0\.0\.1|localhost)(:\d+)?(\/|$)/.test(r.endpoint))diagnostics.push(`${name}: endpoint not explicitly local`);
  if(r.usage&&!Number.isSafeInteger(r.usage.prompt_tokens))diagnostics.push(`${name}: missing prompt usage`);
  if(r.usage&&!Number.isSafeInteger(r.usage.completion_tokens))diagnostics.push(`${name}: missing completion usage`);
  rows.push(r);
 }
 const groups=new Map();
 for(const r of rows){const k=JSON.stringify([r.task_id,r.trial_id]);const g=groups.get(k)||{pi:[],orca:[]};if(['pi','orca'].includes(r.harness))g[r.harness].push(r);groups.set(k,g)}
 for(const [key,g] of groups){
  if(g.pi.length!==1||g.orca.length!==1){diagnostics.push(`${key}: unpaired or duplicate core`);continue}
  for(const f of ['model_digest','fixture_sha256','oracle_sha256','endpoint'])if(g.pi[0][f]!==g.orca[0][f])diagnostics.push(`${key}: unequal ${f}`);
 }
 const metadataPath=path.join(dir,'parity-metadata.json');
 if(!fs.existsSync(metadataPath)){
  return {eligible:false,reason:'PARITY_METADATA_MISSING',trials:rows.length,pairs:groups.size,diagnostics:[...diagnostics,'Missing parity-metadata.json; do not infer generation parameters from usage']};
 }
 let meta;
 try{meta=JSON.parse(fs.readFileSync(metadataPath,'utf8'))}catch(e){return {eligible:false,reason:'INVALID_PARITY_METADATA',diagnostics:[...diagnostics,e.message]}}
 const parity=audit(meta);
 for(const error of parity.errors)diagnostics.push(`parity: ${error}`);
 // Require trial IDs to map one-to-one between the original runner and parity metadata.
 const fromRunner=new Set(rows.map(r=>JSON.stringify([r.task_id,String(r.trial_id),r.harness])));
 const fromMeta=new Set((meta.trials||[]).map(r=>JSON.stringify([r.task_id,String(r.repeat),r.core])));
 if(fromRunner.size!==rows.length)diagnostics.push('Duplicate task/trial/core in source records');
 if(fromMeta.size!==meta.trials?.length)diagnostics.push('Duplicate task/repeat/core in metadata');
 for(const id of fromRunner)if(!fromMeta.has(id))diagnostics.push(`metadata missing source ${id}`);
 for(const id of fromMeta)if(!fromRunner.has(id))diagnostics.push(`source missing metadata ${id}`);
 // Metadata must also agree with the observed trial, not only its opposite core.
 const metaByKey=new Map((meta.trials||[]).map(t=>[JSON.stringify([t.task_id,String(t.repeat),t.core]),t]));
 for(const r of rows){
  const key=JSON.stringify([r.task_id,String(r.trial_id),r.harness]);
  const m=metaByKey.get(key);
  if(!m)continue;
  if(m.model_digest!==r.model_digest)diagnostics.push(`${key}: metadata model_digest disagrees with trial`);
  if(m.oracle_pass!==r.oracle_correct)diagnostics.push(`${key}: metadata oracle_pass disagrees with trial`);
  if(m.exit_status!==r.status)diagnostics.push(`${key}: metadata exit_status disagrees with trial`);
  if(r.usage&&m.usage&&(m.usage.input!==r.usage.prompt_tokens||m.usage.output!==r.usage.completion_tokens))diagnostics.push(`${key}: metadata usage disagrees with trial`);
 }

 return {eligible:diagnostics.length===0,comparison_mode:parity.comparison_mode,native_differences_by_pair:parity.pairs.filter(p=>p.native_differences?.length).map(p=>({task_id:p.task_id,repeat:p.repeat,fields:p.native_differences})),reason:diagnostics.length?'INVALID_OR_NON_EQUIVALENT':'ELIGIBLE_PAIRED',trials:rows.length,pairs:groups.size,diagnostics};
}
if(process.argv[1]&&path.resolve(process.argv[1])===path.resolve(new URL(import.meta.url).pathname)){
 if(process.argv.length!==3){console.error('Usage: node bench/real-task/verify-efficacy-bundle.mjs <results-dir>');process.exit(2)}
 try{const result=verifyDirectory(process.argv[2]);console.log(JSON.stringify(result,null,2));process.exitCode=result.eligible?0:1}catch(e){console.error(e.stack);process.exitCode=2}
}
