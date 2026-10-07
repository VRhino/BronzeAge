// Lo que el acceso recuerda de una partida que se borra (y se recupera al restaurarla de un respaldo). Las membresías humanas se
// revocan con `motivoFin: 'partida_borrada'` —el historial se conserva (doc 5) y restaurar las reactiva—; las de bots se borran,
// y la cuenta de bot que se queda sin ninguna partida se borra entera: se creó solo para jugar en esa.
import type { RepositorioIdentidad } from '../../acceso/repositorio';
import { esCuentaDeBot } from './proveedorClave';

export function olvidarPartida(repositorio: RepositorioIdentidad, gameId: string, ahora: string): { revocadas: number; cuentasDeBotBorradas: number } {
  let revocadas = 0;
  let cuentasDeBotBorradas = 0;
  for (const m of repositorio.listarMembresiasDePartida(gameId)) {
    if (esCuentaDeBot(repositorio, m.usuarioId)) {
      repositorio.borrarMembresia(m.usuarioId, gameId);
      if (repositorio.listarMembresiasDeUsuario(m.usuarioId).length === 0) {
        repositorio.borrarUsuario(m.usuarioId);
        cuentasDeBotBorradas++;
      }
    } else if (m.hasta === undefined) {
      repositorio.otorgarMembresia({ ...m, hasta: ahora, motivoFin: 'partida_borrada' });
      revocadas++;
    }
  }
  return { revocadas, cuentasDeBotBorradas };
}

/** Tras restaurar una partida borrada: vuelven las membresías que cerró el borrado (las de los administradores, y las de los
 * jugadores cuyo héroe sigue en el mundo restaurado). */
export function recuperarPartida(repositorio: RepositorioIdentidad, gameId: string, jugadoresConHeroe: ReadonlySet<string>): number {
  let reactivadas = 0;
  for (const m of repositorio.listarMembresiasDePartida(gameId)) {
    if (m.motivoFin !== 'partida_borrada') continue;
    if (m.jugadorId !== null && !jugadoresConHeroe.has(m.jugadorId)) continue;
    const { hasta: _hasta, motivoFin: _motivo, ...vigente } = m;
    repositorio.otorgarMembresia(vigente);
    reactivadas++;
  }
  return reactivadas;
}
