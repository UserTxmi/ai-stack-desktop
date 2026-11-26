// Proyectar exclusivamente filas ya obtenidas por el transporte autenticado Rust.
import {pathToFileURL} from 'node:url';
import {writeFile} from 'node:fs/promises';
let text='';for await(const p of process.stdin)text+=p;
const {plugin,baseURL,apiKey,target,row,file}=JSON.parse(text);
const {buildStaticProviderEntry,resolveOmniRoutePluginOptions}=await import(pathToFileURL(plugin));
const opts=resolveOmniRoutePluginOptions({providerId:'omniroute',baseURL});
const tiers=row.capabilities.effort_tiers;
if(row.id!==target||[...tiers].sort().join()!=='high,low')throw Error('W1_CATALOGO');
const block=buildStaticProviderEntry([row],[],opts,baseURL+'/v1',apiKey);
block.models[target].reasoning_options=[{type:'effort',values:tiers}];
await writeFile(file,JSON.stringify({provider:{[opts.providerId]:block},tiers}));
