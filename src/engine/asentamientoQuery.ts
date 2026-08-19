import type { Asentamiento, Edificio, EdificioTipo, Point } from '../domain/types';
import type { Mapa } from '../world/mapa';
import {
  CARAVANA_COOLDOWN,
  EDIFICIO_CATALOGO,
  produccionTrigoDeGranja,
  NIVEL_ASENTAMIENTO,
  POBLACION,
  type RecetaProduccion,
} from '../constants';
import { cupoCaravanaExtra, factorProduccionTrigo } from './politicas';
import { mejorFertilidadEnZona } from './zones';

/** Nivel ACTUAL / operativo (Doc Fase_0_5 §6.2): gates de construcción/mejora/reclutamiento/expansión leen
 * este valor, no `asentamiento.nivel` (nivelAlcanzado) directamente — `nivelActual` puede bajar tras un
 * fallo de mantenimiento sostenido, `nivel` nunca. Ausente (partidas guardadas antes de este campo) = igual
 * a `nivel`, sin degradación previa que reconstruir. */
export function nivelActualDe(asentamiento: Asentamiento): number {
  return asentamiento.nivelActual ?? asentamiento.nivel;
}

export function edificiosPorTipoYEstado(
  asentamiento: Asentamiento,
  tipo: EdificioTipo,
  estado: 'activo' | 'en_construccion' | 'en_cola' = 'activo'
) {
  return asentamiento.edificios.filter((e) => e.tipo === tipo && e.estado === estado);
}

export function hayProyectoPendiente(asentamiento: Asentamiento, tipo: EdificioTipo): boolean {
  return asentamiento.edificios.some((e) => e.tipo === tipo && e.estado !== 'activo');
}

/** Cupo de Pesants (Doc 4.1/4.2.1): SEPARADO del de Artesanos — cada Vivienda activa aporta el suyo propio. */
export function capacidadViviendaPesants(asentamiento: Asentamiento): number {
  return edificiosPorTipoYEstado(asentamiento, 'vivienda').length * EDIFICIO_CATALOGO.vivienda.capacidadPesants;
}

/** Cupo de Artesanos (Doc 4.1/4.2.1): SEPARADO del de Pesants, mismo edificio Vivienda pero otro sub-pool —
 * evita que Pesants (crece ~2.4x más rápido) acapare todo el cupo y deje a Artesanos varado. */
export function capacidadViviendaArtesanos(asentamiento: Asentamiento): number {
  return edificiosPorTipoYEstado(asentamiento, 'vivienda').length * EDIFICIO_CATALOGO.vivienda.capacidadArtesanos;
}

export function poblacionTotal(asentamiento: Asentamiento): number {
  const { pesants, artesanos, nobleza } = asentamiento.poblacion;
  return pesants + artesanos + nobleza;
}

/** Nutrición de la población (Doc 4.1, hambruna): ausente (partidas guardadas antes de este campo) = 100,
 * mismo criterio que `nivelActualDe` para campos opcionales nuevos. */
export function nutricionPoblacionDe(asentamiento: Asentamiento): number {
  return asentamiento.nutricionPoblacion ?? POBLACION.hambre.nutricionInicial;
}

/** Ampliación de comercio (a petición del usuario, Doc 3.3): sin Mercado activo no se pueden colocar
 * órdenes de mercado ni construir caravanas propias — ver `colocarOrdenMercado` (engine/market.ts) y
 * `construirCaravanaComercial` (engine/trade.ts). */
export function tieneMercadoActivo(asentamiento: Asentamiento): boolean {
  return edificiosPorTipoYEstado(asentamiento, 'mercado').length > 0;
}

/**
 * Cupo de caravanas propias (ampliación de comercio, a petición del usuario): `cupoCaravanas` del nivel
 * interno actual de Mercado + el bonus aditivo de la política "Ampliación de Flota" (`cupoCaravanaExtra`).
 * 0 si no hay Mercado activo. Cuenta contra este cupo cualquier caravana `tipo: 'comercial'` que el
 * asentamiento tenga construida, en CUALQUIER estado — 'disponible', 'en_transito' o 'retornando' (ver
 * `construirCaravanaComercial`, engine/trade.ts, que no filtra por estado): es un activo persistente del
 * asentamiento pase lo que pase, nunca deja de contar mientras exista.
 */
