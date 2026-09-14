# NPC de Gobernanza — Facciones que juegan solas

Documento vivo. Recoge el estado actual del mecanismo por el que un jugador puede **ceder una Facción al NPC
de gobernanza** y seguir jugando el resto a mano, dentro de la misma partida. Lo que sigue abierto va al final.

No toca ningún documento numerado de `Docs/`: no es una mecánica del mundo simulado ni una regla que un
jugador pueda encontrarse jugando. Es una herramienta de la interfaz para poblar una partida con adversarios
que se gobiernan solos.

## Principio rector

El NPC **no es parte del motor**. Decide con las funciones públicas de `src/engine/*` exactamente lo mismo que
decidiría un humano pulsando botones, y lo hace desde la capa de aplicación, igual que `GameStore`. El motor no
sabe que existe, no lo llama nunca y no tiene una sola línea escrita para él.

Esto no es una preferencia estética: es lo que garantiza que una Facción NPC y una Facción humana estén
sometidas a las mismas reglas. Si el NPC pudiera pedirle algo especial al motor, dejaría de ser un jugador más
y pasaría a ser una excepción — y cualquier medida de balance sacada de una partida con NPCs dejaría de valer
para una partida entre humanos.

## 1. Qué decide el NPC en un tick

Diez pasos, en este orden, siempre **después** de `avanzarSimulacion` (ver `avanzarNpcGobernanza`,
`src/app/npcGobernanza.ts`). Los pasos 3 y 4 (infraestructura comercial, núcleo militar) no se alcanzan en un
tick dado si el paso 2 (Granjas mínimas) no queda satisfecho — el resto de la lista sigue corriendo igual.

0. **Fundación inicial** — si la Facción todavía no tiene ningún asentamiento, funda uno con 5 fundadores
   propios (`npc-<faccionId>-1..5`) en el mejor punto de UN barrido denso de todo el mapa, paso 25 (por debajo
   de `ZONA_INFLUENCIA.radioInicial` = 30, para no saltarse clusters de recursos entre dos puntos muestreados
   — ver `buscarPosicionFundacionInicialPorDefecto`). Madera (bosque alcanzable) y piedra son OBLIGATORIAS —
   un punto sin ambas queda descartado sin más, a petición del usuario (sin piedra el asentamiento no puede
   tener Cantera y nunca cumple el gate de nivel 2, Doc Fase_0_6); entre los que cumplen las dos, gana el que
   además tenga más de {cobre, estaño, oro, livestock} en el radio inicial — comparado contra TODO el mapa,
   no solo contra el primer grupo de candidatos que aparezca. Si ningún punto del mapa tiene madera y piedra a
   la vez, no se funda ese tick (se reintenta en el siguiente). Sin este paso una Facción recién cedida se
   quedaba inerte para siempre — encontrado por el usuario probando la mecánica recién implementada (§9).
   Solo corre cuando `faccionesIds` está presente: en batch, los escenarios fundan sus asentamientos
   iniciales ellos mismos, antes del bucle de ticks, con su propia estrategia de posicionamiento — repetir la
   fundación aquí cambiaría resultados de simulaciones ya corridas y documentadas en los diarios.
1. **Gobernanza base** — Gobernador + Tesorero (el primer fundador para ambos) y reserva de 150 de madera vía
   `reservaManual`. La reserva del costo completo de una Caravana de Fundación se activa solo desde nivel 2:
   reservarla desde el tick de fundación congelaría la Vivienda/Granja/Leñera que el asentamiento necesita
   para sobrevivir sus primeros ~100 ticks.
2. **Granjas mínimas** (`asegurarGranjasMinimas`, a petición del usuario — ver
   `issues/granjas_no_escalan_con_poblacion.md`) — mientras el asentamiento tenga menos de
   `GRANJAS_MINIMAS_ANTES_DE_COMERCIO` (2) Granjas en cualquier estado (activa, en obra o en cola), intenta
   añadir una más por CONSTRUCCIÓN MANUAL y **no deja pasar al asentamiento a los pasos 3 y 4 este tick**. La
   elección de construcción manual no es cosmética: el disparador automático de Granja del motor
   (`evaluarNecesidades`) respeta la reserva de madera del paso 1 (`reservaManual`) SUMADA a su propia reserva
   dinámica — la construcción manual (`anadirEdificioManualmente`) está exenta de esa suma a propósito (mismo
   criterio de siempre para Mercado/Barracón/Galería, ver comentario junto a la suma en
   `engine/construction.ts`). Es el único camino por el que el NPC puede gastar la madera reservada para
   Mantenimiento en una Granja, en vez de dejarla parada protegiendo un Mantenimiento que de nada sirve si la
   población se muere de hambre antes de llegar a pagarlo. Confirmado en batch: sin este paso, ninguna de 89
   Facciones construía nunca una segunda Granja en 3000 ticks; con él, hasta 14 de 18 alcanzaban nivel 2 hacia
   el tick 200 — pero ver la advertencia de colapso en "Abierto".
