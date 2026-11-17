# Contraste y plan de la fase de Murallas (fase 6, tras 4F)

Preparado el 2026-10-22, solo lectura, sobre `claude/lucid-curran-6d6ffa` (HEAD `a3c83df`, foto `BAN19`). El TypeScript se leyó con `git show typescript-final:src/...`
(`engine/muralla.ts` 980 líneas, `engine/combate.ts`, `engine/asentamientoQuery.ts`, `engine/mantenimiento.ts`, `constants.ts`, `__tests__/muralla.test.ts` 1.002 líneas).
Abreviaturas: M = `Docs/Mechanics/`, G = `Docs/Game/`, P = `Docs/Mecanicas a desarrollar.md`, TS = `typescript-final`.

Nota de partida: `typescript-final:Docs/Mechanics/Murallas.md` y el actual solo difieren en el vocabulario del reloj (tick a evento) y en las dos notas añadidas (cabecera del 2026-10-13 y la nota de la 4D al final).
Es decir, M/Murallas.md **es** la descripción del TS; no hay una segunda versión de la mecánica por reconciliar, solo hay que decidir cómo se traduce al reloj reactivo.

---------------------------------------------------------------------------------------------------------------------------------------

## 1. Inventario

### 1.1 Lo que ya existe en Rust

| Pieza | Dónde | Qué hace hoy |
|---|---|---|
| Tipo `Recinto { muros, puertas, completo }` | `crates/trazado/src/tipos.rs:167-177` | Vista mínima para el trazado: `muros` (muro y torre, **sin distinguir clase**) ocupan suelo, `puertas` no, `completo` activa la preferencia intramuros. No hay nivel, avance, orden de recorrido, torres, coste, `mejorandoA`. |
| `Ciudad::set_recintos`, `recintos()`, `reconstruir` | `trazado/src/ciudad.rs:182, 238-248, 356-362` | Los muros bloquean desde que el recinto existe; las puertas no se siembran como calle (coincide con T §5.2 y TS `trazado.ts`). La red se **recalcula entera** en `set_recintos`. |
| `Ciudad::interior_de_recinto` + partición intramuros | `ciudad.rs:457-487`, `colocacion.rs:231-233` | T §6.6 hecha: inundación 4-conexa desde el Centro Urbano que no cruza muro ni puerta, del último recinto con `completo`. Sin acotar a la caja (la acota la ventana de ±96 celdas). |
| Ventana de la ciudad | `ciudad.rs:15` (`MITAD = 96`) | Todo lo que sale de ±96 celdas cuenta como ocupado. El anillo (región + 1 celda + margen de la caja) tiene que caber; con `radioCiudad` máximo 180 u.l. = 60 celdas cabe con holgura. |
| `Urbe::set_recintos` (gancho) | `nucleo/src/urbanismo.rs:241-244` | Sin ningún llamador de producción; sube `rev` (invalida el caché de «¿cabe?»). Probado solo en `nucleo/tests/urbanismo.rs:245-384`. |
| Persistencia de los recintos de la ciudad | `nucleo/src/codec_ciudad.rs:125-127, 174-176` | Guarda `muros`, `puertas`, `completo` (sin nivel, orden, avance). Hoy es el único sitio donde viviría un recinto. |
| `Exterior.recinto_nivel` y gate del nivel 4 | `asentamiento/src/asentamiento.rs:102-103`, `nivel.rs:23-24, 97-105`, `NIVEL_ASENTAMIENTO.requisitos[4]` | Hecho: `recinto_nivel >= recintoCompletoNivelMinimo (2)`. El núcleo pasa la constante `RECINTO_NIVEL = 0` (`nucleo/nucleo/construccion.rs:55-56, 211`). |
| Upkeep en el mantenimiento | `asentamiento/src/mantenimiento.rs:89-91, 114` | `coste(.., upkeep_recintos: &Costo)` ya suma el upkeep **sin escala**; el único llamador pasa `&vec![]` (`asentamiento.rs:291-293`). La reserva de construcción ya lo recoge a través de `coste_mantenimiento` (`asentamiento.rs:314-322`). |
| Cupo de guarnición por recinto | `militar/src/guarnicion.rs:17-28`; bloque `GUARNICION` en `Heroe_Y_Tropa.md:348` | `cupo(r, edificios, nivel_recinto, extra)` ya suma `recintoCompletoPorNivel[nivel-1]` y solo si hay cupo por edificios. El núcleo pasa 0 (`nucleo/nucleo/heroes.rs:118`). |
| Multiplicador defensivo del combate | `combate/src/resolucion.rs:170, 360-363`; `asedio.rs:4-5, 14, 52` | `resolver_combate(.., multiplicador_defensor, ..)` ya multiplica solo al defensor (probado con ×3). `resolver_asedio` lo fija a `SIN_MURALLA = UNO` y lo usa también la avanzada (`avanzada.rs:45`). La columna y la plaza lo usan en `nucleo/nucleo/combates.rs:358, 643`. |
| Ocupación tras conquista | `nucleo/nucleo/conquistas.rs`, `combate/src/conquista.rs` | `protegida_hasta`, `ocupada_hasta`, eventos de fin, saqueo determinista. **`fraccionDanoMuralla` ni siquiera se parsea** (no hay ningún `dano_muralla` en el código); `conquista.rs:9` y `conquistas.rs:26` lo anotan como pendiente. |
| Política «Postura Defensiva» | bloque de políticas en `Economia_Construccion.md:1973` (`factorTiempoMuralla` 0,75) | El factor se lee por nombre (`politicas.factor(.., "factorTiempoMuralla")`); nadie lo consume aún. |
| Reloj: obras con cita propia | `Carga::FinDeObras` (prio 1), `Asentamiento::cumplir_fines` | Patrón listo para la cita de cada celda. El paso de minuto reintenta lo que esperaba (la mejora de Granja sin hueco). |
| Contrato Protobuf | `proto/bronzeage/v1/{entities,battle,orders}.proto` | `CeldaMuro/ClaseCelda`, `Recinto` (con `siguiente_celda_en_ms`, `comprometido_en_ms`, `completado_en_ms`), `RecintoEnBatalla`, `SettlementBattleSnapshot.recintos`, `RecintoInforme`, y las órdenes `abandonar_recinto`=11, `comprometer_recinto`=50, `mejorar_recinto`=86. Sin mapear en `sesiones::traducir` (4H). |

