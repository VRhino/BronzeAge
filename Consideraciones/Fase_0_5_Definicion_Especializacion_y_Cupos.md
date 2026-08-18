# Fase 0.5 — Especialización de asentamientos, cupos por nivel y dependencia entre niveles

> DISEÑO EN CURSO, parcialmente implementado — ver §0.1 "Estado de implementación". Sometido a un consejo LLM
> (5 asesores + revisión cruzada) antes de fijar las decisiones de este documento; sus hallazgos están
> incorporados en el cuerpo del texto, no en una sección aparte. Todas las cifras son placeholder salvo las que
> se citan como estado actual del código.
>
> **Actualización posterior del usuario**: los puntos abiertos #1 y #2 de §7 quedan cerrados (cupo automático
> por orden de llegada; conquista evade el cupo a propósito, como incentivo a la guerra), el hallazgo de
> §0.1/§7 punto 6 queda resuelto (no es un deadlock: hay demasiadas fuentes de XP de Facción como para que
> llegar a nivel 2 sea un bloqueo real), y el resto del diseño todavía no implementado (§3.3, §3.4, §3.5, §3.6,
> §4/§4.1, y los puntos 3, 4 y 7 de §7) queda **descartado del alcance de Fase 0.5** — se conserva el texto
> como registro de la idea, marcado en cada sección.

## 0. Problema que resuelve

Hoy todos los asentamientos tienden a lo mismo: crecer hasta nivel 3 y autoabastecerse. No hay razón mecánica
para tener asentamientos distintos entre sí, ni para que uno dependa de otro. Se busca:

1. Que un asentamiento de **nivel 1 pueda producir mucho más de lo que necesita** para subsistir.
2. Que los de **nivel alto dependan de los bajos** para recursos básicos sin procesar, siendo su fuerte la
   **transformación**.
3. Que exista un **cupo de asentamientos por nivel**, derivado del nivel de Facción, para que no todos puedan
   ser de nivel alto y cada uno tenga una función.

Corrección temprana del usuario, que reorienta todo el documento: **ningún edificio produce menos por estar en
un nivel alto** — un nivel alto no consume MÁS por castigo, consume más porque **tiene más gente, más ejército
y más construcciones** (más población, más mantenimiento, más costes de mejora). La dependencia se construye
por el lado de la demanda, nunca reduciendo producción. Ver §2.

### 0.1 Estado de implementación (esta sesión)

Antes de diseñar más, se instrumentó y midió el sistema actual para no construir sobre supuestos. Eso llevó a
implementar y validar tres piezas — **estas SÍ están en el código**, el resto de este documento sigue siendo
diseño:

1. **`pausadoPorAlmacenLleno`** (`domain/types.ts` `Edificio`, `engine/almacen.ts`
   `agregarRecursoConSobrante`, conectado en los 4 puntos de producción de `engine/construction.ts`): antes,
   la producción que no cabía en el almacén (`Math.min(capacidad, ...)`) se perdía en silencio. Ahora el
   edificio queda marcado, visible para UI/instrumentación.
2. **Mercado sin piedra en su costo BASE** (`constants.ts` `EDIFICIO_CATALOGO.mercado.costo`, antes
   `{madera:100, piedra:40}`, ahora `{madera:100}`): se detectó un deadlock real y total al instrumentar el
   punto 3 — ver §6.1.
3. **`engine/simulacionAutoComercio.ts`** — NPC de trueque **solo para simulación**, apagado por defecto
   (`SIMULACION_AUTO_COMERCIO.activo = 0`, togglable desde el panel de balance), que hace automáticamente el
   trabajo que en el juego real hace un Tesorero/Gobernador humano desde la UI (Doc 3.2 sigue siendo 100%
   manual — este módulo no lo sustituye, permite correr batches largos sin un jugador interactuando en cada
   tick). Marcado con un banner "borrar estos 3 sitios para eliminarlo por completo" (el archivo, el bloque
   `SIMULACION_AUTO_COMERCIO` de `constants.ts`, la llamada gateada en `gameStore.ts`).

**Validado de punta a punta** (mapa completo, seed 1, dos asentamientos de la misma Facción — uno con piedra
alcanzable, otro sin ella): Mercado construido, caravanas propias, trueque propuesto, entrega física por el
mapa — el asentamiento sin mineral pasó de 20 piedra (reserva inicial estancada para siempre) a 290 hacia el
tick 225, pagando con su propio excedente real de madera/trigo. El canal de suministro interno FUNCIONA una
vez destrabado el deadlock de arranque (§6.1).

**Segunda pasada — el núcleo mecánico del rediseño (§3.1, §5, §6.2, §8) YA ESTÁ IMPLEMENTADO**, sobre la base
de todo lo anterior:

