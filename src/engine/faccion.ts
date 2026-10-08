import type { Asentamiento, Faccion, Sigilo } from '../domain/types';
import type { EventoCrudo } from '../domain/eventos';
import { CAP_FUNDACION_POR_NIVEL, CIUDADANIA, CUPO_NIVEL_ASENTAMIENTO, NIVEL_FACCION } from '../constants';
import { dias, transcurrido, type Instante } from '../domain/tiempo';
import { CAMPO_CARGO, esResidente } from './pertenencia';
import { ReglaInvalidaError } from './errores';
import { sigiloLibre } from './sigilo';

/** Fase A5 — payload de `faccion.nivel_subio` (ver `avanzarNivelesFaccion`). */
export interface PayloadFaccionNivelSubio {
  faccionId: string;
  nivelNuevo: number;
}

export class FaccionInvalidaError extends ReglaInvalidaError {}

/** Sin `sigilo`, la Facción recibe uno derivado de su id (la partida pasa los ocupados para que no choque). */
export function crearFaccion(id: string, nombre: string, sigilo: Sigilo = sigiloLibre(id, [])): Faccion {
  const nombreLimpio = nombre.trim();
  if (!nombreLimpio) throw new FaccionInvalidaError('El nombre de la Facción no puede estar vacío.');
  return { id, nombre: nombreLimpio, sigilo, reyId: null, embajadorId: null, nivel: 1, experiencia: 0, ciudadanosIds: [], reputacion: 0 };
}

/**
 * Nivel de Facción por EXPERIENCIA (Doc 1.7, rediseño Fase 0.5): puramente derivado de `faccion.experiencia`
 * (ya monótona por construcción, ver `aplicarAjustesExperiencia`) contra la curva de umbrales acumulados
 * `NIVEL_FACCION.xpParaNivel`.
 */
export function calcularNivelFaccion(faccion: Faccion): number {
  let nivel = 1;
  for (const umbral of NIVEL_FACCION.xpParaNivel) {
    if (faccion.experiencia < umbral) break;
    nivel += 1;
  }
  return Math.min(NIVEL_FACCION.nivelMaximo, nivel);
}

/** Cap de fundación (Doc 1.7): límite duro de asentamientos FUNDADOS (no aplica a conquista/anexión). */
export function calcularCapFundacion(nivel: number): number {
  const idx = Math.min(CAP_FUNDACION_POR_NIVEL.length, Math.max(1, nivel)) - 1;
  return CAP_FUNDACION_POR_NIVEL[idx] ?? CAP_FUNDACION_POR_NIVEL[CAP_FUNDACION_POR_NIVEL.length - 1]!;
}

/** Cupo de asentamientos en nivel 2 o 3 según el nivel de Facción (Doc Fase_0_5 §5). Nivel 1 no tiene cupo. */
export function calcularCupoNivel(nivelFaccion: number, nivelObjetivo: 2 | 3): number {
  const curva = nivelObjetivo === 2 ? CUPO_NIVEL_ASENTAMIENTO.maxNivel2 : CUPO_NIVEL_ASENTAMIENTO.maxNivel3;
  const idx = Math.min(curva.length, Math.max(1, nivelFaccion)) - 1;
  return curva[idx] ?? curva[curva.length - 1]!;
}

export interface AjusteExperiencia {
  faccionId: string;
  delta: number;
  razon: string;
}

/**
 * Aplica una lista de ganancias de experiencia (Doc Fase_0_5 §8: combate/construcción/conquista/caravanas) **y sube el nivel
 * en el mismo momento** si la experiencia cruza un umbral, con su evento `faccion.nivel_subio`. Antes el nivel se
 * re-derivaba en un barrido de cada tick (`avanzarNivelesFaccion`), y el XP que daba un comando tardaba hasta un minuto en
 * verse como nivel (`Consideraciones/Auditoria_Tick_Eventos.md`, ficha A). Ahora quien da la experiencia la convierte en nivel.
 */
export function aplicarExperiencia(facciones: Faccion[], ajustes: AjusteExperiencia[]): { facciones: Faccion[]; eventos: EventoCrudo[] } {
  if (ajustes.length === 0) return { facciones, eventos: [] };
  return avanzarNivelesFaccion(aplicarAjustesExperiencia(facciones, ajustes));
}

