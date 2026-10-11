# Tecnología por Eras y Aedas

Cómo llega una tecnología a una Facción (logro del servidor + hito propio, compra a un Aeda o épica de un residente), cómo avanzan las Eras, cómo se adopta y qué hacen los Aedas: itinerantes que revelan y venden,
residentes que cantan épicas, y la crónica que narra los hechos públicos.

- **Verificado contra:** `4d65075` (2026-10-11).
- **Módulos:** `src/engine/tecnologia.ts` (logros, Eras, aparición, adopción, revelar, vender), `src/engine/aedas.ts` (itinerantes), `src/engine/aedasResidentes.ts` (residentes y épicas), `src/engine/cronica.ts` (crónica),
  `src/engine/reclutamientoMercenario.ts` (`tecnologiasDelCampamento`), `src/session/comandos/tecnologia.ts` (comandos), `src/session/resultadoBatalla.ts` (`contadoresDeBatalla`), `src/session/comandos/tipos.ts` (`exito`),
  `src/session/proyecciones/jugador.ts` (`tecnologiaParaJugador`).
- **Canon:** Doc 6 (6.1–6.7), Doc 2.9 (títulos), Doc 5.8 (qué tropa pide qué tecnología). Los números de **calibración** (las X de los logros, las tarifas, los cupos) son placeholder; el efecto de gatear con tecnología en construcción, mejoras, recetas y reclutamiento está en
  [`Economia_Construccion.md`](Economia_Construccion.md).
