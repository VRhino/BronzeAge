# Contraste y plan del sistema 4 (militar y combate), fase 6

Preparado el 2026-10-19, solo lectura. Rust leído en el árbol `lucid-curran-6d6ffa` mientras otro agente edita `nucleo`/`comercio`/`asentamiento`
(la foto ya pasó de BAN7 a BAN8 durante la lectura: el sistema 4 será BAN9). El TypeScript se leyó en
`.claude/worktrees/quirky-villani-88c432/src` (en `main` ya no hay `src/`; es el de la rama `typescript-final`, algo anterior al último TS: p. ej.
`MILITAR.defensaBaseCaravana` 15 allí y 26 en el bloque de Mechanics).

Rutas: canon = `Docs/Game/5_Sistema_Militar_y_Combate.md` (C5); Mechanics = `Docs/Mechanics/`; Paridad = `Docs/Mecanicas a desarrollar.md`.

---------------------------------------------------------------------------------------------------------------------------------------

## 0. Corrección a la premisa de las cifras

El encargo dice que los bloques `<!-- parametros:NOMBRE -->` son las cifras vigentes y sobrescriben el TypeScript. **Los documentos dicen lo contrario**:
los bloques **copian `constants.ts`** (TypeScript, sin calibrar) y lo vigente de la reconstrucción vive en la **prosa/tablas** de cada documento
(`Mechanics/README.md:3`; `Movimiento_Ejercitos.md:213-227`; `Niebla_De_Guerra.md:207-218`; `Combate.md:203`; `Heroe_Y_Tropa.md:794`). Los crates ya
construidos lo asumen: `comercio/src/datos.rs:6-9` carga los bloques y aplica encima los valores calibrados (120 en vez de 600, velocidades ×1,5…) con un
test que los fija. Para el sistema 4 hay **muchas** cifras en ese caso (ver C3): hay que decidir cómo se trata (pregunta Q1).

---------------------------------------------------------------------------------------------------------------------------------------

## 1. Inventario de lo que ya existe en Rust

### 1.1 Tipos y estado (`crates/nucleo/src/estado.rs`)

- `Ejercito` (l.147-174): `id, lider: String, propietario: String (cuenta), faccion: u32, velocidad (parámetro), posicion, plan, viveres: Ritmo,
  capacidad_viveres, caravana_fundacion: bool, suministro (el carro), caravanas_adjuntas: Vec<u64>, version`. Es una columna "genérica".
- `Reglas`: `consumo_marcha` 830 y `consumo_estacionado` 83 (constantes, no dependen de la tropa), `viveres_bajos_pct`, `limite_idempotencia`.
- `Carga` (eventos de cola): `Llegada, Umbral(víveres), Minuto, Externa{generador,tipo,a,b}, Comercio{..}, Reputacion(..)`. Prioridades: `PRIO_UMBRAL=3`,
  `PRIO_LLEGADA=8`, `PRIO_COMERCIO=6`, `PRIO_REPUTACION=9`, `PRIO_ASENTAMIENTO=2`. Falta la 1 (batallas vencidas / salidas del mundo) y la 7 (bandidos).
- Etiquetas de entidad de la cola en `estado_comercio.rs:14-55` (trueque, orden, caravana, reputación, mercenarios): patrón para añadir batalla,
  convocatoria, bandido, formación, desconexión.
- Andamiaje 3B "la columna hace de héroe" (`estado_comercio.rs:60-80`): `residencias`, `personal` (almacén personal), `cupos` por columna.
- No existen: Heroe, Escuadra, Loadout, Facción/relaciones, Batalla, Convocatoria, Formación, persecución, bandidos, avanzadas, informes de inspección.

### 1.2 Comandos (`comando.rs`)

Numeración usada **1-48** (siguiente libre: 49; el otro agente puede ocupar más): 1 `CrearEjercito` (admin), 2 `MarcharA` (solo punto), 3 `Estacionar`,
4 `Reloj`, 5-11 territorio/fundación (`FijarNivelFaccion`, `DarCaravanaFundacion`, `Fundar`…), 12-20 economía, 21-36 comercio, 37 `DarRecursos`,
38 `DarSuministro`, 39 `AdjuntarCaravana`, 40 `AparcarCaravana`, 41-44 campamento de mercenarios (`ComprarEnCampamento`, `RepostarEnCampamento`,
`FijarResidencia`, `DarAlmacenPersonal`), 45-48 pueblos. Rechazos estables en `rechazos::*`. `sesiones::traducir` solo mapea `MarcharA` (a punto),
`EstacionarEjercito` y `ControlarReloj`: **ninguna de las ~45 órdenes militares del contrato está mapeada** (el `.proto` ya las trae todas).

### 1.3 Ganchos ya preparados para el sistema militar

