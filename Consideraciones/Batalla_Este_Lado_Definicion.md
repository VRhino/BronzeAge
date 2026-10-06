# Batallas con héroes: lo que falta de este lado — Definición

**Estado: APROBADO e IMPLEMENTADO en el backend (2026-10-06), sin push.** Era la entrada §30 de `Docs/Mecanicas a desarrollar.md`. Las
reglas están en el canon (`Docs/Game/5_Sistema_Militar_y_Combate.md` §5.12, §5.14.4, §5.15.1b, §5.16.4 y `2_Sistema_Politico...` §2.2),
el contrato en `Docs/Coordinacion/01` §15 y `02` §3, y la petición a Conquest en su `propuestas/CQ-011_tipos_de_batalla_y_salidas.md`.

## 0. El reparto con Unity

**Unity decide todo lo que pasa DENTRO de una batalla**: banderas, puntos de aparición, si se reaparece y con qué
espera, rendición, cómo se gana y cómo cambian de manos las caravanas dentro de la partida. **BronzeAge solo controla lo
que toca al mundo**: que ocurra y dónde, quién entra, quién sale y qué le queda a cada uno al salir (herido, carro,
conquista, caravana, botín).

*Descartado:* llevar a `BattleRules` el modo de victoria, la reaparición o la rendición. Rompía esa separación: el backend
habría tenido que conocer reglas de partida que no usa para nada.

Las ideas del autor sobre cómo se juega cada tipo (más abajo, §1.3) viajan a Conquest en la CQ-011 como **diseño
propuesto**, no como contrato.

## 1. Dos clases de columna y cuatro tipos de batalla

### 1.1 Ejército y héroe en solitario

| | **Ejército** | **Héroe en solitario** (columna personal) |
|---|---|---|
| Qué es | Varios héroes (o uno que salió así) con sus tropas, caravanas adjuntas y carro común | Un héroe que salió por su cuenta con su carro y las tropas de su Liderazgo |
| Cómo nace | Se moviliza desde un asentamiento contra un destino (Doc 5.12.1) **o se forma en campo con 3 o más (§2)** | `salirAlMundo` desde su residencia |

### 1.2 Quién puede atacar o perseguir a quién

**Un ejército y un héroe en solitario nunca combaten entre sí**, en ninguna dirección: un ejército no persigue ni ataca
a un solitario, y un solitario no puede atacar a un ejército para frenarlo.

| Atacante ↓ / objetivo → | Ejército | Solitario | Plaza | Caravana suelta | Campamento de bandidos |
|---|---|---|---|---|---|
| **Ejército** | Batalla campal (y puede perseguirlo) | ✗ | Asedio | Asalto de caravana | Sí (como hoy) |
| **Solitario** | ✗ | Persecución (y puede perseguirlo) | ✗ abrir; **sí unirse** a un asedio ya abierto (§1.3) | Asalto de caravana | Sí (como hoy) |

**Solo un ejército abre un asedio** (decisión del autor, 2026-10-06). Un solitario no puede `atacar` una plaza, pero sí
unirse a un asedio abierto con las mismas condiciones que cualquiera.

Una caravana **adjunta a un ejército** no es una caravana suelta: es parte del ejército, y solo otro ejército puede
atacarla (batalla campal con las caravanas en juego).

### 1.3 Los tipos de batalla

| Tipo | Se abre | Quién puede unirse (BronzeAge) | Cómo se juega (Unity, diseño propuesto por el autor) |
|---|---|---|---|
| **Asedio** | Un ejército hace `atacar` a una plaza enemiga. **Convocatoria de 30 s antes de empezar** (§3.3) | Ejércitos y solitarios. Atacante: **los del ejército que la abre, primero**; si queda sitio, su Facción y, si su Facción lo admite (§3.2), neutrales o enemigos del defensor. Defensor: su Facción y aliados (vasallos incluidos) | Banderas en orden y base, como el modo Batalla de Conquest; el punto de aparición del atacante avanza con las banderas; si se agota el tiempo gana el defensor; quien muere no reaparece |
| **Batalla campal** | Un ejército `atacar` a otro ejército | **Nadie de fuera** | Dos frentes. Por decidir en Unity: banderas (un fuerte por lado y el centro) o el último en pie con rendición a partir de ~10 min. Se reaparece hasta el final, con espera creciente en cada muerte |
| **Persecución** | Un solitario `atacar` a otro solitario | **Libre**: cualquier solitario, de cualquier Facción, en el bando que elija, mientras haya sitio | Una bandera por bando; gana quien toma la del otro; si se agota el tiempo, el defensor (el perseguido). Quien muere no reaparece |
| **Asalto de caravana** | Un ejército o un solitario `atacar` a una caravana suelta | Atacante: su Facción y, si la admite, neutrales o enemigos del defensor. Defensor: la Facción dueña y sus aliados | Lo define Conquest (las caravanas pueden cambiar de manos dentro de la partida). **En la documentación de Conquest no hay todavía un modo de caravanas**: va como petición en la CQ-011 |
| **Campamento de bandidos — evento PvE** | Un ejército o un solitario `atacar` a un campamento | **Cualquier héroe, de cualquier Facción, siempre en el bando atacante.** Nadie puede ayudar a los bandidos: su bando es solo la IA del campamento | Sin cambios: los héroes contra la IA del campamento |

