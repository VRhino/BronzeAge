import type { Asentamiento, Edificio, Faccion, Point, RecursoAlmacenado, World } from '../domain/types';
import { ALMACEN, FUNDACION, MANTENIMIENTO, NIVEL_ASENTAMIENTO, POBLACION, ZONA_INFLUENCIA } from '../constants';
import { posicionLibreParaFundar } from './zones';
import { calcularCapFundacion, otorgarCiudadania } from './faccion';

export class FundacionInvalidaError extends Error {}

function dentroDelMapa(p: Point, world: World): boolean {
  return p.x >= 0 && p.x <= world.config.ancho && p.y >= 0 && p.y <= world.config.alto;
}

/** Reparte `cantidad` puntos en un anillo alrededor de `centro`, a `radio` de distancia. */
function anilloDePosiciones(centro: Point, cantidad: number, radio: number): Point[] {
  return Array.from({ length: cantidad }, (_, i) => {
    const angulo = (i / cantidad) * Math.PI * 2;
    return { x: centro.x + Math.cos(angulo) * radio, y: centro.y + Math.sin(angulo) * radio };
  });
}

/**
 * Mejor casilla de fertilidad en un pequeño radio alrededor del centro (Doc 1.4), para la Granja inicial.
 * Simplificación deliberada respecto a la auto-construcción (`sitioMejorFertilidad` en construction.ts):
 * al fundar todavía no existe zona de influencia recortada contra rivales que consultar, así que basta
 * con muestrear el propio radio inicial (garantizado libre, ya lo validó `posicionLibreParaFundar`).
 */
function mejorPuntoFertilidadCercano(world: World, centro: Point, radio: number, muestras: number): Point {
  let mejor = centro;
  let mejorFertilidad = -1;
  for (let i = 0; i < muestras; i++) {
    const angulo = (i / muestras) * Math.PI * 2;
    const candidato: Point = { x: centro.x + Math.cos(angulo) * radio, y: centro.y + Math.sin(angulo) * radio };
    const fertilidad = world.fertilidadEn(candidato);
    if (fertilidad > mejorFertilidad) {
      mejorFertilidad = fertilidad;
      mejor = candidato;
    }
  }
  return mejor;
}

/**
 * Bosque más cercano dentro de `radio` de `centro` (Doc 1.4), para la Leñera inicial de fundación (rediseño
 * de progreso Fase 0). Misma simplificación deliberada que `mejorPuntoFertilidadCercano`: sin recortar contra
 * zonas rivales, solo distancia — al fundar todavía no hay zona de influencia recortada que consultar.
 */
function bosqueCercano(world: World, centro: Point, radio: number): { posicion: Point; fuenteId: string } | null {
  const candidatos = world.bosques
    .filter((b) => Math.hypot(b.centro.x - centro.x, b.centro.y - centro.y) <= radio + b.radio)
    .sort((a, b) => Math.hypot(a.centro.x - centro.x, a.centro.y - centro.y) - Math.hypot(b.centro.x - centro.x, b.centro.y - centro.y));
  const elegido = candidatos[0];
  return elegido ? { posicion: elegido.centro, fuenteId: elegido.id } : null;
}

/** Edificios con los que nace todo asentamiento nuevo (Doc 1.3): ya "activo", sin pasar por la cola. */
function edificiosIniciales(world: World, centro: Point, idBase: string): Edificio[] {
  const radioAnillo = ZONA_INFLUENCIA.radioInicial * 0.5;
  const viviendas: Edificio[] = anilloDePosiciones(centro, FUNDACION.viviendasIniciales, radioAnillo).map((posicion, i) => ({
    id: `edificio-${idBase}-vivienda-inicial-${i}`,
    tipo: 'vivienda',
    posicion,
    estado: 'activo',
    ticksRestantes: 0,
  }));
  const granja: Edificio = {
    id: `edificio-${idBase}-granja-inicial`,
    tipo: 'granja',
    posicion: mejorPuntoFertilidadCercano(world, centro, radioAnillo, 12),
    estado: 'activo',
    ticksRestantes: 0,
  };
  const centroUrbano: Edificio = {
    id: `edificio-${idBase}-centro-urbano`,
    tipo: 'centroUrbano',
    posicion: centro,
    estado: 'activo',
    ticksRestantes: 0,
  };
  // Leñera inicial condicional (rediseño de progreso Fase 0, Doc 1.3): solo si hay un bosque alcanzable cerca
  // del punto de fundación — reduce el riesgo de déficit de madera en los primeros ticks, sin garantizarlo.
  const bosque = bosqueCercano(world, centro, radioAnillo);
  const lenera: Edificio[] = bosque
    ? [
        {
          id: `edificio-${idBase}-lenera-inicial`,
          tipo: 'lenera',
          posicion: bosque.posicion,
          estado: 'activo',
          ticksRestantes: 0,
          fuenteId: bosque.fuenteId,
        },
      ]
    : [];
  return [centroUrbano, granja, ...lenera, ...viviendas];
}