### 1.2 Lo que falta (todo el cuerpo de la mecánica)

1. **Trazo** (`trazarRecinto`: núcleo, dilatar, rellenar agujeros, anillo, bucle de conflictos, validación de aislamiento, puertas, colapsar puertas, ordenar, torres, coste, área, dentro/fuera/afuerasDentro). No hay nada de geometría de recintos en `crates/trazado` (`lib.rs:21` lo declara «no cubierto»).
2. **Entidad con estado**: `Recinto { id, nivel, celdas con clase, avance, mejorando_a, siguiente_celda_en, comprometido_en, completado_en }` en el estado de la plaza, su codec y su hash. Foto `BAN20`.
3. **Órdenes**: `comprometerRecinto`, `mejorarRecinto`, `abandonarRecinto` (siguientes números libres: 123-125, salvo que otra sesión los tome), gate de nivel de plaza (2/3/4), rechazos estables, hechos `construccion.recinto_*`.
4. **La obra** celda a celda con coste por clase, reserva, `factorTiempoMuralla`, eventos de fin de obra y de mejora (torres recalculadas).
5. **Consulta de presupuesto** (`trazarRecinto` sin comprometer: celdas, puertas, torres, coste, área). Es necesaria: el nº de puertas es irreversible y el diseño exige enseñarlo antes de pagar (`typescript-final:Consideraciones/Murallas_Definicion.md` §20.1).
6. **Ampliación**: gate de arrabal (≥ 6 edificios extramuros), `contieneA` estricto, área encerrada.
7. **Efectos**: multiplicador del asedio con números, cupo de guarnición real, requisito del nivel 4 real, upkeep real, daño por conquista, predicado «muralla completa» para los contadores de logros (`asedios.contraMurallaCompleta`, `conquistas.conMurallaCompleta`, `Tecnologia_Aedas.md:60-61`; se calcula **antes** del daño).
8. **Dibujo/Unity**: `VistaCiudad` no lleva murallas; `Nucleo::recintos_en_pie`; el ticket del asedio no lleva `SettlementBattleSnapshot` (P, fila 4E); el informe de inspección de una plaza no lleva recintos (`Contenido::Plaza { faccion, heroes, guarnicion }`, M/Niebla_De_Guerra.md:41, 207; canon 5.12.10).
9. **Documentos**: quitar las notas «recinto 0» (lista en §3, paso 8) y bajar al canon la regla del multiplicador (hoy G/5 no la dice).
10. **Batch/calibración**: `batch población` usa `recinto_nivel: 0` (`crates/batch/src/simpob.rs:252`, `armarpob.rs:136, 141`); los bots (fase 7) comprometen el recinto en TS (`bots/cerebro/gobierno.ts:197`).

---------------------------------------------------------------------------------------------------------------------------------------

## 2. Contradicciones y huecos (canon / Mechanics / TypeScript)

