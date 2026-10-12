# Proyecciones y autorización

## 1. Alcance y fuentes

Cubre **quién puede hacer qué** y **qué ve cada uno**: los roles técnicos y las superficies del servidor, el camino de un comando desde la petición hasta el motor, los predicados de pertenencia en los que se apoya la autorización, **la matriz de los 122 comandos**, lo que el servidor proyecta a cada audiencia (administración, jugador, eventos) y cómo se reparte por los canales de tiempo real (foto + delta).

- **Verificado contra:** `3d19d31` (2026-10-12).
- **Fuentes:** `acceso/tipos.ts`, `acceso/rolesDePartida.ts`, `acceso/servicioAutenticacion.ts`, `engine/pertenencia.ts`, `session/comandos/autorizacion.ts`, `session/comandos/registro.ts`, `session/comandos/tipos.ts`, `session/proyecciones/jugador.ts`, `session/canales.ts`, `session/vistas/{segmentos,diff,movimiento}.ts`, `session/estado.ts` (`vistaAdminDeEstado`), `server/rutas/*.ts`, `server/difusion/{hub,difusorDeVistas}.ts`, `server/runnerDePartida.ts` (`ejecutar`).
- **Canon:** [`Docs/Game/2_Sistema_Politico_Facciones_y_Cargos.md`](../Game/2_Sistema_Politico_Facciones_y_Cargos.md) 2.2 (cargos) y 2.5 (la ciudadanía habilita, la presencia ejerce), [`Docs/Game/1_Sistema_de_Mundo_y_Territorio.md`](../Game/1_Sistema_de_Mundo_y_Territorio.md) 1.10 (el héroe está situado). El contrato con los clientes está en `Docs/Coordinacion/` (`02`, `03`); la arquitectura de identidad, en `Docs/Arquitectura/5_Contratos_Identidad_Permisos.md`.
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

### 2.2 Los seis roles técnicos

`RolTecnico = jugador | administrador_partida | moderador | administrador_global | observador | servicio_npc`. **Son un eje distinto del de los cargos del juego** (Rey, Gobernador…): un administrador no hereda ningún cargo ni puede actuar como jugador por serlo.

`rolEnPartida(actor)`: **manda la `Membresia` vigente** de esa partida; solo si no hay ninguna cae a `administrador_global` (si el usuario figura como tal en la instancia); si tampoco, no tiene rol. (Un administrador global que **además** es jugador de esa partida actúa con el rol `jugador` en los comandos.)

`administrador_global` **no es una membresía**: es una propiedad de la instancia (variable de entorno `ADMINISTRADORES`, lista `proveedor:sujetoId`), no repartible por partida. Sin configurar, **nadie** lo es. `servicio_npc` es el actor interno `'sistema'` (`ACTOR_SISTEMA`): el tick y los mensajes del servidor de batalla; **no pasa por la autorización de cliente**.

| Rol | `puedeJugar` | `puedeAdministrar` | `puedeCrearPartida` | `puedeDescartarPartida` | `puedeGestionarMembresias` | Comandos de partida (matriz §5) |
|---|:-:|:-:|:-:|:-:|:-:|---|
| `jugador` | ✔ | – | – | – | – | los que la matriz le da |
| `administrador_partida` | – | ✔ | – | ✔ | ✔ | solo `cancelarBatalla` |
| `moderador` | – | ✔ | – | – | – | **ninguno** |
| `administrador_global` | – | ✔ | ✔ | ✔ | ✔ | solo `cancelarBatalla` |
| `observador` | – | – | – | – | – | ninguno (ni lee nada: ver §11) |
| `servicio_npc` | – | – | – | – | – | no aplica (actor `'sistema'`) |

- `puedeJugar(actor)` = la **membresía vigente** tiene rol `jugador`. Tener acceso técnico no es tener personaje: un administrador sin membresía de jugador no puede usar `/jugador/*`.
- `puedeAdministrar(actor)` = `esAdministradorGlobal` ∨ membresía con rol `administrador_partida` o `moderador`.
- `puedeDescartarPartida` = `esAdministradorGlobal` ∨ rol `administrador_partida` (el moderador queda fuera: no regenera el mundo). `puedeGestionarMembresias`, igual.
- **Roles otorgables** por `POST /admin/partidas/:gameId/membresias`: `administrador_partida`, `moderador`, `observador`. `jugador` solo se obtiene uniéndose (`/membresia`); `administrador_global` y `servicio_npc` no se reparten. Otorgar a quien ya tiene membresía (vigente o no) es 409: hay que revocar antes.
- Quien **crea o reabre** una partida (solo `administrador_global`) recibe una membresía `administrador_partida` sobre ella si no tenía ninguna.

