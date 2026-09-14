# 5. Sistema Militar y de Combate

## 5.1 Principio general: héroe-comandante liderando tropa (heredado de Iberia)
Cada Jugador está en el mundo como su **Héroe** (5.16), un comandante que lidera escuadrones de unidades NPC. Regla de oro: **Tropa > Héroe**. Puede llevar más de un regimiento a una batalla pero solo despliega uno a la vez, intercambiables en puntos tácticos de reabastecimiento dentro del combate.

**El héroe combate por sí mismo.** En las batallas jugadas en Unity lo hace como un personaje de Conquest: clase, nivel, atributos, perks y equipo (5.16). En las que se resuelven con números (NPC contra NPC, 5.15.6) la regla de oro la hace cumplir esta cifra:

> **Un héroe solo vale lo que UNA unidad de la tropa de élite.**

Se deriva del catálogo (5.8) en vez de escribirse a mano, por el mismo motivo que el coste de Liderazgo (5.11.1): `poderBase` es placeholder, y un número suelto se desincronizaría al recalibrar. Son **15**.

Qué significa esa cifra, que es lo que dice si está bien puesta:

| | Poder |
|---|---|
| Un héroe solo | **15** |
| El escuadrón más barato completo (milicia, 25 × 2) | 50 |
| Un escuadrón de élite (arqueros compuesto, 12 × 15) | 180 |

Un héroe vale **menos de un tercio de la peor leva**. Frente a una columna no decide nada —Tropa > Héroe, intacto— y decide justo en el único combate que es suyo: **contra otro héroe solo**. Contra una caravana no: una caravana sin escolta se defiende de un héroe solo (Doc 3.10), a propósito.

**El héroe no muere.** Al perder una batalla queda **herido** (5.16.4) y pierde las bajas de sus escuadrones; si fue en mundo abierto, entrega además **la mitad de su carro** (5.16.6). Gana nivel y experiencia según su desempeño en la batalla (5.16.3).

## 5.2 Modalidades de batalla instanciadas (heredado de Iberia)
El combate ocurre en INSTANCIAS separadas del mapa global (aunque se desencadenen en él), límites simétricos fijos, 2 bandos (Atacante/Defensor), sin empates.

1. **Asedio de asentamientos**: atacante captura banderas/áreas vitales antes de que expire el tiempo; defensor gana resistiendo. Solo defienden miembros de la Facción soberana del nodo o Facciones aliadas/vasallas confirmadas. Mortalidad severa (permadeath). Los héroes en cola rellenan la instancia a medida que caen otros; el tope de la batalla cuenta héroes (5.15.1).
2. **Mundo abierto**: choque de patrullas/ejércitos. Bandera/campamento transitorio; quien la pierde se retira, entrega la mitad de su carro y sus héroes quedan heridos (5.16.4). **Se declara, pero solo estando delante**: la proximidad ofrece atacar y el jugador decide; nadie es arrastrado a un combate por pasar cerca (5.12.3). Alcanzar a quien huye es lo que hace la persecución.
3. **Defensa/intercepción de caravanas**: combate asimétrico móvil (ver Doc 3, sección 3.10). Igual que el anterior, se dispara por proximidad de un ejército a una caravana (5.12).
4. **Entrenamiento/matchmaking** (pospuesto a una fase posterior a Fase 0/1): 15v15 puro, sin permadeath, para probar tácticas. Requiere lo mismo que el Attack Timer (5.6).
### 5.2.5 Resolución numérica y varianza de combate

Las batallas que se resuelven con números (5.15.6) suman el poder de cada bando y **multiplican cada uno por un factor aleatorio de ±15%** (`MILITAR.varianzaCombate`). Gana quien saque el producto más alto. Cuanto más ajustado el resultado, más bajas sufre también el ganador; el perdedor siempre pierde más. El azar entra **una sola vez por bando y por combate** — no hay tiradas por unidad ni por ronda. El poder de cada escuadrón sube un **5% por cada nivel** que tenga (5.16.3).

**Qué implica ese ±15%**: los multiplicadores van de 0.85 a 1.15, así que el cociente entre ambos bandos va de 0.74 a 1.35. Es decir, **un atacante necesita un 35% más de poder para tener la victoria garantizada**; por debajo de eso siempre puede perder. Ese margen no es un detalle de implementación: es lo que decide cuándo merece la pena atacar, sabiendo que marchar cuesta tiempo real, deja la ciudad descubierta, vacía el almacén y las bajas son permanentes.

## 5.3 Formaciones y cohesión táctica (heredado de Iberia)
- Romper formación penaliza duro (ej. arqueros dispersos -30% precisión, escuderos aislados -20% defensa, lanceros sin formación pierden bono anti-carga). Flanquear/aislar formaciones enemigas es táctica válida.
- COHESIÓN ENTRE JUGADORES: varios jugadores anclando una línea juntos ganan Defensa Compartida, resistencia a rotura de moral, regeneración lenta de HP.
- Jugador novato: útil desde el día 1 con infantería básica de escudo barata ("carne de línea") mientras veteranos flanquean.

## 5.4 Ciclo de vida de unidades (heredado de Iberia)

**Los escuadrones son del HÉROE, no del asentamiento.** El asentamiento es donde el héroe tiene su campamento (5.15.2), no quien los posee. Consecuencias:

- Al **conquistar** un asentamiento se aplica 5.15.5: cada héroe defensor queda fuera con los escuadrones que usó en la batalla; el resto de escuadrones de sus residentes, guarnición incluida, quedan a 0 unidades y se van con el campamento de su héroe al asentamiento más cercano de su Facción, donde pasa a residir. No hay captura de guarnición, y el asentamiento conquistado queda sin guarnición.
- **Los antiguos residentes pierden la residencia y con ella los cargos locales**: la ciudad cambia de dueño entera, valga para los que estaban de campaña como para los que estaban en casa (Doc 2.5).
- La ciudad **no se entrega intacta**: la conquista la saquea (población, edificios, murallas) y abre una **ventana de ocupación** de tiempo fijo — inmune a un nuevo asedio, recaudación y crecimiento a la mitad, mantenimiento congelado. El premio sigue siendo "un asentamiento en funcionamiento" (5.12.4), pero **es una inversión que tarda en rendir**, no un subidón inmediato. Detalle en 5.12.9.
- Llevarse escuadrones a campaña los saca del campamento de verdad: dejan de comer del almacén (5.13). Los que se quedan en el campamento sin estar en guarnición no defienden (5.12.4).
- **Un héroe cuya Facción se queda sin asentamientos queda HUÉRFANO** (Doc 0): conserva sus escuadrones, pero sin residencia — sin sitio donde guardarlos, reabastecer ni reclutar. Deja de serlo al volver a residir en algún asentamiento (ciudadanía, Doc 2.5).

Ciclo de vida propiamente dicho:

- PERMADEATH individual (excepto modo entrenamiento): bajas son permanentes.
- El SQUAD (nombre, nivel y experiencia) persiste aunque el regimiento sea aniquilado — se puede rellenar con nuevos reclutas conservando el progreso.
- DESERCIÓN POR HAMBRE: tropas consumen raciones continuamente; sin suministro, la moral colapsa y desertan permanentemente (mismo efecto que perderlas en combate). **Es la misma regla en guarnición y en campaña** — solo cambia de qué despensa se come (5.13).
- **La población civil come ANTES que las tropas.** Los civiles son quienes producen (Granja/Cantera/Fundición/...); las tropas no producen nada, así que bajo escasez sostenida el golpe lo absorbe primero la parte del sistema que no es productiva: la moral militar colapsa y empieza la deserción MUCHO antes de que la nutrición civil llegue a comprometerse. Para que los civiles se quedaran sin nada, la producción tendría que caer por debajo de SOLO su propio consumo. Está pensado para la escala multijugador real, donde el número de héroes por asentamiento y la presión militar crecen con el servidor mientras la producción de comida está acotada por el espacio construido (`Consideraciones/NPC_Gobernanza_Facciones_Controladas.md`).
- **Reclutar exige reserva de trigo proyectada**: antes de aceptar un reclutamiento o una reposición, el trigo en almacén debe cubrir `RESERVA_CONSTRUCCION.horizonteTicksComida` (8) minutos del consumo YA PROYECTADO CON la tropa nueva sumada (civiles + tropas existentes + la que se está reclutando). Es regla del MOTOR, igual para NPC y jugador: igual que la Vivienda acota cuánta población civil puede aparecer, esto acota cuánta tropa puede sostenerse, sin importar cuántos héroes residan en el asentamiento. Si varios reclutan en el mismo minuto, los primeros agotan el margen y el resto falla limpio.
- **El pool de reclutamiento es la población MENOS la ya ocupada en producción**, no la población total: reclutar no puede sacar pesants que cubren los trabajadores de Granja/Cantera/Leñera/minas/Corral, ni artesanos que cubren los de Fundición/Curtiduría/Armería/Carpintería. Es simétrico entre pesants y artesanos, y aplica igual al reclutamiento de un jugador y al de la gobernanza NPC.

## 5.5 Progresión de las tropas (ver también Doc 4.1)
Solo se reclutan Pesants y Artesanos, pagando equipo (5.8). Sus escuadrones suben de **nivel y experiencia** combatiendo de verdad, según su desempeño en la batalla (5.16.3). La Nobleza no se convierte en tropa.

