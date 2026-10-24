# Proyecciones y autorización

> **Reescrito el 2026-10-13** en lo que cambia con la reconstrucción; el resto es la autorización y la proyección tal como son y pasa al contrato 05. Ver §«Cambios de la reconstrucción» al final.

> **Implementado en Rust (2026-10-20, fase 4B-1), autorización provisional de los héroes.** Cada orden de un héroe la da su `cuenta` (`Peticion.actor`) o la administración; «fuera del mundo» bloquea todo menos `conectarse` (`heroe.fuera_del_mundo`); la presencia que pide `reclutarTropa` es `ubicacion = asentamiento` de esa plaza. Los cargos, la ciudadanía como condición y los vetos son del sistema político; mientras, la cuenta de un héroe ciudadano de la Facción de una plaza **manda en ella** como el propietario de una columna (`manda_en`). Las proyecciones y los canales del héroe son de la fase 4H.

> **Implementado en Rust (2026-10-20, fase 4B-2), autorización y proyección de las columnas.** Las 22 órdenes nuevas (69-90, `Movimiento_Ejercitos.md` y `Presencia_Y_Convocatoria.md`) las da la **cuenta del héroe** que las nombra (o la administración); un héroe fuera del mundo solo puede conectarse. Para una columna: `marcharA` y `estacionarEjercito` las da su **Líder** a bordo (no desde dentro de un lugar, no una formación, `columna.en_formacion`), la administración puede con todas; `comerciarEnPlaza` y `fundar`, también su Líder; `adjuntarCaravana`, `aparcarCaravana` y `entregarMateriales`, cualquier participante; las de convocatoria, la cuenta del héroe (cancelar, responder a una petición y partir son del Líder de la convocatoria). Lo demás (política, distancia, Facción, existencias) lo valida el núcleo antes de tocar nada y sus rechazos, con sus códigos propios, están en las notas de cada documento. **Una columna pertenece a las cuentas de todos sus participantes** (`sesiones`): las ve y recibe sus eventos cada una. La entidad `Ejercito` del contrato lleva `participantes`, `tipo`, `lider_id`, `politica_de_union`, `peticiones_de_union`, `escuadron_ids`, `suministro`, `viveres` y `capacidad_viveres`, `formacion_expira_en_ms`, `caravanas_adjuntas_ids`, el estado y el plan; los ids son `heroe-N` y `ej-N`. **Aún no:** la baja de una columna disuelta en un delta (las proyecciones y la fase 4H completa), la visión por bandera con soldados (`alcance_de_vista` ya la calcula), `ordenAtacar` y los eventos de combate.

> **Implementado en Rust (2026-10-23, fase 4H T3a), los ámbitos de la sesión.** Los canales de vista de §7.3 pasan a ser los ámbitos del contrato 05 §3, que el servidor abre y cierra solo (`sesiones/ambitos.rs`): `HEROE` (el héroe de la cuenta), `FACCION`, `ASENTAMIENTO` (el interior: ciudadano de la Facción dueña y dentro, la regla de `vista/asentamiento/<id>`), `CAMPAMENTO` (dentro), `VISTA` (la Facción, con el rectángulo de `CambiarVista`; sin Facción no hay) y `ADMIN` (solo administración; nadie más ve `ADMIN`, y la administración no abre los demás). Perder el derecho (salir de la plaza, cambiar de Facción, la ruina de la plaza) manda `Revocado{sin_derecho | ya_no_existe}` en el mismo reparto que lo causa y ganarlo manda la `Foto`; la reanudación decide por ámbito con `Hola.versiones_conocidas` (foto si el héroe cambió desde esa versión en los que dependen del sitio). El jugador sin héroe recibe `campamentos_para_elegir`. **Aún no** (T3b-T3e): el contenido de `FACCION` y `CAMPAMENTO`, el interior completo de la plaza, la niebla en `VISTA`, el resto del estado en `ADMIN` y las audiencias de eventos (hoy solo por `HEROE`).
>
> **Implementado en Rust (2026-10-23, fase 4H T3b), el contenido de `HEROE` y `FACCION`.** Del `heroe`/`ejercitos`/`caravanas`/`convocatorias`/`ordenes`/`acuerdos` de §6.3 salen las entidades del contrato 05: el héroe de la sesión con sus escuadras y loadouts (`Heroe` con `cupo_guarnicion`, Liderazgo de cada loadout y residencia derivados; nombre, clase, género y avatar por defecto hasta que Conquest publique su catálogo), las columnas en que participa alguna cuenta del héroe, las caravanas adjuntas a ellas o que escolta con una escuadra suya y las batallas en que combate (`HEROE`); y la Facción con sus ciudadanos, relaciones y tecnologías del andamiaje, sus plazas en resumen, su flota completa, sus pueblos y avanzadas (con sus obras de mapa), sus trueques (de las dos Facciones), sus órdenes de mercado con historial y sus convocatorias con las peticiones redactadas (el Líder ve todas, cada solicitante la suya) (`FACCION`). **Quién recibe qué** lo decide una función pura del estado (`proyeccion::audiencia`) que el núcleo evalúa al cerrar cada versión y deja en el registro de cambios junto a los ámbitos de los que cada entidad salió (`CambioVersion.pertenencias`): así llega el `QUITA` de lo que desaparece (una columna disuelta, una caravana perdida) o cambia de dueño (una plaza conquistada). Los cambios viajan completos (`reemplaza = true`). La reanudación por versión de `HEROE` y `FACCION` es siempre por deltas. **Aún no** (T3d-T3e, T5; el interior, el ticket y el informe de inspección llegaron en T3c y T4): la niebla, lo ajeno redactado y las audiencias de eventos. Dudas y recortes: filas «Contrato (4H, T3b)» de `Docs/Mecanicas a desarrollar.md`.

> **Implementado en Rust (2026-10-23, fase 4H T3c), el contenido de `ASENTAMIENTO` y `CAMPAMENTO`** (foto `BAN22`). `vista/asentamiento/<id>` del TypeScript (el interior sin `edificios` más `edificios` como colección y `trazado`) pasa a ser **una** `Asentamiento` completa más los `Pueblo` de la plaza, y `admin/campamento/<id>` / la escena del campamento, **un** `CampamentoMercenarios` (`proyeccion::interior`). La plaza viaja como **cambio parcial con máscara de campos** (decisión 2 del autor: la sesión recuerda la plaza que tiene el cliente y manda solo los campos que cambiaron; una reanudación, la plaza entera) y el campamento y lo demás, completos. El almacén y la población van en escalones con la tendencia «+N/min» (decisión 3: el neto real del último paso de minuto, `tendencia` de la economía de la plaza, que entra en la foto: `BAN22`). El paso de minuto **no marca nada ni sube la versión del mundo** (con sesiones o sin ellas): el interior de la plaza lleva su propia secuencia, `Asentamiento.version_interior` (`Nucleo::version_interior`), que la sesión compara con la que recuerda del cliente y con la que este trae al reanudar (`Hola.versiones_interiores_conocidas`; contrato 05 §7); quién entra, sale, se muda o aparca marca la plaza o el campamento donde cambió (`Clase::Interior`, `nucleo/interiores.rs`, solo con sesiones y dentro de una versión que ya se cierra por otra razón). `ASENTAMIENTO` y `CAMPAMENTO` se reanudan siempre por deltas. Dudas: filas «Contrato (4H, T3c)» de `Docs/Mecanicas a desarrollar.md`.

> **Implementado en Rust (2026-10-23, fase 4H T3d), el ámbito `VISTA` con niebla** (foto `BAN23`). `vista/mapa` del TypeScript (§6.3, la capa «mapa», con lo de la capa «héroe» que es del mapa) pasa a ser el ámbito `VISTA` del contrato 05: `VISTA:f-N` para el ciudadano de una Facción y, sin bandera, `VISTA:heroe-N` (la niebla personal del héroe, que T3a dejaba sin vista), siempre dentro del rectángulo de `CambiarVista`. Lo que se ve lo decide `proyeccion::vista::Mirada` (una por perspectiva, para todas sus sesiones): los ojos propios y los de los aliados, las Miradas abiertas, lo explorado (la memoria más lo que cada columna propia lleva andado sin grabar) y lo recordado; `Mirada::contenido` da cada entidad con su **sello** (versión y, de lo ajeno que anda, dónde acaba el tramo que se le mandó) y la sesión recuerda los sellos que tiene el cliente (`Mapa`): **la `VISTA` no sale del registro de cambios** sino de comparar lo que se ve con lo que el cliente tiene, tras cada reparto y en un muestreo de 250 ms de mundo que no toca el estado (`ACTUALIZA` de lo que entra o cambia, `QUITA` de lo que sale de la vista o del rectángulo, `NieblaProyectada` cuando cambia). Lo propio viaja como en `FACCION`/`HEROE` (la columna entera, la ficha de la plaza, los pueblos y avanzadas con sus obras) y lo ajeno, con las formas públicas de la tabla de abajo (`EjercitoAvistado`, `CaravanaAvistada` → `Caravana.recursos_avistados`, `AsentamientoAvistado`, `HeroePublico` → `Heroe.escuadras_que_lleva`, `BatallaVisible` → `propios::batalla` del espectador) más `FichaConocida` (plazas, pueblos y avanzadas recordados), `CampamentoBandidos`, `CampamentoMercenarios` (los explorados y el suyo), `ZonaFaccion`, `RedCaminos` (aristas exploradas), `MiradaIntel` e `InformePlaza`. La vista no se reanuda: se pide con `CambiarVista` al reconectar. `comprarMirada` y `comprarInformePlaza` existen (comandos 126-127): las da la cuenta del héroe, que tiene que estar dentro del lugar de la taberna (una plaza de su Facción o un campamento); DUDA: el cargo (Rey, Embajador, Gobernador) de §5 es del sistema 5. Dudas y recortes: filas «Contrato (4H, T3d)» de `Docs/Mecanicas a desarrollar.md`.

> **Implementado en Rust (2026-10-23, fase 4H T3e), el ámbito `ADMIN`** (sin cambio de foto; esquema aditivo: `Batalla.asignacion`). `vistaAdminDeEstado` y los canales `admin/*` de §6.1 y §7.3 pasan a ser **un** ámbito del contrato 05, `ADMIN:`, que solo abre una sesión `CLIENTE_ADMIN_WEB` (lo que otra clase de cliente diga conocer de él se ignora; la administración no abre ningún otro ni tiene vista). Lleva **todo el estado, sin niebla ni redacción**, con las proyecciones de los ámbitos de jugador vistas por la administración (`propios::Espectador::Admin`): cada plaza **con su interior** (lo que en el TypeScript era `admin/asentamiento/<id>`, con su secuencia interior y su máscara por minuto), todas las columnas, héroes (con sus escuadras, loadouts, almacén personal e informes de inspección), Facciones con sus tecnologías y relaciones, caravanas, pueblos, avanzadas y obras (también las huérfanas), trueques, órdenes de mercado con su historial, convocatorias con **todas** sus peticiones, campamentos de mercenarios (`admin/campamento/<id>`) y de bandidos, y batallas **enteras** (ticket, incorporaciones, resultado) con la asignación que ganó **sin tokens** (`tokensParticipante` vacío, como aquí; nunca `asignacion_propia`). Y todos los eventos (`admin/eventos`). Se reparte del registro de cambios (todo lo que toca cada versión; ninguna entidad entra ni sale de `ADMIN` sin cambiar) y se reanuda por deltas, con la secuencia interior de cada plaza en `Hola.versiones_interiores_conocidas` bajo `ASENTAMIENTO:as-N`. La versión y el hash del mundo son los mismos con sesiones de administración y sin ellas. **Aún no**: `preciosReferencia`, `relojDeMundoIntervaloMs` (el reloj va en la `Bienvenida` y en `Aviso{reloj.cambiado}`), `titulos`, Aedas, el historial de cada héroe (`admin/heroe/<id>`), la memoria de la niebla de cada Facción (lo explorado, las fichas, las Miradas y los Informes de plaza), `RedCaminos` y las zonas fundidas. **Un solo rol de administración** (§2.2). Dudas y recortes: filas «Contrato (4H, T3e)» de `Docs/Mecanicas a desarrollar.md`.

> **Implementado en Rust (2026-10-24, fase 4H T7), la matriz y lo que faltaba proyectar.** La matriz de §5.1 es la autorización de hoy, orden a orden, declarada en `Comando::autoriza` y comprobada contra este documento por una prueba; la de administración la aplica una sola vez `Nucleo::ejecutar` (antes, cada orden en su sitio). Los rechazos son un catálogo estable (`Docs/Coordinacion/07_Catalogo_de_Rechazos.md`). La vista lleva ya el suelo bajo cada columna propia (`territorioPorEjercito`), el almacén de la plaza propia a cuya puerta se está (`detalleDeAvistados`), la zona de la Facción fundida y la red de caminos con su escalón y las rutas propias; el héroe, el interior que recuerda de su plaza (`plazasRecordadas`); la administración, qué servidor de batalla tiene cada batalla. Lo que falta (de los sistemas 5 y 6 o de Conquest): filas «Contrato (4H)» de `Docs/Mecanicas a desarrollar.md`.

## 1. Alcance y fuentes

Cubre **quién puede hacer qué** y **qué ve cada uno**: los roles técnicos y las superficies del servidor, el camino de un comando desde la petición hasta el motor, los predicados de pertenencia en los que se apoya la autorización, **la matriz de autorización** (en Rust, las 145 órdenes del contrato y los 127 comandos del núcleo, §5.1; la del TypeScript, de 122 comandos, en §5.2), lo que el servidor proyecta a cada audiencia (administración, jugador, eventos) y cómo se reparte por los canales de tiempo real (foto + delta).

