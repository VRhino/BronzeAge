// Campamentos de bandidos (Doc 1.9, a petición del usuario — inspirado en análisis comparativo con Travian):
// amenaza NPC ambiental, distinta del combate entre Facciones (Doc 5, engine/combate.ts). Este módulo cubre
// las dos partes AUTOMÁTICAS de la mecánica (spawn/respawn y ataque a caravanas cercanas, ambas evaluadas
// cada tick dentro de `avanzarSimulacion`); el ataque contra un campamento, con una columna que llegue a él, vive en
// `engine/combate.ts` (`atacarCampamentoConColumna`), junto al resto de resolución de combate.

import type { Asentamiento, CampamentoBandido, CampamentoMercenarios, Escuadron, Faccion, Heroe, NivelBandidos, Point, ZonaBosque, ZonaInfluencia } from '../domain/types';
import { esCiudadano } from './faccion';
import { enProteccionDeCampamento } from './mercenarios';
import { alCampamento, type CaravanaConEscolta, type EjercitoConTropa } from './tropa';
import type { EventoCrudo } from '../domain/eventos';

/** Fase A5 — payloads de los eventos de este subsistema (ver `avanzarSpawnBandidos`/`avanzarAtaquesBandidos`). */
export interface PayloadCampamentoAparece {
  campamentoId: string;
  nivel: NivelBandidos;
  /** El asentamiento al que acosa, o el campamento de mercenarios en cuyo anillo aparece (D42). */
  asentamientoObjetivoId?: string;
  campamentoMercenariosId?: string;
}
export interface PayloadCaravanaInterceptada {
  campamentoId: string;
  caravanaId: string;
}
export interface PayloadCaravanaEscapa {
  caravanaId: string;
}
import type { Mapa } from '../world/mapa';
import type { RandomFn } from '../worldgen';
import { CAMPAMENTOS_BANDIDOS, MILITAR } from '../constants';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import { enRefugio, pointInPolygon } from './zones';
import { aplicarBajas, poderTotal } from './combate';

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/**
 * Bosque no reclamado (Doc 1.9) MÁS CERCANO a un punto de referencia — sin ninguna zona de influencia
 * solapando su centro (territorio no reclamado por ninguna Facción). Misma simplificación que
 * `posicionLibreParaFundar` (engine/zones.ts): chequea el centro del bosque, no su círculo completo.
 */
function bosqueNoReclamadoMasCercano(mapa: Mapa, zonas: ZonaInfluencia[], ocupados: Set<string>, cercaDe: Point): ZonaBosque | undefined {
  const candidatos = mapa
    .listarBosques()
    .filter((b) => !ocupados.has(b.id) && !zonas.some((z) => pointInPolygon(b.centro, z.poligono)));
  if (candidatos.length === 0) return undefined;
  return candidatos.reduce((mejor, b) => (distancia(b.centro, cercaDe) < distancia(mejor.centro, cercaDe) ? b : mejor));
}

/** Un nivel al azar, con los pesos de `CAMPAMENTOS_BANDIDOS.niveles` (D21, D37). */
function nivelAlAzar(rng: RandomFn): NivelBandidos {
  let tirada = rng();
  for (const [nivel, { peso }] of Object.entries(CAMPAMENTOS_BANDIDOS.niveles)) {
    if ((tirada -= peso) < 0) return Number(nivel) as NivelBandidos;
  }
  return 1;
}

const conNivel = (nivel: NivelBandidos) => ({ nivel, poder: CAMPAMENTOS_BANDIDOS.niveles[nivel].poder });

/** Cuántos residentes del campamento son de una Facción sin asentamiento, o de ninguna: los que necesitan bandidos (D42). */
function residentesSinPlaza(campamento: CampamentoMercenarios, facciones: readonly Faccion[], asentamientos: readonly Asentamiento[]): number {
  return campamento.residentesIds.filter((id) => {
    const faccion = facciones.find((f) => esCiudadano(f, id));
    return !faccion || !asentamientos.some((a) => a.faccionId === faccion.id);
  }).length;
}

/** Un punto del anillo de un campamento de mercenarios donde pueden acampar bandidos: tierra firme, fuera de toda zona y fuera de la
 * protección de cualquier campamento. Unos cuantos intentos al azar; sin sitio, nada (se reintenta en la próxima cita). */
