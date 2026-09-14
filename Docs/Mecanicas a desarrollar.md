# Mecánicas por desarrollar

La **lista única de lo pendiente**. El estado completo de todas las mecánicas del juego —hechas, descartadas y
estas— vive en `Consideraciones/Checklist_Mecanicas.md`, que para lo pendiente solo resume y apunta aquí.
Re-contrastado contra `src/` el 2026-09-09.

Cuando una entrada se cierra (diseño **y** implementación), se **borra entera** de este archivo: lo útil que
no esté ya en el canon (`Docs/Game/`) o en una ficha de `Consideraciones/` se mueve allí primero. Mientras una
mecánica se está diseñando, sus acuerdos provisionales pueden vivir aquí como notas.

## Índice

| # | Área | Mecánica | Código hoy |
|---|---|---|---|
| 3 | CARAVANAS | Rutas de caravana avanzadas | ◐ solo el pathfinder base |
| 5 | TRUEQUE | Trueque compuesto de varios materiales | ✘ nada |
| 8 | CARAVANAS | Revamp de caravanas — solo los trozos diferidos (§8.1) | ◐ núcleo hecho; §8.1 no |
| 9 | ASENTAMIENTO | Eventos de asentamiento | ✘ nada |
| 10 | WORLDGEN | Landmarks reconocibles | ✘ nada |
| 11 | JUGADOR | Progresión de Liderazgo del jugador | ✘ nada |
| 13 | POLÍTICA | La capital como decisión del jugador | ✘ proxy placeholder |
| 16 | JUGADOR | Qué hace un huésped: vida dentro de una Facción NPC | ✘ nada |
| 18 | INTEL | Taberna + intel como asset con revelado temporal | ✘ nada |
| 19 | POLÍTICA | El mapa político como entidad | ✘ nada |
| 20 | TECNOLOGÍA | Tecnología: árbol de desarrollo propio, 3 vías, Aedas | ✘ nada |
| 21 | POLÍTICA | Los 4 gremios escasos a nivel de servidor | ✘ nada |
| 22 | POLÍTICA | Exilio como política de soberanía | ✘ nada |
| 23 | RECURSOS | Materiales exóticos | ✘ nada |
| 24 | SERVIDOR | Ciclo de servidor de 12 meses + Maravilla + legado NPC | ◐ solo el edificio |
| 25 | POLÍTICA | Coste/beneficio mecánico de las ordenanzas de trazado | ◐ existen, sin coste propio |
| 26 | CIUDADANÍA | Los beneficios de ciudadanía sin implementar | ◐ 2 de 5 |
| 27 | AMBIENTACIÓN | Identidad visual y de audio | ✘ nada |
| 28 | MILITAR | Declaración formal de guerra | ✘ nada |
| 29 | ONBOARDING | Curva de progresión inicial gradual | ✘ nada |
| 30 | MILITAR | Batallas con héroes: guarnición, campamento y héroes bot | ◐ campamento, guarnición y conquista del canon; el asedio aún con números |
| 31 | HÉROE | Modelo de Héroe: uno por jugador y mundo, dueño de los escuadrones | ◐ fases 1-3 en la rama `heroe-dominio`; faltan perks, equipo y herido |
| 32 | POLÍTICA | Cabos sueltos de diseño político | ✘ sin decidir |
| 33 | COMERCIO | Cabos sueltos de diseño comercial | ✘ sin decidir |
| 34 | SUMINISTRO | La economía no llena el carro de un ejército | ✘ sin decidir |
| 35 | VARIOS | Cabos sueltos de mundo, población y militar | ✘ sin decidir |
| 36 | HÉROE | Comportamiento de los héroes bot en el mundo | ✘ nada |

**Pospuesto explícitamente, fuera de esta lista:** el **Attack Timer** (Doc 5.6, decidido y aplazado a
fase posterior a Fase 0) y el **comercio marítimo / unidades navales** (fuera del alcance de Fase 0 por
diseño). Los ajustes de calibración de mecánicas ya construidas viven en cada ficha de `Consideraciones/` y en
`Preguntas_Abiertas.md`, no aquí. Lo ya cerrado (diseño + implementación) se retira de este archivo por
completo; su estado queda en el checklist.