| Gancho | Dónde | Estado |
|---|---|---|
| `caravana_entra_en_batalla` / `caravana_sale_de_batalla` | `nucleo/nucleo/viaje.rs:556-600` | `pub`; pausa el plan, desplaza al salir; hecho `comercio.caravana_en_batalla` |
| `aplicar_perdida_de_carros` | `viaje.rs:606` | `pub`; reparto de carga proporcional a la capacidad del carro (criterio del implementador, por confirmar) |
| `Caravana::perder_fraccion_de_carga` (tipo B) | `comercio/caravana.rs:846` | **no expuesto en `Nucleo`** |
| Escolta cedida: `Escuadra{id,heroe,liderazgo}`, `CandidataEscolta`, `seleccionar_escolta`, `asignar_escolta`, `liberar_escolta` | `comercio/caravana.rs:60-280, 449-470, 862` | solo estructuras; **sin comandos `AsignarEscolta`/`QuitarEscolta`** ni escuadras reales |
| `cerrar_comercio_por_conquista` | `nucleo/nucleo/flota.rs:435` | `pub`; flota pasa al conquistador, trueques se anulan, las que iban hacia ella regresan |
| `GeneradorEncuentros` (rejilla) | `nucleo/espacial.rs` | solo **emite** `ejercito.encuentro` y `caravana.encuentro`; no ofrece nada ni crea batalla |
| `encuentros::{evento_de_persecucion, eventos_de_plan, Puerta, llegada_a_radio, salida_de_radio, interceptar}` | `encuentros/eventos.rs:54-110`, `lib.rs:198-256` | **sin usar por el núcleo**: persecución y llegada a puerta no están conectadas |
| `GeneradorVision` | `espacial.rs:~105-181` | observadores = columnas (radio fijo `EJERCITO` 30 para todas, no distingue con/sin soldados) + caravanas; emite solo `vision.contacto`; **plazas y avanzadas no observan** (Paridad l.95) |
| `Asentamiento.edificio.danado`, `obras` (reconstrucción barata) | `asentamiento/edificio.rs:41`, `obras.rs:11` | modelo hecho, sin quien lo dispare (Paridad l.78) |
| `Asentamiento.ocupado: bool` | `asentamiento/asentamiento.rs:~165` | sin `protegida_hasta`/`ocupacion_hasta` ni eventos de fin (Reloj §3.1) |
| `racion_guarnicion` (entrada de la economía) | `asentamiento.rs:102`, `nucleo/construccion.rs:50` | constante 0 |
| `pueblos`: datos de guarnición de serie (30 mineros, 6 h) | `asentamiento/pueblos.rs:38-52` | solo dato; sin tropa, sin saqueo (Paridad l.74) |
| `reescalar_marcha`, `velocidad_de_columna` (mínimo con caravanas), `adjuntar_caravana`, `aparcar_caravana` | `viaje.rs:134, 626, 650, 691` | la velocidad base sigue siendo el parámetro `Ejercito.velocidad`; adjuntar no distingue personal/ejército |
| `ComandoSesion::Batalla` (canal del servidor de batalla) | `servidor/nucleo.rs:72`, `sesiones.rs:137` | llega al actor y **se descarta** (`=> {}`) |
| `Respuesta.creadas` → `ReferenciaCreada` | `sesiones.rs:~353` | tipo de entidad fijo en `Ejercito`: no sirve para héroe/batalla |
| Contrato Protobuf completo: `Heroe`, `Escuadron`, `Ejercito`, `Persecucion`, `Convocatoria`, `Pueblo`, `Avanzada`, `Batalla`, `BattleTicket`…, `OrdenAtacar` con `declara_guerra` y objetivos pueblo/avanzada | `proto/bronzeage/v1/` | ya definido; `EstadoCaravana` no tiene EN_BATALLA (coherente con "la batalla las sustituye") |

### 1.4 Qué le falta a `Ejercito` (y al núcleo) para ser la columna del canon

Tipo (`personal`/`ejercito`, fijo al nacer); `participantes[{heroe, unido_en}]`; Líder como héroe (hoy `lider` es un id de texto y `propietario` una cuenta);
política de unión y peticiones con caducidad de 10 s (evento); `escuadron_ids`; `formacion_expira_en` (evento de 10 min); `persiguiendo`; marca de
bloqueo por batalla; víveres compartidos con capacidad = suma de héroes (350 c/u) y ritmo de consumo **calculado** de la composición
(`Σ soldados·0,15·factor + participantes·0,5·factor`, factor ¼ en marcha y ×0,1 acampada); moral/deserción; velocidad **derivada** (mínimo de escuadras
con tropa y de caravanas; 33 el héroe solo); capacidad del carro = participantes × 500 + caravanas; `faccion` opcional (columna sin bandera →
`null`); `desconectado`/`fuera`. Y, fuera del `Ejercito`: la entidad `Heroe` (residencia, ubicación, herido, loadouts, almacén personal, víveres
personales, XP), `Escuadra` (tropa, cantidad, nivel, xp, moral como `Ritmo`, contenedor, guarnición, candado), relaciones entre Facciones,
`Batalla`, convocatorias, bandidos, avanzadas.

### 1.5 Huecos técnicos detectados al leer (no están en Mechanics)

1. **La rejilla no contiene columnas estacionadas ni entidades fijas.** `plan_retirado` hace `rejilla.quitar` (`espacial.rs:56`) y `CrearEjercito` no apunta
   nada: un ejército acampado en un paso (canon l.341) **no genera encuentros**, tampoco los campamentos de bandidos (acoso a caravanas, radio 7) ni las
   plazas. La `Rejilla` ya tiene el índice `fijos` para tramos quietos largos; hay que usarlo.
