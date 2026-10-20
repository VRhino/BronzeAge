# El reloj de mundo y los eventos

Cómo avanza el mundo en la reconstrucción: sin tick. Este documento **sustituye** a `Tick.md` (el «cómo funcionaba» del servidor TypeScript, con su cadena de 24 pasos por minuto, vive en `typescript-final:Docs/Mechanics/Tick.md`). Fija el reloj, la cola de eventos, cómo cada paso de la antigua cadena pasa a ser un **ritmo**, un **evento programado** o un **muestreo**, qué orden se conserva cuando varios eventos caen en el mismo instante y cómo se usa el generador aleatorio. Diseño de fondo: [`Arquitectura/13_Mundo_Reactivo_Rust.md`](../Arquitectura/13_Mundo_Reactivo_Rust.md).

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
> | `BandidosDeLaPlaza` | 7 | la plaza (su generación: la ruina lo descarta) | al fundarla y al vencer los 60 min de reaparición del campamento destruido | pone el campamento de la plaza en el bosque libre más cercano (una tirada de nivel, y ninguna si no hay bosque) |
> | `BandidosDelAnillo` | 7 | `entidad_anillo(campamento)` | al nacer la partida (una por campamento de mercenarios), a `reaparicionMinutos` tras crear uno o si no hubo sitio, y al destruirse uno del anillo | crea un campamento en el anillo si faltan según los residentes sin plaza; si no falta ninguno no se vuelve a programar |
> | acoso | 7 | la caravana (o la columna con caravanas adjuntas) o el campamento: el mismo `Carga::Encuentro` de la 4C, con prioridad 7 | el primer instante en que el plan de la caravana queda a ≤ `radioAtaqueCaravana` (7) de un campamento | una tirada contra la defensa; ganar la hace desaparecer |
> | `FinProteccion` / `FinOcupacion` | 2 | `entidad_conquista(plaza)` (una conquista nueva descarta los de la anterior) | `protegida_hasta` (+1 día) y `ocupada_hasta` (+90 min) | limpian el campo de la plaza y lo cuentan; la ocupación devuelve su recaudación, su crecimiento y su mantenimiento |
> | `ReponeGuarnicion` | 3 | `entidad_guarnicion(pueblo)` | 6 h tras el primer combate que dejó incompleta la guarnición de serie | la deja de nuevo completa, con su experiencia |
>
> Ninguno es recurrente por entidad ociosa: cada uno se programa cuando hay algo que esperar y se detiene solo (con los anillos y las plazas satisfechos la cola no guarda ninguna cita de combate). Mientras una plaza está ocupada, su paso de minuto pasa al `Exterior` los factores de la ocupación (`felicidad_extra` = `factorCrecimiento`, `recaudacion_extra` = `factorRecaudacion`) y la economía congela su mantenimiento (`ocupado`). Los plazos de la conquista y el de la guarnición comparten instante con el paso de minuto de la plaza (prioridad 2) y el consumo (3): dentro del mismo instante y prioridad manda la secuencia de la cola, no una regla (fila de paridad). El orden del generador aleatorio es ya parte de las reglas: la aparición de un campamento gasta una tirada de nivel (más dos por intento de punto en un anillo), un combate las suyas, y lo que no tira no gasta.

## 1. Tiempo

- **Reloj de mundo** = reloj de pared × factor (×1 en producción; ×N para pruebas y para el batch). No hay tick y no existe la unidad «minuto de mundo» como paso: sigue habiendo plazos declarados en minutos, horas y días, y se miden en instantes.
- **Instante** = milisegundos de mundo desde la época (`2026-01-01T00:00:00.000Z`), en entero de 64 bits. Todos los `*En`, `*Hasta` y `expiraEn` son instantes. «Vence» significa `instante ≥ vencimiento`.
- El reloj de pared **no entra en el estado**: dos partidas con la misma semilla y las mismas órdenes en los mismos instantes de mundo dan el mismo mundo.
- **Modo acelerado**: el batch procesa la cola sin esperar al reloj de pared; el factor es solo una forma de la misma regla.

## 2. La cola de eventos

- Cada evento lleva `(instante, secuencia, entidad, generación, tipo)`. Se aplican por `(instante, secuencia)`; con igual entrada, igual mundo.
- **Invalidación perezosa**: cuando una entidad cambia de plan o de ritmo sube su generación y los eventos viejos se descartan al salir de la cola.
- **Desempate del mismo instante**: la antigua cadena fijaba un orden de pasos dentro del minuto y varias reglas dependen de él. Se conserva como **prioridad de tipo** para eventos con el mismo instante, de menor a mayor número:

