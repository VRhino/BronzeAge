# Contrato 05: la sesión en tiempo real

> **Estado (2026-10-23): el esquema de 4H (T1) está en `proto/`, las órdenes se traducen (T2), la sesión abre los ámbitos con su autorización, `Revocado` y reanudación por versión (T3a), `HEROE` y `FACCION` llevan su contenido completo, con `QUITA` (T3b), la batalla de Unity va de extremo a extremo por el contrato (T4: ámbito `BATALLA`, ticket congelado, publicación a los servidores de batalla, acuses, token propio, asedio visible e informe de inspección) y `ASENTAMIENTO` y `CAMPAMENTO` llevan el interior de la plaza y del campamento, con cambios parciales con máscara y la tendencia «+N/min» (T3c) y, desde el 2026-10-23, con su propia secuencia interior por plaza (el paso de minuto no sube la versión del mundo, §7); `VISTA` lleva el mundo visible con niebla (T3d: memoria de la Facción, ojos de plazas y avanzadas, visión de aliados, lo ajeno redactado, el tramo ajeno renovado por muestreo, la vista sin Facción y la intel de la taberna) y `ADMIN` todo el mundo sin niebla ni redacción, nunca un token de batalla (T3e); los hechos de dominio llegan con su audiencia, quince tipados y la crónica (T5); la reanudación, la causalidad y el reparto a 500 sesiones están probados y medidos, con la niebla parcial (T6); y la fase se cerró (T7, 2026-10-24) con lo que el núcleo ya sabía y el contrato no llevaba (el suelo bajo cada columna propia, el almacén de la plaza propia a cuya puerta se está, la zona de la Facción fundida, la red de caminos con sus escalones y las rutas propias, el interior que recuerda el héroe, el servidor que tiene cada batalla), los ids reales en los dorados, el catálogo estable de rechazos ([`07_Catalogo_de_Rechazos.md`](07_Catalogo_de_Rechazos.md)), la matriz de autorización comprobada contra el código (Proyecciones_Y_Autorizacion §5.1) y un cliente de ejemplo que juega solo por el contrato (`contratos/README.md`, «Cómo se escribe un cliente»)** (ver `Docs/Arquitectura/4H_Contraste_y_Plan.md`).
> **La política (2026-10-25, tanda 5H del sistema 5):** las propuestas en `FACCION` de las dos Facciones, la Liga en cada Facción, la forma pública de las demás Facciones en `VISTA`, la ciudadanía y las solicitudes en el `Heroe`, los fundadores de la plaza y ocho hechos tipados más (abajo y §8); el alias `FijarAcceso` del núcleo se retiró (ver `Docs/Arquitectura/Sistema5_Contraste_y_Plan.md`, «Avance de 5H»).
> **La tecnología y los Aedas (2026-10-25, tandas 6C-6G del sistema 6):** las cinco órdenes del sistema 6 tienen comando (ninguna orden del contrato da ya `orden.no_disponible`); `TecnologiasFaccion` entera y `AedaResidente` en `FACCION`, la forma pública del estado de tecnología y los `Alijo` a la vista en `HEROE`, los `AedaItinerante` redactados en `VISTA`, todo entero en `ADMIN`, diez hechos tipados más y la crónica entera por páginas (§8) (ver `Docs/Arquitectura/Sistema6_Contraste_y_Plan.md`, «Avance de 6G»).
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
- **Autenticación**: el cliente obtiene un `token` del servicio de acceso (fuera del alcance del contrato) y lo manda en
  `Hola`. Un token inválido o caducado cierra la sesión con `ErrorSesion{token_invalido, fatal}`. Desde la fase 7 (7B, 2026-10-26) el servicio de acceso
  está en el mismo puerto que el WebSocket, por HTTP (`POST /v1/registro`, `POST /v1/sesiones`): ver [`08_Consumo_del_Contrato.md`](08_Consumo_del_Contrato.md) y
  `Docs/Mechanics/Proyecciones_Y_Autorizacion.md` §2.1.
- **El primer mensaje del cliente es `Hola`**; el servidor responde `Bienvenida` (sesión, héroe, Facción, `InfoMundo`, reloj
  y los ámbitos suscritos con la decisión de reanudación). Cualquier otra cosa antes de `Hola` cierra la sesión.
- **Jugador sin héroe**: `Bienvenida.heroe_id` ausente, sin ámbitos y `campamentos_para_elegir` (de cada campamento de mercenarios: id, posición, origen, cuántos lo eligieron como inicial y cuántos héroes viven allí; desde la paridad P6, 2026-10-27, el núcleo guarda `eligieron_como_inicial` y lo manda también en el `CampamentoMercenarios` del ámbito `CAMPAMENTO`). Es lo único que ve hasta que da `crear_heroe`, que le abre su ámbito en la misma sesión. Con héroe, `heroe_id` es su id (`heroe-N`) y `faccion_id`, la de su ciudadanía.
- **Versión de protocolo** (`version_protocolo` = 1). Reglas de evolución en §11.
- **Identificadores**: todas las entidades se nombran `<prefijo>-<n>` (la Facción también: `f-3`; ver `common.proto`). Dentro de una plaza, `ed-<plaza>-<n>` y `recinto-<plaza>-<n>`. La batalla, un UUID.
- **El artefacto de mundo no viaja por la sesión**: el cliente lo descarga por hash desde `InfoMundo.url_artefacto` (Unity, desde
  `url_paquete_unity`) y lo guarda en disco. Si ya tiene el hash vigente (`Hola.mundo_hash_conocido`), no descarga nada. Si el
  hash cambió, el cliente no puede entrar hasta tener el nuevo. En el servidor Rust (fase 7, 2026-10-26) `url_artefacto` es `/v1/mundo/<hash>`, relativa al
  propio servidor, que lo sirve por HTTP en el mismo puerto; `url_paquete_unity` sigue vacía hasta la fase 8.
- **El archivo común de datos de juego** (las cifras de reglas que ve un jugador) tampoco viaja por la sesión: `InfoMundo.version_datos_de_juego` dice su versión y
  `InfoMundo.url_datos_de_juego` (campo 8, fase 7, 2026-10-26) dónde bajarlo. En el servidor Rust es `/v1/datos/<versión>`, relativa al servidor: un JSON con cada bloque
  `<!-- parametros:NOMBRE -->` de `Docs/Mechanics` por documento, las reglas de la partida que no salen de un bloque y la huella de las reglas
  ([`08_Consumo_del_Contrato.md`](08_Consumo_del_Contrato.md)).

## 3. Ámbitos y qué recibe cada cliente

El servidor suscribe el cliente a los ámbitos que le corresponden; **el cliente nunca los elige** (salvo el interés del mapa). Cada ámbito se nombra `TIPO:id` con los ids de §2 (esa misma cadena es la clave de `Hola.versiones_conocidas`):

