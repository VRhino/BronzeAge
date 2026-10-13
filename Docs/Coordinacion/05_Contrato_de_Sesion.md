# Contrato 05: la sesión en tiempo real

> **Estado (2026-10-13): diseño cerrado de la fase 2, sin código.** Sustituye a los contratos 02 (HTTP) y 03 (tiempo real por
> segmentos) del servidor TypeScript, que quedan en `typescript-final`. El esquema vive en [`proto/bronzeage/v1/`](../../proto/bronzeage/v1/)
> (Protobuf, D8) y es **la fuente de verdad**: este documento explica el protocolo y las garantías; si se contradicen, gana el `.proto`.
> Contexto y problemas que resuelve: [`Consideraciones/Reconstruccion_Definicion.md`](../../Consideraciones/Reconstruccion_Definicion.md) §2.1, P3, P5, P12.

## 1. Qué es

**Una sola sesión persistente por cliente** que trae todo lo que el jugador ve (mapa, héroe, plaza, campamento, batalla,
avisos) y por la que van sus órdenes. Nada sondea ni pide la «proyección entera». Lo usan Unity (cliente final), el admin web,
los bots (Rust, sin privilegios: el mismo contrato que un jugador) y los servidores de batalla de Unity (sesión con otro rol).

| Fichero | Contenido |
|---|---|
| `common.proto` | Convenciones, `Punto`, `Plan`, `Ritmo`, `Existencia`, `Sigilo`, `Ambito` |
| `entities.proto` | **Modelo de datos completo**: todas las entidades del dominio (ver [`01_Modelo_de_datos.md`](01_Modelo_de_datos.md)) |
| `orders.proto` | Las 135 órdenes del cliente y la envoltura `Orden` |
| `events.proto` | `EventoDominio` |
| `battle.proto` | Ciclo de batalla de Unity (ticket, asignación, tokens, inicio, salida, resultado) y la entidad `Batalla` |
| `session.proto` | Mensajes de cliente y de servidor, foto/delta, resultado causal, reloj, reanudación |
| `world.proto` | Contrato de mundo: artefacto y paquete Unity ([`06_Contrato_de_Mundo.md`](06_Contrato_de_Mundo.md)) |

## 2. Transporte y apertura

- **WebSocket con tramas binarias**: una trama = un mensaje Protobuf (`MensajeDeCliente` hacia el servidor, `MensajeDeServidor`
  de vuelta). Compresión **zstd** negociada en `Hola.compresiones`.
- **Autenticación**: el cliente obtiene un `token` del servicio de acceso (fuera del alcance del contrato: fase 3) y lo manda en
  `Hola`. Un token inválido o caducado cierra la sesión con `ErrorSesion{token_invalido, fatal}`.
- **El primer mensaje del cliente es `Hola`**; el servidor responde `Bienvenida` (sesión, héroe, Facción, `InfoMundo`, reloj
  y los ámbitos suscritos con la decisión de reanudación). Cualquier otra cosa antes de `Hola` cierra la sesión.
- **Versión de protocolo** (`version_protocolo` = 1). Reglas de evolución en §11.
- **El artefacto de mundo no viaja por la sesión**: el cliente lo descarga por hash desde `InfoMundo.url_artefacto` (Unity, desde
  `url_paquete_unity`) y lo guarda en disco. Si ya tiene el hash vigente (`Hola.mundo_hash_conocido`), no descarga nada. Si el
  hash cambió, el cliente no puede entrar hasta tener el nuevo.

## 3. Ámbitos y qué recibe cada cliente

El servidor suscribe el cliente a los ámbitos que le corresponden; **el cliente nunca los elige** (salvo el interés del mapa):

| Ámbito | Contenido | Cuándo |
|---|---|---|
| `HEROE` (id) | el héroe propio, sus escuadras, loadouts, inventario | siempre |
| `FACCION` (id) | la Facción propia, sus relaciones, propuestas, tecnologías, memoria de la niebla, sus plazas y pueblos | si tiene Facción |
| `ASENTAMIENTO` (id) | la plaza completa (almacén, cola, edificios, cargos, mercado) | mientras el héroe está dentro |
| `CAMPAMENTO` (id) | el campamento de mercenarios (mercado, planta, residentes) | mientras está dentro |
| `BATALLA` (id) | la batalla en la que combate | mientras combate |
| `VISTA` | el mundo visible a la Facción, filtrado por la niebla, **dentro del rectángulo que fija `CambiarVista`** | siempre |
| `ADMIN` | todo el estado, sin niebla | solo sesiones de administración |

