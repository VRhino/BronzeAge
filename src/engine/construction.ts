import type { Asentamiento, Edificio, EdificioTipo, Faccion, Point, RecursoAlmacenado, World } from '../domain/types';
import { EDIFICIO_CATALOGO, EXTRACCION_MAXIMOS, NECESIDADES, RESERVA_CONSTRUCCION, SITIO } from '../constants';
import { pointInPolygon } from './zones';
import {
  capacidadHabitacional,
  edificiosPorTipoYEstado,
  hayProyectoPendiente,
  poblacionTotal,
  ratioManoObra,
  ratioManoObraArtesanos,
} from './asentamientoQuery';
import { agregarRecurso, descontarRecursos, tieneRecursos } from './almacen';
import { recursosProtegidosPorMantenimiento } from './mantenimiento';
import { factorProduccionTrigo, factorTiempoConstruccion, minimoLenerasPrioritario, politicaActivaDesbloqueaEdificio } from './politicas';
import { consumoComidaPoblacion } from './population';
import { consumoRacionTropas } from './tropas';

/**
 * Recurso propio de cada tipo de edificio "de supervivencia": Granja no respeta la reserva mínima de
 * trigo, ni Leñera la de madera (ver `RESERVA_CONSTRUCCION` en constants.ts) — son la única vía real de
 * recuperar esos recursos, así que bloquearlas por la misma escasez que deben resolver sería un
 * huevo-y-la-gallina sin salida.
 */
const RECURSO_PROPIO: Partial<Record<EdificioTipo, string>> = { granja: 'trigo', lenera: 'madera' };

/**
 * Un edificio en cola solo puede empezar a construirse si, además de poder pagar el costo completo,
 * no deja ningún recurso protegido por Mantenimiento (ver `recursosProtegidosPorMantenimiento`) por debajo
 * de su reserva mínima — salvo el recurso que el propio edificio produce (ver `RECURSO_PROPIO`).
 */
function puedeIniciarConstruccion(
  almacen: Record<string, RecursoAlmacenado>,
  costo: Partial<Record<string, number>>,
  tipo: EdificioTipo,
  nivel: number
): boolean {
  if (!tieneRecursos(almacen, costo)) return false;
  const protegidos = recursosProtegidosPorMantenimiento(nivel);
  const exento = RECURSO_PROPIO[tipo];
  return Object.entries(costo).every(([recurso, cantidad]) => {
    if (recurso === exento || !protegidos.includes(recurso as (typeof protegidos)[number])) return true;
    const reserva = (RESERVA_CONSTRUCCION as Record<string, number>)[recurso] ?? 0;
    const disponible = almacen[recurso]?.cantidad ?? 0;
    return disponible - (cantidad ?? 0) >= reserva;
  });
}

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
 * Un extractor (cantera/mina/minaCobre/minaEstano/corral) necesita otra instancia si NINGUNA fuente propia
 * sigue viva (Doc 1.4: escasez real por ubicación — sin esto, agotado el único yacimiento el asentamiento se
 * queda sin ese recurso PARA SIEMPRE) o si el asentamiento todavía no llegó al tope de extractores del tipo
 * (rediseño de progreso Fase 0: `EXTRACCION_MAXIMOS.porTipo`, desacoplado del nivel de asentamiento — antes
 * escalaba 1:1 con él, pero con el tope de nivel bajando a 3 se quedaría corto).
 */
function necesitaNuevoExtractor(asentamiento: Asentamiento, tipo: EdificioTipo, world: World): boolean {
  const existentes = asentamiento.edificios.filter((e) => e.tipo === tipo);
  const conFuenteViva = existentes.filter((e) => {
    const nodo = world.recursos.find((n) => n.id === e.fuenteId);
    return nodo && nodo.cantidad > 0;
  });
  if (conFuenteViva.length === 0) return true;
  return conFuenteViva.length < EXTRACCION_MAXIMOS.porTipo;
}

function fuentesReclamadas(asentamiento: Asentamiento, tipo: EdificioTipo): Set<string> {
  return new Set(asentamiento.edificios.filter((e) => e.tipo === tipo && e.fuenteId).map((e) => e.fuenteId!));
}

