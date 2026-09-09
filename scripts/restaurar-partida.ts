// Procedimiento de restauración de una partida desde un respaldo (Fase E2).
//
// Existe porque "prueba de restauración documentada" no se cumple con una función exportada que alguien
// tendría que envolver a mano en un momento de urgencia: se cumple con un comando que un operador puede
// ejecutar leyendo una línea de ayuda. La lógica vive en `server/respaldos.ts` y está cubierta por tests;
// esto es solo la cáscara de línea de comandos.
//
// **CON EL SERVIDOR PARADO.** `restaurarPartida` sustituye el archivo de snapshot, pero un proceso servidor
// que tenga la partida abierta conserva el estado viejo en memoria y lo escribiría encima al siguiente
// comando — la restauración quedaría deshecha sin que nadie viera un error. Se avisa aquí y se repite en el
// comentario de `restaurarPartida`, porque es la forma de usar esto mal que no da síntomas.
//
//   npx tsx scripts/restaurar-partida.ts <directorio> <gameId>              # lista los respaldos
//   npx tsx scripts/restaurar-partida.ts <directorio> <gameId> <archivo>    # restaura ese
import { listarRespaldos, restaurarPartida } from '../src/server/respaldos';
import { cargarPartida } from '../src/server/persistenciaPartida';

async function principal(): Promise<void> {
  const [directorio, gameId, archivo] = process.argv.slice(2);
  if (!directorio || !gameId) {
    console.error('uso: npx tsx scripts/restaurar-partida.ts <directorio> <gameId> [archivo-de-respaldo]');
    console.error('     sin el tercer argumento, lista los respaldos disponibles.');
    process.exit(2);
  }

  const respaldos = await listarRespaldos(directorio, gameId);
  if (respaldos.length === 0) {
    console.error(`no hay ningun respaldo de '${gameId}' en '${directorio}'.`);
    process.exit(1);
  }

  if (!archivo) {
    const vigente = await cargarPartida(directorio, gameId);
    console.log(`partida '${gameId}' — version vigente: ${vigente ? vigente.sesion.getState().version : '(sin snapshot)'}`);
    console.log(`\n${respaldos.length} respaldo(s), del mas reciente al mas antiguo:\n`);
    for (const r of respaldos) console.log(`  ${r.momento}   ${(r.bytes / 1024).toFixed(1)} KB   ${r.archivo}`);
    console.log('\npara restaurar, y CON EL SERVIDOR PARADO:');
    console.log(`  npx tsx scripts/restaurar-partida.ts ${directorio} ${gameId} ${respaldos[0]!.archivo}`);
    return;
  }

  const antes = await cargarPartida(directorio, gameId);
  const version = await restaurarPartida(directorio, gameId, archivo);
  console.log(`restaurada '${gameId}': version ${antes ? antes.sesion.getState().version : '?'} -> ${version}`);
  console.log('arranca el servidor de nuevo para que reabra la partida desde el snapshot restaurado.');
}

principal().catch((err) => {
  // El mensaje de `RespaldoInservibleError` ya explica qué pasó y, sobre todo, que el snapshot vigente NO se
  // tocó. Se imprime solo eso: una traza de pila no ayuda a quien está restaurando a las 3 de la mañana.
  console.error(err instanceof Error ? err.message : err);
  process.exit(1);
});