- **Verificado contra:** `3d19d31` (2026-10-12).
- **Fuentes:** `acceso/tipos.ts`, `acceso/rolesDePartida.ts`, `acceso/servicioAutenticacion.ts`, `engine/pertenencia.ts`, `session/comandos/autorizacion.ts`, `session/comandos/registro.ts`, `session/comandos/tipos.ts`, `session/proyecciones/jugador.ts`, `session/canales.ts`, `session/vistas/{segmentos,diff,movimiento}.ts`, `session/estado.ts` (`vistaAdminDeEstado`), `server/rutas/*.ts`, `server/difusion/{hub,difusorDeVistas}.ts`, `server/runnerDePartida.ts` (`ejecutar`).
- **Canon:** [`Docs/Game/2_Sistema_Politico_Facciones_y_Cargos.md`](../Game/2_Sistema_Politico_Facciones_y_Cargos.md) 2.2 (cargos) y 2.5 (la ciudadanía habilita, la presencia ejerce), [`Docs/Game/1_Sistema_de_Mundo_y_Territorio.md`](../Game/1_Sistema_de_Mundo_y_Territorio.md) 1.10 (el héroe está situado). El contrato con los clientes está en `Docs/Coordinacion/` (`02`, `03`); la arquitectura de identidad, en `typescript-final:Docs/Arquitectura/5_Contratos_Identidad_Permisos.md`.
- **No está aquí** (está en su documento):
  - **Las reglas de la niebla** (qué se ve ahora, qué se recuerda, ojos, visión compartida, Miradas) → [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) §3–§5. Aquí solo se dice dónde encaja cada una en la proyección.
  - **Qué hace cada comando por dentro** → el documento de su familia (la tabla de §5 dice cuál).
  - **El ciclo de la batalla de Unity** (rutas `/v1/batallas/*` que usa el servidor de batalla, el candado, aplicar el resultado) → `Batalla_Unity_Ciclo` (pendiente). Aquí solo se nombra su lugar en el camino de un comando (§3).
  - La persistencia, el diario, los respaldos, la auditoría en disco y la telemetría: son de despliegue, no del juego.
  - El generador aleatorio: ninguna de estas piezas lo consume.

## 2. Actores

### 2.1 Identidad

```
Usuario            { id, creadoEn, deshabilitado }                 una cuenta, fuera de cualquier partida
IdentidadVinculada { usuarioId, proveedor, sujetoId, email? }      N identidades por Usuario
Sesion             { id, usuarioId, emitidaEn, expiraEn }          credencial activa; dura DURACION_SESION_MS
Membresia          { usuarioId, gameId, jugadorId|null, rol, desde, hasta?, motivoFin? }
                   — UNA por (Usuario, partida). Vigente si hasta == null ∨ hasta > ahora (`esVigente`).
```

- **Login** (`POST /v1/sesiones`, cabecera `Authorization: <esquema> <credencial>`): el proveedor del esquema verifica la credencial; se busca el `Usuario` de esa identidad externa y, si no existe, **se crea** (y se vincula la identidad). Un usuario `deshabilitado` es rechazado como credencial inválida. **Cada login emite una sesión nueva**; nunca reutiliza otra. Una credencial que falla, un esquema sin proveedor o una cabecera mal formada responden 401.
- **Resolver la sesión** (`Authorization: sesion <id>`; en WebSocket también `?sesion=<id>`, porque un navegador no puede poner cabeceras en el *handshake*): la sesión existe, `expiraEn > ahora` (ISO 8601, comparación lexicográfica) y su usuario no está deshabilitado. Si no, no hay actor (401).
- **Alta de cuenta** (`POST /v1/registro`, proveedor `clave`: nick + contraseña): el nick se normaliza (recortado, en minúsculas) y es la clave de búsqueda y el `sujetoId`. Con `CODIGO_REGISTRO_BOTS` configurado y coincidente, la cuenta es **de bot** (`bot: true`) y se salta el código de invitación; si no, con `CODIGO_REGISTRO` configurado hace falta que coincida (si no, 403); sin él el registro está abierto. No devuelve sesión: el cliente hace login después. Nick repetido = 409; datos inválidos = 400.
- **La Facción del actor no se guarda en la membresía**: se deriva siempre de `Faccion.ciudadanosIds` en el momento de preguntar (una anexión o fusión mueve ciudadanos de Facción y una copia quedaría vieja).
- **`jugadorId`** de una membresía de jugador = el `id` del `Usuario`. El héroe del jugador es el que tiene `Heroe.jugadorId == jugadorId` (puede no existir todavía).

<!-- parametros:ACCESO -->
```json
{
  "duracionSesionMs": 43200000
}
```
<!-- /parametros -->

### 2.2 Los cinco roles técnicos

`RolTecnico = jugador | administrador_partida | moderador | administrador_global | servicio_npc`. **Son un eje distinto del de los cargos del juego** (Rey, Gobernador…): un administrador no hereda ningún cargo ni puede actuar como jugador por serlo.

> **En Rust, un solo rol de administración hasta el sistema 5** (decisión del autor, 2026-10-24): toda sesión `CLIENTE_ADMIN_WEB` ve lo mismo (el ámbito `ADMIN` entero) y da todas las órdenes de administración; `moderador` y `administrador_partida` no se distinguen hasta que el sistema 5 traiga las membresías.

`rolEnPartida(actor)`: **manda la `Membresia` vigente** de esa partida; solo si no hay ninguna cae a `administrador_global` (si el usuario figura como tal en la instancia); si tampoco, no tiene rol. (Un administrador global que **además** es jugador de esa partida actúa con el rol `jugador` en los comandos.)

`administrador_global` **no es una membresía**: es una propiedad de la instancia (variable de entorno `ADMINISTRADORES`, lista `proveedor:sujetoId`), no repartible por partida. Sin configurar, **nadie** lo es. `servicio_npc` es el actor interno `'sistema'` (`ACTOR_SISTEMA`): el tick y los mensajes del servidor de batalla; **no pasa por la autorización de cliente**.

| Rol | `puedeJugar` | `puedeAdministrar` | `puedeCrearPartida` | `puedeDescartarPartida` | `puedeGestionarMembresias` | Comandos de partida (matriz §5) |
|---|:-:|:-:|:-:|:-:|:-:|---|
| `jugador` | ✔ | – | – | – | – | los que la matriz le da |
| `administrador_partida` | – | ✔ | – | ✔ | ✔ | solo `cancelarBatalla` |
| `moderador` | – | ✔ | – | – | – | **ninguno** |
| `administrador_global` | – | ✔ | ✔ | ✔ | ✔ | solo `cancelarBatalla` |
| `servicio_npc` | – | – | – | – | – | no aplica (actor `'sistema'`) |

- `puedeJugar(actor)` = la **membresía vigente** tiene rol `jugador`. Tener acceso técnico no es tener personaje: un administrador sin membresía de jugador no puede usar `/jugador/*`.
- `puedeAdministrar(actor)` = `esAdministradorGlobal` ∨ membresía con rol `administrador_partida` o `moderador`.
- `puedeDescartarPartida` = `esAdministradorGlobal` ∨ rol `administrador_partida` (el moderador queda fuera: no regenera el mundo). `puedeGestionarMembresias`, igual.
- **Roles otorgables** por `POST /admin/partidas/:gameId/membresias`: `administrador_partida`, `moderador`. `jugador` solo se obtiene uniéndose (`/membresia`); `administrador_global` y `servicio_npc` no se reparten. Otorgar a quien ya tiene membresía (vigente o no) es 409: hay que revocar antes.
- Quien **crea o reabre** una partida (solo `administrador_global`) recibe una membresía `administrador_partida` sobre ella si no tenía ninguna.

### 2.3 Superficies (todas bajo `/v1`)

| Superficie | Ruta | Exige |
|---|---|---|
| Sin sesión | `GET /salud` (fuera de `/v1`), `GET /v1/balance`, `GET /v1/openapi.json` | nada |
| Sesión | `POST /sesiones`, `POST /registro` (código de invitación), `GET /sesiones/actual?gameId=` | credencial / sesión |
| Jugador | `GET /jugador/partidas` (lista las abiertas con lo del propio usuario), `POST /jugador/partidas/:g/membresia` | sesión |
| Jugador | `GET /jugador/partidas/:g` (proyección), `.../eventos?desde=`, `.../cronica`, `.../mapa/:mapaId`, `POST .../comandos`, `GET .../batallas/:b/asignacion`, WebSocket `.../tiempo-real` | `puedeJugar` (+ la partida abierta) |
| Administración | `GET /admin/partidas/:g` (estado completo), `.../eventos`, `.../exportar`, `.../mapa/:mapaId`, `POST .../reloj` (acelerar, pausar o fijar el factor del reloj de mundo; sustituye a `POST .../tick`), `POST .../comandos`, `GET .../auditoria`, `PUT .../playtest`, `GET .../carga`, `GET .../membresias`, WebSocket `.../tiempo-real` | `puedeAdministrar` |
| Administración | `POST` y `DELETE .../membresias[/:usuarioId]` | `puedeGestionarMembresias` |
| Instancia | `GET /admin/partidas`, `POST /admin/partidas` (crear / reabrir), `GET` y `PUT /admin/registro/codigo`, `GET /admin/metricas`, `/admin/respaldos*` | `administrador_global` |
| Instancia | `POST /admin/partidas` con `forzar`, `DELETE /admin/partidas/:g` | `administrador_global` ∧ `puedeDescartarPartida` |
| Servidor de batalla | `/batallas/*` | cabecera con un token de servidor **declarado** (ni admin ni jugador) |

Orden de las comprobaciones en cada ruta de partida: **sesión (401) → rol de la superficie (403) → partida abierta en este proceso (404)**. En administración se comprueba el rol antes de la existencia de la partida para no revelarla a quien no puede verla. Un `GET /jugador/partidas/:g/batallas/:b/asignacion` exige además combatir en esa batalla (403) y que ya exista su token (404).

## 3. El camino de un comando

```
POST /jugador/partidas/:g/comandos   |   POST /admin/partidas/:g/comandos          { tipo, params, idempotencyKey? }

 1. sesión → membresía → puedeJugar / puedeAdministrar                              401 / 403 / 404
 2. validación del cuerpo contra `ESQUEMAS_PARAMS[tipo]` (un `oneOf` cerrado por comando; `additionalProperties: false`)
                                                                                    400 (y queda auditado: causa 'esquema')
 3. el ACTOR se arma desde la sesión, nunca desde el cuerpo:
      /jugador: { rol: 'jugador', heroeId: <su héroe o null> }
                si tipo == crearHeroe y la cuenta es de bot → params.controlador = 'bot' (lo decide la cuenta, no el cliente)
      /admin:   { rol: rolEnPartida(actor), heroeId: <su héroe, si tiene membresía de jugador> }
 4. verificarAutorizacion(tipo, params, estado, actor)                              403 { error: 'no autorizado (<motivo>)' }  (§4)
 5. runner.ejecutar(tipo, params, actorId, idempotencyKey)
      – entra en la COLA SERIAL de la partida (el núcleo de un solo hilo; los eventos programados comparten esa cola)
      – idempotencia: la clave es  actor ':' idempotencyKey ; un segundo intento (en curso o ya resuelto) devuelve LA MISMA promesa
        sin volver a aplicar el comando. No se compara que el comando coincida. Recuerda las últimas `limiteIdempotencia` (500) claves. *(En Rust, las últimas `limiteIdempotenciaPorCuenta` (64) de cada cuenta: Reloj_Y_Eventos §9.)*
 6. GameSession.ejecutar → candado de batalla → manejador del comando                (Tick §3)
      – el candado rechaza con `batalla.bloqueado` todo comando que toque algo en una batalla activa, salvo `unirseABatalla` y `cancelarBatalla`
 7. se difunden los eventos del comando por los canales de eventos (§7.2)
 8. respuesta 200 { gameId, instante, version, mapaId, resultado, [proyeccion] }   (`proyeccion` solo en /jugador: la del propio jugador ya actualizada)
```

- **Un rechazo del dominio NO es un error HTTP**: responde 200 con `resultado.ok == false`, `codigoError` (catálogo cerrado de `session/comandos/codigosDeError.ts`) y, si el motor da la causa concreta, `detalleError`. Un cliente decide por el código, nunca parseando el texto. Solo un fallo de persistencia (el comando se aplicó en memoria y no se guardó) responde 409.
- **Tipo desconocido**: 400. La validación de forma del paso 2 ya lo cubre en HTTP; `ejecutarComandoHttp` repite la comprobación como red de seguridad.
- **Actor en el motor** (`actorId`): el `id` del héroe; sin héroe todavía, el `jugadorId` que va a crearlo; en administración sin personaje, `admin:<usuarioId>`.
- **Auditoría**: cada petición deja **una** línea en una de cuatro salidas: tipo inválido o forma inválida (`esquema`), no autorizado (`autorizacion`, con el motivo), ejecutado (aceptado, o rechazado por el dominio con `causa: 'dominio'` y su `codigoError`) y fallo de persistencia. Con `instante`, `version` y milisegundos. Un administrador queda distinguible de un jugador por su `actor`.

## 4. La autorización de un comando

### 4.1 Algoritmo (`verificarAutorizacion`)

```
verificarAutorizacion(tipo, params, estado, actor):
  entrada = MATRIZ_AUTORIZACION[tipo]
  si actor.rol ∉ entrada.rolesPermitidos             → denegado 'rol_insuficiente'
  si actor.rol == 'jugador':
      si actor.heroeId == null                       → (tipo == 'crearHeroe') ? autorizado : denegado 'condicion_dominio'
      si tipo ≠ 'conectarse' ∧ héroe(actor).fuera    → denegado 'fuera_del_mundo'
      si entrada.condicionJugador ∧ ¬condicionJugador(estado, heroeId, params)
                                                     → denegado 'condicion_dominio'
  autorizado
```

