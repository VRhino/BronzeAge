// Verifica `verificarAutorizacion` sobre la matriz real (`comandos/autorizacion.ts`) con estado de partida
// GENUINO (`partidaConAsentamiento`, la misma fixture que el resto de tests de comandos) en vez de mocks.
//
// Eso importa más desde la revisión de separación negocio/infraestructura (2026-08-25): la pertenencia a una
// Facción ya no es un campo que el test pueda declarar por su cuenta, se DERIVA de `Faccion.ciudadanosIds`.
// Un test que quiera un actor con Facción tiene que fundarla y ganarse la ciudadanía como en el juego real,
// así que estas pruebas fallan si cambia el significado de ciudadanía, residencia o cargo en `engine/`.
import { describe, expect, it } from 'vitest';
import { conHeroe, partidaConAsentamiento, OPC } from './fixtures';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from './fundarDePrueba';
import { asignarCargoLocal, asignarRey } from '../comandos/cargos';
import { salirAlMundo } from '../comandos/presencia';
import { REGISTRO_COMANDOS } from '../comandos/registro';
import { MATRIZ_AUTORIZACION, verificarAutorizacion, type ActorDeComando } from '../comandos/autorizacion';

const AUTORIZADO = { autorizado: true };
const POR_DOMINIO = { autorizado: false, motivo: 'condicion_dominio' };

function jugador(heroeId: string): ActorDeComando {
  return { rol: 'jugador', heroeId };
}

/**
 * Partida con una segunda Facción rival, con su propio asentamiento y su propio ciudadano.
 *
 * La funda OTRO actor (`ciudadanoRival`), no el de `OPC`: desde que funda quien ejecuta el comando, usar el
 * mismo actor para las dos Facciones lo haría ciudadano de ambas — un estado que el juego no admite (Doc 0:
 * un jugador, una Facción) y que dejaría sin sentido cualquier prueba sobre "Facción ajena".
 */
const CIUDADANO_RIVAL = 'jugador-troyano';

function partidaConFaccionRival() {
  const base = partidaConAsentamiento();
  base.sesion = conHeroe(base.sesion, CIUDADANO_RIVAL);
  const opcRival = { ...OPC, actor: CIUDADANO_RIVAL };
  const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, opcRival);
  const faccionRivalId = rf.datos!.faccionId;
  const ra = base.sesion.ejecutar(fundarAsentamiento, { faccionId: faccionRivalId }, opcRival);
  return { ...base, faccionRivalId, asentamientoRivalId: ra.datos!.asentamientoId, ciudadanoRival: CIUDADANO_RIVAL };
}

describe('exhaustividad de la matriz', () => {
  it('tiene exactamente una fila por cada comando del registro real', () => {
    expect(Object.keys(MATRIZ_AUTORIZACION).sort()).toEqual(Object.keys(REGISTRO_COMANDOS).sort());
  });
});

/** Lanzar la Caravana de Fundación de una plaza: la condición de dominio es residir en ella y estar dentro. */
const lanzarDesde = (origenAsentamientoId: string) => ({ origenAsentamientoId });

describe('filtro de rol técnico', () => {
  it('un rol no listado se rechaza sin llegar a evaluar la condición de dominio', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion('lanzarCaravanaFundacion', lanzarDesde(asentamientoId), sesion.getState(), { rol: 'moderador', heroeId: null });
    expect(resultado).toEqual({ autorizado: false, motivo: 'rol_insuficiente' });
  });

  it('el rol jugador sin heroeId se deniega: no hay a quién atribuir la acción', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion('lanzarCaravanaFundacion', lanzarDesde(asentamientoId), sesion.getState(), { rol: 'jugador', heroeId: null });
    expect(resultado).toEqual(POR_DOMINIO);
  });
});

describe('la Facción del actor se deriva de ciudadanosIds, no de la membresía', () => {
  it('autoriza a un ciudadano de esa Facción', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    expect(verificarAutorizacion('lanzarCaravanaFundacion', lanzarDesde(asentamientoId), sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });

  it('rechaza a un forastero', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    expect(verificarAutorizacion('lanzarCaravanaFundacion', lanzarDesde(asentamientoId), sesion.getState(), jugador('forastero'))).toEqual(POR_DOMINIO);
  });

  it('un ciudadano de la Facción rival tampoco pasa', () => {
    const { sesion, asentamientoId, ciudadanoRival } = partidaConFaccionRival();
    expect(verificarAutorizacion('lanzarCaravanaFundacion', lanzarDesde(asentamientoId), sesion.getState(), jugador(ciudadanoRival))).toEqual(POR_DOMINIO);
  });
});

