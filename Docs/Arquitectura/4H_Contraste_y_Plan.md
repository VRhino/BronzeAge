# Contraste y plan de la fase 4H (contrato y proyecciones)

Preparado el 2026-10-22, solo lectura, sobre `claude/lucid-curran-6d6ffa` (HEAD `6b191f7`). Nada editado, ningún git.
Verificado en esta sesión: `buf lint` limpio y `cargo test -p contrato` en verde (los 11 pares dorados). **No** se ejecutaron `npm test` (TypeScript) ni `dotnet test` (C#): piden `npm install`/`dotnet restore` y la máquina tiene Smart App Control (memoria del usuario); el estado de esos dos lados se da por el último commit que los tocó (`424d567`, anterior a todo el sistema 4).

Convenciones: `H:n` = `Docs/Arquitectura/Handoff_Fase6.md` línea n; `S4:n` = `Docs/Arquitectura/Sistema4_Contraste_y_Plan.md`; `05:n`/`06:n`/`01:n` = `Docs/Coordinacion/`; `PA:n` = `Docs/Mechanics/Proyecciones_Y_Autorizacion.md`; `NG:n` = `Niebla_De_Guerra.md`; `BU:n` = `Batalla_Unity_Ciclo.md`; `PAR:n` = `Docs/Mecanicas a desarrollar.md`.

---------------------------------------------------------------------------------------------------------------------------------------

## 1. Inventario

### 1.1 Órdenes: núcleo (1-125), `sesiones::traducir` y `.proto`

El núcleo numera **125** comandos (todos los códigos 1-125 existen en `comando.rs:1354-1527`, incluidos 62 `QuitarEscolta` y 94 `FijarRelacion`). El `.proto` trae **135** órdenes (`orders.proto:825-965`, tags 10-144) más los 4 mensajes de batalla.

| Estado | Cuántas | Cuáles |
|---|---:|---|
| **Mapeadas hoy** como `Orden` (`sesiones.rs:428-461`) | **3** | 2 `MarcharA` (solo `punto`; `asentamiento` se rechaza con `objetivo.no_soportado`, `sesiones.rs:436`), 3 `Estacionar`, 4 `Reloj` |
| Mapeadas por el canal del servidor de batalla (`traducir_batalla`, `sesiones.rs:326-417`) | **4** | 101 asignación, 102 inicio, 104 salida, 105 resultado |
| **Total que hoy llega por el contrato** | **7 de 125 (5,6 %)** | |
| Con orden en el proto y **sin mapear** | **89** | 7; 12-24; 26-36; 39; 41-42; 45-64; 69-93; 95; 99-100; 106-112; 114; 116-117; 123-125 |
| **Sin orden en el proto** (jugador: falta añadirla, aditivo) | **10** | 25 `CancelarOrdenMercado`; 40 `AparcarCaravana`; 103 `RegistrarTokens` (el `MensajeDeServidorDeBatalla` no trae `tokens`, `session.proto:64-71`); 113 `RetirarDeAvanzada`; 115 `DepositarEnAvanzada`; 118 `VaciarCarro`; 119-122 `EntrarEnPueblo`/`SalirDePueblo`/`EntrarEnAvanzada`/`SalirDeAvanzada` |
| Andamiaje de administración (sin orden en el proto, por diseño provisional) | **19** | 1 `CrearEjercito`, 5 `FijarNivelFaccion`, 6 `DarCaravanaFundacion`, 8-11 (`FijarNivelAsentamiento`, `AgrandarZona`, `ReclamarParcela`, `SoltarParcela`), 37 `DarRecursos`, 38 `DarSuministro`, 43 `FijarResidencia`, 44 `DarAlmacenPersonal`, 65 `FijarTecnologias`, 66 `FijarFaccionDeHeroe`, 67 `ColocarHeroe`, 68 `HerirHeroe`, 94 `FijarRelacion`, 96 `ColocarBandidos`, 97 `DarEscuadra`, 98 `FijarControlador` |

(7 + 89 + 10 + 19 = 125.)

Detalle de los 89 mapeables, con el nombre del proto (los que necesitan algo más que un cambio de nombre llevan nota):

- Economía/construcción: 7→`fundar` (vacía: la columna sale del héroe de la cuenta); 12→`anadir_edificio_manualmente`; 13→`quitar_de_cola`; 14→`mover_en_cola` (`direccion` texto frente a `arriba: bool`); 15→`mejorar_edificio_ahora`; 16→`alternar_auto_construccion`; 17→`alternar_receta`; 18→`calibrar_reserva_manual`; 19→`solicitar_ascenso`; 20→`activar_politica`.
- Comercio: 21→`proponer_trueque`; 22/23→`aceptar_`/`rechazar_trueque`; 24→`colocar_orden_mercado`; 26→`comerciar_en_plaza`; 27→`crear_caravana`; 28→`agregar_carro_caravana`; 29→`comprar_animal_caravana`; 30→`mover_carro_caravana`; 31→`reservar_caravana`; 32→`preparar_caravana`; 33→`cancelar_caravana`; 34→`mover_carga_caravana_aparcada`; 35→`enviar_caravana_al_origen`; 36→`entregar_de_caravana`; 39→`adjuntar_caravana`; 41→`comprar_en_campamento`; 42→`repostar_en_campamento`.
- Pueblos: 45→`marcar_pueblo`; 46→`entregar_en_obra` (rama pueblo; el núcleo no usa `materiales`, entrega lo que falta); 47→`destruir_pueblo`; 48→`reponer_carro_de_acarreo`.
- Héroe/tropa/presencia: 49→`crear_heroe` (el núcleo no tiene nombre, clase, género ni avatar: PAR:86, PAR:103); 50→`reclutar_tropa`; 51→`reclutar_en_campamento`; 52→`pedir_prestamo`; 53→`reponer_prestamo`; 54→`guardar_loadout`; 55→`borrar_loadout`; 56→`repartir_puntos`; 57→`ordenar_escuadras`; 58/59→`asignar_`/`retirar_guarnicion`; 60→`pasar_a_viveres`; 61/62→`asignar_`/`quitar_escolta`; 63/64→`desconectarse`/`conectarse`.
- Columnas/convocatorias: 69→`salir_al_mundo`; 70→`movilizar_ejercito`; 71→`salir_del_campamento`; 72→`entrar_en_asentamiento`; 73→`salir_de_asentamiento`; 74→`guarnecer`; 75→`entrar_en_campamento`; 76→`unirse_a_ejercito`; 77→`unirse_en_campo`; 78→`responder_peticion_de_union`; 79→`separarse_del_ejercito`; 80→`ceder_liderazgo`; 81→`organizar_ejercito`; 82→`cancelar_formacion`; 83→`repostar` (el proto solo trae `heroe_id`: el núcleo pide la plaza, hay que deducir el lugar a la puerta); 84-90→`convocar_ejercito`, `unirse_a_convocatoria`, `responder_peticion_de_convocatoria`, `cambiar_seleccion_de_convocatoria`, `separarse_de_convocatoria`, `cancelar_convocatoria`, `partir_convocatoria`.
- Encuentros/combate/batalla: 91→`perseguir`; 92→`dejar_de_perseguir`; 93→`inspeccionar`; 95→`atacar` (6 objetivos + `declara_guerra`, ya en el proto); 99→`unirse_a_batalla`; 100→`cancelar_batalla`.
- Avanzadas: 106→`colocar_avanzada`; 107→`entregar_en_obra` (rama avanzada); 108→`mejorar_avanzada`; 109→`reparar_avanzada`; 110→`destruir_avanzada`; 111→`elegir_tropa_de_avanzada` (el núcleo guarda una elección por escalón 2, 3 y 4; el contrato solo `tropa_elegida_escalon2`); 112→`guarnecer_avanzada`; 114→`repostar` (la misma orden que la 83); 116→`cancelar_obra`.
- Acceso y murallas: 117→`fijar_puerta` (`cerrada_a` = los grupos a `false` de `FijarAcceso`; "vasallos" van en aliados; `aedas` no existe en el núcleo); 123→`comprometer_recinto`; 124→`mejorar_recinto`; 125→`abandonar_recinto`.

**Órdenes del proto sin comando en el núcleo: 45** (de 135; las otras 90 corresponden a 92 comandos porque 46/107 comparten `entregar_en_obra` y 83/114 comparten `repostar`):

| Grupo | Cuántas | Cuáles | Quién las trae |
|---|---:|---|---|
| Política y Facciones | 28 | `crear_faccion`, `solicitar_ingreso`, `dejar_faccion`, `responder_solicitud`, `asignar_rey`, `asignar_embajador`, `asignar_cargo_local`, `designar_capital`, `admitir_otras_facciones`, `proponer_/responder_/retirar_` de relación, anexión y fusión, `proponer_paz`, `romper_relacion`, `rebelion_vasallo`, `declarar_guerra`, `vetar_jugador`, `renombrar_asentamiento`, `alternar_reabastecer_aliados`, `cambiar_residencia`, `residir_en_campamento`, `dejar_residencia` | sistema 5 |
| Tecnología, Aedas, intel | 7 | `adoptar_tecnologia`, `empezar_epica`, `abandonar_epica`, `comprar_tecnologia_aeda`, `comprar_mirada`, `comprar_informe_plaza`, `abrir_alijo` | sistema 6 / Taberna |
| Refundación | 5 | `aportar_a_refundacion`, `retirar_de_refundacion`, `comprar_caravana_de_refundacion`, `lanzar_caravana_fundacion`, `desarmar_caravana_fundacion` | deuda de Fundación (PAR, fila Territorio) |
| Caravanas / almacén personal | 5 | `cargar_caravana`, `reclamar_caravana`, `soltar_caravana`, `guardar_en_almacen_personal`, `sacar_del_almacen_personal` | deuda de columnas (PAR:89, PAR:150-151); no es contrato sino núcleo |

Desajuste de numeración a corregir en documentos: `Comandos_Parametros.md:3` y `PA:11` hablan de 122 comandos (los del TypeScript); `05:18` dice 135 órdenes (las del proto); el núcleo tiene 125. La matriz de autorización (`PA:167-285`) hay que reescribirla para 135 + las nuevas.

### 1.2 Entidades y proyecciones

`Entidad` (`session.proto:144-177`) tiene **29** variantes. Estado real:

| Situación | Cuántas | Cuáles |
|---|---:|---|
| Proyectada hoy | 1 | `Ejercito` (`proyeccion.rs:534-568`), solo en el ámbito `HEROE:<cuenta>` y solo la columna de la que se es participante (`sesiones.rs:70-90`). Faltan `persiguiendo`, `te_sigue`, `en_batalla` y `dentro_de` |
| Proyección a medias, sin usar en la sesión | 1 | `Asentamiento` (`proyeccion.rs:571-586`: 9 campos de ≈35; solo la usa `tests/territorio.rs:344`) |
| Con estado completo en el núcleo y **sin proyectar** | 16 | `Heroe`, `Escuadron` (y `Loadout`), `Caravana`, `Convocatoria`, `CampamentoMercenarios`, `CampamentoBandidos`, `AcuerdoTrueque`, `OrdenMercado`, `Batalla`, `Pueblo`, `Avanzada`, `ObraDeMapa`, `NodoRecurso`, `RedCaminos`, `ZonaFaccion` (derivable de `poligono_zona`), `TecnologiasFaccion` (solo el andamiaje `FijarTecnologias`) |
| Sin dominio en el núcleo (otros sistemas) | 11 | `Faccion` (solo `niveles_faccion`), `RelacionPolitica`, `Propuesta`, `Titulo`, `AedaResidente`, `AedaItinerante`, `MiradaIntel`, `InformePlaza`, `Alijo`, `EstadoTecnologia`, `MemoriaFaccion` (la niebla: ver abajo) |

Lo que el usuario pidió mirar, uno a uno:

| Cosa | Proto | Núcleo | Proyección / hueco |
|---|---|---|---|
| **Pueblo** | `Pueblo` (`entities.proto:247`) | sí (3C) | no proyectado. Falta "en asedio" |
| **Avanzada** | `Avanzada` (`:266`) | sí (4F) | no proyectada. Faltan: elecciones de tropa por escalón 2-4 (solo hay `tropa_elegida_escalon2`), guarnición de jugadores, "en asedio" |
| **ObraDeMapa** | `ObraDeMapa` (`:289`) | sí (pueblo y avanzada, mejora, reparación) | no proyectada |
| **Recintos** | `Recinto` (`:160`, con avance, `mejorando_a`, `siguiente_celda_en_ms`), `RecintoEnBatalla`, `RecintoInforme`; órdenes `abandonar_`/`comprometer_`/`mejorar_recinto` | sí (3 tandas) | `Asentamiento.recintos` no sale; ids de texto `recinto-<plaza>-<n>` por convertir (Murallas K14, `Murallas.md:343`); `Nucleo::recintos_en_pie` existe pero **no se congela en el ticket** |
| **Trazado urbano** (calles, caminos, piezas, radio, `layoutVersion`) | **no hay mensaje** (grep de "trazado" en `proto/` = 0) aunque `06:63` y `PA:310` dicen que viaja por la sesión | `VistaCiudad` (`urbanismo.rs:484`) | hueco de contrato: hace falta un `TrazadoUrbano` aditivo y `ancho`/`alto` en `Edificio` |
| **Batalla** | `Batalla` (`battle.proto:314`) | sí (4E) | no proyectada; ámbito `BATALLA` no existe |
| **Ticket** | `BattleTicket` completo | ticket **recortado**: sin `HeroSnapshot` (nombre, clase, nivel, perks, atributos, casillas), sin nivel/experiencia/moral de `SquadSnapshot`, sin `SettlementBattleSnapshot` (edificios, recintos, `layoutVersion`), carros sin carga (PAR:109, PAR:84(3), `BU:19`) | y `Difusion::Batalla` **nunca se envía** (`sesiones.rs` no lo usa); `ComandoSesion::Batalla` sí se recibe |
| **Tokens al servidor de batalla** | `TokensBatalla` en `MensajeParaServidorDeBatalla` | `RegistrarTokens` (103) | falta el mensaje entrante (`MensajeDeServidorDeBatalla` no lo trae) |
| **Token del jugador** (`BU:275`, antes `GET /jugador/.../asignacion`) | **ningún mensaje**: `Batalla` no lleva asignación propia | `token_de_jugador` | hueco de contrato: el jugador no puede conectar con Unity |
| **Acuse de los mensajes del servidor de batalla** | `MensajeParaServidorDeBatalla` solo trae ticket/incorporación/tokens | aceptar/rechazar con código | hoy un rechazo vuelve como `Aviso` (`sesiones.rs:317-322`); sin acuse de lo aceptado (PAR:109) |
| **Asedio visible desde fuera** (3.ª ronda) | no hay campo en `Asentamiento`/`Pueblo`/`Avanzada` ni "puedes unirte" | `Nucleo::batalla_sobre` | hace falta `batalla_id` en los tres, `Batalla.lado_para_unirse` por espectador y el plazo (`convocatoria_hasta_ms` ya existe) |
| **Acceso por relación** | `Asentamiento.puerta_cerrada_a` (`GrupoPuerta`), `fijar_puerta` | `Acceso{aliados, neutrales, enemigos}` | proyectar `puerta_cerrada_a`; pueblos y avanzadas heredan (no necesitan campo) |
| **Ubicación dentro de pueblo/avanzada** | `UbicacionHeroe.donde` solo plaza/columna/campamento/desconectado (`:389-396`) | `Ubicacion::Pueblo`/`Avanzada` (BAN18) | hueco: añadir `pueblo_id` y `avanzada_id` al oneof; y `Ejercito.dentro_de` (la columna que entró queda fuera del mapa) |
| **Población entera** | `Poblacion{Ritmo×3}` "CONTINUA; se lee entera" (`entities.proto:201`) | entera con resto oculto y recuperación (BAN14) | semántica distinta (ver §2) |
| **Almacén en escalones + tendencia "+N/min"** | `Existencia{Ritmo}`; `common.proto:41` obliga al cliente a interpolar | el stock cambia en el paso de minuto | no existe cómputo de tendencia en el núcleo (grep = 0); decisión de presentación de 4H (`Economia_Por_Minuto_Definicion.md:21`) |
| **Informe de inspección** (10 min, columna/caravana/plaza con recintos) | **no hay mensaje** (grep de "nspecc" en `proto/` solo da la orden) | `Nucleo::informes_de_inspeccion` | hueco: `InformeInspeccion` + variante de `Entidad` + `TipoEntidad` |
| **Convocatoria** | `Convocatoria` | sí | no proyectada |
| **Carros perdidos `"<caravana>:<posición>"`** | `BattleResult.carros_perdidos` es `repeated string`: **cabe sin cambio** | sí (`BAN19`) | solo documentar el formato; el contexto sí necesita la lista (abajo) |
| **Lista de caravanas del contexto** | `ContextoCaravana` trae una `caravana_id` + `carros` (`battle.proto:54-59`) | `Contexto::Caravana{caravanas: Vec}` (`ciclo.rs:39`) | aditivo: `repeated CaravanaEnJuego caravanas = 5` |
| **Batalla sin iniciador ni humanos** (bandidos) | `iniciada_por: string`, `BattleSide.faccion_id` opcional, `escuadras_sin_heroe`, `tokens_participante` repetido: **expresable sin cambios** | sí | documentar `iniciada_por` vacío y asignación sin tokens |
| **Niebla / memoria** | `MemoriaFaccion`, `FichaConocida`, `NieblaProyectada` | el crate `vision` calcula máscaras (`niebla_proyectada`), pero **el estado no guarda exploración ni fichas** (grep `exploracion` en `crates/nucleo/src` = 0) y la sesión nunca rellena `niebla` (`sesiones.rs:209`, `:236`) | requiere estado nuevo (foto) |
| **Plazas como ojos** | — | PAR:95: las plazas no observan; solo columnas y avanzadas (`espacial.rs:49-58`) | `NG:80` dice que la plaza ve su zona |

Ids que hoy produce la proyección: `ej-N`, `heroe-N`, `esc-N`, `as-N` (helpers en `proyeccion.rs:490-518`), `car-N` (ad hoc en `proyeccion.rs:561`) y facciones `f{N}` (`proyeccion.rs:550`; los dorados usan `f-1`). No existen para pueblo, avanzada, obra, campamento, bandidos, convocatoria, acuerdo, orden de mercado, edificio, recinto (los tres últimos, `recinto-<plaza>-<n>`) ni batalla (UUID: `Nucleo::uuid_de_batalla`).

### 1.3 Ámbitos y reanudación (lo que de verdad hace `sesiones.rs`)

- Un solo ámbito, `HEROE:<id de cuenta>` (`proyeccion.rs:520-527`), con ejércitos. No existen `FACCION`, `ASENTAMIENTO`, `CAMPAMENTO`, `BATALLA`, `VISTA` (solo se recuerda el rectángulo, `sesiones.rs:44`, `:141-145`) ni `ADMIN`. `Revocado` nunca se emite.
- `CambioVersion` (`estado.rs:580-588`) solo registra **ejércitos y plazas tocados** y eventos; cerrar versión solo pone `version` a esas dos (`nucleo.rs:462-488`). Ni héroes, escuadras, caravanas, pueblos, avanzadas, batallas ni convocatorias tienen "tocado" ni `version`. Hay ≈86 sitios que marcan `tr.tocados`/`tr.asentamientos`.
- `reanudar` (`sesiones.rs:205-217`) sí decide `SinNovedad`/`ConDeltas`/`ConFoto` por la versión conocida; el registro (`retencion_cambios` = 65.536 versiones) no se persiste, así que tras reiniciar todo cliente recibe foto.
- Eventos: `agrupar` solo los reparte a los dueños de las **columnas** nombradas (`sesiones.rs:82-86`): los eventos de plaza (construcción, comercio, mercado, batalla en una plaza) no llegan a nadie. El payload solo lleva `ejercitoId`, posición y números (`proyeccion.rs:602-622`).

### 1.4 Dorados

`contratos/golden/`: **11 pares** (`.json` fuente + `.bin` canónico generado por `contratos/ts`): `MensajeDeCliente` {hola, orden_atacar, orden_controlar_reloj, ping}; `MensajeDeServidor` {bienvenida, delta_parcial (con `QUITA`, evento y `Struct`), error_hola_repetido, foto_ejercito, pong, resultado_rechazo (con `detalle`)}; `ArtefactoDeMundo` {manifiesto}. Rust (`crates/contrato/tests/golden.rs`, exige ≥ 10) pasa; el runner de C# (`GoldenTests.cs`) y el de TS (`golden.test.ts`) saben tres tipos de raíz, lo que basta para cualquier mensaje anidado.

**No cubren**: ninguna entidad más que `Ejercito`; `Heroe`, `Escuadron`, `Asentamiento` (mapas `almacen`, `Ritmo`, `Poblacion`, `Recinto`), `Pueblo`, `Avanzada`, `ObraDeMapa`, `Caravana`, `Convocatoria`, `Batalla`; `MensajeDeCliente.vista`/`.batalla` (los cuatro mensajes del servidor de batalla); `MensajeDeServidor.revocado`/`.aviso`/`.batalla` (ticket, incorporación, tokens); `NieblaProyectada` (bytes); `Foto` en ámbitos distintos de `HEROE`/`VISTA`; ninguna de las 135 órdenes salvo `atacar` y `controlar_reloj`.

Regla de los dorados (`contratos/README.md:21-23`): `Struct` con una sola clave y mapas con **una sola entrada** (el orden de los mapas no está fijado por Protobuf; Rust fuerza `BTreeMap` con `btree_map(["."])` en `build.rs`, C# y TS no). Consecuencia: un `Asentamiento` con 29 existencias no se puede fijar en bytes; los dorados de entidades con mapas llevan 1 entrada y los tests de 4H comparan en estructura, no en bytes, el resto.

Orden de trabajo ya establecido: se edita el JSON, `GOLDEN_UPDATE=1 npx vitest run` regenera el `.bin` (lo escribe TS), y Rust y C# lo verifican. Comprobación adicional: `buf breaking` (reglas `FILE`) contra el `.proto` del commit anterior.

---------------------------------------------------------------------------------------------------------------------------------------

## 2. Desajustes entre el contrato (05/06/01 y los `.proto`) y el núcleo de hoy

| # | Contrato dice | Núcleo hace | Resolución propuesta |
|---|---|---|---|
| D1 | `05:43`: ámbito `HEROE (id)` = héroe propio, escuadras, loadouts, inventario; los dorados usan `HEROE:heroe-3` (`MensajeDeCliente__hola.json`) | `HEROE:<cuenta>` (`proyeccion.rs:520-527`, `sesiones.rs:183`), solo ejércitos (`sesiones.rs:208`); `Bienvenida.heroe_id` = la cuenta, sin `faccion_id`, `InfoMundo` solo con hash (`sesiones.rs:181-192`) | ámbito y `Bienvenida.heroe_id` con el id del héroe (`heroe-N`); si la cuenta aún no tiene héroe, `heroe_id` ausente y se manda lo necesario para crearlo (`06:§6.2` de `PA:298-300`: campamentos para nacer, que el proto no trae) |
| D2 | `05:37-51`: siete ámbitos y `Revocado`; `PA:316`: cada miembro ve **enteras** todas las columnas de su Facción | solo `HEROE`, y solo columnas de las que se es participante; `Revocado` nunca se manda; `CambiarVista` se ignora | ver §3 (T3a-T3e) |
| D3 | `05:74-78`: `Cambio` parcial, `QUITA`, `campos_borrados` | siempre `ACTUALIZA` con la entidad completa, `campos_borrados` vacío, nunca `QUITA` (`proyeccion.rs:588-596`; PAR:152: la columna disuelta no se retira). Además `entities.proto` casi no usa `optional`: con presencia implícita de proto3, un escalar sin cambio y un escalar a 0 no se distinguen en un mensaje parcial | decisión Q2 |
| D4 | `05:64-65`, `05:83`, `common.proto:41`, `13_Mundo_Reactivo_Rust.md:38` y `:109`: el cliente evalúa `valor(t)` del `Ritmo` en cada fotograma y "el servidor no manda nada hasta que un ritmo cambia" | la economía va por paso de minuto (`Economia_Por_Minuto_Definicion.md:11-21`): el stock y la población cambian en escalones cada minuto y el contador debe mostrar el valor del servidor más una tendencia "+N/min" | `Ritmo.en_escalones` (aditivo, abajo) |
| D5 | `01:40`, `entities.proto:201-205`: "la población es continua y se lee entera" | población siempre entera con resto oculto y marca de recuperación (`Economia_Por_Minuto_Definicion.md:23-27`) | corregir 01 y el comentario; `Poblacion` pasa a escalones |
| D6 | `05:100-104`: `atacar` sin guerra → rechazo `guerra.confirmar` con `detalle` (contra quién y arrastre) | código `combate.sin_guerra` (`comando.rs:1860`), `resultado_orden` siempre con `detalle: None` (`sesiones.rs:470`); el arrastre es del sistema 5 | renombrar a `guerra.confirmar` con `detalle {objetivo, facciones}` (el arrastre queda vacío hasta el sistema 5) |
| D7 | `05:127-128`, `PA:383-390`: audiencias de eventos (sin plaza → todos; de plaza → su Facción y quien esté dentro; con héroes nombrados → ellos); "un test exigirá que todo código emitido figure en el catálogo" | solo llegan eventos con columnas nombradas; payload solo numérico (ej. no puede llevar `ganador`, `contexto`); no hay test de catálogo; ≈90 códigos emitidos en `crates/nucleo` frente a ≈180 del catálogo | tanda de eventos (T5) |
| D8 | `session.proto:204-209`, `05:96-97`: `ORDEN_DUPLICADA` "repite el resultado anterior" | `Duplicada` devuelve solo la versión (`nucleo.rs:652`) | aceptable (un cliente que reintenta ya tiene el estado); documentar |
| D9 | `ResultadoOrden.creadas: ReferenciaCreada{TipoEntidad, id}` | `Respuesta.creadas: Vec<u64>` sin tipo (`comando.rs:1936`) y `sesiones.rs:478` lo rotula siempre `Ejercito`; lo producen ≈11 módulos (héroe, edificio, batalla/incorporación, avanzada…) | `Creada{tipo, id}` en el núcleo (T0) y `TipoEntidad` nuevos: `EDIFICIO`, `RECINTO`, `INCORPORACION` |
| D10 | `06:63`, `PA:310`: el trazado urbano viaja por la sesión | no hay mensaje en el proto | `TrazadoUrbano` aditivo (§3 T1) |
| D11 | `BU:275`: el token del jugador se pide por `GET`; `05:12`: nada va fuera de la sesión | `token_de_jugador` existe, ningún canal lo entrega | `Batalla.asignacion_propia` (solo al propio combatiente) |
| D12 | `BU:99-107`, `Murallas.md:183`: el ticket del asedio congela edificios y recintos en pie | `Ticket` sin snapshot de plaza (PAR:84(3)) | congelarlo al abrir (cambio de foto) |
| D13 | `battle.proto:54-59` contexto de caravana con una caravana | lista de caravanas (`BAN19`) | campo aditivo; el viejo conserva su significado ("la primera") |
| D14 | `01:38-40` invariantes: "una caravana aparcada en una plaza que cae se destruye" | el sistema 3B-2 la manda a su origen en la conquista (`comercio/ruina.rs:77-78`, S4 Q8 sin respuesta registrada) | fuera de 4H; recordar que el invariante de 01 y el código discrepan |
| D15 | `PA:316`/`NG:150-176`: todos los móviles ajenos se redactan (`EjercitoAvistado` sin composición, plan solo el tramo visible, `caravanasAvistadas` sin cantidades) | no hay redacción: el único `Ejercito` que sale es el propio, completo | T3d |
| D16 | `entities.proto:389-396`: `UbicacionHeroe` sin pueblo/avanzada; `EstadoColumna` solo `MARCHANDO`/`ESTACIONADA` | `Ubicacion::Pueblo`/`Avanzada` y columna "dentro" (BAN18) | campos aditivos |
| D17 | `ControlarReloj`/admin: `05:100` "el rol lo decide el token"; `PA:296-300` | `admin = hola.cliente == ClienteAdminWeb` (`sesiones.rs:137`); el núcleo acepta andamiaje de administración solo con `Peticion.admin` | mantener; ver Q1 |

Añadidos detectados en el proto vs documentos: `EventoDominio.payload` es `Struct` (decisión recomendada en `05:177-180`); un `Struct` solo lleva `double`, suficiente para milésimas hasta 2⁵³ pero no para ids de 64 bits grandes (los ids de partida son pequeños).

---------------------------------------------------------------------------------------------------------------------------------------

## 3. Descomposición en tandas

Tamaños: S < 600 líneas Rust con pruebas; M 600-1.500; L 1.500-3.500; XL > 3.500 (estimaciones; el sistema 4 entero fue ≈ 25-30.000). Total de 4H estimado **≈ 17-22.000 líneas** (la mitad pruebas) y ≈ 600 líneas de `.proto`.

Reglas que valen para todas: el contrato es **aditivo** (campos con número nuevo, mensajes nuevos, valores de enum nuevos; nada de renumerar ni cambiar semántica de v1; lo viejo, `reserved`); Rust, C# y TS leen los mismos bytes (dorados); cada tanda cierra con `cargo test --workspace --no-fail-fast -j 1` en verde, `buf lint`, `buf breaking`, notas "Implementado en Rust" en los Mechanics afectados y filas de paridad; los cambios de **estado persistente** (foto) se agrupan para no subir el formato más de una vez: propongo un único **`BAN21`** que reúna lo que pidan las tandas T3c, T3d y T4 (tendencia, memoria de niebla, snapshots del ticket y datos del héroe) si el autor los aprueba.

### T0. Andamiaje común (S, ≈ 700 líneas) — sin cambio de foto
- Módulo `ids` en `proyeccion` (formato `<prefijo>-<n>` y su inverso) para todas las entidades; unificar facciones a `f-N` como los dorados; UUID de batalla ya existe.
- `Respuesta.creadas` pasa a `Vec<Creada{tipo, id}>` (D9) y `resultado_orden` lo traduce.
- Seguimiento de "tocados" por tipo (héroe, escuadra, caravana, pueblo, avanzada, obra, campamento, convocatoria, batalla, acuerdo, orden de mercado…) en la transacción, y `CambioVersion` ampliado; versión por entidad en un mapa lateral **no persistido** (tras reiniciar todos reciben foto de todos modos, como hoy).
- `RechazoOrden.detalle` y `guerra.confirmar` (D6).
- Prueba: cada mutación de estado marca su entidad (propiedad: ninguna entidad cambia sin quedar en el registro).

### T1. `.proto` aditivo + dorados en los tres lenguajes (M, ≈ 400 líneas de proto, ≈ 30 JSON, ajustes mínimos de los 3 runners)
Cambios propuestos (todos con número nuevo):
1. `Ritmo.en_escalones = 6` (bool): el cliente no interpola, muestra `valor_en_t0` y rotula `por_minuto` como tendencia. Por defecto `false` = semántica v1.
2. `UbicacionHeroe`: `pueblo_id`, `avanzada_id` en el oneof. `Ejercito`: `optional string dentro_de`, `bool en_batalla`, `optional string batalla_id`.
3. `Asentamiento`, `Pueblo`, `Avanzada`: `optional string batalla_id` (asedio visible). `Avanzada`: `map<uint32,string> tropa_elegida_por_escalon`, `repeated string guarnicion_jugadores_ids`.
4. `Asentamiento`: `optional TrazadoUrbano trazado` (radio, calles, caminos, `layout_version`, `unidades_por_celda`, rectángulos de piezas) y `Edificio.ancho`/`alto`.
5. `Batalla`: `optional LadoId lado_para_unirse`, `optional AsignacionPropia asignacion_propia{instancia, token, expira_en_ms}` (solo al propio combatiente).
6. `ContextoCaravana.caravanas` (`repeated CaravanaEnJuego{caravana_id, repeated string carro_ids}`); documentar `carro_id` = `"<caravana_id>:<posición>"` y `iniciada_por` vacío.
7. `MensajeDeServidorDeBatalla.tokens = 5` (TokensBatalla); `MensajeParaServidorDeBatalla.acuse = 4` (`AcuseBatalla{mensaje, resultado, rechazo}`).
8. `InformeInspeccion` (+ `Entidad.informe_inspeccion = 30`, `TipoEntidad.INFORME_INSPECCION`) con contenido de columna, caravana o plaza (con `RecintoInforme`).
9. `TipoEntidad`: `EDIFICIO`, `RECINTO`, `INCORPORACION` para `creadas`.
10. Órdenes: `OrdenCancelarOrdenMercado`, `OrdenAparcarCaravana`, `OrdenRetirarDeAvanzada`, `OrdenDepositarEnAvanzada`, `OrdenVaciarCarro`, `OrdenEntrarEnPueblo`/`SalirDePueblo`/`EntrarEnAvanzada`/`SalirDeAvanzada` (o una `OrdenEntrarEn`/`SalirDe` con oneof de lugar); `OrdenRepostar.optional lugar_id`; orden(es) de administración (Q1).
11. `Bienvenida.campamentos_para_elegir` (jugador sin héroe).
12. `Cambio.reemplaza` (si Q2 = A).
- Dorados nuevos (JSON → `.bin` por TS → Rust y C#): foto de `Heroe` y `Escuadron`; foto de `Asentamiento` con una existencia en escalones, población, recinto y trazado; `Pueblo`, `Avanzada`, `ObraDeMapa`; `Caravana`; delta con `QUITA` de columna disuelta; `Revocado`; `Aviso`; `Batalla` con `lado_para_unirse` y `asignacion_propia`; `MensajeParaServidorDeBatalla` (ticket de asedio con `SettlementBattleSnapshot` y recintos, ticket de caravana con dos caravanas, incorporación, tokens, acuse); `MensajeDeCliente.batalla` (asignación, inicio, salida, resultado con `carros_perdidos` `"car-7:0"`); `MensajeDeCliente.vista`; una orden de cada familia (≈ 12); `InformeInspeccion` de plaza con recintos; `NieblaProyectada` con `bytes`.
- Puerta de salida: `buf lint`, `buf breaking`, `cargo test -p contrato`, `npm test` en `contratos/ts`, `dotnet test` en `contratos/cs`. Documentos: `05` (§3, §5, §6, §8, §13), `01` (§3 invariantes de población y almacén), `Comandos_Parametros.md`.
- Dependencias: ninguna (puede ir primero y desbloquea Unity y los bots a la vez).

### T2. Traducción de órdenes (L, ≈ 3.000 líneas con pruebas; se parte en dos commits)
- T2a (≈ 1.600): presencia, héroe, columnas, convocatorias, encuentros, combate, batalla, avanzadas, acceso y murallas (49-125 menos lo mapeado: ≈ 70 órdenes).
- T2b (≈ 1.400): economía, comercio, pueblos (7, 12-48: ≈ 37 órdenes + 3 nuevas).
- Módulo `sesiones/ordenes.rs` con conversores de cadena de catálogo ↔ enum (29 recursos, tropas, carros, animales, cargos, políticas), búsqueda del héroe de la cuenta (`heroe_de_cuenta`) y de la columna a la puerta (para `repostar`), `marchar_a` a asentamiento = posición de la plaza (queda dentro del `radioPuerta` de 2 u).
- Rechazos de forma (ids que no parsean) con códigos ya existentes (`heroe.desconocido`…).
- Prueba de exhaustividad (el patrón de `Comando::implicados`): un `match` sin comodín sobre `Comando` que obliga a clasificar cada variante como "mapeada / admin / sin orden"; y una prueba de ida y vuelta por cada una de las ≈ 100 órdenes (orden del contrato → `Comando` esperado).
- Dependencias: T0, T1.

### T3. Proyecciones por ámbito (XL en conjunto, ≈ 8-10.000; cinco subtandas)
- **T3a. Infraestructura de ámbitos (M-L, ≈ 1.800)**: suscripción dinámica (`ASENTAMIENTO` al entrar/salir de una plaza, `CAMPAMENTO`, `BATALLA`), `Revocado`, versión por ámbito y sesión, `Hola.versiones_conocidas` por ámbito con clave `TIPO:id`, foto/delta genéricos a partir de un `Proyector` por entidad, índices por dueño para que `agrupar` no recorra todo en cada reparto. Corrige D1/D2.
- **T3b. Estado propio: ámbitos `HEROE` y `FACCION` (L, ≈ 3.000)**: `Heroe` (con loadouts y `cupo_guarnicion`/`liderazgo_total` derivados), `Escuadron`, `Ejercito` completo (también las de la Facción, `PA:316`), `Caravana` propias, `Convocatoria`, `InformeInspeccion`, `Pueblo`/`Avanzada`/`ObraDeMapa` de las plazas propias, acuerdos y órdenes de mercado de plazas propias. Usa `QUITA` cuando se disuelve algo.
- **T3c. Interior: ámbitos `ASENTAMIENTO` y `CAMPAMENTO` (L, ≈ 2.500; posible cambio de foto)**: `Asentamiento` completo (almacén en escalones con tendencia, población entera, edificios con cola y mejoras, recintos, políticas, ascenso, puerta, trazado urbano), `CampamentoMercenarios` (mercado, planta, residentes). La tendencia exige decisión Q3.
- **T3d. `VISTA` con niebla (XL, ≈ 4.000; cambio de foto)**: memoria de la Facción en el estado (exploración, fichas de plazas ajenas, informes), ojos de plazas (su zona) y avanzadas, visión compartida de aliados (con el andamiaje `FijarRelacion`), entidades ajenas redactadas (columna sin composición con el tramo del plan dentro de la vista y horizonte de 30 s, caravana sin cantidades, plaza como ficha con zona recortada, héroe público), `ZonaFaccion`, `RedCaminos`/caminos explorados, batallas visibles, campamentos de bandidos, `NieblaProyectada` en `Foto` y `Delta`, filtro por el rectángulo de `CambiarVista`. Q5 y Q6.
- **T3e. `ADMIN` (S, ≈ 500)**: todo el estado sin niebla (sin tokens de batalla, `PA:293-296`) para sesiones `CLIENTE_ADMIN_WEB`.
- Dependencias: T0, T1 (campos nuevos); T3b-T3e dependen de T3a.

### T4. Batalla de Unity: ticket, canal y proyección (L, ≈ 2.500; cambio de foto)
- Ticket completo y congelado: `HeroSnapshot` (necesita datos del héroe, Q4), `SquadSnapshot` con nivel, experiencia y moral, `SettlementBattleSnapshot` con edificios y `recintos_en_pie` (`BU:99-107`), carros con identificador y carga.
- Envío por la sesión: `Difusion::Batalla` con ticket, incorporaciones y tokens a las sesiones de servidor de batalla (a todas al conectar y al abrirse; gana la primera asignación, `batalla.ya_asignada`, Q7); acuse de cada mensaje; mensaje entrante de tokens (103).
- Batallas **sin humanos** (cuarta ronda) y **sin iniciador** (bandidos, `iniciada_por` vacío): asignación con la lista de tokens vacía; atacante sin Facción ni héroes.
- Proyección: ámbito `BATALLA` para los combatientes, `asignacion_propia` (token solo al dueño), `Batalla` visible bajo la niebla (`BU:306`: las columnas en batalla no viajan como avistadas), `lado_para_unirse` por espectador, `batalla_id` en plaza/pueblo/avanzada (asedio visible). Caso especial: el plazo de unión en el aviso (PAR:156).
- Dependencias: T0, T1, T3a; el ticket de recintos y el informe de inspección con recintos (Murallas Q5) se cierran aquí.

### T5. Eventos (M, ≈ 1.200)
- `EventoNucleo` con héroes nombrados, Facción y plaza; datos tipados (número, texto, lista) para el `Struct` (Q8); audiencias de `05:127-128`/`PA:383-390`; prefijo `evento_pve.`; crónica; catálogo con la lista de códigos de cada Mechanics y la prueba "todo código emitido está en el catálogo".
- Dependencias: T0, T3a.

### T6. Reanudación, causalidad y carga (M, ≈ 1.000)
- Pruebas de propiedad "foto + deltas ≡ foto final" por ámbito (sin repetir el diff del TS), reconexión con `versiones_conocidas` de varios ámbitos, hueco de secuencia, retención de 65.536 versiones (con el paso de minuto de 1.000 plazas = 1.000 versiones por minuto, la retención alcanza ≈ 1 h de mundo: más allá, foto), `version_esperada`, idempotencia.
- Medición: bytes por minuto por sesión con la economía en escalones, coste del reparto con 500 sesiones, y `cargo test --release -p nucleo --test asentamientos rendimiento -- --ignored` sin empeorar (el paso de minuto no cambia, pero ahora cada plaza tocada genera una proyección si alguien la suscribe: no debe proyectarse nada sin suscriptores).
- Dependencias: T3.

### T7. Cierre (S, ≈ 300 + documentos)
- Reescribir la matriz de autorización de `PA:167-285` a 135+ órdenes, actualizar `Comandos_Parametros.md`, `05`, `01`, `NG`, `BU`, `Murallas.md` §9.1, borrar las filas de paridad "Contrato" y "Proyecciones" resueltas, actualizar el handoff, cliente de ejemplo en `servidor::prueba` que juegue una partida corta solo por el contrato.

### Orden recomendado
`T0 → T1 → (T2a ∥ T3a) → T2b → T3b → T4 → T3c → T3d → T3e → T5 → T6 → T7`.
Después de T2 + T3b un bot ya puede jugar enteramente por el contrato; después de T4 hay batalla de Unity extremo a extremo; la niebla (T3d) es lo más grande y lo que menos bloquea. El batch militar (4G) queda al final como pidió el autor.

Riesgos a vigilar: (a) el reparto a 500 sesiones con proyecciones por plaza cada minuto; (b) un solo `BAN21` para todos los cambios de estado, o varios formatos seguidos que invalidan fotos de prueba; (c) mapas de más de una entrada en dorados (no determinista entre lenguajes); (d) `Struct` con números `double` (los ids y milésimas siguen por debajo de 2⁵³); (e) la autorización de las 135 órdenes es hoy provisional (`manda_en`, cuenta del héroe, Líder): 4H no debe inventar cargos, eso es el sistema 5.

---------------------------------------------------------------------------------------------------------------------------------------

## 4. Preguntas al autor (8)

**Q1. Las 19 órdenes de andamiaje de administración (`CrearEjercito`, `DarRecursos`, `FijarRelacion`, `FijarTecnologias`, `ColocarHeroe`…): ¿cómo viajan por el contrato?**
- A) Una sola `OrdenDeAdministracion{ codigo, parametros: Struct }`, solo para sesiones `CLIENTE_ADMIN_WEB`; se borra cuando los sistemas 5 y 6 las sustituyan. *(Recomendada: no ensucia el `.proto` con 19 mensajes temporales y el cliente de administración las tiene.)*
- B) 19 mensajes tipados.
- C) Quedan fuera del contrato (solo en el núcleo y el batch); el admin web no las tiene hasta el sistema 5.

