# Producto y alcance

**P · Usuario inicial:** desarrollador con un equipo Windows 11 x64 mantenido, cuenta estándar y al menos un método de acceso permitido por el proveedor. **S:** puede completar el consentimiento en su navegador. Un proveedor que exija una capacidad o licencia no disponible figurará como no compatible. Instalar la aplicación no otorga acceso a modelos.

| Recorrido propuesto | Resultado observable |
|---|---|
| Instalar y abrir | Un EXE instala los runtimes privados y comprueba su integridad. Primera apertura sin PATH, terminal, edición de YAML/JSON, Node global ni credenciales anteriores. Se distinguen «listo» y «sin conexiones». |
| Conectar | Elegir proveedor y cuenta; consentimiento OAuth oficial en navegador o entrada de clave autorizada en el flujo correspondiente. Ver estado, cancelar, volver a autorizar, desactivar o desconectar. Indicar si hace falta revocar también en el proveedor. |
| Elegir | Ver catálogo efectivo de esa conexión, origen, capacidades conocidas, contexto y esfuerzo compatibles. Distinguir modelo, alias y combo; mostrar última sincronización, indisponibilidad y cuota sin cambiar de cuenta a escondidas. |
| Trabajar | Elegir carpeta de proyecto; iniciar o reabrir sesión persistente; ver agente, herramientas, progreso y solicitudes de permiso; aceptar, denegar o cancelar. Mostrar qué petición queda interrumpida si falla el gateway. |
| Extender | Mantener los plugins, MCP y skills compatibles de OpenCode. Mostrar procedencia, requisitos de runtime y alcance por proyecto. La instalación o activación de código de terceros es decisión del usuario. |
| Administrar | Abrir paneles upstream aislados, ver salud por componente y métricas, exportar diagnóstico revisable. Cerrar una vista no debe perder la sesión; «Salir» explica si hay trabajo activo. |
| Mantener | Actualización controlada del conjunto; recuperación tras fallo y elección explícita sobre datos al desinstalar. El directorio de proyecto nunca es propiedad del desinstalador. |

**P · Límites del primer lanzamiento:** instalación por usuario, un perfil gestionado y un conjunto de versiones aprobado; interfaz en primer plano o bandeja voluntaria. No se ofrece un gateway de red ni una granja de cuentas. El acceso a proveedores requiere red; arranque, diagnóstico, catálogo en caché identificado como tal y reapertura local deben funcionar sin ella.

**? · Antes de prometer compatibilidad:** matriz de proveedores y planes permitidos, requisitos de herramientas externas por plugin, compatibilidad ARM64 y paridad de terminal integrada nativa. Un artefacto Windows upstream no resuelve por sí solo estas preguntas. Véanse [Pruebas](TESTING.md) y [Hoja de ruta](ROADMAP.md).