/** Suma la experiencia sin tocar el nivel. Para quien necesita el nivel recalculado: `aplicarExperiencia`. */
export function aplicarAjustesExperiencia(facciones: Faccion[], ajustes: AjusteExperiencia[]): Faccion[] {
  if (ajustes.length === 0) return facciones;
  const porId = new Map(facciones.map((f) => [f.id, f]));
  for (const { faccionId, delta } of ajustes) {
    const faccion = porId.get(faccionId);
    if (faccion && delta > 0) porId.set(faccionId, { ...faccion, experiencia: faccion.experiencia + delta });
  }
  return facciones.map((f) => porId.get(f.id)!);
}

/** Recalcula el nivel de las Facciones a partir de su experiencia; devuelve eventos de subida. Devuelve el MISMO array si nada cambió. */
export function avanzarNivelesFaccion(facciones: Faccion[]): { facciones: Faccion[]; eventos: EventoCrudo[] } {
  const eventos: EventoCrudo[] = [];
  const actualizadas = facciones.map((f) => {
    const nuevoNivel = calcularNivelFaccion(f);
    if (nuevoNivel > f.nivel) {
      eventos.push({
        codigo: 'faccion.nivel_subio',
        mensaje: `${f.nombre} sube a nivel de Facción ${nuevoNivel}.`,
        payload: { faccionId: f.id, nivelNuevo: nuevoNivel } satisfies PayloadFaccionNivelSubio,
      });
    }
    return nuevoNivel === f.nivel ? f : { ...f, nivel: nuevoNivel };
  });
  return { facciones: eventos.length === 0 ? facciones : actualizadas, eventos };
}

export function esCiudadano(faccion: Faccion, heroeId: string): boolean {
  return faccion.ciudadanosIds.includes(heroeId);
}

/**
 * Quien pasa a ser ciudadano de una Facción (la funda, la acepta el Rey, funda una plaza…) deja de pedir ingreso en TODAS: «1 y solo 1 Facción» (Doc 0) y una
 * solicitud viva de un ciudadano no se puede aceptar (`faccion.invalida`). Se llama donde se otorga la ciudadanía y no se filtra al proyectar, para que
 * ninguna lista (la del Rey, los bots, la proyección) vea nunca una solicitud de quien ya está dentro.
 */
export function sinSolicitudesDe(facciones: readonly Faccion[], heroeIds: readonly string[]): Faccion[] {
  return facciones.map((f) => (f.solicitudesIds?.some((id) => heroeIds.includes(id)) ? { ...f, solicitudesIds: f.solicitudesIds.filter((id) => !heroeIds.includes(id)) } : f));
}

/** Red de seguridad por tick: borra de las listas de solicitantes a cualquier ciudadano (partidas guardadas antes de `sinSolicitudesDe`, o una ruta futura que se olvide de llamarlo). */
export function sinSolicitudesCaducadas(facciones: readonly Faccion[]): Faccion[] {
  return sinSolicitudesDe(facciones, facciones.flatMap((f) => f.ciudadanosIds));
}

export function otorgarCiudadania(faccion: Faccion, heroeId: string): Faccion {
  if (esCiudadano(faccion, heroeId)) return faccion;
  return { ...faccion, ciudadanosIds: [...faccion.ciudadanosIds, heroeId] };
}

/**
 * Retira la ciudadanía (comando `dejarFaccion`, a petición del usuario).
 *
 * **Sucesión del trono (a petición del usuario, 2026-09-10): una Facción SIEMPRE tiene Rey mientras le quede
 * al menos un ciudadano.** Si se va el Rey, el trono pasa al siguiente de `ciudadanosIds` (orden de ingreso —
 * el fundador primero). Solo queda `reyId: null` si la Facción se queda sin nadie. Si el heredero ocupaba la
 * embajada, esta se vacía (la re-designa el nuevo Rey). El Embajador que se va también libera su cargo.
 *
 * NO toca la residencia: eso lo hace el comando con `dejarResidencia`, que también la libera al abandonar la Facción.
 */
