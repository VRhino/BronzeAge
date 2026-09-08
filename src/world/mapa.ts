// Fachada de consulta del mapa: la ÚNICA vía por la que el motor y la interfaz hablan con el mundo.
//
// Sustituye al acceso crudo a `world.recursos` / `world.bosques` / `world.fertilidadEn`, que estaba
// repartido por `construction.ts`, `settlement.ts`, `asentamientoQuery.ts` y `ui/canvas.ts`. Dos motivos:
//
//  1. RENDIMIENTO. Resolver un `fuenteId` era un `Array.find` lineal ejecutado por cada edificio extractor,
//     en cada tick, y otra vez para pintar el panel de producción. Aquí es un `Map.get`. Las consultas por
//     radio/polígono usan un índice de celdas en vez de recorrer todos los nodos del mapa.
//  2. UN SOLO SITIO. El criterio de "un bosque está al alcance" o "cuántas Leñeras caben" estaba escrito
//     por duplicado en dos archivos del motor, con un comentario admitiendo que debían coincidir. Ahora
//     el mapa es quien responde a eso, y no hay dos versiones que puedan divergir.
//
// Lo que la fachada NO hace: decidir política de juego. No sabe qué es una Leñera ni cuándo conviene
// construirla; sabe qué bosques hay, cuáles tienen hueco y qué punto suyo cae dentro de un polígono dado.
// Los filtros que dependen del estado del asentamiento (edificios ya colocados, fuentes ya reclamadas) se
// le pasan como parámetro — el motor sigue mandando.

import { LENERA_POR_BOSQUE, REGENERACION_NODOS } from '../constants';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import type { BiomaTipo, NodoRecurso, Point, RegionId, RioZona, TerrenoTipo, ZonaBosque } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';

/** Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md) — payload de `mapa.yacimiento_regenerado` (ver
 * `Mapa.avanzarRegeneracion`). */
export interface PayloadYacimientoRegenerado {
  nodoId: string;
  recurso: string;
}
import {
  costeEnPunto,
  esTransitable,
  distanciaARioMasCercano,
  evaluarBioma,
  evaluarElevacion,
  evaluarFertilidad,
  evaluarTerreno,
  type MapaGenerado,
} from '../worldgen';
import { boundingBox, distancia, pointInPolygon } from './geometria';
import { formaCirculo, unirFormas } from './poligonos';

/**
 * Lado de celda del índice espacial, en unidades de mapa. 100 sobre un mapa de 1000x1000 da una rejilla de
 * 10x10 para ~117 nodos: suficiente para que una consulta por radio típica (radio de zona 30-120) mire unas
 * pocas celdas en vez de todos los nodos, sin que la rejilla en sí pese nada.
 */
const LADO_CELDA = 100;

/**
 * Contorno fusionado de los bosques, cacheado por MUNDO GENERADO y no por instancia de `Mapa`.
 *
 * Los bosques son geometría inmutable del mundo (`MapaGenerado`, ver `generarBosques`), mientras que cada
 * foto del historial construye su propia fachada `Mapa` sobre ESE MISMO objeto generado, compartido por
 * referencia (ver `GameStore.getMapa`). Cachear en el campo de la instancia obligaría a recalcular la unión
 * entera cada vez que se mira un tick pasado en el slider de la línea de tiempo; cacheando por mundo se
 * calcula una sola vez por partida. `WeakMap` para no retener mundos de partidas ya descartadas.
 */
const contornosBosquesPorMundo = new WeakMap<MapaGenerado, Point[][]>();

/**
 * Resolución de la fusión de bosques, como fracción del ancho del mapa: sobre los 2000 por defecto da un paso
 * de 5 unidades. Relativo y no absoluto para que un mapa el doble de grande no salga con un contorno el doble
 * de tosco. Se puede permitir ser fino porque un bosque es un círculo de verdad y `formaCirculo` resuelve
 * cada muestra con una resta, sin recorrer aristas: MEDIDO en navegador, los 170 bosques del mapa por
 * defecto se fusionan en 25 siluetas (~1300 puntos en total) en unos 16 ms, una sola vez por mundo.
 */
