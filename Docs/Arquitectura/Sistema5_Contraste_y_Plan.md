# Contraste y plan del sistema 5 (política y Facciones), fase 6

Preparado el 2026-10-24, solo lectura, sobre `claude/lucid-curran-6d6ffa` (HEAD `4377b86`, foto `BAN25`, 145 órdenes del contrato, comandos 1-127).
El TypeScript se leyó con `git show typescript-final:src/...` y `git grep ... typescript-final`.
Abreviaturas: G = `Docs/Game/`, M = `Docs/Mechanics/`, P = `Docs/Mecanicas a desarrollar.md` (sección «Paridad con TypeScript»), TS = `typescript-final:src/`.
G2 = `G/2_Sistema_Politico_Facciones_y_Cargos.md`, G0 = `G/0_Glosario_de_Entidades_Politicas.md`, Dip = `M/Diplomacia_Facciones.md`, Pres = `M/Presencia_Y_Convocatoria.md`,
Proy = `M/Proyecciones_Y_Autorizacion.md`.

Nota de partida: `M/Diplomacia_Facciones.md` se reescribió el 2026-10-13 para el mundo reactivo y es, salvo lo que se anota abajo, la descripción del TS. No hay una segunda
versión de la mecánica: hay que decidir cómo se traduce al reloj reactivo, qué recortes del sistema 4 se deshacen y qué hacer con unas pocas reglas que el canon y Mechanics dicen
de dos maneras.

---------------------------------------------------------------------------------------------------------------------------------------

## 1. Inventario

### 1.1 El TypeScript de política (el objetivo de paridad)

| Fichero (TS) | Líneas | Qué contiene | En Rust hoy |
|---|---:|---|---|
| `engine/faccion.ts` | 256 | `crearFaccion`, `calcularNivelFaccion`, `calcularCapFundacion`, `calcularCupoNivel`, `aplicarExperiencia`, `otorgar/quitarCiudadania` (sucesión), `registrarDerrota`, cooldown de residencia, `dejarResidencia`, `solicitarIngreso`, `responderSolicitud`, `cambiarResidencia` | cap y cupo por nivel sí (leen `niveles_faccion`); lo demás no |
| `engine/cargos.ts` | 49 | quién puede ser designado | no |
| `engine/pertenencia.ts` | 190 | `esResidente`, `estanAliadas`, `estanEnGuerra`, `compartenVision`, `grupoDePuerta`, `puedeEntrarEn` | parcial: `EstadoMilitar::trato`/`Acceso::permite` (`estado_militar.rs:122-160, 234`), sin vetos ni grupo `aedas` |
| `engine/capital.ts` | 41 | `designarCapital`, `encontrarCapital` | no (`ES_CAPITAL = true`, `DISTANCIA_CAPITAL = 0`, `construccion.rs:53-54`) |
| `engine/sigilo.ts` | 92 | validar, duplicado exacto, `sigiloLibre` (FNV-1a) | no (el bloque `CATALOGO_SIGILO` está en Dip §3.8) |
| `engine/reputacion.ts` | 72 | ajustes de reputación, `ajustesPorDefensaDeVasallo` | el ritmo sí (`comercio/src/reputacion.rs`, con todos los `Suceso`); solo lo alimentan los trueques |
| `engine/diplomacia.ts` | 465 | `validarPar`, propuestas, `declararGuerra` con arrastre, paz, `romperRelacion`, `ajustesPorGuerraTrasRomper`, `rebelionVasallo`, `liberarVasallosDeSenoresDesarmados`, `pasarVasallasConquistadas`, `avanzarTributos` | no (solo el mapa de pares de `FijarRelacion`) |
| `engine/liga.ts` | 52 | `computeLigas`, Gran Rey | no |
| `engine/anexion.ts` / `fusion.ts` / `trasladoDeFaccion.ts` | 75 / 147 / 132 | proponer, responder, `trasladarFaccion` | no |
| `engine/titulos.ts` | 107 | `calcularTitulos`, `narrarCambiosDeTitulo` | no |
| `session/comandos/{crearFaccion,ingresoEnFaccion,dejarFaccion,cargos,diplomacia,anexion,fusion}.ts` | 82+56+43+282+236+106+115 | las 28 órdenes de política (+ `activarPolitica`, ya portada) | ninguna (las 28 dan `orden.no_disponible`, `sesiones/ordenes.rs:607-634`) |
| `session/comandos/presencia.ts` (parte) | ≈ 120 de 518 | `fijarPuerta`, `vetarJugador` | `FijarAcceso` (117) en su lugar |
| `session/comandos/autorizacion.ts` | 867 | la matriz con cargos, residencia y presencia (Proy §5.2) | no: `manda_en` y «la cuenta del héroe» (Proy §5.1) |
| `session/resultadoBatalla.ts` (parte) | — | `conReputacionDeVasallos`, XP de Facción, `registrarDerrota` tras Unity | no (P l.108) |
| **Total de reglas y órdenes** | **≈ 3.600** | | |

Pruebas TS de política: 28 ficheros, **≈ 245 casos** (`engine/__tests__/`: `cargos` 8, `diplomaciaDecisiones` 5, `eventosDominio{Diplomacia,Faccion,Politicas,Titulos}` 2+4+2+3,
`experienciaFaccion` 8, `faccion` 6, `reputacionVasallos` 6, `residenciaMercenaria` 16, `zonas_fusionadas` 9, `capital` 7, `puerta` 7, `sigilo` 6, `ubicacion` 3,
`vasallaConquistada` 2; `session/__tests__/`: `comandosAnexion` 11, `comandosCargos` 9, `comandosDiplomacia` 18, `comandosFusion` 11, `comandosMembresiaFaccion` 19,
`comandosResidencia` 6, `derrotas` 2, `faccionDeLaColumna` 2, `sigiloFaccion` 4, `autorizacionComandos` 23 y la parte de puerta de `comandosPresencia` 32). Cada tanda
nombra las que porta.

Fuera del TS (no hay paridad que perseguir): los gremios (G2 §2.10; `Docs/Ideas a diseñar.md` §21), la elección del Rey por voto (P §39) y el título «General con más victorias»
(`titulos.ts:27-29`).

### 1.2 Lo que ya existe en Rust y el sistema 5 reutiliza

| Pieza | Dónde | Estado |
|---|---|---|
| Ciudadanía como dato del héroe | `HeroeNucleo.faccion: Option<u32>` (`estado_militar.rs:42`) | la fija `FijarFaccionDeHeroe` (`nucleo/heroes.rs:325`); sin orden de ingreso (la sucesión es imposible) |
| Residencia del héroe + índice plaza → residentes | `Heroe::residencia`, `EstadoMilitar::{mudar, residentes_de}` (`estado_militar.rs:191-215`) | la fija `FijarResidencia` (`nucleo/mercenarios.rs:325`), que ya suelta la guarnición y retira la tropa prestada; la ruina y la conquista ya reubican (4B-1, 4D) |
| Nivel de Facción | `Estado.niveles_faccion` (`estado.rs:790`), `FijarNivelFaccion` (`zonas.rs:234-241`) | lo leen el cap de fundación (`zonas.rs:340`), el cupo de nivel 2/3 (`construccion.rs:397-400`) y los huecos de política del Gobernador (`construccion.rs:525`) |
| Relaciones | `EstadoMilitar.relaciones: BTreeMap<(menor, mayor), Relacion>` (`estado_militar.rs:175`), `FijarRelacion` (`espacio.rs:306-322`) | sin dirección, estado, historial ni tributo; lo consumen encuentros, defensa, puerta, unirse a batallas y visión compartida |
| Guerra al atacar | `comprobar_agresion` / `declarar_guerra` (`combates.rs:299-335`), `Detalle::Guerra { objetivo, arrastre }` (`comando.rs:2215`) | `arrastre` siempre vacío; hecho `faccion.guerra_declarada` |
| Reputación | `comercio::reputacion::{Reputacion, Suceso}` (ritmo en ppm con la cita del cruce por 0), `Estado.comercio.reputacion` | `Suceso` ya tiene `LiberarVasalloVoluntario`, `RebelionParaSenora`, `RomperAlianza`, `AtacarAliado`, `DefenderVasallo`, `NoDefenderVasallo`, `GuerraTrasRomperAlianza`: nadie los produce |
| Combate digno | `combate::resolucion::Informe` (`ratio`, `digno`) | sin consumidor (P l.103) |
| Derrota | `combate::conquista::faccion_derrotada` (`conquista.rs:177`), hecho `faccion.derrotada` (`construccion.rs:390`, `conquistas.rs:31`) | sin `derrotadaPor` |
| Puerta por relación | `Acceso { aliados, neutrales, enemigos }` (`estado_militar.rs:139-160`), `FijarAcceso` (117, `salidas.rs:527`), expulsión al cerrar, herencia a pueblos y avanzadas | provisional (decisión del 2026-10-22) |
| Memoria de la niebla por Facción y por héroe sin bandera | `nucleo::memoria::Memoria` (4H T3d) | lista para fundirse al crear o ingresar y en el traslado |
| Contrato | `proto/bronzeage/v1/`: las 28 órdenes (`orders.proto`), `Faccion` completa, `RelacionPolitica` con dirección/estado/`rota_por`/`paz_propuesta_por`, `Propuesta{Relacion,Anexion,Fusion}`, `Tributo`, `Titulo`, `Cargos` (`entities.proto:21-108, 176-182`), `GuerraDeclarada.arrastradas_ids` (`events.proto:133-135`) | ya definido; falta una forma pública de la Facción ajena (nombre, sigilo, reputación) |
| Autorización declarativa | `Comando::autoriza` → `enum Autoriza` (`comando.rs:395-417`: `MandaEnLaPlaza`, `CiudadanoDeLaMadre`, `LiderDeLaColumna`…) y la prueba que compara Proy §5.1 con el código | lista para añadir las variantes de cargo |

### 1.3 Andamiaje que el sistema 5 sustituye

| Andamiaje | Dónde | Lo sustituye | Tanda |
|---|---|---|---|
| `FijarFaccionDeHeroe` (administración) | `comando.rs:232`, `heroes.rs:325` | `crear_faccion`, `solicitar_ingreso`, `responder_solicitud`, `dejar_faccion` | 5B |
| `FijarResidencia` (administración) | `comando.rs:178`, `mercenarios.rs:325` | `cambiar_residencia`, `residir_en_campamento`, `dejar_residencia`; residir al fundar | 5B |
| Nivel fijado por la administración (`FijarNivelFaccion`) | `comando.rs:87`, `zonas.rs:234` | experiencia monótona y nivel derivado | 5C |
| `FijarRelacion` (administración) | `comando.rs:291`, `espacio.rs:306` | relaciones reales con propuesta, respuesta, guerra con arrastre, paz, ruptura | 5D |
| `manda_en` | `construccion.rs:389-393` (20 usos en 8 ficheros) | la matriz de Proy §5.2 (cargo, residente presente, Rey…) | 5E |
| «cualquier ciudadano presente» | `taberna.rs:8` (intel), avanzadas y pueblos «ciudadano de la madre» (P l.74-75, 114, 117), `reclutar` ya sigue la regla | Rey/Embajador/Gobernador presentes; Gobernador o Maestro de Obras presentes | 5E |
| cargo «dado por ocupado» | `construccion.rs:527-528` (`ActivarPolitica`), `murallas.rs:10, 160` (recintos), `AbandonarRecinto` sin cargo | el titular real del cargo | 5E |
| `FijarAcceso` (orden de jugador provisional) | `comando.rs:348`, `salidas.rs:527`, `fijar_puerta` traducida a él (`sesiones/ordenes/pruebas.rs:215`) | `fijarPuerta` por grupos + `vetarJugador` (según Q1) | 5E |
| `rey_admite_otras: false` | `batallas.rs:880-882` | `admitir_otras_facciones` | 5D |
| `DarAlmacenPersonal` (administración) | `comando.rs:180`, `mercenarios.rs:342` | `guardar_en_almacen_personal` / `sacar_del_almacen_personal` (según Q6) | 5G |
| `ColocarHeroe`, `CrearEjercito`, `DarSuministro` | `comando.rs:235, 79, 161` | no tienen orden de jugador equivalente para montar escenarios: se quedan como administración (decisión 9 de §7); el batch pasa a las órdenes de jugador en 5I | 5I |

### 1.4 Órdenes del contrato que hoy dan `orden.no_disponible`, y su tanda

| Tanda | Órdenes |
|---|---|
| **5B** (9) | `crear_faccion`, `solicitar_ingreso`, `responder_solicitud`, `dejar_faccion`, `asignar_rey`, `asignar_embajador`, `cambiar_residencia`, `residir_en_campamento`, `dejar_residencia` |
| **5C** (2) | `designar_capital`, `renombrar_asentamiento` |
| **5D** (9) | `proponer_relacion`, `responder_relacion`, `retirar_relacion`, `declarar_guerra`, `proponer_paz`, `romper_relacion`, `rebelion_vasallo`, `admitir_otras_facciones`, `alternar_reabastecer_aliados` |
| **5E** (2 + 1 que cambia de comando) | `asignar_cargo_local`, `vetar_jugador`; `fijar_puerta` pasa de `FijarAcceso` a `FijarPuerta` |
| **5F** (6) | `proponer_anexion`, `responder_anexion`, `retirar_anexion`, `proponer_fusion`, `responder_fusion`, `retirar_fusion` |
| **5G** (10, solo si Q6 = A) | refundación: `aportar_a_refundacion`, `retirar_de_refundacion`, `comprar_caravana_de_refundacion`, `lanzar_caravana_fundacion`, `desarmar_caravana_fundacion`; almacén personal: `guardar_en_almacen_personal`, `sacar_del_almacen_personal`; caravanas a mano: `cargar_caravana`, `reclamar_caravana`, `soltar_caravana` (esta última es `AparcarCaravana`: se suelta solo dentro de una plaza, decisión del 2026-10-19) |
| Sistema 6 (5) | `adoptar_tecnologia`, `empezar_epica`, `abandonar_epica`, `comprar_tecnologia_aeda`, `abrir_alijo` |

Las 28 de política quedan todas mapeadas al cerrar 5F; con Q6 = A, las 43 de `Comandos_Parametros.md` §«Órdenes sin comando» se reducen a las 5 del sistema 6. Cada tanda mapea las
suyas en `sesiones/ordenes.rs` en el mismo commit (la prueba de exhaustividad lo obliga) y actualiza la tabla `<!-- autorizacion -->` de Proy §5.1 (la prueba la compara con el código).
Los comandos nuevos van desde el **128**.

### 1.5 Códigos de evento pendientes del sistema 5 (bloques `<!-- eventos -->` y P l.162), y su tanda

| Tanda | Códigos |
|---|---|
| 5B | `faccion.creada`, `faccion.solicitud_ingreso`, `faccion.ciudadania_union`, `faccion.solicitud_denegada`, `faccion.abandonada`, `cargo.rey_asignado`, `cargo.embajador_asignado`, `ciudadania.residencia_cambiada`, `ciudadania.residencia_dejada`, `ciudadania.residencia_en_campamento` |
| 5C | `faccion.nivel_subio`, `faccion.capital_designada`, `asentamiento.renombrado` (este en el bloque de `Economia_Construccion.md`) |
| 5D | `diplomacia.relacion_propuesta`, `diplomacia.relacion_aceptada`, `diplomacia.relacion_rechazada`, `diplomacia.relacion_retirada`, `diplomacia.relacion_rota`, `diplomacia.guerra_declarada`, `diplomacia.paz_propuesta`, `diplomacia.paz_firmada`, `diplomacia.rebelion_vasallo`, `diplomacia.vasallo_liberado`, `diplomacia.vasallo_conquistado`, `diplomacia.tributo_pagado`, `faccion.admision_en_ataques`, `logistica.reabastecer_aliados` |
| 5E | `cargo.local_asignado`, `asentamiento.veto` (y `asentamiento.puerta_fijada`, decisión 8 de §7) |
| 5F | `diplomacia.anexion_propuesta`, `diplomacia.anexion_rechazada`, `diplomacia.anexion_retirada`, `diplomacia.anexion`, `diplomacia.fusion_propuesta`, `diplomacia.fusion_rechazada`, `diplomacia.fusion_retirada`, `diplomacia.fusion`, `titulo.nace`, `titulo.cambia_manos`; la crónica de la diplomacia y los títulos |
| 5G (Q6) | `expansion.caravana_lanzada`, `expansion.caravana_desarmada`, `fundacion.caravana_devuelta`, `fundacion.caravana_perdida`, `fundacion.caravana_vuelve`, `mercenarios.caravana_refundacion`, `mercenarios.fondo_refundacion`, `ejercito.caravana_cargada`, `ejercito.caravana_soltada`, `heroe.almacen_personal` (P l.164) |

Códigos solo de Rust que cambian: `faccion.guerra_declarada` → `diplomacia.guerra_declarada` (con `arrastradas`); `faccion.relacion` (de `FijarRelacion`) se retira o pasa a los
`diplomacia.*` del andamiaje; `asentamiento.acceso` → `asentamiento.puerta_fijada` (decisión 8). Son 41 códigos en total (los 37 de Dip, `asentamiento.renombrado`,
`logistica.reabastecer_aliados` y los dos de títulos ya contados), más los 10 de 5G.

---------------------------------------------------------------------------------------------------------------------------------------

## 2. Desajustes del Rust de hoy frente a Mechanics y el TypeScript

| # | Desajuste | Rust | Mechanics / TS | Tanda |
|---|---|---|---|---|
| D1 | No hay entidad Facción: el id es un `u32` que aparece en plazas, héroes y columnas. Sin nombre, sigilo, Rey, Embajador, experiencia, ciudadanos **en orden de ingreso**, solicitudes, `derrotadaPor`, `admiteOtrasEnAtaques`, `capitalDesignadaEn` | `estado.rs:790`; `propios.rs:426-438` deriva los ciudadanos filtrando héroes (sin orden) | Dip §2.1 | 5B |
| D2 | Una Facción sin plazas ni héroes no existe; una Facción sin plazas solo existe por sus héroes | `proyeccion/audiencia.rs:117` (`facciones(e)` la deduce) | Dip §8: una Facción derrotada no desaparece | 5B |
| D3 | Relaciones por pareja sin dirección (no se sabe quién es la señora ni quién declaró), sin `estado` (una rota se borra), sin `creadoEn`, `rotaPor`/`rotaEn`, `pazPropuestaPor` ni tributo; id de relación derivado del par (`proyeccion/ids.rs:38`), que no admite dos relaciones del mismo par en el tiempo | `estado_militar.rs:109-119, 175, 228-231`; `propios.rs:449` | Dip §2.3, TS `RelacionPolitica` | 5D |
| D4 | `atacar` con `declara_guerra` **sustituye un vasallaje** por la guerra | `combates.rs:315-317, 329` | `validarPar` (Dip §5.2): con cualquier relación activa no se declara; la vasalla se rebela y la señora libera | 5D |
| D5 | Guerra sin arrastre por vasallaje (`Detalle::Guerra.arrastre` vacío; `GuerraDeclarada.arrastradas_ids` vacío) | `combates.rs:320-322`, `proyeccion/eventos.rs:231` | Dip §5.4 (`declarar`) | 5D |
| D6 | Atacar a un aliado se rechaza (`combate.aliada`) | `combates.rs:315` | TS: asedio a un aliado con `penalizacionAtacarAliado` (`combate.ts:551-557`, `resultadoBatalla.ts:323`); ver C3 | 5D (Q2) |
| D7 | La reputación solo la mueven los trueques; `alianzas` del ritmo no se alimenta de relaciones | `mercado.rs:252`; `comercio/src/reputacion.rs:53-80` | Dip §3.6, §5.4-§5.5, §5.9 | 5D |
| D8 | Sin experiencia de Facción: `NIVEL_FACCION` vacío; el trueque cumplido devuelve las Facciones que la ganan y se descarta; `Informe.digno` sin consumidor | `M/Combate.md:333-336`; P l.103, 131, 134 | G2 §2.2.1, Dip §3.5, `Combate.md:122-127, 152, 179`, `Batalla_Unity_Ciclo.md:292` | 5C |
| D9 | Huecos del `Exterior` de la economía: `CIUDADANOS = 1` (< `nobleza.minCiudadanos` = 3: **ninguna plaza tiene nobleza**), `DISTANCIA_CAPITAL = 0`, `ES_CAPITAL = true`, `FACCION_TIENE_GRAN_FUNDICION = false` | `construccion.rs:51-55` | `Economia_Construccion.md:406, 1715`; G2 §2.2 (capital); G4 (Gran Fundición una por Facción, en la capital) | 5B (ciudadanos, Q8), 5C (capital) |
| D10 | Derrota solo como hecho: sin `derrotadaPor`, sin anularla al fundar, sin liberar vasallos ni pasar la vasalla al conquistador | `construccion.rs:390`, `conquistas.rs:31` | Dip §5.5, §8 | 5C, 5D |
| D11 | `manda_en`: manda en una plaza la administración, el propietario de una columna de su Facción o la cuenta de un héroe ciudadano | `construccion.rs:392` | Proy §5.2.4-§5.2.5: residente presente o titular del cargo (`cargo(A,c)` = reside ∧ presente ∧ titular) | 5E |
| D12 | Cargo dado por ocupado al activar políticas y al comprometer o mejorar recintos; abandonar un recinto sin cargo | `construccion.rs:527-528`, `murallas.rs:10, 160` | Proy §5.2.5 | 5E |
| D13 | Puerta por relación con tres interruptores y por defecto **abierta a neutrales**; sin grupo `aedas` ni vetos | `estado_militar.rs:144-148`, `salidas.rs:527` | Dip §4.3 (`PUERTA.cerradaAPorDefecto` = neutrales y enemigos; capas residente > veto > grupo), Pres §8 | 5E (Q1) |
| D14 | Residencia por administración, sin cooldown, sin las tres órdenes; el fundador no pasa a residir en la plaza que funda; `Fundar` no deshace la columna dentro | `mercenarios.rs:325`; P l.70, 98, 100 | Dip §4.1-§4.2; `Fundacion_Y_Refundacion.md` §5.5-§5.6 | 5B |
| D15 | Intel de la taberna en una plaza: la compra cualquier ciudadano dentro | `taberna.rs:8` | G5 l.519; Proy §5.2.6 (`compraIntel`: Rey, Embajador o Gobernador presentes) | 5E |
| D16 | Pueblos y avanzadas los manda un ciudadano de la madre (o el propietario de una columna) | P l.74-75, 114, 117; Proy §5.1 («ciudadano de la madre») | G4 l.81 (Gobernador o Maestro de Obras presentes), G5 l.883 (la tropa de la avanzada la elige el Gobernador) | 5E |
| D17 | `rey_admite_otras` siempre falso; «aliada para batalla» = el stub | `batallas.rs:880-882`; P l.109 | G2 l.17, Dip §3.4 | 5D |
| D18 | Repostar solo en la plaza propia | `uniones.rs:259`; P l.139 | G5 l.554 (`permiteReabastecerAliados`, lo abre el Gobernador o el Tesorero) | 5D |
| D19 | Un solo rol de administración | Proy l.65 (decisión del 2026-10-24, «hasta que el sistema 5 traiga las membresías») | Proy §2.2: `moderador` sin órdenes de partida; `administrador_partida`/`global` solo `cancelarBatalla` | 5E |
| D20 | Plazas sin nombre; sin `renombrarAsentamiento` | P l.70, 119 | `Fundacion_Y_Refundacion.md` §8 (`nombreDeCiudadLibre`, FNV-1a, 159 nombres); Proy §5.2.4 | 5C |
| D21 | Proyección incompleta: `Faccion` sin nombre, sigilo, Rey, Embajador, experiencia, solicitudes; relación sin dirección; interior sin cargos ni nombre; no hay `Propuesta`, `Titulo` ni la ficha pública de una Facción ajena | `propios.rs:8, 426, 449`, `interior.rs:11`; P l.158 | contrato 05 | 5H |
| D22 | `dejarFaccion` en el TS deja al héroe **sin residencia** (`dejarFaccion.ts` llama a la `dejarResidencia` del motor, que no le busca campamento) | — | G0 l.70: quien deja su casa pasa al campamento más cercano; Dip §3.3 remite a §4.2 | 5B (decisión 10 de §7) |
| D23 | El TS no impide que un participante de un ejército deje la Facción (`conFaccionEnSuColumna` solo cambia la columna si va solo, `ayudas.ts:203-208`): el ejército queda con alguien de otra Facción | — | G0 l.42: «todos son de la misma Facción» | 5B (Q7) |

---------------------------------------------------------------------------------------------------------------------------------------

## 3. Contradicciones entre canon y Mechanics (y dentro de cada uno)

Leyenda: **R** = la resuelve la regla de precedencia (gana Mechanics, se corrige el canon) · **P** = pregunta al autor · **I** = corrección de documento sin impacto de diseño.