function puntoDelAnillo(centro: Point, mapa: Mapa, zonas: readonly ZonaInfluencia[], mercenarios: readonly CampamentoMercenarios[], rng: RandomFn): Point | undefined {
  const { radioMin, radioMax } = CAMPAMENTOS_BANDIDOS.anillo;
  for (let intento = 0; intento < 20; intento++) {
    const angulo = rng() * 2 * Math.PI;
    const radio = radioMin + rng() * (radioMax - radioMin);
    const p = { x: centro.x + radio * Math.cos(angulo), y: centro.y + radio * Math.sin(angulo) };
    if (!mapa.dentroDelMapa(p) || !mapa.esTransitable(p) || mapa.terrenoEn(p) === 'agua') continue;
    if (zonas.some((z) => pointInPolygon(p, z.poligono)) || enProteccionDeCampamento(p, mercenarios)) continue;
    return p;
  }
  return undefined;
}

/**
 * Aparición de campamentos de bandidos (Doc 1.9), con nivel al azar (D21, D37). Dos fuentes de la misma entidad:
 *
 * - **Uno por asentamiento, SIEMPRE**, en SU bosque no reclamado más cercano: ni en la otra punta del mapa sin nadie cerca, ni dentro
 *   de una zona de influencia. **Cada asentamiento lleva su propio plazo** (`Asentamiento.bandidosReaparecenEn`, 2026-09-28): el suyo
 *   reaparece junto a él cuando vence, y uno sin plazo lo recibe ya.
 * - **En el anillo de cada campamento de mercenarios, según la demanda** (D42, D28): uno por cada `residentesPorBandido` residentes de
 *   Facciones sin asentamiento, entre `minimo` y `maximo`, a la misma distancia en todos; mientras falten, aparece uno cada
 *   `reaparicionMinutos` (`CampamentoMercenarios.bandidosEn`).
 */
export function avanzarSpawnBandidos(
  campamentos: CampamentoBandido[],
  zonas: ZonaInfluencia[],
  asentamientos: Asentamiento[],
  mapa: Mapa,
  instante: Instante,
  rng: RandomFn,
  mercenarios: readonly CampamentoMercenarios[] = [],
  facciones: readonly Faccion[] = []
): { campamentos: CampamentoBandido[]; mercenarios: CampamentoMercenarios[]; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const actuales = [...campamentos];
  const aparece = (nuevo: CampamentoBandido, donde: string) => {
    actuales.push(nuevo);
    eventos.push({
      codigo: 'bandidos.campamento_aparece',
      mensaje: `Aparece un campamento de bandidos de nivel ${nuevo.nivel} ${donde} (${nuevo.id}).`,
      payload: {
        campamentoId: nuevo.id,
        nivel: nuevo.nivel,
        ...(nuevo.asentamientoId ? { asentamientoObjetivoId: nuevo.asentamientoId } : {}),
        ...(nuevo.campamentoMercenariosId ? { campamentoMercenariosId: nuevo.campamentoMercenariosId } : {}),
      } satisfies PayloadCampamentoAparece,
    });
  };

  for (const asentamiento of asentamientos) {
    if (actuales.some((c) => c.asentamientoId === asentamiento.id)) continue;
    if (asentamiento.bandidosReaparecenEn !== undefined && instante < asentamiento.bandidosReaparecenEn) continue;
    const bosque = bosqueNoReclamadoMasCercano(mapa, zonas, new Set(actuales.map((c) => c.bosqueId)), asentamiento.posicion);
    if (!bosque) continue;
    // Uno por asentamiento a la vez: su id basta para que no se repita.
    aparece(
      { id: `campamento-${asentamiento.id}`, posicion: bosque.centro, bosqueId: bosque.id, asentamientoId: asentamiento.id, ...conNivel(nivelAlAzar(rng)) },
      `cerca de ${asentamiento.id}, en su bosque no reclamado más cercano`
    );
  }

  const { residentesPorBandido, minimo, maximo, reaparicionMinutos } = CAMPAMENTOS_BANDIDOS.anillo;
  const mercenariosTras = mercenarios.map((m) => {
    if (m.bandidosEn !== undefined && instante < m.bandidosEn) return m;
    const tocan = Math.min(maximo, Math.max(minimo, Math.ceil(residentesSinPlaza(m, facciones, asentamientos) / residentesPorBandido)));
    if (actuales.filter((c) => c.campamentoMercenariosId === m.id).length >= tocan) return m;
    const punto = puntoDelAnillo(m.posicion, mapa, zonas, mercenarios, rng);
    if (!punto) return m;
    aparece(
      { id: `bandidos-${m.id}-${instante}`, posicion: punto, bosqueId: '', campamentoMercenariosId: m.id, ...conNivel(nivelAlAzar(rng)) },
      `en el anillo de ${m.id}`
    );
    return { ...m, bandidosEn: sumar(instante, minutos(reaparicionMinutos)) };
  });
  return { campamentos: actuales, mercenarios: mercenariosTras, eventos };
}

