import type { Asentamiento, Edificio, EdificioTipo, Faccion, Point, RecursoAlmacenado, RecursoTipo } from '../domain/types';
import type { RecetaProduccion } from '../constants';
import {
  EDIFICIO_CATALOGO,
  EXTRACCION_MAXIMOS,
  EXTRACTOR_DESEMPATE,
  LINEAS_PRODUCCION,
  MERCADO_PUESTOS_POR_NIVEL,
  NECESIDADES,
  NIVEL_ASENTAMIENTO,
  produccionTrigoDeGranja,
  SCORE_BANDAS,
  ZONA_INFLUENCIA,
} from '../constants';
import type { Mapa } from '../world/mapa';
import { mejorFertilidadEnZona } from './zones';
// `sitioParaTipo` del trazado se importa con alias: en este archivo ya existe una función con ese nombre, la
// que resuelve la colocación de la construcción MANUAL (que a su vez llama a esta para los tipos internos).
import { anclaNacidaTrasSemilla, CATEGORIA_POR_TIPO, reubicarPorTamano, sitioParaTipo as sitioEnTrazado, sitiosParaTipo, tamanoEdificio } from './trazado';
import {
  capacidadViviendaArtesanos,
  capacidadViviendaPesants,
  edificiosPorTipoYEstado,
  hayProyectoPendiente,
  nivelActualDe,
  ratioManoObra,
  ratioManoObraArtesanos,
} from './asentamientoQuery';
import { agregarRecurso, agregarRecursoConSobrante, descontarRecursos, tieneRecursos } from './almacen';
import { reservaDinamicaConstruccion } from './mantenimiento';
import { factorProduccionTrigo, factorTiempoConstruccion, lineasProduccionPriorizadas } from './politicas';
import { consumoComidaPoblacion } from './population';
import { consumoRacionTropas } from './tropas';

/**
 * Recurso propio de cada tipo de edificio "de supervivencia": Granja no respeta la reserva mínima de
 * trigo, ni Leñera la de madera (ver `reservaDinamicaConstruccion` en engine/mantenimiento.ts) — son la
 * única vía real de recuperar esos recursos, así que bloquearlas por la misma escasez que deben resolver
 * sería un huevo-y-la-gallina sin salida.
 */
const RECURSO_PROPIO: Partial<Record<EdificioTipo, string>> = { granja: 'trigo', lenera: 'madera' };

/**
 * Vista de Asentamiento (a petición del usuario): dos espacios lógicos separados. Casi todo edificio se
 * construye DENTRO del espacio plano del asentamiento (coords locales, origen en el Centro Urbano, ver
 * `Edificio.ambito`); las ÚNICAS excepciones son los extractores minerales, que se plantan sobre su nodo del
 * MAPA GENERAL. Granja/Leñera/Corral son internos aunque su producción/elegibilidad dependa de rasgos del
 * mapa (fertilidad/bosque/livestock) dentro de la zona de influencia — el vínculo con el mapa lo lleva su
 * `fuenteId` (Leñera/Corral) o la fertilidad de zona (Granja), no su posición.
 */
const EDIFICIOS_EN_MAPA = new Set<EdificioTipo>(['mina', 'minaCobre', 'minaEstano', 'cantera']);

/** Espacio lógico en el que vive un tipo de edificio (ver `EDIFICIOS_EN_MAPA`). */
function ambitoDe(tipo: EdificioTipo): 'asentamiento' | 'mapa' {
  return EDIFICIOS_EN_MAPA.has(tipo) ? 'mapa' : 'asentamiento';
}

/**
 * Edificios que extraen contra un NODO finito del mapa y se quedan sin nada cuando lo agotan (Doc 1.4:
 * escasez real por ubicación). Granja y Leñera no están aquí: una produce del campo de fertilidad y la otra
 * de un bosque, y ninguno de los dos se agota.
 *
 * `produccionBase` es una función y no un número porque `EDIFICIO_CATALOGO` se edita en caliente desde el
 * panel de balance: hay que leer el valor en el momento de producir, no al cargar el módulo.
 */
const EXTRACTORES: Partial<Record<EdificioTipo, { recurso: RecursoTipo; produccionBase: () => number; mensajeAgotado: string }>> = {
  cantera: {
    recurso: 'piedra',
    produccionBase: () => EDIFICIO_CATALOGO.cantera.produccionBasePiedra,
    mensajeAgotado: 'El yacimiento de piedra de la cantera se ha agotado.',
  },
  mina: {
    recurso: 'oro',
    produccionBase: () => EDIFICIO_CATALOGO.mina.produccionBaseOro,
    mensajeAgotado: 'El yacimiento de oro de la mina se ha agotado.',
  },
  minaCobre: {
    recurso: 'cobre',
    produccionBase: () => EDIFICIO_CATALOGO.minaCobre.produccionBaseCobre,
    mensajeAgotado: 'El yacimiento de cobre se ha agotado.',
  },
  minaEstano: {
    recurso: 'estano',
    produccionBase: () => EDIFICIO_CATALOGO.minaEstano.produccionBaseEstano,
    mensajeAgotado: 'El yacimiento de estaño se ha agotado.',
  },
  corral: {
    recurso: 'livestock',
    produccionBase: () => EDIFICIO_CATALOGO.corral.produccionBaseLivestock,
    mensajeAgotado: 'El manada de livestock del corral se ha agotado.',
  },
};

/**
 * Un proyecto solo puede COMPROMETERSE (pagarse, overhaul de auto-construcción: ver `evaluarNecesidades`) si,
 * además de poder pagar el costo completo, no deja ningún recurso por debajo de la reserva proyectada
 * (`reserva`, ver `reservaDinamicaConstruccion`) — salvo el recurso que el propio edificio produce (ver
 * `RECURSO_PROPIO`).
 */
function puedeIniciarConstruccion(
  almacen: Record<string, RecursoAlmacenado>,
  costo: Partial<Record<string, number>>,
  tipo: EdificioTipo,
  reserva: Partial<Record<RecursoTipo, number>>
): boolean {
  if (!tieneRecursos(almacen, costo)) return false;
  const exento = RECURSO_PROPIO[tipo];
  return Object.entries(costo).every(([recurso, cantidad]) => {
    if (recurso === exento) return true;
    const reservaRecurso = reserva[recurso as RecursoTipo] ?? 0;
    if (reservaRecurso <= 0) return true;
    const disponible = almacen[recurso]?.cantidad ?? 0;
    return disponible - (cantidad ?? 0) >= reservaRecurso;
  });
}

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

// --- Vista de Asentamiento: colocación sobre el trazado urbano dinámico ---
//
// Toda la geometría (huellas, celdas, aristas, red de calles, manzanas emergentes) vive en `engine/trazado.ts`;
// aquí solo se consume. Ver ese módulo y `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md` antes de tocar
// nada relacionado con dónde cae un edificio.

/** Re-exportados para no romper a quienes ya los importaban desde aquí (`app/gameStore.ts`, tests): su
 * implementación se mudó a `engine/trazado.ts` junto con el resto de la geometría urbana. */
export { angulosDeBarrios, CATEGORIA_POR_TIPO } from './trazado';
export type { CategoriaAsentamiento } from './trazado';

/**
 * Hueco para un edificio de tipo `tipo` dentro de la Vista de Asentamiento — delega en `sitioParaTipo`
 * (engine/trazado.ts), que resuelve huella, barrio, filas y afueras. Se conserva el nombre porque lo usan
 * `engine/settlement.ts` (fundación), la auto-construcción y la construcción manual.
 *
 * Solo necesita `id` + `radioPotencial` de `asentamiento` (narrowing deliberado, no el `Asentamiento`
 * completo) para poder reutilizarse durante la FUNDACIÓN, antes de que exista un `Asentamiento` completo.
 */
export function sitioEnBarrio(
  asentamiento: Pick<Asentamiento, 'id' | 'radioPotencial'>,
  ocupados: Edificio[],
  tipo: EdificioTipo
): Point | null {
  return sitioEnTrazado(asentamiento, ocupados, tipo);
}

/**
 * "Líneas de Producción" (política de Maestro de Obras, a petición del usuario): entre los huecos que
 * `sitiosParaTipo` considera BUENOS —el mejor nivel de preferencia disponible, ver `sitiosParaTipo` en
 * engine/trazado.ts— elige el que minimiza la penalización de distancia (`factorLineaProduccion`, mismo
 * criterio del eslabón más débil que ya usa la producción en marcha) contra las recetas del NIVEL 1 de `tipo`:
 * un edificio de transformación colocado ahí producirá a mejor ritmo desde el primer tick.
 *
 * La optimización se hace DENTRO de ese conjunto, no sobre todos los huecos libres del barrio: el trazado
 * manda sobre la logística, porque saltarse la preferencia de frente de calle rompería las manzanas para ganar
 * unos puntos de factor. Si `tipo` no tiene recetas (Carpintería) cualquier hueco es igual de bueno y se
 * comporta como `sitioEnBarrio`.
 */
export function sitioEnBarrioLineaProduccion(asentamiento: Asentamiento, ocupados: Edificio[], tipo: EdificioTipo): Point | null {
  const categoria = CATEGORIA_POR_TIPO[tipo];
  if (!categoria) return sitioEnBarrio(asentamiento, ocupados, tipo);

  const recetas = nivelesDe(tipo)?.[1]?.recetas ?? [];
  const candidatos = sitiosParaTipo(asentamiento, ocupados, tipo, undefined, true);
  if (recetas.length === 0) return candidatos[0] ?? null;

  let mejor: { punto: Point; score: number } | null = null;
  for (const punto of candidatos) {
    const edificioSimulado: Edificio = { id: '', tipo, posicion: punto, estado: 'activo', ticksRestantes: 0, ambito: 'asentamiento' };
    const score = Math.min(...recetas.map((r) => factorLineaProduccion(edificioSimulado, r, asentamiento)));
    if (!mejor || score > mejor.score) mejor = { punto, score };
  }
  return mejor?.punto ?? null;
}

/** Cantera/edificio de extracción: junto al nodo de recurso más cercano SIN reclamar ya (Doc 4.2, ej. herrería cerca de mina). */
function sitioCercaDeNodo(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  mapa: Mapa,
  tipoRecurso: string,
  fuentesExcluidas: Set<string>
): { posicion: Point; fuenteId: string } | null {
  // 'cima' (Fase 0.1) es inhabitable: la generación ya evita colocar nodos ahí (`RECURSO_BIOMA_PERMITIDO`
  // nunca la lista), pero el rejection-sampling tiene un fallback de "mapa saturado, coloca igual" — este
  // filtro es la garantía dura de que, aun en ese caso raro, nunca se planta un extractor ahí.
  const elegido = mapa
    .nodosEnPoligono(zonaPoligono, {
      tipo: tipoRecurso,
      conStock: true,
      excluir: fuentesExcluidas,
      ordenarPorCercaniaA: asentamiento.posicion,
    })
    .find((n) => mapa.terrenoEn(n.posicion) !== 'cima');
  return elegido ? { posicion: elegido.posicion, fuenteId: elegido.id } : null;
}

/**
 * Recurso base -> tipo de edificio que lo extrae directamente del mapa (mismos siete recursos "crudos" de
 * EXTRACTORES + granja/lenera, que quedan fuera de ese mapa por no agotar nodo, ver arriba). Usado por
 * `fuentesDeRecurso` (líneas de producción, Doc 4.2.1) para saber dónde buscar el origen de un insumo crudo
 * dentro del asentamiento — los insumos que NO aparecen aquí son intermedios de cadena (lingoteCobre, cuero,
 * ...) y su fuente es el transformador que los fabrica, no un extractor.
 */
export const RECURSO_A_EXTRACTOR: Partial<Record<string, EdificioTipo>> = {
  piedra: 'cantera',
  oro: 'mina',
  cobre: 'minaCobre',
  estano: 'minaEstano',
  livestock: 'corral',
  madera: 'lenera',
  trigo: 'granja',
};

