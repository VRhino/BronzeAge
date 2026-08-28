import type { Point, RegionId } from '../domain/types';
import { distancia, distanciaASegmento, type Limites } from './colocacion';

export type { RegionId };

/**
 * Modo geográfico (Fase 0.2 — generación regional): además del mundo libre de siempre (sin `region`, el
 * ruido fractal manda solo, comportamiento sin cambios), se puede pedir que el mundo se parezca a una zona
 * real del Egeo/Levante de la Edad de Bronce. NO es importación de datos de elevación reales (SRTM/GEBCO):
 * no hay forma de traer ese tipo de archivo geográfico a este proyecto. Es una GUÍA hecha a mano — un puñado
 * de crestas/bahías/cuencas que capturan el carácter conocido de la región (dónde está la cordillera
 * principal, dónde se fragmenta la costa, dónde hay una llanura fértil encajonada) — mezclada con el mismo
 * ruido fractal de siempre para que cada seed dentro de esa región siga dando un mundo distinto en el
 * detalle, no una copia exacta. `RegionId` en sí vive en `domain/types.ts` (regla de frontera).
 */

/** Cresta montañosa: sube la elevación cerca del segmento a-b, con caída suave hasta `anchoInfluencia`. */
export interface RasgoCresta {
  tipo: 'cresta';
  a: Point;
  b: Point;
  /** Fracción [0,1] del ancho/alto del mapa (promediados) — no unidades de mapa fijas, para que la guía no
   * dependa del tamaño exacto del mundo. */
  anchoInfluencia: number;
  /** Cuánto SUMA a la elevación en el propio segmento (decae a 0 en `anchoInfluencia`). */
  alturaPico: number;
}

/** Bahía/golfo: baja la elevación cerca del segmento a-b — es lo que fragmenta un continente liso en
 * penínsulas, cortando desde un borde hacia adentro. */
export interface RasgoBahia {
  tipo: 'bahia';
  a: Point;
  b: Point;
  anchoInfluencia: number;
  /** Cuánto RESTA a la elevación en el propio segmento (decae a 0 en `anchoInfluencia`). */
  profundidad: number;
}

/** Cuenca fértil: aplana la elevación hacia `alturaObjetivo` dentro de un radio — una llanura encajonada
 * entre montañas (Tesalia, Mesenia), no solo "tierra baja" sino tierra LLANA de verdad. */
export interface RasgoCuenca {
  tipo: 'cuenca';
  centro: Point;
  radio: number;
  alturaObjetivo: number;
}

/** Valle fluvial: como `cuenca`, pero a lo largo de un segmento a-b en vez de un punto — el corredor
 * llano/fértil que flanquea un río troncal (Nilo, Tigris, Éufrates). Deliberadamente ANGOSTO
 * (`anchoInfluencia` chico): la franja cultivable real junto a esos ríos es estrecha comparada con el
 * desierto/estepa que la rodea, no una llanura amplia como las `cuenca` de Grecia. */
export interface RasgoValle {
  tipo: 'valle';
  a: Point;
  b: Point;
  anchoInfluencia: number;
  alturaObjetivo: number;
}

export type RasgoGuia = RasgoCresta | RasgoBahia | RasgoCuenca | RasgoValle;

/**
 * Río troncal (Nilo, Tigris, Éufrates): a diferencia de los ríos normales (`generarRios` — nacen en una
 * montaña al azar y bajan por gradiente), un río de esta magnitud es un rasgo DEFINITORIO de la región, no
 * una consecuencia del relieve — así que se autora como trazo explícito (puntos de paso en fracciones
 * [0,1]) en vez de dejarlo al alba del descenso por pendiente. Siempre sale `navegable` y con desembocadura
 * real (ver `generarRios`) — no compite por el cupo de `RIOS.proporcionNavegable`, es navegable por
 * definición.
 */
export interface RioTroncalDef {
  id: string;
  /** Puntos de paso en fracciones [0,1], al menos 2 — de un borde del mapa al otro (o a una `cuenca`/`valle`
   * interior si el río muere en un delta/mar interior en vez de cruzar del todo). */
  puntos: Point[];
}

