# Misiones

> **Implementado en Rust el 2026-10-27** (mecánica 49; diseño acordado con el autor el 2026-10-26 y catálogo aprobado el 2026-10-27). Las recompensas y los umbrales son provisionales: se calibran con los batches y los bots. Código: `crates/nucleo/src/misiones.rs` (catálogo y estado) y `crates/nucleo/src/nucleo/misiones.rs` (de dónde sale cada avance, reclamar y la proyección).

> **Revisión del 2026-10-28 (mecánica 59).** Cada misión tiene además una **explicación** en texto libre ([`Misiones_Textos.md`](Misiones_Textos.md)). F1 se cumple con la capital **efectiva** (la primera plaza de la Facción), no con la designada, que pide Palacio (decisión del autor; ver `capital` en §3). `FijarNivelAsentamiento` (administración) también mira el contador `nivel`. Pruebas: `crates/nucleo/tests/misiones_una_a_una.rs` cumple cada misión del catálogo haciendo lo que dice su texto, con el catálogo sin previas (35 de 36; **H13 no tiene camino hoy**: el héroe no gana puntos de atributo, apuntado en `Docs/Mecanicas a desarrollar.md`, 61), y una guarda exige una prueba por misión. H7 pide **20** de tiro (antes 25: la única tropa de tiro sin artesanos, los honderos, trae 20; decisión del autor del 2026-10-28; cambia la huella de las reglas); `tests/propios.rs` (`las_misiones_de_la_faccion_y_sus_cumplidas_llegan_a_cada_ciudadano…`) comprueba que las misiones de la Facción, cumplidas incluidas, llegan en `FACCION` a cada ciudadano, sea o no el Rey, y a ninguna otra Facción.

> **Misiones recurrentes (mecánica 55, 2026-10-28).** Estas misiones son **únicas**: cada una se cumple una vez. Las que se repiten cada día o cada semana van aparte, en el tablón de cada plaza y cada Facción ([`Misiones_Recurrentes.md`](Misiones_Recurrentes.md)); comparten los contadores del héroe (§3), la orden `reclamarRecompensa` y las reglas de la recompensa (§4).

> **Recompensas flexibles (2026-11-01, en Rust; mecánica 66).** Cada misión da una **recompensa fija**, que marca su tipo (participación en una de plaza, experiencia de Facción en una de Facción, ninguna en una de héroe), y una **lista libre** de recursos, oro, experiencia y tropa (§4). Antes cada cadena daba una sola cosa: oro la de héroe, recursos la de plaza y experiencia la de Facción. Las cadenas de plaza A1-A11 dan ahora también 10 de participación. Cambia la huella de las reglas: una foto anterior no carga y hace falta partida nueva (§8). Código: `crates/nucleo/src/recompensas.rs` (leer y validar) y `crates/nucleo/src/nucleo/recompensas.rs` (el reparto al reclamar).

## 1. Alcance

Las misiones guían al jugador al principio del juego para que descubra las mecánicas, con recompensa. Tienen tres dueños: el **héroe**, el **asentamiento** y la **Facción**. Valen igual para los bots (`12_NPC_Fuera_Del_Motor.md` §2), que las siguen como currículo.

## 2. Modelo

- **Catálogo** (`MISIONES`, §6): cada misión tiene un id, su dueño (`heroe`, `asentamiento` o `faccion`), sus **condiciones** (uno o varios contadores con su umbral; se cumplen todos a la vez), su recompensa (una fija y una lista libre, §4) y, si es de una cadena, la misión **previa**. Una misión con `"habilitada": false` existe en el catálogo pero no se activa nunca.
- **Activa**: habilitada, no cumplida y con su previa cumplida (o sin previa). Todo dueño empieza con las primeras de cada cadena activas, también en una partida que ya estaba en marcha.
- **Estado por dueño** (`Estado::misiones`): los contadores que necesita alguna misión **activa** (solo se cuenta mientras hay una activa que los use; un contador que ya no usa ninguna misión sin cumplir se borra), las cumplidas y las reclamadas. Un dueño sin entrada es un dueño nuevo: todo a 0 y nada cumplido.
- **Cumplir** es fijo: una misión cumplida no se pierde aunque el estado cambie después. Al cumplirse sale el hecho `mision.cumplida` y se activan las que la tenían de previa; si ya cumplen, se cumplen en el mismo instante.
- **Reclamar** (`reclamarRecompensa`): la recompensa no se da sola; se pide con la orden, así un almacén lleno no se la come y el cliente tiene su momento. Se reclama una vez y se da entera o no se da (§4).