**Q2. Forma de un cambio de entidad.** El contrato dice "parcial, solo los campos que cambiaron", pero `entities.proto` casi no usa `optional` y con proto3 un escalar sin cambio no se distingue de un 0.
- A) Entidad completa en cada `Cambio` con un `reemplaza: true` nuevo; el cliente sustituye. `campos_borrados` queda para el futuro. *(Recomendada: sin diff por entidad en los tres lenguajes; el tráfico grande —plan, existencias— ya va en `Plan` y `Ritmo`.)*
- B) Parcial real con máscara de campos (ahorra ancho de banda, mucho más código y pruebas en Rust, C# y TS).
- C) Híbrido: completa por defecto y parcial solo para `Asentamiento`.

**Q3. La tendencia "+N/min" del contador de recursos (decisión del 2026-10-22), ¿qué número es?**
- A) La producción neta del último paso de minuto de la plaza (producción − consumo − mantenimiento, sin contar gastos de órdenes), guardada por recurso con la plaza. Es estable y lo que el jugador espera ver. Exige estado nuevo (29 cifras por plaza, cambio de foto). *(Recomendada.)*
- B) La variación observada entre los dos últimos pasos (incluye gastos de obras y reclutamiento: ruidosa, a veces negativa sin motivo). Estado mínimo.
- C) Sin tendencia en 4H: el cliente muestra solo los escalones.

