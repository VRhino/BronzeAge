// La pizarra de una Facción (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §3): lo que sus bots se cuentan entre ellos,
// como un grupo de amigos por el chat de voz. Solo guarda lo que algún bot de la Facción vio o decidió: no da
// información que la Facción no tenga. Vive en el runner, no en la partida, y es desechable (§3.3): si se pierde, los
// bots vuelven a enterarse mirando.
import type { Instante } from '../domain/tiempo';
import type { OrdenMercado } from '../domain/types';
import type { DefensaPlaza } from '../engine/ejercitos';

export interface Pizarra {
  /** Dónde reside cada bot de la Facción: cada uno apunta la suya al pensar. */
  residencias: Map<string, string>;
  /** Quién lleva qué, para no ir dos al mismo sitio: `bandido:<id>`, `campana:<plazaId>`, `explorar`. */
  encargos: Map<string, string>;
  /** La defensa de plazas ajenas inspeccionadas, con cuándo se vio. */
  defensas: Map<string, { defensa: DefensaPlaza; vistoEn: Instante }>;
  /** Las órdenes en pie que se vieron en el mostrador de plazas ajenas, con cuándo. */
  mostradores: Map<string, { ordenes: OrdenMercado[]; vistoEn: Instante }>;
}

export function pizarraVacia(): Pizarra {
  return { residencias: new Map(), encargos: new Map(), defensas: new Map(), mostradores: new Map() };
}

/** Libera los encargos de un bot (al cambiar de plan o al dejar de existir). */
export function soltarEncargos(pizarra: Pizarra, heroeId: string): void {
  for (const [clave, quien] of pizarra.encargos) if (quien === heroeId) pizarra.encargos.delete(clave);
}
