#!/usr/bin/env node
/**
 * CORE-DECISION-002: strict metadata parity gate for agent efficacy trials.
 * Does not infer equality from output metrics or mark a trial as successful.
 * Input: JSON {trials:[{core,task_id,repeat,model_digest,model_id,
 *  context_tokens,output_tokens_limit,temperature,thinking,tool_schema_sha256,
 *  system_prompt_sha256,step_limit,deadline_ms,transport,oracle_id,
 *  oracle_version,tool_choice,exit_status,oracle_pass,usage:{input,output}}]}.
 */
import {readFileSync} from 'node:fs';
const REQUIRED=['task_id','repeat','model_id','model_digest','context_tokens','output_tokens_limit','temperature','thinking','tool_schema_sha256','system_prompt_sha256','step_limit','deadline_ms','transport','oracle_id','oracle_version','tool_choice'];
const NATIVE_FIELDS=['thinking','tool_schema_sha256','system_prompt_sha256','step_limit','transport','tool_choice'];
const SHARED_FIELDS=REQUIRED.filter(k=>!NATIVE_FIELDS.includes(k));
const numeric=['context_tokens','output_tokens_limit','temperature','step_limit','deadline_ms'];
const goodSha=v=>typeof v==='string'&&/^[0-9a-f]{64}$/i.test(v);
export function audit(doc){
 const errors=[],groups=new Map();
 const mode=doc?.comparison_mode??'isolated_core';
 if(!['isolated_core','native_agent'].includes(mode))errors.push('invalid comparison_mode');
 if(!doc||!Array.isArray(doc.trials))return {eligible:false,errors:['missing trials array'],pairs:[]};
 for(const [i,t] of doc.trials.entries()){
  if(!t||typeof t!=='object'){errors.push(`trial ${i}: not object`);continue}
  if(!['pi','orca'].includes(t.core))errors.push(`trial ${i}: core must be pi or orca`);
  for(const k of REQUIRED)if(t[k]===undefined||t[k]===null||t[k]==='')errors.push(`trial ${i}: missing ${k}`);
  for(const k of numeric)if(t[k]!==undefined&&(!Number.isFinite(t[k])||t[k]<0))errors.push(`trial ${i}: invalid ${k}`);
  for(const k of ['model_digest','tool_schema_sha256','system_prompt_sha256'])if(t[k]!==undefined&&!goodSha(t[k]))errors.push(`trial ${i}: invalid ${k}`);
  if(!Number.isInteger(t.repeat)||t.repeat<0)errors.push(`trial ${i}: invalid repeat`);
  if(typeof t.oracle_pass!=='boolean')errors.push(`trial ${i}: missing oracle_pass boolean`);
  if(typeof t.exit_status!=='string')errors.push(`trial ${i}: missing exit_status string`);
  if(!t.usage||!Number.isSafeInteger(t.usage.input)||!Number.isSafeInteger(t.usage.output)||t.usage.input<0||t.usage.output<0)errors.push(`trial ${i}: missing or invalid usage`);
  const key=JSON.stringify([t.task_id,t.repeat]);if(!groups.has(key))groups.set(key,{key,pi:[],orca:[]});
  if(['pi','orca'].includes(t.core))groups.get(key)[t.core].push(t)
 }
 const pairs=[];
 for(const g of groups.values()){
  if(g.pi.length!==1||g.orca.length!==1){errors.push(`pair ${g.key}: requires exactly one pi and one orca (found ${g.pi.length}/${g.orca.length})`);continue}
  const a=g.pi[0],b=g.orca[0],mismatches=[];
  for(const k of (mode==='native_agent'?SHARED_FIELDS:REQUIRED))if(JSON.stringify(a[k])!==JSON.stringify(b[k]))mismatches.push(k);
  const native_differences=NATIVE_FIELDS.filter(k=>JSON.stringify(a[k])!==JSON.stringify(b[k]));
  if(mismatches.length)errors.push(`pair ${g.key}: mismatch ${mismatches.join(',')}`);
  pairs.push({task_id:a.task_id,repeat:a.repeat,parity_ok:mismatches.length===0,mismatches,native_differences,
   pi_correct:a.oracle_pass,orca_correct:b.oracle_pass,
   pi_total_tokens:a.usage?.input+a.usage?.output,
   orca_total_tokens:b.usage?.input+b.usage?.output})
 }
 return {schema_version:1,comparison_mode:mode,eligible:errors.length===0,pair_count:pairs.length,errors,pairs};
}
if(process.argv[1]&&import.meta.url===new URL('file://'+process.argv[1]).href){
 if(process.argv.length!==3){console.error('Usage: node bench/real-task/audit-parity.mjs trials.json');process.exit(2)}
 try{const result=audit(JSON.parse(readFileSync(process.argv[2],'utf8')));console.log(JSON.stringify(result,null,2));process.exitCode=result.eligible?0:1}
 catch(e){console.error(e.message);process.exitCode=2}
}
