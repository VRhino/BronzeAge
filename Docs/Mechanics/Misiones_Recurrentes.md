# Misiones recurrentes y tablón

> **Implementado en Rust el 2026-10-28** (mecánica 55; diseño acordado con el autor el mismo día). Amplía las misiones únicas ([`Misiones.md`](Misiones.md)), que siguen igual: A1-A11 se quedan como misiones únicas de plaza y el tablón va **además** de ellas (decisión de implementación del 2026-10-28). Las cifras del catálogo inicial son un borrador a calibrar con los bots. Código: `crates/nucleo/src/recurrentes.rs` (catálogo, estado y foto) y `crates/nucleo/src/nucleo/recurrentes.rs` (renovar, aceptar, contar, cumplir, reclamar y elegir).

## 1. Qué es

Una misión **recurrente** se repite: dura un día o una semana y, al acabar su periodo, sale otra igual. Viven en el **tablón** de su dueño:

- **Tablón de plaza**: lo ven quienes están dentro de la plaza o la gobiernan; las aceptan sus **residentes**; elige qué sale su **Gobernador**.
- **Tablón de Facción**: lo ven sus ciudadanos; las aceptan sus **ciudadanos**; elige qué sale su **Rey**.

Cada tablón tiene **7 huecos de diarias y 7 de semanales** (decisión del autor del 2026-10-28).

| Campo | Valores |
|---|---|
| Ámbito | plaza (`asentamiento`) o Facción (`faccion`) |
| Duración | `dia` o `semana` |
| Tipo | `individual` (con `limiteSemanal`) o `grupal` |
| Estado | disponible, aceptada, cumplida, reclamada, caducada |
| Quién la acepta | cualquiera del ámbito o solo el titular de un `cargo` |
| Texto | `titulo`, `texto` (qué hacer) y `explicacion` (texto libre) |
| Recompensa | `xp` (experiencia de Facción), `participacion` (solo en plaza) y `oro` (solo individuales, al almacén personal) |

## 2. Reloj

Hay un solo reloj, el **de mundo** (`Reloj_Y_Eventos.md` §1: milisegundos desde `2026-01-01T00:00:00Z`, así que sus 00:00 son las 00:00 UTC del reloj de mundo; coincide con la hora UTC de pared solo si la partida arrancó alineada).

- **Cada día a las 00:00** se renuevan las **diarias**: acaban las del día y salen otras iguales (las **mismas** de esa semana), con el avance a 0.
- **Cada 7 días a las 00:00**, contando desde el primer día de la partida (el día de su primer instante), es la **renovación semanal**: cambian **cuáles** salen, diarias y semanales, según lo elegido (§4); las semanales se renuevan y el tope semanal de cada individual (§3) vuelve a 0.
- Un tablón que nace a mitad de periodo (una plaza recién fundada, una Facción nueva, una partida que ya estaba en marcha) saca sus misiones en ese instante, hasta el final del día o de la semana en curso.
- La renovación es un evento del mundo (`Carga::Tablon`, prioridad 10, una entidad para todo el mundo) que solo está en la cola mientras hay alguna plaza o Facción.

## 3. Individual y grupal

- **Individual**: un héroe del ámbito la **acepta** (`aceptarMision`) y queda suya; cuenta solo lo que hace ese héroe **desde que la acepta** y mientras siga en el ámbito (residente de la plaza, ciudadano de la Facción). Varios héroes aceptan la misma, cada uno la suya; un héroe, una vez por periodo.
  - **Tope semanal**: `limiteSemanal` aceptaciones de esa misión por semana en ese tablón, entre todos sus héroes (también para las diarias: se suman las de los 7 días).
  - La aceptada que no se cumple antes del fin de su periodo queda **caducada**.
- **Grupal**: no se acepta. Cuenta lo que hacen **todos** los héroes del ámbito durante su periodo; se apunta lo que aporta cada uno. Al cumplirse, la recompensa se reparte (§5).

