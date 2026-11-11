# Cómo consumen el contrato los otros repositorios (fase 7, tanda 7A)

Escrito el 2026-10-26 para los frentes de la fase 7: bots (`BronceAgeBotsBrain`, Rust), admin web (`BronceAgeAdminClient`, TypeScript) y cliente de jugador (`BronzeAgeClient`, TypeScript). Todos entran al juego **solo por el contrato 05** (`05_Contrato_de_Sesion.md`), igual que un jugador; ninguno depende del núcleo ni corre dentro del proceso del motor (decisión del autor del 2026-10-26: los bots siempre fuera, para no gastar la CPU del motor).

## Quién es el dueño

- El esquema vive en `proto/bronzeage/v1/` de este repositorio (`VRhino/BronzeAge`). Solo la sesión del motor lo cambia: un frente que necesite un campo, una orden o un evento lo **pide** al motor (a través del autor) y el motor lo hace, con su dorado, `buf lint`, `buf build` y `buf breaking`.
- **Una versión del contrato es un commit de `main`.** Cada frente fija la revisión con la que trabaja (el hash corto) y la sube cuando el motor anuncia un cambio. Revisión de partida: `958b93d` (cierre de la fase 6).
- `buf breaking` impide romper lo que ya existe: un cambio del motor añade, no quita ni renumera. Por eso un cliente con una revisión vieja sigue entendiendo al servidor, aunque no vea lo nuevo.

## Rust (bots): el crate `contrato`

`crates/contrato` trae los tipos generados con `prost` (`contrato::v1::*`) y el almacén de referencia de un cliente (`contrato::almacen::Almacen`, con `plaza::aplicar` y `niebla::aplicar_niebla`). Depende solo de `prost` y `prost-types`; su `build.rs` genera el código desde `../../proto` con un `protoc` incluido, así que necesita el repositorio entero (lo tiene tanto por ruta como por git) y no hace falta instalar nada.

Dos formas, las dos comprobadas el 2026-10-26 con un crate de prueba fuera del workspace:

```toml
# En el PC del autor, contra el checkout local del motor:
contrato = { path = "../BronzeAgeFase0/crates/contrato" }

# Desde cualquier sitio, con revisión fija:
contrato = { git = "https://github.com/VRhino/BronzeAge", rev = "958b93d" }
```

El repositorio es privado: con la forma git, el consumidor necesita en su `.cargo/config.toml`

```toml
[net]
git-fetch-with-cli = true
```

para que cargo use las credenciales de `git` de la máquina.

**Solo `contrato`.** Ni `nucleo`, ni `servidor`, ni ningún otro crate del motor. El cliente WebSocket de las pruebas (`servidor::prueba::ClienteDeEjemplo`, en `crates/servidor/src/prueba.rs`) es código de prueba del servidor: sirve de modelo para escribir el cliente propio (7C), no de dependencia.

## TypeScript (admin y cliente de jugador): `buf generate`

Se genera con `buf` y `protoc-gen-es`, con las mismas versiones y la misma plantilla que `contratos/ts` (donde pasan los 90 dorados):

```yaml
# buf.gen.yaml
version: v2
plugins:
  - local: protoc-gen-es
    out: gen
    opt: target=ts
```

```jsonc
// package.json
"dependencies": { "@bufbuild/protobuf": "2.16.0" },
"devDependencies": { "@bufbuild/buf": "1.73.0", "@bufbuild/protoc-gen-es": "2.16.0" }
```

Y el origen del esquema, por ruta o por git (las dos comprobadas el 2026-10-26; la de git necesita las credenciales de `git` de la máquina):

```sh
npx buf generate ../BronzeAgeFase0/proto --template buf.gen.yaml
npx buf generate "https://github.com/VRhino/BronzeAge.git#ref=958b93d,subdir=proto" --template buf.gen.yaml
```

Conviene guardar la revisión usada en el propio repositorio del cliente (en el script de `package.json`, por ejemplo) para que dos máquinas generen lo mismo.

