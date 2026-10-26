# Contraste y plan del sistema 6 (tecnología y Aedas), fase 6

Preparado el 2026-10-24, solo lectura, sobre `claude/lucid-curran-6d6ffa` (HEAD `4377b86`, foto `BAN25`, 145 órdenes del contrato). Revisado el mismo día sobre `94cf671`
(calibración del sistema 4: líneas de `combates.rs`, `batallas.rs` y de P puestas al día, D18 nuevo) y con las **decisiones del autor** (al final), que ya ajustan las tandas de §5.
El TypeScript se leyó con `git show typescript-final:src/...` y `git grep ... typescript-final`.
Abreviaturas: G = `Docs/Game/`, M = `Docs/Mechanics/`, P = `Docs/Mecanicas a desarrollar.md` (sección «Paridad con TypeScript» salvo que se diga otra), TS = `typescript-final:src/`.
G6 = `G/6_Sistema_de_Tecnologia_y_Aedas.md`, TA = `M/Tecnologia_Aedas.md`, Gen = `M/Generacion_Del_Mundo.md`, Proy = `M/Proyecciones_Y_Autorizacion.md`, Reloj = `M/Reloj_Y_Eventos.md`,
S5 = `Docs/Arquitectura/Sistema5_Contraste_y_Plan.md` (en el árbol principal), BA-006 = `Consideraciones/BA-006_Revision_Tecnologia_Eras.md`.

Nota de partida: TA se reescribió el 2026-10-13 para el mundo reactivo, pero su cuerpo sigue siendo el TS («en cada evaluación», «muestreo de baja frecuencia»). Aquí se fija qué
dispara cada regla sin tick, qué se reutiliza de lo que ya tiene Rust (los ganchos de la economía están puestos y vacíos) y qué depende del sistema 5. **Las Eras IV y V siguen fuera**
(P §20, decisión del 2026-10-05): el código se escribe por datos, así que añadirlas será escribir sus bloques.

---------------------------------------------------------------------------------------------------------------------------------------

## 1. Inventario

### 1.1 El TypeScript del sistema 6 (el objetivo de paridad)

| Fichero (TS) | Líneas | Qué contiene | En Rust hoy |
|---|---:|---|---|
| `engine/tecnologia.ts` | 511 | `EstadoTecnologia`, de arranque, `tecnologiasDe`, contadores (`contadoresDeEventos`, `…DeProduccion`, `…DeReclutamiento`, peso 5 de `carros_guerra`), `cumpleCondicion` (5 tipos de hito), `avanzarTecnologia` (récords, logros, Era, aparición, crónica de descubrimientos), `hacerAparecer`, `adoptarTecnologia`, `conocidaPorAedas`, `revelarTecnologias`, `precioDeVenta`, `venderTecnologia` | solo «¿la tiene adoptada?» (`estado_militar.rs:222-225`) sobre lo que da la administración |
| `engine/aedas.ts` | 116 | itinerantes: objetivo (`max(3, ⌈vivas/3⌉)`), nacimiento repartido por id, avanzar por ruta, estancia, `partir` (frescas por distancia, `calcularRuta` con pesos de la red), puerta `aedas`, revelar parado | no |
| `engine/aedasResidentes.ts` | 326 | cupo (Palacio activo ∧ nobleza > 0, `cupoPorNivel`), llegada (24 h, ×3 con reputación ≤ −50, medida al comparar), salida del último, conquista (pierde la épica), `empezarEpica`, `abandonarEpica`, `hechosDeEventos` (7 tipos y sus claves), `avanzarEpicas` (enfriamiento, claves, capítulos, `hacerAparecer`) | no |
| `engine/cronica.ts` | 122 | públicos tal cual + `cronica.entrada` derivada | **hecha** la derivada sin diplomacia (`nucleo/cronica.rs`, 4H T5) |
| `engine/alijos.ts` | 70 | colocar (generador), `buscaAlijos`, `alijosALaVista`, `abrirAlijo` | colocar: lo hace el generador del artefacto (`generador/src/contenidoVivo.ts`); abrir y ver, no |
| `session/comandos/tecnologia.ts` | 59 | las 4 órdenes de tecnología y Aedas | ninguna (`sesiones/ordenes.rs:635-639`, `orden.no_disponible`) |
| `session/comandos/mercenarios.ts` (parte) | ≈ 15 | `abrirAlijo` | no |
| `engine/reclutamientoMercenario.ts` (parte) | ≈ 15 | `tecnologiasDelCampamento` (de arranque; `primeros` + 72 h **o** ≥ 50 % de las vivas con ella adoptada) | parcial: solo el porcentaje y sin exigir `primeros` (`nucleo/mercenarios.rs:137-145`) |
| `session/comandos/tipos.ts` (`exito`) | ≈ 20 | en cada orden y tick: épicas con los hechos, crónica, contadores de eventos | la crónica en el cierre de versión (`nucleo.rs:548-550`) |
| `session/resultadoBatalla.ts` (`contadoresDeBatalla`) | ≈ 20 | contadores de una batalla de Unity | no (P l.108) |
| `engine/simulation.ts` (parte) | ≈ 40 | contadores de producción, Aedas residentes en la felicidad y la nobleza, orden del tick (tecnología → itinerantes → residentes) | no |
| `session/proyecciones/jugador.ts` (parte) | ≈ 120 | `tecnologiaParaJugador` (Era, logros por contador/umbral/fecha, propias, reveladas con descubridora y `cumplidas[i]`, Era de cada una, capital, épicas posibles), `AedaAvistado`, `AedaResidenteProyectado`, alijos a la vista | solo `adoptadas` (`proyeccion/propios.rs:441-447`) |
| `engine/trasladoDeFaccion.ts`, `fusion.ts`, `titulos.ts` (parte) | ≈ 40 | `primeros` y residentes al trasladar, unión en la fusión, «Mecenas de los Aedas» | no (S5 5F) |
| **Total** | **≈ 1.500** | | |

Pruebas TS del sistema 6: **≈ 930 líneas, 80 casos** (`engine/__tests__/`: `tecnologia` 10, `aedas` 10, `aedasResidentes` 21, `cronica` 7, `alijos` 4, `canteria` 2, la parte de
`tecnologiasDelCampamento` de `reclutamientoMercenario` (≈ 4 de 18); `session/__tests__/comandosTecnologia` 14; `bots/__tests__/aedas` 2). Cada tanda nombra las que porta.

Fuera del TS (no hay paridad que perseguir): las Eras IV y V (BA-006 EV-1 a EV-7), el logro de `logistica_campana`, las «otras vías» (conquista, comercio; descartadas el
2026-10-11), el equipo de asedio con efecto (decidido el 2026-10-19: ninguno con números), y «las políticas podrán sumar uno» al cupo de residentes (G6 l.185-186; ver C5).

### 1.2 Lo que ya existe en Rust y el sistema 6 reutiliza

| Pieza | Dónde | Estado |
|---|---|---|
| Tecnologías por Facción (andamiaje) | `EstadoMilitar.tecnologias: BTreeMap<u32, BTreeSet<String>>` (`estado_militar.rs:172-173`), `tiene_tecnologia` (`:222-225`), codec (`codec_militar.rs:488-521`) | solo «adoptadas», sin aparecidas ni reveladas |
| `FijarTecnologias` (administración, comando 65) | `comando.rs:229-230`, `nucleo/heroes.rs:306-322` (hecho `faccion.tecnologias`) | sustituye el conjunto; toca los campamentos |
| Catálogo | `militar::datos::ReglasTecnologias { todas, de_arranque }` (`militar/src/datos.rs:202-207, 304-307`); `porcentaje_facciones` (`:196, 301`) | solo nombres; el resto del bloque `TECNOLOGIAS` no se lee |
| Puertas de las cuatro vías | edificios y mejoras (`asentamiento/obras.rs:130, 215-216`), recetas (`produccion.rs:124`), pueblos (`pueblos.rs:200`), tropas (`nucleo/heroes.rs:415`, avanzadas `avanzadas.rs:363`), campamentos (`mercenarios.rs:137-145`) | listas: miran `tiene_tecnologia` |
| Ganchos de la economía | `Exterior { tiene_tecnologia, factor_tecnologia, orden_era, felicidad_extra, nobleza_extra }` (`asentamiento/asentamiento.rs:96-112`); `produccion.rs:29-32, 100` y `pueblos.rs:256` multiplican por `factor_tecnologia`; `poblacion.rs:48-50` | **vacíos**: `SIN_BONUS`, `ORDEN_ERA = 1`, `FACTOR_EXTRA = UNO` (`nucleo/construccion.rs:48-59`, usados en `:233-243`) |
| Lo extraído y lo fabricado | `extraer_y_cultivar` y `recetas` devuelven `Vec<(Recurso, Mil)>` (`produccion.rs:71, 113`); `Fuentes::extraer(nodo, pedido) -> Mil` (`produccion.rs:18`) | `avanzar_minuto` (`asentamiento.rs:379`) no lo devuelve: nadie lo cuenta |
| Crónica | `nucleo/cronica.rs` (62 líneas) en el cierre de versión | sin las fuentes públicas del sistema 6 ni la diplomacia |
| Reputación | `comercio::reputacion` (ritmo en ppm, `esta_bajo(umbral, t)` en `reputacion.rs:127`, `proximo_cambio` `:122`) | la llegada de residentes puede leerla ya |
| Alijos | `mundo::Mundo::alijos()` (`mundo/src/lib.rs:166-167`): un alijo lejano por veta y los cercanos por banda de cada campamento (`generador/src/contenidoVivo.ts:24-30, 89, 101-104`), oro en milésimas | sin estado de partida ni orden |
| Contrato | `orders.proto`: las 5 órdenes (`:25-36, 60-63, 250-253, 356-360`); `entities.proto:820-871`: `TecnologiasFaccion`, `EstadoTecnologia`, `PrimeroEnLograr`, `EpicaEnCurso`, `AedaResidente`, `AedaItinerante` (con `Plan`); `Alijo` (`:724-728`), `Heroe.alijos_abiertos` (`:482`); `GRUPO_AEDAS` (`:197`); `session.proto:170-188` (cuerpos) | definido; faltan la forma redactada para el jugador (logros sin id de tecnología, reveladas con detalle, épicas posibles) y la crónica para quien llega tarde |
| Hechos que alimentan contadores y épicas | `combate.resuelto` (`combates.rs:395-411`), `combate.encuentro` (`:413-416`, `ganador` = ejército), `combate.asedio_*` (`:701-720`, `murallaCompleta`), `combate.campamento_destruido` (`:579`), `bandidos.caravana_interceptada` (`bandidos.rs:333`), `combate.caravana_interceptada` (`combates.rs:489`), `batalla.aplicada` (`batallas.rs:1019` con Unity; `:760` el asedio con números tras su convocatoria), `tropas.reclutadas` con `nueva` (`heroes.rs:452`), `construccion.edificio_completado` / `mejora_completada` (`construccion.rs:133-137`), `asentamiento.nivel_subio` (`:151`), `comercio.caravana_animal_comprado` y `comercio.caravana_llega` (`comercio/src/evento.rs:31, 36`) | existen; a varios les faltan datos (D9) |

### 1.3 Andamiaje que el sistema 6 sustituye

| Andamiaje | Dónde | Lo sustituye | Tanda |
|---|---|---|---|
| `FijarTecnologias` | `comando.rs:229`, `heroes.rs:306` | la aparición y la adopción; el comando queda como administración sobre las reglas reales (decisión 8) | 6B |
| `ORDEN_ERA = 1` | `construccion.rs:57` | la Era vigente | 6B |
| `SIN_BONUS` | `construccion.rs:49` | `bonusProduccion` de las adoptadas (`canteria` ×1,5 la piedra) | 6C |
| `FACTOR_EXTRA` en felicidad y nobleza | `construccion.rs:59, 242-243` | `1 + bono × residentes` (y el producto con la ocupación, D11) | 6E |
| tecnologías del campamento sin plazo ni `primeros` | `mercenarios.rs:19-20, 137-145` | `tecnologiasDelCampamento` entero | 6C |
| `TecnologiasFaccion` con solo `adoptadas` | `propios.rs:441-447` | aparecidas, reveladas y su detalle | 6G |

### 1.4 Órdenes del contrato que hoy dan `orden.no_disponible`, y su tanda

| Tanda | Órdenes |
|---|---|
| **6C** | `adoptar_tecnologia` |
| **6D** | `comprar_tecnologia_aeda` |
| **6E** | `empezar_epica`, `abandonar_epica` |
| **6F** | `abrir_alijo` |

Con el sistema 5 cerrado (S5 §1.4, Q6 = A), son las últimas 5: al cerrar 6F **ninguna orden del contrato da `orden.no_disponible`** y la rama de `sesiones/ordenes.rs:635-639` se borra
(la prueba de exhaustividad lo obliga). Cada tanda mapea las suyas en el mismo commit y actualiza la tabla `<!-- autorizacion -->` de Proy §5.1. Los comandos nuevos toman los números
libres siguientes al último del sistema 5.

### 1.5 Códigos de evento pendientes del sistema 6 (bloque `<!-- eventos -->` de TA y P l.163), y su tanda

| Tanda | Códigos |
|---|---|
| 6B | `tecnologia.logro`, `era.comienza` |
| 6C | `tecnologia.aparece`, `tecnologia.adoptada`, `aedas.canta_descubrimiento` |
| 6D | `aedas.revela`, `aedas.venta` |
| 6E | `aedas.residente_llega`, `aedas.residente_se_va`, `aedas.epica_empieza`, `aedas.epica_capitulo`, `aedas.epica_tecnologia`, `aedas.epica_cumplida`, `aedas.epica_abandonada` |
| 6F | `alijo.abierto` |

Son los 15 de TA l.1431. `faccion.tecnologias` (de `FijarTecnologias`) sigue. Cada tanda borra los suyos de P l.163 (la prueba `nucleo/tests/catalogo.rs` lo exige).

---------------------------------------------------------------------------------------------------------------------------------------

## 2. Desajustes del Rust de hoy frente a Mechanics y el TypeScript

| # | Desajuste | Rust | Mechanics / TS | Tanda |
|---|---|---|---|---|
| D1 | No hay estado de tecnología del mundo: ni Era ni `eraDesde`, ni contadores, logros, `primeros` ni `cantadas`; por Facción solo un conjunto de «adoptadas» | `estado_militar.rs:172-173` | TA §1 | 6B |
| D2 | Ningún hecho alimenta los 24 contadores; el paso de minuto no devuelve lo extraído ni lo fabricado | `asentamiento.rs:379`; P l.78 («la producción fabricada no alimenta logros»), l.107, l.135 | TA §2.1 | 6B |
| D3 | La Era es una constante 1 en el `Exterior` (la Maravilla pide la V: hoy nunca, y así debe seguir con las Eras I-III) | `construccion.rs:57` | TA §3.2; G6 l.56 | 6B |
| D4 | Sin aparición: una tecnología solo llega por administración; `tecnologia.aparece` y `primeros` no existen | `heroes.rs:306-322` | TA §3 (paso 3), §3.1 | 6C |
| D5 | Sin `bonusProduccion`: la Cantera no saca ×1,5 con `canteria` | `construccion.rs:48-49, 233`; P l.78 | TA §4.1; G6 l.106-107, l.122 | 6C |
| D6 | El campamento desbloquea por porcentaje aunque nadie haya desbloqueado la tecnología (`primeros` no existe) y sin el plazo de 72 h | `mercenarios.rs:139-145`; P l.98 | TA §5; TS `reclutamientoMercenario.ts:32-43` | 6C |
| D7 | Sin adopción: ni Rey en la capital, ni tarifa por Era | `ordenes.rs:635` | TA §4; G6 6.5 | 6C (con 5B y 5C) |
| D8 | La crónica no lleva los públicos del sistema 6 (`tecnologia.logro`, `era.comienza`, `aedas.canta_descubrimiento`, `aedas.epica_cumplida`) y solo vive en las `Delta`: quien conecta después no la ve | `cronica.rs:12-15`; Proy l.617 | TA §8; G6 l.230-231 y Proy l.614 (`GET …/cronica`, 200 por defecto) | 6C (Q2: todo el historial, §5.3) |
| D9 | A los hechos les faltan datos que piden los contadores y las épicas: `combate.resuelto` no lleva las tropas (hoplitas); `combate.asedio_*` no dice `enCombate` ni `conResidentes` (se deduce de si trae `poderAtacante`; los residentes que lucharon están en `defensa.lucharon`); `comercio.caravana_llega` no lleva la carga total ni el tipo (épica `caravana`, ≥ 100) | `combates.rs:395-411, 701-720`; `comercio/src/evento.rs:36` | TA §2.1, §7.2 | 6B (contadores), 6E (épicas) |
| D10 | Un asedio de Unity que resiste no deja hecho de asedio: el contador `asedios.*` tiene que salir del resultado aplicado | `batallas.rs:967-1019`; P l.82 (punto 2), l.107 | TS `resultadoBatalla.ts:171-188` | 6B |
| D11 | El `Exterior` pone la ocupación **o** el factor extra en la felicidad, no su producto: con Aedas residentes en una plaza ocupada se perdería uno de los dos | `construccion.rs:242` | Economia_Construccion l.400: `políticas × ocupación × (1 + bono × residentes)` | 6E |
| D12 | Sin Aedas itinerantes: ni movimiento, ni revelar, ni vender; el grupo `aedas` de `fijar_puerta` no cierra nada | `ordenes.rs:480-481`; P l.140, l.144, l.153 | TA §6; G6 6.7 | 6D (con 5E) |
| D13 | Sin Aedas residentes ni épicas; el título «Mecenas de los Aedas» no puede nacer | P l.120; S5 decisión 12 | TA §7; G2 2.9 | 6E |
| D14 | Sin alijos en la partida: ni `alijos_abiertos` del héroe, ni orden, ni proyección | `ordenes.rs:639`; Niebla l.20; P l.158 | Gen §12; Niebla l.176 | 6F |
| D15 | Proyección: `TecnologiasFaccion` sin `aparecidas` ni `reveladas`; no viajan ni el estado del mundo (Era, logros, descubridoras cantadas), ni Aedas, ni alijos | `propios.rs:441-447`, `propios.rs:703`; P l.158 | Proy §6.3 (`tecnologia`, `aedasAvistados`, `aedasResidentes`, `alijos`); TS `jugador.ts:381-460` | 6G |
| D16 | `primera_en_llegar` recorre todas las plazas en cada `asentamiento.nivel_subio` con nivel ≥ 3 | `cronica.rs:46-50` | — | 6B (el índice de plazas por nivel, que piden los récords `plazasEnNivel.N`, lo sustituye) |
| D17 | Los recursos de la economía ya no son de tick: el TS contaba la extracción de los extractores de la plaza; en Rust las minas y canteras son pueblos con almacén propio | `asentamiento/pueblos.rs:12, 256` | TS `construction.ts:1280` | 6B (decisión 4) |
| D18 | Desde la calibración del sistema 4 (M8), el asedio con números tras su convocatoria emite **dos** hechos: `combate.asedio_*` y `batalla.aplicada` sin resultado de Unity. Si los contadores o las épicas se sacaran de los hechos, como en el TS, contarían dos veces (`asedios.*`, `batallas.libradas`, la `defensa` y la `conquista` de las épicas, con claves distintas) | `batallas.rs:748-761`, `combates.rs:720` | TS: una sola vía por batalla | 6B, 6E (decisión 3: se cuenta en origen, en `resolver_asedio_con_numeros` y en `aplicar_resultado`, nunca en `batalla.aplicada`) |

