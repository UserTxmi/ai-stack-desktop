# Arquitectura propuesta para Windows

**P · Decisión a revisar:** construir un supervisor por usuario y componer servicios mediante sus interfaces oficiales. Reutilizar el espacio de trabajo de OpenCode, no el instalador de cada escritorio. Las pruebas [T1–T9](TESTING.md) deciden si esta composición llega a producción.

```text
Ventana propia (estado, conexiones, modelo, recursos, mantenimiento)
    │ comandos tipados y limitados
    ▼
Supervisor nativo ── almacén de secretos, manifiesto, ACL, Job Objects
    ├── CLIProxyAPI           ── OAuth/API autorizada ── proveedor
    ├── Node privado + OmniRoute ── inferencia ── CLIProxyAPI
    └── OpenCode serve ── clave de inferencia ── OmniRoute
             ▲
        UI oficial de trabajo (vista sin privilegios de escritorio)
Paneles upstream: vistas efímeras sin IPC nativo; OAuth: navegador del sistema.
```

### Elección de interfaz

| Alternativa | Qué se reutiliza | Coste y criterio |
|---|---|---|
| **A · Tauri 2/Rust + WebView2 + UI oficial** | Un único agente y su UI web. TypeScript para la pequeña consola propia; React es viable para esa consola, no se portaría Solid a React. | Preferida si la UI local funciona con auth, terminal, archivos, SSE y permisos en WebView2, con una sola instancia y aislamiento demostrable. Introduce integración Win32 y dependencia WebView2; no se le asigna ahorro de RAM a priori. |
| **B · Basarse en OpenCode Desktop Electron** | `packages/app`, selección de proyectos, shell, servidor y gestión de ventanas ya integrados; `src/main/server.ts` y el actualizador son referencias directas. | Comparador obligatorio si A pierde paridad. Integrar supervisión en **un solo** proceso principal y coordinar su actualizador. El binario intacto como escritorio separado facilita una prueba, pero añade otra experiencia; mantener un parche permanente exige demostrar que no hay punto de composición upstream. |
| **C · Workspace propio vía SDK** | API, herramientas y persistencia del servidor, pero habría que reproducir permisos, terminal, reconexión y evolución de UI. | Reservar el SDK para funciones de control necesarias. Crear un workspace completo solo ante bloqueo reproducible de A y B y con presupuesto explícito de paridad. |
| **D · OmniRoute Desktop como host** | Dashboard Electron y servicios integrados. | Buena referencia de empaquetado de Next/SQLite; no aporta el workspace OpenCode. Introduce otro propietario de CPA y actualizaciones no coordinadas. |

Fuentes de estas alternativas: [etiquetas, paquetes y artefactos](UPSTREAM_RESEARCH.md). Ni A ni B requiere que el usuario instale un segundo escritorio. **S:** el EXE CLI de OpenCode contiene la UI local adecuada; si usa el respaldo remoto, probar activos oficiales fijados y su licencia. No aceptar código remoto sobre un origen con acceso al agente.

### Propiedad y estados

**P:** el supervisor mantiene un mutex por usuario/perfil con ACL y una tabla de handles, hora de creación, ruta absoluta y generación por proceso. Una segunda apertura activa la ventana existente mediante IPC restringido al usuario. Un puerto ocupado no identifica al servicio: elegir otro y reconfigurar las URL dependientes o informar del conflicto; nunca adoptar ni terminar el proceso ajeno. Si un redirect OAuth exige puerto fijo, la conexión falla con diagnóstico específico.

