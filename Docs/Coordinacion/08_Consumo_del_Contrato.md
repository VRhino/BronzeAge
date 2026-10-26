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

Se genera con `buf` y `protoc-gen-es`, con las mismas versiones y la misma plantilla que `contratos/ts` (donde pasan los 89 dorados):

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
CODIGO_REGISTRO_BOTS=<código de bots> ADMINISTRADORES=victor cargo run --release -p servicio
```

| Variable | Qué | Por defecto |
|---|---|---|
| `BRONZEAGE_DIRECCION` | dónde escucha | `127.0.0.1:7878` |
| `BRONZEAGE_MUNDO` | el artefacto de mundo, o una carpeta con un `mundo-*.bin` | `generador/salida/mundo` |
| `BRONZEAGE_DATOS` | la carpeta de datos: `mundo/` (registro, fotos y crónica) y `acceso/` (cuentas y sesiones) | `datos` (ignorada por git) |
| `BRONZEAGE_SEMILLA` | la semilla de la partida (solo cuenta al crearla) | `42` |
| `CODIGO_REGISTRO` | código de invitación de las cuentas humanas; sin él, registro abierto | — |
| `CODIGO_REGISTRO_BOTS` | código que marca la cuenta como bot; sin él, no se pueden crear bots | — |
| `ADMINISTRADORES` | nicks de los administradores globales, separados por comas | — |

El artefacto de mundo no está en git (`generador/salida/` se ignora): en un checkout nuevo se genera una vez con `node scripts/generar-mundo.mjs [semilla]` o con la tarea «generar-mundo» del panel (unos 15 minutos; instala las dependencias del generador si faltan). Sin él, el servidor no arranca y lo dice, y el panel deja apagado su botón.

Un reinicio con la misma carpeta y el mismo artefacto sigue la partida donde se quedó, y los tokens vigentes siguen valiendo. Ctrl+C para el proceso con su foto. Sin servidores de batalla: toda batalla se resuelve con números.

**Entrar, paso a paso** (el CORS está abierto, así que el admin web puede llamar desde el navegador):

1. `POST /v1/registro` con `{"nick", "clave", "codigo"}`. Respuesta `201 {"cuenta_id", "nick", "bot"}`. El nick se recorta y pasa a minúsculas. Con el código de bots, la cuenta es de bot. Errores (cuerpo `{"error", "mensaje"}`): `400 datos_invalidos`, `403 codigo_incorrecto`, `409 nick_repetido`.
2. `POST /v1/sesiones` con `{"nick", "clave"}`. Respuesta `200 {"token", "cuenta_id", "nick", "bot", "expira_ms"}`, con `expira_ms` en ms de época (12 h, `ACCESO.duracionSesionMs`). Cada login da un token nuevo; `401 credenciales_invalidas` si no coincide.
3. El WebSocket en `ws://<dirección>/` con `Hola.token = <token>` y el `Hola.cliente` que toque: una cuenta de bot solo abre `CLIENTE_BOT`, una humana `CLIENTE_UNITY` y un administrador también `CLIENTE_ADMIN_WEB`. Otro tipo da `ErrorSesion{rol_no_permitido}`; un token caducado o ajeno, `token_invalido`.
4. El mapa: `Bienvenida.mundo.url_artefacto` es `/v1/mundo/<hash>`, una ruta relativa al servidor. `GET` en esa ruta devuelve el artefacto de mundo tal cual (`application/octet-stream`, contrato 06); con `Hola.mundo_hash_conocido` igual al vigente no hace falta bajarlo otra vez.
5. Las cifras de las reglas: `Bienvenida.mundo.url_datos_de_juego` es `/v1/datos/<versión>` (la versión, en `version_datos_de_juego`, es el SHA-256 del archivo; `GET /v1/datos` da el vigente). Es un JSON con `documentos` (cada bloque `<!-- parametros:NOMBRE -->` de `Docs/Mechanics`, ya parseado, por documento y nombre: `documentos.Fundacion_Y_Refundacion.FUNDACION`, por ejemplo), `reglas` (lo que no sale de un bloque: el territorio en milésimas de unidad, el aviso de víveres, la idempotencia y si las batallas van a Unity), `derivados` (lo que el núcleo calcula de varios bloques: `costo_caravana_fundacion` y `costo_refundacion`, por recurso en milésimas) y `huella_reglas`. Son las mismas cifras con las que juega el núcleo: un cliente no las copia a mano. Dónde está lo que más se usa, en la tabla de abajo.
6. Opcional: `GET /v1/sesiones/actual` con `Authorization: sesion <token>` dice de quién es y cuándo caduca. `GET /salud` responde `ok`.

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
| Coste de paso por terreno | ya va en el artefacto (`GrafoNavegacion.aristas[].coste_milesimas`); el factor por terreno, en `documentos.Comercio_Caravanas.COSTE_MOVIMIENTO` |

Un `refundacion.fondo_corto` dice cuánto falta en `RechazoOrden.detalle.faltan` (`{ <recurso>: <milésimas> }`), además del texto.

Una cuenta nueva entra sin héroe (`Bienvenida.campamentos_para_elegir`) y lo crea con la orden `crear_heroe`. El héroe queda ligado a la cuenta (`cuenta_id`, `u-N`): al volver a entrar con otro token de la misma cuenta, la `Bienvenida` trae su `heroe_id`.

Detalle y diferencias con el TypeScript: `Docs/Mechanics/Proyecciones_Y_Autorizacion.md` §2.1.
