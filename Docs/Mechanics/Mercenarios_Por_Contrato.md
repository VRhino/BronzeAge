# Mercenarios por contrato

> **Implementado en Rust el 2026-11-01 (entrada 48).** Reglas puras en el crate `crates/mercenarios` (`tipos.rs`, `datos.rs`, `contrato.rs`, `orden.rs`, `progresion.rs`); la marca de
> ciudadanía en `crates/politica`; el mundo en `crates/nucleo/src/nucleo/contratos/` (`firma`, `mando`, `fin`, `guardas`, `tropa`, `renombre`); lo que ve cada uno en
> `crates/nucleo/src/proyeccion/mercenarios.rs` y `audiencia.rs`. Foto **BAN54** (hoy BAN60; las fotos anteriores a BAN60 ya no se leen). El canon está en [`Docs/Game/7_Bots_Mercenarios.md`](../Game/7_Bots_Mercenarios.md).
> Todas las cifras son borradores que se calibran con bots (fase 7).

## 1. Qué es

Un héroe **bot** que reside en un campamento de mercenarios se ofrece en el tablón de ese campamento, y un cargo de una Facción **humana** lo contrata como **ciudadano con marca** de
esa Facción: para un trabajo concreto (**temporal**) o para formar parte de su ejército (**permanente**). El bot sigue fuera del motor y juega por el contrato 05 como cualquier
jugador: el motor guarda el contrato, cobra la tarifa, aplica las guardas, da permisos con las órdenes y lleva la progresión, pero no comprueba si el bot cumple bien (para eso
están la valoración y el historial, §8.2).

Fuera de esto, y decidido por el autor: alquilar escuadras sin héroe, que un humano se ofrezca, el mercenario como héroe personal de otro. **Las Facciones de bots no contratan**, y el
contrato es la única vía por la que un bot entra en una Facción humana (`SolicitarIngreso` se lo rechaza, `Diplomacia_Facciones.md` §3.2). La distinción entre bots libres y bots
mercenarios es del cerebro de bots, no del motor: el motor solo sabe si un héroe es bot (`Controlador::Bot`) y si una Facción es de bots (su Rey lo es).

- **Verificado contra:** la rama del hilo de mercenarios, 2026-11-01.
- **Pruebas:** `crates/nucleo/tests/mercenarios_{firma,guardas,mando,fin,tropa,renombre,vistas,foto}.rs` y las internas de cada crate.

## 2. Cómo pertenece a la Facción

Todo mercenario contratado es **ciudadano** de la Facción con una marca, `temporal` o `permanente` (`politica::Marca`, en `Faccion::marcas`). Así su Facción
(`HeroeNucleo::faccion`) es la de quien le paga para todo lo que ya depende de ella: encuentros, bando en batalla, puertas, convocatorias, visión y misiones. La marca solo añade
guardas (`Diplomacia_Facciones.md`):

- nunca es Rey: la sucesión y el caso «Facción sin nadie» lo saltan;
- no tiene cargos: ni Embajador ni ninguno local (`mercenario.marcado`);
- no deja la Facción por su cuenta (`DejarFaccion`: `mercenario.marcado`); sale cuando acaba el contrato (§7);
- el directorio de Facciones cuenta solo a los ciudadanos **sin** marca (`sin_marca`), y una Facción cuyos ciudadanos sin marca se van todos pierde todos sus contratos (§7).

El temporal **sigue residiendo en su campamento** y conserva lo que tiene allí; el permanente reside en la plaza que elige quien firma (§5).

## 3. Quién se ofrece y quién contrata

### 3.1 Ofrecerse (`ofrecerse_como_mercenario`, `retirar_oferta`)

Se ofrece un héroe **bot** (`mercenario.solo_bots`), **sin Facción** (`faccion.ya_pertenece`), que **reside en un campamento** (`mercenarios.no_resides`) y está **dentro** de él
(`campamento.fuera_de_la_puerta`). Entra en el tablón de ese campamento; si ya estaba, no cambia nada. La primera vez nace su ficha y recibe su **leva propia** (§8.3). Mientras está
ofrecido no crea Facción, no pide ingreso y no funda (`faccion.mercenario_ofrecido`). Sale del tablón al retirarse, al ser contratado o al mudarse de residencia. Hechos
`mercenarios.ofrecido` y `mercenarios.oferta_retirada`, al propio héroe.
### 3.2 Quién firma qué

El firmante no elige con qué cargo firma: el motor toma el de más rango de los suyos que pueda firmar ese trabajo sobre ese objetivo (Rey > Embajador > General y Gobernador >
Maestro de Caravanas). «Su plaza» es aquella de la que cuelga el objetivo: la plaza misma, la madre del pueblo o de la avanzada, el origen de la caravana. «Su zona» de bandidos es la
zona de influencia de su plaza (`Nucleo::en_zona`). Un cargo local cuenta solo si el héroe reside en esa plaza y es su titular.

| Trabajo | Rey, Embajador | Gobernador | General | Maestro de Caravanas |
|---|---|---|---|---|
| Defender plaza, pueblo o avanzada | todas | su plaza | su plaza | no |
| Atacar un objetivo | todos | bandidos de su zona | bandidos de su zona y cualquier objetivo de una Facción en guerra con la suya | no |
| Escoltar | todas | su plaza | no | su plaza |
| Acompañar | cualquiera | no | la convocatoria o el ejército que encabeza él | no |
| Permanente | sí | no | no | no |

El Tesorero no contrata. El objetivo ha de ser de una clase que admite el trabajo (`TRABAJOS_MERCENARIOS`) y, además, de la propia Facción (defender, escoltar, acompañar, permanente:
su Facción) o bandidos u objetivo de una Facción en guerra (atacar).

### 3.3 Condiciones, en orden

1. El firmante tiene Facción (`mercenario.sin_alcance`) y no es de bots (`mercenario.faccion_de_bots`).
2. El mercenario está en un tablón (`mercenario.no_ofrecido`) y no tiene Facción (`faccion.ya_pertenece`).
3. **En persona**: el firmante está en ese campamento, dentro o con su columna a `radioPuerta` (`campamento.fuera_de_la_puerta`), y tiene allí con qué pagar (§5.1): el almacén
   personal solo en el campamento donde reside (`almacen_personal.fuera_de_tu_residencia`) y el carro, el que lleva allí (`heroe.sin_columna` si no trae ninguno). No hace falta estar
   dentro de ninguna plaza. **O desde la taberna de una plaza** (siguiente nota).

