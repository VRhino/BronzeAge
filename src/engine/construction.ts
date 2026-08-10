import type { Asentamiento, Edificio, EdificioTipo, Faccion, Point, RecursoAlmacenado, RecursoTipo } from '../domain/types';
import { EDIFICIO_CATALOGO, EXTRACCION_MAXIMOS, NECESIDADES, NIVEL_ASENTAMIENTO, SCORE_BANDAS, SITIO, ZONA_INFLUENCIA } from '../constants';
import type { Mapa } from '../world/mapa';
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
 * Edificios que extraen contra un NODO finito del mapa y se quedan sin nada cuando lo agotan (Doc 1.4:
 * escasez real por ubicación). Granja y Leñera no están aquí: una produce del campo de fertilidad y la otra
 * de un bosque, y ninguno de los dos se agota.
 *
 * `produccionBase` es una función y no un número porque `EDIFICIO_CATALOGO` se edita en caliente desde el
 * panel de balance: hay que leer el valor en el momento de producir, no al cargar el módulo.
 */
const EXTRACTORES: Partial<Record<EdificioTipo, { recurso: RecursoTipo; produccionBase: () => number; mensajeAgotado: string }>> = {
  cantera: {
    recurso: 'piedra',
    produccionBase: () => EDIFICIO_CATALOGO.cantera.produccionBasePiedra,
    mensajeAgotado: 'El yacimiento de piedra de la cantera se ha agotado.',
  },
  mina: {
    recurso: 'oro',
    produccionBase: () => EDIFICIO_CATALOGO.mina.produccionBaseOro,
    mensajeAgotado: 'El yacimiento de oro de la mina se ha agotado.',
  },
  minaCobre: {
    recurso: 'cobre',
    produccionBase: () => EDIFICIO_CATALOGO.minaCobre.produccionBaseCobre,
    mensajeAgotado: 'El yacimiento de cobre se ha agotado.',
  },
  minaEstano: {
    recurso: 'estano',
    produccionBase: () => EDIFICIO_CATALOGO.minaEstano.produccionBaseEstano,
    mensajeAgotado: 'El yacimiento de estaño se ha agotado.',
  },
  corral: {
    recurso: 'livestock',
    produccionBase: () => EDIFICIO_CATALOGO.corral.produccionBaseLivestock,
    mensajeAgotado: 'El manada de livestock del corral se ha agotado.',
  },
};

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
  mapa: Mapa,
  ocupados: Edificio[]
): Point | null {
  // Qué puntos son elegibles lo decide el motor (zona de influencia + sitios ya ocupados); cuál de ellos es
  // el más fértil, el mapa.
  const candidatos: Point[] = [];
  for (let i = 0; i < SITIO.muestrasFertilidad; i++) {
    const angulo = (i / SITIO.muestrasFertilidad) * Math.PI * 2;
    const radio = (i % 5) / 5 * asentamiento.radioPotencial;
    const candidato: Point = {
      x: asentamiento.posicion.x + Math.cos(angulo) * radio,
      y: asentamiento.posicion.y + Math.sin(angulo) * radio,
    };
    if (!pointInPolygon(candidato, zonaPoligono) || !sitioLibre(candidato, ocupados)) continue;
    candidatos.push(candidato);
  }
  return mapa.mejorPorFertilidad(candidatos)?.punto ?? null;
}

/** Cantera/edificio de extracción: junto al nodo de recurso más cercano SIN reclamar ya (Doc 4.2, ej. herrería cerca de mina). */
function sitioCercaDeNodo(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  mapa: Mapa,
  tipoRecurso: string,
  fuentesExcluidas: Set<string>
): { posicion: Point; fuenteId: string } | null {
  // 'cima' (Fase 0.1) es inhabitable: la generación ya evita colocar nodos ahí (`RECURSO_BIOMA_PERMITIDO`
  // nunca la lista), pero el rejection-sampling tiene un fallback de "mapa saturado, coloca igual" — este
  // filtro es la garantía dura de que, aun en ese caso raro, nunca se planta un extractor ahí.
  const elegido = mapa
    .nodosEnPoligono(zonaPoligono, {
      tipo: tipoRecurso,
      conStock: true,
      excluir: fuentesExcluidas,
      ordenarPorCercaniaA: asentamiento.posicion,
    })
    .find((n) => mapa.terrenoEn(n.posicion) !== 'cima');
  return elegido ? { posicion: elegido.posicion, fuenteId: elegido.id } : null;
}

