// Red de caminos (Doc 1.6, `Consideraciones/Rutas_Caravana_Avanzadas_Definicion.md`): los caminos son las
// aristas del grafo de navegación (`world/grafoNavegacion.ts`) que ha recorrido alguna ruta comercial. Dos rutas
// que pisan la misma arista comparten tramo, así que la fusión de caminos sale sola. Aquí vive todo lo que se
// deriva de esa red (pesos, escalones, tramos para pintar, el logro) y el trazado de una ruta comercial, que es
// quien la alimenta.

import { PEAJE_PASO, RED_CAMINOS } from '../constants';
import { minutos, type Instante } from '../domain/tiempo';
import type { Asentamiento, Caravana, Point, RedCaminos, RutaComercial, ZonaInfluencia } from '../domain/types';
import { pointInPolygon } from '../world/geometria';
import { claveArista, extremosDeArista } from '../world/grafoNavegacion';
import type { Mapa } from '../world/mapa';
import { calcularRuta } from '../world/rutas';
import { longitudPolilinea } from './movimiento';

export const RED_VACIA: RedCaminos = { aristas: [], rutas: [] };

/** Aristas de una polilínea de `calcularRuta`, sin repetir. */
export function aristasDeTrazado(puntos: readonly Point[]): string[] {
  const claves = new Set<string>();
  for (let i = 0; i < puntos.length - 1; i++) claves.add(claveArista(puntos[i]!, puntos[i + 1]!));
  return [...claves];
}

// Derivados cacheados por objeto de red: el estado es inmutable, así que una red solo cambia al registrar o podar.
const aristasPorRed = new WeakMap<RedCaminos, ReadonlySet<string>>();
const pesosPorRed = new WeakMap<RedCaminos, ReadonlyMap<string, number>>();

/** Todas las aristas de la red, con o sin rutas: sobre cualquiera de ellas se avanza más rápido (decisión 10). */
export function aristasDeRed(red: RedCaminos): ReadonlySet<string> {
  let aristas = aristasPorRed.get(red);
  if (!aristas) {
    aristas = new Set(red.aristas);
    aristasPorRed.set(red, aristas);
  }
  return aristas;
}

/** Peso de cada arista = rutas vigentes que la recorren (decisión 4). Las de peso 0 no aparecen. */
export function pesosDeRed(red: RedCaminos): ReadonlyMap<string, number> {
  let pesos = pesosPorRed.get(red);
  if (!pesos) {
    const cuenta = new Map<string, number>();
    for (const ruta of red.rutas) for (const a of ruta.aristas) cuenta.set(a, (cuenta.get(a) ?? 0) + 1);
    pesos = cuenta;
    pesosPorRed.set(red, pesos);
  }
  return pesos;
}

/** Registra (o recalcula) la ruta origen→destino con su trazado de este lanzamiento. Las aristas que deja de
 * pisar pierden su peso pero siguen en la red como sendero. */
export function registrarRuta(red: RedCaminos, origen: Asentamiento, destino: Asentamiento, trazado: readonly Point[], instante: Instante): RedCaminos {
  const ruta: RutaComercial = {
    id: `${origen.id}>${destino.id}`,
    origenId: origen.id,
    destinoId: destino.id,
    faccionId: origen.faccionId,
    aristas: aristasDeTrazado(trazado),
    ultimoLanzamiento: instante,
  };
  const conocidas = aristasDeRed(red);
  const nuevas = ruta.aristas.filter((a) => !conocidas.has(a));
  return {
    aristas: nuevas.length > 0 ? [...red.aristas, ...nuevas] : red.aristas,
    rutas: [...red.rutas.filter((r) => r.id !== ruta.id), ruta],
  };
}

/** Quita las rutas caducadas (`RED_CAMINOS.caducidadMinutos` sin lanzar) o con un extremo desaparecido. Devuelve
 * la MISMA red si no cambia nada, para no invalidar los derivados cacheados. */