4. **Techo de población por nivel** (§3.1) — `NIVEL_ASENTAMIENTO.techoPoblacion`, aplicado en
   `engine/population.ts` `crecerPoblacion` (escala el crecimiento de las 3 clases a la baja si juntas se
   pasarían del techo; nunca purga población ya asentada).
5. **Split `nivel` (nivelAlcanzado) / `nivelActual`** (§6.2) — `Asentamiento.nivelActual`/`rachaMantenimientoSano`
   nuevos (`domain/types.ts`), helper `nivelActualDe` (`engine/asentamientoQuery.ts`, fallback a `nivel` en
   partidas guardadas antiguas). Todos los gates de CONSTRUIR/MEJORAR/RECLUTAR/lanzar Caravana de Fundación
   (`engine/construction.ts`, `engine/expansion.ts`) leen `nivelActualDe`, no `nivel` — excepción deliberada:
   el techo de RADIO de zona (`ZONA_INFLUENCIA.radioMaximoPorNivel`) sigue leyendo `nivel` (nivelAlcanzado),
   el territorio no se encoge. `engine/mantenimiento.ts` `avanzarMantenimiento`: al tocar 0 el medidor,
   degrada `nivelActual` un escalón (reinicia el medidor a 100) en vez de destruir — solo cae en ruinas si ya
   estaba en `nivelActual` 1. Recupera un escalón tras `MANTENIMIENTO.ticksSanosParaRecuperarNivel` (30,
   placeholder) ticks SEGUIDOS de pago íntegro, nunca antes — verificado con script aislado: no sube ni un
   tick antes de la racha completa, degrada sin destruir desde nivel 2, sí destruye desde nivel 1.
6. **Mantenimiento por POBLACIÓN, no por nivel** (§3.2) — `calcularCostoMantenimiento` reemplaza
   `factorCrecimientoPorNivel` por `factorPoblacion = 1 + poblacionTotal/MANTENIMIENTO.poblacionReferencia`
   (placeholder 500). `nivelParaPiedra`/`nivelParaOro` (qué recursos se cobran) siguen atados a `nivel`
   (nivelAlcanzado, nunca baja) — coherente con "degradar no reduce mantenimiento". Se descarta la idea
   anterior de sumar coste por Nº de edificios: chocaba con el factor de distancia a la capital ya existente,
   que ya es el mecanismo real de "imperio disperso cuesta más" (ver §5.1).
7. **Nivel de Facción por EXPERIENCIA** (§8) — reemplaza por completo la fórmula de población total.
   `Faccion.experiencia` (monótona), `NIVEL_FACCION.xpParaNivel` (curva de umbrales acumulados) +
   `NIVEL_FACCION.xp` (cantidades por evento: combate, edificio completado, conquista, defensa/ataque de
   caravana). XP otorgada en `engine/combate.ts` (las 4 funciones de combate: asedio, campo abierto,
   interceptar caravana, atacar campamento de bandidos — mismo patrón que `aplicarAjustesReputacion` ya
   existente) y en `engine/simulation.ts` (construcción, vía `avanzarConstruccion.edificiosCompletados`
   nuevo). `interceptarCaravana`/`atacarCampamentoBandidos` ganaron parámetro `facciones` (antes no lo
   necesitaban) — actualizado en `gameStore.ts`.
8. **Cupo de asentamientos por nivel** (§5) — `CUPO_NIVEL_ASENTAMIENTO.maxNivel2`/`maxNivel3` (curva por
   nivel de Facción, invariante `maxN2+maxN3 = CAP_FUNDACION_POR_NIVEL-1` verificado), `calcularCupoNivel`
   (`engine/faccion.ts`). Enforcement en `avanzarNivelAsentamiento` (`engine/mantenimiento.ts`, ahora acepta
   un callback opcional `tieneCupoParaNivel`) — sube de a un escalón por vez, se detiene si no hay cupo en
   ese escalón (se queda "elegible, esperando cupo", NUNCA se bloquea ni retrocede). `engine/simulation.ts`
   calcula el cupo libre por Facción al INICIO del tick y lo va consumiendo/liberando según se conceden
   promociones dentro del mismo tick (subir de 2 a 3 libera el cupo de 2, disponible para otro asentamiento
   propio en la misma pasada — verificado con script: un salto directo 1→3 en la misma llamada pide cupo en
   orden 2 y LUEGO 3, nunca al revés). **Decisión interina tomada al implementar** (punto abierto #1 de §7,
   sigue sin decisión explícita del usuario): "el primero que llega" en orden del array de asentamientos —
   no hay promoción manual del Rey todavía. Conquista NO se re-evalúa contra el cupo (punto abierto #2,
   mismo criterio que `CAP_FUNDACION_POR_NIVEL`).

