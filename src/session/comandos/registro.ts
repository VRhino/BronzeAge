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
import { responderSolicitud, solicitarIngreso } from './ingresoEnFaccion';
import { dejarFaccion } from './dejarFaccion';
import { crearHeroe } from './crearHeroe';
import { abrirAlijo, aportarARefundacion, comprarCaravanaDeRefundacion, comprarEnCampamento, pedirPrestamo, reclutarEnCampamento, reponerPrestamo, retirarDeRefundacion } from './mercenarios';
import { asignarGuarnicion, borrarLoadout, guardarEnAlmacenPersonal, guardarLoadout, repartirPuntos, retirarGuarnicion, sacarDelAlmacenPersonal } from './heroe';
import { crearFaccionNpc } from './crearFaccionNpc';
import { activarPolitica, asignarCargoLocal, asignarEmbajador, asignarRey, cambiarResidencia, dejarResidencia, designarCapital, residirEnCampamento } from './cargos';
import { anexionar, declararGuerra, fusionar, proponerPaz, proponerRelacion, rebelionVasallo, romperRelacion } from './diplomacia';
import {
  aceptarTrueque,
  agregarCarroCaravana,
  cancelarCaravana,
  colocarOrdenMercado,
  comerciarEnPlaza,
  comprarAnimalCaravana,
  crearCaravana,
  enviarCaravanaAlOrigen,
  moverCargaCaravanaAparcada,
  moverCarroCaravana,
  prepararCaravana,
  proponerTrueque,
  rechazarTrueque,
  reservarCaravana,
} from './comercio';
import { iniciarAsedio, reclutarTropa } from './militar';
import {
  alternarAutoConstruccion,
  alternarReceta,
  anadirEdificioManualmente,
  calibrarReservaManual,
  mejorarEdificioAhora,
  moverEnCola,
  quitarDeCola,
  renombrarAsentamiento,
} from './construccion';
import { desarmarCaravanaFundacion, fundar, lanzarCaravanaFundacion } from './expansion';
import { abandonarRecinto, comprometerRecinto, mejorarRecinto } from './murallas';
import { solicitarAscenso } from './ascenso';
import { adoptarTecnologia } from './tecnologia';
import {
  adjuntarCaravana,
  alternarReabastecerAliados,
  cargarCaravana,
  entregarDeCaravana,
  estacionarEjercito,
  movilizarEjercito,
  replegarEjercito,
  soltarCaravana,
  unirseAEjercito,
} from './ejercitos';
import { conectarse, desconectarse, entrarEnAsentamiento, entrarEnCampamento, fijarPuerta, guarnecer, marcharA, salirAlMundo, salirDeAsentamiento, salirDelCampamento, vetarJugador } from './presencia';
import { cederLiderazgo, responderPeticionDeUnion, separarseDelEjercito, unirseEnCampo } from './columna';
import { atacar, dejarDePerseguir, inspeccionar, perseguir } from './interaccion';
import { aplicarResultado, cancelarBatalla, conCandadoDeBatalla, confirmarInicio, registrarAsignacion, registrarTokens, unirseABatalla } from './batalla';
import { avanzarTick } from './avanzarTick';

