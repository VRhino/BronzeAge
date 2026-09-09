// Composición de una columna compartida (`session/comandos/columna.ts`, paso 4b, Doc 5.14).
//
// Lo que este archivo congela es el precio del paso 4: un viajero solo rectifica su rumbo cuando quiere, y
// unirse a un ejército lo cambia por un destino que ya no puede tocar. Ninguna regla impone ese coste — sale
// solo de que el ejército tiene el rumbo fijo, y por eso se prueba yendo a comprobar el `tipo`.
//
// Y las dos prohibiciones que sostienen "una columna nunca se queda vacía en campo abierto": el Líder no se
// separa, y el último tampoco. Se comprueban por separado porque son dos reglas, no una.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { cederLiderazgo, responderPeticionDeUnion, separarseDelEjercito, unirseEnCampo } from '../comandos/columna';
import { marcharA, salirAlMundo } from '../comandos/presencia';
import { movilizarEjercito, replegarEjercito } from '../comandos/ejercitos';
import { inspeccionar } from '../comandos/interaccion';
import { OPC, partidaConAsentamiento } from './fixtures';
import { LOGISTICA, MOVIMIENTO, SIMULACION, VISION } from '../../constants';

const opcDe = (jugadorId: string) => ({ ...OPC, actor: jugadorId });
const PUNTO_LEJOS = { tipo: 'punto', punto: { x: 430, y: 430 } } as const;

/** Partida con un escuadrón por jugador y almacén holgado. */
function partidaConDos() {
  const base = partidaConAsentamiento();
  const payload = base.sesion.exportar();
  const a = payload.state.asentamientos[0]!;
  const escuadron = (id: string, jugadorId: string) => ({
    id,
    nombre: 'milicia_lanceros',
    jugadorId,
    origen: 'pesants' as const,
    cantidad: 10,
    veterania: 0,
    moral: 100,
    tropaId: 'milicia_lanceros',
  });
  const sesion = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      asentamientos: [
        {
          ...a,
          almacen: { ...a.almacen, trigo: { capacidad: 100000, cantidad: 5000 } },
          escuadrones: [escuadron('esc-lider', base.fundador), escuadron('esc-vecino', base.vecino)],
        },
        ...payload.state.asentamientos.slice(1),
      ],
    },
  });
  return { ...base, sesion };
}

/**
 * Un ejército del fundador ya en marcha, y el vecino fuera en su columna personal, los dos en el mismo
 * punto para que la distancia no sea lo que estorbe.
 *
 * `politica` es la que el Líder fija al movilizar, y va donde va: en el comando que pare la columna.
 */
function ejercitoYViajero(politica: 'rechazar' | 'aceptar' | 'preguntar') {
  const base = partidaConDos();
  base.sesion.ejecutar(
    movilizarEjercito,
    { asentamientoId: base.asentamientoId, jugadorId: base.fundador, escuadronIds: ['esc-lider'], objetivo: PUNTO_LEJOS, politicaDeUnion: politica },
    opcDe(base.fundador)
  );
  base.sesion.ejecutar(
    salirAlMundo,
    { asentamientoId: base.asentamientoId, jugadorId: base.vecino, escuadronIds: ['esc-vecino'], carga: { trigo: 60 } },
    opcDe(base.vecino)
  );
  const payload = base.sesion.exportar();
  const ejercito = payload.state.ejercitos.find((e) => e.tipo === 'ejercito')!;
  const columna = payload.state.ejercitos.find((e) => e.tipo === 'personal')!;
  const sesion = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      // Se les pone en el mismo punto para que la distancia no sea lo que estorbe en estos tests.
      ejercitos: [ejercito, { ...columna, posicionActual: ejercito.posicionActual }],
    },
  });
  return { ...base, sesion, ejercitoId: ejercito.id, columnaId: columna.id };
}

