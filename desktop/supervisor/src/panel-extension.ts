// Extensión de comandos de Pi: no recibe claves, URLs ni texto del modelo.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';

export default function (pi: ExtensionAPI) {
  for (const [name, description] of Object.entries({
    cuentas: 'Abrir Cuentas y administración', cliproxyapi: 'Abrir el panel de CLIProxyAPI',
    omniroute: 'Abrir el panel de OmniRoute',
  })) {
    pi.registerCommand(name, { description, handler: async () => {
      writeFileSync(join(process.env.GENTLE_SHELL_HOME!, '..', `abrir-${name}`), '1');
    }});
  }
}