- **La condición de dominio solo se evalúa si el actor entra con el rol `jugador`.** Un administrador que pasa el filtro de rol (solo `cancelarBatalla`) no está sujeto a ella: para él el rol ya es autoridad completa.
- **«Fuera del mundo»** (`Heroe.fuera`, el héroe desconectado, Presencia §7) bloquea **todo** menos `conectarse`.
- **Exhaustividad**: la matriz es un tipo indexado `{ [T in TipoComando]: … }`: un comando nuevo sin fila no compila, y `autorizacionComandos.test.ts` compara las claves con el registro real.
- **Lo que la matriz NO decide**: la autorización solo responde «¿tiene este actor, por su relación de juego con la entidad objetivo, derecho a *intentarlo*?». Las demás condiciones (hay oro, está a la puerta, la tropa cabe…) las comprueba el comando y las rechaza con su código de dominio.

### 4.2 Fail-open: una entidad que no existe pasa la matriz

Casi todos los resolutores devuelven «autorizado» cuando la entidad referenciada **no existe** (asentamiento, Facción, caravana, ejército, acuerdo, propuesta, relación, batalla, escuadra): el comando la rechaza después con su código (`asentamiento.no_existe`…), que dice mucho más que un «no tienes permiso» engañoso. Ninguna acción no autorizada llega a aplicarse, porque el comando no puede actuar sobre lo que no existe. **Excepción: `responderSolicitud` es fail-closed** (una Facción inexistente deniega: `faccion.reyId === héroe` es falso).

### 4.3 Predicados

Todos son funciones del estado y del héroe del actor (`yo`). Los de pertenencia viven en `engine/pertenencia.ts` y `engine/faccion.ts` (única definición); la matriz solo los combina.

| Predicado | Definición |
|---|---|
| `ciudadano(F)` | `yo ∈ F.ciudadanosIds` |
| `rey(F)` | `F.reyId == yo` |
| `reyOEmbajador(F)` | `F.reyId == yo ∨ F.embajadorId == yo` |
| `residente(A)` | `yo ∈ A.heroesFundadoresIds ∨ yo ∈ A.casasCompradas` |
| `presente(A)` | `estaEnAsentamiento`: la ubicación del héroe (la registrada, o `ubicacionDeducida` si no hay) es `asentamiento` de **esa** plaza |
| `reside(A)` | `residente(A) ∧ presente(A)` — **si A no existe, verdadero** |
| `cargo(A, c)` | `reside(A) ∧ A.cargos[c] == yo` — **si A no existe, verdadero**. Pregunta si *tú* eres el titular, no si el puesto está cubierto |
| `autoridadDiplomatica(F)` | `ciudadano(F) ∧ reyOEmbajador(F)` (F inexistente: verdadero) |
| `reyDeFaccion(F)` | `ciudadano(F) ∧ rey(F)` (F inexistente: verdadero) |
| `reyDelAsentamiento(A)` | `reyDeFaccion(A.faccionId)` — **no exige residir ni estar presente**: designar Gobernador es un acto de nivel Facción, como designar Embajador |
| `reyDeLaPlaza(A)` | lo mismo, con la Facción dueña de `A` |
| `resideEnOrigen(caravana)` | `reside(caravana.origenAsentamientoId)` |
| `puedeOperarAparcada(caravana, A)` | `residente(origen de la caravana) ∧ presente(A)`: es suya (origen) y está **donde está aparcada**, no en el origen del que marchó |
| `puedeReclutarEnPlaza(A)` | `puedeReclutarEn(A, yo, mi Facción) ≠ 'no' ∧ presente(A)` (`'todo'` si resides; `'solo_reponer'` si eres de la Facción dueña y no estás vetado; si no, `'no'`) — detalle en [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md) |
| `puedeMoverTropaDe(A)` | `residente(A) ∧ presente(A)` (A inexistente: verdadero) |
| `escuadrasPropias(ids)` | cada id que **existe** es una escuadra de `yo` (un id inexistente pasa) |
| `enEjercito(e)` | `yo ∈ e.participantes` ∨ `yo` tiene una escuadra con contenedor `ejercito` en `e` (un viajero solo también va dentro, Doc 5.12.1). Ejército inexistente: verdadero |
| `dirigeAedas(A)` | `presente(A)` ∧ (`rey(F de A)` ∨ `A.cargos.gobernador == yo` ∨ `A.cargos.sacerdote == yo`). No exige residir |
| `compraIntel(origen)` | origen = campamento: verdadero (el comando mira que esté dentro); origen = plaza: `presente(A)` ∧ (`reyOEmbajador(F de A)` ∨ `A.cargos.gobernador == yo`) |
| `yo` | `heroeId (actor) == params.heroeId`: nadie actúa a nombre de otro |

## 5. La matriz de autorización

### 5.1 En Rust: quién da cada orden hoy (4H T7, 2026-10-24)

Las **145 órdenes del contrato 05** (`orders.proto`), la orden genérica de administración y los cinco mensajes del canal del servidor de batalla, con el comando del núcleo que da cada una y quién puede darla. Es la autorización **provisional** hasta el sistema político (5), que traerá los cargos, la residencia como condición, los vetos y los roles de la membresía; las filas de §5.2 son lo que pedirá entonces.

> **Implementado en Rust (2026-10-24, 5B):** las nueve órdenes de la Facción, la ciudadanía y la residencia (128-136) y dos clases nuevas de `Autoriza`: `ReyDeLaFaccion` (`responder_solicitud`, sin excepción, como `responderSolicitudes` de §5.2; `asignar_embajador`) y `CiudadanoDeLaFaccion` (`asignar_rey`: un ciudadano si no hay Rey; con Rey, solo él). Las comprueba el predicado del crate `politica` (`autorizacion::Actor`) sobre el núcleo (`Vista`, sin cargos hasta 5E). El resto de la matriz no cambia hasta 5E.

> **Implementado en Rust (2026-10-24, 5C):** `designar_capital` (137, `ReyDeLaFaccion`) y `renombrar_asentamiento` (143, clase nueva `ResidenteDeLaPlaza`: la cuenta del héroe, que reside en la plaza; basta residir, como el TypeScript). Con 5G-a, quedan 27 órdenes no disponibles.

- **Quién la da** es `Comando::autoriza` (`crates/nucleo/src/comando.rs`, un `match` sin comodín sobre los 136 comandos: un comando nuevo no compila hasta decidirlo). Cada orden comprueba la suya en su sitio y rechaza con `no_autorizado`; la de administración la comprueba una sola vez `Nucleo::ejecutar`. La administración (`Peticion.admin`) puede con todas salvo las del servidor de batalla, que exigen su credencial (`batalla-servidor:<id>`, `batalla.sin_credencial`).
- **La sesión** añade dos filtros antes del núcleo (`sesiones::ordenes::traducir`, `sesiones.rs`): la orden genérica de administración y `controlar_reloj` solo los da una sesión `CLIENTE_ADMIN_WEB` (`no_autorizado` en cualquier otra), y los mensajes del canal de batalla solo llegan de una sesión `CLIENTE_SERVIDOR_BATALLA`. El actor de la petición es siempre la cuenta de la sesión, nunca un dato del cuerpo: las órdenes que no nombran héroe usan el de la cuenta.
- **«Además»** es lo que la orden exige después de autorizarla, con su rechazo de dominio (el catálogo de códigos está en `Docs/Coordinacion/07_Catalogo_de_Rechazos.md`). Un héroe fuera del mundo solo puede `conectarse` (`heroe.fuera_del_mundo`), salvo `cancelar_convocatoria`. Una entidad que no existe se rechaza con su código de «desconocida», antes de mirar quién la manda (como el fail-open de §4.2).
- **La prueba** `la_matriz_de_autorizacion_del_documento_es_la_del_codigo` (`crates/nucleo/src/sesiones/ordenes/pruebas.rs`) lee este bloque y lo compara con el código: falla si falta una orden, sobra una o no coincide su comando o quién la da. La columna «Además» no se compara.

<!-- autorizacion -->

**Quien manda en la plaza**

| Orden del contrato | Comando | Quién la da | Además (lo comprueba la orden, con su código) |
|---|---|---|---|
| `abandonar_recinto` | `AbandonarRecinto` | quien manda en la plaza | de la plaza |
| `aceptar_trueque` | `AceptarTrueque` | quien manda en la plaza | de la plaza B del acuerdo |
| `activar_politica` | `ActivarPolitica` | quien manda en la plaza | de la plaza; el cargo se da por ocupado |
| `agregar_carro_caravana` | `AnadirCarro` | quien manda en la plaza | del origen de la caravana |
| `alternar_auto_construccion` | `AlternarAutoConstruccion` | quien manda en la plaza | de la plaza |
| `alternar_receta` | `AlternarReceta` | quien manda en la plaza | de la plaza |
| `anadir_edificio_manualmente` | `AnadirEdificio` | quien manda en la plaza | de la plaza; el cargo que se invoca se da por ocupado (sistema 5) |
| `calibrar_reserva_manual` | `CalibrarReserva` | quien manda en la plaza | de la plaza |
| `cancelar_caravana` | `CancelarCaravana` | quien manda en la plaza | del origen de la caravana |
| `cancelar_orden_mercado` | `CancelarOrdenMercado` | quien manda en la plaza | de la plaza de la orden |
| `colocar_orden_mercado` | `ColocarOrdenMercado` | quien manda en la plaza | de la plaza |
| `comprar_animal_caravana` | `AnadirAnimal` | quien manda en la plaza | del origen de la caravana |
| `comprometer_recinto` | `ComprometerRecinto` | quien manda en la plaza | de la plaza; el cargo se da por ocupado |
| `crear_caravana` | `CrearCaravana` | quien manda en la plaza | de la plaza |
| `destruir_pueblo` | `DestruirPueblo` | quien manda en la plaza | de la plaza madre |
| `enviar_caravana_al_origen` | `EnviarCaravanaAlOrigen` | quien manda en la plaza | del origen de la caravana |
| `fijar_puerta` | `FijarAcceso` | quien manda en la plaza | de la plaza; no durante un asedio (`batalla.bloqueado`) |
| `marcar_pueblo` | `MarcarPueblo` | quien manda en la plaza | de la plaza madre; la columna del héroe a la puerta (`radioPuerta`) del nodo |
| `mejorar_edificio_ahora` | `MejorarEdificio` | quien manda en la plaza | de la plaza |
| `mejorar_recinto` | `MejorarRecinto` | quien manda en la plaza | de la plaza; el cargo se da por ocupado |
| `mover_carga_caravana_aparcada` | `MoverCargaAparcada` | quien manda en la plaza | del origen de la caravana (DUDA: el documento pide además estar presente donde está aparcada) |
| `mover_carro_caravana` | `MoverCarro` | quien manda en la plaza | del origen de la caravana de la que sale |
| `mover_en_cola` | `MoverEnCola` | quien manda en la plaza | de la plaza |
| `preparar_caravana` | `PrepararCaravana` | quien manda en la plaza | del origen de la caravana |
| `proponer_trueque` | `ProponerTrueque` | quien manda en la plaza | de la plaza A |
| `quitar_de_cola` | `QuitarDeCola` | quien manda en la plaza | de la plaza |
| `rechazar_trueque` | `RechazarTrueque` | quien manda en la plaza | de la plaza B del acuerdo |
| `reponer_carro_de_acarreo` | `ReponerCarro` | quien manda en la plaza | de la plaza madre |
| `reservar_caravana` | `ReservarCaravana` | quien manda en la plaza | del origen de la caravana |
| `solicitar_ascenso` | `SolicitarAscenso` | quien manda en la plaza | de la plaza |

**La columna**

| Orden del contrato | Comando | Quién la da | Además (lo comprueba la orden, con su código) |
|---|---|---|---|
| `adjuntar_caravana` | `AdjuntarCaravana` | un participante de la columna | a la puerta de la caravana |
| `aparcar_caravana` | `AparcarCaravana` | un participante de la columna | la caravana adjunta, con la columna dentro de la plaza (`caravana.soltar_solo_dentro`, como `soltar_caravana`) |
| `cargar_caravana` | `CargarCaravana` | un participante de la columna | la caravana adjunta (`caravana.no_esta_adjunta`), a la puerta (`radioPuerta`) de una plaza de su Facción (`caravana.plaza_ajena`); hasta lo que le cabe (`caravana.sin_capacidad`) |
| `comerciar_en_plaza` | `ComerciarEnPlaza` | el Líder de la columna | a la puerta de la plaza de la orden |
| `depositar_en_avanzada` | `DepositarEnAvanzada` | un participante de la columna | a la puerta de la avanzada operativa |
| `entregar_de_caravana` | `EntregarDeCaravana` | un participante de la columna | la caravana adjunta, a la puerta de la plaza B |
| `entregar_en_obra` | `EntregarMateriales` | un participante de la columna | a la puerta de la obra del pueblo |
| `entregar_en_obra` | `EntregarMaterialesAAvanzada` | un participante de la columna | a la puerta de la obra de la avanzada |
| `estacionar_ejercito` | `Estacionar` | el Líder de la columna | como `marchar_a` |
| `fundar` | `Fundar` | el Líder de la columna | con la Caravana de Fundación (`fundacion.sin_caravana`) y Facción |
| `marchar_a` | `MarcharA` | el Líder de la columna | a bordo, no desde dentro de un lugar (`heroe.dentro_de_un_lugar`), no una formación (`columna.en_formacion`) |
| `soltar_caravana` | `SoltarCaravana` | un participante de la columna | el héroe va en ella; la caravana adjunta, con la columna dentro de una plaza de su Facción (`caravana.soltar_solo_dentro`): queda aparcada |

**La cuenta del héroe**