export function cupoCaravanas(asentamiento: Asentamiento): number {
  const mercado = edificiosPorTipoYEstado(asentamiento, 'mercado')[0];
  if (!mercado) return 0;
  const niveles = (EDIFICIO_CATALOGO.mercado as { niveles?: Record<number, { cupoCaravanas?: number }> }).niveles;
  const base = niveles?.[nivelInternoActual(mercado)]?.cupoCaravanas ?? 0;
  return base + cupoCaravanaExtra(asentamiento);
}

/**
 * Ticks que faltan para que este asentamiento pueda crear otra caravana (Fundación o comercial) — 0 si ya
 * puede. Cooldown COMPARTIDO entre los dos mecanismos (`CARAVANA_COOLDOWN.ticksCooldown`, a petición del
 * usuario): evita spam de creación cuando una caravana recién salida es destruida y el cupo/recursos vuelven a
 * estar disponibles de inmediato. Informativo para la UI (`gameStore.caravanasInfo`); la comprobación real que
 * bloquea la creación vive en `lanzarCaravanaFundacion` (engine/expansion.ts) y `construirCaravanaComercial`
 * (engine/trade.ts), que llaman a `puedeCrearCaravana` más abajo.
 */
export function ticksCooldownCaravanaRestantes(asentamiento: Pick<Asentamiento, 'ultimaCaravanaCreadaEnTick'>, tickActual: number): number {
  if (asentamiento.ultimaCaravanaCreadaEnTick === undefined) return 0;
  return Math.max(0, asentamiento.ultimaCaravanaCreadaEnTick + CARAVANA_COOLDOWN.ticksCooldown - tickActual);
}

/** ¿Puede este asentamiento crear una caravana nueva (Fundación o comercial) ahora mismo? Ver
 * `ticksCooldownCaravanaRestantes`. */
export function puedeCrearCaravana(asentamiento: Pick<Asentamiento, 'ultimaCaravanaCreadaEnTick'>, tickActual: number): boolean {
  return ticksCooldownCaravanaRestantes(asentamiento, tickActual) === 0;
}

const EDIFICIOS_PRODUCTORES: EdificioTipo[] = ['granja', 'cantera', 'lenera', 'mina', 'minaCobre', 'minaEstano', 'corral'];

/** Edificios de transformación con tiers (Doc 4.2.1, rediseño de progreso Fase 0): consumen mano de obra de
 * Artesanos (no Pesants), disparan su aparición (ver `engine/population.ts`), y cuentan para los gates de
 * nivel de asentamiento. Exportado: `population.ts` lo usa para saber si ya hay al menos uno activo. */
export const EDIFICIOS_TRANSFORMACION: EdificioTipo[] = ['fundicion', 'curtiduria', 'armeria', 'carpinteria'];

function nivelInternoActual(edificio: Pick<Edificio, 'nivelInterno'>): number {
  return edificio.nivelInterno ?? 1;
}

/**
 * Suma de `trabajadoresRequeridos` (según el `nivelInterno` actual de cada uno) de los edificios de
 * transformación activos — demanda de mano de obra que escala la producción real (`ratioManoObraArtesanos`).
 * Ya NO limita cuánta población de Artesanos puede aparecer (rediseño a petición del usuario, ver
 * `crecerPoblacion` en `engine/population.ts`: ahora solo necesita el primer edificio de transformación
 * activo para empezar a crecer, sin tope ligado a este número).
 */
function trabajadoresRequeridosTransformacion(asentamiento: Asentamiento): number {
  const activos = EDIFICIOS_TRANSFORMACION.flatMap((tipo) => edificiosPorTipoYEstado(asentamiento, tipo));
  return activos.reduce((acc, e) => {
    const catalogo = EDIFICIO_CATALOGO[e.tipo] as { niveles?: Record<number, { trabajadoresRequeridos: number }> };
    const nivel = catalogo.niveles?.[nivelInternoActual(e)];
    return acc + (nivel?.trabajadoresRequeridos ?? 0);
  }, 0);
}

/** Ratio de mano de obra de Artesanos (0-1) para los edificios de transformación, mismo criterio que `ratioManoObra`. */
export function ratioManoObraArtesanos(asentamiento: Asentamiento): number {
  const requeridos = trabajadoresRequeridosTransformacion(asentamiento);
  return requeridos <= 0 ? 1 : Math.min(1, asentamiento.poblacion.artesanos / requeridos);
}