const MANEJADORES = {
  fundarAsentamiento,
  crearFaccion,
  solicitarIngreso,
  responderSolicitud,
  dejarFaccion,
  crearHeroe,
  repartirPuntos,
  guardarLoadout,
  borrarLoadout,
  asignarGuarnicion,
  guardarEnAlmacenPersonal,
  sacarDelAlmacenPersonal,
  retirarGuarnicion,
  crearFaccionNpc,
  activarPolitica,
  asignarCargoLocal,
  asignarEmbajador,
  designarCapital,
  asignarRey,
  cambiarResidencia,
  dejarResidencia,
  residirEnCampamento,
  reclutarEnCampamento,
  pedirPrestamo,
  reponerPrestamo,
  abrirAlijo,
  comprarEnCampamento,
  aportarARefundacion,
  retirarDeRefundacion,
  comprarCaravanaDeRefundacion,
  anexionar,
  fusionar,
  proponerRelacion,
  rebelionVasallo,
  romperRelacion,
  declararGuerra,
  proponerPaz,
  colocarOrdenMercado,
  crearCaravana,
  agregarCarroCaravana,
  comprarAnimalCaravana,
  reservarCaravana,
  prepararCaravana,
  cancelarCaravana,
  moverCarroCaravana,
  proponerTrueque,
  aceptarTrueque,
  rechazarTrueque,
  comerciarEnPlaza,
  moverCargaCaravanaAparcada,
  enviarCaravanaAlOrigen,
  salirAlMundo,
  marcharA,
  entrarEnAsentamiento,
  conectarse,
  desconectarse,
  salirDeAsentamiento,
  entrarEnCampamento,
  salirDelCampamento,
  guarnecer,
  fijarPuerta,
  vetarJugador,
  inspeccionar,
  atacar,
  perseguir,
  dejarDePerseguir,
  unirseEnCampo,
  responderPeticionDeUnion,
  separarseDelEjercito,
  cederLiderazgo,
  movilizarEjercito,
  unirseAEjercito,
  replegarEjercito,
  estacionarEjercito,
  alternarReabastecerAliados,
  adjuntarCaravana,
  soltarCaravana,
  cargarCaravana,
  entregarDeCaravana,
  iniciarAsedio,
  reclutarTropa,
  alternarAutoConstruccion,
  alternarReceta,
  anadirEdificioManualmente,
  calibrarReservaManual,
  mejorarEdificioAhora,
  moverEnCola,
  quitarDeCola,
  renombrarAsentamiento,
  desarmarCaravanaFundacion,
  lanzarCaravanaFundacion,
  fundar,
  comprometerRecinto,
  abandonarRecinto,
  solicitarAscenso,
  adoptarTecnologia,
  mejorarRecinto,
  unirseABatalla,
  cancelarBatalla,
} satisfies Record<string, ManejadorComando<any, any>>;

/** Todos llevan el candado de batalla (Doc 5.15.1) salvo los dos que actúan sobre la propia batalla. */
export const REGISTRO_COMANDOS = conCandadoDeBatalla(MANEJADORES, ['unirseABatalla', 'cancelarBatalla']);

export type TipoComando = keyof typeof REGISTRO_COMANDOS;

/** Parámetros que espera el manejador de `tipo`, recuperados del `satisfies` de arriba — es lo que permite
 * que un comando serializado por nombre (`apiCliente.ejecutarComando`, `GameStore.despachar`) siga
 * comprobando en tiempo de compilación que `params` tiene la forma correcta, en vez de perderla en `unknown`
 * en el borde cliente-servidor. */
export type ParamsDe<T extends TipoComando> = Parameters<(typeof REGISTRO_COMANDOS)[T]>[3];

/** Datos que devuelve el manejador de `tipo` en `ResultadoComando.datos` cuando acepta el comando. */
export type DatosDe<T extends TipoComando> = ReturnType<(typeof REGISTRO_COMANDOS)[T]>['resultado']['datos'];

/**
 * Todo lo que puede mutar una partida, por NOMBRE: los comandos de jugador, los mensajes del servidor de batalla
 * y las operaciones del sistema. Es lo que el diario de comandos (`server/diarioDePartida.ts`, doc 12 §5.1) anota
 * en cada línea y lo que usa para repasarla al recuperar.
 */
export const REGISTRO_DIARIO = {
  ...REGISTRO_COMANDOS,
  registrarAsignacion,
  confirmarInicio,
  registrarTokens,
  aplicarResultado,
  avanzarTick,
} satisfies Record<string, ManejadorComando<any, any>>;

export type TipoDiario = keyof typeof REGISTRO_DIARIO;

export type ParamsDeDiario<T extends TipoDiario> = Parameters<(typeof REGISTRO_DIARIO)[T]>[3];
export type DatosDeDiario<T extends TipoDiario> = ReturnType<(typeof REGISTRO_DIARIO)[T]>['resultado']['datos'];
