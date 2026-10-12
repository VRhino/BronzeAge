# Guerra logística — definición

> **Estado (2026-10-11): en diseño, cero código, canon sin tocar.** El canon (`Docs/Game/`) se reescribe cuando se
> implementa (decisión del usuario, 2026-10-11); hasta entonces las reglas viven aquí. La lista de canon a reescribir
> está en §6. Entrada de pendientes: `Docs/Mecanicas a desarrollar.md` §44.
>
> - **Diseño cerrado entero (2026-10-11):** §1 guerra de caravanas, §2 pueblos extractores, §3 obras de mapa y §4
>   avanzadas (atalaya → fuerte → ciudadela), que incluye el recorte de la visión de las plazas.
> - Lo siguiente es detallar el plan (§7) y, al implementar, reescribir el canon (§6).
> - **Aparte:** la calibración de distancias y velocidades va en su propia sesión. Los números del ciclo del carro
>   (§2.3) dependen de ella.

## 0. Razón de ser

En los playtests el mundo no da razones para la guerra, que es la finalidad del juego. Y el nivel 2 entra «sin pena ni
gloria»: desbloquea edificios (Barracón, Galería, Palacio, Taberna, empalizada…), pero nada que se vea en el mapa ni
ningún verbo nuevo. Además es la meseta larga de la partida (Era I medida: nivel 2 a los 5,8 días, nadie llega a 3).

La respuesta es **logística física**. La extracción mineral se sale de la ciudad y tiene que acarrearse. El transporte
se puede atacar en todas partes, y la ventaja del defensor depende de dónde ocurra. El nivel 2 desbloquea piezas que
están **en el mapa** y que se levantan llevando los materiales hasta allí.

## 1. Guerra de caravanas (CERRADA)

### 1.1 Se acaba la inmunidad

Hoy, dentro de una zona de influencia que no es del atacante, una caravana no se puede atacar (Doc 1.6, 3.10). **Esa
regla desaparece**: la inmunidad le quitaba a la guerra su ampliación natural, que es la guerra por las caravanas.
Ninguna zona protege del todo. Lo que cambia según la zona es el **tipo de batalla**.

### 1.2 Dos tipos, según dónde está la caravana respecto a su DUEÑO

| Dónde está la caravana | Tipo |
|---|---|
| Zona de influencia **del dueño** o de un **aliado del dueño** | **B — Aguante** |
| Zona de un enemigo, de un aliado de un enemigo, de un neutral, o **fuera de toda zona** | **A — Disputa de carros** |

- **Aliado** usa el mismo predicado que para unirse a una batalla (Doc 5.15.1b): alianza y vasallaje (vasallo y señor
  son aliados).
- Se mira la posición de la caravana **en el momento del ataque**, contra los polígonos de zona (las zonas no se
  solapan: frontera dura, Doc 1.2).
- **A — Disputa** (la que ya estaba definida y pedida en la CQ-011): los carros cambian de manos dentro de la partida y
  el atacante se lleva el carro que logre sacar hasta cierto punto del mapa. El dentro lo decide Conquest.
- **B — Aguante** (nueva): las caravanas están protegidas en la parte trasera del mapa y el defensor tiene que aguantar
  sin rendirse hasta que se agote el tiempo. Si se agota, gana el defensor, como en cualquier batalla (Doc 5.15.1).

### 1.3 Toda agresión es guerra

- Atacar una caravana de otra Facción **exige estar en guerra con su dueño, en cualquier sitio**: campo abierto, zona
  propia incluida. Así el paso forzoso por una ciudad ajena (peaje del 2 %, Doc 1.6) no se convierte en una trampa.
- **Si no hay guerra, el ataque la declara.** Puede hacerlo **cualquier héroe**, no solo el Rey o el Embajador (2.4.1),
  y la guerra se arrastra por vasallaje como siempre.
- **El cliente avisa antes** de lo que significa: contra quién queda la Facción en guerra y quién más entra por
  arrastre. El jugador puede arrepentirse.
- **El servidor no se fía del aviso**: `atacar` sin una confirmación explícita (p. ej. `declaraGuerra: true`) se
  rechaza con un `detalleError` que enumera esas consecuencias. Un cliente viejo o un bot no declaran guerras sin
  querer.
- **La regla es general: toda agresión entre Facciones exige guerra o la declara**: atacar una caravana, saquear un
  pueblo (§2.4), atacar una avanzada (§4), atacar una columna o asediar una plaza. Sustituye al «atacar o asediar
  sigue sin exigir guerra declarada» de hoy (2.4.1).