export interface RegionGeografica {
  /** Peso de la guía frente al ruido fractal en la mezcla final (0-1). Alto a propósito (ver `REGIONES`):
   * es la palanca de "qué tan reconocible" sale la región — con 1.0 todas las seeds de esa región saldrían
   * casi idénticas (el ruido no pintaría nada), con 0 sería indistinguible del mundo libre. */
  pesoGuia: number;
  /** Nivel de referencia (0-1) ANTES de sumar crestas/restar bahías/aplanar cuencas — es lo que decide qué
   * es el terreno "por defecto" de la región lejos de cualquier rasgo. Cada región tiene el suyo a propósito:
   * Mesopotamia/Nilo lo querrán bajo (llanura por defecto, montaña es la excepción); Grecia/Anatolia lo
   * quieren alto (colina/montaña por defecto, la llanura es la excepción — ver `GRECIA_CONTINENTAL`). */
  baseElevacion: number;
  rasgos: RasgoGuia[];
  /** Ríos troncales (Nilo, Tigris/Éufrates) — ausente o vacío en regiones sin uno (Grecia, Anatolia, Egeo).
   * Ver `RioTroncalDef` y `generarRios`. */
  riosTroncales?: RioTroncalDef[];
}

const S = (t: number): number => t * t * (3 - 2 * t);

/** Punto en coordenadas fraccionales [0,1] de `RasgoGuia` escalado a unidades de mapa reales. */
function escalar(p: Point, limites: Limites): Point {
  return { x: p.x * limites.ancho, y: p.y * limites.alto };
}

/**
 * Resuelve una región AUTORADA en fracciones [0,1] (`REGIONES`, fija, independiente del tamaño del mundo) a
 * unidades de mapa absolutas para un `limites` concreto. Se hace UNA VEZ en `generarCampoElevacion` — así
 * `evaluarGuia`/`evaluarElevacion` siguen sin necesitar `limites` en cada consulta, mismo contrato de firma
 * que tenían antes de que existieran regiones (nada fuera de `elevacion.ts` tiene que enterarse del cambio).
 */
export function resolverRegion(region: RegionGeografica, limites: Limites): RegionGeografica {
  const escala = (limites.ancho + limites.alto) / 2;
  return {
    pesoGuia: region.pesoGuia,
    baseElevacion: region.baseElevacion,
    rasgos: region.rasgos.map((rasgo) => {
      if (rasgo.tipo === 'cuenca') {
        return { ...rasgo, centro: escalar(rasgo.centro, limites), radio: rasgo.radio * escala };
      }
      return { ...rasgo, a: escalar(rasgo.a, limites), b: escalar(rasgo.b, limites), anchoInfluencia: rasgo.anchoInfluencia * escala };
    }),
    riosTroncales: region.riosTroncales?.map((rio) => ({ id: rio.id, puntos: rio.puntos.map((p) => escalar(p, limites)) })),
  };
}

/**
 * Elevación de la guía (0-1) en un punto — SIN mezclar con ruido todavía (ver `elevacion.ts`). Pura función
 * de `region` + `p`: mismos rasgos, mismo punto, mismo resultado siempre. Asume `region` YA RESUELTA
 * (`resolverRegion`) — en unidades de mapa absolutas, no en las fracciones con las que se autora en `REGIONES`.
 */
export function evaluarGuia(region: RegionGeografica, p: Point): number {
  let valor = region.baseElevacion;

  for (const rasgo of region.rasgos) {
    if (rasgo.tipo === 'cresta') {
      const d = distanciaASegmento(p, rasgo.a, rasgo.b);
      valor += rasgo.alturaPico * S(Math.min(1, Math.max(0, 1 - d / rasgo.anchoInfluencia)));
    } else if (rasgo.tipo === 'bahia') {
      const d = distanciaASegmento(p, rasgo.a, rasgo.b);
      valor -= rasgo.profundidad * S(Math.min(1, Math.max(0, 1 - d / rasgo.anchoInfluencia)));
    }
  }

  // Cuencas y valles se aplican AL FINAL y aparte del bucle: aplanan hacia su objetivo por encima de lo que
  // hayan hecho crestas/bahías cercanas, en vez de sumarse — son una llanura de verdad, no una colina baja.
  for (const rasgo of region.rasgos) {
    if (rasgo.tipo === 'cuenca') {
      const d = distancia(p, rasgo.centro);
      const factor = S(Math.min(1, Math.max(0, 1 - d / rasgo.radio)));
      valor = valor + (rasgo.alturaObjetivo - valor) * factor;
    } else if (rasgo.tipo === 'valle') {
      const d = distanciaASegmento(p, rasgo.a, rasgo.b);
      const factor = S(Math.min(1, Math.max(0, 1 - d / rasgo.anchoInfluencia)));
      valor = valor + (rasgo.alturaObjetivo - valor) * factor;
    }
  }

  return Math.min(1, Math.max(0, valor));
}

