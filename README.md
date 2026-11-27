# AI Stack Desktop · prototipo Windows x64 `0.0.4-dev`

**[Descargar Windows x64 0.0.4-dev sin cuenta](https://nightly.link/UserTxmi/ai-stack-desktop/actions/runs/45916757368/AI-Stack-Desktop-0.0.4-dev-win-x64.zip)** · [Workspace 2C y resultados](docs/PHASE2C_REPORT.md) · [Guía para Windows](desktop/PROBAR.md). Extraiga el ZIP a una carpeta privada nueva, abra `bin\ai-stack-desktop.exe`, prepare e inicie el stack en **Servicios** y abra su carpeta desde **Proyectos**. La respuesta del agente es sintética.

**1B-CI aprobada para W1 corregido; 1B-Desktop pendiente.** [Preparación automática 2B](docs/PHASE2B_REPORT.md) · [Prueba interactiva básica 2A en Windows 11](docs/PHASE2A_REPORT.md#prueba-interactiva-básica-en-windows-11) · [Resultados de integración W1](docs/PHASE1B_CI_REPORT.md). El prototipo conecta una ventana Tauri a un supervisor nativo y usa únicamente el proveedor de demo. No es un instalador de producción.

**2C cerrada por validación interactiva en Windows.** `0.0.4-dev` con OpenCode es la referencia de recuperación. Para desarrollo nuevo, el motor elegido es **Gentle Shell sobre Pi**; OmniRoute seguirá siendo el único proveedor del agente. [PoC Windows x64, comparación ConPTY/GPUI y recomendación](docs/GENTLE_POC.md). La PoC no reemplaza el EXE que se descarga arriba.

| Para decidir… | Leer |
|---|---|
| Qué debe poder hacer el usuario | [Producto](docs/PRODUCT.md) |
| Qué funciona hoy y qué se midió | [Referencia Linux](docs/CURRENT_STACK.md) |
| Qué publican y permiten redistribuir los proyectos | [Investigación upstream](docs/UPSTREAM_RESEARCH.md) |
| Quién arranca, detiene, instala y actualiza | [Arquitectura](docs/ARCHITECTURE.md) |
| Cómo llegan una conexión, un modelo y su variante al agente | [Integraciones](docs/INTEGRATIONS.md) |
| Qué protege cada frontera | [Seguridad](docs/SECURITY.md) |
| Cómo distinguir carga normal de una regresión | [Rendimiento](docs/PERFORMANCE.md) |
| Cómo demostrarlo en Windows | [Pruebas](docs/TESTING.md) |
| Qué aprobar ahora y qué se desbloquea después | [Decisiones](docs/DECISIONS.md) · [Hoja de ruta](docs/ROADMAP.md) |

**Convención:** **V** = verificado en fuente identificada (no necesariamente ejecutado); **S** = supuesto de trabajo; **P** = propuesta; **?** = cuestión abierta. Las decisiones revisadas y los límites de autorización constan en [Decisiones](docs/DECISIONS.md).