Resolución: **R** = la decide la regla de precedencia (gana Mechanics y se corrige el canon); **P** = pregunta al autor (Q#); **I** = corrección de documento o decisión de implementación sin impacto de diseño.

| # | Qué | Referencias | Res. |
|---|---|---|---|
| K1 | **El jugador no elige dónde va la muralla.** El canon dice que los edificios estratégicos «(murallas, torres, puerto) se colocan a mano»; Mechanics hace el trazo automático y determinista (el jugador solo elige nivel y cargo), «torre» no es un edificio sino una clase de celda y el puerto no existe. | G/4:32 contra M/Murallas.md:42-79 y §4 (l.83-102) | R: corregir G/4:32 |
| K2 | **El canon no tiene la regla del multiplicador.** Solo cita «el multiplicador defensivo pleno» al hablar del daño (G/5:478) y la ventaja del defensor ×1,05 (G/5:35). La fórmula `1 + (bono-1) × integridad / puertas`, «solo el último recinto» y «solo con números» están únicamente en Mechanics. | M/Murallas.md:147-159; M/Combate.md:143-144, 346; G/5:35, 478 | R: escribirla en G/5 al implementar |
| K3 | **Texto viejo del §9.1.** «Solo en el asedio resuelto con números (`iniciarAsedio`, `asediarConEjercito`: bot contra bot)». `iniciarAsedio` se retiró el 2026-10-12 y desde la cuarta ronda de Entrar con todo la vía numérica es «sin servidores de Unity», no «bot contra bot». | M/Murallas.md:156 contra M/Combate.md:41 y `Consideraciones/Entrar_Con_Todo_Definicion.md` (cuarta ronda) | I |
| K4 | **Mejora en curso: números y Unity ven cosas distintas.** Murallas dice que durante la mejora «la integridad es 0 (no defiende, no suma guarnición)» y Heroe_Y_Tropa lo repite; pero a Unity le viaja «el anillo entero al nivel anterior». El mismo anillo físico defiende en Unity y no defiende con números. Además el requisito del nivel 4 y el cupo desaparecen durante una mejora de piedra a adarve (hasta 100 h a 60 min/celda). | M/Murallas.md:133 y 158-159; M/Heroe_Y_Tropa.md:216; M/Batalla_Unity_Ciclo.md:104 | P (Q1) |
| K5 | **Tres reglas distintas para «qué recinto cuenta».** Multiplicador: solo el **último** (aunque esté a medio hacer). Cupo de guarnición: el **mayor nivel entre los completos**. Gate del nivel 4: **cualquiera** completo de nivel ≥ 2. Consecuencia: durante una ampliación (25-100 h) la plaza pierde el multiplicador entero (el exterior tiene integridad baja y el interior «es historia en piedra») pero conserva cupo y gate; en Unity sí ve el anillo interior completo. | M/Murallas.md:149, 163, 168; M/Combate.md:144, 346; M/Heroe_Y_Tropa.md:212; M/Economia_Construccion.md:495 | P (Q1) |
| K6 | **El upkeep de una mejora no es «el del nivel viejo» sobre lo que está en pie.** Al empezar una mejora `avance` se reinicia a −1 y mide la **mejora**, y el upkeep es `tarifa[nivel] × (avance+1)`: el anillo en pie deja de pagar al empezar la mejora. El test del TS lo oculta usando un estado que no existe en el juego (`recintoSintetico(1,1,10,9,2)`: mejorando con `avance` completo). | M/Murallas.md:133, 172; TS `muralla.ts:969-980`; `muralla.test.ts:840-846` | I si Q1 = B (el anillo en pie conserva todo); si Q1 = A, preguntar aparte si el upkeep sigue al anillo en pie |
| K7 | **TS no limpia las torres al mejorar.** `marcarTorres(recinto.celdas, region, nivelNuevo)` solo marca; las torres del nivel 2 (paso 8) permanecen y el contador del nivel 3 (paso 5) las ignora: se pueden juntar torres. El doc dice «recalcular torres». | TS `muralla.ts:468-495` y la rama de mejora de `avanzarObraDeRecintos` (l.782-833); M/Murallas.md:77, 116 | I: en Rust se vuelve a marcar desde las celdas de muro |
| K8 | **Un recinto completo «no se destruye», pero la conquista lo deja incompleto.** `fraccionDanoMuralla` baja `avance`; a partir de ahí el recinto cuenta como incompleto: se puede **abandonar** (se borra entero y libera el suelo), bloquea mejorar y ampliar, y se repara solo por la obra normal a costa del **nuevo** dueño (plaza ocupada, con el mantenimiento congelado). Ni Mechanics ni el TS lo dicen. Y un recinto en obra o en mejora **no** recibe daño (`if (r.avance < len-1) return r`). | M/Murallas.md:137-138, 181; TS `combate.ts:271-275`, `muralla.ts:855-868` | P (Q2) |
| K9 | **Cifras: la escala de costes está invertida al saltar niveles.** `tarifaPorCelda[2]` y `[3]` son el coste de **mejorar** cada celda, y comprometer directamente en un nivel paga solo esa tarifa (decidido el 2026-10-10). Por celda llana: empalizada 22 (20 madera + 2 piedra), piedra 30 (5 + 25), adarve 30 (10 + 20): el camino 1→2→3 cuesta 82, un adarve directo 30, y el adarve gasta menos piedra (20) que el muro de piedra (25). Todas son placeholder del TS. | M/Murallas.md:102, 218-231, 289; TS `constants.ts:1274-1280` | P (Q4) |
| K10 | **Ampliación: el gate de nivel de plaza está sin decidir.** M/Murallas.md solo pide recinto anterior completo + ≥ 6 extramuros + envolver estrictamente. `BA-006` lo dejaba «en estudio» con «segundo anillo de murallas de nivel 5». Los recintos interiores pagan upkeep para siempre y no se pueden demoler (Ideas §47). | M/Murallas.md:140-143; `Consideraciones/BA-006_Revision_Tecnologia_Eras.md:458-459`; `Docs/Ideas a diseñar.md:161` | P (Q3) |
| K11 | **Dos copias del bloque `GUARNICION`**: `Murallas.md:252` y `Heroe_Y_Tropa.md:348` (idénticas hoy). El código solo lee la de Heroe_Y_Tropa (`militar/src/datos.rs:15, 222`). | M/Murallas.md:252-272 | I: dejar una sola (la de Heroe_Y_Tropa) y enlazar |
| K12 | **Restos obsoletos del TS**: el comentario de `constants.ts:2318-2321` («cualquier recinto terminado de nivel 1 basta; la empalizada cuenta igual») contradice el valor de la línea siguiente (`recintoCompletoNivelMinimo: 2`) y el canon G/4:262; `Ritmo_Crecimiento_Asentamientos.md:21, 281` (`nivelMinimoConstruccion` 3, la empalizada basta para el nivel 4) | G/4:143, 262 | I: ganan G/4 y el bloque; no se porta |
| K13 | **Gate de nivel de plaza**: canon (G/4:143), M/Murallas.md:85 y TS (`muralla.ts:646-657`) coinciden (empalizada 2, piedra 3, adarve 4, con el nivel **operativo**, `nivelActualDe`). La decisión del 2026-10-09 está bien recogida; el único cabo suelto es que `Economia_Construccion.md` (§11.1) y el bloque `NIVEL_ASENTAMIENTO` hablan de «integridad ≥ 1» mientras el parámetro de Rust es un solo entero `recinto_nivel` (el nivel del mejor completo): equivalente, pero hay que calcularlo con integridad ≥ 1 y excluir el recinto en mejora. | M/Economia_Construccion.md:495; `nivel.rs:97-105` | I |
| K14 | **Unity y el mapa**: `Batalla_Unity_Ciclo.md:101-105` pide `recintoId`, `nivel`, celdas en pie, `layoutVersion`, `unidadesPorCelda`; el ticket de Rust no lleva el `SettlementBattleSnapshot` (P, fila 4E). Los ids de recinto del documento son texto (`recinto-<plaza>-<n>`); en Rust todo es numérico. | M/Batalla_Unity_Ciclo.md:101-105; `combate/src/ciclo.rs:111-121` | P (Q5) para el momento; id I |
| K15 | **Equipo de asedio**: decidido sin efecto en el combate con números (P filas 92 y 103; Q7 del 2026-10-19). Por tanto el multiplicador de muralla **no** se reduce con ariete o torre de asedio. El canon G/5:85 y G/6:134, 148 (`carpinteria_militar`, `trabajos_asedio`, que se desbloquean con asedios contra «muralla completa») siguen vigentes como progreso de tecnología, y piden el predicado «muralla completa» (§1.2 punto 7). | P:92, 103; G/6:134, 148 | I |
| K16 | **Avanzadas**: G/5:875 llama «fuerte (empalizada)» y «ciudadela (piedra)» a las etapas, pero no son recintos: su fortaleza son las guarniciones de serie por etapa (M/Combate.md:§7.4). `resolver_asedio` se reutiliza para la avanzada: debe seguir con multiplicador 1. | G/5:875-889; `combate/src/avanzada.rs:45` | I: no cambia |

Huecos que se resuelven por precedencia sin pregunta: las tarifas, el upkeep, los pasos de torres, `arrabalMinimo`, `minutosPorCelda`, `bonoDefensaPorNivel` y `fraccionDanoMuralla` quedan como están en los bloques `MURALLA` y `OCUPACION` (fuente de verdad); su calibración es el último paso.

---------------------------------------------------------------------------------------------------------------------------------------

## 3. Descomposición en pasos

Tamaños en líneas de Rust con pruebas: S < 500, M 500-1.500, L 1.500-3.000, XL > 3.000. Referencia: el TS son ≈ 980 líneas de reglas y ≈ 1.000 de pruebas; `crates/trazado` entero son 1.657.
Total estimado: **6.500-7.500 líneas, 4 commits** (pasos 1 / 2+3 / 4+5+6+8 / 7), más el batch de calibración aparte. **Estado (2026-10-22): hechos los pasos 0-6 y 8; solo falta el 7 (batch y calibración), que se hace cuando el autor pida medir.**

**Decisión de arquitectura (propuesta, sin impacto de diseño; hecha el 2026-10-22 con un cambio: `asentamiento` depende de `trazado`, que solo depende de `base`, para usar `CeldaMuro`, `ClaseCelda` y `ParametrosMuralla` sin duplicarlos ni convertirlos).** El modelo de datos y las reglas que tocan la economía viven en un módulo `asentamiento::recintos` (celdas `(col,row,clase)`) y se guardan en el **`asentamiento::Asentamiento`** de la plaza, no en `trazado`. Razones: el upkeep, la reserva, el gate del nivel, la obra con su `almacen` y el daño de conquista (`combate::conquista::aplicar_conquista` ya recibe ese `Asentamiento`) comparten estado y `Exterior`; un crate `murallas` nuevo sería un ciclo (necesita `Costo`/`Recurso` y la economía necesita guardar sus recintos). La **geometría** (trazo, orden, torres, contiene, interior) va a `trazado::muralla`, que ya posee la rejilla y la `Ciudad`. El núcleo calcula el trazo con la `Urbe` y se lo pasa a la economía; `Urbe::set_recintos` recibe una **proyección derivada** (`muros`, `puertas`, `completo`), de modo que `codec_ciudad` deja de persistir recintos (una sola fuente; el hash y la foto los reconstruyen).

### Paso 0. Decisiones y arreglos de documentos (S, sin código) — HECHO 2026-10-22
Q1-Q5 respondidas. Arreglar en Mechanics lo de K3, K6/K7 según Q1, K11, K16 y las notas «recinto 0». Nada en el canon todavía (se escribe al implementar).

### Paso 1. Reglas puras de geometría: `trazado::muralla` (L, ≈ 1.800-2.200) — HECHO 2026-10-22 (≈ 450 líneas de reglas y ≈ 330 de pruebas; el coste del trazo lo calcula la economía, paso 2; ver la nota de implementación al final de `Docs/Mechanics/Murallas.md`)
- Tipos `ClaseCelda {Muro, Puerta, Torre}`, `CeldaMuro`, `Trazo { celdas ordenadas, puertas, torres, area, dentro, fuera, afueras_dentro }`.
- `trazar_recinto(&Ciudad, nivel) -> Option<Trazo>` portando §3 de M/Murallas.md: núcleo = edificios urbanos + **calles** (nunca caminos) menos granjas y corrales dilatados 1; bucle de conflictos (24 iteraciones, `franjaDeRonda` 1); `rellenar_agujeros`; anillo; validación de aislamiento por flood fill; `clasificar_puertas` con la red; `colapsar_puertas` circular; `ordenar_anillo` (puerta de menor `(row,col)`, vecindad 8 en orden `dr`, `dc`); `marcar_torres` (que **limpia** las torres antes de volver a marcar, K7).
- La red del trazo es la de la ciudad **sin recintos** (como el TS): `Ciudad::red_sin_recintos()` = clon con `set_recintos(vec![])`.
- `contiene_a(nuevo, anterior)` estricto, `interior_de_recinto(celdas, centro)` (las puertas bloquean), `area_encerrada`, `edificios_extramuros`.
- Determinismo: sin iterar `HashSet`; vectores ordenados o `BTreeSet` en todo lo que decide.
- Pruebas: **no se pueden comparar celdas contra el TS** (`LAYOUT_VERSION` 3 usa otro hash; `trazado/src/lib.rs:15-17`), así que se portan los invariantes de `muralla.test.ts:131-257`: cierra, no pisa edificios, ninguna granja dentro salvo `afueras_dentro`, ≥ 1 puerta, una puerta por cruce, recorrido sin repetir ni perder celdas, torres separadas, coste = suma por clase, determinismo, sin Centro Urbano devuelve `None`, ventana de ±96; sobre unas 20 ciudades sembradas hechas con `Ciudad::fundar` + `colocar_siguiente`. Prueba de que `red sin recintos` + muros no parte la red (`muralla.test.ts:328-373`).
- Riesgos: coste de CPU del flood fill (grid 193², hasta 24 vueltas: unos ms; basta con cachear el presupuesto por `(rev, nivel)` en la `Urbe`); la puerta «principal» depende de las celdas de la red, que cambia con los hashes enteros: nada que arreglar, solo no esperar paridad.

### Paso 2. Reglas puras de la economía: `asentamiento::recintos` (M, ≈ 900-1.100) — HECHO 2026-10-22 (≈ 600 líneas de reglas y ≈ 330 de pruebas en `crates/asentamiento/src/recintos.rs`; ver la nota de implementación al final de `Docs/Mechanics/Murallas.md`)
- Bloque `MURALLA` parseado a `Reglas.muralla` (entra en la huella de las reglas; `OCUPACION.fraccionDanoMuralla` pasa a parsearse; `GUARNICION` ya está).
- `Recinto { id (plaza, n), nivel, celdas, avance, mejorando_a, siguiente_celda_en, comprometido_en, completado_en }` y, según Q1, separar el progreso de la mejora de `avance` (propuesta: `Mejora { nivel_objetivo, avance }` aparte, para que el anillo en pie conserve su `avance` completo y se evite K6).
- `integridad` en ppm; `coste_de_celda(clase, nivel)`, `coste_de_trazo`; `siguiente_celda(&mut Almacen, reserva, t, factor_tiempo) -> Evento` (una celda por pasada, `siguiente_celda_en = t + minutosPorCelda × factor`); `iniciar_mejora`, `abandonar` (regla de K8 según Q2), `multiplicador_defensivo` en ppm (`1 + (bono-1) × integridad / puertas`, cociente entero hacia abajo, documentado), `upkeep` como `Costo` en milésimas (0,02 = 20 exacto), `nivel_del_mejor_completo`, `dano_de_conquista`.
- Comprobaciones económicas de comprometer (nadie en obra, nivel de plaza ≥ gate, `arrabalMinimo` con el nº de extramuros que da el núcleo); la geometría (¿envuelve?) la hace el núcleo con el paso 1.
- Pruebas: la mitad de `muralla.test.ts` (obra l.398-515, manual 549-623, mejora 693-770, defensa y upkeep 772-864, gate 962-1002) sobre datos sintéticos de celdas, con monotonía del multiplicador (más puertas, menos bono; integridad lineal; nunca < 1) y una prueba de que pagar no baja de la reserva.

### Paso 3. Núcleo: estado, órdenes, reloj, efectos económicos (L, ≈ 2.200-2.800) — HECHO 2026-10-22 (foto `BAN20`, órdenes 123-125, `crates/nucleo/src/nucleo/murallas.rs` y `crates/nucleo/tests/murallas.rs` con 13 pruebas; faltan solo los efectos de los pasos 4-6)
- `Estado`: `economia.recintos`; `codec_economia` (hash y foto **`BAN20`**; `codec_ciudad` deja de escribir recintos, o los comprueba iguales al derivar; foto `BAN19` rechazada con su motivo).
- Órdenes `ComprometerRecinto { asentamiento, cargo, nivel }` (123), `MejorarRecinto { asentamiento, cargo, recinto }` (124), `AbandonarRecinto { asentamiento, recinto }` (125): autorización provisional con `manda_en` y `cargo` como parámetro dado por ocupado (la misma `// DUDA:` que `ActivarPolitica`, `construccion.rs:459-463`; el Gobernador-solo de abandonar y el cargo reales son del sistema 5). `Comando::implicados`, `es_orden_de_economia`, decodificación por número y rechazos `murallas.*` (sin cargo, nivel de plaza, en obra, sin trazo, no envuelve, arrabal, incompleto, ya mejorando, en el máximo, no existe).
- Consulta pura `Nucleo::presupuesto_de_recinto(plaza, nivel) -> Option<Presupuesto>` (celdas, puertas, torres, coste, área, extramuros).
- **Reloj**: cada celda es una cita en `siguiente_celda_en` con la generación de la plaza y prioridad 1, que reutiliza `Carga::FinDeObras` (`cumplir_fines` avanza también la obra de recintos); si no hay materiales o se bajaría de la reserva, **no** se programa nada nuevo y el paso de minuto lo reintenta (igual que la mejora de Granja sin hueco). Un solo recinto en obra por plaza a la vez, así que como mucho una cita viva por plaza. Se crea al comprometer, se reprograma tras cada celda y se descarta con la ruina.
- Sincronía con la ciudad: tras comprometer, completar, empezar una mejora, abandonar o recibir daño, `Urbe::set_recintos(derivado)` (cuesta una reconstrucción de la red; **no** hace falta por cada celda, porque los muros ocupan suelo desde el compromiso).
- Efectos: `Exterior.recinto_nivel` real (mejor recinto con integridad ≥ 1), `Exterior.upkeep_recintos`, `coste_mantenimiento(r, distancia, upkeep)` (hoy sin recintos, `asentamiento.rs:291-293`), `Nucleo::cupo_de_guarnicion` real (`heroes.rs:118`), `factorTiempoMuralla` (Postura Defensiva), hechos `construccion.recinto_comprometido`, `_mejora_iniciada`, `_abandonado`, `_completado`, `_mejorado` (payload `{recintoId, nivel, celdas, puertas}`), `ocupado` congela el upkeep (ya lo hace el mantenimiento).
- *Hecho distinto de lo previsto:* la primera celda se levanta en el instante de la orden; el comando de abandonar no lleva cargo (el Gobernador-solo es del sistema 5); cada celda deja un hecho `construccion.recinto_celda` (es lo que programa la cita); la ruina descarta la cita por la generación de la plaza igual que la de una obra, pero no tiene prueba propia; `Exterior.recinto_nivel` desaparece (el nivel sale de los recintos de la plaza) y `Exterior.upkeep_recintos` no hizo falta (el upkeep se calcula dentro de la economía). El batch (`auditar_cola`/`inv_fines`) incluye la celda que espera cada recinto.
- Pruebas de integración (≈ 40, previstas; 13 hechas más las unitarias de `asentamiento`): ciclo completo comprometer-obra-completar-mejorar con tiempos exactos, sin materiales no avanza ni cancela, la reserva frena, nivel de plaza insuficiente, el nivel 4 sube con un recinto de piedra y no con la empalizada, cupo +10/15/20, upkeep en la reserva, ampliación y `arrabalMinimo`, foto: ida y vuelta a mitad de obra y rechazo de `BAN19`, ruina descarta la cita.

### Paso 4. Trazado y colocación con recintos reales (S-M, ≈ 500-700) — HECHO 2026-10-22 (tercera tanda: `VistaCiudad.murallas` con `RecintoVista`, y las pruebas de colocación en `crates/nucleo/tests/murallas_asedio.rs`; ver la nota de implementación al final de `Docs/Mechanics/Murallas.md`)
- Los edificios nuevos no pisan el anillo (levantado o no); la red sigue siendo un componente; todo edificio conserva frente de calle; el caché de «¿cabe?» se invalida con `rev`; la preferencia intramuros con recintos completos de verdad; ciudades de las pruebas de `tests/urbanismo.rs` ahora con el trazo real en vez de muros inventados.
- `VistaCiudad` gana `murallas` (levantadas y planificadas, fusionadas por clase con `fusionar_celdas`), que es el contrato de dibujo de T §10.

### Paso 5. Combate con números y conquista (S-M, ≈ 400-600) — HECHO 2026-10-22 (tercera tanda: `resolver_asedio` recibe el multiplicador, `SIN_MURALLA` pasa a `SIN_RECINTO`, `aplicar_conquista` daña los recintos y el núcleo repara; `recintos::muralla_completa` es el predicado, publicado en los hechos hasta que el sistema 6 tenga contadores)
- `resolver_asedio` recibe el multiplicador; `atacar_plaza` pasa `multiplicador_de_plaza(x)`; columna, pueblo y avanzada siguen con 1 (`SIN_MURALLA` se renombra a algo como `SIN_RECINTO` y se documenta). **Sin tiradas nuevas**: el multiplicador no consume `Rng`, el orden de las 2 tiradas no cambia.
- `aplicar_conquista` aplica el daño (`floor(fraccionDanoMuralla × celdas)` de `avance`, mínimo −1, solo a recintos completos) **después** de calcular el predicado de los contadores; la plaza reprograma la cita de reparación y sincroniza la `Urbe`. Vale para la vía numérica y para el resultado de Unity (ambas pasan por `conquistar_plaza`).
- Hechos `combate.asedio_*` con `murallaCompleta` y el multiplicador usado; predicado `plaza_con_muralla_completa` para los contadores del sistema 6.
- Pruebas: fórmulas de los ejemplos del doc (×2,5 con 1 puerta, ×1,38 con 4, ×1,19 con 8), medio anillo defiende casi nada, ampliación en obra según Q1, la conquista deja el anillo en 70 % y se repara sola con la obra normal, un asedio sin recinto es idéntico bit a bit al de antes (misma `Rng`).

### Paso 6. Unity y vistas (S, ≈ 250-400; el mapeo al Protobuf es de 4H) — HECHO 2026-10-22 (tercera tanda: `Nucleo::recintos_en_pie`; el ticket del asedio y el informe de inspección quedan para 4H, Q5 = A, con fila de paridad)
- `Nucleo::recintos_en_pie(plaza) -> Vec<RecintoEnPie { id, nivel, celdas }>` con la regla de `Batalla_Unity_Ciclo.md:104` (celdas hasta `avance`, o el anillo entero al nivel anterior si hay mejora; un recinto sin celdas en pie no viaja), `layout_version` y `unidades_por_celda`.
- Según Q5: solo la función (y que el ticket del asedio lo congele en 4H junto con el resto del snapshot) o el campo ya congelado en el ticket (codec de batallas, nueva foto).
- Informe de inspección de una plaza con `recintos {nivel, celdas, avance}` (M/Niebla_De_Guerra.md:207): se deja para 4H salvo que se decida otra cosa.

### Paso 7. Calibración y batch (M, ≈ 600-900; paso aparte, cuando el autor pida medir) — BATCH HECHO 2026-10-24 (calibración pendiente)
**Hecho (2026-10-24):** `batch poblacion` compromete la empalizada en cuanto el nivel operativo 2 lo permite (`simpob::comprometer_muralla`, `--murallas 0` para medir sin ella), sincroniza la ciudad con la proyección de los recintos como el núcleo y mide compromiso, obra, coste, upkeep, ciudades bloqueadas y piezas sin salida (cifras en `Docs/Mecanicas a balancear.md`, «Murallas»; decisiones M1 y M2 en `crates/batch/decisiones/poblacion.json`). Falta: la mejora a piedra (nivel 3, inalcanzable en este batch), la reparación tras una conquista y el efecto en asedios (`batch militar` sin murallas) y reescribir el bloque `MURALLA` cuando el autor decida M1 y M2.
`batch población` deja de fijar `recinto_nivel: 0` y comete el recinto (el bot de TS comprometía el nivel 1 y mejoraba a piedra cerca del nivel 4: `bots/cerebro/gobierno.ts:197`); medir tiempo, coste, upkeep frente al mantenimiento y reparación tras una conquista; con `batch militar` (4G) medir el efecto en asedios. Resultado: reescribir el bloque `MURALLA` con los valores calibrados (Paso 6 del plan original, sin hacer). Ahora mismo no existe ningún dato empírico (el TS no llegó a medirlo, `Murallas_Definicion.md` §16).

### Paso 8. Documentos (S) — HECHO 2026-10-22 (tercera tanda: quitado «recinto 0» y «hasta que termine la fase» de Combate, Heroe_Y_Tropa, Trazado_Urbano, Economia_Construccion, Murallas y de los comentarios de `asedio.rs`, `conquista.rs`, `conquistas.rs` y `lib.rs`; el canon ya decía la regla desde la primera tanda (G/4 K1, G/5 K2); filas de paridad cerradas o reescritas)
Mechanics: nota «Implementado en Rust» en `Murallas.md` (sustituye la de la 4D), quitar «recinto 0» en `Combate.md:28, 43, 130, 143, 192`, `Heroe_Y_Tropa.md:26, 34`, `Trazado_Urbano.md:14`, `Economia_Construccion.md:36` (`sin recintos`), `Murallas.md:156` (K3), los comentarios de `asedio.rs:4-5`, `conquista.rs:9`, `conquistas.rs:26`, `construccion.rs:55`, `urbanismo.rs:241`; P: filas 80, 84, 92, 103 (se cierran). Canon: G/4:32 (K1) y una regla nueva en G/5 (multiplicador, K2), escrita al implementar como manda el criterio del proyecto.

### Orden de commits (Haiku, según CLAUDE.md)
1) Paso 1. 2) Pasos 2+3 (la economía y su núcleo no se prueban sin el otro). 3) Pasos 4+5+6. 4) Paso 8. Paso 7 cuando el autor lo pida. Cada uno deja `cargo test` verde y la foto coherente.

