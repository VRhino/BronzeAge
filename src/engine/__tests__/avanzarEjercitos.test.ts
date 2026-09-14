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
import type { AcuerdoTrueque, Asentamiento, Caravana, Ejercito, Escuadron, Faccion, Heroe, RelacionPolitica } from '../../domain/types';
import { LOGISTICA, MILITAR, MOVIMIENTO, TROPAS_RECLUTABLES } from '../../constants';
import { instante } from '../../domain/tiempo';
import { capacidadCaravana, velocidadCaravana } from '../caravanas';
import { entregarDesdeCaravanaAdjunta } from '../trade';
import {
  adjuntarCaravana,
  atacarColumna,
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
import { campamentoDe, conTropa, indiceTropa, sinTropa, type EjercitoConTropa } from '../tropa';
import {
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  contextoDeTest,
  escuadronDePrueba,
  fundarAsentamientoDeTest,
  heroesCon,
  instanteDeTest,
} from './fixtures';
import { avanzarSimulacion } from '../simulation';
import { createRng, type RandomFn } from '../../worldgen';

const mapa = crearMapaDeterminista(42);

function base() {
  const { asentamiento, facciones } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  return { asentamiento, facciones };
}

/** El residente de la plaza de `base()`: sus escuadras vuelven a casa al regresar (Doc 5.12.6). */
const RESIDENTE = 'jugador-faccion-1-1';
/** El residente de la segunda plaza que fundan los tests (la de faccion-2). */
const RIVAL = 'jugador-faccion-2-1';

const escuadron = (id: string, tropaId: string, cantidad = 10, moral = 100): Escuadron =>
  escuadronDePrueba(id, RESIDENTE, tropaId, cantidad, { moral });

/**
 * Aleja un ejército de cualquier plaza amiga. Desde el Paso 8, un ejército a menos de
 * `LOGISTICA.radioReabastecimiento` de una ciudad suya REPONE cada tick, así que medir el consumo del carro
 * junto a su propio asentamiento mide otra cosa: el saldo neto de comer y repostar a la vez.
 */
function enCampoAbierto<E extends Ejercito>(ejercito: E, origen: Asentamiento): E {
  const lejos = { x: origen.posicion.x, y: origen.posicion.y + LOGISTICA.radioReabastecimiento * 4 };
  return { ...ejercito, posicionActual: lejos, ruta: [lejos, { x: lejos.x, y: lejos.y + 400 }] };
}

/** Ejército sintético que sale de `origen` hacia un punto lejano, con lo que se le indique en el carro. Es una
 * vista con la tropa puesta: `avanzar` saca de ella los héroes dueños. */
function ejercitoDe(origen: Asentamiento, escuadrones: Escuadron[], trigo: number, estado: Ejercito['estado'] = 'marchando'): EjercitoConTropa {
  const destino = { x: origen.posicion.x + 600, y: origen.posicion.y };
  return {
    id: 'ejercito-1',
    faccionId: origen.faccionId,
    origenAsentamientoId: origen.id,
    participantes: [...new Set(escuadrones.map((e) => e.heroeId))].map((heroeId) => ({ heroeId, unidoEn: instante(0) })),
    tipo: 'ejercito',
    politicaDeUnion: 'rechazar',
    liderId: escuadrones[0]?.heroeId ?? RESIDENTE,
    escuadronIds: escuadrones.map((e) => e.id),
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
  opciones: {
    facciones?: Faccion[];
    relaciones?: RelacionPolitica[];
    rng?: RandomFn;
    caravanas?: Caravana[];
    /** Escuadras en el campamento de algún residente: la guarnición que defiende un asedio. */
    campamento?: Escuadron[];
    /** Para encadenar ticks: los héroes que devolvió el anterior. Sin él se sacan de las vistas. */
    heroes?: Heroe[];
    /** Héroes heridos durante todo el tick (Doc 5.16.4). */
    heridos?: string[];
  } = {}
) {
  // Una vista que pasó por `adjuntarCaravana` sigue llevando su tropa aunque el tipo la pierda.
  const enColumnas = ejercitos.flatMap((e) => ('escuadrones' in e ? sinTropa(e as EjercitoConTropa).tropa : []));
  const heroes = (
    opciones.heroes ?? heroesCon([...enColumnas, ...(opciones.campamento ?? [])], [...new Set(asentamientos.flatMap((a) => a.heroesFundadoresIds))])
  ).map((h) => (opciones.heridos?.includes(h.id) ? { ...h, heridoHasta: instanteDeTest(9999) } : h));
  const r = avanzarEjercitos(ejercitos, {
    asentamientos,
    caravanas: opciones.caravanas ?? [],
    facciones: opciones.facciones ?? crearFacciones(),
    relaciones: opciones.relaciones ?? [],
    mapa,
    instante: instanteDeTest(1),
    rng: opciones.rng ?? createRng(1),
    heroes,
  });
  const indice = indiceTropa(r.heroes);
  return {
    ...r,
    ejercitos: r.ejercitos.map((e) => conTropa(e, indice)),
    campamento: (asentamientoId: string) => campamentoDe(r.asentamientos.find((a) => a.id === asentamientoId)!, r.heroes),
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
    const ejercito = enCampoAbierto(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 100), asentamiento);

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos[0]!.progreso).toBeGreaterThan(0);
    // Comen los diez soldados Y el jugador que los lleva (Doc 5.13): la ración por participante es lo que
    // impide que viajar salga gratis a quien va sin tropa.
    expect(r.ejercitos[0]!.suministro['trigo']).toBeCloseTo(100 - 10 * MILITAR.racionPorSoldadoPorMinuto - MOVIMIENTO.consumoPorParticipante);
    // El almacén del asentamiento no se toca: en marcha se come del carro (Doc 5.13).
    expect(r.asentamientos[0]!.almacen['trigo']?.cantidad).toBe(trigoEnGranero);
  });

  it('estacionado no avanza, pero sigue comiendo — a consumo reducido (Doc 5.12.3)', () => {
    const { asentamiento } = base();
    const ejercito = enCampoAbierto(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 100, 'estacionado'), asentamiento);

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos[0]!.progreso).toBe(0);
    expect(r.ejercitos[0]!.posicionActual).toEqual(ejercito.posicionActual);
    const racionCompleta = 10 * MILITAR.racionPorSoldadoPorMinuto + MOVIMIENTO.consumoPorParticipante;
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
  // Lo que retira una columna es quedarse SIN NADIE DENTRO, no sin soldados. La versión anterior de estos
  // tests congelaba lo segundo, y con ello borraba del mapa a un jugador que seguía ahí.
  it('sin un solo soldado en pie NO se disuelve: sus heroes siguen dentro, ahora a pie', () => {
    const { asentamiento } = base();
    // Escuadrón aniquilado pero vivo como identidad: es lo que Doc 5.4 preserva para poder rellenarlo.
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 0);

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos, 'el jugador sigue viajando, solo que sin tropa').toHaveLength(1);
    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'ejercito.disuelto')).toBe(false);
  });

  it('y viaja a la velocidad del JUGADOR, no a cero (Doc 5.12.1)', () => {
    const { asentamiento } = base();
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 100);

    expect(velocidadDeEjercito(ejercito)).toBe(MOVIMIENTO.velocidadJugador);
    // Por encima de CUALQUIER tropa: un hombre solo no arrastra impedimenta.
    expect(velocidadDeEjercito(ejercito)).toBeGreaterThan(Math.max(...TROPAS_RECLUTABLES.map((t) => t.velocidad)));
  });

  it('come aunque no le quede un solo soldado: viajar nunca es gratis', () => {
    const { asentamiento } = base();
    const ejercito = enCampoAbierto(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 100), asentamiento);

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos[0]!.suministro['trigo']).toBeCloseTo(100 - MOVIMIENTO.consumoPorParticipante, 5);
  });

  it('NO se disuelve mientras quede alguien, aunque otro escuadrón esté a cero', () => {
    const { asentamiento } = base();
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0), escuadron('b', 'honderos', 5)], 100);

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos).toHaveLength(1);
  });

  it('sin NADIE dentro sí se disuelve, y sus identidades vuelven a casa', () => {
    const { asentamiento } = base();
    // Vaciar la lista de participantes es lo que harán separarse (Doc 5.14.2) y desconectarse (Doc 1.10.6);
    // aquí se construye a mano porque ninguno de los dos comandos existe todavía.
    const ejercito = { ...ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 0), participantes: [] };

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos).toHaveLength(0);
    const devuelto = r.campamento(asentamiento.id).find((e) => e.id === 'a');
    expect(devuelto, 'la identidad vacía tiene que volver al asentamiento, es donde se rellena').toBeDefined();
    expect(devuelto!.cantidad).toBe(0);
    expect(devuelto!.nombre).toBe('milicia_lanceros');
    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'ejercito.disuelto')).toBe(true);
  });

  it('si su asentamiento ya no existe, se disuelve igual y las identidades se pierden con él', () => {
    const { asentamiento } = base();
    const ejercito = { ...ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 0), participantes: [] };

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
    expect(r.campamento(asentamiento.id).map((e) => e.id)).toContain('a');
    // El sobrante vuelve al almacén — descontado lo que se comió en este último tick.
    const comido = 10 * MILITAR.racionPorSoldadoPorMinuto + MOVIMIENTO.consumoPorParticipante;
    expect(r.asentamientos[0]!.almacen['trigo']!.cantidad).toBeCloseTo(trigoAntes + 60 - comido);
    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'ejercito.regresa')).toBe(true);
  });
});

