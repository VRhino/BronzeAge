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
| `proyeccion/propios.rs:441` — sin `aparecidas` ni `reveladas` | 6G | |
| `proyeccion/propios.rs:703` (la parte de «las Aedas» en `ADMIN`) | 6G | `ADMIN` lleva los Aedas, el estado de tecnología entero y los alijos |

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
| 6H | l.126 (batch de comercio sin tecnologías) y l.155 (batch militar con las de la Era I por administración): pasan a la vía de juego |
| Cada una | l.163 (los eventos de su tanda, tabla §1.5) |
| Fuera | §20 (Eras IV y V, `logistica_campana`, clientes): sigue tal cual |

Fila nueva que abre el sistema 6 (decisión Q3 del autor): **la calibración de las X de los logros y de la velocidad y la estancia de los Aedas, pospuesta a la fase 7** (bots reales);
se escribe en P al cerrar 6H.

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
