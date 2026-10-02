// Lo que cuelga de un asentamiento que cae en ruinas se cierra en el momento de la ruina (`engine/ruina.ts`, decisiones
// del usuario 2026-10-02): su campamento de bandidos, sus trueques y las caravanas que iban o venían. Antes el
// campamento quedaba huérfano para siempre (B1) y el socio de un trueque cobraba la penalización por incumplir (B2).
import { describe, expect, it } from 'vitest';
import type { AcuerdoTrueque, Asentamiento, Caravana } from '../../domain/types';
import type { EstadoSimulacion } from '../simulation';
import { avanzarSimulacion } from '../simulation';
import { createRng } from '../../worldgen';
import { contextoDeTest, crearEstadoDeTest, crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

const mapa = crearMapaDeterminista(7);

/** Tres plazas de tres Facciones; A con una caravana de un buey y madera de sobra. */
function mundo(conBandidos: boolean) {
  let facciones = crearFacciones();
  const plazas: Asentamiento[] = [];
  for (const f of ['faccion-1', 'faccion-2', 'faccion-3']) {
    const r = fundarAsentamientoDeTest(mapa, facciones, f, plazas, 0);
    plazas.push(conBandidos ? r.asentamiento : { ...r.asentamiento, bandidosReaparecenEn: instanteDeTest(10_000_000) });
    facciones = r.facciones;
  }
  const [a, b, c] = plazas as [Asentamiento, Asentamiento, Asentamiento];
  return { a: { ...a, almacen: { ...a.almacen, madera: { ...a.almacen['madera']!, cantidad: 200 } } }, b, c, facciones };
}

/** Un trueque activo de A con C: A entrega 50 de madera, C 10 de piedra. */
const trueque = (a: Asentamiento, c: Asentamiento): AcuerdoTrueque => ({
  id: 'trueque-x',
  asentamientoAId: a.id,
  asentamientoBId: c.id,
  lineasA: [{ recurso: 'madera', cantidadTotal: 50, cantidadEntregada: 0 }],
  lineasB: [{ recurso: 'piedra', cantidadTotal: 10, cantidadEntregada: 0 }],
  creadoEn: instanteDeTest(0),
  expiraEn: instanteDeTest(400),
  estado: 'activo',
});
const caravanaDe = (a: Asentamiento): Caravana =>
  ({ id: 'caravana-a', tipo: 'comercial', estado: 'disponible', origenAsentamientoId: a.id, contenido: {}, posicionActual: a.posicion, progreso: 0, carros: [{ tipoCarro: 'basico', animal: 'buey' }] }) as Caravana;

function arruinar(estado: EstadoSimulacion, id: string): EstadoSimulacion {
  return {
    ...estado,
    asentamientos: estado.asentamientos.map((a) =>
      a.id !== id
        ? a
        : {
            ...a,
            medidorMantenimiento: 0.0001,
            nivelActual: 1,
            fundadoEn: instanteDeTest(-100_000),
            protegidaHasta: undefined,
            almacen: Object.fromEntries(Object.entries(a.almacen).map(([k, v]) => [k, { ...v, cantidad: 0 }])),
          }
    ),
  };
}

function correr(estado: EstadoSimulacion, desde: number, ticks: number) {
  const rng = createRng(7);
  const eventos: string[] = [];
  let actual = estado;
  for (let t = desde + 1; t <= desde + ticks; t++) {
    const r = avanzarSimulacion(actual, mapa, contextoDeTest(t, rng));
    eventos.push(...r.eventosDominio.map((e) => e.codigo));
    actual = r;
  }
  return { estado: actual, eventos };
}

describe('ruina: sus dependientes se cierran al caer', () => {
  it('su campamento de bandidos se dispersa', () => {
    const { a, b, c, facciones } = mundo(true);
    let { estado } = correr(crearEstadoDeTest([a, b, c], facciones), 0, 5);
    expect(estado.campamentosBandidos.some((x) => x.asentamientoId === c.id)).toBe(true);
    const tras = correr(arruinar(estado, c.id), 5, 1);
    expect(tras.eventos).toContain('asentamiento.ruinas');
    expect(tras.eventos).toContain('bandidos.campamento_disperso');
    estado = tras.estado;
    expect(estado.campamentosBandidos.some((x) => x.asentamientoId === c.id)).toBe(false);
  });

  it('un trueque activo con ella se cancela sin penalizar al socio (antes: −8 de reputación)', () => {
    const { a, b, c, facciones } = mundo(false);
    const inicio = crearEstadoDeTest([a, b, c], facciones, { caravanas: [caravanaDe(a)], acuerdos: [trueque(a, c)] });
    const tras = correr(arruinar(inicio, c.id), 0, 420);
    expect(tras.eventos).toContain('comercio.trueque_cancelado');
    expect(tras.estado.acuerdos.find((x) => x.id === 'trueque-x')?.estado ?? 'fuera de la lista').not.toBe('activo');
    expect(tras.estado.facciones.find((f) => f.id === 'faccion-1')!.reputacion).toBe(0);
  });

  it('una caravana en camino hacia ella da la vuelta y devuelve su carga a casa', () => {
    const { a, b, c, facciones } = mundo(false);
    const inicio = crearEstadoDeTest([a, b, c], facciones, { caravanas: [caravanaDe(a)], acuerdos: [trueque(a, c)] });
    // Unos ticks para que salga hacia C con la madera.
    let { estado } = correr(inicio, 0, 5);
    const enCamino = estado.caravanas.find((x) => x.id === 'caravana-a')!;
    expect(enCamino.estado).toBe('en_transito');
    expect(enCamino.contenido['madera']).toBeGreaterThan(0);
    const carga = enCamino.contenido['madera']!;
    const maderaEnCasa = estado.asentamientos.find((x) => x.id === a.id)!.almacen['madera']!.cantidad;

    const tras = correr(arruinar(estado, c.id), 5, 1);
    expect(tras.eventos).toContain('comercio.caravana_vuelve');
    expect(tras.estado.caravanas.find((x) => x.id === 'caravana-a')!.estado).toBe('retornando');

    // Tick a tick hasta que llega: ese tick, el almacén de A recibe la carga (hasta donde quepa).
    estado = tras.estado;
    const rng = createRng(11);
    let llegada: { antes: number; despues: number; capacidad: number } | undefined;
    for (let t = 7; t <= 700 && !llegada; t++) {
      const antes = estado.asentamientos.find((x) => x.id === a.id)!.almacen['madera']!;
      estado = avanzarSimulacion(estado, mapa, contextoDeTest(t, rng));
      const caravana = estado.caravanas.find((x) => x.id === 'caravana-a')!;
      if (caravana.estado === 'disponible') {
        const despues = estado.asentamientos.find((x) => x.id === a.id)!.almacen['madera']!;
        llegada = { antes: antes.cantidad, despues: despues.cantidad, capacidad: despues.capacidad };
        expect(caravana.contenido).toEqual({});
      }
    }
    expect(llegada).toBeDefined();
    // Lo que cabía de la carga entra; el mantenimiento de un tick no llega a una unidad de madera.
    expect(llegada!.despues - llegada!.antes).toBeGreaterThan(Math.min(carga, llegada!.capacidad - llegada!.antes) - 1);
    expect(maderaEnCasa).toBeGreaterThanOrEqual(0);
  });

  it('una caravana que sale de ella se pierde, con evento', () => {
    const { a, b, c, facciones } = mundo(false);
    const deC = { ...caravanaDe(c), id: 'caravana-c' };
    const tras = correr(arruinar(crearEstadoDeTest([a, b, c], facciones, { caravanas: [deC] }), c.id), 0, 1);
    expect(tras.eventos).toContain('comercio.caravana_perdida');
    expect(tras.estado.caravanas.some((x) => x.id === 'caravana-c')).toBe(false);
  });
});
