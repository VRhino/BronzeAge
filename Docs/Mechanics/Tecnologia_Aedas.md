# Tecnología por Eras y Aedas

> **Reescrito el 2026-10-13 para el mundo reactivo.** Donde este documento dice «cada tick», léase **«en cada evaluación»**: los contadores suben **al ocurrir** el hecho (la extracción y la fabricación son ritmos y acumulan al cambiar de ritmo y en cada muestreo), la Era se mira al fijarse un logro y en una cita al fin de su plazo, la aparición se evalúa **al ocurrir lo que mira su hito** (los disparadores de `Docs/Arquitectura/Sistema6_Contraste_y_Plan.md` §5.1; sin muestreo), los Aedas itinerantes se mueven por **planes** con eventos de llegada y de fin de estancia, y los residentes llegan por eventos agendados ([`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §3). **Las Eras IV y V siguen pospuestas** (se definen con el motor nuevo). El «cómo funcionaba» por tick está en `typescript-final:Docs/Mechanics/Tecnologia_Aedas.md`.

Cómo llega una tecnología a una Facción (logro del servidor + hito propio, compra a un Aeda o épica de un residente), cómo avanzan las Eras, cómo se adopta y qué hacen los Aedas: itinerantes que revelan y venden,
residentes que cantan épicas, y la crónica que narra los hechos públicos.

- **Verificado contra:** `4d65075` (2026-10-11).
- **Módulos:** `src/engine/tecnologia.ts` (logros, Eras, aparición, adopción, revelar, vender), `src/engine/aedas.ts` (itinerantes), `src/engine/aedasResidentes.ts` (residentes y épicas), `src/engine/cronica.ts` (crónica),
  `src/engine/reclutamientoMercenario.ts` (`tecnologiasDelCampamento`), `src/session/comandos/tecnologia.ts` (comandos), `src/session/resultadoBatalla.ts` (`contadoresDeBatalla`), `src/session/comandos/tipos.ts` (`exito`),
  `src/session/proyecciones/jugador.ts` (`tecnologiaParaJugador`).
- **Canon:** Doc 6 (6.1–6.7), Doc 2.9 (títulos), Doc 5.8 (qué tropa pide qué tecnología). Los números de **calibración** (las X de los logros de la Era I están balanceadas; las de las Eras II y III, las tarifas y los cupos son placeholder); el efecto de gatear con tecnología en construcción, mejoras, recetas y reclutamiento está en
  [`Economia_Construccion.md`](Economia_Construccion.md).
- **Fuera de alcance aquí:** el reclutamiento en sí (Doc 5.8); la proyección completa del jugador ([`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) para lo que se ve de un Aeda).

## 1. Modelo de datos

```
EstadoTecnologia (del servidor) = {
  era:        EraId                                  la vigente
  eraDesde:   Instante                               cuándo empezó (su plazo cuenta desde aquí)
  contadores: Partial<Record<ContadorLogro, number>> los hechos del mundo, acumulados
  logros:     Partial<Record<TecnologiaId, Instante>>  cuándo se cumplió el logro de cada tecnología; FIJO para siempre
  primeros:   Partial<Record<TecnologiaId, { faccionId, en }>>   la primera Facción que la desbloqueó (descubridora), por hito o por épica
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
| `reclutados.escuadrones` | 1 por cada escuadra **nueva** al reclutar (`antes == 0`), en una plaza (reponer incluido) **o en un campamento de mercenarios** (Q4 del autor, 2026-10-24) |
| `reclutados.caballeria` | soldados reclutados de una tropa cuyo edificio es Caballerizas, **× 5** si es `carros_guerra`, × 1 el resto |
| `reclutados.arqueros`, `.escaramuzadores_jabalina`, `.arqueros_compuesto`, `.jinetes_asirios` | los soldados reclutados de esa tropa (`reclutados.<tropaId>` si el contador existe) |
| `plazasEnNivel.2 / .3` | **récord** de cuántas plazas del mundo han tenido a la vez `nivelActual ≥ N` (se actualiza cada tick) |

Todos los `reclutados.*` cuentan igual en una plaza y en un campamento de mercenarios (Q4 del autor, 2026-10-24; el TS solo contaba la plaza). La guarnición de serie de las avanzadas y la tropa prestada no cuentan.

**En Rust (6B) se cuenta en origen**, nunca a partir de los hechos: cada sitio que produce el hecho suma lo suyo al ocurrir, así que un hecho que deja dos rastros (el asedio con números tras su convocatoria
emite `combate.asedio_*` y `batalla.aplicada`) cuenta una vez. Lo extraído es lo que **sale del nodo**, en la plaza y en sus pueblos extractores (lo que el carro de acarreo vuelca después en la madre no
cuenta otra vez); lo extraído y lo fabricado llegan en milésimas por minuto y el contador sube una unidad cada vez que su resto oculto la completa. Una batalla de Unity suma al aplicar su resultado (con la
muralla de la plaza tal como estaba antes del daño de la conquista); la de una avanzada o un saqueo de pueblo, contextos que el TS no tenía, cuenta solo como batalla librada. `asedios.resistidosConResidentes`
con números es lo del TS: algún defensor combatió en persona (con escuadras que no son de la guarnición). La Caravana de Fundación capturada cuenta como cualquier caravana.

## 3. El tick de tecnología (`avanzarTecnologia`)

Determinista y **sin generador aleatorio**. Los pasos no se evalúan en un tick: los récords y los logros, al sumar el contador; la Era, al fijar un logro y en una **cita global** al fin de su plazo
(prioridad 10, [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §3.2); la aparición, al ocurrir lo que mira su hito; la crónica, en la cita de conocimiento de cada descubrimiento.

```
0. récords:  para N en {2, 3}:  ahora = nº de plazas con nivelActual ≥ N;  si ahora > contadores['plazasEnNivel.N'] → se guarda
1. logros:   para cada tecnología con logro, sin logro cumplido, con contador ≥ umbral:  logros[id] = instante;  evento público tecnologia.logro
2. Era:      siguiente = la Era que sigue a la vigente (ninguna en la última)
             todos   = TODAS las tecnologías de la Era vigente que tienen logro lo tienen cumplido
             porPlazo= instante − eraDesde ≥ plazoSemanas × 7 días
             si todos ∨ porPlazo:  era = siguiente, eraDesde = instante;  evento público era.comienza { era, porPlazo: !todos }       ← solo UNA Era por evaluación