**Q4. Datos del héroe que Unity necesita en el ticket (nombre, clase, género, avatar; equipo e inventario vacíos mientras no haya catálogo de objetos).** El núcleo no los guarda (CQ-001/CQ-004, PAR:86, PAR:103).
- A) `CrearHeroe` los recibe ya (con validación contra una lista de clases y avatares que dé Conquest), se guardan en el héroe (cambio de foto) y el ticket los congela; atributos efectivos = base. *(Recomendada: son campos que el contrato ya trae y una batalla real no arranca sin ellos.)*
- B) El ticket lleva valores por defecto ("Héroe N", clase `default`) hasta que Conquest publique el catálogo; los campos se añaden al estado después.
- C) Se guardan nombre y clase ahora y el avatar queda para después.

**Q5. Alcance de la niebla en 4H.** La memoria de la Facción (exploración, fichas, informes) es estado de juego y entra en la foto.
- A) Niebla completa: memoria, plazas (su zona) y avanzadas como ojos, visión de aliados, máscaras en `Foto` y `Delta`; Miradas e Informes de plaza de la taberna quedan para Taberna/sistema 6. *(Recomendada: es lo que dice `Niebla_De_Guerra.md` y lo que cierra PAR:95.)*
- B) Mínima: solo lo visible ahora con los ojos que ya hay (columnas y avanzadas), sin memoria ni fichas; la memoria en una fase posterior.
- C) Sin niebla: la `VISTA` manda todo lo ajeno redactado (se lee en un DevTools; no vale para jugar, solo para pruebas).

