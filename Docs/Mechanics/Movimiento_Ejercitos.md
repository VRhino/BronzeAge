# Movimiento de ejércitos y columnas

## 1. Alcance y fuentes

Cubre cómo se **mueve, come, se junta y se separa** una columna en el mapa: salir, marchar, perseguir, repostar, formar ejércitos, enganchar caravanas y entrar en plazas y campamentos. El **combate** que ocurre cuando dos columnas, una caravana, un campamento o una plaza se encuentran está en [`Combate.md`](Combate.md); aquí solo se dice dónde empieza.

- **Reescrito el 2026-10-13** para el mundo reactivo (sin tick; [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md)) y la calibración de distancias. El «cómo funcionaba» del servidor TypeScript (por tick, con `regresando` y `origenAsentamientoId`) está en `typescript-final:Docs/Mechanics/Movimiento_Ejercitos.md`.
- **Canon:** Doc 5.11 (Liderazgo), 5.12 (movimiento y encuentros), 5.13 (suministro), 5.14 (ejércitos), 5.15.2b (tropa del héroe y tropa prestada), 1.10 (el jugador en el mundo).
- **Fuera de alcance aquí:** el grafo de navegación y la red de caminos (`Comercio_Caravanas.md` §7), la niebla ([`Niebla_De_Guerra.md`](Niebla_De_Guerra.md)) y el generador aleatorio.

> **Unidades.** El mapa mide **2.667 × 2.000** unidades. Las velocidades son «unidades de mapa por minuto» **antes** del coste del terreno; el reloj es continuo. Un ejército **no tiene destino fijo ni vuelve solo a ningún sitio**: sale quieto a la puerta y su Líder lo dirige con sus clics (Doc 5.12).

> **Implementado en Rust (2026-10-19, fase 4A-1: reglas puras).** Crate `crates/militar`, sin posición ni plan (son del núcleo, fase 4B-2): `racion` (§6: la ración de una columna y de una guarnición, los víveres como **ritmo** con el evento «víveres a 0», `llenar` y `pasarAViveres`; la moral y la deserción están en `escuadra`), `columna` (§2, §4.1, §5, §10 y parte de §11: la composición y el mando de una columna; salir, `unirseEnCampo`, `unirseAEjercito`, peticiones de 10 s, `separarseDelEjercito`, `desgajar`, `cederLiderazgo`, la sucesión por antigüedad, formar un ejército en campo, `desarmar`; la velocidad como mínimo y el alcance de vista) y `persecucion` (§7: las condiciones y el alcance). Cada operación que cambia la gente, el carro o el estado deja los víveres con la capacidad (suma de sus héroes) y el consumo bien anotados. **Decisiones del autor del 2026-10-19 aplicadas en este documento:** persecución solo con la presa a la vista y se cancela si sale de ella (sin `umbralRecalculo`, §7); una caravana solo se suelta dentro de una plaza (§9); los bloques son los valores vigentes (§14). **Fase 4B-1 (2026-10-20):** el hambre de la **guarnición** (§6.3, último párrafo) ya está en el núcleo, minuto a minuto dentro del paso de la plaza (véase [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md)); `pasarAViveres` (`PasarAViveres`, 60) y repostar gratis en un campamento ya llenan los víveres de la columna del héroe. Lo que **no** está todavía: repostar (§8), las caravanas adjuntas dentro de la columna (§9, `caravanasAlEntrar`, derrota), inspeccionar (§12) y el resto de §11 (disolución sin nadie, entrar en un lugar).

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
- El **no residente** retoma la columna con la que entró (aparcada a la puerta); sin ella, error.
- El **residente** elige tropa de su campamento y carga del **almacén personal**: cada recurso pedido debe estar y caber en lo que queda del carro.
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

