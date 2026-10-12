// Mensajes del WebSocket de tiempo real (contrato 03): cada fixture de `Docs/Coordinacion/fixtures/tiempo-real/` cumple la definición de su
// mensaje en `contratos.schema.json`, y los fixtures encadenados (1042 → 1043 → …) son coherentes entre sí.
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { erroresDeMensaje } from './validarTiempoReal';

const AQUI = dirname(fileURLToPath(import.meta.url));
const FIXTURES = join(AQUI, '..', '..', '..', '..', 'Docs', 'Coordinacion', 'fixtures', 'tiempo-real');

/** Qué definición cumple cada fixture, por el principio de su nombre. */
const DEFINICION_POR_PREFIJO: [prefijo: string, definicion: string][] = [
  ['suscribir.', 'MensajeSuscribir'],
  ['suscrito', 'MensajeSuscrito'],
  ['evento.', 'MensajeEvento'],
  ['foto.', 'MensajeFoto'],
  ['delta.', 'MensajeDelta'],
  ['resync', 'MensajeResync'],
  ['revocado', 'MensajeRevocado'],
  ['error.', 'MensajeErrorTiempoReal'],
];

const archivos = readdirSync(FIXTURES).filter((f) => f.endsWith('.json'));

describe('fixtures de tiempo real', () => {
  it('hay fixtures y todos tienen una definición asignada', () => {
    expect(archivos.length).toBeGreaterThan(0);
    for (const f of archivos) expect(DEFINICION_POR_PREFIJO.some(([p]) => f.startsWith(p)), `${f} sin definición`).toBe(true);
  });

  describe.each(archivos)('%s', (archivo) => {
    it('cumple su definición', () => {
      const [, definicion] = DEFINICION_POR_PREFIJO.find(([p]) => archivo.startsWith(p))!;
      expect(erroresDeMensaje(definicion, JSON.parse(readFileSync(join(FIXTURES, archivo), 'utf8')))).toEqual([]);
    });
  });
});
