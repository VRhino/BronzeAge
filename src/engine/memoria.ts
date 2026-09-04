// Lo que cada Facción RECUERDA del mundo (niebla de guerra, Paso 2 —
// `Consideraciones/Niebla_De_Guerra_Definicion.md` §4).
//
// La regla es una sola, y de ella salen los tres estados que ve el jugador: **lo que alcanzas a ver este tick
// queda grabado**. Ver algo ES conocerlo; comerciar no es una vía aparte, es otra forma de verlo un instante.
//
// - "Nunca lo he visto" -> la celda no está en `exploracion`.
// - "Lo vi antes" -> está en `exploracion`, y de una plaza queda además su última `FichaConocida`.
// - "Lo estoy viendo" -> no se guarda: se deriva en cada proyección (`seVeAhora`), porque guardar lo
//   derivable es la clase de error que este proyecto lleva evitando desde el principio.
//
// **Por Facción y no por jugador**, por coherencia antes que por coste: la proyección ya enseña la Facción
// propia COMPLETA a cualquiera de sus ciudadanos, así que un miembro que no supiera lo que otro ya exploró
// sería incoherente con todo lo demás. Que además sea cinco veces más barato de guardar es un extra.
import type { Asentamiento, Ejercito, Faccion, Point } from '../domain/types';
import type { Instante } from '../domain/tiempo';
import { VISION } from '../constants';
import { distancia } from '../world/geometria';
import { marcarVisto, rejillaDe, SIN_EXPLORAR, type Exploracion } from './exploracion';

/**
 * La última foto de una plaza ajena: su FICHA (decisión del usuario, 2026-09-04) y cuándo se tomó.
 *
 * `conocidoEn` es lo que permite que la información vieja se delate sola —"última información: hace 3
 * horas"— en vez de caducar a un plazo que nadie ha medido. Los mismos campos que `AsentamientoAvistado`,
 * más el instante: lo que se recuerda puede estar rancio, lo que se ve no.
 */
export interface FichaConocida {
  asentamientoId: string;
  nombre?: string;
  faccionId: string;
  posicion: Point;
  nivel: number;
  conocidoEn: Instante;
}

export interface MemoriaFaccion {
  /** Terreno pisado o vigilado alguna vez, en rejilla (ver `engine/exploracion.ts`). */
  exploracion: Exploracion;
  /** Última ficha de cada plaza AJENA vista, por `asentamientoId`. Las propias no entran: viajan completas
   * en la proyección, y duplicarlas aquí sería guardar dos veces el mismo hecho. */
  asentamientos: Record<string, FichaConocida>;
}

export const MEMORIA_VACIA: MemoriaFaccion = { exploracion: SIN_EXPLORAR, asentamientos: {} };

/** Un ojo: dónde está y cuánto alcanza. Una plaza vigila su radio más el margen; una columna, lo suyo. */
interface Ojo {
  posicion: Point;
  alcance: number;
}

function ojosDe(faccionId: string, asentamientos: readonly Asentamiento[], ejercitos: readonly Ejercito[]): Ojo[] {
  const ojos: Ojo[] = [];
  for (const a of asentamientos) {
    if (a.faccionId !== faccionId) continue;
    // Contra el DISCO, no contra el polígono de la zona: ese recorte es político, no óptico (ver `seVeAhora`).
    ojos.push({ posicion: a.posicion, alcance: a.radioPotencial + VISION.margenAsentamiento });
  }
  for (const e of ejercitos) {
    if (e.faccionId !== faccionId) continue;
    ojos.push({ posicion: e.posicionActual, alcance: VISION.ejercito });
  }
  return ojos;
}

export interface ContextoMemoria {
  asentamientos: readonly Asentamiento[];
  ejercitos: readonly Ejercito[];
  facciones: readonly Faccion[];
  limites: { ancho: number; alto: number };
  instante: Instante;
}

/**
 * Graba en la memoria de cada Facción lo que sus plazas y sus columnas alcanzan a ver AHORA. Sin borrar nada:
 * **la memoria solo crece.**
 *
 * Se llama al FINAL del tick, cuando los ejércitos ya se han movido: lo que se graba es dónde acabaron, no de
 * dónde salieron.
 *
 * Un jugador HUÉRFANO (Doc 5.4) no aporta memoria a nadie: no tiene Facción a la que grabarla. Su columna
 * sigue viendo en vivo —`proyectarParaJugador` lo resuelve por escuadrón, no por Facción— pero lo que ve no
 * queda registrado en ningún sitio hasta que vuelva a tener bandera.
 */
export function grabarLoVisto(
  memoria: Readonly<Record<string, MemoriaFaccion>>,
  contexto: ContextoMemoria
): Record<string, MemoriaFaccion> {
  const rejilla = rejillaDe(contexto.limites);
  const salida: Record<string, MemoriaFaccion> = { ...memoria };

  for (const faccion of contexto.facciones) {
    const ojos = ojosDe(faccion.id, contexto.asentamientos, contexto.ejercitos);
    if (ojos.length === 0) continue;

    const previa = salida[faccion.id] ?? MEMORIA_VACIA;
    let exploracion = previa.exploracion;
    for (const ojo of ojos) exploracion = marcarVisto(exploracion, rejilla, ojo.posicion, ojo.alcance);

    let asentamientos = previa.asentamientos;
    for (const a of contexto.asentamientos) {
      if (a.faccionId === faccion.id) continue;
      if (!ojos.some((ojo) => distancia(a.posicion, ojo.posicion) <= ojo.alcance)) continue;
      if (asentamientos === previa.asentamientos) asentamientos = { ...previa.asentamientos };
      asentamientos[a.id] = {
        asentamientoId: a.id,
        nombre: a.nombre,
        faccionId: a.faccionId,
        posicion: a.posicion,
        nivel: a.nivel,
        conocidoEn: contexto.instante,
      };
    }

    // Se devuelve la MISMA memoria si no hubo novedad, para que un tick tranquilo no reescriba el estado.
    if (exploracion === previa.exploracion && asentamientos === previa.asentamientos) {
      if (salida[faccion.id] !== undefined) continue;
    }
    salida[faccion.id] = { exploracion, asentamientos };
  }

  return salida;
}
