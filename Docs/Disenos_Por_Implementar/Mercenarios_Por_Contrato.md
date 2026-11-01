# Mercenarios por contrato

> **Diseño acordado con el autor el 2026-10-26 y revisado y cerrado el 2026-11-01. Sin implementar.** Vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente (ver su README). Ninguna cifra de este documento es vigente: tarifas, plazos, renombre, rangos, medallas y topes son borradores que se calibran con bots (fase 7). No hay bloque `parametros:` todavía: se crea al implementarlo. Pendiente en `Docs/Mecanicas a desarrollar.md`, entrada 48.
>
> **Versión visual:** [`Mercenarios_Por_Contrato.html`](Mercenarios_Por_Contrato.html), la misma revisión con esquemas y ejemplos (ábrela en el navegador). Si las dos se contradicen, manda este documento.
>
> **Al implementarlo** (decisión del autor del 2026-11-01): el canon gana un documento **propio y solo de los bots mercenarios** (`Docs/Game/7_Bots_Mercenarios.md`), la mecánica su propio documento (`Docs/Mechanics/Mercenarios_Por_Contrato.md`, con sus bloques `parametros`), y este documento y la entrada 48 se borran. Ninguno de los dos se crea antes de que el motor lo tenga.

## 1. Qué es

Un héroe **bot** que reside en un campamento de mercenarios se ofrece en el tablón de su taberna, y un héroe con cargo de una Facción **humana** lo contrata **como miembro de esa Facción** para un trabajo concreto (**temporal**) o para formar parte de su ejército (**permanente**). El bot sigue **fuera del motor** y juega por el contrato 05 como cualquier jugador (`Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md`): el motor guarda el contrato, retiene el pago, aplica reglas duras, da permisos con las órdenes y lleva la progresión; no sabe ni comprueba si quien cumple lo hace bien (de eso se ocupan la valoración y el historial, §8.2).

Descartado: alquilar escuadras sin héroe; que un humano se ofrezca como mercenario; el mercenario como héroe personal de otro héroe.

**Bots libres y bots mercenarios** (decisión del autor del 2026-11-01). Los **libres** crean Facciones, fundan y juegan como hasta ahora; los **mercenarios** no crean Facciones, viven en el bucle de los mercenarios y se ofrecen en su campamento base. La distinción es **del cerebro** (BotsBrain), no del motor: el motor solo sabe si un héroe es bot (`Controlador::Bot`, desde el 2026-11-01 el de toda cuenta de bot) y si una Facción es de bots (su Rey es bot, `Nucleo::es_faccion_de_bots`). **Las Facciones de bots no contratan**, y el contrato es la **única** vía por la que un bot entra en una Facción humana (`SolicitarIngreso` se lo rechaza con `faccion.no_es_de_bots`, `Diplomacia_Facciones.md` §3.2).

## 2. Cómo pertenece a la Facción

**Todo mercenario contratado es ciudadano de la Facción con una marca**: `temporal` o `permanente` (decisión 1 del 2026-11-01). Así la Facción del héroe (`HeroeNucleo::faccion`, que deciden encuentros, bando en batalla, puerta, convocatorias, visión y misiones en 361 sitios del núcleo) es la de quien le paga, y nada de eso cambia. La marca solo añade guardas:

- **Trono y sucesión**: un marcado nunca es Rey; la sucesión y el caso «Facción sin nadie» (`Diplomacia_Facciones.md` §3.3) lo saltan.
- **Cargos**: un marcado no puede ser Embajador ni tener ningún cargo local (Gobernador, Tesorero, General, Maestro de Obras, Sacerdote, Maestro de Caravanas).
- **Salir**: `DejarFaccion` se rechaza; solo sale cuando el contrato acaba (§7).
- **Lo que ve**: una FACCION reducida (§10).

El temporal **sigue residiendo en su campamento** (un ciudadano ya puede residir en uno) y conserva lo que tiene allí. El permanente reside en la plaza que elige el contratante (§5).

## 3. Quién se ofrece y quién contrata