| # | Qué | Dónde | Resolución y corrección propuesta |
|---|---|---|---|
| C1 | Puerta por defecto: el canon y el bloque `PUERTA` cierran a neutrales y enemigos; la decisión provisional del 2026-10-22 abre a todos menos a los enemigos «hasta el sistema político» | G1 l.274 frente a G1 l.260 y la nota de l.274; Dip l.344, 419-427 frente a Dip l.352, Pres l.48, l.103; `Entrar_Con_Todo_Definicion.md` §5 | **P (Q1)**. Con la respuesta se reescriben G1 1.10.3-1.10.5, Dip §4.3, Pres §4.1 y §8 y el bloque `PUERTA` |
| C2 | Cómo se cierra la puerta: por grupos con `aedas` y vetos individuales (`fijarPuerta`, `vetarJugador`) o por relación con tres interruptores (`FijarAcceso`) | Dip l.345-347, Pres l.163-165, G1 l.274-278, G2 l.130 frente a Pres l.48, Dip l.352 | **P (Q1)** |
| C3 | Atacar a un aliado: G2 2.7 la cuenta como «la más severa» de las penalizaciones y `Combate.md` §6 la aplica al asedio; pero «toda agresión exige guerra o la declara» y `validarPar` no deja declararla con una alianza activa, y `Combate.md` §5 la prohíbe entre columnas | G2 l.115, `Combate.md:151` frente a G2 l.62, l.66, Dip l.451, l.463, `Combate.md:131` | **P (Q2)** |
| C4 | Tributo: «ritmo de flujo continuo» con eventos al agotarse o llenarse frente a «evento recurrente con su periodicidad»; y la economía de las plazas ya no es un ritmo sino un paso de minuto | Dip §5.6 (l.491) frente a `Reloj_Y_Eventos.md:133`; `Reloj_Y_Eventos.md:46, 110` | **P (Q3)** |
| C5 | Quién cuenta para la nobleza: `casasCompradas.length` (no cuenta a los fundadores) frente a «cantidad mínima de ciudadanos (héroes) en el asentamiento»; desde D31 no hay compra de casa | `Economia_Construccion.md:406` frente a G4 l.10, G2 l.70 | **P (Q8)** |
| C6 | El designado de un cargo local «no tiene que residir (el motor no lo mira)», pero solo ejerce quien reside y está presente, y el cargo se vacía «al dejar la residencia», cosa que no le pasa a quien nunca residió | Dip l.86-87, l.89 frente a G2 l.22-23 y Proy §4.3 (`cargo(A, c)`) | **P (Q4)** |
| C7 | La vasalla conquistada pasa al conquistador «en el tick siguiente» | G2 l.58, Dip l.487-488 | **I**: en el instante en que cae su última plaza, después del desarme del señor (Dip l.485 ya lo ordena así). Se corrigen los dos textos |
| C8 | «Solo marca el estado (la puerta, `enemigos`); atacar no exige guerra» | Dip l.474 frente a Dip l.463 y G2 l.66 | **I**: resto de antes del 2026-10-11; se borra la frase |
| C9 | `NIVEL_FACCION` «está en `Economia_Construccion`»; no está en ningún bloque, y el de `Combate.md` está vacío | Dip l.16, `Comercio_Caravanas.md:804`, `Combate.md:333-336` | **I**: se escribe el bloque en Dip §3.5 con las cifras del canon (G2 l.33 = `constants.ts:1494-1534`: curva `[100, 180, 300, 480, 760, 1200, 1900, 3000, 4800]`, `xp` combate 2,5, edificio 1, conquista 10, ascenso 30 por nivel, fundación 20, trueque 2, bandidos 1, `nivelSinXpBandidos` 5, `ratioCombateDigno` 0,5) y se quita el vacío |
| C10 | «La caducidad de las solicitudes es un evento»: no hay plazo; `sinSolicitudesCaducadas` solo limpia las de quien ya es ciudadano | Dip l.73 | **I**: «Las solicitudes no caducan; al ganar la ciudadanía en una Facción se borran de todas» |
| C11 | «Comprar una segunda casa se rechaza» | G2 l.83 | **I**: resto de antes de D31; «La residencia es un solo sitio a la vez» |
| C12 | Ruptura por rebelión: «cancelación inmediata de todos los acuerdos comerciales/tratados vigentes» frente a «los trueques `activo` entre plazas de las dos Facciones pasan a `expirado` sin penalización» | G2 l.56 frente a Dip l.481 | **R**: Mechanics (más preciso); se reescribe G2 2.4 |
| C13 | Dejar la Facción: Dip §3.3 dice que deja la residencia y remite a §4.2, que lo lleva al campamento más cercano, pero el TS no lo lleva a ninguno | Dip l.77-78; G0 l.70; `dejarFaccion.ts` | **R** (G0 y Dip §4.2 coinciden): campamento más cercano; se precisa Dip §3.3 y se anota la divergencia del TS |
| C14 | El evento de fijar la puerta: `asentamiento.puerta_fijada` en Dip §9; `asentamiento.acceso` en el catálogo de Pres | Dip l.586 frente a Pres l.308 | **I**: `asentamiento.puerta_fijada` (decisión 8) |
| C15 | Títulos «se recalculan cuando cambia lo que miran y como muestreo de baja frecuencia» sin cadencia; «Mayor poder económico» mira el oro, que cambia en cada paso de minuto de cada plaza | Dip l.564, `Reloj_Y_Eventos.md:136` | **I** con cifra provisional (decisión 12) |
| C16 | `Proy` l.185 dice que la matriz de §5.1 es provisional y §5.2 «lo que pedirá el sistema 5», pero §5.2.8 («cualquier participante puede estacionar») choca con la decisión de la 4B-2 (lo da el Líder) | Proy l.501-505 frente a Pres l.26 | **R**: queda lo de Rust (decisión del sistema 4); se anota en §5.2.8 que no se adopta |

---------------------------------------------------------------------------------------------------------------------------------------

## 4. `// DUDA:` del código y filas de paridad que cierra cada tanda

### 4.1 `// DUDA:` que esperan al sistema 5 (`git grep -n "DUDA" -- '*.rs'`, 49 en total, 15 de política)

| DUDA | Tanda | Cómo se cierra |
|---|---|---|
| `estado_militar.rs:111` — la relación no distingue señor de vasallo | 5D | `RelacionPolitica` con `a` = señora / quien declaró |
| `nucleo/batallas.rs:881` — «el Rey admite otras Facciones» siempre falso | 5D | `admite_otras_en_ataques` de la Facción atacante |
| `nucleo/uniones.rs:259` — repostar sola en la zona (la parte de las aliadas) | 5D | `puedeRepostarEn` con `permiteReabastecerAliados`; la parte de la zona no es del sistema 5 y sigue abierta |
| `nucleo/construccion.rs:389` — `manda_en` provisional | 5E | se retira `manda_en` |
| `nucleo/construccion.rs:527` — el cargo de `ActivarPolitica` se da por ocupado | 5E | titular real (`cargo_ocupado` real en `Activador`) |
| `nucleo/murallas.rs:10` y `:160` — cargo de recintos por ocupado; abandonar sin cargo | 5E | Gobernador/Maestro de Obras titulares; abandonar, Gobernador |
| `nucleo/flota.rs:184` — `MoverCargaAparcada` sin «residente del origen presente donde está aparcada» | 5E | `puedeOperarAparcada` |
| `nucleo/taberna.rs:8` — la intel la compra cualquier ciudadano dentro | 5E | `compraIntel` |
| `nucleo/salidas.rs:555` — capacidad del almacén personal en una plaza | 5B | se cierra sin cambio: G0 l.73 y G2 l.78 dan 1000 en total, viva donde viva |
| `proyeccion/interior.rs:11` — cargos y nombre de la plaza vacíos | 5H | |
| `proyeccion/propios.rs:8` (la parte de la Facción) y `:426` — Facción sin nombre, sigilo, Rey, Embajador, experiencia | 5H | |
| `proyeccion/propios.rs:449` — relación sin dirección ni fecha | 5H (tras 5D) | |
| `sesiones/ambitos.rs:65` — cuenta con solo héroes de campo (`CrearEjercito`) | 5I | se queda con el andamiaje (decisión 9); se revisa al pasar el batch a órdenes de jugador |

No son del sistema 5 (se dejan): `batallas.rs:266` (`gameId`), `cronica.rs:12` (la conquista por Unity), `mercenarios.rs:111` y `sesiones.rs:260` (`eligieronComoInicial`),
`salidas.rs:562` (interior recordado), las de trazado, comercio, proyección de pueblos y avanzadas y sesiones.

### 4.2 Filas de paridad (`Docs/Mecanicas a desarrollar.md`) que cierra o reescribe cada tanda

| Tanda | Filas (línea de hoy) |
|---|---|
| 5A | l.134 (escribir `NIVEL_FACCION`) |
| 5B | l.70 (fundadores, la columna se deshace al fundar), l.85 (residencia mínima), l.100 (`FijarFaccionDeHeroe`, `FijarResidencia`, el fundador no reside), l.121 (ciudadanos fijos en 1), l.129 (la parte de `FijarResidencia`), l.155 (almacén personal en la plaza) |
| 5C | l.70 (nombre de ciudad, XP de fundación, anulación de la derrota), l.83 (la Facción sin plazas no se derrota), l.96, l.103, l.106 y l.108 (XP de combate, conquista y bandidos; `registrarDerrota`), l.119 (`renombrarAsentamiento`), l.120 (capital, distancia, Gran Fundición), l.122 (XP de edificio y ascenso), l.131 (cifra del trueque) |
| 5D | l.91, l.97, l.104 (relaciones, arrastre, vasallaje, atacar aliado), l.96 y l.108 (la reputación), l.109 (`rey_admite_otras`, aliada para batalla), l.135 (reputación con sucesos de diplomacia y alianzas activas), l.139 (la parte de las aliadas) |
| 5E | l.74, l.75, l.114, l.117 (pueblos y avanzadas por cargo), l.98, l.141, l.145, l.153, l.154 (puerta y vetos), l.102 (vaciar cargos al reubicar), l.110 (revisar el candado), l.119 y l.136 (autorización por cargo, residencia y presencia), l.142 y l.146 (revisar la presencia) |
| 5F | l.138 (anexión o fusión a mitad de trueque, Q5) |
| 5G (Q6) | l.71 (Caravana de Fundación), l.87 (`reclamarCaravana`, `cargarCaravana`, `soltarCaravana`), l.129 y l.140 (`DarAlmacenPersonal`, reclamar), l.164 (sus eventos) |
| 5H | l.158 (Facción, plaza y relaciones en el contrato; las 28 órdenes) |
| 5I | l.156 (ciudadanía, residencia, alianzas y nivel de Facción en el batch) |
| Cada una | l.162 (los eventos de su tanda, tabla §1.5) |

Filas nuevas que el sistema 5 abriría si recorta (a anotar en el momento): la Liga como dato derivado sin estandarte colectivo (sin cliente), el título «Mecenas de los Aedas»
a 0 hasta el sistema 6, y los Aedas residentes que miran la reputación (sistema 6).

---------------------------------------------------------------------------------------------------------------------------------------

## 5. Descomposición en tandas

Patrón de los sistemas 3 y 4: reglas puras en un crate propio, luego el núcleo por piezas (cada una con su foto si cambia el estado), luego contrato y proyecciones, y al final el batch.
Tamaños en líneas de Rust con pruebas. Referencia: el TS de política son ≈ 3.600 líneas de reglas y órdenes y ≈ 3.400 de pruebas; con la proporción del sistema 4 sale
**≈ 15-19.000 líneas y 9 commits** (≈ 2.500 más si Q6 = A).

Crate nuevo **`crates/politica`**, sin dependencia del núcleo (como `comercio`, `militar` y `combate`), con `datos.rs` que lee los bloques de Dip con `include_str!`.
La reputación se queda en `comercio::reputacion` (ya tiene todos los sucesos): `politica` devuelve `Suceso`s y el núcleo los aplica.

| Tanda | Contenido | Tamaño | Depende de | Foto |
|---|---|---|---|---|
| **5A `politica`** (reglas puras) | Datos: `CIUDADANIA`, `CAPITAL`, `PUERTA`, `RELACION_PROPUESTA`, `ANEXION`, `FUSION`, `CATALOGO_SIGILO` y el bloque nuevo `NIVEL_FACCION` (C9, en milésimas). Modelos `Faccion`, `RelacionPolitica`, `Propuesta` (las tres clases en un solo almacén), `Cargos`, `Puerta`. Reglas: crear (nombre único sin mayúsculas, sigilo del catálogo y no duplicado, 1 héroe 1 Facción, cooldown de creación), solicitar/responder/dejar con la sucesión del trono por orden de ingreso, Rey/Embajador/cargos locales (quién puede ser designado), nivel por experiencia y aplicar experiencia (monótona), cap y cupo (se mueven aquí desde sus llamadores), capital efectiva y designación con cooldown, sigilo (`valido`, `duplicado`, `sigilo_libre` FNV-1a), puerta por capas (residente > veto > grupo; la propia Facción nunca), `validar_par`, proponer/responder/retirar con revalidación, `declarar` con arrastre, paz, romper, rebelión, desarme del señor y vasalla conquistada, reparto del tributo, Liga y Gran Rey (componentes conexas, bajo demanda), títulos, el **plan** de traslado (`trasladar_faccion` como lista de cambios puros) y los predicados de la matriz (`ciudadano`, `rey`, `rey_o_embajador`, `residente`, `presente`, `reside`, `cargo`, `autoridad_diplomatica`…) sobre una vista que da el núcleo. Pruebas: los casos de `faccion`, `cargos`, `capital`, `sigilo`, `puerta`, `diplomaciaDecisiones`, `vasallaConquistada`, `experienciaFaccion` (la parte de reglas) y `titulos` del TS | L, ≈ 3.000 | — | — |
| **5B núcleo: Facción, ciudadanía y residencia** | `Estado.politica` con las Facciones (entidad nueva, ids `u32` de siempre), `salidas_faccion` y `cambios_residencia` por héroe; codec y hash. Órdenes `CrearFaccion` (funde la memoria del héroe sin bandera; su columna toma la Facción), `SolicitarIngreso`, `ResponderSolicitud`, `DejarFaccion` (deja residencia, cargos de Facción y guarnición; al campamento más cercano, decisión 10; Q7), `AsignarRey`, `AsignarEmbajador`, `CambiarResidencia`, `ResidirEnCampamento`, `DejarResidencia` con cooldown (instantes guardados, sin eventos). `Fundar`: los fundadores (ciudadanos de la Facción de la caravana en la columna, hasta 5) pasan a residir y la columna se deshace dentro. `Exterior.ciudadanos` real según Q8 (del índice de residentes: O(1) por paso). `FijarFaccionDeHeroe` y `FijarResidencia` pasan a ser administración sobre estas mismas reglas (decisión 9). Pruebas: `comandosMembresiaFaccion`, `comandosResidencia`, `residenciaMercenaria`, `faccionDeLaColumna`, `sigiloFaccion`, `eventosDominioFaccion`; foto ida y vuelta, determinismo normal/acelerado/a trozos | XL, ≈ 3.000 | 5A | **BAN26** |
| **5C núcleo: experiencia, nivel, capital, derrota y nombre** | Experiencia de Facción en milésimas al ocurrir cada hecho: combate digno por héroe participante (números) y sin filtro en Unity, conquista, bandidos (÷ nivel, 0 desde 5), edificio completado, ascenso (30 × nivel), fundación (no la primera), trueque (2 a cada lado); `faccion.nivel_subio`; el nivel derivado sustituye a `niveles_faccion` (cap de fundación, cupo 2/3, huecos de política) y `FijarNivelFaccion` fija la experiencia al umbral (decisión 9). `DesignarCapital` (Palacio activo, cooldown de 14 días, sin cooldown si se perdió) y capital efectiva calculada al paso (plazas de la Facción ≤ 7): `es_capital`, `distancia_capital` y la Gran Fundición única entran en el `Exterior`. `registrarDerrota`/`derrotadaPor` al caer la última plaza (conquista, Unity, ruina) y se borra al fundar. Nombre de ciudad al fundar (`nombreDeCiudadLibre`) y `RenombrarAsentamiento` (residente presente, Proy §5.2.4). Pruebas: `experienciaFaccion`, `capital`, `derrotas`, `eventosDominioPoliticas`; paso de minuto sin empeorar | L, ≈ 2.000 | 5B | **BAN27** |
| **5D núcleo: diplomacia** | `RelacionPolitica` y propuestas en el estado (sustituyen al mapa de pares; los consumidores —encuentros, defensa, puerta, unirse a batallas, visión compartida, tipo A/B de caravana— pasan a los predicados de 5A). Órdenes `ProponerRelacion`, `ResponderRelacion`, `RetirarRelacion`, `DeclararGuerra`, `ProponerPaz`, `RomperRelacion`, `RebelionVasallo`, `AdmitirOtrasFacciones`, `AlternarReabastecerAliados`. `Atacar` con `declara_guerra` usa el mismo `declarar` (arrastre real en `Detalle` y en el hecho; no sustituye un vasallaje: D4) y la regla de Q2. Reputación: romper alianza, liberar, rebelión (trueques entre las dos a `expirado`), guerra tras romper (ventana de 3 días), atacar aliado (Q2), defender/no defender al vasallo con números y en Unity, alianzas activas en el ritmo. Desarme del señor y vasalla conquistada **en el instante de la caída** (C7). Tributo según Q3. `rey_admite_otras` y `puedeRepostarEn` con aliadas. `FijarRelacion` queda como administración que crea o rompe una relación real (decisión 9). Pruebas: `comandosDiplomacia`, `eventosDominioDiplomacia`, `reputacionVasallos`, `vasallaConquistada`, la parte de guerra de `batallas`; prueba de conservación del tributo | XL, ≈ 3.500 | 5C | **BAN28** |
| **5E núcleo: cargos, autorización, puerta y roles** | `Cargos` por plaza; `AsignarCargoLocal` (el Gobernador lo nombra el Rey sin presencia; los otros cuatro, el Gobernador residente y presente; Q4); los cargos se vacían al dejar la residencia, al dejar la Facción, en la conquista, la ruina y el traslado. **Se retira `manda_en`**: cada orden de plaza pasa a su fila de Proy §5.2 (`Autoriza` gana `ResidentePresente`, `Cargo(..)`, `ReyDeLaFaccion`, `AutoridadDiplomatica`… y pierde `MandaEnLaPlaza` y `CiudadanoDeLaMadre`) y la tabla de Proy §5.1 se reescribe entera; el `cargo` de políticas y recintos es el titular; intel (`compraIntel`); pueblos y avanzadas por Gobernador o Maestro de Obras presentes; `MoverCargaAparcada` (`puedeOperarAparcada`). Puerta según Q1 (`FijarPuerta` por grupos con `aedas`, `VetarJugador` y la expulsión de quien ya no puede entrar, decisión 16). Roles técnicos por membresía (D19). Pruebas: `comandosCargos`, `cargos`, `autorizacionComandos` (las 145 órdenes, también fail-open), `puerta`, la parte de puerta de `comandosPresencia`; repetir las suites de sesión 20 veces | XL, ≈ 3.000 (la mitad, pruebas) | 5D | **BAN29** |
| **5F núcleo: anexión, fusión, traslado y títulos** | Propuestas de anexión y fusión (el almacén de 5D, sin foto nueva) y sus seis órdenes con revalidación y bloqueo por batalla abierta; `trasladar_faccion` aplicado en una sola transición: plazas con cargos locales y guarnición, ciudadanos con residencia, columnas, caravanas y flota, rutas, pueblos, avanzadas, convocatorias, Miradas e Informes, memoria fundida, tecnologías del andamiaje (unión en la fusión; sistema 6 hará el resto), reputación y experiencia (máximas en la fusión), relaciones y propuestas; trueques según Q5. Liga, Gran Rey y **títulos** (estado nuevo: los últimos títulos para narrar el cambio; cadencia, decisión 12). Crónica de anexión, fusión, liberación de vasallos y títulos. Pruebas: `comandosAnexion`, `comandosFusion`, `zonas_fusionadas`, `eventosDominioTitulos`; que el traslado no pierda ni duplique ninguna entidad (auditoría) | L, ≈ 2.500 | 5E | **BAN30** |
| **5G núcleo: refundación, almacén personal y caravanas a mano** (solo si Q6 = A) | Caravana de Fundación de verdad: `LanzarCaravanaFundacion` (ciudadano, nivel ≥ 2, cooldown compartido, cap reservado al lanzar), caducidad de 48 h como cita, vuelta sola y desarme, `DesarmarCaravanaFundacion`; fondo de refundación del campamento (`Aportar`/`Retirar`/`ComprarCaravanaDeRefundacion`, solo Facción sin plazas); `GuardarEnAlmacenPersonal`/`SacarDelAlmacenPersonal`; `CargarCaravana`, `ReclamarCaravana`, `soltar_caravana` → `AparcarCaravana`. `DarAlmacenPersonal` y `DarCaravanaFundacion` quedan como administración. Pruebas: las de `expansion`, `refundacion` y `entrarSalirCampamento` del TS | L, ≈ 2.500 | 5C (sin plazas), 5B | **BAN31** |
| **5H contrato y proyecciones** | `Faccion` entera en `FACCION` (nombre, sigilo, Rey, Embajador, capital, experiencia, ciudadanos en orden, solicitudes, `derrotada_por`, `admite_otras_en_ataques`); `RelacionPolitica` con dirección, estado y fechas; `Propuesta` (clase nueva del registro de cambios) solo a las dos Facciones; `Titulo` público; `Cargos`, nombre, capital, vetos, `cerrada_a` y `permite_reabastecer_aliados` en el interior; **forma pública de la Facción ajena** en `VISTA` (nombre, sigilo, reputación: el score es público, G2 l.127) como campo aditivo; las solicitudes propias del solicitante en `HEROE`; eventos tipados de la diplomacia; dorados en Rust, C# y TS; `Comandos_Parametros.md` sin el grupo «Política». Sin cambio de foto | M, ≈ 1.500 | 5F (5G) | — |
| **5I batch político** | `batch politica` (como `batch militar`): bots que crean Facciones, piden ingreso y las aceptan, se mudan, designan Gobernador y Embajador, proponen alianzas y vasallajes con tributo, declaran guerras y firman paces, se rebelan y, de vez en cuando, se anexionan o fusionan. Invariantes orden a orden: un héroe en una Facción; toda Facción con ciudadanos tiene Rey; la sucesión; cargos locales solo de residentes; una relación activa por pareja y una propuesta pendiente de cada clase; el arrastre; conservación del tributo; el traslado no pierde entidades; experiencia monótona; reproducción y foto ida y vuelta a mitad de corrida. El batch de comercio y el militar pasan a las órdenes de jugador para ciudadanía, residencia y relaciones (decisión 9). **Re-medir población** por el cambio de `ciudadanos` y de la distancia a la capital (riesgo R1) | M, ≈ 2.000 | 5H | — |

Notas de orden: 5B va antes que 5C porque la derrota y la capital necesitan la Facción como entidad; 5D antes que 5E porque los grupos de la puerta y la autorización diplomática
necesitan la relación con dirección; 5E antes que 5F porque el traslado mueve cargos. 5G puede ir en paralelo a 5D-5F si lo hace otra sesión (solo toca fundación, campamentos y columnas).

### 5.1 Pruebas comunes a todas las tandas del núcleo

`cargo test --workspace --no-fail-fast` en verde; determinismo normal, acelerado y a trozos; foto ida y vuelta con el formato nuevo (y la anterior rechazada con su motivo); ninguna
entidad ociosa programa eventos (una Facción, una propuesta o una relación sin nada que esperar no deja nada en la cola); paso de minuto sin empeorar
(`cargo test --release -p nucleo --test asentamientos rendimiento -- --ignored`, 1.000 plazas × 3 días); la prueba del catálogo de eventos y la de la matriz de §5.1; notas
«Implementado en Rust (fecha, tanda)» en Dip, Pres, Proy y los Mechanics tocados; filas de paridad cerradas o reescritas.

### 5.2 Riesgos

- **R1. La economía cambia sola.** Con `ciudadanos` real aparece la nobleza en plazas con 3 residentes o más (hoy nunca), y con la distancia a la capital sube el mantenimiento de las
  plazas lejanas. Las cifras de `batch poblacion` del 2026-10-24 dejan de valer: hay que volver a medir en 5I (o antes si el autor lo pide).
- **R2. Las pruebas de hoy usan el andamiaje.** Cientos de pruebas montan escenarios con `FijarFaccionDeHeroe`, `FijarRelacion`, `FijarResidencia` y `manda_en`. La decisión 9 los
  conserva como administración para que 5B-5E no reescriban medio árbol de pruebas; las que comprueban permisos de jugador sí cambian en 5E.
- **R3. La matriz en 5E toca casi todas las órdenes de plaza.** Es la tanda con más riesgo de regresiones en la sesión: repetir las suites de sesión (20 veces y 4 procesos a la vez),
  como en 4H.

---------------------------------------------------------------------------------------------------------------------------------------

## 6. Preguntas al autor (8)

**Q1. La puerta de una plaza cuando exista la política.**
El canon (Doc 1.10.5) y el bloque `PUERTA` dicen: se cierra por grupos (neutrales, aliados, enemigos y los Aedas), el Gobernador puede además vetar a un jugador concreto, y una
plaza que nadie ha tocado deja entrar a los suyos y a los aliados y **cierra a neutrales y enemigos**. Lo provisional del 2026-10-22 (`FijarAcceso`) decide por relación con tres
interruptores, sin Aedas ni vetos, y por defecto **deja entrar también a los neutrales** «hasta el sistema político».
- A) Lo del canon: `fijarPuerta` por grupos (con `aedas`) y `vetarJugador`; por defecto cerrada a neutrales y enemigos; cerrar o vetar expulsa a quien ya no puede entrar.
- B) Lo del canon pero con el defecto provisional: por defecto solo cerrada a enemigos (se corrige el bloque `PUERTA` y el canon).
- C) Se queda `FijarAcceso` (tres interruptores, sin Aedas) y solo se añaden los vetos.
Recomendada: **A** (es lo escrito en canon y Mechanics y lo provisional se anunció «hasta el sistema político»; el veto y el grupo de los Aedas ya tienen comando en el contrato).

**Q2. Atacar a un aliado.**
Doc 2.7 cuenta «atacar a un aliado sin romper la relación» como la penalización más severa (−25) y `Combate.md` la aplica al asedio; pero desde el 2026-10-11 toda agresión exige
guerra o la declara, y con una alianza activa no se puede declarar la guerra. Hoy Rust lo rechaza (`combate.aliada`).
- A) Se rechaza siempre: hay que romper la alianza antes; `penalizacionAtacarAliado` desaparece.
- B) Atacar con `declara_guerra` a un aliado es una traición: rompe la alianza en el acto, cobra −25 (en vez de los −12 de romper) y declara la guerra con su arrastre.
- C) Se permite atacar al aliado sin guerra, con −25 (como el TypeScript), como excepción a «toda agresión exige guerra».
Recomendada: **B** (conserva las dos reglas tuyas: la penalización más severa y que toda agresión deja a las dos en guerra; el cliente avisa con `guerra.confirmar`).

**Q3. El tributo del vasallaje: cuándo se paga.**
`Diplomacia_Facciones` §5.6 lo describe como un flujo continuo; `Reloj_Y_Eventos` como un evento recurrente; y la economía de las plazas es ahora un paso de minuto, no un ritmo.
- A) En el paso de minuto de cada plaza vasalla: paga su parte (`min(stock, cantidadPorMinuto ÷ nº de plazas)`) a la capital de la señora, hasta su capacidad. Sin eventos nuevos.
- B) Un evento recurrente por vasallaje cada minuto (prioridad 9) que cobra a todas las plazas de la vasalla a la vez, como el TypeScript.
- C) Ritmo continuo entre almacenes (exige que el almacén vuelva a ser un ritmo para esos recursos).
Recomendada: **A** (es como funciona ya la economía, no añade nada a la cola y una plaza sin vasallaje no cuesta nada; el hecho `diplomacia.tributo_pagado` sale por plaza que paga).

