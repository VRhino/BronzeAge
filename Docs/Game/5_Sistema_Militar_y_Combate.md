# 5. Sistema Militar y de Combate

## 5.1 Principio general: héroe-comandante liderando tropa (heredado de Iberia)
El jugador NO combate individualmente contra multitudes — asume el rol de HÉROE/COMANDANTE que lidera una tropa de N unidades NPC. Regla de oro: Tropa > Héroe. Puede llevar más de un regimiento a una batalla pero solo despliega uno a la vez, intercambiables en puntos tácticos de reabastecimiento dentro del combate.

## 5.2 Modalidades de batalla instanciadas (heredado de Iberia)
El combate ocurre en INSTANCIAS separadas del mapa global (aunque se desencadenen en él), límites simétricos fijos, 2 bandos (Atacante/Defensor), sin empates.

1. **Asedio de asentamientos**: atacante captura banderas/áreas vitales antes de que expire el tiempo; defensor gana resistiendo. Solo defienden miembros de la Facción soberana del nodo o Facciones aliadas/vasallas confirmadas. Mortalidad severa (permadeath). Jugadores en cola desde mundo abierto rellenan la instancia dinámicamente según bajas.
2. **Mundo abierto**: choque de patrullas/ejércitos. Bandera/campamento transitorio; quien la pierde se retira, deja loot, sufre debuff temporal "Herido". **No se declara: se produce por PROXIMIDAD entre dos ejércitos que se cruzan en el mapa** (5.12).
3. **Defensa/intercepción de caravanas**: combate asimétrico móvil (ver Doc 3, sección 3.10). Igual que el anterior, se dispara por proximidad de un ejército a una caravana (5.12).
4. **Entrenamiento/matchmaking** (POSPUESTO a fase posterior a Fase 0/1): 15v15 puro, sin permadeath, para probar tácticas. CONFIRMADO: pospuesto de forma explícita, requiere factores no disponibles en Fase 0 (mismo motivo que Attack Timer, ver 5.6).

### 5.2.5 Resolución numérica y varianza de combate

Regla vigente, que **hasta ahora solo existía en el código** y no estaba escrita en ningún documento (`resolverCombate`, `engine/combate.ts`; `MILITAR.varianzaCombate = 0.15`):

Se suma el poder de cada bando y **se multiplica cada uno por un factor aleatorio de ±15%**. Gana quien saque el producto más alto. Cuanto más ajustado el resultado, más bajas sufre también el ganador; el perdedor siempre pierde más. El azar entra **una sola vez por bando y por combate** — no hay tiradas por unidad ni por ronda.

**Qué implica ese ±15% en la práctica**: los multiplicadores van de 0.85 a 1.15, así que el cociente entre ambos bandos va de 0.74 a 1.35. Es decir, **un atacante necesita un 35% más de poder para tener la victoria garantizada**; por debajo de eso siempre puede perder. El comentario del código lo justifica como "romper empates", pero 15% por bando es varianza de combate real, no un desempate.

> **DECISIÓN ABIERTA (2026-09-02): revisar esta varianza a la baja o retirarla.** El movimiento de ejércitos multiplica el coste de una mala tirada —marchar cuesta tiempo real, deja la ciudad indefensa, vacía el almacén y las bajas son permanentes— y el juego ya tiene diseñada una fuente de incertidumbre mejor: la niebla de guerra (`Docs/Mecanicas a desarrollar.md` §12), que es incertidumbre **reducible jugando bien** en vez de un dado. Análisis completo y las tres opciones en `Consideraciones/Movimiento_Ejercitos_Definicion.md` §1.4.

## 5.3 Formaciones y cohesión táctica (heredado de Iberia)
- Romper formación penaliza duro (ej. arqueros dispersos -30% precisión, escuderos aislados -20% defensa, lanceros sin formación pierden bono anti-carga). Flanquear/aislar formaciones enemigas es táctica válida.
- COHESIÓN ENTRE JUGADORES: varios jugadores anclando una línea juntos ganan Defensa Compartida, resistencia a rotura de moral, regeneración lenta de HP.
- Jugador novato: útil desde el día 1 con infantería básica de escudo barata ("carne de línea") mientras veteranos flanquean.

## 5.4 Ciclo de vida de unidades (heredado de Iberia)

**Los escuadrones son del JUGADOR, no del asentamiento** (a petición del usuario, 2026-09-01 — cierra una ambigüedad que el modelo arrastraba: el asentamiento los contenía, así que parecía dueño de ellos). El asentamiento es donde están **apostados**, no quien los posee. Consecuencias:

- Un asentamiento **conquistado** hace que sus jugadores **pierdan los escuadrones que estaban apostados ahí**; conservan solo los que llevaban encima en campaña (5.12). Los perdidos **no pasan al conquistador** — son personales de otro jugador, no botín transferible.
- Sacar escuadrones a campaña los quita de la guarnición **de verdad**: dejan de defender y dejan de comer del almacén (5.13).
- **Un jugador al que le conquistan su asentamiento estando de campaña queda HUÉRFANO** (decisión del usuario, 2026-09-02): conserva los escuadrones que lleva encima, pero se queda sin residencia — sin sitio donde reabastecer, reclutar ni volver. Sigue huérfano **hasta que entre en una Facción nueva que tenga asentamiento**. No es una derrota definitiva: es un estado del que se sale por la vía política (Doc 2.5, ciudadanía), no por la militar.