> **Implementado en Rust (2026-11-08, tanda 69):** si el firmante no está en el campamento, puede contratar desde **dentro de una plaza de su Facción con Taberna activa** (`Nucleo::bolsa_de_firma`, `nucleo/contratos/firma.rs`), sin ir al campamento, siempre que el campamento del mercenario caiga en la **región del relieve de la plaza** (`Nucleo::campamentos_de_la_region`; si no, `mercenario.fuera_de_la_region`). Fuera del campamento y sin esa plaza, sigue siendo `campamento.fuera_de_la_puerta`. Desde la plaza solo se paga con el **almacén de la plaza** (`pagar_con` = `almacenPlaza`, y es lo que se toma si no lleva `pagar_con`), con el **fondo de la plaza** (`fondoPlaza`, 2026-11-12, Forja_En_Plazas §6; §5.1) o con **monedas**; con `almacenPersonal` o `carro`, `campamento.fuera_de_la_puerta`. Con el almacén o el fondo de la plaza firma solo el Rey, el Embajador o el Gobernador (la misma condición que la intel de la taberna; otro cargo, `no_autorizado`); con monedas firma el cargo que alcance el trabajo, como siempre (§3.2). El resto de condiciones (de 4 a 7) son las de siempre. Prorrogar sigue siendo en persona: `almacenPlaza` o `fondoPlaza` en `prorrogar_contrato` es `orden.forma_invalida`. Ver el tablón de la región es gratis (§10).
4. No hay espera de recontrato de esa Facción con él (`mercenario.recontrato_en_espera`, `esperaRecontratoHoras` tras un despido).
5. El plazo es uno de la tabla en el temporal y ninguno en el permanente (`mercenario.plazo_invalido`); el permanente lleva una plaza de residencia de la Facción y el temporal ninguna
   (`mercenario.objetivo_invalido`).
6. El objetivo existe y vale para el trabajo (`mercenario.objetivo_invalido`), y algún cargo del firmante alcanza (`mercenario.sin_alcance`).
7. Lo que paga (el carro o el almacén personal, o las monedas del héroe) tiene la tarifa (`mercenarios.sin_oro`; con monedas, `heroe.sin_monedas`).

## 4. Los trabajos

Tabla de datos que el motor lee (`TRABAJOS_MERCENARIOS`, §17): cada trabajo con las clases de objetivo que admite, su orden inicial y las órdenes que admite después (§6.4). Un
id que no es un `Trabajo` del motor hace fallar la carga; quién firma qué va en código, porque depende del cargo (§3.2).

| Trabajo | Tipo | Objetivo |
|---|---|---|
| `defender` | temporal | una plaza, un pueblo o una avanzada de la Facción |
| `escoltar` | temporal | una caravana de la Facción (caravana, carro de acarreo, encargo o ruta) |
| `atacar` | temporal | un campamento de bandidos, o un ejército, una caravana, una plaza, un pueblo o una avanzada de una Facción en guerra con la suya |
| `acompanar` | temporal | una convocatoria o un ejército de la Facción |
| `permanente` | permanente | la Facción |

## 5. Temporal y permanente

| | Temporal | Permanente |
|---|---|---|
| Pago | por adelantado al firmar; no se devuelve nunca | pago único al firmar (`permanentePct`, caro a propósito); no se devuelve |
| Residencia | sigue en su campamento | la plaza que elige quien firma, desde la firma y sin cooldown; después solo cambia con `mudar` (§6.2) |
| Duración | uno de los plazos de la tabla, prorrogable | hasta el despido |
| Órdenes | las que admite su trabajo (§6.4) | todas |
| Reclutar | en su campamento, como residente | en su plaza, como cualquier residente |
| Quién lo firma | Rey, Embajador o el cargo de la plaza (§3.2) | Rey o Embajador |

**El contrato es de la Facción**, o de la plaza si lo firmó un cargo local: si quien firmó pierde el cargo o se va, lo manda quien ocupe ahora ese cargo.

### 5.1 La tarifa y el pago

Valor del mercenario = `oroPorNivelHeroe` × su nivel + lo que costaría hoy reclutar en su campamento cada escuadra suya **no prestada** con los soldados que tiene (precio de
`Campamentos_Mercenarios.md` §5.2, sin recargo de reputación ni descuento). **Tarifa** = ⌈valor × `tarifaPct` del plazo (o `permanentePct`) × `tarifaPct` del rango × `factorOroPct`⌉, en oro
entero. La calcula el motor; el bot no fija precio. El tablón la muestra para cada plazo y para el permanente con el valor de ese momento.

> **Implementado en Rust (2026-11-08, tanda 69, T2):** la tarifa baja a un cuarto (`MERCENARIOS_CONTRATO.factorOroPct`, 25: se multiplica por 25/100 antes de redondear hacia arriba a oro entero; `mercenarios::contrato::tarifa`) y se puede pagar en **monedas del héroe**: `tarifa_en_monedas` = el oro ya reducido × `monedasPorOro` (100) monedas, y el tablón (`TarifaMercenario`) lleva las dos, `oro` y `monedas`, de cada plazo y del permanente. Cifras provisionales, a calibrar con el batch.

**De dónde sale el pago** (orden con `pagar_con` = `almacenPersonal` | `carro` | `monedas` | `almacenPlaza` | `fondoPlaza`, opcional; desde el 2026-11-03 también desde dentro del campamento donde reside, como en su mercado):