**Q4. ¿Puede ocupar un cargo local quien no reside en la plaza?**
Mechanics dice que el designado no tiene que residir, pero solo ejerce quien reside y está dentro, y el cargo se vacía «al dejar la residencia», cosa que nunca le pasa a quien no residía.
- A) Hay que residir en la plaza para ser designado; el cargo se vacía al dejar de residir.
- B) Como el TypeScript: se puede designar a cualquier ciudadano; no ejerce hasta que resida y esté dentro.
- C) Se puede designar a cualquier ciudadano y ejerce estando dentro aunque no resida (se cambia la matriz).
Recomendada: **A** (un cargo que no puede ejercerse ni se vacía nunca es un hueco; «la ciudadanía habilita, la presencia ejerce»).

**Q5. Trueques en curso cuando dos Facciones se anexionan o se fusionan** (fila de paridad «sin decidir»).
- A) Siguen como están: van entre plazas, y las plazas cambian de Facción; los que quedan entre plazas de la misma Facción ya no mueven la reputación al cumplirse o vencer.
- B) Los trueques entre las dos Facciones que se unen se anulan sin penalización (como en la rebelión); los demás siguen.
- C) Se anulan todos los de la Facción que desaparece, sin penalización.
Recomendada: **A** (es lo que hacía el TypeScript sin decirlo y no destruye nada; la reputación ya ignora los trueques internos).

**Q6. ¿Entran en el sistema 5 la refundación, el almacén personal y las caravanas a mano?**
Son 10 órdenes del contrato que hoy dan `orden.no_disponible` fuera del grupo «política». La refundación necesita la ciudadanía y la Facción sin plazas; el almacén personal, la
residencia; las caravanas a mano son deuda del sistema 4.
- A) Las tres, en una tanda propia (5G), y el contrato queda solo con las 5 órdenes del sistema 6 sin hacer.
- B) Solo la refundación y el almacén personal; las caravanas a mano quedan como deuda de columnas.
- C) Ninguna: el sistema 5 son las 28 de política.
Recomendada: **A** (dependen de lo que trae el sistema 5 y cierran la paridad de la Caravana de Fundación; son ≈ 2.500 líneas).

**Q7. Dejar la Facción yendo dentro de un ejército** (o en una convocatoria con otros).
Un ejército es siempre de una sola Facción; el TypeScript deja al héroe dentro con otra Facción.
- A) Se rechaza mientras vaya en un ejército o una convocatoria con otros: primero se separa (y el Líder, cede el mando).
- B) Sale al instante del ejército en una columna personal, en el mismo punto, con su parte del carro y de los víveres (como separarse), y luego deja la Facción.
- C) Como el TypeScript: sigue dentro.
Recomendada: **A** (no inventa movimientos y respeta que el Líder no se separa; el rechazo dice qué hacer).

**Q8. Quién cuenta como «ciudadano en el asentamiento» para que aparezca la nobleza** (hoy es un 1 fijo, por debajo del mínimo de 3, y nunca hay nobleza).
`Economia_Construccion` cuenta las casas compradas (no a los fundadores); el canon dice ciudadanos (héroes) en el asentamiento; y desde D31 no hay casas que comprar.
- A) Todos los residentes de la plaza (fundadores y quienes se mudaron).
- B) Solo quienes se mudaron (`cambiarResidencia`), como el TypeScript.
- C) Los héroes de la Facción que están dentro ahora.
Recomendada: **A** (es lo que dice el canon; con B una plaza fundada por tres héroes nunca tendría nobleza). Con cualquiera, la población cambia y hay que volver a medir (R1).

---------------------------------------------------------------------------------------------------------------------------------------

## 7. Decisiones que tomaría sin preguntar (se marcan con `// DUDA:` si se discuten)

1. **Crate `politica`** puro, con sus datos en los bloques de Dip; la reputación sigue en `comercio::reputacion` y `politica` devuelve sus `Suceso`s.
2. **Experiencia en milésimas** (2,5 no cabe en un entero) y el bloque `NIVEL_FACCION` escrito en Dip §3.5 con las cifras del canon (C9).
3. **Ids**: la Facción sigue siendo `u32` (`f-N`); las relaciones y las propuestas toman ids del contador común (`rel-N`, `propuesta-N`), porque un mismo par puede tener varias relaciones a lo largo del tiempo.
4. **Sin eventos por espera**: las propuestas caducan al mirarlas (`vigente = ahora < expira`, sin hecho, como el TypeScript); los cooldowns (creación, residencia, capital) son instantes guardados que compara la orden; las solicitudes no caducan (C10).
5. **Relaciones rotas** se conservan (las pide la ventana de «guerra tras romper» y la proyección); se indexa la activa de cada par para que la consulta siga siendo O(log n).
6. **Desarme del señor y vasalla conquistada** en la misma transición que la caída de la última plaza, en ese orden (C7), y antes de la reubicación de los residentes.
7. **El arrastre de `declarar`** lo usan las dos vías (`DeclararGuerra` y `Atacar` con `declara_guerra`): `Detalle::Guerra.arrastre` y `GuerraDeclarada.arrastradas_ids` dejan de ir vacíos.
8. **Códigos de evento**: `faccion.guerra_declarada` → `diplomacia.guerra_declarada` (la crónica y el tipado cambian con él); `asentamiento.acceso` → `asentamiento.puerta_fijada`; `faccion.relacion` desaparece. Sin clientes, el cambio es libre; los catálogos se corrigen en el mismo commit.
9. **El andamiaje se queda como administración sobre las reglas reales**, no se borra: `FijarFaccionDeHeroe` otorga o quita la ciudadanía con la sucesión, `FijarResidencia` es la reubicación forzosa (sin cooldown), `FijarNivelFaccion` fija la experiencia al umbral del nivel, `FijarRelacion` crea o rompe una relación con dirección (`a` señora o declarante) sin propuesta ni reputación. `ColocarHeroe`, `CrearEjercito`, `DarSuministro`, `DarAlmacenPersonal` y `DarCaravanaFundacion` siguen para montar escenarios. Los jugadores y los bots solo usan las órdenes del contrato; el batch pasa a ellas en 5I.
10. **Quien deja la Facción o su residencia pasa al campamento más cercano** (G0 l.70, Dip §4.2), con sus escuadras; es una divergencia anotada del TypeScript (D22).
11. **Capital efectiva al paso**: el paso de minuto de una plaza busca la capital de su Facción recorriendo sus plazas (como mucho 7, el cap de fundación); sin índice ni evento. La distancia es entera (milésimas).
12. **Títulos**: un evento **global** (uno para todo el mundo, no por entidad) de prioridad 10 cada 10 minutos de mundo, que recalcula solo si cambió algo de lo que miran desde la última vez; la cadencia va a un bloque `TITULOS.cadenciaMinutos` como placeholder. «Mecenas de los Aedas» vale 0 hasta el sistema 6 (no nace).
13. **Liga** derivada bajo demanda (componentes conexas de las relaciones activas); no se guarda ni se proyecta hasta que un cliente la pida.
14. **Nombre único** comparado tras recortar y pasar a minúsculas (`to_lowercase`, igual en cada máquina); sigilo libre con FNV-1a de 32 bits como el TypeScript.
15. **Roles técnicos por membresía** en 5E (lo anunció la decisión del 2026-10-24): `moderador` ve `ADMIN` y no da órdenes; `administrador_partida` y `administrador_global`, las de administración.
16. **Vetar expulsa** como cerrar la puerta (la regla es «sale quien ya no puede entrar»); a un residente no se le veta (Pres §8).
17. **El Rey de una Facción nueva** es quien la crea; una plaza recién fundada **no** tiene Gobernador hasta que el Rey lo nombre (como el TypeScript, `settlement.ts:301`), y sin Gobernador nadie puede dar lo que pide su cargo (`no_autorizado`, que ya dice «no eres el titular»).
18. **Cada tanda mapea sus órdenes al contrato** y actualiza Proy §5.1 en su commit; 5H solo completa proyecciones, eventos tipados y dorados.
19. **`Proy` §5.2.8** («cualquier participante estaciona») no se adopta: manda el Líder (C16).
20. **Reubicación por ruina y desalojo por conquista** vacían además los cargos locales y avisan a la Facción (`heroe.reubicado` ya existe); no se mira el cupo de viviendas del destino (sigue la fila de paridad de Combate §9).

## Decisiones del autor (2026-10-24)

1. **Q1 Puerta:** la del canon: `fijarPuerta` por grupos (neutrales, aliados, enemigos y Aedas) y `vetarJugador`; por defecto **cerrada a neutrales y enemigos**; cerrar o vetar expulsa a quien ya no puede entrar.
2. **Q2 Atacar a un aliado:** es una **traición**: atacar a un aliado con `declara_guerra` rompe la alianza en el acto, cuesta −25 de reputación (en vez de −12) y declara la guerra con su arrastre.
3. **Q3 Tributo:** en el paso de minuto de cada plaza vasalla, que paga su parte a la capital de la señora; sin eventos nuevos. Con la economía entera (abajo), con su resto oculto.
4. **Q4 Cargos locales:** hay que residir para ser designado; el cargo se vacía al dejar de residir.
5. **Q5 Trueques en una anexión o fusión:** siguen como están; los que quedan entre plazas de la misma Facción dejan de mover la reputación.
6. **Q6 Refundación, almacén personal y caravanas a mano:** entran en el sistema 5, en su tanda propia (5G). Devuelve al oro de bandidos el uso que tenía en el TypeScript (decisión M6 de `Consideraciones/Calibracion_Sistema4_Decisiones.md`).
7. **Q7 Dejar la Facción dentro de un ejército:** se rechaza; primero se separa, y si es el Líder, cede antes el mando.
8. **Q8 Ciudadanos para la nobleza:** todos los residentes (fundadores y quienes se mudaron).

Decisión 9 de la §7 (el andamiaje de administración se queda como herramienta sobre las reglas reales): aceptada por el orquestador como decisión técnica; jugadores y bots usan solo las órdenes del contrato.

### Orden de trabajo acordado
- **5A** (reglas puras, crate `politica`) en paralelo, en un worktree aparte.
- Antes de **5B**, en el árbol principal y por este orden: la **calibración del sistema 4** (`Consideraciones/Calibracion_Sistema4_Decisiones.md`: atalayas, bandidos 6 h, protección al fundar, asedio con convocatoria de 30 s, carga de las adjuntas, empalizada con `LAYOUT_VERSION` 5) y la **economía entera** (todo lo almacenado en enteros con resto oculto y redondeo hacia abajo), porque 5B-5G añaden flujos (tributo, refundación) que deben nacer enteros.
- Después, 5B → 5C → 5D → 5E → 5F → 5G → 5H → 5I.

## Avance de 5A (2026-10-24): reglas puras en `crates/politica`

Hecha en la rama `politica-5a` (worktree aparte, sobre `4377b86`). Crate nuevo `crates/politica` (≈ 3.700 líneas con pruebas), sin dependencia del núcleo: depende de `base`, `asentamiento`
(los helpers de bloques, `Recurso` y `Cargo`, que se reutiliza en vez de duplicarlo) y `comercio` (`Suceso` y `ReglasReputacion`). Solo se tocó el `Cargo.toml` del workspace; el núcleo no.

**Datos** (`datos.rs`, del documento `Diplomacia_Facciones.md`): `CIUDADANIA`, `CAPITAL`, `PUERTA`, `RELACION_PROPUESTA`, `ANEXION`, `FUSION`, `CATALOGO_SIGILO`, `CAP_FUNDACION_POR_NIVEL`,
`CUPO_NIVEL_ASENTAMIENTO` y el bloque nuevo **`NIVEL_FACCION`** (C9, en Dip §3.5, con las cifras de `constants.ts:1494-1534`; experiencia en milésimas, ratio en ppm). Se quitó el bloque vacío de
`Combate.md`. `texto_de_parametros()` queda listo para entrar en la huella del núcleo en 5B.

**Módulos y tipos públicos**
| Módulo | Tipos y funciones | Reglas |
|---|---|---|
| `tipos` | `FaccionId`, `HeroeId`, `AsentamientoId`, `RelacionId`, `PropuestaId`; `Cargo` (el de `asentamiento`), `CARGOS`, `Cargos` (titular, ocupado, es_titular, vaciar_de); `Grupo`, `Grupos` | |
| `faccion` | `Faccion`, `Derrota`, `Facciones` (índice héroe → Facción), `Salida`, `ErrorFaccion` | crear (nombre sin mayúsculas, sigilo, 1 héroe 1 Facción, cooldown), solicitar, responder, otorgar, quitar con **sucesión** por orden de ingreso, dejar (**Q7**), Rey, Embajador, admitir otras, experiencia monótona, derrota |
| `nivel` | `Hecho`, `xp_de`, `combate_digno`, `nivel`, `experiencia_minima`, `cap_fundacion`, `cupo_nivel` | §3.5 y Doc 2.2.1 |
| `cargos` | `asignar_cargo_local`, `ErrorCargo` | §3.4 con **Q4** (hay que residir) |
| `residencia` | `sin_cooldown`, `puede_mudarse`, `ciudadanos_para_nobleza`, `ErrorResidencia` | §4.1-§4.2; **Q8** |
| `capital` | `PlazaDeCapital`, `designada`, `efectiva`, `designar`, `ErrorCapital` | §3.7 |
| `sigilo` | `Sigilo`, `Catalogo` (es_del_catalogo, motivo_rechazo, sigilo_libre), `mismo_sigilo`, `hash32` | §3.8 (FNV-1a sobre UTF-16, como el TS; semilla `f-{id}`) |
| `puerta` | `Puerta`, `Visitante`, `GrupoDePuerta`, `grupo_de_puerta`, `ErrorPuerta` | §4.3 con **Q1** (grupos con `aedas`, vetos, cerrada a neutrales y enemigos, `expulsados`) |
| `relaciones` | `RelacionPolitica`, `TipoRelacion`, `EstadoRelacion`, `Tributo`, `Relaciones` (índice de la activa por par), `Guerra`, `Declaracion`, `Rebelion`, `VasallaConquistada`, `ErrorDiplomacia` | validar par, crear, terminar, `declarar_guerra` con **arrastre**, `declarar_al_atacar` (**Q2**: la traición), paz, romper, rebelión, guerra tras romper, desarme del señor, vasalla conquistada, defensa del vasallo |
| `propuestas` | `Propuesta`, `Clase` (relación, anexión, fusión en un solo almacén), `Pacto`, `Union`, `Propuestas`, `ErrorPropuesta`; `exigir_{relacion,anexion,fusion}_posible`; en `Politica`: proponer, responder y retirar | §5.3, §6.1-§6.2 con revalidación y batalla abierta |
| `tributo` | `pago`, `Pago`, `UNIDAD` | §5.6 con **Q3** en enteros con resto oculto (C3b) |
| `liga` | `Liga`, `ligas` | §5.7 |
| `titulos` | `TituloId`, `Titulo`, `Medida`, `calcular`, `Cambio`, `narrar` | §7 |
| `traslado` | `Plan`, `Cambio`, `planificar`, `aplicar`, `anexionar`, `fusionar`, `marca_tras_fusion` | §6.3 como lista de cambios puros |
| `autorizacion` | `Vista` (la da el núcleo), `Actor` con los predicados de Proy §4.3 y §5.2 | fail-open salvo `responde_solicitudes` |
| `lib` | `Politica { facciones, relaciones, propuestas }` | el estado que guardará el núcleo |

**Pruebas:** 93 en el crate, todas en verde. Portadas del TypeScript (con el fichero de origen en el módulo de pruebas): `sigilo.test.ts` (6 de 6, más el vector de FNV-1a y el sigilo de
«faccion-1» calculados aparte con el algoritmo del TS), `capital.test.ts` (7 de 7), `puerta.test.ts` (6 de 7: `puedeReclutarEn` es de `militar`), `cargos.test.ts` (8 de 8), `faccion.test.ts`
(solicitudes 2 de 2 y la regla de `cambiarResidencia`; mover al héroe es de 5B), `eventosDominioFaccion.test.ts` (3 de 4: la de fundar es de 5C), `experienciaFaccion.test.ts` (las cifras de los 8;
lo que miran del combate, el trueque y el ascenso es de `combate`, `comercio` y 5C), `diplomaciaDecisiones.test.ts` (5 de 5: tributo en enteros, nombre de la fusión, marca de capital),
`eventosDominioDiplomacia.test.ts` (2 de 2, en el tributo), `vasallaConquistada.test.ts` (2 de 2), `eventosDominioTitulos.test.ts` (3 de 3) y de `autorizacionComandos.test.ts` los 9 casos que miran
estos predicados (la exhaustividad de la matriz y los roles son de 5E). Además, reglas de los comandos de `comandosMembresiaFaccion`, `comandosDiplomacia`, `comandosAnexion` y `comandosFusion`
(la orden entera es del núcleo) y las decisiones Q1-Q4, Q7 y Q8.

**Lo que 5A no cubre (y dónde va):** guardar el estado, la foto y los códigos de rechazo (5B); reubicar al campamento más cercano, fundir la memoria y la columna que toma la Facción (5B);
consumir la experiencia, `faccion.nivel_subio`, la capital en el `Exterior` y el nombre de ciudad (5C); aplicar los `Suceso`s, expirar los trueques de la rebelión y llamar al desarme y a la vasalla
conquistada en la caída (5D); la matriz en `Comando::autoriza` y retirar `manda_en` (5E); el traslado de lo no político, la reputación y la tecnología de la fusión y la cadencia de los títulos (5F).
`cap_fundacion` y `cupo_nivel` existen aquí, pero sus llamadores siguen leyendo `ReglasTerritorio` y `asentamiento::nivel` hasta 5C (una prueba iguala los dos bloques de cupo).

**Documentos tocados:** `Diplomacia_Facciones.md` (nota de 5A, bloque `NIVEL_FACCION`, C4/Q3, C6/Q4, C7, C8, Q2, C10, C13/Q7, Q1), `Combate.md` (sin el bloque vacío; Q2), `Comercio_Caravanas.md`
(dónde está `NIVEL_FACCION`), `Economia_Construccion.md` (Q8: residentes para la nobleza), `Presencia_Y_Convocatoria.md` §8 (Q1), `Proyecciones_Y_Autorizacion.md` §5.2 (nota de 5A; C16 en §5.2.8),
`Heroe_Y_Tropa.md` (residencia en 5A), canon `Docs/Game/2_*` (Q2, Q3, Q4, Q7, C7, C11, C12) y `Docs/Game/1_*` (Q1, la puerta por defecto y el veto que expulsa), y las filas de paridad de
`NIVEL_FACCION` en `Docs/Mecanicas a desarrollar.md` (l.96, 103, 108 y 134). C5 queda resuelta por Q8; C9-C14 y C16 corregidas; C15 (cadencia de los títulos) es de 5F.

**Dudas que deja 5A** (marcadas `// DUDA:` en el código):
1. Q2: la traición no cobra además `penalizacionGuerraTrasRomperAlianza` (−15) por la alianza que rompe en ese instante; sí anota `rotaPor`/`rotaEn`.
2. Q3: lo que no cabe en la capital no sale de la vasalla (el TS lo descontaba y lo perdía).
3. Vasalla conquistada: se acaba cualquier relación activa entre vasalla y conquistadora, no solo las guerras, para no dejar dos activas en el par.
4. Q7: ¿un ejército de un solo héroe (que conserva su identidad de ejército) también impide dejar la Facción? Hoy la regla recibe «con otros» y solo rechaza entonces.
5. Fusión: la marca de capital se copia tal cual sin mirar si sigue vigente (`capital::designada` ya la ignora si no vale).
6. `romperRelacion` y `rebelionVasallo` sobre una relación ya rota se rechazan (`NoActiva`); el TS no lo miraba y volvía a cobrar la reputación.

### Decisiones del autor sobre las dudas de 5A (2026-10-24)

1. **5A-1, traición:** solo −25; no suma los −15 de la guerra tras romper. Sin cambio de código; escrito como regla (`relaciones::declarar_al_atacar`, Dip §5.4, G2 2.7).
2. **5A-2, tributo que no cabe: CAMBIO.** Se paga entero y la capital pierde el exceso, como el TypeScript. `tributo::pago` devuelve ahora `pagadas` (lo que sale de la vasalla) y
   `recibidas` (lo que entra en la capital, topado por su hueco). Dip §5.6 y G2 2.4.
3. **5A-3, vasalla conquistada:** se acaba cualquier relación activa entre vasalla y conquistadora. Aceptada; Dip §5.5 y G2 2.4.
4. **5A-4, Q7: CAMBIO.** Dejar la Facción yendo en un ejército se rechaza siempre, también en uno de un solo héroe. `Facciones::dejar(h, ahora, en_ejercito)`: el núcleo pasa `true` por ir
   en cualquier ejército (o en una convocatoria con otros héroes). Dip §3.3 y G2 2.5.
5. **5A-5, marca de capital en la fusión:** pasa tal cual, sin mirar si sigue vigente. Aceptada; Dip §6.2.
6. **5A-6, romper o rebelarse sobre una relación rota:** se rechaza (`NoActiva`). Aceptada; Dip §5.5.
7. **5A-7, `Propuesta.propuesta_por`, héroe o Facción:** se decide en 5H. Hoy el crate guarda el héroe (Dip §2.4) y el contrato dice «facción que propone».
8. **5A-8, ciudadanía en una Facción vacía:** quien la recibe pasa a ser su Rey. Sin cambio de código; escrito en Dip §3.2 y G2 2.5.

Ya no queda ninguna `// DUDA:` en `crates/politica`.

## Avance de 5B (2026-10-24): la Facción, la ciudadanía y la residencia en el núcleo

Hecha sobre `a68ab7c` (rama `claude/lucid-curran-6d6ffa`), sin commit. Foto **`BAN28`** (el plan decía `BAN26`: la calibración del sistema 4 y la economía entera gastaron `BAN26` y `BAN27`).

**Diseño**
- **Estado.** `Estado::politica` es el `politica::Politica` de 5A entero (Facciones con su índice de ciudadanía y `salidas`, y los almacenes de relaciones y propuestas, vacíos hasta 5D y 5F), con su codec (`codec_politica`, todos los campos, también los que aún nadie escribe). En cada héroe, `HeroeNucleo::residencia_cambiada_en` (el cooldown; solo lo estampan las tres órdenes de residencia). Las reglas de la política entran en la huella del núcleo (`ReglasEconomia::politica`).
- **Una sola fuente de la ciudadanía.** La Facción guarda sus ciudadanos en orden de ingreso; `HeroeNucleo::faccion` es su copia para la lectura en caliente y solo la escribe `Nucleo::tras_cambiar_de_faccion` (`nucleo/ciudadania.rs`), que además da la Facción a la columna de quien va solo (§3.1-§3.3), rehace la demanda de bandidos del campamento donde reside y marca las dos Facciones. La auditoría del registro de cambios comprueba en cada versión que la copia es la de la Facción. Lo explorado sin bandera se funde al cerrar la versión (ya lo hacía `exploracion` con el héroe tocado).
- **Órdenes 128-136**, todas de la cuenta del héroe, mapeadas al contrato (`sesiones::ordenes`): `CrearFaccion` (`creadas` = la Facción, clase `FACCION`; id = el siguiente a la mayor Facción nombrada en el mundo), `SolicitarIngreso`, `ResponderSolicitud` (solo el Rey, sin excepción), `DejarFaccion` (Q7 y 5A-4: en un ejército, aunque sea solo, o en uno en preparación, `faccion.en_ejercito`; si residía en una plaza, al campamento de mercenarios más cercano a ella, decisión 10), `AsignarRey`, `AsignarEmbajador`, `CambiarResidencia` (plaza de su Facción, con cooldown), `ResidirEnCampamento` y `DejarResidencia` (al campamento más cercano a donde está, §4.2). Dos clases nuevas de `Autoriza` (`ReyDeLaFaccion`, `CiudadanoDeLaFaccion`) que comprueba `politica::autorizacion::Actor` sobre el núcleo (`impl Vista for Nucleo`, sin cargos hasta 5E). Proy §5.1 y la prueba de la matriz, al día (112 comandos del jugador, 34 órdenes no disponibles).
- **Quien deja su casa estando dentro** (y sin columna dentro, que es como está un residente) sale al mapa por su puerta con todo lo suyo, como al caer la plaza (`salir_al_mapa`): un residente sin casa no puede quedarse dentro sin la columna con la que se entra de visita. Vale para las tres órdenes y para dejar la Facción (duda 1).
- **Fundar.** Los fundadores (los participantes de la columna que son ciudadanos de su Facción, el Líder primero, hasta `FUNDACION.maxJugadoresFundacionGrupal`, que ahora lee `asentamiento::Reglas::max_fundadores`) pasan a residir en la plaza sin estampar el cooldown; sin ninguno, `fundacion.sin_fundadores`. La columna sigue en el mapa (deshacerla dentro es de 5G).
- **Nobleza (Q8).** `Exterior.ciudadanos` son los residentes de la plaza (`Nucleo::ciudadanos_de`). Para que el paso de minuto no pague más consultas, la ración de la guarnición y los ciudadanos salen de **una** consulta al índice de residentes (`residentes_en_el_paso`) y la guarnición come con la ración del `Exterior` del mismo paso (antes se miraba el índice dos veces por paso, y con los ciudadanos habrían sido cuatro).
- **La administración sobre las reglas reales (decisión 9).** `FijarFaccionDeHeroe` quita o da la ciudadanía con la sucesión, sin cooldown, sin Q7 y sin tocar la residencia; la Facción que nombra y no existe nace vacía (`Facciones::crear_vacia`, «Facción N», sigilo libre) y quien entra es su Rey (5A-8). `CrearEjercito` hace ciudadano a su héroe de campo por la misma vía, de modo que toda columna de la administración tiene su Facción como entidad. `FijarResidencia` sigue siendo la reubicación forzosa, sin cooldown. `FijarNivelFaccion`, `FijarRelacion` y `FijarTecnologias` no crean la Facción (5C y 5D los pasan a la entidad).
- **Proyección.** `FACCION` (y `ADMIN`) llevan la Facción con nombre, sigilo, Rey, Embajador, ciudadanos en orden de ingreso, solicitudes, experiencia, capital designada, `derrotada_por` y `admite_otras_en_ataques` (campos que el contrato ya tenía); los ciudadanos ya no se buscan recorriendo todos los héroes. `audiencia::facciones` cuenta las Facciones de la entidad aunque no les quede nada (D2). Sin formas nuevas: ni cambio de esquema ni dorados.
- **Hechos.** Los diez de la tanda (`faccion.creada` con el nombre, `faccion.solicitud_ingreso`, `faccion.ciudadania_union`, `faccion.solicitud_denegada`, `faccion.abandonada`, `cargo.rey_asignado`, `cargo.embajador_asignado`, `ciudadania.residencia_cambiada`, `ciudadania.residencia_dejada`, `ciudadania.residencia_en_campamento`), con `heroe` y `faccion` para que los oigan el héroe y la Facción; en el catálogo de Dip y fuera de la paridad pendiente. 18 rechazos nuevos (`faccion.*`, `residencia.*`, `fundacion.sin_fundadores`) en `07_Catalogo_de_Rechazos.md`.

