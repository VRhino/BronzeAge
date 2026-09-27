# Revisión de BA-006: tecnología por Eras

**Este archivo es la FUENTE de la revisión (2026-09-25).** El árbol de Coordinación de Notion se vació ese
mismo día a petición del usuario: la revisión se trabaja aquí, en el repo, que es donde está el contexto. Notion
volverá a tener copia solo cuando la revisión se cierre y pase al canon (`Docs/Game/`). Incluye lo decidido en
las sesiones web del viaje: D41, D42 y D43.

**Sigue en modo diseño:** nada de esto va al código ni al canon hasta que la revisión esté cerrada.

Revisión de `Docs/Coordinacion/propuestas/BA-006_progresion_historica_y_roster.md` contra el canon
(`Docs/Game/`). Aquí se apuntan las decisiones del usuario y lo que queda abierto; el canon se escribe al
cerrar la revisión, no antes. Iniciada 2026-09-15.

## Decisiones

### Modelo de tecnología

**D1 — Modelo de tecnología: el de BA-006.** Tecnologías conocidas/adoptadas por Facción, catálogo con
requisitos y cuatro puertas. Sustituye al principio de Doc 5.7 ("no hay árbol tecnológico abstracto") y a la idea
de "Planos de X" de Doc 6.5.

**D2 — Los capítulos globales se llaman ERAS.** "Capítulo" queda solo para las épicas de los Aedas (Doc 6.2).
Los ids de BA-006 (`reinos_palaciales`…`reforma_macedonica`) pasan a ser ids de Era.

**D45 — `reforma_macedonica` se queda como TECNOLOGÍA; la Era V necesita nombre propio** (2026-09-25). BA-006
usaba el mismo id para la Era V y para la tecnología compuesta que la caracteriza. Gana la tecnología. **Falta
elegir el id y el nombre de la Era V.**

**D4 — La cronología va de ~1300 a 323 a. C.** (hasta Alejandro Magno), como dice BA-006. El canon aún habla
solo de "Edad Oscura del Bronce": hay que reflejarlo.

**D10 — Sin régimen de compatibilidad para las partidas en marcha.** El playtest arranca de cero con cada
cambio de motor, así que la Era I sigue el flujo de BA-006 tal cual (solo `leva_comunal` y
`hostigamiento_tribal` vienen de inicio; el resto se adopta).

**D25 — La tecnología la adopta la FACCIÓN; cada una tiene requisitos de aparición; se paga con recursos y oro.**

**D29 — Las cuatro puertas son la regla del roster universal.** Cualquier asentamiento, sea de la cultura que
sea, que cumpla para una tropa concreta las cuatro condiciones, la recluta:

```text
Era permitida por el servidor
  + tecnología adoptada por la Facción
  + edificio y nivel interno en el asentamiento
  + población, equipo, animales y recursos disponibles
  = reclutamiento permitido
```

**D30 — El requisito de aparición de una tecnología es un LOGRO DEL SERVIDOR + un HITO DE LA FACCIÓN.** La
aparición tiene que sentirse como un logro compartido de todo el servidor y, a la vez, especial para la primera
Facción que lo consigue. Ejemplo dado por el usuario, `logistica_campana`:
- *logro del servidor:* hay un camino compartido en mundo abierto usado por al menos 3 Facciones y 10 rutas de
  comercio (depende de la fusión de caminos, `Docs/Mecanicas a desarrollar.md` §3);
- *hito de la Facción:* un asentamiento de nivel 4, la Facción en un nivel mínimo, y al menos 3 columnas de
  ejército moviéndose por el mundo a la vez.

Los conceptos sin definir de BA-006 (institución política, ejército permanente…) son tecnologías de una rama
institucional con requisitos de este tipo, no condiciones sueltas.

**D31 — Reglas del logro del servidor y del hito de la Facción:**
- **El logro del servidor queda fijado para siempre** en cuanto se consigue, y lo canta un Aeda en la crónica.
- **"El primero"** es la primera Facción que cumple su hito cuando el logro del servidor ya existe; gana lo que
  dice BA-006: prestigio, crónica y una ventaja temporal de adopción, sin monopolio.
- **Las Eras avanzan con esos mismos logros del servidor**: cada Era tiene su lista, y se pasa a la siguiente al
  cumplir suficientes. Sustituye a los cuatro ejes de BA-006.
- **Las Facciones NPC cuentan** para los logros del servidor.
- **Comercio, Aedas y conquista se saltan el hito de la Facción, nunca el logro del servidor**: se consigue
  pagando a una Facción que ya la tiene adoptada (D39), pagando oro a un Aeda, o conquistando (D35). El hito
  propio es la vía del desarrollo.
- **Una "ruta de comercio" es una caravana que pasa por ese camino**, sea de la Facción que sea.

**D32 — Adoptar una tecnología: la adopta el REY, estando en la CAPITAL, y paga el almacén de la capital.**

**D33 — La capital se ELIGE**; la mecánica es `Docs/Mecanicas a desarrollar.md` §13 (hoy el código usa como
proxy el asentamiento más antiguo). D32 depende de ella.

**D34 — Calendario de Eras dentro de la season de 12 meses.** El servidor arranca en una etapa de inicio (solo
las dos tecnologías de inicio). **Una Era llega cuando se cumplen sus logros, pero nunca antes de su plazo
mínimo** (seis tramos):

**D48 — Cómo avanza una Era (revisa D34)** (2026-09-25):
- **El servidor arranca YA en la Era I, en progreso.** No hay etapa de "Inicio": `leva_comunal` y
  `hostigamiento_tribal` son las tecnologías con las que empieza la Era I.
- Cada logro del servidor cumplido **va llenando** su Era.
- **La Era N+1 empieza cuando se completan TODOS los logros de la Era N, o cuando se agota el plazo del tramo**,
  lo que pase antes. El plazo es un **techo**, no un suelo: completar la Era entera la cierra antes de tiempo.
- Si la Era N+1 empieza por tiempo, **los logros pendientes de la Era N quedan abiertos** y se pueden completar
  igual (siguen siendo condición de aparición de sus tecnologías).
- **Ninguna tecnología de la Era N+1 es adoptable** hasta que ocurra una de las dos cosas.

Esto cierra el "cuántos logros hacen falta para pasar de Era": **todos**, o el plazo.

**D44 (revisado por D48) — La Era V ocupa 6 de los 12 meses, no 8.** Que la última Era sea la mitad de la season
es deliberado —es la meseta de final de partida—, pero ~8 meses era demasiado. Sin etapa de Inicio quedan cinco
tramos. Reparto propuesto, **a confirmar**:

| Tramo | Plazo máximo |
|---|---|
| Era I → Era II | 5 semanas |
| Era II → Era III | 6 semanas |
| Era III → Era IV | 7 semanas |
| Era IV → Era V | 8 semanas |
| Era V → fin de la season | 26 semanas (~6 meses) |

**Aprobado el 2026-09-25.** En semanas corridas desde el arranque del servidor: Era I 0-5, Era II 5-11, Era III
11-18, Era IV 18-26, Era V 26-52. Las cuatro primeras Eras ocupan 26 semanas (6 meses) y la Era V las otras 26.
Cada tramo dura más que el anterior: la partida se ensancha según hay más que construir en ella.

**D35 — Conquista:** al conquistar un asentamiento que ya reclutaba con una tecnología, esa tecnología le
aparece al Rey del conquistador para adoptarla (si no le había aparecido ya). Se paga como cualquier adopción.

