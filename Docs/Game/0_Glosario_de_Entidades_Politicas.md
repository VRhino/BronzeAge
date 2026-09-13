# Glosario de Entidades Políticas

Consultar este documento ante cualquier duda de terminología — el resto de documentos asume estas definiciones.

> **Modelo de Héroe (decisiones del usuario, 2026-09-11 y 2026-09-13 — diseño cerrado, sin implementar).** En el juego que se está construyendo con Unity, quien reside, lidera, posee escuadrones, ocupa cargos y combate es el **Héroe** (ver abajo, Doc 5.15 y 5.16), y el **Jugador** queda como la persona y su cuenta. Mientras no se implemente, este glosario y el resto del canon siguen diciendo "Jugador" para esas funciones: en el modelo nuevo, léase "Héroe" en todas ellas. Los párrafos existentes no se reescriben.

## Jugador
Persona real que juega. Pertenece a 1 y SOLO 1 Facción en todo momento, y reside en 1 y SOLO 1 asentamiento a la vez (Doc 2.5) — es lo que le permite tener como mucho UN escuadrón de cada tropa (ver abajo, y Doc 5.8).

Es además el dueño de sus escuadrones: **los escuadrones son del Jugador, no del asentamiento** (Doc 5.4). El asentamiento es donde están apostados, no quien los posee. Tiene un valor de **Liderazgo** (Doc 5.11) que limita cuántos puede sacar a campaña a la vez.

**Y está SITUADO en el mundo** (Doc 1.10): en todo momento está dentro de un asentamiento, dentro de una columna en el mapa, o desconectado. No es un observador que flota sobre el mapa: **solo ve el interior del asentamiento en el que está físicamente**, y solo puede dar órdenes ahí. Nace en mundo abierto, en un punto aleatorio, y funda donde decide pararse.

**Combate por sí mismo** con el poder de UNA unidad de élite (Doc 5.1). Es poco a propósito — menos de un tercio del escuadrón más barato— así que frente a una columna no decide nada; decide frente a otro jugador solo. No muere ni sufre bajas: al perder entrega la mitad de su carro y entra en **Tregua**.

## Héroe
*(Modelo de Héroe, diseño cerrado sin implementar — Doc 5.15 y 5.16.)* El personaje con el que un Jugador está en el mundo. **Cada Jugador tiene exactamente un Héroe en cada mundo**: pertenece a esa partida, no a la cuenta, y no se puede cambiar por otro dentro de ella. Hace todo lo que hoy el canon atribuye al Jugador como entidad de juego: reside en un asentamiento, lidera, tiene Liderazgo, está situado en el mundo, recuerda y explora, ocupa cargos y **es el dueño de los escuadrones**. Trae de Conquest su estructura de personaje: clase, nivel y experiencia, atributos, perks, equipo e inventario.

## Héroe bot
*(Modelo de Héroe.)* Héroe de una Facción NPC, manejado por la IA del juego. Combate contra los héroes humanos en las partidas de Unity; NPC contra NPC se sigue resolviendo con números (Doc 5.15.6).

## Columna personal
Un Jugador solo, moviéndose por el mapa. Nace cuando sale de su residencia por su cuenta (Doc 1.10), lleve tropas o no. Es la misma ENTIDAD que un Ejército en el motor, pero **no es un Ejército** a efectos de reglas: elige destino libremente y lo rectifica cuando quiera, no puede llevar caravanas adjuntas, y nadie puede unirse a ella — dos viajeros que se cruzan no forman un ejército (Doc 5.12.1).

Se disuelve al entrar en su residencia, o al fundirse en un Ejército que se cruce (Doc 5.14).

## Escuadrón / Tropa / Unidad
Terminología cerrada (a petición del usuario, 2026-09-02). Son **tres conceptos distintos** y conviene no mezclarlos — la jerarquía de entidades es **Jugador → Escuadrón → Unidad**, y "tropa" es el TIPO, no la instancia:

- **Escuadrón:** la entidad. El grupo de soldados que un Jugador recluta y comanda como un bloque. Un Jugador tiene muchos escuadrones, pero **no más de uno de cada tropa**. Persiste como identidad (nombre, veteranía) aunque se quede sin unidades (Doc 5.4). Es lo que se cuenta: "tres escuadrones".
- **Tropa:** el TIPO de escuadrón — "Milicia de lanceros", "Honderos", "Arqueros con arco compuesto"… El catálogo completo está en Doc 5.8. Determina el poder por unidad, el equipo que cuesta reclutarlo, cuántas unidades trae y su coste de Liderazgo (Doc 5.11). Un escuadrón **jamás cambia de tropa**: ganar veteranía lo hace más fuerte, nunca lo convierte en otro (Doc 5.8).
- **Unidad:** cada soldado individual dentro de un escuadrón. Las bajas son permanentes (permadeath, Doc 5.4).

