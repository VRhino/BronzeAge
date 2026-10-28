# Misiones

> **Implementado en Rust el 2026-10-27** (mecánica 49; diseño acordado con el autor el 2026-10-26 y catálogo aprobado el 2026-10-27). Las recompensas y los umbrales son provisionales: se calibran con los batches y los bots. Código: `crates/nucleo/src/misiones.rs` (catálogo y estado) y `crates/nucleo/src/nucleo/misiones.rs` (de dónde sale cada avance, reclamar y la proyección).

> **Revisión del 2026-10-28 (mecánica 59).** Cada misión tiene además una **explicación** en texto libre ([`Misiones_Textos.md`](Misiones_Textos.md)). F1 se cumple con la capital **efectiva** (la primera plaza de la Facción), no con la designada, que pide Palacio (decisión del autor; ver `capital` en §3). `FijarNivelAsentamiento` (administración) también mira el contador `nivel`. Pruebas: `crates/nucleo/tests/misiones_una_a_una.rs` cumple cada misión del catálogo haciendo lo que dice su texto, con el catálogo sin previas (35 de 36; **H13 no tiene camino hoy**: el héroe no gana puntos de atributo, apuntado en `Docs/Mecanicas a desarrollar.md`, 61), y una guarda exige una prueba por misión; `tests/propios.rs` (`las_misiones_de_la_faccion_y_sus_cumplidas_llegan_a_cada_ciudadano…`) comprueba que las misiones de la Facción, cumplidas incluidas, llegan en `FACCION` a cada ciudadano, sea o no el Rey, y a ninguna otra Facción.

## 1. Alcance

Las misiones guían al jugador al principio del juego para que descubra las mecánicas, con recompensa. Tienen tres dueños: el **héroe**, el **asentamiento** y la **Facción**. Valen igual para los bots (`12_NPC_Fuera_Del_Motor.md` §2), que las siguen como currículo.

## 2. Modelo

- **Catálogo** (`MISIONES`, §6): cada misión tiene un id, su dueño (`heroe`, `asentamiento` o `faccion`), sus **condiciones** (uno o varios contadores con su umbral; se cumplen todos a la vez), su recompensa y, si es de una cadena, la misión **previa**. Una misión con `"habilitada": false` existe en el catálogo pero no se activa nunca.
- **Activa**: habilitada, no cumplida y con su previa cumplida (o sin previa). Todo dueño empieza con las primeras de cada cadena activas, también en una partida que ya estaba en marcha.
- **Estado por dueño** (`Estado::misiones`): los contadores que necesita alguna misión **activa** (solo se cuenta mientras hay una activa que los use; un contador que ya no usa ninguna misión sin cumplir se borra), las cumplidas y las reclamadas. Un dueño sin entrada es un dueño nuevo: todo a 0 y nada cumplido.
- **Cumplir** es fijo: una misión cumplida no se pierde aunque el estado cambie después. Al cumplirse sale el hecho `mision.cumplida` y se activan las que la tenían de previa; si ya cumplen, se cumplen en el mismo instante.
- **Reclamar** (`reclamarRecompensa`): la recompensa no se da sola; se pide con la orden, así un almacén lleno no se la come y el cliente tiene su momento. Se reclama una vez.

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
| `bandidos` | héroe y Facción | destruye un campamento de bandidos: cada héroe atacante y su Facción |
| `alijos` | héroe | abre un alijo |
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

`reclamarRecompensa` (`heroeId`, `duenoId`, `misionId`), código 182. `duenoId` es el del dueño: `heroe-N`, `as-N` o `faccion-N`.

- **Quién**: la cuenta del héroe que nombra la orden; la misión del héroe, si es ese héroe; la de una plaza, si es su Gobernador o, si la plaza no tiene Gobernador, uno de sus residentes; la de una Facción, si es su Rey. La administración, siempre.
- **Rechazos**: `mision.desconocida` (el id no está en el catálogo, o no es de ese tipo de dueño), `mision.no_cumplida`, `mision.ya_reclamada`, `no_autorizado`; y, al dar la recompensa, `almacen_personal.lleno` (el oro no cabe entero en el almacén personal del héroe, o no tiene residencia: `heroe.sin_residencia`) y `almacen.lleno` (los recursos no caben enteros en el almacén de la plaza). Un rechazo no deja rastro: la misión sigue por reclamar.
- **Recompensa**: el oro, al almacén personal del héroe; los recursos, al almacén de la plaza; la experiencia, a la Facción (`faccion.experiencia` con el motivo de misión). Sale el hecho `mision.reclamada`.

## 5. Proyección

Cada dueño lleva sus misiones en su ámbito, en el campo `misiones` (`Mision`): `Heroe` en `HEROE` (el héroe propio), `Asentamiento` en `ASENTAMIENTO` (el interior, para quien está dentro o la gobierna) y `Faccion` en `FACCION` (la propia; la forma pública de una Facción ajena no las lleva). `ADMIN` las ve todas.

