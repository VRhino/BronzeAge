# 7. Bots mercenarios

> Implementado el 2026-11-01 (entrada 48). El cómo exacto (fórmulas, orden de las comprobaciones, rechazos, hechos y cifras) está en
> [`Docs/Mechanics/Mercenarios_Por_Contrato.md`](../Mechanics/Mercenarios_Por_Contrato.md), que manda si los dos se contradicen. Todas las cifras son borradores que se calibran con bots.

## 7.1 Qué es y para qué sirve

Un mundo con pocos jugadores humanos tiene que seguir teniendo juego. El **bot mercenario** es un héroe bot que vive en un campamento de mercenarios y se ofrece en su tablón; un cargo
de una Facción humana lo contrata y, mientras dura el contrato, es un ciudadano más de esa Facción: defiende, escolta, ataca o forma parte de su ejército. Los bots juegan desde fuera del
motor, como cualquier jugador (Doc 12 de Arquitectura): el juego guarda el contrato, cobra, pone los límites y lleva su progresión, pero no comprueba si el bot cumple bien; para eso
están la valoración de quien le pagó y su historial.

- Los bots **libres** crean Facciones y juegan como siempre; los bots **mercenarios** no crean Facciones y viven en el bucle del tablón. Esa diferencia es del cerebro de los bots.
- **Las Facciones de bots no contratan**, y el contrato es la única vía por la que un bot entra en una Facción humana.
- No existe alquilar escuadras sin héroe, ni que un humano se ofrezca como mercenario.

## 7.2 Ciudadano con marca

El mercenario contratado es **ciudadano** de la Facción con una marca, **temporal** o **permanente**. Para todo lo demás (bando en las batallas, puertas, visión, convocatorias) es de la
Facción que le paga. La marca le pone límites:

- nunca es Rey ni Embajador ni tiene cargos locales;
- no deja la Facción por su cuenta: sale cuando acaba el contrato;
- nunca declara la guerra, ni funda, ni crea o pide Facción, ni acepta las misiones del tablón de la Facción o de la plaza;
- no gana Participación, aunque lo que hace cuenta para las misiones colectivas;
- el directorio cuenta solo a los ciudadanos sin marca, y si se van todos los ciudadanos sin marca, todos los contratos caen.

## 7.3 Ofrecerse y contratar

El bot se ofrece **dentro de su campamento**, sin Facción. Se le contrata **en persona**, en su campamento, o **desde la taberna de una plaza propia de su misma región del mapa** (más abajo): quien firma llega con su columna y paga con el oro de su carro o, si reside en ese
campamento, también desde dentro, con su almacén personal o con el carro que tiene allí (como en el mercado del campamento). La **tarifa** la fija
el juego por el nivel del héroe y lo que vale su tropa, multiplicada por el plazo y por su rango; el permanente es caro a propósito. **Desde el 2026-11-08 la tarifa es la cuarta parte** de lo que era (factor de oro del 25 %, provisional) y se puede pagar de dos maneras: **en oro** (el de su carro o su almacén personal, como arriba) o **con las monedas del héroe que firma**, en cualquier sitio y sin carro ni almacén, a 100 monedas por cada oro de la tarifa. El oro va al almacén personal del mercenario hasta
llenarlo (lo que no cabe se pierde) y nunca se devuelve; las monedas pagadas pasan a las del mercenario. Vale al contratar y al prorrogar.

**Contratar desde la taberna (2026-11-08).** La **taberna activa de una plaza** enseña, **gratis**, el tablón de los mercenarios de todos los campamentos de su región del mapa, cada oferta con el campamento donde se ofrece, y se les puede contratar desde allí sin ir al campamento, estando dentro de la plaza. Se paga con el **almacén de la plaza** (es lo que se toma por defecto; firma entonces el Rey, el Embajador o el Gobernador, como la intel de la taberna), con el **fondo de la plaza** (desde el 2026-11-12: las monedas del impuesto de su forja, Doc 5.16.1, a la tarifa en monedas; firman los mismos) o con las **monedas** del héroe que firma (el cargo que alcance el trabajo, como siempre). No se puede pagar allí con el almacén personal ni con el carro, el campamento ha de caer en la región de la plaza, y **prorrogar sigue siendo en persona**. El mercenario contratado recibe su primera orden y se pone en camino solo.