const PASO_FUSION_BOSQUES = 1 / 400;

export interface OpcionesNodos {
  /** Solo nodos de este tipo de recurso. */
  tipo?: string;
  /** Excluir nodos ya agotados (`cantidad <= 0`). */
  conStock?: boolean;
  /** Ids de nodo a descartar — p. ej. fuentes ya reclamadas por otro edificio. */
  excluir?: Set<string>;
  /** Si se indica, el resultado sale ordenado del más cercano a este punto al más lejano. */
  ordenarPorCercaniaA?: Point;
}

/**
 * Estado de PARTIDA del mapa: lo único que cambia durante el juego. Vive separado del `MapaGenerado`
 * (que es inmutable desde el momento en que se genera) por tres razones concretas:
 *  - Guardar una foto del historial pasa a clonar un puñado de números en vez de los ~117 objetos-nodo
 *    de todo el mapa, cada tick.
 *  - El mapa generado se puede compartir por referencia entre todas las fotos sin riesgo de que una
 *    partida avanzada contamine el pasado.
 *  - Deja de haber dos nociones de "cantidad" mezcladas en el mismo campo: `cantidadInicial` es del mundo,
 *    lo extraído es de la partida.
 */
export interface EstadoMapa {
  /** Cuánto se lleva sacado de cada yacimiento (id de nodo -> unidades). Ausente = intacto. */
  extraido: Record<string, number>;
  /** Instante de mundo en que un yacimiento agotado (stock 0) vuelve a aparecer con su `cantidadInicial`
   * completa (id de nodo -> `Instante`, ver `Mapa.avanzarRegeneracion`). Ausente = productivo o sin agendar. */
  regeneraEn: Record<string, Instante>;
}

export function crearEstadoMapa(): EstadoMapa {
  return { extraido: {}, regeneraEn: {} };
}

/** Copia independiente del estado de partida del mapa. Son dos registros de números: clonar cuesta lo que
 * cuesta recorrer los nodos ya tocados, no los ~117 del mundo. */
function copiarEstadoMapa(estado: EstadoMapa): EstadoMapa {
  return { extraido: { ...estado.extraido }, regeneraEn: { ...estado.regeneraEn } };
}

/**
 * Índices derivados del MUNDO GENERADO — nada de estado de partida. Se cachean por mundo (misma razón y
 * misma técnica que `contornosBosquesPorMundo`): desde que cada comando construye su propia fachada sobre
 * una copia del estado (ver `GameSession.ejecutar`), instanciar `Mapa` dejó de ser algo que pasa una vez por
 * partida para pasar a ser algo que pasa una vez por comando, y reconstruir tres mapas idénticos sobre los
 * mismos nodos inmutables en cada uno no tiene sentido.
 */
interface IndicesMundo {
  nodosPorId: Map<string, NodoRecurso>;
  bosquesPorId: Map<string, ZonaBosque>;
  /** Índice espacial de NODOS: clave "col,fila" -> nodos cuyo centro cae en esa celda. */
  celdas: Map<string, NodoRecurso[]>;
  /**
   * Posición de cada nodo en el orden de generación. El índice espacial devuelve los nodos agrupados por
   * celda, y ese orden NO es el de generación: sin restaurarlo, dos nodos empatados a distancia se
   * desempatarían distinto que con el recorrido lineal de antes, cambiando dónde se coloca un edificio y
   * haciendo divergir la simulación. El orden de generación es parte del contrato observable del mapa.
   */
  ordenGeneracion: Map<string, number>;
}

const indicesPorMundo = new WeakMap<MapaGenerado, IndicesMundo>();

function claveCelda(p: Point): string {
  return `${Math.floor(p.x / LADO_CELDA)},${Math.floor(p.y / LADO_CELDA)}`;
}