**Pruebas.** `crates/nucleo/tests/politica.rs` (8, sobre el mundo real): crear con sus rechazos, pedir y responder (solo el Rey; una Facción inexistente), Rey y Embajador, la sucesión del trono al dejarla, el cooldown de creación y la Facción vacía que recupera Rey (`comandosMembresiaFaccion`); la columna de quien va solo y Q7 (`faccionDeLaColumna`); las tres órdenes de residencia con su cooldown, la salida al mapa desde el campamento y los ciudadanos de la nobleza (`comandosResidencia`, `residenciaMercenaria`); dejar la Facción desde una plaza; los fundadores y `fundacion.sin_fundadores`; la proyección; determinismo normal y a trozos, ninguna cita en la cola por la política, foto `BAN28` de ida y vuelta y `BAN27` rechazada con su motivo. Además 1 unitaria del codec, 1 del registro de órdenes y 1 del crate (`crear_vacia`). Se adaptaron las que cambian con razón: la cabecera `BAN28` (13 ficheros), la huella de las reglas, el fundador que ahora reside (ruina, desalojo, interior), `cliente_de_ejemplo` (crea su Facción con `crear_faccion` y ya no necesita `fijar_residencia` tras fundar) y una orden no disponible de `sesiones.rs`.
- `cargo test --workspace --no-fail-fast`: **89 resultados, 1.378 pruebas en verde, 13 ignoradas** (+11).
- `BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`: 39 resultados, 474 pruebas, 0 fallos (encontró que crear una Facción no marcaba las Facciones donde el creador tenía solicitudes: arreglado).
- Las 78 pruebas de sesión (`admin`, `ambitos`, `batalla_unity`, `cliente_de_ejemplo`, `eventos`, `interior`, `ordenes`, `ordenes_mundo`, `propios`, `reanudacion`, `sesiones`, `vista` y `politica`) **20 veces seguidas: 1.560 ejecuciones, 0 fallos**.
- `buf lint` y `buf breaking --against '.git#ref=HEAD'` limpios (sin cambio de esquema).
- Paso de minuto (1.000 plazas × 3 días en release, alternando el árbol de antes y el de ahora): **13,44-13,72 s ahora frente a 13,02-13,46 s antes (+2,9 %)**, mismos 3.061.149 pasos y versión 50.132. Es el coste de que cada plaza tenga ya a su fundador de residente (el paso lee sus escuadras); con cuatro consultas al índice por paso era +10 %.

**Batches antes y después** (release, semilla 42, el árbol de `a68ab7c` contra el de ahora, mismo artefacto de mundo):

| Batch | Métricas de juego iguales | Lo único que cambia |
|---|---|---|
| `batch poblacion` (300 pedidas, 230 colocadas × 30 días) | 84 de 84 (nivel 2 en el 92,17 % a los 5,25 días, ruinas 2,61 %, 0 hambre, nivel 3 0 %), series y resumen idénticos | tiempo de pared 51,2 s → 57,2 s (5,1 → 5,8 µs por paso, con otras cargas en la máquina) |
| `batch comercio` (160 pedidas, 146 colocadas × 30 días) | 101 de 101 (136 trueques cumplidos, 390 caravanas, comisión 0,48 % del valor), series y resumen idénticos | tiempo de pared 89,6 s → 79,7 s |

El riesgo R1 no se ve todavía: los batches fundan con el héroe de campo de la administración, que pasa a ser el único residente de su plaza; un residente da `ciudadanos = 1`, lo mismo que el 1 fijo de antes, y la nobleza pide 3 y un Palacio. Aparecerá cuando los bots se muden a las plazas (5I).

**Dudas para el autor**
1. Quien deja su casa (por cualquiera de las tres órdenes o por dejar la Facción) estando dentro de ella sale al mapa por su puerta con todo lo suyo. ¿O se rechaza la orden mientras esté dentro, o se queda dentro con una columna nueva?
2. Q7: se rechaza dejar la Facción también en un ejército en preparación de un solo héroe (la regla dice «con otros»; si se fuera, la convocaría un extranjero).
3. Las tres órdenes de residencia se rechazan en un ejército en preparación (`heroe.en_convocatoria`): la regla no lo dice, pero su tropa se iría con él a otra casa.
4. Dejar la Facción lleva al campamento más cercano **a la plaza que deja** (decisión 10, Doc 0); `dejarResidencia`, al más cercano **a donde está** (§4.2). ¿Los mismos?
5. `dejarResidencia` exige el cooldown (Mechanics §4.1) aunque el TypeScript no lo miraba.
6. Ganar o perder la ciudadanía no cambia la Facción de un ejército de varios héroes (como el TypeScript): un héroe sin bandera de un ejército sin bandera que entra en una Facción deja el ejército con Facciones mezcladas. ¿Se rechaza pedir o aceptar el ingreso yendo en un ejército?
7. La Facción que nace por la administración se llama «Facción N» sin mirar si el nombre está libre, y el id de una Facción nueva es el siguiente a la mayor nombrada en el mundo (no el contador común, para no chocar con los ids que elige la administración).
8. `fundacion.sin_fundadores` se comprueba después de `fundacion.sin_faccion` y antes de `fundacion.cap_alcanzado`.
9. `Asentamiento.fundadores_ids` va vacío y `casas_compradas` lleva a todos los residentes (para Q8 cuentan igual). ¿Se guarda en 5H quién fundó (estado nuevo)?
10. Cambiar de residencia lo bloquea una batalla en la plaza de destino, pero no en la de origen (el campamento de un residente de una plaza asediada se iría con él). ¿Se bloquea también desde una plaza en batalla?
11. `HEROE` no lleva nada nuevo de la Facción (el `Heroe` del contrato no tiene `faccion_id`; las solicitudes propias son de 5H). ¿Un `Heroe.faccion_id` aditivo en 5H?
12. El paso de minuto cuesta un 2,9 % más en la medida de 1.000 plazas porque cada plaza tiene ya a su fundador de residente. ¿Se acepta?
13. `FijarFaccionDeHeroe` (administración) ahora también cambia la Facción de la columna de quien va solo y rehace la demanda de bandidos de su campamento; antes no tocaba nada más.

## Avance de 5G-a (2026-10-24): almacén personal y caravanas a mano

Primera mitad de la tanda 5G, en la rama `tanda-5g-a` (worktree aparte), en paralelo con 5B. La refundación y la Caravana de Fundación (las otras 5 órdenes de 5G) son de **5G-b**, que depende de 5B.

**Órdenes (5)**, en `crates/nucleo/src/nucleo/a_mano.rs`. Los códigos empiezan en el **138**: del 128 al 137 quedan para 5B.
| Código | Comando | Orden del contrato | Quién | Regla |
|---|---|---|---|---|
| 138 | `GuardarEnAlmacenPersonal { heroe, recurso, cantidad }` | `guardar_en_almacen_personal` | la cuenta del héroe | dentro de su residencia, o el Líder con su columna a su puerta; `min(pedido, carro, hueco del almacén)` |
| 139 | `SacarDelAlmacenPersonal { heroe, recurso, cantidad }` | `sacar_del_almacen_personal` | la cuenta del héroe | igual; `min(pedido, lo guardado, hueco del carro)` |
| 140 | `CargarCaravana { ejercito, caravana, asentamiento, recurso, cantidad }` | `cargar_caravana` | un participante de la columna | adjunta, a la puerta de una plaza de su Facción; `min(pedido, hueco de la caravana y de la columna, existencias)`, sin reserva |
| 141 | `ReclamarCaravana { heroe, caravana, politica }` | `reclamar_caravana` | la cuenta del héroe | solo la Caravana de Fundación que vuelve sola: hasta 5G-b, `caravana.no_reclamable` |
| 142 | `SoltarCaravana { ejercito, caravana, heroe }` | `soltar_caravana` | un participante de la columna | solo dentro de la plaza en la que entró la columna: la deja aparcada (hecho propio) |

Todo en unidades enteras (C3b: lo que se mueve se redondea hacia abajo). Rechazos nuevos: `almacen_personal.fuera_de_tu_residencia`, `carro.no_lleva`, `caravana.no_reclamable`, `caravana.soltar_solo_dentro` (los demás ya existían). Hechos que pasan a emitirse: `heroe.almacen_personal`
(con `sentido` como texto), `ejercito.caravana_cargada`, `ejercito.caravana_soltada`. `AparcarCaravana` recibe el código de su hecho para que `soltarCaravana` lo reutilice. M6: el oro de bandidos que llega al carro ya
se guarda en el almacén personal y se gasta en el mercado y la taberna del campamento; el fondo de refundación es de 5G-b.

**Foto:** sin cambio (no hay estado nuevo: el almacén personal y el contenido de las caravanas ya se guardaban). **Esquema:** aditivo, `Heroe.capacidad_almacen_personal = 36` (milésimas, en `HEROE` y `ADMIN`), sin dorado nuevo.
**Contrato:** las 5 órdenes dejan de dar `orden.no_disponible` (quedan 38: 28 de política, 5 del sistema 6 y 5 de refundación); matriz de Proy §5.1 y `Comandos_Parametros.md` al día.

**Pruebas:** `crates/nucleo/tests/a_mano.rs`, 8 (cada orden y cada rechazo, la conservación de lo movido, residencia en campamento y en plaza, dentro y a la puerta, solo el Líder, una adjunta llena que no da espacio, y dos por el WebSocket real);
portadas de `residenciaMercenaria.test.ts` («almacén personal»), `comandosMercenarios.test.ts` y `avanzarEjercitos.test.ts` (`cargarCaravanaAdjunta`, `soltarCaravana`). Ida y vuelta por el registro de los 5 comandos en `comando.rs`.

**Decisiones del autor sobre las dudas de 5G-a (2026-10-24), aplicadas:**
1. **5G-a-1:** `reclamarCaravana` se queda sin caso de éxito hasta 5G-b.
2. **5G-a-2, cambio:** guardar y sacar valen también **dentro** de la residencia, con el carro que el héroe trajo (el de la columna que conserva para sus caravanas, si la conserva).
3. `cargarCaravana` solo de una plaza de la propia Facción hasta 5D (`permiteReabastecerAliados`), con `caravana.plaza_ajena`.
4. **5G-a-4, cambio:** cada carro tiene un espacio fijo y nunca se llena por encima de su límite: `Nucleo::capacidad_de_carga` suma solo el **hueco libre** de cada adjunta (capacidad menos lo que lleva); `cargarCaravana` no pasa del hueco de la columna; el botín, `DarSuministro`, las compras y aparcar ya leen esa capacidad. Canon 5.13.2 y Movimiento §9.
5. **5G-a-5, corrección:** `soltarCaravana` solo vale **dentro** de la plaza en la que entró la columna (`caravana.soltar_solo_dentro`); si era la última adjunta de la columna que un residente conserva en su residencia, la columna se deshace. Después, el autor extendió la regla a `aparcarCaravana` (40), que es la misma acción: solo dentro, con el mismo rechazo y la misma disolución de la columna del residente (prueba `aparcar_es_soltar_y_solo_vale_dentro_de_la_plaza`; las pruebas que aparcaban desde la puerta como preparación lo hacen ahora como administración, que salta la regla).

**`batch militar` tras «solo dentro» (2026-10-24, release, 60 plazas × 30 días, semilla 42).** El Intendente aparcaba desde la puerta de la plaza hermana y, con la regla nueva, se le rechazaban todas en silencio (275 de 275, `caravana.soltar_solo_dentro`; las caravanas se quedaban enganchadas para siempre y de ahí 13.991 `AnadirAnimal` y 11.192 `AnadirCarro` con `caravana.no_disponible`). Ahora entra con su ejército (`Guarnecer`), aparca, la manda a su origen y sale (`SalirDeAsentamiento`): 5.147 aparcadas, 0 rechazos. Las órdenes que el bot da tras comprobar lo que exigen pasan por `Sim::esperada` y un rechazo cuenta en el invariante nuevo `inv_rechazos` (una anomalía del informe): antes 275, ahora 0.

### Decisiones del autor sobre las dudas de 5B (2026-10-24)

1. **5B-1, dejar la casa estando dentro: CAMBIO.** Al dejar la residencia (cualquiera de las tres órdenes) o la Facción estando dentro de la plaza que deja, las escuadras (su campamento) y el almacén personal
   pasan **sin viaje** a la residencia nueva, y el héroe **sigue dentro como visitante** si la puerta le admite, con su carro y sus víveres. Si abandonó la Facción y la puerta ya no le admite, sale expulsado
   al mapa **solo**, con su carro (`heroe.expulsado`, motivo `puerta_cerrada`). En un campamento de mercenarios siempre se sigue dentro (admite a todos). Código: `ciudadania.rs` (la mudanza ya no saca a nadie)
   y `dentro.rs::echar_si_ya_no_entra` (plaza, pueblo o avanzada donde esté, tras dejar la Facción). Dip §3.3 y §4.2, G2 2.5.
2. **5B-2, 5B-3, 5B-5, 5B-7, 5B-8, 5B-9, 5B-11, 5B-12 y 5B-13:** decisiones técnicas del agente, aceptadas por el orquestador (el +2,9 % del paso de minuto, aceptado). 5B-9 (`fundadores_ids`) y
   5B-11 (`Heroe.faccion_id`) van a 5H.
3. **5B-4, el campamento de destino: CAMBIO.** Al dejar la residencia y al dejar la Facción, el campamento de mercenarios más cercano **a la plaza que deja** (se corrige el canon 2.5 y Dip §4.2, que decían
   «a donde está» para `dejarResidencia`).
4. **5B-6, ingreso yendo en un ejército: CAMBIO.** Pedir el ingreso, o que lo acepten, yendo en un ejército (aunque sea de un solo héroe, o en uno en preparación) se rechaza como dejarla
   (`faccion.en_ejercito`); denegar la solicitud sí se puede. `politica::Facciones::{solicitar, responder}` reciben `en_ejercito`. Dip §3.2, G2 2.5.
5. **5B-10, mudarse desde una plaza en batalla: CAMBIO.** También se bloquea (`batalla.bloqueado`), además de mudarse hacia una: las tres órdenes de residencia y dejar la Facción (`Nucleo::sin_batalla_en_su_plaza`),
   aunque el héroe ya haya salido de la plaza. Dip §4.2, G2 2.5.

Pruebas: `politica.rs::quien_deja_su_casa_dentro_sigue_de_visita_si_la_puerta_le_admite_y_si_no_sale_solo_al_mapa`, `::en_un_ejercito_no_se_pide_el_ingreso_ni_se_acepta`, la de las tres órdenes de residencia
(se queda dentro del campamento; el campamento de destino es el de la plaza que deja) y `batallas.rs::nadie_se_muda_de_una_plaza_en_batalla_aunque_ya_este_fuera`; en el crate, `faccion::pruebas` (5B-6).
Dudas que dejan (marcadas `// DUDA:`): dejar la Facción se bloquea también desde una plaza en batalla (5B-10 habla de mudarse) y la expulsión al dejarla mira el lugar donde está, no solo la plaza que deja.

## Avance de 5C (2026-10-24): la experiencia, el nivel, la capital, la derrota y el nombre

Hecha sobre `becf290` (5B y sus correcciones), rama `tanda-5c`. Foto **`BAN29`** en la rama (`BAN30` al integrar: `calibracion-2` tomó `BAN29`; `BAN29` se rechaza con su motivo).

**Diseño**
- **La experiencia se da al ocurrir el hecho** (`crates/nucleo/src/nucleo/experiencia.rs`, `Nucleo::dar_experiencia`): la cuantía de `NIVEL_FACCION.xp` (en milésimas, `politica::nivel::xp_de`), `faccion.experiencia` con `motivo` (1 trueque, 2 combate, 3 batalla de Unity, 4 edificio, 5 conquista, 6 ascenso, 7 fundación, 8 bandidos), `cantidad` y el total, y `faccion.nivel_subio` en el mismo instante si cruza un umbral. Nada espera en la cola. Los hechos: combate con números **digno** en columna contra columna y en el asedio (los dos que premiaba el TypeScript), 2,5 por héroe que combatió a **su** Facción; batalla de Unity, 2,5 por héroe con `participo`, a la Facción del ticket, sin filtro; conquista (10, en `conquistar_plaza`, la misma por números y por Unity); campamento de bandidos destruido (1 ÷ nivel, nada desde el 5, por números y por Unity); edificio completado (1) y ascenso (30 × nivel), en `anotar_sucesos`; fundación salvo la primera (20); trueque cumplido entre Facciones distintas (2 a cada lado: el hecho `faccion.experiencia` existía y no sumaba).
- **El nivel se deriva** (`Nucleo::nivel_de_faccion`): desaparece `Estado::niveles_faccion`. Lo leen el cap de fundación (`politica::nivel::cap_fundacion`), el cupo de los niveles 2 y 3 (`politica::nivel::cupo_nivel`: la economía recibe ahora si queda sitio, `bloqueos_de_ascenso(.., sin_cupo_de_faccion)`), los huecos de políticas del Gobernador y la Gran Fundición. Se quitan los bloques repetidos (`CUPO_NIVEL_ASENTAMIENTO` de `Economia_Construccion.md`, `CAP_FUNDACION_POR_NIVEL` de `Fundacion_Y_Refundacion.md`), `ReglasTerritorio::cap_por_nivel`, `territorio::cap_de_fundacion` y `asentamiento::nivel::cupo_faccion`: un solo bloque de cada uno, en Dip §3.5. `FijarNivelFaccion` (administración, decisión 9) fija la experiencia al umbral del nivel (`Facciones::fijar_experiencia`) y crea la Facción vacía si no existe.
- **La capital** (`designar_capital` → `DesignarCapital`, 137, el Rey): `politica::capital::designar` con sus rechazos `capital.plaza_ajena`, `capital.sin_palacio`, `capital.ya_es_la_capital` y `capital.cooldown`; la marca vive en la plaza (`Asentamiento::capital_de`, en la foto) y sobrevive a la conquista sin valer. La **capital efectiva** se calcula en el paso de minuto de cada plaza (`Nucleo::exterior_de_capital`): una Facción de una plaza no la busca; si tiene más, recorre sus plazas con el índice derivado `Espacio::plazas_por_faccion` (lo mantienen fundar, la ruina y la conquista; la auditoría lo comprueba en cada versión) sin reservar memoria (`capital::{designada, efectiva}` reciben un iterador). Da al `Exterior` `es_capital`, `distancia_capital` (milésimas entre las dos plazas; sube el mantenimiento con `escalaDistancia` = 80 u) y la Gran Fundición: la Facción la tiene si alguna plaza suya la tiene en cualquier estado («ya tiene, o está levantando»), y solo se mira en la capital de una Facción con el nivel que la pide. `Faccion.capital_id` (campo nuevo y aditivo, 15) lleva la efectiva; la ficha de la plaza, `capital_de_faccion_id` (que el contrato ya tenía).
- **La derrota**: `registrarDerrota` (`Derrota::Por` en la conquista, `Derrota::Colapso` en la ruina), y se borra al fundar. `Faccion.derrotada_por` ya se proyectaba.
- **El nombre**: `politica::nombres` (la lista de 159, sin tildes ni mayúsculas, FNV-1a de 32 bits sobre UTF-16 con la semilla `as-N`), al fundar (también en `fundacion.asentamiento_fundado`, `valores.nombre`); `renombrar_asentamiento` → `RenombrarAsentamiento` (143 al integrar; 138 en la rama), lo da quien reside en la plaza (`Autoriza::ResidenteDeLaPlaza`, clase nueva; basta residir, como el TypeScript y Proy §5.2.4), recortado y vacío sin nombre, `asentamiento.renombrado` con el nombre. La ficha de la plaza (`FACCION`, `ADMIN`, `VISTA`) lleva `nombre`.
- **Contrato**: las dos órdenes mapeadas (`sesiones::ordenes`; quedan 32 `orden.no_disponible`), en la matriz de Proy §5.1 y en `Comando::autoriza`; cuatro rechazos nuevos (`capital.*`) en el catálogo; tres hechos que pasan de pendientes a emitidos (`faccion.nivel_subio`, `faccion.capital_designada`, `asentamiento.renombrado`). Esquema aditivo (`Faccion.capital_id`); `buf lint` y `buf breaking` limpios; sin dorados nuevos (ninguno lleva una Facción).

**Pruebas** (nuevas o ampliadas): `tests/politica.rs` (la capital efectiva, la designada con Palacio, los cuatro rechazos de `designar_capital`, la Gran Fundición única por Facción, el cap de fundación por la experiencia, la experiencia de fundar; la distancia que arruina una plaza lejana y no la arruina sin el factor; el nombre libre al fundar, `renombrar_asentamiento` y su rechazo; la foto `BAN29` con nombre y marca, y `BAN28` rechazada con su motivo; el guion de determinismo con `renombrar_asentamiento`), `tests/asentamientos.rs` (la experiencia de edificios y ascenso y los avisos de nivel en la plaza que sube sola; `derrotadaPor` = colapso), `tests/combate.rs` (`derrotadaPor` = el conquistador, la experiencia de conquista y la del combate digno), `tests/batallas.rs` (la de Unity), `politica::nombres` (4, de `nombresDeCiudades.test.ts`). Se adaptaron con razón: las pruebas con dos plazas de la misma Facción junto a campamentos distintos (a 1.000-1.500 u: la segunda caía en ruinas en un día) ponen la segunda junto a la primera (`cambios`, `comercio`, `entrar`, `heroes`, `politica`), `tests/combate.rs` monta las dos Facciones vecinas (la plaza conquistada quedaba a 1.500 u de la capital del conquistador) salvo la prueba de la caravana a mitad de camino (`montar_lejos`), la de la flota en la conquista (`caravanas.rs`) desactiva el factor de distancia (la plaza conquistada, más antigua por id, pasa a ser la capital de F2 y su plaza de origen se arruinaba en hora y media), y las que contaban `faccion.experiencia` filtran el motivo del trueque.

**Cifras** (antes de integrar origin; se repiten tras la integración en el informe).
- Paso de minuto (1.000 plazas × 3 días en release, alternando el árbol de `b3ee1cb` y el de 5C, tres veces cada uno): **17,10-18,94 s antes y 17,59-19,07 s ahora** (medias 18,2 y 18,1 s; la máquina estaba cargada: 5B midió 13,4 s), mismos 3.061.149 pasos y versión 50.132. Una primera versión que reservaba un `Vec` por paso para buscar la capital costaba +5-9 %; con el iterador, la Facción de una plaza que no busca y la Gran Fundición que solo se mira en la capital de nivel suficiente, no se nota.
- **Batches antes y después** (release, semilla 42, el árbol de `b3ee1cb` contra el de 5C, mismo artefacto de mundo):

| Batch | Métricas de juego iguales | Lo que cambia |
|---|---|---|
| `batch poblacion` (300 pedidas × 30 días) | 84 de 86 (las 2 distintas son de tiempo) | nada de juego (cada plaza es de una Facción distinta: es su capital, sin distancia); pared 60,7 → 58,3 s |
| `batch comercio` (160 pedidas, 146 colocadas × 30 días) | 43 de 105 (de las 62 distintas, 6 son de tiempo) | **plazas vivas 146 → 140**, **`ascenso_insolvente` 0 → 3.319** (comprobaciones de ascenso que el mantenimiento por distancia hace insolventes), plazas en nivel 2 al final 114 → 108, trueques cumplidos 136 → 125 (entre Facciones 81,6 → 74,4 %), caravanas 390 → 338 (8 perdidas por plazas arruinadas, antes 0), comisión 0,48 → 0,35 % del valor, peajes 152 → 101, acarreo ingresado 1,6 M → 6,8 M (con el acarreo ahora un 9,8 % de la solvencia, antes 2,0 %), reputación mediana final 9 → 30 |

El riesgo anunciado se ve donde una Facción tiene varias plazas: el batch de comercio da a cada Facción el nivel 8 (cap 6) y expande a 30-70 u de una plaza propia, así que sus plazas pagan ×1,4-×1,9 de mantenimiento (y más las de la cadena), y algunas se arruinan. El de población, con una plaza por Facción, no cambia nada.

**Pruebas.** `cargo test --workspace --no-fail-fast` en verde antes de integrar (89 resultados); `BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`: 39 resultados, 480 pruebas, 0 fallos (con la comprobación nueva del índice de plazas por Facción); `buf lint` y `buf breaking --against '.git#ref=HEAD'` limpios. Las cifras finales, tras integrar, en el informe de la tanda.