**Q6. Cómo se renueva el tramo ajeno del plan (horizonte de ~30 s, `05:81-82`).** Una columna ajena visible que sigue en marcha necesita recibir el siguiente tramo antes de que se le acabe.
- A) Reutilizar el muestreo de visión de 250 ms: cuando al ojo le quedan menos de ≈ 10 s de tramo, se manda el siguiente; no hay citas nuevas en la cola. *(Recomendada: sin eventos nuevos, no cambia el estado de juego, determinista porque solo depende del plan.)*
- B) Una cita recurrente de 30 s por columna ajena visible (más exacta, pero es un evento recurrente por entidad).
- C) Mandar el plan completo de lo ajeno (revela el destino: contradice `NG:175`).

**Q7. Varios servidores de batalla: ¿cómo se reparte una batalla?**
- A) El ticket va a todas las sesiones de servidor de batalla y gana la primera asignación (`batalla.ya_asignada`), como en el ciclo documentado (`BU:232-236`); al conectarse un servidor recibe además las batallas pendientes. *(Recomendada: ya es lo que dice el ciclo y no añade política.)*
- B) El núcleo elige un servidor por turno o por carga y solo a él le manda el ticket.
- C) Un servidor por partida configurado de antemano.

**Q8. Eventos de dominio: ¿qué forma tiene el payload?** Hoy es un `Struct` solo con números; el catálogo pide texto (ganador, contexto) y listas (héroes).
- A) `Struct` para todos, ampliando `EventoNucleo.datos` a número/texto/lista; solo se tipa lo que el cliente tenga que interpretar para actuar cuando haya cliente que lo pida. *(Recomendada: es lo que recomienda `05:177-180`.)*
- B) `Struct` y mensajes tipados para unos 12-15 eventos que mueven interfaz (petición de unión, convocatoria, batalla abierta, expulsión, asedio…).
- C) Todos tipados (≈ 180 mensajes en tres lenguajes).

