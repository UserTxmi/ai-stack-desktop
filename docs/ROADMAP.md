# Hoja de ruta con criterios de paso

**Orden aprobado:** preparar en Linux, medir en Windows aislado y decidir con la evidencia. Los scripts son instrumentos de ensayo, no la experiencia del usuario final.

| Fase | Trabajo acotado tras autorización | Entrega y criterio para avanzar |
|---|---|---|
| 0 · revisada | Doce documentos y arquitectura revisados por el responsable de producto. | [D0–D6 con condiciones](DECISIONS.md). |
| 1A · aprobada | Inventario fijado, fixture SSE, scripts de adquisición, sondas del plugin, variantes, timeouts y Jobs, instrumentación y comparadores. | [Informe reproducible](PHASE1A_REPORT.md). |
| 1B-CI · aprobada | W1 corregido, sin cambios de versiones. | [Evidencia antes y después](PHASE1B_CI_REPORT.md#revisión-de-bloqueos). Conservar pruebas de retirada. |
| 1B-Desktop · pendiente | Windows 11 x64 desechable, comparación UI/PTY, permisos, cierre forzado/logout y rendimiento prolongado. | Adjuntar resultados y decisión D1. |
| 2A · aprobada para alcance experimental | EXE de desarrollo, supervisor Rust y frontend TypeScript; proveedor demo sin cuentas. | [Prueba básica del responsable en Windows 11 e informe separado de CI](PHASE2A_REPORT.md). No cierra 1B-Desktop. |
| 2B · autorizada | ZIP x64 `0.0.2-dev`, carpeta portátil elegida al extraer, preparación, diagnóstico y recuperación desde Tauri. | [Informe 2B](PHASE2B_REPORT.md): EXE preparable sin terminal, regresiones W1 sobre ese mismo runtime y [prueba manual Windows 11](../desktop/PROBAR.md). |
| 2 · núcleo Windows | Una ventana propia, ciclo de vida por usuario, carpetas con ACL, diagnóstico mínimo, recuperación y bandeja; instalar carga útil privada de ensayo. | T1/T2/T6, incluida salida tras crash y coexistencia con procesos ajenos. No habilitar OAuth antes de pasar aislamiento. |
| 3 · proveedores y modelos | Aprovisionar gateway con IDs gestionados; autenticación autorizada, rotación/revocación y catálogo transaccional por conexión. | T3/T4 de extremo a extremo y pruebas limitadas consentidas por proveedor; publicar matriz de capacidades, esfuerzo y límites. |
| 4 · workspace y recursos | Integrar UI OpenCode, sesiones, eventos, proyectos y extensiones confiables. Perfiles WebView y barreras de seguridad. | T5, T6 y T7 prolongada con informe reproducible y presupuestos de recursos acordados; ningún recorte funcional usado como optimización. |
| 5 · distribución candidata | Build Windows reproducible, dependencias cerradas, firma y SBOM, EXE NSIS, migración, actualización y desinstalación. | T1, T8, T9 en máquinas limpias; simulacro de rollback con datos y restauración tras corte. Avisos offline, revisión de términos y plan de soporte. |
| 6 · piloto y mantenimiento | Piloto voluntario x64, diagnósticos revisables, canal de prueba antes de estable. Actualizar una versión upstream por experimento y promover solo conjuntos calificados. | Regresiones de contrato y de rendimiento pasan antes de promoción. Ampliar a ARM64 u otras builds solo con artefactos, ABI, equipo y resultados equivalentes. |

### Registro de riesgos abiertos

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