**D39 — El comercio de una tecnología es un PAGO** a la Facción que ya la tiene adoptada.

**D40 — Rama institucional:** "ejército permanente" e "instrucción profesional" se funden en una sola
tecnología (`ejercito_permanente`). Las tecnologías institucionales **dan efecto propio**, que se define al
definir el árbol.

### Eras y season

**D49 — Qué nivel de asentamiento le toca a cada Era** (2026-09-25). Regla de ritmo: el desbloqueo tiene que ser
fluido, ni todo de golpe ni demasiado espaciado, y **una Era no puede colgar su contenido de un nivel de
asentamiento que en esa Era casi nadie alcanza**.

| Era | Niveles de asentamiento |
|---|---|
| I | 1 y 2; **pocos** asentamientos en 3 |
| II | 1, 2 y 3; **meseta en el 3** (tecnología y tropa concentradas ahí, con cosas sueltas de niveles menores) |
| III | concentrada en 3, **primeras cosas de nivel 4** |
| IV | cosas de nivel 3, **concentrada en 4** |
| V | concentrada en 3 y 4, **primeras cosas de nivel 5** |

De aquí sale P2, y de aquí en adelante toda tropa, edificio o tecnología nueva se comprueba contra esta tabla.

**D54 — El techo de nivel de asentamiento por Era es DERIVADO, no físico** (2026-09-26). No hay una regla que
diga "en la Era II no se puede pasar del nivel 3": el techo sale de que **alguna tecnología de la Era desbloquee
algo necesario para esa subida** (un bien del coste de la obra de ascenso, un edificio, un requisito). La tabla de
D49 se cumple porque ese algo no existe antes de su Era.

**Reparto aprobado el 2026-09-26:**
- **2 → 3:** la obra de ascenso pide **lingotes de bronce** (en vez de cobre y cuero curtido). Los desbloquea
  `aleacion_bronce` (Era I), cuyo logro exige traer estaño por comercio: pocos niveles 3 en la Era I, meseta en la
  Era II.
- **3 → 4:** lo desbloquea **`instituciones_civicas`** (Era III), que así recibe el efecto propio que D40 dejó
  pendiente. **Falta decidir qué desbloquea exactamente** (un edificio o un bien necesario para la obra).
- **4 → 5:** necesita una tecnología de la Era V, y **no hay ninguna cívica en la Era V** (todas son militares
  macedónicas). Pendiente: crear una o atar el Palacio a una tecnología de la Era V.

**D3 — La Maravilla exige que el servidor haya recorrido todas las Eras** (Era V alcanzada), además del
requisito actual de asentamiento en nivel 5 (Doc 4.2.1). **La season termina por tiempo** (12 meses) aunque
nadie la haya construido. La Facción-legado es un tema aparte, por discutir.

### Tropas, equipo y edificios

**D5 — Caballos y livestock.**
- Los caballos se **compran con oro**, de momento. Cuando exista la mecánica de captura y cría, habrá las dos
  fuentes. **La caballería paga su caballo en oro al reclutar** (y al reponer).
- El recurso `livestock` es **ganado vacuno**. Deja de ser la bolsa ovejas/vacas/caballos de Doc 1.4.

**D6 — Carros de guerra en la Era I**, no en la II (el carro es el arma del periodo palacial), y **sin
`equitacion_militar`**: la montura llega después que el carro.

**D7 — El ariete es EQUIPO, no escuadrón, y está disponible desde Carpintería nivel 1**, sin tecnología (el
canon ya lo daba ahí, Doc 4.2.1).

**D8 — La caballería es la ÚNICA unidad que caza las caravanas rápidas** (a caballo y de contrabando, 24). Su
velocidad pasa de 24; ninguna tropa a pie las alcanza.

**D9 — Escalones, catálogo y roster como los trabaja BA-006.** La Era no es un escalón; el arco compuesto sigue
siendo caro y fuerte pero deja de ser la culminación; los catálogos llevan versión y los tickets la congelan;
los números se calibran después con simulación.

**D11 — La Fundición gana un nivel interno 3** (lo pide `forja_hierro_estandarizada`).

**D43 — Armería 3 pide Fundición 2, no Fundición 3** (corrige G2 de P1; D11 se mantiene intacta). Motivo: D11
ata el nivel 3 de la Fundición a `forja_hierro_estandarizada` (Era III), pero G2 lo exigía para la Armería 3,
que hace falta ya en Era II para `bronce_calidad_militar`; eso metía una doble puerta de nivel 3 en el tramo más
corto del calendario (D34). Con Fundición 2 se cumple igual lo que buscaba G2 (quitarle a la Armería 3 la
dependencia del Palacio). **Fundición 3 queda para la Era III** y tiene que ganarse el nivel por sí misma (más
rinde de lingotes, P1) más el arma de hierro de calidad. Detectado el 2026-09-17 con la curva de progresión,
decidido el 2026-09-25.

**D24 — El poder del héroe sigue siendo el de una unidad de la tropa de élite** (Doc 5.1). Al entrar élites más
fuertes en el catálogo, sube con ellas, y con él la defensa de la caravana sin escolta (Doc 3.10).

**D28 — El edificio de caballería se llama CABALLERIZAS** (no Establos: el livestock vacuno ya vive en el Corral).

**D36 — Hierro, según la historia:** el mineral de hierro es **mucho más abundante que el estaño**; se extrae con
una **mina de hierro**. El hierro temprano es malo (blando) y mejora con la tecnología. **Solo armas de hierro**:
las armaduras siguen siendo de bronce (como ya dice BA-006).

**D37 — Caballerizas:** mismo coste y mismos requisitos que el Barracón (Doc 4.2.1), huella de **3×2**. Cupo de
guarnición por héroe residente: **nivel 1 → 0**; en los niveles siguientes, primero el coste de Liderazgo de la
caballería más barata, y en el último nivel el de **una unidad de élite de caballería** (derivado del catálogo).

**D38 — Primer lote de tropas nuevas tras CQ-003:** carros de guerra (Era I), Hequetai (guardia de bronce) y
jinetes asirios (exploradores montados), de la Era II.

## Culturas

Corrige BA-006 ("las culturas aportan disponibilidad, aspecto y doctrina inicial"): la cultura no da ni quita
tropas ni tecnologías.

**D12 — La cultura es del ASENTAMIENTO**: una puntuación por cultura, por debajo, que el jugador no elige ni
modifica directamente; la moldean sus decisiones.

**D13 — El roster es universal** (D29). La cultura no interviene. (Excepción: las tropas de los campamentos de
mercenarios, D26.)

**D14 — Qué suma puntos de cultura en un asentamiento:**
- los soldados reclutados allí de tropas propias de una cultura (las reposiciones cuentan); las tropas
  **neutras** (como la milicia) no suman a ninguna;
- las tropas más usadas en batalla por sus **miembros, que son los héroes residentes** en la plaza (D20);
- políticas propias de cada cultura, que dan un beneficio al asentamiento y suman puntos a esa cultura;
- las tecnologías que adopta la Facción, que inclinan hacia dónde van ella y sus asentamientos;
- (a estudiar) marcadores del mapa: costa, bioma, cercanía a ciertos elementos del generador de mapas.

**D15 — Eras y cultura son independientes, y el generador de mapas no asigna cultura** (el mapa es general).

**D16 — Cada edificio se construye con el estilo de la cultura dominante en ese momento y lo conserva.**

**D17 — Manda la cultura con más puntos.**

**D18 — De momento la cultura es SOLO aspecto visual**: no tiene efecto mecánico propio (las políticas
culturales sí dan su beneficio).

