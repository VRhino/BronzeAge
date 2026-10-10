# Movimiento de ejércitos y columnas

## 1. Alcance y fuentes

Cubre cómo se **mueve, come, se junta y se separa** una columna en el mapa del mundo: salir, marchar, perseguir, repostar, formar ejércitos, enganchar caravanas, entrar en plazas y campamentos, y volver a casa. El **combate** que ocurre cuando dos
columnas, una caravana, un campamento o una plaza se encuentran está en [`Combate.md`](Combate.md) (pendiente); aquí solo se dice dónde empieza.

- **Módulos:** `src/engine/ejercitos.ts` (≈1.900 líneas: casi todo), `movimiento.ts` (avance sobre una polilínea), `persecucion.ts` (seguir a la presa), `formacion.ts` (formar y deshacer), `viveres.ts`, `liderazgo.ts`, `tropas.ts` (`avanzarRacion`), `src/world/rutas.ts` (A* sobre el grafo de navegación).
- **Verificado contra:** `05e4dcb` (2026-10-10).
- **Canon:** Doc 5.11 (Liderazgo), 5.12 (movimiento y encuentros), 5.13 (logística), 5.14 (ejércitos), 1.10 (el jugador en el mundo).
- **Fuera de alcance aquí:** la construcción del grafo de navegación y de la red de caminos (`world/grafoNavegacion.ts`, `engine/redCaminos.ts`), la niebla de guerra (proyección) y el generador de aleatoriedad. Este documento los usa como cajas negras.

> **Unidades.** 1 tick = 1 minuto de mundo (`duracionTickMs` = 60 000). Las distancias son **unidades de mapa** (mapa de 2.000 × 2.000). Las velocidades son «unidades de mapa por tick» **antes** del coste del terreno.

## 2. Modelo de datos

`Ejercito` (`domain/types.ts`) es la columna, sea personal o ejército. Campos que importan:

| Campo | Significado |
|---|---|
| `tipo` | `'personal'` (un viajero a lo suyo) o `'ejercito'`. **Se fija al nacer y no se deriva de cuánta gente va dentro**: un ejército al que se separan miembros hasta quedar uno sigue siendo ejército (y solo su Líder lo dirige). |
| `participantes[]` | `{ heroeId, unidoEn }`: quién va dentro, con independencia de que aporte tropa. `unidoEn` decide la sucesión del mando. |
| `liderId` | Quien manda. En una columna personal es su único participante y no significa nada. |
| `politicaDeUnion` | `rechazar` / `aceptar` / `preguntar`; la fija el Líder al salir y no cambia. |
| `peticionesDeUnion[]` | Solo con `preguntar`: `{ heroeId, pedidoEn, expiraEn }`. **Caducan sin temporizador**: se comparan con el instante al leerlas. Vida: `MOVIMIENTO.vidaPeticionUnionSegundos`. |
| `escuadronIds[]` | Las escuadras viven en su héroe (`Heroe.escuadrones`, `contenedor: ejército`); la columna solo guarda ids. El motor opera sobre una **vista** (`EjercitoConTropa`) con las escuadras puestas (`conTropa`/`sinTropa`, `engine/tropa.ts`). |
| `suministro` | El **carro**: carga (botín, materiales). **No es lo que se come.** |
| `caravanasAdjuntasIds[]` | Caravanas enganchadas (§9). |
| `objetivo` | `{asentamiento, id}` o `{punto}`. |
| `ruta` / `progreso` | Polilínea y fracción 0‥1 recorrida. |
| `posicionActual` | Se recalcula cada tick desde `ruta` y `progreso`. |
| `estado` | `marchando` · `estacionado` · `regresando`. |
| `persiguiendo` | `{tipo: 'ejercito'|'caravana', id, destino?}`: objetivo móvil (§7). |
| `formacion` | `{expiraEn}` si es una formación en espera de ser tres (§10). |
| `origenAsentamientoId` / `origenCampamentoId` | Adónde vuelve (§11). Un ejército nacido en un campamento de mercenarios tiene `origenAsentamientoId: ''`. |

`Heroe.viveres` (número) es el trigo que **come** la columna. `Heroe.ubicacion` dice si el héroe está `asentamiento`, `columna`, `mercenarios`…

## 3. Salir

