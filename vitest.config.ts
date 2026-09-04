import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'node',
    /**
     * 30 s en vez de los 5 s por defecto.
     *
     * Buena parte de la suite construye CIUDADES DE VERDAD —cientos de ticks de simulación por caso, a veces
     * uno por semilla o por perfil de trazado— y eso ronda los 3 s aislado y los 5,5 s cuando la máquina va
     * cargada. O sea, justo encima de la línea: esos tests fallaban por reloj y no por regresión, y siempre
     * en la corrida en la que había algo más corriendo al lado.
     *
     * Se venía tapando caso por caso, con un timeout explícito en cada `it` que se caía (había cinco). Eso
     * solo desplazaba el problema al siguiente que creciera lo suficiente, y además convertía cada flake en
     * una investigación. Subir el techo una vez es lo que corresponde: 30 s sigue siendo un tope real —un
     * test colgado de verdad se detecta igual— pero deja de castigar a los que simplemente cuestan lo que
     * cuestan.
     *
     * Si un test se acerca a este número, el problema es el test, no este valor.
     */
    testTimeout: 30_000,
  },
});
