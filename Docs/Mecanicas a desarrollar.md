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
| 8 | CARAVANAS | Revamp de caravanas — solo los trozos diferidos (§8.1) | ◐ núcleo hecho; §8.1 no |
| 9 | ASENTAMIENTO | Eventos de asentamiento | ✘ nada |
| 10 | WORLDGEN | Landmarks reconocibles | ✘ nada |
| 11 | JUGADOR | Progresión de Liderazgo del jugador | ✘ nada |
| 16 | JUGADOR | Qué hace un huésped: vida dentro de una Facción NPC | ✘ nada |
| 18 | INTEL | Taberna + intel como asset con revelado temporal | ✘ nada |
| 19 | POLÍTICA | El mapa político como entidad | ✘ nada |
| 20 | TECNOLOGÍA | Tecnología por Eras: lo que falta tras las Eras I-III | ◐ Eras I-III hechas |
| 21 | POLÍTICA | Los 4 gremios escasos a nivel de servidor | ✘ nada |
| 23 | RECURSOS | Materiales exóticos | ✘ nada |
| 24 | SERVIDOR | Ciclo de servidor de 12 meses + Maravilla + legado NPC | ◐ solo el edificio |
| 27 | AMBIENTACIÓN | Identidad visual y de audio | ✘ nada |
| 29 | ONBOARDING | Curva de progresión inicial gradual | ✘ nada |
| 30 | MILITAR | Batallas con héroes: guarnición, campamento y héroes bot | ◐ canon de campamento, guarnición y conquista; ciclo de `Batalla` fases 1 y 2 (falta el canal de tiempo real) |
| 31 | HÉROE | Modelo de Héroe: uno por jugador y mundo, dueño de los escuadrones | ◐ fases 1-3 y Herido en la rama `heroe-dominio`; faltan perks y equipo |
| 32 | POLÍTICA | Cabos sueltos de diseño político | ✘ sin decidir |
| 33 | COMERCIO | Cabos sueltos de diseño comercial | ✘ sin decidir |
| 34 | SUMINISTRO | La economía no llena el carro de un ejército | ✘ sin decidir |
| 35 | VARIOS | Cabos sueltos de mundo, población y militar | ✘ sin decidir |
| 36 | HÉROE | Comportamiento de los héroes bot en el mundo | ✘ nada |
| 38 | MILITAR | Tope de héroes en asedio: volver a 15 cuando entren jugadores | ◐ 5 mientras se prueba con NPC |
| 39 | MILITAR | Escala de la experiencia de escuadra de Unity | ✘ espera a CQ-001 |
| 40 | MUNDO | Campamentos de mercenarios: enclave neutral, residencia, reclutamiento y mercado | ✘ nada |
| 42 | TECNOLOGÍA | Aedas: difusión y venta de tecnología, residentes y lore | ◐ diseño parcial, sin código |

**Pospuesto explícitamente, fuera de esta lista:** el **Attack Timer** (Doc 5.6, decidido y aplazado a
fase posterior a Fase 0) y el **comercio marítimo / unidades navales** (fuera del alcance de Fase 0 por
diseño). Los ajustes de calibración de mecánicas ya construidas viven en cada ficha de `Consideraciones/` y en
`Preguntas_Abiertas.md`, no aquí. Lo ya cerrado (diseño + implementación) se retira de este archivo por
completo; su estado queda en el checklist.

## 8. Revamp de caravanas — trozos diferidos

El **núcleo está implementado** (2026-09-08, Pasos 1-5): la caravana `comercial` es un contenedor de carros
con animal que deriva capacidad/velocidad, casco vacío + piezas, lanzamiento manual con estado `preparando`,
y escolta sin héroe. Reglas en `Docs/Game/3` §3.13; decisiones, motor y plan en
`Consideraciones/Revamp_Caravanas_Definicion.md`.

### 8.1 Lo diferido — forma diseñada, implementación en un pase posterior

El diseño de 2026-09-08 recortó seis piezas (la planificación horaria se hizo el 2026-10-02: `prepararCaravana.salirEn`, Doc 3.13.3) del enunciado original para no inflar el primer pase. Ninguna se
descartó: se decidió su forma y se aparcó. Aquí queda cada una con lo que falta para abordarla.