---------------------------------------------------------------------------------------------------------------------------------------

## 5. Decisiones que tomaría sin preguntar (anotarlas con `// DUDA:` si se discuten)

- Ids con la forma `<prefijo>-<n>` en todo el contrato, incluida la Facción (`f-N`); la proyección actual (`f{N}`) se cambia, los dorados ya usan `f-1`.
- `marchar_a` hacia un asentamiento termina en la posición de la plaza (queda dentro de la puerta de 2 u).
- `repostar{heroe_id}` busca la plaza o avanzada operativa a `radioPuerta` de la columna; si hay varias, la más cercana; sin ninguna, `repostar.sin_lugar`.
- `entregar_en_obra` ignora `materiales` (el núcleo entrega lo que falta, hasta lo que haya en el carro y las adjuntas) mientras no haya entrega parcial en el núcleo; documentarlo.
- El registro de cambios sigue sin persistirse (tras un reinicio todos reciben foto).
- La autorización de las órdenes nuevas es la provisional de hoy (`manda_en`, cuenta del héroe, Líder); los cargos son del sistema 5.
- Los rechazos del núcleo (336 constantes) se publican como catálogo estable: se genera la lista en la documentación y se prueba que cada rechazo está documentado.

## Decisiones del autor (2026-10-22)

1. **Administración:** una sola orden genérica `OrdenDeAdministracion{codigo, Struct}` solo para el admin web; temporal, desaparece con los sistemas 5 y 6.
2. **Forma de un cambio de entidad: híbrido.** La entidad completa (sustituye a la anterior) para casi todo; cambios parciales solo para las entidades grandes (plaza y ciudad).
3. **Tendencia «+N/min»:** el neto real del último paso de minuto por recurso, guardado por plaza (cambia la foto).
4. **Datos del héroe para el ticket** (nombre, clase, género, avatar): valores por defecto hasta que Conquest publique su catálogo; el núcleo no los guarda todavía.
5. **Niebla completa:** lo visible, la memoria de la Facción, plazas y avanzadas como ojos, lo que ven los aliados, y Miradas e Informes de plaza de la Taberna (cambia la foto).
6. **Tramo ajeno del plan:** lo renueva el muestreo de visión de 250 ms cuando quedan menos de ~10 s; sin citas nuevas.
7. **Varios servidores de batalla:** el ticket se publica a todos y gana la primera asignación (`batalla.ya_asignada` para las demás).
8. **Eventos:** `Struct` genérico para todos y ~12-15 mensajes tipados para los que mueven la interfaz (batalla, conquista, asedio, hambre…).

