import { defineConfig } from 'vite';

// A diferencia de `cliente/vite.config.ts`, SIN ningún alias `@motor/*` — es la prueba de que este proyecto
// no depende del motor de ninguna forma. Ver README.md.
export default defineConfig({
  root: '.',
  server: {
    port: 5174,
    // El backend (Fastify) corre aparte, en :3000, con todo bajo `/v1`. El proxy evita CORS en desarrollo.
    proxy: {
      '/v1': 'http://localhost:3000',
    },
  },
});
