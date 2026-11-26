# Referencia Linux: qué se puede trasladar

**V · Método:** inspección estática de los ocho archivos de la lista permitida bajo `/work/infra/ai-stack`. No se ejecutaron unidades, comandos de diagnóstico ni peticiones al stack. Las versiones locales de CLIProxyAPI 8.1.6 (`0aaf6d1c`), OmniRoute 3.8.50 y OpenCode 1.18.31 son las **declaradas por el proyecto**; no se inspeccionaron instalaciones, wrappers, sesiones ni conexiones. `mise.toml` sí fija Node 24.21.0.

| Fuente local (ruta relativa) | V · Evidencia estática | Consecuencia de diseño |
|---|---|---|
| `scripts/ai-stack`, líneas 23–76, 127–149 | `systemctl --user`: CLIProxyAPI primero, espera `GET /v1/models` autenticado en 8317; luego OmniRoute y `/healthz` en 20128. Esperas con timeout. Parada inversa, consola con traps, journal y enlaces a `/management.html` y al dashboard. | Separar proceso vivo, listo para HTTP y cuenta utilizable. La consola no supervisa OpenCode. No reproducir mensajes fijos de disponibilidad como si fuesen descubrimiento. |
| `scripts/render-cliproxyapi-config`, 4–14 | Sustituye dos marcadores de plantilla y aplica modo 0600 al YAML generado. | Generar estructura v8 con serializador y ACL desde la creación; no portar sustitución textual ni copiar el fichero resultante. |
| `config/cliproxyapi/config.template.yaml` | Declara host `127.0.0.1`, puerto 8317, secciones de gestión, routing y observabilidad con sintaxis anterior. | La plantilla no es el esquema de distribución v8; contrastar con el ejemplo etiquetado y observar el listener real. |
| `config/omniroute.env` | Nombres de opciones para datos, host/puerto, exigencia de clave cliente, log, memoria e inspector. Se inventariaron los nombres, no los valores de autenticación. | Crear un entorno privado por hijo; no heredar ni importar el entorno de Linux. |
| `scripts/sync-opencode-combos`, 23–56 | Bloqueo `flock`, GET a `/v1/models` y `/api/v1/combos`, exige `.data` array. Cruza `owned_by == "combo"` e `id` con `name` persistido; genera solo `name`. Quita IDs del estado anterior y reemplaza el JSON por `mv`. | Evita algunos combos virtuales, pero no propaga esfuerzo ni capacidades; config y estado se escriben por separado. `jq` exige JSON válido aun con extensión `.jsonc`. No sobrescribir personalizaciones homónimas. |
| `scripts/ai-stack-memory.py` | Muestreo de `/proc` y cgroups, RSS, swap y memoria del sistema; JSONL rotativo (3 × 2 MiB). | Sustituir por contadores Windows y mantener acotados tanto capturas como buffers. |
| `docs/memory-diagnosis.md`, 3–20, 35–41 | Observaciones del 22-09-2026 y límites de su interpretación; ajustes pendientes de prueba prolongada. | Son historia, no benchmark actual; cifras y protocolo en [Rendimiento](PERFORMANCE.md). |
| `mise.toml` | Pin explícito de Node. | Distribución privada verificada, sin mise ni instalación global. |

**?** Los archivos permiten reconstruir el orden y la sincronización, no verificar el grafo de rutas activo, la titularidad de cuentas, el contenido de bases ni el estado en vivo. El camino OpenCode → OmniRoute → CLIProxyAPI del producto es una **propuesta** que se probará con perfiles nuevos.