/**
 * Grecia continental (Fase 0.2, región piloto): cordillera central norte-sur (Pindo) con un espolón al sur
 * (Peloponeso), costa muy fragmentada por golfos que cortan desde varios bordes (Térmico/Pagaseo al
 * noreste, Sarónico al sureste, Corinto cortando casi de lado a lado, jónica al oeste), y dos llanuras
 * fértiles encajonadas (Tesalia al norte, Mesenia al sur) — el resto es ladera de montaña o costa estrecha,
 * deliberadamente poca llanura abierta.
 *
 * `baseElevacion=0.63` (banda 'montana', ver `ELEVACION` en `config.ts`), no un valor tipo "llano": el
 * primer intento (base en banda llana) daba 64% llano lejos de las 2 crestas — MENOS montañoso que el
 * mundo libre por defecto (~50%), justo lo contrario de Grecia real (~80% colina/montaña). Con la base ya
 * alta, el relieve por defecto ES montañoso en todas partes, y son las crestas/bahías/cuencas las que
 * tallan las EXCEPCIONES (picos aún más altos, costa, llanura). `anchoInfluencia` de las bahías ancho
 * (0.14-0.16, no 0.07-0.08) a propósito: con el salto de "fondo de bahía" a `baseElevacion` tan alto, un
 * borde estrecho daba casi todo agua-o-montaña sin franja de costa intermedia — más ancho deja un
 * degradado real por 'costa' antes de llegar a tierra alta.
 *
 * Medido sobre 3 seeds (ver script de calibración): agua+costa ~27-28%, llano ~16-18%,
 * colina+montaña+cima ~55-58% — frente al ~17-19%/~50-55%/~26-28% del mundo libre en esas mismas bandas.
 */
const GRECIA_CONTINENTAL: RegionGeografica = {
  pesoGuia: 0.68,
  baseElevacion: 0.63,
  rasgos: [
    { tipo: 'cresta', a: { x: 0.38, y: 0.05 }, b: { x: 0.45, y: 0.95 }, anchoInfluencia: 0.14, alturaPico: 0.18 },
    { tipo: 'cresta', a: { x: 0.32, y: 0.72 }, b: { x: 0.55, y: 0.92 }, anchoInfluencia: 0.11, alturaPico: 0.15 },
    { tipo: 'bahia', a: { x: 1.0, y: 0.28 }, b: { x: 0.55, y: 0.35 }, anchoInfluencia: 0.16, profundidad: 0.48 },
    { tipo: 'bahia', a: { x: 1.0, y: 0.62 }, b: { x: 0.5, y: 0.68 }, anchoInfluencia: 0.16, profundidad: 0.48 },
    { tipo: 'bahia', a: { x: 0.05, y: 0.7 }, b: { x: 0.75, y: 0.66 }, anchoInfluencia: 0.14, profundidad: 0.5 },
    { tipo: 'bahia', a: { x: 0.0, y: 0.45 }, b: { x: 0.25, y: 0.5 }, anchoInfluencia: 0.16, profundidad: 0.38 },
    { tipo: 'cuenca', centro: { x: 0.55, y: 0.22 }, radio: 0.1, alturaObjetivo: 0.48 },
    { tipo: 'cuenca', centro: { x: 0.4, y: 0.85 }, radio: 0.07, alturaObjetivo: 0.48 },
  ],
};

