// Aedas residentes (Doc 6.7): viven en un asentamiento con Palacio y Nobleza, dan felicidad y nobleza, y desbloquean tecnología
// con una épica por capítulos que avanza con hechos inspiradores. Dos mitades: el ciclo de vida (llegan, se van) que corre en el
// tick, y las épicas, que se avanzan con los eventos de cada hecho de la partida (`avanzarEpicas`, desde `exito`).
// Sin RNG: nombres, llegadas y hechos salen del orden y del instante.
import { AEDAS, EPICAS, ERAS, TECNOLOGIAS, TITULO_CAPITULO, type HechoEpico } from '../constants';
import type { EventoCrudo, EventoDominio } from '../domain/eventos';
import { minutos, sumar, transcurrido, type Instante } from '../domain/tiempo';
import type { AedaResidente, Asentamiento, EdificioTipo, EpicaEnCurso, EstadoAedasResidentes, EstadoTecnologia, Faccion, TecnologiaId } from '../domain/types';
import { edificiosPorTipoYEstado, nivelActualDe } from './asentamientoQuery';
import type { PayloadAsedio, PayloadEncuentroEjercitos } from './combate';
import type { PayloadCaravanaLlega } from './trade';
import { ReglaInvalidaError } from './errores';
import { hacerAparecer, tecnologiasDe } from './tecnologia';

export const RESIDENTES_VACIOS: EstadoAedasResidentes = { aedas: [], siguiente: 1, esperaDesde: {}, cumplidas: {} };

const NOMBRES = [
  'Femio', 'Demódoco', 'Orfeo', 'Lino', 'Tamiris', 'Museo', 'Eumolpo', 'Filamón', 'Arión', 'Hesíodo', 'Terpandro', 'Alcmán',
  'Arquíloco', 'Tirteo', 'Mimnermo', 'Estesícoro', 'Íbico', 'Anacreonte', 'Simónides', 'Corina', 'Safo', 'Alceo', 'Olén', 'Anfión',
];

/** Cuántos Aedas residentes caben en la plaza ahora: ninguno sin Palacio activo o sin Nobleza (Doc 6.7). */
export function cupoDeResidentes(plaza: Asentamiento): number {
  if (edificiosPorTipoYEstado(plaza, 'palacio').length === 0 || plaza.poblacion.nobleza <= 0) return 0;
  return AEDAS.residentes.cupoPorNivel[nivelActualDe(plaza)] ?? 0;
}

/** Aedas residentes por plaza (los efectos de felicidad y nobleza se miden con esto). */
export function residentesPorPlaza(estado: EstadoAedasResidentes): Map<string, number> {
  const cuenta = new Map<string, number>();
  for (const a of estado.aedas) cuenta.set(a.asentamientoId, (cuenta.get(a.asentamientoId) ?? 0) + 1);
  return cuenta;
}

export interface ContextoResidentes {
  asentamientos: readonly Asentamiento[];
  facciones: readonly Faccion[];
  tecnologia: EstadoTecnologia;
  instante: Instante;
}

/** Payloads de los eventos de los residentes (privados de la Facción, salvo `aedas.epica_cumplida`, público). */
export interface PayloadAedaResidente {
  aedaId: string;
  faccionId: string;
}
export interface PayloadEpica extends PayloadAedaResidente {
  tecnologiaId: TecnologiaId;
  capitulo?: number;
  capitulos?: number;
}

const nombreDePlaza = (p: Asentamiento) => p.nombre ?? p.id;

/** Cuánto tarda en llegar el siguiente: más con la reputación por los suelos (Doc 2.7). */
function minutosDeLlegada(faccion: Faccion | undefined): number {
  const baja = faccion !== undefined && faccion.reputacion <= AEDAS.residentes.umbralReputacionBaja;
  return AEDAS.residentes.llegadaMinutos * (baja ? AEDAS.residentes.factorLlegadaReputacionBaja : 1);
}

/**
 * Un tick del ciclo de vida de los residentes: se van los de una plaza que ya no es (o sin nobleza o con menos cupo), pierden la
 * épica los de una plaza conquistada o cuya tecnología ya ha aparecido, y llega uno nuevo a cada plaza con cupo cuando se cumple su plazo.
 */
