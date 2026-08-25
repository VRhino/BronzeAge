// Verifica `verificarAutorizacion` sobre la matriz real (`matriz.ts`), usando estado de partida GENUINO
// (`partidaConAsentamiento`, la misma fixture que usan los tests de `session/comandos/*`) en vez de mocks —
// así estas pruebas fallan de verdad si la forma de `Asentamiento`/`Faccion` cambia, no solo si cambia esta
// matriz.
import { describe, expect, it } from 'vitest';
import { partidaConAsentamiento, OPC } from '../../../session/__tests__/fixtures';
import { crearFaccion } from '../../../session/comandos/crearFaccion';
import { asignarCargoLocal as asignarCargoLocalComando, asignarRey } from '../../../session/comandos/cargos';
import { REGISTRO_COMANDOS } from '../../../session/comandos/registro';
import { MATRIZ_AUTORIZACION } from '../matriz';
import { verificarAutorizacion } from '../verificar';
import type { ActorDeComando } from '../matriz';

function actor(parcial: Partial<ActorDeComando>): ActorDeComando {
  return { rol: 'jugador', jugadorId: null, faccionId: null, ...parcial };
}

describe('exhaustividad de la matriz', () => {
  it('tiene exactamente una fila por cada comando del registro real', () => {
    expect(Object.keys(MATRIZ_AUTORIZACION).sort()).toEqual(Object.keys(REGISTRO_COMANDOS).sort());
  });
});

describe('filtro de rol técnico', () => {
  it('un rol no listado en rolesPermitidos se rechaza sin evaluar dominio', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId, posicion: { x: 0, y: 0 }, numJugadores: 1 },
      sesion.getState(),
      actor({ rol: 'observador' })
    );
    expect(resultado).toEqual({ autorizado: false, motivo: 'rol_insuficiente' });
  });
});

describe('fundarAsentamiento (Facción propia)', () => {
  it('autoriza cuando la Facción del actor coincide', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId, posicion: { x: 0, y: 0 }, numJugadores: 1 },
      sesion.getState(),
      actor({ jugadorId: 'j1', faccionId })
    );
    expect(resultado).toEqual({ autorizado: true });
  });

  it('rechaza cuando el actor pertenece a otra Facción', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'fundarAsentamiento',
      { faccionId, posicion: { x: 0, y: 0 }, numJugadores: 1 },
      sesion.getState(),
      actor({ jugadorId: 'j1', faccionId: 'otra-faccion' })
    );
    expect(resultado).toEqual({ autorizado: false, motivo: 'condicion_dominio' });
  });
});

describe('comprarCasa', () => {
  it('rechaza actuar en nombre de otro jugador', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'comprarCasa',
      { asentamientoId, jugadorId: 'otro' },
      sesion.getState(),
      actor({ jugadorId: 'yo', faccionId: null })
    );
    expect(resultado).toEqual({ autorizado: false, motivo: 'condicion_dominio' });
  });

  it('autoriza a un jugador SIN Facción todavía (comprar casa es una vía de unirse)', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'comprarCasa',
      { asentamientoId, jugadorId: 'yo' },
      sesion.getState(),
      actor({ jugadorId: 'yo', faccionId: null })
    );
    expect(resultado).toEqual({ autorizado: true });
  });

  it('rechaza comprar casa en un asentamiento de una Facción distinta a la del actor', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'comprarCasa',
      { asentamientoId, jugadorId: 'yo' },
      sesion.getState(),
      actor({ jugadorId: 'yo', faccionId: 'faccion-rival' })
    );
    expect(resultado).toEqual({ autorizado: false, motivo: 'condicion_dominio' });
  });
});

describe('asignarCargoLocal', () => {
  it('designar gobernador es directo para cualquier residente', () => {
    const { sesion, asentamientoId, faccionId, fundador } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'asignarCargoLocal',
      { asentamientoId, cargo: 'gobernador', jugadorId: fundador },
      sesion.getState(),
      actor({ jugadorId: fundador, faccionId })
    );
    expect(resultado).toEqual({ autorizado: true });
  });

  it('designar tesorero exige ser YA el gobernador vigente', () => {
    const { sesion, asentamientoId, faccionId, fundador } = partidaConAsentamiento();
    const antesDeSerGobernador = verificarAutorizacion(
      'asignarCargoLocal',
      { asentamientoId, cargo: 'tesorero', jugadorId: fundador },
      sesion.getState(),
      actor({ jugadorId: fundador, faccionId })
    );
    expect(antesDeSerGobernador).toEqual({ autorizado: false, motivo: 'condicion_dominio' });

    sesion.ejecutar(asignarCargoLocalComando, { asentamientoId, cargo: 'gobernador', jugadorId: fundador }, OPC);
    const comoGobernador = verificarAutorizacion(
      'asignarCargoLocal',
      { asentamientoId, cargo: 'tesorero', jugadorId: fundador },
      sesion.getState(),
      actor({ jugadorId: fundador, faccionId })
    );
    expect(comoGobernador).toEqual({ autorizado: true });
  });

  it('un residente que no es gobernador no puede designar tesorero', () => {
    const { sesion, asentamientoId, faccionId, fundador } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'asignarCargoLocal',
      { asentamientoId, cargo: 'tesorero', jugadorId: fundador },
      sesion.getState(),
      actor({ jugadorId: 'otro-residente', faccionId })
    );
    expect(resultado).toEqual({ autorizado: false, motivo: 'condicion_dominio' });
  });
});

