# Glosario de Entidades Políticas

Consultar este documento ante cualquier duda de terminología — el resto de documentos asume estas definiciones.
## Jugador
Persona real que juega. En cada mundo juega con **un único Héroe** (abajo): todo lo que hace dentro del mundo —residir, liderar, poseer escuadrones, ocupar cargos, combatir— lo hace su Héroe. Cuando el canon dice que un jugador hace algo en el mundo, lo hace a través de él.

## Héroe
El personaje con el que un Jugador está en el mundo (Doc 5.16). **Cada Jugador tiene exactamente un Héroe en cada mundo**: pertenece a esa partida, no a la cuenta, y no se puede cambiar por otro dentro de ella. Pertenece a 1 y SOLO 1 Facción en todo momento, y reside en 1 y SOLO 1 asentamiento a la vez (Doc 2.5).

Es el dueño de sus escuadrones: **los escuadrones son del Héroe, no del asentamiento** (Doc 5.4), estén donde estén, y tiene como mucho UNO de cada tropa (Doc 5.8). Tiene un valor de **Liderazgo** (Doc 5.11) que limita cuántos puede llevarse consigo a la vez.

**Está SITUADO en el mundo** (Doc 1.10): en todo momento está dentro de un asentamiento, dentro de una columna en el mapa, o desconectado. No es un observador que flota sobre el mapa: **solo ve el interior del asentamiento en el que está físicamente**, y solo puede dar órdenes ahí. Nace en mundo abierto, en un punto aleatorio, y funda donde decide pararse.

**Combate por sí mismo**, como un personaje de Conquest: clase, nivel y experiencia, atributos, perks, equipo e inventario (Doc 5.1 y 5.16). No muere: al perder una batalla queda **Herido** (abajo) y, si fue en mundo abierto, entrega la mitad de su carro.

## Héroe bot
Héroe de una Facción NPC, manejado por la IA del juego. Nace como fundador del primer asentamiento de su Facción cuando el admin la crea (Doc 5.15.6), y vive en el servidor como un héroe más. Combate contra los héroes humanos en las partidas de Unity; NPC contra NPC se resuelve con números (Doc 5.15.6).

## Columna personal
Un Héroe solo, moviéndose por el mapa. Nace cuando sale de su residencia por su cuenta (Doc 1.10), lleve tropas o no. Es la misma ENTIDAD que un Ejército en el motor, pero **no es un Ejército** a efectos de reglas: elige destino libremente y lo rectifica cuando quiera, no puede llevar caravanas adjuntas, y nadie puede unirse a ella — dos viajeros que se cruzan no forman un ejército (Doc 5.12.1).

Se disuelve al entrar en su residencia, o al fundirse en un Ejército que se cruce (Doc 5.14).

## Escuadrón / Tropa / Unidad
Son **tres conceptos distintos** y conviene no mezclarlos — la jerarquía de entidades es **Héroe → Escuadrón → Unidad**, y "tropa" es el TIPO, no la instancia:

- **Escuadrón:** la entidad. El grupo de soldados que un Héroe recluta y comanda como un bloque. Un Héroe tiene muchos escuadrones, pero **no más de uno de cada tropa en toda la partida**, esté donde esté. Persiste como identidad (nombre, nivel y experiencia) aunque se quede sin unidades (Doc 5.4). Nunca queda herido: sus bajas son siempre permanentes. Es lo que se cuenta: "tres escuadrones".
- **Tropa:** el TIPO de escuadrón — "Milicia de lanceros", "Honderos", "Arqueros con arco compuesto"… El catálogo completo está en Doc 5.8. Determina el poder por unidad, el equipo que cuesta reclutarlo, cuántas unidades trae y su coste de Liderazgo (Doc 5.11). Un escuadrón **jamás cambia de tropa**: subir de nivel lo hace más fuerte, nunca lo convierte en otro (Doc 5.8).
- **Unidad:** cada soldado individual dentro de un escuadrón. Las bajas son permanentes (permadeath, Doc 5.4).

> **"Tropa" se usa además en sentido colectivo** en todos los documentos ("las tropas consumen raciones", "mantenimiento de tropas", "reclutar tropas"), igual que en castellano corriente. Eso es deliberado y no choca: en singular y referido a un catálogo es el tipo; en plural y genérico es el colectivo. **La entidad contable es siempre el escuadrón** — y por eso lleva ese nombre, para que "tropa" quede libre para los otros dos usos.
## Guarnición
Los escuadrones que cada Héroe residente **asigna** a la guarnición de su asentamiento, dentro de un cupo por héroe que dan los edificios y las políticas de ese asentamiento; no gasta su Liderazgo. Los maneja la IA del juego y defienden aunque su héroe no esté. El resto de escuadrones guardados en el asentamiento no combaten sin su héroe. Un asentamiento recién conquistado queda sin guarnición (Doc 5.15.3 y 5.15.5).

