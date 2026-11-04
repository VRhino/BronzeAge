// La suite completa del motor, optimizada y en paralelo (pedido del autor, 2026-11-04). Compila todas las pruebas del workspace con el perfil
// `suite` (opt-level 2 con debug_assert y desbordamientos, ver Cargo.toml) y lanza los binarios de prueba a la vez, de N en N, los más lentos
// primero. `cargo test` a secas sigue igual: una prueba suelta o un crate se lanzan como siempre.
//
//   node scripts/suite.mjs                 la suite entera
//   node scripts/suite.mjs --hilos 8       cuántos binarios a la vez (por defecto, los núcleos de la máquina)
//   node scripts/suite.mjs -p nucleo       solo los binarios de ese paquete (se puede repetir)
//   node scripts/suite.mjs -- <filtro>     lo que vaya detrás de `--` se pasa a cada binario (un filtro de nombre, --ignored...)
//
// Cada binario escribe su salida en target/suite-registros/<paquete>--<binario>.log; al final sale un resumen con las que fallaron y los más
// lentos, y los tiempos se guardan en target/suite-tiempos.json para ordenar la vez siguiente. Sale con código 1 si algo falla.
import { spawn } from 'node:child_process';
import { createWriteStream, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { availableParallelism } from 'node:os';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TARGET = path.join(RAIZ, 'target');
const REGISTROS = path.join(TARGET, 'suite-registros');
const TIEMPOS = path.join(TARGET, 'suite-tiempos.json');

// Argumentos.
let hilos = availableParallelism();
const paquetes = [];
let paraBinarios = [];
{
  const a = process.argv.slice(2);
  for (let i = 0; i < a.length; i++) {
    if (a[i] === '--') { paraBinarios = a.slice(i + 1); break; }
    if (a[i] === '--hilos' || a[i] === '-j') hilos = Math.max(1, parseInt(a[++i], 10) || 1);
    else if (a[i] === '-p' || a[i] === '--paquete') paquetes.push(a[++i]);
    else { console.error(`argumento desconocido: ${a[i]}`); process.exit(2); }
  }
}

const seg = (ms) => `${(ms / 1000).toFixed(1).replace('.', ',')} s`;
const min = (ms) => (ms < 90_000 ? seg(ms) : `${Math.floor(ms / 60_000)} min ${Math.round((ms % 60_000) / 1000)} s`);

// 1. Compilar y recoger los binarios de prueba.
function compilar() {
  return new Promise((resolve) => {
    const args = ['test', '--profile', 'suite', '--no-run', '--message-format=json-render-diagnostics'];
    if (paquetes.length) for (const p of paquetes) args.push('-p', p);
    else args.push('--workspace');
    console.log(`— compilando: cargo ${args.join(' ')}`);
    const t0 = Date.now();
    const cargo = spawn('cargo', args, { cwd: RAIZ, stdio: ['ignore', 'pipe', 'inherit'], env: { ...process.env, CARGO_INCREMENTAL: '0' } });
    const binarios = [];
    let resto = '';
    cargo.stdout.on('data', (d) => {
      const lineas = (resto + d).split('\n');
      resto = lineas.pop();
      for (const l of lineas) {
        if (!l.startsWith('{')) continue;
        const m = JSON.parse(l);
        if (m.reason === 'compiler-artifact' && m.profile?.test && m.executable) {
          const dir = path.dirname(m.manifest_path);
          const paquete = path.basename(dir);
          const tipo = m.target.kind[0];
          // nucleo--asentamientos para una prueba de tests/, nucleo--lib-nucleo para las de src/.
          const clave = `${paquete}--${tipo === 'test' ? '' : `${tipo}-`}${m.target.name}`;
          binarios.push({ paquete, clave, exe: m.executable, dir });
        }
      }
    });
    cargo.on('close', (codigo) => {
      if (codigo !== 0) { console.error(`— no compila (código ${codigo})`); process.exit(codigo ?? 1); }
      console.log(`— compilado en ${min(Date.now() - t0)}: ${binarios.length} binarios de prueba`);
      resolve(binarios);
    });
  });
}

// 2. Lanzar un binario y contar lo que dice.
function lanzar(b) {
  return new Promise((resolve) => {
    const log = path.join(REGISTROS, `${b.clave}.log`);
    const salida = createWriteStream(log);
    const t0 = Date.now();
    const p = spawn(b.exe, paraBinarios, { cwd: b.dir, env: { ...process.env, CARGO_MANIFEST_DIR: b.dir, CARGO_PKG_NAME: b.paquete } });
    let texto = '';
    const tomar = (d) => { salida.write(d); texto += d; };
    p.stdout.on('data', tomar);
    p.stderr.on('data', tomar);
    p.on('close', (codigo, senal) => {
      salida.end();
      const ms = Date.now() - t0;
      let pasan = 0, fallan = 0, ignoradas = 0;
      for (const m of texto.matchAll(/^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored/gm)) {
        pasan += +m[1]; fallan += +m[2]; ignoradas += +m[3];
      }
      const fallidas = [...texto.matchAll(/^test (\S+) \.\.\. FAILED$/gm)].map((m) => m[1]);
      // Un binario que muere sin decir su resultado (pánico fuera de una prueba, señal...) cuenta como fallo.
      const roto = codigo !== 0 && fallan === 0;
      if (roto) fallan = 1;
      resolve({ clave: b.clave, log, ms, pasan, fallan, ignoradas, fallidas, roto, codigo: codigo ?? senal });
    });
  });
}

const binarios = await compilar();
let tiempos = {};
try { tiempos = JSON.parse(readFileSync(TIEMPOS, 'utf8')); } catch { /* primera vez */ }
// Los más lentos primero, y los que no tienen tiempo aún antes que todos (pueden ser lentos).
binarios.sort((x, y) => (tiempos[y.clave] ?? 1e9) - (tiempos[x.clave] ?? 1e9));

rmSync(REGISTROS, { recursive: true, force: true });
mkdirSync(REGISTROS, { recursive: true });
console.log(`— lanzando ${binarios.length} binarios, ${hilos} a la vez`);
const t0 = Date.now();
const resultados = [];
let siguiente = 0;
async function obrero() {
  while (siguiente < binarios.length) {
    const r = await lanzar(binarios[siguiente++]);
    resultados.push(r);
    const marca = r.fallan ? '✗' : '✓';
    console.log(`${marca} [${resultados.length}/${binarios.length}] ${r.clave}: ${r.pasan} bien${r.fallan ? `, ${r.fallan} mal` : ''} (${seg(r.ms)})`);
  }
}
await Promise.all(Array.from({ length: Math.min(hilos, binarios.length) }, obrero));
const total = Date.now() - t0;

// 3. Resumen.
const suma = (k) => resultados.reduce((s, r) => s + r[k], 0);
for (const r of resultados) tiempos[r.clave] = r.ms;
writeFileSync(TIEMPOS, JSON.stringify(tiempos, null, 1));
console.log('');
console.log(`— lentos: ${[...resultados].sort((a, b) => b.ms - a.ms).slice(0, 5).map((r) => `${r.clave} ${seg(r.ms)}`).join(' · ')}`);
const malos = resultados.filter((r) => r.fallan);
for (const r of malos) {
  console.log(`✗ ${r.clave}${r.roto ? ` terminó con ${r.codigo} sin resultado` : ''} → ${path.relative(RAIZ, r.log)}`);
  for (const f of r.fallidas) console.log(`    ${f}`);
}
console.log(`FIN: ${suma('pasan')} bien, ${suma('fallan')} mal, ${suma('ignoradas')} ignoradas; ${binarios.length} binarios en ${min(total)}`);
process.exit(malos.length ? 1 : 0);