- Sin `pagar_con`: el carro de la columna que lleva si va en una (a la puerta, o dentro de un campamento de visita) y, si no (dentro del campamento donde reside), su almacén personal.
- `almacenPersonal`: solo en el campamento donde reside, dentro o con su columna a la puerta (`almacen_personal.fuera_de_tu_residencia` en otro).
- `carro`: el que lleva allí: el de su columna a la puerta o dentro; en el campamento donde reside sin columna, el que trajo al entrar (y si no trajo oro, `mercenarios.sin_oro`).
- `almacenPlaza` (2026-11-08, tanda 69, §8): el oro del **almacén de la plaza** desde cuya taberna se contrata (§3.3), a la tarifa en oro; no se puede en el campamento ni al prorrogar (`orden.forma_invalida`). El oro entra en el almacén personal del mercenario como el de cualquier otra bolsa; si el almacén de la plaza no llega, `mercenarios.sin_oro`.
- `fondoPlaza` (2026-11-12, Forja_En_Plazas §6): el **fondo de monedas de la plaza** desde cuya taberna se contrata, donde entra el impuesto de su forja (`Asentamiento.forja.fondo`). Se paga como `almacenPlaza`: desde dentro de la Taberna activa de una plaza de su Facción, con el campamento en su región (§3.3), y firma el Rey, el Embajador o el Gobernador, dentro (el Tesorero, que fija la forja, no firma: otro cargo, `no_autorizado`); en el campamento y al prorrogar, `orden.forma_invalida`. Paga la **tarifa en monedas**, como `monedas`, pero del fondo; si no llega, `mercenarios.sin_fondo`. Las monedas pasan a las del mercenario, y el hecho `mercenarios.contratado` las lleva en `monedas` (`oro` 0). Una plaza sin Taberna acumula el fondo, pero no lo gasta hasta tenerla.
- `monedas` (2026-11-08, tanda 69): las monedas del héroe que firma (`Heroe.monedas`), a la tarifa en monedas, **en cualquier sitio** donde pueda firmar o prorrogar (no pide carro ni almacén personal). Si no le alcanzan, `heroe.sin_monedas`. Las monedas pagadas **pasan a las del mercenario** (no desaparecen, a diferencia del oro que no cabe). Vale al contratar y al prorrogar; los hechos `mercenarios.contratado` y `mercenarios.prorrogado` llevan `oro` y `monedas` (uno de los dos es 0).

El oro entra en el **almacén personal del mercenario** hasta llenarlo (`capacidadAlmacenPersonal`); lo que no cabe desaparece. Nunca bloquea una firma
y nunca se devuelve. El temporal lo gasta como residente de su campamento (reclutar y reponer, víveres, compras); el permanente, de visita con el carro de su columna o cuando vuelve a
un campamento.

### 5.2 Lo que deja la firma

El mercenario sale del tablón, recibe la ciudadanía con su marca y, si es permanente, pasa a **residir en la plaza elegida**, sin cooldown. El contrato nace con la **orden inicial**
de su trabajo (§6.4), dada por quien firmó y **sin cargo**, con su renombre de plazo ya apuntado en el contrato (§8.2) y sus citas en la cola. Su ficha suma un contrato firmado.
Hecho `mercenarios.contratado` a la Facción y al mercenario (con el oro y el campamento).

## 6. Las órdenes de mando

Una orden le dice al bot qué hacer (le llega por su `HEROE`) y le da permiso en el motor para lo que sin ella se le rechaza. El motor no vigila cómo se mueve ni si cumple; moverse es
libre siempre.

### 6.1 Las cuatro órdenes y el permiso que dan

| Orden | Objetivo | Qué le deja hacer | Plazo |
|---|---|---|---|
| `guardar` | plaza, pueblo, avanzada o caravana de su Facción | guarnecer esa avanzada **sin residir en su madre**; adjuntarse a esa caravana o cederle escuadras de escolta; entrar en una batalla en el bando que defiende ese objetivo | no |
| `ir_a` | un punto | nada nuevo | `plazoOrdenHoras` |
| `atacar` | bandidos, o ejército, caravana, plaza, pueblo o avanzada de una Facción en guerra con la suya | `Atacar` y `Perseguir` ese objetivo; entrar en una batalla en el bando que lo ataca; convocar un ejército (§6.6) | `plazoOrdenHoras` |
| `unirse` | convocatoria o ejército de su Facción | unirse a ese y a ningún otro | no |

Sin una orden que le dé permiso, **solo se defiende**: «en defensiva» no es un estado, basta con que el motor rechace `Atacar`, `Perseguir` y unirse sin ella
(`mercenario.sin_orden`), porque nunca abre combates solo.

### 6.2 Acciones de mando

Se aplican al momento y no quedan como orden.

**Mudar a un permanente** (`mudar_mercenario`). El Rey o el Embajador (`mercenario.sin_alcance`) cambian a un **permanente** (`mercenario.tipo_de_contrato`) a otra plaza de la Facción (`mercenario.objetivo_invalido`), a
distancia y sin cooldown. Si guardaba la plaza vieja, su `guardar` pasa a la nueva. Hecho `mercenarios.mudado`, al mercenario y a quien lo mudó.

**Despedir** (`despedir_mercenario`): acaba el contrato (§7). Lo hace quien podría firmarlo hoy, a distancia.

**Prorrogar** (`prorrogar_contrato`). Alarga un **temporal** (`mercenario.tipo_de_contrato`) vigente de la Facción y sin fin aplazado (`mercenario.sin_contrato`) otro plazo de la tabla, sin cortes. Lo hace quien podría
firmarlo hoy, **en persona**: en el campamento base del mercenario o con su columna a `radioPuerta` de la de él (`campamento.fuera_de_la_puerta`), pagando la tarifa de ese plazo
con el rango de hoy. En el campamento base paga como al contratar (§5.1, `pagar_con`); junto a la columna del mercenario, con el carro de la suya (`heroe.sin_columna` si no va en
una; `almacen_personal.fuera_de_tu_residencia` si pide el almacén personal). El renombre de ese plazo se suma al del contrato. Hecho `mercenarios.prorrogado` a la Facción y al mercenario.

### 6.3 El ciclo de una orden

Cada contrato tiene como mucho una orden vigente. Acaba de tres modos, y en los tres vuelve la **inicial** del contrato si aún vale:

- **Cumplida**: un `atacar` es **un solo ataque**. Se cumple al resolverse la primera batalla en la que él ataca ese objetivo, gane o pierda (`tras_batalla`, llamada en los ocho
  caminos que resuelven una batalla). Si la inicial es esa misma orden (el trabajo «Atacar un objetivo»), no vuelve: el mercenario queda sin orden hasta que le den otra.