export function avanzarResidentes(estado: EstadoAedasResidentes, ctx: ContextoResidentes): { estado: EstadoAedasResidentes; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const esperaDesde = { ...estado.esperaDesde };
  let siguiente = estado.siguiente;
  let aedas = estado.aedas.filter((a) => ctx.asentamientos.some((p) => p.id === a.asentamientoId));

  // Una plaza conquistada cambia de dueño y la épica en curso no la sigue; una tecnología que ya apareció por otra vía no necesita épica.
  aedas = aedas.map((a) => {
    const plaza = ctx.asentamientos.find((p) => p.id === a.asentamientoId)!;
    const cambia = a.faccionId !== plaza.faccionId;
    const sobra = a.epica !== undefined && tecnologiasDe(ctx.tecnologia, plaza.faccionId).aparecidas.includes(a.epica.tecnologiaId);
    if (!cambia && !sobra) return a;
    const { epica: _perdida, ...sinEpica } = a;
    return { ...sinEpica, faccionId: plaza.faccionId };
  });

  for (const plaza of ctx.asentamientos) {
    const cupo = cupoDeResidentes(plaza);
    const propios = aedas.filter((a) => a.asentamientoId === plaza.id).sort((x, y) => x.llegadaEn - y.llegadaEn || (x.id < y.id ? -1 : 1));
    if (propios.length > cupo) {
      const salen = new Set(propios.slice(cupo).map((a) => a.id));
      for (const a of propios.slice(cupo)) {
        eventos.push({
          codigo: 'aedas.residente_se_va',
          mensaje: `${a.nombre}, Aeda de ${nombreDePlaza(plaza)}, se marcha: ${cupo === 0 ? 'ya no hay nobleza que lo sostenga' : 'la ciudad ya no tiene sitio para tantos'}.`,
          payload: { aedaId: a.id, faccionId: plaza.faccionId } satisfies PayloadAedaResidente,
          asentamientoId: plaza.id,
        });
      }
      aedas = aedas.filter((a) => !salen.has(a.id));
    }
    if (propios.length >= cupo) {
      delete esperaDesde[plaza.id];
      continue;
    }
    const faccion = ctx.facciones.find((f) => f.id === plaza.faccionId);
    esperaDesde[plaza.id] ??= ctx.instante;
    if (ctx.instante < sumar(esperaDesde[plaza.id]!, minutos(minutosDeLlegada(faccion)))) continue;
    const nuevo: AedaResidente = { id: `aeda-r${siguiente}`, nombre: NOMBRES[(siguiente - 1) % NOMBRES.length]!, asentamientoId: plaza.id, faccionId: plaza.faccionId, llegadaEn: ctx.instante };
    siguiente++;
    aedas = [...aedas, nuevo];
    delete esperaDesde[plaza.id];
    eventos.push({
      codigo: 'aedas.residente_llega',
      mensaje: `${nuevo.nombre}, un Aeda, se asienta en ${nombreDePlaza(plaza)}.`,
      payload: { aedaId: nuevo.id, faccionId: plaza.faccionId } satisfies PayloadAedaResidente,
      asentamientoId: plaza.id,
    });
  }
  return { estado: { ...estado, aedas, siguiente, esperaDesde }, eventos };
}

// --- Épicas ---

export class EpicaInvalidaError extends ReglaInvalidaError {}

function reemplazar(estado: EstadoAedasResidentes, aeda: AedaResidente): EstadoAedasResidentes {
  return { ...estado, aedas: estado.aedas.map((a) => (a.id === aeda.id ? aeda : a)) };
}

function aedaDeLaPlaza(estado: EstadoAedasResidentes, plaza: Asentamiento, aedaId: string): AedaResidente {
  const aeda = estado.aedas.find((a) => a.id === aedaId && a.asentamientoId === plaza.id);
  if (!aeda) throw new EpicaInvalidaError('Ese Aeda no vive en esta plaza.');
  return aeda;
}

/**
 * El Aeda empieza la épica de una tecnología (Doc 6.7): con el logro del servidor cumplido, la Era abierta, aún no aparecida a la
 * Facción y sin otra épica suya ya en marcha para ella. Sustituye al hito, nunca al logro.
 */
