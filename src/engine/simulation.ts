import type { AcuerdoTrueque, Asentamiento, CaminoComercial, CampamentoBandido, Caravana, Ejercito, Faccion, Jugador, OrdenMercado, RelacionPolitica, Titulo } from '../domain/types';
import type { EventoCrudo, EventoDominio } from '../domain/eventos';
import type { Instante } from '../domain/tiempo';
import type { EstadoMapa, Mapa } from '../world/mapa';
import type { RandomFn } from '../worldgen';
import { computeTodasLasZonas } from './zones';
import { avanzarConstruccion, reclamosDeFuentes } from './construction';
import { avanzarNutricionPoblacion, crecerPoblacion, recaudacionOro } from './population';
import { agregarRecurso } from './almacen';
import { avanzarComercio } from './trade';
import { devolverEscoltaAGuarnicion } from './caravanas';
import { avanzarCaravanasFundacion } from './expansion';
import { caducarOrdenes } from './market';
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
import { avanzarEjercitos } from './ejercitos';
import { grabarLoVisto, type MemoriaFaccion } from './memoria';
import { grabarExploracionPersonal } from './ubicacion';

export interface EstadoSimulacion {
  asentamientos: Asentamiento[];
  facciones: Faccion[];
  caravanas: Caravana[];
  /** Ejércitos en campaña (Doc 5.12) — los mueve `avanzarEjercitos`, al final de la cadena del tick. */
  ejercitos: Ejercito[];
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
  /** Instante de mundo a partir del cual puede aparecer un campamento nuevo si hay menos de
   * `maximoSimultaneos` activos (Doc 1.9) — se adelanta cada vez que un jugador destruye uno. */
  bandidosProximoSpawnEn: Instante;
  /** Lo que cada Facción RECUERDA del mundo (niebla de guerra — ver `engine/memoria.ts`), por `faccionId`.
   * Una Facción ausente no ha visto nada todavía, así que las partidas guardadas antes de la mecánica no
   * necesitan migración. */
  memoriaPorFaccion: Record<string, MemoriaFaccion>;
  /** Quien juega (Doc 1.10). El tick solo lo toca para grabar `exploracionPersonal` de quien aun no tiene
   * bandera (`grabarExploracionPersonal`) — todo lo demas de un `Jugador` lo escribe un comando, no el tick. */
  jugadores: Jugador[];
}

/**
 * Contexto de un avance de simulación (Docs/Arquitectura/2_Estudio..., Fase A punto 3: "Introducir contexto de
 * partida para RNG e IDs"). Agrupa todo lo que el motor necesita del EXTERIOR para resolver un avance y que no
 * es estado de juego: la unidad temporal, el momento de simulación y la fuente de aleatoriedad.
 *
 * Existe para que el motor no lea nunca por su cuenta ni el reloj (`Date.now()`) ni la aleatoriedad global
 * (`Math.random()`): ambos son responsabilidad de quien gobierna la partida (`GameSession` en el backend), y
 * mantenerlos inyectados es lo que hace la simulación reproducible (ver `__tests__/determinismo.test.ts`).
 *
 * Fase D cerrada: el `tick` (unidad interna del motor) ya no forma parte de este contexto — `instante` es la
 * única referencia temporal, `momento` su forma ISO para fechar eventos.
 */
export interface ContextoSimulacion {
  /** Instante de MUNDO de este avance (Fase D / doc 10): con lo que se fechan y comparan los campos
   * `*En: Instante` de las entidades (`fundadoEn`, `expiraEn`, `heridoHasta`…). */
  instante: Instante;
  /** El mismo instante en ISO 8601, para fechar los eventos que viajan al cliente (`EventoDominio.momento`).
   * Redundante con `instante` a propósito — el núcleo puro no puede construir un `Date` (`isoDeInstante` vive
   * en `session/`), así que quien avanza la simulación lo pasa ya formateado. */
  momento: string;
  /** Fuente de aleatoriedad de la simulación (población, combate, bandidos). */
  rng: RandomFn;
}

