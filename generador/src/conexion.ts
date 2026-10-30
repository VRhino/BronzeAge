import type { Point } from './tipos';
import type { CampoElevacion } from './elevacion';
import { esTransitable } from './costeMovimiento';
import type { RelieveRegional } from './regional';
import type { CeldasMuestreadas } from './celdas';

/** Conexión terrestre del relieve final, incluyendo lagos: comprueba aristas, no solo centros.
 * Las islas secundarias no son objetivos de conectividad. */
export function comprobarConexion(campo: CampoElevacion) {
  const regional = campo.regional;
  const { ancho, alto } = regional.limites;
  const n = 160, dx = ancho / n, dy = alto / n;
  const punto = (i: number): Point => ({ x: (i % n + 0.5) * dx, y: (Math.floor(i / n) + 0.5) * dy });
  const valido = new Uint8Array(n * n), visitado = new Uint8Array(n * n);
  for (let i = 0; i < valido.length; i++) valido[i] = Number(esTransitable(campo, punto(i)));
  // Acceso representativo próximo al centro de cada zona, sobre terreno válido.
  const accesos = regional.areas.map((area) => {
    let mejor = -1, distancia = Infinity;
    for (let i = 0; i < valido.length; i++) if (valido[i]) {
      const p = punto(i), d = Math.hypot(p.x - area.centro.x, p.y - area.centro.y);
      if (d < distancia) { distancia = d; mejor = i; }
    }
    return { id: area.id, indice: mejor, distancia };
  });
  const primero = accesos[0]?.indice ?? -1;
  const cola = primero < 0 ? [] : [primero];
  if (primero >= 0) visitado[primero] = 1;
  for (let k = 0; k < cola.length; k++) {
    const i = cola[k]!, x = i % n, y = Math.floor(i / n);
    const vecinos = [x > 0 ? i - 1 : -1, x < n - 1 ? i + 1 : -1, y > 0 ? i - n : -1, y < n - 1 ? i + n : -1];
    for (const j of vecinos) {
      if (j < 0 || !valido[j] || visitado[j]) continue;
      const a = punto(i), b = punto(j);
      const pasos = Math.ceil(Math.hypot(a.x - b.x, a.y - b.y) / 30);
      let libre = true;
      for (let paso = 1; paso < pasos; paso++) {
        const t = paso / pasos;
        if (!esTransitable(campo, { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t })) { libre = false; break; }
      }
      if (libre) { visitado[j] = 1; cola.push(j); }
    }
  }
  // Acceso al continente medido: la pendiente local sola también acepta islas secundarias y repisas aisladas.
  const tieneAcceso = (p: Point): boolean => {
    // La muestra k = 0 de cada segmento es siempre `p`: se comprueba una vez y los segmentos empiezan en k = 1.
    if (!esTransitable(campo, p)) return false;
    const x = Math.floor(p.x / dx), y = Math.floor(p.y / dy);
    const vecinas: { i: number; d: number }[] = [];
    for (let sy = Math.max(0, y - 2); sy <= Math.min(n - 1, y + 2); sy++)
      for (let sx = Math.max(0, x - 2); sx <= Math.min(n - 1, x + 2); sx++) {
        const i = sy * n + sx;
        if (visitado[i]) { const q = punto(i); vecinas.push({ i, d: Math.hypot(q.x - p.x, q.y - p.y) }); }
      }
    // Basta una celda que sirva: se prueban de la más cercana a la más lejana (desempate por índice) y casi siempre vale la primera.
    vecinas.sort((a, b) => a.d - b.d || a.i - b.i);
    for (const { i, d } of vecinas) {
      const q = punto(i), pasos = Math.max(1, Math.ceil(d / 30));
      let libre = true;
      for (let k = 1; k <= pasos; k++) if (!esTransitable(campo, {
        x: p.x + (q.x - p.x) * k / pasos, y: p.y + (q.y - p.y) * k / pasos })) { libre = false; break; }
      if (libre) return true;
    }
    return false;
  };
  const conectadas = accesos.filter((a) => a.indice >= 0 && a.distancia <= Math.min(ancho, alto) * 0.025 && visitado[a.indice]).map((a) => a.id);
  return { tieneAcceso, zonas: accesos.length, conectadas: conectadas.length, ids: conectadas,
    pasoMalla: Math.max(dx, dy), pasoArista: 30, superficieConectadaKm2: cola.length * dx * dy / 1e6 };
}