**184/184 tests en verde** tras las 5 piezas (dos actualizaciones de snapshot conscientes: nivel de Facción ya
no llega a nivel 2 tan rápido con XP como con población — es el cambio de diseño esperado; deriva numérica
menor en el snapshot de mantenimiento por el cambio de fórmula).

**Hallazgo de la pasada anterior, RESUELTO — no es un deadlock**: con `CUPO_NIVEL_ASENTAMIENTO.maxNivel2[0] = 0`
(nivel de Facción 1), ninguna Facción nueva puede promover un asentamiento a nivel 2 hasta subir de nivel de
Facción, y la Caravana de Fundación (Doc 1.8) exige nivel ACTUAL 2 como mínimo. Se planteó como bloqueo real de
expansión temprana. **Decisión del usuario**: no lo es — al existir muchas fuentes de XP (combate, construcción,
conquista, defensa/ataque de caravana), llegar a nivel de Facción 2 no es difícil en la práctica. No se aplica
ninguna excepción ni se toca la curva de cupos por este motivo (cierra el punto abierto #6 de §7).

## 1. Diagnóstico del estado actual (medido contra el código, no supuesto)

**Un asentamiento nivel 3 es cómodamente autárquico con las cifras actuales.** Con los gates vigentes
(`NIVEL_ASENTAMIENTO.requisitos`, 500 pesants + 200 artesanos = 700 habitantes):

| Recurso | Consumo real de un N3 | Techo de producción propia | Holgura |
|---|---|---|---|
| Trigo | 700 hab × 0.1 = **70/tick** | 10 Granjas × 15 = **150/tick** (450 si están mejoradas a nivel interno 4) | 2–6× |
| Madera | Mantenimiento 3 × 1.3 × 2 = **7.8/tick** | 10 Leñeras × 5 = **50/tick** | 6× |
| Piedra | **~7.8/tick** | 10 Canteras × 5 = **50/tick** | 6× |

Constantes implicadas: `POBLACION.consumoComidaPorHabitante = 0.1`, `MANTENIMIENTO.costoBase.madera = 3`,
`MANTENIMIENTO.factorCrecimientoPorNivel = 0.15`, `EXTRACCION_MAXIMOS.porTipo = 10`.

Hallazgos concretos:

- **`factorCrecimientoPorNivel = 0.15` es simbólico**: un nivel 3 paga solo un 30% más de mantenimiento que un
  nivel 1, frente a una holgura de producción de 6×.
- **El nivel NO controla la población.** La población la limita la Vivienda, que es auto-construida. Un nivel 1
  puede alcanzar 700 habitantes igual que un nivel 3. El nivel es hoy solo una llave de edificios.
- **La única dependencia real que existe hoy es de minerales/livestock — y confirmado por el usuario que es
  INTENCIONAL**, no un accidente de generación de mundo: la escasez de nodos está puesta a propósito para
  fomentar comercio desde el arranque del servidor (ver §1.1 para la magnitud real medida).
- **Los costes de mejora no muerden**: la mejora más cara del catálogo ordinario (450 madera + 200 piedra) son
  9 ticks de producción bruta de madera de un N3. El Palacio (1500 + 1000), 30 ticks.
- **Contradicción de diseño, DESCARTADA por el usuario** (no se corrige): el gate de nivel 2 exige tener
  **Armería + Curtiduría + Fundición construidas**, es decir la transformación sigue siendo el peaje para dejar
  de ser nivel 1 en vez del premio de subir. Se mantiene tal cual está hoy — ver §7, punto 3.

### 1.1 Alcance real de minerales por radio de zona (medido, mapa completo)

Diagnóstico sobre `MAPA_DEFAULT` (2000×2000), 5 seeds, posiciones fundables reales — % con AL MENOS un
mineral/livestock alcanzable dentro del radio de zona de cada nivel (`ZONA_INFLUENCIA.radioMaximoPorNivel`:
60/90/120):

| Seed | A radio 60 (N1) | A radio 90 (N2) | A radio 120 (N3) | Con cobre/livestock a 60 |
|---|---|---|---|---|
| 1 | 33.3% | 51.7% | 65.0% | 33.3% |
| 2 | 46.7% | 65.0% | 78.3% | 33.3% |
| 3 | 1.7% | 3.3% | 8.3% | 0.0% |
| 4 | 33.3% | 43.3% | 55.0% | 6.7% |
| 5 | 40.0% | 60.0% | 75.0% | 28.3% |

**Confirmado con el usuario: este diseño es intencional**, no un bug a corregir con más densidad de nodos ni
con más radio. Consecuencia aceptada: la **mayoría** de asentamientos nivel 1 (53-98% según seed) nacen sin
ningún mineral alcanzable — el comercio no es una mejora opcional del juego, es la única vía real de conseguir
piedra/cobre/estaño/oro/livestock para la mayoría de asentamientos desde el principio. Esto hace que §6
(suministro) no sea una pieza más del rediseño: es la que sostiene a la mayoría del mundo incluso hoy, sin
esperar a cupos ni roles.

### 1.2 El deadlock de arranque de comercio (encontrado y corregido esta sesión)

Instrumentar el punto anterior expuso que el sistema de comercio (Doc 3.2/3.12), la vía que se supone resuelve
la escasez de §1.1, era él mismo inalcanzable para la mayoría de esos asentamientos:

- Mercado costaba 100 madera + **40 piedra**. La reserva de fundación (Doc 1.3) es de solo 20 piedra, y un
  asentamiento sin mineral alcanzable nunca junta más — se queda fijo en 20 para siempre.
- **Cada lado de un trueque necesita SU PROPIA caravana en SU PROPIO origen** para entregar lo pactado
  (`asignarCaravanasATrueque`, `engine/trade.ts`) — no basta con que el lado rico tenga Mercado y caravana.
- Consecuencia: un asentamiento sin piedra no podía construir Mercado (necesita piedra) → no podía construir
  caravana (necesita Mercado) → no podía entregar SU lado de ningún trueque, ni para dar ni para recibir — un
  deadlock total y estructural, no una particularidad de un test.

**Corregido**: Mercado ya no pide piedra en su construcción BASE (`{madera: 100}`), a petición explícita del
usuario — *"quitar el coste de piedra para construir el mercado, pero para mejorarlo se mantiene el coste de
piedra"*. Las mejoras de nivel 2/3 de Mercado (150+100 piedra, 450+200 piedra) no cambian: para cuando el
asentamiento mejora el Mercado, ya tuvo tiempo de conseguir piedra, por extracción propia o por el comercio que
el Mercado nivel 1 acaba de destrabar. Validado de punta a punta, ver §0.1.

## 2. Principio rector (decisión cerrada, reformulado tras corrección del usuario)

**Ningún edificio produce menos por estar en un asentamiento de nivel alto.** Un multiplicador de producción
que castigue el nivel alto es contraintuitivo y queda descartado explícitamente.

**Lo que crece con el nivel es el CONSUMO, no un castigo a la producción**: más habitantes (§3.1), más
ejército, más construcciones y mejoras (§3.2/§3.3), más recetas de transformación activas (§3.4). El nivel
alto no rinde menos — es que necesita mucho más para sostenerse a sí mismo de lo que puede sostener solo.

Formulación operativa:

> La extracción es un **caudal**; la mejora es un **depósito**. Un coste puntual, por grande que sea, cualquier
> asentamiento acaba pagándolo solo: solo retrasa, no crea dependencia. Lo que crea dependencia estructural es
> que el **margen neto** (producción − consumo real de población/mantenimiento) del nivel alto tienda a cero,
> de modo que todo depósito grande (mejoras, edificios de nivel alto) solo pueda llenarse con caudal ajeno.

## 3. Vectores de demanda (el motor de la dependencia)

Ordenados por cuánto muerden.

### 3.1 Techo de población por nivel (pieza central, sin objeciones del consejo)

El nivel pasa a ser lo que **abre el techo de habitantes**, en vez de una simple llave de edificios. Cupo
máximo de población por nivel, por encima del cual la Vivienda deja de dar cupo:

| Nivel | Techo de población (placeholder) | Consumo de trigo resultante |
|---|---|---|
| 1 | ~300 | 30/tick |
| 2 | ~1500 | 150/tick |
| 3 | ~6000 | 600/tick |

Contra un techo agrícola de 150–450 trigo/tick, **un nivel 3 poblado no puede comer sin importar**. De paso,
"nivel alto = más habitantes, más ejército, más construcciones" pasa a ser una ley del sistema y no una
casualidad de partida — y de esto sale también la respuesta a "¿por qué el suministro importado es necesario"
sin imponer un gate arbitrario: es consecuencia directa de que el techo de población sea matemáticamente
insostenible con producción local (ver §7 antiguo punto 8, ya resuelto: NO se gatea el nivel a tener un
trueque activo, es puramente una consecuencia numérica).

### 3.2 Mantenimiento — por población/tamaño real, NO por nº de edificios (revisado)

**Corrección importante sobre la versión anterior de este documento**: se había propuesto sumar coste de
mantenimiento por número de edificios activos. Se retira: el mantenimiento YA escala por distancia a la
capital (`factorDistancia`, Doc 4.5), que es el mecanismo real de "imperio disperso cuesta más" — sumarle
además un coste por nº de edificios castigaría DOBLE al mismo imperio distribuido que el diseño quiere
fomentar (ver §5.1, "imperios distribuidos pero cohesionados"). El mantenimiento escala por **población real
del asentamiento** (ya conectado a §3.1); la distancia sigue siendo el eje que castiga la dispersión, sin
tocar.

**Importante — el mantenimiento NO baja al degradar de nivel** (aclaración del usuario, ver §6.2): la misma
gente sigue comiendo lo mismo con o sin nivel. El nivel deja de ser un multiplicador de coste — es solo un
gate de capacidad (§6.2).

### 3.3 Costes de construcción/mejora crecientes, pagados en MATERIALES PROCESADOS — DESCARTADO

> **DESCARTADO por el usuario** (esta pasada): fuera del alcance de Fase 0.5. Queda el texto original como
> registro de la idea, por si se retoma más adelante, pero no entra a implementación.

Más allá del mantenimiento, mejorar cuesta más en niveles altos. Pero en vez de limitarse a subir las cifras de
madera/piedra, **las mejoras de nivel alto se pagan en materiales procesados** (vigas escuadradas, sillería,
lingote de bronce — el catálogo exacto queda por definir, ver §7 punto #4).

La ciudad **puede fabricarlos ella misma** — es literalmente su fuerte — pero para fabricarlos necesita que le
lleguen mineral, ganado y madera en bruto. Su capacidad de transformación deja de ser solo un producto de
exportación y pasa a ser **el requisito de su propio crecimiento**. Esto convierte la dependencia de aritmética
("tarda más") en categórica ("no puede sin que le suministren").

Freno adicional ya existente en el código, que juega a favor: el almacén arranca en **200 por recurso**
(`ALMACEN.capacidadInicialPorRecurso`) y suma 300 por Almacén construido. Un Palacio de 1500 madera necesita 5
Almacenes solo para poder *guardar* el material — el techo de almacenaje ya obliga a que el material llegue
fluyendo mientras se gasta, en vez de ahorrarse en silencio.

### 3.4 Transformación a escala = consumo de bruto a escala — DESCARTADO

> **DESCARTADO por el usuario** (esta pasada): fuera del alcance de Fase 0.5.

El nivel alto tiene Fundición 2 + Curtiduría 3 + Armería 3, más artesanos y más recetas activas. **Producir más
obliga a consumir más** cobre, estaño, livestock y madera. Es el vector más elegante porque no penaliza nada: el
premio genera la dependencia por sí solo. Aquí sí se sube el throughput del nivel alto.

### 3.5 Ejército — DESCARTADO

> **DESCARTADO por el usuario** (esta pasada): fuera del alcance de Fase 0.5.

El nivel alto es el único que puede reclutar tropas de tier alto (Barracón 3 / Armería 3). Raciones de trigo
(Doc 5.4) + equipo fabricado. Refuerza, no lidera.

### 3.6 Huella urbana contra tierra cultivable — DESCARTADO

> **DESCARTADO por el usuario** (esta pasada): fuera del alcance de Fase 0.5, pese a ser barato de implementar.

Ya existe a medias: hay rejilla de celdas (`REJILLA_ASENTAMIENTO`) y la Granja crece de 2x2 a 6x6 según su nivel
interno. Una ciudad grande literalmente se queda sin sitio donde poner granjas. **No produce menos: es que no le
caben.** Apretar esto es barato porque el sistema ya está construido.

## 4. Roles / vocación de asentamiento — DESCARTADO

> **DESCARTADO por el usuario** (esta pasada): fuera del alcance de Fase 0.5. Se conserva el texto original
> (incluida su subsección 4.1) como registro de la idea.

**El problema que resuelve**: la auto-construcción se dispara por **déficit propio**. Un nivel 1 con 2 Granjas
cubre sus 15 trigo/tick y no construye la tercera jamás. Sin un motivo para producir de más, no hay excedente
que exportar y toda la cadena se queda en el papel.

**Solución**: cada asentamiento tiene un ROL asignado que fija **cuotas de excedente objetivo**. La
auto-construcción trata la cuota como demanda propia: sigue levantando extractores hasta cubrir
*consumo propio + cuota*, en vez de parar al cubrirse a sí mismo.

- **Solo cuota, sin bonus de producción** (decisión cerrada). El rol no multiplica nada — solo cambia el objetivo
  que persigue la auto-construcción. Un bonus por rol reintroduciría "produces más o menos según tu etiqueta",
  que es justo lo que el principio de §2 descarta.
- **Roles condicionados a lo que la zona realmente tiene**: no se puede designar Granero en una estepa sin
  fertilidad, ni Cuenca Minera sin nodos minerales en la zona de influencia.
- Roles propuestos: **Granero** (trigo), **Aserradero** (madera), **Cuenca Minera** (piedra/cobre/estaño/oro
  según los nodos de la zona), **Dehesa** (livestock), y **Ciudad**.

### 4.1 Unificación: el cupo de nivel ES el cupo de Ciudades

En vez de dos conceptos separados (nivel de asentamiento + rol), **promover un asentamiento = cambiarle la
vocación de productor a Ciudad**. Los roles productivos no tienen cupo (puedes tener todos los Graneros que
quieras dentro del cap total de fundación ya existente, Doc 1.7); las Ciudades sí.

## 5. Cupo de asentamientos por nivel (decisión cerrada en su forma, placeholder en cifras)

Cupos **separados por nivel**: un asentamiento ocupa únicamente el cupo de **su nivel actual**. Si una Ciudad de
nivel 2 sube a nivel 3, **libera el cupo de nivel 2** que ocupaba. Confirmado explícitamente por el usuario:
*"me gusta el approach de los asentamientos solo ocupan el spot de su nivel"*.

Curva propuesta, ligada al nivel de Facción (§8, ya no a la fórmula de población vieja):

| Nivel Facción | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
|---|---|---|---|---|---|---|---|---|---|---|
| Total fundables (ya existe) | 1 | 2 | 3 | 3 | 4 | 5 | 5 | 6 | 6 | 7 |
| Máx. nivel 2 | 0 | 1 | 1 | 1 | 2 | 2 | 2 | 3 | 3 | 3 |
| Máx. nivel 3 | 0 | 0 | 0 | 1 | 1 | 1 | 2 | 2 | 2 | 3 |

**Invariante de diseño**: `máxN2 + máxN3 = total − 1` en todos los tramos. Siempre queda al menos un
asentamiento obligatoriamente en nivel 1 — el granero/aserradero que nunca se puede convertir en ciudad.

**Aviso heredado del consejo, ya resuelto**: con la fórmula ANTERIOR de nivel de Facción (población total), este
cupo se autodesactivaba (una sola ciudad grande disparaba el nivel de Facción al máximo). §8 lo corrige de raíz
al desacoplar el nivel de Facción de la población. **Quién consume el cupo** (punto abierto #1 de §7) queda
confirmado como automático — "el primero que llega" en orden del array de asentamientos, sin promoción manual
del Rey por ahora.

### 5.1 Imperios distribuidos, pero cohesionados (aclaración del usuario, resuelve tensión detectada por el consejo)

El consejo señaló una contradicción real: castigar por nº de edificios (§3.2, versión anterior) penalizaba
justo el imperio distribuido que este documento quiere fomentar. El usuario aclaró la intención: **la meta ES
un imperio distribuido, pero cohesionado — no con sus partes dispersas por todo el mapa.**

Esto no necesita mecanismo nuevo: el mantenimiento **ya** escala por distancia a la capital (`factorDistancia`,
Doc 4.5) — ese es exactamente el "impuesto de cohesión". Un Granero a 100 unidades de la capital paga poco
extra; el mismo Granero al otro lado del mapa paga mucho más. §3.2 ya retira la propuesta de coste por nº de
edificios por este motivo — se apoya en el mecanismo que ya existe en vez de duplicarlo.

## 6. Suministro y fallo de suministro

### 6.1 El suministro va por TRUEQUE, o por comercio activo con otra Facción — ambos valen

Aclaración del usuario: la dependencia puede resolverse por **logística interna** (asentamientos propios de la
misma Facción) o por **comercio activo** (trueque con otra Facción) — ambas vías son válidas, no hay preferencia
de diseño por una sobre otra. Esto también resuelve la objeción del consejo de que "si el proveedor y el
cliente son el mismo dueño no hay dependencia real, solo logística" — en este juego un jugador reside en UN
solo asentamiento (Doc 2.1/2.5) y una Facción agrupa a varios jugadores, así que el trueque intra-Facción SÍ
es entre partes distintas incluso sin cruzar de Facción.

Se usa el canal ya implementado (Doc 3.2/3.12): trueque, con comisión reducida intra-Facción (3% frente a 8%
externa, `COMISION.tasaMismaFaccion`), con caravanas propias reales que viajan por el mapa. Validado de punta a
punta esta sesión tras el fix de §1.2 — ver §0.1.

**Sobre "no hay precio" (aclaración pedida por el usuario)**: el trueque (Doc 3.2) es un intercambio de
cantidades FIJAS pactadas una vez, sin tipo de cambio — a diferencia del Mercado (Doc 3.3), que sí tiene precio
dinámico por escasez (Doc 3.4). El punto del consejo era que sin precio no hay señal de escasez ni forma de que
el lado proveedor capture más valor cuando lo que da escasea de verdad. Sigue siendo una simplificación de Fase
0 ya aceptada en el propio Doc 3.2 del proyecto ("Plan: pasar a asignación/carga manual en Fase 1+") — no
bloquea el diseño de este documento, queda anotado como mejora futura posible, no como requisito.

**Pendiente de dimensionar** (sin resolver todavía): una Ciudad que depende de varios proveedores necesita
flota para varias rutas permanentes. El cupo de caravanas viene del nivel de Mercado + la política "Ampliación
de Flota" (Doc 3.12) — falta verificar con cifras reales de la curva de cupos si alcanza.

**Nota de implementación, no de diseño**: el NPC de simulación (§0.1) tuvo que asignar Tesorero y usar
`reservaManual.madera` (mecanismo YA existente para un jugador humano, Doc 4.5) para poder construir su propia
caravana sin que la auto-construcción del asentamiento le ganara la madera antes. Esto no es un problema de
diseño — el mecanismo para resolverlo ya existe y funciona — pero puede valer la pena una nota en la UI/tutorial
de que un Tesorero activo con esa reserva calibrada ayuda a arrancar comercio antes.

### 6.2 Cortar el suministro CONGELA CAPACIDAD, no reduce mantenimiento ni destruye población (revisado tras corrección del usuario)

**Corrección central del usuario sobre la versión anterior de este documento**: *"al bajar de nivel no baja el
mantenimiento, al seguir viviendo la misma cantidad de personas siguen comiendo lo mismo ergo el consumo es el
mismo, solo que no podrá construir edificios de nivel 3, ni mejoras, ni reclutar de nivel 3 así tenga los
edificios, hasta que logre subir."*

Esto además resuelve de raíz el problema de "yo-yo" que había señalado el consejo (subir/bajar de nivel en
bucle porque el gate se reevalúa cada tick contra población fresca). Se separan dos conceptos que antes eran
uno solo (`asentamiento.nivel`):

- **`nivelAlcanzado`** — histórico, MONÓTONO, nunca baja. De aquí sale el techo de población (§3.1): la gente
  ya asentada nunca se purga por una crisis de suministro temporal. Solo sube al cumplir gates de verdad
  (población + infraestructura, con cupo disponible, ver punto abierto #1).
- **`nivelActual`** — operativo, puede subir y bajar según la salud sostenida del mantenimiento. De aquí sale
  qué se puede CONSTRUIR/MEJORAR/RECLUTAR ahora mismo. Baja un escalón cuando el medidor de mantenimiento toca
  0 (mismo umbral de siempre, Doc 4.5); solo vuelve a subir tras mantenimiento sano varios ticks seguidos (no
  cada tick) — así no oscila con un solo bache.

**Nivel 3 → nivel 2 → nivel 1 → ruinas**, aplicado a `nivelActual`: cada vez que el medidor toca 0 baja un
escalón y se reinicia; solo un asentamiento que ya está en `nivelActual` 1 y vuelve a tocar 0 cae en ruinas.

Efectos de la degradación de `nivelActual` (todos sobre CAPACIDAD, ninguno sobre población/consumo):
- Libera el cupo de `nivelActual` que ocupaba (§5), disponible de inmediato para otro asentamiento de la
  Facción — sin cooldown, tal como está implementado (cooldown de liberación **descartado por el usuario**,
  ver §7 punto 7). El techo de población, en cambio, sigue el de `nivelAlcanzado` — **nunca se purga gente**.
- Los edificios de nivel alto YA construidos **siguen produciendo con normalidad** — no se apagan. Solo se
  congela la capacidad de construir/mejorar NUEVOS edificios de ese nivel, y de reclutar tropas de ese tier,
  hasta recuperar `nivelActual`.
- El mantenimiento NO cambia — sigue calculado sobre población/distancia real (§3.2), ajeno a `nivelActual`.

Esto convierte el asedio económico y el corte de rutas en una estrategia con sentido (le arrancas capacidad a
una ciudad rival y liberas su cupo, sin matar a nadie) y convierte un fracaso logístico en algo recuperable —
la ciudad sigue viva y produciendo, solo estancada — en vez de una espiral de purga o un yo-yo de nivel.

## 7. Puntos abiertos (no decididos — entran a evaluación)

1. ~~¿Quién decide qué asentamiento consume un cupo de `nivelAlcanzado`?~~ — **RESUELTO, confirmado por el
   usuario**: automático de momento — "el primero que llega" en orden del array de asentamientos, dentro del
   mismo tick (`engine/simulation.ts`), sin promoción manual del Rey. Se mantiene tal como se implementó
   (`avanzarNivelAsentamiento` en `engine/mantenimiento.ts` ya acepta el callback `tieneCupoParaNivel`, así
   que si más adelante se quiere control manual hay un punto de extensión listo, pero no es un punto abierto
   ahora mismo).
2. ~~¿Los asentamientos conquistados/anexados cuentan para el cupo?~~ — **RESUELTO, confirmado por el
   usuario**: NO, y es intencional — mismo criterio que `CAP_FUNDACION_POR_NIVEL` (Doc 1.7), `iniciarAsedio`
   (`engine/combate.ts`) reasigna `faccionId` sin volver a chequear cupo. Un conquistador puede acabar con más
   asentamientos de nivel alto que su cupo permitiría fundar por su cuenta — **la conquista es a propósito una
   vía para evadir el cupo, como incentivo a la guerra** en vez de expandirse solo por Caravana de Fundación.
3. ~~¿Se invierten los gates de nivel?~~ — **DESCARTADO por el usuario** (esta pasada). El nivel 2 sigue
   exigiendo Armería + Curtiduría + Fundición como hoy; no se toca el disparador de Artesanos ni el orden
   premio/peaje de la transformación.
4. ~~Catálogo de materiales procesados de construcción~~ (§3.3) — **DESCARTADO por el usuario** junto con
   §3.3: ningún coste de mejora se paga en materiales procesados.
5. **Todas las cifras** (techos de población, cuotas por rol, curva de cupos, curva de XP de Facción §8,
   `poblacionReferencia`, `ticksSanosParaRecuperarNivel`) son placeholder sin calibrar por simulación, igual
   que el resto del proyecto — ahora ya viven en `constants.ts`, listas para ajustar desde el panel de
   balance sin tocar código.
6. ~~Interacción con la Caravana de Fundación (Doc 1.8) — deadlock de expansión temprana~~ — **RESUELTO,
   no es un deadlock**: confirmado por el usuario que, al existir muchas fuentes de XP de Facción (combate,
   construcción, conquista, defensa/ataque de caravana), llegar a nivel de Facción 2 no es difícil en la
   práctica. No se aplica excepción ni se cambia la curva de cupo por este motivo (ver también §0.1).
7. ~~¿Puede degradarse un asentamiento voluntariamente para liberar cupo?~~ — **DESCARTADO por el usuario**
   (esta pasada), junto con el cooldown de liberación de cupo (ver §6.2, ya actualizado): la liberación de
   cupo por degradación involuntaria queda tal como está implementada, inmediata y sin cooldown.
8. ~~¿El suministro importado debe ser un gate explícito de nivel?~~ — RESUELTO: no, debe ser consecuencia
   natural del techo de población (§3.1), no una condición arbitraria tipo "ten un trueque activo". Ver §3.1.
   **Implementado**: §3.1 (techo de población) ya está en el código.
9. ~~¿Existen facciones NPC que deban operar cadenas de suministro por sí solas?~~ — RESUELTO por el usuario:
   no existen facciones NPC en este diseño. Se descarta como riesgo.

## 8. Nivel de Facción por experiencia (NUEVO — prerrequisito de §5, no un tema aparte)

**Por qué entra en este documento y no queda para después**: §5 (cupo por nivel de Facción) no tiene sentido
sin esto. La fórmula ANTERIOR de nivel de Facción (`NIVEL_FACCION`, `engine/faccion.ts`
`calcularNivelFaccion`) suma población total de la Facción — el consejo detectó que esto hace que el cupo se
autodesactive solo: una única Ciudad de 6000 habitantes ya dispara el nivel de Facción al máximo (10, clampado)
por sí sola, y further peor, cuantos más asentamientos de nivel 1 se funden, más rápido sube el nivel de
Facción (`puntosPorAsentamiento`) — fundar graneros regala plazas de Ciudad. El cupo, tal como estaba, no
limitaba nada.

**Rediseño confirmado por el usuario**: el nivel de Facción sube por EXPERIENCIA acumulada de la actividad de
sus jugadores/asentamientos, no por población ni por conteo de asentamientos:

- Combatir (cualquier jugador de la Facción, PvP o PvE) otorga experiencia.
- Construir un edificio en cualquier asentamiento de la Facción otorga experiencia (poca).
- Conquistar un asentamiento por la fuerza otorga experiencia.
- Defender una caravana propia otorga experiencia.
- Atacar/interceptar una caravana rival otorga experiencia.
- **Monótono**: el nivel de Facción NUNCA baja, solo sube.
- Rango 1 a 10 (igual que hoy, `NIVEL_FACCION.nivelMaximo`), pero **cada nivel cuesta bastante más XP que el
  anterior** (curva creciente, no lineal — cifra exacta sin definir, punto abierto #5).

Esto desacopla por completo el cupo de §5 de la población interna de la Facción: subir de nivel exige jugar
activamente (combate, construcción, guerra, comercio defendido/atacado), no simplemente fundar y esperar a que
la población crezca sola.

**Punto abierto derivado, no cerrado todavía**: cantidades exactas de XP por evento y la curva de coste por
nivel (mismo criterio que el resto de cifras del documento — placeholder pendiente de calibración por
simulación).