| Orden del contrato | Comando | Quién la da | Además (lo comprueba la orden, con su código) |
|---|---|---|---|
| `aportar_a_refundacion` | `AportarARefundacion` | la cuenta del héroe | en un campamento (dentro o con su columna a la puerta, `campamento.fuera_de_la_puerta`), ciudadano (`faccion.no_pertenece`) de una Facción sin plazas (`refundacion.con_plazas`); del almacén personal residiendo allí (`refundacion.no_resides`) o del carro siendo el Líder de su columna (`columna.solo_el_lider`) (5G-b) |
| `asignar_escolta` | `AsignarEscolta` | la cuenta del héroe | escuadras suyas en el campamento del origen de la caravana; cupo de escolta |
| `asignar_guarnicion` | `AsignarGuarnicion` | la cuenta del héroe | residente de la plaza y cupo de guarnición |
| `atacar` | `Atacar` | la cuenta del héroe | en guerra o con `declara_guerra` (`guerra.confirmar`); a la puerta del objetivo |
| `borrar_loadout` | `BorrarLoadout` | la cuenta del héroe | — |
| `cambiar_seleccion_de_convocatoria` | `CambiarSeleccionDeConvocatoria` | la cuenta del héroe | integrante |
| `cancelar_batalla` | `CancelarBatalla` | la cuenta del héroe | quien la inició, antes de que empiece la partida (`batalla.no_puede_cancelar`) |
| `cancelar_convocatoria` | `CancelarConvocatoria` | la cuenta del héroe, aunque esté fuera | el Líder de la convocatoria |
| `cancelar_formacion` | `CancelarFormacion` | la cuenta del héroe | el héroe es el Líder de su columna (`columna.solo_el_lider`) |
| `cancelar_obra` | `CancelarObraDeAvanzada` | la cuenta del héroe, ciudadano de la madre | una mejora o reparación en curso |
| `ceder_liderazgo` | `CederLiderazgo` | la cuenta del héroe | el Líder |
| `colocar_avanzada` | `ColocarAvanzada` | la cuenta del héroe, ciudadano de la madre | la columna del héroe a la puerta (`radioPuerta`) del sitio |
| `comprar_caravana_de_refundacion` | `ComprarCaravanaDeRefundacion` | la cuenta del héroe | en un campamento, ciudadano de una Facción sin plazas (`refundacion.con_plazas`) ni otra Caravana de Fundación (`refundacion.ya_tiene_caravana`); el fondo de su Facción cubre el coste (`refundacion.fondo_corto`) (5G-b) |
| `comprar_en_campamento` | `ComprarEnCampamento` | la cuenta del héroe | dentro del campamento o a su puerta; cupo diario |
| `comprar_informe_plaza` | `ComprarInformePlaza` | la cuenta del héroe | como `comprar_mirada`; conocer la plaza |
| `comprar_mirada` | `ComprarMirada` | la cuenta del héroe | ciudadano, dentro de una plaza de su Facción con Taberna o en un campamento (DUDA: el cargo es del sistema 5) |
| `conectarse` | `Conectarse` | la cuenta del héroe, aunque esté fuera | el único permitido fuera del mundo |
| `convocar_ejercito` | `ConvocarEjercito` | la cuenta del héroe | dentro de la plaza o del campamento |
| `crear_heroe` | `CrearHeroe` | su cuenta | una por cuenta (`heroe.ya_existe`); la administración crea los de otra cuenta y los bots |
| `dejar_de_perseguir` | `DejarDePerseguir` | la cuenta del héroe | el héroe es el Líder de su columna (`columna.solo_el_lider`) |
| `desarmar_caravana_fundacion` | `DesarmarCaravanaFundacion` | la cuenta del héroe | su titular (`fundacion.no_titular`), con la caravana suelta en su origen (`caravana.no_disponible`) (5G-b) |
| `desconectarse` | `Desconectarse` | la cuenta del héroe | — |
| `destruir_avanzada` | `DestruirAvanzada` | la cuenta del héroe, ciudadano de la madre | — |
| `elegir_tropa_de_avanzada` | `ElegirTropaDeAvanzada` | la cuenta del héroe, ciudadano de la madre | — |
| `entrar_en_asentamiento` | `EntrarEnAsentamiento` | la cuenta del héroe | su columna a la puerta (`radioPuerta`); el acceso de la plaza (`acceso.cerrado`) |
| `entrar_en_avanzada` | `EntrarEnAvanzada` | la cuenta del héroe | como `entrar_en_pueblo` |
| `entrar_en_campamento` | `EntrarEnCampamento` | la cuenta del héroe | el héroe es el Líder de su columna (`columna.solo_el_lider`) si no va solo; su columna a la puerta (`radioPuerta`) |
| `entrar_en_pueblo` | `EntrarEnPueblo` | la cuenta del héroe | el héroe es el Líder de su columna (`columna.solo_el_lider`) si no va solo; su columna a la puerta (`radioPuerta`); el acceso heredado |
| `guardar_loadout` | `GuardarLoadout` | la cuenta del héroe | — |
| `guardar_en_almacen_personal` | `GuardarEnAlmacenPersonal` | la cuenta del héroe | dentro de su residencia; o el Líder de su columna (`columna.solo_el_lider`) con ella a la puerta de su residencia (`almacen_personal.fuera_de_tu_residencia`) |
| `guarnecer` | `Guarnecer` | la cuenta del héroe | el héroe es el Líder de su columna (`columna.solo_el_lider`); su columna a la puerta (`radioPuerta`) |
| `guarnecer_avanzada` | `GuarnecerAvanzada` | la cuenta del héroe, ciudadano de la madre | su columna a la puerta (`radioPuerta`) de la avanzada |
| `inspeccionar` | `Inspeccionar` | la cuenta del héroe | el objetivo en el mapa (`inspeccion.a_cubierto`) y a su alcance |
| `lanzar_caravana_fundacion` | `LanzarCaravanaFundacion` | la cuenta del héroe | ciudadano de la Facción de la plaza (`fundacion.no_ciudadano`) que reside y está en ella (`fundacion.no_resides`, Proy §5.2.4); nivel operativo 2 (`fundacion.nivel_insuficiente`), cooldown de caravanas (`caravana.en_cooldown`), cap con las caravanas vivas (`fundacion.cap_alcanzado`) y el coste (`caravana.sin_recursos`) (5G-b) |
| `mejorar_avanzada` | `MejorarAvanzada` | la cuenta del héroe, ciudadano de la madre | — |
| `movilizar_ejercito` | `MovilizarEjercito` | la cuenta del héroe | como `salir_al_mundo` |
| `ordenar_escuadras` | `OrdenarEscuadras` | la cuenta del héroe | — |
| `organizar_ejercito` | `OrganizarEjercito` | la cuenta del héroe | el Líder |
| `partir_convocatoria` | `PartirConvocatoria` | la cuenta del héroe | el Líder de la convocatoria |
| `pasar_a_viveres` | `PasarAViveres` | la cuenta del héroe | el héroe es el Líder de su columna (`columna.solo_el_lider`) |
| `pedir_prestamo` | `PedirPrestamo` | la cuenta del héroe | dentro del campamento |
| `perseguir` | `Perseguir` | la cuenta del héroe | el héroe es el Líder de su columna (`columna.solo_el_lider`) |
| `quitar_escolta` | `QuitarEscolta` | la cuenta del héroe | las escuadras que él cedió |
| `reclamar_caravana` | `ReclamarCaravana` | la cuenta del héroe | a bordo de su columna, en mundo abierto, a `radioEncuentro` (`caravana.lejos`) de la Caravana de Fundación de su Facción (`caravana.faccion_ajena`) que vuelve sola (`caravana.no_reclamable`); su columna personal pasa a ser un ejército (5G-b) |
| `reclutar_en_campamento` | `ReclutarEnCampamento` | la cuenta del héroe | dentro del campamento o a su puerta |
| `reclutar_tropa` | `ReclutarTropa` | la cuenta del héroe | dentro de la plaza; residente (todo) o de la Facción dueña (solo reponer) |
| `reparar_avanzada` | `RepararAvanzada` | la cuenta del héroe, ciudadano de la madre | — |
| `repartir_puntos` | `RepartirPuntos` | la cuenta del héroe | — |
| `reponer_prestamo` | `ReponerPrestamo` | la cuenta del héroe | dentro del campamento |
| `repostar` | `Repostar` | la cuenta del héroe | el héroe es el Líder de su columna (`columna.solo_el_lider`); su columna a la puerta (`radioPuerta`) de la plaza (`repostar.sin_lugar`) |
| `repostar` | `RepostarEnAvanzada` | la cuenta del héroe | el héroe es el Líder de su columna (`columna.solo_el_lider`); su columna a la puerta (`radioPuerta`) de la avanzada |
| `repostar_en_campamento` | `RepostarEnCampamento` | la cuenta del héroe | a la puerta del campamento |
| `responder_peticion_de_convocatoria` | `ResponderPeticionDeConvocatoria` | la cuenta del héroe | el Líder de la convocatoria |
| `responder_peticion_de_union` | `ResponderPeticionDeUnion` | la cuenta del héroe | el Líder de la columna |
| `retirar_de_refundacion` | `RetirarDeRefundacion` | la cuenta del héroe | como `aportar_a_refundacion`, sin mirar las plazas; lo que aportó (`refundacion.sin_aporte`), hasta el hueco (5G-b) |
| `retirar_de_avanzada` | `RetirarDeAvanzada` | la cuenta del héroe, ciudadano de la madre | su columna a la puerta (`radioPuerta`) de la avanzada |
| `retirar_guarnicion` | `RetirarGuarnicion` | la cuenta del héroe | — |
| `sacar_del_almacen_personal` | `SacarDelAlmacenPersonal` | la cuenta del héroe | como `guardar_en_almacen_personal`; hasta lo que cabe en el carro (`carro.no_cabe`) |
| `salir_al_mundo` | `SalirAlMundo` | la cuenta del héroe | desde su residencia (`salida.solo_de_tu_residencia`); escuadras suyas |
| `salir_de_asentamiento` | `SalirDeAsentamiento` | la cuenta del héroe | estar dentro |
| `salir_de_avanzada` | `SalirDeAvanzada` | la cuenta del héroe | estar dentro |
| `salir_de_pueblo` | `SalirDePueblo` | la cuenta del héroe | estar dentro |
| `salir_del_campamento` | `SalirDelCampamento` | la cuenta del héroe | desde el campamento donde está |
| `separarse_de_convocatoria` | `SepararseDeConvocatoria` | la cuenta del héroe | integrante |
| `separarse_del_ejercito` | `SepararseDelEjercito` | la cuenta del héroe | participante |
| `unirse_a_batalla` | `UnirseABatalla` | la cuenta del héroe | en el sitio del bando (dentro para defender, fuera para atacar) |
| `unirse_a_convocatoria` | `UnirseAConvocatoria` | la cuenta del héroe | dentro del lugar de la convocatoria |
| `unirse_a_ejercito` | `UnirseAEjercito` | la cuenta del héroe | su columna a la puerta (`radioPuerta`) de la columna; la política de unión |
| `unirse_en_campo` | `UnirseEnCampo` | la cuenta del héroe | a la vista de la columna; la política de unión |
| `vaciar_carro` | `VaciarCarro` | la cuenta del héroe | dentro de su residencia |

**La Facción, la ciudadanía y la residencia (sistema 5, 5B)**

| Orden del contrato | Comando | Quién la da | Además (lo comprueba la orden, con su código) |
|---|---|---|---|
| `asignar_embajador` | `AsignarEmbajador` | la cuenta del héroe, Rey de la Facción | el designado es ciudadano (`faccion.no_ciudadano`); la Facción existe (`faccion.no_existe`) |
| `asignar_rey` | `AsignarRey` | la cuenta del héroe, ciudadano de la Facción (con Rey, solo él) | el designado es ciudadano (`faccion.no_ciudadano`) |
| `cambiar_residencia` | `CambiarResidencia` | la cuenta del héroe | una plaza de su Facción (`residencia.plaza_ajena`) donde aún no reside (`residencia.ya_reside`); sin cooldown (`residencia.cooldown`) ni ejército en preparación (`heroe.en_convocatoria`); ni la plaza de destino ni la suya están en batalla (`batalla.bloqueado`, 5B-10) |
| `crear_faccion` | `CrearFaccion` | la cuenta del héroe | nombre (`faccion.nombre_vacio`, `faccion.nombre_duplicado`), sigilo (`faccion.sigilo_invalido`, `faccion.sigilo_duplicado`), sin Facción (`faccion.ya_pertenece`) ni cooldown de creación (`faccion.cooldown_creacion`) |
| `dejar_faccion` | `DejarFaccion` | la cuenta del héroe | es ciudadano (`faccion.no_pertenece`); no va en un ejército ni en uno en preparación (`faccion.en_ejercito`, Q7); su plaza no está en batalla (`batalla.bloqueado`, 5B-10) |
| `dejar_residencia` | `DejarResidencia` | la cuenta del héroe | reside en una plaza (`residencia.sin_plaza`); sin cooldown ni ejército en preparación; su plaza no está en batalla (`batalla.bloqueado`, 5B-10) |
| `residir_en_campamento` | `ResidirEnCampamento` | la cuenta del héroe | el campamento existe y aún no reside en él (`residencia.ya_reside`); sin cooldown ni ejército en preparación; su plaza no está en batalla (`batalla.bloqueado`, 5B-10) |
| `responder_solicitud` | `ResponderSolicitud` | la cuenta del héroe, Rey de la Facción | sin excepción: si la Facción no existe, `faccion.no_existe`; el héroe lo pidió (`faccion.no_ha_pedido`) y no es ciudadano de otra; aceptar a quien va en un ejército, `faccion.en_ejercito` (5B-6) |
| `solicitar_ingreso` | `SolicitarIngreso` | la cuenta del héroe | la Facción existe y no es ciudadano de ninguna (`faccion.ya_pertenece`); no va en un ejército (`faccion.en_ejercito`, 5B-6); pedir dos veces no cambia nada |

**La capital y el nombre de la plaza (sistema 5, 5C)**

