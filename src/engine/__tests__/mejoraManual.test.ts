// Mejora manual de un edificio individual (Doc 4.2, a petición del usuario): además de la mejora automática
// de `avanzarMejoras` que sigue corriendo cada tick, el jugador puede forzar la de un edificio concreto vía
// Gobernador/Maestro de Obras. Comparte gates y costo con la ruta automática (`elegibleParaMejora`), así que
// estos tests se centran en las validaciones propias de la acción MANUAL: cargo, existencia/estado del
// edificio, fondos y cuadrillas. Desde el 2026-09-26 la mejora TARDA: la acción manual la arranca y el tick la
// termina, con la misma mudanza que el camino automático.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Edificio } from '../../domain/types';
import { EDIFICIO_CATALOGO, MEJORA_EDIFICIO } from '../../constants';
import { minutos, sumar } from '../../domain/tiempo';
import {
  avanzarConstruccion,
  ConstruccionManualInvalidaError,
  estadoMejoraEdificio,
  mejorarEdificioManualmente,
  reclamosDeFuentes,
} from '../construction';
import { celdaMinimaDeEdificio, tamanoDeEdificio } from '../trazado';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';
import { TODAS_LAS_TECNOLOGIAS } from '../tecnologia';

const SEED = 7;
const AHORA = instanteDeTest(0);

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
    expect(() => mejorarEdificioManualmente(asentamiento, 'gobernador', granja.id, undefined, AHORA, TODAS_LAS_TECNOLOGIAS)).toThrow(
      ConstruccionManualInvalidaError
    );
  });

  it('rechaza un id de edificio que no existe en el asentamiento', () => {
    const asentamiento = conGobernador(base());
    expect(() => mejorarEdificioManualmente(asentamiento, 'gobernador', 'edificio-inexistente', undefined, AHORA, TODAS_LAS_TECNOLOGIAS)).toThrow(
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
      ambito: 'asentamiento',
    };
    const conProyecto = { ...asentamiento, edificios: [...asentamiento.edificios, enCola] };
    expect(() => mejorarEdificioManualmente(conProyecto, 'gobernador', enCola.id, undefined, AHORA, TODAS_LAS_TECNOLOGIAS)).toThrow(
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
    expect(() => mejorarEdificioManualmente(enMaximo, 'gobernador', granja.id, undefined, AHORA, TODAS_LAS_TECNOLOGIAS)).toThrow(
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
    expect(() => mejorarEdificioManualmente(conFundicion, 'gobernador', fundicion.id, undefined, AHORA, TODAS_LAS_TECNOLOGIAS)).toThrow(
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
    expect(() => mejorarEdificioManualmente(sinFondos, 'gobernador', granja.id, undefined, AHORA, TODAS_LAS_TECNOLOGIAS)).toThrow(
      ConstruccionManualInvalidaError
    );
  });
});

describe('mejorarEdificioManualmente — éxito', () => {
  function conFondos(): Asentamiento {
    const asentamiento = conGobernador(base());
    return {
      ...asentamiento,
      almacen: {
        ...asentamiento.almacen,
        madera: { cantidad: 1000, capacidad: 99999 },
        piedra: { cantidad: 1000, capacidad: 99999 },
      },
    };
  }
  const minutosGranja2 = EDIFICIO_CATALOGO.granja.tiempoConstruccionMinutos * MEJORA_EDIFICIO.multiplicadorPorNivel;

  it('arranca la mejora: paga el costoMejora exacto, fija cuándo termina y el nivel NO sube todavía', () => {
    const a = conFondos();
    const granja = granjaDe(a);
    const costoMejora = (EDIFICIO_CATALOGO.granja.niveles as Record<number, { costoMejora?: Record<string, number> }>)[2]!.costoMejora!;

    const resultado = mejorarEdificioManualmente(a, 'gobernador', granja.id, undefined, AHORA, TODAS_LAS_TECNOLOGIAS);

    const enMejora = resultado.edificios.find((e) => e.id === granja.id)!;
    expect(enMejora.nivelInterno ?? 1).toBe(1);
    expect(enMejora.estado).toBe('activo'); // sigue produciendo mientras se mejora
    expect(enMejora.mejora).toEqual({ nivelObjetivo: 2, completaEn: sumar(AHORA, minutos(minutosGranja2)) });
    expect(resultado.almacen.madera!.cantidad).toBe(1000 - costoMejora.madera!);
    expect(resultado.almacen.piedra!.cantidad).toBe(1000 - costoMejora.piedra!);
  });

  it('al pasar su tiempo la termina el tick: sube nivelInterno y muda la Granja, que crece de huella', () => {
    const mapa = crearMapaDeterminista(SEED);
    const a = conFondos();
    const granja = granjaDe(a);
    const enMejora = mejorarEdificioManualmente(a, 'gobernador', granja.id, undefined, AHORA, TODAS_LAS_TECNOLOGIAS);
    const fin = sumar(AHORA, minutos(minutosGranja2));

    const antes = avanzarConstruccion(enMejora, [], mapa, undefined, reclamosDeFuentes([enMejora]), sumar(fin, minutos(-1)), 0, TODAS_LAS_TECNOLOGIAS);
    expect(antes.asentamiento.edificios.find((e) => e.id === granja.id)!.nivelInterno ?? 1).toBe(1);

    const tras = avanzarConstruccion(enMejora, [], mapa, undefined, reclamosDeFuentes([enMejora]), fin, 0, TODAS_LAS_TECNOLOGIAS);
    const mejorada = tras.asentamiento.edificios.find((e) => e.id === granja.id)!;
    expect(mejorada.nivelInterno).toBe(2);
    // La del 2 ya no está en curso. Con fondos de sobra, la ruta automática puede haber arrancado ya la del 3 en
    // este mismo tick: es lo esperado (las mejoras se encadenan), no un resto de la anterior.
    expect(mejorada.mejora?.nivelObjetivo).not.toBe(2);
    // Se compara la HUELLA (celda mínima + tamaño), no `posicion`: con la rejilla doble (§E6.11) dos huellas de
    // alto par e impar pueden compartir centro, así que moverse no implica cambiar de `posicion`.
    expect({ ...celdaMinimaDeEdificio(mejorada), ...tamanoDeEdificio(mejorada) }).not.toEqual({
      ...celdaMinimaDeEdificio(granja),
      ...tamanoDeEdificio(granja),
    });
  });

  it('con las dos cuadrillas ocupadas, espera: se rechaza', () => {
    const a = conFondos();
    const granja = granjaDe(a);
    const ocupadas = {
      ...a,
      edificios: [
        ...a.edificios,
        { id: 'obra-1', tipo: 'vivienda' as const, posicion: { x: 90, y: 0 }, estado: 'en_construccion' as const, ambito: 'asentamiento' as const },
        { id: 'obra-2', tipo: 'vivienda' as const, posicion: { x: 95, y: 0 }, estado: 'en_construccion' as const, ambito: 'asentamiento' as const },
      ],
    };
    expect(() => mejorarEdificioManualmente(ocupadas, 'gobernador', granja.id, undefined, AHORA, TODAS_LAS_TECNOLOGIAS)).toThrow(ConstruccionManualInvalidaError);
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
    expect(estadoMejoraEdificio(enMaximo, { ...granja, nivelInterno: 4 }, undefined, TODAS_LAS_TECNOLOGIAS)).toBeNull();
  });

  it('reporta elegible:false con motivo cuando faltan fondos, y elegible:true cuando los hay', () => {
    const asentamiento = base();
    const granja = granjaDe(asentamiento);

    const sinFondos: Asentamiento = {
      ...asentamiento,
      almacen: { ...asentamiento.almacen, piedra: { ...asentamiento.almacen.piedra!, cantidad: 0 } },
    };
    const estadoSinFondos = estadoMejoraEdificio(sinFondos, granja, undefined, TODAS_LAS_TECNOLOGIAS)!;
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
    const estadoConFondos = estadoMejoraEdificio(conFondos, granja, undefined, TODAS_LAS_TECNOLOGIAS)!;
    expect(estadoConFondos.elegible).toBe(true);
    expect(estadoConFondos.nivelActual).toBe(1);
    expect(estadoConFondos.nivelSiguiente).toBe(2);
  });
});
