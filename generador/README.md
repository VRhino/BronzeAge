# Generador de mundo

Herramienta **fuera de línea** (TypeScript) que produce el artefacto de mundo del contrato
([`06_Contrato_de_Mundo.md`](../Docs/Coordinacion/06_Contrato_de_Mundo.md), [`world.proto`](../proto/bronzeage/v1/world.proto)).
Parte del generador regional de Codex (`codex/worldgen-relieve`, `4e6fd44`).

## Flujo

`src/relieve.ts` (relieve, hidrografía, costas, bosque y suelos) → `src/mundoEstrategico.ts` / `recursos.ts` / `contenidoVivo.ts`
(campamentos, metales, parcelas, recursos, bandidos y alijos) → vados y grafo de navegación → `src/artefacto.ts`.
Lo orquesta `scripts/generar-mundo.ts`.

Otros scripts:
- `scripts/mundo-estrategico.ts`: dibuja el mundo estratégico de una seed en SVG.
- `scripts/ver-artefacto.ts`: dibuja un artefacto leyendo solo el archivo.

## Uso

```bash
npm ci
npm run generar                                  # código de protobuf en src/gen
npx tsc -p tsconfig.json                         # tipos
npx vitest run                                   # pruebas
npx tsx scripts/generar-mundo.ts [seed] [carpeta]   # escribe mundo-<hash>.bin (algo más de un minuto)
```