| Orden del contrato | Comando | Quién la da | Además (lo comprueba la orden, con su código) |
|---|---|---|---|
| `designar_capital` | `DesignarCapital` | la cuenta del héroe, Rey de la Facción | la Facción existe (`faccion.no_existe`); una plaza suya (`capital.plaza_ajena`) con Palacio activo (`capital.sin_palacio`) que no es ya la capital (`capital.ya_es_la_capital`); con la designada en pie, `CAPITAL.cooldownDias` (`capital.cooldown`) |
| `renombrar_asentamiento` | `RenombrarAsentamiento` | la cuenta del héroe, residente de la plaza | basta residir (§5.2.4); el nombre se recorta y vacío deja la plaza sin nombre |

**Administración y servidor de batalla**

| Orden del contrato | Comando | Quién la da | Además (lo comprueba la orden, con su código) |
|---|---|---|---|
| `administracion:agrandar_zona` | `AgrandarZona` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:colocar_bandidos` | `ColocarBandidos` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:colocar_heroe` | `ColocarHeroe` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:crear_ejercito` | `CrearEjercito` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:dar_almacen_personal` | `DarAlmacenPersonal` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:dar_caravana_fundacion` | `DarCaravanaFundacion` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:dar_escuadra` | `DarEscuadra` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:dar_recursos` | `DarRecursos` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:dar_suministro` | `DarSuministro` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:fijar_controlador` | `FijarControlador` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:fijar_faccion_de_heroe` | `FijarFaccionDeHeroe` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:fijar_nivel_asentamiento` | `FijarNivelAsentamiento` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:fijar_nivel_faccion` | `FijarNivelFaccion` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:fijar_relacion` | `FijarRelacion` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:fijar_residencia` | `FijarResidencia` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:fijar_tecnologias` | `FijarTecnologias` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:herir_heroe` | `HerirHeroe` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:reclamar_parcela` | `ReclamarParcela` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `administracion:soltar_parcela` | `SoltarParcela` | administración | andamiaje hasta los sistemas 5 y 6; la orden genérica solo desde una sesión `CLIENTE_ADMIN_WEB` |
| `controlar_reloj` | `Reloj` | administración | y solo desde una sesión `CLIENTE_ADMIN_WEB` (la sesión lo comprueba) |
| `servidor:asignacion` | `RegistrarAsignacion` | servidor de batalla | por el canal del servidor de batalla (`MensajeDeServidorDeBatalla`), con su credencial; responde con un `AcuseBatalla` |
| `servidor:inicio` | `ConfirmarInicio` | servidor de batalla | por el canal del servidor de batalla (`MensajeDeServidorDeBatalla`), con su credencial; responde con un `AcuseBatalla` |
| `servidor:resultado` | `AplicarResultado` | servidor de batalla | por el canal del servidor de batalla (`MensajeDeServidorDeBatalla`), con su credencial; responde con un `AcuseBatalla` |
| `servidor:salida` | `RegistrarSalida` | servidor de batalla | por el canal del servidor de batalla (`MensajeDeServidorDeBatalla`), con su credencial; responde con un `AcuseBatalla` |
| `servidor:tokens` | `RegistrarTokens` | servidor de batalla | por el canal del servidor de batalla (`MensajeDeServidorDeBatalla`), con su credencial; responde con un `AcuseBatalla` |

**Aún no disponibles (`orden.no_disponible`)**

| Orden del contrato | Comando | Quién la da | Además (lo comprueba la orden, con su código) |
|---|---|---|---|
| `abandonar_epica` | — | no disponible | sistema 6 (tecnología, Aedas y alijos) |
| `abrir_alijo` | — | no disponible | sistema 6 (tecnología, Aedas y alijos) |
| `admitir_otras_facciones` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `adoptar_tecnologia` | — | no disponible | sistema 6 (tecnología, Aedas y alijos) |
| `alternar_reabastecer_aliados` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `asignar_cargo_local` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `comprar_tecnologia_aeda` | — | no disponible | sistema 6 (tecnología, Aedas y alijos) |
| `declarar_guerra` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `empezar_epica` | — | no disponible | sistema 6 (tecnología, Aedas y alijos) |
| `proponer_anexion` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `proponer_fusion` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `proponer_paz` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `proponer_relacion` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `rebelion_vasallo` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `responder_anexion` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `responder_fusion` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `responder_relacion` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `retirar_anexion` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `retirar_fusion` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `retirar_relacion` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `romper_relacion` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |
| `vetar_jugador` | — | no disponible | sistema 5 (política, Facciones, cargos y residencia) |

<!-- /autorizacion -->

### 5.2 La matriz del TypeScript (lo que pedirá el sistema 5)

La autorización del motor TypeScript (`session/comandos/autorizacion.ts`, verificada contra `3d19d31`): con cargos, residencia y presencia. En Rust aún no existe (§5.1); queda aquí como la regla que el sistema 5 tiene que traer. Las secciones siguientes son sus filas.

> **Implementado en Rust (2026-10-24, 5A, solo los predicados):** `crates/politica::autorizacion` (`Actor` sobre una `Vista` de las plazas que dará el núcleo): `ciudadano`, `rey`, `rey_o_embajador`, `residente`, `presente`, `reside`, `cargo`, `algun_cargo`, `autoridad_diplomatica`, `rey_de_faccion`, `rey_de_la_plaza`, `responde_solicitudes` (fail-closed), `asigna_rey`, `asigna_cargo_local`, `fija_puerta`, `veta`, `mueve_tropa_de`, `opera_aparcada`, `dirige_aedas`, `compra_intel_en`, `compra_tecnologia_aeda`, `se_muda_a` y `ciudadano_o_sin_faccion`, con el fail-open de §4.2. La matriz de §5.1 no cambia hasta 5E, que retira `manda_en` y pasa cada orden a su fila de esta sección.

Todas las filas admiten únicamente el rol `jugador` (más `cancelarBatalla`, §5.2.9). La columna «Condición» es lo que se evalúa **además** de «héroe existente y dentro del mundo».

#### 5.2.1 Sin condición adicional (23)

El actor es siempre el héroe de la sesión: estos comandos no reciben un `heroeId` con el que suplantar. Quién puede de verdad lo decide el comando.

| Comando | Qué lo limita (en el comando, no aquí) | Documento |
|---|---|---|
| `crearHeroe` | único comando de quien aún no tiene héroe (`heroe.ya_existe` si ya lo tiene) | [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md) |
| `repartirPuntos`, `guardarLoadout`, `borrarLoadout`, `ordenarEscuadras`, `pasarAViveres` | siempre sobre el héroe del actor | [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md), [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) |
| `asignarGuarnicion`, `retirarGuarnicion` | residir donde guarnece y el cupo | [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md) |
| `guardarEnAlmacenPersonal`, `sacarDelAlmacenPersonal` | ser el Líder de su columna | [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md) |
| `crearFaccion`, `solicitarIngreso`, `dejarFaccion` | «1 jugador, 1 Facción» y el cooldown son reglas de negocio de la partida (qué transición es válida), no de autorización | [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) |
| `fundar` | ser el titular de la Caravana de Fundación que lleva (lo mira el comando) | [`Fundacion_Y_Refundacion.md`](Fundacion_Y_Refundacion.md) |
| `aportarARefundacion`, `retirarDeRefundacion`, `comprarCaravanaDeRefundacion` | dónde está, que su Facción no tenga plazas y el fondo | [`Fundacion_Y_Refundacion.md`](Fundacion_Y_Refundacion.md) |
| `reclutarEnCampamento`, `pedirPrestamo`, `reponerPrestamo`, `abrirAlijo`, `comprarEnCampamento`, `repostarEnCampamento` | residir en el campamento / estar dentro o a su puerta (`campamento.lejos`) y el pago | [`Campamentos_Mercenarios.md`](Campamentos_Mercenarios.md) |

#### 5.2.2 Solo «yo» (29)

Condición: `yo` (`params.heroeId == héroe del actor`). La geometría y el resto lo comprueba el comando.

| Comando | Documento |
|---|---|
| `conectarse` (el único permitido con el héroe fuera del mundo), `desconectarse`, `salirDeAsentamiento`, `entrarEnAsentamiento`, `entrarEnCampamento`, `salirDelCampamento`, `guarnecer`, `marcharA`, `residirEnCampamento`, `dejarResidencia` | [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md), [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) |
| `convocarEjercito`, `unirseAConvocatoria`, `responderPeticionDeConvocatoria`, `cambiarSeleccionDeConvocatoria`, `separarseDeConvocatoria`, `cancelarConvocatoria`, `partirConvocatoria` | [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md) §9 |
| `unirseEnCampo`, `responderPeticionDeUnion`, `separarseDelEjercito`, `cederLiderazgo`, `organizarEjercito`, `cancelarFormacion` | [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) |
| `inspeccionar`, `perseguir`, `dejarDePerseguir` | [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md), [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) |
| `atacar` | [`Combate.md`](Combate.md) |
| `comerciarEnPlaza` | [`Comercio_Caravanas.md`](Comercio_Caravanas.md): estar a la puerta con la columna lo comprueba el motor |
| `unirseABatalla` | [`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) |

#### 5.2.3 «Yo» y algo más (12)

| Comando | Condición |
|---|---|
| `cambiarResidencia` | `yo` ∧ (el destino no existe ∨ `ciudadano` de la Facción dueña del destino) |
| `reclutarTropa` | `yo` ∧ `puedeReclutarEnPlaza(params.asentamientoId)` |
| `salirAlMundo` | `yo` ∧ `reside(params.asentamientoId)` ∧ `escuadrasPropias(params.escuadronIds)` |
| `movilizarEjercito` | `yo` ∧ `puedeMoverTropaDe(params.asentamientoId)` ∧ `escuadrasPropias(params.escuadronIds)` |
| `unirseAEjercito` | `yo` ∧ `reside(params.asentamientoId)` ∧ `escuadrasPropias(params.escuadronIds)` |
| `prepararCaravana`, `asignarEscolta`, `quitarEscolta` | `yo` ∧ `resideEnOrigen(params.caravanaId)` (la escolta la cede cada residente con lo suyo) |
| `moverCargaCaravanaAparcada`, `enviarCaravanaAlOrigen` | `yo` ∧ `puedeOperarAparcada(params.caravanaId, params.asentamientoId)` |
| `fijarPuerta` | `yo` ∧ (`cargo(A, gobernador)` ∨ `reyDeLaPlaza(A)`) — el Rey **no** tiene que estar presente |
| `vetarJugador` | `yo` ∧ `cargo(A, gobernador)` — **el Rey no puede** (asimetría con `fijarPuerta`, ver [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md) §8) |

#### 5.2.4 Residente presente (8)

Condición: `reside(A)` — residente **y** dentro de la plaza. «La ciudadanía habilita, la presencia ejerce» (Doc 2.5): un Gobernador de campaña sigue siéndolo pero no gobierna desde el camino.

| Comando | Plaza `A` | Documento |
|---|---|---|
| `lanzarCaravanaFundacion` | `origenAsentamientoId` | [`Fundacion_Y_Refundacion.md`](Fundacion_Y_Refundacion.md) |
| `renombrarAsentamiento` | `asentamientoId` (basta residir: el canon deja abierto exigir un cargo) | [`Economia_Construccion.md`](Economia_Construccion.md) |
| `crearCaravana` | `asentamientoId` | [`Comercio_Caravanas.md`](Comercio_Caravanas.md) |
| `agregarCarroCaravana`, `comprarAnimalCaravana`, `reservarCaravana`, `cancelarCaravana` | `resideEnOrigen(caravanaId)` | [`Comercio_Caravanas.md`](Comercio_Caravanas.md) |
| `moverCarroCaravana` | `resideEnOrigen(desdeCaravanaId)` | [`Comercio_Caravanas.md`](Comercio_Caravanas.md) |

#### 5.2.5 Cargo local (18)

Condición: `cargo(A, c)`. En los comandos con `params.cargo`, **el cliente declara con qué cargo actúa** y la matriz comprueba que sea el titular de **ese** cargo; que el cargo declarado sirva para esa acción lo rechaza el comando.

| Comando | Cargo exigido |
|---|---|
| `activarPolitica`, `anadirEdificioManualmente`, `quitarDeCola`, `moverEnCola`, `mejorarEdificioAhora`, `comprometerRecinto`, `mejorarRecinto` | `params.cargo` |
| `abandonarRecinto`, `solicitarAscenso` | `gobernador` (sin `cargo` en `params`: decisión de gobierno, no de obra) |
| `alternarReabastecerAliados` | `gobernador` ∨ `tesorero` |
| `alternarAutoConstruccion`, `alternarReceta` | `gobernador` ∨ `maestroObras` |
| `calibrarReservaManual` | `tesorero` |
| `proponerTrueque` (en `asentamientoAId`), `colocarOrdenMercado` | `tesorero` ∨ `gobernador` (decidido el 2026-10-12: el canon da el comercio al Tesorero y el Gobernador manda en la plaza) |
| `aceptarTrueque`, `rechazarTrueque` | `tesorero` ∨ `gobernador` **del asentamiento `asentamientoBId` del acuerdo**: contesta el lado receptor, no quien propuso. Acuerdo inexistente: pasa |
| `asignarCargoLocal` | si `params.cargo == 'gobernador'`: `reyDelAsentamiento(A)` (sin presencia); si no: `cargo(A, gobernador)` |

Documentos: [`Economia_Construccion.md`](Economia_Construccion.md) (políticas, cola, mejoras, ascenso, auto-construcción, recetas, reserva), [`Comercio_Caravanas.md`](Comercio_Caravanas.md) (trueques y Mercado), [`Murallas.md`](Murallas.md), [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) (cargos), [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) (reabastecer aliados).

#### 5.2.6 Tecnología e intel (6)

| Comando | Condición | Documento |
|---|---|---|
| `adoptarTecnologia` | `rey(params.faccionId)` (Facción inexistente: verdadero). Que esté en la capital y pueda pagar, lo mira el motor | [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md) |
| `empezarEpica`, `abandonarEpica` | `dirigeAedas(params.asentamientoId)` | [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md) |
| `comprarTecnologiaAeda` | plaza y Facción existen ⇒ `presente(A)` ∧ (`rey(F)` ∨ `A.cargos.gobernador == yo`) | [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md) |
| `comprarMirada`, `comprarInformePlaza` | `compraIntel(params.origen)` | [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) §6 |