**a) Cría de animales de arrastre.** Hoy los animales solo se compran con oro. El enunciado quiere obtenerlos
también por cría. El Corral (Doc 1.4/4.2.1) produce *livestock*, que es un recurso distinto — la cría de
bueyes/caballos/camellos necesitaría su propio edificio o una receta que consuma livestock + trigo y tarde
ticks. Falta: decidir si es un edificio nuevo o una función del Corral, el coste y el ritmo, y si cada tipo
de animal exige condiciones (el camello, un bioma; el caballo, quizá un nivel de asentamiento).

**b) Visibilidad por tamaño.** Las caravanas pequeñas no deberían aparecer en el mapa general salvo que haya
un jugador cerca (regla de niebla actual); las grandes deberían **llamar la atención desde que se preparan**,
al punto de ser visibles para asentamientos hasta cierta distancia, para que salgan a interceptarlas. Es el
gancho de conflicto del enunciado. Falta: un umbral de tamaño (nº de carros y/o carga) que decida si la
caravana entra en la proyección de niebla de otras Facciones y a qué radio, y que eso aplique **durante el
estado `preparando`**, no solo en ruta. Engancha con Doc 5.12 (niebla de guerra).

**c) Inmunidad del camello al desierto — POSPUESTO a fase posterior a Fase 0** (decisión del usuario,
2026-09-09). El camello "no se muere en los desiertos"; buey y caballo sí. Pero no existe un bioma `desierto`
de primera clase (el tipo es `agua|costa|estepa|llanuraFertil|colina|montana|cima`; la aridez del Nilo es
`estepa` de fertilidad baja). Necesitaría o un `BiomaTipo` nuevo, o anclar el "desierto" a `estepa` bajo un
umbral de fertilidad + una regla de *attrition* por tick sobre buey/caballo al cruzarlo. Es un cambio de
worldgen que no aporta a Fase 0; se retoma cuando el mapa tenga terreno árido real. Mientras tanto el camello
es una "opción media" a secas, aceptado.

**d) Catálogo ampliado de carros.** El primer pase trae solo dos carros: el básico (Mercado) y uno
"reforzado" (Carpintería) que solo da más capacidad. El enunciado habla de "varios tipos" fabricables en la
Carpintería. Falta: los ejes que diferencian un carro de otro más allá de la capacidad — resistencia a la
captura (un carro que sobrevive a una derrota), penalización de velocidad (un carro que no frena tanto al
animal rápido), coste en recursos más caros. Se abre cuando la Carpintería tenga niveles internos que lo
justifiquen.

**e) Unificación con el carro de columna.** `Ejercito.suministro` (Doc 5.13) y los carros de una caravana son
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

## 20. Tecnología por Eras: lo que falta tras las Eras I-III

**Estado: Eras I-III en el canon (Doc 6) y en el código (2026-09-29, vía del desarrollo), `código: ◐`.** Plan y
bitácora: `Consideraciones/Tecnologia_Eras_I-III_Definicion.md`. Falta:

- **Eras IV y V**: catálogo, roster y logros siguen en la revisión (`Consideraciones/BA-006_Revision_Tecnologia_Eras.md`,
  EV-1 a EV-7). La subida 4 → 5 la tiene que desbloquear una tecnología de la Era V (D54): hasta entonces el código
  mantiene un techo provisional en el nivel 4 (`ASCENSO_ASENTAMIENTO.nivelTechoProvisional`).
- **Calibrar la X de cada logro** con el batch (`scripts/batch/medidorTecnologia.ts`): cada umbral, en lo que marque su
  contador en su semana objetivo (Doc 6.3).
- **Otras vías** (Doc 6.1): conquista (aparece al conquistar una plaza que reclutaba con ella) y comercio (pago a
  otra Facción; ¿la vendedora acepta y fija el precio?). La de los Aedas va en §42.
- **Logro de `logistica_campana`** (Era IV, D30): la condición ya existe como consulta pura,
  `caminoCompartidoAbierto` (`engine/redCaminos.ts`, red de caminos del Doc 1.6); falta engancharla al catálogo al
  escribir la Era IV. Prioridad: antes de que un servidor llegue a la Era V (semana 26).