| Trabajo | Tipo | Quién lo firma |
|---|---|---|
| Defender una plaza, un pueblo o una avanzada | temporal (1, 3 o 7 días) | Rey, Embajador, o el Gobernador o el General de esa plaza |
| Escoltar una caravana (o carro de acarreo, encargo o ruta) | temporal | Rey, Embajador, o el Gobernador o el Maestro de Caravanas de esa plaza |
| Atacar un objetivo (bandidos, o algo de una Facción en guerra) | temporal | Rey, Embajador; el Gobernador, bandidos de su zona; el General, bandidos de su zona y enemigos en guerra |
| Acompañar una convocatoria o un ejército | temporal | Rey, Embajador, o el General a lo que encabeza |
| Ser parte del ejército de la Facción | permanente | Rey o Embajador |

El temporal sigue viviendo en su campamento; el permanente pasa a vivir en la plaza que elige quien firma. Un temporal se puede **prorrogar** en persona. Tras un **despido**, la
misma Facción no puede recontratarlo hasta pasados unos días.

## 7.4 Órdenes

Una orden le dice al bot qué hacer y le da permiso para lo que sin ella no puede: **sin orden, solo se defiende**.

| Orden | Le deja |
|---|---|
| **Guardar** algo suyo (plaza, pueblo, avanzada, caravana) | guarnecer esa avanzada aunque no viva en su madre, escoltar esa caravana, defenderlo en batalla |
| **Ir a** un lugar | nada nuevo |
| **Atacar** bandidos o algo de una Facción en guerra | atacarlo, perseguirlo, unirse a la batalla contra él y convocar un ejército **solo de mercenarios** |
| **Unirse** a una convocatoria o un ejército de su Facción | unirse a ese y a ningún otro |

Cada trabajo admite unas órdenes. Ordenan el Rey y el Embajador a todos; el General, el Gobernador y el Maestro de Caravanas, a los de su plaza y con menos órdenes. Una orden solo la
cambia quien tiene igual o más rango que quien la dio; la orden con la que nace el contrato no tiene rango. **Atacar es un solo ataque**: se cumple en su primera batalla contra ese
objetivo, gane o pierda.

## 7.5 Fin del contrato

Acaba por plazo, por despido, si la plaza que lo firmó se pierde, si en la Facción no queda nadie sin marca o si deja de ser su ciudadano. **En mitad de una batalla, el fin espera a
que acabe.** Al acabar se separa de su ejército donde esté, sale de la Facción y el permanente vuelve a su campamento. Si una plaza veta a un permanente que vive en ella, vuelve a su
campamento sin perder el contrato.

## 7.6 Progresión

El mercenario tiene experiencia propia, el **renombre**, que solo gana bajo contrato: al terminar un temporal sin despido, al ganar el ataque que le mandaron, al ganar una batalla
defendiendo lo que guarda, al llegar a destino la caravana que escolta y con cada medalla nueva. El del temporal se le apunta al acabar (con despido lo pierde); el del permanente, al
momento. Quien lo firmó o el Rey lo **valora** al acabar: buena suma la mitad otra vez, mala se lo quita.

| Rango | Le da |
|---|---|
| 1. A sueldo | al ofrecerse por primera vez, una escuadra suya de cada tropa de la leva (lanceros de milicia, granjeros, mineros y leñadores) |
| 2. Lanza · 3. Capitán · 4. Veterano · 5. Señor de la guerra | una **escuadra de premio** gratis por rango, la **licencia** de reclutar en su campamento las tropas de su edificio hasta ese escalón en cuanto alguien en el mundo descubre su tecnología, y una tarifa más alta; al llegar a Veterano y a Señor de la guerra, además, una tirada de la tabla de objetos de su rango, a su mochila (decidido el 2026-11-12) |

**El Liderazgo no cambia con el rango**: es igual para todos los héroes.

**Medallas**, con bronce, plata y oro, que no se pierden: Palabra cumplida, Cazador de bandidos, Muralla, Buen camino, Asaltante, Campaña, Bien valorado y Leal.

## 7.7 Lo que ve cada uno

El mercenario ve su contrato, su orden, su ficha y lo de su Facción a lo que apunta su trabajo, pero no los miembros, relaciones, tecnología ni el mapa de la Facción: un bot no se lleva
información de una Facción humana al mundo de los bots. Su memoria del mapa sigue siendo suya. Los ciudadanos ven los contratos de su Facción; todos ven si un héroe es mercenario, su
rango y sus medallas; el tablón, desde dentro del campamento o, gratis, desde la taberna de una plaza de su región, muestra a cada ofrecido con su ficha, su tropa y su tarifa. Nadie ve para quién trabajó ni quién le valoró.
