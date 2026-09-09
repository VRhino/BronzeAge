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
    // El backend (Fastify) corre aparte, con todo bajo `/v1` (Fase C6: versionado del contrato). El proxy
    // evita CORS en desarrollo: desde el navegador `fetch('/v1/admin/...')` es same-origin. Por defecto
    // apunta a `:3000` (local); `BACKEND_URL` lo repunta a otra instancia (p.ej. la de Render) sin tocar
    // código — `changeOrigin` para que un backend HTTPS acepte el Host.
    proxy: {
      '/v1': { target: process.env.BACKEND_URL ?? 'http://localhost:3000', changeOrigin: true, ws: true },
    },
  },
});