/** Tipos cuya producción es "de supervivencia": entrada de la que dependen el resto de construcciones
 * (madera) y el Mantenimiento (madera + trigo), o la comida de la población. Ver `slotsReservadosSupervivencia`. */
const TIPOS_SUPERVIVENCIA = new Set<EdificioTipo>(['granja', 'lenera']);

/** Extractores base cuyo recurso (piedra/cobre/oro/estaño/livestock) es insumo de otros edificios (Doc 4.2.1,
 * rediseño de progreso Fase 0) — ver `slotsReservadosExtractores`. */
const TIPOS_EXTRACTORES_BASE = new Set<EdificioTipo>(['cantera', 'minaCobre', 'mina', 'minaEstano', 'corral']);

function intentarSitioBosque(asentamiento: Asentamiento, zonaPoligono: Point[], world: World, nextId: () => string): Edificio | null {
  if (hayProyectoPendiente(asentamiento, 'lenera')) return null;
  const sitio = sitioEnBosque(asentamiento, zonaPoligono, world, fuentesReclamadas(asentamiento, 'lenera'));
  return sitio ? crearEdificioEnCola('lenera', sitio.posicion, nextId(), sitio.fuenteId) : null;
}

/**
 * Evalúa déficits reales (Doc 4.2) y encola como máximo un proyecto nuevo por tipo de edificio por tick,
 * respetando el cupo global de `NECESIDADES.maximoEnCola` edificios `en_cola` simultáneos.
 *
 * Reparto de la cola en tres cupos (rebalance — antes Vivienda/Almacén/Taller podían copar los slots con
 * proyectos atascados por falta de recursos y dejar a Granja/Leñera sin hueco para encolarse NUNCA, un
 * interbloqueo real: sin Leñera nueva entrando en juego, el déficit de madera no se corregía y el
 * asentamiento caía en ruinas por Mantenimiento impago; el mismo patrón reapareció con el rediseño de
 * progreso Fase 0 — Curtiduría/Armería podían copar los slots generales esperando piedra, sin dejarle nunca
 * un hueco a Cantera, su única fuente):
 * - `slotsReservadosSupervivencia` solo lo pueden usar Granja/Leñera (recursos de los que depende TODO lo
 *   demás: madera para construir y para Mantenimiento, trigo para no morir de hambre).
 * - `slotsReservadosExtractores` solo lo pueden usar Cantera/MinaCobre/Mina/MinaEstaño/Corral (sus recursos
 *   son insumo de Vivienda/Almacén/Curtiduría/Armería, Doc 4.2.1).
 * - El resto del cupo es de libre concurrencia entre cualquier tipo, incluidos los anteriores si su
 *   reservado ya está ocupado.
 *
 * Orden de evaluación: primero los recursos de supervivencia (Granja, Leñera), luego los extractores base,
 * luego los edificios de transformación (Curtiduría/Armería/Fundición/Carpintería), y al final los edificios
 * de crecimiento/lujo (Vivienda, Almacén) — antes Vivienda/Almacén iban primero y eran los que más a menudo
 * acababan copando la cola.
 */