## 3. Rutas de caravana avanzadas

El pathfinder base ya está (`world/rutas.ts`, `engine/caminos.ts`): calcula por coste de terreno, el agua es
infranqueable, y el camino generado da bonus de velocidad a las caravanas que lo siguen. Falta todo lo demás.

El pathfinder de las rutas debe buscar evitar bosques (rodearlos) o ríos (no los puede atravesar).

Cuando hay muchos caminos que pasan cerca en el mapa general debido a rutas de caravana se deberían juntar
para formar caminos unificados; y si estos caminos, para ir de A a C, tienen a B justo en el camino o cruzan
la zona de influencia de B, deben pasar por la ciudad B de camino a C y dejar una pequeña comisión. Cuando una
caravana está cruzando una ciudad neutral o aliada no puede ser atacada.

Cuando se forma un camino se evalúa la proximidad con otros caminos; cada otra caravana que use ese camino le
agrega 1 punto. Mientras más puntos, más grande se ve en el mapa real y atrae con más fuerza a otras rutas
para que se desvíen, aunque sea un poco, de su camino — creando caminos principales.

*Relacionado:* que un río corte el paso necesita vados o puentes para no fragmentar el mapa
(`Docs/Game/1` §1.6, `worldgen/costeMovimiento.ts`).

## 5. Trueque compuesto de varios materiales

`AcuerdoTrueque` es hoy un intercambio de **un** recurso por **un** recurso (`recursoA` / `recursoB`). Un
acuerdo debería poder llevar varios materiales por lado.

## 8. Revamp de caravanas — trozos diferidos

El **núcleo está implementado** (2026-09-08, Pasos 1-5): la caravana `comercial` es un contenedor de carros
con animal que deriva capacidad/velocidad, casco vacío + piezas, lanzamiento manual con estado `preparando`,
y escolta sin héroe. Reglas en `Docs/Game/3` §3.13; decisiones, motor y plan en
`Consideraciones/Revamp_Caravanas_Definicion.md`.

### 8.1 Lo diferido — forma diseñada, implementación en un pase posterior

El diseño de 2026-09-08 recortó seis piezas del enunciado original para no inflar el primer pase. Ninguna se
descartó: se decidió su forma y se aparcó. Aquí queda cada una con lo que falta para abordarla.

**a) Planificación horaria de caravanas.** El enunciado pide que una caravana no solo se lance ahora, sino
que se deje *programada* para salir a cierta hora de mundo. Es un scheduler: una caravana `preparada` con
carga/destino/escolta fijados y una hora de disparo. El motor ya tiene el gancho — la infra de *scheduled
commands* está documentada como "aterriza con su primera mecánica de Fase 1" (roadmap D5, Doc 7). Este sería
ese primer consumidor. Falta: el comando `programarCaravana(id, { …, dispararEn: Instante })`, el estado
`programada` y su ejecución diferida en el reloj de mundo (`RunnerDePartida`).

**b) Cría de animales de arrastre.** Hoy los animales solo se compran con oro. El enunciado quiere obtenerlos
también por cría. El Corral (Doc 1.4/4.2.1) produce *livestock*, que es un recurso distinto — la cría de
bueyes/caballos/camellos necesitaría su propio edificio o una receta que consuma livestock + trigo y tarde
ticks. Falta: decidir si es un edificio nuevo o una función del Corral, el coste y el ritmo, y si cada tipo
de animal exige condiciones (el camello, un bioma; el caballo, quizá un nivel de asentamiento).

**c) Visibilidad por tamaño.** Las caravanas pequeñas no deberían aparecer en el mapa general salvo que haya
un jugador cerca (regla de niebla actual); las grandes deberían **llamar la atención desde que se preparan**,
al punto de ser visibles para asentamientos hasta cierta distancia, para que salgan a interceptarlas. Es el
gancho de conflicto del enunciado. Falta: un umbral de tamaño (nº de carros y/o carga) que decida si la
caravana entra en la proyección de niebla de otras Facciones y a qué radio, y que eso aplique **durante el
estado `preparando`**, no solo en ruta. Engancha con Doc 5.12 (niebla de guerra).

