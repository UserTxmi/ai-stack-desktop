// Instrumento de ensayo: cliente del protocolo JSONL, no ejecuta herramientas.
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {createInterface} from 'node:readline';
import {writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';

const [launcher, phase, result, ...args] = process.argv.slice(2);
const child = spawn(process.execPath, [launcher, '--isolated', '--mode', 'rpc', ...args],
  {stdio: ['pipe', 'pipe', 'ignore']});
const exit = once(child, 'exit');
const pending = new Map();
let seq = 0, deltas = '', reason, end, aborted, toolOK = false;
function rpc(type, fields = {}) {
  const id = String(++seq);
  const answer = new Promise((ok, fail) => pending.set(id, {ok, fail}));
  child.stdin.write(JSON.stringify({type, id, ...fields})+'\n');
  return answer;
}
createInterface({input: child.stdout}).on('line', line => {
  let e; try { e = JSON.parse(line); } catch { return; }
  if (e.type === 'response' && pending.has(e.id)) {
    const p = pending.get(e.id); pending.delete(e.id);
    if (e.success) p.ok(e.data); else p.fail(new Error('RPC: '+e.command));
  }
  if (e.type === 'message_update' && e.assistantMessageEvent?.type === 'text_delta') {
    deltas += e.assistantMessageEvent.delta;
    if (phase === 'cancel' && !aborted) aborted = rpc('abort');
  }
  if (e.type === 'tool_execution_end' && e.toolName === 'subagent_run') toolOK = !e.isError;
  if (e.type === 'message_end' && e.message?.role === 'assistant') reason = e.message.stopReason;
  if (e.type === 'agent_end') end?.();
});
const watchdog = setTimeout(() => process.exit(2), 115_000);
async function probe() {
  const state = await rpc('get_state');
  const output = {};
  if (phase === 'resume') {
    const {messages} = await rpc('get_messages');
    assert(messages.some(m => m.role === 'user' && JSON.stringify(m.content).includes('GS-LOW')));
    assert(messages.some(m => m.role === 'assistant' && m.content.some(p => p.text === 'LABLABLAB')));
    output.sessionId = state.sessionId;
  } else {
    const {models} = await rpc('get_available_models');
    assert.deepEqual(models.map(m => m.provider+'/'+m.id).sort(),
      ['omniroute/child/lab-reasoner','omniroute/main/lab-reasoner']);
    assert.equal(state.model.id, 'main/lab-reasoner');
    assert.deepEqual((await rpc('get_available_thinking_levels')).levels, ['low','high']);
    await rpc('set_thinking_level', {level: phase === 'high' ? 'high' : 'low'});
    const done = new Promise(resolve => {end = resolve});
    await rpc('prompt', {message:'GS-'+phase.toUpperCase()});
    await done;
    if (phase === 'cancel') {assert(aborted && deltas.includes('LAB')); await aborted; assert.equal(reason,'aborted');}
    else {assert.equal(reason,'stop'); assert.equal(deltas,'LABLABLAB');}
    if (phase === 'delegate') assert(toolOK);
    const after = await rpc('get_state');
    Object.assign(output, {sessionId:after.sessionId, sessionFile:after.sessionFile});
  }
  writeFileSync(result, JSON.stringify(output));
}
try {
  await Promise.race([probe(), exit.then(() => {throw new Error('launcher terminó antes de la prueba')})]);
  child.stdin.end();
  await exit;
} catch (e) {
  // Sólo marcas de prueba y línea propia; nunca serializar respuestas o estado de Pi.
  console.error(phase, e.name, e.stack?.split('\n').find(l => l.includes('rpc.mjs:'))?.match(/rpc\.mjs:\d+/)?.[0]);
  writeFileSync(result, JSON.stringify({failure:e.stack?.match(/rpc\.mjs:(\d+)/)?.[1] ?? 'exit'}));
  process.exitCode = 1;
} finally { clearTimeout(watchdog); child.stdin.end(); }