**Los campamentos de bandidos son un evento PvE**, la primera de una clase de batallas contra el mundo (puede haber más).
No hay diplomacia que mirar: todos los héroes contra la IA. Se narran como evento propio (`evento_pve.*`, no `batalla.*`
entre Facciones), y cada héroe que participa cobra su oro como hoy (Doc 1.9, D22).

El tope por bando sigue en **5 héroes** (el asedio vuelve a 15 cuando entren jugadores, §38). Si el ejército que abre un
asedio ya llena el bando, no entra nadie más.

## 2. Formar un ejército en campo

Excepción nueva a dos reglas del canon: «un Ejército solo se origina en un asentamiento» (Doc 5.12.1) y «dos Columnas
personales no se fusionan» (Doc 5.14.1).

| | Regla |
|---|---|
| **Empezar** | Un solitario pulsa «organizar ejército». En su posición aparece un ejército **en formación**, quieto. Él es el Líder y fija la política de unión (aceptar o preguntar, Doc 5.14.1) |
| **Unirse** | Solitarios **de la misma Facción**, junto a la miniatura, con lo que llevan encima: escuadras y carro, validados contra su Liderazgo |
| **Mientras haya menos de 3** | No se mueve, y **para el combate sigue contando como solitarios**: se le puede perseguir y atacar como a ellos (una persecución, con todos sus miembros de defensores). Formar no sirve de escudo |
| **Si no llega a 3** | Pasado el plazo (`FORMACION_EJERCITO.plazoMinutos`, provisional 10) se deshace y cada uno vuelve a su columna personal con lo suyo. Mientras tanto cualquiera puede salir; si sale el Líder, el mando pasa al más antiguo, y si no queda nadie, desaparece |
| **Al llegar a 3** | Pasa a ser un ejército normal: el Líder fija el destino, que ya no se toca (Doc 5.12.1), y desde ahí rige todo el canon. Si luego baja de 3 sigue siendo ejército: el mínimo cuenta solo para formarlo |
| **Origen** | La residencia del Líder al formarlo (su plaza o su campamento de mercenarios). Ahí vuelve si cancela la marcha, y lo conserva quien se separe |
| **Caravanas** | Nace sin ellas (un solitario no las lleva). Puede engancharlas después, como cualquier ejército |
| **Bots** | No la usan por ahora |

*Descartado:* formar con 2 (el autor quiere un ejército de verdad, no una pareja), y tratar la formación como un
ejército para el combate desde el primer héroe (sería un refugio contra persecuciones).

## 3. Entrada

### 3.1 Las reglas de siempre, más la tabla de §1.3

Un héroe se une si está **sano**, **no está ya en otra batalla**, su columna está **a 15** del punto de la batalla (o de la
plaza asediada) y **el bando tiene sitio**. Qué bando puede elegir lo da §1.3. En la persecución elige el bando; en el
resto lo deduce el servidor de su Facción y su diplomacia.

### 3.2 «Admitir a otras Facciones en nuestros ataques»

**Ajuste permanente de la Facción**, que solo cambia el Rey. Activado, en los asedios y asaltos de caravana que **abra
su Facción** pueden unirse al bando atacante héroes de Facciones **neutrales o enemigas del defensor** (nunca un aliado
del defensor), mientras quede sitio. Desactivado por defecto.

*Descartado:* un permiso batalla a batalla. Una batalla dura poco y el Rey puede no estar conectado: casi nunca llegaría a
tiempo.

**Para unirse a una batalla, un vasallo y su señor cuentan como aliados** (decisión del autor, 2026-10-06): pueden
defenderse entre sí, y ninguno puede unirse al ataque contra el otro. Es una regla de las batallas: no cambia
`estanAliadas` en el resto del juego (comercio, caminos, ataques).