| Transición | Operación y condición de salida |
|---|---|
| Inactivo → preparando | Verificar manifiesto, ejecutables, arquitectura, WebView2 y versión de datos. Generar entorno mínimo, carpetas y secretos por perfil. No crear servicios SCM ni requerir administrador. |
| Preparando → arrancando | Crear un Job por árbol supervisado con `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; crear proceso suspendido sin consola, asignar al Job antes de reanudar, no heredar handle de Job ni permitir *breakaway*. Si no puede asignarse, abortar antes de ejecutar. Registrar al CLI padre y sus hijos; un hijo servidor de Node es normal. |
| Arrancando → listo | CPA con YAML v8 y `GET /v1/models` autenticado; luego `node.exe` privado + `bin/omniroute.mjs serve --no-open --no-tray`, sin daemon. Su [CLI](https://github.com/cheaper-inference/OmniRoute/blob/v3.8.50/bin/cli/commands/serve.mjs) tiene recuperación propia y puede lanzar `node` por nombre: PATH privado para sus hijos y un único presupuesto de reinicios. Verificar en el artefacto el efecto de `--no-recovery` antes de delegar toda recuperación al host. `GET /healthz` y catálogo autenticado; luego OpenCode `serve` en loopback, contraseña por instancia, `/global/health`. Sondas con plazo/cancelación; enlazar listener con PID del Job **antes** de enviar credenciales. |
| Listo → degradado | La salud del proceso no implica acceso a proveedor. Distinguir sin cuentas, credencial caducada, cupo, ABI incorrecta, catálogo obsoleto y caída. Aislar fallo por componente. Propuesta inicial: máximo 3 reinicios/5 min con espera creciente y variación; después reparación manual en UI. No reejecutar automáticamente un turno o una herramienta con efectos. |
| Listo → cerrando | «Cerrar ventana» conserva servicios solo en modo bandeja consentido, destruye paneles y desuscribe eventos. «Salir», fin de sesión o actualizar: bloquear nuevos turnos, esperar o cancelar con permiso, drenar escritura de sesiones; cerrar agente, gateway y CPA en orden inverso. Usar parada cooperativa comprobada por binario; pasado el plazo, terminar el Job propio y esperar handles. No asumir semántica POSIX de SIGTERM en Windows. |
| Caída/arranque posterior | Último handle cerrado debe retirar descendientes; verificar handles y puertos al recuperar. Reabrir la última generación sana y reconciliar operaciones incompletas. Procesos del navegador del usuario y WebView2 compartidos se liberan vía su controlador, nunca por nombre de ejecutable. |

Base Win32: [Jobs y anidamiento](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects). El Job delimita propiedad, no es una sandbox de seguridad para plugins. **?** Confirmar procesos auxiliares, posibles jobs externos del runner, consola/PTY y método de cierre limpio para cada binario. Si el panel permite iniciar otro CPA, bloquear las operaciones de ciclo de vida en la vista gestionada con controles efectivos de servidor/mediador, no solo ocultar botones. Si no es viable, ofrecer solo funciones seguras de gestión mediante adaptador propio y mantener ese panel fuera del perfil gestionado.

### Instalación, versiones y datos

**P:** EXE NSIS firmado, por usuario, sin red para el conjunto de runtimes principal. Carga útil: supervisor y assets, ZIP CPA y panel verificado, runtime Node privado y paquete OmniRoute preparado para su ABI (incluido `better-sqlite3` si lo usa), CLI OpenCode, UI y plugin fijado con sus dependencias. Preparar en máquina de build; sin `npm install`, compilador C++ ni descarga `latest` en el equipo final. Detectar WebView2 Evergreen compatible; si falta, usar redistribuible offline oficial incluido si lo permiten licencia y política. No eliminar WebView2 compartido. [Opciones NSIS/WebView2 de Tauri](https://v2.tauri.app/distribute/windows-installer/) y [distribución Microsoft](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).

| Área propuesta | Contenido y propietario |
|---|---|
| `%LOCALAPPDATA%\Programs\AI Stack Desktop\versions\<conjunto>` | Carga útil inmutable para la aplicación; el launcher elige solo conjuntos con manifiesto firmado. Sin añadir al PATH global. |
| `%LOCALAPPDATA%\AI Stack Desktop\profiles\<id>` | Configuración generada, estado de aprovisionamiento, directorios independientes de CPA, OmniRoute y OpenCode, sesiones y snapshot del catálogo. ACL del usuario; los formatos internos son propiedad de cada upstream. |
| Área operativa separada | Caché WebView por origen de confianza, métricas y logs rotativos, staging y copia consistente de datos durante actualización. Abrir un proyecto guarda una referencia; nunca se copia ni limpia el proyecto entero. |

**P · Actualizar como conjunto:** manifiesto firmado por el producto con versión, plataforma, hashes, esquema de datos y compatibilidad. Comprobar firma, caducidad/anti-replay, canal, espacio y cada archivo antes de activar. Descargar a staging junto al destino, impedir traversal/reparse points, preservar el conjunto A. Esperar a que el usuario cierre actividad, detener hijos y tomar copia consistente y protegida de datos/config (incluidos los ficheros auxiliares de la base **estando parada**). Ensayar B con copia de datos; solo tras salud, catálogo y reapertura de una sesión de prueba cambiar puntero por reemplazo atómico. Diario durable de etapas para recuperar tras corte de alimentación. Si falla, arrancar A con sus datos compatibles. Tras uso real de B, nunca retroceder sobre datos migrados ni descartar sesiones nuevas en silencio: exportar/conservar B y requerir confirmación para restaurar el snapshot. Desactivar autoactualizadores independientes, incluidos paneles y plugins incluidos. Evergreen compartido se valida aparte.

**P · Desinstalar:** verificar inventario y propiedad, detener solo Jobs propios, retirar ejecutables, accesos, registro de desinstalación y staging. Ofrecer conservar sesiones/configuración y explicar cómo borrar datos y credenciales por perfil; purga explícita elimina también caché, logs, copias y entradas protegidas propias. Revocación remota se hace antes si está disponible. No tocar otras instalaciones, proyectos ni runtime compartido. **?** Firma Authenticode, política corporativa y matriz offline requieren VM de Windows limpia. Runners Windows de GitHub Actions para reproducibilidad y firmas bajo entorno protegido; escritorio interactivo, callbacks y recursos se califican en VM/equipo dedicado.
