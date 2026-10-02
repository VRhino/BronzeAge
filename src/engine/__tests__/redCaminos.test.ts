// Red de caminos (Doc 1.6, `Consideraciones/Rutas_Caravana_Avanzadas_Definicion.md`): registro y recálculo de
// rutas, pesos, poda, tramos para pintar, el logro de `logistica_campana`, el paso forzado por ciudades ajenas
// con su peaje, y la inmunidad de las caravanas dentro de las zonas.

import { describe, expect, it } from 'vitest';
import { PEAJE_PASO, RED_CAMINOS } from '../../constants';
import { minutos, sumar } from '../../domain/tiempo';
import type { Asentamiento, Faccion, Point, RedCaminos, ZonaInfluencia } from '../../domain/types';
import { claveArista } from '../../world/grafoNavegacion';
import {
  aristasDeTrazado,
  caminoCompartidoAbierto,
  escalonDePeso,
  pesosDeRed,
  podarRutas,
  RED_VACIA,
  registrarRuta,
  tramosDeRed,
  trazarRutaComercial,
} from '../redCaminos';
import { avanzarComercio } from '../trade';
import { avanzarAtaquesBandidos } from '../bandidos';
import { enRefugio } from '../zones';
import { createRng } from '../../worldgen';
import { instanteDeTest } from './fixtures';
import { almacenSintetico, caravanaComercialCasiLlegando, mapaSintetico } from './tradeFixtures';

function asentamiento(id: string, faccionId: string, posicion: Point, recursos: Record<string, number> = {}): Asentamiento {
  return { id, faccionId, posicion, almacen: almacenSintetico(recursos), politicasActivas: [], edificios: [] } as unknown as Asentamiento;
}

/** Zona cuadrada de lado 2·r alrededor de un asentamiento. */
function zonaDe(a: Asentamiento, r: number): ZonaInfluencia {
  const { x, y } = a.posicion;
  return { asentamientoId: a.id, poligono: [{ x: x - r, y: y - r }, { x: x + r, y: y - r }, { x: x + r, y: y + r }, { x: x - r, y: y + r }] };
}

const A = asentamiento('A', 'f1', { x: 0, y: 0 });
const B = asentamiento('B', 'f1', { x: 900, y: 0 });

describe('registrarRuta', () => {
  it('recalcular el trazado de un par mueve su peso, pero las aristas viejas se quedan como sendero', () => {
    const primero: Point[] = [{ x: 0, y: 0 }, { x: 450, y: 0 }, { x: 900, y: 0 }];
    const segundo: Point[] = [{ x: 0, y: 0 }, { x: 450, y: 450 }, { x: 900, y: 0 }];
    const red1 = registrarRuta(RED_VACIA, A, B, primero, instanteDeTest(0));
    const red2 = registrarRuta(red1, A, B, segundo, instanteDeTest(1));

    expect(red2.rutas).toHaveLength(1);
    expect(red2.aristas).toHaveLength(4);
    const pesos = pesosDeRed(red2);
    for (const a of aristasDeTrazado(primero)) expect(pesos.get(a)).toBeUndefined();
    for (const a of aristasDeTrazado(segundo)) expect(pesos.get(a)).toBe(1);
  });

  it('dos pares que pisan la misma arista la comparten: su peso es 2', () => {
    const C = asentamiento('C', 'f2', { x: 900, y: 450 });
    const red = registrarRuta(
      registrarRuta(RED_VACIA, A, B, [{ x: 0, y: 0 }, { x: 450, y: 0 }, { x: 900, y: 0 }], instanteDeTest(0)),
      A,
      C,
      [{ x: 0, y: 0 }, { x: 450, y: 0 }, { x: 900, y: 450 }],
      instanteDeTest(0)
    );
    expect(pesosDeRed(red).get(claveArista({ x: 0, y: 0 }, { x: 450, y: 0 }))).toBe(2);
  });
});

