// ============================================================================
// ⚠️  SOLO PARA SIMULACIÓN — NO ES PARTE DEL JUEGO REAL.
// ============================================================================
// El juego real (Doc 3.2) es 100% manual: un Tesorero/Gobernador humano decide con quién comerciar y lo
// propone desde la UI. Este módulo simula ESE mismo trabajo con un NPC virtual, para poder correr batches
// de muchos ticks y medir si la dependencia entre asentamientos (excedente de uno cubriendo el déficit de
// otro, ver `Consideraciones/Fase_0_5_Definicion_Especializacion_y_Cupos.md`) funciona de verdad, sin
// necesitar un jugador humano interactuando en cada tick.
//
// Apagado por defecto (`SIMULACION_AUTO_COMERCIO.activo = 0`, constants.ts) — activable en caliente desde
// el panel de balance. `avanzarSimulacion` (engine/simulation.ts) NUNCA llama a este módulo; el único punto
// de enganche está en `gameStore.ts`, gateado por el mismo flag (buscar "SIMULACION_AUTO_COMERCIO").
//
// PARA ELIMINAR ESTE MECANISMO POR COMPLETO: borra este archivo entero, el bloque `SIMULACION_AUTO_COMERCIO`
// de constants.ts (y su entrada en balanceConfig.ts), y la llamada gateada en gameStore.ts.
// ============================================================================

import type { AcuerdoTrueque, Asentamiento, Caravana, Faccion, RecursoTipo } from '../domain/types';
import type { Instante } from '../domain/tiempo';
import type { Mapa } from '../world/mapa';
import { CARAVANA_CATALOGO, SIMULACION_AUTO_COMERCIO } from '../constants';
import { computeTodasLasZonas } from './zones';
import { anadirEdificioManualmente, reclamosDeFuentes, RECURSO_A_EXTRACTOR, ConstruccionManualInvalidaError } from './construction';
import { aceptarTrueque, construirCaravanaComercial, proponerTrueque, CaravanaInvalidaError, TruequeInvalidoError } from './trade';
import { cupoCaravanas, tieneMercadoActivo } from './asentamientoQuery';
import { asignarCargoLocal, CargoInvalidoError } from './cargos';
import { encontrarCapital } from './mantenimiento';
import type { EstadoSimulacion } from './simulation';

/** Recursos crudos cuya escasez local es el foco del rediseño de especialización (Doc Fase_0_5) — minerales
 * y livestock, no madera/trigo (que casi todo asentamiento puede producir por sí mismo, ver diagnóstico). */
const RECURSOS_A_EQUILIBRAR: RecursoTipo[] = ['piedra', 'cobre', 'estano', 'oro', 'livestock'];
/** Recursos que un asentamiento deficitario puede ofrecer a cambio (casi cualquiera puede producir alguno
 * de estos dos, ver Doc 1.4). */
const RECURSOS_DE_PAGO: RecursoTipo[] = ['madera', 'trigo'];

function tieneExtractorActivo(asentamiento: Asentamiento, recurso: RecursoTipo): boolean {
  const tipoExtractor = RECURSO_A_EXTRACTOR[recurso];
  if (!tipoExtractor) return false;
  return asentamiento.edificios.some((e) => e.tipo === tipoExtractor && e.estado === 'activo');
}

function fraccionDisponible(asentamiento: Asentamiento, recurso: RecursoTipo): number {
  const item = asentamiento.almacen[recurso];
  if (!item || item.capacidad <= 0) return 0;
  return item.cantidad / item.capacidad;
}

/** Mejor recurso de pago que el asentamiento deficitario puede ofrecer ahora mismo, o `undefined` si ninguno
 * supera su propio colchón de seguridad. */
function mejorRecursoDePago(asentamiento: Asentamiento): RecursoTipo | undefined {
  let mejor: RecursoTipo | undefined;
  let mejorFraccion = SIMULACION_AUTO_COMERCIO.colchonExcedente;
  for (const recurso of RECURSOS_DE_PAGO) {
    const fraccion = fraccionDisponible(asentamiento, recurso);
    if (fraccion > mejorFraccion) {
      mejorFraccion = fraccion;
      mejor = recurso;
    }
  }
  return mejor;
}