## 4. El tablón

- **Catálogo** (§6): las recurrentes posibles de cada ámbito, en orden. Por defecto salen las 7 primeras diarias y las 7 primeras semanales de su ámbito.
- **Elegir** (`elegirTablon`): el Gobernador (en la plaza) o el Rey (en la Facción) elige hasta 7 diarias y hasta 7 semanales para la **próxima renovación semanal**, y puede cambiar su elección durante la semana. Lo que está en el tablón no cambia hasta entonces. La elección se queda para las semanas siguientes hasta que se cambie; una misión elegida que ya no esté en el catálogo no sale.
- **Cargo**: una misión con `cargo` solo la acepta su titular: en la plaza, `gobernador`, `tesorero`, `general`, `maestroObras` o `sacerdote`; en la Facción, `rey` o `embajador`.

## 5. Reclamar y recompensas

Con `reclamarRecompensa` (el de las únicas, `Misiones.md` §4), con el id de la recurrente y el dueño del tablón (`as-N` o `f-N`).

- **Individual**: la reclama el héroe que la cumplió. Recibe la `participacion` en la plaza (Participacion §2) y el `oro` en su almacén personal (`almacen_personal.lleno` si no cabe; `heroe.sin_residencia` sin residencia); la Facción dueña del tablón (la de la plaza, o la Facción) recibe la `xp`.
- **Grupal**: la reclama el Gobernador de la plaza (o, sin Gobernador, un residente) o el Rey de la Facción. La `xp` va a la Facción; la `participacion`, **a partes iguales** entre los héroes que aportaron algo, en la plaza (la división entera; el resto se pierde).
- **Rechazos**: `mision.desconocida`, `mision.no_cumplida`, `mision.ya_reclamada`, `no_autorizado`.
- Las cumplidas sin reclamar **se pueden seguir reclamando** después de su periodo: su misión se queda en el tablón como vencida hasta que se reclama todo lo cumplido.

## 6. Catálogo editable en caliente

El catálogo de las recurrentes (cuáles hay, sus condiciones, topes, recompensas y textos) es **dato**, no código: el inicial es el bloque `MISIONES_RECURRENTES` de abajo y el vigente vive en el estado (va en la foto). Lo cambia la orden de administración `fijar_catalogo_misiones` (comando 187, parámetro `catalogo`: el JSON entero en texto, con la forma del bloque), sin reiniciar: entra como una petición del registro con su instante, así que una partida se reproduce igual desde el diario.

- Un cambio no toca las misiones que ya están en el tablón (aceptadas o no): vale para las que salen desde la **siguiente renovación** (la diaria saca las diarias con la definición nueva; la semanal, todo).
- Se valida entero o no entra (`mision.catalogo_invalido`, con el motivo): ids únicos y distintos de los de las misiones únicas, condiciones con los contadores de §7 y umbral > 0, `limiteSemanal` > 0 en las individuales, `cargo` del ámbito y **al menos una misión en cada ámbito** (`asentamiento` y `faccion`; decisión del autor del 2026-10-28: un catálogo que deja vacío un tablón no entra, `{}` tampoco).
- El catálogo **vigente** se lee con `GET /v1/admin/misiones/catalogo` (solo administradores), en el mismo JSON que acepta la orden; `/v1/datos` trae solo el inicial.
- Las misiones únicas (H, A, F) siguen en `Misiones.md`, con la huella de las reglas.

## 7. Contadores

Son los del héroe de `Misiones.md` §3, sumados en origen: `salidas`, `prestamos`, `llegadas`, `repostajes`, `escuadrones`, `infanteria`, `tiro`, `caballeria`, `uniones`, `batallas`, `bandidos`, `alijos` y `entregas`; y dos más, del mismo sitio que la participación (Participacion §2): `defensas` (defiende una plaza, un pueblo, una aldea o una caravana, también del acoso de bandidos) y `victorias` (está en el bando ganador de una batalla). Lo que cuenta un héroe va al tablón de la plaza donde reside y al de su Facción.