/**
 * Fuentes del mapa ya tomadas, sumando TODOS los asentamientos vivos.
 *
 * Antes esto se calculaba por asentamiento, mirando solo sus propios edificios. Entre facciones distintas
 * daba igual (las zonas de influencia se recortan y no se solapan, ver `engine/zones.ts`), pero dos
 * asentamientos de la MISMA facción sí solapan zona: los dos veían el mismo yacimiento libre y los dos le
 * plantaban una mina encima, drenándolo al doble de velocidad y dejando a ambos sin recurso mucho antes de
 * lo que ninguna cuenta del diseño preveía. Con el conteo global, quien llega primero se lo queda.
 *
 * Se calcula una vez por tick y se MUTA a medida que cada asentamiento compromete obra (ver
 * `registrarReclamo`), para que dos asentamientos procesados en el mismo tick tampoco choquen entre sí.
 */
export interface ReclamosFuentes {
  /** Yacimientos con un extractor ya asignado (mina/cantera/corral...). Exclusivos: uno por nodo. */
  nodos: Set<string>;
  /** Leñeras por bosque. NO son exclusivas: un bosque grande admite hasta 3 (ver `mapa.capacidadLeneras`). */
  lenerasPorBosque: Map<string, number>;
}

export function reclamosDeFuentes(asentamientos: Asentamiento[]): ReclamosFuentes {
  const reclamos: ReclamosFuentes = { nodos: new Set(), lenerasPorBosque: new Map() };
  for (const asentamiento of asentamientos) {
    for (const edificio of asentamiento.edificios) {
      registrarReclamo(reclamos, edificio);
    }
  }
  return reclamos;
}

/** Anota la fuente que ocupa un edificio (si ocupa alguna). Idempotente para nodos, acumulativo para bosques. */
function registrarReclamo(reclamos: ReclamosFuentes, edificio: Edificio): void {
  if (!edificio.fuenteId) return;
  if (edificio.tipo === 'lenera') {
    reclamos.lenerasPorBosque.set(edificio.fuenteId, (reclamos.lenerasPorBosque.get(edificio.fuenteId) ?? 0) + 1);
  } else {
    reclamos.nodos.add(edificio.fuenteId);
  }
}

/**
 * Leñera: junto al bosque más cercano que tenga hueco libre según su capacidad (varias Leñeras pueden
 * compartir un bosque grande) Y algún punto suyo dentro de la zona de influencia — no exige que sea justo
 * el centro del bosque. Ambos criterios los resuelve el mapa (`bosqueParaLenera`); aquí solo se aporta el
 * dato que el mapa no puede saber: cuántas Leñeras tiene ya cada bosque, que es estado del asentamiento.
 */
function sitioEnBosque(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  mapa: Mapa,
  conteoPorBosque: Map<string, number>
): { posicion: Point; fuenteId: string } | null {
  return mapa.bosqueParaLenera(zonaPoligono, conteoPorBosque, asentamiento.posicion);
}

function crearEdificioEnCola(tipo: EdificioTipo, posicion: Point, id: string, fuenteId?: string): Edificio {
  const edificio: Edificio = {
    id,
    tipo,
    posicion,
    estado: 'en_cola',
    ticksRestantes: EDIFICIO_CATALOGO[tipo].tiempoConstruccionTicks,
    ambito: ambitoDe(tipo),
  };
  return fuenteId ? { ...edificio, fuenteId } : edificio;
}

/**
 * Puestos que se añaden a la ZONA de Mercado al alcanzar `nivel` (a petición del usuario, ver
 * Consideraciones/Vista_Asentamiento_Trazado_Urbano.md). Nacen GRATIS y ya `activo`: son parte del Mercado que
 * ya se pagó, no obras independientes — encolarlos metería 7 entradas de golpe en la cola al subir a nivel 2 y
 * competirían por el cupo de obras simultáneas.
 *
 * `existentes` debe incluir todo lo que ya ocupa suelo (incluidos los puestos creados en esta misma llamada,
 * que se van acumulando) para que `sitioEnBarrio` no proponga dos veces la misma celda. Si un puesto no
 * encuentra hueco se salta en silencio: la zona es superficie, no función — el cupo de flota lo da el nivel de
 * la pieza principal, tenga o no todo su acompañamiento.
 */
function crearPuestosDeMercado(
  asentamiento: Pick<Asentamiento, 'id' | 'radioPotencial'>,
  nivel: number,
  existentes: Edificio[]
): Edificio[] {
  const formas = MERCADO_PUESTOS_POR_NIVEL[nivel] ?? [];
  if (formas.length === 0) return [];

  // Ids únicos verificados contra los que ya existen: mismo criterio que `nextId` en `evaluarNecesidades`, que
  // documenta el bug de ids repetidas que duplicaba edificios (y con ellos su producción).
  const idsUsadas = new Set(existentes.map((e) => e.id));
  let contador = existentes.length;
  const nuevos: Edificio[] = [];

  for (const forma of formas) {
    const posicion = sitioEnTrazado(asentamiento, [...existentes, ...nuevos], 'puestoMercado', forma);
    if (!posicion) continue;
    let id = `edificio-${asentamiento.id}-${contador++}`;
    while (idsUsadas.has(id)) id = `edificio-${asentamiento.id}-${contador++}`;
    idsUsadas.add(id);
    nuevos.push({
      id,
      tipo: 'puestoMercado',
      posicion,
      estado: 'activo',
      ticksRestantes: 0,
      ambito: 'asentamiento',
      // En un puesto `nivelInterno` no es progresión: identifica su FORMA (ver `PUESTO_MERCADO_FORMA`).
      nivelInterno: forma,
    });
  }
  return nuevos;
}

/**
 * Talleres que se añaden a la ZONA de Carpintería al completarse (§9 del doc de trazado urbano, Etapa 3 de
 * anclas y satélites): 2 piezas gratis, ya activas, iguales entre sí — a diferencia del Mercado, Carpintería
 * no tiene recetas que progresen por nivel interno, así que no hay nada que escalonar: los 2 talleres nacen
 * juntos, una sola vez, al completarse la pieza principal (no en las subidas de nivel interno posteriores).
 *
 * `existentes` debe incluir todo lo que ya ocupa suelo, mismo criterio que `crearPuestosDeMercado`. Si un
 * taller no encuentra hueco se salta en silencio: la zona es superficie, no función.
 */
function crearTalleresDeCarpinteria(asentamiento: Pick<Asentamiento, 'id' | 'radioPotencial'>, existentes: Edificio[]): Edificio[] {
  const idsUsadas = new Set(existentes.map((e) => e.id));
  let contador = existentes.length;
  const nuevos: Edificio[] = [];

  for (let i = 0; i < 2; i++) {
    const posicion = sitioEnTrazado(asentamiento, [...existentes, ...nuevos], 'tallerCarpinteria');
    if (!posicion) continue;
    let id = `edificio-${asentamiento.id}-${contador++}`;
    while (idsUsadas.has(id)) id = `edificio-${asentamiento.id}-${contador++}`;
    idsUsadas.add(id);
    nuevos.push({ id, tipo: 'tallerCarpinteria', posicion, estado: 'activo', ticksRestantes: 0, ambito: 'asentamiento' });
  }
  return nuevos;
}

/**
 * Un extractor (cantera/mina/minaCobre/minaEstano/corral) necesita otra instancia si NINGUNA fuente propia
 * sigue viva (Doc 1.4: escasez real por ubicación — sin esto, agotado el único yacimiento el asentamiento se
 * queda sin ese recurso PARA SIEMPRE) o si el asentamiento todavía no llegó al tope de extractores del tipo
 * (rediseño de progreso Fase 0: `EXTRACCION_MAXIMOS.porTipo`, desacoplado del nivel de asentamiento — antes
 * escalaba 1:1 con él, pero con el tope de nivel bajando a 3 se quedaría corto).
 */
function necesitaNuevoExtractor(asentamiento: Asentamiento, tipo: EdificioTipo, mapa: Mapa): boolean {
  const existentes = asentamiento.edificios.filter((e) => e.tipo === tipo);
  const conFuenteViva = existentes.filter((e) => mapa.nodoProductivo(e.fuenteId));
  if (conFuenteViva.length === 0) return true;
  return conFuenteViva.length < EXTRACCION_MAXIMOS.porTipo;
}

/**
 * Insumo "de arranque" para auto-construir un edificio de transformación (Doc 4.2.1, a petición del usuario):
 * exige tener YA en almacén (cantidad > 0, sea de extracción propia o de trueque — no se distingue origen) al
 * menos uno de los insumos directos de la receta de NIVEL 1. Sin esto, un asentamiento sin ningún nodo de
 * cobre/estaño/livestock en su zona (la mayoría, ver comentario de `RECETA_ARMA_MADERA` en constants.ts —
 * solo ~6% nace con uno) auto-construía Curtiduría/Fundición igual y se quedaba con el edificio produciendo 0
 * para siempre, además de bloquear el turno de Armería en el orden fijo de abajo. Armería queda exenta en la
 * práctica: su receta de nivel 1 incluye `armaMadera`, que solo pide madera — casi siempre > 0.
 *
 * Solo aplica a la AUTO-construcción (`evaluarNecesidades`, más abajo). La construcción MANUAL
 * (`anadirEdificioManualmente`) no la respeta a propósito: es una decisión informada de Gobernador/Maestro de
 * Obras, no un heurístico que deba protegerla de sí misma.
 */
/**
 * ¿El asentamiento ya llegó a su tope de Almacenes (a petición del usuario)? La capacidad de almacenamiento no
 * puede crecer sin límite: cada nivel de asentamiento admite un número fijo
 * (`NECESIDADES.maximoAlmacenesPorNivel`).
 *
 * Cuenta los de CUALQUIER estado —activo, en obra y en cola— para que el tope no se pueda saltar encolando
 * varios a la vez, y se aplica igual a la auto-construcción y a la adición manual: es una regla del juego, no
 * un heurístico interno.
 */
export function alcanzoTopeDeAlmacenes(asentamiento: Asentamiento): boolean {
  // nivelActual (Doc Fase_0_5 §6.2), no nivelAlcanzado: un asentamiento degradado no puede seguir ampliando
  // almacenaje hasta recuperar nivel, aunque ya haya "desbloqueado" un tope mayor alguna vez.
  const tope = NECESIDADES.maximoAlmacenesPorNivel[nivelActualDe(asentamiento)];
  if (tope === undefined) return false;
  return asentamiento.edificios.filter((e) => e.tipo === 'almacen').length >= tope;
}

/**
 * Máximo de Viviendas útil en `nivel` (a petición del usuario): a diferencia de Almacenes, no es un número
 * fijo por nivel — se DERIVA de la población que exige alcanzar el SIGUIENTE nivel
 * (`NIVEL_ASENTAMIENTO.requisitos[nivel+1]`) entre la capacidad de una Vivienda.
 *
 * Se calcula el tope necesario para PESANTS (`.pesants / capacidadPesants`) y para ARTESANOS
 * (`.artesanos / capacidadArtesanos`) por separado y se toma el MAYOR de los dos — no basta con pesants:
 * una Vivienda da 15 cupos de pesants pero solo 5 de artesanos (proporción 3:1), y el gate de nivel 3 pide
 * 500 pesants / 200 artesanos (proporción 2.5:1, más artesanos de lo que esa proporción de vivienda regala).
 * Usar solo el tope por pesants (34 Viviendas ahí) daba una capacidad de solo 170 artesanos — por debajo de
 * los 200 exigidos, un DEADLOCK real detectado por el usuario: nunca se podía subir a nivel 3 porque el
 * propio tope de Vivienda impedía construir las Viviendas de más que hacían falta solo para alojar artesanos
 * (con el mayor de los dos, 40 Viviendas, se cubren los dos cupos: 600 pesants y 200 artesanos).
 *
 * En el nivel MÁXIMO (sin "siguiente" requisito) se usa el techo de población de ese propio nivel
 * (`techoPoblacion`) como referencia para pesants, ya que ahí no hay otro nivel que fije la meta y no hay
 * gate de artesanos posterior con el que pueda entrar en conflicto.
 */