3. aparición: para cada Facción con ≥ 1 asentamiento, para cada tecnología en el orden del catálogo:
             saltar si es de arranque, ya aparecida, sin logro cumplido, o de una Era posterior a la vigente (con la Era ya actualizada en 2)
             si TODAS las condiciones del hito se cumplen → aparece: se añade a aparecidas (y se quita de reveladas);
                 si primeros[id] no existe → primeros[id] = { faccionId, en: instante } (la descubridora);
                 evento tecnologia.aparece { faccionId, tecnologiaId, primera } atribuido a la capital (privado de la Facción)
4. crónica:  para cada tecnología aún no cantada que los Aedas YA conocen (§6.1): se canta;  evento público aedas.canta_descubrimiento { tecnologiaId, faccionId: la descubridora }
```

Las condiciones del hito se miran con el estado de la Facción **al empezar la evaluación de cada tecnología** (una condición `tecnologia` cuenta las **adoptadas**, no las aparecidas).

> **Implementado en Rust (2026-10-25, 6C): la aparición por disparadores** (`crates/nucleo/src/nucleo/tecnologias.rs`). Lo que cambia algo que mira un hito marca a la Facción y, al cerrar la versión, a cada Facción marcada que siga viva le aparecen sus candidatas con el hito cumplido (`tecnologia.aparece { faccion, primera; tecnologia }`, privado, atribuido a su capital), en la misma versión que su causa: fijarse un logro o empezar una Era (todas las vivas); completarse o mejorarse un edificio, subir el nivel actual de una plaza o crecer su zona (su Facción); fundar, conquistar, anexionar, fusionar, perder una plaza o designar la capital (las Facciones de antes y de ahora); adoptar (también por administración); y el paso de minuto de la capital solo si una candidata pide `recursoEnCapital`, tiene su logro y su hito ya se cumple (O(1) en el paso: hoy solo la aleación del bronce). Al registrarse la descubridora (por cualquier vía) se programan sus dos citas (§5 y §6.1). Paso 4, la crónica de descubrimientos: en la **cita de conocimiento** de cada tecnología (`primeros.en + retrasoConocimiento`, prioridad 10) y también al fijarse un logro o empezar una Era (una descubridora sin logro, por administración, o de una Era aún cerrada se canta en cuanto se puede); `aedas.canta_descubrimiento { faccion: la descubridora; tecnologia }` es público y va a la crónica.

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

> **Implementado en Rust (2026-10-25, 6C):** la orden `AdoptarTecnologia` (comando 175, `adoptar_tecnologia` del contrato; la tecnología por su id del catálogo) la da la cuenta del Rey de la Facción (`ReyDeLaFaccion`), y la Facción tiene que existir. Rechazos finos en el orden de arriba: `tecnologia.desconocida`, `tecnologia.sin_capital` (sin capital efectiva), `tecnologia.rey_fuera_de_la_capital` (el Rey no está **dentro** de ella), `tecnologia.ya_adoptada` (también las de arranque), `tecnologia.no_aparecida` y `tecnologia.sin_tarifa` (no se cobra nada). La tarifa sale del almacén de la capital en unidades enteras. `tecnologia.adoptada { faccion, capital; tecnologia }` es privado, atribuido a la capital; deja el hecho de épica `adopcion` para 6E. El `bonusProduccion` de las adoptadas (§4.1: `canteria` ×1,5 la piedra) es el `factor_tecnologia` del paso de minuto de cada plaza y de sus pueblos, en ppm, aplicado al ritmo antes del resto oculto.

### 4.1 Qué gatea una tecnología

Una tecnología adoptada habilita —las «cuatro puertas» del Doc 6.1— construir un edificio (`EDIFICIO_CATALOGO[..].requiereTecnologia`), mejorar un nivel interno (`niveles[n].requiereTecnologia`), producir una receta (`receta.requiereTecnologia`) o reclutar una tropa (`TROPAS_RECLUTABLES[..].tecnologia`);
y algunas dan un `bonusProduccion` (hoy solo `canteria`: ×1,5 la piedra que sacan las Canteras de la Facción). Cada sistema comprueba contra `tecnologiasDe(estado, faccionId).adoptadas`.

## 5. Campamentos de mercenarios (`tecnologiasDelCampamento`)

Un campamento no adopta nada: **desbloquea** cada tecnología —al instante y gratis— cuando se cumple lo que llegue antes de:

```
(instante − primeros[id].en) ≥ horasTrasLaPrimera × 60 min      ← primeros[id] debe existir: una tecnología que nadie ha desbloqueado (por hito, épica o compra) no está en el campamento
(Facciones con ≥ 1 asentamiento que la tienen ADOPTADA) / (Facciones con ≥ 1 asentamiento) ≥ porcentajeFacciones     (0 Facciones → no)
                                                                                                  bots y humanos cuentan igual: juegan por la misma vía