**Se ofrece** (`ofrecerse_como_mercenario`): un héroe bot, **sin Facción**, que reside en un campamento, dentro de él. Entra en el tablón de ese campamento. Mientras está ofrecido o contratado no puede crear Facción, pedir ingreso ni fundar (`faccion.mercenario_ofrecido`). Retira la oferta con `retirar_oferta`; al firmar se retira sola.

**Se contrata en persona.** Quien firma viaja con su columna al campamento con el pago en el carro, entra, elige del tablón y paga con el oro del carro (como comprar de visita, `Comercio_Caravanas.md` §9.2). No hay contratación a distancia. El oro va al **almacén personal del mercenario**, que lo gasta en el campamento: es un sumidero, no una fuente. Un campamento no tiene puerta ni vetos: contratar no depende de relaciones. Se rechaza si el Rey de la Facción del contratante es bot (`mercenario.faccion_de_bots`).

**Quién firma qué** (decisión 5 del 2026-11-01, con el General añadido por el autor):

| Trabajo | Rey, Embajador | Gobernador | General | Maestro de Caravanas |
|---|---|---|---|---|
| Defender plaza, pueblo o avanzada | todas | su plaza | su plaza | no |
| Atacar un objetivo | todos | solo bandidos de su zona | bandidos de su zona y enemigos en guerra | no |
| Escoltar caravana, carro de acarreo, encargo o ruta | todas | su plaza | no | su plaza |
| Acompañar a un ejército | cualquiera | no | su propia convocatoria | no |
| Permanente | sí | no | no | no |

El **Tesorero no contrata**: paga quien firma, con su carro. «Su plaza» incluye sus pueblos, sus avanzadas y sus caravanas.

## 4. Los trabajos (`TRABAJOS_MERCENARIOS`)

Tabla de datos que el motor lee: añadir un trabajo es añadir una fila con su objetivo, su orden inicial y las órdenes que admite (§6.4). «Despejar un campamento de bandidos» del diseño del 26-10 se amplía a **«Atacar un objetivo»** (bandidos o cualquier objetivo de una Facción en guerra; decisión A del 2026-11-01).

| Trabajo | Tipo | Objetivo |
|---|---|---|
| Defender una plaza, un pueblo o una avanzada | temporal | la plaza, el pueblo o la avanzada |
| Escoltar | temporal | una caravana, el carro de acarreo de un pueblo, un encargo o una ruta (escolta la caravana de cada viaje) |
| Atacar un objetivo | temporal | un campamento de bandidos, un ejército, una caravana, una plaza, un pueblo o una avanzada |
| Acompañar a un ejército | temporal | una convocatoria o un ejército |
| Ser parte del ejército de la Facción | permanente | la Facción |

## 5. Temporal y permanente

| | Temporal | Permanente |
|---|---|---|
| Pago | por adelantado al firmar; no se devuelve nunca | pago único al firmar; no se devuelve |
| Residencia | sigue en su campamento | la plaza que elige el contratante, desde la firma y sin cooldown; después solo cambia con la acción `mudar` |
| Duración | 1, 3 o 7 días de mundo (borrador) | hasta el despido |
| Órdenes | las que admite su trabajo (§6.4) | todas |
| Reclutar | en su campamento, como residente | en su plaza, como cualquier residente (las reservas del mantenimiento y del Tesorero protegen lo esencial) |
| Quién lo firma | Rey, Embajador o el cargo de la plaza (§3) | Rey o Embajador |

**Tarifa.** La fija el motor por fórmula: nivel y tropa del mercenario con los precios de reclutar del campamento (`Heroe_Y_Tropa.md` §12, `Campamentos_Mercenarios.md` §5.2), **multiplicada por el plazo** (más tiempo, más oro) y **por su rango** (§8.3). El bot no fija precio.

**El contrato es de la Facción**, o de la plaza si lo firmó un cargo local: si quien firmó pierde el cargo o se va, lo manda quien ocupe ahora ese cargo.

## 6. Las órdenes de mando

