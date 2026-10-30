import type { Point } from './tipos';
import { createRng, derivarSeed } from './rng';
import { anilloEn, type Anillo, type Campamento } from './campamentos';
import { cuotasPorAnillo, type NodoMetal } from './metales';
import type { ClaseTerreno, RecursoMundo, TipoRecurso } from './mundoTipos';
import type { Relieve } from './mundoEstrategico';
import { evaluarElevacion, evaluarTerreno, gradienteElevacion } from './elevacion';
import { esTransitable } from './costeMovimiento';
import { densidadForestal } from './forestacionRegional';
import { evaluarRuido } from './ruido';
import { humedadRegionalEn } from './regional';
import type { MuestraRelieve } from './muestras';

/**
 * Recursos no metálicos (piedra, cobre, hierro) repartidos por anillos (Doc 1.9c) y cantidades iniciales de todos
 * los yacimientos. El estaño y el oro viven en `metales.ts`; aquí solo se convierten (`metalesARecursos`).
 * Unidades del generador, salvo los parámetros marcados como estratégicos.
 */
export type TipoNoMetalico = 'piedra' | 'cobre' | 'hierro';

export const PARAMETROS_RECURSOS = {
  /**
   * Totales de partida: los de Codex con densidad 4 (piedra 140·4, cobre 60·4) para su mundo de 32000×24000,
   * más el hierro a 1,5 veces el cobre (`HIERRO.factorCantidad` de typescript-final). Se escalan por área.
   */
  totalBase: { piedra: 5600, cobre: 240, hierro: 360 } as Record<TipoNoMetalico, number>,
  areaBase: 32000 * 24000,
  /**
   * Reparto por anillo (suma 1 por tipo). El anillo con 0 no recibe ninguno salvo que otro no tenga sitio:
   *  - piedra: abundante en el foco (60 %), el resto en la frontera; la tierra lejana es de minas.
   *  - cobre y hierro: «algo» en el foco (20 %) y el grueso en la frontera (80 %), donde se expande la segunda generación.
   */
  reparto: {
    piedra: { foco: 0.6, frontera: 0.4, lejana: 0 },
    cobre: { foco: 0.2, frontera: 0.8, lejana: 0 },
    hierro: { foco: 0.2, frontera: 0.8, lejana: 0 },
  } as Record<TipoNoMetalico, Record<Anillo, number>>,
  /** Separación mínima (generador) entre yacimientos del mismo tipo (la piedra, más densa, 70) y con cualquier otro. */
  separacionMismoTipo: { piedra: 70, cobre: 250, hierro: 250 } as Record<TipoNoMetalico, number>,
  separacionOtroTipo: 45,
  /** Rejilla de prospección y azar de cada punto (generador), como la «prospección fina» de Codex. */
  pasoPrueba: 150,
  azarPrueba: 75,
  /** Pendiente máxima (×1000, la de Codex) en un yacimiento. */
  pendienteMaxima: 0.35,
  /** El cobre y el hierro (mismo terreno: colina o montaña) caben en el llano solo si no hay colinas: su peso queda por debajo del de cualquier colina. */
  factorLlanoMineral: 0.1,
  /** Calidad mínima para ser candidato. */
  calidadMinima: 0.2,
  /** Penalización por yacimiento del mismo tipo ya colocado en torno al mismo campamento: reparte entre focos. */
  penalizacionPorCampamento: 0.18,
  /**
   * Capa de la tierra lejana (decisión del autor del 2026-10-28): el foco y la frontera quedan como arriba (mínimo garantizado) y,
   * después de todo lo demás, se añaden yacimientos solo en la tierra lejana a esta fracción de la densidad de la frontera (nodos
   * por candidato), con `factorCantidad` veces la cantidad de un nodo normal. Si no caben todos, se ponen los que caben.
   */
  capaLejana: {
    densidadRelativa: { cobre: 0.25, hierro: 0.25, piedra: 0.1 } as Partial<Record<TipoNoMetalico, number>>,
    factorCantidad: 1.5,
  },
};