describe('podarRutas', () => {
  const red = registrarRuta(RED_VACIA, A, B, [A.posicion, B.posicion], instanteDeTest(0));

  it('una ruta que lleva más de la caducidad sin lanzar deja de contar; la arista sigue', () => {
    const tarde = sumar(instanteDeTest(0), minutos(RED_CAMINOS.caducidadMinutos + 1));
    const podada = podarRutas(red, tarde, new Set(['A', 'B']));
    expect(podada.rutas).toEqual([]);
    expect(podada.aristas).toEqual(red.aristas);
  });

  it('desaparece con uno de sus extremos', () => {
    expect(podarRutas(red, instanteDeTest(1), new Set(['A'])).rutas).toEqual([]);
  });

  it('sin cambios devuelve la MISMA red (no invalida los derivados cacheados)', () => {
    expect(podarRutas(red, instanteDeTest(1), new Set(['A', 'B']))).toBe(red);
  });
});

describe('tramosDeRed', () => {
  it('fusiona una cadena del mismo escalón en una sola polilínea', () => {
    const puntos: Point[] = [{ x: 0, y: 0 }, { x: 45, y: 0 }, { x: 90, y: 45 }, { x: 135, y: 45 }];
    const tramos = tramosDeRed({ aristas: aristasDeTrazado(puntos), rutas: [] });
    expect(tramos).toHaveLength(1);
    expect(tramos[0]!.escalon).toBe(0);
    const extremos = [tramos[0]!.puntos[0], tramos[0]!.puntos.at(-1)];
    expect(extremos).toEqual(expect.arrayContaining([puntos[0], puntos[3]]));
    expect(tramos[0]!.puntos).toHaveLength(4);
  });

  it('corta donde cambia el escalón', () => {
    const comun: Point[] = [{ x: 0, y: 0 }, { x: 45, y: 0 }];
    let red: RedCaminos = { aristas: aristasDeTrazado([...comun, { x: 90, y: 0 }]), rutas: [] };
    for (let i = 0; i < RED_CAMINOS.escalonCamino; i++) {
      red = registrarRuta(red, asentamiento(`o${i}`, 'f1', comun[0]!), asentamiento(`d${i}`, 'f1', comun[1]!), comun, instanteDeTest(0));
    }
    expect(tramosDeRed(red).map((t) => t.escalon).sort()).toEqual([0, 1]);
  });

  it('escalones por peso', () => {
    expect(escalonDePeso(0)).toBe(0);
    expect(escalonDePeso(RED_CAMINOS.escalonCamino)).toBe(1);
    expect(escalonDePeso(RED_CAMINOS.escalonCalzada)).toBe(2);
  });
});

describe('caminoCompartidoAbierto (logro de logistica_campana)', () => {
  const tramo: Point[] = [{ x: 1000, y: 1000 }, { x: 1045, y: 1000 }];
  function redCompartida(rutas: number, facciones: number): RedCaminos {
    let red = RED_VACIA;
    for (let i = 0; i < rutas; i++) {
      const f = `f${i % facciones}`;
      red = registrarRuta(red, asentamiento(`o${i}`, f, tramo[0]!), asentamiento(`d${i}`, f, tramo[1]!), tramo, instanteDeTest(0));
    }
    return red;
  }

  it('se cumple con una arista fuera de toda zona, con las rutas y Facciones del umbral', () => {
    expect(caminoCompartidoAbierto(redCompartida(RED_CAMINOS.logroRutas, RED_CAMINOS.logroFacciones), [])).toBe(true);
  });

  it('no con una Facción de menos, ni con una ruta de menos', () => {
    expect(caminoCompartidoAbierto(redCompartida(RED_CAMINOS.logroRutas, RED_CAMINOS.logroFacciones - 1), [])).toBe(false);
    expect(caminoCompartidoAbierto(redCompartida(RED_CAMINOS.logroRutas - 1, RED_CAMINOS.logroFacciones), [])).toBe(false);
  });

  it('no si la arista cae dentro de una zona de influencia', () => {
    const zona = zonaDe(asentamiento('Z', 'f9', { x: 1020, y: 1000 }), 100);
    expect(caminoCompartidoAbierto(redCompartida(RED_CAMINOS.logroRutas, RED_CAMINOS.logroFacciones), [zona])).toBe(false);
  });
});

