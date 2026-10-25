# Fundación y refundación

> **Reescrito el 2026-10-13** para el mundo reactivo; distancias de fundación calibradas: separación mínima entre plazas **30** y exclusión de **20** respecto a un campamento (Doc 1.8). El «cómo funcionaba» está en `typescript-final:Docs/Mechanics/Fundacion_Y_Refundacion.md`.

## 1. Alcance y fuentes

Cubre la **Caravana de Fundación** —la única forma de fundar un asentamiento— desde que se prepara hasta que funda, se desarma o caduca, y la **refundación**: cómo una Facción que se quedó sin plazas compra esa caravana en un campamento de mercenarios con un fondo común.

- **Verificado contra:** `30c30cd` (2026-10-12).
- **Fuentes:** `engine/expansion.ts`, `engine/refundacion.ts`, `engine/nombresDeCiudades.ts`, `session/comandos/expansion.ts` (`lanzarCaravanaFundacion`, `desarmarCaravanaFundacion`, `fundar`), `session/comandos/mercenarios.ts` (`aportarARefundacion`, `retirarDeRefundacion`, `comprarCaravanaDeRefundacion`),
  `engine/settlement.ts` (`fundarAsentamiento`, `exigirPuertaDeFundacion`), `engine/faccion.ts` (`calcularCapFundacion`), `engine/asentamientoQuery.ts` (cooldown de caravanas).
