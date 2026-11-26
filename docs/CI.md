# Validación pública Windows W1

`windows-w1.yml` usa **un runner estándar `windows-2025` x64**, no un runner grande. Captura la edición, versión y build reales con CIM. En GitHub público el cómputo del runner estándar no descuenta cuota de minutos privada ([facturación oficial](https://docs.github.com/en/billing/concepts/product-billing/github-actions)). La API de uso de la cuenta no estaba disponible con los scopes actuales; no confundir consumo observado del job con facturación. No se activó pago adicional. Caché incluida: 10 GB por repo; vigilar `gh api repos/UserTxmi/ai-stack-desktop/actions/cache/usage`. Al aproximarse a 8 GB, revisar entradas antes de ampliar cachés. No solicitar presupuestos ni permisos nuevos desde el job.

**Cuándo corre:** push en `main` solo para rutas del laboratorio headless o el propio workflow; `workflow_dispatch` para una comprobación explícita. Concurrencia por rama, cancelación de obsoletos y límite de 30 minutos. Ni PR de forks ni cambios exclusivos de docs ejecutan binarios. Actions oficiales fijadas por SHA contrastado con su etiqueta; checkout sin persistir credenciales, `contents: read`. El token del job no se entrega al stack. Sólo se cachean cuatro familias pequeñas de descargas públicas, con clave de arquitectura/versión y hashes de `versions.json` y lock del plugin. Cada uso verifica el contenido; el lock transitivo de OmniRoute está fijado por hash en el manifiesto. Nunca se restaura un perfil ni un entorno ya ejecutado.

**Pruebas:** adquisición de binarios, CPA y OpenCode independientes, construcción del árbol npm privado sin lifecycle scripts, SQLite nativo, OmniRoute independiente, plugin publicado y sesión completa con proveedor loopback. Fallar una dependencia omite los casos que la necesitan, sin darles por aprobados. El modo hospedado tiene acceso a internet; no pedir la UI con fallback remoto ni usar `--isolated-offline`. PTY, WebView2, Electron, OAuth real y presupuesto de RAM quedan fuera de este job.

```sh
gh repo view UserTxmi/ai-stack-desktop --json nameWithOwner,visibility,defaultBranchRef
gh run list --workflow windows-w1.yml --limit 5
# Solo si no existe ya un run para el cambio que se quiere verificar:
gh workflow run windows-w1.yml --ref main
gh run watch ID --interval 30 --exit-status
gh run view ID --json jobs,conclusion,url,headSha
gh run download ID --pattern 'w1-informe-*' --dir experiments/phase1/.lab/ci-results/ID
```

**Evidencia pública:** artefacto de 1 día con un único JSON construido a partir de nombres de caso, estados, tiempos, esfuerzos sintéticos y agregados por rol. `private_bytes` es compromiso privado; `working_set` puede incluir páginas compartidas; `cpu_s` es un máximo por muestra de CPU acumulada de procesos vivos, no porcentaje. No exportar stdout de proveedores, cabeceras, homes, bases, trazas, conversaciones ni binarios. Para futuros portables/instaladores, añadir un job independiente sólo tras aprobación de fase, inventario de licencias y procedencia. En esta fase no hay pipeline de releases.
