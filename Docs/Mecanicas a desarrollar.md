# Mecánicas por desarrollar

La **lista única de lo pendiente**. El estado completo de todas las mecánicas del juego —hechas, descartadas y
estas— vive en `Consideraciones/Checklist_Mecanicas.md`, que para lo pendiente solo resume y apunta aquí.
Re-contrastado contra `src/` el 2026-09-09. Lo que solo necesita calibración con números (batch, mediciones) vive en
`Docs/Mecanicas a balancear.md`. Las ideas aparcadas sin plan van a `Docs/Ideas a diseñar.md`.

Cuando una entrada se cierra (diseño **y** implementación), se **borra entera** de este archivo: lo útil que
no esté ya en el canon (`Docs/Game/`) o en una ficha de `Consideraciones/` se mueve allí primero. Mientras una
mecánica se está diseñando, sus acuerdos provisionales pueden vivir aquí como notas.

## Índice

| # | Área | Mecánica | Código hoy |
|---|---|---|---|
| 8 | CARAVANAS | Revamp de caravanas — solo los trozos diferidos (§8.1) | ◐ núcleo hecho; §8.1: solo (b) hecha |
| 20 | TECNOLOGÍA | Tecnología por Eras: lo que falta tras las Eras I-III | ◐ Eras I-III hechas |
| 30 | MILITAR | Batallas con héroes: lo que falta de este lado (canal de tiempo real, sustituir participante, botín, persecución) | ◐ ciclo de `Batalla` fases 1 y 2; falta el canal de tiempo real |
| 32 | POLÍTICA | Fusión de Facciones: consentimiento de la otra y traslado de lo que cuelga de ellas | ✘ sin código |
| 38 | MILITAR | Tope de héroes en asedio: volver a 15 cuando entren jugadores | ◐ 5 mientras se prueba con NPC |

**Descartado** (decisión del usuario): los **landmarks** (antes §10, 2026-10-05), la **curva de progresión inicial gradual** (antes §29, 2026-10-06), el **bloqueo de la explotación del bosque por los campamentos de bandidos** y el **catálogo ampliado de políticas** (los dos, antes en los cabos sueltos §32/§35, 2026-10-06), y también, de los cabos sueltos §33/§35 (2026-10-06): la **riqueza acumulada** por comisiones, el **bonus por distancia en el mostrador**, **cortar rutas como guerra económica**, la **cola de prioridad de reclutamiento** y las **armas de asedio** como mecánica aparte.

**Pospuesto explícitamente, fuera de esta lista:** las **Eras IV y V** de tecnología con todo lo que cuelga de ellas (decisión del
usuario, 2026-10-05; ver §20), el **ciclo de servidor de 12 meses + Maravilla + legado NPC** (antiguo §24, 2026-10-06; su texto está en `Docs/Ideas a diseñar.md`), el **Attack Timer** (Doc 5.6, decidido y aplazado a
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

El diseño de 2026-09-08 recortó seis piezas (la planificación horaria se hizo el 2026-10-02: `prepararCaravana.salirEn`, Doc 3.13.3; la visibilidad por tamaño, el 2026-10-05: Doc 5.12.7) del enunciado original para no inflar el primer pase. Ninguna se
descartó: se decidió su forma y se aparcó. Aquí queda cada una con lo que falta para abordarla.

**a) Cría de animales de arrastre.** Hoy los animales solo se compran con oro. El enunciado quiere obtenerlos
también por cría. El Corral (Doc 1.4/4.2.1) produce *livestock*, que es un recurso distinto — la cría de
bueyes/caballos/camellos necesitaría su propio edificio o una receta que consuma livestock + trigo y tarde
ticks. Falta: decidir si es un edificio nuevo o una función del Corral, el coste y el ritmo, y si cada tipo
de animal exige condiciones (el camello, un bioma; el caballo, quizá un nivel de asentamiento).

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


## 20. Tecnología por Eras: lo que falta tras las Eras I-III

**Estado: Eras I-III en el canon (Doc 6) y en el código (2026-09-29, vía del desarrollo), `código: ◐`.** Plan y
bitácora: `Consideraciones/Tecnologia_Eras_I-III_Definicion.md`. Falta:

- **Eras IV y V — POSPUESTAS** (decisión del usuario, 2026-10-05): el alcance se queda en las Eras I-III. Catálogo, roster y
  logros siguen en la revisión (`Consideraciones/BA-006_Revision_Tecnologia_Eras.md`, EV-1 a EV-7). La subida 4 → 5 la tiene
  que desbloquear una tecnología de la Era V (D54): hasta entonces el código mantiene el techo provisional en el nivel 4
  (`ASCENSO_ASENTAMIENTO.nivelTechoProvisional`), que es lo esperado mientras la Era V esté fuera.
