# Niebla de guerra: vista, memoria e intel

Qué ve un jugador de lo ajeno, qué recuerda de lo que dejó de ver, cómo se guarda lo explorado y qué se puede comprar para ver más. Es la regla que decide **qué sale del servidor** hacia un cliente de jugador: lo que no se proyecta no se puede leer en un DevTools,
así que la proyección es una frontera de seguridad, no una capa de presentación.

- **Reescrito el 2026-10-13** para el mundo reactivo (sin tick: lo visible y lo explorado se **muestrean**, [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §4) y la calibración de distancias. El «cómo funcionaba» por tick está en `typescript-final:Docs/Mechanics/Niebla_De_Guerra.md`.
- **Módulos:** `src/engine/exploracion.ts` (rejilla de lo explorado, `marcarVisto`, fusión, máscara proyectada), `src/engine/memoria.ts` (lo que cada Facción recuerda, `grabarLoVisto`), `src/engine/intel.ts` (Miradas e Informes),
  `src/engine/ubicacion.ts` (`grabarExploracionPersonal`, foto del interior), `src/engine/ejercitos.ts` (`alcanceDeVista`), `src/engine/pertenencia.ts` (`compartenVision`), `src/engine/alijos.ts`,
  `src/session/proyecciones/jugador.ts` (`proyectarParaJugador`: dónde se aplican todas las reglas), `src/session/comandos/intel.ts` (comprar), `src/session/comandos/ayudas.ts` (`conExploracionFundida`),
  `src/engine/trasladoDeFaccion.ts` (memoria al fusionar/anexar).
- **Canon:** Doc 5.12.7 (qué ve), 5.12.8 (qué recuerda), 5.12.10 (intel), 1.10 (el jugador en el mundo), 1.3 (exploración sin bandera). El documento de diseño que la precedió (`Consideraciones/Niebla_De_Guerra_Definicion.md`) se borró el 2026-10-11 al pasar todo aquí; sigue en el historial de git.
- **Fuera de alcance aquí:** la inspección de cerca (anillo de 8) → [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md); la proyección de lo que es propio (almacén, cola, etc.) salvo en lo que afecta a la niebla; el terreno, que **lo tapa el cliente** con la máscara (el servidor solo dice qué se ha explorado); el cliente de administración no tapa nada.

> **Implementado en Rust (2026-10-20, fase 4C).** Los **informes de inspección** son del héroe (`Nucleo::informes_de_inspeccion`): uno por objetivo, `INSPECCION.informeMinutos` = 10 minutos, renovables al volver a mirar (`militar::inspeccion`); falta proyectarlos al cliente. La visión muestreada ya da a cada columna su alcance según la tropa en pie: 30 con soldados, 15 sin ellos (§3). Las plazas siguen sin ser ojos; las avanzadas operativas sí lo son desde la fase 4F (ver abajo).

## 1. Idea

**Ver algo es conocerlo.** Hay un solo mecanismo —lo que alcanzan tus ojos queda grabado— y de él salen los tres estados que ve el jugador:

| Estado | Qué ve | Cómo se deduce |
|---|---|---|
| **Nunca visto** | Nada (terreno tapado) | la celda no está en lo explorado |
| **Visto antes** | La última foto, con su fecha | está en lo explorado y, si es una plaza, tiene ficha en la memoria; no está en lo visible ahora |
| **Viéndolo** | La ficha **en vivo** | está en lo visible ahora |

Lo que se ve ahora **no se guarda**: se deriva en cada proyección de dónde están los ojos. La memoria **solo crece**; no caduca (el dato viejo se delata solo por su fecha).
La memoria es **de la Facción**, no del ciudadano: lo que explora uno lo saben todos los suyos. Quien aún no tiene bandera lleva la suya (§5.3).

## 2. Modelo de datos

```
Exploracion = string     hexadecimal, 1 bit por celda de la rejilla; '' = nada explorado
Rejilla     = { columnas = ceil(ancho / tamanoCelda), filas = ceil(alto / tamanoCelda), tamanoCelda }     ← se DERIVA de los límites del mapa en cada uso, nunca se guarda

MemoriaFaccion = {
  exploracion:   Exploracion
  asentamientos: Record<asentamientoId, FichaConocida>      última ficha de cada plaza AJENA vista
  informes?:     Record<asentamientoId, InformePlaza>       último informe comprado en una taberna
}
FichaConocida  = { asentamientoId, nombre?, faccionId, posicion, nivel, conocidoEn: Instante, radioPotencial? }
InformePlaza   = { asentamientoId, faccionId, nombre?, nivel, conocidoEn, posicion?, edificios[{tipo, posicion, estado, nivelInterno?, ambito?}], recintos[{nivel, celdas, avance}], guarnicion: Escuadron[], heroesIds }
MiradaIntel    = { id, faccionId, origenId, centro, radio, compradaEn, expiraEn, libreEn }
Heroe.exploracionPersonal?: Exploracion                     lo explorado SIN bandera
Heroe.plazasRecordadas?:    Record<asentamientoId, InteriorRecordado>   foto del interior de una plaza al salir: { vistoEn, almacen, cola (edificios en_cola/en_construccion), guarnicion }
Estado: memoriaPorFaccion: Record<faccionId, MemoriaFaccion>   (una Facción ausente no ha visto nada)    ·    miradasIntel: MiradaIntel[]
```

### 2.1 La rejilla de lo explorado

- La celda `(columna, fila)` ocupa `[columna·t, (columna+1)·t) × [fila·t, (fila+1)·t)`. Su **índice** es `fila · columnas + columna` y su **bit** es el `indice mod 8` del byte `indice div 8`, **contando desde el bit menos significativo**.
- Cada byte son **dos caracteres hex en minúscula, con ceros a la izquierda** (`byte.toString(16).padStart(2, '0')`), el byte 0 el primero. Hay `ceil(columnas · filas / 8)` bytes (800 para 80×80).
- Una cadena **más corta** que la rejilla significa «el resto, sin explorar» (se rellena con ceros al leer); una **más larga** se trunca al leer. Un carácter no hexadecimal se lee como 0.
- `estaExplorado(e, rejilla, punto)`: `false` fuera del mapa o si la cadena no llega a ese byte; si no, el bit.

### 2.2 Operaciones

```
marcarVisto(exploracion, rejilla, centro, radio):
    marcar SIEMPRE la celda que contiene el ojo (aunque el radio no alcance a su centro)
    para cada celda del rectángulo [floor((c−r)/t), floor((c+r)/t)] recortado a la rejilla:
        centroCelda = (col·t + t/2, fila·t + t/2);  si distancia² al ojo > radio²  → saltar
        marcar
    devuelve la MISMA cadena (idéntica por referencia) si no hubo ningún bit nuevo
fundirExploraciones(a, b) = OR dígito hex a dígito hex (la corta se completa con '0' por la derecha); si una es '' devuelve la otra
```

Una celda cuenta como vista cuando lo está **su centro** (la niebla se retira un poco más despacio de lo que en rigor se ve, nunca destapa terreno que nadie miró). Que devuelva la misma cadena sin novedad hace que un tick tranquilo no reescriba el estado.

### 2.3 La máscara que viaja (`NieblaProyectada`)

`{ tamanoCelda, columnas, filas, celdas, visibles }`: **dos** máscaras sobre la misma rejilla, más su geometría (el cliente no debe ir a buscarla a otro sitio: bastaría una versión desfasada para pintar la niebla corrida).
`celdas` = lo explorado alguna vez (incluye lo que se ve con ojos propios); `visibles` = lo que se ve ahora, que **puede incluir celdas que no están en `celdas`** (la visión prestada por aliados y Miradas no se explora, §4.4).

> **Implementado en Rust (2026-10-22, fase 4F).** `GeneradorVision` suma como ojo a cada avanzada **operativa** (`Observador::fijo` con `vision::radios::AVANZADA` = 12, igual en las tres etapas, `distancia ≤ radio`, no estricto): una obra pendiente, unas ruinas o una avanzada destruida no ven. Por lo demás la visión es la de siempre (el único hecho que emite sigue siendo `vision.contacto`). Una columna aparcada a la puerta de un lugar cuyo héroe entró sigue siendo un ojo del muestreo (comportamiento anterior a la fase 4F, anotado en la tabla de paridad). Falta la memoria de las avanzadas ajenas «como una ciudad» y el resto de la proyección.

## 3. Los ojos

| Ojo | Alcance | Notas |
|---|---|---|
| Una plaza | `radioPotencial` (`VISION.margenAsentamiento` = 0: **la plaza ve solo su zona**) | contra el **disco**, no contra el polígono de la zona (el recorte por fronteras es político, no óptico) |
| Una **avanzada** (Doc 5.17) | círculo fijo de **12** alrededor de sí, igual en las tres etapas | es lo que ve la plaza fuera de su frontera; en ruinas no ve nada |
| Una columna **con soldados en pie** (`alcanceDeVista`) | `VISION.ejercito` | algún escuadrón de la columna con `cantidad > 0` |
| Una columna **sin soldados** (un jugador solo, o con tropa aniquilada) | `VISION.jugadorSolo` | un hombre solo no despliega batidores |
| Una Mirada comprada | `radio` de la Mirada, centrada en su punto | mientras `expiraEn > ahora` (§7) |
| Una caravana **grande** (para ser vista, no para ver) | `VISION.caravanaGrande.radio` | §6, «Caravanas» |

`seVeAhora(punto, plazas, avanzadas, columnas, tropa, miradas)` = `∃ plaza: distancia ≤ radioPotencial` **∨** `∃ avanzada: distancia ≤ 12` **∨** `∃ columna: distancia ≤ alcanceDeVista` **∨** `∃ mirada: distancia ≤ radio` (desigualdad **no estricta**).

## 4. Qué se graba y cuándo

### 4.1 En cada muestreo (`grabarLoVisto`)

Se ejecuta en cada **muestreo de visión** (cada 250 ms, en paralelo por clave de Facción, sobre la rejilla espacio-temporal; [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §4), con las posiciones que dan los planes en ese instante: se graba dónde está cada columna ahora. No hace falta el instante exacto en que algo entra en la vista; los hechos con efecto de juego (alcanzar una puerta, interceptar) son eventos exactos y no dependen de este muestreo.

```
para cada Facción con algún ojo propio (sus plazas + sus avanzadas + sus columnas con faccionId == la suya):
    exploracion = marcarVisto de cada ojo (plaza o columna)                       ← NO entran los ojos aliados ni las Miradas
    para cada plaza AJENA (faccionId distinto) dentro del alcance de ALGÚN ojo propio:
        asentamientos[a.id] = { id, nombre, faccionId, posicion, nivel, conocidoEn: instante, radioPotencial }   ← sobrescribe la anterior
    si no hubo novedad y la Facción ya tenía registro, se devuelve la misma memoria (un tick tranquilo no reescribe el estado)
```

**Se olvida lo de una plaza que ya no existe** (decidido el 2026-10-11): si la plaza de una ficha o de un informe ya no está entre los asentamientos del mundo **y** su sitio cae en el alcance de algún ojo propio, la ficha y el informe se borran de la memoria (el informe usa su `posicion`, o la de la ficha si es un informe anterior a ese campo). Mientras el sitio no se vuelve a ver, la foto vieja se queda. Una plaza que sigue en pie —aunque haya cambiado de dueño— nunca se olvida: su ficha se actualiza.

Una Facción **sin ojos** (sin plazas, avanzadas ni columnas) no graba nada. Los **pueblos extractores** y las **avanzadas** ajenas se recuerdan como una ciudad (no aportan visión propia el pueblo; la avanzada sí, 12). Las plazas **propias** no entran en `asentamientos` (viajan completas o como avistadas). Una columna **huérfana** (`faccionId == ''`) no graba en ninguna Facción.

### 4.2 Sin bandera (`grabarExploracionPersonal`, en el mismo muestreo)

Para cada héroe en una columna cuya `faccionId == ''`: `exploracionPersonal ← marcarVisto(exploracionPersonal, posición de la columna en ese instante, alcanceDeVista(columna))`. Un héroe con Facción **no** graba aquí (ya lo hace `grabarLoVisto`): grabaría dos veces el mismo hecho.

### 4.3 Fusiones

- `conExploracionFundida(estado, heroeId, faccionId)`: al **fundar** o **entrar** en una Facción, `memoria.exploracion ← fundir(memoria.exploracion, héroe.exploracionPersonal)` y se **borra** `exploracionPersonal`. Sin nada que fundir, no cambia nada.
- `trasladarFaccion(origen → destino)` (anexión y fusión de Facciones): `exploracion` = OR de las dos; `asentamientos` e `informes` = unión (gana la del destino en conflicto) **menos las plazas de cualquiera de las dos Facciones** (ya no son ajenas); la memoria del origen desaparece.

### 4.4 Lo que NO se graba

- **La visión prestada** (aliados, señor y vasallo; Miradas): solo cuenta para lo **visible ahora**, nunca para `celdas` ni para las fichas. Si la relación se rompe o la Mirada caduca, desaparece en la proyección siguiente y no queda «último conocido».
- Los ejércitos, las caravanas y los campamentos de bandidos: no tienen memoria (una foto vieja mentiría sobre dónde están ahora).

### 4.5 Foto del interior de una plaza

Al **salir al mundo** desde una plaza (`salirAlMundo`) se guarda en el héroe, **ya sin la tropa ni la carga que se lleva**, `plazasRecordadas[plazaId] = { vistoEn, almacen, cola = edificios en_cola/en_construccion, guarnicion }`. No se refresca mientras se está dentro (se ve lo vivo).
Lo `activo` no entra: es público y viaja vivo en la ficha.

## 5. La proyección de jugador (`proyectarParaJugador`)

### 5.1 Quién es «propio»

- `faccionId` = la Facción de la que el héroe es ciudadano (`null` si ninguna).
- `asentamientosPropios` = plazas de esa Facción.
- `ejercitosPropios` = columnas de **su Facción**, **o** en las que **va dentro** (`participantes`), **o** en las que **lleva una escuadra suya** (un huérfano conserva sus escuadras). Sin bandera, es lo que le permite ver su propia columna.
- `dentroDe` = la plaza propia donde está el héroe (`ubicacion.tipo == 'asentamiento'`; sin registro, `ubicacionDeducida`: la columna gana a la residencia).

### 5.2 Visión compartida y Miradas

`faccionesQueComparten` = Facciones con una relación `activa` de **alianza o vasallaje** con la propia (cualquier dirección) — `compartenVision`. Sus plazas y columnas se suman a los **ojos** («ojosAsent», «ojosEjercito») para lo visible ahora.
`miradasVistas` = Miradas activas (`expiraEn > ahora`) de la Facción y de las que comparten visión.

### 5.3 La máscara (`nieblaDe`)

```
visibles = marcarVisto de: plazas propias + aliadas (radio), avanzadas propias + aliadas (12), columnas propias + aliadas (alcanceDeVista), Miradas vistas
celdas   = exploradoDelJugador  ∪  marcarVisto de: plazas propias, avanzadas propias, columnas propias          ← los ojos aliados y las Miradas NO entran
exploradoDelJugador = fundir( memoria.exploracion de su Facción, héroe.exploracionPersonal )
```

Unir lo grabado con lo que se ve ahora **no es redundante**: entre un comando y el siguiente tick, lo recién visto (p. ej. una plaza recién fundada) todavía no está en la memoria; sin la unión, el cliente pintaría niebla encima de lo que el jugador acaba de hacer.

### 5.4 Regla de cada cosa del mundo

| Cosa | Se proyecta si… | Qué viaja | Memoria |
|---|---|---|---|
| **Plaza en la que estás** (propia) | siempre | la plaza **completa** (`asentamientos`) | – |
| **Resto de plazas** (propias o ajenas) | `seVeAhora(a.posicion)` y no es `dentroDe` → `asentamientosAvistados` | id, nombre, `faccionId`, posición, `nivel`, **zona real** (polígono recortado contra todos los vecinos), edificios `activo`; si es **propia**: `cargos`, `politicasActivas` y, con una columna tuya a la puerta, el `almacen`; `interiorRecordado` si el héroe tiene foto | las ajenas no vistas ahora salen de `memoria.asentamientos` como `asentamientosConocidos` (la ficha con su `conocidoEn`) |
| **Ejércitos ajenos** | `seVeAhora(posición)` ∧ no está en una batalla activa | id, `tipo`, `faccionId`, **tramo del plan dentro de tu vista y con horizonte corto (unos 30 s), renovado**, nº de participantes (rombos), `heroeIds`, `enFormacion`; `teSigue` solo si persigue a una columna **tuya** | ninguna |
| **Caravanas ajenas** | no es propia ni tiene destino propio, `estado != 'disponible'`, y `seVeAhora` **o** «llama la atención» | id, tramo de su plan visible (horizonte corto), `faccionId` (de su plaza de origen, si existe), `escoltada` (adjunta a un ejército o con escolta cedida), `recursos` (**nombres, sin cantidades**, ordenados) | ninguna |
| **Campamentos de bandidos** | `seVeAhora(posición)` | tal cual | ninguna (aparecen y desaparecen) |
| **Campamentos de mercenarios** | **explorado** su sitio (`celdas`) o es su casa/donde está dentro | tal cual | por exploración |
| **Caminos** (red comercial) | **explorado**: se proyecta cada arista con algún extremo en `celdas`, fusionadas en tramos (polilíneas de un mismo escalón) | `{id: 'tramo-i', escalon 0/1/2, puntos}` | por exploración |
| **Alijos** | el héroe puede buscarlos (de una Facción **sin asentamientos**, o sin Facción) y su columna está a `≤ VISION.jugadorSolo` (15) del alijo y no lo ha abierto | los alijos a la vista | – |
| **Aedas itinerantes** | `seVeAhora(posición)` | id, posición, `enAsentamientoId` | ninguna |
| **Batallas activas** | el héroe combate en ella **o** `seVeAhora(batalla.punto)` | `batallaVisible`: id, estado, contexto, punto, bandos (Facción, nº de héroes, capacidad), `ladoPropio` | – |
| **Héroes ajenos** | van en una columna propia o avistada, o están dentro de la plaza en que estás | solo su parte pública (`HeroePublico`: nombre, clase, nivel, herido, escuadras que lleva, equipo) | – |
| **Territorio bajo tus columnas** | siempre, por cada columna propia | `territorioPorEjercito[ejercitoId] = faccionId` de la zona (de **todas** las Facciones) que contiene la columna; nunca qué plaza | – |
| **Mercados** | de las plazas propias, todo; de una plaza ajena en cuya puerta estás, solo las órdenes `activa` | órdenes | – |
| **Informes de inspección** | los del héroe no caducados (`INSPECCION.informeMinutos`) | tal cual | – |
| **Informes de plaza** | los de la memoria de su Facción | tal cual (con `conocidoEn`) | memoria |

Reglas transversales:

- **Lo que se ve gana a lo que se recuerda**: una plaza aparece en `asentamientosAvistados` **o** en `asentamientosConocidos`, nunca en las dos (y las `conocidas` excluyen `dentroDe`).
- **Una plaza ajena nunca revela** almacén, escuadras, edificios en obra, colas, cargos ni políticas, ni en vivo ni recordada. `edificios` viaja **solo los `activo`**.
- **Caravana «grande»** (`esCaravanaGrande`: comercial con ≥ `VISION.caravanaGrande.carros` carros con animal) llama la atención si está `preparando` o `en_transito` y hay un ojo **propio, aliado o prestado** a `≤ VISION.caravanaGrande.radio` de ella: una plaza o columna propia, una plaza o columna de una Facción que comparte visión, o el centro de una Mirada (aunque no entre en la vista normal). Una que vuelve vacía o va enganchada a una columna no llama la atención por sí misma.
- **Lo propio viaja entero; lo ajeno, solo el tramo que se ve.** El plan completo de una columna ajena revelaría su destino (intención), así que se manda únicamente el tramo dentro de la vista y con horizonte corto, renovado. De lo propio viaja el plan completo una sola vez y un cambio de plan llega como plan nuevo que empieza donde está la columna.
- El jugador **ve siempre por donde ha andado él**: su `exploracionPersonal` se funde con la memoria de la Facción en la máscara.
- Un jugador sin Facción nunca tiene `memoria`: se usa `MEMORIA_VACIA`.

### 5.5 Eventos (audiencia)

`eventosVisiblesParaJugador`: a un jugador le llegan los eventos **sin plaza** (globales), los de **una plaza propia** (`asentamientoId` de su Facción) y los que **nombran a su héroe** (`heroesNombrados`). El historial completo no viaja en las lecturas de estado: se pide con un cursor (`desde` = última versión vista).

## 6. Intel: lo que se compra en la taberna

Dos productos, **exactos en el instante de la compra** (sin ruido ni datos falsos):

### 6.1 Mirada (`comprarMirada`)

```
validar:  centro dentro del mapa [0,ancho]×[0,alto];  cupo > 0
vigentes  = Miradas con libreEn > ahora;   propias = las de mi Facción
si (Miradas ACTIVAS —expiraEn > ahora— de MI Facción con el mismo origenId) ≥ cupo → error «ya tiene abiertas todas las Miradas que admite»
si alguna Mirada propia vigente tiene el centro a < INTEL.mirada.radio del nuevo → error «se ha mirado hace poco» (enfriamiento por zona)
expiraEn = ahora + duracionMinutos;   libreEn = expiraEn + cooldownMinutos;   radio = INTEL.mirada.radio
precio  = ceil( oroBase + oroPorUnidad × distancia mínima del centro a mis ojos propios )     ← ojos propios: la taberna donde compro, mis plazas y mis columnas
```

- Resultado: la lista de Miradas del servidor **sin las ya enfriadas** más la nueva. **No avisa a nadie** y **no deja memoria**: mientras `expiraEn > ahora` suma a «lo visible ahora» de la Facción y de sus aliados.
- Cupo: taberna de plaza → `cupoMiradas` del nivel interno (1/2/3); taberna de campamento → `INTEL.campamento.cupoMiradas` por Facción y campamento. La compra la hace el Rey, el Embajador o el Gobernador **presente** en la plaza (se paga del almacén de esa plaza) o cualquier héroe con Facción presente en el campamento (se paga con su oro del almacén personal del residente o del carro del visitante).

### 6.2 Informe de plaza (`comprarInformePlaza`)

```
validar:  no es de mi Facción;  la Facción CONOCE la plaza (tiene ficha en la memoria  ∨  su posición cae en una Mirada activa mía);
          no hay un informe de esa plaza con conocidoEn + informe.cooldownMinutos > ahora
precio  = informe.oroPorNivel × nivel de la plaza
informe = { edificios sin los en_cola, recintos {nivel, celdas, avance}, guarnición, heroesIds }, fechado ahora      ← nunca almacén, colas, cargos ni políticas
efecto:  memoria[mi Facción].informes[plaza] = informe;  evento asentamiento.informe_pedido a la plaza espiada, sin decir quién
```

## 7. Parámetros

**Los bloques son los valores vigentes** (reescritos el 2026-10-19, decisión del autor para el sistema 4). Respecto al motor TypeScript cambian, tras la calibración (Calibracion_Distancias_Definicion §4.4-§4.5 y Guerra_Logistica):

| Parámetro | Motor TypeScript | Vigente (en el bloque) |
|---|---|---|
| `VISION.ejercito` | 150 | **30** |
| `VISION.jugadorSolo` | 80 | **15** |
| `VISION.margenAsentamiento` | 60 | **0** (la plaza ve solo su zona) |
| `VISION.caravanaGrande.radio` | 300 | **60** |
| Visión de una avanzada | — | **12** |
| `EXPLORACION.tamanoCelda` | 25 | **5** (~27 KB por Facción sobre 2.667 × 2.000) |
| `INTEL.mirada.radio` | 150 | **30** |
| `INTEL.mirada.oroPorUnidad` | 0,1 | **0,5** (mismo precio por viaje) |

<!-- parametros:VISION -->
```json
{
  "ejercito": 30,
  "jugadorSolo": 15,
  "margenAsentamiento": 0,
  "caravanaGrande": {
    "carros": 3,
    "radio": 60
  }
}
```
<!-- /parametros -->

<!-- parametros:EXPLORACION -->
```json
{
  "tamanoCelda": 5
}
```
<!-- /parametros -->

<!-- parametros:INTEL -->
```json
{
  "mirada": {
    "radio": 30,
    "duracionMinutos": 120,
    "oroBase": 40,
    "oroPorUnidad": 0.5,
    "cooldownMinutos": 120
  },
  "informe": {
    "oroPorNivel": 60,
    "cooldownMinutos": 360
  },
  "campamento": {
    "cupoMiradas": 1
  }
}
```
<!-- /parametros -->

<!-- parametros:INSPECCION -->
```json
{
  "informeMinutos": 10
}
```
<!-- /parametros -->

## 8. Invariantes

| Invariante | Dónde se vigila |
|---|---|
| Una plaza rival fuera de su radio y de la vista de toda columna propia **no aparece** en la proyección, ni redactada | `proyecciones/__tests__/jugador.test.ts` |
| Lo proyectado de un rival **nunca** incluye almacén, escuadras, edificios en obra ni colas (se comprueba por `Object.keys`: un campo nuevo rompe el test) | ídem |
| Una plaza vista y además recordada aparece **una sola vez**, como vista | ídem |
| La foto congelada **no cambia** cuando cambia la plaza real; solo al volver a verla | `engine/__tests__/memoria.test.ts` |
| Una Facción sin registro proyecta listas vacías y exploración vacía, nunca `undefined` | `jugador.test.ts` |
| Lo que entra en alcance y vuelve a salir sigue proyectándose como recordado | `memoria.test.ts` |
| La exploración solo crece | `exploracion.test.ts`, `session/__tests__/memoriaNiebla.test.ts` |
| La zona de una plaza avistada es la silueta real del motor | `jugador.test.ts` |
| `territorioPorEjercito` acierta aunque la ciudad que manda en esa tierra no se vea | ídem |
| Un camino entero en tierra sin explorar no viaja; uno que cruza lo explorado viaja entero | ídem |
| Un campamento de bandidos en un rincón explorado pero no visible ahora no viaja | ídem |
| La visión compartida entra en `visibles` y en los `*Avistados`, **nunca** en `celdas` ni en las fichas; al romperse la relación desaparece | `jugador.test.ts` |

## 9. Casos límite y decisiones que parecen raras

- **Una celda se marca por su centro**, salvo la del propio ojo, que se marca siempre.
- **Una plaza destruida se olvida al volver a ver el sitio** (§4.1), no antes: hasta entonces su foto sigue en la memoria. Una plaza **anexada o fusionada** se limpia de la memoria de la Facción resultante (§4.3).
- **Una plaza conquistada conserva su ficha vieja** (Facción y nivel de antes) hasta que se vuelva a ver.
- **Lo explorado por la visión prestada no se graba**, pero un Informe sí se puede comprar de una plaza que solo se vio en una Mirada.
- **El mismo disco para ver y para grabar**: el alcance de una plaza se mide contra `radioPotencial`, no contra el polígono, y por tanto crece con el nivel (una plaza de nivel 5 ve 30, lo mismo que una columna; lo de fuera de su frontera lo ven sus avanzadas).
- **Una columna sin soldados en pie ve `jugadorSolo`**, aunque tenga escuadras con `cantidad` 0.
- **Un héroe huérfano** ve en vivo con su columna pero no graba en ninguna Facción: lo que ve queda en su `exploracionPersonal`.
- **El terreno lo tapa el cliente**, no el servidor: el servidor solo manda las máscaras.
- **El interior recordado** solo viaja con una plaza avistada, y las propias siempre lo están (son ojos de sí mismas).

## 10. Divergencias

Ninguna abierta.
