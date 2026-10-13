import type { Point } from '../src/tipos';
import type { CampoElevacion } from '../src/types';
import { evaluarSuelo, type CampoSuelos, type RecursoRegional } from '../src/suelosRegional';
import { areaRegionalEn } from '../src/regional';
import { gradienteElevacion } from '../src/elevacion';
import { costeEnPunto, esTransitable } from '../src/costeMovimiento';
import { evaluarForestal, type CampoForestal } from '../src/forestacionRegional';
import { distanciaASegmento } from '../src/colocacion';

/** Diagnóstico de rutas ponderadas, no carreteras ni trazado de asentamientos.
 * Malla de 250 m aproximadamente; verifica cada enlace cada 30 m. */
export function analizarEstrategia(campo: CampoElevacion, bosque: CampoForestal, recursos: RecursoRegional[], suelos: CampoSuelos, opciones: { analizarDemandaRegional?: boolean } = {}) {
  const { ancho, alto } = campo.regional!.limites;
  const nx = Math.ceil(ancho / 250), ny = Math.ceil(alto / 250), dx = ancho / nx, dy = alto / ny;
  const punto = (i: number): Point => ({ x: (i % nx + 0.5) * dx, y: (Math.floor(i / nx) + 0.5) * dy });
  const enlaces: { j: number; coste: number }[][] = Array.from({ length: nx * ny }, () => []);
  const densidades = new Float64Array(nx * ny);
  const pesos = Array.from({ length: nx * ny }, (_, i) => {
    const p = punto(i);
    if (!esTransitable(campo, p)) return Infinity;
    const f = evaluarForestal(bosque, campo, p); densidades[i] = f.densidad;
    return costeEnPunto(campo, p) * f.costeMovimiento;
  });
  const segmentoLibre = (a: Point, b: Point) => {
    const pasos = Math.max(1, Math.ceil(Math.hypot(a.x - b.x, a.y - b.y) / 30));
    for (let t = 0; t <= pasos; t++) if (!esTransitable(campo, { x: a.x + (b.x - a.x) * t / pasos, y: a.y + (b.y - a.y) * t / pasos })) return false;
    return true;
  };
  for (let i = 0; i < pesos.length; i++) if (Number.isFinite(pesos[i])) {
    const x = i % nx, y = Math.floor(i / nx);
    for (const j of [x + 1 < nx ? i + 1 : -1, y + 1 < ny ? i + nx : -1]) {
      if (j < 0 || !Number.isFinite(pesos[j]) || !segmentoLibre(punto(i), punto(j))) continue;
      const coste = Math.hypot(punto(i).x - punto(j).x, punto(i).y - punto(j).y) * (pesos[i]! + pesos[j]!) / 2;
      enlaces[i]!.push({ j, coste }); enlaces[j]!.push({ j: i, coste });
    }
  }
  const acceso = (p: Point) => {
    const x = Math.floor(p.x / dx), y = Math.floor(p.y / dy);
    const opciones: number[] = [];
    for (let sy = Math.max(0, y - 2); sy <= Math.min(ny - 1, y + 2); sy++)
      for (let sx = Math.max(0, x - 2); sx <= Math.min(nx - 1, x + 2); sx++) if (enlaces[sy * nx + sx]!.length) opciones.push(sy * nx + sx);
    opciones.sort((a, b) => Math.hypot(punto(a).x - p.x, punto(a).y - p.y) - Math.hypot(punto(b).x - p.x, punto(b).y - p.y));
    return opciones.find(i => segmentoLibre(p, punto(i))) ?? -1;
  };
  const dijkstra = (inicio: number | number[], bloqueo?: Point, previo?: Int32Array) => {
    const d = new Float64Array(nx * ny).fill(Infinity), heap: { i: number; d: number }[] = [];
    const push = (v: { i: number; d: number }) => {
      heap.push(v); let k = heap.length - 1;
      while (k > 0) { const p = (k - 1) >> 1; if (heap[p]!.d <= v.d) break; heap[k] = heap[p]!; k = p; } heap[k] = v;
    };
    for (const i of typeof inicio === 'number' ? [inicio] : inicio) {
      if (i < 0 || !Number.isFinite(pesos[i])) continue;
      d[i] = 0; if(previo)previo[i]=i; push({ i, d: 0 });
    }
    while (heap.length) {
      const v = heap[0]!, ultimo = heap.pop()!;
      if (heap.length) {
        let k = 0;
        while (k * 2 + 1 < heap.length) { let c = k * 2 + 1; if (c + 1 < heap.length && heap[c + 1]!.d < heap[c]!.d) c++; if (heap[c]!.d >= ultimo.d) break; heap[k] = heap[c]!; k = c; } heap[k] = ultimo;
      }
      if (v.d !== d[v.i]) continue;
      for (const e of enlaces[v.i]!) {
        if (bloqueo && distanciaASegmento(bloqueo, punto(v.i), punto(e.j)) < 400) continue;
        const coste = v.d + e.coste;
        if (coste < d[e.j]!) { d[e.j] = coste; if(previo)previo[e.j]=v.i; push({ i: e.j, d: coste }); }
      }
    }
    return d;
  };
  const accesoRegional = (p: Point) => {
    const directo = acceso(p);
    if (directo >= 0) return directo;
    let mejor = -1, distancia = Math.min(ancho, alto) * 0.025;
    for (let i = 0; i < pesos.length; i++) if (enlaces[i]!.length) {
      const q = punto(i), d = Math.hypot(q.x - p.x, q.y - p.y);
      if (d < distancia) { mejor = i; distancia = d; }
    }
    return mejor;
  };
  const accesosRecursos = recursos.map(r => acceso(r.posicion));
  const areas = campo.regional!.areas.map(a => {
    const origen = accesoRegional(a.centro), distancias = dijkstra(origen);
    const cercanos: Record<string, number | null> = {};
    for (const tipo of ['piedra', 'cobre', 'estano', 'oro', 'ganaderia']) {
      let mejor = Infinity;
      recursos.forEach((r, i) => { const j = accesosRecursos[i]!; if (r.tipo === tipo && j >= 0) mejor = Math.min(mejor, distancias[j]!); });
      cercanos[tipo] = Number.isFinite(mejor) ? Math.round(mejor) : null;
    }
    const tin = recursos.map((r, i) => ({ r, distancia: accesosRecursos[i]! >= 0 ? distancias[accesosRecursos[i]!]! : Infinity }))
      .filter(v => v.r.tipo === 'estano' && Number.isFinite(v.distancia)).sort((a, b) => a.distancia - b.distancia);
    const menor = tin.find(v => v.r.categoriaEstano === 'aislado');
    const grande = tin.find(v => v.r.categoriaEstano === 'cluster');
    return { area: a.id, acceso: origen >= 0, recursos: cercanos, estano: {
      menor: menor ? Math.round(menor.distancia) : null, grande: grande ? Math.round(grande.distancia) : null,
      alternativa: tin[1] ? Math.round(tin[1].distancia) : null,
      fuenteCercana: tin[0]?.r.id ?? null, fuenteAlternativa: tin[1]?.r.id ?? null } };

  });
  const desdePrincipal = dijkstra(accesoRegional(campo.regional!.areas[0]!.centro));
  const recursosAislados = recursos.filter((_, i) => accesosRecursos[i]! < 0 || !Number.isFinite(desdePrincipal[accesosRecursos[i]!]!)).map(r => r.id);
  // Prospección independiente de la malla del visor. No coloca asentamientos ni
  // certifica su economía: mide oportunidades con accesos a tres fuentes básicas.
  const regiones = campo.regional!.areas.map(a => ({ area: a.id, nombre: a.nombre,
    superficieKm2: 0, habitableKm2: 0, fertilKm2: 0, bosqueKm2: 0, pastosKm2: 0,
    abastecidaKm2: 0, sectoresOportunidad: 0, recursos: {} as Record<string, number>,
    especialidades: [] as string[], alertas: [] as string[] }));
  const fuentesMadera: number[] = [], fuentesAlimento: number[] = [];
  const habitables: number[] = [], regionCelda = new Int16Array(nx * ny).fill(-1);
  const areaCelda = dx * dy / 1e6;
  for (let i = 0; i < pesos.length; i++) {
    if (!Number.isFinite(desdePrincipal[i])) continue;
    const p = punto(i), area = areaRegionalEn(campo.regional!, p).id, r = regiones[area]!;
    regionCelda[i] = area; r.superficieKm2 += areaCelda;
    const g = gradienteElevacion(campo, p, 2), pendiente = Math.hypot(g.dx, g.dy) * 1000;
    const suelo = evaluarSuelo(suelos, campo, p, densidades[i]);
    if (densidades[i]! >= 0.35) { fuentesMadera.push(i); r.bosqueKm2 += areaCelda; }
    const agricola = suelo.fertilidad >= 0.55 && densidades[i]! < 0.35;
    const pasto = suelo.pastos >= 0.55 && densidades[i]! < 0.35;
    if (agricola) r.fertilKm2 += areaCelda;
    if (pasto) r.pastosKm2 += areaCelda;
    if (agricola || pasto) fuentesAlimento.push(i);
    if (pendiente <= 0.12 && densidades[i]! < 0.6) { habitables.push(i); r.habitableKm2 += areaCelda; }
  }
  const madera = dijkstra(fuentesMadera), alimento = dijkstra(fuentesAlimento);
  const piedra = dijkstra(accesosRecursos.filter((j, i) => recursos[i]!.tipo === 'piedra' && j >= 0 && Number.isFinite(desdePrincipal[j])));
  const oportunidades = new Set<string>();
  const cobertura = { maderaKm2: 0, alimentoKm2: 0, piedraKm2: 0 };
  for (const i of habitables) {
    if (madera[i]! <= 2000) cobertura.maderaKm2 += areaCelda;
    if (alimento[i]! <= 2000) cobertura.alimentoKm2 += areaCelda;
    if (piedra[i]! <= 2000) cobertura.piedraKm2 += areaCelda;
    if (madera[i]! > 2000 || alimento[i]! > 2000 || piedra[i]! > 2000) continue;
    const r = regiones[regionCelda[i]!]!, p = punto(i);
    r.abastecidaKm2 += areaCelda;
    const sector = `${Math.floor(p.x / 1500)},${Math.floor(p.y / 1500)}`;
    if (!oportunidades.has(sector)) { oportunidades.add(sector); r.sectoresOportunidad++; }
  }
  for (const r of recursos) if (!recursosAislados.includes(r.id)) regiones[r.area]!.recursos[r.tipo] = (regiones[r.area]!.recursos[r.tipo] ?? 0) + 1;
  const concentracion = ['cobre', 'estano', 'oro', 'piedra'].map(tipo => {
    const cuentas = regiones.map(r => r.recursos[tipo] ?? 0), total = cuentas.reduce((a, b) => a + b, 0);
    return { tipo, total, regiones: cuentas.filter(n => n > 0).length, maxFraccion: total ? Math.max(...cuentas) / total : 0 };
  });
  for (const r of regiones) {
    if (r.fertilKm2 > r.superficieKm2 * 0.12) r.especialidades.push('agrícola');
    if (r.bosqueKm2 > r.superficieKm2 * 0.3) r.especialidades.push('forestal');
    if (r.pastosKm2 > r.superficieKm2 * 0.25) r.especialidades.push('ganadera');
    if (['cobre','estano','oro'].some(t => (r.recursos[t] ?? 0) >= 3)) r.especialidades.push('minera');
    if (!r.especialidades.length) r.especialidades.push('mixta');
    if (r.habitableKm2 && r.abastecidaKm2 / r.habitableKm2 < 0.25) r.alertas.push('poca superficie con abastecimiento cercano');
    if (!r.recursos.piedra) r.alertas.push('sin piedra local');
    if (r.bosqueKm2 < 0.5) r.alertas.push('escasez de bosque');
    if (r.fertilKm2 + r.pastosKm2 < 1) r.alertas.push('poco suelo alimentario abierto');
  }
  const estanoPublicado = recursos.filter(r => r.tipo === 'estano' && !recursosAislados.includes(r.id));
  const ofertaEstano = estanoPublicado.reduce((s, r) => s + (r.ofertaRelativa ?? 1), 0);
  const estano = { pequenos: estanoPublicado.filter(r => r.categoriaEstano === 'aislado').length,
    grandes: estanoPublicado.filter(r => r.categoriaEstano === 'cluster').length,
    clusters: new Set(estanoPublicado.filter(r => r.clusterEstano).map(r => r.clusterEstano)).size,
    regionesConNodos: new Set(estanoPublicado.map(r => r.area)).size,
    cuotaGrandes: estanoPublicado.filter(r => r.categoriaEstano === 'cluster').reduce((s, r) => s + (r.ofertaRelativa ?? 1), 0) / (ofertaEstano || 1),
    regionesConAccesoMenor: areas.filter(a => a.estano.menor !== null).length,
    maxDistanciaMenor: Math.max(...areas.map(a => a.estano.menor ?? Infinity)),
    regionesConAlternativa: areas.filter(a => a.estano.alternativa !== null).length };
  const balance = { estano, regiones, concentracion, cobertura, sectoresOportunidad: oportunidades.size,
    objetivoAsentamientos: 100, alcanceBasico: 2000, ladoSector: 1500,
    nota: 'Sectores de prospección, no plazas de asentamiento. Falta validar tamaños, producción, demanda y competencia por fuentes.' };
  const pasos = campo.regional!.barreras.flatMap(b => b.pasos.map(p => {
    const r = b.ancho * 2.8 + 350;
    const buscarLado = (signo: number) => {
      for (const fraccion of [1, 0.85, 0.7, 0.55, 0.4]) {
        const distancia = r * fraccion;
        if (distancia < 450) continue;
        const q = { x: p.centro.x + signo * p.tangente.y * distancia, y: p.centro.y - signo * p.tangente.x * distancia };
        if (q.x < 0 || q.y < 0 || q.x > ancho || q.y > alto) continue;
        const j = acceso(q);
        if (j >= 0 && Math.hypot(punto(j).x - p.centro.x, punto(j).y - p.centro.y) > 400) return j;
      }
      return -1;
    };
    const a = buscarLado(-1), z = buscarLado(1);
    const base = a < 0 || z < 0 ? Infinity : dijkstra(a)[z]!;
    const rodeo = a < 0 || z < 0 ? Infinity : dijkstra(a, p.centro)[z]!;
    return { ...p.centro, recorrido: Number.isFinite(base) ? Math.round(base) : null,
      rodeo: Number.isFinite(rodeo) ? Math.round(rodeo) : null,
      factor: Number.isFinite(base) && Number.isFinite(rodeo) && base > 0 ? rodeo / base : null };
  }));
  // Proxy demand: shortest weighted paths between geographic anchors, not actual caravan traffic.
  const demandaRegional = opciones.analizarDemandaRegional ? (() => {
    const anchors=campo.regional!.areas.map(a=>accesoRegional(a.centro));
    const through=campo.regional!.areas.map(a=>({area:a.id,paresDePaso:0}));
    const routes:{from:number;to:number;cost:number;through:number[];points:Point[]}[]=[];
    for(let a=0;a<anchors.length;a++){
      if(anchors[a]!<0)continue;
      const prev=new Int32Array(nx*ny).fill(-1),dist=dijkstra(anchors[a]!,undefined,prev);
      for(let b=a+1;b<anchors.length;b++){
        const end=anchors[b]!;if(end<0||!Number.isFinite(dist[end]))continue;
        const crossed=new Set<number>(),cells:number[]=[];let i=end,steps=0;
        while(i!==anchors[a]&&i>=0&&steps++<nx*ny){cells.push(i);crossed.add(areaRegionalEn(campo.regional!,punto(i)).id);i=prev[i]!;}
        if(i!==anchors[a])continue;
        crossed.delete(campo.regional!.areas[a]!.id);crossed.delete(campo.regional!.areas[b]!.id);
        cells.push(anchors[a]!);cells.reverse();
        const path=cells.filter((v,j)=>j===0||j===cells.length-1||(v-cells[j-1]!)!==(cells[j+1]!-v)).map(punto);
        const ids=[...crossed].sort((x,y)=>x-y);ids.forEach(id=>through.find(r=>r.area===id)!.paresDePaso++);
        routes.push({from:campo.regional!.areas[a]!.id,to:campo.regional!.areas[b]!.id,cost:Math.round(dist[end]!),through:ids,points:path});
      }
    }
    return {through,routes,note:'Rutas entre anclas geográficas. No tráfico real, peajes ni permiso de extracción.'};
  })() : undefined;
  return { pasoMalla: Math.max(dx, dy), areas, recursosAislados, pasos, balance, demandaRegional };
}