function evaluarNecesidades(asentamiento: Asentamiento, zonaPoligono: Point[], world: World): Edificio[] {
  const nuevos: Edificio[] = [];
  let contador = asentamiento.edificios.length;
  const nextId = () => `edificio-${asentamiento.id}-${contador++}`;
  const ocupados = () => [...asentamiento.edificios, ...nuevos];

  const enCola = asentamiento.edificios.filter((e) => e.estado === 'en_cola');
  const enColaSupervivencia = enCola.filter((e) => TIPOS_SUPERVIVENCIA.has(e.tipo)).length;
  const enColaExtractores = enCola.filter((e) => TIPOS_EXTRACTORES_BASE.has(e.tipo)).length;
  const enColaGeneral = enCola.length - enColaSupervivencia - enColaExtractores;
  let espacioReservado = Math.max(0, NECESIDADES.slotsReservadosSupervivencia - enColaSupervivencia);
  let espacioExtractores = Math.max(0, NECESIDADES.slotsReservadosExtractores - enColaExtractores);
  let espacioGeneral = Math.max(
    0,
    NECESIDADES.maximoEnCola - NECESIDADES.slotsReservadosSupervivencia - NECESIDADES.slotsReservadosExtractores - enColaGeneral
  );

  const encolar = (edificio: Edificio | null): void => {
    if (!edificio) return;
    if (TIPOS_SUPERVIVENCIA.has(edificio.tipo) && espacioReservado > 0) {
      nuevos.push(edificio);
      espacioReservado -= 1;
      return;
    }
    if (TIPOS_EXTRACTORES_BASE.has(edificio.tipo) && espacioExtractores > 0) {
      nuevos.push(edificio);
      espacioExtractores -= 1;
      return;
    }
    if (espacioGeneral > 0) {
      nuevos.push(edificio);
      espacioGeneral -= 1;
    }
  };

  // Protección de Riesgos (política de Maestro de Obras): mientras esté activa y no se llegue al mínimo de
  // Leñeras (activas + en curso/cola), SOLO se evalúa esa necesidad — se ignora cualquier otra este tick.
  // Excepción: si no hay NINGÚN bosque libre en la zona (`sitioEnBosque` no encuentra sitio) y tampoco hay
  // ya una Leñera en camino, bloquear igual sería un interbloqueo sin salida (0 progreso posible durante
  // toda la duración de la política) — en ese caso se deja pasar la evaluación normal de abajo.
  const objetivoLenerasPrioritario = minimoLenerasPrioritario(asentamiento);
  if (objetivoLenerasPrioritario > 0) {
    const lenerasActivas = edificiosPorTipoYEstado(asentamiento, 'lenera').length;
    const lenerasPendientes = asentamiento.edificios.filter((e) => e.tipo === 'lenera' && e.estado !== 'activo').length;
    if (lenerasActivas + lenerasPendientes < objetivoLenerasPrioritario) {
      if (hayProyectoPendiente(asentamiento, 'lenera')) {
        return nuevos; // ya hay una Leñera en camino hacia el objetivo: seguimos bloqueando el resto.
      }
      const sitio = sitioEnBosque(asentamiento, zonaPoligono, world, fuentesReclamadas(asentamiento, 'lenera'));
      if (sitio) {
        encolar(crearEdificioEnCola('lenera', sitio.posicion, nextId(), sitio.fuenteId));
        return nuevos;
      }
      // Sin bosque disponible todavía: no bloquear el resto — se retomará la prioridad en cuanto la
      // zona de influencia crezca lo suficiente para alcanzar un bosque.
    }
  }

  // Granja: al menos una, y más si estoy en DÉFICIT de trigo — producción actual de todas las Granjas activas
  // (fertilidad + mano de obra + Edicto de Cosecha, mismo cálculo que la producción real, ver más abajo) por
  // debajo del consumo actual (población + tropas, Doc 4.1/5.4) — no "cuántos ticks de reserva quedan al
  // ritmo de hoy" (esa estimación era optimista: el consumo sigue creciendo con la población mientras se
  // construye, y no contaba las raciones de tropas).
  const granjasActivasEdificios = edificiosPorTipoYEstado(asentamiento, 'granja');
  const ratioManoActual = ratioManoObra(asentamiento);
  const factorTrigoActual = factorProduccionTrigo(asentamiento);
  const produccionTrigoActual = granjasActivasEdificios.reduce(
    (acc, e) => acc + EDIFICIO_CATALOGO.granja.produccionBaseTrigo * world.fertilidadEn(e.posicion) * ratioManoActual * factorTrigoActual,
    0
  );
  const consumoTrigoActual = consumoComidaPoblacion(asentamiento) + consumoRacionTropas(asentamiento);
  const enDeficitTrigo = produccionTrigoActual < consumoTrigoActual;
  // En déficit se permite tener varias Granjas en camino a la vez (hasta el tope), no solo una: sin esto, un
  // déficit severo solo podía corregirse construyendo Granjas en SERIE (una cada 6 ticks), quedándose muy por
  // detrás de la necesidad real.
  const granjasPendientes = asentamiento.edificios.filter((e) => e.tipo === 'granja' && e.estado !== 'activo').length;
  const limiteGranjasPendientes = enDeficitTrigo ? NECESIDADES.maximoGranjasPendientesEnDeficit : 1;
  if ((granjasActivasEdificios.length === 0 || enDeficitTrigo) && granjasPendientes < limiteGranjasPendientes) {
    const sitio = sitioMejorFertilidad(asentamiento, zonaPoligono, world, ocupados());
    if (sitio) encolar(crearEdificioEnCola('granja', sitio, nextId()));
  }

  // Lenera: mismo tope que los extractores minerales (los bosques no se agotan, Doc 1.4).
  if (edificiosPorTipoYEstado(asentamiento, 'lenera').length < EXTRACCION_MAXIMOS.porTipo) {
    encolar(intentarSitioBosque(asentamiento, zonaPoligono, world, nextId));
  }

  if (necesitaNuevoExtractor(asentamiento, 'cantera', world) && !hayProyectoPendiente(asentamiento, 'cantera')) {
    const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, world, 'piedra', fuentesReclamadas(asentamiento, 'cantera'));
    if (sitio) encolar(crearEdificioEnCola('cantera', sitio.posicion, nextId(), sitio.fuenteId));
  }

  // Corral (Doc 4.2.1, rediseño de progreso Fase 0): extractor de livestock, mismo patrón que cantera/minas.
  if (necesitaNuevoExtractor(asentamiento, 'corral', world) && !hayProyectoPendiente(asentamiento, 'corral')) {
    const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, world, 'livestock', fuentesReclamadas(asentamiento, 'corral'));
    if (sitio) encolar(crearEdificioEnCola('corral', sitio.posicion, nextId(), sitio.fuenteId));
  }

  // Mina de cobre (Doc 1.1/5.7): mismo patrón, necesaria para reclutamiento militar (Sprint 5).
  if (necesitaNuevoExtractor(asentamiento, 'minaCobre', world) && !hayProyectoPendiente(asentamiento, 'minaCobre')) {
    const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, world, 'cobre', fuentesReclamadas(asentamiento, 'minaCobre'));
    if (sitio) encolar(crearEdificioEnCola('minaCobre', sitio.posicion, nextId(), sitio.fuenteId));
  }

  // Mina de oro (Doc 3.1): igual que la cantera, junto al nodo más cercano dentro de la zona.
  if (necesitaNuevoExtractor(asentamiento, 'mina', world) && !hayProyectoPendiente(asentamiento, 'mina')) {
    const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, world, 'oro', fuentesReclamadas(asentamiento, 'mina'));
    if (sitio) encolar(crearEdificioEnCola('mina', sitio.posicion, nextId(), sitio.fuenteId));
  }

  // Mina de estaño (Doc 1.1/5.7): mismo patrón; sin nodo de estaño en la zona simplemente no se completa.
  if (necesitaNuevoExtractor(asentamiento, 'minaEstano', world) && !hayProyectoPendiente(asentamiento, 'minaEstano')) {
    const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, world, 'estano', fuentesReclamadas(asentamiento, 'minaEstano'));
    if (sitio) encolar(crearEdificioEnCola('minaEstano', sitio.posicion, nextId(), sitio.fuenteId));
  }

  const capacidadVivienda = capacidadHabitacional(asentamiento);
  const total = poblacionTotal(asentamiento);
  const ocupacion = capacidadVivienda <= 0 ? 1 : total / capacidadVivienda;
  if ((capacidadVivienda === 0 || ocupacion >= NECESIDADES.umbralViviendaOcupada) && !hayProyectoPendiente(asentamiento, 'vivienda')) {
    const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
    if (sitio) encolar(crearEdificioEnCola('vivienda', sitio, nextId()));
  }

  const necesitaAlmacen = Object.values(asentamiento.almacen).some(
    (r) => r.capacidad > 0 && r.cantidad / r.capacidad >= NECESIDADES.umbralAlmacenAmpliacion
  );
  if (necesitaAlmacen && !hayProyectoPendiente(asentamiento, 'almacen')) {
    const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
    if (sitio) encolar(crearEdificioEnCola('almacen', sitio, nextId()));
  }

  // Edificios de transformación (Doc 4.2.1, rediseño de progreso Fase 0): reemplazan al antiguo Taller
  // genérico. Curtiduría/Armería/Fundición no tienen gate de nivel para su construcción BASE — solo sus
  // mejoras de nivel interno lo exigen (ver `avanzarMejoras` más abajo) — porque el propio gate de nivel 2 del
  // asentamiento exige tenerlas construidas, así que tienen que poder construirse desde el principio.
  // Se evalúan DESPUÉS de Vivienda/Almacén y como máximo UNA de las tres puede estar EN COLA A LA VEZ (no solo
  // "una nueva por tick" — bug detectado en simulación: con solo eso, en 2 ticks igual se acumulaban 2-3
  // atascadas esperando piedra en un punto de fundación pobre en ese recurso, copando los slots generales y
  // dejando a Vivienda sin hueco para siempre). Mismo espíritu que el resto de protecciones de esta función.
  const transformacionEnCurso = (['curtiduria', 'armeria', 'fundicion'] as const).some(
    (tipo) => hayProyectoPendiente(asentamiento, tipo)
  );
  if (!transformacionEnCurso) {
    for (const tipo of ['curtiduria', 'armeria', 'fundicion'] as const) {
      if (edificiosPorTipoYEstado(asentamiento, tipo).length === 0) {
        const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
        if (sitio) {
          encolar(crearEdificioEnCola(tipo, sitio, nextId()));
          break;
        }
      }
    }
  }

  // Carpintería: a diferencia de los otros 3, su construcción BASE sí exige nivel de asentamiento
  // (`requisitoNivelAsentamientoConstruccion`) — habilita Armería/Barracón/Galería de tiro nivel 2.
  const requisitoCarpinteria = (EDIFICIO_CATALOGO.carpinteria as { requisitoNivelAsentamientoConstruccion?: number }).requisitoNivelAsentamientoConstruccion ?? 0;
  if (
    asentamiento.nivel >= requisitoCarpinteria &&
    edificiosPorTipoYEstado(asentamiento, 'carpinteria').length === 0 &&
    !hayProyectoPendiente(asentamiento, 'carpinteria')
  ) {
    const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
    if (sitio) encolar(crearEdificioEnCola('carpinteria', sitio, nextId()));
  }

  return nuevos;
}

