// Fusión con aceptación del Rey de B y nacimiento de la Facción nueva (Doc 2.6, opción 2), por la ruta real: comandos de `GameSession`.
import { describe, expect, it } from 'vitest';
import { FUSION } from '../../constants';
import { dias } from '../../domain/tiempo';
import { calcularNivelFaccion } from '../../engine/faccion';
import { GameSession } from '../gameSession';
import type { GameSessionState } from '../estado';
import { proponerRelacion } from '../comandos/diplomacia';
import { proponerFusion, responderFusion, retirarFusion } from '../comandos/fusion';
import { faccionAsentadaDePrueba } from './faccionAsentadaDePrueba';
import { OPC } from './fixtures';

function partida(...nombres: string[]) {
  const s = GameSession.crear('fusion', { seed: 11 });
  const facciones = nombres.map((nombre) => s.ejecutar(faccionAsentadaDePrueba, { nombre }).datos!);
  return { s, ids: facciones.map((f) => f.faccionId), plazas: facciones.map((f) => f.asentamientoId) };
}

const codigos = (s: GameSession) => s.getState().eventosDominio.map((e) => e.codigo);
const propuestas = (s: GameSession) => s.getState().propuestasFusion ?? [];
const faccion = (s: GameSession, id: string) => s.getState().facciones.find((f) => f.id === id)!;

function conEstado(s: GameSession, f: (e: GameSessionState) => GameSessionState): GameSession {
  const x = s.exportar();
  return GameSession.importar({ ...x, state: f(x.state) });
}

/** Propone la fusión de ids[0] con ids[1] en 'Nueva', con el Rey de la primera por Rey, y devuelve el id de la propuesta. */
function proponer(s: GameSession, ids: string[]) {
  const r = s.ejecutar(proponerFusion, { faccionAId: ids[0]!, faccionBId: ids[1]!, nuevoNombre: 'Nueva', nuevoReyId: faccion(s, ids[0]!).reyId! }, OPC);
  expect(r.ok).toBe(true);
  return r.datos!.propuestaId;
}

describe('proponerFusion', () => {
  it('deja la propuesta pendiente con caducidad, sin tocar nada más', () => {
    const { s, ids } = partida('Alfa', 'Beta');
    const reyB = faccion(s, ids[1]!).reyId!;
    const r = s.ejecutar(proponerFusion, { faccionAId: ids[0]!, faccionBId: ids[1]!, nuevoNombre: '  Nueva  ', nuevoReyId: reyB }, OPC);

    expect(r.ok).toBe(true);
    const p = propuestas(s)[0]!;
    expect(p).toMatchObject({
      id: r.datos!.propuestaId,
      faccionAId: ids[0],
      faccionBId: ids[1],
      nuevoNombre: 'Nueva',
      nuevoReyId: reyB,
      propuestaPor: faccion(s, ids[0]!).reyId,
    });
    expect(p.expiraEn - p.creadaEn).toBe(dias(FUSION.caducidadDias));
    expect(s.getState().facciones.map((f) => f.id)).toEqual(expect.arrayContaining(ids));
    expect(codigos(s)).toContain('diplomacia.fusion_propuesta');
  });

  it('rechaza un Rey que no es el de ninguna de las dos, un nombre vacío y fusionarse consigo misma', () => {
    const { s, ids } = partida('Alfa', 'Beta');
    const base = { faccionAId: ids[0]!, faccionBId: ids[1]!, nuevoNombre: 'Nueva', nuevoReyId: faccion(s, ids[0]!).reyId! };
    const reyes = [faccion(s, ids[0]!).reyId, faccion(s, ids[1]!).reyId];
    const raso = s.getState().heroes.find((h) => !reyes.includes(h.id))!.id;

    for (const params of [{ ...base, nuevoReyId: raso }, { ...base, nuevoNombre: '   ' }, { ...base, faccionBId: ids[0]! }]) {
      const r = s.ejecutar(proponerFusion, params, OPC);
      expect(r.ok).toBe(false);
      expect(r.codigoError).toBe('fusion.invalida');
    }
    expect(propuestas(s)).toEqual([]);
  });

  it('rechaza una segunda propuesta entre el mismo par, en cualquier sentido, y a una vasalla de un tercero', () => {
    const { s, ids } = partida('Alfa', 'Beta', 'Gamma');
    proponer(s, ids);
    const reyB = faccion(s, ids[1]!).reyId!;

    expect(s.ejecutar(proponerFusion, { faccionAId: ids[1]!, faccionBId: ids[0]!, nuevoNombre: 'Otra', nuevoReyId: reyB }, OPC).codigoError).toBe('fusion.invalida');

    s.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: ids[2]!, faccionBId: ids[1]! }, OPC);
    const reyA = faccion(s, ids[0]!).reyId!;
    expect(s.ejecutar(proponerFusion, { faccionAId: ids[0]!, faccionBId: ids[1]!, nuevoNombre: 'X', nuevoReyId: reyA }, OPC).codigoError).toBe('fusion.invalida');
    // Su propio señor sí puede: el vasallaje termina en la fusión.
    const reyG = faccion(s, ids[2]!).reyId!;
    expect(s.ejecutar(proponerFusion, { faccionAId: ids[2]!, faccionBId: ids[1]!, nuevoNombre: 'X', nuevoReyId: reyG }, OPC).ok).toBe(true);
  });

  it('una propuesta caducada ya no cuenta: no se puede contestar y se puede volver a proponer', () => {
    const { s: s0, ids } = partida('Alfa', 'Beta');
    const id = proponer(s0, ids);
    const s = conEstado(s0, (e) => ({ ...e, propuestasFusion: e.propuestasFusion!.map((p) => ({ ...p, expiraEn: p.creadaEn })) }));

    expect(s.ejecutar(responderFusion, { propuestaId: id, aceptar: true }, OPC).codigoError).toBe('fusion.caducada');
    proponer(s, ids);
    expect(propuestas(s)).toHaveLength(1);
  });
});