export interface ResultadoTick extends EstadoSimulacion {
  /**
   * Estado del mapa tras el tick (yacimientos extraídos y calendario de regeneración).
   *
   * Sale por aquí, y no como efecto lateral sobre la fachada `Mapa` que se pasó, porque el tick es el único
   * avance que toca el mapa (`extraer` en `avanzarConstruccion`, `avanzarRegeneracion` más abajo) y sin esto
   * el `ResultadoTick` no bastaba para reconstruir la partida: quien descartara el resultado —un fallo al
   * persistir, en la Fase B3— se quedaba igualmente con los yacimientos vaciados. El llamador es quien
   * decide adoptarlo (ver `session/comandos/avanzarTick.ts`).
   */
  estadoMapa: EstadoMapa;
  /** Eventos del tick en forma estructurada (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase A5 — 13/13
   * subsistemas migrados). Único contrato de salida del tick: el log en texto que muestra la interfaz sale
   * de `mensaje` (ver `exito()` en `session/comandos/tipos.ts`), así que no hace falta un array paralelo. */
  eventosDominio: EventoDominio[];
}

/** Añade el contexto que un subsistema no conoce (`momento`, `asentamientoId`) a lo que ya produjo — un
 * `string` se envuelve como `codigo: 'legado'` (subsistema todavía sin migrar); un evento ya migrado
 * conserva su `codigo`/`payload` tal cual (ver `EventoCrudo`).
 *
 * La atribución por lotes (`asentamientoId`) sirve a la mayoría de subsistemas porque este bucle ya va
 * asentamiento a asentamiento. Un subsistema global que itera sobre otra cosa —los ejércitos, cada uno con
 * su propio origen— la trae en el evento, y esa gana: ver `EventoCrudo`. */
function comoEventosDominio(eventos: EventoCrudo[], contexto: ContextoSimulacion, asentamientoId?: string): EventoDominio[] {
  const { momento } = contexto;
  return eventos.map((evento) => {
    if (typeof evento === 'string') return { codigo: 'legado', mensaje: evento, momento, asentamientoId };
    return { ...evento, momento, asentamientoId: evento.asentamientoId ?? asentamientoId };
  });
}

/**
 * Avanza un tick de simulación: crecimiento de zonas, construcción/producción, políticas, tropas, nivel y
 * mantenimiento (por asentamiento) y, a nivel global, comercio, mercado, tributos, nivel de Facción,
 * reputación y títulos dinámicos (Sprint 6, cierre).
 */