- **Caducada** (`mercenarios.orden_caducada` con su motivo): `plazo` (vence `ir_a` o `atacar` sin haber atacado), `objetivo` (desapareció o un `guardar`/`unirse` dejó de ser de la
  Facción), `paz` (la Facción ya no está en guerra con la que tenía el objetivo cuando se dio el `atacar`) o `salida` (estuvo dentro de lo que acompañaba y ya no lo está). Se mira al
  cerrar cada versión, solo para las órdenes cuyo objetivo o Facción tocó la transición (`vigilar_ordenes`).
- **Sustituida** por otra válida.

Una convocatoria que **parte** convierte en ejército lo que se acompañaba: los contratos de acompañarla y las órdenes `unirse` a ella pasan al ejército nuevo, con su `unido`. Solo
caducan si la convocatoria se cancela. Hechos `mercenarios.orden_cumplida` y `mercenarios.orden_caducada`, al mercenario y a quien la dio.

### 6.4 Qué admite cada trabajo

La tabla `TRABAJOS_MERCENARIOS` (§17) da a cada trabajo sus clases de objetivo, su orden inicial y las órdenes que admite después (`mercenario.orden_fuera_de_contrato` si no):
`guardar_objetivo` es `guardar` del objetivo del contrato y `atacar_en_radio`, `atacar` de algo que esté a ≤ `radioDefensa` del objetivo del contrato. La orden inicial es la de su
fila sobre el objetivo del contrato, salvo en el permanente, que es `guardar` de la plaza donde reside.

### 6.5 Quién ordena qué y el rango

| Cargo | A quién alcanza | Qué órdenes |
|---|---|---|
| Rey, Embajador | a todos los mercenarios de la Facción | todas |
| General de P | contratos de P y permanentes que residen en P | `guardar` de lo que cuelga de P, `atacar`, `unirse` solo a lo que encabeza él, `ir_a` |
| Gobernador de P | contratos de P y permanentes que residen en P | `guardar` de lo que cuelga de P, `atacar` solo bandidos, `ir_a` |
| Maestro de Caravanas de P | escoltas firmados en P | `guardar` de caravanas de P, `ir_a` |

Fuera de su alcance, `mercenario.sin_alcance`. Una orden la sustituye quien tiene **igual o más rango** que el cargo con el que se dio (`mercenario.orden_de_mas_rango`); General y
Gobernador tienen el mismo rango, así que manda la última. La orden inicial **no tiene rango**: la sustituye cualquier cargo que alcance al mercenario. La orden de quien deja el
cargo sigue vigente con el cargo con el que se dio hasta que acaba.

**Ordenar** (`ordenar_mercenario`). Una orden lleva **de 1 a 16 mercenarios** sin repetir (`mercenario.lista_invalida`) y es la misma para todos; se da a distancia. Antes de tocar nada se comprueba cada uno: contrato
vigente con la Facción de quien ordena (`mercenario.sin_contrato`), objetivo que existe y vale para la orden (`mercenario.objetivo_invalido`), alcance, trabajo que la admite y rango.
Si uno falla no entra ninguna y el rechazo lo nombra. Quien ya tiene esa misma orden (tipo y objetivo) se queda como está; si la tienen todos, no cambia nada. Un hecho
`mercenarios.ordenado` por mercenario, que oyen él y quien la dio.

### 6.6 Lo que un marcado no hace

Las guardas viven en una sola función antes del despacho (`rechazo_por_contrato`), con un `match` exhaustivo sobre `Comando`: toda orden nueva obliga a decidir. La administración
no pasa por ellas.

| Orden | Para un marcado |
|---|---|
| `CrearFaccion`, `SolicitarIngreso`, `Fundar` | rechazadas también al **ofrecido** (`faccion.mercenario_ofrecido`) |
| `DeclararGuerra`, `Atacar` con `declara_guerra` | nunca (`mercenario.prohibido`): solo ataca con la guerra ya declarada |
| `AceptarMision` del tablón de la Facción o de una plaza | nunca (`mercenario.prohibido`) |
| `Atacar`, `Perseguir` | con `atacar` de ese objetivo (`mercenario.sin_orden`) |
| `UnirseAEjercito`, `UnirseEnCampo`, `UnirseAConvocatoria` | con `unirse` de ese ejército o esa convocatoria |
| `UnirseABatalla` | con `guardar` del objetivo, en el bando que lo defiende; con `atacar`, en el que lo ataca |
| `ConvocarEjercito` | con una orden `atacar` vigente (`mercenario.sin_orden`) |
| `AdjuntarCaravana`, `AsignarEscolta`, `GuarnecerAvanzada` | con `guardar` de esa caravana o esa avanzada |
| `CederLiderazgo` a un marcado | nunca (`mercenario.marcado`) |
| `ReclutarTropa` en una plaza con el almacén cerrado a ciudadanos | nunca (`almacen.cerrado`) |

**La convocatoria de un mercenario** (decisión del 2026-11-01): un marcado con orden `atacar` puede convocar su ejército, pero a esa convocatoria solo se unen mercenarios de su
Facción con la orden `unirse` a ella; un ciudadano sin marca recibe `mercenario.convocatoria_de_mercenarios`. Si su contrato acaba mientras la lidera, la convocatoria se cancela.

Lo demás lo hace como cualquier ciudadano de su Facción: moverse, entrar en sus plazas, guarnecer pueblos, comerciar, reclutar en su residencia, usar el almacén de la plaza.
**Líder de ejército**: la sucesión automática (`sucesor_automatico`) prefiere a un conectado sin marca, luego a cualquiera sin marca, luego a un conectado y por último a cualquiera,
así que un marcado solo manda si en la columna no queda nadie sin marca.

## 7. El ciclo del contrato

Acaba por **plazo** (cita `Vence`), por **despido** o por lo que le pasa al mundo, que se mira al cerrar cada versión solo si la transición tocó alguna Facción, plaza o batalla
(`vigilar_fines`). De más grave a menos:

| Motivo | Cuándo |
|---|---|
| `faccion_perdida` | ya no es ciudadano marcado de la Facción del contrato (la administración, una anexión, una fusión) o ella ya no existe |
| `plaza_perdida` | el contrato es de una plaza y la plaza cae o cambia de dueño |
| `sin_ciudadanos` | en la Facción no queda ningún ciudadano sin marca |
| `plazo` | vence el temporal |
| `despido` | `despedir_mercenario`: quien podría firmar hoy ese contrato, a distancia (`mercenario.sin_contrato`, `mercenario.sin_alcance`) |

