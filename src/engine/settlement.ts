import type { Asentamiento, Edificio, Faccion, Point, RecursoAlmacenado } from '../domain/types';
import type { Instante } from '../domain/tiempo';
import { ALMACEN, FUNDACION, MANTENIMIENTO, POBLACION, ZONA_INFLUENCIA } from '../constants';
import type { Mapa } from '../world/mapa';
import { posicionLibreParaFundar } from './zones';
import { sitioEnBarrio } from './construction';
import { calcularCapFundacion, otorgarCiudadania } from './faccion';

export class FundacionInvalidaError extends Error {}

/**
 * Edificios con los que nace todo asentamiento nuevo (Doc 1.3): ya "activo", sin pasar por la cola.
 *
 * Vista de Asentamiento (a petición del usuario): TODOS son internos (`ambito: 'asentamiento'`) y nacen en
 * coordenadas LOCALES del espacio plano — el Centro Urbano en el origen `(0,0)`; Viviendas y Granja usan la
 * MISMA rejilla de celdas y reparto de barrios por dirección (`sitioEnBarrio`, engine/construction.ts) que el
 * crecimiento posterior, para que la ciudad se lea consistente desde el tick 0 (y no en un anillo aparte que
 * ignora las direcciones aleatorias del asentamiento). El punto de fundación en el MAPA GENERAL vive en
 * `asentamiento.posicion`.
 */
function edificiosIniciales(idBase: string): Edificio[] {
  const contexto = { id: idBase, radioPotencial: ZONA_INFLUENCIA.radioInicial };
  const centroUrbano: Edificio = {
    id: `edificio-${idBase}-centro-urbano`,
    tipo: 'centroUrbano',
    posicion: { x: 0, y: 0 },
    estado: 'activo',
    ambito: 'asentamiento',
  };

  const edificios: Edificio[] = [centroUrbano];
  for (let i = 0; i < FUNDACION.viviendasIniciales; i++) {
    // `sitioEnBarrio` siempre encuentra hueco aquí (fundación: solo el Centro Urbano ocupa celda todavía),
    // pero el fallback al origen es defensivo — nunca debe dejar una vivienda sin `Point`.
    const sitio = sitioEnBarrio(contexto, edificios, 'vivienda');
    const posicion = sitio?.punto ?? { x: 0, y: 0 };
    edificios.push({
      id: `edificio-${idBase}-vivienda-inicial-${i}`,
      tipo: 'vivienda',
      posicion,
      estado: 'activo',
      ambito: 'asentamiento',
      rotado: sitio?.rotado,
    });
  }

  const posicionGranja = sitioEnBarrio(contexto, edificios, 'granja')?.punto ?? { x: 0, y: 0 };
  edificios.push({
    id: `edificio-${idBase}-granja-inicial`,
    tipo: 'granja',
    posicion: posicionGranja,
    estado: 'activo',
    ambito: 'asentamiento',
  });

  // Leñera inicial: DEPRECADA (a petición del usuario) — la reserva de materiales iniciales (madera+piedra,
  // ver `almacenInicial` más abajo) ya es suficiente por sí sola para evitar el deadlock de madera (bug #1,
  // `Correcciones_Durante_Desarrollo.md`); esta mitigación extra dejó de ser necesaria.
  return edificios;
}

export interface ViabilidadFundacion {
  /** Radio de la zona con la que nacería el asentamiento — lo devuelve la consulta para que la interfaz
   * pueda dibujar la previsualización sin tener que conocer `ZONA_INFLUENCIA`. */
  radioInicial: number;
  dentroDelMapa: boolean;
  posicionLibre: boolean;
  /** Hay al menos un bosque cuyo borde entra en el radio inicial — condición crítica, ver `evaluarViabilidadFundacion`. */
  bosqueAlcanzable: boolean;
  /** Nodos de recurso que caen dentro del radio inicial, agrupados por tipo. */
  recursosEnRadio: { tipo: string; nodos: number }[];
  /** `false` si el terreno es 'cima' (banda de elevación más alta) o 'agua' — inhabitables. */
  terrenoValido: boolean;
  /** Se puede fundar aquí (lo que valida `fundarAsentamiento`): dentro del mapa, sin solapar otra zona y en
   * terreno habitable (ni 'cima' ni 'agua'). */
  fundable: boolean;
  /** Además de fundable, el emplazamiento es SOSTENIBLE (tiene madera al alcance). */
  recomendable: boolean;
}