Ciclo de vida propiamente dicho:

- PERMADEATH individual (excepto modo entrenamiento): bajas son permanentes.
- El SQUAD (nombre, nivel veterano) persiste aunque el regimiento sea aniquilado — se puede rellenar con nuevos reclutas conservando el progreso.
- DESERCIÓN POR HAMBRE: tropas consumen raciones continuamente; sin suministro, la moral colapsa y desertan permanentemente (mismo efecto que perderlas en combate). **Es la misma regla en guarnición y en campaña** — solo cambia de qué despensa se come (5.13).
- **Población civil come ANTES que las Tropas** (rediseño, a petición del usuario — pensando en escala
  multijugador real: cantidad de jugadores por asentamiento y presión militar PvP crecen con el servidor,
  mientras que la producción de comida está acotada por espacio construido; ver
  `Consideraciones/NPC_Gobernanza_Facciones_Controladas.md` §"Abierto" para el diagnóstico completo).
  `avanzarNutricionPoblacion` corre antes que `avanzarMantenimientoTropas` en el tick (`engine/simulation.ts`)
  — antes era al revés. Los civiles son quienes producen (Granja/Cantera/Fundición/...); las Tropas no
  producen nada, así que bajo escasez sostenida el shock lo absorbe primero la parte del sistema que no es
  productiva: la moral militar colapsa y empieza la deserción MUCHO antes de que la nutrición civil llegue a
  comprometerse — para que los civiles se quedaran sin nada, la producción tendría que caer por debajo de
  SOLO su propio consumo, un escalón de escasez peor que el que ya habría vaciado el ejército.
- **Reclutar exige reserva de trigo proyectada** (`reclutarTropa`, `engine/tropas.ts`): antes de aceptar un
  reclutamiento o reposición, el trigo en almacén debe cubrir `RESERVA_CONSTRUCCION.horizonteTicksComida` (8)
  ticks del consumo YA PROYECTADO CON la tropa nueva sumada (civiles + tropas existentes + la que se está
  reclutando). Es una regla del MOTOR, no un heurístico del NPC — igual que Vivienda acota cuánta población
  civil puede aparecer, esto acota cuánta tropa puede sostenerse, sin importar cuántos jugadores residen en el
  asentamiento ni cuánto pesants tenga cada uno disponible en el momento. Reemplaza un throttle anterior que
  vivía solo en la gobernanza NPC (1 residente reclutando por tick mientras el asentamiento estuviera en
  nivel 1, los 5 a la vez desde nivel 2) — ese interruptor binario por nivel quedó retirado: con la reserva
  real, si varios residentes intentan reclutar el mismo tick, los primeros agotan el margen y el resto falla
  limpio, sin necesidad de un tope artificial.
- **El pool de reclutamiento es población MENOS la ya ocupada en producción, no la población total**
  (`poblacionDisponibleParaReclutar`, `engine/asentamientoQuery.ts`, consumida por `reclutarTropa`,
  `engine/tropas.ts`). La reserva de trigo del punto anterior protege el CONSUMO proyectado, pero no evitaba
  que reclutar sacara pesants que ya estaban cubriendo `trabajadoresRequeridos` de Granja/Cantera/Leñera/
  minas/Corral (`ratioManoObra`) — ni artesanos que ya cubrían Fundición/Curtiduría/Armería/Carpintería
  (`ratioManoObraArtesanos`). La UI ya mostraba el número correcto ("Pool de pesants para reclutamiento",
  `manoObraInfo.excedente`) sin que el motor lo hiciera cumplir; ahora el motor lee exactamente ese número. Es
  simétrico entre pesants y artesanos, y aplica igual a reclutamiento manual y al de la gobernanza NPC.
  Medido en batch: de la mayoría del colapso de transición a nivel 2 que quedaba abierto (11/20 asentamientos
  de prueba), este arreglo por sí solo lo baja a 3/20 — detalle en
  `issues/granjas_no_escalan_con_poblacion.md`, sección "Tercera continuación".

## 5.5 Doble carril de progresión (ver también Doc 4, sección 4.1)
- Carril COMBATE REAL (Pesants + Artesanos): tropas Tier bajo/medio, deben veteranizar combatiendo de verdad.
- Carril PROGRESIÓN PLANA (Nobleza): conversión instantánea a élite con equipo disponible.

## 5.6 Attack Timer (heredado de Iberia, pospuesto a fase posterior a Fase 0)
Asedios FORMALES en ventanas limitadas (ej. 2 veces/semana, horario fijo, ~1h de duración). Ataques logísticos (mundo abierto, caravanas) libres 24/7. CONFIRMADO: pospuesto a Fase 1+ de forma explícita — requiere factores no disponibles en Fase 0 (infraestructura de instanciado multijugador programado, sistema de colas/horarios), no implementable en el prototipo de datos puros.

## 5.7 Reclutamiento por infraestructura física (adaptado a la Edad de Bronce)
No hay árbol tecnológico abstracto — el tipo de unidad reclutable depende de la infraestructura física y NPCs residentes del asentamiento.