function indicesDe(generado: MapaGenerado): IndicesMundo {
  const cacheado = indicesPorMundo.get(generado);
  if (cacheado) return cacheado;

  // Los bosques NO se indexan espacialmente: son 25 y se consultan por solapamiento de círculos (radio
  // contra radio), donde una rejilla por punto central no ayudaría — un bosque grande alcanza varias celdas.
  const celdas = new Map<string, NodoRecurso[]>();
  for (const nodo of generado.nodos) {
    const clave = claveCelda(nodo.posicion);
    const lista = celdas.get(clave);
    if (lista) lista.push(nodo);
    else celdas.set(clave, [nodo]);
  }

  const indices: IndicesMundo = {
    nodosPorId: new Map(generado.nodos.map((n) => [n.id, n])),
    bosquesPorId: new Map(generado.bosques.map((b) => [b.id, b])),
    celdas,
    ordenGeneracion: new Map(generado.nodos.map((n, i) => [n.id, i])),
  };
  indicesPorMundo.set(generado, indices);
  return indices;
}

export class Mapa {
  private readonly generado: MapaGenerado;
  /**
   * Estado de partida del mapa, SIEMPRE una copia propia — nunca el objeto que pasó el llamador.
   *
   * Es lo que hace que `extraer`/`avanzarRegeneracion` no se lleven por delante el estado de la partida: la
   * fachada trabaja sobre su copia y quien la creó decide si adopta el resultado (`estadoActual()`) o lo
   * descarta. Antes se aliaseaba, y por eso un tick que fallara al persistirse dejaba los yacimientos ya
   * vaciados aunque se descartara el estado devuelto (Fase B3, prerrequisito de la persistencia).
   */
  private readonly estado: EstadoMapa;
  /** Mutaciones aceptadas sobre esta fachada. Solo sirve para que el llamador pueda comprobar que no se le
   * olvidó adoptar `estadoActual()` — ver la comprobación de `GameSession.ejecutar`. */
  private mutaciones = 0;
  private readonly nodosPorId: Map<string, NodoRecurso>;
  private readonly bosquesPorId: Map<string, ZonaBosque>;
  private readonly celdas: Map<string, NodoRecurso[]>;
  private readonly ordenGeneracion: Map<string, number>;

  constructor(generado: MapaGenerado, estado: EstadoMapa) {
    this.generado = generado;
    this.estado = copiarEstadoMapa(estado);
    const indices = indicesDe(generado);
    this.nodosPorId = indices.nodosPorId;
    this.bosquesPorId = indices.bosquesPorId;
    this.celdas = indices.celdas;
    this.ordenGeneracion = indices.ordenGeneracion;
  }

  /**
   * Estado de partida resultante de lo aplicado sobre esta fachada, como valor independiente listo para
   * entrar en el estado de la partida. Copia en cada llamada a propósito: quien lo guarda es su dueño, y
   * seguir mutando esta fachada después no debe poder tocarlo.
   */
  estadoActual(): EstadoMapa {
    return copiarEstadoMapa(this.estado);
  }

  /** Cuántas mutaciones se han aceptado sobre esta fachada (`extraer` con resultado > 0, regeneraciones). */
  get mutacionesAplicadas(): number {
    return this.mutaciones;
  }

  /**
   * Nodos de las celdas que solapan la caja dada, devueltos en ORDEN DE GENERACIÓN (ver `ordenGeneracion`).
   * Superconjunto del resultado: el llamador filtra fino.
   */
  private nodosEnCaja(minX: number, minY: number, maxX: number, maxY: number): NodoRecurso[] {
    const colMin = Math.floor(minX / LADO_CELDA);
    const colMax = Math.floor(maxX / LADO_CELDA);
    const filaMin = Math.floor(minY / LADO_CELDA);
    const filaMax = Math.floor(maxY / LADO_CELDA);
    const resultado: NodoRecurso[] = [];
    for (let fila = filaMin; fila <= filaMax; fila++) {
      for (let col = colMin; col <= colMax; col++) {
        const lista = this.celdas.get(`${col},${fila}`);
        if (lista) resultado.push(...lista);
      }
    }
    return resultado.sort((a, b) => this.ordenGeneracion.get(a.id)! - this.ordenGeneracion.get(b.id)!);
  }

