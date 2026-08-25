import { defineConfig } from 'vite';

export default defineConfig({
  root: '.',
  server: {
    port: 5173,
    // `main.ts`/`admin.ts` hablan con el backend (`npm run server`, Fastify en :3000) por HTTP — el proxy
    // evita tener que añadir CORS al servidor: desde el navegador, `fetch('/partidas/...')` es same-origin.
    proxy: {
      '/partidas': 'http://localhost:3000',
    },
  },
});