2. **Los encuentros no se filtran**: hoy se emite `ejercito.encuentro` entre cualquier par (misma Facción, aliados, personal contra ejército,
   héroes heridos, columnas en batalla).
3. **Repostar automático y tipo A/B de caravana** necesitan geometría contra el **polígono** de la zona (entrada/salida exactas de un tramo en un polígono;
   hoy solo hay discos). Es el único algoritmo nuevo de cierta dificultad.
4. `perseguir` necesita re-planificar al perseguidor cada vez que la presa cambia de plan: `evento_de_persecucion` lo calcula, falta el bucle de
   generaciones en el núcleo.
5. La ocupación/protección tras conquista y la protección de un día al fundar (Paridad l.70) no tienen campo con instante.
6. `ámbito HEROE:<cliente>` y `CambioVersion` solo llevan ejércitos; héroes, escuadras, caravanas, convocatorias y batallas necesitan su lista.

---------------------------------------------------------------------------------------------------------------------------------------

## 2. Contraste canon / Mechanics / TypeScript

Leyenda de resolución: **R** = se resuelve por la regla de precedencia (gana Mechanics; luego recencia) · **P** = pregunta al autor (Q#) ·
**I** = corrección de documento o decisión de implementación sin impacto de diseño.

### 2.1 Contradicciones y cifras distintas

| # | Qué | Dónde | Resolución |
|---|---|---|---|
| C1 | Los bloques no son lo vigente (ver §0) | `README.md:3`; `Movimiento_Ejercitos.md:213`; `Combate.md:203` | P (Q1) |
| C2 | Tope de héroes del asedio: 15 en canon (C5 l.726), `Combate.md:43,203`, `Batalla_Unity_Ciclo.md:382`, `Heroe_Y_Tropa.md:794`, Paridad l.54 (§38); **5** en el bloque `BATALLA.capacidad` (`Batalla_Unity_Ciclo.md:294`) y en el TS. `BATALLA.capacidad` solo tiene `asedio`/`resto`: no dice qué usan la avanzada (canon l.726: 15) ni el saqueo de pueblo (5) | R: 15; completar tabla §4 de `Batalla_Unity_Ciclo` con contextos `saqueo_pueblo` y `avanzada` (el `.proto` ya los tiene, `battle.proto:50-70`) |
| C3 | Cifras calibradas frente al bloque TS: velocidades de tropa 30/24/18/42 (C5 l.367-372; `Movimiento_Ejercitos.md:215-217`) contra 20/16/12/28 en el bloque `TROPAS_RECLUTABLES` (`Heroe_Y_Tropa.md:355-743`); `velocidadJugador` 33/22; `radioPuerta` 2/10; `radioInspeccion` 8/40; `radioEncuentro` 3/15 (`Movimiento_Ejercitos.md:253`, `Combate.md:85`); `umbralRecalculo` 6/30; `VISION` 30/15/0/60 contra 150/80/60/300 (`Niebla_De_Guerra.md:207-218`) | R: vigente = prosa; cómo reflejarlo, Q1 |
| C3b | Los **Mineros** (C5 l.113; `Heroe_Y_Tropa.md:793`) no están en el bloque `TROPAS_RECLUTABLES` (26 tropas; el canon tiene 27). `Guerra_Logistica_Definicion.md` (Consideraciones, no vinculante) les da velocidad 20, acarreo 16 y visión de avanzada 60 (l.228); el canon manda 30, 24 y 12 | I |
| C4 | Defensa base de la caravana: 26 (`Combate.md:198`; C3 §3.10) contra 15 en el TS | R: 26 (= 13 carreteros × poder 2) |
| C5 | Captura de caravana: `Batalla_Unity_Ciclo.md:268` aún aplica `umbralCapturaCaravana` (50 % de la carga al atacante); `Combate.md:124`, `Batalla_Unity_Ciclo.md:379` y C3 l.89-90 lo sustituyen por A (carros) y B (70/30). El bloque `MILITAR` conserva el umbral "solo de referencia" (`Combate.md:197`) | R: A/B; corregir l.268 y quitar el umbral del bloque |
| C6 | **Poder del héroe** en el combate numérico: C5 l.6-20 (15 por héroe; "pendiente, no implementada") frente a `Combate.md` §3.3/§4, que solo suma escuadras y lanza `CombateInvalidoError` con un bando vacío (l.73). La defensa base 26 > 15 (C3 §3.10 l.75) depende de esa cifra | P (Q2) |
| C7 | **Caravana suelta**: C5 l.570 y `Movimiento_Ejercitos.md:141` la dejan "`disponible` suelta donde esté la columna"; C5 l.287 ofrece "Interceptar" una caravana suelta; pero `Movimiento_Ejercitos.md:125` excluye `disponible` de lo alcanzable y la máquina de estados de Rust trata `disponible` como "en su ciudad, no interceptable" (`caravana.rs:1482`). El TS la deja `disponible` con `posicionActual` y no la persigue (`ejercitos.ts:762, 1880`) | P (Q3) |
| C8 | La presa "sale del alcance de vista" se evalúa **en el muestreo de visión** (`Movimiento_Ejercitos.md:122`) mientras `Reloj_Y_Eventos.md:35,77` dice que lo que cambia el juego es evento exacto y solo lo informativo se muestrea. Además `umbralRecalculo` (C5 l.308; `Movimiento_Ejercitos.md:123`) es un resto del tick: con el plan exacto de la presa el recálculo es un evento (`Movimiento_Ejercitos.md:120`) | P (Q5) |
| C9 | Caravana aparcada en una plaza **conquistada**: `Combate.md:154` y C3 l.262 dicen que se destruye; lo implementado en 3B-2 (decisión del implementador, DUDA en `comercio/ruina.rs:78`) la manda a su origen, y la nota `Comercio_Caravanas.md:37` lo describe así | P (Q8) |
| C10 | Equipo de asedio: Paridad l.27 lo da por descartado ("armas de asedio como mecánica aparte", 2026-10-06) y l.45/48 dicen que "entra con el sistema militar"; C5 l.85 todavía lista ariete y torre de asedio como producto de Carpintería | P (Q7) |
| C11 | Murallas: el asedio usa el **último** recinto (`Combate.md:101,242`) y el cupo de guarnición el **mayor nivel** con integridad ≥ 1 (`Heroe_Y_Tropa.md:162`): es intencional, pero `Murallas.md` no tiene sistema asignado y Rust no tiene recintos (Paridad l.81) | P (Q4) |
| C12 | Convocatoria del asedio de 30 s "en la práctica hasta el siguiente tick" (`Batalla_Unity_Ciclo.md:111`) | I: sin tick es un evento exacto a +30 s (la regla no cambia) |
| C13 | `Proyecciones_Y_Autorizacion.md:271-275` aún cita `replegarEjercito` (retirado: `Movimiento_Ejercitos.md:209`, `Comandos_Parametros.md:143`) y l.194 marca `Batalla_Unity_Ciclo` como "pendiente" aunque existe | I |
| C14 | `Heroe_Y_Tropa.md` tiene dos secciones "## 12" (l.269 y l.788); C5 l.104 enumera las tropas sin oro del Centro Urbano sin los Mineros (l.113 sí los exime) | I |
| C15 | `Combate.md` mezcla tiradas de `rng`: 2 en `resolverCombate`, 1 en caravana y campamento, 0 sin combate; el saqueo de pueblo y la avanzada "usan `resolverCombate`" (l.134) pero no dicen cuántas tiradas ni con qué ventaja del defensor (B ×1,25 vale para caravana; en saqueo se cita "ventaja del defensor de B") | I: fijar por escrito al hacer 4A (afecta a la reproducción) |

### 2.2 Huecos: reglas del canon sin equivalente en Mechanics (o al revés)

| # | Hueco | Dónde |
|---|---|---|
| H1 | "Los aliados no se cruzan" y "un ejército enemigo manda sobre una caravana" (con las dos al alcance se combate la columna) | C5 l.333-334; Mechanics solo valida al atacar (`Combate.md:85`) |
| H2 | "Solo se persigue lo que se ve desde la propia columna" es precondición en C5 l.308, pero en `Movimiento_Ejercitos.md:118` solo aparece como causa de abandono (l.122) | I: exigirlo al ordenar |
| H3 | Guerra: "toda agresión exige guerra o la declara" necesita relaciones entre Facciones (guerra, alianza, vasallaje, arrastre) que son del sistema 5 | `Combate.md:33`; C5 l.752 |
| H4 | Héroe sin Facción: `faccionId ''` ↔ `null` en el ticket (`Batalla_Unity_Ciclo.md:81`); en Rust `Ejercito.faccion` es `u32` obligatorio | I: `Option<u32>` |
| H5 | `NIVEL_FACCION` (xp de combate 2,5, conquista 10, `ratioCombateDigno`) no está en ningún bloque de Mechanics (`Combate.md:223-226` vacío; Paridad l.98) | sistema 5 |
| H6 | Entrada/salida exacta de **zona** (polígono) para repostar y para el tipo A/B: Mechanics lo describe (`Movimiento_Ejercitos.md:129`, `Combate.md:116`) sin algoritmo | I: 4B-2 |
| H7 | Moral y deserción: la regla es por minuto con `ceil`; en tiempo continuo hay que fijar cómo se cuantiza (eventos recurrentes de un minuto mientras moral = 0 y víveres = 0, que conserva el resultado del TS) | `Movimiento_Ejercitos.md:113` |

### 2.3 Reglas que dependen de sistemas todavía no hechos

- **Sistema 5, política y Facciones**: ciudadanía y residencia completas (`cambiarResidencia`, casas compradas, cooldown, `sinGuarnicion` al mudarse),
  cargos (Gobernador, Tesorero, General, Maestro de Obras: autorización de políticas, `alternarReabastecerAliados`, tavernas), relaciones y
  `declararGuerra` con arrastre por vasallaje, `admitirOtrasFacciones` (Rey admite atacantes neutrales), reputación (`penalizacionAtacarAliado`,
  `ajustesPorDefensaDeVasallo`), XP de Facción por combate/conquista/bandidos, `registrarDerrota` y reubicación de héroes al campamento más cercano,
  traslado de héroes y escuadras en anexión/fusión, puertas (`fijarPuerta`, `vetarJugador`, exilio), capital (distancia).
- **Sistema 6, tecnología y Aedas**: la puerta de tecnología de cada tropa (`TROPAS_RECLUTABLES.tecnologia`), contadores de logros del servidor
  (`reclutados.*`, `batallas.*`, `asedios.*`, `conquistas.*`, `caravanas.destruidasOCapturadas`, `bandidos.campamentosDestruidos`), tecnologías del
  campamento de mercenarios (`tecnologiasDelCampamento`), épicas de Aedas que leen eventos `batalla.*`, equipo de asedio. Las Eras IV-V están pospuestas.
- **Niebla/proyecciones (transversal)**: plazas y avanzadas como observadores, memoria, Intel de la taberna (Miradas e Informes: canon 5.12.10),
  informes de inspección de 10 min, proyección pública del héroe, `NieblaProyectada` de ejércitos ajenos. Paridad l.95.
- **Murallas** (`Murallas.md`): recintos, multiplicador de defensa y cupo de guarnición.

---------------------------------------------------------------------------------------------------------------------------------------

## 3. Descomposición en fases

Patrón del sistema 3 (3A reglas puras → 3B-1 estático en el núcleo → 3B-2 viaje → 3C pueblos). Tamaños en líneas de Rust con pruebas
(referencia: el sistema 3 fue ≈ 6.000 en `comercio` + ≈ 3.200 en `nucleo` + ≈ 1.200 de codec + ≈ 4.000 de pruebas + ≈ 1.400 de batch; el TS del
sistema 4 son ≈ 8.100 líneas, sin avanzadas, pueblos ni guerra logística). Estimación total del sistema 4: **25-30.000 líneas, 8-9 commits**.

Crates nuevos: **`crates/militar`** (héroe, tropa, liderazgo, reclutamiento, columna) y **`crates/combate`** (resolución, consecuencias, ciclo de
batalla como máquina de estados pura). Ambos sin dependencia del núcleo, como `comercio`; sus datos salen de los bloques de Mechanics en un
`datos.rs` (patrón de `comercio/datos.rs`).

| Fase | Contenido | Tamaño | Depende de |
|---|---|---|---|
| **4A-1 `militar`** (reglas puras) | Datos: `TROPAS_RECLUTABLES` (27 con Mineros), `LIDERAZGO`, `GUARNICION`, `LOGISTICA`, `MOVIMIENTO`, `HEROE`, `MILITAR`, oro por escalón. Modelos `Heroe`, `Escuadra`, `Loadout`. Liderazgo (`puede_llevar`), cupo de guarnición, reclutar (permisos, pool de población por escalón, coste con "Leva Forzosa", reserva de trigo), reponer, préstamo y reclutas de campamento, orden de escuadras, herido (`herir`, al leer). Columna: velocidad mínima, ritmo de ración, moral/deserción, víveres (reparto al unir/separar/desarmar), `desgajar` a prorrata, sucesión del Líder, política de unión, formación en campo (3 héroes, 10 min), convocatoria (modelo y partir todo o nada), reglas de persecución | L, ≈ 4.000 | — |
| **4A-2 `combate`** (reglas puras) | Poder de escuadra, `resolver_combate` (2 tiradas del `Rng` determinista, orden A-D), bajas permanentes y XP de escuadra, chatarra, reparto de botín por rondas, caravana A/B (1 tirada, 50 % de carros, 70/30), campamento de bandidos, saqueo de pueblo (50 % protegido), avanzada (etapas, ruinas), `aplicar_conquista` (saqueo determinista sobre `asentamiento::Asentamiento`), desalojo y derrota como planes puros, tipo A/B por posición. Ciclo de batalla Unity como máquina de estados: `Batalla`, estados, plazos, candado, `lado_para_unirse`, checklist de `validar` del resultado, idempotencia, retención | L, ≈ 3.500 | 4A-1 |
| **4B-1 núcleo: héroes y tropa** (estático) | Entidades `Heroe`/`Escuadra` en `Estado` y codec (BAN9); sustituye el andamiaje 3B (`residencias`, `personal`, `cupos`, `DarSuministro`). Comandos: `CrearHeroe` (nace en un campamento), `ReclutarTropa` en plaza, `ReclutarEnCampamento`, `PedirPrestamo`/`ReponerPrestamo`, loadouts, `ordenarEscuadras`, `asignar/retirarGuarnicion` (la ración entra en `racion_guarnicion` de la plaza y el hambre de la guarnición es un ritmo con umbrales; "la población come antes"), `RepartirPuntos`, `pasarAViveres`, `AsignarEscolta`/`QuitarEscolta`, presencia de plazas y campamentos (`entrar/salir`, `conectarse/desconectarse` como evento de +2 min 30 s). Andamiaje de admin: `FijarTecnologias` (hasta el sistema 6), `FijarRelacion` (hasta el 5). Residencia mínima (una residencia por héroe) | XL, ≈ 5.500 con pruebas | 4A-1 |
| **4B-2 núcleo: columnas** | `Ejercito` completo (tipo, participantes, Líder, política); `salirAlMundo`, `movilizarEjercito`, `salirDelCampamento`, `marcharA` a asentamiento, `guarnecer`/`entrarEnCampamento` (desarmar), `unirseAEjercito`, `unirseEnCampo` y `responderPeticion` (evento de 10 s), `separarse`, `cederLiderazgo`, `organizar/cancelarFormacion`, convocatoria completa, `repostar` (puerta) y repostar automático por zona (eventos de entrada/salida de polígono), víveres/moral/deserción por eventos, velocidad real, `soltar/reclamarCaravana`, `cargarCaravana`, `caravanasAlEntrar`, disolución sin nadie, desconexión con aplazo | XL, ≈ 6.000 | 4B-1 |
| **4C núcleo: encuentros, persecución, inspección** | Rejilla con columnas estacionadas y entidades fijas (plazas, campamentos, bandidos); filtro de encuentros (aliados, misma Facción, clases, heridos, en batalla); `perseguir`/`dejarDePerseguir` con `evento_de_persecucion` (re-plan al cambiar la presa, `presa_alcanzada/perdida/a_cubierto`); llegada a puerta; `inspeccionar` (anillo de 8, aviso al observado, informe de 10 min); radio de visión por soldados | M-L, ≈ 3.000 | 4B-2 |
| **4D núcleo: combate con números y consecuencias** | `Atacar` (6 objetivos, guerra o `declara_guerra`, protección de 10 junto a campamentos), resolver y aplicar (heridos, mitad del carro, botín, chatarra, XP de escuadra), asedio con muralla ×1 (hasta Murallas), conquista (`protegida_hasta`/`ocupacion_hasta` con eventos y ritmos ×0,5, `cerrar_comercio_por_conquista`, desalojo, ruina de la madre), protección de un día al fundar, bandidos (aparición, respawn, acoso a caravanas a 7 u., prioridad 7), saqueo de pueblo con guarnición de 30 mineros y reposición de 6 h, hooks de caravana (entra/sale/pérdida/fracción B) | XL, ≈ 5.500 | 4C |
| **4E núcleo: ciclo de la batalla de Unity** | `Batalla` en el estado y codec; apertura (campo, caravana, bandidos, asedio, saqueo, avanzada) con tickets congelados desde `urbanismo`/`planta_de_pueblo`; candado `reserva_batalla` y bloqueos; convocatoria de 30 s y plazos como eventos de prioridad 1; unirse y bando; los 5 mensajes de `ComandoSesion::Batalla` (asignación, inicio, tokens, salida, resultado), checklist e idempotencia; aplicar resultado reutilizando 4D; retención de 60 min; eventos `batalla.*`/`evento_pve.*` | L, ≈ 4.000 | 4D |
| **4F avanzadas** | Obras de mapa para avanzada (colocar, entregar, mejorar, reparar, destruir) sobre el mecanismo de pueblos; visión de 12; almacén y repostar; guarnición de serie (elegir tropa) y de jugadores; ataque; ruinas, invulnerabilidad de un día, conquista y ruina de la madre | M, ≈ 3.000 | 4D, 4E |
| **4G batch militar** | `batch militar` (como `batch comercio`): héroes mínimos que reclutan, salen, comen, persiguen y combaten con números; invariantes (bajas, hambre, carros, herido, reproducción del azar); informe | M, ≈ 2.000 | 4D (idealmente 4F) |
| 4H contrato (diferible) | Mapear las ~45 órdenes militares a `Comando`, proyección de héroe/escuadras/ejércitos/batallas y niebla de ejércitos ajenos. Como en el sistema 3 (Paridad l.88), puede quedar en la fase transversal de contrato y proyecciones | L, ≈ 3.500 | 4E |

Notas de orden: 4A-1 y 4A-2 pueden ir en commits separados pero seguidos (como 3A). 4B-1 va antes que 4B-2 porque las columnas necesitan al héroe como
dueño de la tropa. Los bandidos se incluyen en 4D porque son el otro lado de los combates de caravana y no están en ninguna otra fase.

### 3.1 Qué se queda fuera porque lo resuelve Unity (`Batalla_Unity_Ciclo.md`, C5 l.740)

Todo lo de dentro de la partida: banderas, reaparición, rendición, cómo se gana, daño y bajas por unidad, XP del héroe y de las escuadras, botín (objetos y
monedas), qué carros se pierden y cómo cambian de manos en la modalidad A, el mapa interior de cada etapa de avanzada. BronzeAge solo: decidir que la
batalla ocurra y dónde, congelar el ticket, custodiar tokens (sin verificarlos), bloquear lo que combate, validar el resultado (checklist entero o nada),
aplicar consecuencias (heridos, conquista, carro, chatarra, XP de Facción) y contarlo. El menú "la geometría ofrece" es del cliente: el servidor emite
el encuentro y valida la orden.

### 3.2 Qué cae en otros sistemas

- **Sistema 5**: ver §2.3 (relaciones, cargos, residencia completa, reputación, XP de Facción, derrota, traslado, puertas). El sistema 4 deja stubs de
  administración (`FijarRelacion`, guerra por `declara_guerra` sin arrastre) y puntos de llamada claros.
- **Sistema 6**: tecnología (stub `FijarTecnologias`), contadores de logros, Aedas y épicas, equipo de asedio si se decide (Q7).
- **Transversales**: Murallas (Q4), Intel/taberna y niebla, proyecciones y contrato.
- **Fase 7 (bots)**: perfiles que usan estas órdenes; el combate numérico bot contra bot ya queda cubierto por 4D.

### 3.3 Paridad con TypeScript que cierra o abre el sistema 4

Cierra (filas de `Mecanicas a desarrollar.md`): l.74 (mineros, saqueo), l.75 (pueblos con Gobernador/columnas reales), l.76 (avanzadas), l.78 (reconstrucción
tras saqueo), l.80 (guarnición y ración, reserva de trigo, pool de reclutamiento), l.82 (residentes en la cascada de ruinas, parcial), l.85 (ocupación), l.92
(héroes y columnas del comercio), l.96-97 (escolta y combate de caravanas), l.100 (autorización por presencia/Líder), l.101 (escolta del acarreo).
Abre (a anotar si se recorta): relaciones/guerra como stub, tecnologías como stub, murallas ×1, Intel, contrato 05.

---------------------------------------------------------------------------------------------------------------------------------------

## 4. Preguntas al autor (8)

**Q1. Cifras vigentes: bloques de Mechanics o código.**
Los bloques `parametros:*` copian el TypeScript (velocidades de tropa 20/16/12/28, `radioEncuentro` 15, `radioPuerta` 10, `BATALLA.capacidad.asedio` 5,
`VISION` 150/80/60/300…) y lo calibrado vive en la prosa. En el sistema 3 se dejaron los bloques y se corrigió en código con un test. En el sistema 4 son
decenas de cifras, y ya no hay `npm run mecanicas` que los pise.
- A) Reescribir los bloques con los valores calibrados y que el código los lea tal cual (una vez; cambia la huella de las reglas y las fotos viejas dejan de valer).
- B) Dejar los bloques y fijar los valores vigentes en `datos.rs` con un test, como en comercio.
- C) Un bloque nuevo `VIGENTES` aparte.
Recomendada: **A** (una sola fuente de verdad; B repite el error de dos fuentes que ya costó una discrepancia en comercio).