/**
 * Evalúa un emplazamiento ANTES de fundar — solo lectura, no altera nada. Existe por un resultado de
 * simulación (ver `Diario_Simulaciones_Batch_500_Transformacion.md` y el análisis del escalón militar): la
 * madera es el recurso maestro del juego temprano — paga Mantenimiento desde el tick 1, y es el costo
 * dominante de Leñera/Granja/Vivienda/Barracón. Fundar sin un bosque al alcance del radio inicial es una
 * sentencia: en 200 runs × 900 ticks, exigir bosque alcanzable bajó el colapso del 70% al 47% y subió la
 * proporción de asentamientos que llegan a tener tropa del 50% al 98%.
 *
 * Deliberadamente NO bloquea la fundación (decisión de diseño confirmada con el usuario): `fundarAsentamiento`
 * sigue aceptando cualquier posición legal. Esto solo alimenta el aviso de la interfaz — el jugador conserva
 * la libertad de fundar en un mal sitio a sabiendas.
 */
export function evaluarViabilidadFundacion(
  mapa: Mapa,
  posicion: Point,
  asentamientosExistentes: Asentamiento[]
): ViabilidadFundacion {
  const enMapa = mapa.dentroDelMapa(posicion);
  const libre = posicionLibreParaFundar(posicion, asentamientosExistentes);
  // Terreno inhabitable: 'cima' (banda de elevación más alta, Fase 0.1) y 'agua'.
  //
  // El agua se añadió el 2026-09-02, al hacerla infranqueable para ejércitos y caravanas: hasta entonces
  // fundar en el mar era solo absurdo, y desde entonces es una TRAMPA — un asentamiento sobre agua queda
  // incomunicado para siempre, sin caravana ni ejército que pueda entrar o salir. Lo destapó el fixture de
  // pruebas, que llevaba fundando en (500,500) de la seed 42 sin que nadie notara que ahí hay mar.
  const terreno = mapa.terrenoEn(posicion);
  const terrenoValido = terreno !== 'cima' && terreno !== 'agua';
  const radio = ZONA_INFLUENCIA.radioInicial;

  // Un bosque es alcanzable si su BORDE entra en el radio inicial, no hace falta que lo esté su centro —
  // criterio único del mapa (`hayBosqueEnRadio`), el mismo que usa la colocación de Leñeras.
  const bosqueAlcanzable = mapa.hayBosqueEnRadio(posicion, radio);

  const porTipo = new Map<string, number>();
  for (const nodo of mapa.nodosEnRadio(posicion, radio)) {
    porTipo.set(nodo.tipo, (porTipo.get(nodo.tipo) ?? 0) + 1);
  }

  const fundable = enMapa && libre && terrenoValido;
  return {
    radioInicial: radio,
    dentroDelMapa: enMapa,
    posicionLibre: libre,
    bosqueAlcanzable,
    recursosEnRadio: [...porTipo.entries()].map(([tipo, nodos]) => ({ tipo, nodos })),
    terrenoValido,
    fundable,
    recomendable: fundable && bosqueAlcanzable,
  };
}

/**
 * Fundación libre — Doc 1.2/1.3. Hasta 5 jugadores pueden fundar juntos; cada uno recibe automáticamente
 * una casa en el asentamiento recién fundado (ocupa cupo de vivienda, Doc 2.5) y, con ella, ciudadanía
 * inmediata de la Facción fundadora. Respeta el cap de fundación por Facción (Doc 1.7): no aplica a
 * conquista/anexión (fuera de alcance aquí), solo a fundación directa.
 */
export function fundarAsentamiento(
  mapa: Mapa,
  facciones: Faccion[],
  faccionId: string,
  posicion: Point,
  jugadoresFundadoresIds: string[],
  asentamientosExistentes: Asentamiento[],
  fundadoEn: Instante
): { asentamiento: Asentamiento; facciones: Faccion[] } {
  if (jugadoresFundadoresIds.length < 1 || jugadoresFundadoresIds.length > FUNDACION.maxJugadoresFundacionGrupal) {
    throw new FundacionInvalidaError(
      `La fundación grupal admite entre 1 y ${FUNDACION.maxJugadoresFundacionGrupal} jugadores.`
    );
  }
  if (!mapa.dentroDelMapa(posicion)) {
    throw new FundacionInvalidaError('La posición cae fuera de los límites del mapa.');
  }
  if (!posicionLibreParaFundar(posicion, asentamientosExistentes)) {
    throw new FundacionInvalidaError('La posición está dentro de una zona de influencia existente.');
  }
  if (mapa.terrenoEn(posicion) === 'cima') {
    throw new FundacionInvalidaError('La posición cae en terreno de cima — inhabitable.');
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
    'armaMadera', 'armaCobre', 'armaBronce', 'armaBronceCalidad',
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
    nivelActual: 1,
    rachaMantenimientoSano: 0,
    fundadoEn,
    radioPotencial: ZONA_INFLUENCIA.radioInicial,
    poblacion: { pesants: POBLACION.pesants.inicial, artesanos: 0, nobleza: 0 },
    almacen: almacenInicial,
    edificios: edificiosIniciales(id),
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