**d) Inmunidad del camello al desierto — POSPUESTO a fase posterior a Fase 0** (decisión del usuario,
2026-09-09). El camello "no se muere en los desiertos"; buey y caballo sí. Pero no existe un bioma `desierto`
de primera clase (el tipo es `agua|costa|estepa|llanuraFertil|colina|montana|cima`; la aridez del Nilo es
`estepa` de fertilidad baja). Necesitaría o un `BiomaTipo` nuevo, o anclar el "desierto" a `estepa` bajo un
umbral de fertilidad + una regla de *attrition* por tick sobre buey/caballo al cruzarlo. Es un cambio de
worldgen que no aporta a Fase 0; se retoma cuando el mapa tenga terreno árido real. Mientras tanto el camello
es una "opción media" a secas, aceptado.

**e) Catálogo ampliado de carros.** El primer pase trae solo dos carros: el básico (Mercado) y uno
"reforzado" (Carpintería) que solo da más capacidad. El enunciado habla de "varios tipos" fabricables en la
Carpintería. Falta: los ejes que diferencian un carro de otro más allá de la capacidad — resistencia a la
captura (un carro que sobrevive a una derrota), penalización de velocidad (un carro que no frena tanto al
animal rápido), coste en recursos más caros. Se abre cuando la Carpintería tenga niveles internos que lo
justifiquen.

**f) Unificación con el carro de columna.** `Ejercito.suministro` (Doc 5.13) y los carros de una caravana son
el mismo concepto físico: un vehículo con capacidad tirado para llevar carga por el mapa. Doc 5.13.3 ya dejó
anotado que se unifican "cuando se diseñe el revamp". El revamp los deja **separados a propósito** en este
pase —una caravana adjunta a un ejército sigue siendo su propia entidad— porque unificar el modelo físico es
un refactor sin premio de juego inmediato. Falta: un tipo `Carro` compartido y que tanto `Ejercito` como
`Caravana` lo compongan.

## 9. Eventos de asentamiento

Los asentamientos tienen eventos propios como, por ejemplo, ser sitiados por bandidos (y los jugadores tienen
cierta cantidad de horas para formular la defensa y jugar la defensa). Pensar en otro tipo de eventos que
mantengan entretenido el juego.

*Base ya disponible:* los campamentos de bandidos existen y atacan caravanas cada tick
(`engine/bandidos.ts`), pero nunca asedian un asentamiento.

*Incluye:* **NPCs hostiles más allá de los bandidos** — hoy los bandidos son la única amenaza no-jugador del
mundo. Fauna peligrosa, incursores estacionales, u otros agresores ambientales caben aquí.

## 10. Landmarks

Hay que añadir la creación de landmarks reconocibles en el mapa, de modo que el jugador que lo explora pueda
reconocer por dónde va sin perderse del todo — algo que ayude a reconocer qué cosas están cerca de qué.

## 11. Progresión de Liderazgo del jugador

`Jugador` tiene hoy `liderazgoBase` y nada más. El propio código lo anota: *"el efectivo es base + progresión,
pero la progresión todavía no está diseñada, así que hoy coinciden"* (`domain/types.ts`, `constants.ts` §1587).
Un jugador sin registro usa `LIDERAZGO.base`.

Falta decidir qué hace subir el liderazgo (combatir, ganar, tiempo al mando, cargo militar…) y con qué curva.
Los **escuadrones** progresan por nivel y experiencia (Doc 5.16.3), sin cambiar nunca de tropa (Doc 5.8); el
motor aún usa veteranía (§31). La mecánica de Liderazgo ya admite un efectivo > base sin tocar nada — solo falta la
fuente.

## 13. La capital como decisión del jugador

La capital tiene que ser una **decisión consciente de los jugadores**, no algo heredado.