/**
 * Fuentes del mapa ya tomadas, sumando TODOS los asentamientos vivos.
 *
 * Antes esto se calculaba por asentamiento, mirando solo sus propios edificios. Entre facciones distintas
 * daba igual (las zonas de influencia se recortan y no se solapan, ver `engine/zones.ts`), pero dos
 * asentamientos de la MISMA facción sí solapan zona: los dos veían el mismo yacimiento libre y los dos le
 * plantaban una mina encima, drenándolo al doble de velocidad y dejando a ambos sin recurso mucho antes de
 * lo que ninguna cuenta del diseño preveía. Con el conteo global, quien llega primero se lo queda.
 *
 * Se calcula una vez por tick y se MUTA a medida que cada asentamiento compromete obra (ver
 * `registrarReclamo`), para que dos asentamientos procesados en el mismo tick tampoco choquen entre sí.
 */
export interface ReclamosFuentes {
  /** Yacimientos con un extractor ya asignado (mina/cantera/corral...). Exclusivos: uno por nodo. */
  nodos: Set<string>;
  /** Leñeras por bosque. NO son exclusivas: un bosque grande admite hasta 3 (ver `mapa.capacidadLeneras`). */
  lenerasPorBosque: Map<string, number>;
}

export function reclamosDeFuentes(asentamientos: Asentamiento[]): ReclamosFuentes {
  const reclamos: ReclamosFuentes = { nodos: new Set(), lenerasPorBosque: new Map() };
  for (const asentamiento of asentamientos) {
    for (const edificio of asentamiento.edificios) {
      registrarReclamo(reclamos, edificio);
    }
  }
  return reclamos;
}

/** Anota la fuente que ocupa un edificio (si ocupa alguna). Idempotente para nodos, acumulativo para bosques. */
function registrarReclamo(reclamos: ReclamosFuentes, edificio: Edificio): void {
  if (!edificio.fuenteId) return;
  if (edificio.tipo === 'lenera') {
    reclamos.lenerasPorBosque.set(edificio.fuenteId, (reclamos.lenerasPorBosque.get(edificio.fuenteId) ?? 0) + 1);
  } else {
    reclamos.nodos.add(edificio.fuenteId);
  }
}

/**
 * Leñera: junto al bosque más cercano que tenga hueco libre según su capacidad (varias Leñeras pueden
 * compartir un bosque grande) Y algún punto suyo dentro de la zona de influencia — no exige que sea justo
 * el centro del bosque. Ambos criterios los resuelve el mapa (`bosqueParaLenera`); aquí solo se aporta el
 * dato que el mapa no puede saber: cuántas Leñeras tiene ya cada bosque, que es estado del asentamiento.
 */
