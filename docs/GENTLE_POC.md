# Gentle Shell como motor · prueba de concepto Windows x64

**Decisión de producto:** usar Gentle Shell sobre Pi. **Recomendación de interfaz:** calificar primero **B, el escritorio GPUI oficial**, por su editor de perfiles por agente, gestión de sesiones y controles de subagentes ya implementados. Mantener **A, la TUI sobre ConPTY**, como alternativa si el host RPC pierde decisiones de permisos o GPUI no puede funcionar dentro de la raíz privada. No elegir por tamaño de descarga o por RAM de un proceso suelto.

| Aspecto | A · TUI oficial + ConPTY | B · GPUI oficial + RPC |
|---|---|---|
| Qué aporta upstream | Pantallas oficiales `/gentle:profiles`, herramientas y diálogos del terminal; el motor ya renderiza y procesa las teclas. | Ventana, pestañas, panel de agentes, modelo/esfuerzo por rol, editor, terminales y sesiones. Cada pestaña inicia `gentle-shell --mode rpc`. |
| Trabajo propio | Alojar una pseudoconsola, renderizar VT con un terminal existente, enrutar entrada, clipboard, IME, resize/DPI, foco y cierre. Encaja en la ventana de servicios actual. | Lanzar el ZIP oficial con backend privado y conectar el ciclo de vida de la ventana al supervisor. No incrustar una ventana nativa a la fuerza ni crear otro frontend RPC. La PoC utiliza ventanas separadas. |
| Diferencias funcionales | Mayor fidelidad a comandos interactivos; ergonomía de terminal y accesibilidad dependen del host. | El canal Desktop permite editar perfiles y cancelar helpers sin emular teclas. Algunos comandos sólo TUI están ocultos; el modo Auto puede rechazar una decisión RPC. |
| Propiedad y rutas | Un Job por árbol, incluyendo auxiliares de ConPTY; evitar que el emulador se quede con pipes al cerrar. | Un árbol puede contener varios backends, terminales, procesos de Git o vistas web. Evitar que el Setup integrado instale otro runtime, active `/login` o adopte un proceso global. |
| Prueba decisiva | Teclado español/IME, texto Unicode, resize, permisos y cancelación con foco real. | GPU/driver y renderizado, permiso solicitado por un hijo, cierre de múltiples pestañas y ausencia de escrituras/actualizaciones externas. |

## Contratos utilizados

