# El reloj de mundo y los eventos

Cómo avanza el mundo en la reconstrucción: sin tick global (la economía de cada plaza da un paso de un minuto, ver la nota del 2026-10-22). Este documento **sustituye** a `Tick.md` (el «cómo funcionaba» del servidor TypeScript, con su cadena de 24 pasos por minuto, vive en `typescript-final:Docs/Mechanics/Tick.md`). Fija el reloj, la cola de eventos, cómo cada paso de la antigua cadena pasa a ser un **ritmo**, un **evento programado** o un **muestreo**, qué orden se conserva cuando varios eventos caen en el mismo instante y cómo se usa el generador aleatorio. Diseño de fondo: [`Arquitectura/13_Mundo_Reactivo_Rust.md`](../Arquitectura/13_Mundo_Reactivo_Rust.md).

- **Estado (2026-10-13):** reescritura de la fase 1; los detalles de cada regla viven en el documento de su sistema (también en reescritura).
- **Canon:** Doc 5.15.1 (qué bloquea una batalla), Doc 1.10.6 (presencia).
- **Lo que no cambia:** las reglas de juego. Cambia *cuándo* se evalúan.

> **Implementado en Rust (2026-10-20, fase 4B-1), en el reloj de los héroes.** La prioridad **1** ya la usa la salida del mundo de quien se desconectó (`Carga::SalidaDelMundo`, `PRIO_SALIDA`, §7). El hambre de la guarnición **no** es un evento: es parte del paso de minuto de la plaza (prioridad 2), que ya existe, justo después de la población (la población come antes): una plaza sin residentes no hace nada y no hay evento por héroe ni por escuadra. Los fines de herido, de petición de unión y de formación siguen sin evento (el herido se lee al mirar).

> **Implementado en Rust (2026-10-20, fase 4B-2), las columnas en el reloj.** Cuatro plazos nuevos, todos en la cola con generación (`Carga::Militar(SucesoMilitar)`; la clave es la de la entidad, así que contestar, deshacer o cambiar de ritmo la sube y el evento viejo se descarta al salir):
>
> | Evento | Prioridad | Entidad (etiqueta sobre `1<<56`) | Cuándo | Qué hace |
> |---|---|---|---|---|
> | `Desercion` | 3 (`PRIO_UMBRAL`, el consumo de la tropa) | la columna | la próxima fase de deserción de una escuadra con la moral a 0 (la de cada escuadra es el instante en que su moral llega a 0 y luego cada minuto) | deserta `ceil(5 %)` de cada escuadra cuya fase cae ahora, re-ancla el consumo y se reprograma |
> | `CaducaFormacion` | 8 (`PRIO_LLEGADA`) | 6: la formación | `formacion_expira_en` | deshace la formación (`columna.formacion_disuelta`) |
> | `CaducaPeticion { columna }` | 8 | 7: el solicitante | `pedido_en + vidaPeticionUnion` (10 s) | retira la petición (`columna.peticion_caducada`) |
> | `CaducaPeticionDeConvocatoria { convocatoria }` | 8 | 8: el solicitante | ídem | retira la petición (`convocatoria.peticion_caducada`) |
>
> La **deserción es el único evento nuevo ligado al hambre**, y solo existe mientras la columna tiene hambre y soldados: una columna quieta con ración, o sin soldados, no programa nada, y los víveres a 0 siguen siendo el `Umbral(0)` de siempre (prioridad 3). La moral de la tropa es un ritmo que se lee al mirar, y el de la columna con hambre se ancla una vez (`Escuadra::con_racion` no re-ancla si el ritmo ya es el buscado), sin evento por minuto. Una **convocatoria** no tiene plazo; **salirse del mundo** sigue siendo el evento de prioridad 1 de la 4B-1, ahora también para el héroe de una columna. Un evento con la generación vencida **permanece en la cola hasta que su instante pasa** (se descarta al salir; no hay borrado activo): es una fila de paridad.

