// Quién puede unirse a una batalla y a qué bando (Doc 5.15.1b). Es lo único de una batalla que depende del mundo
// —diplomacia, clase de columna, Facción— y no de la partida: Unity decide lo que pasa dentro.
import type { Ejercito, Heroe, RelacionPolitica } from '../domain/types';
import type { LadoId } from '../contratos/v1/dto';
import { esCiudadano } from '../engine/faccion';
import { BatallaInvalidaError, type Batalla } from './batallas';
import type { GameSessionState } from './estado';

/** La Facción de un héroe: aquella de la que es ciudadano. */
export function faccionDe(estado: Pick<GameSessionState, 'facciones'>, heroeId: string): string | null {
  return estado.facciones.find((f) => esCiudadano(f, heroeId))?.id ?? null;
}

/** Aliadas **para unirse a una batalla**: una alianza o un vasallaje activos (decisión del autor, 2026-10-06). No es
 * `estanAliadas`, que sigue mirando solo las alianzas para el resto del juego (comercio, caminos, ataques). */
export function sonAliadasEnBatalla(relaciones: readonly RelacionPolitica[], a: string, b: string): boolean {
  return relaciones.some(
    (r) => r.estado === 'activa' && r.tipo !== 'guerra' && ((r.faccionAId === a && r.faccionBId === b) || (r.faccionAId === b && r.faccionBId === a))
  );
}

/**
 * El bando al que se une `heroeId` con `columna`, o por qué no puede (Doc 5.15.1b):
 *
 * - **Batalla campal** (ejércitos): nadie de fuera. **Persecución** (columnas personales): cualquiera, al bando que elija.
 * - **Campamento de bandidos** (evento PvE): cualquiera, siempre contra los bandidos.
 * - **Asedio y asalto de caravana:** al defensor, su Facción y sus aliados (vasallos incluidos); al atacante, la suya y,
 *   si su Rey lo admite, Facciones aliadas de nadie del defensor: neutrales o enemigas de este.
 */
export function ladoParaUnirse(
  estado: Pick<GameSessionState, 'facciones' | 'relaciones'>,
  batalla: Batalla,
  heroe: Pick<Heroe, 'id'>,
  columna: Pick<Ejercito, 'tipo'>,
  ladoPedido?: LadoId
): LadoId {
  const contexto = batalla.ticket.contextoEstrategico;
  const rechazar = (motivo: string): never => {
    throw new BatallaInvalidaError(motivo);
  };

  if (contexto.tipo === 'campo_abierto') {
    if (contexto.columnas === 'ejercitos') return rechazar('Una batalla campal no admite a nadie de fuera.');
    if (columna.tipo === 'ejercito') return rechazar('Una persecución es de columnas personales: un ejército no se une.');
    return ladoPedido ?? rechazar('Elige bando: en una persecución se puede ir con cualquiera.');
  }
  if (contexto.tipo === 'campamento_bandidos') {
    return ladoPedido === 'defensor' ? rechazar('Nadie puede ayudar a los bandidos.') : 'atacante';
  }

  const faccionId = faccionDe(estado, heroe.id);
  if (!faccionId) return rechazar('Hace falta una Facción para unirse a esta batalla.');
  const { atacante, defensor } = batalla.ticket.bandos;
  const admite = estado.facciones.find((f) => f.id === atacante.faccionId)?.admiteOtrasEnAtaques === true;
  const lado: LadoId | undefined =
    faccionId === atacante.faccionId
      ? 'atacante'
      : defensor.faccionId && (faccionId === defensor.faccionId || sonAliadasEnBatalla(estado.relaciones, faccionId, defensor.faccionId))
        ? 'defensor'
        : admite
          ? 'atacante'
          : undefined;
  if (!lado) return rechazar('Tu Facción no combate en esta batalla.');
  if (ladoPedido && ladoPedido !== lado) return rechazar('No puedes unirte a ese bando.');
  return lado;
}