describe('unirseEnCampo — el precio es la libertad de movimiento (Doc 5.14.1)', () => {
  it('con política `aceptar` funde la columna en el ejército y le pasa su destino', () => {
    const { sesion, ejercitoId, vecino } = ejercitoYViajero('aceptar');

    const r = sesion.ejecutar(unirseEnCampo, { ejercitoId, jugadorId: vecino }, opcDe(vecino));

    expect(r.ok).toBe(true);
    expect(sesion.getState().ejercitos, 'la columna personal deja de existir').toHaveLength(1);
    const ejercito = sesion.getState().ejercitos[0]!;
    expect(ejercito.participantes).toHaveLength(2);
    expect(ejercito.escuadrones.map((e) => e.id).sort()).toEqual(['esc-lider', 'esc-vecino']);
    expect(ejercito.suministro['trigo'], 'y su carro entra en el común').toBeGreaterThan(60);
    expect(sesion.getState().jugadores.find((j) => j.id === vecino)!.ubicacion).toEqual({ tipo: 'columna', ejercitoId });
  });

  it('y a partir de ahí ya no puede rectificar el rumbo: ese es el precio', () => {
    const { sesion, ejercitoId, vecino } = ejercitoYViajero('aceptar');
    sesion.ejecutar(unirseEnCampo, { ejercitoId, jugadorId: vecino }, opcDe(vecino));

    const r = sesion.ejecutar(marcharA, { jugadorId: vecino, objetivo: { tipo: 'punto', punto: { x: 420, y: 420 } } }, opcDe(vecino));

    expect(r.ok, 'marchar acompañado cuesta la libertad de movimiento').toBe(false);
  });

  it('con política `rechazar` no entra nadie', () => {
    const { sesion, ejercitoId, vecino } = ejercitoYViajero('rechazar');

    const r = sesion.ejecutar(unirseEnCampo, { ejercitoId, jugadorId: vecino }, opcDe(vecino));

    expect(r.ok).toBe(false);
    expect(sesion.getState().ejercitos).toHaveLength(2);
  });

  it('no se une desde lejos', () => {
    const { sesion, ejercitoId, columnaId, vecino } = ejercitoYViajero('aceptar');
    const payload = sesion.exportar();
    const lejos = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        ejercitos: payload.state.ejercitos.map((e) =>
          e.id === columnaId ? { ...e, posicionActual: { x: e.posicionActual.x + LOGISTICA.radioEncuentro + 1, y: e.posicionActual.y } } : e
        ),
      },
    });

    const r = lejos.ejecutar(unirseEnCampo, { ejercitoId, jugadorId: vecino }, opcDe(vecino));

    expect(r.ok).toBe(false);
  });

  it('un EJÉRCITO no se une a otro ejército', () => {
    const { sesion, ejercitoId, columnaId, vecino } = ejercitoYViajero('aceptar');
    const payload = sesion.exportar();
    // Se convierte la columna del vecino en un ejército: la geometría es la misma, lo que cambia es qué es.
    const dosEjercitos = GameSession.importar({
      ...payload,
      state: { ...payload.state, ejercitos: payload.state.ejercitos.map((e) => (e.id === columnaId ? { ...e, tipo: 'ejercito' as const } : e)) },
    });

    const r = dosEjercitos.ejecutar(unirseEnCampo, { ejercitoId, jugadorId: vecino }, opcDe(vecino));

    expect(r.ok, 'un ejército solo se origina en un asentamiento, no fundiendo dos en el camino').toBe(false);
  });
});

describe('unirseEnCampo con `preguntar` — el silencio es un no (Doc 5.14.1)', () => {
  it('deja una petición viva en vez de meter a nadie', () => {
    const { sesion, ejercitoId, vecino } = ejercitoYViajero('preguntar');

    const r = sesion.ejecutar(unirseEnCampo, { ejercitoId, jugadorId: vecino }, opcDe(vecino));

    expect(r.ok).toBe(true);
    expect(r.datos!.unido, 'pedir no es entrar').toBe(false);
    expect(sesion.getState().ejercitos).toHaveLength(2);
    expect(sesion.getState().ejercitos.find((e) => e.id === ejercitoId)!.peticionesDeUnion).toHaveLength(1);
  });

  it('el Líder acepta y entra', () => {
    const { sesion, ejercitoId, fundador, vecino } = ejercitoYViajero('preguntar');
    sesion.ejecutar(unirseEnCampo, { ejercitoId, jugadorId: vecino }, opcDe(vecino));

    const r = sesion.ejecutar(
      responderPeticionDeUnion,
      { ejercitoId, jugadorId: fundador, solicitanteId: vecino, aceptar: true },
      opcDe(fundador)
    );

    expect(r.ok).toBe(true);
    expect(sesion.getState().ejercitos).toHaveLength(1);
    expect(sesion.getState().ejercitos[0]!.participantes).toHaveLength(2);
  });

  it('solo el Líder contesta', () => {
    const { sesion, ejercitoId, vecino } = ejercitoYViajero('preguntar');
    sesion.ejecutar(unirseEnCampo, { ejercitoId, jugadorId: vecino }, opcDe(vecino));

    const r = sesion.ejecutar(
      responderPeticionDeUnion,
      { ejercitoId, jugadorId: vecino, solicitanteId: vecino, aceptar: true },
      opcDe(vecino)
    );

    expect(r.ok, 'contestarse a uno mismo sería la puerta trasera de `preguntar`').toBe(false);
  });

  it('la petición caduca a los 10 s SIN que nada la barra: se comprueba al leer', () => {
    const { sesion, ejercitoId, fundador, vecino } = ejercitoYViajero('preguntar');
    sesion.ejecutar(unirseEnCampo, { ejercitoId, jugadorId: vecino }, opcDe(vecino));
    // Se avanza el reloj sin ejecutar ningún barrido: un tick es 60 s, de sobra para pasarse de los 10.
    expect(MOVIMIENTO.vidaPeticionUnionSegundos * 1000).toBeLessThan(SIMULACION.duracionTickMs);
    sesion.avanzarTick();
    // La petición sigue EN el estado — nadie la borró — pero ya no sirve.
    expect(sesion.getState().ejercitos.find((e) => e.id === ejercitoId)!.peticionesDeUnion).toHaveLength(1);

    const r = sesion.ejecutar(
      responderPeticionDeUnion,
      { ejercitoId, jugadorId: fundador, solicitanteId: vecino, aceptar: true },
      opcDe(fundador)
    );

    expect(r.ok, 'el silencio cuenta como un no').toBe(false);
  });
});