### Decisiones de implementación que no requieren al autor (propuestas)
- La obra celda a celda se programa con cita exacta y se reintenta en el paso de minuto si faltan materiales (el doc post-2026-10-22 dice «como cualquier otra obra de fin programado», `Economia_Construccion.md:46`). La alternativa (hacerlo solo en el paso de minuto, como el TS) es equivalente para el jugador y más simple; se puede degradar sin tocar reglas.
- La reserva se calcula con el estado al empezar la obra del recinto (Paso 8 del TS ve el almacén tras recetas y mejoras): diferencia de pocos recursos, sin efecto visible.
- Id de recinto numérico `(plaza, n)` con `n` = recintos existentes + 1 (como el doc); se convierte a texto en 4H.
- Un recinto que **alguna vez estuvo completo** no se abandona (propuesta para K8; si Q2 dice otra cosa se cambia).
- Sin comando de «reparar»: la reparación es la obra normal (canon y K8).
- Sin demolición, sin brecha, sin jerarquía de puertas (Ideas §47), sin efecto del equipo de asedio (K15).

### Riesgos
- Dos veces la misma ciudad: la red del trazo (sin recintos) no es la real (con muros); cualquier prueba que mezcle ambas dará falsos fallos.
- `set_recintos` reconstruye la red completa; con 1.000 plazas y conquistas masivas conviene mirar el coste en el batch.
- La ventana de ±96 celdas: un recinto de ampliación tras muchas anclas de saturación podría rozar el borde; el trazo debe devolver `None` (y la orden un rechazo claro) en vez de recortar.
- La foto sube a `BAN20` y cambia la huella de las reglas (nuevo bloque `MURALLA` en la identidad).