| Ámbito | Contenido | Cuándo (autorización) |
|---|---|---|
| `HEROE:heroe-N` | el héroe propio (con sus loadouts), sus escuadras, las columnas en que participa, las caravanas que lleva en su columna o que escolta con una escuadra suya y sus informes de inspección vigentes (inventario y equipo, vacíos hasta el catálogo de Conquest); desde 6G, la **forma pública del estado de tecnología del mundo** (`EstadoTecnologia`, id `tecnologia`, la misma para todos) y los **alijos que tiene a la vista** (`Alijo`, si puede buscarlos) | siempre que la cuenta tenga héroe (el de verdad; si solo tiene héroes de campo del andamiaje de administración, el primero) |
| `FACCION:f-N` | la Facción propia con sus ciudadanos, sus relaciones y sus tecnologías, sus plazas en resumen, su flota de caravanas, sus pueblos y avanzadas (y las obras de mapa que aún no lo son), sus trueques, sus órdenes de mercado, sus convocatorias, las batallas de los dos bandos en que está su Facción, resumidas si no combate, las **propuestas** pendientes en que está (5H; la memoria de la niebla va en `VISTA`, T3d) y, desde 6G, sus **Aedas residentes** con su épica | si el héroe tiene Facción |
| `ASENTAMIENTO:as-N` | la plaza completa vista desde dentro y los pueblos de la plaza (con su almacén local): almacén y población en escalones con su tendencia, edificios y cola, recintos, trazado, políticas, ascenso, guarnición, quiénes están dentro... (abajo) | mientras el héroe está dentro **y** es ciudadano de la Facción dueña (dentro de una plaza ajena no se ve ningún interior) |
| `CAMPAMENTO:camp-N` | el campamento de mercenarios completo: reclutas, stock en venta, residentes y presentes, préstamos, tropas que ofrece y cupo diario | mientras está dentro |
| `BATALLA:<uuid>` | la batalla en la que combate, entera: ticket congelado, incorporaciones, resultado, su bando y su token (`asignacion_propia`) | mientras combate (el héroe de la sesión o el de campo de su cuenta) y el núcleo conserve la batalla (así ve el resultado); `Revocado{sin_derecho}` si sale y `Revocado{ya_no_existe}` al retirarse |
| `VISTA:f-N` (`VISTA:heroe-N` sin Facción) | el mundo visible a la Facción —o al héroe sin bandera, con su niebla personal—, filtrado por la niebla, **dentro del rectángulo que fija `CambiarVista`** (abajo, «La vista») | desde el primer `CambiarVista` válido de un jugador con héroe (no viaja en la `Bienvenida`: el rectángulo es de la sesión) |
| `ADMIN:` | todo el estado, sin niebla ni redacción (abajo, «La administración»), **nunca** un token de batalla | solo sesiones `CLIENTE_ADMIN_WEB` (que no tienen ninguno de los demás: lo que un jugador, un bot o un servidor de batalla diga conocer de `ADMIN:` se ignora); un servidor de batalla no tiene ámbitos |

Los derechos se calculan del estado y se reevalúan cuando cambia el héroe (su sitio, su Facción), una plaza o una Facción: al **perder** uno el servidor manda `Revocado{ambito, motivo}` (`sin_derecho`; `ya_no_existe` si la plaza cayó en ruinas) y el cliente **descarta** las entidades de ese ámbito; al **ganar** uno manda su `Foto`. Primero van los `Revocado` y después las `Foto` nuevas, y todo antes del `ResultadoOrden` de la orden que lo causó. Salir de la plaza o del campamento, que la plaza caiga o el desalojo de una conquista son el mismo caso: el héroe cambia de sitio. Cambiar la Facción pierde a la vez la Facción vieja, la plaza (ya no es ciudadano) y la vista, y gana la nueva.

`CambiarVista` (el único ámbito que pide el cliente): fija el rectángulo (`min` ≤ `max`, en milésimas) y el servidor manda la `Foto` de `VISTA` con lo que cae dentro, otra vez en cada cambio de rectángulo (la foto sustituye a la anterior). Sin héroe, o en una sesión de administración, no hay vista: `Aviso{vista.sin_derecho}`; con un rectángulo incompleto o al revés: `Aviso{vista.invalida}`. Lo que sale del rectángulo o de la vista sale con `QUITA`. No hay mensaje para cerrar la vista.

Qué lleva cada ámbito hoy: `HEROE`, `FACCION` y `BATALLA`, su contenido completo (T3b y T4, abajo); `ASENTAMIENTO` y `CAMPAMENTO`, el interior completo (T3c, abajo); `VISTA`, el mundo visible con niebla (T3d, abajo); `ADMIN`, todo (T3e, abajo). Los eventos llegan a cada sesión una vez, por el primero de sus ámbitos que los oye (`ASENTAMIENTO`, `BATALLA`, `HEROE`, `FACCION`, `VISTA`), y todos por `ADMIN` (T5, §8).

**La administración (T3e).** `ADMIN` lleva todo el mundo tal como es, con las mismas entidades que los ámbitos de jugador y sin redactar nada: cada `Asentamiento` **con su interior** (como en `ASENTAMIENTO`: almacén y población en escalones, edificios, recintos, trazado, quiénes están dentro...; con su secuencia interior, cambio parcial con máscara en cada paso de minuto), todas las `Ejercito` enteras (composición, Líder, suministro, víveres y el plan hasta el destino), todos los `Heroe` enteros con sus `Escuadron`, sus loadouts y sus `InformeInspeccion`, todas las `Faccion` con sus `TecnologiasFaccion` y sus `RelacionPolitica`, toda la flota de `Caravana`, los `Pueblo`, `Avanzada` y `ObraDeMapa` (también las avanzadas huérfanas), los `AcuerdoTrueque`, las `OrdenMercado` con su historial, las `Convocatoria` con **todas** sus peticiones, todos los `CampamentoMercenarios` y todos los `CampamentoBandidos`, y las `Batalla` que conserva el núcleo **enteras** (ticket, incorporaciones, resultado) más la asignación que ganó en `Batalla.asignacion` (intento, revisión, instancia y, desde T7, `servidor_id`: el id de la credencial del servidor de batalla que la tiene) **con `tokens_participante` vacío**: ningún token de batalla viaja a la administración (ni el de la asignación, ni los de las incorporaciones, ni `asignacion_propia`). No lleva lo que es de la perspectiva de un jugador (`NieblaProyectada`, `FichaConocida`, `ZonaFaccion`, `RedCaminos`, `lado_propio`, `lado_para_unirse`) ni, de momento, la memoria de cada Facción, las Miradas y los Informes de plaza, ni la red de caminos (paridad «Contrato (4H)»: no son clases del registro de cambios del que sale `ADMIN`). Sale del registro de cambios: cada `Delta` lleva `ACTUALIZA` de todo lo que la versión tocó (`QUITA` de lo que ya no existe), las caravanas que lleva una columna tocada y todos los hechos de la versión; no necesita reconciliar por sellos como `VISTA` porque nada entra ni sale de `ADMIN` sin que cambie el estado. Se reanuda por deltas como `HEROE`: la secuencia interior de cada plaza va en `Hola.versiones_interiores_conocidas` con la clave de su ámbito (`ASENTAMIENTO:as-N`), y la plaza cuya secuencia no es la de hoy (o que no se declara) llega entera. No se revoca nunca y no tiene vista (`CambiarVista` → `Aviso{vista.sin_derecho}`). Desde 6G lleva también el **estado de tecnología entero** (`EstadoTecnologia` con los contadores en curso, los logros y las descubridoras por tecnología), los `AedaResidente` con su épica y la memoria de hechos contados (`claves`) y los `AedaItinerante` enteros (plaza, estancia, destino, plan entero y `recientes`); los contadores (los suma el paso de minuto) y los itinerantes (su viaje no sube la versión) no pasan por el registro: `ADMIN` los compara en cada reparto y manda los que cambiaron, y al reanudar por deltas van enteros en la misma `Delta` (sin novedad, se dan por sabidos). Las órdenes de administración (`OrdenDeAdministracion`, `controlar_reloj`) siguen siendo solo de esta clase de cliente (`no_autorizado` en cualquier otra). **Hay un solo rol de administración hasta el sistema 5** (decisión del autor, 2026-10-24): toda sesión `CLIENTE_ADMIN_WEB` ve y ordena lo mismo (Proyecciones_Y_Autorizacion §2.2).