> **"Tropa" se usa además en sentido colectivo** en todos los documentos ("las tropas consumen raciones", "mantenimiento de tropas", "reclutar tropas"), igual que en castellano corriente. Eso es deliberado y no choca: en singular y referido a un catálogo es el tipo; en plural y genérico es el colectivo. **La entidad contable es siempre el escuadrón** — y por eso lleva ese nombre, para que "tropa" quede libre para los otros dos usos.

> **Modelo de Héroe (2026-09-13):** el escuadrón es del Héroe, y "no más de uno de cada tropa" vale en toda la partida, esté el escuadrón donde esté. La veteranía se sustituye por **nivel y experiencia**, y un escuadrón nunca queda herido: sus bajas son siempre permanentes (Doc 5.16).

## Guarnición
Los escuadrones apostados en un asentamiento, es decir, los que NO salieron a campaña. Son los únicos que lo defienden de un asedio (Doc 5.12). Un asentamiento cuyos jugadores se llevaron todo queda indefenso.

> **Modelo de Héroe (2026-09-13):** cambia de sentido. La guarnición son solo los escuadrones que cada héroe residente **asigna** a su asentamiento, dentro de un cupo que dan los edificios y las políticas; los maneja la IA del juego. El resto de escuadrones guardados en el asentamiento no defienden sin su héroe. Tras una conquista el asentamiento queda sin guarnición (Doc 5.15.3 y 5.15.5).

## Campamento
*(Modelo de Héroe.)* Donde un Héroe guarda los escuadrones que no lleva consigo; está en el asentamiento donde reside. Un Héroe sin residencia no tiene campamento: es huérfano (Doc 5.15.2).

## Ejército
Una columna nacida de **movilizarse contra un destino** desde un asentamiento, que se mueve por el mapa como **una sola entidad** (Doc 5.12). Lo que la hace un Ejército es cómo salió, no cuántos van dentro: puede empezar con uno solo y crecer según se le suman los demás. Cada uno sigue limitado por su propio Liderazgo, y **todos son de la misma Facción**: ni aliados ni neutrales pueden marchar dentro de una columna ajena (Doc 5.14.1).

**Solo se origina en un asentamiento, nunca en campo abierto**, y **ser un Ejército es una identidad, no un recuento**: uno al que se le van separando miembros hasta quedar en uno solo **sigue siendo un Ejército** — conserva su ruta fija y sus caravanas. Lo contrario también: una Columna personal no se convierte en Ejército porque alguien se le sume. La diferencia práctica está en Doc 5.12.1.

Un Ejército **nunca se queda vacío en campo abierto**: el último miembro es siempre su Líder y el Líder no puede separarse, así que la salida es cancelar y volver (Doc 5.14).

Mientras está en campaña, un ejército **no come del almacén de su asentamiento**: lleva su propio carro de suministros (Doc 5.13) — uno por Jugador, de capacidad fija. Puede además llevar **caravanas adjuntas** que amplían su alcance, y que pueden ir cargadas de mercancía (escolta de caravanas, Doc 5.13.2-5.13.3). Si el ejército es derrotado, las caravanas adjuntas se pierden con él.

Una marcha se puede **cancelar**, lo que dispara la vuelta por el mismo camino (Doc 5.12.6).

## Líder (de un Ejército)
El Jugador que **formó** el Ejército. No es un cargo político —no tiene nada que ver con los de Doc 2.2— sino el mando de una columna concreta mientras dura su campaña.

**El Líder no puede separarse** (Doc 5.14): para irse tiene que **elevar a otro integrante a Líder**, y entonces sí. De ahí sale, sin necesidad de enunciarla aparte, que un Ejército nunca se quede vacío en campo abierto — el último que queda es siempre el Líder.

Si se **desconecta**, el mando pasa solo al integrante con más antigüedad: no puede irse queriendo, pero una caída de conexión no deja a la columna sin mando. Y nunca falta candidato —todos los que van dentro son de su misma Facción—, así que mientras quede alguien hay Líder.

Fija además la **política de unión** de su columna al formarla, responde las peticiones cuando esa política es *preguntar* —y si no contesta en 10 segundos, la petición se da por rechazada—, y es **el único que puede cancelar la marcha**: el resto, si no quiere seguir, se separa.

