import type { Asentamiento, Edificio, EdificioTipo, Faccion, Point, World } from '../domain/types';
import { EDIFICIO_CATALOGO, NECESIDADES, POBLACION, SITIO } from '../constants';
import { pointInPolygon } from './zones';
import { capacidadHabitacional, edificiosPorTipoYEstado, hayProyectoPendiente, poblacionTotal } from './asentamientoQuery';
import { agregarRecurso, descontarRecursos, tieneRecursos } from './almacen';
import { factorTiempoConstruccion } from './politicas';

function distancia(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function sitioLibre(p: Point, ocupados: Edificio[]): boolean {
  return ocupados.every((e) => distancia(e.posicion, p) >= SITIO.espacioMinimoEntreEdificios);
}

/** Crecimiento concéntrico desde el centro (Doc 4.2): recorre anillos de radio creciente buscando hueco libre. */
function sitioConcentrico(asentamiento: Asentamiento, zonaPoligono: Point[], ocupados: Edificio[]): Point | null {
  for (let anillo = 0; anillo < SITIO.anillos; anillo++) {
    const radio = ((anillo + 1) / SITIO.anillos) * asentamiento.radioPotencial;
    for (let m = 0; m < SITIO.muestrasPorAnillo; m++) {
      const angulo = (m / SITIO.muestrasPorAnillo) * Math.PI * 2 + anillo * 0.3;
      const candidato: Point = {
        x: asentamiento.posicion.x + Math.cos(angulo) * radio,
        y: asentamiento.posicion.y + Math.sin(angulo) * radio,
      };
      if (pointInPolygon(candidato, zonaPoligono) && sitioLibre(candidato, ocupados)) {
        return candidato;
      }
    }
  }
  return null;
}

/** Granja: mejor casilla disponible según fertilidad del suelo (Doc 1.4/4.2). */
function sitioMejorFertilidad(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  world: World,
  ocupados: Edificio[]
): Point | null {
  let mejor: Point | null = null;
  let mejorFertilidad = -1;
  for (let i = 0; i < SITIO.muestrasFertilidad; i++) {
    const angulo = (i / SITIO.muestrasFertilidad) * Math.PI * 2;
    const radio = (i % 5) / 5 * asentamiento.radioPotencial;
    const candidato: Point = {
      x: asentamiento.posicion.x + Math.cos(angulo) * radio,
      y: asentamiento.posicion.y + Math.sin(angulo) * radio,
    };
    if (!pointInPolygon(candidato, zonaPoligono) || !sitioLibre(candidato, ocupados)) continue;
    const fertilidad = world.fertilidadEn(candidato);
    if (fertilidad > mejorFertilidad) {
      mejorFertilidad = fertilidad;
      mejor = candidato;
    }
  }
  return mejor;
}

/** Cantera/edificio de extracción: junto al nodo de recurso más cercano SIN reclamar ya (Doc 4.2, ej. herrería cerca de mina). */
function sitioCercaDeNodo(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  world: World,
  tipoRecurso: string,
  fuentesExcluidas: Set<string>
): { posicion: Point; fuenteId: string } | null {
  const candidatos = world.recursos
    .filter((n) => n.tipo === tipoRecurso && n.cantidad > 0 && !fuentesExcluidas.has(n.id) && pointInPolygon(n.posicion, zonaPoligono))
    .sort((a, b) => distancia(a.posicion, asentamiento.posicion) - distancia(b.posicion, asentamiento.posicion));
  const elegido = candidatos[0];
  return elegido ? { posicion: elegido.posicion, fuenteId: elegido.id } : null;
}

/** Lenera: junto al bosque más cercano (sin repetir uno ya explotado por otra lenera propia) dentro de la zona. */
function sitioEnBosque(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  world: World,
  fuentesExcluidas: Set<string>
): { posicion: Point; fuenteId: string } | null {
  const candidatos = world.bosques
    .filter((b) => !fuentesExcluidas.has(b.id) && pointInPolygon(b.centro, zonaPoligono))
    .sort((a, b) => distancia(a.centro, asentamiento.posicion) - distancia(b.centro, asentamiento.posicion));
  const elegido = candidatos[0];
  return elegido ? { posicion: elegido.centro, fuenteId: elegido.id } : null;
}

function crearEdificioEnCola(tipo: EdificioTipo, posicion: Point, id: string, fuenteId?: string): Edificio {
  const edificio: Edificio = {
    id,
    tipo,
    posicion,
    estado: 'en_cola',
    ticksRestantes: EDIFICIO_CATALOGO[tipo].tiempoConstruccionTicks,
  };
  return fuenteId ? { ...edificio, fuenteId } : edificio;
}

/**
 * Un extractor (cantera/mina/minaCobre) necesita otra instancia si NINGUNA fuente propia sigue viva (Doc 1.4:
 * escasez real por ubicación — sin esto, agotado el único yacimiento el asentamiento se queda sin ese recurso
 * PARA SIEMPRE) o si el nivel del asentamiento ya soporta más capacidad de extracción de la que tiene activa
 * (el coste de Mantenimiento escala con el nivel, Doc 4.5; la producción no puede quedarse fija mientras tanto).
 */
function necesitaNuevoExtractor(asentamiento: Asentamiento, tipo: EdificioTipo, world: World): boolean {
  const existentes = asentamiento.edificios.filter((e) => e.tipo === tipo);
  const conFuenteViva = existentes.filter((e) => {
    const nodo = world.recursos.find((n) => n.id === e.fuenteId);
    return nodo && nodo.cantidad > 0;
  });
  if (conFuenteViva.length === 0) return true;
  return conFuenteViva.length < asentamiento.nivel;
}

function fuentesReclamadas(asentamiento: Asentamiento, tipo: EdificioTipo): Set<string> {
  return new Set(asentamiento.edificios.filter((e) => e.tipo === tipo && e.fuenteId).map((e) => e.fuenteId!));
}

/** Evalúa déficits reales (Doc 4.2) y encola como máximo un proyecto nuevo por tipo de edificio por tick. */
function evaluarNecesidades(asentamiento: Asentamiento, zonaPoligono: Point[], world: World): Edificio[] {
  const nuevos: Edificio[] = [];
  let contador = asentamiento.edificios.length;
  const nextId = () => `edificio-${asentamiento.id}-${contador++}`;
  const ocupados = () => [...asentamiento.edificios, ...nuevos];

  const capacidadVivienda = capacidadHabitacional(asentamiento);
  const total = poblacionTotal(asentamiento);
  const ocupacion = capacidadVivienda <= 0 ? 1 : total / capacidadVivienda;
  if ((capacidadVivienda === 0 || ocupacion >= NECESIDADES.umbralViviendaOcupada) && !hayProyectoPendiente(asentamiento, 'vivienda')) {
    const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
    if (sitio) nuevos.push(crearEdificioEnCola('vivienda', sitio, nextId()));
  }

  // Granja: al menos una, y más si la reserva de trigo no alcanza para sostener a la población actual (Doc 4.2).
  const granjasActivas = edificiosPorTipoYEstado(asentamiento, 'granja').length;
  const consumoTotal = poblacionTotal(asentamiento) * POBLACION.consumoComidaPorHabitante;
  const trigoDisponible = asentamiento.almacen['trigo']?.cantidad ?? 0;
  const reservaTicks = consumoTotal > 0 ? trigoDisponible / consumoTotal : Number.POSITIVE_INFINITY;
  if ((granjasActivas === 0 || reservaTicks < NECESIDADES.umbralComidaTicksReserva) && !hayProyectoPendiente(asentamiento, 'granja')) {
    const sitio = sitioMejorFertilidad(asentamiento, zonaPoligono, world, ocupados());
    if (sitio) nuevos.push(crearEdificioEnCola('granja', sitio, nextId()));
  }

  if (necesitaNuevoExtractor(asentamiento, 'cantera', world) && !hayProyectoPendiente(asentamiento, 'cantera')) {
    const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, world, 'piedra', fuentesReclamadas(asentamiento, 'cantera'));
    if (sitio) nuevos.push(crearEdificioEnCola('cantera', sitio.posicion, nextId(), sitio.fuenteId));
  }

  // Lenera: escala con el nivel igual que los extractores minerales (los bosques no se agotan, Doc 1.4).
  if (edificiosPorTipoYEstado(asentamiento, 'lenera').length < asentamiento.nivel && !hayProyectoPendiente(asentamiento, 'lenera')) {
    const sitio = sitioEnBosque(asentamiento, zonaPoligono, world, fuentesReclamadas(asentamiento, 'lenera'));
    if (sitio) nuevos.push(crearEdificioEnCola('lenera', sitio.posicion, nextId(), sitio.fuenteId));
  }

  const necesitaAlmacen = Object.values(asentamiento.almacen).some(
    (r) => r.capacidad > 0 && r.cantidad / r.capacidad >= NECESIDADES.umbralAlmacenAmpliacion
  );
  if (necesitaAlmacen && !hayProyectoPendiente(asentamiento, 'almacen')) {
    const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
    if (sitio) nuevos.push(crearEdificioEnCola('almacen', sitio, nextId()));
  }

  if (
    asentamiento.poblacion.pesants >= NECESIDADES.pesantsParaHabilitarTaller &&
    edificiosPorTipoYEstado(asentamiento, 'taller').length === 0 &&
    !hayProyectoPendiente(asentamiento, 'taller')
  ) {
    const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
    if (sitio) nuevos.push(crearEdificioEnCola('taller', sitio, nextId()));
  }

  // Mina de oro (Doc 3.1): igual que la cantera, junto al nodo más cercano dentro de la zona.
  if (necesitaNuevoExtractor(asentamiento, 'mina', world) && !hayProyectoPendiente(asentamiento, 'mina')) {
    const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, world, 'oro', fuentesReclamadas(asentamiento, 'mina'));
    if (sitio) nuevos.push(crearEdificioEnCola('mina', sitio.posicion, nextId(), sitio.fuenteId));
  }

  // Mina de cobre (Doc 1.1/5.7): mismo patrón, necesaria para reclutamiento militar (Sprint 5).
  if (necesitaNuevoExtractor(asentamiento, 'minaCobre', world) && !hayProyectoPendiente(asentamiento, 'minaCobre')) {
    const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, world, 'cobre', fuentesReclamadas(asentamiento, 'minaCobre'));
    if (sitio) nuevos.push(crearEdificioEnCola('minaCobre', sitio.posicion, nextId(), sitio.fuenteId));
  }

  return nuevos;
}