> **Implementado en Rust (2026-10-20, fase 4C), los encuentros en el reloj.** Dos cargas nuevas de la cola, ambas de prioridad 8 y ligadas a la generación de su entidad: `Carga::Encuentro { otra, otra_generacion }` (la entidad es la columna o la caravana cuyo plan acaba de cambiar; el evento muere si cambia el plan de cualquiera de las dos) y `SucesoMilitar::PresaFueraDeVista { presa, presa_generacion }` (la entidad es quien persigue). Los dos son eventos **exactos plan contra plan**, no muestreos, y se programan solo cuando una cosa cambia de plan o entra en el mapa: una columna quieta y sin perseguir a nadie no programa nada. El hambre y el resto no cambian. Un evento invalidado sigue en la cola hasta que pasa su instante (fila de paridad de 4B-2).

> **Implementado en Rust (2026-10-20, fase 4D), el combate en el reloj.** Una carga nueva de la cola (`Carga::Combate(SucesoCombate)`, etiqueta propia por entidad sobre `1<<56`: 9 el campamento de bandidos, 10 la conquista de una plaza, 11 el anillo de un campamento de mercenarios, 12 la guarnición de un pueblo) y **la prioridad 7 (`PRIO_BANDIDOS`) por fin en uso**:
>
> | Evento | Prioridad | Entidad | Cuándo | Qué hace |
> |---|---|---|---|---|
> | `BandidosDeLaPlaza` | 7 | la plaza (su generación: la ruina lo descarta) | al fundarla y al vencer las 6 h de reaparición (`respawnMinutos`) del campamento destruido | pone el campamento de la plaza en el bosque libre más cercano (una tirada de nivel, y ninguna si no hay bosque) |
> | `BandidosDelAnillo` | 7 | `entidad_anillo(campamento)` | al nacer la partida (una por campamento de mercenarios), a `reaparicionMinutos` tras crear uno o si no hubo sitio, y al destruirse uno del anillo | crea un campamento en el anillo si faltan según los residentes sin plaza; si no falta ninguno no se vuelve a programar |
> | acoso | 7 | la caravana (o la columna con caravanas adjuntas) o el campamento: el mismo `Carga::Encuentro` de la 4C, con prioridad 7 | el primer instante en que el plan de la caravana queda a ≤ `radioAtaqueCaravana` (7) de un campamento | una tirada contra la defensa; ganar la hace desaparecer. Con servidores de batalla y una caravana adjunta a una columna con algún héroe sano, abre una batalla de caravana en Unity en lugar de tirar |
> | `FinProteccion` / `FinOcupacion` | 2 | `entidad_conquista(plaza)` (una conquista nueva descarta los de la anterior) | `protegida_hasta` (+1 día, tras la conquista y **al fundar**: `zonas::fundar` cita solo `FinProteccion`) y `ocupada_hasta` (+90 min) | limpian el campo de la plaza y lo cuentan; la ocupación devuelve su recaudación, su crecimiento y su mantenimiento |
> | `ReponeGuarnicion` | 3 | `entidad_guarnicion(pueblo)` | 12 min (`minutosPorMinero`) tras el combate que dejó incompleta la guarnición de serie y, mientras falte alguno, tras el minero anterior | vuelve un minero, con la experiencia de la escuadra; si aún falta alguno, cita el siguiente |
>
> Ninguno es recurrente por entidad ociosa: cada uno se programa cuando hay algo que esperar y se detiene solo (con los anillos y las plazas satisfechos la cola no guarda ninguna cita de combate). Mientras una plaza está ocupada, su paso de minuto pasa al `Exterior` los factores de la ocupación (`felicidad_extra` = `factorCrecimiento`, `recaudacion_extra` = `factorRecaudacion`) y la economía congela su mantenimiento (`ocupado`). Los plazos de la conquista y el de la guarnición comparten instante con el paso de minuto de la plaza (prioridad 2) y el consumo (3): dentro del mismo instante y prioridad manda la secuencia de la cola, no una regla (fila de paridad). El orden del generador aleatorio es ya parte de las reglas: la aparición de un campamento gasta una tirada de nivel (más dos por intento de punto en un anillo), un combate las suyas, y lo que no tira no gasta.

