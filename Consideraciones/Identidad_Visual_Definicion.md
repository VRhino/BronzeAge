# Identidad visual (sigilo y estandarte) — decisiones y plan

**Reglas:** `Docs/Game/2_Sistema_Politico_Facciones_y_Cargos.md` §2.8.1 (canon). Este documento solo guarda las
decisiones con sus alternativas, el modelo propuesto y los pasos. Diseño cerrado con el usuario el 2026-10-05, y
**acotado ese mismo día tras pasarlo al consejo**: la cultura sale de este desarrollo. Origen: la entrada §27 de
`Docs/Mecanicas a desarrollar.md` (retirada al cerrarse, ver §6) y `Preguntas_Abiertas.md` §9. **Cerrado el 2026-10-05:
diseño e implementación hechos** (backend, cliente de administración y BronzeAgeClient).

## 1. Decisiones (2026-10-05)

| # | Decisión | Elegido | Alternativas descartadas |
|---|---|---|---|
| I1 | Qué cubre este desarrollo | **Solo la identidad de la Facción: sigilo y estandarte, y la insignia de los títulos.** La **cultura queda fuera**: sigue siendo del asentamiento y de sus edificios, **no afecta a los estandartes**, y su diseño vive en `Docs/Mecanicas a desarrollar.md` §43 | Mosaico estético por cultura aplicado también al sigilo (contradice D27 y mezcla dos ejes: quién eres y dónde estás); una sola estética común |
| I2 | Quién crea el sigilo y con qué | El Rey **compone con piezas** de un catálogo cerrado: campo, emblema y dos colores de una paleta cerrada. Dato = ids + versión de catálogo; **nunca imagen subida**, así no hay moderación | Solo catálogo de sigilos hechos (colisiones, se agota); libre con imagen o dibujo (moderación y almacenamiento) |
| I3 | Quién tiene sigilo | **La Facción lo guarda.** La Liga **se deriva** (no es entidad, Doc 0). Cada título de servidor lleva una **insignia fija** por id estable | Liga persistida con estandarte propio (rompe «la Liga no es entidad»); solo Facción (deja imperios y títulos sin identidad) |
| I4 | Cambio | **No se cambia nunca**: se elige al crear la Facción y queda para toda la partida (decidido por el usuario: evita la confusión de los Aedas y lógica innecesaria: sin comando, cooldown, histórico ni reasignación) | Cambio por el Rey con cooldown de 14 días (el consejo avisó del engaño en guerra y de la confusión en la crónica); libre sin cooldown |
| I5 | Unicidad | **Único por servidor**: se rechaza un sigilo idéntico al de otra Facción. **Solo el duplicado exacto**: los parecidos se permiten (decidido por el usuario, aunque el consejo pedía una regla de distinción) | Sin validación; regla de distinción perceptual |
| I6 | Imperios | **Marcos derivados, nada guardado**: el Gran Rey añade una corona o marco al estandarte de su Facción; la Liga por vasallaje usa el de la señora; por alianza, la orla de los sigilos miembros. Los Aedas narran los títulos con su insignia | Solo texto y color; estandarte de Liga elegido por el Rey (obliga a persistir la Liga) |
| I7 | Audio | **Fuera de este desarrollo y de este repo.** Lo resuelven los clientes; la música por cultura entra con la cultura (§43) | Tema musical por Facción; posponer también la parte visual |
| I8 | Cómo se avanza | **Apoyándose en los clientes para lo visual**: ellos dibujan y prueban el catálogo; este repo guarda ids, validación y contrato | Cerrar el catálogo desde el repo sin probarlo a tamaño de mapa |

Decididas de paso, derivadas de lo anterior: quien no tiene Facción (héroe suelto, Doc 1) lleva el sigilo **neutro**, y
los bandidos y los campamentos de mercenarios, los suyos, **reservados del servidor** que ninguna Facción puede elegir.
Las Facciones no desaparecen del estado, así que un sigilo no se libera. La fusión (Doc 2.6) hereda el de la Facción A.

