// Verifica `verificarAutorizacion` sobre la matriz real (`comandos/autorizacion.ts`) con estado de partida
// GENUINO (`partidaConAsentamiento`, la misma fixture que el resto de tests de comandos) en vez de mocks.
//
// Eso importa más desde la revisión de separación negocio/infraestructura (2026-08-25): la pertenencia a una
// Facción ya no es un campo que el test pueda declarar por su cuenta, se DERIVA de `Faccion.ciudadanosIds`.
// Un test que quiera un actor con Facción tiene que fundarla y ganarse la ciudadanía como en el juego real,
// así que estas pruebas fallan si cambia el significado de ciudadanía, residencia o cargo en `engine/`.
import { describe, expect, it } from 'vitest';
import { escuadronDePrueba } from '../../engine/__tests__/fixtures';
import { instanteDeTick } from '../estado';
import { GameSession } from '../gameSession';
import { conHeroe, partidaConAsentamiento, ACTOR, OPC } from './fixtures';
import { crearFaccion } from '../comandos/crearFaccion';
import { fundarAsentamiento } from '../comandos/fundarAsentamiento';
import { asignarCargoLocal, asignarRey } from '../comandos/cargos';
import { entrarEnAsentamiento, salirAlMundo } from '../comandos/presencia';
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

describe('filtro de rol técnico', () => {
  it('un rol no listado se rechaza sin llegar a evaluar la condición de dominio', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId },
      sesion.getState(),
      { rol: 'observador', heroeId: null }
    );
    expect(resultado).toEqual({ autorizado: false, motivo: 'rol_insuficiente' });
  });

  it('el rol jugador sin heroeId se deniega: no hay a quién atribuir la acción', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId },
      sesion.getState(),
      { rol: 'jugador', heroeId: null }
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });
});

describe('la Facción del actor se deriva de ciudadanosIds, no de la membresía', () => {
  it('autoriza a un ciudadano de esa Facción', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId },
      sesion.getState(),
      jugador(fundador)
    );
    expect(resultado).toEqual(AUTORIZADO);
  });

  it('rechaza a un forastero: fundar consume el cap de fundación de la Facción', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId },
      sesion.getState(),
      jugador('forastero')
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });

  it('ARRANQUE: en una Facción recién creada, sin ciudadanos, sí puede fundar quien no tiene Facción', () => {
    // Sin esta excepción `crearFaccion` -> `fundarAsentamiento` sería imposible y toda Facción nacería
    // muerta: es fundar lo que otorga la primera ciudadanía.
    const sesion = GameSession.crear('arranque', { seed: 42 });
    const rf = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);

    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId: rf.datos!.faccionId },
      sesion.getState(),
      jugador(ACTOR)
    );
    expect(resultado).toEqual(AUTORIZADO);
  });

  it('...pero NO quien ya es ciudadano de otra Facción: un jugador pertenece solo a una', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const rf = sesion.ejecutar(crearFaccion, { nombre: 'Vacia' }, { ...OPC, actor: 'otro' });

    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId: rf.datos!.faccionId },
      sesion.getState(),
      jugador(fundador)
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });

  it('un ciudadano de la Facción rival tampoco pasa', () => {
    const { sesion, faccionId, ciudadanoRival } = partidaConFaccionRival();

    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId },
      sesion.getState(),
      jugador(ciudadanoRival)
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });
});

describe('comprarCasa', () => {
  it('rechaza comprar en nombre de otro jugador', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion('comprarCasa', { asentamientoId, heroeId: 'otro' }, sesion.getState(), jugador('yo'));
    expect(resultado).toEqual(POR_DOMINIO);
  });

  it('autoriza a quien no es ciudadano de ninguna Facción todavía: comprar casa es una vía de unirse', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion('comprarCasa', { asentamientoId, heroeId: 'recien-llegado' }, sesion.getState(), jugador('recien-llegado'));
    expect(resultado).toEqual(AUTORIZADO);
  });

  it('rechaza a un ciudadano de otra Facción (un jugador solo pertenece a una)', () => {
    const { sesion, asentamientoRivalId, fundador } = partidaConFaccionRival();
    const resultado = verificarAutorizacion(
      'comprarCasa',
      { asentamientoId: asentamientoRivalId, heroeId: fundador },
      sesion.getState(),
      jugador(fundador)
    );
    expect(resultado).toEqual(POR_DOMINIO);
  });
});

