# Fase 2B · AI Stack Desktop 0.0.2-dev · Windows x64

La carpeta elegida al extraer el ZIP es la raíz efectiva. Abra `bin\ai-stack-desktop.exe` y pulse **Preparar o reintentar**. La aplicación detecta/reutiliza los componentes íntegros, descarga e instala los faltantes y habilita el arranque tras verificar W1. Véanse la [guía de Windows 11](../desktop/PROBAR.md) y el [workflow de Actions](https://github.com/UserTxmi/ai-stack-desktop/actions/workflows/windows-w1.yml).

**[Descargar Windows x64 0.0.2-dev](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45905213262/artifacts/15158390656)** · [CI completa aprobada](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45905213262) · commit [`8d1919f`](https://github.com/UserTxmi/ai-stack-desktop/commit/8d1919f146b2b53898e83c1b11e90cf931324d16) · disponible hasta el **26-12-2026 22:41 UTC**, con sesión iniciada en GitHub.

Se descargó el artefacto y se cotejaron su SHA-256 y los nueve archivos de `SHA256SUMS.txt`. ZIP: **3 246 571 bytes**, SHA-256 `ae0d906bf9559b715a2a96e52fe1fb8f8e1b596c78d1361cdca379cddb38df6a`. EXE: **9 839 616 bytes**, PE32+ Windows GUI x86-64, SHA-256 `49629e863cfb7645c6e263386095a264dc43f42485889a46ef48f1ef9f162bbb`.

## Evidencia por entorno

- **Windows 11 físico, 2A:** [prueba interactiva básica comunicada por el responsable](PHASE2A_REPORT.md#prueba-interactiva-básica-en-windows-11). Tauri, W1, catálogo `low/high`, respuesta sintética y tres Jobs vacíos en orden inverso. Picos individuales 75,4 / 629,7 / 990,5 MiB para CPA / OmniRoute / OpenCode; no son mediciones simultáneas ni una certificación.
- **Desarrollo local:** compilación TypeScript, comprobación cruzada Rust para Windows x64, siete pruebas del receptor sintético y cotejo de los cinco resultados del backport contra las huellas W1. No equivalen a ejecutar el EXE en Windows.
- **Windows Server, 2B:** `windows-2025`, Windows Server 2025 Datacenter `10.0.26100` AMD64; job de **11 min 34 s**, todos los pasos aprobados. MSVC x64 y tres pruebas unitarias: descarga interrumpida/corrupta, no adoptar carpetas ajenas y no adoptar un listener ajeno. El mismo EXE preparó una raíz nueva con espacios y `ñ`, superó reapertura, repetición sin caché, corrupción de EXE y caché, staging parcial, ACL denegada y cierre normal/forzado de los cuatro Jobs, incluido el proveedor sintético. La repetición reutilizó los cinco componentes con **cero descargas y cero instalaciones**; ante corrupción de CPA sólo se obtuvo y sustituyó CPA. `preparacion-ci.txt`, `backend.txt` e `informe.json` están en el ZIP.
- **Regresiones W1 en el árbol instalado por 2B:** independientes de CPA, OpenCode y OmniRoute, plugin publicado y cadena completa aprobados. Cola agotada en **15,078 s**, cero despachos de la petición vencida, slot liberado. Catálogo efectivo `low/high`, `medium` rechazado, principal `low`/título `low` y principal `high`/título `low`, cero peticiones sin atribuir. SSE `late`: primer contenido/fin **19,281/40,359 s**; `body`: **18,078/39,250 s**. Cancelación aprobada en el receptor.
- **Windows 11 interactivo, 2B:** pendiente de ejecutar el [guion manual](../desktop/PROBAR.md). CI es headless; no prueba el foco, la ventana ni el comportamiento de WebView2 en el equipo del usuario. La validación amplia de 1B-Desktop tampoco se da por cerrada.

**Primer run ([45904153696](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45904153696)).** Pasaron las pruebas unitarias, la compilación MSVC, la batería headless del EXE y el catálogo del plugin. Las tres pruebas independientes se detuvieron antes de arrancar: el antiguo script de preparación creaba `.lab` como efecto lateral y el EXE usa su propia raíz. El laboratorio crea ahora `.lab` explícitamente antes de guardar sus ensayos. No se emitió un ZIP en ese run.

## Preparación y recuperación

- Inventario incluido en el EXE: CPA **8.1.7**, OmniRoute **3.8.54**, Node **24.21.0**, npm **11.19.0**, SQLite **13.0.3**, OpenCode **1.19.6** y plugin **0.2.2**. [URLs y huellas de origen](../experiments/phase1/versions.json), [huellas W1 de los archivos activos](../desktop/w1.json), [backport de cola por grafo](../desktop/w1-backport.json). Los EXE de CPA y OpenCode se identifican por SHA-256 del binario de la versión fijada. Node ejecuta además una consulta SQLite en memoria y comprueba plataforma/ABI `x64 137`.
- HTTPS con WinHTTP y validación de sistema para los seis archivos directos. Se muestran bytes reales y total si la cabecera existe. Cada descarga se guarda en `.part`; sólo pasa a la caché tras verificarse. Si falla se descarta ese parcial y se reutilizan las descargas completas, sin reanudación por rango. Una caché corrupta se vuelve a obtener sólo si ese componente necesita reparación. npm verifica los SRI del lock para SQLite, plugin y dependencias; usa el Node privado, `npm ci --ignore-scripts`, opciones fijadas y caché local. Su progreso se muestra por etapa, sin simular porcentajes.
- Publicación por componente: extraer en `preparacion/trabajo`, validar el resultado, pasar a `runtimes` por renombrado en el mismo volumen. El siguiente intento elimina el staging inconcluso y conserva las unidades ya publicadas que pasen la verificación. El bloqueo de archivo exclusivo une preparación y supervisión por instalación. Las versiones, hashes de entradas, manifests y enlaces workspace se verifican al reabrir y antes de iniciar. No es una auditoría de cada archivo transitorio de npm tras la instalación.
- El backport valida preimágenes, anclas únicas y los cinco hashes de salida. Sigue el [criterio de retirada W1](DECISIONS.md). El laboratorio 2A queda como antecedente reproducible; los scripts históricos no van en el ZIP. Los subprocesos de preparación y los de la demo quedan en Jobs de Windows con cierre al perder el propietario.

## Organización de archivos

| Carpeta | Contenido |
| --- | --- |
| `bin/` | EXE propio |
| `runtimes/` | `cpa`, `node`, `oc`, `package` (OmniRoute/SQLite), `peers` (plugin) |
| `config/` | Configuraciones sintéticas producidas por arranque |
| `datos/` | WebView2 persistente y perfiles sintéticos por arranque |
| `cache/` | Descargas verificadas, cache npm y cachés de demo |
| `logs/` | Resúmenes propios y registro local de npm acotado a cinco archivos |
| `preparacion/` | Árbol transitorio, TMP/TEMP de los procesos y auxiliares efímeros |

La raíz tiene un marcador de propiedad/bloqueo. Se requiere disco local NTFS y una carpeta nueva o ya gestionada por la app. Las rutas propias se mantienen debajo de la raíz con entornos privados y PATH sólo para cada proceso; no se modifica el PATH del host. Los enlaces de workspace NTFS impiden trasladar un árbol preparado sin reparación: para el ensayo, extraiga una copia limpia en la nueva ubicación. Las carpetas asignadas a otra instalación no se adoptan. Los perfiles sintéticos `demo-*` se limpian al detener normalmente; tras un cierre abrupto pueden quedar residuos en esa misma raíz.

**Excepciones de plataforma:** Windows mantiene su almacén de certificados, resolución/red y metadatos de ejecución; WebView2 Runtime se instala y actualiza a nivel de Windows/Microsoft. La carpeta de datos de WebView2 se fija por proceso a `datos/webview2`; políticas corporativas de Edge pueden imponer otra ubicación. Cualquier caché interna de Next ligada a su distribución se mantiene debajo de `runtimes/package`, además de la caché de perfil configurada en `cache/`. La descarga necesita acceso HTTPS a los orígenes del inventario y a npm; la adquisición nativa no hereda credenciales/proxy de aplicaciones ajenas.

El ZIP contiene el EXE propio, sus avisos, la guía, inventarios, versión de commit, checksums y sólo evidencia filtrada. No incluye dependencias W1 distribuidas ni datos de usuario. Es un artefacto de Actions para 30 días, sin release ni instalador definitivo.