/**
 * Edificios especiales vía política (Doc 4.4/4.2.1, rediseño de progreso Fase 0): Barracón, Galería de tiro y
 * Palacio NO son auto-construcción — solo se encolan mientras la política de desbloqueo correspondiente esté
 * activa (ver `politicaActivaDesbloqueaEdificio`) y se cumpla su gate de nivel/edificio previo. Van en un
 * CLUSTER DE COLA APARTE: no cuentan contra `NECESIDADES.maximoEnCola` ni compiten con `evaluarNecesidades`.
 */
function evaluarEdificiosEspeciales(asentamiento: Asentamiento, zonaPoligono: Point[]): Edificio[] {
  const nuevos: Edificio[] = [];
  let contador = asentamiento.edificios.length + 1000; // rango separado para no colisionar con evaluarNecesidades
  const nextId = () => `edificio-${asentamiento.id}-especial-${contador++}`;
  const ocupados = () => [...asentamiento.edificios, ...nuevos];

  const candidatos: { tipo: 'barracon' | 'galeriaDeTiro' | 'palacio'; requisitoNivel: number }[] = [
    { tipo: 'barracon', requisitoNivel: 0 },
    { tipo: 'galeriaDeTiro', requisitoNivel: 0 },
    { tipo: 'palacio', requisitoNivel: (EDIFICIO_CATALOGO.palacio as { requisitoNivelAsentamientoConstruccion?: number }).requisitoNivelAsentamientoConstruccion ?? 0 },
  ];

  for (const { tipo, requisitoNivel } of candidatos) {
    if (!politicaActivaDesbloqueaEdificio(asentamiento, tipo)) continue;
    if (asentamiento.nivel < requisitoNivel) continue;
    if (edificiosPorTipoYEstado(asentamiento, tipo).length > 0 || hayProyectoPendiente(asentamiento, tipo)) continue;
    const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
    if (sitio) nuevos.push(crearEdificioEnCola(tipo, sitio, nextId()));
  }

  return nuevos;
}