### 3.3 Convocatoria de un asedio: 30 segundos

Al abrirse un asedio hay **30 segundos** de mundo (`BATALLA.convocatoriaSegundos`) antes de que la batalla salga hacia
Unity. En ese tiempo se une quien cumpla §1.3, y **entra en el ticket** como uno de los que estaban al empezar, no
como incorporación. Al cerrarse la convocatoria se congela el ticket (revisión 0) y solo entonces aparece en
`GET /v1/batallas/pendientes`. Mientras dura, quien la abrió puede cancelarla.

Para Conquest el cambio es mínimo: recibe el ticket 30 segundos más tarde y ya completo. Los que llegan después
siguen entrando como incorporaciones, mientras haya sitio.

Solo en los asedios; el resto de batallas sale hacia Unity al momento. Una batalla que se resuelve con números (bot
contra bot) no tiene convocatoria.

## 4. Salida

Unity avisa de que un héroe sale de la batalla con **un solo mensaje**, `POST /v1/batallas/:battleId/salidas` (credencial
de servidor), con `heroeId`, cómo quedaron sus escuadras y `derrotado: boolean`. Por qué sale lo decide Unity.

| Cuándo | Qué hace BronzeAge |
|---|---|
| **Antes de empezar** (`asignada`: no se conectó) | Sale libre con sus escuadras intactas y `derrotado: false`. **Sube `ticketRevision`**; la asignación y los tokens anteriores dejan de valer y la batalla vuelve a `convocando`. No hay sustituto: el bando sigue con los que quedan |
| **En curso** (Unity lo sacó de la partida: murió sin reaparición, abandonó, se desconectó) | Aplica las bajas de sus escuadras, suelta sus candados y, si viene `derrotado`, lo deja **herido** 2 minutos. Su columna sale del bloqueo si no le queda nadie dentro. **El ticket no cambia**: las salidas, como las incorporaciones, solo se añaden a una lista |

Si tras una salida antes de empezar **no queda ningún héroe humano**, o el atacante se queda **sin héroes**, la batalla se
cancela sin castigo. Si sale quien la inició, el derecho a cancelarla pasa al primer héroe del bando atacante.

**Herido = `derrotado`** (Doc 5.16.4). El `BattleResult` trae `derrotado` en cada héroe, y queda herido quien lo
traiga, sea del bando que sea. Así lo cubren también el abandono y la desconexión de CQ-010. En el combate con números
(bot contra bot) sigue como hoy: todo el bando que pierde.

*Descartado:* que el iniciador sustituya a otros (podría expulsar a aliados), y un sustituto automático (la cola de
espera es de Unity).

## 5. Consecuencias: botín y chatarra

Lo que gana el bando vencedor —la mitad del carro de cada columna vencida, la carga de la caravana capturada y la
**chatarra** de las bajas (Doc 4.2.1)— se reparte **a partes iguales entre las columnas del bando ganador**. Lo que no
le cabe a una pasa a las que tengan sitio, y si no cabe en ninguna se pierde. La chatarra de un asedio que gana el
defensor sigue yendo al almacén de la plaza. Los objetos y monedas del héroe (CQ-004) no entran aquí: los reparte Unity
en el `BattleResult`.

*Descartado:* reparto por héroes (premia la columna grande) y por supervivientes (paga más ganar con pocas bajas).

## 6. Persecución en el mapa

Perseguir fija un objetivo móvil y **la ruta se recalcula de verdad** (hoy no se calcula ninguna: una columna parada no
se mueve). Se recalcula solo cuando la presa se ha movido más de `PERSECUCION.umbralRecalculo` (provisional) desde el
último destino calculado. Así, en casi todos los ticks el coste es una comparación de distancias.

La persecución termina además de dos formas nuevas:

- **La presa sale del alcance de vista** del perseguidor: se suelta (`columna.presa_perdida`). Si no, la trayectoria del
  perseguidor revelaría dónde está la presa bajo la niebla.
- **La presa entra en una plaza o campamento**: se suelta, y el perseguidor va hasta su puerta y acampa
  (`columna.presa_a_cubierto`). Allí se le ofrece lo de una plaza.

Y solo se persigue dentro de la misma clase (§1.2): ejército a ejército, solitario a solitario o caravana.

## 7. Canal de tiempo real `batalla/<battleId>`