## 5.6 Attack Timer (heredado de Iberia, pospuesto a fase posterior a Fase 0)
Asedios FORMALES en ventanas limitadas (ej. 2 veces/semana, horario fijo, ~1h de duración). Ataques logísticos (mundo abierto, caravanas) libres 24/7. Pospuesto a Fase 1+: requiere instanciado multijugador programado y un sistema de colas y horarios.

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
- Centro Urbano: reclutamiento de la defensa mínima (Milicia de lanceros, ver roster 5.8). La tropa de entrada no depende del Barracón, que exige nivel 2 (Doc 4.2.1), sino del único edificio que nace `activo` con el asentamiento, sin cola de construcción ni requisito: todo asentamiento puede defenderse desde el primer minuto, aunque sea con la unidad más débil del roster.
- Fundición: fabricación de lingotes de cobre/estaño/bronce — auto-construcción (ver Doc 4.2).
- Curtiduría: tratamiento de cuero (livestock → cuero → cuero curtido → cuero de calidad).
- Armería: fabricación de armas y armaduras a partir de lingotes y cuero — insumo directo del reclutamiento de Barracón/Galería de tiro.
- Carpintería: recluta armas de asedio (ariete, torre de asedio) y habilita construir/mejorar Palacio, Armería, Barracón y Galería de tiro de nivel 2+.
- Barracón: reclutamiento de tropas cuerpo a cuerpo (ver roster 5.8). Adición MANUAL de Gobernador/Maestro de Obras a la cola (Doc 4.2).
- Galería de tiro: reclutamiento de tropas a distancia (ver roster 5.8). Adición MANUAL de Gobernador/Maestro de Obras a la cola (Doc 4.2).
- Mina de cobre: extractor de cobre (Doc 1.4/4.2.1) — único extractor de cobre del juego, insumo obligatorio de Fundición para todo equipo de bronce. Extractores finitos con reemplazo automático al agotarse (mecanismo completo en Doc 4.2, incluye número fijo por tipo — NO ligado al nivel de asentamiento).
- Gran Fundición: edificio de élite; requiere nivel de Facción 3 (Doc 4.2.1).
- **No hay carros de guerra en Fase 0**: se posponen a Fase 1, y con ellos los Establos.

**Materiales limitantes (clave anti-"ejército meta universal"):**
- COBRE: relativamente abundante.
- ESTAÑO: raro, concentrado en pocas ubicaciones (base histórica real: la disrupción de rutas de estaño es una teoría real del colapso de la Edad de Bronce). El bronce de calidad — y por tanto las tropas de tier alto — depende del acceso a estaño.

## 5.8 Roster de tropas (reclutamiento por edificio + nivel interno, ver Doc 4.2.1)

Catálogo en `TROPAS_RECLUTABLES`. Terminología (Doc 0): son tres conceptos y la jerarquía de entidades es **Héroe → Escuadrón → Unidad**. Una **tropa** es el TIPO (ej. "Lanceros con escudo de mimbre") y las tablas de abajo son su catálogo; un **escuadrón** es la instancia que un héroe posee y comanda; una **unidad** es cada soldado individual dentro de él. El número de unidades **NO lo elige el jugador** (ver "Unidades por defecto" más abajo) — cada escuadrón reclutado añade siempre el mismo tamaño fijo.

El roster no se organiza por Tier abstracto (inspiración Total War Troy, foco Egeo/Grecia) — cada tropa se recluta en Centro Urbano, Barracón o Galería de tiro, según el NIVEL INTERNO del edificio (1-3, ver Doc 4.2.1; Centro Urbano no tiene niveles), pagando el equipo correspondiente fabricado en Armería (ver catálogo completo de recetas en Doc 4.2.1). "Costo" en las tablas de abajo es POR SOLDADO — el costo real de reclutar es ese valor × "Unidades". Cada tropa tiene además un `poderBase` (placeholder) que usan las batallas que se resuelven con números (5.2.5).

**Coste de oro por reclutar** (`Consideraciones/Economia_Del_Oro_Definicion.md`): además del equipo, reclutar cuesta **oro por soldado según el escalón de la tropa** (`RECLUTAMIENTO_ORO_POR_ESCALON` en `constants.ts`, PLACEHOLDER 1/2/4/7/11 para escalones 1-5 — curva que sube más deprisa que el poder, igual criterio que `LIDERAZGO.costePorEscalon`). **Única excepción: la Milicia de lanceros del Centro Urbano** (`tropa.edificio === 'centroUrbano'`), que cuesta solo madera: la defensa mínima no depende del tesoro. Todo lo del Barracón/Galería de tiro cuesta oro, escalón 1 incluido: sin oro solo tienes la milicia. Reponer bajas vuelve a pagar oro por los soldados repuestos. `factorCostoReclutamiento` ("Leva Forzosa", Doc 4.4) toca **solo el equipo, no el oro** — no se conscribe moneda. Regla de motor uniforme: se cobra igual a NPC y jugador.

**Centro Urbano (defensa mínima, sin edificio dedicado) — carril Pesants + Artesanos:**

| Tropa | Costo (por soldado) | poderBase | Unidades |
|---|---|---|---|
| Milicia de lanceros | 2 Madera (en bruto, sin pasar por Armería) | 2 | 25 |

**Barracón (cuerpo a cuerpo) — carril Pesants + Artesanos, combate real (Doc 4.1/5.5):**

| Nivel | Tropa | Costo (por soldado) | poderBase | Unidades |
|---|---|---|---|---|
| 1 | Lanceros con escudo de mimbre | 1 Arma de Madera | 3 | 25 |
| 1 | Espadachines de espada corta de cobre | 1 Arma de Cobre + 1 Armadura Básica | 4 | 20 |
| 2 | Hacheros ligeros | 1 Arma de Bronce + 1 Armadura Básica | 7 | 18 |
| 2 | Espadachines con espadas y escudos de bronce | 2 Arma de Bronce + 1 Armadura Intermedia | 9 | 18 |
| 3 | Lanceros pesados micénicos (escudos grandes) | 2 Arma de Bronce + 2 Armadura Intermedia | 14 | 15 |
| 3 | Hacheros armados (armadura media) | 1 Arma de Bronce + 1 Armadura Intermedia | 12 | 15 |

**Galería de tiro (a distancia) — carril Pesants + Artesanos, combate real:**

| Nivel | Tropa | Costo (por soldado) | poderBase | Unidades |
|---|---|---|---|---|
| 1 | Honderos (escaramuzadores) | 1 Arma de Madera | 5 | 20 |
| 2 | Escaramuzadores con jabalina | 1 Arma de Bronce + 1 Armadura Básica | 8 | 18 |
| 2 | Arqueros | 1 Arma de Bronce + 1 Armadura Intermedia | 9 | 18 |
| 3 | Arqueros con arco compuesto | 3 Arma de Bronce + 2 Armadura Intermedia | 15 | 12 |

`poderBase` es placeholder. Las unidades de cada escuadrón las fija el escalón de su tropa (5.11.1).

**Unidades por defecto** (`unidadesPorDefecto`): es el TOPE del escuadrón, el jugador nunca elige cuántos soldados reclutar. `costoEquipo` sigue siendo por soldado. Reclutar desde cero cuesta "Costo (por soldado)" × "Unidades" de la tabla — ej. Milicia de lanceros cuesta 2 Madera/soldado × 25 = 50 Madera. Si el escuadrón ya existe y está por debajo del tope (bajas de combate, Doc 5.4), reclutar de nuevo REPONE solo las unidades que faltan hasta el tope, al mismo costo por soldado — no es un bloque nuevo completo (ver el párrafo "Escuadrón por héroe" más abajo). Antes de confirmar, la interfaz muestra el coste de ESTE reclutamiento, que puede ser parcial.

**Milicia de lanceros y Arma de Madera.** La Milicia de lanceros se recluta en el Centro Urbano y se paga con madera en bruto, sin pasar por la Armería: es deliberadamente la más débil del roster (poderBase 2) y existe para que el bucle de juego arranque pronto, no para ganar batallas. Para reclutarla basta con residir en el asentamiento (Doc 2.5) y tener los 25 soldados de población y los 50 de madera del escuadrón. "Lanceros con escudo de mimbre" y "Honderos" se pagan con Arma de Madera: si la cadena metalúrgica o del cuero completa fuera el único camino, casi ningún asentamiento tendría tropa pronto, porque muy pocos nacen con cobre o livestock en su zona.

**La Nobleza no se recluta.** Sigue existiendo como clase de población (crecimiento, requisito de Palacio, ciudadanos mínimos), pero no se convierte en tropa. El único carril de reclutamiento es el de equipo (Centro Urbano/Barracón/Galería de tiro), abierto a Pesants y Artesanos.

**Relación entre tropas ya reclutadas y el edificio que las produjo**: NO existe ninguna relación posterior al reclutamiento. Una vez una tropa está reclutada y en el mundo, es independiente del edificio (Barracón/Galería de tiro) que la originó. Si el edificio sube de nivel después, los escuadrones ya existentes NO se ven afectados de ninguna forma — ni mejoran ni empeoran. "Mejorar" solo significa poder reclutar tropas nuevas de mayor nivel a partir de ese momento.

