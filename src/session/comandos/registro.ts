// Registro de comandos por NOMBRE (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md, Fase B3 — migración de
// `main.ts`). `GameSession.ejecutar` ya anotaba que un registro así haría falta "cuando la API reciba
// comandos serializados" (ver comentario en `../gameSession.ts`, escrito pensando en Fase C) — se construye
// ahora, un ciclo de fase antes de lo previsto: sin él, los 31 comandos de partida no tienen forma de llegar
// al backend por HTTP (`server/api.ts` solo puede recibir un `tipo: string`, nunca una referencia a función).
//
// Un único lugar con los ~30 manejadores, en vez de mantenerlos duplicados en `app/gameStore.ts` (cliente,
// que ya no los necesita) y en `server/api.ts` (que sí). El cliente solo importa `TipoComando` (tipo, sin
// coste en tiempo de ejecución), nunca los manejadores en sí.
import type { ManejadorComando } from './tipos';
import { fundarAsentamiento } from './fundarAsentamiento';
import { crearFaccion } from './crearFaccion';
import { unirseAFaccion } from './unirseAFaccion';
import { dejarFaccion } from './dejarFaccion';
import { alternarFaccionNpc } from './alternarFaccionNpc';
import { activarPolitica, asignarCargoLocal, asignarEmbajador, asignarRey, comprarCasa } from './cargos';
import { anexionar, fusionar, proponerRelacion, rebelionVasallo, romperRelacion } from './diplomacia';
import { colocarOrdenMercado, crearCaravana, proponerTrueque } from './comercio';
import {
  atacarCampamentoBandidos,
  combateCampoAbierto,
  interceptarCaravana,
  iniciarAsedio,
  reclutarTropa,
} from './militar';
import {
  alternarAutoConstruccion,
  anadirEdificioManualmente,
  calibrarReservaManual,
  mejorarEdificioAhora,
  moverEnCola,
  quitarDeCola,
  renombrarAsentamiento,
} from './construccion';
import { desarmarCaravanaFundacion, lanzarCaravanaFundacion } from './expansion';
import { abandonarRecinto, comprometerRecinto, mejorarRecinto } from './murallas';

export const REGISTRO_COMANDOS = {
  fundarAsentamiento,
  crearFaccion,
  unirseAFaccion,
  dejarFaccion,
  alternarFaccionNpc,
  activarPolitica,
  asignarCargoLocal,
  asignarEmbajador,
  asignarRey,
  comprarCasa,
  anexionar,
  fusionar,
  proponerRelacion,
  rebelionVasallo,
  romperRelacion,
  colocarOrdenMercado,
  crearCaravana,
  proponerTrueque,
  atacarCampamentoBandidos,
  combateCampoAbierto,
  interceptarCaravana,
  iniciarAsedio,
  reclutarTropa,
  alternarAutoConstruccion,
  anadirEdificioManualmente,
  calibrarReservaManual,
  mejorarEdificioAhora,
  moverEnCola,
  quitarDeCola,
  renombrarAsentamiento,
  desarmarCaravanaFundacion,
  lanzarCaravanaFundacion,
  comprometerRecinto,
  abandonarRecinto,
  mejorarRecinto,
} satisfies Record<string, ManejadorComando<any, any>>;

export type TipoComando = keyof typeof REGISTRO_COMANDOS;

/** Parámetros que espera el manejador de `tipo`, recuperados del `satisfies` de arriba — es lo que permite
 * que un comando serializado por nombre (`apiCliente.ejecutarComando`, `GameStore.despachar`) siga
 * comprobando en tiempo de compilación que `params` tiene la forma correcta, en vez de perderla en `unknown`
 * en el borde cliente-servidor. */
export type ParamsDe<T extends TipoComando> = Parameters<(typeof REGISTRO_COMANDOS)[T]>[3];

/** Datos que devuelve el manejador de `tipo` en `ResultadoComando.datos` cuando acepta el comando. */
export type DatosDe<T extends TipoComando> = ReturnType<(typeof REGISTRO_COMANDOS)[T]>['resultado']['datos'];