Una orden hace dos cosas: le dice al bot qué hacer (le llega por su ámbito HEROE) y le da **permiso en el motor** para lo que sin ella se le rechaza. El motor no vigila cómo se mueve ni si cumple bien: solo controla lo que necesita permiso. Moverse es libre siempre.

### 6.1 Las cuatro órdenes (`ordenar_mercenario`)

| Orden | Qué le pide | Qué le deja hacer en el motor | Cuándo acaba |
|---|---|---|---|
| `guardar(objetivo)` | quedarse con el objetivo y defenderlo; si es una caravana, un encargo o una ruta, escoltarla | guarnecer ese pueblo o esa avanzada (la avanzada **sin residir en su madre**: única excepción a `avanzada.no_resides`); adjuntarse a esa caravana o cederle escuadras; entrar en una batalla donde su Facción defiende ese objetivo | no acaba sola; si el objetivo desaparece, vuelve la orden inicial |
| `ir_a(lugar)` | ir allí y esperar | nada nuevo | llegar no la acaba (espera allí); acaba al sustituirla o al vencer su plazo (dato) |
| `atacar(objetivo)` | atacar ese ejército, caravana, campamento de bandidos, plaza, pueblo o avanzada | `Atacar` y `Perseguir` ese objetivo y `UnirseABatalla` contra él; contra otra Facción, solo con la guerra ya declarada | se **cumple** cuando el objetivo cae o desaparece; **caduca** con la paz o al vencer su plazo |
| `unirse(convocatoria o ejército)` | entrar en esa convocatoria o ese ejército y seguir a su Líder | unirse a ese y a ningún otro; dentro, ataca lo que ataque el ejército porque manda su Líder | cuando el ejército se disuelve, la convocatoria se cancela o le separan |

**Sin una orden que le dé permiso, solo se defiende**: su columna va en defensiva y `Atacar`, `Perseguir` o unirse a un ejército se rechazan con `mercenario.sin_orden`. **Prohibido siempre**, tenga la orden que tenga: declarar guerra (`declara_guerra` se rechaza), atacar a su propia Facción, convocar un ejército, fundar y crear o pedir Facción.

### 6.2 Acciones de mando

Se aplican al momento y no quedan como orden:

- **`mudar(plaza)`**: cambia la residencia de un permanente a otra plaza de la Facción, sin cooldown. Rey o Embajador.
- **`despedir`**: acaba el contrato (§7). Quien podría firmar ese contrato.
- **`prorrogar`**: alarga un temporal pagando otro plazo; el contrato sigue sin cortes. **En persona**: quien firma, con el oro en el carro, en el campamento o con su columna a `radioPuerta` de la del mercenario (decisión D del 2026-11-01).

### 6.3 El ciclo de una orden

Cada orden guarda tipo, objetivo, quién la dio, cuándo y su plazo. Al firmar entra la **orden inicial** del trabajo, vigente. Desde vigente:

- **Cumplida** (el objetivo cayó) o **caducada** (paz, objetivo perdido, plazo vencido): vuelve la orden inicial del contrato. Un «Atacar un objetivo» cuyo objetivo cayó no tiene a qué volver: el mercenario espera donde está hasta otra orden.
- **Sustituida**: otra orden válida pasa a vigente.
- **Anulada**: el contrato acaba.

### 6.4 Qué admite cada trabajo

| Trabajo | Orden inicial | Órdenes que admite |
|---|---|---|
| Defender plaza, pueblo o avanzada | `guardar(ese objetivo)` | `guardar` de ese objetivo, `ir_a`, `atacar` solo lo que esté a ≤ un radio de defensa del objetivo (dato) |
| Escoltar | `guardar(esa caravana, encargo o ruta)` | `guardar` de ese objetivo, `ir_a` |
| Atacar un objetivo | `atacar(ese objetivo)` | `atacar` (lo que pueda ordenar quien manda), `ir_a` |
| Acompañar a un ejército | `unirse(esa convocatoria)` | `unirse`, `atacar`, `ir_a`, `guardar` |
| Permanente | `guardar(su plaza)` | todas |