export function avanzarSimulacion(estado: EstadoSimulacion, mapa: Mapa, contexto: ContextoSimulacion): ResultadoTick {
  const { instante, rng } = contexto;
  // El crecimiento de la zona de influencia ya no es puramente temporal (rediseño Doc 1.2, a petición del
  // usuario): ahora se dispara al completarse cada edificio, dentro de `avanzarConstruccion`.
  const crecidos = estado.asentamientos;
  const zonas = computeTodasLasZonas(crecidos);
  const eventosDominio: EventoDominio[] = [];

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
      reclamos,
      instante
    );
    if (edificiosCompletados > 0) {
      ajustesExperiencia.push({
        faccionId: asentamiento.faccionId,
        delta: edificiosCompletados * NIVEL_FACCION.xp.edificioCompletado,
        razon: 'edificio completado',
      });
    }

    const { asentamiento: trasPoliticas, eventos: eventosPoliticas } = avanzarPoliticas(trasConstruccion, instante);
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
    const { asentamiento: trasNivel, eventos: eventosNivel } = avanzarNivelAsentamiento(trasPoliticas, tieneCupoParaNivel);
    // Población COME ANTES que Tropas (a petición del usuario — mano de obra/reclutamiento a futuro con
    // jugadores reales): antes el orden era al revés y las Tropas se llevaban su ración aseguradas mientras
    // la Población civil se quedaba con lo que sobrara. Los civiles son quienes producen (trabajan Granja/
    // Cantera/Fundición/...); las Tropas no producen nada. Invertido así, bajo escasez sostenida la moral
    // militar colapsa y empieza la deserción (`avanzarMantenimientoTropas`, `engine/tropas.ts`) MUCHO antes
    // de que la nutrición civil llegue a comprometerse — para que los civiles se quedaran sin nada, la
    // producción tendría que caer por debajo de SOLO su propio consumo, un escalón de escasez peor que el
    // que ya habría vaciado el ejército. El shock lo absorbe la parte del sistema que no es productiva antes
    // de tocar la que sí lo es. Ver `Consideraciones/NPC_Gobernanza_Facciones_Controladas.md` §"Abierto".
    const { asentamiento: trasNutricion, eventos: eventosNutricion } = avanzarNutricionPoblacion(trasNivel);
    const { asentamiento: trasTropas, eventos: eventosTropas } = avanzarMantenimientoTropas(trasNutricion);
    const { poblacion, eventos: eventosPoblacion } = crecerPoblacion(trasTropas, rng);
    // Recaudación de oro por población (Doc 4.1, bloque "economía del oro"): se suma DESPUÉS de crecer (recauda
    // sobre la población de este tick) y ANTES de `avanzarMantenimiento` (que el oro recién recaudado pueda
    // cubrir el mantenimiento del mismo tick). Respeta la capacidad de almacén, igual que la producción de mina.
    const conPoblacion = {
      ...trasTropas,
      poblacion,
      almacen: agregarRecurso(trasTropas.almacen, 'oro', recaudacionOro({ ...trasTropas, poblacion })),
    };

    const capital = capitalesPorFaccion.get(asentamiento.faccionId);
    const { asentamiento: trasMantenimiento, eventos: eventosMantenimiento, destruido } = avanzarMantenimiento(conPoblacion, capital, instante);

    const eventosAsentamiento = [
      ...eventosConstruccion,
      ...eventosPoliticas,
      ...eventosTropas,
      ...eventosNivel,
      ...eventosNutricion,
      ...eventosPoblacion,
      ...eventosMantenimiento,
    ];
    eventosDominio.push(...comoEventosDominio(eventosAsentamiento, contexto, asentamiento.id));

    return { asentamiento: trasMantenimiento, destruido };
  });

  // Ruinas por abandono/mal mantenimiento (Doc 4.5): el asentamiento se elimina, su zona queda libre.
  const actualizados = procesados.filter((p) => !p.destruido).map((p) => p.asentamiento);

  const trasComercio = avanzarComercio(actualizados, estado.facciones, estado.caravanas, estado.acuerdos, mapa, estado.caminos, instante);
  eventosDominio.push(...comoEventosDominio(trasComercio.eventos, contexto));

  // Caravanas de Fundación (Doc 1.8): expanden una Facción más allá de su primer asentamiento — se avanzan
  // aparte de las comerciales (destino es un punto del mapa, no un asentamiento existente).
  const trasExpansion = avanzarCaravanasFundacion(trasComercio.caravanas, mapa, trasComercio.facciones, trasComercio.asentamientos, instante);
  eventosDominio.push(...comoEventosDominio(trasExpansion.eventos, contexto));

  // Regeneración de yacimientos agotados (a petición del usuario): escribe en la fachada `mapa`, mismo patrón
  // que `mapa.extraer` dentro de `avanzarConstruccion` más arriba en este mismo tick. La fachada trabaja
  // sobre su propia copia del estado del mapa, que sale de aquí en `ResultadoTick.estadoMapa`.
  const eventosRegeneracion = mapa.avanzarRegeneracion(instante);
  eventosDominio.push(...comoEventosDominio(eventosRegeneracion, contexto));

  // Campamentos de bandidos (Doc 1.9): spawn/respawn primero, después atacan cualquier caravana ya movida
  // este tick (comercial o de fundación) que pase cerca — mismo orden que el resto del tick, sobre posiciones
  // ya actualizadas.
  const trasSpawnBandidos = avanzarSpawnBandidos(estado.campamentosBandidos, estado.bandidosProximoSpawnEn, zonas, trasExpansion.asentamientos, mapa, instante);
  eventosDominio.push(...comoEventosDominio(trasSpawnBandidos.eventos, contexto));
  // Los ejércitos entran aquí solo como ESCOLTA: una caravana enganchada se defiende con el poder de su
  // columna y no con la defensa base fija (Doc 5.13.3). El movimiento de los ejércitos sigue después.
  const trasAtaquesBandidos = avanzarAtaquesBandidos(
    trasSpawnBandidos.campamentos,
    trasExpansion.caravanas,
    rng,
    estado.ejercitos,
    instante
  );
  eventosDominio.push(...comoEventosDominio(trasAtaquesBandidos.eventos, contexto));

  // Escolta sin héroe que vuelve a casa tras perder contra los bandidos (Doc 3.13.4): se funde con la
  // guarnición de su origen. `trasExpansion.asentamientos` es la lista con la que sigue el tick.
  let asentamientosTrasEscolta = trasExpansion.asentamientos;
  if (trasAtaquesBandidos.escoltasDevueltas.length > 0) {
    asentamientosTrasEscolta = asentamientosTrasEscolta.map((a) => {
      const devueltas = trasAtaquesBandidos.escoltasDevueltas.filter((d) => d.asentamientoId === a.id);
      if (devueltas.length === 0) return a;
      const escuadrones = devueltas.reduce((esc, d) => devolverEscoltaAGuarnicion(esc, d.escuadrones), a.escuadrones);
      return { ...a, escuadrones };
    });
  }

  // Ejércitos (Doc 5.12): comer del carro, moverse, repostar, llegar. Va DESPUÉS de los bandidos, al final de
  // la cadena. Solo consume aleatoriedad cuando un asedio llega a resolverse contra una plaza defendida
  // (Paso 7): sin eso, una partida sin ejércitos hace exactamente las mismas llamadas al RNG, en el mismo
  // orden, que antes de existir la mecánica — y por eso el guardián de determinismo sigue verde sin tocarlo.
  //
  // Recibe `trasAtaquesBandidos.caravanas` y NO `trasExpansion.caravanas`: los bandidos ya han podido
  // destruir alguna este tick, y partir de la lista anterior las habría resucitado al devolver la suya.
  const trasEjercitos = avanzarEjercitos(estado.ejercitos, {
    asentamientos: asentamientosTrasEscolta,
    caravanas: trasAtaquesBandidos.caravanas,
    facciones: trasExpansion.facciones,
    relaciones: estado.relaciones,
    mapa,
    instante,
    rng,
  });
  eventosDominio.push(...comoEventosDominio(trasEjercitos.eventos, contexto));

  // El mercado ya no LIQUIDA nada en el tick: una orden es una oferta en pie en una plaza y se cumple en el
  // mostrador, con alguien que ha ido hasta alli (`comerciarEnPlaza`, `Comercio_Fisico_Definicion.md`). Lo
  // unico que queda automatico es retirar las que nadie tomo — sin eso, nada las cerraria nunca.
  const trasMercado = caducarOrdenes(estado.ordenes, instante);
  eventosDominio.push(...comoEventosDominio(trasMercado.eventos, contexto));

  const trasTributos = avanzarTributos(estado.relaciones, trasEjercitos.asentamientos);
  eventosDominio.push(...comoEventosDominio(trasTributos.eventos, contexto));

  // Doc Fase_0_5 §8: se aplica la XP de construcción acumulada arriba junto a la del resto del tick (combate/
  // caravanas, aplicadas ya directamente sobre `facciones` en `engine/combate.ts`) antes de recalcular nivel.
  // `trasEjercitos.facciones` y no `trasExpansion.facciones`: un asedio ganado por un ejército otorga XP de
  // combate/conquista y puede penalizar reputación, y ese resultado tiene que entrar en la cadena.
  const faccionesConXp = aplicarAjustesExperiencia(trasEjercitos.facciones, ajustesExperiencia);
  const trasNivelFaccion = avanzarNivelesFaccion(faccionesConXp);
  eventosDominio.push(...comoEventosDominio(trasNivelFaccion.eventos, contexto));

  const faccionesFinal = avanzarReputacion(trasNivelFaccion.facciones, estado.relaciones);

  const titulosActuales = calcularTitulos(faccionesFinal, trasTributos.asentamientos, estado.relaciones, trasEjercitos.ejercitos);
  const eventosTitulos = narrarCambiosDeTitulo(estado.titulos, titulosActuales, faccionesFinal);
  eventosDominio.push(...comoEventosDominio(eventosTitulos, contexto));

  return {
    asentamientos: trasTributos.asentamientos,
    facciones: faccionesFinal,
    caravanas: trasEjercitos.caravanas,
    ejercitos: trasEjercitos.ejercitos,
    acuerdos: trasComercio.acuerdos,
    ordenes: trasMercado.ordenes,
    relaciones: estado.relaciones,
    titulos: titulosActuales,
    caminos: estado.caminos,
    campamentosBandidos: trasSpawnBandidos.campamentos,
    bandidosProximoSpawnEn: estado.bandidosProximoSpawnEn,
    // Al FINAL, y con lo que ya se movió: lo que se graba es dónde acabaron las columnas este minuto, no de
    // dónde salieron. No emite eventos ni cambia nada más — la memoria solo mira.
    memoriaPorFaccion: grabarLoVisto(estado.memoriaPorFaccion, {
      asentamientos: trasTributos.asentamientos,
      ejercitos: trasEjercitos.ejercitos,
      facciones: faccionesFinal,
      limites: mapa.limites,
      instante,
    }),
    jugadores: grabarExploracionPersonal(estado.jugadores, trasEjercitos.ejercitos, mapa.limites),
    estadoMapa: mapa.estadoActual(),
    eventosDominio,
  };
}