---------------------------------------------------------------------------------------------------------------------------------------

## 3. Contradicciones entre canon y Mechanics (y dentro de cada uno)

Leyenda: **R** = la resuelve la regla de precedencia (gana Mechanics, se corrige el canon) · **P** = pregunta al autor · **I** = corrección de documento sin impacto de diseño.

| # | Qué | Dónde | Resolución y corrección propuesta |
|---|---|---|---|
| C1 | El campamento desbloquea cuando «la **tiene** el 50 %» frente a «la tienen **ADOPTADA**» | G6 l.100 frente a TA §5 (l.129) | **R**: adoptada; se corrige G6 6.5b |
| C2 | «Consumen el generador … los sorteos de aparición (Aedas, eventos)» frente a «determinista y sin generador aleatorio» (itinerantes, residentes, épicas) | Reloj l.168 frente a TA §3 (l.72), TS `aedas.ts:3-4`, `aedasResidentes.ts:4` | **I**: se quita «Aedas» de Reloj l.168 |
| C3 | «Las condiciones de **adopción** se evalúan al subir un contador» y «en un muestreo de baja frecuencia» | Reloj l.137, TA l.3 y §3 (l.72) | **I**: son las de **aparición**, y no hace falta muestreo: se reescribe TA §3 con la lista de disparadores de la decisión 2 (todo se evalúa al ocurrir algo o en una cita) |
| C4 | La crónica de la caída sale de `combate.asedio_conquista`, que la conquista por Unity no emite; Rust usa `asentamiento.conquistada` | TA §8 (l.239) frente a `cronica.rs:12` | **I**: se escribe en TA §8 lo que hace Rust (las dos vías) y se cierra la DUDA |
| C5 | «Las políticas podrán sumar uno» al cupo de residentes, sin política en Doc 4.4 ni regla en TA | G6 l.185-186 | **I**: se marca como idea (va a `Docs/Ideas a diseñar.md`), no se implementa |
| C6 | Bandas de los alijos: el bloque `ALIJOS` (100-200 / 200-300 / 300-450, 3·3 + 2·5 + 1·11 por campamento) frente a 20-40 / 40-90 y el lejano junto a cada veta | Gen l.1126-1150 y §12 frente a G1 l.190 y `Calibracion_Distancias_Definicion.md` l.179 | **I**: el bloque de Gen describe el generador del TS a propósito (Gen l.3); los alijos de la reconstrucción los pone el artefacto con las cifras calibradas (`contenidoVivo.ts:24-30`). Se añade a Gen §12 la nota con las bandas del artefacto, como la de bandidos (Gen l.854) |
| C7 | `abrirAlijo` agrupado con las órdenes de campamento («residir en el campamento / estar dentro o a su puerta») | Proy l.410 frente a Gen l.842 (columna a `radioPuerta` **del alijo**) | **I**: se saca a su fila con la condición de Gen §12 |
| C8 | La épica la dirige un cargo «presente» que «no exige residir», y el sistema 5 decidió que para ser designado hay que residir (S5 Q4) | Proy l.177, l.755 frente a S5 decisión 4 | **I**: sin conflicto (Gobernador y Sacerdote siempre residen; el Rey no tiene por qué); se precisa Proy l.755 |
| C9 | El evento de la épica perdida: al conquistar la plaza o al aparecer la tecnología por otra vía la épica se borra **sin hecho** | TA §7.1 paso 2; TS `aedasResidentes.ts:72-79` | **I**: se emite `aedas.epica_abandonada` con `motivo` (decisión 11); se escribe en TA §7.1 |
| C10 | El cupo de residentes mira el nivel «actual» y los récords `plazasEnNivel.N` también; la crónica de la primera plaza mira `nivel` (el alcanzado) | TA §3 (paso 0), §7.1 frente a TA §8 y `cronica.rs:50` | **I**: se escribe en TA qué nivel mira cada regla (decisión 5); sin cambio de diseño |

No hay contradicciones de fondo entre G6 y TA: el catálogo, las tarifas, el cupo, las cifras de los Aedas y las épicas coinciden (comprobado bloque contra tabla, incluidos los
`nivelInterno` de los hitos).

---------------------------------------------------------------------------------------------------------------------------------------

## 4. `// DUDA:` del código y filas de paridad que cierra cada tanda

### 4.1 `// DUDA:` que esperan al sistema 6 (`git grep -n "DUDA" -- '*.rs'`, 49 en total, 3 del sistema 6)

| DUDA | Tanda | Cómo se cierra |
|---|---|---|
| `nucleo/cronica.rs:12` — la caída sale de `asentamiento.conquistada`; faltan los públicos del sistema 6 | 6C | queda la regla de Rust (C4); se añaden las fuentes; la parte de la diplomacia la cierra S5 5D-5F |
| `proyeccion/propios.rs:441` — sin `aparecidas` ni `reveladas` | 6G | **cerrada (6G)**: `TecnologiasFaccion` entera, con el detalle de cada revelada (descubridora y `cumplidas[i]`) y las épicas posibles |
| `proyeccion/propios.rs:703` (la parte de «las Aedas» en `ADMIN`) | 6G | `ADMIN` lleva los Aedas, el estado de tecnología entero y los alijos. **Cerrada (6G)**: los residentes con sus claves por el registro (`Clase::Residente`); el estado entero con los contadores y los itinerantes enteros, comparados en cada reparto; los alijos, en el artefacto y en `Heroe.alijos_abiertos` |

Más los placeholders sin `DUDA` de `construccion.rs:48-59` (tabla §1.3), que se retiran en 6B, 6C y 6E.

### 4.2 Filas de paridad (`Docs/Mecanicas a desarrollar.md`) que cierra o reescribe cada tanda

| Tanda | Filas (línea de hoy) |
|---|---|
Líneas de `94cf671` (el sistema 5 y la calibración del sistema 4 las moverán otra vez: cada tanda las busca por su texto).

| Tanda | Filas (línea de hoy) |
|---|---|
| 6B | l.78 (contadores; la parte de bonus va en 6C), l.82 punto (2) (`asedios.contraMurallaCompleta`, `conquistas.conMurallaCompleta`, también desde Unity), l.107 (los contadores de una batalla de Unity), l.135 (`animales.comprados`), l.99 y l.155 (la parte de `FijarTecnologias`: queda como administración) |
| 6C | l.74 (la tecnología del pueblo la da la administración), l.78 (bonus de producción, adopción, hitos), l.85 (`tecnologia_adoptada` como dato), l.98 (el plazo de 72 h), l.158 (la parte de `adoptar_tecnologia`) |
| 6D | l.140, l.144, l.153 (la parte del grupo `aedas`, que 5E habrá reescrito), l.158 (Aedas itinerantes, `comprar_tecnologia_aeda`) |
| 6E | l.120 (Aedas en la felicidad y la nobleza), l.135 (el hecho `caravana` de las épicas), l.158 (residentes, épicas, sus dos órdenes) |
| 6F | l.158 (alijos, `abrir_alijo`) |
| 6G | l.158 (tecnologías aparecidas y reveladas en el contrato; las 5 órdenes) |
| 6H | l.126 (batch de comercio sin tecnologías) y l.155 (batch militar con las de la Era I por administración): pasan a la vía de juego. **Cerradas (6H)**, con la del batch político (5I) y las de 6D-6F que esperaban al batch |
| Cada una | l.163 (los eventos de su tanda, tabla §1.5) |
| Fuera | §20 (Eras IV y V, `logistica_campana`, clientes): sigue tal cual |

Fila nueva que abre el sistema 6 (decisión Q3 del autor): **la calibración de las X de los logros y de la velocidad y la estancia de los Aedas, pospuesta a la fase 7** (bots reales);
se escribe en P al cerrar 6H (**escrita**: fila «Tecnología (calibración, fase 7)», con el día de cada logro medido por el batch).

---------------------------------------------------------------------------------------------------------------------------------------

## 5. Descomposición en tandas

Patrón de los sistemas 3-5: reglas puras en un crate propio, luego el núcleo por piezas (cada una con su foto si cambia el estado), luego contrato y proyecciones, y al final el batch.
Referencia: el TS del sistema 6 son ≈ 1.500 líneas de reglas y órdenes y ≈ 930 de pruebas; con la proporción del sistema 5 sale **≈ 9-10.000 líneas y 8 commits** (6H se reduce
a los invariantes por Q3; la crónica con todo el historial suma ≈ 400 por Q2).

**Orden (decisión Q1 del autor): todo el sistema 6 va después de 5I**, en serie en el árbol principal, una foto cada vez: **6A → 6B → 6C → 6D → 6E → 6F → 6G → 6H**. Así ya están
la Facción como entidad, el Rey y la sucesión (5B), la capital efectiva y la Facción sin plazas (5C), los cargos, la matriz de autorización y la puerta con el grupo `aedas` (5E),
los títulos (5F), la refundación y el botín personal (5G), la proyección política (5H) y el batch político con órdenes de jugador (5I); y la economía entera (antes de 5B). No queda
ningún andamiaje intermedio que hacer para el sistema 6.

Crate nuevo **`crates/tecnologia`**, sin dependencia del núcleo, con `datos.rs` que lee de TA con `include_str!` los bloques `TECNOLOGIAS`, `ERAS`, `TARIFA_ADOPCION`, `AEDAS`,
`EPICAS`, `TITULO_CAPITULO` y `MERCENARIOS.tecnologia`. `militar::datos::ReglasTecnologias` pasa a este crate (militar conserva el id de tecnología de cada tropa).

| Tanda | Contenido | Tamaño | Depende de | Foto |
|---|---|---|---|---|
| **6A `tecnologia`** (reglas puras) | Datos (tasas en ppm, tiempos en ms, umbrales enteros). Modelo: `Contador` (enum de los 24, sin cadenas en caliente), `EstadoTecnologia`, `TecnologiasFaccion` (tres conjuntos de índices de catálogo), `AedaItinerante`, `AedaResidente`, `Epica`. Reglas: sumar a un contador y **el siguiente umbral pendiente de cada contador** (los logros de un contador ordenados: O(1) por suma), Era (todos los logros de la vigente o plazo; una por evaluación), condiciones de hito sobre una vista que da el núcleo (`edificio`, `tecnologia`, `recursoEnCapital`, `capitalEnNivel`, `yacimientoEnTerritorio`), candidatas a aparecer (logro, Era, no aparecida), `hacer_aparecer` con `primeros`, validación de la adopción, conocida por los Aedas, revelar (`faltan` como índices del hito), venta, tecnologías del campamento, residentes (cupo, plazo de llegada con la reputación), épicas (empezar, abandonar, hecho tipado → avance con claves y enfriamiento), itinerantes (objetivo, reparto de nacimiento, candidatos por distancia y recientes), alijos (`puede_buscar`, a la vista, abrir). Pruebas: los casos de `tecnologia`, `aedas` (la parte de reglas), `aedasResidentes`, `alijos` (sin la colocación), `canteria` y `tecnologiasDelCampamento` del TS; invariantes de TA §12 sobre el catálogo | L, ≈ 2.000 | 5I (Q1; no depende del código) | — |
| **6B núcleo: estado del mundo, contadores, logros y Eras** | `Estado.tecnologia` sustituye a `militar.tecnologias` (codec y hash). Contadores **en origen** (decisión 3): `Fuentes::extraer` del núcleo suma lo extraído de plazas y pueblos (D17), `avanzar_minuto` devuelve lo fabricado de las cuatro recetas de bronce, el combate con números suma al resolver (resuelto, encuentro, asedio con `enCombate`/`conResidentes`/muralla, campamento, caravanas), `aplicar_resultado` suma lo de una batalla de Unity (D10) y `batalla.aplicada` no suma nada (D18), reclutar suma soldados y escuadra nueva **en una plaza (reponer incluido) y en un campamento de mercenarios** (Q4; no la guarnición de serie de las avanzadas ni la tropa prestada), `comprar_animal` suma 1; `plazasEnNivel.N` con un índice de plazas por nivel actual (sube y baja al cambiar de nivel; el récord, al subir). Al cruzar un umbral, `tecnologia.logro` público. La Era: **una cita global** al fin del plazo (prioridad 10) y la comprobación de «todos» al fijar un logro; `era.comienza` público; `orden_era` real en el `Exterior`. `FijarTecnologias` sobre las reglas reales (decisión 8). Pruebas: `tecnologia.test` (contadores, Era), determinismo normal/acelerado/a trozos, foto ida y vuelta, el paso de minuto sin empeorar; una prueba por contador y vía (también el recluta de campamento) | L, ≈ 1.300 | 6A | **sí** |
| **6C núcleo: aparición, adopción, campamentos y crónica** | Aparición **por disparadores** (decisión 2), solo para las candidatas de cada Facción viva; `recursoEnCapital` en el paso de minuto **de la capital** y solo si hay una candidata que lo pida. `AdoptarTecnologia`: Rey presente en la capital efectiva, tarifa de la Era del almacén de la capital, instantánea; `tecnologia.adoptada` (y su hecho para las épicas, 6E). `factor_tecnologia` real (`canteria` en plazas y pueblos). Campamentos: de arranque, `primeros` + 72 h (una cita por tecnología descubierta) o ≥ 50 % de las vivas con ella **adoptada**; tocan los campamentos. Cita de conocimiento a `primeros.en + 24 h` por tecnología: `aedas.canta_descubrimiento` público (y, en 6D, la revelación a las plazas con un Aeda parado). Crónica: los cuatro públicos y **el historial entero en su almacén aparte** (§5.3, Q2), con la consulta paginada. Pruebas: `comandosTecnologia` (adopción y rechazos), `tecnologia.test` (aparición y Era cerrada), `canteria`, `tecnologiasDelCampamento`, `cronica.test` (la parte pública); la crónica tras un corte (recuperar, repetir el registro, ninguna entrada duplicada ni perdida); repetir las suites de sesión | L, ≈ 1.700 | 6B | **sí** (las citas y `cantadas`; la crónica, no: va fuera de la foto) |
| **6D núcleo: Aedas itinerantes** | Entidad nueva con `Plan` por la red de caminos (el mismo planificador de rutas que las columnas, a `velocidad`), **dos eventos por Aeda** (llegada y fin de estancia; sin camino, otra estancia). Objetivo `max(3, ⌈vivas/3⌉)` al cambiar las Facciones vivas; nunca bajan. Plazas que los aceptan: la puerta con el grupo `aedas` de S5 5E; **cerrarles la puerta o vetarlos con un Aeda dentro lo hace salir al instante hacia su siguiente destino**, y lo que ya reveló queda revelado (Q5). Revelar al llegar y en la cita de conocimiento (a las plazas con un Aeda parado); `aedas.revela` privado de la plaza con `descubridora` y `faltan`. `ComprarTecnologiaAeda`: Rey o Gobernador presentes, Aeda parado en la plaza, Era vendible, conocida, no aparecida, oro de la plaza (2 × tarifa); `aedas.venta`. En la visión como móviles ajenos no atacables. Pruebas: `aedas.test` (10), la parte de venta de `comandosTecnologia`; ninguna entidad ociosa (un Aeda siempre tiene su siguiente evento y nada más); cerrar la puerta con un Aeda dentro | L, ≈ 1.300 | 6C | **sí** |
| **6E núcleo: Aedas residentes y épicas** | Residentes por plaza: cupo (Palacio activo ∧ nobleza > 0, nivel actual), **cita de llegada solo mientras la plaza espera con cupo libre** (24 h; con la reputación baja, la regla de la decisión 6), salida del último al bajar el cupo (en el paso de minuto solo si cambia nobleza 0/>0 o el nivel, y al perder el Palacio), conquista y ruina. `felicidad_extra` y `nobleza_extra` reales (D11). `EmpezarEpica`/`AbandonarEpica` (`dirigeAedas`). Hechos de épica **tipados y anotados en la transición** desde donde ocurren (defensa, conquista, batalla, obra, ascenso, adopción, caravana con carga ≥ 100, D9), procesados antes de cerrar la versión: a lo sumo uno por Aeda y versión, enfriamiento 6 h, claves (50); al cerrar el último capítulo, `hacer_aparecer`, `cumplidas[F] += 1`, `aedas.epica_tecnologia` (privado) y `aedas.epica_cumplida` (público, a la crónica). Los datos de «Mecenas de los Aedas» para los títulos de S5 5F. Pruebas: `aedasResidentes.test` (21), la parte de épicas de `comandosTecnologia`, la parte de Mecenas de `eventosDominioTitulos` | L, ≈ 1.500 | 6D | **sí** |
| **6F núcleo: alijos** | `alijos_abiertos` por héroe (índices del artefacto). `AbrirAlijo` con el héroe de la cuenta: Facción sin plazas o sin Facción, no abierto, su columna a `radioPuerta` en el instante; el oro (entero) al carro con `cargar_botin_en`, lo que no cabe se pierde (decisión 10); `alijo.abierto { heroe, alijo, oro, cargado }`. Rejilla de alijos para «a la vista» (los cuenta el artefacto: unos cientos). Pruebas: `alijos.test` (ver y abrir), una prueba de que nada programa eventos | S, ≈ 500 | 6E | **sí** |
| **6G contrato y proyecciones** | `TecnologiasFaccion` entera en `FACCION` y, aditivo, el detalle de las reveladas (descubridora, `cumplidas[i]`) y las épicas posibles; **forma pública del estado del mundo** en `HEROE` (Era, `era_desde`, logros cumplidos como `{contador, umbral, en}` sin id de tecnología, descubridoras ya cantadas; **los contadores en curso no**, Q6); `EstadoTecnologia` entero, con los contadores, solo en `ADMIN`; `AedaItinerante` en `VISTA` redactado (posición, plan, plaza; sin `recientes`); `AedaResidente` con su épica en `FACCION`; `Alijo` a la vista en `HEROE` (se renueva en el muestreo de la vista, como la frontera) y `Heroe.alijos_abiertos`; la crónica (§5.3: un mensaje de sesión para pedir un tramo, y las entradas nuevas en vivo como hoy); eventos tipados; dorados en Rust, C# y TS; `Comandos_Parametros.md` sin «Órdenes sin comando». Sin cambio de foto | M, ≈ 1.000 | 6F | — |
| **6H batch tecnológico: invariantes** (Q3: sin calibrar) | `batch tecnologia` con bots mínimos que construyen hacia los hitos, adoptan con el Rey en la capital, compran a los Aedas, empiezan épicas, abren alijos y reclutan en plazas y campamentos, **para comprobar reglas, no cifras**: las X, la velocidad y la estancia de los Aedas se quedan las del TS hasta la fase 7. Invariantes orden a orden: logro fijo y nunca antes de su umbral; ningún contador baja; `adoptadas ⊆ aparecidas`; nada aparece de una Era cerrada; comprar nunca salta el logro; una épica por tecnología y Facción; un hecho no cuenta dos veces (también el asedio con números, D18); los itinerantes nunca bajan y ninguno queda dentro de una plaza que les cierra la puerta; un alijo se abre una vez por héroe; la crónica guardada es exactamente la emitida; reproducción y foto ida y vuelta a mitad de corrida. Informa (sin juzgar) en qué día cae cada logro y cada Era, para la fase 7. El batch de comercio y el militar pasan a la vía de juego (sin `FijarTecnologias`) | M, ≈ 1.000 | 6G | — |