- **`perseguir`** fija `persiguiendo = {tipo, id}` y `estado: marchando`. Condiciones: no a sí mismo; la columna propia con algún héroe sano; presa de la **misma clase** (`personal` con `personal`, `ejército` con `ejército`) y, si es una columna, con algún héroe sano. Antes: no a los tuyos ni a un aliado, no a una caravana adjunta, y **un ejército no persigue caravanas** (las persigue una columna personal). **Solo se inicia con la presa a la vista** (decidido el 2026-10-19): a ≤ `alcanceDeVista` del perseguidor (`VISION.ejercito` = 30 si lleva soldados, si no `VISION.jugadorSolo` = 15); si no, se rechaza. **Solo se inicia con la presa a la vista** (decidido el 2026-10-19): a ≤ `alcanceDeVista` del perseguidor (`VISION.ejercito` = 30 si lleva soldados, si no `VISION.jugadorSolo` = 15); si no, se rechaza.
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
- **En una avanzada con almacén** (fuerte o ciudadela, Doc 5.17): un ejército de la Facción reposta en su puerta con la misma orden.
- **Dónde se puede (`puedeRepostarEn`):** plaza **propia**, siempre; **aliada** —alianza o vasallaje, en cualquier dirección— solo si tiene `permiteReabastecerAliados`; neutral u hostil, nunca; **campamento de mercenarios**, cualquiera y gratis.
- Con las mismas reglas que llenarlos al salir: hasta donde quepa y sin bajar de la reserva de trigo de la plaza. Solo se **narra** (`ejercito.reabastecido`) si llevaba menos de `umbralNarrarReposte` (50 %).
- **`alternarReabastecerAliados { asentamientoId, permitido }`**: abre o cierra el almacén de la plaza a los ejércitos aliados (Gobernador o Tesorero). Sin cambio → `sinCambios`; con cambio → `logistica.reabastecer_aliados`. Cerrar no es retroactivo.
- **`radioReabastecimiento` desaparece** (Doc 5.13): recoger tropas, adjuntar y cargar caravanas y entregar un trueque se hacen en la **puerta**.

## 9. Caravanas adjuntas

- **`adjuntarCaravana`**: **solo dentro del lugar donde está la caravana** (una plaza, o un campamento para la Caravana de Fundación), nunca en mundo abierto; misma Facción; no ya enganchada; estado `disponible` o `aparcada`. La Caravana de Fundación la lleva su titular: otro solo la reclama si el titular ya no es vigente. No se comprueba el cupo de Mercado: engancharla ya la saca de comerciar.
- Una adjunta pasa a `estado: 'adjunta'` y su posición **es la de la columna**.
- **`soltarCaravana`**: **solo dentro de una plaza** (decidido el 2026-10-19; no hay caravanas sueltas por el mapa): la caravana queda `aparcada` en ella, como al entrar con la columna (§11). La Caravana de Fundación se suelta dentro del campamento donde se enganchó y queda `disponible`. En mundo abierto se rechaza.
- **`reclamarCaravana`**: en mundo abierto, un héroe de la misma Facción, junto a la **Caravana de Fundación que vuelve sola** a su origen, la reclama: pasa a su columna. **Reclamarla convierte al instante su columna personal en un ejército**, con él de Líder y la política de unión que elija; si ya iba en un ejército, la suma a ese. Vale solo para ella: las caravanas comerciales no quedan sueltas en el mapa.
- **`cargarCaravana { ejercitoId, caravanaId, asentamientoId, recurso, cantidad }`**: carga de una plaza con el ejército **en su puerta** y donde pueda repostar; `min(pedido, espacio, disponible)`; sin reserva. Devuelve `{ cargado }` y emite `ejercito.caravana_cargada`. Rechaza si la caravana no va con ese ejército, la cantidad no es positiva, la plaza no abre su almacén (`puedeRepostarEn`) o la caravana va llena (capacidad efectiva, con «Carga Ampliada» del origen).
- **Capacidad de carga** del ejército = `participantes × capacidadCarroPorJugador` + Σ `capacidadCaravana` de las adjuntas.
- Si el ejército es **derrotado**, las adjuntas se pierden (`ejercito.caravanas_perdidas`). Si su gente se desconecta y se queda sin nadie, vuelven solas a su origen (Doc 1.10.6, 3.10).

## 10. Juntar y separar

### 10.1 Unirse en tu plaza (`unirseAEjercito`)
Residente de esa plaza; mismo bando; ejército **en la puerta** de la plaza; la política `rechazar` cierra la puerta (salvo reforzar el ejército en el que ya vas); selecciona escuadras como §3.1; liderazgo por héroe. Sumar tropa a un ejército donde ya vas no te hace dos participantes.

### 10.2 Unirse en campo (`unirseEnCampo`)
- El ejército no rechaza; es `ejercito` **o** una formación; la que se une es `personal` sin formación; ambas de la misma Facción; a ≤ `radioEncuentro` (3).
- Aporta lo que ya lleva: escuadras, carro y víveres. `unidoEn = ahora`. **Adopta el rumbo del ejército y ya no puede rectificar**.
- Con `preguntar`: la petición vive `vidaPeticionUnionSegundos`; el silencio vale como «no». **`responderPeticionDeUnion { heroeId (el Líder), ejercitoId, solicitanteId, aceptar }`**: solo el Líder, con petición viva. Rechazar emite `columna.union_rechazada`; aceptar ejecuta `unirseEnCampo` con la geometría **de ese instante** (el solicitante tiene que seguir a ≤ `radioEncuentro`), completa la formación si llega a 3 y emite `columna.union_en_campo`.