**Q2. Poder de un héroe en el combate con números.**
El canon (5.1) le da 15, pero dice que no está implementado, y `Combate.md` solo suma escuadras (un bando sin escuadras lanza error). La defensa base de las
caravanas (26) se justifica contra esos 15. Hace falta para columnas sin tropa (persecución solitaria entre bots) y para que se cumpla "una caravana sola
se defiende de un jugador solo".
- A) Cada héroe sano que combate suma 15 al poder de su bando, además de sus escuadras.
- B) Solo cuenta cuando el bando no tiene escuadras con soldados.
- C) No existe: una columna sin tropa no puede combatir con números (pierde sin tirada o el ataque se rechaza).
Recomendada: **A** (aditiva, simple, no deja el caso vacío; con un héroe + una milicia sigue mandando la tropa). Se anotaría en `Combate.md` §3.3.

**Q3. Caravana "suelta" (soltarla en marcha y reclamarla).**
C5 l.570 la deja "suelta donde esté la columna" (`disponible`), pero `disponible` significa hoy "en su ciudad" y no es interceptable, mientras C5 l.287
ofrece interceptar una caravana suelta y `Movimiento_Ejercitos.md:125` no deja alcanzar las `disponible`.
- A) Nuevo estado `suelta` en el mapa: atacable (tipo A, fuera de toda zona o según zona) y reclamable por un héroe de su Facción.
- B) Sigue `disponible` pero quieta en el campo: ni atacable ni perseguible hasta que alguien la reclame (como el TS).
- C) Solo se puede soltar dentro de una plaza o campamento (no en marcha).
Recomendada: **A** (es la única que cumple a la vez "se queda suelta", "interceptar" y "reclamar"); toca la máquina de estados de `comercio`.