**La vista (T3d).** `VISTA` es lo que ven los ojos de la Facción —sus plazas (su disco), sus avanzadas operativas (12), sus columnas (30, o 15 sin soldados) y sus Miradas abiertas— y los de sus aliados (alianza o vasallaje, en cualquier dirección), con lo que recuerda, dentro del rectángulo ([`Mechanics/Niebla_De_Guerra.md`](../Mechanics/Niebla_De_Guerra.md) §3-§6). Sin Facción, `VISTA:heroe-N`: los ojos son sus columnas y la memoria, la suya.

| Qué | Cuándo | Cómo viaja |
|---|---|---|
| `NieblaProyectada` | siempre | en la `Foto` y en cada `Delta` en que cambia: `celdas` = lo explorado (la memoria de la Facción más lo que cada columna propia lleva andado) y `visibles` = lo que se ve ahora (con lo de los aliados y las Miradas, que no exploran). Celda de 5 u: 534 × 400 en el mapa real, 26.700 bytes por máscara |
| `Ejercito` | lo propio (de la Facción o en que va la cuenta), siempre; lo ajeno, si se ve y no lo aparta una batalla | lo propio entero, como en `HEROE`; lo propio lleva además `territorio_faccion_id` (T7, `territorioPorEjercito`): la Facción dueña del suelo que pisa ahora, de cualquier Facción y aunque su plaza no se vea, o vacío en tierra de nadie (vuelve a viajar al cruzar una frontera); lo ajeno redactado: `faccion_id`, `tipo`, `participantes` (solo quiénes), `formacion_expira_en_ms`, `estado`, `posicion` y en `plan` **solo el tramo** al alcance de los ojos que la ven (no el rectángulo; 2026-10-23) y dentro de 30 s, sin el destino; `te_sigue` si persigue a una columna propia. El tramo se renueva solo (el muestreo de 250 ms manda otro cuando al cliente le quedan menos de 10 s) |
| `Caravana` | las de la Facción (por su plaza de origen o su destino) y las adjuntas a una columna propia; las ajenas que se ven o que llaman la atención (grande, a 60 de un ojo) | las propias enteras; las ajenas con `faccion_id`, `posicion`, su tramo, `escoltada` y `recursos_avistados` (nombres, sin cantidades: `contenido` vacío) |
| `Asentamiento`, `Pueblo`, `Avanzada`, `ObraDeMapa` | lo propio, siempre; lo ajeno, si se ve | lo propio como en `FACCION` y, si la columna del héroe de la sesión está a la puerta de una plaza propia, esa plaza con su `almacen` (T7: la parte del almacén de `detalleDeAvistados`, que se renueva con cada paso de minuto mientras siga a la puerta); la plaza ajena como ficha con su `zona` real recortada y sus edificios activos (sin almacén, población, obras, cola, pueblos, avanzadas, trazado); el pueblo y la avanzada ajenos sin almacén, carro, guarnición ni tropas elegidas; la obra ajena sin lo pedido ni lo entregado |
| `FichaConocida` (id del sitio, clase `FICHA_CONOCIDA`) | un sitio ajeno (plaza, pueblo o avanzada: `tipo`) que se vio y no se ve ahora | la última foto, con `conocido_en_ms` y el `estado` del sitio tal como se vio (activo, en obra, en ruinas; también unas ruinas sin dueño, con `faccion_id` vacío; decisión del autor, 2026-10-24); nunca a la vez que el sitio visible; se olvida si se vuelve a mirar su sitio y ya no está |
| `Heroe` | los que van en una columna de la vista y los que están en la plaza donde está el héroe (no el suyo) | solo lo público: nombre, clase, avatar, nivel, `herido_hasta_ms` y `escuadras_que_lleva` (tropa, cantidad, nivel) |
| `Batalla` | las activas cuyo punto se ve o en que está su Facción | como la ve el héroe de la sesión: entera si combate; si no, resumida con `lado_para_unirse` (así la ve también quien no es de ningún bando) |
| `CampamentoBandidos` | los que se ven ahora | tal cual (sin memoria) |
| `CampamentoMercenarios` | los explorados y donde el héroe reside o está | como en `CAMPAMENTO` |
| `ZonaFaccion` (id `f-N`) | con Facción | **fundida** (T7): el contorno de la unión de las zonas recortadas de sus plazas (las de la misma Facción se solapan), un lazo por cada grupo de plazas que se tocan más uno por cada hueco que rodea sin reclamar; los exteriores van en sentido antihorario y los huecos al revés (pintados juntos con la regla `nonzero` el hueco queda fuera) |
| `RedCaminos` (id `red`) | las aristas con algún extremo explorado y alguno en el rectángulo | ids `x1,y1:x2,y2` (unidades), `escalones` (T7: el de cada arista en el mismo orden, 0 sendero, 1 camino, 2 calzada, de las rutas vigentes que la pisan) y `rutas` (las vigentes **de la propia Facción**, `ruta-<origen>-<destino>`, con sus aristas y su último lanzamiento; las ajenas no viajan: DUDA de T7) |
| `MiradaIntel`, `InformePlaza` (id `as-N`) | las Miradas de la Facción mientras enfrían su zona; sus Informes de plaza | tal cual (con sus fechas) |
| `RelacionPolitica`, `Titulo` | siempre (públicos: sin niebla ni rectángulo) | las relaciones activas (5D) y los títulos de prestigio (5F) |
| `AedaItinerante` (6G, id `aeda-N`, clase `AEDA`) | los que se ven ahora (`seVeAhora` de su posición; ninguna memoria) | redactado como una columna ajena: `posicion`, en `plan` solo el tramo al alcance de los ojos y dentro de 30 s (renovado por el muestreo) si va de camino y `en_asentamiento_id` si está parado; nunca `hasta_ms`, `destino_id` ni `recientes`. No es una clase del registro de cambios (su viaje no sube la versión): su sello sale de su plan o de su estancia |
| `Faccion` (5H) | las demás Facciones que nombra lo que se ve: sus columnas, caravanas, plazas, pueblos, avanzadas, fichas y batallas, y las relaciones y los títulos (la propia va entera en `FACCION`) | su **forma pública**: nombre, sigilo, nivel, reputación, Rey y Embajador, ciudadanos, solicitudes, derrota, `admite_otras_en_ataques` y `liga`; sin experiencia, `capital_id` ni `capital_designada_en_ms` |

