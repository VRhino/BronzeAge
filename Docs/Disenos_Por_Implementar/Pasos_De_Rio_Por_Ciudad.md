# Pasos de río por una ciudad ribereña

**Estado:** diseño heredado del servidor TypeScript, **sin implementar en Rust**. Sacado del canon y de `Docs/Mechanics/` el 2026-11-02 (decisión del autor, revisión de la documentación contra el motor). Entrada 69 de `Docs/Mecanicas a desarrollar.md`.

## Qué hace hoy el motor

Los ríos solo se cruzan por los **vados del mapa** (el grafo de navegación del artefacto de mundo solo tiene aristas de vado). Una ciudad junto a un río no abre paso. El parámetro `NAVEGACION.radioPasoCiudadMin` se carga y nadie lo usa.

## Regla diseñada

- Una ciudad ribereña abre un **paso propio** sobre el río: una arista que corta un cauce **fuera de un vado** se puede usar si el punto del cruce está a `≤ radioPasoCiudad` de la posición de **algún asentamiento** del mundo (`pasosRio` = las posiciones de todos los asentamientos, de cualquier Facción).
- `radioPasoCiudad = max(radioPasoCiudadMin, 1,5 × espaciado)` (el TypeScript usaba `radioPasoCiudadMin` 60 con su espaciado; Movimiento lo daba como 10 en el mapa calibrado).
- Lo usan las rutas de caravanas (`trazarRutaComercial`, acarreo), la marcha de los ejércitos y cualquier otro cálculo de ruta por tierra.
- Una plaza que cae en ruinas deja de abrir paso; una plaza nueva lo abre al fundarse (las rutas ya trazadas no cambian hasta el siguiente trazado).

## Qué falta decidir al implementarlo

- El radio calibrado para la retícula de 4 u del generador vigente.
- Si el paso es de todas las plazas o solo de las propias, aliadas o abiertas (hoy el diseño dice todas).
- Cómo entra en el grafo de `rutas` (aristas de cruce marcadas en el artefacto, activadas por plaza) sin recalcular el grafo entero.