**En mitad de una batalla**, el fin se **aplaza** hasta que la batalla acaba; el primero que llega es el que se cumple. Despedir a quien ya tiene un fin aplazado no cambia nada.

Al acabar: se separa de su convocatoria (si la lidera, se cancela) y de su ejército donde esté, cediendo antes el mando si es su Líder; sale de la Facción en el mismo instante y deja
lo que la ciudadanía le daba (sus guarniciones de pueblo); el **permanente** vuelve a residir en su **campamento base**, sin cooldown, y el temporal sigue en el suyo; quien está
dentro de un lugar que ya no le admite sale al mapa con su carro. **Nunca se devuelve nada.** Si el fin es un despido, empieza la espera de recontrato en esa Facción. Se apunta en su
ficha (terminado o despido) y lo que toque de renombre (§8.2), y queda un `Terminado` que se puede valorar durante `valoracionHoras`. Hecho `mercenarios.terminado` con su motivo, a la
Facción y al mercenario.

Un **permanente sin plaza de su Facción donde residir** (la perdió y la Facción no tiene otra, o la administración lo mudó) vuelve a su campamento base **sin perder el contrato**. Si
la Facción tiene otras plazas, la reubicación de siempre ya lo dejó en una.

## 8. Progresión

### 8.1 Por qué

Cada campamento tiene un solo edificio militar y solo ofrece las tropas cuya tecnología ha llegado a él (`Campamentos_Mercenarios.md` §5.1); la leva prestada no gana experiencia y se
pierde al dejar de residir. Para que un mercenario sea útil desde el primer contrato y crezca, tiene una progresión propia: renombre (§8.2), rangos con leva propia, premios y
licencia de reclutar (§8.3) y medallas (§9).

### 8.2 Renombre y valoración

El renombre es la experiencia propia del mercenario, aparte del nivel del héroe y de la experiencia de cada escuadra. Solo se gana bajo contrato:

| Qué hace | Renombre |
|---|---|
| Termina un temporal sin despido | el del plazo (`plazos[].renombre`), sumado por cada plazo firmado o prorrogado |
| Gana el ataque de su orden `atacar` | `renombre.atacar` según el objetivo (bandidos, por nivel del campamento) |
| Defiende el objetivo de su `guardar` en una batalla que el atacante no gana (también el acoso de bandidos a la caravana que guarda) | `renombre.defensaGanada` |
| Llega a destino una caravana que guardaba yendo en su escolta cedida (entrega, acarreo o la parada de destino de un encargo; volver a casa no cuenta) | `renombre.escoltaLlegada` |
| Cada grado nuevo de una medalla | `renombre.gradoMedalla` |

**Cuándo se apunta.** En el **temporal**, lo que gana se guarda en el contrato y se apunta en la ficha **al acabar**, salvo con despido, que lo pierde. En el **permanente**, que solo
acaba por despido, se apunta en el momento. El renombre de las medallas se apunta siempre en el momento. El renombre nunca baja de 0, el rango alcanzado no baja y los premios ganados
se quedan.

**Valoración** (`valorar_mercenario`). Mientras dura `valoracionHoras` tras el fin, quien firmó el contrato o el Rey de esa Facción (`mercenario.sin_alcance`) lo valora una vez (`mercenario.sin_valoracion` si no hay
contrato acabado de ese mercenario con su Facción, ya se valoró o pasó el plazo):

- **Buena**: suma lo apuntado de ese contrato × (`buenaPct` − 100) / 100, cuenta una valoración buena en el historial y la medalla Bien valorado.
- **Mala**: quita lo apuntado × (100 − `malaPct`) / 100 (con `malaPct` 0, todo lo apuntado), cuenta una mala y, si ese contrato contó Palabra cumplida, se la descuenta del contador
  (el grado ganado no baja).
- **Sin valorar**: queda como se apuntó.

Un despido y un permanente no apuntan nada al acabar, así que su valoración solo cuenta para el historial y la medalla. Hecho `mercenarios.valorado` a la Facción y al mercenario, con
quién valoró, si fue buena y el renombre que de verdad sumó o quitó (una mala no quita más del que tiene la ficha).

### 8.3 Rangos, leva propia, premio y licencia

| Rango | id | Renombre | Al llegar | Licencia de reclutar | Tarifa |
|---|---|---|---|---|---|
| 1 | `a_sueldo` | 0 | la leva propia | — | ×1 |
| 2 | `lanza` | 100 | una escuadra de premio | escalón ≤ 2 | ×1,2 |
| 3 | `capitan` | 300 | una escuadra de premio | escalón ≤ 3 | ×1,5 |
| 4 | `veterano` | 700 | una escuadra de premio | escalón ≤ 4 | ×1,8 |
| 5 | `senor_de_la_guerra` | 1500 | una escuadra de premio | escalón ≤ 5 | ×2,2 |

Si un renombre salta varios rangos de golpe, da un premio y un hecho `mercenarios.rango_subido` por cada uno, en orden. **El Liderazgo no cambia con el rango** (decisión del autor):
es igual para todos los héroes.

- **Leva propia.** La primera vez que se ofrece recibe gratis, una sola vez, una escuadra completa y **suya** de cada tropa de `levaPropia`, en su campamento; si ya tenía una de esas
  tropas, se la completan (una escuadra por tropa en toda la partida). No es prestada: gana experiencia y no se pierde al mudarse. La **repone pagando** en su campamento aunque el
  campamento no tenga Centro Urbano (solo las tropas de la leva que ya tiene). Hecho `mercenarios.leva_recibida`. Encarece su tarifa, porque es tropa suya.
- **Escuadra de premio** (`reclamar_premio_de_rango`). En su **campamento base** (el de su contrato o, sin contrato, aquel donde reside: `mercenarios.no_resides` si ninguno), dentro o
  con la columna a la puerta (`campamento.fuera_de_la_puerta`), cambia un premio (`mercenario.sin_premio`) por una escuadra completa y suya de una tropa de su licencia
  (`mercenario.fuera_de_licencia`). Si ya tiene esa tropa se la reponen entera, donde esté; si ya está completa, el premio no se gasta (`mercenarios.al_tope`); si no la tiene, nace en
  el campamento. La orden devuelve la escuadra nueva. Hecho `mercenarios.premio_reclamado`.
