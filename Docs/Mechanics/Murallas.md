# Murallas — los recintos

Cómo se traza, se paga, se mejora, se amplía, defiende, cuesta mantener y se daña un **recinto** (la muralla de un asentamiento). Un recinto **no es un edificio**: es un anillo cerrado de celdas
alrededor del casco urbano que se levanta celda a celda.

- **Verificado contra:** `e18c0b4` (2026-10-09).
- **Módulos:** `src/engine/muralla.ts` (≈980 líneas: trazo, comprometer, obra, mejora, ampliación, defensa, upkeep), `src/engine/trazado.ts` (suelo ocupado, red de calles, preferencia intramuros),
  `src/engine/combate.ts` (uso en el asedio y daño en la conquista), `src/engine/asentamientoQuery.ts` (cupo de guarnición), `src/engine/mantenimiento.ts` (upkeep),
  `src/session/comandos/murallas.ts` (comandos).
- **Canon:** Doc 4 «Murallas», Doc 4.5 (nivel 4), Doc 5.15.3 (guarnición). **Diseño e historia:** `typescript-final:Consideraciones/Murallas_Definicion.md`.
- Vocabulario de celdas, rejilla y red de calles: [`Trazado_Urbano.md`](Trazado_Urbano.md).

## 1. Idea

La muralla es una ventaja defensiva abrumadora —**menos puertas benefician al defensor**, que solo defiende un embudo— y por eso tiene que ser cara de obtener **y de mantener**. Eso fija la forma
de las reglas: el coste escala con el perímetro, existe un mantenimiento por celda, y el bono defensivo se reparte entre las puertas. Amurallar pronto da una fortaleza con pocas puertas; amurallar tarde
protege una metrópoli con más frentes que cubrir.

## 2. Modelo de datos

```
CeldaMuro = { col, row, clase: 'muro' | 'puerta' | 'torre' }

Recinto = {
  id:             'recinto-<idAsentamiento>-<n>'  (n = nº de recintos existentes + 1)
  nivel:          1 empalizada · 2 muro de piedra · 3 muralla con adarve
  celdas:         CeldaMuro[]   el anillo COMPLETO en orden de recorrido desde la puerta principal — es el orden en que se levanta. CONGELADO al comprometer
  avance:         índice de la última celda ya levantada. −1 = ninguna. Completo ⇔ avance = celdas.length − 1
  mejorandoA?:    nivel al que se está mejorando; mientras exista, `avance` mide el progreso de la MEJORA (se reinicia a −1 al empezarla)
  siguienteCeldaEn?: instante antes del cual no se levanta otra celda
  comprometidoEn, completadoEn?
}
Asentamiento.recintos?: Recinto[]    del más interior al más exterior
```

- `integridad = (avance + 1) / celdas.length` (0 si no hay celdas). Un recinto en mejora vuelve a integridad 0.
- Las celdas **no levantadas ocupan suelo igual** desde que se compromete (si no, se construiría encima y el anillo no podría cerrarse). Las **puertas no ocupan**.
- **Las puertas se congelan al trazar**: ni mejorar ni el crecimiento de la ciudad las cambian. Ganar puertas solo se consigue **ampliando**.

## 3. El trazo (`trazarRecinto`, función pura)

Entrada: el asentamiento y el `nivel` que se presupuesta (por defecto 1). Salida: `TrazoRecinto { celdas (ordenadas), puertas, torres, areaEncerrada, dentro[], fuera[], afuerasDentro[], costo }` o **`null`** (no hay
trazo válido: «un recinto que no se puede cerrar no es un recinto»). Determinista.

