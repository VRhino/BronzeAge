# Docs/Coordinacion — el contrato entre el servidor y quien lo consume

Esta carpeta es **el único sitio** donde vive el contrato entre BronzeAge (servidor, Rust) y quien lo consume (Unity, admin web,
bots, servidores de batalla de Unity). Reglas de juego: `Docs/Game/` (canon). Cómo se calcula cada mecánica: `Docs/Mechanics/`.
Por qué se decidió así: `Consideraciones/`.

**Regla de la casa:** todo cambio de contrato (mensaje, campo, orden, evento, código de error) actualiza su `.proto` y su documento
**en el mismo commit**. Dentro de `bronzeage.v1` solo se añade (nuevos campos con número nuevo; nunca renumerar ni reutilizar); un cambio
que rompa es `bronzeage.v2` con plan de migración.

## Dónde está cada pieza

| Pieza | Fuente de verdad | Documento |
|---|---|---|
| **Sesión en tiempo real**: handshake, ámbitos, foto/delta, órdenes con respuesta causal, reloj sincronizado, reanudación por versión | [`proto/bronzeage/v1/session.proto`](../../proto/bronzeage/v1/session.proto) | [`05_Contrato_de_Sesion.md`](05_Contrato_de_Sesion.md) |
| **Modelo de datos completo** (todas las entidades) | `entities.proto`, `battle.proto`, `common.proto` | [`01_Modelo_de_datos.md`](01_Modelo_de_datos.md) |
| **Órdenes del cliente** (135) | `orders.proto` | 05 §6; forma de params en [`Mechanics/Comandos_Parametros.md`](../Mechanics/Comandos_Parametros.md) |
| **Eventos de dominio** | `events.proto` | 05 §8 y las tablas de eventos de `Docs/Mechanics` |
| **Batalla de Unity** | `battle.proto` | [`Mechanics/Batalla_Unity_Ciclo.md`](../Mechanics/Batalla_Unity_Ciclo.md) |
| **Mundo** (artefacto por hash y paquete de Unity) | `world.proto` | [`06_Contrato_de_Mundo.md`](06_Contrato_de_Mundo.md) |
| Propuestas entrantes de Codex/Conquest (`BA-*`) y su cierre | [`propuestas/`](propuestas/README.md) | [`propuestas/CIERRE_FASE_2.md`](propuestas/CIERRE_FASE_2.md) |

El contrato HTTP y de canales del servidor TypeScript (`01` modelo JSON, `02` HTTP, `03` tiempo real por segmentos, `04` migración) está en
`typescript-final:Docs/Coordinacion/`.

## Cómo añadir algo al contrato

1. Cambia el `.proto` (solo aditivo en v1).
2. Actualiza el documento (01 si es una entidad; 05 si es protocolo, orden o evento; 06 si es mundo) **en el mismo commit**.
3. Si es un mensaje o entidad que cruza a Unity, añade su fichero dorado y su prueba de lectura en los tres lenguajes.
4. Comprueba que el `.proto` sigue cargando (`protoc` o equivalente) y que no hay valores de enumeración repetidos en el paquete.