**Dudas para el autor**
1. **La capital efectiva tras una conquista.** «La plaza viva más antigua» mira `fundado_en`, y una plaza conquistada conserva el suyo: conquistar una plaza más antigua que las propias la hace capital (si no hay una designada), y entonces la plaza de origen paga la distancia. En la prueba de la flota (`caravanas.rs`) la plaza de F2 a 1.500 u de la conquistada se arruinó en hora y media. ¿Se mide la antigüedad desde que la plaza es de la Facción, o se deja la regla del TypeScript (que tenía lo mismo)?
2. **La escala de la distancia (`escalaDistancia` = 80 u).** Una plaza a 80 u de la capital paga el doble; a 300 u, ×4,75; a 1.000 u, ×13,5 y cae en ruinas en un día. En `batch comercio` (Facciones de seis plazas a 30-70 u unas de otras) seis plazas más se arruinan y aparecen 3.319 comprobaciones de ascenso insolventes. ¿Se calibra ya o se espera a 5I (bots de verdad)?
3. **La experiencia de combate con números** se da en columna contra columna y en el asedio, como el TypeScript; el saqueo de un pueblo, el ataque a una avanzada y el asalto a una caravana no dan nada (no existían en el TypeScript). ¿Se les da la misma regla (digno, por héroe)?
4. **Un campamento de bandidos destruido en Unity** da la experiencia de la batalla (2,5 por héroe, sin filtro) y además la de bandidos (1 ÷ nivel), como el TypeScript daba cada una en su sitio. ¿Las dos?
5. **La Gran Fundición** cuenta en cualquier estado (en cola, en obra o en pie) y en cualquier plaza de la Facción, también en una conquistada: quien conquista una plaza con Gran Fundición ya no puede levantar la suya en su capital. ¿Es así?
6. **`renombrar_asentamiento`** no limita el largo ni mira si otra plaza ya se llama así (como el TypeScript); solo recorta. ¿Un largo máximo y nombres únicos?
7. **`FijarNivelFaccion`** (administración) pone la experiencia en el umbral del nivel, también hacia abajo (no es monótona), y crea la Facción vacía si no existe. Se aceptó como andamiaje (decisión 9); lo anoto.
8. **Un hecho `faccion.experiencia` por edificio completado**: en una plaza activa son decenas por semana. ¿Se agrupan en uno por paso, o se quedan uno por edificio (como cada `construccion.edificio_completado`)?
9. **5B-10 en dejar la Facción:** se bloquea también dejar la Facción con la plaza donde reside en batalla (la regla habla de mudarse; dejar la Facción muda).
10. **5B-1 en pueblos y avanzadas:** la expulsión al dejar la Facción mira dónde está el héroe (la plaza que deja, otra plaza de su antigua Facción, un pueblo o una avanzada), no solo la plaza que deja.
11. **La semilla del nombre** es el id del contrato de la plaza (`as-N`); el TypeScript usaba su id interno. Solo cambia qué nombre sale.

**Integración (2026-10-24, sobre `0f2ddbd`: 5G-a y la calibración 2).** Foto **`BAN30`** (familia «BAN3»; `BAN29` —la de la calibración 2— se rechaza con su motivo: sus plazas no traen nombre ni marca de capital y guardan el nivel de Facción suelto). `RenombrarAsentamiento` pasa al código **143** (5G-a tomó 138-142; `DesignarCapital` sigue en el 137). Matriz y recuentos: 143 comandos, 119 de órdenes del jugador, 27 órdenes no disponibles. Dos pruebas de 5G-a ajustadas por 5C: `a_mano.rs` pone la segunda plaza junto a la primera (la de junto al campamento 2 se arruinaba por la distancia) y `urbanismo.rs` prueba la familia ajena con «BAN4». Tras integrar: `cargo test --workspace --no-fail-fast` **90 resultados, 1.396 pruebas en verde, 13 ignoradas**; auditoría **40 resultados, 490 pruebas, 0 fallos**; las 83 pruebas de sesión (13 binarios con `politica`) **20 veces seguidas: 1.660 ejecuciones, 0 fallos**; `buf lint` y `buf breaking` contra origin limpios; paso de minuto 18,72-20,83 s en `0f2ddbd` y 19,54-20,68 s integrado (+2,8 % de media, dentro del ruido de la máquina; mismos 3.061.149 pasos y versión 50.132).

## Avance de 5G-b (2026-10-24): la refundación y la Caravana de Fundación

Segunda mitad de la tanda 5G, en la rama `tanda-5g-b` (worktree aparte, sobre `0f2ddbd`), en paralelo con 5C. Foto **`BAN31`** (`BAN30`, la de 5C, se rechaza: integrado sobre `b1f4cf8`).

**La Caravana de Fundación es una entidad** (`Estado::fundacion`, `crates/nucleo/src/estado_fundacion.rs`; reglas en `nucleo/fundacion.rs`). Deja de ser la marca `Ejercito::caravana_fundacion` (se retira de la columna y de su codec). Cada una lleva su Facción, su titular, su origen (plaza, campamento o ninguno: la de la administración), dónde está, lo que costó, los aportes de cada héroe (la de un campamento), su situación (`Disponible`, suelta y parada en su origen; `Adjunta(ejército)`; o `Retornando(plan)`) y su caducidad. Toma su id del contador de las caravanas comerciales, así que en el contrato es una `Caravana` más (`car-N`, tipo `CARAVANA_DE_FUNDACION`, con `titular_id`, `caduca_en_ms`, `aportes` y `origen_campamento_id`). El fondo de refundación es `campamento → héroe → recurso`, en unidades enteras.

**Sin tick.** Dos citas de la cola de prioridad 6 (`Carga::Fundacion`: `Caduca`, a las 48 h de la suelta, y `Llega`, al fin del plan de la que vuelve sola), con la generación de `entidad_fundacion(id)`: cada cambio de situación la sube, así que a la enganchada no le queda nada en la cola. Lo que el TypeScript miraba en cada tick (¿la sigue llevando su titular?) lo mira `Nucleo::vigilar_fundaciones` al cerrar cada versión, solo para las caravanas cuya columna o cuyo titular tocó la transición. Al fundirse dos columnas la caravana sigue con su titular; un ejército que entra en un lugar con ella sigue siendo ejército; la columna que sale del mundo deja la caravana donde estaba (`baja_de_columna`).

**Órdenes (5)**, todas de la cuenta del héroe (la orden del contrato no lo nombra: el de la cuenta). Los códigos empiezan en el **153**: del 143 al 152 quedan para 5C.

| Código | Comando | Orden del contrato | Regla |
|---|---|---|---|
| 153 | `AportarARefundacion { heroe, recurso, cantidad, lado }` | `aportar_a_refundacion` | en un campamento (dentro o con la columna a su puerta); ciudadano de una Facción sin plazas; del almacén personal residiendo allí o del carro siendo el Líder; `min(pedido, lo que hay)` |
| 154 | `RetirarDeRefundacion { heroe, recurso, cantidad, lado }` | `retirar_de_refundacion` | igual, sin mirar las plazas; `min(pedido, lo aportado, el hueco)` |
| 155 | `ComprarCaravanaDeRefundacion { heroe }` | `comprar_caravana_de_refundacion` | sin plazas ni otra Caravana de Fundación, cap ≥ 1 y el fondo de la Facción cubre `costoRefundacion`; se gasta en orden de ingreso de los ciudadanos |
| 156 | `LanzarCaravanaFundacion { heroe, asentamiento }` | `lanzar_caravana_fundacion` | ciudadano que reside y está en la plaza (Proy §5.2.4); nivel operativo 2; cooldown de caravanas compartido; cap con las caravanas vivas; el coste completo |
| 157 | `DesarmarCaravanaFundacion { heroe, caravana }` | `desarmar_caravana_fundacion` | su titular, suelta en su origen; devuelve lo que costó |

Además, `AdjuntarCaravana` (39) engancha la Caravana de Fundación (un ejército de su Facción, a la puerta de donde está, en el que va su titular), `ReclamarCaravana` (141) tiene su caso de éxito (un héroe de su Facción a `radioEncuentro` de la que vuelve sola; su columna personal pasa a ser un ejército con él de Líder y la política de la orden) y `Fundar` gasta la entidad. `DarCaravanaFundacion` (administración) crea una sin origen ni coste, con el Líder de titular.

Rechazos nuevos (10): `fundacion.no_resides`, `fundacion.no_ciudadano`, `fundacion.nivel_insuficiente`, `fundacion.no_titular`, `refundacion.con_plazas`, `refundacion.ya_tiene_caravana`, `refundacion.fondo_corto`, `refundacion.no_resides`, `refundacion.sin_aporte`, `caravana.lejos` (catálogo regenerado). Hechos que pasan a emitirse (los 7 pendientes de Fundacion_Y_Refundacion): `expansion.caravana_lanzada`, `expansion.caravana_desarmada`, `fundacion.caravana_devuelta`, `fundacion.caravana_perdida`, `fundacion.caravana_vuelve`, `mercenarios.caravana_refundacion` y `mercenarios.fondo_refundacion` (con `sentido`); `fundacion.asentamiento_fundado` lleva `caravana`. Proyección, sin cambio de esquema: la caravana en `HEROE` (su titular y quienes van en su columna), `FACCION` y `ADMIN`; el fondo **por Facción** (`CampamentoMercenarios.fondos`, `f-N`) en `CAMPAMENTO` y `ADMIN` (la `VISTA` del campamento no lo lleva). Contrato: las 5 órdenes dejan de dar `orden.no_disponible` (con 5C, quedan 22: 17 de política y 5 del sistema 6); matriz de Proy §5.1 y `Comandos_Parametros.md` al día.

**Pruebas:** `crates/nucleo/tests/fundacion.rs`, 10 sobre el mundo real: el coste; lanzar con cada rechazo y desarmar (lo que vuelve, hasta el hueco del almacén); el nivel 1 y la caducidad a las 48 h; engancharla (columna personal, ejército sin su titular) y fundar con ella; que vuelva sola cuando su titular se separa, los rechazos de otra Facción y de lejos, el reclamo que convierte la columna personal en ejército, la vuelta por dejar de ser ciudadano y la llegada que la desarma, con la foto `BAN31` de ida y vuelta a mitad de camino y `BAN30` rechazada; la de la administración; el fondo (aportar y retirar con cada rechazo, la compra por orden de ciudadanía, lo que queda en el fondo, el desarme que devuelve a nombre de cada aportante y pierde lo de quien dejó la Facción); el **ciclo entero de una Facción sin plazas** (sale del campamento como ejército, destruye bandidos del anillo y cobra su oro por no tener plaza, Cal-5; lo aporta desde el carro; compra; la engancha; funda y pasa a residir); y una por el WebSocket real (aportar con el fondo en el campamento, comprar con la caravana en el delta, desarmar, una forma inválida). Se adaptaron: la cabecera `BAN31` (16 ficheros), el número de comandos y de órdenes no disponibles, la prueba de pueblos que nombraba la caravana 1 (ahora la 1 es la de Fundación de la administración) y las del registro que tocaban `caravana_fundacion`.

**Dudas para el autor (5G-b)**
1. `adjuntarCaravana` con la Caravana de Fundación pide que su titular **vaya en el ejército** que la engancha (el TypeScript solo pedía que nadie más la enganchara si el titular seguía vigente, y la soltaba al tick siguiente si no iba con ella).
2. La de la administración (`DarCaravanaFundacion`) va con su columna sea quien sea su titular y solo se pierde si la columna desaparece: si volviera sola como las demás, `fundacion.sin_fundadores` no podría darse nunca (con la regla real el titular va en la columna y es ciudadano) y medio árbol de pruebas y el batch, que la usan como atajo, cambiarían.
3. `fundar` lo sigue dando el **Líder** de la columna (no el titular, como en el TypeScript): el titular va en ella por la regla de §7, así que la caravana es de un fundador.
4. La Caravana de Fundación no se suelta a mano (ni `soltarCaravana` ni `aparcarCaravana`): para dejarla basta separarse o entrar en su origen, y vuelve sola o se desarma al llegar. ¿Se quiere soltar dentro del campamento, como dice Movimiento §9?
5. La que vuelve sola **no** está en la rejilla de encuentros ni en la `VISTA` ajena: todavía no se puede interceptar (Combate §7.1). Tampoco frena a la columna que la lleva (velocidad 10). Fila de paridad.
6. `desarmarCaravanaFundacion`, como en el TypeScript, no exige que el titular esté en el origen (basta con que la caravana esté suelta allí).
7. El fondo va **por Facción** en `CAMPAMENTO` (el contrato lo dice así): cualquiera que esté dentro del campamento ve el fondo de cada Facción. ¿Se deja así, o solo el de la propia?
8. Reclamar pide `radioEncuentro` (3) entre la columna y la caravana («junto a ella»); el TypeScript usaba `radioReabastecimiento`.
9. Resuelta al integrar 5C: el cap de lanzar y comprar es el de `fundar`, `politica::nivel::cap_fundacion` con el nivel por experiencia (`Nucleo::nivel_de_faccion`).
10. `Fundar` sigue sin deshacer la columna dentro de la plaza nueva (Fundacion §5.5-6): cambiaría cientos de pruebas que siguen usando la columna tras fundar; queda en la fila de paridad.
11. Las caravanas comerciales y las de Fundación comparten contador: la de la administración consume un id de caravana, así que los ids de la flota se corren uno por cada fundación de escenario (una prueba de pueblos nombraba la caravana 1).

### Decisiones del autor sobre las dudas de 5G-b (2026-10-24)

Aplicadas en la rama `tanda-5g-b2` (sobre `c34920d`). Sin estado nuevo: la foto sigue en `BAN31`.

1. **5G-b-1, 5G-b-2, 5G-b-3, 5G-b-6, 5G-b-8 y 5G-b-10:** aceptadas tal como estaban (enganchar con el titular en el ejército; la de la administración va con su columna; funda el Líder; desarmar sin estar en el origen; reclamar a `radioEncuentro`; contador de ids compartido).
2. **5G-b-4, cambio: se suelta a mano, solo dentro de su origen.** `SoltarCaravana` (142) con la Caravana de Fundación vale si el héroe está dentro de la plaza o del campamento de origen con la columna que la lleva; queda suelta y vuelve a caducar a las 48 h. En cualquier otro sitio, `caravana.soltar_solo_dentro`. El residente que entra en su residencia con ella conserva la columna dentro (como con una comercial) y al soltarla la columna se deshace. Canon 1.8 y 5.13.2, Movimiento §9, Fundacion §7.
3. **5G-b-5, cambio: la que vuelve sola está en el mapa.** En la rejilla de encuentros (`Cosa::Fundacion`), en la `VISTA` ajena como caravana avistada y se intercepta con `atacar` (Combate §7.1: defensa de los carreteros, tipo A o B por la zona). Combate §7.1 no dice qué pierde una caravana sin carros: `// DUDA:` en `nucleo/fundacion.rs::atacar_fundacion` — en disputa el que gana se la lleva entera (se pierde) y en aguante pierde el 70 % y sigue volviendo; con Unity se resuelve también con números. **Pregunta al autor:** ¿es ese el reparto, y la de Fundación abre una batalla de Unity como las comerciales?
4. **5G-b-7, cambio: en `CAMPAMENTO` cada sesión ve solo el fondo de su Facción;** `ADMIN`, todos. Fundacion §9.1.
5. **5G-b-9:** sigue en la fila de paridad (`Fundar` no deshace la columna dentro).
6. **5G-b-11, cambio: suelta en una plaza que cae (conquista o ruina), se destruye con ella** y lo que costó se pierde, como las aparcadas (canon 3.13.7). Canon 1.8, Fundacion §7.

**Pruebas nuevas** en `crates/nucleo/tests/fundacion.rs` (6, `decision_5gb*`): soltar en una plaza de origen y en un campamento de origen (con sus rechazos a la puerta y en mundo abierto); el encuentro con una columna enemiga, la `VISTA` ajena y la propia, y la intercepción en disputa (fuera de su zona: se la lleva entera) y en aguante (dentro: 70 % perdido, sigue volviendo y devuelve el resto); el fondo por Facción en `CAMPAMENTO` para cada espectador; y la caravana suelta en una plaza conquistada.

### Decisión del autor sobre la intercepción de la Caravana de Fundación (2026-10-24)

Responde a la pregunta del punto 3. Aplicada en `tanda-5g-b2`, sin estado nuevo (foto `BAN31`).

1. **El reparto provisional queda como regla:** en disputa quien gana se la lleva entera (se pierde; él carga lo que costó, hasta lo que le quepa); en aguante pierde el 70 %, redondeado hacia abajo, y sigue su vuelta con el 30 %. Sin `// DUDA:`; Combate §7.1, canon 1.8, Fundacion §7.
2. **Con servidores de batalla abre una batalla de caravana en Unity**, por el mismo camino que las comerciales (`abrir_batalla_de_ataque`, `Contexto::Caravana`): un solo carro `car-N:0` con lo que costó y los carreteros de defensa (`apertura_de_fundacion` en `nucleo/batallas.rs`). `apartar_caravana`/`reponer_caravana` la dejan quieta durante la batalla y retoman su vuelta; `captura_de_batalla` aplica `caer_fundacion` según el tipo. Sin servidores, con números, como hasta ahora. Prueba nueva en `tests/batalla_unity.rs` por el servidor de batalla real (disputa: el ticket, el resultado, la caravana perdida, la carga del vencedor y la foto).
### Decisiones del autor sobre las dudas de 5C (2026-10-24)

1. **5C-1, la antigüedad de la capital: CAMBIO.** La capital efectiva sin designar es la plaza que lleva **más tiempo en la Facción**, contado desde que es suya y no desde su fundación: conquistar una plaza no cambia la capital. Cada plaza guarda `de_la_faccion_desde` (al fundar, su fundación; al conquistar, el instante de la conquista), en la foto (`BAN31`), y `politica::capital::PlazaDeCapital::desde` sustituye a `fundada_en`. Dip §3.7, G2 2.2. Prueba: `tests/caravanas.rs::la_conquista_pasa_la_flota_hace_volver_a_las_que_iban_y_destruye_la_aparcada`.
2. **5C-2, la escala de la distancia (80 u):** sin cambio; se calibra en 5I con el batch político. Anotado entonces en `Docs/Mecanicas a balancear.md` con las cifras de `batch comercio`; hoy, P1 de `politica.json` y «Calibración del batch político», abajo.
3. **5C-3, experiencia de combate con números: CAMBIO.** Todo combate con números digno la da, sea cual sea el objetivo: también el saqueo de un pueblo y el ataque a una avanzada (con su `Informe`) y el asalto a una caravana (digno por el poder del atacante contra la defensa; solo gana la Facción atacante, la caravana no lleva héroes). `Nucleo::experiencia_de_asalto`. Combate §4 y §7.1, Dip §3.5, G2 2.2.1. Pruebas en `tests/combate.rs`.
4. **5C-4, bandidos en Unity: CAMBIO.** Destruir un campamento de bandidos en Unity da **solo** la experiencia de bandidos, no además la de la batalla: el mismo premio con Unity y sin él. Batalla_Unity_Ciclo §9.6, Combate §7.2, G2 2.2.1. Prueba en `tests/batallas.rs`.
5. **5C-5, la Gran Fundición:** una por Facción venga de donde venga (como está).
6. **5C-6, renombrar: CAMBIO.** Como mucho `NOMBRE_ASENTAMIENTO.largoMaximo` = 32 caracteres (bloque nuevo en `Economia_Construccion.md` §12, leído por `asentamiento::Reglas::largo_maximo_nombre`) y único entre las plazas vivas del mundo, sin tildes ni mayúsculas (`politica::nombres::normalizar`). Rechazos nuevos `asentamiento.nombre_largo` y `asentamiento.nombre_duplicado`. G1 1.2. Prueba en `tests/politica.rs`.
7. **5C-7 a 5C-11:** decisiones técnicas, aceptadas por el orquestador (5C-8: un hecho por edificio completado está bien). 5C-12 (atribución de un commit): no se reescribe historia.

## Avance de 5D (2026-10-24): las relaciones entre Facciones en el núcleo

Hecha sobre `9b4efce` (las correcciones de 5C), rama `tanda-5d`. Foto **`BAN31`** en la rama, **`BAN32`** al integrar (la misma que las correcciones de 5C, en la misma rama: `BAN30` se rechaza con su motivo). Código en `crates/nucleo/src/nucleo/diplomacia.rs`; pruebas en `crates/nucleo/tests/diplomacia.rs`.

**Diseño**
- **Las relaciones viven en `Estado::politica`** (el `politica::Relaciones` de 5A, con su codec de 5B): desaparece `EstadoMilitar::relaciones` (el mapa de pares del andamiaje). `Estado::relacion(a, b)` da la clase de la **activa** del par por el índice del almacén (O(log n)), y sobre ella `Estado::{trato, defiende_a, aliadas}` (lo que miraban encuentros, defensa, puerta, unirse a batallas, visión compartida y tipo A/B de caravana: no cambian). En `politica` se añadieron el índice vasalla → vasallaje (`senora_de` en O(log n), para el tributo en el paso de minuto; un vasallaje con una vasalla que ya lo es de otra se rechaza, `YaEsVasalla`) y `Relaciones::arrastre` (el bando arrastrado, sin declarar: lo que enseña `guerra.confirmar`). Los ids de relaciones y propuestas salen del **contador común** (decisión 3), mirando y gastándolos solo si la regla acepta (un rechazo no deja rastro).
- **Órdenes 144-152** (154-162 en la rama; al integrar, 5G-b ya había tomado 153-157), mapeadas al contrato: `ProponerRelacion`, `ResponderRelacion`, `RetirarRelacion`, `DeclararGuerra`, `ProponerPaz`, `RomperRelacion`, `RebelionVasallo` (clase nueva de `Autoriza`, `AutoridadDiplomatica`: Rey o Embajador de la Facción que actúa; `ResponderRelacion` y `AdmitirOtrasFacciones`, `ReyDeLaFaccion`), `AdmitirOtrasFacciones` y `AlternarReabastecerAliados` (`MandaEnLaPlaza` hasta 5E). `ProponerRelacion` crea una propuesta (`TipoCreado::Propuesta`, `prop-N`); `ResponderRelacion` revalida y no acepta con una batalla abierta de alguna de las dos.
- **La guerra al atacar** (Q2): `comprobar_agresion` rechaza contra la propia señora o vasalla (`combate.vasallaje`, nuevo), deja pasar la guerra y, con `declara_guerra`, declara con `Relaciones::declarar_al_atacar` (arrastre y traición contra una aliada: rompe la alianza con `rotaPor`, −25 y nada más). Sin `declara_guerra`, `guerra.confirmar` lleva el arrastre en `Detalle::Guerra` y el mensaje avisa de la traición. `combate.aliada` queda retirado en el catálogo. `DeclararGuerra` (la orden) contra una aliada sigue exigiendo romper antes, y cobra la guerra tras romper.
- **Paz, ruptura y rebelión**: las de `politica`, con su reputación; la rebelión expira sin penalizar los trueques **activos** entre plazas de las dos (`comercio.trueque_cancelado`).
- **Desarme del señor y vasalla conquistada** en el instante en que una Facción pierde su última plaza (`registrar_derrota`, por conquista y por ruina), en ese orden (C7); la vasalla conquistada también al aceptar una señora nueva sin plazas (Dip §12).
- **Tributo** (Q3, 5A-2): al final del paso de minuto de cada plaza vasalla, tras comer su guarnición; `politica::tributo::pago` con el resto oculto en `Asentamiento::resto_tributo` (foto); lo que no cabe en la capital de la señora se paga y se pierde; sin capital, no se paga. Un `diplomacia.tributo_pagado` por plaza que paga. No marca nada en el registro (los almacenes son del interior, que va por su secuencia de pasos de minuto). Sin vasallaje, el paso de minuto paga una consulta a un índice vacío.
- **Reputación**: los `Suceso`s de `politica` en `comercio::reputacion` en el instante del hecho, y las **alianzas activas** de cada Facción en el ritmo al cambiar (`recontar_alianzas`). **Defender al vasallo** (§5.9) tras cada combate con números cuyo defensor es de una vasalla (columna, caravana —la señora nunca está—, asedio, pueblo y avanzada) y tras el resultado de Unity (los defensores que participaron), antes de las consecuencias (una conquista cambiaría la señora).
- **Admitir otras Facciones**: `rey_admite_otras` del reparto de las batallas es el `admite_otras_en_ataques` de la Facción atacante. **Reabastecer aliados**: `Asentamiento::permite_reabastecer_aliados` (foto; la conquista lo cierra y borra el resto del tributo, como el acceso) y `Nucleo::abre_su_almacen_a` en `Repostar` y en `CargarCaravana` (cierra la duda 3 de 5G-a).
- **La administración** (decisión 9): `FijarRelacion` rompe la activa del par y crea la pedida con `a` señora o declarante, sin propuesta, arrastre ni reputación; las Facciones que no existen nacen vacías (también al declarar al atacar).
- **Contrato y proyección**: `RelacionPolitica` entera (dirección, tributo, `creado_en_ms`, `rota_por`, `rota_en_ms`, `paz_propuesta_por`) mientras está activa, en `FACCION` de las dos, en `VISTA` de todos (son públicas: siempre, sin rectángulo) y en `ADMIN`; al romperse, `QUITA`. La clase `Relacion` del registro va por el id de la relación (`rel-N`; antes, el par). Sin cambio de esquema: `buf lint` y `buf breaking` limpios.
- **Hechos**: los doce `diplomacia.*` de las relaciones (`relacion_propuesta`, `_aceptada`, `_rechazada`, `_retirada`, `relacion_rota`, `guerra_declarada` —sustituye a `faccion.guerra_declarada`, con `traicion` y la lista `arrastradas`, que el `GuerraDeclarada` tipado lleva en `arrastradas_ids`—, `paz_propuesta`, `paz_firmada`, `rebelion_vasallo`, `vasallo_liberado`, `vasallo_conquistado`, `tributo_pagado`), `faccion.admision_en_ataques` y `logistica.reabastecer_aliados`; `faccion.relacion` se retira. La crónica canta la guerra, la rebelión, la liberación y el traspaso de una vasalla. 18 rechazos nuevos (17 `diplomacia.*` y `combate.vasallaje`) en el catálogo, regenerado.
- **Matriz**: 152 comandos, 128 de órdenes del jugador, 18 órdenes no disponibles (`asignar_cargo_local`, `vetar_jugador` y las seis de anexión y fusión, más las del sistema 6 y la refundación).

**Pruebas** (`tests/diplomacia.rs`, 11): proponer y aceptar con su autorización, una sola pendiente, rechazar, retirar y caducar sin rastro; la guerra con arrastre a la señora y a las demás vasallas (y su tipado); `guerra.confirmar` con el arrastre y la traición (−25, la alianza rota, la guerra arrastrada); romper (−12), la guerra tras romper (−15) y la paz mutua; liberar (+6) y la rebelión (−10, la guerra de la vasalla); admitir otras Facciones; el tributo de 1,5 por minuto en enteros y el que no cabe en la capital; la vasalla conquistada por un tercero (mismo tributo, la guerra arrastrada a la señora, −3 por no defenderla) y el desarme del señor; repostar en una aliada solo si lo permite; determinismo normal y a trozos, la foto `BAN31` con cada pieza moviendo el hash, ninguna cita de relaciones ni propuestas en la cola. Se adaptaron con razón: los rechazos `combate.aliada` (ahora vasallaje), los ids `rel-a-b` (ahora `rel-N`, buscados), `faccion.guerra_declarada` y `faccion.relacion` (ahora `diplomacia.*`), la crónica de una guerra de la administración en `tests/ordenes.rs` y una orden no disponible de `tests/sesiones.rs`; 9 casos nuevos de la traducción y el ida y vuelta de los 9 comandos.

