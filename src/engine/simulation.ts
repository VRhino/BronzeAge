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
import { avanzarNivelesFaccion, aplicarAjustesExperiencia, calcularCupoNivel, type AjusteExperiencia } from './faccion';
import { NIVEL_FACCION } from '../constants';
import { avanzarMantenimientoTropas } from './tropas';
import { avanzarNivelAsentamiento, avanzarMantenimiento, encontrarCapital } from './mantenimiento';
import { nivelActualDe } from './asentamientoQuery';
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

  // Doc Fase_0_5 §8: experiencia de Facción por construcción — se acumula aquí (un asentamiento por vez, sabe
  // su propia `faccionId`) y se aplica junto al resto de ajustes de experiencia más abajo en el tick.
  const ajustesExperiencia: AjusteExperiencia[] = [];

  // Doc Fase_0_5 §5: cupo de asentamientos en nivel 2/3 por Facción, derivado de su nivel actual (el de
  // INICIO de este tick — se recalcula al final vía XP, ver más abajo). Se parte de cuántos asentamientos YA
  // ocupan cada nivel — contra `nivelActual` (operativo), NO `nivel` (nivelAlcanzado, monótono): así una
  // ciudad hundida por mal mantenimiento libera su cupo de verdad al degradarse, en vez de bloquearlo para
  // siempre solo porque alguna vez llegó a ese nivel (bug detectado por el consejo LLM — el comentario de
  // `CUPO_NIVEL_ASENTAMIENTO` en constants.ts ya prometía esta liberación, el código no la cumplía). Se va
  // consumiendo/liberando según se conceden promociones dentro de este mismo tick (subir de 2 a 3 libera el
  // cupo de 2 que se abandona, disponible para otro asentamiento propio en la misma pasada).
  const cupoRestante = new Map<string, number>();
  for (const faccion of estado.facciones) {
    const propios = crecidos.filter((a) => a.faccionId === faccion.id);
    cupoRestante.set(`${faccion.id}:2`, calcularCupoNivel(faccion.nivel, 2) - propios.filter((a) => nivelActualDe(a) === 2).length);
    cupoRestante.set(`${faccion.id}:3`, calcularCupoNivel(faccion.nivel, 3) - propios.filter((a) => nivelActualDe(a) === 3).length);
  }

  const procesados = crecidos.map((asentamiento) => {
    const zona = zonas.find((z) => z.asentamientoId === asentamiento.id);
    const { asentamiento: trasConstruccion, eventos: eventosConstruccion, edificiosCompletados } = avanzarConstruccion(
      asentamiento,
      zona?.poligono ?? [],
      mapa,
      capitalesPorFaccion.get(asentamiento.faccionId),
      reclamos
    );
    if (edificiosCompletados > 0) {
      ajustesExperiencia.push({
        faccionId: asentamiento.faccionId,
        delta: edificiosCompletados * NIVEL_FACCION.xp.edificioCompletado,
        razon: 'edificio completado',
      });
    }

    const { asentamiento: trasPoliticas, eventos: eventosPoliticas } = avanzarPoliticas(trasConstruccion, tickActual);
    const { asentamiento: trasTropas, eventos: eventosTropas } = avanzarMantenimientoTropas(trasPoliticas);
    // `avanzarNivelAsentamiento` sube de a un escalón por llamada, en orden creciente — para llegar a pedir
    // cupo de nivel 3, el asentamiento tuvo que pasar por (y consumir) el cupo de nivel 2 primero, sea porque
    // ya estaba ahí desde antes de este tick, o porque acaba de conseguirlo en la llamada anterior de este
    // mismo bucle. En ambos casos toca liberar ese cupo de 2 al conceder el de 3 — nunca hace falta mirar de
    // dónde venía, el orden de las llamadas ya lo garantiza (Doc Fase_0_5 §5).
    const tieneCupoParaNivel = (nivelObjetivo: number): boolean => {
      // CUPO_NIVEL_ASENTAMIENTO (Doc Fase_0_5 §5) solo tiene curva definida para nivel 2 y 3 de asentamiento
      // — Doc Fase_0_6 sube el tope de nivel a 5 pero deliberadamente NO extiende esta curva todavía (queda
      // pendiente de que el usuario defina cupos para 4/5 en una pasada aparte). Sin este `> 3` los niveles
      // 4/5 leerían `cupoRestante.get(...)` como `undefined ?? 0` y NUNCA podrían subir — bloqueo silencioso
      // detectado por el consejo LLM antes de implementar. Niveles 4/5 quedan sin cupo (ilimitados) mientras
      // tanto, igual que nivel 1.
      if (nivelObjetivo < 2 || nivelObjetivo > 3) return true;
      const clave = `${asentamiento.faccionId}:${nivelObjetivo}`;
      const libre = cupoRestante.get(clave) ?? 0;
      if (libre <= 0) return false;
      cupoRestante.set(clave, libre - 1);
      if (nivelObjetivo === 3) {
        const claveN2 = `${asentamiento.faccionId}:2`;
        cupoRestante.set(claveN2, (cupoRestante.get(claveN2) ?? 0) + 1);
      }
      return true;
    };
    const { asentamiento: trasNivel, eventos: eventosNivel } = avanzarNivelAsentamiento(trasTropas, tieneCupoParaNivel);
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

  // Doc Fase_0_5 §8: se aplica la XP de construcción acumulada arriba junto a la del resto del tick (combate/
  // caravanas, aplicadas ya directamente sobre `facciones` en `engine/combate.ts`) antes de recalcular nivel.
  const faccionesConXp = aplicarAjustesExperiencia(trasExpansion.facciones, ajustesExperiencia);
  const trasNivelFaccion = avanzarNivelesFaccion(faccionesConXp);
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
