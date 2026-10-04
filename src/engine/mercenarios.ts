// Campamentos de mercenarios (Doc 1.9b, `Docs/Mecanicas a desarrollar.md` §40): enclaves neutrales del mundo abierto. Se
// colocan al crear la partida (`colocarCampamentosIniciales`) y no aparecen más; aquí también vive la residencia.
//
// Un campamento no es un asentamiento: no tiene Facción, ni zona de influencia (ninguna lo absorbe, porque las zonas
// salen de los asentamientos), ni crece, y no desaparece. Por eso no vive en `asentamientos` sino en su propia lista.
import type { Asentamiento, CampamentoMercenarios, EdificioCampamentoTipo, Ejercito, Heroe, Point } from '../domain/types';
import { MERCENARIOS } from '../constants';
import type { Instante } from '../domain/tiempo';
import type { Mapa } from '../world/mapa';
import { createRng } from '../worldgen';
import { dejarResidencia } from './faccion';
import { esResidente } from './pertenencia';
import { ReglaInvalidaError } from './errores';
import { mercadoInicial } from './mercadoMercenario';

const distancia = (a: Point, b: Point): number => Math.hypot(a.x - b.x, a.y - b.y);

/** Terreno donde cabe un campamento: dentro del mapa, que se pueda pisar y que no sea agua ni cima. */
function terrenoValido(mapa: Mapa, p: Point): boolean {
  if (!mapa.dentroDelMapa(p) || !mapa.esTransitable(p)) return false;
  const terreno = mapa.terrenoEn(p);
  return terreno !== 'agua' && terreno !== 'cima';
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
    eligieronComoInicial: 0,
    // Nace lleno: el tope son sus viviendas.
    poblacion: edificios.filter((e) => e === 'vivienda').length * MERCENARIOS.poblacionPorVivienda,
    poblacionEn: instante,
    mercado: mercadoInicial(),
    fondos: {},
    creadoEn: instante,
  };
}

/**
 * Los campamentos del mundo (Doc 1.9b, D1/D32): se colocan UNA vez, al crear la partida, y no aparecen más. Puntos al azar
 * en tierra firme —pegados a un bosque si lo hay cerca— a `separacion` como mínimo unos de otros y a `margenBorde` del
 * borde; caben los que caben. El azar sale de una semilla DERIVADA de la del mapa (D35): misma seed, mismos
 * campamentos, sin consumir el generador compartido de la partida.
 */
export function colocarCampamentosIniciales(mapa: Mapa, instante: Instante): CampamentoMercenarios[] {
  const rng = createRng(mapa.seed ^ MERCENARIOS.salSemilla);
  const { ancho, alto } = mapa.limites;
  const m = MERCENARIOS.margenBorde;
  const puestos: Point[] = [];
  for (let i = 0; i < MERCENARIOS.intentosColocacion; i++) {
    const p = pegadoABosque(mapa, { x: m + rng() * (ancho - 2 * m), y: m + rng() * (alto - 2 * m) });
    if (terrenoValido(mapa, p) && puestos.every((q) => distancia(p, q) >= MERCENARIOS.separacion)) puestos.push(p);
  }
  return puestos.map((p, i) => nuevoCampamento(i, p, instante));
}

export class MercenariosInvalidoError extends ReglaInvalidaError {}

/** ¿Está `p` bajo la protección de algún campamento (M4/D78)? Ahí nadie inicia un combate. */
export const enProteccionDeCampamento = (p: Point, campamentos: readonly CampamentoMercenarios[]): boolean =>
  campamentos.some((c) => distancia(p, c.posicion) <= MERCENARIOS.radioProteccion);

/** El campamento donde reside un héroe, si reside en alguno. */
export const campamentoDeResidente = (campamentos: readonly CampamentoMercenarios[], heroeId: string): CampamentoMercenarios | undefined =>
  campamentos.find((c) => c.residentesIds.includes(heroeId));

/** El campamento más cercano a un punto (a igual distancia, el de id menor: determinista). */
export function campamentoMasCercano(campamentos: readonly CampamentoMercenarios[], desde: Point): CampamentoMercenarios | undefined {
  return [...campamentos].sort((a, b) => distancia(a.posicion, desde) - distancia(b.posicion, desde) || (a.id < b.id ? -1 : 1))[0];
}

/** Los campamentos sin esos héroes como residentes. */
function sinResidente(campamentos: readonly CampamentoMercenarios[], ...heroeIds: string[]): CampamentoMercenarios[] {
  return campamentos.map((c) => (c.residentesIds.some((id) => heroeIds.includes(id)) ? { ...c, residentesIds: c.residentesIds.filter((id) => !heroeIds.includes(id)) } : c));
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
export function posicionDeHeroe(heroe: Heroe, asentamientos: readonly Asentamiento[], ejercitos: readonly Ejercito[]): Point | undefined {
  const u = heroe.ubicacion;
  if (u.tipo === 'columna') return ejercitos.find((e) => e.id === u.ejercitoId)?.posicionActual;
  if (u.tipo === 'asentamiento') return asentamientos.find((a) => a.id === u.asentamientoId)?.posicion;
  return undefined;
}

/**
 * Se acaba el huérfano (Doc 0, 5.15.5, decidido el 2026-10-02): quien pierde la casa pasa a residir, en el acto, en el campamento
 * de mercenarios más cercano a `desde` —el sitio que perdió—. Lo llaman los caminos por los que se pierde una casa: la conquista de
 * su plaza, su ruina y dejar la residencia; sin campamentos devuelve lo mismo. Un héroe que ya reside en uno se muda al más cercano.
 */
export function acogerEnCampamentoMasCercano(campamentos: readonly CampamentoMercenarios[], heroeIds: readonly string[], desde: Point): CampamentoMercenarios[] {
  const destino = campamentoMasCercano(campamentos, desde);
  if (!destino || heroeIds.length === 0) return [...campamentos];
  let sin = [...campamentos];
  for (const id of heroeIds) sin = sinResidente(sin, id);
  return sin.map((c) => (c.id === destino.id ? { ...c, residentesIds: [...c.residentesIds, ...heroeIds] } : c));
}

/**
 * Los residentes de una plaza que desaparece (ruina por abandono, Doc 4.5) se reubican: en la plaza más cercana de su Facción si le
 * queda alguna —donde pasan a residir— y, si no, en el campamento de mercenarios más cercano. Sin esto quedaban sin casa.
 */
export function reubicarResidentesDeRuina(
  ruina: Asentamiento,
  restantes: readonly Asentamiento[],
  campamentos: readonly CampamentoMercenarios[]
): { asentamientos: Asentamiento[]; campamentos: CampamentoMercenarios[] } {
  const residentes = [...ruina.heroesFundadoresIds, ...ruina.casasCompradas];
  const refugio = restantes
    .filter((a) => a.faccionId === ruina.faccionId)
    .sort((a, b) => distancia(a.posicion, ruina.posicion) - distancia(b.posicion, ruina.posicion) || (a.id < b.id ? -1 : 1))[0];
  if (residentes.length === 0) return { asentamientos: [...restantes], campamentos: [...campamentos] };
  if (refugio) {
    return {
      asentamientos: restantes.map((a) => (a.id === refugio.id ? { ...a, casasCompradas: [...a.casasCompradas, ...residentes] } : a)),
      campamentos: [...campamentos],
    };
  }
  return { asentamientos: [...restantes], campamentos: acogerEnCampamentoMasCercano(campamentos, residentes, ruina.posicion) };
}
