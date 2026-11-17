import { defineConfig } from "vitest/config";
// Generar un mundo regional completo tarda decenas de segundos por caso.
export default defineConfig({ test: { include: ["src/**/*.test.ts"], testTimeout: 120_000 } });
