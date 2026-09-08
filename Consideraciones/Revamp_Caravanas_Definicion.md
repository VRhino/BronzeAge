# Revamp de caravanas — decisiones y plan

> **Estado (2026-09-08): diseñado, sin implementar.** Cuatro rondas de decisiones con el usuario. Reglas de
> juego → `Docs/Game/3` §3.13 (y toques en §3.6, §3.10, Doc 4.2.1 Mercado, Doc 5.13.3). Aquí solo decisiones,
> representación en el motor, plan e invariantes.
>
> Sale de `Docs/Mecanicas a desarrollar.md` §8. Difiere trozos a fichas/pases posteriores (§7 de este
> documento).

## 0. El punto de partida

Hoy una caravana `comercial` (Doc 3.12, implementado) es un **activo único** con capacidad y velocidad fijas
por catálogo (`CARAVANA_CATALOGO.comercial = 500 / 16`). Se construye por 50 madera contra el `cupoCaravanas`
del Mercado, nace `'disponible'`, y `asignarCaravanasATrueque` la reparte por scoring a los lados pendientes de
un trueque. El comercio NPC y el laboratorio dependen enteramente de ese reparto automático.

El enunciado de §8 pide que una caravana se **arme con piezas** —carros, animales de arrastre, escuadrones de
escolta sin héroe— y que haya una vía de preparación manual además del reparto automático.

## 1. Decisiones cerradas con el usuario (2026-09-08)

### Ronda 1

1. **Una caravana `comercial` pasa a ser un contenedor de tres partes**: carros, animales, escolta. Capacidad y
   velocidad se **derivan**, no se guardan.
2. **Cada carro lleva como mucho UN animal** (Ronda 2 lo cerró: exactamente uno, no varios). Un carro es en la
   práctica un par `{tipoCarro, animal?}`.
3. **Escolta sin héroe = escuadrones cedidos por un jugador**, no una figura de "tropa del asentamiento" nueva.
4. **Modelo nivel-caravana**, no árbol por carro: la caravana tiene una lista plana de carros.
5. **Se difieren**: cría de animales (solo compra con oro), planificación horaria, visibilidad por tamaño,
   fusión de caminos (eso es §3, no §8).

### Ronda 2

6. **Escolta: cupo por nivel interno de Mercado** (placeholder 1/2/3). Si la caravana es capturada, los
   escuadrones **vuelven a casa derrotados** (con debuff de derrota); se pierden carga y carros, no la tropa.
7. **La escolta no consume ración** — abstracción. No es una campaña (a diferencia de Doc 5.13).
8. **Baseline de calibración**: `1 carro básico + 1 buey ≈ 500 / 16`. Todo lo demás se calibra alrededor de esa
   ancla; el batch NPC no debe moverse.
9. **La caravana es persistente y se reconfigura** (como hoy). El pool de carros/animales **vive dentro de
   caravanas**, no suelto: se construye un carro y se compra su animal *sobre* una caravana concreta.

### Ronda 3

10. **Caravana capturada: carros y animales destruidos.** El atacante se queda solo con la carga, en su carro,
    como hoy. Sin transferencia de activos entre Facciones.
11. **Estado `'preparando'` en el origen: bloquea todo, cancelable con devolución total** mientras no haya
    salido. Igual que quitar una obra `'en_cola'` (Doc 4.2).
12. **Escolta por viaje**: se cede al lanzar, se recupera cuando la caravana vuelve. No es un enganche
    permanente.
13. **Carro de Carpintería: por ahora solo uno, "reforzado", que da más capacidad.** El catálogo de carros se
    amplía más adelante (otros ejes: resistencia a captura, penalización de velocidad).

### Ronda 4

14. **Inmunidad del camello al desierto: diferida.** No existe bioma `desierto` de primera clase (el tipo es
    `agua|costa|estepa|llanuraFertil|colina|montana|cima`; la aridez del Nilo es `estepa` de fertilidad baja).
    En este pase el camello es solo "opción media: entre buey y caballo en velocidad y carga, más barato que
    el caballo". La inmunidad se diseña cuando exista un bioma árido real.
15. **Auto vs manual: flag de reserva explícito.** `reservadaManual = true` saca la caravana del reparto
    automático sea cual sea su tamaño. No se infiere de la composición.

### Ronda 5 (implementando el Paso 2)

16. **`crearCaravana` crea un casco VACÍO, gratis.** El coste está en las piezas: carro básico 20 madera
    (Mercado), carro reforzado 40 madera (Carpintería), y los animales aparte.
