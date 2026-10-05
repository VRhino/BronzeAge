# 6. Sistema de Tecnología y Aedas

## 6.1 La tecnología es de la Facción

La tecnología la **adopta la Facción** y vale en todos sus asentamientos. Hay un catálogo cerrado de tecnologías, cada
una de una **Era** (6.2), y cada una desbloquea tropas, edificios, niveles internos o recetas.

**Las cuatro puertas.** Cualquier asentamiento, sea de la cultura que sea, recluta una tropa si cumple a la vez:

```text
Era permitida por el servidor
  + tecnología adoptada por la Facción
  + edificio y nivel interno en el asentamiento
  + población, equipo, animales y recursos disponibles
  = reclutamiento permitido
```

Lo mismo vale para construir un edificio, mejorar un nivel interno o producir una receta que piden tecnología
(Doc 4.2.1). **El roster es universal**: la cultura del asentamiento no da ni quita tropas ni tecnologías, solo
aspecto. La tecnología dice qué existe; el equipo dice con qué se hace (una tropa puede depender de una tecnología
y su arma de otra).

Una tecnología pasa por cuatro estados para cada Facción: **oculta**, **revelada** (un Aeda se la ha mostrado: sabe que
existe, qué le falta y quién la desbloqueó, pero aún no puede adoptarla), **aparecida** (cumplidas sus condiciones o
comprada a un Aeda: puede adoptarla) y **adoptada**.

**Cuatro vías para conseguirla:**
1. **Desarrollo propio**: cumplir su logro del servidor y el hito de la Facción (6.3) y pagarla (6.5).
2. **Comercio**: pagar a una Facción que ya la tiene adoptada.
3. **Aedas**: pagar oro a un Aeda itinerante que la conoce (6.7), o completar la épica de un Aeda residente (6.7).
4. **Conquista**: al conquistar un asentamiento que ya reclutaba con una tecnología, esa tecnología le aparece al
   conquistador.

Comercio, Aedas (venta y épica) y conquista **se saltan el hito de la Facción, nunca el logro del servidor**. En todas las vías la
adopción se paga igual (6.5).

## 6.2 Eras

La season (12 meses) se divide en cinco Eras, de ~1300 a 323 a. C. (hasta Alejandro Magno):

| Era | Id | Nombre | Semanas | Niveles de asentamiento |
|---|---|---|---|---|
| I | `reinos_palaciales` | Reinos palaciales | 0-5 | 1 y 2; pocos en el 3 |
| II | `crisis_adaptacion` | Crisis y adaptación | 5-11 | 1, 2 y 3; meseta en el 3 |
| III | `polis_imperios` | Polis e imperios | 11-18 | concentrada en el 3; primeras cosas de nivel 4 |
| IV | `profesionalizacion_imperial` | Profesionalización | 18-26 | cosas de nivel 3; meseta en el 4 |
| V | `hegemonia_macedonica` | Hegemonía macedónica | 26-52 | concentrada en 3 y 4; primeras cosas de nivel 5 |

- **El servidor arranca en la Era I**, con dos tecnologías adoptadas por todas las Facciones: `leva_comunal` y
  `hostigamiento_tribal`.
- Cada Era tiene sus logros del servidor (6.3). **La Era siguiente empieza cuando se cumplen TODOS los logros de la
  Era, o cuando se agota su plazo**, lo que pase antes: el plazo es un techo, no un suelo.
- Si la Era siguiente empieza por tiempo, los logros pendientes de la anterior siguen abiertos y se pueden cumplir.
- **Ninguna tecnología de una Era se puede adoptar antes de que esa Era empiece.**
- **El techo de nivel de asentamiento de cada Era es derivado**: no hay una regla que prohíba subir, sino que la
  subida pide algo que solo existe a partir de cierta Era (Doc 4.5). La tabla de niveles de arriba es la guía con la
  que se reparte el contenido de cada Era.
- La Maravilla exige que el servidor haya llegado a la Era V (Doc 4.2.1). La season termina por tiempo aunque nadie
  la haya construido.

## 6.3 Logro del servidor e hito de la Facción

Para que una tecnología aparezca por desarrollo propio hacen falta dos cosas:

- **El logro del servidor**: algo que ha pasado en todo el mundo entre todas las Facciones, las de bots incluidas (por
  ejemplo, "X de estaño extraído en el mundo"). **Se fija para siempre** en cuanto se cumple.
  **Es público**: lo canta un Aeda en la crónica, **sin decir qué tecnología desbloquea**.
- **El hito de la Facción**: condiciones de la propia Facción (edificios, niveles, otras tecnologías adoptadas…).

Cuando una Facción cumple el hito de una tecnología cuyo logro ya existe y cuya Era ya empezó, la tecnología **le
aparece**. La primera Facción en conseguirlo queda en la crónica como su descubridora.

Las cifras X de los logros se ajustan para que cada logro se cumpla en una semana concreta de su Era: los de cada
Era se reparten por su tramo en el orden de adopción, y el último cae una semana antes del plazo.

## 6.4 Visibilidad

- **Toda tecnología está oculta** hasta que se cumplen sus condiciones de aparición.
- Cuando una Facción la desbloquea, le aparece solo a ella; **para el resto del servidor sigue oculta**.
- Los **Aedas itinerantes** esparcen el conocimiento (6.7): al llegar a un asentamiento, le **revelan** las
  tecnologías que conocen y la Facción aún no tiene aparecidas: cuáles son, qué requisitos de hito le faltan y quién
  las desbloqueó.
- **La ventaja del primero** es ese tiempo: los Aedas conocen una tecnología **24 horas** después de que alguien la
  desbloquea (`AEDAS.retrasoConocimientoHoras`, placeholder), no antes. No hay descuento: todos pagan la misma tarifa.

## 6.5 Adopción

**La adopta el Rey, estando en la capital, y la paga el almacén de la capital.** La adopción es instantánea. La capital
la designa el Rey (Doc 2.2); mientras no designe ninguna, es el asentamiento vivo más antiguo de la Facción.
Las Facciones de bots siguen la misma regla (5.15.6).

| Era de la tecnología | Oro | Bien de la Era |
|---|---|---|
| I | 100 | 200 madera |
| II | 300 | 30 lingotes de bronce |
| III | 600 | 30 lingotes de hierro |
| IV | 1.000 | 50 lingotes de hierro |
| V | 1.500 | 80 lingotes de hierro |

## 6.5b Tecnología propia de los campamentos de mercenarios (2026-10-02)

Un campamento de mercenarios (Doc 1.9b) no adopta nada: **desbloquea cada tecnología el último**, al instante y gratis, cuando la tiene el **50 %** de las Facciones humanas vivas (las que no gobierna la IA y conservan algún asentamiento) **o** han pasado **72 horas** desde que la primera Facción la desbloqueó, lo que llegue antes. Las de arranque ya están. Así se recluta allí lo que una Facción no alcanza por edificio o materiales (Doc 5.8), y la ventaja del primero dura unos días sin ser eterna. Una tecnología que nadie ha desbloqueado no está en el campamento. Cifras en `MERCENARIOS.tecnologia` (placeholder).

## 6.6 Catálogo

Las X de la Era I se fijaron con el batch en la semana objetivo de su logro (6.3) y caen donde toca. Las de las Eras II y III son provisionales: sus contadores dependen de qué tecnologías adopta cada Facción, y con jugadores NPC se adoptan pocas, así que cada cambio mueve el mundo entero. Siguen sin medida `carpinteria_militar`, `falange_hoplita`, `arqueria_especializada`, `bronce_laminado` y `trabajos_asedio`, cuyo contador no se mueve. "Edificio" en un hito = activo en cualquier asentamiento de la Facción. Las
tropas de cada tecnología, con su edificio, equipo y escalón, en Doc 5.8. **Una tecnología también puede dar un
bonus de producción** en vez de abrir contenido (`bonusProduccion`: un recurso y un factor sobre lo que sacan sus
extractores): hoy solo `canteria`, que se pensó para la piedra del nivel 3 (Doc 4.5).

**Era I — Reinos palaciales**

