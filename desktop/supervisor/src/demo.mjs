// Adaptación sin Python del escenario normal de provider.py. Sin cliente saliente.
import http from 'node:http';
let text=''; for await (const part of process.stdin) text+=part;
const {port,key,fixture:{model,efforts,scenarios:{normal}}}=JSON.parse(text);
const server=http.createServer({requestTimeout:5000},async(req,res)=>{
  const send=(code,data)=>res.writeHead(code,{'Content-Type':'application/json'}).end(JSON.stringify(data));
  if(req.headers.authorization!==`Bearer ${key}`)return send(401,{});
  if(req.method==='GET' && req.url==='/v1/models')return send(200,{object:'list',data:[{id:model,object:'model',owned_by:'lab',context_length:32768,max_output_tokens:2048,capabilities:{reasoning:true,effort_tiers:efforts}}]});
  if(req.method!=='POST'||req.url!=='/v1/chat/completions')return send(404,{});
  try {
    let size=0,parts=[];for await(const p of req){size+=p.length;if(size>1048576)return send(413,{});parts.push(p);}
    const d=JSON.parse(Buffer.concat(parts));if(d.model!==model||(d.reasoning_effort && !efforts.includes(d.reasoning_effort)))return send(400,{});
    const base={id:'chatcmpl-lab',model,created:1};if(!d.stream)return send(200,{...base,object:'chat.completion',choices:[{index:0,message:{role:'assistant',content:'LAB'},finish_reason:'stop'}]});
    res.writeHead(200,{'Content-Type':'text/event-stream'});
    for(let i=0;i<=normal.chunks&&!res.destroyed;i++){
      const done=i===normal.chunks,delta=done?{}:{content:'LAB',...(i===0?{role:'assistant'}:{})};
      res.write('data: '+JSON.stringify({...base,object:'chat.completion.chunk',choices:[{index:0,delta,finish_reason:done?'stop':null}]})+'\n\n');
      if(!done)await new Promise(r=>setTimeout(r,normal.interval_ms));
    }
    res.end('data: [DONE]\n\n');
  } catch {if(!res.headersSent)send(400,{});else res.destroy();}
});
server.listen(port,'127.0.0.1');