**D19 — Al fundar, el asentamiento es NEUTRO, sin puntos** (neutro también tiene su estilo visual). **Al
conquistarlo no cambia**: conserva los puntos que tenía.

**D20 — "Miembros" de un asentamiento son sus héroes residentes.**

**D21 — Mejorar un edificio (nivel interno) le da el estilo de la cultura dominante del momento.**

**D22 — Las escuadras no cambian de aspecto por cultura**: solo los edificios.

**D23 — El canon de culturas va en un documento nuevo propio.**

**D27 — La Facción NO tiene cultura de conjunto**: la cultura existe solo por asentamiento.

**Propuesta de catálogo (a confirmar):** Neutra, Micénica, Hitita/Anatolia, Egipcia, Mesopotámica,
Fenicia/Levantina, Helénica (incluye Macedonia y **Tracia**, decidido 2026-09-16), Persa.

**Pendiente de culturas:**
- Confirmar el catálogo y la afinidad de cada tropa (las de nombre histórico ya la llevan: ver el roster de P1).
- Afinidad cultural de cada tecnología, y a qué asentamientos suman sus puntos al adoptarla la Facción.
- Políticas culturales: catálogo; ¿las tiene cualquiera o se desbloquean con puntos?
- Desempate cuando dos culturas tienen los mismos puntos; ¿los puntos decaen con el tiempo?
- Contrato con Conquest: cada `Edificio` llevaría su estilo (doc 01 §17, `visualSeed`/`visualCatalogVersion`);
  propuesta CQ para los modelos por cultura.

## Campamentos de mercenarios

**D26 — Campamentos de mercenarios en mundo abierto**, como los de bandidos, pero **no se pueden atacar**: son
el único sitio donde se reclutan ciertas unidades específicas.
- **Son escuadrones normales del héroe** (cuentan Liderazgo, uno por tropa), pero **solo se rellenan en esos
  campamentos**.
- **Cada campamento tiene un origen fijo** y lo que ofrece depende de la Era.
- **Aparecen en un punto aleatorio cerca de los asentamientos y, una vez aparecen, se quedan.** Hay un número
  limitado por servidor.
- **Recluta cualquier héroe con su columna delante.**
- **Suman puntos de cultura** (la de su origen) **a la plaza donde reside el héroe.**

**Pendiente de mercenarios:** coste (¿oro?, con el recargo por mala reputación que ya prevé Doc 2.7); cuántos
por servidor y a qué distancia de los asentamientos; ritmo de aparición hasta el tope; si se ven solo en vivo o
quedan en la memoria (son fijos, como un camino); catálogo de orígenes y qué tropa ofrece cada uno por Era;
reponer: ¿en cualquier campamento del mismo origen o solo en el suyo?; `infanteria_profesional` deja de dar
mercenarios (ahora vienen de los campamentos); definiciones de escuadra en Conquest para cada tropa mercenaria.

## Catálogo de tecnologías (INCOMPLETO — en construcción)

> **D46 — El catálogo y el roster se afinan Era por Era** (2026-09-25). Las dos tablas de abajo son un borrador
> de trabajo, **no una lista cerrada**: orden de aparición, Eras, edificios, tecnologías y equipo se revisan
> una Era cada vez, cruzando cada fila con lo ya decidido. Lo que se detecte en esas pasadas (ver *Afinado por
> Eras*, más abajo) se resuelve en la pasada de su Era, no antes.

Cambios aplicados sobre BA-006: carros a la Era I sin equitación (D6), ariete fuera del árbol (D7),
`ejercito_permanente` absorbe la instrucción profesional (D40), rama institucional nueva (D30). Las dos
tecnologías "de inicio" no tienen requisitos (D10). Columnas de logro e hito: se rellenan con el usuario.

| Tecnología | Era | Desbloquea | Logro del servidor | Hito de la Facción |
|---|---|---|---|---|
| `leva_comunal` | inicio | milicia de lanceros | — (de inicio) | — |
| `hostigamiento_tribal` | inicio | honderos; escaramuzadores con jabalina (piden arma de cobre: en la práctica esperan a `metalurgia_cobre`) | — (de inicio) | — |
| `metalurgia_cobre` | I | receta del arma de cobre (Armería) | se han extraído X de cobre entre todas las Facciones | Fundición activa |
| `aleacion_bronce` | I | recetas del lingote (Fundición 2) y del arma de bronce (Armería 2); **tropas con arma de bronce sin tecnología propia: hacheros ligeros, espadachines de bronce, hacheros armados** | X caravanas han llegado a destino cargadas de estaño (Uluburun) | Fundición nivel 2 y estaño en el almacén de la capital |
| `escudos_ligeros` | I | lanceros con escudo de mimbre | X campamentos de bandidos destruidos | Barracón activo |
| `armamento_palacial` | I | espadachines de cobre | X asentamientos del servidor en nivel 2 (lineal B) | Armería activa (y `metalurgia_cobre` adoptada) |
| `arqueria_palacial` | I | arqueros | X batallas libradas en el servidor | Galería de tiro activa |
| `carros_guerra` | I | carros de guerra (Caballerizas 2) y la pieza carro de guerra (Carpintería 1) | X animales comprados en todo el servidor (caravanas incluidas) | Caballerizas y Carpintería activas *(ajustado tras P1: antes Carpintería nivel 2; a confirmar)* |
| `bronce_calidad_militar` | II | recetas del arma de bronce de calidad y de la armadura de bronce (Armería 3) | *borrador:* X piezas de equipo de bronce fabricadas en el servidor (Dendra) | *borrador:* Armería nivel 3 |
| `forja_hierro_temprana` | II | mina de hierro, lingote de hierro (Fundición 2), arma de hierro (Armería 2) | *borrador:* X caravanas destruidas o capturadas en el servidor (colapso del estaño) | *borrador:* Fundición nivel 2 y un yacimiento de hierro en su territorio |
| `panoplia_bronce` | II | Hequetai (guardia de bronce), lanceros pesados micénicos | *borrador:* X escuadrones reclutados en total en el servidor (Vaso de los Guerreros) | *borrador:* `bronce_calidad_militar` adoptada y Barracón nivel 2 |
| `disciplina_formacion` | II | formaciones cerradas (táctico, en Conquest) | *borrador:* primera vez que un asedio resiste en el servidor (Batalla del Delta) | *borrador:* Barracón nivel 2 |
| `arco_compuesto` | II | arqueros con arco compuesto | *borrador:* X arqueros reclutados en el servidor | *borrador:* Carpintería nivel 2 y Galería de tiro nivel 3 |
| `equitacion_militar` | II | jinetes asirios | *borrador:* X carros de guerra reclutados en el servidor (Assurnasirpal) | *borrador:* Caballerizas activas |
| `carpinteria_militar` | II | escalas y defensas de campaña (equipo) | *borrador:* primera conquista de un asentamiento en el servidor (Dapur) | *borrador:* Carpintería nivel 2 |
| `instituciones_civicas` | III | (institucional) | | |
| `ciudadania_militar` | III | hoplitas ciudadanos | | |
| `falange_hoplita` | III | Espartiatas | | |
| `pantalla_escaramuzadores` | III | peltastas | | |
| `arqueria_especializada` | III | arqueros escitas | | |
| `forja_hierro_estandarizada` | III | arma de hierro de calidad (Armería 3) | | |
| `bronce_laminado` | III | armadura de bronce de calidad (Armería 3) | | |
| `caballeria_organizada` | III | jinetes escitas, caballería asiria | | |
| `trabajos_asedio` | III | cuadrillas de asedio | | |
| `ejercito_permanente` | IV | (institucional) | | |
| `logistica_campana` | IV | (institucional) | camino compartido: ≥3 Facciones y ≥10 rutas (D30) | asentamiento nivel 4 + nivel de Facción X + ≥3 columnas en marcha a la vez |
| `cuerpo_oficiales` | IV | (institucional) | | |
| `infanteria_profesional` | IV | Epílektoi, Batallón Sagrado, Inmortales | | |
| `hostigadores_profesionales` | IV | Peltastas de Ifícrates | | |
| `carga_caballeria` | IV | caballería tesalia, caballería noble persa | | |
| `maestria_asedio` | IV | asalto coordinado, máquinas avanzadas | | |
| `maestros_obras_militares` | IV | obras de asedio más baratas | | |
| `reforma_macedonica` | V | (compuesta); agrianos | | |
| `falange_sarisa` | V | falangitas | | |
| `cuerpo_hipaspistas` | V | hipaspistas | | |
| `formacion_cuna` | V | pródromos | | |
| `companeros_reales` | V | Compañeros | | |

