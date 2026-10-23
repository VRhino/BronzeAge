# Contrato 05: la sesión en tiempo real

> **Estado (2026-10-23): el esquema de 4H (T1) está en `proto/`, las órdenes se traducen (T2), la sesión abre los ámbitos con su autorización, `Revocado` y reanudación por versión (T3a), `HEROE` y `FACCION` llevan su contenido completo, con `QUITA` (T3b), la batalla de Unity va de extremo a extremo por el contrato (T4: ámbito `BATALLA`, ticket congelado, publicación a los servidores de batalla, acuses, token propio, asedio visible e informe de inspección) y `ASENTAMIENTO` y `CAMPAMENTO` llevan el interior de la plaza y del campamento, con cambios parciales con máscara y la tendencia «+N/min» (T3c) y, desde el 2026-10-23, con su propia secuencia interior por plaza (el paso de minuto no sube la versión del mundo, §7); falta el contenido de `VISTA` y `ADMIN` (T3d-T3e)** (ver `Docs/Arquitectura/4H_Contraste_y_Plan.md`).
> **Diseño cerrado de la fase 2 (2026-10-13), sin código.** Sustituye a los contratos 02 (HTTP) y 03 (tiempo real por
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
| `orders.proto` | Las 145 órdenes del cliente (135 de la fase 2 y 10 de 4H, `Orden.cuerpo` 145-154; la última es la orden genérica de administración) y la envoltura `Orden` |
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
- **Jugador sin héroe**: `Bienvenida.heroe_id` ausente, sin ámbitos y `campamentos_para_elegir` (de cada campamento de mercenarios: id, posición, origen, cuántos lo eligieron como inicial y cuántos héroes viven allí; el servidor Rust manda `eligieron_como_inicial = 0` hasta que el núcleo lo guarde). Es lo único que ve hasta que da `crear_heroe`, que le abre su ámbito en la misma sesión. Con héroe, `heroe_id` es su id (`heroe-N`) y `faccion_id`, la de su ciudadanía.
- **Versión de protocolo** (`version_protocolo` = 1). Reglas de evolución en §11.
- **Identificadores**: todas las entidades se nombran `<prefijo>-<n>` (la Facción también: `f-3`; ver `common.proto`). Dentro de una plaza, `ed-<plaza>-<n>` y `recinto-<plaza>-<n>`. La batalla, un UUID.
- **El artefacto de mundo no viaja por la sesión**: el cliente lo descarga por hash desde `InfoMundo.url_artefacto` (Unity, desde
  `url_paquete_unity`) y lo guarda en disco. Si ya tiene el hash vigente (`Hola.mundo_hash_conocido`), no descarga nada. Si el
  hash cambió, el cliente no puede entrar hasta tener el nuevo.

## 3. Ámbitos y qué recibe cada cliente

El servidor suscribe el cliente a los ámbitos que le corresponden; **el cliente nunca los elige** (salvo el interés del mapa). Cada ámbito se nombra `TIPO:id` con los ids de §2 (esa misma cadena es la clave de `Hola.versiones_conocidas`):

| Ámbito | Contenido | Cuándo (autorización) |
|---|---|---|
| `HEROE:heroe-N` | el héroe propio (con sus loadouts), sus escuadras, las columnas en que participa, las caravanas que lleva en su columna o que escolta con una escuadra suya y sus informes de inspección vigentes (inventario y equipo, vacíos hasta el catálogo de Conquest) | siempre que la cuenta tenga héroe (el de verdad; si solo tiene héroes de campo del andamiaje de administración, el primero) |
| `FACCION:f-N` | la Facción propia con sus ciudadanos, sus relaciones y sus tecnologías, sus plazas en resumen, su flota de caravanas, sus pueblos y avanzadas (y las obras de mapa que aún no lo son), sus trueques, sus órdenes de mercado, sus convocatorias y las batallas de los dos bandos en que está su Facción, resumidas si no combate (propuestas y memoria de la niebla: sistemas 5 y T3d) | si el héroe tiene Facción |
| `ASENTAMIENTO:as-N` | la plaza completa vista desde dentro y los pueblos de la plaza (con su almacén local): almacén y población en escalones con su tendencia, edificios y cola, recintos, trazado, políticas, ascenso, guarnición, quiénes están dentro... (abajo) | mientras el héroe está dentro **y** es ciudadano de la Facción dueña (dentro de una plaza ajena no se ve ningún interior) |
| `CAMPAMENTO:camp-N` | el campamento de mercenarios completo: reclutas, stock en venta, residentes y presentes, préstamos, tropas que ofrece y cupo diario | mientras está dentro |
| `BATALLA:<uuid>` | la batalla en la que combate, entera: ticket congelado, incorporaciones, resultado, su bando y su token (`asignacion_propia`) | mientras combate (el héroe de la sesión o el de campo de su cuenta) y el núcleo conserve la batalla (así ve el resultado); `Revocado{sin_derecho}` si sale y `Revocado{ya_no_existe}` al retirarse |
| `VISTA:f-N` | el mundo visible a la Facción, filtrado por la niebla, **dentro del rectángulo que fija `CambiarVista`** | desde el primer `CambiarVista` válido de un jugador con Facción (no viaja en la `Bienvenida`: el rectángulo es de la sesión) |
| `ADMIN:` | todo el estado, sin niebla | solo sesiones de administración (que no tienen ninguno de los demás); un servidor de batalla no tiene ámbitos |

Los derechos se calculan del estado y se reevalúan cuando cambia el héroe (su sitio, su Facción), una plaza o una Facción: al **perder** uno el servidor manda `Revocado{ambito, motivo}` (`sin_derecho`; `ya_no_existe` si la plaza cayó en ruinas) y el cliente **descarta** las entidades de ese ámbito; al **ganar** uno manda su `Foto`. Primero van los `Revocado` y después las `Foto` nuevas, y todo antes del `ResultadoOrden` de la orden que lo causó. Salir de la plaza o del campamento, que la plaza caiga o el desalojo de una conquista son el mismo caso: el héroe cambia de sitio. Cambiar la Facción pierde a la vez la Facción vieja, la plaza (ya no es ciudadano) y la vista, y gana la nueva.

`CambiarVista` (el único ámbito que pide el cliente): fija el rectángulo (`min` ≤ `max`, en milésimas) y el servidor manda la `Foto` de `VISTA` con lo que cae dentro, otra vez en cada cambio de rectángulo (la foto sustituye a la anterior). Sin Facción, o en una sesión de administración, no hay vista: `Aviso{vista.sin_derecho}`; con un rectángulo incompleto o al revés: `Aviso{vista.invalida}`. Una columna que se toca fuera del rectángulo sale con `QUITA`. No hay mensaje para cerrar la vista.

Qué lleva cada ámbito hoy: `HEROE`, `FACCION` y `BATALLA`, su contenido completo (T3b y T4, abajo); `ASENTAMIENTO` y `CAMPAMENTO`, el interior completo (T3c, abajo); `VISTA`, las columnas de la propia Facción dentro del rectángulo (sin niebla todavía: nunca lo ajeno); `ADMIN`, todas las columnas y todas las plazas (la plaza en ficha). Los eventos solo llegan por `HEROE` (T5 les da audiencia).

**El interior (T3c).** `ASENTAMIENTO` lleva **una** `Asentamiento` con todo lo que la plaza enseña a quien está dentro, más los `Pueblo` de la plaza (el mismo `Pueblo` que en `FACCION`; su almacén local se refresca en cada paso de minuto **solo por `ASENTAMIENTO`**, con la secuencia interior de la plaza):

| Campo | Contenido |
|---|---|
| `version_interior` | **la secuencia propia del interior de la plaza** (abajo): cuántos pasos de minuto lleva dados; el paso de minuto la sube y no sube `version`. La lleva siempre cada cambio parcial (máscara `version`, `version_interior`, ...) y cada `Pueblo` de la plaza |
| `almacen` | los 29 recursos, cada `Existencia` con `cantidad` **en escalones** (`en_escalones = true`: valor del servidor, sin interpolar), `capacidad` y, en `por_minuto`, la **tendencia «+N/min»**: el neto **real** del último paso de minuto (producción − consumo − recaudación − mantenimiento − lo que come la guarnición; no el gasto de las órdenes), guardado por plaza (foto `BAN22`) |
| `poblacion`, `nutricion`, `medidor_mantenimiento`, `racha_mantenimiento_sano` | habitantes **enteros** por clase (`valor_en_t0` múltiplo de 1.000; `por_minuto` = lo que cambió la clase en el último paso), `Poblacion.en_recuperacion` (el hambre: no crece hasta recuperar nutrición 100), nutrición y medidor 0..100 en escalones |
| `edificios` | cada uno con su `estado` (`EN_COLA` con su `prioridad`, `EN_CONSTRUCCION` con `completa_en_ms` **exacto**, `ACTIVO`), `mejora` en curso con su fin exacto, su sitio (`posicion` = centro de la huella en milésimas de unidad local), `ancho`/`alto`, la `fuente_id` que explota, `pausado_por_almacen_lleno` y `danado`. La cola es la lista ordenada por estado y prioridad |
| `recintos`, `trazado` | cada recinto de muralla con su anillo entero (`celdas`), su `avance`, su mejora y la cita de la próxima celda; `TrazadoUrbano` (`layout_version`, `unidades_por_celda`, `radio_ul`, calles y caminos): lo que el cliente necesita para dibujar la ciudad y sus murallas |
| `politicas_activas`, `ascenso`, `reserva_manual`, `recetas_pausadas`, `auto_construccion_pausada` | lo configurable de la plaza, con los instantes exactos de caducidad y de fin del ascenso |
| `puerta_cerrada_a`, `casas_compradas`, `presentes_ids`, `guarnicion_ids`, `cupo_guarnicion`, `caravanas_aparcadas_ids`, `flota_ids`, `batalla_id` | el acceso por relación, quién reside, quién está dentro (residentes y visitantes: su ficha pública llega por `VISTA`), las escuadras en la guarnición y su cupo, las caravanas aparcadas y la flota de la plaza (ids; las entidades están en `FACCION`/`HEROE`), y la batalla que la disputa |

`CAMPAMENTO` lleva **un** `CampamentoMercenarios`: `poblacion` son los reclutas como un `Ritmo` (se recuperan solos hasta el tope de sus viviendas: aquí sí se interpola), `mercado` el stock en venta, `contadores_mercado` y `repone_en_ms`, `residentes_ids` y `presentes_ids`, `prestamos_ids` (las escuadras que prestó, estén donde estén), `tropas_ofrecidas` y `cupo_diario`. Lo que cada héroe lleva comprado hoy y su almacén personal van en su `Heroe` (`HEROE`). La planta del campamento no viaja (el motor aún no la tiene).

**Qué pertenece a `HEROE` y a `FACCION` (T3b).** El núcleo apunta, al cerrar cada versión, a qué ámbitos pertenece cada entidad que cambia y de cuáles salió (`CambioVersion.pertenencias`; la regla es `proyeccion::audiencia`), así que cada `Delta` lleva `ACTUALIZA` de lo que le pertenece y `QUITA` de lo que desapareció o dejó de ser suyo. Todas viajan **completas** (`reemplaza = true`) y una misma entidad puede estar en dos ámbitos (una caravana adjunta a tu columna es de `HEROE` y de `FACCION`; el cliente la descarta cuando ningún ámbito la tiene):

| Entidad | Ámbito | Cuándo |
|---|---|---|
| `Heroe`, `Escuadron` | `HEROE` | solo el héroe de la sesión y sus escuadras (también las prestadas y las cedidas como escolta). Nombre, clase, género y avatar, por defecto (decisión 4: «Héroe N», `default`) |
| `Ejercito` | `HEROE` | las columnas en que participa algún héroe de la cuenta; con `persiguiendo`, `en_batalla`, `batalla_id` y `dentro_de` (`te_sigue` es de la vista del perseguido: T3d) |
| `Caravana` | `HEROE` y `FACCION` | `FACCION`: toda la flota de las plazas de la Facción (también el carro de acarreo de un pueblo); `HEROE`: las adjuntas a una columna en que participa y las que escolta con una escuadra suya |
| `Convocatoria` | `FACCION` | las de la Facción; el Líder ve todas las peticiones y cada solicitante la suya (los demás, ninguna) |
| `Batalla` | `BATALLA` y `FACCION` | `BATALLA`: la de quien combate, entera. `FACCION`: las de las Facciones de los dos bandos y de quienes combaten; la ve entera quien combate en ella y resumida (sin ticket ni token) el resto, con `lado_para_unirse` si podría unirse. Ya no va a `HEROE` |
| `Faccion`, `TecnologiasFaccion`, `RelacionPolitica` | `FACCION` | la propia; las relaciones del andamiaje `FijarRelacion` (`rel-<menor>-<mayor>`, la menor figura como `a`) llegan a las dos Facciones y se quitan con `QUITA` |
| `Asentamiento` | `FACCION` | en resumen (la ficha: Facción, sitio, nivel, zona y los ids de sus pueblos y avanzadas); el interior es de `ASENTAMIENTO` |
| `Pueblo`, `Avanzada`, `ObraDeMapa` | `FACCION` | un pueblo o una avanzada que se está levantando es una `ObraDeMapa` (`obra-N`), no un `Pueblo`/`Avanzada`; al terminar, la obra se quita y llega el pueblo o la avanzada. Una avanzada que se mejora o se repara es las dos cosas |
| `AcuerdoTrueque`, `OrdenMercado` | `FACCION` | un trueque es de las Facciones de sus dos plazas; las órdenes (también las cerradas del historial), de la de su plaza |

Una plaza que cambia de dueño se quita de la Facción vieja y llega a la nueva con todo lo que depende de ella (sus pueblos, avanzadas, órdenes, trueques y flota). La reanudación por versión de `HEROE` y `FACCION` es **siempre por deltas** si el registro alcanza la versión del cliente: el registro dice a qué ámbito pertenecía cada entidad en cada versión, también si el héroe estuvo fuera, y los `QUITA` de lo que desapareció mientras tanto vienen en la delta.


La visibilidad de lo ajeno la decide el servidor ([`Mechanics/Niebla_De_Guerra.md`](../Mechanics/Niebla_De_Guerra.md)): la plaza ve solo su zona, las avanzadas ven 12, una columna ve 30 (15 si va sola), y lo ajeno solo se manda si se ve.

## 4. El reloj sincronizado

El mundo no tiene tick: `Reloj.mundo_ms` es el instante de mundo, `factor_milesimas` el ritmo (1000 = tiempo real) y `pausado`
indica si está parado.

1. En `Bienvenida` y en cada `Pong` el servidor manda su `Reloj`.
2. El cliente manda `Ping{cliente_envio_ms}` al abrir y cada pocos segundos; el `Pong` devuelve el eco.
3. Con `rtt = recepcion - cliente_envio_ms` estima `desfase = reloj.mundo_ms - (cliente_envio_ms + rtt/2)` y lo suaviza con el
   mínimo de las últimas muestras. La hora de mundo local es `reloj_local_monotono + desfase`.
4. Con esa hora **el cliente evalúa los planes y los ritmos en cada fotograma**: `posicion(t)` interpola el `Plan`; una cantidad es
   `valor(t)` del `Ritmo` (salvo los `Ritmo.en_escalones`, que se muestran tal cual: §5). El servidor no manda posiciones ni cantidades por intervalo.
5. Un cambio de factor o una pausa llegan como `Aviso{reloj.cambiado, reloj}`.

## 5. Foto, delta y cambios parciales

- **`Foto`**: estado completo de un ámbito en una versión. Se manda al suscribirse, al reanudar tarde, tras un hueco de secuencia
  y tras un `Revocado` seguido de una nueva suscripción.
- **`Delta{ambito, desde, hasta, cambios, eventos}`**: lo que cambió entre dos versiones. `desde` es la última versión que el
  cliente tiene de ese ámbito: si no coincide con la suya, el cliente pide reanudar.
- **`Cambio`** (forma **híbrida**, decisión del autor del 2026-10-22): `ACTUALIZA` (crea o actualiza) o `QUITA`. Hay dos formas de actualizar y las distingue `reemplaza`:
  - **Entidad completa** (`reemplaza = true`): para todas las entidades menos el asentamiento. `datos` es la entidad entera y **sustituye** a la que el cliente tenía; lo que no
    venga ha vuelto a su valor por defecto. No hay diff por entidad que calcular ni aplicar.
  - **Parcial con máscara** (`reemplaza = false`): solo para el `Asentamiento` (la plaza y su ciudad, que son grandes). `campos` lista los campos de `Asentamiento` que el cambio
    actualiza (`almacen`, `edificios`, `trazado`...); lo que no está en la máscara no cambia. Un campo de la máscara con su valor por defecto **sí** se pone a ese valor (con proto3 un `0` y
    un «sin cambio» no se distinguen, por eso hay máscara) y un `optional` de la máscara que no viene en `datos` se borra. Los repetidos y los mapas de la máscara vienen completos.
  - `reemplaza = false` con `campos` vacío es la semántica de v1 (parcial por presencia, `campos_borrados` para borrar un opcional); el servidor Rust ya no la emite.
- **`Entidad.version`**: cada entidad lleva la versión de mundo en que cambió por última vez; el almacén normalizado del cliente
  descarta cualquier actualización con `version` menor que la que ya tiene.
- Cada `Delta` y cada `Foto` pueden llevar `NieblaProyectada` (dos máscaras: explorado y visible ahora).
- **Qué viaja de un movimiento**: lo propio, el `Plan` completo una sola vez (un cambio de plan llega como plan nuevo que empieza
  donde está la columna: ni parada ni salto); lo ajeno, solo el **tramo** del plan dentro de la vista y con horizonte corto (~30 s),
  renovado. Una columna que anda no cuesta ningún mensaje mientras anda.
- **Economía, obras y colas**: `Ritmo` («cantidad en t₀ + ritmo») y «termina en t». El servidor no manda nada hasta que un ritmo cambia. **La economía de una plaza no es continua**
  (decisión del autor del 2026-10-22, «economía por minuto»): el almacén y la población cambian en escalones, en el paso de minuto de la plaza, y sus `Ritmo` vienen con
  `en_escalones = true`: el cliente **no interpola**, muestra `valor_en_t0` y rotula `por_minuto` como tendencia «+N/min» (el neto real del último paso de minuto, guardado por plaza).
  Los demás ritmos (víveres de una columna, medidor, reputación) siguen siendo rectas que se evalúan en cada fotograma.
- **Población entera**: cada clase tiene habitantes enteros (`valor_en_t0` múltiplo de 1.000); el crecimiento fraccionario entre persona y persona (el «resto oculto») no viaja.

## 6. Órdenes y respuesta causal

```
cliente → Orden { peticion_id, version_esperada?, cuerpo }
servidor → [Delta* con los cambios que produjo]  →  ResultadoOrden { peticion_id, resultado, version_tras, rechazo?, creadas[], datos? }
creadas[] = ReferenciaCreada { entidad: TipoEntidad, id }   (una clase de entidad por orden; p. ej. EDIFICIO `ed-7-12`, RECINTO, OBRA_DE_MAPA, ESCUADRON, EJERCITO, LOADOUT, INCORPORACION `<battle_id>:<n>`)
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
  `guerra.confirmar` y en `detalle` las consecuencias: `{ objetivo: "f-N", facciones: ["f-N"...] }`, la Facción contra la que quedaría en guerra y las que entrarían por arrastre
  (aliados y vasallos; vacía hasta que exista el sistema político). El cliente confirma repitiendo la orden con `declara_guerra: true`.
- **Orden de administración**: `OrdenDeAdministracion{codigo, parametros}` es una sola orden genérica para el andamiaje provisional del núcleo (crear ejércitos, dar recursos,
  fijar relaciones...); solo la acepta una sesión `CLIENTE_ADMIN_WEB` y desaparece cuando los sistemas político y de tecnología sustituyan lo que da (decisión del autor, 2026-10-22).
- **Duplicadas**: `ORDEN_DUPLICADA` devuelve solo la versión (no repite `creadas`): un cliente que reintenta ya recibió el estado nuevo por el delta.
- **Límite de pendientes**: `Bienvenida.maximo_ordenes_pendientes`; pasarse es `ErrorSesion{demasiadas_ordenes}`.

## 7. Reanudación por versión

1. El cliente guarda, por ámbito, la última versión que aplicó y su estado normalizado en caché local.
2. Al reconectar manda `Hola.versiones_conocidas` (clave `"TIPO:id"`) y, de la plaza que tuviera abierta, `Hola.versiones_interiores_conocidas` (la misma clave: el `Asentamiento.version_interior` de la plaza que conserva; ausente = 0).
3. Por cada ámbito al que **tiene derecho ahora** (lo que el cliente diga conocer de otro no le abre nada) el servidor decide y lo anuncia en `Bienvenida.ambitos`: `REANUDA_SIN_NOVEDAD`, `REANUDA_CON_DELTAS` (manda
   el tramo consolidado `k → hoy`) o `REANUDA_CON_FOTO` (versión que el registro de cambios no alcanza o del futuro, o ámbito nuevo). `HEROE`, `FACCION`, `BATALLA`, `ASENTAMIENTO` y `CAMPAMENTO` se reanudan **siempre por deltas** si el registro alcanza la versión: el registro dice qué cambió en cada uno desde entonces, aunque el héroe haya salido y vuelto (desde T3c ya no se pide foto por eso; un cliente que descartó el ámbito con un `Revocado` no vuelve a declarar su versión). En `ASENTAMIENTO` el servidor no sabe qué campos de la plaza tiene el cliente, así que la delta de reanudación lleva la plaza **entera** (`reemplaza = true`); `REANUDA_SIN_NOVEDAD` si la plaza no cambió **y** el cliente trae la secuencia interior de hoy (abajo). Tras la `Bienvenida` llega un mensaje (`Foto` o `Delta`) por cada ámbito que no reanuda sin novedad, en el orden de la lista. `VISTA` no se reanuda: se abre con `CambiarVista`.
4. Para poder mandar deltas, el servidor conserva en memoria las deltas de cada ámbito durante un tiempo de retención y, más allá,
   las reconstruye del registro de eventos; si no puede, foto.
5. Un cliente que detecta un hueco de `secuencia` **cierra la conexión y reconecta** con el mismo procedimiento (nuevo `Hola` con `versiones_conocidas`). No hay reanudación a mitad de sesión: `Hola` solo puede ser el primer mensaje, y un segundo `Hola` es `ErrorSesion{hola_repetido}` (no fatal; se ignora).
6. **El reloj de mundo se congela mientras el servidor está apagado** ([`Mechanics/Reloj_Y_Eventos.md`](../Mechanics/Reloj_Y_Eventos.md)): una
   reapertura tras una caída no trae «tiempo perdido» que simular.

**La secuencia interior de la plaza (4H T3c, decisión del autor 2026-10-23).** Lo que cambia el paso de minuto dentro de una plaza —almacén, población, medidor, nutrición, recuperación, edificios parados, tendencia y almacén local de sus pueblos— **no avanza la versión del mundo ni entra en el registro de cambios del mundo**: avanza una secuencia propia de la plaza, `Asentamiento.version_interior` (y `Pueblo.version_interior`, la de su plaza madre), de uno en uno por cada paso de minuto. Las órdenes y los sucesos discretos (obras, mejoras, políticas, quién entra y sale...) avanzan la versión del mundo como siempre. Efectos:
- **El contador `version` y el hash del mundo son idénticos con sesiones y sin ellas**, y una repetición del registro al recuperar acaba en la misma versión y la misma secuencia interior que el proceso vivo: ningún cliente se ve «del futuro» por esto.
- **Cómo viaja.** Un paso de minuto manda a los clientes de la plaza una `Delta` de `ASENTAMIENTO` con el cambio parcial (`campos` = `version`, `version_interior` y los campos que cambiaron; los `Pueblo` activos de la plaza, completos) y **sin subir de versión**: `hasta` es la del mundo en ese momento (puede ser igual a `desde`). Si nada visible cambió, no se manda nada y el servidor solo avanza la plaza que recuerda del cliente (el `version_interior` que tiene el cliente puede ir por detrás del del servidor sin que falte nada).
- **Cómo se reanuda.** El cliente trae `versiones_conocidas` (versión del mundo del ámbito) y `versiones_interiores_conocidas` (su secuencia interior). Si la secuencia del cliente es **mayor** que la de hoy (un servidor recuperado de un registro más corto: «del futuro»), `REANUDA_CON_FOTO`. Si es **menor**, `REANUDA_CON_DELTAS` con la plaza entera (`reemplaza = true`, más los pueblos), porque no hace falta guardar un registro del interior: la plaza se puede rehacer entera del estado. Si es igual y el registro del mundo no tocó la plaza, `REANUDA_SIN_NOVEDAD`. `Bienvenida.ambitos[].version_interior_actual` dice la de hoy. Un cliente que no la trae (0) recibe la plaza entera, nunca una plaza vieja.
- **No se persiste nada nuevo**: la secuencia es `(ultimo_minuto − fundado_en) / 60 s` de la economía de la plaza, que ya viaja en la foto (`BAN22` sin cambios).
- **`Pueblo` en `FACCION`**: su almacén local ya no se refresca por minuto en `FACCION` (solo en `ASENTAMIENTO`); en `FACCION` llega con el último cambio discreto del pueblo (DUDA, T3c).

La reapertura de Unity pinta el último estado desde la caché local **antes** de que llegue la sesión, y la sesión lo pone al día.

## 8. Eventos de dominio

`EventoDominio` es un **hecho ocurrido** (crónica, avisos, narración), nunca el cambio de estado: ese viaja como entidades en la misma
`Delta`. Llevan `codigo` (`familia.nombre`), `version`, `en_ms`, `asentamiento_id` (quién lo oye) y `heroes_ids` (los nombrados lo oyen
aunque no vean la plaza), y un `payload` estructurado.

- **Dos formas a la vez** (decisión del autor, 2026-10-22): todo evento lleva el `payload` genérico (`Struct`: número, texto o lista) y los **quince** que mueven la interfaz llevan además un
  mensaje tipado en `EventoDominio.tipado`: `BatallaAbierta` (`batalla.abierta`), `BatallaAsignada` (`batalla.asignada`), `BatallaCerrada` (`batalla.aplicada`, `cancelada`, `fallida`),
  `PlazaConquistada` (`asentamiento.conquistada`), `PlazaEnRuinas` (`asentamiento.ruinas`), `AsedioResistido` (`combate.asedio_resistido`), `UnionPedida` (`columna.union_pedida`,
  `convocatoria.union_pedida`), `ConvocatoriaConvocada`, `ConvocatoriaPartio`, `HeroeExpulsado` (`heroe.expulsado`, `heroe.desalojado`), `ColumnaConHambre` (`ejercito.viveres_bajos`,
  `ejercito.sin_viveres`), `DesercionPorHambre` (`tropas.desercion`), `GuerraDeclarada` (`faccion.guerra_declarada`), `ObraTerminada` (edificio, mejora, recinto, pueblo o avanzada
  completados) y `HeroeHerido`. Contra un campamento de bandidos el prefijo es `evento_pve.` y se tipan igual. El resto de los códigos solo tienen `payload`.
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
`MensajeDeServidor.batalla` (`BattleTicket`, `IncorporacionBatalla`, `TokensBatalla`, y el `AcuseBatalla` de cada mensaje suyo) y responde con `MensajeDeCliente.batalla`
(`BattleServerAssignment`, `InicioBatalla`, `SalidaBatalla`, `BattleResult` y `TokensBatalla`, los de quienes se unieron tras la asignación). El acuse dice cuál era el mensaje, su `battle_id` y
`ORDEN_ACEPTADA`, `ORDEN_SIN_CAMBIOS` (repetir uno ya aplicado), `ORDEN_DUPLICADA` o `ORDEN_RECHAZADA` con su `RechazoOrden` (`batalla.ya_asignada`: gana la primera asignación; `batalla.revision_vieja`...).
El ticket se publica a todos los servidores de batalla conectados y gana la primera asignación (decisión del autor, 2026-10-22).
Una batalla sin humanos (bot contra bot) o sin iniciador (`Batalla.iniciada_por` vacío: los bandidos) se asigna igual, con `tokens_participante` vacío. El ciclo (estados, plazos, candado de escuadras,
idempotencia del resultado) es el de [`Mechanics/Batalla_Unity_Ciclo.md`](../Mechanics/Batalla_Unity_Ciclo.md). **El token del propio jugador** (antes un `GET`) viaja en la proyección: `Batalla.asignacion_propia{instancia, token, expira_en_ms}`, solo al
propio combatiente. **Asedio visible desde fuera**: `Asentamiento`, `Pueblo` y `Avanzada` llevan `batalla_id` mientras se juega una batalla contra ellos y `Batalla.lado_para_unirse` dice, por espectador, a qué lado podría
unirse. **Caravanas**: `ContextoCaravana.caravanas` lista todas las que se juegan (con los acosadores entran todas las adjuntas) y los carros se nombran `"<caravana_id>:<posición>"` (también en `BattleResult.carros_perdidos`). El terreno de la batalla se
toma del artefacto de mundo (`BattleMapReference.mapa_id` = hash). **Modalidades nuevas**: asalto de caravana `disputa` / `aguante`,
saqueo de pueblo y ataque a una avanzada ([`Combate.md`](../Mechanics/Combate.md) §7). Unity decide lo de dentro; el mundo sigue mientras se lucha.

**Cómo lo hace el servidor Rust (4H T4, 2026-10-23).**

- **Qué publica y cuándo.** El ticket sale, a **todas** las sesiones de servidor de batalla conectadas, cuando la batalla queda *pendiente de asignar* con una revisión que aún no se mandó: al abrirse (todos los contextos salvo el asedio), al acabar la convocatoria de un asedio (a los 30 s de mundo) y de nuevo, con la revisión subida, tras una salida `no_conectado` anterior al inicio. Las `IncorporacionBatalla` (con su `secuencia`) salen a todos mientras nadie tiene la batalla y solo al servidor de la asignación después. Un servidor que se conecta recibe, tras la `Bienvenida`, el ticket y las incorporaciones vigentes de las batallas que esperan servidor y de las que ya tiene asignadas.
- **Quién gana.** La primera `BattleServerAssignment` que procesa el núcleo; las demás reciben un `AcuseBatalla` con `ORDEN_RECHAZADA` y `batalla.ya_asignada`. Los tokens (`tokens_participante` de la asignación y `TokensBatalla` posteriores) los custodia el núcleo; el mensaje saliente `MensajeParaServidorDeBatalla.tokens` no se usa.
- **Acuses.** Cada mensaje del servidor de batalla recibe su `AcuseBatalla{mensaje, battle_id, resultado, rechazo}` después de lo que ese mensaje haya provocado (p. ej. el ticket nuevo de una salida previa al inicio): `ORDEN_ACEPTADA`, `ORDEN_SIN_CAMBIOS`, `ORDEN_DUPLICADA` (el mismo mensaje otra vez) o `ORDEN_RECHAZADA` con el código (`batalla.invalida`, `batalla.no_existe`, `batalla.ya_asignada`...).
- **Qué ve cada uno.** `BATALLA:<uuid>` lleva la `Batalla` entera a quien combate: `ticket` (el `BattleTicket` de arriba, congelado: la plaza de un asedio con sus edificios y las celdas de muralla en pie, el nivel, la experiencia y la moral de cada escuadra), `incorporaciones`, `resultado` (una vez aplicado), `lado_propio` y `asignacion_propia` (solo el token de **su** héroe). Quien no combate pero es de la Facción de un bando la ve en `FACCION`, resumida: sin ticket ni token y con `lado_para_unirse` si podría unirse. `Asentamiento`, `Pueblo` y `Avanzada` llevan `batalla_id` mientras se juega la batalla contra ellos.
- **Pendiente.** Quien no es de la Facción de ningún bando (un neutral que podría unirse al ataque desde fuera) la verá por la `VISTA` con niebla (T3d); hasta entonces la orden `unirse_a_batalla` le sirve si conoce el id.
- **Informes de inspección.** `InformeInspeccion` (`insp-<héroe>-<objetivo>`) llega a `HEROE` del que inspeccionó, uno por objetivo (`ej-N`, `car-N`, `as-N`), y vale hasta su `expira_en_ms`: el cliente lo descarta al caducar. El de una plaza trae los recintos con alguna celda levantada (`RecintoInforme`: nivel, anillo entero y `avance`).

## 10. Errores de sesión

`ErrorSesion{codigo, mensaje, fatal}`: `token_invalido`, `version_protocolo`, `hueco_de_secuencia` (lo detecta el cliente: cierra y reconecta), `hola_repetido` (no fatal),
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

1. ~~**Payload de los eventos**~~ — **cerrada (2026-10-22)**: `Struct` genérico para todos y quince mensajes tipados para los que mueven la interfaz (§8).
2. **Autenticación y acceso** (registro, claves, roles): fase 3, junto al servidor.
3. **Retención de deltas y tamaño del registro**: se fijan al construir el núcleo con mediciones.