describe('la puerta (Doc 1.10.5)', () => {
  // Quien puede tocarla es autorización, no regla del comando: la matriz es la única verdad sobre quién
  // puede ejecutar qué, y repetir el chequeo dentro del comando dejaría dos que se desincronizan.
  it('fijar la política de acceso exige ser el Gobernador', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const params = { asentamientoId, heroeId: fundador, politica: 'abierto' as const };

    expect(verificarAutorizacion('fijarPoliticaDeAcceso', params, sesion.getState(), jugador(fundador))).toEqual(POR_DOMINIO);

    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
    expect(verificarAutorizacion('fijarPoliticaDeAcceso', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });

  it('vetar también', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    const params = { asentamientoId, heroeId: fundador, vetadoId: vecino, vetar: true };

    expect(verificarAutorizacion('vetarJugador', params, sesion.getState(), jugador(fundador))).toEqual(POR_DOMINIO);

    sesion.ejecutar(asignarCargoLocal, { asentamientoId, cargo: 'gobernador', heroeId: fundador }, OPC);
    expect(verificarAutorizacion('vetarJugador', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
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

describe('héroe y Facciones NPC', () => {
  it('sin héroe, un jugador solo puede crearlo (doc 02 §4.2)', () => {
    const { sesion } = partidaConAsentamiento();
    const sinHeroe: ActorDeComando = { rol: 'jugador', heroeId: null };
    const heroe = { displayName: 'Ana', classDefinitionId: 'Spear', genero: 'femenino' as const, avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' } };

    expect(verificarAutorizacion('crearHeroe', heroe, sesion.getState(), sinHeroe)).toEqual(AUTORIZADO);
    expect(verificarAutorizacion('crearFaccion', { nombre: 'Troya' }, sesion.getState(), sinHeroe)).toEqual(POR_DOMINIO);
  });

  it('crearFaccionNpc es solo de administración: ningún jugador, ni siendo Rey', () => {
    const { sesion, fundador } = partidaConAsentamiento();
    const params = { nombre: 'Tirinto' };

    expect(verificarAutorizacion('crearFaccionNpc', params, sesion.getState(), { rol: 'administrador_partida', heroeId: null })).toEqual(AUTORIZADO);
    expect(verificarAutorizacion('crearFaccionNpc', params, sesion.getState(), jugador(fundador))).toEqual({
      autorizado: false,
      motivo: 'rol_insuficiente',
    });
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

describe('combate: residente del atacante Y dueño de los escuadrones comprometidos', () => {
  /** Estado con dos escuadrones en el campamento de la fixture: uno del fundador, otro del vecino. */
  function conEscuadrones(sesion: ReturnType<typeof partidaConAsentamiento>['sesion'], fundador: string, vecino: string) {
    const estado = sesion.getState();
    const suyas: Record<string, string> = { [fundador]: 'esc-fundador', [vecino]: 'esc-vecino' };
    return {
      ...estado,
      heroes: estado.heroes.map((h) => (suyas[h.id] ? { ...h, escuadrones: [escuadronDePrueba(suyas[h.id]!, h.id, 't1')] } : h)),
    };
  }

  it('iniciarAsedio: el residente puede comprometer SU escuadrón, no el de un co-residente', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    const estado = conEscuadrones(sesion, fundador, vecino);

    expect(
      verificarAutorizacion('iniciarAsedio', { atacanteId: asentamientoId, defensorId: 'x', escuadronIds: ['esc-fundador'] }, estado, jugador(fundador))
    ).toEqual(AUTORIZADO);
    expect(
      verificarAutorizacion('iniciarAsedio', { atacanteId: asentamientoId, defensorId: 'x', escuadronIds: ['esc-vecino'] }, estado, jugador(fundador))
    ).toEqual(POR_DOMINIO);
    expect(
      verificarAutorizacion(
        'iniciarAsedio',
        { atacanteId: asentamientoId, defensorId: 'x', escuadronIds: ['esc-fundador', 'esc-vecino'] },
        estado,
        jugador(fundador)
      )
    ).toEqual(POR_DOMINIO);
  });

  it('un escuadronId inexistente se deja pasar: lo rechaza el comando, no la autorización', () => {
    const { sesion, asentamientoId, fundador, vecino } = partidaConAsentamiento();
    const estado = conEscuadrones(sesion, fundador, vecino);
    expect(
      verificarAutorizacion('atacarCampamentoBandidos', { atacanteId: asentamientoId, campamentoId: 'c', escuadronIds: ['no-existe'] }, estado, jugador(fundador))
    ).toEqual(AUTORIZADO);
  });

  // El caso de `combateCampoAbierto` ("solo se exige propiedad en el lado donde el actor reside") se fue con
  // el comando en el Paso 11. La regla que probaba —que un jugador solo dispone de SUS escuadrones— no se ha
  // perdido: vive en `movilizarEjercito`, que rechaza sacar un escuadrón de otro (`seleccionarParaCampana`,
  // engine/ejercitos.ts), y se comprueba en `comandosEjercitos.test.ts`.
});

describe('diplomacia: ciudadanía + autoridad de Rey/Embajador', () => {
  function conRelacion(sesion: ReturnType<typeof partidaConAsentamiento>['sesion'], tipo: 'alianza' | 'vasallaje', aId: string, bId: string) {
    return {
      ...sesion.getState(),
      relaciones: [{ id: 'r1', tipo, faccionAId: aId, faccionBId: bId, creadoEn: instanteDeTick(0), estado: 'activa' as const }],
    };
  }

  it('romperRelacion exige ser Rey/Embajador de la Facción iniciadora', () => {
    const { sesion, faccionId, faccionRivalId, fundador, vecino } = partidaConFaccionRival();
    const params = { relacionId: 'r1', iniciadorFaccionId: faccionId };

    // vecino es ciudadano de la Facción iniciadora pero ni Rey ni Embajador; el fundador es su Rey.
    expect(verificarAutorizacion('romperRelacion', params, conRelacion(sesion, 'alianza', faccionId, faccionRivalId), jugador(vecino))).toEqual(POR_DOMINIO);
    expect(verificarAutorizacion('romperRelacion', params, conRelacion(sesion, 'alianza', faccionId, faccionRivalId), jugador(fundador))).toEqual(AUTORIZADO);
  });

  it('rebelionVasallo la ejerce el VASALLO: el Rey de la Facción señora no puede', () => {
    const { sesion, faccionId, faccionRivalId, fundador } = partidaConFaccionRival();
    sesion.ejecutar(asignarRey, { faccionId, heroeId: fundador }, OPC);

    // faccionId es la señora (faccionAId) y faccionRivalId la vasalla: el fundador manda en la señora.
    const resultado = verificarAutorizacion('rebelionVasallo', { relacionId: 'r1' }, conRelacion(sesion, 'vasallaje', faccionId, faccionRivalId), jugador(fundador));
    expect(resultado).toEqual(POR_DOMINIO);
  });

  it('anexionar exige autoridad en la Facción absorbente', () => {
    const { sesion, faccionId, faccionRivalId, fundador, vecino } = partidaConFaccionRival();
    const params = { faccionAId: faccionId, faccionBId: faccionRivalId };

    expect(verificarAutorizacion('anexionar', params, sesion.getState(), jugador(vecino))).toEqual(POR_DOMINIO);
    expect(verificarAutorizacion('anexionar', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });
});

// La presencia (paso 7, Doc 2.5: "la ciudadania habilita, la presencia ejerce"). Hasta aqui un jugador
// estaba en todas partes a la vez y le bastaba ser vecino; ahora ademas tiene que estar ahi.
describe('presencia: ser vecino ya no basta, hay que estar dentro', () => {
  /** Saca al fundador de su plaza y devuelve la sesion con el ya en el mapa. */
  function deCampana() {
    const base = partidaConAsentamiento();
    const r = base.sesion.ejecutar(
      salirAlMundo,
      { asentamientoId: base.asentamientoId, heroeId: base.fundador, escuadronIds: [], carga: {} },
      { ...OPC, actor: base.fundador }
    );
    expect(r.ok, 'setup del test: tiene que poder salir').toBe(true);
    return base;
  }

  it('dentro de su plaza, un vecino puede reclutar', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const params = { asentamientoId, heroeId: fundador, tropaId: 'milicia_lanceros', cantidad: 1, origen: 'pesants' as const };

    expect(verificarAutorizacion('reclutarTropa', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });

  it('en campaña NO, aunque siga siendo vecino de esa misma plaza', () => {
    const { sesion, asentamientoId, fundador } = deCampana();
    const params = { asentamientoId, heroeId: fundador, tropaId: 'milicia_lanceros', cantidad: 1, origen: 'pesants' as const };

    expect(verificarAutorizacion('reclutarTropa', params, sesion.getState(), jugador(fundador))).toEqual(POR_DOMINIO);
  });

  it('un GOBERNADOR de campaña sigue siendo Gobernador, pero no gobierna desde el camino', () => {
    // Es la consecuencia buscada de Doc 2.5: no pierde el cargo, pierde la capacidad de dar ordenes nuevas.
    // Y por eso la delegacion pasa a importar.
    const base = partidaConAsentamiento();
    base.sesion.ejecutar(asignarCargoLocal, { asentamientoId: base.asentamientoId, cargo: 'gobernador', heroeId: base.fundador }, OPC);
    const params = { asentamientoId: base.asentamientoId, heroeId: base.fundador, cargo: 'tesorero' as const };
    expect(verificarAutorizacion('asignarCargoLocal', params, base.sesion.getState(), jugador(base.fundador))).toEqual(AUTORIZADO);

    base.sesion.ejecutar(
      salirAlMundo,
      { asentamientoId: base.asentamientoId, heroeId: base.fundador, escuadronIds: [], carga: {} },
      { ...OPC, actor: base.fundador }
    );

    const enCampana = base.sesion.getState();
    expect(enCampana.asentamientos[0]!.cargos.gobernadorId, 'el cargo NO se pierde').toBe(base.fundador);
    expect(verificarAutorizacion('asignarCargoLocal', params, enCampana, jugador(base.fundador))).toEqual(POR_DOMINIO);
  });

  it('y al volver a entrar lo recupera', () => {
    const { sesion, asentamientoId, fundador } = deCampana();
    sesion.ejecutar(entrarEnAsentamiento, { asentamientoId, heroeId: fundador }, { ...OPC, actor: fundador });
    const params = { asentamientoId, heroeId: fundador, tropaId: 'milicia_lanceros', cantidad: 1, origen: 'pesants' as const };

    expect(verificarAutorizacion('reclutarTropa', params, sesion.getState(), jugador(fundador))).toEqual(AUTORIZADO);
  });

  it('un residente que todavía no ha dado ninguna orden está en su residencia y puede actuar en ella', () => {
    const { sesion, asentamientoId, vecino } = partidaConAsentamiento();
    const params = { asentamientoId, heroeId: vecino, tropaId: 'milicia_lanceros', cantidad: 1, origen: 'pesants' as const };

    expect(verificarAutorizacion('reclutarTropa', params, sesion.getState(), jugador(vecino))).toEqual(AUTORIZADO);
  });
});