### 2.3 Superficies (todas bajo `/v1`)

| Superficie | Ruta | Exige |
|---|---|---|
| Sin sesión | `GET /salud` (fuera de `/v1`), `GET /v1/balance`, `GET /v1/openapi.json` | nada |
| Sesión | `POST /sesiones`, `POST /registro` (código de invitación), `GET /sesiones/actual?gameId=` | credencial / sesión |
| Jugador | `GET /jugador/partidas` (lista las abiertas con lo del propio usuario), `POST /jugador/partidas/:g/membresia` | sesión |
| Jugador | `GET /jugador/partidas/:g` (proyección), `.../eventos?desde=`, `.../cronica`, `.../mapa/:mapaId`, `POST .../comandos`, `GET .../batallas/:b/asignacion`, WebSocket `.../tiempo-real` | `puedeJugar` (+ la partida abierta) |
| Administración | `GET /admin/partidas/:g` (estado completo), `.../eventos`, `.../exportar`, `.../mapa/:mapaId`, `POST .../tick`, `POST .../comandos`, `GET .../auditoria`, `PUT .../playtest`, `GET .../carga`, `GET .../membresias`, WebSocket `.../tiempo-real` | `puedeAdministrar` |
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
      – entra en la COLA SERIAL de la partida (la misma que el tick)
      – idempotencia: la clave es  actor ':' idempotencyKey ; un segundo intento (en curso o ya resuelto) devuelve LA MISMA promesa
        sin volver a aplicar el comando. No se compara que el comando coincida. Recuerda las últimas `limiteIdempotencia` (500) claves.
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

Todas las filas admiten únicamente el rol `jugador` (más `cancelarBatalla`, §5.9). La columna «Condición» es lo que se evalúa **además** de «héroe existente y dentro del mundo».

### 5.1 Sin condición adicional (23)

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

### 5.2 Solo «yo» (29)

Condición: `yo` (`params.heroeId == héroe del actor`). La geometría y el resto lo comprueba el comando.

| Comando | Documento |
|---|---|
| `conectarse` (el único permitido con el héroe fuera del mundo), `desconectarse`, `salirDeAsentamiento`, `entrarEnAsentamiento`, `entrarEnCampamento`, `salirDelCampamento`, `guarnecer`, `marcharA`, `residirEnCampamento`, `dejarResidencia` | [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md), [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) |
| `convocarEjercito`, `unirseAConvocatoria`, `responderPeticionDeConvocatoria`, `cambiarSeleccionDeConvocatoria`, `separarseDeConvocatoria`, `cancelarConvocatoria`, `partirConvocatoria` | [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md) §9 |
| `unirseEnCampo`, `responderPeticionDeUnion`, `separarseDelEjercito`, `cederLiderazgo`, `organizarEjercito`, `cancelarFormacion` | [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) |
| `inspeccionar`, `perseguir`, `dejarDePerseguir` | [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md), [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) |
| `atacar` | [`Combate.md`](Combate.md) |
| `comerciarEnPlaza` | [`Comercio_Caravanas.md`](Comercio_Caravanas.md): estar a la puerta con la columna lo comprueba el motor |
| `unirseABatalla` | `Batalla_Unity_Ciclo` (pendiente) |

### 5.3 «Yo» y algo más (12)

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

### 5.4 Residente presente (12)

Condición: `reside(A)` — residente **y** dentro de la plaza. «La ciudadanía habilita, la presencia ejerce» (Doc 2.5): un Gobernador de campaña sigue siéndolo pero no gobierna desde el camino.

