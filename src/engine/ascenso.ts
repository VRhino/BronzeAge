// Subida de nivel de asentamiento MANUAL y con coste (Doc 4.5; decisión del usuario del 2026-09-26,
// `Consideraciones/Ritmo_Crecimiento_Asentamientos.md` §10). Antes el nivel subía solo en cuanto se cumplían los
// gates de población+edificios, y el batch medía nivel 2 a las 3 horas de vida y muerte por oro a las 4 h 40: el
// asentamiento llegaba al nivel 2 antes de tener con qué pagarlo. Ahora los gates son el REQUISITO para pedir la
// subida, y la subida pide además cuatro cosas:
//   1. pagar el coste de la obra (`ASCENSO_ASENTAMIENTO`), entero al empezar;
//   2. esperar la obra (`completaEn`), que es la palanca de ritmo que no depende de la economía;
//   3. ser SOLVENTE: los ingresos de hoy cubren el mantenimiento del nivel objetivo, recurso a recurso;
//   4. cupo de la Facción para ese nivel (`CUPO_NIVEL_ASENTAMIENTO`, Doc Fase_0_5 §5), que se reserva al pedir.
//
// Este archivo es motor puro: quién puede pedirla (el Gobernador) lo decide la capa de sesión
// (`session/comandos/ascenso.ts`), y cuándo la pide un NPC, `session/npcGobernanza.ts`.
import type { Asentamiento, Faccion } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { ASCENSO_ASENTAMIENTO, NIVEL_ASENTAMIENTO } from '../constants';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import type { Mapa } from '../world/mapa';
import { descontarRecursos, tieneRecursos } from './almacen';
import { nivelActualDe, produccionPorMinuto } from './asentamientoQuery';
import { calcularCupoNivel } from './faccion';
import { calcularCostoMantenimiento, encontrarCapital, evaluarGatesDeNivel, type PayloadNivelSubio } from './mantenimiento';
import { recaudacionOro } from './population';
import { computeZonaInfluencia } from './zones';

export class AscensoInvalidoError extends Error {}

/** Por qué no se puede pedir la subida. Puede haber varios a la vez; el cliente los enseña todos. */
export type BloqueoAscenso =
  | 'nivel_maximo'
  | 'ascenso_en_curso'
  | 'falta_poblacion'
  | 'faltan_edificios'
  | 'sin_cupo_de_faccion'
  | 'recursos_insuficientes'
  | 'insolvente';

export interface SolvenciaRecurso {
  recurso: string;
  /** Lo que el asentamiento produce o recauda hoy por minuto de mundo. */
  ingresoPorMinuto: number;
  /** Lo que el mantenimiento le cobraría por minuto en el nivel objetivo. */
  costoPorMinuto: number;
}

export interface EvaluacionAscenso {
  /** `nivel` (alcanzado) de hoy. */
  nivel: number;
  /** `nivel + 1`, o `null` si ya está en el máximo. */
  nivelObjetivo: number | null;
  costo: Partial<Record<string, number>>;
  obraMinutos: number;
  solvencia: SolvenciaRecurso[];
  bloqueos: BloqueoAscenso[];
  puede: boolean;
}

export interface PayloadAscensoIniciado {
  asentamientoId: string;
  nivelObjetivo: number;
  costo: Partial<Record<string, number>>;
  completaEn: Instante;
}

/**
 * ¿Queda plaza en `nivelObjetivo` para otro asentamiento de esta Facción? Ocupan plaza los que ya están en ese
 * nivel (`nivelActual`, mismo criterio que antes) y los que tienen una obra de ascenso en curso hacia él: la
 * plaza se RESERVA al pedir, para que dos asentamientos no paguen a la vez por la última y al terminar la obra no
 * haya que volver a mirar. Subir de 2 a 3 libera sola la plaza de 2, porque se recuenta desde el estado.
 *
 * La curva solo existe para los niveles 2 y 3; los niveles 4 y 5 no tienen cupo todavía (ilimitados), igual que
 * antes de la subida manual.
 */