**Corrección del orquestador a D14:** el código ya destruye la caravana aparcada en la plaza que cae (`crates/comercio/src/ruina.rs:102`, con test): el invariante del contrato 01 y el código coinciden; no hay desajuste.

---------------------------------------------------------------------------------------------------------------------------------------

## Avance (2026-10-22, T0, T1 y T2 hechas, sin commit)

**T0 (andamiaje común).** Hecha sin cambio de foto (`BAN20`):
- `proyeccion::ids`: `<prefijo>-<n>` y su inverso para todas las entidades, la Facción también (`f-N`); los rechazos del núcleo ya nombran las entidades con esos ids (`caravana-N` → `car-N`, `avanzada-N` → `av-N`, `campamento-N` → `camp-N`, `orden-N` → `ord-N`).
- `Respuesta.creadas` lleva su clase (`Respuesta::tipo_creadas`, que sale de `Comando::tipo_creado`; en vez de `Vec<Creada{tipo,id}>` para no tocar los ≈400 usos de `creadas[0]` de las pruebas y del batch) y `ResultadoOrden.creadas` la traduce con `proyeccion::referencia_creada` (edificio y recinto con su plaza, la incorporación con el UUID de la batalla, el pueblo y la avanzada recién marcados como `OBRA_DE_MAPA`, la parcela de administración no sale). Una orden que crea algo y no declara qué falla en depuración.
- `CambioVersion::tocados` (`Tocados`): 14 clases (`Clase`: columna, plaza, Facción, héroe, escuadra, caravana, trueque, orden de mercado, pueblo, avanzada, campamento de mercenarios, bandidos, convocatoria, batalla) en vez de las dos listas de antes; `Nucleo::version_de(clase, id)` con el mapa lateral no persistido. Se marcan: lo que **crea** una orden, lo que **nombra** (`Comando::nombradas`), lo que cambian las reglas en cada sitio (héroes y escuadras, comercio, pueblos, avanzadas, convocatorias, batallas, bandidos, campamentos, Facciones) y lo que **desaparece** (QUITA). Una orden rechazada o sin cambios descarta lo que marcó.
- La **prueba de completitud** (la propiedad «ninguna entidad cambia sin quedar en el registro») no es un test aparte sino una auditoría dentro del núcleo (`nucleo/auditoria.rs`, `Ajustes::auditar_cambios`): compara la huella de cada entidad en cada versión y falla si cambió sin marcarse. Con `BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo` corre sobre las 353 pruebas de `nucleo` (todas en verde); `tests/cambios.rs` la pone en dos escenarios (columnas, héroes, escuadras, hambre y combate; economía, comercio, órdenes de mercado y convocatorias sobre el mundo real). Dejarla en CI es barato.
- `guerra.confirmar` (renombrado desde `combate.sin_guerra`, constante `rechazos::GUERRA_CONFIRMAR`) con `detalle` (`Resultado::Rechazada.detalle`, `Detalle::Guerra { objetivo, arrastre }`; en el contrato `{ objetivo: "f-N", facciones: [...] }`).
- **No** se marca el cambio que el paso de minuto hace al almacén de una plaza ni al de un pueblo (ver la fila de paridad «Contrato (4H, T0)»): lo hará T3c con la tendencia.