| Comando | Plaza `A` | Documento |
|---|---|---|
| `lanzarCaravanaFundacion` | `origenAsentamientoId` | [`Fundacion_Y_Refundacion.md`](Fundacion_Y_Refundacion.md) |
| `renombrarAsentamiento` | `asentamientoId` (basta residir: el canon deja abierto exigir un cargo) | [`Economia_Construccion.md`](Economia_Construccion.md) |
| `proponerTrueque` | `asentamientoAId` | [`Comercio_Caravanas.md`](Comercio_Caravanas.md) |
| `aceptarTrueque`, `rechazarTrueque` | `asentamientoBId` **del acuerdo**: contesta el lado receptor, no quien propuso | [`Comercio_Caravanas.md`](Comercio_Caravanas.md) |
| `colocarOrdenMercado`, `crearCaravana` | `asentamientoId` | [`Comercio_Caravanas.md`](Comercio_Caravanas.md) |
| `agregarCarroCaravana`, `comprarAnimalCaravana`, `reservarCaravana`, `cancelarCaravana` | `resideEnOrigen(caravanaId)` | [`Comercio_Caravanas.md`](Comercio_Caravanas.md) |
| `moverCarroCaravana` | `resideEnOrigen(desdeCaravanaId)` | [`Comercio_Caravanas.md`](Comercio_Caravanas.md) |

### 5.5 Cargo local (14)

Condición: `cargo(A, c)`. En los comandos con `params.cargo`, **el cliente declara con qué cargo actúa** y la matriz comprueba que sea el titular de **ese** cargo; que el cargo declarado sirva para esa acción lo rechaza el comando.

| Comando | Cargo exigido |
|---|---|
| `activarPolitica`, `anadirEdificioManualmente`, `quitarDeCola`, `moverEnCola`, `mejorarEdificioAhora`, `comprometerRecinto`, `mejorarRecinto` | `params.cargo` |
| `abandonarRecinto`, `solicitarAscenso` | `gobernador` (sin `cargo` en `params`: decisión de gobierno, no de obra) |
| `alternarReabastecerAliados` | `gobernador` ∨ `tesorero` |
| `alternarAutoConstruccion`, `alternarReceta` | `gobernador` ∨ `maestroObras` |
| `calibrarReservaManual` | `tesorero` |
| `asignarCargoLocal` | si `params.cargo == 'gobernador'`: `reyDelAsentamiento(A)` (sin presencia); si no: `cargo(A, gobernador)` |

Documentos: [`Economia_Construccion.md`](Economia_Construccion.md) (políticas, cola, mejoras, ascenso, auto-construcción, recetas, reserva), [`Murallas.md`](Murallas.md), [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) (cargos), [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) (reabastecer aliados).

### 5.6 Tecnología e intel (6)

| Comando | Condición | Documento |
|---|---|---|
| `adoptarTecnologia` | `rey(params.faccionId)` (Facción inexistente: verdadero). Que esté en la capital y pueda pagar, lo mira el motor | [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md) |
| `empezarEpica`, `abandonarEpica` | `dirigeAedas(params.asentamientoId)` | [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md) |
| `comprarTecnologiaAeda` | plaza y Facción existen ⇒ `presente(A)` ∧ (`rey(F)` ∨ `A.cargos.gobernador == yo`) | [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md) |
| `comprarMirada`, `comprarInformePlaza` | `compraIntel(params.origen)` | [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) §6 |

### 5.7 Facción y diplomacia (19)

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

### 5.8 Dentro de un ejército (6)

Condición: `enEjercito(params.ejercitoId)`. **No exige ser el Líder**: cualquier participante puede replegar o estacionar (la cesión de mando de una coalición no existe en esta fase); que el comando exija además ser el Líder (cancelar la marcha, Doc 5.14.3) lo comprueba el motor.

`adjuntarCaravana`, `soltarCaravana`, `cargarCaravana`, `entregarDeCaravana`, `replegarEjercito`, `estacionarEjercito` — [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md).

### 5.9 Administración (1)

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
| `heroe` | héroe | el héroe propio completo, **sin** `plazasRecordadas` ni `exploracionPersonal`, más lo derivado: `costeLiderazgo` por escuadra, `liderazgoTotal` por loadout, `cupoGuarnicion` y `guarnicionOcupada`, `capacidadAlmacenPersonal`, `residenciaId` | única |
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