describe('separarseDelEjercito — devuelve la libertad (Doc 5.14.2)', () => {
  /** Ejército de dos: el fundador es el Líder, el vecino se unió por el camino. */
  function ejercitoDeDos() {
    const base = ejercitoYViajero('aceptar');
    base.sesion.ejecutar(unirseEnCampo, { ejercitoId: base.ejercitoId, jugadorId: base.vecino }, opcDe(base.vecino));
    return base;
  }

  it('el que se separa sale con lo suyo y recupera el rumbo libre', () => {
    const { sesion, vecino } = ejercitoDeDos();

    const r = sesion.ejecutar(separarseDelEjercito, { jugadorId: vecino }, opcDe(vecino));

    expect(r.ok).toBe(true);
    const columna = sesion.getState().ejercitos.find((e) => e.id === r.datos!.columnaId)!;
    expect(columna.tipo).toBe('personal');
    expect(columna.escuadrones.map((e) => e.id)).toEqual(['esc-vecino']);
    expect(sesion.getState().jugadores.find((j) => j.id === vecino)!.ubicacion).toEqual({ tipo: 'columna', ejercitoId: columna.id });
    // Y ya puede volver a girar.
    expect(sesion.ejecutar(marcharA, { jugadorId: vecino, objetivo: { tipo: 'punto', punto: { x: 420, y: 420 } } }, opcDe(vecino)).ok).toBe(true);
  });

  it('el origen NO cambia: hereda el del ejército, no el suyo', () => {
    const { sesion, vecino, asentamientoId } = ejercitoDeDos();

    const r = sesion.ejecutar(separarseDelEjercito, { jugadorId: vecino }, opcDe(vecino));

    const columna = sesion.getState().ejercitos.find((e) => e.id === r.datos!.columnaId)!;
    expect(columna.origenAsentamientoId).toBe(asentamientoId);
  });

  it('el LÍDER no se separa', () => {
    const { sesion, fundador } = ejercitoDeDos();

    const r = sesion.ejecutar(separarseDelEjercito, { jugadorId: fundador }, opcDe(fundador));

    expect(r.ok, 'para irse tiene que ceder antes el liderazgo').toBe(false);
  });

  it('el ÚLTIMO tampoco, aunque sea el Líder: la columna no se vacía en campo abierto', () => {
    const { sesion, fundador, vecino } = ejercitoDeDos();
    sesion.ejecutar(cederLiderazgo, { ejercitoId: sesion.getState().ejercitos[0]!.id, jugadorId: fundador, sucesorId: vecino }, opcDe(fundador));
    sesion.ejecutar(separarseDelEjercito, { jugadorId: fundador }, opcDe(fundador));
    const ejercito = sesion.getState().ejercitos.find((e) => e.tipo === 'ejercito')!;
    expect(ejercito.participantes).toHaveLength(1);

    const r = sesion.ejecutar(separarseDelEjercito, { jugadorId: vecino }, opcDe(vecino));

    expect(r.ok, 'su salida es cancelar y volver, no irse dejando la columna tirada').toBe(false);
  });
});

describe('cederLiderazgo — el único camino para que el Líder se vaya (Doc 5.14.3)', () => {
  function ejercitoDeDos() {
    const base = ejercitoYViajero('aceptar');
    base.sesion.ejecutar(unirseEnCampo, { ejercitoId: base.ejercitoId, jugadorId: base.vecino }, opcDe(base.vecino));
    return base;
  }

  it('cede, y entonces el anterior Líder ya puede separarse', () => {
    const { sesion, ejercitoId, fundador, vecino } = ejercitoDeDos();

    const cesion = sesion.ejecutar(cederLiderazgo, { ejercitoId, jugadorId: fundador, sucesorId: vecino }, opcDe(fundador));

    expect(cesion.ok).toBe(true);
    expect(sesion.getState().ejercitos.find((e) => e.id === ejercitoId)!.liderId).toBe(vecino);
    expect(sesion.ejecutar(separarseDelEjercito, { jugadorId: fundador }, opcDe(fundador)).ok).toBe(true);
  });

  it('no lo cede quien no es Líder', () => {
    const { sesion, ejercitoId, fundador, vecino } = ejercitoDeDos();

    const r = sesion.ejecutar(cederLiderazgo, { ejercitoId, jugadorId: vecino, sucesorId: fundador }, opcDe(vecino));

    expect(r.ok).toBe(false);
  });

  it('el sucesor tiene que ir dentro de la columna', () => {
    const { sesion, ejercitoId, fundador } = ejercitoDeDos();

    const r = sesion.ejecutar(cederLiderazgo, { ejercitoId, jugadorId: fundador, sucesorId: 'alguien-de-fuera' }, opcDe(fundador));

    expect(r.ok).toBe(false);
  });
});