Notas de orden: con Q1 la cadena es lineal y cada tanda depende de la anterior; dentro de ella, 6C es la que más toca la sesión (adopción, crónica) y 6D la que más toca el espacio (planes y
visión de los itinerantes). 6F podría ir en cualquier punto tras 6B (no depende de 6C-6E): se deja en su sitio para no tener dos fotos abiertas.

### 5.1 Disparadores de la aparición (sin tick)

Una tecnología solo puede aparecer cuando cambia algo que mira. Las candidatas de una Facción son a lo sumo 24 (las de logro cumplido, Era abierta y no aparecidas), sus hitos se miran
sobre sus plazas (≤ 7) y la consulta es barata: se evalúa la Facción afectada al ocurrir:

| Ocurre | Se evalúan |
|---|---|
| se fija un logro; empieza una Era | todas las Facciones vivas, solo esa tecnología o las de la Era nueva |
| un edificio se completa o se mejora; una plaza sube de nivel | la Facción de la plaza (`edificio`, `capitalEnNivel`) |
| se adopta una tecnología | la Facción (`tecnologia`) |
| la zona de una plaza crece (solo por obra, `Economia_Construccion.md` l.131) | la Facción (`yacimientoEnTerritorio`) |
| la Facción gana una plaza (fundación, conquista, anexión, fusión) o cambia de capital | la Facción |
| el paso de minuto de la capital, si alguna candidata pide `recursoEnCapital` | esa candidata (hoy solo `aleacion_bronce` con estaño) |

Perder algo no hace falta mirarlo: aparecer es para siempre.

### 5.2 Pruebas comunes a todas las tandas del núcleo

`cargo test --workspace --no-fail-fast` en verde; determinismo normal, acelerado y a trozos; foto ida y vuelta con el formato nuevo (y la anterior rechazada con su motivo); **ninguna
entidad ociosa programa eventos** (un estado de tecnología sin cita pendiente, una plaza sin cupo, un Aeda residente sin épica, un alijo: nada en la cola); el paso de minuto sin empeorar
(`cargo test --release -p nucleo --test asentamientos rendimiento -- --ignored`, 1.000 plazas × 3 días); la prueba del catálogo de eventos y la de la matriz de Proy §5.1; notas
«Implementado en Rust (fecha, tanda)» en TA, Gen, Proy y los Mechanics tocados; filas de paridad cerradas o reescritas.

### 5.3 Dónde va la crónica (decisión Q2: todo el historial, fuera de la foto)

La crónica es una **salida determinista** del mundo: cada entrada sale de los hechos de una versión, y repetir el registro desde una foto la vuelve a producir idéntica. Por eso no
necesita entrar en la foto ni en el hash: basta un fichero propio que se escribe al cerrar cada versión y se recompone en el arranque.

- **Almacén.** Un segundo `persistencia::Registro` (los mismos segmentos con longitud + CRC32, `crates/persistencia/src/registro.rs`) en un directorio hermano del de la partida,
  `cronica/`. Cada entrada: `versión u64 · instante i64 · tipo u16 · Facciones · plaza? · nivel?` en binario fijo (la misma forma que `cronica.entrada` y los cuatro públicos del
  sistema 6), ≈ 40 bytes. Con una entrada cada pocos minutos de mundo, una season son pocos MB.
- **Escritura.** El núcleo deja las entradas de la versión en una cola; las escribe el mismo hilo que ya sincroniza el registro de órdenes, por lotes, **fuera del camino de la orden**
  (no la retrasa). La secuencia del almacén es su propio contador sin huecos; la versión del mundo va dentro de cada entrada.
- **Recuperación.** Al arrancar se abre como el registro (corta la cola dañada y avisa); se **truncan las entradas con versión mayor que la de la foto** que se carga, y al repetir el
  registro de órdenes desde esa foto el núcleo las vuelve a escribir. Así no hay duplicadas ni perdidas tras un corte, se corte donde se corte. Si el almacén falta o está dañado antes
  de la foto, se avisa y se sigue: la crónica antigua no se puede recomponer sin repetir desde el principio (herramienta de administración aparte, no en el arranque).
- **Lectura.** Un mensaje de sesión nuevo (aditivo, contrato 05) `PedirCronica { desde_version?, limite }` → `Cronica { entradas[], mas_antiguas: bool }`, paginado hacia atrás desde
  la última (200 por defecto, entre 1 y 1.000, como Proy l.614), igual para todos. Se sirve del fichero con un índice en memoria de versión → desplazamiento cada 256 entradas: leer un
  tramo no toca el estado del núcleo ni bloquea el actor. Las entradas nuevas siguen llegando en las `Delta` como hoy.
- **Rendimiento.** Coste por versión: empujar unos pocos registros a una cola (las versiones sin entradas, nada). Nada más en la foto ni en el paso de minuto.

### 5.4 Riesgos

- **R1. La economía cambia sola.** Con la Era real nada cambia (la Maravilla sigue cerrada), pero `canteria` sube la piedra ×1,5 y los residentes suben la felicidad y la nobleza: las
  cifras de `batch poblacion` dejan de valer donde aparezcan. Con Q3 no se recalibra: 6H solo informa.
- **R2. Las X de los logros no valen en este mundo (aceptado hasta la fase 7, Q3).** Las de la Era I se calibraron con el batch del TS (otro mundo, otra escala, otra economía): la Era
  puede cambiar por plazo con logros sin cumplir o cumplirlos en horas. 6H deja anotado en qué día cae cada uno para la fase 7.
- **R3. Contar en origen exige no olvidarse de ninguna vía.** Cada contador lleva una prueba que provoca su hecho por cada vía (números, Unity, el asedio con convocatoria, el
  recluta de campamento) y comprueba el incremento exacto (D18).
- **R4. La crónica fuera de la foto.** Si se escribiera tarde, una foto podría quedar por delante del almacén: **antes de escribir una foto se vacía la cola de la crónica** (las
  entradas con versión ≤ la de la foto ya están en disco), y la recuperación trunca lo posterior a la foto (§5.3).

---------------------------------------------------------------------------------------------------------------------------------------

## 6. Preguntas al autor (6; respondidas el 2026-10-24, ver «Decisiones del autor» al final)

**Q1. Cuándo se hace el sistema 6 respecto del sistema 5.**
6A (reglas puras) no depende de nada; 6B (contadores, logros, Eras) y 6F (alijos) solo de la economía entera; 6C-6E piden el Rey, la capital y los cargos del sistema 5.
- A) 6A ya, en paralelo, en un worktree aparte (como 5A); 6B y 6F intercaladas entre tandas del sistema 5 en el árbol principal, una foto cada vez; 6C-6E detrás de 5E y 5F; 6G-6H al final.
- B) Todo el sistema 6 después de 5I.
- C) 6A y 6B en paralelo con el sistema 5 en otra sesión (dos fotos nuevas a la vez: habría que renumerar al fusionar).
Recomendada: **A** (adelanta lo que no depende de la política, sin dos sesiones cambiando la foto a la vez).

**Q2. La crónica para quien llega tarde.**
El canon y Mechanics dan la crónica como una consulta (`GET …/cronica`, las 200 últimas, la misma para todos); en Rust solo viaja en las `Delta` en vivo: quien no estaba conectado no la ve.
- A) El núcleo guarda las últimas 200 entradas públicas (un anillo en la foto, pocos KB) y el cliente las recibe al conectar; en vivo, como hoy.
- B) Solo en vivo, como hoy (lo que no se vio se pierde).
- C) Todo el historial, en un almacén aparte de la foto.
Recomendada: **A** (es lo que dicen G6 y Proy; cuesta un anillo fijo y deja la crónica lista para los clientes).

**Q3. Las X de los logros (y las cifras de escala de los Aedas) en el mundo de Rust.**
Las X de la Era I se calibraron con el batch del TypeScript; las de las Eras II y III eran placeholder. El mundo de Rust es otro (calibración de distancias, economía por minuto y entera),
y la velocidad de los itinerantes (20) y su estancia están en la escala vieja.
- A) Se portan tal cual y 6H mide los contadores semana a semana en este mundo y propone las X nuevas (y la velocidad de los Aedas) para que las decidas.
- B) Se portan tal cual y no se vuelven a medir hasta la fase 7 (bots reales).
- C) Se escalan ya por un factor provisional (lo extraído y lo reclutado en Rust frente al TS en una semana).
Recomendada: **A** (BA-006 D53: «la X es lo que marque el contador del batch en su semana objetivo»; sin medirla, la Era cambia por plazo).

**Q4. ¿Qué reclutas cuentan para los logros `reclutados.*`?**
En el TypeScript solo cuenta reclutar (y reponer) en una plaza; el recluta en un campamento de mercenarios y la guarnición de serie de una avanzada no suman. El canon dice
«X soldados reclutados» sin más.
- A) Como el TypeScript: solo en una plaza (reponer incluido).
- B) También en un campamento de mercenarios.
- C) También la guarnición de serie de las avanzadas.
Recomendada: **A** (paridad, y la X de la Era I se midió así; el campamento ya recibe la tecnología el último).

**Q5. Un Aeda itinerante dentro de una plaza que le cierra la puerta.**
Desde el sistema 5 (Q1), cerrar la puerta o vetar expulsa a quien ya no puede entrar. En el TypeScript el Aeda acababa su estancia.
- A) El Aeda sale al instante hacia su siguiente destino; lo que ya reveló queda revelado.
- B) Acaba su estancia, como el TypeScript (y se le puede seguir comprando).
Recomendada: **A** (una sola regla para todo lo que está dentro: sale quien ya no puede entrar).

**Q6. Qué ve todo el mundo del avance de la tecnología.**
El canon dice que el logro es público y «no dice qué tecnología abre»; el TypeScript enseñaba los logros cumplidos (contador, umbral, cuándo), nunca los contadores en curso.
- A) Como el TypeScript: los logros cumplidos y la Era; los contadores, solo la administración.
- B) También los contadores en curso (una barra de progreso pública de cada hecho del mundo).
- C) Solo la Era y que «se cumplió un logro», sin contador ni umbral.
Recomendada: **A** (con B el umbral pendiente de cada contador dice qué logro falta y de qué Era; C quita lo que canta la crónica).

---------------------------------------------------------------------------------------------------------------------------------------

## 7. Decisiones que tomaría sin preguntar (se marcan con `// DUDA:` si se discuten)

1. **Crate `tecnologia`** puro, con sus datos en los bloques de TA; las tecnologías van por índice del catálogo (`u8`) en el estado y por id de catálogo en el contrato.
2. **Sin muestreo de baja frecuencia**: todo lo que TA dice «en cada evaluación» se dispara al ocurrir (§5.1) o en una cita. Citas, todas de prioridad 10 (Reloj l.98): el plazo de la Era
   (una, global), el conocimiento (`primeros.en + 24 h`) y el campamento (`primeros.en + 72 h`) de cada tecnología descubierta (≤ 24 cada una en toda la partida), la llegada o el fin de
   estancia de cada itinerante, y la llegada de un residente en cada plaza que espera con cupo.
3. **Contadores en origen y tipados**: no se reparsean los hechos de la versión (como hacía `exito`); cada sitio que produce el hecho suma a su `Contador`. Los hechos de épica se anotan
   igual en la transición y se procesan antes de cerrar la versión, de modo que lo que aparece por una épica entra en la misma versión y su `aedas.epica_cumplida` llega a la crónica.
4. **Lo extraído cuenta en plazas y pueblos** (D17), por `Fuentes::extraer`: es la cantidad real que sale del nodo, en unidades enteras tras la economía entera. Lo fabricado, las cuatro
   recetas de bronce de TA §2.1.
5. **Qué nivel mira cada regla**: récords `plazasEnNivel.N` y cupo de residentes, el **actual** (operativo, como el TS); la crónica de la primera plaza, el alcanzado (como hoy). El índice
   de plazas por nivel actual sustituye el recorrido de `primera_en_llegar` (D16).
6. **Llegada de residentes con la reputación baja, exacta**: la cita de llegada vence a `espera + 24 h`; si entonces la reputación está en −50 o menos, se programa el primero de
   `espera + 72 h` y el cruce hacia arriba de −50 de su ritmo (se recalcula si la reputación cambia con un suceso). Es la semántica del TS («se mide al comparar», su prueba
   `aedasResidentes.test` l.89) sin muestreo.
7. **Ids**: itinerantes `aeda-N` y residentes `aeda-rN` con sus contadores propios (los nombres salen de `(N − 1) mod 24`, como el TS); alijos, por índice del artefacto con su id de
   cadena en el contrato.
8. **`FijarTecnologias` se queda como administración sobre las reglas reales** (como la decisión 9 del S5): pone las dadas como aparecidas y adoptadas, registra `primeros` si nadie la
   tenía (para que el campamento y los Aedas se comporten como en juego) y no cobra tarifa. Los jugadores y los bots solo usan las órdenes del contrato.
9. **Rechazos finos**, como el resto del catálogo de Rust (`Docs/Coordinacion/07_Catalogo_de_Rechazos.md`), no los tres del TS (`tecnologia.adopcion_invalida`, `aedas.venta_invalida`,
   `aedas.epica_invalida`): `tecnologia.no_aparecida`, `tecnologia.ya_adoptada`, `tecnologia.sin_capital`, `tecnologia.rey_fuera_de_la_capital`, `tecnologia.sin_tarifa`,
   `aedas.no_hay_aeda`, `aedas.no_la_conoce`, `aedas.era_no_vendible`, `aedas.ya_aparecida`, `aedas.sin_oro`, `epica.*` (los ocho motivos de TA §7.2), `alijo.desconocido`,
   `alijo.no_puedes_buscar`, `alijo.ya_abierto`, `alijo.lejos`. El detalle en castellano, como siempre.
10. **Alijo con el carro lleno**: se abre igual y lo que no cabe se pierde (TS, y el precedente C7 de la calibración del comercio: lo que no cabe se pierde); el hecho dice `oro` y
    `cargado`. Una columna con varios héroes: cada uno abre el suyo una vez y el oro va al carro común (`cargar_botin_en`).
11. **La épica perdida avisa**: al cambiar la plaza de dueño o al aparecer la tecnología por otra vía se emite `aedas.epica_abandonada` con `motivo` (`conquista`, `aparecida`, `orden`),
    sin código nuevo (C9).
12. **`faltan` y `cumplidas` como índices del hito**, no texto: el cliente compone la frase con el catálogo (como la crónica).
13. **Anexión y fusión** (S5 5F): `primeros` y la Facción de los residentes pasan a la que recoge; las épicas cumplidas se suman; en la fusión, unión de los tres conjuntos de
    tecnologías; en la anexión, las de la absorbida se pierden (TS `trasladoDeFaccion.ts:49-52, 87-126`, `fusion.ts:87-133`).
14. **Los itinerantes en la visión**: móviles con plan que la visión muestreada ve como una columna ajena (Niebla §5.4); no entran en la rejilla de encuentros (no se les ataca ni
    interceptan).
15. **Las Eras IV y V por datos**: el código no conoce el número de Eras; mientras sus bloques no existan, la III es la última (sin cita de plazo) y `nivelTechoProvisional` sigue en 4.
16. **Venta solo a la Facción dueña de la plaza**, sea cual sea su relación con quien la mira (los Aedas son neutrales, TA §13), y paga el almacén de esa plaza.
17. **Todo en enteros** (`Consideraciones/Calibracion_Sistema4_Decisiones.md`, C3b): tarifas y precio de venta en unidades enteras; contadores `u64` en unidades (lo extraído y lo
    fabricado ya llegan enteros de la economía entera); factores (`canteria` 1,5, bonos de los residentes 0,05 y 0,25, `porcentajeFacciones` 0,5) en ppm, aplicados al ritmo antes del
    resto oculto; plazos en ms de mundo. El oro de un alijo viene del artefacto en milésimas y se carga en unidades enteras, redondeando hacia abajo.

## Decisiones del autor (2026-10-24)

1. **Q1 Orden:** todo el sistema 6 **después del sistema 5** (tras 5I), no intercalado. → §5: 6A → 6B → … → 6H en serie, sin andamiaje intermedio.
2. **Q2 Crónica:** se guarda **todo el historial** de entradas públicas, en un almacén aparte de la foto, y quien llega tarde lo puede leer. → §5.3 (un `Registro` propio en
   `cronica/`, truncado a la versión de la foto al recuperar y rehecho al repetir el registro; consulta paginada `PedirCronica`), en 6C; su lectura por el contrato, en 6G.
3. **Q3 Umbrales de los logros (X) y velocidad de los Aedas:** se portan tal cual del TypeScript y **no se vuelven a medir hasta la fase 7** (bots reales). → 6H se reduce a los
   invariantes; fila de paridad nueva para la fase 7.
4. **Q4 Reclutas que cuentan para los logros `reclutados.*`:** los de una plaza (reponer incluido) **y también los de un campamento de mercenarios**. → 6B (divergencia aprobada del
   TS, que solo contaba la plaza; se anota en TA §2.1 y §14).
