# Propuestas de Codex/Conquest para BronzeAge

Carpeta de solicitudes `BA-*` que Codex detecta al trabajar en `Conquest_prototype`. Claude, como propietario de BronzeAge, también
mantiene esta carpeta (decisión del usuario, 2026-10-13): actualiza el estado de cada propuesta y **retira las cerradas**, dejando aquí
qué fue de ellas. El texto original de las retiradas está en el historial de git (`git log -- Docs/Coordinacion/propuestas`).

## Vigentes

- [BA-006 — Progresión histórica, tecnología y roster](BA-006_progresion_historica_y_roster.md): **pospuesta** con las Eras IV y V; se retoma con el motor nuevo. Las Eras I-III están en el canon (Doc 6).

## Cerradas y retiradas (2026-10-13)

Absorbidas en el contrato 05 de la reconstrucción; la correspondencia con los esquemas está en [`CIERRE_FASE_2.md`](CIERRE_FASE_2.md).

| Propuesta | Qué pasó | Dónde vive ahora |
|---|---|---|
| BA-001 Ciclo de batalla | Absorbida | `battle.proto`, `session.proto`, `Mechanics/Batalla_Unity_Ciclo.md` |
| BA-002 Identidades compartidas | Absorbida | los `id` de `entities.proto` |
| BA-003 Worldgen y asentamientos 3D | Sustituida | [`06_Contrato_de_Mundo.md`](../06_Contrato_de_Mundo.md), `world.proto` |
| BA-004 Modelo compartido | Absorbida | [`01_Modelo_de_datos.md`](../01_Modelo_de_datos.md), `entities.proto` |
| BA-005 Huellas y escala | Absorbida | el servidor calcula el trazado; ciudad −80 % |
| Revisiones de Codex (2026-09-11) | Resueltas y superadas por el contrato Protobuf | — |