## Tregua
Estado temporal de quien acaba de ser derrotado en campo abierto (Doc 5.12.3). Dura unos minutos y **corta por los dos lados**: nadie puede perseguirle ni atacarle, y él tampoco puede perseguir ni atacar a nadie. Es lo que impide tanto el acoso en cadena al mismo viajero como usar la inmunidad de escudo para depredar sin riesgo.

## Huérfano
Jugador al que conquistaron su asentamiento mientras estaba de campaña (Doc 5.4). Conserva los escuadrones que llevaba encima, pero se queda sin residencia: sin sitio donde reabastecer, reclutar ni volver. Deja de serlo cuando entra en una Facción que tenga asentamiento — se sale por la vía política, no por la militar.

> **Modelo de Héroe (2026-09-13):** al perder su asentamiento, el Héroe pasa a residir en el asentamiento más cercano de su Facción, adonde se muda su campamento; solo queda huérfano si su Facción no tiene ninguno. Sus escuadrones siguen siendo suyos, a 0 unidades, con su nivel y experiencia, hasta que vuelva a residir en algún sitio (Doc 5.15.5).

## Liderazgo
Valor del Jugador que limita **cuánto puede sacar a campaña a la vez** — no cuánto puede poseer. Cada tropa tiene un coste de Liderazgo proporcional a su poder; la suma de lo que un Jugador se lleva no puede exceder su Liderazgo (Doc 5.11). Lo que se queda, defiende (ver Guarnición).

> **Modelo de Héroe (2026-09-13):** "lo que se queda, defiende" deja de ser cierto: solo defiende la guarnición asignada, que tiene su propio cupo y no gasta Liderazgo (Doc 5.15.3). Con el Liderazgo se prepara el **loadout** (abajo).

## Loadout
*(Modelo de Héroe, traído de Conquest.)* Selección de escuadrones que un Héroe deja guardada para salir a mundo abierto o unirse a un Ejército, limitada por su Liderazgo. Es una comodidad: no cambia lo que puede llevarse, solo lo deja preparado (Doc 5.16.5).

## Almacén (y por qué NO se llama "granero")

La despensa de un asentamiento es su **almacén** — `Asentamiento.almacen` en el motor, ampliable con el edificio `almacen` del catálogo. Ahí es donde vive el trigo, y de ahí come la guarnición.

**No existe ningún edificio llamado "granero".** La palabra sí está cogida, pero para otra cosa: un **Granero** es un *rol de asentamiento*, el que se especializa en producir trigo, junto a Aserradero, Cuenca Minera, Dehesa y Ciudad (`Consideraciones/Fase_0_5_Definicion_Especializacion_y_Cupos.md` §4). Es una **vocación de un asentamiento entero**, no un edificio suyo.

Así que "el ejército come del granero" no solo nombraba un edificio inexistente: colisionaba con un término del diseño que significa algo distinto. Se dice **almacén**.

## Asentamiento
Unidad territorial con zona de influencia, edificios (auto-construidos), población NPC (Pesants/Artesanos/Nobleza), y cargos LOCALES (Gobernador, Tesorero, General, Maestro de Obras, Sacerdote). Pertenece a 1 y SOLO 1 Facción. Una Facción puede tener muchos asentamientos.

## Facción (pieza central)
Entidad política soberana que agrupa jugadores y asentamientos. Una Facción puede tener muchos jugadores y muchos asentamientos. Tiene cargos de NIVEL FACCIÓN: Rey y Embajador. La ciudadanía de un jugador se extiende a TODOS los asentamientos de su propia Facción (no a la Liga completa).

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
Las 3 clases de población NPC de un asentamiento (distintas de los Jugadores).
- **Pesants:** base, crecimiento rápido, origen de tropas básicas.
- **Artesanos:** operan edificios de producción especializada, origen de tropas Tier 2.
- **Nobleza:** crecimiento lento, requiere cantidad mínima de ciudadanos (Jugadores) en el asentamiento, origen de tropas de élite.

## Aedas / Poetas
Mismo rol: los NPCs viajantes que dan acceso a tecnología (itinerantes o residentes si hay nobleza) Y narran el lore/histórico del servidor (títulos de prestigio, hazañas).

## Oro
Recurso/medio de intercambio: metal precioso en bruto pesado (no moneda acuñada), origen en minas ubicadas en el mapa. Se usa para servicios/NPCs (sueldos, mercenarios, tecnología) y también como recurso más de trueque entre Facciones.

## Nodo / Zona de Influencia
Al fundar un asentamiento se genera automáticamente su zona de influencia (también llamada nodo, usado como sinónimo del área territorial del asentamiento — NO como los "nodos" de grid fijo de proyectos previos; aquí la posición es libre en el mapa continuo).