/** Tipos de edificio de transformación con tiers (Doc 4.2.1): mejoran de nivelInterno y ejecutan recetas. */
const EDIFICIOS_CON_NIVELES = ['fundicion', 'curtiduria', 'armeria', 'carpinteria', 'barracon', 'galeriaDeTiro'] as const;

function nivelesDe(tipo: EdificioTipo): Record<number, { trabajadoresRequeridos: number; recetas: { produce: string; produccionBase: number; consumePorUnidad: Partial<Record<string, number>> }[]; costoMejora?: Partial<Record<string, number>>; requisitoNivelAsentamiento?: number; requiereEdificio?: string; requiereEdificioNivel?: number }> | undefined {
  return (EDIFICIO_CATALOGO[tipo] as { niveles?: Record<number, any> }).niveles;
}

/**
 * Mejora de nivel interno de un edificio de transformación activo (Doc 4.2.1, rediseño de progreso Fase 0):
 * instantánea — si se cumple el gate del siguiente nivel (nivel de asentamiento + edificio previo, si aplica)
 * y hay fondos para `costoMejora` (respetando la misma reserva mínima que protege el inicio de construcción),
 * se paga y sube `nivelInterno` en el mismo tick. No hay tiempo de mejora especificado en el diseño original.
 */
function avanzarMejoras(asentamiento: Asentamiento, almacen: Record<string, RecursoAlmacenado>): { asentamiento: Asentamiento; almacen: Record<string, RecursoAlmacenado>; eventos: string[] } {
  const eventos: string[] = [];
  let almacenActual = almacen;
  const edificios = asentamiento.edificios.map((edificio) => {
    if (edificio.estado !== 'activo' || !(EDIFICIOS_CON_NIVELES as readonly string[]).includes(edificio.tipo)) return edificio;
    const niveles = nivelesDe(edificio.tipo);
    if (!niveles) return edificio;
    const nivelActual = edificio.nivelInterno ?? 1;
    const siguiente = niveles[nivelActual + 1];
    if (!siguiente) return edificio;
    if (siguiente.requisitoNivelAsentamiento && asentamiento.nivel < siguiente.requisitoNivelAsentamiento) return edificio;
    if (siguiente.requiereEdificio) {
      const previo = edificiosPorTipoYEstado(asentamiento, siguiente.requiereEdificio as EdificioTipo);
      if (previo.length === 0) return edificio;
      if (siguiente.requiereEdificioNivel && (previo[0]!.nivelInterno ?? 1) < siguiente.requiereEdificioNivel) return edificio;
    }
    const costo = siguiente.costoMejora ?? {};
    if (!puedeIniciarConstruccion(almacenActual, costo, edificio.tipo, asentamiento.nivel)) return edificio;
    almacenActual = descontarRecursos(almacenActual, costo);
    eventos.push(`${edificio.tipo} mejora a nivel interno ${nivelActual + 1}.`);
    return { ...edificio, nivelInterno: nivelActual + 1 };
  });
  return { asentamiento: { ...asentamiento, edificios }, almacen: almacenActual, eventos };
}