## Qué más hay que leer

- **Cómo se escribe un cliente**: la sección del mismo nombre en `contratos/README.md` (conectar, guardar por ámbito, ordenar, la `VISTA`, reconectar y la crónica) y su ejemplo, `crates/nucleo/tests/cliente_de_ejemplo.rs`.
- **Los dorados**: `contratos/golden/` (`<Tipo>__<caso>.json` y `.bin`). Un cliente puede usarlos como prueba propia de que decodifica lo mismo que el servidor.
- **Quién puede dar cada orden**: `Docs/Mechanics/Proyecciones_Y_Autorizacion.md` §5.1. **Los rechazos**: `07_Catalogo_de_Rechazos.md` (se decide por el código, nunca por el texto). **La forma de cada orden**: `Docs/Mechanics/Comandos_Parametros.md`.

## El servidor y el acceso (tanda 7B)

El proceso del servidor es `bronzeage-servidor` (`crates/servicio`): carga el artefacto de mundo, abre el núcleo persistente y sirve **en un solo puerto** el WebSocket del contrato 05 y el HTTP del servicio de acceso. Se arranca desde la raíz del repositorio:

```sh
CODIGO_REGISTRO_BOTS=<código de bots> cargo run --release -p servicio
```

| Variable | Qué | Por defecto |
|---|---|---|
| `BRONZEAGE_DIRECCION` | dónde escucha | `127.0.0.1:7878` |
| `BRONZEAGE_MUNDO` | el artefacto de mundo, o una carpeta con un `mundo-*.bin`; su paquete de relieve (`relieve-<hash>.bin`), si está al lado, también se sirve | `generador/salida/mundo` |
| `BRONZEAGE_DATOS` | la carpeta de datos: `mundo/` (registro, fotos y crónica) y `acceso/` (cuentas y sesiones) | `datos` (ignorada por git) |
| `BRONZEAGE_SEMILLA` | la semilla de la partida (solo cuenta al crearla) | `42` |
| `CODIGO_REGISTRO` | código de invitación de las cuentas humanas; sin él, registro abierto | — |
| `CODIGO_REGISTRO_BOTS` | código que marca la cuenta como bot; sin él, no se pueden crear bots | — |
| `CODIGO_REGISTRO_SERVIDORES` | código que marca la cuenta como servidor de batalla (la del lanzador); sin él, no hay ninguna (desde el 2026-11-11) | — |
| `BRONZEAGE_BATALLAS_EN_UNITY` | `1`: las batallas las juegan los servidores de batalla (el lanzador) en vez de resolverse con números. Es una regla de la partida: arrancar una ya creada con el otro valor falla (desde el 2026-11-11) | con números |
| `BRONZEAGE_IPS_ADMIN` | IPs, separadas por comas, desde las que se puede administrar además de esta máquina (el WebSocket `CLIENTE_ADMIN_WEB` y `/v1/admin/*`; siempre hace falta además la cuenta con la marca `admin`) | solo esta máquina |
| `BRONZEAGE_ORIGENES` | nombres de máquina, separados por comas, desde los que una página web puede llamar al servidor, además de esta máquina y de la IP con la que se llega al servidor (desde el 2026-11-08) | — |

El artefacto de mundo no está en git (`generador/salida/` se ignora): en un checkout nuevo se genera una vez con `node scripts/generar-mundo.mjs [semilla]` o con la tarea «generar-mundo» del panel (unos dos minutos con 4 hilos; instala las dependencias del generador si faltan). Escribe el artefacto y, al lado, su paquete de relieve (`relieve-<hash>.bin`, 06 §8). Sin él, el servidor no arranca y lo dice, y el panel deja apagado su botón.