describe('alternarFaccionNpc', () => {
  it('administrador_partida no necesita condición de dominio', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'alternarFaccionNpc',
      { faccionId, activo: true },
      sesion.getState(),
      actor({ rol: 'administrador_partida' })
    );
    expect(resultado).toEqual({ autorizado: true });
  });

  it('un jugador que no es Rey de la Facción se rechaza', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion('alternarFaccionNpc', { faccionId, activo: true }, sesion.getState(), actor({ jugadorId: 'j1', faccionId }));
    expect(resultado).toEqual({ autorizado: false, motivo: 'condicion_dominio' });
  });

  it('el Rey de la Facción propia sí puede', () => {
    const { sesion, faccionId, fundador } = partidaConAsentamiento();
    sesion.ejecutar(asignarRey, { faccionId, jugadorId: fundador }, OPC);
    const resultado = verificarAutorizacion(
      'alternarFaccionNpc',
      { faccionId, activo: true },
      sesion.getState(),
      actor({ jugadorId: fundador, faccionId })
    );
    expect(resultado).toEqual({ autorizado: true });
  });
});

describe('colocarOrdenMercado / reclutarTropa: residente del asentamiento objetivo', () => {
  it('colocarOrdenMercado rechaza a quien no reside ahí', () => {
    const { sesion, asentamientoId } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'colocarOrdenMercado',
      { asentamientoId, tipo: 'venta', recurso: 'trigo', cantidad: 1 },
      sesion.getState(),
      actor({ jugadorId: 'forastero', faccionId: null })
    );
    expect(resultado).toEqual({ autorizado: false, motivo: 'condicion_dominio' });
  });

  it('reclutarTropa rechaza reclutar a nombre de otro jugador aunque ambos residan ahí', () => {
    const { sesion, asentamientoId, fundador } = partidaConAsentamiento();
    const resultado = verificarAutorizacion(
      'reclutarTropa',
      { asentamientoId, jugadorId: fundador, tropaId: 'x', origen: 'pesants' },
      sesion.getState(),
      actor({ jugadorId: 'otro-fundador-mismo-asentamiento', faccionId: null })
    );
    expect(resultado).toEqual({ autorizado: false, motivo: 'condicion_dominio' });
  });
});

describe('romperRelacion / rebelionVasallo / anexionar (relaciones entre Facciones)', () => {
  function partidaConSegundaFaccion() {
    const base = partidaConAsentamiento();
    const rf = base.sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, OPC);
    const faccionRivalId = rf.datos!.faccionId;
    return { ...base, faccionRivalId };
  }

  it('romperRelacion exige ser rey/embajador de la Facción iniciadora, aunque la relación exista', () => {
    const { sesion, faccionId, faccionRivalId, fundador } = partidaConSegundaFaccion();
    const estadoConRelacion = {
      ...sesion.getState(),
      relaciones: [{ id: 'r1', tipo: 'alianza' as const, faccionAId: faccionId, faccionBId: faccionRivalId, creadoEnTick: 0, estado: 'activa' as const }],
    };
    const sinCargo = verificarAutorizacion(
      'romperRelacion',
      { relacionId: 'r1', iniciadorFaccionId: faccionId },
      estadoConRelacion,
      actor({ jugadorId: fundador, faccionId })
    );
    expect(sinCargo).toEqual({ autorizado: false, motivo: 'condicion_dominio' });

    sesion.ejecutar(asignarRey, { faccionId, jugadorId: fundador }, OPC);
    const conCargo = verificarAutorizacion(
      'romperRelacion',
      { relacionId: 'r1', iniciadorFaccionId: faccionId },
      { ...sesion.getState(), relaciones: estadoConRelacion.relaciones },
      actor({ jugadorId: fundador, faccionId })
    );
    expect(conCargo).toEqual({ autorizado: true });
  });

  it('rebelionVasallo exige ser rey/embajador del VASALLO (faccionBId), no del señor', () => {
    const { sesion, faccionId, faccionRivalId, fundador } = partidaConSegundaFaccion();
    sesion.ejecutar(asignarRey, { faccionId, jugadorId: fundador }, OPC);
    const estadoConVasallaje = {
      ...sesion.getState(),
      relaciones: [
        { id: 'r1', tipo: 'vasallaje' as const, faccionAId: faccionId, faccionBId: faccionRivalId, creadoEnTick: 0, estado: 'activa' as const },
      ],
    };

    // El fundador es rey de la Facción SEÑORA (faccionId), no de la vasalla (faccionRivalId): no puede rebelarse.
    const comoSenor = verificarAutorizacion(
      'rebelionVasallo',
      { relacionId: 'r1' },
      estadoConVasallaje,
      actor({ jugadorId: fundador, faccionId })
    );
    expect(comoSenor).toEqual({ autorizado: false, motivo: 'condicion_dominio' });
  });

  it('anexionar exige ser rey/embajador de la Facción absorbente (faccionAId)', () => {
    const { sesion, faccionId, faccionRivalId, fundador } = partidaConSegundaFaccion();
    const sinCargo = verificarAutorizacion(
      'anexionar',
      { faccionAId: faccionId, faccionBId: faccionRivalId },
      sesion.getState(),
      actor({ jugadorId: fundador, faccionId })
    );
    expect(sinCargo).toEqual({ autorizado: false, motivo: 'condicion_dominio' });

    sesion.ejecutar(asignarRey, { faccionId, jugadorId: fundador }, OPC);
    const conCargo = verificarAutorizacion(
      'anexionar',
      { faccionAId: faccionId, faccionBId: faccionRivalId },
      sesion.getState(),
      actor({ jugadorId: fundador, faccionId })
    );
    expect(conCargo).toEqual({ autorizado: true });
  });
});