```
internos := edificios con ámbito 'asentamiento';  centro := el Centro Urbano (si no hay → null)
red := redDeCalles(id, edificios)                               // SIN recintos: el trazo se calcula sobre la ciudad, no sobre sí mismo

1. vetadas := celdas de granjas y corrales;  vetadasConMargen := dilatar(vetadas, 1)
2. núcleoBruto := celdas de los edificios urbanos (no afueras) ∪ celdas de CALLE (nunca caminos: un camino rural arrastraría la región hasta las granjas),
                  todo menos vetadasConMargen
   núcleo := componente conexa (vecindad 4) de núcleoBruto que contiene al Centro Urbano;  si vacío → null
3-5. semilla := núcleo
   repetir hasta MAX_ITERACIONES_CONFLICTO (24):
       dilatada := dilatar(semilla, franjaDeRonda)                         // vecindad 4 → esquinas achaflanadas
       región   := rellenarAgujeros( componente conexa del Centro Urbano en (dilatada − vetadasConMargen) , caja+2 )
       muro     := anillo(región) = celdas fuera de la región 4-adyacentes a ella
       conflictivos := edificios con alguna celda en muro
       si ninguno → resuelto
       si no: quitar de la semilla la vecindad (dilatada franjaDeRonda+1) de esos edificios; si no se quita nada → null; semilla := componente del Centro Urbano; si vacía → null
   si no resuelto → null
6. validar: un flood fill 4-conexo desde el borde de la caja que no cruce el muro no puede alcanzar el Centro Urbano; si lo alcanza → null
7. clasificarPuertas:  celda del muro que pisa la red (calle o camino) → 'puerta'; el resto 'muro'
8. ordenarAnillo → colapsarPuertas → marcarTorres
```

- **Dilatación** con vecindad 4 (`franjaDeRonda` pasos): sigue la forma real de la ciudad (una estrella de distritos), no un círculo ni un casco convexo.
- **`rellenarAgujeros`**: toda celda fuera de la región y sin salida al exterior de la caja pasa a la región (si no, un patio encerrado generaría un anillo interior). **Excepción conocida:** una
  granja o corral rodeado por los cuatro costados queda **intramuros** (`afuerasDentro`); no es un fallo, ya no son «las afueras». Aparece desde el nivel 3 (0,3–12 % de las celdas urbanas).
- **El anillo** es la capa 4-adyacente exterior: mide una celda y es impermeable (para entrar hay que pisar una celda 4-adyacente a la región, y todas son muro).
- **`ordenarAnillo`**: empieza en la **puerta de menor `(row, col)`** (la «principal»; si no hay puertas, la celda de menor `(row, col)`), y sigue por **vecindad 8** probando `dr = −1..1`, `dc = −1..1` (en ese orden)
  la primera vecina no visitada. Si el paseo se atasca en un punto grueso, añade las celdas que falten ordenadas por `(row, col)`.
- **`colapsarPuertas`** (una puerta por cruce): recorre el anillo **circularmente**; de cada tirada contigua de puertas conserva **una** celda —la primera que «atraviesa» (tiene red dentro de la región a un lado y red fuera del muro al otro),
  o la del medio (`floor(n/2)`) si ninguna lo hace— y devuelve el resto a `muro`. Importa porque **el nº de puertas es el divisor del bono defensivo**: un camino que rozara el muro cinco celdas dividiría la defensa por cinco.
- **`marcarTorres`** (solo niveles con `pasoTorres`: 2 y 3; la empalizada no tiene): recorre el anillo ya ordenado con un contador `desdeLaUltima` que empieza en `paso`. En cada celda: si es puerta, solo cuenta; si no,
  se convierte en torre cuando `desdeLaUltima ≥ paso` **y** es **esquina convexa** (la región está en su vertical y en su horizontal), o cuando se llevan `2 × paso` sin ninguna. Una puerta nunca es torre.
- **Coste del trazo** (`costoDeTrazo`): `Σ tarifaPorCelda[nivel] × factor(clase)`, con `factor` = `factorPuerta` (4) para puertas, `factorTorre` (3) para torres, 1 para muro; redondeado por recurso.

## 4. Comprometer

`comprometerRecintoManualmente(asentamiento, cargo, nivel, instante)` (comando `comprometerRecinto`, Gobernador o Maestro de Obras residente con ese cargo):

1. Debe haber alguien con ese cargo. 2. **Gate de nivel de asentamiento por nivel de recinto** (`MURALLA.nivelAsentamientoPorNivelDeRecinto`): empalizada (1) desde el asentamiento nivel 2, muro de piedra (2) desde el 3,
muralla con adarve (3) desde el 4 (`nivelActualDe`; decidido el 2026-10-09). 3. Delega en `comprometerRecinto`:

```
si algún recinto existente está incompleto → error («ya hay un recinto en obra»)
si hay recintos (AMPLIACIÓN):
    extramuros := edificiosExtramurosDe(asentamiento, último recinto);  si < arrabalMinimo (6) → error
trazo := trazarRecinto(asentamiento, { nivel });  si null → error
si hay recintos y el trazo no CONTIENE ESTRICTAMENTE al anterior → error
crear Recinto { nivel, celdas: trazo.celdas, avance: −1, comprometidoEn }
```

- **Comprometer es gratis.** Lo que cuesta es levantarlo, celda a celda (§5). Así «anillo a medio cerrar» es un estado normal y visible.
- **`contieneA(nuevo, anterior)`**: ninguna celda del anillo nuevo coincide con una del viejo, y un flood fill desde fuera del anillo nuevo no llega a ninguna del viejo sin cruzar el muro nuevo. Es el invariante «el área encerrada nunca decrece».
- **Extramuros** = edificios urbanos (no granja/corral) con alguna celda fuera del interior del recinto.
- El interior de un recinto ya congelado (`regionInteriorDeRecinto`) es la **inundación 4-conexa desde una celda del Centro Urbano** que no cruza ninguna celda del anillo (**las puertas bloquean** aquí: son un hueco para quien camina,
  no un agujero en la contabilidad del interior), acotada a la caja del anillo +1. `areaEncerradaDeRecinto` da el mismo número en cualquier tick posterior (el anillo no cambia).
- Un recinto se puede comprometer **directamente en cualquier nivel permitido por su gate** (el esquema acepta 1, 2 o 3) pagando solo la tarifa de ese nivel, sin pasar por los anteriores. Es intencional (2026-10-10): permite defenderse antes con lo que se pueda, o levantar la muralla más tarde con la ciudad más grande.

## 5. La obra (`avanzarObraDeRecintos`)

La corre el tick de construcción de cada asentamiento (`construction.ts`) **después** de `evaluarNecesidades`, para que la construcción normal tenga preferencia sobre los materiales. No ocupa hueco de la cola de edificios.

```
para cada recinto incompleto (avance < celdas.length − 1):
    nivelDePago := mejorandoA ?? nivel
    si siguienteCeldaEn existe y instante < siguienteCeldaEn → nada
    coste := tarifaPorCelda[nivelDePago] × factor(clase de la celda avance+1)
    si no hay recursos, o pagarlos dejaría un recurso por debajo de la RESERVA de mantenimiento → nada este tick (no falla, no se cancela, no hay deuda)
    descontar; avance += 1;  siguienteCeldaEn := instante + minutosPorCelda[nivelDePago] × factorTiempoMuralla
    si completa:
        si hay mejora: nivel := mejorandoA; recalcular torres sobre el mismo anillo (marcarTorres con la región interior); borrar mejorandoA; evento construccion.recinto_mejorado
        si no: completadoEn := instante; evento construccion.recinto_completado
```

- La **reserva** es la misma que cualquier obra automática: la obra nunca se come lo que Mantenimiento y la comida van a cobrar. Sin excepción por recurso.
- **`factorTiempoMuralla`**: la política «Postura Defensiva» (Maestro de Obras) lo pone a 0,75 (más rápido). Producto de los factores de las políticas activas.
- Se avanza **una celda por pasada** (el contador `siguienteCeldaEn` espacia el ritmo): 15 / 30 / 60 minutos por celda a empalizada / piedra / adarve.
- El orden de levantamiento es el del anillo: se ve cerrarse desde la puerta principal.

## 6. Mejorar de nivel

`iniciarMejoraDeRecintoManualmente(asentamiento, cargo, recintoId)` (comando `mejorarRecinto`, mismo camino que comprometer):

1. Cargo asignado. 2. **Gate de nivel de asentamiento del nivel destino** (`nivel + 1`): a piedra desde el 3, a adarve desde el 4. 3. `iniciarMejoraDeRecinto`: el recinto existe, está **completo** (`avance ≥ celdas.length − 1`),
no tiene ya una mejora en curso y `nivel < nivelMaximo (3)`. Resultado: `mejorandoA = nivel + 1`, `avance = −1`. Gratis empezarla.

