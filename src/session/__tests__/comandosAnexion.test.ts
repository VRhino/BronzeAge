// Anexión con aceptación de la absorbida (Doc 2.6) y liberación de vasallos del señor desarmado (Doc 2.4, ruptura 4), por la ruta real:
// comandos de `GameSession`, no el motor suelto.
import { describe, expect, it } from 'vitest';
import { ANEXION } from '../../constants';
import { dias } from '../../domain/tiempo';
import { GameSession } from '../gameSession';
import type { GameSessionState } from '../estado';
import { proponerRelacion } from '../comandos/diplomacia';
import { proponerAnexion, responderAnexion, retirarAnexion } from '../comandos/anexion';
import { faccionAsentadaDePrueba } from './faccionAsentadaDePrueba';
import { OPC } from './fixtures';

function partida(...nombres: string[]) {
  const s = GameSession.crear('anexion', { seed: 11 });
  const facciones = nombres.map((nombre) => s.ejecutar(faccionAsentadaDePrueba, { nombre }).datos!);
  return { s, ids: facciones.map((f) => f.faccionId), plazas: facciones.map((f) => f.asentamientoId) };
}

const codigos = (s: GameSession) => s.getState().eventosDominio.map((e) => e.codigo);
const propuestas = (s: GameSession) => s.getState().propuestasAnexion ?? [];

function conEstado(s: GameSession, f: (e: GameSessionState) => GameSessionState): GameSession {
  const x = s.exportar();
  return GameSession.importar({ ...x, state: f(x.state) });
}

describe('proponerAnexion', () => {
  it('deja la propuesta pendiente con caducidad, sin tocar nada más', () => {
    const { s, ids } = partida('Alfa', 'Beta');
    const r = s.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);

    expect(r.ok).toBe(true);
    const p = propuestas(s)[0]!;
    expect(p).toMatchObject({ id: r.datos!.propuestaId, absorbenteId: ids[0], absorbidaId: ids[1] });
    expect(p.expiraEn - p.creadaEn).toBe(dias(ANEXION.caducidadDias));
    expect(s.getState().facciones.map((f) => f.id)).toEqual(expect.arrayContaining(ids));
    expect(codigos(s)).toContain('diplomacia.anexion_propuesta');
  });

  it('rechaza una segunda propuesta entre el mismo par, en cualquier sentido', () => {
    const { s, ids } = partida('Alfa', 'Beta');
    s.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);

    for (const [a, b] of [[ids[0]!, ids[1]!], [ids[1]!, ids[0]!]] as const) {
      const r = s.ejecutar(proponerAnexion, { faccionAId: a, faccionBId: b }, OPC);
      expect(r.ok).toBe(false);
      expect(r.codigoError).toBe('anexion.invalida');
    }
    expect(propuestas(s)).toHaveLength(1);
  });

  it('rechaza anexionarse a sí misma y anexionar a la vasalla de un tercero', () => {
    const { s, ids } = partida('Alfa', 'Beta', 'Gamma');
    expect(s.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[0]! }, OPC).codigoError).toBe('anexion.invalida');

    s.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: ids[2]!, faccionBId: ids[1]! }, OPC);
    expect(s.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC).codigoError).toBe('anexion.invalida');
    // Su propio señor sí puede: el vasallaje termina en la anexión.
    expect(s.ejecutar(proponerAnexion, { faccionAId: ids[2]!, faccionBId: ids[1]! }, OPC).ok).toBe(true);
  });

  it('una propuesta caducada ya no cuenta: se puede volver a proponer y no se puede contestar', () => {
    const { s: s0, ids } = partida('Alfa', 'Beta');
    s0.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);
    const id = propuestas(s0)[0]!.id;
    const s = conEstado(s0, (e) => ({ ...e, propuestasAnexion: e.propuestasAnexion!.map((p) => ({ ...p, expiraEn: e.propuestasAnexion![0]!.creadaEn })) }));

    expect(s.ejecutar(responderAnexion, { propuestaId: id, aceptar: true }, OPC).codigoError).toBe('anexion.caducada');
    expect(s.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC).ok).toBe(true);
    expect(propuestas(s)).toHaveLength(1);
  });
});