export function maximoViviendasPorNivel(nivel: number): number {
  const requisitoSiguiente = NIVEL_ASENTAMIENTO.requisitos[nivel + 1];
  if (!requisitoSiguiente) {
    const techo = NIVEL_ASENTAMIENTO.techoPoblacion[nivel];
    return techo === undefined ? Infinity : Math.ceil(techo / EDIFICIO_CATALOGO.vivienda.capacidadPesants);
  }
  const topePorPesants = Math.ceil(requisitoSiguiente.pesants / EDIFICIO_CATALOGO.vivienda.capacidadPesants);
  const topePorArtesanos = Math.ceil(requisitoSiguiente.artesanos / EDIFICIO_CATALOGO.vivienda.capacidadArtesanos);
  return Math.max(topePorPesants, topePorArtesanos);
}

/**
 * ¿El asentamiento ya llegó a su tope de Viviendas para su nivel? Mismo criterio que `alcanzoTopeDeAlmacenes`:
 * cuenta CUALQUIER estado (activo/en obra/en cola) y usa `nivelActual` (no `nivel`/nivelAlcanzado), para que
 * un asentamiento degradado no pueda seguir construyendo Viviendas de más hasta recuperar nivel.
 */
export function alcanzoTopeDeViviendas(asentamiento: Asentamiento): boolean {
  const tope = maximoViviendasPorNivel(nivelActualDe(asentamiento));
  return asentamiento.edificios.filter((e) => e.tipo === 'vivienda').length >= tope;
}

export function tieneInsumoDeArranque(asentamiento: Asentamiento, tipo: EdificioTipo): boolean {
  const receta = nivelesDe(tipo)?.[1]?.recetas ?? [];
  if (receta.length === 0) return true;
  return receta.some((r) => Object.keys(r.consumePorUnidad).some((insumo) => (asentamiento.almacen[insumo]?.cantidad ?? 0) > 0));
}

interface Candidato {
  edificio: Edificio;
  score: number;
}

/** Score final de un candidato dentro de su banda (ver `SCORE_BANDAS`, constants.ts): `base` + hasta 100 de
 * urgencia (clamp evita que una urgencia mal calculada cruce a la banda siguiente) + `bonus` opcional (ver
 * `EXTRACTOR_DESEMPATE`) que SÍ puede salir de 0-100 a propósito, para poder ganarle el desempate a un
 * candidato empatado en urgencia máxima. */
function conUrgencia(base: number, urgencia: number, bonus = 0): number {
  return base + Math.max(0, Math.min(100, urgencia)) + bonus;
}

/** Gate de nivel de asentamiento para el mecanismo de semilla/saturación de anclas (Etapa 3, §5.7.1): por
 * debajo de este nivel, un núcleo sin ancla alcanzable cae al reparto de barrio de siempre (comportamiento de
 * antes de la Etapa 3) en vez de fijar un ancla nueva mal colocada en un disco todavía pequeño. Se aplica al
 * MECANISMO de spawn, no a tipos de edificio concretos (a diferencia de lo que el doc propone para
 * Barracón/Galería de tiro): así protege también al núcleo residencial (Vivienda) sin gatear su construcción,
 * que rompería el crecimiento de población inicial. */
const NIVEL_GATE_ANCLAS = 2;

/** Si `nuevo` (ya comprometido/colocado, incluido en `edificios`) hizo nacer un ancla de saturación al
 * colocarse (Etapa 3, §5.4/5.6), la devuelve — o `null` si no aplica (nivel insuficiente, categoría sin ancla
 * de saturación, ya había una alcanzable, o no se encontró sitio válido, ver `anclaNacidaTrasSemilla`). */
function anclaSiNace(asentamiento: Asentamiento, edificios: Edificio[], nuevo: Edificio, id: string): Edificio | null {
  if (nivelActualDe(asentamiento) < NIVEL_GATE_ANCLAS) return null;
  return anclaNacidaTrasSemilla(asentamiento, edificios, nuevo, id);
}

/**
 * Overhaul de auto-construcción: evalúa déficits reales (Doc 4.2, misma elegibilidad que siempre —
 * `enDeficitTrigo`, `necesitaNuevoExtractor`, ocupación de vivienda/almacén, etc.) pero en vez de una cadena
 * de `if`s con slots reservados por categoría, cada candidato elegible recibe un SCORE (ver `SCORE_BANDAS`)
 * y se COMPROMETE (paga de inmediato, ver `puedeIniciarConstruccion`) en orden de score descendente mientras
 * queden fondos y cupo (`NECESIDADES.maximoEnCola`, cuenta proyectos ya en vuelo + los comprometidos este
 * tick). Reemplaza el viejo modelo donde el pago ocurría recién al ARRANCAR la construcción — un proyecto
 * podía quedar `en_cola` indefinidamente sin fondos porque otro se los gastaba antes; ahora, si no se puede
 * pagar YA, el candidato simplemente no se crea este tick (se reevalúa, normalmente con más urgencia, el
 * siguiente) — sin placeholders atascados.
 *
 * Las bandas de score (supervivencia > extractorBase > crecimiento > transformación) no se solapan, así que
 * preservan la misma garantía que antes daban los slots reservados: supervivencia nunca pierde el reparto
 * frente a crecimiento/lujo, sin necesitar mecanismo aparte.
 */