export function podarRutas(red: RedCaminos, instante: Instante, asentamientoIds: ReadonlySet<string>): RedCaminos {
  const caducidad = minutos(RED_CAMINOS.caducidadMinutos);
  const vigentes = red.rutas.filter(
    (r) => instante - r.ultimoLanzamiento < caducidad && asentamientoIds.has(r.origenId) && asentamientoIds.has(r.destinoId)
  );
  return vigentes.length === red.rutas.length ? red : { ...red, rutas: vigentes };
}

/** Escalón visual por peso: 0 sendero, 1 camino, 2 calzada. */
export function escalonDePeso(peso: number): 0 | 1 | 2 {
  return peso >= RED_CAMINOS.escalonCalzada ? 2 : peso >= RED_CAMINOS.escalonCamino ? 1 : 0;
}

export interface TramoDeRed {
  escalon: 0 | 1 | 2;
  puntos: Point[];
}

/** La red como polilíneas: cadenas de aristas contiguas del mismo escalón (lo que se pinta, §7 de la ficha).
 * `visible` filtra aristas antes de fusionar (niebla). Determinista: sigue el orden de `red.aristas`. */
export function tramosDeRed(red: RedCaminos, visible: (a: Point, b: Point) => boolean = () => true): TramoDeRed[] {
  const pesos = pesosDeRed(red);
  // Adyacencia por nodo, separada por escalón: dos aristas solo se encadenan si son del mismo.
  const porNodo = new Map<string, string[]>();
  const escalonDe = new Map<string, 0 | 1 | 2>();
  const claveNodo = (p: Point) => `${p.x},${p.y}|`;
  for (const arista of red.aristas) {
    const [a, b] = extremosDeArista(arista);
    if (!visible(a, b)) continue;
    const escalon = escalonDePeso(pesos.get(arista) ?? 0);
    escalonDe.set(arista, escalon);
    for (const p of [a, b]) {
      const k = claveNodo(p) + escalon;
      porNodo.set(k, [...(porNodo.get(k) ?? []), arista]);
    }
  }

  const usadas = new Set<string>();
  const tramos: TramoDeRed[] = [];
  const otroExtremo = (arista: string, p: Point): Point => {
    const [a, b] = extremosDeArista(arista);
    return a.x === p.x && a.y === p.y ? b : a;
  };
  /** Avanza desde `p` mientras el nodo sea de paso (exactamente 2 aristas del escalón). */
  const extender = (p: Point, escalon: 0 | 1 | 2, puntos: Point[]): void => {
    for (;;) {
      const vecinas = porNodo.get(claveNodo(p) + escalon) ?? [];
      const siguiente = vecinas.length === 2 ? vecinas.find((a) => !usadas.has(a)) : undefined;
      if (!siguiente) return;
      usadas.add(siguiente);
      p = otroExtremo(siguiente, p);
      puntos.push(p);
    }
  };

  for (const arista of escalonDe.keys()) {
    if (usadas.has(arista)) continue;
    usadas.add(arista);
    const escalon = escalonDe.get(arista)!;
    const [a, b] = extremosDeArista(arista);
    const haciaB: Point[] = [b];
    extender(b, escalon, haciaB);
    const haciaA: Point[] = [a];
    extender(a, escalon, haciaA);
    tramos.push({ escalon, puntos: [...haciaA.reverse(), ...haciaB] });
  }
  return tramos;
}

/**
 * Logro `logistica_campana` (BA-006 D30, decisión 11): ¿hay una arista FUERA de toda zona de influencia que
 * recorran al menos `RED_CAMINOS.logroRutas` rutas de al menos `logroFacciones` Facciones distintas? Consulta pura;
 * la engancha el sistema de tecnología cuando exista `logistica_campana` (Era IV, aún sin código).
 */
