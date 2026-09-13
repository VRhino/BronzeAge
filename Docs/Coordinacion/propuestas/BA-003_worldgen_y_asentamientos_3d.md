# BA-003 — Frontera de worldgen y asentamientos para Unity 3D

**Destino:** `BronzeAgeFase0`  
**Estado:** ACEPTADO (revisión 2026-09-11 — usuario confirmó la recomendación híbrida de worldgen; matices de
Codex incorporados, ver abajo)  
**Prioridad:** Media, posterior al modelo compartido

## Problema

Unity reemplazará al cliente Vite y representará en 3D tanto el mundo como los asentamientos. Duplicar las
fórmulas TypeScript en C# sin contrato verificable crearía una fuente de verdad adicional. Además, las ciudades
3D serán escenarios reales de asedio, por lo que su trazado no puede separarse del estado persistente.

## Responsabilidad propuesta

BronzeAge conserva coordenadas, topología estratégica, regiones, caminos, propiedad, grilla de asentamiento,
huellas de edificios, variantes persistidas, murallas y elementos con efecto en reglas.

Conquest conserva terreno/mallas, modelos, materiales, vegetación, LOD, navegación, streaming y presentación
de variantes que no alteren la huella lógica.

Cada tipo/nivel de edificio puede tener varias variantes 3D, pero todas deben respetar la misma ocupación de
grilla. El mismo snapshot de asentamiento debe reconstruir su vista normal y su mapa de asedio.

## Trabajo de diseño

Comparar worldgen determinista común, chunks publicados por servidor y un modelo híbrido de topología
estratégica más detalle cosmético Unity. La elección debe medir tamaño, exactitud TypeScript/C#, streaming,
navegación, niebla de guerra y migración.

## Criterios de aceptación futuros

- Sistema de coordenadas y conversión a Unity explícitos.
- Fixtures dorados cruzados para terreno, ríos, biomas, límites y grillas.
- Ninguna regla estratégica depende de detalle cosmético del cliente.
- Versiones incompatibles se rechazan.
- El streaming no expone información oculta.
- Un asentamiento y su escenario de asedio derivan del mismo estado autoritativo.

## Revisión desde BronzeAge (2026-09-11)

### Asentamiento — resuelto, con matices de Codex incorporados (2026-09-11)

El contrato de grilla/edificios/murallas para reconstrucción 3D y asedio ya existe casi completo en el
dominio actual (`Edificio` + `EDIFICIO_TAMANO`/`EDIFICIO_CATALOGO` para footprint, `Recinto`/`CeldaMuro` para
murallas y puertas). El detalle completo está en la revisión de BA-004, punto 4. Correcciones de Codex que
se incorporan al diseño aceptado:

- **`CeldaMuro` no tiene ID propio.** Su identidad estable se define como `enclosureId` (el `Recinto.id` que
  la contiene) + índice de recorrido dentro de `celdas` — o se le añade un `id` explícito si el consumo desde
  Unity resulta más simple con uno. Pendiente de decidir al implementar, no cambia el modelo.
- **Calles/trazado son datos derivados**, no persistidos punto a punto — necesitan `layoutVersion` (y la
  seed/perfil que ya use el trazado urbano) para que Unity pueda reconstruirlas idénticas al armar un
  escenario de asedio, en vez de recalcular con una versión distinta del algoritmo de trazado.
- `visualSeed` y `visualCatalogVersion` deben viajar en el snapshot que consume Unity, no ser solo un campo
  interno — si no, Unity no puede elegir variante determinística sin volver a preguntar.
- Invariante explícito: **una variante 3D nunca cambia huella, acceso, producción ni colisión estratégica**
  — solo aspecto. Esto ya lo implicaba el diseño, pero Codex lo deja como regla explícita a validar.
- Si en el futuro un asedio destruye edificios/murallas, el servidor de batalla Unity reporta HECHOS
  (qué se destruyó) en el `BattleResult`, igual que con bajas de escuadra — BronzeAge es quien decide y
  persiste el daño resultante, nunca Unity directamente.

Solo falta definir el factor de conversión celda→unidad Unity (sistema de coordenadas explícito).

### Mundo abierto (worldgen) — ACEPTADO por el usuario (2026-09-11), con frontera de Codex incorporada

Se confirma la recomendación híbrida: BronzeAge sigue siendo la fuente autoritativa de todo lo que afecta
reglas — topología estratégica, regiones, ríos, caminos, propiedad, zonas de influencia (ya lo es hoy,
`src/worldgen/` + `src/world/mapa.ts`) — y el detalle cosmético de terreno se genera del lado de Unity a
partir de la seed/config ya persistida (`WorldConfig`: `ancho`, `alto`, `seed`, `region?`), sin viajar por
red salvo la seed y `worldgenVersion`. Evita duplicar la generación determinista completa en C# (divergencia
TypeScript/C#) y evita servir el mundo entero como asset estático.

**Frontera añadida por Codex, incorporada como parte de la aceptación:** "cosmético" no es un cajón que
Unity pueda ensanchar a su criterio. Cualquier relieve que cambie navegación, visibilidad o accesibilidad
**deja de ser cosmético** y debe provenir de datos autoritativos de BronzeAge — Unity solo puede generar
microdetalle (piedras sueltas, hierba, texturas) cuando de verdad no altera ninguna regla. Esta frontera es
la que hace cumplible el criterio de aceptación "ninguna regla estratégica depende de detalle cosmético del
cliente".