**Cifras antes de integrar.** `cargo test --workspace --no-fail-fast`: 90 resultados, todas en verde. Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): 41 resultados, 501 pruebas, 0 fallos. Las 94 pruebas de sesión (14 binarios, con `politica` y `diplomacia`) 20 veces seguidas: 1.880 ejecuciones, 0 fallos. Paso de minuto (1.000 plazas × 3 días en release, alternando el árbol de `9b4efce` y el de 5D, tres veces cada uno, sin sesiones): 18,62 / 20,33 / 19,77 s antes y 19,50 / 23,39 / 17,15 s ahora (máquina cargada; mismos 3.061.149 pasos y versión 50.132). `buf lint` y `buf breaking` limpios.

**`batch militar`** (release, 60 plazas × 30 días, semilla 42; antes, el árbol de `9b4efce`; las alianzas por parejas ya no las fija la administración: el Rey de la impar propone y el de la par acepta, y la guerra se declara al atacar con la regla de 5D):

| | antes (`9b4efce`) | después (5D) |
|---|---|---|
| Guerras declaradas al atacar | 12 | 12 |
| Asedios abiertos / plazas conquistadas | 26 / 26 | 26 / 26 |
| Ataques a avanzadas que ganó el atacante | 462 | 462 |
| Campamentos de bandidos destruidos | 1.531 | 1.531 |
| Acosos de bandidos a caravanas | 57 | 57 |
| Órdenes dadas | 382.486 | 382.496 (+10: las cinco propuestas y sus respuestas) |
| Violaciones (todos los invariantes) | 0 | 0 |
| Foto ida y vuelta a mitad (`BAN31`) | igual | igual |
| CPU por día de mundo | 12,7 s | 12,4 s (los dos a la vez en la máquina) |

Todas las demás métricas del informe son iguales: en el batch no hay vasallajes, las alianzas son las mismas (ahora con propuesta) y ningún bot ataca a un aliado, así que la regla nueva no cambia lo que pasa; su ritmo de reputación (+0,0005 por minuto y alianza) no supera al decaimiento.

**Dudas para el autor**
1. Las relaciones activas son **públicas**: van a la `VISTA` de todos, sin niebla. ¿O solo las de las Facciones que se ven?
2. Solo se proyectan las **activas**; una rota sale del almacén del cliente (`QUITA`) y el hecho dice cómo acabó (`rota_por`, `rota_en_ms` no llegan a verse). ¿Se proyectan también las rotas recientes?
3. El tributo se paga **al final** del paso de minuto de la vasalla, después de comer su guarnición. ¿O antes de que coma?
4. Una guerra o una alianza que fija la **administración** emite los `diplomacia.*` y la crónica la canta como una de verdad.
5. `alternar_reabastecer_aliados` lo da quien manda en la plaza hasta 5E (luego, el Gobernador o el Tesorero). La conquista cierra el almacén a los aliados y borra el resto del tributo. Repostar en una **avanzada** aliada sigue cerrado.
6. La defensa del vasallo se aplica también al saqueo de un pueblo y al ataque a una avanzada (el TypeScript no los tenía); en una caravana suelta la señora nunca está presente.
7. «Batalla abierta» al aceptar una relación es una batalla activa en la que alguna de las dos es el atacante o el defensor del ticket (no basta con un héroe suyo incorporado).
8. Las **propuestas** no se proyectan todavía (5H): un cliente que reconecta no las recupera; viajan en `creadas` y en sus hechos.
9. `ProponerRelacion` de una alianza con tributo se rechaza (`diplomacia.tributo_invalido`) en vez de ignorarlo; con uno solo de los dos campos del tributo, el otro toma su valor por defecto (trigo, 0).
10. La administración salta la autoridad diplomática en las nueve órdenes (como en el resto de las órdenes del jugador).
11. Los ids de las relaciones salen del contador común: `FijarRelacion` gasta uno, y cambia los ids de lo que se crea después en los escenarios de administración.

**Integración (2026-10-24, sobre `c34920d`: 5G-b).** Foto **`BAN32`** (5G-b tomó `BAN31` con la Caravana de Fundación como entidad; `BAN31` se rechaza con su motivo —las plazas no dicen desde cuándo son de su Facción y las relaciones son pares sin dirección— y `BAN30` con los dos). Los comandos de 5D pasan a **144-152** (5G-b tomó 153-157, dejando el hueco de 5D); la matriz y los recuentos quedan en 157 comandos, 133 de órdenes del jugador y 13 órdenes no disponibles (`asignar_cargo_local`, `vetar_jugador`, las seis de anexión y fusión y las cinco del sistema 6); el catálogo de rechazos, regenerado. Tras integrar: `cargo test --workspace --no-fail-fast` **92 resultados, 1.418 pruebas en verde, 13 ignoradas**; auditoría **42 resultados, 511 pruebas, 0 fallos**; las 104 pruebas de sesión (15 binarios, con `politica`, `diplomacia` y `fundacion`) **20 veces seguidas: 2.080 ejecuciones, 0 fallos**; `buf lint` y `buf breaking` contra origin limpios; paso de minuto (1.000 plazas × 3 días en release, sin sesiones, alternando con el árbol de `c34920d`): 16,72 / 19,81 / 15,83 s el de origin y 18,29 / 16,42 / 19,97 s el integrado (medias 17,5 y 18,2 s, dentro del ruido de la máquina; mismos 3.061.149 pasos y versión 50.132). El `batch militar` (semilla 42) integrado da las mismas métricas de juego que el de la tabla de arriba, con la foto `BAN32` ida y vuelta igual.

**Decisiones del autor sobre las dudas de 5D (2026-10-25).** 1 (relaciones activas públicas en `VISTA`, sin niebla), 3 (el tributo al final del paso de minuto, cuando la vasalla ya ha comido), 6 (la defensa del vasallo también en pueblos y avanzadas) y 8 (las propuestas se proyectan en 5H): como están. 2, 4, 5, 7, 9, 10 y 11: técnicas, aceptadas. 12 (atribución de un commit): no se reescribe historia. Registro: `Consideraciones/Politica_Tecnologia_Decisiones.md`.

**Segunda integración (2026-10-25, sobre `7ea2ba2`: las decisiones de 5G-b y la intercepción de la Caravana de Fundación).** Sin cambio de foto (sigue `BAN32`). Conflictos solo en documentos (Combate §7.1, la fila de paridad de la Caravana de Fundación y este plan). Por coherencia con 5C-3 y con §5.9, el asalto con números a la Caravana de Fundación que vuelve sola (`Nucleo::atacar_fundacion`) da la experiencia de un combate digno y mira la defensa del vasallo, como el de una caravana comercial. `cargo test --workspace --no-fail-fast`: 92 resultados, 1.425 pruebas en verde, 13 ignoradas; auditoría: 42 resultados, 518 pruebas, 0 fallos. Las 111 pruebas de sesión (15 binarios) 20 veces seguidas: 2.220 ejecuciones, 0 fallos; `buf lint` y `buf breaking` limpios.

## Avance de 5E (2026-10-25): los cargos locales, la matriz de autorización y la puerta

Hecha sobre `3be9b7f` (5D integrada con 5G-b), rama `tanda-5e`, en paralelo con 5F. Foto **`BAN33`** en la rama (`BAN32` se rechaza con su motivo). Código en `crates/nucleo/src/nucleo/cargos.rs`; pruebas en `crates/nucleo/tests/cargos.rs`.

**Diseño**
- **Estado.** Cada plaza guarda sus cargos (`Asentamiento::cargos`, el `politica::Cargos` de 5A: el titular de cada uno de los cinco) y su puerta (`Asentamiento::puerta`, `politica::puerta::Puerta`: los grupos cerrados —`None` = los de `PUERTA.cerradaAPorDefecto`, neutrales y enemigos— y los vetados, ordenados), que sustituye al `Acceso` provisional (desaparece de `estado_militar`). Foto `BAN33`. `EstadoMilitar` gana un índice cuenta → héroes (`heroes_de_cuenta`, derivado; `heroe_de_cuenta` deja de recorrer todos los héroes).
- **La matriz** (Proyecciones §5.2, que pasa a ser §5.1 entera). Se retiran `manda_en` y `Autoriza::MandaEnLaPlaza`; `Autoriza` gana `Cargo(&[Cargo])` (titular de alguno de esos cargos, residente y dentro: `cargo(A, c)`), `CargoDeLaOrden` (`activar_politica`, `comprometer_recinto`, `mejorar_recinto`: el cargo que declara el cliente), `ResidenteDentro` (`reside(A)`, `resideEnOrigen`: crear y montar caravanas, prepararlas, cancelarlas), `OperaAparcada` (`puedeOperarAparcada`), `AsignaCargoLocal`, `FijaPuerta` y `CompraIntel`. Una sola función las aplica (`Nucleo::autorizar_en_plaza`) con los predicados de `politica::autorizacion::Actor` sobre el núcleo (`impl Vista for Nucleo`, que ya da los cargos de verdad). «La ciudadanía habilita, la presencia ejerce»: el cargo exige ser su titular, residir en la plaza y estar dentro; el Rey designa al Gobernador y fija la puerta sin estar presente. Las órdenes de plaza del contrato no nombran héroe: vale cualquiera de los de la cuenta, buscados por el índice (O(log n) más los de la cuenta: uno para un jugador). La administración puede con todo, como siempre.
- **Qué pide cada orden.** Obras de la cola, auto-construcción y recetas: el Gobernador o el Maestro de Obras; ascenso y abandonar un recinto: el Gobernador; la reserva: el Tesorero; trueques, órdenes de mercado (también cancelarlas) y abrir el almacén a los aliados: el Gobernador o el Tesorero (el de la plaza A al proponer y el de la B al contestar); pueblos (marcar, destruir, reponer el carro) y avanzadas (mejorar, reparar, destruir, cancelar la obra): el Gobernador o el Maestro de Obras de la madre, dentro (el panel de la plaza madre, Doc 4.2.2); elegir la tropa de serie de una avanzada: el Gobernador (Doc 5.17); guarnecer y retirar de una avanzada: la cuenta del héroe (como `asignar_guarnicion`). La intel de la taberna de una plaza: el Rey, el Embajador o el Gobernador, dentro (la columna a la puerta ya no basta). Colocar una avanzada sigue siendo de un ciudadano de la madre con la columna en el sitio (`Autoriza::CiudadanoDeLaMadre`, duda 1). `activar_politica` ya no da el cargo por ocupado (`Activador::cargo_ocupado` es el titular; la administración lo da por cubierto).
- **Los cargos** (`AsignarCargoLocal { heroe, asentamiento, cargo, designado }`, 158): al Gobernador lo nombra el Rey sin estar presente; a los otros cuatro, el Gobernador dentro, y solo con Gobernador (`cargo.sin_gobernador`); el designado es ciudadano de la dueña (`cargo.no_ciudadano`) y reside en la plaza (`cargo.no_reside`, Q4); sustituye al titular anterior; ya titular, `sin_cambios`. Se vacían al dejar de residir (toda mudanza pasa por `mudar_de_residencia`: las tres órdenes de residencia, dejar la Facción, la reubicación por ruina, el desalojo y `FijarResidencia`), al perder la ciudadanía de la dueña sin mudarse (`FijarFaccionDeHeroe`) y todos en la conquista. Una plaza nueva no tiene Gobernador.
- **La puerta (Q1).** `FijarPuerta { heroe, asentamiento, cerrada_a }` (159): el Gobernador dentro o el Rey sin estar presente; `VetarJugador { heroe, asentamiento, vetado, vetar }` (160): solo el Gobernador; a un residente no se le veta (`puerta.residente`). Por capas (residente > vetado > propia Facción > grupo); cerrar o vetar saca al mapa a quien está dentro de la plaza, de sus pueblos o de sus avanzadas y ya no puede entrar; con un pueblo o una avanzada de la plaza en batalla, un jugador no la toca. La conquista la devuelve a la de por defecto y borra los vetos. Un vetado no repone en la plaza (`puedeReclutarEn`) ni se muda a ella (`residencia.vetado`). `FijarAcceso` (117) queda en el núcleo como **alias**: cierra los grupos que van a `false` y deja los Aedas como estaban; ninguna orden del contrato lo da ya (`fijar_puerta` va a `FijarPuerta`).
- **Roles técnicos por membresía (D19).** `servidor::Identidad::rol` (`RolTecnico`: jugador, administrador de partida, moderador, administrador global; lo da el servicio de acceso). En una sesión `CLIENTE_ADMIN_WEB` el moderador ve `ADMIN` y no da ninguna orden (`no_autorizado`); los administradores dan las de administración. El autenticador de prueba da `ok:` = administrador de partida y `mod:` = moderador.
- **Contrato.** Las tres órdenes mapeadas (`asignar_cargo_local`, `fijar_puerta`, `vetar_jugador`; quedan 11 no disponibles: las seis de 5F y las cinco del sistema 6); 160 comandos (136 de órdenes del jugador con el alias, 19 de administración, 5 del servidor de batalla). Proyección aditiva sin cambio de esquema: `Asentamiento.cargos`, `puerta_cerrada_a` (ahora con `GRUPO_AEDAS`) y `vetados_ids` en el interior de la plaza (`ASENTAMIENTO`, y `ADMIN`, que lo usa); `buf lint` y `buf breaking` limpios. Hechos `cargo.local_asignado` (con el `cargo` como texto), `asentamiento.puerta_fijada` (sustituye a `asentamiento.acceso`; `cerrada_a` como lista) y `asentamiento.veto`; cuatro rechazos nuevos (`cargo.no_ciudadano`, `cargo.no_reside`, `cargo.sin_gobernador`, `puerta.residente`), catálogo regenerado.
- **Comandos** del 158 al 160: en la rama eran el 168-170, pero 5F (integrada antes en origin) tomó el 168-173 y dejó el 158-167 para 5E.

**Pruebas.** `tests/cargos.rs` (6, sin andamiaje: cada cuenta da sus órdenes tal cual): quién designa cada cargo y a quién, con todos sus rechazos y el hecho; el cargo que se ejerce dentro y no desde el camino; el vaciado al dejar la residencia, por la reubicación y al perder la ciudadanía; la matriz orden a orden (obras, auto-construcción, reserva, reabastecer, ascenso, el cargo de la orden, caravanas, fuera de la plaza, la administración); la puerta por defecto y por grupos (Aedas incluidos), quién la fija, la expulsión, el veto (el Rey no veta, a un residente no, a la propia Facción sí, levantarlo), el alias; el vetado que no repone ni se muda; la foto `BAN33` con cada pieza moviendo el hash y `BAN32` rechazada. Además: el moderador en `tests/admin.rs`, el cliente de ejemplo que se nombra Gobernador antes de construir (`tests/cliente_de_ejemplo.rs`), la tabla de traducción y la matriz del documento (`sesiones/ordenes/pruebas.rs`), el ida y vuelta de los tres comandos por el registro y la conquista que vacía cargos y puerta (`tests/batallas.rs`).

**Lo que cambia en las pruebas de antes (riesgo R2/R3).** La matriz real rechazaba cientos de órdenes de plaza de los escenarios, escritos con «manda quien es ciudadano». Andamiaje explícito, en `tests/comun`: `mandaba` (una orden de plaza de quien mandaba con la regla de antes —un héroe de su cuenta ciudadano de la Facción de la plaza— se da como administración: decisión 9; quien no mandaba sigue siendo un jugador y la matriz real le rechaza), `como_dueno` (lo mismo para los ayudantes cuya cuenta es siempre la dueña), `gobierna` y `gobernador_de_oficio` (los cargos de verdad para las pruebas de sesión, que van por el contrato). Las pruebas que miden el candado de batalla dan la orden tal cual (`!cuenta`). Los escenarios donde entra un neutral abren la puerta a los neutrales (la de antes de 5E) y las que miraban la puerta por defecto o la de tras una conquista esperan ahora la de Q1 (el neutral sale al conquistar).

**Batches** (release, semilla 42, el árbol de `3be9b7f` contra el de 5E, mismo artefacto de mundo):

| Batch | Antes (`3be9b7f`) | 5E | Lo que cambia |
|---|---|---|---|
| `batch militar` (60 plazas × 30 días) | 12 guerras al atacar, 26 asedios y 26 conquistas, 57 acosos, 1.531 campamentos de bandidos destruidos, 5.140 caravanas lanzadas y 5.133 aparcadas, 1.420.902 eventos, 382.496 órdenes, **`inv_rechazos` 0**, 0 violaciones | **las 92 métricas de juego iguales** (mismos eventos, la foto ida y vuelta igual), **`inv_rechazos` 0**, 0 violaciones, ningún `no_autorizado` | 381.484 órdenes (−1.012: el segundo Guardián de una plaza ya no gobierna, más los `AsignarCargoLocal` del Rey); CPU por día 10,0 → 10,4 s (con la máquina compilando a la vez) |
| `batch comercio` (160 pedidas, 146 colocadas × 30 días) | 125 trueques cumplidos de 127, 338 caravanas, 8 perdidas, 140 plazas vivas, 108 en nivel 2 | 88 de 105 métricas iguales; **los trueques, idénticos** (127 / 125 / 125), ningún `no_autorizado` | 141 plazas vivas (+1), 5 caravanas perdidas (−3, las de la plaza que ya no cae), 109 en nivel 2, acarreo +0,2 %, 6.343.590 → 6.372.077 eventos. Causa probable: cada plaza tiene un héroe y una columna más (su Gobernador de oficio), y los ids de todo lo creado después se corren (cambia el orden de lo que vence en el mismo instante); el residente de más no cambia la economía (la nobleza pide tres) |
| `batch poblacion` (300 pedidas, 230 colocadas × 30 días) | nivel 2 en el 92,17 % a los 5,25 días, 0 violaciones | **86 de 88 iguales** (las 2 distintas son de tiempo de pared) | nada: este batch no da órdenes al núcleo |

**Cifras.** Antes de integrar: `cargo test --workspace --no-fail-fast` **93 resultados, 1.431 pruebas en verde, 13 ignoradas**; auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`) **43 resultados, 524 pruebas, 0 fallos**; las pruebas de sesión (16 binarios: los 15 de 5D y `cargos`) **20 veces seguidas: 2.340 ejecuciones, 0 fallos**; `buf lint` y `buf breaking` limpios (sin cambio de esquema); paso de minuto (1.000 plazas × 3 días en release, alternando el árbol de `3be9b7f` y el de 5E): 15,31 / 15,39 s antes y 15,29 / 15,38 s ahora en las tandas sin la máquina ocupada (mismos 3.061.149 pasos y versión 50.132): la matriz no toca el paso de minuto.

**Dudas para el autor**
1. **Colocar una avanzada.** Doc 4.2.2: el panel de la plaza madre es del Gobernador o del Maestro de Obras presentes, y «la coloca el héroe en el sitio». Hoy la coloca cualquier ciudadano de la madre con su columna en el sitio (la regla de antes, `// DUDA:` en `Autoriza::CiudadanoDeLaMadre`). ¿O tiene que ser su Gobernador o su Maestro de Obras (titular y residente, en el sitio en vez de dentro)?
2. **El cargo de las obras de la cola.** En `anadir_edificio_manualmente`, `quitar_de_cola`, `mover_en_cola` y `mejorar_edificio_ahora` el cliente declara `cargo` (Gobernador o Maestro de Obras) y el comando del núcleo no lo lleva (llevarlo cambiaría el formato del registro de peticiones de órdenes que ya existían): vale el titular de cualquiera de los dos. El TypeScript exigía ser titular del declarado. ¿Se lleva el cargo al comando?
3. **Una plaza recién fundada no tiene Gobernador** (como el TypeScript): hasta que el Rey lo nombre nadie construye a mano, comercia ni pide el ascenso. Los batches crean uno «de oficio» y el cliente de ejemplo se nombra antes de construir. ¿Se queda así, o el fundador (o el Líder de la columna que funda) es su Gobernador al fundar?
4. **La intel de una plaza pide estar dentro**: con la matriz (`compraIntel`), la columna a la puerta ya no basta (el canon dice «estando en ella»). ¿Bien?
5. **Una caravana aparcada**: `mover_carga_caravana_aparcada` y `enviar_caravana_al_origen` miran la plaza donde está aparcada (la de la caravana, no un `asentamientoId` de la orden); para una que no está aparcada (la orden la rechaza después), su origen.
6. **La administración activa una política** con el cargo dado por cubierto aunque no haya titular (`cargo_ocupado = admin ∨ titular`), como antes de 5E.
7. **Vetar con una batalla** en un pueblo o una avanzada de la plaza se rechaza (`batalla.bloqueado`), como fijar la puerta; levantar un veto, no. **Fijar la puerta** con los grupos que ya cierra es `sin_cambios` (también fijar a mano la de por defecto: queda sin fijar). **La conquista** borra también los vetos.
8. **Perder la ciudadanía sin mudarse** (`FijarFaccionDeHeroe` de la administración, que no toca la residencia) vacía los cargos en la plaza donde reside, aunque la regla habla de dejar de residir.
9. **Los batches.** En `batch militar` el Guardián es el Gobernador, nombrado por el Rey con la orden real; en una plaza sin Gobernador (tras una conquista o una mudanza) el Rey nombra a quien gobierna ahora, y si dos Guardianes reubicados residen en la misma plaza solo gobierna el titular (el otro solo pone su leva en la guarnición: 1.012 órdenes menos que antes). En `batch comercio` y `batch territorio` cada plaza tiene un Gobernador de oficio (un héroe nuevo con su cuenta `g{plaza}`, creado por el andamiaje y montado con las órdenes reales: se muda, entra, el Rey le nombra Gobernador y él se nombra Tesorero y Maestro de Obras) que da las órdenes de plaza del «jugador». ¿Bien, o el batch político de 5I lo cambia?
10. **Las pruebas de antes** dan las órdenes de plaza de quien mandaba con la regla de antes como administración (`tests/comun::mandaba`, decisión 9) y montan sus escenarios con la puerta de antes (abierta a los neutrales). ¿Se reescriben con cargos reales en 5I, o se quedan como andamiaje?
11. **Roles técnicos.** El moderador no da ninguna orden; el administrador de partida y el global dan todas las de administración (el andamiaje y el reloj); no hay `cancelarBatalla` de administrador (en Rust la cancela quien la inició). El rol lo da el servicio de acceso (`Identidad::rol`); el de prueba da `ok:` = administrador de partida y `mod:` = moderador.
12. **`FijarAcceso` (117)** queda en el núcleo como alias de `FijarPuerta` (lo usan pruebas y el registro de peticiones viejo; ninguna orden del contrato lo da). ¿Se retira?
13. **El General** no tiene ninguna orden que le pida su cargo (tampoco en el TypeScript): se designa y nada más.

**Integración con 5F (2026-10-25, `28e3cca` de origin en `tanda-5e`).**
- **Foto `BAN34`**: la `BAN33` de 5F (los títulos) más los cargos y la puerta de 5E. `BAN33` se rechaza (sin cargos ni puerta por grupos) y `BAN32` también (sin títulos ni cargos), cada una con su motivo.
- **Comandos**: los de 5E se quedan en el 158-160 y los de 5F en el 168-173. Son 166 comandos: 142 de órdenes del jugador (con el alias), 19 de administración y 5 del servidor de batalla. Ninguna orden de política sigue `orden.no_disponible`: quedan las cinco del sistema 6.
- **Un fallo de la unión, arreglado.** El traslado de 5F cambiaba de Facción a los ciudadanos antes que a las plazas. Con la regla de 5E, «quien deja de ser de la Facción dueña de su plaza pierde sus cargos», cada residente de la absorbida perdía sus cargos aunque su plaza se fuera con él. Ahora las plazas cambian primero (`nucleo/traslado.rs`, pasos 2 y 3; Diplomacia §6.3, «Cargos locales»). Lo comprueba `la_anexion_con_plazas_…`: el Gobernador de B sigue siéndolo tras la anexión. Con el orden de antes, la aserción falla.
- **Andamiaje.** El `E::ord` de `tests/anexion_fusion.rs` usa también `comun::mandaba`. Su trueque entre plazas lo daba el fundador desde fuera, y la matriz real lo rechaza. `sesiones.rs` usa `adoptar_tecnologia` como orden no disponible.
- **Cifras tras integrar.**
  - `cargo test --workspace --no-fail-fast`: **94 resultados, 1.439 pruebas en verde, 14 ignoradas**.
  - Auditoría: **44 resultados, 532 pruebas, 0 fallos**.
  - Las pruebas de sesión y de política, 20 veces seguidas: 17 binarios, los 16 de antes y `anexion_fusion`. **2.500 ejecuciones, 0 fallos**.
  - `buf lint` y `buf breaking` contra `main`: limpios. El esquema es el de 5F.
  - Paso de minuto, alternando con `3be9b7f`: **15,33 / 15,36 / 15,40 s antes y 15,47 / 15,63 / 15,46 s ahora**. Las medianas son 15,36 y 15,47 s (+0,7 %). Hay 392 pasos más, que son los plazos de títulos de 5F (3.061.149 → 3.061.541). El coste por paso sigue en 5,0-5,1 µs.
  - `batch militar`: las mismas métricas de juego que 5E antes de integrar, `inv_rechazos` 0, ningún `no_autorizado`. Hay 2.149 eventos más, que son los plazos de títulos.
  - `batch comercio`: idéntico a 5E antes de integrar.

## Avance de 5F (2026-10-25): la anexión, la fusión, el traslado, la Liga, el Gran Rey y los títulos

Hecha sobre `3be9b7f` (5D integrada, foto `BAN32`), rama `tanda-5f`, en paralelo con 5E (rama `tanda-5e`). Foto **`BAN33`** (`BAN32` se rechaza con su motivo: no trae los títulos de la última evaluación ni su plazo). Código en `crates/nucleo/src/nucleo/traslado.rs` (órdenes y traslado) y `titulos.rs` (títulos, Liga y Gran Rey); pruebas en `crates/nucleo/tests/anexion_fusion.rs`.