describe('responderFusion', () => {
  it('rechazar la deja sin efecto y las dos siguen en pie', () => {
    const { s, ids } = partida('Alfa', 'Beta');
    const id = proponer(s, ids);

    expect(s.ejecutar(responderFusion, { propuestaId: id, aceptar: false }, OPC).ok).toBe(true);
    expect(propuestas(s)).toEqual([]);
    expect(ids.every((i) => s.getState().facciones.some((f) => f.id === i))).toBe(true);
    expect(codigos(s)).toContain('diplomacia.fusion_rechazada');
  });

  it('aceptar la ejecuta: nace C con el Rey y el nombre pactados y todo lo de A y B pasa a ella', () => {
    const { s: s0, ids, plazas } = partida('Alfa', 'Beta', 'Gamma', 'Delta', 'Epsilon');
    const [a, b, aliadaDeB, vasallaDeA, vasallaDeB] = ids as [string, string, string, string, string];
    s0.ejecutar(proponerRelacion, { tipo: 'alianza', faccionAId: b, faccionBId: aliadaDeB }, OPC);
    s0.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: a, faccionBId: vasallaDeA, tributoRecurso: 'madera', tributoCantidad: 2 }, OPC);
    s0.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: b, faccionBId: vasallaDeB, tributoRecurso: 'piedra', tributoCantidad: 3 }, OPC);
    const [fa, fb] = [faccion(s0, a), faccion(s0, b)];
    // A: más reputación y una tecnología propia; B: más experiencia y otra (aparecida pero sin adoptar).
    const s = conEstado(s0, (e) => ({
      ...e,
      facciones: e.facciones.map((f) => (f.id === a ? { ...f, reputacion: 40, experiencia: 10 } : f.id === b ? { ...f, reputacion: -5, experiencia: 900 } : f)),
      tecnologia: {
        ...e.tecnologia,
        porFaccion: {
          ...e.tecnologia.porFaccion,
          [a]: { aparecidas: ['tecA' as never], adoptadas: ['tecA' as never] },
          [b]: { aparecidas: ['tecB' as never], adoptadas: [] },
        },
      },
    }));
    // El Rey de B será el Rey de C: la propuesta lo nombra.
    const id = s.ejecutar(proponerFusion, { faccionAId: a, faccionBId: b, nuevoNombre: 'Imperio', nuevoReyId: fb.reyId! }, OPC).datos!.propuestaId;

    const r = s.ejecutar(responderFusion, { propuestaId: id, aceptar: true }, OPC);

    expect(r.ok).toBe(true);
    const e = s.getState();
    const c = e.facciones.find((f) => f.nombre === 'Imperio')!;
    expect(e.facciones.some((f) => f.id === a || f.id === b)).toBe(false);
    expect(c).toMatchObject({ reyId: fb.reyId, embajadorId: null, sigilo: fa.sigilo, reputacion: 40, experiencia: 900 });
    expect(c.nivel).toBe(calcularNivelFaccion(c));
    expect(c.ciudadanosIds).toEqual(expect.arrayContaining([...fa.ciudadanosIds, ...fb.ciudadanosIds]));
    expect(e.asentamientos.filter((p) => plazas.slice(0, 2).includes(p.id)).map((p) => p.faccionId)).toEqual([c.id, c.id]);
    expect([...e.ejercitos, ...e.caravanas].every((x) => x.faccionId !== a && x.faccionId !== b)).toBe(true);
    expect(Object.keys(e.memoriaPorFaccion).some((k) => k === a || k === b)).toBe(false);
    expect(e.tecnologia.porFaccion[a]).toBeUndefined();
    expect(e.tecnologia.porFaccion[c.id]).toMatchObject({ aparecidas: expect.arrayContaining(['tecA', 'tecB']), adoptadas: ['tecA'] });
    // Alianzas y guerras de las dos se cancelan; sus vasallos pasan a C con su tributo.
    const activas = e.relaciones.filter((x) => x.estado === 'activa');
    expect(activas).toHaveLength(2);
    expect(activas).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ tipo: 'vasallaje', faccionAId: c.id, faccionBId: vasallaDeA, tributo: { recurso: 'madera', cantidadPorMinuto: 2 } }),
        expect.objectContaining({ tipo: 'vasallaje', faccionAId: c.id, faccionBId: vasallaDeB, tributo: { recurso: 'piedra', cantidadPorMinuto: 3 } }),
      ])
    );
    expect(e.propuestasFusion).toEqual([]);
    expect(codigos(s)).toContain('diplomacia.fusion');
  });

  it('la capital de C es la que había designado el Rey electo; la marca de la otra desaparece', () => {
    const { s: s0, ids, plazas } = partida('Alfa', 'Beta');
    const s = conEstado(s0, (e) => ({
      ...e,
      asentamientos: e.asentamientos.map((p) => (p.id === plazas[0] ? { ...p, capitalDeFaccionId: ids[0] } : p.id === plazas[1] ? { ...p, capitalDeFaccionId: ids[1] } : p)),
    }));
    const id = s.ejecutar(proponerFusion, { faccionAId: ids[0]!, faccionBId: ids[1]!, nuevoNombre: 'Imperio', nuevoReyId: faccion(s, ids[1]!).reyId! }, OPC).datos!.propuestaId;
    s.ejecutar(responderFusion, { propuestaId: id, aceptar: true }, OPC);

    const c = s.getState().facciones.find((f) => f.nombre === 'Imperio')!;
    expect(s.getState().asentamientos.find((p) => p.id === plazas[1])!.capitalDeFaccionId).toBe(c.id);
    expect(s.getState().asentamientos.find((p) => p.id === plazas[0])!.capitalDeFaccionId).toBeUndefined();
  });

  it('el vasallaje entre A y B termina con la fusión sin dejar relaciones colgando', () => {
    const { s, ids } = partida('Alfa', 'Beta');
    s.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: ids[0]!, faccionBId: ids[1]! }, OPC);
    s.ejecutar(responderFusion, { propuestaId: proponer(s, ids), aceptar: true }, OPC);

    expect(s.getState().relaciones.filter((r) => r.estado === 'activa')).toEqual([]);
  });

  it('se rechaza si el Rey de A cambió o B pasó a ser vasalla de un tercero mientras esperaba', () => {
    const { s, ids } = partida('Alfa', 'Beta', 'Gamma');
    const id = proponer(s, ids);
    const fa = faccion(s, ids[0]!);
    const otroReyDeA = fa.ciudadanosIds.find((c) => c !== fa.reyId) ?? 'otro-rey';
    const cambiado = conEstado(s, (e) => ({ ...e, facciones: e.facciones.map((f) => (f.id === ids[0] ? { ...f, reyId: otroReyDeA } : f)) }));
    expect(cambiado.ejecutar(responderFusion, { propuestaId: id, aceptar: true }, OPC).codigoError).toBe('fusion.invalida');

    s.ejecutar(proponerRelacion, { tipo: 'vasallaje', faccionAId: ids[2]!, faccionBId: ids[1]! }, OPC);
    expect(s.ejecutar(responderFusion, { propuestaId: id, aceptar: true }, OPC).codigoError).toBe('fusion.invalida');
    expect(ids.every((i) => s.getState().facciones.some((f) => f.id === i))).toBe(true);
  });

  it('las propuestas que colgaban de A o de B desaparecen con ellas', () => {
    const { s, ids } = partida('Alfa', 'Beta', 'Gamma');
    const reyG = faccion(s, ids[2]!).reyId!;
    s.ejecutar(proponerFusion, { faccionAId: ids[2]!, faccionBId: ids[1]!, nuevoNombre: 'X', nuevoReyId: reyG }, OPC);
    s.ejecutar(responderFusion, { propuestaId: proponer(s, ids), aceptar: true }, OPC);

    expect(propuestas(s)).toEqual([]);
  });
});

describe('retirarFusion', () => {
  it('quita la propuesta pendiente', () => {
    const { s, ids } = partida('Alfa', 'Beta');
    expect(s.ejecutar(retirarFusion, { propuestaId: proponer(s, ids) }, OPC).ok).toBe(true);
    expect(propuestas(s)).toEqual([]);
    expect(codigos(s)).toContain('diplomacia.fusion_retirada');
  });
});
