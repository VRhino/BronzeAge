// Revamp de caravanas (Doc 3.13, `Consideraciones/Revamp_Caravanas_Definicion.md`): una caravana `comercial`
// es una lista de carros, cada uno con su animal, y de ahí deriva capacidad y velocidad. Este módulo tiene
// las derivaciones puras; vive aparte porque lo consumen tanto `trade.ts` como `ejercitos.ts`, que no se
// importan entre sí.
//
// El motor NO sabe de jugadores ni de sesiones: aquí solo se hace aritmética sobre la caravana y los
// catálogos. La validación de residencia / propiedad de escuadrones para la escolta vive en `session/`.

import { ANIMAL_CATALOGO, CARRO_CATALOGO } from '../constants';
import type { Caravana, Escuadron } from '../domain/types';

/** Escolta sin héroe (Doc 3.13.4) que vuelve a la guarnición de su origen tras un combate — el llamador
 * (`simulation.ts` / `avanzarEjercitos`) la funde con la guarnición vía `devolverEscoltaAGuarnicion`. */
export interface EscoltaDevuelta {
  asentamientoId: string;
  escuadrones: Escuadron[];
}

/** Los carros que llevan animal — los únicos que viajan y cuentan capacidad (Doc 3.13.1). */
function carrosConTraccion(caravana: Caravana) {
  return (caravana.carros ?? []).filter((c) => c.animal !== undefined);
}

/**
 * Capacidad de carga de una caravana (Doc 3.13.1): suma de `capacidadBase × factorCarga` sobre los carros con
 * animal. Sin carros con tracción → 0.
 *
 * NO aplica los multiplicadores de política (`carga_ampliada`): eso lo hace el llamador, que es quien tiene
 * el asentamiento de origen.
 */
export function capacidadCaravana(caravana: Caravana): number {
  return carrosConTraccion(caravana).reduce(
    (suma, c) => suma + CARRO_CATALOGO[c.tipoCarro].capacidadBase * ANIMAL_CATALOGO[c.animal!].factorCarga,
    0
  );
}

/**
 * Velocidad de una caravana (Doc 3.13.1): la del animal más lento. Los carros no capean velocidad todavía
 * (todos los tipos manejan igual). Con carros pero sin ningún animal → 0: una caravana así no puede salir.
 *
 * NO aplica los multiplicadores de política (`rutas_rapidas`): eso lo hace el llamador.
 */
export function velocidadCaravana(caravana: Caravana): number {
  const conTraccion = carrosConTraccion(caravana);
  if (conTraccion.length === 0) return 0;
  return Math.min(...conTraccion.map((c) => ANIMAL_CATALOGO[c.animal!].velocidad));
}

/**
 * Coste de la caravana por defecto (1 carro básico + 1 buey), sumado por recurso. Es lo que cuesta montar una
 * caravana lista para viajar desde cero, y lo que reserva el NPC de laboratorio antes de intentarlo
 * (`simulacionAutoComercio.ts`). Hoy son 50 madera — el mismo número que costaba crear una caravana antes del
 * revamp, a propósito (ancla de calibración, Doc 3.13.2).
 */
export function costoCaravanaPorDefecto(): Record<string, number> {
  const total: Record<string, number> = {};
  for (const costo of [CARRO_CATALOGO.basico.costo, ANIMAL_CATALOGO.buey.costo] as Record<string, number>[]) {
    for (const [recurso, cantidad] of Object.entries(costo)) total[recurso] = (total[recurso] ?? 0) + cantidad;
  }
  return total;
}

/**
 * Devuelve escuadrones-escolta (Doc 3.13.4) a la guarnición de un asentamiento, fundiéndolos con el escuadrón
 * del mismo jugador y tropa si ya existe — invariante "un jugador tiene UN escuadrón por tropa" (types.ts).
 * Lo usan la vuelta normal de la caravana (`avanzarCaravanas`), la cancelación de preparación y el regreso de
 * la escolta tras un combate perdido (`avanzarAtaquesBandidos` / `resolverEncuentros`).
 */
export function devolverEscoltaAGuarnicion(guarnicion: readonly Escuadron[], escolta: readonly Escuadron[]): Escuadron[] {
  const resultado = guarnicion.map((e) => ({ ...e }));
  for (const s of escolta) {
    const existente = resultado.find((e) => e.heroeId === s.heroeId && e.tropaId === s.tropaId);
    if (existente) {
      existente.cantidad += s.cantidad;
      existente.veterania = Math.max(existente.veterania, s.veterania);
      existente.moral = Math.min(existente.moral, s.moral);
      if (s.heridoHasta !== undefined) existente.heridoHasta = s.heridoHasta;
    } else {
      resultado.push({ ...s });
    }
  }
  return resultado;
}