- **Licencia de reclutar.** En su campamento recluta, además de lo que el campamento ofrece a todos, las tropas **de su edificio militar** hasta el escalón de su rango **en cuanto
  alguien en el mundo ha descubierto su tecnología** (de arranque o con `campamento_libre_en`), sin esperar las 72 h ni a la mitad de las Facciones. No salta Eras y no cambia de
  edificio. No se guarda nada: se mira al reclutar.
- **Objetos** de los rangos 4 y 5 (entrada 68, 2026-11-12): al alcanzar el rango 4 o el 5 (en el mismo momento que su escuadra de premio y su `rango_subido`; si salta varios,
  cada uno el suyo), el mercenario recibe además la tabla de su rango de `BOTIN_OBJETOS.rangoMercenario` ([`Objetos_Del_Heroe.md`](Objetos_Del_Heroe.md) §3.7): un plano de tier IV
  (o V en el rango 5), una medalla grande del héroe y un cofre de monturas. Va a su mochila y, si no cabe, a su almacén de objetos; si tampoco, se pierde (D17 y D22), y lo cuenta
  el hecho `heroe.botin` (origen `mercenario`), que oye solo él. Los rangos 2 y 3 no tienen tabla.

## 9. Medallas

Logros con tres grados (bronce, plata, oro) que no se pierden; el grado nunca baja y cada grado nuevo da su renombre la primera vez. Tabla `MEDALLAS_MERCENARIOS` (§17). Hecho
`mercenarios.medalla` con su id y grado.

| Medalla | Cuenta uno cuando |
|---|---|
| Palabra cumplida | acaba por **plazo** un temporal (una mala valoración de ese contrato lo descuenta) |
| Cazador de bandidos | gana un ataque contra un campamento de bandidos, con o sin orden de atacarlo |
| Muralla | defiende el objetivo de su `guardar` en una batalla que el atacante no gana |
| Buen camino | llega a destino una caravana que guardaba, yendo en su escolta cedida (la adjunta a una columna no viaja sola y no «llega») |
| Asaltante | gana el ataque de su orden `atacar` contra un pueblo, una avanzada o una plaza |
| Campaña | se deshace el ejército de su orden `unirse` (se desarma a la puerta de un lugar o se queda sin nadie) o se cancela esa convocatoria (también cuando la cancela el fin del contrato de su Líder mercenario), estando él dentro hasta el final |
| Bien valorado | recibe una valoración buena |
| Leal | sus días seguidos como permanente de la misma Facción, contados a los 7, 30 y 90 días de la firma (citas `Leal`); el contador guarda la racha más larga |

## 10. Qué ve cada uno

**El mercenario no tiene el ámbito `FACCION`**: lo suyo le llega por su `HEROE`.

| Dónde | Qué lleva |
|---|---|
| Su `HEROE` | su contrato (`Heroe.contrato`, con la orden vigente), su ficha entera (con el renombre del siguiente rango, la cuenta de cada medalla y el umbral del grado siguiente, el historial y los premios), la Facción que le paga en su forma pública (nombre y sigilo, sin miembros, contratos, relaciones, propuestas ni tecnología) y lo **de esa Facción** a lo que apuntan su contrato o su orden: la plaza, el pueblo, la avanzada, la caravana, el ejército o la convocatoria. Le llega en cuanto firma o recibe la orden, y sale de su `HEROE` al acabar |
| `FACCION` (los ciudadanos sin marca) | los contratos vigentes de la Facción (`Faccion.contratos`) y la marca de cada miembro; el cargo decide quién ordena, no quién ve |
| El héroe público (`VISTA`) | la marca (`Heroe.contratado`) y la ficha reducida: rango, renombre y grado de cada medalla, nunca el historial ni el contrato |
| `CAMPAMENTO` (desde dentro) | el tablón: cada ofrecido con su ficha entera, su tropa (la suya, no la prestada), si está herido y la tarifa de cada plazo y del permanente |
| `ASENTAMIENTO` (el interior de una plaza con Taberna activa) | `Asentamiento.mercenarios_region`: el tablón de **todos** los campamentos de la región de la plaza juntos, **gratis** y con la forma del anterior; cada oferta lleva el `campamento_id` donde se ofrece (`OfertaMercenario.campamento_id`, campo 7, que también va en el tablón del campamento). Sin Taberna activa el campo va vacío (`proyeccion/interior.rs`, `proyeccion/mercenarios.rs::tablon_de`) |
| Nunca | quién le valoró ni para quién trabajó: el historial da cifras, no nombres |

Lo **enemigo** que ataca le llega solo por sus ojos (su `VISTA`): no ve el interior de la plaza, el carro ni la composición del rival por tener la orden.

**Memoria de mapa.** Su `VISTA` es la personal. Sus ojos alimentan la memoria de la Facción mientras dura el contrato, pero su memoria personal no se transfiere ni se borra al
firmar y sigue creciendo con sus ojos, y al irse no se lleva la de la Facción (excepción a `Niebla_De_Guerra.md` §4.3).

## 11. Otras reglas

- **Misiones y Participación**: lo que hace cuenta para las misiones colectivas de su Facción, de la plaza donde reside y de la plaza de su contrato (sin contar dos veces la misma);
  no avanza las individuales de esos tablones ni las acepta. **No gana Participación** y no cuenta en el reparto de una grupal: su parte es de los demás. Sus misiones personales siguen
  como siempre.
- **Conquistas**: una plaza que toma es de su Facción; como no puede ser Gobernador, queda sin Gobernador hasta que el Rey nombre uno.
- **Guarniciones**: como ciudadano guarnece pueblos y entra en las plazas de su Facción; las avanzadas, con `guardar` de esa avanzada.
- **Almacén de la plaza**: el permanente lo usa como cualquier ciudadano; cerrarlo a los ciudadanos también le corta el reclutamiento en ella.
- **Vetos**: la plaza puede vetar a un permanente que reside en ella; vetarlo le quita esa residencia y vuelve a su campamento base **sin perder el contrato**.
- **Herido**: se le puede contratar herido (el tablón lo muestra) y el plazo corre igual.
- **Leva prestada**: puede pedirla como cualquier residente; no cuenta para la tarifa ni para nada de lo anterior.
- **Muchos permanentes en una plaza** (decisión del autor): no hay tope por Facción y cuentan como residentes para la nobleza; lo que frena es el precio (`permanentePct`).

