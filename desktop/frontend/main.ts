type Command = 'estado' | 'ubicacion' | 'progreso' | 'preparar' | 'iniciar' | 'detener' | 'probar' | 'elegir' | 'abrir' | 'vista';
interface Row { name: string; version: string; active: boolean; healthy: boolean; metrics: [number, number] | null; output_bytes: number }
interface Status { preparation: string; ready: boolean; services: Row[]; events: string[]; error: string | null }
interface Progress { component: string; stage: string; completed: number; stages: number; bytes: number; total_bytes: number | null }
declare global { interface Window { __TAURI__: { core: { invoke<T>(cmd: Command, args?:object): Promise<T> } } } }
const invoke = <T>(cmd: Command,args?:object) => window.__TAURI__.core.invoke<T>(cmd,args);
const el = (id: string) => document.getElementById(id)!;
const button = (id: string) => el(id) as HTMLButtonElement;
const bar = (id: string) => el(id) as HTMLProgressElement;
const mib = (bytes: number) => (bytes / 1048576).toFixed(1);
let busy: Command | null = null, last: Status | null = null;
let view='workspace';
const tabs=Array.from(document.querySelectorAll<HTMLButtonElement>('[data-view]'));
async function show(name:string){
  view=name;
  await invoke('vista',{visible:name==='workspace'});
  for(const tab of tabs){el(tab.dataset.view!).hidden=tab.dataset.view!==view;tab.setAttribute('aria-current',String(tab.dataset.view===view));}
}
for(const tab of tabs)tab.onclick=()=>void show(tab.dataset.view!);
el('manage').onclick=()=>void show('stack');
let recent:string[]=[];
try {const stored:unknown=JSON.parse(localStorage.getItem('projects')||'[]'); if(Array.isArray(stored))recent=stored.filter(p=>typeof p==='string').slice(0,16);}catch{}
function projects(){el('recent').replaceChildren(...recent.map(path=>{const li=document.createElement('li'),b=document.createElement('button');b.textContent=path;b.onclick=()=>void openProject(path);li.append(b);return li;}));}
async function openProject(path?:string){
  if(busy)return;
  if(!last?.services.every(s=>s.healthy)){await show('stack');el('feedback').textContent='Prepare e inicie el stack antes de abrir un proyecto.';return;}
  busy='abrir';controls();
  try {const carpeta=path??await invoke<string|null>('elegir'); if(!carpeta)return;
    el('feedback').textContent='Abriendo la interfaz local de OpenCode…';
    await invoke('abrir',{carpeta}); await show('workspace');recent=[carpeta,...recent.filter(p=>p!==carpeta)].slice(0,16);localStorage.setItem('projects',JSON.stringify(recent));projects();el('feedback').textContent='';
  }catch(e){el('feedback').textContent=String(e);}finally{busy=null;controls();}
}
for(const id of ['choose','add'])el(id).onclick=()=>void openProject();projects();

function controls() {
  const active = last?.services.some(s => s.active);
  button('prepare').disabled = busy !== null || !last || !!active;
  button('prepare').textContent = last?.ready ? 'Verificar runtimes' : 'Preparar o reintentar';
  button('start').disabled = busy !== null || !last?.ready || !!active;
  button('stop').disabled = busy !== null || !active;
  button('test').disabled = busy !== null || !last?.services.every(s => s.healthy);
}

async function location() {
  const root = await invoke<string>('ubicacion');
  el('location').textContent = `Instalación: ${root}`;
  for (const [label, dir] of [['Ejecutable', 'bin\\ai-stack-desktop.exe'], ['CLIProxyAPI', 'runtimes\\cpa'],
    ['Node y npm', 'runtimes\\node'], ['OpenCode', 'runtimes\\oc'], ['OmniRoute y SQLite', 'runtimes\\package'],
    ['Plugin OpenCode', 'runtimes\\peers'], ['Configuración', 'config'], ['Datos / WebView2', 'datos'],
    ['Cachés', 'cache'], ['Diagnóstico', 'logs'], ['Temporales', 'preparacion']]) {
    const li = document.createElement('li');
    li.textContent = `${label}: ${root}\\${dir}`;
    el('paths').append(li);
  }
}

async function refresh() {
  const state = await invoke<Status>('estado');
  last = state;
  el('prep').textContent = state.ready ? 'Preparado · integridad W1 comprobada' : `Pendiente · ${state.preparation}`;
  el('error').textContent = state.error || '';
  el('services').replaceChildren(...state.services.map(s => {
    const node = document.createElement('section');
    const name = document.createElement('strong');
    name.textContent = `${s.name} ${s.version}`;
    const info = document.createElement('div');
    info.textContent = [s.active ? (s.healthy ? 'En marcha · sano' : 'En marcha · sin salud confirmada') : 'Detenido',
      ...(s.metrics ? [`${mib(s.metrics[0])} MiB pico`, `${s.metrics[1].toFixed(2)} s CPU`, `${s.output_bytes} B de salida`] : [])].join(' · ');
    node.append(name, info);
    return node;
  }));
  el('events').textContent = state.events.join('\n') || 'Sin eventos.';
  controls();
}

async function updateProgress() {
  const p = await invoke<Progress | null>('progreso');
  if (busy !== 'preparar' || !p) return;
  el('stage').textContent = `${p.component}: ${p.stage}`;
  bar('steps').max = p.stages;
  bar('steps').value = p.completed;
  el('count').textContent = `${p.completed} / ${p.stages} componentes`;
  el('transfer').hidden = p.stage !== 'Descargando';
  if (p.total_bytes) {
    bar('bytes').max = p.total_bytes;
    bar('bytes').value = p.bytes;
  } else bar('bytes').removeAttribute('value');
  el('size').textContent = `${mib(p.bytes)}${p.total_bytes ? ` / ${mib(p.total_bytes)}` : ''} MiB`;
}

for (const [id, cmd, label] of [['prepare', 'preparar', 'Comprobando la preparación…'],
  ['start', 'iniciar', 'Verificando y arrancando W1…'], ['stop', 'detener', 'Cerrando los Jobs…'],
  ['test', 'probar', 'Esperando la respuesta sintética…']] as const) {
  button(id).onclick = async () => {
    if (busy) return;
    busy = cmd;
    controls();
    el('feedback').textContent = label;
    if (cmd === 'preparar') {
      el('progress').hidden = false;
      el('transfer').hidden = true;
      el('stage').textContent = label;
      bar('steps').value = 0;
      el('count').textContent = '';
    }
    try {
      await invoke(cmd);
      el('feedback').textContent = cmd === 'probar' ? 'El proveedor sintético respondió.' : cmd === 'preparar' ? 'Preparación completa. SQLite nativo comprobado.' : '';
    } catch (e) { el('feedback').textContent = String(e); }
    finally {
      el('progress').hidden = true;
      try { await refresh(); } catch { el('error').textContent = 'Sin comunicación con el supervisor'; }
      busy = null;
      controls();
    }
  };
}

async function poll() {
  try {
    if (busy === 'preparar') await updateProgress();
    else if (!busy) await refresh();
  } catch { el('error').textContent = 'Sin comunicación con el supervisor'; }
  finally { setTimeout(poll, busy === 'preparar' ? 300 : 900); }
}
void location().catch(() => { el('location').textContent = 'Ubicación no disponible'; });
void poll();
export {};