Un reinicio con la misma carpeta y el mismo artefacto sigue la partida donde se quedó, y los tokens vigentes siguen valiendo. Ctrl+C para el proceso con su foto. Sin `BRONZEAGE_BATALLAS_EN_UNITY=1`, toda batalla se resuelve con números; con él, las juega el lanzador de servidores de batalla (`bronzeage-lanzador`, [`Batalla_Unity_Ciclo.md`](../Mechanics/Batalla_Unity_Ciclo.md) §8.8; en el panel, la casilla «batallas en Unity» de una partida nueva y el servicio `bronzeage-lanzador`).

**Quién puede llamar desde un navegador** (desde el 2026-11-08; antes el CORS era `*`): si la petición HTTP o el handshake del WebSocket trae `Origin`, solo vale si es de esta máquina (`localhost`, `127.0.0.1`, `[::1]`, cualquier puerto), de la misma IP con la que se llega al servidor o de un nombre de `BRONZEAGE_ORIGENES`; si no, `403 origen_no_permitido` sin cabeceras CORS, y el WebSocket no se abre. El CORS devuelve el origen permitido que pidió (con `Vary: Origin`), no `*`. Un dominio que no esté en la lista no vale aunque coincida con `Host` (contra el DNS rebinding). Sin `Origin` (bots, Unity, `curl`) no se mira nada. **Administración por IP** (desde el 2026-11-08): las rutas `/v1/admin/*` (`403 ip_no_permitida`) y un `Hola` de `CLIENTE_ADMIN_WEB` (`rol_no_permitido`, fatal) solo se atienden desde la máquina del servidor o una IP de `BRONZEAGE_IPS_ADMIN`; detrás de un proxy inverso en la misma máquina todas las conexiones parecen locales y la protección es solo la cuenta.

**Entrar, paso a paso:**

1. `POST /v1/registro` con `{"nick", "clave", "codigo"}`. Respuesta `201 {"cuenta_id", "nick", "bot", "admin", "servidor"}`. El nick se recorta y pasa a minúsculas. Con el código de bots, la cuenta es de bot; con el de servidores, de servidor de batalla. **Cómo nace la primera cuenta de administrador**: si al arrancar el servidor no hay ninguna cuenta administradora, imprime en su consola (y por tanto en el log del panel) un código de un solo uso; quien se registra con él en `codigo` nace administrador (`admin: true`) y lo gasta. Ese código solo existe en memoria y mientras no haya administrador; un bot nunca lo es. Con administrador ya hecho, los demás se dan y quitan con las rutas de abajo. Errores (cuerpo `{"error", "mensaje"}`): `400 datos_invalidos`, `403 codigo_incorrecto`, `409 nick_repetido`.
2. `POST /v1/sesiones` con `{"nick", "clave"}`. Respuesta `200 {"token", "cuenta_id", "nick", "bot", "admin", "servidor", "expira_ms"}`, con `expira_ms` en ms de época (12 h, `ACCESO.duracionSesionMs`). Cada login da un token nuevo; `401 credenciales_invalidas` si no coincide.
3. El WebSocket en `ws://<dirección>/` con `Hola.token = <token>` y el `Hola.cliente` que toque: una cuenta de bot solo abre `CLIENTE_BOT`, una de servidor de batalla solo `CLIENTE_SERVIDOR_BATALLA`, una humana `CLIENTE_UNITY` y una cuenta con `admin` también `CLIENTE_ADMIN_WEB`. Otro tipo da `ErrorSesion{rol_no_permitido}`; un token caducado o ajeno, `token_invalido`.
4. El mapa: `Bienvenida.mundo.url_artefacto` es `/v1/mundo/<hash>`, una ruta relativa al servidor. `GET` en esa ruta devuelve el artefacto de mundo tal cual (`application/octet-stream`, contrato 06); con `Hola.mundo_hash_conocido` igual al vigente no hace falta bajarlo otra vez. Los clientes 3D bajan además el relieve fino: `Bienvenida.mundo.url_relieve` es `/v1/mundo/<hash>/relieve` y `hash_relieve`, su hash (`PaqueteRelieve`, 06 §8, unos 9 MB). Se baja una vez por mundo y se guarda por hash: si el que hay en disco tiene ese `hash_relieve`, no se baja otra vez. Si el servidor no tiene el paquete, las dos van vacías. Los bots y el admin web no lo necesitan.
5. Las cifras de las reglas: `Bienvenida.mundo.url_datos_de_juego` es `/v1/datos/<versión>` (la versión, en `version_datos_de_juego`, es el SHA-256 del archivo; `GET /v1/datos` da el vigente). Es un JSON con `documentos` (cada bloque `<!-- parametros:NOMBRE -->` de `Docs/Mechanics`, ya parseado, por documento y nombre: `documentos.Fundacion_Y_Refundacion.FUNDACION`, por ejemplo), `reglas` (lo que no sale de un bloque: el territorio en milésimas de unidad, el aviso de víveres, la idempotencia y si las batallas van a Unity), `derivados` (lo que el núcleo calcula de varios bloques: `costo_caravana_fundacion` y `costo_refundacion`, por recurso en milésimas) y `huella_reglas`. Son las mismas cifras con las que juega el núcleo: un cliente no las copia a mano. Dónde está lo que más se usa, en la tabla de abajo.
6. Opcional: `GET /v1/sesiones/actual` con `Authorization: sesion <token>` dice de quién es (con `admin`) y cuándo caduca. `DELETE /v1/sesiones/actual` con la misma cabecera cierra la sesión: anula ese token (`204`; `401 token_invalido` si no había sesión) y los demás tokens de la cuenta siguen valiendo (desde el 2026-11-08). Si el servidor tiene todos sus cálculos de clave ocupados, el registro y el login contestan `503 ocupado`: se reintenta en unos segundos. `GET /salud` responde `ok`.