function evaluarNecesidades(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  mapa: Mapa,
  reserva: Partial<Record<RecursoTipo, number>>,
  reclamos: ReclamosFuentes
): { nuevos: Edificio[]; almacen: Record<string, RecursoAlmacenado>; extractoresTicksSinCupo: Partial<Record<EdificioTipo, number>> } {
  const candidatos: Candidato[] = [];
  let contador = asentamiento.edificios.length;
  /**
   * Ids únicos DENTRO del asentamiento, verificado contra los que ya existen.
   *
   * El contador arranca en `edificios.length`, pero cada candidato PROPUESTO consume un número aunque
   * luego no se comprometa (sin fondos o sin cupo). Como el contador del tick siguiente vuelve a partir de
   * la longitud real, esos números se reutilizan: bastaba una pasada que propusiera 3 y comprometiera solo
   * el de mayor score para que un tick posterior repitiera una id ya usada. El edificio duplicado aparecía
   * DOS VECES en `edificios` (el mapa por id de `avanzarConstruccion` resuelve las dos posiciones al mismo
   * objeto), y con él se duplicaban su producción y su reclamo de fuente — se veía como un bosque con más
   * Leñeras de las que admite o un yacimiento con dos extractores.
   */
  const idsUsadas = new Set(asentamiento.edificios.map((e) => e.id));
  const nextId = (): string => {
    let id = `edificio-${asentamiento.id}-${contador++}`;
    while (idsUsadas.has(id)) id = `edificio-${asentamiento.id}-${contador++}`;
    idsUsadas.add(id);
    return id;
  };
  const ocupados = () => [...asentamiento.edificios, ...candidatos.map((c) => c.edificio)];
  const proponer = (edificio: Edificio | null, score: number): void => {
    if (edificio) candidatos.push({ edificio, score });
  };
  /**
   * ¿Hay ya un proyecto de este tipo en el asentamiento, O propuesto en esta misma pasada?
   *
   * Mirar solo `asentamiento.edificios` (que es lo que hace `hayProyectoPendiente`) no basta: como la fuente
   * no se reclama hasta el commit (`registrarReclamo`, al final), dos ramas que propusieran el mismo tipo en
   * el mismo tick elegirían el MISMO bosque o yacimiento — bosques con más Leñeras de las que admite su
   * capacidad, nodos con dos extractores encima. Esta es la misma garantía que `reclamosDeFuentes` da entre
   * asentamientos distintos, pero DENTRO de un único asentamiento en el mismo tick.
   */
  const proyectoEnCurso = (tipo: EdificioTipo): boolean =>
    hayProyectoPendiente(asentamiento, tipo) || candidatos.some((c) => c.edificio.tipo === tipo);

  // Fertilidad de referencia de la zona (Vista de Asentamiento): las Granjas se construyen DENTRO del espacio
  // plano, pero todas rinden con la mejor fertilidad que la zona toca en el mapa general (ver `mejorFertilidadEnZona`).
  const fertilidadZona = mejorFertilidadEnZona(asentamiento, zonaPoligono, mapa);

  // Granja: al menos una, y más si estoy en DÉFICIT de trigo — producción actual de todas las Granjas
  // activas (fertilidad de zona + mano de obra + Edicto de Cosecha) por debajo del consumo actual (población +
  // tropas, Doc 4.1/5.4) — no "cuántos ticks de reserva quedan al ritmo de hoy".
  const granjasActivasEdificios = edificiosPorTipoYEstado(asentamiento, 'granja');
  const ratioManoActual = ratioManoObra(asentamiento);
  const factorTrigoActual = factorProduccionTrigo(asentamiento);
  // Suma por granja, no `nº granjas × base`: cada una rinde según su propio nivel interno (§7 del trazado
  // urbano — una Granja mejorada ocupa más y produce más).
  const produccionTrigoActual = granjasActivasEdificios.reduce(
    (acc, g) => acc + produccionTrigoDeGranja(g.nivelInterno) * fertilidadZona * ratioManoActual * factorTrigoActual,
    0
  );
  const consumoTrigoActual = consumoComidaPoblacion(asentamiento) + consumoRacionTropas(asentamiento);
  const enDeficitTrigo = produccionTrigoActual < consumoTrigoActual;
  // Objetivo PROACTIVO (a petición del usuario, issues/granjas_no_escalan_con_poblacion.md): además del
  // déficit reactivo de arriba (población YA asentada), un segundo umbral más generoso mide contra la
  // capacidad de Vivienda ya construida/en camino (`capacidadViviendaPesants`/`capacidadViviendaArtesanos`),
  // no la población que ya llegó — mismo criterio proactivo que Vivienda ya usa contra su propio umbral de
  // ocupación (85%): construir ANTES de saturarse, no reaccionar después. `Math.max` garantiza que nunca sea
  // MENOS estricto que el reactivo. A propósito NO desbloqueaba por sí solo el modo ráfaga
  // (`maximoGranjasPendientesEnDeficit`, más abajo): la capacidad de Vivienda casi SIEMPRE va por delante de
  // la población real (así está diseñada), así que tratar ese margen normal como una emergencia de varias
  // Granjas a la vez sobre-construía (verificado en simulación: 4 Granjas en vez de 1, diluyendo mano de obra
  // y material que otros edificios de transformación necesitaban). El proactivo solo adelanta CUÁNDO se pide
  // la siguiente Granja (una a la vez), la ráfaga sigue reservada a un déficit REAL ya ocurrido.
  const poblacionObjetivo = {
    ...asentamiento.poblacion,
    pesants: Math.max(asentamiento.poblacion.pesants, capacidadViviendaPesants(asentamiento)),
    artesanos: Math.max(asentamiento.poblacion.artesanos, capacidadViviendaArtesanos(asentamiento)),
  };
  const consumoTrigoObjetivo = consumoComidaPoblacion({ ...asentamiento, poblacion: poblacionObjetivo }) + consumoRacionTropas(asentamiento);
  const enDeficitProyectado = produccionTrigoActual < consumoTrigoObjetivo;
  // Prioridad real a MEJORAR sobre CONSTRUIR (a petición del usuario) — no el orden accidental de que
  // `avanzarMejoras` corra antes en el mismo tick, que el costo geométrico de la mejora (×2 por nivel) podía
  // anular en la práctica. Si alguna Granja activa por debajo de nivel máximo puede pagar YA su siguiente
  // mejora (misma reserva que protege el resto de construcción), esa es la vía: sube el rinde sin sumar nada
  // al denominador compartido de `ratioManoObra` (`trabajadoresRequeridos` fijo en los 4 niveles) — construir
  // una Granja nueva encima solo diluiría la mano de obra de las que ya existen. Con 0 Granjas activas esto es
  // trivialmente falso (nada que mejorar todavía): el arranque nunca se bloquea.
  const nivelesGranja = nivelesDe('granja');
  const hayMejoraGranjaDisponible = granjasActivasEdificios.some((g) => {
    const siguiente = nivelesGranja?.[(g.nivelInterno ?? 1) + 1];
    return !!siguiente && puedeIniciarConstruccion(asentamiento.almacen, siguiente.costoMejora ?? {}, 'granja', reserva);
  });
  // En déficit REACTIVO (no el proyectado) se permite tener varias Granjas en camino a la vez (hasta el
  // tope), no solo una: sin esto, un déficit severo ya ocurrido solo podía corregirse construyendo Granjas en
  // SERIE, quedándose muy por detrás.
  const granjasPendientes = asentamiento.edificios.filter((e) => e.tipo === 'granja' && e.estado !== 'activo').length;
  const limiteGranjasPendientes = enDeficitTrigo ? NECESIDADES.maximoGranjasPendientesEnDeficit : 1;
  if (
    (granjasActivasEdificios.length === 0 || enDeficitProyectado) &&
    !hayMejoraGranjaDisponible &&
    granjasPendientes < limiteGranjasPendientes
  ) {
    const sitio = sitioEnBarrio(asentamiento, ocupados(), 'granja');
    if (sitio) {
      const deficitRatio = consumoTrigoObjetivo > 0 ? (consumoTrigoObjetivo - produccionTrigoActual) / consumoTrigoObjetivo : 1;
      const urgencia = granjasActivasEdificios.length === 0 ? 100 : deficitRatio * 100;
      proponer(crearEdificioEnCola('granja', sitio, nextId()), conUrgencia(SCORE_BANDAS.supervivencia, urgencia));
    }
  }

  // Lenera: mismo tope que los extractores minerales (los bosques no se agotan, Doc 1.4). Urgencia máxima
  // si no hay ninguna, alta si la reserva proyectada de madera ya está comprometida, baja si solo falta
  // para llegar al tope.
  const leneras = edificiosPorTipoYEstado(asentamiento, 'lenera');
  if (leneras.length < EXTRACCION_MAXIMOS.porTipo && !proyectoEnCurso('lenera')) {
    // Elegibilidad y `fuenteId` siguen saliendo del bosque que toca la zona en el mapa general (`sitioEnBosque`);
    // la Leñera se COLOCA dentro del espacio plano del asentamiento (posición local), no sobre el bosque.
    const fuente = sitioEnBosque(asentamiento, zonaPoligono, mapa, reclamos.lenerasPorBosque);
    const local = fuente ? sitioEnBarrio(asentamiento, ocupados(), 'lenera') : null;
    if (fuente && local) {
      const maderaBajoReserva = (asentamiento.almacen.madera?.cantidad ?? 0) < (reserva.madera ?? 0);
      const urgencia = leneras.length === 0 ? 100 : maderaBajoReserva ? 90 : 30;
      proponer(crearEdificioEnCola('lenera', local, nextId(), fuente.fuenteId), conUrgencia(SCORE_BANDAS.supervivencia, urgencia));
    }
  }

  // Extractores base (cantera/corral/minaCobre/mina/minaEstano, Doc 1.1/1.4/3.1/4.2.1/5.7): mismo patrón —
  // urgencia máxima si NINGUNA fuente propia sigue viva, moderada si solo falta para llegar al tope.
  const extractores: { tipo: EdificioTipo; recurso: string }[] = [
    { tipo: 'cantera', recurso: 'piedra' },
    { tipo: 'corral', recurso: 'livestock' },
    { tipo: 'minaCobre', recurso: 'cobre' },
    { tipo: 'mina', recurso: 'oro' },
    { tipo: 'minaEstano', recurso: 'estano' },
  ];
  // Ids de los candidatos de extractor propuestos este tick, por tipo (a lo sumo uno por tipo, ver
  // `proyectoEnCurso`) — tras el commit final se usa para saber cuáles consiguieron cupo y actualizar
  // `extractoresTicksSinCupo` (ver `EXTRACTOR_DESEMPATE`).
  const extractorCandidatoIds: Partial<Record<EdificioTipo, string>> = {};
  for (const { tipo, recurso } of extractores) {
    if (necesitaNuevoExtractor(asentamiento, tipo, mapa) && !proyectoEnCurso(tipo)) {
      const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, mapa, recurso, reclamos.nodos);
      if (sitio) {
        // Minas/Cantera se plantan SOBRE su nodo del mapa general (posición del nodo, `ambito:'mapa'`); el
        // Corral es interno (Vista de Asentamiento): conserva el `fuenteId` del nodo de livestock de la zona,
        // pero se coloca dentro del espacio plano. Si no hay hueco local, no se propone este tick.
        const posicion = ambitoDe(tipo) === 'mapa' ? sitio.posicion : sitioEnBarrio(asentamiento, ocupados(), tipo);
        if (!posicion) continue;
        const conFuenteViva = asentamiento.edificios
          .filter((e) => e.tipo === tipo)
          .some((e) => mapa.nodoProductivo(e.fuenteId));
        const ticksSinCupo = asentamiento.extractoresTicksSinCupo?.[tipo] ?? 0;
        const bonusDesempate = Math.min(ticksSinCupo * EXTRACTOR_DESEMPATE.bonusPorTickStarved, EXTRACTOR_DESEMPATE.bonusMaximo);
        const edificio = crearEdificioEnCola(tipo, posicion, nextId(), sitio.fuenteId);
        extractorCandidatoIds[tipo] = edificio.id;
        proponer(edificio, conUrgencia(SCORE_BANDAS.extractorBase, conFuenteViva ? 40 : 100, bonusDesempate));
      }
    }
  }

  // Cupos separados por clase (ver `crecerPoblacion`, engine/population.ts): una Vivienda nueva amplía
  // ambos a la vez, así que basta con que CUALQUIERA de los dos ya esté saturado para dispararla. Urgencia
  // escala con el % de ocupación.
  const capacidadPesantsVivienda = capacidadViviendaPesants(asentamiento);
  const capacidadArtesanosVivienda = capacidadViviendaArtesanos(asentamiento);
  const ocupacionPesants = capacidadPesantsVivienda <= 0 ? 1 : asentamiento.poblacion.pesants / capacidadPesantsVivienda;
  const ocupacionArtesanos = capacidadArtesanosVivienda <= 0 ? 1 : asentamiento.poblacion.artesanos / capacidadArtesanosVivienda;
  const ocupacionMaxima = Math.max(ocupacionPesants, ocupacionArtesanos);
  if (
    (capacidadPesantsVivienda === 0 || ocupacionMaxima >= NECESIDADES.umbralViviendaOcupada) &&
    !hayProyectoPendiente(asentamiento, 'vivienda') &&
    !alcanzoTopeDeViviendas(asentamiento)
  ) {
    const sitio = sitioEnBarrio(asentamiento, ocupados(), 'vivienda');
    if (sitio) proponer(crearEdificioEnCola('vivienda', sitio, nextId()), conUrgencia(SCORE_BANDAS.crecimiento, ocupacionMaxima * 100));
  }

  // Almacén: urgencia escala con el % de ocupación del recurso más lleno, con tope por nivel de asentamiento.
  const ocupacionAlmacenes = Object.values(asentamiento.almacen)
    .filter((r) => r.capacidad > 0)
    .map((r) => r.cantidad / r.capacidad);
  const ocupacionAlmacenMaxima = ocupacionAlmacenes.length ? Math.max(...ocupacionAlmacenes) : 0;
  if (
    ocupacionAlmacenMaxima >= NECESIDADES.umbralAlmacenAmpliacion &&
    !hayProyectoPendiente(asentamiento, 'almacen') &&
    !alcanzoTopeDeAlmacenes(asentamiento)
  ) {
    const sitio = sitioEnBarrio(asentamiento, ocupados(), 'almacen');
    if (sitio) proponer(crearEdificioEnCola('almacen', sitio, nextId()), conUrgencia(SCORE_BANDAS.crecimiento, ocupacionAlmacenMaxima * 100));
  }

  // Edificios de transformación (Doc 4.2.1, rediseño de progreso Fase 0; gate de nivel añadido en Doc
  // Fase_0_6): Curtiduría/Armería/Fundición ahora sí exigen nivel de asentamiento 2 para su construcción
  // BASE (antes construibles desde nivel 1) — mismo mecanismo que ya usaba Carpintería, ver
  // `requisitoNivelBase` más abajo. Como máximo UNA de las tres puede estar en vuelo a la vez (bug detectado
  // en simulación: sin este límite, varias podían acumularse atascadas esperando piedra en un punto de
  // fundación pobre en ese recurso). Además, cada una exige tener ya el insumo de arranque en almacén (ver
  // `tieneInsumoDeArranque`) — si Curtiduría no lo tiene, el bucle sigue probando Armería/Fundición en el
  // mismo tick en vez de detenerse ahí. Política "Líneas de Producción" del Maestro de Obras: sitúa el
  // edificio nuevo cerca de la fuente de sus insumos en vez del primer hueco libre de siempre (ver
  // `sitioEnBarrioLineaProduccion`).
  const transformacionEnCurso = (['curtiduria', 'armeria', 'fundicion'] as const).some(
    (tipo) => hayProyectoPendiente(asentamiento, tipo)
  );
  if (!transformacionEnCurso && nivelActualDe(asentamiento) >= requisitoNivelBase('fundicion')) {
    for (const tipo of ['curtiduria', 'armeria', 'fundicion'] as const) {
      if (alcanzoTopeDeTransformacion(asentamiento, tipo)) continue;
      if (!tieneInsumoDeArranque(asentamiento, tipo)) continue;
      const sitio = lineasProduccionPriorizadas(asentamiento)
        ? sitioEnBarrioLineaProduccion(asentamiento, ocupados(), tipo)
        : sitioEnBarrio(asentamiento, ocupados(), tipo);
      if (sitio) {
        proponer(crearEdificioEnCola(tipo, sitio, nextId()), SCORE_BANDAS.transformacion);
        break;
      }
    }
  }

  // Carpintería: a diferencia de los otros 3, su construcción BASE sí exige nivel de asentamiento
  // (`requisitoNivelAsentamientoConstruccion`) — habilita Armería/Barracón/Galería de tiro nivel 2.
  const requisitoCarpinteria =
    (EDIFICIO_CATALOGO.carpinteria as { requisitoNivelAsentamientoConstruccion?: number }).requisitoNivelAsentamientoConstruccion ?? 0;
  if (
    nivelActualDe(asentamiento) >= requisitoCarpinteria &&
    !alcanzoTopeDeTransformacion(asentamiento, 'carpinteria') &&
    !hayProyectoPendiente(asentamiento, 'carpinteria')
  ) {
    const sitio = sitioEnBarrio(asentamiento, ocupados(), 'carpinteria');
    if (sitio) proponer(crearEdificioEnCola('carpinteria', sitio, nextId()), SCORE_BANDAS.transformacion);
  }

  // Commit único: ordena todos los candidatos de este tick por score descendente y paga en ese orden
  // mientras haya cupo (`maximoEnCola`, cuenta SOLO lo ya `en_cola` — pagado, a la espera de un hueco de
  // obra — más lo que se va comprometiendo aquí mismo; NO cuenta `en_construccion`, que tiene su propio
  // cupo separado — `maximoEnConstruccionSimultanea`, ver Paso 2 de `avanzarConstruccion` — contarlo aquí
  // también duplicaría la restricción y frenaría el desarrollo sin necesidad) y fondos suficientes
  // respetando la reserva proyectada. El pago ocurre AQUÍ: el candidato entra `en_cola` ya descontado del
  // almacén (overhaul de auto-construcción — Paso 2 ya no comprueba fondos al arrancar la obra).
  const enColaActual = asentamiento.edificios.filter((e) => e.estado === 'en_cola').length;
  let cupoDisponible = Math.max(0, NECESIDADES.maximoEnCola - enColaActual);
  let almacenActual = asentamiento.almacen;
  const nuevos: Edificio[] = [];
  for (const candidato of [...candidatos].sort((a, b) => b.score - a.score)) {
    if (cupoDisponible <= 0) break;
    if (
      TIPOS_TRANSFORMACION.includes(candidato.edificio.tipo) &&
      alcanzoTopeDeTransformacion(asentamiento, candidato.edificio.tipo, nuevos)
    ) {
      continue;
    }
    const costo = EDIFICIO_CATALOGO[candidato.edificio.tipo].costo as Partial<Record<string, number>>;
    if (!puedeIniciarConstruccion(almacenActual, costo, candidato.edificio.tipo, reserva)) continue;
    almacenActual = descontarRecursos(almacenActual, costo);
    const comprometido = { ...candidato.edificio, prioridad: candidato.score };
    nuevos.push(comprometido);
    // Semilla de grupo (Etapa 3, §5.4/5.6): si este compromiso hizo nacer un ancla nueva, se añade gratis en
    // el mismo tick — mismo criterio que `crearPuestosDeMercado` (nace ya activa, no pasa por cola, no cuenta
    // contra `cupoDisponible`).
    const ancla = anclaSiNace(asentamiento, [...asentamiento.edificios, ...nuevos], comprometido, nextId());
    if (ancla) nuevos.push(ancla);
    // La fuente queda tomada en el momento en que se PAGA el proyecto, no al proponerlo: un candidato que
    // no llega a comprometerse (sin fondos o sin cupo) no debe bloquear el yacimiento a nadie más.
    registrarReclamo(reclamos, candidato.edificio);
    cupoDisponible -= 1;
  }

  // Actualiza el desempate anti-inanición (ver `EXTRACTOR_DESEMPATE`): un tipo que se propuso este tick pero
  // no llegó a comprometerse (perdió el desempate o se quedó sin fondos/cupo) suma un tick a su contador; uno
  // que sí consiguió cupo lo resetea a 0. Los tipos que ni siquiera se propusieron este tick (sin sitio, o ya
  // sin necesidad) conservan su contador tal cual — no hay inanición nueva que registrar, pero tampoco se
  // pierde el historial de una racha interrumpida por, p. ej., quedarse un tick sin sitio libre.
  const idsComprometidos = new Set(nuevos.map((n) => n.id));
  const extractoresTicksSinCupo: Partial<Record<EdificioTipo, number>> = { ...asentamiento.extractoresTicksSinCupo };
  for (const [tipo, id] of Object.entries(extractorCandidatoIds) as [EdificioTipo, string][]) {
    extractoresTicksSinCupo[tipo] = idsComprometidos.has(id) ? 0 : (asentamiento.extractoresTicksSinCupo?.[tipo] ?? 0) + 1;
  }

  return { nuevos, almacen: almacenActual, extractoresTicksSinCupo };
}