Hoy no lo es. `encontrarCapital` (`engine/mantenimiento.ts`) devuelve **el asentamiento vivo más antiguo de la
Facción** —literalmente `sort((a,b) => a.fundadoEn - b.fundadoEn)[0]`— y el propio código lo marca como
*"placeholder = proxy de capital"*. De ahí salen tres problemas:

1. **No se elige, se hereda.** El primer asentamiento es capital para siempre, aunque acabe siendo un
   villorrio y la Facción tenga su verdadero centro de poder en otra parte. No hay forma de trasladarla.
2. **El Palacio no pinta nada.** Existe el edificio `palacio` (Doc 4.2.1) y la capital lo ignora por completo.
   Lo natural sería que la capital fuera *donde está el Palacio*, o que designarla lo exigiera.
3. **Y sí tiene efecto mecánico real**, así que no es cosmético: el mantenimiento de cada asentamiento escala
   con su distancia a la capital — `factorDistancia = 1 + min(1, dist/400) × (2-1)`, o sea ×1 en la capital y
   hasta **×2 a distancia 400**, topado a partir de ahí. Es el mecanismo anti-snowball de "cohesión"
   (Fase_0_5 §5.1).

Dos cosas que hay que decidir con ello:

- **Cómo se designa y qué cuesta trasladarla.** Si mover la capital es gratis, el jugador la reubica cada vez
  que conquista algo y el factor de distancia deja de morder.
- **El tope a 400 desactiva el anti-snowball.** Más allá de esa distancia no hay penalización adicional: un
  imperio de punta a punta del mapa paga lo mismo que uno moderadamente disperso. Con provincias de radio ~76
  (ver la escala del mundo), 400 son ~5 provincias — o sea que el "radio cómodo" de un reino ya está fijado en
  el código sin que nadie lo decidiera.

## 16. Qué hace un huésped: vida dentro de una Facción NPC

**Estado: idea, sin diseñar.** Sale de la entrada al mundo
(`Consideraciones/Entrada_Al_Mundo_Definicion.md` §5) y no es un adorno: **es el contenido del vestíbulo**.
Sin ella, la fase de huésped —antes de poder fundar— se reduce a "espera", que es la peor versión de esto.

La idea del usuario: **ganar posición dentro de una Facción IA haciendo cosas para ella** — escoltar sus
caravanas, explorar, buscar cosas en el mapa de campaña. Acciones que sirven de tutorial y que dan recompensa
dentro de esa misma Facción.

**Lo que ya existe y reutilizaría:** la escolta de caravanas (`Caravana.escolta` / `adjuntarCaravana`, Doc
5.13.3), la exploración (`engine/exploracion.ts`), la reputación de Facción y la experiencia (`REPUTACION`,
`aplicarAjustesExperiencia`).

**Lo que no existe, y es el corazón de la mecánica:** *standing por jugador dentro de una Facción*. Hoy la
reputación y la experiencia son de la Facción entera, no de cada uno de sus miembros. Sin eso no hay nada que
subir ni nada que recompensar.

**Lo que hay que decidir:** qué encargos existen y quién los publica; qué se gana (¿acceso a reclutar? ¿casa?
¿aval para fundar?); y si ese standing sobrevive a marcharse de la Facción.

## 18. Taberna + intel como asset

**Estado: idea, sin diseñar (`Consideraciones/Taberna_Intel_Definicion.md`, aún vacío).** Con la niebla de
guerra ya en el juego, saber qué pasa en otro sitio gana valor — y ese valor es el **sink recurrente de oro**
que la calibración de la economía del oro asume que va a llegar
(`Economia_Del_Oro_Definicion.md`, "Fuera de este plan").

La idea del usuario: un edificio nuevo, la **taberna**, donde se compra **intel** o **mapas** — información
con **revelado temporal**: qué está pasando en otro sitio durante un tiempo limitado, o la situación actual
de una Facción. No solo lo visual del mapa: también información interna de asentamientos. El **layout de un
asentamiento** se puede comprar como asset, para preparar asedios a futuro.