/**
 * Progresa colas/construcción/producción de un tick y evalúa nuevas necesidades.
 * NOTA: los nodos de recurso del `world` se agotan mutando `cantidad` in-place (simplificación deliberada de Fase 0
 * para evitar clonar cientos de nodos cada tick); el resto del estado se trata de forma inmutable.
 */
export function avanzarConstruccion(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  world: World
): { asentamiento: Asentamiento; eventos: string[] } {
  const eventos: string[] = [];
  let almacen = asentamiento.almacen;
  const edificiosActualizados: Edificio[] = [];

  const activos = edificiosPorTipoYEstado(asentamiento, 'granja').concat(
    edificiosPorTipoYEstado(asentamiento, 'cantera'),
    edificiosPorTipoYEstado(asentamiento, 'lenera'),
    edificiosPorTipoYEstado(asentamiento, 'mina'),
    edificiosPorTipoYEstado(asentamiento, 'minaCobre')
  );
  const trabajadoresRequeridos = activos.reduce((acc, e) => acc + (EDIFICIO_CATALOGO[e.tipo] as { trabajadoresRequeridos?: number }).trabajadoresRequeridos! , 0);
  const ratioMano = trabajadoresRequeridos <= 0 ? 1 : Math.min(1, asentamiento.poblacion.pesants / trabajadoresRequeridos);

  for (const edificio of asentamiento.edificios) {
    if (edificio.estado === 'en_construccion') {
      const restantes = edificio.ticksRestantes - 1;
      if (restantes <= 0) {
        eventos.push(`${edificio.tipo} completado.`);
        if (edificio.tipo === 'almacen') {
          const bonus = EDIFICIO_CATALOGO.almacen.capacidadPorRecursoAdicional;
          for (const recurso of Object.keys(almacen)) {
            almacen = { ...almacen, [recurso]: { ...almacen[recurso]!, capacidad: almacen[recurso]!.capacidad + bonus } };
          }
        }
        edificiosActualizados.push({ ...edificio, estado: 'activo', ticksRestantes: 0 });
      } else {
        edificiosActualizados.push({ ...edificio, ticksRestantes: restantes });
      }
      continue;
    }

    if (edificio.estado === 'en_cola') {
      const costo = EDIFICIO_CATALOGO[edificio.tipo].costo as Partial<Record<string, number>>;
      if (tieneRecursos(almacen, costo)) {
        almacen = descontarRecursos(almacen, costo);
        eventos.push(`Comienza construcción de ${edificio.tipo}.`);
        // Vía Rápida de Construcción (Maestro de Obras, Doc 2.2/4.4) acelera el tiempo restante al arrancar.
        const ticks = Math.max(1, Math.round(EDIFICIO_CATALOGO[edificio.tipo].tiempoConstruccionTicks * factorTiempoConstruccion(asentamiento)));
        edificiosActualizados.push({ ...edificio, estado: 'en_construccion', ticksRestantes: ticks });
      } else {
        edificiosActualizados.push(edificio);
      }
      continue;
    }

    // activo: producción
    if (edificio.tipo === 'granja') {
      const yieldTrigo = EDIFICIO_CATALOGO.granja.produccionBaseTrigo * world.fertilidadEn(edificio.posicion) * ratioMano;
      almacen = agregarRecurso(almacen, 'trigo', yieldTrigo);
    } else if (edificio.tipo === 'cantera') {
      const nodo = world.recursos.find((n) => n.id === edificio.fuenteId);
      if (nodo && nodo.cantidad > 0) {
        const extraido = Math.min(EDIFICIO_CATALOGO.cantera.produccionBasePiedra * ratioMano, nodo.cantidad);
        nodo.cantidad -= extraido;
        almacen = agregarRecurso(almacen, 'piedra', extraido);
        if (nodo.cantidad <= 0) eventos.push('El yacimiento de piedra de la cantera se ha agotado.');
      }
    } else if (edificio.tipo === 'lenera') {
      const bosque = world.bosques.find((b) => b.id === edificio.fuenteId);
      if (bosque) {
        const yieldMadera = EDIFICIO_CATALOGO.lenera.produccionBaseMadera * bosque.densidad * ratioMano;
        almacen = agregarRecurso(almacen, 'madera', yieldMadera);
      }
    } else if (edificio.tipo === 'mina') {
      const nodo = world.recursos.find((n) => n.id === edificio.fuenteId);
      if (nodo && nodo.cantidad > 0) {
        const extraido = Math.min(EDIFICIO_CATALOGO.mina.produccionBaseOro * ratioMano, nodo.cantidad);
        nodo.cantidad -= extraido;
        almacen = agregarRecurso(almacen, 'oro', extraido);
        if (nodo.cantidad <= 0) eventos.push('El yacimiento de oro de la mina se ha agotado.');
      }
    } else if (edificio.tipo === 'minaCobre') {
      const nodo = world.recursos.find((n) => n.id === edificio.fuenteId);
      if (nodo && nodo.cantidad > 0) {
        const extraido = Math.min(EDIFICIO_CATALOGO.minaCobre.produccionBaseCobre * ratioMano, nodo.cantidad);
        nodo.cantidad -= extraido;
        almacen = agregarRecurso(almacen, 'cobre', extraido);
        if (nodo.cantidad <= 0) eventos.push('El yacimiento de cobre se ha agotado.');
      }
    }
    edificiosActualizados.push(edificio);
  }

  const asentamientoConProgreso: Asentamiento = { ...asentamiento, almacen, edificios: edificiosActualizados };
  const nuevosProyectos = evaluarNecesidades(asentamientoConProgreso, zonaPoligono, world);
  if (nuevosProyectos.length > 0) {
    for (const p of nuevosProyectos) eventos.push(`Nueva necesidad detectada: se encola ${p.tipo}.`);
  }

  return {
    asentamiento: { ...asentamientoConProgreso, edificios: [...edificiosActualizados, ...nuevosProyectos] },
    eventos,
  };
}