**Diseño**
- **Órdenes 168-173** (del 158 al 167 quedan para 5E), mapeadas al contrato (`sesiones::ordenes`): `ProponerAnexion` y `RetirarAnexion` (la autoridad diplomática de la absorbente, `AutoridadDiplomatica`), `ResponderAnexion` (el Rey de la absorbida), `ProponerFusion` y `RetirarFusion` (el Rey de A) y `ResponderFusion` (el Rey de B), las tres últimas con `ReyDeLaFaccion` (Proy §5.2.7); sin clase nueva de `Autoriza`. Proponer da `creadas` = la propuesta (`prop-N`, clase `PROPUESTA`) y aceptar una fusión, la Facción nueva (`f-N`, clase `FACCION`). Las reglas son las de `politica::propuestas` (5A): una sola pendiente de cada clase por par, caducidad al mirarla (sin nada en la cola), revalidación al aceptar y `diplomacia.batalla_abierta` si alguna de las dos ataca o defiende una batalla activa (la misma definición que las relaciones, 5D-7). Rechazos nuevos: `diplomacia.rey_no_valido` y `diplomacia.proponente_ya_no_es_rey`; los demás se comparten con las relaciones (`ya_pendiente`, `propuesta_desconocida`, `propuesta_caducada`, `batalla_abierta`, `sin_rey`, `vasalla_de_tercera`) y con el nombre de una Facción (`faccion.nombre_vacio`, `faccion.nombre_duplicado`); catálogo regenerado.
- **El traslado** (`Nucleo::trasladar`, §6.3): `politica::traslado::{anexionar, fusionar}` aplican lo político (vasallajes del origen al destino salvo que la vasalla sea el destino o ya tenga relación con él; todo lo demás roto sin reputación; propuestas que nombran al origen descartadas; ciudadanos del origen, en su orden, al final de los del destino; `derrotadaPor` redirigido; el origen desaparece) y el núcleo, en la misma transición: la copia de la ciudadanía, la columna de quien va solo y la demanda de bandidos de su campamento (`tras_cambiar_de_faccion`); las plazas con todo lo que cuelga de ellas (cargos locales, guarnición, residentes, flota, pueblos, avanzadas, que el registro de pertenencias recoloca solo), sin la marca de capital del origen (en cualquier plaza) ni el resto de su tributo; en la anexión, las plazas son del destino **desde ese instante** (`de_la_faccion_desde`: la capital del destino no cambia, como en una conquista, 5C-1), en la fusión conservan su antigüedad (Doc 2.6: la capital de C sin designar es la plaza más antigua de las dos); las columnas, las convocatorias, las Caravanas de Fundación y las rutas de la red del origen; la reputación y las tecnologías del andamiaje del origen se borran (su cita de reputación muere con su generación); una Facción derrotada que recibe plazas deja de estarlo (§2.1); si una de las dos no tenía plazas se rehacen las demandas de los anillos de bandidos; y quien queda dentro de un lugar que ya no le admite (un aliado del origen dentro de una plaza suya; un ciudadano del origen dentro de la de una enemiga del destino) sale al mapa con lo suyo (`echar_si_ya_no_entra`, `heroe.expulsado` con `puerta_cerrada`).
- **La memoria de la niebla** se funde al cerrar la versión, **después** de `explorar` (`Nucleo::fundir_memorias`, con la lista `uniones_de_memoria` de la transición): así lo que vieron los ojos del origen hasta ese instante (sus columnas barren con la bandera de antes) queda grabado y luego pasa al destino: lo explorado por unión de bits, las fichas y los Informes con los del destino delante y sin los de sitios que ya son propios, y las Miradas, al destino. La memoria del origen desaparece.
- **La fusión** (`Nucleo::fusionar`): C toma el id siguiente al mayor del mundo, que nunca es el de una Facción desaparecida (`Politica::mayor_desaparecida`, estado nuevo); antes de los dos traslados se le dan la reputación mayor de las dos (en ese instante, con sus alianzas a 0: nace sin ellas), la unión de las tecnologías del andamiaje y la marca de capital que había designado el Rey electo (5A-5); la de la otra se borra.
- **Q5**: los trueques no se tocan. Van entre plazas y `comercio` mira la Facción de cada plaza al cumplirse o vencer: los que quedan dentro de la misma Facción ya no dan ni quitan reputación ni experiencia.
- **Los títulos** (`titulos.rs`, §7): un solo plazo **global** en la cola (`Carga::Titulos`, `entidad_titulos`, prioridad 10) a `TITULOS.cadenciaMinutos` (bloque nuevo en Dip §7, 10 min, placeholder) de lo primero que pasa en el mundo (una orden aceptada o un evento de la cola que no sea él mismo) sin otro pendiente (`Estado::titulos_en`); al vencer recalcula todo (`politica::titulos::calcular` con las medidas de cada Facción como entidad: plazas, población, oro en unidades, soldados de todos sus ciudadanos; Mecenas a 0) y narra contra los guardados (`Politica::titulos`, foto): `titulo.nace` y `titulo.cambia_manos`, públicos y cantados en la crónica. No se rearma solo: un mundo quieto o sin Facciones no deja nada en la cola. **La Liga y el Gran Rey** se derivan bajo demanda (`Nucleo::ligas`, `politica::liga`); el Gran Rey es un título. Coste: una comprobación de un `Option` por evento y un recálculo O(plazas + escuadras de los ciudadanos) cada 10 minutos de mundo.
- **Proyección** (aditiva, sin cambio de esquema): `Clase::Titulo` (la 17.ª del registro de cambios, id = su posición en `TituloId::TODOS`) va a la `VISTA` de todos (público, como las relaciones) y a `ADMIN`; un título que desaparece sale. La Facción que desaparece ya no existe (`audiencia::existe_faccion`): sale de `FACCION` y `ADMIN` con `QUITA`, y sus ciudadanos pierden el ámbito de la vieja como al cambiar de Facción. Las propuestas de anexión y fusión no se proyectan todavía (5H, como las de relación). El almacén de referencia del contrato (`contrato::almacen`) conoce el `Titulo`.
- **Hechos**: `diplomacia.anexion_propuesta`, `_rechazada`, `_retirada`, `diplomacia.anexion`, `diplomacia.fusion_propuesta`, `_rechazada`, `_retirada`, `diplomacia.fusion` (con `faccionA`, `faccionB` y, en la fusión, `faccion` = la nueva, `rey` y el `nombre`), `titulo.nace` y `titulo.cambia_manos` (con `titulo`, `nombre`, `faccion` y, al cambiar de manos, `faccionAnterior`). La crónica canta la anexión, la fusión y los títulos. Del catálogo del sistema 5 solo quedan sin emitir `cargo.local_asignado` y `asentamiento.veto` (5E).
- **Matriz**: 163 comandos, 139 de órdenes del jugador, 7 órdenes no disponibles (`asignar_cargo_local`, `vetar_jugador` y las cinco del sistema 6). Proy §5.1, `Comandos_Parametros.md` y la prueba de la matriz, al día.

**Pruebas** (`tests/anexion_fusion.rs`, 8 y 1 ignorada de coste): proponer, rechazar, retirar y caducar una anexión con cada rechazo (`comandosAnexion`); el traslado sin plazas (ciudadanos en su orden, columnas, vasallaje heredado con su id y tributo, alianza rota sin reputación, propuestas descartadas y la de otros al destino que sigue, reputación y tecnologías del origen perdidas, el hecho y la crónica, la Facción que sale de `ADMIN`); la fusión (`comandosFusion`: solo el Rey de A, Rey no válido, nombre vacío o de una tercera, el Rey que ya no es el que la propuso, el id de C, nombre, Rey, sigilo de A, experiencia y reputación mayores, la unión de las tecnologías, ciudadanos en orden); el id de una Facción desaparecida que no se vuelve a dar; con plazas (mundo real): la **conservación** (héroes, escuadras, almacenes personales, carros, columnas, víveres, planes, economía de cada plaza, flota, trueques, Caravanas de Fundación y fondos idénticos antes y después), la plaza del destino desde ahora y la capital que no cambia, la memoria fundida y **Q5** (el trueque sigue y al vencer sin entregas no mueve la reputación); la batalla abierta que impide aceptar y el aliado del origen, en guerra con el destino, que sale de la plaza; los títulos (`eventosDominioTitulos`: nacen a su cadencia, públicos, el Gran Rey de una Liga con vasallaje, cambian de manos con la anexión, el Gran Rey que desaparece sin cantar, la crónica, ningún plazo sin Facciones); determinismo normal y a trozos con una anexión, una fusión y los títulos, foto `BAN33` de ida y vuelta, `BAN32` rechazada, los títulos y su plazo que mueven el hash y como mucho un plazo de títulos en la cola. Se adaptaron con razón: la cabecera `BAN33` (16 ficheros); las pruebas que cuentan versiones o eventos de la cola en una ventana en la que vence el plazo de los títulos (`batallas.rs` y `combate.rs` toman la versión tras los 10 minutos; `asentamientos.rs` mide la ventana sin el plazo o lo cuenta; `heroes.rs` no cuenta el plazo global entre las citas de un héroe; el `eventos` de `nucleo.rs` salta los hechos que no son de una columna); el mapa de entidades de `tests/admin.rs`; los recuentos de la matriz y de las órdenes no disponibles; 6 casos nuevos de la traducción y el ida y vuelta de los 6 comandos.

**Cifras** (antes de integrar con origin; se repiten tras la integración en el informe).
- `cargo test --workspace --no-fail-fast`: **93 resultados, 1.433 pruebas en verde, 14 ignoradas** (+8 y +1 ignorada sobre las 1.425 de la segunda integración de 5D). Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): **43 resultados, 526 pruebas, 0 fallos**. `buf lint` y `buf breaking` limpios (sin cambio de esquema).
- Las 119 pruebas de sesión y de política (16 binarios: los 15 de 5D y `anexion_fusion`) **20 veces seguidas: 2.380 ejecuciones, 0 fallos**.
- Paso de minuto (1.000 plazas × 3 días en release, alternando el árbol de `3be9b7f` y el de 5F, tres veces cada uno): **15,85 / 15,87 / 16,45 s antes y 15,72 / 15,73 / 17,82 s ahora** (medianas 15,87 y 15,73 s: sin empeorar); 3.061.149 → 3.061.541 pasos (+392: el plazo de los títulos, cada 10 minutos de mundo) y versión 50.132 → 50.427 (+295: los títulos que nacen o cambian de manos entre mil Facciones de una plaza).
- **Coste de una fusión grande** (release, `coste_de_una_fusion_grande`, dos Facciones de columnas sueltas con un ciudadano cada una y una vasalla): 4.002 columnas y ciudadanos, **1,7 ms**; 20.002, **7,7 ms**; 100.002, **59 ms** (≈ 0,4-0,6 µs por entidad trasladada). Una primera versión comparaba los ciudadanos uno a uno al unirlos (`Vec::contains`, cuadrático: 44 ms con 20.002); ahora mira el índice de ciudadanía de `politica::Facciones`.

**Dudas para el autor**
1. **Los títulos solo se recalculan en su plazo** (cada `TITULOS.cadenciaMinutos` = 10 min de mundo, placeholder, mientras pasa algo), no en el instante en que cambia lo que miran (el canon dice «por dependencia y por muestreo»): una conquista o una anexión cambia los títulos hasta 10 minutos después. ¿Se recalculan además al instante cuando cambian las plazas, las relaciones o los ciudadanos de una Facción?
2. **Quién arma el plazo**: cualquier orden aceptada o evento de la cola (el paso de minuto de cualquier plaza incluido), así que un mundo con plazas recalcula los títulos cada 10 minutos para siempre (432 veces en 3 días). El primer recálculo canta un `titulo.nace` por título. ¿Vale, o solo se arma con lo que mira (plazas, relaciones, escuadras, oro)?
3. **Batalla abierta** al aceptar una anexión o una fusión es la de 5D-7: alguna de las dos es el atacante o el defensor de una batalla activa. Un héroe de la absorbida incorporado a la batalla de una tercera no la impide, y cambia de Facción a mitad de la batalla. ¿Se impide también eso?
4. **Gran Fundición**: si las dos tenían una, la Facción que queda tiene dos (la regla «una por Facción» es de construir; ninguna dice que se pierda una). ¿Se pierde la de la plaza que no es la capital?
5. **La antigüedad de las plazas**: en la anexión, las de la absorbida son de la absorbente desde ese instante (como en una conquista, 5C-1: la capital de A no cambia); en la fusión conservan la suya y la capital de C sin designar es la más antigua de las dos (Doc 2.6). ¿Es así?
6. **Una Facción derrotada que absorbe a otra con plazas** deja de estar derrotada (Mechanics §2.1: `derrotadaPor` solo mientras no tiene plazas). Una C nacida de dos derrotadas sin plazas nace sin `derrotadaPor` (como el TypeScript).
7. **La reputación de C** es la mayor de las dos en el instante de la fusión, con su ritmo sin alianzas (nace sin ellas); si ninguna de las dos tenía reputación todavía, C tampoco.
8. **El id de una Facción que desaparece no se vuelve a dar** (`Politica::mayor_desaparecida`, estado nuevo de la foto): sin esto, una Facción nueva podía tomar el id de la absorbida y heredar sus relaciones rotas (la ventana de «guerra tras romper» las mira).
9. **Quien queda dentro y ya no puede entrar** sale con el motivo `puerta_cerrada` (0), también el que estaba dentro de una plaza de la absorbida que pasa a una enemiga suya; ¿o `cambia_de_dueno` (2), como en la conquista?
10. **El traslado no emite un `diplomacia.relacion_rota` por cada relación que rompe** (como el TypeScript): las relaciones salen del almacén del cliente con `QUITA` y el hecho es `diplomacia.anexion` o `diplomacia.fusion`.
11. **Tecnologías**: la fusión une solo las del andamiaje (`FijarTecnologias`); las aparecidas, adoptadas y reveladas son del sistema 6, igual que el «Mecenas de los Aedas» (vale 0 y no nace) y el traslado de los Aedas residentes y las épicas cumplidas (fila de paridad «Política (5F)»).
12. **La anexión trae la distancia a la capital de A** al mantenimiento de las plazas de B: una plaza de B lejos de la capital de A paga mucho más (la escala se calibra en 5I, 5C-2) y puede arruinarse.

### Decisiones del autor sobre las dudas de 5E y 5F (2026-10-25)

Aplicadas en la rama `tanda-5h` (sobre `ae711cf`), en el primer commit de la tanda. Sin estado nuevo: la foto sigue en `BAN34`.

**5E**
1. **5E-1, colocar una avanzada: CAMBIO.** La coloca el Gobernador o el Maestro de Obras de la plaza madre, titular y residente, con su columna **en el sitio** (no dentro de la plaza; Doc 4.2.2). Se retira `Autoriza::CiudadanoDeLaMadre`: `ColocarAvanzada` pide `Autoriza::CargoEnElSitio(Gobernador, Maestro de Obras)`, que comprueba `politica::autorizacion::Actor::cargo_en_el_sitio` (titular de alguno de los dos y residente, sin mirar la presencia); que la columna esté en el sitio lo sigue mirando la orden (`avanzada.fuera_del_sitio`). Proy §5.1, Economia_Construccion §14.1, canon 4.2.2. Pruebas: `tests/avanzadas.rs::colocar_se_rechaza_sin_rastro_y_en_su_orden` (un ciudadano sin cargo y un residente sin cargo, `no_autorizado`); las demás pruebas de avanzadas colocan con un Maestro de Obras de la administración (`Esc::colocador`, `comun::coloca_avanzadas`).
2. **5E-3, el Gobernador de oficio: CAMBIO.** Quien funda una plaza (el Líder de la columna que funda) es su Gobernador desde la fundación, si es uno de los fundadores (el cargo pide residir, Q4); el Rey puede cambiarlo después. `fundar` lo fija y emite `cargo.local_asignado` tras `fundacion.asentamiento_fundado` (`nucleo/zonas.rs`). Dip §3.4 y canon 2.2. DUDA: un Líder que no es ciudadano de la Facción de la caravana no reside en la plaza y esta nace sin Gobernador, como antes. Pruebas: `tests/cargos.rs` (el Rey fundador es Gobernador de oficio con su hecho; al dejar de residir, el cargo se vacía), `tests/cliente_de_ejemplo.rs` (la fundadora ya no se nombra a sí misma). **Los batches:** el Gobernador de oficio de `sim::gobernador_de_oficio` (batch `poblacion`, `territorio` y `comercio`) **no sobra**: el fundador es la columna de fundación, que sigue en el mapa (la columna no se deshace al fundar, 5G-b-9) y busca el sitio de la siguiente, y el cargo solo se ejerce dentro; el Rey lo sustituye por el de oficio, que vive dentro, con la misma orden de antes. En `batch militar`, el Rey nombra al Guardián en cuanto nacen los héroes de la plaza (`SimMil::nombrar_gobernador`), sustituyendo al fundador; sin este cambio el Guardián no habría gobernado nunca.
3. **5E-4:** comprar intel en la taberna pide estar dentro (como está).
4. **5E-12:** el alias `FijarAcceso` se retira del contrato en 5H (abajo, «Avance de 5H»).
5. **5E-2, 5, 6, 7, 8, 9, 10, 11, 13 y 14:** técnicas, aceptadas por el orquestador (la 10: el andamiaje de las pruebas de antes se revisa en 5I).

**5F**
1. **5F-1:** los títulos, la Liga y el Gran Rey se recalculan cada 10 min en su plazo (como está).
2. **5F-3, la batalla abierta: CAMBIO.** No se acepta una anexión ni una fusión si alguna de las dos ataca o defiende una batalla activa **o si algún héroe de las dos combate en una, como bando o incorporado** a la de otros (`Nucleo::union_en_batalla`, `nucleo/traslado.rs`); las relaciones (5D-7) siguen con la regla de antes. Dip §6.1-§6.2, canon 2.6. Prueba: `tests/anexion_fusion.rs::no_se_acepta_si_un_heroe_de_las_dos_combate_incorporado_en_una_batalla_ajena` (F4 asedia la plaza de F3 y una heroína de F2 se une a la defensa: F1 no absorbe a F2 hasta que acaba).
3. **5F-4:** si las dos tenían Gran Fundición, la resultante conserva las dos (como está).
4. **5F-9, el motivo de quien sale: CAMBIO.** Quien sale de un lugar que tras el traslado ya no le admite lleva el motivo `cambia_de_dueno` (2), como en la conquista; `echar_si_ya_no_entra` recibe el motivo (dejar la Facción sigue con `puerta_cerrada`). Dip §6.3, canon 2.6. Prueba: `tests/anexion_fusion.rs::no_se_acepta_con_una_batalla_abierta_y_quien_ya_no_puede_estar_dentro_sale`.
5. **5F-2, 5, 6, 7, 8, 10, 11 y 12:** técnicas, aceptadas por el orquestador.

**Un fallo del núcleo, hallado al aplicar 5E-1 y arreglado.** `Nucleo::avisar_plan` leía el plan de la columna con `ejercitos[&id]`, y `baja_de_columna` lo llama después de quitarla: con un generador en marcha (la visión) el núcleo entraba en pánico al deshacerse una columna dentro de una plaza (un residente que entra con `Guarnecer`). Ahora el plan de una columna que ya no está se retira (`tests/avanzadas.rs::una_avanzada_operativa_ve_a_12_en_los_limites_y_una_destruida_ya_no_ve`, que lo encontró al pasar a ser residente quien coloca).

## Avance de 5H (2026-10-25): el contrato y las proyecciones de la política

Hecha sobre `5d1c859` (las decisiones del autor sobre 5E y 5F), rama `tanda-5h`. Foto **`BAN35`** (`BAN34` se rechaza con su motivo: sus plazas no dicen quién las fundó). Esquema aditivo; `buf lint` y `buf breaking --against '.git#ref=HEAD'` limpios.

**Diseño**
- **Las propuestas** (relación, anexión y fusión) son la 18.ª clase del registro de cambios (`Clase::Propuesta`, id `prop-N`): van a `FACCION` de las dos Facciones de su par (`audiencia::de`) y a `ADMIN`, enteras (`propios::propuesta`). Viajan mientras están en el almacén de `politica`, también caducadas: caducar no es un hecho (decisión 4), así que el cliente las descarta por `expira_en_ms` y salen con `QUITA` cuando se contestan, se retiran, las poda otra de su clase o las descarta un traslado. Como el almacén cambia también sin que la orden lo diga (la poda, el traslado), `Nucleo::con_lo_politico` envuelve las órdenes de la ciudadanía, la diplomacia, la anexión y la fusión y marca la diferencia del almacén entre antes y después.
- **5A-7, `Propuesta.propuesta_por`: el héroe.** Diplomacia §2.4 y el TypeScript (`propuestaPor` = el actor, o el Rey de A en la fusión) lo dicen así; la Facción que propone es la `a` de su clase. Se corrige el comentario del `.proto` («facción que propone»), sin cambio de tipo.
- **La Liga y el Gran Rey** viajan en cada Facción miembro (`Faccion.liga = 16`, `LigaDeFaccion`: miembros, si tiene vasallaje y su Gran Rey), derivados de las relaciones activas con `politica::liga::liga_de` (la de `ligas` que contiene a la Facción, sin calcular las demás; una prueba los iguala). Al cerrar la versión, cada relación tocada marca las Facciones de la Liga de sus dos extremos (`Nucleo::marcar_ligas`): al nacer una relación la Liga nueva contiene a las dos de antes y al acabar la de antes es la unión de las dos nuevas, así que marcar las de después basta. El Gran Rey es además el título de 5F. No hay entidad Liga (no tiene identidad propia).
- **La Facción ajena en `VISTA`**: cada otra Facción (entidad) que nombra lo que se ve (sus columnas, caravanas, plazas, pueblos, avanzadas, fichas y batallas, y las relaciones y los títulos, que son públicos) en su forma pública (`propios::faccion_publica`: la Facción sin su experiencia, su capital ni cuándo la designó; Proyecciones §6 dice «públicas en la ficción», y dónde está la capital es de la niebla), reconciliada por su versión (su clave y su sello, una vez por perspectiva, `Mirada::publicas`). La propia va entera en `FACCION`. Una primera versión mandaba todas las Facciones a cada vista: +12 % de actor con 500 sesiones y +19 % por orden.
- **El héroe**: `Heroe.faccion_id = 37` (5B-11) y `Heroe.solicitudes_faccion_ids = 38` (las Facciones a las que pidió entrar), en `HEROE` y `ADMIN`. `con_lo_politico` marca al héroe cuyas solicitudes cambian (pedir, contestar, la ciudadanía que las borra todas, la Facción que desaparece con las suyas); la auditoría cuenta las solicitudes en la huella del héroe.
- **Los fundadores** (5B-9, estado nuevo): `Asentamiento::fundadores`, quienes fundaron la plaza y siguen residiendo en ella, el Líder primero (`heroesFundadoresIds` del TypeScript). Quien se muda deja de serlo (`mudar_de_residencia`) y la conquista los borra (`combate.ts`); el traslado no los toca. Viajan en `fundadores_ids` del interior, de la ficha propia de `FACCION` y de `ADMIN`, no en la plaza ajena de la `VISTA`. En el interior viajan también `permite_reabastecer_aliados` (5D) y `ultima_caravana_creada_en_ms`, que salían a su valor por defecto.
- **Ocho hechos tipados de la política** (`events.proto`, `tipado` 25-32; `proyeccion::eventos`): `PropuestaDiplomatica`, `PropuestaResuelta`, `RelacionTerminada`, `FaccionesUnidas`, `CargoAsignado`, `TituloCambiado`, `SolicitudDeIngreso` y `CiudadaniaCambiada`, con los códigos de la tabla del módulo; con `GuerraDeclarada` (5D) son 23 tipados.
- **5E-12, el alias `FijarAcceso` (117) se retira**: desaparece del núcleo (`Comando`, la matriz, el registro: leer un 117 da `comando retirado`); las pruebas que montaban la puerta de antes usan `comun::acceso`, una `FijarPuerta` de la administración. **No había nada que reservar en el `.proto`**: el alias nunca fue una orden del contrato (`fijar_puerta` va a `FijarPuerta` desde 5E).
- **Lo que no se hace**: las relaciones rotas siguen sin proyectarse (5D-2, decidido como está). Ninguna orden de política da ya `orden.no_disponible` (quedan las cinco del sistema 6: 165 comandos, 141 de órdenes del jugador, 19 de administración y 5 del servidor de batalla).

**Contrato y dorados.** `entities.proto`: `Faccion.liga`, `LigaDeFaccion`, `Heroe.faccion_id` y `solicitudes_faccion_ids`, el comentario de `Propuesta.propuesta_por` y de `Asentamiento.fundadores_ids`, y la forma pública de la Facción; `events.proto`: los ocho mensajes y sus campos del `oneof tipado`. Seis dorados nuevos (`foto_faccion_con_propuestas_y_liga`, `delta_propuesta_contestada`, `foto_vista_faccion_publica`, `foto_heroe_con_faccion_y_solicitudes`, `foto_asentamiento_con_fundadores`, `delta_eventos_politicos`), verdes en Rust (`cargo test -p contrato`, que además lee su significado: `los_dorados_de_5h_dicen_lo_que_deben`), C# (`dotnet test`, 81) y TypeScript (`npm test`, 81). El almacén de referencia (`contrato::almacen`) conoce la `Propuesta`.

**El cliente de ejemplo** (`tests/cliente_de_ejemplo.rs`) juega ahora, solo por el contrato y por el WebSocket real, la política: beto nace, pide entrar en la Facción de ana (su solicitud en su `Heroe`; ana la oye tipada y la ve en su Facción), ana le acepta, beto se muda a la plaza y ana le nombra su Gobernador sin estar en ella (la ficha de su plaza lo dice, con sus fundadores); gus crea otra Facción, ana le propone una alianza (la propuesta en las dos Facciones, con quién la hizo, y su hecho tipado), gus la acepta (la propuesta sale, llega la relación y la Liga de las dos) y en la `VISTA` de ana está la forma pública de la Facción de gus. Al final lo de los cuatro clientes es una foto nueva y reanudar no trae nada.

**Pruebas nuevas** (`tests/politica_contrato.rs`, 5, sobre el mundo real): las propuestas en las dos Facciones y fuera al contestarlas, retirarlas y podarlas (la caducada sigue hasta la poda); la Liga en cada miembro, que marca a toda la Liga y cambia al romperse una relación; el héroe con su Facción y sus solicitudes y la forma pública de otra Facción en la `VISTA`; los fundadores (en su orden, Gobernador de oficio, en el interior y en la ficha propia y no en la ajena, foto `BAN35` de ida y vuelta, `BAN34` rechazada, fuera al mudarse); los hechos tipados de la ciudadanía, la diplomacia, la anexión, el cargo al fundar y los títulos. Además `politica::liga` (1) y `comando` (el 117 retirado, 1). Se adaptaron: la cabecera `BAN35` (18 ficheros), el cliente de prueba de `tests/vista.rs` (usa `contrato::almacen::clave` y admite la Facción pública), `tests/entrar.rs` (fijar la puerta pide un héroe con derecho) y `tests/cargos.rs` (el alias).