export interface ProgresoNivelAsentamiento {
  nivel: number;
  esMaximo: boolean;
  siguiente?: {
    nivelObjetivo: number;
    pesants: { actual: number; requerido: number };
    artesanos: { actual: number; requerido: number };
    /** Candidatos del conjunto que todavía NO están construidos — informativo: con `edificiosMinimo` (Doc
     * Fase_0_6) no hace falta tenerlos TODOS, basta con `edificiosConstruidos >= edificiosRequeridos`. */
    edificiosFaltantes: string[];
    /** Cuántos tipos DISTINTOS del conjunto ya están construidos — junto a `edificiosRequeridos` reemplaza
     * el cálculo previo "requeridos - faltantes.length", que asumía "hacen falta todos" y rompía con gates
     * de tipo "al menos N de M" (Doc Fase_0_6, nivel 2: 3 de 6 edificios de extracción). */
    edificiosConstruidos: number;
    edificiosRequeridos: number;
  };
}

/**
 * Progreso hacia el siguiente nivel de asentamiento (modelo de gates, Doc 4.5, rediseño de progreso Fase 0):
 * población actual vs. requerida y qué edificios de la lista todavía faltan por tener activos. La usa
 * `gameStore.nivelAsentamientoInfo` (UI) y `engine/mantenimiento.ts` (decidir si se sube de nivel).
 */
export function progresoNivelAsentamiento(asentamiento: Asentamiento): ProgresoNivelAsentamiento {
  if (asentamiento.nivel >= NIVEL_ASENTAMIENTO.nivelMaximo) {
    return { nivel: asentamiento.nivel, esMaximo: true };
  }
  const nivelObjetivo = asentamiento.nivel + 1;
  const requisito = NIVEL_ASENTAMIENTO.requisitos[nivelObjetivo];
  if (!requisito) return { nivel: asentamiento.nivel, esMaximo: true };
  const edificiosFaltantes = requisito.edificios.filter(
    (tipo) => edificiosPorTipoYEstado(asentamiento, tipo as EdificioTipo).length === 0
  );
  return {
    nivel: asentamiento.nivel,
    esMaximo: false,
    siguiente: {
      nivelObjetivo,
      pesants: { actual: asentamiento.poblacion.pesants, requerido: requisito.pesants },
      artesanos: { actual: asentamiento.poblacion.artesanos, requerido: requisito.artesanos },
      edificiosFaltantes,
      edificiosConstruidos: requisito.edificios.length - edificiosFaltantes.length,
      edificiosRequeridos: requisito.edificiosMinimo ?? requisito.edificios.length,
    },
  };
}

/**
 * Suma de `trabajadoresRequeridos` de todos los edificios activos de extracción/producción primaria
 * (granja+cantera+leñera+3 minas). No hay asignación por edificio: es una única demanda agregada de
 * mano de obra para todo el asentamiento (ver `ratioManoObra`).
 */
function trabajadoresRequeridosTotal(asentamiento: Asentamiento): number {
  const activos = EDIFICIOS_PRODUCTORES.flatMap((tipo) => edificiosPorTipoYEstado(asentamiento, tipo));
  return activos.reduce(
    (acc, e) => acc + (EDIFICIO_CATALOGO[e.tipo] as { trabajadoresRequeridos?: number }).trabajadoresRequeridos!,
    0
  );
}

/**
 * Ratio de mano de obra disponible (0-1) para los edificios de extracción/producción primaria:
 * la producción de cada uno se escala por este ratio cuando faltan pesants para cubrir los
 * `trabajadoresRequeridos` combinados (ver misma fórmula en `avanzarConstruccion`, construction.ts).
 */
export function ratioManoObra(asentamiento: Asentamiento): number {
  const trabajadoresRequeridos = trabajadoresRequeridosTotal(asentamiento);
  return trabajadoresRequeridos <= 0 ? 1 : Math.min(1, asentamiento.poblacion.pesants / trabajadoresRequeridos);
}

export interface ManoObraInfo {
  pesants: number;
  trabajadoresRequeridos: number;
  ocupados: number;
  excedente: number;
  ratioMano: number;
}