```

Las de arranque están siempre. «Vivas» son todas las Facciones que conservan algún asentamiento (no se distingue a los bots).

> **Implementado en Rust (2026-10-20, fase 4B-1), provisional:** `militar::datos::ReglasTecnologias` lee las claves del bloque `TECNOLOGIAS` (las que existen y las `deArranque`). Mientras no exista este sistema, la administración da tecnologías a una Facción con `FijarTecnologias` (solo ids que existan); toda Facción tiene las de arranque. El campamento desbloquea una tecnología si es de arranque o si la tienen adoptada al menos `porcentajeFacciones` de las Facciones vivas; el plazo de `horasTrasLaPrimera` (que pide `primeros`) no existe aún. Esas mismas tecnologías son las que miran los requisitos de edificios, recetas, pueblos y tropas.

> **Implementado en Rust (2026-10-25, 6C):** el campamento desbloquea por las reglas de esta sección (`EstadoTecnologia::tecnologias_del_campamento`): las de arranque; las demás, si tienen descubridora y pasaron `horasTrasLaPrimera` (cada tecnología descubierta deja su **cita de campamento** a `primeros.en + 72 h`, que marca todos los campamentos para quien los mira) o si las tiene **adoptadas** al menos `porcentajeFacciones` de las vivas (adoptar, nacer o morir una Facción viva también los marca). Lo que ofrece un campamento no se guarda: se mira al reclutar y al proyectar.

## 6. Aedas itinerantes

### 6.1 Qué saben

Un Aeda **conoce** una tecnología (`conocidaPorAedas`) si: su logro está cumplido ∧ existe `primeros[id]` ∧ `primeros[id].en + retrasoConocimientoMinutos ≤ instante` ∧ su Era ≤ la vigente. Es la ventaja del primero: 24 horas desde que alguien la desbloquea por hito.

### 6.2 Cuántos, y su vida

```
objetivo = (hay asentamientos) ? max(itinerantes.minimo, ceil(Facciones con algún asentamiento / faccionesPorAeda)) : 0
nacen los que faltan hasta el objetivo (nunca se eliminan si baja):  id 'aeda-<i+1>', repartidos en las plazas que los aceptan ordenadas por id, la i-ésima en la plaza (i mod n), ya "llegada" (con su estancia)
una plaza los acepta salvo que su Facción les haya cerrado la puerta con el grupo 'aedas' (puertaCerradaA; el cierre por defecto no los incluye)
```

En cada evaluación, para cada Aeda **en el orden del array** (y luego los recién nacidos):

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

**La puerta que se cierra con un Aeda dentro** (Q5 del autor, 2026-10-24): cerrarle la puerta (el grupo `aedas`) a un Aeda parado en la plaza lo hace **salir al instante** hacia su siguiente destino (`partir` desde la plaza), como sale cualquiera que ya no puede entrar; lo que ya reveló queda revelado. El que llega a una plaza que le cerró la puerta mientras iba de camino no entra: sigue hacia la siguiente (en el TypeScript solo se miraba al elegir destino). Si la plaza en la que está desaparece, parte desde ella; si desaparece su destino, elige otro desde donde está.

### 6.3 Revelar (`revelarTecnologias`)

En cada evaluación, por cada Aeda **detenido en una plaza** cuya Facción existe, en el orden del array: para cada tecnología que el Aeda **conoce** (§6.1) y la Facción no tiene **aparecida ni revelada**, se añade a `reveladas` y se emite `aedas.revela { faccionId, tecnologiaId, descubridorFaccionId, faltan[] }`
(atribuido a la plaza; privado). `faltan` es el texto de las condiciones del hito que la Facción aún no cumple. Revelan a **la Facción dueña de la plaza**, sea cual sea (también a una enemiga). Una tecnología revelada pasa a aparecida —y sale de `reveladas`— cuando se cumple el hito.

### 6.4 Vender (`venderTecnologia`, comando `comprarTecnologiaAeda`)

Hay un Aeda detenido en la plaza (`aedaEn`). Validaciones, en orden: la tecnología existe; su Era es vendible (`orden ≤ AEDAS.venta.ordenEraMaximo`); el Aeda la **conoce**; no está ya aparecida;
el almacén de **la plaza** tiene el oro: `precio = TARIFA_ADOPCION[era].oro × AEDAS.venta.factorOro`.
Efecto: la plaza paga; la tecnología pasa a **aparecida** (no a adoptada: adoptarla se paga igual, §4); evento `aedas.venta` (privado). **Se salta el hito, nunca el logro del servidor.**

## 7. Aedas residentes

Viven en un asentamiento con **Palacio activo** y **Nobleza > 0**. Dan felicidad y crecimiento de nobleza ([`Economia_Construccion.md`](Economia_Construccion.md) §9.1) y cuentan para el título de prestigio «Mecenas de los Aedas» (Doc 2.9: valor = épicas cumplidas × 10 + Aedas residentes).

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

A lo sumo **llega uno por plaza y evaluación**; si sigue habiendo cupo, la espera vuelve a empezar.

> **Implementado en Rust (2026-10-25, 6E): el ciclo de vida sin tick** (`crates/nucleo/src/nucleo/residentes.rs`, foto `BAN40`). El cupo se mira con el **nivel actual** y solo cuando puede cambiar: en el paso de minuto si la nobleza pasa de 0 a más (o al revés) o cambia el nivel actual, al completarse un Palacio, al fijar el nivel por administración, en la conquista (que daña edificios) y al cambiar la reputación de la dueña. Mientras la plaza tiene cupo libre espera y tiene **una sola cita** en la cola (`Carga::Residentes`, prioridad 10): a `espera + llegadaMinutos`; si entonces la reputación de la dueña está en `umbralReputacionBaja` o por debajo (una Facción sin reputación guardada la tiene en 0), en el primero de `espera + llegadaMinutos × factorLlegadaReputacionBaja` y el cruce hacia arriba del umbral (decisión técnica 6 del plan del sistema 6: la semántica del TS «se mide al comparar», sin muestreo). Sin cupo libre no hay espera ni cita. Tras llegar uno, la espera vuelve a empezar en ese mismo instante (el TS, en el tick siguiente). Paso 1 (la plaza que ya no existe): la ruina se lleva a sus residentes, su espera y su cita, sin hecho, como el TS. Paso 2: la conquista pasa los residentes a la conquistadora y les quita la épica **con aviso** (`aedas.epica_abandonada { motivo: conquista }`, a la Facción de antes; C9); la tecnología que aparece por otra vía cierra las épicas que la cantaban (`motivo: aparecida`) en el momento en que aparece. La anexión y la fusión (decisión técnica 13) pasan los residentes de la que desaparece a la que recoge **con su épica** (como `trasladoDeFaccion.ts`) y le suman sus épicas cumplidas; la épica de una tecnología que la que recoge ya tiene se pierde (`aparecida`). Los ids son `aeda-rN`, con su contador propio. «Mecenas de los Aedas» (Doc 2.9) mide `cumplidas × 10 + residentes de sus plazas`.

### 7.2 Épicas

`empezarEpica(plaza, aedaId, tecnologiaId)` (Rey, Gobernador o Sacerdote de la plaza, presentes) exige, en orden: el Aeda vive en esa plaza; la tecnología tiene épica (`EPICAS`); el Aeda es de la Facción dueña actual; el Aeda no canta ya otra épica; el **logro** del servidor de esa tecnología está cumplido;
su Era ha empezado; **no ha aparecido** a la Facción; y ningún otro Aeda de la Facción canta ya su épica. Crea `epica = { tecnologiaId, capitulo: 0, hechos: 0, claves: [] }` (evento `aedas.epica_empieza`). No cuesta oro. Una épica puede empezar aunque la tecnología solo esté revelada, o ni eso.
`abandonarEpica` borra la épica en curso y todo lo avanzado (`aedas.epica_abandonada`).

**Hechos inspiradores** (`hechosDeEventos`), sacados de los eventos de cada tick o comando (se evalúan en `exito`, [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §7):

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

> **Implementado en Rust (2026-10-25, 6E): las épicas** (`crates/nucleo/src/nucleo/residentes.rs`). `EmpezarEpica` (177) y `AbandonarEpica` (178) con `dirigeAedas` y los rechazos finos `epica.*` de este apartado, en su orden (decisión técnica 9; `Proyecciones_Y_Autorizacion.md` §5.1). Los hechos inspiradores **no se sacan de los eventos**: se anotan tipados en la transición donde ocurren (decisión técnica 3) —la defensa y la conquista del asedio con números al resolverlo (con `enCombate` = hubo tirada), el encuentro entre columnas de Facciones distintas, la batalla de Unity al aplicar su resultado (nunca desde `batalla.aplicada`, D18), la obra y el ascenso de la plaza, la adopción y la llegada de una caravana **comercial** con carga total ≥ `cargaMinimaCaravana` (D9: la carga y el tipo los mira el núcleo en la llegada, no el hecho)— y se procesan al cerrar la versión, antes de la aparición por hito: a lo sumo uno por Aeda y versión, con el enfriamiento y las claves. Lo que aparece por una épica entra en la misma versión; si es la primera del mundo, programa sus dos citas (conocimiento y campamento) como cualquier descubrimiento. `aedas.epica_cumplida` es público y entra en la crónica (§8), sin nombrar la tecnología.

`hacerAparecer(estado, facción, id, instante)` registra `primeros[id] = { faccionId, en: instante }` si nadie la tenía (un descubrimiento lo es sea cual sea la vía, decidido 2026-10-11): desde entonces los Aedas itinerantes la conocen tras el retraso, se canta y los campamentos la desbloquean. Cada tecnología con logro tiene su épica (`EPICAS`): 3 capítulos en la Era I, 4 en la II, 5 en la III.

## 8. Crónica (`entradasDeCronica`)

No es estado: es una **vista de los eventos**. Se calcula en `exito` sobre los hechos de cada tick o comando y se emite como eventos **públicos** (sin plaza atribuida). Entran tal cual los eventos públicos `tecnologia.logro`, `era.comienza`, `titulo.nace`, `titulo.cambia_manos`,
`aedas.canta_descubrimiento` y `aedas.epica_cumplida`. Se **derivan** entradas `cronica.entrada { tipo, faccionIds }` de:

| Evento | Entrada |
|---|---|
| `combate.asedio_conquista` | caída de la plaza: quién la arranca de quién (una de dos frases, según un hash del momento y el mensaje) |
| `asentamiento.ruinas` | lamento por las ruinas de la plaza |
| `fundacion.asentamiento_fundado` | fundación de la plaza por la Facción |
| `diplomacia.guerra_declarada`, `.rebelion_vasallo`, `.vasallo_liberado`, `.vasallo_conquistado`, `.anexion`, `.fusion` | la frase de cada hecho |
| `asentamiento.nivel_subio` con `nivelNuevo ≥ 3` | solo si es **la primera plaza del mundo** en llegar a ese nivel (ninguna otra, fuera de las que suben en ese mismo lote, tenía `nivel ≥` el nuevo); si dos suben a la vez se canta una sola |

La variante de una frase es determinista: `hash(clave) mod nº de opciones`, con `h = (h·31 + código del carácter) mod 2³²`.

> **Implementado en Rust (2026-10-23, 4H T5).** `nucleo/cronica.rs`: al cerrar cada versión, de sus hechos sale `cronica.entrada { tipo, faccionIds, asentamientoId?, nivel? }` **público** (lo oyen todos; sin sitio ni nombrados), de `asentamiento.conquistada` (la conquista por números y por Unity: la de Unity no emite `combate.asedio_conquista`), `asentamiento.ruinas`, `fundacion.asentamiento_fundado`, `faccion.guerra_declarada` (el equivalente de `diplomacia.guerra_declarada` mientras no haya sistema político) y `asentamiento.nivel_subio` con la regla de la primera plaza del mundo desde el nivel 3. El contrato no lleva texto: la frase y su variante las compone el cliente. Faltan las fuentes de la diplomacia (sistema 5) y los públicos de la tecnología y los Aedas (sistema 6).

> **Implementado en Rust (2026-10-25, 5F).** La crónica canta también `diplomacia.anexion` (la absorbente y la absorbida), `diplomacia.fusion` (las dos que se disuelven y la que nace) y los títulos: `titulo.nace` y `titulo.cambia_manos` son hechos públicos y además dejan su `cronica.entrada` (con la clave del título en `titulo`).

> **Implementado en Rust (2026-10-25, 6C): los públicos del sistema 6.** `tecnologia.logro` (con su contador y su umbral), `era.comienza` (con su Era, su orden y si llegó por plazo) y `aedas.canta_descubrimiento` (la descubridora y la tecnología) son hechos públicos que además dejan su `cronica.entrada`, como los títulos: así entran en el historial guardado. `aedas.epica_cumplida` entra con 6E (la Facción, sin la tecnología). C4 queda como Rust: la caída sale de `asentamiento.conquistada`, que dan las dos vías.

> **Implementado en Rust (2026-10-25, 6C, infraestructura; decisión Q2 del autor: todo el historial).** La crónica **se guarda entera**, fuera de la foto y del hash, para quien llega tarde. En el núcleo persistente cada `cronica.entrada`, ya como el `EventoDominio` que viaja en las `Delta` (≈ 130 bytes), va con la versión que la produjo a un diario aparte (`<partida>/cronica/`, `persistencia::Diario`: un `Registro` de segmentos con longitud y CRC32 y un índice en memoria de versión → desplazamiento cada 256 entradas). Lo escribe por lotes el hilo de la persistencia, fuera del camino de la orden, y antes de escribir una foto escribe y sincroniza lo anterior. Al recuperar se truncan las entradas posteriores a la versión de la foto y la repetición del registro las vuelve a escribir idénticas: ni duplicadas ni perdidas, se corte donde se corte. Se lee por páginas con `PedirCronica` → `Cronica` (contrato 05 §8): 200 por defecto, 1.000 como mucho, hacia atrás hasta una versión o hacia delante desde ella, sin partir nunca las entradas de una misma versión; es la misma para todos y la puede pedir cualquier sesión. Las entradas nuevas siguen llegando en las `Delta`. Un núcleo sin persistencia (pruebas, batch) no la guarda y contesta páginas vacías. Faltan las fuentes públicas del sistema 6 (`tecnologia.logro`, `era.comienza`, `aedas.canta_descubrimiento`, `aedas.epica_cumplida`), que llegan con 6B-6E.

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
  "radioProteccion": 10,
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

Cada entrada: `era`, `deArranque?`, `logro {contador, umbral}` (el umbral es la X del logro: **calibrado con batch para la Era I**; las de las Eras II y III siguen siendo placeholder), `hito[]` y `bonusProduccion?`.

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

- **Una sola Era por evaluación**, aunque se cumplieran las condiciones de dos.
- **El logro de una Era posterior puede cumplirse antes de que empiece**: queda fijado, y la tecnología aparece en cuanto la Era llega y se cumple el hito.
- **`plazasEnNivel.N` es un récord**, no una suma: una plaza que sube, baja y vuelve a subir no cuenta dos veces.
- **Revelar es a la Facción dueña de la plaza donde está el Aeda**, sea amiga o enemiga: los Aedas son neutrales y no tienen Facción.
- **Un Aeda itinerante nunca muere** y su número solo crece (si bajan las Facciones, sobran).
- **Un hecho de plaza solo inspira al Aeda de esa plaza**; uno de Facción, a los de todas sus plazas.
- **A lo sumo un hecho por Aeda por evaluación**, y evaluar ocurre en cada orden y en cada evento (no solo en un muestreo).
- **Una tecnología aparecida por épica o compra sí cuenta para el campamento de mercenarios**: `primeros` se registra por cualquier vía (corregido el 2026-10-17; no registrarlo era un error). La compra a un Aeda **nunca** es la primera: un Aeda solo vende lo que conoce, y conocer exige descubridora (§6.1); así que solo la épica puede registrar `primeros` (precisado en 6D, duda 1 de 6A).

## 14. Divergencias

Ninguna abierta. Las **Eras IV y V** son diseño a medias aún sin implementar (no es una divergencia: el código solo llega a la III).

Divergencia aprobada del TS (Q4 del autor, 2026-10-24): los reclutas de un campamento de mercenarios también suman a `reclutados.*` (§2.1).

> **Implementado en Rust (2026-10-25, 6A): las reglas puras**, en el crate `crates/tecnologia` (sin núcleo; lo conectan 6B-6F). `datos` lee de este documento `TECNOLOGIAS` (en el orden del documento: el índice es el id interno), `ERAS`, `TARIFA_ADOPCION`, `AEDAS`, `EPICAS`, `TITULO_CAPITULO` y `MERCENARIOS.tecnologia`, con los minutos en ms, los factores en ppm y las tarifas en unidades enteras; las cifras son las del TS (Q3). `militar::datos::ReglasTecnologias` vive ahora en este crate (militar la reexporta). `contador`: los 24 contadores como enum y las vías que suman (extracción, fabricación, combate con números, asedio, batalla de Unity, reclutamiento con Q4). `estado`: sumar con el siguiente umbral pendiente en O(1), récords, la Era (una por llamada, cita de fin de plazo, sin plazo en la III), candidatas, `hacer_aparecer`, conocida por los Aedas y la crónica de descubrimientos. `hito`: las cinco condiciones sobre una vista que implementa el núcleo, la aparición por hito y revelar (`faltan` como índices del hito). `adopcion`: adoptar (con rechazos finos), vender, el bonus de producción y las tecnologías del campamento (`primeros` + 72 h o el 50 % de las vivas con ella **adoptada**). `itinerantes`: objetivo, nacimiento repartido por id, llegada con `recientes`, candidatos por distancia y un solo evento pendiente por Aeda. `residentes`: cupo por nivel actual, la llegada exacta con la reputación baja (decisión técnica 6), ajuste al cupo, cambio de dueño, aparición por otra vía y ruina. `epicas`: empezar y abandonar con los nueve rechazos, los hechos tipados con su clave y el avance (enfriamiento, claves recordadas, capítulos, cumplida). `alijos`: ver y abrir (oro hacia abajo a unidades enteras; lo que no cabe se pierde). Las pruebas portan los casos de `tecnologia`, `aedas`, `aedasResidentes`, `alijos`, `canteria`, `tecnologiasDelCampamento` y `comandosTecnologia` del TS (la parte de reglas) y los invariantes de §12 sobre el catálogo.

> **Implementado en Rust (2026-10-25, 6B): el estado del mundo, los contadores, los logros y las Eras en el núcleo** (`crates/nucleo/src/nucleo/logros.rs`; foto `BAN37`, `codec_tecnologia`). `Estado::tecnologia` (el `EstadoTecnologia` del crate) sustituye a las tecnologías que daba la administración. Los contadores se suman **en origen** (§2.1): lo extraído en `FuentesMundo::extraer` (plazas y pueblos) y lo fabricado por el gancho `Fuentes::fabricado` de las recetas, con un resto oculto en milésimas; el combate con números al resolverse (columna contra columna con su encuentro, caravana, acoso de bandidos, campamento, asedio); la batalla de Unity en `aplicar_resultado`; reclutar en una plaza y en un campamento; comprar un animal. `plazasEnNivel.N` sale de un índice derivado de plazas por nivel actual (no va en la foto) que sube y baja con cada cambio de nivel; el récord solo al subir. Al cruzar un umbral, `tecnologia.logro { contador, umbral }` público; la Era se mira al fijar un logro y en la cita global `Carga::Era` (prioridad 10), que se arma como el plazo de los títulos (con alguna Facción, al pasar algo en el mundo) y cae en el instante del fin del plazo, o en el mismo instante si la Era que empieza ya tiene todos sus logros (una Era por evaluación); `era.comienza { era, orden, porPlazo }` público; la III no tiene cita. El `Exterior` de las plazas lleva el `orden` de la Era real. `FijarTecnologias` queda como administración sobre las reglas reales: las dadas aparecen (con su descubridora si nadie la tenía) y las adoptadas de la Facción son las de arranque más las dadas, sin tarifa. En la fusión, la Facción nueva tiene la unión de los tres conjuntos; en la anexión, las de la absorbida se pierden; sus descubrimientos pasan a la que la recoge.

> **Implementado en Rust (2026-10-25, 6E): los Aedas residentes y las épicas** (`crates/nucleo/src/nucleo/residentes.rs`, foto `BAN40`; ver las notas de §7.1 y §7.2). Órdenes 177 y 178; `felicidad_extra` y `nobleza_extra` reales en el paso de minuto (`Economia_Construccion.md` §9.1); los datos de «Mecenas de los Aedas» para los títulos (5F).

> **Implementado en Rust (2026-10-25, 6C): la aparición, la adopción, los campamentos y las citas de cada descubrimiento** (`crates/nucleo/src/nucleo/tecnologias.rs`, foto `BAN38`). La aparición se evalúa por disparadores al cerrar la versión (§3), la adopción es la orden `AdoptarTecnologia` (175, §4), el `factor_tecnologia` del paso de minuto aplica el `bonusProduccion` de las adoptadas (§4.1), el campamento desbloquea por `primeros` + 72 h o por la mitad de las vivas con ella adoptada (§5), y cada tecnología descubierta deja dos citas de prioridad 10 (conocimiento y campamento) que vencen una vez. Los tres públicos de esta tanda entran en la crónica (§8). Sin tick ni muestreo: nada más programa eventos por la tecnología.

> **Implementado en Rust (2026-10-25, 6D): los Aedas itinerantes** (`crates/nucleo/src/nucleo/aedas.rs`, foto `BAN39`). `Estado::aedas` guarda cada `aeda-N` (posición, plaza en la que está parado y hasta cuándo, destino con su plan, `recientes`); cada uno tiene **un solo evento** en la cola (`Carga::Aeda`, prioridad 10): su llegada o el fin de su estancia, que solo se invalida cuando le cierran la puerta o desaparece su plaza o su destino. El viaje es un plan por la red de caminos (el A* de las caravanas con la atracción de la red, y los tramos de camino más rápidos) a `itinerantes.velocidad`; sin camino a ninguna candidata, otra estancia. El objetivo (§6.2) se mira al cambiar las Facciones vivas o al abrirse una puerta; nunca bajan. La puerta con el grupo `aedas` es la de 5E, con la regla de Q5 (§6.2). Revelan (§6.3) al llegar o nacer, al cambiar de dueño la plaza y cuando los Aedas conocen algo nuevo (en el mismo instante en que lo cantan: la cita de conocimiento, un logro que se fija, una Era que empieza); la revelación se hace al cerrar la versión, con lo que acaba de aparecer ya aparecido, y `faltan` son los índices del hito. La compra es la orden `ComprarTecnologiaAeda` (176, §6.4): el Rey o el Gobernador dentro de la plaza, con los rechazos finos `aedas.no_hay_aeda`, `aedas.era_no_vendible`, `aedas.no_la_conoce`, `aedas.ya_aparecida` y `aedas.sin_oro`; paga el almacén de la plaza en unidades enteras y vende a su Facción dueña. En la visión se muestrean como móviles de nadie; no entran en la rejilla de encuentros. Su proyección (`AedaItinerante` en `VISTA`) es de 6G.

## Catálogo de eventos

Los códigos de los hechos de dominio de este documento (4H T5, 2026-10-23; contrato 05 §8). La prueba `crates/nucleo/tests/catalogo.rs` lee el bloque de cada documento de `Docs/Mechanics` y exige que todo código que emite el núcleo esté en alguno, y que los que el núcleo aún no emite figuren como paridad pendiente en `Docs/Mecanicas a desarrollar.md`. Desde 6E se emiten todos: `tecnologia.logro` y `era.comienza` desde 6B; `tecnologia.aparece`, `tecnologia.adoptada` y `aedas.canta_descubrimiento` desde 6C; `aedas.revela` y `aedas.venta` desde 6D; `aedas.residente_llega`, `aedas.residente_se_va`, `aedas.epica_empieza`, `aedas.epica_capitulo`, `aedas.epica_tecnologia`, `aedas.epica_cumplida` y `aedas.epica_abandonada` desde 6E.

<!-- eventos -->
```json
[
  "aedas.canta_descubrimiento", "aedas.epica_abandonada", "aedas.epica_capitulo", "aedas.epica_cumplida", "aedas.epica_empieza", "aedas.epica_tecnologia",
  "aedas.residente_llega", "aedas.residente_se_va", "aedas.revela", "aedas.venta", "alijo.abierto", "cronica.entrada", "era.comienza", "faccion.tecnologias",
  "tecnologia.adoptada", "tecnologia.aparece", "tecnologia.logro"
]
```
<!-- /eventos -->