Una orden que el trabajo no admite: `mercenario.orden_fuera_de_contrato`.

### 6.5 Quién ordena qué y el rango entre órdenes

| Cargo | A quién manda | Qué órdenes |
|---|---|---|
| Rey, Embajador | a todos los mercenarios de la Facción | todas y las tres acciones |
| General de la plaza P | contratos de P y permanentes que residen en P | `guardar` (objetivos de P), `atacar`, `unirse` (solo a su convocatoria), `ir_a` |
| Gobernador de P | contratos de P y permanentes que residen en P | `guardar` (objetivos de P), `atacar` (solo bandidos), `ir_a` |
| Maestro de Caravanas de P | escoltas de P | `guardar` (caravanas, encargos y rutas de P), `ir_a` |

Fuera de su alcance: `mercenario.sin_alcance`. **Rango** (decisión B del 2026-11-01): Rey > Embajador > General y Gobernador > Maestro de Caravanas; una orden solo la sustituye quien tiene igual o más rango que quien la dio (`mercenario.orden_de_mas_rango`). General y Gobernador tienen el mismo rango: manda la última.

## 7. El ciclo del contrato

| | Temporal | Permanente |
|---|---|---|
| Al firmar | entra con la orden inicial de su trabajo | reside en la plaza elegida; `guardar(su plaza)` |
| Acaba por | plazo, despido, la plaza que firmó cae o cambia de dueño, no queda ningún ciudadano sin marca, se borra su cuenta | despido, no queda ningún ciudadano sin marca, se borra su cuenta |
| Dónde queda | donde estaba, residiendo en su campamento | vuelve a residir en su campamento base (el de la firma), sin cooldown |
| Si pierde su plaza | no le afecta, salvo que fuera la que firmó | se reubica como cualquier ciudadano; si la Facción se queda sin plazas, vuelve a su campamento base sin perder el contrato |

Para los dos:

- **En mitad de una batalla**, el fin se **aplaza hasta que la batalla acaba** (decisión C del 2026-11-01).
- **En un ejército o una convocatoria**, se separa con su columna en el sitio donde esté.
- **Sus guarniciones** vuelven a su campamento, como hoy cuando un héroe cambia de Facción.
- Sale de la Facción en el mismo instante y su orden queda anulada. **Nunca se devuelve nada.**
- **Si se va el último ciudadano sin marca**, todos los contratos caen y se aplica el caso de siempre de «Facción sin nadie».
- **Tras un despido**, no se le recontrata en la misma Facción hasta pasado un tiempo de datos (`mercenario.recontrato_en_espera`).

## 8. Progresión

### 8.1 El problema

Cada campamento tiene **un solo edificio militar** (barracón, galería de tiro o caballerizas, por su posición) y solo ofrece las tropas cuya tecnología ha llegado a él: 72 h después de que alguien la descubra o cuando la tiene adoptada la mitad de las Facciones vivas (`Tecnologia_Aedas.md` §5). Al empezar el mundo solo están las de arranque: con galería se reclutan honderos y escaramuzadores con jabalina; con barracón o caballerizas, nada. La leva prestada (`Campamentos_Mercenarios.md` §6) no gana experiencia y se pierde al dejar de residir, así que al permanente se le va al mudarse.

### 8.2 Renombre y valoración

El mercenario tiene una experiencia propia, el **renombre** (decisión E del 2026-11-01), aparte del nivel del héroe (que hoy no sube, CQ-001) y de la experiencia de cada escuadra (que sigue como hoy). Solo lo gana bajo contrato. Borrador:

| Qué hace | Renombre | Lo comprueba |
|---|---|---|
| Termina un temporal sin despido | 10 · 25 · 50 por plazo de 1 · 3 · 7 días | el motor, al vencer el plazo |
| Cumple una orden `atacar` | 5 a 30 según el objetivo (bandidos por nivel, columna, pueblo, plaza) | el motor, cuando cae el objetivo y él estaba en la batalla |
| Gana una batalla defendiendo su objetivo de `guardar` | 10 | el motor, al cerrar la batalla |
| Llega a destino una caravana que escoltaba | 5 por viaje | el motor, a la llegada |
| Cada grado nuevo de una medalla (§9) | 10 · 30 · 80 | el motor |

