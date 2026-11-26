# AI Stack Desktop · laboratorio de fase 1A

**Estado: fase 0 revisada; preparación de 1A.** El [informe 1A](docs/PHASE1A_REPORT.md) contiene evidencias y comandos para la futura ejecución en Windows. El objetivo final es instalar un solo EXE por usuario y trabajar desde una ventana de Windows: conectar una cuenta autorizada, elegir un modelo y continuar una sesión de OpenCode. Todavía no hay aplicación de producción ni instalador.

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