export function cupoLibreParaNivel(
  faccion: Faccion | undefined,
  asentamientos: readonly Asentamiento[],
  nivelObjetivo: number
): boolean {
  if (nivelObjetivo !== 2 && nivelObjetivo !== 3) return true;
  if (!faccion) return false;
  const ocupadas = asentamientos.filter(
    (a) =>
      a.faccionId === faccion.id && (nivelActualDe(a) === nivelObjetivo || a.ascenso?.nivelObjetivo === nivelObjetivo)
  ).length;
  return ocupadas < calcularCupoNivel(faccion.nivel, nivelObjetivo);
}

/**
 * Ingresos por minuto de cada recurso contra el mantenimiento que el asentamiento pagaría en `nivelObjetivo`
 * (`calcularCostoMantenimiento` con ese nivel). Ingreso = producción bruta de sus edificios
 * (`produccionPorMinuto`) más, para el oro, la recaudación (`recaudacionOro`). Bruto a propósito: la prueba es
 * "¿puede sostener el nivel?", no "¿le sobra?", y lo que consuman recetas o tropas ya lo ve el Gobernador.
 */
function solvenciaEnNivel(
  asentamiento: Asentamiento,
  nivelObjetivo: number,
  asentamientos: readonly Asentamiento[],
  mapa: Mapa,
  instante: Instante
): SolvenciaRecurso[] {
  const capital = encontrarCapital(asentamiento.faccionId, [...asentamientos]);
  const costo = calcularCostoMantenimiento({ ...asentamiento, nivel: nivelObjetivo, nivelActual: nivelObjetivo }, capital);
  const zona = computeZonaInfluencia(asentamiento, [...asentamientos]).poligono;
  const ingresos = new Map<string, number>();
  for (const item of produccionPorMinuto(asentamiento, mapa, zona)) {
    ingresos.set(item.recurso, (ingresos.get(item.recurso) ?? 0) + item.cantidadPorMinuto);
  }
  ingresos.set('oro', (ingresos.get('oro') ?? 0) + recaudacionOro(asentamiento, instante));
  return Object.entries(costo).map(([recurso, costoPorMinuto]) => ({
    recurso,
    ingresoPorMinuto: ingresos.get(recurso) ?? 0,
    costoPorMinuto: costoPorMinuto ?? 0,
  }));
}

/** Coste y obra de la subida a `nivelObjetivo`, o `undefined` si no se puede pedir: por encima del nivel máximo o del
 * techo provisional (`ASCENSO_ASENTAMIENTO.nivelTechoProvisional`). */
export function tarifaDeAscenso(nivelObjetivo: number): { costo: Partial<Record<string, number>>; obraMinutos: number } | undefined {
  if (nivelObjetivo > ASCENSO_ASENTAMIENTO.nivelTechoProvisional) return undefined;
  return ASCENSO_ASENTAMIENTO.porNivelObjetivo[nivelObjetivo];
}

/**
 * Si este asentamiento puede pedir ya la subida al nivel siguiente y, si no, por qué. Solo lectura: la usan el
 * comando del Gobernador (para validar), el NPC (para decidir) y el servidor (para enseñárselo al jugador).
 */
export function evaluarAscenso(
  asentamiento: Asentamiento,
  asentamientos: readonly Asentamiento[],
  facciones: readonly Faccion[],
  mapa: Mapa,
  instante: Instante
): EvaluacionAscenso {
  const nivel = asentamiento.nivel;
  const nivelObjetivo = nivel < NIVEL_ASENTAMIENTO.nivelMaximo ? nivel + 1 : null;
  const tarifa = nivelObjetivo === null ? undefined : tarifaDeAscenso(nivelObjetivo);
  if (nivelObjetivo === null || !tarifa) {
    return { nivel, nivelObjetivo: null, costo: {}, obraMinutos: 0, solvencia: [], bloqueos: ['nivel_maximo'], puede: false };
  }

  const bloqueos: BloqueoAscenso[] = [];
  if (asentamiento.ascenso) bloqueos.push('ascenso_en_curso');
  const gates = evaluarGatesDeNivel(asentamiento, nivelObjetivo);
  if (gates && !gates.poblacion) bloqueos.push('falta_poblacion');
  if (gates && !gates.edificios) bloqueos.push('faltan_edificios');
  const faccion = facciones.find((f) => f.id === asentamiento.faccionId);
  if (!cupoLibreParaNivel(faccion, asentamientos, nivelObjetivo)) bloqueos.push('sin_cupo_de_faccion');
  if (!tieneRecursos(asentamiento.almacen, tarifa.costo)) bloqueos.push('recursos_insuficientes');
  const solvencia = solvenciaEnNivel(asentamiento, nivelObjetivo, asentamientos, mapa, instante);
  if (solvencia.some((s) => s.ingresoPorMinuto < s.costoPorMinuto)) bloqueos.push('insolvente');

  return { nivel, nivelObjetivo, costo: tarifa.costo, obraMinutos: tarifa.obraMinutos, solvencia, bloqueos, puede: bloqueos.length === 0 };
}