- **Equipo de asedio**: `carpinteria_militar` y `trabajos_asedio` se adoptan, pero su equipo no tiene efecto en combate.
- **Clientes**: el panel de tecnología del cliente de administración está hecho (2026-10-02). Faltan, en BronzeAgeClient,
  el mismo panel, y en Conquest las definiciones de escuadra de las 15 tropas nuevas y los modelos de los edificios nuevos (CQ-006).

## 21. Los 4 gremios escasos a nivel de servidor

**Estado: diseño parcial en el canon (Doc 2.10), `código: ✘` — nada.** Los 4 gremios (Comerciantes,
Artesanos, Constructores, Ladrones), escasos a nivel de servidor, con una tirada periódica sujeta a cuatro
requisitos simultáneos (reputación > 90, título de servidor específico, nivel de asentamiento en el máximo,
mantenimiento > 90 %), y su mecanismo de pérdida. El `patioDeGremios` de `constants.ts` es solo una parcela
decorativa del trazado urbano, sin relación con esta mecánica.

Pendiente de decidir (Preguntas_Abiertas §14): valores numéricos de cada requisito por gremio, el título de
servidor asociado a cada uno, el detalle de beneficios de Comerciantes/Artesanos/Constructores, y si hay
margen de gracia antes de perder el gremio.

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

## 27. Identidad visual y de audio

**Estado: `código: ✘` — nada.** Sigilo / estandarte de Facción, identidad visual y de audio de imperios y
títulos. Sin referencias estéticas concretas decididas (micénica, hitita, mesopotámica…). Preguntas_Abiertas
§9. Es sobre todo trabajo de un repo de interfaz aparte, pero la *representación* (qué campo lleva el sigilo,
dónde vive) toca este repo.

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

**Ciclo de `Batalla` con Unity (BA-001), fase 1 de 3 hecha en la rama `heroe-dominio` (2026-09-15).** Con
servidores de batalla declarados (`SERVIDORES_BATALLA`; sin ellos todo sigue con números, decisión del usuario),
un combate con algún héroe humano abre una `Batalla` en vez de resolverse: `atacar` (también contra una plaza, que es
asediarla) y, contra un humano, el ejército bot que llega a su plaza o la columna bot que alcanza a su presa
(`session/batallas.ts`). El ticket se congela con la forma
del contrato, las escuadras quedan reservadas, y lo que interviene se bloquea sin parar el mundo (Doc 5.15.1: la
plaza asediada no abre puertas ni da órdenes, las columnas y la caravana no se mueven ni se pueden atacar, y la
batalla se ve en el mapa en su lugar). Se unen compañeros de Facción mientras quede sitio (`unirseABatalla`),
quien la inició la cancela antes de empezar (`cancelarBatalla`), y si vence un plazo queda `fallida` sin castigo.
Una Facción NPC con algo en batalla no gobierna mientras dura (`ponytail:`, por plaza si se nota). Conquest habla por
`/v1/batallas/*` con su propia credencial (`server/rutas/batallas.ts`: pendientes, ticket, incorporaciones,
asignación, inicio y tokens, validados contra `contratos.schema.json`), y cada jugador recoge su token por
`GET /v1/jugador/partidas/:gameId/batallas/:battleId/asignacion`. Fase 2 hecha el mismo día
(`session/resultadoBatalla.ts`, `POST /v1/batallas/:battleId/resultado`): el checklist de doc 02 §3.3 entero o nada;
cada escuadra queda con sus supervivientes y suma su XP, cada héroe suma su XP y su botín, y el nivel no cambia
hasta que Conquest publique su curva (CQ-001); el bando que pierde queda herido; en mundo abierto pierde la mitad
del carro, que va a la primera columna del que gana; y según el contexto cae la plaza (con el saqueo de siempre),
la caravana o el campamento. Repetir el mismo resultado no cambia nada. Los objetos del botín se aceptan sin
catálogo, como `visual` y a precio 0 (decisión del usuario, 2026-09-15; CQ-004). Falta: el canal de tiempo real
(fase 3); sustituir un participante con una revisión nueva del ticket, que no tiene aún quién la dispare; y el
botín para los que se unieron al bando ganador, que hoy va entero a la primera columna.

