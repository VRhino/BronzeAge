// Alijos de exploración (D29, D60-D63): escondites de oro alrededor de cada campamento de mercenarios, colocados al crear el mundo.
// Son POR HÉROE: cada uno abre cada alijo una sola vez y no reaparecen, así que nadie le quita uno a otro y quien llega tarde
// encuentra lo mismo que el primero. Solo los ven y los abren los héroes de Facciones sin asentamiento, y su oro va al oro de botín.
import type { Alijo, Asentamiento, CampamentoMercenarios, Ejercito, Faccion, Heroe, Point } from '../domain/types';
import { ALIJOS, MOVIMIENTO, VISION } from '../constants';
import type { Mapa } from '../world/mapa';
import { createRng } from '../worldgen';
import { enProteccionDeCampamento } from './mercenarios';
import { esCiudadano } from './faccion';
import { ReglaInvalidaError } from './errores';

export class AlijoInvalidoError extends ReglaInvalidaError {}

const distancia = (a: Point, b: Point): number => Math.hypot(a.x - b.x, a.y - b.y);

/**
 * Los alijos del mundo (D62): por cada campamento, los de cada banda de distancia, en tierra firme, fuera de la protección de
 * cualquier campamento y con ESE campamento como el más cercano (los de la banda lejana caen en los huecos entre campamentos). Con
 * semilla derivada de la del mapa (D35): misma seed, mismos alijos. Si una banda no encuentra sitio, ese alijo no se pone.
 */
export function colocarAlijos(mapa: Mapa, campamentos: readonly CampamentoMercenarios[]): Alijo[] {
  const rng = createRng(mapa.seed ^ ALIJOS.salSemilla);
  const alijos: Alijo[] = [];
  for (const c of campamentos) {
    for (const banda of ALIJOS.bandas) {
      for (let n = 0; n < banda.cuantos; n++) {
        for (let intento = 0; intento < 40; intento++) {
          const angulo = rng() * 2 * Math.PI;
          const radio = banda.desde + rng() * (banda.hasta - banda.desde);
          const p = { x: c.posicion.x + radio * Math.cos(angulo), y: c.posicion.y + radio * Math.sin(angulo) };
          if (!mapa.dentroDelMapa(p) || !mapa.esTransitable(p) || mapa.terrenoEn(p) === 'agua') continue;
          if (enProteccionDeCampamento(p, campamentos)) continue;
          if (campamentos.some((otro) => otro.id !== c.id && distancia(p, otro.posicion) < distancia(p, c.posicion))) continue;
          alijos.push({ id: `alijo-${alijos.length}`, posicion: p, oro: banda.oro });
          break;
        }
      }
    }
  }
  return alijos;
}

/** ¿Puede este héroe buscar alijos? Solo el de una Facción sin asentamiento, o sin Facción (D63). */
export function buscaAlijos(heroeId: string, facciones: readonly Faccion[], asentamientos: readonly Asentamiento[]): boolean {
  const faccion = facciones.find((f) => esCiudadano(f, heroeId));
  return !faccion || !asentamientos.some((a) => a.faccionId === faccion.id);
}

/** Los alijos que el héroe ve ahora (D62): a la vista de su columna (`VISION.jugadorSolo`) y que él no ha abierto. */
export function alijosALaVista(alijos: readonly Alijo[], heroe: Heroe, columna: Ejercito | undefined): Alijo[] {
  if (!columna) return [];
  const abiertos = new Set(heroe.alijosAbiertos ?? []);
  return alijos.filter((a) => !abiertos.has(a.id) && distancia(a.posicion, columna.posicionActual) <= VISION.jugadorSolo);
}

/** Abrir un alijo estando en el sitio (D62): su oro, al oro de botín del héroe, y queda abierto para él. */
export function abrirAlijo(alijo: Alijo, heroe: Heroe, columna: Ejercito | undefined, puedeBuscar: boolean): Heroe {
  if (!puedeBuscar) throw new AlijoInvalidoError('Los alijos solo los abre quien es de una Facción sin asentamiento.');
  if (heroe.alijosAbiertos?.includes(alijo.id)) throw new AlijoInvalidoError('Ese alijo ya lo abriste.');
  if (!columna || distancia(columna.posicionActual, alijo.posicion) > MOVIMIENTO.radioPuerta) throw new AlijoInvalidoError('Hay que estar en el sitio.');
  return { ...heroe, oroDeBotin: (heroe.oroDeBotin ?? 0) + alijo.oro, alijosAbiertos: [...(heroe.alijosAbiertos ?? []), alijo.id] };
}
