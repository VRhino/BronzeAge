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
import type { AcuerdoTrueque, Asentamiento, Caravana, Ejercito, Escuadron, Faccion, RelacionPolitica } from '../../domain/types';
import { CARAVANA_CATALOGO, LOGISTICA, MILITAR } from '../../constants';
import { entregarDesdeCaravanaAdjunta } from '../trade';
import {
  adjuntarCaravana,
  cargarCaravanaAdjunta,
  ladoPendienteParaEjercito,
  avanzarEjercitos,
  capacidadCargaDe,
  MovilizacionInvalidaError,
  soltarCaravana,
  velocidadDeEjercito,
} from '../ejercitos';
import { esResidente, resideEnOtroAsentamiento } from '../pertenencia';
import { reservaDeTrigo } from '../tropas';
import { crearEstadoDeTest, crearFacciones, crearMapaDeterminista, contextoDeTest, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';
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

/**
 * Aleja un ejército de cualquier plaza amiga. Desde el Paso 8, un ejército a menos de
 * `LOGISTICA.radioReabastecimiento` de una ciudad suya REPONE cada tick, así que medir el consumo del carro
 * junto a su propio asentamiento mide otra cosa: el saldo neto de comer y repostar a la vez.
 */
function enCampoAbierto(ejercito: Ejercito, origen: Asentamiento): Ejercito {
  const lejos = { x: origen.posicion.x, y: origen.posicion.y + LOGISTICA.radioReabastecimiento * 4 };
  return { ...ejercito, posicionActual: lejos, ruta: [lejos, { x: lejos.x, y: lejos.y + 400 }] };
}

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

/**
 * `avanzarEjercitos` con el contexto que el Paso 7 le anadio (Facciones, relaciones, instante y RNG del
 * asedio). Los casos de este archivo que no asedian nada no lo usan para nada, y llamarlos con siete
 * argumentos cada vez solo tapaba lo que cada test mide.
 */
function avanzar(
  ejercitos: Ejercito[],
  asentamientos: Asentamiento[],
  opciones: { facciones?: Faccion[]; relaciones?: RelacionPolitica[]; rng?: RandomFn; caravanas?: Caravana[] } = {}
) {
  return avanzarEjercitos(ejercitos, {
    asentamientos,
    caravanas: opciones.caravanas ?? [],
    facciones: opciones.facciones ?? crearFacciones(),
    relaciones: opciones.relaciones ?? [],
    mapa,
    instante: instanteDeTest(1),
    rng: opciones.rng ?? createRng(1),
  });
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
    const ejercito = enCampoAbierto(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 100), asentamiento);

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos[0]!.progreso).toBeGreaterThan(0);
    expect(r.ejercitos[0]!.suministro['trigo']).toBeCloseTo(100 - 10 * MILITAR.racionPorSoldadoPorMinuto);
    // El almacén del asentamiento no se toca: en marcha se come del carro (Doc 5.13).
    expect(r.asentamientos[0]!.almacen['trigo']?.cantidad).toBe(trigoEnGranero);
  });

  it('estacionado no avanza, pero sigue comiendo — a consumo reducido (Doc 5.12.3)', () => {
    const { asentamiento } = base();
    const ejercito = enCampoAbierto(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 100, 'estacionado'), asentamiento);

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos[0]!.progreso).toBe(0);
    expect(r.ejercitos[0]!.posicionActual).toEqual(ejercito.posicionActual);
    const racionCompleta = 10 * MILITAR.racionPorSoldadoPorMinuto;
    expect(r.ejercitos[0]!.suministro['trigo']).toBeCloseTo(100 - racionCompleta * LOGISTICA.factorConsumoEstacionado);
    // Reducido, pero NUNCA cero: aparcar no es gratis.
    expect(r.ejercitos[0]!.suministro['trigo']).toBeLessThan(100);
  });

  it('sin trigo en el carro, la moral cae — la MISMA regla que en guarnición', () => {
    const { asentamiento } = base();
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 10, 50)], 0);

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos[0]!.escuadrones[0]!.moral).toBe(50 - MILITAR.degradacionMoralSinRacion);
  });
});

describe('avanzarEjercitos — el ejército fantasma (Doc 5.13.4)', () => {
  it('un ejército sin un solo soldado en pie SE DISUELVE, y sus identidades vuelven a casa', () => {
    const { asentamiento } = base();
    // Escuadrón aniquilado pero vivo como identidad: es lo que Doc 5.4 preserva para poder rellenarlo.
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 0);

    const r = avanzar([ejercito], [asentamiento]);

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

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos).toHaveLength(1);
  });

  it('si su asentamiento ya no existe, se disuelve igual y las identidades se pierden con él', () => {
    const { asentamiento } = base();
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 0);

    const r = avanzar([ejercito], []);

    expect(r.ejercitos).toHaveLength(0);
    expect(r.asentamientos).toHaveLength(0);
  });
});