export function empezarEpica(
  estado: EstadoAedasResidentes,
  tecnologia: EstadoTecnologia,
  plaza: Asentamiento,
  aedaId: string,
  id: TecnologiaId
): { estado: EstadoAedasResidentes; eventos: EventoCrudo[] } {
  const aeda = aedaDeLaPlaza(estado, plaza, aedaId);
  const definicion = EPICAS[id];
  if (!definicion) throw new EpicaInvalidaError('Esa tecnología no tiene épica.');
  if (aeda.faccionId !== plaza.faccionId) throw new EpicaInvalidaError(`${aeda.nombre} aún no se ha asentado con el nuevo dueño de la plaza.`);
  if (aeda.epica) throw new EpicaInvalidaError(`${aeda.nombre} ya canta una épica: hay que abandonarla antes.`);
  if (tecnologia.logros[id] === undefined) throw new EpicaInvalidaError(`El logro del servidor de ${TECNOLOGIAS[id].nombre} aún no se ha cumplido.`);
  if (ERAS[TECNOLOGIAS[id].era].orden > ERAS[tecnologia.era].orden) throw new EpicaInvalidaError(`La Era de ${TECNOLOGIAS[id].nombre} aún no ha empezado.`);
  if (tecnologiasDe(tecnologia, plaza.faccionId).aparecidas.includes(id)) throw new EpicaInvalidaError(`${TECNOLOGIAS[id].nombre} ya le ha aparecido a la Facción.`);
  if (estado.aedas.some((a) => a.faccionId === plaza.faccionId && a.epica?.tecnologiaId === id)) throw new EpicaInvalidaError(`Otro Aeda de la Facción ya canta la épica de ${TECNOLOGIAS[id].nombre}.`);
  const epica: EpicaEnCurso = { tecnologiaId: id, capitulo: 0, hechos: 0, claves: [] };
  return {
    estado: reemplazar(estado, { ...aeda, epica }),
    eventos: [
      {
        codigo: 'aedas.epica_empieza',
        mensaje: `${aeda.nombre} empieza el ${definicion.nombre}.`,
        payload: { aedaId: aeda.id, faccionId: plaza.faccionId, tecnologiaId: id, capitulo: 0, capitulos: definicion.capitulos.length } satisfies PayloadEpica,
        asentamientoId: plaza.id,
      },
    ],
  };
}

/** El Aeda abandona la épica en curso y pierde lo avanzado. */
export function abandonarEpica(estado: EstadoAedasResidentes, plaza: Asentamiento, aedaId: string): { estado: EstadoAedasResidentes; eventos: EventoCrudo[] } {
  const aeda = aedaDeLaPlaza(estado, plaza, aedaId);
  if (!aeda.epica) throw new EpicaInvalidaError(`${aeda.nombre} no canta ninguna épica.`);
  const { epica, ...sinEpica } = aeda;
  return {
    estado: reemplazar(estado, sinEpica),
    eventos: [
      {
        codigo: 'aedas.epica_abandonada',
        mensaje: `${aeda.nombre} abandona el ${EPICAS[epica.tecnologiaId]!.nombre}.`,
        payload: { aedaId: aeda.id, faccionId: plaza.faccionId, tecnologiaId: epica.tecnologiaId } satisfies PayloadEpica,
        asentamientoId: plaza.id,
      },
    ],
  };
}

/**
 * Lo que la épica lee de `batalla.aplicada` (`PayloadBatalla`, `session/batallas.ts`): las batallas de Unity no emiten los eventos
 * de combate del motor, así que cuentan por aquí. Solo los campos que se usan: el motor no importa de la capa de sesión.
 */
interface PayloadBatallaAplicada {
  battleId: string;
  contexto: { tipo: string; asentamientoId?: string };
  faccionAtacanteId: string | null;
  faccionDefensoraId: string | null;
  ganador?: 'atacante' | 'defensor';
}

/** Un hecho inspirador sacado de un evento: de qué tipo, de quién, dónde y con qué clave irrepetible. */
interface Hecho {
  tipo: HechoEpico;
  faccionId: string;
  plazaId?: string;
  edificio?: EdificioTipo;
  clave: string;
}