/**
 * Recetas de crafting de los edificios de transformación activos (Doc 4.2.1, rediseño de progreso Fase 0):
 * recorre las recetas del `nivelInterno` actual EN ORDEN (permite que una receta consuma el output de otra
 * del mismo tick, ej. Lingote de Bronce consumiendo Lingote de Cobre/Estaño recién producidos). La producción
 * real se limita por `min(produccionBase * ratioManoObraArtesanos, insumo_disponible / consumePorUnidad)`,
 * mismo criterio que ya usan los extractores minerales contra `nodo.cantidad`.
 */
function avanzarRecetas(asentamiento: Asentamiento, almacen: Record<string, RecursoAlmacenado>): Record<string, RecursoAlmacenado> {
  const ratioArtesano = ratioManoObraArtesanos(asentamiento);
  let almacenActual = almacen;
  for (const edificio of asentamiento.edificios) {
    if (edificio.estado !== 'activo') continue;
    const niveles = nivelesDe(edificio.tipo);
    if (!niveles) continue;
    const nivel = niveles[edificio.nivelInterno ?? 1];
    if (!nivel) continue;
    for (const receta of nivel.recetas) {
      let cantidad = receta.produccionBase * ratioArtesano;
      for (const [insumo, porUnidad] of Object.entries(receta.consumePorUnidad)) {
        if (!porUnidad) continue;
        const disponible = almacenActual[insumo]?.cantidad ?? 0;
        cantidad = Math.min(cantidad, disponible / porUnidad);
      }
      if (cantidad <= 0) continue;
      const consumo: Partial<Record<string, number>> = {};
      for (const [insumo, porUnidad] of Object.entries(receta.consumePorUnidad)) {
        if (porUnidad) consumo[insumo] = porUnidad * cantidad;
      }
      almacenActual = descontarRecursos(almacenActual, consumo);
      almacenActual = agregarRecurso(almacenActual, receta.produce, cantidad);
    }
  }
  return almacenActual;
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
  const resultados = new Map<string, Edificio>();

  const ratioMano = ratioManoObra(asentamiento);

  // Paso 1 (orden original): progreso de construcciones en curso + producción de edificios activos. Los
  // `en_cola` se resuelven en un segundo paso por PRIORIDAD (ver abajo), no aquí.
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
        resultados.set(edificio.id, { ...edificio, estado: 'activo', ticksRestantes: 0 });
      } else {
        resultados.set(edificio.id, { ...edificio, ticksRestantes: restantes });
      }
      continue;
    }

    if (edificio.estado === 'en_cola') continue;

    // activo: producción
    if (edificio.tipo === 'granja') {
      const yieldTrigo = EDIFICIO_CATALOGO.granja.produccionBaseTrigo * world.fertilidadEn(edificio.posicion) * ratioMano * factorProduccionTrigo(asentamiento);
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
    } else if (edificio.tipo === 'minaEstano') {
      const nodo = world.recursos.find((n) => n.id === edificio.fuenteId);
      if (nodo && nodo.cantidad > 0) {
        const extraido = Math.min(EDIFICIO_CATALOGO.minaEstano.produccionBaseEstano * ratioMano, nodo.cantidad);
        nodo.cantidad -= extraido;
        almacen = agregarRecurso(almacen, 'estano', extraido);
        if (nodo.cantidad <= 0) eventos.push('El yacimiento de estaño se ha agotado.');
      }
    } else if (edificio.tipo === 'corral') {
      const nodo = world.recursos.find((n) => n.id === edificio.fuenteId);
      if (nodo && nodo.cantidad > 0) {
        const extraido = Math.min(EDIFICIO_CATALOGO.corral.produccionBaseLivestock * ratioMano, nodo.cantidad);
        nodo.cantidad -= extraido;
        almacen = agregarRecurso(almacen, 'livestock', extraido);
        if (nodo.cantidad <= 0) eventos.push('El manada de livestock del corral se ha agotado.');
      }
    }
    resultados.set(edificio.id, edificio);
  }

  // Paso 2: arranque de construcciones en cola, en orden de PRIORIDAD (supervivencia > extractores > general)
  // en vez de orden de inserción — bug real detectado jugando: Curtidería/Armería/Fundición no tienen gate de
  // nivel y se encolan casi desde el tick 1, con un costo de madera (80) muy superior al de Granja/Leñera
  // (30/10); si quedaban antes en el array que una Granja/Leñera nueva encolada más tarde por crecimiento de
  // población, se llevaban la madera disponible primero aunque conceptualmente su categoría (general) tenga
  // menos prioridad que supervivencia — dejando el asentamiento sin margen para sostenerse a sí mismo. El
  // `sort` es estable: dentro de la misma categoría se respeta el orden de inserción de siempre.
  const categoriaPrioridad = (tipo: EdificioTipo): number =>
    TIPOS_SUPERVIVENCIA.has(tipo) ? 0 : TIPOS_EXTRACTORES_BASE.has(tipo) ? 1 : 2;
  const enColaPorPrioridad = asentamiento.edificios
    .filter((e) => e.estado === 'en_cola')
    .sort((a, b) => categoriaPrioridad(a.tipo) - categoriaPrioridad(b.tipo));

  for (const edificio of enColaPorPrioridad) {
    const costo = EDIFICIO_CATALOGO[edificio.tipo].costo as Partial<Record<string, number>>;
    if (puedeIniciarConstruccion(almacen, costo, edificio.tipo, asentamiento.nivel)) {
      almacen = descontarRecursos(almacen, costo);
      eventos.push(`Comienza construcción de ${edificio.tipo}.`);
      // Vía Rápida de Construcción (Maestro de Obras, Doc 2.2/4.4) acelera el tiempo restante al arrancar.
      const ticks = Math.max(1, Math.round(EDIFICIO_CATALOGO[edificio.tipo].tiempoConstruccionTicks * factorTiempoConstruccion(asentamiento)));
      resultados.set(edificio.id, { ...edificio, estado: 'en_construccion', ticksRestantes: ticks });
    } else {
      resultados.set(edificio.id, edificio);
    }
  }

  const edificiosActualizados = asentamiento.edificios.map((e) => resultados.get(e.id)!);

  // Rediseño de progreso (Fase 0, Doc 4.2.1): recetas de crafting de los edificios de transformación activos.
  almacen = avanzarRecetas({ ...asentamiento, edificios: edificiosActualizados }, almacen);

  // Mejora de nivel interno (Doc 4.2.1): evalúa después de las recetas, con el almacén ya actualizado por ellas.
  const trasMejoras = avanzarMejoras({ ...asentamiento, edificios: edificiosActualizados }, almacen);
  almacen = trasMejoras.almacen;
  eventos.push(...trasMejoras.eventos);

  const asentamientoConProgreso: Asentamiento = { ...trasMejoras.asentamiento, almacen };
  const nuevosProyectos = evaluarNecesidades(asentamientoConProgreso, zonaPoligono, world);
  const nuevosEspeciales = evaluarEdificiosEspeciales(asentamientoConProgreso, zonaPoligono);
  for (const p of [...nuevosProyectos, ...nuevosEspeciales]) eventos.push(`Nueva necesidad detectada: se encola ${p.tipo}.`);

  return {
    asentamiento: { ...asentamientoConProgreso, edificios: [...asentamientoConProgreso.edificios, ...nuevosProyectos, ...nuevosEspeciales] },
    eventos,
  };
}

