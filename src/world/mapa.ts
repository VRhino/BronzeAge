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

import { LENERA_POR_BOSQUE } from '../constants';
import type { NodoRecurso, Point, ZonaBosque } from '../domain/types';
import { evaluarFertilidad, type MapaGenerado } from '../worldgen';
import { boundingBox, distancia, pointInPolygon } from './geometria';

/**
 * Lado de celda del índice espacial, en unidades de mapa. 100 sobre un mapa de 1000x1000 da una rejilla de
 * 10x10 para ~117 nodos: suficiente para que una consulta por radio típica (radio de zona 30-120) mire unas
 * pocas celdas en vez de todos los nodos, sin que la rejilla en sí pese nada.
 */
const LADO_CELDA = 100;

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
}

export function crearEstadoMapa(): EstadoMapa {
  return { extraido: {} };
}

export class Mapa {
  private readonly generado: MapaGenerado;
  private readonly estado: EstadoMapa;
  private readonly nodosPorId: Map<string, NodoRecurso>;
  private readonly bosquesPorId: Map<string, ZonaBosque>;
  /** Índice espacial de NODOS: clave "col,fila" -> nodos cuyo centro cae en esa celda. */
  private readonly celdas: Map<string, NodoRecurso[]>;
  /**
   * Posición de cada nodo en el orden de generación. El índice espacial devuelve los nodos agrupados por
   * celda, y ese orden NO es el de generación: sin restaurarlo, dos nodos empatados a distancia se
   * desempatarían distinto que con el recorrido lineal de antes, cambiando dónde se coloca un edificio y
   * haciendo divergir la simulación. El orden de generación es parte del contrato observable del mapa.
   */
  private readonly ordenGeneracion: Map<string, number>;

  constructor(generado: MapaGenerado, estado: EstadoMapa) {
    this.generado = generado;
    this.estado = estado;
    this.nodosPorId = new Map(generado.nodos.map((n) => [n.id, n]));
    this.bosquesPorId = new Map(generado.bosques.map((b) => [b.id, b]));
    this.ordenGeneracion = new Map(generado.nodos.map((n, i) => [n.id, i]));

    // Los bosques NO se indexan: son 25 y se consultan por solapamiento de círculos (radio contra radio),
    // donde una rejilla por punto central no ayudaría — un bosque grande alcanza varias celdas.
    this.celdas = new Map();
    for (const nodo of generado.nodos) {
      const clave = this.claveCelda(nodo.posicion);
      const lista = this.celdas.get(clave);
      if (lista) lista.push(nodo);
      else this.celdas.set(clave, [nodo]);
    }
  }

  private claveCelda(p: Point): string {
    return `${Math.floor(p.x / LADO_CELDA)},${Math.floor(p.y / LADO_CELDA)}`;
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

  dentroDelMapa(p: Point): boolean {
    return p.x >= 0 && p.x <= this.generado.config.ancho && p.y >= 0 && p.y <= this.generado.config.alto;
  }

  /**
   * Tipo de terreno en un punto. Fase 0 tiene el terreno completamente plano (Doc 1.1: sin relieve, ríos ni
   * mar), así que hoy siempre responde 'llano'. Existe para que las fases con relieve tengan dónde entrar
   * sin volver a repartir conocimiento del mapa por todo el motor — ver Doc 1.5 (chokepoints).
   */
  terrenoEn(_p: Point): 'llano' {
    return 'llano';
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
   * escrito en seis sitios distintos de `construction.ts`, y lo que se toca es el estado de partida —
   * el mundo generado no cambia nunca.
   */
  extraer(nodoId: string | undefined, cantidad: number): number {
    const nodo = this.nodo(nodoId);
    if (!nodo || cantidad <= 0) return 0;
    const disponible = this.stock(nodo.id);
    if (disponible <= 0) return 0;
    const extraido = Math.min(cantidad, disponible);
    this.estado.extraido[nodo.id] = (this.estado.extraido[nodo.id] ?? 0) + extraido;
    return extraido;
  }

  /** `true` si el nodo existe y todavía tiene stock. */
  nodoProductivo(id: string | undefined): boolean {
    return this.stock(id) > 0;
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
   * ¿Hay algún bosque cuyo BORDE entre en el círculo dado? No hace falta que lo esté su centro: los bosques
   * tienen radio 30-80 y una zona de influencia puede tocar un bosque grande por un punto muy alejado de su
   * centro. Este es el criterio único — antes vivía duplicado en `settlement.ts` y `construction.ts`.
   */
  hayBosqueEnRadio(centro: Point, radio: number): boolean {
    return this.generado.bosques.some((b) => distancia(b.centro, centro) < radio + b.radio);
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
   */
  bosqueParaLenera(
    poligono: Point[],
    ocupacionPorBosque: Map<string, number>,
    cercaDe: Point
  ): { posicion: Point; fuenteId: string } | null {
    const candidatos = this.generado.bosques
      .map((bosque) => {
        const ocupadas = ocupacionPorBosque.get(bosque.id) ?? 0;
        if (ocupadas >= this.capacidadLeneras(bosque.id)) return null;
        const punto = this.puntoDeTrabajoEnBosque(bosque.id, poligono, ocupadas);
        return punto ? { bosque, punto } : null;
      })
      .filter((c): c is { bosque: ZonaBosque; punto: Point } => c !== null)
      .sort((a, b) => distancia(a.bosque.centro, cercaDe) - distancia(b.bosque.centro, cercaDe));
    const elegido = candidatos[0];
    return elegido ? { posicion: elegido.punto, fuenteId: elegido.bosque.id } : null;
  }
}

export function crearMapa(generado: MapaGenerado, estado: EstadoMapa = crearEstadoMapa()): Mapa {
  return new Mapa(generado, estado);
}
