import type { AcuerdoTrueque, Asentamiento, CaminoComercial, CampamentoBandido, Caravana, Faccion, OrdenMercado, RelacionPolitica, Titulo } from '../domain/types';
import type { Mapa } from '../world/mapa';
import { computeTodasLasZonas } from './zones';
import { avanzarConstruccion, reclamosDeFuentes } from './construction';
import { consumirComida, crecerPoblacion } from './population';
import { avanzarComercio } from './trade';
import { avanzarCaravanasFundacion } from './expansion';
import { avanzarMercado } from './market';
import { avanzarPoliticas } from './politicas';
import { avanzarTributos } from './diplomacia';
import { avanzarNivelesFaccion } from './faccion';
import { avanzarMantenimientoTropas } from './tropas';
import { avanzarNivelAsentamiento, avanzarMantenimiento, encontrarCapital } from './mantenimiento';
import { avanzarReputacion } from './reputacion';
import { calcularTitulos, narrarCambiosDeTitulo } from './titulos';
import { avanzarAtaquesBandidos, avanzarSpawnBandidos } from './bandidos';

export interface EstadoSimulacion {
  asentamientos: Asentamiento[];
  facciones: Faccion[];
  caravanas: Caravana[];
  acuerdos: AcuerdoTrueque[];
  ordenes: OrdenMercado[];
  relaciones: RelacionPolitica[];
  titulos: Titulo[];
  /** Caminos comerciales (Fase 0.3, Doc 1.6) — se crean fuera del tick, al proponer trueque (ver
   * `GameStore.proponerTrueque`/`engine/caminos.ts`); el tick solo los LEE para el bonus de velocidad de
   * caravana (`engine/trade.ts`), nunca los modifica. */
  caminos: CaminoComercial[];
  /** Campamentos de bandidos activos (Doc 1.9) — ver `engine/bandidos.ts`. */
  campamentosBandidos: CampamentoBandido[];
  /** Tick a partir del cual puede aparecer un campamento nuevo si hay menos de `maximoSimultaneos` activos
   * (Doc 1.9) — se adelanta cada vez que un jugador destruye uno (`GameStore.atacarCampamentoBandidos`). */
  bandidosProximoSpawnTick: number;
}

export interface ResultadoTick extends EstadoSimulacion {
  eventos: string[];
}

/**
 * Avanza un tick de simulación: crecimiento de zonas, construcción/producción, políticas, tropas, nivel y
 * mantenimiento (por asentamiento) y, a nivel global, comercio, mercado, tributos, nivel de Facción,
 * reputación y títulos dinámicos (Sprint 6, cierre).
 */