describe('avanzarEjercitos — llegar', () => {
  it('al llegar a su destino acampa (el asedio es el Paso 7)', () => {
    const { asentamiento } = base();
    const ejercito = { ...ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 100), progreso: 0.999 };

    const r = avanzar([ejercito], [asentamiento]);

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

    const r = avanzar([ejercito], [asentamiento]);

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
    const eventos = avanzar([e], [a]).eventos;

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

    const eventos = avanzar([llega, disuelto, vuelve], [a]).eventos;
    const codigos = eventos.filter((ev): ev is Exclude<typeof ev, string> => typeof ev !== 'string');

    expect(codigos.map((ev) => ev.codigo).sort()).toEqual(['ejercito.disuelto', 'ejercito.llega', 'ejercito.regresa']);
    for (const ev of codigos) expect(ev.asentamientoId).toBe(a.id);
  });
});

// ---------------------------------------------------------------------------------------------------------
// Paso 7 — llegar a una plaza ajena ES el asedio (Doc 5.12.4).
// ---------------------------------------------------------------------------------------------------------

/** Dos asentamientos de Facciones distintas, y un ejército del primero a un paso de plantarse en el segundo. */
function frenteDeGuerra(defensores: Escuadron[], atacantes = [escuadron('a1', 'milicia_lanceros', 50)]) {
  const facciones = crearFacciones();
  const propio = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
  const enemigoBase = fundarAsentamientoDeTest(mapa, propio.facciones, 'faccion-2', [propio.asentamiento]);
  const enemigo: Asentamiento = { ...enemigoBase.asentamiento, escuadrones: defensores };

  const ejercito: Ejercito = {
    ...ejercitoDe(propio.asentamiento, atacantes, 5000),
    objetivo: { tipo: 'asentamiento', id: enemigo.id },
    ruta: [propio.asentamiento.posicion, enemigo.posicion],
    progreso: 0.999, // llega en este mismo tick
  };
  return { facciones: enemigoBase.facciones, propio: propio.asentamiento, enemigo, ejercito };
}