describe('paso forzado por una ciudad ajena y su peaje', () => {
  const origen = asentamiento('O', 'f1', { x: 100, y: 1000 });
  const destino = asentamiento('D', 'f1', { x: 1900, y: 1000 });
  // Una ciudad a un lado de la recta cuya zona la corta: el trazado tiene que pasar por su centro.
  const ajena = asentamiento('X', 'f2', { x: 1000, y: 1150 });
  const zonas = [zonaDe(ajena, 200)];

  it('si el trazado cruza la zona de una ciudad AJENA, pasa por la ciudad y anota el peaje', () => {
    const t = trazarRutaComercial(mapaSintetico(), RED_VACIA, origen, destino, [origen, destino, ajena], zonas)!;
    expect(t.ruta).toContainEqual(ajena.posicion);
    expect(t.ruta[0]).toEqual(origen.posicion);
    expect(t.ruta.at(-1)).toEqual(destino.posicion);
    expect(t.peajes).toHaveLength(1);
    expect(t.peajes[0]!.asentamientoId).toBe('X');
    expect(t.peajes[0]!.progreso).toBeGreaterThan(0.4);
    expect(t.peajes[0]!.progreso).toBeLessThan(0.6);
  });

  it('una ciudad PROPIA no fuerza nada', () => {
    const propia = { ...ajena, faccionId: 'f1' };
    const t = trazarRutaComercial(mapaSintetico(), RED_VACIA, origen, destino, [origen, destino, propia], zonas)!;
    expect(t.ruta).not.toContainEqual(ajena.posicion);
    expect(t.peajes).toEqual([]);
  });

  it('al pasar por la ciudad la caravana deja el peaje en su almacén, y sigue con el resto', () => {
    const o = asentamiento('O', 'f1', { x: 0, y: 0 });
    const d = asentamiento('D', 'f1', { x: 1000, y: 0 });
    const x = asentamiento('X', 'f2', { x: 500, y: 0 }, { cobre: 0 });
    const caravana = caravanaComercialCasiLlegando(o, d, {
      contenido: { cobre: 100 },
      progreso: 0,
      posicionActual: o.posicion,
      peajes: [{ asentamientoId: 'X', progreso: 0.001 }],
    });

    const r = avanzarComercio([o, d, x], [] as Faccion[], [caravana], [], mapaSintetico(), RED_VACIA, [], instanteDeTest(1));

    expect(r.asentamientos.find((a) => a.id === 'X')!.almacen.cobre!.cantidad).toBeCloseTo(100 * PEAJE_PASO.tasa, 10);
    const tras = r.caravanas.find((c) => c.id === caravana.id)!;
    expect(tras.contenido.cobre).toBeCloseTo(100 * (1 - PEAJE_PASO.tasa), 10);
    expect(tras.peajes).toBeUndefined();
    expect(r.eventos.some((e) => typeof e !== 'string' && e.codigo === 'comercio.peaje_paso')).toBe(true);
  });
});

describe('inmunidad dentro de las zonas', () => {
  const propia = asentamiento('P', 'f1', { x: 500, y: 500 });
  const ajena = asentamiento('Q', 'f2', { x: 1500, y: 500 });
  const zonas = [zonaDe(propia, 100), zonaDe(ajena, 100)];

  it('para un atacante, la zona de otro es refugio y la suya no', () => {
    expect(enRefugio(ajena.posicion, zonas, [propia, ajena], 'f1')).toBe(true);
    expect(enRefugio(propia.posicion, zonas, [propia, ajena], 'f1')).toBe(false);
    expect(enRefugio({ x: 1000, y: 500 }, zonas, [propia, ajena], 'f1')).toBe(false);
  });

  it('los bandidos no atacan una caravana dentro de cualquier zona', () => {
    const caravana = { ...caravanaComercialCasiLlegando(propia, ajena), posicionActual: ajena.posicion };
    const campamento = { id: 'c', posicion: ajena.posicion, bosqueId: 'b', asentamientoId: 'Q', poder: 10_000 };
    const r = avanzarAtaquesBandidos([campamento], [caravana], createRng(1), [], zonas);
    expect(r.caravanas.map((c) => c.id)).toEqual([caravana.id]);
    expect(r.eventos).toEqual([]);
    // Control: el mismo ataque fuera de toda zona sí ocurre.
    expect(avanzarAtaquesBandidos([campamento], [caravana], createRng(1), [], []).eventos).not.toEqual([]);
  });
});
