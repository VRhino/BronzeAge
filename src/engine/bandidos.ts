// Campamentos de bandidos (Doc 1.9, a petición del usuario — inspirado en análisis comparativo con Travian):
// amenaza NPC ambiental, distinta del combate entre Facciones (Doc 5, engine/combate.ts). Este módulo cubre
// las dos partes AUTOMÁTICAS de la mecánica (spawn/respawn y ataque a caravanas cercanas, ambas evaluadas
// cada tick dentro de `avanzarSimulacion`); el ataque MANUAL de un jugador contra un campamento vive en
// `engine/combate.ts` (`atacarCampamentoBandidos`), junto al resto de resolución de combate.

import type { Asentamiento, Caravana, CampamentoBandido, Ejercito, Point, ZonaBosque, ZonaInfluencia } from '../domain/types';
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
import type { Instante } from '../domain/tiempo';
import { pointInPolygon } from './zones';
import { poderTotal } from './combate';

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/**
 * Asentamiento vivo sin campamento ASIGNADO todavía (Doc 1.9, a petición del usuario) — por `asentamientoId`,
 * no por distancia: un criterio anterior basado en radio dejaba asentamientos vecinos (a menos de la
 * distancia de cobertura de otro) sin campamento propio para siempre, porque el primero que aparecía
 * "cubría" a los demás sin que ninguno tuviera el suyo (bug real detectado con 3 asentamientos cercanos,
 * donde solo llegaba a aparecer 1). Devuelve el PRIMERO sin cubrir (orden estable) — cuál exactamente no
 * importa, el respawn ya reparte uno por tick.
 */
function asentamientoSinCampamento(asentamientos: Asentamiento[], campamentos: CampamentoBandido[]): Asentamiento | undefined {
  return asentamientos.find((a) => !campamentos.some((c) => c.asentamientoId === a.id));
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
 * Spawn/respawn de campamentos de bandidos (Doc 1.9, a petición del usuario) — UNO por asentamiento, tomando
 * como referencia SU bosque no reclamado más cercano: ni en la otra punta del mapa sin nadie cerca para
 * atacarlo, ni dentro de una zona de influencia (ya excluido por `bosqueNoReclamadoMasCercano`). El tope deja
 * de ser un número fijo — es el número de asentamientos vivos, cada uno con SU PROPIO campamento asignado
 * (`asentamientoId`), sin importar lo cerca que esté de otro asentamiento ya atendido (bug corregido: un
 * criterio anterior por radio dejaba asentamientos vecinos sin campamento propio para siempre). Mientras se
 * haya cumplido el plazo de reaparición, cada tick se cubre COMO MUCHO un asentamiento sin campamento (mismo
 * ritmo que antes) — si no queda ningún bosque libre para él ese tick, simplemente no aparece nada y se
 * reintenta el siguiente (sin bloquear la simulación ni lanzar error).
 */
export function avanzarSpawnBandidos(
  campamentos: CampamentoBandido[],
  proximoSpawnEn: Instante,
  zonas: ZonaInfluencia[],
  asentamientos: Asentamiento[],
  mapa: Mapa,
  instante: Instante,
  contador = 0
): { campamentos: CampamentoBandido[]; eventos: EventoCrudo[] } {
  if (campamentos.length >= asentamientos.length || instante < proximoSpawnEn) {
    return { campamentos, eventos: [] };
  }
  const asentamientoObjetivo = asentamientoSinCampamento(asentamientos, campamentos);
  if (!asentamientoObjetivo) return { campamentos, eventos: [] };

  const ocupados = new Set(campamentos.map((c) => c.bosqueId));
  const bosque = bosqueNoReclamadoMasCercano(mapa, zonas, ocupados, asentamientoObjetivo.posicion);
  if (!bosque) return { campamentos, eventos: [] };

  const nuevo: CampamentoBandido = {
    id: `campamento-${contador}`,
    posicion: bosque.centro,
    bosqueId: bosque.id,
    asentamientoId: asentamientoObjetivo.id,
    poder: CAMPAMENTOS_BANDIDOS.poder,
  };
  return {
    campamentos: [...campamentos, nuevo],
    eventos: [
      {
        codigo: 'bandidos.campamento_aparece',
        mensaje: `Aparece un campamento de bandidos cerca de ${asentamientoObjetivo.id}, en su bosque no reclamado más cercano (${nuevo.id}).`,
        payload: { campamentoId: nuevo.id, asentamientoObjetivoId: asentamientoObjetivo.id } satisfies PayloadCampamentoAparece,
      },
    ],
  };
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
  caravanas: Caravana[],
  rng: RandomFn,
  /** Ejércitos en campaña: una caravana que va enganchada a uno se defiende con el poder de la COLUMNA, no
   * con su defensa base (Doc 5.13.3, decisión del usuario 2026-09-04). Sin esto, escoltar no protegía de lo
   * único que hoy ataca caravanas en el mundo. */
  ejercitos: readonly Ejercito[] = [],
  instante?: Instante
): { caravanas: Caravana[]; eventos: EventoCrudo[] } {
  if (campamentos.length === 0) return { caravanas, eventos: [] };
  const eventos: EventoCrudo[] = [];
  const perdidas = new Set<string>();

  const escoltaDe = (caravanaId: string): Ejercito | undefined =>
    ejercitos.find((e) => e.caravanasAdjuntasIds.includes(caravanaId));

  for (const caravana of caravanas) {
    // Parada en su ciudad (disponible, o preparándose para un envío manual, Doc 3.13.3) — nada que interceptar.
    if (caravana.estado === 'disponible' || caravana.estado === 'preparando') continue;
    const campamentoCercano = campamentos.find(
      (c) => distancia(c.posicion, caravana.posicionActual) <= CAMPAMENTOS_BANDIDOS.radioAtaqueCaravana
    );
    if (!campamentoCercano) continue;

    // Escoltada: el bandido se encuentra con el ejército, no con la caravana. Esa es toda la mecánica de la
    // escolta (Doc 5.13.3) — "viaja protegida por el poder de combate del ejército en vez de por su defensa
    // base fija" —, y por eso el número contra el que tira es `poderTotal` de la columna.
    const escolta = escoltaDe(caravana.id);
    const defensa =
      escolta && instante !== undefined ? poderTotal(escolta.escuadrones, instante, true) : MILITAR.defensaBaseCaravana;

    const jitter = 1 + (rng() * 2 - 1) * MILITAR.varianzaCombate;
    const gana = campamentoCercano.poder * jitter > defensa;
    if (gana) {
      perdidas.add(caravana.id);
      eventos.push({
        codigo: 'bandidos.caravana_interceptada',
        mensaje: `Un campamento de bandidos (${campamentoCercano.id}) intercepta y destruye la caravana ${caravana.id}.`,
        payload: { campamentoId: campamentoCercano.id, caravanaId: caravana.id } satisfies PayloadCaravanaInterceptada,
      });
    } else {
      eventos.push({
        codigo: 'bandidos.caravana_escapa',
        mensaje: `La caravana ${caravana.id} escapa de un campamento de bandidos cercano.`,
        payload: { caravanaId: caravana.id } satisfies PayloadCaravanaEscapa,
      });
    }
  }

  return { caravanas: perdidas.size === 0 ? caravanas : caravanas.filter((c) => !perdidas.has(c.id)), eventos };
}