---------------------------------------------------------------------------------------------------------------------------------------

## 4. Preguntas al autor (5), listas para presentar

**Q1. ¿Qué recinto defiende mientras otro está en obra?**
Hoy hay dos casos en los que el anillo físico y la regla de números no coinciden. (a) Al **mejorar** un recinto, la integridad vuelve a 0: con números no defiende, no suma cupo de guarnición y no vale como requisito del nivel 4 (hasta 100 h para llegar a adarve), pero a Unity le viaja el anillo entero al nivel anterior. (b) Al **ampliar**, solo defiende el último recinto aunque esté a medio hacer, así que durante 25-100 h la plaza pierde el multiplicador entero aunque el interior esté completo (el cupo y el requisito del nivel 4 sí lo conservan; Unity ve los dos anillos). Además, al empezar una mejora el mantenimiento del muro baja a 0 porque se cuenta con el progreso de la mejora, no con las celdas en pie.
- A) Como está escrito: mejora e integridad 0, ampliación con el último recinto (ventana de debilidad a propósito; el upkeep del anillo en pie se corrige para que no baje).
- B) Defiende lo que está en pie: el mejor recinto completo (con su nivel viejo) sigue dando multiplicador, cupo y requisito mientras el nuevo o la mejora está en obra; solo cuenta el nuevo al cerrarse.
- C) Mixto: la mejora conserva el nivel viejo (B) pero la ampliación mantiene la ventana de debilidad (A).
Recomendada: **B** (el mismo anillo hace lo mismo con números y en Unity, y el upkeep queda coherente). Cambia una regla explícita tuya del 2026-10-09 (la integridad 0 en mejora), por eso la pregunto.

