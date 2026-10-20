# Comercio y caravanas

## 1. Alcance y fuentes

Cubre todo lo que mueve mercancía u oro **entre** lugares sin que sea guerra: el **trueque** entre asentamientos, la **flota de caravanas comerciales** (cómo se compone, se lanza, viaja, se
reconfigura y vuelve), el **mercado de una plaza** (órdenes y mostrador), el **mercado de un campamento de mercenarios**, los precios y las comisiones, las **rutas** de las caravanas (trazado,
peajes, red de caminos) y lo que ocurre con todo ello cuando una plaza cae en ruinas.

- **Verificado contra:** `e83cb9b` (2026-10-11) más las decisiones de esa jornada (comisión, recargo por reputación, reparto, capacidad, reputación entre Facciones).
- **Fuentes:** `engine/trade.ts`, `engine/trueque.ts`, `engine/caravanas.ts`, `engine/market.ts`, `engine/mercadoMercenario.ts`, `engine/redCaminos.ts`, `engine/ruina.ts`, `engine/reputacion.ts`,
  `engine/politicas.ts`, `engine/asentamientoQuery.ts` (cupos), `world/rutas.ts`, `world/grafoNavegacion.ts`, `session/comandos/comercio.ts`, `session/comandos/mercenarios.ts` (`comprarEnCampamento`),
  `session/comandos/ejercitos.ts` (`entregarDeCaravana`), `session/comandos/autorizacion.ts`, `session/proyecciones/jugador.ts` (lo que se ve).
