import type { Point } from './tipos';
import type { Limites } from './colocacion';
import { esTransitable } from './costeMovimiento';
import { evaluarTerreno } from './elevacion';
import type { CampoElevacion } from './types';

const LADO = 40;
const MAX_REPARACIONES = 5;

interface Analisis {
  tierra: boolean[];
  transitable: boolean[];
  grupo: number[];
  componentes: number[][];
}

const centro = (i: number, limites: Limites): Point => ({
  x: (i % LADO + 0.5) * limites.ancho / LADO,
  y: (Math.floor(i / LADO) + 0.5) * limites.alto / LADO,
});

function vecinos(i: number): number[] {
  const x = i % LADO, y = Math.floor(i / LADO);
  return [x > 0 ? i - 1 : -1, x < LADO - 1 ? i + 1 : -1,
    y > 0 ? i - LADO : -1, y < LADO - 1 ? i + LADO : -1].filter((v) => v >= 0);
}

function analizar(campo: CampoElevacion, limites: Limites): Analisis {
  const tierra: boolean[] = [];
  const transitable: boolean[] = [];
  for (let i = 0; i < LADO * LADO; i++) {
    const p = centro(i, limites);
    const seca = evaluarTerreno(campo, p) !== 'agua';
    tierra.push(seca);
    transitable.push(seca && esTransitable(campo, p));
  }
  const grupo = Array(LADO * LADO).fill(-1) as number[];
  const componentes: number[][] = [];
  for (let i = 0; i < grupo.length; i++) {
    if (!transitable[i] || grupo[i] !== -1) continue;
    const id = componentes.length;
    const cola = [i]; grupo[i] = id;
    for (let j = 0; j < cola.length; j++) {
      for (const v of vecinos(cola[j]!)) {
        if (!transitable[v] || grupo[v] !== -1) continue;
        grupo[v] = id; cola.push(v);
      }
    }
    componentes.push(cola);
  }
  return { tierra, transitable, grupo, componentes };
}

/** Busca la conexión seca más corta entre dos componentes. Una isla no se convierte en istmo. */
function buscarConexion(analisis: Analisis, origen: number[], destino: Set<number>,
  campo: CampoElevacion, limites: Limites): Point[] | undefined {
  const previo = Array(LADO * LADO).fill(-1) as number[];
  const cola = [...origen];
  for (const i of origen) previo[i] = i;
  let llegada = -1;
  for (let k = 0; k < cola.length && llegada < 0; k++) {
    const i = cola[k]!;
    for (const v of vecinos(i)) {
      if (!analisis.tierra[v] || previo[v]! >= 0) continue;
      const a = centro(i, limites), b = centro(v, limites);
      if (evaluarTerreno(campo, { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }) === 'agua') continue;
      previo[v] = i;
      if (destino.has(v)) { llegada = v; break; }
      cola.push(v);
    }
  }
  if (llegada < 0) return undefined;
  const celdas = [llegada];
  while (previo[celdas[celdas.length - 1]!] !== celdas[celdas.length - 1]!)
    celdas.push(previo[celdas[celdas.length - 1]!]!);
  celdas.reverse();
  // Colapsar segmentos rectos limita tamaño serializado y coste de consulta.
  const puntos = celdas.filter((v, i) => i === 0 || i === celdas.length - 1
    || (celdas[i + 1]! - v) !== (v - celdas[i - 1]!)).map((v) => centro(v, limites));
  return puntos.length >= 2 ? puntos : undefined;
}

/** Repara como máximo cinco separaciones continentales causadas por crestas.
 * Se conservan las islas sin conexión seca y se descarta cualquier reparación que
 * no aumente la mayor componente. El campo y la estructura comparten referencia. */
export function repararConectividad(campo: CampoElevacion, limites: Limites): number {
  const estructura = campo.estructura;
  if (!estructura || estructura.perfil === 'archipielago') return 0;
  estructura.conectores ??= [];
  let reparaciones = 0;
  for (; reparaciones < MAX_REPARACIONES;) {
    const antes = analizar(campo, limites);
    const orden = antes.componentes.map((c, id) => ({ c, id })).sort((a, b) => b.c.length - a.c.length);
    if (!orden.length) break;
    const principal = orden[0]!;
    const total = antes.transitable.filter(Boolean).length;
    let reparada = false;
    for (const candidata of orden.slice(1)) {
      if (candidata.c.length < total * 0.03) break;
      const puntos = buscarConexion(antes, candidata.c, new Set(principal.c), campo, limites);
      if (!puntos) continue;
      estructura.conectores.push({ puntos, radio: Math.min(limites.ancho, limites.alto) * 0.075, elevacion: 0.51 });
      const despues = analizar(campo, limites);
      const mayor = Math.max(...despues.componentes.map((c) => c.length), 0);
      if (mayor <= principal.c.length) { estructura.conectores.pop(); continue; }
      reparaciones++;
      reparada = true;
      break;
    }
    if (!reparada) break;
  }
  return reparaciones;
}