- **Excepción: unirse a un asedio que abrió otro** como atacante, siendo neutral y admitido por el Rey de la Facción
  atacante (Doc 5.15.1b), **no declara la guerra**.

### 1.4 Consecuencias

**Los carros no cambian de manos en el mundo: solo la carga.** El atacante se queda con lo que le **quepa en los carros
de sus columnas**. El resto de lo perdido se destruye.

- **A — Disputa.** Conquest responde **qué carros se perdieron**. Nosotros calculamos la carga perdida, que es la de
  esos carros, y se destruyen los carros con su animal. La caravana sigue con los que le queden; si los pierde todos,
  se elimina (3.13.6: libera cupo y aplica el cooldown).
- **B — Aguante.** Si gana el atacante, el dueño pierde el **70 %** de la carga, recurso a recurso, y el **30 % se salva
  siempre**. Conquest también informa de los carros perdidos, que se destruyen con su animal. El 30 % llega a su
  destino en los carros que queden o, si no queda ninguno, con los carreteros. Si gana el defensor, no se pierde nada.
- Desaparece el «captura exactamente el 50 %» de hoy (3.10).
- **Bot contra bot** (resolución con números, Doc 5.2.5):
  - B usa una `ventajaDefensor` mayor que el ×1,05 general (placeholder ×1,25).
  - A, si gana el atacante, le hace perder la mitad de los carros, redondeando hacia arriba (placeholder). En B pierde
    el 70 % de la carga y la mitad de los carros (placeholder).
- **Sin cambios**: la defensa base de los carreteros (13 de milicia, un jugador solo no roba caravanas), la escolta sin
  héroe (3.13.4), la escolta por ejército (5.13.3) y el tope de 5 héroes por bando.

### 1.5 Bandidos: otro evento PvE que ya existe

Los campamentos de bandidos **no se mueven**. Solo atacan las caravanas que pasan cerca de donde están apostados, y se
resuelve con números (Doc 1.9). Es un evento PvE ya implementado que **no figura en la tabla de tipos de batalla**
(5.15.1b, `Batalla_Este_Lado` §1.3), donde el único PvE es atacar el campamento. Se anota en el canon al implementar
(§6). Los bandidos **siguen sin actuar dentro de ninguna zona**: quitar la inmunidad es una regla entre Facciones, no
contra el PvE.

### 1.6 Contrato

- **Petición**: **el servidor decide el tipo** y se lo dice a Conquest. Campo nuevo en
  `contextoEstrategico.caravana { caravanaId, punto, modalidad: 'disputa' | 'aguante' }`, con los carros identificados
  (id y carga de cada uno).
- **Respuesta**: quién ganó (ya existe) y `carrosPerdidos: carroId[]`, en A y en B.
- Va al modelo de datos (Doc 01), al contrato (Doc 02), a `src/contratos/v1/` y como ampliación de la CQ-011. **No se
  envía a Conquest sin permiso del usuario.**

## 2. Pueblos extractores (CERRADA)

### 2.1 Qué extracción pasa a pueblo

- **Minerales** (cobre, estaño, hierro y oro) y **toda Cantera después de la primera** solo se explotan con un pueblo
  extractor.
- **Granja, Leñera y Corral** siguen dentro de la ciudad, como hoy.
- **La primera Cantera** sigue siendo directa: sin piedra no hay obra de ascenso al nivel 2 (400 de piedra). Así el
  requisito de nivel 2 (3 de los 7 tipos de extracción, Doc 4.5) no cambia.
- Consecuencia: en nivel 1 ya no hay minas. El oro de esa etapa sale de la recaudación por población (Doc 3.1).

### 2.2 Qué es un pueblo

- Es el extractor de mapa de siempre (`ambito: 'mapa'`, plantado sobre su nodo) **más un almacén local, un carro con
  buey y una guarnición de mineros** (§2.4).
- **No es un asentamiento**: no tiene zona propia, nivel, cargos, residencia ni medidor de mantenimiento. Cuenta como
  **edificio activo de su plaza madre**, que paga su mantenimiento (Doc 4.5).
- **Requisitos**: plaza madre de **nivel 2 o más** y el nodo dentro de la zona de la plaza madre.
- **Cupo por nivel de la madre**: **2 / 3 / 4 / 5** pueblos en los niveles 2 a 5 (placeholder).
- **Se construye como obra de mapa** (§3). El coste incluye el carro y el buey.
- **Trabajadores**: pesants de la plaza madre. El pueblo no tiene población propia ni crece.
- **Almacén local** (capacidad placeholder; el ejemplo del usuario usa 500). Lo extraído se queda ahí. Con el almacén
  lleno, la extracción para.