3. **Infraestructura comercial** — Mercado y una caravana comercial propia, sin esperar a que la Facción tenga
   2+ asentamientos. Sin esto un trueque se puede pactar pero no entregar.
4. **Núcleo militar** (`asegurarNucleoMilitar`, cierra `issues/nivel_3_inalcanzable_sin_jugador_humano.md`) —
   Barracón primero, Galería de tiro después de que el Barracón ya esté en curso. Sin esto ningún asentamiento
   sin jugador humano podía reunir los 5 edificios que exige el gate de nivel 3
   (`NIVEL_ASENTAMIENTO.requisitos[3]`): la política de auto-construcción que antes los gateaba se retiró sin
   sustituto, y la gobernanza NPC solo añadía Mercado.
5. **Trueque de supervivencia** — si el stock no cubre 20 ticks de Mantenimiento (madera siempre, +piedra
   desde nivel 2, +oro desde nivel 3), busca un socio con excedente real y le propone un trueque, pagando con
   su mejor recurso de sobra; oro como último recurso. No pacta de nuevo si ya tiene ayuda en camino para ese
   recurso, venga del socio que venga.
6. **Trueque de especialización** — se delega en `avanzarAutoComercioSimulado` (motor), que equilibra
   minerales/livestock dentro de una misma Facción con 2+ asentamientos. Ver §6.
7. **Reclutamiento** — cada residente repone su escuadrón. Un solo gate propio del NPC: no recluta si el
   almacén de madera no llega a 150. **El throttle anterior por nivel (1 residente en nivel 1, los 5 desde
   nivel 2) se RETIRÓ** — era un interruptor binario, no gradual, y quedó reemplazado por una regla real del
   MOTOR: `reclutarTropa` (`engine/tropas.ts`) exige que el trigo en almacén cubra una reserva proyectada
   (consumo civil + tropas + la tropa nueva, × 8 ticks) antes de aceptar el reclutamiento — aplica igual al
   NPC y a un jugador humano reclutando a mano. Ver Doc 5 §5.4 para el detalle completo, incluido el
   reordenamiento del tick (población come antes que tropas) que hace que esta reserva tenga sentido.
8. **Ataque a campamentos de bandidos** (`atacarCampamentosCercanos`) — con todos los escuadrones del
   asentamiento a la vez, no de uno en uno: la XP de Facción por combate se multiplica por jugadores distintos
   participantes. Dos gates propios del NPC: no ataca si la nutrición está por debajo de 50
   (`UMBRAL_NUTRICION_ANTES_DE_ATACAR`), ni si sus propios escuadrones ya están heridos por debajo del 60% de
   su tamaño nominal (`SALUD_ESCUADRONES_ANTES_DE_ATACAR`) — no mandar a pelear otra vez a un ejército que
   salió mal parado de la última pelea. **Ninguno de los dos resuelve el colapso masivo medido en batch** —
   ver "Abierto". `ConfigNpcGobernanza.atacarCampamentos` (por defecto `true`) sigue como palanca de
   EXPERIMENTO para desactivar el combate por completo en batch — no pensada para partida real.
9. **Expansión** — lanza una Caravana de Fundación cuando el asentamiento cumple los requisitos reales del
   motor (nivel ≥ 2 y recursos), hacia el primer punto viable de un barrido radial
   (`buscarDestinoFundacionPorDefecto`; quien decide si un punto es viable es el motor). Distinto del paso 0:
   este expande desde un asentamiento ya existente, aquel funda el primero sin ningún origen.

Las prudencias de los pasos 1, 5 y 7 salieron de batches reales, no de teoría — ver
`Diarios_Simulaciones_Batch/Diario_Simulaciones_Batch_10x_Diagnostico_Colapso_Madera.md` y
`…_TruequeSupervivencia.md`.

