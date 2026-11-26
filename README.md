# AI Stack Desktop · prototipo Windows x64 `0.0.1-dev`

**[Descargar EXE y preparación W1 (ZIP de Actions)](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45895323904/artifacts/15153592383)** · [Guía de ejecución](desktop/PROBAR.md). EXE y backend probados en Windows Server; ventana pendiente de prueba interactiva en Windows 11.

**1B-CI aprobada para W1 corregido; 1B-Desktop pendiente.** [Prototipo y evidencia 2A](docs/PHASE2A_REPORT.md) · [Probar el EXE en Windows 11](desktop/PROBAR.md) · [Resultados de integración W1](docs/PHASE1B_CI_REPORT.md). El prototipo conecta una ventana Tauri a un supervisor nativo y usa únicamente el proveedor de demo. No es un instalador de producción.

**Recomendación:** reutilizar el servidor y el espacio de trabajo web de OpenCode, el gateway y el panel de OmniRoute, y los flujos de conexión de CLIProxyAPI. Un supervisor nativo sería el único propietario de sus procesos. La elección de Tauri frente al escritorio Electron oficial queda condicionada a una comparación en Windows con las mismas sesiones y herramientas. No hay una estimación de RAM de Windows validada.

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
