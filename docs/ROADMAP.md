# Hoja de ruta con criterios de paso

**Orden aprobado:** preparar en Linux, medir en Windows aislado y decidir con la evidencia. Los scripts son instrumentos de ensayo, no la experiencia del usuario final.

| Fase | Trabajo acotado tras autorización | Entrega y criterio para avanzar |
|---|---|---|
| 0 · revisada | Doce documentos y arquitectura revisados por el responsable de producto. | [D0–D6 con condiciones](DECISIONS.md). |
| 1A · aprobada | Inventario fijado, fixture SSE, scripts de adquisición, sondas del plugin, variantes, timeouts y Jobs, instrumentación y comparadores. | [Informe reproducible](PHASE1A_REPORT.md). |
| 1B-CI · aprobada | W1 corregido, sin cambios de versiones. | [Evidencia antes y después](PHASE1B_CI_REPORT.md#revisión-de-bloqueos). Conservar pruebas de retirada. |
| 1B-Desktop · pendiente | Windows 11 x64 desechable, comparación UI/PTY, permisos, cierre forzado/logout y rendimiento prolongado. | Adjuntar resultados y decisión D1. |
| 2A · aprobada para alcance experimental | EXE de desarrollo, supervisor Rust y frontend TypeScript; proveedor demo sin cuentas. | [Prueba básica del responsable en Windows 11 e informe separado de CI](PHASE2A_REPORT.md). No cierra 1B-Desktop. |
| 2B · prueba interactiva básica aprobada | ZIP x64 `0.0.2-dev`, preparación y recuperación desde Tauri. | [Informe 2B](PHASE2B_REPORT.md): CI en Windows Server y confirmación básica por el usuario en Windows 11. |
| 2C · cerrada | OpenCode oficial local, proyectos y sesiones. `0.0.4-dev` queda como recuperación. | [Aceptación interactiva: preparación, proyecto, modelo, low y high](PHASE2C_REPORT.md). |
| G1 · PoC sintética verificada en CI | Gentle Shell/Pi → OmniRoute → CPA. Dos rutas, delegación real, abort, reapertura, memoria y Jobs. | [Resultados y comparación de interfaces](GENTLE_POC.md): ambas emitieron un turno al receptor en Windows Server x64. |
| G2 · GPUI verificada en CI | Supervisor Rust y preparación automática del candidato 0.0.5-dev. | [Evidencia G2](GENTLE_G2.md): coexistencia, deny, allow y cierre con hijo activo aprobados. Pendiente empaquetar sin repetir G1/W1 y validar el prototipo manualmente en Windows 11. |
| 2 · núcleo Windows | Una ventana propia, ciclo de vida por usuario, carpetas con ACL, diagnóstico mínimo, recuperación y bandeja; instalar carga útil privada de ensayo. | T1/T2/T6, incluida salida tras crash y coexistencia con procesos ajenos. No habilitar OAuth antes de pasar aislamiento. |
| 3 · proveedores y modelos | Aprovisionar gateway con IDs gestionados; autenticación autorizada, rotación/revocación y catálogo transaccional por conexión. | T3/T4 de extremo a extremo y pruebas limitadas consentidas por proveedor; publicar matriz de capacidades, esfuerzo y límites. |
| 4 · workspace y recursos | Consolidar la UI oficial Gentle Shell seleccionada, sesiones Pi, Engram, proyectos y extensiones confiables. | T5, T6 y T7 prolongada con informe reproducible y presupuestos de recursos acordados; ningún recorte funcional usado como optimización. |
| 5 · distribución candidata | Build Windows reproducible, dependencias cerradas, firma y SBOM, EXE NSIS, migración, actualización y desinstalación. | T1, T8, T9 en máquinas limpias; simulacro de rollback con datos y restauración tras corte. Avisos offline, revisión de términos y plan de soporte. |
| 6 · piloto y mantenimiento | Piloto voluntario x64, diagnósticos revisables, canal de prueba antes de estable. Actualizar una versión upstream por experimento y promover solo conjuntos calificados. | Regresiones de contrato y de rendimiento pasan antes de promoción. Ampliar a ARM64 u otras builds solo con artefactos, ABI, equipo y resultados equivalentes. |

**Criterio obligatorio de fase 5:** entregar dos EXE diferenciados: instalador clásico (tipo Inno Setup o NSIS) y aplicación principal. El asistente permite seleccionar ubicación, prepara dependencias automáticamente con barra de progreso, crea accesos al escritorio y al menú Inicio, ofrece desinstalador y abre la aplicación instalada. No se implementa en 2C.

### Registro de riesgos abiertos

Prueba de aceptación del instalador clásico: partir de una máquina sin Visual C++ Redistributable x64, instalarlo automáticamente y comprobar la primera apertura sin error `VCRUNTIME140.dll`.

| Riesgo / impacto | Responsable y decisión pendiente |
|---|---|
| Crítico: UI web hace fallback a internet; panel CPA se descarga aparte. Una página no fijada obtendría autoridad de su origen local. | Seguridad/desktop: exigir recursos locales verificados y ausencia de fallback en T1/T2; reevaluar Desktop oficial si no se puede garantizar. |
| Crítico: plugin ofrece niveles inferidos o transforma un ID de combo, y el proveedor acepta la petición ignorando esfuerzo. | Integración: T4 con receptor de ensayo y contraste de capacidades por conexión. Usar solo rutas verificadas; si el plugin falla, separar fallo de upstream de política de producto. |
| Alto: dos supervisores, exposición de controles de servicio de OmniRoute o reinicio tras logout. | Windows: T2/T6; escoger una autoridad y restringir operaciones alcanzables, no solo ocultar botones. |
| Alto: ABI de `better-sqlite3`, selección de binario CPU, rutas Unicode/PTY y herramientas habitualmente usadas desde WSL. | Build/desktop: T1/T5 con perfil estándar; inventariar herramientas realmente necesarias y sus licencias. No prometer paridad ARM64. |
| Alto: capas de configuración de OpenCode mezclan credenciales/permisos con otra instalación; plugins/MCP heredan secretos. | Seguridad: T2/T5/T8 con perfil ajeno y marcadores sintéticos; delimitar la confianza en código local y la propiedad de los datos. |
| Alto: migración irreversible o plugin/activo se actualiza por fuera del conjunto. | Release: T8/T9; bloquear actualización si no existe snapshot consistente y ruta probada de retorno. |
| Alto: autorización del proveedor o derechos de redistribuir activo/auxiliar desconocidos. | Producto/release: validar por proveedor, plan y artefacto; no anunciar ni incluir el elemento sin revisión. |
| Medio/alto: CPU en reposo o crecimiento de RAM desconocidos en Windows. | Rendimiento: T7 con sesiones largas, contabilidad compartida y control A/B; presupuesto después de medir. |

**? · Cierre de cada fase:** responsable adjunta evidencia filtrada, limitaciones y decisión; un fallo bloqueante genera investigación o cambio acotado, no un fork por defecto. Proponer un fork solo con reproducción, intento upstream, coste de mantenimiento y condición de retirada.