**Valoración del contratante** (decisión F): al acabar un contrato, quien lo firmó (o el Rey) lo valora con `valorar_mercenario`: **buena** (×1,5 sobre el renombre de ese contrato), **sin valorar** (×1) o **mala** (×0). Queda en su historial del tablón. Cubre lo que el motor no puede comprobar (si defendió de verdad). Un despido no resta, pero ese contrato no suma. El permanente no tiene plazo: gana por órdenes cumplidas, batallas, escoltas y medallas. Como las Facciones de bots no contratan, los bots no pueden darse renombre entre ellos.

### 8.3 Rangos y premios

| Rango | Nombre | Renombre | Al llegar | Licencia de reclutar | Tarifa |
|---|---|---|---|---|---|
| 1 | A sueldo | 0 | la leva comunal, suya | — | ×1 |
| 2 | Lanza | 100 | una escuadra de premio | escalón ≤ 2 | ×1,2 |
| 3 | Capitán | 300 | una escuadra de premio | escalón ≤ 3 | ×1,5 |
| 4 | Veterano | 700 | una escuadra de premio y, cuando haya catálogo, un objeto | escalón ≤ 4 | ×1,8 |
| 5 | Señor de la guerra | 1500 | una escuadra de premio y, cuando haya catálogo, un objeto | escalón ≤ 5 | ×2,2 |

- **Rango 1: la leva, suya** (pedido del autor el 2026-11-01). La primera vez que se ofrece recibe gratis, **una sola vez**, una escuadra de cada tropa de la leva comunal: lanceros de milicia, granjeros, mineros y leñadores. Son **suyas**, no prestadas: ganan experiencia, no se pierden al mudarse y las repone pagando en su campamento aunque el campamento no tenga Centro Urbano. Ocupan 35 de Liderazgo (14 + 7 + 7 + 7). Así es útil desde el primer contrato.
- **Escuadra de premio** (decisión G). Al subir de rango gana el derecho a una escuadra completa, gratis y suya. La reclama en su campamento base con `reclamar_premio_de_rango` (como la recompensa de una misión) y elige la tropa entre las de su licencia; si ya la tiene, se la reponen entera; si su licencia aún no tiene ninguna tropa, el premio espera.
- **Licencia de reclutar** (decisión H). Recluta en su campamento base las tropas **de su edificio** hasta su escalón **en cuanto alguien en el mundo ha descubierto la tecnología**, sin esperar las 72 h ni a la mitad de las Facciones. No salta las Eras (lo que nadie descubrió sigue cerrado) y no cambia de edificio: un mercenario de galería es un tirador.
- **Objetos**: reservados en la tabla hasta que exista su catálogo (CQ-004).
- **El Liderazgo no cambia con el rango.** Es igual para todos los héroes, bots, humanos o mercenarios, y no se toca (decisión del autor del 2026-11-01).

## 9. Medallas

Logros que el mercenario gana una vez y no pierde nunca, con tres grados (bronce, plata y oro). Cada grado da renombre la primera vez (§8.2). Se cuentan con los hechos que el motor ya anota, solo bajo contrato. Es una tabla de datos (`MEDALLAS_MERCENARIOS`: id, contador y umbrales), como los logros de tecnología. Aprobadas por el autor el 2026-11-01; umbrales en borrador:

| Medalla | Qué cuenta | Bronce · plata · oro |
|---|---|---|
| Palabra cumplida | contratos terminados sin despido ni mala valoración | 1 · 10 · 30 |
| Cazador de bandidos | campamentos de bandidos destruidos | 3 · 10 · 25 |
| Muralla | batallas ganadas defendiendo su objetivo | 3 · 10 · 25 |
| Buen camino | caravanas que llegan a destino con él de escolta | 5 · 20 · 50 |
| Asaltante | pueblos, avanzadas o plazas tomados con orden de atacar | 1 · 5 · 15 |
| Campaña | convocatorias en las que sigue hasta que el ejército se disuelve | 1 · 5 · 15 |
| Bien valorado | valoraciones buenas | 5 · 20 · 50 |
| Leal | días seguidos como permanente de la misma Facción | 7 · 30 · 90 |