/**
 * Anatolia (Fase 0.2): meseta central elevada pero relativamente abierta (no tan quebrada como Grecia),
 * encajonada entre dos cordilleras casi paralelas cerca de la costa — Póntica al norte, Tauro al sur — con
 * una franja costera fragmentada al oeste (egea) y bordes norte/sur más cerrados (Mar Negro/Mediterráneo).
 *
 * A diferencia de Grecia (montaña en TODAS partes salvo excepciones), aquí la base es de meseta (`llano`
 * alto/`colina` bajo) y son las DOS crestas horizontales anchas las que hacen de pared norte/sur — la meseta
 * en sí queda relativamente caminable, que es justo el contraste con Grecia que se busca.
 */
const ANATOLIA: RegionGeografica = {
  pesoGuia: 0.55,
  baseElevacion: 0.5,
  rasgos: [
    { tipo: 'cresta', a: { x: 0.0, y: 0.1 }, b: { x: 1.0, y: 0.14 }, anchoInfluencia: 0.2, alturaPico: 0.32 },
    { tipo: 'cresta', a: { x: 0.05, y: 0.88 }, b: { x: 0.95, y: 0.85 }, anchoInfluencia: 0.2, alturaPico: 0.34 },
    { tipo: 'bahia', a: { x: 0.0, y: 0.35 }, b: { x: 0.22, y: 0.4 }, anchoInfluencia: 0.14, profundidad: 0.35 },
    { tipo: 'bahia', a: { x: 0.0, y: 0.62 }, b: { x: 0.2, y: 0.58 }, anchoInfluencia: 0.14, profundidad: 0.35 },
    { tipo: 'cuenca', centro: { x: 0.55, y: 0.5 }, radio: 0.28, alturaObjetivo: 0.49 },
  ],
};

/**
 * Egeo (Fase 0.2): archipiélago — mayormente mar, con islas dispersas. Mecanismo distinto al resto: en vez
 * de "tierra por defecto, agua como excepción" (Grecia/Anatolia), aquí es al revés — `baseElevacion` cae en
 * banda 'agua' y cada isla es una `cresta` de tipo PUNTO (`a === b`, el caso degenerado de
 * `distanciaASegmento` es simplemente distancia al punto) que empuja un bulto de tierra por encima del mar.
 * Tamaños variados a propósito (islotes chicos junto a un par de islas grandes tipo Creta/Rodas) para que no
 * se lea como una rejilla uniforme de puntos.
 */
const EGEO: RegionGeografica = {
  pesoGuia: 0.7,
  baseElevacion: 0.25,
  rasgos: [
    { tipo: 'cresta', a: { x: 0.5, y: 0.25 }, b: { x: 0.5, y: 0.25 }, anchoInfluencia: 0.16, alturaPico: 0.55 },
    { tipo: 'cresta', a: { x: 0.68, y: 0.85 }, b: { x: 0.68, y: 0.85 }, anchoInfluencia: 0.2, alturaPico: 0.5 },
    { tipo: 'cresta', a: { x: 0.2, y: 0.35 }, b: { x: 0.2, y: 0.35 }, anchoInfluencia: 0.09, alturaPico: 0.4 },
    { tipo: 'cresta', a: { x: 0.75, y: 0.2 }, b: { x: 0.75, y: 0.2 }, anchoInfluencia: 0.07, alturaPico: 0.38 },
    { tipo: 'cresta', a: { x: 0.35, y: 0.6 }, b: { x: 0.35, y: 0.6 }, anchoInfluencia: 0.08, alturaPico: 0.4 },
    { tipo: 'cresta', a: { x: 0.55, y: 0.55 }, b: { x: 0.55, y: 0.55 }, anchoInfluencia: 0.06, alturaPico: 0.35 },
    { tipo: 'cresta', a: { x: 0.85, y: 0.55 }, b: { x: 0.85, y: 0.55 }, anchoInfluencia: 0.07, alturaPico: 0.38 },
    { tipo: 'cresta', a: { x: 0.15, y: 0.75 }, b: { x: 0.15, y: 0.75 }, anchoInfluencia: 0.06, alturaPico: 0.35 },
    { tipo: 'cresta', a: { x: 0.45, y: 0.15 }, b: { x: 0.45, y: 0.15 }, anchoInfluencia: 0.06, alturaPico: 0.33 },
  ],
};

