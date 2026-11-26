// Observador sin mutación del limitador: sólo dos firmas literales de OmniRoute W1.
const fs = require('node:fs');
const original = console.log;
const append = fields => fs.appendFileSync(process.env.W1_QUEUE_TRACE, JSON.stringify(fields)+'\n');
const warn = console.warn;
console.warn = function (...args) {
  if(args.some(s => typeof s==='string' && s.includes('retrying same account once'))) append({kind:'transport-retry'});
  return warn.apply(this,args);
};
console.log = function (...args) {
  for (const arg of args) if (typeof arg === 'string') {
    const budget = arg.match(/\[RATE-LIMIT\] cid=\S+ provider=\S+ remainingForQueue=(\d+)ms/);
    const expired = arg.match(/Request exceeded queue budget maxWaitMs=(\d+)ms/);
    const clamp = arg.match(/executionMaxWaitMs (\d+)ms clamped to upstream (\d+)ms/);
    if (budget || expired) append({kind:budget?'admission':'expired',ms:Number((budget||expired)[1])});
    if (clamp) append({kind:'execution',ms:Number(clamp[2])});
  }
  return original.apply(this,args);
};