describe('replegarEjercito — cancelar es del Líder (Doc 5.14.3)', () => {
  it('un integrante que no manda no puede hacer volver a todos', () => {
    const base = ejercitoYViajero('aceptar');
    base.sesion.ejecutar(unirseEnCampo, { ejercitoId: base.ejercitoId, jugadorId: base.vecino }, opcDe(base.vecino));

    const r = base.sesion.ejecutar(replegarEjercito, { ejercitoId: base.ejercitoId }, opcDe(base.vecino));

    expect(r.ok, 'el rumbo lo acordaron varios; el que no quiera seguir se separa').toBe(false);
    expect(base.sesion.getState().ejercitos[0]!.estado).not.toBe('regresando');
  });

  it('y el Líder sí', () => {
    const base = ejercitoYViajero('aceptar');
    base.sesion.ejecutar(unirseEnCampo, { ejercitoId: base.ejercitoId, jugadorId: base.vecino }, opcDe(base.vecino));

    const r = base.sesion.ejecutar(replegarEjercito, { ejercitoId: base.ejercitoId }, opcDe(base.fundador));

    expect(r.ok).toBe(true);
    expect(base.sesion.getState().ejercitos[0]!.estado).toBe('regresando');
  });
});

// El menu de interaccion (paso 8, Doc 5.12.3). Lo que congela este bloque es que mirar CUESTA: hay que
// meterse en el anillo de 40 y el observado se entera.
describe('inspeccionar — la informacion se compra acercandose', () => {
  it('desde el anillo de 40 devuelve la composicion de la columna ajena', () => {
    const { sesion, ejercitoId, vecino, fundador } = ejercitoYViajero('rechazar');

    const r = sesion.ejecutar(inspeccionar, { jugadorId: vecino, objetivo: { tipo: 'ejercito', id: ejercitoId } }, opcDe(vecino));

    expect(r.ok).toBe(true);
    const composicion = r.datos as { jugadoresIds: string[]; escuadrones: { tropaId: string; jugadorId: string }[] };
    expect(composicion.jugadoresIds, 'se ve de quien es').toEqual([fundador]);
    expect(composicion.escuadrones.map((e) => e.tropaId), 'y con que tropas va').toEqual(['milicia_lanceros']);
  });

  it('y el observado RECIBE AVISO: mirar te delata', () => {
    const { sesion, ejercitoId, vecino } = ejercitoYViajero('rechazar');

    const r = sesion.ejecutar(inspeccionar, { jugadorId: vecino, objetivo: { tipo: 'ejercito', id: ejercitoId } }, opcDe(vecino));

    const aviso = r.eventos!.find((e) => e.codigo === 'columna.observada');
    expect(aviso, 'sin aviso, inspeccionar seria telemetria gratis').toBeDefined();
    expect((aviso!.payload as { observadorId: string }).observadorId).toBe(vecino);
  });

  it('desde MAS LEJOS de 40 no se puede: hay que acercarse de verdad', () => {
    const { sesion, ejercitoId, columnaId, vecino } = ejercitoYViajero('rechazar');
    const payload = sesion.exportar();
    const lejos = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        ejercitos: payload.state.ejercitos.map((e) =>
          e.id === columnaId ? { ...e, posicionActual: { x: e.posicionActual.x + MOVIMIENTO.radioInspeccion + 1, y: e.posicionActual.y } } : e
        ),
      },
    });

    const r = lejos.ejecutar(inspeccionar, { jugadorId: vecino, objetivo: { tipo: 'ejercito', id: ejercitoId } }, opcDe(vecino));

    expect(r.ok).toBe(false);
  });

  it('y ese anillo esta ENTRE ver y chocar, que es lo que lo hace un juego de dos', () => {
    expect(MOVIMIENTO.radioInspeccion).toBeGreaterThan(LOGISTICA.radioEncuentro);
    expect(MOVIMIENTO.radioInspeccion).toBeLessThan(VISION.ejercito);
  });
});