## 10. Qué ve cada uno

**El mercenario** (decisión 19 del 2026-11-01) recibe una **FACCION reducida**: nombre y sigilo de la Facción, su contrato, su orden vigente, las convocatorias a las que le invitan y lo que necesita de su objetivo (la plaza, el pueblo, la caravana o el ejército). Sin miembros, relaciones, propuestas, tecnología ni memoria de mapa de la Facción. Así un bot no se lleva información de una Facción humana al ecosistema de bots.

**Memoria de mapa.** Sus ojos alimentan a la Facción mientras dura el contrato (es parte de lo que se paga), pero su memoria personal **no** se transfiere ni se borra al firmar (excepción a `Niebla_De_Guerra.md` §4.3), y al irse no se lleva la de la Facción.

**Los de la Facción.** Los contratos van en FACCION a todos los ciudadanos sin marca (FACCION no filtra por cargo); el cargo decide quién ordena, no quién ve.

**Su ficha.** Rango, renombre (con lo que falta para el siguiente), medallas con su grado e historial (contratos firmados, terminados, despidos, valoraciones buenas y malas):

| Dónde | Qué lleva |
|---|---|
| Tablón del campamento (ámbito CAMPAMENTO, solo desde dentro: la entrada 62 está aparcada) | la ficha entera, más su tropa, si está herido y su tarifa por trabajo y plazo; aquí se elige a quién contratar |
| Su héroe público (quien lo vea) | rango, renombre y medallas, y la marca `contratado` si está bajo contrato |
| Su ámbito HEROE | todo, más su contrato, su orden y lo que le falta para el siguiente grado de cada medalla |
| Nunca | quién le valoró ni para quién trabajó: el historial da cifras, no nombres |

El directorio de Facciones cuenta solo a los ciudadanos **sin** marca. La ciudadanía sigue siendo pública (`ciudadanos_ids`).

## 11. Otras reglas

- **Misiones** (decisión 11): lo que hace cuenta para las misiones de la Facción y de la plaza; él no gana Participación ni recibe misiones de héroe de la Facción.
- **Conquistas**: una plaza que toma es de su Facción; como no puede ser Gobernador, queda sin Gobernador hasta que el Rey nombre uno.
- **Guarniciones**: como ciudadano guarnece pueblos y entra en plazas de su Facción; las avanzadas, solo con `guardar` de esa avanzada (§6.1).
- **Escolta**: su columna va adjunta a la caravana o cede escuadras a su escolta como cualquier ciudadano.
- **Almacén de la plaza**: el permanente lo usa como cualquier ciudadano; `CerrarAlmacenACiudadanos` también lo cierra para él.
- **Vetos**: la plaza puede vetar a un permanente; vetarlo equivale a quitarle esa residencia, y vuelve a su campamento base sin perder el contrato.
- **Herido**: se le puede contratar herido (el tablón lo muestra) y el plazo corre igual.
- **Leva prestada**: puede pedirla como cualquier residente; no cuenta para nada de lo anterior.

## 12. Qué toca en el motor (orientativo)