#### 5.2.7 Facción y diplomacia (19)

Todas con «F inexistente ⇒ pasa» salvo `responderSolicitud`.

| Comando | Condición |
|---|---|
| `responderSolicitud` | `F.reyId == yo` (**fail-closed**) — `D31`, `D46` |
| `asignarRey` | `ciudadano(F) ∧ (F.reyId == null ∨ rey(F))`: la primera asignación está abierta a cualquier ciudadano; una vez hay Rey, solo él traspasa el trono |
| `designarCapital`, `admitirOtrasFacciones`, `asignarEmbajador` | `reyDeFaccion(params.faccionId)` |
| `proponerRelacion`, `declararGuerra`, `proponerAnexion` | `autoridadDiplomatica(params.faccionAId)` (la que propone, declara o absorbe) |
| `proponerPaz` | `autoridadDiplomatica(params.faccionId)` (la paz la ofrece cualquiera de las dos en guerra) |
| `romperRelacion` | `autoridadDiplomatica(params.iniciadorFaccionId)` |
| `rebelionVasallo` | `autoridadDiplomatica(relación.faccionBId)`: se rebela el **vasallo**, no el señor |
| `responderRelacion` | `reyDeFaccion(propuesta.faccionBId)`: contesta **solo el Rey** de la otra Facción |
| `retirarRelacion` | `autoridadDiplomatica(propuesta.faccionAId)` |
| `responderAnexion` | `rey(propuesta.absorbidaId)` |
| `retirarAnexion` | `autoridadDiplomatica(propuesta.absorbenteId)` |
| `proponerFusion`, `retirarFusion` | `reyDeFaccion(faccionAId)` (en `retirarFusion`, la `faccionAId` de la propuesta) |
| `responderFusion` | `reyDeFaccion(propuesta.faccionBId)` |
| `desarmarCaravanaFundacion` | `ciudadano` de la Facción de la caravana (sin Facción derivable: pasa). Que además sea el titular lo mira el comando |

Documento: [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) (y [`Fundacion_Y_Refundacion.md`](Fundacion_Y_Refundacion.md) para la última).

#### 5.2.8 Dentro de un ejército (6)

Condición: `enEjercito(params.ejercitoId)`. **No exige ser el Líder**: cualquier participante puede estacionar (la cesión de mando de una coalición no existe en esta fase); que el comando exija además ser el Líder (cancelar la marcha, Doc 5.14.3) lo comprueba el motor.

`adjuntarCaravana`, `soltarCaravana`, `cargarCaravana`, `entregarDeCaravana`, `estacionarEjercito` — [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md).

**No se adopta en Rust** (contraste del sistema 5, C16): que cualquier participante estacione choca con la decisión de la 4B-2 (lo da el Líder, [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md)); queda lo de §5.1.

#### 5.2.9 Administración (1)

| Comando | Roles | Condición (solo si entra como `jugador`) |
|---|---|---|
| `cancelarBatalla` | `jugador`, `administrador_partida`, `administrador_global` (**no** `moderador`) | la batalla no existe ∨ `batalla.iniciadaPor == yo`. El administrador cancela cualquiera |

## 6. Qué ve cada audiencia

La proyección es una **frontera de seguridad**, no una capa de presentación: lo que no sale del servidor no se puede leer en un DevTools. Lo que decide qué secciones viajan es siempre el servidor.

### 6.1 Administración (`vistaAdminDeEstado`)

`GET /admin/partidas/:g` y los canales `admin/*` (§7.3). **Todo el estado, sin niebla ni redacción**, menos:
- `mapa` (viaja su identidad, `mapaId`; el mapa real se pide una vez por `GET …/mapa/:mapaId`) y `eventosDominio` (se piden con el cursor).
- Los **tokens de entrada a una batalla** (`batallas[].asignacion.tokensParticipante` se manda vacío): son de cada jugador, ni el administrador los ve.
- Se le suman los campos impuros que calcula el *runner*: `preciosReferencia`, `relojDeMundoIntervaloMs`, y la geometría de todos los asentamientos (`zonas`, `zonasFusionadas`, `trazadoPorAsentamiento`). `moderador` ve lo mismo que `administrador_partida`.

### 6.2 Jugador sin héroe

Si la membresía no tiene héroe, `GET /jugador/partidas/:g` y la respuesta de cualquier comando devuelven `{ gameId, instante, version, mapaId, sinHeroe: true, campamentos }`, con `campamentos = campamentosParaElegir`: de **cada** campamento de mercenarios, `{ id, posicion, origen, eligieronComoInicial, residentes (cuenta) }`. Es lo único que ve: dónde nacer.

### 6.3 Jugador con héroe: tres lecturas, una sola regla

`proyectarParaJugador(estado, heroeId, geometria)` se compone de dos mitades, y **los canales de tiempo real reparten esas mismas mitades**, de modo que la regla de qué se ve está escrita una vez y no puede separarse entre el `GET` y el empuje:

```
perspectiva = perspectivaDeMapa(estado, heroeId)           ← desde dónde se mira (Facción, columnas, explorado, aliados)
mapa        = proyectarMapa(estado, perspectiva, geometria) ← lo del mundo; igual para quien tenga la misma perspectiva.clave   → vista/mapa
{ajustes, dentroDe, personal} = proyectarPersonal(...)      ← lo de este héroe                                                   → vista/heroe
proyección  = componerProyeccion(mapa, heroeId, ajustes) + personal + { asentamientos: dentroDe ? [dentroDe] : [], trazadoPorAsentamiento }
              + (el runner añade) preciosReferencia, produccionDeAsentamiento*, ascensoDeAsentamiento*   (* solo si está dentro de una plaza)
```

**Perspectiva** (`PerspectivaDeMapa`):
- `faccionId` = la Facción de la que es ciudadano (o `null`).
- `ejercitosPropios` = columnas que `esColumnaDe`: **de su Facción**, **o** en las que va dentro (`participantes`), **o** en las que lleva una escuadra suya (un huérfano conserva las suyas). *Todos los miembros de una Facción ven enteras todas las columnas de la Facción.*
- `faccionesQueComparten` = Facciones con alianza o vasallaje `activa` con la propia (`compartenVision`).
- `explorado` = la memoria de la Facción fundida con `héroe.exploracionPersonal`.
- **`clave`** = `f:<facciónId>|<aliadas ordenadas, separadas por coma>` si es **compartible**, y `h:<heroeId>` si no. Es compartible si `faccionId ≠ null` ∧ todas sus columnas propias son de la Facción ∧ `explorado` es idéntico a la memoria de la Facción (no ha explorado por su cuenta más que ella). Dos héroes con la misma clave reciben exactamente el mismo mapa.

#### Cada campo, su capa y su regla

La regla fina de cada cosa del mundo (qué se ve, redactado a qué, con memoria o sin ella) está en [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) §5.4; aquí, el mapa completo de qué campo viaja por dónde.

| Campo | Capa | Quién lo recibe y qué | Forma |
|---|---|---|---|
| `mapaId`, `estadoMapa` | mapa | todos (el estado de los recursos del mapa) | escalar |
| `facciones` | mapa | **todas**, sin redactar: nombre, nivel, reputación, Rey/Embajador, ciudadanía, solicitudes, sigilo — público en la ficción | colección |
| `relaciones`, `titulos` | mapa | **todas**: la diplomacia y el ranking son públicos | colección |
| `asentamientosAvistados` | mapa | las plazas que se **ven ahora**, redactadas a ficha (`AsentamientoAvistado`); incluye las propias y la que pisa (que `componerProyeccion` quita para el que está dentro) | colección |
| `asentamientosConocidos` | mapa | la última foto de las vistas alguna vez (`FichaConocida` + `conocidoEn`), menos las que se ven ahora | colección |
| `ejercitosAvistados`, `caravanasAvistadas`, `aedasAvistados` | mapa | lo ajeno que se ve ahora, redactado | colección (móviles) |
| `heroesVisibles` | mapa (+ héroe) | solo la parte pública (`HeroePublico`) de quienes van en una columna propia o avistada | colección |
| `batallas` | mapa (+ héroe) | las que se ven; el bando propio (`ladoPropio`) lo añade la capa de héroe | colección |
| `campamentosBandidos` | mapa | los que se ven **ahora**, sin memoria | colección |
| `campamentosMercenarios` | mapa (+ héroe) | los explorados; la capa de héroe añade los suyos (casa y donde está dentro) aunque no los haya explorado | colección |
| `caminos` | mapa | tramos de la red con algún extremo explorado, fusionados en polilíneas | colección |
| `exploracion` | mapa | las dos máscaras (`celdas` exploradas, `visibles` ahora) que aplica el **cliente de jugador** | bloque |
| `territorioPorEjercito` | mapa | por cada columna **propia**, la Facción cuya zona contiene su posición (solo el `faccionId`, nunca qué plaza); se resuelve en el servidor porque el cliente solo tiene las zonas de lo que ve | mapa `id → valor` |
| `nombresDeDirigentes` | mapa | `heroeId → nombre` del Rey y del Embajador de **cada** Facción | mapa |
| `nombresDeCompaneros` | mapa | `heroeId → nombre` de cada ciudadano de la Facción propia (se le vea o no) **y de quienes piden entrar** | mapa |
| `zonas`, `zonasFusionadas` | mapa | **solo las de la Facción propia**: calcularlas exige las posiciones de todos los vecinos (entrada privilegiada) | colección |
| `faccionId`, `miConvocatoriaId`, `plazaPisadaId` | héroe | escalares propios | escalar |
| `heroe` | héroe | el héroe propio completo, **sin** `plazasRecordadas` ni `exploracionPersonal`, más lo derivado: `costeLiderazgo` por escuadra, `liderazgoTotal` por loadout, `cupoGuarnicion` y `guarnicionOcupada`, `capacidadAlmacenPersonal` (en Rust `capacidad_almacen_personal`, desde 5G-a), `residenciaId` | única |
| `ejercitos` | héroe | las columnas propias **completas** (+ `capacidadCarga` derivado) | colección (móviles) |
| `caravanas` | héroe | las propias **completas**: las de plazas propias, las de la Facción, y **las que tienen una plaza propia como destino** aunque sean ajenas; las comerciales de una plaza propia llevan `escoltaLiderazgo { usado, cupo }` | colección (móviles) |
| `acuerdos` | héroe | los trueques con alguna de las dos plazas propia | colección |
| `ordenes` | héroe | todas las de las plazas propias (más el historial de cumplidas), y de una plaza ajena **en cuya puerta tiene columna**, solo las `activa` (el escaparate, no la contabilidad) | colección |
| `informesDeInspeccion` | héroe | los suyos no caducados (`ahora < expiraEn`) | colección |
| `convocatorias` | héroe | las de **su lugar** (plaza o campamento donde está) y de **su Facción**, más la propia; las peticiones: el Líder ve todas, cada solicitante la suya | colección |
| `miradasIntel` | héroe | las de la Facción propia, abiertas o en cooldown (`libreEn > ahora`); las de los aliados no viajan, solo lo que ven | colección |
| `propuestasAnexion`, `propuestasRelacion`, `propuestasFusion` | héroe | las vigentes que hace o recibe la Facción propia | colección |
| `informesPlaza` | héroe | los de la memoria de la Facción (el último de cada plaza, con fecha) | colección |
| `tarifasIntel` | héroe | las tarifas y cupos (para cotizar sin copiar la regla) | bloque |
| `aedasResidentes` | héroe | los de las plazas propias, con su épica; los rivales no ven los suyos | colección |
| `alijos` | héroe | los a la vista de su columna, solo si puede buscarlos (Facción sin asentamientos, o sin Facción) | colección |
| `escenaCampamento`, `mercadoCampamento` | héroe | **solo dentro de un campamento de mercenarios**: la planta; los precios por unidad, el cupo que le queda hoy y (si reside) lo que se puede reclutar | bloque |
| `historial` | héroe | las líneas de su historial personal | colección |
| `tecnologia` | héroe | la Era, los logros del servidor (públicos, sin decir qué abren), y las tecnologías de la Facción propia (las de un rival no viajan nunca) | bloque |
| `detalleDeAvistados` | héroe | de las plazas avistadas, lo que solo él ve: su `almacen` (si es de su Facción y su columna está a la puerta) e `interiorRecordado` | mapa |
| `campamentosPropios`, `heroesVisiblesExtra`, `ladoPropioPorBatalla`, `batallasPropias` | héroe | los ajustes de §6.3: campamentos que conoce por vivir en ellos; héroes dentro de la plaza que pisa; su bando en cada batalla; las batallas en que combate aunque el mapa no las vea | varios |
| `preciosReferencia` | runner | el precio de referencia por recurso (necesita el almacén de todas las plazas) | bloque |
| `produccionDeAsentamiento`, `ascensoDeAsentamiento` | runner | **solo dentro de una plaza**, la que pisa | bloque |

#### Lo que solo viaja si estás dentro de una plaza de tu Facción

`asentamientos` es un array de **cero o un** elemento: el interior **completo** (`Asentamiento` entero: almacén, cola, cargos, políticas, edificios en obra, población…) de la plaza donde el héroe **está** (`ubicacion.tipo == 'asentamiento'`, la registrada o, si no hay, la deducida) **y** es de su Facción. Con él viaja el `trazadoPorAsentamiento` de las plazas propias. Dentro de una plaza **ajena** (aliada, neutral) no se ve ningún interior: la plaza aparece, si se ve, como ficha. El resto de plazas de la propia Facción, desde fuera, se ven como cualquier otra, más `cargos` y `politicasActivas` (público dentro de casa) y, con una columna a la puerta, el `almacen`.

#### Lo redactado: formas públicas