## 2. Modelo (implementado el 2026-10-05, pasos 1-6)

```text
Sigilo
  formaId            silueta del escudo  (clasico, aspis, ocho, rombo, ovalo, torre)
  campoId            reparto del fondo con los dos colores  (18: heráldica sencilla y patrones de la época)
  emblemaId          icono central: animal, astro, arma…  (56)
  colorPrimarioId    paleta cerrada de 12
  colorSecundarioId  paleta cerrada (distinto del primario)
  colorEmblemaId     color del icono central
  orlaId             marco opcional  (ninguna, lisa, greca, cuerda, puntos, dentada)
  colorOrlaId        color del marco; no cuenta si no hay orla

Faccion.sigilo: Sigilo        obligatorio: se elige al crear y no cambia
Titulo.tituloId               id estable: faccionMasGrande, mayorPoderEconomico, mayorEjercito, granRey, mecenasAedas
```

- **Catálogo**: datos en `CATALOGO_SIGILO` (`constants.ts`, versión 4: 6 formas de escudo, 18 campos, 56 emblemas de época, 6 orlas, 12 colores con `hex` de referencia,
  más los tres sigilos reservados); se publica como `src/contratos/v1/catalogoSigilos.json` (generado, con test) y
  definiciones `Sigilo`/`CatalogoSigilos` en `contratos.schema.json` y `dto.ts`. Los ids solo se añaden, nunca se
  retiran; no hay versión por sigilo. Cosmético: el motor solo comprueba que los ids existen (como `Avatar`).
- **Validación** (`engine/sigilo.ts`, puro): del catálogo, dos colores distintos, no idéntico al de otra Facción;
  `sigiloLibre(semilla, ocupados)` da uno libre y estable para quien no elige (bots). Más de 57.000 combinaciones.
- **Comando** `crearFaccion` admite `sigilo` opcional (si falta, se asigna uno libre). Rechazos
  `faccion.sigilo_invalido` y `faccion.sigilo_duplicado`. `faccion.creada` lleva el sigilo en el payload. **No hay
  `cambiarSigilo`.**
- **Fusión** (`engine/fusion.ts`): la Facción nueva hereda el sigilo de A.
- **Proyección al jugador**: sin código nuevo (`Faccion` viaja entera). **Títulos**: `Titulo.tituloId` y los eventos
  `titulo.nace` / `titulo.cambia_manos` lo llevan en el payload junto a `tituloNombre`.
- **Liga y marcos**: se calculan en el cliente desde `facciones`, `relaciones` y `granReyFaccionId`. No hay campo.
- **Cliente de administración**: la pestaña Facción muestra el sigilo (vista previa con los dos colores y las piezas).
  La creación sigue siendo por nombre (recibe un sigilo libre).
- **BronzeAgeClient** (cliente de jugador, jurisdicción nuestra; rama `sigilo-faccion`, 0.9.0): copia del catálogo, dibujo
  SVG del escudo (`src/sigilo/sigilo.ts`), selector de piezas con vista previa al crear la Facción, sigilo en su ficha y
  color de territorio en el mapa = color principal del sigilo. Verificado contra el backend local.
- **Partidas anteriores**: eran de prueba y se descartan (decidido por el usuario); sin migración, `sigilo` es obligatorio.

## 3. Qué es de cada repo

| Pieza | Este repo | Clientes |
|---|---|---|
| Catálogo de ids (campos, emblemas, colores, insignias) | Define y publica | Dibujan cada id, lo prueban a tamaño de mapa y proponen piezas |
| Validación, unicidad, comando, evento | Hecho | — |
| Contrato `Faccion.sigilo`, `Titulo.tituloId` | Hecho | Consumen |
| Dibujo del estandarte, marcos de Gran Rey y Liga, insignias de título, editor de piezas | — | Sí |
| Cultura y audio | **Fuera** (§43 de `Docs/Ideas a diseñar.md`) | — |

## 4. Plan por pasos