**Dónde está cada cifra en `/v1/datos`** (dentro de `documentos` las cifras van como en el bloque, en unidades del juego; en `reglas` y `derivados`, en milésimas):

| Qué | Ruta |
|---|---|
| Coste de la Caravana de Fundación / de la refundación desde un campamento | `derivados.costo_caravana_fundacion` / `derivados.costo_refundacion` |
| Catálogo de tropas: poder (`poderBase`), velocidad, tamaño de escuadra y de la leva prestada (`unidadesPorDefecto`), coste y edificio | `documentos.Heroe_Y_Tropa.TROPAS_RECLUTABLES` |
| Consumo de ración por soldado y minuto, poder por héroe | `documentos.Combate.MILITAR` (`racionPorSoldadoPorMinuto`, `poderPorHeroe`) |
| Víveres al salir (capacidad por héroe) y consumo en marcha y estacionado | `documentos.Movimiento_Ejercitos.LOGISTICA` (`capacidadViveresPorHeroe`, `factorConsumoEnMarcha`, `factorConsumoEstacionado`) |
| Ración gratis del campamento (cada cuánto llena los víveres) | `documentos.Campamentos_Mercenarios.MERCENARIOS.racion.cadaMinutos` |
| Radio de encuentro / radio de puerta / velocidad del jugador | `documentos.Movimiento_Ejercitos.LOGISTICA.radioEncuentro` / `.MOVIMIENTO.radioPuerta` / `.MOVIMIENTO.velocidadJugador` |
| Visión del jugador solo | `documentos.Movimiento_Ejercitos.VISION.jugadorSolo` (el mismo bloque está en `Niebla_De_Guerra.VISION`) |
| Anillo de bandidos de un campamento (radios, cuántos por residentes, reaparición) y sus niveles | `documentos.Generacion_Del_Mundo.CAMPAMENTOS_BANDIDOS` (`anillo`, `niveles`) |
| Misiones: catálogo (condiciones, previas y recompensa: la fija, `participacion` en las de plaza y `xp` en las de Facción, y la lista `recompensas` de elementos `{ tipo, ... }`; Misiones §4.1) y sus textos (título, qué hacer, explicación y nombre de cada contador) | `documentos.Misiones.MISIONES` y `documentos.Misiones_Textos.TEXTOS_MISIONES` (`misiones.<id>.titulo`/`.texto`/`.explicacion`, `contadores.<contador>`) |
| Participación: los puntos de cada origen (defensa, batalla ganada) | `documentos.Participacion.PARTICIPACION` (`defensa`, `batallaGanada`) |
| Misiones recurrentes: el catálogo **inicial** (ámbito, duración, tipo, tope, cargo, condiciones, recompensa, que es la fija y la lista `recompensas` como en las únicas, y textos) | `documentos.Misiones_Recurrentes.MISIONES_RECURRENTES`; el **vigente** (la administración lo cambia en caliente) viaja en cada `Tablon.catalogo`, y cada misión del tablón lleva su definición |
| Aldea agrícola: niveles (granjas, corrales, coste y obra), fertilidad mínima, separación, cupo de pueblos por nivel de la madre; especies del rebaño (tope y pie de cría por corral, crecimiento por hora, productos por cabeza) | `documentos.Economia_Construccion.PUEBLOS_EXTRACTORES` (`aldea`, `cupoPorNivel`, `nivelMadreMinimo`) y `documentos.Economia_Construccion.ESPECIES`; el estado de cada aldea va en `Pueblo` (`nivel_aldea`, `fertilidad`, `almacen_por_recurso`, `rebano_especie`, `rebano_cabezas`) |
| Guarnición de jugadores de un pueblo o una aldea: cupo por héroe en Liderazgo (nivel 1 y lo que suma cada nivel) | `documentos.Economia_Construccion.PUEBLOS_EXTRACTORES.guarnicionJugadores` (`cupo`, `porNivel`); el cupo ya calculado de cada pueblo va en `Pueblo.cupo_guarnicion_por_heroe` |
| Coste de paso por terreno | ya va en el artefacto (`GrafoNavegacion.aristas[].coste_milesimas`); el factor por terreno, en `documentos.Comercio_Caravanas.COSTE_MOVIMIENTO` |

