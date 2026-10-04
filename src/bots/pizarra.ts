// La pizarra de una Facción (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §3): lo que sus bots se cuentan entre ellos,
// como un grupo de amigos por el chat de voz. Solo guarda lo que algún bot de la Facción vio o decidió: no da
// información que la Facción no tenga. Vive en el runner, no en la partida, y es desechable (§3.3): si se pierde, los
// bots vuelven a enterarse mirando.
import type { Instante } from '../domain/tiempo';
import type { OrdenMercado, Point } from '../domain/types';
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
  /**
   * Sin plaza: la columna que sale de un campamento (a cazar o a fundar) y espera en la puerta hasta `hasta` a que se le unan
   * los compañeros que residen allí. Por campamento.
   */
  salidas: Map<string, { ejercitoId: string; liderId: string; hasta: Instante; para: 'cazar' | 'fundar' }>;
  /** Sin plaza: los bandidos que algún bot de la Facción vio, con dónde y cuándo (los campamentos de bandidos no tienen memoria). */
  bandidos: Map<string, { posicion: Point; poder: number; vistoEn: Instante }>;
  /** Sin plaza: quién está dentro de qué campamento listo para salir, y si con la ración llena. Vale hasta `hasta`. */
  listos: Map<string, { campamentoId: string; conRacion: boolean; hasta: Instante }>;
  /** Sin plaza: cuándo volvió el último explorador del anillo de cada campamento. Sin esto, no se saca la caravana. */
  explorados: Map<string, Instante>;
}

export function pizarraVacia(): Pizarra {
  return { residencias: new Map(), encargos: new Map(), defensas: new Map(), mostradores: new Map(), salidas: new Map(), bandidos: new Map(), listos: new Map(), explorados: new Map() };
}

/** Libera los encargos de un bot (al cambiar de plan o al dejar de existir). */
export function soltarEncargos(pizarra: Pizarra, heroeId: string): void {
  for (const [clave, quien] of pizarra.encargos) if (quien === heroeId) pizarra.encargos.delete(clave);
}