| Tecnología | Desbloquea | Logro del servidor | Hito de la Facción |
|---|---|---|---|
| `leva_comunal` | milicia de lanceros, leñadores, granjeros | — (de arranque) | — |
| `hostigamiento_tribal` | honderos, escaramuzadores con jabalina | — (de arranque) | — |
| `metalurgia_cobre` | Arma de Cobre (Armería 1) | X de cobre extraído | Fundición activa |
| `aleacion_bronce` | Lingote de Estaño y de Bronce (Fundición 2), Arma de Bronce (Armería 2); espadachines de bronce, hacheros armados | X de estaño extraído en el mundo (Uluburun) | Fundición 2 + estaño en el almacén de la capital |
| `escudos_ligeros` | lanceros con escudo de mimbre | X campamentos de bandidos destruidos | Barracón activo |
| `armamento_palacial` | espadachines de cobre, hacheros ligeros | X asentamientos en nivel 2 (lineal B) | Armería activa + `metalurgia_cobre` adoptada |
| `arqueria_palacial` | arqueros | X batallas libradas | Galería de tiro activa |
| `cria_caballar` | Caballerizas; exploradores a caballo | X animales comprados (caravanas incluidas) | Corral activo |
| `carros_guerra` | Carro de Guerra (Carpintería 2); carros de guerra | X batallas a campo abierto (Qadesh) | `cria_caballar` + Caballerizas 2 + Carpintería 2 |
| `canteria` | las Canteras de la Facción sacan **1,5×** la piedra (en todos sus asentamientos) | X de piedra extraída en el mundo | Cantera activa |

**Era II — Crisis y adaptación**

| Tecnología | Desbloquea | Logro del servidor | Hito de la Facción |
|---|---|---|---|
| `bronce_calidad_militar` | Arma de Bronce de Calidad y Armadura de Bronce (Armería 3); Shardana | X piezas de equipo de bronce fabricadas (Dendra) | Armería 3 |
| `forja_hierro_temprana` | Mina de hierro, Lingote de Hierro (Fundición 2), Arma de Hierro (Armería 2); guerreros filisteos | X caravanas destruidas o capturadas (colapso del estaño) | Fundición 2 + yacimiento de hierro en su territorio |
| `panoplia_bronce` | lanceros pesados micénicos, Hequetai | X escuadrones reclutados (Vaso de los Guerreros) | `bronce_calidad_militar` + Barracón 3 |
| `disciplina_formacion` | formaciones cerradas (táctico, en la batalla) | X asedios resistidos en combate (Batalla del Delta); no cuentan los rebotes por ocupación | Barracón 2 |
| `arco_compuesto` | arqueros con arco compuesto | X arqueros reclutados | Carpintería 2 + Galería 3 |
| `equitacion_militar` | jinetes asirios | X soldados de caballería reclutados, **cada soldado de carro de guerra vale por 5 de jinete** (Assurnasirpal) | Caballerizas activas |
| `carpinteria_militar` | escalas y defensas de campaña (equipo) | primera conquista de una plaza con muralla completa (Dapur) | Carpintería 2 |

**Era III — Polis e imperios**

| Tecnología | Desbloquea | Logro del servidor | Hito de la Facción |
|---|---|---|---|
| `instituciones_civicas` | Sala del Consejo (requisito del nivel 4) | X asentamientos en nivel 3 (nacimiento de la polis) | capital en nivel 3 con Mercado activo |
| `ciudadania_militar` | hoplitas ciudadanos | X asedios resistidos con los residentes dentro (reforma hoplita) | `instituciones_civicas` + Barracón 2 |
| `falange_hoplita` | Espartiatas | X batallas libradas usando hoplitas | `ciudadania_militar` + Barracón 3 |
| `pantalla_escaramuzadores` | peltastas, honderos rodios | X escaramuzadores con jabalina reclutados | Galería 2 + Armería 2 |
| `arqueria_especializada` | arqueros escitas | X arqueros con arco compuesto reclutados | Galería 3 |
| `forja_hierro_estandarizada` | Fundición 3, Arma de Hierro de Calidad (Armería 3) | X de hierro extraído (acero templado) | Fundición 2 + Mina de hierro activa |
| `bronce_laminado` | Armadura de Bronce de Calidad (Armería 3) | X armaduras de bronce fabricadas (coraza musculada) | Armería 3 |
| `caballeria_organizada` | jinetes escitas, caballería asiria | X jinetes asirios reclutados (Tiglat-Pileser III) | Caballerizas 2 |
| `trabajos_asedio` | equipo de asedio de Carpintería 2 | X asedios contra plazas con muralla completa (Laquis) | Carpintería 2 |