**Cómo se reparte.** La vista no sale del registro de cambios: lo que se ve cambia sin que cambie el mundo. El servidor recuerda qué tiene el cliente de cada entidad (un *sello*: su versión y, de lo ajeno que anda, dónde acaba el tramo que se le mandó) y, en un muestreo cada 250 ms de mundo y tras los repartos (tras una **orden**, al momento solo a quien la dio, y a los demás en el muestreo siguiente; tras un cambio que no viene de una orden, o con el reloj parado, a todos al momento; decisión del autor del 2026-10-24: la vista ajena puede llegar hasta 250 ms de mundo después que la causal), le manda una `Delta` con lo que entra o cambió (`ACTUALIZA`, completo), lo que dejó de verse (`QUITA`) y la niebla si cambió. Una `Delta` solo de la vista va de la versión que tiene el cliente a la del mundo de ese momento (puede ser la misma). El muestreo no toca el estado. La memoria la graba el núcleo al cerrar sus versiones, no el muestreo: es la misma con sesiones y sin ellas.

**El interior (T3c).** `ASENTAMIENTO` lleva **una** `Asentamiento` con todo lo que la plaza enseña a quien está dentro, más los `Pueblo` de la plaza (el mismo `Pueblo` que en `FACCION`; su almacén local se refresca en cada paso de minuto **solo por `ASENTAMIENTO`**, con la secuencia interior de la plaza):

| Campo | Contenido |
|---|---|
| `version_interior` | **la secuencia propia del interior de la plaza** (abajo): cuántos pasos de minuto lleva dados; el paso de minuto la sube y no sube `version`. La lleva siempre cada cambio parcial (máscara `version`, `version_interior`, ...) y cada `Pueblo` de la plaza |
| `almacen` | los 29 recursos, cada `Existencia` con `cantidad` **en escalones** (`en_escalones = true`: valor del servidor, sin interpolar), `capacidad` y, en `por_minuto`, la **tendencia «+N/min»**: el neto **real** del último paso de minuto (producción − consumo − recaudación − mantenimiento − lo que come la guarnición; no el gasto de las órdenes), guardado por plaza (desde la foto `BAN22`) |
| `poblacion`, `nutricion`, `medidor_mantenimiento`, `racha_mantenimiento_sano` | habitantes **enteros** por clase (`valor_en_t0` múltiplo de 1.000; `por_minuto` = lo que cambió la clase en el último paso), `Poblacion.en_recuperacion` (el hambre: no crece hasta recuperar nutrición 100), nutrición y medidor 0..100 en escalones |
| `edificios` | cada uno con su `estado` (`EN_COLA` con su `prioridad`, `EN_CONSTRUCCION` con `completa_en_ms` **exacto**, `ACTIVO`), `mejora` en curso con su fin exacto, su sitio (`posicion` = centro de la huella en milésimas de unidad local), `ancho`/`alto`, la `fuente_id` que explota, `pausado_por_almacen_lleno` y `danado`. La cola es la lista ordenada por estado y prioridad |
| `recintos`, `trazado` | cada recinto de muralla con su anillo entero (`celdas`), su `avance`, su mejora y la cita de la próxima celda; `TrazadoUrbano` (`layout_version`, `unidades_por_celda`, `radio_ul`, calles y caminos): lo que el cliente necesita para dibujar la ciudad y sus murallas |
| `politicas_activas`, `ascenso`, `reserva_manual`, `recetas_pausadas`, `auto_construccion_pausada` | lo configurable de la plaza, con los instantes exactos de caducidad y de fin del ascenso |
| `puerta_cerrada_a`, `casas_compradas`, `presentes_ids`, `guarnicion_ids`, `cupo_guarnicion`, `caravanas_aparcadas_ids`, `flota_ids`, `batalla_id` | el acceso por relación, quién reside, quién está dentro (residentes y visitantes: su ficha pública llega por `VISTA`), las escuadras en la guarnición y su cupo, las caravanas aparcadas y la flota de la plaza (ids; las entidades están en `FACCION`/`HEROE`), y la batalla que la disputa |
| `cargos`, `vetados_ids`, `fundadores_ids`, `permite_reabastecer_aliados`, `ultima_caravana_creada_en_ms` | los titulares de los cinco cargos locales y los vetados de la puerta (5E); quienes la fundaron y siguen residiendo (5H, 5B-9: también en la ficha propia de `FACCION`, no en la ajena de la `VISTA`); si abre su almacén a los aliados y cuándo creó su última caravana (5H) |

`CAMPAMENTO` lleva **un** `CampamentoMercenarios`: `poblacion` son los reclutas como un `Ritmo` (se recuperan solos hasta el tope de sus viviendas: aquí sí se interpola), `mercado` el stock en venta, `contadores_mercado` y `repone_en_ms`, `residentes_ids` y `presentes_ids`, `prestamos_ids` (las escuadras que prestó, estén donde estén), `tropas_ofrecidas` y `cupo_diario`. Lo que cada héroe lleva comprado hoy y su almacén personal van en su `Heroe` (`HEROE`). La planta del campamento no viaja (el motor aún no la tiene).

**Qué pertenece a `HEROE` y a `FACCION` (T3b).** El núcleo apunta, al cerrar cada versión, a qué ámbitos pertenece cada entidad que cambia y de cuáles salió (`CambioVersion.pertenencias`; la regla es `proyeccion::audiencia`), así que cada `Delta` lleva `ACTUALIZA` de lo que le pertenece y `QUITA` de lo que desapareció o dejó de ser suyo. Todas viajan **completas** (`reemplaza = true`) y una misma entidad puede estar en dos ámbitos (una caravana adjunta a tu columna es de `HEROE` y de `FACCION`; el cliente la descarta cuando ningún ámbito la tiene):

