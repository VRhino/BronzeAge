// Arranque del proceso de bots (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §4): `npm run bots`. En Render, un Background
// Worker junto al servidor. Configuración por entorno:
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
import { ProcesoDeBots } from './procesoDeBots';

function obligatoria(nombre: string): string {
  const valor = process.env[nombre]?.trim();
  if (!valor) throw new Error(`falta ${nombre}`);
  return valor;
}
const numero = (nombre: string, porDefecto: number) => {
  const valor = Number(process.env[nombre]);
  return Number.isFinite(valor) && valor > 0 ? valor : porDefecto;
};

const gameId = obligatoria('BOTS_PARTIDA');
const proceso = new ProcesoDeBots({
  servidor: obligatoria('BOTS_SERVIDOR'),
  gameId,
  codigoRegistroBots: obligatoria('CODIGO_REGISTRO_BOTS'),
  registro: process.env['BOTS_REGISTRO']?.trim() || `./datos/bots-${gameId}.json`,
  total: numero('BOTS_TOTAL', 30),
  diasLlegada: numero('BOTS_DIAS_LLEGADA', 3),
  semilla: numero('BOTS_SEMILLA', 1),
  cadaMs: numero('BOTS_CADA_MS', 15_000),
  horario: process.env['BOTS_HORARIO'] === 'siempre' ? 'siempre' : 'por-semilla',
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