describe('llegada a un asentamiento ajeno = asedio (Paso 7)', () => {
  it('una plaza SIN defensores cae sin combate, y sin consumir aleatoriedad (Doc 5.12.4)', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([]);
    // Envuelve un RNG real para CONTAR llamadas sin perder su marca de tipo ni su `estado`.
    const base = createRng(1);
    let tiradas = 0;
    const rng: RandomFn = Object.assign(
      () => {
        tiradas++;
        return base();
      },
      { estado: base.estado }
    );

    const r = avanzar([ejercito], [propio, enemigo], { facciones, rng });

    const despues = r.asentamientos.find((a) => a.id === enemigo.id)!;
    expect(despues.faccionId).toBe('faccion-1');
    expect(tiradas, 'conquistar una plaza desguarnecida no debe tocar el RNG').toBe(0);
    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'combate.asedio_conquista')).toBe(true);
  });

  it('la guarnición del conquistado queda a CERO pero conserva dueño y veteranía (Doc 5.4)', () => {
    // Veteranía y dueño distintos de los de la fixture para que la aserción diga algo: lo que se pierde son
    // los hombres, no el escuadrón ni su progreso.
    const veterano: Escuadron = { ...escuadron('d1', 'milicia_lanceros', 1), jugadorId: 'rival-a', veterania: 3 };
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([veterano]);

    const r = avanzar([ejercito], [propio, enemigo], { facciones });

    const despues = r.asentamientos.find((a) => a.id === enemigo.id)!;
    expect(despues.faccionId, 'con 50 atacantes contra 1 defensor la plaza cae').toBe('faccion-1');
    // El escuadrón SOBREVIVE como cascarón: sin hombres, pero con su dueño y su veteranía (Doc 5.4 — "el
    // SQUAD persiste aunque el regimiento sea aniquilado"). Lo que no hay es botín: cero unidades para nadie.
    expect(despues.escuadrones.map((e) => e.cantidad)).toEqual([0]);
    expect(despues.escuadrones[0]!.jugadorId, 'sigue siendo de su dueño, no del conquistador').toBe('rival-a');
    expect(despues.escuadrones[0]!.veterania, 'el progreso del escuadrón no se pierde').toBe(3);
  });

  it('conquistar deja HUÉRFANOS a los residentes: pierden residencia y cargos (Doc 5.4)', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([]);
    expect(enemigo.jugadoresFundadoresIds.length, 'la plaza arranca con residentes').toBeGreaterThan(0);

    const r = avanzar([ejercito], [propio, enemigo], { facciones });
    const despues = r.asentamientos.find((a) => a.id === enemigo.id)!;

    expect(despues.jugadoresFundadoresIds).toEqual([]);
    expect(despues.casasCompradas).toEqual([]);
    expect(Object.values(despues.cargos).every((v) => v === null)).toBe(true);
    // Lo que NO se toca: la ciudad se entrega entera y en funcionamiento (Doc 5.12.4).
    expect(despues.edificios.length).toBe(enemigo.edificios.length);
    expect(despues.poblacion).toEqual(enemigo.poblacion);
  });

  it('el que asedia conserva su columna: no entra en la ciudad, acampa fuera', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([]);

    const r = avanzar([ejercito], [propio, enemigo], { facciones });

    expect(r.ejercitos).toHaveLength(1);
    expect(r.ejercitos[0]!.estado).toBe('estacionado');
    expect(r.ejercitos[0]!.escuadrones.map((e) => e.id)).toEqual(['a1']);
    expect(r.asentamientos.find((a) => a.id === enemigo.id)!.escuadrones).toEqual([]);
  });

  it('el asedio se resuelve UNA vez: acampado junto a la plaza ya no vuelve a atacar', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([escuadron('d1', 'milicia_lanceros', 200)]);

    const primero = avanzar([ejercito], [propio, enemigo], { facciones });
    const defensorTrasPrimero = primero.asentamientos.find((a) => a.id === enemigo.id)!;
    expect(primero.ejercitos[0]!.estado, 'resistió, y el atacante queda acampado').toBe('estacionado');

    const segundo = avanzar(primero.ejercitos, primero.asentamientos, { facciones: primero.facciones });
    const defensorTrasSegundo = segundo.asentamientos.find((a) => a.id === enemigo.id)!;

    expect(defensorTrasSegundo.escuadrones.map((e) => e.cantidad)).toEqual(defensorTrasPrimero.escuadrones.map((e) => e.cantidad));
    expect(segundo.eventos.some((e) => typeof e !== 'string' && e.codigo.startsWith('combate.'))).toBe(false);
  });

  it('llegar a un asentamiento PROPIO no dispara nada: solo acampa', () => {
    // El segundo se re-etiqueta en vez de fundarse: el cap de fundación en nivel 1 es UNO por Facción
    // (`CAP_FUNDACION_POR_NIVEL`), y lo que aquí importa es el destino, no cómo llegó a ser propio.
    const { facciones, propio, enemigo } = frenteDeGuerra([escuadron('d1', 'milicia_lanceros', 200)]);
    const otroPropio: Asentamiento = { ...enemigo, faccionId: 'faccion-1' };
    const e: Ejercito = {
      ...ejercitoDe(propio, [escuadron('a1', 'milicia_lanceros', 50)], 5000),
      objetivo: { tipo: 'asentamiento', id: otroPropio.id },
      ruta: [propio.posicion, otroPropio.posicion],
      progreso: 0.999,
    };

    const r = avanzar([e], [propio, otroPropio], { facciones });

    expect(r.asentamientos.find((a) => a.id === otroPropio.id)!.faccionId).toBe('faccion-1');
    expect(r.asentamientos.find((a) => a.id === otroPropio.id)!.escuadrones, 'ni un rasguño a la guarnición amiga').toHaveLength(1);
    expect(r.eventos.some((ev) => typeof ev !== 'string' && ev.codigo === 'ejercito.llega')).toBe(true);
    expect(r.eventos.some((ev) => typeof ev !== 'string' && ev.codigo.startsWith('combate.'))).toBe(false);
  });

  it('un asedio RESISTIDO se narra a los dos lados: al hogar del atacante y a la plaza que aguantó', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([escuadron('d1', 'milicia_lanceros', 200)]);

    const r = avanzar([ejercito], [propio, enemigo], { facciones });
    const resistido = r.eventos.filter((e) => typeof e !== 'string' && e.codigo === 'combate.asedio_resistido');

    expect(resistido).toHaveLength(2);
    expect(resistido.map((e) => (typeof e !== 'string' ? e.asentamientoId : undefined)).sort()).toEqual(
      [propio.id, enemigo.id].sort()
    );
  });

  it('una CONQUISTA se narra una sola vez: la plaza ya es del atacante y se la contaría dos veces', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([]);

    const r = avanzar([ejercito], [propio, enemigo], { facciones });
    const conquista = r.eventos.filter((e) => typeof e !== 'string' && e.codigo === 'combate.asedio_conquista');

    expect(conquista).toHaveLength(1);
    expect(typeof conquista[0] !== 'string' && conquista[0]!.asentamientoId).toBe(propio.id);
  });

  it('el jugador que estaba de campaña queda HUÉRFANO: sin residencia, pero con su tropa (Doc 5.4)', () => {
    // El caso completo de la regla: mientras la columna de faccion-1 asedia, la SUYA cae. No hace falta
    // guardar "huérfano" en ninguna parte — es no residir en ningún asentamiento, y sale solo de que la
    // conquista vacíe las listas de residencia.
    const { facciones, propio, enemigo } = frenteDeGuerra([]);
    const suDueno = propio.jugadoresFundadoresIds[0]!;
    const enCampana: Ejercito = {
      ...ejercitoDe(propio, [{ ...escuadron('a1', 'milicia_lanceros', 40), jugadorId: suDueno }], 5000),
      id: 'ejercito-suyo',
      objetivo: { tipo: 'asentamiento', id: enemigo.id },
      ruta: [propio.posicion, enemigo.posicion],
      progreso: 0.5, // sigue de marcha: no llega ni conquista nada este tick
    };
    // Y a la vez, una columna enemiga se planta en SU ciudad, que quedó desguarnecida.
    const invasor: Ejercito = {
      ...ejercitoDe(enemigo, [escuadron('inv', 'milicia_lanceros', 40)], 5000),
      id: 'ejercito-invasor',
      faccionId: 'faccion-2',
      origenAsentamientoId: enemigo.id,
      objetivo: { tipo: 'asentamiento', id: propio.id },
      ruta: [enemigo.posicion, propio.posicion],
      progreso: 0.999,
    };

    const r = avanzar([enCampana, invasor], [propio, enemigo], { facciones });

    const casa = r.asentamientos.find((a) => a.id === propio.id)!;
    expect(casa.faccionId, 'su ciudad cambió de dueño').toBe('faccion-2');
    expect(esResidente(casa, suDueno), 'ya no reside ahí').toBe(false);
    expect(resideEnOtroAsentamiento(r.asentamientos, 'ninguno', suDueno), 'ni en ningún otro sitio').toBe(false);

    const suya = r.ejercitos.find((e) => e.id === 'ejercito-suyo')!;
    expect(suya.escuadrones[0]!.cantidad, 'conserva intacto lo que llevaba encima').toBe(40);
  });

  it('los ejércitos se recorren en orden canónico por id, no en el del array', () => {
    // Dos ejércitos que asedian consumen RNG; si el orden dependiera del array, la misma partida con los
    // ejércitos guardados al revés divergiría.
    const { facciones, propio, enemigo } = frenteDeGuerra([escuadron('d1', 'milicia_lanceros', 30)]);
    const uno: Ejercito = {
      ...ejercitoDe(propio, [escuadron('a1', 'milicia_lanceros', 20)], 5000),
      id: 'ejercito-aaa',
      objetivo: { tipo: 'asentamiento', id: enemigo.id },
      ruta: [propio.posicion, enemigo.posicion],
      progreso: 0.999,
    };
    const dos: Ejercito = { ...uno, id: 'ejercito-bbb', escuadrones: [escuadron('a2', 'honderos', 20)] };

    const enOrden = avanzar([uno, dos], [propio, enemigo], { facciones, rng: createRng(7) });
    const alReves = avanzar([dos, uno], [propio, enemigo], { facciones, rng: createRng(7) });

    const resumen = (r: ReturnType<typeof avanzar>) =>
      JSON.stringify({
        ejercitos: [...r.ejercitos].sort((a, b) => (a.id < b.id ? -1 : 1)).map((e) => e.escuadrones.map((x) => x.cantidad)),
        defensor: r.asentamientos.find((a) => a.id === enemigo.id)!.escuadrones.map((x) => x.cantidad),
      });
    expect(resumen(enOrden)).toBe(resumen(alReves));
  });
});