/** Los hechos que cuentan para una épica (Doc 6.7), de unos eventos recién ocurridos. Lo repetible o trivial no entra: asedios sin combate, caravanas ligeras, batallas contra la propia Facción. */
export function hechosDeEventos(eventos: readonly EventoDominio[], asentamientos: readonly Asentamiento[]): Hecho[] {
  const faccionDe = (plazaId: string) => asentamientos.find((a) => a.id === plazaId)?.faccionId;
  const hechos: Hecho[] = [];
  for (const e of eventos) {
    switch (e.codigo) {
      case 'combate.asedio_resistido': {
        const p = e.payload as PayloadAsedio;
        if (p.enCombate) hechos.push({ tipo: 'defensa', faccionId: p.faccionDefensoraId, plazaId: p.defensorId, clave: `defensa:${p.defensorId}:${e.momento}` });
        break;
      }
      case 'combate.asedio_conquista': {
        const p = e.payload as PayloadAsedio;
        if (p.enCombate) hechos.push({ tipo: 'conquista', faccionId: p.faccionAtacanteId, clave: `conquista:${p.defensorId}:${e.momento}` });
        break;
      }
      case 'combate.encuentro': {
        const p = e.payload as PayloadEncuentroEjercitos;
        if (p.faccionAId === p.faccionBId) break;
        hechos.push({ tipo: 'batalla', faccionId: p.ganadorId === p.ejercitoAId ? p.faccionAId : p.faccionBId, clave: `batalla:${p.ejercitoAId}:${p.ejercitoBId}:${e.momento}` });
        break;
      }
      case 'construccion.edificio_completado':
      case 'construccion.mejora_completada': {
        const p = e.payload as { edificioId: string; edificioTipo: EdificioTipo; nivelNuevo?: number };
        const faccionId = e.asentamientoId && faccionDe(e.asentamientoId);
        if (faccionId) hechos.push({ tipo: 'obra', faccionId, plazaId: e.asentamientoId, edificio: p.edificioTipo, clave: `obra:${p.edificioId}:${p.nivelNuevo ?? 0}` });
        break;
      }
      case 'asentamiento.nivel_subio': {
        const p = e.payload as { asentamientoId: string; nivelNuevo: number };
        const faccionId = faccionDe(p.asentamientoId);
        if (faccionId) hechos.push({ tipo: 'ascenso', faccionId, plazaId: p.asentamientoId, clave: `ascenso:${p.asentamientoId}:${p.nivelNuevo}` });
        break;
      }
      case 'batalla.aplicada': {
        // Una batalla de Unity: se canta una vez aunque llegue un evento por cada plaza implicada (la clave es la de la batalla).
        const p = e.payload as PayloadBatallaAplicada;
        const { contexto, ganador } = p;
        if (!ganador) break;
        if (contexto.tipo === 'campo_abierto' && p.faccionAtacanteId && p.faccionDefensoraId && p.faccionAtacanteId !== p.faccionDefensoraId) {
          hechos.push({ tipo: 'batalla', faccionId: ganador === 'atacante' ? p.faccionAtacanteId : p.faccionDefensoraId, clave: `batalla:${p.battleId}` });
        } else if (contexto.tipo === 'asedio' && contexto.asentamientoId) {
          if (ganador === 'defensor' && p.faccionDefensoraId) hechos.push({ tipo: 'defensa', faccionId: p.faccionDefensoraId, plazaId: contexto.asentamientoId, clave: `defensa:${p.battleId}` });
          if (ganador === 'atacante' && p.faccionAtacanteId) hechos.push({ tipo: 'conquista', faccionId: p.faccionAtacanteId, clave: `conquista:${p.battleId}` });
        }
        break;
      }
      case 'tecnologia.adoptada': {
        const p = e.payload as { faccionId: string; tecnologiaId: string };
        hechos.push({ tipo: 'adopcion', faccionId: p.faccionId, clave: `adopcion:${p.faccionId}:${p.tecnologiaId}` });
        break;
      }
      case 'comercio.caravana_llega': {
        const p = e.payload as PayloadCaravanaLlega;
        const faccionId = faccionDe(p.origenId);
        const carga = Object.values(p.contenido).reduce((a, n) => a + n, 0);
        if (faccionId && p.tipo === 'comercial' && carga >= AEDAS.epica.cargaMinimaCaravana) hechos.push({ tipo: 'caravana', faccionId, plazaId: p.origenId, clave: `caravana:${p.caravanaId}` });
        break;
      }
    }
  }
  return hechos;
}

/** Los hechos de plaza solo inspiran al Aeda de esa plaza; los de la Facción, a los de cualquiera de sus plazas. */
function inspira(hecho: Hecho, aeda: AedaResidente, capitulo: { hecho: HechoEpico; edificio?: EdificioTipo }): boolean {
  if (hecho.tipo !== capitulo.hecho || (capitulo.edificio && hecho.edificio !== capitulo.edificio)) return false;
  return hecho.plazaId === undefined ? hecho.faccionId === aeda.faccionId : hecho.plazaId === aeda.asentamientoId;
}