> **Implementado en Rust (2026-10-21, fase 4E), las batallas de Unity en el reloj.** Una carga nueva de la cola (`Carga::Batalla(SucesoBatalla)`, etiqueta propia por entidad: 13 la batalla) y la prioridad **1** (`PRIO_BATALLA`, la misma fila que la salida del mundo) para los tres plazos de §6 y de [`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §3.3:
>
> | Evento | Prioridad | Entidad | Cuándo | Qué hace |
> |---|---|---|---|---|
> | `FinConvocatoria` | 1 | la batalla (su generación) | `convocatoria_hasta` (solo un asedio, +30 s) | con servidores de batalla, cuenta `batalla.convocatoria_cerrada`: el ticket ya puede publicarse y asignarse; sin ellos (M8, 2026-10-24), resuelve el asedio con números con todas las columnas atacantes y deja la batalla `aplicada` |
> | `Vence` | 1 | la batalla | `expira_en` (cambia con cada estado: +5 min de asignación, +5 de inicio, duración + 5 de margen) | pasa a `fallida` sin castigo: suelta candados y devuelve al mundo lo apartado |
> | `Retira` | 1 | la batalla | `cerrada_en` + 60 min | la saca del estado (`batalla.retirada`) |
>
> Cada cambio de estado sube la generación de la batalla y reprograma lo que siga haciendo falta (una activa tiene en la cola una o dos citas, una cerrada una, y ninguna entidad ociosa programa nada). Como `Vence` es de prioridad 1, **una batalla que vence en un instante ya no bloquea en ese mismo instante**. Una columna apartada **no come** (`militar::racion::Estado::Apartada`: consumo 0) y su plan queda en pausa (`Ejercito::plan_en_pausa`): al volver lo retoma desde donde se quedó, desplazado lo que duró la batalla y escalado a la velocidad que le dejen las bajas. Quien combata, dentro de una columna o de la plaza asediada, no sale del mundo hasta que acaba.

> **Implementado en Rust (2026-10-22), la economía por minuto con fines exactos** ([`Economia_Por_Minuto_Definicion.md`](../../Consideraciones/Economia_Por_Minuto_Definicion.md), que corrige el «sin tick, tampoco para la economía» de este documento y del 13). El paso de un minuto de cada plaza (`Carga::Minuto`, prioridad 2, recurrente con la fase de su fundación) **se queda**: producción, recetas, consumo de comida, hambre, crecimiento, recaudación, mantenimiento, guarnición, pueblos y evaluación de la auto-construcción. Lo que el jugador nota sale del paso y tiene su **cita propia** en la cola, con la generación de la plaza (la ruina las descarta), una por cosa en curso, programada al empezar y borrada al cumplirse (ninguna recurrente, ninguna por plaza ociosa):
>
> | Evento | Prioridad | Entidad | Cuándo | Qué hace |
> |---|---|---|---|---|
> | `FinDeObras` (`PRIO_FIN_DE_OBRAS`) | **1** | la plaza | el `completaEn` de cada obra, mejora o ascenso que empieza | `Asentamiento::cumplir_fines`: activa lo que vence, libera la cuadrilla y **arranca en ese mismo instante** la siguiente obra de la cola; el núcleo agranda la zona uno por edificio completado |
> | `CaducaPolitica` (`PRIO_CADUCA_POLITICA`) | **3** | la plaza | el `expiraEn` de cada política activada | `Asentamiento::caducar_politicas` |
>
> Una **obra encargada a mano** arranca en el instante de la orden si hay cuadrilla libre (`ObraIniciada` sale en la misma respuesta). **Orden dentro de un mismo instante:** el fin de obra, de mejora y de ascenso (1) va **antes** del paso de minuto de la plaza (2), para que el edificio terminado y el nivel nuevo cuenten en ese minuto, como ya hacía `completar_obras`, que iba primero; la caducidad de política (3) va **después**, para que una política de `d` minutos se aplique en `d` pasos (el último minuto de una política todavía la aplica, como antes). Comparte fila con las batallas y las salidas del mundo (1) y con el consumo de tropa (3) sin interactuar con ellos. Las citas son idempotentes (una que llega con la plaza ya cambiada no encuentra nada) y el batch comprueba que todo lo que está en curso tiene la suya (`inv_fines`). La foto es `BAN14`.

> **Implementado en Rust (2026-10-22, fase 4F), las avanzadas en el reloj.** Tres plazos de combate nuevos y ninguno recurrente. (En la 4F el fin de la obra de mapa reutilizaba la cita de la plaza madre; desde las decisiones del mismo día, foto `BAN16`, tiene la suya, ligada a la avanzada, para poder cancelarla.)
>
> | Evento | Prioridad | Entidad | Cuándo | Qué hace |
> |---|---|---|---|---|
> | `FinObraAvanzada` (`SucesoCombate`) | 1 (`PRIO_FIN_DE_OBRAS`) | `entidad_obra_de_avanzada(id)` | el `completaEn` de la obra de una avanzada (construirla, mejorarla o repararla), que arranca al entregar el último material | la plaza madre cumple sus fines (`fines_de_plaza`): `Asentamiento::cumplir_fines` también cumple las avanzadas: la primera obra la deja operativa (y la apunta en la rejilla de encuentros), la mejora sube de etapa y de almacén, la reparación la saca de ruinas. Antes del paso de minuto del mismo instante, como el resto de fines. **Cancelar la obra** (`CancelarObraDeAvanzada`), destruir la avanzada, derribarla un ataque o caer su madre **suben la generación de esa entidad** y la cita no se cumple |
> | `FinInvulnerabilidad` | 2 | `entidad_avanzada(id)` | `invulnerable_hasta` de un fuerte que acaba de caer en ruinas (+1 día) | quita la invulnerabilidad (`avanzada.invulnerabilidad_terminada`); es idempotente |
> | `ReponeAvanzada` | 3 | `entidad_avanzada(id)` | `repone_en`: la vuelta del siguiente soldado, `reposicionMinutos` / soldados de la guarnición completa después de la baja o del soldado anterior (entera en 6 h desde 0) | vuelve un soldado a la primera escuadra mermada y cita el siguiente; completa, rehace la composición vigente y cuenta `avanzada.guarnicion_repuesta` |
>
> Una avanzada ociosa (operativa, sin guarnición mermada ni ruinas) **no programa nada**: la cola no guarda ninguna cita por ella. La avanzada no sube la generación de `entidad_avanzada` al cambiar de estado: sus plazos son idempotentes y comprueban el estado al vencer (una cita que llega con la avanzada ya destruida, reparada o rehecha no encuentra nada que cumplir). Los encuentros de las columnas con una avanzada son los `Carga::Encuentro` de prioridad 8 de siempre (`avanzada.encuentro`). El mantenimiento de la madre cuenta las avanzadas operativas en su paso de minuto, sin evento propio; la ración de la guarnición de jugadores, también (come en el paso de minuto de la madre).

> **Implementado en Rust (2026-10-22, murallas, segunda tanda), la obra de un recinto en el reloj.** Una cita por **celda**, que reutiliza `Carga::FinDeObras` (prioridad **1**, antes del paso de minuto de la plaza del mismo instante) y la generación de la plaza (la ruina las descarta); no hay carga ni prioridad nuevas.
>
> | Evento | Prioridad | Entidad | Cuándo | Qué hace |
> |---|---|---|---|---|
> | `FinDeObras` por una celda de muralla | 1 (`PRIO_FIN_DE_OBRAS`) | la plaza | `siguiente_celda_en` del recinto: la primera celda se levanta **en el instante de la orden** (`ComprometerRecinto`, `MejorarRecinto`) y cada celda deja la cita de la siguiente a `minutosPorCelda[nivel] × factorTiempoMuralla` | `Asentamiento::cumplir_fines` → `levantar_recintos`: paga y levanta **una celda** de cada recinto en obra, dañado o con mejora que ya toque; la cita se programa a partir de `Evento::RecintoCelda { siguiente_en }` en `anotar_sucesos`, como las demás |
>
> Lo que no sale como una cita es el **reintento**: si al llegar la cita faltan materiales, o pagar dejaría un recurso por debajo de la reserva, no se levanta nada **ni se programa otra cita** (un recinto parado no deja nada en la cola, ni siquiera una cita por plaza ociosa), y el **paso de minuto** de la plaza lo reintenta después de la auto-construcción y con la misma reserva (Paso 8 de la cadena, `Economia_Construccion.md` §1); la cadencia de la celda siguiente cuenta desde el instante en que se levantó de verdad. Un recinto cerrado o sin mejora no programa nada. Ninguna cita es recurrente, y el batch las vigila (`inv_fines` incluye la celda que espera cada recinto). El orden dentro de un mismo instante es el de siempre: la celda (1) va antes del paso de minuto (2).

## 1. Tiempo

- **Reloj de mundo** = reloj de pared × factor (×1 en producción; ×N para pruebas y para el batch). No hay tick y no existe la unidad «minuto de mundo» como paso: sigue habiendo plazos declarados en minutos, horas y días, y se miden en instantes.
- **Instante** = milisegundos de mundo desde la época (`2026-01-01T00:00:00.000Z`), en entero de 64 bits. Todos los `*En`, `*Hasta` y `expiraEn` son instantes. «Vence» significa `instante ≥ vencimiento`.
- El reloj de pared **no entra en el estado**: dos partidas con la misma semilla y las mismas órdenes en los mismos instantes de mundo dan el mismo mundo.
- **Modo acelerado**: el batch procesa la cola sin esperar al reloj de pared; el factor es solo una forma de la misma regla.
- **Parar el reloj** (`ControlarReloj` de administración: pausa o factor 0) es **solo para mantenimiento o como último recurso de la administración, y en pruebas** (decisión del autor, 2026-10-24): el juego en vivo no se para. Al pararlo o reanudarlo se avisa a todas las sesiones. Con el reloj parado no hay muestreo de visión que esperar: lo que cambia una orden se reconcilia al momento en todas las vistas.

## 2. La cola de eventos

- Cada evento lleva `(instante, secuencia, entidad, generación, tipo)`. Se aplican por `(instante, secuencia)`; con igual entrada, igual mundo.
- **Invalidación perezosa**: cuando una entidad cambia de plan o de ritmo sube su generación y los eventos viejos se descartan al salir de la cola.
- **Desempate del mismo instante**: la antigua cadena fijaba un orden de pasos dentro del minuto y varias reglas dependen de él. Se conserva como **prioridad de tipo** para eventos con el mismo instante, de menor a mayor número:

| Prioridad | Qué | Por qué en ese orden (regla que se conserva) |
|---|---|---|
| 1 | Cierre de batallas vencidas, salidas del mundo y **fin de obras, de mejoras y de ascensos de las plazas, de cada celda de sus murallas y de las obras de mapa de sus avanzadas** | una batalla que vence no bloquea en ese mismo instante; el edificio terminado y el nivel nuevo cuentan en el paso de minuto del mismo instante |
| 2 | **Paso de minuto de la plaza** (producción, comida y hambre, crecimiento, recaudación, mantenimiento y ruinas, en ese orden y de forma atómica); fin de la protección y de la ocupación; fin de la invulnerabilidad de un fuerte en ruinas | la población come **antes** que las tropas; la recaudación usa la población ya crecida; el oro recaudado paga el mantenimiento |
| 3 | Consumo de tropa, **caducidad de políticas** y reposición gratuita de la guarnición de serie de pueblos y avanzadas | la población (2) come antes que las tropas; una política todavía se aplica en su último minuto |
| 4 | (dentro de la fila 2) crecimiento de población y recaudación de oro | no tienen evento propio: van en el paso de minuto de la plaza |
| 5 | (dentro de la fila 2) mantenimiento y ruinas | una ruina deja de existir para el comercio, los bandidos y los ejércitos en ese instante |
| 6 | Comercio, trueques, órdenes de mercado, caravanas | |
| 7 | Regeneración, aparición y ataques de bandidos | los bandidos ven a los ejércitos tal como estaban antes de moverse |
| 8 | Llegadas y encuentros de columnas | |
| 9 | Diplomacia, tributos, experiencia, reputación | |
| 10 | Títulos, tecnología, Aedas, caravanas de fundación | |

- **Persecución, emboscada y llegada a una puerta** son eventos exactos (Arquitectura/13 §3); la visión y la niebla se **muestrean** (§4).

## 3. Cada paso de la antigua cadena, en el mundo reactivo

Tres clases. **Ritmo**: el estado guarda `(valor en t₀, ritmo)` y el valor se calcula al leer. **Evento**: algo ocurre en un instante conocido o en el instante en que un valor cruza un umbral. **Muestreo**: se recalcula cada pocos cientos de ms para lo informativo.

### 3.1 Por asentamiento

| Antes (por minuto) | Ahora | Eventos que programa |
|---|---|---|
| Recetas, producción, extracción | **Paso de minuto** de la plaza (decidido el 2026-10-22; el «ritmo» continuo del 2026-10-13 se descartó) | — |
| Obras y mejoras | **Evento** con su instante de fin (`FinDeObras`); la obra encargada a mano arranca al instante de la orden y la cola arranca al liberarse la cuadrilla | obra terminada, mejora terminada |
| Auto-construcción (decidir qué se encola) | Se evalúa en el **paso de minuto** | — |
| Políticas expiradas | **Evento** a su caducidad (`CaducaPolitica`) | política caduca |
| Obra de ascenso | **Evento** a su fin (`FinDeObras`) | ascenso completado |
| Nutrición y consumo de comida | **Paso de minuto** (el hambre y su recuperación, §9.2 de Economia_Construccion) | — |
| Mantenimiento de tropas (ración de la guarnición) | **Ritmo** de consumo; moral y deserción por umbrales (la guarnición come en el paso de la plaza) | despensa a 0, deserción |
| Crecimiento de población | **Paso de minuto**, siempre entera (resto oculto por clase) | — |
| Recaudación de oro | **Paso de minuto** (impuesto × población entera por clase) | — |
| Mantenimiento (costes, medidor, bajar nivel, ruina) | **Paso de minuto** | ruina |
| Fin de ocupación y de protección | **Evento** a su instante | ocupación terminada |

### 3.2 Global

| Antes | Ahora |
|---|---|
| Ruina y su cascada (reubicar residentes, derrota, campamento de bandidos, trueques, caravanas) | **Evento** de ruina que ejecuta la cascada completa en el orden de siempre |
| Comercio: caravanas, trueques, órdenes de mercado, asignación automática | Caravanas = **planes** con evento de llegada; trueques y órdenes = **eventos** a su caducidad; la asignación automática se reevalúa cuando nace un trueque, queda libre una caravana o cambia el almacén de un lado |
| Regeneración de yacimientos | **Evento** al fin del plazo de cada nodo |
| Aparición de bandidos | **Evento** al fin del plazo de reaparición de cada plaza |
| Ataques de bandidos a caravanas | **Encuentro exacto** (el plan de la caravana cruza el radio del campamento) |
| Ejércitos: moverse, hambre, repostar, perseguir, combate, asedio | **Plan** por columna; hambre = ritmo de víveres con evento de fin; combate y asedio = eventos de encuentro |
| Formaciones vencidas | **Evento** a `expiraEn` |
| Vasallos liberados de un señor desarmado; tributos | El desarme del señor y la vasalla conquistada ocurren en el instante en que una Facción pierde su última plaza; el tributo se paga en el **paso de minuto de cada plaza vasalla** (Q3, en Rust desde 5D, 2026-10-24), al final de su paso, sin eventos propios en la cola |
| Experiencia de Facción, reputación | Se aplican **al ocurrir** el hecho que las produce; el decaimiento de la reputación es un ritmo |
| Reposición de mercados de mercenarios | **Evento** agendado (ya era una cita) |
| Títulos | **Muestreo** o reevaluación cuando cambia lo que mira (Facciones, plazas, relaciones). **Implementado en Rust (2026-10-25, 5F):** un solo plazo **global** (`Carga::Titulos`, prioridad 10) a `TITULOS.cadenciaMinutos` de lo primero que pasa (orden aceptada o evento de la cola que no sea él mismo) sin otro pendiente; al vencer recalcula todo y no se rearma solo (Diplomacia_Facciones §7) |
| Tecnología, logros, Aedas, residentes | Los contadores suben al ocurrir el hecho; la aparición se evalúa al ocurrir lo que mira su hito (Sistema 6, plan §5.1) y la Era al fijar un logro o en la cita de su plazo; los Aedas itinerantes, eventos programados |
| Caravanas de fundación (vuelve sola, se desarma, caduca) | **Planes** y eventos de caducidad |
| Memoria de la niebla y exploración | **Muestreo** (§4) |

## 4. Lo informativo se muestrea

Qué ve cada Facción, la niebla explorada, la memoria de plazas y la exploración personal se recalculan en paralelo cada **250 ms** (valor inicial) sobre la rejilla espacio-temporal, por clave de visión. *(En Rust la memoria se graba al cerrar cada versión, Niebla_De_Guerra §4.1, y el muestreo de 250 ms es el de las vistas de las sesiones: tras una orden se pone al día al momento solo la vista de quien la dio y las demás en el muestreo siguiente; decisión del autor, 2026-10-24.)* No hace falta el instante exacto en que algo entra en la vista. Lo que sí tiene efecto de juego (una columna entra en la zona de otra, alcanza una puerta, intercepta una caravana) es un evento exacto.

## 5. Una orden (la antigua transacción)

Toda orden pasa por el núcleo, de un solo hilo y en orden (Arquitectura/13 §4):

1. Se valida la forma (en el borde) y, en el núcleo, la autorización y las reglas, con el estado **en el instante de la orden**. Una orden a mitad de camino parte de la posición real en ese instante.
2. Si se acepta, produce cambios de estado, **eventos de dominio** y, si altera un plan o un ritmo, la reprogramación de los eventos de esa entidad.
3. Si se rechaza o no cambia nada, **no deja rastro**: ni versión, ni dados, ni ids gastados.
4. La **versión** sube en un único sitio, al aceptar un cambio. Cada evento de dominio lleva la versión que produjo.
5. La respuesta es **causal**: dice qué versión y qué eventos produjo la orden (contrato 05).

Lo que se estampa en toda orden aceptada (sin cambios respecto al motor anterior): las épicas de los Aedas residentes con los eventos del hecho; la crónica narrada; la suma de logros del servidor y de lo comerciado entre Facciones para los mercados de mercenarios.

## 6. Batallas de Unity: qué bloquea

Una columna o una caravana que participa en una batalla activa (`convocando`, `asignada` o `en_curso`) y dentro de plazo queda **apartada**: su plan queda en pausa, no come, no reposta, no es perseguida ni atacada y no se la puede tocar con órdenes. Al acabar la batalla vuelve al mundo con un plan nuevo desde donde esté. Una batalla cuyo plazo vence (`instante ≥ expiraEn`) pasa a `fallida` sin que nadie pierda nada (evento de prioridad 1). Un asentamiento asediado **sigue produciendo**: solo se bloquean sus órdenes y las entradas y salidas. Detalle del ciclo: [`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md).

