# Registro de decisiones

**26-11-2026 · Revisión del responsable del producto:** aceptados los doce entregables de fase 0 y las siguientes condiciones. Se autoriza la **preparación experimental 1A** en este repositorio. La aceptación de la arquitectura no certifica ninguna combinación Windows ni autoriza producción. La fuente de verdad es `docs/`, incluso si un resumen de Engram anterior aún dice «pendiente».

| ID | Resolución | Condición que se mantiene |
|---|---|---|
| D0 · ratificada | Protección absoluta de la infraestructura Linux operativa. | Ni pruebas ni lectura de estado en vivo, procesos, credenciales, servicios o bases. El laboratorio solo usa perfiles nuevos, proveedores sintéticos y cachés del repositorio. |
| D1 · aceptada parcialmente | Reutilizar el workspace oficial, proyectos, permisos, eventos, herramientas y terminal interna de OpenCode. | Elección del shell **abierta** entre Tauri/WebView2 y Desktop Electron/Solid. Decidir por paridad y mediciones con el [protocolo A/B](UI_COMPARISON.md); ningún chat propio o fork como atajo. |
| D2 · aceptada | Un supervisor es propietario de los procesos gestionados. | OmniRoute no debe arrancar otro CPA ni el shell un segundo agente. Jobs son mecanismo candidato; cierre normal y pérdida del propietario se validan por separado. |
| D3 · aceptada como candidata | Probar `@omniroute/opencode-plugin@0.2.2` publicado e íntegro. | Un mapeo correcto en memoria no garantiza una selección funcional. Exigir catálogo efectivo y confirmación de `low` y `high` en la petición recibida al final de la cadena. |
| D4 · aceptada | Separar perfiles, credenciales administrativas y de inferencia, orígenes y permisos conforme a [Seguridad](SECURITY.md). | Paneles originales disponibles cuando se puedan aislar; no prestar IPC privilegiado, importar credenciales ni dar por seguro un plugin con permisos del usuario. |
| D5 · aceptada como objetivo final | Windows 11 x64, instalación por usuario con **un EXE**, runtimes privados y aprovisionamiento automático. | Un prototipo con scripts es suficiente en 1A/1B. NSIS, firma de release, migración y rollback pertenecen a distribución. El operador de laboratorio usa comandos; el usuario final no necesita CMD, PowerShell, WSL, Docker ni editar archivos. |
| D6 · aceptada | Optimizar con mediciones reales de proceso/árbol y sesiones comparables. | Sin presupuesto Windows sin medir; no reducir contexto, permisos ni capacidades del agente para mejorar la cifra. |

**Candidato experimental W1 (no baseline de producción):** CPA 8.1.7, OmniRoute 3.8.54, Node 24.21.0, OpenCode 1.19.6 y plugin 0.2.2. Motivo, comparación y límites de evidencia en el [informe 1A](PHASE1A_REPORT.md). Ninguna modificación de las versiones Linux. **Fase 2 bloqueada hasta revisión explícita de 1B.**

**Revisión posterior · 1A aprobada:** se autoriza ejecutar W1 en GitHub Actions, con CachyOS como entorno de desarrollo. Por instrucción corregida del responsable, el repositorio `UserTxmi/ai-stack-desktop` es **público**. El runner estándar `windows-2025` permite calificar rutas headless, no WebView2 offline, escritorio ni soporte final de Windows 11. Las evidencias públicas se limitan a [CI](CI.md); no hay distribución de binarios.