## 2. Cómo se usa

**Las Facciones NPC las crea el admin** (decisión del usuario, 2026-09-14): pestaña **Facción** del cliente de
administración → *"Nueva Facción NPC"*, o el comando `crearFaccionNpc` (`nombre`, `posicion?`). Antes una
Facción NPC era la de un jugador cedida a la IA con una casilla; eso ya no existe.

- La Facción nace **ya asentada**: su primer asentamiento se funda en el acto (en `posicion`, o donde lo
  elige el paso 0) con 5 **héroes bot** como fundadores, y el primero queda como Rey.
- Es NPC **hasta que se destruye**: no hay retoma manual ni cesión de una Facción de jugador.
- Si se queda sin asentamientos, el paso 0 vuelve a fundar con sus propios bots.

## 3. Dónde vive cada pieza

| Pieza | Archivo | Capa |
|---|---|---|
| Comportamiento del NPC | `src/app/npcGobernanza.ts` | aplicación |
| Fundación inicial + heurística de posición | `fundarAsentamientosIniciales` / `buscarPosicionFundacionInicialPorDefecto` (`src/app/npcGobernanza.ts`) | aplicación |
| Qué Facciones son NPC | `GameState.faccionesNpcIds` (`src/app/gameStore.ts`) | aplicación |
| Creación de la Facción NPC | `crearFaccionNpc` (`src/session/comandos/`), que reutiliza el paso 0 | aplicación |
| Turno del NPC | `GameStore.avanzarFaccionesNpc`, tras `avanzarSimulacion` | aplicación |
| Formulario y distintivos | `#faccion-npc-form` / `renderFaccionesTab` (`cliente/src/main.ts`) | interfaz |

`src/engine/*` y `src/domain/types.ts` no participan. El comportamiento vivía antes en
`simulaciones-batch/npcGobernanza.ts`, fuera de `src/`, y se movió al mudarse de "solo scripts de batch" a
"también partida real"; los scripts de batch lo siguen usando igual (§8).

## 4. Decisiones de diseño, y por qué

### 4.1 El flag vive en la capa de aplicación, no en `Faccion`

`faccionesNpcIds` está en `GameState` (`src/app/gameStore.ts`) y no como campo de `Faccion`
(`src/domain/types.ts`). Quién maneja los mandos de una Facción no es un dato del mundo simulado: dos partidas
con el mismo estado de mundo y distinto reparto humano/NPC son la misma simulación.

La consecuencia práctica es la que importa: `avanzarTick` construye el `EstadoSimulacion` que recibe el motor
campo a campo, y ese campo no está en la lista. El motor no puede leerlo ni aunque alguien lo intentara.

### 4.2 El filtro va por `facciones`, nunca por `asentamientos`

El paso 4 delega en un módulo del motor que recorre todas las Facciones. Para acotarlo a las NPC sin tocarlo se
le pasa una **vista del estado** con `facciones` ya filtrado — el módulo saca de cada Facción sus asentamientos
propios, así que filtrar la lista basta para que ignore por completo a las Facciones humanas. Al salir se
restaura `facciones` entero, porque el módulo devuelve el estado recibido con la vista recortada dentro.

`asentamientos` se le pasa **completo**, y esto es la parte delicada: ese módulo calcula zonas de influencia y
reclamos de yacimientos sobre lo que recibe. Recortarlos le haría ver un mundo vacío alrededor y colocaría
edificios sobre suelo o fuentes que en realidad ya pertenecen a otro. Un filtro mal puesto aquí no da un error:
da un NPC que hace trampa sin que nadie lo note.

Regla general que conviene conservar: **filtrar quién decide, nunca qué existe.**

### 4.3 En un trueque, el socio también tiene que ser NPC

`proponerTrueque` (motor) pacta sin pedir consentimiento al otro lado — en el juego real el trueque nace de un
humano que ya aceptó al pulsar el botón. Sin restricción, un NPC podría comprometer el almacén de un
asentamiento del jugador sin que este lo aprobara.

Mientras no exista aceptación explícita de trueque, un NPC solo pacta con otros NPC. En batch, donde no hay
humano, el filtro no está y el socio puede ser de cualquier Facción, como siempre.

### 4.4 El NPC comparte el contador de ids con el jugador