/**
 * Avanza las épicas con los hechos de unos eventos (Doc 6.7). Cada hecho cuenta una vez por épica (su clave) y a lo sumo uno
 * cada `AEDAS.epica.enfriamientoMinutos`; al cerrar el último capítulo la tecnología le aparece a la Facción sin hito.
 */
export function avanzarEpicas(
  estado: EstadoAedasResidentes,
  eventos: readonly EventoDominio[],
  ctx: ContextoResidentes
): { estado: EstadoAedasResidentes; tecnologia: EstadoTecnologia; eventos: EventoCrudo[] } {
  const sinCambios = { estado, tecnologia: ctx.tecnologia, eventos: [] as EventoCrudo[] };
  if (!estado.aedas.some((a) => a.epica)) return sinCambios;
  const hechos = hechosDeEventos(eventos, ctx.asentamientos);
  if (hechos.length === 0) return sinCambios;

  let actual = estado;
  let tecnologia = ctx.tecnologia;
  const salida: EventoCrudo[] = [];
  for (const aeda of estado.aedas) {
    const epica = aeda.epica;
    const plaza = ctx.asentamientos.find((p) => p.id === aeda.asentamientoId);
    // Una plaza que cambió de dueño este tick aún no ha soltado la épica (`avanzarResidentes`): no acredita hechos a la Facción vieja.
    if (!epica || !plaza || plaza.faccionId !== aeda.faccionId) continue;
    const definicion = EPICAS[epica.tecnologiaId]!;
    const capitulo = definicion.capitulos[epica.capitulo]!;
    if (epica.ultimoHechoEn !== undefined && transcurrido(epica.ultimoHechoEn, ctx.instante) < minutos(AEDAS.epica.enfriamientoMinutos)) continue;
    const hecho = hechos.find((h) => !epica.claves.includes(h.clave) && inspira(h, aeda, capitulo));
    if (!hecho) continue;

    const claves = [...epica.claves, hecho.clave].slice(-AEDAS.epica.clavesRecordadas);
    const hechosContados = epica.hechos + 1;
    if (hechosContados < capitulo.cantidad) {
      actual = reemplazar(actual, { ...aeda, epica: { ...epica, hechos: hechosContados, ultimoHechoEn: ctx.instante, claves } });
      continue;
    }
    const payload = { aedaId: aeda.id, faccionId: aeda.faccionId, tecnologiaId: epica.tecnologiaId, capitulo: epica.capitulo + 1, capitulos: definicion.capitulos.length } satisfies PayloadEpica;
    if (epica.capitulo + 1 < definicion.capitulos.length) {
      actual = reemplazar(actual, { ...aeda, epica: { ...epica, capitulo: epica.capitulo + 1, hechos: 0, ultimoHechoEn: ctx.instante, claves } });
      salida.push({ codigo: 'aedas.epica_capitulo', mensaje: `${aeda.nombre} canta un capítulo del ${definicion.nombre}: ${TITULO_CAPITULO[capitulo.hecho].toLowerCase()}.`, payload, asentamientoId: aeda.asentamientoId });
      continue;
    }
    const { epica: _cumplida, ...sinEpica } = aeda;
    actual = { ...reemplazar(actual, sinEpica), cumplidas: { ...actual.cumplidas, [aeda.faccionId]: (actual.cumplidas[aeda.faccionId] ?? 0) + 1 } };
    tecnologia = hacerAparecer(tecnologia, aeda.faccionId, epica.tecnologiaId);
    const faccion = ctx.facciones.find((f) => f.id === aeda.faccionId);
    salida.push(
      { codigo: 'aedas.epica_tecnologia', mensaje: `${aeda.nombre} completa el ${definicion.nombre}: ${TECNOLOGIAS[epica.tecnologiaId].nombre} aparece, sin pasar por su hito.`, payload, asentamientoId: aeda.asentamientoId },
      // Pública y sin nombrar la tecnología: la épica se canta, no su premio (Doc 6.3).
      { codigo: 'aedas.epica_cumplida', mensaje: `Los Aedas cantan que ${aeda.nombre}, de ${faccion?.nombre ?? aeda.faccionId}, ha completado una épica.`, payload: { aedaId: aeda.id, faccionId: aeda.faccionId } satisfies PayloadAedaResidente }
    );
  }
  return { estado: actual, tecnologia, eventos: salida };
}