## 3. De dónde sale cada avance

Cada avance se suma **en origen**, en el sitio que produce el hecho y al ocurrir (como los contadores de los logros, `Tecnologia_Aedas.md` §2.1), nunca leyendo los hechos después ni barriendo: un hecho que deja dos rastros cuenta una vez. Los contadores de **estado** no se suman: se leen del estado (el máximo visto) cuando ocurre el hecho que puede cambiarlos y cuando se activa la misión que los usa.

| Contador | Dueño | Se suma o se lee |
|---|---|---|
| `salidas` | héroe | sale al mundo (`salirAlMundo`, `movilizarEjercito`, `salirDelCampamento`, partir una convocatoria) |
| `prestamos` | héroe | pide tropa prestada en un campamento (`mercenarios.prestamo`) |
| `llegadas` | héroe | su columna llega al final de su plan (`ejercito.llega`): cada participante |
| `repostajes` | héroe | reposta su columna en una plaza, una avanzada o un campamento (el Líder que la manda; la ración gratuita del campamento no cuenta) |
| `ciudadania` | héroe | entra como ciudadano en una Facción (`faccion.ciudadania_union`) |
| `escuadrones` | héroe | recluta una escuadra **nueva** (en una plaza o en un campamento) |
| `infanteria`, `tiro`, `caballeria` | héroe | soldados reclutados según el edificio de su tropa: Centro Urbano y Barracón; Galería de tiro; Caballerizas |
| `uniones` | héroe | se une a una convocatoria o a un ejército en campo o desde la plaza |
| `batallas` | héroe | combate en una batalla (con números o de Unity): cada héroe que combatió |
| `bandidos` | héroe y Facción | destruye un campamento de bandidos: cada héroe del ejército atacante (también el herido o el que va sin tropa; decisión del 2026-10-31) y su Facción |
| `alijos` | héroe | abre un alijo: cuenta para todos los héroes de la columna de quien lo abre (decisión del 2026-10-31) |
| `entregas` | héroe | entrega materiales a la obra de un pueblo o de una avanzada |
| `atributos` | héroe | reparte puntos de atributo |
| `mercado`, `barracon` | asentamiento | **estado**: 1 si tiene ese edificio activo |
| `nivel` | asentamiento | **estado**: el nivel de la plaza |
| `pueblos` | asentamiento | completa un pueblo |
| `artesanos`, `nobles` | asentamiento | **estado**: 1 si tiene alguno |
| `recintos` | asentamiento | completa un recinto de la muralla |
| `avanzadas` | asentamiento | completa una avanzada (su madre) |
| `asedios` | asentamiento | resiste un asedio en combate |
| `aldeas` | asentamiento | **estado**: las aldeas agrícolas activas |
| `capital` | Facción | **estado**: 1 si tiene capital **efectiva** (`Diplomacia_Facciones.md` §3.7: la designada o, sin ella, su plaza más antigua), es decir, en cuanto tiene una plaza; se mira al fundar y al conquistar (decisión del autor del 2026-10-28: antes pedía la capital designada, con Palacio, y F1 cerraba la cadena de Facción durante días) |
| `cargos` | Facción | asigna un Embajador o un cargo local en una de sus plazas |
| `fundaciones` | Facción | funda una plaza |
| `tecnologias` | Facción | adopta una tecnología |
| `politicas` | Facción | activa una política |
| `trueques` | Facción | cumple un trueque con otra Facción (las dos) |
| `aliados` | Facción | acepta una alianza (las dos) |
| `conquistas` | Facción | conquista una plaza (con números o en Unity) |
| `epicas` | Facción | empieza una épica |