describe('está enchufado al tick y NO consume aleatoriedad', () => {
  it('un ejército se mueve dentro de `avanzarSimulacion`', () => {
    const { asentamiento, facciones } = base();
    const { ejercito, tropa } = sinTropa(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 500));
    const estado = crearEstadoDeTest([asentamiento], facciones, { ejercitos: [ejercito], heroes: heroesCon(tropa) });

    const tras = avanzarSimulacion(estado, mapa, contextoDeTest(1, createRng(42)));

    expect(tras.ejercitos[0]!.progreso).toBeGreaterThan(0);
  });

  it('con y sin ejércitos, el tick consume el MISMO número de valores del RNG', () => {
    // Es lo que permite que el guardián de determinismo siga verde sin tocarlo: añadir la mecánica no
    // desplaza el flujo de aleatoriedad de una partida que no la usa.
    const contar = (conEjercito: boolean) => {
      const { asentamiento, facciones } = base();
      const columna = sinTropa(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 500));
      const estado = crearEstadoDeTest([asentamiento], facciones, conEjercito ? { ejercitos: [columna.ejercito], heroes: heroesCon(columna.tropa) } : {});
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
    const disuelto = { ...ejercitoDe(a, [escuadron('s2', 'milicia_lanceros', 0)], 0), id: 'ejercito-2', participantes: [] };
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

/** Dos asentamientos de Facciones distintas, y un ejército del primero a un paso de plantarse en el segundo. Los
 * defensores son el campamento del residente rival (Doc 5.15.2): hay que pasarlos a `avanzar` como `campamento`. */
function frenteDeGuerra(defensores: Escuadron[], atacantes = [escuadron('a1', 'milicia_lanceros', 50)]) {
  const facciones = crearFacciones();
  const propio = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
  const enemigoBase = fundarAsentamientoDeTest(mapa, propio.facciones, 'faccion-2', [propio.asentamiento]);
  const enemigo = enemigoBase.asentamiento;
  // Defienden desde la guarnición: el resto del campamento no defiende (Doc 5.15.3).
  const campamento = defensores.map((e) => ({ ...e, heroeId: RIVAL, enGuarnicion: true }));

  const ejercito: EjercitoConTropa = {
    ...ejercitoDe(propio.asentamiento, atacantes, 5000),
    objetivo: { tipo: 'asentamiento', id: enemigo.id },
    ruta: [propio.asentamiento.posicion, enemigo.posicion],
    progreso: 0.999, // llega en este mismo tick
  };
  return { facciones: enemigoBase.facciones, propio: propio.asentamiento, enemigo, ejercito, campamento };
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

  it('con todos sus héroes heridos, el ejército espera a la puerta en vez de asediar (Doc 5.16.4)', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([]);

    const r = avanzar([ejercito], [propio, enemigo], { facciones, heridos: ejercito.participantes.map((p) => p.heroeId) });

    expect(r.asentamientos.find((a) => a.id === enemigo.id)!.faccionId, 'la plaza no cae').toBe(enemigo.faccionId);
    expect(r.ejercitos[0]!.estado, 'no acampa: la llegada se vuelve a mirar el tick siguiente').toBe('marchando');
  });

  it('conquistar deja la plaza SIN guarnición y al vencido con su campamento a 0, pero suyo (Doc 5.15.5)', () => {
    const veterano: Escuadron = { ...escuadron('d1', 'milicia_lanceros', 1), experiencia: 3 };
    const { facciones, propio, enemigo, ejercito, campamento } = frenteDeGuerra([veterano]);

    const r = avanzar([ejercito], [propio, enemigo], { facciones, campamento });

    const despues = r.asentamientos.find((a) => a.id === enemigo.id)!;
    expect(despues.faccionId, 'con 50 atacantes contra 1 defensor la plaza cae').toBe('faccion-1');
    expect(r.campamento(enemigo.id), 'nadie reside ya ahí: no hay guarnición').toEqual([]);
    const suya = r.heroes.find((h) => h.id === RIVAL)!.escuadrones.find((e) => e.id === 'd1')!;
    expect(suya.cantidad).toBe(0);
    expect(suya.experiencia, 'la escuadra conserva lo aprendido').toBeGreaterThanOrEqual(3);
    expect(despues.ocupacionHasta, 'abre la ventana de ocupación').toBeDefined();
  });

  it('conquistar deja HUÉRFANOS a los residentes y saquea la plaza (Doc 5.4 / Ocupacion §2.2)', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([]);
    expect(enemigo.heroesFundadoresIds.length, 'la plaza arranca con residentes').toBeGreaterThan(0);

    const r = avanzar([ejercito], [propio, enemigo], { facciones });
    const despues = r.asentamientos.find((a) => a.id === enemigo.id)!;

    expect(despues.heroesFundadoresIds).toEqual([]);
    expect(despues.casasCompradas).toEqual([]);
    expect(Object.values(despues.cargos).every((v) => v === null)).toBe(true);
    // Los edificios no se BORRAN (siguen en el array, dañados en la cola), pero la población se saquea.
    expect(despues.edificios.length).toBe(enemigo.edificios.length);
    expect(despues.poblacion.pesants).toBeLessThan(enemigo.poblacion.pesants);
    expect(despues.poblacion.nobleza, 'la nobleza no se saquea').toBe(enemigo.poblacion.nobleza);
  });

  it('el ejército conquistador NO se vuelve guarnición: acampa a la puerta con su tropa (Doc 5.15.5)', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([]);

    const r = avanzar([ejercito], [propio, enemigo], { facciones });

    expect(r.ejercitos).toHaveLength(1);
    expect(r.ejercitos[0]!.estado).toBe('estacionado');
    expect(r.ejercitos[0]!.escuadronIds).toEqual(['a1']);
    expect(r.campamento(enemigo.id)).toEqual([]);
  });

  it('una plaza recién conquistada es INMUNE a un segundo ejército el mismo tick (Ocupacion §2.4)', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([]);
    // La Facción desalojada manda un ejército a recuperar su plaza en el mismo tick — y rebota.
    const segundo: EjercitoConTropa = {
      ...ejercitoDe(propio, [escuadron('b1', 'milicia_lanceros', 80)], 5000),
      id: 'ejercito-segundo',
      faccionId: 'faccion-2',
      origenAsentamientoId: propio.id,
      objetivo: { tipo: 'asentamiento', id: enemigo.id },
      ruta: [propio.posicion, enemigo.posicion],
      progreso: 0.999,
    };

    const r = avanzar([ejercito, segundo], [propio, enemigo], { facciones });

    const plaza = r.asentamientos.find((a) => a.id === enemigo.id)!;
    expect(plaza.faccionId, 'la conquistó el primero; el segundo rebota').toBe('faccion-1');
    expect(r.ejercitos.find((e) => e.id === 'ejercito-1')!.escuadrones[0]!.cantidad, 'el primero, intacto a la puerta').toBe(50);
    const segundoDespues = r.ejercitos.find((e) => e.id === 'ejercito-segundo')!;
    expect(segundoDespues.escuadrones[0]!.cantidad, 'rebotó sin combatir: ni una baja').toBe(80);
  });

  it('el asedio se resuelve UNA vez: acampado junto a la plaza ya no vuelve a atacar', () => {
    const { facciones, propio, enemigo, ejercito, campamento } = frenteDeGuerra([escuadron('d1', 'milicia_lanceros', 200)]);

    const primero = avanzar([ejercito], [propio, enemigo], { facciones, campamento });
    const defensorTrasPrimero = primero.campamento(enemigo.id);
    expect(primero.ejercitos[0]!.estado, 'resistió, y el atacante queda acampado').toBe('estacionado');

    const segundo = avanzar(primero.ejercitos, primero.asentamientos, { facciones: primero.facciones, heroes: primero.heroes });

    expect(segundo.campamento(enemigo.id).map((e) => e.cantidad)).toEqual(defensorTrasPrimero.map((e) => e.cantidad));
    expect(segundo.eventos.some((e) => typeof e !== 'string' && e.codigo.startsWith('combate.'))).toBe(false);
  });

  it('llegar a un asentamiento PROPIO no dispara nada: solo acampa', () => {
    // El segundo se re-etiqueta en vez de fundarse: el cap de fundación en nivel 1 es UNO por Facción
    // (`CAP_FUNDACION_POR_NIVEL`), y lo que aquí importa es el destino, no cómo llegó a ser propio.
    const { facciones, propio, enemigo, campamento } = frenteDeGuerra([escuadron('d1', 'milicia_lanceros', 200)]);
    const otroPropio: Asentamiento = { ...enemigo, faccionId: 'faccion-1' };
    const e: EjercitoConTropa = {
      ...ejercitoDe(propio, [escuadron('a1', 'milicia_lanceros', 50)], 5000),
      objetivo: { tipo: 'asentamiento', id: otroPropio.id },
      ruta: [propio.posicion, otroPropio.posicion],
      progreso: 0.999,
    };

    const r = avanzar([e], [propio, otroPropio], { facciones, campamento });

    expect(r.asentamientos.find((a) => a.id === otroPropio.id)!.faccionId).toBe('faccion-1');
    expect(r.campamento(otroPropio.id).map((x) => x.cantidad), 'ni un rasguño a la guarnición amiga').toEqual([200]);
    expect(r.eventos.some((ev) => typeof ev !== 'string' && ev.codigo === 'ejercito.llega')).toBe(true);
    expect(r.eventos.some((ev) => typeof ev !== 'string' && ev.codigo.startsWith('combate.'))).toBe(false);
  });

  it('un asedio RESISTIDO se narra a los dos lados: al hogar del atacante y a la plaza que aguantó', () => {
    const { facciones, propio, enemigo, ejercito, campamento } = frenteDeGuerra([escuadron('d1', 'milicia_lanceros', 200)]);

    const r = avanzar([ejercito], [propio, enemigo], { facciones, campamento });
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
    const suDueno = propio.heroesFundadoresIds[0]!;
    const enCampana: EjercitoConTropa = {
      ...ejercitoDe(propio, [{ ...escuadron('a1', 'milicia_lanceros', 40), heroeId: suDueno }], 5000),
      id: 'ejercito-suyo',
      objetivo: { tipo: 'asentamiento', id: enemigo.id },
      ruta: [propio.posicion, enemigo.posicion],
      progreso: 0.5, // sigue de marcha: no llega ni conquista nada este tick
    };
    // Y a la vez, una columna enemiga se planta en SU ciudad, que quedó desguarnecida.
    const invasor: EjercitoConTropa = {
      ...ejercitoDe(enemigo, [{ ...escuadron('inv', 'milicia_lanceros', 40), heroeId: RIVAL }], 5000),
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
    const { facciones, propio, enemigo, campamento } = frenteDeGuerra([escuadron('d1', 'milicia_lanceros', 30)]);
    const uno: EjercitoConTropa = {
      ...ejercitoDe(propio, [escuadron('a1', 'milicia_lanceros', 20)], 5000),
      id: 'ejercito-aaa',
      objetivo: { tipo: 'asentamiento', id: enemigo.id },
      ruta: [propio.posicion, enemigo.posicion],
      progreso: 0.999,
    };
    const dos: EjercitoConTropa = { ...uno, id: 'ejercito-bbb', escuadronIds: ['a2'], escuadrones: [escuadron('a2', 'honderos', 20)] };

    const enOrden = avanzar([uno, dos], [propio, enemigo], { facciones, rng: createRng(7), campamento });
    const alReves = avanzar([dos, uno], [propio, enemigo], { facciones, rng: createRng(7), campamento });

    const resumen = (r: ReturnType<typeof avanzar>) =>
      JSON.stringify({
        ejercitos: [...r.ejercitos].sort((a, b) => (a.id < b.id ? -1 : 1)).map((e) => e.escuadrones.map((x) => x.cantidad)),
        defensor: r.heroes.find((h) => h.id === RIVAL)!.escuadrones.map((x) => x.cantidad),
      });
    expect(resumen(enOrden)).toBe(resumen(alReves));
  });
});

