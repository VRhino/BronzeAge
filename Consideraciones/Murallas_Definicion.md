# Murallas — definición y diseño

> **Estado (2026-08-31): diseño cerrado, CERO código escrito.** Este documento es la especificación completa
> de la mecánica de murallas, escrita ANTES de tocar el motor a petición del usuario ("antes de crear la
> primera línea de código primero debemos definirla y diseñarla al completo para evitar dejar blind spots").
> El enunciado original es `Docs/Mecanicas a desarrollar.md` §6. La mecánica se apoya geométricamente en el
> modelo de CELDAS de la Etapa 6 (`Consideraciones/Vista_Asentamiento_Trazado_Urbano.md` §E6.1–E6.23), que es
> su prerrequisito y ya está implementada hasta el Paso 2.
>
> **Revisado por consejo (2026-08-31)** — cinco revisores independientes con revisión cruzada. Sus cuatro
> hallazgos nuevos están en §19.1 y lo que pidieron reabrir, en §20.1. Dos de sus críticas quedaron
> **invertidas** por la razón de ser que el usuario fijó después (§0): pocas puertas es la ventaja del
> defensor, no un estrangulamiento. Ese es el orden en que hay que leer el documento: **§0 primero**.

## 0. La razón de ser (aclaración del usuario, 2026-08-31)

Esta sección va primero porque **reordena la lectura de todo lo demás**, y porque la revisión por consejo
(§19.1) la echó de menos: sin ella, la muralla parece un peaje caro sin contrapartida.

> La muralla es una **ventaja defensiva abrumadora en 3D**. Tener menos entradas **no perjudica al defensor:
> lo beneficia**, porque solo tiene que defender una. Poder apostar soldados encima del muro es otra ventaja
> más. Por eso debe ser **costosa y difícil de obtener y de mantener**.

Tres consecuencias que cambian el diseño, no solo su justificación:

1. **Pocas puertas es el PREMIO, no el castigo.** El documento venía tratando "una sola puerta" como un
   estrangulamiento a evitar (riesgos 6-8). Es justo al revés: un recinto con una puerta es un embudo que
   concentra toda la defensa en un punto. Lo que cuesta pocas puertas es **logística** —caminos más largos,
   comida que rodea— y eso es el precio de la fortaleza, no una trampa. Ver §5.1, reescrito.
2. **El eje estratégico real es CUÁNDO amurallas**, y tiene dos extremos legítimos:
   - **Pronto**: ciudad compacta, perímetro corto (barato), pocas puertas ⇒ **fortaleza**. A cambio, casi todo
     el crecimiento futuro queda extramuros y sin proteger.
   - **Tarde**: ciudad grande ya encerrada, perímetro largo (caro), muchas puertas ⇒ **metrópoli**. Protege
     mucho más, pero hay mucho más frente que defender y la guarnición se reparte.

   No hay una jugada dominante: es un intercambio entre superficie protegida y densidad defensiva. Eso
   **anula el "incentivo perverso"** que el consejo apuntó (§19.1 punto 8), que asumía que menos puertas era
   peor.
3. **El coste no es solo de construcción: es de MANTENIMIENTO.** "Difícil de obtener **y de mantener**" es una
   condición de diseño, no un detalle aplazable. Un muro sin upkeep sería una ventaja permanente pagada una
   sola vez — exactamente lo que una ventaja abrumadora no puede ser. Ver §16, reescrito.

**El contrapeso que ya existe sin código nuevo**, y que hay que medir antes de añadir ninguno más: la
fortaleza temprana solo protege lo que cabe dentro. Todo lo que la ciudad necesite para crecer después
—Viviendas, Almacenes, distritos enteros— nace en el arrabal, fuera del muro y desprotegido (§9). El recinto
barato es barato porque encierra poco.

## 1. El enunciado, traducido a este motor

El enunciado del usuario dice cinco cosas:

| Enunciado | Traducción a este motor |
|---|---|
| "que rodee todo el espacio interno de la ciudad ocupando celdas" | Un anillo cerrado de celdas de la rejilla del asentamiento, que entra en `sueloOcupado` |
| "cada celda de muralla tiene un coste, es decir que escala mientras más celdas tenga" | El coste NO es del edificio: es `nº de celdas × tarifa por celda`. El perímetro es la unidad económica |
| "deja granjas y corrales fuera" | Comprobación explícita en el trazo: la región encerrada no puede contener ninguna celda de Granja o Corral — si la dilatación llega a una, el muro hace un entrante y la deja fuera (§11.2). *Una primera versión lo resolvía con un tope de radio; el Paso 0 lo desmintió con datos (§11.1)* |
| "las murallas tienen mejora, el coste es de madera y piedra" | Tres niveles con tarifa por celda distinta (§7-§8) |
| "al principio no se pueden apostar soldados arriba, con niveles más altos sí; al subir de nivel gana torres, puertas" | Nivel 3 habilita guarnición. Torres sí son recompensa de nivel; **puertas NO** — son geométricas por obligación (§5) y quedan congeladas al trazar (§5.1). Ganar puertas nuevas solo se consigue **ampliando el recinto** (§10) |

## 2. Decisiones

### 2.1 Cerradas con el usuario (2026-08-31)

| # | Decisión | Elegido |
|---|---|---|
| 1 | Crecimiento tras levantar la muralla | **Recinto fijo + arrabal extramuros + ampliación manual**. El trazo se congela al comprometerse; cuando la ciudad se llena, lo nuevo nace fuera |
| 2 | Representación en el motor | **Entidad `Recinto` persistida + obra por celdas**. El `EdificioTipo 'muralla'` desaparece |
| 3 | Alcance de la primera pasada | **Geometría + economía + laboratorio + las dos piezas que hacen real la razón de ser**: el multiplicador defensivo de `iniciarAsedio` y el upkeep por celda (revisada 2026-08-31 tras §0 — antes las dos quedaban fuera). Siguen fuera: guarnición sobre el adarve, brecha, daño estructural y armas de asedio |
| 3b | Qué es el nivel 1 | **Empalizada de madera**. La progresión de materiales madera → piedra es lo que da sentido al enunciado, y el escalón barato es lo que hace jugable el extremo "fortaleza temprana" del eje de §0 |
| 4 | Qué pasa con el muro viejo al ampliar | **Se queda como muro interior**, con sus puertas abiertas. Estratos, no reformas |

### 2.2 Derivadas — no se preguntan porque los invariantes vigentes ya las obligan

| # | Decisión | Por qué no hay alternativa |
|---|---|---|
| 5 | **Toda celda de muro que caiga sobre una calle o un camino existente es una PUERTA** | El invariante de la Etapa 6 (*"la red es un único componente conexo alcanzable a pie desde el Centro Urbano"*) muere en el instante en que un anillo de suelo ocupado corta un corredor. Si las puertas fueran una recompensa de nivel, el nivel 1 partiría la red (§5) |
| 6 | **El recinto es persistido, no derivado** | La red de calles se recalcula entera en cada consulta (`redDeCalles`, replay en orden de construcción). El recinto se PAGA: no puede recalcularse ni depender del orden, o el jugador vería su muralla moverse sola |
| 7 | **Las celdas PLANIFICADAS ocupan suelo desde el commit**, no desde que se construyen | Si no, un edificio se planta encima del trazo a medio levantar y el anillo deja de poder cerrarse. Es el mismo razonamiento que hizo que las celdas de calle entraran en `ocupadas` (§E6.2) |
| 8 | **Una torre no tiene huella propia**: es una celda de muro marcada | Cualquier huella extra (2x2 hacia dentro) chocaría con lo ya construido, y hacia fuera no hay garantía de suelo libre |
| 9 | **El trazo que no se puede cerrar se rechaza entero**, no se cierra a medias | Es el gate duro de §E6.5 aplicado a esta capa: un recinto que falla después de cobrarse sería el fallo en silencio que costó la Etapa 5 |
| 10 | **Las puertas se congelan con el trazo: un recinto ya levantado no gana puertas nuevas nunca** | Aclaración del usuario (2026-08-31), y es lo que la geometría ya impone: las celdas de muro están ocupadas, así que `redDeCalles` no puede sembrar calle sobre ellas ni un corredor puede atravesarlas. Ningún camino posterior cruza un muro existente — todos desembocan en una puerta ya abierta (§5.1) |

## 3. La entidad

Campo nuevo y persistido en `Asentamiento`:

```ts
/** Recintos amurallados, del más interior al más exterior (orden de construcción). Un asentamiento sin
 *  murallas no tiene el campo. Ver Consideraciones/Murallas_Definicion.md */
recintos?: Recinto[];

interface Recinto {
  id: string;
  /** 1 = empalizada · 2 = muro de piedra · 3 = muralla con adarve (§7). */
  nivel: number;
  /** El anillo completo, en orden de recorrido desde la puerta principal. Congelado al comprometerse. */
  celdas: CeldaMuro[];
  /** Índice de la última celda construida del recorrido (§8). -1 = nada construido todavía. */
  avance: number;
  /** Nivel al que se está mejorando ahora mismo, si hay mejora en curso; el avance se reutiliza. */
  mejorandoA?: number;
  comprometidoEn: Instante;
  completadoEn?: Instante;
}

interface CeldaMuro {
  col: number;
  row: number;
  clase: 'muro' | 'puerta' | 'torre';
}
```

Notas de contrato:

- `celdas` es la fuente de verdad geométrica: se calcula UNA vez, al comprometer, y no se vuelve a tocar nunca
  (decisión 6). Ni una mejora de nivel ni el crecimiento de la ciudad la modifican.
- `integridad = (avance + 1) / celdas.length` — derivada, nunca persistida. Es lo que escalará el efecto
  defensivo (§16) y lo que el laboratorio pinta como anillo a medio cerrar.
- `clase` se decide al trazar y no cambia con el nivel: una celda que nació puerta muere puerta. El nivel
  cambia lo que esa puerta VALE, no que exista (decisión 5).
- Es estado pagado ⇒ **sube la versión de snapshot**. Política ya establecida del proyecto: las partidas
  guardadas se borran, no se migran (§E6.14, §8 del doc de trazado).

## 4. El trazo

Función pura y determinista: `trazarRecinto(asentamiento) → Recinto | null`. Entrada: los edificios y la red
del asentamiento tal como están AHORA. Sin aleatoriedad: la misma ciudad da siempre el mismo anillo.

1. **`permitido`** = las celdas del disco dibujable del asentamiento (barandilla ligada a `radioPotencial`,
   §11.2 — no un tope de balance), menos las celdas de todo edificio de afueras (`esDeAfueras`) **y menos su
   vecindad inmediata**, para que el muro no pueda pegarse a una granja ni encerrarla al dilatar.
2. **`nucleo`** = las celdas de los edificios urbanos (`ambito: 'asentamiento'`, `!esDeAfueras`) y de
   `red.calles`, intersecadas con `permitido`, quedándose solo con la **componente conexa que contiene al
   Centro Urbano**. Lo que quede fuera de esa componente es ciudad que el muro no va a proteger, y eso es
   correcto: nace extramuros (§9).
3. **`region`** = `dilatar(nucleo, MURALLA.franjaDeRonda)`, con los agujeros interiores rellenados (un patio
   encerrado no genera un anillo interior) y recortada otra vez por `permitido`. `franjaDeRonda` es el
   *pomerium*: la banda libre entre el último edificio y el muro, por la que se circula y se defiende.
4. **`muro`** = `dilatar(region, 1) \ region`, o sea la capa inmediatamente exterior. Por construcción es un
   anillo cerrado de 1 celda de grosor, 8-conexo, y **disjunto de `region`** — así que no puede pisar nada de
   lo que encierra.
5. **Resolución de conflictos** (lo que el paso 4 no garantiza): si alguna celda de `muro` cae sobre un
   edificio que NO está en `region` —un edificio urbano recortado por `permitido`, o uno que la dilatación no
   alcanzó—, se retira esa vecindad de `region` (`region ← region \ dilatar(celdasDelEdificio, 1)`) y se
   repite desde 4. Cada iteración solo QUITA celdas, así que termina; el tope de iteraciones es duro.
6. **Validación**. El trazo es válido solo si, al terminar: `region` sigue siendo conexa, sigue conteniendo al
   Centro Urbano, y `muro` no pisa ninguna celda de edificio. Si no, `trazarRecinto` devuelve `null` y la
   operación se rechaza con un mensaje explícito (decisión 9). Se reintenta cuando la ciudad cambie.
7. **Clasificación**: cada celda de `muro` que esté en `red.calles` o `red.caminos` se marca `'puerta'`; las
   esquinas convexas y una cada `MURALLA.pasoTorres` celdas de tramo recto se marcan `'torre'` (§6); el resto,
   `'muro'`. Si el anillo no tocara ninguna celda de red —posible solo en una ciudad recién fundada—, se
   fuerza una puerta en la celda del anillo más cercana a la dirección de la ranura 0 del asentamiento, para
   que la regla *"siempre hay al menos una puerta"* no dependa de la suerte.
8. **Orden de recorrido**: el anillo se serializa empezando en la puerta de menor `(row, col)` y siguiendo el
   contorno en sentido horario. Ese orden es el de la obra (§8) y el que hace que el laboratorio enseñe el
   muro cerrándose desde la puerta principal, no apareciendo a manchas.

**Coste computacional**: dos dilataciones y un flood fill sobre la caja del disco urbano. Se calcula una vez
por commit (y una vez por presupuesto), **nunca por tick ni por candidato** — a diferencia de `redDeCalles`.
No entra en el camino caliente que la Etapa 6 tuvo que optimizar dos veces.

## 5. Puertas — por qué son geométricas y no una recompensa de nivel

El enunciado dice que al subir de nivel la muralla "gana torres, puertas". Torres sí. Puertas **no puede ser**,
y conviene dejar escrito por qué, porque es la trampa más cara de esta mecánica:

- Invariante vigente de la Etapa 6: *la red es un único componente conexo alcanzable a pie desde el Centro
  Urbano*, y *todo edificio tiene una celda de calle adyacente*.
- Toda Granja y todo Corral vive a ≥ `radioAfuerasMin` y se conecta por un **camino** que necesariamente cruza
  el anillo. Un muro sin puerta en ese cruce deja los campos —y la comida— desconectados de la ciudad.
- Con "N puertas por nivel", un nivel 1 con 1 puerta en una ciudad con 5 caminos rompe el invariante cuatro
  veces, y el síntoma sería tardío y visual (granjas huérfanas), no un error de tipos. Exactamente el patrón
  de fallo que este proyecto ya pagó dos veces.

**Regla:** toda celda del anillo que pise una calle o un camino existente es una puerta, y una puerta **cuenta
como celda de red**: es transitable y `redDeCalles` la trata como calle. Con eso, la red no pierde ni una
celda al levantar el muro y **la conectividad se conserva por construcción**, sin ninguna comprobación —el
mismo argumento inductivo de §E6.4, aplicado a esta capa.

Lo que el nivel sí cambia: una puerta de nivel 1 es un portón de madera, la de nivel 2 una puerta reforzada y
la de nivel 3 una casa-puerta con torres flanqueantes. Es su VALOR defensivo y su tarifa (§8), no su
existencia.

### 5.1 Las puertas se congelan con el trazo (aclaración del usuario, 2026-08-31)

> Una vez creada la muralla con sus puertas, al estar ocupadas las celdas **ya no se crean puertas nuevas en
> esa muralla**; por tanto no habrá caminos nuevos que atraviesen un muro ya construido.

No es una regla añadida: es lo que la geometría ya impone, y conviene tenerlo escrito porque cambia cómo se
lee toda la mecánica.

- `redDeCalles` comprueba `ocupadas` antes de sembrar cada celda de calle (anillo, retículo o corredor). Las
  celdas de muro y torre están ocupadas ⇒ **ninguna calle ni camino posterior puede nacer sobre el muro**.
- `corredorHastaLaRed` es un BFS sobre celdas libres ⇒ **no puede atravesarlo**, solo rodearlo.
- Las celdas de puerta sí son red, así que el BFS las encuentra y termina ahí. Todo lo que venga después
  **desemboca en una puerta ya abierta**, nunca abre una nueva.

Tres consecuencias que hay que asumir a propósito:

1. **El número de puertas de un recinto queda fijado para siempre en el instante en que se traza**, y sale de
   cuántos caminos y calles cruzaban el anillo ese día. *Cuándo* levantas la muralla decide cuántas puertas
   tendrás el resto de la partida — es la consecuencia estratégica real de la decisión 1 (recinto fijo), no un
   efecto secundario.
2. **Una Granja nueva al otro lado de la ciudad rodea el muro hasta la puerta más cercana.** Su camino se
   alarga, y eso es correcto: es exactamente cómo el tráfico rural converge en las puertas. El número que lo
   permite es `capCorredorAfueras` = 200 celdas, contra un rodeo máximo del orden de 160 (perímetro de un
   recinto en el tope de 19 celdas de radio). Cabe — pero es un margen calculado, no medido: entra en el
   Paso 2 (§18).
3. **Una muralla muy temprana se queda con muy pocas puertas — y eso es una VENTAJA, no un accidente** (§0).
   Menos puertas = menos frente que defender. El precio es logístico: los caminos rurales rodean, la comida
   tarda más, y el arrabal futuro se apiña alrededor de esos pocos accesos.

   **`MURALLA.puertasMinimas` queda RETIRADA.** Se propuso como salvaguarda contra el estrangulamiento, pero
   bajo la razón de ser correcta haría lo contrario de lo que pretende: **abriría a la fuerza brechas que el
   defensor no quiere**, convirtiendo una salvaguarda en una debilidad impuesta. Era además la única regla no
   emergente de toda la mecánica (abría una puerta donde no cruzaba ningún camino), así que retirarla también
   devuelve el diseño al principio rector. El único mínimo que se mantiene es el de §4 punto 7: **al menos una
   puerta**, porque cero puertas sí rompe el invariante de conectividad.

   Lo que sustituye a la salvaguarda es **información**: el presupuesto (§14) enseña el nº de puertas de forma
   prominente antes de pagar, porque es el dato irreversible del trazo. El jugador que amuralla con una puerta
   tiene que saber que está eligiendo una fortaleza, no descubrirlo quince turnos después.

**Corolario del arrabal**: un edificio nuevo fuera del muro alcanza la red rodeando las celdas de muro y
desembocando en la puerta más cercana. El arrabal crece por tanto pegado a las puertas sin ninguna regla nueva
— que es literalmente cómo crecieron los arrabales reales.

## 6. Torres

- Una torre es una celda de muro con `clase: 'torre'` (decisión 8): no tiene huella propia, no puede chocar
  con nada, y sobrevive a cualquier cambio de tamaños del catálogo.
- **Aparecen a partir del nivel 2.** Al mejorar, se reclasifican las celdas ya trazadas: no se retraza nada.
- Colocación determinista: todas las esquinas convexas del anillo, más una celda cada `MURALLA.pasoTorres` en
  los tramos rectos más largos que ese paso. El nivel 3 baja el paso (torres más densas).
- Se dibujan más oscuras y sobresaliendo del trazo (§14). Son la lectura visual inmediata del nivel del muro.

## 7. Niveles

| Nivel | Nombre | Qué añade | Bono base (§16) |
|---|---|---|---|
| 1 | **Empalizada** | El anillo cierra. Madera, sin torres, sin guarnición. Es el escalón barato que hace jugable el extremo "fortaleza temprana" de §0 | 1.3 |
| 2 | **Muro de piedra** | Torres en esquinas y tramos largos. Puertas reforzadas. Coste dominado por piedra | 1.8 |
| 3 | **Muralla con adarve** | Habilita **guarnición sobre el muro** (§16). Torres más densas. Casas-puerta | 2.5 |

El bono base es el techo: el multiplicador real se divide entre el nº de puertas y se escala por la
integridad del anillo (§16). Una empalizada cerrada con una sola puerta (**×1.30**) defiende más que un muro
de piedra terminado con seis puertas (**×1.13**), y muchísimo más que ese mismo muro de piedra a medio
construir (**×1.07**) — el nivel importa, pero la forma del recinto importa igual o más.

- La mejora es del RECINTO entero, no por celda: se paga `nº de celdas × tarifa de mejora` y avanza con la
  misma obra progresiva que la construcción (§8), recorriendo el mismo anillo desde la puerta principal.
- **No se puede mejorar un recinto incompleto**: primero se cierra el anillo, después se sube de nivel. Un
  muro medio empalizada y medio piedra no significa nada y multiplicaría los estados a probar.
- Cada recinto tiene su propio nivel. Al ampliar (§10), el recinto nuevo **no hereda** el nivel del viejo:
  nace al nivel que se pague.

## 8. Economía: coste por celda y obra progresiva

**Trazar es gratis y no compromete nada**: `trazarRecinto` devuelve el anillo y su presupuesto para que el
jugador (y el laboratorio) lo vean antes de decidir. Es una consulta pura.

**Comprometer** congela el trazo, mete todas sus celdas en el suelo ocupado (decisión 7) y arranca la obra.
No cobra nada por adelantado.

**La obra avanza por celdas, pagando sobre la marcha.** Cada tick, dentro de `avanzarConstruccion`:

- se intenta levantar hasta `MURALLA.celdasPorMinuto` celdas, en el orden de recorrido del anillo;
- cada celda cobra su tarifa según su `clase` y el nivel del recinto (`muro` ×1, `puerta` ×`factorPuerta`,
  `torre` ×`factorTorre`);
- si no hay materiales —respetando `reservaDinamicaConstruccion`, igual que cualquier otra obra automática—,
  **la obra simplemente no avanza ese tick**. No falla, no se cancela, no acumula deuda.

Por qué pago sobre la marcha y no por adelantado: es lo que el enunciado pide literalmente ("cada celda tiene
un coste"), evita que una muralla vacíe el almacén de golpe (los costes totales están en el orden del Palacio o
la Maravilla), y hace que el estado "anillo a medio cerrar" sea un estado normal y visible en vez de un
artefacto.

**La obra NO ocupa un hueco de `maximoEnConstruccionSimultanea` ni entra en la cola de edificios.** Es una obra
pública del asentamiento, no un proyecto de la cola: meter 80 entradas en una cola con tope 8 haría ilegible el
panel y el orden de prioridad. Sí respeta la misma reserva de mantenimiento, que es lo que impide que la
muralla mate al asentamiento de hambre.

**Abandonar.** El Gobernador puede abandonar un recinto incompleto: se borra entero (celdas construidas
incluidas) y se libera todo su suelo, **sin devolución de materiales**. Existe como escotilla de seguridad: sin
ella, una obra atascada por falta de piedra bloquearía su franja de suelo para siempre y podría dejar sin sitio
a una ciudad. Un recinto **completo no se puede abandonar** (no hay demolición en Fase 0, ver §"Abierto" del
doc de trazado).

**Quién la manda construir.** Adición manual del **Gobernador o el Maestro de Obras**, mismo camino que
Barracón/Galería/Mercado (`anadirEdificioManualmente` tiene el precedente y las mismas comprobaciones de
cargo). **No hay auto-construcción de murallas.** La gobernanza NPC la añade a mano cuando el asentamiento
cumple el gate, igual que ya hace con el Mercado — si no, ninguna facción del batch llegaría nunca a nivel 4
(es el mismo agujero que documenta `issues/nivel_3_inalcanzable_sin_jugador_humano.md`, y hay que cerrarlo en
el mismo paso o el gate de nivel 4 queda muerto).

**Gate de construcción**: nivel de asentamiento 3, el mismo que tenía el edificio `muralla` que sustituye.

## 9. El arrabal (extramuros)

Consecuencia directa de la decisión 1: el recinto es fijo, la ciudad no.

- **Preferencia intramuros, no reserva.** Mientras el asentamiento tenga un recinto **completo**, la búsqueda
  de sitio ofrece primero todos los candidatos que caen dentro del recinto más exterior; solo si no hay
  ninguno, ofrece los de fuera. Es un filtro con fallback, no un término más del desempate: **añadir un
  criterio al orden lexicográfico rompería los cuatro perfiles de trazado de §E6.23**, que son permutaciones
  exactas de ese orden. Dentro de cada grupo, el desempate del perfil sigue mandando sin tocarse.
- **El arrabal nace en las puertas.** No hace falta ninguna regla: el gate de conexión (`capCorredorUrbano`,
  12 celdas) solo lo pasan los sitios que alcanzan una celda de red, y fuera del muro la red solo existe a
  partir de las puertas. Hay que **medir** si 12 celdas bastan; si el arrabal se ahoga, el número a mover es
  un `capCorredorArrabal` propio, no el urbano (mismo error que §E6.17 punto 4 y que §E6.10 avisó).
- Los edificios extramuros son ciudad normal en todo lo demás: producen, cuentan para los gates de nivel y
  pagan mantenimiento. Lo único que no tienen es protección — que es precisamente lo que la ampliación (§10)
  y el efecto defensivo (§16) convierten en una decisión.

## 10. Ampliación de recinto

- **Gate**: solo con el recinto exterior actual completo (integridad 1) y al menos `MURALLA.arrabalMinimo`
  edificios extramuros. Sin eso, ampliar sería spam y el muro dejaría de significar una decisión.
- El trazo nuevo se calcula con el mismo `trazarRecinto` (§4), que ahora ve más ciudad, y **debe contener por
  completo al recinto anterior**: si no lo contiene, se rechaza. Invariante: el área encerrada nunca decrece.
- **El recinto viejo se queda** (decisión 4), con sus celdas y sus puertas. Sus puertas siguen siendo red, así
  que la conectividad interior se conserva. La ciudad acaba registrando su historia en la piedra, igual que
  los estratos políticos de §E6.23.
- **Ampliar es la ÚNICA forma de ganar puertas**, porque el recinto viejo no gana ninguna nunca (§5.1). Y como
  el trazo nuevo ve la red que ya sale por las puertas viejas, sus puertas tienden a caer en la prolongación
  de esos mismos caminos: se forman ejes radiales que atraviesan los dos anillos. Emergente, sin ninguna regla
  que lo pida.
- Riesgo asumido y a medir: un muro interior es suelo permanentemente perdido dentro de la ciudad. Si en el
  laboratorio se ve que asfixia el casco antiguo, la palanca es `franjaDeRonda` (más ancha, más aire) antes
  que reabrir la decisión.

## 11. Afueras: por qué "deja granjas y corrales fuera" sale gratis

> **REESCRITA CON DATOS (Paso 0, 2026-08-31).** La versión anterior de esta sección proponía un tope fijo
> `radioMaximoMuro = radioAfuerasMin / tamanoCelda − 1` = **19 celdas**, y celebraba que la garantía saliera
> "gratis" del mismo número que gobierna las afueras. **La medición lo desmiente:** con 19 celdas, el 51 % de
> los edificios urbanos de una ciudad de nivel 3 —incluidas CINCO anclas, o sea distritos enteros— quedaría
> extramuros y sin posibilidad de encerrarse jamás, porque la ampliación también estaría capada. La elegancia
> era falsa y el consejo tenía razón al marcar el "probablemente no muerde" como un prejuicio (§19.1).

### 11.1 Lo que midió el Paso 0

5 seeds × 400 ticks, montaje del laboratorio (Facción a nivel máximo, Gobernador, materiales rellenados cada
tick para no medir una ciudad limitada por economía sino la más grande que el trazado puede producir). Radios
en CELDAS desde el origen del asentamiento.

| Nivel | `radioPotencial` | Edificios urbanos | Radio máx. del tejido | p90 | p99 | Ancla más lejana |
|---|---|---|---|---|---|---|
| 1 | 20 celdas | 22–70 | **11.0 – 32.0** | 8–28 | 10–31 | 3.5 – 26.3 |
| 2 | 30 celdas | 63–70 | **22.2 – 28.9** | 18–24 | 21–27 | 19.0 – 22.2 |
| 3 | 40 celdas | 124–129 | **33.5 – 44.3** | 26–34 | 32–41 | 30.7 – 38.3 |

**Tres hechos que el documento no sabía:**

1. **El tejido urbano no tiene ningún tope anclado al origen.** Crece con el nivel y **supera su propio
   `radioPotencial`** (nivel 1: fabric a 32 celdas contra un radio de 20). La causa es estructural, no un bug:
   `candidatosLibres` acota la distancia al **ancla de referencia**, no al centro del asentamiento, y las
   anclas encadenan (§5.10). Cualquier tope fijo para el muro habría estado mal en algún nivel.
2. **`radioAfuerasMin` no es donde están las granjas, es su PISO.** Como las afueras prefieren siempre el hueco
   más lejano (§11 del doc de trazado), acaban pegadas al borde exterior de su banda: medido, la granja más
   cercana al centro está a **32.7–32.9 celdas**, no a 20. El tope de 19 celdas no protegía de nada — ni
   siquiera estaba cerca de las granjas.
3. **Ciudad y afueras solo se entremezclan a partir del nivel 3, y poco.** Celdas urbanas más lejos que la
   granja más cercana: **0 % en niveles 1 y 2**; en nivel 3, entre **0.3 % y 12 %** según la seed. O sea que
   la separación ciudad/campo aguanta bien hasta nivel 3 y ahí empieza a deshilacharse por la cola.

Con un anillo a 19 celdas, los tipos que quedaban fuera en nivel 3 (suma de 5 seeds): 218 Viviendas, 22
Fundiciones, 21 Talleres, 19 Armerías, 10 Curtidurías **y 5 anclas** (Carpintería, Patio de Gremios, Parque,
Plaza, Plaza de Armas) — la industria y el núcleo militar enteros, extramuros para siempre.

### 11.2 La regla que sustituye al tope fijo

**El recinto deja de tener un radio máximo.** La garantía del enunciado pasa a comprobarse directamente, en el
mismo bucle de resolución de conflictos que ya tiene el trazo (§4, paso 5):

> **`region` no puede contener ninguna celda de un edificio de afueras.** Si la dilatación llega a encerrar
> una Granja o un Corral, se retira esa vecindad de `region` y se recalcula — el muro hace un entrante y la
> deja fuera.

Por qué es mejor que el tope de radio, y no solo distinto:

- **Cumple el enunciado literalmente** ("deja granjas y corrales fuera") en vez de por un proxy que resultó no
  serlo.
- **Escala sola**: vale igual para una ciudad de nivel 1 de 20 celdas que para una de nivel 5, sin recalibrar
  ningún número. Un tope fijo habría que moverlo en cada nivel nuevo.
- **No acopla dos mecánicas por una constante.** Era el riesgo 4 del consejo: mover `radioAfuerasMin` por
  rebalancear granjas habría cambiado en silencio el tamaño máximo de todas las ciudades amuralladas.
- **Casi nunca muerde**: con las granjas a ~33 celdas y el tejido acabando antes en niveles 1-2, el entrante
  solo aparece en la cola del nivel 3+ (0.3-12 % de las celdas).

Se conserva **un tope de cordura**, pero ligado a `radioPotencial` (que ya escala con el nivel y ya acota la
colocación), no a las afueras: el muro nunca puede salir del disco dibujable del asentamiento. No es una regla
de balance, es una barandilla contra un trazo degenerado.

**Consecuencia económica que hay que asumir:** el perímetro real es mucho mayor de lo que estimaba §17. Un
recinto de nivel 3 rodea un tejido de ~30-44 celdas de radio, no las ~19 supuestas, así que las tarifas por
celda están calibradas contra un perímetro entre 2 y 3 veces demasiado pequeño. **No se pueden fijar hasta
medir el perímetro del trazo REAL** (el contorno de una ciudad-estrella no es el de un círculo) — que es
exactamente lo que produce el Paso 1.

## 12. Interacción con el trazado ya implementado

| Pieza vigente | Qué cambia |
|---|---|
| `sueloOcupado()` | Suma las celdas de muro y torre de todos los recintos (las de **puerta no**: son transitables). Un solo punto, que es exactamente por lo que se centralizó en el Paso 2 |
| `redDeCalles()` | Siembra las celdas de puerta como calle antes del replay. Nada más: la red no pierde celdas (§5) |
| `candidatosLibres()` / campo de distancia | Sin cambios de código: las celdas de muro llegan ya dentro de `ocupadas`, así que el BFS las rodea y el gate duro de conexión funciona igual |
| Orden de commit (§E6.16, ABIERTO) | El recinto se compromete en su **propio paso del tick**, después de `evaluarNecesidades`, contra el estado ya commiteado. No se ve afectado por el desajuste de orden que sigue abierto para los edificios — pero **el bug abierto sigue abierto**, y conviene cerrarlo antes o a la vez: un edificio plantado sobre una calle también puede plantarse sobre un trazo recién comprometido |
| `esDeAfueras` / `radioMaximoAfueras` | Sin cambios (§11) |
| Perfiles de trazado (§E6.23) | Sin cambios: la preferencia intramuros es un filtro previo, no un término del orden (§9) |
| `reubicarPorTamano` (mejora de Granja) | Sin cambios: las Granjas están siempre fuera del muro y se mudan hacia afuera |

## 13. Qué sustituye al `muralla` actual

> **No hay migración, y esto es lo primero que hay que leer de esta sección.** Las partidas guardadas con el
> edificio `muralla` viejo —incluidos los asentamientos que ya subieron a nivel 4 con él— **se borran**. Es la
> política ya establecida del proyecto (§E6.14 y §8 del doc de trazado: "una partida de versión de guardado
> anterior se rechaza al importar", sin retrocompatibilidad, a petición del usuario). Se dice aquí arriba
> porque en la revisión por consejo **cinco lectores independientes preguntaron por la migración** aunque
> estuviera escrita al final de la lista: si cinco no la ven, está mal colocada, no mal decidida.

El `EdificioTipo 'muralla'` de hoy (1 celda, 2000 piedra, sin efecto mecánico, "implementación mínima a
propósito" según `Fase_0_6_Definicion...md`) **desaparece**. Lista de puntos a tocar, para que ninguno se
quede colgado:

- `domain/types.ts`: fuera de `EdificioTipo` y de `TODOS_LOS_EDIFICIOS`. Entra `Asentamiento.recintos`.
- `constants.ts`: fuera de `EDIFICIO_CATALOGO`. Entra `MURALLA` (§17).
- `NIVEL_ASENTAMIENTO.requisitos[4]`: `edificios: ['muralla']` pasa a un requisito nuevo de **recinto completo
  de nivel ≥ 1**. `calcularNivelAsentamiento` hoy solo sabe contar tipos de edificio: hay que extender el
  requisito, no falsear un edificio fantasma. Mismo cambio en `progresoNivelAsentamiento` (el "qué me falta"
  que el laboratorio y el cliente ya muestran).
- `scripts/run-batch-sim.ts`: `murallasActivas` → `recintosCompletos`, `celdasMuroMedia`, `arrabalPct`.
- `cliente/src/main.ts`, `cliente/src/ui/canvas.ts`, `lab/src/render.ts`, `scripts/catalogo-edificios.ts`: la
  etiqueta y el color `#5a5a5a` se mudan de la tabla de edificios a la capa de muralla.
- Gobernanza NPC: añade la orden de construir muralla al cumplir el gate (§8).
- Partidas guardadas: se borran (decisión 6 / §E6.14).

## 14. Contrato de dibujo y laboratorio

`TrazadoAsentamiento` gana un campo, con el mismo criterio de payload que las calles (celdas fusionadas en
tiradas horizontales, nunca celda a celda):

```ts
murallas: TrazadoMuralla[];

interface TrazadoMuralla {
  recintoId: string;
  nivel: number;
  integridad: number;          // 0..1
  muro: RectanguloLocal[];     // construido
  planificado: RectanguloLocal[];
  puertas: RectanguloLocal[];
  torres: RectanguloLocal[];
}
```

Derivado, como el resto de `GeometriaAsentamientos` (se calcula en `RunnerDePartida`, no se persiste) — así que
esquema + cliente + laboratorio, sin tocar disco. Lo que sí toca disco es `Asentamiento.recintos` (§3).

**El laboratorio es la herramienta de validación de esta mecánica**, igual que lo fue de la Etapa 6. Necesita:

- **Presupuesto en vivo**: un botón "Trazar recinto" que pinte el anillo PROPUESTO en punteado sobre la ciudad
  actual, con nº de celdas, desglose de puertas/torres y coste total, **sin comprometer nada**. Es lo que
  permite ver cómo cambia el trazo tick a tick según crece la ciudad.
- **Comprometer / Avanzar obra / Mejorar nivel / Ampliar / Abandonar** como botones, para recorrer todos los
  estados sin esperar a la economía.
- **Dibujo**: muro construido en gris piedra sólido, planificado en punteado del mismo tono, puertas en ocre,
  torres más oscuras y sobresalientes. El arrabal (edificios fuera del recinto exterior) con un halo, para que
  se lea de un vistazo cuánta ciudad quedó fuera.
- **Parámetros en caliente** (pestaña Parámetros, sección nueva "Murallas"): `franjaDeRonda`, `pasoTorres`,
  tarifas por celda de cada nivel, `celdasPorMinuto`, `arrabalMinimo`. Con el marcado en amarillo de los
  valores tocados que ya existe.
- `window.__lab` expone `recintos` y el trazo propuesto, para medir invariantes a mano sin instrumentar nada
  (es como se aisló §E6.16).

## 15. Invariantes a congelar en tests

1. **El anillo CIERRA**: un flood fill 4-conexo desde fuera que no cruce **ninguna celda del anillo** —puertas
   incluidas— no alcanza nunca el Centro Urbano. *Corregido en el Paso 1: la primera redacción decía que las
   puertas sí se cruzaban, lo que hacía la prueba trivialmente falsa. Que la puerta sea transitable es lo que
   mantiene la red conexa (§5), no una propiedad del cierre del anillo.*
2. **Ninguna celda de muro/torre/puerta pisa un edificio.**
3. **Ninguna celda de Granja o Corral queda dentro de un recinto** (§11).
4. **Tras levantar el recinto, la red sigue siendo UN único componente conexo** y todo edificio conserva una
   celda de calle adyacente — los dos invariantes que la Etapa 6 pagó y que esta mecánica es capaz de romper.
5. **Determinismo**: misma ciudad ⇒ mismo trazo, mismas clases, mismo orden de recorrido.
6. **El cobro es exactamente `Σ tarifa(clase, nivel)`** sobre las celdas construidas: ni una celda gratis, ni
   una cobrada dos veces al mejorar.
7. **Ampliar nunca reduce el área encerrada** y el recinto nuevo contiene al anterior.
8. **Un trazo inválido devuelve `null` y no muta nada** (decisión 9).
9. **Las puertas de un recinto no cambian nunca** (§5.1): avanzar N ticks tras completar el muro deja el
   conjunto de celdas `'puerta'` idéntico, y ninguna celda de calle o camino nueva cae sobre una celda de
   muro o de torre. Es el invariante que congela la aclaración del usuario.
10. **Segundo fixture obligatorio**: la ciudad del laboratorio (seed 60 con Mercado manual, la que destapó
   §E6.19 y §E6.21), además de la seed 99 de `trazado.test.ts`. La suite ya demostró que una sola ciudad de
   fixture no ve los bugs de esta capa.

## 16. Defensa — es la RAZÓN DE SER, no un extra (§0)

Esta sección dejó de ser "lo que quizá se añada después". Es lo que justifica que la mecánica exista, y por
tanto lo que fija su coste: **una ventaja abrumadora tiene que ser cara de obtener y cara de mantener**.

**Dos de estas piezas ENTRAN en la primera pasada** (decisión 3, revisada): el multiplicador defensivo y el
upkeep por celda. Es el mínimo que convierte el muro en una decisión de juego en vez de un peaje, y las dos
son pocas líneas sobre código que ya existe. Se implementan y se miden **en su propio paso** (§18, Paso 3b),
separadas del cambio geométrico, para no perder la atribución. **Siguen fuera**: guarnición sobre el adarve,
brecha, daño estructural y armas de asedio.

- **Multiplicador defensivo en `iniciarAsedio`**, aplicado a los escuadrones defensores, escalado por `nivel` y
  por `integridad` (un anillo a medio cerrar no defiende: se entra por el hueco).
- **La ventaja crece cuantas MENOS puertas tenga el recinto** (§0): el atacante tiene que entrar por una
  puerta, y el defensor concentra ahí toda su fuerza. Fórmula propuesta, que **nunca puede bajar de 1** (un
  muro jamás perjudica):

  ```
  multiplicador = 1 + (bonoDefensaPorNivel[nivel] − 1) × integridad / nºPuertas
  ```

  Con el nivel 3 (`bono` 2.5) y el anillo entero: **×2.5 con una puerta**, ×1.38 con cuatro, ×1.19 con ocho.
  Es lo que convierte "amurallar pronto" en fortaleza y "amurallar tarde" en metrópoli sin ninguna regla
  adicional — y `integridad` hace que un anillo a medio cerrar no defienda casi nada, porque se entra por el
  hueco.
- **Guarnición sobre el adarve a partir del nivel 3**: capacidad = `f(nº de celdas)`, escuadrones asignados al
  muro que ganan un bono adicional. Es el "apostar soldados arriba" del enunciado. Un muro largo admite más
  guarnición pero necesita más para cubrirse — otra vez el mismo intercambio, sin regla nueva.
- **Puertas y torres como puntos duros**: la puerta es el punto débil (lo que un ariete ataca), la torre el
  fuerte. La brecha, el daño estructural y las armas de asedio (ariete, torre de asedio, que la Carpintería ya
  recluta en el diseño) son **Fase 1**, junto con el asedio instanciado.
- **Mantenimiento del muro — OBLIGATORIO, no opcional** (§0: *"difícil de obtener **y de mantener**"*). Un
  upkeep por celda es lo único que impide que una ventaja abrumadora se pague una sola vez y dure para
  siempre. `Docs/Game/5_Sistema_Militar_y_Combate.md` ya dice que los Constructores mantienen murallas.
  Consecuencia de diseño: un recinto grande cuesta cada tick, así que **ampliar por ampliar deja de ser
  gratis** y la metrópoli amurallada es una decisión económica sostenida, no una compra.

  **✅ Implementado (Paso 3b, 2026-09-01) — resuelto a petición del usuario**: *"el upkeep tiene que existir
  pero hay que balancearlo bien... el punto ideal sería que ninguna facción de un solo asentamiento sin
  comercio pueda mantenerse con la muralla si quieres tener muralla (nivel 2 y 3) tienes que ser lo
  suficientemente próspero"*. `upkeepDeRecintos` (`engine/muralla.ts`) se **SUMA dentro de
  `calcularCostoMantenimiento`** (`engine/mantenimiento.ts`) — el MISMO pote que ya paga población y
  edificios, no uno aparte. Esto cierra también el corolario de abajo (antes "pendiente de decidir"): **no
  hay una segunda vía de castigo que degrade solo la integridad del muro** — si el total no se cubre, es
  `avanzarMantenimiento` quien responde, exactamente igual que si el déficit viniera de población o
  edificios (medidor de mantenimiento baja, y si llega a 0, `nivelActual` cae). Un asentamiento no puede
  "proteger" su muro sin poner en juego su propio nivel operativo.

  **Por qué esto BASTA para el objetivo sin números nuevos** (cifras siguen siendo las PLACEHOLDER de §17,
  calibración final en el Paso 6): a población tope de nivel 3 (6000 hab, `poblacionReferencia=500`),
  Mantenimiento YA cobra piedra a `piedraBase(3) × factorPoblación(13) ≈ 39/minuto` — el propio comentario de
  `MANTENIMIENTO.poblacionReferencia` en `constants.ts` ya advertía que esto "se come casi toda la producción
  bruta de una ciudad llena" **sin contar la muralla**. Un recinto de nivel 3 (178-225 celdas medidas, Paso 1)
  a `upkeepPorCelda[3].piedra = 0.04` añade **7-9 piedra/minuto** más — suficiente para tumbar a déficit a
  cualquier asentamiento que no llegue al techo teórico de extracción propia (~50 piedra/minuto con los 10
  canteras del tope, y el propio batch mide que ni todas las facciones tienen piedra alcanzable siquiera). El
  nivel 1 (empalizada, upkeep en madera, recintos más cortos) se queda deliberadamente barato — 2-3
  madera/minuto contra un mantenimiento que ya cobra ~39 de base a población tope —, así que sigue siendo el
  escalón que CUALQUIER asentamiento pobre puede sostener (§0, "fortaleza temprana"). **Sin verificar en
  batch todavía**: el batch de humo (Paso 2c) no consiguió que ningún asentamiento llegara a nivel 3 en 3000
  ticks — el mismo cuello de botella de progresión que ya bloqueó la verificación de `asegurarMuralla` en
  batch bloquea también verificar esto con datos reales. La aritmética de arriba es la justificación de
  diseño; la confirmación empírica queda pendiente de que ese cuello de botella se resuelva, o del Paso 6.

## 17. Constantes nuevas (todas PLACEHOLDER, a calibrar en el laboratorio)

```ts
export const MURALLA = {
  franjaDeRonda: 1,        // celdas libres entre el último edificio y el muro (pomerium)
  pasoTorres: { 2: 8, 3: 5 },
  // `puertasMinimas` RETIRADA (§5.1): pocas puertas es la ventaja del defensor, no un problema a corregir.
  arrabalMinimo: 6,        // edificios extramuros necesarios para poder ampliar
  celdasPorMinuto: 1,      // ritmo de obra
  factorPuerta: 4,         // multiplicador de tarifa sobre la celda de muro
  factorTorre: 3,
  tarifaPorCelda: {
    1: { madera: 20, piedra: 2 },   // empalizada — madera domina (decisión 3b)
    2: { madera: 5,  piedra: 25 },  // mejora a muro de piedra
    3: { madera: 10, piedra: 20 },  // mejora a muralla con adarve
  },
  // Upkeep por celda y tick, por nivel (§16). Es la mitad de "difícil de obtener Y DE MANTENER": sin esto,
  // una ventaja abrumadora se pagaría una sola vez y duraría para siempre.
  upkeepPorCelda: {
    1: { madera: 0.02 },
    2: { piedra: 0.02 },
    3: { piedra: 0.04 },
  },
  // Bono defensivo del recinto en `iniciarAsedio` (§16):
  //   multiplicador = 1 + (bonoDefensaPorNivel[nivel] − 1) × integridad / nºPuertas
  // El divisor por puertas es lo que hace real el eje fortaleza↔metrópoli de §0, y el "1 +" garantiza que un
  // muro nunca perjudique al defensor por muchas puertas que tenga.
  bonoDefensaPorNivel: { 1: 1.3, 2: 1.8, 3: 2.5 },
};
```

**`radioMaximoMuro` ya no existe** (§11.2, Paso 0): el recinto no tiene tope de radio. La garantía "granjas
fuera" es una comprobación explícita en el bucle de conflictos del trazo, y la única barandilla que queda está
ligada a `radioPotencial`.

> ⚠️ **Las tarifas de arriba están calibradas contra un perímetro equivocado.** Salían de suponer un núcleo de
> 17x14 celdas (§E6.1) ⇒ perímetro ≈ 76 celdas ⇒ ≈ 2.100 madera para cerrar una empalizada. El Paso 0 midió el
> tejido real: 22-29 celdas de radio en nivel 2 y **33-44 en nivel 3**, así que el perímetro verdadero está
> entre 2 y 3 veces por encima — y el contorno de una ciudad-estrella es más largo que el de un círculo del
> mismo radio, así que puede ser peor. **No se fijan hasta que el Paso 1 dé el perímetro del trazo real.**
>
> Comparables para cuando toque calibrar: Palacio 1.500/1.000, Maravilla 5.000/5.000, la `muralla` que
> sustituye 2.000 piedra de golpe. La diferencia es que aquí se paga a 1 celda por minuto, no de una vez — y
> que el coste tiene que doler, porque compra una ventaja abrumadora (§0).

## 18. Plan de ejecución

Misma convención que la Etapa 6: **cada paso cambia UNA cosa medible**, y cada paso se cierra anotando aquí lo
que se midió de verdad, no lo que se esperaba medir.

#### Paso 0. La medición que podía mover una constante — ✅ completado 2026-08-31
Movió más que una constante: **eliminó el tope de radio entero.** Resultados y tabla en §11.1; la regla que lo
sustituye, en §11.2.
- [x] Medido con `scripts/medicion-muralla.ts` sobre el motor real (montaje del laboratorio: Facción a
      nivel máximo, Gobernador, materiales rellenados cada tick), 5 seeds × 400 ticks × niveles 1/2/3
      (`NIVEL=3 npx tsx scripts/medicion-muralla.ts`). **No se instrumentó el batch**: la pregunta era
      puntual y su respuesta no necesita vigilancia continua, igual que el A/B efímero de §E6.18. El script se
      queda mientras la mecánica esté en curso —el Paso 1 lo reutiliza para medir el perímetro del trazo
      real— y **se borra al cerrarla**
- [x] **`radioAfuerasMin` NO se mueve.** El blind spot era real (con 19 celdas, el 51 % de una ciudad de nivel
      3 quedaba fuera, incluidas 5 anclas) pero la causa no era esa constante: era suponer que el tejido
      urbano cabe en un radio fijo. No cabe en ninguno — crece con el nivel y supera su propio `radioPotencial`
- [x] Descubierto de paso, y anotado porque afecta al trazado en general, no solo a las murallas: **las
      granjas no están en `radioAfuerasMin` sino a ~33 celdas** (prefieren el hueco más lejano), y **ciudad y
      afueras empiezan a entremezclarse en nivel 3** (0.3-12 % de las celdas urbanas caen más lejos que la
      granja más cercana; 0 % en niveles 1-2)

#### Paso 1. El trazo, sin economía — ✅ completado 2026-08-31
- [x] `src/engine/muralla.ts`: `trazarRecinto` (§4) como función PURA, `costoDeTrazo`, `trazadoDeRecinto` y
      `TrazadoMuralla`. Módulo aparte de `engine/trazado.ts` (ya 1900+ líneas) a propósito: el trazado razona
      sobre cómo CRECE la ciudad, esto sobre dónde TERMINA; solo comparten el vocabulario de celdas
- [x] `MURALLA` en `constants.ts` (§17), con las tarifas marcadas como calibradas contra un perímetro
      equivocado hasta que este paso diera el real
- [x] Laboratorio: botón **"Trazar recinto"** con selector de nivel, anillo punteado sobre la ciudad y
      presupuesto en vivo. **No compromete nada** — es una consulta pura, y el nivel se recalcula al vuelo
- [x] `muralla.test.ts`: **23 tests**, invariantes 1, 2, 3, 5, 6, 8 de §15 sobre DOS ciudades (seed 99, la de
      `trazado.test.ts`; seed 60, la del laboratorio). Suite completa **721 tests**, `tsc` limpio en motor y lab
- [x] **Tooltip del anillo**: al pasar el cursor por una celda de muralla, su ficha —clase, celda, coste de
      ESA celda al nivel elegido, y en qué punto del recorrido de la obra se levantará. El anillo era lo único
      dibujado que no se podía interrogar con el ratón, y el laboratorio existe para inspeccionar
- [x] `window.__lab` expone `murallaTrazo` y `trazarRecinto`, y se refresca también al trazar (antes solo en
      `computar()`, así que la consola habría mentido con un trazo viejo)
- [x] Verificado en el laboratorio: seed 60, tick 200 → anillo cerrado, granjas fuera, caminos saliendo por
      las puertas, tooltip correcto en muro/puerta/torre, 0 errores de consola. **721 tests**

**El perímetro real, que es lo que este paso existía para medir** (5 seeds, ciudades de nivel 1/2/3):

| Ciudad | Celdas de anillo | Puertas | Torres (niv. 2) | Área encerrada | Urbanos dentro/fuera |
|---|---|---|---|---|---|
| Nivel 1 | 59 – 140 | **1 – 4** | 5 – 12 | 236 – 853 | 22-70 / 0-10 |
| Nivel 2 | 94 – 128 | **2 – 6** | 10 – 12 | 536 – 837 | 60-70 / 0-10 |
| Nivel 3 | 178 – 225 | **5 – 10** | 15 – 24 | 1313 – 1843 | 78-122 / 3-48 |

*(Las puertas son las de después del colapso de tiradas, ver el punto 4 de abajo. Antes de esa corrección la
fila de nivel 3 decía 17-29, y eran falsas.)*

**El eje fortaleza↔metrópoli de §0 no era una teoría: está en los datos.** Amurallar en nivel 1 da 59-61
celdas de anillo con **1-2 puertas**; amurallar en nivel 3 da 178-225 celdas con **5-10**. Tres veces el
perímetro y de tres a cinco veces las puertas.

**Y de ahí sale el primer problema de balance real, que solo se ve con los números** (§19.1 punto 12): con la
fórmula de §16 y un recinto de nivel 3, la fortaleza de 1 puerta defiende **×2.50** y la metrópoli de 10
puertas **×1.15** — o sea que la ciudad que paga 3-4 veces más por su muralla obtiene bastante menos. Eso no
es un intercambio, es una inversión: pagar más por menos.

**Tres cosas que solo aparecieron al escribir el código:**

1. **La clasificación de torres corría sobre un `Set`, no sobre el recorrido del anillo**, así que
   `pasoTorres` no significaba nada: salían **27-35 torres en un anillo de 120 celdas**, a pares y a tríos.
   La dilatación con vecindad 4 achaflana las esquinas, y cada giro produce varias esquinas convexas seguidas.
   Corregido separando el orden de la clasificación —puertas (geométricas) → recorrido → torres espaciadas a
   lo largo de él— y con una separación mínima real. Ahora salen **10-12**, una cada ~11 celdas.
2. **El relleno de agujeros puede tragarse una Granja** si la ciudad la ha rodeado por los cuatro costados.
   No se puede impedir sin fabricar un anillo interior alrededor de ella, que sería peor. Es correcto y
   deliberado —una granja envuelta por la ciudad ya no es "las afueras", es un huerto urbano— pero es la única
   grieta de la garantía del enunciado, así que se EXPONE en el resultado (`afuerasDentro`) en vez de
   esconderse: una grieta medible es una grieta vigilada, y hay un test que exige que esté vacía.
4. **Una puerta no era una puerta, era un tramo** (encontrado por el usuario jugando con el laboratorio, no
   por la suite — otra vez). Marcar como puerta TODA celda del anillo que pise la red daba tiradas de 4 y 5
   celdas seguidas: un camino no siempre CRUZA el muro, muchas veces corre pegado a él un tramo antes de
   salir, y todo ese tramo se pintaba como un portón. El daño no era estético: **el nº de puertas es el
   divisor del bono defensivo**, así que un camino que lamía el muro cinco celdas dividía la defensa de la
   ciudad por cinco. Corregido con `colapsarPuertas`, que recorre el anillo circularmente y de cada tirada
   contigua conserva UNA celda: la que de verdad atraviesa —red a un lado dentro y red al otro lado fuera— o
   la del medio si ninguna lo hace. **Medido: en nivel 3 las puertas caen de 17-29 a 5-10**; seed 60 pasa de
   5 a 3. Congelado como test ("no hay dos puertas seguidas en el recorrido").

3. **El invariante 1 de §15 estaba mal redactado.** Decía que las puertas "sí se cruzan" en el flood fill, lo
   que hacía la prueba trivialmente falsa (por una puerta se llega dentro siempre). Lo que hay que probar es
   que el **anillo esté CERRADO**, contando las puertas como parte de él. Corregido en el test y en §15.

#### Paso 2a. La entidad y la ocupación del suelo — ✅ completada 2026-08-31
Separada de la obra por consejo de la revisión (§20.1): 2a cambia GEOMETRÍA (quién ocupa qué suelo), 2b cambia
ECONOMÍA (quién paga). Mezcladas, un movimiento de métrica no tendría un solo culpable.

**El disparador fue un reporte del usuario jugando con el laboratorio**: *"cuando se construye después de que
ya la muralla está, los edificios se montan sobre las celdas de la muralla"*. Correcto, y era exactamente el
límite del Paso 1: mientras el trazo era solo una consulta, **ningún archivo de `src/engine` importaba
`muralla.ts`** y la colocación era ciega al anillo.

- [x] `Recinto` y `CeldaMuro` en `domain/types.ts` (estado PERSISTIDO, no geometría derivada) +
      `Asentamiento.recintos`
- [x] `comprometerRecinto` congela el trazo en el asentamiento. `avance` arranca completo EN ESTE PASO: la obra
      progresiva es 2b, y la única línea que cambia entonces es esa
- [x] **Las celdas de muro y torre entran en `sueloOcupado`** — las de puerta no, son transitables
- [x] `redDeCalles` ve el muro como ocupado: ninguna calle ni corredor puede nacer encima ni atravesarlo
- [x] `TrazadoAsentamiento.murallas` en el contrato de dibujo; el laboratorio gana **Comprometer** y **Quitar**
- [x] `muralla.test.ts` sube a **40 tests** con los invariantes de §15 nº 4 sobre dos ciudades amuralladas y
      150 ticks de construcción POSTERIOR — que es donde estaba el fallo, no en comprometer
- [x] Verificado en el laboratorio: seed 60, muro en el tick 150 (36 edificios) → tick 350 con **68 edificios y
      CERO pisando el muro**, con arrabal creciendo fuera del anillo

**Dos fallos que solo aparecieron al escribir el código, y el segundo es el más caro de toda la mecánica:**

1. **Sembrar las puertas como calle partía la red en pedazos.** La idea era "así la red no pierde ni una celda
   y la conectividad se conserva por construcción". Lo que hacía en realidad era dejar **islas de calle
   sueltas** en mitad del muro, sin nada que las uniera al resto — `anadirConectadas` existe precisamente para
   no añadir tramos huérfanos, y este atajo se lo saltaba. Medido: **71 celdas alcanzables de 175**. Corregido
   invirtiendo el modelo: una puerta es un **HUECO** en el anillo, no una calle prefabricada. No bloquea, y el
   replay la convierte en calle sola cuando estira un corredor a través de ella.

2. **Dos definiciones del mismo "suelo ocupado", y una no conocía la muralla.** `anclaActivaParaCategoria`
   —la consulta que decide a qué ancla se pega el próximo satélite— montaba su propio conjunto con
   `conCeldasDeRed(celdasOcupadas(...), red)`, sin las celdas de muro. Resultado: daba el Centro Urbano por
   USABLE porque veía 5-8 huecos en su banda **que eran el propio anillo**, mientras la colocación real, que sí
   los contaba, encontraba CERO. Como el ancla se reportaba usable, nunca se marcaba `anclaLlena`, nunca nacía
   una Plaza de relevo, y la ciudad **dejaba de construir viviendas para siempre**.

   Medido antes de arreglarlo: **68 edificios sin muralla contra 47 con ella, y la población congelada en la
   mitad (600 → 315)**. Después: seed 60 vuelve a 68 edificios y 600 pesants, idéntico a no tener muralla.

   Es la misma clase de bug que hizo centralizar `sueloOcupado` en el Paso 2 de la Etapa 6 —*"olvidarlo en UNO
   de los cuatro puntos de colocación bastaba para plantar edificios sobre las calles"*— reaparecida por la
   única puerta trasera que quedaba. Cerrada de raíz: ahora hay **una sola** definición exportada
   (`ocupadasConRed`) y las dos la usan.

   El test que lo congela no exige que la ciudad crezca —una ciudad puede llenarse de verdad dentro de su
   recinto— sino que **no se quede en deadlock**: o crece, o marca su ancla como llena. Esa distinción es la
   que separa "está llena" de "está confundida".

**Lo que este paso NO resuelve, y es esperado**: seeds 99 y 42 siguen frenándose (49 y 53 edificios contra 68
y 78). Ahí el núcleo está lleno de verdad y el arrabal todavía no arranca por sí solo — es el Paso 4, y el
laboratorio ya enseña arrabal creciendo en las ciudades que tienen sitio junto a una puerta.

#### Paso 2b. La obra y su coste — ✅ completada 2026-08-31
- [x] **Comprometer es gratis**; `avance` arranca en -1 y el anillo se levanta a `celdasPorMinuto` celdas por
      tick, **pagando cada celda al levantarla** con su tarifa por clase (puerta ×4, torre ×3)
- [x] `avanzarObraDeRecintos` colgada de `avanzarConstruccion`, **después** de `evaluarNecesidades`: la
      construcción normal tiene preferencia sobre los materiales, porque una ciudad que deja de producir por
      levantar su muro no sobrevive para verlo terminado
- [x] Sujeta a la MISMA reserva de mantenimiento que la cola. Sin materiales **la obra no avanza y ya**: no
      falla, no se cancela, no acumula deuda. Una muralla parada se ve parada
- [x] **Fuera de la cola de edificios** y sin ocupar hueco de `maximoEnConstruccionSimultanea`: es una obra
      pública, y meter 100 entradas en una cola con tope 8 haría ilegible el panel y el orden de pago
- [x] `abandonarRecinto`: borra un recinto INCOMPLETO entero y libera su suelo, sin devolución. Uno terminado
      no se puede abandonar (en Fase 0 no se destruye nada). Es una escotilla de seguridad, no una mecánica:
      sin ella una obra atascada por falta de piedra bloquearía su franja de suelo para siempre
- [x] Evento de dominio `construccion.recinto_completado` con payload tipado
- [x] El dibujo separa **levantado** de **pendiente**: `TrazadoMuralla.planificado` trae las celdas del anillo
      todavía no construidas, que se pintan con el **mismo contorno rojo que un edificio `en_construccion`**
      — porque es literalmente lo mismo: suelo ya comprometido que aún no está en pie. Sin esto ocupaban suelo
      y eran INVISIBLES, así que en su sitio se veía césped vacío que rechazaba edificios sin explicación:
      la peor clase de bug, el que parece del motor y en realidad es una omisión del dibujo *(lo señaló el
      usuario antes de que llegara a pasar)*. Congelado con un test: **la suma de celdas dibujadas —levantadas
      más pendientes— es siempre el anillo entero, para cualquier `avance`**
- [x] El tooltip del laboratorio distingue las tres situaciones: `levantada`, `por construir · su suelo ya
      está ocupado: nadie puede edificar aquí`, y presupuesto sin comprometer
- [x] Laboratorio: **Abandonar** (mecánica real) junto a **Quitar** (atajo del lab), y el estado de la obra en
      vivo — `nivel 2 · 79% (en obra 60/76) · 3 puertas`
- [x] `muralla.test.ts` → **49 tests**. Suite completa **747**, `tsc` limpio en motor y lab
- [x] Verificado en el laboratorio: seed 60, comprometer en el tick 150 (76 celdas, gratis, `avance` -1) →
      exactamente 1 celda por tick (10 ticks = 10/76, 60 ticks = 60/76), con el anillo a medio cerrar visible

#### Paso 2c. Los mandos y la medición en batch — ✅ completada 2026-09-01 (con una pieza abierta, ver abajo)
Separado de 2b porque son las dos cosas que hacen falta para que la muralla exista en una PARTIDA y no solo en
el laboratorio — hasta ahora solo el lab comprometía recintos.
- [x] **Adición manual** (`comprometerRecintoManualmente`/`abandonarRecintoManualmente`, `engine/muralla.ts`):
      mismo camino que `anadirEdificioManualmente` con Barracón/Galería/Mercado — gate de cargo (Gobernador
      o Maestro de Obras para comprometer; **solo Gobernador** para abandonar, es decisión de gobierno, no de
      obra) y gate de nivel de asentamiento (`MURALLA.nivelMinimoConstruccion = 3`, el mismo que tenía el
      edificio `muralla` que este recinto sustituye). Cableado hasta la API real: comandos de partida
      `comprometerRecinto`/`abandonarRecinto` (`session/comandos/murallas.ts`), registrados en
      `REGISTRO_COMANDOS`, con fila propia en `MATRIZ_AUTORIZACION` y en `ESQUEMAS_PARAMS`, y
      `RecintoInvalidoError` mapeado a un código de dominio estable (`recinto.invalido`, el 14º). El
      laboratorio sigue llamando a `comprometerRecinto`/`abandonarRecinto` SIN gate a propósito: sus
      asentamientos de prueba no siempre tienen cargos asignados, y no debería necesitarlos para probar
      geometría.
- [x] **Orden en la gobernanza NPC** (`asegurarMuralla`, `session/npcGobernanza.ts`, después del núcleo
      militar): mismo agujero que ya cerró `asegurarNucleoMilitar` con Barracón/Galería
      (`issues/nivel_3_inalcanzable_sin_jugador_humano.md`) — sin auto-construcción de murallas (§8), ninguna
      Facción del batch sin jugador humano llegaría jamás al gate de nivel 4 que el recinto sustituye (Paso
      5). Solo el PRIMER recinto (nivel 1, el más barato): si ya hay uno, este paso no hace nada — ampliar
      tiene sus propios gates y es del Paso 3, no de este. Verificado por test dedicado
      (`session/__tests__/murallaNpc.test.ts`, nivel 3 forzado) — **no** por una corrida de batch real: una
      corrida de 3000 ticks/20 facciones (0 excepciones, invariantes 4/6 intactos) terminó con NINGÚN
      asentamiento en nivel 3 todavía, cuello de botella de progresión preexistente y ajeno a murallas (los
      5 edificios del gate de nivel 3 tardan más que esta ventana con la calibración actual de recursos). Con
      el batch más largo o mejor calibrado, `asegurarMuralla` debería empezar a dispararse solo.
- [x] **Invariantes 4 y 6 en batch**: `scripts/run-batch-sim.ts` media `componentesDeRedMedia` y
      `edificiosConFrenteRealPct` desde siempre, pero `medirCalles` llamaba a `redDeCalles` SIN `recintos` —
      con la gobernanza NPC ya comprometiendo murallas, eso habría medido una red que ya no es la que el
      motor usa de verdad (el mismo bug que costó 68→47 edificios en el Paso 2a, ver más arriba). Una línea:
      pasar `asentamiento.recintos`.
- [x] **Invariante 9** (§5.1, el conjunto de puertas no cambia tras cerrar el muro): garantizado por
      CONSTRUCCIÓN, no solo medido — `recinto.celdas` no tiene ningún punto de mutación en todo `engine/`
      (comprobado por grep), así que no puede cambiar aunque la ciudad siga creciendo alrededor. Ya lo
      congelaban los tests del Paso 2a (150 ticks de construcción posterior, dos seeds) y ahora corre también
      dentro del batch en cada asentamiento amurallado, sin excepciones en las corridas de humo (1200-3000
      ticks).
- [x] **La medición del eje fortaleza↔metrópoli** (§0), parcial — la tabla final de `run-batch-sim.ts` saca,
      por cada recinto vivo: `comprometidoEnTick`, nivel, celdas totales, puertas, torres, **área encerrada**
      (`areaEncerradaDeRecinto`, nueva en `engine/muralla.ts` — reconstruida de las celdas YA CONGELADAS del
      recinto, no del trazado actual, así que da el mismo número en cualquier tick posterior al compromiso) y
      **coste total** (`costoDeTrazo`). **Abierto**: el **largo medio del corredor de afueras** de la lista
      original NO está — no tiene una definición estable todavía porque el Paso 4 (arrabal) no existe: no hay
      filtro intramuros ni preferencia de sitio que darle forma al corredor que se mediría. Se retoma dentro
      del Paso 4, con la métrica ya construida esta vez sobre arrabal real en vez de sobre una aproximación.

#### Paso 3. Niveles, torres y ampliación — ✅ completada 2026-09-01
- [x] **Mejora de nivel** (`iniciarMejoraDeRecinto`/`iniciarMejoraDeRecintoManualmente`, `engine/muralla.ts`):
      gratis empezarla, gate "recinto completo" + "nada mejorándose ya" + "no está ya en `nivelMaximo`".
      Reinicia `avance` a -1 y marca `Recinto.mejorandoA` (campo nuevo en `domain/types.ts`, ya estaba en el
      contrato del §3 desde el diseño original) — la MISMA `avanzarObraDeRecintos` de la construcción recorre
      otra vez el anillo entero pagando la tarifa de MEJORA por celda (`tarifaPorCelda[mejorandoA]`), y al
      terminar sube `nivel`, borra `mejorandoA` y reclasifica torres — **sin retrazar nada** (§6): mismo
      anillo, mismas puertas, solo algunas celdas de `'muro'` pasan a `'torre'`.
      Reclasificación implementada reutilizando `marcarTorres` (ya existía, Paso 1) sobre un `region`
      reconstruido del recinto congelado — nueva `regionInteriorDeRecinto`, la misma inundación que ya usaba
      `areaEncerradaDeRecinto` (Paso 2c), ahora compartida entre las dos.
      Comando de partida `mejorarRecinto` (`session/comandos/murallas.ts`), cableado igual que los otros dos
      del Paso 2c (registro, matriz de autorización, esquema de params).
- [x] **Ampliación** (§10): nuevo gate en `comprometerRecinto` — al menos `MURALLA.arrabalMinimo` edificios
      urbanos (no Granja/Corral, que están fuera por diseño desde el Paso 0 y no son "arrabal") fuera del
      interior del recinto exterior actual. La otra mitad del gate de §10 ("recinto completo") no necesitó
      código propio: el guardia genérico que ya existía ("hay un recinto en obra") cubre cualquier recinto
      incompleto, éste incluido. La CONTENCIÓN (`contieneA`) y el invariante 7 ("el área nunca decrece") ya
      estaban del Paso 2a — son, de hecho, la MISMA propiedad: si `contieneA` pasa, el área nueva contiene a
      la vieja por construcción, así que "nunca decrece" es un corolario, no un chequeo aparte.
      **Sin verificar en una ampliación real de punta a punta**: lograr que una ciudad de fixture junte
      `arrabalMinimo` edificios urbanos CONECTADOS a la red fuera del anillo (no solo sintéticos para probar
      el gate) depende del arrabal creciendo solo, que el Paso 4 todavía no soporta con una preferencia
      explícita — mismo límite que ya dejó abierto el Paso 2c con el corredor de afueras. Cubierto por tests
      con edificios sintéticos que sí ejercitan el gate en sí (aceptan o rechazan por la razón correcta), no
      por una ampliación que de verdad se complete.
- [x] `muralla.test.ts` → 69 tests (10 nuevos: 3 de ampliación, 7 de mejora). Suite 748→758, `tsc` limpio en
      motor y cliente.

#### Paso 3b. Defensa y upkeep — la razón de ser, aislada — 🟡 mecanismo hecho, medición bloqueada (2026-09-01)
Va en su propio paso justo por lo que la decisión 3 quería proteger: son los dos únicos cambios de BALANCE de
toda la mecánica, y mezclarlos con la geometría haría ilegible cuál movió qué.
- [x] **Multiplicador defensivo** en `iniciarAsedio`: `1 + (bono[nivel] − 1) × integridad / nºPuertas` (§16).
      `multiplicadorDefensivoDeRecintos` (`engine/muralla.ts`) lee el recinto EXTERIOR (el último tras una
      ampliación); `resolverCombate` gana un parámetro `multiplicadorDefensor` (1 por defecto) que
      `iniciarAsedio` pasa y `combateCampoAbierto` NUNCA pasa — en mundo abierto no hay ningún recinto que
      atravesar. Cubierto en `combate.test.ts` (nuevo: no había ningún test directo de asedio antes de esto),
      con un enfrentamiento controlado (rng sin varianza) que de verdad cambia de ganador con y sin muro.
- [x] **Upkeep por celda y nivel**, sumado a `calcularCostoMantenimiento`. `upkeepDeRecintos` cuenta solo
      celdas YA LEVANTADAS, a la tarifa del nivel ACTUAL del recinto (nunca `mejorandoA`: lo que hay en pie es
      lo que hay que mantener). Resuelve también el corolario de §16/§20 que quedaba abierto: sin degradación
      de integridad aparte, el mismo mecanismo de déficit de `avanzarMantenimiento` responde por los dos.
- [ ] **Medir el batch antes y después de estas dos líneas solas**: asedios resueltos, vivos/colapsados,
      medidor de mantenimiento medio. **Bloqueado**: el batch no consigue que ningún asentamiento llegue a
      nivel 3 en 3000 ticks (mismo cuello de botella que ya afectó la verificación de `asegurarMuralla`, Paso
      2c) — sin un recinto real no hay nada que medir. Sustituido por una justificación aritmética a mano en
      §16 (el upkeep de nivel 3 come 7-9 piedra/minuto contra un mantenimiento que ya cobra ~39/minuto a
      población tope), que debe confirmarse en cuanto ese cuello de botella se resuelva o en el Paso 6.
- [ ] Comprobar el **riesgo 11** con el bono ya activo: ¿la fortaleza temprana de una puerta es dominante?
      Los contrapesos (todo crece extramuros, el nivel 3 es caro, el upkeep cobra siempre) se validan aquí.
      Mismo bloqueo que el punto anterior — sin batch que llegue a nivel 3, no hay datos que comprobar contra.

#### Paso 4. Arrabal y presión intramuros — ✅ completada 2026-09-01 (con el mismo límite de siempre)
- [x] **Filtro intramuros con fallback** (§9): `conPreferenciaIntramuros` (`engine/trazado.ts`), aplicado dentro
      de `sitiosParaTipo` a las dos rutas de sitio que devuelven una LISTA de candidatos —
      Palacio/Almacén/Leñera y la atracción dura ancla-satélite (`sitiosPorAtraccionDura`); Granja/Corral
      (siempre extramuros por diseño) y la creación de una ancla nueva (un único candidato, no una lista) se
      quedan fuera a propósito. Es una PARTICIÓN ESTABLE (dentro primero, fuera después), no un término nuevo
      del desempate — no toca el orden lexicográfico de los cuatro perfiles de §E6.23. Sin recinto exterior
      completo, no-op exacto (probado). Reutiliza el mismo interior-de-recinto que `areaEncerradaDeRecinto`
      (Paso 2c), pero reimplementado con claves de texto en `trazado.ts` en vez de importado de `muralla.ts`
      (que usa claves numéricas y ya importa de `trazado.ts` — el import inverso habría creado un ciclo; misma
      duplicación deliberada que ya tenían `celdasBloqueadasDeRecintos`/`celdasDePuertas`). 2 tests nuevos en
      `muralla.test.ts` (con `almacen`, no `vivienda`: la ancla residencial activa a tick 200 ya suele haber
      migrado ella misma al arrabal por el fix del Paso 2a, así que sus candidatos salen siempre fuera y no
      prueban la partición — medido con un script de sondeo efímero, ya borrado).
- [x] **Medir `arrabalPct`, `ocupacionNucleoPct` y `manzanasCerradasMedia`**: instrumentado en
      `run-batch-sim.ts` (`arrabalPct` nuevo; los otros dos ya existían de la Etapa 6, pero `manzanasCerradas`
      tenía el MISMO bug que `medirCalles` — llamaba a `redDeCalles` sin `recintos`, arreglado de paso). **En
      el batch real sigue sin poder medirse**: mismo cuello de botella de nivel 3 que ya bloqueó Paso 2c/3b.

      `scripts/medicion-arrabal.ts` (efímero) rodeó el bloqueo: materiales de sobra, un anillo nivel 1
      comprometido justo cuando la ciudad toca su techo de nivel 2 (~tick 300), y ahí mismo se le da a mano el
      `radioPotencial`/`nivel` de nivel 3 —da igual si nivel 3 es alcanzable jugando (issue ya documentado
      aparte) para responder esta pregunta, que es qué le pasa al TRAZADO cuando la ciudad sigue creciendo
      por delante de un anillo que ya no la sigue. Dos corridas del mismo seed (con muro vs. sin muro nunca),
      3 seeds, hasta 1200 ticks (la población se estabiliza en 1200 pesants —techo de nivel 3— hacia el tick
      700 en las 6 corridas):

      | seed | `arrabalPct` final (con muro) | `ocupacionNucleoPct` control → con muro | `manzanasCerradas` control → con muro |
      |---|---|---|---|
      | 7  | 51.1% | 41.4 → 43.4 (+2.0) | 20 → 19 (−1) |
      | 60 | 51.8% | 39.7 → 39.2 (−0.5) | 15 → 23 (+8) |
      | 99 | 54.5% | 38.2 → 30.8 (**−7.4**) | 27 → 35 (+8) |

      **El arrabal nace y crece de verdad**: 51-55% de los edificios urbanos terminan extramuros — ni se ahoga
      contra `capCorredorUrbano` (12 celdas) ni necesita ayuda. **El riesgo 1 ("la presión intramuros devuelve
      el coágulo") NO se confirma con estos datos**: `ocupacionNucleoPct` con muro es igual o MENOR que el
      control sin muro en 2 de 3 seeds (hasta 7.4 puntos menos en la seed 99) — la preferencia intramuros
      llena huecos existentes antes que amontonar, y en cuanto el núcleo se satura de verdad, desborda al
      arrabal en vez de comprimirse. `manzanasCerradas` sí sube más con muro en 2 de 3 seeds (más manzanas
      cerradas, calles más entrelazadas) — no está claro si es bueno, malo o neutro por sí solo (es un
      guardián de regresión, no una métrica con dirección "mejor/peor"), pero no hay señal de colapso
      (siempre > 0, sin caída a 0 componentes ni desconexión).

      **Límite de esta medición, honesto**: es UN anillo nivel 1 comprometido pronto (tick 300, la ciudad más
      pequeña posible que puede tener uno) con espacio de nivel 3 dado a mano, sobre 3 seeds — no una
      ampliación, no un anillo tardío/grande, no una corrida de cientos de facciones. Es evidencia de que el
      mecanismo FUNCIONA y no es catastrófico, no un veredicto de calibración final (eso es el Paso 6).
- [x] **`capCorredorArrabal` propio: NO hace falta, por ahora**. Con `capCorredorUrbano` = 12 celdas el
      arrabal alcanza 51-55% de los edificios urbanos en las tres seeds medidas — muy lejos de "ahogado". Si
      el Paso 6 (playtest real, con ciudades más grandes y anillos más tardíos) encuentra lo contrario, la
      palanca ya está identificada y aislada (§9); no hace falta tocar nada hasta entonces.

#### Paso 5. Retirada del `muralla` viejo y gate de nivel 4 — ✅ completada 2026-09-01
Los ocho puntos de §13, uno por uno:
- [x] `domain/types.ts`: `'muralla'` fuera de `EdificioTipo` y de `TODOS_LOS_EDIFICIOS`.
- [x] `constants.ts`: `muralla` fuera de `EDIFICIO_CATALOGO`.
- [x] `NIVEL_ASENTAMIENTO.requisitos[4]`: `edificios: ['muralla']` → `edificios: [], recintoCompletoNivelMinimo: 1`
      (nuevo campo). `calcularNivelAsentamiento` (engine/mantenimiento.ts) y `progresoNivelAsentamiento`
      (engine/asentamientoQuery.ts) extendidos con `tieneRecintoCompletoDeNivelMinimo` — cualquier recinto
      TERMINADO de nivel ≥ 1 basta, la empalizada barata cuenta igual que la muralla de piedra. El campo
      nuevo de `progresoNivelAsentamiento.siguiente.recinto` (`{cumplido, nivelMinimoRequerido}`) es aparte de
      `edificiosFaltantes` porque un `Recinto` no es un `EdificioTipo` — no cabía ahí.
- [x] `scripts/run-batch-sim.ts`: `murallasActivas` (contaba `edificiosPorTipoYEstado(a,'muralla')`, ya
      imposible de compilar) → `celdasMuroMedia` (nº medio de celdas de los recintos completos);
      `asentamientosConRecintoCompleto`/`arrabalPct` ya existían de los Pasos 2c/4.
- [x] `cliente/src/main.ts`, `cliente/src/ui/canvas.ts`, `lab/src/render.ts`, `lab/src/main.ts`,
      `scripts/catalogo-edificios.ts`: etiqueta, color `#5a5a5a` y entradas de catálogo retiradas. El lab ya
      tenía su "capa de muralla" propia desde el Paso 1 (dibujo del anillo con sus propios colores, aparte de
      `EDIFICIO_COLOR`) — no había nada que mudar ahí, solo que borrar. `cliente/` todavía no dibuja recintos
      en absoluto (ver Paso 6, "playtest en la interfaz"): no había nada de qué migrar tampoco.
- [x] Gobernanza NPC: ya lo hacía `asegurarMuralla` desde el Paso 2c — este punto llegó resuelto.
- [x] Partidas guardadas: `partidas/local.json` (snapshot local de desarrollo, sin usuarios reales) borrado.
- [x] **Verificado en vivo**: laboratorio (traza + compromete un recinto nivel 2, sin errores de consola, el
      catálogo de "Construcción manual" ya no ofrece Muralla) y servidor+cliente reales arrancados desde cero
      (sin la partida borrada) — la leyenda de edificios del mapa tampoco la lista, cero errores.
- [ ] **Verificar en batch que las facciones NPC vuelven a alcanzar nivel 4**: sigue bloqueado por el mismo
      cuello de botella de nivel 3 que ya impidió medir esto en los Pasos 2c/3b/4 — ningún asentamiento del
      batch llega ni siquiera a nivel 3, así que nivel 4 no es medible todavía. No es un problema nuevo de
      este paso, es el mismo de siempre sin resolver aún (ver `task_0a505c6d`).
- 8 tests nuevos en `muralla.test.ts` (`calcularNivelAsentamiento`/`progresoNivelAsentamiento` con y sin
  recinto). Suite 779→783, `tsc` limpio en motor, cliente y laboratorio.

#### Paso 6. Calibración — ⬜
- [ ] Tarifas, `franjaDeRonda`, `pasoTorres`, `celdasPorMinuto`, `arrabalMinimo`, por playtest del laboratorio
- [ ] **Playtest en la interfaz** (`cliente/`), no solo en el lab: los bugs más caros de este módulo los
      encontró el usuario mirando la pantalla

## 19. Riesgos conocidos, ordenados por lo que costarían

1. **La presión intramuros devuelve el coágulo.** Es el riesgo más caro: la Etapa 6 existió para abrir el
   núcleo (−23 puntos de ocupación) y esta mecánica empuja en sentido contrario a propósito. Guardianes:
   `ocupacionNucleoPct` y `manzanasCerradasMedia`, ya instrumentados.
2. **El muro interior asfixia el casco antiguo** tras una ampliación. Palanca: `franjaDeRonda`.
3. **El arrabal no alcanza la red** con `capCorredorUrbano` = 12 y se queda sin construir en silencio. Es el
   mismo fallo de §E6.17 punto 4, que ya se coló una vez estando avisado por escrito.
4. **§E6.16 sigue abierto**: el orden de commit por score puede plantar un edificio sobre un trazo recién
   comprometido, igual que hoy lo planta sobre una calle. Cerrarlo antes o a la vez.
5. **El trazo se rechaza a menudo** en ciudades de forma rara (perfil `caminera`, anclas muy dispersas) y la
   muralla resulta inconstruible. Medible en el Paso 1, y por eso el Paso 1 no lleva economía.
6. **El coste logístico de pocas puertas puede pasarse de frenada** (§5.1, reformulado tras §0). Que un
   recinto temprano tenga una sola puerta ya NO es el riesgo —es la ventaja que se está comprando—, pero su
   precio sí lo es: todos los caminos rurales rodean hasta ese único acceso y pueden llegar a rozar
   `capCorredorAfueras` (200 celdas contra un rodeo máximo del orden de 160). Si lo rozan, una Granja nueva
   deja de tener sitio válido **en silencio**, y el jugador ve su ciudad dejar de crecer sin saber por qué.
   Ese es el fallo a vigilar, no el embudo. Palanca: `capCorredorAfueras`, más un aviso explícito en la
   interfaz cuando una colocación de afueras se descarta por alcance. Medición: nº de puertas y largo medio
   del corredor de afueras, por tick de commit, en el Paso 2.

### 19.1 Hallazgos de la revisión por consejo (2026-08-31)

Cuatro riesgos que este documento no tenía y que salieron de pasar la especificación por un consejo de cinco
revisores independientes con revisión cruzada. Los cuatro son adiciones: ninguno revierte una decisión, pero
dos apuntan a decisiones que hay que reabrir (§20).

7. ~~**El ciclo vicioso de la salvaguarda.**~~ — **DISUELTO por §0.** El hallazgo era correcto *dada* la
   salvaguarda: si `puertasMinimas` se quedaba corta, la única cura era ampliar, que exige `arrabalMinimo` = 6
   edificios extramuros — o sea, sobrevivir estrangulado el tiempo suficiente para levantar 6 edificios fuera
   del muro que te asfixia. La cura exigía el síntoma. **Pero la salvaguarda ya no existe** (§5.1,
   `puertasMinimas` retirada): pocas puertas no es una patología que corregir, es la ventaja del defensor. Sin
   salvaguarda no hay ciclo. Se deja anotado porque es el ejemplo más limpio de un parche que fabrica el
   problema que dice resolver.

8. ~~**Incentivo perverso: el muro castiga al jugador prudente.**~~ — **INVERTIDO por §0.** El hallazgo
   asumía que menos puertas era peor para el defensor. Es al revés: quien amuralla pronto obtiene un embudo
   defensivo barato (fortaleza) a cambio de dejar todo su crecimiento futuro extramuros; quien amuralla tarde
   protege mucha más ciudad a cambio de un perímetro caro, más puertas y más frente que cubrir. Es un
   intercambio con dos extremos legítimos, no un castigo. **Lo que sí sobrevive del hallazgo** es que el
   documento nunca se había preguntado si la consecuencia era la deseada — ahora §0 responde que sí, y por qué.

9. **Partida por tiempo.** El proyecto implementó partidas de duración acotada (`d3cc09a`). Una obra a
   `celdasPorMinuto` = 1 sobre un anillo de ~76 celdas son ~76 ticks solo para la empalizada, más la mejora.
   Contra una partida corta, la muralla puede ser **literalmente inalcanzable**, y con ella el nivel 4 y el
   nivel 5. Hay que comprobar el presupuesto de ticks de una partida típica contra el coste temporal completo
   de un recinto antes de fijar `celdasPorMinuto`.

10. **Equidad competitiva.** El anillo sale del historial de crecimiento de cada ciudad, que el jugador no
    decide directamente. Dos jugadores igual de buenos pueden acabar con recintos objetivamente distintos
    —más puertas, mejor relación perímetro/área, más suelo encerrado— sin que ninguna decisión suya lo
    explique. En un juego de un solo jugador es sabor; en uno multijugador es balance. Medible: dispersión de
    `celdasMuro`, nº de puertas y área encerrada entre las 100 facciones del batch, al mismo tick de commit.

12. **La ventaja se evapora en la metrópoli** (medido en el Paso 1, §18). Con la fórmula de §16, un recinto
    de nivel 3 defiende **×2.50** con 1 puerta y **×1.15** con 10. Como una ciudad de nivel 3 tiene 5-10
    puertas medidas y paga 3-4 veces más perímetro, **la ciudad grande paga mucho más por bastante menos**. El eje
    fortaleza↔metrópoli se convierte así en una jugada dominante (amurallar pronto) en vez de un intercambio,
    que es justo lo contrario de §0. Palanca a probar en el Paso 3b: suavizar el divisor —`/ √nºPuertas` da
    ×2.50 contra ×1.47, un rango que sigue premiando el embudo sin anular el muro grande— o repartir el bono
    por FRENTE en vez de por puerta. No se toca hasta medirlo con el bono activo.

    El bono ya está activo (`multiplicadorDefensivoDeRecintos`, Paso 3b) — la fórmula implementada es la
    LINEAL de §16 (`/ nºPuertas`), sin la alternativa `/ √nºPuertas` todavía: sigue siendo la misma medición
    pendiente, ahora ya ejecutable, solo que bloqueada por el mismo cuello de botella de nivel 3 del batch que
    el riesgo 11 de abajo. Cuando se resuelva, comparar las dos fórmulas es cambiar una línea
    (`multiplicadorDefensivoDeRecintos`), no un rediseño.

11. **La fortaleza temprana barata** (riesgo NUEVO, consecuencia directa de §0 y el único que la razón de ser
    introduce en vez de resolver). Si el coste es por celda y la ventaja defensiva sube al bajar el nº de
    puertas, la jugada extrema es **amurallar lo antes posible**: perímetro corto ⇒ barato, una sola puerta ⇒
    embudo. Ventaja máxima al precio mínimo.

    **Tres contrapesos ya están en el diseño, sin código nuevo:** (a) el recinto barato encierra poco, así que
    casi toda la ciudad futura crece extramuros y desprotegida (§9); (b) la ventaja *abrumadora* exige nivel 3
    —piedra, adarve, guarnición—, que se paga por celda **y** por nivel, y una ciudad diminuta no lo sostiene;
    (c) el upkeep por celda (§16) cobra todos los ticks. **Hay que comprobar que bastan**: si no, la palanca es
    un mínimo de nivel de asentamiento o de población para poder trazar, no un parche geométrico. Medición:
    tick de commit más temprano viable × área encerrada × coste total, en el batch.

    **✅ (b) y (c) implementados en el Paso 3b** — ver el bloque "Implementado" de §16 para la aritmética
    completa (nivel 3 añade 7-9 piedra/minuto de upkeep sobre un mantenimiento que ya cobra ~39/minuto a
    población tope, contra un techo de extracción propia de ~50/minuto). **Sigue sin comprobarse en batch**:
    bloqueado por el mismo cuello de botella de progresión que ya impidió medir `asegurarMuralla` (Paso 2c) —
    ningún asentamiento del batch llega a nivel 3 en 3000 ticks con la calibración actual de recursos.

## 20. Abierto — lo que este documento deja sin cerrar a propósito

- **Cifras**: todas las de §17 son placeholder. Se calibran en el Paso 6, no antes.
- **Guarnición sobre el adarve** (nivel 3, §16): especificada, fuera de esta pasada. El multiplicador
  defensivo y el upkeep SÍ entran (decisión 3 revisada, **✅ Paso 3b, 2026-09-01**).
- ~~**Degradación de integridad por upkeep impagado** (§16): propuesta, sin decidir.~~ — **decidido en el
  Paso 3b: NO.** A petición del usuario, el upkeep se suma dentro de `calcularCostoMantenimiento` — el mismo
  pote que ya puede bajar `nivelActual`, sin una segunda vía de castigo aparte que solo tocara la integridad
  del muro (eso habría dejado a la ciudad a salvo de su propia elección de amurallarse, justo lo contrario de
  lo que se pedía). Sigue sin existir NINGÚN camino de destrucción de algo ya construido en Fase 0 — el muro
  en sí nunca pierde celdas, lo que se resiente es el asentamiento entero, igual que con cualquier otro
  déficit de mantenimiento.
- **Brecha, daño y armas de asedio**: Fase 1, junto con el asedio instanciado.
- ~~**`radioAfuerasMin` puede moverse** según lo que mida el Paso 0~~ — **medido: no se mueve** (§11.1). Lo
  que se retiró fue el tope de radio del muro, no esa constante.
- **Ciudad y afueras se entremezclan a partir del nivel 3** (0.3-12 % de las celdas urbanas caen más lejos que
  la granja más cercana, §11.1). Es un hallazgo del trazado EN GENERAL, no de las murallas — la mecánica solo
  lo destapó, y lo absorbe sin problema con el entrante de §11.2. Merece decidirse aparte si "las afueras"
  deben seguir significando algo a partir de nivel 3.
- ~~**`puertasMinimas`**~~ — **RETIRADA** (§5.1): bajo la razón de ser de §0 abriría a la fuerza brechas que
  el defensor no quiere. Ya no es una decisión abierta.
- **Demolición de un recinto completo**: hoy no existe destrucción de nada en Fase 0 (§"Abierto" del doc de
  trazado). Cuando exista —incendio, asedio, abandono—, el muro es el primer candidato a tener estado dañado
  por celda, y el modelo de `CeldaMuro` ya lo admite sin cambios.
- **Jerarquía de puertas** (postigo / puerta / casa-puerta) más allá del multiplicador de tarifa: aplazada,
  igual que la jerarquía callejón/avenida de §E6.3.

### 20.1 Lo que la revisión por consejo pidió reabrir, y en qué quedó

- ~~**Regla de puertas congeladas.**~~ **CERRADA a favor de la regla actual** (§0, aclaración del usuario). El
  consejo la señalaba como causa raíz de tres riesgos porque asumía que pocas puertas perjudicaba al defensor.
  Es al revés. La regla se mantiene, `puertasMinimas` se retira, y lo único que se exige es que el presupuesto
  enseñe el **nº de puertas** antes de pagar: es el dato irreversible del trazo, y el jugador tiene que elegir
  la fortaleza a sabiendas. Descartada también la alternativa de "abrir puerta" de pago sobre un recinto ya
  construido: sería una forma de renunciar voluntariamente a la ventaja, y no hay razón para pagar por
  debilitarse.
- ~~**Decisión 3 (efecto defensivo y upkeep fuera de esta pasada).**~~ **REABIERTA Y CAMBIADA** (2026-08-31).
  §0 convierte la defensa en la razón de ser: sin ella lo que se entrega es un peaje caro e irreversible cuyo
  único beneficio es el gate de nivel 4, y el upkeep no es un extra sino la mitad de *"difícil de obtener **y
  de mantener**"*. Entran las dos piezas mínimas —multiplicador defensivo y upkeep por celda— aisladas en su
  propio paso (§18, Paso 3b) con su propia medición, que es lo que conserva la atribución que la decisión 3
  quería proteger.

### 20.2 Ganchos futuros que la revisión sacó a la luz

No son alcance de esta mecánica, pero conviene que estén escritos antes de que otra mecánica invente su propio
número para lo mismo:

- **`region` es una zona de influencia geométrica** ya calculada. Las rutas de caravanas (§3 del backlog)
  necesitan exactamente eso para decidir si una ruta cruza una ciudad y paga comisión.
- **Las torres son landmarks deterministas** (§10 del backlog) para el cliente 3D, sin arte extra.
- **Intramuros / extramuros es una asimetría informacional natural** para la niebla de guerra (§12 del
  backlog).
- **El estrato de muros concéntricos** es el único sistema del juego que escribiría historia visible en el
  terreno: una ciudad que sobrevivió tres ampliaciones se **ve** como tal, solo con geometría acumulada.
- **La obra progresiva es visible en directo**: un rival viendo cerrarse un anillo es tensión narrativa
  gratuita ("atacar antes de que cierren la puerta") el día que exista movimiento de ejércitos.
