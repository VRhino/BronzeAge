# Glosario de Entidades Políticas

Consultar este documento ante cualquier duda de terminología — el resto de documentos asume estas definiciones.

## Jugador
Persona real que juega. Pertenece a 1 y SOLO 1 Facción en todo momento, y reside en 1 y SOLO 1 asentamiento a la vez (Doc 2.5) — es lo que le permite tener como mucho UN escuadrón de cada tropa (ver abajo, y Doc 5.8).

Es además el dueño de sus escuadrones: **los escuadrones son del Jugador, no del asentamiento** (Doc 5.4). El asentamiento es donde están apostados, no quien los posee. Tiene un valor de **Liderazgo** (Doc 5.11) que limita cuántos puede sacar a campaña a la vez.

## Escuadrón / Tropa / Unidad
Terminología cerrada (a petición del usuario, 2026-09-02). Son **tres conceptos distintos** y conviene no mezclarlos — la jerarquía de entidades es **Jugador → Escuadrón → Unidad**, y "tropa" es el TIPO, no la instancia:

- **Escuadrón:** la entidad. El grupo de soldados que un Jugador recluta y comanda como un bloque. Un Jugador tiene muchos escuadrones, pero **no más de uno de cada tropa**. Persiste como identidad (nombre, veteranía) aunque se quede sin unidades (Doc 5.4). Es lo que se cuenta: "tres escuadrones".
- **Tropa:** el TIPO de escuadrón — "Milicia de lanceros", "Honderos", "Arqueros con arco compuesto"… El catálogo completo está en Doc 5.8. Determina el poder por unidad, el equipo que cuesta reclutarlo, cuántas unidades trae y su coste de Liderazgo (Doc 5.11). Un escuadrón **jamás cambia de tropa**: ganar veteranía lo hace más fuerte, nunca lo convierte en otro (Doc 5.8).
- **Unidad:** cada soldado individual dentro de un escuadrón. Las bajas son permanentes (permadeath, Doc 5.4).

> **"Tropa" se usa además en sentido colectivo** en todos los documentos ("las tropas consumen raciones", "mantenimiento de tropas", "reclutar tropas"), igual que en castellano corriente. Eso es deliberado y no choca: en singular y referido a un catálogo es el tipo; en plural y genérico es el colectivo. **La entidad contable es siempre el escuadrón** — y por eso lleva ese nombre, para que "tropa" quede libre para los otros dos usos.

## Guarnición
Los escuadrones apostados en un asentamiento, es decir, los que NO salieron a campaña. Son los únicos que lo defienden de un asedio (Doc 5.12). Un asentamiento cuyos jugadores se llevaron todo queda indefenso.

## Ejército
Uno o más escuadrones que han salido del asentamiento y se mueven por el mapa como **una sola entidad** (Doc 5.12). Puede ser de un solo Jugador o de varios juntos; en ambos casos es la misma cosa, y cada Jugador que lo compone sigue limitado por su propio Liderazgo.

Mientras está en campaña, un ejército **no come del almacén de su asentamiento**: lleva su propio carro de suministros (Doc 5.13) — uno por Jugador, de capacidad fija. Puede además llevar **caravanas adjuntas** que amplían su alcance, y que pueden ir cargadas de mercancía (escolta de caravanas, Doc 5.13.2-5.13.3). Si el ejército es derrotado, las caravanas adjuntas se pierden con él.

Una marcha se puede **cancelar**, lo que dispara la vuelta por el mismo camino (Doc 5.12.6).

## Huérfano
Jugador al que conquistaron su asentamiento mientras estaba de campaña (Doc 5.4). Conserva los escuadrones que llevaba encima, pero se queda sin residencia: sin sitio donde reabastecer, reclutar ni volver. Deja de serlo cuando entra en una Facción que tenga asentamiento — se sale por la vía política, no por la militar.

## Liderazgo
Valor del Jugador que limita **cuánto puede sacar a campaña a la vez** — no cuánto puede poseer. Cada tropa tiene un coste de Liderazgo proporcional a su poder; la suma de lo que un Jugador se lleva no puede exceder su Liderazgo (Doc 5.11). Lo que se queda, defiende (ver Guarnición).

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
- **Rey:** autoridad sobre vasallaje, políticas superiores hacia los asentamientos de la Facción, y todas las funciones del Embajador. Por defecto en Liga-por-vasallaje; electo por voto en Liga-por-alianza.
- **Embajador:** crea alianzas y declara guerras. Designado por el Rey.

## Cargos (nivel asentamiento, uno de cada por asentamiento)
- **Gobernador:** electo por ciudadanos, máxima autoridad del asentamiento, designa al resto de cargos locales.
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