0. **Prototipo visual** (hecho): `galeria-sigilos.html` de BronzeAgeClient muestra todos los emblemas a 96 y a 24 px.
1-6. **Hechos el 2026-10-05**: catálogo, dominio y validación, comando y evento, `tituloId`, contrato y cliente de
   administración (ver §2).
7. **Cliente de jugador** (BronzeAgeClient, jurisdicción nuestra; rama `sigilo-faccion`, 0.9.0): **hecho**. Selector de
   piezas al crear la Facción, sigilo en su ficha y en el color del mapa, **corona del Gran Rey** sobre su estandarte
   (marco derivado), **Liga** con el sigilo de la señora (vasallaje) o con los de los miembros (alianza) y las **insignias
   de los 5 títulos** (`src/sigilo/imperio.ts`, `insignias.json`, iconos de game-icons.net). Verificado contra el backend
   local: con una Facción real salen los 3 títulos que existen sin tener asentamientos.
8. **Conquest**: sugerencia dejada en `Conquest_prototype/Docs/Coordinacion/propuestas/CQ-009_sigilo_de_faccion_y_estandartes.md`
   (estandartes y banderas, insignias de título, marcos). Ya la tiene Codex; la implementa cuando quiera.

## 5. Revisión del consejo (2026-10-05) y qué se hizo con ella

Cinco asesores independientes coincidieron en lo siguiente:
- **Legibilidad a distancia, sin probar** → paso 0, con los clientes. Forma antes que color, por el daltonismo.
- **Unicidad exacta insuficiente** → el usuario decidió **solo duplicado exacto**; no hay regla de distinción.
- **Cambiar el sigilo antes de una ofensiva, reasignar sigilos liberados, histórico** → desaparecen al **no poder
  cambiarse nunca** el sigilo.
- **Versionado** → los ids solo se añaden.
- **Espacio del catálogo** → más de 57.000 combinaciones; `sigiloLibre` busca hasta encontrar uno.
- **Emblemas problemáticos** (símbolos de odio, marcas) → revisar el catálogo con los clientes en el paso 0.
- **Derivados (Liga, Gran Rey)** → reglas del cliente, documentadas en el canon; si divergen, un resolver en el contrato.
- **La contradicción cultura/estandarte** → resuelta por I1: la cultura sale.

## 6. Cierre y pendientes externos

La entrada `Docs/Mecanicas a desarrollar.md` §27 se **retiró el 2026-10-05**: el diseño y todo lo que es de este proyecto
(backend, cliente de administración y BronzeAgeClient) están hechos. Quedan dos cosas que no son nuestras:
- **Conquest (CQ-009)**: lo dibujan Codex y su equipo. Hasta entonces el sigilo no se ve en el 3D.
- **Siete emblemas sin icono** en game-icons.net, por encargar a un artista: escudo en ocho, Puerta de los Leones, águila
  bicéfala hitita, toro alado, árbol sagrado, casco de colmillos y sol de Vergina. Se añaden al catálogo (solo ids nuevos)
  cuando existan.
- Tampoco hay formulario de crear Facción en el cliente de administración (descartado por el usuario); sus Facciones
  reciben un sigilo libre.
- Insignias de títulos de jugador-poseedor, cuando exista `general con más victorias`.

## 7. Referencias históricas para los emblemas (2026-10-05)

Búsqueda de qué símbolos usaban de verdad el Egeo, Anatolia, Mesopotamia, el Levante, Egipto y Persia, para ajustar el
catálogo (los ids se añaden, no se retiran, así que lo que no encaje se deja de ofrecer en el selector sin borrarlo).
Los emblemas no son heráldica de la época (los blasones personales en el escudo son griegos, de finales del siglo V a. C.):
son los **símbolos sagrados y reales** de cada civilización, que es lo que de verdad aparece en sellos, relieves y monedas.

