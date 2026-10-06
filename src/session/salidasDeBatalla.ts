// Un héroe sale de una batalla de Unity (doc 01 §15, doc 02 §3.3, CQ-011, Doc 5.15.1). Por qué sale lo decide Unity; aquí
// solo se trae de vuelta al mundo: sus escuadras, su columna y, si lo da por derrotado, su herida.
import { BATALLA } from '../constants';
import { minutos, sumar, type Instante } from '../domain/tiempo';
import type { SalidaBatalla } from '../contratos/v1/dto';
import { columnaDe } from '../engine/ejercitos';
import { herir } from '../engine/heroe';
import {
  BatallaInvalidaError,
  cerrarSinResultado,
  conBatalla,
  conReserva,
  exigirAsignacionActiva,
  exigirRevisionVigente,
  participacionesDe,
  type Batalla,
} from './batallas';
import type { GameSessionState } from './estado';

/**
 * - **Antes de empezar** (`asignada`, `no_conectado`): sale sin castigo con sus escuadras intactas, sube `ticketRevision`
 *   y la batalla vuelve a `convocando`: la asignación y los tokens anteriores dejan de valer. No entra nadie en su
 *   lugar. Si no queda ningún héroe humano, o el atacante se queda sin héroes, se cancela sin castigo.
 * - **En curso** (`eliminado`, `abandono`, `desconexion`): se aplican las bajas de sus escuadras y, si viene `derrotado`,
 *   queda herido. El ticket no cambia; el resultado final ya no lo repite.
 *
 * Repetir una salida que ya está registrada no cambia nada: es lo que deja a Conquest reintentar.
 */
export function registrarSalida(estado: GameSessionState, batalla: Batalla, salida: SalidaBatalla, servidorId: string, ahora: Instante): GameSessionState {
  // Antes que nada: tras una salida previa a la partida la asignación y la revisión ya no son las del mensaje, y un reintento tiene que responder bien.
  if ((batalla.salidas ?? []).some((s) => s.heroeId === salida.heroeId)) return estado;
  exigirAsignacionActiva(batalla, salida.intentoAsignacionId, servidorId);
  exigirRevisionVigente(batalla, salida.ticketRevision);

  const antes = salida.motivo === 'no_conectado';
  if (antes ? batalla.estado !== 'asignada' : batalla.estado !== 'en_curso') {
    throw new BatallaInvalidaError(antes ? 'Solo se da de baja por no conectarse antes de que empiece la partida.' : 'Solo se sale así con la partida en curso.');
  }
  const participante = participacionesDe(batalla).find((p) => p.participante.heroeId === salida.heroeId)?.participante;
  if (!participante) throw new BatallaInvalidaError('Ese héroe no combate en esta batalla.');

  const autorizados = new Map(participante.escuadras.map((e) => [e.squadId, e.efectivosAutorizados]));
  const queda = new Map<string, number>();
  for (const e of antes ? [] : salida.escuadras) {
    const autorizadas = autorizados.get(e.squadId);
    if (autorizadas === undefined || queda.has(e.squadId)) throw new BatallaInvalidaError(`La escuadra ${e.squadId} no es de ese héroe, o va repetida.`);
    if (e.supervivientes + e.muertos !== autorizadas) throw new BatallaInvalidaError(`${e.squadId} no cierra sobre sus ${autorizadas} efectivos.`);
    queda.set(e.squadId, e.supervivientes);
  }

  const derrotado = !antes && salida.derrotado;
  const conBajas = estado.heroes.map((h) =>
    h.escuadrones.some((e) => queda.has(e.id)) ? { ...h, escuadrones: h.escuadrones.map((e) => (queda.has(e.id) ? { ...e, cantidad: queda.get(e.id)! } : e)) } : h
  );
  const libres = conReserva(conBajas, [...autorizados.keys()], undefined);
  const heroes = derrotado ? herir(libres, [salida.heroeId], ahora) : libres;

  const sale = { heroeId: salida.heroeId, motivo: salida.motivo, derrotado, en: ahora };
  const salidas = [...(batalla.salidas ?? []), sale];
  const sinEl = { ...batalla, salidas };
  const quedan = participacionesDe(sinEl);

  // Su columna deja de estar quieta si no queda nadie más de ella en la batalla.
  const columna = columnaDe(estado.ejercitos, salida.heroeId);
  const sigueLaColumna = columna && quedan.some((p) => columna.participantes.some((c) => c.heroeId === p.participante.heroeId));
  const bloqueo = columna && !sigueLaColumna ? { ...batalla.bloqueo, ejercitoIds: batalla.bloqueo.ejercitoIds.filter((id) => id !== columna.id) } : batalla.bloqueo;
  const iniciadaPor = batalla.iniciadaPor === salida.heroeId ? (quedan.find((p) => p.lado === 'atacante')?.participante.heroeId ?? batalla.iniciadaPor) : batalla.iniciadaPor;
  const siguiente: GameSessionState = { ...estado, heroes };

  if (!antes) return conBatalla(siguiente, { ...sinEl, bloqueo, iniciadaPor });

  const bandos = (['atacante', 'defensor'] as const).map((lado) => [lado, { ...batalla.ticket.bandos[lado], participantes: batalla.ticket.bandos[lado].participantes.filter((p) => p.heroeId !== salida.heroeId) }] as const);
  const revisada: Batalla = {
    ...sinEl,
    bloqueo,
    iniciadaPor,
    estado: 'convocando',
    asignacion: undefined,
    ticket: { ...batalla.ticket, ticketRevision: batalla.ticket.ticketRevision + 1, bandos: Object.fromEntries(bandos) as Batalla['ticket']['bandos'] },
    expiraEn: sumar(ahora, minutos(BATALLA.plazoAsignacionMinutos)),
  };
  const sinHumano = !quedan.some((p) => p.participante.controlador === 'humano');
  const sinAtacantes = !quedan.some((p) => p.lado === 'atacante');
  return sinHumano || sinAtacantes ? cerrarSinResultado(conBatalla(siguiente, revisada), revisada, 'cancelada') : conBatalla(siguiente, revisada);
}