  /**
   * Aplica los filtros de `OpcionesNodos` y el orden. Se mantiene el orden de generación como criterio
   * estable de base (el `filter` de un array lo preserva) porque de él dependen desempates aguas abajo:
   * dos nodos a la misma distancia deben resolverse siempre igual, o la simulación deja de ser reproducible.
   */
  private filtrar(nodos: NodoRecurso[], opciones: OpcionesNodos): NodoRecurso[] {
    let resultado = nodos;
    if (opciones.tipo !== undefined) resultado = resultado.filter((n) => n.tipo === opciones.tipo);
    if (opciones.conStock) resultado = resultado.filter((n) => this.stock(n.id) > 0);
    if (opciones.excluir) resultado = resultado.filter((n) => !opciones.excluir!.has(n.id));
    const referencia = opciones.ordenarPorCercaniaA;
    if (referencia) {
      resultado = [...resultado].sort((a, b) => distancia(a.posicion, referencia) - distancia(b.posicion, referencia));
    }
    return resultado;
  }

  // --- Límites y terreno ---

  get limites(): { ancho: number; alto: number } {
    return { ancho: this.generado.config.ancho, alto: this.generado.config.alto };
  }

  get seed(): number {
    return this.generado.config.seed;
  }

  /** Región geográfica con la que se generó este mundo (Fase 0.2), `undefined` = mundo libre. Expuesta
   * junto a `seed` por la misma razón: `terrenoCacheParaFrame` (`main.ts`) la necesita en su clave de
   * cache — dos mundos con la MISMA seed pero región distinta tienen una capa de terreno distinta. */
  get region(): RegionId | undefined {
    return this.generado.config.region;
  }

  dentroDelMapa(p: Point): boolean {
    return p.x >= 0 && p.x <= this.generado.config.ancho && p.y >= 0 && p.y <= this.generado.config.alto;
  }

  /** Elevación continua 0-1 en un punto cualquiera (Fase 0.1) — ver `evaluarElevacion`. */
  elevacionEn(p: Point): number {
    return evaluarElevacion(this.generado.elevacion, p);
  }

  /**
   * Tipo de terreno en un punto (Fase 0.1: relieve real, derivado del campo de elevación por umbral —
   * antes era un stub fijo a 'llano').
   */
  terrenoEn(p: Point): TerrenoTipo {
    return evaluarTerreno(this.generado.elevacion, p);
  }

  /** Bioma en un punto (Fase 0.1) — terreno + fertilidad + humedad, ver `evaluarBioma`. */
  biomaEn(p: Point): BiomaTipo {
    return evaluarBioma(this.generado.elevacion, this.generado.fertilidad, this.generado.rios, p);
  }

  /** Multiplicador de coste de movimiento en un punto (Fase 0.3) — ver `costeEnPunto`. Lo consulta el
   * pathfinding (`world/rutas.ts`) y el avance por tick de las caravanas (`engine/movimiento.ts`). */
  costeEnPunto(p: Point): number {
    return costeEnPunto(this.generado.elevacion, p);
  }

  /** ¿Se puede pisar este punto? El agua es un OBSTÁCULO, no terreno caro (a petición del usuario) — ver
   * `esTransitable` en `worldgen/costeMovimiento.ts` para lo que eso implica en el pathfinding. */
  esTransitable(p: Point): boolean {
    return esTransitable(this.generado.elevacion, p);
  }

  // --- Ríos ---

  listarRios(): readonly RioZona[] {
    return this.generado.rios;
  }

  /** Río más cercano a un punto y la distancia mínima a su trazo. `null` si el mundo no tiene ríos.
   * Recorrido lineal: ~6 ríos no justifica un índice espacial, igual que bosques hoy. */
  rioMasCercano(p: Point): { rio: RioZona; distancia: number } | null {
    let mejor: { rio: RioZona; distancia: number } | null = null;
    for (const rio of this.generado.rios) {
      const d = distanciaARioMasCercano([rio], p);
      if (!mejor || d < mejor.distancia) mejor = { rio, distancia: d };
    }
    return mejor;
  }