| Civilización (Era) | Símbolos documentados | Ya en el catálogo | Propuesta de añadir |
|---|---|---|---|
| **Micénica** (I-II) | escudo en ocho, casco de colmillos de jabalí (50-60 colmillos, de la élite), carro, grifo y caballo en hojas de espada, espiral, león (Puerta de los Leones) | `escudo_ocho`, `grifo`, `jabali`, `espiral`, `leon`, `carro`, `puerta`, `caballo` | `casco_colmillos` (el `casco` actual es genérico), `hoja_grabada` |
| **Minoica** (heredada en I) | doble hacha (solo acompaña a diosas), toro y cuernos de consagración, serpiente, pulpo, delfín, columna | `doble_hacha`, `toro`, `serpiente`, `pulpo`, `delfin`, `columna` | `cuernos_consagracion` |
| **Hitita / Anatolia** (I-II) | águila bicéfala (insignia real), disco solar con toro y ciervo, león, rayo del dios de la tormenta, «Mi Sol» y el disco solar alado | `leon`, `ciervo`, `toro`, `rayo`, `sol` | `aguila_bicefala`, `disco_alado` |
| **Mesopotámica / Asiria** (I-III) | estrella de ocho puntas (Ishtar), roseta (más que la estrella en época neoasiria), disco solar de Shamash, creciente de Sin, lamassu (toro o león alado), árbol sagrado, león | `luna`, `sol`, `leon`, `toro` | `estrella_ocho`, `roseta`, `toro_alado`, `arbol_sagrado` |
| **Fenicia / Levante** (II-V) | galera, murex (púrpura), cedro, palmera datilera, delfín (primeras monedas de Tiro), caballo y palma (Cartago); la palma en monedas es de época helenística | `nave`, `palma`, `delfin`, `caballo` | `murex`, `cedro` |
| **Egipcia** (I-III) | uraeus (cobra), halcón de Horus, escarabajo, anj, loto y papiro, ojo (udyat), buitre de Nekhbet, corona doble, disco solar alado | `halcon`, `serpiente`, `loto`, `ojo`, `corona`, `esfinge` | `escarabajo`, `anj`, `uraeus`, `papiro` |
| **Helénica** (IV-V) | casco corintio (el del hoplita, arcaico y clásico, no del Bronce), lechuza de Atenea, lambda de Esparta (no antes de 431 a. C.), maza de Heracles en Tebas, sol de Vergina en Macedonia, aspis redonda | `casco`, `sol`, `espada`, `lanza` | `casco_corintio`, `lechuza`, `lambda`, `maza`, `sol_vergina`, `aspis` |
| **Persa** (IV-V) | disco alado (faravahar, tomado de Asiria), grifo en capiteles, toro y león luchando, protomos de toro, lamassu en la Puerta de las Naciones | `grifo`, `leon`, `toro` | `disco_alado`, `leon_y_toro` |

**Cronología.** El casco hoplita (corintio) es de los siglos VIII-V a. C., así que pertenece a las Eras IV-V; el casco del Bronce
micénico es el de colmillos de jabalí. Conviene tener los dos, y que `casco_colmillos` y `casco_corintio` se dibujen
distintos del `casco` genérico actual.

**Lo del catálogo actual que menos encaja:** `lobo` (sin tradición fuerte en el Bronce egeo ni levantino; sí en Anatolia y Troya,
con Apolo Licio) y `corona` (genérica y moderna de aspecto). Pueden quedarse. `estrella` de cinco puntas es posterior
al Bronce: la de la época es de ocho puntas (Ishtar).

**Más opciones para el jugador (catálogo v4, 2026-10-05).** El usuario pidió que el jugador pueda armar su emblema con más libertad: además de campo, emblema y dos colores, ahora elige la **forma del escudo** (6), el **color del emblema**, **más campos** (18, con patrones de la época: bandas, zigzag, ondas, greca, rombos, radiante) y una **orla opcional** con su color (6). Quien no elige (los bots) recibe una combinación al azar de todo ello (más de 600 millones). La unicidad sigue siendo solo el duplicado exacto; el color de la orla no cuenta si no hay orla.