## 4. Reclamar

`reclamarRecompensa` (`heroeId`, `duenoId`, `misionId`), código 182. `duenoId` es el del dueño: `heroe-N`, `as-N` o `f-N` (la Facción; `faccion-N` se rechaza con `orden.forma_invalida`).

- **Quién**: la cuenta del héroe que nombra la orden; la misión del héroe, si es ese héroe; la de una plaza, si es su Gobernador o, si la plaza no tiene Gobernador, uno de sus residentes; la de una Facción, si es su Rey. La administración, siempre.
- **Rechazos**: `mision.desconocida` (el id no está en el catálogo, o no es de ese tipo de dueño), `mision.no_cumplida`, `mision.ya_reclamada`, `no_autorizado`; y los de dar la recompensa, que no dejan rastro (más abajo): `heroe.sin_residencia`, `almacen_personal.lleno`, `mision.almacen_lleno` y `mision.sin_capital`. Sale el hecho `mision.reclamada` (§5.1).

### 4.1 La recompensa

Cada misión da una **recompensa fija**, que marca el tipo de su dueño, y una **lista libre**, `recompensas`. Es igual en estas misiones y en las recurrentes ([`Misiones_Recurrentes.md`](Misiones_Recurrentes.md) §5).

| Dueño | Recompensa fija | Clave del catálogo |
|---|---|---|
| Héroe | Ninguna: tiene que llevar algo en la lista | |
| Plaza | Participación, en puntos enteros (Participacion §2) | `participacion` |
| Facción | Experiencia de Facción | `xp` |

La fija es obligatoria y mayor que 0. La lista tiene elementos con su `tipo`; no repite ningún elemento ni la fija:

| `tipo` | Campos | Dónde vale |
|---|---|---|
| `recurso` | `recurso`, `cantidad` | En todas (no vale `"oro"`: para eso está `oro`) |
| `oro` | `cantidad` | En todas |
| `xp` | `cantidad` | En todas salvo las de Facción, donde es la fija |
| `tropa` | `tropa`, `cantidad` (soldados) | Solo en las misiones **personales**: las cadenas de héroe y las recurrentes individuales |

### 4.2 Adónde va

El destino lo decide quién cobra. Una misión **personal** (la cadena de héroe y la individual del tablón) paga al héroe; una **colectiva** (la cadena de plaza o de Facción y la grupal del tablón) paga a la plaza o a la Facción.

