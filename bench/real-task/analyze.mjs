#!/usr/bin/env node
// CORE-BENCH-REAL-001: independent post-run analysis. No model calls.
import fs from 'node:fs';
import path from 'node:path';
import {verifyDirectory} from './verify-efficacy-bundle.mjs';
const [gate, dir] = process.argv.slice(2);
if (!['consumption','speed','efficacy'].includes(gate) || !dir) {
  console.error('Usage: node bench/real-task/analyze.mjs <consumption|speed|efficacy> <results-dir>');
  process.exit(2);
}
const parity = verifyDirectory(dir);
if (!parity.eligible) throw Error(`Ineligible efficacy comparison: ${parity.reason}; ${parity.diagnostics.join('; ')}`);
const files = fs.readdirSync(dir).filter(f => f.endsWith('.json') && !['manifest.json','parity-metadata.json'].includes(f)).sort();
if (!files.length) throw Error('No trial JSON; refuse empty PASS');
const records = files.map(f => {
  const r = JSON.parse(fs.readFileSync(path.join(dir,f),'utf8'));
  const required = ['task_id','trial_id','harness','model_digest','fixture_sha256','oracle_sha256','endpoint','status','oracle_correct','wall_ms','usage'];
  for (const k of required) if (!(k in r)) throw Error(`${f}: missing ${k}`);
  if (!['pi','orca'].includes(r.harness)) throw Error(`${f}: unknown harness`);
  if (typeof r.oracle_correct !== 'boolean') throw Error(`${f}: external oracle boolean required`);
  if (!Number.isFinite(r.wall_ms) || r.wall_ms < 0) throw Error(`${f}: invalid wall_ms`);
  if (typeof r.endpoint !== 'string' || !/^http:\/\/127[.]0[.]0[.]1(?::[0-9]+)?(?:\/|$)/.test(r.endpoint)) throw Error(`${f}: nonlocal endpoint`);
  if (!r.model_digest || !r.fixture_sha256 || !r.oracle_sha256) throw Error(`${f}: unidentified model/fixture/oracle`);
  return r;
});
const pairs = new Map();
for (const r of records) {
  const id = [r.task_id,r.trial_id].join('::');
  const p = pairs.get(id) ?? {};
  if (p[r.harness]) throw Error(`Duplicate trial ${id}/${r.harness}`);
  p[r.harness] = r;
  pairs.set(id,p);
}
for (const [id,p] of pairs) {
  if (!p.pi || !p.orca) throw Error(`Unpaired trial: ${id}`);
  for (const k of ['model_digest','fixture_sha256','oracle_sha256','endpoint']) {
    if (p.pi[k] !== p.orca[k]) throw Error(`Non-equivalent ${k} for ${id}`);
  }
}
const sum = (v) => v.reduce((a,b) => a+b,0);
const aggregate = {};
for (const h of ['pi','orca']) {
  const rows = records.filter(r => r.harness === h);
  const correct = rows.filter(r => r.oracle_correct).length;
  const duration = sum(rows.map(r => r.wall_ms));
  const usageOk = rows.every(r => r.usage && Number.isSafeInteger(r.usage.prompt_tokens) && r.usage.prompt_tokens >= 0 && Number.isSafeInteger(r.usage.completion_tokens) && r.usage.completion_tokens >= 0);
  const prompt = usageOk ? sum(rows.map(r => r.usage.prompt_tokens)) : null;
  const completion = usageOk ? sum(rows.map(r => r.usage.completion_tokens)) : null;
  aggregate[h] = {
    attempted:rows.length, correct, success_rate:correct/rows.length,
    wall_ms_total:duration, wall_ms_per_correct:correct ? duration/correct : null,
    prompt_tokens_total:prompt, completion_tokens_total:completion,
    tokens_per_correct:usageOk && correct ? (prompt+completion)/correct : null,
    unknown_tokens:!usageOk
  };
}
const verdict = 'PILOT_INCONCLUSIVE'; // Never turn this pilot into a winner claim.
const report = {schema_version:1,protocol:'CORE-BENCH-REAL-001',gate,paired_trials:pairs.size,comparison_mode:parity.comparison_mode,native_differences_by_pair:parity.native_differences_by_pair,verdict};
if (gate === 'efficacy') {
  report.metrics = Object.fromEntries(Object.entries(aggregate).map(([h,a]) => [h,{attempted:a.attempted,correct:a.correct,success_rate:a.success_rate}]));
} else if (gate === 'speed') {
  report.metrics = Object.fromEntries(Object.entries(aggregate).map(([h,a]) => [h,{wall_ms_total:a.wall_ms_total,wall_ms_per_correct:a.wall_ms_per_correct}]));
} else {
  report.metrics = Object.fromEntries(Object.entries(aggregate).map(([h,a]) => [h,{prompt_tokens_total:a.prompt_tokens_total,completion_tokens_total:a.completion_tokens_total,tokens_per_correct:a.tokens_per_correct,unknown_tokens:a.unknown_tokens}]));
}
console.log(JSON.stringify(report,null,2));