- **Nodo agotado**: el pueblo espera a que se regenere (`REGENERACION_NODOS`) **sin mudarse**. Los pueblos no entran
  en el reemplazo automático ni en `EXTRACCION_MAXIMOS` (Doc 4.2).
- **Destruirlo a mano** (para liberar cupo o mover la extracción): se puede, y **no se recupera nada**.
- **Visibilidad**: se recuerda como una ciudad, porque no se mueve. Si se destruye, la memoria se corrige cuando se
  vuelve a ver. No aporta visión propia: está dentro de la zona, que la plaza ya ve.
- **Escala**: como mucho **1 unidad de mapa** (40 locales). Vale para todo elemento nuevo del mapa.

### 2.3 El carro de acarreo

- Es una caravana de tipo propio (`acarreo`) ligada al pueblo: **1 carro básico con buey** (500 de capacidad, velocidad
  16). Hace una ruta fija entre el pueblo y la plaza madre.
- **Solo sale con el carro lleno.** Así cada viaje es un botín grande con un ritmo previsible, que se puede vigilar y
  defender. Vuelca en el almacén de la madre y vuelve vacío.
- **No cuenta contra los cupos del Mercado** (caravanas 2/4/6, carros 3/9/18) ni exige Mercado.
- **Se puede escoltar**:
  - con escolta sin héroe de los residentes de la madre: el cupo es el del Mercado de la madre si lo tiene (100/200/300)
    y **100** si no lo tiene;
  - con escolta por ejército.
- **Casi siempre lucha en B**, porque viaja dentro de la zona del dueño. Si el polígono de la zona está recortado por una
  frontera y la ruta sale de ella, ese tramo es A.
- **Si se destruye**: se compra otro en la plaza madre, **solo si el pueblo no tiene ninguno**, al coste de carro más
  buey (20 madera + ~12 oro). Mientras no tenga carro, el pueblo acumula hasta llenar el almacén.
- Al acarrear, la «línea de producción» física sustituye a la penalización por distancia que ya se eliminó (el usuario
  lo hizo en otra sesión; al implementar, comprobar que 4.2 del canon ya no la menciona).

### 2.4 Mineros, guarnición y saqueo de un pueblo

- **Tropa nueva: Mineros.** Se recluta en el **Centro Urbano**, como los leñadores y los granjeros (Doc 5.8): leva,
  escalón 1, sin forma en batalla, con las mismas cifras que los leñadores (`leva_comunal`, poder 1, 30 unidades,
  velocidad 20). Cuestan 1 de piedra por soldado (su pico) y no pagan oro (excepción del Centro Urbano). **Entran
  también en la tropa prestada del campamento de mercenarios**, junto a la milicia, los leñadores y los granjeros.
- **Todo pueblo trae de serie una guarnición interna de mineros**, manejada por la IA. Así **un héroe sin tropa no
  puede atacarlo y ganar**. Es el equivalente a la defensa base de los carreteros de una caravana. Es **una escuadra
  de 30 mineros sin dueño** que no ocupa cupo, y gana siempre a un héroe solo (30 frente a 15 × 1,353). Si cae, se
  repone gratis con pesants de la madre en unas horas (placeholder 6 h).
- **Saquear un pueblo** es un **tipo de batalla propio**, cuyo dentro decide Conquest (mapa: el trazado del pueblo,
  §2.5). Exige guerra o la declara (§1.3).
- **El resultado es solo saqueo, nunca conquista**: el pueblo no cambia de dueño ni se destruye.
- **El pueblo protege el 50 % de su capacidad.** Si gana el atacante, se lleva solo lo que esté **por encima de la
  mitad de la capacidad**, recurso a recurso, hasta lo que le quepa en los carros. Si gana el defensor, no se lleva
  nada.

  | Almacén de 500 | Gana el atacante | Gana el defensor |
  |---|---|---|
  | 250 o menos | nada | nada |
  | 275 | 25 | nada |
  | 500 | 250 | nada |

- **El contraste es a propósito**: el pueblo protege el 50 %, pero el carro en marcha pierde el 70 % en B (§1.4). El
  punto vulnerable es el transporte.
