# Fase 1A · informe de ejecución, 26-11-2026

**Resultado:** laboratorio preparado en [experiments/phase1](../experiments/phase1). En Linux se verificaron los artefactos y el plugin publicado contra un receptor sintético; se midió el comportamiento del receptor, **no el del stack Windows**. No se consultó ni ejecutó la instalación Linux de referencia. La validación 1B y la elección de shell están pendientes.

## Conjunto experimental W1

| Cambio desde la referencia | Evidencia y razón de selección |
|---|---|
| CLIProxyAPI 8.1.6 → **8.1.7** | [Comparación de etiquetas](https://github.com/router-for-me/CLIProxyAPI/compare/v8.1.6...v8.1.7): tres commits, correcciones de orden de partes Gemini y preservación de routing al reconfigurar plugins. Patch pequeño dentro de v8; el ZIP x64 contiene `cli-proxy-api.exe`, ejemplo v8 y LICENSE. No implica que se pruebe Gemini. |
| OmniRoute 3.8.50 → **3.8.54** | [Diferencias](https://github.com/cheaper-inference/OmniRoute/compare/v3.8.50...v3.8.54), [release](https://github.com/cheaper-inference/OmniRoute/releases/tag/v3.8.54): cambios en readiness, routing, failover optativo, modelos y distribución Electron. El cambio determinante es la separación de presupuestos descrita abajo. Crear conexión ahora la deja inactiva: el ensayo comprueba `POST /api/providers/{id}/test` antes de sincronizar. No usa el escritorio de OmniRoute ni su instalador de CPA. |
| Node **24.21.0** | Mantener esta variable de control: encaja en `engines` de OmniRoute. ZIP privado con npm 11.19.0. El tarball publicado de OmniRoute contiene `dist/server-ws.mjs` y `dist/server.js`, pero no `node_modules`. El lock upstream fija Next 16.4.2 y `better-sqlite3@13.0.3`. El módulo incluye `prebuilds/win32-x64.node`: PE AMD64, SHA-256 `e21e5efd71fba66578e95b62554d9028064a80dafd7221bf8a8ef155de8d240a`. Comprobar el PE no equivale a cargarlo. |
| OpenCode 1.18.31 → **1.19.6** | [Comparación](https://github.com/anomalyco/opencode/compare/v1.18.31...v1.19.6): variantes derivadas de opciones de razonamiento, cambios de timeout de Cloudflare y rutas de plugins en Windows. [Release](https://github.com/anomalyco/opencode/releases/tag/v1.19.6): Desktop ofrece descargar OpenCode 2; no actualizar durante la comparación. Cambian también tratamiento de imágenes y compactación tras cuerpo excesivo. Misma versión CLI y Desktop; probar empaquetado offline, no inferirlo. |
| Plugin **0.2.2**, peer **1.19.6** | Artefacto específico de [npm](https://registry.npmjs.org/@omniroute%2fopencode-plugin/0.2.2), no el código de una rama. Descargar catálogo y proyectar variantes funcionó en Node Linux. Queda por confirmar qué envía el agente a través de OmniRoute y CPA. |

Los URL exactos, versiones, licencias de primer nivel y hashes están en [versions.json](../experiments/phase1/versions.json); árbol OmniRoute fijado con el SHA-256 del lock, plugin/peers con [lock local](../experiments/phase1/peers/package-lock.json), shells del ensayo A con [Cargo.lock](../experiments/phase1/ui/Cargo.lock). El laboratorio conserva los avisos de los paquetes. No es un SBOM del instalador definitivo.

## `RATE_LIMIT_EXECUTION_TIMEOUT` y la regresión de 15 s

**Verificado en código, no diagnosticado en una cuenta real:** en [3.8.50, `rateLimitManager.ts`](https://github.com/cheaper-inference/OmniRoute/blob/v3.8.50/open-sse/services/rateLimitManager.ts#L561-L570) `maxWaitMs` se pasa al control de admisión y también a `expiration` del trabajo Bottleneck. Este último reloj empieza **después de despachar el trabajo**. Una expiración con `legacy resilienceSettings.requestQueue.maxWaitMs=15000ms` genera `RATE_LIMIT_EXECUTION_TIMEOUT`; por sí sola no prueba 15 s esperando turno, un 429 remoto ni una conexión TCP fallida.

| Reloj | Cómo distinguirlo en W1 |
|---|---|
| Admisión / espera en cola | Tiempo hasta que el receptor obtiene la petición. [3.8.54](https://github.com/cheaper-inference/OmniRoute/blob/v3.8.54/open-sse/services/rateLimitManager.ts#L676-L735) usa presupuesto restante de cola y cancela el temporizador al despachar. Control positivo: dos peticiones en una conexión, concurrencia 1, primer trabajo sin cabeceras durante 18 s; exigir `RATE_LIMIT_QUEUE_TIMEOUT` en la segunda antes de que llegue al receptor. |
| Ejecución local | En 3.8.54, Bottleneck usa `executionMaxWaitMs` global o de conexión; si hay timeout del ejecutor aplicable se usa el mayor. **No es necesariamente el tiempo total del cuerpo SSE:** depende de hasta cuándo el trabajo retiene el slot. [Clasificación local](https://github.com/cheaper-inference/OmniRoute/blob/v3.8.54/open-sse/services/rateLimitManager/errors.ts): ejecución 504, cola 503, cola llena 429. Aplicar por API **solo al perfil de ensayo** 15 000/90 000 ms y releer antes de probar. |
| Transporte / conexión | El establecimiento TCP/TLS es anterior a la recepción HTTP; un connect timeout o `ECONNREFUSED` no es tiempo en cola. `providerSpecificData.timeoutMs` es una opción por conexión lógica en OmniRoute, **no** una medida pura del handshake TCP. [upstreamTimeouts.ts](https://github.com/cheaper-inference/OmniRoute/blob/v3.8.54/open-sse/handlers/chatCore/upstreamTimeouts.ts) distingue espera de cabeceras y lectura de chunks. Las sondas usan loopback sin TLS: no miden un handshake WAN. |
| Proveedor y stream | El upstream puede contestar 504 explícito; control `provider504` responde con código sintético distinto. Un stream puede dar 200 y cortarse después. Exigir contenido y `[DONE]`, no solo HTTP 200 ni keepalives. Medir por separado cabeceras, primer contenido y fin. En `cancel`, cerrar el consumidor y observar desconexión efectiva en el receptor antes de 6 s. |

**Regresión W1:** [`late` y `body`](../experiments/phase1/fixtures.json) atraviesan OmniRoute→CPA→fixture: el primero tarda 18 s en enviar cabeceras y otro segundo en enviar contenido; el segundo entrega cabeceras inmediatamente y demora 18 s el primer contenido. Ambos continúan más de 20 s con chunks espaciados. La sonda verifica tiempos, todos los fragmentos y una única petición recibida. Un error de ejecución observado es fallo, no indicación de subir límites o usar otra cuenta. El control comparativo con 3.8.50 es de **fuente**; no se ejecutó esa versión contra el fixture.

## Evidencia por estado

**Ejecutadas y aprobadas (Linux, sin servicios reales):**

- `python3 -B experiments/phase1/prepare.py --inventory`: los nueve artefactos descargados en `.lab/downloads` coinciden con sus SHA-256/SRI previstos; verificado cruce de los locks para SQLite y plugin. El noveno fija el árbol fuente por commit: el tarball npm omite `packages/browser-pool` y `packages/ui-kit`, aunque el lock los enlaza. El preparador completa esos workspaces antes de `npm ci`. Inspección interna del ZIP/PE sin ejecutar .exe.
- `python3 -B experiments/phase1/test_fixture.py`: autenticación, catálogo, dos esfuerzos y rechazo inválido, cancelación, 504 sintético, SSE prolongado y llamada de herramienta sintética. En la primera corrida cronometrada, `body`: cabeceras **0,001 s**, primer contenido **18,001 s**, fin **39,004 s**; `late`: **18,001 / 19,001 / 40,004 s**. Son controles del instrumento en loopback. No son tiempos de OmniRoute.
- `test_plugin.py`: Node Linux 26.13.1, `npm ci --ignore-scripts` local con lock. `defaultOmniRouteModelsFetcher` del paquete real leyó `/v1/models` del fixture; `mapRawModelToModelV2` devolvió exactamente `low → {reasoningEffort:low}` y `high → {reasoningEffort:high}`. Sin OpenCode, sin inferencia y sin credenciales administrativas.
- Resolución previa del árbol OmniRoute con `npm ci --dry-run --os=win32 --cpu=x64 --legacy-peer-deps` y locks verificados, sin ejecutar scripts ni simular una aprobación Windows. Cargo generó el lock del comparador; no se compiló el host en Linux.

**Ejecutada y fallida:** la primera simulación `npm ci --dry-run` sin política de peers terminó `ERESOLVE`: `marked-terminal@7.3.0` pide `marked <16` y la raíz fija `18.0.11`. El [`.npmrc` de v3.8.54](https://github.com/cheaper-inference/OmniRoute/blob/v3.8.54/.npmrc) define `legacy-peer-deps=true`. Se conservan versiones; `prepare.py` reproduce esa opción solo para ese lock. La segunda simulación pasó. Sigue sin demostrar que Next o los addons arranquen en Windows.

**Incidencia de adquisición:** una invocación de inventario con límite externo de 120 s no terminó durante la descarga ampliada. Al repetir con mayor plazo se verificaron los nueve hashes y los cuatro manifiestos de los workspaces. No se ejecutó ningún binario descargado para Windows en Linux.

**Preparadas, no ejecutadas en Windows:**

| Exp. | Instrumento y aceptación en 1B | Entorno adecuado |
|---|---|---|
| E1 Inventario | `prepare.py --inventory`; rechazar archivo alterado, no ejecutar nada sin hash. | CI Windows o Linux |
| E2 CPA | `run.py --only cpa`: YAML v8 en perfil nuevo, loopback y PID en Job, catálogo con ID conocido, stream que llega al fixture. | Runner Windows / VM |
| E3 OmniRoute | `prepare.py` consulta SQLite `:memory:` con Node privado; `--only omni` espera `/healthz` con texto `ok`, sin Electron ni segundo CPA. | Runner Windows / VM |
| E4 OpenCode | `--only oc`: `serve`, puerto de propiedad verificada, Basic auth y `GET /global/health`. | Runner Windows / VM |
| E5 Integración | `run.py`: login y CSRF de OmniRoute, nodo compatible `/v1`, probar conexión, importar catálogo, clave cliente de alcance limitado, turnos desde sesión OpenCode. | VM aislada; CI con control real de salida |
| E6 Catálogo | Releer modelo `labcpa/lab-reasoner` desde gateway y desde el proveedor del plugin en `/provider`; sin lista estática inyectada en OpenCode. Se declaran los tiers **del fixture** vía override upstream porque el listado genérico de CPA no los transporta. | Igual que E5 |
| E7 Esfuerzo | Dos sesiones/turnos con `variant=low/high`. Exigir respuesta del modelo, coincidencia `reasoning_effort` en receptor y modelo exacto; los turnos auxiliares del agente se distinguen en la lista de esfuerzos recibidos. | Igual que E5 |
| E8 Regresión | `gateway-late`, `gateway-body`, `gateway-cancel`, `control-cola`; evidencia temporal, recepción y cancelación. | Igual que E5 |
| E9 Supervisión | Trabajador espera entrada antes del spawn; ante fallo de asignación al Job no se lanza el binario. `finish()` requiere árbol vacío, sin matar por nombre. Forzar cierre del **propietario** durante un stream en VM y verificar desaparición de PIDs/puertos propios aparte. No equivale a probar cierre cooperativo ni persistencia de producción. | Job normal en runner; cierre forzado y logout en VM |
| E10 UI offline | Sonda HTML, MIME y assets JS/CSS del servidor, sin seguir redirects, con red externa desconectada. Verificación de navegación, chunks tardíos y panel Network en vista real por separado. | VM sin red externa; runner online no certifica offline |
| E11 UI A/B | [Protocolo y comandos](UI_COMPARISON.md); A = host WebView2 sin IPC, B = Desktop oficial; autenticación, sesiones, permisos, PTY, duplicados y recursos. | Windows 11 con escritorio interactivo |
| E12 Recursos | `metrics.jsonl`: roles y PIDs del Job, tiempo CPU acumulado, creación, private bytes y working set. `measure-ui.ps1` para árboles externos. Trabajadores auxiliares Python cuentan como sobrecarga del ensayo. | Smoke en runner; cifras comparables en equipo/VM dedicado |

## Ejecución exacta de 1B

1. Obtener una **VM Windows 11 x64 desechable**, usuario estándar nuevo sin perfiles anteriores, con CPython 3.12+ x64 y Python Launcher (`py -3.12` en los ejemplos). Python es herramienta de ensayo, no dependencia final. Copiar el repositorio sin `.lab`, en ruta corta, p. ej. `C:\lab\stack`. Permitir rutas largas o usar un directorio bastante corto para npm; prever espacio para dependencias opcionales y staging. No usar el equipo Linux como upstream, proxy ni almacén de datos.
2. **Preparación con red**, desde la raíz en PowerShell. Descargas desde upstream, locks e integridad verificados. Nada global, sin postinstall ni `npm rebuild`:

   ```powershell
   py -3.12 -B experiments/phase1/prepare.py --inventory
   py -3.12 -B experiments/phase1/prepare.py
   ```

   No continuar si cualquiera termina con código distinto de cero. `runtime/prepared` bajo `.lab` se crea solo tras cargar el addon con el Node privado y consultar SQLite. En caso de intento parcial, conservar el diagnóstico y repetir con otra copia del repo o snapshot limpio; el script rehúsa sobreescribir un runtime parcial. Para E11 preparar ambos comparadores **ahora** según su protocolo. Tomar snapshot de la VM preparada.
3. **Desconectar la NIC externa de la VM** desde el hipervisor; solo loopback debe quedar disponible. El flag `--isolated-offline` registra la precondición del operador, no cambia el firewall. Los componentes upstream pueden intentar conexiones de fondo; un HTML recibido online no prueba disponibilidad offline. Ejecutar de forma independiente:

   ```powershell
   py -3.12 -B experiments/phase1/test_fixture.py
   py -3.12 -B experiments/phase1/run.py --isolated-offline --only cpa
   py -3.12 -B experiments/phase1/run.py --isolated-offline --only omni
   py -3.12 -B experiments/phase1/run.py --isolated-offline --only oc
   py -3.12 -B experiments/phase1/run.py --isolated-offline
   ```

   Cada `run` crea `.lab/run-<id>` y sus propios puertos, contraseñas y carpetas. Leer `results.json` (estado por caso, HTTP/código permitido o línea de fallo) y `metrics.jsonl`. Si un prerrequisito falla y un caso no aparece, marcarlo **no ejecutado**. El modo completo termina con error si falla una sonda independiente. stdout/stderr de hijos no se guardan: diagnosticar fallos por etapa en el perfil sintético. No transferir `auth.json`, YAML, cookies, bases ni homes. No publicar releases ni artefactos de CI con perfiles.
4. Para E9 forzado: repetir el modo completo con `--interactive`, anotar PIDs y puertos propios, enviar un turno con escenario `cancel`, detener **solo el PID Python del lanzador** desde el Administrador de tareas. Contrastar en esa VM los handles/PID+creación y listeners con la última muestra; si alguno propio permanece, E9 forzado falla. Repetir E11 con perfil nuevo y los mismos datos. Adjuntar versión de Windows, hardware, versión WebView2, hashes, resultados y limitaciones a un informe 1B posterior.

**GitHub Actions:** los runners hospedados Windows pueden automatizar adquisición/inventario, SQLite y sondas deterministas con datos sintéticos. Usan imágenes Windows Server y tienen red saliente: no contarlos como certificación Windows 11, prueba offline ni benchmark interactivo. Usar runner efímero propio con aislamiento de salida verificable para ensayos offline automatizados; calificar PTY, permisos, selector de servidor, UX, comparación de memoria y cierre de sesión en escritorio real. No se creó ni disparó un workflow.

**Bloqueos reales y decisiones pendientes:** no se dispuso de un Windows 11 interactivo para ejecutar PE, cargar la ABI, medir Jobs o abrir WebView2. Si falta cualquier dependencia al arrancar offline, si `/provider` no presenta los tiers exactos, si no se libera un stream o si Electron crea otro servidor, registrar la evidencia y mantener el caso fallido; no elevar permisos, cambiar de cuenta o relajar la prueba para aprobarlo. Falta decidir el shell, aprobar la combinación de versiones y acordar presupuestos medidos. **No avanzar a fase 2 sin revisión explícita.**
