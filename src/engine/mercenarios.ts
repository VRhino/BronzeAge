// Campamentos de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40): enclaves neutrales del mundo abierto. Este
// módulo cubre su APARICIÓN; lo que se hace dentro (residir, reclutar, comprar) llega en los pasos siguientes.
//
// Un campamento no es un asentamiento: no tiene Facción, ni zona de influencia (ninguna lo absorbe, porque las zonas
// salen de los asentamientos), ni crece, y no desaparece. Por eso no vive en `asentamientos` sino en su propia lista.
import type { Asentamiento, CampamentoMercenarios, EdificioCampamentoTipo, Ejercito, Faccion, Heroe, Point, ZonaInfluencia } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { MERCENARIOS } from '../constants';
import type { Instante } from '../domain/tiempo';
import type { Mapa } from '../world/mapa';
import { pointInPolygon } from './zones';
import { dejarResidencia, esCiudadano } from './faccion';
import { esResidente } from './pertenencia';
import { ReglaInvalidaError } from './errores';

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
    residentesIds: [],
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

export class MercenariosInvalidoError extends ReglaInvalidaError {}

/** El campamento donde reside un héroe, si reside en alguno. */
export const campamentoDeResidente = (campamentos: readonly CampamentoMercenarios[], heroeId: string): CampamentoMercenarios | undefined =>
  campamentos.find((c) => c.residentesIds.includes(heroeId));

/** El campamento más cercano a un punto (a igual distancia, el de id menor: determinista). */
export function campamentoMasCercano(campamentos: readonly CampamentoMercenarios[], desde: Point): CampamentoMercenarios | undefined {
  return [...campamentos].sort((a, b) => distancia(a.posicion, desde) - distancia(b.posicion, desde) || (a.id < b.id ? -1 : 1))[0];
}

/** Los campamentos sin ese héroe como residente. */
function sinResidente(campamentos: readonly CampamentoMercenarios[], heroeId: string): CampamentoMercenarios[] {
  return campamentos.map((c) => (c.residentesIds.includes(heroeId) ? { ...c, residentesIds: c.residentesIds.filter((id) => id !== heroeId) } : c));
}

/**
 * Pasar a residir en un campamento de mercenarios (Doc 2.5): cualquier héroe, de cualquier Facción y aunque la suya tenga
 * asentamientos. Deja la residencia que tuviera —casa en un asentamiento, con sus cargos locales, u otro campamento—: se reside
 * en un solo sitio. No cuesta nada y no exige estar allí, como el resto de cambios de residencia.
 */
export function residirEnCampamento(
  campamentos: readonly CampamentoMercenarios[],
  asentamientos: readonly Asentamiento[],
  heroeId: string,
  campamentoId: string
): { campamentos: CampamentoMercenarios[]; asentamientos: Asentamiento[] } {
  const destino = campamentos.find((c) => c.id === campamentoId);
  if (!destino) throw new MercenariosInvalidoError('Ese campamento de mercenarios no existe.');
  if (destino.residentesIds.includes(heroeId)) throw new MercenariosInvalidoError('El héroe ya reside en ese campamento.');

  const casa = asentamientos.some((a) => esResidente(a, heroeId)) ? dejarResidencia(asentamientos, heroeId) : undefined;
  return {
    campamentos: sinResidente(campamentos, heroeId).map((c) => (c.id === destino.id ? { ...c, residentesIds: [...c.residentesIds, heroeId] } : c)),
    asentamientos: casa ? asentamientos.map((a) => (a.id === casa.id ? casa : a)) : [...asentamientos],
  };
}

/** Deja el campamento donde residiera (al comprar casa o mudarse a un asentamiento). */
export const salirDeCampamentos = sinResidente;

/** Dónde está un héroe en el mapa, si se sabe: su columna o la plaza donde está. */
function posicionDe(heroe: Heroe, asentamientos: readonly Asentamiento[], ejercitos: readonly Ejercito[]): Point | undefined {
  const u = heroe.ubicacion;
  if (u.tipo === 'columna') return ejercitos.find((e) => e.id === u.ejercitoId)?.posicionActual;
  if (u.tipo === 'asentamiento') return asentamientos.find((a) => a.id === u.asentamientoId)?.posicion;
  return undefined;
}

/**
 * Se acaba el huérfano (Doc 0, 5.15.5, decidido el 2026-10-02): el ciudadano de una Facción que no reside en ningún asentamiento
 * ni campamento pasa a residir en el campamento de mercenarios más cercano a donde está. Es lo que le pasa a quien pierde su
 * última plaza —conquista o ruina— y a quien deja su casa. Se mira cada tick, así que cubre todas las vías.
 *
 * No toca a quien aún no tiene Facción (un recién llegado), ni a los héroes bot: los de una Facción NPC sin plazas desaparecerán
 * (pendiente), no se mudan.
 */
export function acogerHeroesSinCasa(
  campamentos: CampamentoMercenarios[],
  asentamientos: readonly Asentamiento[],
  heroes: readonly Heroe[],
  facciones: readonly Faccion[],
  ejercitos: readonly Ejercito[]
): CampamentoMercenarios[] {
  if (campamentos.length === 0) return campamentos;
  let actuales = campamentos;
  for (const heroe of heroes) {
    if (heroe.controlador === 'bot') continue;
    if (!facciones.some((f) => esCiudadano(f, heroe.id))) continue;
    if (asentamientos.some((a) => esResidente(a, heroe.id)) || campamentoDeResidente(actuales, heroe.id)) continue;
    const posicion = posicionDe(heroe, asentamientos, ejercitos);
    const destino = posicion ? campamentoMasCercano(actuales, posicion) : actuales[0];
    if (!destino) continue;
    actuales = actuales.map((c) => (c.id === destino.id ? { ...c, residentesIds: [...c.residentesIds, heroe.id] } : c));
  }
  return actuales;
}
