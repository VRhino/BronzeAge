# El héroe y su tropa

## 1. Alcance y fuentes

Cubre al **héroe como personaje** (cómo nace, qué puntos y qué monedas tiene, cómo se hiere), su **Liderazgo**, sus **loadouts**, su **guarnición** y el **orden** de sus escuadras, y la **tropa**: qué es una escuadra, dónde puede estar, cómo
se **recluta** en una plaza (la cadena de condiciones y el coste), cómo se construye el catálogo de tropas y la **chatarra** que dejan los caídos.

- **Verificado contra:** `da09799` (2026-10-12).
- **Fuentes:** `engine/heroe.ts`, `engine/liderazgo.ts`, `engine/tropa.ts`, `engine/tropas.ts`, `engine/chatarra.ts`, `engine/asentamientoQuery.ts` (`cupoGuarnicion`, `poblacionDisponibleParaReclutar`), `engine/pertenencia.ts` (`puedeReclutarEn`),
  `engine/politicas.ts` (`factorCostoReclutamiento`), `session/comandos/{crearHeroe,heroe,militar}.ts`, `session/comandos/autorizacion.ts` (quién puede cada uno), `session/resultadoBatalla.ts` (`conLoQueTrajo`, lo que Unity añade al héroe y a sus escuadras).