**Q2. ¿Qué pasa con la muralla de una plaza que cae?**
El daño de conquista (30 % del avance a cada recinto completo) deja el anillo «incompleto»: pierde multiplicador, cupo y requisito del nivel 4 hasta que se repare; mientras tanto no se puede mejorar ni ampliar y, tal como está escrito, el nuevo Gobernador podría **abandonarlo** y borrarlo entero. La reparación es la obra normal, sola y a costa del nuevo dueño, en una plaza recién ocupada. Un recinto que estaba en obra o en mejora no recibe daño y la sigue el nuevo dueño (a la obra de ascenso, en cambio, la conquista sí la cancela).
- A) Como en TypeScript: reparación y obras en curso siguen solas; un recinto dañado se puede abandonar.
- B) Igual, pero un recinto que alguna vez estuvo completo no se abandona (solo los que nunca llegaron a cerrarse).
- C) Además, la obra de muralla en curso (construcción o mejora) se cancela al caer la plaza, como el ascenso, y la reparación espera a una orden del nuevo dueño.
Recomendada: **B** (conserva la regla «un recinto completo no se destruye» y es lo mínimo que hace falta cambiar; deja la reparación automática que dice el canon).

**Q3. ¿Ampliar el recinto exige algún nivel de plaza?**
Hoy basta con el recinto anterior completo, 6 edificios extramuros y que el nuevo envuelva al viejo; no hay tope de anillos y cada anillo interior paga upkeep para siempre (no hay demolición). `BA-006` apuntaba, «en estudio», a que el segundo anillo fuese de nivel 5.
- A) Solo lo vigente (arrabal ≥ 6).
- B) Además, nivel de plaza ≥ 4.
- C) Además, nivel de plaza 5.
Recomendada: **A** (es lo que dicen Mechanics y el TS; el coste del upkeep acumulado ya frena el abuso; se puede añadir un gate después sin tocar nada más).