describe('responderSolicitud (D31, D46)', () => {
  it('solo el Rey de esa Facción responde a sus solicitudes', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const params = { faccionId, heroeId: 'recien-llegado', aceptar: true };
    expect(verificarAutorizacion('responderSolicitud', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
    expect(verificarAutorizacion('responderSolicitud', params, sesion.getState(), jugador('otro'))).toEqual(POR_DOMINIO);
  });
});

describe('la puerta (Doc 1.10.5)', () => {
  // Quien puede tocarla es autorización, no regla del comando: la matriz es la única verdad sobre quién
  // puede ejecutar qué, y repetir el chequeo dentro del comando dejaría dos que se desincronizan.
  it('fijar la puerta exige ser el Gobernador de la plaza o el Rey de su Facción', () => {
    const { sesion, faccionId, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    const params = (heroeId: string) => ({ asentamientoId, heroeId, cerradaA: [] });
    const autoriza = (heroeId: string, actor = heroeId) => verificarAutorizacion('fijarPuerta', params(heroeId), sesion.getState(), jugador(actor));

    // El fundador es el Rey: puede sin ser Gobernador. Un vecino cualquiera, no.
    expect(autoriza(fundador)).toEqual(AUTORIZADO);
    expect(autoriza(vecino)).toEqual(POR_DOMINIO);

    // El Gobernador de la plaza, aunque no sea el Rey.
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: vecino }, OPC);
    expect(autoriza(vecino)).toEqual(AUTORIZADO);

    // Con otro Rey, el fundador deja de poder; y nadie actúa en nombre de otro.
    sesion.ejecutar(asignarRey, { faccionId, heroeId: vecino }, OPC);
    expect(autoriza(fundador)).toEqual(POR_DOMINIO);
    expect(autoriza(vecino, fundador)).toEqual(POR_DOMINIO);
  });

  it('vetar también', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    const params = { asentamientoId, heroeId: fundador, vetadoId: vecino, vetar: true };

    expect(verificarAutorizacion('vetarJugador', params, sesion.getState(), jugador(fundador))).toEqual(POR_DOMINIO);

    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
    expect(verificarAutorizacion('vetarJugador', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });
});

describe('la capital (Doc 2.2)', () => {
  it('solo la designa el Rey de la Facción', () => {
    const { sesion, faccionId, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    const params = { faccionId, asentamientoId };

    expect(verificarAutorizacion('designarCapital', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
    expect(verificarAutorizacion('designarCapital', params, sesion.getState(), jugador(vecino))).toEqual(POR_DOMINIO);
  });
});

describe('admitir a otras Facciones en los ataques (Doc 2.2, 5.15.1b)', () => {
  it('solo lo decide el Rey de la Facción', () => {
    const { sesion, faccionId, fundador, vecino } = partidaConAsentamiento();
    const params = { faccionId, admitir: true };

    expect(verificarAutorizacion('admitirOtrasFacciones', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
    expect(verificarAutorizacion('admitirOtrasFacciones', params, sesion.getState(), jugador(vecino))).toEqual(POR_DOMINIO);
  });
});

describe('subida de nivel del asentamiento (Doc 4.5)', () => {
  it('solicitarAscenso es solo del Gobernador: ni el fundador sin cargo ni otro residente', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    const params = { asentamientoId };

    expect(verificarAutorizacion('solicitarAscenso', params, sesion.getState(), jugador(fundador))).toEqual(POR_DOMINIO);

    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
    expect(verificarAutorizacion('solicitarAscenso', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
    expect(verificarAutorizacion('solicitarAscenso', params, sesion.getState(), jugador(vecino))).toEqual(POR_DOMINIO);
  });
});

describe('asignarCargoLocal', () => {
  it('designar Gobernador es directo para un residente', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'asignarCargoLocal',
      { asentamientoId, cargo: 'gobernador', heroeId: fundador },
      sesion.getState(),
      jugador(fundador)
    );
    expect(resultado).toEqual(AUTORIZADO);
  });

  it('designar Tesorero exige ser el Gobernador vigente', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const params = { asentamientoId, cargo: 'tesorero' as const, heroeId: fundador };

    expect(verificarAutorizacion('asignarCargoLocal', params, sesion.getState(), jugador(fundador))).toEqual(POR_DOMINIO);

    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
    expect(verificarAutorizacion('asignarCargoLocal', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });

  it('otro residente sin cargo no puede designar Tesorero aunque ya haya Gobernador', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);

    const resultado = verificarAutorizacion(
      'asignarCargoLocal',
      { asentamientoId, cargo: 'tesorero', heroeId: fundador },
      sesion.getState(),
      jugador(vecino)
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });
});

describe('el cargo local se comprueba sobre el titular, no sobre si el puesto está cubierto', () => {
  it('calibrarReservaManual: el Tesorero sí, otro residente no, aunque el cargo esté ocupado', () => {
    const { sesion, asentamientoId, fundador, vecino: otro } = partidaConAsentamiento();
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'tesorero', heroeId: fundador }, OPC);
    const params = { asentamientoId, recurso: 'trigo' as const, valor: 10 };

    expect(verificarAutorizacion('calibrarReservaManual', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
    expect(verificarAutorizacion('calibrarReservaManual', params, sesion.getState(), jugador(otro))).toEqual(POR_DOMINIO);
  });
});

describe('héroe sin héroe', () => {
  it('sin héroe, un jugador solo puede crearlo (doc 02 §4.2)', () => {
    const { sesion } = partidaConAsentamiento();
    const sinHeroe: ActorDeComando = { rol: 'jugador', heroeId: null };
    const heroe = { displayName: 'Ana', campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino' as const, avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' } };

    expect(verificarAutorizacion('crearHeroe', heroe, sesion.getState(), sinHeroe)).toEqual(AUTORIZADO);
    expect(verificarAutorizacion('crearFaccion', { nombre: 'Troya' }, sesion.getState(), sinHeroe)).toEqual(POR_DOMINIO);
  });
});

describe('ir en un ejército', () => {
  it('el viajero solo, sin escuadras, va en su columna: engancha y suelta caravanas (Doc 5.12.1)', () => {
    const { sesion, fundador, vecino, asentamientoId } = partidaConAsentamiento();
    sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, { ...OPC, actor: fundador });
    const ejercitoId = sesion.getState().ejercitos.find((e) => e.liderId === fundador)!.id;
    const params = { ejercitoId, caravanaId: 'caravana-x', heroeId: fundador };

    expect(verificarAutorizacion('adjuntarCaravana', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
    expect(verificarAutorizacion('adjuntarCaravana', { ...params, heroeId: vecino }, sesion.getState(), jugador(vecino))).toEqual(POR_DOMINIO);
  });
});

describe('comerciar es del Tesorero o del Gobernador (Doc 2.2, decisión del autor 2026-10-12)', () => {
  const trueque = (asentamientoId: string) => ({ asentamientoAId: asentamientoId, lineasA: [], asentamientoBId: 'otra', lineasB: [] });
  const orden = (asentamientoId: string) => ({ asentamientoId, tipo: 'venta' as const, recurso: 'trigo' as const, cantidad: 1 });

  it('un residente sin cargo no propone trueques ni coloca órdenes; sí lanza caravanas', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const estado = sesion.getState();
    expect(verificarAutorizacion('proponerTrueque', trueque(asentamientoId), estado, jugador(fundador))).toEqual(POR_DOMINIO);
    expect(verificarAutorizacion('colocarOrdenMercado', orden(asentamientoId), estado, jugador(fundador))).toEqual(POR_DOMINIO);
    expect(verificarAutorizacion('crearCaravana', { asentamientoId }, estado, jugador(fundador))).toEqual(AUTORIZADO);
  });

  it('el Tesorero y el Gobernador sí; otro residente sin cargo, no', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'tesorero', heroeId: vecino }, OPC);
    const estado = sesion.getState();
    for (const quien of [fundador, vecino]) {
      expect(verificarAutorizacion('proponerTrueque', trueque(asentamientoId), estado, jugador(quien))).toEqual(AUTORIZADO);
      expect(verificarAutorizacion('colocarOrdenMercado', orden(asentamientoId), estado, jugador(quien))).toEqual(AUTORIZADO);
    }
    expect(verificarAutorizacion('colocarOrdenMercado', orden(asentamientoId), estado, jugador('forastero'))).toEqual(POR_DOMINIO);
  });

  it('un acuerdo que no existe pasa la matriz: lo rechaza el comando', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    expect(verificarAutorizacion('aceptarTrueque', { acuerdoId: 'no-existe' }, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });
});

describe('residencia en el asentamiento objetivo', () => {
  it('crearCaravana rechaza a quien no reside ahí', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion('crearCaravana', { asentamientoId }, sesion.getState(), jugador('forastero'));
    expect(resultado).toEqual(POR_DOMINIO);
  });

  it('reclutarTropa rechaza reclutar a nombre de otro, aunque ambos residan ahí', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'reclutarTropa',
      { asentamientoId, heroeId: fundador, tropaId: 'x', origen: 'pesants' },
      sesion.getState(),
      jugador(vecino)
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });
});
