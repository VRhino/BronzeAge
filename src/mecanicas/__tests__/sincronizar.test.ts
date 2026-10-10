// `Docs/Mechanics/` copia sus cifras de `constants.ts`: si una cambia y el documento no, esto falla y dice qué hacer (`npm run mecanicas`).
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parametrosDe, sincronizar } from '../sincronizar';

const RAIZ = join(fileURLToPath(new URL('../../../', import.meta.url)), 'Docs', 'Mechanics');

function documentos(directorio: string): string[] {
  return readdirSync(directorio).flatMap((nombre) => {
    const ruta = join(directorio, nombre);
    return statSync(ruta).isDirectory() ? documentos(ruta) : ruta.endsWith('.md') ? [ruta] : [];
  });
}

describe('sincronizar bloques de parámetros', () => {
  it('reescribe un bloque con el valor actual de la constante y deja el resto', () => {
    const viejo = 'antes\n<!-- parametros:CAPITAL -->\n```json\n{ "viejo": 1 }\n```\n<!-- /parametros -->\ndespués';
    const { texto, desconocidos } = sincronizar(viejo);
    expect(desconocidos).toEqual([]);
    expect(texto).toBe(`antes\n<!-- parametros:CAPITAL -->\n\`\`\`json\n${parametrosDe('CAPITAL')}\n\`\`\`\n<!-- /parametros -->\ndespués`);
  });

  it('avisa de un nombre que no existe en constants.ts y no lo toca', () => {
    const bloque = '<!-- parametros:NO_EXISTE -->\n```json\n1\n```\n<!-- /parametros -->';
    expect(sincronizar(bloque)).toEqual({ texto: bloque, desconocidos: ['NO_EXISTE'] });
  });
});

describe('Docs/Mechanics está al día con constants.ts', () => {
  for (const ruta of documentos(RAIZ)) {
    it(ruta.slice(RAIZ.length + 1), () => {
      const original = readFileSync(ruta, 'utf-8').replace(/\r\n/g, '\n');
      const { texto, desconocidos } = sincronizar(original);
      expect(desconocidos, 'bloques de parámetros que no existen en constants.ts').toEqual([]);
      expect(texto, 'una cifra de constants.ts cambió: ejecuta `npm run mecanicas`').toBe(original);
    });
  }
});