/**
 * Empieza la obra de ascenso: cobra el coste entero del almacén del asentamiento y fija cuándo termina. Lanza
 * `AscensoInvalidoError` con todos los bloqueos si no se puede.
 */
export function iniciarAscenso(
  asentamiento: Asentamiento,
  asentamientos: readonly Asentamiento[],
  facciones: readonly Faccion[],
  mapa: Mapa,
  instante: Instante
): { asentamiento: Asentamiento; eventos: EventoCrudo[] } {
  const evaluacion = evaluarAscenso(asentamiento, asentamientos, facciones, mapa, instante);
  if (!evaluacion.puede || evaluacion.nivelObjetivo === null) {
    throw new AscensoInvalidoError(`No se puede subir de nivel: ${evaluacion.bloqueos.join(', ')}.`);
  }
  const completaEn = sumar(instante, minutos(evaluacion.obraMinutos));
  return {
    asentamiento: {
      ...asentamiento,
      almacen: descontarRecursos(asentamiento.almacen, evaluacion.costo),
      ascenso: { nivelObjetivo: evaluacion.nivelObjetivo, iniciadoEn: instante, completaEn },
    },
    eventos: [
      {
        codigo: 'asentamiento.ascenso_iniciado',
        mensaje: `${asentamiento.id} empieza la obra de ascenso a nivel ${evaluacion.nivelObjetivo}.`,
        payload: {
          asentamientoId: asentamiento.id,
          nivelObjetivo: evaluacion.nivelObjetivo,
          costo: evaluacion.costo,
          completaEn,
        } satisfies PayloadAscensoIniciado,
      },
    ],
  };
}

/**
 * Paso del tick: si la obra de ascenso ha terminado, sube el nivel. No vuelve a mirar gates, cupo ni solvencia:
 * todo se comprobó al pedirla y el cupo quedó reservado desde entonces.
 *
 * Doc Fase_0_5 §6.2: la promoción sube `nivelActual` junto con `nivel`, salvo que el asentamiento estuviera
 * degradado (nivelActual < nivel), en cuyo caso la subida no "cura" la degradación de golpe.
 */
export function avanzarAscenso(
  asentamiento: Asentamiento,
  instante: Instante
): { asentamiento: Asentamiento; eventos: EventoCrudo[] } {
  const obra = asentamiento.ascenso;
  if (!obra || instante < obra.completaEn) return { asentamiento, eventos: [] };
  const yaEstabaAlDia = nivelActualDe(asentamiento) === asentamiento.nivel;
  const { ascenso: _terminado, ...sinObra } = asentamiento;
  return {
    asentamiento: { ...sinObra, nivel: obra.nivelObjetivo, ...(yaEstabaAlDia ? { nivelActual: obra.nivelObjetivo } : {}) },
    eventos: [
      {
        codigo: 'asentamiento.nivel_subio',
        mensaje: `${asentamiento.id} sube a nivel ${obra.nivelObjetivo}.`,
        payload: { asentamientoId: asentamiento.id, nivelNuevo: obra.nivelObjetivo } satisfies PayloadNivelSubio,
      },
    ],
  };
}
