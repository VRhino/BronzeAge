# Textos de las misiones

> **Implementado el 2026-10-27** (decisión del autor: los textos los tiene el motor, no en la foto sino en un catálogo aparte que se envía en los datos de juego). Las reglas de las misiones están en [`Misiones.md`](Misiones.md).

## 1. Qué es

El catálogo de textos que el cliente enseña de cada misión: su **título**, una frase de **qué hacer**, una **explicación** más larga en texto libre (qué es eso, dónde se hace y qué cuenta; añadida el 2026-10-28, mecánica 59) y, para las barras de avance, el **nombre de cada contador**. Va en `GET /v1/datos`, en `documentos.Misiones_Textos.TEXTOS_MISIONES`.

- **No es regla.** No entra en la huella de las reglas (`ReglasEconomia::huella` solo lee `Misiones.md`) ni en la foto: cambiar un texto no rompe ninguna partida, solo cambia la versión de los datos de juego.
- **Está entero.** Cada misión del catálogo `MISIONES` tiene su título, su texto y su explicación, y cada contador que usa alguna condición tiene su nombre (lo vigila `nucleo::misiones::pruebas::cada_mision_y_cada_contador_tiene_su_texto`).
- **Sorpresa.** Que el catálogo viaje entero no quiere decir que el cliente lo enseñe: una misión no se muestra hasta que se activa (decisión del autor del 2026-10-27).

## 2. Catálogo

`misiones`: por id, `titulo`, `texto` y `explicacion`. `contadores`: por contador, el nombre que acompaña a la barra («3 / 5 trueques»).