Todas son funciones puras `(plaza, campamento, jugador, heroeId, …) → columna nueva + plaza con el carro cargado`.

### 3.1 `movilizarEjercito` (salir CONTRA un destino)
1. Solo un **residente** de la plaza sale desde ella (ahí está su campamento y su tropa).
2. El destino no puede ser la propia plaza de origen.
3. Escuadras: deben existir en el campamento de la plaza, ser del héroe, tener `cantidad > 0` y **no estar en la guarnición**. Hace falta al menos una. Se rechaza en vez de filtrar en silencio.
4. **Liderazgo** (`exigirLiderazgo`): `puedeLlevar(héroe, [lo que ya tiene fuera del campamento] + [las nuevas])`, con `liderazgoDe(héroe) = héroe.liderazgoBase ?? LIDERAZGO.base` y coste por escuadra = `LIDERAZGO.costePorEscalon[escalón de su tropa]`, **sin mirar cuántos soldados quedan** (una escuadra mermada paga el precio completo). El tope es **por héroe**; en un ejército de varios no hay tope agregado.
5. Ruta: `calcularRuta(mapa, plaza, destino, { pasosRio: [posiciones de todos los asentamientos] })`. Si no hay camino por tierra → se rechaza.
6. Carro: `cargarCarroElegido` (§6.2), con capacidad de **un** carro (`capacidadCarroPorJugador`).
7. Nace `tipo: 'ejercito'`, `estado: marchando` (o `estacionado` si no se dio destino: sale quieto a la puerta y su Líder lo dirige con `marcharA`), un participante, `suministro` = lo cargado. **Los víveres no los llena esta función**: los llena quien la llama (`llenarViveres`, §6.1).

### 3.2 `salirAlMundo` (salir a lo tuyo)
Igual, con estas diferencias: se puede salir **sin tropas** (el viajero solo es una forma de jugar); no se puede salir si ya vas en una columna; nace `tipo: 'personal'`, `politicaDeUnion: 'rechazar'`, `estacionado` en la posición de la plaza y sin ruta.

### 3.3 `salirDelCampamento` (de un campamento de mercenarios)
- El **no residente** retoma la columna con la que entró (aparcada a la puerta); sin ella, error.
- El **residente** elige tropa de su campamento y carga del **almacén personal** (no del de una plaza): cada recurso pedido debe estar y caber en lo que queda del carro (que puede traer un resto de cuando entró).
- **Ración gratis** (`MERCENARIOS.racion.cadaMinutos`): si pasó ese plazo desde `racionEn` (o nunca la tuvo), sale con los víveres al tope (`capacidadViveresPorHeroe`) y `racionEn = ahora`. No se acumula.
- Con `politicaDeUnion ≠ 'rechazar'` sale como ejército (y con ruta si se le dio rumbo); si no, columna personal quieta.

## 4. Velocidad y avance

### 4.1 Velocidad de la columna (`velocidadDeEjercito`)
Es el **mínimo** de:
- la `velocidad` (catálogo `TROPAS_RECLUTABLES`) de cada escuadra con `cantidad > 0` (una aniquilada no frena a nadie);
- la `velocidadCaravana` de cada caravana adjunta.

Sin ninguna de las dos: `MOVIMIENTO.velocidadJugador` si va alguien dentro, 0 si no va nadie. De aquí salen solas las distinciones de rol: una escuadra pesada (12) frena a una fuerza ligera (20) y le quita la capacidad de cazar caravanas comerciales (16); engancharse una caravana baja la velocidad al mínimo de las dos, así que **no se puede escoltar y depredar a la vez**.

### 4.2 Un paso (`avanzarPosicionEnRuta`)
```
longitud    = max(1, longitud de la polilínea)
recorrido   = progreso × longitud
(punto, tramo) = ubicar(ruta, recorrido)
enCamino    = el tramo (par de vértices consecutivos) es una arista de la red de caminos
avance      = velocidad / ( costeEnPunto(punto) × (enCamino ? COSTE_MOVIMIENTO.factorCamino : 1) )
progreso'   = min(1, (recorrido + avance) / longitud)
posicion'   = punto de la polilínea a progreso' × longitud
```
- `costeEnPunto` (mapa) es el coste del terreno **en el punto donde está al empezar el tick**, no a lo largo del paso: es una aproximación deliberada (un paso largo cruza terreno que no ve).
- Llano = 1, y sube con colinas, montañas, bosque (disco, `1 + bosquePorDensidad × densidad`)… (`COSTE_MOVIMIENTO`, `worldgen/config.ts`). Sobre la red de caminos el coste se multiplica por `factorCamino` (0,5).
- Una ruta de un solo punto o vacía no se mueve.