export class ConstruccionManualInvalidaError extends Error {}

/**
 * Fundición/Gran Fundición (Doc 4.2/5.7): a diferencia del resto de edificios, son de colocación MANUAL —
 * decisión militar deliberada del jugador, no auto-construcción por necesidad. El sitio sigue eligiéndose
 * solo (Doc 4.2: el jugador no elige ubicación salvo fundación y edificios estratégicos).
 */
export function construirManualmente(
  asentamiento: Asentamiento,
  faccion: Faccion,
  zonaPoligono: Point[],
  tipo: 'fundicion' | 'granFundicion',
  contador = 0
): Asentamiento {
  if (edificiosPorTipoYEstado(asentamiento, tipo).length > 0 || hayProyectoPendiente(asentamiento, tipo)) {
    throw new ConstruccionManualInvalidaError(`Ya existe (o está en curso) una ${tipo} en este asentamiento.`);
  }
  if (tipo === 'granFundicion' && faccion.nivel < EDIFICIO_CATALOGO.granFundicion.nivelFaccionMinimo) {
    throw new ConstruccionManualInvalidaError(
      `Requiere nivel de Facción ${EDIFICIO_CATALOGO.granFundicion.nivelFaccionMinimo} (actual: ${faccion.nivel}).`
    );
  }
  const sitio = sitioConcentrico(asentamiento, zonaPoligono, asentamiento.edificios);
  if (!sitio) throw new ConstruccionManualInvalidaError('No hay sitio disponible dentro de la zona de influencia.');

  const nuevo = crearEdificioEnCola(tipo, sitio, `edificio-${asentamiento.id}-manual-${contador}`);
  return { ...asentamiento, edificios: [...asentamiento.edificios, nuevo] };
}