La obra de mejora es **la misma obra** (§5) pagando `tarifaPorCelda[mejorandoA]`: las tarifas de los niveles 2 y 3 son el coste de **mejorar cada celda**, no el acumulado. No se retraza nada: puertas y largo del anillo no
cambian; solo se reclasifican qué celdas de muro pasan a torre. Mientras dura, la integridad es 0 (no defiende, no suma guarnición) y el upkeep sigue siendo el del nivel viejo.

## 7. Abandonar

`abandonarRecinto`: solo el **Gobernador**. Solo un recinto **incompleto**: se borra entero (celdas levantadas incluidas), su suelo queda libre, **sin devolución**. Es una escotilla de seguridad (una obra atascada por
falta de piedra bloquearía su franja para siempre), no una mecánica. Un recinto completo no se destruye.

## 8. Ampliar

Es comprometer otro recinto cuando ya hay uno (§4): el anterior completo, ≥ 6 edificios extramuros (`arrabalMinimo`) y el trazo nuevo envuelve estrictamente al viejo. Los recintos interiores son «historia en piedra»:
**solo el último (el exterior) defiende**.

## 9. Qué hace un recinto

### 9.1 Bono defensivo en el asedio

`multiplicadorDefensivoDeRecintos(recintos)` — solo el **último** recinto; `1` si no hay ninguno:

```
1 + (bonoDefensaPorNivel[nivel] − 1) × integridad / nºPuertas        (nºPuertas = 0 → 1)
```

El «1 +» garantiza que un muro nunca perjudique al defensor; `integridad` hace que un anillo a medio cerrar defienda casi nada; dividir por las puertas es lo que hace real el eje fortaleza↔metrópoli. Con el bono de
nivel 3 (2,5) y el anillo entero: ×2,5 con una puerta, ×1,38 con cuatro, ×1,19 con ocho. **Solo se aplica en el asedio resuelto con números** (`iniciarAsedio`, `asediarConEjercito`: bot contra bot): en mundo abierto no hay recinto que atravesar. Multiplica el poder del defensor junto con `MILITAR.ventajaDefensor`.

**En una batalla de Unity el recinto no es un multiplicador: viaja como geometría.** `session/batallas.ts` entrega a Unity, por cada recinto, `{ recintoId, nivel, celdas }` con las celdas levantadas
(`celdas[0..avance]`), o el anillo entero al nivel de antes si hay una mejora en curso; con `layoutVersion` y `unidadesPorCelda`. Lo que la muralla vale dentro de la partida lo decide Unity.

### 9.2 Cupo de guarnición

`cupoGuarnicion` suma `GUARNICION.recintoCompletoPorNivel[n−1]` (+10 / +15 / +20), con `n` = nivel del **mejor** recinto con integridad ≥ 1 (no se acumulan), y solo si el asentamiento ya tiene cupo por edificios (Barracón, Galería de tiro o Caballerizas; sin ninguno el cupo es 0).
Un recinto en mejora no suma. Progresivo por decisión del autor (2026-10-10).

### 9.3 Requisito del nivel 4

`NIVEL_ASENTAMIENTO.requisitos[4]` exige `recintoCompletoNivelMinimo = 2`: un recinto **completo de piedra o superior** (la empalizada no basta), 1.000 pesants, 800 artesanos y una Sala del Consejo.

### 9.4 Mantenimiento (upkeep)

`upkeepDeRecintos`: por cada recinto con celdas levantadas, `upkeepPorCelda[nivel actual] × (avance + 1)` por recurso y tick. Usa el nivel **actual**, nunca `mejorandoA`. Se **suma** al coste de mantenimiento del asentamiento
(`calcularCostoMantenimiento`) sin el factor de población ni de distancia: el muro cuesta lo que pesan sus celdas.

### 9.5 Colocación de edificios

Los recintos influyen en dónde caen los edificios: muro y torre ocupan suelo, y con un recinto exterior completo los urbanos se colocan primero intramuros ([`Trazado_Urbano.md`](Trazado_Urbano.md) §6.6).

### 9.6 Daño por conquista

`aplicarConquista`: cada recinto **completo** pierde `floor(fraccionDanoMuralla × celdas.length)` de `avance` (mínimo −1). La muralla no cae: deja de dar el multiplicador pleno y el cupo hasta repararse por la obra normal (§5).