<!-- parametros:TEXTOS_MISIONES -->
```json
{
  "misiones": {
    "H1": { "titulo": "Primeros pasos", "texto": "Sal al mundo desde tu plaza o tu campamento", "explicacion": "Tu héroe empieza dentro de su plaza o de su campamento. Sal al mapa: solo, con tu ejército o con una convocatoria que parte. Desde fuera se explora, se comercia y se combate." },
    "H2": { "titulo": "Tropa de préstamo", "texto": "Pide tropa prestada en un campamento de mercenarios", "explicacion": "Los campamentos de mercenarios prestan tropa a los héroes que pasan por ellos. Entra en uno y pide un préstamo: es la forma más rápida de tener soldados sin plaza propia." },
    "H3": { "titulo": "Hasta el final del camino", "texto": "Marcha y llega a tu destino", "explicacion": "Traza un camino en el mapa y deja que tu columna lo recorra hasta el final. Cuenta para todos los héroes que van en ella al llegar." },
    "H4": { "titulo": "Repostar", "texto": "Rellena los víveres de tu columna en una plaza, avanzada o campamento", "explicacion": "Una columna en marcha gasta víveres. Cuando estés en una plaza, una avanzada o un campamento, rellénalos antes de seguir. La ración gratuita de los campamentos no cuenta." },
    "H5": { "titulo": "Ciudadano", "texto": "Entra en una Facción", "explicacion": "Las Facciones agrupan a los héroes: comparten plazas, cargos y misiones. Pide entrar en una que ya exista o crea la tuya." },
    "H6": { "titulo": "Tu primera escuadra", "texto": "Recluta una escuadra nueva", "explicacion": "Recluta una escuadra nueva en una plaza o en un campamento. Reforzar una escuadra que ya tienes no cuenta." },
    "H7": { "titulo": "Un ejército de verdad", "texto": "Recluta 50 de infantería, 25 de tiro y 10 de caballería", "explicacion": "Cuenta los soldados que recluten tus escuadras desde que cumples «Tu primera escuadra». La infantería sale del Centro Urbano y del Barracón, el tiro de la Galería de tiro y la caballería de las Caballerizas." },
    "H8": { "titulo": "Juntos", "texto": "Únete a un ejército o a una convocatoria", "explicacion": "Los ejércitos se juntan para ser más fuertes. Únete a una convocatoria o a un ejército que ya esté en marcha, en el campo o desde la plaza." },
    "H9": { "titulo": "Bautismo de fuego", "texto": "Combate en una batalla", "explicacion": "Entra en combate, contra bandidos o contra otra Facción. Basta con que tus tropas combatan en una batalla, se gane o se pierda." },
    "H10": { "titulo": "Limpieza de caminos", "texto": "Destruye un campamento de bandidos", "explicacion": "Los campamentos de bandidos rondan los caminos y asaltan las caravanas. Ataca uno y destrúyelo. Cuenta para cada héroe que ataca y también para su Facción." },
    "H11": { "titulo": "Tesoro escondido", "texto": "Abre un alijo", "explicacion": "Por el mapa hay alijos escondidos con oro, madera, piedra o trigo. Llega hasta uno y ábrelo." },
    "H12": { "titulo": "Manos a la obra", "texto": "Entrega materiales a la obra de un pueblo o una avanzada", "explicacion": "Los pueblos y las avanzadas se levantan con los materiales que traen los héroes. Lleva madera, piedra u otro material a una de esas obras y entrégalo." },
    "H13": { "titulo": "Forjar al héroe", "texto": "Reparte tus puntos de atributo", "explicacion": "Al subir, tu héroe gana puntos de atributo. Repártelos para hacerlo más fuerte en lo que prefieras." },
    "A1": { "titulo": "La aldea", "texto": "Funda una aldea agrícola", "explicacion": "Una aldea agrícola es un pueblo en un punto fértil cerca de la plaza que manda trigo en carro. Fúndala desde la plaza: se cumple mientras la plaza tenga alguna activa." },
    "A2": { "titulo": "Abrir el mercado", "texto": "Ten un Mercado activo", "explicacion": "El Mercado permite comerciar con otras plazas y Facciones. Construye uno y mantenlo activo." },
    "A3": { "titulo": "Primer barracón", "texto": "Ten un Barracón activo", "explicacion": "El Barracón recluta infantería. Construye uno y mantenlo activo." },
    "A4": { "titulo": "Crecer", "texto": "Sube la plaza a nivel 2", "explicacion": "La plaza sube de nivel cuando cumple sus requisitos de población y de edificios. Para el nivel 2 hace falta, entre otras cosas, la Granja a nivel 2 y una aldea agrícola." },
    "A5": { "titulo": "El primer pueblo", "texto": "Completa un pueblo", "explicacion": "Los pueblos extraen recursos del territorio. Elige un nodo, empieza la obra y llévale materiales hasta terminarla." },
    "A6": { "titulo": "Oficios", "texto": "Ten algún artesano", "explicacion": "Los artesanos aparecen cuando la plaza tiene su primer taller (Curtiduría, Armería, Fundición o Carpintería). Se cumple en cuanto haya alguno." },
    "A7": { "titulo": "Murallas", "texto": "Completa un recinto de la muralla", "explicacion": "La muralla se levanta por recintos alrededor de la plaza. Termina uno entero." },
    "A8": { "titulo": "Avanzada", "texto": "Completa una avanzada", "explicacion": "Una avanzada es un puesto lejos de la plaza que sirve de base y de repostaje. Funda una y termina su obra." },
    "A9": { "titulo": "Ciudad", "texto": "Sube la plaza a nivel 3", "explicacion": "Sube la plaza a nivel 3 cumpliendo sus requisitos de población y de edificios." },
    "A10": { "titulo": "Nobleza", "texto": "Ten algún noble", "explicacion": "Los nobles llegan cuando la plaza tiene un Palacio activo y suficientes residentes. Se cumple en cuanto haya alguno." },
    "A11": { "titulo": "Resistir", "texto": "Resiste un asedio", "explicacion": "Cuando un enemigo asedie la plaza, defiéndela y gana el combate." },
    "F1": { "titulo": "Una capital", "texto": "Funda la primera plaza de tu Facción", "explicacion": "La primera plaza de la Facción es su capital mientras el Rey no designe otra con Palacio. Fúndala con una Caravana de Fundación (o conquista una) y la Facción ya tiene capital." },
    "F2": { "titulo": "Repartir cargos", "texto": "Nombra un Embajador o un cargo local", "explicacion": "El Rey reparte el poder: nombra un Embajador de la Facción o un cargo local en una de sus plazas." },
    "F3": { "titulo": "Expansión", "texto": "Funda 2 plazas", "explicacion": "Entre todos los miembros, funda dos plazas nuevas para la Facción. Cuentan las que se funden desde que se activa la misión." },
    "F4": { "titulo": "Un reino", "texto": "Funda 5 plazas", "explicacion": "Sigue fundando: la cuenta de «Expansión» continúa hasta llegar a cinco plazas fundadas." },
    "F5": { "titulo": "Progreso", "texto": "Adopta una tecnología", "explicacion": "La Facción progresa adoptando tecnologías cuando cumple sus requisitos. Adopta una." },
    "F6": { "titulo": "Gobernar", "texto": "Activa una política", "explicacion": "Los cargos activan políticas que cambian cómo funciona la plaza o la Facción. Activa una." },
    "F7": { "titulo": "Comercio", "texto": "Cumple un trueque con otra Facción", "explicacion": "Acuerda un trueque con una plaza de otra Facción y que las caravanas lo completen. Cuenta para las dos." },
    "F8": { "titulo": "Rutas comerciales", "texto": "Cumple 5 trueques", "explicacion": "Sigue comerciando hasta completar cinco trueques con otras Facciones." },
    "F9": { "titulo": "Aliados", "texto": "Acepta una alianza", "explicacion": "Las alianzas unen a dos Facciones contra los demás. Propón una o acepta una propuesta. Cuenta para las dos." },
    "F10": { "titulo": "Paz en la región", "texto": "Destruid 5 campamentos de bandidos", "explicacion": "Entre todos los miembros, destruid cinco campamentos de bandidos. Cada campamento destruido por un héroe de la Facción suma uno." },
    "F11": { "titulo": "Conquista", "texto": "Conquista una plaza", "explicacion": "Ataca y toma una plaza de otra Facción, en batalla de números o en Unity." },
    "F12": { "titulo": "Cantares", "texto": "Empieza una épica", "explicacion": "Las épicas son cantares que hacen famosa a la Facción. Empieza una." }
  },
  "contadores": {
    "salidas": "salidas al mundo",
    "prestamos": "préstamos",
    "llegadas": "llegadas",
    "repostajes": "repostajes",
    "ciudadania": "ciudadanía",
    "escuadrones": "escuadras nuevas",
    "infanteria": "infantería",
    "tiro": "tiro",
    "caballeria": "caballería",
    "uniones": "uniones",
    "batallas": "batallas",
    "bandidos": "campamentos de bandidos",
    "alijos": "alijos",
    "entregas": "entregas",
    "atributos": "atributos repartidos",
    "mercado": "Mercado",
    "barracon": "Barracón",
    "nivel": "nivel",
    "pueblos": "pueblos",
    "artesanos": "artesanos",
    "nobles": "nobles",
    "recintos": "recintos",
    "avanzadas": "avanzadas",
    "asedios": "asedios resistidos",
    "aldeas": "aldeas",
    "capital": "capital",
    "cargos": "cargos",
    "fundaciones": "plazas fundadas",
    "tecnologias": "tecnologías",
    "politicas": "políticas",
    "trueques": "trueques",
    "aliados": "alianzas",
    "conquistas": "conquistas",
    "epicas": "épicas"
  }
}
```
<!-- /parametros -->