5. **Q5 Aeda itinerante en una plaza que le cierra la puerta:** sale al instante hacia su siguiente destino; lo que ya reveló queda revelado. → 6D.
6. **Q6 Qué ve todo el mundo del avance:** los logros cumplidos y la Era, como el TypeScript; los contadores en curso, solo la administración. → 6G.

Las 17 decisiones de §7 quedan como decisiones técnicas del agente, sin objeción del autor por ahora.

## Avance de 6A (2026-10-25): reglas puras en `crates/tecnologia`

Hecha en la rama `tanda-6a` (worktree aparte, sobre `7ab7986`, foto `BAN35`; sin cambio de foto ni de esquema). Crate nuevo `crates/tecnologia` (≈ 2.650 líneas con pruebas), sin dependencia
del núcleo: depende de `base` y `asentamiento` (los helpers de bloques, `Recurso`, `Tipo`, `Almacen`, `Costo`), con `comercio` solo en pruebas (para igualar la escala de la reputación).
Fuera del crate solo se tocaron el `Cargo.toml` del workspace y `militar`: `militar::datos::ReglasTecnologias` vive ahora en `tecnologia::datos` y `militar` la reexporta (mismo orden
alfabético de antes, así que el núcleo, las proyecciones y los dorados no cambian). El núcleo no se tocó.

**Datos** (`datos.rs`, de TA): `TECNOLOGIAS` **en el orden del documento** (serde_json ordena las claves; el orden se saca de la posición de cada clave en el bloque, y es el índice
`TecId`), `ERAS` (plazo en ms), `TARIFA_ADOPCION` (como `Costo` en unidades enteras), `AEDAS` (minutos en ms, bonos en ppm, umbral de reputación en la escala de
`comercio::reputacion`, carga mínima en milésimas, `cupoPorNivel` indexado por nivel), `EPICAS`, `TITULO_CAPITULO` y `MERCENARIOS.tecnologia` (ppm y ms). Las cifras son las del TS (Q3).
Los bloques del documento no cambian (la identidad de las reglas del núcleo ya incluye TA por `militar::datos::texto_de_parametros`).

**Módulos y tipos públicos**

| Módulo | Tipos y funciones | Reglas |
|---|---|---|
| `lib` | `TecId` (u8), `FaccionId`, `AsentamientoId`, `Conjunto` (mapa de bits de ≤ 64 tecnologías) | |
| `datos` | `Reglas` (catálogo, Eras, Aedas, campamento, títulos de capítulo, `de_arranque`, `por_contador`), `Definicion`, `Condicion`, `Logro`, `Bonus`, `HechoEpico`, `Capitulo`, `DefinicionEpica`, `Era`, `ReglasAedas`, `ReglasCampamento`, `ReglasTecnologias` | TA §11 |
| `contador` | `Contador` (enum de los 24, `nombre`, `es_record`), `Delta`, `de_extraccion`, `de_fabricacion`, `combate_resuelto`, `asedio`, `ContextoBatalla`, `batalla_unity`, `reclutamiento`, `PESO_CARROS_GUERRA` | TA §2.1 en origen (decisión 3), D18, **Q4** |
| `estado` | `EstadoTecnologia` (sumar, aplicar, subir_record, umbral_pendiente, todos_los_logros_de_la_era, fin_del_plazo, avanzar_era, candidatas, hacer_aparecer, conocida_desde, conocida_por_aedas, cantar), `TecnologiasFaccion`, `Primero`, `CambioEra` | TA §1-§3, §6.1; el siguiente umbral en O(1) (a lo sumo un logro por contador); una Era por llamada; sin plazo en la III (decisión 15) |
| `hito` | `VistaFaccion` (la implementa el núcleo), `cumple`, `faltan`, `Aparicion`, `Revelacion`; en `EstadoTecnologia`: aparecer_por_hito, pide_recurso_en_capital, revelar | TA §3.1, §3 paso 3, §6.3; disparadores de §5.1; `faltan` como índices (decisión 12) |
| `adopcion` | `ErrorAdopcion`, `ErrorVenta`, `precio_de_venta`, `factor_produccion`; en `EstadoTecnologia`: adoptar, vender, del_campamento, campamento_libre_en, tecnologias_del_campamento | TA §4, §4.1, §5 (C1: adoptada), §6.4; rechazos finos (decisión 9) |
| `itinerantes` | `AedaItinerante` (llegar, candidatos, salir, esperar, perder_destino, siguiente_evento), `objetivo`, `nacer`, `aeda_en` | TA §6.2; un evento pendiente por Aeda; Q5 la aplica el núcleo con `candidatos` y `salir` |
| `residentes` | `Residentes` (ajustar, llega, cambiar_de_dueno, al_aparecer, quitar_plaza, de_plaza, de_faccion, cumplidas_de), `AedaResidente`, `NOMBRES`, `cupo`, `Llegada`, `llegada`, `Ajuste`, `factor_felicidad`, `factor_nobleza` | TA §7.1; nivel actual (decisión 5); la llegada exacta con la reputación baja (decisión 6) |
| `epicas` | `Epica`, `Hecho` (defensa, conquista, batalla, de_unity, obra, ascenso, adopcion, caravana), `Clave`, `ContextoUnity`, `ErrorEpica`, `Avance`; en `Residentes`: empezar_epica, abandonar_epica, avanzar_epicas | TA §7.2; hechos tipados y claves sin cadenas; la épica perdida la devuelven `cambiar_de_dueno` y `al_aparecer` (decisión 11) |
| `alijos` | `Abiertos`, `ErrorAlijo`, `Apertura`, `a_la_vista`, `abrir` | Gen §12; oro hacia abajo a unidades enteras, lo que no cabe se pierde (decisión 10) |

**Pruebas:** 67 en el crate, todas en verde (más las de `militar`, que siguen pasando con la `ReglasTecnologias` movida). Portadas del TypeScript, con el fichero de origen en cada módulo
de pruebas: `tecnologia.test.ts` (los 10: catálogo 3, estado 1, avance 3, contadores 3), `aedas.test.ts` (8 de 10: cuántos, nacen, recorrido, revelar 2, vender 3; la puerta cerrada es de
`politica::puerta` y del núcleo, y revelar durante la estancia lo dispara el núcleo, 6D), `aedasResidentes.test.ts` (20 de 21: catálogo, ciclo de vida 8, efectos 1 de 2, empezar y
abandonar 3, hechos 3, avance 4; «Mecenas» es de `politica::titulos`), `alijos.test.ts` (2 de 4: ver y abrir; colocar es del generador del artefacto), `canteria.test.ts` (1 de 2: el factor; el
rendimiento de la Cantera es de la economía, 6C), `reclutamientoMercenario.test.ts` (los 3 de `tecnologiasDelCampamento`) y `comandosTecnologia.test.ts` (la parte de reglas: adopción y
sus rechazos, compra y su precio, la épica cerrada por una adopción). Además, los invariantes de TA §12 sobre el catálogo, los contadores de una batalla de Unity
(`resultadoBatalla.ts`), la memoria de claves, la llegada con el cruce de la reputación y los alijos en unidades enteras. `cargo test --workspace --no-fail-fast` (una pasada: la tanda no toca
sesión, proyecciones ni hechos): **1.519 pruebas en verde** (98 resultados, 14 ignoradas, 0 fallos); `buf lint` limpio.

**Documentos corregidos** (contradicciones de §3 que solo tocaban documentos; gana Mechanics): C1 (G6 6.5b: «la tiene **adoptada**»), C2 (Reloj: los Aedas no consumen azar), C3 (TA y
Reloj: la aparición se evalúa al ocurrir lo que mira su hito; la Era, al fijar un logro o en su cita), C5 (G6: «las políticas podrán sumar uno» pasa a `Docs/Ideas a diseñar.md` §32).
Q4 queda escrita en TA §2.1 y §14. La nota «Implementado en Rust (6A)» está en TA §14. Las demás (C4, C6-C10) son de sus tandas.

**Dudas para el autor**

1. **Comprar a un Aeda nunca crea `primeros`.** Un Aeda solo vende lo que conoce, y conocer exige descubridora; así que la compra nunca es «la primera» y la frase de TA §13 («aparecida por
   épica o compra sí cuenta para el campamento: `primeros` se registra por cualquier vía») solo vale para la épica. Recomendado: dejarlo así y precisar TA §13 en 6D.
2. **La adopción no rechaza «sin Rey»** en las reglas puras (el TS sí, con `tecnologia.adopcion_invalida`): quien da la orden ya tiene que ser el Rey por la autorización del núcleo, así
   que el caso no llega. Recomendado: sin rechazo propio.
3. **La clave de una obra lleva la plaza** (`obra:<plaza>:<edificio>:<nivel>`): en Rust el id de un edificio es de su plaza, no global como en el TS. No cambia ninguna regla.

## Avance de 6F (2026-10-25, rama `tanda-6f`, foto `BAN36`)

Hecha en paralelo con 6A (el crate `tecnologia`) y con la crónica con todo su historial, sobre `7ab7986` (foto `BAN35`): los alijos no dependen de la tecnología.

**Diseño.**
- **Reglas puras:** las de `tecnologia::alijos` (6A: `a_la_vista`, `abrir` con `Apertura { oro, cargado }`). 6F las escribió primero en `crates/nucleo/src/alijos.rs`, en paralelo; al integrar con `origin` (6A, `b18a259`) ese módulo se borró y el núcleo usa las del crate (`nucleo` depende ya de `tecnologia`). «Puede buscar» (sin Facción o de una sin plazas) lo calcula el núcleo y se lo pasa.
- **El artefacto** (55 alijos en el de hoy) se lee una vez en la geografía del núcleo (`nucleo/alijos.rs`, `AlijosDelMundo`): id de cadena, posición y oro en milésimas, con una rejilla de 10 u. La rejilla es la de la economía (`economia::Rejilla`, ahora `pub(crate)`): no hacía falta otra. La de encuentros no sirve (es de móviles con plan). No es estado: no entra en la foto.
- **Estado:** `HeroeNucleo::alijos_abiertos: BTreeSet<u32>` (índices del artefacto), en el códec del héroe tras `residencia_cambiada_en`. Foto `BAN36`; `BAN35` se rechaza con su motivo.
- **Orden:** `AbrirAlijo { heroe, alijo }`, **174** (el siguiente libre tras el 173), `Autoriza::CuentaDelHeroe`, implica al héroe (candado de batalla). Rechazos en este orden: `alijo.desconocido`, `alijo.no_puedes_buscar`, `alijo.ya_abierto`, `alijo.lejos` (también sin columna en el mapa). El oro va en unidades enteras hacia abajo al carro con `cargar_botin_en`; lo que no cabe se pierde y el alijo queda abierto igual (decisión 10). Hecho `alijo.abierto { heroe, oro, cargado }` con `alijo` = id del artefacto en `valores`, solo para el héroe que lo abre.
- **«A la vista»:** la consulta `Nucleo::alijos_a_la_vista(heroe, t)` (si puede buscar, columna en el mapa a `VISION.jugadorSolo`, sin abrir). Su proyección en `HEROE`, renovada en el muestreo, es de 6G y no se hizo.
- **Contrato:** `abrir_alijo` mapeada (el héroe de la cuenta; `alijo_id` es el id del artefacto, resuelto por `Consulta::alijo`; uno desconocido se rechaza sin rastro), su fila en la matriz de Proy §5.1 (y la de §5.2 sacada de las de campamento, C7), cuatro rechazos en el catálogo, `Heroe.alijos_abiertos = 22` (ya estaba en el esquema) lleno en `HEROE` y `ADMIN`, dos dorados nuevos (`orden_abrir_alijo`, `foto_heroe_con_alijos_abiertos`). Sin cambio de esquema: `buf breaking` limpio. Quedan 4 órdenes `orden.no_disponible` (tecnología y Aedas).
- **Sin tick:** abrir no programa nada y un alijo no tiene cita; la prueba lo mira con la longitud de la cola.

**Pruebas** (`crates/nucleo/tests/alijos.rs`, 5; las unitarias de las reglas son las de `tecnologia::alijos`, 6A): ver y abrir una vez por héroe, otro héroe abre el mismo, cada rechazo sin rastro (también con plaza en la Facción), el carro casi lleno y lleno (conservación: entra exactamente lo cargado), la foto `BAN36` ida y vuelta con el hash y `BAN35` rechazada, y por el WebSocket real: abrir, el héroe con su alijo en `HEROE` y en `ADMIN`, `alijo.ya_abierto` y `alijo.desconocido`.

**Cifras** (tras integrar 6A, `b18a259`). `cargo test --workspace --no-fail-fast`: 1.525 pruebas en verde, 14 ignoradas, 0 fallos (99 resultados de prueba; antes de integrar, 1.462). Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): 545 sin fallos. Las pruebas de sesión (los 15 binarios con pruebas por el WebSocket real, 101 pruebas) 20 veces seguidas: 2.020 sin un fallo. `buf lint` y `buf breaking` limpios; dorados verdes en Rust, C# (83) y TypeScript (83). El paso de minuto no se midió: la tanda no lo toca.

**Dudas para el autor (6F).** (1) `alijo.lejos` también cuando el héroe no va en una columna en el mapa (dentro de un lugar no se abre, como el TypeScript). (2) El hecho lo oyen el héroe y los participantes de su columna, porque el oro entra en el carro común. (3) Un alijo desconocido se rechaza en la traducción del contrato, sin pasar por el núcleo, como los demás ids. **Decidido por el autor (2026-10-25):** (1) sí, solo desde el mapa y
junto a él; (2) solo lo oye el héroe que lo abre (cambiado: el hecho va a `heroes`, no a la columna); (3) sí.

## Avance de la crónica (6C, infraestructura)

Hecho el 2026-10-25 en la rama `tanda-6-cronica`, adelantado del resto de 6C porque no depende de las reglas de tecnología. **Sin cambio de foto** (sigue `BAN35`): la crónica no entra en la
foto ni en el hash, y la versión y el hash son los mismos con ella y sin ella (lo comprueban las pruebas contra el mismo guion en memoria). Esquema aditivo y dos dorados nuevos (84 pares con los dos de 6F).

**Almacén** (`crates/persistencia/src/diario.rs`, `persistencia::Diario`). Un `Registro` propio (los mismos segmentos con longitud + CRC32) en `<partida>/cronica/`. Cada entrada lleva en sus datos
la versión que la produjo (8 bytes) y el `EventoDominio` `cronica.entrada` codificado, el mismo que viaja en las `Delta`; la secuencia del registro es su contador propio, sin huecos. Índice en
memoria: una marca (versión, secuencia, segmento, desplazamiento) cada 256 entradas, que se rehace leyendo el diario una vez al abrirlo. `Registro` gana `anadir_ubicado` (dice dónde quedará cada
entrada) y `confirmar` (el volcado de `anadir_lote`); nada cambia para el registro de órdenes.

**Escritura** (`crates/nucleo/src/nucleo.rs`, `nucleo/cronica.rs`). Solo en el núcleo persistente: al cerrar cada versión, sus entradas se proyectan y codifican (`apuntar_cronica`, ≈ 0,7 µs cada
una; las versiones sin entradas no hacen nada) y quedan en una cola que `volcar_log` manda al hilo de las fotos, ahora **hilo de la persistencia**, por el mismo canal que las fotos: así toda entrada
con versión ≤ la de una foto está escrita antes que la foto (R4), y el hilo sincroniza el diario antes de escribirla. El hilo agrupa en un solo volcado lo que encuentra esperando. Si el diario no se
puede escribir, el hilo deja de hacer fotos (aviso por `stderr`): al reiniciar se vuelve a una foto anterior y la repetición rehace lo que falte, sin huecos.

**Recuperación.** `Nucleo::con_persistencia` abre el diario con la versión de la foto que carga (0 sin foto): repara la cola dañada como el registro (avisa), **trunca lo posterior** y la repetición del
registro de órdenes lo vuelve a escribir igual. Si el diario no estaba y había foto, avisa (`Aviso::DiarioPerdido`) y sigue.

**Lectura** (contrato 05 §8). `PedirCronica { peticion_id, desde_version?, hasta_version?, limite }` → `Cronica { peticion_id, entradas, mas_antiguas, mas_recientes, version }`: hacia atrás hasta
una versión (o desde la última) o hacia delante desde ella; 200 por defecto, 1.000 como mucho; **una página nunca parte las entradas de una versión**. El actor la pasa al hilo de la persistencia
(`Nucleo::pedir_cronica`), que la contesta detrás de lo ya escrito: no toca el estado ni bloquea el actor, y la página llega hasta la versión del mundo de ese momento. La respuesta va directa al canal
de la sesión; en el servidor cuenta como pendiente, como una orden. Sin persistencia, página vacía.

**Pruebas.** `persistencia` (3 nuevas y 1 medición): páginas contra el recorte hecho a mano con 7 límites y 10 combinaciones de desde/hasta, cruzando segmentos, y encadenadas hacia atrás y hacia
delante sin repetir ni saltar; truncar al abrir y volver a escribir; cola cortada reparada con aviso; diario perdido. `nucleo/tests/cronica.rs` (4 nuevas y 1 medición): guardar, apagar, recuperar y
leer igual que la referencia en memoria, con la misma versión y el mismo hash; **tres cortes tras una foto** (sin apagar, con la crónica por delante de la foto; con el lote siguiente perdido; con el
lote a medias), y en los tres la crónica de la referencia, también al seguir jugando; las páginas de 200 por defecto, el tope de 1.000 y el encadenado; **por el WebSocket real**, quien llega tarde
lee todo, una entrada nueva está en la página en cuanto su orden tiene resultado (la misma que llega en la `Delta`) y tras parar y recuperar la misma. `servidor/tests/sesion.rs` (1): `PedirCronica`
llega y cuenta como pendiente. `contrato` (1): el significado de los dos dorados. TS y C#: 83 dorados cada uno. `cargo test --workspace --no-fail-fast`: 1.461 pruebas en verde (97 resultados, 16 ignoradas); las 105 pruebas de sesión (`admin`, `ambitos`, `batalla_unity`, `cliente_de_ejemplo`, `cronica`, `eventos`, `interior`, `ordenes`, `ordenes_mundo`, `politica`, `propios`, `reanudacion`, `sesiones`, `vista` y `servidor/tests/sesion.rs`) 20 veces seguidas: 2.100 ejecuciones, 0 fallos; `buf lint` y `buf breaking` limpios.

**Mediciones** (release, con otras compilaciones en la máquina a la vez: ruido de ±20 %):