- **Fuera de alcance aquí:** el reclutamiento en sí (Doc 5.8); la proyección completa del jugador ([`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) para lo que se ve de un Aeda).

## 1. Modelo de datos

```
EstadoTecnologia (del servidor) = {
  era:        EraId                                  la vigente
  eraDesde:   Instante                               cuándo empezó (su plazo cuenta desde aquí)
  contadores: Partial<Record<ContadorLogro, number>> los hechos del mundo, acumulados
  logros:     Partial<Record<TecnologiaId, Instante>>  cuándo se cumplió el logro de cada tecnología; FIJO para siempre
  primeros:   Partial<Record<TecnologiaId, { faccionId, en }>>   la primera Facción que la desbloqueó POR HITO (descubridora)
  cantadas?:  TecnologiaId[]                          descubrimientos que los Aedas ya han cantado
  porFaccion: Record<faccionId, TecnologiasFaccion>   una Facción sin entrada solo tiene las de arranque
}
TecnologiasFaccion = { aparecidas[], adoptadas[] (⊆ aparecidas), reveladas? }      reveladas = un Aeda se las mostró y aún no le han aparecido

DefinicionTecnologia = { nombre, era, deArranque?, logro?: { contador, umbral }, hito: CondicionHito[], bonusProduccion?: { recurso, factor } }
CondicionHito = edificio{edificio, nivelInterno?} | tecnologia{id} | recursoEnCapital{recurso} | capitalEnNivel{nivel, conEdificio} | yacimientoEnTerritorio{recurso}

AedaItinerante = { id, posicion, enAsentamientoId?, hasta?, destinoId?, ruta?, progreso, recientes[] }
AedaResidente  = { id, nombre, asentamientoId, faccionId (la dueña cuando llegó), llegadaEn, epica?: { tecnologiaId, capitulo, hechos, ultimoHechoEn?, claves[] } }
EstadoAedasResidentes = { aedas[], siguiente (nº del próximo, solo crece), esperaDesde: Record<asentamientoId, Instante>, cumplidas: Record<faccionId, nº> }
```

Una tecnología pasa, para cada Facción, por **oculta → revelada → aparecida → adoptada**. Hay **dos vías** para que aparezca: el **desarrollo propio** (logro del servidor + hito, §3) y los **Aedas** (compra, §6.4, o épica, §7.2); el comercio entre Facciones y la conquista se descartaron (2026-10-11). La **tecnología de arranque** (las de `deArranque`: `leva_comunal` y `hostigamiento_tribal`) la tiene toda Facción aparecida y adoptada desde que nace, sin logro ni hito.
`tecnologiasDe(estado, faccionId)` devuelve las de arranque a una Facción sin entrada.

## 2. Logros del servidor

Un **contador** es un hecho del mundo (entre todas las Facciones, bots incluidos); el **logro** de una tecnología se cumple cuando su contador llega a su `umbral`, y **queda fijo** (se guarda el instante).
Cada logro cumplido emite el evento **público** `tecnologia.logro { contador, umbral }`: dice qué pasó, **nunca qué tecnología abre**.

### 2.1 De dónde sale cada contador

Todos los contadores se **suman** salvo `plazasEnNivel.N`, que es un **récord** (el máximo, no una suma). Se suman por tres vías: el tick (producción), los eventos de **cualquier** comando o tick (`exito`) y los comandos de reclutamiento.

| Contador | Se suma… |
|---|---|
| `extraido.cobre / hierro / piedra / estano` | lo extraído de yacimientos cada tick (`avanzarConstruccion`, cantidad real extraída) |
| `fabricado.equipoBronce` | cada tick: lo fabricado de `armaBronce` + `armaBronceCalidad` + `armaduraBronce` + `armaduraBronceCalidad` |
| `fabricado.armaduraBronce` | cada tick: solo `armaduraBronce` |
| `batallas.libradas` | 1 por evento `combate.resuelto`; 1 por batalla de Unity aplicada |
| `batallas.conHoplitas` | 1 si el `combate.resuelto` lleva la tropa `hoplitas_ciudadanos` (`payload.tropaIds`); en Unity, si alguna escuadra de la batalla es de esa tropa |
| `batallas.campoAbierto` | 1 por `combate.encuentro`; 1 por batalla de Unity de contexto `campo_abierto` |
| `asedios.resistidosEnCombate` | 1 por `combate.asedio_resistido` con `enCombate` (no un rebote por protección ni una plaza sin defensores); en Unity, ganador defensor de un asedio |
| `asedios.resistidosConResidentes` | lo anterior si además `conResidentes`; en Unity, si hubo algún héroe defensor |
| `asedios.contraMurallaCompleta` | 1 por asedio `enCombate` contra una plaza con muralla completa (integridad ≥ 1) |
| `conquistas.conMurallaCompleta` | 1 por `combate.asedio_conquista` contra una plaza con muralla completa; en Unity, ganador atacante con muralla completa |
| `caravanas.destruidasOCapturadas` | 1 por `combate.caravana_interceptada_por_ejercito` con `capturada`; 1 por `bandidos.caravana_interceptada`; en Unity, ganador atacante de un asalto de caravana |
| `bandidos.campamentosDestruidos` | 1 por `combate.campamento_destruido`; en Unity, ganador atacante contra un campamento |
| `animales.comprados` | 1 por `comercio.caravana_animal_comprado` |
| `reclutados.escuadrones` | 1 por cada escuadra **nueva** al reclutar (`antes == 0`) |
| `reclutados.caballeria` | soldados reclutados de una tropa cuyo edificio es Caballerizas, **× 5** si es `carros_guerra`, × 1 el resto |
| `reclutados.arqueros`, `.escaramuzadores_jabalina`, `.arqueros_compuesto`, `.jinetes_asirios` | los soldados reclutados de esa tropa (`reclutados.<tropaId>` si el contador existe) |
| `plazasEnNivel.2 / .3` | **récord** de cuántas plazas del mundo han tenido a la vez `nivelActual ≥ N` (se actualiza cada tick) |

## 3. El tick de tecnología (`avanzarTecnologia`)

Corre al final de la cadena del tick ([`Tick.md`](Tick.md) §5.3, paso 19), después de sumar los contadores de producción. Determinista y **sin generador aleatorio**.

```
0. récords:  para N en {2, 3}:  ahora = nº de plazas con nivelActual ≥ N;  si ahora > contadores['plazasEnNivel.N'] → se guarda
1. logros:   para cada tecnología con logro, sin logro cumplido, con contador ≥ umbral:  logros[id] = instante;  evento público tecnologia.logro
2. Era:      siguiente = la Era que sigue a la vigente (ninguna en la última)
             todos   = TODAS las tecnologías de la Era vigente que tienen logro lo tienen cumplido
             porPlazo= instante − eraDesde ≥ plazoSemanas × 7 días
             si todos ∨ porPlazo:  era = siguiente, eraDesde = instante;  evento público era.comienza { era, porPlazo: !todos }       ← solo UNA Era por tick
3. aparición: para cada Facción con ≥ 1 asentamiento, para cada tecnología en el orden del catálogo:
             saltar si es de arranque, ya aparecida, sin logro cumplido, o de una Era posterior a la vigente (con la Era ya actualizada en 2)
             si TODAS las condiciones del hito se cumplen → aparece: se añade a aparecidas (y se quita de reveladas);
                 si primeros[id] no existe → primeros[id] = { faccionId, en: instante } (la descubridora);
                 evento tecnologia.aparece { faccionId, tecnologiaId, primera } atribuido a la capital (privado de la Facción)
4. crónica:  para cada tecnología aún no cantada que los Aedas YA conocen (§6.1): se canta;  evento público aedas.canta_descubrimiento { tecnologiaId, faccionId: la descubridora }
```

Las condiciones del hito se miran con el estado de la Facción **al empezar la evaluación de cada tecnología** (una condición `tecnologia` cuenta las **adoptadas**, no las aparecidas).

### 3.1 Condiciones de hito (`cumpleCondicion`)

| Condición | Se cumple si… |
|---|---|
| `edificio {edificio, nivelInterno?}` | algún asentamiento de la Facción tiene ese edificio `activo` con `nivelInterno ≥ nivelInterno` (por defecto 1) |
| `tecnologia {id}` | la tecnología está **adoptada** por la Facción |
| `recursoEnCapital {recurso}` | el almacén de su capital (`encontrarCapital`) tiene `> 0` de ese recurso |
| `capitalEnNivel {nivel, conEdificio}` | existe capital con `nivelActual ≥ nivel` **y** ese edificio activo en la propia capital |
| `yacimientoEnTerritorio {recurso}` | alguna zona de influencia de un asentamiento de la Facción contiene algún nodo de ese recurso (con o sin stock) |

### 3.2 Eras

El servidor arranca en la Era I (`reinos_palaciales`). Cada Era tiene un plazo (`ERAS[..].plazoSemanas`) que es un **techo**: la siguiente empieza antes si se cumplen todos los logros de la vigente. Si empieza por plazo, los logros pendientes de la anterior **siguen abiertos**.
**Una tecnología de una Era no puede aparecer, venderse ni empezar su épica antes de que esa Era empiece.**

## 4. Adopción (`adoptarTecnologia`)

La pide el **Rey**, estando en la **capital**, y la paga el almacén de la capital; es instantánea.

```
exige: la tecnología existe;  la Facción tiene Rey;  tiene capital (encontrarCapital);  el Rey está en la capital (estaEnAsentamiento);
       no está ya adoptada;  está APARECIDA;  la capital tiene la tarifa de la Era de la tecnología (TARIFA_ADOPCION[era])
efecto: adoptadas += id;  capital.almacen −= tarifa;  evento tecnologia.adoptada { faccionId, tecnologiaId, capitalId } atribuido a la capital
```

Vale en todos los asentamientos de la Facción. **No hay descuento** por llegar tarde ni por la vía (hito, Aeda, épica): todos pagan la misma tarifa.

### 4.1 Qué gatea una tecnología

Una tecnología adoptada habilita —las «cuatro puertas» del Doc 6.1— construir un edificio (`EDIFICIO_CATALOGO[..].requiereTecnologia`), mejorar un nivel interno (`niveles[n].requiereTecnologia`), producir una receta (`receta.requiereTecnologia`) o reclutar una tropa (`TROPAS_RECLUTABLES[..].tecnologia`);
y algunas dan un `bonusProduccion` (hoy solo `canteria`: ×1,5 la piedra que sacan las Canteras de la Facción). Cada sistema comprueba contra `tecnologiasDe(estado, faccionId).adoptadas`.

## 5. Campamentos de mercenarios (`tecnologiasDelCampamento`)

Un campamento no adopta nada: **desbloquea** cada tecnología —al instante y gratis— cuando se cumple lo que llegue antes de:

```
(instante − primeros[id].en) ≥ horasTrasLaPrimera × 60 min      ← primeros[id] debe existir: una tecnología que nadie ha desbloqueado por hito no está en el campamento
(Facciones con ≥ 1 asentamiento que la tienen ADOPTADA) / (Facciones con ≥ 1 asentamiento) ≥ porcentajeFacciones     (0 Facciones → no)
                                                                                                  bots y humanos cuentan igual: juegan por la misma vía
```

Las de arranque están siempre. «Vivas» son todas las Facciones que conservan algún asentamiento (no se distingue a los bots).

## 6. Aedas itinerantes

### 6.1 Qué saben

Un Aeda **conoce** una tecnología (`conocidaPorAedas`) si: su logro está cumplido ∧ existe `primeros[id]` ∧ `primeros[id].en + retrasoConocimientoMinutos ≤ instante` ∧ su Era ≤ la vigente. Es la ventaja del primero: 24 horas desde que alguien la desbloquea por hito.

### 6.2 Cuántos, y su vida

```
objetivo = (hay asentamientos) ? max(itinerantes.minimo, ceil(Facciones con algún asentamiento / faccionesPorAeda)) : 0
nacen los que faltan hasta el objetivo (nunca se eliminan si baja):  id 'aeda-<i+1>', repartidos en las plazas que los aceptan ordenadas por id, la i-ésima en la plaza (i mod n), ya "llegada" (con su estancia)
una plaza los acepta salvo que su Facción les haya cerrado la puerta con el grupo 'aedas' (puertaCerradaA; el cierre por defecto no los incluye)
```

Cada tick, para cada Aeda **en el orden del array** (y luego los recién nacidos):

```
si tiene ruta y destino:
    si el destino ya no existe → elige otro desde donde esté (partir)
    avanza `velocidad` por la ruta (sobre camino va más rápido, como una columna); si progreso ≥ 1 → llegado
si no, y está detenido (hasta > instante) y (su plaza existe ∨ no tiene plaza) → espera
si no → partir
llegado(plaza): posicion = plaza.posicion;  hasta = instante + estanciaMinutos;  recientes = (recientes sin esa plaza) + plaza, últimas memoriaVisitas
partir:  candidatos = plazas distintas de la actual que los aceptan;  frescos = los no incluidos en recientes (si no hay ninguno, todos los candidatos);
         ordenados por distancia a su posición y, si empata, por id;  sale hacia el primero al que `calcularRuta` encuentre camino (pesos de la red de caminos, pasos de río en las posiciones de las plazas);
         si ninguno tiene camino → se queda otra estancia (hasta = instante + estanciaMinutos)
```

No son atacables ni capturables y se ven como una columna ajena solo dentro del radio de visión ([`Niebla_De_Guerra.md`](Niebla_De_Guerra.md)).

### 6.3 Revelar (`revelarTecnologias`)

Cada tick, por cada Aeda **detenido en una plaza** cuya Facción existe, en el orden del array: para cada tecnología que el Aeda **conoce** (§6.1) y la Facción no tiene **aparecida ni revelada**, se añade a `reveladas` y se emite `aedas.revela { faccionId, tecnologiaId, descubridorFaccionId, faltan[] }`
(atribuido a la plaza; privado). `faltan` es el texto de las condiciones del hito que la Facción aún no cumple. Revelan a **la Facción dueña de la plaza**, sea cual sea (también a una enemiga). Una tecnología revelada pasa a aparecida —y sale de `reveladas`— cuando se cumple el hito.

### 6.4 Vender (`venderTecnologia`, comando `comprarTecnologiaAeda`)

Hay un Aeda detenido en la plaza (`aedaEn`). Validaciones, en orden: la tecnología existe; su Era es vendible (`orden ≤ AEDAS.venta.ordenEraMaximo`); el Aeda la **conoce**; no está ya aparecida;
el almacén de **la plaza** tiene el oro: `precio = TARIFA_ADOPCION[era].oro × AEDAS.venta.factorOro`.
Efecto: la plaza paga; la tecnología pasa a **aparecida** (no a adoptada: adoptarla se paga igual, §4); evento `aedas.venta` (privado). **Se salta el hito, nunca el logro del servidor.**

## 7. Aedas residentes

Viven en un asentamiento con **Palacio activo** y **Nobleza > 0**. Dan felicidad y crecimiento de nobleza ([`Economia_Construccion.md`](Economia_Construccion.md) §9.1) y cuentan para el título de prestigio «Mecenas de los Aedas» (Doc 2.9: más épicas cumplidas y, a igualdad, más residentes).

### 7.1 Ciclo de vida (`avanzarResidentes`, en el tick, tras tecnología y Aedas itinerantes)

```
cupoDeResidentes(plaza) = (sin Palacio activo ∨ nobleza ≤ 0) ? 0 : AEDAS.residentes.cupoPorNivel[nivelActual] ?? 0
1. se quitan los Aedas de plazas que ya no existen
2. un Aeda cuya plaza cambió de dueño, o cuya épica es de una tecnología que ya apareció a la Facción por otra vía, PIERDE la épica; si cambió de dueño, su faccionId pasa a ser el del nuevo dueño
3. para cada plaza, en el orden del array:
     propios = sus residentes ordenados por llegada (y por id);  si hay más que el cupo, se van los ÚLTIMOS en llegar (evento aedas.residente_se_va)
     si propios ≥ cupo → se olvida su espera;  si no:
        esperaDesde[plaza] ??= instante
        si instante ≥ esperaDesde + llegadaMinutos × (reputación de la Facción ≤ umbralReputacionBaja ? factorLlegadaReputacionBaja : 1):
             llega uno ('aeda-r<siguiente>', nombre NOMBRES[(siguiente−1) mod 24]), siguiente++, se olvida la espera;  evento aedas.residente_llega
```

A lo sumo **llega uno por plaza y tick**; si sigue habiendo cupo, la espera vuelve a empezar al tick siguiente.

### 7.2 Épicas

`empezarEpica(plaza, aedaId, tecnologiaId)` (Rey, Gobernador o Sacerdote de la plaza, presentes) exige, en orden: el Aeda vive en esa plaza; la tecnología tiene épica (`EPICAS`); el Aeda es de la Facción dueña actual; el Aeda no canta ya otra épica; el **logro** del servidor de esa tecnología está cumplido;
su Era ha empezado; **no ha aparecido** a la Facción; y ningún otro Aeda de la Facción canta ya su épica. Crea `epica = { tecnologiaId, capitulo: 0, hechos: 0, claves: [] }` (evento `aedas.epica_empieza`). No cuesta oro. Una épica puede empezar aunque la tecnología solo esté revelada, o ni eso.
`abandonarEpica` borra la épica en curso y todo lo avanzado (`aedas.epica_abandonada`).

**Hechos inspiradores** (`hechosDeEventos`), sacados de los eventos de cada tick o comando (se evalúan en `exito`, [`Tick.md`](Tick.md) §7):

| Hecho | Evento | De quién | Clave irrepetible |
|---|---|---|---|
| `defensa` | `combate.asedio_resistido` con `enCombate`; o `batalla.aplicada` asedio con ganador defensor | su **plaza** (la defendida) | `defensa:<plaza>:<momento>` / `defensa:<battleId>` |
| `conquista` | `combate.asedio_conquista` con `enCombate`; o `batalla.aplicada` asedio con ganador atacante | su **Facción** (la atacante) | `conquista:<plaza>:<momento>` / `conquista:<battleId>` |
| `batalla` | `combate.encuentro` entre Facciones **distintas**; o `batalla.aplicada` de campo abierto entre Facciones distintas | su Facción (la ganadora) | `batalla:<A>:<B>:<momento>` / `batalla:<battleId>` |
| `obra` | `construccion.edificio_completado` o `construccion.mejora_completada` | su **plaza** | `obra:<edificioId>:<nivelNuevo ?? 0>` |
| `ascenso` | `asentamiento.nivel_subio` | su **plaza** | `ascenso:<plaza>:<nivel>` |
| `adopcion` | `tecnologia.adoptada` | su Facción | `adopcion:<facción>:<tecnología>` |
| `caravana` | `comercio.caravana_llega` de una caravana **comercial** con carga total ≥ `AEDAS.epica.cargaMinimaCaravana` | su **plaza** (la de origen) | `caravana:<caravanaId>` |

`avanzarEpicas(estado, eventos, ctx)`, para cada Aeda con épica cuya plaza sigue siendo de su Facción:

```
si ultimoHechoEn y instante − ultimoHechoEn < enfriamientoMinutos → nada
hecho = el PRIMERO de la lista (en el orden de los eventos) cuya clave no está en epica.claves y que inspira al capítulo actual:
        mismo tipo que capitulo.hecho  ∧  (el capítulo no pide edificio ∨ hecho.edificio == capitulo.edificio)  ∧
        ( hecho sin plaza → misma Facción  ∨  hecho con plaza → es la plaza del Aeda )
si no hay → nada.  A lo sumo UN hecho por Aeda y por llamada.
claves = (claves + clave del hecho), las últimas clavesRecordadas;  hechos += 1
si hechos < capitulo.cantidad → se guarda (ultimoHechoEn = instante)
si no, si quedan capítulos → capitulo += 1, hechos = 0, ultimoHechoEn = instante;  evento aedas.epica_capitulo
si no (último capítulo) → la épica termina: cumplidas[facción] += 1; la tecnología APARECE a la Facción (hacerAparecer, sin hito);  eventos aedas.epica_tecnologia (privado) y aedas.epica_cumplida (PÚBLICO, sin nombrar la tecnología)
```

`hacerAparecer` **no registra** `primeros[id]` (ver Divergencias). Cada tecnología con logro tiene su épica (`EPICAS`): 3 capítulos en la Era I, 4 en la II, 5 en la III.

## 8. Crónica (`entradasDeCronica`)

No es estado: es una **vista de los eventos**. Se calcula en `exito` sobre los hechos de cada tick o comando y se emite como eventos **públicos** (sin plaza atribuida). Entran tal cual los eventos públicos `tecnologia.logro`, `era.comienza`, `titulo.nace`, `titulo.cambia_manos`,
`aedas.canta_descubrimiento` y `aedas.epica_cumplida`. Se **derivan** entradas `cronica.entrada { tipo, faccionIds }` de:

| Evento | Entrada |
|---|---|
| `combate.asedio_conquista` | caída de la plaza: quién la arranca de quién (una de dos frases, según un hash del momento y el mensaje) |
| `asentamiento.ruinas` | lamento por las ruinas de la plaza |
| `fundacion.asentamiento_fundado` | fundación de la plaza por la Facción |
| `diplomacia.guerra_declarada`, `.rebelion_vasallo`, `.vasallo_liberado`, `.anexion`, `.fusion` | la frase de cada hecho |
| `asentamiento.nivel_subio` con `nivelNuevo ≥ 3` | solo si es **la primera plaza del mundo** en llegar a ese nivel (ninguna otra, fuera de las que suben en ese mismo lote, tenía `nivel ≥` el nuevo); si dos suben a la vez se canta una sola |

La variante de una frase es determinista: `hash(clave) mod nº de opciones`, con `h = (h·31 + código del carácter) mod 2³²`.

## 9. Comandos y permisos

| Comando | Quién | Qué |
|---|---|---|
| `adoptarTecnologia { faccionId, tecnologiaId }` | el **Rey** de la Facción | §4. `tecnologia.adoptada` |
| `comprarTecnologiaAeda { asentamientoId, tecnologiaId }` | el **Rey** o el **Gobernador** de la plaza, **presente** en ella | §6.4. Error si no hay Aeda detenido en la plaza |
| `empezarEpica { asentamientoId, aedaId, tecnologiaId }` | Rey, Gobernador o **Sacerdote** de la plaza, presente | §7.2 |
| `abandonarEpica { asentamientoId, aedaId }` | ídem | §7.2 |

## 10. Qué ve el jugador de su tecnología

`tecnologiaParaJugador`: la Era y desde cuándo; los **logros cumplidos** (contador, umbral, cuándo; públicos), ordenados por fecha; las tecnologías **propias** (aparecidas, adoptadas); las **reveladas** con su hito y qué condiciones cumple ya (`cumplidas[i]` ↔ `hito[i]`) y quién la desbloqueó;
la Era de cada tecnología propia (decide lo que cuesta adoptarla o comprarla); la capital; y las **épicas posibles**: las que tienen logro cumplido, Era abierta, que aún no han aparecido a la Facción y que ningún Aeda suyo canta ya. Los Aedas residentes se ven solo de la Facción propia.

## 11. Parámetros

<!-- parametros:ERAS -->
```json
{
  "reinos_palaciales": {
    "orden": 1,
    "nombre": "Reinos palaciales",
    "plazoSemanas": 5
  },
  "crisis_adaptacion": {
    "orden": 2,
    "nombre": "Crisis y adaptación",
    "plazoSemanas": 6
  },
  "polis_imperios": {
    "orden": 3,
    "nombre": "Polis e imperios",
    "plazoSemanas": 7
  }
}
```
<!-- /parametros -->

<!-- parametros:TARIFA_ADOPCION -->
```json
{
  "reinos_palaciales": {
    "oro": 100,
    "madera": 200
  },
  "crisis_adaptacion": {
    "oro": 300,
    "lingoteBronce": 30
  },
  "polis_imperios": {
    "oro": 600,
    "lingoteHierro": 30
  }
}
```
<!-- /parametros -->

<!-- parametros:AEDAS -->
```json
{
  "retrasoConocimientoMinutos": 1440,
  "itinerantes": {
    "faccionesPorAeda": 3,
    "minimo": 3,
    "estanciaMinutos": 360,
    "velocidad": 20,
    "memoriaVisitas": 4
  },
  "residentes": {
    "cupoPorNivel": {
      "2": 1,
      "3": 2,
      "4": 3,
      "5": 4
    },
    "llegadaMinutos": 1440,
    "umbralReputacionBaja": -50,
    "factorLlegadaReputacionBaja": 3,
    "bonoFelicidad": 0.05,
    "bonoNobleza": 0.25
  },
  "epica": {
    "enfriamientoMinutos": 360,
    "cargaMinimaCaravana": 100,
    "clavesRecordadas": 50
  },
  "venta": {
    "factorOro": 2,
    "ordenEraMaximo": 3
  }
}
```
<!-- /parametros -->

<!-- parametros:MERCENARIOS -->
```json
{
  "separacion": 900,
  "margenBorde": 150,
  "intentosColocacion": 2000,
  "salSemilla": 1835364963,
  "racion": {
    "cadaMinutos": 30
  },
  "radioExclusionFundar": 100,
  "radioProteccion": 60,
  "origenes": 3,
  "edificiosFijos": [
    "taberna",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "mercado"
  ],
  "edificiosMilitares": [
    "barracon",
    "galeriaDeTiro",
    "caballerizas"
  ],
  "layout": {
    "radioLocal": 60,
    "nivelMercado": 1
  },
  "margenBosque": 100,
  "pegadoAlBorde": 10,
  "poblacionPorVivienda": 10,
  "poblacionPorHora": 10,
  "recargo": 1.5,
  "descuentoSinAsentamientos": 0.6,
  "nivelEdificios": 3,
  "valorEquipoEnOro": 8,
  "tecnologia": {
    "porcentajeFacciones": 0.5,
    "horasTrasLaPrimera": 72
  },
  "mercado": {
    "cadaHoras": 3,
    "proporcionRepone": 0.03,
    "topePorBien": 300,
    "stockInicial": 100,
    "pilas": {
      "madera": 300,
      "piedra": 60,
      "trigo": 300
    },
    "cupoDiario": {
      "madera": 60,
      "piedra": 10
    },
    "margen": 1.3
  },
  "refundacion": {
    "porcentajeCoste": 0.4
  }
}
```
<!-- /parametros -->

<!-- parametros:TITULO_CAPITULO -->
```json
{
  "defensa": "Las murallas resisten",
  "batalla": "La batalla en campo abierto",
  "conquista": "La ciudad que cae",
  "obra": "La obra de las manos",
  "ascenso": "La ciudad crece",
  "adopcion": "El saber se hace ley",
  "caravana": "El camino y sus carretas"
}
```
<!-- /parametros -->

### 11.1 Catálogo de tecnologías

Cada entrada: `era`, `deArranque?`, `logro {contador, umbral}` (el umbral es la X del logro, **placeholder** hasta calibrarlo con batch en su semana objetivo), `hito[]` y `bonusProduccion?`.

<!-- parametros:TECNOLOGIAS -->
```json
{
  "leva_comunal": {
    "nombre": "Leva comunal",
    "era": "reinos_palaciales",
    "deArranque": true,
    "hito": []
  },
  "hostigamiento_tribal": {
    "nombre": "Hostigamiento tribal",
    "era": "reinos_palaciales",
    "deArranque": true,
    "hito": []
  },
  "metalurgia_cobre": {
    "nombre": "Metalurgia del cobre",
    "era": "reinos_palaciales",
    "logro": {
      "contador": "extraido.cobre",
      "umbral": 41000
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "fundicion"
      }
    ]
  },
  "aleacion_bronce": {
    "nombre": "Aleación del bronce",
    "era": "reinos_palaciales",
    "logro": {
      "contador": "extraido.estano",
      "umbral": 245000
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "fundicion",
        "nivelInterno": 2
      },
      {
        "tipo": "recursoEnCapital",
        "recurso": "estano"
      }
    ]
  },
  "escudos_ligeros": {
    "nombre": "Escudos ligeros",
    "era": "reinos_palaciales",
    "logro": {
      "contador": "bandidos.campamentosDestruidos",
      "umbral": 2250
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "barracon"
      }
    ]
  },
  "armamento_palacial": {
    "nombre": "Armamento palacial",
    "era": "reinos_palaciales",
    "logro": {
      "contador": "plazasEnNivel.2",
      "umbral": 12
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "armeria"
      },
      {
        "tipo": "tecnologia",
        "id": "metalurgia_cobre"
      }
    ]
  },
  "arqueria_palacial": {
    "nombre": "Arquería palacial",
    "era": "reinos_palaciales",
    "logro": {
      "contador": "batallas.libradas",
      "umbral": 100
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "galeriaDeTiro"
      }
    ]
  },
  "cria_caballar": {
    "nombre": "Cría caballar",
    "era": "reinos_palaciales",
    "logro": {
      "contador": "animales.comprados",
      "umbral": 700
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "corral"
      }
    ]
  },
  "carros_guerra": {
    "nombre": "Carros de guerra",
    "era": "reinos_palaciales",
    "logro": {
      "contador": "batallas.campoAbierto",
      "umbral": 69
    },
    "hito": [
      {
        "tipo": "tecnologia",
        "id": "cria_caballar"
      },
      {
        "tipo": "edificio",
        "edificio": "caballerizas",
        "nivelInterno": 2
      },
      {
        "tipo": "edificio",
        "edificio": "carpinteria",
        "nivelInterno": 2
      }
    ]
  },
  "canteria": {
    "nombre": "Cantería",
    "era": "reinos_palaciales",
    "logro": {
      "contador": "extraido.piedra",
      "umbral": 11000000
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "cantera"
      }
    ],
    "bonusProduccion": {
      "recurso": "piedra",
      "factor": 1.5
    }
  },
  "bronce_calidad_militar": {
    "nombre": "Bronce de calidad militar",
    "era": "crisis_adaptacion",
    "logro": {
      "contador": "fabricado.equipoBronce",
      "umbral": 5900
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "armeria",
        "nivelInterno": 3
      }
    ]
  },
  "forja_hierro_temprana": {
    "nombre": "Forja del hierro temprana",
    "era": "crisis_adaptacion",
    "logro": {
      "contador": "caravanas.destruidasOCapturadas",
      "umbral": 11000
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "fundicion",
        "nivelInterno": 2
      },
      {
        "tipo": "yacimientoEnTerritorio",
        "recurso": "hierro"
      }
    ]
  },
  "panoplia_bronce": {
    "nombre": "Panoplia de bronce",
    "era": "crisis_adaptacion",
    "logro": {
      "contador": "reclutados.escuadrones",
      "umbral": 1200
    },
    "hito": [
      {
        "tipo": "tecnologia",
        "id": "bronce_calidad_militar"
      },
      {
        "tipo": "edificio",
        "edificio": "barracon",
        "nivelInterno": 3
      }
    ]
  },
  "disciplina_formacion": {
    "nombre": "Disciplina de formación",
    "era": "crisis_adaptacion",
    "logro": {
      "contador": "asedios.resistidosEnCombate",
      "umbral": 140
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "barracon",
        "nivelInterno": 2
      }
    ]
  },
  "arco_compuesto": {
    "nombre": "Arco compuesto",
    "era": "crisis_adaptacion",
    "logro": {
      "contador": "reclutados.arqueros",
      "umbral": 350
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "carpinteria",
        "nivelInterno": 2
      },
      {
        "tipo": "edificio",
        "edificio": "galeriaDeTiro",
        "nivelInterno": 3
      }
    ]
  },
  "equitacion_militar": {
    "nombre": "Equitación militar",
    "era": "crisis_adaptacion",
    "logro": {
      "contador": "reclutados.caballeria",
      "umbral": 1800
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "caballerizas"
      }
    ]
  },
  "carpinteria_militar": {
    "nombre": "Carpintería militar",
    "era": "crisis_adaptacion",
    "logro": {
      "contador": "conquistas.conMurallaCompleta",
      "umbral": 1
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "carpinteria",
        "nivelInterno": 2
      }
    ]
  },
  "instituciones_civicas": {
    "nombre": "Instituciones cívicas",
    "era": "polis_imperios",
    "logro": {
      "contador": "plazasEnNivel.3",
      "umbral": 12
    },
    "hito": [
      {
        "tipo": "capitalEnNivel",
        "nivel": 3,
        "conEdificio": "mercado"
      }
    ]
  },
  "ciudadania_militar": {
    "nombre": "Ciudadanía militar",
    "era": "polis_imperios",
    "logro": {
      "contador": "asedios.resistidosConResidentes",
      "umbral": 158
    },
    "hito": [
      {
        "tipo": "tecnologia",
        "id": "instituciones_civicas"
      },
      {
        "tipo": "edificio",
        "edificio": "barracon",
        "nivelInterno": 2
      }
    ]
  },
  "falange_hoplita": {
    "nombre": "Falange hoplita",
    "era": "polis_imperios",
    "logro": {
      "contador": "batallas.conHoplitas",
      "umbral": 10
    },
    "hito": [
      {
        "tipo": "tecnologia",
        "id": "ciudadania_militar"
      },
      {
        "tipo": "edificio",
        "edificio": "barracon",
        "nivelInterno": 3
      }
    ]
  },
  "pantalla_escaramuzadores": {
    "nombre": "Pantalla de escaramuzadores",
    "era": "polis_imperios",
    "logro": {
      "contador": "reclutados.escaramuzadores_jabalina",
      "umbral": 1240
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "galeriaDeTiro",
        "nivelInterno": 2
      },
      {
        "tipo": "edificio",
        "edificio": "armeria",
        "nivelInterno": 2
      }
    ]
  },
  "arqueria_especializada": {
    "nombre": "Arquería especializada",
    "era": "polis_imperios",
    "logro": {
      "contador": "reclutados.arqueros_compuesto",
      "umbral": 120
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "galeriaDeTiro",
        "nivelInterno": 3
      }
    ]
  },
  "forja_hierro_estandarizada": {
    "nombre": "Forja del hierro estandarizada",
    "era": "polis_imperios",
    "logro": {
      "contador": "extraido.hierro",
      "umbral": 1380000
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "fundicion",
        "nivelInterno": 2
      },
      {
        "tipo": "edificio",
        "edificio": "minaHierro"
      }
    ]
  },
  "bronce_laminado": {
    "nombre": "Bronce laminado",
    "era": "polis_imperios",
    "logro": {
      "contador": "fabricado.armaduraBronce",
      "umbral": 200
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "armeria",
        "nivelInterno": 3
      }
    ]
  },
  "caballeria_organizada": {
    "nombre": "Caballería organizada",
    "era": "polis_imperios",
    "logro": {
      "contador": "reclutados.jinetes_asirios",
      "umbral": 1700
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "caballerizas",
        "nivelInterno": 2
      }
    ]
  },
  "trabajos_asedio": {
    "nombre": "Trabajos de asedio",
    "era": "polis_imperios",
    "logro": {
      "contador": "asedios.contraMurallaCompleta",
      "umbral": 10
    },
    "hito": [
      {
        "tipo": "edificio",
        "edificio": "carpinteria",
        "nivelInterno": 2
      }
    ]
  }
}
```
<!-- /parametros -->

### 11.2 Épicas

`cap(hecho, cantidad, edificio?)` en el código: cada capítulo pide `cantidad` hechos de un tipo (y, para `obra`, de un edificio).

<!-- parametros:EPICAS -->
```json
{
  "metalurgia_cobre": {
    "nombre": "Canto del fuego rojo",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "fundicion"
      },
      {
        "hecho": "caravana",
        "cantidad": 2
      },
      {
        "hecho": "ascenso",
        "cantidad": 1
      }
    ]
  },
  "aleacion_bronce": {
    "nombre": "Canto de la aleación",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "fundicion"
      },
      {
        "hecho": "caravana",
        "cantidad": 2
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      }
    ]
  },
  "escudos_ligeros": {
    "nombre": "Canto del mimbre trenzado",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "barracon"
      },
      {
        "hecho": "defensa",
        "cantidad": 1
      },
      {
        "hecho": "batalla",
        "cantidad": 1
      }
    ]
  },
  "armamento_palacial": {
    "nombre": "Canto de los almacenes del palacio",
    "capitulos": [
      {
        "hecho": "ascenso",
        "cantidad": 1
      },
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "armeria"
      },
      {
        "hecho": "defensa",
        "cantidad": 1
      }
    ]
  },
  "arqueria_palacial": {
    "nombre": "Canto de la cuerda tensa",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "galeriaDeTiro"
      },
      {
        "hecho": "defensa",
        "cantidad": 1
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      }
    ]
  },
  "cria_caballar": {
    "nombre": "Canto de las yeguas",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "corral"
      },
      {
        "hecho": "caravana",
        "cantidad": 2
      },
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "caballerizas"
      }
    ]
  },
  "carros_guerra": {
    "nombre": "Canto de los carros de Qadesh",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "caballerizas"
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "conquista",
        "cantidad": 1
      }
    ]
  },
  "canteria": {
    "nombre": "Canto de la piedra viva",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "cantera"
      },
      {
        "hecho": "ascenso",
        "cantidad": 1
      },
      {
        "hecho": "adopcion",
        "cantidad": 1
      }
    ]
  },
  "bronce_calidad_militar": {
    "nombre": "Canto del bronce de Dendra",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "armeria"
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "defensa",
        "cantidad": 1
      },
      {
        "hecho": "adopcion",
        "cantidad": 1
      }
    ]
  },
  "forja_hierro_temprana": {
    "nombre": "Canto del hierro que llega",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "minaHierro"
      },
      {
        "hecho": "caravana",
        "cantidad": 2
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "ascenso",
        "cantidad": 1
      }
    ]
  },
  "panoplia_bronce": {
    "nombre": "Canto del vaso de los guerreros",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "barracon"
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "defensa",
        "cantidad": 1
      },
      {
        "hecho": "conquista",
        "cantidad": 1
      }
    ]
  },
  "disciplina_formacion": {
    "nombre": "Canto de la batalla del Delta",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "barracon"
      },
      {
        "hecho": "defensa",
        "cantidad": 2
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "adopcion",
        "cantidad": 1
      }
    ]
  },
  "arco_compuesto": {
    "nombre": "Canto del arco de cuerno",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "carpinteria"
      },
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "galeriaDeTiro"
      },
      {
        "hecho": "defensa",
        "cantidad": 1
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      }
    ]
  },
  "equitacion_militar": {
    "nombre": "Canto de los jinetes de Assur",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "caballerizas"
      },
      {
        "hecho": "batalla",
        "cantidad": 3
      },
      {
        "hecho": "caravana",
        "cantidad": 2
      },
      {
        "hecho": "conquista",
        "cantidad": 1
      }
    ]
  },
  "carpinteria_militar": {
    "nombre": "Canto de las escalas de Dapur",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "carpinteria"
      },
      {
        "hecho": "defensa",
        "cantidad": 1
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "conquista",
        "cantidad": 1
      }
    ]
  },
  "instituciones_civicas": {
    "nombre": "Canto de la asamblea",
    "capitulos": [
      {
        "hecho": "ascenso",
        "cantidad": 1
      },
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "mercado"
      },
      {
        "hecho": "caravana",
        "cantidad": 3
      },
      {
        "hecho": "adopcion",
        "cantidad": 1
      },
      {
        "hecho": "ascenso",
        "cantidad": 1
      }
    ]
  },
  "ciudadania_militar": {
    "nombre": "Canto del ciudadano en armas",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "barracon"
      },
      {
        "hecho": "defensa",
        "cantidad": 2
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "adopcion",
        "cantidad": 1
      },
      {
        "hecho": "defensa",
        "cantidad": 1
      }
    ]
  },
  "falange_hoplita": {
    "nombre": "Canto de las lanzas largas",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "barracon"
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "defensa",
        "cantidad": 2
      },
      {
        "hecho": "batalla",
        "cantidad": 3
      },
      {
        "hecho": "conquista",
        "cantidad": 1
      }
    ]
  },
  "pantalla_escaramuzadores": {
    "nombre": "Canto de la jabalina ligera",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "galeriaDeTiro"
      },
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "armeria"
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "defensa",
        "cantidad": 1
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      }
    ]
  },
  "arqueria_especializada": {
    "nombre": "Canto de los arqueros de la estepa",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "galeriaDeTiro"
      },
      {
        "hecho": "defensa",
        "cantidad": 1
      },
      {
        "hecho": "batalla",
        "cantidad": 3
      },
      {
        "hecho": "caravana",
        "cantidad": 2
      },
      {
        "hecho": "conquista",
        "cantidad": 1
      }
    ]
  },
  "forja_hierro_estandarizada": {
    "nombre": "Canto del acero templado",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "fundicion"
      },
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "minaHierro"
      },
      {
        "hecho": "caravana",
        "cantidad": 3
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "adopcion",
        "cantidad": 1
      }
    ]
  },
  "bronce_laminado": {
    "nombre": "Canto de la coraza musculada",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "armeria"
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "defensa",
        "cantidad": 2
      },
      {
        "hecho": "conquista",
        "cantidad": 1
      },
      {
        "hecho": "adopcion",
        "cantidad": 1
      }
    ]
  },
  "caballeria_organizada": {
    "nombre": "Canto de los jinetes de Tiglat-Pileser",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "caballerizas"
      },
      {
        "hecho": "batalla",
        "cantidad": 3
      },
      {
        "hecho": "caravana",
        "cantidad": 2
      },
      {
        "hecho": "conquista",
        "cantidad": 1
      },
      {
        "hecho": "batalla",
        "cantidad": 3
      }
    ]
  },
  "trabajos_asedio": {
    "nombre": "Canto de las rampas de Laquis",
    "capitulos": [
      {
        "hecho": "obra",
        "cantidad": 1,
        "edificio": "carpinteria"
      },
      {
        "hecho": "defensa",
        "cantidad": 2
      },
      {
        "hecho": "conquista",
        "cantidad": 1
      },
      {
        "hecho": "batalla",
        "cantidad": 2
      },
      {
        "hecho": "conquista",
        "cantidad": 1
      }
    ]
  }
}
```
<!-- /parametros -->

## 12. Invariantes

| Invariante | Dónde se vigila |
|---|---|
| Solo las de arranque van sin logro; todas las demás llevan logro con umbral positivo | `tecnologia.test.ts` |
| Un hito solo pide tecnologías de su Era o anteriores; cada Era tiene tarifa de adopción | ídem |
| Una tecnología de una Era cerrada no aparece aunque su logro y su hito se cumplan; el logro queda fijado | ídem |
| La Era avanza al agotar el plazo o al cumplir todos sus logros, lo que llegue antes | ídem |
| Comprar a un Aeda nunca salta el logro del servidor | `aedas.test.ts` |
| Una épica no cuenta dos veces el mismo hecho, ni dos hechos antes del enfriamiento | `aedasResidentes.test.ts` |
| Un Aeda de una plaza conquistada pierde la épica | ídem |

## 13. Casos límite y decisiones que parecen raras

- **Una sola Era por tick**, aunque se cumplieran las condiciones de dos.
- **El logro de una Era posterior puede cumplirse antes de que empiece**: queda fijado, y la tecnología aparece en cuanto la Era llega y se cumple el hito.
- **`plazasEnNivel.N` es un récord**, no una suma: una plaza que sube, baja y vuelve a subir no cuenta dos veces.
- **Revelar es a la Facción dueña de la plaza donde está el Aeda**, sea amiga o enemiga: los Aedas son neutrales y no tienen Facción.
- **Un Aeda itinerante nunca muere** y su número solo crece (si bajan las Facciones, sobran).
- **Un hecho de plaza solo inspira al Aeda de esa plaza**; uno de Facción, a los de todas sus plazas.
- **A lo sumo un hecho por Aeda por evaluación**, y evaluar ocurre en cada comando y tick (no solo en el tick).
- **Una tecnología solo aparecida por épica o compra no cuenta para el campamento de mercenarios** si nadie la ha desbloqueado por hito (`primeros` no se registra por esas vías).

## 14. Divergencias

Abierta una: la épica no registra descubridora (ver [`Divergencias.md`](Divergencias.md)). Las **Eras IV y V** son diseño a medias aún sin implementar (no es una divergencia: el código solo llega a la III).