Sigue distinto del canon, mientras no haya servidores de batalla: el asedio se resuelve con números
(`iniciarAsedio`, `engine/combate.ts`); lo mismo el ataque a un campamento de bandidos, que se hace con la columna
que llega a él (`atacar`, Doc 1.9; los bots, con `cazarBandidos`). La persecución no recalcula la ruta hacia la presa, aunque Doc 5.12.3 dice que
sí: hoy solo marca a quién se ataca (bot) o de quién avisar (humano) si se cruzan a 15. CQ-002 en Conquest para la
IA de escuadras sin héroe y de héroes bot; CQ-005 para las incorporaciones a una batalla en curso.

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
(`HeroePublico`) y `nombresDeCompaneros`. Y el estado Herido (Doc 5.16.4, 2026-09-14), que sustituye a la Tregua de
columna: 2 minutos para todos los héroes del bando que pierde cualquier batalla; mientras dura no persiguen, no se
les persigue ni entran en batallas, y sus escuadras no combaten. Falta: los perks y `equipar`, que esperan a los
catálogos de Conquest (CQ-004; decisión del usuario 2026-09-14); de dónde salen los puntos de atributo (en Conquest
ningún nivel los da) y la subida de nivel (CQ-001); y en el cliente de jugador (`BronzeAgeClient`, que ya tiene el
panel del héroe y enseña el Herido) la pantalla de crear héroe definitiva, el equipo y la ficha de los héroes
ajenos (su `docs/Features_Pendientes.md` §0).

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
- **Voz en política exterior y protección militar explícita** (Doc 2.5, antes entrada 26): la «voz exterior» se reabrirá con la votación real; la «protección» es lo que ya hacen la guarnición y la defensa de la plaza.
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

## 38. Tope de héroes en asedio: volver a 15 cuando entren jugadores

Mientras se prueba con NPC, el asedio admite 5 héroes por bando como el resto de batallas (decisión del usuario,
2026-09-27; `BATALLA.capacidad.asedio`, Doc 5.15.1). Cuando entren jugadores vuelve a **15 contra 15**: cambiar la
constante y el canon, y avisar a Conquest, que abre la instancia con esa capacidad (`BattleSide.capacidadMaxima`).

## 39. Escala de la experiencia de escuadra de Unity

La curva de nivel de escuadra (Doc 5.16.3, `MILITAR.experienciaParaSubirEscuadra`) está calibrada para el combate con
números, que da 1 de experiencia por victoria. La que trae una batalla de Unity (`xpGanada` por escuadra) entra en la
misma curva: cuando Conquest publique su escala (CQ-001), comprobar que encaja o convertirla al entrar.

## 40. Campamentos de mercenarios

**Estado: diseñado con el usuario el 2026-09-28, `código: ✘`. NO está en el canon: se pasa al implementarlo
(ver "Qué toca del canon", al final).**
Amplía y sustituye en parte D26 (`Consideraciones/BA-006_Revision_Tecnologia_Eras.md`): desaparece el roster
mercenario propio y la cultura. Cifras N, R, X, T, K y topes: PLACEHOLDER.

**Qué es.** Entidad neutral del mundo abierto, miniatura en el mapa, en la que se entra como en un asentamiento.
No es de nadie, no se puede atacar y no desaparece. Lo que está **dentro** es intocable: héroes residentes y
también columnas y caravanas visitantes, sin límite de tiempo. Lo que está en la puerta o fuera, no.

**Aparición.**
- Al menos **uno desde el día 1**. Los demás, en puntos sin reclamar con zonas de influencia de al menos **N
  Facciones distintas** a menos de **R**; tope por servidor y distancia mínima entre campamentos.
- Pegado a un bosque si lo hay (preferencia, no requisito; los bandidos viven *dentro* del bosque).
- **Enclave**: ninguna zona de influencia lo absorbe; no tiene zona propia.
- Niebla: como un camino, visto una vez queda en la memoria.

**Forma.** Empalizada con una sola entrada y layout fijo; no crece. Pocas variantes, elegidas por el **origen**
del campamento, que solo es aspecto (sin cultura). Taberna, viviendas, mercado y una combinación aleatoria de
barracón, galería de tiro y caballeriza. Sin extractores, fabricación ni zona de influencia.