- **Canon:** [`Docs/Game/3_Sistema_Economico_y_Comercio.md`](../Game/3_Sistema_Economico_y_Comercio.md) (y Doc 1.6 para la red de caminos, Doc 1.9b para el campamento de mercenarios).
- **No está aquí** (está en su documento): la caravana **adjunta** a un ejército, sus víveres y su carga (→ [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §9); interceptar, capturar o ser atacada por
  bandidos, y los tipos A/B según la zona (→ [`Combate.md`](Combate.md) §7.1); la **Caravana de Fundación** (→ [`Fundacion_Y_Refundacion.md`](Fundacion_Y_Refundacion.md)); el Mercado como edificio (costes, niveles,
  puestos) y las políticas como sistema (→ `Economia_Construccion.md`); el generador aleatorio (el comercio **no consume** ninguna tirada).

**Reescrito el 2026-10-13** para el mundo reactivo ([`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md)): las caravanas son **planes** con eventos de salida, de peaje y de llegada; los trueques y las órdenes de mercado caducan como **eventos** a su instante; la asignación automática se reevalúa cuando nace un trueque, queda libre una caravana o cambia el almacén de un lado. El «cómo funcionaba» por tick está en `typescript-final:Docs/Mechanics/Comercio_Caravanas.md`.

> **Implementado en Rust (2026-10-19, fase 3B-1: la parte estática del comercio).** Las reglas puras son el crate `crates/comercio` (fase 3A); su integración en `crates/nucleo`:
> - `estado_comercio.rs` (`Estado.comercio`: trueques, libro de órdenes con su historial, flota, reputación por Facción, `ultimaCaravanaCreadaEn` por plaza, contadores de ids, y —provisionales— el carro de cada columna y las caravanas adjuntas), `codec_comercio.rs` (serialización para el hash y las fotos; formato **`BAN5`**, `BAN4` se rechaza) y `nucleo/mercado.rs` y `nucleo/flota.rs` (órdenes, plazos, consultas y cierres). **La huella de las reglas de las fotos incluye los bloques de parámetros de `Comercio_Caravanas.md`, `Tecnologia_Aedas.md` y `Movimiento_Ejercitos.md`.**
> - **Plazos como eventos** `Carga::Comercio` de prioridad 6: caducidad de un trueque (`propuesto` sin contestar → `expirado` sin penalizar; `activo` → vence y penaliza a cada lado en proporción a lo no entregado, solo entre Facciones distintas), caducidad de una orden de mercado y fin de la preparación de una caravana (de momento solo se registra: `comercio.preparacion_terminada`). Cada entidad lleva su propia generación (clave con etiqueta en los bits altos), y contestar, cancelar o cerrar invalida el plazo. **Orden dentro del mismo instante** (la cola solo ordena por prioridad): llegadas, fin de preparación, caducidades y reparto; un evento de rango `r` se reprograma `r` veces en ese instante, de modo que sale detrás de los que ya estaban. La reputación (`Carga::Reputacion`, prioridad 9) reprograma su cita tras cada cambio: cruce del 0 (reancla) o salida del umbral `-40`.
> - **Órdenes** (`Comando`, códigos 21-40; cada rechazo lleva un código estable en `rechazos`): `ProponerTrueque`, `AceptarTrueque`, `RechazarTrueque`, `ColocarOrdenMercado`, `CancelarOrdenMercado`, `ComerciarEnPlaza`, `CrearCaravana`, `AnadirCarro`, `AnadirAnimal`, `MoverCarro`, `ReservarCaravana`, `PrepararCaravana`, `CancelarCaravana`, `MoverCargaAparcada`, `EnviarCaravanaAlOrigen`, `EntregarDeCaravana` (§4.5) y, como andamiaje de administración, `DarRecursos` y `DarSuministro` (`AdjuntarCaravana` y `AparcarCaravana` pasaron a ser órdenes reales en la fase 3B-2). **Autorización provisional**: la administración o el propietario de una columna de la Facción de la plaza (en el trueque, la del lado al que le toca); los cargos, la residencia y la presencia vendrán del sistema de héroes y residentes.
> - **Contra el almacén real**: el mostrador, los carros, los animales, la carga de `prepararCaravana` y su cancelación (reingreso topado por la capacidad), la carga de una aparcada y la entrega mueven el almacén de la plaza; **el oro solo cambia por lo que el diseño acuña (la comisión de la entrega) o destruye (los animales, lo que no cabe, una caravana perdida)**: el recargo por mala reputación sale del oro del origen y entra en el del destino, y la comisión del mostrador se queda en la plaza. Los **cupos** de la flota salen del Mercado activo y de las políticas de la plaza (`FactoresPolitica`).
> - **Ruina y conquista** (§10): `arruinar` llama a `comercio::ruina::cerrar_dependientes` y aplica el resultado (trueques `expirado` sin penalizar con `trueque_cancelado` a la socia, caravanas de la flota perdidas, las que iban a la plaza vuelven con su carga, las aparcadas se destruyen, órdenes borradas). `Nucleo::cerrar_comercio_por_conquista` hace lo de la conquista: la flota pasa con la plaza, los trueques se anulan igual y las que iban hacia ella regresan, y las aparcadas en ella se destruyen; el sistema militar la llamará.
> - **Consultas**: `acuerdo`, `acuerdos_de`, `libro_ordenes`, `escaparate(plaza, presente)`, `flota_de`, `caravana`, `reputacion`, `suministro`, `cupos_de_flota`. `Nucleo::entregar_contenido` es el gancho de la llegada de una caravana (§4.4) con el que las pruebas entregan una carga sin esperar al viaje (el evento de llegada de la fase 3B-2 usa las mismas piezas).
> - **Falta**: lo que cubre la nota siguiente (fase 3B-2). Pruebas de esta fase: `crates/nucleo/tests/comercio.rs`.

> **Implementado en Rust (2026-10-19, fase 3B-2: las caravanas viajan).** Reglas puras en `crates/comercio` (con tres campos nuevos en `Caravana`: `costes`, el coste de cada tramo de su ruta, y `pausada_desde`; `Navegacion.factor_camino`; y `RedCaminos::registrar_ruta` devuelve qué aristas abre) y `crates/rutas` (`Opciones.atraccion`: la red de caminos como pesos que abaratan aristas en el A\*, y `a_plan_por_minuto`: plan con el coste de cada tramo y la velocidad en unidades por minuto). Integración en `crates/nucleo/src/nucleo/{viaje,mercenarios}.rs`; formato de foto **`BAN6`** (`BAN5` se rechaza).
> - **Plan y generación.** Una caravana en viaje es un `Plan` cuya generación es la de su entidad en la tabla de la cola (`entidad_caravana`): cada plan nuevo la sube y descarta de golpe la llegada, el peaje y el fin de preparación programados con el anterior; el mismo número viaja en `Plan.generacion`, que es lo que ve la rejilla de encuentros. Tiempo de un tramo = `coste × espaciado × 60 / velocidad` ms, con el coste dividido por `factorCamino` sobre las aristas de la red. El regreso desanda la ida con las mismas duraciones (`invertir_plan`).
> - **Trazado (§7.2).** `trazarRutaComercial` como está escrito: un A\* por cada zona de otra Facción que cruza (la zona es el disco recortado por las fronteras; la ruta pasa por el centro de la ciudad y deja un peaje pendiente), con la atracción de la red vigente. Los A\* se memorizan por `(extremos, época de la red)` y los pares sin ruta por tierra se recuerdan; la memoria no es estado. Un rechazo (o una consulta) no toca el estado: la poda de la red solo ocurre al registrar una ruta en una orden o un evento aceptados.
> - **Eventos de la cola, prioridad 6** (`SucesoComercio`, rango dentro del instante): llegada y peaje (0), fin de preparación (1), caducidades (2), pulso del reparto y reposición de mercenarios (3). La ruina (el paso de minuto de la plaza, prioridad 2) va antes que todos: una caravana `preparando` hacia una plaza que cae en ese mismo instante no llega a salir.
> - **Peajes** fijados al lanzar (las ciudades ajenas cruzadas, también aliadas y vasallas) y cobrados en el instante de paso: el 2 % de lo que lleva en ese momento a esa ciudad; el último se cobra antes de entregar; el regreso no paga.
> - **Reparto automático (§4.3).** Se reevalúa al aceptar un trueque (las dos plazas), al quedar libre una caravana (animal montado, reserva quitada, carro movido, preparación cancelada, vuelta a casa, llegada tras una entrega) y con un **pulso de un minuto por plaza** que solo existe mientras haya un lado pendiente sin cargar y una caravana libre; se detiene solo y no recurre por plaza ociosa. Sale con cualquier carga positiva, y la urgencia cuenta desde la aceptación.
> - **Red de caminos (§7.4).** Cada lanzamiento registra su ruta, también el que se cancela. La poda es perezosa: las consultas (`pesos_de_red`, `camino_compartido_abierto`) ya no ven las rutas con 4.320 min, y la red guardada se poda al registrar la siguiente. Una arista que entra por primera vez en la red hace que las caravanas que van por ella rehagan su plan desde donde están (conservan su pasado y sus peajes); la que acaba de registrarla no (el primer viaje abre el camino, el segundo va el doble de rápido).
> - **Adjunta y aparcada.** `AdjuntarCaravana`/`AparcarCaravana` son órdenes del dueño de la columna, con la Facción y la puerta (2 u) comprobadas; la columna viaja a la velocidad del más lento y su plan se reescala al engancharse o soltarse. `Ejercito.suministro` (el carro) y `Ejercito.caravanas_adjuntas` viajan en la foto con la columna. Con carga, `EnviarCaravanaAlOrigen` viaja sin atracción ni peajes y vuelca al llegar.
> - **Ruina y conquista con caravanas en ruta.** Las que iban hacia la plaza dan la vuelta con su carga por un plan nuevo desde su posición real (sin camino por tierra se pierden); las que salían de la ruina se pierden; las `preparando` vuelven al almacén; la aparcada en una plaza conquistada se destruye con todo, como en la ruina (decidido el 2026-10-19; antes volvía a su origen).
> - **Mercenarios (§9).** `ComprarEnCampamento`, `RepostarEnCampamento` y, como andamiaje, `FijarResidencia` y `DarAlmacenPersonal` (códigos 41-44). El stock solo se guarda de los campamentos tocados; la reposición es una cita a la fase de siempre (`creación + k × 3 h`) que solo se programa si hay contadores o una pila por debajo de su tope. Lo comerciado entre Facciones (trueque cumplido y mostrador) suma a los contadores.
> - **Batalla.** `Nucleo::caravana_entra_en_batalla`, `caravana_sale_de_batalla` (el plan se desplaza lo que duró la pausa) y `aplicar_perdida_de_carros`; la rejilla de encuentros deja el hecho `caravana.encuentro` entre dos caravanas o con una columna. La resolución (combate A/B) es del sistema militar. Una caravana grande en tránsito se ve a 60 unidades (`VISION.caravanaGrande.radio` vigente).
> - **Hechos nuevos** (no están en §12): `comercio.caravana_en_casa`, `comercio.caravana_en_batalla`, `comercio.caravana_sale_de_batalla`, `comercio.carros_perdidos`, `caravana.encuentro`, `mercenarios.repuesto`, `ejercito.caravana_adjuntada`, `ejercito.caravana_aparcada`. `comercio.preparacion_terminada` desaparece: el fin de preparación emite `comercio.caravana_sale`.
> - **Falta:** del resto de sistemas (héroes, militar, política, niebla, contrato 05): véase «Paridad con TypeScript» en `Docs/Mecanicas a desarrollar.md`. Pruebas: `crates/nucleo/tests/caravanas.rs`.

> **Implementado en Rust (2026-10-19, fase 3C: el carro de acarreo).** Los pueblos extractores y su obra están en `Economia_Construccion.md` §14; aquí, el carro. Reglas puras en `crates/comercio/src/acarreo.rs` (funciones sobre una [`Caravana`] de tipo `acarreo`, ya sin la estructura propia de la fase 3A) y, en `Caravana::llegar`, la llegada nueva `Llegada::Acarreo`; integración en `crates/nucleo/src/nucleo/{pueblos,viaje}.rs`. Sin cambio de formato de foto más allá del `BAN7` de los pueblos (hoy `BAN8`, ventana del acarreo; `Economia_Construccion.md` §14): el carro es una caravana más de la flota (`TipoCaravana::Acarreo`, ya serializable).
> - **Es una caravana de la flota de la madre** (origen = la plaza madre, parada en el pueblo), con su plan, su generación y sus eventos `LlegadaCaravana` / `PeajeCaravana`; no cuenta contra `cupoCaravanas` ni `cupoCarros`, ni entra en el reparto automático ni en el trueque, y no le afectan «Carga Ampliada» ni «Rutas Rápidas» (capacidad 500, velocidad 24).
> - **Sale lleno**, sin evento propio: en el paso de minuto de la plaza en que el almacén local llega a 500 (`pueblo.almacen_lleno`), cuando el carro vuelve a casa y el almacén ya está lleno, o al reponerlo. Carga exactamente la capacidad y emprende la ruta pueblo → madre: el trazado de §7.2 (A\* con la atracción de la red de caminos, peajes de las zonas ajenas que cruce), el plan por tramos de `crates/rutas` y los mismos eventos de llegada y peaje. **No registra su ruta en la red de caminos** (la red es de rutas entre plazas y su poda mira esas plazas) y la ruta se traza en cada salida (el A\* está memorizado) en vez de guardarse «una sola vez».
> - **Al llegar** vuelca en el almacén de la madre sin comisión ni trueque, **topado por su capacidad** (lo que no cabe se pierde: `acarreo.llega` lleva `perdido`; una plaza con 400 de capacidad por recurso pierde 100 de cada 500), y vuelve vacío desandando el camino (`acarreo.vuelve`). Si la madre ya no existe, el carro se pierde.
> - **Si se destruye** (batalla, `DestruirPueblo`, ruina de la madre), el pueblo se queda sin carro: acumula hasta llenar el almacén y para. `ReponerCarro` lo compra en la madre (carro + buey) solo si no tiene ninguno.
> - **Sin ruta por tierra** entre el pueblo y la madre el carro no sale (DUDA).
> - **Batalla.** Es una caravana normal para `caravana_entra_en_batalla`, `caravana_sale_de_batalla` y `aplicar_perdida_de_carros`, y para la rejilla de encuentros; la resolución (casi siempre B) es del sistema militar. La escolta cedida por héroes residentes existe desde la fase 4B-1 (§5.5); la del carro de acarreo, sin probar.

> **Batch de comercio (2026-10-19).** `batch comercio [--plazas 160] [--dias 30] [--semilla 42] [--decisiones crates/batch/decisiones/comercio.json]` corre el núcleo real con 146-160 plazas de Facciones de cuatro y un «jugador» mínimo (Mercado, caravana, trueques por doble coincidencia, pueblos de Cantera con materiales comprados a los mercenarios) y deja `batch/salida/comercio/informe.html`. Resultados de la primera corrida: los invariantes del comercio se cumplen (0 violaciones) y la corrida es determinista; lo medido que merece decisión (peaje y repescas, acarreo contra un almacén lleno, comisión de bienes sin precio, reputación que se borra en 1,74 días, comercio que se apaga sin sumideros) está en `Docs/Mecanicas a balancear.md` §43-§46 y en las decisiones C1-C6 del informe.

## 2. Modelo de datos

### 2.1 `AcuerdoTrueque` (`estado.acuerdos`)

| Campo | Significado |
|---|---|
| `id` | `trueque-{A}-{B}-{contador}` |
| `asentamientoAId`, `asentamientoBId` | Los dos lados. Siempre asentamientos distintos. |
| `lineasA`, `lineasB` | Lo que entrega cada lado: lista de `{recurso, cantidadTotal, cantidadEntregada}`. |
| `creadoEn`, `expiraEn` | Instantes de mundo. Mientras está `propuesto`, `expiraEn` es el plazo para **contestar**; al aceptar se **recalcula** desde el instante de la aceptación. |
| `estado` | `propuesto` → `activo` → `cumplido` \| `expirado`; o `propuesto` → `rechazado`. |

Invariantes de las líneas (los comprueba `proponerTrueque`): cada lado tiene **al menos una** línea; toda `cantidad > 0`; **un lado no repite recurso** (se suman en una sola línea). El recurso puede
ser cualquiera de `RECURSOS_TIPO`, **oro incluido**.

### 2.2 `OrdenMercado` (`estado.ordenes` en pie; `estado.historialOrdenes` las cerradas)

`id` (`orden-{asentamiento}-{contador}`), `asentamientoId`, `tipo` (`compra` = la plaza quiere comprar; `venta` = la plaza vende), `recurso`, `cantidad`, `cantidadCumplida`, `precioUnitario`, `creadoEn`,
`expiraEn`, `estado` (`activa` → `cumplida` \| `expirada`).

### 2.3 `Caravana` comercial (`estado.caravanas`)

Una caravana es un contenedor de **carros** (cada uno con a lo sumo un animal) y de **escolta cedida** (solo ids). Campos que usa el comercio:

| Campo | Significado |
|---|---|
| `tipo: 'comercial'` | La flota propia. Las otras (`construccion` = Fundación; `militar`, `contrabando` = solo datos) no pasan por aquí salvo donde se indica. |
| `origenAsentamientoId` | Su casa para siempre. Nunca cambia, tampoco al viajar. |
| `destinoAsentamientoId` | El destino de la entrega. **Se conserva durante el regreso** (§6.4): el destino ya entregado sigue siendo `destino`. |
| `contenido` | Lo que lleva, `recurso → cantidad`. |
| `carros[]` | `{tipoCarro: 'basico'\|'reforzado', animal?: 'buey'\|'caballo'\|'camello'}`. Siempre presente en una comercial. |
| `estado` | `disponible` (parada en su origen, sin asignar) · `preparando` (lanzada a mano, esperando en el origen) · `en_transito` (hacia el destino) · `retornando` (de vuelta) · `adjunta` (enganchada a un ejército) · `aparcada` (dejada en una plaza de su Facción). |
| `ruta`, `progreso`, `posicionActual`, `peajes[]` | La polilínea calculada al lanzar, el avance `0..1` sobre su longitud, y los pasos forzados por ciudades ajenas que aún no ha cruzado (`{asentamientoId, progreso}`). |
| `origenAcuerdoId`, `ladoAcuerdo` | Solo en el reparto automático: a qué trueque y a qué lado sirve. |
| `preparaHasta` | Solo en `preparando`: el instante en que sale. |
| `escoltaIds[]` | Escuadrones cedidos (viven en su héroe con `contenedor: {tipo: 'escolta', caravanaId}`). |
| `reservadaManual` | `true` ⇒ fuera del reparto automático, sea cual sea su tamaño. |

`Asentamiento.ultimaCaravanaCreadaEn` guarda el instante de la última caravana creada desde él (comercial **o** de Fundación: el cooldown es compartido).

### 2.4 Red de caminos (`estado.red: RedCaminos`)

- `aristas: string[]` — claves canónicas `claveArista(a, b)` = `"x,y;x,y"` con los dos extremos **redondeados a entero** y ordenados como cadenas. Una arista entra cuando alguna ruta la pisa y **no sale nunca**.
- `rutas: RutaComercial[]` — `{id: "{origen}>{destino}", origenId, destinoId, faccionId (la del origen al lanzar), aristas, ultimoLanzamiento}`. Una por par ordenado.

### 2.5 Mercado de mercenarios

`CampamentoMercenarios.mercado: Record<bien, unidades>` (el stock en venta de ese campamento) y `estado.mercadoMercenario: {contadores: Record<bien, número>, reponeEn: Instante}` (lo comerciado en el
mundo desde la última reposición, y cuándo toca la siguiente; **global**, no de un campamento). `Heroe.cupoCampamento?: {dia, comprado}` lleva el cupo diario.

## 3. Precio de referencia, comisión y bonificación

### 3.1 Precio de referencia (`calcularPrecioReferencia(recurso, asentamientos)`)

```
base = PRECIO_BASE[recurso]            ; si no tiene (el oro) → 0  (no cotiza contra sí mismo)
stock = Σ sobre TODOS los asentamientos del mundo de cantidadDisponible(almacén, recurso)
factor = clamp( stockObjetivoGlobal / max(1, stock), factorMin, factorMax )
precio = base × factor
```

Es **global** (suma todas las plazas de todas las Facciones, también de bots) y sin componente de distancia. Se evalúa **en el momento**: en el tick, sobre el mapa de asentamientos tal como va
quedando.

> **Precio base de los manufacturados (2026-10-19, decidido por el autor).** `PRECIO_BASE` solo cubre las materias primas, así que armas, cuero, lingotes y armaduras valían 0 al calcular la comisión (`batch comercio`: 77,8 de oro en 519 entregas). Ahora todo bien con receta tiene un **precio base derivado: lo que consume su receta, a precio base, por unidad fabricada** (cuero con 0,25 de ganado = 0,5; arma de madera con 2 de madera = 2; lingote de bronce con 1,6 de lingote de cobre y 0,4 de estaño = 21,6). Se resuelve en cadena hasta que nada cambia (un lingote de bronce necesita antes los de cobre y estaño) y después sigue la misma fórmula (`base × factor` con el stock global de ese bien). Un bien cuyos insumos no cotizan (las chatarras) o el oro no tienen precio. Si un bien tiene recetas distintas por taller o nivel se toma la **más barata** (la más conservadora para la comisión). No hay bloque de parámetros nuevo: se deriva de `EDIFICIO_CATALOGO` al cargar (`ReglasPrecio::derivados`, `precios_derivados` en `crates/comercio/src/datos.rs`), así que **la huella de las reglas no cambia**. **Los mercenarios no venden los manufacturados**: `BIENES_EN_VENTA` sigue siendo solo `PRECIO_BASE` (§9.1).

### 3.2 Tasa de comisión

```
si origen.faccionId == destino.faccionId :  COMISION.tasaMismaFaccion
si no                                    :  COMISION.tasaExterna × factorComisionExterna(destino)
```

`factorComisionExterna` es el **producto** de ese campo en las políticas activas del destino (`Comercio Abierto` ×0,6, `Aranceles Proteccionistas` ×1,5 y `Arterias Comerciales` ×0,8, esta del Maestro
de Obras; 1 si no hay ninguna; ver `POLITICA_CATALOGO` en `Economia_Construccion.md`).

### 3.3 Comisión de una entrega de trueque o de caravana manual (`comisionDeEntrega`)

```
valorTotal = Σ cantidad × calcularPrecioReferencia(recurso)       ; el oro vale 0
distancia  = max(1, distancia en LÍNEA RECTA origen→destino)       ; no la longitud de la ruta
bonus      = 1 + min(1, distancia / distanciaParaBonusMax) × (bonusPorDistanciaMax − 1)
comisión   = origen.id == destino.id ? 0 : valorTotal × tasa × bonus
```

La comisión se **acuña**: se suma como oro al almacén del **destino** (topada por su capacidad de oro) y no se resta de nadie. Es una de las fuentes de oro del juego (Doc 3.1). **Solo entre dos asentamientos
distintos**: llevarse la carga a uno mismo no es comerciar (decidido el 2026-10-11).

**Recargo por mala reputación** (`recargoPorReputacion`, decidido el 2026-10-11): si las Facciones de origen y destino son **distintas** y la reputación de la del **origen** es
`≤ REPUTACION.umbralBajoParaComision`, esa Facción paga **de su bolsillo** un extra: `min( comisión × (factorComisionPorReputacionBaja − 1), oro del almacén de origen )`. Sale del oro del almacén
de origen y se suma al del destino (no se acuña: se conserva). Entre plazas de la misma Facción no hay recargo. El mismo cálculo vale para la entrega manual (§4.5).

### 3.4 Comisión en el mostrador

Distinta: se **conserva** (§8.3). No lleva bonus por distancia ni recargo por reputación.

<!-- parametros:COMISION -->
```json
{
  "tasaMismaFaccion": 0.03,
  "tasaExterna": 0.08,
  "bonusPorDistanciaMax": 1.5,
  "distanciaParaBonusMax": 600
}
```
<!-- /parametros -->

<!-- parametros:PRECIO_BASE -->
```json
{
  "madera": 1,
  "piedra": 1.2,
  "trigo": 1.5,
  "cobre": 3,
  "estano": 6,
  "hierro": 4,
  "livestock": 2
}
```
<!-- /parametros -->

<!-- parametros:PRECIO_REFERENCIA -->
```json
{
  "stockObjetivoGlobal": 500,
  "factorMin": 0.4,
  "factorMax": 3
}
```
<!-- /parametros -->

## 4. El trueque

### 4.1 Proponer (`proponerTrueque`, comando del mismo nombre)

Condiciones del motor: A ≠ B; los dos asentamientos existen; las líneas cumplen §2.1. Resultado: acuerdo `propuesto`, `expiraEn = ahora + TRUEQUE.plazoMinutosPorDefecto`, `id` con el siguiente
contador de la sesión. **No se comprueba** ni Mercado, ni stock, ni relación política entre las Facciones, ni que sean de Facciones distintas: se puede proponer a cualquiera, y consigo mismo (entre dos
plazas propias). Autoriza el comando: el Tesorero o el Gobernador del asentamiento A, residente y dentro (decidido el 2026-10-12; ver §11; el «basta residir» de TypeScript queda superado).

### 4.2 Contestar (`aceptarTrueque`, `rechazarTrueque`)

Solo mientras sigue `propuesto` (una vez), y solo el Tesorero o el Gobernador del asentamiento **B**, residente y dentro (§11).
- **Aceptar**: `estado = activo`, `expiraEn = ahora + plazoMinutosPorDefecto` (el plazo **se cuenta desde el sí**). Aceptar no traza ningún camino.
- **Rechazar**: `estado = rechazado` (queda, no se borra).
- **Un trueque activo no se cancela** (decidido el 2026-10-12): una vez aceptado, o se cumple o vence. No hay comando para retirarlo; la única salida anticipada es la ruina de una de las dos plazas (§10, sin penalización).
- Una propuesta que nadie contesta **caduca** en `asignarCaravanasATrueque` (§4.3) al cumplirse `expiraEn`: `estado = expirado`, evento `comercio.trueque_expirado`, **sin penalización**.

### 4.3 El reparto automático (`asignarCaravanasATrueque`, dentro de `avanzarComercio`)

Corre **después** de mover las caravanas (§6) y de podar la red. Para cada acuerdo, en el orden del array:

1. `propuesto`: si `instante ≥ expiraEn` → caduca (arriba) y sigue.
2. Si no está `activo`: se ignora.
3. `activo` y `instante ≥ expiraEn` → **vence**: `expirado`, evento `comercio.trueque_expirado`, y por cada lado con `fraccionIncumplida > 0` un ajuste de reputación
   `REPUTACION.penalizacionTruequeIncumplido × fraccionIncumplida` a la Facción de ese lado (§4.6). Lo ya entregado no se devuelve.
4. `activo` vivo: se forman los dos **lados pendientes** (A: sale de A hacia B con las líneas de `lineasA` que faltan; B al revés). Un lado entra si le falta algo **y** no hay ya una caravana
   `en_transito` con `origenAcuerdoId == acuerdo.id` y `ladoAcuerdo == lado` (un solo cargamento en ruta por lado; una `retornando` ya no cuenta, porque al entregar pierde esas marcas).

Luego, **por asentamiento de origen** (en el orden en que aparecen):

```
disponibles = caravanas con tipo 'comercial', estado 'disponible', !reservadaManual, origen = este asentamiento, capacidadCaravana > 0    (en el orden del array de caravanas)
             ; las que no tienen ningún carro con animal no cuentan: no pueden cargar nada ni ocupan el turno de las que sí
si no hay → siguiente asentamiento
para cada lado pendiente con destino existente: score ∈ [0,100]   ; orden DESCENDENTE (sort estable)
idx = 0
para cada (lado, destino) en ese orden:
    si idx ≥ |disponibles| → fin
    caravana = disponibles[idx]
    libre = capacidadCaravana(caravana) × factorCapacidadCaravana(origen)
    carga = {} ; para cada línea pendiente del lado, EN EL ORDEN DEL ACUERDO:
        cantidad = min( cantidadDisponible(almacén origen, recurso), faltante, libre )
        si cantidad > 0 : carga[recurso] = cantidad ; libre −= cantidad
    si carga está vacía → continue                        ; NO avanza idx: la caravana se reintenta con el siguiente lado y el siguiente tick
    trazado = trazarRutaComercial(origen → destino)       ; §7.2
    si no hay trazado → continue                          ; sin camino por tierra no sale y NO gasta la caravana
    idx++
    red = registrarRuta(red, origen, destino, trazado.ruta, instante)
    almacén origen −= carga
    caravana → { estado en_transito, destino, contenido carga, origenAcuerdoId, ladoAcuerdo, posicionActual = origen.posicion, progreso 0, ruta, peajes }
    evento comercio.caravana_sale
```

**Score** (`scoreAsignacion`): `urgenciaExpiracion × peso + urgenciaVolumen × peso + cercanía × peso`, con

```
urgenciaExpiracion = 100 × (1 − max(0, expiraEn − ahora) / (expiraEn − creadoEn))        ; 100 si el plazo total es 0
urgenciaVolumen    = 100 × min(1, (total − entregado) / total)                             ; sumas de TODO el lado
cercanía           = 100 × (1 − min(1, distanciaRecta / ASIGNACION_CARAVANA.distanciaReferencia))
```

Notas: **la urgencia se cuenta desde la aceptación** (decidido el 2026-10-18; el TypeScript no reescribía `creadoEn` y el tramo ya consumido del plazo contaba desde la propuesta): se guarda `aceptadoEn` y `urgenciaExpiracion` mide `expiraEn − aceptadoEn`; y el reparto **no mira Mercado ni cupos**: una caravana ya creada es
reparto aunque el Mercado ya no esté. El reparto **no compone** caravanas ni asigna escolta; despacha la escolta que la caravana ya tenga cedida (`escoltaIds` se conserva).

<!-- parametros:TRUEQUE -->
```json
{
  "plazoMinutosPorDefecto": 1440,
  "retencionTerminadosMinutos": 10080
}
```
<!-- /parametros -->

<!-- parametros:ASIGNACION_CARAVANA -->
```json
{
  "pesoUrgenciaExpiracion": 0.5,
  "pesoUrgenciaVolumen": 0.3,
  "pesoCercania": 0.2,
  "distanciaReferencia": 600
}
```
<!-- /parametros -->

### 4.4 Entrega y cierre (`avanzarCaravanas` → `aplicarEntregaATrueque`)

Al llegar al destino (§6.3) la caravana entrega todo su `contenido` (cada recurso con `agregarRecurso`, **topado por la capacidad** de ese recurso en el almacén del destino; **lo que no cabe se
pierde**) más la comisión (§3.3), y emite `comercio.caravana_llega`. Si la caravana lleva `origenAcuerdoId`, se abona al trueque:

```
para cada (recurso, cantidad) en contenido :  línea del lado de esa caravana con ese recurso . cantidadEntregada += cantidad       ; lo que ninguna línea pide no suma
si acuerdoSaldado (todas las líneas de los dos lados: cantidadEntregada ≥ cantidadTotal) :
    estado = cumplido ; evento comercio.trueque_cumplido {entreFacciones: faccion(A) ≠ faccion(B), intercambiado: Σ cantidadTotal por recurso, de los dos lados}
    si entreFacciones, para faccionId en [faccion(A), faccion(B)] :        ; la reputación y la experiencia son entre Facciones
        reputación += REPUTACION.bonusTruequeCumplido
        experiencia de Facción += NIVEL_FACCION.xp.truequeCumplido
```

El abono usa lo que la caravana **lleva al llegar**, es decir, tras los peajes (§7.3): si un peaje se llevó un 2 %, la línea queda corta y hace falta otro viaje.

> **Línea saldada por resto (2026-10-19, decidido por el autor; `batch comercio` medía 3,35 salidas por trueque).** Una línea con algo entregado y **menos de una unidad por entregar** se da por **saldada**: no se repesca y el resto queda en el almacén de origen. Con `Linea.faltante() = 0` el trueque se salda (`acuerdoSaldado`), no cuenta como incumplimiento (`fraccionIncumplida`) y deja de verse como pendiente para el reparto (`pendiente`, `entregable`, `ladoQueDebe`, urgencia por volumen). La regla es por línea y solo tras un abono (una línea sin tocar nunca se da por saldada, por pequeña que sea). Ejemplo: 32 u con un peaje del 2 % llegan 31,36 y falta 0,64 u: saldado en una sola salida (antes, una segunda de 0,64 u y una tercera de 0,01 u). Una entrega grande sigue necesitando repesca (1.000 u dejan 980 y faltan 20 u). El peaje no cambia (sigue siendo el 2 % de la carga). Implementado en `crates/comercio/src/trueque.rs` (`Linea::saldada`, `UNIDAD`); `abonar_a_trueque` del núcleo no cambia. Sin bloque de parámetros ni cambio de formato de foto.

### 4.5 Entrega manual con una caravana adjunta (`entregarDeCaravana`)

Cuando la caravana va **enganchada a un ejército** (`adjunta`) el reparto automático no la ve y la entrega la decide el jugador. Condiciones: la caravana va con ese ejército; el trueque está `activo` y
**uno de sus lados lo debe la Facción del ejército** con líneas pendientes (`ladoPendienteParaEjercito`; el primer lado que lo cumpla, A antes que B); el ejército está en la **puerta** del asentamiento que **recibe** (`radioPuerta`; el radio de reabastecimiento desaparece, Movimiento_Ejercitos §8). Entrega por línea `min(lo que lleva, lo que falta)` (error `EntregaInvalidaError` si no entrega nada), cobra la **misma** comisión y el mismo recargo por reputación
(§3.3, distancia recta origen→destino; el recargo se descuenta del almacén de origen) y cierra por la misma vía (§4.4). Emite `comercio.entrega_escoltada` (más `trueque_cumplido` si cierra). La caravana **sigue adjunta** y con lo que sobre.

### 4.6 Reputación y experiencia, en una tabla

| Hecho | Reputación | Experiencia de Facción |
|---|---|---|
| Trueque `cumplido` entre Facciones distintas | `+bonusTruequeCumplido` a la Facción de A **y** a la de B | `+xp.truequeCumplido` a cada lado |
| Trueque `activo` que vence entre Facciones distintas | `penalizacionTruequeIncumplido × fraccionIncumplida` por lado | — |
| Cualquiera de los dos entre plazas de la **misma** Facción | nada (la reputación es entre Facciones) | nada |
| Propuesta sin contestar, rechazada, o cancelada por ruina | nada | nada |

`fraccionIncumplida(lado) = 1 − min(Σ entregado por línea, cantidadTotal) / Σ cantidadTotal` (sumando cantidades de recursos distintos sin ponderar por precio).

### 4.7 Poda (`podarAcuerdosTerminados`, fin del tick)

Sale de `estado.acuerdos` el `cumplido`, `expirado` o `rechazado` con `instante > expiraEn + TRUEQUE.retencionTerminadosMinutos` **que ninguna caravana cite** (`origenAcuerdoId`). Devuelve el mismo
array si no hay nada que quitar.

## 5. La flota de caravanas comerciales

### 5.1 Cupos (todos salen del **Mercado activo**, `edificiosPorTipoYEstado(…, 'mercado')[0]`, y valen 0 sin él)

| Cupo | Valor | Se mide |
|---|---|---|
| `cupoCaravanas` | `niveles[nivelInterno].cupoCaravanas` del Mercado **+** `cupoCaravanaExtra` de las políticas (suma; "Ampliación de Flota" +1) | al **crear**: caravanas `comercial` con `origenAsentamientoId == este` en **cualquier estado** |
| `cupoCarros` | `niveles[nivelInterno].cupoCarros` | al **fabricar** un carro: `carrosEnUso(asentamiento)` = Σ carros de las comerciales de ese origen (sin las nacidas en un campamento) |
| `cupoEscolta` | `CARAVANA_ESCOLTA.liderazgoPorNivelMercado[nivelInterno − 1]` | **por caravana**, en puntos de Liderazgo |

(Los valores de cada nivel están en el bloque `EDIFICIO_CATALOGO`, edificio `mercado`, de `Economia_Construccion.md`.) El cupo de carros es de la **flota entera** del asentamiento, repartido como se quiera; mover
carros entre caravanas propias no lo toca.

### 5.2 Crear (`crearCaravana`)

Condiciones, en este orden: **Mercado activo**; `puedeCrearCaravana` (han pasado `CARAVANA_COOLDOWN.cooldownMinutos` desde `ultimaCaravanaCreadaEn`; sin marca previa, sí); cupo de caravanas no alcanzado.
Resultado: un casco **vacío y gratis** `{carros: [], contenido: {}, estado: disponible, reservadaManual: false, posicionActual: la del asentamiento}` y `ultimaCaravanaCreadaEn = ahora`. No hay comando para
desmantelarla: solo se pierde por captura, destrucción o ruina. `construirCaravanaComercial` (NPC de laboratorio, no un comando) encadena crear + 1 carro básico + 1 buey.

### 5.3 Carros y animales

- **`agregarCarroCaravana`**: caravana comercial **`disponible`**; la flota no está en su `cupoCarros`; el `reforzado` exige una **Carpintería activa** en el origen (el `basico` solo el Mercado, que ya se
  exige para crear); el almacén tiene el `costo` (se descuenta; no hay reserva previa). El carro nace **sin animal**.
- **`comprarAnimalCaravana`**: caravana `disponible`; el índice de carro existe y **no tiene animal**; el almacén tiene el `costo` del animal (oro). Cuenta para el contador de logros `animales.comprados`.
- **`moverCarroCaravana`**: las dos comerciales `disponibles`, distintas y **del mismo origen**; el carro (con su animal) pasa de una a otra. Sin coste, tiempo ni comprobación de cupo.
- **`reservarCaravana`**: pone o quita `reservadaManual`. No comprueba estado.

### 5.4 Lo que se deriva de los carros (`engine/caravanas.ts`)

```
carrosConTraccion   = carros con animal
capacidadCaravana   = Σ  CARRO_CATALOGO[tipo].capacidadBase × ANIMAL_CATALOGO[animal].factorCarga       ; 0 sin carros con animal
velocidadCaravana   = min  ANIMAL_CATALOGO[animal].velocidad                                              ; 0 sin carros con animal
esCaravanaGrande    = tipo 'comercial' y |carrosConTraccion| ≥ VISION.caravanaGrande.carros              ; para ser vista (Niebla)
```

Los multiplicadores de política los aplica **quien llama**, siempre con las políticas del asentamiento de **origen**: `factorCapacidadCaravana` ("Carga Ampliada" ×1,5) al **cargar para viajar**
(reparto automático y `prepararCaravana`) y `factorVelocidadCaravana` ("Rutas Rápidas" ×1,5) al **mover** una comercial (ida y vuelta). El multiplicador de capacidad vale para **toda** caravana propia
(decidido el 2026-10-11): `capacidadEfectiva(caravana, asentamientos)` = `capacidadCaravana × factorCapacidadCaravana(origen)` es la que usan el reparto, `prepararCaravana`, la carga de una
caravana **adjunta** (`cargarCaravanaAdjunta`), la de una **aparcada** y `capacidadCargaDe` de un ejército con caravanas (botín, mostrador, víveres). Si el origen ya no existe, sin política.

<!-- parametros:CARRO_CATALOGO -->
```json
{
  "basico": {
    "capacidadBase": 500,
    "costo": {
      "madera": 20
    },
    "fabrica": "mercado"
  },
  "reforzado": {
    "capacidadBase": 800,
    "costo": {
      "madera": 40
    },
    "fabrica": "carpinteria"
  }
}
```
<!-- /parametros -->

<!-- parametros:ANIMAL_CATALOGO -->
```json
{
  "buey": {
    "factorCarga": 1,
    "velocidad": 16,
    "costo": {
      "oro": 12
    }
  },
  "caballo": {
    "factorCarga": 0.5,
    "velocidad": 24,
    "costo": {
      "oro": 60
    }
  },
  "camello": {
    "factorCarga": 0.75,
    "velocidad": 19,
    "costo": {
      "oro": 40
    }
  }
}
```
<!-- /parametros -->

<!-- parametros:CARAVANA_COOLDOWN -->
```json
{
  "cooldownMinutos": 10
}
```
<!-- /parametros -->

### 5.5 Escolta cedida (`asignarEscolta`, `quitarEscolta`, y dentro de `prepararCaravana`)

> **Implementado en Rust (2026-10-20, fase 4B-1).** `AsignarEscolta { caravana, heroe, escuadras }` (61) y `QuitarEscolta { caravana, heroe, escuadras }` (62) usan `comercio::caravana::{seleccionar_escolta, asignar_escolta, quitar_escolta}` con las escuadras reales del héroe (`Contenedor::Escolta(caravana)`), el cupo de `Nucleo::cupos_de_flota` y la autorización del héroe (su cuenta o la administración). Las escuadras vuelven al campamento al llegar la caravana a casa (`Llegada::Casa`), al cancelar la preparación, al perderse (ruina o todos sus carros) y al retirarlas su héroe. **Provisional:** ceder la escolta dentro de `prepararCaravana` no existe (se cede antes con `AsignarEscolta`); la baja de soldados de una escolta derrotada será del combate (4D).

> **Implementado en Rust (2026-10-20, fase 4B-2).** La caravana adjunta ya vive en una columna real (`Ejercito.caravanas_adjuntas`, ids ascendentes). `AdjuntarCaravana` exige un **ejército** o una formación (una columna personal no las lleva: `columna.personal_sin_caravanas`) y manda en ella cualquier participante; `capacidad_de_carga` suma `participantes × capacidadCarroPorJugador` y la de cada adjunta. **Se sueltan solo dentro de una plaza** (`AparcarCaravana`; decidido 2026-10-19). Al **guarnecer**, todas las adjuntas han de ser de esa plaza y quedan aparcadas en ella (`ejercito.caravana_aparcada`; si alguna es de otra plaza, `caravana.ajena_al_lugar` y no se entra); al **entrar en un campamento** no se admite ninguna caravana comercial adjunta. Si la columna **se queda sin nadie** (el último se desconecta), sus caravanas comerciales vuelven solas a su origen por el camino y, si no hay por tierra, se pierden (`ejercito.disuelto`). **Provisional:** la Caravana de Fundación (§1.8) sigue siendo una marca de la columna (`caravana_fundacion`) que da la administración y que `Fundar` consume sin disolver la columna: no se engancha ni se suelta en un campamento ni se reclama (`reclamarCaravana`); tampoco existen `cargarCaravana` ni la derrota de la columna con sus caravanas (4D).

- **Seleccionar** (`seleccionarEscoltaCaravana`): el héroe **reside en el origen** (su campamento está ahí) y cada escuadrón es **suyo**, está **en su campamento** (`contenedor.tipo == 'campamento'`),
  **no** en la guarnición y no está aniquilado.
- **Cupo**: `liderazgoComprometido(ya cedidas ∪ nuevas) ≤ cupoEscolta(origen)`. Cada escuadra suma su coste de Liderazgo por escalón (no depende de cuánta gente le quede) y se cede **entera**. El cupo
  es de la caravana, lo comparten todos los residentes que ceden, y **no gasta el Liderazgo del que presta**.
- **`asignarEscolta`**: caravana `comercial` y `disponible` en su origen, ≥ 1 escuadrón, sin repetidos; los escuadrones quedan `contenedor: escolta`. **`quitarEscolta`**: solo con la caravana `disponible`;
  cada héroe retira únicamente los suyos (los ids por defecto son todos los suyos cedidos a esa caravana); vuelven al campamento.
- Los cede cada residente por sí mismo (`heroeId` = el actor). La escolta **no consume ración**.
- Vuelven al campamento: al terminar el regreso (§6.4), al cancelar la preparación, al perderse la caravana por ruina o por presencia (los casos de combate están en `Combate.md`).

<!-- parametros:CARAVANA_ESCOLTA -->
```json
{
  "liderazgoPorNivelMercado": [
    100,
    200,
    300
  ]
}
```
<!-- /parametros -->

### 5.6 Lanzar a mano (`prepararCaravana`) y cancelar (`cancelarCaravana`)

`prepararCaravanaManual(caravana, origen, destino, carga, escolta, yaCedidas, mapa, territorio, ahora, salirEn?)`. Validaciones, en este orden (cualquiera falla ⇒ `CaravanaInvalidaError`, no se
cambia nada):

1. La caravana es comercial con `carros`; está `disponible`; su origen es `origen`.
2. `capacidad = capacidadCaravana × factorCapacidadCaravana(origen)` es > 0.
3. `Σ carga > 0` y `≤ capacidad + 1e-6`; ninguna cantidad negativa; el almacén del origen tiene cada cantidad (sin reserva previa).
4. Cupo de escolta (§5.5) con lo ya cedido más lo nuevo.
5. Hay **ruta por tierra** (`trazarRutaComercial`, §7.2): sin ella se rechaza (el agua es infranqueable).
6. Si hay `salirEn`: `salirEn > ahora` y `salirEn ≤ ahora + CARAVANA_PREPARACION.maxProgramacionDias` días.

**No se comprueba** nada del destino: puede ser el propio origen, una plaza de otra Facción, de un enemigo, o una ruina que aún no se haya limpiado. Tampoco el cargo ni la tecnología.

Efecto:

```
prepMinutos = CARAVANA_PREPARACION.kPorCarro × max(0, |carros| − 1)          ; cuenta TODOS los carros, con o sin animal
saleEn      = max( ahora + prepMinutos, salirEn si lo hay )
estado      = saleEn > ahora ? 'preparando' : 'en_transito'
almacén origen −= carga                 ; la carga se reserva YA
caravana: destino, contenido (las cantidades > 0), posicionActual = origen, progreso 0, ruta, peajes (si hay), preparaHasta = saleEn (solo si preparando),
          escoltaIds = ya cedidas ∪ nuevas (si hay)
tropa: las nuevas escuadras pasan a contenedor 'escolta'
red = registrarRuta(red, origen, destino, ruta, ahora)         ; la ruta entra en la red YA, aunque luego se cancele
evento comercio.caravana_preparando
```

Salida inmediata (`en_transito` desde el comando): **no** hay evento `comercio.caravana_sale`; su plan empieza en ese instante. Con preparación, el **evento programado** a `preparaHasta` la pasa a
`en_transito`, emite `comercio.caravana_sale` y le calcula el plan desde ese instante. Una caravana `preparando` está en su ciudad: no es interceptable.

**`cancelarCaravana`** (solo `preparando`): la carga vuelve al almacén del origen (`agregarRecurso`, topada por capacidad), la caravana pasa a `disponible` sin destino/contenido/ruta/escolta y se devuelve
la escolta al campamento. La ruta ya registrada en la red **no se retira**.

<!-- parametros:CARAVANA_PREPARACION -->
```json
{
  "kPorCarro": 2,
  "maxProgramacionDias": 3
}
```
<!-- /parametros -->

### 5.7 Caravana aparcada

Una caravana `aparcada` (dejada en una plaza de su Facción al entrar un ejército, `Movimiento_Ejercitos.md` §9) puede:

- **Cargarse/descargarse** (`moverCargaCaravanaAparcada`) contra el almacén de la plaza anfitriona; hay que estar **presente** en ella siendo **residente del origen de la caravana**, y la caravana tiene que estar
  exactamente en la posición de la plaza. `cargar`: `min(pedido, capacidadCaravana − ya cargado, disponible en el almacén)`; `descargar`: `min(pedido, lo que lleva, hueco del almacén de ese recurso)`;
  si el resultado es ≤ 0 se rechaza. No hay reserva.
- **Volver** (`enviarCaravanaAlOrigen`): **vacía** ⇒ pasa a `disponible` **al instante** en su origen (sin viaje). **Con carga** ⇒ `retornando` con `destinoAsentamientoId = la anfitriona`, ruta
  `calcularRuta(anfitriona → origen)` **sin atracción de la red ni peajes**; al llegar vuelca el contenido en el almacén del origen (topado) y queda `disponible`. Sin ruta por tierra ⇒ error.

## 6. El viaje (el plan de la caravana)

Una caravana en viaje es un **plan** (vértices con su instante de paso, [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §4.2) y se mueve por eventos, no por barrido:

1. **`preparando`**: si `preparaHasta` definido e `instante ≥ preparaHasta` → `en_transito` (+ evento `comercio.caravana_sale`); si no, queda. No se mueve en ningún caso.
2. **Sin `destinoAsentamientoId`** (Fundación, `disponible`, `adjunta`, `aparcada`): se deja pasar; la mueve su propio subsistema.
3. **Origen inexistente**, o destino inexistente **y** no está `retornando`: la caravana **desaparece** (sin evento) y su escolta se libera al campamento. (Hoy solo ocurre con partidas guardadas antes de
   `ruina.ts`, que cierra el caso al caer la plaza.)
4. **Plan**: `velocidad = velocidadCaravana × (comercial ? factorVelocidadCaravana(origen) : 1)` (calibrada: buey 24, camello 28, caballo 36); el plan se construye de una vez con las **aristas de la red** (más rápido sobre camino; fórmula en
   `Movimiento_Ejercitos.md` §4.2). El plan se rehace si cambia la velocidad, se construye un camino en el trayecto o se engancha. `distanciaTotal = max(1, distancia recta entre los dos extremos del viaje)`; solo sirve para el bonus de la comisión.
5. **Peajes** (solo a la ida, §7.3): un evento en el instante exacto en que el plan cruza cada zona ajena.
6. Al **evento de llegada** (el último vértice del plan):
   - **`retornando`** ⇒ llegada a casa: el contenido que aún lleve (solo si vino de `enviarCaravanaAlOrigen`) se vuelca en el origen (topado); la caravana queda
     `{estado: disponible, contenido: {}, destino: –, origenAcuerdoId: –, ladoAcuerdo: –, progreso: 0, posicionActual: origen, ruta: –, escoltaIds: –}` y su escolta vuelve al campamento.
   - **`en_transito`** ⇒ **entrega** (§4.4; vale igual sin trueque) y, si es `comercial`, pasa a `retornando` con `contenido {}`, `progreso 0`, `posicionActual = destino` y la **misma ruta invertida**.
     El regreso **desanda el camino de ida**; no se recalcula. Una caravana de otro tipo que llegase aquí **se pierde** al entregar (la de Fundación y la de acarreo tienen su propio subsistema).

Una caravana nunca se teletransporta salvo la vacía enviada a casa (§5.7).

## 7. Rutas: trazado, peajes y red de caminos

### 7.1 El grafo de navegación (`world/grafoNavegacion.ts`) y el A\* (`world/rutas.ts`)

Se calcula **una vez por mundo** y se cachea (no se guarda). Es una rejilla 8-conexa sobre todo el mapa:

```
ladoMayor = max(ancho, alto)
espaciado = max(NAVEGACION.espaciadoMin, round(ladoMayor / NAVEGACION.nodosPorLado))
cols = floor(ancho / espaciado) + 1 ; filas = floor(alto / espaciado) + 1
nodo(col,fila) = fila × cols + col  en (col × espaciado, fila × espaciado)
paso = min(muestreoMax, espaciado / 3)
VECINOS (col,fila) = [1,0] [−1,0] [0,1] [0,−1] [1,1] [−1,−1] [1,−1] [−1,1]       ; el opuesto de la dirección d es d XOR 1
```

**Coste de una arista** (`evaluarTramo(a, b)`): `n = max(1, ceil(largo / paso))`; si **alguno** de los `n + 1` puntos equiespaciados (extremos incluidos) no es transitable (`mapa.esTransitable`: agua,
fuera del mapa) ⇒ `Infinity`. Si no: `coste = (largo / n) × Σ_{k=0..n−1} costeEnPunto(punto en (k+0,5)/n)`. Las 4 aristas de cada nodo (una de cada pareja) se calculan y se copian a su opuesta.

**Ríos**: una arista que corta un cauce **fuera de un vado** queda en `crucesRio` con el punto del cruce y solo se puede usar si ese punto está a `≤ radioPasoCiudad` de la posición de algún asentamiento
(`pasosRio` = todos los asentamientos del mundo). Hay vado en la distancia `s` desde el nacimiento si `s ≤ separacionVados / 2` (arroyo) o `|s − round(s / separacionVados) × separacionVados| ≤ espaciado`.
`separacionVados = max(separacionVadosMin, ladoMayor / vadosPorLado)`; `radioPasoCiudad = max(radioPasoCiudadMin, 1,5 × espaciado)`.

**`calcularRuta(mapa, origen, destino, {pesos?, pasosRio?})`** → polilínea `[origen, …nodos, destino]` o **`undefined`**:

1. Origen y destino deben ser transitables.
2. Si `distancia(origen, destino) < espaciado`: tramo directo (`[origen, destino]`) si su coste es finito y su cruce de río está permitido; si no `undefined`.
3. **Enganches**: del origen a cada nodo de la ventana de 4×4 (`col0−1 … col0+2`, `fila0−1 … fila0+2`, con `col0 = floor(x / espaciado)`) con tramo recto de coste finito y cruce permitido (coste = el del tramo);
   del destino igual, en sentido inverso. Sin enganches en alguno de los dos lados ⇒ `undefined`.
4. A\* sobre los nodos más un nodo virtual `META`. `g` inicial de cada nodo de salida = coste de enganche. Heurística `h(nodo) = distancia(nodo, destino) × costeMinimo`, con `costeMinimo = atraccionMin`
   si hay `pesos` y `1` si no. Al cerrar un nodo con enganche de llegada se relaja `META`. Para cada vecino de coste finito y cruce permitido, `coste' = coste × factorAtraccion(peso de esa arista)` con
   `factorAtraccion(p) = p > 0 ? max(atraccionMin, 1 − atraccionPorPeso × p) : 1`.
5. Se reconstruye desde `META`. Determinista: mismo mundo, puntos y opciones ⇒ misma ruta (el desempate es el orden del montículo binario).

<!-- parametros:NAVEGACION -->
```json
{
  "espaciadoMin": 45,
  "nodosPorLado": 160,
  "muestreoMax": 30,
  "separacionVadosMin": 300,
  "vadosPorLado": 16,
  "radioPasoCiudadMin": 60,
  "atraccionPorPeso": 0.1,
  "atraccionMin": 0.5
}
```
<!-- /parametros -->

<!-- parametros:COSTE_MOVIMIENTO -->
```json
{
  "llano": 1,
  "costa": 1.1,
  "colina": 1.8,
  "montana": 3.5,
  "cima": 12,
  "agua": 15,
  "factorCamino": 0.5,
  "bosquePorDensidad": 2
}
```
<!-- /parametros -->

### 7.2 Trazado de una ruta comercial (`trazarRutaComercial`)

Se usa en **todo** lanzamiento de una comercial (reparto y manual).

```
opciones = { pesos: pesosDeRed(red), pasosRio: posiciones de todos los asentamientos }
pendientes = zonas de influencia de asentamientos  b  con  b ≠ origen,  b ≠ destino,  b.faccionId ≠ origen.faccionId     ; "ajenas" incluye aliadas y vasallas
ruta = [origen] ; paradas = [] ; actual = origen
bucle:
    tramo = calcularRuta(actual → destino, opciones)           ; si undefined → TODA la ruta es undefined (no sale)
    zona = el PRIMER punto de tramo[1..] que cae dentro de alguna zona pendiente  (la primera zona de `pendientes` que lo contiene)
    ciudad = asentamiento de esa zona
    hastaCiudad = calcularRuta(actual → ciudad)
    si no hay zona, o no hay hastaCiudad :  ruta += tramo[1..] ; fin
    ruta += hastaCiudad[1..] ; paradas += { ciudad, distancia acumulada de la ruta } ; quitar la zona de pendientes ; actual = ciudad
peajes = paradas.map( { asentamientoId, progreso = distancia acumulada / longitud TOTAL de la ruta final } )
```

El trazado **no vuelve a examinar** el tramo hasta la ciudad: si roza otra zona ajena antes de llegar, no para en ella. Una ciudad se visita **una vez**.

### 7.3 Peaje (en `avanzarCaravanas`, solo a la ida)

Tras mover, para cada `peajes[i]` con `progreso_i ≤ progreso` actual (en orden): `cobrado[r] = contenido[r] × PEAJE_PASO.tasa` (del contenido **de ese momento**, ya reducido por peajes anteriores), solo
`> 0`. Si no hay nada que cobrar se salta. Se suma al almacén de esa ciudad (`agregarRecurso`, topado) y se resta del contenido; evento `comercio.peaje_paso` (con `asentamientoId` = la ciudad). Después se
descartan de `peajes` todos los ya alcanzados, se haya cobrado o no. **La ciudad no se detiene a nadie**: la caravana sigue de largo. El regreso va sin peajes (`peajes` solo se aplica a la ida) y vacío.

<!-- parametros:PEAJE_PASO -->
```json
{
  "tasa": 0.02
}
```
<!-- /parametros -->

### 7.4 La red de caminos (`engine/redCaminos.ts`)

- **`registrarRuta`** (en cada lanzamiento): `id = "{origen}>{destino}"`; `aristas = aristasDeTrazado(ruta)` (las claves de cada par de puntos consecutivos, sin repetir); `ultimoLanzamiento = ahora`. La ruta
  anterior del mismo par se **sustituye**; las aristas nuevas se añaden a `red.aristas`; las que la ruta nueva ya no pisa **siguen** en la red (como sendero).
- **`pesosDeRed`** = para cada arista, cuántas rutas vigentes la contienen (las de peso 0 no aparecen). **`aristasDeRed`** = todas las aristas, con o sin peso: sobre cualquiera la caravana avanza más rápido.
- **`podarRutas`** (cuando se registra o se usa una ruta, y como evento periódico de baja frecuencia): quita las rutas con `ahora − ultimoLanzamiento ≥ caducidadMinutos` o con un extremo que ya no existe. Devuelve la **misma** red si no cambia
  nada (los derivados están cacheados por identidad del objeto).
- **`escalonDePeso(p)`**: `p ≥ escalonCalzada → 2` (calzada); `p ≥ escalonCamino → 1` (camino); si no `0` (sendero). `tramosDeRed(red, visible)` encadena aristas contiguas del mismo escalón en polilíneas
  (solo por nodos con exactamente 2 aristas de ese escalón) para pintarlas; `visible(a, b)` filtra por niebla (`Niebla_De_Guerra.md`).
- **`caminoCompartidoAbierto(red, zonas)`**: ¿hay una arista, con su **punto medio fuera de toda zona**, que pisen ≥ `logroRutas` rutas de ≥ `logroFacciones` Facciones? Es la consulta del logro
  `logistica_campana` (Era IV, aún sin código que la llame; ver `Tecnologia_Aedas.md`).

<!-- parametros:RED_CAMINOS -->
```json
{
  "caducidadMinutos": 4320,
  "escalonCamino": 2,
  "escalonCalzada": 6,
  "logroRutas": 10,
  "logroFacciones": 3
}
```
<!-- /parametros -->

## 8. El mercado de una plaza

### 8.1 Colocar una orden (`colocarOrdenMercado`)

Condiciones: el actor **reside** en el asentamiento; el asentamiento existe y tiene **Mercado activo**; `cantidad > 0`; si es `venta`, el almacén tiene hoy `≥ cantidad` de ese recurso. Resultado:
`{id, tipo, recurso, cantidad, cantidadCumplida: 0, precioUnitario: precio dado ?? calcularPrecioReferencia, expiraEn: ahora + MERCADO.plazoOrdenMinutos, estado: activa}`.

Lo que **no** hace: no reserva nada (ni la mercancía de una venta ni el oro de una compra), no valida el precio dado (cero o negativo se acepta y luego no se puede liquidar), no limita cuántas órdenes
tiene una plaza, y no impide ordenar un recurso sin precio de referencia (el oro). Todo eso se comprueba **al liquidar**.

### 8.2 Caducidad e historial

En su instante de caducidad (**evento**): una orden `activa` con `instante ≥ expiraEn` pasa a `expirada` (evento `mercado.orden_expirada`); las que ya no están `activa` salen de `estado.ordenes`. Las cerradas
se anexan a `historialOrdenes` (`anexarAlHistorialDeOrdenes`), que recuerda solo las **últimas `MERCADO.historialPorPlaza` de cada plaza**, del más viejo al más nuevo. No existe emparejamiento automático:
nada cierra una orden salvo el mostrador o el plazo.

### 8.3 El mostrador (`comerciarEnPlaza` → `comerciarEnPlaza` del motor)

Un jugador toma una orden **en persona**. Condiciones (cada una es un `OrdenInvalidaError`): el actor es el **Líder** de la columna (`exigirColumnaDe(heroeId)`, la suya); la orden es de esa plaza, está
`activa` y no ha caducado (`instante < expiraEn`; se busca también en el historial para dar el error correcto); la columna está **en la puerta** de la plaza
(`distancia ≤ MOVIMIENTO.radioPuerta`); la plaza sigue teniendo Mercado activo; `cantidad > 0`; `precioUnitario > 0`.

```
tasa            = misma Facción ? tasaMismaFaccion : tasaExterna × factorComisionExterna(plaza)        ; sin factor de reputación ni bonus
pendiente       = orden.cantidad − cantidadCumplida
espacioCarro    = max(0, capacidadCargaDe(columna, caravanas) − Σ suministro)
precioNeto      = venta ? precio × (1 + tasa) : precio × (1 − tasa)                                    ; comprando se paga de más, vendiendo se cobra de menos
crecimiento/u   = venta ? 1 − precioNeto : precioNeto − 1                                              ; cuánto crece el carro por unidad
topeCarro       = crecimiento/u > 0 ? espacioCarro / crecimiento/u : ∞
si la plaza VENDE :  cantidad = min(pedida, pendiente, almacén plaza[recurso], oro del carro / precioNeto, topeCarro)
si la plaza COMPRA:  cantidad = min(pedida, pendiente, carro[recurso], hueco del almacén de la plaza para ese recurso, almacén plaza[oro] / precioNeto, topeCarro)
si cantidad ≤ 0 → error (no se devuelve "0 servido")
valor = cantidad × precio ; comisión = valor × tasa ; oroMovido = cantidad × precioNeto
VENDE : carro.oro −= oroMovido ; carro[recurso] += cantidad ; plaza[recurso] −= cantidad ; plaza.oro += oroMovido
COMPRA: carro[recurso] −= cantidad ; carro.oro += oroMovido ; plaza.oro −= oroMovido ; plaza[recurso] += cantidad
las entradas a 0 se borran del carro ; cantidadCumplida += cantidad ; estado = cumplida si cantidadCumplida ≥ cantidad
```

El **oro se conserva**: lo que la comisión quita a quien comercia se queda en la plaza (es lo que cuesta usar el mercado de otro). El oro ocupa carro como cualquier recurso. Una orden `cumplida` sale de
`estado.ordenes` hacia el historial. El comando emite `mercado.comercio_en_plaza` con `entreFacciones` = el héroe es ciudadano de alguna Facción y **no** de la de la plaza (un héroe sin Facción cuenta como no
`entreFacciones`). (El motor no emite evento propio: lo emite el comando.)

Quién lo puede hacer: cualquier héroe (`rolesPermitidos: ['jugador']`) en su propio nombre; no exige residir ni estar dentro, solo estar en la puerta con su columna.

### 8.4 Qué se ve

La proyección del jugador incluye: los `acuerdos` donde **alguno de los dos lados es propio**; las `ordenes` **propias** (todas, también las cumplidas) más, de una plaza **ajena en cuya puerta tiene una
columna**, solo las **`activa`**; el `historialOrdenes` propio; y los `caminos` por **explorado** (`Niebla_De_Guerra.md`). Las caravanas ajenas y su visibilidad están en `Niebla_De_Guerra.md`.

<!-- parametros:MERCADO -->
```json
{
  "historialPorPlaza": 200,
  "plazoOrdenMinutos": 200
}
```
<!-- /parametros -->

## 9. El mercado de un campamento de mercenarios

### 9.1 Qué vende y a cuánto

Solo **vende**. En venta: los bienes con `PRECIO_BASE` (`BIENES_EN_VENTA`); el oro no. Precio unitario = `calcularPrecioReferencia(recurso) × MERCENARIOS.mercado.margen` (parámetros al final de esta sección; también en `Tecnologia_Aedas.md`). Un bien sin precio no se vende. El **oro cobrado se destruye** (sumidero).

Un campamento nace con `stockInicial` de cada bien en venta y, **sobre eso**, sus **pilas** propias (`madera`, `piedra`, `trigo`; las pilas sustituyen al stock inicial de esos tres).

### 9.2 Comprar (`comprarEnCampamento`)

El héroe tiene que estar **en el campamento** (dentro, o con su columna en la puerta: `MOVIMIENTO.radioPuerta`). Cantidades **enteras**: pedir menos de 1, o pedir oro, se rechaza.

- **Reside en él** ⇒ compra cualquier bien en venta; lo comprado va a su **almacén personal**; con **cupo diario** por héroe en `cupoDiario` (madera y piedra; el día es `floor(instante / 86 400 000)`).
- **No reside** ⇒ compra **cualquier bien en venta desde la puerta, salvo el trigo** (cobre, estaño, madera… decidido el 2026-10-11; el trigo se reposta gratis a víveres con `repostarEnCampamento` y nunca va al carro: pedirlo es un error), al mismo precio, **pagando con el oro de su carro** y llevándoselo **al carro de su
  columna** (hace falta la columna); **sin cupo diario**, pero compite por el mismo stock. No toca su almacén personal.

**Residente:**

```
coste(n)   = ceil(n × precio)
guardado   = almacén personal.oro ;  delAlmacén(n) = coste(n)                        ; ya no hay oro de botín aparte (2026-10-11)
cantidad   = min( floor(pedida), floor(stock del campamento), cupo restante hoy, floor(guardado / precio) )
sitio      : ALMACEN_PERSONAL.capacidad − total del almacén personal
mientras  cantidad − delAlmacén(cantidad) > sitio : cantidad−−       ; en el almacén cada unidad ocupa 1 y libera lo que cuesta en oro
mientras  coste(cantidad) > guardado : cantidad−−
si cantidad ≤ 0 → error con el motivo (sin stock / cupo agotado / sin oro / almacén lleno)
oro cobrado = coste(cantidad) ; stock −= cantidad
```

<!-- parametros:ALMACEN_PERSONAL -->
```json
{
  "capacidad": 1000
}
```
<!-- /parametros -->

**No residente (de visita):**

```
oroCarro   = columna.suministro.oro
libre      = capacidadCargaDe(columna, caravanas, asentamientos) − Σ suministro      ; con las caravanas adjuntas y «Carga Ampliada»
cantidad   = min( floor(pedida), floor(stock), floor(oroCarro / precio) )                ; sin cupo
mientras  cantidad − coste(cantidad) > libre : cantidad−−       ; cada unidad cambia coste de oro por una de mercancía: el carro solo crece si el precio es < 1
mientras  coste(cantidad) > oroCarro : cantidad−−
si cantidad ≤ 0 → error (sin stock / sin oro en el carro / carro lleno)
carro.oro −= coste(cantidad) ; carro[recurso] += cantidad ; stock −= cantidad          ; el oro cobrado se destruye
```

### 9.2b Repostar gratis (`repostarEnCampamento`)

Cualquier héroe, residente o no, en un campamento (dentro, o con su columna a la puerta) llena sus **víveres** hasta `LOGISTICA.capacidadViveresPorHeroe`, **gratis** y **sin gastar el stock** del mercado; falla si ya los tiene llenos. El trigo no pasa por el carro ni por el almacén personal.

Emite `mercenarios.repostado { campamentoId, heroeId, cantidad }` (la cantidad cargada), sin plaza.

### 9.3 Cómo se repone

- **Qué cuenta como comercio del mundo** (`comerciadoDeEventos`, leído de los eventos de cada comando en `exito`): el `intercambiado` (por recurso) de cada `comercio.trueque_cumplido` con
  `entreFacciones`, y la `cantidad` de cada `mercado.comercio_en_plaza` con `entreFacciones`. Se suma a `mercadoMercenario.contadores`. Lo que una Facción comercia consigo misma no cuenta.
- **Reposición** (`reponerMercados`, evento agendado): si `instante ≥ reponeEn`: para cada campamento, el stock arranca como el suyo **con las pilas devueltas a su tope**, y a cada bien con contador
  `> 0` y que **no sea una pila** se le suma `contador × proporcionRepone`, topado por `topePorBien`. Después los contadores vuelven a `{}` y `reponeEn = instante + cadaHoras × 60` min. Es una cita
  agendada: los demás ticks solo comparan un instante. El primer `reponeEn` es `creación + cadaHoras` horas.

Las cifras de `MERCENARIOS.mercado` (`cadaHoras`, `proporcionRepone`, `topePorBien`, `stockInicial`, `pilas`, `cupoDiario`, `margen`) están en el bloque `MERCENARIOS` de `Tecnologia_Aedas.md`.

## 10. Cuando una plaza cae en ruinas (`cerrarDependientesDeRuina`)

> **Cambio de dueño por conquista (decidido el 2026-10-18; antes solo estaba escrito el caso de ruina).** La **flota** y los **pueblos extractores** de la plaza pasan al conquistador junto con su zona. Los trueques con la plaza conquistada se **anulan sin penalizar a nadie** (como en la ruina) y las caravanas que iban hacia ella **regresan con su carga**; las **aparcadas** en ella **se destruyen** (decidido el 2026-10-19). La fusión o anexión de Facciones a mitad de trueque queda por decidir con la política.

En el instante del evento de ruina (antes de cualquier evento de comercio del mismo instante, prioridad 5 frente a 6), para la plaza `R` destruida:

- Su **campamento de bandidos** (el que nació de esa plaza, `asentamientoId = R`) desaparece con el evento `bandidos.campamento_disperso { campamentoId, asentamientoArruinadoId }`.
- Sus **trueques** `activo` o `propuesto` pasan a `expirado` **sin penalizar a nadie** (evento `comercio.trueque_cancelado` a la plaza socia).
- Las caravanas, salvo las `construccion` y las `adjunta` (siguen su propio camino):
  - **Salían de `R`** ⇒ se **pierden** (`comercio.caravana_perdida`), su escolta vuelve al campamento.
  - **Iban hacia `R`** (`en_transito` o `preparando`) o **estaban `aparcada`** en ella:
    - `preparando` ⇒ la carga vuelve al almacén del origen y queda `disponible` (`comercio.caravana_vuelve`).
    - en camino ⇒ `retornando` con plan `calcularRuta(posición actual → origen)` (**sin atracción**) y `destino = R`, con su carga (la vuelca en el origen al llegar); sin camino por tierra, o sin
      origen, se pierde.
    - **aparcada en `R`** ⇒ **se destruye** con sus carros, animales y carga, sin captura (decidido el 2026-10-13, Doc 3.13.7; **también en la conquista**, decidido el 2026-10-19).
  - Las demás siguen como estaban.

La red de caminos se poda sola al faltar un extremo (§7.4).

## 11. Quién puede qué (`autorizacion.ts`)

| Comando | Quién |
|---|---|
| `proponerTrueque` | **Tesorero o Gobernador** del asentamiento A, residente y dentro (decidido 2026-10-12; antes bastaba residir) |
| `aceptarTrueque`, `rechazarTrueque` | Tesorero o Gobernador del asentamiento B, residente y dentro |
| `colocarOrdenMercado` | Tesorero o Gobernador del asentamiento, residente y dentro |
| `crearCaravana` | residente del asentamiento (las caravanas siguen abiertas a cualquier residente) |
| `agregarCarroCaravana`, `comprarAnimalCaravana`, `reservarCaravana`, `cancelarCaravana`, `moverCarroCaravana` | residente del **origen** de la caravana |
| `prepararCaravana`, `asignarEscolta`, `quitarEscolta` | el actor es el `heroeId` del comando **y** reside en el origen |
| `moverCargaCaravanaAparcada`, `enviarCaravanaAlOrigen` | el actor es el `heroeId`; residente del **origen** de la caravana y **presente** en la plaza que la hospeda |
| `comerciarEnPlaza` | el actor es el `heroeId`; el motor exige columna propia en la puerta |
| `comprarEnCampamento`, `repostarEnCampamento` | cualquier héroe; el motor exige estar en el campamento |

Un id inexistente se deja pasar a propósito para que el comando devuelva el error específico, no un «sin permiso».

## 12. Eventos y quién los consume

| Evento | Quién lo emite | Quién lo lee |
|---|---|---|
| `comercio.trueque_propuesto`, `_aceptado`, `_rechazado` | comandos | narración |
| `comercio.trueque_cumplido` | entrega (§4.4) | mercado de mercenarios (`entreFacciones`); xp y reputación se aplican en el mismo sitio |
| `comercio.trueque_expirado` | reparto (§4.3) | narración |
| `comercio.trueque_cancelado` | ruina (§10) | narración |
| `comercio.caravana_construida`, `_carro_agregado`, `_carro_movido`, `_reserva`, `_preparando`, `_cancelada`, `_escolta_cedida`, `_escolta_retirada`, `carga_caravana_aparcada`, `caravana_enviada_al_origen` | comandos | narración |
| `comercio.caravana_animal_comprado` | `comprarAnimalCaravana` | contador de logros `animales.comprados` (`Tecnologia_Aedas.md`) |
| `comercio.caravana_sale`, `peaje_paso`, `caravana_llega` (esta lleva `comision` y `recargo`) | tick | `caravana_llega`: hecho `caravana` de las épicas (origen = plaza que cuenta; `comercial` con ≥ `AEDAS.epica.cargaMinimaCaravana` de carga) |
| `comercio.entrega_escoltada` | `entregarDeCaravana` | narración |
| `comercio.caravana_perdida`, `comercio.caravana_vuelve` | ruina (§10) y presencia (`Movimiento_Ejercitos.md`) | narración |
| `mercado.orden_colocada`, `mercado.orden_expirada` | comando / tick | narración |
| `mercado.comercio_en_plaza` | comando | mercado de mercenarios (`entreFacciones`) |
| `mercenarios.comprado` | `comprarEnCampamento` | narración |

## 13. Parámetros

**Reputación (decidido el 2026-10-18):** el decaimiento baja de 0,2 a **0,002 por minuto** (placeholder a calibrar): con 0,2 un +5 o un −8 se borraban en 25-40 min y el umbral de −40 casi no se alcanzaba, contra el «decae lentamente» del canon (Doc 2.7). El bonus por minuto de alianza se reduce en la misma proporción (0,05 → 0,0005) para conservar la regla «por sí solo no sube la reputación por encima de ~0,05»: el bonus sigue por debajo del decaimiento (0,0005 < 0,002).

Los bloques de las secciones anteriores son los de este sistema. Además, de otros documentos: `REPUTACION` (también en §4.6) y `NIVEL_FACCION.xp.truequeCumplido` (`Economia_Construccion.md`), `LOGISTICA`
(`Movimiento_Ejercitos.md`), `MERCENARIOS` (`Tecnologia_Aedas.md`), `POLITICA_CATALOGO` y `EDIFICIO_CATALOGO` (`Economia_Construccion.md`).

<!-- parametros:REPUTACION -->
```json
{
  "decaimientoPorMinuto": 0.002,
  "bonusTruequeCumplido": 5,
  "penalizacionTruequeIncumplido": -8,
  "bonusLiberarVasalloVoluntario": 6,
  "penalizacionRebelionParaSenora": -10,
  "penalizacionRomperAlianza": -12,
  "penalizacionAtacarAliado": -25,
  "bonusDefenderVasallo": 15,
  "penalizacionNoDefenderVasallo": -3,
  "penalizacionGuerraTrasRomperAlianza": -15,
  "ventanaGuerraTrasRomperAlianzaDias": 3,
  "bonusPorMinutoAlianzaActiva": 0.0005,
  "umbralBajoParaEmbajador": -40,
  "umbralBajoParaComision": -40,
  "factorComisionPorReputacionBaja": 1.5
}
```
<!-- /parametros -->

<!-- parametros:CARAVANA_CATALOGO -->
```json
{
  "militar": {
    "capacidad": 40,
    "velocidad": 12
  },
  "construccion": {
    "capacidad": 150,
    "velocidad": 10
  },
  "contrabando": {
    "capacidad": 20,
    "velocidad": 24
  }
}
```
<!-- /parametros -->

(`CARAVANA_CATALOGO` es el catálogo de las categorías **sin flota propia**: `construccion` la usa la Caravana de Fundación; `militar` y `contrabando` son solo datos sin ningún disparador.)

## 14. Invariantes

| Invariante | Test que lo vigila |
|---|---|
| Un trueque tiene ≥ 1 línea por lado, cantidades > 0 y sin recurso repetido en un lado | `truequeCompuesto.test.ts` («rechaza un lado vacío…») |
| Cada recurso entregado se abona a su línea; el acuerdo se cierra solo con todas saldadas | `truequeCompuesto.test.ts` («abonos y cierre») |
| Un cargamento por lado: líneas en orden hasta llenar la caravana; sin stock de una línea sale lo de las demás | `truequeCompuesto.test.ts` («un cargamento por lado») |
| Vencer sin cumplir penaliza en proporción a lo no entregado | `truequeCompuesto.test.ts` («vencer sin cumplir») |
| La ruta de una caravana empieza en **su** origen, en los dos sentidos | `caravanas.test.ts` («orientación de la ruta…») |
| El cooldown de creación es compartido y se respeta | `caravanas.test.ts` («cooldown de creación…») |
| Una comercial entregada pasa a `retornando`, avanza de verdad y solo al llegar queda `disponible`; no se reasigna mientras vuelve | `caravanas.test.ts` («retorno real…») |
| La misma caravana sirve varios envíos del mismo trueque | `caravanas.test.ts` («reuso de caravana propia…») |
| Capacidad = Σ por carro con animal; velocidad = animal más lento; sin animal no viaja | `caravanas.test.ts` («derivación…») |
| El cupo de carros es de la flota; el reforzado exige Carpintería; el animal exige carro libre | `caravanas.test.ts` («composición por piezas») |
| Lanzar a mano: preparación, salida programada, cancelación, rechazos | `caravanas.test.ts` («lanzamiento manual») |
| Escolta: cupo de Liderazgo por Mercado, por escalón, de la caravana; ceder/quitar solo parada | `caravanas.test.ts` («escolta sin héroe») |
| El mostrador: la plaza paga/cobra hasta el último gramo, sirve lo que puede, el carro es tope, hay que estar allí y ser el Líder, una orden sin precio no se liquida | `mostrador.test.ts` |
| Las órdenes caducan y las cumplidas salen sin evento | `mostrador.test.ts` («las ofertas que nadie toma…») |
| El historial recuerda solo las últimas por plaza | `historialOrdenes.test.ts` |
| La poda quita lo terminado viejo salvo lo citado por una caravana | `podaAcuerdos.test.ts` |
| Registrar una ruta mueve el peso pero deja las aristas; dos pares comparten arista; podar es idempotente | `redCaminos.test.ts` |
| Paso forzado por una ciudad ajena, peaje, ciudad propia sin peaje | `redCaminos.test.ts` («paso forzado…») |
| El mercado de mercenarios nace lleno salvo el oro, solo cuenta el comercio entre Facciones, repone con tope y pilas | `mercadoMercenario.test.ts` |
| El A\* rodea agua y obstáculos, nunca cruza el mar, cruza ríos solo por vado o ciudad, y se arrima a la red | `world/__tests__/rutas.test.ts` |
| Cada evento del comercio sale con su código | `eventosDominioComercio.test.ts` |
| Los dependientes de una ruina se cierran sin penalizar | `ruinaDependientes.test.ts` |
| Llevarse la carga al propio origen no acuña; la mala reputación paga el extra del almacén de origen (solo entre Facciones, sin pasarse del oro que tenga) | `comercioDecisiones.test.ts` |
| La reputación y la experiencia del trueque son entre Facciones (cumplir y vencer) | `comercioDecisiones.test.ts` |
| Un casco sin carros no bloquea el reparto; un destino inalcanzable no gasta la caravana | `comercioDecisiones.test.ts` |
| «Carga Ampliada» vale para la caravana aparcada (y, con la misma función, adjunta y de ejército) | `comercioDecisiones.test.ts` |

## 15. Casos límite y decisiones que parecen raras

- **El recargo por reputación solo cuenta entre Facciones y paga lo que haya**: con el almacén de origen sin oro, el castigo se queda en lo que tenga (nunca negativo). El mostrador no lo lleva.
- **Lo que no cabe en el almacén del destino se pierde, y el trueque lo da por entregado.** `agregarRecurso` topa por capacidad; `aplicarEntregaATrueque` abona lo que la caravana llevaba, no lo que entró.
  La comisión en oro se topa igual por la capacidad de oro.
- **Una caravana que llega después de vencer el acuerdo** sigue abonando líneas (no mira el `estado`): si con ello se saldan los dos lados, el acuerdo `expirado` pasa a `cumplido` y reparte la
  recompensa, aunque ya se hubiera penalizado el vencimiento (decisión del 2026-10-11: se deja así). El orden del tick (mover antes de caducar) permite que la que llega justo el tick del vencimiento cierre bien.
- **Una manual puede llevar la carga a cualquier sitio** (otra Facción, un enemigo…): solo se mira que no sea el propio origen para cobrar comisión (§3.3). Regalar mercancía por esta vía es posible.
- **El regreso desanda la ida entera, incluido un rodeo que ya no hace falta**, y no cobra peajes.
- **Un peaje se calcula sobre lo que lleva en ese momento**: dos ciudades seguidas cobran el 2 % cada una sobre lo que queda tras la primera.
- **Las órdenes no reservan nada.** Una orden de venta puede quedarse sin mercancía y una de compra sin oro; el mostrador sirve lo que haya o falla.
- **Una orden con precio 0 o negativo se coloca pero no se liquida** (`precioUnitario ≤ 0`), igual que cualquiera sobre el oro cuando no se da precio.
- **`prepararCaravana` registra la ruta en la red al lanzar, no al salir**: cancelar deja el camino.
- **Los trueques entre plazas de la misma Facción no tocan la reputación, la experiencia ni el mercado de mercenarios** (decisión del 2026-09-27 para los dos últimos; del 2026-10-11 para la reputación). Sí pagan la comisión del 3 %.
- **El oro vale 0 al calcular la comisión** (`PRECIO_BASE` no lo tiene): un trueque de oro por oro no acuña nada.
- **El `distanciaTotal` del bonus es en línea recta**, no la longitud de la ruta ni los peajes.

## 16. Divergencias

Ninguna abierta. Las del 2026-10-11 se resolvieron: comisión solo entre asentamientos distintos, recargo por reputación, reparto sin atascos, «Carga Ampliada» para toda caravana, reputación solo entre Facciones, código muerto retirado.

## 12. Carro de acarreo (guerra logística, Doc 3.13.8)

> Implementado en Rust el 2026-10-19 (fase 3C): véase la nota del principio del documento.

Cada pueblo extractor (Economia_Construccion §pueblos) tiene un carro propio: una caravana de tipo `acarreo` ligada al pueblo.

- **Un carro básico con buey** (500 de capacidad, velocidad **24**) y una **ruta fija** pueblo ↔ plaza madre, calculada una vez como cualquier plan.
- **Solo sale con el carro lleno**: el evento de salida se programa cuando el almacén local del pueblo alcanza la capacidad del carro (o al reponerse el carro con almacén ya lleno). Vuelca en el almacén de la madre y vuelve vacío.
- **No cuenta** contra los cupos de caravanas ni de carros del Mercado, ni exige Mercado. No entra en el reparto automático ni en el trueque.
- **Escolta**: sin héroe, de los residentes de la madre (cupo del Mercado de la madre si lo tiene, 100/200/300; **100** si no) o por ejército (Movimiento §9).
- **No le afectan** «Carga Ampliada» ni «Rutas Rápidas» (decidido el 2026-10-18): solo mueven a las caravanas comerciales (§6.4); con ×1,5 de capacidad el carro de 750 superaría el almacén de 500 del pueblo y «sale lleno» no se dispararía nunca.
- **Casi siempre lucha en B** (viaja dentro de la zona del dueño); si una frontera recorta la zona y la ruta sale de ella, ese tramo es A (Combate §7.1).
- **Si se destruye**, se compra otro en la plaza madre solo si el pueblo no tiene ninguno, al coste de carro + buey (20 madera + ~12 oro). Mientras no tenga carro, el pueblo acumula hasta llenar el almacén y la extracción para.
- La **solvencia** de la madre (Doc 4.5) cuenta lo que llega por acarreo como la media de las llegadas en 24 h o como la producción del pueblo.

Valores calibrados (2026-10-12): `COMISION.distanciaParaBonusMax` y `ASIGNACION_CARAVANA.distanciaReferencia` 600 → **120** (los bloques de arriba son los de TypeScript); velocidades ×1,5; `NAVEGACION.espaciadoMin` 45 → **8** y vados cada 100.