  // --- Fertilidad ---

  fertilidadEn(p: Point): number {
    return evaluarFertilidad(this.generado.fertilidad, p);
  }

  /**
   * De una lista de puntos candidatos, el más fértil (y su fertilidad). `null` si la lista viene vacía.
   * La GENERACIÓN de candidatos se queda en el motor a propósito: quién puede construir dónde depende de
   * la zona de influencia y de los edificios ya colocados, que son estado de partida, no del mapa.
   */
  mejorPorFertilidad(candidatos: Point[]): { punto: Point; fertilidad: number } | null {
    let mejor: Point | null = null;
    let mejorFertilidad = -1;
    for (const candidato of candidatos) {
      const fertilidad = this.fertilidadEn(candidato);
      if (fertilidad > mejorFertilidad) {
        mejorFertilidad = fertilidad;
        mejor = candidato;
      }
    }
    return mejor ? { punto: mejor, fertilidad: mejorFertilidad } : null;
  }

  // --- Nodos de recurso ---

  nodo(id: string | undefined): NodoRecurso | undefined {
    return id === undefined ? undefined : this.nodosPorId.get(id);
  }

  /** Todos los nodos, en orden de generación. Solo lectura — para dibujar y para diagnósticos. */
  listarNodos(): readonly NodoRecurso[] {
    return this.generado.nodos;
  }

  nodosEnRadio(centro: Point, radio: number, opciones: OpcionesNodos = {}): NodoRecurso[] {
    const candidatos = this.nodosEnCaja(centro.x - radio, centro.y - radio, centro.x + radio, centro.y + radio);
    const enRadio = candidatos.filter((n) => distancia(n.posicion, centro) <= radio);
    return this.filtrar(enRadio, opciones);
  }

  nodosEnPoligono(poligono: Point[], opciones: OpcionesNodos = {}): NodoRecurso[] {
    const caja = boundingBox(poligono);
    if (!caja) return [];
    const candidatos = this.nodosEnCaja(caja.minX, caja.minY, caja.maxX, caja.maxY);
    const dentro = candidatos.filter((n) => pointInPolygon(n.posicion, poligono));
    return this.filtrar(dentro, opciones);
  }

  /** Unidades que quedan en un yacimiento: lo que tenía al generarse menos lo ya extraído en partida. */
  stock(id: string | undefined): number {
    const nodo = this.nodo(id);
    if (!nodo) return 0;
    return nodo.cantidadInicial - (this.estado.extraido[nodo.id] ?? 0);
  }

  /**
   * Extrae hasta `cantidad` de un nodo y devuelve lo realmente extraído (0 si el nodo no existe o está
   * agotado). ÚNICA vía de mutación del mapa: el agotamiento de yacimientos deja de ser un `nodo.cantidad -= x`
   * escrito en seis sitios distintos de `construction.ts`, y lo que se toca es la copia local del estado de
   * partida — ni el mundo generado ni el estado de quien creó la fachada cambian nunca.
   */
  extraer(nodoId: string | undefined, cantidad: number): number {
    const nodo = this.nodo(nodoId);
    if (!nodo || cantidad <= 0) return 0;
    const disponible = this.stock(nodo.id);
    if (disponible <= 0) return 0;
    const extraido = Math.min(cantidad, disponible);
    this.estado.extraido[nodo.id] = (this.estado.extraido[nodo.id] ?? 0) + extraido;
    this.mutaciones++;
    return extraido;
  }

  /** `true` si el nodo existe y todavía tiene stock. */
  nodoProductivo(id: string | undefined): boolean {
    return this.stock(id) > 0;
  }