**Residencia (adiós al huérfano).**
- Un asentamiento aloja sin límite a los héroes de su Facción; un campamento, a héroes de **cualquier** Facción,
  también sin límite. Cualquier héroe puede residir en él aunque su Facción tenga asentamientos. Sin guarnición ni
  cargos.
- Si una Facción humana pierde su último asentamiento, sus héroes van con su campamento al campamento mercenario
  **más cercano** al que perdieron. El estado de huérfano (Doc 0, 5.15.5) desaparece.
- Los héroes de una Facción NPC que se queda sin asentamientos **desaparecen**.
- **Almacén personal** para todos los héroes: pequeño, tope fijo, **viaja con su campamento** en cada cambio de
  residencia (lo único que se mueve sin caravana).
- La Facción sin asentamientos sigue existiendo y puede **refundar**: fundación "de supervivencia", más barata,
  pagada juntando los almacenes personales de los héroes residentes en el **mismo** campamento que el fundador
  (los demás se mudan antes con `cambiarResidencia`); cada héroe decide si aporta y cuánto. Lo reunido sale del
  campamento como columna con su caravana de fundación.

**Reclutamiento.**
- Solo las tropas que permiten los edificios de ese campamento.
- Control de tecnologías propio: desbloquea cada una **el último**, al instante y gratis, cuando la tiene un X %
  de las Facciones humanas vivas o T horas después de la primera, lo que llegue antes. Así se recluta aquí lo que
  una Facción no alcanza por edificio o materiales. Hasta que exista el árbol (entrada 20), ofrece lo de la Era.
- Población propia que crece hasta el tope de sus viviendas; sin mantenimiento ni comida.
- Cuesta **oro + población del campamento**. Precio: base → recargo del campamento (más caro que en casa) →
  recargo por reputación (Doc 2.7) → descuento si la Facción no tiene asentamientos.
- Paga el residente con su almacén personal; con la columna delante, con el carro, y el escuadrón se une al héroe
  si le cabe en el Liderazgo. Un escuadrón por tropa (Doc 5.8): si ya la tiene, repone.
- Para quien no tiene asentamientos, el botín de bandidos es una fuente pequeña de oro.

**Mercado.**
- **Solo vende** recursos básicos y materiales ya desbloqueados en el mundo, con stock limitado. El oro cobrado
  **se destruye** (sumidero).
- Entrega como todo el comercio: caravana desde el asentamiento de la Facción, que vuelve a él. El residente
  compra directo a su almacén personal.
- Reposición barata para el tick: cada trueque o venta cerrada suma lo intercambiado a un **contador global por
  bien**; cada **K horas** cada campamento repone en proporción a esos contadores, con tope por bien, y los
  contadores vuelven a cero. Precio = tabla base + margen.

**Taberna.** Por ahora, lugar neutral de rumores; la versión completa va con la entrada 18.

**Afecta a Conquest:** entidad nueva en `src/contratos/v1/` y escena con las variantes de layout.

**Qué toca del canon al implementarlo:**
- **Doc 0 (Glosario):** entrada nueva *Campamento de mercenarios*; *Huérfano* se borra (o queda como histórico)
  y *Campamento* (del héroe) dice que puede estar en un campamento mercenario; entrada *Almacén personal* del héroe.
- **Doc 1:** sección nueva junto a 1.9 (bandidos) con aparición, enclave y forma; 1.2 (Fundación) añade la
  fundación de supervivencia pagada con almacenes personales y salida desde el campamento.
- **Doc 2:** 2.5 (Ciudadanía / `cambiarResidencia`) admite residir en un campamento mercenario sin ser de su
  Facción, y el almacén personal viaja con el campamento; 2.7 (Reputación) añade el recargo al reclutar allí.
- **Doc 3:** 3.3 (Órdenes de mercado) añade el vendedor neutral, el sumidero de oro y la reposición por
  contadores globales; 3.10 (Combate de caravanas) aclara que dentro del campamento no se ataca.