| Qué | Cifra |
|---|---|
| Diario solo: 100.000 entradas de 120 bytes (15,4 MB) | escribir 88 ms; abrir e indexar 105 ms; página de las 200 últimas 0,38 ms, de 1.000 hasta la mitad 0,87 ms |
| Por el núcleo: 200.000 órdenes que dejan 100.003 entradas | 13,3 MB (132 bytes por entrada); página de las 200 últimas 0,66 ms y de 1.000 hasta la mitad 1,6-2,1 ms (con la decodificación y el canal); recuperar 0,22 s (antes 0,11-0,14 s: abrir e indexar el diario) |
| Coste por orden (las mismas 200.000 órdenes persistentes, una de cada dos canta) | base 2,14-2,68 s, nueva 2,50-2,79 s (media +0,26 s: ≈ 2,6 µs por entrada; nada en las órdenes sin entrada) |
| Paso de minuto (`asentamientos rendimiento`, 1.000 plazas × 3 días, sin persistencia) | base 28,7 y 27,1 s, nueva 27,1 y 25,9 s; los mismos 3.061.541 pasos y la versión 50.408 |

**Dudas para el autor** (también en la fila de paridad «Crónica (6C, infraestructura)»):

1. **El directorio** va dentro del de la partida (`<partida>/cronica/`), no al lado como decía §5.3: una partida es un solo directorio y dos partidas en la misma carpeta no se pisan.
2. **La forma guardada** es el `EventoDominio` codificado (≈ 132 bytes por entrada; 100.000 entradas, 13 MB), no el binario fijo de ≈ 40 bytes de §5.3: un solo formato, el mismo que en vivo, y
   sin códec propio. Si una season da muchas más entradas de las previstas, se puede compactar el formato sin tocar el contrato.
3. **Quién escribe**: el hilo de las fotos (ahora de la persistencia), no el núcleo, que es quien sincroniza el registro de órdenes y lo hace en el camino de la orden. Así la orden no espera al
   disco de la crónica y el orden respecto de las fotos sale del canal.
4. **Un fallo al escribir la crónica deja de hacer fotos** en vez de abortar como el registro de órdenes: el mundo sigue y al reiniciar se rehace lo que falte; el precio es una repetición más
   larga. **Decidido por el autor (2026-10-25): se queda así.**
5. **La historia perdida antes de la foto** (el diario borrado o dañado antes de ella) solo se avisa: no hay herramienta para recomponerla, y no siempre se podría, porque `compactar` borra los
   segmentos del registro de órdenes anteriores a la foto más antigua. **Decidido por el autor (2026-10-25): lo más eficiente para el servidor, así que
   `compactar` sigue borrando y no se conserva el registro entero; la historia anterior a la foto más antigua solo existe en el diario.**
6. **Una página puede pasar del límite** si su primera versión tiene más entradas que el límite (va entera para no partirla); en la práctica una versión trae pocas.
7. **Sin persistencia no se guarda** (pruebas y batch): las páginas van vacías. El batch político y el militar, que comprueban la foto de ida y vuelta abriendo un núcleo persistente sobre una
   foto suelta, reciben el aviso `DiarioPerdido` (no lo miran).
8. **La página y las `Delta` se solapan**: la página llega hasta la versión al pedirla y las `Delta` siguen llegando; el cliente reconoce las repetidas por la versión.
9. **Lo que falta de la crónica en 6C**: los cuatro públicos del sistema 6 (`tecnologia.logro`, `era.comienza`, `aedas.canta_descubrimiento`, `aedas.epica_cumplida`), que entran tal cual cuando
   existan sus reglas (6B-6E); y `primera_en_llegar` sigue recorriendo las plazas (D16, 6B).

## Avance de 6B (2026-10-25, rama `tanda-6b`, foto `BAN37`)

Hecha sobre `b18a259` (6A) e integrada con 6F (`BAN36`, orden 174) y la crónica (sin foto). Sin comandos ni órdenes nuevas, sin cambio de esquema ni dorados (la proyección es de 6G).

**Estado.** `Estado::tecnologia` (el `EstadoTecnologia` del crate) sustituye a `EstadoMilitar::tecnologias`; `Estado::era_en` dice si la cita de la Era está en la cola. Codec propio
(`crates/nucleo/src/codec_tecnologia.rs`, desestructurado entero) al final del estado, antes de las generaciones. `ReglasEconomia::tecnologia` lleva las reglas del crate (sus bloques ya entraban en la
huella con los militares). `EstadoTecnologia` gana `restos` (milésimas de la unidad siguiente) y `sumar_milesimas`: lo extraído y lo fabricado llegan de la economía en milésimas por minuto, no en
unidades (la decisión técnica 17 suponía unidades), y el contador sube una unidad cuando el resto la completa (a trozos o de una vez da lo mismo). `Conjunto::iter` recorre solo los bits puestos.

**Contar en origen** (`crates/nucleo/src/nucleo/logros.rs`; la tabla de vías está en su cabecera y en TA §2.1):

| Vía | Dónde |
|---|---|
| lo extraído (D17) | `FuentesMundo::extraer` anota lo que sale del nodo por contador, en plazas y pueblos; `con_exterior` lo suma al acabar el paso |
| lo fabricado | gancho nuevo `Fuentes::fabricado` (método por defecto vacío del trait de `asentamiento`), al que `avanzar_minuto` pasa lo que devuelven las recetas, en vez de cambiar el tipo que devuelve `avanzar_minuto` (13 llamadas) |
| columna contra columna con números | `combate_resuelto(hoplitas)` + `batallas.campoAbierto` (su `combate.encuentro`) |
| caravana interceptada, acoso que ganan los bandidos, Caravana de Fundación capturada | `caravanas.destruidasOCapturadas` |
| campamento de bandidos destruido | `bandidos.campamentosDestruidos` |
| asedio con números (al acabar su convocatoria) | `contador::asedio(conquista, enCombate = hubo tirada, conResidentes = alguna escuadra defensora fuera de la guarnición, muralla)`; `batalla.aplicada` no suma (D18) |
| batalla de Unity | `aplicar_resultado` con `contadores_de_batalla`, la muralla mirada **antes** de las consecuencias (la conquista la daña); avanzada y saqueo de pueblo: solo `batallas.libradas` (D10) |
| reclutar en plaza (reponer incluido) y en campamento (Q4) | `contador::reclutamiento` con el edificio de la tropa; ni la guarnición de serie ni la tropa prestada |
| animal comprado | `AnadirAnimal` |
| `plazasEnNivel.N` | índice derivado `Espacio::plazas_por_nivel` (no va en la foto; lo reconstruye `reconstruir_espacio` y lo vigila la auditoría): `con_exterior` compara el nivel actual antes y después, más fundar, `FijarNivelAsentamiento` y la ruina; el récord solo al subir |

**Logros y Era.** Al fijar un logro, `tecnologia.logro { umbral; contador como texto }` público y se mira la Era. La Era tiene **una cita global** (`Carga::Era`, entidad `18·ETIQUETA`, prioridad 10)
armada como la de los títulos (al pasar algo en el mundo con alguna Facción, si no hay otra) en el fin del plazo de la vigente, o en el mismo instante si la vigente ya tiene todos sus logros; al
cambiar de Era sube su generación y se rearma; la III no tiene cita (decisión 15). `era.comienza { orden, porPlazo; era como texto }` público. El `Exterior` de las plazas lleva el `orden` real
(`Nucleo::orden_era`). Consultas nuevas: `Nucleo::{tecnologia, orden_era, plazas_en_nivel}`.

**`FijarTecnologias`** queda como administración sobre las reglas reales (decisión 8): las dadas aparecen (`hacer_aparecer`, con su descubridora si nadie la tenía) y las adoptadas son las de
arranque más las dadas, sin tarifa; aparecer es para siempre. Traslado (5F, decisión 13): en la fusión la Facción nueva tiene la unión de los tres conjuntos; en la anexión las de la absorbida se
pierden; las descubiertas por la que desaparece pasan a la que la recoge. La proyección `TecnologiasFaccion.adoptadas` sale del estado nuevo con el mismo orden de antes.

**Rendimiento.** El primer intento empeoró el paso de minuto un 7-14 % (`tiene_tecnologia` buscaba el id en el catálogo entero en cada receta y cada candidata de la auto-construcción). Ahora se
compara solo con las adoptadas de la Facción (`estado::adopta`) y la suma de los flujos no clona las reglas. `rendimiento_de_mil_plazas_tres_dias` (1.000 plazas × 3 días, release), alternando
binarios con la máquina compartida con otros agentes: base 27,2 / 25,7 / 25,7 s, 6B 26,2 / 35,2 (ruido) / 25,2 s; mismos 3.061.541 pasos y la misma versión 50.408 (en ese escenario no se cumple
ningún logro).

**Pruebas.** `cargo test --workspace --no-fail-fast` (una pasada; la tanda no toca la sesión ni el contrato): antes de integrar 1.527 en verde (99 resultados, 14 ignoradas; +8), tras integrar 6F y la
crónica 1.542 (101 resultados, 16 ignoradas), 0 fallos. Auditoría `BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`: 555 sin fallos. Una prueba por vía, cada una con su hecho
y comprobando que cuenta una vez y nada más (`comun::logros_de_hechos`): extracción en `pueblos.rs` (lo que vuelca el carro no se cuenta otra vez), el gancho de fabricación en
`asentamiento/tests/simulacion.rs`, columnas, caravanas, acoso y campamento con números en `combate.rs`, el asedio con convocatoria con combate y sin él (con el primer logro, umbral 1) en
`murallas_asedio.rs`, Unity (campo abierto, asedio resistido y conquistado con muralla, campamento, caravana) en `batallas.rs`, reclutar en plaza y en campamento en `heroes.rs`, animales en las
caravanas de `combate.rs` y `batallas.rs`; y en `tests/logros.rs` la Era por plazo y sin cita en la III, nada en un mundo sin Facciones, la Era normal y a trozos, la foto `BAN37` (cada campo
cambia el hash, `BAN36` se rechaza con su motivo), `FijarTecnologias` y el récord de plazas por nivel. En `tests/nucleo.rs`, un mundo quieto con Facción solo deja en la cola la cita de la Era.

**Filas de paridad** cerradas o reescritas: «Población y construcción» (tecnologías por administración), Murallas tercera tanda punto (2), «Militar y combate (4E)» (contadores de Unity),
«Economía y comercio (3B-1)» (`animales.comprados`), «Militar y combate (4B-1)» (`FijarTecnologias`), «Eventos (4H, T5)» (`tecnologia.logro`, `era.comienza`); fila nueva «Tecnología (6B)».

**Dudas para el autor**

1. **Una Era que empieza con todos sus logros ya cumplidos** da paso a la siguiente en una cita del mismo instante (otra evaluación: «una Era por evaluación»). El TS lo hacía en el tick siguiente.
   Recomendado: así.
2. **D16 no se cierra**: `primera_en_llegar` de la crónica mira el nivel **alcanzado** (decisión 5) y el índice nuevo es del nivel **actual**; sigue recorriendo las plazas en cada
   `asentamiento.nivel_subio` de nivel ≥ 3 (raro). Recomendado: dejarlo; si hiciera falta, un segundo índice por nivel alcanzado.
3. **La Caravana de Fundación capturada** cuenta en `caravanas.destruidasOCapturadas` (deja el mismo hecho que cualquier caravana). Recomendado: que cuente.
4. **`tecnologia.logro` y `era.comienza` aún no entran en la crónica**: es de 6C (TA §8), con los otros dos públicos.
5. **La fabricación** se prueba en el gancho del crate `asentamiento` y en `tecnologia::contador` (las cuatro recetas), no con un taller de bronce dentro del núcleo (montarlo pide nivel, artesanos
   y tecnología; lo cubrirá el batch de 6H).

## Avance de 6C (2026-10-25, rama `tanda-6c`, foto `BAN38`)

Hecha sobre `9a55217` (6A, 6B, 6F y la infraestructura de la crónica ya integradas). Orden nueva `AdoptarTecnologia` (**175**, el siguiente libre tras 174); sin cambio de esquema ni dorados nuevos (la proyección del estado de tecnología es de 6G).

**Aparición por disparadores** (`crates/nucleo/src/nucleo/tecnologias.rs`, §5.1). Lo que cambia algo que mira un hito marca la Facción en `Nucleo::aparicion_pendiente` (transitorio: no va en la foto, se vacía al cerrar cada versión y una orden que no se acepta lo descarta); al cerrar la versión, a cada Facción marcada que siga viva le aparecen sus candidatas con el hito entero (`EstadoTecnologia::listas_por_hito`, nuevo en el crate: la vista del núcleo lee el mismo estado, así que primero se listan y después se hacen aparecer). Marcan: `tras_logros` y `mirar_era` (todas las vivas), `anotar_sucesos` (edificio completado o mejorado, nivel subido), `contar_nivel` (el nivel actual sube; gana la Facción como parámetro), `agrandar_zona`, `situar_plaza` y `cambiar_plaza_de_faccion` (fundar, conquistar, anexión, fusión, ruina: las dos Facciones), `DesignarCapital`, la adopción y `FijarTecnologias`. El paso de minuto de una capital mira `recursoEnCapital` (`mirar_recurso_en_capital`): primero, sin buscar nada, si alguna de `Reglas::piden_capital` (nuevo, hoy solo la aleación del bronce) tiene su logro; después su hito con la vista; solo entonces marca. La vista del hito (`VistaDeFaccion`) recorre las plazas de la Facción por el índice de `Espacio` y busca la capital una vez si hace falta; `yacimientoEnTerritorio` mira los nodos de ese recurso de las rejillas del mapa dentro del disco de cada plaza y luego su zona real. `tecnologia.aparece { faccion, primera; tecnologia }` es privado, atribuido a la capital, en la misma versión que su causa.

**Citas de cada descubrimiento.** `Nucleo::registrar_aparicion` (la usan el hito y `FijarTecnologias`; la usarán la compra de 6D y la épica de 6E) programa, cuando la Facción es la descubridora, dos citas `Carga::Tecnologia(SucesoTecnologia::{Conocimiento, Campamento})` de prioridad 10 con la entidad `entidad_tecnologia(t)` (etiqueta 19): a `primeros.en + 24 h` cantan los Aedas (`aedas.canta_descubrimiento { faccion; tecnologia }`, público; ahí irá la revelación de 6D, `cita_de_tecnologia`) y a `primeros.en + 72 h` se marcan todos los campamentos. Vencen una vez; ningún otro evento por la tecnología (la prueba lo mira en la cola). Los Aedas cantan también al fijarse un logro o empezar una Era.

**Adopción.** `AdoptarTecnologia { heroe, faccion, tecnologia: u8 }` (índice del catálogo; el contrato lo traduce con `Consulta::tecnologia` y uno desconocido sale `tecnologia.desconocida` en la traducción), `Autoriza::ReyDeLaFaccion`, implica al héroe (candado de batalla) y nombra a la Facción. Rechazos: `faccion.no_existe`, `no_autorizado` (no es su Rey), `tecnologia.desconocida` y los cinco finos de §7.9 en el orden del TS (`sin_capital`, `rey_fuera_de_la_capital`, `ya_adoptada`, `no_aparecida`, `sin_tarifa`, con las reglas de `tecnologia::adopcion` de 6A). La tarifa sale del almacén de la capital efectiva en unidades enteras; `tecnologia.adoptada { faccion, capital; tecnologia }` es privado, atribuido a la capital; toca la capital, la Facción y los campamentos, marca la aparición y deja `epicas::Hecho::adopcion` en `Nucleo::hechos_epicos` (se descarta al cerrar la versión hasta 6E). La orden del contrato `adoptar_tecnologia` ya no da `orden.no_disponible`: quedan 3, las de los Aedas.

**`factor_tecnologia` real.** El `Exterior` de cada plaza lleva `tecnologia::adopcion::factor_produccion` sobre las adoptadas de su Facción (con `Reglas::con_bonus`, nuevo: sin ninguna adoptada con bonus no mira nada): `canteria` ×1,5 la piedra de sus Canteras y de sus pueblos, en ppm, antes del resto oculto. `SIN_BONUS` desaparece.

**Campamentos.** `tecnologia_del_campamento` usa `EstadoTecnologia::tecnologias_del_campamento` (6A) con las vivas del índice: arranque, o descubridora y 72 h, o la mitad de las vivas con ella **adoptada**. No se guarda; lo marcan la cita de campamento, la adopción, `FijarTecnologias` y que cambie el número de Facciones vivas. La huella de un campamento en la auditoría mira la parte que no depende del plazo (`Instante::MIN`): la del plazo cambia sola con el tiempo y la marca su cita (si la mirase, un suceso del mismo instante anterior a la cita la vería ya cambiada).

**Crónica.** `tecnologia.logro` (contador y umbral), `era.comienza` (Era, orden y si llegó por plazo) y `aedas.canta_descubrimiento` (descubridora y tecnología) dejan su `cronica.entrada`, como los títulos, y así entran en el diario. C4 queda cerrada con la regla de Rust. De paso: el `orden` de una Era se traducía en el payload como id de orden de mercado; ya viaja como número.

**Foto `BAN38`**: las dos cargas nuevas en la cola (códigos 17 y 18); `cantadas` ya iba en BAN37. BAN37 se rechaza con su motivo (sus descubrimientos no traen las citas). Las aserciones de foto de las pruebas y los textos del batch, al día.

**Rendimiento** (`rendimiento_de_mil_plazas_tres_dias`, 1.000 plazas × 3 días, release, `--exact`, alternando los binarios): base 20,11 / 20,88 / 20,29 s, 6C 20,45 / 20,25 / 20,61 s (medias 20,43 y 20,44); los mismos 3.061.541 pasos y la misma versión 50.408. Un primer intento, que buscaba la Facción en el paso de cada capital antes de mirar si había logro, daba +1,9 %.

