// `avanzarEjercitos` (Paso 4 del movimiento de ejércitos): un tick de campaña — comer, moverse, llegar.
//
// Se prueba la función directamente y no a través de `avanzarSimulacion` porque lo que hay que congelar son
// sus reglas, no el cableado; hay un test aparte al final que sí comprueba que está enchufada al tick.
//
// La regla más importante de todas, y la que más fácil sería perder en una refactorización: esta función NO
// consume aleatoriedad. Colocada al final de la cadena y muda de RNG, una partida sin ejércitos hace
// exactamente las mismas llamadas, en el mismo orden, que antes de que la mecánica existiera — y por eso el
// guardián de determinismo sigue verde sin tocarlo.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Ejercito, Escuadron } from '../../domain/types';
import { LOGISTICA, MILITAR } from '../../constants';
import { avanzarEjercitos, velocidadDeEjercito } from '../ejercitos';
import { crearEstadoDeTest, crearFacciones, crearMapaDeterminista, contextoDeTest, fundarAsentamientoDeTest } from './fixtures';
import { avanzarSimulacion } from '../simulation';
import { createRng, type RandomFn } from '../../worldgen';

const mapa = crearMapaDeterminista(42);

function base() {
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  return { asentamiento, facciones };
}

const escuadron = (id: string, tropaId: string, cantidad = 10, moral = 100): Escuadron => ({
  id,
  nombre: tropaId,
  jugadorId: 'jugador-1',
  origen: 'pesants',
  cantidad,
  veterania: 0,
  moral,
  tropaId,
});

/** Ejército sintético que sale de `origen` hacia un punto lejano, con lo que se le indique en el carro. */
function ejercitoDe(origen: Asentamiento, escuadrones: Escuadron[], trigo: number, estado: Ejercito['estado'] = 'marchando'): Ejercito {
  const destino = { x: origen.posicion.x + 600, y: origen.posicion.y };
  return {
    id: 'ejercito-1',
    faccionId: origen.faccionId,
    origenAsentamientoId: origen.id,
    escuadrones,
    suministro: { trigo },
    caravanasAdjuntasIds: [],
    objetivo: { tipo: 'punto', punto: destino },
    ruta: [origen.posicion, destino],
    progreso: 0,
    posicionActual: origen.posicion,
    estado,
  };
}

describe('velocidadDeEjercito — el ritmo lo marca el más lento (Doc 5.12.5)', () => {
  it('una fuerza ligera va a 20', () => {
    const e = ejercitoDe(base().asentamiento, [escuadron('a', 'milicia_lanceros'), escuadron('b', 'honderos')], 100);
    expect(velocidadDeEjercito(e)).toBe(20);
  });

  it('UN solo escuadrón pesado frena a toda la fuerza a 12', () => {
    const e = ejercitoDe(
      base().asentamiento,
      [escuadron('a', 'milicia_lanceros'), escuadron('b', 'honderos'), escuadron('c', 'lanceros_pesados')],
      100
    );
    // Y con eso deja de poder cazar una caravana comercial (16): incursión y asedio son composiciones
    // distintas, no la misma fuerza con otra orden.
    expect(velocidadDeEjercito(e)).toBe(12);
  });
});