- **Audiencia de un jugador** (`eventosVisiblesParaJugador`): le llegan los eventos **sin plaza** (`asentamientoId` ausente: globales), los de **una plaza de su Facción** y los que **nombran a su héroe** (`heroesNombrados`).
- **`heroesNombrados(evento)`** = la unión de `payload.heroeId`, `payload.liderId` (a quién se le pide algo), `payload.heroesIds` y los `heroesIds` de cada lado (`atacante`, `defensor`) de un informe de combate. Hace falta porque la atribución por plaza no cubre una columna salida de un campamento (`asentamientoId: ''`) ni un combate entre Facciones (el evento va a la plaza de un solo bando). La misma función filtra el cursor HTTP y el canal `heroe/<id>`: no pueden separarse.
- **Cursor**: `GET …/eventos?desde=<versión>` devuelve los de `version > desde`, ya filtrados. `desde` no entero o negativo = 400. Sin héroe: lista vacía. El historial **no viaja** en las lecturas de estado (solo crece y era casi toda la lectura): se pide una vez con `desde=0` y luego por cursor; los de los comandos propios ya vienen en `resultado.eventos`.
- **Crónica** (`GET …/cronica`): los eventos `esDeCronica` (la entrada derivada `cronica.entrada` y los públicos de la lista: logros, Eras, títulos, descubrimientos y épicas, **siempre sin plaza**) — **la misma para todos**. `limite` (200 por defecto, entre 1 y 1000) las más recientes, en orden cronológico. Sin `desde` mira solo los eventos en memoria; con `desde`, el historial completo.
- **Administración**: `GET …/eventos` sin filtro.

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

Son cifras de **infraestructura** (`constantsInfra.ts`): no alteran lo que ve nadie, solo cuánto tarda en llegar. La duración de la sesión (`ACCESO.duracionSesionMs`, §2.1) y `INFRA.limiteIdempotencia` (§3, [`Tick.md`](Tick.md) §8) son las otras dos. Las del juego que intervienen en lo que se ve (`VISION`, `INSPECCION`, `INTEL`, `ALMACEN_PERSONAL`…) están en [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) §7.

## 9. Invariantes

| Invariante | Test |
|---|---|
| La matriz tiene exactamente una fila por cada comando del registro, y este documento nombra todos los comandos del registro en §5 | `session/__tests__/autorizacionComandos.test.ts`, `mecanicas/__tests__/matrizDocumentada.test.ts` |
| Un rol no listado se rechaza sin evaluar la condición de dominio; un jugador sin héroe solo puede `crearHeroe` | `autorizacionComandos.test.ts` |
| La Facción del actor se deriva de `ciudadanosIds`, no de la membresía | `autorizacionComandos.test.ts` |
| Solo el Rey responde a las solicitudes de ingreso, designa la capital, decide la admisión de otras Facciones; `solicitarAscenso` es solo del Gobernador; `fijarPuerta`: Gobernador o Rey; `vetarJugador`: Gobernador; designar Gobernador es del Rey, los demás cargos del Gobernador vigente | `autorizacionComandos.test.ts` |
| El cargo se comprueba sobre el titular, no sobre si el puesto está cubierto | `autorizacionComandos.test.ts` |
| Nadie recluta (ni actúa) a nombre de otro | `autorizacionComandos.test.ts` |
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
- **`moderador` puede avanzar el tick a mano** (`POST …/tick` pide `puedeAdministrar`) y leer todo, pero **no ejecuta ningún comando** (ni siquiera `cancelarBatalla`).
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

Abiertas, a decidir por el autor (también en [`Divergencias.md`](Divergencias.md)):

- **Tesorero y trueque/Mercado.** El canon (Doc 2.2, glosario) define al Tesorero como «gestión económica (trueque + Mercado)», pero `proponerTrueque`, `aceptarTrueque`, `rechazarTrueque` y `colocarOrdenMercado` los puede hacer **cualquier residente presente**, sin cargo. El cargo solo pesa en las políticas, la reserva manual y abrir el almacén a un aliado.
- **Rol `observador`.** `Docs/Arquitectura/5` lo define «solo lectura, proyección pública» y el servidor permite otorgarlo, pero **no hay ninguna ruta que lo admita**: `puedeAdministrar` y `puedeJugar` lo excluyen, así que una membresía de observador no concede nada (ni siquiera leer).