function existeAcuerdoActivo(acuerdos: AcuerdoTrueque[], aId: string, bId: string, recurso: RecursoTipo): boolean {
  return acuerdos.some(
    (ac) =>
      ac.estado === 'activo' &&
      ((ac.asentamientoAId === aId && ac.asentamientoBId === bId && ac.recursoB === recurso) ||
        (ac.asentamientoBId === aId && ac.asentamientoAId === bId && ac.recursoA === recurso))
  );
}

/**
 * Asegura la infraestructura mínima para que un asentamiento pueda comerciar de verdad (Mercado + al menos
 * una caravana propia, Doc 3.12) — sin esto el trueque se propone pero nunca viaja (Doc 3.2: "si no hay
 * ninguna disponible, el envío simplemente espera"). Silenciosa si algo no se puede pagar todavía: se
 * reintenta en la siguiente invocación. También asegura un Gobernador (requisito de `anadirEdificioManualmente`)
 * y un Tesorero (requisito de `calibrarReservaManual`, ver `gameStore.ts`), usando al primer jugador
 * fundador para ambos, que ya es ciudadano de la Facción por definición.
 *
 * Reserva manual de madera (a petición del usuario — mismo mecanismo que un Tesorero humano usaría para
 * evitar que la auto-construcción se coma la madera antes de poder construir su propia caravana, Doc 4.2):
 * sin esto, `avanzarConstruccion` (Vivienda/Granja/Leñera re-disparándose casi cada tick) puede consumir toda
 * la madera disponible antes de que este módulo tenga su turno, dejando al asentamiento con Mercado activo
 * pero sin caravana para siempre — bug real observado instrumentando este mismo módulo. La reserva NO bloquea
 * la construcción MANUAL (Mercado/caravana de este módulo, "exenta a propósito", ver `construction.ts`),
 * solo protege ese margen del camino automático.
 */
function asegurarInfraestructuraComercial(
  asentamiento: Asentamiento,
  faccion: Faccion,
  caravanas: Caravana[],
  zonaPoligono: ReturnType<typeof computeTodasLasZonas>[number]['poligono'],
  mapa: Mapa,
  capital: Asentamiento | undefined,
  reclamos: ReturnType<typeof reclamosDeFuentes>,
  instante: Instante,
  contador: number
): { asentamiento: Asentamiento; caravanaNueva?: Caravana } {
  let actual = asentamiento;
  const fundador = actual.jugadoresFundadoresIds[0];

  if (!actual.cargos.gobernadorId && fundador) {
    try {
      actual = asignarCargoLocal(actual, faccion, 'gobernador', fundador);
    } catch (err) {
      if (!(err instanceof CargoInvalidoError)) throw err;
    }
  }
  if (!actual.cargos.tesoreroId && fundador) {
    try {
      actual = asignarCargoLocal(actual, faccion, 'tesorero', fundador);
    } catch (err) {
      if (!(err instanceof CargoInvalidoError)) throw err;
    }
  }

  const reservaCaravana = CARAVANA_CATALOGO.comercial.costoConstruccion.madera;
  if (actual.cargos.tesoreroId && (actual.reservaManual?.madera ?? 0) < reservaCaravana) {
    actual = { ...actual, reservaManual: { ...actual.reservaManual, madera: reservaCaravana } };
  }

  if (!actual.cargos.gobernadorId) return { asentamiento: actual };

  if (!tieneMercadoActivo(actual)) {
    try {
      actual = anadirEdificioManualmente(actual, faccion, 'gobernador', 'mercado', zonaPoligono, mapa, capital, reclamos, contador);
    } catch (err) {
      if (!(err instanceof ConstruccionManualInvalidaError)) throw err;
    }
    return { asentamiento: actual };
  }

  const propias = caravanas.filter((c) => c.tipo === 'comercial' && c.origenAsentamientoId === actual.id).length;
  if (propias < cupoCaravanas(actual)) {
    try {
      const resultado = construirCaravanaComercial(actual, caravanas, instante, contador);
      return { asentamiento: resultado.asentamiento, caravanaNueva: resultado.caravana };
    } catch (err) {
      if (!(err instanceof CaravanaInvalidaError)) throw err;
    }
  }

  return { asentamiento: actual };
}