| Entidad | Ámbito | Cuándo |
|---|---|---|
| `Heroe`, `Escuadron` | `HEROE` | solo el héroe de la sesión y sus escuadras (también las prestadas y las cedidas como escolta). Nombre, clase, género y avatar, por defecto (decisión 4: «Héroe N», `default`). Desde 5H, `faccion_id` (su ciudadanía) y `solicitudes_faccion_ids` (las Facciones a las que pidió entrar). `plazas_recordadas` (T7): de cada plaza de la que salió al mundo, la foto de su interior tal como la dejó (almacén, cola y guarnición, con `visto_en_ms`; Niebla_De_Guerra §4.5) |
| `Ejercito` | `HEROE` | las columnas en que participa algún héroe de la cuenta; con `persiguiendo`, `en_batalla`, `batalla_id` y `dentro_de` (`te_sigue` es de la vista del perseguido: va en `VISTA`). Con plan, `posicion` es la de su primer vértice (la proyección no depende del instante en que se pide) |
| `Caravana` | `HEROE` y `FACCION` | `FACCION`: toda la flota de las plazas de la Facción (también el carro de acarreo de un pueblo); `HEROE`: las adjuntas a una columna en que participa y las que escolta con una escuadra suya |
| `Convocatoria` | `FACCION` | las de la Facción; el Líder ve todas las peticiones y cada solicitante la suya (los demás, ninguna) |
| `Batalla` | `BATALLA` y `FACCION` | `BATALLA`: la de quien combate, entera. `FACCION`: las de las Facciones de los dos bandos y de quienes combaten; la ve entera quien combate en ella y resumida (sin ticket ni token) el resto, con `lado_para_unirse` si podría unirse. Ya no va a `HEROE` |
| `Faccion`, `TecnologiasFaccion`, `RelacionPolitica` | `FACCION` | la propia, con su `liga` (5H) y sus tecnologías enteras (6G: `aparecidas`, `adoptadas`, `reveladas`; de cada revelada su descubridora y qué condiciones de su hito cumple ya, `detalle_reveladas`; y las `epicas_posibles`); las de otra Facción no viajan nunca; las relaciones activas (`rel-N`, con dirección: en un vasallaje `a` es la señora) llegan a las dos Facciones y se quitan con `QUITA` al acabar |
| `AedaResidente` (6G, id `aeda-rN`, clase `AEDA`) | `FACCION` | los de las plazas de la Facción (los rivales no ven los suyos), con su épica: `capitulo`, `hechos`, `ultimo_hecho_en_ms` (el enfriamiento) y, desde 6G, `capitulos` y `hechos_necesarios`; `claves` va vacío (solo `ADMIN`). Un residente que se va o cae con su plaza sale con `QUITA` |
| `EstadoTecnologia` (6G, id `tecnologia`, clase `ESTADO_TECNOLOGIA`) | `HEROE` | de todos (es pública): la Era y `era_desde_ms`, `logros_cumplidos` (`contador` llegó a `umbral` en `en_ms`, por orden, sin decir qué tecnología abren), `cantadas`, de `primeros` solo las cantadas, y los dos bordes de la Era vigente (`era_minimo_hasta_ms`: desde cuándo puede pasar si cumple todos sus logros, la decisión del autor sobre 6B; `era_plazo_hasta_ms`: cuándo pasa por plazo; ausentes en la última); nunca `contadores` ni `logros` por tecnología (decisión Q6 del autor). Viaja cuando cambia (un logro, una Era, un canto) |
| `Alijo` (6G) | `HEROE` | los que el héroe tiene a la vista (Niebla_De_Guerra §5.4: sin Facción o de una sin plazas, su columna en el mapa a `VISION.jugadorSolo`, sin abrir), con su sitio y su oro (milésimas, el del artefacto); se renuevan con el muestreo de la vista y tras cada reparto, como el tramo ajeno (`ACTUALIZA` al entrar, `QUITA` al salir o al abrirlo). Al reanudar por deltas van en la misma `Delta` |
| `Propuesta` (5H) | `FACCION` | las de relación, anexión y fusión en que está la Facción (las dos de su par), con `propuesta_por` = el héroe que la hizo; viajan mientras están en el almacén, también caducadas (el cliente las descarta por `expira_en_ms`), y salen con `QUITA` al contestarlas, retirarlas, podarlas o descartarlas un traslado |
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
- **Niebla parcial (4H T6)**: en una `Delta`, a quien ya tiene la niebla anterior de ese ámbito, la niebla viaja como cambio parcial: `parcial = true`, `celdas` y `visibles` vacíos y, en
  `tramos_celdas` y `tramos_visibles`, los tramos de bytes que cambiaron de cada máscara (`TramoDeNiebla{desde, datos}`: a partir del byte `desde`, `datos` sustituye a los que había). El cliente
  sobrescribe esos bytes en las dos máscaras que tiene. La `Foto` la trae siempre entera, y también la `Delta` si cambian las dimensiones o los tramos no ahorran nada. Lo visible cambia en casi cada
  muestreo de 250 ms (los ojos andan) pero solo en el borde de sus discos: con 450 jugadores en el mapa la vista pasó de 12 MB por minuto y sesión en crudo (113 KB con zstd) a 26 KB (23 KB).
- **Qué viaja de un movimiento**: lo propio, el `Plan` completo una sola vez (un cambio de plan llega como plan nuevo que empieza
  donde está la columna: ni parada ni salto); lo ajeno, solo el **tramo** del plan dentro de la vista y con horizonte corto (~30 s),
  renovado. Una columna que anda no cuesta ningún mensaje mientras anda.
- **Economía, obras y colas**: `Ritmo` («cantidad en t₀ + ritmo») y «termina en t». El servidor no manda nada hasta que un ritmo cambia. **La economía de una plaza no es continua**
  (decisión del autor del 2026-10-22, «economía por minuto»): el almacén y la población cambian en escalones, en el paso de minuto de la plaza, y sus `Ritmo` vienen con
  `en_escalones = true`: el cliente **no interpola**, muestra `valor_en_t0` y rotula `por_minuto` como tendencia «+N/min» (el neto real del último paso de minuto, guardado por plaza).
  Los demás ritmos (víveres de una columna, medidor, reputación) siguen siendo rectas que se evalúan en cada fotograma.
- **Población entera**: cada clase tiene habitantes enteros (`valor_en_t0` múltiplo de 1.000); el crecimiento fraccionario entre persona y persona (el «resto oculto») no viaja.

**Implementación de referencia del cliente (4H T7).** Lo que hace un cliente con lo que le llega está escrito una vez, en el crate `contrato` y sin depender del núcleo: `contrato::plaza::aplicar` (el cambio parcial de la plaza con su máscara; el servidor la calcula con `contrato::plaza::mascara`), `contrato::niebla::aplicar_niebla` (la niebla entera o parcial) y `contrato::almacen::Almacen` (los ámbitos de un cliente: `Foto`, `Delta`, `Revocado`, los hechos y el `Hola` de quien reanuda). El cliente de ejemplo que juega solo por el contrato (`servidor::prueba::ClienteDeEjemplo`, `crates/nucleo/tests/cliente_de_ejemplo.rs`) los usa, y `contratos/README.md` («Cómo se escribe un cliente») es la guía para Unity y el admin web.

## 6. Órdenes y respuesta causal

```
cliente → Orden { peticion_id, version_esperada?, cuerpo }
servidor → [Delta* con los cambios que produjo]  →  ResultadoOrden { peticion_id, resultado, version_tras, rechazo?, creadas[], datos? }
creadas[] = ReferenciaCreada { entidad: TipoEntidad, id }   (una clase de entidad por orden; p. ej. EDIFICIO `ed-7-12`, RECINTO, OBRA_DE_MAPA, ESCUADRON, EJERCITO, LOADOUT, INCORPORACION `<battle_id>:<n>`)
```