### 10.3 Separarse (`separarseDelEjercito` → `desgajar`)
- Debe ir dentro; de una columna personal no te separas; **el Líder no puede** (cede antes) salvo en una formación; **el último tampoco**.
- `desgajar`: nace una columna personal `estacionado` donde estás, con **tus escuadras**, **un carro como mucho** (`min(capacidad de 1 carro, total / nº de participantes)`, a prorrata por recurso) y **tu parte igual de los víveres del ejército**. **No hay origen que heredar ni plaza a la que replegarse**: la tropa que va con su héroe se queda con su héroe (Doc 5.15.2b).
- `cederLiderazgo`: solo el Líder, a otro participante.

### 10.4 Formar un ejército en campo
- `organizarEjercito`: una columna personal sin formación y con **un** participante queda quieta con `formacion.expiraEn = ahora + FORMACION_EJERCITO.plazoMinutos`. Evento a esa caducidad.
- `completarFormacion`: con ≥ `FORMACION_EJERCITO.minimo` (3) participantes pasa a `tipo: 'ejercito'` y pierde la marca.
- Al vencer `expiraEn` sin llegar a 3, la formación se deshace (`columna.formacion_disuelta`).
- `disolverFormacion` (también usada para **desarmar** un ejército que entra en un lugar): el Líder se queda con la columna (queda `personal`, política `rechazar`, sin peticiones **ni caravanas**) y cada otro nace como personal donde estaba, vía `desgajar`.
- **`cancelarFormacion { heroeId }`**: solo el Líder de una formación; aplica el deshacer y emite `columna.formacion_disuelta`.

## 11. Terminar la campaña

- **Disolución por quedarse sin nadie dentro** (`participantes.length == 0`): un ejército no guarda tropa por su cuenta (tropa, carro y víveres son de cada héroe y salen con él), así que **no hay tropa que devolver**. Las adjuntas vuelven solas a su origen si la causa fue la desconexión, y se pierden si fue una derrota. Evento `ejercito.disuelto`. **Perder todos los soldados no basta**: los jugadores siguen dentro y viajan a pie (Doc 5.13.4).
- **Entrar en una plaza propia** (`guarnecer`): solo un ejército, de la Facción de la plaza, a ≤ `MOVIMIENTO.radioPuerta` (2). Lo ordena el Líder y se **desarma** en la puerta con `disolverFormacion`; cada héroe entra como lo haría solo: los que **residen** allí, normal (tropa al campamento, carro al almacén); los que no, **de visita** con su columna aparcada a la puerta. Una puerta cerrada o un veto les alcanza como a cualquiera.
- **Entrar en un campamento** (`entrarEnCampamento`): se entra **solo** (un ejército con más gente se desarma antes), a ≤ 2 de su puerta; el residente deja tropa y vuelca el carro en su almacén personal hasta `ALMACEN_PERSONAL.capacidad` (lo que no cabe se queda en el carro, con la columna aparcada).
- **Caravanas al entrar** (`caravanasAlEntrar`): solo entra si **todas** las adjuntas son de ese lugar (si no, hay que soltarlas antes). En una plaza pasan a `aparcada` (si la plaza cae, se destruyen, Doc 3.13.7); en un campamento (Caravana de Fundación) quedan `disponible` y empiezan a caducar.

## 12. Inspeccionar

A ≤ `MOVIMIENTO.radioInspeccion` (8): se ve la composición de una columna ajena (héroes y `{tropaId, cantidad, heroeId}` de cada escuadra con soldados), la defensa de una plaza ajena (guarnición de sus residentes y héroes dentro; **no** el almacén) o de una caravana (si va escoltada y qué recursos lleva, **no cuánto**). El observado recibe aviso: obtener información cuesta ser visto.

## 13. Qué programa cada columna

| Evento programado | Cuándo | Qué hace |
|---|---|---|
| Llegada | fin del plan | acampa (`ejercito.llega`) |
| Víveres a 0 | `víveres / ritmo de consumo` | empieza la degradación de moral |
| Deserción | moral ≤ 0 con soldados | `tropas.desercion` |
| Encuentro | cruce exacto de planes (Combate §2) | ofrece atacar o interceptar |
| Intercepción | punto calculado al perseguir (§7) | `columna.presa_alcanzada` |
| Presa fuera de vista | primer instante en que sale del alcance de vista (§7) | cancela la persecución (`columna.presa_perdida`) |
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
| `columna.persecucion_iniciada` | `perseguir` | la de la columna | `{ ejercitoId, heroeId, objetivo }` |
| `asentamiento.observado` / `columna.observada` / `caravana.observada` | `inspeccionar` (§12) | la **observada** | `{ observadorId, objetivo }` |

`ejercito.repliegue` y `replegarEjercito` **se retiran** (no hay vuelta a casa).

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