export function quitarCiudadania(faccion: Faccion, heroeId: string): Faccion {
  if (!esCiudadano(faccion, heroeId)) return faccion;
  const ciudadanosIds = faccion.ciudadanosIds.filter((id) => id !== heroeId);
  const reyId = faccion.reyId === heroeId ? (ciudadanosIds[0] ?? null) : faccion.reyId;
  return {
    ...faccion,
    ciudadanosIds,
    reyId,
    embajadorId: faccion.embajadorId === heroeId || faccion.embajadorId === reyId ? null : faccion.embajadorId,
  };
}

/**
 * Si al caer una plaza su Facción se queda sin ningún asentamiento, anota quién la derrotó (`Faccion.derrotadaPor`,
 * Doc 5.15.5). Lo llaman los dos caminos de la conquista (asedio con números y batalla de Unity) con el mundo ya
 * conquistado; qué pasa después con sus héroes lo decide quien los gobierna.
 */
export function registrarDerrota(
  facciones: readonly Faccion[],
  asentamientos: readonly Asentamiento[],
  perdedoraId: string,
  /** `null` si nadie la derrotó: su última plaza colapsó (Doc 4.5). */
  ganadoraId: string | null
): Faccion[] {
  if (asentamientos.some((a) => a.faccionId === perdedoraId)) return [...facciones];
  return facciones.map((f) => (f.id === perdedoraId ? { ...f, derrotadaPor: ganadoraId } : f));
}

/**
 * Doc 2.5: no se cambia de residencia hasta pasado `CIUDADANIA.cooldownCambioResidenciaDias` desde el
 * último cambio (`cambiarResidencia` o `dejarResidencia`). Quien nunca ha cambiado (`undefined`) está libre, y la
 * reubicación forzosa por conquista no cuenta. Lo llaman los comandos, que son quienes guardan el instante.
 */
export function exigirSinCooldownDeResidencia(ultimoCambioEn: Instante | undefined, instante: Instante): void {
  if (ultimoCambioEn !== undefined && transcurrido(ultimoCambioEn, instante) < dias(CIUDADANIA.cooldownCambioResidenciaDias)) {
    throw new FaccionInvalidaError(`Cambiaste de residencia hace poco: espera ${CIUDADANIA.cooldownCambioResidenciaDias} días de mundo entre cambios.`);
  }
}

/** El asentamiento sin ese héroe como residente (ni por fundar ni por haberse mudado) y sin sus cargos locales: no se gobierna donde no se vive. */
function sinResidente(asentamiento: Asentamiento, heroeId: string): Asentamiento {
  const cargos = { ...asentamiento.cargos };
  for (const campo of Object.values(CAMPO_CARGO)) {
    if (cargos[campo] === heroeId) cargos[campo] = null;
  }
  return {
    ...asentamiento,
    heroesFundadoresIds: asentamiento.heroesFundadoresIds.filter((id) => id !== heroeId),
    casasCompradas: asentamiento.casasCompradas.filter((id) => id !== heroeId),
    cargos,
  };
}

/**
 * Dejar la residencia (Doc 2.5, 2026-10-02): libera la vivienda y vacía los cargos locales ahí, sin tomar otra casa. El
 * héroe sigue siendo ciudadano, y el comando lo lleva en el acto al campamento de mercenarios más cercano
 * (`acogerEnCampamentoMasCercano`) hasta que se mude a otra plaza (`cambiarResidencia`). No hay reembolso.
 */
export function dejarResidencia(asentamientos: readonly Asentamiento[], heroeId: string): Asentamiento {
  const actual = asentamientos.find((a) => esResidente(a, heroeId));
  if (!actual) throw new FaccionInvalidaError('El jugador no reside en ningún asentamiento.');
  return sinResidente(actual, heroeId);
}

/**
 * Pedir el ingreso en una Facción (D5, D6, D46): entra en su lista de solicitantes, y el Rey acepta o deniega. Quien ya es ciudadano de
 * una no pide (Doc 0: 1 y solo 1 Facción). Pedir otra vez es pedir lo mismo: no cambia nada.
 */