// ---------------------------------------------------------------------------------------------------------
// Paso 8 — repostar al pasar (Doc 5.13).
// ---------------------------------------------------------------------------------------------------------

describe('reabastecimiento en ruta', () => {
  /** Un ejército a medio carro, plantado justo encima de `plaza`. */
  function juntoA(plaza: Asentamiento, origen: Asentamiento, faccionId = origen.faccionId): Ejercito {
    const e = ejercitoDe(origen, [escuadron('a', 'milicia_lanceros', 10)], 100, 'estacionado');
    return { ...e, faccionId, posicionActual: plaza.posicion };
  }

  it('en una plaza PROPIA repone siempre, hasta llenar el carro, y sale del almacén de ella', () => {
    const { asentamiento } = base();
    const trigoAntes = asentamiento.almacen['trigo']?.cantidad ?? 0;
    const ejercito = juntoA(asentamiento, asentamiento);

    const r = avanzar([ejercito], [asentamiento]);

    const carro = r.ejercitos[0]!.suministro['trigo']!;
    expect(carro, 'el carro sube por encima de lo que tenía').toBeGreaterThan(100);
    expect(carro).toBeLessThanOrEqual(LOGISTICA.capacidadCarroPorJugador);
    // Y el trigo sale de la plaza: repostar cuesta stock real a quien lo da.
    expect(r.asentamientos[0]!.almacen['trigo']!.cantidad).toBeLessThan(trigoAntes);
  });

  it('nunca deja a la plaza por debajo de su reserva de comida', () => {
    const { asentamiento } = base();
    const reserva = reservaDeTrigo(asentamiento);
    const apurado: Asentamiento = {
      ...asentamiento,
      almacen: { ...asentamiento.almacen, trigo: { ...asentamiento.almacen['trigo']!, cantidad: reserva + 20 } },
    };

    const r = avanzar([juntoA(apurado, apurado)], [apurado]);

    const plaza = r.asentamientos[0]!;
    expect(plaza.almacen['trigo']!.cantidad).toBeGreaterThanOrEqual(reservaDeTrigo(plaza) - 1e-9);
  });

  it('en una plaza AJENA sin alianza no repone nada', () => {
    const facciones = crearFacciones();
    const propio = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const ajenoBase = fundarAsentamientoDeTest(mapa, propio.facciones, 'faccion-2', [propio.asentamiento]);
    const ajeno = ajenoBase.asentamiento;
    const trigoAntes = ajeno.almacen['trigo']?.cantidad ?? 0;

    const r = avanzar([juntoA(ajeno, propio.asentamiento)], [ajeno], { facciones: ajenoBase.facciones });

    expect(r.ejercitos[0]!.suministro['trigo']).toBeLessThanOrEqual(100);
    expect(r.asentamientos[0]!.almacen['trigo']!.cantidad).toBe(trigoAntes);
  });

  it('en una plaza ALIADA repone solo si ella lo permite', () => {
    const facciones = crearFacciones();
    const propio = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const aliadaBase = fundarAsentamientoDeTest(mapa, propio.facciones, 'faccion-2', [propio.asentamiento]);
    const alianza: RelacionPolitica[] = [
      { id: 'r1', faccionAId: 'faccion-1', faccionBId: 'faccion-2', tipo: 'alianza', estado: 'activa', creadoEn: instanteDeTest(0) },
    ];

    const cerrada = aliadaBase.asentamiento;
    const sinPermiso = avanzar([juntoA(cerrada, propio.asentamiento)], [cerrada], { facciones, relaciones: alianza });
    expect(sinPermiso.ejercitos[0]!.suministro['trigo'], 'sin la opción activa, la puerta está cerrada').toBeLessThanOrEqual(100);

    const abierta: Asentamiento = { ...cerrada, permiteReabastecerAliados: true };
    const conPermiso = avanzar([juntoA(abierta, propio.asentamiento)], [abierta], { facciones, relaciones: alianza });
    expect(conPermiso.ejercitos[0]!.suministro['trigo']).toBeGreaterThan(100);
  });

  it('la alianza NO basta por sí sola: sin la opción no hay reposte aunque sean aliadas', () => {
    // Complementa al anterior desde el otro lado: aquí la plaza tiene la opción ABIERTA pero no hay alianza.
    const facciones = crearFacciones();
    const propio = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const otraBase = fundarAsentamientoDeTest(mapa, propio.facciones, 'faccion-2', [propio.asentamiento]);
    const abiertaSinAlianza: Asentamiento = { ...otraBase.asentamiento, permiteReabastecerAliados: true };

    const r = avanzar([juntoA(abiertaSinAlianza, propio.asentamiento)], [abiertaSinAlianza], { facciones });

    expect(r.ejercitos[0]!.suministro['trigo']).toBeLessThanOrEqual(100);
  });

  it('demasiado lejos de la plaza, no repone', () => {
    const { asentamiento } = base();
    const lejos = enCampoAbierto(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 100, 'estacionado'), asentamiento);
    const trigoAntes = asentamiento.almacen['trigo']?.cantidad ?? 0;

    const r = avanzar([lejos], [asentamiento]);

    expect(r.ejercitos[0]!.suministro['trigo']).toBeLessThan(100); // comió y no repuso
    expect(r.asentamientos[0]!.almacen['trigo']!.cantidad).toBe(trigoAntes);
  });

  it('acampado junto a una plaza amiga se sostiene: 50 ticks y el carro va a MÁS, no a menos', () => {
    // La razón de ser del paso (Doc 5.12.3). Se compara contra el mismo ejército en campo abierto, que es la
    // única forma de decir que lo que sostiene la posición es el reposte y no que estacionado coma poco.
    const { asentamiento } = base();
    const correr = (inicial: Ejercito) => {
      let ejercitos = [inicial];
      let asentamientos = [asentamiento];
      for (let i = 0; i < 50; i++) {
        const r = avanzar(ejercitos, asentamientos);
        ejercitos = r.ejercitos;
        asentamientos = r.asentamientos;
      }
      return ejercitos;
    };

    const acampado = correr(juntoA(asentamiento, asentamiento));
    const enRuta = correr(enCampoAbierto(juntoA(asentamiento, asentamiento), asentamiento));

    expect(acampado, 'no se disolvió por hambre').toHaveLength(1);
    expect(acampado[0]!.escuadrones[0]!.cantidad, 'ni un desertor').toBe(10);
    expect(acampado[0]!.suministro['trigo'], 'el carro acaba con MÁS trigo del que empezó').toBeGreaterThan(100);
    expect(
      acampado[0]!.suministro['trigo']!,
      'y con más que el mismo ejército sin plaza al lado'
    ).toBeGreaterThan(enRuta[0]!.suministro['trigo']!);
  });
});

