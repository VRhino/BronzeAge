# Presencia y convocatoria

## 1. Alcance y fuentes

Cubre **dónde está un héroe** (los cuatro sitios posibles y cómo pasa de uno a otro), los comandos de **presencia** —salir al mundo, entrar y salir de una plaza o de un campamento, guarnecer, marchar, conectarse y desconectarse, fijar la puerta y vetar— y la **convocatoria**: un ejército en preparación *dentro* de una plaza o un campamento, antes de salir.

- **Verificado contra:** `b3953c6` (2026-10-12).
- **Fuentes:** `engine/ubicacion.ts`, `engine/presencia.ts`, `engine/convocatoria.ts`, `session/comandos/presencia.ts`, `session/comandos/convocatoria.ts`, `session/comandos/desarmar.ts`, `session/comandos/autorizacion.ts` (quién puede cada uno), `session/proyecciones/jugador.ts` (`convocatoriasDelLugar`), y del motor de ejércitos (`engine/ejercitos.ts`): `entrarEnCampamento`, `salirDelCampamento`, `marcharA`, `exigirPuedeGuarnecer`, `caravanasAlEntrar`.
- **Canon:** [`Docs/Game/1_Sistema_de_Mundo_y_Territorio.md`](../Game/1_Sistema_de_Mundo_y_Territorio.md) 1.10 (el héroe está situado), [`Docs/Game/5_Sistema_Militar_y_Combate.md`](../Game/5_Sistema_Militar_y_Combate.md) 5.12.1 y 5.14.5.
- **No está aquí** (está en su documento):
  - Qué hace por dentro `salirAlMundo` / `salirDelCampamento` / `movilizarEjercito` (elegir tropa, Liderazgo, cargar el carro, ruta) → [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §3; terminar la campaña, `caravanasAlEntrar`, inspeccionar → §11–§12; juntar y separar columnas, formar un ejército en campo → §10.
  - La **desconexión que aplica el tick** (`salirDelMundo`: cuándo, aplazamiento, qué pasa con la columna) → [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §4.1. Aquí solo los comandos que la piden y la cancelan.
  - La regla de la **puerta** (grupos, capas, exilio) → [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md); aquí, los dos comandos que la fijan.
  - La **niebla** que se graba al salir y la foto del interior → [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md).
  - Dónde defiende un héroe que está dentro de una plaza cuando la atacan → [`Combate.md`](Combate.md) §3.2.

## 2. Dónde está un héroe (`Heroe.ubicacion`)

```
UbicacionHeroe = asentamiento { asentamientoId }      dentro de una plaza
               | columna      { ejercitoId }          en el mapa, en su columna (personal o ejército)
               | mercenarios  { campamentoId }        dentro de un campamento de mercenarios
               | desconectado { punto }               fuera del mundo (solo si salió con su columna, Tick §4.1)
```

Residir es otra cosa (ciudadanía, Doc 2.5); un héroe de campaña sigue residiendo en su plaza aunque esté en el camino. **Solo los comandos de esta sección escriben la ubicación** (más `fundar`, que deja a la columna dentro de la plaza recién levantada).

**Ubicación deducida** (`ubicacionDeducida`) cuando no hay registro (un héroe que nunca actuó, o una partida anterior a la mecánica): *la columna gana a la residencia* —si va en alguna columna, `columna`—; si no, `asentamiento` de la plaza donde reside; si no, `desconectado` en `(0, 0)` (un punto de relleno).
`estaEnAsentamiento(heroes, id, plaza, …)` = `(registrada ?? deducida)` es `asentamiento` de esa plaza.

### 2.1 El diagrama de transiciones

| De → a | Comando | Qué ocurre con la columna |
|---|---|---|
| (nada) → `mercenarios` | `crearHeroe` | No tiene columna: nace dentro, como residente. |
| `asentamiento` (su residencia) → `columna` | `salirAlMundo` (o `movilizarEjercito`) | Nace una columna con lo que elige. |
| `columna` → `asentamiento` (su residencia) | `entrarEnAsentamiento` | La columna **se deshace** dentro. |
| `columna` → `asentamiento` (otra) | `entrarEnAsentamiento` | La columna **queda aparcada** a la puerta. |
| `asentamiento` (otra) → `columna` | `salirDeAsentamiento` | Retoma la aparcada. |
| `columna` → `mercenarios` | `entrarEnCampamento` | En el propio, se deshace; en otro, queda a la puerta. |
| `mercenarios` → `columna` | `salirDelCampamento` | Residente: elige tropa y carga; visitante: retoma la aparcada. |
| `columna` (ejército) → `asentamiento` | `guarnecer` | El ejército se desarma en la puerta y cada héroe entra. |
| `columna` → `asentamiento` | `fundar` | La columna se deshace en la plaza que funda. |
| cualquiera → `desconectado` / de vuelta | `desconectarse` + tick / `conectarse` | Ver §7. |

No hay una salida de la residencia que no sea `salirAlMundo` (o `movilizarEjercito`, o `salirDelCampamento`): ahí se tiene el roster entero y el almacén, y hay algo que elegir.

## 3. Salir de la residencia (`salirAlMundo`)

Comando `{ asentamientoId, heroeId, escuadronIds, carga }`. Elegir tropa, validar Liderazgo y cargar el carro es la función de `Movimiento_Ejercitos.md` §3.2. Lo que añade el comando:

1. Crea la columna con id `ejercito-{ids.siguiente()}` y la añade; la plaza queda con la tropa fuera y la carga descontada.
2. **Llena los víveres** del héroe (`conViveresLlenos`), **después** de elegir la carga y con la tropa ya fuera (Movimiento §6.1).
3. **Toma la foto del interior** que deja atrás (`conFotoTomadaPor`): del asentamiento **ya sin** la tropa ni la carga que se lleva, guarda `{ vistoEn, almacen, cola (edificios no activos), guarnición }`. Lo `activo` no entra: es público y viaja vivo. Es lo que verá de esa plaza al volver a mirar de lejos.
4. `ubicacion = columna`. Evento `jugador.sale_al_mundo { ejercitoId, asentamientoId, heroeId, escuadronIds, liderazgoUsado, cargaTotal }` a la plaza.

Está envuelto en `sinConvocatoria` (§9): quien está en un ejército en preparación no sale por su cuenta.

## 4. Entrar y salir de una plaza

### 4.1 `entrarEnAsentamiento { asentamientoId, heroeId }` (`cruzarLaPuerta`)

Exige tener columna (`jugador.sin_columna`). Cuatro condiciones, en este orden (`MovilizacionInvalidaError`):

1. **La columna es personal**: «Vas en un ejército: hay que separarse antes de entrar en una plaza». (Un ejército entra por `guarnecer`, §4.3.)
2. **En la puerta**: `distancia(columna, plaza) ≤ MOVIMIENTO.radioPuerta` (10). Entrar es un acto, no un roce.
3. **Con permiso**: `puedeEntrarEn(plaza, héroe, columna.faccionId, relaciones)`: un residente entra siempre; luego manda el veto; luego el grupo (la propia Facción nunca se bloquea). Detalle en [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) (`PUERTA.cerradaAPorDefecto`).
4. **Residencia o no**:
   - **Su residencia**: la columna **se deshace** (`absorberColumna(…, devolverSuministro = true)`): las escuadras vuelven al campamento (`contenedor: campamento`) y el carro entero al almacén de la plaza (hasta donde quepa); la columna desaparece.
   - **Cualquier otra**: la columna **se queda aparcada** a la puerta, intacta.

Después `ubicacion = asentamiento`. Evento `jugador.entra_en_asentamiento`. De la segunda rama salen gratis dos casos: si conquistan la plaza mientras estás dentro, tu columna está fuera y la retomas; y si te la destruyen aparcada, sales a pie.

### 4.2 `salirDeAsentamiento { asentamientoId, heroeId }`

Retoma la columna aparcada de una plaza **ajena**, sin pantalla de equipamiento. Rechaza **a propósito** desde la propia residencia (manda a `salirAlMundo`). Exige tener columna. Pone `ubicacion = columna` y emite `jugador.sale_de_asentamiento`. No comprueba la geometría ni que el héroe estuviera dentro de *esa* plaza: lo único que mira de la plaza es si reside en ella. Envuelto en `sinConvocatoria`.

### 4.3 `guarnecer { asentamientoId, heroeId }` (un ejército vuelve a una plaza propia)

1. `exigirPuedeGuarnecer`: es un **ejército** (`tipo = ejercito`), de la **Facción de la plaza**, a `≤ radioPuerta`; si no, error.
2. `caravanasAlEntrar`: solo entra si **todas** las caravanas adjuntas son de esa plaza; pasan a `aparcada` (Movimiento §11).
3. `desarmarAlEntrar`: **solo su Líder** hace entrar al ejército; se deshacen las formaciones: cada héroe queda en una columna personal (el Líder con el resto del carro, los demás con su parte a prorrata).
4. Cada héroe de la columna **entra como lo haría solo** (§4.1): el residente deja su tropa y su carro; el que no reside entra de visita con su columna a la puerta.
5. Eventos: uno `jugador.entra_en_asentamiento` por héroe y `ejercito.guarnece { asentamientoId, ejercitoId, escuadrones, caravanasAparcadas }`.

## 5. Entrar y salir de un campamento de mercenarios

### 5.1 `entrarEnCampamento { campamentoId, heroeId }`

Exige columna. Si la columna es **personal con un solo participante**, entra él solo (`entrarEnCampamento` del motor); si no —un ejército, o una personal con compañía— entra el ejército entero, ordenado por su Líder:

- **Solo (motor)**: error si no es el Líder de una columna de un participante; error si no está a `≤ radioPuerta` (10) del campamento. `ubicacion = mercenarios`.
  - **No residente** (visitante): su columna queda a la puerta, intacta, sin tocar nada.
  - **Residente**: vuelca el carro al almacén personal **hasta `ALMACEN_PERSONAL.capacidad`**, recurso a recurso en el orden del carro; lo que no cabe **se queda en el carro** (la columna sigue aparcada a la puerta, sin escuadras y solo con ese resto); las escuadras vuelven al campamento (`alCampamento`).
- **Ejército**: debe estar a la puerta; `desarmarAlEntrar` (como `guarnecer`: solo el Líder, caravanas solo de este campamento, cada héroe en su columna personal) y cada héroe entra como lo haría solo. Evento `ejercito.entra_en_campamento`.
Evento por héroe: `jugador.entra_en_campamento`.

### 5.2 `salirDelCampamento`

Ver [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §3.3 (el residente elige tropa y carga del almacén personal y recibe la ración gratis; el visitante retoma su columna). El comando ajusta la ubicación, el almacén personal, `racionEn` y los víveres del héroe y emite `jugador.sale_de_campamento`. Envuelto en `sinConvocatoria`.

## 6. Marchar (`marcharA`)

Fija o rectifica el destino de la columna **del que va en ella**, tantas veces como quiera:

- Un **ejército** lo dirige solo su Líder.
- Una formación (ejército aún por formar) no se mueve.
- El héroe debe tener `ubicacion = columna` (dentro de una plaza hay que salir antes).
- Destino `ObjetivoEjercito` (un asentamiento o un punto) → `calcularRuta` con los asentamientos como pasos de río; sin ruta por tierra, error. El agua no se cruza.
- Efecto: `estado = marchando`, `objetivo`, `ruta`, `progreso = 0`, y se **abandona la persecución** (marchar a un punto y perseguir a la vez no existe).
- `rectifica = (la columna ya estaba marchando)`. Evento `jugador.marcha_fijada { ejercitoId, heroeId, objetivo, rectifica }`.

## 7. Conectarse y desconectarse

- **`desconectarse { heroeId }`**: si ya está `fuera` o ya tiene `desconectaEn`, no hace nada. Si no: `desconectaEn = ahora + PRESENCIA.retardoDesconexionMs` (150 000 ms = 2 min 30 s). Evento `jugador.se_desconecta`. El héroe sigue en el mundo esos dos minutos y medio, moviéndose como iba.
- **`conectarse { heroeId }`**:
  - Si **no estaba `fuera`**: si tenía `desconectaEn`, lo **cancela** (sin eventos); si no, nada.
  - Si estaba `fuera`: `volverAlMundo`. Dentro de una plaza o un campamento solo recupera el control. Si salió con su columna, **reaparece en su punto** con una columna personal nueva (`ejercito-{ids.siguiente()}`): `tipo personal`, `estacionado`, `politicaDeUnion rechazar`, `liderId` él, `suministro` = su carro de `fuera`, `escuadronIds` = sus escuadras `contenedor: fuera` (que vuelven a `ejercito`), `faccionId` la de su ciudadanía (o `''`), `origenAsentamientoId` su residencia (o `''`). Evento `jugador.vuelve_al_mundo` atribuido a su residencia.
- **En plena batalla de Unity** los dos comandos **no llevan el candado** de batalla: `desconectarse` se anota (`desconectaEn`) pero el tick no lo aplica mientras el héroe combate —sale en el primer tick tras acabar—, y `conectarse` la anula ([`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §5.1).
- **Fuera del mundo no se da ninguna orden**: la autorización rechaza todo menos `conectarse` con el motivo `fuera_del_mundo`.
- Quién lo aplica y cuándo: el tick ([`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §4.1). Lo pide el cliente al cerrarse, o el runner de bots al acabar su sesión.

## 8. Fijar la puerta y vetar

- **`fijarPuerta { asentamientoId, heroeId, cerradaA[] }`**: quien puede es el **Gobernador residente** de la plaza o el **Rey** de su Facción (no hace falta que esté presente); es **un único estado** (el último manda). Guarda `puertaCerradaA = grupos únicos`. No expira. Evento `asentamiento.puerta_fijada`.
- **`vetarJugador { asentamientoId, heroeId, vetadoId, vetar }`**: solo el **Gobernador residente**. Añade o quita a `vetadosIds` (ordenado). **A un residente no se le veta** (`PuertaInvalidaError`): eso es el exilio. Evento `asentamiento.veto`.
- La regla que decide quién entra (residente > veto > grupo > propia Facción) está en [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md).

## 9. La convocatoria (ejército en preparación)

Un héroe **dentro** de una plaza o un campamento convoca un ejército; otros de su Facción **que están dentro del mismo lugar** se le unen, cada uno con la tropa y la carga con las que saldría. **Nadie ha salido todavía**: las escuadras siguen en su campamento y el almacén intacto, así que cancelar o separarse no devuelve nada. Espera **sin caducidad**.

### 9.1 Modelo (`estado.convocatorias?`, ausente = `[]`)

```
Convocatoria {
  id: "convocatoria-{n}",  faccionId,  lugar: { tipo: asentamiento|campamento, id },
  liderId,  politicaDeUnion: aceptar | preguntar,                          (nunca «rechazar»)
  integrantes: [ { heroeId, unidoEn, escuadronIds[], carga{} } ],          (el Líder primero)
  peticiones?: [ { heroeId, pedidoEn, expiraEn, escuadronIds[], carga{} } ],
  creadaEn
}
```

`lugarDe(héroe)`: `asentamiento` si `ubicacion = asentamiento`, `campamento` si `mercenarios`, nada en una columna. `integrantesDentro(c, héroes)` = los integrantes que **siguen en el mismo lugar**: quien se fue o se desconectó no sale con los demás. Un héroe va como mucho en una convocatoria (`convocatoriaDe`).

### 9.2 La comprobación de «saldría con eso»

Convocar, unirse, aceptar una petición y cambiar la selección **simulan la salida real** del héroe sobre un estado que se descarta (`exigirQuePuedeSalir` → `salirDelLugar`), por el mismo camino que usará al partir:

| Lugar | Reside en él | No reside |
|---|---|---|
| Plaza | `salirAlMundo` con su selección | `salirDeAsentamiento` (su selección **se ignora**: sale con la columna que dejó aparcada) |
| Campamento | `salirDelCampamento` con su selección | `salirDelCampamento` (retoma la aparcada; la selección se ignora) |

Si no puede, falla con el motivo concreto: «{héroe} no puede salir con lo elegido: …». Reutilizar la salida garantiza que tropa, Liderazgo, carga y víveres se validen igual que al salir solo.

### 9.3 Los comandos

- **`convocarEjercito { heroeId, escuadronIds, carga, politicaDeUnion }`**: debe estar dentro de un lugar (si no, «se convoca desde dentro de una plaza o de un campamento»), tener Facción, no estar ya en una convocatoria y poder salir con lo elegido. Crea la convocatoria con él de Líder e integrante único. Evento `convocatoria.convocada`.
- **`unirseAConvocatoria { heroeId, convocatoriaId, … }`**: la convocatoria existe; el héroe está en el **mismo lugar** y es de la **misma Facción** (no vacía); no está ya en otra; y puede salir con lo elegido.
  - Política `aceptar`: se une ya (`convocatoria.unido`; quita su petición pendiente si la había). Devuelve `{ unido: true }`.
  - Política `preguntar`: deja una **petición** al Líder que vive `MOVIMIENTO.vidaPeticionUnionSegundos` (10 s); al anotar una nueva se barren las caducadas y la propia anterior. Evento `convocatoria.union_pedida { expiraEn }`; `{ unido: false }`.
- **`responderPeticionDeConvocatoria { heroeId, convocatoriaId, solicitanteId, aceptar }`**: solo el **Líder**; la petición debe existir y **no haber caducado** («el silencio cuenta como un no»). Rechazar la quita y avisa (`convocatoria.union_rechazada`). Aceptar **revalida todo** con la selección de la petición (sigue dentro, misma Facción, puede salir) y lo une (`convocatoria.unido`).
- **`cambiarSeleccionDeConvocatoria { heroeId, escuadronIds, carga }`**: cualquier integrante, también el Líder, cambia **solo la suya**; se valida como al unirse. El orden de `escuadronIds` es el orden con el que su columna sale y combate. Evento `convocatoria.seleccion_cambiada`.
- **`separarseDeConvocatoria`**: un integrante se va; sigue dentro del lugar. El **Líder no se separa: cancela**. Evento `convocatoria.separado`.
- **`cancelarConvocatoria`**: solo el Líder; la convocatoria desaparece y todos siguen dentro con todo en su sitio. Avisa también a quienes tenían una petición. Evento `convocatoria.cancelada`.
- **`partirConvocatoria`**: ver §9.4.

Todos exigen estar en una convocatoria (salvo convocar y unirse). Los eventos llevan `heroesIds` (Líder, protagonista, todos los integrantes y los avisados extra) para que lleguen al canal `heroe/<id>` de cada uno; se atribuyen a la plaza si el lugar es una plaza, a ninguna si es un campamento.

### 9.4 Partir (`partirConvocatoria`)

Solo el **Líder**. `dentro = integrantesDentro`; el **Líder debe ser el primero** de los que siguen dentro («El Líder ya no está dentro del lugar» si no).

```
para cada integrante de `dentro`, en orden:
    salirDelLugar(estado, …, su selección)        ← la salida REAL; si falla, falla TODO (ver abajo)
    anota (heroeId, ejercitoId de su columna)
base = la columna del Líder  →  se convierte en el ejército:
        tipo = ejercito, politicaDeUnion = la de la convocatoria,
        origenCampamentoId = lugar.id si el lugar es un campamento
para cada otra columna:  unirseEnCampo(ejército, columna con la posición del ejército)   ← todas salen por la misma puerta
        se quita su columna; el héroe pasa a ubicacion = columna del ejército
la convocatoria se elimina
```

**Todo o nada**: si alguien ya no puede salir con lo que eligió, el comando entero se aborta (transacción) y nadie sale; el mensaje dice quién y por qué. Resultado `{ ejercitoId }`, evento `convocatoria.partio { ejercitoId }` más los eventos de cada salida. El ejército nace **quieto en la puerta** y lo dirige su Líder con `marcharA`. Los integrantes que ya no estaban dentro **quedan fuera** y no salen.

### 9.5 Quién no puede salir por su cuenta

`sinConvocatoria` envuelve `salirAlMundo`, `salirDeAsentamiento`, `salirDelCampamento`, `movilizarEjercito` y `unirseAEjercito`: si el héroe va en una convocatoria, rechazan con «Estás en un ejército en preparación: sepárate o cancélalo antes de salir por tu cuenta» (`movilizacion.invalida`). `partirConvocatoria` usa los manejadores **sin** envolver. Los demás comandos (`entrarEn*`, `desconectarse`, `fundar`, `guarnecer`) no se envuelven.

### 9.6 Lo que ve cada uno (proyección)

`convocatoriasDelLugar`: las convocatorias **de la propia Facción del héroe en el mismo lugar**, más la suya aunque ya no esté. Cada una trae sus integrantes dentro (con tropa nombrada, nivel, moral y cantidad, **en el orden elegido**) y las **peticiones vivas** solo para el Líder y para quien las hizo. `soyLider`, `soyIntegrante` y `miConvocatoriaId` se derivan por héroe. Nada sale del lugar.

## 10. Autorización

Todos son `rolesPermitidos: ['jugador']` y exigen `heroeId === actor` (nadie actúa a nombre de otro), salvo `crearHeroe` (el único de un actor sin héroe) y estos:

| Comando | Condición añadida |
|---|---|
| `salirAlMundo` | reside en la plaza y las escuadras elegidas son suyas |
| `fijarPuerta` | Gobernador residente de la plaza **o** Rey de su Facción |
| `vetarJugador` | Gobernador residente de la plaza |
| los de convocatoria | `heroeId === actor` (el resto lo valida el motor) |

Un héroe `fuera` del mundo: solo `conectarse` (§7).

## 11. Parámetros

<!-- parametros:PRESENCIA -->
```json
{
  "retardoDesconexionMs": 150000,
  "topeAplazamientoMs": 300000
}
```
<!-- /parametros -->

(`MOVIMIENTO.radioPuerta` y `vidaPeticionUnionSegundos` están en el bloque `MOVIMIENTO` de [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §14; `PUERTA` en [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md).)

## 12. Invariantes

| Invariante | Test que lo vigila |
|---|---|
| `salirAlMundo`: saca con las tropas y la carga que elige, sin tropas también, rechaza carga que no cabe (tope sobre el total), no deja a la plaza sin la comida de su guarnición, no se sale dos veces | `session/__tests__/comandosPresencia.test.ts` |
| `entrarEnAsentamiento`: volver a la residencia deshace la columna; en otra queda aparcada; no desde lejos; no en un ejército; quien no está en el mundo no tiene puerta | `comandosPresencia.test.ts` |
| `salirDeAsentamiento`: devuelve a su columna con lo que llevaba; se rechaza desde la propia residencia | `comandosPresencia.test.ts` |
| `guarnecer` vuelca la tropa, consume el ejército y deja al jugador dentro; una columna personal no guarnece | `comandosPresencia.test.ts` |
| `marcharA`: pone en marcha, se rectifica, lo dirige el Líder, sin destino sale quieto, no desde dentro de una plaza, no por mar | `comandosPresencia.test.ts` |
| La puerta la controlan Gobernador y Rey; por defecto deja entrar a los suyos; el veto pesa más que el grupo y se levanta con el mismo comando; un residente entra aunque esté cerrada y no se le veta | `comandosPresencia.test.ts`, `engine/__tests__/puerta.test.ts` |
| Campamentos: sale con tropa y carga, al volver se deshace; lo que no cabe en el almacén queda en el carro; otro campamento aparca; lejos no se entra | `session/__tests__/entrarSalirCampamento.test.ts` |
| Desconectarse: dentro de la plaza no se mueve nada; en campo se lleva tropa y carro; se aplaza si le persiguen hasta el tope; el Líder cede el mando al conectado más antiguo; volver antes de 2:30 cancela | `session/__tests__/presenciaEnElMundo.test.ts` |
| Convocatoria en plaza: convocar no saca a nadie; parten como un ejército quieto; el Líder lo dirige; cancelar no mueve nada; separarse; política `preguntar`; no se sale por su cuenta ni se une a dos; no se convoca con carga que no podría salir; quien salió queda fuera al partir | `session/__tests__/convocatoria.test.ts` |
| Convocatoria en campamento; cambiar la selección (cada uno la suya, el orden es el de salida); la proyección; los avisos nombran a todos | `convocatoria.test.ts` |
| Ubicación: la grabada de exploración personal solo a quien está en columna sin bandera | `engine/__tests__/ubicacion.test.ts` |

**Sin test propio:** `ubicacionDeducida` y `estaEnAsentamiento` fuera de las pruebas de presencia, la caducidad de una petición de convocatoria, el reintento de `partirConvocatoria` cuando un integrante se queda sin Liderazgo, y la foto del interior al salir.

## 13. Casos límite y decisiones que parecen raras

- **Una convocatoria no caduca ni se limpia sola**: si su Líder se desconecta, desaparece de la plaza o es expulsado, sigue ahí hasta que él la cancele. Cancelar no mira el lugar: el Líder puede hacerlo desde donde esté, siempre que no esté `fuera` del mundo (entonces nadie puede, y la convocatoria espera a que vuelva).
- **Partir exige que el Líder siga dentro**; los demás pueden haberse ido (se quedan fuera) pero el Líder no.
- **La selección de un visitante se ignora al partir**: un no residente sale con la columna que dejó aparcada, elija lo que elija; la comprobación de «saldría» es solo que tiene esa columna.
- **Entrar a tu residencia con carga que no cabe pierde lo que desborda la capacidad del almacén de la plaza**: `agregarRecurso` topa por capacidad; en un campamento, en cambio, el resto se queda en el carro aparcado.
- **El almacén personal se llena en orden de recurso del carro**, no por prioridad: el último que no cabe es el que se queda fuera.
- **`salirDeAsentamiento` no comprueba el lugar**: basta tener columna y no residir en esa plaza; es inocuo (la ubicación pasa a su columna) pero no valida que estuviera dentro.
- **El ejército entra por `guarnecer` solo a plazas propias**; a una ajena entra cada héroe con su columna personal, separado antes.
- **Desconectado no es ausente**: dentro de una plaza se queda donde está, defiende la guarnición y no él en persona (Combate §3.2); en el mapa, sale con su columna y no se le puede cazar.
- **`vetarJugador` solo lo hace el Gobernador, `fijarPuerta` también el Rey**: el Rey puede cerrar grupos pero no vetar a un jugador concreto.

## 14. Divergencias

Ninguna abierta.