export class ConstruccionManualInvalidaError extends Error {}

/**
 * Gran Fundición (Doc 4.2/5.7): a diferencia del resto de edificios, es de colocación MANUAL — decisión
 * militar deliberada del jugador, no auto-construcción por necesidad. El sitio sigue eligiéndose solo (Doc
 * 4.2: el jugador no elige ubicación salvo fundación y edificios estratégicos). Rediseño de progreso (Fase
 * 0): Fundición deja de ser manual — ahora es auto-construcción con recetas reales (ver `evaluarNecesidades`).
 */
export function construirManualmente(
  asentamiento: Asentamiento,
  faccion: Faccion,
  zonaPoligono: Point[],
  tipo: 'granFundicion',
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
  const enCola = asentamiento.edificios.filter((e) => e.estado === 'en_cola').length;
  if (enCola >= NECESIDADES.maximoEnCola) {
    throw new ConstruccionManualInvalidaError(`La cola de construcción está llena (máximo ${NECESIDADES.maximoEnCola}).`);
  }
  const sitio = sitioConcentrico(asentamiento, zonaPoligono, asentamiento.edificios);
  if (!sitio) throw new ConstruccionManualInvalidaError('No hay sitio disponible dentro de la zona de influencia.');

  const nuevo = crearEdificioEnCola(tipo, sitio, `edificio-${asentamiento.id}-manual-${contador}`);
  return { ...asentamiento, edificios: [...asentamiento.edificios, nuevo] };
}