## 8. Proyección

`Asentamiento.tablon` (interior y `ADMIN`) y `Faccion.tablon` (la propia y `ADMIN`; la forma pública no lo lleva), mensaje `Tablon`: las misiones del tablón con su periodo (`desde_ms`, `hasta_ms`), las vencidas por reclamar, las ids de lo que sale esta semana y de lo elegido, cuándo son las dos renovaciones y el catálogo de su ámbito para elegir. Cada misión lleva su definición entera (textos incluidos); una individual, las aceptadas de esta semana y cada aceptación con su héroe, su estado y su avance; una grupal, su estado, su avance y lo que aportó cada héroe. Detalle en `Docs/Coordinacion/05_Contrato_de_Sesion.md`.

### 8.1 Hechos

- `mision.aceptada { mision, dueno, heroe }`: al héroe y al dueño.
- `mision.cumplida { mision, dueno }` (el de las únicas), con el héroe en una individual.
- `mision.reclamada`, como en las únicas.
- `tablon.renovado { semanal }`: a cada plaza y Facción con tablón.

<!-- eventos -->
```json
["mision.aceptada", "tablon.renovado"]
```
<!-- /eventos -->

## 9. Foto

Desde **BAN50**, al final del estado: el catálogo vigente (su JSON), el origen de las semanas, la renovación pendiente y cada tablón (lo que sale, lo elegido, las aceptadas de la semana y cada misión con su definición, su periodo y su avance). Las fotos BAN40 a BAN49 se leen con el catálogo inicial y sacan los tablones al cargar.

## 10. Catálogo inicial