Un `refundacion.fondo_corto` dice cuánto falta en `RechazoOrden.detalle.faltan` (`{ <recurso>: <milésimas> }`), además del texto.

Una cuenta nueva entra sin héroe (`Bienvenida.campamentos_para_elegir`) y lo crea con la orden `crear_heroe`. El héroe queda ligado a la cuenta (`cuenta_id`, `u-N`): al volver a entrar con otro token de la misma cuenta, la `Bienvenida` trae su `heroe_id`.

Detalle y diferencias con el TypeScript: `Docs/Mechanics/Proyecciones_Y_Autorizacion.md` §2.1.

## Administrar partidas (admin web, fase 7)

Dos sitios, porque parar y arrancar el proceso no lo puede hacer el propio proceso:

**El servidor** (`crates/servicio/src/admin.rs`), en su mismo puerto y solo con `Authorization: sesion <token>` de una cuenta con la marca `admin` (sin token o caducado, `401 token_invalido`; de otra cuenta, `403 no_autorizado`):

| Ruta | Qué |
|---|---|
| `GET /v1/admin/partida` | `{ carpeta, id, mundo_hash, semilla, version, instante, cuentas, bots, codigo_registro, respaldos }`; `id` es el de `<datos>/partida.id`, el `gameId` del contrato de batalla |
| `GET /v1/admin/registro/codigo` | `{ codigo }` (`null`: registro abierto) |
| `PUT /v1/admin/registro/codigo` | cuerpo `{ codigo: texto \| null }`; cambia en caliente hasta el próximo reinicio (después vuelve `CODIGO_REGISTRO`) |
| `POST /v1/admin/respaldos` | foto ahora y copia de `mundo/` y `acceso/` en `<datos>/respaldos/<id>/`; `201 { id, creado_ms, version, instante, mundo_hash, semilla, bytes }` |
| `GET /v1/admin/respaldos` | `{ respaldos: [...] }`, del más nuevo al más viejo |
| `GET /v1/admin/misiones/catalogo` | el catálogo **vigente** de las misiones recurrentes `{ asentamiento: [...], faccion: [...] }`, en el JSON que acepta la orden de administración `fijar_catalogo_misiones` (la fija, `participacion` o `xp`, y la lista `recompensas`, con las cantidades en unidades; Misiones_Recurrentes §6) |
| `GET /v1/admin/carga` | la medida de carga del motor: `{ activo, detalle, cada_s, ultima }`, con la última ventana de 10 s (`null` si aún no hay); formato en `Docs/Arquitectura/Medida_De_Carga.md` (desde el 2026-11-05) |
| `POST /v1/admin/carga` | cuerpo `{ activo?, detalle? }` (booleanos); enciende o apaga en caliente lo básico y el detalle; contesta como el `GET` |
| `GET /v1/admin/administradores` | `{ administradores: [nick, ...] }`, ordenados (desde el 2026-11-08) |
| `PUT /v1/admin/administradores` | cuerpo `{ nick, admin }`; da (`true`) o quita (`false`) la marca de administrador a la cuenta; `200 { nick, admin }`. `404 no_encontrado`, `400 es_bot`, `409 ultimo_administrador` (nunca se queda la partida sin administradores). Lo mismo, por la entrada estándar del proceso: `admin dar <nick>` y `admin quitar <nick>` |