## 7. Desconexión

`desconectarse` programa la salida a `ahora + PRESENCIA.retardoDesconexionMs` (2 min 30 s). Al vencer el evento sale del mundo quien cumple **todo**: no combate, no está `fuera` y, si lo persiguen, han pasado ya `PRESENCIA.topeAplazamientoMs` (5 min) desde que lo pidió. Quien huye desconectándose no gana nada. `salirDelMundo` y `volverAlMundo` conservan sus reglas (Presencia_Y_Convocatoria §7): en plaza o campamento no se mueve nada; en la columna sale con sus escuadras y su carro; en un ejército se separa con lo suyo y el mando pasa al más antiguo conectado. Las caravanas adjuntas sin titular vuelven solas a su origen.

## 8. El generador aleatorio

- **Una sola secuencia de la partida**, consumida en el orden de la cola (que es total y determinista). Cualquier cambio de orden cambia la partida a partir de ahí, así que el orden de la cola es parte de las reglas.
- Consumen el generador: el **combate resuelto con números** (varianza de ±15 %), el **nivel y el punto** de un campamento de bandidos nuevo, y los **sorteos de aparición** de eventos (los Aedas no consumen azar: Tecnologia_Aedas §3). El crecimiento de población no. Lo que construye el mundo con una semilla propia (campamentos de mercenarios, alijos, el artefacto de mundo) **no** toca la secuencia de la partida.
- Lo que vuelve de un worker (una ruta, un cálculo de combate) entra como un evento más y **se escribe en el registro**, para que una reproducción no dependa de qué hilo terminó antes.
- El crecimiento de población **no** consume el generador (§10, decisión 1).