**T1 (esquema aditivo y dorados).** Hecha: ver las filas de paridad «Contrato (4H, T1)», los comentarios de los `.proto` y `Docs/Coordinacion/05` (§1, §2, §5, §6, §8, §9, §13), `01` (reglas e invariantes) y `06` (trazado urbano). Desviaciones y añadidos respecto a la lista del plan:
- `ContextoCaravana.caravanas` usa `CaravanaEnJuego { caravana_id, repeated CarroEnBatalla carros }` (con la carga de cada carro; el plan decía `carro_ids`), y `AcuseBatalla { mensaje, battle_id, resultado, rechazo }` lleva el `battle_id`.
- `TipoEntidad.LOADOUT` (para el `creadas` de `guardar_loadout`) y `TrazadoUrbano` sin rectángulos de piezas: las huellas van en `Edificio.ancho/alto` y los recintos en `Asentamiento.recintos`.
- Los 15 eventos tipados (decisión 8): `BatallaAbierta`, `BatallaAsignada`, `BatallaCerrada`, `PlazaConquistada`, `PlazaEnRuinas`, `AsedioResistido`, `UnionPedida`, `ConvocatoriaConvocada`, `ConvocatoriaPartio`, `HeroeExpulsado`, `ColumnaConHambre`, `DesercionPorHambre`, `GuerraDeclarada`, `ObraTerminada`, `HeroeHerido`.
- Se tradujo en `sesiones.rs` el mensaje entrante `tokens` del servidor de batalla (`RegistrarTokens`): era imprescindible para que el `match` siguiera siendo exhaustivo.
- Verificado: `cargo test --workspace --no-fail-fast -j 1` en verde (69 binarios, 1.131 pruebas, 6 ignoradas; +8 sobre las 1.123 de antes: 5 de `cambios.rs`, una de ids, una de `referencia_creada` y la de significado de los dorados); `buf lint` y `buf breaking` (contra `HEAD`) limpios; `cargo test -p contrato`, `npm test` en `contratos/ts` y `dotnet test` en `contratos/cs` (62 pruebas cada uno) en verde en esta máquina.

