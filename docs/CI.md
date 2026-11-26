# Validación pública Windows W1

`windows-w1.yml` usa **un runner estándar `windows-2025` x64**, no un runner grande. Captura la edición, versión y build reales con CIM. En GitHub público el cómputo del runner estándar no descuenta cuota de minutos privada ([facturación oficial](https://docs.github.com/en/billing/concepts/product-billing/github-actions)). La API de uso de la cuenta no estaba disponible con los scopes actuales; no confundir consumo observado del job con facturación. No se activó pago adicional. Caché incluida: 10 GB por repo; vigilar `gh api repos/UserTxmi/ai-stack-desktop/actions/cache/usage`. Al aproximarse a 8 GB, revisar entradas antes de ampliar cachés. No solicitar presupuestos ni permisos nuevos desde el job.

**Cuándo corre:** push en `main` para implementación 2A, laboratorio o workflow; dispatch sólo con nueva evidencia. Concurrencia por rama y límite de 50 minutos. Acciones oficiales por SHA, checkout sin persistir credenciales, `contents: read`. Caché de crates/compilación y cuatro familias de descargas verificadas. Ningún perfil del stack en caché.

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

**Evidencia pública:** JSON W1 por lista permitida, 1 día. Para 2A además el artefacto de 30 días con EXE propio, preparación, checksums y evidencia; sin runtimes W1, perfiles o logs crudos. Sin GitHub Releases. En el laboratorio `private_bytes` es compromiso privado y `cpu_s` CPU acumulada, no porcentaje.

**Selección:** push relevante → completa; dispatch focal disponible para diagnóstico sin EXE publicado. La completa exige regresiones y backend nativo. El Node de Actions y el privado de W1 son distintos.