export function avanzarSimulacion(estado: EstadoSimulacion, mapa: Mapa, tickActual: number): ResultadoTick {
  // El crecimiento de la zona de influencia ya no es puramente temporal (rediseño Doc 1.2, a petición del
  // usuario): ahora se dispara al completarse cada edificio, dentro de `avanzarConstruccion`.
  const crecidos = estado.asentamientos;
  const zonas = computeTodasLasZonas(crecidos);
  const eventos: string[] = [];

  const capitalesPorFaccion = new Map(
    [...new Set(crecidos.map((a) => a.faccionId))].map((faccionId) => [faccionId, encontrarCapital(faccionId, crecidos)])
  );

  // Fuentes del mapa ya tomadas por CUALQUIER asentamiento (ver `reclamosDeFuentes`): se calcula una vez y
  // se va actualizando según cada asentamiento compromete obra, para que dos que se procesan en el mismo
  // tick no se adjudiquen el mismo yacimiento.
  const reclamos = reclamosDeFuentes(crecidos);

  const procesados = crecidos.map((asentamiento) => {
    const zona = zonas.find((z) => z.asentamientoId === asentamiento.id);
    const { asentamiento: trasConstruccion, eventos: eventosConstruccion } = avanzarConstruccion(
      asentamiento,
      zona?.poligono ?? [],
      mapa,
      capitalesPorFaccion.get(asentamiento.faccionId),
      reclamos
    );

    const { asentamiento: trasPoliticas, eventos: eventosPoliticas } = avanzarPoliticas(trasConstruccion, tickActual);
    const { asentamiento: trasTropas, eventos: eventosTropas } = avanzarMantenimientoTropas(trasPoliticas);
    const { asentamiento: trasNivel, eventos: eventosNivel } = avanzarNivelAsentamiento(trasTropas);
    const trasConsumo = consumirComida(trasNivel);
    const { poblacion, eventos: eventosPoblacion } = crecerPoblacion(trasConsumo);
    const conPoblacion = { ...trasConsumo, poblacion };

    const capital = capitalesPorFaccion.get(asentamiento.faccionId);
    const { asentamiento: trasMantenimiento, eventos: eventosMantenimiento, destruido } = avanzarMantenimiento(conPoblacion, capital, tickActual);

    for (const e of [...eventosConstruccion, ...eventosPoliticas, ...eventosTropas, ...eventosNivel, ...eventosPoblacion, ...eventosMantenimiento]) {
      eventos.push(`${asentamiento.id}: ${e}`);
    }

    return { asentamiento: trasMantenimiento, destruido };
  });

  // Ruinas por abandono/mal mantenimiento (Doc 4.5): el asentamiento se elimina, su zona queda libre.
  const actualizados = procesados.filter((p) => !p.destruido).map((p) => p.asentamiento);

  const trasComercio = avanzarComercio(actualizados, estado.facciones, estado.caravanas, estado.acuerdos, mapa, estado.caminos, zonas, tickActual);
  eventos.push(...trasComercio.eventos);

  // Caravanas de Fundación (Doc 1.8): expanden una Facción más allá de su primer asentamiento — se avanzan
  // aparte de las comerciales (destino es un punto del mapa, no un asentamiento existente).
  const trasExpansion = avanzarCaravanasFundacion(trasComercio.caravanas, mapa, trasComercio.facciones, trasComercio.asentamientos, tickActual);
  eventos.push(...trasExpansion.eventos);

  // Regeneración de yacimientos agotados (a petición del usuario): muta `mapa` directamente, mismo patrón
  // que `mapa.extraer` dentro de `avanzarConstruccion` más arriba en este mismo tick.
  eventos.push(...mapa.avanzarRegeneracion(tickActual));

  // Campamentos de bandidos (Doc 1.9): spawn/respawn primero, después atacan cualquier caravana ya movida
  // este tick (comercial o de fundación) que pase cerca — mismo orden que el resto del tick, sobre posiciones
  // ya actualizadas.
  const trasSpawnBandidos = avanzarSpawnBandidos(estado.campamentosBandidos, estado.bandidosProximoSpawnTick, zonas, trasExpansion.asentamientos, mapa, tickActual);
  eventos.push(...trasSpawnBandidos.eventos);
  const trasAtaquesBandidos = avanzarAtaquesBandidos(trasSpawnBandidos.campamentos, trasExpansion.caravanas);
  eventos.push(...trasAtaquesBandidos.eventos);

  const trasMercado = avanzarMercado(trasExpansion.asentamientos, estado.ordenes);
  eventos.push(...trasMercado.eventos);

  const trasTributos = avanzarTributos(estado.relaciones, trasMercado.asentamientos);
  eventos.push(...trasTributos.eventos);

  const trasNivelFaccion = avanzarNivelesFaccion(trasExpansion.facciones, trasTributos.asentamientos);
  eventos.push(...trasNivelFaccion.eventos);

  const faccionesFinal = avanzarReputacion(trasNivelFaccion.facciones, estado.relaciones);

  const titulosActuales = calcularTitulos(faccionesFinal, trasTributos.asentamientos, estado.relaciones);
  eventos.push(...narrarCambiosDeTitulo(estado.titulos, titulosActuales, faccionesFinal));

  return {
    asentamientos: trasTributos.asentamientos,
    facciones: faccionesFinal,
    caravanas: trasAtaquesBandidos.caravanas,
    acuerdos: trasComercio.acuerdos,
    ordenes: trasMercado.ordenes,
    relaciones: estado.relaciones,
    titulos: titulosActuales,
    caminos: estado.caminos,
    campamentosBandidos: trasSpawnBandidos.campamentos,
    bandidosProximoSpawnTick: estado.bandidosProximoSpawnTick,
    eventos,
  };
}