- **Canon:** [`Docs/Game/1_Sistema_de_Mundo_y_Territorio.md`](../Game/1_Sistema_de_Mundo_y_Territorio.md) 1.2, 1.7, 1.8, 1.9b.
- **No está aquí** (está en su documento):
  - El **estado inicial del asentamiento** que nace (edificios, población, almacén, zona de influencia), la validación `fundarAsentamiento` y la viabilidad de un sitio → [`Economia_Construccion.md`](Economia_Construccion.md) §3.
  - **Enganchar** la caravana a una columna (`adjuntarCaravana`), soltarla, escoltarla y que `caravanasAlEntrar` la aparque → [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §9.
  - El campamento de mercenarios donde se compra: su colocación → [`Generacion_Del_Mundo.md`](Generacion_Del_Mundo.md) §11; su mercado y sus residentes → [`Comercio_Caravanas.md`](Comercio_Caravanas.md) §9. El almacén personal → `Movimiento_Ejercitos.md` §3.
  - Interceptar la caravana de fundación → [`Combate.md`](Combate.md) §7.1. Las demás caravanas (`comercial`) → `Comercio_Caravanas.md`.

Corre por **eventos**: la caravana de fundación tiene un plan (vuelve sola, se desarma al llegar) y un evento de caducidad ([`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md)). Sin titular vuelve sola a su origen; si una plaza anfitriona cae, las caravanas aparcadas en ella se destruyen (Doc 3.13.7).

## 2. Modelo de datos

La caravana de fundación es una `Caravana` de `tipo: 'construccion'`. Campos propios de esta mecánica:

| Campo | Significado |
|---|---|
| `id` | `caravana-fundacion-{origen}-{ids.siguiente()}` (el origen es una plaza o un campamento). |
| `faccionId` | **Su Facción**, siempre (D48): `faccionDeCaravana` usa este campo y, si falta, la Facción de la plaza de origen. |
| `origenAsentamientoId` | La plaza de origen; para la de un campamento lleva **el id del campamento** (y a la vez `origenCampamentoId`). |
| `origenCampamentoId?` | Solo la comprada en un campamento. |
| `contenido` | Lo que costó (el coste completo, o el de refundación): lo que se devuelve. |
| `titularId` | Quien la lleva y funda. Ver §4. |
| `caducaEn?` | Instante en que, suelta, caduca. |
| `aportes?` | Solo la de un campamento: `heroeId → recurso → cantidad` que puso cada aportante (para devolvérselo). |
| `estado`, `ruta`, `progreso`, `posicionActual` | `disponible` (parada) · `adjunta` (en una columna) · `retornando` (vuelve sola a su origen) · `aparcada` (dejada en una plaza con la orden de soltarla; desde el 2026-10-22 entrar con la columna ya no la aparca, entra con ella). |

**Fondo de refundación:** `CampamentoMercenarios.fondos`: `heroeId → recurso → cantidad`. Lo aportado por cada héroe en ese campamento.

> **Implementado en Rust (2026-10-24, 5G-b).** La Caravana de Fundación deja de ser la marca `caravana_fundacion` de la columna y es una entidad, `nucleo::CaravanaFundacion` en `Estado::fundacion` (`crates/nucleo/src/estado_fundacion.rs`, reglas en `nucleo/fundacion.rs`; foto **`BAN31`**, `BAN30` se rechaza): `{ id, faccion, titular, origen, posicion, contenido, aportes, situacion, caduca_en }`. El `id` sale del contador de las caravanas comerciales (en el contrato es `car-N`, una `Caravana` de tipo `CARAVANA_DE_FUNDACION`). `origen` es `Plaza(as)`, `Campamento(camp)` o `Ninguno` (la que da la administración con `DarCaravanaFundacion`, sin coste). `situacion` es `Disponible` (suelta y parada en su origen: es el único sitio donde está suelta), `Adjunta(ej)` o `Retornando(plan)`; el `aparcada` del TypeScript no existe (desde el 2026-10-22 entrar no aparca). El fondo es `EstadoFundacion::fondos: campamento → héroe → recurso → cantidad`, en unidades enteras. Sus dos plazos son eventos de la cola de prioridad 6 con la clave `entidad_fundacion(id)` (`Carga::Fundacion`: `Caduca` y `Llega`); cada cambio de situación sube su generación, así que a una caravana solo le queda en la cola el de la situación en que está y a la enganchada, ninguno.

## 3. El coste (`costoCaravanaFundacion`)

```
costo = FUNDACION.materialesIniciales                                     { madera, piedra, trigo, oro }
      + EDIFICIO_CATALOGO.granja.costo                                    (la granja inicial)
      + EDIFICIO_CATALOGO.vivienda.costo × FUNDACION.viviendasIniciales   (las viviendas iniciales)
      + madera += FUNDACION.costoMaderaExtraCaravana                      (fabricar la caravana en sí)
```

El Centro Urbano no cuesta nada. Con las cifras de hoy: `madera 160, piedra 20, trigo 100, oro 100`.
**Coste de refundación** (`costoRefundacion`): cada recurso del coste normal × `MERCENARIOS.refundacion.porcentajeCoste` (0,4), **redondeado hacia arriba**, quitando los que quedan en 0. Hoy: `madera 64, piedra 8, trigo 40, oro 40`.

> **Implementado en Rust (2026-10-24, 5G-b):** `Nucleo::costo_caravana_fundacion` y `Nucleo::costo_refundacion` (este, redondeado hacia arriba **a unidades enteras**, C3b). `FUNDACION.costoMaderaExtraCaravana` y `caducidadCaravanaHoras` los lee `asentamiento::Reglas`; `porcentajeCoste`, `comercio::ReglasMercenarios::porcentaje_refundacion`. La huella de las reglas no cambia (los bloques ya eran estos).

## 4. Lanzarla desde una plaza (`lanzarCaravanaFundacion`)

Comando `lanzarCaravanaFundacion { origenAsentamientoId }`; el **actor es el titular**. Condiciones, en este orden (`ExpansionInvalidaError`, salvo la primera, que es del comando):

1. El asentamiento existe y su Facción existe; el actor es un héroe.
2. El origen pertenece a la Facción.
3. El titular es **ciudadano** de la Facción.
4. **`nivelActual` ≥ 2** del origen (no el alcanzado: un origen degradado no puede lanzar).
5. **Cooldown de caravanas**: `ultimaCaravanaCreadaEn + CARAVANA_COOLDOWN.cooldownMinutos ≤ ahora`. El cooldown lo **comparte** con la caravana comercial; la de un campamento (§7) no lo toca.
6. **Cap de fundación**: `asentamientos efectivos < calcularCapFundacion(nivel de la Facción)`. Efectivos = plazas de la Facción **más** sus caravanas de fundación vivas: el cupo se **reserva al lanzar**, no al fundar. Cap por nivel: `CAP_FUNDACION_POR_NIVEL` (`calcularCapFundacion` toma el índice `min(largo, max(1, nivel)) − 1`).
7. El almacén del origen cubre `costoCaravanaFundacion()`.

Efecto: el almacén se descuenta, `origen.ultimaCaravanaCreadaEn = ahora`, y nace la caravana `disponible`, **sin destino**, parada en la plaza (`posicionActual = origen.posicion`), con `titularId`, `faccionId` y `caducaEn = ahora + 60 min × caducidadCaravanaHoras` (48 h). Evento `expansion.caravana_lanzada` a la plaza.

> **Implementado en Rust (2026-10-24, 5G-b):** `LanzarCaravanaFundacion { heroe, asentamiento }` (**156**; orden `lanzar_caravana_fundacion`, el héroe es el de la cuenta). Rechazos, en este orden: `asentamiento.desconocido`; `fundacion.no_ciudadano` (el titular no es ciudadano de la Facción de la plaza); `fundacion.no_resides` (reside en la plaza **y está dentro**: Proyecciones §5.2.4, `reside(A)`); `fundacion.nivel_insuficiente` (nivel operativo < 2); `caravana.en_cooldown` (`comercio.ultima_caravana` de la plaza, compartido con las comerciales: lanzar lo estampa); `fundacion.cap_alcanzado` (plazas + Caravanas de Fundación vivas de la Facción ≥ el cap de su nivel: el cupo se reserva al lanzar); `caravana.sin_recursos`. `creadas` lleva la caravana. Hecho `expansion.caravana_lanzada { caravana, heroe, faccion }`, de la plaza.

## 5. Fundar (`fundar`)

Comando sin parámetros; lo ejecuta el **titular**, con la caravana enganchada a su columna. Se funda **donde está la columna** (`columna.posicionActual`).

1. El actor es un héroe con columna (`sinColumna` si no); la columna lleva una caravana `construccion` con `titularId === actor` (y con `faccionId`); si no, `fundacion.sin_caravana`.
2. **Fundadores** = el titular y los demás participantes de la columna, **solo los ciudadanos de la Facción de la caravana**, recortado a `FUNDACION.maxJugadoresFundacionGrupal` (5): unirse a la columna es el consentimiento (D12). Hace falta `exigirPuertaDeFundacion`: al menos `minFundadoresParaFaccionNueva` (1 hoy) fundadores; `exigeCiudadaniaPrevia` está desactivado, y el comando pasa `yaFueCiudadano = true`.
   > **Implementado en Rust (2026-10-25, 5E-3 y 5H)**: el Líder, si es uno de los fundadores, es el Gobernador de oficio de la plaza; la plaza guarda sus fundadores (`Asentamiento::fundadores`, `heroesFundadoresIds`, foto `BAN35`, en el contrato `fundadores_ids`): dejan de serlo al mudarse y la conquista los borra.
   > **Implementado en Rust (2026-10-24, 5B)**: los fundadores son los participantes de la columna (el Líder primero) que son ciudadanos de su Facción, hasta `maxJugadoresFundacionGrupal`, y **pasan a residir en la plaza** al fundarla (dejan la plaza o el campamento donde vivieran; su campamento se traslada con ellos; no estampa el cooldown de residencia). Sin ninguno, `fundacion.sin_fundadores`, después de `fundacion.sin_faccion` y antes de `fundacion.cap_alcanzado`. La columna sigue en el mapa, a la puerta: que se deshaga dentro (paso 5) es de 5G.
3. `fundarConCaravana` añade a lo de `fundarAsentamiento` ([`Economia_Construccion.md`](Economia_Construccion.md) §3: dentro del mapa, fuera de toda zona ya recortada, a 30 o más de toda plaza, no cima ni agua ni celda no transitable, tope de fundación) la exigencia de estar **a `MERCENARIOS.radioExclusionFundar` (20) o más de cualquier campamento** de mercenarios (estricto: a 20 exactos se funda).
   > **Implementado en Rust (2026-10-24, 5G-b)**: `fundar` gasta la Caravana de Fundación enganchada a la columna (la primera, si lleva varias; `fundacion.sin_caravana` si no lleva ninguna) y su hecho lleva `caravana`. Lo da el Líder de la columna (como antes): el titular va en ella mientras la lleva, por la regla de §7. El cap que mira es el de las plazas (la caravana ya ocupó el suyo al lanzarse).
   > **Implementado en Rust (2026-10-13)**: `Comando::Fundar` (`crates/nucleo/src/nucleo/zonas.rs`). Orden de las comprobaciones y códigos de rechazo: `fundacion.sin_caravana`, `fundacion.cap_alcanzado` (plazas de la Facción ≥ cap de su nivel), `fundacion.fuera_del_mapa`, `fundacion.sobre_agua`, `fundacion.sobre_cima`, `fundacion.no_transitable`, `fundacion.dentro_de_zona`, `fundacion.cerca_de_plaza`, `fundacion.cerca_de_campamento`. Un rechazo no deja rastro. Hasta que existan la economía y los héroes, la caravana la da la administración (`DarCaravanaFundacion`) y el nivel de Facción se fija a mano (`FijarNivelFaccion`); la columna del titular **no** se disuelve en la plaza (llega con las plazas habitadas).
4. Nombre: `nombreDeCiudadLibre(nombres de las plazas existentes, id de la nueva)` (§8). Si no queda ninguno, la plaza se queda sin nombre.
   > **Implementado en Rust (2026-10-24, 5C)**: `politica::nombres::ciudad_libre` con la semilla `as-N` (el id del contrato de la plaza nueva); el nombre viaja en `fundacion.asentamiento_fundado` (`valores.nombre`) y en la ficha de la plaza. Fundar borra además la derrota de la Facción y da `NIVEL_FACCION.xp.fundacion` si ya tenía alguna plaza (Diplomacia_Facciones §3.5, §8).
5. **La columna entra en la plaza que acaba de levantar y se deshace dentro** (`absorberColumna` con devolver suministro): la tropa vuelve al campamento y el carro al almacén de la plaza nueva. Sus ids de caravana adjunta restantes quedan `aparcada` en ella. **La caravana de fundación se gasta.**
6. Todos los de la columna quedan `ubicacion: asentamiento` (dentro); los fundadores salen de cualquier campamento de mercenarios donde residieran; la exploración personal de cada fundador se funde con la de la Facción; historial `Funda {id}.`.
7. **Nace protegida un día** (`protegidaHasta = fundadoEn + OCUPACION.proteccionMinutos`, como una plaza recién conquistada; Doc 1.2, 5.12.9; decisión del usuario del 2026-09-27 en el TypeScript, `settlement.ts`): mientras dure, un asedio rebota sin combate ni tirada (`combate.plaza_protegida`). No abre ventana de ocupación.
   > **Implementado en Rust (2026-10-24, calibración del sistema 4, M5)**: `zonas::fundar` llama a `proteger_al_fundar` (`nucleo/conquistas.rs`), que fija `protegida_hasta` y cita `SucesoCombate::FinProteccion` con la clave de `entidad_conquista(plaza)`: el mismo plazo y el mismo evento (`asentamiento.proteccion_terminada`) que tras una conquista; una conquista posterior sube la generación y descarta la cita. Sin cambio de foto (el campo ya existía). Prueba: `nucleo/tests/combate.rs::una_plaza_recien_fundada_nace_protegida_un_dia_y_luego_se_la_puede_asediar`.
8. Eventos: `fundacion.asentamiento_fundado { asentamientoId, faccionId, caravanaId, posicion, heroesIds, nombre? }` y los de `fundarAsentamiento` (p. ej. experiencia de Facción: la fundación da `NIVEL_FACCION.xp.fundacion` solo si la Facción ya tenía alguna plaza).

## 6. Desarmarla a mano (`desarmarCaravanaFundacion`)

Solo su **titular** (el actor), y solo si es `construccion`, está **`disponible`** (suelta: no enganchada ni volviendo) y a `≤ MOVIMIENTO.radioPuerta` de la posición de su origen (plaza o campamento); si no, `ExpansionInvalidaError`. Devuelve el coste (§7.2) y elimina la caravana. Evento `expansion.caravana_desarmada`.
La restricción de la puerta existe para que desarmar no sea mover el coste de un sitio a otro sin recorrer el camino: lo que anda sola o caduca ya lo devuelve donde toca.

> **Implementado en Rust (2026-10-24, 5G-b):** `DesarmarCaravanaFundacion { heroe, caravana }` (**157**): `caravana.desconocida`, `fundacion.no_titular`, `caravana.no_disponible` (enganchada o volviendo). La suelta está siempre parada en su origen, así que la puerta no se mira aparte; como en el TypeScript, el titular no tiene que estar allí. Devuelve como §7.1 y emite `expansion.caravana_desarmada { caravana, heroe, faccion, devuelto }` (con `campamento` si es de uno).

## 7. El tiempo (`avanzarCaravanasDeFundacion`)

Sin ninguna caravana con titular, no hace nada. Para cada caravana con `titularId` (las demás pasan igual):

- **`adjunta`**: busca su columna. Se queda si **la columna existe, el titular va en ella y sigue siendo ciudadano** de la Facción. Si no (otro la lleva, el titular dejó la Facción o salió del mundo, la columna ya no existe): se **quita de la columna** y calcula la ruta de vuelta (`calcularRuta` desde donde está hasta su origen).
  Con ruta: pasa a `retornando` (`progreso 0`), evento `fundacion.caravana_vuelve`. Sin ruta o sin origen: se **pierde** (`fundacion.caravana_perdida`). Mientras vuelve, otro ciudadano puede **reclamarla** (`adjuntarCaravana`) y pasa a ser titular.
- **`retornando`**: avanza `CARAVANA_CATALOGO.construccion.velocidad` (10) por la ruta con `avanzarPosicionEnRuta`; al llegar (`progreso ≥ 1`) **se desarma y devuelve** (`fundacion.caravana_devuelta`).
- **`aparcada` sin `caducaEn`** (la dejó una columna dentro de una plaza; desde el 2026-10-22 guarnecer ya no la aparca, queda enganchada a la columna del Líder hasta que se suelte a mano): pasa a `disponible` y le empieza a correr `caducaEn = ahora + 48 h`.
- **`disponible` con `caducaEn` vencido** (`ahora ≥ caducaEn`): **caduca y devuelve**.
- Cualquier otra: sigue.

> **Implementado en Rust (2026-10-24, 5G-b), sin tick.** Lo que el TypeScript miraba en cada tick lo mira `Nucleo::vigilar_fundaciones` **al cerrar cada versión**, y solo para las caravanas enganchadas cuya columna o cuyo titular tocó la transición (lo único que puede cambiar la respuesta): si la columna ya no existe, el titular no va en ella o ya no es ciudadano de su Facción, se suelta y vuelve sola desde donde estaba la columna (`fundacion.caravana_vuelve { caravana, heroe, faccion, llegaEn }`), con un plan por el A\* sin red a `CARAVANA_CATALOGO.construccion.velocidad` (10 u/min) y la cita `Llega` a su fin. Ya a la puerta de su origen, llega en el acto. Sin camino por tierra o sin origen, se pierde (`fundacion.caravana_perdida`). La suelta caduca con la cita `Caduca` a las 48 h. Al fundirse dos columnas, la caravana sigue con su titular en la que queda; un ejército que entra en un lugar con ella sigue siendo ejército (como con las comerciales). **La de la administración** (`origen = Ninguno`) es andamiaje: va con su columna sea quien sea su titular y solo se pierde si la columna desaparece (así `fundacion.sin_fundadores` sigue pudiendo probarse). Enganchar es `adjuntarCaravana` con su id (`AdjuntarCaravana`, 39): un ejército (`columna.personal_sin_caravanas`) de su Facción (`caravana.faccion_ajena`) en el que va su titular (`fundacion.no_titular`), a la puerta de donde está (`caravana.fuera_de_la_puerta`), suelta (`caravana.estado_invalido`); deja de caducar. Reclamarla es `reclamarCaravana` (Movimiento_Ejercitos §9).

> **Decisiones del autor sobre las dudas de 5G-b (2026-10-24), implementadas en `tanda-5g-b2`:**
> - **Soltarla (5G-b-4).** `soltarCaravana` (142) con la Caravana de Fundación vale **solo dentro de su origen**: el héroe que la orden nombra está dentro de la plaza o del campamento de origen con la columna que la lleva (entró con ella). Queda `Disponible` en la puerta de su origen y vuelve a caducar a las 48 h; en cualquier otro sitio, `caravana.soltar_solo_dentro`. El residente que entra en su residencia con ella conserva la columna dentro para llevarla (como con una comercial adjunta) y, al soltarla, la columna se deshace. Hecho `ejercito.caravana_soltada { caravana, heroe }`. Prueba: `nucleo/tests/fundacion.rs::decision_5gb4_*`.
> - **En el mapa (5G-b-5).** La que vuelve sola está en la rejilla de encuentros (`Cosa::Fundacion`, clave `entidad_fundacion`: `caravana.encuentro` con las columnas enemigas; se reconstruye al abrir el núcleo), se puede perseguir como presa (solo si alguien la tiene por presa: `perseguir` nombra las caravanas comerciales) y se ve en la `VISTA` ajena como cualquier caravana avistada (tipo, Facción, posición, tramo visible y recursos sin cantidades). Se intercepta con `atacar` contra ella (Combate §7.1): defensa de los carreteros (`defensaBaseCaravana`), aguante en la zona de su dueño o de un aliado. Regla (decisión del autor del 2026-10-24): sin carros, en **disputa** el atacante que gana se la lleva entera (la caravana se pierde, `fundacion.caravana_perdida { interceptada: 1 }`, y él carga lo que costó hasta donde le quepa) y en **aguante** pierde el 70 % de lo que lleva, redondeado hacia abajo (también de los aportes, por orden de héroe), y sigue volviendo con el 30 % (`Nucleo::caer_fundacion`). **Con servidores de batalla** abre una batalla de caravana en Unity, como las comerciales: el ticket lleva un solo carro, `car-N:0`, con lo que costó, y los carreteros de defensa; la batalla la deja quieta donde está (fuera de la rejilla) y, si sigue, retoma la vuelta al acabar; el resultado aplica el mismo reparto (no mira `carros_perdidos`). Sin servidores se resuelve con números: hecho `combate.caravana_interceptada` / `combate.caravana_resiste`. Pruebas: `decision_5gb5_*` (números) y, en `tests/batalla_unity.rs`, `la_caravana_de_fundacion_que_vuelve_sola_se_intercepta_en_una_batalla_de_unity_y_en_disputa_se_la_llevan_entera`.
> - **Destruida con su plaza (5G-b-11).** Al caer una plaza (conquista o ruina, `cerrar_comercio`), la Caravana de Fundación suelta en ella se destruye y lo que costó se pierde (`fundacion.caravana_perdida { plaza }`), como las caravanas aparcadas (canon 3.13.7). Prueba: `decision_5gb11_*`.
> - Aceptadas tal como estaban: 1 (enganchar con el titular dentro), 2 (la de la administración va con su columna), 3 (funda el Líder), 6 (desarmar sin estar en el origen), 8 (reclamar a `radioEncuentro`), 9 (`fundar` no deshace la columna: fila de paridad) y 10 (contador de ids compartido).

### 7.1 Devolver (`devolverCaravana`)

- Si **no tiene origen en pie** (la plaza ya no existe; no aplica a un campamento) → se pierde, con evento `fundacion.caravana_perdida`.
- **La de una plaza**: el `contenido` íntegro al almacén del origen, recurso a recurso con `agregarRecurso` (**hasta donde quepa**: lo que excede la capacidad se pierde).
- **La de un campamento** (`origenCampamentoId`): `devolverAportes`: **al fondo de ese campamento, a nombre de cada aportante** según `aportes`. Lo de quien ya no es ciudadano de esa Facción **se pierde**. Nada llega a nadie por arte de magia: cada uno lo retira en persona (§9).

> **Implementado en Rust (2026-10-24, 5G-b):** `fundacion.caravana_devuelta { caravana, heroe, faccion, devuelto }` (con `campamento` si es de uno): `devuelto` es lo que entró de verdad (en una plaza, hasta el hueco de cada recurso; en un campamento, lo de los aportantes que siguen siendo ciudadanos).

## 8. Nombre de la ciudad (`nombreDeCiudadLibre`)

Lista fija `NOMBRES_DE_CIUDADES` (159 hoy) de ciudades de la Edad del Bronce. Se normalizan (sin tildes, minúsculas) para comparar con los nombres ya usados; entre los libres se elige el índice `hash(semilla) mod libres.length`, con `semilla` = el id de la plaza nueva y `hash` = **FNV-1a de 32 bits** sobre los códigos de carácter (`h = 0x811c9dc5`; por carácter `h ^= c; h = imul(h, 0x01000193)`; resultado `h >>> 0`).
No gasta el azar de la partida. Sin libres → `undefined`. Es presentación: el nombre no cambia lo que hace la plaza.

> **Implementado en Rust (2026-10-24, 5C):** `crates/politica/src/nombres.rs` (la lista de 159, la normalización sin tildes ni mayúsculas y FNV-1a de 32 bits sobre UTF-16, el `sigilo::hash32` de la Facción), con las pruebas de `nombresDeCiudades.test.ts`. `renombrarAsentamiento` (Economia_Construccion §12): `RenombrarAsentamiento` (138), lo da quien reside en la plaza.

## 9. Refundar desde un campamento

Solo una Facción **sin ninguna plaza**; quien conserva alguna expande con la caravana de plaza (§4).

### 9.1 El fondo (`aportarARefundacion`, `retirarDeRefundacion`)

El héroe debe estar **en el campamento**: dentro, o con su columna a la puerta (`campamentoDondeEsta`; si no, `campamento.lejos`). Comandos `{ recurso, cantidad, lado }` con `lado ∈ almacen | carro`:

- **Aportar**: el actor es ciudadano de una Facción; esa Facción no tiene asentamientos; cantidad > 0. `lado = almacen` exige **residir en ese campamento** (el almacén personal vive donde se reside); `lado = carro` exige ser el **Líder** de la columna que está en la puerta. Se mueve `min(cantidad, disponible)` (rechaza si es 0) del almacén personal o del carro a `fondos[heroe][recurso]`. Evento `mercenarios.fondo_refundacion` (`sentido: aporta`).
- **Retirar** lo propio: cantidad > 0; mismo requisito de lado; se mueve `min(cantidad, aportado, sitio)` (sitio = `ALMACEN_PERSONAL.capacidad − total` del almacén personal, o `capacidadCargaDe(columna) − lo que ya lleva` en el carro). `sentido: retira`. **El oro es un recurso más**. Se puede retirar mientras no se gaste.
- **Fondo de la Facción** (`fondoDeFaccion`): suma de `fondos` de los héroes que **son ciudadanos de esa Facción**, por recurso.

> **Implementado en Rust (2026-10-24, 5G-b):** `AportarARefundacion { heroe, recurso, cantidad, lado }` (**153**) y `RetirarDeRefundacion` (**154**), con `lado` `almacen | carro` (`LadoDelFondo`; otro valor, `orden.forma_invalida`). Rechazos: `campamento.fuera_de_la_puerta` (ni dentro ni con la columna a su puerta), `faccion.no_pertenece`, `refundacion.con_plazas` (solo al aportar), `cantidad.no_positiva` (menos de una unidad), `refundacion.no_resides` (el almacén con otra residencia), `heroe.sin_columna` y `columna.solo_el_lider` (el carro); aportar, `almacen.sin_existencias` o `carro.no_lleva`; retirar, `refundacion.sin_aporte`, `almacen_personal.lleno` o `carro.no_cabe`. Se mueve en unidades enteras, redondeando hacia abajo. Hecho `mercenarios.fondo_refundacion { campamento, heroe, faccion, recurso, cantidad }` con `sentido` (`aporta | retira`) como texto. `Nucleo::fondo_de_faccion(campamento, faccion)`. El contrato lleva el fondo **por Facción** (`CampamentoMercenarios.fondos`, `f-N`): en `CAMPAMENTO` cada sesión ve **solo el de su propia Facción** (decisión del autor 5G-b-7, `interior::campamento_por_dentro` con el espectador); `ADMIN`, el de todas; la `VISTA`, ninguno.

### 9.2 Comprar la caravana (`comprarCaravanaDeRefundacion`)

El actor es ciudadano; la Facción no tiene plazas; **no tiene ya una caravana de fundación**; `calcularCapFundacion(nivel) ≥ 1`; y el fondo cubre `costoRefundacion()` (si no: «el fondo no cubre el coste: faltan …»). Se gasta recurso a recurso **sobre el mismo conjunto que cuenta el fondo**, recorriendo `faccion.ciudadanosIds` **en orden de ciudadanía** y tomando de cada uno lo que tenga hasta cubrir; lo que sobra de cada uno **se queda en el fondo**, y lo gastado a cada uno se anota en `aportes`.
Nace parada en el campamento (`origenCampamentoId = origenAsentamientoId = campamento.id`, `posicionActual = campamento.posicion`), **sin destino**, con el actor de titular, `contenido = costoRefundacion()` y `caducaEn = ahora + 48 h`. Evento `mercenarios.caravana_refundacion`. **No aplica el cooldown** de caravanas (no es de una plaza).
A partir de ahí es la misma entidad: se engancha, se funda con `fundar` (§5), se desarma (§6) o caduca (§7).

> **Implementado en Rust (2026-10-24, 5G-b):** `ComprarCaravanaDeRefundacion { heroe }` (**155**): `campamento.fuera_de_la_puerta`, `faccion.no_pertenece`, `refundacion.con_plazas`, `refundacion.ya_tiene_caravana`, `fundacion.cap_alcanzado` (cap < 1), `refundacion.fondo_corto` (dice cuánto falta). Se gasta de los ciudadanos **en orden de ingreso** (`Faccion::ciudadanos`); nace parada a la puerta del campamento, con titular, `aportes` y caducidad. No toca el cooldown. Hecho `mercenarios.caravana_refundacion { caravana, heroe, faccion, campamento }`. El ciclo entero (oro de los bandidos del anillo, que solo cobran los héroes sin plaza, Cal-5; aportar; comprar; engancharla; fundar) lo prueba `nucleo/tests/fundacion.rs::una_faccion_sin_plazas_refunda_con_el_oro_de_los_bandidos_del_anillo`.

## 10. Parámetros

<!-- parametros:FUNDACION -->
```json
{
  "maxJugadoresFundacionGrupal": 5,
  "minFundadoresParaFaccionNueva": 1,
  "exigeCiudadaniaPrevia": false,
  "distanciaMinimaAparicion": 200,
  "intentosDeAparicion": 200,
  "materialesIniciales": {
    "madera": 50,
    "piedra": 20,
    "trigo": 100,
    "oro": 100
  },
  "viviendasIniciales": 3,
  "costoMaderaExtraCaravana": 50,
  "caducidadCaravanaHoras": 48
}
```
<!-- /parametros -->

El bloque `CAP_FUNDACION_POR_NIVEL` vive en [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) §3.5 (desde 5C, 2026-10-24, un solo bloque: `politica::nivel::cap_fundacion`).

<!-- parametros:CARAVANA_COOLDOWN -->
```json
{
  "cooldownMinutos": 10
}
```
<!-- /parametros -->

(`MERCENARIOS.refundacion.porcentajeCoste` y `radioExclusionFundar` están en el bloque `MERCENARIOS` de [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md); `CARAVANA_CATALOGO.construccion` —capacidad 150, velocidad 10— en [`Comercio_Caravanas.md`](Comercio_Caravanas.md) §13.)

## 11. Invariantes

| Invariante | Test que lo vigila |
|---|---|
| Lanzar: nace sin destino, parada, con titular, Facción y caducidad; cobra el coste y arranca el cooldown; rechaza no ciudadano, nivel < 2, sin recursos, cooldown y cupo del Cap (que cuenta las ya lanzadas) | `engine/__tests__/caravanaFundacion.test.ts`, `session/__tests__/comandosExpansion.test.ts` |
| Desarmar: la de plaza devuelve íntegro; la de campamento, al fondo a nombre de cada aportante; solo suelta y en la puerta de su origen | `caravanaFundacion.test.ts`, `fundacionDesdePlaza.test.ts` |
| Con el tiempo: suelta caduca y devuelve; enganchada sin su titular se suelta y vuelve; al llegar se desarma; aparcada al guarnecer cuenta como suelta; sin origen se pierde | `caravanaFundacion.test.ts`, `fundacionCampamento.test.ts` |
| Fundar: no a menos de `radioExclusionFundar` de un campamento (cualquiera), sí a esa distancia; ni sin la caravana enganchada; ni dentro de una zona; ni sobre cima | `caravanaFundacion.test.ts`, `fundacionDesdePlaza.test.ts`, `fundacion_terreno.test.ts` |
| La columna del titular funda en grupo con los ciudadanos que lleva, la caravana se gasta y todos residen en la plaza nueva | `fundacionCampamento.test.ts`, `fundacionDesdePlaza.test.ts` |
| Refundación: coste por porcentaje; el fondo suma a sus ciudadanos y no a otros; aportar desde almacén (residente) o carro (en la puerta); retirar; el oro vuelve como un recurso; rechazos (con plazas, no ciudadano, más de lo que hay) | `engine/__tests__/refundacion.test.ts` |
| Comprar: nace parada con titular y aportes, deja el sobrante; gasta lo mismo que cuenta (el aporte de quien se mudó se descuenta, el de otra Facción no se toca); rechaza fondo corto, Facción con plaza o con otra caravana | `refundacion.test.ts` |
| Nombres: son al menos cien, sin repetidos; el mismo id da el mismo nombre; nunca uno ya usado (con o sin tildes y mayúsculas); reparten por toda la lista | `engine/__tests__/nombresDeCiudades.test.ts` |

**Sin test propio:** el reclamo de la caravana que vuelve (`adjuntarCaravana` por otro ciudadano) fuera del flujo de campamento, la pérdida por falta de ruta de vuelta y el redondeo hacia arriba del coste de refundación.

## 12. Casos límite y decisiones que parecen raras

- **El cupo se reserva al lanzar**, no al fundar: una caravana en el camino ya ocupa un puesto del Cap, y una destruida lo libera.
- **El cooldown es de la plaza, y compartido con las caravanas comerciales**: lanzar una de fundación impide construir una comercial durante 10 minutos, y viceversa. Comprar una de refundación en un campamento no lo toca.
- **Devolver a una plaza llena pierde el exceso**: `agregarRecurso` topa por capacidad.
- **Quien deja la Facción pierde su aporte**: `devolverAportes` y `fondoDeFaccion` solo cuentan ciudadanos.
- **Gastar por orden de ciudadanía, no por quién aportó primero**: el fondo se consume de los ciudadanos en el orden de `faccion.ciudadanosIds`, hasta cubrir cada recurso.
- **El titular es único y no se cambia comprándola de nuevo**: solo otro ciudadano que la reclama por el camino (mientras vuelve sola) se convierte en titular.
- **Aparcada ≠ caducando** (la del TypeScript, que aparcaba al guarnecer; hoy se aparca solo a mano): la caravana que una columna deja aparcada empieza a caducar cuando el evento de aparcado la registra (entonces pasa a `disponible` con `caducaEn = ahora + 48 h`), no desde que se aparcó; hasta entonces no tiene `caducaEn`.
- **Sin camino de vuelta, se pierde**: no hay teletransporte; una isla o un enclave sin ruta pierde lo que costó la caravana.
- **La plaza fundada no se mira como zona de nadie**: el punto no puede caer en la zona de **ninguna** plaza existente, ni la de la propia Facción.
- **`minFundadoresParaFaccionNueva` = 1 y `exigeCiudadaniaPrevia` = false** son configuración de las primeras pruebas; el comando pasa `yaFueCiudadano = true`, así que esa segunda exigencia nunca se evalúa hoy.

## 13. Divergencias

Ninguna abierta.

## Catálogo de eventos

Los códigos de los hechos de dominio de este documento (4H T5, 2026-10-23; contrato 05 §8). La prueba `crates/nucleo/tests/catalogo.rs` lee el bloque de cada documento de `Docs/Mechanics` y exige que todo código que emite el núcleo esté en alguno, y que los que el núcleo aún no emite figuren como paridad pendiente en `Docs/Mecanicas a desarrollar.md`. Desde 5G-b (2026-10-24) el núcleo los emite todos.

<!-- eventos -->
```json
[
  "expansion.caravana_desarmada", "expansion.caravana_lanzada", "fundacion.asentamiento_fundado", "fundacion.caravana_devuelta", "fundacion.caravana_perdida",
  "fundacion.caravana_vuelve", "mercenarios.caravana_refundacion", "mercenarios.fondo_refundacion"
]
```
<!-- /eventos -->
