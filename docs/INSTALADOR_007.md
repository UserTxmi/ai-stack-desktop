# 0.0.7-dev · evidencia Windows x64

[Run final, commit `854872e`](https://github.com/UserTxmi/ai-stack-desktop/actions/runs/45950091803). Inno Setup 6.7.1. ZIP público `117270fb67b5f244796db766f3f2a34f94c42eef4cc0a7945af4b2a4cd9ee4e5` comprobado por descarga anónima; contiene sólo el instalador. [Instrucciones](../desktop/INSTALAR.md).

Se instaló en una ruta con espacio y `ñ`, se comprobaron los accesos de Inicio y escritorio, se abrió dos veces con respuesta sintética y se desinstaló conservando el perfil. La actualización usó el portable comprobado de 0.0.6-dev con una sesión existente y `codex-auto-review` guardado como modelo inicial. La nueva sesión respondió con la ruta conversacional de prueba; reinstalar y desinstalar preservó la bóveda DPAPI, el registro del companion y la sesión anterior. No quedaron procesos de la muestra después del cierre. No se utilizaron cuentas reales ni se repitieron G1/G2/W1.

## Una muestra por escenario, mismo runner Windows Server 2025 x64

| Versión y escenario | Hasta GPUI | Hasta respuesta | Procesos | MiB privados | Working set bruto, MiB | CPU (100 % = 1 núcleo) |
|---|---:|---:|---:|---:|---:|---:|
| 0.0.6 limpio | 168,4 s | 194,2 s | 24 | 1290 | 1469 | 205,4 % |
| 0.0.7 limpio | 881,4 s | 909,3 s | 18 | 1383 | 1485 | 152,0 % |
| 0.0.6 reapertura | 10,6 s | 20,0 s | 24 | 1038 | 1162 | 228,4 % |
| 0.0.7 reapertura | 11,7 s | 22,1 s | 18 | 1036 | 1125 | 225,0 % |
| 0.0.7 sobre perfil 0.0.6 | 10,1 s | 18,8 s | 18 | 1055 | 1146 | 233,4 % |

La muestra incluye el supervisor y sus descendientes (también el receptor sintético). CPU: ventana breve después del turno, no reposo prolongado; working set bruto puede contar páginas compartidas varias veces. Se quitaron seis procesos con creación suspendida y asignación a Jobs antes de reanudar. No se demuestra una reducción general de CPU o memoria privada. Se evitó releer los mismos binarios por sus enlaces duros, se suprimió la sonda Node/SQLite repetida y se redujo la frecuencia del refresco automático una vez que hay catálogo.

**Disco del perfil tras reapertura:** runtimes 2828,3 → 2826,4 MiB; caché 1019,3 → 943,9 MiB; datos sintéticos 123,4 → 126,3 MiB. Bytes de archivos distintos, sin seguir junctions ni duplicar enlaces duros; no incluye la carpeta del instalador. Se deduplicó SQLite y se descartó el ZIP del SDK Go después de preparar una instalación nueva. La caché del portable de recuperación se conserva.

**Limitación principal: preparación fría.** En el runner se comparó el portable del volumen temporal con una instalación nueva en LocalAppData: almacenamiento y rutas distintos. La instalación de dependencias privadas de OmniRoute por npm concentró la mayor espera. Para quien ya tenga 0.0.6-dev, reutilizar el portable evita esa preparación. Estas observaciones no fijan un presupuesto para Windows 11. Quedan para escritorio la interacción con los paneles, un equipo que realmente carezca del redistribuible VC++ y el consumo en reposo prolongado.