  /**
   * Avanza la regeneración de yacimientos agotados (a petición del usuario): SEGUNDA y única otra vía de
   * mutación del mapa además de `extraer`. Un nodo que llega a stock 0 agenda su reaparición para
   * `instante + N` (N según su tipo, `REGENERACION_NODOS`) la primera vez que se detecta agotado; cuando
   * ese instante llega, vuelve a stock completo (se borra lo extraído) y se olvida el calendario. Un nodo
   * agotado de una partida guardada ANTES de que existiera este sistema (sin entrada en `regeneraEn`)
   * se agenda solo la primera vez que corre esto — autocurativo, no hace falta migrar datos.
   */
  avanzarRegeneracion(instante: Instante): EventoCrudo[] {
    const eventos: EventoCrudo[] = [];
    for (const nodo of this.generado.nodos) {
      if (this.stock(nodo.id) > 0) continue;
      const pendiente = this.estado.regeneraEn[nodo.id];
      if (pendiente === undefined) {
        const cooldownTicks =
          nodo.tipo === 'livestock' ? REGENERACION_NODOS.livestock.cooldownMinutos : REGENERACION_NODOS.metales.cooldownMinutos;
        this.estado.regeneraEn[nodo.id] = sumar(instante, minutos(cooldownTicks));
        this.mutaciones++;
        continue;
      }
      if (instante >= pendiente) {
        delete this.estado.extraido[nodo.id];
        delete this.estado.regeneraEn[nodo.id];
        this.mutaciones++;
        eventos.push({
          codigo: 'mapa.yacimiento_regenerado',
          mensaje: `El yacimiento de ${nodo.tipo} (${nodo.id}) se regenera.`,
          payload: { nodoId: nodo.id, recurso: nodo.tipo } satisfies PayloadYacimientoRegenerado,
        });
      }
    }
    return eventos;
  }

  /**
   * Nodos con su cantidad RESTANTE, para serializar la partida (el formato de archivo guarda `cantidad`,
   * ya descontada) y para diagnósticos. No es la vista que usa el motor: ahí se consulta `stock(id)`.
   */
  nodosConStock(): (Omit<NodoRecurso, 'cantidadInicial'> & { cantidad: number })[] {
    return this.generado.nodos.map(({ cantidadInicial: _inicial, ...resto }) => ({
      ...resto,
      cantidad: this.stock(resto.id),
    }));
  }

  // --- Bosques ---

  bosque(id: string | undefined): ZonaBosque | undefined {
    return id === undefined ? undefined : this.bosquesPorId.get(id);
  }

  listarBosques(): readonly ZonaBosque[] {
    return this.generado.bosques;
  }

  /**
   * Los bosques como UNA silueta: contorno de la unión de todos sus discos, en lazos cerrados
   * (a petición del usuario). Para DIBUJAR el mapa general; ninguna regla de juego lo consulta — el alcance
   * de una Leñera sigue resolviéndose bosque a bosque (`bosqueParaLenera`, `hayBosqueEnRadio`).
   *
   * Los bosques se solapan mucho por diseño (170 discos de radio 45-120 sobre 2000 unidades, ver `BOSQUE`),
   * así que dibujarlos uno a uno daba un amasijo de círculos con un anillo más oscuro en cada intersección,
   * en vez de manchas de bosque con una silueta propia. Los claros que quedan encerrados por una corona de
   * bosques salen como lazos de orientación opuesta y el relleno `nonzero` del canvas los recorta solo.
   *
   * Resultado inmutable y cacheado por mundo — el llamador no debe modificar los arrays devueltos.
   */
  contornosBosques(): readonly Point[][] {
    const cacheado = contornosBosquesPorMundo.get(this.generado);
    if (cacheado) return cacheado;
    const paso = this.generado.config.ancho * PASO_FUSION_BOSQUES;
    const contornos = unirFormas(
      this.generado.bosques.map((b) => formaCirculo(b.centro, b.radio)),
      { paso, tolerancia: paso * 0.4, areaMinima: paso * paso * 4 }
    );
    contornosBosquesPorMundo.set(this.generado, contornos);
    return contornos;
  }

  /**
   * ¿Hay algún bosque cuyo BORDE entre en el círculo dado? No hace falta que lo esté su centro: los bosques
   * tienen radio 30-80 y una zona de influencia puede tocar un bosque grande por un punto muy alejado de su
   * centro. Este es el criterio único — antes vivía duplicado en `settlement.ts` y `construction.ts`.
   */
  hayBosqueEnRadio(centro: Point, radio: number): boolean {
    return this.generado.bosques.some((b) => distancia(b.centro, centro) < radio + b.radio);
  }