/** Tipos de edificio de transformación con tiers (Doc 4.2.1): mejoran de nivelInterno y ejecutan recetas.
 * Mercado se suma aquí solo por el mecanismo de MEJORA de nivel interno (`avanzarMejoras`) — sus "recetas"
 * están vacías, el nivel interno solo cambia `cupoCaravanas` (ver `cupoCaravanas`, asentamientoQuery.ts).
 * Granja también, y con dos particularidades propias: su nivel sube el rinde de trigo
 * (`produccionTrigoDeGranja`) y AGRANDA su huella, lo que obliga a mudarla (ver `avanzarMejoras`). */
const EDIFICIOS_CON_NIVELES = ['fundicion', 'curtiduria', 'armeria', 'carpinteria', 'barracon', 'galeriaDeTiro', 'mercado', 'granja'] as const;

function nivelesDe(tipo: EdificioTipo): Record<number, { trabajadoresRequeridos: number; recetas: { produce: string; produccionBase: number; consumePorUnidad: Partial<Record<string, number>> }[]; costoMejora?: Partial<Record<string, number>>; requisitoNivelAsentamiento?: number; requiereEdificio?: string; requiereEdificioNivel?: number }> | undefined {
  return (EDIFICIO_CATALOGO[tipo] as { niveles?: Record<number, any> }).niveles;
}

/** Resultado de evaluar SOLO los gates de la siguiente mejora (nivel de asentamiento + edificio previo, si
 * aplica) — no comprueba fondos. `null` si el edificio no puede evaluarse (inactivo, tipo sin niveles) o si ya
 * está en su nivel máximo, o si no cumple algún gate del siguiente nivel. Extraído de `avanzarMejoras` para que
 * `estadoMejoraEdificio`/`mejorarEdificioManualmente` (mejora manual, a petición del usuario) compartan el
 * mismo criterio de elegibilidad que el camino automático. */
function elegibleParaMejora(
  asentamiento: Asentamiento,
  edificio: Edificio
): { nivelActual: number; nivelSiguiente: number; costo: Partial<Record<string, number>> } | null {
  if (edificio.estado !== 'activo' || !(EDIFICIOS_CON_NIVELES as readonly string[]).includes(edificio.tipo)) return null;
  const niveles = nivelesDe(edificio.tipo);
  if (!niveles) return null;
  const nivelActual = edificio.nivelInterno ?? 1;
  const nivelSiguiente = nivelActual + 1;
  const siguiente = niveles[nivelSiguiente];
  if (!siguiente) return null;
  // nivelActual del ASENTAMIENTO (Doc Fase_0_5 §6.2) — no confundir con `nivelActual` de arriba (nivel
  // INTERNO del edificio): un asentamiento degradado no puede seguir mejorando edificios de nivel alto.
  if (siguiente.requisitoNivelAsentamiento && nivelActualDe(asentamiento) < siguiente.requisitoNivelAsentamiento) return null;
  if (siguiente.requiereEdificio) {
    const previo = edificiosPorTipoYEstado(asentamiento, siguiente.requiereEdificio as EdificioTipo);
    if (previo.length === 0) return null;
    if (siguiente.requiereEdificioNivel && (previo[0]!.nivelInterno ?? 1) < siguiente.requiereEdificioNivel) return null;
  }
  return { nivelActual, nivelSiguiente, costo: siguiente.costoMejora ?? {} };
}

/**
 * Mejora de nivel interno de un edificio de transformación activo (Doc 4.2.1, rediseño de progreso Fase 0):
 * instantánea — si se cumple el gate del siguiente nivel (nivel de asentamiento + edificio previo, si aplica)
 * y hay fondos para `costoMejora` (respetando la misma reserva mínima que protege el inicio de construcción),
 * se paga y sube `nivelInterno` en el mismo tick. No hay tiempo de mejora especificado en el diseño original.
 */
function avanzarMejoras(
  asentamiento: Asentamiento,
  almacen: Record<string, RecursoAlmacenado>,
  reserva: Partial<Record<RecursoTipo, number>>
): { asentamiento: Asentamiento; almacen: Record<string, RecursoAlmacenado>; eventos: string[] } {
  const eventos: string[] = [];
  let almacenActual = almacen;
  // Bucle sobre una copia mutable, no `map`: una mejora puede MUDAR el edificio (ver abajo), y la siguiente
  // tiene que ver esa posición nueva para no elegir un hueco que ya se acaba de ocupar.
  const edificios = [...asentamiento.edificios];
  for (let indice = 0; indice < edificios.length; indice++) {
    const edificio = edificios[indice]!;
    const info = elegibleParaMejora(asentamiento, edificio);
    if (!info) continue;
    const { nivelActual, nivelSiguiente, costo } = info;
    if (!puedeIniciarConstruccion(almacenActual, costo, edificio.tipo, reserva)) continue;

    // Mudanza por crecimiento de huella (hoy solo Granja, §7 del trazado urbano): al subir de nivel ocupa más
    // celdas, así que se muda al hueco de afueras más cercano posible en vez de exigir que quepa donde está.
    // La mejora manda sobre la cercanía: si el único hueco está en el extremo opuesto del mapa, se muda igual.
    // Solo se frena si NO hay hueco para la huella nueva en ningún sitio — mudarla encima de otro edificio es
    // lo único que no se negocia.
    const tamanoActual = tamanoEdificio(edificio.tipo, nivelActual);
    const tamanoNuevo = tamanoEdificio(edificio.tipo, nivelSiguiente);
    let posicion = edificio.posicion;
    if (tamanoNuevo.ancho !== tamanoActual.ancho || tamanoNuevo.alto !== tamanoActual.alto) {
      const destino = reubicarPorTamano(asentamiento, edificio, edificios, nivelSiguiente);
      if (!destino) continue;
      posicion = destino;
    }

    almacenActual = descontarRecursos(almacenActual, costo);
    eventos.push(`${edificio.tipo} mejora a nivel interno ${nivelSiguiente}.`);
    edificios[indice] = { ...edificio, nivelInterno: nivelSiguiente, posicion };

    // La zona de Mercado se puebla al subir de nivel: los puestos se añaden a ESTA misma lista, no a una
    // aparte, para que las mejoras que queden por evaluar en este mismo tick vean sus celdas ya ocupadas.
    if (edificio.tipo === 'mercado') edificios.push(...crearPuestosDeMercado(asentamiento, nivelSiguiente, edificios));
  }
  return { asentamiento: { ...asentamiento, edificios }, almacen: almacenActual, eventos };
}

/**
 * Posiciones de los edificios ACTIVOS del asentamiento que producen `recurso` — un extractor dedicado si es
 * un recurso crudo (ver `RECURSO_A_EXTRACTOR`), o cualquier transformador cuya receta del `nivelInterno`
 * ACTUAL lo tenga como `produce` si es un intermedio de cadena (ej. Fundición -> lingoteCobre, insumo de
 * Armería). Líneas de producción (Doc 4.2.1, a petición del usuario): usado por `factorLineaProduccion` para
 * medir qué tan lejos tiene que "viajar" cada insumo de una receta dentro del asentamiento.
 *
 * Vista de Asentamiento: solo se devuelven fuentes DEL MISMO ESPACIO que el consumidor (`ambitoConsumidor`),
 * porque las coordenadas de los dos espacios (plano del asentamiento vs. mapa general) no son comparables.
 * Una fuente cruzada — p. ej. una Fundición interna que consume `cobre` de una Mina del mapa general — cuenta
 * como "sin fuente local" y cae a `LINEAS_PRODUCCION.distanciaEstandarSinFuente` (ver `factorLineaProduccion`).
 */
function fuentesDeRecurso(asentamiento: Asentamiento, recurso: string, ambitoConsumidor: 'asentamiento' | 'mapa'): Point[] {
  const mismoAmbito = (e: Edificio): boolean => (e.ambito ?? 'asentamiento') === ambitoConsumidor;
  const extractorTipo = RECURSO_A_EXTRACTOR[recurso];
  if (extractorTipo) {
    return asentamiento.edificios
      .filter((e) => e.tipo === extractorTipo && e.estado === 'activo' && mismoAmbito(e))
      .map((e) => e.posicion);
  }
  const posiciones: Point[] = [];
  for (const e of asentamiento.edificios) {
    if (e.estado !== 'activo' || !mismoAmbito(e)) continue;
    const nivel = nivelesDe(e.tipo)?.[e.nivelInterno ?? 1];
    if (nivel?.recetas.some((r) => r.produce === recurso)) posiciones.push(e.posicion);
  }
  return posiciones;
}