### 4.3 La ruta (`calcularRuta`, A\* sobre el grafo de navegación)
- Origen y destino deben ser transitables (no agua) o devuelve `undefined`; ningún llamador sigue sin ruta (una recta de reserva cruzaría el agua).
- Los **ríos** se cruzan por un vado o por una ciudad ribereña: `pasosRio` = posiciones de asentamientos, que cuentan como paso a menos de `radioPasoCiudad`.
- Si origen y destino están a menos de un espaciado del grafo, un tramo directo si es válido.
- Si no, se «engancha» origen y destino a los nodos de su alrededor (4×4) alcanzables en recta, y se corre A\* con 8 vecinos y heurística admisible. Resultado: `[origen, …nodos, destino]`.
- `opciones.pesos` (atracción de la red de caminos) **no** lo usan los ejércitos hoy; sí lo usan las caravanas.

## 5. Destino, rumbo y estado

- **`marcharA`** fija o rectifica el destino: solo columna `personal` **o el Líder** de un ejército; no en una formación; y el héroe debe estar en `columna` (si está dentro de una plaza, hay que salir primero). Recalcula la ruta **desde `posicionActual`**, pone `estado: marchando`, `progreso: 0` y **borra `persiguiendo`** (no se puede marchar y perseguir a la vez).
- **`estacionarEjercito`**: `estacionado`, `objetivo: punto actual`. Error si ya lo estaba.
- **`replegarEjercito`** (volver a casa / cancelar): solo el Líder si lo pide un actor (el sistema no tiene actor); error si ya regresa o no hay casa. Si **marchaba**: invierte la ruta y `progreso = 1 − progreso` (la posición no cambia y desanda exactamente su camino). Si estaba **estacionado**: calcula ruta nueva hasta casa. `objetivo` = el asentamiento de origen (o el punto del campamento).
- **Llegar** (`progreso ≥ 1`) no es atacar ni entrar: el ejército **acampa** (`estacionado`) donde llega, con evento `ejercito.llega`. Quien persigue no acampa al llegar al último punto visto de su presa (§7).

## 6. Comer: víveres, carro y hambre

### 6.1 Los víveres (`engine/viveres.ts`)
- Tope por héroe: `LOGISTICA.capacidadViveresPorHeroe` (350, derivado del radio operativo: ida y vuelta a un cuarto del mapa = 50 min a velocidad 20). **En un ejército se suman.** Van siempre con el héroe, dentro o fuera.
- Se llenan **solo** (a) al salir, de la plaza (`llenarViveres`), (b) al repostar (§8), (c) con la ración del campamento de mercenarios (§3.3), (d) desde el carro con `pasarAViveres` (solo el Líder; mueve `min(pedido, trigo del carro, hueco)`; nunca al revés).
- `sacarTrigoDe(plaza, consumoTropas, espacio)`: suelta `min(espacio, trigo − reservaDeTrigo)`. La **reserva** es lo que comen la población y la guarnición durante `RESERVA_CONSTRUCCION.horizonteMinutosComida`; sacar una columna cuesta stock real pero **no puede dejar la ciudad en hambruna**: si no llega, se sale con menos.
- `descontarViveres` descuenta **a prorrata de lo que lleva cada héroe** (comer del montón común no vacía primero los de uno); por eso separarse se lleva su parte sin reparto.

### 6.2 Cargar el carro (`cargarCarroElegido`)
Tres topes, y falla en vez de recortar: (1) total ≤ capacidad (carro entero, no un cajón por recurso); (2) cada recurso ≤ disponible en el almacén (el trigo, además, menos la reserva); (3) nada negativo (sería un depósito encubierto).