## P1 — equipo y requisitos de edificios para las 5 Eras

**Aprobada 2026-09-16** (G1-G4): la escalera de equipo tal cual (G1); Armería 3 pide Fundición 3 en vez de
Palacio (G2) **[corregido por D43: Fundición 2]**; Armería 2 pide Fundición 2 en vez de Carpintería (G3);
tecnología nueva `bronce_laminado` (Era III) para la armadura de bronce de calidad (G4). Los nombres de las
tropas genéricas se cerraron el mismo día (N1-N3, R1-R3, abajo).

Motivo: los requisitos de edificios y equipo se hicieron cuando las 11 tropas eran todo el roster. P1 los
reordena para las cinco Eras.

**Principio:** el nivel del edificio militar dice la CLASE de tropa (1 leva/ligera, 2 línea, 3 pesada/élite); la
tecnología dice CUÁNDO existe (Era); el equipo dice CON QUÉ se hace. La Era pone el mínimo; el nivel del
asentamiento pone el cuándo real de cada plaza.

**Escalera de equipo** (recetas placeholder):

| Pieza | Tecnología (Era) | Dónde | Receta por unidad | Referente |
|---|---|---|---|---|
| Arma de madera | inicio | Armería 1 | 2 madera | — |
| Arma de cobre | `metalurgia_cobre` (I) | Armería 1 | 1 lingote de cobre + 1 madera | — |
| Arma de bronce | `aleacion_bronce` (I) | Armería 2 | 1 lingote de bronce + 2 madera | armas micénicas |
| Arma de bronce de calidad | `bronce_calidad_militar` (II) | Armería 3 | 5 lingotes de bronce + 5 madera | espada Naue II (~1200) |
| **Arma de hierro** (nueva) | `forja_hierro_temprana` (II) | Armería 2 | 1 lingote de hierro + 2 madera | hierro forjado blando: abundante y barato, no mejor que el bronce |
| **Arma de hierro de calidad** (nueva) | `forja_hierro_estandarizada` (III) | Armería 3 | 2 lingotes de hierro + 3 madera (carbón para templar) | acero templado: lanza hoplita, xiphos, punta de sarisa |
| Armadura básica | inicio | Armería 1 | 5 cuero | — |
| Armadura intermedia | sin tecnología | Armería 2 | 1 lingote de cobre + 5 cuero curtido | — |
| Armadura de bronce | `bronce_calidad_militar` (II) | Armería 3 | 1 lingote de bronce + 5 cuero de calidad | panoplia de Dendra, coraza acampanada |
| **Armadura de bronce de calidad** (nueva) | `bronce_laminado` (III, tecnología nueva) | Armería 3 | 2 lingotes de bronce + 5 cuero de calidad | coraza musculada clásica |
| **Carro de guerra** (pieza nueva) | `carros_guerra` (I) | Carpintería 1 | madera + cuero curtido + lingote de bronce | ruedas de radios |
| Caballo | — | se compra con oro (D5) | — | — |

Materias nuevas: mineral de hierro (**mina de hierro**) y **lingote de hierro** (Fundición 2 con
`forja_hierro_temprana`). Las armaduras siguen siendo de bronce en todas las Eras (D36): el estaño nunca deja de
importar.

**Cambios de requisitos de edificios:**

| Edificio y nivel | Hoy | Propuesta | Por qué |
|---|---|---|---|
| Armería 3 | nivel 3 + Palacio (≈ nivel 4) | nivel 3 + **Fundición 2** (D43; G2 decía Fundición 3) | su equipo es de las Eras II-III; con Palacio llegaría en la IV-V |
| Armería 2 | nivel 2 + Carpintería (≈ nivel 3) | nivel 2 + **Fundición 2** | el arma sale de la fundición; las tropas siguen esperando a Barracón/Galería 2 |
| Fundición 2 | cobre, estaño, bronce | + lingote de hierro | con `forja_hierro_temprana` |
| Fundición 3 (D11) | no existe | nivel 3; más rinde de lingotes; la pide `forja_hierro_estandarizada` (Era III) | D43 la saca de la cadena de la Armería 3 |
| Mina de hierro | no existe | como la de cobre, con `forja_hierro_temprana` | — |
| Carpintería 1 | sin recetas | + carro de guerra y ariete (D7) | — |
| Barracón, Galería, Caballerizas | 1: nivel 2 · 2: + Carpintería · 3: + Palacio (Galería: Carpintería 2) | sin cambio | el nivel del edificio es la clase de tropa |

**Roster con el equipo nuevo — TABLA INCOMPLETA** (D46; escalón tentativo, poder y escalón se calibran con
simulación, D9). Nivel efectivo = nivel de asentamiento que hace falta de verdad. Falta al menos todo el árbol
de asedio (`trabajos_asedio`, `maestria_asedio`, `maestros_obras_militares` desbloquean cuadrillas y máquinas
que no tienen fila aquí).