**Cifras.**
- `cargo test --workspace --no-fail-fast`: **95 resultados, 1.448 pruebas en verde, 14 ignoradas** (+8 sobre las 1.440 de las decisiones de 5E y 5F: 5 de `politica_contrato`, 1 de `politica::liga`, 1 de `comando` y 1 de `contrato`).
- Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): **45 resultados, 539 pruebas, 0 fallos** (la huella del héroe lleva sus solicitudes; la de la Facción, su Liga; la de la plaza, sus fundadores, `permite_reabastecer_aliados` y su última caravana; las propuestas tienen la suya).
- Las pruebas de sesión y de política (18 binarios: los 17 de 5E integrada y `politica_contrato`; 131 pruebas), **20 veces seguidas: 2.620 ejecuciones, 0 fallos**.
- `cargo test -p contrato`, `npm test` en `contratos/ts` (81) y `dotnet test` en `contratos/cs` (81) en verde; `buf lint` y `buf breaking` limpios.
- **Carga** (`tests/carga.rs`, release, 500 sesiones; la base es `5d1c859` archivado aparte, alternando): 5 min de mundo con 500 sesiones, pared del actor **22,37 / 22,31 s la base y 22,56 / 22,40 s ahora** (+0,4-0,8 %); una orden de marcha con las 500 conectadas y el reloj parado, **14,4 / 14,3 ms la base y 15,3 / 14,7 ms ahora** (+2,9-6,6 %); la foto de `VISTA` de un jugador del mapa, 6,2 → 6,6 KB con zstd; los bytes por minuto, iguales. La primera versión, que mandaba todas las Facciones a cada vista, costaba +12 % de actor y +19 % por orden. Versión y hash, los mismos con 1 sesión y con 500 (3.403 versiones).

**Dudas para el autor (5H)**
1. **Qué Facciones ajenas van en la `VISTA`**: solo las que nombra lo que se ve (sus columnas, caravanas, plazas, pueblos, avanzadas, fichas y batallas, y las relaciones y los títulos, que son públicos), no todas como el TypeScript (que las mandaba todas, enteras): una Facción aparece y desaparece de la vista con lo suyo. Mandarlas todas costaba un 12 % de actor con 500 sesiones. ¿Bien así?
2. **La forma pública** no lleva la experiencia, la capital ni cuándo se designó (el TypeScript las mandaba); sí el nivel, la reputación, los dirigentes, los ciudadanos, las solicitudes, la derrota y la Liga (decidido por el autor el 2026-10-26: de la Facción ajena, solo el nivel). Y vuelve a viajar cuando cambia la versión de la Facción, también por lo que no enseña (su experiencia sube con cada edificio).
3. **El coste por orden** sube un 2,9-6,6 % con el reloj parado y 500 vistas (cada sesión reconcilia las Facciones que nombra). ¿Se acepta, o solo se manda la forma pública de las que nombran las columnas y plazas que se ven?
4. **Las propuestas caducadas** siguen viajando hasta que otra de su clase las poda o se contestan (caducar no es un hecho, decisión 4): el cliente las descarta por `expira_en_ms`.
5. **La Liga** viaja dentro de cada Facción miembro (`Faccion.liga`), sin entidad propia ni id; cuentan las Facciones que existen como entidad.
6. **Los fundadores** solo viajan en la plaza propia (interior, `FACCION`, `ADMIN`), no en la ajena de la `VISTA`; quien se muda deja de serlo y no vuelve a serlo si regresa (como `heroesFundadoresIds`).
7. **`FijarAcceso`**: se retiró del núcleo y del registro (un 117 ya no se lee); en el `.proto` no había nada que marcar como `reserved`, porque el alias nunca fue una orden del contrato.
8. **`PropuestaResuelta`** también tipa la `diplomacia.relacion_aceptada` de una relación que fija la administración, sin `propuesta_id`.
9. **El héroe público** (`HeroePublico`, `VISTA`) sigue sin `faccion_id`: la Facción de lo que se ve va en su columna.

## Avance de 5I (2026-10-25): el batch político

Hecha sobre `12b1f21` (5H, foto `BAN35`), rama `tanda-5i`. Sin cambio de foto, de esquema ni de reglas: el núcleo no cambió. Código en `crates/batch/src/simmil/pol.rs` (los bots y los invariantes) y `crates/batch/src/armarpol.rs` (el informe); prueba corta en `crates/batch/tests/politica.rs`; conclusiones y preguntas en `crates/batch/decisiones/politica.json`.

**Diseño**
- **`batch politica [--plazas 60] [--dias 30] [--semilla 42] [--nobleza 1]`** es un modo del batch militar (`simmil::simular_politica`): la misma población (tres plazas por Facción, cuatro héroes bots por plaza con sus papeles, combates con números y todos los invariantes del militar), con la política en manos de **bots de diplomacia deterministas** que solo dan órdenes del contrato. El militar sin el modo da la misma huella que antes (6605fff2…).
- **Al nacer:** el Rey de cada Facción es un héroe del campamento que da `CrearFaccion` y se muda a su primera plaza (`CambiarResidencia`); los cuatro héroes de cada plaza piden el ingreso (`SolicitarIngreso`), el Rey los acepta (`ResponderSolicitud`) y se mudan; el Rey nombra Gobernador al Guardián y Embajador a un Intendente, y el Gobernador nombra Tesorero, General, Maestro de Obras y Sacerdote. El andamiaje que queda: el nivel de partida (`FijarNivelFaccion` a 3: cap 3 y una plaza de nivel 2), la Era I (`FijarTecnologias`, sistema 6), la colonia de cada plaza (`crear_ejercito` + `DarCaravanaFundacion`) y colocar a cada héroe dentro al nacer (`ColocarHeroe`).
- **Cada 4 h, el turno de cada Facción** (su Rey, o su Embajador la mitad de las veces que la orden lo deja): contesta las solicitudes (admite el 80 %) y las propuestas (alianza 70 %; vasallaje 75 % si es más débil; anexión 25 % sin plazas, 50 % de su señora; fusión 30 %), firma o propone la paz (pasadas 48 h), se rebela si es vasalla y más fuerte que su señora, designa capital al tener una plaza con Palacio (que no sea ya la efectiva), nombra Embajador si le falta, refunda si no tiene plazas y toma como mucho **una iniciativa** con pesos: guerra (las belicosas: la mitad de las veces contra quien tiene un título, si no la mitad contra una que está en un vasallaje, si no la vecina más cercana), alianza (con una de las 4 vecinas), vasallaje con 0,5 de trigo por minuto, anexión (las ambiciosas, de Facciones sin plazas o de sus vasallas), fusión, romper una alianza, liberar a una vasalla, traición (las traicioneras: sus Capitanes atacan a su aliada con `declara_guerra` durante 3 días, Q2), pasar la corona o cambiar la admisión de otras Facciones en sus ataques.
- **Los Capitanes y Cazadores** solo atacan a quien está en guerra con su Facción (o a la aliada que traicionan); quien conquista se muda con `CambiarResidencia`. **El Gobernador** pide además el Palacio y la Gran Fundición, repone los cargos vacíos con residentes, abre o cierra la puerta a los neutrales, veta y levanta vetos y abre el almacén a las aliadas.
- **Colonos y secesión:** cada 12 h nace un héroe que pide entrar en dos Facciones con plazas y, admitido, se muda a la plaza suya con menos residentes (también los desalojados, que luego salen del campamento y vuelven a casa). Una vez al día, con el 30 %, un Cazador de una Facción grande la deja (`DejarFaccion`) y, pasado el cooldown, crea la suya.
- **Refundación** de una Facción sin plazas: junta el fondo en el campamento de su Rey (`AportarARefundacion` del almacén personal; lo que falta lo compra con su oro, `ComprarEnCampamento`; sin oro, uno de los suyos sale a cazar los bandidos del anillo, Cal-5, y vacía el carro en su almacén), compra la Caravana de Refundación, la engancha a su ejército (`SalirDelCampamento`, `AdjuntarCaravana`), marcha a un sitio y funda; los suyos que residen en campamentos se mudan a la plaza nueva.

**Invariantes** (todos a 0 con las semillas 42 y 7): `inv_ciudadania` (un héroe en una sola Facción, la misma en el héroe y en la Facción), `inv_rey` (Rey y Embajador entre los ciudadanos; toda Facción con ciudadanos tiene Rey), `inv_cargos` (cargos locales de residentes ciudadanos de la dueña), `inv_relaciones` (una activa por par, el índice de la activa, una señora por vasalla), `inv_arrastre` (tras cada `DeclararGuerra`, `RebelionVasallo` y `Atacar` con `declara_guerra`: el hecho existe, el bando entero queda en guerra con el atacante y las arrastradas son las de `guerra.confirmar` antes de la orden), `inv_tributo` (cada pago: pagado ≥ recibido, de un vasallaje con tributo de esa vasalla; lo pagado por vasallaje no pasa de lo pactado por minuto; lo perdido es la diferencia), `inv_reputacion` (sin un suceso ni una relación suya —ni el cruce del 0— la reputación sigue el ritmo anterior, con 10 millonésimas de punto de tolerancia por el redondeo de reanclar; ningún suceso pasa del mayor del bloque; nunca fuera de ±100), `inv_traslado` (en cada anexión o fusión aceptada, en el mismo instante: recursos de plazas, pueblos, almacenes personales, carros, columnas, flota, Caravanas de Fundación y fondos, héroes, escuadras, soldados, plazas y caravanas; los ciudadanos y las plazas de la que queda son la unión, cada ciudadano con su Facción, la que desaparece ya no existe), `inv_ids_faccion` (ninguna desaparecida vuelve; la de una fusión no toma un id visto), `inv_xp_faccion` (la experiencia no baja) e `inv_rechazos` (las órdenes que el bot da por seguras: nacer, cargos, Embajador, capital, paz, denegar solicitudes y rechazar propuestas, comprar la caravana, engancharla), más los 23 del militar, la foto `BAN35` de ida y vuelta a mitad de la corrida y la reproducción.

**Fallos encontrados y arreglados** (del batch, no del núcleo):
1. `inv_ruina` del batch militar contaba como «flota que queda» la caravana **adjunta** a una columna cuya plaza de origen cae en ruinas; Comercio §10 dice que las adjuntas siguen su camino. Ahora no la cuenta (en el militar no había ocurrido nunca).
2. El Intendente aparcaba en la plaza hermana sin mirar si el origen de su caravana seguía siendo de su Facción ni si la hermana se había vuelto su casa (la ruina de la suya le reubica en ella): dos rechazos en `inv_rechazos`. Ahora lo mira.
3. El bot designaba capital la plaza que ya era la efectiva (`capital.ya_es_la_capital`, 106 rechazos en la primera corrida). Arreglado en el bot.
4. El invariante de reputación daba un falso positivo de una millonésima de punto: reanclar el ritmo fuera de un minuto entero (la cita del umbral bajo) redondea. Se tolera y se anota (no es un fallo del núcleo; ver «Dudas»).

**Corridas** (release, 60 plazas × 30 días; el árbol de `12b1f21` contra el de 5I, mismo artefacto de mundo):

| Batch | Antes (`12b1f21`) | 5I | Lo que cambia |
|---|---|---|---|
| `batch politica`, semilla 42 (dos veces) | — | huella 404e0eac… las dos veces; 0 violaciones; 8,0 / 8,3 s de CPU por día de mundo; 1.002.622 eventos y 327.740 órdenes | nuevo |
| `batch politica`, semilla 7 | — | huella 0c37d781…; 0 violaciones; 7,8 s por día | nuevo |
| `batch militar`, semilla 42 | huella 6605fff2…, 93 métricas de juego | **la misma huella y las 93 iguales**; CPU 10,2 → 9,9 s por día | nada de juego |
| `batch comercio`, semilla 42 | 101 métricas de juego | **las 101 iguales**; métrica nueva `multiplicador_distancia` (×1,78, p90 ×2,30) | pared 84,9 → 90,3 s |
| `batch poblacion`, semilla 42 | 86 métricas de juego | **las 86 iguales** | pared 48,9 → 48,6 s |

| Medida (`batch politica`) | Semilla 42 | Semilla 7 |
|---|---|---|
| Facciones creadas / al final / con plazas | 25 / 11 / 11 | 28 / 14 / 12 |
| Ingresos aceptados (colonos nacidos) / mudanzas / residentes por plaza al final | 285 (59) / 310 / 9,1 | 283 (59) / 310 / 6,6 |
| Cargos locales / Reyes (traspasos) / Embajadores | 383 / 28 / 21 | 392 / 39 / 24 |
| Puertas fijadas / vetos / expulsados | 408 / 112 / 4 | 530 / 111 / 5 |
| Alianzas / vasallajes / guerras (arrastradas) | 104 / 22 / 27 (3) | 155 / 23 / 32 (5) |
| Traiciones (planeadas) / paces / alianzas rotas / liberaciones / rebeliones | 1 (13) / 13 / 76 / 7 / 10 | 0 (1) / 21 / 117 / 11 / 10 |
| Tributo pagado / perdido (u) | 32.833 / 0 | 26.599 / 0 |
| Reputación final: mediana / mínima | 0 / −46,5 | 0 / −45,0 |
| Anexiones / fusiones (plazas en Facciones nuevas) | 7 / 7 (45) | 7 / 7 (40) |
| Derrotadas / refundadas (tiempo sin plazas) / secesiones | 0 / 1 (1 día) / 7 | 1 / 3 (1 día) / 13 |
| Títulos: nacen / cambian de manos | 5 / 215 | 11 / 202 |
| Nivel de Facción mediano (máximo), desde 3 | 5 (6) | 4 (5) |
| Capitales designadas / plazas con Palacio / nobles | 9 / 18 / 1.422 | 9 / 19 / 1.501 |
| Plazas vivas / ruinas (lejos de su capital) / multiplicador por distancia mediano (p90) | 40 / 21 (21) / ×8,3 (×17,1) | 54 / 8 (8) / ×2,1 (×15,5) |

**La distancia a la capital (5C-2, P1).** Experimento local (el bloque no cambia: un binario aparte con `asentamiento::mantenimiento::coste` parcheado, revertido), semilla 42:

| Curva | Político: vivas / ruinas | Político: multiplicador mediano (p90) | Comercio: vivas / insolventes | Comercio: multiplicador mediano (p90) |
|---|---|---|---|---|
| Lineal 80 u (hoy) | 40 / 21 | ×8,3 (×17,1) | 141 / 3.319 | ×1,78 (×2,30) |
| Tope ×3 | 55 / 6 | ×3,0 (×3,0) | 141 / 3.319 | ×1,78 (×2,30) |
| **Raíz `1 + √(d/80)`** (recomendada) | 57 / 4 | ×1,95 (×4,5) | 141 / 664 | ×1,88 (×2,15) |
| Lineal 200 u | 57 / 3 | ×1,34 (×5,7) | 145 / 0 | ×1,31 (×1,56) |

**La nobleza (riesgo R1, P5).** Con los héroes residiendo en las plazas aparece en toda plaza con Palacio activo (18, 1.422 nobles). Frente a la misma corrida sin nobleza (`--nobleza 0`): población final 26.732 frente a 25.075 (+6,6 %; sin los nobles, +0,9 %), plazas vivas 40 frente a 39, ruinas 21 en las dos, nivel 2 en 20 en las dos. `batch poblacion` no cambia (sus plazas no tienen residentes).

**El andamiaje de las pruebas antiguas (5E-10).** `tests/comun::sin_andamiaje` (`BRONZEAGE_SIN_ANDAMIAJE=1`) apaga `mandaba` y `como_dueno` para medir quién depende todavía de ellos: sin ellos fallan 179 de 539 pruebas del núcleo en 20 binarios (`pueblos` 27, `avanzadas` 29, `caravanas` 21, `comercio` 16, `batallas` 13, `murallas` 12, `murallas_asedio` 11, `combate` 11, `asentamientos` 10, `entrar` 8, `batalla_unity` 7, `a_mano` 6 y uno en `anexion_fusion`, `cambios`, `columnas`, `diplomacia`, `heroes`, `interior`, `propios` y `urbanismo`). En los **10 binarios que pasan sin él** (`admin`, `ambitos`, `eventos`, `fundacion`, `niebla`, `ordenes_mundo`, `politica`, `reanudacion`, `taberna` y `vista`) se quitó: sus órdenes de plaza pasan ya la matriz real. Lo que sigue siendo andamiaje, documentado en `comun/mod.rs` y en la fila de paridad «Política (5I)»: `mandaba`/`como_dueno` en los 20 binarios de arriba (montan escenarios donde «manda quien es ciudadano»; reescribirlos con cargos de verdad es nombrar Gobernador y Tesorero en cada escenario), la puerta de antes abierta a los neutrales (`comun::acceso`, 42 usos en 7 binarios: sus visitantes son neutrales; con la regla real habría que aliar las Facciones, y eso cambia quién defiende) y los cargos dados por la administración (`gobierna`, `gobernador_de_oficio`, `coloca_avanzadas`).

**Pruebas.** `cargo test --workspace --no-fail-fast`: **96 resultados, 1.450 pruebas en verde, 14 ignoradas** (+2: `tests/politica.rs`, con los invariantes y la reproducción de un escenario corto). Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): **45 resultados, 539 pruebas, 0 fallos**. Las pruebas de sesión no se repitieron 20 veces: la tanda no toca sesión, proyecciones, hechos ni contrato (decisión del autor).

**Dudas para el autor** (las de calibración son P1-P9 de `politica.json`):
1. **La reputación redondea al reanclar**: la cita del umbral bajo reancla el ritmo en un instante que no es un minuto entero y el valor se desvía una millonésima de punto. Sin efecto de juego; ¿se deja?
2. **La caravana adjunta de una plaza arruinada** queda enganchada para siempre (P7).
3. **Los batches militar y de comercio** siguen con `FijarFaccionDeHeroe`/`FijarResidencia` al nacer, para no romper su comparación (P9).

## Calibración del batch político (2026-10-25)

Decisiones del autor sobre P1-P10 de `crates/batch/decisiones/politica.json` (todas cerradas). Sin cambio de foto (`BAN35`): ninguna toca el estado ni el contrato.

- **P1, la distancia a la capital (cambio de cifra).** `escalaDistancia` 80 u → **300 u** (`asentamiento::mantenimiento`; primero como valor vigente sobre el bloque de TypeScript, que decía 400, y desde el 2026-10-26 en el bloque, que el código lee); la fórmula lineal no cambia: ×2 a 300 u, ×3 a 600 u, ×4,3 a 1.000 u. Un reino sostiene media región alrededor de su capital y sigue limitado más allá. Mechanics (`Economia_Construccion.md` §10.1 y §15) y canon (Doc 1.0a y 4.5). Pruebas: `mantenimiento::pruebas::la_escala_de_distancia_vigente_es_300_media_region` y `el_coste_escala_con_edificios_y_distancia...` (×2 a 300 u, ×1,2667 a 80 u); `tests/politica.rs::la_distancia_a_la_capital_sube_el_mantenimiento_de_la_plaza_lejana` reescrita: la plaza a 1.345 u paga más madera que sin el factor y sigue en pie (con 80 u caía en ruinas en un día).
- **P2, los títulos (cambio).** `TITULOS.margenAspirante` = 0,05 (bloque de `Diplomacia_Facciones.md` §7; canon 2.9): `politica::titulos::calcular` recibe los títulos guardados y el margen; la poseedora conserva un título si la mejor no la supera en un 5 % (en la medida principal; en la Facción más grande, a igual número de plazas, en la población). El Gran Rey no lleva margen. Sin estado nuevo. Prueba: `titulos::pruebas::la_aspirante_tiene_que_superar_a_la_poseedora_en_un_cinco_por_ciento` y la cifra en `datos::pruebas`.
- **P3** (Gran Fundición inalcanzable en 30 días): se deja; fase 7, batches de más semanas.
- **P4, el cupo de nivel 2 (cambio de cifra).** `CUPO_NIVEL_ASENTAMIENTO.maxNivel2` = [1,1,2,2,3,3,3,4,4,5] (dos tercios de `CAP_FUNDACION_POR_NIVEL`); `maxNivel3` igual. Mechanics (Dip §3.5) y canon (Doc 2.2.1). Prueba: `politica::datos::pruebas::las_cifras_son_las_del_documento`.
- **P5** (nobleza con residentes reales): se acepta.
- **P6** (reputación que no sube): fase 7, con EE-7 y C1b.
- **P7, la adjunta de una plaza arruinada (cambio).** `comercio::ruina::cerrar_dependientes` da `Pierde` a la adjunta cuyo origen cae en ruinas (en la conquista sigue con su columna); el núcleo (`Nucleo::perder_caravana_de_ruina`) la saca de la columna con `perder_caravana` —la carga sigue a los carros, M10: lo que la columna ya no puede llevar sin ellos se pierde— y la columna lo oye con `ejercito.caravanas_perdidas` `{ caravanas: 1, carros, carga, ruina }` (código que ya existía; `ruina` es dato nuevo). El invariante de ruina de `batch militar`/`politica` vuelve a contar las adjuntas. Comercio §10, Movimiento §9, Doc 3.13. Pruebas: `ruina::pruebas::las_de_fundacion_siguen_su_propio_camino_y_la_adjunta_solo_cae_con_la_ruina_de_su_origen` y `tests/caravanas.rs::la_caravana_adjunta_cuya_plaza_de_origen_cae_en_ruinas_se_pierde_con_ella_y_con_lo_que_la_columna_llevaba_en_sus_carros` (falla sin el cambio).
- **P8** (refundación en solitario lenta): bots, fase 7. **P9** (batches militar y comercio): siguen con el andamiaje. **P10** (millonésima de la reputación): se deja. Las de fase 7, en la fila «Batch (5I, calibración)» de `Mecanicas a desarrollar.md`.

**Medición** (release, semilla 42, 30 días; antes = `e06d4e7`, después = las cuatro a la vez):

| Medida (semilla 42, 30 días) | Antes (80 u, sin margen, cupo viejo) | Después |
|---|---|---|
| Político: plazas vivas / ruinas (lejos de su capital) | 40 / 21 (21) | **54 / 6 (6)**, distancia mediana de las lejanas 1.048 → 1.653 u |
| Político: multiplicador por distancia mediano (p90, máximo) | ×8,3 (×17,1, ×24,2) | **×1,47** (×7,1, ×7,9) |
| Político: ascensos insolventes / sin cupo de Facción | 3 / 5.134 | 1 / **1.631** |
| Político: plazas de nivel 2 | 20 | **48** |
| Político: títulos que cambian de manos (Ejército / Facción más grande / Poder económico / Gran Rey) | 215 (131 / 48 / 22 / 14) | **42** (11 / 24 / 6 / 1) |
| Político: caravanas perdidas | 119 | 92 |
| Comercio: plazas vivas / ascensos insolventes | 141 / 3.319 | **146 / 0** |
| Comercio: multiplicador mediano (p90) | ×1,78 (×2,30) | ×1,21 (×1,37) |
| Comercio: plazas de nivel 2 al final | 109 | 142 |
| Comercio: caravanas perdidas | 5 | 0 |
| Población | — | sin cambios (mismas métricas; solo el tiempo) |
| Violaciones e `inv_rechazos` (los tres batches) | 0 | 0 |

**Pruebas.** `cargo test --workspace --no-fail-fast`: **96 resultados, 1.452 pruebas en verde, 14 ignoradas** (+2: el margen de los títulos y la adjunta de la plaza arruinada). Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): **45 resultados, 540 pruebas, 0 fallos**. Una pasada: la tanda no toca sesión ni contrato.

**Dudas de la calibración:**
1. Con el cupo nuevo, `batch comercio` sube a 142 plazas de nivel 2 y sus zonas crecen: los **peajes** pasan del 0,7 % al **22,1 %** de la carga que sale (rojo), las salidas por trueque de 2,70 a 3,23 (rojo) y el trueque en el percentil 90 de 12,8 a 18,0 min. ¿Se mira el peaje con zonas de nivel 2 (Comercio §6) o se deja para la fase 7?
2. `escalaDistancia` sigue siendo un valor vigente en el código sobre el bloque de TypeScript (400), como antes con 80: ¿se pasa al bloque? **Sí** (2026-10-26): el bloque dice 300 y el código lo lee.

## Estado de cierre del sistema 5 (2026-10-25)

Hecho: las reglas puras (5A), la Facción, la ciudadanía y la residencia (5B), la experiencia, el nivel, la capital, la derrota y el nombre (5C), las relaciones (5D), los cargos, la matriz de autorización y la puerta (5E), la anexión, la fusión, el traslado, la Liga y los títulos (5F), la refundación, el almacén personal y las caravanas a mano (5G), el contrato y las proyecciones (5H, foto `BAN35`) y el batch político (5I). Las 28 órdenes de política y las 10 de 5G están en el contrato; solo quedan `orden.no_disponible` las cinco del sistema 6. **Decididas y aplicadas** las de calibración P1-P10 del batch político (sección anterior). **Pendiente del autor:** las dudas de esa calibración, las abiertas de 5H (forma pública de la Facción ajena, coste por orden con 500 vistas) y de 5I. **Pendiente de otros sistemas:** los Aedas residentes, las épicas y la tecnología de la fusión y el título «Mecenas de los Aedas» (sistema 6); que la columna se deshaga dentro al fundar (fila de 5G-b); el andamiaje de las pruebas antiguas (arriba). Siguiente: el sistema 6.

**Al cerrar la fase 6 (2026-10-26).** De la calibración, los peajes quedan para la fase 7 (decisión del autor del 2026-10-25) y `escalaDistancia` quedó en el bloque (300, 2026-10-26); las dudas de 5I las cerraron P7, P9 y P10; las nueve de 5H siguen sin decisión. Lo del sistema 6 que colgaba de aquí está hecho (6B, 6E). Todas las decisiones, en `Consideraciones/Politica_Tecnologia_Decisiones.md`; lo abierto, en `Docs/Mecanicas a desarrollar.md`.