**Q4. Cifras de coste: ¿se importan tal cual y se calibran después?**
Las tarifas de los niveles 2 y 3 son el coste de **mejorar** cada celda, y comprometer directamente en un nivel paga solo esa tarifa. Por celda llana: empalizada 22, muro de piedra 30, adarve 30; subir 1→2→3 cuesta 82 en total. Saltarse niveles sale mucho más barato, y el adarve gasta menos piedra (20) que el muro de piedra (25). Son cifras placeholder del TS y nunca se midieron.
- A) Importarlas tal cual (bloque `MURALLA`) y calibrar con `batch` en el último paso, como el resto de sistemas.
- B) Arreglar ahora la escala (p. ej. tarifas acumuladas por nivel, de modo que subir cueste lo mismo que construir directo).
- C) Prohibir comprometer directamente por encima del nivel 1 (hay que escalar), manteniendo las tarifas.
Recomendada: **A** (calibrar con medidas, no a ojo; la inversión se anota en la tabla de balance para el paso 7). Si prefieres B o C, cambia una regla tuya del 2026-10-10 («se puede comprometer directamente en cualquier nivel»).

**Q5. ¿Cuánto del lado de Unity y de las vistas entra en esta fase?**
Las murallas viajan a Unity como geometría (`recintos` con `nivel` y celdas en pie), pero el ticket del asedio no lleva todavía el mapa de la plaza (`SettlementBattleSnapshot`, 4H), ni el informe de inspección de una plaza lleva los recintos (Niebla de guerra, canon 5.12.10).
- A) Solo la función `recintos_en_pie(plaza)` con la regla del documento y el dibujo (`VistaCiudad.murallas`); el ticket y el informe los congela y proyecta 4H junto con edificios.
- B) Además, esta fase añade ya el campo congelado de recintos al ticket del asedio (nueva foto) y los recintos al informe de inspección.
- C) A y, en el ticket, solo `recintos` (sin edificios), para que 4H complete el resto.
Recomendada: **A** (el snapshot se congela entero de una vez; añadir solo los recintos obligaría a otra foto y a retocarlo en 4H).