| Era | Tropa | Tecnología | Edificio | Equipo por soldado | Escalón | Nivel efectivo |
|---|---|---|---|---|---|---|
| inicio | Milicia de lanceros | `leva_comunal` | Centro Urbano | 2 madera | 1 | 1 |
| inicio | Honderos | `hostigamiento_tribal` | Galería 1 | arma de madera | 2 | 2 |
| inicio | Escaramuzadores con jabalina | `hostigamiento_tribal` | Galería 2 | **arma de cobre** (hoy de bronce) + básica | 3 | 3 |
| I | Lanceros con escudo de mimbre | `escudos_ligeros` | Barracón 1 | arma de madera | 1 | 2 |
| I | Espadachines de cobre | `armamento_palacial` | Barracón 1 | arma de cobre + básica | 2 | 2 |
| I | Arqueros | `arqueria_palacial` | Galería 2 | arma de bronce + intermedia | 3 | 3 |
| I | Hacheros ligeros | `aleacion_bronce` | Barracón 2 | arma de bronce + básica | 3 | 3 |
| I | Espadachines de bronce | `aleacion_bronce` | Barracón 2 | 2 armas de bronce + intermedia | 3 | 3 |
| I | Hacheros armados | `aleacion_bronce` | Barracón 3 | arma de bronce + intermedia | 4 | 4 |
| I | **Carros de guerra** | `carros_guerra` | Caballerizas 2 | carro de guerra + 2 caballos + arma de bronce | 4 | 3 |
| II | Lanceros pesados micénicos | `panoplia_bronce` | Barracón 3 | 2 armas de bronce + **armadura de bronce** | 4 | 4 |
| II | **Hequetai** (guardia de bronce) | `panoplia_bronce` | Barracón 3 | **arma de bronce de calidad + armadura de bronce** | 5 | 4 |
| II | Arqueros con arco compuesto | `arco_compuesto` | Galería 3 | 3 armas de bronce + 2 intermedias | 5 | 3 |
| II | **Jinetes asirios** | `equitacion_militar` | Caballerizas 1 | arma de hierro + caballo | 2 | 2-3 |
| III | **Hoplitas ciudadanos** | `ciudadania_militar` | Barracón 2 | arma de hierro + armadura de bronce | 3 | 3 |
| III | **Espartiatas** | `falange_hoplita` | Barracón 3 | arma de hierro de calidad + armadura de bronce de calidad | 4 | 4 |
| III | **Peltastas** | `pantalla_escaramuzadores` | Galería 2 | 2 armas de hierro (jabalinas), sin armadura | 3 | 3 |
| III | **Arqueros escitas** | `arqueria_especializada` | Galería 3 | arma de hierro + básica | 4 | 3 |
| III | **Jinetes escitas** | `caballeria_organizada` | Caballerizas 2 | arma de bronce + básica + caballo | 3 | 3 |
| III | **Caballería asiria** | `caballeria_organizada` | Caballerizas 2 | arma de hierro + intermedia + caballo | 3 | 3 |
| IV | **Epílektoi ("los escogidos")** | `infanteria_profesional` | Barracón 2 | arma de hierro de calidad + armadura de bronce | 3 | 3 |
| IV | **Batallón Sagrado** | `infanteria_profesional` | Barracón 3 | arma de hierro de calidad + armadura de bronce de calidad | 5 | 4 |
| IV | **Inmortales** | `infanteria_profesional` | Barracón 3 | arma de hierro de calidad + arma de bronce (arco) + armadura intermedia (escamas) | 5 | 4 |
| IV | **Peltastas de Ifícrates** | `hostigadores_profesionales` | Galería 3 | 2 armas de hierro de calidad + básica | 4 | 3 |
| IV | **Caballería tesalia** | `carga_caballeria` | Caballerizas 3 | arma de hierro de calidad (lanza) + armadura intermedia + caballo; menos coraza, más rápida | 4 | 4 |
| IV | **Caballería noble persa** | `carga_caballeria` | Caballerizas 3 | 2 armas de hierro (jabalinas) + armadura de bronce + caballo; más coraza, más lenta | 4 | 4 |
| V | **Falangitas** | `falange_sarisa` | Barracón 2 | arma de hierro de calidad (sarisa) + intermedia (lino y cuero) | 3 | 3 |
| V | **Hipaspistas** | `cuerpo_hipaspistas` | Barracón 3 | arma de hierro de calidad + armadura de bronce de calidad | 5 | 4 |
| V | **Pródromos** | `formacion_cuna` | Caballerizas 2 | arma de hierro + caballo | 3 | 3 |
| V | **Compañeros** | `companeros_reales` | Caballerizas 3 | arma de hierro de calidad + armadura de bronce de calidad + caballo | 5 | 4 |
| V | **Agrianos** | `reforma_macedonica` | Galería 3 | 2 armas de hierro de calidad (jabalinas) + básica | 5 | 3 |

**Decidido sobre el roster (2026-09-16, N1-N3):**
- Infantería profesional → **Epílektoi ("los escogidos")**, helénica.
- Hostigadores de élite → **Peltastas de Ifícrates**.
- Caballería pesada → **dos tropas distintas**, con atributos distintos: **Caballería tesalia** (helénica) y
  **Caballería noble persa** (persa).
- Guardia real → **dos tropas distintas**, con culturas y valores distintos: **Batallón Sagrado** (helénica) e
  **Inmortales** (persa).
- **Agrianos** entran en la Era V.
- **Tracia no es cultura propia: sus tropas (peltastas, peltastas de Ifícrates, agrianos) suman a la Helénica.**
- (R1) Equipo de las tropas gemelas **confirmado** tal como está en la tabla. Comparten tecnología y edificio; sus
  diferencias en BronzeAge son equipo, poder, velocidad y coste; el comportamiento táctico es de Conquest.
- (R2) Los **agrianos** los desbloquea `reforma_macedonica` (Galería 3, escalón 5).
- (R3) Nombres **confirmados**: Hequetai (guardia de bronce, micénica), Espartiatas (hoplitas veteranos,
  helénica), Jinetes asirios (exploradores montados, mesopotámica), Caballería asiria (lanceros montados,
  mesopotámica), Arqueros escitas (arqueros regionales, neutra), Jinetes escitas (arqueros a caballo, neutra).

## Beneficios de asentamiento de nivel 4-5 (en discusión, 2026-09-17)

Detectado al cruzar D34 con el "nivel efectivo" de P1: **los niveles 4 y 5 de asentamiento casi no tienen
contenido propio.** Para tropas está bien (el roster se cierra en nivel efectivo 4), pero hoy el nivel 5 solo
existe como requisito de la Maravilla (D3) y el nivel 4 como requisito de los Aedas residentes
(Palacio/Nobleza). Falta algo que haga querer llegar al máximo. Criterio del usuario: **beneficios que no sean
de reclutamiento de tropas, disruptivos, sin salirse del canon histórico.**

**D41 — Descartados como beneficio de nivel 4-5:** festival / juegos panhelénicos, corte y embajadas (capa
diplomática), necrópolis real y tumbas monumentales, templo / oráculo, archivo real de escribas.

**D42 — La capital NO se ata al nivel 4:** se queda como está (D33), porque la eligen otras mecánicas y atarla a
un nivel tan alto rompería su uso.

**En estudio (ninguno decidido):**
- **Murallas:** hay tres tipos (empalizada, muralla y una superior). El tercer tipo sería de nivel 4, y el
  **segundo anillo de murallas** de nivel 5.
- **Gremios especiales de nivel alto:** mecánica ya prevista en `Docs/Mecanicas a desarrollar.md`; encaja como
  contenido de nivel 4/5.
- **Talleres de lujo** (púrpura, marfil, orfebrería): bienes de alto valor que solo produce una plaza de nivel
  4/5; se cruzaría con los gremios.
- **Emporio comercial internacional:** el nivel 5 atrae caravanas y rutas por sí mismo (referentes: Ugarit, Ura).

**Pendiente de nivel 4-5:**
- Definir de verdad las cuatro líneas vivas (murallas, gremios, talleres de lujo, emporio) y decidir cuáles
  entran.
- Talleres de lujo y emporio: el usuario los quiere, pero "hay que abordarlo" — falta el cómo.
- Revisar si los gremios de `Mecanicas a desarrollar` se dan por buenos tal cual o se rehacen para esta línea
  (esa página no se ha leído en esta revisión).

## Afinado por Eras (D46) — notas que se resuelven en la pasada de cada Era

Lo detectado al repasar el documento el 2026-09-25. Cada punto se decide cuando se trabaje SU Era, no antes.

**Era I:** resuelta el 2026-09-25 — ver la sección *Era I* más abajo (D48-D53, P2).

**Era II:**
- **Los jinetes asirios llevan arma de hierro, que no la da su tecnología:** los desbloquea
  `equitacion_militar`, pero el arma sale de `forja_hierro_temprana`, otra tecnología de la misma Era y sin
  relación. Se puede adoptar la primera y no poder reclutarlos. Hay precedente aceptado (los escaramuzadores de
  inicio esperan a `metalurgia_cobre`), así que puede ser deliberado; decidirlo en la pasada de la Era II.