/**
 * Factor 0..1 de producción según la distancia a la fuente más cercana de UN insumo (`LINEAS_PRODUCCION`,
 * constants.ts): 1 hasta `distanciaSinPenalizacion`, decae linealmente hasta `factorMinimo` en
 * `distanciaMaxima`. Nunca toca `consumePorUnidad` — solo cuánto se produce ese tick, simulando que el
 * insumo tarda más en llegar cuanto más lejos está su origen dentro del asentamiento.
 */
export function factorPorDistancia(distanciaFuente: number): number {
  const { distanciaSinPenalizacion, distanciaMaxima, factorMinimo } = LINEAS_PRODUCCION;
  if (distanciaFuente <= distanciaSinPenalizacion) return 1;
  if (distanciaFuente >= distanciaMaxima) return factorMinimo;
  const progreso = (distanciaFuente - distanciaSinPenalizacion) / (distanciaMaxima - distanciaSinPenalizacion);
  return 1 - progreso * (1 - factorMinimo);
}

/**
 * Factor de línea de producción de UNA receta completa: el insumo más penalizado manda (el eslabón más
 * débil de la cadena), no un promedio — así una receta con varios insumos no "diluye" el efecto de tener
 * uno de ellos lejos. Si un insumo no tiene ninguna fuente propia en el asentamiento (llega solo por
 * trueque/caravana), se usa `LINEAS_PRODUCCION.distanciaEstandarSinFuente` en su lugar.
 */
export function factorLineaProduccion(edificio: Edificio, receta: RecetaProduccion, asentamiento: Asentamiento): number {
  const insumos = Object.entries(receta.consumePorUnidad)
    .filter(([, porUnidad]) => !!porUnidad)
    .map(([insumo]) => insumo);
  if (insumos.length === 0) return 1;
  const ambitoConsumidor = edificio.ambito ?? 'asentamiento';
  return Math.min(
    ...insumos.map((insumo) => {
      const fuentes = fuentesDeRecurso(asentamiento, insumo, ambitoConsumidor);
      const distanciaFuente =
        fuentes.length > 0 ? Math.min(...fuentes.map((p) => distancia(edificio.posicion, p))) : LINEAS_PRODUCCION.distanciaEstandarSinFuente;
      return factorPorDistancia(distanciaFuente);
    })
  );
}

/**
 * Recetas de crafting de los edificios de transformación activos (Doc 4.2.1, rediseño de progreso Fase 0):
 * recorre las recetas del `nivelInterno` actual EN ORDEN (permite que una receta consuma el output de otra
 * del mismo tick, ej. Lingote de Bronce consumiendo Lingote de Cobre/Estaño recién producidos). La producción
 * real se limita por `min(produccionBase * ratioManoObraArtesanos, insumo_disponible / consumePorUnidad)`,
 * mismo criterio que ya usan los extractores minerales contra `nodo.cantidad`, multiplicado además por el
 * factor de línea de producción (`factorLineaProduccion`, Doc 4.2.1) — nunca al revés: la penalización de
 * distancia reduce cuánto se produce, no cuánto insumo hace falta por unidad.
 */
function avanzarRecetas(
  asentamiento: Asentamiento,
  almacen: Record<string, RecursoAlmacenado>
): { almacen: Record<string, RecursoAlmacenado>; pausados: Set<string> } {
  const ratioArtesano = ratioManoObraArtesanos(asentamiento);
  let almacenActual = almacen;
  const pausados = new Set<string>();
  for (const edificio of asentamiento.edificios) {
    if (edificio.estado !== 'activo') continue;
    const niveles = nivelesDe(edificio.tipo);
    if (!niveles) continue;
    const nivel = niveles[edificio.nivelInterno ?? 1];
    if (!nivel) continue;
    for (const receta of nivel.recetas) {
      let cantidad = receta.produccionBase * ratioArtesano * factorLineaProduccion(edificio, receta, asentamiento);
      for (const [insumo, porUnidad] of Object.entries(receta.consumePorUnidad)) {
        if (!porUnidad) continue;
        const disponible = almacenActual[insumo]?.cantidad ?? 0;
        cantidad = Math.min(cantidad, disponible / porUnidad);
      }
      if (cantidad <= 0) continue;
      const consumo: Partial<Record<string, number>> = {};
      for (const [insumo, porUnidad] of Object.entries(receta.consumePorUnidad)) {
        if (porUnidad) consumo[insumo] = porUnidad * cantidad;
      }
      almacenActual = descontarRecursos(almacenActual, consumo);
      const resultado = agregarRecursoConSobrante(almacenActual, receta.produce, cantidad);
      almacenActual = resultado.almacen;
      if (resultado.sobrante > 0) pausados.add(edificio.id);
    }
  }
  return { almacen: almacenActual, pausados };
}

/**
 * Progresa colas/construcción/producción de un tick y evalúa nuevas necesidades.
 * NOTA: los yacimientos se agotan mutando el mapa in-place vía `mapa.extraer` (simplificación deliberada de
 * Fase 0 para evitar clonar cientos de nodos cada tick); el resto del estado se trata de forma inmutable.
 * `capital` (overhaul de auto-construcción): asentamiento "capital" de la Facción, ya calculado en
 * `simulation.ts` — se usa para proyectar la reserva mínima dinámica (ver `reservaDinamicaConstruccion`,
 * engine/mantenimiento.ts, que reutiliza `calcularCostoMantenimiento`, sensible a la distancia a la capital).
 */
export function avanzarConstruccion(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  mapa: Mapa,
  capital: Asentamiento | undefined,
  reclamos: ReclamosFuentes
): { asentamiento: Asentamiento; eventos: string[]; edificiosCompletados: number } {
  const eventos: string[] = [];
  let almacen = asentamiento.almacen;
  const resultados = new Map<string, Edificio>();
  // Puestos de la zona de Mercado creados en este tick (ver `crearPuestosDeMercado`) — se añaden al final,
  // junto a los proyectos nuevos, porque no estaban en `asentamiento.edificios` al empezar.
  const puestosNuevos: Edificio[] = [];

  const ratioMano = ratioManoObra(asentamiento);
  // Fertilidad de referencia de la zona (Vista de Asentamiento): todas las Granjas del asentamiento rinden con
  // este único valor (la mejor fertilidad que la zona toca en el mapa general), porque viven en el espacio
  // plano local y ya no muestrean fertilidad bajo su propia posición (ver `mejorFertilidadEnZona`).
  const fertilidadZona = mejorFertilidadEnZona(asentamiento, zonaPoligono, mapa);
  // Crecimiento de zona ligado a construcción activa (Doc 1.2, rediseño a petición del usuario): ya no es
  // puramente temporal — cada edificio completado este tick empuja el radio hacia el techo de su nivel.
  let edificiosCompletadosEsteTick = 0;

  // Paso 1 (orden original): progreso de construcciones en curso + producción de edificios activos. Los
  // `en_cola` se resuelven en un segundo paso por PRIORIDAD (ver abajo), no aquí.
  for (const edificio of asentamiento.edificios) {
    if (edificio.estado === 'en_construccion') {
      const restantes = edificio.ticksRestantes - 1;
      if (restantes <= 0) {
        eventos.push(`${edificio.tipo} completado.`);
        edificiosCompletadosEsteTick += 1;
        if (edificio.tipo === 'almacen') {
          const bonus = EDIFICIO_CATALOGO.almacen.capacidadPorRecursoAdicional;
          for (const recurso of Object.keys(almacen)) {
            almacen = { ...almacen, [recurso]: { ...almacen[recurso]!, capacidad: almacen[recurso]!.capacidad + bonus } };
          }
        }
        // El Mercado no nace solo: al terminarse aparece con los puestos de su nivel 1 (a petición del
        // usuario, es una ZONA). Los de niveles 2 y 3 los añade `avanzarMejoras` al subir de nivel interno.
        if (edificio.tipo === 'mercado') {
          puestosNuevos.push(...crearPuestosDeMercado(asentamiento, 1, [...asentamiento.edificios, ...puestosNuevos]));
        }
        // Carpintería tampoco nace sola (§9, Etapa 3 de anclas y satélites): al completarse aparecen sus 2
        // talleres de una vez (no progresan por nivel interno, a diferencia del Mercado).
        if (edificio.tipo === 'carpinteria') {
          puestosNuevos.push(...crearTalleresDeCarpinteria(asentamiento, [...asentamiento.edificios, ...puestosNuevos]));
        }
        resultados.set(edificio.id, { ...edificio, estado: 'activo', ticksRestantes: 0 });
      } else {
        resultados.set(edificio.id, { ...edificio, ticksRestantes: restantes });
      }
      continue;
    }

    if (edificio.estado === 'en_cola') continue;

    // activo: producción
    let pausadoPorAlmacenLleno = false;
    if (edificio.tipo === 'granja') {
      const yieldTrigo = produccionTrigoDeGranja(edificio.nivelInterno) * fertilidadZona * ratioMano * factorProduccionTrigo(asentamiento);
      const resultado = agregarRecursoConSobrante(almacen, 'trigo', yieldTrigo);
      almacen = resultado.almacen;
      pausadoPorAlmacenLleno = resultado.sobrante > 0;
    } else if (edificio.tipo === 'lenera') {
      // Los bosques no se agotan (Doc 1.4): la Leñera no extrae contra un stock, rinde según la densidad.
      const bosque = mapa.bosque(edificio.fuenteId);
      if (bosque) {
        const yieldMadera = EDIFICIO_CATALOGO.lenera.produccionBaseMadera * bosque.densidad * ratioMano;
        const resultado = agregarRecursoConSobrante(almacen, 'madera', yieldMadera);
        almacen = resultado.almacen;
        pausadoPorAlmacenLleno = resultado.sobrante > 0;
      }
    } else {
      const extraccion = EXTRACTORES[edificio.tipo];
      if (extraccion && mapa.nodoProductivo(edificio.fuenteId)) {
        const extraido = mapa.extraer(edificio.fuenteId, extraccion.produccionBase() * ratioMano);
        const resultado = agregarRecursoConSobrante(almacen, extraccion.recurso, extraido);
        almacen = resultado.almacen;
        pausadoPorAlmacenLleno = resultado.sobrante > 0;
        if (!mapa.nodoProductivo(edificio.fuenteId)) eventos.push(extraccion.mensajeAgotado);
      }
    }
    resultados.set(edificio.id, pausadoPorAlmacenLleno !== !!edificio.pausadoPorAlmacenLleno ? { ...edificio, pausadoPorAlmacenLleno } : edificio);
  }

  // Paso 2: arranque de obra. Overhaul de auto-construcción: los `en_cola` ya están PAGADOS (el pago ocurrió
  // al comprometerse, ver `evaluarNecesidades` más abajo) — ya no hace falta comprobar fondos aquí. Lo único
  // que limita el arranque es el cupo de obras activas simultáneas (`maximoEnConstruccionSimultanea`,
  // cuadrillas limitadas: evita que un tick con el almacén lleno dispare media docena de construcciones en
  // paralelo), repartido por `prioridad` (score capturado al comprometerse, mayor primero).
  const enConstruccionActual = asentamiento.edificios.filter((e) => e.estado === 'en_construccion').length;
  let cupoObraDisponible = Math.max(0, NECESIDADES.maximoEnConstruccionSimultanea - enConstruccionActual);
  const enColaPorPrioridad = asentamiento.edificios
    .filter((e) => e.estado === 'en_cola')
    .sort((a, b) => (b.prioridad ?? 0) - (a.prioridad ?? 0));

  for (const edificio of enColaPorPrioridad) {
    if (cupoObraDisponible <= 0) {
      resultados.set(edificio.id, edificio);
      continue;
    }
    eventos.push(`Comienza construcción de ${edificio.tipo}.`);
    // Vía Rápida de Construcción (Maestro de Obras, Doc 2.2/4.4) acelera el tiempo restante al arrancar.
    const ticks = Math.max(1, Math.round(EDIFICIO_CATALOGO[edificio.tipo].tiempoConstruccionTicks * factorTiempoConstruccion(asentamiento)));
    resultados.set(edificio.id, { ...edificio, estado: 'en_construccion', ticksRestantes: ticks });
    cupoObraDisponible -= 1;
  }

  let edificiosActualizados = [...asentamiento.edificios.map((e) => resultados.get(e.id)!), ...puestosNuevos];

  // Rediseño de progreso (Fase 0, Doc 4.2.1): recetas de crafting de los edificios de transformación activos.
  const recetasResultado = avanzarRecetas({ ...asentamiento, edificios: edificiosActualizados }, almacen);
  almacen = recetasResultado.almacen;
  edificiosActualizados = edificiosActualizados.map((e) => {
    const pausado = recetasResultado.pausados.has(e.id);
    return pausado !== !!e.pausadoPorAlmacenLleno ? { ...e, pausadoPorAlmacenLleno: pausado } : e;
  });

  const reserva = reservaDinamicaConstruccion({ ...asentamiento, edificios: edificiosActualizados, almacen }, capital);
  // Reserva manual del Tesorero (a petición del usuario, ver `Asentamiento.reservaManual`): se SUMA a la
  // dinámica y solo aplica a este camino AUTOMÁTICO (avanzarMejoras + evaluarNecesidades más abajo) —
  // `anadirEdificioManualmente` calcula su propia reserva sin esta suma, exenta a propósito.
  for (const [recurso, valor] of Object.entries(asentamiento.reservaManual ?? {})) {
    if (valor) reserva[recurso as RecursoTipo] = (reserva[recurso as RecursoTipo] ?? 0) + valor;
  }

  // Mejora de nivel interno (Doc 4.2.1): evalúa después de las recetas, con el almacén ya actualizado por ellas.
  const trasMejoras = avanzarMejoras({ ...asentamiento, edificios: edificiosActualizados }, almacen, reserva);
  almacen = trasMejoras.almacen;
  eventos.push(...trasMejoras.eventos);

  const radioPotencial = Math.min(
    ZONA_INFLUENCIA.radioMaximoPorNivel[asentamiento.nivel] ?? ZONA_INFLUENCIA.radioMaximoPorNivel[NIVEL_ASENTAMIENTO.nivelMaximo]!,
    asentamiento.radioPotencial + ZONA_INFLUENCIA.crecimientoPorEdificioCompletado * edificiosCompletadosEsteTick
  );
  const asentamientoConProgreso: Asentamiento = { ...trasMejoras.asentamiento, almacen, radioPotencial };

  // Paso 3: compromiso de necesidades (overhaul — reemplaza el viejo "encolar sin pagar"). Pausable por el
  // jugador (ver `Asentamiento.autoConstruccionPausada`): mientras está pausada, no se detectan/comprometen
  // NUEVAS necesidades, pero lo ya pagado (Paso 1/2 de arriba) sigue avanzando normal. Barracón/Galería de
  // tiro/Palacio/Mercado ya NO se auto-detectan aquí (política de desbloqueo retirada, a petición del
  // usuario) — solo se añaden por decisión manual (ver `anadirEdificioManualmente` más abajo).
  let nuevosProyectos: Edificio[] = [];
  let almacenFinal = asentamientoConProgreso.almacen;
  let extractoresTicksSinCupo = asentamiento.extractoresTicksSinCupo;
  if (!asentamiento.autoConstruccionPausada) {
    const trasNecesidades = evaluarNecesidades(asentamientoConProgreso, zonaPoligono, mapa, reserva, reclamos);
    nuevosProyectos = trasNecesidades.nuevos;
    almacenFinal = trasNecesidades.almacen;
    extractoresTicksSinCupo = trasNecesidades.extractoresTicksSinCupo;
  }
  for (const p of nuevosProyectos) eventos.push(`Nueva necesidad detectada: se compromete ${p.tipo} (pagado).`);

  const edificiosFinal = [...asentamientoConProgreso.edificios, ...nuevosProyectos];
  // Reordena los `en_cola` por `prioridad` (mismo criterio que el Paso 2) para que la posición mostrada en la
  // UI (ver main.ts) coincida con el orden real en que arrancarán en el próximo tick.
  const enColaOrdenados = edificiosFinal.filter((e) => e.estado === 'en_cola').sort((a, b) => (b.prioridad ?? 0) - (a.prioridad ?? 0));
  let indiceEnCola = 0;
  const edificiosOrdenados = edificiosFinal.map((e) => (e.estado === 'en_cola' ? enColaOrdenados[indiceEnCola++]! : e));

  return {
    asentamiento: { ...asentamientoConProgreso, almacen: almacenFinal, edificios: edificiosOrdenados, extractoresTicksSinCupo },
    eventos,
    // Doc Fase_0_5 §8: cuántos edificios completó ESTE asentamiento este tick — el llamador (simulation.ts)
    // lo usa para otorgar experiencia de Facción (`NIVEL_FACCION.xp.edificioCompletado`).
    edificiosCompletados: edificiosCompletadosEsteTick,
  };
}