- Con el carro saliendo lleno, el almacén solo pasa de la mitad entre un viaje y el siguiente. **Saquear rinde de verdad
  cuando el carro no vuelve**, porque está destruido o bloqueado. El saqueo premia el bloqueo.
- Bot contra bot, con números y la ventaja de B.

### 2.5 Trazado

- Se copia la receta de los campamentos de mercenarios (`layoutCampamento`): el trazado de asentamientos con **la mina
  (o la cantera) como ancla**, unas casas, el almacén y un corral para el buey.
- Se coloca una sola vez, sale del id del pueblo (función pura, no se guarda nada) y mide 1 de mapa como mucho.
- Es lo que se ve dentro y el mapa de su batalla de saqueo.

### 2.6 Conquista, ruina y solvencia

- **Conquista de la madre**: sus pueblos pasan al conquistador junto con la zona.
- **Ruina de la madre**: sus pueblos desaparecen.
- **Solvencia para subir de nivel** (Doc 4.5): mira madera, piedra y oro. Lo que llega por acarreo cuenta como **media
  de las llegadas** (ventana placeholder de 24 h) o como la producción del pueblo; las dos sirven. Solo el oro no es
  local en parte.

## 3. Obras de mapa (CERRADA)

Todo elemento de ámbito mapa (pueblos y avanzadas) **se construye igual**. **No pasa por la cola de
construcción**, que es local, del interior del asentamiento.

1. **Panel en la plaza madre.** Lo usan el Gobernador o el Maestro de Obras presentes, igual que la cola (propuesta).
   - **Pueblos**: el panel muestra los **candidatos**, que son los nodos de la zona sin pueblo. Un botón crea la obra en
     ese nodo.
   - **Avanzadas**: **no hay candidatos**. El panel muestra el límite actual, lo que hace falta para construirla y su
     función, y explica que se coloca donde se quiera dentro de la zona. La coloca el jugador: su héroe, en el sitio.
2. **La obra aparece en el mapa** en estado **`pendiente de materiales`**, con la lista de lo que necesita. Así el
   jugador sabe cuánto llevar antes de salir. Ocupa cupo desde ese momento.
3. **Llevar los materiales físicamente**, en los carros de las columnas de los héroes o en caravanas que van en un
   ejército. Se entregan en el sitio, con la columna a `radioPuerta`: es una acción de entrega, no un destino nuevo de
   caravana, para que no haya movimiento automático. Pueden entregar los ciudadanos de la Facción.
4. **Con todo entregado, arranca la obra**, que es corta (placeholder). Al acabar, el elemento queda activo.
5. **Cancelar o destruir** a mano: no se recupera nada de lo entregado.

Mientras está pendiente **se ve en el mapa, pero no se puede atacar**. Mejorar una avanzada a la etapa siguiente y
repararla siguen este mismo flujo.

## 4. Avanzadas (CERRADA)

### 4.1 Decidido

- **Nombre: Avanzada.** Tres etapas: **atalaya → fuerte (empalizada) → ciudadela (piedra)**. Las etapas son sus tres
  niveles; no hay niveles internos dentro de cada una. **Siempre se empieza por la atalaya** y se va subiendo: no se
  construye directamente un fuerte ni una ciudadela.
- **Nivel de la plaza madre**: atalaya y fuerte desde el **nivel 2**; ciudadela desde el **3** (como las murallas).
- **Cupo**: **una avanzada por nivel** de la plaza madre (2 en nivel 2 … 5 en nivel 5).
- **Ubicación**: donde quiera el jugador **dentro de la zona propia, nunca fuera**, y no dentro del asentamiento.
  Se construye, se mejora y se repara como **obra de mapa** (§3).
- **No genera zona de influencia.**
- **Visión: la plaza ve solo su zona, y lo de fuera lo ven las avanzadas.**
  - **La plaza deja de ver más allá de su frontera**: `VISION.margenAsentamiento` pasa de 60 a **0**. Ve su
    `radioPotencial` y nada más.
  - **Cada avanzada ve un círculo de 60** alrededor de sí (fijo en las tres etapas), independiente del de la plaza.
    Puesta en la frontera, asoma 60 más allá, en su dirección. Como solo se construye dentro de la zona, nunca ve más
    de 60 por fuera.
  - Por qué: con el margen de hoy, una avanzada de radio 60 dentro de la zona veía un subconjunto de lo que ya veía la
    plaza (se vio con el usuario en un visual interactivo, 2026-10-11). Consecuencias en §5.