| Forma | Lleva | Deliberadamente NO lleva |
|---|---|---|
| `AsentamientoAvistado` | id, nombre, `faccionId`, posición, `nivel`, `zona` (polígono real, recortado contra todos los vecinos), `edificios` **solo `activo`**; si es propia: `cargos`, `politicasActivas`, y con columna a la puerta el `almacen`; `interiorRecordado` si el héroe la pisó | almacén (ajena), escuadras, edificios en obra, colas, cargos y políticas (ajena), trazado |
| `EjercitoAvistado` | id, `tipo`, `faccionId`, posición, `enFormacion?`, nº de `participantes` (los rombos), `heroeIds`, `teSigue?` (solo a quien es perseguido) | composición, `ruta`, `objetivo`, `estado`, origen, suministro |
| `CaravanaAvistada` | id, posición, `faccionId?` (por su plaza de origen), `escoltada`, `recursos` (nombres, **sin cantidades**, ordenados) | cantidades, ruta, destino |
| `HeroePublico` | id, nombre, clase, nivel, `heridoHasta?`, `escuadrasQueLleva` (tropa, cantidad, nivel) y equipo puesto | campamento, puntos, Liderazgo, si es humano o bot |
| `BatallaVisible` | id, estado, contexto, punto, por bando: Facción, nº de héroes y capacidad; `ladoPropio?` | tokens, escuadras, listas de héroes |
| `AedaAvistado` | id, posición, `enAsentamientoId?` | qué sabe |
| `ConvocatoriaProyectada` | lugar, Líder, política de unión, integrantes (escuadras con tropa, nivel, moral, cantidad; carga), peticiones, `soyLider`, `soyIntegrante` | – |

> **Fuga aceptada** (decisión del autor, 2026-09-05): la `zona` de una plaza vista está recortada contra *todos* sus vecinos, incluidos los que el jugador no ha visto; un lado plano en la frontera delata un tercero en esa dirección. Se prefiere «exacta con una pista de más» a «limpia pero falsa».

### 6.4 Eventos

- **Audiencia de un jugador** (`eventosVisiblesParaJugador`): le llegan los eventos **sin plaza** (`asentamientoId` ausente: globales), los de **una plaza de su Facción** y los que **nombran a su héroe** (`heroesNombrados`). *(En Rust, reglas del autor del 2026-10-24: un hecho sin plaza que nombra a alguien llega **solo a los nombrados y a sus Facciones**, no a todos —a todos, solo los públicos: la crónica y la reposición de los mercados—, y los de una plaza los oye su Facción y **quien esté dentro**, ciudadano o visitante; contrato 05 §8.)*
- **`heroesNombrados(evento)`** = la unión de `payload.heroeId`, `payload.liderId` (a quién se le pide algo), `payload.heroesIds` y los `heroesIds` de cada lado (`atacante`, `defensor`) de un informe de combate. Hace falta porque la atribución por plaza no cubre una columna salida de un campamento (`asentamientoId: ''`) ni un combate entre Facciones (el evento va a la plaza de un solo bando). La misma función filtra el cursor HTTP y el canal `heroe/<id>`: no pueden separarse.
- **Cursor**: `GET …/eventos?desde=<versión>` devuelve los de `version > desde`, ya filtrados. `desde` no entero o negativo = 400. Sin héroe: lista vacía. El historial **no viaja** en las lecturas de estado (solo crece y era casi toda la lectura): se pide una vez con `desde=0` y luego por cursor; los de los comandos propios ya vienen en `resultado.eventos`.
- **Crónica** (`GET …/cronica`): los eventos `esDeCronica` (la entrada derivada `cronica.entrada` y los públicos de la lista: logros, Eras, títulos, descubrimientos y épicas, **siempre sin plaza**) — **la misma para todos**. `limite` (200 por defecto, entre 1 y 1000) las más recientes, en orden cronológico. Sin `desde` mira solo los eventos en memoria; con `desde`, el historial completo.
- **Administración**: `GET …/eventos` sin filtro.

> **Implementado en Rust (2026-10-23, 4H T5).** Sin cursor ni canales: los hechos viajan en las `Delta` de la sesión (contrato 05 §8). Quién oye cada uno se decide al emitirlo (`EventoNucleo::completar`): los nombrados (`heroesNombrados` por las claves `heroe` y `lider`, más los participantes de las columnas que nombra), las Facciones dueñas de sus plazas y las que nombran sus datos; y en la sesión (`Mesa::oyentes`) cada sesión lo recibe **una vez**, por el primero de sus ámbitos que lo oye: `ASENTAMIENTO` (quien está dentro), `BATALLA` (quien combate), `HEROE` (el nombrado), `FACCION` y la `VISTA` de quien ve la batalla. Los globales son solo los públicos (`cronica.entrada`, `mercenarios.repuesto`): un hecho sin plaza que nombra a alguien ya no es de todos (no puede revelar lo que la niebla oculta). La administración, todos (`ADMIN`). La crónica es la de Tecnologia_Aedas §8, como hechos públicos en las mismas `Delta`.

## 7. Tiempo real

Una conexión WebSocket por cliente y superficie —`/jugador/partidas/:g/tiempo-real`, `/admin/partidas/:g/tiempo-real`— con canales lógicos encima. **La autenticación va ANTES de completar el *handshake*** (`preValidation`): sin sesión, 401; sin el permiso de la superficie, 403; con la partida cerrada, 404 — como respuesta HTTP a la petición de *upgrade*, no como un socket que se abre y se cierra. No hay mensaje de «conectado»: lo es el evento `open`. **Al reconectar se pierden las suscripciones**: el cliente se suscribe de nuevo.

Mensajes del cliente: `{ accion: 'suscribir' | 'desuscribir' | 'resincronizar', canal, ritmo? }`. Un mensaje que no es JSON o no tiene esa forma responde `error` con código `formato`.

### 7.1 Presencia por la conexión

Abrir el socket de jugador ejecuta `conectarse` para su héroe (si lo tiene); cerrar **el último** socket del jugador (varias pestañas cuentan) ejecuta `desconectarse`. Lo hace el servidor con el id del héroe como actor (no pasa por la matriz). Un fallo al anotarlo se registra y no tumba el socket: la presencia se corrige en la siguiente conexión. Apagar el servidor (un despliegue) **no** desconecta a nadie. Qué hacen ambos comandos: [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md) §7.

### 7.2 Canales de eventos (el *hub*)

Un evento por canal, serializado una vez por evento y canal, y solo si alguien lo escucha. No hay foto ni versión de canal: es un canal de hechos.

| Canal | Lleva | Quién puede suscribirse (`puedeSuscribirseA`) |
|---|---|---|
| `mapa/general` | eventos **sin** `asentamientoId` | cualquier jugador de la partida |
| `asentamiento/<id>` | eventos de **esa** plaza | `ciudadano` de la Facción dueña. Plaza inexistente = no autorizado (no se distingue de «ajena») |
| `batalla/<id>` | los `batalla.*` y `evento_pve.*` con ese `battleId` | `combateEn(batalla, héroe)` (también quien salió) ∨ la batalla está en `proyectarParaJugador(...).batallas` (la ve bajo la niebla). Batalla inexistente = no |
| `heroe/<id>` | lo que nombra al héroe y **no** va ya por `mapa/general` | solo ese héroe |
| `admin/eventos` | **todos** los eventos de la partida, sin filtro | `puedeAdministrar`, reevaluado al difundir |

- **Canales de un evento** (`canalesDeEvento`): `canalDeEvento` (general si no tiene plaza; si no, el de la plaza) + el de la batalla si es un `batalla.*`/`evento_pve.*` con `battleId` + el `heroe/<id>` de **cada héroe nombrado**, **salvo** que el evento sea global (ya les llega por el general). Más `admin/eventos`.
- **No se revalida una suscripción de jugador activa** (una anexión que cambie la Facción dueña de la plaza no la corta; el cliente se resuscribe al reconectar). **Sí** se revalida la de administración en cada difusión: si pierde el derecho, `revocado { motivo: 'sin_derecho' }` y fuera.
- Un canal de forma desconocida, o que no existe: `no_autorizado` (los de jugador) — no se revela si existe.

### 7.3 Canales de vista (foto y delta)

Siete canales, dos audiencias. Un canal de una audiencia **no existe** para la otra (`canal_desconocido`: ni se revela su nombre).

| Canal | Audiencia | Foto | `puedeVer` | Grupo (comparten foto, diff y serialización) |
|---|---|---|---|---|
| `admin/partida` | admin | la partida **ligera**: asentamientos sin `almacen`, `recintos` ni edificios no `mapa` (más `resumen`), héroes, campamentos de mercenarios sin `mercado`/`fondos`, ejércitos, caravanas, Facciones, relaciones, batallas, bandidos, Aedas, títulos, tecnología, red, acuerdos, órdenes, residentes, `preciosReferencia` | siempre | el propio canal |
| `admin/asentamiento/<id>` | admin | el asentamiento entero + sus edificios como colección | el asentamiento existe | el propio canal |
| `admin/campamento/<id>` | admin | el campamento (entidad única) | existe | el propio canal |
| `admin/heroe/<id>` | admin | `almacenPersonal` + historial con ids sintéticos | existe | el propio canal |
| `vista/mapa` | jugador | `MapaProyectado` de su perspectiva | tiene héroe | `vista/mapa|<perspectiva.clave>` |
| `vista/heroe` | jugador | su capa personal + ajustes + impuros | tiene héroe | `vista/heroe|<heroeId>` |
| `vista/asentamiento/<id>` | jugador | el interior: asentamiento (sin `edificios`), `edificios` como colección y `trazado` | ciudadano de la Facción dueña **y** pisándola (`ubicacion` = esa plaza) | el propio canal (la foto no depende de quien mira; el derecho se comprueba por suscriptor) |

Sin héroe (aún) todos los `vista/*` de jugador son `no_autorizado`. Si lo mirado deja de existir (plaza destruida, héroe borrado) el canal se **revoca** con `ya_no_existe`.

**Esquema de cada canal** (qué claves son colecciones con id, mapas, entidades únicas, escalares, bloques y cabecera): `admin/partida` — colecciones `asentamientos, heroes, campamentosMercenarios, ejercitos, caravanas, facciones, relaciones, batallas, campamentosBandidos, aedas, titulos (por tituloId), acuerdos, ordenes`; escalares `mapaId, estadoMapa`; móviles propios `ejercitos, caravanas`. `vista/mapa` — colecciones `ejercitosAvistados, caravanasAvistadas, asentamientosAvistados, asentamientosConocidos (por asentamientoId), aedasAvistados, batallas (por battleId), campamentosBandidos, campamentosMercenarios, heroesVisibles (por heroeId), facciones, zonas (por asentamientoId), zonasFusionadas (por faccionId), caminos, relaciones, titulos`; mapas `territorioPorEjercito, nombresDeCompaneros, nombresDeDirigentes`; escalares `mapaId, estadoMapa`; móviles **ajenos** `ejercitosAvistados, caravanasAvistadas`. `vista/heroe` — colecciones `ejercitos, caravanas, convocatorias, informesDeInspeccion (con id sintético `tipo:id` del objetivo), miradasIntel, informesPlaza (por asentamientoId), propuestas*, acuerdos, ordenes, aedasResidentes, historial (id sintético), alijos, campamentosPropios, heroesVisiblesExtra (por heroeId), batallasPropias (por battleId)`; mapas `detalleDeAvistados, ladoPropioPorBatalla`; única `heroe`; escalares `faccionId, miConvocatoriaId, plazaPisadaId`; móviles propios `ejercitos, caravanas`. Todo lo demás (`tecnologia`, `tarifasIntel`, `escenaCampamento`, `mercadoCampamento`, `preciosReferencia`, `produccionDeAsentamiento`, `ascensoDeAsentamiento`, `exploracion`) son **bloques**: se reemplazan enteros si cambian. En la cabecera de cada mensaje, no en `datos`: `instante`, `version`, `tick`.

> El `historial` de `admin/heroe` y de `vista/heroe` se identifica con un id **sintético y estable**: `momento#<hash djb2 del texto en base 36>.<n>`, con `n` = cuántas líneas idénticas hay antes. Así dos líneas iguales del mismo instante no chocan.

### 7.4 Foto, delta y su aplicación

Cada grupo guarda **una** foto (ya filtrada para su audiencia) y una `version` (= `estado.version`). Un cambio del estado la reproyecta **una vez** y calcula **un** diff, serializado **una vez** para todos los del grupo. **El delta es siempre el diff de dos fotos ya filtradas para ese grupo**: nada de lo que el grupo no podía ver entra en ninguna.

**Mensajes del servidor:**

| Mensaje | Cuándo |
|---|---|
| `suscrito { canal, intervaloTickMs, ritmo }` | tras suscribirse (el `ritmo` es el aplicado) |
| `foto { gameId, canal, version, instante, datos }` | al suscribirse, en `resincronizar`, tras recuperarse un socket lento, tras cambiar de grupo. `datos` sin `instante`/`version`/`tick` |
| `delta { gameId, canal, desde, hasta, instante, cambios, escalares }` | en cada volcado con algo que contar, **o** si avanzó el reloj (cada tick manda al menos el latido) |
| `resync { canal, motivo }` | seguida de una foto: motivos `visibilidad`, `cliente_lento`, `partida_reemplazada` |
| `revocado { canal, motivo }` | `sin_derecho` o `ya_no_existe` |
| `desuscrito { canal }` | tras desuscribir |
| `error { canal?, error, codigo }` | `canal_desconocido`, `no_autorizado`, `demasiado_frecuente`, `formato` |

