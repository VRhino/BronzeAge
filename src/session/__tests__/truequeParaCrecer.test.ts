// Trueque para crecer (`session/npcGobernanza.ts`, decisión del usuario 2026-09-27): el NPC pide por trueque lo que le
// falta para su siguiente nivel y no produce. Salió del batch de la Era I: el bronce del nivel 3 no se movía nunca
// entre plazas NPC, y ninguna llegaba al nivel 3.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Faccion } from '../../domain/types';
import { createRng } from '../../worldgen';
import { avanzarNpcGobernanza } from '../npcGobernanza';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from '../../engine/__tests__/fixtures';

const lleno = (cantidad: number) => ({ cantidad, capacidad: cantidad });

function escenario(nivelFaccion: number) {
  const mapa = crearMapaDeterminista(7);
  const facciones: Faccion[] = crearFacciones().map((f) => (f.id === 'faccion-1' ? { ...f, nivel: nivelFaccion } : f));
  const pide = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
  const da = fundarAsentamientoDeTest(mapa, pide.facciones, 'faccion-2', [pide.asentamiento]);
  const necesitado: Asentamiento = {
    ...pide.asentamiento,
    nivel: 2,
    nivelActual: 2,
    almacen: { ...pide.asentamiento.almacen, madera: lleno(2000), piedra: lleno(2000), oro: lleno(2000) },
  };
  const donante: Asentamiento = { ...da.asentamiento, almacen: { ...da.asentamiento.almacen, lingoteBronce: lleno(2000) } };
  const r = avanzarNpcGobernanza(crearEstadoDeTest([necesitado, donante], da.facciones), mapa, contextoDeTest(1, createRng(5)), {});
  return { r, necesitado, donante };
}

describe('trueque para crecer', () => {
  it('una plaza de nivel 2 sin bronce se lo pide a una plaza NPC a la que le sobra, y esta acepta', () => {
    const { r, necesitado, donante } = escenario(3);
    const bronce = r.estado.acuerdos.find((a) => a.asentamientoAId === necesitado.id && a.lineasB[0]!.recurso === 'lingoteBronce');
    expect(bronce).toBeDefined();
    expect(bronce!.asentamientoBId).toBe(donante.id);
    expect(bronce!.lineasB[0]!.cantidadTotal).toBe(100); // lo que pide el coste del nivel 3
    expect(bronce!.estado).toBe('activo');
  });

  it('sin cupo de nivel 3 en su Facción, no lo pide: no acapara bronce para una subida que no puede hacer', () => {
    const { r, necesitado } = escenario(1);
    expect(r.estado.acuerdos.some((a) => a.asentamientoAId === necesitado.id && a.lineasB[0]!.recurso === 'lingoteBronce')).toBe(false);
  });
});