/**
 * Conexión terrestre medida sobre la rejilla de celdas del artefacto (la autoridad de transitabilidad): 8 vecinos, la
 * diagonal solo si las dos ortogonales están libres, como el grafo de navegación. Con el relieve a escala de pantalla los
 * valles y las crestas son más finos que la malla gruesa de `comprobarConexion`, que ya no ve los pasos entre ellos.
 * La tierra principal es la mayor componente; una zona está conectada si tiene tierra principal a menos de un 2,5 % del
 * lado del mapa de su centro (no basta la celda más cercana: puede caer en una bolsa entre crestas).
 * `tieneAcceso(p)`: la celda de `p` es de la tierra principal.
 */
export function conexionPorCeldas(regional: RelieveRegional, celdas: CeldasMuestreadas) {
  const { columnas, filas, lado, transitable } = celdas, n = columnas * filas, { ancho, alto } = regional.limites;
  const celdaDe = (p: Point) => { const cx = Math.floor(p.x / lado), cy = Math.floor(p.y / lado); return cx < 0 || cy < 0 || cx >= columnas || cy >= filas ? -1 : cy * columnas + cx; };
  // Componentes de la tierra transitable; la principal, la mayor (empate: la primera en orden de fila).
  const componente = new Int32Array(n).fill(-1), cola = new Int32Array(n);
  let principal = -1, mayor = 0;
  for (let semilla = 0, c = 0; semilla < n; semilla++) {
    if (!transitable[semilla] || componente[semilla]! >= 0) continue;
    let fin = 0;
    componente[semilla] = c; cola[fin++] = semilla;
    for (let k = 0; k < fin; k++) {
      const i = cola[k]!, x = i % columnas, y = (i - x) / columnas;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
        const x2 = x + dx, y2 = y + dy;
        if ((!dx && !dy) || x2 < 0 || y2 < 0 || x2 >= columnas || y2 >= filas) continue;
        const j = y2 * columnas + x2;
        if (componente[j]! >= 0 || !transitable[j] || (dx && dy && !(transitable[y * columnas + x2] && transitable[y2 * columnas + x]))) continue;
        componente[j] = c; cola[fin++] = j;
      }
    }
    if (fin > mayor) { mayor = fin; principal = c; }
    c++;
  }
  const enPrincipal = (i: number) => i >= 0 && componente[i] === principal;
  const maxima = Math.min(ancho, alto) * 0.025, r = Math.ceil(maxima / lado);
  const accesos = regional.areas.map((area) => {
    const c = celdaDe(area.centro), cx = c % columnas, cy = Math.floor(c / columnas);
    let mejor = -1, distancia = Infinity;
    for (let y = cy - r; y <= cy + r; y++) for (let x = cx - r; x <= cx + r; x++) {
      if (x < 0 || y < 0 || x >= columnas || y >= filas || !enPrincipal(y * columnas + x)) continue;
      const d = Math.hypot((x + 0.5) * lado - area.centro.x, (y + 0.5) * lado - area.centro.y);
      if (d < distancia) { distancia = d; mejor = y * columnas + x; }
    }
    return { id: area.id, indice: mejor, distancia };
  });
  const tieneAcceso = (p: Point): boolean => enPrincipal(celdaDe(p));
  const conectadas = accesos.filter((a) => a.indice >= 0 && a.distancia <= maxima).map((a) => a.id);
  return { tieneAcceso, zonas: accesos.length, conectadas: conectadas.length, ids: conectadas,
    pasoMalla: lado, pasoArista: lado, superficieConectadaKm2: mayor * lado * lado / 1e6 };
}
