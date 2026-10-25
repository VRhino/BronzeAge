# Movimiento de ejércitos y columnas

## 1. Alcance y fuentes

Cubre cómo se **mueve, come, se junta y se separa** una columna en el mapa: salir, marchar, perseguir, repostar, formar ejércitos, enganchar caravanas y entrar en plazas y campamentos. El **combate** que ocurre cuando dos columnas, una caravana, un campamento o una plaza se encuentran está en [`Combate.md`](Combate.md); aquí solo se dice dónde empieza.

- **Reescrito el 2026-10-13** para el mundo reactivo (sin tick; [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md)) y la calibración de distancias. El «cómo funcionaba» del servidor TypeScript (por tick, con `regresando` y `origenAsentamientoId`) está en `typescript-final:Docs/Mechanics/Movimiento_Ejercitos.md`.
- **Canon:** Doc 5.11 (Liderazgo), 5.12 (movimiento y encuentros), 5.13 (suministro), 5.14 (ejércitos), 5.15.2b (tropa del héroe y tropa prestada), 1.10 (el jugador en el mundo).
- **Fuera de alcance aquí:** el grafo de navegación y la red de caminos (`Comercio_Caravanas.md` §7), la niebla ([`Niebla_De_Guerra.md`](Niebla_De_Guerra.md)) y el generador aleatorio.

> **Unidades.** El mapa mide **2.667 × 2.000** unidades. Las velocidades son «unidades de mapa por minuto» **antes** del coste del terreno; el reloj es continuo. Un ejército **no tiene destino fijo ni vuelve solo a ningún sitio**: sale quieto a la puerta y su Líder lo dirige con sus clics (Doc 5.12).

> **Implementado en Rust (2026-10-19, fase 4A-1: reglas puras).** Crate `crates/militar`, sin posición ni plan (son del núcleo, fase 4B-2): `racion` (§6: la ración de una columna y de una guarnición, los víveres como **ritmo** con el evento «víveres a 0», `llenar` y `pasarAViveres`; la moral y la deserción están en `escuadra`), `columna` (§2, §4.1, §5, §10 y parte de §11: la composición y el mando de una columna; salir, `unirseEnCampo`, `unirseAEjercito`, peticiones de 10 s, `separarseDelEjercito`, `desgajar`, `cederLiderazgo`, la sucesión por antigüedad, formar un ejército en campo, `desarmar`; la velocidad como mínimo y el alcance de vista) y `persecucion` (§7: las condiciones y el alcance). Cada operación que cambia la gente, el carro o el estado deja los víveres con la capacidad (suma de sus héroes) y el consumo bien anotados. **Decisiones del autor del 2026-10-19 aplicadas en este documento:** persecución solo con la presa a la vista y se cancela si sale de ella (sin `umbralRecalculo`, §7); una caravana solo se suelta dentro de una plaza (§9); los bloques son los valores vigentes (§14). **Fase 4B-1 (2026-10-20):** el hambre de la **guarnición** (§6.3, último párrafo) ya está en el núcleo, minuto a minuto dentro del paso de la plaza (véase [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md)); `pasarAViveres` (`PasarAViveres`, 60) y repostar gratis en un campamento ya llenan los víveres de la columna del héroe. Lo que quedaba (repostar, las caravanas adjuntas, la disolución sin nadie y entrar en un lugar) llegó en la 4B-2 (nota siguiente); siguen sin estar la derrota de una columna con caravanas (4D) e inspeccionar (§12, 4C).

> **Implementado en Rust (2026-10-20, fase 4B-2: las columnas completas).** `crates/nucleo/src/nucleo/{columnas,salidas,uniones,convocatorias}.rs` y `codec_columna.rs`; formato de foto **`BAN10`** (`BAN9` se rechaza). `Ejercito` es `{ id, columna: militar::Columna, posicion, plan, velocidad_fija, caravana_fundacion, caravanas_adjuntas, version }`: lo que compone y manda la columna (tipo, Líder, participantes, política, peticiones, formación, escuadras, carro y víveres) es de `militar::Columna`; el núcleo pone la posición, el plan y los plazos.
>
> - **Nada derivado se guarda.** El consumo (§6.3) se calcula de la composición cada vez que cambia (`Nucleo::consumo_de_columna` = `racion_columna(soldados, héroes, estado)`: acampada, marchando o apartada por una batalla) y la velocidad es el mínimo de la composición (`velocidad_de_ejercito`, §4.1; `velocidad_fija` solo la usa `CrearEjercito`, la administración). La capacidad del carro es `participantes × capacidadCarroPorJugador + Σ caravanas adjuntas` (`capacidad_de_carga`). Cualquier cambio de gente, de escuadras (reclutar en la columna, reponer el préstamo, desertar) o de estado re-ancla los víveres al ritmo nuevo (`recomponer_columna`) y reescala la marcha si cambió la velocidad.
> - **Hambre sin eventos por columna ociosa (§6.3, §13).** Con ración no hay evento. Cuando los víveres cruzan 0 salta el único `Umbral(0)` de la columna, que ancla la moral de cada escuadra (`Escuadra::con_racion`, idempotente) y programa **un** evento `Desercion` por columna con hambre (prioridad 3) en la próxima fase de deserción de sus escuadras: la de cada escuadra es el instante en que su moral llega a 0 y luego cada minuto (`desercion_a_partir_de`). Al saltar, deserta `ceil(5 %)` de cada escuadra cuya fase cae ahora (`tropas.desercion`), cambia el consumo y se reprograma. Si vuelven los víveres (`pasarAViveres`, repostar, unirse), la generación sube y el evento muere; una columna sin soldados o con la moral subiendo no tiene evento. **Decisión:** no hay evento recurrente por columna quieta y con ración (la moral solo sube y se lee al mirar).
> - **Salir (§3) es siempre la misma operación en dos tiempos:** `validar_salida` comprueba todo sin tocar nada (presencia, escuadras —existe, es suya, en el campamento, con soldados, fuera de la guarnición, sin candado—, Liderazgo, carga no negativa, que quepa en el carro, existencias con el trigo por encima de la reserva, ración gratis del campamento) y `aplicar_salida` la ejecuta. `Reservas` acumula lo que gastan varias salidas simuladas en el mismo lugar (las convocatorias). Quien sale con otra política que `rechazar` sale como ejército.
> - **Órdenes nuevas (69-90).** `SalirAlMundo`, `MovilizarEjercito`, `SalirDelCampamento`, `EntrarEnAsentamiento`, `SalirDeAsentamiento`, `Guarnecer`, `EntrarEnCampamento` (§3, §11), `UnirseAEjercito`, `UnirseEnCampo`, `ResponderPeticionDeUnion`, `SepararseDelEjercito`, `CederLiderazgo`, `OrganizarEjercito`, `CancelarFormacion` (§10), `Repostar` (§8) y las seis de convocatoria (`Presencia_Y_Convocatoria.md` §9). Todas se validan sobre el estado antes de tocarlo: **un rechazo no deja rastro** (ni versión, ni ids). Las dirige quien manda en el héroe (su cuenta o la administración); `MarcharA` y `Estacionar` las da el **Líder** de la columna (o la administración).
> - **Entrar (§11).** `entrarEnAsentamiento`: el residente se deshace de su columna (la tropa vuelve a su campamento y los víveres al héroe; el carro se queda con él, desde el 2026-10-22); el de visita entra con su columna, dentro, con lo suyo (sigue comiendo a ritmo de acampada) y sale con `salirDeAsentamiento`. `guarnecer` desarma el ejército del Líder en la puerta de una plaza que le abre su puerta y cada héroe entra como lo haría solo; las caravanas adjuntas entran con él, sean de donde sean (desde el 2026-10-22; antes quedaban aparcadas en la plaza y, si alguna era de otra, no entraba: `caravana.ajena_al_lugar`). `entrarEnCampamento` hace lo mismo en un campamento.
> - **Una columna sin nadie se disuelve (§11).** Sus caravanas adjuntas vuelven solas a su origen por tierra o se pierden si no hay camino (`ejercito.disuelto`). Un héroe que se desconecta en campo (`Presencia_Y_Convocatoria.md` §7) se lleva sus escuadras, su parte del carro y de los víveres y el mando pasa al más antiguo conectado.
> - **Caravanas adjuntas (§9).** `adjuntarCaravana` exige un **ejército** (una columna personal no las lleva: `columna.personal_sin_caravanas`). Se sueltan (`aparcarCaravana`) solo dentro de una plaza (Q3). Siguen en `Comercio_Caravanas.md` §8 y en la fila de paridad de `Docs/Mecanicas a desarrollar.md` la Caravana de Fundación como caravana de verdad (adjuntar/soltar en campamento, `reclamarCaravana`).
> - **Repostar (§8)** es `Repostar { heroe, asentamiento }` (83): el Líder, con la columna a `radioPuerta` de una plaza de su Facción, rellena los víveres sin bajar de la reserva de trigo (`columna.viveres_llenos`, `repostar.sin_existencias`, `repostar.plaza_cerrada`). **Mientras no exista el polígono de la zona** como entrada/salida de columnas, la misma orden vale para una columna personal (que en el TypeScript reposta sola dentro de la zona); `puedeRepostarEn` solo conoce la plaza propia (las aliadas, `permiteReabastecerAliados`, son del sistema político). Ambos recortes están como filas de paridad.
> - **Fuera de esta fase:** la derrota de la columna con sus caravanas y el botín (4D), `cargarCaravana`, `alternarReabastecerAliados`, el evento `jugador.marcha_fijada` (lo emite desde 4H T5) y la puerta/vetos de las plazas (sistema 5: mientras, entra quien reside o es de la Facción, y los demás según el acceso por relación que fija el dueño, 2026-10-22).

