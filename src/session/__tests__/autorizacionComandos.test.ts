// Verifica `verificarAutorizacion` sobre la matriz real (`comandos/autorizacion.ts`) con estado de partida
// GENUINO (`partidaConAsentamiento`, la misma fixture que el resto de tests de comandos) en vez de mocks.
//
// Eso importa más desde la revisión de separación negocio/infraestructura (2026-08-25): la pertenencia a una
// Facción ya no es un campo que el test pueda declarar por su cuenta, se DERIVA de `Faccion.ciudadanosIds`.
// Un test que quiera un actor con Facción tiene que fundarla y ganarse la ciudadanía como en el juego real,
// así que estas pruebas fallan si cambia el significado de ciudadanía, residencia o cargo en `engine/`.
import { describe, expect, it } from 'vitest';
import { partidaConAsentamiento, OPC } from './fixtures';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { asignarCargoLocal, asignarRey } from '../comandos/cargos';
import { REGISTRO_COMANDOS } from '../comandos/registro';
import { MATRIZ_AUTORIZACION, verificarAutorizacion, type ActorDeComando } from '../comandos/autorizacion';

const AUTORIZADO = { autorizado: true };
const POR_DOMINIO = { autorizado: false, motivo: 'condicion_dominio' };

function jugador(jugadorId: string): ActorDeComando {
  return { rol: 'jugador', jugadorId };
}

/** Segundo fundador del asentamiento: ciudadano y residente igual que el primero, pero sin ningún cargo —
 * el actor que distingue "reside" de "manda". */
function segundoFundador(sesion: ReturnType<typeof partidaConAsentamiento>['sesion']): string {
  return sesion.getState().asentamientos[0]!.jugadoresFundadoresIds[1]!;
}

/** Partida con una segunda Facción rival, con su propio asentamiento y ciudadanos. */
function partidaConFaccionRival() {
  const base = partidaConAsentamiento();
  const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, OPC);
  const faccionRivalId = rf.datos!.faccionId;
  const ra = base.sesion.ejecutar(fundarAsentamiento, { faccionId: faccionRivalId, posicion: { x: 900, y: 900 }, numJugadores: 1 }, OPC);
  return { ...base, faccionRivalId, asentamientoRivalId: ra.datos!.asentamientoId };
}

describe('exhaustividad de la matriz', () => {
  it('tiene exactamente una fila por cada comando del registro real', () => {
    expect(Object.keys(MATRIZ_AUTORIZACION).sort()).toEqual(Object.keys(REGISTRO_COMANDOS).sort());
  });
});

describe('filtro de rol técnico', () => {
  it('un rol no listado se rechaza sin llegar a evaluar la condición de dominio', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId, posicion: { x: 0, y: 0 }, numJugadores: 1 },
      sesion.getState(),
      { rol: 'observador', jugadorId: null }
    );
    expect(resultado).toEqual({ autorizado: false, motivo: 'rol_insuficiente' });
  });

  it('el rol jugador sin jugadorId se deniega: no hay a quién atribuir la acción', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId, posicion: { x: 0, y: 0 }, numJugadores: 1 },
      sesion.getState(),
      { rol: 'jugador', jugadorId: null }
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });
});

describe('la Facción del actor se deriva de ciudadanosIds, no de la membresía', () => {
  it('autoriza a un ciudadano de esa Facción', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId, posicion: { x: 0, y: 0 }, numJugadores: 1 },
      sesion.getState(),
      jugador(fundador)
    );
    expect(resultado).toEqual(AUTORIZADO);
  });

  it('rechaza a quien no es ciudadano de ella', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId, posicion: { x: 0, y: 0 }, numJugadores: 1 },
      sesion.getState(),
      jugador('forastero')
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });

  it('un ciudadano de la Facción rival tampoco pasa', () => {
    const { sesion, faccionId, asentamientoRivalId } = partidaConFaccionRival();
    const ciudadanoRival = sesion.getState().asentamientos.find((a) => a.id === asentamientoRivalId)!.jugadoresFundadoresIds[0]!;

    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId, posicion: { x: 0, y: 0 }, numJugadores: 1 },
      sesion.getState(),
      jugador(ciudadanoRival)
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });
});

describe('comprarCasa', () => {
  it('rechaza comprar en nombre de otro jugador', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion('comprarCasa', { asentamientoId, jugadorId: 'otro' }, sesion.getState(), jugador('yo'));
    expect(resultado).toEqual(POR_DOMINIO);
  });

  it('autoriza a quien no es ciudadano de ninguna Facción todavía: comprar casa es una vía de unirse', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion('comprarCasa', { asentamientoId, jugadorId: 'recien-llegado' }, sesion.getState(), jugador('recien-llegado'));
    expect(resultado).toEqual(AUTORIZADO);
  });

  it('rechaza a un ciudadano de otra Facción (un jugador solo pertenece a una)', () => {
    const { sesion, asentamientoRivalId, fundador } = partidaConFaccionRival();
    const resultado = verificarAutorizacion(
      'comprarCasa',
      { asentamientoId: asentamientoRivalId, jugadorId: fundador },
      sesion.getState(),
      jugador(fundador)
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });
});

