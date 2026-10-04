// Cuándo y cómo llegan los bots (D56, D57): escalonados a lo largo de los días, como llegarían los jugadores. Grupos de
// tres amigos que llegan juntos, solitarios y, en la segunda mitad, los que llegan tarde. Fuera del motor: lo usa quien da de
// alta a los bots (el batch, y el runner remoto cuando exista), que crea cada héroe por el puerto en su minuto.
import { createRng } from '../worldgen';
import type { CampamentoElegible, PuertoBot } from './puerto';
import type { Perfil, RunnerDeBots } from './runner';

/** Reparto de perfiles (D57). ponytail: placeholder hasta medir con batch. */
const REPARTO = { amigos: 0.5, solitario: 0.3 };
const AMIGOS_POR_GRUPO = 3;

/** Una llegada: en `tick`, los `cuantos` del grupo `grupo` (1 salvo los amigos), con ese perfil. */
export interface Llegada {
  tick: number;
  grupo: number;
  perfil: 'amigos' | 'solitario' | 'tardio';
  cuantos: number;
}

/** `total` bots en `dias` días, ordenados por tick (y grupo). Misma semilla, mismas llegadas. */
export function planDeLlegadas(semilla: number, total: number, dias: number): Llegada[] {
  const rng = createRng(semilla ^ 0x11e9);
  const minutos = Math.max(1, Math.round(dias * 24 * 60));
  const llegadas: Llegada[] = [];
  let quedan = total;
  for (let grupo = 0; quedan > 0; grupo++) {
    const r = rng();
    const perfil = r < REPARTO.amigos && quedan >= AMIGOS_POR_GRUPO ? 'amigos' : r < REPARTO.amigos + REPARTO.solitario ? 'solitario' : 'tardio';
    // Los tardíos, en la segunda mitad: cuando ya hay Facciones a las que pedir entrar.
    const desde = perfil === 'tardio' ? minutos / 2 : 0;
    const tick = 1 + Math.floor(desde + rng() * (minutos - desde));
    const cuantos = perfil === 'amigos' ? AMIGOS_POR_GRUPO : 1;
    llegadas.push({ tick, grupo, perfil, cuantos });
    quedan -= cuantos;
  }
  return llegadas.sort((a, b) => a.tick - b.tick || a.grupo - b.grupo);
}

/** Como elegiría quien llega (§3.1): el campamento con menos residentes. */
export const campamentoConMenosResidentes = (campamentos: readonly CampamentoElegible[]): string | undefined =>
  [...campamentos].sort((a, b) => a.residentes - b.residentes || (a.id < b.id ? -1 : 1))[0]?.id;

/**
 * Da de alta una llegada: crea sus héroes por el puerto (`nombres` da el nombre de cada uno), todos en el mismo campamento, y
 * los pone a jugar con su perfil; el primero de unos amigos es su líder. Devuelve los creados, con su perfil.
 */
export async function darDeAlta(puerto: PuertoBot, bots: RunnerDeBots, llegada: Llegada, nombres: () => string): Promise<{ heroeId: string; perfil: Perfil }[]> {
  const creados: { heroeId: string; perfil: Perfil }[] = [];
  // Los que llegan juntos van al mismo campamento: el que elige el primero.
  let campamentoId: string | undefined;
  const elegir = (campamentos: CampamentoElegible[]) => (campamentoId ??= campamentoConMenosResidentes(campamentos));
  for (let i = 0; i < llegada.cuantos; i++) {
    const nombre = nombres();
    const id = await puerto.llegar(nombre, DATOS_DE_BOT(nombre), elegir);
    if (!id) continue;
    const perfil: Perfil = llegada.perfil === 'amigos' ? { tipo: 'amigos', lider: creados[0]?.heroeId ?? id } : { tipo: llegada.perfil };
    creados.push({ heroeId: id, perfil });
    bots.alta(id, perfil);
  }
  return creados;
}

const DATOS_DE_BOT = (nombre: string) => ({
  displayName: nombre,
  classDefinitionId: 'Spear',
  genero: 'masculino' as const,
  avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' },
});