// ---------------------------------------------------------------------------------------------------------
// Paso 8 — repostar al pasar (Doc 5.13).
// ---------------------------------------------------------------------------------------------------------

describe('reabastecimiento en ruta', () => {
  /** Un ejército a medio carro, plantado justo encima de `plaza`. */
  function juntoA(plaza: Asentamiento, origen: Asentamiento, faccionId = origen.faccionId): EjercitoConTropa {
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
    // La plaza no tiene campamento (el ejército lleva la única escuadra), así que su reserva es la de la población.
    const reserva = reservaDeTrigo(asentamiento, 0);
    const apurado: Asentamiento = {
      ...asentamiento,
      almacen: { ...asentamiento.almacen, trigo: { ...asentamiento.almacen['trigo']!, cantidad: reserva + 20 } },
    };

    const r = avanzar([juntoA(apurado, apurado)], [apurado]);

    const plaza = r.asentamientos[0]!;
    expect(plaza.almacen['trigo']!.cantidad).toBeGreaterThanOrEqual(reservaDeTrigo(plaza, 0) - 1e-9);
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
    const correr = (inicial: EjercitoConTropa) => {
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
  // Caravana por defecto del revamp (Doc 3.13): 1 carro básico + 1 buey → deriva 500/16.
  carros: [{ tipoCarro: 'basico', animal: 'buey' }],
});

describe('caravanas adjuntas', () => {
  it('la capacidad de una caravana NO es menor que el carro de un Heroe (Doc 5.13.2)', () => {
    // El invariante que justifica el rebalance del Paso 9: si cargara menos, engancharla no tendría sentido.
    expect(capacidadCaravana(caravanaDe('c', 'o', { x: 0, y: 0 }))).toBeGreaterThanOrEqual(LOGISTICA.capacidadCarroPorJugador);
  });

  it('suman su capacidad a la del carro', () => {
    const { asentamiento } = base();
    const e = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 0);
    const c = caravanaDe('c1', asentamiento.id, asentamiento.posicion);

    expect(capacidadCargaDe(e, [])).toBe(LOGISTICA.capacidadCarroPorJugador);
    const conCaravana = adjuntarCaravana(e, c, asentamiento).ejercito;
    expect(capacidadCargaDe(conCaravana, [c])).toBe(LOGISTICA.capacidadCarroPorJugador + capacidadCaravana(c));
  });

  it('entran en el MÍNIMO de velocidad: una comercial frena a una fuerza ligera de 20 a 16', () => {
    const { asentamiento } = base();
    const ligero = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 0);
    expect(velocidadDeEjercito(ligero)).toBe(20);

    const c = caravanaDe('c1', asentamiento.id, asentamiento.posicion);
    const conCaravana = { ...ligero, ...adjuntarCaravana(ligero, c, asentamiento).ejercito };
    expect(velocidadDeEjercito(conCaravana, [c])).toBe(velocidadCaravana(c));
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
    expect(r.ejercitos[0]!.suministro['trigo']).toBe(LOGISTICA.capacidadCarroPorJugador + capacidadCaravana(c));
  });

  it('si el ejército se deshace, las adjuntas se PIERDEN (Doc 5.13.2)', () => {
    const { asentamiento } = base();
    const c = caravanaDe('c1', asentamiento.id, asentamiento.posicion);
    // Sin nadie dentro: la columna se disuelve en este tick (Doc 5.13.4). Antes bastaba con que sus
    // escuadrones estuvieran a cero, y eso borraba del mapa a un jugador que seguía ahí.
    const conCaravana = adjuntarCaravana(ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros', 0)], 0), c, asentamiento).ejercito;
    const e = { ...conCaravana, participantes: [] };

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

// ---------------------------------------------------------------------------------------------------------
// Paso 10 — encuentros por proximidad (Doc 5.12.3). Nadie los ordena: salen de la geometría.
// ---------------------------------------------------------------------------------------------------------

// Encuentros: **ya no salen de la geometria** (paso 8, Doc 5.12.3). Acercarse abre opciones; pelear hay que
// pedirlo. Lo que esta funcion resuelve es el final de una PERSECUCION — elegir ir detras de alguien es
// elegir el combate—, y lo que desaparece es pelear por haber pasado cerca.
describe('encuentros: solo se resuelve lo que se persigue', () => {
  /** Dos ejércitos de Facciones distintas, a `separacion` uno de otro y sin nada más alrededor. */
  // El carro sale con 100 y no con miles: la capacidad de un carro son 500, y llenarlo por encima en el
  // fixture dejaba sitio 0 para el botín — que es exactamente lo que descubrió el test de la emboscada.
  function dosColumnas(separacion: number, escuadronesA = 30, escuadronesB = 30) {
    const facciones = crearFacciones();
    const uno = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const dos = fundarAsentamientoDeTest(mapa, uno.facciones, 'faccion-2', [uno.asentamiento]);
    // Lejos de las dos ciudades, para que no se mezcle el reposte ni la llegada.
    const punto = { x: 1000, y: 1000 };
    const a: EjercitoConTropa = {
      ...ejercitoDe(uno.asentamiento, [escuadron('a1', 'milicia_lanceros', escuadronesA)], 100, 'estacionado'),
      id: 'ejercito-a',
      posicionActual: punto,
    };
    const b: EjercitoConTropa = {
      ...ejercitoDe(dos.asentamiento, [{ ...escuadron('b1', 'milicia_lanceros', escuadronesB), heroeId: RIVAL }], 100, 'estacionado'),
      id: 'ejercito-b',
      faccionId: 'faccion-2',
      origenAsentamientoId: dos.asentamiento.id,
      posicionActual: { x: punto.x + separacion, y: punto.y },
    };
    return { facciones: dos.facciones, asentamientos: [uno.asentamiento, dos.asentamiento], a, b };
  }

  it('dos columnas enemigas que se cruzan NO combaten: nadie lo ha pedido', () => {
    // Era la regla contraria hasta el paso 8, y este test decia lo opuesto. Ahora pasar cerca no cuesta nada.
    const { facciones, asentamientos, a, b } = dosColumnas(LOGISTICA.radioEncuentro - 1);

    const r = avanzar([a, b], asentamientos, { facciones });

    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo.startsWith('combate.'))).toBe(false);
    expect(r.ejercitos.every((e) => e.escuadrones[0]!.cantidad === 30), 'ni un rasguño').toBe(true);
  });

  it('pero si uno PERSIGUE al otro y lo alcanza, hay combate', () => {
    const { facciones, asentamientos, a, b } = dosColumnas(LOGISTICA.radioEncuentro - 1);
    const cazador: Ejercito = { ...a, persiguiendo: { tipo: 'ejercito', id: b.id } };

    const r = avanzar([cazador, b], asentamientos, { facciones });

    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'combate.encuentro')).toBe(true);
    const totalDespues = r.ejercitos.reduce((n, e) => n + e.escuadrones.reduce((m, x) => m + x.cantidad, 0), 0);
    expect(totalDespues, 'los dos bandos sufren bajas').toBeLessThan(60);
  });

  it('y al alcanzarla suelta la presa: se persigue para pelear, y ya se peleo', () => {
    const { facciones, asentamientos, a, b } = dosColumnas(LOGISTICA.radioEncuentro - 1);
    const cazador: Ejercito = { ...a, persiguiendo: { tipo: 'ejercito', id: b.id } };

    const r = avanzar([cazador, b], asentamientos, { facciones });

    expect(r.ejercitos.find((e) => e.id === a.id)!.persiguiendo).toBeUndefined();
  });

  it('a quien NO persigue nadie no se le toca, aunque este pegado al que si', () => {
    const { facciones, asentamientos, a, b } = dosColumnas(1);
    const tercero: Ejercito = { ...b, id: 'ejercito-c', posicionActual: { ...a.posicionActual } };
    const cazador: Ejercito = { ...a, persiguiendo: { tipo: 'ejercito', id: b.id } };

    const r = avanzar([cazador, b, tercero], asentamientos, { facciones });

    expect(r.ejercitos.find((e) => e.id === 'ejercito-c')!.escuadrones[0]!.cantidad, 'el tercero ni se entera').toBe(30);
  });

  it('quien cae al ser alcanzado entrega la mitad del carro al que lo persiguió, igual que en un ataque (Doc 5.12.3)', () => {
    // Un soldado contra trescientos cae entero: en el tick solo cuenta como derrota quedarse sin nadie en pie.
    const { facciones, asentamientos, a, b } = dosColumnas(LOGISTICA.radioEncuentro - 1, 300, 1);
    const cazador: Ejercito = { ...a, persiguiendo: { tipo: 'ejercito', id: b.id } };
    // El mismo tick sin combate da lo que queda en cada carro después de comer, que va antes de los encuentros.
    const sinChoque = avanzar([a, b], asentamientos, { facciones }).ejercitos;
    const carroA = sinChoque.find((e) => e.id === a.id)!.suministro['trigo'] ?? 0;
    const carroB = sinChoque.find((e) => e.id === b.id)!.suministro['trigo'] ?? 0;

    const tras = avanzar([cazador, b], asentamientos, { facciones });
    const r = tras.ejercitos;
    const presa = r.find((e) => e.id === b.id)!;

    expect(presa.escuadrones.every((e) => e.cantidad === 0), 'la presa cae entera').toBe(true);
    expect(tras.heroes.find((h) => h.id === RIVAL)!.heridoHasta, 'su héroe queda herido (Doc 5.16.4)').toBeDefined();
    expect(tras.heroes.find((h) => h.id === RESIDENTE)!.heridoHasta, 'el que gana, no').toBeUndefined();
    expect(presa.suministro['trigo'], 'se queda con la mitad').toBe(carroB / 2);
    expect(r.find((e) => e.id === a.id)!.suministro['trigo'], 'y el cazador carga la otra mitad').toBe(carroA + carroB / 2);
  });

  it('una columna con todos sus héroes HERIDOS no se puede alcanzar, aunque la persigas y la tengas encima', () => {
    const { facciones, asentamientos, a, b } = dosColumnas(1);
    const cazador: Ejercito = { ...a, persiguiendo: { tipo: 'ejercito', id: b.id } };

    const r = avanzar([cazador, b], asentamientos, { facciones, heridos: [RIVAL] });

    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo.startsWith('combate.'))).toBe(false);
  });

  it('y quien lleva a todos sus héroes heridos tampoco alcanza a nadie: corta por los dos lados', () => {
    // Sin esta mitad, la inmunidad seria un escudo para depredar sin riesgo.
    const { facciones, asentamientos, a, b } = dosColumnas(1);
    const cazador: Ejercito = { ...a, persiguiendo: { tipo: 'ejercito', id: b.id } };

    const r = avanzar([cazador, b], asentamientos, { facciones, heridos: [RESIDENTE] });

    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo.startsWith('combate.'))).toBe(false);
  });

  it('un EJÉRCITO derrotado en campo abierto entrega la mitad del carro, igual que un viajero (Doc 5.12.3)', () => {
    // Hasta el 2026-09-13 solo la columna personal perdía el carro: el ejército se iba con la tregua y nada más.
    // Un ataque que no tumba al defensor es una derrota del ATACANTE: con 5 contra 300 no depende del RNG.
    const { facciones, a, b } = dosColumnas(1, 5, 300);

    const r = atacarColumna(a, b, facciones, [], [], new Set(), createRng(1));

    expect(r.atacante.tipo).toBe('ejercito');
    expect(r.atacante.suministro['trigo'], 'se queda con la mitad').toBe(50);
    expect(r.defensor.suministro['trigo'], 'y el vencedor carga la otra mitad').toBe(150);
    expect(r.vencidos, 'y sus héroes quedan heridos (Doc 5.16.4)').toEqual([RESIDENTE]);
  });

  it('el DEFENSOR que gana carga el botín con sus caravanas adjuntas, igual que el atacante (Doc 5.13.2)', () => {
    // Antes solo contaban sus carros: con el carro por encima de 500 gracias a la adjunta, no le cabía nada.
    const { facciones, a, b } = dosColumnas(1, 5, 300);
    const adjunta: Caravana = { ...caravanaDe('c-b', b.origenAsentamientoId, b.posicionActual), estado: 'adjunta' };
    const defensor: EjercitoConTropa = { ...b, caravanasAdjuntasIds: [adjunta.id], suministro: { trigo: 800 } };
    expect(capacidadCargaDe(defensor, [adjunta]), 'cabe más que el carro solo').toBeGreaterThan(850);

    const r = atacarColumna(a, defensor, facciones, [], [adjunta], new Set(), createRng(1));

    expect(r.defensor.suministro['trigo'], 'carga la mitad del carro del atacante').toBe(850);
  });

  it('las escuadras de un héroe herido no combaten: se apartan y vuelven intactas (Doc 5.16.4)', () => {
    // El sano lleva 1 soldado contra 300: cae entero, que es lo que el motor cuenta como derrota del defensor.
    const { facciones, a, b } = dosColumnas(1, 300, 1);
    const herida: Escuadron = { ...escuadron('b2', 'milicia_lanceros', 30), heroeId: 'heroe-herido' };
    const mixta: EjercitoConTropa = {
      ...b,
      escuadronIds: [...b.escuadronIds, herida.id],
      escuadrones: [...b.escuadrones, herida],
      participantes: [...b.participantes, { heroeId: 'heroe-herido', unidoEn: instante(0) }],
    };

    const r = atacarColumna(a, mixta, facciones, [], [], new Set(['heroe-herido']), createRng(1));

    expect(r.defensor.escuadrones.find((e) => e.id === 'b2')!.cantidad, 'ni pelea ni sufre bajas').toBe(30);
    expect(r.defensor.escuadrones.find((e) => e.id === 'b1')!.cantidad, 'la del sano sí').toBe(0);
    expect(r.vencidos, 'pierde la columna entera').toEqual([RIVAL, 'heroe-herido']);
  });

  it('a una columna de solo heridos no se la puede atacar, y una de solo heridos no ataca', () => {
    const { facciones, a, b } = dosColumnas(1);
    expect(() => atacarColumna(a, b, facciones, [], [], new Set([RIVAL]), createRng(1))).toThrow('solo lleva héroes heridos');
    expect(() => atacarColumna(a, b, facciones, [], [], new Set([RESIDENTE]), createRng(1))).toThrow('están heridos');
  });

  it('fuera del radio de encuentro no pasa nada, aunque se vean de sobra', () => {
    // A 15 se tropieza; a 150 se ve. Entre medias hay muchísimo sitio para decidir.
    const { facciones, asentamientos, a, b } = dosColumnas(LOGISTICA.radioEncuentro + 1);

    const r = avanzar([a, b], asentamientos, { facciones });

    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo.startsWith('combate.'))).toBe(false);
    expect(r.ejercitos.every((e) => e.escuadrones[0]!.cantidad === 30)).toBe(true);
  });

  it('los ALIADOS no se cruzan: compartir ruta no es masacrarse cada tick', () => {
    const { facciones, asentamientos, a, b } = dosColumnas(1);
    const alianza: RelacionPolitica[] = [
      { id: 'r1', faccionAId: 'faccion-1', faccionBId: 'faccion-2', tipo: 'alianza', estado: 'activa', creadoEn: instanteDeTest(0) },
    ];

    const r = avanzar([a, b], asentamientos, { facciones, relaciones: alianza });

    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo.startsWith('combate.'))).toBe(false);
  });

  it('tampoco se cruzan dos columnas de la MISMA Facción', () => {
    const { facciones, asentamientos, a, b } = dosColumnas(1);
    const mismaFaccion: Ejercito = { ...b, faccionId: a.faccionId };

    const r = avanzar([a, mismaFaccion], asentamientos, { facciones });

    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo.startsWith('combate.'))).toBe(false);
  });

  it('un ejército choca UNA vez por tick, aunque dos le persigan a la vez', () => {
    const { facciones, asentamientos, a, b } = dosColumnas(1);
    const tercero: Ejercito = { ...b, id: 'ejercito-c', posicionActual: { ...a.posicionActual }, persiguiendo: { tipo: 'ejercito' as const, id: a.id } };
    const cazador: Ejercito = { ...b, persiguiendo: { tipo: 'ejercito' as const, id: a.id } };

    const r = avanzar([a, cazador, tercero], asentamientos, { facciones });

    // Se cuentan PAREJAS y no eventos: cada encuentro se narra dos veces, una a cada hogar (ver la doble
    // atribución en `avanzarEjercitos`), así que contar eventos contaría el doble.
    const parejas = new Set(
      r.eventos
        .filter((e) => typeof e !== 'string' && e.codigo === 'combate.encuentro')
        .map((e) => JSON.stringify((e as { payload: { ejercitoAId: string; ejercitoBId: string } }).payload))
    );
    expect(parejas.size, 'un solo encuentro: sin cascada dentro del mismo minuto').toBe(1);
  });

  it('el orden es canónico por id, no el del array', () => {
    const { facciones, asentamientos, a, b } = dosColumnas(1);
    const resumen = (r: ReturnType<typeof avanzar>) =>
      JSON.stringify([...r.ejercitos].sort((x, y) => (x.id < y.id ? -1 : 1)).map((e) => e.escuadrones.map((s) => s.cantidad)));

    expect(resumen(avanzar([a, b], asentamientos, { facciones, rng: createRng(9) }))).toBe(
      resumen(avanzar([b, a], asentamientos, { facciones, rng: createRng(9) }))
    );
  });

  it('embosca una caravana enemiga en ruta y se queda con parte de la carga', () => {
    const { facciones, asentamientos, a } = dosColumnas(1);
    const suya = asentamientos[1]!; // la de faccion-2
    const presa: Caravana = {
      ...caravanaDe('c-presa', suya.id, { ...a.posicionActual }),
      estado: 'en_transito',
      contenido: { piedra: 100 },
    };

    const cazador: Ejercito = { ...a, persiguiendo: { tipo: 'caravana' as const, id: presa.id } };

    const r = avanzar([cazador], asentamientos, { facciones, caravanas: [presa], rng: createRng(3) });

    const ev = r.eventos.find((e) => typeof e !== 'string' && e.codigo === 'combate.caravana_interceptada_por_ejercito');
    expect(ev, 'hubo emboscada').toBeDefined();
    expect(r.caravanas, 'la caravana capturada se elimina').toHaveLength(0);
    expect(r.ejercitos[0]!.suministro['piedra'], 'el botín viaja en el carro').toBe(100 * MILITAR.umbralCapturaCaravana);
  });

  it('una caravana ESCOLTADA no es un objetivo blando: perseguirla no la alcanza', () => {
    // La escolta no la protege por un chequeo aparte: una caravana `adjunta` sencillamente no es presa. El
    // que quiera su carga tiene que perseguir al EJÉRCITO que la lleva, que es otra decisión y otro riesgo.
    const { facciones, asentamientos, a, b } = dosColumnas(1);
    const escoltada: Caravana = { ...caravanaDe('c-esc', asentamientos[1]!.id, { ...b.posicionActual }), estado: 'adjunta', contenido: { piedra: 100 } };
    const conEscolta: Ejercito = { ...b, caravanasAdjuntasIds: ['c-esc'] };
    const cazador: Ejercito = { ...a, persiguiendo: { tipo: 'caravana' as const, id: 'c-esc' } };

    const r = avanzar([cazador, conEscolta], asentamientos, { facciones, caravanas: [escoltada] });

    expect(
      r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'combate.caravana_interceptada_por_ejercito'),
      'no se la embosca por su cuenta'
    ).toBe(false);
    expect(r.caravanas, 'sigue viva mientras su escolta aguante').toHaveLength(1);
  });

  it('una caravana propia o de un aliado no se toca', () => {
    const { facciones, asentamientos, a } = dosColumnas(1);
    const propia: Caravana = {
      ...caravanaDe('c-propia', asentamientos[0]!.id, { ...a.posicionActual }),
      estado: 'en_transito',
      contenido: { piedra: 100 },
    };

    const r = avanzar([a], asentamientos, { facciones, caravanas: [propia] });

    expect(r.caravanas).toHaveLength(1);
    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo.startsWith('combate.'))).toBe(false);
  });
});