/**
 * Un tick del NPC de comercio interno simulado: por Facción con 2+ asentamientos propios, (a) da un paso de
 * infraestructura comercial por asentamiento (Mercado/caravana, ver arriba), y (b) para cada recurso crudo
 * en `RECURSOS_A_EQUILIBRAR`, empareja un asentamiento SIN extractor activo de ese recurso (deficitario) con
 * uno CON extractor activo y stock por encima del colchón de seguridad (superavitario) de la misma Facción, y
 * propone un trueque — el deficitario paga con lo que él sí tenga de sobra (madera o trigo). No repite un
 * trueque si ya hay uno 'activo' entre el mismo par para ese recurso.
 */
export function avanzarAutoComercioSimulado(estado: EstadoSimulacion, mapa: Mapa, instante: Instante): EstadoSimulacion {
  if (!SIMULACION_AUTO_COMERCIO.activo) return estado;

  let asentamientos = [...estado.asentamientos];
  let caravanas = [...estado.caravanas];
  const acuerdosNuevos: AcuerdoTrueque[] = [];
  const zonas = computeTodasLasZonas(asentamientos);
  const reclamos = reclamosDeFuentes(asentamientos);
  let contador = 0;

  const actualizar = (id: string, siguiente: Asentamiento) => {
    asentamientos = asentamientos.map((a) => (a.id === id ? siguiente : a));
  };

  for (const faccion of estado.facciones) {
    const propios = asentamientos.filter((a) => a.faccionId === faccion.id);
    if (propios.length < 2) continue;
    const capital = encontrarCapital(faccion.id, asentamientos);

    // Paso A: infraestructura (un paso por asentamiento por tick, se completa a lo largo de varios ticks).
    for (const asentamiento of propios) {
      const zonaPoligono = zonas.find((z) => z.asentamientoId === asentamiento.id)?.poligono ?? [];
      const resultado = asegurarInfraestructuraComercial(
        asentamiento,
        faccion,
        caravanas,
        zonaPoligono,
        mapa,
        capital,
        reclamos,
        instante,
        contador++
      );
      actualizar(asentamiento.id, resultado.asentamiento);
      if (resultado.caravanaNueva) caravanas = [...caravanas, resultado.caravanaNueva];
    }

    // Paso B: emparejar déficit/superávit y proponer trueques.
    const propiosActualizados = asentamientos.filter((a) => a.faccionId === faccion.id);
    for (const recurso of RECURSOS_A_EQUILIBRAR) {
      const deficitarios = propiosActualizados.filter((a) => !tieneExtractorActivo(a, recurso));
      const superavitarios = propiosActualizados.filter(
        (a) => tieneExtractorActivo(a, recurso) && fraccionDisponible(a, recurso) > SIMULACION_AUTO_COMERCIO.colchonExcedente
      );
      if (deficitarios.length === 0 || superavitarios.length === 0) continue;

      for (const deficitario of deficitarios) {
        const socio = superavitarios.find(
          (s) => s.id !== deficitario.id && !existeAcuerdoActivo([...estado.acuerdos, ...acuerdosNuevos], deficitario.id, s.id, recurso)
        );
        if (!socio) continue;
        const pago = mejorRecursoDePago(deficitario);
        if (!pago) continue;

        try {
          const acuerdo = proponerTrueque(
            asentamientos,
            deficitario.id,
            socio.id,
            pago,
            recurso,
            SIMULACION_AUTO_COMERCIO.cantidadPorTrueque,
            SIMULACION_AUTO_COMERCIO.cantidadPorTrueque,
            instante,
            contador++
          );
          // Los DOS lados son plazas de la misma simulación automática, así que el auto-comercio contesta
          // por la receptora en el acto — pero pasando por `aceptarTrueque`, la misma función que usaría un
          // jugador, y no escribiendo `'activo'` a mano. Si el laboratorio tuviera un atajo propio,
          // mediríamos una economía que no es la del juego.
          acuerdosNuevos.push(aceptarTrueque(acuerdo, instante));
        } catch (err) {
          if (!(err instanceof TruequeInvalidoError)) throw err;
        }
      }
    }
  }

  return { ...estado, asentamientos, caravanas, acuerdos: [...estado.acuerdos, ...acuerdosNuevos] };
}