/**
 * Fundación libre — Doc 1.2/1.3. Hasta 5 jugadores pueden fundar juntos; cada uno recibe automáticamente
 * una casa en el asentamiento recién fundado (ocupa cupo de vivienda, Doc 2.5) y, con ella, ciudadanía
 * inmediata de la Facción fundadora. Respeta el cap de fundación por Facción (Doc 1.7): no aplica a
 * conquista/anexión (fuera de alcance aquí), solo a fundación directa.
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
  for (const tipo of [
    'madera', 'piedra', 'trigo', 'cobre', 'estano', 'oro', 'livestock',
    // Rediseño de progreso (Fase 0, Doc 4.2.1): tipos intermedios de las cadenas de crafting — sembrados en
    // 0 desde el principio para que ya tengan `capacidad` asignada (si no, `agregarRecurso` los crearía con
    // capacidad 0 la primera vez que algo intente producirlos, capando la producción en silencio para siempre).
    'lingoteCobre', 'lingoteEstano', 'lingoteBronce',
    'cuero', 'cueroCurtido', 'cueroCalidad',
    'armaCobre', 'armaBronce', 'armaBronceCalidad',
    'armaduraBasica', 'armaduraIntermedia', 'armaduraBronce',
  ]) {
    almacenInicial[tipo] = {
      cantidad: FUNDACION.materialesIniciales[tipo] ?? 0,
      capacidad: ALMACEN.capacidadInicialPorRecurso,
    };
  }

  const id = `asentamiento-${asentamientosExistentes.length}-${Math.round(posicion.x)}-${Math.round(posicion.y)}`;

  const asentamiento: Asentamiento = {
    id,
    faccionId,
    jugadoresFundadoresIds,
    posicion,
    nivel: 1,
    fundadoEnTick: tickActual,
    radioPotencial: ZONA_INFLUENCIA.radioInicial,
    poblacion: { pesants: POBLACION.pesants.inicial, artesanos: 0, nobleza: 0 },
    almacen: almacenInicial,
    edificios: edificiosIniciales(world, posicion, id),
    cargos: { gobernadorId: null, tesoreroId: null, generalId: null, maestroObrasId: null, sacerdoteId: null },
    casasCompradas: [...jugadoresFundadoresIds],
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

/**
 * Avanza el crecimiento de la zona de influencia potencial de cada asentamiento en `deltaTicks` ticks, hacia
 * el techo del nivel ACTUAL (rediseño de progreso Fase 0, Doc 1.2/4.5) — subir de nivel no salta el radio de
 * golpe, solo levanta el techo hacia el que la zona ya venía creciendo gradualmente.
 */
export function avanzarCrecimientoZonas(asentamientos: Asentamiento[], deltaTicks = 1): Asentamiento[] {
  return asentamientos.map((a) => ({
    ...a,
    radioPotencial: Math.min(
      ZONA_INFLUENCIA.radioMaximoPorNivel[a.nivel] ?? ZONA_INFLUENCIA.radioMaximoPorNivel[NIVEL_ASENTAMIENTO.nivelMaximo]!,
      a.radioPotencial + ZONA_INFLUENCIA.crecimientoPorTick * deltaTicks
    ),
  }));
}