Los ids que genera el motor llevan tick y número de secuencia (`caravana-comercial-<asentamiento>-<tick>-<n>`).
`GameStore` lleva su propio contador para las acciones del jugador; el NPC arrancaba de 0 en cada tick, lo que
en batch da igual (nadie más crea entidades) pero en partida real puede colisionar. El store le pasa su
contador (`contadorInicial`) y lo adelanta con el que devuelve (`contadorFinal`).

### 4.5 El NPC juega después del motor

Mismo orden que los scripts de batch, y el mismo que un humano: primero el mundo avanza, después se decide
sobre el mundo ya avanzado. Invertirlo haría que el NPC decidiera sobre stocks que el Mantenimiento de ese
mismo tick aún no ha cobrado.

### 4.6 Errores de dominio: se ignoran, no se propagan

Cada paso captura la excepción de dominio que le corresponde (`CargoInvalidoError`,
`ConstruccionManualInvalidaError`, `ReclutamientoInvalidoError`…) y sigue. Un NPC que no puede pagar el Mercado
este tick no es un fallo: lo reintenta al siguiente. Cualquier otra excepción se propaga — un error de
programación no debe quedar enterrado bajo un `catch` silencioso.

## 5. Lo que el NPC NO hace

- No hace diplomacia: ni alianzas, ni vasallajes, ni tributos, ni guerra contra otras Facciones. Solo pelea
  contra campamentos de bandidos.
- No usa el Mercado (órdenes de compra/venta); solo trueque.
- No activa políticas, ni nombra General/Maestro de Obras/Sacerdote.
- No gestiona la cola de construcción a mano: se apoya en la auto-construcción del motor, salvo Mercado,
  Barracón, Galería de tiro y (desde el paso 2) Granja.
- No fusiona ni anexiona Facciones.

## 6. Relación con `SIMULACION_AUTO_COMERCIO`

El paso 6 solo hace algo con `SIMULACION_AUTO_COMERCIO.activo = 1`. El repo lo mantiene en `0` y así se queda:
se enciende en caliente desde la pestaña **"Valores de simulación"** cuando se quiera. Con el flag apagado el
NPC conserva los otros nueve pasos.

Son dos mecanismos distintos que conviene no confundir: ese flag es un NPC de comercio **interno** que actúa
sobre cualquier Facción con 2+ asentamientos, incluida la del jugador; el NPC de gobernanza actúa solo sobre
las Facciones cedidas. Cuando ambos están activos, el segundo llama al primero acotado a sus Facciones (§4.2).

## 7. Persistencia

`faccionesNpcIds` viaja en el archivo exportado (`SimulacionExportada`) y en cada foto del historial. Los
archivos anteriores a esta mecánica se importan sin ninguna Facción NPC. Al importar se descartan los ids cuya
Facción no existe en el archivo: un id huérfano volvería a activarse solo si alguien creara después una Facción
con ese mismo id. La misma limpieza corre tras una anexión o fusión (`sincronizarFaccionesNpc`); la Facción
nueva que nace de una fusión empieza bajo control manual.

## 8. Uso desde los scripts de batch

Sin cambios de comportamiento: omitir `faccionesIds` es gobernar el mundo entero, que es lo que hacen
`scripts/run-batch-sim.ts` y los escenarios temporales. Lo único que cambió fue la ruta del import y que el
buscador de destino de fundación ya no se duplica en cada escenario. Ver `simulaciones-batch/README.md`.

## 9. Verificación

`src/app/__tests__/faccionNpc.test.ts` cubre la garantía que sostiene todo lo demás:

1. Con dos Facciones y una cedida: la NPC acaba con Gobernador, Tesorero y reserva; la del jugador queda sin
   cargos, sin reserva, sin escuadrones, sin caravanas propias, y ningún acuerdo de trueque la toca.
2. La cesión y la retoma funcionan a mitad de partida, y retomar no deshace lo hecho.
3. La marca sobrevive a exportar/importar.

Probado además en el navegador: una Facción cedida, asentamiento fundado a mano, 30 ticks — el NPC nombró
cargos, completó el Mercado y destruyó un campamento de bandidos, con los eventos marcados `[NPC]` en el
registro.