Al salir de la plaza o del campamento, o al acabar la batalla, el servidor manda `Revocado{ambito}` y el cliente **descarta**
las entidades de ese ámbito. La visibilidad de lo ajeno la decide el servidor ([`Mechanics/Niebla_De_Guerra.md`](../Mechanics/Niebla_De_Guerra.md)):
la plaza ve solo su zona, las avanzadas ven 12, una columna ve 30 (15 si va sola), y lo ajeno solo se manda si se ve.

## 4. El reloj sincronizado

El mundo no tiene tick: `Reloj.mundo_ms` es el instante de mundo, `factor_milesimas` el ritmo (1000 = tiempo real) y `pausado`
indica si está parado.

1. En `Bienvenida` y en cada `Pong` el servidor manda su `Reloj`.
2. El cliente manda `Ping{cliente_envio_ms}` al abrir y cada pocos segundos; el `Pong` devuelve el eco.
3. Con `rtt = recepcion - cliente_envio_ms` estima `desfase = reloj.mundo_ms - (cliente_envio_ms + rtt/2)` y lo suaviza con el
   mínimo de las últimas muestras. La hora de mundo local es `reloj_local_monotono + desfase`.
4. Con esa hora **el cliente evalúa los planes y los ritmos en cada fotograma**: `posicion(t)` interpola el `Plan`; una cantidad es
   `valor(t)` del `Ritmo`. El servidor no manda posiciones ni cantidades por intervalo.
5. Un cambio de factor o una pausa llegan como `Aviso{reloj.cambiado, reloj}`.

## 5. Foto, delta y cambios parciales

- **`Foto`**: estado completo de un ámbito en una versión. Se manda al suscribirse, al reanudar tarde, tras un hueco de secuencia
  y tras un `Revocado` seguido de una nueva suscripción.
- **`Delta{ambito, desde, hasta, cambios, eventos}`**: lo que cambió entre dos versiones. `desde` es la última versión que el
  cliente tiene de ese ámbito: si no coincide con la suya, el cliente pide reanudar.
- **`Cambio`**: `ACTUALIZA` (crea o actualiza) o `QUITA`. Una actualización es **parcial**: solo los campos que cambiaron
  (presencia de Protobuf). Los campos repetidos y los mapas, cuando cambian, vienen **completos**. Un campo opcional se borra
  listándolo en `campos_borrados`. Esto elimina por diseño los «campos borrados que llegan como `undefined`».
- **`Entidad.version`**: cada entidad lleva la versión de mundo en que cambió por última vez; el almacén normalizado del cliente
  descarta cualquier actualización con `version` menor que la que ya tiene.
- Cada `Delta` y cada `Foto` pueden llevar `NieblaProyectada` (dos máscaras: explorado y visible ahora).
- **Qué viaja de un movimiento**: lo propio, el `Plan` completo una sola vez (un cambio de plan llega como plan nuevo que empieza
  donde está la columna: ni parada ni salto); lo ajeno, solo el **tramo** del plan dentro de la vista y con horizonte corto (~30 s),
  renovado. Una columna que anda no cuesta ningún mensaje mientras anda.
- **Economía, obras y colas**: `Ritmo` («cantidad en t₀ + ritmo») y «termina en t». El servidor no manda nada hasta que un ritmo cambia.

## 6. Órdenes y respuesta causal

```
cliente → Orden { peticion_id, version_esperada?, cuerpo }
servidor → [Delta* con los cambios que produjo]  →  ResultadoOrden { peticion_id, resultado, version_tras, rechazo?, creadas[], datos? }
```