- **Guarnición de jugadores**: cupo **por héroe residente de la madre y por avanzada**, en puntos de Liderazgo:
  **14 / 45 / 90** (atalaya / fuerte / ciudadela). Ejemplo: en una atalaya, cada uno de 3 jugadores puede poner una
  escuadra de granjeros y otra de leñadores (7 + 7). Las escuadras las llevan sus héroes hasta allí y comen el trigo
  de la madre.
- **Guarnición de serie**, manejada por la IA y fuera del cupo. Mejora mucho con cada etapa.
  - **La tropa la elige el Gobernador de la madre** entre las que tiene desbloqueadas la Facción en cada escalón; por
    defecto, la milicia de lanceros en el escalón 2.
  - **Se repone gratis en unas horas**, como los mineros del pueblo.

  | Etapa | Escalón 2 (el de la milicia) | Escalón 3 | Escalón 4 |
  |---|---|---|---|
  | Atalaya | 1 escuadra | — | — |
  | Fuerte | 3 | 2 | — |
  | Ciudadela | — | 5 | 3 |

  La ciudadela lleva solo las 8 mejores, sin escuadras del escalón 2, para no cargar a Unity con 18 escuadras de IA.
- **Batalla**: la misma en las tres etapas. Cambian el **mapa interior** (uno por etapa, para Conquest) y la
  guarnición de serie. **La abre solo un ejército y dura 30 minutos**, como un asedio. Atacarla es guerra o la
  declara (§1.3).
- **Almacén y reabastecimiento**: la atalaya no tiene almacén; el fuerte, unos **2.000**; la ciudadela, unos
  **10.000**. Se llena con los carros de los héroes. Un **ejército** de la Facción reposta en su puerta con el botón
  **Repostar**, como en una plaza (Doc 5.13), y así se alarga el radio operativo de una campaña. Las columnas personales
  no lo necesitan: dentro de la zona ya repostan solas de la plaza.
- **Si gana el atacante:**

  | Etapa | Resultado |
  |---|---|
  | Atalaya | **Destruida** |
  | Fuerte | **En ruinas**, con **1 día de invulnerabilidad**. Se puede reparar. Si pasado ese día sigue en ruinas, la atacan y gana el atacante, queda **destruido** |
  | Ciudadela | **En ruinas**, **nunca se destruye**. Se puede reparar |

  - Del almacén, el atacante se lleva lo que le **quepa en los carros**.
  - **En ruinas no funciona nada**: ni visión, ni guarnición, ni reabastecimiento. La guarnición de los jugadores
    queda a 0 y vuelve al campamento de su héroe (como 5.15.5).
  - **Reparar cuesta el 50 % de construir** la etapa, como obra de mapa.
- **Destrucción manual** por quien la construyó, para reubicarla: atalaya y fuerte sí, la ciudadela no. No se recupera
  nada.
- **Conquista de la madre**: pasan al conquistador.
- **Ruina de la madre**: atalaya y fuerte desaparecen. La **ciudadela queda en el mundo como ruina sin dueño**, y
  quien tenga después esa zona puede repararla.
- **Mantenimiento**: edificio activo de la madre. **Niebla**: se recuerda como una ciudad. **Escala**: 1 de mapa como
  mucho.

## 5. Transversal

- **Bots**: tienen que aprender a marcar obras de mapa y llevar materiales, escoltar el acarreo, saquear, guarnecer
  avanzadas y aceptar (o no) el riesgo de declarar la guerra al atacar.
- **Contrato, cliente y canales WS**: entidades nuevas (pueblo, carro de acarreo, avanzada, obra pendiente) que se
  transcriben enteras en el modelo de datos. Canales de vista: mapa y plaza (panel de candidatos).
- **Conquest**: modos A y B de caravana, saqueo de pueblo, batalla de avanzada con un mapa por etapa, y las
  definiciones de escuadra de los mineros.
- **Visión de las plazas recortada a su zona** (§4.1):
  - Una plaza de nivel 1 (zona 30-60, sin avanzadas hasta el nivel 2) ve mucho menos que hoy. Un ejército sigue
    viendo 150, así que gana todavía más iniciativa frente a una plaza joven; el día de protección tras fundar
    amortigua el arranque.
  - `radioReabastecimiento` (60) deja de coincidir con lo que ve la plaza: una columna puede repostar en una ciudad
    que no la ve. Es aceptable, porque reponer víveres no depende de verla.
  - Ganan valor las Miradas de la taberna (Doc 5.12.10) y la visión compartida entre aliados.
  - La memoria del mapa (`engine/memoria.ts`), la proyección del jugador (`session/proyecciones/jugador.ts`), sus
    tests y los bots, que deciden con lo que ven, usan hoy `margenAsentamiento`.
