type Command='estado'|'iniciar'|'detener'|'probar';
interface Row{name:string;version:string;active:boolean;healthy:boolean;metrics:[number,number]|null;output_bytes:number}
interface Status{preparation:string;ready:boolean;services:Row[];events:string[];error:string|null}
declare global{interface Window{__TAURI__:{core:{invoke<T>(cmd:Command):Promise<T>}}}}
const el=(id:string)=>document.getElementById(id)!;
const button=(id:string)=>el(id) as HTMLButtonElement;
let busy=false,last:Status|null=null;
function controls(){button('start').disabled=busy||!!last?.services.some(s=>s.active);button('stop').disabled=busy||!last?.events.length;button('test').disabled=busy||!last?.services.every(s=>s.healthy);}
async function refresh(){const state=await window.__TAURI__.core.invoke<Status>('estado');last=state;el('prep').textContent=state.preparation;el('error').textContent=state.error||'';
  el('services').replaceChildren(...state.services.map(s=>{const node=document.createElement('section');const name=document.createElement('strong');name.textContent=`${s.name} ${s.version}`;const info=document.createElement('div');info.textContent=[s.active?(s.healthy?'En marcha · sano':'En marcha · sin salud confirmada'):'Detenido',...(s.metrics?[`${(s.metrics[0]/1048576).toFixed(1)} MiB pico`,`${s.metrics[1].toFixed(2)} s CPU`,`${s.output_bytes} B de salida`]:[])].join(' · ');node.append(name,info);return node;}));el('events').textContent=state.events.join('\n')||'Sin eventos.';controls();}
for(const [id,cmd,label] of [['start','iniciar','Verificando y arrancando W1…'],['stop','detener','Cerrando los Jobs…'],['test','probar','Esperando la respuesta sintética…']] as const)button(id).onclick=async()=>{if(busy)return;busy=true;controls();el('feedback').textContent=label;try{await window.__TAURI__.core.invoke(cmd);el('feedback').textContent=cmd==='probar'?'El proveedor sintético respondió.':'';}catch(e){el('feedback').textContent=String(e);}finally{busy=false;await refresh();}};
async function poll(){try{if(!busy)await refresh();}catch{el('error').textContent='Sin comunicación con el supervisor';}finally{setTimeout(poll,2500);}}void poll();
export {};