- **Causalidad**: el servidor envía `ResultadoOrden` **después** de toda `Delta` con `hasta ≤ version_tras` que afecte a los
  ámbitos del cliente. Al recibir `ORDEN_ACEPTADA`, el cliente ya tiene el estado nuevo: **no vuelve a pedir nada**.
- **Resultados**: `ORDEN_ACEPTADA`, `ORDEN_RECHAZADA` (con `RechazoOrden{codigo, mensaje, detalle}`), `ORDEN_SIN_CAMBIOS`
  (válida pero no cambió nada; no sube la versión) y `ORDEN_DUPLICADA` (misma `peticion_id`).
- **Idempotencia**: el servidor recuerda las últimas `peticion_id` de la sesión (y las de la partida para reconexiones). Reenviar una
  orden tras reconectar no la ejecuta dos veces.
- **`version_esperada`**: si viene y el mundo ya cambió más allá, la orden se rechaza con `version.obsoleta`; útil para acciones que
  dependen de lo que el jugador estaba viendo.
- **Orden sin sesión de héroe** (p. ej. administración): el rol lo decide el `token`.
- **Rechazos**: aparte de la validación de forma (la hace el parseo del `.proto`), el núcleo rechaza por autorización
  (`no_autorizado`, `fuera_del_mundo`, `batalla.bloqueado`), por reglas del dominio (códigos estables como `ascenso.invalido`)
  y por la regla de guerra: `atacar` sin `declara_guerra: true` contra quien no está en guerra se rechaza con el código
  `guerra.confirmar` y en `detalle` las consecuencias (contra quién queda la Facción en guerra y quién entra por arrastre).
- **Límite de pendientes**: `Bienvenida.maximo_ordenes_pendientes`; pasarse es `ErrorSesion{demasiadas_ordenes}`.

## 7. Reanudación por versión

1. El cliente guarda, por ámbito, la última versión que aplicó y su estado normalizado en caché local.
2. Al reconectar manda `Hola.versiones_conocidas` (clave `"TIPO:id"`).
3. Por cada ámbito el servidor decide y lo anuncia en `Bienvenida.ambitos`: `REANUDA_SIN_NOVEDAD`, `REANUDA_CON_DELTAS` (manda
   el tramo) o `REANUDA_CON_FOTO` (versión demasiado vieja, ámbito nuevo o cambio de grupo de visión).
4. Para poder mandar deltas, el servidor conserva en memoria las deltas de cada ámbito durante un tiempo de retención y, más allá,
   las reconstruye del registro de eventos; si no puede, foto.
5. Un cliente que detecta un hueco de `secuencia` reanuda con el mismo procedimiento, sin cerrar la sesión de red.
6. **El reloj de mundo se congela mientras el servidor está apagado** ([`Mechanics/Reloj_Y_Eventos.md`](../Mechanics/Reloj_Y_Eventos.md)): una
   reapertura tras una caída no trae «tiempo perdido» que simular.

La reapertura de Unity pinta el último estado desde la caché local **antes** de que llegue la sesión, y la sesión lo pone al día.

## 8. Eventos de dominio

`EventoDominio` es un **hecho ocurrido** (crónica, avisos, narración), nunca el cambio de estado: ese viaja como entidades en la misma
`Delta`. Llevan `codigo` (`familia.nombre`), `version`, `en_ms`, `asentamiento_id` (quién lo oye) y `heroes_ids` (los nombrados lo oyen
aunque no vean la plaza), y un `payload` estructurado.

- **El catálogo de códigos y la forma del payload de cada uno son las tablas de eventos de cada documento de `Docs/Mechanics`**
  (p. ej. `Movimiento_Ejercitos` §13.1, `Comercio_Caravanas` §13, `Presencia_Y_Convocatoria`, `Diplomacia_Facciones`). Un test del
  contrato exigirá que todo código emitido figure en el catálogo del archivo común de datos de juego.
- **Códigos nuevos de la reconstrucción**: `guerra.declarada_por_ataque`, `caravana.disputa`, `caravana.aguante`, `pueblo.marcado`,
  `pueblo.completado`, `pueblo.saqueado`, `pueblo.destruido`, `acarreo.sale`, `acarreo.llega`, `acarreo.perdido`, `avanzada.colocada`,
  `avanzada.mejorada`, `avanzada.en_ruinas`, `avanzada.destruida`, `obra.material_entregado`, `obra.cancelada`. Se retiran
  `ejercito.repliegue` y `ejercito.regresa`.
