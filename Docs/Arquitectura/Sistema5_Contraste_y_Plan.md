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