- **Ritmo**: hay que volver a medir con batch. Era I hoy: nivel 2 a los 5,8 días. El objetivo explícito es más guerra.
- **Migración**: mundos con minas ya construidas en plazas de nivel 1 (decidir al implementar: convertirlas en pueblo
  o retirarlas).

## 6. Canon a reescribir al implementar

| Doc | Sección | Cambio |
|---|---|---|
| 1 | 1.6 | Fuera la inmunidad; tipos A/B |
| 1 | 1.4 | Minerales solo con pueblo |
| 1 | 1.9 | Ataque de bandidos a caravanas como evento PvE |
| 2 | 2.4.1 | Toda agresión entre Facciones exige guerra o la declara; cualquier héroe; excepción del asedio ajeno |
| 3 | 3.10 | Consecuencias A/B, 70/30, sin el 50 % |
| 3 | 3.13 | Carro de acarreo |
| 3 | 3.13.6 | Captura por carro |
| 4 | 4.2 / 4.2.1 | Pueblos extractores, obras de mapa, primera Cantera directa, reemplazo automático |
| 4 | 4.5 | Solvencia con media de llegadas |
| 5 | 5.2 | Modalidades: aguante, saqueo de pueblo, avanzada, bandidos PvE |
| 5 | 5.8 | Mineros en el Centro Urbano |
| 5 | 5.12.7 | La plaza ve solo su zona (`margenAsentamiento` 0); avanzadas de radio 60 |
| 5 | 5.13 | Reabastecimiento en avanzadas |
| 5 | 5.15.1b / 5.15.3 | Tabla de tipos; guarnición de pueblos y avanzadas (de serie y de jugadores) |
| — | Doc 01, Doc 02, `src/contratos/v1/` | Ídem |
| — | `Docs/Mechanics/` | El doc de cada mecánica que cambie |

## 7. Plan (borrador, por detallar)

1. Guerra de caravanas: tipos A/B en el motor, declaración automática con confirmación, consecuencias por carro y
   70/30, números bot contra bot, contrato.
2. Obras de mapa: candidatos, estado pendiente de materiales, entrega en el sitio.
3. Pueblos: entidad, almacén, extracción y reglas de nivel 1.
4. Mineros (Centro Urbano y tropa prestada) y guarnición de serie del pueblo.
5. Carro de acarreo: ciclo, compra, escolta.
6. Saqueo de pueblo: batalla y números.
7. Trazado de pueblo.
8. Cliente: aviso de guerra, panel de candidatos, pueblos, obras y carros en el mapa.
9. Bots.
10. Batch.
11. Avanzadas: obra, etapas, guarnición de serie y de jugadores, almacén, ruinas.

## 8. Placeholders

- **Combate**: `ventajaDefensor` de B ×1,25. Pérdida con números: la mitad de los carros (A y B) y el 70 % de la carga
  (B).
- **Pueblo**: saqueo por encima del 50 %; almacén de 500; carro de acarreo de 500 a velocidad 16; cupo de pueblos
  2 / 3 / 4 / 5; escolta del acarreo sin Mercado 100; mineros de serie: 30, que se reponen en 6 h.
- **Solvencia**: ventana de la media de 24 h.
- **Avanzadas**: visión 60; guarnición de jugadores 14 / 45 / 90 por héroe; almacén 0 / 2.000 / 10.000; guarnición
  de serie que se repone en unas horas (como los mineros).

**Costes y obras (decididos 2026-10-11, cifras placeholder).** Están medidos en viajes de carro de héroe (500), porque todo se lleva a mano:

| Obra | Coste | Viajes de carro (500) | Obra |
|---|---|---|---|
| Pueblo | coste de su extractor + 50 madera (casas y almacén) + carro 20 madera + buey 12 oro. Mina de cobre: 100 madera + 12 oro | 1 | la del extractor (4-8 h) |
| Atalaya | 200 madera + 100 piedra | 1 | 2 h |
| Atalaya → fuerte | 600 madera + 400 piedra + 50 oro (≈ la obra de ascenso a nivel 2) | 2 | 12 h |
| Fuerte → ciudadela | 1.500 madera + 2.000 piedra + 300 oro (≈ la mitad del ascenso a nivel 4) | ~8: un ejército con caravana | 2 días |
| Reparar | 50 % del coste de la etapa | — | 50 % de la obra |