17. **El buey se paga en MADERA (30), no en oro.** Mismo criterio que quitarle la piedra al Mercado (Doc
    4.2.1): "todo animal cuesta oro" —enunciado original— revive el deadlock "sin caravana no hay comercio,
    sin comercio no hay oro, sin oro no hay caravana" para cualquier asentamiento sin mina. El buey barato en
    madera es la vía de entrada; caballo (60 oro) y camello (40 oro) son la mejora. `basico` + `buey` =
    50 madera, el mismo coste que crear una caravana antes → el batch NPC no se mueve.
18. **El bootstrap del Mercado (caravana #0 gratis) se DESCARTA.** Medido: una sola caravana gratis por
    asentamiento al completarse el Mercado mueve 42 métricas del batch (seed 7, 40 Facciones, 400 ticks) —
    `oroMedio` 155→122, artesanos y edificios de transformación a **cero**, `nivelFaccionMax` 8→7. Inunda el
    mapa de caravanas de trueque activas y los asentamientos exportan lo que necesitan para subir de nivel. El
    valor que aportaba era marginal (evitar un `crearCaravana` tras el Mercado, y ni siquiera hay espera: el
    casco vacío es gratis). Se cae. El enunciado ("el Mercado te da un carro y un animal") queda como sabor,
    no como mecánica.
19. **Se elimina el modelo viejo y su fallback** (a petición del usuario: en fase de desarrollo no se dejan
    versiones incompletas anteriores en el código). Fuera `CARAVANA_CATALOGO.comercial` (capacidad/velocidad/
    `costoConstruccion` fijos) y fuera la rama `if (carros === undefined) → catálogo` de
    `capacidadCaravana`/`velocidadCaravana`. La migración v11→v12 ya garantiza que toda comercial tenga
    `carros`; el NPC reserva vía `costoCaravanaPorDefecto()`. Batch bit-idéntico.

## 2. Lo que ya existe y no hay que inventar

- **El ciclo de estados de la caravana** ya es una máquina: `disponible | adjunta | en_transito | retornando`
  (`Caravana.estado`). Añadir `preparando` es un estado más, no un sistema nuevo.
- **La vuelta a casa desandando la ruta** (`retornando`, `avanzarCaravanas` en `engine/trade.ts`) ya está — el
  revamp no toca el "nunca hay teletransporte".
- **Ceder escuadrones y sacarlos de la guarnición** ya tiene precedente exacto: `movilizarEjercito` mueve
  escuadrones de `Asentamiento.escuadrones` a `Ejercito.escuadrones` y por eso la guarnición deja de contar
  con ellos (Doc 5.12.4). La escolta hace lo mismo con destino una caravana en vez de un ejército.
- **La resolución de combate asimétrico** (atacante vs defensa) ya sirve para tres casos: intercepción entre
  Facciones (`interceptarCaravanaConEjercito`), bandidos (`engine/bandidos.ts`) y la defensa base fija
  (`defensaBaseCaravana`, Doc 3.10). La escolta añade un cuarto: atacante vs poder de los escuadrones-escolta.
- **El cupo de flota, el cooldown y el scoring de asignación** (Doc 3.12) no cambian.
- **Las políticas del Tesorero** `carga_ampliada` (×1.5 capacidad) y `rutas_rapidas` (×1.5 velocidad) siguen
  aplicando — como multiplicador FINAL sobre la capacidad/velocidad ya derivadas.

## 3. Representación en el motor

### Cambios en `Caravana` (`src/domain/types.ts`)

Solo para `tipo: 'comercial'`. Las otras tres categorías (militar/construccion/contrabando) no cambian y
siguen leyendo `CARAVANA_CATALOGO`.

```ts
type CarroTipo = 'basico' | 'reforzado';
type AnimalTipo = 'buey' | 'caballo' | 'camello';

interface Caravana {
  // ... campos actuales ...
  /** Revamp (Doc 3.13). Una caravana comercial deriva capacidad/velocidad de aquí. La migración de snapshot
   *  v11→v12 la puso en TODAS las comerciales guardadas (1 carro básico + 1 buey), así que a partir de v12
   *  siempre está presente en una comercial — sin `carros` con tracción, capacidad y velocidad son 0. */
  carros?: { tipoCarro: CarroTipo; animal?: AnimalTipo }[];
  /** Escuadrones cedidos como escolta sin héroe (Doc 3.13.4). Solo presente en viaje (estado ≠ 'disponible').
   *  Los ids apuntan a Escuadron de la guarnición del origen; mientras están aquí, esa guarnición no los
   *  cuenta como defensores. */
  escoltaEscuadronIds?: string[];
  /** Fuera del reparto automático (asignarCaravanasATrueque) cuando true. Decisión explícita del jugador. */
  reservadaManual?: boolean;
  /** Instante en que termina la preparación, presente solo en estado 'preparando'. */
  preparaHasta?: Instante;
  estado?: 'disponible' | 'preparando' | 'adjunta' | 'en_transito' | 'retornando';
}
```

### Derivaciones (no se guardan)

- `capacidadCaravana(c)` = Σ (`CARRO_CATALOGO[carro.tipoCarro].capacidadBase` × `ANIMAL_CATALOGO[carro.animal].factorCarga`) sobre los carros con animal. **Sin fallback a un catálogo fijo** — sin carros con tracción da 0. El factor de política `carga_ampliada` lo aplica el llamador.
- `velocidadCaravana(c)` = min de `ANIMAL_CATALOGO[animal].velocidad` sobre los carros con animal; 0 sin animales (no puede salir). El factor `rutas_rapidas` lo aplica el llamador.
- `costoCaravanaPorDefecto()` = suma por recurso de `CARRO_CATALOGO.basico.costo` + `ANIMAL_CATALOGO.buey.costo` (hoy 50 madera) — lo que reserva el NPC de laboratorio antes de montar una caravana.
- `prepTicks(c)` = `CARAVANA_PREPARACION.kPorCarro × max(0, nº carros − 1)`.
- `poderDefensaCaravana(c, ctx)` = si `escoltaEscuadronIds?.length` → Σ `poderEscuadron` de esos escuadrones; si adjunta a ejército → poder del ejército (ya existe, 5.13.3); si no → `defensaBaseCaravana` (ya existe).
- `cupoEscolta(mercado)` = `CARAVANA_ESCOLTA.cupoPorNivelMercado[nivelInterno − 1]`.

### Constantes nuevas (`src/constants.ts`) — todas placeholder

```ts
export const CARRO_CATALOGO = {
  basico:    { capacidadBase: 500, costo: { madera: 20 }, fabrica: 'mercado' },
  reforzado: { capacidadBase: 800, costo: { madera: 40, /* placeholder */ }, fabrica: 'carpinteria' },
} as const;

export const ANIMAL_CATALOGO = {
  buey:    { factorCarga: 1.0,  velocidad: 16, costo: { madera: 30 } }, // madera, no oro — ver Ronda 5 §17
  caballo: { factorCarga: 0.5,  velocidad: 24, costo: { oro: 60 } },
  camello: { factorCarga: 0.75, velocidad: 19, costo: { oro: 40 } },
} as const;

// Pendientes de los Pasos 3/4:
// export const CARAVANA_PREPARACION = { kPorCarro: 2 };
// export const CARAVANA_ESCOLTA = { cupoPorNivelMercado: [1, 2, 3] };
```

**`CARAVANA_CATALOGO.comercial` se ELIMINA** (Ronda 5 §19): no queremos dejar el modelo viejo de capacidad/
velocidad fijas conviviendo con el nuevo. La migración v11→v12 garantiza que toda comercial guardada tenga
`carros`, así que no hay a quién servir de fallback. `CARAVANA_CATALOGO` se queda solo con `militar` /
`construccion` (Fundación, Doc 1.8) / `contrabando`, que no son este revamp.

### Comandos nuevos (`src/session/comandos/`)

| Comando | Qué hace |
|---|---|
| `crearCaravana(asentamientoId)` | **HECHO (Paso 2).** Casco vacío, gratis, cuenta cupo + cooldown (`crearCaravanaVacia`, engine/trade.ts). |
| `agregarCarroCaravana(caravanaId, tipoCarro)` | **HECHO (Paso 2).** Paga `CARRO_CATALOGO[tipo].costo`, añade un carro sin animal. `reforzado` exige Carpintería activa. Solo caravana `'disponible'`. |
| `comprarAnimalCaravana(caravanaId, carroIndice, tipoAnimal)` | **HECHO (Paso 2).** Paga `ANIMAL_CATALOGO[tipo].costo`, engancha el animal a un carro sin tracción. |
| `reservarCaravana(caravanaId, reservada)` | **HECHO (Paso 2).** Set `reservadaManual`. |
| `moverPieza(desdeId, haciaId, pieza)` | Diferido a Paso 3 (no hay caller hasta la UI de preparación). Reasigna un carro entre dos caravanas propias `'disponible'` en el mismo asentamiento. Sin coste. |
| `prepararCaravana(caravanaId, { carga, destinoAsentamientoId, escoltaEscuadronIds })` | Paso 3. Reserva carga del almacén, valida cupo de escolta y residencia del cedente, pasa a `'preparando'` con `preparaHasta`. |
| `cancelarPreparacion(caravanaId)` | Paso 3. Solo en `'preparando'`. Devuelve carga, libera escolta, vuelve a `'disponible'`. |

### Motor

- `asignarCaravanasATrueque` (`engine/trade.ts`): filtrar `!c.reservadaManual`.
- `avanzarCaravanas` (`engine/trade.ts`): al llegar `preparaHasta`, `preparando → en_transito` con la ruta ya calculada.
- `interceptarCaravanaConEjercito` (`engine/combate.ts`) y `engine/bandidos.ts`: usar `poderDefensaCaravana`; en captura de una caravana con escolta, devolver los escuadrones a la guarnición del origen con debuff de derrota, aplicar permadeath de bajas, destruir `carros`.
- Guarnición defensora (`engine/combate.ts`, asedio): excluir los escuadrones cuyos ids están en `escoltaEscuadronIds` de alguna caravana de ese asentamiento no `'disponible'`.
- ~~Bootstrap del Mercado~~: descartado (Ronda 5 §18) — regresaba el batch NPC. El Mercado no cambia de coste.

### Migración de snapshot

`vN → vN+1`: toda `Caravana` con `tipo === 'comercial'` y sin `carros` recibe
`carros: [{ tipoCarro: 'basico', animal: 'buey' }]`, `reservadaManual: false`. Estado y ruta se conservan. Las
de Fundación (`tipo: 'construccion'`) y las efímeras no se tocan.

### Separación de capas — se respeta la que ya hay (`CAPAS_PERMITIDAS`, `src/__tests__/arquitectura.test.ts`)

Nada de este revamp cruza una frontera nueva. Cada pieza cae en su capa:

| Capa | Qué le toca | Reglas que sigue valiendo |
|---|---|---|
| **`domain`** | Campos nuevos de `Caravana`, tipos `CarroTipo` / `AnimalTipo`, estado `'preparando'` | Sin dependencias. Solo datos. |
| **`constants`** | `CARRO_CATALOGO`, `ANIMAL_CATALOGO`, `CARAVANA_PREPARACION`, `CARAVANA_ESCOLTA` | Solo ve `domain`. |
| **`engine`** | `capacidadCaravana` / `velocidadCaravana` / `prepTicks` / `poderDefensaCaravana`; el filtro `!reservadaManual` en `asignarCaravanasATrueque`; el countdown `preparando → en_transito` en `avanzarCaravanas`; la resolución de escolta en `interceptarCaravanaConEjercito` / `bandidos.ts`; el bootstrap del Mercado en `construction.ts` | **Puro y sin reloj**: `preparaHasta` es un `Instante`; `avanzarCaravanas` lo compara contra `ctx.instante`, nunca contra `Date.now()`. `prepTicks` se convierte con `minutos()` (`domain/tiempo.ts`) y `sumar(instante, …)` al lanzar. El motor **no sabe de jugadores ni de sesiones**: recibe `escoltaEscuadronIds` ya validados y solo mueve los escuadrones. |
| **`session`** | Los comandos nuevos (`construirCarro`, `comprarAnimal`, `prepararCaravana`, `cancelarPreparacion`, `moverPieza`, `reservarCaravana`, `fabricarCarroReforzado`); la autorización de la cesión de escolta (¿el actor es residente del origen? ¿son suyos esos escuadrones?) en `comandos/autorizacion.ts`, reusando `comandaEscuadrones` | **Comando como función pura, síncrona, sin E/S** (Doc 7). La residencia y la propiedad de los escuadrones se comprueban aquí, no en el motor. |
| **`server`** | La migración de snapshot en `persistenciaPartida.ts` (`migrarSnapshot`, `FORMATO_SNAPSHOT_VERSION`) | Único sitio con disco. La planificación horaria (diferida) aterrizaría aquí, en `RunnerDePartida` — no en este pase. |

El test de fronteras (`arquitectura.test.ts`) es el guardián: si un paso mete un `import` de `session` en
`engine`, falla en rojo antes de nada.

## 4. La tensión de verdad: el batch

Quitarle a la caravana su capacidad/velocidad fijas es tocar el número que mueve toda la economía NPC del
laboratorio. La defensa es doble:

1. **La caravana por defecto (1 carro básico + 1 buey) reproduce 500 / 16 exactos.** El NPC nunca compone
   nada más, así que en batch toda caravana es esa. Debe medir **idéntico** al commit anterior — mismo oro
   medio, mismos acuerdos cumplidos, mismos vivos.
2. **`prepTicks(1 carro) = 0`.** Sin esto, meter un retardo de preparación en cada envío de trueque frenaría
   el comercio NPC de forma medible. Con 1 carro sale al instante, como hoy.

El control de la medición es "acuerdos cumplidos" y "oro medio": si el revamp del modelo los mueve con el NPC
sin componer nada, hay un bug en la derivación.

## 5. Plan

Cada paso respeta la separación motor / sesión / infra de arriba y verifica en el navegador de punta a punta
(no basta boot+render — memoria `feedback_verificacion_end_to_end`).

1. ~~**Modelo + migración.**~~ **HECHO (commit `8ca635d`, limpieza en un commit posterior).** Campos nuevos
   en `Caravana`, `CarroTipo`/`AnimalTipo`, `CARRO_CATALOGO`/`ANIMAL_CATALOGO`, `capacidadCaravana`/
   `velocidadCaravana` (engine/caravanas.ts). Estado `preparando` en el tipo (sin usar aún).
   `asignarCaravanasATrueque` filtra `reservadaManual`. Snapshot v11→v12. `CARAVANA_CATALOGO.comercial` y el
   fallback al catálogo se eliminaron (§19). **Batch bit-idéntico.**
2. ~~**Piezas.**~~ **HECHO.** `crearCaravanaVacia` + `agregarCarroACaravana` + `comprarAnimalParaCaravana`
   (engine/trade.ts) y sus comandos `crearCaravana` / `agregarCarroCaravana` / `comprarAnimalCaravana` /
   `reservarCaravana`. `construirCaravanaComercial` recompone la caravana por defecto para el NPC (50 madera,
   sin cambio). Bootstrap descartado (§18). `moverPieza` diferido a Paso 3. **Batch bit-idéntico.**
3. **Preparación.** Estado `preparando` operativo: `prepararCaravana` (lanzamiento manual con carga/destino),
   `cancelarPreparacion`, countdown en `avanzarCaravanas`. `moverPieza` cae aquí. `bandidos.ts` trata
   `preparando` como "en el origen".
4. **Escolta sin héroe.** Cesión y recuperación de escuadrones, exclusión de guarnición, cupo por nivel de
   Mercado, `poderDefensaCaravana` en intercepción y bandidos, vuelta a casa con debuff en captura.
5. **Canon + medición.** Reconciliar Doc 3.13 / 4.2.1 / 5.13.3 con lo medido; batch de control.

## 6. Invariantes

- Una caravana `comercial` siempre tiene **≥1 carro**. La #0 nace con uno; no se puede quitar el último.
- **Solo los carros con animal** cuentan capacidad y viajan. Un carro sin animal se queda en el origen.
- `escoltaEscuadronIds` no vacío ⇒ `estado !== 'disponible'`. La escolta solo existe en viaje.
- Un escuadrón en `escoltaEscuadronIds` **no** cuenta en la guarnición defensora de su asentamiento.
- Ceder tropa a una escolta **no libera Liderazgo** del jugador cedente.
- `reservadaManual` ⇒ invisible a `asignarCaravanasATrueque`.
- `prepTicks` de una caravana de 1 carro es **0** — el timing del batch no cambia.
- La captura **destruye** carros y animales; **nunca** los transfiere a otra Facción.
- La caravana **nunca se teletransporta**: `retornando` desanda la ruta (regla heredada de 3.12).
- **El motor no importa `session` ni lee el reloj.** La validación de residencia y propiedad de escuadrones
  vive en `session`; el motor compara `preparaHasta` contra `ctx.instante`. `arquitectura.test.ts` lo vigila.

## 7. Puntos abiertos

- **Todas las cifras son placeholder** sin calibrar por simulación: `kPorCarro`, `factorCarga`/`velocidad`/
  `costoOro` de cada animal, `capacidadBase` de cada carro, `cupoPorNivelMercado`.
- **¿El camello se gana el sitio sin su inmunidad al desierto?** Riesgo de que sea un "buey peor" hasta que
  exista el bioma árido. Vigilar en calibración; si no aporta nada, recortarlo del catálogo hasta entonces.
- **Experiencia de Facción por defensa de caravana.** Hoy `NIVEL_FACCION.xp.defensaCaravana` no se multiplica
  por jugadores participantes porque "la escolta no tiene escuadrones/jugadores reales" (comentario en
  `constants.ts`). Con escolta real de escuadrones, decidir si pasa a multiplicarse como el resto de combate.
- **Visibilidad por tamaño y planificación horaria** (diferidas) condicionan la UI de preparación: construirla
  sin cerrarles la puerta.
- **Unificación con `Ejercito.suministro`** (Doc 5.13, nota de 5.13.3): sigue diferida; el carro de columna y
  el carro de caravana son el mismo concepto físico y algún día se unifican.
