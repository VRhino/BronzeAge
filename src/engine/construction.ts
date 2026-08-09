import type { Asentamiento, Edificio, EdificioTipo, Faccion, Point, RecursoAlmacenado, RecursoTipo, World, ZonaBosque } from '../domain/types';
import { BOSQUE, EDIFICIO_CATALOGO, EXTRACCION_MAXIMOS, NECESIDADES, NIVEL_ASENTAMIENTO, SCORE_BANDAS, SITIO, ZONA_INFLUENCIA } from '../constants';
import { pointInPolygon } from './zones';
import {
  capacidadViviendaArtesanos,
  capacidadViviendaPesants,
  edificiosPorTipoYEstado,
  hayProyectoPendiente,
  ratioManoObra,
  ratioManoObraArtesanos,
} from './asentamientoQuery';
import { agregarRecurso, descontarRecursos, tieneRecursos } from './almacen';
import { reservaDinamicaConstruccion } from './mantenimiento';
import {
  factorProduccionTrigo,
  factorTiempoConstruccion,
  minimoGranjasPrioritario,
  minimoLenerasPrioritario,
  politicaActivaDesbloqueaEdificio,
} from './politicas';
import { consumoComidaPoblacion } from './population';
import { consumoRacionTropas } from './tropas';

/**
 * Recurso propio de cada tipo de edificio "de supervivencia": Granja no respeta la reserva mínima de
 * trigo, ni Leñera la de madera (ver `reservaDinamicaConstruccion` en engine/mantenimiento.ts) — son la
 * única vía real de recuperar esos recursos, así que bloquearlas por la misma escasez que deben resolver
 * sería un huevo-y-la-gallina sin salida.
 */
const RECURSO_PROPIO: Partial<Record<EdificioTipo, string>> = { granja: 'trigo', lenera: 'madera' };

/**
 * Un proyecto solo puede COMPROMETERSE (pagarse, overhaul de auto-construcción: ver `evaluarNecesidades`) si,
 * además de poder pagar el costo completo, no deja ningún recurso por debajo de la reserva proyectada
 * (`reserva`, ver `reservaDinamicaConstruccion`) — salvo el recurso que el propio edificio produce (ver
 * `RECURSO_PROPIO`).
 */