- Viajan, en el orden del catálogo, las **activas**, las **cumplidas** (por reclamar) y las **reclamadas** (`EstadoMision`). Las que esperan a su previa y las deshabilitadas no.
- Cada una lleva su `id`, su `previa`, sus condiciones (`contador`, `avance`, `umbral`; en una cumplida o reclamada el avance es el umbral) y su recompensa: `recompensa` (oro o recursos, `Cantidades` en milésimas) o `recompensa_experiencia` (milésimas, como `Faccion.experiencia`).
- El avance viaja con su dueño: cada contador que cambia toca al héroe, a la plaza o a la Facción.

Detalle en `Docs/Coordinacion/05_Contrato_de_Sesion.md`. Los títulos y textos de cada misión y el nombre de cada contador no viajan con el dueño: están en [`Misiones_Textos.md`](Misiones_Textos.md), que va en `GET /v1/datos` (fuera de la huella y de la foto).

### 5.1 Hechos

- `mision.cumplida { mision, dueno }`: al dueño (el héroe; quien oye la plaza; la Facción).
- `mision.reclamada { mision, dueno, oro | xp }`: al héroe que la reclama y al dueño.

<!-- eventos -->
```json
["mision.cumplida", "mision.reclamada"]
```
<!-- /eventos -->

## 6. Parámetros

<!-- parametros:MISIONES -->
```json
{
  "heroe": [
    { "id": "H1", "condiciones": { "salidas": 1 }, "oro": 10 },
    { "id": "H2", "previa": "H1", "condiciones": { "prestamos": 1 }, "oro": 10 },
    { "id": "H3", "previa": "H1", "condiciones": { "llegadas": 1 }, "oro": 10 },
    { "id": "H4", "previa": "H3", "condiciones": { "repostajes": 1 }, "oro": 10 },
    { "id": "H5", "previa": "H1", "condiciones": { "ciudadania": 1 }, "oro": 20 },
    { "id": "H6", "previa": "H5", "condiciones": { "escuadrones": 1 }, "oro": 20 },
    { "id": "H7", "previa": "H6", "condiciones": { "infanteria": 50, "tiro": 25, "caballeria": 10 }, "oro": 50 },
    { "id": "H8", "previa": "H5", "condiciones": { "uniones": 1 }, "oro": 20 },
    { "id": "H9", "previa": "H6", "condiciones": { "batallas": 1 }, "oro": 30 },
    { "id": "H10", "previa": "H9", "condiciones": { "bandidos": 1 }, "oro": 50 },
    { "id": "H11", "previa": "H3", "condiciones": { "alijos": 1 }, "oro": 20 },
    { "id": "H12", "previa": "H5", "condiciones": { "entregas": 1 }, "oro": 20 },
    { "id": "H13", "condiciones": { "atributos": 1 }, "oro": 10 }
  ],
  "asentamiento": [
    { "id": "A1", "condiciones": { "aldeas": 1 }, "recursos": { "trigo": 300 } },
    { "id": "A2", "condiciones": { "mercado": 1 }, "recursos": { "madera": 200 } },
    { "id": "A3", "condiciones": { "barracon": 1 }, "recursos": { "madera": 200, "piedra": 100 } },
    { "id": "A4", "previa": "A2", "condiciones": { "nivel": 2 }, "recursos": { "piedra": 300 } },
    { "id": "A5", "previa": "A4", "condiciones": { "pueblos": 1 }, "recursos": { "madera": 300 } },
    { "id": "A6", "previa": "A4", "condiciones": { "artesanos": 1 }, "recursos": { "trigo": 300 } },
    { "id": "A7", "previa": "A4", "condiciones": { "recintos": 1 }, "recursos": { "piedra": 400 } },
    { "id": "A8", "previa": "A5", "condiciones": { "avanzadas": 1 }, "recursos": { "madera": 400 } },
    { "id": "A9", "previa": "A6", "condiciones": { "nivel": 3 }, "recursos": { "piedra": 500 } },
    { "id": "A10", "previa": "A9", "condiciones": { "nobles": 1 }, "recursos": { "oro": 100 } },
    { "id": "A11", "condiciones": { "asedios": 1 }, "recursos": { "piedra": 300 } }
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

## 8. Casos límite y decisiones que parecen raras

- **Contadores que empiezan al activarse.** Lo que pasó antes de que una misión estuviera activa no cuenta (H7 cuenta los reclutados desde que se cumple H6), salvo los contadores de estado, que se leen al activarse: una plaza que ya tenía Mercado cumple A2 en cuanto se mira.
- **Partidas que ya estaban en marcha.** Al cargar una foto anterior a las misiones, todos los dueños empiezan con todo a 0; los contadores de estado se miran en ese momento, sin hecho.
- **Conquista.** La plaza conserva sus misiones al cambiar de Facción: son de la plaza, no de su dueña. Lo que se cobre después va al almacén de la plaza, sea de quien sea.