**Q4. Murallas: ¿dentro del sistema 4?**
El asedio multiplica la defensa por el recinto y la guarnición suma su cupo; `Murallas.md` (302 líneas) no tiene sistema asignado y Rust no tiene recintos
(Paridad l.81).
- A) Sistema 4 con recinto = 0 (asedio ×1, sin cupo de muralla) y Murallas como fase propia después de 4F, como recorte anotado en Paridad.
- B) Murallas antes del asedio (nueva fase 4C-bis), con el multiplicador real desde el primer asedio.
- C) Murallas como "sistema 2b" previo al 4.
Recomendada: **A** (no bloquea nada de lo militar y el batch de guerra puede medirse sin muros primero).

**Q5. Persecución: perder de vista a la presa.**
`Movimiento_Ejercitos.md:122` lo comprueba "en cada muestreo de visión" (250 ms); `Reloj_Y_Eventos.md` dice que lo que cambia el juego es evento exacto.
- A) Evento exacto: se calcula el primer instante en que el plan de la presa sale del radio de vista del perseguidor (el crate `encuentros` ya sabe hacerlo plan contra plan).
- B) Muestreo de 250 ms (fiel al texto; el resultado dependería de la cadencia y habría que registrarlo para reproducir).
Y `umbralRecalculo`: con el plan exacto de la presa se recalcula al cambiar su plan; ¿se conserva el umbral de 6 como histéresis o se retira?
Recomendada: **A**, y retirar el umbral (el recálculo es un evento); se actualiza `Movimiento_Ejercitos.md` §7 y C5 l.308.