### 6.3 La ración del tick (primer paso de cada ejército en `avanzarEjercitos`)
```
factor  = factorConsumoEnMarcha × (estado == 'estacionado' ? factorConsumoEstacionado : 1)
ración  = Σ(cantidad de soldados) × MILITAR.racionPorSoldadoPorMinuto × factor
        + participantes × MOVIMIENTO.consumoPorParticipante × factor
```
(`consumoRacionDeColumna`). Se consume `min(víveres, ración)` y se descuenta de los héroes a prorrata. Con `factorSuministro = min(1, víveres/ración)`:
- si `≥ 1`: cada escuadra gana `regeneracionMoralPorMinuto` de moral (tope 100);
- si `< 1`: pierde `degradacionMoralSinRacion × (1 − factorSuministro)`;
- con moral ≤ 0 y soldados: deserta `ceil(cantidad × desercionFraccionPorMinutoSinMoral)` (evento `tropas.desercion`, atribuido a la plaza de origen para que no se cuente a todo el mundo que un rival pierde hombres).

La guarnición usa **la misma `avanzarRacion`**, con factor 1 y 0 participantes; no hay dos reglas de hambre.

## 7. Perseguir

- **`perseguir`** fija `persiguiendo = {tipo, id}` y `estado: marchando`. Condiciones: no a sí mismo; la columna propia con algún héroe sano; presa de la **misma clase** (`personal` con `personal`, `ejército` con `ejército`); si la presa es una columna, con algún héroe sano. Antes, `exigirPresaPerseguible`: no a los tuyos ni a un aliado, no a una caravana adjunta, y **un ejército no persigue caravanas** (las persigue una columna personal).
- **Cada tick** (`seguirPresa`), con la posición de la presa **al empezar el tick** (para no depender del orden):
  1. Presa fuera del mapa (entró en plaza/campamento, se disolvió, adjunta o en batalla) → se suelta, `columna.presa_a_cubierto`.
  2. Presa a más de `alcanceDeVista` del perseguidor (`VISION.ejercito` si lleva soldados en pie, si no `VISION.jugadorSolo`) → se suelta, `columna.presa_perdida`. Así la trayectoria no delata dónde está bajo la niebla.
  3. Si ya hay destino calculado, la columna no ha llegado a él y la presa se ha movido ≤ `PERSECUCION.umbralRecalculo` desde él → no se recalcula nada.
  4. Si no: se recalcula la ruta hacia la presa; si no hay camino por tierra → se suelta, `presa_perdida`.
  Al soltar sin destino calculado la columna se queda quieta donde está; con destino, termina el rumbo y acampa.
- **Alcance** (`cerrarPersecuciones`, fase 2 del tick): si, con el perseguidor con héroe sano, la presa está a ≤ `LOGISTICA.radioEncuentro` (15), es enemiga (no misma Facción ni aliada), tiene héroe sano (columna) o **no** está en un refugio ni adjunta/disponible (caravana) → `persiguiendo` se borra y se emite `columna.presa_alcanzada` (a la plaza de origen de los dos). **No hay combate**: atacar es una orden aparte.

## 8. Repostar al pasar

Tras moverse y antes de resolver la llegada (`repostarSiPuede`):
1. Candidatas: plazas a ≤ `LOGISTICA.radioReabastecimiento` (60) donde `puedeRepostarEn`: la **propia** Facción siempre; una **aliada** solo si tiene `permiteReabastecerAliados`; neutral u hostil, nunca.
2. Se elige la **más cercana** (empate: id menor).
3. `sacarTrigoDe(plaza, ración de su campamento, hueco de víveres de la columna)`; el trigo se reparte llenando héroe a héroe (`repartirEnViveres`).
4. Sin límite de veces: acampar junto a una plaza amiga repone cada tick (eso hace sostenible «sostener un paso»). **Solo se narra** (`ejercito.reabastecido`) si la columna llevaba menos de `umbralNarrarReposte` (50 %) de víveres.

## 9. Caravanas adjuntas