> **Implementado en Rust (2026-10-20, fase 4C: encuentros, persecución e inspección).** `crates/nucleo/src/nucleo/{espacio,persecuciones,inspecciones}.rs`, `crates/militar/src/inspeccion.rs` y `crates/encuentros`; formato de foto **`BAN11`** (`BAN10` se rechaza). Órdenes **91-94**: `Perseguir`, `DejarDePerseguir`, `Inspeccionar` y el andamiaje de administración `FijarRelacion` (alianza, vasallaje, guerra por pareja de Facciones, hasta el sistema 5; solo la alianza cuenta en la 4C).
>
> - **El espacio (Combate §2).** El núcleo guarda en una rejilla espacio-temporal todo lo que cuenta en el mapa: las **columnas en el mapa** (en marcha con su plan, o acampadas, quietas hasta el horizonte), las **caravanas** en tránsito y las **plazas** (fijas). No es estado: se reconstruye al abrir el núcleo y los encuentros ya programados viajan en la cola (`Carga::Encuentro`, prioridad 8). Una columna está en el mapa si algún héroe va a bordo y no la aparta una batalla: la que ha entrado en un lugar sale, y con ella sus encuentros y sus perseguidores (`columna.presa_a_cubierto`). Cada vez que una cosa cambia de plan, de generación o entra en el mapa se recalculan **solo sus pares**; lo que ya estaba junto cuando cambia de plan no se «encuentra» otra vez, y lo que entra en el mapa junto a otra cosa, sí. El generador externo `GeneradorEncuentros` se retira: los encuentros ya no dependen de que alguien lo registre. Un cruce se **filtra al vencer**, con el estado de ese instante: ambas cosas en el mapa y sin batalla, la columna con algún héroe sano, de Facciones distintas y no aliadas, y de la misma clase (personal con personal, ejército con ejército). Deja solo un hecho: `ejercito.encuentro` (dos columnas), `caravana.encuentro` (una caravana con una columna o con otra caravana) y `asentamiento.encuentro` (una columna junto a una plaza ajena). No hay combate ni estado «a la espera»: atacar es una orden aparte (fase 4D).
> - **Perseguir (§7).** `Perseguir { heroe, objetivo }` la da el Líder a bordo, no en formación ni en batalla; valida con `militar::persecucion` (`persecucion.*`) y, si hay camino por tierra, deja a la columna en marcha por el A\* hasta el **punto de intercepción** (`encuentros::interceptar`: el primer sitio donde, a su velocidad, queda a ≤ `radioEncuentro` según el plan de la presa). Ese plan se rehace **cada vez que la presa cambia el suyo** o cuando el perseguidor llega al punto sin alcanzarla (la intercepción supone la velocidad nominal). El **alcance** es el encuentro exacto del par plan contra plan (`columna.presa_alcanzada`); la **presa fuera de vista** es el primer instante, plan contra plan, en que pasa de `alcanceDeVista` (`SucesoMilitar::PresaFueraDeVista`, `columna.presa_perdida`) y se reprograma con cada plan nuevo de cualquiera de las dos y con cada cambio de la tropa en pie. Una presa que sale del mapa (entra en un lugar, se disuelve, va adjunta, batalla) da `columna.presa_a_cubierto`; sin camino por tierra, `columna.presa_perdida`. `marcharA`, parar la columna o `dejarDePerseguir` borran la persecución; solo el último avisa (`columna.persecucion_abandonada`) y sin presa es «sin cambios». Con la presa ya a ≤ 3 al ordenar, `persecucion_iniciada` y `presa_alcanzada` salen en el mismo instante. Quien huye **desconectándose** de una columna perseguida no sale hasta `pedido + topeAplazamiento` (Reloj_Y_Eventos §7).
> - **Inspeccionar (§12).** `Inspeccionar { heroe, objetivo }` (columna, caravana o plaza ajena) a ≤ `radioInspeccion` (8): guarda un **informe** en el héroe (`Nucleo::informes_de_inspeccion`, uno por objetivo, `INSPECCION.informeMinutos` = 10 minutos, renovable) y avisa al observado (`columna.observada`, `caravana.observada` —a la plaza de origen— y `asentamiento.observado`). Rechazos: `inspeccion.la_tuya`, `inspeccion.plaza_propia`, `inspeccion.lejos`, `inspeccion.a_cubierto`.
> - **Batalla (4E).** `Nucleo::columna_entra_en_batalla` / `columna_sale_de_batalla` (y, dentro de una transición, `apartar_columna` / `reponer_columna`) apartan y devuelven una columna, y los llama el ciclo de la batalla de Unity ([`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md)): sale del espacio, **no come** (`Estado::Apartada`, consumo 0), no admite órdenes de movimiento, persecución ni inspección (`columna.en_batalla`) y su plan queda en `Ejercito::plan_en_pausa`. Al volver **retoma ese plan** desde donde se quedó, desplazado lo que duró la batalla y reescalado si las bajas le cambian la velocidad (si no marchaba, vuelve quieta y comiendo a ritmo de acampada); la persecución que llevaba no se retoma. Quien pidió salir del mundo mientras combatía lo reintenta ya.
> - **Visión.** `GeneradorVision` ya distingue al observador con soldados en pie (30) del que no (15).
> - **Zonas.** El repostar automático de una columna personal dentro de una zona sigue sin hacer; el análisis y lo que falta están en la fila de paridad de `Docs/Mecanicas a desarrollar.md`. Para el tipo A/B de una caravana basta `Nucleo::en_zona` en el instante del ataque.

> **Implementado en Rust (2026-10-24, fase 4G): el batch militar** (`batch militar`, ver [`Combate.md`](Combate.md), nota de la 4G). Sus bots salen (`MovilizarEjercito`, `SalirAlMundo`, convocatorias que parten), se unen en la puerta (`UnirseAEjercito`) y en campo (`UnirseEnCampo`), marchan, persiguen (atacan en el instante en que la presa queda a tiro según los dos planes), comen y reponen (`PasarAViveres`, `Repostar`), llevan caravanas adjuntas y vuelven a casa (`Guarnecer`, `EntrarEnAsentamiento`). Mide salidas, convocatorias y uniones, persecuciones iniciadas, cumplidas y canceladas, columnas sin víveres y deserciones, y comprueba en cada observación que toda columna sin víveres y con soldados tiene su cita de deserción, que ninguna con víveres la tiene y que nadie deserta con víveres. Con 60 plazas y 30 días (semilla 42): 53.931 salidas, 11.935 convocatorias (2.576 héroes unidos), 4.555 uniones, 4.245 persecuciones (3.211 alcanzadas), 60 columnas sin víveres y 11 desertores. Halló tres fallos, arreglados con su prueba: (1) **el reparto del carro al desarmar** dejaba al Líder por encima de un carro por el redondeo (§10.3, `militar::columna::desgajar`); (2) **`unirseAEjercito` con la columna que se conserva para las caravanas** tumbaba el núcleo al fundirla (§10.1, `Nucleo::comprobar_fusion`, compartida con la convocatoria); (3) **aparcar una adjunta con el carro lleno** dejaba la columna por encima de su capacidad (§9; desde el 2026-10-24, M10, lo que no cabe se queda en la caravana aparcada).

`Ejercito` es la columna, sea personal o ejército. Campos que importan:

| Campo | Significado |
|---|---|
| `tipo` | `'personal'` o `'ejercito'`. **Se fija al nacer**: un ejército al que se separan miembros hasta quedar uno sigue siendo ejército (y solo su Líder lo dirige). |
| `participantes[]` | `{ heroeId, unidoEn }`. `unidoEn` decide la sucesión del mando. **Un ejército es un conjunto de héroes con sus columnas**: la tropa, el carro y los víveres de cada uno van con él. |
| `liderId` | Quien manda. |
| `politicaDeUnion` | `rechazar` / `aceptar` / `preguntar`; la fija el Líder al salir y no cambia. |
| `peticionesDeUnion[]` | Solo con `preguntar`: `{ heroeId, pedidoEn, expiraEn }`. Caducan a `MOVIMIENTO.vidaPeticionUnionSegundos` (10 s) como **evento**. |
| `escuadronIds[]` | Las escuadras viven en su héroe; la columna guarda ids. |
| `suministro` | El **carro** (carga: botín, materiales). No es lo que se come. |
| `viveres` | Los víveres del ejército (un montón propio; en una columna personal son los del héroe). |
| `caravanasAdjuntasIds[]` | Caravanas enganchadas (§9). |
| `plan` | La ruta como lista de vértices con el **instante de paso por cada uno** (§4). Sin plan, la columna está quieta en `posicion`. |
| `generacion` | Sube con cada plan nuevo; invalida los eventos programados con el anterior. |
| `estado` | `marchando` · `estacionado`. (No existe `regresando`.) |
| `persiguiendo` | `{tipo: 'ejercito'|'caravana', id}` (§7). |
| `formacion` | `{expiraEn}` si es una formación en espera de ser tres (§10.4). |

No hay `origenAsentamientoId` ni `origenCampamentoId`: nadie vuelve «a casa» solo. `posicion(t)` se calcula del plan.

## 3. Salir

Funciones puras que producen la columna nueva y la plaza con el carro cargado.

### 3.1 Salir como ejército (`movilizarEjercito`)
1. Solo un **residente** de la plaza sale desde ella (ahí están su campamento y su tropa).
2. Escuadras: deben existir en el campamento, ser del héroe, tener `cantidad > 0` y **no estar en la guarnición**. Al menos una; se rechaza en vez de filtrar en silencio.
3. **Liderazgo** (`exigirLiderazgo`): `puedeLlevar(héroe, [lo que ya tiene fuera] + [las nuevas])`, con `liderazgoDe(héroe) = héroe.liderazgoBase ?? LIDERAZGO.base` y el coste de la escuadra = `LIDERAZGO.costePorEscalon[escalón de su tropa]`, **sin mirar cuántos soldados quedan** (una escuadra mermada paga el precio completo, Doc 5.11.1). El tope es **por héroe**.
4. Carro: `cargarCarroElegido` (§6.2), con capacidad de un carro (`capacidadCarroPorJugador`).
5. Nace `tipo: 'ejercito'`, **`estacionado` en la puerta**, un participante, sin plan: **no se le da destino al salir** (decisión del autor, 2026-10-08). Su Líder lo dirige con `marcharA`. Los víveres los llena quien llama (`llenarViveres`, §6.1).

### 3.2 `salirAlMundo` (salir a lo tuyo)
Igual, con estas diferencias: se puede salir **sin tropas**; no se puede salir si ya vas en una columna; nace `tipo: 'personal'`, `politicaDeUnion: 'rechazar'`, `estacionado` en la posición de la plaza.

### 3.3 `salirDelCampamento` (de un campamento de mercenarios)
- El **no residente** sale con todo lo que entró, su columna (la selección se ignora). Sin ella sale solo, con sus víveres; si intenta sacar tropa o carga que no tiene aquí, `salida.solo_de_tu_residencia` (en Rust; el TypeScript daba error por no tener columna).
- El **residente** elige tropa de su campamento y carga del **almacén personal**: cada recurso pedido debe estar y caber en lo que queda del carro, que ya lleva el que trajo a su residencia (Entrar con todo).
- **El almacén personal vive donde se reside**: `guardarEnAlmacenPersonal` / `sacarDelAlmacenPersonal` exigen ser el Líder de la columna **y** tenerla a la puerta de la propia residencia (a ≤ `MOVIMIENTO.radioPuerta`).
- **Ración gratis** (`MERCENARIOS.racion.cadaMinutos`): si pasó ese plazo desde `racionEn`, sale con los víveres al tope y `racionEn = ahora`. No se acumula.
- Con `politicaDeUnion ≠ 'rechazar'` sale como ejército; si no, columna personal quieta.

## 4. Velocidad y plan

### 4.1 Velocidad de la columna
Es el **mínimo** de: la `velocidad` de cada escuadra con `cantidad > 0` (una aniquilada no frena a nadie) y la `velocidadCaravana` de cada caravana adjunta. Sin ninguna de las dos: `MOVIMIENTO.velocidadJugador` si va alguien. Una escuadra pesada (18) frena a una fuerza ligera (30) y le quita la capacidad de cazar caravanas comerciales (24); engancharse una caravana baja la velocidad al mínimo de las dos, así que **no se puede escoltar y depredar a la vez**.

**Velocidades calibradas (2026-10-12, ×1,5):** montada 42, ligera 30, media 24, pesada 18; héroe solo 33; caravana con caballo 36, con camello 28, con buey (comercial base y acarreo) 24; Caravana de Fundación que vuelve sola 15. Las reglas de Doc 5.12.5 se mantienen: la ligera (30) alcanza al buey (24); solo la montada (42) alcanza a la de caballo (36).

### 4.2 Construir el plan
Al dar la orden (`marcharA`, perseguir, o cualquier cambio que altere la velocidad), el servidor calcula **de una vez** el plan completo:

```
ruta        = calcularRuta(...)                          (A* sobre el grafo, §4.3)
partir la ruta donde cambia el coste (terreno, bosque, entrar o salir de un camino)
para cada tramo:   enCamino = el tramo es una arista de la red de caminos
                   avance   = velocidad / ( coste del tramo × (enCamino ? COSTE_MOVIMIENTO.factorCamino : 1) )
                   instante de paso por el vértice siguiente = instante del anterior + longitud / avance
```
- El coste se evalúa **a lo largo del tramo**, no en un punto al inicio de un paso: el plan no tiene la aproximación del tick.
- Llano = 1, y sube con colinas, montañas y bosque (`1 + bosquePorDensidad × densidad`) (`COSTE_MOVIMIENTO`, `worldgen/config.ts`). Sobre la red de caminos el coste se multiplica por `factorCamino`.
- `posicion(t)` interpola dentro del tramo. Un plan de un solo punto o vacío no se mueve.
- **El plan se rehace** por un hecho: la orden cambia, muere la tropa que fijaba la velocidad mínima, se engancha o suelta una caravana, se construye un camino en el trayecto, empieza o acaba una batalla. El plan nuevo **empieza donde está la columna en ese instante**.
- **Eventos que programa:** llegada, fin de víveres, deserción por moral (§6.3), encuentros (Combate §2).

### 4.3 La ruta (`calcularRuta`, A\* sobre el grafo de navegación)
- Origen y destino deben ser transitables (no agua) o no hay ruta; ningún llamador sigue sin ruta (una recta cruzaría el agua).
- Los **ríos** se cruzan por un vado o por una ciudad ribereña: `pasosRio` = posiciones de asentamientos, que cuentan como paso a menos de `NAVEGACION.radioPasoCiudadMin` (10).
- Si origen y destino están a menos de un espaciado del grafo, tramo directo si es válido; si no, se «engancha» a los nodos de alrededor y corre A\* con 8 vecinos y heurística admisible.
- El cálculo es caro y corre **fuera del núcleo** (en paralelo); su resultado entra como un evento más y se escribe en el registro (Reloj_Y_Eventos §8).
- **Grafo calibrado:** espaciado 8 y ~333 nodos por lado (mapa de 2.667), un vado cada 100 (antes 300), para distinguir plazas a 30.

## 5. Destino, rumbo y estado

- **`marcharA`** fija o rectifica el destino: solo columna `personal` **o el Líder** de un ejército; no en una formación; y el héroe debe estar en `columna`. Calcula el plan **desde la posición real en ese instante**, pone `estado: marchando` y **borra `persiguiendo`**.
- **`estacionarEjercito`** (cancelar la marcha): solo el Líder si lo pide un actor. **Detiene la columna donde está**: `estado: estacionado`, sin plan. Error si ya estaba quieta. **No hay vuelta a casa automática** (Doc 5.12.6).
- **Llegar** al final del plan no es atacar ni entrar: la columna **acampa** (`estacionado`) donde llega, con evento `ejercito.llega`. Quien persigue no acampa al llegar al último punto visto de su presa (§7).
- **Deshacer un ejército** es llevarlo a la puerta de una plaza o de un campamento y desarmarlo (§11), no vaciarlo en campo abierto.

## 6. Comer: víveres, carro y hambre

### 6.1 Los víveres
- Tope por héroe: `LOGISTICA.capacidadViveresPorHeroe` (350), derivado del radio operativo: **un jugador solo debe poder recorrer un cuarto del mapa ida y vuelta** (unas 1.333 unidades, 44 min a velocidad 30; la tropa prestada completa de un campamento más su héroe comen ~3,3 por minuto, 146 en ese trayecto: los 350 sobran, a recalibrar en batch). **En un ejército son unos víveres propios, cuya capacidad es la suma de sus héroes**; de ahí come todo el ejército.
- **Al unirse**, el héroe aporta sus víveres y su capacidad. **Al separarse o al desarmarse el ejército**, cada héroe se lleva una parte igual: víveres × su capacidad / la capacidad total.
- Se llenan **solo**: (a) al salir, de la plaza (`llenarViveres`); (b) al repostar (§8); (c) con la ración del campamento de mercenarios (§3.3) o con `repostarEnCampamento` (gratis, cualquier héroe en un campamento, hasta el tope; no gasta el stock del mercado y no pasa por el carro); (d) desde el carro con `pasarAViveres` (solo el Líder; `min(pedido, trigo del carro, hueco)`; nunca al revés).
- `sacarTrigoDe(plaza, consumoTropas, espacio)`: suelta `min(espacio, trigo − reservaDeTrigo)`; la reserva es lo que comen la población y la guarnición durante `RESERVA_CONSTRUCCION.horizonteMinutosComida`. Sacar una columna cuesta stock real pero no puede dejar la ciudad en hambruna.
- **Los víveres no son carga**: no se descargan, no se venden y no se roban.
- **En unidades enteras con resto oculto** (economía entera, EE-3, 2026-10-24; Rust). El ritmo de los víveres es el valor exacto y hace de resto oculto: se **ven** en unidades enteras hacia arriba (la unidad que se está comiendo sigue hasta acabarse, como en un almacén, Economia_Construccion §4) y se **acaban** cuando lo exacto llega a 0, así que el evento «víveres a 0», el hambre y la deserción (§6.3) caen en el mismo instante que antes. Lo que **entra** desde un almacén o un carro (salir con trigo, repostar en una plaza o una avanzada, pasar del carro) va en unidades enteras hacia abajo, y faltando menos de una unidad ya están llenos (`columna.viveres_llenos`); lo gratuito (la ración del campamento, `repostarEnCampamento`) llena hasta el tope, que es entero. **Al separarse o desarmarse**, la parte de cada héroe va en unidades enteras hacia abajo y el resto oculto se queda en la columna; al unirse, los víveres se suman tal cual.

### 6.2 Cargar el carro
Tres topes, y falla en vez de recortar: total ≤ capacidad; cada recurso ≤ disponible en el almacén (el trigo, menos la reserva); nada negativo. **El carro solo se llena parado en un lugar** (dentro de un asentamiento o campamento, o en su puerta para las acciones que se hacen desde ella).

### 6.3 La ración, como ritmo
```
factor  = factorConsumoEnMarcha × (estado == 'estacionado' ? factorConsumoEstacionado : 1)
ritmo de consumo (por minuto) = Σ(cantidad de soldados) × MILITAR.racionPorSoldadoPorMinuto × factor
                              + participantes × MOVIMIENTO.consumoPorParticipante × factor
```
- El consumo es un **ritmo** sobre los víveres de la columna: la cantidad en cada instante es `víveres₀ − ritmo × (t − t₀)`. El servidor **programa el evento «víveres a 0»** y lo reprograma cuando cambia el ritmo (cambia el estado, muere tropa, se une o separa alguien, se reposta).
- Mientras haya víveres, la moral de cada escuadra sube `regeneracionMoralPorMinuto` (tope 100). Con víveres a 0: pierde `degradacionMoralSinRacion` por minuto; con moral ≤ 0 y soldados, **deserta** `ceil(cantidad × desercionFraccionPorMinutoSinMoral)` en eventos de umbral (`tropas.desercion`, atribuido a la plaza de origen de la escuadra para no contar a todo el mundo que un rival pierde hombres).
- La guarnición usa **la misma regla de hambre** (ritmo contra la despensa del almacén, factor 1, 0 participantes).

## 7. Perseguir

- **`perseguir`** fija `persiguiendo = {tipo, id}` y `estado: marchando`. Condiciones: no a sí mismo; la columna propia con algún héroe sano; presa de la **misma clase** (`personal` con `personal`, `ejército` con `ejército`) y, si es una columna, con algún héroe sano. Antes: no a los tuyos ni a un aliado, no a una caravana adjunta, y **un ejército no persigue caravanas** (las persigue una columna personal). **Solo se inicia con la presa a la vista** (decidido el 2026-10-19): a ≤ `alcanceDeVista` del perseguidor (`VISION.ejercito` = 30 si lleva soldados, si no `VISION.jugadorSolo` = 15); si no, se rechaza.
- **`dejarDePerseguir { heroeId }`** suelta la presa; siempre anota el historial y emite `columna.persecucion_abandonada`. Cualquier `marcharA` también la suelta, sin evento.
- **El servidor conoce el plan de la presa** y calcula el **punto de intercepción**: el primer instante en que el perseguidor, a su velocidad, está a ≤ `LOGISTICA.radioEncuentro` (3) de la presa. Lo recalcula cuando la presa cambia de plan (**un evento, no un sondeo**).
  1. Presa fuera del mapa (entró en plaza o campamento, se disolvió, adjunta o en batalla) → se suelta, `columna.presa_a_cubierto`.
  2. **Presa fuera del alcance de vista** (a más de 30 si el perseguidor lleva soldados, si no de 15) → la persecución se **cancela** (`columna.presa_perdida`): ya no está a la vista, y si no la trayectoria del perseguidor delataría dónde está bajo la niebla. Es un **evento exacto**: se calcula, plan contra plan, el primer instante en que la presa sale del radio, y se reprograma cuando cualquiera de los dos cambia de plan (no es un muestreo, porque cambia el juego).
  3. Si no hay camino por tierra → se suelta, `presa_perdida`.
  (**No hay umbral de recálculo** (decidido el 2026-10-19): el plan del perseguidor se recalcula cada vez que la presa cambia el suyo, que es un evento.)
- **Alcance:** si la presa está a ≤ `radioEncuentro`, es enemiga (no misma Facción ni aliada), tiene héroe sano (columna) o no está en un refugio ni adjunta ni `disponible` (caravana) → `persiguiendo` se borra y se emite `columna.presa_alcanzada`. **No hay combate**: atacar es una orden aparte (Combate §2).

## 8. Repostar (Doc 5.13)

- **Columna personal: automático, dentro de la zona de influencia.** Mientras el héroe con su columna está dentro de la zona de una plaza donde puede repostar, sus víveres se rellenan **solos**, sin orden y sin límite de veces. La plaza es la dueña del polígono de zona donde está (las zonas no se solapan). Es un evento de entrada en la zona más un ritmo de rellenado mientras permanece.
- **Ejército: con la orden `repostar { heroeId }`, en la puerta.** El Líder, con el ejército a ≤ `radioPuerta` (2) de un asentamiento donde pueda repostar, rellena los **víveres del ejército** de su almacén. Un ejército no reposta solo.
- **En una avanzada con almacén** (fuerte o ciudadela, Doc 5.17): un ejército de la Facción reposta en su puerta con la misma orden. *Implementado en Rust (2026-10-22, fase 4F): `RepostarEnAvanzada { heroe, avanzada }` (114), del Líder con la columna a `radioPuerta` de una avanzada operativa de su Facción; toma `min(hueco, trigo del almacén)` sin reserva (la avanzada no tiene población que alimentar) y, como `repostar` en una plaza, vale también para una columna personal (decisión del autor, 2026-10-22). Rechazos: `avanzada.sin_almacen` (la atalaya), `avanzada.no_operativa`, `repostar.sin_existencias`, `columna.viveres_llenos`, `avanzada.faccion_ajena`, `avanzada.fuera_de_la_puerta`. El almacén se llena con `DepositarEnAvanzada` (115): descarga de la columna (y sus caravanas adjuntas) un recurso hasta donde quepa (2.000 por recurso en un fuerte, 10.000 en una ciudadela).*
- **Dónde se puede (`puedeRepostarEn`):** plaza **propia**, siempre; **aliada** —alianza o vasallaje, en cualquier dirección— solo si tiene `permiteReabastecerAliados`; neutral u hostil, nunca; **campamento de mercenarios**, cualquiera y gratis.
- Con las mismas reglas que llenarlos al salir: hasta donde quepa y sin bajar de la reserva de trigo de la plaza. Solo se **narra** (`ejercito.reabastecido`) si llevaba menos de `umbralNarrarReposte` (50 %).
- **`alternarReabastecerAliados { asentamientoId, permitido }`**: abre o cierra el almacén de la plaza a los ejércitos aliados (Gobernador o Tesorero). Sin cambio → `sinCambios`; con cambio → `logistica.reabastecer_aliados`. Cerrar no es retroactivo.

> **Implementado en Rust (2026-10-24, 5D):** `AlternarReabastecerAliados` (152; desde 5E, el Gobernador o el Tesorero, residente y dentro) guarda `Asentamiento::permite_reabastecer_aliados` (en la foto; la conquista lo cierra, como el acceso); `Nucleo::abre_su_almacen_a` es `puedeRepostarEn`: la plaza propia, siempre; la de una aliada o vasalla (o la señora), si lo permite. Lo miran `Repostar` y `CargarCaravana` (la duda 3 de 5G-a). Repostar en una avanzada sigue siendo solo de la propia Facción.
- **`radioReabastecimiento` desaparece** (Doc 5.13): recoger tropas, adjuntar y cargar caravanas y entregar un trueque se hacen en la **puerta**.

## 9. Caravanas adjuntas

- **`adjuntarCaravana`**: **solo dentro del lugar donde está la caravana** (una plaza, o un campamento para la Caravana de Fundación), nunca en mundo abierto; misma Facción; no ya enganchada; estado `disponible` o `aparcada`. La Caravana de Fundación la lleva su titular: otro solo la reclama si el titular ya no es vigente. No se comprueba el cupo de Mercado: engancharla ya la saca de comerciar.
- Una adjunta pasa a `estado: 'adjunta'` y su posición **es la de la columna**.
- **`soltarCaravana`**: **solo dentro de una plaza** (decidido el 2026-10-19; no hay caravanas sueltas por el mapa), la plaza **dentro de la cual está el ejército** (entró con todo; a su puerta se está fuera y se rechaza, decisión del autor 5G-a-5): la caravana queda `aparcada` en ella (la orden es explícita: entrar con la columna ya no aparca las caravanas, §11). La Caravana de Fundación se suelta solo dentro de su origen, la plaza o el campamento donde se enganchó, y queda `disponible` (implementado en Rust, decisión del autor 5G-b-4, 2026-10-24: `soltarCaravana` con su id; Fundacion_Y_Refundacion §7). En mundo abierto se rechaza.
- **`reclamarCaravana`**: en mundo abierto, un héroe de la misma Facción, junto a la **Caravana de Fundación que vuelve sola** a su origen, la reclama: pasa a su columna. **Reclamarla convierte al instante su columna personal en un ejército**, con él de Líder y la política de unión que elija; si ya iba en un ejército, la suma a ese. Vale solo para ella: las caravanas comerciales no quedan sueltas en el mapa.
- **`cargarCaravana { ejercitoId, caravanaId, asentamientoId, recurso, cantidad }`**: carga de una plaza con el ejército **en su puerta** y donde pueda repostar; `min(pedido, espacio, disponible)`; sin reserva. Devuelve `{ cargado }` y emite `ejercito.caravana_cargada`. Rechaza si la caravana no va con ese ejército, la cantidad no es positiva, la plaza no abre su almacén (`puedeRepostarEn`) o la caravana va llena (capacidad efectiva, con «Carga Ampliada» del origen).
- **Capacidad de carga** del ejército = `participantes × capacidadCarroPorJugador` + Σ **hueco libre** de las adjuntas (`capacidadCaravana − lo que ya lleva`; decisión del autor del 2026-10-24, 5G-a-4: cada carro tiene un espacio fijo y nunca se llena por encima de su límite, así que una adjunta llena no da espacio). Lo cargado y el botín van al hueco que quede (primero los carros de los héroes) y lo que no cabe se pierde. **La carga sigue a los carros** (decisión del autor del 2026-10-24, calibración del sistema 4, M10; hallado por `batch militar`): aparcar una adjunta (`aparcarCaravana`) quita su capacidad y **lo que la columna ya no puede llevar sin ella se queda en la caravana aparcada**, hasta lo que le quepa (lo que tampoco quepa en ella se pierde); `ejercito.caravana_aparcada` lleva `dejado`. Perderla sin que lo pida nadie (la destruyen los bandidos o un asedio, Unity da por perdidos sus carros, la columna derrotada con números pierde la mitad de los carros de cada adjunta) **pierde lo que iba en sus carros**: lo que la columna ya no puede llevar sin ellos sale con ellos (`Nucleo::sacar_lo_que_no_cabe`, del último recurso al primero, `militar::sacar_lo_que_sobra`) y, cuando hay vencedor, es botín como el resto de la carga de la caravana. El carro nunca pasa de su capacidad. Pruebas: `nucleo/tests/entrar.rs::al_aparcar_la_caravana_lo_que_la_columna_ya_no_puede_llevar_se_queda_en_ella` y `nucleo/tests/combate.rs::una_caravana_adjunta_a_un_ejercito_se_defiende_con_el_poder_de_la_columna_que_no_sufre_bajas`.
- Si el **origen** de una adjunta **cae en ruinas**, la adjunta se pierde con su plaza y con su carga, y lo que la columna ya no puede llevar sin sus carros se pierde con ella (M10; `ejercito.caravanas_perdidas` con `ruina`; decisión del autor del 2026-10-25, P7, [`Comercio_Caravanas.md`](Comercio_Caravanas.md) §10).
- Si el ejército es **derrotado**, cada adjunta pierde la mitad de sus carros con su carga (`ejercito.caravanas_perdidas`; la que se queda sin carros desaparece; decisión del autor, 2026-10-24, Combate §5). Si su gente se desconecta y se queda sin nadie, vuelven solas a su origen (Doc 1.10.6, 3.10).

> **Implementado en Rust (2026-10-24, 5G-a: almacén personal y caravanas a mano).** `crates/nucleo/src/nucleo/a_mano.rs`; sin cambio de foto (no hay estado nuevo). Órdenes **138-142** (del 128 al 137 quedan para 5B), todas en unidades enteras (C3b: lo que se mueve se redondea hacia abajo):
> - **`GuardarEnAlmacenPersonal { heroe, recurso, cantidad }`** (138) y **`SacarDelAlmacenPersonal`** (139): **dentro de su residencia** (decisión del autor 5G-a-2), con el carro que trajo (500 u., o el de la columna que conserva para sus caravanas); o **fuera**, siendo el Líder de su columna (`columna.solo_el_lider`), a bordo (`heroe.sin_columna`) y con la columna a `radioPuerta` de la puerta de su residencia, plaza o campamento (`almacen_personal.fuera_de_tu_residencia`). Sin residencia, `heroe.sin_residencia`. Guardar mueve `min(pedido, lo que lleva el carro, ALMACEN_PERSONAL.capacidad − total)` (`carro.no_lleva`, `almacen_personal.lleno`); sacar, `min(pedido, lo guardado, hueco del carro)` (`almacen.sin_existencias`, `carro.no_cabe`). Menos de una unidad: `cantidad.no_positiva`. Hecho `heroe.almacen_personal` `{ heroe, recurso, cantidad, sentido }`.
> - **`CargarCaravana { ejercito, caravana, asentamiento, recurso, cantidad }`** (140): un participante; la caravana adjunta a esa columna (`caravana.no_esta_adjunta`), la columna a la puerta de la plaza (`asentamiento.fuera_de_la_puerta`), una plaza de su Facción (`caravana.plaza_ajena`: las aliadas con `permiteReabastecerAliados` son de 5D); `min(pedido, capacidad efectiva − lo que lleva, existencias)`, sin pasar del hueco que le queda a la columna (5G-a-4) y sin reserva (`caravana.sin_capacidad`, `almacen.sin_existencias`). Hecho `ejercito.caravana_cargada` `{ caravana, plaza, recurso, cargado }`.
> - **`SoltarCaravana { ejercito, caravana, heroe }`** (142): un participante, que es el héroe que la orden nombra; solo **dentro** de la plaza en la que entró la columna (el héroe está dentro con ella); a la puerta o en mundo abierto, `caravana.soltar_solo_dentro` (5G-a-5). Es `aparcarCaravana` en esa plaza (de su Facción, `caravana.plaza_ajena`; la carga sigue a los carros, M10) con su propio hecho, `ejercito.caravana_soltada` `{ caravana, plaza, dejado, heroe }`. Si era la última adjunta de la columna que un residente conserva en su residencia, la columna se deshace como al entrar sin caravanas (el carro, con el héroe). `aparcarCaravana` (40) es la misma acción y sigue la misma regla (decisión del autor del 2026-10-24): solo dentro, con `caravana.soltar_solo_dentro`; si era la última adjunta de la columna de un residente, la columna también se deshace.
> - **`ReclamarCaravana { heroe, caravana, politica }`** (141): el héroe a bordo de su columna; solo vale para la Caravana de Fundación que vuelve sola. **Caso de éxito desde 5G-b (2026-10-24)**: un héroe de su Facción (`caravana.faccion_ajena`), a bordo (`heroe.sin_columna`) de una columna que no sea una formación (`columna.en_formacion`), a `radioEncuentro` o menos de ella (`caravana.lejos`), la reclama: pasa a ser su titular y va con su columna; una columna personal se convierte al instante en un ejército con él de Líder y la `politica` de la orden; si ya iba en un ejército, va con él. La que no vuelve sola, `caravana.no_reclamable` (también cualquier comercial). Hecho `ejercito.caravana_adjuntada { caravana, heroe, reclamada: 1 }`. La regla de cuándo vuelve sola (la columna desaparece, el titular sale de ella o deja de ser ciudadano) está en Fundacion_Y_Refundacion §7. **`adjuntarCaravana`** acepta también la Caravana de Fundación suelta en su origen: un ejército de su Facción, a su puerta, en el que va su titular (`fundacion.no_titular`).
> - **Capacidad (5G-a-4):** `Nucleo::capacidad_de_carga` suma solo el hueco libre de cada adjunta; la leen el botín, `DarSuministro`, las compras, aparcar y `cargarCaravana`.
> - Pruebas: `crates/nucleo/tests/a_mano.rs` (cada orden, cada rechazo, la conservación, una adjunta llena que no da espacio y dos por el WebSocket real).

## 10. Juntar y separar

### 10.1 Unirse en tu plaza (`unirseAEjercito`)
Residente de esa plaza; mismo bando; ejército **en la puerta** de la plaza; la política `rechazar` cierra la puerta (salvo reforzar el ejército en el que ya vas); selecciona escuadras como §3.1; liderazgo por héroe. Sumar tropa a un ejército donde ya vas no te hace dos participantes. Quien conserva en su residencia la columna con la que lleva sus caravanas (§11, un ejército) no se suma así: su columna tendría que fundirse con la del que pasa y solo se funde una columna personal de la misma Facción (`columna.solo_una_personal`, como al partir con una convocatoria; hallado por `batch militar` el 2026-10-24: el núcleo fallaba con la salida ya aplicada).

### 10.2 Unirse en campo (`unirseEnCampo`)
- El ejército no rechaza; es `ejercito` **o** una formación; la que se une es `personal` sin formación; ambas de la misma Facción; a ≤ `radioEncuentro` (3).
- Aporta lo que ya lleva: escuadras, carro y víveres. `unidoEn = ahora`. **Adopta el rumbo del ejército y ya no puede rectificar**.
- Con `preguntar`: la petición vive `vidaPeticionUnionSegundos`; el silencio vale como «no». **`responderPeticionDeUnion { heroeId (el Líder), ejercitoId, solicitanteId, aceptar }`**: solo el Líder, con petición viva. Rechazar emite `columna.union_rechazada`; aceptar ejecuta `unirseEnCampo` con la geometría **de ese instante** (el solicitante tiene que seguir a ≤ `radioEncuentro`), completa la formación si llega a 3 y emite `columna.union_en_campo`.

### 10.3 Separarse (`separarseDelEjercito` → `desgajar`)
- Debe ir dentro; de una columna personal no te separas; **el Líder no puede** (cede antes) salvo en una formación; **el último tampoco**.
- `desgajar`: nace una columna personal `estacionado` donde estás, con **tus escuadras**, **un carro como mucho** (`min(capacidad de 1 carro, ⌈total / nº de participantes⌉)`, a prorrata por recurso sin perder milésimas: lo que el redondeo de cada recurso deja se toma de milésima en milésima de los recursos en su orden) y **tu parte igual de los víveres del ejército**. La parte va hacia arriba para que quien se queda —el Líder, al desarmar— no pase de un carro por el redondeo (`batch militar`, 2026-10-24: con la parte hacia abajo se quedaba con 500,001 de un carro de 500). **No hay origen que heredar ni plaza a la que replegarse**: la tropa que va con su héroe se queda con su héroe (Doc 5.15.2b).
- `cederLiderazgo`: solo el Líder, a otro participante.

### 10.4 Formar un ejército en campo
- `organizarEjercito`: una columna personal sin formación y con **un** participante queda quieta con `formacion.expiraEn = ahora + FORMACION_EJERCITO.plazoMinutos`. Evento a esa caducidad.
- `completarFormacion`: con ≥ `FORMACION_EJERCITO.minimo` (3) participantes pasa a `tipo: 'ejercito'` y pierde la marca.
- Al vencer `expiraEn` sin llegar a 3, la formación se deshace (`columna.formacion_disuelta`).
- `disolverFormacion` (también usada para **desarmar** un ejército que entra en un lugar): el Líder se queda con la columna (queda `personal`, política `rechazar`, sin peticiones **ni caravanas**) y cada otro nace como personal donde estaba, vía `desgajar`.
- **`cancelarFormacion { heroeId }`**: solo el Líder de una formación; aplica el deshacer y emite `columna.formacion_disuelta`.

## 11. Terminar la campaña

- **Disolución por quedarse sin nadie dentro** (`participantes.length == 0`): un ejército no guarda tropa por su cuenta (tropa, carro y víveres son de cada héroe y salen con él), así que **no hay tropa que devolver**. Las adjuntas vuelven solas a su origen si la causa fue la desconexión, y se pierden si fue una derrota. Evento `ejercito.disuelto`. **Perder todos los soldados no basta**: los jugadores siguen dentro y viajan a pie (Doc 5.13.4).
- **Entrar en una plaza** (`guarnecer`): solo un ejército, a ≤ `MOVIMIENTO.radioPuerta` (2) de una plaza que le abre su puerta (la de su Facción siempre; el resto, según el acceso por relación que fije el dueño). Lo ordena el Líder y se **desarma** en la puerta con `disolverFormacion`; cada héroe entra como lo haría solo: los que **residen** allí, normal (la tropa al campamento y el carro **con ellos**, que lo vacían con `vaciarCarro`); los que no, **de visita** con su columna entera, dentro. Una puerta cerrada les alcanza como a cualquiera. (Antes solo se guarnecía una plaza propia, y el carro se volcaba solo en el almacén.)
- **Entrar en un campamento** (`entrarEnCampamento`): se entra **solo** (un ejército con más gente se desarma antes), a ≤ 2 de su puerta; el residente deja la tropa y se queda con el carro (`vaciarCarro` lo pasa al almacén personal, hasta `ALMACEN_PERSONAL.capacidad`; lo que no cabe se queda en el carro); el visitante entra con su columna entera.
- **Caravanas al entrar** (decidido el 2026-10-22): se entra con **todas** las adjuntas, sean del origen que sean, y se sale con ellas. **No se aparcan**: siguen con la columna (la del Líder, si el ejército se desarma al entrar) mientras no se suelte la orden explícita `aparcarCaravana`. Si la plaza de su origen cae, siguen la suerte de su origen como las que van por el mapa. (Antes solo entraba si **todas** eran de ese lugar y en una plaza pasaban a `aparcada`.) La caravana aparcada **dentro de una plaza** (Doc 3.13.7) no cambia: si la plaza cae, se destruye.

### 11.1 «A la puerta» y «dentro» (decisión del autor, 2026-10-22)

Dos estados que valen para todo lo que se puede entrar: una plaza, un campamento de mercenarios, un pueblo extractor, una avanzada o cualquier otro ente en el que se pueda entrar.

- **A la puerta** es estar **fuera, en el mapa del mundo, muy cerca del elemento** (a ≤ `radioPuerta` cuando la acción lo pide). Desde ahí se hacen las interacciones que **no son defensa** —entregar materiales a una obra, depositar en un almacén, repostar, comerciar, recoger tropas—, y esas no cambian. Pero una columna a la puerta **no defiende**.
- **Dentro** es haber hecho la interacción de entrar (`entrarEnAsentamiento`, `guarnecer`, `entrarEnCampamento`). Quien entra lo hace **con todo lo suyo**: su columna entera queda dentro (ya no está en el mapa: no se la ve, ni se la persigue, ni se la ataca) y sale con él. No existe la «columna aparcada a la puerta».
- **Solo lo que está dentro cuenta para la defensa**, y solo si es de la misma Facción, aliado o vasallo del dueño: neutrales y enemigos no defienden nunca, aunque estén dentro (a un enemigo solo entra si su dueño le abre la puerta). Por los dos caminos. Con números ([`Combate.md`](Combate.md) §3.2): la guarnición, los residentes y los visitantes amigos que están dentro; una avanzada y un pueblo, solo sus guarniciones.
- **Si el lugar cae** (conquista o ruina) quien estaba dentro y no podía defender **no pierde nada**: sale al mapa con lo que tenía (columna, tropa, carro, víveres y caravanas). Quien defendió queda sujeto a lo que le pase en la defensa.
- **Acceso por relación**: la misma Facción entra siempre; los aliados y vasallos, los neutrales y los enemigos, según lo que decida el dueño (por defecto, todos menos los enemigos, hasta el sistema político). Los pueblos extractores y las avanzadas heredan la decisión de su plaza madre. Con Unity ([`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §4 y §7): el defensor del ticket y quien se une a defender un asedio, un pueblo o una avanzada ha de estar dentro (`union.rs`: `Reparto::dentro`); una columna del mapa solo puede unirse al ataque.
- **Una caravana no se entra**, va por el mapa: no tiene puerta ni dentro, y quien se une a defenderla lo hace desde el mapa.
- **En los pueblos extractores y las avanzadas se entra como en una plaza** (2026-10-22: `EntrarEnPueblo` y `EntrarEnAvanzada`, con el acceso de la plaza madre): defienden su guarnición (la de serie, y en la avanzada la de jugadores, que son escuadras que ya viven en ella) y los héroes que están dentro, de la Facción de la madre, aliados o vasallos. **Al ataque solo se une quien está fuera; a la defensa, quien está dentro.**

> **Implementado en Rust (2026-10-22, entrar con todo, segunda ronda).** Órdenes `EntrarEnPueblo`/`SalirDePueblo`/`EntrarEnAvanzada`/`SalirDeAvanzada` (119-122), expulsión al cerrar la puerta, desalojo y carro del residente derrotado, foto `BAN18`: [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md) y [`Combate.md`](Combate.md) §9.

> **Implementado en Rust (2026-10-22, entrar con todo, tercera ronda).** Foto `BAN18` (sin cambios). Todo defensor derrotado pierde la mitad del carro, la conquista de la madre solo saca de sus pueblos y avanzadas a quienes el acceso heredado ya no deja entrar, y en las batallas de Unity entra de oficio solo la guarnición (los de dentro se unen; desde fuera se unen al ataque neutrales y enemigos del dueño): [`Combate.md`](Combate.md) §9, [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md) y [`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §7.1.

> **Implementado en Rust (2026-10-22, entrar con todo).** [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md) recoge el detalle (órdenes `FijarAcceso` 117 y `VaciarCarro` 118, foto `BAN17`). Lo que cambia aquí: **no hay «columna aparcada a la puerta»**; el visitante entra con **todo** (tropa, carro, víveres y todas sus caravanas adjuntas, de donde sean) y su columna queda **dentro**, fuera del mapa, comiendo de sus víveres como un ejército acampado; el residente entra con el carro y **no lo vacía**; entrar ya no rechaza por caravanas de otro origen ni las aparca; el dueño de la plaza decide por relación quién puede entrar (los pueblos y las avanzadas heredan su decisión); y si el lugar cae (conquista o ruina) quien estaba dentro sin poder defender sale al mapa con lo suyo, sin perder nada.

> **Implementado en Rust (2026-10-22, decisiones de 4F).** `combate::union::Reparto` gana `dentro` y `ErrorUnion::HayQueEstarDentro` (`batalla.invalida`: «para defender hay que estar dentro»); `unirse_a_batalla` lo calcula (`presencia.ubicacion == Asentamiento(plaza)` en un asedio; siempre falso en un pueblo o una avanzada). La defensa con números (`defensa_de_plaza`, `atacar_pueblo`, `atacar_avanzada`) y la apertura de las batallas de Unity ya miraban solo lo de dentro; lo que cambia es que ya no se une a defender una columna aliada del mapa a la puerta.

## 12. Inspeccionar

A ≤ `MOVIMIENTO.radioInspeccion` (8): se ve la composición de una columna ajena (héroes y `{tropaId, cantidad, heroeId}` de cada escuadra con soldados), la defensa de una plaza ajena (guarnición de sus residentes y héroes dentro; **no** el almacén) o de una caravana (si va escoltada y qué recursos lleva, **no cuánto**). El observado recibe aviso: obtener información cuesta ser visto.

## 13. Qué programa cada columna

| Evento programado | Cuándo | Qué hace |
|---|---|---|
| Llegada | fin del plan | acampa (`ejercito.llega`) |
| Víveres a 0 | `víveres / ritmo de consumo` | empieza la degradación de moral |
| Deserción | moral ≤ 0 con soldados | `tropas.desercion` |
| Encuentro | cruce exacto de planes (Combate §2) | deja el hecho `*.encuentro` (4C) |
| Intercepción | punto calculado al perseguir (§7) | `columna.presa_alcanzada` |
| Presa fuera de vista | primer instante en que sale del alcance de vista (§7) | cancela la persecución (`columna.presa_perdida`) |
| Caducidad de petición o formación | `expiraEn` | retira la petición o deshace la formación |

Los eventos se descartan si la generación de la columna cambió. Sin ejércitos en combate, un mundo no consume el generador aleatorio por movimiento.

### 13.1 Eventos de los comandos de columnas

Cada uno lo emite el comando que lo nombra, en la misma transición. «Plaza» es el `asentamientoId` al que se atribuye (decide quién lo oye, [`Proyecciones_Y_Autorizacion.md`](Proyecciones_Y_Autorizacion.md) §8); sin plaza, es global del héroe.

| Evento | Comando | Plaza | Payload |
|---|---|---|---|
| `ejercito.movilizado` | `movilizarEjercito` | la de salida | `{ ejercitoId, heroeId, escuadronIds, liderazgoUsado, trigoCargado }` |
| `ejercito.refuerzo` | `unirseAEjercito` | la desde la que se refuerza | `{ ejercitoId, asentamientoId, heroeId, escuadronIds, trigoCargado }` |
| `ejercito.estacionado` | `estacionarEjercito` | — | `{ ejercitoId }` |
| `ejercito.caravana_adjuntada` / `ejercito.caravana_soltada` | `adjuntarCaravana` / `soltarCaravana` | la del lugar | `{ ejercitoId, caravanaId, heroeId }` |
| `columna.formacion_iniciada` | `organizarEjercito` | la de la formación | `{ ejercitoId, liderId }` |
| `columna.separacion` | `separarseDelEjercito` | la del ejército | `{ ejercitoId, columnaId, heroeId }` |
| `columna.liderazgo_cedido` | `cederLiderazgo` | la del ejército | `{ ejercitoId, anteriorId, sucesorId }` |
| `columna.persecucion_iniciada` | `perseguir` | — | `{ heroe, presa }` (una columna) o `{ heroe, caravana }` (una caravana) |
| `columna.presa_alcanzada` / `columna.presa_perdida` / `columna.presa_a_cubierto` | el encuentro exacto con la presa / la presa sale de la vista o no hay camino / la presa sale del mapa | — (oyen las dos columnas) | `{ presa }` (una columna) o `{ caravana }` (una caravana), con su id (2026-10-24) |
| `columna.persecucion_abandonada` | `dejarDePerseguir` | — | `{ heroe }` |
| `ejercito.encuentro` / `caravana.encuentro` / `asentamiento.encuentro` | un cruce exacto a ≤ `radioEncuentro` (§13) | la plaza, en la última | `{ faccionA, faccionB }` más `caravana` (y `caravanaB`) si la hay |
| `ejercito.en_batalla` / `ejercito.sale_de_batalla` | el hook de la 4E | — | `{}` |
| `asentamiento.observado` / `columna.observada` / `caravana.observada` | `inspeccionar` (§12) | la **observada** | `{ observador }` (el héroe que mira) |

`ejercito.repliegue` y `replegarEjercito` **se retiran** (no hay vuelta a casa).

> **Implementado en Rust (2026-10-23, 4H T5).** «Sin plaza, global del héroe» es, en Rust, **del héroe**: un hecho de columna sin plaza lo oyen los participantes de la columna (los de `ejercitos` y los de la clave `ejercito`, resueltos al emitirlo, así que la columna que se disuelve en la misma orden sigue nombrándolos) y el héroe de las claves `heroe` y `lider`, no todo el mundo (la niebla: contrato 05 §8). Los encuentros los oyen además las dos Facciones de sus claves `faccionA`/`faccionB`; `faccion.relacion`, las dos Facciones. `ejercito.caravana_cargada` y `ejercito.caravana_soltada` se emiten desde 5G-a (2026-10-24). `ejercito.caravanas_perdidas` se emite desde el 2026-10-24 (Combate §5).

## 14. Parámetros

**Los bloques de este documento son los valores vigentes de la reconstrucción** (reescritos el 2026-10-19, decisión del autor para el sistema 4): ya no copian `constants.ts`. Respecto al motor TypeScript cambian, tras la calibración (Calibracion_Distancias_Definicion §4):

| Parámetro | Motor TypeScript | Vigente (en el bloque) |
|---|---|---|
| `MOVIMIENTO.velocidadJugador` | 22 | **33** |
| Velocidades de tropa (`TROPAS_RECLUTABLES`, [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md)) | 28 / 20 / 16 / 12 | **42 / 30 / 24 / 18** |
| `MOVIMIENTO.radioPuerta` | 10 | **2** |
| `MOVIMIENTO.radioInspeccion` | 40 | **8** |
| `LOGISTICA.radioEncuentro` | 15 | **3** |
| `LOGISTICA.radioReabastecimiento` | 60 | **retirado** (§8) |
| `VISION.ejercito` | 150 | **30** |
| `VISION.jugadorSolo` | 80 | **15** |
| `VISION.margenAsentamiento` | 60 | **0** (la plaza ve solo su zona; las avanzadas ven 12) |
| `VISION.caravanaGrande.radio` | 300 | **60** |
| `PERSECUCION.umbralRecalculo` | 30 | **retirado** (§7) |

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

<!-- parametros:LOGISTICA -->
```json
{
  "capacidadCarroPorJugador": 500,
  "capacidadViveresPorHeroe": 350,
  "factorConsumoEnMarcha": 0.25,
  "factorConsumoEstacionado": 0.1,
  "umbralNarrarReposte": 0.5,
  "radioEncuentro": 3
}
```
<!-- /parametros -->

<!-- parametros:FORMACION_EJERCITO -->
```json
{
  "minimo": 3,
  "plazoMinutos": 10
}
```
<!-- /parametros -->

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

`MOVIMIENTO` reúne lo que no es de logística: `velocidadJugador`, `consumoPorParticipante`, `radioPuerta`, `vidaPeticionUnionSegundos`, `radioInspeccion` y `fraccionRobada` (la mitad del carro del derrotado).

<!-- parametros:MOVIMIENTO -->
```json
{
  "velocidadJugador": 33,
  "consumoPorParticipante": 0.5,
  "radioPuerta": 2,
  "vidaPeticionUnionSegundos": 10,
  "radioInspeccion": 8,
  "fraccionRobada": 0.5
}
```
<!-- /parametros -->

## 15. Invariantes

- La posición de una columna es función de `(plan, instante)`; salvo `desgajar` y las entradas en lugares, nada la mueve fuera de §4.2.
- Cada héroe va como mucho en una columna.
- Un ejército en marcha no cambia de rumbo: solo su Líder lo hace con `marcharA`, o lo detiene con `estacionarEjercito`.
- Una caravana adjunta está en `caravanasAdjuntasIds` de **exactamente** un ejército y su posición es la de él.
- Ni el hambre ni el reposte pueden dejar a una plaza por debajo de su `reservaDeTrigo`.
- Ninguna tropa vuelve sola a ningún sitio: la que va con su héroe se queda con él (Doc 5.15.2b).
- Los eventos de una columna con generación vieja nunca se aplican.

## 16. Casos límite y decisiones que parecen raras

- **El tipo no se deriva del número de miembros** (§2): derivarlo reabriría el agujero de rectificar rumbo en un ejército reducido a uno.
- **Cancelar la marcha es del Líder, no atrapa a nadie**: el que no quiera seguir se separa.
- **Una escuadra mermada paga su Liderazgo completo**: si se midiera por soldados vivos, la tropa cara se podría sacar «en dosis pequeñas» (regla en Doc 5.11.1).
- **Una orden a mitad de camino parte de la posición real del instante de la orden** (Reloj_Y_Eventos §10).
- **Los eventos de deserción llevan `asentamientoId`**: sin él saldrían globales.
- **Una columna que cruza una frontera de zona** deja de repostar sola en una plaza y empieza a hacerlo en la otra, en el instante exacto del cruce.

## 17. Divergencias

Con el motor TypeScript (a rehacer en Rust): `regresando`, `origenAsentamientoId`, `replegarEjercito`, `radioReabastecimiento`, la disolución que devolvía la tropa «a la plaza de origen» y la posición por tick. El canon (Doc 5.12-5.14) es el que manda.

> **Implementado en Rust (2026-10-20, fase 4D: atacar).** `Atacar { heroe, objetivo, declara_guerra }` (95) con seis objetivos (`Ejercito`, `Caravana`, `Campamento`, `Asentamiento`, `Pueblo`, `Avanzada`): el héroe sano de la columna ataca lo que tiene a su alcance (el encuentro de 4C o el radio de ataque) y el combate se resuelve al instante con `combate::resolver_*`; el encuentro hostil que una orden pide pasa a combate numérico. Las consecuencias (heridos, mitad del carro del derrotado, botín, chatarra, experiencia de escuadra, deserción) se aplican a las columnas reales; la escolta de una caravana derrotada pierde soldados. Sin Unity (4E) se pierden los últimos carros. Rechazos nuevos `combate.*` y `heroe.herido`. **Recortes:** atacar a un aliado se rechaza, la guerra se declara por pareja de Facciones sin arrastrar aliados y sustituye al vasallaje (anotado en las filas de paridad); el asedio no reúne varias columnas (sin convocatoria).

> **Implementado en Rust (2026-10-23, 4H T4: el informe de inspección en el contrato).** Foto `BAN21`. El informe de una plaza ajena guarda también sus recintos de muralla con alguna celda levantada (nivel, anillo entero y celdas levantadas) y los tres informes (columna, caravana, plaza) llegan al ámbito `HEROE` de quien inspeccionó como `InformeInspeccion` (`insp-<héroe>-<objetivo>`, hasta su `expira_en_ms`; el cliente lo descarta al caducar). Ver el contrato 05 §9.

## Catálogo de eventos

Los códigos de los hechos de dominio de este documento (4H T5, 2026-10-23; contrato 05 §8). La prueba `crates/nucleo/tests/catalogo.rs` lee el bloque de cada documento de `Docs/Mechanics` y exige que todo código que emite el núcleo esté en alguno, y que los que el núcleo aún no emite figuren como paridad pendiente en `Docs/Mecanicas a desarrollar.md`. El núcleo los emite todos (`logistica.reabastecer_aliados` desde 5D, 2026-10-24; `faccion.relacion`, el del andamiaje `fijarRelacion`, se retiró en 5D: la administración emite los `diplomacia.*`).

<!-- eventos -->
```json
[
  "asentamiento.encuentro", "asentamiento.observado", "avanzada.encuentro", "caravana.encuentro", "caravana.observada", "columna.formacion_completada",
  "columna.formacion_disuelta", "columna.formacion_iniciada", "columna.liderazgo_cedido", "columna.observada", "columna.persecucion_abandonada",
  "columna.persecucion_iniciada", "columna.peticion_caducada", "columna.presa_a_cubierto", "columna.presa_alcanzada", "columna.presa_perdida", "columna.separacion",
  "columna.union_en_campo", "columna.union_pedida", "columna.union_rechazada", "ejercito.caravana_adjuntada", "ejercito.caravana_aparcada", "ejercito.caravana_cargada",
  "ejercito.caravana_soltada", "ejercito.caravanas_perdidas", "ejercito.disuelto", "ejercito.en_batalla", "ejercito.encuentro", "ejercito.estacionado", "ejercito.llega",
  "ejercito.movilizado", "ejercito.reabastecido", "ejercito.refuerzo", "ejercito.sale_de_batalla", "ejercito.sin_viveres", "ejercito.viveres_bajos",
  "logistica.reabastecer_aliados"
]
```
<!-- /eventos -->