El núcleo atiende el respaldo entre dos órdenes y no hace nada más mientras copia, así que la copia es coherente. El histórico de batallas retiradas está en `<datos>/mundo/batallas.jsonl` (un JSON por línea, Batalla_Unity_Ciclo §3.3), así que va en el respaldo.

**El panel de arranque** (`scripts/panel.mjs`, puerto 3999), que pasa su dirección al admin como `PANEL_URL` (como `BOTS_URL`). Lanza dos procesos de bots con el mismo ejecutable: los libres (`bronzeage-bots`, puerto 4000, control en `BOTS_URL`) y los mercenarios (`bronzeage-bots-mercenarios`, `BOTS_POBLACION=mercenarios`, puerto 4001, control en `MERCS_URL`; desde el 2026-11-02). A los dos les pasa `BOTS_REGISTROS` con la ruta absoluta de `partidas/` (desde el 2026-11-03): los bots guardan sus registros dentro de cada partida (`partidas/<nombre>/bots.json` y `mercs.json`), así que borrar una partida borra también sus bots, y `servicio-bots.json` y `servicio-mercs.json` quedan en `partidas/`. Los secretos de los bots ya no están en `.claude/launch.json` (desde el 2026-11-08): el panel genera `CODIGO_REGISTRO_BOTS` (servidor y los dos procesos de bots) y `BOTS_TOKEN` (token del canal de control, solo los bots) la primera vez, los guarda en `partidas/secretos.json` y los reutiliza; manda el valor que traiga el `env` del servicio o el entorno del propio panel. Desde el 2026-11-11 genera igual `CODIGO_REGISTRO_SERVIDORES` (servidor y lanzador) y `LANZADOR_CLAVE` (la clave de la cuenta `lanzador-local` del servicio `bronzeage-lanzador`, que solo arranca con una partida en Unity y juega cada batalla con el proceso que diga su `env` en `.claude/launch.json`: el de mentira, de serie). El token se ve, con un botón «Copiar», en la pestaña «Servicios» junto a los bots y sale en `GET /api/estado` (`token_control`) para pegarlo en el campo del admin; el código de registro no sale del panel. Cuerpos JSON. Desde el 2026-11-08 solo atiende si `Host` es de esta máquina (`localhost`, `127.0.0.1`, `[::1]`) y, si viene `Origin`, si también lo es; si no, `403`. El CORS devuelve ese origen, no `*`. Desde el 2026-10-27 las partidas y el mundo se gestionan en la pestaña «Mundo y partidas» del propio panel (la otra pestaña, «Servicios», es la vista de procesos de siempre), porque funcionan con el servidor apagado; el admin ya no las lleva. Una partida es una carpeta de datos del servidor en `partidas/<nombre>/` (ignorada por git) con su `partida.json` (`semilla`, `mundo`, y el artefacto con el que se creó; los administradores ya no se guardan aquí: viven en las cuentas de la partida, en `acceso/cuentas.jsonl`); la carpeta `datos/` de antes cuenta como la partida `datos`. El servidor del panel arranca siempre con la partida actual.