- **Audiencia** (quién oye cada evento): los sin plaza son globales; los de una plaza, para su Facción y para quien esté dentro; los
  que nombran a un héroe, para él. Sin cambios respecto al motor anterior.

## 9. Sesión de servidor de batalla

Un servidor de batalla de Unity abre una sesión con `Hola.cliente = CLIENTE_SERVIDOR_BATALLA` y un token de servicio. Recibe
`MensajeDeServidor.batalla` (`BattleTicket`, `IncorporacionBatalla`, `TokensBatalla`) y responde con `MensajeDeCliente.batalla`
(`BattleServerAssignment`, `InicioBatalla`, `SalidaBatalla`, `BattleResult`). El ciclo (estados, plazos, candado de escuadras,
idempotencia del resultado) es el de [`Mechanics/Batalla_Unity_Ciclo.md`](../Mechanics/Batalla_Unity_Ciclo.md); el terreno de la batalla se
toma del artefacto de mundo (`BattleMapReference.mapa_id` = hash). **Modalidades nuevas**: asalto de caravana `disputa` / `aguante`,
saqueo de pueblo y ataque a una avanzada ([`Combate.md`](../Mechanics/Combate.md) §7). Unity decide lo de dentro; el mundo sigue mientras se lucha.

## 10. Errores de sesión

`ErrorSesion{codigo, mensaje, fatal}`: `token_invalido`, `version_protocolo`, `hueco_de_secuencia` (no fatal: reanudar),
`demasiadas_ordenes`, `partida_no_disponible`, `mundo_distinto` (el hash cambió), `rol_no_permitido`, `interno`.

## 11. Evolución del contrato

- **Solo aditiva dentro de la misma versión**: nuevos campos con número nuevo, nuevos mensajes, nuevos valores de enum. **Nunca**
  renumerar, cambiar el tipo de un campo ni reutilizar un número; lo retirado se marca `reserved`.
- Un cambio que rompa → `bronzeage.v2` y plan de migración.
- Los valores de catálogo (recursos, edificios, tropas, tecnologías, políticas) son cadenas del archivo común de datos de juego,
  no enumeraciones: **añadir contenido no cambia el contrato**.
- Generación de código para Rust (servidor, bots), C# (Unity) y TypeScript (admin web, generador). Cada repositorio vendrá el `proto/`
  como submódulo o paquete versionado.
- **Pruebas de contrato**: ficheros dorados (un mensaje de cada tipo en binario y en texto) que los tres lenguajes deben leer y
  escribir idénticos; un test por repositorio.

## 12. Qué cambia respecto al contrato TypeScript

| Antes (02/03) | Ahora (05) |
|---|---|
| HTTP + canales WS de eventos + sondeo de la proyección | Una sesión WS; nada fuera de ella |
| Posiciones y progreso por tick | Planes y ritmos; el cliente evalúa con el reloj sincronizado |
| `GET` de la proyección tras cada orden | Respuesta causal: el estado nuevo llega antes que el «hecho» |
| `POST /tick`, `ritmo` de suscripción | Control del reloj por la orden `controlar_reloj` (administración) |
| JSON por esquema a mano; campos borrados llegan como `undefined` | Protobuf con presencia; `campos_borrados` explícito |
| Recargar rehace todo | Reanudación por versión con caché local |
| `replegarEjercito` | Retirada; cancelar la marcha es `estacionar_ejercito` |

## 13. Decisiones abiertas

1. **Payload de los eventos**: hoy `google.protobuf.Struct` con los campos documentados por código. La alternativa es un mensaje
   Protobuf tipado por cada uno de los ≈180 eventos. Recomiendo `Struct` ahora (son narración, no estado) y tipar solo los que un cliente
   necesite interpretar para actuar.
2. **Autenticación y acceso** (registro, claves, roles): fase 3, junto al servidor.
3. **Retención de deltas y tamaño del registro**: se fijan al construir el núcleo con mediciones.