describe('avanzarEjercitos — comer y moverse', () => {
  it('avanza por su ruta y come del CARRO, no del almacén', () => {
    const { asentamiento } = base();
    const trigoEnGranero = asentamiento.almacen['trigo']?.cantidad ?? 0;
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 100);

    const r = avanzarEjercitos([ejercito], [asentamiento], mapa);

    expect(r.ejercitos[0]!.progreso).toBeGreaterThan(0);
    expect(r.ejercitos[0]!.suministro['trigo']).toBeCloseTo(100 - 10 * MILITAR.racionPorSoldadoPorMinuto);
    // El almacén del asentamiento no se toca: en marcha se come del carro (Doc 5.13).
    expect(r.asentamientos[0]!.almacen['trigo']?.cantidad).toBe(trigoEnGranero);
  });

  it('estacionado no avanza, pero sigue comiendo — a consumo reducido (Doc 5.12.3)', () => {
    const { asentamiento } = base();
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 100, 'estacionado');

    const r = avanzarEjercitos([ejercito], [asentamiento], mapa);

    expect(r.ejercitos[0]!.progreso).toBe(0);
    expect(r.ejercitos[0]!.posicionActual).toEqual(asentamiento.posicion);
    const racionCompleta = 10 * MILITAR.racionPorSoldadoPorMinuto;
    expect(r.ejercitos[0]!.suministro['trigo']).toBeCloseTo(100 - racionCompleta * LOGISTICA.factorConsumoEstacionado);
    // Reducido, pero NUNCA cero: aparcar no es gratis.
    expect(r.ejercitos[0]!.suministro['trigo']).toBeLessThan(100);
  });

  it('sin trigo en el carro, la moral cae — la MISMA regla que en guarnición', () => {
    const { asentamiento } = base();
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 10, 50)], 0);

    const r = avanzarEjercitos([ejercito], [asentamiento], mapa);

    expect(r.ejercitos[0]!.escuadrones[0]!.moral).toBe(50 - MILITAR.degradacionMoralSinRacion);
  });
});

describe('avanzarEjercitos — el ejército fantasma (Doc 5.13.4)', () => {
  it('un ejército sin un solo soldado en pie SE DISUELVE, y sus identidades vuelven a casa', () => {
    const { asentamiento } = base();
    // Escuadrón aniquilado pero vivo como identidad: es lo que Doc 5.4 preserva para poder rellenarlo.
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 0);

    const r = avanzarEjercitos([ejercito], [asentamiento], mapa);

    expect(r.ejercitos).toHaveLength(0);
    const devuelto = r.asentamientos[0]!.escuadrones.find((e) => e.id === 'a');
    expect(devuelto, 'la identidad vacía tiene que volver al asentamiento, es donde se rellena').toBeDefined();
    expect(devuelto!.cantidad).toBe(0);
    expect(devuelto!.nombre).toBe('milicia_lanceros');
    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'ejercito.disuelto')).toBe(true);
  });

  it('NO se disuelve mientras quede alguien, aunque otro escuadrón esté a cero', () => {
    const { asentamiento } = base();
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0), escuadron('b', 'honderos', 5)], 100);

    const r = avanzarEjercitos([ejercito], [asentamiento], mapa);

    expect(r.ejercitos).toHaveLength(1);
  });

  it('si su asentamiento ya no existe, se disuelve igual y las identidades se pierden con él', () => {
    const { asentamiento } = base();
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 0);

    const r = avanzarEjercitos([ejercito], [], mapa);

    expect(r.ejercitos).toHaveLength(0);
    expect(r.asentamientos).toHaveLength(0);
  });
});

describe('avanzarEjercitos — llegar', () => {
  it('al llegar a su destino acampa (el asedio es el Paso 7)', () => {
    const { asentamiento } = base();
    const ejercito = { ...ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 100), progreso: 0.999 };

    const r = avanzarEjercitos([ejercito], [asentamiento], mapa);

    expect(r.ejercitos[0]!.estado).toBe('estacionado');
    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'ejercito.llega')).toBe(true);
  });

  it('regresando: al llegar a casa reintegra la tropa Y devuelve el sobrante del carro', () => {
    const { asentamiento } = base();
    const trigoAntes = asentamiento.almacen['trigo']?.cantidad ?? 0;
    const ejercito: Ejercito = {
      ...ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 60),
      estado: 'regresando',
      progreso: 0.999,
    };

    const r = avanzarEjercitos([ejercito], [asentamiento], mapa);

    expect(r.ejercitos).toHaveLength(0);
    expect(r.asentamientos[0]!.escuadrones.map((e) => e.id)).toContain('a');
    // El sobrante vuelve al almacén — descontado lo que se comió en este último tick.
    const comido = 10 * MILITAR.racionPorSoldadoPorMinuto;
    expect(r.asentamientos[0]!.almacen['trigo']!.cantidad).toBeCloseTo(trigoAntes + 60 - comido);
    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'ejercito.regresa')).toBe(true);
  });
});