- **Doc 5:** 5.8 (Roster) añade el reclutamiento en el campamento, su desbloqueo de tecnología y el precio;
  5.12.3 (qué se puede atacar): lo que está dentro es intocable; 5.12.8 (Memoria): se recuerda como un camino;
  5.15.2 (Campamento del héroe) y 5.15.5 (Cuando cae el asentamiento): se muda al campamento mercenario más
  cercano en vez de quedar huérfano; 5.15.6 (Facciones NPC): sus héroes desaparecen. Revisar también las otras
  menciones a 5.15.5 (el párrafo "Al conquistar" del principio del Doc 5, 5.12.4 y 5.12.9).
- **BA-006 (Consideraciones):** D26 queda superada; en su lista de canon, "Doc 1.9 (o sección nueva)" y la
  entrada de glosario *Campamento de mercenarios* pasan a apuntar a esta entrada.

## 42. Aedas: difusión y venta de tecnología, residentes y lore

**Estado: diseño parcial (canon Doc 6.2-6.5; `Consideraciones/BA-006_Revision_Tecnologia_Eras.md` D31 y D55),
`código: ✘`.** Lo único que existe es la narración de cambios de título (`engine/titulos.ts`, Doc 2.9). Aquí se reúne
todo lo decidido sobre ellos hasta el 2026-09-29.

**Qué son.** NPCs viajantes, de dos clases, itinerantes y residentes, que dan acceso a tecnología y además narran el
lore del servidor (Doc 6.2, 6.3, Glosario: Aedas y Poetas son el mismo rol).

**Itinerantes:**
- **Difunden la tecnología (D55).** Toda tecnología está oculta hasta que se cumplen sus condiciones de aparición; a
  la Facción que cumple su hito le aparece sola, y para el resto sigue oculta. Los Aedas **conocen una tecnología
  desde que alguien la desbloquea, pero con un retraso**, que es la ventaja del primero. Al llegar a un asentamiento
  de otra Facción, le revelan la tecnología, los requisitos que le faltan y quién la desbloqueó.
- **La venden (D31, Doc 6.2).** Pagándole oro, la Facción se salta su hito, **nunca el logro del servidor**. El pago
  de adopción (tarifa por Era, D32) se paga igual.
- **Cantan los logros del servidor (D31):** son públicos y quedan en la crónica, pero no dicen qué tecnología
  desbloquean.

**Residentes (Doc 6.2):**
- Se quedan en un asentamiento con **Nobleza** y se van si la pierde; su número depende del tamaño o nivel del
  asentamiento, o de políticas. Con el Palacio desde el nivel 2 (Doc 4.2.1), puede haberlos desde el nivel 2.
- Desbloquean tecnología con una **épica por capítulos** que avanza con eventos inspiradores (por ejemplo,
  batallas). Ejemplo de la propuesta BA-006 original: "Canto de las lanzas largas" (conocer una formación, fabricar
  armas de asta, instrucción, combatir en formación, armas combinadas → `reforma_macedonica`). Los eventos no pueden
  ser acciones repetibles para farmear: hechos relevantes con límites por batalla o periodo.
- Dan felicidad, crecimiento de nobleza, desbloqueo de tecnologías y prestigio.
- Una Facción con reputación muy baja los atrae peor (Doc 2.7, 6.4).

**Lore (Doc 6.3):** narran títulos (ya en el código), asentamientos que caen, tecnologías descubiertas y guerras,
consultable en una interfaz propia y como eventos. No afecta al balance. El gremio de Comerciantes puede dar "rutas o
Aedas comerciales especiales" (Doc 2.10).

**Sin decidir:**
- Cuánto dura el retraso de la ventaja del primero.
- Cuántos Aedas itinerantes hay por servidor, cómo se mueven (caminos, rutas de caravana, al azar), si se ven en el
  mapa y si se les puede atacar o retener.
- Precio en oro de la venta; qué tecnologías vende un itinerante (Doc 6.2 dice "básicas") y cuáles solo llegan por la
  épica de un residente; cómo encaja la épica con el modelo de logro + hito (D30).
- Cupo de residentes por nivel.

**Qué toca del canon al implementarlo:** Doc 6.2 (reescrito con D55 y la venta), Doc 6.5 ("Fase 0 sin Aedas") fuera,
Glosario (Aedas / Poetas).
