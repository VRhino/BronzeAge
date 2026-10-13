# Generador de mundo

Herramienta **fuera de línea** (TypeScript, decisión D3) que produce el artefacto de mundo del contrato
([`06_Contrato_de_Mundo.md`](../Docs/Coordinacion/06_Contrato_de_Mundo.md), [`world.proto`](../proto/bronzeage/v1/world.proto)).
No depende del motor del servidor.

## Origen

Punto de partida: el generador regional de Codex, rama `codex/worldgen-relieve` en el commit `4e6fd44`
(`src/worldgen`, 35 archivos, y los scripts `visor-worldgen.*` y `unity-regional.datos`). Codex no trabaja sobre esa rama mientras
Claude lo extiende aquí.

Cambios al traerlo:
- `src/tipos.ts` reúne los tipos que usaba de `domain/types` del motor viejo.
- Fuera, a propósito: `relieve_estrategico.test.ts` (usaba el `Mapa` y las rutas del motor viejo) y la auditoría de enclaves
  (`enclaves-regional.audit.ts`, usaba las zonas de influencia del motor viejo, radio 90). Los enclaves de estaño se rediseñan:
  gradiente por anillos y vetas (Doc 1.9c).
- No se traen `Tools/UnityRegional` ni `exports-regional` (siguen siendo de Codex).

## Estado

Base copiada y verde. Falta (fase 4): anillos por campamento, estaño y oro por gradiente, vetas, parcelas de bosque y la salida
como artefacto con hash.

## Uso

Usa el `node_modules` de la raíz mientras el motor viejo exista.

```bash
npx tsc -p generador/tsconfig.json     # tipos
cd generador && npx vitest run         # pruebas (unos 2-3 min)
```