- **Quién se suscribe:** con el mismo criterio con que la proyección enseña la batalla en el mapa: combate en ella, o
  la ve ahora mismo bajo la niebla. Se evalúa al suscribirse, como `asentamiento/<id>`.
- **Qué viaja:** los eventos de estado (`abierta`, `refuerzos`, `asignada`, `en_curso`, `salida`, `revisada`,
  `aplicada`, `cancelada`, `fallida`) con el estado público: contexto, punto, Facciones y número de héroes por bando,
  capacidad y si admite a otras Facciones. Nunca tokens, escuadras ni listas de héroes.
- **Los hogares** se siguen enterando por su canal de asentamiento, como hoy.
- **En el mapa** se ve la batalla en su punto, bajo la niebla (Doc 5.15.1). Desde su miniatura se une uno a ella.

## 8. Contrato (CQ-011)

| Cambio | Dónde |
|---|---|
| `campo_abierto` añade `columnas: 'ejercitos' \| 'solitarios'` (campal o persecución); Unity elige el modo con eso | `ContextoEstrategico` |
| `BattleParticipantSnapshot.faccionId`: un bando ya puede mezclar Facciones (neutrales en un ataque, persecución libre). `BattleSide.faccionId` sigue siendo la titular | ticket e incorporaciones |
| `POST /v1/batallas/:battleId/salidas` y su schema `SalidaBatalla` | doc 02 §3.3, doc 01 §15 |
| `BattleResult.porHeroe[].derrotado` | doc 01 §15 |
| El resultado ya no repite a quien salió antes | checklist doc 02 §3.3, puntos 2 y 4 |
| Canal `batalla/<id>` con sus eventos | doc 02 §3.5 |
| `unirseABatalla` admite `lado` (obligatorio en una persecución) | doc 02 §3.1 |
| El asedio tiene una convocatoria de 30 s: el ticket se publica al cerrarla y ya incluye a los que se unieron en ella | doc 01 §15 (estado `convocando`), doc 02 §3.2 |
| Para Conquest: el diseño de cada tipo (§1.3), el asalto de caravanas que falta, y cuánto espera antes de dar a alguien por no conectado | CQ-011 |

## 9. Cómo afecta a lo que ya existe

| Mecánica | Efecto |
|---|---|
| **Bots y batch** | Los ejércitos bot en campaña persiguen hoy cualquier columna enemiga que ven (`bots/cerebro/militar.ts`, `perseguirLoQueVe`), solitarios incluidos (exploradores, mudanzas). Con §1.2 dejan de hacerlo. **El batch cambia a propósito**: con la misma semilla solo debe variar eso. Los solitarios bot no persiguen (no llevan tropa) |
| **Combate con números** (bot contra bot) | Misma matriz de §1.2 en `validarAtaqueAColumna` y en el asedio. Herido sigue siendo «bando perdedor» |
| **Asedio** | Hoy una columna personal puede asediar (`validarAsedio` no mira el tipo). Pasa a ser solo de ejércitos. Los asedios bot ya salen como ejército (`movilizarEjercito`), así que no cambian |
| **Campamentos de bandidos** | Los bots (`sinPlaza.ts`, `militar.ts`) los siguen atacando igual. Lo nuevo es que cualquiera se puede unir, y el evento se narra como PvE |
| **Escolta por ejército** (Doc 5.13.3) | Un solitario ya no puede robar caravanas adjuntas a un ejército: solo otro ejército. Las caravanas sueltas con escolta sin héroe, cualquiera |
| **Unirse en campo y política de unión** (Doc 5.14) | Sin cambios para los ejércitos que ya existen. La formación (§2) usa la misma política y la misma sucesión del Líder |
| **Salir del mundo** (Doc 1.10.6, D66) | Quien está en una formación y sale del mundo la abandona. Quien está en una batalla no sale hasta su salida (§4). El aplazamiento si te persiguen no cambia |
| **Diplomacia** (Doc 2) | Ajuste nuevo de la Facción (§3.2), solo del Rey. El vasallaje cuenta como alianza solo para unirse a batallas, con una función propia; `estanAliadas` no cambia |
| **Anexión y fusión** | `faccionesConBatallaAbierta` mira hoy las columnas y plazas de cada bando; con bandos mezclados tiene que mirar también la Facción de cada participante |
| **XP de Facción y reputación** | La XP de combate va a la Facción de **cada participante**, no a la del bando. La penalización por asediar a un aliado sigue siendo de la Facción titular |
| **Logros por Eras** (`batallas.campoAbierto`, umbral 69) | Cuentan campales y persecuciones |
| **Aedas** (épica `batalla`) | Lee `campo_abierto` con dos Facciones distintas; sigue valiendo con el campo nuevo |
| **Proyección y clientes** | `BatallaVisible` añade las Facciones de cada bando y si admite a otras. Clientes: miniatura con «unirse» y elección de bando, botón «organizar ejército», miniatura de la formación y el ajuste del Rey |
| **Notion** | Resincronizar tras pasar al canon |