describe('asignarCargoLocal', () => {
  it('designar Gobernador es directo para un residente', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'asignarCargoLocal',
      { asentamientoId, cargo: 'gobernador', jugadorId: fundador },
      sesion.getState(),
      jugador(fundador)
    );
    expect(resultado).toEqual(AUTORIZADO);
  });

  it('designar Tesorero exige ser el Gobernador vigente', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const params = { asentamientoId, cargo: 'tesorero' as const, jugadorId: fundador };

    expect(verificarAutorizacion('asignarCargoLocal', params, sesion.getState(), jugador(fundador))).toEqual(POR_DOMINIO);

    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', jugadorId: fundador }, OPC);
    expect(verificarAutorizacion('asignarCargoLocal', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });

  it('otro residente sin cargo no puede designar Tesorero aunque ya haya Gobernador', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', jugadorId: fundador }, OPC);

    const resultado = verificarAutorizacion(
      'asignarCargoLocal',
      { asentamientoId, cargo: 'tesorero', jugadorId: fundador },
      sesion.getState(),
      jugador(segundoFundador(sesion))
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });
});

describe('el cargo local se comprueba sobre el titular, no sobre si el puesto está cubierto', () => {
  it('calibrarReservaManual: el Tesorero sí, otro residente no, aunque el cargo esté ocupado', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const otro = segundoFundador(sesion);
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', jugadorId: fundador }, OPC);
    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'tesorero', jugadorId: fundador }, OPC);
    const params = { asentamientoId, recurso: 'trigo' as const, valor: 10 };

    expect(verificarAutorizacion('calibrarReservaManual', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
    expect(verificarAutorizacion('calibrarReservaManual', params, sesion.getState(), jugador(otro))).toEqual(POR_DOMINIO);
  });
});

describe('alternarFaccionNpc', () => {
  it('administrador_partida no evalúa condición de dominio', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'alternarFaccionNpc',
      { faccionId, activo: true },
      sesion.getState(),
      { rol: 'administrador_partida', jugadorId: null }
    );
    expect(resultado).toEqual(AUTORIZADO);
  });

  it('un ciudadano que no es Rey se rechaza; el Rey pasa', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const params = { faccionId, activo: true };

    expect(verificarAutorizacion('alternarFaccionNpc', params, sesion.getState(), jugador(fundador))).toEqual(POR_DOMINIO);

    sesion.ejecutar(asignarRey, { faccionId, jugadorId: fundador }, OPC);
    expect(verificarAutorizacion('alternarFaccionNpc', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });
});

describe('residencia en el asentamiento objetivo', () => {
  it('colocarOrdenMercado rechaza a quien no reside ahí', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'colocarOrdenMercado',
      { asentamientoId, tipo: 'venta', recurso: 'trigo', cantidad: 1 },
      sesion.getState(),
      jugador('forastero')
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });

  it('reclutarTropa rechaza reclutar a nombre de otro, aunque ambos residan ahí', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'reclutarTropa',
      { asentamientoId, jugadorId: fundador, tropaId: 'x', origen: 'pesants' },
      sesion.getState(),
      jugador(segundoFundador(sesion))
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });
});

describe('diplomacia: ciudadanía + autoridad de Rey/Embajador', () => {
  function conRelacion(sesion: ReturnType<typeof partidaConAsentamiento>['sesion'], tipo: 'alianza' | 'vasallaje', aId: string, bId: string) {
    return {
      ...sesion.getState(),
      relaciones: [{ id: 'r1', tipo, faccionAId: aId, faccionBId: bId, creadoEnTick: 0, estado: 'activa' as const }],
    };
  }

  it('romperRelacion exige ser Rey/Embajador de la Facción iniciadora', () => {
    const { sesion, faccionId, faccionRivalId, fundador } = partidaConFaccionRival();
    const params = { relacionId: 'r1', iniciadorFaccionId: faccionId };

    expect(verificarAutorizacion('romperRelacion', params, conRelacion(sesion, 'alianza', faccionId, faccionRivalId), jugador(fundador))).toEqual(POR_DOMINIO);

    sesion.ejecutar(asignarRey, { faccionId, jugadorId: fundador }, OPC);
    expect(verificarAutorizacion('romperRelacion', params, conRelacion(sesion, 'alianza', faccionId, faccionRivalId), jugador(fundador))).toEqual(AUTORIZADO);
  });

  it('rebelionVasallo la ejerce el VASALLO: el Rey de la Facción señora no puede', () => {
    const { sesion, faccionId, faccionRivalId, fundador } = partidaConFaccionRival();
    sesion.ejecutar(asignarRey, { faccionId, jugadorId: fundador }, OPC);

    // faccionId es la señora (faccionAId) y faccionRivalId la vasalla: el fundador manda en la señora.
    const resultado = verificarAutorizacion('rebelionVasallo', { relacionId: 'r1' }, conRelacion(sesion, 'vasallaje', faccionId, faccionRivalId), jugador(fundador));
    expect(resultado).toEqual(POR_DOMINIO);
  });

  it('anexionar exige autoridad en la Facción absorbente', () => {
    const { sesion, faccionId, faccionRivalId, fundador } = partidaConFaccionRival();
    const params = { faccionAId: faccionId, faccionBId: faccionRivalId };

    expect(verificarAutorizacion('anexionar', params, sesion.getState(), jugador(fundador))).toEqual(POR_DOMINIO);

    sesion.ejecutar(asignarRey, { faccionId, jugadorId: fundador }, OPC);
    expect(verificarAutorizacion('anexionar', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });
});
