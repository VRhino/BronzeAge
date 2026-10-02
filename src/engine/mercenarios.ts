// Campamentos de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40): enclaves neutrales del mundo abierto. Este
// módulo cubre su APARICIÓN; lo que se hace dentro (residir, reclutar, comprar) llega en los pasos siguientes.
//
// Un campamento no es un asentamiento: no tiene Facción, ni zona de influencia (ninguna lo absorbe, porque las zonas
// salen de los asentamientos), ni crece, y no desaparece. Por eso no vive en `asentamientos` sino en su propia lista.
import type { Asentamiento, CampamentoMercenarios, EdificioCampamentoTipo, Point, ZonaInfluencia } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { MERCENARIOS } from '../constants';
import type { Instante } from '../domain/tiempo';
import type { Mapa } from '../world/mapa';
import { pointInPolygon } from './zones';

export interface PayloadCampamentoMercenariosAparece {
  campamentoId: string;
  posicion: Point;
  origen: number;
}

const distancia = (a: Point, b: Point): number => Math.hypot(a.x - b.x, a.y - b.y);

/** Terreno donde cabe un campamento: dentro del mapa, que se pueda pisar y que no sea agua ni cima. */
function terrenoValido(mapa: Mapa, p: Point): boolean {
  if (!mapa.dentroDelMapa(p) || !mapa.esTransitable(p)) return false;
  const terreno = mapa.terrenoEn(p);
  return terreno !== 'agua' && terreno !== 'cima';
}

/** El punto válido más cercano a `ancla`, buscando en anillos crecientes; `undefined` si no hay ninguno al alcance. */
function validoCercaDe(mapa: Mapa, ancla: Point, esValido: (p: Point) => boolean): Point | undefined {
  for (let r = 0; r <= MERCENARIOS.radioBusqueda; r += MERCENARIOS.pasoBusqueda) {
    const lados = r === 0 ? 1 : 8;
    for (let i = 0; i < lados; i++) {
      const ang = (2 * Math.PI * i) / lados;
      const p = { x: ancla.x + r * Math.cos(ang), y: ancla.y + r * Math.sin(ang) };
      if (terrenoValido(mapa, p) && esValido(p)) return p;
    }
  }
  return undefined;
}

/** Si hay un bosque a menos de `margenBosque` del punto, el punto pegado a su borde (preferencia, no requisito). */
function pegadoABosque(mapa: Mapa, p: Point): Point {
  let mejor: { d: number; punto: Point } | undefined;
  for (const bosque of mapa.listarBosques()) {
    const hastaCentro = distancia(p, bosque.centro);
    const alBorde = hastaCentro - bosque.radio;
    if (alBorde > MERCENARIOS.margenBosque || (mejor && alBorde >= mejor.d)) continue;
    // Junto al borde, del lado del que viene el punto. Un punto ya dentro del bosque se queda donde está.
    const dir = hastaCentro > 0 ? { x: (p.x - bosque.centro.x) / hastaCentro, y: (p.y - bosque.centro.y) / hastaCentro } : { x: 1, y: 0 };
    const punto = alBorde <= 0 ? p : { x: bosque.centro.x + dir.x * (bosque.radio + MERCENARIOS.pegadoAlBorde), y: bosque.centro.y + dir.y * (bosque.radio + MERCENARIOS.pegadoAlBorde) };
    mejor = { d: alBorde, punto };
  }
  return mejor && terrenoValido(mapa, mejor.punto) ? mejor.punto : p;
}

/**
 * El aspecto y el edificio militar salen de la posición, no del azar de la partida: así dos corridas con la misma
 * semilla ponen lo mismo, y el campamento no consume del generador compartido (que desplazaría todo lo demás).
 */
function nuevoCampamento(indice: number, posicion: Point, instante: Instante): CampamentoMercenarios {
  const huella = Math.abs(Math.floor(posicion.x) * 31 + Math.floor(posicion.y) * 17 + indice);
  const militar = MERCENARIOS.edificiosMilitares[huella % MERCENARIOS.edificiosMilitares.length]!;
  const edificios: EdificioCampamentoTipo[] = [...MERCENARIOS.edificiosFijos, militar];
  return {
    id: `mercenarios-${indice}`,
    posicion,
    origen: Math.floor(huella / MERCENARIOS.edificiosMilitares.length) % MERCENARIOS.origenes,
    edificios,
    creadoEn: instante,
  };
}

