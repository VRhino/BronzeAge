// Campamentos de bandidos (Doc 1.9, a petición del usuario — inspirado en análisis comparativo con Travian):
// amenaza NPC ambiental, distinta del combate entre Facciones (Doc 5, engine/combate.ts). Este módulo cubre
// las dos partes AUTOMÁTICAS de la mecánica (spawn/respawn y ataque a caravanas cercanas, ambas evaluadas
// cada tick dentro de `avanzarSimulacion`); el ataque contra un campamento, con una columna que llegue a él, vive en
// `engine/combate.ts` (`atacarCampamentoConColumna`), junto al resto de resolución de combate.

import type { Asentamiento, CampamentoBandido, CampamentoMercenarios, Escuadron, Point, ZonaBosque, ZonaInfluencia } from '../domain/types';
import { enProteccionDeCampamento } from './mercenarios';
import { alCampamento, type CaravanaConEscolta, type EjercitoConTropa } from './tropa';
import type { EventoCrudo } from '../domain/eventos';

/** Fase A5 — payloads de los eventos de este subsistema (ver `avanzarSpawnBandidos`/`avanzarAtaquesBandidos`). */
export interface PayloadCampamentoAparece {
  campamentoId: string;
  asentamientoObjetivoId: string;
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

/**
 * Spawn/respawn de campamentos de bandidos (Doc 1.9) — UNO por asentamiento, en SU bosque no reclamado más cercano:
 * ni en la otra punta del mapa sin nadie cerca para atacarlo, ni dentro de una zona de influencia (ya excluido por
 * `bosqueNoReclamadoMasCercano`).
 *
 * **Cada asentamiento lleva su propio plazo** (`Asentamiento.bandidosReaparecenEn`, 2026-09-28, decisión del usuario):
 * el suyo reaparece junto a él cuando vence, y uno sin plazo lo recibe ya. Antes el plazo era uno para todo el mundo
 * y el campamento nuevo iba al PRIMER asentamiento de la lista sin cubrir: en la Era I medida era siempre la capital
 * de la Facción 1, que lo destruía al minuto y se quedaba con la experiencia de todos los bandidos del mapa.
 *
 * Si no queda un bosque libre para alguno, ese no recibe nada y se reintenta el tick siguiente.
 */
export function avanzarSpawnBandidos(
  campamentos: CampamentoBandido[],
  zonas: ZonaInfluencia[],
  asentamientos: Asentamiento[],
  mapa: Mapa,
  instante: Instante
): { campamentos: CampamentoBandido[]; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const actuales = [...campamentos];
  for (const asentamiento of asentamientos) {
    if (actuales.some((c) => c.asentamientoId === asentamiento.id)) continue;
    if (asentamiento.bandidosReaparecenEn !== undefined && instante < asentamiento.bandidosReaparecenEn) continue;
    const bosque = bosqueNoReclamadoMasCercano(mapa, zonas, new Set(actuales.map((c) => c.bosqueId)), asentamiento.posicion);
    if (!bosque) continue;
    // Uno por asentamiento a la vez: su id basta para que no se repita.
    const nuevo: CampamentoBandido = {
      id: `campamento-${asentamiento.id}`,
      posicion: bosque.centro,
      bosqueId: bosque.id,
      asentamientoId: asentamiento.id,
      poder: CAMPAMENTOS_BANDIDOS.poder,
    };
    actuales.push(nuevo);
    eventos.push({
      codigo: 'bandidos.campamento_aparece',
      mensaje: `Aparece un campamento de bandidos cerca de ${asentamiento.id}, en su bosque no reclamado más cercano (${nuevo.id}).`,
      payload: { campamentoId: nuevo.id, asentamientoObjetivoId: asentamiento.id } satisfies PayloadCampamentoAparece,
    });
  }
  return { campamentos: actuales, eventos };
}

/** Destruido un campamento, su asentamiento agenda la reaparición del suyo (Doc 1.9). */
export function agendarReaparicionBandidos(asentamientos: readonly Asentamiento[], campamento: CampamentoBandido, instante: Instante): Asentamiento[] {
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