- **Estado**: ofertas por campamento; contratos (por Facción o plaza) con trabajo, objetivo, plazo, orden vigente y su historia corta; marca de ciudadanía; por héroe mercenario: renombre, rango, premios pendientes, contadores y grados de medallas, historial y si ya recibió su leva; espera de recontrato. Foto **BAN54**.
- **Órdenes nuevas desde la 171** (nombres provisionales): `ofrecerse_como_mercenario`, `retirar_oferta`, `contratar_mercenario`, `prorrogar_contrato`, `ordenar_mercenario`, `mudar_mercenario`, `despedir_mercenario`, `valorar_mercenario`, `reclamar_premio_de_rango`. Filas nuevas en la matriz de autorización (`Proyecciones_Y_Autorizacion.md` §5.1).
- **Rechazos** (provisionales, al catálogo 07): `mercenario.faccion_de_bots`, `mercenario.sin_orden`, `mercenario.orden_fuera_de_contrato`, `mercenario.sin_alcance`, `mercenario.orden_de_mas_rango`, `mercenario.recontrato_en_espera`, `mercenario.no_ofrecido`, `mercenario.sin_premio`, `faccion.mercenario_ofrecido`.
- **Hechos** (provisionales): `mercenarios.ofrecido`, `.contratado`, `.ordenado`, `.terminado`, `.despedido`, `.prorrogado`, `.valorado`, `.rango_subido`, `.medalla`, `.premio_reclamado`. Firma y fin a la Facción y al mercenario; la orden solo al mercenario y a quien la dio; nada público salvo la ficha y la marca.
- **Guardas** en `DejarFaccion`, en el Rey y la sucesión, en los cargos, en `Atacar`/`Perseguir`/`UnirseABatalla`/unirse a ejércitos, en `GuarnecerAvanzada`, en fundar y crear o pedir Facción, y en la proyección de FACCION y en la transferencia de memoria al entrar.

## 13. Plan por tandas

Cada tanda deja la suite verde y sube versiones (`CLAUDE.md`).

1. **M1. Oferta y firma**: tabla de trabajos, ofertas, contratos, marca, plazo en el reloj, rechazo a Facciones de bots, foto BAN54.
2. **M2. Mando**: las cuatro órdenes, permisos y rechazos, defensiva, ciclo de la orden, rango, contrato que cae con la plaza.
3. **M3. Permanente**: residencia, guardas de trono y cargos, `mudar`, `despedir`, fin aplazado en batalla, Facción sin nadie sin marca, espera de recontrato.
4. **M4. Trabajos**: avanzadas por `guardar`, escoltas por caravana, encargo o ruta, misiones que cuentan, `prorrogar`.
5. **M5. Vistas**: FACCION reducida, memoria sin transferir, marca pública, tablón, hechos.
6. **M6. Progresión**: renombre, valoración, rangos, leva propia, premio a reclamar, licencia, tarifa por rango, medallas y ficha.
7. **M7. Documentación**: el canon nuevo de bots mercenarios, el documento de Mechanics propio, la ronda final de Mechanics y canon contra el diff, y se borran este documento y la entrada 48.

M1 y M2 juntas ya permiten probar con bots el ciclo básico (ofrecerse, firmar, obedecer).

## 14. Fuera del motor

- **BotsBrain**: perfil mercenario (no funda ni crea Facción, se ofrece en su campamento base, viaja al objetivo, cumple la orden vigente, reclama sus premios y vuelve a ofrecerse). Sin él no hay a quién contratar en pruebas.
- **BronzeAgeClient**: pestaña «Mercenarios» de la Facción (contratos vivos con trabajo, objetivo, plazo, ubicación, tropa y orden), «Contratar» desde el tablón con la ficha, «Ordenar», «Prorrogar», «Despedir» y «Valorar».
- **Calibración con bots**: tarifas y multiplicadores, renombre, umbrales de rangos y medallas, radio de defensa, plazo de `ir_a` y de `atacar`, espera de recontrato, topes de contratos y de permanentes por Facción, número de bots por campamento, oro que vuelve a circular y coste de órdenes por bot (techo medido ≈ 65 órdenes/s con ≈ 450 vistas, T6 de 4H).

## 15. Casos límite

- El motor no comprueba que el temporal cumpla: la valoración y el historial son su reputación.
- Un bot cuya cuenta se borra: sus contratos caen sin devolución.
- Un campamento no tiene puerta ni vetos: contratar no depende de relaciones.
- Una orden de un cargo que deja de serlo sigue vigente hasta que se cumple, caduca o la sustituye alguien con rango.
