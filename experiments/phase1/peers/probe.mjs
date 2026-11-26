// Prueba de caja negra sobre las exportaciones del tarball, sin un proceso OpenCode.
import assert from 'node:assert/strict';
import { buildStaticProviderEntry, resolveOmniRoutePluginOptions, defaultOmniRouteModelsFetcher, mapRawModelToModelV2 } from '@omniroute/opencode-plugin';
let text = ''; for await (const s of process.stdin) text += s;
const {port, key, model, efforts} = JSON.parse(text);
const baseURL = `http://127.0.0.1:${port}`;
const [row] = await defaultOmniRouteModelsFetcher(baseURL, key, 3000);
assert.equal(row.id, model);
const entry = mapRawModelToModelV2(row, {providerId: 'opencode-omniroute', baseURL});
assert.deepEqual(entry.variants, Object.fromEntries(efforts.map(v => [v, {reasoningEffort:v}])));
const staticModel = buildStaticProviderEntry([row], [], resolveOmniRoutePluginOptions({baseURL}), baseURL, key).models[row.id];
assert.equal(staticModel.variants, undefined);
assert.equal(staticModel.reasoning_options, undefined);
console.log(JSON.stringify({modelo:row.id, variantes:entry.variants}));