<!-- parametros:MISIONES_RECURRENTES -->
```json
{
  "asentamiento": [
    { "id": "PD1", "duracion": "dia", "tipo": "individual", "limiteSemanal": 20, "condiciones": { "batallas": 1 }, "xp": 2, "participacion": 10, "titulo": "Guardia del día", "texto": "Combate en una batalla", "explicacion": "Cualquier batalla cuenta: contra bandidos, contra otra Facción, con números o en Unity, se gane o se pierda." },
    { "id": "PD2", "duracion": "dia", "tipo": "individual", "limiteSemanal": 20, "condiciones": { "defensas": 1 }, "xp": 2, "participacion": 10, "titulo": "Escudo de la plaza", "texto": "Defiende una plaza, un pueblo o una caravana", "explicacion": "Cuenta cada vez que defiendes con tus tropas una plaza en un asedio, un pueblo o una aldea en un saqueo, o una caravana comercial, también de los bandidos." },
    { "id": "PD3", "duracion": "dia", "tipo": "individual", "limiteSemanal": 20, "condiciones": { "entregas": 1 }, "xp": 2, "participacion": 10, "titulo": "Manos a la obra", "texto": "Entrega materiales a una obra", "explicacion": "Lleva materiales a la obra de un pueblo o de una avanzada de cualquier plaza." },
    { "id": "PD4", "duracion": "dia", "tipo": "individual", "limiteSemanal": 20, "condiciones": { "llegadas": 3 }, "xp": 2, "participacion": 10, "titulo": "Patrulla", "texto": "Llega 3 veces al final de tu camino", "explicacion": "Traza caminos y recórrelos hasta el final con tu columna. Cuenta cada llegada." },
    { "id": "PD5", "duracion": "dia", "tipo": "grupal", "condiciones": { "batallas": 5 }, "xp": 5, "participacion": 20, "titulo": "Plaza en armas", "texto": "Entre todos, combatid en 5 batallas", "explicacion": "Cuenta cada batalla de cada residente de la plaza durante el día. La participación se reparte a partes iguales entre quienes combatieron." },
    { "id": "PD6", "duracion": "dia", "tipo": "grupal", "condiciones": { "repostajes": 5 }, "xp": 5, "participacion": 20, "titulo": "Despensa abierta", "texto": "Entre todos, repostad 5 veces", "explicacion": "Cuenta cada vez que un residente reposta su columna en una plaza, una avanzada o un campamento." },
    { "id": "PD7", "duracion": "dia", "tipo": "individual", "limiteSemanal": 7, "cargo": "maestroObras", "condiciones": { "entregas": 3 }, "xp": 3, "participacion": 10, "titulo": "Capataz", "texto": "Entrega materiales a 3 obras", "explicacion": "Solo para el Maestro de Obras de la plaza: lleva materiales a las obras de sus pueblos y avanzadas." },
    { "id": "PD8", "duracion": "dia", "tipo": "individual", "limiteSemanal": 20, "condiciones": { "bandidos": 1 }, "xp": 3, "participacion": 10, "titulo": "Caminos seguros", "texto": "Destruye un campamento de bandidos", "explicacion": "Ataca un campamento de bandidos y destrúyelo." },
    { "id": "PD9", "duracion": "dia", "tipo": "individual", "limiteSemanal": 20, "condiciones": { "escuadrones": 1 }, "xp": 2, "participacion": 10, "titulo": "Leva", "texto": "Recluta una escuadra nueva", "explicacion": "Recluta una escuadra nueva en una plaza o en un campamento." },
    { "id": "PD10", "duracion": "dia", "tipo": "individual", "limiteSemanal": 7, "cargo": "general", "condiciones": { "victorias": 1 }, "xp": 3, "participacion": 10, "titulo": "Orden del General", "texto": "Gana una batalla", "explicacion": "Solo para el General de la plaza: gana una batalla de cualquier tipo." },
    { "id": "PS1", "duracion": "semana", "tipo": "individual", "limiteSemanal": 10, "condiciones": { "victorias": 5 }, "xp": 10, "participacion": 10, "titulo": "Veterano", "texto": "Gana 5 batallas", "explicacion": "Cuenta cada batalla que gana tu bando durante la semana, de cualquier tipo." },
    { "id": "PS2", "duracion": "semana", "tipo": "individual", "limiteSemanal": 10, "condiciones": { "bandidos": 3 }, "xp": 10, "participacion": 10, "titulo": "Cazador de bandidos", "texto": "Destruye 3 campamentos de bandidos", "explicacion": "Ataca y destruye campamentos de bandidos durante la semana." },
    { "id": "PS3", "duracion": "semana", "tipo": "individual", "limiteSemanal": 10, "condiciones": { "defensas": 5 }, "xp": 10, "participacion": 10, "titulo": "Muralla viva", "texto": "Defiende 5 veces", "explicacion": "Defiende plazas, pueblos, aldeas o caravanas durante la semana." },
    { "id": "PS4", "duracion": "semana", "tipo": "individual", "limiteSemanal": 10, "condiciones": { "infanteria": 50 }, "xp": 8, "participacion": 10, "titulo": "Recluta de infantería", "texto": "Recluta 50 de infantería", "explicacion": "La infantería sale del Centro Urbano y del Barracón." },
    { "id": "PS5", "duracion": "semana", "tipo": "grupal", "condiciones": { "batallas": 20 }, "xp": 20, "participacion": 20, "titulo": "Semana de hierro", "texto": "Entre todos, combatid en 20 batallas", "explicacion": "Cuenta cada batalla de cada residente de la plaza durante la semana." },
    { "id": "PS6", "duracion": "semana", "tipo": "grupal", "condiciones": { "bandidos": 10 }, "xp": 20, "participacion": 20, "titulo": "Limpieza de la comarca", "texto": "Entre todos, destruid 10 campamentos de bandidos", "explicacion": "Cuenta cada campamento de bandidos que destruye un residente de la plaza durante la semana." },
    { "id": "PS7", "duracion": "semana", "tipo": "grupal", "condiciones": { "entregas": 20 }, "xp": 20, "participacion": 20, "titulo": "Gran obra", "texto": "Entre todos, entregad materiales 20 veces", "explicacion": "Cuenta cada entrega de materiales a la obra de un pueblo o de una avanzada de un residente de la plaza." },
    { "id": "PS8", "duracion": "semana", "tipo": "individual", "limiteSemanal": 10, "condiciones": { "tiro": 20 }, "xp": 8, "participacion": 10, "titulo": "Recluta de tiro", "texto": "Recluta 20 de tiro", "explicacion": "El tiro sale de la Galería de tiro." },
    { "id": "PS9", "duracion": "semana", "tipo": "individual", "limiteSemanal": 10, "condiciones": { "caballeria": 10 }, "xp": 8, "participacion": 10, "titulo": "Recluta de caballería", "texto": "Recluta 10 de caballería", "explicacion": "La caballería sale de las Caballerizas." },
    { "id": "PS10", "duracion": "semana", "tipo": "individual", "limiteSemanal": 10, "condiciones": { "alijos": 1 }, "xp": 8, "participacion": 10, "oro": 10, "titulo": "Tesoro escondido", "texto": "Abre un alijo", "explicacion": "Los alijos están repartidos por el mapa y solo los abre un héroe sin Facción o de una Facción sin plazas." }
  ],
  "faccion": [
    { "id": "FD1", "duracion": "dia", "tipo": "grupal", "condiciones": { "batallas": 3 }, "xp": 5, "titulo": "Por la Facción", "texto": "Entre todos, combatid en 3 batallas", "explicacion": "Cuenta cada batalla de cada ciudadano de la Facción durante el día." },
    { "id": "FD2", "duracion": "dia", "tipo": "grupal", "condiciones": { "bandidos": 2 }, "xp": 5, "titulo": "Batida", "texto": "Entre todos, destruid 2 campamentos de bandidos", "explicacion": "Cuenta cada campamento de bandidos que destruye un ciudadano de la Facción durante el día." },
    { "id": "FD3", "duracion": "dia", "tipo": "individual", "limiteSemanal": 30, "condiciones": { "uniones": 1 }, "xp": 2, "oro": 5, "titulo": "Hombro con hombro", "texto": "Únete a un ejército o a una convocatoria", "explicacion": "Únete a una convocatoria o a un ejército en campo o desde la plaza." },
    { "id": "FD4", "duracion": "dia", "tipo": "individual", "limiteSemanal": 7, "cargo": "embajador", "condiciones": { "llegadas": 1 }, "xp": 3, "titulo": "Embajada", "texto": "Llega al final de un camino", "explicacion": "Solo para el Embajador: viaja hasta tu destino." },
    { "id": "FS1", "duracion": "semana", "tipo": "grupal", "condiciones": { "victorias": 20 }, "xp": 20, "titulo": "Semana gloriosa", "texto": "Entre todos, ganad 20 batallas", "explicacion": "Cuenta cada batalla que gana un ciudadano de la Facción durante la semana." },
    { "id": "FS2", "duracion": "semana", "tipo": "grupal", "condiciones": { "defensas": 10 }, "xp": 20, "titulo": "Fronteras firmes", "texto": "Entre todos, defended 10 veces", "explicacion": "Cuenta cada defensa de una plaza, un pueblo, una aldea o una caravana de un ciudadano de la Facción." },
    { "id": "FS3", "duracion": "semana", "tipo": "grupal", "condiciones": { "escuadrones": 10 }, "xp": 15, "titulo": "Movilización", "texto": "Entre todos, reclutad 10 escuadras nuevas", "explicacion": "Cuenta cada escuadra nueva que recluta un ciudadano de la Facción durante la semana." },
    { "id": "FS4", "duracion": "semana", "tipo": "individual", "limiteSemanal": 5, "cargo": "rey", "condiciones": { "victorias": 3 }, "xp": 15, "titulo": "Rey guerrero", "texto": "Gana 3 batallas", "explicacion": "Solo para el Rey: gana 3 batallas durante la semana." }
  ]
}
```
<!-- /parametros -->
