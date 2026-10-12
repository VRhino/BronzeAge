# 04 — Mundo reactivo (sin tick, servidor en Rust)

> **Estado (2026-10-12): sustituido por la reconstrucción.** Decisión del usuario: no hay migración por fases ni
> convivencia con el servidor TypeScript. El servidor se reconstruye entero en Rust sobre el mundo reactivo,
> en `main` (el último TypeScript queda en la rama `typescript-final`); el playtest se apaga y el cliente final es Unity. Problemas, decisiones (D1-D11) y orden:
> [`Consideraciones/Reconstruccion_Definicion.md`](../../Consideraciones/Reconstruccion_Definicion.md). Diseño del núcleo:
> [`Docs/Arquitectura/13_Mundo_Reactivo_Rust.md`](../Arquitectura/13_Mundo_Reactivo_Rust.md).

## Qué cambia para los clientes

| Servidor actual (tick, contrato 03) | Servidor reconstruido (contrato 05) |
|---|---|
| `progreso` + `avancePorTick` por tick; ajenos con `desplazamientoPorTick` | **Plan** de movimiento: vértices con instante de paso. Ajenos: tramo visible con horizonte corto |
| El cliente interpola un tick por detrás o extrapola | El cliente evalúa `posición(t)` del plan con el reloj sincronizado: exacto, sin retraso |
| Almacén, obras y colas cambian en cada tick | **Ritmos** (cantidad en t₀ + ritmo, instante de fin): el cliente anima en local |
| Órdenes por HTTP y proyección entera después | Órdenes por la sesión, con respuesta que llega después de sus cambios |
| Un delta por tick aunque nada cambie | Mensajes solo cuando cambia un plan o un ritmo |
| JSON | Protobuf, con tipos generados para Rust, C# y TypeScript |
| Canales de vista, fotos, deltas, `resync`, `revocado`, grupos por clave | **Se conservan** como envoltorio |

Clientes del contrato 05: Unity (cliente final), el admin web (herramienta) y los bots (Rust). El cliente web de
jugador se retira.
