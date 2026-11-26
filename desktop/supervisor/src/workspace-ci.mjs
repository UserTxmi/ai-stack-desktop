// Misma API v1 utilizada por el SDK de la vista oficial. Salida pública: sólo etapa.
import assert from 'node:assert/strict';
let input='';for await(const p of process.stdin)input+=p;
const {url,session,directory}=JSON.parse(input),origin=new URL(url).origin;
const wait=ms=>new Promise(r=>setTimeout(r,ms));let stage='CI_AUTH';
try {
  assert.equal((await fetch(origin+'/global/health')).status,401);
  const entry=await fetch(url,{redirect:'manual'});assert.equal(entry.status,303);
  const cookie=entry.headers.get('set-cookie').split(';')[0];
  const request=(path,method='GET',data,signal)=>fetch(origin+path,{method,headers:{cookie,origin,'content-type':'application/json'},body:data===undefined?undefined:JSON.stringify(data),signal:signal??AbortSignal.timeout(90000)});
  const api=async(path,method,data)=>{const r=await request(path,method,data);assert.ok(r.ok);return r.status===204?null:r.json()};
  assert.equal((await fetch(url,{redirect:'manual'})).status,403);
  assert.equal((await request('/assets/does-not-exist.js')).status,404);
  assert.equal((await request('/global/config','PATCH',{})).status,404);
  assert.equal((await request('/session?directory=C%3A%5C')).status,403);
  stage='CI_ACTIVOS';const html=await (await request('/')).text();
  assert.ok(html.includes('<title>OpenCode</title>'));for(const [,path] of html.matchAll(/(?:src|href)="(\/assets\/[^" ]+)"/g)){const r=await request(path);assert.ok(r.ok);assert.ok((await r.arrayBuffer()).byteLength>1024)}
  stage='CI_CATALOGO';assert.equal((await api('/global/health')).healthy,true);
  const providerID='opencode-omniroute',modelID='labcpa/lab-reasoner',model={providerID,modelID};
  const p=(await api('/provider')).all.find(p=>p.id===providerID);assert.deepEqual(Object.keys(p.models[modelID].variants).sort(),['high','low']);
  const agents=await api('/agent');for(const name of ['build','plan'])assert.ok(agents.some(a=>a.name===name));
  if(session){stage='CI_HISTORIAL';const sess=await api('/session/'+session);assert.equal(sess.directory.toLowerCase(),directory.toLowerCase());assert.ok((await api('/session')).some(s=>s.id===session));assert.ok((await api(`/session/${session}/message`)).some(m=>m.parts.some(p=>p.type==='text'&&p.text.includes('LAB'))));}
  else {
    stage='CI_SESION';const {id}=await api('/session','POST',{title:'Workspace CI'});
    const turn=(text,agent,variant)=>({agent,model,variant,parts:[{type:'text',text}]});
    const base=`/session/${id}`;
    const message=await api(base+'/message','POST',turn('LAB','plan','low'));assert.ok(!message.info.error);assert.ok(message.parts.some(p=>p.type==='text'&&p.text.includes('LAB')));
    stage='CI_SSE';const ctl=new AbortController();const stream=await request('/global/event','GET',undefined,ctl.signal);
    assert.match(stream.headers.get('content-type'),/event-stream/);let delta=false;
    const observe=(async()=>{const decoder=new TextDecoder();let buf='';for await(const chunk of stream.body){buf+=decoder.decode(chunk,{stream:true}).replaceAll('\r','');let end;while((end=buf.indexOf('\n\n'))>=0){const frame=buf.slice(0,end);buf=buf.slice(end+2);for(const line of frame.split('\n'))if(line.startsWith('data:')){const ev=JSON.parse(line.slice(5)).payload;if(ev?.type==='message.part.delta'&&ev.properties.sessionID===id&&ev.properties.delta.includes('LAB'))delta=true;}}}})().catch(()=>{});
    await api(base+'/prompt_async','POST',turn('LAB-LENTO','build','high'));
    for(let n=0;n<240&&!delta;n++)await wait(100);assert.ok(delta);
    stage='CI_CANCELAR';await api(base+'/abort','POST',{});ctl.abort();await observe;
    const until=async(check)=>{for(let i=0;i<150;i++){const v=await check();if(v)return v;await wait(150)}throw Error('plazo')};
    await until(async()=>{const m=await api(base+'/message');return m.some(x=>x.info.error?.name==='MessageAbortedError')});
    stage='CI_PERMISO';const tool=(await api('/session','POST',{title:'Permiso CI'})).id;
    await api(`/session/${tool}/prompt_async`,'POST',turn('LAB-PERMISO','build','high'));
    const perm=await until(async()=>(await api('/permission')).find(p=>p.sessionID===tool));
    assert.equal(perm.permission,'bash');await api(`/session/${tool}/permissions/${perm.id}`,'POST',{response:'reject'});
    await until(async()=>(await api(`/session/${tool}/message`)).some(m=>m.parts.some(p=>p.type==='tool'&&p.state.status==='error')));
    process.stdout.write(id);
  }
}catch{console.error(stage);process.exitCode=1}