**Pruebas.** Nuevas en `crates/nucleo/tests/tecnologias.rs` (10; las reglas cambian el logro de las tecnologías que miran por `plazasEnNivel.2 ≥ 1`, que cumple `FijarNivelAsentamiento`): el logro sin hito no hace aparecer nada, la Cantera completada sí y en su misma versión, y la Era cerrada espera a la suya (aparece en la versión de `era.comienza`); `recursoEnCapital` en el paso de minuto de la capital y no al dar el recurso; el canto a las 24 h, una vez, con la descubridora y en la crónica; las dos citas en la cola y ninguna al vencer; el campamento a las 72 h sin nadie con ella adoptada y la cita que marca los campamentos; la adopción con cada rechazo sin rastro y la tarifa exacta; la Cantera ×1,5; la foto `BAN38` ida y vuelta con las citas y `BAN37` rechazada; el mismo mundo de una vez, a trozos y con una foto entre el descubrimiento y sus citas; y por el WebSocket real, `tecnologia.desconocida`, `rey_fuera_de_la_capital`, la adopción con su hecho en la `Delta` de la Facción y `ya_adoptada`. En el crate, `listas_por_hito` con `piden_capital`, `todas`, `con_bonus` y `Reglas::indexar` (lo derivado del catálogo, también para las pruebas que lo cambian). `cargo test --workspace --no-fail-fast`: 1.552 pruebas en verde (102 resultados, 16 ignoradas, 0 fallos). Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): 565 sin fallos. Los 19 binarios con pruebas por el WebSocket real (`a_mano`, `admin`, `alijos`, `ambitos`, `batalla_unity`, `carga`, `cliente_de_ejemplo`, `cronica`, `eventos`, `fundacion`, `interior`, `ordenes`, `ordenes_mundo`, `propios`, `reanudacion`, `sesiones`, `tecnologias`, `vista` y `servidor/tests/sesion.rs`) 20 veces seguidas: 380 ejecuciones, 2.640 pruebas, 0 fallos. `buf lint` y `buf breaking` limpios (sin cambio de esquema).

**Dudas para el autor (6C)**

1. **La aparición se evalúa al cerrar la versión**, con todo lo que cambió la transición; el TS la evaluaba en el tick. Así el `tecnologia.aparece` va en la versión de su causa. Recomendado: así.
2. **Los Aedas cantan también al fijarse un logro o empezar una Era** (una tecnología con descubridora antes que logro, por administración, o de una Era cerrada), además de en su cita. Recomendado: así.
3. **«El Rey en la capital» es estar dentro**; con su columna a la puerta se rechaza. Recomendado: así (es lo que miraba `estaEnAsentamiento`).
4. **`recursoEnCapital` espera al siguiente paso de minuto de la capital**: lo que llega por una orden o una caravana a mitad de minuto aparece al minuto siguiente. Recomendado: así (el plan lo pone en el paso de minuto).
5. **`FijarTecnologias` programa las dos citas** al registrar la descubridora (decisión 8): en los batches que dan las de la Era I por administración, los campamentos las desbloquean a las 72 h y los Aedas no las cantan (no tienen logro). Recomendado: así hasta que 6H pase los batches a la vía de juego.
6. **La cita de campamento marca todos los campamentos** aunque ya la desbloquease la mitad de las vivas. Recomendado: así (una vez por tecnología).

## Avance de 6D (2026-10-25, rama `tanda-6d`, foto `BAN39`)

Hecha sobre `a5da8bb` (6A, 6B, 6C, 6F y la crónica integradas), en paralelo con 6E. Orden nueva `ComprarTecnologiaAeda` (**176**); sin cambio de esquema ni dorados nuevos (la proyección de los Aedas es de 6G).

**Estado.** `Estado::aedas: Vec<tecnologia::itinerantes::AedaItinerante>` (6A), por su número (`aeda-N` es la posición N − 1), en la foto tras la cita de la Era (`codec_tecnologia::{escribir_aedas, leer_aedas}`, desestructurado entero). Sin índice derivado: los Aedas son pocos (`max(3, ⌈vivas/3⌉)`, 334 con 1.000 Facciones) y se recorren enteros solo cuando desaparece una plaza o se cierra una puerta.

**Un evento por Aeda** (`crates/nucleo/src/nucleo/aedas.rs`). `Carga::Aeda` (código 19 en la cola), prioridad 10 (decisión técnica 2), entidad `entidad_aeda(N)` (etiqueta 20): si va de camino, su llegada; si está parado, el fin de su estancia. Al llegar se detiene `estanciaMinutos` y la plaza se marca para revelar; al acabar la estancia parte (`partir`): las candidatas de 6A (las plazas que lo aceptan, frescas primero, de cerca a lejos), y hacia la primera a la que hay camino con el A* de las caravanas y la atracción de la red (`Nucleo::camino_por_la_red`), los tramos de camino rebajados (`costes_efectivos`) y `rutas::a_plan_por_minuto` a `velocidad` (20 u/min, la del TS, Q3); sin camino, otra estancia. Su evento solo se invalida (sube la generación) cuando deja de valer: le cierran la puerta, o desaparecen su plaza o su destino. El evento de un Aeda **no arma el plazo de los títulos ni la cita de la Era** (su viaje no cambia nada de lo que miran): sin esa excepción el plazo de los títulos se movía y la versión del escenario de rendimiento pasaba de 50.408 a 50.409.

**Nacer.** `nacer_aedas` en `tras_ganar_o_perder_plaza` (fundar, conquistar, anexión, fusión, ruina) y al abrirse una puerta: si el objetivo pasa de los que hay, nacen los que faltan con `itinerantes::nacer` (6A), ya parados, y sus plazas se marcan para revelar. O(1) cuando no falta ninguno. Nunca se eliminan.

**La puerta (Q5).** `fijar_puerta` llama a `puerta_de_los_aedas`: cerrada al grupo `aedas`, cada Aeda parado en la plaza sale al instante (`echar_aeda`: sube su generación y parte desde la plaza); abierta, pueden nacer los que faltaban. La conquista devuelve la puerta a la de por defecto, que los admite. El que llega a una plaza que ya no lo acepta no entra y parte hacia otra (`// DUDA:`). Si su plaza cae en ruinas parte desde ella; si cae su destino, `perder_destino` y otro desde donde está.

**Revelar (§6.3).** Transitorio `Nucleo::revelar_pendiente` (plazas), como la aparición de 6C: lo marcan la llegada y el nacimiento, el cambio de dueño de la plaza (`cambiar_plaza_de_faccion`) y `cantar` cuando canta algo (el único momento en que los Aedas pasan a conocer algo: la cita de conocimiento de 6C, un logro que se fija, una Era que empieza). Al cerrar la versión, **después de las apariciones**, en cada plaza marcada con un Aeda parado la Facción dueña recibe lo que el Aeda conoce y ella no tiene aparecido ni revelado: `aedas.revela { faccion, descubridora; tecnologia, faltan }` (privado de la plaza; `faltan`, lista de índices del hito, decisión 12) y se toca la Facción. El crate gana `EstadoTecnologia::por_revelar` (sin marcar): la vista del hito lee el mismo estado.

**Vender (§6.4).** `ComprarTecnologiaAeda { heroe, asentamiento, tecnologia: u8 }`, clase nueva de autorización `Autoriza::CompraTecnologiaAeda` (la cuenta del héroe, dentro de la plaza y Rey de su Facción o su Gobernador titular: `politica::autorizacion::Actor::compra_tecnologia_aeda`, que ya existía), implica al héroe (candado de batalla). Rechazos, en este orden: `asentamiento.desconocido`, `no_autorizado`, `aedas.no_hay_aeda`, `tecnologia.desconocida` y los cuatro de `tecnologia::adopcion::vender` (6A) en el orden del TS: `aedas.era_no_vendible`, `aedas.no_la_conoce`, `aedas.ya_aparecida`, `aedas.sin_oro`. Paga el almacén de la plaza (2 × el oro de la tarifa, en unidades enteras) y le aparece a su Facción dueña (decisión 16); `aedas.venta { faccion, aeda, precio; tecnologia }` privado de la plaza; toca la plaza y la Facción. La orden del contrato `comprar_tecnologia_aeda` deja de dar `orden.no_disponible` (quedan 2, las épicas de 6E); su fila en la matriz de Proy §5.1 y los cinco rechazos en el catálogo (regenerado).

**En la visión** (decisión 14). `GeneradorVision` los muestrea como móviles de nadie (`espacial::BANDERA_AEDAS`), por su plan o fijos; no entran en la rejilla de encuentros. La `VISTA` es de 6G.

**Foto `BAN39`**: los Aedas y su carga en la cola; `BAN38` se rechaza con su motivo. Las aserciones de foto de las pruebas y los textos del batch, al día.

**Rendimiento** (`rendimiento_de_mil_plazas_tres_dias`, 1.000 plazas × 3 días, release, `--exact`, alternando los binarios, con la máquina compartida con otros agentes: ruido grande): base 24,36 / 37,02 / 23,60 s, 6D 32,32 / 32,61 / 21,15 s (la primera medida, aún con el evento armando los títulos, 24,70 s). La versión, la misma (50.408); los pasos, 3.061.541 en la base y 3.069.110 en 6D: los 7.569 de más son los dos eventos de los Aedas (sus llegadas y fines de estancia en tres días); el paso de minuto no cambia (nada de los Aedas está en él).

**Pruebas.** Nuevas en `crates/nucleo/tests/aedas.rs` (11; las de 6A en `tecnologia::itinerantes` siguen siendo las reglas puras): nacen el mínimo con la primera plaza, ya parados, y no nacen más con la segunda Facción; el recorrido de tres días por las dos plazas, comprobando cada hora que **cada Aeda tiene exactamente un evento vigente en la cola, el suyo, en el instante de su siguiente evento** (ninguno ocioso); la plaza que les cierra la puerta no los recibe y el cierre por defecto no cuenta; **cerrarles la puerta con un Aeda dentro lo hace salir al instante** (su plan empieza en ese instante, el evento viejo deja de valer) y lo revelado queda revelado, y sin otra plaza que los acepte quedan varados esperando otra estancia; la ventaja del primero (nada antes de las 24 h, a las 24 h exactas una sola revelación aunque haya tres Aedas en la plaza, con la descubridora y `faltan = [0]`, revelada y no aparecida); no se revela a quien ya la tiene aparecida, y al llegar se revela lo que ya conocían; comprar cuesta 200 de oro (2 × la tarifa de la Era I), sale del almacén de la plaza, la tecnología aparece sin hito ni adopción y deja de estar revelada, la descubridora no cambia, y después se adopta; cada rechazo sin rastro (versión y hash iguales); la foto `BAN39` ida y vuelta (cada campo de un Aeda cambia el hash, su evento viaja en la cola) y `BAN38` rechazada con su motivo; el mismo mundo de una vez, a trozos y con una foto y un apagado a mitad; y por el WebSocket real, la compra con `tecnologia.desconocida`, `no_autorizado` en una plaza ajena, el hecho `aedas.venta` en la `Delta` de la Facción y `aedas.ya_aparecida`. Portan `aedas.test.ts` (los 10: cuántos y nacen, recorrido, la puerta, revelar 3, vender 3) y la parte de la compra de `comandosTecnologia.test.ts` (revelada y comprada, rechazo sin Aeda, solo el Rey o el Gobernador presentes). `cargo test --workspace --no-fail-fast`: 1.563 pruebas en verde (103 resultados, 16 ignoradas, 0 fallos). Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): 576 sin fallos. Los 19 binarios con pruebas por el WebSocket real de `crates/nucleo/tests` (los 18 de 6C y `aedas`) y `servidor/tests/sesion.rs` 20 veces seguidas: 400 ejecuciones, 2.860 pruebas, 0 fallos (una primera tanda, sin `--no-fail-fast`, dio 2 fallos de 20 en `ordenes_mundo::un_jugador_nuevo_crea_su_heroe_pide_tropa_sale_del_campamento_y_marcha_por_el_contrato`, que no tiene plazas ni Aedas: depende del reloj real con la máquina cargada; aislada, 40 de 40). `buf lint` y `buf breaking` limpios (sin cambio de esquema).

**Dudas para el autor (6D)**

1. **El que llega a una plaza que le cerró la puerta mientras iba de camino** no entra y sigue hacia la siguiente; el TS solo miraba la puerta al elegir destino y lo dejaba entrar. Recomendado: así (la regla de Q5, «sale quien ya no puede entrar», aplicada a la entrada).
2. **Expulsado sin otra plaza que lo acepte**, el Aeda queda varado junto a la plaza (sin plaza, sin comprarle nada) y vuelve a buscar destino al cabo de una estancia, como el TS con el Aeda sin camino. Recomendado: así.
3. **El evento de un Aeda no arma el plazo de los títulos ni la cita de la Era** (sí lo hacen todos los demás eventos): su viaje no cambia nada de lo que miran, y si los armara movería el instante de los títulos sin motivo. Recomendado: así.
4. **El plan del viaje se fija al salir**: un camino que se abre a mitad de viaje no lo acelera (el TS avanzaba cada tick con la red del momento), y no hay `pasosRio` en las plazas (el A* es el de las caravanas, con los vados del grafo). Recomendado: dejarlo (fila de paridad) hasta medir con bots en la fase 7.
5. **La revelación se hace al cerrar la versión**, como la aparición de 6C: lo que aparece en la misma transición ya no se revela. Recomendado: así.
6. **Con varios Aedas parados en la misma plaza** (nacen los tres en la primera), se le compra al primero por número (`aedaEn` del TS) y revelan una sola vez (lo mismo para todos). Recomendado: así.
7. **Los Aedas no son una clase del registro de cambios**: moverse no sube la versión ni viaja a nadie. Para 6G: proyectarlos en `VISTA` desde el muestreo, como las columnas ajenas, y en `ADMIN` con una clase propia si hace falta. Recomendado: así.
8. **`precio` de `aedas.venta` va en milésimas** (como el oro de `alijo.abierto`). Recomendado: así.

## Avance de 6E (2026-10-25, rama `tanda-6e`, foto `BAN40`)

Hecha sobre `a5da8bb` (6A, 6B, 6C, 6F y la crónica ya integradas), en paralelo con 6D (Aedas itinerantes, rama `tanda-6d`). Órdenes nuevas `EmpezarEpica` (**177**) y `AbandonarEpica` (**178**; el 176 es de 6D); sin cambio de esquema ni dorados nuevos (los residentes en la proyección son de 6G). Código en `crates/nucleo/src/nucleo/residentes.rs`; las reglas puras son las de `tecnologia::{residentes, epicas}` (6A), sin cambios en el crate.

**Estado.** `Estado::residentes` (el `Residentes` del crate: los Aedas con su épica, el contador de ids `aeda-rN`, la espera de cada plaza con cupo libre y las épicas cumplidas por Facción) y `Estado::residentes_citas` (el instante de la cita de llegada de cada plaza que espera). Codec en `codec_tecnologia::{escribir,leer}_residentes`, desestructurado entero, al final del estado tras `era_en`. Índice derivado `Espacio::residentes_por_plaza` (no va en la foto; lo rehace `reconstruir_espacio` y lo vigila la auditoría): el paso de minuto lee de ahí cuántos residentes tiene la plaza sin recorrerlos.

**El cupo, sin tick.** `revisar_residentes(plaza)` (idempotente: ajusta al cupo, despide a los últimos en llegar con `aedas.residente_se_va`, hace llegar a uno si ya le toca con `aedas.residente_llega` y deja en la cola la cita que corresponda o ninguna) se llama solo cuando el cupo puede cambiar: el paso de minuto si la nobleza pasa de 0 a más (o al revés) o cambia el nivel actual (comparados antes y después del paso, sin buscar el Palacio), un Palacio completado (`anotar_sucesos`), `FijarNivelAsentamiento`, la conquista (tras el saqueo) y un cambio de la reputación de la dueña (`programar_reputacion`, solo para sus plazas que esperan). La cita de llegada (`Carga::Residentes`, código 22, prioridad 10, entidad `entidad_residentes`, etiqueta 21; la etiqueta 20 y los códigos 19-21 quedan para 6D) es **una por plaza y solo mientras espera**: a `espera + llegada`; con la reputación en el umbral bajo o por debajo, en el primero de `espera + llegada × 3` y el cruce hacia arriba del umbral (decisión técnica 6). Si el instante deseado cambia, sube la generación y se programa la nueva; sin cupo libre no hay ninguna.

**Felicidad y nobleza (D11).** `construccion::factores_extra`: `felicidad_extra = ocupación × (1 + bonoFelicidad × residentes)` y `nobleza_extra = 1 + bonoNobleza × residentes`, en ppm. Sin residentes es lo de antes (`FACTOR_EXTRA` queda solo para la recaudación).

**Las órdenes.** `EmpezarEpica { heroe, asentamiento, aeda, tecnologia }` y `AbandonarEpica { heroe, asentamiento, aeda }`, clase nueva `Autoriza::DirigeAedas` (`politica::autorizacion::Actor::dirige_aedas`: dentro de la plaza y Rey de su dueña, Gobernador o Sacerdote; no exige residir), con el candado de batalla del héroe. Rechazos: `asentamiento.desconocido`, `no_autorizado`, `tecnologia.desconocida` y los nueve finos `epica.*` (§7.9) en el orden del TS: `no_vive_aqui`, `sin_epica`, `aeda_ajeno`, `ya_canta`, `sin_logro`, `era_cerrada`, `ya_aparecida`, `otro_la_canta`; al abandonar, `no_canta`. Hechos `aedas.epica_empieza { aedaResidente, faccion, capitulo: 0, capitulos; tecnologia }` y `aedas.epica_abandonada { …; motivo: orden }`, privados de la plaza. En el contrato, `empezar_epica` y `abandonar_epica` dejan de dar `orden.no_disponible` (`aeda_id` es `aeda-rN`; uno sin esa forma sale `epica.no_vive_aqui` en la traducción). Queda una orden no disponible: `comprar_tecnologia_aeda` (6D).

**Los hechos de épica, en origen.** `Nucleo::hecho_epico` anota un `epicas::Hecho` tipado donde ocurre: la defensa o la conquista en `resolver_asedio_con_numeros` (con `enCombate` = hubo tirada; no desde `batalla.aplicada`, D18), la `batalla` en el encuentro entre columnas (las banderas de las dos), la batalla de Unity en `aplicar_resultado` (por su id y con las Facciones del ticket: campo abierto, asedio resistido o conquistado), la obra (`EdificioCompletado`, y `MejoraCompletada` con el tipo del edificio) y el ascenso (`NivelSubio`) en `anotar_sucesos`, la adopción (6C) y la caravana comercial que llega con carga ≥ 100 (en la llegada real y en `entregar_contenido`; D9 se cierra sin tocar el hecho de comercio). Al cerrar la versión, **antes** de la aparición por hito, `avanzar_epicas` los pasa al crate: a lo sumo uno por Aeda, con el enfriamiento y las claves. Un capítulo cerrado da `aedas.epica_capitulo`; el último, la tecnología aparecida (el crate hace `hacer_aparecer`; el núcleo programa las dos citas del descubrimiento si es la primera del mundo, cierra las otras épicas de esa tecnología y mira la aparición de la Facción), `cumplidas[F] += 1`, `aedas.epica_tecnologia` (privado) y `aedas.epica_cumplida` (público, sin la tecnología, y a la crónica). `registrar_aparicion` (hito, administración y, con 6D, la compra) cierra las épicas que sobran (`motivo: aparecida`).