- **D38 mete a los Hequetai en el primer lote de Conquest**, y es la tropa con más puertas de su Era
  (Barracón 3 + Armería 3 + `panoplia_bronce` + `bronce_calidad_militar` adoptada).

**Era III y siguientes:**
- El árbol de asedio no tiene ninguna fila en el roster (ver la nota de la tabla).

## Era I — CERRADA el 2026-09-25 (D48-D53, P2 aprobada)

Id `reinos_palaciales`. Con D48 el servidor **arranca aquí**. Criterio de reparto: D49 (Era I vive en los
niveles 1-2, con poca cosa en el 3).

**D50 — P2: la escalera de edificios se recoloca a D49.** Hoy la Carpintería exige asentamiento de nivel 3
(`src/constants.ts`, `carpinteria.requisitoNivelAsentamientoConstruccion: 3`) y de ella cuelgan Barracón 2,
Galería 2 y Caballerizas 2; eso metía media Era I en el nivel 3 y una tropa en el 4. Cambios (**a confirmar**):

| Edificio y nivel | Hoy | P2 | Por qué |
|---|---|---|---|
| Carpintería (construcción) | asentamiento nivel 3 | **nivel 2** | es el cuello de botella de la Era I: abre Barracón 2, Galería 2 y Caballerizas 2 |
| Barracón 3 | nivel 3 + Palacio (≈ nivel 4) | **nivel 3 + Carpintería 2** | igual que la Galería 3; deja la infantería pesada en la meseta de nivel 3 de la Era II (D49) |
| Caballerizas 3 | igual que Barracón | **nivel 3 + Carpintería 2** | ídem |
| Fundición 3 (D11) | no existe | **nivel 4** | la puerta del nivel 4 pasa a ser el EQUIPO de calidad (Era III), no el cuartel |
| Pieza del carro de guerra | Carpintería 1 (P1) | **Carpintería 2** | es lo que pone los carros en nivel 3, como pide E1 |

**P2 aprobada el 2026-09-25.**

Con esto el nivel 4 deja de ser una puerta de tropa y pasa a ser puerta de equipo de élite (Fundición 3, arma de
hierro de calidad) y de Palacio: justo lo que D49 pide para las Eras III-IV.

**D51 — Tropa nueva: exploradores montados (Caballerizas 1).** Caballería ligera **de exploración, floja en
combate**: da uso al nivel 1 de las Caballerizas, que con D37 no daba cupo ni tropa hasta la Era II. Llega con
una tecnología nueva de la Era I, **`cria_caballar`**, que desbloquea además el propio edificio; `carros_guerra`
pasa a depender de ella. Cadena histórica limpia: primero el caballo (montado a pelo, mensajería y
exploración), luego el carro, y ya en la Era II la `equitacion_militar` como arma de combate — sin romper D6.
**Nombre decidido el 2026-09-25: Exploradores a caballo, cultura NEUTRA** (no suma puntos de cultura, D14).
Se descartaron Medjay (egipcia) y Kartappu (hitita). D51 aprobada entera, incluida la cadena
`cria_caballar` → `carros_guerra` y el logro nuevo de `carros_guerra`.

**D52 — Ninguna tropa de la Era I pasa del nivel 3.** **Los hacheros armados bajan a Barracón 2** (nivel 2).
Los carros llegan en nivel 3 (E1, confirmado) y son la única tropa de la Era I en ese nivel. Consecuencia: el
**Barracón 3 se estrena en la Era II**, con los lanceros pesados micénicos y los Hequetai — justo la meseta de
nivel 3 que pide D49. Su **escalón baja de 4 a 3** (2026-09-25), para respetar el principio de P1: el nivel 2 del
edificio es tropa de línea. Quedan como la mejor tropa de línea de la Era I.

**D53 — Reparto de tecnologías de la Era I**, para que ninguna cargue con tres tropas:
- `armamento_palacial` → espadachines de cobre **+ hacheros ligeros**;
- `aleacion_bronce` → espadachines de bronce + hacheros armados;
- los escaramuzadores con jabalina siguen siendo de `hostigamiento_tribal` aunque su arma de cobre venga de
  `metalurgia_cobre`: es el patrón normal de las cuatro puertas (la tecnología dice qué tropa existe, el equipo
  dice con qué se hace).
- Los números X de los logros quedan como **placeholder** y se calibran con el playtest, igual que poder y
  escalón (D9).

**Catálogo de la Era I (7 tecnologías):**

| Tecnología | Desbloquea | Logro del servidor | Hito de la Facción |
|---|---|---|---|
| `leva_comunal` | milicia de lanceros | — (de arranque) | — |
| `hostigamiento_tribal` | honderos, escaramuzadores con jabalina | — (de arranque) | — |
| `metalurgia_cobre` | arma de cobre (Armería 1) | X de cobre extraído entre todas las Facciones | Fundición activa |
| `aleacion_bronce` | lingote de bronce (Fundición 2), arma de bronce (Armería 2); espadachines de bronce, hacheros armados | X caravanas llegadas con estaño (Uluburun) | Fundición 2 + estaño en el almacén de la capital |
| `escudos_ligeros` | lanceros con escudo de mimbre | X campamentos de bandidos destruidos | Barracón activo |
| `armamento_palacial` | espadachines de cobre, hacheros ligeros | X asentamientos del servidor en nivel 2 (lineal B) | Armería activa + `metalurgia_cobre` adoptada |
| `arqueria_palacial` | arqueros | X batallas libradas en el servidor | Galería de tiro activa |
| `cria_caballar` **(nueva)** | Caballerizas, exploradores montados | X animales comprados en todo el servidor (caravanas incluidas) | Corral activo |
| `carros_guerra` | pieza del carro (Carpintería 2), carros de guerra (Caballerizas 2) | X batallas a campo abierto en el servidor (Qadesh) | Caballerizas 2 y Carpintería 2 activas (= nivel 3) |

**Roster de la Era I (11 tropas) con P2:**

| Tropa | Tecnología | Edificio | Equipo | Escalón | Nivel |
|---|---|---|---|---|---|
| Milicia de lanceros | `leva_comunal` | Centro Urbano | 2 madera | 1 | 1 |
| Honderos | `hostigamiento_tribal` | Galería 1 | arma de madera | 2 | 2 |
| Escaramuzadores con jabalina | `hostigamiento_tribal` | Galería 2 | arma de cobre + básica | 3 | 2 |
| Lanceros con escudo de mimbre | `escudos_ligeros` | Barracón 1 | arma de madera | 1 | 2 |
| Espadachines de cobre | `armamento_palacial` | Barracón 1 | arma de cobre + básica | 2 | 2 |
| Hacheros ligeros | `armamento_palacial` | Barracón 2 | arma de bronce + básica | 3 | 2 |
| Espadachines de bronce | `aleacion_bronce` | Barracón 2 | 2 armas de bronce + intermedia | 3 | 2 |
| Arqueros | `arqueria_palacial` | Galería 2 | arma de bronce + intermedia | 3 | 2 |
| **Exploradores a caballo** (neutra) | `cria_caballar` | Caballerizas 1 | arma de cobre + caballo | 2 | 2 |
| Hacheros armados | `aleacion_bronce` | Barracón 2 | arma de bronce + intermedia | 3 | 2 |
| Carros de guerra | `carros_guerra` | Caballerizas 2 | carro + 2 caballos + arma de bronce | 4 | 3 |