## 9. Parámetros

El reloj de mundo ya no tiene tick. `SIMULACION.duracionTickMs` (60 000) queda solo como equivalencia de las cifras heredadas (1 minuto) mientras las constantes de juego viven en `constants.ts`; en el archivo común de datos (Reconstrucción §3) se expresan en instantes y en unidades por minuto.

<!-- parametros:SIMULACION -->
```json
{
  "epocaInicial": "2026-01-01T00:00:00.000Z",
  "duracionTickMs": 60000
}
```
<!-- /parametros -->

<!-- parametros:PRESENCIA -->
```json
{
  "retardoDesconexionMs": 150000,
  "topeAplazamientoMs": 300000
}
```
<!-- /parametros -->

Cifras de **infraestructura** (en `constantsInfra.ts`; no alteran el resultado de una simulación). `ticksMaximosPorPasada` pertenece al reloj por tick y desaparece; `limiteIdempotenciaPorCuenta` (claves que el servidor recuerda **de cada cuenta**: las últimas 64; en el TypeScript, `limiteIdempotencia`, 500 por partida; decisión del autor, 2026-10-24, para que un reintento tras reconectar no se aplique dos veces por muchos jugadores que haya; va en la foto, `BAN24`) y `eventosMaximosEnMemoria` (recorte del registro en memoria) se conservan.

