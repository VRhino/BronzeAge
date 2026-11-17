// Hilo del paquete de relieve: pide teselas al hilo principal, las muestrea y las devuelve codificadas (ver `relieveFino.ts`).
import { parentPort, workerData } from 'node:worker_threads';
import { codificarTesela, muestrearTesela, type DimensionesRelieve } from './paqueteRelieve';
import type { CampoElevacion } from './elevacion';

const { campo, d } = workerData as { campo: CampoElevacion; d: DimensionesRelieve };
parentPort!.on('message', (k: number | null) => {
  if (k === null) { parentPort!.close(); return; }
  const { altura, agua } = muestrearTesela(campo, d, k % d.teselasX, Math.floor(k / d.teselasX));
  const t = codificarTesela(altura, agua, d.lado);
  parentPort!.postMessage({ k, altura: t.altura, agua: t.agua }, [t.altura.buffer as ArrayBuffer, t.agua.buffer as ArrayBuffer]);
});
parentPort!.postMessage({ k: -1 });