function sitioEnBosque(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  mapa: Mapa,
  conteoPorBosque: Map<string, number>
): { posicion: Point; fuenteId: string } | null {
  return mapa.bosqueParaLenera(zonaPoligono, conteoPorBosque, asentamiento.posicion);
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
function necesitaNuevoExtractor(asentamiento: Asentamiento, tipo: EdificioTipo, mapa: Mapa): boolean {
  const existentes = asentamiento.edificios.filter((e) => e.tipo === tipo);
  const conFuenteViva = existentes.filter((e) => mapa.nodoProductivo(e.fuenteId));
  if (conFuenteViva.length === 0) return true;
  return conFuenteViva.length < EXTRACCION_MAXIMOS.porTipo;
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
  mapa: Mapa,
  reserva: Partial<Record<RecursoTipo, number>>,
  reclamos: ReclamosFuentes
): { nuevos: Edificio[]; almacen: Record<string, RecursoAlmacenado> } {
  const candidatos: Candidato[] = [];
  let contador = asentamiento.edificios.length;
  /**
   * Ids únicos DENTRO del asentamiento, verificado contra los que ya existen.
   *
   * El contador arranca en `edificios.length`, pero cada candidato PROPUESTO consume un número aunque
   * luego no se comprometa (sin fondos o sin cupo). Como el contador del tick siguiente vuelve a partir de
   * la longitud real, esos números se reutilizan: bastaba una pasada que propusiera 3 y comprometiera solo
   * el de mayor score para que un tick posterior repitiera una id ya usada. El edificio duplicado aparecía
   * DOS VECES en `edificios` (el mapa por id de `avanzarConstruccion` resuelve las dos posiciones al mismo
   * objeto), y con él se duplicaban su producción y su reclamo de fuente — se veía como un bosque con más
   * Leñeras de las que admite o un yacimiento con dos extractores.
   */
  const idsUsadas = new Set(asentamiento.edificios.map((e) => e.id));
  const nextId = (): string => {
    let id = `edificio-${asentamiento.id}-${contador++}`;
    while (idsUsadas.has(id)) id = `edificio-${asentamiento.id}-${contador++}`;
    idsUsadas.add(id);
    return id;
  };
  const ocupados = () => [...asentamiento.edificios, ...candidatos.map((c) => c.edificio)];
  const proponer = (edificio: Edificio | null, score: number): void => {
    if (edificio) candidatos.push({ edificio, score });
  };
  /**
   * ¿Hay ya un proyecto de este tipo en el asentamiento, O propuesto en esta misma pasada?
   *
   * Mirar solo `asentamiento.edificios` (que es lo que hace `hayProyectoPendiente`) no basta: varias ramas
   * de esta función pueden proponer el mismo tipo en el mismo tick — p. ej. Leñera por la vía de Protección
   * de Riesgos y otra vez por la de expansión — y como la fuente no se reclama hasta el commit
   * (`registrarReclamo`, al final), las dos elegían el MISMO bosque o yacimiento. El resultado eran bosques
   * con más Leñeras de las que admite su capacidad y nodos con dos extractores encima, justo lo que
   * `reclamosDeFuentes` existe para impedir entre asentamientos distintos.
   */
  const proyectoEnCurso = (tipo: EdificioTipo): boolean =>
    hayProyectoPendiente(asentamiento, tipo) || candidatos.some((c) => c.edificio.tipo === tipo);

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
        if (proyectoEnCurso('lenera')) {
          progresando = true; // ya hay una Leñera en camino hacia el objetivo.
        } else {
          const sitio = sitioEnBosque(asentamiento, zonaPoligono, mapa, reclamos.lenerasPorBosque);
          if (sitio) {
            proponer(crearEdificioEnCola('lenera', sitio.posicion, nextId(), sitio.fuenteId), SCORE_BANDAS.supervivencia + 100);
            progresando = true;
          }
        }
      }

      if (granjasFaltan) {
        if (proyectoEnCurso('granja')) {
          progresando = true; // ya hay una Granja en camino hacia el objetivo.
        } else {
          const sitio = sitioMejorFertilidad(asentamiento, zonaPoligono, mapa, ocupados());
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
      (acc, e) => acc + EDIFICIO_CATALOGO.granja.produccionBaseTrigo * mapa.fertilidadEn(e.posicion) * ratioManoActual * factorTrigoActual,
      0
    );
    const consumoTrigoActual = consumoComidaPoblacion(asentamiento) + consumoRacionTropas(asentamiento);
    const enDeficitTrigo = produccionTrigoActual < consumoTrigoActual;
    // En déficit se permite tener varias Granjas en camino a la vez (hasta el tope), no solo una: sin esto, un
    // déficit severo solo podía corregirse construyendo Granjas en SERIE, quedándose muy por detrás.
    const granjasPendientes = asentamiento.edificios.filter((e) => e.tipo === 'granja' && e.estado !== 'activo').length;
    const limiteGranjasPendientes = enDeficitTrigo ? NECESIDADES.maximoGranjasPendientesEnDeficit : 1;
    if ((granjasActivasEdificios.length === 0 || enDeficitTrigo) && granjasPendientes < limiteGranjasPendientes) {
      const sitio = sitioMejorFertilidad(asentamiento, zonaPoligono, mapa, ocupados());
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
    if (leneras.length < EXTRACCION_MAXIMOS.porTipo && !proyectoEnCurso('lenera')) {
      const sitio = sitioEnBosque(asentamiento, zonaPoligono, mapa, reclamos.lenerasPorBosque);
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
      if (necesitaNuevoExtractor(asentamiento, tipo, mapa) && !proyectoEnCurso(tipo)) {
        const sitio = sitioCercaDeNodo(asentamiento, zonaPoligono, mapa, recurso, reclamos.nodos);
        if (sitio) {
          const conFuenteViva = asentamiento.edificios
            .filter((e) => e.tipo === tipo)
            .some((e) => mapa.nodoProductivo(e.fuenteId));
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
    // La fuente queda tomada en el momento en que se PAGA el proyecto, no al proponerlo: un candidato que
    // no llega a comprometerse (sin fondos o sin cupo) no debe bloquear el yacimiento a nadie más.
    registrarReclamo(reclamos, candidato.edificio);
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
  // Mismo riesgo de reutilización de id que en `evaluarNecesidades` (ver allí el detalle): un candidato que
  // no llega a comprometerse consume número igual, y el contador del tick siguiente vuelve a partir de la
  // longitud real.
  const idsUsadas = new Set(asentamiento.edificios.map((e) => e.id));
  const nextId = (): string => {
    let id = `edificio-${asentamiento.id}-especial-${contador++}`;
    while (idsUsadas.has(id)) id = `edificio-${asentamiento.id}-especial-${contador++}`;
    idsUsadas.add(id);
    return id;
  };
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
 * NOTA: los yacimientos se agotan mutando el mapa in-place vía `mapa.extraer` (simplificación deliberada de
 * Fase 0 para evitar clonar cientos de nodos cada tick); el resto del estado se trata de forma inmutable.
 * `capital` (overhaul de auto-construcción): asentamiento "capital" de la Facción, ya calculado en
 * `simulation.ts` — se usa para proyectar la reserva mínima dinámica (ver `reservaDinamicaConstruccion`,
 * engine/mantenimiento.ts, que reutiliza `calcularCostoMantenimiento`, sensible a la distancia a la capital).
 */
export function avanzarConstruccion(
  asentamiento: Asentamiento,
  zonaPoligono: Point[],
  mapa: Mapa,
  capital: Asentamiento | undefined,
  reclamos: ReclamosFuentes
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
      const yieldTrigo = EDIFICIO_CATALOGO.granja.produccionBaseTrigo * mapa.fertilidadEn(edificio.posicion) * ratioMano * factorProduccionTrigo(asentamiento);
      almacen = agregarRecurso(almacen, 'trigo', yieldTrigo);
    } else if (edificio.tipo === 'lenera') {
      // Los bosques no se agotan (Doc 1.4): la Leñera no extrae contra un stock, rinde según la densidad.
      const bosque = mapa.bosque(edificio.fuenteId);
      if (bosque) {
        const yieldMadera = EDIFICIO_CATALOGO.lenera.produccionBaseMadera * bosque.densidad * ratioMano;
        almacen = agregarRecurso(almacen, 'madera', yieldMadera);
      }
    } else {
      const extraccion = EXTRACTORES[edificio.tipo];
      if (extraccion && mapa.nodoProductivo(edificio.fuenteId)) {
        const extraido = mapa.extraer(edificio.fuenteId, extraccion.produccionBase() * ratioMano);
        almacen = agregarRecurso(almacen, extraccion.recurso, extraido);
        if (!mapa.nodoProductivo(edificio.fuenteId)) eventos.push(extraccion.mensajeAgotado);
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
    const trasNecesidades = evaluarNecesidades(asentamientoConProgreso, zonaPoligono, mapa, reserva, reclamos);
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
