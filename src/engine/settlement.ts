import type { Asentamiento, Faccion, Point, RecursoAlmacenado, World } from '../domain/types';
import { ALMACEN, FUNDACION, MANTENIMIENTO, POBLACION, ZONA_INFLUENCIA } from '../constants';
import { posicionLibreParaFundar } from './zones';
import { calcularCapFundacion, otorgarCiudadania } from './faccion';

export class FundacionInvalidaError extends Error {}

function dentroDelMapa(p: Point, world: World): boolean {
  return p.x >= 0 && p.x <= world.config.ancho && p.y >= 0 && p.y <= world.config.alto;
}

/**
 * Fundación libre — Doc 1.2/1.3. Hasta 5 jugadores pueden fundar juntos, todos reciben ciudadanía
 * inmediata de la Facción fundadora (Doc 2.5). Respeta el cap de fundación por Facción (Doc 1.7):
 * no aplica a conquista/anexión (fuera de alcance aquí), solo a fundación directa.
 */
export function fundarAsentamiento(
  world: World,
  facciones: Faccion[],
  faccionId: string,
  posicion: Point,
  jugadoresFundadoresIds: string[],
  asentamientosExistentes: Asentamiento[],
  tickActual: number
): { asentamiento: Asentamiento; facciones: Faccion[] } {
  if (jugadoresFundadoresIds.length < 1 || jugadoresFundadoresIds.length > FUNDACION.maxJugadoresFundacionGrupal) {
    throw new FundacionInvalidaError(
      `La fundación grupal admite entre 1 y ${FUNDACION.maxJugadoresFundacionGrupal} jugadores.`
    );
  }
  if (!dentroDelMapa(posicion, world)) {
    throw new FundacionInvalidaError('La posición cae fuera de los límites del mapa.');
  }
  if (!posicionLibreParaFundar(posicion, asentamientosExistentes)) {
    throw new FundacionInvalidaError('La posición está dentro de una zona de influencia existente.');
  }

  const faccion = facciones.find((f) => f.id === faccionId);
  if (!faccion) throw new FundacionInvalidaError('La Facción no existe.');
  const asentamientosDeFaccion = asentamientosExistentes.filter((a) => a.faccionId === faccionId).length;
  const cap = calcularCapFundacion(faccion.nivel);
  if (asentamientosDeFaccion >= cap) {
    throw new FundacionInvalidaError(`Cap de fundación alcanzado (${asentamientosDeFaccion}/${cap} en nivel ${faccion.nivel}).`);
  }

  const almacenInicial: Record<string, RecursoAlmacenado> = {};
  for (const tipo of ['madera', 'piedra', 'trigo', 'cobre', 'estano', 'oro', 'livestock']) {
    almacenInicial[tipo] = {
      cantidad: FUNDACION.materialesIniciales[tipo] ?? 0,
      capacidad: ALMACEN.capacidadInicialPorRecurso,
    };
  }

  const asentamiento: Asentamiento = {
    id: `asentamiento-${asentamientosExistentes.length}-${Math.round(posicion.x)}-${Math.round(posicion.y)}`,
    faccionId,
    jugadoresFundadoresIds,
    posicion,
    nivel: 1,
    fundadoEnTick: tickActual,
    radioPotencial: ZONA_INFLUENCIA.radioInicial,
    poblacion: { pesants: POBLACION.pesants.inicial, artesanos: 0, nobleza: 0 },
    almacen: almacenInicial,
    edificios: [],
    cargos: { gobernadorId: null, tesoreroId: null, generalId: null, maestroObrasId: null, sacerdoteId: null },
    casasCompradas: [],
    politicasActivas: [],
    escuadrones: [],
    medidorMantenimiento: MANTENIMIENTO.medidorInicial,
  };

  let faccionActualizada = faccion;
  for (const jugadorId of jugadoresFundadoresIds) {
    faccionActualizada = otorgarCiudadania(faccionActualizada, jugadorId);
  }

  return {
    asentamiento,
    facciones: facciones.map((f) => (f.id === faccionId ? faccionActualizada : f)),
  };
}

/** Avanza el crecimiento de la zona de influencia potencial de cada asentamiento en `deltaTicks` ticks. */
export function avanzarCrecimientoZonas(asentamientos: Asentamiento[], deltaTicks = 1): Asentamiento[] {
  return asentamientos.map((a) => ({
    ...a,
    radioPotencial: Math.min(
      ZONA_INFLUENCIA.radioMaximo,
      a.radioPotencial + ZONA_INFLUENCIA.crecimientoPorTick * deltaTicks
    ),
  }));
}