- **Canon:** [`Docs/Game/5_Sistema_Militar_y_Combate.md`](../Game/5_Sistema_Militar_y_Combate.md) (5.4 hambre, 5.7–5.8 tropas y reclutamiento, 5.11 Liderazgo, 5.15.2–5.15.3 campamento y guarnición, 5.16 el Héroe), Doc 2.5 (residencia).
- **No está aquí** (está en su documento):
  - La ración y el hambre de una columna, los víveres y el almacén personal → [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §3.1 y §6 (`avanzarRacion` es una sola función; la guarnición la usa con factor 1).
  - El poder de una escuadra en un combate, las bajas y la experiencia que deja → [`Combate.md`](Combate.md) §3–§4. Aquí solo se repite la fórmula de nivel.
  - Reclutar y pedir prestada tropa en un campamento de mercenarios, y crear el héroe **dentro** de uno → [`Comercio_Caravanas.md`](Comercio_Caravanas.md) §9 y [`Generacion_Del_Mundo.md`](Generacion_Del_Mundo.md) §11.
  - Dónde está el héroe (conectarse, entrar y salir de una plaza, `ubicacion`) y sacar la tropa a campaña con `movilizarEjercito`, que **exige** el Liderazgo definido aquí → `Movimiento_Ejercitos.md`.
  - El ciclo de la batalla de Unity, que aplica XP y botín al héroe (§4.3 resume lo que le hace).
  - El mantenimiento de la población que reclutar consume → [`Economia_Construccion.md`](Economia_Construccion.md) §9; las tecnologías que habilitan cada tropa → [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md).

## 2. Modelo de datos

### 2.1 `Heroe` (`estado.heroes`)

Uno por jugador y partida. Es el **dueño** de todo lo que antes se atribuía al jugador: sus escuadras, su residencia y su ciudadanía cuelgan de él por id.

| Campo | Significado |
|---|---|
| `id`, `jugadorId` | `heroe-{n}` (siguiente id de la partida) y la identidad de la cuenta (`acceso/`). La acción de un jugador sin héroe aún usa su `jugadorId` como actor (solo `crearHeroe`). |
| `controlador` | `'humano'` por defecto; `'bot'` solo lo pone el servidor (no admite el esquema HTTP). Decide si una batalla lleva Unity (hay algún humano) o se resuelve con números. |
| `displayName`, `classDefinitionId`, `genero`, `avatar` | Identidad y aspecto. El motor no los interpreta: la clase y el catálogo de cabezas, pelo, barba y cejas son de Conquest. |
| `liderazgoBase` | Puntos de Liderazgo (§5). Nace en `LIDERAZGO.base`; **nada lo modifica hoy**. |
| `ubicacion` | `asentamiento` · `columna` · `mercenarios` · `desconectado`. Ver [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md). |
| `escuadrones` | **Todas** sus escuadras, estén donde estén; `contenedor` dice dónde (§2.3). |
| `nivel`, `experienciaHaciaSiguienteNivel` | Nivel y XP hacia el siguiente (no acumulada). |
| `puntosDeAtributoSinGastar`, `puntosDePerkSinGastar`, `atributosBase`, `perksDesbloqueados` | Progresión traída de Conquest (§4). |
| `loadouts` | Selecciones de escuadras guardadas (§6). |
| `inventario`, `equipamiento`, `monedasHeroe` | Objetos y las tres monedas del héroe (`bronce`/`plata`/`oro`), **sin relación** con el oro-recurso de las plazas. |
| `heridoHasta?` | Instante hasta el que está herido (§9). |
| `desconectaEn?`, `fuera?` | Desconexión (Doc 1.10.6); `fuera` ausente = conectado. |
| `viveres?`, `almacenPersonal?`, `exploracionPersonal?`, … | Ver sus documentos. |

`AtributosHeroe = { fuerza, destreza, armadura, vitalidad }`; `Equipamiento` = 6 huecos (`arma`, `casco`, `torso`, `guantes`, `pantalones`, `botas`), cada uno un `ItemInstancia` o `null`.

### 2.2 `Loadout`

`{ id, displayName, squadIds[], perksSeleccionados[], activo }`. Una selección de escuadras (y perks) guardada para salir. El **activo** es el que defiende una plaza en persona (Combate §3.2). El Liderazgo que suma **no se guarda**: se deriva (§5).

### 2.3 `Escuadron` (vive en su héroe)

| Campo | Significado |
|---|---|
| `id`, `nombre`, `heroeId`, `tropaId`, `origen` | `escuadron-{asentamiento.id}-{contador}` al reclutar; `nombre = "{tropa.nombre} de {heroeId}"`; la clase de población de la que salió (§11.2). |
| `cantidad` | Soldados. **Persiste a 0** (aniquilada) y se repone reclutando; nunca desaparece. |
| `nivel`, `experiencia` | Nivel (1‥`nivelMaximoEscuadra`) y XP **acumulada** (§10.2). |
| `moral` | 0–100. A 0 hay deserción continua (Movimiento §6.3). |
| `habilidadesDesbloqueadas`, `formacionesDesbloqueadas`, `formacionSeleccionada` | Progresión táctica de Conquest; el servidor la guarda y la sirve, no la interpreta. Nacen `[]`, `[]`, `0`. |
| `contenedor` | `campamento` (en la plaza donde reside su héroe, o en ninguna si es huérfano) · `ejercito {ejercitoId}` · `escolta {caravanaId}` · `fuera` (con su héroe desconectado). |
| `enGuarnicion` | En la guarnición de la plaza donde reside su héroe (§7). Solo con `contenedor: campamento`. |
| `reservaBatalla?` | Candado `{battleId}` mientras está reservada para una batalla de Unity. |
| `prestada?` | `{campamentoId}`: tropa prestada por un campamento de mercenarios. **No es del héroe**: no gana experiencia, no cuenta para «una escuadra por tropa» y se pierde al dejar de residir allí. |

### 2.4 Las vistas (`EjercitoConTropa`, `CaravanaConEscolta`)

Los ejércitos y las caravanas guardan solo los **ids** de sus escuadras (`escuadronIds`, `escoltaIds`). El motor militar trabaja sobre **vistas** que las llevan puestas, que monta al entrar en una operación y deshace al salir:

- `indiceTropa(heroes)`: `id → Escuadron`, de todas las escuadras de todos los héroes.
- `conTropa(ejercito, indice)`: añade `escuadrones` (las que existen, en el orden de `escuadronIds`).
- `sinTropa(vista)`: devuelve `{ ejercito (sin escuadrones, con escuadronIds), tropa }`; la tropa vuelve marcada `contenedor: ejercito` y `enGuarnicion: false`.
- `conEscolta` / `sinEscolta`: lo mismo para la escolta de una caravana (`contenedor: escolta`; sin escolta, `escoltaIds` desaparece).
- `conEscuadrones(heroes, actualizadas)`: devuelve a su héroe las escuadras que cambiaron (por id) y añade las nuevas. Una escuadra cuyo héroe no existe **lanza** (`Error`): es un estado corrupto, no algo que ignorar.
- `campamentoDe(plaza, heroes)`: las escuadras con `contenedor: campamento` de los **residentes** de la plaza (lo que come del almacén de la plaza). `guarnicionDe`: las de esas con `enGuarnicion`. `sinGuarnicion(heroes, heroeId)`: suelta la guarnición de un héroe (al dejar de residir, Diplomacia §5).

## 3. Crear el héroe (`crearHeroe`)

Lo primero que hace un jugador. Validaciones, en este orden (cada una rechaza con su código):

1. `displayName.trim()` vacío → `heroe.nombre_vacio`.
2. Ya existe un héroe con `jugadorId === actor` o `id === actor` → `heroe.ya_existe` (uno por jugador y partida).
3. El `campamentoId` no existe → `heroe.campamento_desconocido`.

Efecto: `heroe-{ids.siguiente()}`, `controlador = params.controlador ?? 'humano'`, `liderazgoBase = LIDERAZGO.base`, `ubicacion = mercenarios {campamento}`, `escuadrones = []` y la progresión inicial (§4.1); el campamento añade al héroe a `residentesIds` y suma 1 a `eligieronComoInicial`. Evento `heroe.creado`. **No tiene columna en el mapa**: sale cuando quiera, como de su casa.
`classDefinitionId` y `avatar` no se validan contra ningún catálogo (el esquema solo exige cadenas); `genero` ∈ `masculino | femenino`. Autorización: es el **único** comando de un actor sin héroe.

## 4. Progresión del héroe

### 4.1 Con qué nace (`progresionInicial`)

```
nivel = 1 ; experienciaHaciaSiguienteNivel = 0 ; puntosDeAtributoSinGastar = 0 ; puntosDePerkSinGastar = 0
atributosBase = { fuerza: 0, destreza: 0, armadura: 0, vitalidad: 0 } ; perksDesbloqueados = []
loadouts = [ { id: "{heroeId}-loadout-default", displayName: "Default", squadIds: [], perksSeleccionados: [], activo: true } ]
inventario = [] ; equipamiento = todos los huecos a null
monedasHeroe = { bronce: HEROE.bronceInicial, plata: 0, oro: 0 }
```

Como en Conquest (`HeroDataService.CreateNewHero`), pero **sin equipo inicial** hasta que exista el catálogo de objetos.

### 4.2 Repartir puntos (`repartirPuntos`)

`repartirAtributos(heroe, reparto)`: cada punto suma 1 a su atributo. Rechaza (`heroe.invalido`): alguna cantidad que no sea un entero ≥ 0; un reparto de **total 0**; un total mayor que `puntosDeAtributoSinGastar`; o dejar un atributo por encima de `HEROE.topeAtributo`
(la comprobación es atributo a atributo y **tras** sumar, sin deshacer nada: el comando entero se aborta). Los perks esperan al catálogo de Conquest: no hay comando para gastarlos.

### 4.3 Qué le suma el servidor

- **Nivel:** el servidor **no lo sube nunca** (no hay curva hasta que Conquest la publique, CQ-001; decidido 2026-09-15). Por tanto `puntosDeAtributoSinGastar` y `puntosDePerkSinGastar` nacen en 0 y **ningún camino los aumenta**: `repartirPuntos` solo puede gastar lo que traiga un snapshot cargado a mano.
- **XP:** una batalla de Unity la suma tal cual (`experienciaHaciaSiguienteNivel += xpGanada` del héroe) y, por cada escuadra, `conExperiencia(e, xpGanada)` y `cantidad = supervivientesAlCierre`.
- **Botín de Unity:** cada objeto va a una casilla libre del inventario (`BATALLA.casillasInventario`), como `tipo: 'visual'` y `precio: 0` (sin catálogo de objetos, CQ-004); las monedas se suman a `monedasHeroe`.
- **Equipar:** no existe el comando; el equipamiento solo cambia por la vía de Conquest.

### 4.4 Eventos del héroe

Sin plaza (globales del héroe):

| Evento | Comando | Payload |
|---|---|---|
| `heroe.puntos_repartidos` | `repartirPuntos` | `{ heroeId, atributos }` |
| `heroe.almacen_personal` | `guardarEnAlmacenPersonal` / `sacarDelAlmacenPersonal` | `{ heroeId, recurso, cantidad (lo movido), sentido: guarda \| saca }` |
| `heroe.viveres` | `pasarAViveres` | `{ heroeId, cantidad (lo movido) }` |

## 5. Liderazgo

Un límite de **salida**, no de posesión: se puede poseer mucha más tropa de la que se saca de una vez.

```
costeLiderazgo(tropaId) = LIDERAZGO.costePorEscalon[escalón de la tropa]      ; 0 si el tropaId no existe en el catálogo
                          (no mira la cantidad: una escuadra mermada paga el precio completo)
liderazgoDe(héroe)       = héroe.liderazgoBase ?? LIDERAZGO.base
comprometido(escuadras)  = Σ costeLiderazgo(e.tropaId)
puedeLlevar(héroe, esc)  = comprometido(esc) ≤ liderazgoDe(héroe)
disponible(héroe, esc)   = max(0, liderazgoDe(héroe) − comprometido(esc))
```

El coste sigue al **escalón**, no al poder (`poderBase`): así el poder por punto de Liderazgo difiere entre tropas y elegir composición es una decisión. En un ejército de varios héroes, cada uno se valida contra **el suyo**; no hay tope agregado.
Con los números de hoy (7/14/22/32/45): 1 élite + 1 pesada + 1 veterana = 99; 2 pesadas + 1 veterana + 1 de línea = 100.

## 6. Loadouts (`guardarLoadout`, `borrarLoadout`)

`guardarLoadout(heroe, datos)` crea (sin `id`) o reescribe (con `id`) un loadout. Validaciones, en este orden (`heroe.invalido`):

1. `displayName.trim()` no vacío.
2. Con `id`, el loadout existe.
3. Sin escuadras repetidas.
4. Todas las `squadIds` son **suyas** (están en `heroe.escuadrones`).
5. `puedeLlevar`: las escuadras caben en su Liderazgo.
6. Todo perk de `perksSeleccionados` está en `perksDesbloqueados`.

Resultado: `id = anterior.id` o `loadout-{ids.siguiente()}`; `activo = datos.activo ?? anterior.activo ?? false`. **Marcarlo activo desactiva los demás.** Uno nuevo se **añade al final**. El comando devuelve `{ loadoutId, liderazgoTotal }` (el Liderazgo que suma, derivado). No emite eventos.

Es una comodidad: **no hay tope de loadouts** y **la misma escuadra puede estar en varios** (solo uno está activo a la vez, así que no compiten); **no mira dónde están las escuadras** (pueden estar en un ejército o heridas…); eso se comprueba al salir con ellas. `borrarLoadout` quita el loadout (rechaza si no existe) y **no garantiza que quede uno activo**: sin loadout activo, el héroe defiende con ninguna escuadra de su loadout (Combate §3.2 usa `loadouts.find(activo)?.squadIds ?? []`).

## 7. Guarnición

La guarnición de una plaza son las escuadras que sus residentes han entregado a la IA: defienden siempre, aun con su dueño herido o desconectado, y viven en el campamento (`enGuarnicion`).

**Cupo** (`cupoGuarnicion(plaza)`, **por héroe residente**, en puntos de Liderazgo):

```
edificios = Σ para Barracón y Galería de tiro (el PRIMERO activo de cada tipo):  GUARNICION.cupoPorNivelEdificio[nivelInterno − 1]
          + (Caballerizas, la primera activa) GUARNICION.cupoPorNivelCaballerizas[nivelInterno − 1]
si edificios = 0 → 0                                                         ; sin Barracón ni Galería no hay guarnición, aunque haya muralla
muralla   = el MAYOR nivel de entre los recintos con integridad ≥ 1  →  GUARNICION.recintoCompletoPorNivel[nivel − 1]    ; 0 si no hay
cupo      = edificios + muralla + cupoGuarnicionExtra(plaza)                  ; la política «Levas de guarnición»
```

Un recinto en mejora vuelve a integridad 0 y mientras tanto no suma.

**`asignarGuarnicion(heroe, residencia, squadId)`** — la `residencia` es el asentamiento donde el héroe es residente (`esResidente`); rechaza (`heroe.invalido`): sin residencia en un asentamiento; escuadra que no es suya; que esté **reservada para una batalla** (`reservaBatalla`); que no esté en `contenedor: campamento`; que ya esté en la guarnición;
o que `guarnicionOcupada(heroe) + costeLiderazgo(esa tropa) > cupo` (`guarnicionOcupada` = Liderazgo de sus escuadras con `enGuarnicion`). Marca `enGuarnicion = true`. **Si el cupo baja después, lo asignado se queda**: solo impide asignar más.
**`retirarGuarnicion`**: rechaza si esa escuadra no está en la guarnición o está reservada para una batalla; la deja `enGuarnicion = false`.

Mientras una escuadra está en la guarnición, el héroe **no puede sacarla** de la plaza (la salida a campaña lo rechaza: `El escuadrón … está en la guarnición`). Dejar de residir suelta la guarnición (`sinGuarnicion`).
El campamento (todas las escuadras con `contenedor: campamento` de los residentes) **come del almacén de la plaza**: `avanzarMantenimientoTropas` (tick, paso 5) llama a `avanzarRacion(campamento, trigo del almacén)` con factor 1 y 0 participantes, y descuenta el trigo consumido del almacén ([`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md), [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §6.3).

## 8. Orden de las escuadras (`ordenarEscuadras`)

`ordenarEscuadras(heroe, columna, ids)`: rechaza una escuadra repetida o ajena. Pone las de `ids` **primero, en ese orden**, y el resto detrás **como estaban**. Es el orden en que entran en combate: Unity las toma de `heroe.escuadrones`; el combate con números, de `columna.escuadronIds`.
Si el héroe va en una columna, **sus escuadras ocupan en `escuadronIds` los mismos huecos que ya tenían, en el orden nuevo**; las de otros héroes no se mueven.

## 9. Herido

`estaHerido(h, ahora) = h.heridoHasta !== undefined ∧ ahora < h.heridoHasta`: se comprueba **al leer**; nada se dispara al vencer. `herir(heroes, ids, ahora)` pone `heridoHasta = ahora + HEROE.heridoMinutos` (2 min de mundo) a cada héroe de `ids` que **no lo esté ya** (a un herido no se le alarga:
no ha podido volver a perder, porque un herido no entra en batallas). Un herido no persigue, no se le persigue, no entra en batallas y sus escuadras no combaten sin él (excepto la guarnición). `heridosEn(heroes, ahora)` devuelve el conjunto de ids heridos que reciben los resolutores de combate. Quién queda herido y cuándo: [`Combate.md`](Combate.md) §11.

## 10. Las escuadras

### 10.1 Una por tropa

Como mucho **una escuadra por (`heroeId`, `tropaId`)** en toda la partida (sin contar las `prestada`): reclutar la misma tropa **repone** la que hay (§11). Tope de soldados por escuadra: `tropa.unidadesPorDefecto`.

### 10.2 Nivel y experiencia

```
nivelDeEscuadra(xp):  nivel = 1 ; resto = xp
    para pide en MILITAR.experienciaParaSubirEscuadra:
        si nivel ≥ nivelMaximoEscuadra  o  resto < pide:  parar
        resto −= pide ; nivel++
conExperiencia(e, ganada) = e                                  si e.prestada
                          = { ...e, experiencia: e.experiencia + ganada, nivel: nivelDeEscuadra(experiencia) }
poderEscuadron(e) = tropa.poderBase × e.cantidad × (1 + e.nivel × MILITAR.bonusPoderPorNivelEscuadra)
```

Con la curva de hoy, 275 de XP llevan al nivel 10. El nivel da **+1 % por nivel al mismo escuadrón** (el nivel 1 ya cuenta) sin cambiar nunca su `tropaId`: «mejorar» una tropa es reclutar **otra** distinta y mejor cuando el edificio suba de nivel interno.

### 10.3 Chatarra de los caídos (`chatarra.ts`)

El metal del equipo de los caídos se recoge como un recurso aparte (para que Armería y Gran Fundición no formen un bucle). Por soldado de una tropa:

```
para cada (equipo, piezas) de tropa.costoEquipo:
    para cada (lingote, porPieza) que gasta fabricar ese equipo:             ← las recetas de Armería y Carpintería de todos los niveles (si hay varias, gana la última)
        si el lingote tiene chatarra (lingoteCobre → chatarraCobre ; lingoteBronce → chatarraBronce ; lingoteHierro → chatarraHierro):
            chatarra[recurso] += piezas × porPieza × CHATARRA.fraccion
```

Sin equipo de metal (madera, cuero) no deja nada. `chatarraDeMuertos(tropaId, n)` = lo anterior × n (0 si n ≤ 0); `chatarraDeBajas(antes, despues)` suma los caídos de cada escuadra (cruzadas por `id`: `antes.cantidad − despues.cantidad`; una escuadra que no aparece en `despues` no pierde nada); `sumarChatarra` suma recursos. Quién la recoge y qué hace la Gran Fundición con ella: Combate y Economía.

## 11. Reclutar tropa en una plaza (`reclutarTropa`)

El único origen de escuadras del motor (fuera del préstamo de un campamento de mercenarios). Comando `reclutarTropa { asentamientoId, heroeId, tropaId, origen? }` — `origen` ya no decide nada (la clase de población la pone el escalón) y solo se admite por compatibilidad.

### 11.1 Quién y dónde

`puedeReclutarEn(plaza, heroeId, facciónDelJugador)`:

- **`todo`** si el héroe **reside** en la plaza: escuadra nueva, reposición y cualquier tropa que el edificio permita.
- **`no`** si no reside y su Facción no es la de la plaza, **o** está vetado (`vetadosIds`).
- **`solo_reponer`** si no reside, es ciudadano de la Facción de la plaza y no está vetado: solo repone una escuadra que ya tiene **aquí** (en su columna a la puerta); nunca una nueva.

La autorización del comando exige además `heroeId === actor` y que `puedeReclutarEn ≠ 'no'` **y el héroe esté presente en la plaza**. Reclutar **no exige el cargo de General** (la residencia manda, Doc 2.5).

### 11.2 Condiciones, en el orden en que se comprueban

Todas lanzan `tropas.reclutamiento_invalido`:

1. Permiso `no`.
2. La tropa no está en el catálogo.
3. La **tecnología** que habilita la tropa no está adoptada por la **Facción de la plaza** (`tecnologiasDe(estado, plaza.faccionId).adoptadas`, Doc 6.1).
4. El héroe no existe.
5. `existente` = su escuadra de esa tropa (no prestada). Si está **reservada para una batalla** (`reservaBatalla`): rechazo (no se repone hasta que acabe). Si existe y **no es reponible aquí**: rechazo. Reponible aquí = `contenedor: campamento` y el héroe reside en esta plaza; o `contenedor: ejercito` y esa columna está a `≤ MOVIMIENTO.radioPuerta` de la plaza. En escolta o fuera del mundo: nunca.
6. Permiso `solo_reponer` y no hay `existente`.
7. `cantidad = tropa.unidadesPorDefecto − (existente?.cantidad ?? 0)`; si `cantidad ≤ 0`, ya está al tope.
8. **Población**: `poblacionDisponibleParaReclutar(plaza, origen) ≥ cantidad`, con `origen = poblacionDeTropa(tropa)`: escalón 1–2 → `pesants`, 3 → `artesanos`, 4–5 → `nobleza`.
   Disponible = nobleza → toda la nobleza; artesanos → `max(0, artesanos − trabajadoresRequeridosTransformacion)`; pesants → el excedente de mano de obra (`manoObraInfo.excedente`). No se recluta lo que ya hace falta para producir.
9. **Edificio**: el **primer** edificio `activo` de `tropa.edificio` tiene `nivelInterno ≥ tropa.nivelRequerido` (nivel interno 1 si no lo trae).
10. **Coste** (§11.3): el almacén lo cubre.
11. **Reserva de trigo**: `almacén.trigo ≥ reservaDeTrigo(plaza, consumo del campamento actual, cantidad × MILITAR.racionPorSoldadoPorMinuto)`, con
    `reservaDeTrigo = (consumoComidaPoblacion + consumoTropas + consumoExtra) × RESERVA_CONSTRUCCION.horizonteMinutosComida` y `consumoTropas = Σ soldados del campamento × racionPorSoldadoPorMinuto`. Es la misma vara con la que construyen y salen los ejércitos.

### 11.3 Coste

```
equipo[r]  = tropa.costoEquipo[r] × cantidad × factorCostoReclutamiento(plaza)         ; «Leva Forzosa» (General) = 0,7
si tropa.edificio ≠ centroUrbano:                                                       ; la milicia del Centro Urbano no cuesta oro
    oroPorSoldado = RECLUTAMIENTO_ORO_POR_ESCALON[escalón] + (tropa.caballos ?? 0) × ORO_POR_CABALLO
    equipo.oro   += oroPorSoldado × cantidad                                            ; el factor de Leva Forzosa NO toca el oro
```

El oro y el equipo salen del **almacén de la plaza** (no del almacén personal del héroe).

### 11.4 Efecto

- Si existe la escuadra: `cantidad += cantidad` (moral, nivel y experiencia **no cambian**). Si no: una nueva `escuadron-{plaza.id}-{ids.siguiente()}` con `PROGRESION_INICIAL` (nivel 1, experiencia 0, sin habilidades ni formaciones), `moral = 100`, `contenedor: campamento`, `enGuarnicion: false`.
- `plaza.poblacion[origen] −= cantidad`; `plaza.almacen −= coste`.
- El comando suma a los contadores de la tecnología: `reclutados.escuadrones` si la escuadra es nueva, `reclutados.caballeria` (× 5 para `carros_guerra`) si es de Caballerizas, y `reclutados.{tropaId}` si esa tropa tiene logro cantado ([`Tecnologia_Aedas.md`](Tecnologia_Aedas.md)).
- Anota en el historial del jugador `Recluta {n} de "{tropa}" en {plaza}` y emite `tropas.reclutadas { asentamientoId, heroeId, tropaId, origen, reclutados }` a la plaza.
- Resultado `{ reclutados }` = la `cantidad` final menos la anterior.

## 12. Parámetros

Las del catálogo viven en `constants.ts`; los números de balance de combate y de ración de `MILITAR` están en [`Combate.md`](Combate.md) §12.

<!-- parametros:LIDERAZGO -->
```json
{
  "base": 100,
  "costePorEscalon": {
    "1": 7,
    "2": 14,
    "3": 22,
    "4": 32,
    "5": 45
  }
}
```
<!-- /parametros -->

<!-- parametros:HEROE -->
```json
{
  "bronceInicial": 500,
  "topeAtributo": 100,
  "heridoMinutos": 2
}
```
<!-- /parametros -->

<!-- parametros:GUARNICION -->
```json
{
  "cupoPorNivelEdificio": [
    7,
    14,
    22
  ],
  "cupoPorNivelCaballerizas": [
    0,
    14,
    32
  ],
  "recintoCompletoPorNivel": [
    10,
    15,
    20
  ]
}
```
<!-- /parametros -->

<!-- parametros:RECLUTAMIENTO_ORO_POR_ESCALON -->
```json
{
  "1": 1,
  "2": 2,
  "3": 4,
  "4": 7,
  "5": 11
}
```
<!-- /parametros -->

<!-- parametros:ORO_POR_CABALLO -->
```json
5
```
<!-- /parametros -->

<!-- parametros:CHATARRA -->
```json
{
  "fraccion": 0.5
}
```
<!-- /parametros -->

<!-- parametros:RESERVA_CONSTRUCCION -->
```json
{
  "horizonteMinutosMantenimiento": 8,
  "horizonteMinutosComida": 8
}
```
<!-- /parametros -->

<!-- parametros:TROPAS_RECLUTABLES -->
```json
[
  {
    "id": "milicia_lanceros",
    "tecnologia": "leva_comunal",
    "nombre": "Milicia de lanceros",
    "edificio": "centroUrbano",
    "nivelRequerido": 1,
    "costoEquipo": {
      "madera": 2
    },
    "poderBase": 2,
    "velocidad": 20,
    "escalon": 2,
    "unidadesPorDefecto": 25
  },
  {
    "id": "lenadores",
    "tecnologia": "leva_comunal",
    "nombre": "Leñadores",
    "edificio": "centroUrbano",
    "nivelRequerido": 1,
    "costoEquipo": {
      "madera": 1
    },
    "poderBase": 1,
    "velocidad": 20,
    "escalon": 1,
    "unidadesPorDefecto": 30
  },
  {
    "id": "granjeros",
    "tecnologia": "leva_comunal",
    "nombre": "Granjeros",
    "edificio": "centroUrbano",
    "nivelRequerido": 1,
    "costoEquipo": {},
    "poderBase": 1,
    "velocidad": 20,
    "escalon": 1,
    "unidadesPorDefecto": 30
  },
  {
    "id": "lanceros_mimbre",
    "tecnologia": "escudos_ligeros",
    "nombre": "Lanceros con escudo de mimbre",
    "edificio": "barracon",
    "nivelRequerido": 1,
    "costoEquipo": {
      "armaMadera": 1
    },
    "poderBase": 3,
    "velocidad": 20,
    "escalon": 2,
    "unidadesPorDefecto": 25
  },
  {
    "id": "espadachines_cobre",
    "tecnologia": "armamento_palacial",
    "nombre": "Espadachines de espada corta de cobre",
    "edificio": "barracon",
    "nivelRequerido": 1,
    "costoEquipo": {
      "armaCobre": 1,
      "armaduraBasica": 1
    },
    "poderBase": 4,
    "velocidad": 16,
    "escalon": 2,
    "unidadesPorDefecto": 20
  },
  {
    "id": "hacheros_ligeros",
    "tecnologia": "armamento_palacial",
    "nombre": "Hacheros ligeros",
    "edificio": "barracon",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaBronce": 1,
      "armaduraBasica": 1
    },
    "poderBase": 7,
    "velocidad": 16,
    "escalon": 3,
    "unidadesPorDefecto": 18
  },
  {
    "id": "espadachines_bronce",
    "tecnologia": "aleacion_bronce",
    "nombre": "Espadachines con espadas y escudos de bronce",
    "edificio": "barracon",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaBronce": 2,
      "armaduraIntermedia": 1
    },
    "poderBase": 9,
    "velocidad": 16,
    "escalon": 3,
    "unidadesPorDefecto": 18
  },
  {
    "id": "lanceros_pesados",
    "tecnologia": "panoplia_bronce",
    "nombre": "Lanceros pesados micénicos",
    "edificio": "barracon",
    "nivelRequerido": 3,
    "costoEquipo": {
      "armaBronce": 2,
      "armaduraBronce": 1
    },
    "poderBase": 14,
    "velocidad": 12,
    "escalon": 4,
    "unidadesPorDefecto": 15
  },
  {
    "id": "hacheros_armados",
    "tecnologia": "aleacion_bronce",
    "nombre": "Hacheros armados",
    "edificio": "barracon",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaBronce": 1,
      "armaduraIntermedia": 1
    },
    "poderBase": 12,
    "velocidad": 12,
    "escalon": 3,
    "unidadesPorDefecto": 15
  },
  {
    "id": "honderos",
    "tecnologia": "hostigamiento_tribal",
    "nombre": "Honderos",
    "edificio": "galeriaDeTiro",
    "nivelRequerido": 1,
    "costoEquipo": {
      "armaMadera": 1
    },
    "poderBase": 5,
    "velocidad": 20,
    "escalon": 2,
    "unidadesPorDefecto": 20
  },
  {
    "id": "escaramuzadores_jabalina",
    "tecnologia": "hostigamiento_tribal",
    "nombre": "Escaramuzadores con jabalina",
    "edificio": "galeriaDeTiro",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaCobre": 1,
      "armaduraBasica": 1
    },
    "poderBase": 8,
    "velocidad": 20,
    "escalon": 3,
    "unidadesPorDefecto": 18
  },
  {
    "id": "arqueros",
    "tecnologia": "arqueria_palacial",
    "nombre": "Arqueros",
    "edificio": "galeriaDeTiro",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaBronce": 1,
      "armaduraIntermedia": 1
    },
    "poderBase": 9,
    "velocidad": 16,
    "escalon": 3,
    "unidadesPorDefecto": 18
  },
  {
    "id": "arqueros_compuesto",
    "tecnologia": "arco_compuesto",
    "nombre": "Arqueros con arco compuesto",
    "edificio": "galeriaDeTiro",
    "nivelRequerido": 3,
    "costoEquipo": {
      "armaBronce": 3,
      "armaduraIntermedia": 2
    },
    "poderBase": 15,
    "velocidad": 12,
    "escalon": 5,
    "unidadesPorDefecto": 12
  },
  {
    "id": "exploradores_caballo",
    "tecnologia": "cria_caballar",
    "nombre": "Exploradores a caballo",
    "edificio": "caballerizas",
    "nivelRequerido": 1,
    "costoEquipo": {
      "armaCobre": 1
    },
    "caballos": 1,
    "poderBase": 3,
    "velocidad": 28,
    "escalon": 2,
    "unidadesPorDefecto": 20
  },
  {
    "id": "carros_guerra",
    "tecnologia": "carros_guerra",
    "nombre": "Carros de guerra",
    "edificio": "caballerizas",
    "nivelRequerido": 2,
    "costoEquipo": {
      "carroGuerra": 1,
      "armaBronce": 1
    },
    "caballos": 2,
    "poderBase": 13,
    "velocidad": 20,
    "escalon": 4,
    "unidadesPorDefecto": 15
  },
  {
    "id": "jinetes_asirios",
    "tecnologia": "equitacion_militar",
    "nombre": "Jinetes asirios",
    "edificio": "caballerizas",
    "nivelRequerido": 1,
    "costoEquipo": {
      "armaHierro": 1
    },
    "caballos": 1,
    "poderBase": 5,
    "velocidad": 28,
    "escalon": 2,
    "unidadesPorDefecto": 20
  },
  {
    "id": "guerreros_filisteos",
    "tecnologia": "forja_hierro_temprana",
    "nombre": "Guerreros filisteos (Peleset)",
    "edificio": "barracon",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaHierro": 1,
      "armaduraIntermedia": 1
    },
    "poderBase": 9,
    "velocidad": 16,
    "escalon": 3,
    "unidadesPorDefecto": 18
  },
  {
    "id": "shardana",
    "tecnologia": "bronce_calidad_militar",
    "nombre": "Shardana",
    "edificio": "barracon",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaBronceCalidad": 1,
      "armaduraBasica": 1
    },
    "poderBase": 15,
    "velocidad": 16,
    "escalon": 4,
    "unidadesPorDefecto": 15
  },
  {
    "id": "hequetai",
    "tecnologia": "panoplia_bronce",
    "nombre": "Hequetai",
    "edificio": "barracon",
    "nivelRequerido": 3,
    "costoEquipo": {
      "armaBronceCalidad": 1,
      "armaduraBronce": 1
    },
    "poderBase": 17,
    "velocidad": 12,
    "escalon": 5,
    "unidadesPorDefecto": 12
  },
  {
    "id": "honderos_rodios",
    "tecnologia": "pantalla_escaramuzadores",
    "nombre": "Honderos rodios",
    "edificio": "galeriaDeTiro",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaHierro": 1,
      "armaduraBasica": 1
    },
    "poderBase": 9,
    "velocidad": 20,
    "escalon": 3,
    "unidadesPorDefecto": 18
  },
  {
    "id": "peltastas",
    "tecnologia": "pantalla_escaramuzadores",
    "nombre": "Peltastas",
    "edificio": "galeriaDeTiro",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaHierro": 2
    },
    "poderBase": 9,
    "velocidad": 20,
    "escalon": 3,
    "unidadesPorDefecto": 18
  },
  {
    "id": "jinetes_escitas",
    "tecnologia": "caballeria_organizada",
    "nombre": "Jinetes escitas",
    "edificio": "caballerizas",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaBronce": 1,
      "armaduraBasica": 1
    },
    "caballos": 1,
    "poderBase": 8,
    "velocidad": 28,
    "escalon": 3,
    "unidadesPorDefecto": 18
  },
  {
    "id": "hoplitas_ciudadanos",
    "tecnologia": "ciudadania_militar",
    "nombre": "Hoplitas ciudadanos",
    "edificio": "barracon",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaHierro": 1,
      "armaduraBronce": 1
    },
    "poderBase": 10,
    "velocidad": 16,
    "escalon": 3,
    "unidadesPorDefecto": 18
  },
  {
    "id": "caballeria_asiria",
    "tecnologia": "caballeria_organizada",
    "nombre": "Caballería asiria",
    "edificio": "caballerizas",
    "nivelRequerido": 2,
    "costoEquipo": {
      "armaHierro": 1,
      "armaduraBronce": 1
    },
    "caballos": 1,
    "poderBase": 10,
    "velocidad": 28,
    "escalon": 3,
    "unidadesPorDefecto": 18
  },
  {
    "id": "arqueros_escitas",
    "tecnologia": "arqueria_especializada",
    "nombre": "Arqueros escitas",
    "edificio": "galeriaDeTiro",
    "nivelRequerido": 3,
    "costoEquipo": {
      "armaHierro": 1,
      "armaduraBasica": 1
    },
    "poderBase": 17,
    "velocidad": 16,
    "escalon": 5,
    "unidadesPorDefecto": 12
  },
  {
    "id": "espartiatas",
    "tecnologia": "falange_hoplita",
    "nombre": "Espartiatas",
    "edificio": "barracon",
    "nivelRequerido": 3,
    "costoEquipo": {
      "armaHierroCalidad": 1,
      "armaduraBronceCalidad": 1
    },
    "poderBase": 17,
    "velocidad": 12,
    "escalon": 4,
    "unidadesPorDefecto": 15
  }
]
```
<!-- /parametros -->

## 13. Invariantes

| Invariante | Test que lo vigila |
|---|---|
| Un héroe por jugador y partida, dentro del campamento que elige y como residente; nace con nivel 1, sin puntos, 500 de bronce y el loadout «Default» vacío y activo | `session/__tests__/comandosHeroe.test.ts` |
| Repartir puntos: +1 por punto, sin pasar de los que hay, sin negativos y sin pasar del tope | `comandosHeroe.test.ts` |
| Loadouts: nuevo e inactivo; activar desactiva los demás; reescribir por id no crea otro; rechaza escuadras ajenas, pasarse de Liderazgo, perks que no tiene y ids que no existen | `comandosHeroe.test.ts` |
| Guarnición: asigna dentro del cupo, rechaza pasarse, retirar la devuelve; sin Barracón ni Galería no hay cupo y quien no reside no tiene guarnición; mudarse la suelta | `comandosHeroe.test.ts`, `engine/__tests__/guarnicion.test.ts` |
| Cupo: Caballerizas no dan nada en el nivel 1; suma edificios, recinto completo y «Levas de guarnición»; sin Barracón ni Galería no hay guarnición aunque haya muralla | `guarnicion.test.ts` |
| Costes de Liderazgo por escalón (crecientes, independientes del poder, 0 para tropa desconocida); mezclas máximas; el límite es de salida; sin héroe usa el base; lo libre nunca es negativo | `engine/__tests__/liderazgo.test.ts` |
| Una escuadra reservada para una batalla no se repone | `tropas.test.ts` |
| Una escuadra reservada no entra ni sale de la guarnición, pero sí cabe en un loadout | `comandosHeroe.test.ts` |
| Dos héroes reclutan la misma tropa en la misma plaza → dos escuadras; reclutar de nuevo repone hasta el tope sin crear otra; al tope se rechaza | `engine/__tests__/tropas.test.ts` |
| Otra Facción no recluta; un ciudadano que no reside solo repone su columna a la puerta; reclutar no exige General; sin la tecnología no se recluta | `tropas.test.ts` |
| La clase de población sale del escalón; una tropa de nobleza sin nobles no se recluta | `tropas.test.ts` |
| Oro: la milicia del Centro Urbano no cuesta; el resto cuesta oro por escalón y soldado; la caballería paga 5 por caballo; el coste es creciente y positivo | `engine/__tests__/reclutamientoOro.test.ts` |
| Hambre: sube la moral con trigo, cae sin él, nunca se consume más del disponible; a moral 0 hay deserción narrada; una escuadra aniquilada conserva su identidad | `engine/__tests__/avanzarRacion.test.ts`, `eventosDominioTropas.test.ts` |

**Sin test propio:** la conversión de vistas (`conTropa`/`sinTropa`/`conEscolta`), el orden de las escuadras (`ordenarEscuadras`), `herir`/`estaHerido` fuera de los combates, la chatarra, la reserva de trigo al reclutar y el veto en `puedeReclutarEn`.

## 14. Casos límite y decisiones que parecen raras

- **El nivel del héroe no sube y los puntos nunca llegan**: es una decisión expresa (espera la curva de Conquest, CQ-001), no un olvido. La XP del héroe sí se acumula.
- **`liderazgoBase` es por héroe pero nada lo cambia**: «el techo con equipo» queda para cuando exista la artesanía de armaduras.
- **Reclutar paga del almacén de la plaza, no del héroe**: el oro-recurso y el equipo son de la plaza; las monedas del héroe son otra economía.
- **Una escuadra mermada paga el Liderazgo completo** (el coste mira el tipo, no la cantidad): anotado como punto abierto en el diseño del movimiento. Es lo que impide sacar a campaña élite «en dosis pequeñas».
- **«Una por tropa» es por héroe, no por plaza**: dos héroes de la misma plaza tienen cada uno la suya. La tropa prestada de un campamento no cuenta.
- **Se repone solo donde está la escuadra**: en su campamento si reside aquí, o en su columna si está a la puerta. Una escuadra en una escolta o fuera del mundo no se repone.
- **Reponer no cura moral ni cambia nivel**: solo añade soldados; la experiencia de la escuadra se «diluye» sin que el motor la corrija.
- **El pool de reclutamiento no es la población total** sino lo que no hace falta para producir: un reclutamiento puede fallar con población de sobra si toda ella trabaja.
- **La tropa del Centro Urbano es gratis en oro**: solo cuesta su equipo; el resto de edificios añaden oro por escalón y por caballo.
- **Sin loadout activo no defiende en persona**: borrar el activo no activa otro. La guarnición sigue defendiendo.
- **El candado `reservaBatalla`** (decidido el 2026-10-12) impide abrir otra batalla con esa escuadra, **reponerla** y **entrar o salir de la guarnición** mientras dura la batalla. **No** afecta a los loadouts: son selecciones, no mueven nada, y la misma escuadra puede estar en varios.
- **`guardarLoadout` no mira dónde están las escuadras**, ni si están heridas o prestadas; solo que sean suyas y quepan.
- **`asignarGuarnicion` toma la primera plaza donde el héroe es residente**; un residente de un campamento de mercenarios no tiene guarnición (rechaza «no resides en ningún asentamiento»).
- **Una escuadra sin héroe es un error, no un caso**: `conEscuadrones` lanza si la escuadra apunta a un héroe que no existe.

## 15. Divergencias

Ninguna abierta.
