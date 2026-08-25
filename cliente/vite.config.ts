import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';

// `@motor/*` es la ÚNICA costura entre este cliente y el backend: apunta al `src/` del repo servidor, donde
// viven `engine/`, `session/`, `domain/`, `world/`, `worldgen/` y `constants.ts`. Al sacar esta carpeta a su
// propio repositorio hay que repuntarla (paquete npm, submódulo o copia vendorizada) — ver README.md.
const MOTOR = fileURLToPath(new URL('../src', import.meta.url));

export default defineConfig({
  root: '.',
  resolve: {
    alias: [{ find: /^@motor\/(.*)$/, replacement: `${MOTOR}/$1` }],
  },
  // `main.ts` usa `await` de nivel de módulo (decisión de Fase B3: evita envolver el arranque en un IIFE).
  // El target por defecto de Vite no lo admite, así que se declara el mismo que ya usa `tsconfig.json` — sin
  // esto `vite build` falla, aunque `vite dev` funcione.
  build: { target: 'es2022' },
  server: {
    port: 5173,
    // El backend (Fastify) corre aparte, en :3000. El proxy evita CORS en desarrollo: desde el navegador
    // `fetch('/admin/...')` es same-origin. Al separar los repos, un cliente servido desde otro origen SÍ
    // necesitará CORS en el servidor (tarea C6 de la Fase C).
    proxy: {
      '/admin': 'http://localhost:3000',
      '/jugador': 'http://localhost:3000',
      '/sesiones': 'http://localhost:3000',
    },
  },
});
