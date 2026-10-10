// Reescribe los bloques de parámetros de `Docs/Mechanics/**.md` con los valores actuales de `constants.ts` (`npm run mecanicas`).
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { sincronizar } from '../src/mecanicas/sincronizar';

const RAIZ = join(fileURLToPath(new URL('..', import.meta.url)), 'Docs', 'Mechanics');

function documentos(directorio: string): string[] {
  return readdirSync(directorio).flatMap((nombre) => {
    const ruta = join(directorio, nombre);
    return statSync(ruta).isDirectory() ? documentos(ruta) : ruta.endsWith('.md') ? [ruta] : [];
  });
}

let cambiados = 0;
for (const ruta of documentos(RAIZ)) {
  const original = readFileSync(ruta, 'utf-8');
  const { texto, desconocidos } = sincronizar(original);
  if (desconocidos.length > 0) console.error(`${ruta}: no existe en constants.ts → ${desconocidos.join(', ')}`);
  if (texto !== original) {
    writeFileSync(ruta, texto, 'utf-8');
    cambiados++;
    console.log(`actualizado: ${ruta}`);
  }
}
console.log(cambiados === 0 ? 'Todo al día.' : `${cambiados} documento(s) actualizado(s).`);