- **`adjuntarCaravana`**: misma Facción que el ejército; no ya enganchada; estado `disponible` o `aparcada` (o `retornando` si es de Fundación); a ≤ `radioReabastecimiento` del ejército. La **Caravana de Fundación** la lleva su titular: otro solo la reclama si el titular ya no es vigente. No se comprueba el cupo de Mercado: engancharla ya la saca de comerciar.
- Una adjunta pasa a `estado: 'adjunta'` y su `posicionActual` sigue a la columna cada tick.
- **`soltarCaravana`**: queda `disponible` donde esté la columna; no vuelve sola.
- **`cargarCaravanaAdjunta`**: de una plaza a ≤ 60 donde pueda repostar; `min(pedido, espacio, disponible)`; **sin reserva** (igual que el comercio automático).
- **Capacidad de carga** del ejército (`capacidadCargaDe`) = `participantes × capacidadCarroPorJugador` + Σ `capacidadCaravana` de las adjuntas.
- Si el ejército se disuelve (§11) las adjuntas **se pierden con él** (`ejercito.caravanas_perdidas`); también si lo derrotan (Combate).

## 10. Juntar y separar

### 10.1 Unirse (`unirseAEjercito`, desde una plaza)
Residente de esa plaza; mismo bando; ejército a ≤ `radioReabastecimiento` de la plaza; la política `rechazar` cierra la puerta (salvo reforzar el ejército en el que ya vas); selecciona escuadras como §3.1; liderazgo por héroe. Sumar tropa a un ejército donde ya vas **no** te hace dos participantes.

### 10.2 Unirse en campo (`unirseEnCampo`, `validarUnionEnCampo`)
- El ejército no rechaza; es `ejercito` **o** una formación; la que se une es `personal` sin formación; ambas con Facción no vacía e igual; a ≤ `radioEncuentro` (15).
- Aporta lo que ya lleva: escuadras y carro. `unidoEn = ahora`. **Adopta el destino del ejército y ya no puede rectificar**: ese es el precio de marchar acompañado.
- Con `preguntar`: `anotarPeticionDeUnion` valida antes de anotar (no hacer perder al Líder los 10 s), sustituye la petición previa de ese jugador y barre las vencidas. `retirarPeticionDeUnion`: solo el Líder, y la petición debe estar viva (el silencio vale como «no»).

### 10.3 Separarse (`separarseDelEjercito` → `desgajar`)
- Debe ir dentro; de una columna personal no te separas; **el Líder no puede** (cede antes) salvo en una formación, donde el mando pasa al que lleva más tiempo (`sucesorEnFormacion`); **el último tampoco** (cancela la marcha).
- `desgajar(ejercito, heroeId, id)`: nace una columna personal `estacionado` donde está, con sus escuadras, **heredando `origenAsentamientoId`** (separarse no inventa casa), y con **un carro como mucho**: `min(capacidad de 1 carro, total / nº de participantes)`, repartido a prorrata por recurso. (Sin ese reparto el primero en irse se llevaba un carro entero y el Líder se quedaba sin trigo.)
- `cederLiderazgo`: solo el Líder, a otro participante.

### 10.4 Formar un ejército en campo (`formacion.ts`)
- `organizarEjercito`: una columna personal, sin formación, con **un** participante. Queda quieta (`estacionado`, objetivo = su punto, sin presa) con `formacion.expiraEn = ahora + FORMACION_EJERCITO.plazoMinutos`.
- `completarFormacion`: con ≥ `FORMACION_EJERCITO.minimo` (3) participantes pasa a `tipo: 'ejercito'` y pierde la marca; queda quieto hasta que su Líder lo dirija.
- Cada tick `disolverFormacionesVencidas`: las que pasaron `expiraEn` sin llegar a 3 se deshacen (`columna.formacion_disuelta`).
- `disolverFormacion` (también usada para **desarmar** un ejército que va a entrar en un lugar): el Líder se queda con la columna (queda `personal`, política `rechazar`, sin peticiones **ni caravanas**) y cada otro nace como personal donde estaba, vía `desgajar`.

## 11. Terminar la campaña

- **Disolución por quedarse sin nadie dentro** (`participantes.length == 0`, paso 2 del tick; antes de moverlo): la tropa vuelve a la plaza de origen y el carro entra en su almacén (`absorberColumna`), las identidades vuelven a los campamentos de sus héroes, las adjuntas se pierden, evento `ejercito.disuelto`. **Perder todos los soldados no basta**: los jugadores siguen dentro y viajan a pie.
- **Volver a casa** (`regresando` y `progreso ≥ 1`, `reintegrar`):
  - Si el origen existe: quien **no reside** allí se desgaja en su propia columna, `estacionado`, a la puerta, con lo suyo; el resto entra: tropa a los campamentos, carro al almacén (`absorberColumna(…, true)`), héroes situados en la plaza. Evento `ejercito.regresa`.
  - Si el origen ya no existe (cayó): la tropa vuelve a los campamentos de sus héroes; `reintegrado: false`.
  - Un ejército de **campamento de mercenarios** no se disuelve en ninguna guarnición: se para a su puerta (si queda uno solo, pasa a personal).