*Estado en código:* nada. No hay edificio `taberna` ni concepto de "intel como asset".

## 19. El mapa político como entidad

**Estado: idea, sin diseñar.** Las fronteras "de dónde a dónde llegan" como una entidad de primera clase, no
algo que se recalcula al vuelo. Hoy las zonas de influencia y las fronteras ajenas se derivan
(`engine/zones.ts`, fronteras ajenas 2026-09-05); no existe un "mapa político" consultable como objeto.

## 20. Tecnología: árbol de desarrollo propio, 3 vías, Aedas

**Estado: diseño en el canon (`Docs/Game/6_Sistema_de_Tecnologia_y_Aedas.md`), `código: ✘` — nada.** Ni
árbol, ni las tres vías de acceso (comercio / desarrollo propio / Aedas), ni la compra de tecnología con oro,
ni el sistema de Aedas. Lo único con el nombre "Aeda" en el código es la narración de cambios de título de
servidor (`engine/titulos.ts`), que no tiene relación con esto.

Sin decidir: si el árbol del desarrollo propio es estructurado o libre/emergente (checklist "Tecnología").

## 21. Los 4 gremios escasos a nivel de servidor

**Estado: diseño parcial en el canon (Doc 2.10), `código: ✘` — nada.** Los 4 gremios (Comerciantes,
Artesanos, Constructores, Ladrones), escasos a nivel de servidor, con una tirada periódica sujeta a cuatro
requisitos simultáneos (reputación > 90, título de servidor específico, nivel de asentamiento en el máximo,
mantenimiento > 90 %), y su mecanismo de pérdida. El `patioDeGremios` de `constants.ts` es solo una parcela
decorativa del trazado urbano, sin relación con esta mecánica.

Pendiente de decidir (Preguntas_Abiertas §14): valores numéricos de cada requisito por gremio, el título de
servidor asociado a cada uno, el detalle de beneficios de Comerciantes/Artesanos/Constructores, y si hay
margen de gracia antes de perder el gremio.

## 22. Exilio como política de soberanía

**Estado: diseño cerrado en el canon (Doc 5.9 / 2.8), `código: ✘` — nada.** El exilio como palanca de
soberanía de una Facción sobre sus miembros. Sin implementar.

## 23. Materiales exóticos

**Estado: `código: ✘` — no existe ese tipo de recurso.** Los pide el diseño de la Maravilla (Doc 6 / §24) y
posiblemente el catálogo de commodities de Nobleza. Hoy la Maravilla se paga con un coste placeholder de
recursos ya existentes. Falta: qué materiales son, de dónde salen (¿nodos raros? ¿bioma? ¿solo comercio de
larga distancia?), y qué los consume además de la Maravilla.

## 24. Ciclo de servidor de 12 meses + Maravilla + legado NPC

**Estado: el EDIFICIO Maravilla implementado (único, nivel de asentamiento máximo, vía cola manual, coste
placeholder); el CICLO no.** Diseño cerrado en `Consideraciones/Roadmap_Escalado.md` Eje 4 y
`Preguntas_Abiertas.md` §14d: 12 meses de servidor, cierre anticipado por la primera Facción que complete la
Maravilla, y la Facción ganadora persiste como Facción-legado NPC de solo mantenimiento y comercio.

Requiere infraestructura de servidor / multi-instancia (reset, generación del nuevo mapa, destino de las
Facciones no ganadoras, si la legado es atacable). Nada de eso tiene código. Ver la lista completa en el
Roadmap.

## 25. Coste/beneficio mecánico de las ordenanzas de trazado

**Estado: las cinco ordenanzas existen y funcionan (`código: ◐`), pero no tienen coste/beneficio propio.** Las
cuatro ordenanzas de perfil de trazado del Maestro de Obras (`postura_defensiva` / `arterias_comerciales` /
`barrios_gremiales` / `plazas_mayores`) más `lineas_produccion` orientan *cómo* se distribuye la ciudad, y ya
compiten por el único slot de Maestro de Obras. Pero compiten en desventaja contra Vía Rápida (−25 % de tiempo
de obra), que sí da un beneficio medible.