| Qué | Personal | Colectiva de plaza | Colectiva de Facción |
|---|---|---|---|
| Recursos y oro | Almacén personal del héroe | Almacén de la plaza | Almacén de la capital efectiva de la Facción ([`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) §3.7) |
| Tropa | Campamento donde reside el héroe | No se da | No se da |
| Experiencia, la de la lista o la fija | Facción del héroe (también en una individual del tablón de una plaza); en una individual del tablón de una Facción, esa Facción | Facción de la plaza | Esa Facción (es la fija) |
| Participación fija | Solo en la individual del tablón de una plaza: al héroe, en esa plaza | Cadena: al héroe que reclama, y a nadie si reclama la administración sin héroe. Grupal: a partes iguales entre los que aportaron, sin los mercenarios contratados (división entera: el resto se pierde) | No hay |

- **Tropa.** La escuadra nace en el **campamento** donde reside el héroe, nunca en su columna, y no mira el Liderazgo: se mira cuando el héroe sale con ella. Si el héroe ya tiene una escuadra de esa tropa (una por tropa), los soldados se suman a ella, esté donde esté, hasta el tope de la tropa; lo que sobra se pierde y no impide reclamar.
- **Experiencia.** Va a la Facción con el motivo de misión (`faccion.experiencia`). Un héroe sin Facción no suma experiencia y cobra todo lo demás.
- **Cantidades.** Los recursos y el oro van en unidades enteras, porque los almacenes solo guardan unidades enteras ([`Economia_Construccion.md`](Economia_Construccion.md), decisión C3b); la experiencia admite decimales (se guarda en milésimas); los soldados de una tropa son un entero.

### 4.3 Todo o nada

Reclamar comprueba **todo antes de dar nada**: si una parte no cabe o no tiene dónde ir, se rechaza la reclamación entera y la misión sigue por reclamar. Los rechazos de dar la recompensa:

- `heroe.sin_residencia`: la recompensa lleva recursos, oro o tropa para un héroe sin residencia (no tiene almacén personal ni campamento).
- `almacen_personal.lleno`: todo lo que va al almacén personal del héroe, recursos y oro **juntos**, no cabe en su capacidad.
- `mision.almacen_lleno`: algún recurso o el oro no cabe en el almacén de la plaza o de la capital.
- `mision.sin_capital`: la recompensa lleva recursos u oro para una Facción que no tiene plazas.

## 5. Proyección

Cada dueño lleva sus misiones en su ámbito, en el campo `misiones` (`Mision`): `Heroe` en `HEROE` (el héroe propio), `Asentamiento` en `ASENTAMIENTO` (el interior, para quien está dentro o la gobierna) y `Faccion` en `FACCION` (la propia; la forma pública de una Facción ajena no las lleva). `ADMIN` las ve todas.

- Viajan, en el orden del catálogo, las **activas**, las **cumplidas** (por reclamar) y las **reclamadas** (`EstadoMision`). Las que esperan a su previa y las deshabilitadas no.
- Cada una lleva su `id`, su `previa`, sus condiciones (`contador`, `avance`, `umbral`; en una cumplida o reclamada el avance es el umbral) y su recompensa: la fija, que es `participacion` (una de plaza) o `recompensa_experiencia` (una de Facción, en milésimas, como `Faccion.experiencia`), y la lista `recompensas` (§4.1), de elementos `RecompensaMision { tipo, clave, cantidad }`: `clave` es el recurso o la tropa (los ids de los datos) y va vacía en el oro y la experiencia; `cantidad` va en milésimas, o en soldados si es una tropa. Una misión de héroe no lleva fija.
- El avance viaja con su dueño: cada contador que cambia toca al héroe, a la plaza o a la Facción.

Detalle en `Docs/Coordinacion/05_Contrato_de_Sesion.md`. Los títulos y textos de cada misión y el nombre de cada contador no viajan con el dueño: están en [`Misiones_Textos.md`](Misiones_Textos.md), que va en `GET /v1/datos` (fuera de la huella y de la foto).

### 5.1 Hechos

- `mision.cumplida { mision, dueno }`: al dueño (el héroe; quien oye la plaza; la Facción).
- `mision.reclamada { mision, dueno, ... }`: al héroe que la reclama y al dueño. Lleva solo lo que se dio: `participacion` (la de cada uno de los que la cobraron), `xp` (toda la que sumó la Facción, en milésimas), `oro` (en milésimas), `recursos` (lista de `[recurso, milésimas]`) y `escuadras` (lista de `[tropa, soldados, id de la escuadra]`; los soldados son los que de verdad se dieron, sin los que se perdieron por pasar del tope de la tropa).

<!-- eventos -->
```json
["mision.cumplida", "mision.reclamada"]
```
<!-- /eventos -->

## 6. Parámetros

Cada misión del bloque lleva su `id`, su `previa` (si es de una cadena), sus `condiciones`, `habilitada` (por defecto `true`), su recompensa fija (`participacion` en `asentamiento`, `xp` en `faccion`, ninguna en `heroe`) y la lista `recompensas` con la forma de §4.1. Las cantidades van en unidades: enteras en los recursos y el oro; la experiencia admite decimales (`"cantidad": 2.5`). Un catálogo **se rechaza entero** (no carga) si:

- falta la recompensa fija de su tipo o no es mayor que 0, o una misión de héroe no lleva nada en `recompensas`;
- hay un `tipo` desconocido, una `cantidad` que no es mayor que 0 o un recurso u oro con decimales;
- la lista repite un elemento (el mismo recurso, el oro, la experiencia o la misma tropa) o la recompensa fija, o lleva `"recurso": "oro"`;
- una misión colectiva lleva `tropa`;
- un recurso o una tropa no existen, o una tropa pide más soldados de los que lleva su escuadra;
- lleva una clave de antes (`oro` o `recursos`) o la fija de otro tipo (`xp` en una de plaza, `participacion` en una de Facción): se rechaza en vez de perder la recompensa en silencio.

<!-- parametros:MISIONES -->
```json
{
  "heroe": [
    { "id": "H1", "condiciones": { "salidas": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 10 }] },
    { "id": "H2", "previa": "H1", "condiciones": { "prestamos": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 10 }] },
    { "id": "H3", "previa": "H1", "condiciones": { "llegadas": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 10 }] },
    { "id": "H4", "previa": "H3", "condiciones": { "repostajes": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 10 }] },
    { "id": "H5", "previa": "H1", "condiciones": { "ciudadania": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 20 }] },
    { "id": "H6", "previa": "H5", "condiciones": { "escuadrones": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 20 }] },
    { "id": "H7", "previa": "H6", "condiciones": { "infanteria": 50, "tiro": 20, "caballeria": 10 }, "recompensas": [{ "tipo": "oro", "cantidad": 50 }] },
    { "id": "H8", "previa": "H5", "condiciones": { "uniones": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 20 }] },
    { "id": "H9", "previa": "H6", "condiciones": { "batallas": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 30 }] },
    { "id": "H10", "previa": "H9", "condiciones": { "bandidos": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 50 }] },
    { "id": "H11", "previa": "H3", "condiciones": { "alijos": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 20 }] },
    { "id": "H12", "previa": "H5", "condiciones": { "entregas": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 20 }] },
    { "id": "H13", "condiciones": { "atributos": 1 }, "recompensas": [{ "tipo": "oro", "cantidad": 10 }] }
  ],
  "asentamiento": [
    { "id": "A1", "condiciones": { "aldeas": 1 }, "participacion": 10, "recompensas": [{ "tipo": "recurso", "recurso": "trigo", "cantidad": 300 }] },
    { "id": "A2", "condiciones": { "mercado": 1 }, "participacion": 10, "recompensas": [{ "tipo": "recurso", "recurso": "madera", "cantidad": 200 }] },
    { "id": "A3", "condiciones": { "barracon": 1 }, "participacion": 10, "recompensas": [{ "tipo": "recurso", "recurso": "madera", "cantidad": 200 }, { "tipo": "recurso", "recurso": "piedra", "cantidad": 100 }] },
    { "id": "A4", "previa": "A2", "condiciones": { "nivel": 2 }, "participacion": 10, "recompensas": [{ "tipo": "recurso", "recurso": "piedra", "cantidad": 300 }] },
    { "id": "A5", "previa": "A4", "condiciones": { "pueblos": 1 }, "participacion": 10, "recompensas": [{ "tipo": "recurso", "recurso": "madera", "cantidad": 300 }] },
    { "id": "A6", "previa": "A4", "condiciones": { "artesanos": 1 }, "participacion": 10, "recompensas": [{ "tipo": "recurso", "recurso": "trigo", "cantidad": 300 }] },
    { "id": "A7", "previa": "A4", "condiciones": { "recintos": 1 }, "participacion": 10, "recompensas": [{ "tipo": "recurso", "recurso": "piedra", "cantidad": 400 }] },
    { "id": "A8", "previa": "A5", "condiciones": { "avanzadas": 1 }, "participacion": 10, "recompensas": [{ "tipo": "recurso", "recurso": "madera", "cantidad": 400 }] },
    { "id": "A9", "previa": "A6", "condiciones": { "nivel": 3 }, "participacion": 10, "recompensas": [{ "tipo": "recurso", "recurso": "piedra", "cantidad": 500 }] },
    { "id": "A10", "previa": "A9", "condiciones": { "nobles": 1 }, "participacion": 10, "recompensas": [{ "tipo": "oro", "cantidad": 100 }] },
    { "id": "A11", "condiciones": { "asedios": 1 }, "participacion": 10, "recompensas": [{ "tipo": "recurso", "recurso": "piedra", "cantidad": 300 }] }
  ],
  "faccion": [
    { "id": "F1", "condiciones": { "capital": 1 }, "xp": 10 },
    { "id": "F2", "previa": "F1", "condiciones": { "cargos": 1 }, "xp": 10 },
    { "id": "F3", "previa": "F1", "condiciones": { "fundaciones": 2 }, "xp": 20 },
    { "id": "F4", "previa": "F3", "condiciones": { "fundaciones": 5 }, "xp": 50 },
    { "id": "F5", "previa": "F1", "condiciones": { "tecnologias": 1 }, "xp": 15 },
    { "id": "F6", "previa": "F2", "condiciones": { "politicas": 1 }, "xp": 15 },
    { "id": "F7", "condiciones": { "trueques": 1 }, "xp": 15 },
    { "id": "F8", "previa": "F7", "condiciones": { "trueques": 5 }, "xp": 40 },
    { "id": "F9", "previa": "F7", "condiciones": { "aliados": 1 }, "xp": 30 },
    { "id": "F10", "condiciones": { "bandidos": 5 }, "xp": 30 },
    { "id": "F11", "previa": "F3", "condiciones": { "conquistas": 1 }, "xp": 40 },
    { "id": "F12", "previa": "F5", "condiciones": { "epicas": 1 }, "xp": 20 }
  ]
}
```
<!-- /parametros -->

## 7. Invariantes

- Una misión se cumple una vez y se reclama una vez; reclamada implica cumplida.
- Solo hay contadores de misiones activas; un contador compartido por una cadena (F3 y F4, `fundaciones`) sigue contando de una a la siguiente.
- El estado de las misiones va en la foto (BAN46) y en el hash.
- Reclamar es todo o nada: un rechazo no deja ningún cambio, y una recompensa dada no se da otra vez.

## 8. Casos límite y decisiones que parecen raras

- **Contadores que empiezan al activarse.** Lo que pasó antes de que una misión estuviera activa no cuenta (H7 cuenta los reclutados desde que se cumple H6), salvo los contadores de estado, que se leen al activarse: una plaza que ya tenía Mercado cumple A2 en cuanto se mira.
- **Partidas que ya estaban en marcha.** Al cargar una foto anterior a las misiones, todos los dueños empiezan con todo a 0; los contadores de estado se miran en ese momento, sin hecho.
- **Conquista.** La plaza conserva sus misiones al cambiar de Facción: son de la plaza, no de su dueña. Lo que se cobre después va al almacén de la plaza, sea de quien sea, y la experiencia, a la Facción que sea su dueña al reclamar.
- **La experiencia de un héroe va a su Facción, no a la de la plaza.** En una misión personal, incluida una individual del tablón de una plaza, la cobra la Facción del héroe (decisión del 2026-11-01); antes la de una individual de plaza iba a la Facción de la plaza.
- **Una Facción sin plazas no cobra recursos ni oro.** No tiene capital efectiva donde dejarlos: `mision.sin_capital`. Como reclamar es todo o nada, la misión entera, experiencia incluida, sigue por reclamar hasta que la Facción tenga una plaza (§4.3).
- **La tropa no rechaza por estar completa.** Si el héroe ya tiene la escuadra al tope, los soldados se pierden y el resto de la recompensa se cobra (decisión del 2026-11-01).
- **La administración reclama sin héroe.** Una cadena de plaza reclamada por la administración sin héroe da todo lo demás pero la participación fija no se apunta a nadie.
- **Foto anterior.** Las recompensas flexibles cambian la huella de las reglas: una foto anterior no carga y hace falta partida nueva. La foto sigue en BAN54 (hoy BAN55, por el nombre de las caravanas).
