# Mechanics — cómo funciona lo que ya está implementado

Esta carpeta describe, **con la precisión suficiente para reimplementarlo en otro lenguaje**, el comportamiento que hoy tiene el juego. Nace el
2026-10-09 por decisión del autor: las reglas de trazado y de murallas vivían solo en el código y en documentos de diseño narrados como historia,
y una migración habría perdido lo que el código sabe.

## Qué es cada carpeta de documentación

| Carpeta | Responde a | Ejemplo |
|---|---|---|
| `Docs/Game/` (canon) | **Qué reglas rigen y por qué**, en el lenguaje del juego | «la empalizada se abre en el nivel 2» |
| `Docs/Mechanics/` (esta) | **Cómo se calcula exactamente**: datos, algoritmos, orden, fórmulas, casos límite | «el anillo es la capa de vecindad 4 exterior a la región dilatada» |
| `Docs/Coordinacion/` | Contrato con los clientes: comandos, eventos, proyecciones, modelo de datos compartido | `GET /jugador/partidas` |
| `Docs/Arquitectura/` | Cómo está organizado el servidor | tick, sesiones, persistencia |
| `Consideraciones/` | **Por qué se decidió así**: decisiones, planes, mediciones, historia. Se consulta, no se implementa desde ahí | `Murallas_Definicion.md` |

Si el canon y `Mechanics` se contradicen, gana el canon (es la intención) y `Mechanics` está desactualizado **o** el código no cumple el canon: en los dos
casos se anota en [`Divergencias.md`](Divergencias.md) hasta resolverlo.

## Regla de trabajo (decidida el 2026-10-09)

> **Todo cambio de comportamiento actualiza su documento de `Mechanics` en el mismo commit.**

- Las cifras no se escriben a mano: viven en bloques `<!-- parametros:NOMBRE -->` que se copian de `src/constants.ts`.
  `npm run mecanicas` los reescribe y un test (`src/mecanicas/__tests__/sincronizar.test.ts`) falla si alguno quedó viejo.
- Lo que **no** es una cifra (un algoritmo, un orden, una excepción) lo tiene que tocar quien cambia el código; no hay test que lo detecte.
- Lo que queda obsoleto en `Consideraciones/` se **borra** cuando su contenido pasa aquí (decisión del autor): no se mantienen dos versiones.

## Estructura de cada documento

1. **Alcance y fuentes** — qué cubre, qué módulos, y el commit contra el que se verificó.
2. **Modelo de datos** — entidades y campos, con sus invariantes.
3. **Algoritmos** — paso a paso, en el orden en que corren, con pseudocódigo donde el texto no baste.
4. **Parámetros** — bloques sincronizados con `constants.ts`.
5. **Invariantes** — lo que siempre debe ser cierto (y qué test lo vigila).
6. **Casos límite y decisiones que parecen raras** — con el porqué.
7. **Divergencias** — qué difiere del canon o de lo que dice el diseño.

## Qué NO está aquí (todavía)

- **El generador de números aleatorios y su semilla** (decisión del autor, 2026-10-09: anotado, fuera de alcance). Los sistemas que lo usan lo indican, pero para
  una migración **bit-idéntica** de las simulaciones habrá que especificarlo aparte (`createRng`/`restaurarRng`, `worldgen`).
- Los textos de presentación y la interfaz: son de los clientes.
- **Las Eras IV y V** (tecnologías, tarifas de adopción, el techo de nivel 5 y la Maravilla): el canon las define a medias y el código solo llega a la III. Se documentarán cuando se implementen.

## Índice

| Documento | Estado | Cubre |
|---|---|---|
| [`Trazado_Urbano.md`](Trazado_Urbano.md) | ✅ 2026-10-09 | rejilla, huellas, red de calles, colocación de edificios, anclas, perfiles |
| [`Murallas.md`](Murallas.md) | ✅ 2026-10-09 | recintos: trazo, puertas y torres, obra, mejora, ampliación, defensa, mantenimiento, daño |
| [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) | ✅ 2026-10-10 | salir, rutas y velocidad, víveres y hambre, perseguir, repostar, caravanas adjuntas, unirse/separarse/formar, volver a casa, inspección |
| [`Combate.md`](Combate.md) | ✅ 2026-10-10 | `atacar`, resolución numérica, encuentros, caravanas, bandidos, asedio, conquista, desalojo, heridos |
| [`Economia_Construccion.md`](Economia_Construccion.md) | ✅ 2026-10-10 | fundación y zona, almacén, producción y recetas, cola de obras y auto-construcción, mejoras, población y hambre, mantenimiento, nivel de asentamiento y su ascenso, políticas económicas |
| [`Tick.md`](Tick.md) | ✅ 2026-10-11 | reloj de mundo, transacción de un comando, `avanzarTick`, orden y fotos de la cadena `avanzarSimulacion`, consumo del generador aleatorio, salidas del mundo, lo que `exito` estampa |
| [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) | ✅ 2026-10-11 | rejilla de lo explorado, ojos y alcance, memoria de la Facción, exploración sin bandera, visión compartida, regla de cada cosa en la proyección, Miradas e Informes |
| [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md) | ✅ 2026-10-11 | logros del servidor y sus contadores, Eras, aparición por hito, adopción, campamentos de mercenarios, Aedas itinerantes (revelar y vender), residentes y épicas, crónica |
| [`Comercio_Caravanas.md`](Comercio_Caravanas.md) | ✅ 2026-10-11 | precio de referencia y comisiones, trueque (propuesta, reparto automático, entrega, vencimiento), flota de caravanas (cupos, carros, animales, escolta, lanzamiento y programación, aparcada), viaje, rutas (grafo, A*, peajes, red de caminos), mercado de plaza (órdenes y mostrador), mercado de mercenarios, ruinas |
| [`Divergencias.md`](Divergencias.md) | ✅ vivo | lo que el código y el canon dicen distinto |

Pendientes, por orden de riesgo si se perdieran: diplomacia y Facciones · generación del mundo.