<!-- parametros:INFRA -->
```json
{
  "ticksMaximosPorPasada": 5,
  "limiteIdempotenciaPorCuenta": 64,
  "eventosMaximosEnMemoria": 5000
}
```
<!-- /parametros -->

## 10. Decisiones (2026-10-13)

1. **La población es siempre entera** (corregido el 2026-10-22; antes era continua, con decimales leídos con suelo y, para los requisitos de nivel, redondeados). Cada clase guarda habitantes enteros y un resto oculto en milésimas; el crecimiento de un minuto y las muertes por hambre se suman al resto, y entra o muere una persona entera cuando el resto llega a 1. Se elimina la tirada del redondeo estocástico: el crecimiento deja de consumir el generador aleatorio.
2. **Tras una caída del servidor, el mundo no se recupera.** El reloj de mundo **se congela mientras el servidor está apagado** (regla del autor del 2026-09-05): al arrancar, el instante de mundo es el último guardado, no se procesan eventos que "vencieron" durante la caída y los plazos conservan el tiempo que les quedaba. (Sustituye a la lectura de Arquitectura/13 §6; ver su corrección.)
3. **Las órdenes a mitad de camino parten de la posición real en el instante de la orden**, no de la del inicio de un minuto.

## 11. Invariantes

| Invariante | Cómo se vigila |
|---|---|
| Misma semilla y mismas órdenes en los mismos instantes ⇒ mismo estado y mismos eventos | prueba de reproducción sobre el registro |
| Un reloj parado no procesa eventos; el modo acelerado y el normal dan el mismo mundo | prueba con ambos modos |
| Almacén ∈ [0, capacidad], población ≥ 0, medidor ∈ [0, 100], `nivel` monótono, reputación ∈ [−100, 100] | prueba de invariantes sobre una simulación larga |
| Una orden rechazada o sin cambios no sube la versión ni mueve el generador ni los ids | prueba de órdenes |
| Una columna o caravana en una batalla activa no cambia mientras dura | prueba de batallas |
| Quien combate no sale del mundo hasta que acaba la batalla | prueba de presencia |
| Los eventos de una entidad con generación vieja nunca se aplican | prueba de invalidación |

## 12. Divergencias

Ninguna abierta.