export function caminoCompartidoAbierto(red: RedCaminos, zonas: readonly ZonaInfluencia[]): boolean {
  const porArista = new Map<string, { rutas: number; facciones: Set<string> }>();
  for (const ruta of red.rutas) {
    for (const a of ruta.aristas) {
      const uso = porArista.get(a) ?? { rutas: 0, facciones: new Set<string>() };
      uso.rutas++;
      uso.facciones.add(ruta.faccionId);
      porArista.set(a, uso);
    }
  }
  for (const [arista, uso] of porArista) {
    if (uso.rutas < RED_CAMINOS.logroRutas || uso.facciones.size < RED_CAMINOS.logroFacciones) continue;
    const [a, b] = extremosDeArista(arista);
    const medio = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    if (!zonas.some((z) => pointInPolygon(medio, z.poligono))) return true;
  }
  return false;
}

export interface TrazadoComercial {
  ruta: Point[];
  /** Ciudades ajenas por las que se fuerza el paso, en orden, con el `progreso` en que se llega a cada una. */
  peajes: NonNullable<Caravana['peajes']>;
}

/**
 * Trazado de una caravana COMERCIAL de `origen` a `destino` (decisiones 5 y 6): se arrima a los caminos con peso
 * y, si cruza la zona de influencia de un asentamiento AJENO (otra Facción que la del origen), se fuerza el paso
 * por esa ciudad —A→B→C—, una vez por ciudad y en el orden en que las encuentra. Una ciudad propia no fuerza
 * nada. `undefined` = no hay ruta por tierra.
 *
 * Ponytail: el tramo A→B no se vuelve a examinar; si roza otra zona ajena antes de llegar a B, no para en ella.
 */
export function trazarRutaComercial(
  mapa: Mapa,
  red: RedCaminos,
  origen: Asentamiento,
  destino: Asentamiento,
  asentamientos: readonly Asentamiento[],
  zonas: readonly ZonaInfluencia[]
): TrazadoComercial | undefined {
  const opciones = { pesos: pesosDeRed(red), pasosRio: asentamientos.map((a) => a.posicion) };
  const porId = new Map(asentamientos.map((a) => [a.id, a]));
  const pendientes = zonas.filter((z) => {
    const b = porId.get(z.asentamientoId);
    return b !== undefined && b.id !== origen.id && b.id !== destino.id && b.faccionId !== origen.faccionId;
  });

  const ruta: Point[] = [origen.posicion];
  const paradas: { asentamientoId: string; distancia: number }[] = [];
  let actual = origen.posicion;
  for (;;) {
    const tramo = calcularRuta(mapa, actual, destino.posicion, opciones);
    if (!tramo) return undefined;
    const zona = tramo.slice(1).reduce<ZonaInfluencia | undefined>(
      (hallada, p) => hallada ?? pendientes.find((z) => pointInPolygon(p, z.poligono)),
      undefined
    );
    const ciudad = zona && porId.get(zona.asentamientoId)!;
    const hastaCiudad = ciudad && calcularRuta(mapa, actual, ciudad.posicion, opciones);
    if (!ciudad || !hastaCiudad) {
      ruta.push(...tramo.slice(1));
      break;
    }
    ruta.push(...hastaCiudad.slice(1));
    paradas.push({ asentamientoId: ciudad.id, distancia: longitudPolilinea(ruta) });
    pendientes.splice(pendientes.indexOf(zona!), 1);
    actual = ciudad.posicion;
  }
  const total = Math.max(1, longitudPolilinea(ruta));
  return { ruta, peajes: paradas.map((p) => ({ asentamientoId: p.asentamientoId, progreso: p.distancia / total })) };
}

/** Peaje en especie al pasar por una ciudad (decisión 6): `PEAJE_PASO.tasa` de cada recurso que lleva. */
export function peajeDe(contenido: Readonly<Record<string, number>>): Record<string, number> {
  return Object.fromEntries(
    Object.entries(contenido)
      .map(([recurso, cantidad]) => [recurso, cantidad * PEAJE_PASO.tasa] as const)
      .filter(([, q]) => q > 0)
  );
}
