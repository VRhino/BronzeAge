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
import type { Asentamiento, Ejercito, Escuadron, Faccion, RelacionPolitica } from '../../domain/types';
import { LOGISTICA, MILITAR } from '../../constants';
import { avanzarEjercitos, velocidadDeEjercito } from '../ejercitos';
import { esResidente, resideEnOtroAsentamiento } from '../pertenencia';
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
  opciones: { facciones?: Faccion[]; relaciones?: RelacionPolitica[]; rng?: RandomFn } = {}
) {
  return avanzarEjercitos(
    ejercitos,
    asentamientos,
    mapa,
    opciones.facciones ?? crearFacciones(),
    opciones.relaciones ?? [],
    instanteDeTest(1),
    opciones.rng ?? createRng(1)
  );
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

    const r = avanzar([ejercito], [asentamiento]);

    expect(r.ejercitos[0]!.progreso).toBeGreaterThan(0);
    expect(r.ejercitos[0]!.suministro['trigo']).toBeCloseTo(100 - 10 * MILITAR.racionPorSoldadoPorMinuto);
    // El almacén del asentamiento no se toca: en marcha se come del carro (Doc 5.13).
    expect(r.asentamientos[0]!.almacen['trigo']?.cantidad).toBe(trigoEnGranero);
  });

  it('estacionado no avanza, pero sigue comiendo — a consumo reducido (Doc 5.12.3)', () => {
    const { asentamiento } = base();
    const ejercito = ejercitoDe(asentamiento, [escuadron('a', 'milicia_lanceros')], 100, 'estacionado');

    const r = avanzar([ejercito], [asentamiento]);

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

  it('la guarnición del conquistado SE PIERDE, no pasa al conquistador (Doc 5.4)', () => {
    const { facciones, propio, enemigo, ejercito } = frenteDeGuerra([escuadron('d1', 'milicia_lanceros', 1)]);

    const r = avanzar([ejercito], [propio, enemigo], { facciones });

    const despues = r.asentamientos.find((a) => a.id === enemigo.id)!;
    expect(despues.faccionId, 'con 50 atacantes contra 1 defensor la plaza cae').toBe('faccion-1');
    expect(despues.escuadrones, 'los escuadrones del vencido no son botín transferible').toEqual([]);
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
