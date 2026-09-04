// Cierra `issues/nivel_3_inalcanzable_sin_jugador_humano.md`: la gobernanza NPC ahora añade Barracón y
// Galería de tiro a mano (mismo patrón que ya usaba con Mercado), no solo Mercado — sin esto ningún
// asentamiento sin jugador humano podía reunir los 5 edificios que exige el gate de nivel 3
// (`NIVEL_ASENTAMIENTO.requisitos[3]`), porque la política de auto-construcción que antes los gateaba se
// retiró sin sustituto.
import { describe, expect, it } from 'vitest';
import { instanteDeTest } from '../../engine/__tests__/fixtures';
import type { Asentamiento } from '../../domain/types';
import type { EstadoSimulacion } from '../../engine/simulation';
import { avanzarNpcGobernanza } from '../npcGobernanza';
import { contextoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from '../../engine/__tests__/fixtures';
import { createRng } from '../../worldgen';
import { ZONA_INFLUENCIA } from '../../constants';

const SEED = 42;

/** Asentamiento de nivel 2, con almacén de sobra: lo único que puede hacer avanzar (o no) la construcción
 * militar es la lógica de gobernanza que se está probando, no falta de nivel ni de fondos. */
function estadoBase(): { estado: EstadoSimulacion; mapa: ReturnType<typeof crearMapaDeterminista>; faccionId: string } {
  const mapa = crearMapaDeterminista(SEED);
  const facciones = crearFacciones();
  const { asentamiento, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
  const abundante: Asentamiento = {
    ...asentamiento,
    nivel: 2,
    nivelActual: 2,
    // `radioPotencial` sube con construcción activa (`crecimientoPorEdificioCompletado`), no con el nivel a
    // secas — este fixture fuerza el nivel sin simular ninguna obra, así que hay que subirlo a mano al tope de
    // nivel 2 o el disco se queda en el mínimo de fundación (30 = 5 celdas). Con el Centro Urbano ya ocupando
    // buena parte de esas 5 celdas, la zona de seguridad entre anclas (2 celdas, `separacionSeguridadAnclas`)
    // no encuentra ningún hueco para el Mercado y la prueba deja de poder verificar lo que quiere verificar
    // (la lógica de gobernanza militar, no el espacio disponible).
    radioPotencial: ZONA_INFLUENCIA.radioMaximoPorNivel[2]!,
    almacen: Object.fromEntries(
      Object.entries(asentamiento.almacen).map(([recurso, item]) => [
        recurso,
        { ...item, cantidad: 2000, capacidad: 2000 },
      ])
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
      memoriaPorFaccion: {},
    },
    mapa,
    faccionId: 'faccion-1',
  };
}

describe('Gobernanza NPC — núcleo militar (Barracón / Galería de tiro)', () => {
  it('añade Barracón y Galería de tiro a mano en un asentamiento de nivel 2', () => {
    const { estado, mapa, faccionId } = estadoBase();
    const config = { faccionesIds: [faccionId] };
    const rng = createRng(SEED);

    // Llamada 1: sin Gobernador todavía, así que `asegurarGobernanzaBase` lo asigna en el mismo paso.
    // Dentro de esa misma llamada, `asegurarInfraestructuraComercial` (Mercado) y `asegurarNucleoMilitar`
    // (Barracón) corren en secuencia sobre el mismo asentamiento — con cola y fondos de sobra, los dos
    // entran en el mismo tick.
    const r1 = avanzarNpcGobernanza(estado, mapa, contextoDeTest(0, rng), config);
    const a1 = r1.estado.asentamientos[0]!;
    expect(a1.cargos.gobernadorId).toBeTruthy();
    expect(a1.edificios.some((e) => e.tipo === 'mercado')).toBe(true);
    expect(a1.edificios.some((e) => e.tipo === 'barracon')).toBe(true);
    expect(a1.edificios.some((e) => e.tipo === 'galeriaDeTiro')).toBe(false);

    // Llamada 2: Barracón ya está en curso (en cola), así que el núcleo militar pasa a Galería de tiro.
    const r3 = avanzarNpcGobernanza(r1.estado, mapa, contextoDeTest(1, rng), config);
    const a3 = r3.estado.asentamientos[0]!;
    expect(a3.edificios.some((e) => e.tipo === 'galeriaDeTiro')).toBe(true);

    // Los 5 tipos que exige el gate de nivel 3 ya tienen representación en la cola/activos de este
    // asentamiento — antes de este fix, Barracón y Galería de tiro se quedaban en cero para siempre en un
    // asentamiento sin jugador humano.
    for (const tipo of ['armeria', 'curtiduria', 'fundicion'] as const) {
      // Estos tres siguen siendo responsabilidad de la auto-construcción del motor (no de este NPC) — solo
      // se confirma aquí que el núcleo militar no los desplaza ni los duplica.
      expect(a3.edificios.filter((e) => e.tipo === tipo)).toHaveLength(0);
    }
    expect(a3.edificios.filter((e) => e.tipo === 'barracon')).toHaveLength(1);
    expect(a3.edificios.filter((e) => e.tipo === 'galeriaDeTiro')).toHaveLength(1);
  });

  it('no toca un asentamiento de nivel 1 (el gate de construcción exige nivel 2)', () => {
    const { estado, mapa, faccionId } = estadoBase();
    const nivel1: EstadoSimulacion = {
      ...estado,
      asentamientos: [{ ...estado.asentamientos[0]!, nivel: 1, nivelActual: 1 }],
    };
    const config = { faccionesIds: [faccionId] };
    const rng = createRng(SEED);

    let actual = nivel1;
    for (let i = 0; i < 3; i++) {
      actual = avanzarNpcGobernanza(actual, mapa, contextoDeTest(i, rng), config).estado;
    }
    const asentamiento = actual.asentamientos[0]!;
    expect(asentamiento.edificios.some((e) => e.tipo === 'barracon')).toBe(false);
    expect(asentamiento.edificios.some((e) => e.tipo === 'galeriaDeTiro')).toBe(false);
  });
});