**Conquista, ruina y traslado.** La conquista pasa los residentes a la conquistadora y les quita la épica (`aedas.epica_abandonada { motivo: conquista }`, a la Facción de antes, con la plaza en los datos) y revisa la plaza. La ruina se los lleva con su espera y su cita, sin hecho. La anexión y la fusión (decisión técnica 13): los residentes de la que desaparece pasan a la que recoge con su épica, sus épicas cumplidas se suman y la épica de una tecnología que la que recoge ya tiene se pierde (`aparecida`); las plazas movidas se revisan.

**Mecenas de los Aedas.** `medidas_de_titulos` da `mecenas = cumplidas × 10 + residentes de sus plazas` (TS `titulos.ts`): el título ya puede nacer.

**Foto `BAN40`.** Los residentes, sus épicas con sus claves y las citas de llegada. 6D llegó antes a `origin` con `BAN39` (los itinerantes), así que 6E queda en `BAN40`, consecutiva: `BAN39` y `BAN38` se rechazan con su motivo (no traen los residentes ni las épicas).

**Pruebas.** `crates/nucleo/tests/aedas_residentes.rs` (9, sobre el mundo real; la plaza levanta su Palacio por orden y la nobleza pide un ciudadano): la llegada exacta tras el plazo y solo con cupo libre (sin Palacio y con el cupo lleno, nada en la cola), el segundo al subir el cupo, los nombres; la reputación baja (×3); los rechazos de empezar y abandonar uno a uno sin rastro, `otro_la_canta` con dos residentes y la épica que se pierde al aparecer por administración; las obras que cierran los capítulos (la que no es del capítulo no cuenta, el enfriamiento), la aparición sin hito con su descubridora y sus dos citas, los tres hechos y la crónica, en la versión de la obra, y Mecenas (11); la anexión que pasa el residente y suma la épica cumplida; la conquista por números (los residentes a la conquistadora, la épica perdida con `motivo: conquista`); el cupo que cae con el mantenimiento colapsado (`residente_se_va` con cupo 0) y la ruina que no deja nada en la cola; la foto `BAN40` ida y vuelta y `BAN38` rechazada; el mismo mundo de una vez, a trozos y tras una foto con un capítulo en curso; y por el WebSocket real, empezar y abandonar con su hecho en la `Delta` de la Facción y sus rechazos. Unitaria: D11 (`residentes::pruebas`). Ya estaban en el crate (6A) los 20 casos de `aedasResidentes.test` que son reglas y las épicas de `comandosTecnologia`.

**Cifras.** `cargo test --workspace --no-fail-fast`: 1.562 pruebas en verde (103 resultados, 16 ignoradas, 0 fallos). Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): 575 sin fallos. Los 20 binarios con pruebas por el WebSocket real (los 19 de 6C más `aedas_residentes`) 20 veces seguidas: 400 ejecuciones, 2.820 pruebas, 0 fallos. `buf lint` y `buf breaking` limpios (sin cambio de esquema). Paso de minuto (`rendimiento_de_mil_plazas_tres_dias`, 1.000 plazas × 3 días, release, `--exact`, alternando los binarios con la máquina compartida con otros agentes): base 22,7 / 26,9 / 27,4 / 25,9 / 24,2 / 24,5 s (media 25,3), 6E 35,0 (ruido) / 28,8 / 26,6 / 24,8 / 26,3 / 24,6 s (media sin el atípico 26,2; las cuatro últimas parejas, 25,5 contra 25,6); los mismos 3.061.541 pasos y la misma versión 50.408. Lo que añade el paso: una búsqueda en el índice de residentes y dos comparaciones.

**Integración con 6D** (merge de `origin` en `3e6629b`, 6D con la foto `BAN39`, la orden 176, la etiqueta 20 y la carga 19; 6E usa la etiqueta 21 y la carga 22, sin choque). La foto de 6E queda en `BAN40`, consecutiva. La compra a un Aeda itinerante cierra la épica de esa tecnología que cantara un residente de la Facción (`motivo: aparecida`, `epicas_sobrantes`). Al cerrar la versión: las épicas, la aparición por hito y después las revelaciones de 6D. Con las dos tandas **ninguna orden del contrato da `orden.no_disponible`**: se retiró `no_disponible` de la traducción (el código sigue en el catálogo) y `tests/sesiones.rs` prueba ahora que una de esas órdenes, de una cuenta sin héroe, sale `heroe.desconocido`. Tras integrar: `cargo test --workspace --no-fail-fast`, 1.573 pruebas en verde (104 resultados, 16 ignoradas, 0 fallos); auditoría, 586 sin fallos; los 21 binarios con pruebas por el WebSocket real (con `aedas` de 6D) 20 veces seguidas: 420 ejecuciones, 3.040 pruebas, 0 fallos.

**Dudas para el autor (6E)**

1. **Tras llegar un Aeda la espera siguiente empieza en ese mismo instante** (el TS, en el tick siguiente: un minuto después). Recomendado: así.
2. **La ruina se lleva a los residentes y su épica sin hecho** (como el TS; la decisión técnica 11 solo nombra conquista, aparecida y orden). Recomendado: así (la ruina ya se avisa).
3. **La épica perdida por conquista la oye la Facción de antes** (con la plaza en los datos), no la plaza, cuya dueña ya es otra. Recomendado: así.
4. **En la anexión y la fusión la épica sigue con el Aeda** (como `trasladoDeFaccion.ts`), aunque la que recoge ya tenga otro Aeda con la misma: siguen los dos y la primera que termina cierra la otra (`aparecida`). Recomendado: así.
5. **Una Facción sin reputación guardada la tiene en 0** para el umbral bajo (si se subiera el umbral a 0 o más, tardarían tres veces más). Recomendado: así (es `Reputacion::nueva`).
6. **Un `aeda_id` sin la forma `aeda-rN` sale `epica.no_vive_aqui`** en la traducción, sin un código nuevo de «desconocido». Recomendado: así.
7. **Las épicas se procesan antes que la aparición por hito** al cerrar la versión: si en la misma versión las dos la cumplen, cuenta la épica (y suma a Mecenas). Recomendado: así (el TS avanzaba las épicas con los hechos de la orden y el hito en el tick).
8. **Empezar o abandonar una épica no lleva el candado de la plaza en batalla**, solo el del héroe (el TS no lo bloqueaba). Recomendado: así.

**Decidido por el autor (2026-10-25), dudas de 6E:** 1, 2, 3, 5, 6, 7 y 8, como están. La 4 cambia: **dos épicas de la misma tecnología se unifican** al trasladar (sigue la más avanzada: capítulos, luego hechos, luego la del Aeda que llegó antes; la otra se pierde con `motivo: unificada`). Regla pura `Residentes::unificar` en `crates/tecnologia`, llamada desde `trasladar_residentes`.

**Decidido por el autor (2026-10-25), dudas de 6B, 6C y 6D:** 6B-1 cambia: **cada Era dura un mínimo de 1 semana** (`ERAS[..].minimoSemanas`); con todos sus logros pasa al cumplir el mínimo, nunca antes, y por plazo al agotar el techo (`EstadoTecnologia::proxima_era_en`; la cita global se mueve al mínimo cuando se fija el último logro). 6B-2: `primera_en_llegar` sale del índice derivado `Espacio::plazas_por_alcanzado` (D16 cerrada; la auditoría lo vigila). 6B-3: la Caravana de Fundación capturada cuenta. 6B-4: la fabricación con taller en el núcleo, a 6H. 6C: las seis como están. 6D: las ocho como están (el precio en milésimas es la convención del contrato: toda cantidad viaja ×1.000).

## Avance de 6G (2026-10-25, rama `tanda-6g`, sin cambio de foto)

Hecha sobre `886466d` (6A-6F y la crónica integradas, foto `BAN40`). Sin órdenes ni comandos nuevos; esquema Protobuf **aditivo** (`buf lint` y `buf breaking` limpios).

**Qué ve cada ámbito**

| Ámbito | Qué lleva del sistema 6 | Qué no |
|---|---|---|
| `FACCION` | `TecnologiasFaccion` entera: `aparecidas`, `adoptadas` (las de arranque primero, como desde 6B), `reveladas` y, nuevo, `detalle_reveladas` (de cada revelada su descubridora y `cumplidas[i]`, la condición `i` de su hito cumplida ya o no) y `epicas_posibles` (con épica, logro cumplido, Era abierta, no aparecida y sin un Aeda de la Facción que ya la cante). `AedaResidente` (`aeda-rN`) de sus plazas con su épica: capítulo, hechos, último hecho y, nuevos, `capitulos` y `hechos_necesarios` | las tecnologías y los residentes de otra Facción; las `claves` de las épicas |
| `HEROE` | la **forma pública** del estado de tecnología del mundo (`EstadoTecnologia`, id `tecnologia`, clase nueva `ESTADO_TECNOLOGIA`), la misma para todos: la Era, `era_desde_ms`, `logros_cumplidos` (nuevo: `{contador, umbral, en}` por orden, sin la tecnología que abren), `cantadas` y, de `primeros`, solo las cantadas (públicas desde su canto). Los `Alijo` que el héroe tiene a la vista (si puede buscarlos), con su sitio y su oro | los contadores en curso y los logros y las descubridoras por tecnología (Q6) |
| `VISTA` | los `AedaItinerante` (`aeda-N`) que se ven ahora, redactados como una columna ajena: posición, tramo visible del plan (30 s, renovado por el muestreo) y la plaza en que están parados | hasta cuándo se quedan, adónde van, dónde estuvieron |
| `ADMIN` | todo: el estado de tecnología entero (los 24 contadores, los logros y las descubridoras por tecnología), los residentes con sus `claves` (texto del TS con los ids del contrato), los itinerantes enteros | — |

**Cómo viaja**

- **Por el registro de cambios** (como siempre): `Clase::Residente` (la 19.ª; id = N de `aeda-rN`, pertenece a `FACCION` de su dueña; cada sitio de `nucleo/residentes.rs` que cambia un residente lo marca, y la auditoría lo vigila) y `Clase::Tecnologia` (la 20.ª; id 0, la parte pública: se marca al fijarse un logro, empezar una Era o cantarse algo). La forma pública es de todos: `Mesa::repartir` la mete en el cubo de cada `HEROE` cuando la versión la toca y la `Foto` de `HEROE` la lleva siempre. `TecnologiasFaccion` sigue en la versión de su Facción; desde 6G además se marca la Facción: con un logro o una Era nuevos, todas (sus épicas posibles), y con un cambio que mira un hito, la que tiene reveladas (sus `cumplidas`; solo en una versión que ya se cierra por otra razón, para que el contador `version` sea el mismo con sesiones y sin ellas).
- **Fuera del registro** (lo que cambia sin subir la versión): los **itinerantes** en `VISTA` se calculan una vez por perspectiva en `Mirada::de` (los que ven sus ojos) y cada sesión filtra su rectángulo; su sello sale de su plan o de su estancia (no son una clase: duda 7 de 6D). Los **alijos a la vista** en `HEROE` los pone al día `Mesa::reconciliar_alijos` con el mismo ritmo que la vista (en cada muestreo, y tras una orden para quien la dio). En `ADMIN`, `Mesa::admin_al_dia` compara en cada reparto el estado de tecnología (con los contadores, que suma el paso de minuto) y los itinerantes con lo que el cliente tiene (sin copiar nada si no cambió) y manda lo que cambió. Al reanudar por deltas, todo esto va entero en la misma `Delta`; si la reanudación es sin novedad, se da por sabido (no hay mensaje de más).

**Contrato.** `TipoEntidad::ESTADO_TECNOLOGIA = 34`; `TecnologiasFaccion.detalle_reveladas` (mensaje nuevo `TecnologiaRevelada`) y `.epicas_posibles`; `EstadoTecnologia.logros_cumplidos` (mensaje nuevo `LogroCumplido`) y, tras integrar la decisión del autor sobre 6B (cada Era dura un mínimo), `era_minimo_hasta_ms` y `era_plazo_hasta_ms`, públicos; `EpicaEnCurso.capitulos` y `.hechos_necesarios`; comentarios de qué ve cada ámbito en `AedaResidente`, `AedaItinerante` y `Alijo`. **Diez hechos tipados** que cubren los quince códigos del sistema 6 (`EventoDominio.tipado` 33-42): `LogroCumplido`, `EraComenzada`, `TecnologiaAparecida`, `TecnologiaAdoptada`, `DescubrimientoCantado`, `RevelacionDeAeda`, `CompraAAeda`, `ResidenteCambia`, `EpicaCambia` (con su `motivo`, también `unificada`) y `AlijoAbierto`; el payload genérico nombra además al Aeda (`aedaId`) y a la descubridora (`descubridoraId`). `contrato::almacen` conoce las entidades nuevas. `Comandos_Parametros.md` ya no tiene «Órdenes sin comando».

**Rendimiento** (release, alternando los binarios de la base `886466d` y de 6G):

| Qué | Base | 6G |
|---|---|---|
| `rendimiento_de_mil_plazas_tres_dias` (1.000 plazas × 3 días) | 20,41 / 20,45 / 20,44 s | 20,68 / 20,36 / 20,59 s |
| Carga, 5 min de mundo en pasos de 250 ms con 500 sesiones (el muestreo) | 31,92 / 32,45 / 36,21 s | 32,80 / 33,34 / 35,30 s |
| Carga, coste por orden con el reloj parado y 500 sesiones | 20.875 / 21.188 / 24.361 µs | 22.057 / 21.939 / 22.369 µs |

Los mismos 3.069.110 pasos y la misma versión 50.408; en la carga, las mismas 3.403 versiones y los mismos bytes por sesión (la foto de `HEROE` crece 0,1 KB/min por la forma pública). Un primer intento, que miraba a cada Aeda contra los ojos de cada sesión, daba +2 s en el muestreo; mirarlos una vez por perspectiva lo deja en el ruido.

**Pruebas.** Nuevas por el WebSocket real, una por ámbito y comprobando lo que **no** se ve: `aedas.rs` (`HEROE` con la forma pública sin contadores ni logros por tecnología; `FACCION` con la revelada y su detalle y nunca las tecnologías de la otra; `VISTA` con los tres Aedas redactados y nada para quien no los ve; `ADMIN` entero y al día cuando un Aeda cambia sin que cambie la versión), `aedas_residentes.rs` (el residente en `FACCION` antes y después de empezar su épica, sin claves; las épicas posibles; nada para quien no es de la Facción; el residente en `ADMIN`; el hecho tipado), `alijos.rs` (el alijo a la vista en la `Foto` de `HEROE`, su `QUITA` al abrirlo, el hecho tipado y nada para quien es de una Facción con plazas), `cronica.rs` (la Era que empieza llega en vivo a `HEROE` con su hecho tipado, su entrada de crónica y la forma pública en la Era nueva, y la misma entrada en la página de la crónica) y `eventos.rs` (los diez tipados de los quince códigos, ida y vuelta en bytes). Cinco dorados nuevos (89 pares) con su significado en `crates/contrato/tests/golden.rs`. Ajustadas las que contaban las entidades de `HEROE` (ahora también la forma pública) y las claves del almacén de prueba.

**Cifras** (antes de integrar). `cargo test --workspace --no-fail-fast`: 1.577 pruebas en verde (104 resultados, 16 ignoradas, 0 fallos). Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): 589 sin fallos. Dorados: Rust, C# (90 pruebas) y TypeScript (90 pruebas) en verde. **Tras integrar** `origin` en `bd108a3` (6E: las épicas que se unifican) y `0578013` (6B: cada Era dura un mínimo; la forma pública gana sus dos bordes): `cargo test --workspace --no-fail-fast`, 1.579 pruebas en verde (104 resultados, 16 ignoradas, 0 fallos); auditoría, 590 sin fallos; los 21 binarios con pruebas por el WebSocket real (`a_mano`, `admin`, `aedas`, `aedas_residentes`, `alijos`, `ambitos`, `batalla_unity`, `carga`, `cliente_de_ejemplo`, `cronica`, `eventos`, `fundacion`, `interior`, `ordenes`, `ordenes_mundo`, `propios`, `reanudacion`, `sesiones`, `tecnologias`, `vista` y `servidor/tests/sesion.rs`) 20 veces seguidas: 420 ejecuciones, 3.120 pruebas, 0 fallos; dorados en Rust, C# (90) y TypeScript (90) en verde; `buf lint` y `buf breaking` limpios.

**Dudas para el autor (6G)**

1. **El Aeda itinerante viaja en `VISTA` con el tramo visible de su plan**, como una columna ajena, no solo con su posición (Niebla §5.4 y el TS: id, posición, plaza). Sin el tramo, uno que anda viajaría en cada muestreo; con él, una vez por tramo. Recomendado: así (Niebla al día).
2. **Lo que no pasa por el registro** (los contadores y los itinerantes en `ADMIN`, los alijos en `HEROE`) va entero en la `Delta` de una reanudación que trae algo; en una sin novedad se da por sabido: un Aeda que se movió sin que nada más cambiara queda viejo en el cliente de administración hasta su siguiente cambio. Recomendado: así (sin novedad solo pasa con el mundo quieto).
3. **Los alijos a la vista se renuevan con el muestreo de la vista** y tras cada reparto; sin `VISTA` abierta no se arma un muestreo propio (se renuevan con lo que pase en el mundo). Recomendado: así (quien busca alijos mira el mapa).
4. **Un héroe que deja de poder buscar alijos estando desconectado** y reanuda por deltas conserva en su cliente los que tenía hasta su siguiente `Foto` de `HEROE`. Recomendado: dejarlo (raro; arreglarlo pide quitar todos los alijos del mundo a cada héroe que reconecta).
5. **`cumplidas[i]` de una condición `recursoEnCapital` puede ir atrasada** hasta el siguiente cambio de la Facción: el paso de minuto de la capital solo la marca cuando el hito entero ya se cumple. Recomendado: así (hoy solo la pide la aleación del bronce).
6. **No viajan `eraDe` ni `capitalId`** del `TecnologiaJugador` del TS: el cliente tiene el catálogo y `Faccion.capital_id`. Recomendado: así.
7. **`Alijo.oro` es el del artefacto, en milésimas** (el mismo mensaje que `ArtefactoDeMundo.alijos`); lo que se carga, en unidades enteras, lo dice `alijo.abierto`. Recomendado: así.
8. **Al fijarse un logro o empezar una Era se marcan todas las Facciones** (sus épicas posibles miran los logros y la Era): unas decenas de veces por partida. Recomendado: así.
9. **Las `claves` de una épica solo van a `ADMIN`**, como el texto del TS con los ids del contrato (`obra:ed-6-4:0`, `defensa:as-3:<instante>`, `batalla:<battleId>`). Recomendado: así.
10. **`aeda-N` y `aeda-rN` comparten la clase `AEDA`** del contrato (ids distintos, cuerpos distintos: `AedaItinerante` y `AedaResidente`). Recomendado: así (ya estaba en el esquema).
11. **La forma pública lleva los dos bordes de la Era** (`era_minimo_hasta_ms` y `era_plazo_hasta_ms`), no `proxima_era_en`: este diría si la vigente ya tiene todos sus logros, y la forma pública no dice qué logros son de qué Era. Recomendado: así.

