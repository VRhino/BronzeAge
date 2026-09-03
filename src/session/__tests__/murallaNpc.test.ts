// Cierra el mismo agujero que `nucleoMilitarNpc.test.ts` documenta para Barracón/Galería, pero para el
// recinto de muralla (`Consideraciones/Murallas_Definicion.md`, Paso 2c): sin `asegurarMuralla`, ninguna
// Facción del batch sin jugador humano llegaría jamás a tener un recinto, y el gate de nivel 4 que lo exige
// (Paso 5) quedaría muerto para siempre.
import { describe, expect, it } from 'vitest';
import { instanteDeTest } from '../../engine/__tests__/fixtures';
import type { Asentamiento } from '../../domain/types';
import type { EstadoSimulacion } from '../../engine/simulation';
import { avanzarNpcGobernanza } from '../npcGobernanza';
import { contextoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from '../../engine/__tests__/fixtures';
import { createRng } from '../../worldgen';
import { ZONA_INFLUENCIA } from '../../constants';

const SEED = 42;

/** Asentamiento de nivel 3 con almacén de sobra — lo único que puede hacer o no aparecer un recinto es la
 * lógica de gobernanza bajo prueba, no falta de nivel ni de fondos. Mismo criterio que
 * `nucleoMilitarNpc.test.ts`: `radioPotencial` se fuerza al tope porque sube con obra completada, no con el
 * nivel a secas, y el fixture no simula ninguna. */
function estadoBase(nivel: number): { estado: EstadoSimulacion; mapa: ReturnType<typeof crearMapaDeterminista>; faccionId: string } {
  const mapa = crearMapaDeterminista(SEED);
  const facciones = crearFacciones();
  const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
  const abundante: Asentamiento = {
    ...asentamiento,
    nivel,
    nivelActual: nivel,
    radioPotencial: ZONA_INFLUENCIA.radioMaximoPorNivel[Math.min(nivel, 3)]!,
    almacen: Object.fromEntries(
      Object.entries(asentamiento.almacen).map(([recurso, item]) => [recurso, { ...item, cantidad: 2000, capacidad: 2000 }])
    ),
  };
  return {
    estado: {
      asentamientos: [abundante],
      facciones: faccionesTrasFundar,
      caravanas: [],
    ejercitos: [],
      acuerdos: [],
      ordenes: [],
      relaciones: [],
      titulos: [],
      caminos: [],
      campamentosBandidos: [],
      bandidosProximoSpawnEn: instanteDeTest(0),
    },
    mapa,
    faccionId: 'faccion-1',
  };
}

describe('Gobernanza NPC — primer recinto de muralla (Paso 2c)', () => {
  it('compromete un recinto a mano en un asentamiento de nivel 3', () => {
    const { estado, mapa, faccionId } = estadoBase(3);
    const config = { faccionesIds: [faccionId] };
    const rng = createRng(SEED);

    // Llamada 1: sin Gobernador todavía, así que `asegurarGobernanzaBase` lo asigna en el mismo paso, y
    // `asegurarMuralla` corre después, en la misma llamada.
    let actual = avanzarNpcGobernanza(estado, mapa, contextoDeTest(0, rng), config).estado;
    for (let i = 1; i < 3 && (actual.asentamientos[0]?.recintos ?? []).length === 0; i++) {
      actual = avanzarNpcGobernanza(actual, mapa, contextoDeTest(i, rng), config).estado;
    }
    const asentamiento = actual.asentamientos[0]!;
    expect(asentamiento.cargos.gobernadorId).toBeTruthy();
    expect(asentamiento.recintos).toHaveLength(1);
    expect(asentamiento.recintos![0]!.avance).toBe(-1); // comprometer es gratis: nace sin nada levantado
  });

  it('no toca un asentamiento de nivel 2 (el gate de muralla exige nivel 3)', () => {
    const { estado, mapa, faccionId } = estadoBase(2);
    const config = { faccionesIds: [faccionId] };
    const rng = createRng(SEED);

    let actual = estado;
    for (let i = 0; i < 3; i++) {
      actual = avanzarNpcGobernanza(actual, mapa, contextoDeTest(i, rng), config).estado;
    }
    expect(actual.asentamientos[0]!.recintos ?? []).toHaveLength(0);
  });

  it('no compromete un segundo recinto si ya hay uno (ampliar es responsabilidad del Paso 3, no de este paso)', () => {
    const { estado, mapa, faccionId } = estadoBase(3);
    const config = { faccionesIds: [faccionId] };
    const rng = createRng(SEED);

    let actual = avanzarNpcGobernanza(estado, mapa, contextoDeTest(0, rng), config).estado;
    for (let i = 1; i < 3 && (actual.asentamientos[0]?.recintos ?? []).length === 0; i++) {
      actual = avanzarNpcGobernanza(actual, mapa, contextoDeTest(i, rng), config).estado;
    }
    expect(actual.asentamientos[0]!.recintos).toHaveLength(1);
    const primerRecinto = actual.asentamientos[0]!.recintos![0]!;

    for (let i = 3; i < 6; i++) {
      actual = avanzarNpcGobernanza(actual, mapa, contextoDeTest(i, rng), config).estado;
    }
    expect(actual.asentamientos[0]!.recintos).toHaveLength(1);
    expect(actual.asentamientos[0]!.recintos![0]!.id).toBe(primerRecinto.id);
  });
});