**Oficios/NPCs especialistas:**
- Broncista/Fundidor: funde cobre+estaño → bronce; fabrica armas.
- Curtidor-Armero: cuero y bronce laminado → armaduras, escudos, cascos.
- Carpintero: astas de lanza, ejes de carro de guerra, estructuras de escudo.
- Constructores: mantenimiento de murallas y defensas pasivas.
- Maestro de Armas: entrena y mejora estadísticas de tropas.
- Sacerdote (ya es cargo de jugador, no NPC nuevo): buff de área.

**Edificios** (catálogo completo con costos/recetas en Doc 4.2.1, rediseño Fase 0):
- Centro Urbano: reclutamiento de la defensa mínima (Milicia de lanceros, ver roster 5.8) — a petición del usuario, decisión post Sprint 6: la tropa de entrada NO depende de Barracón (que exige nivel de asentamiento + política del General antes de existir siquiera), sino del único edificio que nace `activo` con el asentamiento desde el tick de fundación, sin cola de construcción ni gate. Así todo asentamiento puede defenderse desde el minuto uno, así sea con la unidad más débil del roster.
- Fundición: fabricación de lingotes de cobre/estaño/bronce — auto-construcción, ya no manual (ver Doc 4.2).
- Curtiduría: tratamiento de cuero (livestock → cuero → cuero curtido → cuero de calidad).
- Armería: fabricación de armas y armaduras a partir de lingotes y cuero — insumo directo del reclutamiento de Barracón/Galería de tiro.
- Carpintería: recluta armas de asedio (ariete, torre de asedio) y habilita construir/mejorar Palacio, Armería, Barracón y Galería de tiro de nivel 2+.
- Barracón: reclutamiento de tropas cuerpo a cuerpo (ver roster 5.8). Adición MANUAL de Gobernador/Maestro de Obras a la cola (Doc 4.2) — la política de desbloqueo que lo gateaba antes se retiró (Doc 4.4).
- Galería de tiro: reclutamiento de tropas a distancia (ver roster 5.8). Adición MANUAL de Gobernador/Maestro de Obras a la cola (Doc 4.2) — la política de desbloqueo que lo gateaba antes se retiró (Doc 4.4).
- Mina de cobre: extractor de cobre (Doc 1.4/4.2.1) — único extractor de cobre del juego, insumo obligatorio de Fundición para todo equipo de bronce. Extractores finitos con reemplazo automático al agotarse (mecanismo completo en Doc 4.2, incluye número fijo por tipo — NO ligado al nivel de asentamiento).
- Gran Fundición: único edificio de tier élite, sin cambios respecto a la versión ya implementada (queda para iteraciones posteriores la integración con la nueva Fundición).
- PENDIENTE: Establos (ligados a carros de guerra) no tiene equivalente en el catálogo del rediseño — las unidades de carro de guerra del roster anterior (ver 5.8) quedan sin edificio de reclutamiento definido. RESUELTO: POSPUESTO a Fase 1 de forma EXPLÍCITA e INTENCIONAL (no es un vacío accidental) — los carros de guerra no se implementan en Fase 0.

**Materiales limitantes (clave anti-"ejército meta universal"):**
- COBRE: relativamente abundante.
- ESTAÑO: raro, concentrado en pocas ubicaciones (base histórica real: la disrupción de rutas de estaño es una teoría real del colapso de la Edad de Bronce). El bronce de calidad — y por tanto las tropas de tier alto — depende del acceso a estaño.

## 5.8 Roster de tropas (rediseño Fase 0: reclutamiento por edificio + nivel interno, ver Doc 4.2.1)

**IMPLEMENTADO** (ver `constants.ts` `TROPAS_RECLUTABLES`, `engine/tropas.ts` `reclutarTropa`). Terminología (Doc 0/Glosario, **precisada 2026-09-02**): son tres conceptos y la jerarquía de entidades es **Jugador → Escuadrón → Unidad**. Una **tropa** es el TIPO (ej. "Lanceros con escudo de mimbre") y las tablas de abajo son su catálogo; un **escuadrón** es la instancia que un jugador posee y comanda; una **unidad** es cada soldado individual dentro de él. El número de unidades **NO lo elige el jugador** (ver "Unidades por defecto" más abajo) — cada escuadrón reclutado añade siempre el mismo tamaño fijo.

El roster ya no se organiza por Tier abstracto (inspiración Total War Troy, foco Egeo/Grecia) — cada tropa se recluta en Centro Urbano, Barracón o Galería de tiro, según el NIVEL INTERNO del edificio (1-3, ver Doc 4.2.1; Centro Urbano no tiene niveles), pagando el equipo correspondiente fabricado en Armería (ver catálogo completo de recetas en Doc 4.2.1). "Costo" en las tablas de abajo es POR SOLDADO — el costo real de reclutar es ese valor × "Unidades". Cada tropa tiene además un `poderBase` (PLACEHOLDER, ver más abajo) usado en el cálculo de combate en vez del poderBase por tier del roster anterior.

**Centro Urbano (defensa mínima, sin edificio dedicado) — carril Pesants + Artesanos:**

| Tropa | Costo (por soldado) | poderBase | Unidades |
|---|---|---|---|
| Milicia de lanceros | 2 Madera (en bruto, sin pasar por Armería) | 2 | 25 |

**Barracón (cuerpo a cuerpo) — carril Pesants + Artesanos, combate real (Doc 4.1/5.5):**

