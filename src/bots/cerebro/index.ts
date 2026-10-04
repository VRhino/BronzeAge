// El cerebro de un bot (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §3.4): primero lo que su cargo le pide; sin plaza donde
// vivir, el arranque en los campamentos (`sinPlaza`); con ella, lo que hace cualquier residente en casa y, si no manda nada,
// salir; fuera, lo que toca a su columna.
import type { Cerebro } from '../runner';
import { gobernar } from './gobierno';
import { enColumna, residir, salir } from './militar';
import { plazaDentro } from './comun';
import { sinPlaza } from './sinPlaza';

export const cerebroDeBot: Cerebro = (ctx) => {
  gobernar(ctx);
  if (!ctx.vista.heroe?.residenciaId) {
    sinPlaza(ctx);
  } else if (plazaDentro(ctx.vista)) {
    residir(ctx);
    salir(ctx);
  } else {
    enColumna(ctx);
  }
};