**Q6. Corte entre sistema 4 y sistema 5 (héroe, residencia y puertas).**
El héroe necesita residencia, ubicación y presencia ya en el 4B-1, pero ciudadanía, cargos, casas compradas, puertas y relaciones son de política.
- A) El 4 trae héroe, una residencia simple (plaza o campamento), presencia (entrar/salir/desconectar), `declara_guerra` sin arrastre y relaciones por
  administración; el 5 trae ciudadanía, cargos, casas compradas, cooldown, puertas y vetos, arrastre, reputación y derrota.
- B) Todo lo de `Presencia_Y_Convocatoria.md` y la residencia (Doc 2.5) entra entero en el 4.
- C) Residencia y puertas se hacen antes, como parte del sistema 5 adelantado.
Recomendada: **A** (mantiene el 4 acotado y deja el contrato de autorización de la matriz de comandos listo para que el 5 lo rellene).

**Q7. Equipo de asedio (ariete y torre).**
Paridad l.27 dice que las armas de asedio como mecánica aparte están descartadas (2026-10-06); l.45/48 dicen que el equipo de asedio de `carpinteria_militar` y
`trabajos_asedio` "entra con el sistema militar"; C5 l.85 aún lista ariete y torre como producto de Carpintería.
- A) No hay equipo de asedio con efecto (queda como producto sin uso; se limpia el canon y Paridad).
- B) Efecto en el asedio con números (p. ej. reduce el multiplicador de muralla), con Unity aparte.
- C) Solo efecto dentro de Unity; el servidor no lo modela.
Recomendada: **A** (coherente con la decisión del 2026-10-06 y con el recorte de murallas).