/**
 * Nilo (Fase 0.2): un único río troncal cruzando el mapa de borde a borde (sur→norte, con una curva suave —
 * no una línea recta, para que se lea como cauce natural), flanqueado por una franja fértil ANGOSTA
 * (`valle`), y desierto llano (no montañoso: `baseElevacion` en banda 'llano') a ambos lados — la aridez es
 * cosa de `evaluarBioma`/fertilidad, no de relieve, así que el terreno en sí se mantiene plano.
 */
const NILO: RegionGeografica = {
  pesoGuia: 0.55,
  baseElevacion: 0.47,
  rasgos: [
    { tipo: 'valle', a: { x: 0.52, y: 0.02 }, b: { x: 0.48, y: 0.35 }, anchoInfluencia: 0.035, alturaObjetivo: 0.46 },
    { tipo: 'valle', a: { x: 0.48, y: 0.35 }, b: { x: 0.55, y: 0.65 }, anchoInfluencia: 0.035, alturaObjetivo: 0.46 },
    { tipo: 'valle', a: { x: 0.55, y: 0.65 }, b: { x: 0.5, y: 0.98 }, anchoInfluencia: 0.035, alturaObjetivo: 0.46 },
    { tipo: 'cresta', a: { x: 1.0, y: 0.3 }, b: { x: 0.95, y: 0.7 }, anchoInfluencia: 0.15, alturaPico: 0.22 },
  ],
  riosTroncales: [
    {
      id: 'nilo',
      puntos: [
        { x: 0.52, y: 0.02 },
        { x: 0.48, y: 0.35 },
        { x: 0.55, y: 0.65 },
        { x: 0.5, y: 0.98 },
      ],
    },
  ],
};

/**
 * Mesopotamia (Fase 0.2): DOS ríos troncales casi paralelos (Tigris/Éufrates) que convergen hacia un borde
 * (el golfo, real Shatt al-Arab), cada uno con su propia franja fértil angosta — entre ambos, el terreno
 * llano de la `baseElevacion` ya lee como planicie cultivable sin necesitar una `cuenca` aparte (están lo
 * bastante cerca para que la humedad de cualquiera de los dos alcance la franja intermedia, ver
 * `BIOMA.radioHumedadRio`). Cordillera de Zagros como pared este, borde real de la llanura mesopotámica.
 */
const MESOPOTAMIA: RegionGeografica = {
  pesoGuia: 0.5,
  baseElevacion: 0.46,
  rasgos: [
    { tipo: 'valle', a: { x: 0.35, y: 0.02 }, b: { x: 0.45, y: 0.5 }, anchoInfluencia: 0.03, alturaObjetivo: 0.45 },
    { tipo: 'valle', a: { x: 0.45, y: 0.5 }, b: { x: 0.55, y: 0.98 }, anchoInfluencia: 0.03, alturaObjetivo: 0.45 },
    { tipo: 'valle', a: { x: 0.55, y: 0.02 }, b: { x: 0.58, y: 0.5 }, anchoInfluencia: 0.03, alturaObjetivo: 0.45 },
    { tipo: 'valle', a: { x: 0.58, y: 0.5 }, b: { x: 0.56, y: 0.98 }, anchoInfluencia: 0.03, alturaObjetivo: 0.45 },
    { tipo: 'cresta', a: { x: 1.0, y: 0.0 }, b: { x: 0.92, y: 1.0 }, anchoInfluencia: 0.18, alturaPico: 0.3 },
  ],
  riosTroncales: [
    {
      id: 'tigris',
      puntos: [
        { x: 0.35, y: 0.02 },
        { x: 0.45, y: 0.5 },
        { x: 0.55, y: 0.98 },
      ],
    },
    {
      id: 'eufrates',
      puntos: [
        { x: 0.55, y: 0.02 },
        { x: 0.58, y: 0.5 },
        { x: 0.56, y: 0.98 },
      ],
    },
  ],
};

export const REGIONES: Record<RegionId, RegionGeografica> = {
  greciaContinental: GRECIA_CONTINENTAL,
  anatolia: ANATOLIA,
  egeo: EGEO,
  nilo: NILO,
  mesopotamia: MESOPOTAMIA,
};