Diez tropas en los niveles 1-2 y una sola en el 3 (los carros), como pide D49. **El ritmo de la Era I ya no lo marca el nivel del
asentamiento sino el orden de adopción**: arranque → `metalurgia_cobre` → `escudos_ligeros` /
`armamento_palacial` → `aleacion_bronce` → `arqueria_palacial` → `cria_caballar` → `carros_guerra`.

## Era II — EN CURSO (desde el 2026-09-27)

Criterio de reparto: D49 (**meseta en el nivel 3**, con cosas sueltas de niveles menores). Con P2 aprobada, el
Barracón 3, la Galería 3 y las Caballerizas 3 piden nivel 3 + Carpintería 2, y la Armería 3 nivel 3 + Fundición 2
(D43): la tabla del roster de arriba, que pone nivel 4 a los lanceros pesados y a los Hequetai, es anterior a P2.

**Catálogo de la Era II (7 tecnologías; logros e hitos aprobados el 2026-09-27, EII-3):**

| Tecnología | Desbloquea | Logro del servidor | Hito de la Facción |
|---|---|---|---|
| `bronce_calidad_militar` | arma de bronce de calidad y armadura de bronce (Armería 3) | X piezas de equipo de bronce fabricadas (Dendra) | Armería 3 |
| `forja_hierro_temprana` | mina de hierro, lingote de hierro (Fundición 2), arma de hierro (Armería 2) | X caravanas destruidas o capturadas (colapso del estaño) | Fundición 2 + yacimiento de hierro en su territorio |
| `panoplia_bronce` | lanceros pesados micénicos, Hequetai | X escuadrones reclutados (Vaso de los Guerreros) | `bronce_calidad_militar` adoptada + **Barracón 3** |
| `disciplina_formacion` | formaciones cerradas (táctico, Conquest) | **X asedios resistidos en combate** en el servidor (Batalla del Delta); no cuentan los rebotes por ocupación | Barracón 2 |
| `arco_compuesto` | arqueros con arco compuesto | X arqueros reclutados | Carpintería 2 + Galería 3 |
| `equitacion_militar` | jinetes asirios | X carros de guerra reclutados (Assurnasirpal) | Caballerizas activas |
| `carpinteria_militar` | escalas y defensas de campaña (equipo) | **primera conquista de una plaza con muralla completa** (Dapur) | Carpintería 2 |

**Roster de la Era II (4 tropas) con P2:**

| Tropa | Tecnología | Edificio | Equipo | Escalón | Nivel |
|---|---|---|---|---|---|
| Jinetes asirios | `equitacion_militar` | Caballerizas 1 | arma de hierro + caballo | 2 | 2 |
| Lanceros pesados micénicos | `panoplia_bronce` | Barracón 3 | 2 armas de bronce + armadura de bronce | 4 | 3 |
| Hequetai | `panoplia_bronce` | Barracón 3 | arma de bronce de calidad + armadura de bronce | 5 | 3 |
| Arqueros con arco compuesto | `arco_compuesto` | Galería 3 | 3 armas de bronce + 2 intermedias | 5 | 3 |

**Decidido el 2026-09-27:**
- **EII-1 — (a)**: los jinetes asirios se quedan con arma de hierro aunque la dé `forja_hierro_temprana`, como los
  escaramuzadores de la Era I (D53).
- **EII-2 — nivel 3** para lanceros pesados, Hequetai y arqueros con arco compuesto (la tabla de arriba).
- **EII-4 — los nodos de hierro son 1,5 veces más frecuentes que los de cobre** (D36).
- **EII-5 — ningún asentamiento por encima del nivel 3 por ahora**: techo provisional en el código
  (`ASCENSO_ASENTAMIENTO.nivelTechoProvisional`) hasta que exista `instituciones_civicas` (D54).

- **EII-3 — logros e hitos aprobados** (la tabla de arriba). Cambios sobre el borrador: `carpinteria_militar` pasa
  a "primera conquista de una plaza con muralla completa" y `disciplina_formacion` a "X asedios resistidos en
  combate" (los dos originales ya se habrían cumplido en la Era I); el hito de `panoplia_bronce` pasa a Barracón 3,
  el edificio donde nacen sus tropas, como en la Era I. Las X quedan como placeholder hasta el playtest (D53).
- Los jinetes asirios se quedan en nivel 2 (Caballerizas 1).

**Pendiente (EII):** EII-5, cuánta meseta en el nivel 3 al acabar la Era, con el batch de la Era II delante. Al
bajar la Era II a código, P1/P2 (Barracón 3 y Armería 3 dejan de pedir Palacio).

## Era III — EN CURSO (desde el 2026-09-27)

Criterio: D49 (concentrada en el nivel 3, primeras cosas de nivel 4). 7 semanas (11-18).

| Tecnología | Desbloquea | Logro del servidor | Hito de la Facción |
|---|---|---|---|
| `instituciones_civicas` | la subida 3 → 4 (D54; **con qué, pendiente: EIII-1**) | X asentamientos del servidor en nivel 3 (nacimiento de la polis) | capital en nivel 3 con Mercado activo |
| `ciudadania_militar` | hoplitas ciudadanos | X asedios resistidos con los residentes dentro (reforma hoplita) | `instituciones_civicas` adoptada + Barracón 2 |
| `falange_hoplita` | Espartiatas | **X batallas libradas usando hoplitas** | `ciudadania_militar` adoptada + Barracón 3 |
| `pantalla_escaramuzadores` | peltastas | X escaramuzadores con jabalina reclutados | Galería 2 + Armería 2 |
| `arqueria_especializada` | arqueros escitas | X arqueros con arco compuesto reclutados | Galería 3 |
| `forja_hierro_estandarizada` | arma de hierro de calidad, Fundición 3 (D11) | X de hierro extraído (acero templado) | Fundición 2 + mina de hierro activa |
| `bronce_laminado` | armadura de bronce de calidad (Armería 3) | X armaduras de bronce fabricadas (coraza musculada) | Armería 3 |
| `caballeria_organizada` | jinetes escitas, caballería asiria | X jinetes asirios reclutados (Tiglat-Pileser III) | Caballerizas 2 |
| `trabajos_asedio` | equipo de asedio de Carpintería 2 | X asedios contra plazas con muralla completa (Laquis) | Carpintería 2 |

**Decidido el 2026-09-27:**
- **EIII-2 — (b)**: la caballería asiria lleva armadura de bronce (Armería 3) y pasa a nivel 3; peltastas y jinetes
  escitas se quedan en nivel 2 como piezas sueltas.
- **EIII-3**: logros e hitos de la tabla, con `falange_hoplita` = X batallas libradas usando hoplitas.
- **EIII-4 — (b)**: las "cuadrillas de asedio" de `trabajos_asedio` son equipo de asedio de Carpintería 2, como las
  escalas de la Era II (coherente con D7).

**Pendiente (EIII):** EIII-1 (qué desbloquea `instituciones_civicas` para el 3 → 4); EIII-5 (cuántas plazas en
nivel 4 al acabar la Era, con el batch delante). Detectado para la Era IV: todo lo que lleve arma de hierro de
calidad pasa a nivel 4 con P2 (Epílektoi, Peltastas de Ifícrates, Falangitas, Agrianos, Caballería tesalia).

## Nobleza, reclutamiento y escalones (2026-09-27)

