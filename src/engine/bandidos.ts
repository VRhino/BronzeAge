// Campamentos de bandidos (Doc 1.9, a petición del usuario — inspirado en análisis comparativo con Travian):
// amenaza NPC ambiental, distinta del combate entre Facciones (Doc 5, engine/combate.ts). Este módulo cubre
// las dos partes AUTOMÁTICAS de la mecánica (spawn/respawn y ataque a caravanas cercanas, ambas evaluadas
// cada tick dentro de `avanzarSimulacion`); el ataque MANUAL de un jugador contra un campamento vive en
// `engine/combate.ts` (`atacarCampamentoBandidos`), junto al resto de resolución de combate.

import type { Asentamiento, Caravana, CampamentoBandido, Point, ZonaBosque, ZonaInfluencia } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { CAMPAMENTOS_BANDIDOS, MILITAR } from '../constants';
import { pointInPolygon } from './zones';

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/**
 * Asentamiento vivo sin ningún campamento ya asignado cerca (Doc 1.9, a petición del usuario) — "cerca" se
 * mide con el mismo `distanciaCobertura` que decide cuándo un asentamiento ya está atendido, así un
 * asentamiento no acumula varios campamentos mientras otros se quedan sin ninguno. Devuelve el PRIMERO sin
 * cubrir (orden estable) — cuál exactamente no importa, el respawn ya reparte uno por tick.
 */
function asentamientoSinCampamentoCercano(asentamientos: Asentamiento[], campamentos: CampamentoBandido[]): Asentamiento | undefined {
  return asentamientos.find(
    (a) => !campamentos.some((c) => distancia(c.posicion, a.posicion) <= CAMPAMENTOS_BANDIDOS.distanciaCobertura)
  );
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
 * de ser un número fijo — es el número de asentamientos vivos, cada uno con como mucho un campamento dentro
 * de su radio de cobertura. Mientras se haya cumplido el plazo de reaparición, cada tick se cubre COMO MUCHO
 * un asentamiento sin campamento cercano (mismo ritmo que antes) — si no queda ningún bosque libre para él
 * ese tick, simplemente no aparece nada y se reintenta el siguiente (sin bloquear la simulación ni lanzar error).
 */
export function avanzarSpawnBandidos(
  campamentos: CampamentoBandido[],
  proximoSpawnEnTick: number,
  zonas: ZonaInfluencia[],
  asentamientos: Asentamiento[],
  mapa: Mapa,
  tickActual: number,
  contador = 0
): { campamentos: CampamentoBandido[]; eventos: string[] } {
  if (campamentos.length >= asentamientos.length || tickActual < proximoSpawnEnTick) {
    return { campamentos, eventos: [] };
  }
  const asentamientoObjetivo = asentamientoSinCampamentoCercano(asentamientos, campamentos);
  if (!asentamientoObjetivo) return { campamentos, eventos: [] };

  const ocupados = new Set(campamentos.map((c) => c.bosqueId));
  const bosque = bosqueNoReclamadoMasCercano(mapa, zonas, ocupados, asentamientoObjetivo.posicion);
  if (!bosque) return { campamentos, eventos: [] };

  const nuevo: CampamentoBandido = {
    id: `campamento-${tickActual}-${contador}`,
    posicion: bosque.centro,
    bosqueId: bosque.id,
    poder: CAMPAMENTOS_BANDIDOS.poder,
  };
  return {
    campamentos: [...campamentos, nuevo],
    eventos: [`Aparece un campamento de bandidos cerca de ${asentamientoObjetivo.id}, en su bosque no reclamado más cercano (${nuevo.id}).`],
  };
}

/**
 * Campamentos de bandidos atacan caravanas que pasen cerca (Doc 1.9/3.10) — mismo tipo de resolución que
 * `interceptarCaravana` (engine/combate.ts): poder fijo del atacante con jitter contra la defensa base de
 * caravana, sin escolta de jugadores modelada en detalle. A diferencia de la intercepción entre Facciones,
 * el bandido no tiene almacén propio que reciba la carga capturada — si gana, la caravana se pierde por
 * completo (Doc 3.10: "se elimina si es capturada"), sin transferencia a nadie.
 */
export function avanzarAtaquesBandidos(campamentos: CampamentoBandido[], caravanas: Caravana[]): { caravanas: Caravana[]; eventos: string[] } {
  if (campamentos.length === 0) return { caravanas, eventos: [] };
  const eventos: string[] = [];
  const perdidas = new Set<string>();

  for (const caravana of caravanas) {
    if (caravana.estado === 'disponible') continue; // parada en origen, no viajando — nada que interceptar.
    const campamentoCercano = campamentos.find(
      (c) => distancia(c.posicion, caravana.posicionActual) <= CAMPAMENTOS_BANDIDOS.radioAtaqueCaravana
    );
    if (!campamentoCercano) continue;

    const jitter = 1 + (Math.random() * 2 - 1) * MILITAR.varianzaCombate;
    const gana = campamentoCercano.poder * jitter > MILITAR.defensaBaseCaravana;
    if (gana) {
      perdidas.add(caravana.id);
      eventos.push(`Un campamento de bandidos (${campamentoCercano.id}) intercepta y destruye la caravana ${caravana.id}.`);
    } else {
      eventos.push(`La caravana ${caravana.id} escapa de un campamento de bandidos cercano.`);
    }
  }

  return { caravanas: perdidas.size === 0 ? caravanas : caravanas.filter((c) => !perdidas.has(c.id)), eventos };
}