**Hecho (catálogo v3, 2026-10-05).** Primero se dibujaron a mano (v2) y el usuario no quedó conforme con los trazos; se
cambió a iconos de **game-icons.net** (CC BY 3.0, hay que citar a los autores en los créditos del juego), elegidos uno a uno
por parecido con esta tabla. El catálogo tiene **56 emblemas**, por cultura: micénica y minoica (toro, minotauro, león, grifo,
caballo, jabalí, cabra, serpiente, pulpo, delfín, espiral, doble hacha, cuernos de consagración, carro); hitita,
mesopotámica y persa (águila, ciervo, rayo, sol, estrella de ocho puntas, roseta, creciente, disco alado, escorpión,
carnero); fenicia (galera, murex, cedro, palma, ancla, ánfora); egipcia (halcón, uraeus, escarabajo, anj, ojo udyat, loto,
papiro, esfinge, corona doble, obelisco, pirámide); helénica (columna jónica, templo, trirreme, casco corintio, aspis,
lechuza, maza, lambda, olivo, corona de laurel, trigo, lanza, espada, arco, tridente). En BronzeAgeClient viven en
`src/sigilo/emblemas.json` con su autor, y los créditos en `src/sigilo/CREDITOS.md` y bajo el selector.

**Sin icono, por encargar a un artista** (no hay en game-icons.net, y por eso no están en el catálogo): escudo en ocho, Puerta
de los Leones, águila bicéfala hitita, toro alado (lamassu), árbol sagrado asirio, casco de colmillos de jabalí y sol de Vergina.
Se pudo retirar y renombrar ids porque no hay partidas reales; **desde que las haya, solo se añaden**. No hay desbloqueo por
Era: los emblemas helénicos tardíos (`lambda`, `lechuza`, `casco_corintio`) se ofrecen desde el principio.

**Fuentes:** [Mycenaean warfare, Ancient World Magazine](https://www.ancientworldmagazine.com/articles/mycenaean-warfare/),
[Figura de ocho, Teacher Curator](https://www.teachercurator.com/ancient-greek-art/the-figure-of-eight-shield/),
[Dendra panoply, Wikipedia](https://en.wikipedia.org/wiki/Dendra_panoply),
[Símbolos de Cnosos](https://knossos-palace.gr/symbols-of-knossos/),
[Labrys, Wikipedia](https://en.wikipedia.org/wiki/Labrys),
[Cuernos de consagración, Wikipedia](https://en.wikipedia.org/wiki/Horns_of_Consecration),
[Águila bicéfala hitita](https://arkeonews.net/the-legacy-of-the-double-headed-eagle-from-hittite-kings-to-modern-icons/),
[«Mi Sol» y el disco solar alado](https://www.ayk.gov.tr/wp-content/uploads/2015/01/MICHAUX-COLOMBOT-Dani%C3%A8le-THE-ROYAL-HITTITE-TITLE-%E2%80%98MY-SUN%E2%80%99-AND-THE-WINGED-SUN-DISK.pdf),
[Estrella de Ishtar, Wikipedia](https://en.wikipedia.org/wiki/Star_of_Ishtar),
[Lamassu, Wikipedia](https://en.wikipedia.org/wiki/Lamassu),
[Símbolos asirios](https://aryaassyria.com/blogs/blog/iconic-symbols-motifs-assyrian-art),
[Monedas fenicias, CoinWeek](https://coinweek.com/ancient-phoenician-coins/),
[Palma en monedas, American Numismatic Society](https://numismatics.org/pocketchange/palm-reading/),
[Uraeus, Wikipedia](https://en.wikipedia.org/wiki/Regalia_of_the_Pharaoh),
[Casco corintio, Ancient World Magazine](https://www.ancientworldmagazine.com/articles/corinthian-helmet/),
[Blasones griegos, Ancient World Magazine](https://www.ancientworldmagazine.com/articles/ancient-greek-shield-blazons/),
[Esparta y el lambda](https://scotthibberson.com/2025/07/01/the-lambda-and-the-line-shields-symbols-and-solidarity-in-sparta/),
[Imaginería persa, Fiveable](https://fiveable.me/art-prehistoric-to-middle-ages/unit-7/persian-art-persepolis-achaemenid-royal-imagery/study-guide/E6v4PASmYAxWRqCH).