export class ConstruccionManualInvalidaError extends Error {}

/** Tipos que solo admiten UNA instancia por asentamiento (progresan por `nivelInterno` en vez de repetirse) —
 * añadir una segunda no tiene sentido estructural, sea cual sea el mecanismo (auto o manual). */
const EDIFICIOS_UNICOS = new Set<EdificioTipo>([
  'barracon',
  'galeriaDeTiro',
  'palacio',
  'mercado',
  'granFundicion',
  'maravilla',
  // Muralla (Doc Fase_0_6): una sola por asentamiento, mismo patrón que Palacio/Mercado — no auto-
  // construcción, se añade manualmente (Gobernador/Maestro de Obras).
  'muralla',
]);

/** Cupo de cada tipo de edificio de transformaciÃ³n por nivel operativo del asentamiento.
 * Los niveles 4 y 5 mantienen el mismo tope que el nivel 3 hasta que se definan nuevos escalones. */
const MAXIMO_TRANSFORMACION_POR_NIVEL: Partial<Record<number, number>> = {
  2: 3,
  3: 5,
  4: 5,
  5: 5,
};
const TIPOS_TRANSFORMACION: readonly EdificioTipo[] = ['fundicion', 'curtiduria', 'armeria', 'carpinteria'];

function alcanzoTopeDeTransformacion(asentamiento: Asentamiento, tipo: EdificioTipo, adicionales: Edificio[] = []): boolean {
  const tope = MAXIMO_TRANSFORMACION_POR_NIVEL[nivelActualDe(asentamiento)];
  if (tope === undefined) return false;
  const cantidad = asentamiento.edificios.filter((e) => e.tipo === tipo).length + adicionales.filter((e) => e.tipo === tipo).length;
  return cantidad >= tope;
}

/** Requisito de NIVEL DE ASENTAMIENTO para la construcción BASE de un tipo (Doc 4.2.1) — no confundir con los
 * gates de MEJORA de nivel interno, que viven en `niveles[n].requisitoNivelAsentamiento` y no aplican aquí
 * (una mejora nunca pasa por la cola, ver `avanzarMejoras`). Lo tienen Carpintería, Palacio, Maravilla y
 * (Doc Fase_0_6) Fundición/Curtiduría/Armería y Muralla. */
function requisitoNivelBase(tipo: EdificioTipo): number {
  const catalogo = EDIFICIO_CATALOGO[tipo] as { requisitoNivelAsentamientoConstruccion?: number };
  return catalogo.requisitoNivelAsentamientoConstruccion ?? 0;
}

/** Resuelve dónde iría un tipo de edificio si se añade manualmente — mismos algoritmos de colocación que la
 * auto-construcción (Doc 4.2: la ubicación NUNCA la elige el jugador, ni siquiera al añadir manualmente). */
function sitioParaTipo(
  tipo: EdificioTipo,
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  mapa: Mapa,
  reclamos: ReclamosFuentes
): { posicion: Point; fuenteId?: string } | null {
  // Vista de Asentamiento: Granja y el resto de urbanos van a un hueco de la rejilla local (`sitioEnBarrio`).
  // Leñera/Corral conservan su `fuenteId` del mapa general (bosque/livestock en la zona) pero también se
  // colocan dentro. Solo las Minas/Cantera se plantan sobre su nodo del mapa general (posición del nodo).
  if (tipo === 'granja') {
    const posicion = sitioEnBarrio(asentamiento, asentamiento.edificios, 'granja');
    return posicion ? { posicion } : null;
  }
  if (tipo === 'lenera') {
    const fuente = sitioEnBosque(asentamiento, zonaPoligono, mapa, reclamos.lenerasPorBosque);
    if (!fuente) return null;
    const posicion = sitioEnBarrio(asentamiento, asentamiento.edificios, 'lenera');
    return posicion ? { posicion, fuenteId: fuente.fuenteId } : null;
  }
  const recursoExtractor = EXTRACTORES[tipo]?.recurso;
  if (recursoExtractor) {
    const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, mapa, recursoExtractor, reclamos.nodos);
    if (!sitio) return null;
    if (ambitoDe(tipo) === 'mapa') return sitio;
    // Corral: interno — posición local, `fuenteId` del nodo de livestock de la zona.
    const posicion = sitioEnBarrio(asentamiento, asentamiento.edificios, tipo);
    return posicion ? { posicion, fuenteId: sitio.fuenteId } : null;
  }
  const posicion = sitioEnBarrio(asentamiento, asentamiento.edificios, tipo);
  return posicion ? { posicion } : null;
}

function cargoOcupado(asentamiento: Asentamiento, cargo: 'gobernador' | 'maestroObras'): string | null {
  return cargo === 'gobernador' ? asentamiento.cargos.gobernadorId : asentamiento.cargos.maestroObrasId;
}

/**
 * Añade CUALQUIER edificio del catálogo (salvo Centro Urbano, que nunca pasa por cola, Doc 1.3) a la cola de
 * construcción por decisión MANUAL de Gobernador o Maestro de Obras (Doc 4.2, cambio de base a petición del
 * usuario — reemplaza el mecanismo de desbloqueo vía política que tenían Barracón/Galería de tiro/Palacio/
 * Mercado: ahora conviven en el mismo carril manual que cualquier otro edificio, incluida Gran Fundición).
 * Respeta exactamente las mismas reglas que la auto-construcción: paga de inmediato (igual que
 * `evaluarNecesidades`), respeta la reserva mínima de Mantenimiento, nunca elige ubicación (la decide
 * `sitioParaTipo`, mismos algoritmos que usa el motor), y cuenta contra el mismo cupo `NECESIDADES.maximoEnCola`.
 */
