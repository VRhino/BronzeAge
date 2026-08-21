// Mejora manual de un edificio individual (Doc 4.2, a petición del usuario): además de la mejora automática
// de `avanzarMejoras` que sigue corriendo cada tick, el jugador puede forzar la de un edificio concreto vía
// Gobernador/Maestro de Obras. Comparte gates y costo con la ruta automática (`elegibleParaMejora`), así que
// estos tests se centran en las validaciones propias de la acción MANUAL: cargo, existencia/estado del
// edificio, fondos y — en el caso de éxito — que pague y mude exactamente igual que el camino automático.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { EDIFICIO_CATALOGO } from '../../constants';
import { ConstruccionManualInvalidaError, estadoMejoraEdificio, mejorarEdificioManualmente } from '../construction';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, mockMathRandomDeterminista } from './fixtures';

const SEED = 7;

function base() {
  const mapa = crearMapaDeterminista(SEED);
  const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  return asentamiento;
}

function conGobernador(asentamiento: Asentamiento): Asentamiento {
  return { ...asentamiento, cargos: { ...asentamiento.cargos, gobernadorId: 'jugador-faccion-1-1' } };
}

function granjaDe(asentamiento: Asentamiento): Edificio {
  const granja = asentamiento.edificios.find((e) => e.tipo === 'granja' && e.estado === 'activo');
  if (!granja) throw new Error('Fixture inválido: el asentamiento fundado no trae Granja activa.');
  return granja;
}

describe('mejorarEdificioManualmente — validaciones', () => {
  it('rechaza sin Gobernador ni Maestro de Obras asignado', () => {
    const asentamiento = base();
    const granja = granjaDe(asentamiento);
    expect(() => mejorarEdificioManualmente(asentamiento, 'gobernador', granja.id, undefined)).toThrow(
      ConstruccionManualInvalidaError
    );
  });

  it('rechaza un id de edificio que no existe en el asentamiento', () => {
    const asentamiento = conGobernador(base());
    expect(() => mejorarEdificioManualmente(asentamiento, 'gobernador', 'edificio-inexistente', undefined)).toThrow(
      ConstruccionManualInvalidaError
    );
  });

  it('rechaza un edificio que todavía no está activo (en cola)', () => {
    const asentamiento = conGobernador(base());
    const enCola: Edificio = {
      id: `granja-encola-${asentamiento.id}`,
      tipo: 'granja',
      posicion: { x: 40, y: 0 },
      estado: 'en_cola',
      ticksRestantes: 0,
      ambito: 'asentamiento',
    };
    const conProyecto = { ...asentamiento, edificios: [...asentamiento.edificios, enCola] };
    expect(() => mejorarEdificioManualmente(conProyecto, 'gobernador', enCola.id, undefined)).toThrow(
      ConstruccionManualInvalidaError
    );
  });

  it('rechaza un edificio ya en su nivel interno máximo', () => {
    const asentamiento = conGobernador(base());
    const granja = granjaDe(asentamiento);
    const enMaximo = {
      ...asentamiento,
      edificios: asentamiento.edificios.map((e) => (e.id === granja.id ? { ...e, nivelInterno: 4 } : e)),
    };
    expect(() => mejorarEdificioManualmente(enMaximo, 'gobernador', granja.id, undefined)).toThrow(
      ConstruccionManualInvalidaError
    );
  });

  it('rechaza cuando no cumple el gate de nivel de asentamiento del siguiente nivel', () => {
    const asentamiento = conGobernador(base());
    // Fundición nivel 2 exige `requisitoNivelAsentamiento: 2` — se inyecta directa ya activa (mismo patrón que
    // `gate_militar_nivel2.test.ts`/`mercado_zona.test.ts`) para probar solo el gate de MEJORA, no el de
    // construcción base.
    const fundicion: Edificio = {
      id: `fundicion-${asentamiento.id}`,
      tipo: 'fundicion',
      posicion: { x: 40, y: 0 },
      estado: 'activo',
      ticksRestantes: 0,
      ambito: 'asentamiento',
      nivelInterno: 1,
    };
    const conFundicion: Asentamiento = {
      ...asentamiento,
      nivel: 1,
      nivelActual: 1,
      edificios: [...asentamiento.edificios, fundicion],
      almacen: {
        ...asentamiento.almacen,
        madera: { ...asentamiento.almacen.madera!, cantidad: 500 },
        piedra: { ...asentamiento.almacen.piedra!, cantidad: 500 },
      },
    };
    expect(() => mejorarEdificioManualmente(conFundicion, 'gobernador', fundicion.id, undefined)).toThrow(
      ConstruccionManualInvalidaError
    );
  });

  it('rechaza sin fondos suficientes para el costoMejora', () => {
    const asentamiento = conGobernador(base());
    const granja = granjaDe(asentamiento);
    const sinFondos: Asentamiento = {
      ...asentamiento,
      almacen: {
        ...asentamiento.almacen,
        madera: { ...asentamiento.almacen.madera!, cantidad: 0 },
        piedra: { ...asentamiento.almacen.piedra!, cantidad: 0 },
      },
    };
    expect(() => mejorarEdificioManualmente(sinFondos, 'gobernador', granja.id, undefined)).toThrow(
      ConstruccionManualInvalidaError
    );
  });
});