**Q8. Caravana aparcada en una plaza que cae por conquista.**
`Combate.md:154` y C3 3.13.7 dicen que se destruye (carros, animales y carga). En 3B-2 se implementó que, si la plaza es conquistada (no arruinada), la
caravana aparcada vuelve a su origen (con DUDA en `comercio/ruina.rs:78`).
- A) Se destruye también en la conquista (lo escrito en canon y Mechanics); se cambia el código de 3B-2.
- B) Vuelve a su origen en la conquista y se destruye solo en la ruina; se corrige el texto.
Recomendada: **A** (es la decisión explícita del 2026-10-13 y la conquista es justo el caso militar; el tren de suministros es objetivo, C5 l.575).

---------------------------------------------------------------------------------------------------------------------------------------

## 5. Decisiones de implementación que no requieren al autor (propuestas)

- Tecnologías mientras no exista el sistema 6: orden de administración `FijarTecnologias` por Facción (como `FijarNivelFaccion`); el batch adopta las Eras I-III.
- Relaciones mientras no exista el 5: `FijarRelacion(f1, f2, guerra|alianza|vasallaje)` de administración; sin relación = neutral.
- Radios y tiempos con prioridad: añadir `PRIO_BATALLA = 1` (cierre de batallas y salidas del mundo) y `PRIO_BANDIDOS = 7`.
- Fotos: `BAN9`; la huella de reglas incluirá los bloques de `Combate.md`, `Heroe_Y_Tropa.md`, `Batalla_Unity_Ciclo.md` y `Movimiento_Ejercitos.md` (este último ya entra por comercio).
- `creadas` de la respuesta y los ámbitos de sesión necesitan tipo de entidad (héroe, escuadra, batalla): se resuelve en 4H, no antes.
- Una prueba de reproducción por fase que consuma el generador en el orden documentado (2 tiradas por combate, 1 en caravana y campamento, 0 sin combate).

> **Estado (2026-10-20): la fase 4D está implementada** (`Atacar`, conquista con cascada de comercio, bandidos en el espacio, saqueo de pueblo, ganchos de caravana; foto `BAN12`). Quedan 4E (ciclo de Unity), 4F (avanzadas), 4G (batch militar) y 4H (contrato y proyecciones).

> **Estado (2026-10-21): la fase 4E está implementada** (el ciclo de la batalla de Unity en el núcleo: apertura con `atacar`, candado y bloqueos, plazos de prioridad 1, los cinco mensajes del servidor de batalla, `UnirseABatalla` / `CancelarBatalla`, aplicar el resultado y retención; foto `BAN13`). Quedan 4F (avanzadas), 4G (batch militar) y 4H (contrato y proyecciones).
