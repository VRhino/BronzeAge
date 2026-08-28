# Definición de Fase 0.1 — Evolución del generador de mundo

Sub-fase dentro de Fase 0 (sigue siendo datos puros, mapa 2D top-down, sin Unity ni 3D — ver
`Fase_0_Definicion.md`). Amplía el generador de mundo (`src/worldgen/`) de un mapa plano (fertilidad como
único campo continuo, nodos por rejection-sampling uniforme) a uno con relieve, ríos, biomas y colocación
de recursos condicionada al terreno (ej. metales en montaña).

Justificación de traer esto a Fase 0 pese a que `Fase_0_Definicion.md` lista "relieve de terreno
(montañas/ríos/mar)" como fuera de alcance: la razón NO es adelantar Fase 1 por gusto, sino que el dato de
mundo (`MapaGenerado`) es la pieza que menos conviene rehacer más adelante — decidir su forma ahora, con
Fase 1 ya en mente, evita reconstruirlo cuando llegue.

## Objetivo final al que apunta (contexto, no alcance de 0.1)

`Roadmap_Escalado.md` (Fase 1, eje 1) ya define el destino: mapa de campaña estilo Total War, con
movimiento continuo. Pero **sin el grafo de provincias de TW** — el proyecto ya tiene su propio mecanismo
de territorio (zonas de influencia por asentamiento, polígono recortado contra fronteras vecinas), que no
se toca ni se duplica.

## Decisión de arquitectura (la que gobierna toda la implementación)

**Todo como campos paramétricos continuos + geometría vectorial (puntos/polilíneas). Nunca como rejilla
horneada ni tiles.**

Motivo: es la construcción de menor fricción hacia el destino final.
- Un campo de elevación como función pura `evaluarElevacion(campo, punto) → altura` (mismo patrón que
  `evaluarFertilidad` ya usa) ES un heightmap — muestreable a cualquier resolución. El día que exista un
  motor 3D, la malla de terreno se genera muestreando esa misma función a la resolución de vértices que
  pida el motor, sin tocar la generación.
- Si en cambio se hornea ahora una rejilla fija dentro de `MapaGenerado` (el objeto que se guarda/deriva de
  la seed), el mundo queda atado a esa resolución — el error concreto a evitar.
- Ríos como polilínea de puntos (no celdas marcadas "es río"): es directamente una spline reutilizable en
  3D.
- Chokepoints como geometría (puntos/zonas, igual que ya son bosques y nodos), no como aristas de un grafo.
- Coste de movimiento (cuando llegue movimiento militar) como función continua de la pendiente, no lookup
  por celda — generaliza directo de cómo caravanas ya interpolan (`progreso` 0→1) sobre una línea.
- Ruido en forma cerrada (octavas de seno/coseno, como ya usa fertilidad), no una librería de noise
  externa: portable a un shader/nodo de noise del motor 3D sin traducción.

Tiles quedan descartados: chocan con el modelo espacial entero del proyecto (`Point` continuo, zonas de
influencia como polígonos, colocación en coordenadas flotantes) y son un paso atrás respecto al destino 3D
continuo, no un paso intermedio útil.

## Alcance de 0.1

1. **Elevación** — campo continuo (multi-octava), clasificación por umbral en categorías de terreno
   (agua/costa/llano/colina/montaña).
2. **Ríos** — nacen en puntos altos, descienden por gradiente de máxima pendiente hasta agua/borde del mapa
   o quedan atrapados en mínimo local (lago). Dato: polilínea de puntos.
3. **Biomas** — elevación + fertilidad existente + humedad (campo nuevo, o proxy = distancia a río)
   clasificados por umbral (4-6 categorías, mismo nivel de granularidad que `Rareza`).
4. **Colocación de nodos condicionada al terreno** — metales con mayor densidad en colina/montaña,
   livestock en llanura fértil, bosques condicionados a humedad/elevación media-baja. Sigue siendo
   rejection-sampling, pero el candidato se filtra también por bioma/elevación, no solo por espaciado.
5. `Mapa.terrenoEn(p)` (`src/world/mapa.ts`) deja de ser el stub `'llano'` y devuelve el tipo real — el
   enchufe ya estaba preparado para esto.
6. Render 2D (`src/ui/canvas.ts`): capa de terreno/elevación por color + trazo de ríos, muestreados en
   rejilla SOLO para pintar (igual que ya hace el filtro de fertilidad) — nunca guardados como parte del
   dato de mundo.

## Fuera de alcance de 0.1

- Coste de movimiento / pathfinding sobre el terreno (es Fase 1, movimiento militar) — 0.1 solo deja los
  datos listos para que esa función se añada sin rehacer el generador.
- Cualquier cosa de mar/naval (eje 2 del roadmap sigue en fase inicial, sin mar).
- Grafo de provincias — descartado permanentemente para este proyecto, no solo pospuesto.

## Contratos que se mantienen sin excepción

- Determinista por seed (mulberry32) — mismo seed, mismo mundo.
- Orden de consumo del PRNG es parte del contrato — cualquier llamada nueva se añade en un punto explícito
  del pipeline y se documenta.
- `MapaGenerado` sigue siendo datos puros, serializable, sin funciones cerradas.
- Cambiar el pipeline sube `WORLDGEN_VERSION` — se acepta que todas las seeds/mundos guardados cambian.
- Sin dependencias externas de generación (ruido en forma cerrada, como el resto de `worldgen/`).