/** El del día 1 (Doc 1.9b): en el centro del mapa, en tierra firme y pegado a un bosque si lo hay cerca. */
function campamentoInicial(mapa: Mapa, instante: Instante): CampamentoMercenarios | undefined {
  const { ancho, alto } = mapa.limites;
  const centro = { x: ancho / 2, y: alto / 2 };
  const ancla = pegadoABosque(mapa, centro);
  const posicion = validoCercaDe(mapa, ancla, () => true);
  return posicion ? nuevoCampamento(0, posicion, instante) : undefined;
}

/**
 * Un punto sin reclamar donde aparece uno nuevo (Doc 1.9b): fuera de toda zona de influencia, con zonas de al menos
 * `MERCENARIOS.facciones` Facciones distintas a menos de `MERCENARIOS.radioZonas`, y a `distanciaMinima` de los demás.
 *
 * Los candidatos son los puntos medios entre plazas de Facciones distintas —es donde se juntan las fronteras— y gana el que
 * tiene más Facciones cerca, a igualdad el más cercano al centro del mapa. Determinista: sin azar, para que dos corridas
 * con la misma semilla pongan los campamentos en los mismos sitios.
 */
function puntoDeAparicion(
  campamentos: readonly CampamentoMercenarios[],
  asentamientos: readonly Asentamiento[],
  zonas: readonly ZonaInfluencia[],
  mapa: Mapa
): Point | undefined {
  const { ancho, alto } = mapa.limites;
  const centro = { x: ancho / 2, y: alto / 2 };
  const facciones = (p: Point): number =>
    new Set(asentamientos.filter((a) => distancia(p, a.posicion) - a.radioPotencial <= MERCENARIOS.radioZonas).map((a) => a.faccionId)).size;
  const libre = (p: Point): boolean =>
    terrenoValido(mapa, p) &&
    !zonas.some((z) => pointInPolygon(p, z.poligono)) &&
    campamentos.every((c) => distancia(p, c.posicion) >= MERCENARIOS.distanciaMinima);

  let mejor: { p: Point; n: number; d: number } | undefined;
  for (let i = 0; i < asentamientos.length; i++) {
    for (let j = i + 1; j < asentamientos.length; j++) {
      const a = asentamientos[i]!;
      const b = asentamientos[j]!;
      if (a.faccionId === b.faccionId) continue;
      const medio = pegadoABosque(mapa, { x: (a.posicion.x + b.posicion.x) / 2, y: (a.posicion.y + b.posicion.y) / 2 });
      if (!libre(medio)) continue;
      const n = facciones(medio);
      if (n < MERCENARIOS.facciones) continue;
      const d = distancia(medio, centro);
      if (!mejor || n > mejor.n || (n === mejor.n && d < mejor.d)) mejor = { p: medio, n, d };
    }
  }
  return mejor?.p;
}

/**
 * Aparición de campamentos de mercenarios (Doc 1.9b). Siempre hay al menos uno desde el día 1; los demás aparecen, de uno
 * en uno, donde se juntan las zonas de varias Facciones y hasta el tope del servidor. La búsqueda es cara para hacerla en
 * cada tick, así que se mira cada `MERCENARIOS.cadaMinutos`.
 */
export function avanzarAparicionMercenarios(
  campamentos: CampamentoMercenarios[],
  asentamientos: Asentamiento[],
  zonas: ZonaInfluencia[],
  mapa: Mapa,
  instante: Instante
): { campamentos: CampamentoMercenarios[]; eventos: EventoCrudo[] } {
  let nuevo: CampamentoMercenarios | undefined;
  if (campamentos.length === 0) {
    nuevo = campamentoInicial(mapa, instante);
  } else if (campamentos.length < MERCENARIOS.topePorServidor && Math.floor(instante / 60_000) % MERCENARIOS.cadaMinutos === 0) {
    const posicion = puntoDeAparicion(campamentos, asentamientos, zonas, mapa);
    if (posicion) nuevo = nuevoCampamento(campamentos.length, posicion, instante);
  }
  if (!nuevo) return { campamentos, eventos: [] };

  return {
    campamentos: [...campamentos, nuevo],
    eventos: [
      {
        codigo: 'mercenarios.campamento_aparece',
        mensaje: `Aparece un campamento de mercenarios (${nuevo.id}).`,
        payload: { campamentoId: nuevo.id, posicion: nuevo.posicion, origen: nuevo.origen } satisfies PayloadCampamentoMercenariosAparece,
      },
    ],
  };
}
