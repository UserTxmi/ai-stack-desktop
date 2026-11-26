// Ensayo: la API de OpenCode acepta un nombre de variante arbitrario. No dejar que
// un turno explícito del proveedor de prueba se degrade al valor por defecto.
import {writeFile} from 'node:fs/promises';
export default async (_input, {providerID, modelID, efforts, rejected}) => ({
  'chat.message': async ({model, variant}) => {
    if (model?.providerID === providerID && model.modelID === modelID && variant && !efforts.includes(variant)) {
      await writeFile(rejected,'W1_UNSUPPORTED_VARIANT',{flag:'wx'});
      throw new Error('W1_UNSUPPORTED_VARIANT');
    }
  },
});
