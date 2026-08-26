# Triaje de las consultas de `GameStore`

`GameStore` expone **34 comandos y 31 consultas**. El diseño de `GameSession`
([doc 7](7_Diseno_GameSession.md)) resolvió los comandos pero dejó las consultas sin ubicar, que era un hueco
real: moverlas todas a `GameSession` reproduciría el mismo objeto-dios que la separación por comandos evita.

Este documento las clasifica. **Nada de esto está implementado**; es la guía para los pasos 2-4 de la
migración.

> **Cableado al roadmap (2026-08-26).** Esta migración dejó de ser opcional: es el hito **C10** de la Fase C.
> El diagnóstico de aislamiento del cliente
> ([4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md#diagnóstico-de-aislamiento-del-cliente-2026-08-26))
> confirmó que estas consultas son la razón principal por la que `cliente/` todavía necesita importar el
> motor, y la Fase C no cierra hasta que un cliente sin motor pueda jugar. Dos correcciones a lo que dice
> este documento:
>
> - **La cuenta real es 26, no 31**, contando lo que `GameStore` calcula hoy con el motor (incluye `CATALOGOS`
>   y las dos exportaciones). El triaje de abajo sigue siendo válido en su criterio.
> - **Falta una categoría**: el TERRENO. `drawTerreno` evalúa `biomaEn`/`elevacionEn` por píxel sobre
>   parámetros de ruido, no sobre un ráster. No es una consulta de `GameStore`, así que este triaje no la vio
>   — es el hito **C11**.
> - **El criterio "→ `engine/`" necesita un segundo eje** (2026-08-26). Este documento clasifica por *de qué
>   depende* la consulta. Falta *quién puede calcularla sin viaje de red*: las que son **tabla pura** sobre
>   constantes de balance (`capFundacion`, `cupoVivienda`, `slotsPoliticaDisponibles`, `nivelFaccionInfo`,
>   `CATALOGOS`) las resuelve el cliente en cuanto **C7** le sirva el balance — patrón *Static Data Export*,
>   ver [6_Sincronizacion_Visibilidad_y_Escala.md §6.3](6_Sincronizacion_Visibilidad_y_Escala.md#63-la-frontera-real-no-es-motor-sí--motor-no-es-reglas-vs-simulación).
>   Las que son **fórmula sobre estado vivo** (`produccionInfo`, `mantenimientoInfo`, `manoObraInfo`…) las
>   manda el servidor ya calculadas. Ambas caen hoy en el grupo "→ `engine/`" de abajo, y no deberían tener el
>   mismo destino. Desglose en los cuatro grupos: doc 4, § "C10 partido en dos".

## El criterio

Una consulta va a un sitio u otro según **de qué depende y quién la necesita**:

| Destino | Criterio | Por qué |
|---|---|---|
| **`engine/`** | Es un cálculo de dominio: solo depende de entidades de juego y constantes de balance | Es una regla, no una vista. Debe poder probarse y usarse en batch sin capa de partida |
| **`session/`** | Necesita el estado completo de la partida o la fachada `Mapa` | Es de la partida, pero no una regla del mundo |
| **Proyección (Fase C)** | Compone datos para una audiencia concreta y debe filtrarse por visibilidad | Su forma la decide el cliente que la consume, y **debe respetar "último conocido"** (doc 6 §3) |
| **Cliente** | Es estado de presentación o de la sesión de navegador | No tiene nada que hacer en el servidor |
| **Admin** | Solo la consume un administrador | Endpoint protegido por rol técnico (doc 5) |

## Clasificación

### → `engine/` — cálculos de dominio puros (11)

Dependen solo de entidades y constantes. Varias ya delegan en el motor y lo único que hacen es reexportar,
así que mover la firma es casi todo el trabajo.

| Consulta | Nota |
|---|---|
| `capFundacion` | Deriva de `CAP_FUNDACION_POR_NIVEL` |
| `cupoVivienda` | Cálculo sobre edificios del asentamiento |
| `cupoAsentamientosFaccion` | Ya usa `calcularCupoNivel` |
| `nivelFaccionInfo` | Deriva de `NIVEL_FACCION` y la XP |
| `nivelAsentamientoInfo` | Gates de nivel, regla pura |
| `cupoNivelInfo` | Ídem |
| `slotsPoliticaDisponibles` | Deriva de `POLITICAS` |
| `poderMilitarInfo` | Ya usa `poderEscuadron` de `engine/combate.ts` |
| `manoObraInfo` | Reparto de mano de obra, regla de producción |
| `produccionInfo` | Recetas y capacidad; regla de producción |
| `edificioEconomiaInfo` | Coste/rinde de un edificio |

⚠️ `produccionInfo` arrastra un bug abierto en `Notas_revision.md` (`[PRODUCCION]`: el consumo se muestra mal
cuando la producción es 0). Conviene arreglarlo **al moverla**, no antes: en `engine/` queda cubierta por la
suite del motor.

### → `session/` — necesitan el estado de partida o el mapa (8)

| Consulta | Nota |
|---|---|
| `getState` | Acceso al estado; ya existe en `GameSession` |
| `getMapa` | Fachada de consultas espaciales; ya existe |
| `getZonas` | Zonas de influencia de todos los asentamientos |
| `getTrazadoAsentamiento` | Trazado urbano derivado |
| `chokepointsControl` | Necesita zonas + relaciones |
| `getLigas` | Necesita relaciones + facciones |
| `precioReferencia` | Depende de todos los asentamientos (mercado global) |
| `viabilidadFundacion` | Necesita la fachada `Mapa` |

Estas son candidatas a **caché derivada** en el runner: `getZonas` reaparece dentro del tick
(`computeTodasLasZonas`) y el perfilado la midió en ~2% del coste. No urge, pero conviene no recalcularla dos
veces por tick cuando exista el runner.

### → Proyección por audiencia (Fase C) — filtradas por visibilidad (5)

Son las que **no pueden servirse tal cual a un jugador**: hoy devuelven todo, y deben pasar por el modelo de
visibilidad de [doc 6](6_Sincronizacion_Visibilidad_y_Escala.md) §3 (espacial + contacto + alianza, mostrando
**último conocido** para lo ajeno).

| Consulta | Riesgo si se sirve sin filtrar |
|---|---|
| `mantenimientoInfo` | Estado económico interno de un asentamiento ajeno |
| `poblacionInfo` | Ídem |
| `caravanasInfo` | Revela rutas y cargas en vuelo de rivales |
| `jugadoresDeAsentamiento` | Revela composición de facciones rivales |
| `infoMejoraEdificio` | Revela planes de construcción |

### → Cliente — presentación pura (4)

No pertenecen al servidor en ninguna forma. Se quedan donde están cuando `GameStore` pase a ser cliente.

`subscribe`, `getSnapshot`, `getTickRange`, `esFaccionNpc` — las tres primeras sostienen la línea de tiempo de
depuración (que `GameSession` deliberadamente no tiene, doc 7 §1); la última es una lectura trivial del
estado que el cliente ya recibe.

### → Administración — rol técnico (3)

`exportarSimulacion`, `exportarMapaUnity`, `getBalance`. Endpoints protegidos (doc 5). `exportarMapaUnity`
además **no es parte del juego**: es una herramienta de `worldgen/` y no debería colgar de la partida.

## Orden sugerido

1. **`engine/` primero** (11 consultas). Sin dependencias de nada nuevo, la suite del motor las cubre al
   llegar, y descargan `GameStore` antes de tocarlo.
2. **`session/` después** (8), al convertir `GameStore` en adaptador (paso 2 del doc 7 §6).
3. **Proyecciones al final** (5), en Fase C junto a `ConocimientoJugador` — son las únicas que dependen de una
   decisión de diseño todavía sin implementar.
4. Las de **cliente** no se tocan; las de **admin**, al montar los endpoints por rol.

## Consecuencia para el tamaño de `GameStore`

De las 31, **19 salen** de la capa de aplicación de navegador (11 a `engine/`, 8 a `session/`) y 5 se
transforman en proyecciones. Junto con los 34 comandos, eso es lo que reduce `GameStore` de 1582 líneas al
cliente delgado que el paso 4 del plan describe.