/** Unidades con las que nace cada tipo de yacimiento (rango sorteado). */
export const RANGO_CANTIDAD: Record<TipoRecurso, { min: number; max: number }> = {
  piedra: { min: 600, max: 1200 },
  cobre: { min: 100, max: 300 },
  hierro: { min: 100, max: 300 },
  estano: { min: 50, max: 150 },
  oro: { min: 150, max: 400 },
};

/**
 * Cantidad inicial entera y determinista de un yacimiento, entre el mínimo y el máximo de su tipo (ambos incluidos),
 * multiplicada por `factor` (el foco lleva un tercio). Depende solo de (seed, tipo, id).
 */
export function cantidadInicial(tipo: TipoRecurso, id: string, seed = 0, factor = 1): number {
  const { min, max } = RANGO_CANTIDAD[tipo];
  const base = min + Math.floor(createRng(derivarSeed(seed, `cantidad-${tipo}-${id}`))() * (max - min + 1));
  return Math.max(1, Math.round(base * factor));
}

/** Estaño y oro de `metales.ts` como recursos del mundo: aplica `cantidadFactor` y propaga `vetaId`. */
export function metalesARecursos(nodos: readonly NodoMetal[], seed = 0): RecursoMundo[] {
  return nodos.map(n => ({
    id: n.id, tipo: n.tipo, posicion: n.posicion, anillo: n.anillo,
    cantidadInicial: cantidadInicial(n.tipo, n.id, seed, n.cantidadFactor),
    ...(n.vetaId ? { vetaId: n.vetaId } : {}),
  }));
}

/** Lo que se consulta del terreno para elegir sitios; `muestreoDeRelieve` lo saca del relieve regional y las pruebas lo inyectan. */
export interface MuestreoRecursos {
  terreno(p: Point): ClaseTerreno;
  /** Pendiente en las unidades de Codex (×1000). */
  pendiente(p: Point): number;
  /** Transitable y con acceso desde el continente. */
  accesible(p: Point): boolean;
  altura(p: Point): number;
  geologia(p: Point): number;
  bosque(p: Point): number;
}

export function muestreoDeRelieve(r: Relieve): MuestreoRecursos {
  const e = r.elevacion;
  // El bucle de candidatos pide terreno, pendiente, acceso, altura y bosque del mismo punto seguidos: se guarda el último.
  // La altura se calcula al entrar en un punto nuevo; pendiente y humedad solo si alguien las pide (el agua no paga el gradiente).
  let cx = NaN, cy = NaN, altura = 0, pendiente: number | undefined, humedad: number | undefined;
  const punto = (p: Point) => {
    if (p.x !== cx || p.y !== cy) { cx = p.x; cy = p.y; altura = evaluarElevacion(e, p); pendiente = undefined; humedad = undefined; }
  };
  const pendienteEn = (p: Point): number => {
    punto(p);
    if (pendiente === undefined) { const g = gradienteElevacion(e, p, 2); pendiente = Math.hypot(g.dx, g.dy) * 1000; }
    return pendiente;
  };
  const muestraEn = (p: Point): MuestraRelieve => {
    const pend = pendienteEn(p);
    humedad ??= humedadRegionalEn(e.regional, p);
    return { altura, pendiente: pend, humedad };
  };
  return {
    terreno: p => { punto(p); return evaluarTerreno(e, p, altura); },
    pendiente: pendienteEn,
    accesible: p => { punto(p); return esTransitable(e, p, altura) && r.conexion.tieneAcceso(p); },
    altura: p => { punto(p); return altura; },
    geologia: p => evaluarRuido(r.suelos.geologia, p),
    bosque: p => densidadForestal(r.forestacion, e, p, muestraEn(p)),
  };
}

const clamp = (x: number) => Math.max(0, Math.min(1, x));
const dist = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

interface Candidato {
  posicion: Point; terreno: ClaseTerreno; altura: number; geologia: number; bosque: number;
  anillo: Anillo; campamento: string;
}