**Una tropa reclutada JAMÁS cambia de identidad/tipo al ganar experiencia.** "Milicia de lanceros" que sube de nivel se queda siendo "Milicia de lanceros" con más poder — nunca pasa a ser "Hacheros" ni ningún otro `tropaId`. El pool de origen (Pesants/Artesanos) tampoco cambia. Subir de nivel hace más fuerte al MISMO escuadrón (5.16.3); "mejorar" de tropa solo ocurre reclutando una tropa DISTINTA y mejor cuando Barracón/Galería de tiro suba de nivel interno, y eso crea un escuadrón nuevo, no transforma el existente.
**Escuadrón por héroe, no por asentamiento.** Cada héroe tiene como mucho **un escuadrón de cada tropa en toda la partida**, esté donde esté (en su campamento, en su columna o de escolta), con tope `unidadesPorDefecto`. Reclutar de nuevo una tropa que ya tiene repone el faltante si hay bajas (ver "Unidades por defecto" arriba); nunca crea un segundo escuadrón. Si ese escuadrón está fuera, no se puede reclutar otro de su tropa: solo reponerlo allí donde está. Reclutar no exige un General asignado: solo que el héroe resida en el asentamiento (fundador o casa comprada, Doc 2.5), y un héroe reside en un solo asentamiento a la vez.

**Reclutar y mover tropa:**

| Acción | Requisito |
|---|---|
| Reclutar un escuadrón **NUEVO** | Residir en el asentamiento **y** estar en él, y no tener ya un escuadrón de esa tropa en ninguna parte |
| **Reponer** un escuadrón que ya tienes | Estar donde está el escuadrón (en tu campamento o en tu columna), dentro de una plaza **de tu Facción** que lo permita (`politicaDeAcceso` ≠ `cerrado`, sin veto). Gasta población y almacén de esa plaza, autolimitado por la reserva de trigo |
| **Mover** escuadrones propios | Donde estén |

Consolidar una plaza —reclutar escuadrones nuevos ahí, asignar guarnición, ejercer cargos, recaudación al 100%— exige residir en ella, es decir, trasladar allí el campamento (`cambiarResidencia`, Doc 2.5).

## 5.9 Exilio como política de soberanía (heredado de Iberia, ver también Doc 2.8)
El Gobernador puede decretar exilio de jugadores enemigos de su territorio; coste de reubicación (pérdida parcial de materiales, desplazamiento físico para recuperarlos).

## 5.10 Dónde se resuelve el combate
Las batallas con algún héroe humano se juegan como partidas reales en Unity (5.15). Las que no tienen ninguno —NPC contra NPC, bandidos contra una caravana— se resuelven con números (5.2.5, 5.15.6). En los dos casos la batalla nace en el mapa de BronzeAge: los ejércitos se mueven por el mundo (5.12), y el combate empieza donde se encuentran.

## 5.11 Liderazgo

Cada Héroe tiene un valor de **Liderazgo**, y cada tropa (tipo) un **coste de Liderazgo**. Al salir a campaña, la suma de los costes de los escuadrones que ese Héroe se lleva no puede exceder su Liderazgo.

**Es un límite de SALIDA, no de posesión.** Se pueden poseer muchos más escuadrones de los que se pueden sacar de una vez. Lo que se queda en el campamento no defiende por sí solo: solo la guarnición que el héroe asigna, que tiene su propio cupo y no gasta Liderazgo (5.15.3). Esto convierte "¿qué me llevo?" en la decisión central de cada campaña. La selección se puede guardar como **loadout** (5.16.5).

En un ejército de varios héroes, **cada uno se valida contra SU propio Liderazgo, por separado**. No hay tope agregado del ejército: cuatro jugadores juntos sacan cuatro veces más.

### 5.11.1 El coste va por ESCALÓN

**A mayor calidad de la tropa, mayor coste de Liderazgo.** Las tropas se agrupan en
**cinco escalones**, de leva a élite, y el coste es el de su escalón — no una fórmula sobre su poder.

| Escalón | Coste | Soldados por escuadrón | Caben con 100 | Tropas |
|---|---|---|---|---|
| 1 — leva | 7 | 25 | 14 | Milicia de lanceros, Lanceros con escudo de mimbre |
| 2 — tropa de línea | 14 | 20 | 7 | Espadachines de cobre, Honderos |
| 3 — veterana | 22 | 18 | 4 | Hacheros ligeros, Escaramuzadores, Espadachines de bronce, Arqueros |
| 4 — pesada | 32 | 15 | 3 | Hacheros armados, Lanceros pesados |
| 5 — élite | 45 | 12 | 2 | Arqueros con arco compuesto |

Liderazgo base **100**.

**El tamaño del escuadrón también va por escalón**: cuanto más de élite, menos cuerpos. No es decorativo — es
lo que hace que subir de escalón sea *calidad* y no *cantidad*, y lo que ordena el rendimiento por punto de
Liderazgo de mayor (leva) a menor (élite).

**Por qué no se deriva del poder.** Un coste proporcional al poder nominal daría **el mismo poder por punto de
Liderazgo a las once tropas**: cinco milicias rendirían lo mismo que un lancero pesado, y la élite no sería
mejor por punto, solo vendría en envase más pequeño. Elegir composición no sería una decisión, sería aritmética.

Con coste por escalón el precio crece **más deprisa que el poder**, y eso es lo buscado: la élite es
deliberadamente ineficiente por punto. Se la lleva uno porque veinte cuerpos de élite aguantan un paso que
cien de leva no, no porque rindan más por punto gastado.

**El presupuesto está elegido para que las mezclas interesantes queden JUSTO en el techo**, que es lo que hace
que la decisión duela:

| Composición | Coste |
|---|---|
| 1 élite + 1 pesada + 1 veterana | 99 |
| 2 pesadas + 1 veterana + 1 de línea | 100 |
| 3 veteranas + 1 pesada | 98 |
| 2 élites + 1 leva | 97 |
| Hueste de leva | 14 escuadrones |

Ninguna sobra ni falta por poco, y ninguna admite una unidad más. La cantidad sigue siendo una estrategia
—catorce escuadrones de leva son 350 hombres— pero se paga en suministro: esa hueste vacía un carro en **diez
minutos**, mientras que dos escuadrones de élite (24 hombres) aguantan más de **dos horas** (5.13). Catorce
veces más alcance. **El eje de la decisión no es el poder, es el alcance.**

El Liderazgo puede crecer por encima de la base con la progresión del Héroe (`liderazgoBase`).

## 5.12 Ejércitos y movimiento por el mapa

Los ejércitos se mueven por el mapa del mundo para atacar, igual que las caravanas: siguen una ruta que rodea el terreno costoso, y tardan en llegar.

### 5.12.1 Columna personal y Ejército: la misma entidad, distintas reglas

El Héroe puede salir **solo** (con las tropas que quiera, incluidas ninguna) o **junto a otros héroes**. En el motor son **la misma entidad** y comparten movimiento, suministro y combate. En las REGLAS no son lo mismo, y la línea que los separa **no es cuánta gente va dentro: es cómo salió la columna**.

**Lo decide el acto de salir, y no cambia nunca:** salir *a lo tuyo* —sin destino— hace una Columna personal; **movilizarse contra un destino** hace un Ejército, aunque en ese primer momento vayas solo. Por eso "salir juntos" no necesita ninguna ceremonia aparte: uno moviliza y los demás se suman, en su plaza o en el campo (5.14).

| | **Columna personal** | **Ejército** |
|---|---|---|
| Cómo nace | Se **sale al mundo** desde la residencia, **sin destino** | Se **moviliza** desde un asentamiento **contra un destino**, aunque salga uno solo |
| Destino | **Se rectifica** en cualquier momento | Fijado al salir; cancelar es volver (5.12.6) |
| Caravanas adjuntas | **No puede llevarlas** | Sí, y las conserva aunque baje a un miembro |
| Se le pueden unir otros | No | Sí, **solo de su misma Facción** (5.14) |
| Separarse | No aplica: separarse es disolverla | Sí, **mientras quede alguien** (5.14) |

Dos reglas cierran el modelo:

- **Un Ejército solo se origina en un asentamiento, nunca en campo abierto.** Dos viajeros que se cruzan en el camino no forman un ejército; lo que sí puede hacer uno es unirse a un ejército que ya existe.
- **Ser un Ejército es una identidad, no un recuento.** Uno al que se le separan miembros hasta quedar en uno solo **sigue siendo un Ejército**: conserva su ruta fija y sus caravanas. No recupera la libertad de movimiento por haberse quedado corto.

**Por qué el destino de un ejército no se toca.** Un jugador solo es libre de decidir porque decide por sí mismo; un ejército lleva a varios, así que su rumbo es un compromiso compartido y se fija al salir. Eso hace que **unirse a un ejército cueste la libertad de movimiento**, y que separarse la devuelva al instante — el precio de marchar acompañado, sin ninguna regla extra que lo imponga.

*(El compromiso es un compromiso, no una cárcel: un grupo que quiera cambiar de rumbo puede separarse, moverse y volver a unirse. Cambiar una decisión compartida exige que todos vuelvan a actuar, que es exactamente lo que debe costar.)*

**Un Héroe puede unirse a un ejército ya en campaña** de dos formas distintas, con geometrías distintas: pasando el ejército por **su asentamiento**, de donde saca tropas frescas; o **cruzándoselo en el campo**, aportando lo que ya lleva encima (5.14). En ambos casos sus escuadrones se validan contra su propio Liderazgo.

### 5.12.2 Identificación en el mapa

Un ejército se dibuja como **rombos, uno por cada Héroe que va en él**, uno detrás de otro medio superpuestos, cada uno del color de su Facción. El rombo lo distingue del triángulo de caravana y del círculo de asentamiento.

### 5.12.3 La geometría OFRECE, el jugador decide