- **Causalidad**: el servidor envía `ResultadoOrden` **después** de toda `Delta` con `hasta ≤ version_tras` que afecte a los
  ámbitos del cliente. Al recibir `ORDEN_ACEPTADA`, el cliente ya tiene el estado nuevo: **no vuelve a pedir nada**.
- **Resultados** (los códigos de rechazo, catálogo estable en [`07_Catalogo_de_Rechazos.md`](07_Catalogo_de_Rechazos.md), 4H T7): `ORDEN_ACEPTADA`, `ORDEN_RECHAZADA` (con `RechazoOrden{codigo, mensaje, detalle}`), `ORDEN_SIN_CAMBIOS`
  (válida pero no cambió nada; no sube la versión) y `ORDEN_DUPLICADA` (misma `peticion_id`).
- **Idempotencia**: el servidor recuerda las últimas `peticion_id` de la sesión (y las de la partida para reconexiones). Reenviar una
  orden tras reconectar no la ejecuta dos veces. En el servidor Rust las recuerda el estado del mundo (va en la foto): las últimas 500 órdenes aceptadas de toda la partida, por cuenta y
  `peticion_id` (`Reglas::limite_idempotencia`); con 500 jugadores a una orden cada 10 s eso son ≈ 10 s (DUDA, 4H T6).
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
2. Al reconectar manda `Hola.versiones_conocidas` (clave `"TIPO:id"`) y, de la plaza que tuviera abierta, `Hola.versiones_interiores_conocidas` (la misma clave: el `Asentamiento.version_interior` de la plaza que conserva; ausente = 0). La administración da en ese mapa la secuencia de **cada** plaza que conserva, con la clave `ASENTAMIENTO:as-N` de cada una (T3e).
3. Por cada ámbito al que **tiene derecho ahora** (lo que el cliente diga conocer de otro no le abre nada) el servidor decide y lo anuncia en `Bienvenida.ambitos`: `REANUDA_SIN_NOVEDAD`, `REANUDA_CON_DELTAS` (manda
   el tramo consolidado `k → hoy`) o `REANUDA_CON_FOTO` (versión que el registro de cambios no alcanza o del futuro, o ámbito nuevo). `HEROE`, `FACCION`, `BATALLA`, `ASENTAMIENTO`, `CAMPAMENTO` y `ADMIN` se reanudan **siempre por deltas** si el registro alcanza la versión: el registro dice qué cambió en cada uno desde entonces, aunque el héroe haya salido y vuelto (desde T3c ya no se pide foto por eso; un cliente que descartó el ámbito con un `Revocado` no vuelve a declarar su versión). En `ASENTAMIENTO` el servidor no sabe qué campos de la plaza tiene el cliente, así que la delta de reanudación lleva la plaza **entera** (`reemplaza = true`); `REANUDA_SIN_NOVEDAD` si la plaza no cambió **y** el cliente trae la secuencia interior de hoy (abajo). Tras la `Bienvenida` llega un mensaje (`Foto` o `Delta`) por cada ámbito que no reanuda sin novedad, en el orden de la lista. `VISTA` no se reanuda: se abre con `CambiarVista`.
4. Para poder mandar deltas, el servidor conserva en memoria las deltas de cada ámbito durante un tiempo de retención y, más allá,
   las reconstruye del registro de eventos; si no puede, foto. **El servidor Rust (4H T6)** conserva las últimas **65.536 versiones** del registro de cambios del mundo
   (`Ajustes::retencion_cambios`): quien conserva una versión que el registro aún alcanza —la más vieja retenida incluida— reanuda por deltas; una anterior, foto. No las reconstruye del
   registro de eventos, pero tras un reinicio el núcleo persistente repite su registro desde la última foto llevando a qué ámbito pertenece cada entidad (como el proceso vivo), así que una
   versión posterior a esa foto también reanuda por deltas. Medido: 65.536 versiones son ≈ 7 h de mundo sin órdenes (450 columnas que marchan y 30 plazas: ≈ 160 versiones por minuto) y
   ≈ 22 min con 500 jugadores que dan una orden cada 10 s (cada orden aceptada es una versión).
   **Un hecho no se cuenta dos veces**: al reanudar solo llegan los hechos de versiones posteriores a la **mayor** que el cliente declara en `versiones_conocidas` (un ámbito al día hasta `k`
   significa que estaba conectado cuando se repartió `k`, y entonces recibió sus hechos por el ámbito que los oía, aunque ya no lo tenga).
5. Un cliente que detecta un hueco de `secuencia` **cierra la conexión y reconecta** con el mismo procedimiento (nuevo `Hola` con `versiones_conocidas`). No hay reanudación a mitad de sesión: `Hola` solo puede ser el primer mensaje, y un segundo `Hola` es `ErrorSesion{hola_repetido}` (no fatal; se ignora).
6. **El reloj de mundo se congela mientras el servidor está apagado** ([`Mechanics/Reloj_Y_Eventos.md`](../Mechanics/Reloj_Y_Eventos.md)): una
   reapertura tras una caída no trae «tiempo perdido» que simular.

**La secuencia interior de la plaza (4H T3c, decisión del autor 2026-10-23).** Lo que cambia el paso de minuto dentro de una plaza —almacén, población, medidor, nutrición, recuperación, edificios parados, tendencia y almacén local de sus pueblos— **no avanza la versión del mundo ni entra en el registro de cambios del mundo**: avanza una secuencia propia de la plaza, `Asentamiento.version_interior` (y `Pueblo.version_interior`, la de su plaza madre), de uno en uno por cada paso de minuto. Las órdenes y los sucesos discretos (obras, mejoras, políticas, quién entra y sale...) avanzan la versión del mundo como siempre. Efectos:
- **El contador `version` y el hash del mundo son idénticos con sesiones y sin ellas**, y una repetición del registro al recuperar acaba en la misma versión y la misma secuencia interior que el proceso vivo: ningún cliente se ve «del futuro» por esto.
- **Cómo viaja.** Un paso de minuto manda a los clientes de la plaza una `Delta` de `ASENTAMIENTO` con el cambio parcial (`campos` = `version`, `version_interior` y los campos que cambiaron; los `Pueblo` activos de la plaza, completos) y **sin subir de versión**: `hasta` es la del mundo en ese momento (puede ser igual a `desde`). Si nada visible cambió, no se manda nada y el servidor solo avanza la plaza que recuerda del cliente (el `version_interior` que tiene el cliente puede ir por detrás del del servidor sin que falte nada).
- **Cómo se reanuda.** El cliente trae `versiones_conocidas` (versión del mundo del ámbito) y `versiones_interiores_conocidas` (su secuencia interior). Si la secuencia del cliente es **mayor** que la de hoy (un servidor recuperado de un registro más corto: «del futuro»), `REANUDA_CON_FOTO`. Si es **menor**, `REANUDA_CON_DELTAS` con la plaza entera (`reemplaza = true`, más los pueblos), porque no hace falta guardar un registro del interior: la plaza se puede rehacer entera del estado. Si es igual y el registro del mundo no tocó la plaza, `REANUDA_SIN_NOVEDAD`. `Bienvenida.ambitos[].version_interior_actual` dice la de hoy. Un cliente que no la trae (0) recibe la plaza entera, nunca una plaza vieja. En `ADMIN` (T3e) se mira plaza a plaza: la que trae una secuencia distinta de la de hoy (mayor, menor o ninguna) llega entera dentro de la delta de reanudación y `ADMIN` sigue por deltas; `version_interior_actual` va a 0 (cada plaza lleva la suya).
- **No se persiste nada nuevo**: la secuencia es `(ultimo_minuto − fundado_en) / 60 s` de la economía de la plaza, que ya viaja en la foto (sin cambio de formato por esto; hoy `BAN23`, por la memoria de la niebla de T3d).
- **`Pueblo` en `FACCION`**: su almacén local ya no se refresca por minuto en `FACCION` (solo en `ASENTAMIENTO`); en `FACCION` llega con el último cambio discreto del pueblo (DUDA, T3c).