## Campamento
Donde un Héroe guarda los escuadrones que no lleva consigo; está en el asentamiento donde reside. Un Héroe sin residencia no tiene campamento: es huérfano (Doc 5.15.2).

## Ejército
Una columna nacida de **movilizarse contra un destino** desde un asentamiento, que se mueve por el mapa como **una sola entidad** (Doc 5.12). Lo que la hace un Ejército es cómo salió, no cuántos van dentro: puede empezar con uno solo y crecer según se le suman los demás. Cada uno sigue limitado por su propio Liderazgo, y **todos son de la misma Facción**: ni aliados ni neutrales pueden marchar dentro de una columna ajena (Doc 5.14.1).

**Solo se origina en un asentamiento, nunca en campo abierto**, y **ser un Ejército es una identidad, no un recuento**: uno al que se le van separando miembros hasta quedar en uno solo **sigue siendo un Ejército** — conserva su ruta fija y sus caravanas. Lo contrario también: una Columna personal no se convierte en Ejército porque alguien se le sume. La diferencia práctica está en Doc 5.12.1.

Un Ejército **nunca se queda vacío en campo abierto**: el último miembro es siempre su Líder y el Líder no puede separarse, así que la salida es cancelar y volver (Doc 5.14).

Mientras está en campaña, un ejército **no come del almacén de su asentamiento**: lleva su propio carro de suministros (Doc 5.13) — uno por Héroe, de capacidad fija. Puede además llevar **caravanas adjuntas** que amplían su alcance, y que pueden ir cargadas de mercancía (escolta de caravanas, Doc 5.13.2-5.13.3). Si el ejército es derrotado, entrega la mitad de su carro y las caravanas adjuntas se pierden con él.

Una marcha se puede **cancelar**, lo que dispara la vuelta por el mismo camino (Doc 5.12.6).

## Líder (de un Ejército)
El Héroe que **formó** el Ejército. No es un cargo político —no tiene nada que ver con los de Doc 2.2— sino el mando de una columna concreta mientras dura su campaña.

**El Líder no puede separarse** (Doc 5.14): para irse tiene que **elevar a otro integrante a Líder**, y entonces sí. De ahí sale, sin necesidad de enunciarla aparte, que un Ejército nunca se quede vacío en campo abierto — el último que queda es siempre el Líder.

Si se **desconecta**, el mando pasa solo al integrante con más antigüedad: no puede irse queriendo, pero una caída de conexión no deja a la columna sin mando. Y nunca falta candidato —todos los que van dentro son de su misma Facción—, así que mientras quede alguien hay Líder.

Fija además la **política de unión** de su columna al formarla, responde las peticiones cuando esa política es *preguntar* —y si no contesta en 10 segundos, la petición se da por rechazada—, y es **el único que puede cancelar la marcha**: el resto, si no quiere seguir, se separa.

## Herido
Estado de un Héroe tras perder una batalla. Lo sufren **todos los héroes del bando perdedor**, sea cual sea la batalla, y dura **2 minutos**. Mientras dura, nadie puede perseguirle ni atacarle, y él tampoco puede perseguir ni entrar en batallas: lo primero evita el acoso en cadena al mismo viajero; lo segundo, que la inmunidad se use de escudo para depredar sin riesgo. Como es por héroe, una columna se puede atacar mientras lleve algún héroe sano; si todos sus héroes están heridos, nadie puede tocarla (Doc 5.16.4).

Los escuadrones nunca quedan heridos: sus bajas son siempre permanentes.

## Huérfano
Héroe sin residencia y, por tanto, sin campamento: sin sitio donde guardar escuadrones, reabastecer ni reclutar. Al perder su asentamiento, un héroe pasa a residir en el más cercano de su Facción; solo queda huérfano si su Facción no tiene ninguno (Doc 5.15.5). Sus escuadrones siguen siendo suyos, con su nivel y experiencia, y deja de ser huérfano en cuanto vuelve a residir en algún asentamiento (ciudadanía, Doc 2.5).

## Liderazgo
Valor del Héroe que limita **cuánto puede llevarse consigo a la vez** — no cuánto puede poseer. Cada tropa tiene un coste de Liderazgo según su escalón; la suma de lo que un Héroe se lleva no puede exceder su Liderazgo (Doc 5.11). Lo que se queda en su campamento no defiende: solo la guarnición que asigna, que tiene su propio cupo (ver Guarnición). La selección de qué llevarse se puede guardar como **loadout** (abajo).

## Loadout
Selección de escuadrones que un Héroe deja guardada para salir a mundo abierto o unirse a un Ejército, limitada por su Liderazgo. Es una comodidad: no cambia lo que puede llevarse, solo lo deja preparado (Doc 5.16.5).