Las escalas, las defensas de campaña y el equipo de asedio de Carpintería 2 todavía no tienen efecto en combate: la
tecnología se adopta y su equipo llega con la mecánica de asedio.

## 6.7 Aedas

Los Aedas (o Poetas, mismo rol; Glosario) son NPCs que dan acceso a la tecnología y narran el lore del servidor. No
atacan ni se les ataca, no combaten y no se capturan ni se retienen. Son de dos clases.

### Itinerantes

- **Cuántos**: uno por cada 3 Facciones vivas, con un mínimo de 3 (`AEDAS.itinerantes`, placeholder).
- **Cómo se mueven**: recorren los asentamientos por la **red de caminos** (el grafo de navegación, Doc 1.6): sobre
  camino van más rápido. Eligen el asentamiento más cercano que no han pisado hace poco y se detienen unas horas en
  cada uno (`AEDAS.itinerantes.estanciaMinutos`) antes de seguir. Una plaza que **le ha cerrado la puerta a los
  neutrales a propósito** (el exilio, Doc 2.8) los deja fuera; el cierre por defecto no cuenta.
- **Se ven** como cualquier columna ajena, solo dentro del radio de visión de la Facción (Doc 5.12.7); no son
  atacables ni retenibles.
- **Difunden** (6.4): mientras están detenidos en un asentamiento le revelan a su Facción lo que conocen y aún no tiene
  aparecido; lo que pasa a conocerse durante su estancia se revela en cuanto el retraso se cumple.
- **Venden**: mientras están en un asentamiento, el Rey o el Gobernador de esa plaza puede comprarles una tecnología
  que conocen, de las **Eras I-III**, con el **logro del servidor ya cumplido** y la Era ya abierta. Cuesta **2 × el oro
  de la tarifa de adopción de su Era** (6.5; 200 / 600 / 1.200), que sale del almacén de esa plaza. La Facción se salta el
  hito y la tecnología le **aparece**; la adopción se paga igual, por el Rey en la capital. Las Eras IV y V solo llegan
  por hito propio, comercio, conquista o épica de un residente.
- **Cantan** los logros del servidor (6.3) y los descubrimientos (crónica, abajo): hechos públicos, sin decir qué
  tecnología abre cada logro.

### Residentes

- **Dónde**: en un asentamiento con **Palacio** y **Nobleza**; si pierde toda su nobleza, el Aeda se va. Con el Palacio
  desde el nivel 2 (Doc 4.2.1), pueden vivir allí desde ese nivel.
- **Cupo por asentamiento**: nivel 2, 1; nivel 3, 2; nivel 4, 3; nivel 5, 4 (`AEDAS.cupoResidentes`). Las políticas
  podrán sumar uno (Doc 4.4).
- **Llegada**: con cupo libre, llega uno cada `AEDAS.llegadaHoras` (placeholder). Una Facción con reputación de **−50 o
  menos** (Doc 2.7) los atrae **tres veces más despacio**.
- **Épica**: un residente desbloquea tecnología con una **épica por capítulos** que sustituye al **hito** de la Facción
  —nunca al logro del servidor— para cualquier tecnología con logro cumplido y Era abierta. Eligen la tecnología al
  empezar la épica; no cuesta oro (la adopción se paga igual, 6.5). Cada capítulo avanza con **eventos inspiradores**
  (una batalla ganada, una conquista, un asedio resistido, un edificio de culto levantado…): hechos relevantes, **no
  acciones repetibles**: cada hecho cuenta una vez, como máximo uno por batalla y uno por capítulo cada
  `AEDAS.epica.enfriamientoHoras`, para que no se pueda farmear.
- **Qué dan**: felicidad, crecimiento de nobleza y prestigio (sin efecto mecánico, como los títulos, Doc 2.9), además de
  la épica.

### Lore y crónica

Los Aedas narran los hechos públicos del servidor: logros del servidor, descubrimientos (el nombre de la primera
Facción en desbloquear una tecnología, **cuando el retraso de 24 horas ya ha pasado**), comienzo de Era, cambios de
títulos de prestigio (Doc 2.9), asentamientos que caen y declaraciones de guerra y paz. Todo queda en una **crónica**
persistente, consultable en una interfaz propia y emitida además como evento en el juego. **No afecta al balance.**