  /**
   * Como `hayBosqueEnRadio`, pero además exige que algún bosque alcanzable tenga capacidad de Leñera SIN
   * reclamar — `capacidadLeneras` menos las Leñeras ya contadas para ese bosque en `lenerasPorBosque`. Un
   * bosque a tope no sirve para fundar: `bosqueParaLenera` no colocará una Leñera ahí y el asentamiento se
   * queda sin madera (ver `evaluarViabilidadFundacion` y el diagnóstico de
   * `Consideraciones/Economia_Del_Oro_Definicion.md` §10 — la mayoría de las muertes por madera del batch NPC
   * son por fundar sobre un bosque que un vecino ya trabaja).
   */
  hayBosqueLibreEnRadio(centro: Point, radio: number, lenerasPorBosque: Map<string, number>): boolean {
    return this.generado.bosques.some(
      (b) => distancia(b.centro, centro) < radio + b.radio && (lenerasPorBosque.get(b.id) ?? 0) < this.capacidadLeneras(b.id)
    );
  }

  /** Cuántas Leñeras admite un bosque a la vez según su tamaño (mín 1 / máx 3, ver `LENERA_POR_BOSQUE`). */
  capacidadLeneras(bosqueId: string): number {
    const bosque = this.bosque(bosqueId);
    if (!bosque) return 0;
    if (bosque.radio >= LENERA_POR_BOSQUE.umbral3) return 3;
    if (bosque.radio >= LENERA_POR_BOSQUE.umbral2) return 2;
    return 1;
  }

  /**
   * Punto utilizable de un bosque que caiga DENTRO del polígono dado, o `null` si el bosque no llega.
   * Se prueba primero el punto preferido (el centro, o uno desplazado si el bosque ya tiene otras Leñeras)
   * y, si cae fuera, se muestrean anillos crecientes dentro del propio bosque: un bosque puede entrar en
   * contacto con la zona por un punto distinto de su centro. Corrige un bug real detectado jugando —
   * comprobar solo el centro dejaba bosques grandes invisibles para siempre y el asentamiento moría de
   * falta de madera pese a tener el bosque pegado (ver `Correcciones_Durante_Desarrollo.md`).
   */
  puntoDeTrabajoEnBosque(bosqueId: string, poligono: Point[], indiceOcupacion: number): Point | null {
    const bosque = this.bosque(bosqueId);
    if (!bosque) return null;

    const preferido =
      indiceOcupacion === 0
        ? bosque.centro
        : {
            x: bosque.centro.x + Math.cos((indiceOcupacion / 3) * Math.PI * 2) * bosque.radio * 0.4,
            y: bosque.centro.y + Math.sin((indiceOcupacion / 3) * Math.PI * 2) * bosque.radio * 0.4,
          };
    if (pointInPolygon(preferido, poligono)) return preferido;

    const muestrasPorAnillo = 12;
    for (let anillo = 1; anillo <= 3; anillo++) {
      const radio = (bosque.radio * anillo) / 3;
      for (let i = 0; i < muestrasPorAnillo; i++) {
        const angulo = (i / muestrasPorAnillo) * Math.PI * 2;
        const candidato: Point = { x: bosque.centro.x + Math.cos(angulo) * radio, y: bosque.centro.y + Math.sin(angulo) * radio };
        if (pointInPolygon(candidato, poligono)) return candidato;
      }
    }
    return null;
  }