Falta: darle a cada ordenanza un coste o un beneficio mecánico propio, de modo que elegir una sobre otra sea
una decisión con precio. Anotado como pendiente en el propio `constants.ts`. Diseño del trazado en
`Consideraciones/Vista_Asentamiento_Trazado_Urbano.md`.

## 26. Los beneficios de ciudadanía sin implementar

**Estado: `código: ◐` — 2 de 5.** La ciudadanía da hoy en el motor el nivel intermedio de comisiones y la
residencia/reclutamiento. Los otros tres beneficios del diseño (residencia en cualquier asentamiento de la
Facción, protección militar explícita, voz en política exterior) no están cableados. Doc 2 (`CIUDADANIA`).

También sin cerrar:

- **Cooldown de cambio de residencia**: `CIUDADANIA.cooldownCambioResidenciaDias` está reservado, sin valor ni
  efecto. Frenaría mudarse en cada conquista para exprimir la recaudación.
- **Dejar una casa sin tomar otra**: abandonar la Facción no libera la residencia ni los cargos locales, y no
  hay forma de vender una casa a secas.

## 27. Identidad visual y de audio

**Estado: `código: ✘` — nada.** Sigilo / estandarte de Facción, identidad visual y de audio de imperios y
títulos. Sin referencias estéticas concretas decididas (micénica, hitita, mesopotámica…). Preguntas_Abiertas
§9. Es sobre todo trabajo de un repo de interfaz aparte, pero la *representación* (qué campo lleva el sigilo,
dónde vive) toca este repo.

## 28. Declaración formal de guerra

**Estado: `código: ✘` — nada.** Hoy el combate es un cálculo puntual entre columnas y plazas; no existe un
estado de "guerra activa" entre dos Facciones. Falta decidir si declarar la guerra requiere una condición
previa (frontera compartida, casus belli) o es libre, y qué habilita (Preguntas_Abiertas §1). Bloquea el
bloqueo/escolta militar de chokepoints que quedó a medias en Fase 0.3 (Preguntas_Abiertas §3).

## 29. Curva de progresión inicial gradual

**Estado: decisión de diseño real pendiente (Preguntas_Abiertas §14e), `código: ✘`.** El inicio del juego
debe ser suave, con sistemas desbloqueándose progresivamente en vez de exponer los 7 cargos, la
Liga/vasallaje, la reputación y los gremios desde el primer tick.

Sin resolver: qué sistemas se difieren y cuáles no; si el desbloqueo se ata al nivel de asentamiento, al nivel
de Facción, al tiempo o a una combinación; y si aplica solo a la interfaz (ocultar) o también a las reglas
(bloquear). Distinto de §16 (contenido del vestíbulo) y del onboarding ya hecho (spawn + fundación grupal):
esto es el ritmo de la primera hora una vez dentro.

## 30. Batallas con héroes: guarnición, campamento y héroes bot

**Estado: reglas principales cerradas (canon Doc 5.15, 2026-09-13), `código: ◐`.** Desde la fase 2 del Héroe
(rama `heroe-dominio`, 2026-09-14) las escuadras viven en su héroe y el campamento sigue el canon: se guarnece
solo si todos los que van dentro residen ahí; conquistar deja la plaza sin guarnición, el ejército acampado a
la puerta y a los residentes vencidos con su campamento a 0 en la plaza más cercana de su Facción (huérfanos si
no queda ninguna); cambiar de residencia traslada el campamento; la escolta de una caravana perdida vuelve a 0
al campamento; y el Liderazgo suma la escolta a lo que el héroe lleva en columna (Doc 3.13.4). Desde la fase 3,
también la guarnición: cada residente asigna escuadras a la de su residencia dentro de su cupo, y un asedio lo
defienden la guarnición y el loadout activo de los residentes que están dentro; el resto del campamento no
defiende. Los héroes bot no usan la guarnición, defienden con su loadout.