| Ruta | Qué |
|---|---|
| `GET /api/partidas` | `{ actual, servidor: { lanzado, escucha }, partidas: [{ nombre, carpeta, semilla, mundo, creada_ms, batallas_en_unity, respaldos, actual, en_marcha }], artefactos: [{ archivo, hash, creado_ms }] }`. `actual: null` es «sin partida activa»: el servidor no arranca hasta crear o activar una |
| `POST /api/partidas` | `{ nombre, semilla?, mundo?, batallas_en_unity?, activar? }`: crea la partida con el artefacto dado (archivo o hash; por defecto el más nuevo), con las batallas en Unity si `batallas_en_unity: true` (desde el 2026-11-11; se guarda en su `partida.json` y el servidor arranca con `BRONZEAGE_BATALLAS_EN_UNITY`) y, salvo `activar: false`, para el servidor y lo arranca con ella. `201`; `409 partida_repetida` o `sin_mundo` |
| `GET /api/partidas/<nombre>` | la partida y su `lista_respaldos` |
| `POST /api/partidas/<nombre>/activar` | cambiar de partida: para el servidor (con su foto) y lo arranca con esa |
| `POST /api/partidas/<nombre>/apagar` | solo la activa: para el servidor (con su foto) y deja el panel **sin partida activa**; la partida se queda guardada. `{ apagada, estaba_en_marcha, actual: null }`; `409 no_es_la_activa` |
| `DELETE /api/partidas/<nombre>` | la borra entera. La activa pide el cuerpo `{ "confirmar": true }` (si no, `409 partida_activa`): para el servidor, la borra y deja el panel sin partida activa |
| `GET /api/partidas/<nombre>/respaldos` | `{ respaldos }` (los del servidor y los del panel) |
| `POST /api/partidas/<nombre>/respaldar` | `{ nota? }`: copia `mundo/` y `acceso/` en `respaldos/panel-<ms>/`. Si es la activa y está en marcha, para el servidor (con su foto), copia y lo vuelve a arrancar. `201` |
| `POST /api/partidas/<nombre>/restaurar` | `{ respaldo: <id> }`: para el servidor si sirve esa partida, guarda lo de ahora como el respaldo `antes-de-restaurar-<ms>`, copia el respaldo encima y vuelve a arrancar |
| `GET /api/mundo` | `{ generando, semilla, ultima: { semilla, codigo, fin_ms, artefacto }, artefactos }`: el admin lo consulta para saber cuándo terminó (código 0: bien) |
| `POST /api/mundo` | `{ semilla }`: lanza la tarea «generar-mundo» con esa semilla (unos 2 min); `202`, o `409 ya_generando` |
| `DELETE /api/mundo/<archivo>` | Borra ese artefacto de mundo (el `archivo` o el `hash` de `artefactos`). `409 mundo_en_uso` (con `partidas`) si lo usa alguna partida: las que lo fijaron al crearse y, si es el más nuevo, las que usan «el último»; `409 ya_generando` mientras se genera uno. En el panel: botón «Borrar» en la lista de la sección Mundo |

Las que arrancan el servidor contestan también `{ arrancado, aviso }`: si no pudo arrancar, `aviso` dice por qué (falta el artefacto de mundo, por ejemplo).
