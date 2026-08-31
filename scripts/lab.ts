// Servidor del LABORATORIO VISUAL de trazado urbano (`lab/`).
//
// Qué es: el motor real corriendo EN EL NAVEGADOR, sin API, sin partida y sin red. Se funda un único
// asentamiento, se le dan materiales infinitos y se avanza el tick a mano para ver crecer la ciudad y el árbol
// de anclas en vivo. Existe porque los dos bugs más caros del trazado —el Mercado pegado al Centro Urbano
// (doc trazado §5.7) y las anclas huérfanas (Etapa 5)— los encontró el usuario MIRANDO LA PANTALLA, no la
// suite de tests.
//
// Por qué vive aquí y no en `cliente/`: se eliminó en `73a12dfb` precisamente porque ejecuta el motor y por
// tanto "no era un cliente y no podía aislarse por red ni acompañar a `cliente/` a otro repositorio". Aquel
// commit lo dejó en el historial "por si se rescata como herramienta de desarrollo de este repo" — que es
// exactamente lo que es ahora.
//
// Por qué esbuild y no vite: `cliente/` se lleva vite cuando salga a su repositorio, y el backend no debería
// heredar un bundler entero por una herramienta de depuración. esbuild basta y ya estaba en el árbol.
import { context } from 'esbuild';

const PUERTO = Number(process.env.LAB_PORT ?? 5180);

const ctx = await context({
  entryPoints: ['lab/src/main.ts'],
  bundle: true,
  format: 'esm',
  target: 'es2022',
  sourcemap: 'inline',
  outfile: 'lab/dist/main.js',
  logLevel: 'info',
});

// `serve` reconstruye en cada petición: recargar el navegador basta para ver un cambio del motor, sin proceso
// de watch ni recarga en caliente que mantener.
const { hosts, port } = await ctx.serve({ servedir: 'lab', port: PUERTO });
const host = hosts.includes('127.0.0.1') ? '127.0.0.1' : (hosts[0] ?? 'localhost');
console.log(`\nLaboratorio de trazado: http://${host}:${port}/\n`);
console.log('Motor real en el navegador, sin API ni red. Ctrl+C para parar.\n');