**Hallazgo real #1 (post-implementación):** el usuario probó el flujo más simple — crear una Facción,
marcarla NPC sin fundar nada, avanzar 20 ticks — y el NPC no hizo nada en absoluto, porque el paso 0
(fundación inicial) todavía no existía y el NPC solo sabía gobernar un asentamiento ya existente. Se añadió
el paso 0 (§1) y el test 4 de `faccionNpc.test.ts` reproduce exactamente ese escenario: crea una Facción, la
marca NPC sin fundar nada, avanza 10 ticks, y comprueba que aparece un asentamiento propio con Gobernador.
Confirmado también en el navegador: `[t1] [NPC] NPC Test 2 funda su asentamiento inicial
asentamiento-0-300-600.`

**Hallazgo real #2 (a petición del usuario, mismo día):** la primera versión del paso 0 se quedaba con el
PRIMER punto viable del barrido, sin exigir piedra ni puntuar otros minerales — podía fundar en un sitio con
madera pero sin ningún extractor mineral cerca, condenando al asentamiento a no poder subir de nivel 2 nunca.
Se cambió a la heurística de §1: piedra obligatoria, bonus por minerales adicionales, mismo criterio de
puntuación que ya usaba `scripts/run-batch-sim.ts` para su propio posicionamiento inicial. El test 5 de
`faccionNpc.test.ts` verifica directamente contra la API pública del mapa (`hayBosqueEnRadio`,
`nodosEnRadio`) que el punto elegido tiene madera y piedra en el radio inicial real del asentamiento fundado
— no contra la lógica interna de la heurística, para no probar la implementación con la implementación.

**Hallazgo real #3 (a petición del usuario, mismo día):** con piedra ya obligatoria, el NPC seguía fundando
en sitios con SOLO madera+piedra aunque el mapa tuviera de sobra sitios con algún mineral extra alcanzable
cerca — el bug estaba en el barrido de grueso a fino: se quedaba con el mejor candidato del PRIMER paso de
rejilla donde encontrara alguno con piedra (300, 150, 75 o 40), y esos pasos son varias veces mayores que
`ZONA_INFLUENCIA.radioInicial` (30) — entre dos puntos muestreados con ese paso puede caber sin ser visto un
clúster entero con madera + piedra + otro mineral. Al ser el primer paso que encontraba ALGO, nunca llegaba a
comparar contra el resto del mapa. Se sustituyó por un único barrido denso (paso 25, por debajo del radio,
§1) que recorre el mapa entero y se queda con el mejor candidato visto hasta el momento, comparando cada
punto válido contra el mejor anterior en vez de conformarse con el primer grupo que aparezca. El test 6 de
`faccionNpc.test.ts` lo prueba contra el mundo real (seed 1, el de `new GameStore()`): confirma primero, con
un barrido de referencia independiente, que ese mundo tiene sitios con mineral extra, y después comprueba que
el NPC efectivamente eligió uno de ellos y no uno con bonus 0.

## 10. Cómo eliminar el mecanismo por completo

Mismo patrón que documenta `SIMULACION_AUTO_COMERCIO` en `constants.ts`: borrar `src/app/npcGobernanza.ts` y
su test, quitar `faccionesNpcIds` del estado junto con `crearFaccionNpc`, `avanzarFaccionesNpc` y
`sincronizarFaccionesNpc`, y el formulario "Nueva Facción NPC" del cliente. El motor no hay que tocarlo, porque
nunca se tocó.

## Abierto

- **RESUELTO — el gate de reclutamiento binario por nivel.** El "1 residente en nivel 1, los 5 de golpe desde
  nivel 2" del paso 7 se retiró: reemplazado por una reserva de trigo real en el motor
  (`reclutarTropa`, `engine/tropas.ts`) más el reordenamiento del tick (población come antes que tropas, Doc
  5 §5.4) — ver el detalle de diseño en la respuesta al usuario que llevó a este cambio, resumida en el paso 7
  de arriba. Verificado en batch que corta el reclutamiento descontrolado (`reclutamientosAcumulados` pasó de
  subir sin parar hasta 442 a estancarse en ~67) y que los asentamientos que sobreviven la transición a nivel
  2 dejan de recaer — antes el nivel 2 estable oscilaba (14→1), ahora se asienta y se mantiene (6, estable
  tick 300→600).
