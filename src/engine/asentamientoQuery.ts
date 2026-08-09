import type { Asentamiento, Edificio, EdificioTipo } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { EDIFICIO_CATALOGO, NIVEL_ASENTAMIENTO } from '../constants';
import { cupoCaravanaExtra, factorProduccionTrigo } from './politicas';

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
 * asentamiento tenga construida, esté 'disponible' o 'en_transito' (ver `engine/trade.ts`).
 */
export function cupoCaravanas(asentamiento: Asentamiento): number {
  const mercado = edificiosPorTipoYEstado(asentamiento, 'mercado')[0];
  if (!mercado) return 0;
  const niveles = (EDIFICIO_CATALOGO.mercado as { niveles?: Record<number, { cupoCaravanas?: number }> }).niveles;
  const base = niveles?.[nivelInternoActual(mercado)]?.cupoCaravanas ?? 0;
  return base + cupoCaravanaExtra(asentamiento);
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
    edificiosFaltantes: string[];
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
      edificiosRequeridos: requisito.edificios.length,
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

export interface ProduccionItem {
  tipo: EdificioTipo;
  recurso: string;
  activos: number;
  cantidadPorTick: number;
}

/**
 * Producción por tick de cada edificio activo de extracción/producción primaria, agrupada por tipo
 * de edificio. Solo lectura: no descuenta nodos de recurso (a diferencia de `avanzarConstruccion`,
 * que sí llama a `mapa.extraer` al aplicar la producción real).
 */
export function produccionPorTick(asentamiento: Asentamiento, mapa: Mapa): ProduccionItem[] {
  const ratioMano = ratioManoObra(asentamiento);
  const items: ProduccionItem[] = [];

  const granjas = edificiosPorTipoYEstado(asentamiento, 'granja');
  if (granjas.length) {
    const factorTrigo = factorProduccionTrigo(asentamiento);
    const total = granjas.reduce(
      (acc, e) => acc + EDIFICIO_CATALOGO.granja.produccionBaseTrigo * mapa.fertilidadEn(e.posicion) * ratioMano * factorTrigo,
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

  return items;
}
