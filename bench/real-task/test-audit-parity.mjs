import test from 'node:test';
import assert from 'node:assert/strict';
import {audit} from './audit-parity.mjs';
const sha='a'.repeat(64), sha2='b'.repeat(64);
function trial(core,patch={}){return {core,task_id:'repo__issue-1',repeat:0,model_id:'model',model_digest:sha,context_tokens:4096,output_tokens_limit:2048,temperature:0,thinking:'off',tool_schema_sha256:sha,system_prompt_sha256:sha,step_limit:12,deadline_ms:120000,transport:'openai-compatible',oracle_id:'unit',oracle_version:'1',tool_choice:'auto',oracle_pass:false,exit_status:'completed',usage:{input:100,output:50},...patch}}
test('paired and fully equal metadata is eligible even when both fail oracle',()=>{const r=audit({trials:[trial('pi'),trial('orca')]});assert.equal(r.eligible,true);assert.equal(r.pair_count,1)});
test('different step and prompt contract invalidates pairing',()=>{const r=audit({trials:[trial('pi'),trial('orca',{step_limit:10,system_prompt_sha256:sha2})]});assert.equal(r.eligible,false);assert.deepEqual(r.pairs[0].mismatches,['system_prompt_sha256','step_limit'])});
test('different generation cap invalidates',()=>{const r=audit({trials:[trial('pi'),trial('orca',{output_tokens_limit:1})]});assert.equal(r.eligible,false);assert.match(r.errors.join(' '),/output_tokens_limit/)});
test('rejects missing cells and duplicate rows',()=>{assert.equal(audit({trials:[trial('pi')]}).eligible,false);assert.equal(audit({trials:[trial('pi'),trial('pi'),trial('orca')]}).eligible,false)});
test('rejects unknown usage and invalid hashes',()=>{const r=audit({trials:[trial('pi',{usage:null}),trial('orca',{model_digest:'invalid'})]});assert.equal(r.eligible,false)});
test('rejects empty payload',()=>assert.equal(audit({}).eligible,false));

test('native-agent comparison preserves native prompts, schemas and step caps',()=>{const r=audit({comparison_mode:'native_agent',trials:[trial('pi'),trial('orca',{step_limit:10,system_prompt_sha256:sha2,tool_schema_sha256:sha2,thinking:'low',transport:'ollama-native'})]});assert.equal(r.eligible,true);assert.deepEqual(r.pairs[0].native_differences,['thinking','tool_schema_sha256','system_prompt_sha256','step_limit','transport'])});
test('native-agent still rejects different actual output budgets',()=>{const r=audit({comparison_mode:'native_agent',trials:[trial('pi'),trial('orca',{output_tokens_limit:1})]});assert.equal(r.eligible,false)});
test('native-agent still rejects different model and oracle',()=>{const r=audit({comparison_mode:'native_agent',trials:[trial('pi'),trial('orca',{oracle_version:'other',model_id:'other'})]});assert.equal(r.eligible,false)});
test('unknown comparison mode fails closed',()=>{assert.equal(audit({comparison_mode:'unknown',trials:[trial('pi'),trial('orca')]}).eligible,false)});