export function anadirEdificioManualmente(
  asentamiento: Asentamiento,
  faccion: Faccion,
  cargo: 'gobernador' | 'maestroObras',
  tipo: EdificioTipo,
  zonaPoligono: Point[],
  mapa: Mapa,
  capital: Asentamiento | undefined,
  reclamos: ReclamosFuentes,
  contador = 0
): Asentamiento {
  if (!cargoOcupado(asentamiento, cargo)) {
    throw new ConstruccionManualInvalidaError(`Se necesita un ${cargo} asignado para añadir edificios a la cola.`);
  }
  if (tipo === 'centroUrbano') {
    throw new ConstruccionManualInvalidaError('El Centro Urbano nunca pasa por la cola de construcción.');
  }
  if (tipo === 'puestoMercado') {
    throw new ConstruccionManualInvalidaError('Los puestos son parte de la zona de Mercado: aparecen solos al subir su nivel interno.');
  }
  if (EDIFICIOS_UNICOS.has(tipo) && (edificiosPorTipoYEstado(asentamiento, tipo).length > 0 || hayProyectoPendiente(asentamiento, tipo))) {
    throw new ConstruccionManualInvalidaError(`Ya existe (o está en curso) una ${tipo} en este asentamiento.`);
  }
  if (tipo === 'almacen' && alcanzoTopeDeAlmacenes(asentamiento)) {
    throw new ConstruccionManualInvalidaError(
      `Este asentamiento ya tiene el máximo de Almacenes para su nivel (${NECESIDADES.maximoAlmacenesPorNivel[nivelActualDe(asentamiento)]}).`
    );
  }
  if (tipo === 'vivienda' && alcanzoTopeDeViviendas(asentamiento)) {
    throw new ConstruccionManualInvalidaError(
      `Este asentamiento ya tiene el máximo de Viviendas para su nivel (${maximoViviendasPorNivel(nivelActualDe(asentamiento))}).`
    );
  }
  // nivelActual (Doc Fase_0_5 §6.2), no nivelAlcanzado: un asentamiento degradado no puede construir
  // manualmente edificios de nivel alto hasta recuperarse, aunque su `nivel` histórico ya los desbloqueara.
  const requisito = requisitoNivelBase(tipo);
  const nivelOperativo = nivelActualDe(asentamiento);
  if (nivelOperativo < requisito) {
    throw new ConstruccionManualInvalidaError(`Requiere nivel de asentamiento ${requisito} (actual: ${nivelOperativo}).`);
  }
  if (TIPOS_TRANSFORMACION.includes(tipo) && alcanzoTopeDeTransformacion(asentamiento, tipo)) {
    throw new ConstruccionManualInvalidaError(
      `Este asentamiento ya tiene el m\u00e1ximo de ${tipo} para su nivel (${MAXIMO_TRANSFORMACION_POR_NIVEL[nivelOperativo]}).`
    );
  }
  if (tipo === 'granFundicion' && faccion.nivel < EDIFICIO_CATALOGO.granFundicion.nivelFaccionMinimo) {
    throw new ConstruccionManualInvalidaError(
      `Requiere nivel de Facción ${EDIFICIO_CATALOGO.granFundicion.nivelFaccionMinimo} (actual: ${faccion.nivel}).`
    );
  }
  const enColaActual = asentamiento.edificios.filter((e) => e.estado === 'en_cola').length;
  if (enColaActual >= NECESIDADES.maximoEnCola) {
    throw new ConstruccionManualInvalidaError(`La cola de construcción está llena (máximo ${NECESIDADES.maximoEnCola}).`);
  }
  const sitio = sitioParaTipo(tipo, asentamiento, zonaPoligono, mapa, reclamos);
  if (!sitio) throw new ConstruccionManualInvalidaError('No hay sitio disponible dentro de la zona de influencia.');

  const costo = EDIFICIO_CATALOGO[tipo].costo as Partial<Record<string, number>>;
  const reserva = reservaDinamicaConstruccion(asentamiento, capital);
  if (!puedeIniciarConstruccion(asentamiento.almacen, costo, tipo, reserva)) {
    throw new ConstruccionManualInvalidaError('No hay fondos suficientes (respetando la reserva de mantenimiento) para pagarla ahora.');
  }

  const almacen = descontarRecursos(asentamiento.almacen, costo);
  const nuevo = {
    ...crearEdificioEnCola(tipo, sitio.posicion, `edificio-${asentamiento.id}-manual-${contador}`, sitio.fuenteId),
    prioridad: SCORE_BANDAS.manual,
  };
  const edificiosConNuevo = [...asentamiento.edificios, nuevo];
  // Semilla de grupo (Etapa 3, §5.4/5.6): igual que en auto-construcción, un edificio añadido a mano puede ser
  // el que abre un núcleo nuevo o satura uno existente.
  const ancla = anclaSiNace(asentamiento, edificiosConNuevo, nuevo, `edificio-${asentamiento.id}-manual-${contador}-ancla`);
  return { ...asentamiento, almacen, edificios: ancla ? [...edificiosConNuevo, ancla] : edificiosConNuevo };
}

/**
 * Quita un proyecto `en_cola` de la cola (Doc 4.2, a petición del usuario) — SOLO antes de que arranque la
 * obra (`en_construccion` ya no se puede quitar; qué pasa con los recursos comprometidos a mitad de obra
 * queda PENDIENTE, ver Preguntas_Abiertas.md #14b). Devuelve el costo COMPLETO pagado al comprometerse
 * (overhaul de auto-construcción: el pago ocurrió al encolar, no al empezar a construir).
 */
export function quitarDeCola(asentamiento: Asentamiento, cargo: 'gobernador' | 'maestroObras', edificioId: string): Asentamiento {
  if (!cargoOcupado(asentamiento, cargo)) {
    throw new ConstruccionManualInvalidaError(`Se necesita un ${cargo} asignado para modificar la cola.`);
  }
  const edificio = asentamiento.edificios.find((e) => e.id === edificioId);
  if (!edificio) throw new ConstruccionManualInvalidaError('Ese proyecto no existe en este asentamiento.');
  if (edificio.estado !== 'en_cola') {
    throw new ConstruccionManualInvalidaError('Solo se puede quitar un proyecto que aún no empezó a construirse.');
  }
  const costo = EDIFICIO_CATALOGO[edificio.tipo].costo as Partial<Record<string, number>>;
  let almacen = asentamiento.almacen;
  for (const [recurso, cantidad] of Object.entries(costo)) {
    if (cantidad) almacen = agregarRecurso(almacen, recurso, cantidad);
  }
  return { ...asentamiento, almacen, edificios: asentamiento.edificios.filter((e) => e.id !== edificioId) };
}

/**
 * Mueve un proyecto `en_cola` una posición arriba/abajo en el orden de arranque (Doc 4.2, a petición del
 * usuario) — intercambia su `prioridad` con la del vecino inmediato en ese sentido dentro de la lista
 * ordenada por prioridad descendente (mismo orden que `avanzarConstruccion` usa para decidir quién arranca
 * obra primero). No hace nada si el proyecto ya está en el extremo correspondiente. La ubicación de
 * construcción nunca se ve afectada por este orden.
 */
export function moverEnCola(
  asentamiento: Asentamiento,
  cargo: 'gobernador' | 'maestroObras',
  edificioId: string,
  direccion: 'arriba' | 'abajo'
): Asentamiento {
  if (!cargoOcupado(asentamiento, cargo)) {
    throw new ConstruccionManualInvalidaError(`Se necesita un ${cargo} asignado para modificar la cola.`);
  }
  const enCola = asentamiento.edificios.filter((e) => e.estado === 'en_cola').sort((a, b) => (b.prioridad ?? 0) - (a.prioridad ?? 0));
  const indice = enCola.findIndex((e) => e.id === edificioId);
  if (indice === -1) throw new ConstruccionManualInvalidaError('Ese proyecto no está en la cola.');
  const vecinoIndice = direccion === 'arriba' ? indice - 1 : indice + 1;
  if (vecinoIndice < 0 || vecinoIndice >= enCola.length) return asentamiento;

  const actual = enCola[indice]!;
  const vecino = enCola[vecinoIndice]!;
  const prioridadActual = actual.prioridad ?? 0;
  const prioridadVecino = vecino.prioridad ?? 0;
  const edificios = asentamiento.edificios.map((e) => {
    if (e.id === actual.id) return { ...e, prioridad: prioridadVecino };
    if (e.id === vecino.id) return { ...e, prioridad: prioridadActual };
    return e;
  });
  return { ...asentamiento, edificios };
}

/** Estado de la próxima mejora de un edificio, para mostrar en UI y para validar `mejorarEdificioManualmente`
 * (mejora manual, a petición del usuario) — `null` si el edificio no tiene mejora posible (tipo sin niveles,
 * inactivo, o ya en su nivel máximo/gate de nivel de asentamiento o edificio previo no cumplido). Si no es
 * `null`, `elegible` indica si HOY hay fondos suficientes (misma reserva exenta de `reservaManual` que usa
 * `anadirEdificioManualmente` — la mejora manual es, igual que añadir a la cola, una acción exenta a propósito). */
export interface EstadoMejoraEdificio {
  nivelActual: number;
  nivelSiguiente: number;
  costo: Partial<Record<string, number>>;
  elegible: boolean;
  motivoBloqueo?: string;
}

export function estadoMejoraEdificio(
  asentamiento: Asentamiento,
  edificio: Edificio,
  capital: Asentamiento | undefined
): EstadoMejoraEdificio | null {
  const info = elegibleParaMejora(asentamiento, edificio);
  if (!info) return null;
  const reserva = reservaDinamicaConstruccion(asentamiento, capital);
  const elegible = puedeIniciarConstruccion(asentamiento.almacen, info.costo, edificio.tipo, reserva);
  return {
    ...info,
    elegible,
    motivoBloqueo: elegible ? undefined : 'No hay fondos suficientes (respetando la reserva de mantenimiento) para pagarla ahora.',
  };
}

/**
 * Fuerza la mejora de UN edificio concreto por decisión MANUAL de Gobernador o Maestro de Obras (Doc 4.2, a
 * petición del usuario — la mejora automática de `avanzarMejoras` sigue corriendo cada tick igual que antes;
 * esto solo permite adelantar la de un edificio elegido en vez de esperar a que el bucle automático llegue a
 * él). Mismos gates y costo que la ruta automática (`elegibleParaMejora`/`estadoMejoraEdificio`), incluida la
 * mudanza por crecimiento de huella (hoy solo Granja).
 */
export function mejorarEdificioManualmente(
  asentamiento: Asentamiento,
  cargo: 'gobernador' | 'maestroObras',
  edificioId: string,
  capital: Asentamiento | undefined
): Asentamiento {
  if (!cargoOcupado(asentamiento, cargo)) {
    throw new ConstruccionManualInvalidaError(`Se necesita un ${cargo} asignado para forzar una mejora.`);
  }
  const edificio = asentamiento.edificios.find((e) => e.id === edificioId);
  if (!edificio) throw new ConstruccionManualInvalidaError('Ese edificio no existe en este asentamiento.');
  if (edificio.estado !== 'activo') {
    throw new ConstruccionManualInvalidaError('Solo se puede forzar la mejora de un edificio activo.');
  }
  const estado = estadoMejoraEdificio(asentamiento, edificio, capital);
  if (!estado) throw new ConstruccionManualInvalidaError('Ya está en su nivel máximo (o no tiene mejoras disponibles).');
  if (!estado.elegible) throw new ConstruccionManualInvalidaError(estado.motivoBloqueo!);

  const { nivelActual, nivelSiguiente, costo } = estado;
  const tamanoActual = tamanoEdificio(edificio.tipo, nivelActual);
  const tamanoNuevo = tamanoEdificio(edificio.tipo, nivelSiguiente);
  let posicion = edificio.posicion;
  if (tamanoNuevo.ancho !== tamanoActual.ancho || tamanoNuevo.alto !== tamanoActual.alto) {
    const destino = reubicarPorTamano(asentamiento, edificio, asentamiento.edificios, nivelSiguiente);
    if (!destino) throw new ConstruccionManualInvalidaError('No hay espacio para reubicar el edificio en su nuevo tamaño.');
    posicion = destino;
  }

  const almacen = descontarRecursos(asentamiento.almacen, costo);
  let edificios = asentamiento.edificios.map((e) => (e.id === edificioId ? { ...e, nivelInterno: nivelSiguiente, posicion } : e));
  if (edificio.tipo === 'mercado') edificios = [...edificios, ...crearPuestosDeMercado(asentamiento, nivelSiguiente, edificios)];

  return { ...asentamiento, almacen, edificios };
}
