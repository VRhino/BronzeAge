import { mkdirSync, writeFileSync } from 'node:fs';
import { availableParallelism } from 'node:os';
import { join } from 'node:path';
import { Worker } from 'node:worker_threads';
import { create } from '@bufbuild/protobuf';
import type { CampoElevacion } from './elevacion';
import { TeselaRelieveSchema, type TeselaRelieve } from './gen/bronzeage/v1/world_pb';
import { construirPaqueteRelieve, dimensionesRelieve } from './paqueteRelieve';

// `tsx` no se registra dentro de los hilos: cada hilo lo registra él mismo y luego carga su módulo en TypeScript.
const ARRANQUE_HILO = `import(${JSON.stringify(import.meta.resolve('tsx/esm/api'))})
  .then(({ register }) => { register(); return import(${JSON.stringify(new URL('./relieveFinoHilo.ts', import.meta.url).href)}); })`;

/**
 * El paquete de relieve de todo el mapa (`paqueteRelieve.ts`), repartido en `hilos` hilos: cada hilo pide la siguiente tesela
 * cuando acaba la suya (las de mar cuestan poco y las del macizo, mucho). El resultado no depende del reparto.
 * `ancho` y `alto` en unidades estratégicas.
 */
export async function generarPaqueteRelieve(campo: CampoElevacion, ancho: number, alto: number, hashArtefacto: string,
  hilos = availableParallelism()): Promise<{ bytes: Uint8Array; hash: string }> {
  const d = dimensionesRelieve(ancho, alto), total = d.teselasX * d.teselasY, teselas: TeselaRelieve[] = new Array(total);
  let siguiente = 0;
  await Promise.all(Array.from({ length: Math.max(1, Math.min(hilos, total)) }, () => new Promise<void>((resolver, fallar) => {
    const w = new Worker(ARRANQUE_HILO, { eval: true, workerData: { campo, d } });
    w.on('message', (m: { k: number; altura?: Uint8Array; agua?: Uint8Array }) => {
      if (m.k >= 0) teselas[m.k] = create(TeselaRelieveSchema, { altura: m.altura!, agua: m.agua! });
      w.postMessage(siguiente < total ? siguiente++ : null);
    });
    w.on('error', fallar);
    w.on('exit', c => (c === 0 ? resolver() : fallar(new Error(`un hilo del relieve acabó con el código ${c}`))));
  })));
  return construirPaqueteRelieve(d, teselas, hashArtefacto);
}

/** Escribe `relieve-<hash del artefacto>.bin` junto al artefacto. */
export async function escribirPaqueteRelieve(campo: CampoElevacion, ancho: number, alto: number, hashArtefacto: string, dir: string,
  hilos?: number): Promise<{ ruta: string; hash: string; bytes: number }> {
  const p = await generarPaqueteRelieve(campo, ancho, alto, hashArtefacto, hilos);
  mkdirSync(dir, { recursive: true });
  const ruta = join(dir, `relieve-${hashArtefacto}.bin`);
  writeFileSync(ruta, p.bytes);
  return { ruta, hash: p.hash, bytes: p.bytes.length };
}