- **Otras vías** (Doc 6.1): conquista (aparece al conquistar una plaza que reclutaba con ella) y comercio (pago a
  otra Facción; ¿la vendedora acepta y fija el precio?). La de los Aedas (venta de tecnología) ya está en el juego, Doc 6.7.
- **Logro de `logistica_campana` (Era IV) — POSPUESTO** con la Era IV: la condición ya existe como consulta pura,
  `caminoCompartidoAbierto` (`engine/redCaminos.ts`); se engancha al catálogo al escribir la Era IV.
- **Equipo de asedio**: `carpinteria_militar` y `trabajos_asedio` se adoptan, pero su equipo no tiene efecto en combate.
- **Clientes**: el panel de tecnología del cliente de administración está hecho (2026-10-02). Faltan, en BronzeAgeClient,
  el mismo panel, y en Conquest las definiciones de escuadra de las 15 tropas nuevas y los modelos de los edificios nuevos (CQ-006).

## 30. Batallas con héroes: lo que falta de este lado

**Estado: canon cerrado (Doc 5.15 y 5.16), `código: ◐`.** El ciclo de `Batalla` con Unity (fases 1 y 2), el Héroe
(fases 1-3 y Herido), la guarnición y el campamento están hechos; su historia vive en el checklist y en
`Docs/Coordinacion/` (BA-001, BA-004). Sin servidores de batalla declarados (`SERVIDORES_BATALLA`) el asedio y el ataque
a un campamento de bandidos se siguen resolviendo con números. Falta, **de este lado**:

- **Canal de tiempo real de la batalla** (fase 3 del ciclo).
- **Sustituir un participante** con una revisión nueva del ticket: hoy no hay quién la dispare.
- **Botín para quien se unió al bando ganador**: hoy va entero a la primera columna.
- **Persecución sin recalcular la ruta** hacia la presa, aunque el Doc 5.12.3 dice que sí: hoy solo marca a quién se
  ataca (bot) o de quién avisar (humano) si se cruzan a 15.

**Esperan a Conquest** (propuestas en `Conquest_prototype/Docs/Coordinacion/propuestas/`): XP y nivel de escuadra y de
héroe, y de dónde salen los puntos de atributo (CQ-001); la IA de escuadras sin héroe y de héroes bot (CQ-002); el
botín y el catálogo de objetos, y con ellos perks, `equipar` y las preguntas que deja el botín (consumibles, equipo
perdido, compatibilidad arma/armadura, fuentes de objetos fuera de batalla) (CQ-004); y las incorporaciones a una
batalla en curso (CQ-005). Mientras tanto, `experiencia` de escuadra suma lo que sumaba la veteranía y `nivel` se queda
en 1. En el cliente de jugador, la pantalla definitiva de crear héroe, el equipo y la ficha de los héroes ajenos
(`BronzeAgeClient`, `docs/Features_Pendientes.md` §0).

## 32. Fusión de Facciones: consentimiento y traslado

**Estado: detectado al cerrar la anexión (2026-10-06), `código: ✘`.** La fusión (Doc 2.6, opción 2) tiene los mismos huecos que tenía la
anexión: **cualquiera de las dos Facciones la ejecuta** sin que la otra acepte (`autorizacion.ts`, `fusionar`), y `engine/fusion.ts` solo mueve
asentamientos y ciudadanos: **ejércitos, caravanas, rutas, Aedas residentes, miradas, relaciones y memoria de niebla** quedan apuntando a las dos
Facciones que desaparecen. Falta decidir con el usuario: el consentimiento (propuesta y respuesta, como la anexión), la votación real del Rey de la
nueva Facción (hoy lo indica quien llama) y qué pasa con las relaciones, la tecnología y la reputación de las dos. `engine/anexion.ts` ya tiene el
traslado de lo que cuelga de una Facción, que se puede generalizar.

## 38. Tope de héroes en asedio: volver a 15 cuando entren jugadores

Mientras se prueba con NPC, el asedio admite 5 héroes por bando como el resto de batallas (decisión del usuario,
2026-09-27; `BATALLA.capacidad.asedio`, Doc 5.15.1). Cuando entren jugadores vuelve a **15 contra 15**: cambiar la
constante y el canon, y avisar a Conquest, que abre la instancia con esa capacidad (`BattleSide.capacidadMaxima`).