describe('responderAnexion', () => {
  it('rechazar la deja sin efecto y B sigue en pie', () => {
    const { s, ids } = partida('Alfa', 'Beta');
    s.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);
    const r = s.ejecutar(responderAnexion, { propuestaId: propuestas(s)[0]!.id, aceptar: false }, OPC);

    expect(r.ok).toBe(true);
    expect(propuestas(s)).toHaveLength(0);
    expect(s.getState().facciones.some((f) => f.id === ids[1])).toBe(true);
    expect(codigos(s)).toContain('diplomacia.anexion_rechazada');
  });

  it('aceptar la ejecuta: plazas, ciudadanos y ejércitos pasan a A; sus relaciones se cancelan o se heredan', () => {
    const { s, ids, plazas } = partida('Alfa', 'Beta', 'Gamma', 'Delta');
    const [a, b, aliada, vasalla] = ids as [string, string, string, string];
    s.ejecutar(proponerRelacion, { tipo: 'alianza', faccionAId: b, faccionBId: aliada }, OPC);
    s.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: b, faccionBId: vasalla, tributoRecurso: 'madera', tributoCantidad: 3 }, OPC);
    const ciudadanosB = s.getState().facciones.find((f) => f.id === b)!.ciudadanosIds;
    s.ejecutar(proponerAnexion, { faccionAId: a, faccionBId: b }, OPC);

    const r = s.ejecutar(responderAnexion, { propuestaId: propuestas(s)[0]!.id, aceptar: true }, OPC);

    expect(r.ok).toBe(true);
    const e = s.getState();
    expect(e.facciones.some((f) => f.id === b)).toBe(false);
    expect(e.asentamientos.find((p) => p.id === plazas[1])!.faccionId).toBe(a);
    expect(e.facciones.find((f) => f.id === a)!.ciudadanosIds).toEqual(expect.arrayContaining(ciudadanosB));
    expect(e.ejercitos.every((x) => x.faccionId !== b)).toBe(true);
    expect(e.propuestasAnexion).toEqual([]);
    const activas = e.relaciones.filter((x) => x.estado === 'activa');
    expect(activas).toHaveLength(1);
    expect(activas[0]).toMatchObject({ tipo: 'vasallaje', faccionAId: a, faccionBId: vasalla, tributo: { recurso: 'madera', cantidadPorMinuto: 3 } });
    expect(codigos(s)).toContain('diplomacia.anexion');
  });

  it('el vasallaje entre A y B termina con la anexión sin dejar relaciones colgando', () => {
    const { s, ids } = partida('Alfa', 'Beta');
    s.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);
    s.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);
    s.ejecutar(responderAnexion, { propuestaId: propuestas(s)[0]!.id, aceptar: true }, OPC);

    expect(s.getState().relaciones.filter((r) => r.estado === 'activa')).toEqual([]);
  });

  it('se rechaza si B pasó a ser vasalla de un tercero mientras esperaba', () => {
    const { s, ids } = partida('Alfa', 'Beta', 'Gamma');
    s.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);
    s.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: ids[2]!, faccionBId: ids[1]! }, OPC);

    const r = s.ejecutar(responderAnexion, { propuestaId: propuestas(s)[0]!.id, aceptar: true }, OPC);
    expect(r.codigoError).toBe('anexion.invalida');
    expect(s.getState().facciones.some((f) => f.id === ids[1])).toBe(true);
  });

  it('las propuestas que colgaban de B desaparecen con ella', () => {
    const { s, ids } = partida('Alfa', 'Beta', 'Gamma');
    s.ejecutar(proponerAnexion, { faccionAId: ids[2]!, faccionBId: ids[1]! }, OPC);
    s.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);
    s.ejecutar(responderAnexion, { propuestaId: propuestas(s)[1]!.id, aceptar: true }, OPC);

    expect(propuestas(s)).toEqual([]);
  });
});

describe('retirarAnexion', () => {
  it('quita la propuesta pendiente', () => {
    const { s, ids } = partida('Alfa', 'Beta');
    s.ejecutar(proponerAnexion, { faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);
    expect(s.ejecutar(retirarAnexion, { propuestaId: propuestas(s)[0]!.id }, OPC).ok).toBe(true);
    expect(propuestas(s)).toEqual([]);
    expect(codigos(s)).toContain('diplomacia.anexion_retirada');
  });
});

describe('desarme del señor (Doc 2.4, ruptura 4)', () => {
  it('sus vasallos quedan libres en el tick en que se queda sin asentamientos, y no antes', () => {
    const { s: s0, ids, plazas } = partida('Alfa', 'Beta');
    s0.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);
    s0.avanzarTick();
    expect(s0.getState().relaciones[0]!.estado).toBe('activa');

    const s = conEstado(s0, (e) => ({ ...e, asentamientos: e.asentamientos.filter((a) => a.id !== plazas[0]) }));
    s.avanzarTick();

    expect(s.getState().relaciones[0]!.estado).toBe('rota');
    expect(codigos(s)).toContain('diplomacia.vasallo_liberado');
    expect(s.getState().facciones.map((f) => f.reputacion)).toEqual(s0.getState().facciones.map((f) => f.reputacion));
  });
});