## Avance de 6H (2026-10-26, rama `tanda-6h`, sin cambio de foto, de reglas ni de esquema)

Hecha sobre `34e5335e` (6A-6G y la crónica integradas, foto `BAN40`). El núcleo no cambia: la tanda es el batch, una prueba de núcleo nueva (la fabricación) y documentos. **Q3: no se calibra nada**; se comprueban reglas y se informa en qué día cae cada logro y cada Era.

**`batch tecnologia [--plazas 60] [--dias 42] [--semilla 42] [--escala-logros 1]`** (`crates/batch/src/simmil/tec.rs` y `armartec.rs`; prueba corta en `crates/batch/tests/tecnologia.rs`; preguntas T1-T4 en `crates/batch/decisiones/tecnologia.json`) es un modo del batch político (`simmil::simular_tecnologia`): la misma población (tres plazas por Facción, cuatro héroes bots por plaza, bots de diplomacia), con bots de tecnología que solo dan órdenes del contrato. 42 días por defecto: el plazo de la Era I es de cinco semanas. `--escala-logros D` divide cada umbral por `D` (como mínimo 1) en el arranque del núcleo, como experimento para ejercitar en pocos días lo que con las X del TS no llega (el bloque no cambia; el informe lo dice).

- **La vía de juego, para todos los batches** (`Sim::adoptar`, en el turno de gobierno de cada hora): si la plaza es la capital efectiva de su Facción y le apareció algo sin adoptar, su Rey lo adopta con `AdoptarTecnologia`, dentro de la capital y con la tarifa de su almacén; con el Rey fuera o sin la tarifa, espera (se cuenta). En el batch tecnológico, si el Rey (un héroe que no es bot militar) está dentro de otra plaza suya, sale (`SalirAlMundo` de su residencia, `SalirDeAsentamiento` de otra), marcha a la capital y entra (`EntrarEnAsentamiento`). En `batch comercio` (`simcom::Sim::adoptar`) el Rey de la fundación va en la columna de la plaza y la necesita para sus misiones: le pasa la corona (`AsignarRey`) al Gobernador de oficio de la capital, que reside y está dentro, y este adopta. Ningún batch usa ya `FijarTecnologias`.
- **El Gobernador de la capital construye hacia los hitos** cada 2 h: el primer edificio (o nivel interno) que pide el hito (`edificio`, `capitalEnNivel`) de una tecnología de una Era abierta que a su Facción aún no le apareció y que ninguna plaza suya tiene; no pide lo que el catálogo no deja (nivel de la plaza, tecnología adoptada, nivel interno inexistente) y, tras un `obra.sin_sitio`, espera un día. **Cada plaza de nivel 2** marca un pueblo minero (cobre, estaño, hierro) si a ese `extraido.*` le queda un logro de una Era abierta.
- **El Gobernador de cada plaza**, con un Aeda itinerante parado en ella, le compra la primera revelada que el almacén puede pagar (`ComprarTecnologiaAeda`); el 3 % de sus turnos les cierra la puerta (`FijarPuerta` con `aedas`) y la reabre a las 12 h; a cada residente sin épica le encarga la posible más a mano (por hecho de cada capítulo: una obra que la plaza puede pedir 0; adopción, batalla y caravana 1; ascenso, defensa, conquista y una obra que no puede pedir 3), levanta o mejora la obra que pide el capítulo en curso, abandona la épica sin un hecho en 4 días (y no la vuelve a empezar en esa plaza) y, al azar, el 1 ‰ de sus turnos. Las caravanas del Intendente llevan 120 u de trigo (en el militar, 30) para que inspiren el capítulo `caravana` (≥ 100 u).
- **Seis buscadores** sin Facción, uno por campamento, salen (`SalirDelCampamento`), abren hasta tres alijos a 200 u como mucho (`AbrirAlijo`; el 30 % de las veces lo intentan otra vez y tiene que rechazarse con `alijo.ya_abierto` sin rastro), vuelven (`EntrarEnCampamento`), vacían el carro (`VaciarCarro`) y reclutan en su campamento (`ReclutarEnCampamento`, Q4). **Andamiaje declarado:** el oro de los alijos (≈ 4 u cada uno) no paga un recluta (300 u el más barato), así que al nacer la administración les da el de dos (`DarAlmacenPersonal`).
- **El núcleo es persistente desde el principio**, en un directorio temporal propio (sin `fsync` por lote), para que la crónica se guarde; la foto ida y vuelta a mitad se hace en el **mismo** directorio (apagar, foto, abrir otro núcleo desde ella), y al final se lee el diario entero por páginas (`PedirCronica`, 1.000 por página).

**Invariantes** (todos a 0; los del militar y el político siguen):

| Invariante | Cuándo | Qué |
|---|---|---|
| `inv_logro` | tras cada orden | un logro fijado no cambia ni se borra; uno nuevo se fijó en el tramo (entre la orden anterior y esta) y con su contador cruzando el umbral en él; uno sin fijar no tiene el contador cruzado |
| `inv_contador` | tras cada orden | ninguno de los 24 contadores baja |
| `inv_adoptadas` | tras cada orden | `adoptadas ⊆ aparecidas` y ninguna revelada está aparecida, en cada Facción |
| `inv_era_cerrada` | en cada `tecnologia.aparece`, `aedas.venta` y `aedas.epica_tecnologia` (contra la Era de los hechos leídos hasta él) y tras cada orden (el estado) | nada aparece de una Era que no ha empezado, también por la unión de una fusión |
| `inv_era` | tras cada orden | la Era pasa de una en una, nunca antes de su mínimo (1 semana) ni después de su plazo; la vigente no sigue pasado su plazo ni, con todos sus logros, pasados su mínimo y su último logro |
| `inv_sin_logro` | en cada hecho de aparición y en cada compra aceptada; tras cada orden | nada aparece sin su logro; comprar nunca salta el logro (y lo comprado lo conocían los Aedas: descubridora + 24 h) |
| `inv_epica` | tras cada orden | una épica por tecnología y Facción (también tras unificarlas en una anexión o una fusión) y ninguna de una tecnología ya aparecida |
| `inv_doble` | en cada observación, por tramo | los 14 contadores discretos suben exactamente lo que dicen los hechos leídos, con la tabla de vías de `tecnologia::contador` aplicada a los hechos (`combate.resuelto`, `combate.encuentro`, las caravanas interceptadas por columnas y por bandidos, el campamento destruido, `combate.asedio_*` —**nunca su `batalla.aplicada`, D18**—, el animal comprado, `tropas.reclutadas` y `mercenarios.reclutado`); `conHoplitas` y `conResidentes`, que no viajan en el hecho, no pasan de `batallas.libradas` y `asedios.resistidosEnCombate` |
| `inv_itinerantes` | tras cada orden | los itinerantes nunca bajan |
| `inv_puerta_aedas` | tras cada orden (también justo después de cerrarles la puerta) | ningún Aeda parado en una plaza que no existe o que les cierra la puerta |
| `inv_alijo` | en cada `alijo.abierto` y en cada segundo intento | un héroe abre un alijo una vez; lo cargado no pasa del oro |
| `inv_cronica` | al final | la crónica guardada es exactamente la emitida (los `EventoDominio` de los cambios de versión), también tras la foto ida y vuelta |
| `inv_rechazos` | en cada orden que el bot da por segura | adoptar, comprar, empezar y abandonar épicas, la puerta a los Aedas, abrir alijos, salir y entrar del campamento, el Rey que va a la capital |

**Corridas** (release; 60 plazas; el batch tecnológico 42 días; seis o siete procesos a la vez en una máquina de 16 hilos):

| Corrida | Huella | Violaciones | Pared |
|---|---|---|---|
| `tecnologia`, semilla 42 (dos veces) | aa3c74a0… las dos | 0 (12 del sistema 6 + 33 del político y el militar; `inv_rechazos` 0); foto ida y vuelta igual; crónica 192 = 192 | 101 s y 102 s (2,4 s de CPU por día) |
| `tecnologia`, semilla 7 | ff6d80d0… | 0; crónica 199 = 199 | 99 s |
| `tecnologia --escala-logros 1000`, semilla 42 (dos veces) | 06fd3920… las dos | 0 | 99 s y 100 s |
| `tecnologia --escala-logros 1000`, semilla 7 | 15f2179b… | 0 | 99 s |

| Medida (X del TS) | Semilla 42 | Semilla 7 | Escala 1000, semilla 42 | Escala 1000, semilla 7 |
|---|---|---|---|---|
| Era II / Era III (día) | 35,0 por plazo / — | 35,0 por plazo / — | 35,0 por plazo / — | 35,0 por plazo / — |
| Adopciones / aparecidas (hito, compra, épica) | 72 / 90 (82, 8, 0) | 39 / 46 (40, 6, 0) | 104 / 119 (97, 22, 0) | 68 / 78 (58, 20, 0) |
| Aedas itinerantes al final / revelaciones / puertas cerradas con alguno dentro | 7 / 8 / 154 | 7 / 9 / 148 | 7 / 22 / 156 | 7 / 23 / 127 |
| Residentes llegados / épicas empezadas / capítulos / cumplidas | 51 / 95 / 5 / 0 | 41 / 72 / 1 / 0 | 50 / 111 / 28 / 0 | 45 / 147 / 18 / 0 |
| Épicas perdidas: `aparecida` / `unificada` / atascadas | 16 / 1 / 63 | 8 / 6 / 48 | 11 / 2 / 70 | 5 / 5 / 102 |
| Alijos abiertos (oro) / reclutas de campamento | 30 (114 u) / 80 soldados | 30 / 80 | 30 / 80 | 30 / 80 |

**Días de los logros con las X del TS** (semilla 42 / 7; «—», no se cumple en 42 días; entre paréntesis, el día en que aparece por primera vez): Era I: carros de guerra 3,1 / 4,7 (nunca: su hito pide la cría caballar adoptada y Caballerizas y Carpintería de nivel 2), arquería palacial 4,6 (7,2) / 5,6 (7,1), armamento palacial 5,5 (15,6) / 5,5 (22,2), metalurgia del cobre 15,6 / 22,2, cantería 18,7 / 20,2, cría caballar 24,6 / —, escudos ligeros 26,0 / —, aleación del bronce — / — (`extraido.estano` a 0: T2). Eras II y III: ninguno salvo, en otras corridas de prueba, carpintería militar (`conquistas.conMurallaCompleta` ≥ 1, con la primera conquista con muralla). Con los umbrales entre mil, la forja del hierro temprana cumple su logro el día 1,5 y aparece justo el 35,0, en la versión de `era.comienza` (la Era cerrada espera a la suya).

**El militar, el político y el de comercio por la vía de juego** (semilla 42, 30 días; antes = `34e5335e`, con `FijarTecnologias`; R1: solo se informa):

| Batch | Antes | 6H | Lo que cambia |
|---|---|---|---|
| `militar` | huella bff6b183…, 93 métricas | huella bc4c51c5…; 44 de 93 iguales; 40 adopciones (el Rey, el héroe de la fundación, está siempre dentro de la primera plaza) | antes tenía la Era I entera desde el minuto 0 (`canteria` ×1,5 incluida) y ahora la va adoptando: combates 6.529 → 7.072, conquistas 16 → 24, bajas 142.365 → 145.971, reclutados 130.247 → 133.709; las tropas reclutadas son las mismas (lanceros, leñadores, granjeros y mineros: la tecnología de Barracón no la usaba ni antes); 0 violaciones; pared 486 → 481 s |
| `politica` | 131 métricas | 52 de 131 iguales; la corona pasa a menudo a un bot militar que sale de campaña | divergencia caótica de la política: vasallajes 27 → 7, paces 65 → 35, tributo 77.320 → 33.222, anexiones 15 → 9, fusiones 3 → 7, plazas vivas 57 → 54, reclutados 49.317 → 44.621; 0 violaciones; pared 440 → 431 s |
| `comercio` | 103 métricas | 89 de 103 iguales; 40 coronas pasadas y 40 adopciones (entre ellas `canteria`) | el acarreo sube del 89,8 % al 123,6 % de la producción nominal de los pueblos (cantería ×1,5) y sus viajes de 167.735 a 231.042; peajes 22,1 % → 28,0 % de la carga; lo ingresado por acarreo casi igual (el almacén de la madre se llena: 97,5 % perdido); 0 violaciones; pared 91 → 104 s (con otras seis corridas a la vez) |

**La fabricación de bronce dentro del núcleo** (duda 4 de 6B): `crates/nucleo/tests/fabricacion.rs` levanta en una plaza del mundo real, por las órdenes de construcción, la Fundición de nivel 2 y la Armería de nivel 2 (el primer taller del catálogo con armas de bronce), con la aleación adoptada, artesanos y lingotes, y comprueba en un tramo de un salto de 6 h y en otro minuto a minuto que lo que suben `fabricado.equipoBronce` y `fabricado.armaduraBronce` (en milésimas, con su resto) es exactamente lo que entra en el almacén (1.440 u de equipo y 360 u de armadura en cada tramo).

**Fallos encontrados.** Ninguno del núcleo: los invariantes salieron a 0 desde la primera corrida. Del batch, al montarlo: el Rey que sale de su residencia para ir a la capital tiene que salir al mundo (`salida.desde_tu_residencia` en `inv_rechazos`, 212 en la primera corrida); el recluta de campamento probaba todas las tropas tras un `mercenarios.sin_oro`; el Gobernador pedía obras que el catálogo no deja (nivel, tecnología). Arreglados en el bot.

**Pruebas.** Las cifras de la suite y la auditoría, al final («Cifras»). No se tocó el paso de minuto ni el camino de la orden, ni la sesión, las proyecciones o el contrato: una pasada de la suite.

**Dudas para el autor (6H)** (las cuatro primeras, T1-T4 del fichero de decisiones):

1. **T1. Ninguna épica se cumple**: 21 de las 24 empiezan por la obra de un edificio del hito de su tecnología, y como para empezarla el logro ya está cumplido, terminar esa obra hace aparecer la tecnología por el hito y la épica se pierde (`aparecida`); las demás esperan defensas, conquistas o ascensos. Recomendado: dejarlo hasta la fase 7 (la épica es la vía de quien no puede cumplir el hito).
2. **T2. No se extrae estaño** en este mundo con estos bots: la aleación del bronce no cumple su logro, la Era I pasa siempre por plazo y no hay bronce que fabricar. Recomendado: medirlo en la fase 7.
3. **T3. Los alijos dan ≈ 4 u de oro** y un recluta de campamento cuesta 300 u: el batch da el oro por administración (declarado). Recomendado: medirlo en la fase 7 con la refundación y los héroes sin plaza.
4. **T4. El Rey solo adopta dentro de la capital**; en el político la corona pasa a menudo a un bot militar en campaña (3.172 de 3.181 turnos sin adoptar, semilla 42). Recomendado: mantenerlo.
5. **El Rey del batch de comercio pasa la corona** al Gobernador de oficio de la capital para poder adoptar (su columna hace las misiones de la plaza). Recomendado: así (es una jugada del contrato; sus cifras de comercio cambian por la cantería, no por la corona).
6. **`conHoplitas` y `conResidentes` no viajan en los hechos** (`combate.resuelto`, `combate.asedio_*`): el invariante de la doble cuenta solo los acota por arriba. Recomendado: así (añadir el dato al hecho sería un cambio del contrato; las pruebas de 6B los cuentan exactos por cada vía).
7. **El mínimo de una semana de la Era y el paso por todos los logros no se ejercitan en el batch** (la Era I nunca completa sus logros, T2): los cubren las pruebas de `nucleo/tests/logros.rs` y del crate. Recomendado: así.
8. **`--escala-logros` es un experimento del batch** (como `--nobleza 0` del político): cambia los umbrales en el arranque del núcleo, no en el bloque, y el informe lo marca. Recomendado: así.

**Cifras.** `cargo test --workspace --no-fail-fast` (una pasada: la tanda no toca la sesión, las proyecciones ni el contrato): 1.582 pruebas en verde (106 resultados, 16 ignoradas, 0 fallos; +3: la fabricación y las dos del batch tecnológico). Auditoría (`BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento`): 591 sin fallos. `cargo test --release -p batch --test tecnologia`: 2 en verde en 2,5 s. El paso de minuto no se midió: la tanda no lo toca.

**Decidido por el autor (2026-10-26), dudas de 6G y 6H:** las once de 6G y las ocho de 6H, **como están** («todo ok»). T1 (ninguna épica se cumple), T2 (no se extrae estaño) y T3 (alijos de ≈ 4 u de oro frente a las 300 u del recluta) quedan para la fase 7; T4 (el Rey adopta solo dentro de la capital) se mantiene. Las `// DUDA:` del código de las tandas 6B-6G se reescribieron como reglas con su fecha. Registro de todas las decisiones del sistema: `Consideraciones/Politica_Tecnologia_Decisiones.md`.

## Estado de cierre del sistema 6 (2026-10-26)

Hecho: las reglas puras (6A), el estado del mundo, los contadores, los logros y las Eras (6B, foto `BAN37`), la aparición, la adopción y los campamentos (6C, `BAN38`), los Aedas itinerantes (6D, `BAN39`), los residentes y las épicas (6E, `BAN40`), los alijos (6F, `BAN36`), la crónica entera, el contrato y las proyecciones (6G) y el batch tecnológico (6H). Ninguna orden del contrato da `orden.no_disponible`; ningún batch usa `FijarTecnologias`. Con él se cierra la fase 6. **Pendiente:** la calibración (Q3, T1-T3) en la fase 7; las tres dudas de 6A y las de la infraestructura de la crónica que el autor no decidió expresamente (`Docs/Mecanicas a desarrollar.md`, «Dudas abiertas»); los recortes de paridad del viaje de los Aedas (duda 4 de 6D); las Eras IV y V, pospuestas.
