"""Backport experimental para el dist npm 3.8.54; no modifica el tarball.

nonStreamingResponse captura errores de semáforo y relanza los demás. El borde HTTP
convierte así un timeout local de cola en error de transporte. Si (y sólo si) el
WeakMap upstream acredita QUEUE_TIMEOUT, devolver el error de cola con no-replay.
Los dos grafos de Turbopack tienen distintos IDs y nombres minificados.
"""
import hashlib
from pathlib import Path

CHUNKS = {
    '_07qw_x1vu-gul._.js': ('d007f1f5610639a4f3b88621c02a4b3f18bd4786acdae3667967454544d4b3fc',0),
    '_0iucxs0cnkog_._.js': ('e635cf188e411360e2270102babbb294f0368c83e3e8a4af531bba009563008c',0),
    '_1h2t8vfsm40kc._.js': ('96610fca04209508d8e9d01e23819d4c1946809bb4be2b9203d4881d8c068fa2',0),
    '_0a_c1ny8f0j3_._.js': ('5fef60ac9460de4f934c8329f3b6fdd367506eb84c66489a8f8badabb7015d3d',1),
    '_2182e77j3cy7d._.js': ('ce0a8d2648ff7333d949c1d071ebe144f4256229bcbfc48027729196329043ad',1),
}


def apply_to(dist):
    staged = []
    for file,(sha,graph) in CHUNKS.items():
        path = Path(dist)/'.build/next/server/chunks'/file
        original = path.read_bytes()
        if hashlib.sha256(original).hexdigest()!=sha:
            raise ValueError('El parche sólo admite los chunks originales W1')
        text=original.decode()
        anchor, lookup, caught, make, carry = (
            ('434833,e=>e.a(async(t,r)=>{try{var n=e.i(631303)', 'e.i(522244),noReplay=e.i(223950)', 'n','en',
             'translatedBody:B,currentModel:l,finalBody:k,providerResponse:j,providerHeaders:D,effectiveServiceTier:c,claudePromptCacheLogMeta:r,reasoningReplayHistory:L,pipelineRecovered:P'),
            ('23221,r=>r.a(async(n,o)=>{try{var i=r.i(730720)', 'r.i(589651),noReplay=r.i(917754)', 'i','ei',
             'translatedBody:U,currentModel:c,finalBody:I,providerResponse:q,providerHeaders:j,effectiveServiceTier:p,claudePromptCacheLogMeta:o,reasoningReplayHistory:B,pipelineRecovered:L')
        )[graph]
        end = 'errorType:"account_semaphore_capacity",errorCode:'+('n' if graph else 't')+'},carry:{'+carry+'}}}throw '+caught
        if text.count(anchor)!=1 or text.count(end)!=1:
            raise ValueError('Estructura inesperada del handler W1')
        fix = (f'const fW1=localLimits.x({caught});if(fW1?.code===localLimits.L){{'
               f'const v={make}(fW1.status,fW1.message,null,fW1.code);'
               f'localLimits.$(v.response,{caught});noReplay.a(v.response);'
               +'return {response:{...v,errorCode:fW1.code},carry:{'+carry+'}}}throw '+caught)
        out=text.replace(anchor,anchor+',localLimits='+lookup).replace(end,end.rsplit('throw ',1)[0]+fix)
        # El catch interior nonStreamingProviderLeg debe conservar el objeto marcado:
        # si lo convierte primero en legError(502), su procedencia ya no se puede recuperar.
        leg, exc, imported = (
            ('203765,e=>e.a(async(t,r)=>{try{var n=e.i(257956)', 'p','e.i(522244)'),
            ('631273,r=>r.a(async(n,o)=>{try{var i=r.i(120445)', 'm','r.i(589651)')
        )[graph]
        catch = f'catch({exc}){{if({exc}&&"object"==typeof {exc}&&("SEMAPHORE_TIMEOUT"'
        if out.count(leg)!=1 or out.count(catch)!=1:
            raise ValueError('Estructura inesperada de nonStreamingProviderLeg')
        out=out.replace(leg,leg+',queueErrors='+imported).replace(catch,
            f'catch({exc}){{if(queueErrors.x({exc})?.code===queueErrors.L)throw {exc};'+catch.split('{',1)[1])
        staged.append((path,out.encode()))
    for path,content in staged:
        path.write_bytes(content)
