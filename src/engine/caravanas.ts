// Revamp de caravanas (Doc 3.13, `Consideraciones/Revamp_Caravanas_Definicion.md`): una caravana `comercial`
// es una lista de carros, cada uno con su animal, y de ahí deriva capacidad y velocidad. Este módulo tiene
// las derivaciones puras; vive aparte porque lo consumen tanto `trade.ts` como `ejercitos.ts`, que no se
// importan entre sí.
//
// El motor NO sabe de jugadores ni de sesiones: aquí solo se hace aritmética sobre la caravana y los
// catálogos. La validación de residencia / propiedad de escuadrones para la escolta vive en `session/`.

import { ANIMAL_CATALOGO, CARRO_CATALOGO, VISION } from '../constants';
import type { Asentamiento, Caravana } from '../domain/types';
import { factorCapacidadCaravana } from './politicas';

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
 * Capacidad de carga de una caravana **con la política «Carga Ampliada»** de su asentamiento de origen (Doc 3.12): la que vale siempre que se
 * carga, viaje, esté enganchada a un ejército o aparcada. Si el origen ya no está en `asentamientos`, sin política.
 */
export function capacidadEfectiva(caravana: Caravana, asentamientos: readonly Asentamiento[]): number {
  const origen = asentamientos.find((a) => a.id === caravana.origenAsentamientoId);
  return capacidadCaravana(caravana) * (origen ? factorCapacidadCaravana(origen) : 1);
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
 * ¿Llama la atención? (Doc 5.12.7): una caravana comercial con al menos `VISION.caravanaGrande.carros` carros con animal, o sea, de
 * las que salen. Cuenta lo que viajaría y no el casco, así que sirve igual en `preparando` que en ruta. Una de Fundación no es grande
 * por esto: es lo que sea su columna.
 */
export function esCaravanaGrande(caravana: Caravana): boolean {
  return caravana.tipo === 'comercial' && carrosConTraccion(caravana).length >= VISION.caravanaGrande.carros;
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