describe('mejorarEdificioManualmente — éxito', () => {
  let restaurar: () => void;

  beforeEach(() => {
    restaurar = mockMathRandomDeterminista(SEED);
  });

  afterEach(() => {
    restaurar();
  });

  it('paga el costoMejora exacto, sube nivelInterno y reubica si cambia de tamaño (Granja 1 -> 2)', () => {
    const asentamiento = conGobernador(base());
    const conFondos: Asentamiento = {
      ...asentamiento,
      almacen: {
        ...asentamiento.almacen,
        madera: { cantidad: 1000, capacidad: 99999 },
        piedra: { cantidad: 1000, capacidad: 99999 },
      },
    };
    const granja = granjaDe(conFondos);
    const costoMejora = (EDIFICIO_CATALOGO.granja.niveles as Record<number, { costoMejora?: Record<string, number> }>)[2]!
      .costoMejora!;

    const resultado = mejorarEdificioManualmente(conFondos, 'gobernador', granja.id, undefined);

    const granjaMejorada = resultado.edificios.find((e) => e.id === granja.id)!;
    expect(granjaMejorada.nivelInterno).toBe(2);
    expect(resultado.almacen.madera!.cantidad).toBe(1000 - costoMejora.madera!);
    expect(resultado.almacen.piedra!.cantidad).toBe(1000 - costoMejora.piedra!);
    // Nivel 2 mide 2x3 frente a 2x2 en nivel 1 (EDIFICIO_CATALOGO.granja.niveles) — obliga a mudarla.
    expect(granjaMejorada.posicion).not.toEqual(granja.posicion);
  });
});

describe('estadoMejoraEdificio — selector de solo lectura (usado por gameStore/UI)', () => {
  it('devuelve null cuando el edificio ya está en su nivel máximo', () => {
    const asentamiento = base();
    const granja = granjaDe(asentamiento);
    const enMaximo = {
      ...asentamiento,
      edificios: asentamiento.edificios.map((e) => (e.id === granja.id ? { ...e, nivelInterno: 4 } : e)),
    };
    expect(estadoMejoraEdificio(enMaximo, { ...granja, nivelInterno: 4 }, undefined)).toBeNull();
  });

  it('reporta elegible:false con motivo cuando faltan fondos, y elegible:true cuando los hay', () => {
    const asentamiento = base();
    const granja = granjaDe(asentamiento);

    const sinFondos: Asentamiento = {
      ...asentamiento,
      almacen: { ...asentamiento.almacen, piedra: { ...asentamiento.almacen.piedra!, cantidad: 0 } },
    };
    const estadoSinFondos = estadoMejoraEdificio(sinFondos, granja, undefined)!;
    expect(estadoSinFondos.elegible).toBe(false);
    expect(estadoSinFondos.motivoBloqueo).toBeTruthy();

    const conFondos: Asentamiento = {
      ...asentamiento,
      almacen: {
        ...asentamiento.almacen,
        madera: { cantidad: 1000, capacidad: 99999 },
        piedra: { cantidad: 1000, capacidad: 99999 },
      },
    };
    const estadoConFondos = estadoMejoraEdificio(conFondos, granja, undefined)!;
    expect(estadoConFondos.elegible).toBe(true);
    expect(estadoConFondos.nivelActual).toBe(1);
    expect(estadoConFondos.nivelSiguiente).toBe(2);
  });
});