**Nada se dispara por proximidad. La proximidad abre un menú.**

Una columna se acerca a algo y el juego le ofrece lo que puede hacer con ello; el jugador elige, o sigue su camino:

| Sobre qué | Qué se le ofrece |
|---|---|
| Ejército o columna ajena | **Inspeccionar** · **Perseguir** |
| Caravana ajena o neutral | **Inspeccionar** · **Interceptar** |
| Asentamiento, en su puerta | **Entrar** · **Asediar** · **Consultar** · **Comerciar** (Doc 3.3) |
| Campamento de bandidos | **Atacar** |

*Los bandidos son la excepción, y por el motivo obvio: un campamento no tiene a nadie que pulse. Los NPC siguen atacando caravanas por su cuenta — su intención es su política.*

#### Los tres anillos

Cada distancia significa una cosa distinta, y las tres juntas son el corazón de la mecánica:

| Distancia | Qué habilita | Quién se entera |
|---|---|---|
| **150 / 80** (columna con tropas / jugador solo) | Ves que hay algo y de quién es | Nadie |
| **40 — inspección** | Ver la composición: qué tropas, de quién. De una caravana, si lleva escolta y **qué** recursos carga, nunca cuántos | **El inspeccionado recibe aviso** |
| **15 — encuentro** | Se cierra una persecución y se ofrece atacar | Los dos |

**El anillo de inspección es lo que convierte el reconocimiento en un juego de dos.** Mirar cuesta ser visto mirando; y el que mira puede huir, porque va más rápido que una columna entera y porque no ha tenido que meterse hasta los 15. De lejos, un ejército ajeno sigue siendo lo que siempre fue: una bandera y unos estandartes, sin composición ni poder (5.12.7).

#### Persecución

Perseguir fija un objetivo **móvil** en vez de un punto: la ruta se recalcula hacia donde esté. Para inspeccionar no hace falta perseguir — basta con verlo. Una persecución termina de cuatro formas:

- se llega a **15**, y ahí se ofrece atacar;
- el perseguidor **cambia de destino**;
- el objetivo queda **herido** por haber sido derrotado (5.16.4);
- o no llega a empezar, porque todos los héroes del objetivo ya estaban heridos.

**El consentimiento es de una sola parte, y así debe ser:** el agresor elige perseguir, el perseguido no elige nada. Escapar depende de ser más rápido — lo que convierte la velocidad de tropa (5.12.5) en la estadística que decide quién puede forzar un combate.

#### Derrota en campo abierto

Quien pierde un choque en campo abierto entrega **la mitad de su carro** —igual una columna personal que un ejército, cuyo carro es el de todos sus miembros— y sus héroes quedan **heridos** 2 minutos (5.16.4). Con el carro vacío no hay botín: solo la herida.

Esa mitad pasa al carro del vencedor, gane quien gane, y **cabe solo lo que quepa**: sus carros más los de sus caravanas adjuntas. Lo que no cabe se pierde.

**La herida corta por los dos lados**: nadie puede perseguir ni atacar a un héroe herido, **y él tampoco puede perseguir ni entrar en batallas**. La primera mitad evita el acoso en cadena al mismo viajero; la segunda evita que la inmunidad se use de escudo para depredar sin riesgo.

#### Lo que sigue saliendo de la geometría

Elegir atacar no es teletransportarse: hay que estar delante. Cuando dos columnas sí se enfrentan, estas reglas acotan lo que pasa:

- **Un encuentro por ejército y minuto.** Sin ese tope, tres columnas juntas se trituran en cascada dentro del mismo tick y el resultado depende de a quién se mire primero.
- **Los aliados no se cruzan**, ni las columnas de la misma Facción. Compartir camino con un amigo no puede costar una masacre cada minuto.
- **Un ejército enemigo manda sobre una caravana.** Con las dos cosas al alcance, se combate: dejar pasar la amenaza real para saquear un carro no tendría sentido.
- **Una caravana escoltada no es un objetivo blando**: quien se topa con ella se topa con su ejército, y eso ya es un choque entre ejércitos (5.13.3).

En un choque en campo abierto **no hay atacante ni defensor**: los dos iban a lo suyo, así que ninguno recibe el bonus de cohesión defensiva. Esa ventaja es de quien defiende una plaza, no de quien se topa con otro en un camino.

Al emboscar una caravana, el botín —el 50% de su carga, 3.10— **viaja en el carro del ejército**, con dos consecuencias: cabe solo lo que quepa, y llega a casa como el resto del carro —el sobrante vuelve al almacén de origen al replegarse (5.13)—. El carro no puede descargarse en ruta, así que esto no lo convierte en un transporte de mercancías: para eso están las caravanas adjuntas.

Un ejército puede además quedarse **estacionado** en un punto indefinidamente — aparcar en un paso de montaña para cortarlo es una jugada legítima. Estacionado consume **una décima parte** de lo que consume en marcha, pero **nunca cero**.

La cifra es lo que hace que estacionar signifique algo: a una décima parte, un carro lleno sostiene una posición diez veces más de lo que dura en marcha, y aparcar pasa a ser una jugada de verdad en vez de un aplazamiento contra el hambre.

### 5.12.4 Quién defiende un asentamiento

Un asentamiento lo defienden **los héroes que están dentro cuando lo atacan**, cada uno con las escuadras de su loadout activo (5.16.5) que tenga en el campamento, que ya caben en su Liderazgo, y **su guarnición**, que maneja la IA del juego (5.15). Los escuadrones que se quedaron en el campamento sin estar en guarnición no defienden. **Un asentamiento sin defensores presentes ni guarnición se asedia igual, sin defensores.** Esta es la tensión central de la mecánica: atacar cuesta dejar la casa descubierta, salvo lo que se deje en guarnición.

**Llegar no es asediar.** Un ejército que llega **acampa delante**, y asediar es una acción que se elige en la puerta (5.12.3).

Eso permite **plantarse frente a una ciudad enemiga sin atacarla.** Bloquear, sitiar sin asaltar, esperar refuerzos o negociar con el ejército a la vista son jugadas legítimas.

El asedio es **una sola batalla**, cuando se ordena. Un ejército acampado junto a una plaza no la muele a asaltos tick tras tick.

Si el asedio **resiste**, el ejército se queda acampado fuera con sus escuadrones. Si **conquista**, se aplica 5.15.5: la plaza queda **sin guarnición** y el ejército conquistador sigue siendo una columna fuera; sus héroes pueden trasladar allí su campamento y asignar guarnición. Ver 5.12.9.

**Reforzar una plaza propia.** Un ejército en la puerta de una plaza de su Facción puede entrar: sus héroes quedan DENTRO con los escuadrones que llevan, y si atacan la plaza la defienden en persona. No se convierten en guarnición: la guarnición solo la asigna quien reside (5.15.3). Es la forma de **reforzar una frontera o defender una plaza amenazada marchando a ella** sin mudar la residencia. Las **caravanas adjuntas** quedan `'aparcadas'` en esa plaza (siguen siendo de su origen; ver Doc 3.13.7). Para volver a campaña se sale con `movilizarEjercito` (5.8).

**Y el premio justifica el riesgo**: conquistar entrega **un asentamiento en funcionamiento** — saqueado y bajo ocupación un tiempo (5.12.9), pero tuyo — y además **amplía los asentamientos de la Facción por encima del cupo de su nivel** (Doc Fase_0_5 §5; la ocupación no toca esa regla — un conquistado conserva su `nivel` sin verificar cupo). Conquistar es la única vía de crecer más allá del techo que marca el nivel de Facción — fundar sí respeta el cupo, conquistar no. Ese es el incentivo, y es lo que impide que la guerra sea un intercambio de pérdidas donde a nadie le compensa atacar.

### 5.12.5 Velocidad

**Un ejército es tan rápido como su escuadrón más lento** — el mismo criterio que una caravana, tan rápida como su animal más lento (Doc 3.13.1). El terreno modula por encima de eso: cruzar colina o montaña cuesta más que el llano.

**Y el agua no se cruza** (Doc 1.0b): un ejército la rodea, y si no hay camino por tierra hasta el destino, sencillamente **no se puede movilizar**. Tampoco replegarse, si el regreso quedara cortado. No hay embarque.

**Cada tropa tiene velocidad propia.** Tres clases:

| Clase | Tropas | Velocidad |
|---|---|---|
| **Ligera** | Milicia de lanceros, Honderos, Lanceros con escudo de mimbre, Escaramuzadores con jabalina | **20** |
| **Media** | Espadachines de cobre, Espadachines de bronce, Hacheros ligeros, Arqueros | **16** |
| **Pesada** | Hacheros armados, Lanceros pesados micénicos, Arqueros con arco compuesto | **12** |

Dos reglas fijan esta tabla:

1. **Una caravana inicial no puede ser más rápida que un ejército en movimiento.** La caravana comercial va a 16, así que ni siquiera un ejército medio se deja adelantar por ella.
2. **Un jugador solo con infantería ligera tiene que poder alcanzar una caravana inicial.** Ligera 20 > comercial 16: la caza.

Consecuencias que salen del `min` sin escribir ninguna regla más:

- Un ejército **pesado (12) no alcanza a ninguna caravana**, y eso es correcto: un ejército de asedio no persigue mercaderes. La intercepción es cosa de tropa ligera.
- **Meter un solo escuadrón pesado en una partida de incursión la frena a 12** y le quita la capacidad de cazar. Incursión y asedio pasan a ser composiciones distintas, no la misma fuerza con otra orden.
- La caravana de **contrabando (24) sigue escapando de todo**, lo cual es deliberado: el contrabandista evade por diseño, y no es una caravana "inicial".

