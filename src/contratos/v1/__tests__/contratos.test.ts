import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import Ajv from 'ajv';
import { describe, expect, it } from 'vitest';
import { catalogoTropas } from '../catalogoTropas';
import {
  ASIGNACION,
  HEROE,
  HEROE_BOT,
  HEROE_PUBLICO,
  INCORPORACION,
  INICIO,
  RESULTADO_ASEDIO,
  TICKET_ASEDIO,
  TICKET_BANDIDOS,
  TOKENS,
} from '../fixtures';

const DIR = join(dirname(fileURLToPath(import.meta.url)), '..');
const leer = (archivo: string): unknown => JSON.parse(readFileSync(join(DIR, archivo), 'utf8'));
const versionDe = (x: unknown) => (x as { version?: number } | undefined)?.version;

// `strictRequired` es una comprobación de estilo de ajv, no de validez: rechaza el `if: { required: [...] }` con
// el que el schema distingue equipo único de apilable, que es JSON Schema estándar.
const ajv = new Ajv({ allErrors: true, strict: true, strictRequired: false, allowUnionTypes: true });
ajv.addSchema(leer('contratos.schema.json') as object, 'v1');

function erroresDe(definicion: string, dato: unknown) {
  const validar = ajv.getSchema(`v1#/definitions/${definicion}`);
  if (!validar) throw new Error(`El schema no define ${definicion}`);
  return validar(dato) ? [] : validar.errors;
}

/** Lo que se publica para Conquest: cada archivo es el dato de la derecha, serializado, y cumple su definición. */
const PUBLICADOS: [archivo: string, definicion: string, dato: unknown][] = [
  ['catalogoTropas.json', 'CatalogoTropas', catalogoTropas()],
  ['fixtures/heroe.json', 'Heroe', HEROE],
  ['fixtures/heroe.bot.json', 'Heroe', HEROE_BOT],
  ['fixtures/heroePublico.json', 'HeroePublico', HEROE_PUBLICO],
  ['fixtures/battleTicket.asedio.json', 'BattleTicket', TICKET_ASEDIO],
  ['fixtures/battleTicket.bandidos.json', 'BattleTicket', TICKET_BANDIDOS],
  ['fixtures/incorporacionBatalla.json', 'IncorporacionBatalla', INCORPORACION],
  ['fixtures/battleServerAssignment.json', 'BattleServerAssignment', ASIGNACION],
  ['fixtures/tokensBatalla.json', 'TokensBatalla', TOKENS],
  ['fixtures/inicioBatalla.json', 'InicioBatalla', INICIO],
  ['fixtures/battleResult.json', 'BattleResult', RESULTADO_ASEDIO],
];

describe.each(PUBLICADOS)('%s', (archivo, definicion, dato) => {
  it(`cumple la definición ${definicion}`, () => {
    expect(erroresDe(definicion, dato)).toEqual([]);
  });

  it('es idéntico al JSON publicado (regenerar con ACTUALIZAR_CONTRATOS=1)', () => {
    const ruta = join(DIR, archivo);
    const publicado = existsSync(ruta) ? leer(archivo) : undefined;
    if (process.env.ACTUALIZAR_CONTRATOS && !isDeepStrictEqual(publicado, dato)) {
      // Lo que lleva `version` (el catálogo) no se regenera sin subirla: es como sabe Conquest que cambió.
      if (versionDe(dato) !== undefined) {
        expect(versionDe(dato), `sube la versión de ${archivo}`).toBeGreaterThan(versionDe(publicado) ?? 0);
      }
      writeFileSync(ruta, JSON.stringify(dato, null, 2) + '\n');
    }
    expect(leer(archivo)).toEqual(dato);
  });
});

describe('el schema rechaza', () => {
  it('campos que no existen', () => {
    expect(erroresDe('BattleTicket', { ...TICKET_ASEDIO, autorizacion: 'x' })).not.toEqual([]);
  });

  it('un equipamiento al que le falta un hueco', () => {
    const { botas: _, ...sinBotas } = HEROE.equipamiento;
    expect(erroresDe('Heroe', { ...HEROE, equipamiento: sinBotas })).not.toEqual([]);
  });

  it('XP negativa o no entera', () => {
    for (const xpGanada of [-1, 2.5]) {
      const porHeroe = [{ ...RESULTADO_ASEDIO.porHeroe[1]!, xpGanada }];
      expect(erroresDe('BattleResult', { ...RESULTADO_ASEDIO, porHeroe })).not.toEqual([]);
    }
  });

  it('equipo único con cantidad distinta de 1, y estadísticas en un objeto apilable', () => {
    const base = { itemDefinitionId: 'x', tipo: 'arma', precio: 1, casillaInventario: 0 };
    expect(erroresDe('ItemInstancia', { ...base, cantidad: 2, itemInstanceId: 'i-1' })).not.toEqual([]);
    expect(erroresDe('ItemInstancia', { ...base, cantidad: 2, estadisticas: [] })).not.toEqual([]);
  });
});

it('el resultado de ejemplo cuadra con su ticket y su incorporación (doc 02 §3.3, puntos 1, 4 y 5)', () => {
  const bandos = [TICKET_ASEDIO.bandos.atacante, TICKET_ASEDIO.bandos.defensor];
  const escuadras = [
    ...bandos.flatMap((b) => [...b.participantes.flatMap((p) => p.escuadras), ...b.escuadrasSinHeroe]),
    ...INCORPORACION.participante.escuadras,
  ];
  expect(RESULTADO_ASEDIO.battleId).toBe(TICKET_ASEDIO.battleId);
  expect(RESULTADO_ASEDIO.ticketRevision).toBe(TICKET_ASEDIO.ticketRevision);
  expect(RESULTADO_ASEDIO.porEscuadra.map((e) => e.squadId).sort()).toEqual(escuadras.map((e) => e.squadId).sort());
  for (const e of RESULTADO_ASEDIO.porEscuadra) {
    const autorizados = escuadras.find((s) => s.squadId === e.squadId)!.efectivosAutorizados;
    expect(e.supervivientesAlCierre + e.muertos).toBe(autorizados);
  }
});
