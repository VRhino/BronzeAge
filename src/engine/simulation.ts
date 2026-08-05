import type { AcuerdoTrueque, Asentamiento, Caravana, Faccion, OrdenMercado, RelacionPolitica, Titulo, World } from '../domain/types';
import { avanzarCrecimientoZonas } from './settlement';
import { computeTodasLasZonas } from './zones';
import { avanzarConstruccion } from './construction';
import { consumirComida, crecerPoblacion } from './population';
import { avanzarComercio } from './trade';
import { avanzarMercado } from './market';
import { avanzarPoliticas } from './politicas';
import { avanzarTributos } from './diplomacia';
import { avanzarNivelesFaccion } from './faccion';
import { avanzarMantenimientoTropas } from './tropas';
import { avanzarNivelAsentamiento, avanzarMantenimiento, encontrarCapital } from './mantenimiento';
import { avanzarReputacion } from './reputacion';
import { calcularTitulos, narrarCambiosDeTitulo } from './titulos';

export interface EstadoSimulacion {
  asentamientos: Asentamiento[];
  facciones: Faccion[];
  caravanas: Caravana[];
  acuerdos: AcuerdoTrueque[];
  ordenes: OrdenMercado[];
  relaciones: RelacionPolitica[];
  titulos: Titulo[];
}

export interface ResultadoTick extends EstadoSimulacion {
  eventos: string[];
}

/**
 * Avanza un tick de simulación: crecimiento de zonas, construcción/producción, políticas, tropas, nivel y
 * mantenimiento (por asentamiento) y, a nivel global, comercio, mercado, tributos, nivel de Facción,
 * reputación y títulos dinámicos (Sprint 6, cierre).
 */
export function avanzarSimulacion(estado: EstadoSimulacion, world: World, tickActual: number): ResultadoTick {
  const crecidos = avanzarCrecimientoZonas(estado.asentamientos);
  const zonas = computeTodasLasZonas(crecidos);
  const eventos: string[] = [];

  const capitalesPorFaccion = new Map(
    [...new Set(crecidos.map((a) => a.faccionId))].map((faccionId) => [faccionId, encontrarCapital(faccionId, crecidos)])
  );

  const procesados = crecidos.map((asentamiento) => {
    const zona = zonas.find((z) => z.asentamientoId === asentamiento.id);
    const { asentamiento: trasConstruccion, eventos: eventosConstruccion } = avanzarConstruccion(
      asentamiento,
      zona?.poligono ?? [],
      world
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

  const trasComercio = avanzarComercio(actualizados, estado.facciones, estado.caravanas, estado.acuerdos, tickActual);
  eventos.push(...trasComercio.eventos);

  const trasMercado = avanzarMercado(trasComercio.asentamientos, estado.ordenes);
  eventos.push(...trasMercado.eventos);

  const trasTributos = avanzarTributos(estado.relaciones, trasMercado.asentamientos);
  eventos.push(...trasTributos.eventos);

  const trasNivelFaccion = avanzarNivelesFaccion(trasComercio.facciones, trasTributos.asentamientos);
  eventos.push(...trasNivelFaccion.eventos);

  const faccionesFinal = avanzarReputacion(trasNivelFaccion.facciones, estado.relaciones);

  const titulosActuales = calcularTitulos(faccionesFinal, trasTributos.asentamientos, estado.relaciones);
  eventos.push(...narrarCambiosDeTitulo(estado.titulos, titulosActuales, faccionesFinal));

  return {
    asentamientos: trasTributos.asentamientos,
    facciones: faccionesFinal,
    caravanas: trasComercio.caravanas,
    acuerdos: trasComercio.acuerdos,
    ordenes: trasMercado.ordenes,
    relaciones: estado.relaciones,
    titulos: titulosActuales,
    eventos,
  };
}