  /**
   * Mejor bosque donde plantar una Leñera: el más cercano a `cercaDe` que (a) no haya agotado su capacidad
   * según `ocupacionPorBosque` y (b) tenga algún punto suyo dentro del polígono.
   * `ocupacionPorBosque` (cuántas Leñeras tiene ya cada bosque) lo aporta el motor: es estado del
   * asentamiento, no del mapa.
   *
   * **Descarta lejos, ordena poco, para pronto** (Fase E3, 2026-09-05). La versión anterior calculaba el
   * punto de trabajo de TODOS los bosques y solo después ordenaba por distancia para quedarse con uno.
   * Medido, era el 19,5 % del tick: 170 bosques por llamada, 154 evaluaciones, **5 698 tests de
   * punto-en-polígono, ~108 000 operaciones de arista** — y ~30 llamadas por tick con 30 asentamientos.
   * El **98,3 % era trabajo tirado**: solo ~2,8 bosques de 170 están lo bastante cerca como para poder tocar
   * el polígono siquiera.
   *
   * El resultado es EL MISMO, y conviene decir por qué en las tres piezas:
   *
   *  1. El descarte por distancia es **conservador por construcción**: todo punto que
   *     `puntoDeTrabajoEnBosque` llega a probar cae dentro del disco `(bosque.centro, bosque.radio)` —el
   *     preferido a 0,4·radio, los anillos a radio·k/3 con k≤3—, y el polígono cabe entero en el disco
   *     `(centroide, radioEnvolvente)`. Si un punto estuviera en los dos, la desigualdad triangular obliga a
   *     que los centros disten como mucho la suma de los radios. Rechazar por encima de eso no puede perder
   *     ningún bosque que hubiera dado un punto válido.
   *  2. Ordenar ANTES y parar en el primero viable da el mismo ganador que calcularlos todos y ordenar
   *     después: en los dos casos gana el bosque viable de menor distancia a `cercaDe`, y los empates rompen
   *     igual porque `sort` es estable y ambos parten del orden de `generado.bosques`.
   *  3. Se ordenan solo los supervivientes del descarte (~3 de 170), no los 170: ordenar es lo caro de esta
   *     forma de resolverlo, y así deja de serlo.
   *
   * El criterio de cercanía es el mismo que `hayBosqueEnRadio` de más arriba — no es un invento local.
   */
  bosqueParaLenera(
    poligono: Point[],
    ocupacionPorBosque: Map<string, number>,
    cercaDe: Point
  ): { posicion: Point; fuenteId: string } | null {
    // Sin polígono no hay nada que contenga un punto. La versión anterior llegaba a `null` dando el rodeo
    // completo por los 170 bosques; salir aquí es lo mismo, y de paso evita dividir entre cero.
    if (poligono.length === 0) return null;

    // Disco que envuelve al polígono: centroide de los vértices más la distancia al más lejano. O(vértices)
    // —19 de media— una sola vez por llamada.
    let cx = 0;
    let cy = 0;
    for (const v of poligono) {
      cx += v.x;
      cy += v.y;
    }
    const centroide: Point = { x: cx / poligono.length, y: cy / poligono.length };
    let radioEnvolvente = 0;
    for (const v of poligono) {
      const d = distancia(v, centroide);
      if (d > radioEnvolvente) radioEnvolvente = d;
    }

    const alcanzables = this.generado.bosques
      .filter((b) => distancia(b.centro, centroide) <= radioEnvolvente + b.radio)
      .sort((a, b) => distancia(a.centro, cercaDe) - distancia(b.centro, cercaDe));

    for (const bosque of alcanzables) {
      const ocupadas = ocupacionPorBosque.get(bosque.id) ?? 0;
      if (ocupadas >= this.capacidadLeneras(bosque.id)) continue;
      const punto = this.puntoDeTrabajoEnBosque(bosque.id, poligono, ocupadas);
      if (punto) return { posicion: punto, fuenteId: bosque.id };
    }
    return null;
  }
}

/**
 * Fachada sobre un mundo generado y una foto de su estado de partida. `estado` se COPIA: la fachada no
 * escribe nunca en el objeto recibido, así que crear una es siempre seguro. Para recuperar lo que se haya
 * aplicado sobre ella, `estadoActual()`.
 */
export function crearMapa(generado: MapaGenerado, estado: EstadoMapa = crearEstadoMapa()): Mapa {
  return new Mapa(generado, estado);
}
