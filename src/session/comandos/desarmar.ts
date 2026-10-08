// Un ejército que entra en una plaza o en un campamento se desarma en la puerta (2026-10-08): cada héroe queda en una columna personal
// —el Líder con el resto del carro, los demás con su parte a prorrata (`desgajar`)— y entra después como lo haría solo. Aquí solo se
// prepara ese estado; la entrada de cada uno es la de siempre (`entrarEnAsentamiento`, `entrarEnCampamento`).
import type { Ejercito, Point } from '../../domain/types';
import { caravanasAlEntrar, MovilizacionInvalidaError } from '../../engine/ejercitos';
import { deshacerFormaciones } from '../../engine/formacion';
import type { GameSessionState } from '../estado';

export type LugarDeEntrada = { tipo: 'asentamiento' | 'campamento'; id: string; posicion: Point };

/** Solo su Líder lo hace entrar; las caravanas adjuntas tienen que ser del lugar (se dejan en él); el resto, columnas personales. */
export function desarmarAlEntrar(estado: GameSessionState, ejercito: Ejercito, heroeId: string, lugar: LugarDeEntrada): GameSessionState {
  if (ejercito.liderId !== heroeId) throw new MovilizacionInvalidaError('Solo el Líder hace entrar al ejército.');
  const cambiadas = new Map(caravanasAlEntrar(ejercito, estado.caravanas, lugar).map((c) => [c.id, c]));
  const deshecho = deshacerFormaciones(estado.ejercitos, estado.heroes, new Set([ejercito.id]));
  return { ...estado, ejercitos: deshecho.ejercitos, heroes: deshecho.heroes, caravanas: estado.caravanas.map((c) => cambiadas.get(c.id) ?? c) };
}