/**
 * Desglose informativo de la demanda de mano de obra: no representa una asignación real por edificio
 * (el motor no la tiene, ver `ratioManoObra`) sino cuántos pesants "cubrirían" la demanda agregada de
 * trabajadoresRequeridos, y cuántos quedan sin absorber por ningún edificio productor.
 */
export function manoObraInfo(asentamiento: Asentamiento): ManoObraInfo {
  const pesants = asentamiento.poblacion.pesants;
  const trabajadoresRequeridos = trabajadoresRequeridosTotal(asentamiento);
  const ocupados = Math.min(pesants, trabajadoresRequeridos);
  const excedente = Math.max(0, pesants - trabajadoresRequeridos);
  const ratioMano = trabajadoresRequeridos <= 0 ? 1 : Math.min(1, pesants / trabajadoresRequeridos);
  return { pesants, trabajadoresRequeridos, ocupados, excedente, ratioMano };
}

/**
 * Pool real de reclutamiento (a petición del usuario, tras encontrar en pruebas asentamientos colapsando
 * porque el NPC reclutaba población que ya estaba cubriendo producción): población general MENOS la que ya
 * hace falta para cubrir la demanda de mano de obra vigente — para pesants, exactamente
 * `manoObraInfo(asentamiento).excedente` (el mismo número que la UI ya le mostraba al jugador como "Pool de
 * pesants para reclutamiento", `ui`/`main.ts`, sin que el motor lo hiciera cumplir); para artesanos, el
 * equivalente contra `trabajadoresRequeridosTransformacion` (fundición/curtiduría/armería/carpintería).
 *
 * Regla del MOTOR, no del NPC (`engine/tropas.ts`, `reclutarTropa`): aplica igual al reclutamiento manual y al
 * de la gobernanza NPC — mismo criterio que la reserva de trigo antes de reclutar.
 */
export function poblacionDisponibleParaReclutar(asentamiento: Asentamiento, origen: 'pesants' | 'artesanos'): number {
  if (origen === 'artesanos') {
    return Math.max(0, asentamiento.poblacion.artesanos - trabajadoresRequeridosTransformacion(asentamiento));
  }
  return manoObraInfo(asentamiento).excedente;
}

export interface ProduccionItem {
  tipo: EdificioTipo;
  recurso: string;
  activos: number;
  cantidadPorTick: number;
}

/**
 * Producción de recursos intermedios (lingotes/cuero/armas/armaduras) de los edificios de
 * transformación activos (Fundición/Curtiduría/Armería) para el tick actual — mismo criterio que
 * `avanzarRecetas` (engine/construction.ts, que sí aplica el cambio real al almacén): recorre las
 * recetas del `nivelInterno` en orden sobre una copia local de las cantidades disponibles, así que
 * una receta puede consumir el output de otra calculada antes en este mismo tick (ej. Lingote de
 * Bronce sobre Lingote de Cobre/Estaño). Solo lectura: no toca `asentamiento.almacen`.
 */
function produccionRecetas(asentamiento: Asentamiento): ProduccionItem[] {
  const ratioArtesano = ratioManoObraArtesanos(asentamiento);
  const disponible = new Map<string, number>();
  for (const [recurso, r] of Object.entries(asentamiento.almacen)) disponible.set(recurso, r.cantidad);

  const items: ProduccionItem[] = [];
  for (const edificio of asentamiento.edificios) {
    if (edificio.estado !== 'activo') continue;
    const niveles = (EDIFICIO_CATALOGO[edificio.tipo] as { niveles?: Record<number, { recetas: RecetaProduccion[] }> })
      .niveles;
    const nivel = niveles?.[edificio.nivelInterno ?? 1];
    if (!nivel) continue;

    for (const receta of nivel.recetas) {
      let cantidad = receta.produccionBase * ratioArtesano;
      for (const [insumo, porUnidad] of Object.entries(receta.consumePorUnidad)) {
        if (!porUnidad) continue;
        cantidad = Math.min(cantidad, (disponible.get(insumo) ?? 0) / porUnidad);
      }
      if (cantidad <= 0) continue;

      for (const [insumo, porUnidad] of Object.entries(receta.consumePorUnidad)) {
        if (porUnidad) disponible.set(insumo, (disponible.get(insumo) ?? 0) - porUnidad * cantidad);
      }
      disponible.set(receta.produce, (disponible.get(receta.produce) ?? 0) + cantidad);

      const existente = items.find((it) => it.tipo === edificio.tipo && it.recurso === receta.produce);
      if (existente) existente.cantidadPorTick += cantidad;
      else items.push({ tipo: edificio.tipo, recurso: receta.produce, activos: edificiosPorTipoYEstado(asentamiento, edificio.tipo).length, cantidadPorTick: cantidad });
    }
  }
  return items;
}