### 5.12.6 Cancelar la marcha

Una marcha en curso **se puede cancelar en cualquier momento**, y hacerlo **dispara la vuelta**: el ejército pasa a `regresando` y desanda su ruta hacia el asentamiento de origen. No se teletransporta ni se desvanece — volver cuesta el mismo camino que costó ir, y sigue comiendo del carro durante el regreso.

**Esto vale para un EJÉRCITO. Una Columna personal no cancela: rectifica** (5.12.1). Un jugador solo hace clic en otro punto y su ruta se recalcula desde donde esté, tantas veces como quiera. Un ejército no tiene esa opción, y por eso cancelar —volver a casa— es su única salida de una marcha empezada: el rumbo lo acordaron varios.

**Cancelar es del Líder, y solo suyo** (5.14.3). El rumbo lo acordaron varios y deshacerlo no puede ser cosa de uno cualquiera. No atrapa a nadie: el que no quiera seguir **se separa**, que eso sí puede hacerlo cualquiera.

Y es la razón por la que **deshacer un ejército es replegarlo, no vaciarlo**: el último miembro no puede separarse (5.14), así que la vuelta es el camino.

### 5.12.7 Qué ve el jugador — la vista

Un jugador ve de lo AJENO lo que alcancen sus tres clases de ojos:

1. **Sus plazas**, que vigilan su radio de influencia **más 60** unidades de mapa. La ciudad mira algo más allá de su frontera, como una atalaya.
2. **Sus ejércitos en marcha**, que ven **150** a la redonda.
3. **Él mismo, viajando solo**, que ve **80**. Menos que una columna a propósito: un hombre solo no despliega batidores. Es bastante para viajar sin caer a ciegas en una emboscada, y poco para que el explorador solitario sea la mejor unidad de información del juego.

**Los dos números solo se entienden comparados.** Que la columna vea más lejos que la plaza es lo que mantiene el valor de explorar: un ejército divisa una ciudad mucho antes de que la ciudad lo divise a él, y conserva la iniciativa. Y el margen de la plaza coincide con la distancia a la que un ejército puede repostar en ella (5.13), lo que da una regla fácil de recordar: **si una columna está lo bastante cerca como para repostar en tu ciudad, tu ciudad la ve.**

Sobre el mundo de 2000×2000, los 150 del ejército son unas **dos provincias** (Doc 1.0a) — varias ciudades por delante si la geografía lo permite — y siguen muy por debajo del radio de cohesión de un reino (~5 provincias): ver no es lo mismo que controlar. Los 60 de margen **duplican el área vigilada de una plaza recién fundada**, así que se notan desde el primer minuto.

El margen de la plaza se mide sobre su **radio**, no sobre la silueta de su zona. La zona viene recortada por las fronteras con Facciones rivales (Doc 1.2), y ese recorte es político, no óptico: que un rival plante su frontera pegada a tu ciudad no ciega a tus vigías.

> **Consecuencia**: como el radio crece con el nivel (60 → 180), una plaza de **nivel 5 vigila 240 y ve más lejos que un ejército**. Es coherente con la ficción —una capital tiene atalayas— pero diluye la ventaja de explorar cerca de las grandes ciudades.

De lo ajeno se ve **quién es y dónde está, nunca su interior**:

| Lo avistado | Qué se sabe | Qué NO se sabe |
|---|---|---|
| **Ejército** | Dónde está, de qué Facción es, cuántos rombos lo componen (5.12.2) | Composición, poder, ruta, objetivo, suministro, de dónde salió |
| **Asentamiento** | Nombre, Facción, posición y **nivel** | Almacén, guarnición, edificios, colas de construcción, cargos |

El **nivel** de una plaza sí se ve porque una ciudad grande se ve grande desde fuera; no dice cuánta tropa tiene dentro, que es lo que decidiría un ataque.

**Salvo que te acerques a mirar.** Esa tabla describe lo que llega A DISTANCIA DE VISTA, y sigue siendo la regla general. Dentro del **anillo de inspección (40)** se puede pedir ver la composición de una columna ajena —qué tropas y de quién son— y de una caravana, si lleva escolta y qué recursos carga, nunca cuántos (5.12.3). No es gratis: **el inspeccionado recibe aviso de que lo están mirando**. La telemetría de rival que esta sección prohíbe se paga acercándose y delatándose, que es lo contrario de obtenerla desde el sofá.

**Las caravanas ajenas y neutrales solo se ven dentro del radio de visión.** Fuera de él están ocultas, y **sin memoria**: a diferencia de una ciudad, una caravana se mueve, así que una foto vieja no diría "aquí hubo una" — diría una mentira sobre dónde está ahora.

### 5.12.8 Qué recuerda el jugador — la memoria

Ver algo **es** conocerlo. No hay dos mecanismos: lo que esté al alcance de tus plazas o de tus columnas en un momento dado queda grabado, y comerciar con una ciudad no es una vía aparte — es otra forma de verla un instante.

De ahí salen los tres estados en que el jugador ve el mundo:

| Estado | Qué ve |
|---|---|
| **Nunca lo he visto** | Nada: ese trozo de mundo está tapado, terreno incluido |
| **No lo veo, pero lo vi antes** | La última foto que tomó, con la fecha en que la tomó |
| **Lo estoy viendo** | La ficha en vivo. Al dejar de verlo, cae al estado anterior |

**Lo recordado no caduca, se refresca.** No hace falta un plazo de caducidad porque el dato viejo se delata solo: la foto viaja con el instante en que se tomó, y la interfaz puede decir "última información: hace 3 horas". El jugador juzga si fiarse. Volver a ver la plaza actualiza la foto; borrar información que el jugador ya sabe se sentiría arbitrario, dejarla envejecer a la vista no.

**La memoria es de la FACCIÓN, no del ciudadano.** Lo que uno explora lo saben todos los suyos — es lo coherente con que la Facción propia ya se vea entera desde dentro. Un héroe sin Facción lleva su propia memoria, que se funde con la de la Facción al entrar en una o al fundarla (Doc 1.3).

**Lo que se ve gana a lo que se recuerda.** Cuando una plaza está a la vez avistada y en la memoria, lo que llega es la información en vivo: estar mirándola es mejor información que acordarse de ella.

**Lo que hay en el mundo tampoco es público.** Que algo no pertenezca a ninguna Facción no lo pone a la vista de todos: un **campamento de bandidos** en un bosque que nadie ha pisado, o un **camino comercial** que une dos ciudades al otro lado del mundo, es información que no se ha ido a buscar. Cada uno pasa por la niebla con la regla que le corresponde:

| Cosa del mundo | Se ve si | Por qué |
|---|---|---|
| **Camino comercial** | Lo has **explorado** | Es infraestructura estática, como el terreno: la calzada que recorriste sigue donde estaba aunque hoy no la mires |
| **Campamento de bandidos** | Lo estás **viendo ahora** | Aparecen y desaparecen, así que no tienen memoria: recordarlos enseñaría el que nació después de que te fueras y el que ya arrasó otro |

Un camino se conoce **entero o no se conoce**: haber recorrido un tramo revela qué dos plazas une, que es en la ficción lo que una calzada dice de sí misma.

**Con la ciudad se ve su FRONTERA.** Una zona de influencia está marcada sobre el terreno, así que quien pasa por delante la ve — y lo que se ve es la línea de verdad, con su forma real, no un círculo idealizado. Lo que la acota es la misma niebla que acota todo lo demás: **del contorno solo llegas a ver el tramo que cae en tierra que has explorado.** Pasar cerca de una frontera te enseña ese tramo, no el mapa político entero.

De una frontera que solo **recuerdas** queda hasta dónde llegaba, no su trazo exacto: la línea real está negociada con vecinos que quizá no conozcas, y de esos no sabes nada. Se dibuja aproximada, y se ve que lo es.

**Y sabes en tierra de quién estás cuando marchas.** Una columna que entra en territorio ajeno lo nota, aunque no alcance a ver la ciudad que manda ahí — que es posible: una capital de nivel 5 vigila 240 y una columna ve 150, así que hay una franja en la que estás dentro de sus dominios sin haberla divisado. Lo que se sabe es **de qué Facción es el suelo**, nunca dónde tiene su capital.

> **El terreno lo tapa el cliente de jugador, no el servidor.** La geografía no es información táctica —es la misma para todos— así que el servidor solo dice QUÉ has explorado y cada cliente decide cómo pintarlo. El de administración no tapa nada: es una herramienta de operación, no un jugador.

### 5.12.9 Ocupación tras la conquista

**Para qué sirve:** una plaza recién conquistada queda sin guarnición (5.15.5). La ventana de ocupación la protege durante un tiempo fijo para que no cambie de manos cada vez que pasa un ejército; pasado ese tiempo, quien quiera conservarla tiene que defenderla: trasladar allí su campamento y asignar guarnición, o estar dentro cuando la ataquen.

Diseño en `Consideraciones/Ocupacion_Post_Conquista_Definicion.md`. Cifras placeholder en `OCUPACION`.

**Al conquistar**:

