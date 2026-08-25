// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `trade.ts` migrado. Reusa las fixtures sintéticas
// de `tradeFixtures.ts` (mismo patrón que `trade_peaje.test.ts`) — el peaje (`comercio.peaje`) ya queda
// cubierto ahí, no se repite aquí.
import { describe, expect, it } from 'vitest';
import type { AcuerdoTrueque, Asentamiento, Caravana, Faccion } from '../../domain/types';
import { avanzarComercio } from '../trade';
import type {
  PayloadCaravanaLlega,
  PayloadCaravanaSale,
  PayloadTruequeCumplido,
  PayloadTruequeExpirado,
} from '../trade';
import { almacenSintetico, caravanaComercialCasiLlegando, mapaSintetico } from './tradeFixtures';

function asentamiento(id: string, faccionId: string, posicion: { x: number; y: number }, oro: number): Asentamiento {
  return { id, faccionId, posicion, almacen: almacenSintetico({ oro }), politicasActivas: [] } as unknown as Asentamiento;
}

describe('eventos de dominio — trade.ts', () => {
  it('caravana que llega a destino produce comercio.caravana_llega', () => {
    const origen = asentamiento('origen', 'f-origen', { x: 0, y: 0 }, 0);
    const destino = asentamiento('destino', 'f-destino', { x: 1000, y: 0 }, 0);
    const mapa = mapaSintetico();
    const caravana = caravanaComercialCasiLlegando(origen, destino, { contenido: { madera: 10 } });

    const resultado = avanzarComercio([origen, destino], [] as Faccion[], [caravana], [], mapa, [], [], 1);

    const evento = resultado.eventos.find((e) => typeof e !== 'string' && e.codigo === 'comercio.caravana_llega');
    expect(evento).toBeDefined();
    const p = (evento as { payload: unknown }).payload as PayloadCaravanaLlega;
    expect(p.caravanaId).toBe('caravana-1');
    expect(p.origenId).toBe('origen');
    expect(p.destinoId).toBe('destino');
    expect(p.contenido).toEqual({ madera: 10 });
  });

  it('acuerdo vencido sin cumplirse produce comercio.trueque_expirado', () => {
    const origen = asentamiento('origen', 'f-origen', { x: 0, y: 0 }, 0);
    const destino = asentamiento('destino', 'f-destino', { x: 1000, y: 0 }, 0);
    const acuerdo: AcuerdoTrueque = {
      id: 'acuerdo-1',
      asentamientoAId: 'origen',
      asentamientoBId: 'destino',
      recursoA: 'madera',
      recursoB: 'piedra',
      cantidadTotalA: 100,
      cantidadTotalB: 100,
      cantidadEntregadaA: 0,
      cantidadEntregadaB: 0,
      creadoEnTick: 0,
      expiraEnTick: 1,
      estado: 'activo',
    };
    const mapa = mapaSintetico();

    const resultado = avanzarComercio([origen, destino], [] as Faccion[], [], [acuerdo], mapa, [], [], 5);

    const evento = resultado.eventos.find((e) => typeof e !== 'string' && e.codigo === 'comercio.trueque_expirado');
    expect(evento).toBeDefined();
    const p = (evento as { payload: unknown }).payload as PayloadTruequeExpirado;
    expect(p.acuerdoId).toBe('acuerdo-1');
    expect(resultado.acuerdos[0]!.estado).toBe('expirado');
  });

  it('caravana disponible asignada a un trueque pendiente produce comercio.caravana_sale', () => {
    const origen = asentamiento('origen', 'f-origen', { x: 0, y: 0 }, 0);
    origen.almacen = almacenSintetico({ madera: 1000, oro: 0 });
    const destino = asentamiento('destino', 'f-destino', { x: 1000, y: 0 }, 0);
    const acuerdo: AcuerdoTrueque = {
      id: 'acuerdo-2',
      asentamientoAId: 'origen',
      asentamientoBId: 'destino',
      recursoA: 'madera',
      recursoB: 'piedra',
      cantidadTotalA: 50,
      cantidadTotalB: 50,
      cantidadEntregadaA: 0,
      cantidadEntregadaB: 0,
      creadoEnTick: 0,
      expiraEnTick: 100,
      estado: 'activo',
    };
    const caravanaDisponible: Caravana = {
      id: 'caravana-disponible',
      tipo: 'comercial',
      origenAsentamientoId: 'origen',
      destinoAsentamientoId: undefined,
      contenido: {},
      posicionActual: origen.posicion,
      progreso: 0,
      estado: 'disponible',
    };
    const mapa = mapaSintetico();

    const resultado = avanzarComercio([origen, destino], [] as Faccion[], [caravanaDisponible], [acuerdo], mapa, [], [], 1);

    const evento = resultado.eventos.find((e) => typeof e !== 'string' && e.codigo === 'comercio.caravana_sale');
    expect(evento).toBeDefined();
    const p = (evento as { payload: unknown }).payload as PayloadCaravanaSale;
    expect(p.origenId).toBe('origen');
    expect(p.destinoId).toBe('destino');
    expect(p.recurso).toBe('madera');
  });

  it('entrega que completa un acuerdo por ambos lados produce comercio.trueque_cumplido', () => {
    const origen = asentamiento('origen', 'f-origen', { x: 0, y: 0 }, 0);
    const destino = asentamiento('destino', 'f-destino', { x: 1000, y: 0 }, 0);
    const acuerdo: AcuerdoTrueque = {
      id: 'acuerdo-3',
      asentamientoAId: 'origen',
      asentamientoBId: 'destino',
      recursoA: 'madera',
      recursoB: 'piedra',
      cantidadTotalA: 10,
      cantidadTotalB: 10,
      cantidadEntregadaA: 10,
      cantidadEntregadaB: 0,
      creadoEnTick: 0,
      expiraEnTick: 100,
      estado: 'activo',
    };
    const mapa = mapaSintetico();
    // Lado B: la caravana de `destino` entrega su parte y con eso el acuerdo queda cumplido por los dos lados.
    const caravana = caravanaComercialCasiLlegando(destino, origen, {
      id: 'caravana-b',
      contenido: { piedra: 10 },
      origenAcuerdoId: acuerdo.id,
      ladoAcuerdo: 'B',
    });

    const resultado = avanzarComercio([origen, destino], [] as Faccion[], [caravana], [acuerdo], mapa, [], [], 1);

    const evento = resultado.eventos.find((e) => typeof e !== 'string' && e.codigo === 'comercio.trueque_cumplido');
    expect(evento).toBeDefined();
    const p = (evento as { payload: unknown }).payload as PayloadTruequeCumplido;
    expect(p.acuerdoId).toBe('acuerdo-3');
    expect(resultado.acuerdos.find((a) => a.id === 'acuerdo-3')!.estado).toBe('cumplido');
  });
});