**Decidido por el usuario:**
- **Población por escalón**: escalones 1-2 se reclutan con **pesants**, el 3 con **artesanos**, y los "altos",
  **4 y 5, solo con Nobleza**. Contradice el canon actual (Doc 5.8: "la Nobleza no se recluta"), que cambia al
  bajar esto a código.
- **Palacio mejorable desde el nivel 2**, con el cupo de nobles duplicado sobre la propuesta: **Palacio 1** en
  asentamiento nivel 2, cupo 80; **Palacio 2** en nivel 3, cupo 240; **Palacio 3** en nivel 4, cupo 400 (sigue
  siendo requisito del nivel 5). Motivo: la élite de las Eras I-II (carros, lanceros pesados, Hequetai, arco
  compuesto) está en nivel 3 y necesita Nobleza.
- **La Nobleza deja de ser intocable**: con hambre sostenida pierde el **1 % por minuto** (pesants y artesanos, el 5 %:
  es la más resistente); el saqueo de una conquista le quita la misma fracción que a las otras clases; y reclutarla
  la saca de la población. Hambre y saqueo, ya en el código y el canon.
- **Artesanos**: aparecen **1,5 veces más rápido** (tasa 0,15), tienen **el doble de sitio** en cada Vivienda (10) y
  los requisitos de nivel piden **el doble** (400 / 800 / 1.600 en los niveles 3 / 4 / 5). Ya en el código y el canon.
- **Las unidades por escuadra son un valor propio de cada tropa**, no del escalón (ya en el código, con los valores
  que daba el escalón).
- **Roster:**
  - Las tropas de leva pasan a **ligeras**: milicia de lanceros y lanceros con escudo de mimbre suben a escalón 2.
  - **Dos tropas nuevas de escalón 1**, reclutables desde el principio como la milicia, muy malas en combate y muy
    desorganizadas: **leñadores** y **granjeros**. `leva_comunal`, Centro Urbano, cultura neutra, poder 1 (la
    milicia tiene 2), 30 hombres por escuadra, velocidad 20; los leñadores pagan 1 madera (su hacha) y los
    granjeros nada (sus herramientas). Lo de "desorganizados" es comportamiento en Conquest (sin formaciones).
  - **Honderos rodios** (helénica), escalón 3: Era III, `pantalla_escaramuzadores` (con los peltastas), Galería 2,
    arma de hierro + armadura básica, nivel 2. Referente: en la Anábasis alcanzaban más que los arqueros persas.
  - **Falangitas: escalón 4**, pero se reclutan con **artesanos**, no con nobleza (excepción a la regla por escalón:
    históricamente eran los "compañeros de a pie", no nobles).
  - **Arqueros escitas: escalón 5**, no peores que su contraparte de la Era II (el arco compuesto, 5).

## Qué cambia en el canon (al cerrar)

- **Doc 6** pasa a ser el documento de tecnología: Eras y su calendario (D34), logro del servidor + hito de
  Facción (D30-D31), estados conocida/adoptada/disponible, catálogo, vías (desarrollo, comercio, Aedas,
  conquista), adopción por el Rey en la capital (D32), y el requisito de Eras de la Maravilla (D3). Se retira
  "Planos de X" (6.5).
- **Doc 5.7**: fuera "no hay árbol tecnológico abstracto" (D1); los carros dejan de estar "sin sitio" (D6); la
  Carpintería no "recluta" armas de asedio, fabrica equipo (D7); Caballerizas (D28, D37).
- **Doc 5.8**: el roster crece por Eras con las cuatro puertas (D29); la caballería paga el caballo en oro (D5).
- **Doc 5.12.5 y 3.10**: cuarta clase de velocidad, montada, por encima de 24; el contrabando y la caravana a
  caballo ya no "escapan de todo": solo de lo que va a pie (D8).
- **Doc 5.15.3**: cupo de guarnición de las Caballerizas (D37).
- **Doc 1.4**: livestock = ganado vacuno; los caballos se compran (D5); mineral de hierro abundante (D36).
- **Doc 1.9 (o sección nueva)**: campamentos de mercenarios (D26).
- **Doc 4.2.1**: la Maravilla pide Era V (D3); Fundición con nivel interno 3 (D11); Mina de hierro (D36);
  Caballerizas (D37); Armería 2 y 3 con sus nuevos requisitos (P1, D43).
- **Documento nuevo de culturas** (D12-D23, D27).
- **Glosario (Doc 0)**: entradas Era, Tecnología, Logro del servidor, Cultura, Capital y Campamento de
  mercenarios; cronología 1300–323 a. C. (D4).
- **Roadmap_Escalado Eje 4**: la Maravilla depende de las Eras y la season acaba por tiempo (D3, D34).

## Pendiente

**Derivado de las decisiones:**
- D45: id y nombre de la Era V (`reforma_macedonica` se queda para la tecnología).
- D51: valores de los exploradores a caballo (poder, velocidad, coste) — el nombre y la cultura ya están.
- D48/D49: **a qué ritmo crece un asentamiento de nivel** — es lo que hace cierta o falsa la tabla de D49.
  Analizado el 2026-09-25 en `Consideraciones/Ritmo_Crecimiento_Asentamientos.md`: el crecimiento va en horas
  y el calendario de Eras en semanas, dos órdenes de magnitud de diferencia. Ahí está la propuesta (techo de
  nivel por Era + calibración) y falta decidirla.
- D34 / P1: **cuándo sube de nivel cada edificio dentro de una Era.** D34 fija los tramos entre Eras, pero dentro
  de un tramo no hay nada que diga a qué ritmo llega cada nivel de Barracón, Galería, Caballerizas, Carpintería,
  Armería o Fundición. Detectado el 2026-09-17 al cruzar D34 con el "nivel efectivo" de P1 en una curva de
  progresión.
- Era II: el borrador de logros e hitos del catálogo está sin aprobar; hay que pasarlo por D49 (meseta de nivel 3).
- El roster pasa de 11 a 32 tropas (D51 añade una) (BA-006 apuntaba a 24-30): cada una es una definición de escuadra nueva en
  Conquest (lotes CQ).
- P1: velocidad de cada tropa nueva.
- `bronce_laminado` (G4): logro del servidor + hito de la Facción.
- D35: qué significa exactamente "ya reclutaba con una tecnología" (¿la plaza tenía el edificio y nivel de alguna
  tropa de esa tecnología y su Facción la tenía adoptada?).
- D36: rareza exacta y terreno de los nodos de hierro; cuánto peor es el arma de hierro temprano que la de bronce.
- D39: ¿la Facción vendedora acepta la venta y fija el precio?
- D3: la Facción-legado (tema aparte).
- D7: el resto del árbol de asedio y la torre de asedio (canon: Carpintería nivel 2).
- D8: cifra de velocidad de la caballería (placeholder); si los carros van también a esa velocidad.
- D30: logro del servidor + hito de Facción de cada tecnología del catálogo (en construcción, arriba); efectos
  propios de la rama institucional (D40).
- D30: requisitos que dependen de mecánicas sin construir (fusión de caminos, §3) retrasan su tecnología hasta que
  existan.

**Sin tratar todavía:**
- Aedas residentes exigen Nobleza (Palacio, nivel 4): al principio solo sirven los itinerantes.

**D47 — `defensaBaseCaravana`: manda el documento, se cambia el código** (2026-09-25, no es de BA-006). El
código tiene `MILITAR.defensaBaseCaravana: 15` fijo; Doc 3.10 dice `poderHeroe × 1.7` = 26 (13 milicianos). El
cambio se aplica cuando esta revisión se cierre y toque bajar al código, no ahora.