1. **La plaza queda sin guarnición** (5.15.5): los escuadrones de los antiguos residentes, guarnición incluida, quedan a 0 unidades y se van con el campamento de su héroe; no hay captura. El ejército conquistador sigue siendo una columna fuera.
2. **Saqueo determinista** (sin azar): `pesants` y `artesanos` pierden `OCUPACION.fraccionSaqueoPoblacion` (nobleza intacta, huye/negocia); una fracción `OCUPACION.fraccionEdificiosDanados` de los edificios activos —por orden de id, **exentos Centro Urbano y al menos una Granja y una Leñera**— pasan a la cola marcados `danado`; cada recinto de muralla completo pierde `OCUPACION.fraccionDanoMuralla` de su `avance` (la muralla no cae, deja de dar el multiplicador defensivo pleno hasta repararse por la vía normal de obra).
3. `medidorMantenimiento` a 100 y se abre la **ventana de ocupación** (`Asentamiento.ocupacionHasta`, `OCUPACION.duracionMinutos`, mismo orden que el período de gracia de fundación).

**Un edificio `danado`** se reconstruye por la auto-construcción normal, pero al arrancar la obra cuesta solo `OCUPACION.fraccionCosteReconstruccion` del costo de catálogo y tarda esa misma fracción — se repara, no se levanta de cero. El flag se limpia al volver a activo.

**Durante la ventana** (mientras `instante < ocupacionHasta`):

| Efecto | Valor |
|---|---|
| Inmune a un nuevo asedio | rebota sin combate ni azar; el atacante acampa |
| Recaudación de oro | `× OCUPACION.factorRecaudacion` |
| Crecimiento de población | el factor de felicidad `× OCUPACION.factorCrecimiento` |
| Mantenimiento | no degrada (misma rama que el período de gracia) |

**Al vencer** (tiempo fijo, nada la acorta): se limpia `ocupacionHasta` y la plaza vuelve a las reglas normales. La defiende quien se haya instalado: los héroes conquistadores que trasladaron allí su campamento, con su guarnición y en persona. Sin nadie, un asedio se juega sin defensores (5.12.4). Consolidar la conquista —reclutar escuadrones nuevos ahí, asignar guarnición, ejercer cargos, recaudación al 100%— exige residir en la plaza tomada (`cambiarResidencia`, Doc 2.5).

**Supervivencia:** el saqueo nunca toca el Centro Urbano ni deja al asentamiento sin una Granja y una Leñera activas, el mantenimiento queda suspendido toda la ventana y la reconstrucción es barata — para que un asentamiento pequeño saqueado no colapse por la penalización.

## 5.13 Suministro en campaña

**Un ejército en marcha NO come del almacén de su asentamiento.** Lleva su propio **carro de suministros** con la comida que consume mientras se mueve. Si se queda sin comida, la moral colapsa y los soldados desertan — exactamente la misma regla del hambre que en guarnición (5.4), solo cambia de qué despensa se come.

- **Capacidad**: **FIJA e igual para todos los Héroes** — es un carro, no una abstracción proporcional a lo que llevas. Se **suma** al formar ejército: un ejército de cuatro lleva cuatro carros.
- **Carga**: al salir o al unirse, cada Héroe **toma del asentamiento**. Si el almacén no llega, se sale con menos autonomía; no se bloquea la salida. Sacar un ejército **cuesta stock real** al asentamiento.
- **Reabastecimiento en ruta**: al pasar por un asentamiento **propio**, siempre. Por uno **aliado**, solo si ese asentamiento tiene la opción activada. Por uno neutral u hostil, nunca. "Al pasar" es estar dentro del **radio de reabastecimiento**, el mismo que decide dónde puede un ejército recoger refuerzos (5.12.1): por dónde puede pasar a recogerte y dónde puede repostar son la misma geografía.

  Repostar rellena el carro con las **mismas dos reglas que cargarlo al salir**: hasta donde quepa, y sin bajar nunca de la reserva de comida de la plaza que lo da. Abrir el almacén a un aliado **cuesta stock real**, y por eso es una decisión suya y no un derecho del que pasa — quien manda o custodia el tesoro de esa plaza (Gobernador o Tesorero) la toma, y puede cerrarla cuando quiera. Cerrar no es retroactivo: lo repuesto está repuesto.

  Sin límite de veces. Un ejército acampado junto a una plaza amiga repone cada tick, y **eso es lo que convierte "sostener un paso de montaña" en una posición** (5.12.3) en vez de una cuenta atrás.
- **Regreso**: el sobrante **vuelve al almacén** del asentamiento de origen. **El carro NO se descarga en ruta ni en otro asentamiento** — si pudiera, el ejército sería un transporte de mercancías gratuito que dejaría sin sentido a las caravanas. Para mover carga está el punto siguiente.

### 5.13.1 El radio operativo es la constante de diseño

La capacidad del carro **no es un número elegido, es una consecuencia**. La regla que la fija:

> **Un jugador solo, con su carro, tiene que poder recorrer al menos un cuarto del mapa ida y vuelta con la comida que carga.**

Sobre el mapa de 2000×2000 eso son 1.000 unidades de recorrido. Una carga máxima de Liderazgo son ~70 soldados, que a velocidad ligera (20) tardan 50 ticks en ese trayecto y comen `70 × 0.15 × 50 = 525`. De ahí sale la capacidad del carro.

Como la autonomía se mide en **ticks** y no en distancia, **la velocidad pasa a ser también un atributo logístico**: un ejército rápido cubre más mapa con la misma comida. Un ejército pesado tiene la mitad de alcance con el mismo carro — y por eso necesita caravanas.

### 5.13.2 Caravanas adjuntas al ejército

Un ejército puede llevar **caravanas adjuntas** que amplían su capacidad de carga más allá de la suma de los carros de sus héroes. Es la forma de proyectar una campaña lejos sin depender de que se unan más héroes.

- La capacidad de una caravana es **igual o mayor que la del carro de un jugador** — si cargara menos, la caravana no tendría sentido como tren de suministros. Son iguales: 500.
- **Se engancha y se suelta en marcha**, con cuatro condiciones: que sea de tu Facción, que esté **disponible** (una caravana ya despachada está cumpliendo un trueque y secuestrarla lo rompería), que esté al alcance —el mismo radio con el que se recogen refuerzos y se reposta— y que no vaya ya enganchada. Al soltarla se queda **donde esté la columna**: no vuelve sola a casa, igual que un ejército no se teletransporta al replegarse.
- Mientras va enganchada **viaja con el ejército**: su posición es la de la columna, no una ruta propia.
- **Entran en el `min` de velocidad.** Una caravana comercial va a 16, así que adjuntarla baja un ejército ligero de 20 a 16 y **le quita la capacidad de cazar caravanas**. No se puede tener alcance profundo y velocidad de incursión a la vez.
- **Cuestan comercio.** El cupo de caravanas de un Mercado es 2/4/6 según su nivel (Doc 3): enganchar la flota a un ejército es apagar tu comercio mientras dure la campaña.
- **Si el ejército es derrotado, las caravanas adjuntas se pierden.** Eso convierte el tren de suministros en un objetivo militar de verdad: cortar la retaguardia gana campañas sin asaltar una muralla. Se pierden igualmente si la columna **se disuelve** por quedarse sin nadie dentro (5.13.4): en los dos casos deja de existir en campo abierto, y lo que vuelve a casa son las *identidades* de sus escuadrones, no bienes físicos — una caravana sin nadie que la lleve no se teletransporta a ninguna parte. *(Distinto es que se quede sola porque su gente se desconectó sin llegar a disolverla: entonces vuelve a su origen, Doc 3.10.)*

### 5.13.3 Escolta de caravanas

Una caravana adjunta **puede ir cargada de mercancía y hacer su entrega** mientras marcha con el ejército. Es la escolta de caravanas por ejército (Doc 3.10): la caravana viaja protegida por el poder de combate del ejército en vez de por su defensa base fija.

**Una caravana enganchada deja de ser automática.** No la reparte el sistema por score: el jugador que la engancha **elige qué carga y a dónde la lleva**. Son viajes conscientes.

En la práctica son tres actos del jugador:

1. **Enganchar** una caravana disponible, estando la columna a su alcance.
2. **Cargar** lo que quiera del almacén de una plaza que le abra la puerta — la propia siempre, una aliada si tiene la opción activa. La misma geografía con la que reposta el trigo.
3. **Entregar**, al llegar: interactuando con un asentamiento se ven los **trueques activos**, cuánto falta **por tu parte** en cada uno, y se entrega de lo que la caravana lleve. Se entrega el menor de lo cargado y lo que falta, y el destino cobra su comisión exactamente igual que en una entrega automática — el camino manual no es una puerta trasera con otras reglas.

**Los bandidos atacan a la columna, no a la caravana**: una caravana escoltada se resuelve contra el poder de combate del ejército y no contra la defensa base fija. Si no, escoltar no protegería de los bandidos.

La regla de velocidad lo equilibra sola: escoltar baja el ejército a la velocidad de la caravana, así que **no se puede escoltar y depredar a la vez**.

**Escolta SIN héroe (Doc 3.13.4).** Una tercera vía: un héroe residente del origen **cede escuadrones** a la caravana por viaje (`Caravana.escolta`), sin marchar con ella. Salen de su campamento, cuentan Liderazgo, no comen ración y combaten manejados por la IA del juego. Siguen con la caravana hasta que termina el viaje o es destruida; si se destruye, quedan a 0 unidades y vuelven al campamento de su héroe (5.15.4). Es distinto de esta escolta por ejército (que exige un héroe en columna) y de la defensa base fija (Doc 3.10): tres capas de defensa.

### 5.13.4 Una columna se disuelve cuando no queda NADIE dentro