function puedeIniciarConstruccion(
  almacen: Record<string, RecursoAlmacenado>,
  costo: Partial<Record<string, number>>,
  tipo: EdificioTipo,
  reserva: Partial<Record<RecursoTipo, number>>
): boolean {
  if (!tieneRecursos(almacen, costo)) return false;
  const exento = RECURSO_PROPIO[tipo];
  return Object.entries(costo).every(([recurso, cantidad]) => {
    if (recurso === exento) return true;
    const reservaRecurso = reserva[recurso as RecursoTipo] ?? 0;
    if (reservaRecurso <= 0) return true;
    const disponible = almacen[recurso]?.cantidad ?? 0;
    return disponible - (cantidad ?? 0) >= reservaRecurso;
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

/** Cuántas Leñeras admite un bosque a la vez según su tamaño (Doc 1.4/4.2, a petición del usuario): bosques
 * grandes permiten más de una Leñera trabajándolo en paralelo, mín 1 / máx 3 (`BOSQUE.capacidadLenerasPorRadio`). */
function capacidadLenerasBosque(bosque: ZonaBosque): number {
  if (bosque.radio >= BOSQUE.capacidadLenerasPorRadio.umbral3) return 3;
  if (bosque.radio >= BOSQUE.capacidadLenerasPorRadio.umbral2) return 2;
  return 1;
}

/** Leñeras ya colocadas por bosque (fuenteId -> cantidad), para respetar `capacidadLenerasBosque`. */
function conteoLenerasPorBosque(asentamiento: Asentamiento): Map<string, number> {
  const conteo = new Map<string, number>();
  for (const e of asentamiento.edificios) {
    if (e.tipo === 'lenera' && e.fuenteId) conteo.set(e.fuenteId, (conteo.get(e.fuenteId) ?? 0) + 1);
  }
  return conteo;
}

/**
 * Punto utilizable de un bosque que caiga DENTRO de la zona de influencia (Doc 1.4). Bug real detectado
 * jugando (reportado por el usuario): antes solo se comprobaba si el CENTRO exacto del bosque caía dentro de
 * la zona (`pointInPolygon(bosque.centro, zonaPoligono)`) — pero los bosques tienen radio (30-80) y el radio
 * de zona tiene un TOPE por nivel (60/90/120, ver ZONA_INFLUENCIA). Un bosque grande cuyo borde ya está bien
 * dentro de la zona pero cuyo centro exacto queda un poco más allá del tope de nivel era invisible para
 * siempre — el asentamiento nunca conseguía Leñera pese a que la zona "tocaba" el bosque, y acababa cayendo
 * en ruinas por falta de madera para Mantenimiento. Ahora se prueba primero el punto preferido (centro, o un
 * punto con offset si el bosque ya tiene otras Leñeras — ver `capacidadLenerasBosque`) y, si ese cae fuera de
 * la zona, se muestrean puntos en anillos crecientes dentro del propio bosque hasta encontrar uno que sí esté
 * dentro — el mismo bosque puede "entrar en contacto" con la zona por un punto distinto de su centro.
 */
function puntoEnBosqueDentroDeZona(bosque: ZonaBosque, zonaPoligono: Point[], indiceOcupacion: number): Point | null {
  const preferido =
    indiceOcupacion === 0
      ? bosque.centro
      : {
          x: bosque.centro.x + Math.cos((indiceOcupacion / 3) * Math.PI * 2) * bosque.radio * 0.4,
          y: bosque.centro.y + Math.sin((indiceOcupacion / 3) * Math.PI * 2) * bosque.radio * 0.4,
        };
  if (pointInPolygon(preferido, zonaPoligono)) return preferido;

  const muestrasPorAnillo = 12;
  for (let anillo = 1; anillo <= 3; anillo++) {
    const radio = (bosque.radio * anillo) / 3;
    for (let i = 0; i < muestrasPorAnillo; i++) {
      const angulo = (i / muestrasPorAnillo) * Math.PI * 2;
      const candidato: Point = { x: bosque.centro.x + Math.cos(angulo) * radio, y: bosque.centro.y + Math.sin(angulo) * radio };
      if (pointInPolygon(candidato, zonaPoligono)) return candidato;
    }
  }
  return null;
}

/** Lenera: junto al bosque más cercano que tenga hueco libre según su capacidad (varias Leñeras pueden
 * compartir un bosque grande, ver `capacidadLenerasBosque`) Y algún punto suyo dentro de la zona de
 * influencia (ver `puntoEnBosqueDentroDeZona` — no exige que sea justo el centro). */
function sitioEnBosque(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  world: World,
  conteoPorBosque: Map<string, number>
): { posicion: Point; fuenteId: string } | null {
  const candidatos = world.bosques
    .map((bosque) => {
      const ocupadas = conteoPorBosque.get(bosque.id) ?? 0;
      if (ocupadas >= capacidadLenerasBosque(bosque)) return null;
      const punto = puntoEnBosqueDentroDeZona(bosque, zonaPoligono, ocupadas);
      return punto ? { bosque, punto } : null;
    })
    .filter((c): c is { bosque: ZonaBosque; punto: Point } => c !== null)
    .sort((a, b) => distancia(a.bosque.centro, asentamiento.posicion) - distancia(b.bosque.centro, asentamiento.posicion));
  const elegido = candidatos[0];
  return elegido ? { posicion: elegido.punto, fuenteId: elegido.bosque.id } : null;
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

interface Candidato {
  edificio: Edificio;
  score: number;
}

/** Score final de un candidato dentro de su banda (ver `SCORE_BANDAS`, constants.ts): `base` + hasta 100 de
 * urgencia — clamp evita que un urgencia mal calculada cruce a la banda siguiente. */
function conUrgencia(base: number, urgencia: number): number {
  return base + Math.max(0, Math.min(100, urgencia));
}

/**
 * Overhaul de auto-construcción: evalúa déficits reales (Doc 4.2, misma elegibilidad que siempre —
 * `enDeficitTrigo`, `necesitaNuevoExtractor`, ocupación de vivienda/almacén, etc.) pero en vez de una cadena
 * de `if`s con slots reservados por categoría, cada candidato elegible recibe un SCORE (ver `SCORE_BANDAS`)
 * y se COMPROMETE (paga de inmediato, ver `puedeIniciarConstruccion`) en orden de score descendente mientras
 * queden fondos y cupo (`NECESIDADES.maximoEnCola`, cuenta proyectos ya en vuelo + los comprometidos este
 * tick). Reemplaza el viejo modelo donde el pago ocurría recién al ARRANCAR la construcción — un proyecto
 * podía quedar `en_cola` indefinidamente sin fondos porque otro se los gastaba antes; ahora, si no se puede
 * pagar YA, el candidato simplemente no se crea este tick (se reevalúa, normalmente con más urgencia, el
 * siguiente) — sin placeholders atascados.
 *
 * Las bandas de score (supervivencia > extractorBase > crecimiento > transformación) no se solapan, así que
 * preservan la misma garantía que antes daban los slots reservados: supervivencia nunca pierde el reparto
 * frente a crecimiento/lujo, sin necesitar mecanismo aparte.
 */
function evaluarNecesidades(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  world: World,
  reserva: Partial<Record<RecursoTipo, number>>
): { nuevos: Edificio[]; almacen: Record<string, RecursoAlmacenado> } {
  const candidatos: Candidato[] = [];
  let contador = asentamiento.edificios.length;
  const nextId = () => `edificio-${asentamiento.id}-${contador++}`;
  const ocupados = () => [...asentamiento.edificios, ...candidatos.map((c) => c.edificio)];
  const proponer = (edificio: Edificio | null, score: number): void => {
    if (edificio) candidatos.push({ edificio, score });
  };

  // Protección de Riesgos (política de Maestro de Obras, a petición del usuario: "construye 2 Leñeras y 3
  // Granjas, prioriza esto y no construyas nada más hasta que se cumpla"): mientras esté activa y falte
  // cualquiera de los dos objetivos, esta pasada SOLO propone Leñera/Granja — el resto de la función no se
  // evalúa este tick. Ambas se evalúan en la misma pasada (no hace falta terminar la Leñera antes de empezar
  // la Granja).
  // Excepción: si NINGUNA de las dos pudo avanzar este tick (sin sitio disponible para la que falte, y
  // ninguna ya en camino), bloquear igual sería un interbloqueo sin salida — en ese caso se deja pasar la
  // evaluación normal de abajo.
  const objetivoLenerasPrioritario = minimoLenerasPrioritario(asentamiento);
  const objetivoGranjasPrioritario = minimoGranjasPrioritario(asentamiento);
  let soloSupervivencia = false;
  if (objetivoLenerasPrioritario > 0 || objetivoGranjasPrioritario > 0) {
    const lenerasActivas = edificiosPorTipoYEstado(asentamiento, 'lenera').length;
    const lenerasPendientes = asentamiento.edificios.filter((e) => e.tipo === 'lenera' && e.estado !== 'activo').length;
    const lenerasFaltan = lenerasActivas + lenerasPendientes < objetivoLenerasPrioritario;

    const granjasActivasPrioridad = edificiosPorTipoYEstado(asentamiento, 'granja').length;
    const granjasPendientesPrioridad = asentamiento.edificios.filter((e) => e.tipo === 'granja' && e.estado !== 'activo').length;
    const granjasFaltan = granjasActivasPrioridad + granjasPendientesPrioridad < objetivoGranjasPrioritario;

    if (lenerasFaltan || granjasFaltan) {
      let progresando = false;

      if (lenerasFaltan) {
        if (hayProyectoPendiente(asentamiento, 'lenera')) {
          progresando = true; // ya hay una Leñera en camino hacia el objetivo.
        } else {
          const sitio = sitioEnBosque(asentamiento, zonaPoligono, world, conteoLenerasPorBosque(asentamiento));
          if (sitio) {
            proponer(crearEdificioEnCola('lenera', sitio.posicion, nextId(), sitio.fuenteId), SCORE_BANDAS.supervivencia + 100);
            progresando = true;
          }
        }
      }

      if (granjasFaltan) {
        if (hayProyectoPendiente(asentamiento, 'granja')) {
          progresando = true; // ya hay una Granja en camino hacia el objetivo.
        } else {
          const sitio = sitioMejorFertilidad(asentamiento, zonaPoligono, world, ocupados());
          if (sitio) {
            proponer(crearEdificioEnCola('granja', sitio, nextId()), SCORE_BANDAS.supervivencia + 100);
            progresando = true;
          }
        }
      }

      soloSupervivencia = progresando;
      // Si NO progresando: ni Leñera ni Granja pudieron avanzar este tick — no bloquear el resto, cae a la
      // evaluación normal de abajo (misma zona de escape que antes).
    }
  }

  if (!soloSupervivencia) {
    // Granja: al menos una, y más si estoy en DÉFICIT de trigo — producción actual de todas las Granjas
    // activas (fertilidad + mano de obra + Edicto de Cosecha) por debajo del consumo actual (población +
    // tropas, Doc 4.1/5.4) — no "cuántos ticks de reserva quedan al ritmo de hoy".
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
    // déficit severo solo podía corregirse construyendo Granjas en SERIE, quedándose muy por detrás.
    const granjasPendientes = asentamiento.edificios.filter((e) => e.tipo === 'granja' && e.estado !== 'activo').length;
    const limiteGranjasPendientes = enDeficitTrigo ? NECESIDADES.maximoGranjasPendientesEnDeficit : 1;
    if ((granjasActivasEdificios.length === 0 || enDeficitTrigo) && granjasPendientes < limiteGranjasPendientes) {
      const sitio = sitioMejorFertilidad(asentamiento, zonaPoligono, world, ocupados());
      if (sitio) {
        const deficitRatio = consumoTrigoActual > 0 ? (consumoTrigoActual - produccionTrigoActual) / consumoTrigoActual : 1;
        const urgencia = granjasActivasEdificios.length === 0 ? 100 : deficitRatio * 100;
        proponer(crearEdificioEnCola('granja', sitio, nextId()), conUrgencia(SCORE_BANDAS.supervivencia, urgencia));
      }
    }

    // Lenera: mismo tope que los extractores minerales (los bosques no se agotan, Doc 1.4). Urgencia máxima
    // si no hay ninguna, alta si la reserva proyectada de madera ya está comprometida, baja si solo falta
    // para llegar al tope.
    const leneras = edificiosPorTipoYEstado(asentamiento, 'lenera');
    if (leneras.length < EXTRACCION_MAXIMOS.porTipo && !hayProyectoPendiente(asentamiento, 'lenera')) {
      const sitio = sitioEnBosque(asentamiento, zonaPoligono, world, conteoLenerasPorBosque(asentamiento));
      if (sitio) {
        const maderaBajoReserva = (asentamiento.almacen.madera?.cantidad ?? 0) < (reserva.madera ?? 0);
        const urgencia = leneras.length === 0 ? 100 : maderaBajoReserva ? 90 : 30;
        proponer(crearEdificioEnCola('lenera', sitio.posicion, nextId(), sitio.fuenteId), conUrgencia(SCORE_BANDAS.supervivencia, urgencia));
      }
    }

    // Extractores base (cantera/corral/minaCobre/mina/minaEstano, Doc 1.1/1.4/3.1/4.2.1/5.7): mismo patrón —
    // urgencia máxima si NINGUNA fuente propia sigue viva, moderada si solo falta para llegar al tope.
    const extractores: { tipo: EdificioTipo; recurso: string }[] = [
      { tipo: 'cantera', recurso: 'piedra' },
      { tipo: 'corral', recurso: 'livestock' },
      { tipo: 'minaCobre', recurso: 'cobre' },
      { tipo: 'mina', recurso: 'oro' },
      { tipo: 'minaEstano', recurso: 'estano' },
    ];
    for (const { tipo, recurso } of extractores) {
      if (necesitaNuevoExtractor(asentamiento, tipo, world) && !hayProyectoPendiente(asentamiento, tipo)) {
        const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, world, recurso, fuentesReclamadas(asentamiento, tipo));
        if (sitio) {
          const conFuenteViva = asentamiento.edificios
            .filter((e) => e.tipo === tipo)
            .some((e) => {
              const nodo = world.recursos.find((n) => n.id === e.fuenteId);
              return nodo && nodo.cantidad > 0;
            });
          proponer(
            crearEdificioEnCola(tipo, sitio.posicion, nextId(), sitio.fuenteId),
            conUrgencia(SCORE_BANDAS.extractorBase, conFuenteViva ? 40 : 100)
          );
        }
      }
    }

    // Cupos separados por clase (ver `crecerPoblacion`, engine/population.ts): una Vivienda nueva amplía
    // ambos a la vez, así que basta con que CUALQUIERA de los dos ya esté saturado para dispararla. Urgencia
    // escala con el % de ocupación.
    const capacidadPesantsVivienda = capacidadViviendaPesants(asentamiento);
    const capacidadArtesanosVivienda = capacidadViviendaArtesanos(asentamiento);
    const ocupacionPesants = capacidadPesantsVivienda <= 0 ? 1 : asentamiento.poblacion.pesants / capacidadPesantsVivienda;
    const ocupacionArtesanos = capacidadArtesanosVivienda <= 0 ? 1 : asentamiento.poblacion.artesanos / capacidadArtesanosVivienda;
    const ocupacionMaxima = Math.max(ocupacionPesants, ocupacionArtesanos);
    if (
      (capacidadPesantsVivienda === 0 || ocupacionMaxima >= NECESIDADES.umbralViviendaOcupada) &&
      !hayProyectoPendiente(asentamiento, 'vivienda')
    ) {
      const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
      if (sitio) proponer(crearEdificioEnCola('vivienda', sitio, nextId()), conUrgencia(SCORE_BANDAS.crecimiento, ocupacionMaxima * 100));
    }

    // Almacén: urgencia escala con el % de ocupación del recurso más lleno.
    const ocupacionAlmacenes = Object.values(asentamiento.almacen)
      .filter((r) => r.capacidad > 0)
      .map((r) => r.cantidad / r.capacidad);
    const ocupacionAlmacenMaxima = ocupacionAlmacenes.length ? Math.max(...ocupacionAlmacenes) : 0;
    if (ocupacionAlmacenMaxima >= NECESIDADES.umbralAlmacenAmpliacion && !hayProyectoPendiente(asentamiento, 'almacen')) {
      const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
      if (sitio) proponer(crearEdificioEnCola('almacen', sitio, nextId()), conUrgencia(SCORE_BANDAS.crecimiento, ocupacionAlmacenMaxima * 100));
    }

    // Edificios de transformación (Doc 4.2.1, rediseño de progreso Fase 0): Curtiduría/Armería/Fundición no
    // tienen gate de nivel para su construcción BASE — solo sus mejoras de nivel interno lo exigen (ver
    // `avanzarMejoras`). Como máximo UNA de las tres puede estar en vuelo a la vez (bug detectado en
    // simulación: sin este límite, varias podían acumularse atascadas esperando piedra en un punto de
    // fundación pobre en ese recurso).
    const transformacionEnCurso = (['curtiduria', 'armeria', 'fundicion'] as const).some(
      (tipo) => hayProyectoPendiente(asentamiento, tipo)
    );
    if (!transformacionEnCurso) {
      for (const tipo of ['curtiduria', 'armeria', 'fundicion'] as const) {
        if (edificiosPorTipoYEstado(asentamiento, tipo).length === 0) {
          const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
          if (sitio) {
            proponer(crearEdificioEnCola(tipo, sitio, nextId()), SCORE_BANDAS.transformacion);
            break;
          }
        }
      }
    }

    // Carpintería: a diferencia de los otros 3, su construcción BASE sí exige nivel de asentamiento
    // (`requisitoNivelAsentamientoConstruccion`) — habilita Armería/Barracón/Galería de tiro nivel 2.
    const requisitoCarpinteria =
      (EDIFICIO_CATALOGO.carpinteria as { requisitoNivelAsentamientoConstruccion?: number }).requisitoNivelAsentamientoConstruccion ?? 0;
    if (
      asentamiento.nivel >= requisitoCarpinteria &&
      edificiosPorTipoYEstado(asentamiento, 'carpinteria').length === 0 &&
      !hayProyectoPendiente(asentamiento, 'carpinteria')
    ) {
      const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
      if (sitio) proponer(crearEdificioEnCola('carpinteria', sitio, nextId()), SCORE_BANDAS.transformacion);
    }
  }

  // Commit único: ordena todos los candidatos de este tick por score descendente y paga en ese orden
  // mientras haya cupo (`maximoEnCola`, cuenta SOLO lo ya `en_cola` — pagado, a la espera de un hueco de
  // obra — más lo que se va comprometiendo aquí mismo; NO cuenta `en_construccion`, que tiene su propio
  // cupo separado — `maximoEnConstruccionSimultanea`, ver Paso 2 de `avanzarConstruccion` — contarlo aquí
  // también duplicaría la restricción y frenaría el desarrollo sin necesidad) y fondos suficientes
  // respetando la reserva proyectada. El pago ocurre AQUÍ: el candidato entra `en_cola` ya descontado del
  // almacén (overhaul de auto-construcción — Paso 2 ya no comprueba fondos al arrancar la obra).
  const enColaActual = asentamiento.edificios.filter((e) => e.estado === 'en_cola').length;
  let cupoDisponible = Math.max(0, NECESIDADES.maximoEnCola - enColaActual);
  let almacenActual = asentamiento.almacen;
  const nuevos: Edificio[] = [];
  for (const candidato of [...candidatos].sort((a, b) => b.score - a.score)) {
    if (cupoDisponible <= 0) break;
    const costo = EDIFICIO_CATALOGO[candidato.edificio.tipo].costo as Partial<Record<string, number>>;
    if (!puedeIniciarConstruccion(almacenActual, costo, candidato.edificio.tipo, reserva)) continue;
    almacenActual = descontarRecursos(almacenActual, costo);
    nuevos.push({ ...candidato.edificio, prioridad: candidato.score });
    cupoDisponible -= 1;
  }

  return { nuevos, almacen: almacenActual };
}

/**
 * Edificios especiales vía política (Doc 4.4/4.2.1, rediseño de progreso Fase 0): Barracón, Galería de tiro y
 * Palacio NO son auto-construcción — solo se encolan mientras la política de desbloqueo correspondiente esté
 * activa (ver `politicaActivaDesbloqueaEdificio`) y se cumpla su gate de nivel/edificio previo. Van en un
 * CLUSTER DE COLA APARTE: no cuentan contra `NECESIDADES.maximoEnCola` ni compiten con `evaluarNecesidades`
 * por ese cupo — pero, overhaul de auto-construcción, pagan de inmediato al comprometerse igual que el resto
 * (ver `puedeIniciarConstruccion`), y sí cuentan contra `maximoEnConstruccionSimultanea` al arrancar obra
 * (comparten cuadrillas con el resto de proyectos, Paso 2 de `avanzarConstruccion`).
 */
function evaluarEdificiosEspeciales(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  reserva: Partial<Record<RecursoTipo, number>>,
  almacen: Record<string, RecursoAlmacenado>
): { nuevos: Edificio[]; almacen: Record<string, RecursoAlmacenado> } {
  const nuevos: Edificio[] = [];
  let contador = asentamiento.edificios.length + 1000; // rango separado para no colisionar con evaluarNecesidades
  const nextId = () => `edificio-${asentamiento.id}-especial-${contador++}`;
  const ocupados = () => [...asentamiento.edificios, ...nuevos];
  let almacenActual = almacen;

  const candidatos: { tipo: 'barracon' | 'galeriaDeTiro' | 'palacio' | 'mercado'; requisitoNivel: number }[] = [
    { tipo: 'barracon', requisitoNivel: 0 },
    { tipo: 'galeriaDeTiro', requisitoNivel: 0 },
    { tipo: 'palacio', requisitoNivel: (EDIFICIO_CATALOGO.palacio as { requisitoNivelAsentamientoConstruccion?: number }).requisitoNivelAsentamientoConstruccion ?? 0 },
    // Ampliación de comercio (a petición del usuario): mismo patrón que Barracón/Galería — vía política del
    // Tesorero ("Construir Mercado"), sin gate de nivel de asentamiento para la construcción BASE (solo sus
    // mejoras de nivel interno lo exigen, ver EDIFICIO_CATALOGO.mercado.niveles).
    { tipo: 'mercado', requisitoNivel: 0 },
  ];

  for (const { tipo, requisitoNivel } of candidatos) {
    if (!politicaActivaDesbloqueaEdificio(asentamiento, tipo)) continue;
    if (asentamiento.nivel < requisitoNivel) continue;
    if (edificiosPorTipoYEstado(asentamiento, tipo).length > 0 || hayProyectoPendiente(asentamiento, tipo)) continue;
    const sitio = sitioConcentrico(asentamiento, zonaPoligono, ocupados());
    if (!sitio) continue;
    const costo = EDIFICIO_CATALOGO[tipo].costo as Partial<Record<string, number>>;
    if (!puedeIniciarConstruccion(almacenActual, costo, tipo, reserva)) continue;
    almacenActual = descontarRecursos(almacenActual, costo);
    nuevos.push({ ...crearEdificioEnCola(tipo, sitio, nextId()), prioridad: SCORE_BANDAS.crecimiento });
  }

  return { nuevos, almacen: almacenActual };
}

/** Tipos de edificio de transformación con tiers (Doc 4.2.1): mejoran de nivelInterno y ejecutan recetas.
 * Mercado se suma aquí solo por el mecanismo de MEJORA de nivel interno (`avanzarMejoras`) — sus "recetas"
 * están vacías, el nivel interno solo cambia `cupoCaravanas` (ver `cupoCaravanas`, asentamientoQuery.ts). */
const EDIFICIOS_CON_NIVELES = ['fundicion', 'curtiduria', 'armeria', 'carpinteria', 'barracon', 'galeriaDeTiro', 'mercado'] as const;

function nivelesDe(tipo: EdificioTipo): Record<number, { trabajadoresRequeridos: number; recetas: { produce: string; produccionBase: number; consumePorUnidad: Partial<Record<string, number>> }[]; costoMejora?: Partial<Record<string, number>>; requisitoNivelAsentamiento?: number; requiereEdificio?: string; requiereEdificioNivel?: number }> | undefined {
  return (EDIFICIO_CATALOGO[tipo] as { niveles?: Record<number, any> }).niveles;
}

/**
 * Mejora de nivel interno de un edificio de transformación activo (Doc 4.2.1, rediseño de progreso Fase 0):
 * instantánea — si se cumple el gate del siguiente nivel (nivel de asentamiento + edificio previo, si aplica)
 * y hay fondos para `costoMejora` (respetando la misma reserva mínima que protege el inicio de construcción),
 * se paga y sube `nivelInterno` en el mismo tick. No hay tiempo de mejora especificado en el diseño original.
 */
function avanzarMejoras(
  asentamiento: Asentamiento,
  almacen: Record<string, RecursoAlmacenado>,
  reserva: Partial<Record<RecursoTipo, number>>
): { asentamiento: Asentamiento; almacen: Record<string, RecursoAlmacenado>; eventos: string[] } {
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
    if (!puedeIniciarConstruccion(almacenActual, costo, edificio.tipo, reserva)) return edificio;
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
 * `capital` (overhaul de auto-construcción): asentamiento "capital" de la Facción, ya calculado en
 * `simulation.ts` — se usa para proyectar la reserva mínima dinámica (ver `reservaDinamicaConstruccion`,
 * engine/mantenimiento.ts, que reutiliza `calcularCostoMantenimiento`, sensible a la distancia a la capital).
 */
export function avanzarConstruccion(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  world: World,
  capital: Asentamiento | undefined
): { asentamiento: Asentamiento; eventos: string[] } {
  const eventos: string[] = [];
  let almacen = asentamiento.almacen;
  const resultados = new Map<string, Edificio>();

  const ratioMano = ratioManoObra(asentamiento);
  // Crecimiento de zona ligado a construcción activa (Doc 1.2, rediseño a petición del usuario): ya no es
  // puramente temporal — cada edificio completado este tick empuja el radio hacia el techo de su nivel.
  let edificiosCompletadosEsteTick = 0;

  // Paso 1 (orden original): progreso de construcciones en curso + producción de edificios activos. Los
  // `en_cola` se resuelven en un segundo paso por PRIORIDAD (ver abajo), no aquí.
  for (const edificio of asentamiento.edificios) {
    if (edificio.estado === 'en_construccion') {
      const restantes = edificio.ticksRestantes - 1;
      if (restantes <= 0) {
        eventos.push(`${edificio.tipo} completado.`);
        edificiosCompletadosEsteTick += 1;
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

  // Paso 2: arranque de obra. Overhaul de auto-construcción: los `en_cola` ya están PAGADOS (el pago ocurrió
  // al comprometerse, ver `evaluarNecesidades` más abajo) — ya no hace falta comprobar fondos aquí. Lo único
  // que limita el arranque es el cupo de obras activas simultáneas (`maximoEnConstruccionSimultanea`,
  // cuadrillas limitadas: evita que un tick con el almacén lleno dispare media docena de construcciones en
  // paralelo), repartido por `prioridad` (score capturado al comprometerse, mayor primero).
  const enConstruccionActual = asentamiento.edificios.filter((e) => e.estado === 'en_construccion').length;
  let cupoObraDisponible = Math.max(0, NECESIDADES.maximoEnConstruccionSimultanea - enConstruccionActual);
  const enColaPorPrioridad = asentamiento.edificios
    .filter((e) => e.estado === 'en_cola')
    .sort((a, b) => (b.prioridad ?? 0) - (a.prioridad ?? 0));

  for (const edificio of enColaPorPrioridad) {
    if (cupoObraDisponible <= 0) {
      resultados.set(edificio.id, edificio);
      continue;
    }
    eventos.push(`Comienza construcción de ${edificio.tipo}.`);
    // Vía Rápida de Construcción (Maestro de Obras, Doc 2.2/4.4) acelera el tiempo restante al arrancar.
    const ticks = Math.max(1, Math.round(EDIFICIO_CATALOGO[edificio.tipo].tiempoConstruccionTicks * factorTiempoConstruccion(asentamiento)));
    resultados.set(edificio.id, { ...edificio, estado: 'en_construccion', ticksRestantes: ticks });
    cupoObraDisponible -= 1;
  }

  const edificiosActualizados = asentamiento.edificios.map((e) => resultados.get(e.id)!);

  // Rediseño de progreso (Fase 0, Doc 4.2.1): recetas de crafting de los edificios de transformación activos.
  almacen = avanzarRecetas({ ...asentamiento, edificios: edificiosActualizados }, almacen);

  const reserva = reservaDinamicaConstruccion({ ...asentamiento, edificios: edificiosActualizados, almacen }, capital);

  // Mejora de nivel interno (Doc 4.2.1): evalúa después de las recetas, con el almacén ya actualizado por ellas.
  const trasMejoras = avanzarMejoras({ ...asentamiento, edificios: edificiosActualizados }, almacen, reserva);
  almacen = trasMejoras.almacen;
  eventos.push(...trasMejoras.eventos);

  const radioPotencial = Math.min(
    ZONA_INFLUENCIA.radioMaximoPorNivel[asentamiento.nivel] ?? ZONA_INFLUENCIA.radioMaximoPorNivel[NIVEL_ASENTAMIENTO.nivelMaximo]!,
    asentamiento.radioPotencial + ZONA_INFLUENCIA.crecimientoPorEdificioCompletado * edificiosCompletadosEsteTick
  );
  const asentamientoConProgreso: Asentamiento = { ...trasMejoras.asentamiento, almacen, radioPotencial };

  // Paso 3: compromiso de necesidades (overhaul — reemplaza el viejo "encolar sin pagar"). Pausable por el
  // jugador (ver `Asentamiento.autoConstruccionPausada`): mientras está pausada, no se detectan/comprometen
  // NUEVAS necesidades, pero lo ya pagado (Paso 1/2 de arriba) sigue avanzando normal.
  let nuevosProyectos: Edificio[] = [];
  let nuevosEspeciales: Edificio[] = [];
  let almacenFinal = asentamientoConProgreso.almacen;
  if (!asentamiento.autoConstruccionPausada) {
    const trasNecesidades = evaluarNecesidades(asentamientoConProgreso, zonaPoligono, world, reserva);
    nuevosProyectos = trasNecesidades.nuevos;
    almacenFinal = trasNecesidades.almacen;
    const trasEspeciales = evaluarEdificiosEspeciales(
      { ...asentamientoConProgreso, almacen: almacenFinal },
      zonaPoligono,
      reserva,
      almacenFinal
    );
    nuevosEspeciales = trasEspeciales.nuevos;
    almacenFinal = trasEspeciales.almacen;
  }
  for (const p of [...nuevosProyectos, ...nuevosEspeciales]) eventos.push(`Nueva necesidad detectada: se compromete ${p.tipo} (pagado).`);

  const edificiosFinal = [...asentamientoConProgreso.edificios, ...nuevosProyectos, ...nuevosEspeciales];
  // Reordena los `en_cola` por `prioridad` (mismo criterio que el Paso 2) para que la posición mostrada en la
  // UI (ver main.ts) coincida con el orden real en que arrancarán en el próximo tick.
  const enColaOrdenados = edificiosFinal.filter((e) => e.estado === 'en_cola').sort((a, b) => (b.prioridad ?? 0) - (a.prioridad ?? 0));
  let indiceEnCola = 0;
  const edificiosOrdenados = edificiosFinal.map((e) => (e.estado === 'en_cola' ? enColaOrdenados[indiceEnCola++]! : e));

  return {
    asentamiento: { ...asentamientoConProgreso, almacen: almacenFinal, edificios: edificiosOrdenados },
    eventos,
  };
}

export class ConstruccionManualInvalidaError extends Error {}

/**
 * Gran Fundición (Doc 4.2/5.7): a diferencia del resto de edificios, es de colocación MANUAL — decisión
 * militar deliberada del jugador, no auto-construcción por necesidad. El sitio sigue eligiéndose solo (Doc
 * 4.2: el jugador no elige ubicación salvo fundación y edificios estratégicos). Rediseño de progreso (Fase
 * 0): Fundición deja de ser manual — ahora es auto-construcción con recetas reales (ver `evaluarNecesidades`).
 * Overhaul de auto-construcción: paga de inmediato al comprometerse, igual que el resto — rechaza con
 * `ConstruccionManualInvalidaError` si no alcanzan los fondos (respetando la reserva proyectada) en vez de
 * dejarla atascada `en_cola` esperando.
 */
export function construirManualmente(
  asentamiento: Asentamiento,
  faccion: Faccion,
  zonaPoligono: Point[],
  tipo: 'granFundicion',
  contador = 0,
  capital: Asentamiento | undefined = undefined
): Asentamiento {
  if (edificiosPorTipoYEstado(asentamiento, tipo).length > 0 || hayProyectoPendiente(asentamiento, tipo)) {
    throw new ConstruccionManualInvalidaError(`Ya existe (o está en curso) una ${tipo} en este asentamiento.`);
  }
  if (tipo === 'granFundicion' && faccion.nivel < EDIFICIO_CATALOGO.granFundicion.nivelFaccionMinimo) {
    throw new ConstruccionManualInvalidaError(
      `Requiere nivel de Facción ${EDIFICIO_CATALOGO.granFundicion.nivelFaccionMinimo} (actual: ${faccion.nivel}).`
    );
  }
  const enColaActual = asentamiento.edificios.filter((e) => e.estado === 'en_cola').length;
  if (enColaActual >= NECESIDADES.maximoEnCola) {
    throw new ConstruccionManualInvalidaError(`La cola de construcción está llena (máximo ${NECESIDADES.maximoEnCola}).`);
  }
  const sitio = sitioConcentrico(asentamiento, zonaPoligono, asentamiento.edificios);
  if (!sitio) throw new ConstruccionManualInvalidaError('No hay sitio disponible dentro de la zona de influencia.');

  const costo = EDIFICIO_CATALOGO[tipo].costo as Partial<Record<string, number>>;
  const reserva = reservaDinamicaConstruccion(asentamiento, capital);
  if (!puedeIniciarConstruccion(asentamiento.almacen, costo, tipo, reserva)) {
    throw new ConstruccionManualInvalidaError('No hay fondos suficientes (respetando la reserva de mantenimiento) para pagarla ahora.');
  }

  const almacen = descontarRecursos(asentamiento.almacen, costo);
  const nuevo = { ...crearEdificioEnCola(tipo, sitio, `edificio-${asentamiento.id}-manual-${contador}`), prioridad: SCORE_BANDAS.crecimiento };
  return { ...asentamiento, almacen, edificios: [...asentamiento.edificios, nuevo] };
}