## 10. Decisiones del autor (2026-10-06)

- Reparto con Unity (§0); dos clases de columna que nunca combaten entre sí (§1.2).
- Solo un ejército abre un asedio; un solitario puede unirse a uno abierto.
- Convocatoria de 30 s al abrir un asedio (§3.3).
- Campamento de bandidos = evento PvE: todos contra los bandidos.
- Vasallaje = alianza para unirse a batallas.
- Persecución libre: cualquiera, al bando que elija, aunque sea contra un compañero de Facción.
- Ajuste permanente del Rey para admitir a otras Facciones en sus ataques.
- Formar ejército en campo con 3 o más (§2); botín a partes iguales por columna (§5); persecución que se suelta al
  perder de vista o a cubierto (§6).

## 11. Cifras provisionales

`BATALLA.convocatoriaSegundos` (30), `FORMACION_EJERCITO.plazoMinutos` (10) y `minimo` (3) y `PERSECUCION.umbralRecalculo` (30), en
`constants.ts`, con su calibración en `Docs/Mecanicas a balancear.md` §36.

## 12. Cómo quedó implementado, y dónde se apartó del borrador

| Pieza | Dónde | Notas |
|---|---|---|
| Quién combate con quién | `engine/ejercitos.ts` (`exigirMismaClase`, `exigirCaravanaSuelta`, `validarAsedio`) | El bot ya no persigue a quien no es de su clase |
| Persecución | `engine/persecucion.ts`, enganchado en `avanzarEjercitos` | Se persigue lo que se ve **desde la propia columna** (alcance de vista), también al ordenarlo. El destino se guarda en `persiguiendo.destino` |
| Botín | `engine/repartoDeBotin.ts`, `session/resultadoBatalla.ts` | Ronda a ronda: parte igual y lo que sobra pasa a las columnas con sitio |
| Entrada | `session/entradaEnBatalla.ts`, `unirseABatalla` | Vasallaje = alianza solo aquí (`sonAliadasEnBatalla`) |
| Ajuste del Rey | comando `admitirOtrasFacciones`, `Faccion.admiteOtrasEnAtaques` | Solo el Rey; desactivado por defecto |
| Convocatoria | `Batalla.convocatoriaHasta`, `enConvocatoria` | Mientras dura, el ticket no sale en `/pendientes` ni en `/ticket`, y `registrarAsignacion` rechaza. Un tick dura un minuto: en la práctica, hasta el siguiente |
| Salidas | `session/salidasDeBatalla.ts`, `POST /v1/batallas/:id/salidas` | Idempotente. `no_conectado` solo en `asignada`; los demás motivos solo en `en_curso` |
| Canal | `session/canales.ts`, `HubDeDifusion` | El permiso se pregunta a la propia proyección, para que no se separe del mapa. Quien tenga el hogar y la batalla suscritos recibe el evento dos veces, una por canal |
| Formación | `engine/formacion.ts`, `session/comandos/formacion.ts` | `Ejercito.formacion` y `destinoPendiente`; `marcharA` fija el destino una vez |

**Desviaciones del borrador**

- `BattleResult.porHeroe[].derrotado` es **opcional** (sin él, hiere el bando perdedor): así el contrato actual de Conquest sigue valiendo.
- Si se va el Líder de una **formación**, el mando pasa al más antiguo por `separarseDelEjercito`; además hay `cancelarFormacion`, que
  deshace todo y solo es del Líder.
- Las salidas **antes de empezar** no tienen sustituto, y la chatarra de las bajas de quien salió **en curso** no se recoge (el resultado ya
  no incluye sus escuadras). Si se echa en falta, que la salida traiga las bajas y se sumen.
- El batch con la misma semilla (12 Facciones, 30 240 ticks) sale **idéntico** antes y después, pero no demuestra nada sobre guerra: en
  esas tres semanas no hay una sola campaña. La compatibilidad de los bots la cubren los tests; falta medir con un checkpoint de Era II.