| Prioridad | Qué | Por qué en ese orden (regla que se conserva) |
|---|---|---|
| 1 | Cierre de batallas vencidas y salidas del mundo | una batalla que vence no bloquea en ese mismo instante |
| 2 | Fin de obras, de ascensos y de políticas | la obra usa las políticas antes de que expiren |
| 3 | Consumo de población y de tropa | la población come **antes** que las tropas |
| 4 | Crecimiento de población y recaudación de oro | la recaudación usa la población ya crecida |
| 5 | Mantenimiento y ruinas | una ruina deja de existir para el comercio, los bandidos y los ejércitos en ese instante |
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
| Obras, recetas, producción, mejoras | **Ritmo** neto por recurso; las obras y mejoras son eventos con su instante de fin | almacén lleno (cambia el ritmo), recurso a 0, obra terminada, mejora terminada |
| Auto-construcción (decidir qué se encola) | **Se reevalúa por dependencia**: cuando cambia lo que mira (almacén, cola, población, nivel, políticas), no cada minuto | — |
| Políticas expiradas | **Evento** a su caducidad | política caduca |
| Obra de ascenso | **Evento** a su fin | ascenso completado |
| Nutrición y consumo de comida | **Ritmo** de consumo; la nutrición cambia por umbrales de reserva | reserva a 0, nutrición cruza umbral |
| Mantenimiento de tropas (ración de la guarnición) | **Ritmo** de consumo; moral y deserción por umbrales | despensa a 0, deserción |
| Crecimiento de población | **Ritmo** logístico contra el cupo (la población se guarda continua) | población cruza un cupo o un umbral de nivel |
| Recaudación de oro | **Ritmo** (impuesto × población por clase) | almacén de oro lleno |
| Mantenimiento (costes, medidor, bajar nivel, ruina) | **Ritmo** de coste y de medidor | medidor cruza un umbral, baja el nivel, ruina |
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
| Vasallos liberados de un señor desarmado; tributos | Se reevalúan cuando cambia el estado del señor; el tributo mantiene su periodicidad como evento recurrente |
| Experiencia de Facción, reputación | Se aplican **al ocurrir** el hecho que las produce; el decaimiento de la reputación es un ritmo |
| Reposición de mercados de mercenarios | **Evento** agendado (ya era una cita) |
| Títulos | **Muestreo** o reevaluación cuando cambia lo que mira (Facciones, plazas, relaciones) |
| Tecnología, logros, Aedas, residentes | Los contadores suben al ocurrir el hecho; las condiciones de adopción se evalúan al subir un contador; los Aedas itinerantes, eventos programados |
| Caravanas de fundación (vuelve sola, se desarma, caduca) | **Planes** y eventos de caducidad |
| Memoria de la niebla y exploración | **Muestreo** (§4) |

## 4. Lo informativo se muestrea

Qué ve cada Facción, la niebla explorada, la memoria de plazas y la exploración personal se recalculan en paralelo cada **250 ms** (valor inicial) sobre la rejilla espacio-temporal, por clave de visión. No hace falta el instante exacto en que algo entra en la vista. Lo que sí tiene efecto de juego (una columna entra en la zona de otra, alcanza una puerta, intercepta una caravana) es un evento exacto.

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
- Consumen el generador: el **combate resuelto con números** (varianza de ±15 %), el **nivel y el punto** de un campamento de bandidos nuevo, y los **sorteos de aparición** (Aedas, eventos). El crecimiento de población no. Lo que construye el mundo con una semilla propia (campamentos de mercenarios, alijos, el artefacto de mundo) **no** toca la secuencia de la partida.
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

Cifras de **infraestructura** (en `constantsInfra.ts`; no alteran el resultado de una simulación). `ticksMaximosPorPasada` pertenece al reloj por tick y desaparece; `limiteIdempotencia` (claves que recuerda el servidor por partida) y `eventosMaximosEnMemoria` (recorte del registro en memoria) se conservan.

<!-- parametros:INFRA -->
```json
{
  "ticksMaximosPorPasada": 5,
  "limiteIdempotencia": 500,
  "eventosMaximosEnMemoria": 5000
}
```
<!-- /parametros -->

## 10. Decisiones (2026-10-13)

1. **La población es continua.** Se guarda como número continuo (valor en t₀ y ritmo logístico) y se **lee en entero (suelo)**, salvo los **requisitos de nivel de asentamiento**, que se leen **redondeados al entero más cercano** (decidido el 2026-10-18: con el suelo una clase que se acerca al cupo sin tocarlo, como 399,997 de 400, no los cumpliría nunca). Se elimina la tirada del redondeo estocástico: el crecimiento deja de consumir el generador aleatorio.
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