| Nivel | Tropa | Costo (por soldado) | poderBase | Unidades |
|---|---|---|---|---|
| 1 | Lanceros con escudo de mimbre | 1 Arma de Madera | 3 | 20 |
| 1 | Espadachines de espada corta de cobre | 1 Arma de Cobre + 1 Armadura Básica | 4 | 20 |
| 2 | Hacheros ligeros | 1 Arma de Bronce + 1 Armadura Básica | 7 | 18 |
| 2 | Espadachines con espadas y escudos de bronce | 2 Arma de Bronce + 1 Armadura Intermedia | 9 | 18 |
| 3 | Lanceros pesados micénicos (escudos grandes) | 2 Arma de Bronce + 2 Armadura Intermedia | 14 | 15 |
| 3 | Hacheros armados (armadura media) | 1 Arma de Bronce + 1 Armadura Intermedia | 12 | 15 |

**Galería de tiro (a distancia) — carril Pesants + Artesanos, combate real:**

| Nivel | Tropa | Costo (por soldado) | poderBase | Unidades |
|---|---|---|---|---|
| 1 | Honderos (escaramuzadores) | 1 Arma de Madera | 5 | 25 |
| 2 | Escaramuzadores con jabalina | 1 Arma de Bronce + 1 Armadura Básica | 8 | 20 |
| 2 | Arqueros | 1 Arma de Bronce + 1 Armadura Intermedia | 9 | 25 |
| 3 | Arqueros con arco compuesto | 3 Arma de Bronce + 2 Armadura Intermedia | 15 | 20 |

`poderBase` es PLACEHOLDER: no estaba en el diseño original (solo equipo/nivel), interpolado a partir de la progresión ya existente del roster anterior (3 → 6 → 12 → 25 en 4 tiers) repartida en estas 10 tropas a lo largo de 3 niveles — pendiente de calibración por simulación.

**Unidades por defecto** (`unidadesPorDefecto`, a petición del usuario — corrige una contradicción real con la propia definición de "tropa" de arriba, que ya decía "se recluta de una vez" mientras el motor aceptaba una `cantidad` libre): `unidadesPorDefecto` es el TOPE del escuadrón, el jugador nunca elige cuántos soldados reclutar. `costoEquipo` sigue siendo por soldado. Reclutar desde cero cuesta "Costo (por soldado)" × "Unidades" de la tabla — ej. Milicia de lanceros cuesta 2 Madera/soldado × 25 = 50 Madera. Si el escuadrón ya existe y está por debajo del tope (bajas de combate, Doc 5.4), reclutar de nuevo REPONE solo las unidades que faltan hasta el tope, al mismo costo por soldado — no es un bloque nuevo completo (ver el párrafo "Escuadrón por jugador" más abajo). Cifras PLACEHOLDER sin calibrar por simulación todavía. En la UI (pestaña Guerra), el segmento "Info:" bajo el selector de tropa muestra el desglose (por soldado y costo de ESTE reclutamiento, que puede ser parcial) antes de confirmar.