## 12. El modelo en el motor

`Estado::contratos` (`EstadoContratos`, todo en `BTreeMap`):

| Campo | Qué guarda |
|---|---|
| `ofertas` | por héroe: el campamento de su tablón y desde cuándo |
| `contratos` | por héroe (un contrato por mercenario, sin id propio): el contrato vigente |
| `fichas` | por héroe: la ficha de progresión; nace con su primer ofrecimiento y no se borra |
| `esperas` | por (mercenario, Facción): hasta cuándo no se le puede recontratar tras un despido |
| `terminados` | por (mercenario, Facción): el último contrato acabado mientras se puede valorar |

**Contrato**: mercenario, Facción, ámbito (de la **Facción** si lo firmó el Rey o el Embajador; de la **plaza** si lo firmó un cargo local), trabajo, objetivo, plazo en días
(ninguno en el permanente), quién lo firmó y con qué cargo (`puesto_firma`), cuándo, cuándo vence, campamento base (el de la firma), la orden vigente, el fin aplazado si lo hay y el
renombre que lleva acumulado (§8.2).

**Objetivo**: `Plaza`, `Pueblo`, `Avanzada`, `Caravana` (una sola clase para caravana, carro de acarreo, encargo y ruta: en el motor todo eso es una caravana), `Bandidos`,
`Ejercito`, `Convocatoria`, `Faccion` o `Punto`.

**Orden**: tipo (`guardar`, `ir_a`, `atacar`, `unirse`), objetivo, quién la dio, con qué cargo (`puesto`; ninguno en la orden inicial), cuándo, cuándo vence (`ir_a` y `atacar`), la
Facción contra la que se dio un `atacar` (`contra`) y si el mercenario ya estuvo dentro de lo que acompaña (`unido`, en `unirse`).

**Ficha**: renombre, rango (1 a 5), escuadras de premio sin reclamar, contador y grado de cada medalla, historial (firmados, terminados, despidos, valoraciones buenas y malas) y si
ya recibió su leva propia.

**Terminado**: quién firmó, el renombre que se apuntó al acabar, hasta cuándo se puede valorar, si ya se valoró y si ese contrato contó Palabra cumplida.

Los plazos van en la cola (`Carga::Contrato`): el vencimiento del contrato (`Vence`), el de la orden (`VenceOrden`) y las citas de la medalla Leal (`Leal`). Una cita que quedó vieja
(contrato prorrogado, orden sustituida, contrato acabado) se reconoce porque su instante ya no es el guardado y no hace nada.

## 13. Órdenes, rechazos y hechos

| Orden (contrato 05) | Comando | Quién |
|---|---|---|
| 171 `ofrecerse_como_mercenario` | 198 `OfrecerseComoMercenario` | el bot |
| 172 `retirar_oferta` | 199 `RetirarOferta` | el bot |
| 173 `contratar_mercenario` | 200 `ContratarMercenario` (mercenario, trabajo, objetivo, plazo, residencia); 210 si lleva `pagar_con` (`almacenPersonal`, `carro`, `monedas` o, desde la taberna de una plaza, `almacenPlaza` o `fondoPlaza`) | el cargo que firma |
| 174 `prorrogar_contrato` | 201 `ProrrogarContrato` (mercenario, plazo); 211 si lleva `pagar_con` | quien podría firmarlo |
| 175 `ordenar_mercenario` | 202 `OrdenarMercenario` (mercenarios, tipo, objetivo) | el cargo que alcanza |
| 176 `mudar_mercenario` | 203 `MudarMercenario` (mercenario, plaza) | Rey o Embajador |
| 177 `despedir_mercenario` | 204 `DespedirMercenario` (mercenario) | quien podría firmarlo |
| 178 `valorar_mercenario` | 205 `ValorarMercenario` (mercenario, buena) | quien firmó o el Rey |
| 179 `reclamar_premio_de_rango` | 206 `ReclamarPremioDeRango` (tropa) | el mercenario |

Todas son de la cuenta del héroe. Ordenar, despedir y valorar **no llevan el candado de batalla** (despedir a quien combate se acepta y el fin se aplaza). Ofrecerse y reclamar el
premio devuelven las escuadras que crean. Los rechazos `mercenario.*` están en el catálogo 07.

**Quién oye cada hecho**: `ofrecido`, `oferta_retirada`, `rango_subido`, `medalla`, `premio_reclamado` y `leva_recibida`, solo el mercenario; `ordenado`, `orden_cumplida`,
`orden_caducada` y `mudado`, el mercenario y quien dio la orden; `contratado`, `prorrogado`, `terminado` y `valorado`, la Facción y el mercenario. Nada público salvo la ficha y la
marca.

## 14. Fuera del motor y pendiente

- **Cerebro de bots**: el perfil mercenario (no funda ni crea Facción, se ofrece en su campamento base, viaja al objetivo, cumple la orden vigente, reclama sus premios y vuelve a
  ofrecerse) y las conductas de grupo (compañías con capitán).
- **Clientes**: la pestaña «Mercenarios» de la Facción, contratar desde el tablón, ordenar, prorrogar, despedir y valorar.
- **Calibración con bots**: tarifas y multiplicadores, renombre, umbrales de rangos y medallas, radio de defensa, plazos de orden, de recontrato y de valoración.
- **Pendiente**: el bot no ve más del objetivo enemigo que lo que alcanzan sus ojos.

## 15. Casos límite y decisiones que parecen raras

- **El motor no comprueba que el temporal cumpla**: la valoración y el historial son su reputación.
- **Un ataque, una vez**: la orden `atacar` se cumple aunque pierda; solo el renombre pide ganar. En «Atacar un objetivo», la orden inicial que caduca por plazo sin haber atacado
  vuelve igual, con otro plazo: mientras dura el contrato siempre tiene esa orden hasta que ataca.