export function solicitarIngreso(faccion: Faccion, facciones: readonly Faccion[], heroeId: string): Faccion {
  if (facciones.some((f) => esCiudadano(f, heroeId))) throw new FaccionInvalidaError('Ya eres ciudadano de una Facción (Doc 0: 1 y solo 1).');
  if (faccion.solicitudesIds?.includes(heroeId)) return faccion;
  return { ...faccion, solicitudesIds: [...(faccion.solicitudesIds ?? []), heroeId] };
}

/**
 * El Rey responde a una solicitud (D46): aceptada, el héroe es ciudadano y sus solicitudes en otras Facciones caen; denegada, sale de la
 * lista y nada más. Devuelve TODAS las Facciones porque aceptar toca las listas de las demás.
 */
export function responderSolicitud(facciones: readonly Faccion[], faccionId: string, heroeId: string, aceptar: boolean): Faccion[] {
  const faccion = facciones.find((f) => f.id === faccionId);
  if (!faccion?.solicitudesIds?.includes(heroeId)) throw new FaccionInvalidaError('Ese héroe no ha pedido entrar.');
  if (aceptar && facciones.some((f) => esCiudadano(f, heroeId))) throw new FaccionInvalidaError('Ese héroe ya es ciudadano de otra Facción.');
  // Aceptada: sale de TODAS las listas; denegada: solo de la de esta Facción.
  const sinSolicitud = (f: Faccion): Faccion => (f.solicitudesIds?.includes(heroeId) ? { ...f, solicitudesIds: f.solicitudesIds.filter((id) => id !== heroeId) } : f);
  if (aceptar) return sinSolicitudesDe(facciones, [heroeId]).map((f) => (f.id === faccionId ? otorgarCiudadania(f, heroeId) : f));
  return facciones.map((f) => (f.id === faccionId ? sinSolicitud(f) : f));
}

/**
 * Cambiar de residencia (Doc 2.5/2.6, comando nuevo 2026-09-08 — cierra la limitación conocida "no hay comando
 * vender casa / dejar residencia"): atómico, deja la residencia actual y toma otra plaza de la MISMA Facción.
 *
 * Deja la vieja: fuera de `casasCompradas` Y `heroesFundadoresIds` (ya no reside por ninguna vía), y sus
 * cargos LOCALES ahí se vacían (no se gobierna donde no se vive — misma regla que la conquista). Su campamento
 * se traslada con él (Doc 2.5/5.15.2) sin mover nada: las escuadras viven en el héroe y el campamento es su
 * residencia.
 *
 * La ciudadanía de Facción no cambia (es la misma Facción). Quien no reside en ninguna plaza —vive en un campamento de mercenarios, por
 * ejemplo el ciudadano recién aceptado— entra sin dejar ninguna (`origen` ausente): es la única puerta a la residencia en una plaza
 * que no es fundarla (D31: no hay compra de casa).
 *
 * Tiene cooldown (`exigirSinCooldownDeResidencia`), que comprueba el comando; sin coste.
 */
export function cambiarResidencia(
  facciones: Faccion[],
  asentamientos: Asentamiento[],
  destinoId: string,
  heroeId: string
): { origen: Asentamiento | undefined; destino: Asentamiento } {
  const destino = asentamientos.find((a) => a.id === destinoId);
  if (!destino) throw new FaccionInvalidaError('El asentamiento de destino no existe.');
  const faccion = facciones.find((f) => f.id === destino.faccionId);
  if (!faccion || !esCiudadano(faccion, heroeId)) {
    throw new FaccionInvalidaError('Solo se reside en un asentamiento de la propia Facción.');
  }
  const origen = asentamientos.find((a) => a.id !== destinoId && esResidente(a, heroeId));
  if (destino.casasCompradas.includes(heroeId) || destino.heroesFundadoresIds.includes(heroeId)) {
    throw new FaccionInvalidaError('El jugador ya reside en el destino.');
  }
  if (destino.vetadosIds?.includes(heroeId)) {
    throw new FaccionInvalidaError('El asentamiento de destino no admite nuevos residentes ahora mismo.');
  }

  return {
    origen: origen && sinResidente(origen, heroeId),
    destino: { ...destino, casasCompradas: [...destino.casasCompradas, heroeId] },
  };
}