Sigue distinto del canon: el asedio se resuelve con números (`iniciarAsedio`, `engine/combate.ts`), no como
batalla de Unity; la Tregua por columna en vez del estado Herido del héroe; el campamento de bandidos se ataca con un comando
desde un asentamiento (`atacarCampamentoBandidos`) en vez de con una columna que llegue a él (Doc 1.9); quien
está DENTRO de una plaza cuando cae conserva esa ubicación, cuando el canon lo deja fuera; y el desalojo no mira
el cupo de viviendas de la plaza que recibe a los vencidos (`desalojarResidentes`). Depende del ciclo de
`Batalla` con Unity (BA-001; CQ-002 en Conquest para la IA de escuadras sin héroe y de héroes bot).

Sin resolver:

- La XP de las escuadras en batallas que se resuelven con números: hoy `experiencia` suma lo que sumaba la
  veteranía (+1 al ganar, +0,5 al perder) y `nivel` se queda en 1; la curva se fija cuando Conquest publique
  la suya (CQ-001).

## 31. Modelo de Héroe

**Estado: reglas principales cerradas (canon Doc 5.16 y glosario, 2026-09-11 y 2026-09-13), `código: ◐`.**
Datos y orden de implementación en `Docs/Coordinacion/01_Modelo_de_datos_compartido.md` §12-§14 y BA-004.
Hecho: el contrato (`src/contratos/v1/`) y, en la rama `heroe-dominio` (2026-09-14), la fase 1 —`Heroe` en el
dominio con su identidad, `heroeId` como dueño en todo el motor, `crearHeroe`, y Facciones NPC creadas por el
admin con héroes bot— y la fase 2 —las escuadras viven en `Heroe.escuadrones` con su `contenedor` (campamento,
ejército o escolta), nivel y experiencia en vez de veteranía, sin `heridoHasta`; ejércitos y caravanas guardan
solo ids—. La fase 3 añade la progresión del héroe (nace como en Conquest: nivel 1, sin puntos, 500 de bronce y el
loadout "Default"), `repartirPuntos` (solo atributos), `guardarLoadout`/`borrarLoadout`,
`asignarGuarnicion`/`retirarGuarnicion`, y en la proyección `heroe` (con el coste de Liderazgo de cada escuadra y la guarnición ocupada), `heroesVisibles`
(`HeroePublico`) y `nombresDeCompaneros`. Falta:
los perks y `equipar`, que esperan a los catálogos de Conquest (CQ-004; decisión del usuario 2026-09-14); de
dónde salen los puntos de atributo (en Conquest ningún nivel los da) y la subida de nivel (CQ-001); el estado
Herido del héroe; y en el cliente de jugador (`BronzeAgeClient`), que ya lee los datos nuevos y tiene los
comandos tipados, la interfaz: la pantalla de crear héroe definitiva y la del héroe (su
`docs/Features_Pendientes.md` §0).

Sin resolver:

- Lo que el botín deja abierto (preguntado a Conquest en CQ-004): si en batalla se gastan consumibles o se
  pierde equipo, quién aplica la compatibilidad arma/armadura al equipar, y si el héroe tiene fuentes de
  objetos y monedas fuera de la batalla (tienda, recompensas).

## 32. Cabos sueltos de diseño político

**Estado: preguntas abiertas que el canon (Docs 2 y 4) no cierra, `código: ✘`.** Piezas pequeñas, sin
mecánica propia:

- **Sucesión con Liga** (Doc 2.2): si el Rey de una Liga abandona su Facción, ¿se hereda el vasallaje? ¿se
  re-vota en una federación? Depende de la votación real.
- **Desarme del señor** (Doc 2.4): la condición exacta por la que un señor cuenta como "desarmado" y sus
  vasallos quedan libres.
- **Fusión/anexión** (Doc 2.6): si requiere aceptación mutua explícita, o si la anexión se puede forzar con
  suficiente diferencia de poder.
- **Reputación** (Doc 2.7): el valor de cada evento, la velocidad de decaimiento y los umbrales de cada uso.
- **Cupo de nivel 4 y 5** (Doc 2.2.1): hoy no tienen cupo por Facción; falta decidir si deben tenerlo y con
  qué curva.