---------------------------------------------------------------------------------------------------------------------------------------

## 5. Anexo: puntos del código y de los documentos a tocar (para el implementador)

Código: `crates/trazado/src/{lib,ciudad,tipos}.rs` (+ módulo `muralla.rs`), `crates/asentamiento/src/{asentamiento,mantenimiento,nivel,datos}.rs` (+ `recintos.rs`), `crates/combate/src/{asedio,conquista,datos}.rs`, `crates/militar/src/guarnicion.rs` (sin cambios: ya recibe el nivel), `crates/nucleo/src/{estado,comando,codec_economia,codec_ciudad,urbanismo,sesiones}.rs`, `crates/nucleo/src/nucleo/{construccion,combates,conquistas,heroes,zonas}.rs`, pruebas en `crates/nucleo/tests/` (nuevo `murallas.rs`) y `crates/trazado/tests/`.

Referencias TS para portar: `muralla.ts` (trazo 238-348, aislamiento 349-381, puertas 394-466, torres 468-499, orden 501-531, coste 533-546, comprometer 578-618, gate 646-657, abandonar 660-674, mejora 676-709, contieneA 711-746, obra 757-852, interior 880-922, extramuros 924-948, multiplicador 950-962, upkeep 969-980), `combate.ts:271-275` (daño), `asentamientoQuery.ts:141-142` (cupo), `mantenimiento.ts:68-94, 161` (gate y upkeep), `constants.ts:1250-1300` (MURALLA), `muralla.test.ts` (invariantes a portar).

Invariantes a vigilar en pruebas (M/Murallas.md §12): el anillo aísla el Centro Urbano; una puerta por cruce; la red sigue conexa; las celdas sin levantar ocupan suelo desde el compromiso; la ampliación envuelve estrictamente; un incompleto no se mejora; cada nivel respeta su gate (2/3/4); el multiplicador no baja de 1. Añadir: el asedio sin recinto no cambia ni una tirada; foto y hash idénticos tras ida y vuelta; ningún `HashSet` decide un orden.

## Decisiones del autor (2026-10-22)

- **Q1 → B. Defiende lo que está en pie.** Mientras un recinto se mejora o se amplía, el mejor recinto completo (con su nivel anterior) sigue dando multiplicador, cupo de guarnición y requisito del nivel 4; lo nuevo cuenta al cerrarse. Igual con números que en Unity. El mantenimiento cuenta las celdas en pie. Sustituye la regla del 2026-10-09 de integridad 0 al mejorar.
- **Q2 → B.** Tras una conquista la reparación sigue sola a costa del nuevo dueño; un recinto que alguna vez estuvo completo no se puede abandonar (solo los que nunca se cerraron).
- **Q3 → A.** Ampliar solo exige el arrabal (recinto anterior completo y ≥ 6 edificios extramuros), sin nivel de plaza.
- **Q4 → B. Tarifas acumuladas.** Construir directamente en un nivel cuesta lo mismo que subir nivel a nivel: el coste directo del nivel N es la suma de las tarifas de los niveles 1..N (por celda llana: empalizada 22, piedra 52, adarve 82). Se puede seguir comprometiendo directamente en cualquier nivel (regla del 2026-10-10).
- **Q5 → A.** Solo `recintos_en_pie(plaza)` y el dibujo en `VistaCiudad`; el ticket del asedio y el informe de inspección los congela y proyecta 4H.
