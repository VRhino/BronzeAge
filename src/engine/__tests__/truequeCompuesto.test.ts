// Trueque compuesto (Doc 3.2, 2026-10-02): cada lado de un acuerdo entrega una o varias líneas. Las líneas salen en UN
// cargamento, se abonan por recurso y el acuerdo solo se cumple con todas saldadas. Al vencer, la penalización de
// reputación es proporcional a lo no entregado.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Caravana, Faccion, Point } from '../../domain/types';
import { REPUTACION } from '../../constants';
import { aceptarTrueque, aplicarEntregaATrueque, avanzarComercio, construirCaravanaComercial, proponerTrueque, TruequeInvalidoError } from '../trade';
import { capacidadCaravana } from '../caravanas';
import { fraccionIncumplida, lineasPendientes, ofreceRecurso } from '../trueque';
import { RED_VACIA } from '../redCaminos';
import { almacenSintetico, mapaSintetico } from './tradeFixtures';
import { instanteDeTest } from './fixtures';

const plaza = (id: string, posicion: Point, recursos: Record<string, number>, conMercado: boolean): Asentamiento =>
  ({
    id,
    faccionId: `faccion-${id}`,
    posicion,
    almacen: almacenSintetico(recursos),
    politicasActivas: [],
    edificios: conMercado ? [{ id: `mercado-${id}`, tipo: 'mercado', posicion, estado: 'activo', nivelInterno: 1 }] : [],
  }) as unknown as Asentamiento;

describe('proponerTrueque con varias líneas', () => {
  const a = plaza('a', { x: 0, y: 0 }, {}, false);
  const b = plaza('b', { x: 30, y: 0 }, {}, false);
  const propone = (lineasA: { recurso: string; cantidad: number }[], lineasB: { recurso: string; cantidad: number }[]) =>
    proponerTrueque([a, b], 'a', 'b', lineasA, lineasB, instanteDeTest(0), 0);

  it('guarda cada línea con su recurso, lo pactado y 0 entregado', () => {
    const t = propone([{ recurso: 'madera', cantidad: 100 }, { recurso: 'piedra', cantidad: 50 }], [{ recurso: 'oro', cantidad: 10 }]);
    expect(t.lineasA).toEqual([
      { recurso: 'madera', cantidadTotal: 100, cantidadEntregada: 0 },
      { recurso: 'piedra', cantidadTotal: 50, cantidadEntregada: 0 },
    ]);
    expect(t.lineasB).toEqual([{ recurso: 'oro', cantidadTotal: 10, cantidadEntregada: 0 }]);
  });

  it('rechaza un lado vacío, una cantidad no positiva y un recurso repetido en el mismo lado', () => {
    expect(() => propone([], [{ recurso: 'oro', cantidad: 1 }])).toThrow(TruequeInvalidoError);
    expect(() => propone([{ recurso: 'madera', cantidad: 0 }], [{ recurso: 'oro', cantidad: 1 }])).toThrow(TruequeInvalidoError);
    expect(() => propone([{ recurso: 'madera', cantidad: 5 }, { recurso: 'madera', cantidad: 5 }], [{ recurso: 'oro', cantidad: 1 }])).toThrow(TruequeInvalidoError);
  });
});

describe('abonos y cierre', () => {
  const a = plaza('a', { x: 0, y: 0 }, {}, false);
  const b = plaza('b', { x: 30, y: 0 }, {}, false);
  const porId = new Map([a, b].map((p) => [p.id, p]));
  const acuerdo = aceptarTrueque(
    proponerTrueque([a, b], 'a', 'b', [{ recurso: 'madera', cantidad: 10 }, { recurso: 'piedra', cantidad: 10 }], [{ recurso: 'oro', cantidad: 5 }], instanteDeTest(0), 0),
    instanteDeTest(0)
  );

  it('cada recurso se abona a su línea, y no se cumple hasta saldarlas todas', () => {
    const r1 = aplicarEntregaATrueque(acuerdo, 'A', { madera: 10 }, porId);
    expect(lineasPendientes(r1.acuerdo, 'A')).toEqual([{ recurso: 'piedra', faltante: 10 }]);
    const r2 = aplicarEntregaATrueque(r1.acuerdo, 'B', { oro: 5 }, porId);
    expect(r2.acuerdo.estado, 'falta la piedra de A').toBe('activo');
    const r3 = aplicarEntregaATrueque(r2.acuerdo, 'A', { piedra: 10 }, porId);
    expect(r3.acuerdo.estado).toBe('cumplido');
  });

  it('fraccionIncumplida pondera por lo pactado, y ofreceRecurso mira las líneas del lado', () => {
    const medio = aplicarEntregaATrueque(acuerdo, 'A', { madera: 10 }, porId).acuerdo;
    expect(fraccionIncumplida(medio, 'A')).toBeCloseTo(0.5);
    expect(fraccionIncumplida(medio, 'B')).toBe(1);
    expect(ofreceRecurso(acuerdo, 'A', 'piedra')).toBe(true);
    expect(ofreceRecurso(acuerdo, 'B', 'piedra')).toBe(false);
  });
});

