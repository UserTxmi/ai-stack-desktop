// Observador de CI: no modifica payload, no conserva cuerpos, cabeceras ni texto de usuario.
import {appendFileSync} from 'node:fs';
export default function observe(pi){
  const out=process.env.G2_TOOL_REPORT;
  if(!out)return;
  const write=(data)=>appendFileSync(out,JSON.stringify(data)+'\n');
  pi.on('before_provider_request',(e)=>{
    const tools=e.payload?.tools??[];
    write({stage:'pi',count:tools.length,subagent:tools.some((t)=>t?.function?.name==='subagent_run'),
      marker:JSON.stringify(e.payload?.messages??[]).includes('GS-DELEGATE'),bytes:Buffer.byteLength(JSON.stringify(e.payload))});
  });
  pi.on('after_provider_response',(e)=>{write({stage:'response',status:e.status});});
}