## 10. Comandos y permisos

| Comando | Quién | Qué |
|---|---|---|
| `comprometerRecinto { asentamientoId, cargo, nivel ∈ {1,2,3} }` | residente con el cargo `gobernador` o `maestroObras` | §4. Evento `construccion.recinto_comprometido` |
| `mejorarRecinto { asentamientoId, cargo, recintoId }` | ídem | §6. Evento `construccion.recinto_mejora_iniciada` |
| `abandonarRecinto { asentamientoId, recintoId }` | solo el Gobernador | §7. Evento `construccion.recinto_abandonado` |

La auto-construcción **no** levanta murallas; la gobernanza de los bots las añade a mano (`bots/cerebro/gobierno.ts`: comprometer el nivel 1 y mejorar a piedra cuando la subida al nivel 4 está al alcance).
Eventos de la obra: `construccion.recinto_completado` y `construccion.recinto_mejorado`, con `{ recintoId, nivel, celdas, puertas }`.

## 11. Parámetros

<!-- parametros:MURALLA -->
```json
{
  "nivelAsentamientoPorNivelDeRecinto": {
    "1": 2,
    "2": 3,
    "3": 4
  },
  "nivelMaximo": 3,
  "franjaDeRonda": 1,
  "pasoTorres": {
    "2": 8,
    "3": 5
  },
  "arrabalMinimo": 6,
  "minutosPorCelda": {
    "1": 15,
    "2": 30,
    "3": 60
  },
  "factorPuerta": 4,
  "factorTorre": 3,
  "tarifaPorCelda": {
    "1": {
      "madera": 20,
      "piedra": 2
    },
    "2": {
      "madera": 5,
      "piedra": 25
    },
    "3": {
      "madera": 10,
      "piedra": 20
    }
  },
  "upkeepPorCelda": {
    "1": {
      "madera": 0.02
    },
    "2": {
      "piedra": 0.02
    },
    "3": {
      "piedra": 0.04
    }
  },
  "bonoDefensaPorNivel": {
    "1": 1.3,
    "2": 1.8,
    "3": 2.5
  }
}
```
<!-- /parametros -->

<!-- parametros:GUARNICION -->
```json
{
  "cupoPorNivelEdificio": [
    7,
    14,
    22
  ],
  "cupoPorNivelCaballerizas": [
    0,
    14,
    32
  ],
  "recintoCompletoPorNivel": [
    10,
    15,
    20
  ]
}
```
<!-- /parametros -->

<!-- parametros:OCUPACION -->
```json
{
  "proteccionMinutos": 1440,
  "duracionMinutos": 90,
  "fraccionSaqueoPoblacion": 0.25,
  "fraccionEdificiosDanados": 0.25,
  "fraccionCosteReconstruccion": 0.5,
  "fraccionDanoMuralla": 0.3,
  "factorRecaudacion": 0.5,
  "factorCrecimiento": 0.5
}
```
<!-- /parametros -->

Los valores de `tarifaPorCelda`, `upkeepPorCelda` y los ritmos son **placeholder sin calibrar** (Paso 6 del plan de murallas). `fraccionDanoMuralla` está en `OCUPACION`; `ventajaDefensor` en `MILITAR`.

## 12. Invariantes

| Invariante | Dónde se vigila |
|---|---|
| El anillo aísla el Centro Urbano: un flood fill desde fuera, sin cruzar muro/torre, no lo alcanza | `muralla.test.ts` (y validación en el trazo) |
| Cada cruce con la red es **una** puerta | `muralla.test.ts` |
| La red de calles sigue siendo un solo componente conexo con el muro puesto | `muralla.test.ts` |
| Las celdas no levantadas ocupan suelo desde el compromiso | `muralla.test.ts`, `edificiosSobreCalle.test.ts` |
| Una ampliación envuelve estrictamente a la anterior | `muralla.test.ts` |
| Un recinto incompleto no se mejora; uno completo no se abandona | `muralla.test.ts` |
| Cada nivel de recinto respeta su gate de asentamiento (2/3/4) al trazar y al mejorar | `muralla.test.ts` |
| El bono nunca baja de 1 | `muralla.test.ts` |