// ---------------------------------------------------------------------------------------------------------
// Paso 9a — el tren de suministros: caravanas adjuntas (Doc 5.13.2).
// ---------------------------------------------------------------------------------------------------------

const caravanaDe = (id: string, origenId: string, posicion: { x: number; y: number }): Caravana => ({
  id,
  tipo: 'comercial',
  origenAsentamientoId: origenId,
  contenido: {},
  posicionActual: posicion,
  progreso: 0,
  estado: 'disponible',
});

describe('caravanas adjuntas', () => {
  it('la capacidad de una caravana NO es menor que el carro de un Jugador (Doc 5.13.2)', () => {
    // El invariante que justifica el rebalance del Paso 9: si cargara menos, engancharla no tendría sentido.
    expect(CARAVANA_CATALOGO.comercial.capacidad).toBeGreaterThanOrEqual(LOGISTICA.capacidadCarroPorJugador);
  });

  it('suman su capacidad a la del carro', () => {
    const { asentamiento } = base();
    const e = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 0);
    const c = caravanaDe('c1', asentamiento.id, asentamiento.posicion);

    expect(capacidadCargaDe(e, [])).toBe(LOGISTICA.capacidadCarroPorJugador);
    const conCaravana = adjuntarCaravana(e, c, asentamiento).ejercito;
    expect(capacidadCargaDe(conCaravana, [c])).toBe(
      LOGISTICA.capacidadCarroPorJugador + CARAVANA_CATALOGO.comercial.capacidad
    );
  });

  it('entran en el MÍNIMO de velocidad: una comercial frena a una fuerza ligera de 20 a 16', () => {
    const { asentamiento } = base();
    const ligero = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 0);
    expect(velocidadDeEjercito(ligero)).toBe(20);

    const c = caravanaDe('c1', asentamiento.id, asentamiento.posicion);
    const conCaravana = adjuntarCaravana(ligero, c, asentamiento).ejercito;
    expect(velocidadDeEjercito(conCaravana, [c])).toBe(CARAVANA_CATALOGO.comercial.velocidad);
    expect(velocidadDeEjercito(conCaravana, [c]), 'y eso le quita la capacidad de cazar una comercial').toBeLessThan(20);
  });

  it('rechaza enganchar una caravana ajena, ya despachada, lejos, o dos veces', () => {
    const { asentamiento } = base();
    const e = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 0);
    const c = caravanaDe('c1', asentamiento.id, asentamiento.posicion);
    const ajeno: Asentamiento = { ...asentamiento, faccionId: 'faccion-2' };

    expect(() => adjuntarCaravana(e, c, ajeno)).toThrow(MovilizacionInvalidaError);
    expect(() => adjuntarCaravana(e, c, undefined)).toThrow(MovilizacionInvalidaError);
    expect(() => adjuntarCaravana(e, { ...c, estado: 'en_transito' }, asentamiento)).toThrow(MovilizacionInvalidaError);
    const lejos = { x: asentamiento.posicion.x + LOGISTICA.radioReabastecimiento * 3, y: asentamiento.posicion.y };
    expect(() => adjuntarCaravana(e, { ...c, posicionActual: lejos }, asentamiento)).toThrow(MovilizacionInvalidaError);

    const yaEnganchada = adjuntarCaravana(e, c, asentamiento).ejercito;
    expect(() => adjuntarCaravana(yaEnganchada, c, asentamiento)).toThrow(MovilizacionInvalidaError);
  });

  it('soltarla la deja donde está la columna, y rechaza soltar la que no lleva', () => {
    const { asentamiento } = base();
    const c = caravanaDe('c1', asentamiento.id, asentamiento.posicion);
    const e = adjuntarCaravana(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 0), c, asentamiento).ejercito;

    expect(soltarCaravana(e, c).ejercito.caravanasAdjuntasIds).toEqual([]);
    expect(() => soltarCaravana(e, { ...c, id: 'no-existe' })).toThrow(MovilizacionInvalidaError);
  });

  it('viajan CON el ejército: su posición sigue a la columna', () => {
    const { asentamiento } = base();
    const c = caravanaDe('c1', asentamiento.id, asentamiento.posicion);
    const e = adjuntarCaravana(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 500), c, asentamiento).ejercito;

    const r = avanzar([e], [asentamiento], { caravanas: [c] });

    expect(r.ejercitos[0]!.posicionActual).not.toEqual(asentamiento.posicion); // se movió
    expect(r.caravanas[0]!.posicionActual, 'la caravana va enganchada, no se queda atrás').toEqual(
      r.ejercitos[0]!.posicionActual
    );
  });

  it('el carro se llena hasta la capacidad AMPLIADA por las adjuntas', () => {
    const { asentamiento } = base();
    const rico: Asentamiento = {
      ...asentamiento,
      almacen: { ...asentamiento.almacen, trigo: { cantidad: 100000, capacidad: 100000 } },
    };
    const c = caravanaDe('c1', rico.id, rico.posicion);
    const e = adjuntarCaravana({ ...ejercitoDe(rico, [escuadron('a', 'milicia_lanceros')], 0), estado: 'estacionado' as const }, c, rico).ejercito;

    const r = avanzar([e], [rico], { caravanas: [c] });

    // Repostando en su propia plaza llena hasta carro + caravana, no solo hasta el carro.
    expect(r.ejercitos[0]!.suministro['trigo']).toBe(
      LOGISTICA.capacidadCarroPorJugador + CARAVANA_CATALOGO.comercial.capacidad
    );
  });

  it('si el ejército se deshace, las adjuntas se PIERDEN (Doc 5.13.2)', () => {
    const { asentamiento } = base();
    const c = caravanaDe('c1', asentamiento.id, asentamiento.posicion);
    // Escuadrón aniquilado: el ejército se disuelve en este tick.
    const e = adjuntarCaravana(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 0), c, asentamiento).ejercito;

    const r = avanzar([e], [asentamiento], { caravanas: [c] });

    expect(r.ejercitos, 'el ejército se disolvió').toHaveLength(0);
    expect(r.caravanas, 'y su tren de suministros con él').toHaveLength(0);
    expect(r.eventos.some((ev) => typeof ev !== 'string' && ev.codigo === 'ejercito.caravanas_perdidas')).toBe(true);
  });

  it('un ejército sin adjuntas no toca las caravanas del mundo', () => {
    const { asentamiento } = base();
    const ajena = caravanaDe('c-ajena', asentamiento.id, { x: 1, y: 1 });
    const e = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 500);

    const r = avanzar([e], [asentamiento], { caravanas: [ajena] });

    expect(r.caravanas).toEqual([ajena]);
  });
});

