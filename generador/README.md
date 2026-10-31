# Generador de mundo

Herramienta **fuera de línea** (TypeScript) que produce el artefacto de mundo del contrato
([`06_Contrato_de_Mundo.md`](../Docs/Coordinacion/06_Contrato_de_Mundo.md), [`world.proto`](../proto/bronzeage/v1/world.proto)).
Parte del generador regional de Codex (`codex/worldgen-relieve`, `4e6fd44`).

## Flujo

`src/relieve.ts` (relieve regional, hidrografía, costas, el detalle a escala de pantalla de `relieveDetalle.ts`, bosque,
suelos, la rejilla de celdas de 4 u y la conexión medida en ella) → `src/mundoEstrategico.ts` / `recursos.ts` / `contenidoVivo.ts`
(campamentos, metales, parcelas, recursos, bandidos y alijos) → vados y grafo de navegación → `src/artefacto.ts` →
el paquete de relieve de los clientes 3D, la altura cada 0,5 u (`src/paqueteRelieve.ts`; `src/relieveFino.ts` lo reparte en
hilos). Lo orquesta `scripts/generar-mundo.ts`, que escribe `mundo-<hash>.bin` y `relieve-<hash>.bin`.

Otros scripts:
- `scripts/mundo-estrategico.ts`: dibuja el mundo estratégico de una seed en SVG.
- `scripts/ver-artefacto.ts`: dibuja un artefacto leyendo solo el archivo.
- `scripts/datos-mesa-3d.ts`: saca la ventana de terreno de la mesa de escalas 3D
  (`Consideraciones/artefactos/mesa_escalas_3d`, ver `Consideraciones/Escalas_Mundo_3D_Definicion.md`).

## Uso

```bash
npm ci
npm run generar                                  # código de protobuf en src/gen
npx tsc -p tsconfig.json                         # tipos
npx vitest run                                   # pruebas
npx tsx scripts/generar-mundo.ts [seed] [carpeta]   # escribe mundo-<hash>.bin y relieve-<hash>.bin (unos dos minutos)
```
