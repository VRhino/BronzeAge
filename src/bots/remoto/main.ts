// Arranque del proceso de bots (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §4): `npm run bots`. En Render, un servicio
// junto al servidor. Dos modos, según haya `BOTS_TOKEN`:
//
// **Con `BOTS_TOKEN` (servicio administrable).** Arranca inerte y abre el canal de control (`ws://…/control`, puerto `PORT`,
// 4000 por defecto) al que se conecta la pestaña «Bots» del cliente admin, que lo pone en marcha y lo maneja. Si estaba en
// marcha cuando se cayó, vuelve a ponerse solo. Las demás variables son opcionales y solo proponen valores al panel:
//
//   BOTS_TOKEN             secreto con el que el cliente admin se autentica en el canal de control
//   CODIGO_REGISTRO_BOTS   el código con que el servidor da de alta bots (si falta, lo manda el panel)
//   BOTS_SERVIDOR, BOTS_PARTIDA, BOTS_TOTAL, BOTS_DIAS_LLEGADA, BOTS_SEMILLA, BOTS_CADA_MS, BOTS_HORARIO   (valores del panel)
//   BOTS_DIRECTORIO        dónde guarda los registros de cuentas y su estado                (./datos)
//
// **Sin él (modo directo).** Juega ya, con la configuración del entorno:
//
//   BOTS_SERVIDOR          raíz del servidor (p. ej. https://bronzeage.onrender.com)        obligatoria
//   BOTS_PARTIDA           gameId de la partida                                              obligatoria
//   CODIGO_REGISTRO_BOTS   el mismo código que tiene el servidor para dar de alta bots       obligatoria
//   BOTS_REGISTRO          fichero del registro de cuentas        (./datos/bots-<partida>.json)
//   BOTS_TOTAL             cuántos bots llegan                                           (30)
//   BOTS_DIAS_LLEGADA      a lo largo de cuántos días                                     (3)
//   BOTS_SEMILLA           semilla de las llegadas, horarios y decisiones                 (1)
//   BOTS_CADA_MS           cada cuánto mira si avanzó el mundo                        (15000)
//   BOTS_HORARIO           `siempre` para tenerlos conectados todo el día (pruebas)   (por-semilla)
import { join } from 'node:path';
import { ProcesoDeBots } from './procesoDeBots';
import { ServicioDeBots } from '../control/servicio';
import { servidorDeControl } from '../control/servidorDeControl';

const entorno = (nombre: string): string | undefined => process.env[nombre]?.trim() || undefined;

function obligatoria(nombre: string): string {
  const valor = entorno(nombre);
  if (!valor) throw new Error(`falta ${nombre}`);
  return valor;
}
const numero = (nombre: string, porDefecto: number) => {
  const valor = Number(process.env[nombre]);
  return Number.isFinite(valor) && valor > 0 ? valor : porDefecto;
};
const horario = () => (entorno('BOTS_HORARIO') === 'siempre' ? 'siempre' : 'por-semilla');

const token = entorno('BOTS_TOKEN');
if (token) await servicioAdministrable(token);
else await modoDirecto();

async function servicioAdministrable(token: string): Promise<void> {
  const directorio = entorno('BOTS_DIRECTORIO') ?? './datos';
  const servicio = new ServicioDeBots({
    directorioRegistros: directorio,
    rutaEstado: join(directorio, 'servicio-bots.json'),
    codigoRegistroBots: entorno('CODIGO_REGISTRO_BOTS'),
    porDefecto: {
      servidor: entorno('BOTS_SERVIDOR'),
      partida: entorno('BOTS_PARTIDA'),
      total: numero('BOTS_TOTAL', 30),
      diasLlegada: numero('BOTS_DIAS_LLEGADA', 3),
      semilla: numero('BOTS_SEMILLA', 1),
      cadaMs: numero('BOTS_CADA_MS', 15_000),
      horario: horario(),
    },
  });
  const control = await servidorDeControl(servicio, { token, puerto: Number(process.env.PORT ?? process.env.BOTS_PUERTO ?? 4000) });
  console.log(`[bots] canal de control en :${control.puerto}/control`);

  for (const senal of ['SIGINT', 'SIGTERM'] as const) {
    process.once(senal, () => {
      // Apagar el proceso no es pedir que se paren: el estado deseado se queda como estaba y al volver retoma.
      void servicio.cerrar().then(() => control.cerrar()).finally(() => process.exit(0));
    });
  }
  try {
    if (await servicio.reanudarSiToca()) console.log('[bots] retoma lo que hacía antes de caerse');
  } catch (err) {
    console.error('[bots] no pudo retomar:', err instanceof Error ? err.message : err);
  }
}

async function modoDirecto(): Promise<void> {
  const gameId = obligatoria('BOTS_PARTIDA');
  const proceso = new ProcesoDeBots({
    servidor: obligatoria('BOTS_SERVIDOR'),
    gameId,
    codigoRegistroBots: obligatoria('CODIGO_REGISTRO_BOTS'),
    registro: entorno('BOTS_REGISTRO') ?? `./datos/bots-${gameId}.json`,
    total: numero('BOTS_TOTAL', 30),
    diasLlegada: numero('BOTS_DIAS_LLEGADA', 3),
    semilla: numero('BOTS_SEMILLA', 1),
    cadaMs: numero('BOTS_CADA_MS', 15_000),
    horario: horario(),
  });

  // Apagar el proceso es cerrar el cliente de todos sus bots: se desconectan y salen del mundo como un humano.
  for (const senal of ['SIGINT', 'SIGTERM'] as const) {
    process.once(senal, () => {
      void proceso.parar().finally(() => process.exit(0));
    });
  }

  await proceso.arrancar();
  console.log(`[bots] jugando en '${gameId}' contra ${process.env['BOTS_SERVIDOR']}`);
  await proceso.correr();
}