describe('está enchufado al tick y NO consume aleatoriedad', () => {
  it('un ejército se mueve dentro de `avanzarSimulacion`', () => {
    const { asentamiento, facciones } = base();
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 500);
    const estado = crearEstadoDeTest([asentamiento], facciones, { ejercitos: [ejercito] });

    const tras = avanzarSimulacion(estado, mapa, contextoDeTest(1, createRng(42)));

    expect(tras.ejercitos[0]!.progreso).toBeGreaterThan(0);
  });

  it('con y sin ejércitos, el tick consume el MISMO número de valores del RNG', () => {
    // Es lo que permite que el guardián de determinismo siga verde sin tocarlo: añadir la mecánica no
    // desplaza el flujo de aleatoriedad de una partida que no la usa.
    const contar = (conEjercito: boolean) => {
      const { asentamiento, facciones } = base();
      const ejercitos = conEjercito ? [ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 500)] : [];
      const estado = crearEstadoDeTest([asentamiento], facciones, { ejercitos });
      let usos = 0;
      // Envuelve un rng real para no perder `estado()`, que forma parte del contrato de `RandomFn`.
      const real = createRng(42);
      const rngContado: RandomFn = Object.assign(
        () => {
          usos += 1;
          return real();
        },
        { estado: () => real.estado() }
      );
      avanzarSimulacion(estado, mapa, contextoDeTest(1, rngContado));
      return usos;
    };

    expect(contar(true)).toBe(contar(false));
  });
});

describe('atribucion de los eventos: un ejercito NO narra en global (Doc 5.12.7)', () => {
  // Por que esto es un test y no un detalle de formato: un `EventoDominio` sin `asentamientoId` es GLOBAL, y
  // `proyectarParaJugador` lo manda a TODOS los jugadores de la partida. Sin esta atribucion, redactar los
  // ejercitos ajenos en la proyeccion no serviria de nada: el log seguiria contandole a medio mundo que a un
  // rival se le desertan los hombres o que su columna acaba de llegar a destino.
  const origen = () => base().asentamiento;

  it('la desercion por hambre se atribuye al asentamiento de origen', () => {
    const a = origen();
    // Sin trigo en el carro y con la moral ya en el suelo: desertan en este mismo tick.
    const e = ejercitoDe(a, [escuadron('s1', 'milicia_lanceros', 10, 0)], 0);
    const eventos = avanzarEjercitos([e], [a], mapa).eventos;

    const desercion = eventos.filter((ev) => typeof ev !== 'string' && ev.codigo === 'tropas.desercion');
    expect(desercion.length).toBeGreaterThan(0);
    for (const ev of desercion) {
      expect(typeof ev !== 'string' && ev.asentamientoId).toBe(a.id);
    }
  });

  it('llegar, regresar y disolverse tambien se atribuyen', () => {
    const a = origen();
    // Ya en el ultimo tramo, para que llegue en este tick.
    const llega = { ...ejercitoDe(a, [escuadron('s1', 'milicia_lanceros')], 500), progreso: 0.999 };
    const disuelto = { ...ejercitoDe(a, [escuadron('s2', 'milicia_lanceros', 0)], 0), id: 'ejercito-2' };
    const vuelve = { ...ejercitoDe(a, [escuadron('s3', 'milicia_lanceros')], 500), id: 'ejercito-3', progreso: 0.999, estado: 'regresando' as const };

    const eventos = avanzarEjercitos([llega, disuelto, vuelve], [a], mapa).eventos;
    const codigos = eventos.filter((ev): ev is Exclude<typeof ev, string> => typeof ev !== 'string');

    expect(codigos.map((ev) => ev.codigo).sort()).toEqual(['ejercito.disuelto', 'ejercito.llega', 'ejercito.regresa']);
    for (const ev of codigos) expect(ev.asentamientoId).toBe(a.id);
  });
});