- **Entrar en una plaza propia** (`exigirPuedeGuarnecer`): solo un ejército, de la Facción de la plaza, a ≤ `MOVIMIENTO.radioPuerta` (10). Se desarma con `disolverFormacion` y cada héroe entra como lo haría solo. **Entrar en un campamento** (`entrarEnCampamento`): se entra **solo** (un ejército con más gente se desarma antes), a ≤ 10 de su puerta; el residente deja tropa y vuelca el carro en su almacén personal hasta `ALMACEN_PERSONAL.capacidad` (lo que no cabe se queda en el carro, la columna sigue aparcada); un visitante entra con la columna aparcada intacta.
- **Caravanas al entrar** (`caravanasAlEntrar`): solo entra si **todas** las adjuntas son de ese lugar (si no, hay que soltarlas o llevarlas a su origen). En una plaza pasan a `aparcada`; en un campamento (Caravana de Fundación) quedan `disponible` y empiezan a caducar.

## 12. Inspeccionar

A ≤ `MOVIMIENTO.radioInspeccion` (40): se ve la composición de una columna ajena (héroes y `{tropaId, cantidad, heroeId}` de cada escuadra con soldados), la defensa de una plaza ajena (guarnición de sus residentes y héroes dentro; **no** el almacén) o de una caravana (si va escoltada y qué recursos lleva, **no cuánto**). El observado recibe aviso: obtener información cuesta ser visto.

## 13. El tick de ejércitos (`avanzarEjercitos`)

No consume aleatoriedad. Orden canónico por id. Para cada ejército:
1. **Comer** (§6.3).
2. **Disolver** si no va nadie dentro (§11).
2b. **Seguir a la presa** (§7).
3. **Mover** (salvo `estacionado`).
3b. **Repostar** (§8) y mover las adjuntas con la columna.
4. **Llegar** (§5, §11).

Fase 2 (después de todos): `cerrarPersecuciones` (§7). Cada escuadra vuelve a su héroe con su estado final.

## 14. Parámetros

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
  "radioReabastecimiento": 60,
  "umbralNarrarReposte": 0.5,
  "radioEncuentro": 15
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
```
<!-- /parametros -->

## 15. Invariantes

- La posición de una columna es función de `(ruta, progreso)`; salvo `desgajar` y las entradas en lugares, nada la mueve fuera de §4.2.
- Cada héroe va como mucho en una columna (`columnaDe`).
- Un ejército en marcha no cambia de rumbo: solo su Líder lo hace con `marcharA`, o se cancela con `replegarEjercito`.
- Una caravana adjunta está en `caravanasAdjuntasIds` de **exactamente** un ejército y su posición es la de él.
- Ni el hambre ni el reposte pueden dejar a una plaza por debajo de su `reservaDeTrigo`.

## 16. Casos límite y decisiones que parecen raras

- **`costeEnPunto` se lee al empezar el tick**: un paso largo no «ve» la montaña que está a medio paso. Es barato y suficiente con ticks de un minuto.
- **El tipo no se deriva del número de miembros** (ver §2): derivarlo reabriría el agujero de rectificar rumbo en un ejército reducido a uno.
- **Cancelar la marcha es del Líder, no atrapa a nadie**: el que no quiera seguir se separa.
- **Una escuadra mermada paga su Liderazgo completo**: si se midiera por soldados vivos, la tropa cara se podría sacar «en dosis pequeñas». Punto abierto conocido (`Consideraciones/Movimiento_Ejercitos_Definicion.md` §7).
- **Disolverse por hambre pierde las caravanas adjuntas** aunque el canon solo lo diga del ejército derrotado: en los dos casos deja de existir en campo abierto.
- **Los eventos de deserción llevan `asentamientoId`**: sin él saldrían globales.

## 17. Divergencias

Ninguna abierta a 2026-10-10.
