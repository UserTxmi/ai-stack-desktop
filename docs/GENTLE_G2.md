# G2 · candidato GPUI 0.0.5-dev · entrega bloqueada

**Decisión:** GPUI sigue siendo la interfaz principal elegida. El ejecutable de desarrollo existe, pero **no hay ZIP aprobado de 0.0.5-dev**. La condición del usuario es probar permisos del hijo y cierre mientras delega en la interfaz que recibirá; una conversación de texto o una prueba RPC fuera de GPUI no satisfacen esa condición. La descarga funcional sigue siendo [0.0.4-dev](PHASE2C_REPORT.md).

## Lo que se ejecutó

| Prueba | Resultado y alcance |
|---|---|
| Referencia OpenCode, preparación y W1 completa | [Aprobada en Windows Server x64](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45928048829), también después del cambio del supervisor compartido. La copia de recuperación original y sus hashes siguen en el [informe G1](GENTLE_POC.md#recuperación). |
| EXE del candidato | [Run del candidato](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45928048782): compilación MSVC, preparación desde raíz vacía, GPUI con instancia propia, respuesta `LABLABLAB` en el historial Pi y cierre normal. La segunda apertura llegó al plazo de **delegación activa en receptor**. Los pasos de empaquetado fueron omitidos por fallo. |
| Propiedad y coexistencia | Dos procesos GPUI oficiales con registros y listeners distintos, PID verificado en cada Job. Se comprobó que la ventana externa siguió viva y que los bytes de tres testigos externos (configuración, sesión, credencial sintética) no cambiaron al finalizar cada árbol gestionado. No es una auditoría de todos los archivos de un usuario Windows. |
| Motor común, fuera de la ventana | Catálogo, low/high atribuidos, sesión reabierta, memoria tras reinicio, delegación `main/low → child/high` y abort con desconexión del receptor: aprobados. |
| Permisos | En diagnóstico local sin HOME ni red del host, la distribución npm 5.0.0 permitió la lectura del testigo fuera del proyecto sin pregunta. Con el [cambio oficial de Gentle Shell](https://github.com/Gentleman-Programming/gentle-shell/commit/ad183931612106fe1cee05d53979e1caef725cf5) el mismo hijo produjo `confirm`: denegar impidió la lectura; aceptar devolvió el contenido. Se fijaron el diff y cinco postimágenes, sin modificar GPUI. Retirar el backport al calificar una versión publicada que supere la misma sonda. No se observó ese diálogo en la ventana Windows. |
| Cierre con un hijo activo | No verificado en GPUI. En los tres escenarios el Job vació sus procesos y la ventana externa sobrevivió, pero el receptor del hijo **no recibió ninguna petición**. No equivale a matar un subagente ocupado ni a probar la pérdida de su supervisor. |

## Diagnóstico reproducible

El [informe del run cancelado 45930024545](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45930024545) localizó el fallo común. Antes de OmniRoute, Pi construyó cada turno con **64 herramientas**, `subagent_run` incluido, y 148.740–149.274 bytes. El proveedor respondió HTTP 400 dos veces; el error clasificado por Pi fue entrada/tokens por encima del contexto del modelo. El gateway no quitó herramientas: el receptor vio cero porque la petición válida nunca llegó a él. El fixture G1 declaraba 32.768 tokens, suficiente para el chat anterior pero insuficiente para el payload oficial completo de GPUI. `deny`, `allow` y `active-close` fallaron antes de crear un hijo. Es un error de diseño del laboratorio, no evidencia de fallo en permisos, diálogos o cierre del producto.

El workflow completo queda sólo bajo `workflow_dispatch`; no volverá a ejecutarse por `push`. La corrección focal cambia únicamente el contexto anunciado por el fixture a 262.144 y prueba coexistencia, `deny`, `allow` y `active-close`. No aumenta plazos ni repite G1/W1. [Workflow focal](../.github/workflows/windows-g2-focal.yml).

La [ejecución focal 1/2](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45931868127) falló antes de abrir la ventana propia: el camino corto llamó G2 antes de definir `GENTLE_SHELL_BIN` (`KeyError`, G2 vacío). Es un error de orden del laboratorio, no un resultado de producto. La segunda ejecución será la última autorizada en esta sesión.

La prueba focal exige en la ventana denegación sin lectura, aceptación con lectura y pérdida del propietario con un hijo que ya haya emitido su primer fragmento. No acepta una llamada sintética a una herramienta que el modelo no recibió. El informe no exporta sesiones, cabeceras, cuerpos ni perfiles.

## Integración preparada

`desktop/supervisor/src/bin/ai-stack-gentle.rs` prepara automáticamente y presenta progreso por consola; **GPUI es la única interfaz de escritorio**. El supervisor Rust reutiliza adquisición verificada, staging, CLIProxyAPI, OmniRoute y Jobs Win32; la rama Gentle termina de provisionar el gateway antes de la rama OpenCode y no prepara ni ejecuta OpenCode. La única conexión gestionada apunta al receptor sintético. Los dos IDs del demo comparten ese receptor. Pi consume los modelos y esfuerzos anunciados por el gateway; GPUI conserva su editor oficial de perfiles. El estado de GPUI (`GENTLE_SHELL_GPUI_DATA_DIR`), el launcher, el home Pi, las rutas Gentle y la memoria Engram se seleccionan por separado. Los tokens de servicios se generan en ejecución. El instalador clásico sigue siendo un requisito de distribución posterior.
