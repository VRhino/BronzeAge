// Mide un tramo del batch real desde un punto de control de run-batch-sim.ts.
// Uso: npx tsx scripts/bench-batch-checkpoint.ts <checkpoint.json> [ticks=100]
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import type { EstadoSimulacion } from '../src/engine/simulation';
import { avanzarSimulacion } from '../src/engine/simulation';
import { avanzarNpcGobernanza, type ConfigNpcGobernanza } from '../src/session/npcGobernanza';
import { instanteDeTick, isoDeInstante } from '../src/session/estado';
import { EDIFICIO_CATALOGO, PERFILES_TRAZADO, TRAZADO, type PerfilTrazado } from '../src/constants';
import { crearMapa, type EstadoMapa } from '../src/world/mapa';
import { generarMapa, MAPA_DEFAULT, restaurarRng } from '../src/worldgen';

interface Checkpoint {
  version: number;
  seed: number;
  facciones: number;
  tick: number;
  estado: EstadoSimulacion;
  estadoMapa: EstadoMapa;
  estadoRng: number;
  contadorNpc: number;
  configNpc: ConfigNpcGobernanza;
  perfilForzado: string | null;
  trigoX: number;
}

const ruta = process.argv[2];
const ticks = Number(process.argv[3] ?? 100);
if (!ruta || !Number.isSafeInteger(ticks) || ticks <= 0) {
  throw new Error('Uso: npx tsx scripts/bench-batch-checkpoint.ts <checkpoint.json> [ticks=100]');
}
const cp = JSON.parse(readFileSync(ruta, 'utf8')) as Checkpoint;
if (cp.version !== 1) throw new Error(`Versión de checkpoint no soportada: ${cp.version}`);
if (cp.perfilForzado !== null) {
  if (!(PERFILES_TRAZADO as readonly string[]).includes(cp.perfilForzado)) throw new Error('Perfil inválido en checkpoint');
  TRAZADO.perfilForzado = cp.perfilForzado as PerfilTrazado;
}
if (cp.trigoX !== 1) {
  if (!Number.isFinite(cp.trigoX) || cp.trigoX <= 0) throw new Error('Multiplicador de trigo inválido');
  const granja = EDIFICIO_CATALOGO.granja as { produccionBaseTrigo?: number; niveles?: Record<number, { produccionBaseTrigo?: number }> };
  if (granja.produccionBaseTrigo !== undefined) granja.produccionBaseTrigo *= cp.trigoX;
  for (const nivel of Object.values(granja.niveles ?? {})) {
    if (nivel.produccionBaseTrigo !== undefined) nivel.produccionBaseTrigo *= cp.trigoX;
  }
}

const generado = generarMapa({ ancho: MAPA_DEFAULT.ancho, alto: MAPA_DEFAULT.alto, seed: cp.seed });
const mapa = crearMapa(generado, cp.estadoMapa);
const rng = restaurarRng(cp.estadoRng);
let estado = cp.estado;
let contadorNpc = cp.contadorNpc;
let msMotor = 0;
let msNpc = 0;
const inicio = performance.now();
for (let tick = cp.tick + 1; tick <= cp.tick + ticks; tick++) {
  const instante = instanteDeTick(tick);
  const contexto = { instante, momento: isoDeInstante(instante), rng };
  const t0 = performance.now();
  const trasMotorCrudo = avanzarSimulacion(estado, mapa, contexto);
  const trasMotor = trasMotorCrudo;
  const t1 = performance.now();
  const trasNpc = avanzarNpcGobernanza(trasMotor, mapa, contexto, { ...cp.configNpc, contadorInicial: contadorNpc });
  const t2 = performance.now();
  estado = trasNpc.estado;
  contadorNpc = trasNpc.contadorFinal;
  msMotor += t1 - t0;
  msNpc += t2 - t1;
}
const total = performance.now() - inicio;
const firma = createHash('sha256').update(JSON.stringify({
  estado,
  estadoMapa: mapa.estadoActual(),
  estadoRng: rng.estado(),
  contadorNpc,
})).digest('hex');
console.log(JSON.stringify({
  checkpoint: ruta,
  tickInicial: cp.tick,
  tickFinal: cp.tick + ticks,
  ticks,
  asentamientos: estado.asentamientos.length,
  edificios: estado.asentamientos.reduce((n, a) => n + a.edificios.length, 0),
  acuerdos: estado.acuerdos.length,
  caravanas: estado.caravanas.length,
  msPorTick: +(total / ticks).toFixed(3),
  msMotorPorTick: +(msMotor / ticks).toFixed(3),
  msNpcPorTick: +(msNpc / ticks).toFixed(3),
  firma,
}, null, 2));
