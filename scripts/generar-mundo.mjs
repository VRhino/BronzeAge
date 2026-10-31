// Genera el artefacto de mundo que carga el servidor (`generador/salida/mundo/mundo-<hash>.bin`) y su paquete de relieve para los clientes 3D
// (`relieve-<hash>.bin`, al lado). Lo lanza el panel («generar-mundo») o `node scripts/generar-mundo.mjs [semilla]`. Instala las dependencias
// del generador si faltan, genera los tipos del contrato y escribe los dos ficheros; tarda unos dos minutos con 4 hilos. Con la misma semilla
// y el mismo código salen los mismos hashes.
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const GENERADOR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../generador');
const semilla = process.argv[2] ?? process.env.BRONZEAGE_SEMILLA ?? '42';

function paso(titulo, comando) {
  console.log(`— ${titulo}: ${comando}`);
  const r = spawnSync(comando, { cwd: GENERADOR, shell: true, stdio: 'inherit' });
  if (r.status !== 0) {
    console.error(`— falló «${titulo}» (código ${r.status})`);
    process.exit(r.status ?? 1);
  }
}

if (!existsSync(path.join(GENERADOR, 'node_modules'))) paso('dependencias del generador', 'npm ci');
paso('tipos del contrato', 'npm run generar');
paso(`mundo con la semilla ${semilla} (unos 2 min)`, `npx tsx scripts/generar-mundo.ts ${semilla} salida/mundo`);
console.log('— listo: el artefacto y su relieve están en generador/salida/mundo; ya se puede arrancar bronzeage-servidor');