La reapertura de Unity pinta el último estado desde la caché local **antes** de que llegue la sesión, y la sesión lo pone al día.

## 8. Eventos de dominio

`EventoDominio` es un **hecho ocurrido** (crónica, avisos, narración), nunca el cambio de estado: ese viaja como entidades en la misma
`Delta`. Llevan `codigo` (`familia.nombre`), `version`, `en_ms`, `asentamiento_id` (quién lo oye), `heroes_ids` (los nombrados lo oyen
aunque no vean la plaza), `facciones_ids` (las Facciones que lo oyen enteras, 4H T5) y un `payload` estructurado.

- **Ids del contrato también en el `payload`** (decisión del autor, 2026-10-24): un dato que nombra una entidad viaja con su id del contrato y la clave terminada en `Id` (`heroeId: "heroe-3"`, `faccionId: "f-2"`, `campamentoId: "band-4"` o `"camp-4"`, `edificioId: "ed-3-8"`; la batalla, `battleId`), como en los tipados y en las tablas de Mechanics; lo que no nombra una entidad (cantidades, niveles, marcas) va como número.
- **Dos formas a la vez** (decisión del autor, 2026-10-22): todo evento lleva el `payload` genérico (`Struct`: número, texto o lista) y los que mueven la interfaz (**quince** de 4H y **ocho** de la política, 5H) llevan además un
  mensaje tipado en `EventoDominio.tipado`: `BatallaAbierta` (`batalla.abierta`), `BatallaAsignada` (`batalla.asignada`), `BatallaCerrada` (`batalla.aplicada`, `cancelada`, `fallida`),
  `PlazaConquistada` (`asentamiento.conquistada`), `PlazaEnRuinas` (`asentamiento.ruinas`), `AsedioResistido` (`combate.asedio_resistido`), `UnionPedida` (`columna.union_pedida`,
  `convocatoria.union_pedida`), `ConvocatoriaConvocada`, `ConvocatoriaPartio`, `HeroeExpulsado` (`heroe.expulsado`, `heroe.desalojado`), `ColumnaConHambre` (`ejercito.viveres_bajos`,
  `ejercito.sin_viveres`), `DesercionPorHambre` (`tropas.desercion`), `GuerraDeclarada` (`faccion.guerra_declarada`), `ObraTerminada` (edificio, mejora, recinto, pueblo o avanzada
  completados) y `HeroeHerido`. Contra un campamento de bandidos el prefijo es `evento_pve.` y se tipan igual. Desde 5H (2026-10-25), **ocho más de la política**: `PropuestaDiplomatica`
  (`diplomacia.relacion_propuesta`, `anexion_propuesta`, `fusion_propuesta`), `PropuestaResuelta` (las aceptadas, rechazadas y retiradas), `RelacionTerminada` (`diplomacia.relacion_rota`,
  `paz_firmada`, `rebelion_vasallo`, `vasallo_liberado`), `FaccionesUnidas` (`diplomacia.anexion`, `diplomacia.fusion`), `CargoAsignado` (`cargo.local_asignado`, `rey_asignado`,
  `embajador_asignado`), `TituloCambiado` (`titulo.nace`, `titulo.cambia_manos`), `SolicitudDeIngreso` (`faccion.solicitud_ingreso`) y `CiudadaniaCambiada` (`faccion.creada`,
  `ciudadania_union`, `solicitud_denegada`, `abandonada`); `GuerraDeclarada` es la de `diplomacia.guerra_declarada` desde 5D. Desde 6G (2026-10-25), **diez de la tecnología y los Aedas**, que cubren sus quince códigos:
  `LogroCumplido` (`tecnologia.logro`), `EraComenzada` (`era.comienza`), `TecnologiaAparecida` (`tecnologia.aparece`), `TecnologiaAdoptada` (`tecnologia.adoptada`), `DescubrimientoCantado`
  (`aedas.canta_descubrimiento`), `RevelacionDeAeda` (`aedas.revela`, con `faltan` como índices del hito), `CompraAAeda` (`aedas.venta`), `ResidenteCambia` (`aedas.residente_llega`, `residente_se_va`),
  `EpicaCambia` (`aedas.epica_empieza`, `epica_capitulo`, `epica_tecnologia`, `epica_cumplida` —pública, sin la tecnología ni la plaza— y `epica_abandonada`, con su `motivo`: `orden`, `conquista`,
  `aparecida` o `unificada`) y `AlijoAbierto` (`alijo.abierto`). El resto de los códigos solo tienen `payload`.
- **El catálogo de códigos y la forma del payload de cada uno son los documentos de `Docs/Mechanics`**: sus tablas de eventos (p. ej. `Movimiento_Ejercitos` §13.1, `Comercio_Caravanas` §12,
  `Presencia_Y_Convocatoria`, `Diplomacia_Facciones` §9) y, desde 4H T5, el bloque `<!-- eventos -->` de la sección «Catálogo de eventos» de cada uno, la lista de códigos que manda. La prueba
  `crates/nucleo/tests/catalogo.rs` exige que todo código que emite el núcleo esté en el bloque de algún documento y que los que aún no emite figuren como paridad pendiente en
  `Docs/Mecanicas a desarrollar.md` (al cerrar la fase 6, 2026-10-26: 288 códigos, 286 emitidos y 2 pendientes, los de los yacimientos que se agotan y se regeneran).
- **Códigos de la reconstrucción**: los de pueblos, acarreo, avanzadas y obras de mapa son los de `Economia_Construccion` (`pueblo.marcado`, `pueblo.completado`, `pueblo.destruido`, `acarreo.sale`,
  `acarreo.llega`, `avanzada.obra_marcada`, `avanzada.materiales_entregados`, `avanzada.obra_cancelada`, `avanzada.en_ruinas`, `avanzada.destruida`...), los del combate los de `Combate`
  (`combate.pueblo_saqueado`, `combate.caravana_interceptada`/`_resiste` con su modalidad, `faccion.guerra_declarada` al declarar la guerra atacando). Se retiran `ejercito.repliegue` y `ejercito.regresa`.