- **Catálogo de políticas** (Doc 4.4): las políticas concretas de cada pool más allá de las que existen, y si
  son excluyentes entre sí dentro de un slot.
- **Redistribución de Vivienda** (idea): una política que cambie la proporción fija 15/5 de Pesants/Artesanos
  de cada Vivienda. Falta si desplaza cupo de una clase a otra o añade cupo extra, sus valores y de qué cargo
  es (Maestro de Obras o Sacerdote).

## 33. Cabos sueltos de diseño comercial

**Estado: preguntas abiertas que el canon (Docs 1 y 3) no cierra.**

- **Caravanas militar y de contrabando** (Doc 3.6): existen en `CARAVANA_CATALOGO` con capacidad y velocidad,
  pero el motor nunca las instancia. Falta conectarlas: la militar, a llevar equipo antes de un asedio; la de
  contrabando, a una mecánica de detección reducida.
- **Riqueza acumulada** (Doc 3.5): en qué se usa el oro que las plazas acumulan por comisiones.
- **Comisión intermedia** para Facciones aliadas o vasallas de la misma Liga (Doc 3.5 y §26).
- **Bonus por distancia en el mostrador** (Doc 3.8): solo aplica al trueque; aplicarlo a las órdenes de
  mercado es una decisión de diseño abierta.
- **Intercambio directo entre jugadores** (Doc 3.7): cara a cara en mitad del mapa, sin plaza ni acuerdo de
  por medio. No existe.
- **Cortar rutas como guerra económica** (Doc 3.9): que el combate de caravanas interactúe con los acuerdos en
  curso.
- **Retirada del almacén al salir** (Doc 1.10.2): que el Tesorero pueda fijar cuánto material puede llevarse
  cada héroe.

## 34. La economía no llena el carro de un ejército

**Estado: medido el 2026-09-04, sin decidir.** La capacidad del carro (Doc 5.13.1) se derivó del radio
operativo sin comprobar que hubiera trigo con el que llenarlo. En batch, ningún asentamiento llegaba a llenar un
carro y 26 de 28 no podían aportar ni un grano sin bajar de su reserva de comida. Después se dobló la
producción de la Granja (Doc 4.2.1); falta volver a medir. Cifras, causa y las cuatro palancas posibles en
`Consideraciones/Movimiento_Ejercitos_Definicion.md` §10.

## 35. Cabos sueltos de mundo, población y militar

- **Campamentos de bandidos** (Doc 1.9): si su poder debería escalar con la región o con la cercanía de
  Facciones fuertes, y si deberían bloquear la explotación del bosque que ocupan.
- **Cola de prioridad de reclutamiento** (Doc 4.1): con qué criterio se reparte el pool de población cuando la
  demanda de reclutas lo supera.
- **Gran Fundición** (Doc 4.2.1, 5.7): está en el catálogo (nivel de Facción 3) pero no tiene función desde que
  la Nobleza dejó de reclutarse; falta decidir su papel junto a la Fundición.
- **Armas de asedio** (Doc 4.2.1): la Carpintería está pensada para arietes y torres de asedio, que Fase 0 no
  tiene.

## 36. Comportamiento de los héroes bot en el mundo

**Estado: sin diseñar, `código: ◐`.** Los héroes bot nacen como fundadores de su Facción NPC cuando el admin la
crea (`crearFaccionNpc`, Doc 5.15.6, `Docs/Coordinacion/02` §4.2) y viven en la partida como cualquier otro
héroe. Hoy la gobernanza NPC ya los usa como a sus residentes de siempre: ocupan cargos, reclutan y salen de
campaña con sus escuadras, pero una campaña no los sitúa en la columna (su `ubicacion` se queda en el
asentamiento). Falta decidir qué hacen en el mundo de BronzeAge: si salen con columnas en persona, defienden
sus plazas, se unen a ejércitos...

Su comportamiento EN batalla no es de BronzeAge: lo maneja la IA de Conquest (CQ-002).