- [Gentle Shell 5.0.0, launcher y perfiles](https://github.com/Gentleman-Programming/gentle-shell/blob/v5.0.0/docs/readme-reference.md): `--isolated`, Pi adyacente al paquete, `GENTLE_SHELL_HOME`, `GENTLE_PI_CONFIG_HOME`; `orchestrator` es una clave reservada distinta de los roles. El helper se ejecuta en [otro Pi RPC](https://github.com/Gentleman-Programming/gentle-shell/blob/v5.0.0/lib/agents-runner.ts). No usar `--link` ni importar credenciales.
- [Pi 1.6.1, modelos](https://github.com/earendil-works/pi/blob/v1.6.1/packages/coding-agent/docs/models.md) y [RPC](https://github.com/earendil-works/pi/blob/v1.6.1/packages/coding-agent/docs/rpc.md): proyectar el catálogo efectivo `/v1/models` del gateway a `agent/models.json`, `openai-completions`, `thinkingFormat: openai`, `supportsReasoningEffort`, `thinkingLevelMap` con `null` para niveles no anunciados. El registro del proveedor Pi no es el archivo `config/models.json` de asignaciones Gentle. Pi puede ajustar un esfuerzo desconocido al soportado más cercano; no aceptar un ajuste silencioso al materializar perfiles gestionados.
- [GPUI 0.1.28](https://github.com/Gentleman-Programming/gentle-shell-gpui/tree/v0.1.28), [resolución del backend](https://github.com/Gentleman-Programming/gentle-shell-gpui/blob/v0.1.28/crates/pi-rpc/src/launch.rs): `GENTLE_SHELL_BIN` selecciona el launcher privado; la UI usa el canal Desktop del Shell. El [ZIP y el setup x86_64](https://github.com/Gentleman-Programming/gentle-shell-gpui/releases/tag/v0.1.28) son alternativas oficiales de distribución de GPUI, no el instalador completo del producto.
- [Gentle AI 5.0.0, instalación Windows](https://github.com/Gentleman-Programming/gentle-ai/tree/v5.0.0/internal/shellinstaller) proporciona aprovisionamiento, no otro motor. El [instalador package-local de Shell](https://github.com/Gentleman-Programming/gentle-shell/blob/v5.0.0/scripts/gentle-ai-installer.mjs) compila el módulo fijado con Go y verifica su procedencia en Windows. Usar Go en build, no exigirlo al usuario final. Auditar binarios, licencias y marcas antes de redistribuirlos.
- [Engram 3.7.2](https://github.com/Gentleman-Programming/engram/blob/v3.7.2/DOCS.md) con `gentle-engram` 0.8.0, `ENGRAM_BIN`, `ENGRAM_URL` y `ENGRAM_DATA_DIR` explícitos. El supervisor debe iniciar un servidor propio en loopback; el puerto 7437 del usuario no es una dependencia.

## Ensayo y reproducción

[Workflow](../.github/workflows/windows-gentle.yml) separado del build de recuperación. Ejecutar **Windows x64 · PoC Gentle Shell** sobre un runner estándar, sin inputs secretos. El laboratorio usa W1 fijado y dos instancias aisladas del mismo receptor sintético, una por ruta de CPA, para distinguir `main/lab-reasoner` de `child/lab-reasoner` incluso después de que CPA quite el prefijo. El fixture ordena `subagent_run` sin pasarle modelo ni esfuerzo: el hijo debe usar su asignación de perfil. La aceptación exige observar `high` en el receptor hijo mientras el padre usa `low`.

`low` y `high` se prueban en procesos nuevos mediante RPC. `cancel` exige delta antes de `abort`, final `aborted` y desconexión en el receptor. `resume` abre el JSONL previo y verifica identidad e historial; Engram se reinicia sobre el mismo almacén y se busca una observación conocida. El host de CI usa [Jobs de laboratorio W1](../experiments/phase1/winjob.py); aún no es integración del motor en el supervisor Rust. Las cifras PSAPI incluyen los workers de ensayo; bytes privados no equivalen a RAM física única. Las mediciones de arranque de interfaces no son una comparativa de carga interactiva sostenida.

La adquisición verifica el lock npm y las huellas de los ZIP; `npm ci --ignore-scripts`, sin instalaciones globales. Se crean HOME, AppData, temporales, almacenes, sesiones y runtimes de prueba separados. Sólo se pasa al agente la clave de inferencia temporal; las claves de los receptores y la administración quedan fuera de su entorno. El artefacto público incluye únicamente resultados reducidos. Mantener estas barreras en el EXE y validar las rutas efectivamente abiertas por cada UI (una variable de entorno no prueba aislamiento completo).

## Recuperación

La referencia funcional sigue en el [informe 2C](PHASE2C_REPORT.md): ZIP, hashes, caducidad y commit del EXE. Tras la caducidad, reconstruir la referencia desde ese commit con su workflow W1 y verificar el conjunto antes de distribuir el nuevo artefacto; no asumir que una recompilación tendrá la misma huella. No abrir la base de OpenCode con Pi ni transformar sesiones en sitio. El requisito final sigue siendo instalador EXE por usuario separado de la aplicación y preparación automática.

Se volvió a descargar el ZIP exacto a `experiments/phase1/.lab/downloads/recovery-0.0.4-dev.zip` (copia local ignorada por Git): SHA-256 `90cc1b35d4178829d7e281fc79d87508f1eb88f28c576ab31b9d6de1c1b912bf`. La caducidad del artefacto de Actions no elimina esta copia.

## Evidencia Windows x64

**Resultado final: [run 45921904464 aprobado](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45921904464)** en `windows-2025` x64, job de 6 min 49 s, [commit probado](https://github.com/UserTxmi/ai-stack-desktop/commit/14f62e816d1bcd657577d700e95cc12db1e89460). Se descargó y leyó el [artefacto JSON](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45921904464/artifacts/15165665893), digest del ZIP `4f5b036e56e3ee04594c630c298fa8e4345175f59fd397eaa4eefe3e51cff4e0`. Pasaron descubrimiento, low, high, reapertura, delegación, cancelación y memoria tras reinicio. Además, **ambas interfaces enviaron su propio turno identificado hasta el receptor**. Los seis Jobs de interfaces y servicios quedaron vacíos; los cinco Jobs RPC cerraron normalmente por EOF sin descendientes antes del cierre del Job.

| Árbol (medición tras recibir el turno de esa interfaz) | Procesos | Bytes privados | CPU acumulada de los procesos presentes |
|---|---:|---:|---:|
| TUI oficial en ConPTY, con salida VT y resize | 8 | 523,5 MiB | 8,67 s |
| Ventana oficial GPUI con `--prompt` | 10 | 553,6 MiB | 37,12 s |

Los picos del **árbol RPC** en esa misma ejecución fueron 780,5 MiB (low), 620,1 (high), 523,2 (reabrir), **1083,3 (delegar)** y 577,9 (cancelar). Las dos interfaces están suficientemente probadas para el siguiente ensayo de escritorio; sólo se ha medido una muestra por interfaz, con auxiliares de ensayo distintos y sin medir interacción humana, consumo gráfico ni sesiones prolongadas. La diferencia de 30,1 MiB no justifica reimplementar la experiencia de GPUI. Antes de adoptarla: verificar en Windows 11 la decisión de permisos, el aislamiento frente a otra instalación GPUI ya abierta y el cierre durante una delegación; comparar con ConPTY bajo esa misma carga.

### Historial de diagnóstico

El [primer ensayo](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45920881692) verificó la preparación y detectó que el ZIP MinGit fijado incluye `usr/bin/sh.exe` y no `bash.exe`. El laboratorio selecciona ese shell POSIX explícitamente; esto no acredita compatibilidad con scripts específicos de Bash.

El [segundo ensayo, commit `3ba5c30`](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45921263595) **pasó todas las sondas del motor**, pero el job falló al añadir el cierre de ConPTY a un array de informe que todavía no existía. El [JSON público de esa ejecución](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45921263595/artifacts/15164681561) diferencia los pasos efectivamente medidos:

| Sonda | Resultado comprobado | Pico de bytes privados del árbol del agente | CPU acumulada muestreada / tiempo de ensayo |
|---|---|---:|---:|
| Orquestador `low` | Dos modelos en Pi, sólo dos esfuerzos; `LABLABLAB` y exactamente una petición `low` en main | 766,0 MiB | 34,38 / 40,95 s |
| Orquestador `high` | `LABLABLAB` y exactamente una petición `high` en main | 626,1 MiB | 21,95 / 20,88 s |
| Reabrir | Mismo ID, prompt y respuesta en otro proceso Pi | 511,3 MiB | 20,72 / 20,45 s |
| Delegar | `gentle-ai-explore`: una petición marcada en la ruta child con `high`; padre en main con `low`, `subagent_run` completado | 1017,0 MiB | 37,33 / 22,47 s |
| Cancelar | Delta visible en RPC antes del abort; mensaje abortado y socket del receptor desconectado | 574,4 MiB | 21,59 / 20,67 s |

En los cinco casos el Job no tenía descendientes tras EOF. Engram conservó la observación al matar y relanzar su servidor privado; la prueba es de persistencia del core, no de calidad del recuerdo generado por un modelo. El ensayo comprobó también el vaciado de ambos CPA, gateway y Engram. La primera muestra ConPTY a los 18 s tuvo salida VT, resize, 8 procesos y 342,8 MiB privados. Cada fase inicia el runtime de nuevo: no interpretar la diferencia entre `low` y `high` como coste del esfuerzo del modelo ni usar el pico del subagente como cifra de reposo.