## Almacén y Granero

La despensa general de un asentamiento es su **almacén** (`Asentamiento.almacen`): guarda todos los recursos, y su capacidad la amplía el edificio **Almacén**. De ahí come la población y la tropa que no está en campaña, y de ahí se carga el carro de un ejército.

El **Granero** es un edificio que amplía solo la capacidad de **trigo**, mucho más que un Almacén (Doc 4.2.1). No confundir con el **rol** de especialización de un asentamiento que se dedica a producir trigo, que también se llama granero, junto a Aserradero, Cuenca Minera, Dehesa y Ciudad (`Consideraciones/Fase_0_5_Definicion_Especializacion_y_Cupos.md` §4): uno es un edificio; el otro, la vocación de un asentamiento entero.

## Asentamiento
Unidad territorial con zona de influencia, edificios (auto-construidos), población NPC (Pesants/Artesanos/Nobleza), y cargos LOCALES (Gobernador, Tesorero, General, Maestro de Obras, Sacerdote). Pertenece a 1 y SOLO 1 Facción. Una Facción puede tener muchos asentamientos.

## Facción (pieza central)
Entidad política soberana que agrupa héroes y asentamientos. Una Facción puede tener muchos héroes y muchos asentamientos. Tiene cargos de NIVEL FACCIÓN: Rey y Embajador. La ciudadanía de un héroe se extiende a TODOS los asentamientos de su propia Facción (no a la Liga completa).

El VASALLAJE y las ALIANZAS ocurren entre FACCIONES enteras, nunca entre asentamientos individuales sueltos. Un asentamiento nunca es vasallo por sí solo; es su Facción la que lo es o no.

## Liga
NO es una entidad con cargos propios ni ciudadanía propia. Es la RED de Facciones conectadas entre sí, sin importar si el vínculo interno de cada conexión es de Vasallaje o de Alianza.

## Vasallo / Señor
Relación ATADA entre dos Facciones (una Facción señora, una Facción vasalla).

## Aliado
Relación LIBRE entre dos Facciones, revocable en cualquier momento, con consecuencias.

## Gran Rey
Título dinámico de PRESTIGIO (no cargo mecánico nuevo) otorgado al Rey de la Facción cuya Liga incluye otras Facciones enteras sometidas como vasallas.

---

## Cargos (nivel Facción)
- **Rey:** autoridad sobre vasallaje, designación de Gobernadores, políticas superiores hacia los asentamientos de la Facción, y todas las funciones del Embajador. Por defecto en Liga-por-vasallaje; electo por voto en Liga-por-alianza. **Fase 0: una Facción SIEMPRE tiene Rey** — quien la crea lo es, y al abandonarla el trono pasa al siguiente ciudadano (ver Doc 2.2).
- **Embajador:** crea alianzas y declara guerras. Designado por el Rey.

## Cargos (nivel asentamiento, uno de cada por asentamiento)
- **Gobernador:** máxima autoridad del asentamiento, designa al resto de cargos locales. Canónicamente electo por ciudadanos; **en Fase 0 lo designa el Rey de la Facción** (ver Doc 2.2).
- **Tesorero:** gestión económica (acuerdos de trueque + Mercado).
- **General:** mando militar del asentamiento.
- **Maestro de Obras:** gestiona prioridades de auto-construcción, bonus de tiempos de construcción.
- **Sacerdote:** acelerador de aparición de nobleza + bonificación de felicidad.

## Población NPC: Pesants / Artesanos / Nobleza
Las 3 clases de población NPC de un asentamiento (distintas de los Héroes).
- **Pesants:** base, crecimiento rápido; trabajan los recursos genéricos y se reclutan como tropa.
- **Artesanos:** operan los edificios de transformación y también se reclutan como tropa.
- **Nobleza:** crecimiento lento; aparece con Palacio y un mínimo de ciudadanos (Héroes) en el asentamiento. Es la clase que más oro recauda. No se recluta (Doc 5.8).

## Aedas / Poetas
Mismo rol: los NPCs viajantes que dan acceso a tecnología (itinerantes o residentes si hay nobleza) Y narran el lore/histórico del servidor (títulos de prestigio, hazañas).

## Oro
Recurso/medio de intercambio: metal precioso en bruto pesado (no moneda acuñada), origen en minas ubicadas en el mapa. Se usa para servicios/NPCs (sueldos, mercenarios, tecnología) y también como recurso más de trueque entre Facciones.

## Nodo / Zona de Influencia
Al fundar un asentamiento se genera automáticamente su zona de influencia (también llamada nodo, usado como sinónimo del área territorial del asentamiento — NO como los "nodos" de grid fijo de proyectos previos; aquí la posición es libre en el mapa continuo).
