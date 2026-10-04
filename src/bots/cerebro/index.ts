// El cerebro de un bot (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §3.4): primero lo que su cargo le pide, luego lo
// que hace cualquier residente en casa y, si no manda nada, salir; fuera, lo que toca a su columna.
//
// Falta la fase «sin plaza» (bots-héroe en los campamentos, paso 4): un bot sin residencia hoy no hace nada.
import type { Cerebro } from '../runner';
import { gobernar } from './gobierno';
import { enColumna, residir, salir } from './militar';
import { plazaDentro } from './comun';

export const cerebroDeBot: Cerebro = (ctx) => {
  gobernar(ctx);
  if (plazaDentro(ctx.vista)) {
    residir(ctx);
    salir(ctx);
  } else {
    enColumna(ctx);
  }
};