describe('un cargamento por lado', () => {
  function montar(almacenOrigen: Record<string, number>, lineasA: { recurso: string; cantidad: number }[]) {
    const origen0 = plaza('origen', { x: 0, y: 0 }, { madera: 50, oro: 100, ...almacenOrigen }, true);
    const destino0 = plaza('destino', { x: 30, y: 0 }, { oro: 1000 }, false);
    const { asentamiento: origen, caravana } = construirCaravanaComercial(origen0, [], instanteDeTest(0), 0);
    const acuerdo = aceptarTrueque(proponerTrueque([origen, destino0], 'origen', 'destino', lineasA, [{ recurso: 'oro', cantidad: 1 }], instanteDeTest(0), 0), instanteDeTest(0));
    const mapa = mapaSintetico({ limites: { ancho: 1000, alto: 1000 } });
    const tick = (asentamientos: Asentamiento[], caravanas: Caravana[], acuerdos = [acuerdo], n = 1) =>
      avanzarComercio(asentamientos, [] as Faccion[], caravanas, acuerdos, mapa, RED_VACIA, [], instanteDeTest(n));
    return { origen, destino0, caravana, acuerdo, tick, capacidad: capacidadCaravana(caravana) };
  }

  it('sale con todas las líneas a la vez y entrega abonando cada una', () => {
    const { origen, destino0, caravana, tick } = montar({ piedra: 1000 }, [{ recurso: 'piedra', cantidad: 100 }, { recurso: 'madera', cantidad: 20 }]);
    const sale = tick([origen, destino0], [caravana]);
    expect(sale.caravanas[0]!.contenido).toEqual({ piedra: 100, madera: 20 });
    expect(sale.asentamientos.find((p) => p.id === 'origen')!.almacen['piedra']!.cantidad).toBe(900);

    let estado = sale;
    for (let t = 2; t < 20 && estado.acuerdos[0]!.lineasA.some((l) => l.cantidadEntregada === 0); t++) {
      estado = tick(estado.asentamientos, estado.caravanas, estado.acuerdos, t);
    }
    expect(estado.acuerdos[0]!.lineasA.map((l) => l.cantidadEntregada)).toEqual([100, 20]);
  });

  it('llena la caravana por orden de líneas y deja el resto para el siguiente viaje', () => {
    const { capacidad } = montar({ piedra: 100_000 }, [{ recurso: 'piedra', cantidad: 1 }]);
    // La primera línea pide más que la capacidad: ocupa todo y la segunda espera.
    const m = montar({ piedra: 100_000 }, [{ recurso: 'piedra', cantidad: capacidad * 2 }, { recurso: 'madera', cantidad: 20 }]);
    const sale = m.tick([m.origen, m.destino0], [m.caravana]);
    expect(sale.caravanas[0]!.contenido).toEqual({ piedra: capacidad });
  });

  it('si una línea no tiene stock, sale lo que hay de las demás', () => {
    const { origen, destino0, caravana, tick } = montar({ piedra: 1000 }, [{ recurso: 'estano', cantidad: 50 }, { recurso: 'piedra', cantidad: 40 }]);
    expect(tick([origen, destino0], [caravana]).caravanas[0]!.contenido).toEqual({ piedra: 40 });
  });
});

describe('vencer sin cumplir', () => {
  it('penaliza a cada lado en proporción a lo que no entregó', () => {
    const a = plaza('a', { x: 0, y: 0 }, {}, false);
    const b = plaza('b', { x: 30, y: 0 }, {}, false);
    const porId = new Map([a, b].map((p) => [p.id, p]));
    const inicial = aceptarTrueque(
      proponerTrueque([a, b], 'a', 'b', [{ recurso: 'madera', cantidad: 10 }, { recurso: 'piedra', cantidad: 10 }], [{ recurso: 'oro', cantidad: 10 }], instanteDeTest(0), 0),
      instanteDeTest(0)
    );
    // A entrega la mitad de lo pactado; B no entrega nada.
    const acuerdo = aplicarEntregaATrueque(inicial, 'A', { madera: 10 }, porId).acuerdo;
    const facciones = [
      { id: 'faccion-a', reputacion: 0 },
      { id: 'faccion-b', reputacion: 0 },
    ] as unknown as Faccion[];

    const r = avanzarComercio([a, b], facciones, [], [acuerdo], mapaSintetico({ limites: { ancho: 1000, alto: 1000 } }), RED_VACIA, [], acuerdo.expiraEn);

    expect(r.acuerdos[0]!.estado).toBe('expirado');
    const rep = (id: string) => r.facciones.find((f) => f.id === id)!.reputacion;
    expect(rep('faccion-a')).toBeCloseTo(REPUTACION.penalizacionTruequeIncumplido * 0.5);
    expect(rep('faccion-b')).toBeCloseTo(REPUTACION.penalizacionTruequeIncumplido);
  });
});