**Diff** (`diferenciar(antes, despues, esquema)`), por cada clave del esquema:
- *Escalar*: viaja en `escalares` solo si cambia; quitarlo es ponerlo a `null`.
- *Entidad única*: `remove` si desaparece, `upsert` si aparece, y si cambia, un `patch` (o `upsert` si la entidad nueva tiene algún `null`).
- *Colección con clave* (array de entidades, `id` por defecto): `remove` de las que faltan, `upsert` de las nuevas, y para las que están en las dos: ninguna operación si es la **misma referencia**; si no, `patch` con las claves de primer nivel que cambian (las que desaparecen van a `null`). **Una entidad con algún `null` legítimo se manda entera (`upsert`)**, para no confundir un valor con un borrado.
- *Mapa* `id → valor`: `remove` de las claves que faltan, `upsert` de las nuevas o cambiadas.
- *Bloque*: `set` entero si cambia.
- **Orden de los cambios: primero todos los `remove`, luego `upsert`/`patch`** (así un id reutilizado no se borra tras reinsertarse). Las colecciones salen en el orden del esquema, las entidades en el de la foto nueva. Igualdad = estructural de JSON, cortada por identidad.

**Aplicar** (`aplicarDiferencia`, lo que hace el cliente): `foto + N deltas ≡ foto final`. Un `patch` sobre una entidad que no existe es un **hueco** y falla; `null` en un `patch` borra la clave.

**Movimiento** (`anotarMovimiento`): para las colecciones móviles, la `ruta` viaja una vez y luego solo `progreso` + `avancePorTick` (propios) o `posicionActual` + `desplazamientoPorTick` (ajenos, sin ruta). El avance **no sale de un cálculo nuevo**: es la diferencia con la foto anterior del mismo canal dividida por `ticksEntre` (= `round(Δinstante / duracionTickMs)`, mínimo 1; **0 si es el mismo instante**). Reglas: sin foto anterior no se anota; `estado == 'estacionado'` ⇒ `avancePorTick = 0`; con la **misma** `ruta` y `progreso` numérico en las dos fotos, el avance es la diferencia entre ellos; con otra ruta no se inventa (se espera al siguiente tick); **un comando entre dos ticks (mismo instante) hereda lo anotado en vez de poner cero**.

### 7.5 Cuándo y a quién se entrega

- **Coalescencia**: un cambio solo *programa* un volcado; como mucho **uno por partida cada `coalescerMs`**, y los cambios de dentro se juntan en un delta. Sin suscriptores no se proyecta nada.
- **Reevaluación del derecho en cada volcado**, antes de proyectar para nadie: quien pierde el permiso (`derecho()` del socket) o ya no `puedeVer` recibe `revocado { sin_derecho }` y sale del grupo.
- **Cambio de grupo**: si la clave de un suscriptor cambió (otra Facción, una alianza creada o rota, una columna que ya no es de la Facción, exploración propia) pasa a su grupo nuevo con `resync { visibilidad }` + la foto de ese grupo. **Nunca un delta entre grupos distintos.**
- **Partida reemplazada** (`forzar`: la versión del estado es *menor* que la del grupo): foto nueva a todos con `resync { partida_reemplazada }`.
- **Suscribir** a un grupo que ya existe lo vuelca primero, para que la foto que se da esté al día y la cadena `desde → hasta` no tenga hueco. Un `suscribir` repetido manda la foto otra vez pero no reinicia el reloj de `minMsEntreFotos` (que arranca en la primera alta).
- **Ritmo** (`ritmo` al suscribir): se recorta a un entero en `[1, ritmoMaximo]` (sin pedirlo o no finito: 1). Con `ritmo > 1` el suscriptor recibe **un delta coalescido cada `ritmo` ticks**, calculado desde lo último que se le entregó; dos de ritmo lento a la par comparten el mensaje.
- **Contrapresión**: si el búfer de un socket supera `umbralLentoBytes`, **no se le envía nada** (se marca desactualizado). Cuando baja de `umbralRecuperadoBytes` recibe `resync { cliente_lento }` + foto. Si sigue desactualizado más de `lentoMaxMs`, se cierra con el código **4408**.
- **`resincronizar`**: foto de nuevo, a lo sumo **una por canal y socket cada `minMsEntreFotos`** (si no, `demasiado_frecuente`); sobre un canal al que no se está suscrito, `no_autorizado`.
- **Tope** de `maxSuscripcionesPorSocket` canales por socket (`formato`).
- **Borrar la partida** cierra todos los sockets con **4404** y descarta sus grupos.

> **Implementado en Rust (2026-10-23, 4H T6).** Sin grupos ni diff de fotos: cada ámbito de cada sesión lleva su versión y el servidor reparte lo que dice el registro de cambios (contrato 05 §5-§7). Lo que aquí se garantiza con `aplicarDiferencia` («foto + N deltas ≡ foto final») lo prueba `crates/nucleo/tests/reanudacion.rs` en los siete ámbitos a la vez con mundos y órdenes aleatorios de semilla fija, y también que reanudar (todos los ámbitos a la vez, al día o con un hueco) deja lo mismo que una foto nueva. **Sin suscriptores no se proyecta nada**: sin ningún ámbito abierto el reparto no busca ni los oyentes de los hechos, y el ticket de una batalla solo se proyecta si hay un servidor de batalla que deba oírlo. La **coalescencia** es la del actor (un reparto por despertar, la vista también en cada muestreo de 250 ms) y la **contrapresión**, la de `servidor` (un cliente lento se descarta y reanuda). La niebla de la vista viaja como cambio parcial (los bytes que cambiaron de cada máscara) a quien tiene la anterior, y se calcula y se compara una vez por perspectiva. Con 500 sesiones (450 en el mapa con vista, 45 en su plaza, 5 de administración) repartir 5 minutos de mundo ocupa al actor 22 s (el 7 % de un núcleo en tiempo real; antes, 105-108 s) y cada orden 15 ms (antes, 54 ms), casi todo en la vista.

## 8. Parámetros

<!-- parametros:DIFUSION -->
```json
{
  "coalescerMs": 100,
  "umbralLentoBytes": 262144,
  "umbralRecuperadoBytes": 32768,
  "lentoMaxMs": 15000,
  "maxSuscripcionesPorSocket": 64,
  "minMsEntreFotos": 2000,
  "ritmoMaximo": 60
}
```
<!-- /parametros -->

Son cifras de **infraestructura** (`constantsInfra.ts`): no alteran lo que ve nadie, solo cuánto tarda en llegar. La duración de la sesión (`ACCESO.duracionSesionMs`, §2.1) y `INFRA.limiteIdempotencia` (§3, [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §8) son las otras dos. Las del juego que intervienen en lo que se ve (`VISION`, `INSPECCION`, `INTEL`, `ALMACEN_PERSONAL`…) están en [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) §7.

## 9. Invariantes

| Invariante | Test |
|---|---|
| La matriz tiene exactamente una fila por cada comando del registro, y este documento nombra todos los comandos del registro en §5 | `session/__tests__/autorizacionComandos.test.ts`, `mecanicas/__tests__/matrizDocumentada.test.ts` |
| Un rol no listado se rechaza sin evaluar la condición de dominio; un jugador sin héroe solo puede `crearHeroe` | `autorizacionComandos.test.ts` |
| La Facción del actor se deriva de `ciudadanosIds`, no de la membresía | `autorizacionComandos.test.ts` |
| Solo el Rey responde a las solicitudes de ingreso, designa la capital, decide la admisión de otras Facciones; `solicitarAscenso` es solo del Gobernador; `fijarPuerta`: Gobernador o Rey; `vetarJugador`: Gobernador; designar Gobernador es del Rey, los demás cargos del Gobernador vigente | `autorizacionComandos.test.ts` |
| El cargo se comprueba sobre el titular, no sobre si el puesto está cubierto | `autorizacionComandos.test.ts` |
| Nadie recluta (ni actúa) a nombre de otro | `autorizacionComandos.test.ts` |
| Trueques y órdenes de Mercado, solo el Tesorero o el Gobernador (residente y dentro); las caravanas, cualquier residente | `autorizacionComandos.test.ts` |
| `membresía` manda sobre `administrador_global`; `puedeJugar` solo con rol `jugador`; el moderador no descarta | `acceso/__tests__/rolesDePartida.test.ts` |
| Un canal de evento de plaza solo lo abre un ciudadano de su Facción; el personal, solo su héroe; la batalla, quien combate o la ve | `session/__tests__/canales.test.ts`, `canalDeBatalla.test.ts` |
| `foto + deltas ≡ foto final`, tick tras tick, y dos suscriptores del mismo grupo reciben el MISMO texto con UNA proyección | `server/difusion/__tests__/difusorDeVistas.test.ts`, `session/vistas/__tests__/diff.test.ts` |
| `vista/mapa` + los ajustes de `vista/heroe` ≡ la proyección HTTP, para cada jugador de cada Facción | `server/difusion/__tests__/mapaPorFaccion.test.ts`, `server/__tests__/tiempoRealVistas.test.ts` |
| Lo que una Facción no ve no está en su mapa, ni en la foto ni en los deltas; lo personal de un jugador no llega al mapa de su Facción | `mapaPorFaccion.test.ts` |
| Perder el derecho revoca el canal en el mismo volcado; salir de la plaza revoca `vista/asentamiento` | `difusorDeVistas.test.ts`, `tiempoRealVistas.test.ts` |
| Un jugador no ve los canales de administración (`canal_desconocido`); un administrador no puede nombrar `vista/*` | `tiempoRealVistas.test.ts`, `difusorDeVistas.test.ts` |
| Ningún evento del ciclo de batalla lleva tokens, escuadras ni listas de héroes | `canalDeBatalla.test.ts` |
| El avance por tick es la diferencia entre fotos; un comando entre ticks conserva lo anotado | `session/vistas/__tests__/movimiento.test.ts` |

## 10. Casos límite y decisiones que parecen raras

- **El administrador global que también juega** actúa con el rol `jugador` en los comandos (la membresía manda). Por `/admin/…/comandos` pasa entonces la condición de dominio de su héroe, no la de un administrador.
- **`moderador` puede controlar el reloj a mano** (`POST …/reloj` pide `puedeAdministrar`) y leer todo, pero **no ejecuta ningún comando** (ni siquiera `cancelarBatalla`).
- **La condición de `fijarPuerta` y `vetarJugador` difiere**: el Rey puede fijar la puerta de cualquier plaza de su Facción sin estar presente; no puede vetar.
- **`asignarCargoLocal` de Gobernador no pide presencia, el resto de cargos sí**: designar Gobernador es un acto de la Facción; los demás cargos los designa el Gobernador *desde la plaza*.
- **Un forastero con columna a la puerta de una plaza ajena ve su mostrador** (`ordenes` activas), pero no su interior.
- **Las caravanas ajenas con destino en una plaza propia viajan completas** en `caravanas` (con su contenido): es lo que llega a tu puerta.
- **`dirigeAedas` y `comprarTecnologiaAeda` no exigen residir**, solo estar dentro de la plaza con el cargo.
- **El mapa se agrupa por perspectiva, no por Facción**: un miembro que explora por su cuenta (o va en una columna que no es de la Facción) deja el grupo de su Facción y recibe su propia foto, con `resync { visibilidad }`.
- **El mapa de una Facción sin aliados y el de la misma Facción con una alianza nueva** son grupos distintos: crear o romper una alianza rehace los grupos con foto nueva.
- **La batalla en el mapa sustituye a sus participantes**: las columnas y caravanas que están en una batalla activa no viajan en `ejercitosAvistados`/`caravanasAvistadas`.
- **Un `patch` sobre una entidad ausente es un hueco y falla en el cliente** (`aplicarDiferencia` lanza): por eso el difusor nunca manda un delta entre grupos y siempre foto al cambiar de grupo.
- **Un id inexistente pasa la matriz** (§4.2): un `404` lógico (`*.no_existe`) llega del comando, no un 403.

## 11. Divergencias

Ninguna abierta. Las resueltas (2026-10-12): comerciar por la plaza es del Tesorero o del Gobernador (§5.2.5), y el rol `observador` («solo lectura») se retiró porque ninguna ruta lo admitía.

## Cambios de la reconstrucción (2026-10-13)

- **Sin tick.** Los canales de vista ya no mandan «progreso por tick» ni `avancePorTick`: lo propio viaja como **plan completo** (vértices con su instante) una vez, y un cambio de plan llega como plan nuevo que empieza donde está la columna; lo ajeno, solo el tramo dentro de la vista con horizonte corto (~30 s), renovado (Niebla §5.4). Las economías y obras viajan como `(cantidad en t₀, ritmo)` y «termina en t». Se conservan fotos + deltas por canal, agrupación por clave de visión, serialización única por grupo y `resync`/`revocado`. El `ritmo` de suscripción (§ de difusión) deja de existir: lo sustituye el reloj de mundo. Detalle: contrato 05 (fase 2).
- **Comandos retirados**: `replegarEjercito` (no hay vuelta a casa; cancelar = `estacionarEjercito`), el `POST …/tick`.
- **Comandos nuevos o ampliados** (parámetros por fijar en el contrato 05, ver [`Comandos_Parametros.md`](Comandos_Parametros.md)): `repostar` (el Líder, con el ejército en la puerta), `reclamarCaravana` (junto a una caravana suelta), `atacar` con `declaraGuerra: true` y los objetivos `pueblo` y `avanzada`, y los de las obras de mapa: marcar un pueblo, colocar una avanzada, entregar materiales en el sitio, cancelar o destruir una obra, mejorar y reparar una avanzada, elegir la tropa de serie de una avanzada, guarnecer una avanzada y reponer el carro de acarreo. Autorización: las obras de mapa las usan el Gobernador o el Maestro de Obras **presentes** en la plaza madre; entregar materiales, cualquier ciudadano de la Facción con la columna en el sitio.
- **Proyección**: la plaza ve solo su zona y las avanzadas ven 12 (Niebla §3); pueblos y avanzadas ajenos se recuerdan como una ciudad; la plaza deja de ver más allá de su frontera.
- **Predicado nuevo**: «en guerra o declarable» para `atacar` (toda agresión entre Facciones exige guerra o la declara).