- **Audiencia** (quién oye cada evento; reglas del autor del 2026-10-24): los de una plaza, su Facción y **quien esté dentro**, sea ciudadano o visitante, aliado o neutral; los que nombran a un héroe,
  él; los de una Facción, ella entera; **un hecho sin plaza que nombra a alguien, solo los nombrados y sus Facciones**; a todos, **solo los públicos** (la crónica, la reposición de los mercados de
  mercenarios); la administración los oye **todos** (`ADMIN`, T3e; como `admin/eventos` del TypeScript). El «sin plaza, global» del motor anterior no rige: lo estrecha la niebla (abajo).

**Cómo lo hace el servidor Rust (4H T5, 2026-10-23).**

- **A quién se cuenta se decide al emitir el hecho**, con el estado de ese instante (`EventoNucleo::completar`): los participantes de las columnas que nombra (y de la clave `ejercito` de sus datos),
  los héroes de las claves `heroe` y `lider` (como `heroesNombrados` del TypeScript), las Facciones dueñas de sus plazas (también la de una plaza que cae en ruinas en la misma transición) y las de las
  claves de Facción de los datos (`faccion`, `faccionA`/`faccionB`, `faccionAtacante`/`faccionDefensora`, `faccionAnterior`). Así una columna disuelta en la misma transición sigue nombrando a sus
  héroes, y la versión, el hash y los hechos (con sus oyentes) son los mismos con sesiones y sin ellas: los hechos no se guardan en la foto.
- **Por qué ámbito**: a cada sesión **una sola vez**, por el primero de sus ámbitos abiertos que lo oye, en este orden: `ASENTAMIENTO` de la plaza (quien está dentro, que es ciudadano de su
  Facción), `BATALLA` del hecho de batalla (quien combate), `HEROE` del héroe nombrado o de quien está dentro de la plaza sin ser de su Facción (un visitante; 2026-10-24), `FACCION` de una Facción que lo oye y, para un hecho de batalla, la `VISTA` de quien tiene esa batalla en su
  mapa (la ve bajo la niebla). `ADMIN` los lleva todos. Un ámbito que se abre con su `Foto` en el mismo reparto que el hecho no lo lleva (una `Foto` no trae hechos): va por el siguiente que lo oye
  (4H T6; antes se perdía, p. ej. `jugador.entra_en_asentamiento` para quien entraba). Al reanudar, solo los de versiones posteriores a la mayor que el cliente declara (§7).
- **La niebla**: un hecho sin plaza que nombra a alguien llega **solo** a los nombrados (en el TypeScript era global: «el ejército X llega a su destino» lo oía todo el mundo); solo son de todos los
  hechos **públicos** (`publico`: la crónica y la reposición de los mercados de mercenarios), que no nombran a nadie ni dicen dónde. Un hecho sin nadie a quien contárselo solo lo oye la administración.
  `vision.contacto` lo oye la Facción que ve, no la vista; `jugador.marcha_fijada` (con el destino), solo quien marcha.
- **Payload**: `ejercitoId` (la primera columna), `x`/`y` (dónde), los datos enteros (lo que nombra una entidad, con el id del contrato y la clave terminada en `Id`: `heroe: 3` → `heroeId: "heroe-3"`; desde T7 también `sitioId` del lugar de un `heroe.expulsado`, `parcelaId`, `atacanteId` de una guerra e `iniciadorId` de una batalla; los bandidos no son una Facción: su `faccion…Id` va vacío), los textos y las listas (`valores`) y, en los hechos de
  batalla, `battleId` con el id del contrato. **Tipados**: los treinta y tres de arriba, con el mismo contenido en ids del contrato (`proyeccion::eventos`).
- **Lo que T7 cerró de los tipados**: `BatallaAbierta` sale de los datos del propio hecho de apertura (contra qué se juega, quién la inició, los bandos y el plazo de la convocatoria), así que se tipa también si la batalla ya se retiró del estado cuando se proyecta (una reanudación tardía); `ObraTerminada.nivel` lleva el nivel interno de un edificio recién terminado y la etapa de una avanzada (1-3), además del de la mejora y el recinto (un pueblo no tiene nivel); `HeroeExpulsado.motivo` distingue `destruido` (lo destruye su dueño o un ataque) de `ruina` (queda en ruinas o cae su plaza madre).
- **La crónica** (Tecnologia_Aedas §8): no es estado; al cerrar cada versión se deriva de sus hechos un `cronica.entrada { tipo, faccionIds, asentamientoId?, nivel? }` público (la conquista, las ruinas,
  la fundación, la guerra declarada y la primera plaza del mundo en llegar a cada nivel desde el 3). La frase la compone el cliente.
- **La crónica entera para quien llega tarde** (sistema 6, tanda 6C, 2026-10-25; decisión Q2 del autor). En las `Delta` solo llega lo que pasa mientras se está conectado; el historial se pide
  con `MensajeDeCliente.pedir_cronica` (`PedirCronica { peticion_id, desde_version?, hasta_version?, limite }`) y llega `MensajeDeServidor.cronica` (`Cronica { peticion_id, entradas[], mas_antiguas,
  mas_recientes, version }`):
  - **Hacia atrás** (sin `desde_version`): las últimas entradas con versión ≤ `hasta_version` (sin ella, las últimas de todas). Para seguir: `hasta_version` = versión de la primera − 1.
  - **Hacia delante** (con `desde_version`): las primeras con versión ≥ `desde_version` (y ≤ `hasta_version`, si viene). Para seguir: `desde_version` = versión de la última + 1.
  - `limite`: 0 = 200; como mucho 1.000 (más se recorta). **Una página nunca parte las entradas de una misma versión**: si la primera versión no cabe, va entera; así encadenar páginas no repite ni salta.
  - `entradas` son los mismos `EventoDominio` `cronica.entrada` que viajaron en las `Delta`, byte a byte. `version` es la del mundo al contestar: lo guardado llega hasta ella; lo posterior llega
    en las `Delta`, y una entrada que esté en las dos se reconoce por su versión.
  - La misma para todos y para cualquier sesión (jugador, bot, administración, servidor de batalla). Cuenta como pendiente, igual que una orden (`Bienvenida.maximo_ordenes_pendientes`); de más,
    `ErrorSesion{demasiadas_ordenes}`. No toca el estado ni bloquea el actor: la lee el hilo de la persistencia, detrás de lo que el núcleo ya le dio. Un servidor sin persistencia contesta páginas vacías.
  - Dorados: `MensajeDeCliente__pedir_cronica`, `MensajeDeServidor__cronica`.

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
- **Bajo la niebla (T3d).** Quien no es de la Facción de ningún bando (un neutral que podría unirse al ataque desde fuera) la ve por su `VISTA` si ve su punto: resumida, con `lado_para_unirse`.
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
2. ~~**Autenticación y acceso**~~ — **cerrada en lo mínimo (2026-10-26, fase 7, 7B)**: registro con código, cuenta de bot, login con token opaco de `ACCESO.duracionSesionMs` y administradores globales por nick (`crates/acceso`). Quedan las membresías por partida (moderador, administrador de la partida) y los tokens de los servidores de batalla (fase 8).
3. **Retención de deltas y tamaño del registro**: se fijan al construir el núcleo con mediciones.