- **RESUELTO en su mayor parte — el colapso masivo en la transición a nivel 2 SÍ era mayormente reclutamiento,
  solo que de un recurso distinto al que ya se había arreglado.** La reserva de trigo (punto anterior) protege
  el CONSUMO proyectado, pero nada impedía que `reclutarTropa` sacara pesants que ya estaban cubriendo
  `trabajadoresRequeridos` de Granja/Cantera/Leñera/minas/Corral (`ratioManoObra`,
  `engine/asentamientoQuery.ts`) — la UI ya mostraba ese número como "Pool de pesants para reclutamiento"
  (`manoObraInfo.excedente`, `main.ts`) sin que el motor lo hiciera cumplir. Reportado por el usuario tras
  probar en la interfaz. Cambio en el MOTOR (`poblacionDisponibleParaReclutar`, `engine/asentamientoQuery.ts`;
  `reclutarTropa`, `engine/tropas.ts` — aplica igual a pesants y a artesanos, y a reclutamiento manual y NPC):
  el pool de reclutamiento pasa de ser la población total a población MENOS la ya requerida por la mano de obra
  vigente. Medido en batch, mismas 20 Facciones de prueba: **colapsados 11/20 → 3/20**, `nivelFaccionMax` 3→4,
  `conGateNivel2Cumplido` 6/20→13/20, tropas vivas de un puñado estancado (3) a decenas fluctuando con combate
  real (35-52). Detalle completo, con la tabla de la corrida:
  `issues/granjas_no_escalan_con_poblacion.md`, sección "Tercera continuación".
- **ABIERTO, pero ya minoritario — el daño de combate front-loaded sigue sin gate.** Con el reclutamiento de
  mano de obra ya arreglado, quedan 3/20 asentamientos colapsando en la misma corrida — consistentes con el
  mecanismo de daño front-loaded documentado antes de este arreglo: se probaron dos gates de ataque (nutrición,
  salud de escuadrones — paso 8 de arriba) en varios umbrales, incluidos casos extremos (nutrición 95, salud
  0.95 — casi exigiendo curación completa antes de volver a pelear), y ninguno cambió el resultado frente a no
  tener gate — solo un umbral IMPOSIBLE de cumplir (nunca atacar) lo cambiaba, y eso no es un gate usable, es
  apagar la función. La razón, confirmada entonces: el daño no viene de mandar un ejército YA HERIDO a pelear
  otra vez (eso es justo lo que el gate de salud evita, y no cambió nada) — viene de que el PRIMER combate de
  cada asentamiento, casi simultáneo desde el tick 1, ya inflige de golpe el daño agregado que dispara el
  colapso. Un gate reactivo no tiene nada a lo que reaccionar en ese primer ataque — la salud del escuadrón es
  1.0 hasta que pierde su primera pelea. Probablemente hace falta pausar el PRIMER combate hasta que el
  asentamiento tenga cierta madurez (nivel, población, ticks desde la fundación), no seguir ajustando el
  umbral de un gate reactivo — sin implementar todavía, decisión pendiente del usuario. Detalle completo con
  los números de cada prueba en el comentario de `UMBRAL_NUTRICION_ANTES_DE_ATACAR` /
  `SALUD_ESCUADRONES_ANTES_DE_ATACAR`, `src/app/npcGobernanza.ts`.
- **Sin calibrar por simulación:** el umbral de nutrición (50) y de salud de escuadrones (0.6) de los dos
  gates de ataque de arriba son valores razonables de partida, no medidos — a diferencia del horizonte de 8
  ticks de la reserva de trigo, que reutiliza una constante ya existente (`RESERVA_CONSTRUCCION`).
- **Aceptación de trueque.** Cuando exista, la restricción de §4.3 puede levantarse: un NPC podría proponer a
  un humano y este aceptar o rechazar. Es el desbloqueo natural de "comerciar con los NPC".
- **Diplomacia y guerra NPC** (§5). Hoy dos Facciones NPC vecinas se ignoran mutuamente para siempre.
- **Personalidades.** Todos los NPC juegan idéntico: mismas reservas, mismos umbrales, misma agresividad. Un
  perfil por Facción (expansionista, mercantil, militarista) sería el primer paso para que una partida con
  varias Facciones NPC no se lea como una sola repetida.
- **Coste por tick.** Con muchas Facciones NPC el paso 3 recorre todos los asentamientos contra todos; en
  batch de 100 Facciones aguanta, pero no está medido como parte del bucle de UI.