/**
 * Producción por tick de cada edificio activo de extracción/producción primaria Y de transformación
 * (recursos intermedios, ver `produccionRecetas`), agrupada por tipo de edificio. Solo lectura: no
 * descuenta nodos de recurso ni almacén (a diferencia de `avanzarConstruccion`/`avanzarRecetas`, que
 * sí aplican la producción real).
 */
export function produccionPorTick(asentamiento: Asentamiento, mapa: Mapa, zonaPoligono: Point[] = []): ProduccionItem[] {
  const ratioMano = ratioManoObra(asentamiento);
  const items: ProduccionItem[] = [];

  const granjas = edificiosPorTipoYEstado(asentamiento, 'granja');
  if (granjas.length) {
    const factorTrigo = factorProduccionTrigo(asentamiento);
    // Vista de Asentamiento: las Granjas viven en el espacio plano local (sin fertilidad propia) y rinden
    // todas con la MEJOR fertilidad que la zona toca en el mapa general (ver `mejorFertilidadEnZona`).
    const fertilidadZona = mejorFertilidadEnZona(asentamiento, zonaPoligono, mapa);
    // Por granja, no `nº granjas × base`: cada una rinde según su propio nivel interno (§7 del trazado urbano,
    // ver `produccionTrigoDeGranja`).
    const total = granjas.reduce(
      (acc, e) => acc + produccionTrigoDeGranja(e.nivelInterno) * fertilidadZona * ratioMano * factorTrigo,
      0
    );
    items.push({ tipo: 'granja', recurso: 'trigo', activos: granjas.length, cantidadPorTick: total });
  }

  const leneras = edificiosPorTipoYEstado(asentamiento, 'lenera');
  if (leneras.length) {
    const total = leneras.reduce((acc, e) => {
      const bosque = mapa.bosque(e.fuenteId);
      return acc + (bosque ? EDIFICIO_CATALOGO.lenera.produccionBaseMadera * bosque.densidad * ratioMano : 0);
    }, 0);
    items.push({ tipo: 'lenera', recurso: 'madera', activos: leneras.length, cantidadPorTick: total });
  }

  const minado: { tipo: 'cantera' | 'mina' | 'minaCobre' | 'minaEstano' | 'corral'; recurso: string; base: number }[] = [
    { tipo: 'cantera', recurso: 'piedra', base: EDIFICIO_CATALOGO.cantera.produccionBasePiedra },
    { tipo: 'mina', recurso: 'oro', base: EDIFICIO_CATALOGO.mina.produccionBaseOro },
    { tipo: 'minaCobre', recurso: 'cobre', base: EDIFICIO_CATALOGO.minaCobre.produccionBaseCobre },
    { tipo: 'minaEstano', recurso: 'estano', base: EDIFICIO_CATALOGO.minaEstano.produccionBaseEstano },
    { tipo: 'corral', recurso: 'livestock', base: EDIFICIO_CATALOGO.corral.produccionBaseLivestock },
  ];
  for (const { tipo, recurso, base } of minado) {
    const edificios = edificiosPorTipoYEstado(asentamiento, tipo);
    if (!edificios.length) continue;
    const total = edificios.reduce((acc, e) => {
      // Lo que queda en el yacimiento, no lo que tenía al generarse: un nodo casi agotado rinde ese resto
      // y no su tasa nominal — mismo tope que aplica la producción real (`mapa.extraer`).
      const restante = mapa.stock(e.fuenteId);
      if (restante <= 0) return acc;
      return acc + Math.min(base * ratioMano, restante);
    }, 0);
    items.push({ tipo, recurso, activos: edificios.length, cantidadPorTick: total });
  }

  return [...items, ...produccionRecetas(asentamiento)];
}