/**
 * El botín de un campamento de bandidos destruido (D22, D26, D27): oro, el del nivel para cada héroe de la columna que lo destruye,
 * a su oro de botín —que solo se gasta en un campamento—. Con rendimientos decrecientes por héroe: en las últimas 24 h, completo las
 * primeras veces, luego cada vez menos, y al final solo experiencia.
 */
export function botinDeBandidos(
  heroes: readonly Heroe[],
  heroeIds: readonly string[],
  nivel: NivelBandidos,
  instante: Instante
): { heroes: Heroe[]; oro: Record<string, number> } {
  const { ventanaHoras, completas, caidaPorCada, soloExperienciaDesde } = CAMPAMENTOS_BANDIDOS.rendimientos;
  const desde = instante - ventanaHoras * 3_600_000;
  const oro: Record<string, number> = {};
  const tras = heroes.map((h) => {
    if (!heroeIds.includes(h.id)) return h;
    const recientes = (h.bandidosDestruidosEn ?? []).filter((t) => t > desde);
    const n = recientes.length + 1;
    const factor = n <= completas ? 1 : n >= soloExperienciaDesde ? 0 : Math.max(0, 1 - caidaPorCada * (n - completas));
    const ganado = Math.round(CAMPAMENTOS_BANDIDOS.niveles[nivel].oroPorHeroe * factor);
    oro[h.id] = ganado;
    return { ...h, bandidosDestruidosEn: [...recientes, instante], ...(ganado > 0 ? { oroDeBotin: (h.oroDeBotin ?? 0) + ganado } : {}) };
  });
  return { heroes: tras, oro };
}

/** Destruido un campamento, su asentamiento agenda la reaparición del suyo (Doc 1.9). */
export function agendarReaparicionBandidos(asentamientos: readonly Asentamiento[], campamento: CampamentoBandido, instante: Instante): Asentamiento[] {
  if (!campamento.asentamientoId) return asentamientos as Asentamiento[];
  const cuando = sumar(instante, minutos(CAMPAMENTOS_BANDIDOS.respawnMinutos));
  return asentamientos.map((a) => (a.id === campamento.asentamientoId ? { ...a, bandidosReaparecenEn: cuando } : a));
}

/**
 * Campamentos de bandidos atacan caravanas que pasen cerca (Doc 1.9/3.10) — mismo tipo de resolución que
 * `interceptarCaravanaConEjercito` (engine/combate.ts): poder del atacante con jitter contra la defensa base de
 * caravana, sin escolta de jugadores modelada en detalle. A diferencia de la intercepción entre Facciones,
 * el bandido no tiene almacén propio que reciba la carga capturada — si gana, la caravana se pierde por
 * completo (Doc 3.10: "se elimina si es capturada"), sin transferencia a nadie.
 */