**T2 (traducción de órdenes).** Hecha entera (T2a y T2b juntas), sin cambio de foto ni de esquema; módulo `crates/nucleo/src/sesiones/ordenes.rs` (con sus pruebas en `ordenes/pruebas.rs`), tabla completa en `Docs/Mechanics/Comandos_Parametros.md` (último apartado):
- `sesiones::ordenes::traducir(&dyn Consulta, orden, cliente, admin)` sustituye a `Actor::traducir`. `Consulta` es lo que la traducción pregunta al mundo (versión, héroe de la cuenta, columna del héroe, posición de una plaza, lugar donde repostar, campamento del héroe, pueblo o avanzada de una obra, batalla de un UUID): la implementa `Nucleo` y las pruebas usan un mundo de mentira, así que la tabla se prueba sin el artefacto de mundo.
- **Cifras**: de las 145 órdenes del contrato, **99 órdenes del jugador** dan **101 comandos** (`entregar_en_obra` da `EntregarMateriales` o `EntregarMaterialesAAvanzada` según la obra; `repostar`, `Repostar` o `RepostarEnAvanzada` según el lugar), **1** es la orden de administración (**19 comandos** de andamiaje, solo en una sesión de administración: `no_autorizado` en cualquier otra) y **45** no tienen comando en el núcleo y se rechazan con `orden.no_disponible` (política y Facciones 28, tecnología, Aedas e intel 7, refundación 5, caravanas a mano y almacén personal 5). De los 125 comandos del núcleo, 101 + 19 + 5 (el canal del servidor de batalla, 101-105) = 125.
- Respeta las decisiones de la sección 5: `marchar_a` a un asentamiento termina en la posición de su plaza; `repostar` sin `lugar_id` busca la plaza o avanzada operativa más cercana a `radioPuerta` de la columna (`Nucleo::lugar_de_repostar`; `repostar.sin_lugar` si no hay); `entregar_en_obra` ignora `materiales`; `comprar_en_campamento` y `repostar_en_campamento` sin `campamento_id` usan el campamento donde está el héroe (`Nucleo::campamento_del_heroe`). Las órdenes que no nombran héroe usan el de la cuenta. Códigos nuevos: `orden.no_disponible`, `orden.forma_invalida`, `obra_de_mapa.desconocida`, `repostar.sin_lugar`; `ids::nodo` (`nodo-<n>`); `Cargo::desde_clave`.
- **Prueba de exhaustividad** (`todo_comando_esta_clasificado_y_toda_orden_del_contrato_esta_mapeada_o_es_no_disponible`): lee las variantes de `Comando` de su definición (125) y exige que cada una salga de una orden del jugador (101, por la tabla orden ↔ comando), de la administración (19) o del canal del servidor de batalla (5), sin repetirse; cuenta las órdenes de `orders.proto` (145) y exige que la tabla las cubra todas (mapeada, `no_disponible` o administración). Además el `match` sobre `Orden.cuerpo` de `traducir` no tiene comodín: una orden nueva del contrato no compila hasta clasificarse.
- **Pruebas** (suite completa, `cargo test --workspace --no-fail-fast -j 1`: 71 binarios, **1.145 pruebas** en verde, 6 ignoradas; +14 sobre las 1.131 de T1): 6 en `sesiones::ordenes::pruebas` (cada una de las 101 órdenes del jugador con el comando esperado y los datos de la petición, las 45 no disponibles, las 19 de administración y su autorización, lo implícito que se pregunta al mundo, los rechazos de forma y la exhaustividad), 1 en `tests/ordenes.rs` (la administración monta por el contrato un mundo sintético y los jugadores marchan, estacionan y atacan) y 7 en `tests/ordenes_mundo.rs` sobre el mundo real, una por familia (presencia y columnas de un jugador nuevo, economía, acceso y murallas, pueblos, avanzadas, comercio, batalla de Unity, y repostar sin lugar), todas con la respuesta causal (las `Delta` antes del resultado, `creadas` con su clase y su id del contrato). Las ayudas de sesión (`Servicio`, `dar`, `foto`, `delta`…) pasaron de `tests/sesiones.rs` a `tests/comun/sesion.rs`.
- Dudas y órdenes sin comando: filas de paridad «Contrato (4H, T2)» de `Docs/Mecanicas a desarrollar.md`. Lo principal: `crear_heroe` ignora nombre, clase, género y avatar; `movilizar_ejercito` y `salir_del_campamento` con `objetivo` se rechazan (`objetivo.no_soportado`: sale sin destino y después `marchar_a`); `cancelar_obra` de un pueblo se rechaza (es `destruir_pueblo`); `cancelar_batalla` y las demás órdenes sin héroe usan el héroe de la cuenta.
- Lección de pruebas por la sesión: lo que el reloj de mundo hace por su cuenta (p. ej. el `ejercito.encuentro` de dos columnas que se ven, un hecho debido en el instante en que se crean) llega a los dueños como una `Delta` propia o junto a la `Delta` de la orden siguiente, según cuándo despierte el actor; ninguna de las dos es un fallo. Una prueba que cuente las `Delta` de una orden debe esperar antes, con `esperar_hecho` (`tests/comun/sesion.rs`), lo que el reloj ya tiene que decir (así se arregló la intermitencia de `tests/ordenes.rs`, 40 % de fallos antes y 0 de 40 después). En `tests/ordenes_mundo.rs` los escenarios se preparan en el núcleo antes de arrancar la sesión y no queda ningún hecho de reloj por llegar (se comprobó volcando el registro de cambios de cada prueba).
- No hecho (es de T3-T4): ninguna proyección nueva; lo que proyecta la sesión sigue igual.