**Milicia de lanceros y Arma de Madera** (post Sprint 6, decisión real del usuario tras diagnóstico por simulación — ver `Correcciones_Durante_Desarrollo.md` #32): antes, las 3 tropas de nivel 1 exigían la cadena metalúrgica o del cuero COMPLETA (Mina de Cobre/Corral → Fundición/Curtiduría → Armería), y menos del 6% de los asentamientos nace con un nodo de cobre o livestock dentro de su zona inicial — la primera tropa tardaba una mediana de ~196 ticks y solo la conseguía el 5.7% de los asentamientos en un batch de 200 runs × 900 ticks. "Milicia de lanceros" se paga con madera en bruto (sin pasar por Armería) y es deliberadamente la más débil del roster (poderBase 2) — existe para que el bucle de juego arranque pronto, no para ganar batallas. "Lanceros con escudo de mimbre" y "Honderos" se recostearon de Arma de Cobre/Armadura Básica a Arma de Madera (corrige además una incoherencia temática: un escudo de mimbre pagado con tecnología de cobre, y una honda pagada con armadura de cuero). Con el cambio shipeado, el mismo diagnóstico sube a 44.3% de asentamientos con al menos una tropa, mediana tick 21.

**Milicia de lanceros pasa de Barracón a Centro Urbano** (corrección posterior, a petición del usuario — la defensa mínima seguía dependiendo de un edificio con su propio gate: nivel de asentamiento indirecto + política "Construir Barracón" del General + cola de construcción, ver `Correcciones_Durante_Desarrollo.md` #36): "Milicia de lanceros" ahora se recluta vía Centro Urbano, el único edificio que nace `activo` con el asentamiento desde el tick de fundación (Doc 1.3), sin cola ni política. El único requisito que queda es ser residente del asentamiento (Doc 2.5 — ya NO se exige un General asignado, ver párrafo "Escuadrón por jugador" más abajo) y los 25 soldados de población + 50 madera del escuadrón — verificado en el navegador: reclutable en el tick 11 (en cuanto la población alcanza 25 pesants desde los 20 iniciales), muy por delante de cuándo Barracón podría siquiera empezar a construirse.

**Nobleza (progresión plana) — YA NO recluta tropas** (decisión real del usuario, ver `Correcciones_Durante_Desarrollo.md` #30 — corrige el texto anterior de esta sección, que seguía describiendo el reclutamiento vía Gran Fundición como vigente): Nobleza sigue existiendo sin cambios como clase de población (crecimiento, requisito de Palacio, ciudadanos mínimos), pero se retiró por completo la posibilidad de convertirla en tropa. El único carril de reclutamiento militar en Fase 0 es el de equipo (Centro Urbano/Barracón/Galería de tiro), abierto a Pesants y Artesanos.

**Relación entre tropas ya reclutadas y el edificio que las produjo**: CONFIRMADO — NO existe ninguna relación posterior al reclutamiento. Una vez una tropa está reclutada y en el mundo, es independiente del edificio (Barracón/Galería de tiro) que la originó. Si el edificio sube de nivel después, los escuadrones ya existentes NO se ven afectados de ninguna forma — ni mejoran ni empeoran. "Mejorar" solo significa poder reclutar tropas nuevas de mayor nivel a partir de ese momento (ver párrafo de RESUELTO más abajo).

**Regla confirmada (a petición del usuario, cierra una ambigüedad real detectada auditando el código)**: una tropa reclutada JAMÁS cambia de identidad/tipo al ganar experiencia. "Milicia de lanceros" que sube de veteranía se queda siendo "Milicia de lanceros" con más poder — nunca pasa a ser "Hacheros" ni ningún otro `tropaId`. El pool de origen (Pesants/Artesanos) tampoco cambia. La veteranía da un bonus de poder continuo al MISMO escuadrón (`poderBase * (1 + veterania * bonusVeteraniaPorPunto)`, fórmula sin cambios, ver `poderEscuadron` en `engine/combate.ts`) — "mejorar" de tropa solo ocurre reclutando una tropa DISTINTA y mejor cuando Barracón/Galería de tiro suba de nivel interno; eso crea un escuadrón nuevo, no transforma el existente.

**Auditado contra el código real, código muerto RETIRADO**: el texto anterior de esta sección decía que el ascenso automático de tier por veteranía (`ascenderTierSiCorresponde`, `TROPA_CATALOGO`, `ASCENSO_TROPA`) "convivía" con el sistema de tropas de equipo, aplicándose a Artesanos/Nobleza. Eso no era cierto — Nobleza no recluta tropas en absoluto (ver arriba), y Artesanos recluta por el mismo carril `reclutarTropa` que Pesants, que SIEMPRE asigna `tropaId`. Como `ascenderTierSiCorresponde` se desactivaba explícitamente en cuanto `tropaId` estaba presente, ningún escuadrón real pasaba por esa rama — era código muerto. Se retiró por completo: `TROPA_CATALOGO`, `ASCENSO_TROPA`, `ascenderTierSiCorresponde` y el campo `Escuadron.tier` (siempre valía 1, nunca cambiaba) ya no existen en el código. `Escuadron.tropaId` pasó de opcional a OBLIGATORIO (único origen real de escuadrones). La UI (tabla de escuadrones en Asentamientos, panel militar en Guerra) mostraba "Tier 1" de forma engañosa para toda tropa sin excepción — ahora muestra el `nivelRequerido` real de la tropa reclutada (Doc 5.8, catálogo `TROPAS_RECLUTABLES`). Verificado en el navegador reclutando Milicia de lanceros: se muestra "Nivel 1" correctamente en ambas vistas, sin errores de consola.

**Escuadrón por jugador, no por asentamiento** (RESUELTO — bug real reportado por el usuario en la UI de Combate: dos escuadrones de 25 aparecían fundidos en un solo chip de 50, inseleccionable de forma independiente y que además fallaba al validarlo en el ataque): `reclutarTropa` fusionaba por `tropaId` a nivel de ASENTAMIENTO (`asentamiento.escuadrones.find(e => e.tropaId === tropaId)`), así que dos jugadores reclutando la misma tropa en el mismo asentamiento —o el mismo jugador reclutando dos veces— terminaban compartiendo un único escuadrón. Causa raíz real, más de fondo que el bug puntual: el modelo correcto (Doc 2.1/2.5, a petición del usuario) es que CADA jugador residente tiene su PROPIO escuadrón de cada tropa — el jugador reside en un solo asentamiento (Doc 2.1) y ahí solo puede tener sus propias tropas, nunca las de otro. `Escuadron` ahora lleva `jugadorId` (domain/types.ts) y `reclutarTropa` empareja por `jugadorId` + `tropaId` (engine/tropas.ts): cada jugador tiene como mucho un escuadrón por tropa, tope `unidadesPorDefecto`; reclutar de nuevo repone el faltante si hay bajas (ver párrafo "Unidades por defecto" arriba), nunca crea un segundo escuadrón del mismo jugador. Reclutar deja de exigir un General asignado (Doc 2.2 vs 2.5, ganó 2.5): solo exige que el jugador sea residente (fundador o casa comprada, Doc 2.5) — y por eso `comprarCasa` (engine/faccion.ts) ahora rechaza que un jugador resida en más de un asentamiento a la vez, invariante que antes no existía. La UI de Combate (chips de escuadrones propios) agrupa los chips por jugador: seleccionar 1+ chips bajo cada jugador representa "estos jugadores se unen al combate, cada uno con las tropas marcadas" (ver Doc 5.2/5.10, combate multi-escuadrón ya soportado sin cambios).

PENDIENTE:
- Establos / unidades de carro de guerra (Carros escaramuzadores, Carros de guerra reforzados del roster anterior): sin edificio de reclutamiento definido en el rediseño — Carpintería solo cubre armas de asedio (ariete, torre de asedio), no carros. Queda sin resolver si se retiran de Fase 0 o necesitan su propio edificio.

## 5.9 Exilio como política de soberanía (heredado de Iberia, ver también Doc 2.8)
El Gobernador puede decretar exilio de jugadores enemigos de su territorio; coste de reubicación (pérdida parcial de materiales, desplazamiento físico para recuperarlos).

## 5.10 Fuera de alcance de Fase 0
Todo lo instanciado/visual (combate real en escena, formaciones renderizadas, modo entrenamiento, attack timer con UI) pertenece a Fase 1+. En Fase 0, el combate se resuelve como CÁLCULO/LOG DE TEXTO (quién gana, bajas resultantes), sin representación gráfica.

**Precisión (2026-09-01):** lo que queda fuera es la ESCENA de batalla, no el mapa. El **movimiento de ejércitos por el mapa del mundo SÍ es Fase 0** (5.12): es desplazamiento sobre el mapa continuo con la misma maquinaria que ya mueve caravanas, no una instancia renderizada. Un ejército llega a su destino, y ahí el combate se sigue resolviendo como cálculo.

## 5.11 Liderazgo (a petición del usuario, 2026-09-01)

Cada Jugador tiene un valor de **Liderazgo**, y cada tropa (tipo) un **coste de Liderazgo**. Al salir a campaña, la suma de los costes de los escuadrones que ese Jugador se lleva no puede exceder su Liderazgo.

**Es un límite de SALIDA, no de posesión.** Se pueden poseer muchos más escuadrones de los que se pueden sacar de una vez; lo que se queda forma la guarnición y defiende el asentamiento (5.12). Esto convierte "¿qué me llevo?" en la decisión central de cada campaña, y le da un propósito real a la guarnición, que antes era simplemente "todo lo que tienes".

En un ejército de varios jugadores, **cada uno se valida contra SU propio Liderazgo, por separado**. No hay tope agregado del ejército: cuatro jugadores juntos sacan cuatro veces más.

### 5.11.1 El coste depende del poder

Decisión del usuario: **a mayor poder de la tropa, mayor coste de Liderazgo.** Se deriva del poder nominal completo del escuadrón —lo que se comanda son soldados, no estadísticas por soldado— en vez de escribirse a mano tropa por tropa:

```
coste de Liderazgo = poderBase × unidades × factor
```

Con Liderazgo base **50** y el factor anclado para que la Milicia de lanceros cueste **10**:

| Tropa | poderBase | Unidades | Poder total | Coste |
|---|---|---|---|---|
| Milicia de lanceros | 2 | 25 | 50 | 10 |
| Lanceros con escudo de mimbre | 3 | 20 | 60 | 12 |
| Espadachines de espada corta de cobre | 4 | 20 | 80 | 16 |
| Honderos | 5 | 25 | 125 | 25 |
| Hacheros ligeros | 7 | 18 | 126 | 25 |
| Escaramuzadores con jabalina | 8 | 20 | 160 | 32 |
| Espadachines con espadas y escudos de bronce | 9 | 18 | 162 | 32 |
| Hacheros armados | 12 | 15 | 180 | 36 |
| Lanceros pesados micénicos | 14 | 15 | 210 | 42 |
| Arqueros | 9 | 25 | 225 | 45 |
| Arqueros con arco compuesto | 15 | 20 | 300 | 60 |

Se deriva y no se escribe a mano porque `poderBase` sigue siendo PLACEHOLDER pendiente de calibración (5.8): once números escritos a mano se desincronizarían del poder en cuanto se calibre, una fórmula no.

**Consecuencia buscada:** un escuadrón de Arqueros con arco compuesto (60) es **infielable** para un jugador sin progresión. La tropa de élite queda gateada detrás del Liderazgo, no solo detrás de recursos y edificios.

**Contrapeso que sale del cruce con el suministro:** la ración es por SOLDADO y el coste de Liderazgo por PODER. Por punto de Liderazgo, la Milicia da 2.5 soldados y los Arqueros con arco compuesto 0.33 — las tropas baratas comen mucho más por punto gastado. Con el mismo carro, una horda barata tiene mucha menos autonomía que una fuerza de élite (5.13). El Liderazgo premia la calidad; el suministro castiga la cantidad.

> El Liderazgo es **base + progresión**. La progresión en sí (cómo sube) es la mecánica de progreso de jugador, todavía sin diseñar — ver `Docs/Mecanicas a desarrollar.md` §11.

## 5.12 Ejércitos y movimiento por el mapa (a petición del usuario, 2026-09-01)

Los ejércitos se mueven por el mapa del mundo para atacar, igual que las caravanas: siguen una ruta que rodea el terreno costoso, y tardan en llegar.

### 5.12.1 Las dos formas de salir son la misma entidad

El Jugador puede salir **solo** (eligiendo qué escuadrones se lleva) o **como parte de un ejército de varios jugadores** que se mueve como una sola entidad. No son dos cosas distintas: **salir solo es un ejército de un participante**. Un solo concepto, las mismas reglas de movimiento, suministro y combate en ambos casos.

Un Jugador **puede unirse a un ejército ya en campaña**, siempre que este pase por su asentamiento (si no, unirse sería teletransportar refuerzos). Al unirse aporta sus escuadrones —validados contra su propio Liderazgo— y su parte del carro de suministros.

### 5.12.2 Identificación en el mapa

Un ejército se dibuja como **rombos, uno por cada Jugador que va en él**, uno detrás de otro medio superpuestos, cada uno del color de su Facción. El rombo lo distingue del triángulo de caravana y del círculo de asentamiento.

### 5.12.3 Un destino; los encuentros salen de la geometría

Un ejército solo sabe **ir a un sitio** (un asentamiento o un punto del mapa). Todo lo demás se produce por proximidad, sin declararlo:

- **Al llegar** a un asentamiento enemigo → asedio (5.2.1).
- **Al cruzarse** con un ejército enemigo → combate en mundo abierto (5.2.2).
- **Al pasar cerca** de una caravana enemiga → intercepción (5.2.3, Doc 3.10).

Un ejército puede además quedarse **estacionado** en un punto indefinidamente — aparcar en un paso de montaña para cortarlo es una jugada legítima. Estacionado consume menos suministro que marchando, pero **nunca cero**.

### 5.12.4 La guarnición es lo único que defiende

Como los escuadrones que salen se van de verdad (5.4), **un asentamiento cuyos jugadores se llevaron todo queda indefenso**, y un asedio contra él lo conquista sin combate. Esta es la tensión central de la mecánica: atacar cuesta dejar la casa descubierta.

**Y el premio justifica el riesgo** (decisión del usuario, 2026-09-02): conquistar entrega **un asentamiento completo y en funcionamiento**, y además **amplía los asentamientos de la Facción por encima del cupo de su nivel** (Doc Fase_0_5 §5). Conquistar es la única vía de crecer más allá del techo que marca el nivel de Facción — fundar sí respeta el cupo, conquistar no. Ese es el incentivo, y es lo que impide que la guerra sea un intercambio de pérdidas donde a nadie le compensa atacar.

### 5.12.5 Velocidad

**Un ejército es tan rápido como su escuadrón más lento** — mismo criterio que el usuario fijó para las caravanas (una caravana es tan rápida como su carro más lento). El terreno modula por encima de eso: cruzar colina o montaña cuesta más que el llano.

**Y el agua no se cruza** (Doc 1.0b): un ejército la rodea, y si no hay camino por tierra hasta el destino, sencillamente **no se puede movilizar**. Tampoco replegarse, si el regreso quedara cortado. No hay embarque.

**Cada tropa tiene velocidad propia** (decisión del usuario, 2026-09-02). Tres clases:

| Clase | Tropas | Velocidad |
|---|---|---|
| **Ligera** | Milicia de lanceros, Honderos, Lanceros con escudo de mimbre, Escaramuzadores con jabalina | **20** |
| **Media** | Espadachines de cobre, Espadachines de bronce, Hacheros ligeros, Arqueros | **16** |
| **Pesada** | Hacheros armados, Lanceros pesados micénicos, Arqueros con arco compuesto | **12** |

Las dos reglas que fijan esta tabla, ambas del usuario:

1. **Una caravana inicial no puede ser más rápida que un ejército en movimiento.** La caravana comercial va a 16, así que ni siquiera un ejército medio se deja adelantar por ella.
2. **Un jugador solo con infantería ligera tiene que poder alcanzar una caravana inicial.** Ligera 20 > comercial 16: la caza.

Consecuencias que salen del `min` sin escribir ninguna regla más:

- Un ejército **pesado (12) no alcanza a ninguna caravana**, y eso es correcto: un ejército de asedio no persigue mercaderes. La intercepción es cosa de tropa ligera.
- **Meter un solo escuadrón pesado en una partida de incursión la frena a 12** y le quita la capacidad de cazar. Incursión y asedio pasan a ser composiciones distintas, no la misma fuerza con otra orden.
- La caravana de **contrabando (24) sigue escapando de todo**, lo cual es deliberado: el contrabandista evade por diseño, y no es una caravana "inicial".

### 5.12.6 Cancelar la marcha

Una marcha en curso **se puede cancelar en cualquier momento**, y hacerlo **dispara la vuelta**: el ejército pasa a `regresando` y desanda su ruta hacia el asentamiento de origen. No se teletransporta ni se desvanece — volver cuesta el mismo camino que costó ir, y sigue comiendo del carro durante el regreso.

### 5.12.7 Qué ve un ejército, y qué ve el jugador

Un ejército en marcha **ve a su alrededor en un radio de 150** (unidades de mapa). Sobre el mundo de 2000×2000 eso son unas **dos provincias** (Doc 1.0a), así que una columna en campaña divisa varias ciudades por delante si la geografía lo permite — y sigue muy por debajo del radio de cohesión de un reino (~5 provincias), de modo que ver no es lo mismo que controlar.

Un jugador, por tanto, ve de lo AJENO dos cosas:

1. **Lo que entre en su zona de influencia** — su territorio, que vigila por definición.
2. **Lo que sus propios ejércitos alcancen a ver** mientras marchan.

De un ejército ajeno se sabe **dónde está, de qué Facción es y cuánta gente lo compone** (el número de rombos, Doc 5.12.2). NO se sabe su composición ni su poder: eso exigiría ver sus escuadrones, y sería telemetría de un rival.

> Esto es la fuente **espacial** de la niebla de guerra (`Docs/Mecanicas a desarrollar.md` §12), que estaba bloqueada precisamente por faltar este número. Lo que sigue pendiente ahí es la capa de MEMORIA — el "último conocido", que recuerda lo que viste cuando dejas de verlo.

## 5.13 Suministro en campaña (a petición del usuario, 2026-09-01)

**Un ejército en marcha NO come del almacén de su asentamiento.** Lleva su propio **carro de suministros** con la comida que consume mientras se mueve. Si se queda sin comida, la moral colapsa y los soldados desertan — exactamente la misma regla del hambre que en guarnición (5.4), solo cambia de qué despensa se come.

- **Capacidad**: **FIJA e igual para todos los Jugadores** — es un carro, no una abstracción proporcional a lo que llevas. Se **suma** al formar ejército: un ejército de cuatro lleva cuatro carros.
- **Carga**: al salir o al unirse, cada Jugador **toma del asentamiento**. Si el almacén no llega, se sale con menos autonomía; no se bloquea la salida. Sacar un ejército **cuesta stock real** al asentamiento.
- **Reabastecimiento en ruta**: al pasar por un asentamiento **propio**, siempre. Por uno **aliado**, solo si ese asentamiento tiene la opción activada. Por uno neutral u hostil, nunca.
- **Regreso**: el sobrante **vuelve al almacén** del asentamiento de origen. **El carro NO se descarga en ruta ni en otro asentamiento** — si pudiera, el ejército sería un transporte de mercancías gratuito que dejaría sin sentido a las caravanas. Para mover carga está el punto siguiente.

### 5.13.1 El radio operativo es la constante de diseño

La capacidad del carro **no es un número elegido, es una consecuencia**. La regla que la fija (decisión del usuario, 2026-09-02):

> **Un jugador solo, con su carro, tiene que poder recorrer al menos un cuarto del mapa ida y vuelta con la comida que carga.**

Sobre el mapa de 2000×2000 eso son 1.000 unidades de recorrido. Una carga máxima de Liderazgo son ~70 soldados, que a velocidad ligera (20) tardan 50 ticks en ese trayecto y comen `70 × 0.15 × 50 = 525`. De ahí sale la capacidad del carro.

Como la autonomía se mide en **ticks** y no en distancia, **la velocidad pasa a ser también un atributo logístico**: un ejército rápido cubre más mapa con la misma comida. Un ejército pesado tiene la mitad de alcance con el mismo carro — y por eso necesita caravanas.

### 5.13.2 Caravanas adjuntas al ejército

Un ejército puede llevar **caravanas adjuntas** que amplían su capacidad de carga más allá de la suma de los carros de sus jugadores. Es la forma de proyectar una campaña lejos sin depender de que se unan más jugadores.

- La capacidad de una caravana es **igual o mayor que la del carro de un jugador** — si cargara menos, la caravana no tendría sentido como tren de suministros.
- **Entran en el `min` de velocidad.** Una caravana comercial va a 16, así que adjuntarla baja un ejército ligero de 20 a 16 y **le quita la capacidad de cazar caravanas**. No se puede tener alcance profundo y velocidad de incursión a la vez.
- **Cuestan comercio.** El cupo de caravanas de un Mercado es 2/4/6 según su nivel (Doc 3): enganchar la flota a un ejército es apagar tu comercio mientras dure la campaña.
- **Si el ejército es derrotado, las caravanas adjuntas se pierden.** Eso convierte el tren de suministros en un objetivo militar de verdad: cortar la retaguardia gana campañas sin asaltar una muralla.

### 5.13.3 Escolta de caravanas

Una caravana adjunta **puede ir cargada de mercancía y hacer su entrega normal** mientras marcha con el ejército. Eso resuelve la escolta de caravanas, que estaba pendiente sin implementar (Doc 3.10): la caravana viaja protegida por el poder de combate del ejército en vez de por su defensa base fija.

La misma regla de velocidad la equilibra sola: escoltar baja el ejército a la velocidad de la caravana, así que **no se puede escoltar y depredar a la vez**.

### 5.13.4 Un ejército que se queda sin nada se disuelve

Regla necesaria por el TIEMPO REAL (decisión del usuario, 2026-09-02). Un tick es un minuto real, así que un
ejército cuyo jugador no vuelve sigue existiendo y consumiendo durante horas. Cuando el carro se vacía, la
deserción por hambre lo lleva a cero — pero un escuadrón **persiste como identidad aunque se quede sin
unidades** (5.4), de modo que sin una regla explícita quedaría un **ejército fantasma**: cero soldados (y por
tanto cero ración, ya no pasa hambre) marchando indefinidamente, llegando a su destino y atacando con poder 0.

Por eso: **un ejército cuyos escuadrones están todos a cero se disuelve**, y sus identidades de escuadrón
—vacías pero con su nombre y su veteranía— **vuelven al asentamiento de origen**, que es donde se pueden
rellenar reclutando (5.4). No se pierden: lo que murió son las unidades, no el squad.

Si el asentamiento de origen ya no existe, sus jugadores quedan huérfanos (5.4) y con ellos esas identidades,
hasta que entren en una Facción con asentamiento.

> El carro de suministros y los carros/animales de tiro del revamp de caravanas (`Docs/Mecanicas a desarrollar.md` §8) son el mismo concepto físico; unificarlos queda para cuando esa mecánica se diseñe.