Regla necesaria por el TIEMPO REAL. Un minuto de juego es un minuto real, así que una
columna que nadie atiende seguiría existiendo y consumiendo durante horas. Sin una regla que la retire
quedaría un **ejército fantasma** marchando indefinidamente, llegando a su destino y atacando con poder 0.

**Lo que la disuelve es quedarse sin gente, no sin soldados.**
Son dos cosas distintas y confundirlas costaba las dos mitades:

| | Qué pasa |
|---|---|
| Sus escuadrones caen todos | **Sigue existiendo.** Sus héroes están dentro, solo que ahora viajan sin tropa: a la velocidad del viajero (5.12.5) y comiendo su ración de héroe. Lo que no puede es combatir |
| No queda ningún héroe dentro | **Se disuelve** |

Al disolverse, las identidades de escuadrón —vacías pero con su nombre, su nivel y su experiencia— **vuelven
al campamento de su héroe**, que es donde se pueden rellenar reclutando (5.4). No se pierden: lo que murió
son las unidades, no el squad. Si su héroe no tiene campamento (huérfano, Doc 0), siguen con él hasta que
vuelva a residir en algún asentamiento.

**Quién vacía una columna, entonces:** separarse (5.14.2) y desconectarse (Doc 1.10.6) — y el último que
queda no puede separarse, así que en la práctica una columna atendida no se disuelve nunca sola: vuelve a
casa. La que se vacía de verdad es aquella cuya gente se desconectó, que es justo el caso que esta regla
nació para cerrar.

## 5.14 Unirse y separarse en campo

Un Héroe que se cruza con un Ejército en el camino **puede unirse a él**, y un Héroe que va en un Ejército **puede separarse** y seguir por libre. Las dos se ofrecen desde el menú de interacción (5.12.3), igual que inspeccionar o perseguir.

### 5.14.1 Unirse en campo

Es distinto de unirse pasando el ejército por tu asentamiento (5.12.1), y conviene no confundirlos:

| | Unirse en tu plaza | Unirse en campo |
|---|---|---|
| Qué exige | Que el ejército pase cerca de **tu asentamiento** | Que estéis **uno junto al otro** en el mapa |
| Qué aportas | Tropas frescas sacadas **de casa** | Lo que **ya llevas encima**: tus escuadrones y tu carro |

En ambos casos lo que aportas se valida contra tu propio Liderazgo (5.11).

**Quién puede unirse: solo gente de la MISMA Facción.** Ni neutrales ni **aliados** — una columna la componen compatriotas, y su bandera es la de todos los que van dentro.

Es una frontera deliberada, y tiene precio: **una alianza no tiene brazo militar conjunto.** Dos Facciones aliadas que quieran hacer campaña juntas marchan como **dos columnas separadas**, y como una batalla enfrenta a una entidad contra otra, no suman: pelean por turnos. Concentrar fuerza dentro de una Facción vale más que sumarla entre dos. Las alianzas se ejercen en el comercio y en la no agresión (Doc 2), no dentro de la misma columna.

Lo que sí pueden hacer es **compartir camino sin riesgo**: dos columnas aliadas que se cruzan no se atacan (5.12.3). Marchan juntas, llegan juntas, y pelean cada una lo suyo.

**Y con permiso, según lo que decidiera el Líder al formar la columna.** Al crear un Ejército se fija su política de unión, y no cambia:

| Política | Qué hace |
|---|---|
| **Rechazar** | Nadie se suma en campo. La columna sale con quien salió |
| **Aceptar** | Cualquiera que cumpla lo de arriba se suma sin preguntar |
| **Preguntar al Líder** | La petición le llega al Líder, que acepta o rechaza. **Vive 10 segundos**: si nadie contesta, se da por rechazada |

Sin esto, un desconocido podría engancharse a tu marcha sin que pudieras negarte — y el que llega comparte tu carro, tu destino y tus encuentros.

**El silencio es un no, y es breve a propósito.** Diez segundos son pocos para que el que pide no se quede esperando plantado en mitad del mapa, y bastantes para que un grupo que está hablando se organice. La consecuencia hay que asumirla: *preguntar* solo funciona con el Líder al teclado, así que una columna que marcha en serio elegirá casi siempre *aceptar* o *rechazar*.

**Y la distancia se comprueba al aceptar, no al pedir.** El que entra tiene que estar junto a la columna en ese momento, no donde estaba cuando lo pidió.

**Y ahí está el precio:** al unirte adoptas un destino que ya no puedes cambiar (5.12.1). Marchar acompañado cuesta la libertad de movimiento.

**Solo se puede unir a un EJÉRCITO.** Dos Columnas personales que se cruzan no se fusionan — un ejército solo se origina en un asentamiento (5.12.1).

### 5.14.2 Separarse

Sales del Ejército con lo tuyo —tus escuadrones y **hasta un carro** de suministro, que es lo que aportaste— y naces como **Columna personal** en la posición donde estabas. Recuperas la libertad de movimiento al instante.

**El origen NO cambia.** Tu columna nueva conserva el asentamiento del que salió el Ejército, no el tuyo: el destino puede cambiar, el origen no. Es donde te replegarás. *(Es también lo que ya ocurría al unirse pasando por tu plaza — un jugador que se suma a un ejército de otra ciudad vuelve con él a esa ciudad, no a la suya.)*

**El Líder no puede separarse.** Para irse tiene que **elevar a otro integrante a Líder**, y entonces sí.

De esa regla sale sola otra que no hace falta escribir aparte: **un Ejército nunca se queda vacío en campo abierto**, porque el último que queda es siempre su Líder —los demás se fueron cediendo el mando o sin tenerlo—. Deshacerlo es **cancelar y volver** (5.12.6), no irse vaciándolo — si no, sus caravanas adjuntas, su suministro y sus escuadrones quedarían abandonados en mitad del mapa.

### 5.14.3 El Líder

Es el Héroe que **formó** el Ejército. No es un cargo político (Doc 2.2): es el mando de una columna concreta mientras dura su campaña. Hace cuatro cosas:

1. **Fija la política de unión** al formarla (5.14.1).
2. **Responde las peticiones** cuando esa política es *preguntar*.
3. **Cancela la marcha** — es suya en exclusiva (5.12.6).
4. **No puede separarse** sin ceder antes el liderazgo (arriba).

**Que cancelar sea solo suyo no atrapa a nadie**, porque cancelar no es la salida de la marcha: es hacer volver a todos. El que no quiera seguir **se separa**, y eso puede hacerlo cualquiera. Lo que el mando decide no es quién se queda, sino si la columna entera da media vuelta.

**Si el Líder se desconecta, el mando pasa al integrante con más ANTIGÜEDAD** — y si ese también está fuera, al siguiente. No puede irse por su voluntad, pero sí puede caérsele la conexión, y una columna en campaña no puede quedarse sin nadie que la mande. **Nunca se queda sin candidato**: todos los que van dentro son de su misma Facción y cualquiera vale, así que mientras quede alguien hay Líder — y si no queda nadie, la columna ya se disolvió (Doc 1.10.6).

Y como ser Ejército es una identidad y no un recuento (5.12.1), **el que se queda solo sigue en un Ejército**: mantiene la ruta fija y las caravanas. No hereda la libertad del viajero por quedarse sin compañía.

## 5.15 Batallas con héroes: quién combate, guarnición y campamento

Las batallas se juegan como partidas reales en Unity. Los datos que se intercambian con Unity están en `Docs/Coordinacion/01_Modelo_de_datos_compartido.md`.

### 5.15.1 Una escuadra combate con su héroe

Las batallas no son automáticas, salvo NPC contra NPC, que se resuelve con números. **Una escuadra solo combate si su héroe entra en la batalla y la usa**, y cuántas lleva lo limita su Liderazgo (5.11). En un asedio atacan héroes con sus escuadras y defienden los héroes presentes en el asentamiento, cada uno con las escuadras que le permite su Liderazgo. El Liderazgo equilibra los dos bandos.

Las escuadras de un héroe que no está en la batalla no combaten, estén donde estén. Solo hay dos excepciones, en las que una escuadra combate sin su héroe, manejada por la IA del juego: la **guarnición** de un asentamiento (5.15.3) y la **escolta** de una caravana (5.15.4).

Los topes de una batalla cuentan **héroes**: **15 por bando en un asedio**, y **5** en mundo abierto, contra una caravana o contra un campamento de bandidos. Los héroes que sobran esperan en cola y entran a medida que caen otros. Las escuadras sin héroe no ocupan plaza.

Una partida dura como mucho **30 minutos en un asedio** y **15 en el resto**. Si agota su tiempo, **gana el defensor**, en cualquier batalla; el atacante es siempre quien inicia el combate. En un asedio gana la plaza (5.2); en campo abierto, el atacado; contra una caravana, la caravana; y contra un campamento de bandidos, los bandidos: los héroes que lo atacaron pierden.

### 5.15.2 El campamento del héroe

El campamento es donde un héroe guarda las escuadras que no lleva consigo, y está en el asentamiento donde reside. Si no reside en ninguno, no tiene campamento: es **huérfano**, y sus escuadras siguen siendo suyas, con su experiencia y su nivel, hasta que vuelva a residir en algún asentamiento.

### 5.15.3 La guarnición

