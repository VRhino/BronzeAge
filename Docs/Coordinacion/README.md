# Docs/Coordinacion — todo lo que un cliente necesita saber del servidor

Esta carpeta es **el único sitio** donde vive el contrato entre BronzeAge (servidor) y quien lo consume (cliente
jugador, cliente admin, Unity/Conquest, servidores de batalla). Si un cliente necesita saber cómo se habla con el
servidor, está aquí o se enlaza desde aquí. Reglas de juego: `Docs/Game/` (canon, no se mueve). Cómo se calcula cada
mecánica: `Docs/Mechanics/`. Por qué se decidió así: `Consideraciones/`.

**Regla de la casa:** todo cambio de contrato (ruta, campo, mensaje WS, código de error) actualiza su documento **en el
mismo commit**. Los cambios en v1 son **solo aditivos**; lo que se retira se anota como retirado, no se borra en silencio.

## Dónde está cada pieza

| Pieza | Fuente de verdad | Documento |
|---|---|---|
| Modelo de datos compartido (Heroe, Escuadron, Batalla, ticket, resultado, campamentos, intel, fusión…) | `src/contratos/v1/dto.ts` + `contratos.schema.json` | [`01_Modelo_de_datos_compartido.md`](01_Modelo_de_datos_compartido.md) |
| Convenciones HTTP: login, registro, whoami, `/v1`, comandos y su éxito de transporte vs. de dominio, `409` | `src/server/rutas/sesiones.ts`, `comandos.ts` | [`02`](02_Contrato_Comunicacion_Servidor_Cliente.md) §2 |
| Endpoints de **jugador** (`/jugador/partidas/**`: lista, membresía, proyección, eventos, crónica, mapa, comandos, asignación de batalla) | `src/server/rutas/jugador.ts`, `batallas.ts` (la asignación) | 02 §2, §3.4, §4 |
| Endpoints de **admin** (`/admin/**`: partidas, estado completo, tick, exportar, mapa, membresías, métricas, código de registro, respaldos) | `src/server/rutas/admin.ts`, `respaldos.ts` | 02 §2 y §9 (inventario) |
| Endpoints **servidor a servidor** (`/batallas/**`, credencial `batalla-servidor`) | `src/server/rutas/batallas.ts` | 02 §3.2-§3.3 |
| Proyección de jugador (qué se ve, niebla, héroes públicos) | `src/session/proyecciones/jugador.ts` | 02 §4, 01 §19 |
| Catálogos y cifras de balance (`GET /v1/balance`) | `src/server/rutas/balance.ts` | 02 §4.2b |
| Comandos (nombre, `params`, rechazos, códigos de error) | `src/session/comandos/` (`REGISTRO_COMANDOS`, `esquemas.ts`, `codigosDeError.ts`) y OpenAPI | 02 §3.1, §4.2-§4.3 |
| Eventos de dominio y su audiencia | `src/domain/eventos.ts`, `src/session/canales.ts` | 02 §4.1b; canales de eventos: 02 §2 |
| **Tiempo real: canales de eventos** (`mapa/general`, `asentamiento/<id>`, `heroe/<id>`, `batalla/<id>`) y mensaje `tick` | `src/server/rutas/tiempoReal.ts`, `difusion/hub.ts`, `session/canales.ts` | 02 §2 (resumen) y §3.5 |
| **Tiempo real por segmentos de vista** (foto + deltas; sustituye al sondeo) | *por implementar* | [`03`](03_Tiempo_real_por_segmentos.md) |
| Formato de error y versión (`ERROR_RESPUESTA`, `ERROR_VERSION_INCOMPATIBLE`, `schemaVersion`) | `src/server/rutas/esquemas.ts` | 02 §5 |
| OpenAPI generado | `GET /v1/openapi.json` (reflejo del código, `src/server/openapi.ts`) | — |
| Schemas JSON y fixtures verificados por test | `src/contratos/v1/` | 02 §6 |
| Fixtures de contrato de tiempo real (ejemplos de mensajes WS) | [`fixtures/tiempo-real/`](fixtures/tiempo-real/) (a promover a `src/contratos/v1/fixtures/`) | 03 §5, §9 |
| Fixture geométrico de asentamiento (Unity) | [`fixtures/`](fixtures/README.md) (local, fuera de git salvo `tiempo-real/`) | `fixtures/README.md` |
| Propuestas entrantes de Codex/Conquest (BA-*) y su revisión | [`propuestas/`](propuestas/README.md) | ver estado abajo |

Los comentarios de cabecera de `src/server/rutas/*.ts` (p. ej. el protocolo WS de `tiempoReal.ts`) describen el
*porqué* del código, no el contrato: el contrato es lo de arriba. Cuando un comentario y un documento se contradigan,
gana el documento y se corrige el comentario.

## Estado de las propuestas (`propuestas/`)

Se dejan en su sitio: otros documentos (`Consideraciones/`, `Docs/Arquitectura/`, comentarios y tests de `src/`) enlazan
estas rutas, así que mover o archivar rompería referencias. Las aceptadas están absorbidas en 01/02; la propuesta es
el histórico de la decisión, el contrato vigente es 01/02.

| Propuesta | Estado escrito en el archivo | Dónde vive lo vigente |
|---|---|---|
| BA-001 Ciclo de batalla | ACEPTADO — implementado | 01 §15-§16, 02 §3 |
| BA-002 Identidades compartidas | ACEPTADO — implementado | 01 §0-§1 |
| BA-003 Worldgen y asentamientos 3D | ACEPTADO (híbrido) | 01 §11, §18 |
| BA-004 Adopción del modelo compartido | ACEPTADO — implementado | 01, 02 §6 |
| BA-005 Huellas y escala | ACEPTADO — implementado (`layoutVersion` 2) | `Docs/Mechanics/Trazado_Urbano.md`, fixture geométrico |
| BA-006 Progresión histórica y roster | LISTO_PARA_REVISION; Eras I-III implementadas, IV-V en diseño | `Consideraciones/BA-006_Revision_Tecnologia_Eras.md` |
| REVISION_CODEX_2026-09-11, REVISION_CONTRATOS_CODEX_2026-09-11 | RESUELTO (R01-R08 incorporadas a 02) | 02 (marcadas «corrección R0x») |

Las propuestas nuevas de Codex entran en `propuestas/` como hasta ahora (única carpeta que Codex edita).

## Cómo añadir algo al contrato

1. Cambia el código y su `schema` de ruta (OpenAPI se actualiza solo).
2. Actualiza 01 (si es una entidad) y/o 02 (si es ruta, comando, evento o campo de proyección) **en el mismo commit**;
   si es tiempo real, 03.
3. Si es un mensaje o entidad que cruza a Unity, añade su fixture y su definición en `src/contratos/v1/` (test incluido).
4. Solo aditivo en v1. Un cambio que rompa → nueva versión de ruta/contrato y plan de migración en este índice.
5. Comprobar que las referencias a rutas de `Docs/Coordinacion/` siguen válidas (`grep -rn "Coordinacion/" --include=*.md --include=*.ts`).