export function avanzarAtaquesBandidos(
  campamentos: CampamentoBandido[],
  /** Con la escolta puesta (`engine/tropa.ts`): la escolta sin héroe defiende con su poder. */
  caravanas: CaravanaConEscolta[],
  rng: RandomFn,
  /** Ejércitos en campaña: una caravana que va enganchada a uno se defiende con el poder de la COLUMNA, no
   * con su defensa base (Doc 5.13.3, decisión del usuario 2026-09-04). Sin esto, escoltar no protegía de lo
   * único que hoy ataca caravanas en el mundo. */
  ejercitos: readonly EjercitoConTropa[] = [],
  /** Zonas de influencia: dentro de cualquiera, una caravana no es presa de bandidos (inmunidad, Doc 1.6). */
  zonas: readonly ZonaInfluencia[] = [],
  /** Campamentos de mercenarios: junto a uno, nadie inicia un combate (M4/D78). */
  campamentosMercenarios: readonly CampamentoMercenarios[] = []
): { caravanas: CaravanaConEscolta[]; eventos: EventoCrudo[]; escoltasPerdidas: Escuadron[] } {
  if (campamentos.length === 0) return { caravanas, eventos: [], escoltasPerdidas: [] };
  const eventos: EventoCrudo[] = [];
  const escoltasPerdidas: Escuadron[] = [];

  const escoltaEjercitoDe = (caravanaId: string): EjercitoConTropa | undefined =>
    ejercitos.find((e) => e.caravanasAdjuntasIds.includes(caravanaId));

  const resultado: CaravanaConEscolta[] = [];
  for (const caravana of caravanas) {
    // Parada en una plaza (disponible en su origen; preparándose para un envío manual, Doc 3.13.3; o
    // 'aparcada' en una plaza anfitriona tras guarnecer, Ocupacion §2.3d) — nada que interceptar.
    if (caravana.estado === 'disponible' || caravana.estado === 'preparando' || caravana.estado === 'aparcada') {
      resultado.push(caravana);
      continue;
    }
    const campamentoCercano = campamentos.find(
      (c) => distancia(c.posicion, caravana.posicionActual) <= CAMPAMENTOS_BANDIDOS.radioAtaqueCaravana
    );
    if (!campamentoCercano || enRefugio(caravana.posicionActual, zonas, []) || enProteccionDeCampamento(caravana.posicionActual, campamentosMercenarios)) {
      resultado.push(caravana);
      continue;
    }

    const conEscoltaSinHeroe = (caravana.escolta?.length ?? 0) > 0;

    // Contra qué defensa tira el bandido, de más a menos protegida:
    //  - escoltada por un EJÉRCITO (Doc 5.13.3): el bandido choca con la columna, `poderTotal` de sus escuadrones.
    //  - escolta SIN HÉROE (Doc 3.13.4): `poderTotal` de los escuadrones cedidos a la caravana.
    //  - sin nada: la defensa base fija (Doc 3.10) — frena a un jugador solo y nada más.
    const escoltaEjercito = escoltaEjercitoDe(caravana.id);
    const defensa = escoltaEjercito
      ? poderTotal(escoltaEjercito.escuadrones, true)
      : conEscoltaSinHeroe
        ? poderTotal(caravana.escolta!, true)
        : MILITAR.defensaBaseCaravana;

    const jitter = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
    const gana = campamentoCercano.poder * jitter > defensa;

    // La escolta sin héroe sufre bajas en los dos casos (ligeras si aguanta, fuertes si cae) — lo que se pierde
    // son la carga y los carros, no la tropa (Doc 3.13.6).
    const escoltaTrasCombate = conEscoltaSinHeroe ? aplicarBajas(caravana.escolta!, gana ? 0.25 : 0.05, !gana) : undefined;

    if (gana) {
      eventos.push({
        codigo: 'bandidos.caravana_interceptada',
        mensaje: `Un campamento de bandidos (${campamentoCercano.id}) intercepta y destruye la caravana ${caravana.id}.`,
        payload: { campamentoId: campamentoCercano.id, caravanaId: caravana.id } satisfies PayloadCaravanaInterceptada,
      });
      // La caravana se elimina (Doc 3.10), y su escolta sin héroe queda a 0 y vuelve al campamento de su héroe
      // (Doc 5.15.4): la escuadra no desaparece, conserva su nivel y su experiencia.
      if (escoltaTrasCombate) escoltasPerdidas.push(...alCampamento(escoltaTrasCombate.map((e) => ({ ...e, cantidad: 0 }))));
      continue;
    }

    eventos.push({
      codigo: 'bandidos.caravana_escapa',
      mensaje: `La caravana ${caravana.id} escapa de un campamento de bandidos cercano.`,
      payload: { caravanaId: caravana.id } satisfies PayloadCaravanaEscapa,
    });
    resultado.push(escoltaTrasCombate ? { ...caravana, escolta: escoltaTrasCombate } : caravana);
  }

  return { caravanas: resultado, eventos, escoltasPerdidas };
}