- Un héroe puede asignar algunas de sus escuadras a la guarnición del asentamiento **donde reside**, y solo ahí.
- Tiene un **cupo propio por héroe** que decide el asentamiento: sus edificios y políticas dan "liderazgo de guarnición" a cada héroe residente, en la misma escala que el coste de Liderazgo de las tropas (5.11.1: leva 7, línea 14, veterana 22, pesada 32, élite 45). **No gasta el Liderazgo del héroe**, que sigue siendo lo que lleva consigo.

  | Fuente | Cupo de guarnición |
  |---|---|
  | Barracón, nivel 1 / 2 / 3 | 7 / 14 / 22 |
  | Galería de tiro, nivel 1 / 2 / 3 | 7 / 14 / 22 |
  | Recinto de muralla completo | +14 |
  | Política "Levas de guarnición" (General, Doc 4.4) | +14 |

  Sin Barracón ni Galería de tiro no hay guarnición. Con todo al máximo son 72 —unas dos escuadras pesadas y una de leva—, por debajo de los 100 de Liderazgo que el héroe se lleva consigo. Como el cupo es por héroe, cuantos más residentes tenga una plaza, más guarnición puede tener.
- Una escuadra en guarnición se **entrega a la IA**: la maneja la IA y el héroe no puede usarla mientras siga asignada, aunque esté presente en la batalla.
- En un asedio entra directamente, sin ocupar plaza de héroe.
- Si el cupo baja por debajo de lo que ya tiene asignado (expira la política, se daña un edificio), lo asignado se queda; solo se impide asignar más.
- Si no hay héroes defensores presentes, la guarnición es la única defensa: la batalla se juega igual, con ella, o sin defensores si no hay guarnición.

### 5.15.4 La escolta

Una escuadra cedida como escolta sigue con la caravana hasta que esta termina el viaje o es destruida. **Si la caravana es destruida, por el motivo que sea, la escuadra queda a 0 unidades y vuelve al campamento de su héroe** (a donde resida).

### 5.15.5 Cuando cae el asentamiento

Si los atacantes conquistan un asentamiento:

- **Cada héroe defensor queda fuera**, en el mundo, junto al asentamiento, con las escuadras que usó en la batalla (las que sobrevivieron), en su propia columna y **con el carro vacío**.
- **Nadie se queda dentro de un asentamiento enemigo** (decisión del usuario, 2026-09-14): los que estaban dentro sin defender también salen. Un visitante vuelve a la columna que dejó aparcada; un residente herido sale solo, y sus escuadras, que no combatieron, corren la suerte del resto del campamento.
- **El resto de las escuadras de sus residentes, guarnición incluida, quedan a 0 unidades** y se van con el campamento de su héroe al asentamiento más cercano de su Facción, **donde el héroe pasa a residir**. Si la Facción no tiene ninguno, quedan a 0 unidades y sin asentamiento (huérfanas), pero siguen siendo de su héroe, con su experiencia y su nivel, hasta que el héroe traslade su campamento a otro asentamiento (ciudadanía, Doc 2.5).
- **No hay captura**: la guarnición no pasa al conquistador. Cae a 0 y sigue siendo de su héroe.
- **El asentamiento conquistado queda sin guarnición**. Nadie lo guarnece solo por haberlo ganado: los héroes conquistadores pueden trasladar allí su campamento (pasar a residir) y asignar guarnición dentro del cupo que dé el asentamiento. **Si se quiere defender algo, hay que defenderlo activamente.** La ventana de ocupación (5.12.9) sigue dándole inmunidad durante un tiempo fijo; después, sin guarnición ni defensores presentes, un asedio se juega sin defensores.

### 5.15.6 Facciones NPC

Las Facciones NPC las crea el admin, ya asentadas, y así siguen hasta que se destruyen: una Facción de jugador nunca pasa a la IA. Tienen **héroes bot**, manejados por la IA del juego: nacen como los fundadores de su primer asentamiento (5, el primero como Rey) y viven en el servidor como un héroe más. Un humano que ataca a una Facción NPC combate contra sus héroes bot y su guarnición. NPC contra NPC se resuelve con números, sin partida en Unity. Los héroes bot no usan la guarnición: defienden su plaza con su loadout activo mientras están en ella.

**Un héroe que ataca un campamento de bandidos también combate en Unity**: su columna contra las tropas del campamento, manejadas por la IA del juego (Doc 1.9).

Un bando puede no tener ningún humano (solo héroes bot, solo guarnición o escolta, o nadie); la batalla necesita al menos un héroe humano en total.

## 5.16 El Héroe

El detalle de datos del héroe está en `Docs/Coordinacion/01_Modelo_de_datos_compartido.md` §12-§14.

### 5.16.1 Un héroe por jugador y mundo

- Cada Jugador tiene **exactamente un Héroe en cada mundo**. El Héroe pertenece a esa partida, no a la cuenta: su progreso no pasa de un mundo a otro.
- **No se cambia de héroe** dentro de una partida.
- Todo lo que el Jugador hace dentro del mundo lo hace su Héroe: reside, lidera, tiene Liderazgo, está situado en el mundo, recuerda y explora, ocupa cargos y posee los escuadrones. El Jugador es la persona y su cuenta.
- Trae de Conquest su estructura de personaje: clase, nivel y experiencia, puntos de atributo y puntos de perk por separado, atributos, perks, equipo, inventario y sus propias monedas. Esas monedas **no tienen relación con el oro** recurso de BronzeAge.

### 5.16.2 Sus escuadrones

- **Todos los escuadrones son del Héroe**, estén donde estén: en su campamento, en su columna o de escolta (5.15.2).
- **Como mucho un escuadrón de cada tropa en toda la partida.** Si ya tiene uno de ese tipo, solo puede reponerlo allí donde está; no se recluta otro mientras el primero esté fuera.
- **Solo recluta las tropas que le permite el asentamiento.** El Héroe no tiene tipos de tropa desbloqueados propios.
- Las bajas de un escuadrón son siempre **permanentes**: los escuadrones no tienen estado de herido.

### 5.16.3 Progresión

- Héroe y escuadrones progresan con **nivel y experiencia**, al estilo de Conquest.
- **La experiencia del Héroe depende de su desempeño en la batalla**: unidades y héroes abatidos, capturas de bandera, daño hecho y recibido, si fue el mejor de la partida, en qué puesto de la tabla de su bando terminó, entre otros factores. La calcula la propia partida (Unity), que es la única que ve esos datos. **La de los escuadrones también**: la calcula la partida con toda la información de la batalla.
- **El botín también lo decide la partida**: qué objetos y cuántas monedas gana cada héroe en una batalla lo calcula Unity, igual que la experiencia, y el héroe lo guarda en su inventario y sus monedas.

### 5.16.4 Herido

"Herido" es un estado del **Héroe**, no de sus escuadrones ni de su columna:

- Lo sufren **todos los héroes del bando que pierde una batalla**, sea cual sea: asedio, mundo abierto o caravana. No hace falta que nadie lo marque: sale del resultado.
- Dura **2 minutos** de tiempo de mundo.
- Mientras dura, el héroe **no puede ser perseguido, no puede perseguir y no puede entrar en batallas**.
- Como es por héroe, una columna se puede atacar mientras lleve algún héroe sano; los heridos no entran en esa batalla. Si todos los héroes de la columna están heridos, nadie puede tocarla.
- Las escuadras de un héroe herido tampoco combaten: sin su héroe solo combaten la escolta de una caravana y la guarnición (decisión del usuario, 2026-09-14). Si una columna con algún herido pierde en mundo abierto, pierde igualmente la mitad entera del carro, porque el carro es de la columna.
- En un asedio, el loadout de un residente herido no defiende (la guarnición sí). Un ejército con todos sus héroes heridos espera a la puerta a que alguno sane antes de asediar (decisión del usuario, 2026-09-14).
- Perder cuenta también cuando una caravana se zafa del ataque o un campamento de bandidos aguanta. Sin combate —una plaza que cae sin defensores, o una ocupada que rebota el asedio— no pierde nadie.

### 5.16.5 Loadout

Traído de Conquest: el Héroe puede guardar selecciones de escuadrones (**loadouts**) para salir a mundo abierto o unirse a un Ejército. Cada una está limitada por su Liderazgo (5.11). Es una comodidad: no cambia lo que puede llevarse, solo lo deja preparado.

### 5.16.6 Derrota

Un héroe derrotado en una partida real pierde **parte de las tropas que llevó a la batalla** (las bajas que hayan sufrido sus escuadrones) y, **si la batalla fue en mundo abierto, la mitad de lo que lleva en el carro**. La mitad del carro se pierde igual en una columna personal que en un ejército, cuyo carro es el de todos sus miembros sumado. Además, queda **herido** (5.16.4).

### 5.16.7 Qué ven los demás de un héroe

- **Público:** nombre, clase, nivel, si está herido (y hasta cuándo), los escuadrones que lleva consigo (tipo, unidades y nivel) y el equipo que lleva puesto.
- **Privado, solo para su jugador:** todo lo demás — experiencia, puntos sin gastar, atributos, perks, Liderazgo, residencia, los escuadrones de su campamento, loadouts, inventario, monedas, género, avatar y si es humano o bot.
- **Entre compañeros de Facción:** cada ciudadano conoce el **nombre** de todos los demás ciudadanos de su Facción, los vea o no (decisión del usuario, 2026-09-14). El resto de su ficha sigue la regla de lo público.

"Público" significa que lo ve cualquiera que pueda ver al héroe. Dónde está un héroe sigue sujeto a la niebla de guerra (5.12.7), y su memoria y exploración son solo suyas (5.12.8).