- **La orden de un cargo que deja de serlo** sigue vigente hasta que se cumple, caduca o la sustituye alguien con rango.
- **Un campamento no tiene puerta ni vetos**: contratar no depende de relaciones.
- **El pago puede perderse**: lo que no cabe en el almacén personal desaparece (decisión del autor: si en el futuro hay más en qué gastar, el tope no estorba).
- **El fin de un contrato de la administración**: quitarle la ciudadanía a un marcado (`FijarFaccionDeHeroe`) acaba su contrato con `faccion_perdida`. El borrado de cuenta no existe en
  el motor.

## 16. Invariantes

- Un mercenario tiene como mucho un contrato, y quien tiene contrato es ciudadano marcado de la Facción del contrato (o su fin está aplazado y se cumple al acabar su batalla).
- Un marcado nunca es Rey, Embajador ni titular de un cargo local, y no está en un tablón.
- La orden vigente es de una clase que admite el trabajo del contrato; la inicial no lleva cargo.
- El renombre y los contadores de medalla nunca bajan de 0; el rango, los grados y los premios ganados nunca bajan.
- Ningún recorrido de los contratos ocurre sin que la transición haya tocado lo que puede cambiarlos (`vigilar_ordenes`, `vigilar_fines`); la audiencia de una entidad mira solo los
  marcados de la Facción dueña.
- Pruebas: `crates/nucleo/tests/mercenarios_*.rs` (sobre el artefacto de mundo real) y las internas de `crates/mercenarios`.

## 17. Parámetros

Las tarifas son porcentajes enteros del valor del mercenario (20 de oro por nivel del héroe más lo que costaría hoy reclutar su tropa en su campamento); `permanentePct` es caro a propósito
(decisión del autor del 2026-11-01). El radio de defensa va en unidades de mapa; los plazos, en horas de mundo. El rango `escalon` 0 quiere decir sin licencia de reclutar.

<!-- parametros:MERCENARIOS_CONTRATO -->
```json
{
  "plazos": [
    { "dias": 1, "tarifaPct": 15, "renombre": 10 },
    { "dias": 3, "tarifaPct": 35, "renombre": 25 },
    { "dias": 7, "tarifaPct": 70, "renombre": 50 }
  ],
  "permanentePct": 500,
  "factorOroPct": 25,
  "monedasPorOro": 100,
  "oroPorNivelHeroe": 20,
  "radioDefensa": 20,
  "plazoOrdenHoras": 24,
  "esperaRecontratoHoras": 72,
  "valoracionHoras": 72,
  "renombre": {
    "atacar": { "bandidosPorNivel": 5, "ejercito": 10, "caravana": 5, "pueblo": 15, "avanzada": 15, "plaza": 30 },
    "defensaGanada": 10,
    "escoltaLlegada": 5,
    "gradoMedalla": [10, 30, 80]
  },
  "valoracion": { "buenaPct": 150, "malaPct": 0 },
  "rangos": [
    { "id": "a_sueldo", "renombre": 0, "escalon": 0, "tarifaPct": 100 },
    { "id": "lanza", "renombre": 100, "escalon": 2, "tarifaPct": 120 },
    { "id": "capitan", "renombre": 300, "escalon": 3, "tarifaPct": 150 },
    { "id": "veterano", "renombre": 700, "escalon": 4, "tarifaPct": 180 },
    { "id": "senor_de_la_guerra", "renombre": 1500, "escalon": 5, "tarifaPct": 220 }
  ],
  "levaPropia": ["milicia_lanceros", "granjeros", "mineros", "lenadores"]
}
```
<!-- /parametros -->

Cada trabajo con las clases de objetivo que admite, su orden inicial y las órdenes que admite después. `guardar_objetivo` es `guardar` del objetivo del contrato; `atacar_en_radio`,
`atacar` de lo que esté a ≤ `radioDefensa` de él. La orden inicial del permanente es `guardar` de su plaza de residencia, no de su objetivo.

<!-- parametros:TRABAJOS_MERCENARIOS -->
```json
[
  { "id": "defender", "objetivos": ["plaza", "pueblo", "avanzada"], "inicial": "guardar", "admite": ["guardar_objetivo", "ir_a", "atacar_en_radio"] },
  { "id": "escoltar", "objetivos": ["caravana"], "inicial": "guardar", "admite": ["guardar_objetivo", "ir_a"] },
  { "id": "atacar", "objetivos": ["bandidos", "ejercito", "caravana", "plaza", "pueblo", "avanzada"], "inicial": "atacar", "admite": ["atacar", "ir_a"] },
  { "id": "acompanar", "objetivos": ["convocatoria", "ejercito"], "inicial": "unirse", "admite": ["unirse", "atacar", "ir_a", "guardar"] },
  { "id": "permanente", "objetivos": ["faccion"], "inicial": "guardar", "admite": ["guardar", "ir_a", "atacar", "unirse"] }
]
```
<!-- /parametros -->

Las medallas, con los umbrales de bronce, plata y oro de su contador (cada medalla cuenta uno solo, el de su id).

<!-- parametros:MEDALLAS_MERCENARIOS -->
```json
[
  { "id": "palabra_cumplida", "umbrales": [1, 10, 30] },
  { "id": "cazador_de_bandidos", "umbrales": [3, 10, 25] },
  { "id": "muralla", "umbrales": [3, 10, 25] },
  { "id": "buen_camino", "umbrales": [5, 20, 50] },
  { "id": "asaltante", "umbrales": [1, 5, 15] },
  { "id": "campana", "umbrales": [1, 5, 15] },
  { "id": "bien_valorado", "umbrales": [5, 20, 50] },
  { "id": "leal", "umbrales": [7, 30, 90] }
]
```
<!-- /parametros -->

## 18. Catálogo de hechos

<!-- eventos -->
```json
["mercenarios.ofrecido", "mercenarios.oferta_retirada", "mercenarios.contratado", "mercenarios.prorrogado", "mercenarios.ordenado", "mercenarios.orden_cumplida",
 "mercenarios.orden_caducada", "mercenarios.mudado", "mercenarios.terminado", "mercenarios.valorado", "mercenarios.rango_subido", "mercenarios.medalla",
 "mercenarios.premio_reclamado", "mercenarios.leva_recibida"]
```
<!-- /eventos -->