/** Calidad 0..1 de un candidato para un tipo (la de Codex), o 0 si el terreno no vale. */
function calidad(tipo: TipoNoMetalico, c: Candidato): number {
  if (tipo === 'piedra') return clamp(0.4 + (c.altura - 0.4) * 1.2);
  return clamp(0.25 + c.geologia * 0.65); // cobre y hierro; el llano pesa menos (`factorLlanoMineral`, abajo)
}

/**
 * Reparte piedra, cobre e hierro. `ocupados` son posiciones ya tomadas por otros yacimientos (los metales) que
 * quedan a `separacionOtroTipo`. Mismo `seed`, mismo reparto. Si un anillo no tiene sitio para su cuota, el resto pasa a los
 * demás anillos del tipo; si tampoco caben, lanza error (los totales son exactos).
 */
export function repartirRecursosNoMetalicos(
  seed: number,
  campamentos: readonly Campamento[],
  m: MuestreoRecursos,
  limites: { ancho: number; alto: number },
  ocupados: readonly Point[] = [],
  p = PARAMETROS_RECURSOS,
): RecursoMundo[] {
  // Candidatos: una rejilla con azar, sin agua ni cimas ni pendientes fuertes ni sitios sin acceso.
  const azarPunto = createRng(derivarSeed(seed, 'recursos-candidatos-1'));
  const candidatos: Candidato[] = [];
  for (let y = p.pasoPrueba / 2; y < limites.alto; y += p.pasoPrueba) for (let x = p.pasoPrueba / 2; x < limites.ancho; x += p.pasoPrueba) {
    const posicion = { x: x + (azarPunto() - 0.5) * p.azarPrueba, y: y + (azarPunto() - 0.5) * p.azarPrueba };
    const terreno = m.terreno(posicion);
    if (terreno === 'agua' || terreno === 'cima' || m.pendiente(posicion) > p.pendienteMaxima || !m.accesible(posicion)) continue;
    const bosque = m.bosque(posicion), a = anilloEn(campamentos, posicion);
    candidatos.push({ posicion, terreno, altura: m.altura(posicion), geologia: m.geologia(posicion), bosque, anillo: a.anillo, campamento: a.campamento });
  }

  // Índice espacial de lo ya colocado (celdas de 250 ≥ cualquier separación).
  const celda = 250, indice = new Map<string, { posicion: Point; tipo: string }[]>();
  const clave = (q: Point) => `${Math.floor(q.x / celda)},${Math.floor(q.y / celda)}`;
  const anotar = (q: Point, tipo: string) => {
    const k = clave(q), entrada = { posicion: q, tipo }, enCelda = indice.get(k);
    if (enCelda) enCelda.push(entrada); else indice.set(k, [entrada]);
  };
  ocupados.forEach(q => anotar(q, 'otro'));
  const bloqueado = (q: Point, tipo: TipoNoMetalico) => {
    const cx = Math.floor(q.x / celda), cy = Math.floor(q.y / celda);
    for (let j = cy - 1; j <= cy + 1; j++) for (let i = cx - 1; i <= cx + 1; i++)
      for (const o of indice.get(`${i},${j}`) ?? []) if (dist(o.posicion, q) < (o.tipo === tipo ? p.separacionMismoTipo[tipo] : p.separacionOtroTipo)) return true;
    return false;
  };

  const recursos: RecursoMundo[] = [];
  const escala = limites.ancho * limites.alto / p.areaBase;
  // Los más escasos primero: así la piedra, que sobra, no les quita el sitio.
  const capas: (() => void)[] = [];
  for (const tipo of ['cobre', 'hierro', 'piedra'] as TipoNoMetalico[]) {
    const azar = createRng(derivarSeed(seed, `recursos-${tipo}-1`));
    const pool = candidatos.map(c => ({
      c, q: calidad(tipo, c), azar: azar(),
    })).filter(e => e.q > p.calidadMinima)
      .map(e => ({ c: e.c, peso: e.q * (0.4 + e.azar * 0.6) * (tipo === 'piedra' && e.c.bosque >= 0.15 ? 2 : 1)
        * ((tipo === 'cobre' || tipo === 'hierro') && e.c.terreno !== 'colina' && e.c.terreno !== 'montana' ? p.factorLlanoMineral : 1) }));
    const total = Math.round(p.totalBase[tipo] * escala), cuotas = cuotasPorAnillo(total, p.reparto[tipo]);
    const anillos = (Object.keys(cuotas) as Anillo[]).filter(a => p.reparto[tipo][a] > 0).sort((a, b) => p.reparto[tipo][b] - p.reparto[tipo][a]);
    const porCampamento = new Map<string, number>();
    let n = 0;
    const colocar = (anillo: Anillo, cuantos: number, prefijo = '', factor = 1): number => {
      let puestos = 0;
      // Una cola por campamento, de mayor a menor peso: el mejor es la cabeza con más peso tras la penalización.
      const colas = new Map<string, { c: Candidato; peso: number }[]>();
      for (const e of pool) if (e.c.anillo === anillo) {
        const cola = colas.get(e.c.campamento);
        if (cola) cola.push(e); else colas.set(e.c.campamento, [e]);
      }
      const cabezas = [...colas.entries()].map(([camp, cola]) => ({ camp, cola: cola.sort((x, y) => y.peso - x.peso), i: 0 }));
      while (puestos < cuantos) {
        let mejor: (typeof cabezas)[number] | undefined, valor = -Infinity;
        for (const h of cabezas) {
          while (h.i < h.cola.length && bloqueado(h.cola[h.i]!.c.posicion, tipo)) h.i++; // la separación solo crece: se descarta para siempre
          if (h.i >= h.cola.length) continue;
          const v = h.cola[h.i]!.peso / (1 + p.penalizacionPorCampamento * (porCampamento.get(h.camp) ?? 0));
          if (v > valor) { valor = v; mejor = h; }
        }
        if (!mejor) break;
        const c = mejor.cola[mejor.i++]!.c;
        const id = `${tipo}-${prefijo}${n++}`;
        recursos.push({ id, tipo, posicion: c.posicion, anillo, cantidadInicial: cantidadInicial(tipo, id, seed, factor) });
        anotar(c.posicion, tipo); porCampamento.set(c.campamento, (porCampamento.get(c.campamento) ?? 0) + 1); puestos++;
      }
      return puestos;
    };
    let falta = 0;
    for (const a of anillos) falta = cuotas[a] + falta - colocar(a, cuotas[a] + falta);
    // Último recurso: lo que no cupo en sus anillos se coloca en cualquiera (el anillo declarado sigue siendo el real).
    for (const a of [...anillos, ...(['foco', 'frontera', 'lejana'] as Anillo[]).filter(x => !anillos.includes(x))]) { if (falta <= 0) break; falta -= colocar(a, falta); }
    if (falta > 0) throw new Error(`Recursos: faltan ${falta} sitios de ${tipo} (de ${total})`);
    const relativa = p.capaLejana.densidadRelativa[tipo];
    if (relativa) {
      const enFrontera = recursos.filter(r => r.tipo === tipo && r.anillo === 'frontera').length;
      const cuenta = (a: Anillo) => candidatos.filter(c => c.anillo === a).length;
      const extra = Math.round(enFrontera / Math.max(1, cuenta('frontera')) * relativa * cuenta('lejana'));
      // Se coloca al final, con todo lo demás ya puesto: el foco y la frontera no cambian.
      capas.push(() => { n = 0; colocar('lejana', extra, 'l', p.capaLejana.factorCantidad); });
    }
  }
  capas.forEach(c => c());
  return recursos;
}

export function generarRecursosNoMetalicos(relieve: Relieve, campamentos: Campamento[], ocupados: readonly Point[] = []): RecursoMundo[] {
  return repartirRecursosNoMetalicos(relieve.config.seed, campamentos, muestreoDeRelieve(relieve), relieve.config, ocupados);
}
