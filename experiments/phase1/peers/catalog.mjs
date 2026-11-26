// El shim del plugin respeta provider[id] si ya está configurado. Usamos su misma
// proyección y añadimos el contrato oficial de OpenCode 1.19.6, sin modificar el tarball.
import {buildStaticProviderEntry, defaultOmniRouteModelsFetcher, resolveOmniRoutePluginOptions} from '@omniroute/opencode-plugin';
import {writeFile} from 'node:fs/promises';
let text = ''; for await (const s of process.stdin) text += s;
const {baseURL, apiKey, target, file} = JSON.parse(text);
const opts = resolveOmniRoutePluginOptions({providerId:'omniroute',baseURL});
const rows = (await defaultOmniRouteModelsFetcher(baseURL, apiKey)).filter(row => row.id === target);
const tiers = rows[0]?.capabilities?.effort_tiers;
if (rows.length !== 1 || !Array.isArray(tiers) || !tiers.length || !tiers.every(x => typeof x === 'string')) throw Error('W1_NO_CATALOG');
const block = buildStaticProviderEntry(rows, [], opts, baseURL+'/v1', apiKey);
block.models[target].reasoning_options = [{type:'effort',values:tiers}];
await writeFile(file, JSON.stringify({provider:{[opts.providerId]:block}, tiers}), {mode:0o600});
