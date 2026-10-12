// Dónde se va el tiempo de guardar el snapshot de una partida (contrato 03 §13.9): sobre una COPIA, no una partida viva.
//   npx tsx scripts/medicion-guardado.ts <directorio> <gameId>
import { performance } from 'node:perf_hooks';
import { crearAlmacenEnDisco } from '../src/server/almacen/enDisco';
import { cargarPartida, guardarPartida } from '../src/server/persistenciaPartida';

const [directorio, gameId] = process.argv.slice(2);
if (!directorio || !gameId) {
  console.error('uso: npx tsx scripts/medicion-guardado.ts <directorio> <gameId>');
  process.exit(2);
}
const almacen = crearAlmacenEnDisco(directorio);
const { sesion } = (await cargarPartida(almacen, gameId))!;
const mediana = (xs: number[]) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)] ?? 0;
const medir = async (f: () => Promise<unknown> | unknown, veces = 9) => {
  const t: number[] = [];
  for (let i = 0; i < veces; i++) {
    const t0 = performance.now();
    await f();
    t.push(performance.now() - t0);
  }
  return mediana(t);
};

const clave = `${gameId}.json`;
const texto = (await almacen.leer(clave))!;
const exportar = await medir(() => sesion.exportar());
const leerYParsear = await medir(async () => JSON.parse((await almacen.leer(clave))!));
const parsear = await medir(() => JSON.parse(texto));
const serializar = await medir(() => JSON.stringify(JSON.parse(texto))) - parsear;
const escribir = await medir(() => almacen.escribir(clave, texto));
const todo = await medir(() => guardarPartida(almacen, sesion, new Date().toISOString(), { forzar: false }));
const sinLectura = await medir(() => guardarPartida(almacen, sesion, new Date().toISOString(), { forzar: true }));

console.log(`'${gameId}': snapshot de ${(texto.length / 1024 / 1024).toFixed(1)} MiB (medianas de 9)`);
console.log(`  exportar            ${exportar.toFixed(1)} ms`);
console.log(`  leer + parsear      ${leerYParsear.toFixed(1)} ms   (solo para comprobar la versión; con forzar se salta)`);
console.log(`  serializar          ${serializar.toFixed(1)} ms`);
console.log(`  escribir (atómico)  ${escribir.toFixed(1)} ms`);
console.log(`  guardarPartida      ${todo.toFixed(1)} ms     sin comprobar la versión (forzar): ${sinLectura.toFixed(1)} ms`);