// ---------------------------------------------------------------------------------------------------------
// Paso 9b — la escolta: carga y entrega CONSCIENTES (Doc 5.13.3).
// ---------------------------------------------------------------------------------------------------------

describe('escolta: cargar y entregar a mano', () => {
  it('engancharla la saca de `disponible` — el comercio automático deja de verla', () => {
    // El bug que esto cierra: mientras seguía 'disponible', `asignarCaravanasATrueque` podía despacharla por
    // debajo del ejército que la lleva.
    const { asentamiento } = base();
    const c = caravanaDe('c1', asentamiento.id, asentamiento.posicion);
    const r = adjuntarCaravana(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 0), c, asentamiento);

    expect(r.caravana.estado).toBe('adjunta');
    expect(soltarCaravana(r.ejercito, r.caravana).caravana.estado, 'soltarla la devuelve al pool').toBe('disponible');
  });

  it('el jugador elige QUÉ carga, del almacén de una plaza al alcance', () => {
    const { asentamiento } = base();
    const rico: Asentamiento = {
      ...asentamiento,
      almacen: { ...asentamiento.almacen, piedra: { cantidad: 300, capacidad: 1000 } },
    };
    const c = caravanaDe('c1', rico.id, rico.posicion);
    const { ejercito, caravana } = adjuntarCaravana(ejercitoDe(rico, [escuadron('a', 'milicia_lanceros')], 0), c, rico);

    const r = cargarCaravanaAdjunta(ejercito, caravana, rico, 'piedra', 120, []);

    expect(r.cargado).toBe(120);
    expect(r.caravana.contenido['piedra']).toBe(120);
    expect(r.plaza.almacen['piedra']!.cantidad, 'sale del almacén de la plaza').toBe(180);
  });

  it('no carga más de lo que cabe ni de lo que hay, y rechaza plazas lejanas o ajenas', () => {
    const { asentamiento } = base();
    const conPoco: Asentamiento = {
      ...asentamiento,
      almacen: { ...asentamiento.almacen, piedra: { cantidad: 10, capacidad: 1000 } },
    };
    const c = caravanaDe('c1', conPoco.id, conPoco.posicion);
    const { ejercito, caravana } = adjuntarCaravana(ejercitoDe(conPoco, [escuadron('a', 'milicia_lanceros')], 0), c, conPoco);

    expect(cargarCaravanaAdjunta(ejercito, caravana, conPoco, 'piedra', 999, []).cargado, 'tope por stock').toBe(10);

    const lejos: Asentamiento = { ...conPoco, posicion: { x: conPoco.posicion.x + 1000, y: conPoco.posicion.y } };
    expect(() => cargarCaravanaAdjunta(ejercito, caravana, lejos, 'piedra', 5, [])).toThrow(MovilizacionInvalidaError);

    const ajena: Asentamiento = { ...conPoco, faccionId: 'faccion-2' };
    expect(() => cargarCaravanaAdjunta(ejercito, caravana, ajena, 'piedra', 5, [])).toThrow(MovilizacionInvalidaError);
  });

  it('`ladoPendienteParaEjercito` dice de qué lado estás y cuánto debes — lo que pinta la interfaz', () => {
    const { asentamiento } = base();
    const otro: Asentamiento = { ...asentamiento, id: 'otro', faccionId: 'faccion-2' };
    const e = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 0);
    const acuerdo: AcuerdoTrueque = {
      id: 'ac1',
      asentamientoAId: asentamiento.id,
      asentamientoBId: otro.id,
      recursoA: 'piedra',
      recursoB: 'oro',
      cantidadTotalA: 100,
      cantidadTotalB: 50,
      cantidadEntregadaA: 30,
      cantidadEntregadaB: 0,
      creadoEn: instanteDeTest(0),
      expiraEn: instanteDeTest(1000),
      estado: 'activo',
    };

    const mio = ladoPendienteParaEjercito(e, acuerdo, [asentamiento, otro])!;
    expect(mio.lado).toBe('A');
    expect(mio.recurso).toBe('piedra');
    expect(mio.faltante, '100 pactadas menos 30 ya entregadas').toBe(70);
    expect(mio.destinoId, 'se entrega en el OTRO lado').toBe('otro');

    // Saldado o cerrado: no hay nada que entregar.
    expect(ladoPendienteParaEjercito(e, { ...acuerdo, cantidadEntregadaA: 100 }, [asentamiento, otro])).toBeNull();
    expect(ladoPendienteParaEjercito(e, { ...acuerdo, estado: 'cumplido' }, [asentamiento, otro])).toBeNull();
    // Y un ejército de una Facción que no es parte del trueque tampoco.
    expect(ladoPendienteParaEjercito({ ...e, faccionId: 'faccion-3' }, acuerdo, [asentamiento, otro])).toBeNull();
  });

  it('entregar avanza el trueque, deja la mercancía y cobra comisión', () => {
    const { asentamiento } = base();
    const otro: Asentamiento = { ...asentamiento, id: 'otro', faccionId: 'faccion-2', posicion: { x: asentamiento.posicion.x + 10, y: asentamiento.posicion.y } };
    const c = { ...caravanaDe('c1', asentamiento.id, asentamiento.posicion), contenido: { piedra: 200 }, estado: 'adjunta' as const };
    const acuerdo: AcuerdoTrueque = {
      id: 'ac1',
      asentamientoAId: asentamiento.id,
      asentamientoBId: otro.id,
      recursoA: 'piedra',
      recursoB: 'oro',
      cantidadTotalA: 100,
      cantidadTotalB: 50,
      cantidadEntregadaA: 0,
      cantidadEntregadaB: 0,
      creadoEn: instanteDeTest(0),
      expiraEn: instanteDeTest(1000),
      estado: 'activo',
    };

    const r = entregarDesdeCaravanaAdjunta(c, acuerdo, 'A', 'piedra', 100, otro, asentamiento, [asentamiento, otro], crearFacciones());

    expect(r.entregado, 'entrega el FALTANTE, no todo lo que carga').toBe(100);
    expect(r.caravana.contenido['piedra'], 'y conserva el resto').toBe(100);
    expect(r.acuerdo.cantidadEntregadaA).toBe(100);
    expect(r.destino.almacen['piedra']!.cantidad).toBeGreaterThan(asentamiento.almacen['piedra']?.cantidad ?? 0);
    expect(r.comision, 'el destino cobra su comisión, igual que en una entrega automática').toBeGreaterThan(0);
  });
});
