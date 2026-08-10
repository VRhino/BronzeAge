# Definición de Fase 0

Objetivo: validar la simulación core (reglas, economía, crecimiento) sin invertir en arte, escenas ni modelos 3D. Representación mínima: mapa 2D top-down con formas simples (círculos, polígonos, líneas) y paneles de texto/UI de debug.

> Nota: para el detalle exacto y actualizado de cada mecánica, `Docs/1` a `Docs/6` son la fuente autoritativa. Este documento define el **alcance y la representación** de Fase 0, no repite el detalle mecánico completo.

## Sistemas totalmente testeables en Fase 0 (todos como datos puros)

- **Fundación y zonas de influencia** — punto de fundación → círculo/polígono de zona de influencia, expansión, colisión de fronteras (línea/borde de color).
- **Auto-construcción por necesidad** — lista/contador de "edificios lógicos" en panel, sin renderizar edificios reales. Colocación = "mejor casilla disponible según reglas", marcada como icono/color simple.
- **Recursos** — trigo/commodities de nobleza/madera/piedra/livestock, con ubicación (nodo, zona de bosque, o atributo de fertilidad según el tipo — ver Doc 1). Cadenas de producción invisibles (solo inputs/outputs netos visibles).
- **Población** — 3 clases con sus fórmulas de crecimiento.
- **Almacenamiento** — límites por recurso, disparo de construcción por superávit.
- **Políticas** — slots y pools por cargo, layout dinámico como flag simple (sin representación visual real de layout aún).
- **Tecnología** — 3 vías como checklist/flags por asentamiento, sin representación visual más allá de una lista.
- **Comercio y caravanas** — caravana = punto moviéndose entre dos nodos en el mapa 2D. Intercepción resuelta por cálculo (resultado en texto/log, sin animación).
- **Mantenimiento** — medidor 0-100, coste escalonado, degradación proporcional, destrucción a 0.
- **Escala social y política** — Facción/Liga/vasallaje/alianzas representadas como colores de facción en los polígonos de zona de influencia, sin interfaz gráfica especial.
- **Cargos y Ciudadanía** — como flags/datos, sin necesidad de UI dedicada.
- **Guerra** — resuelta como cálculo/log de texto (quién gana, bajas), sin instancias visuales.
- **Progresión sin condición de victoria** — recálculo periódico de títulos como dato puro.

## Lo que NO cabe en Fase 0 (queda para Fase 1 o posterior)

- Cualquier cosa que dependa de ver el asentamiento "por dentro" (edificios individuales con posición exacta, barrios, disposición visual real)
- Combate visual/animado (el resultado numérico sí se prueba, pero no la representación)
- Exploración con niebla de guerra o cualquier cosa que dependa de recorrer el mapa en primera/tercera persona
- Todo lo estético (identidad visual, culturas de referencia)
- Comercio marítimo, mar
- ~~Relieve de terreno (montañas/ríos)~~ — adelantado parcialmente en Fase 0.1 (ver
  `Fase_0_1_Definicion.md`): el generador de mundo pasa a tener elevación/ríos/biomas, como preparación de
  bajo costo para el mapa de campaña de Fase 1. Sigue sin haber mar ni movimiento/pathfinding sobre el
  terreno.
