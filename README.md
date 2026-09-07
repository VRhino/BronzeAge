# Bronze Age Collapse Game — Documentación de Diseño

Juego de estrategia/MMO ambientado en la Edad Oscura de la Edad de Bronce (colapso de civilizaciones, foco geográfico Grecia/Egeo). Sandbox de guerra persistente sin condiciones de victoria. Proyecto separado de un juego previo de control territorial (Casas/nodos/caravanas), del cual se reutilizan mecánicas adaptadas.

## Cómo usar esta documentación (para Claude Code / generación de código)

Leer en este orden:

1. **`Docs/0_Glosario_de_Entidades_Politicas.md`** — terminología central (Jugador, Asentamiento, Facción, Liga, Vasallo, Aliado, Gran Rey, Cargos, Población, Aedas, Oro, Nodo). Consultar ante cualquier ambigüedad de término en el resto de documentos.
2. **`Docs/1_Sistema_de_Mundo_y_Territorio.md`** — generación de mapa, fundación, zonas de influencia, fronteras, recursos, caminos, cap de fundación.
3. **`Docs/2_Sistema_Politico_Facciones_y_Cargos.md`** — Facción, Liga, cargos, vasallaje, ciudadanía, fusión/anexión, reputación, progresión sin victoria.
4. **`Docs/3_Sistema_Economico_y_Comercio.md`** — oro, trueque, caravanas, mercado, precios, comisiones.
5. **`Docs/4_Sistema_de_Poblacion_Construccion_y_Mantenimiento.md`** — 3 clases de población, auto-construcción, políticas, mantenimiento.
6. **`Docs/5_Sistema_Militar_y_Combate.md`** — combate, formaciones, roster de tropas, reclutamiento.
7. **`Docs/6_Sistema_de_Tecnologia_y_Aedas.md`** — vías de tecnología, Aedas/Poetas.
8. **`Consideraciones/Fase_0_Definicion.md`** — alcance exacto de lo que se implementa primero (simulación pura de datos, sin gráficos).
9. **`Consideraciones/Plan_Implementacion_Tecnica.md`** — stack (TypeScript), modelo de datos, orden de sprints. **Documento más directamente accionable para generar código.**
10. **`Consideraciones/Roadmap_Escalado.md`** — fases posteriores a Fase 0 (Fase 1 mapa de campaña en Unity/C#, Fase final integración total).
11. **`Consideraciones/Checklist_Mecanicas.md`** — estado de completitud (cerrado/pendiente) de cada sistema.
12. **`Consideraciones/Preguntas_Abiertas.md`** — lo que sigue sin resolver. Usar valores placeholder razonables para estos puntos, no bloquear la implementación por ellos.
13. **`Consideraciones/Correcciones_Durante_Desarrollo.md`** — bugs de diseño/lógica encontrados y corregidos al implementar Fase 0; varios resuelven preguntas que estaban marcadas como pendientes.
14. **`Consideraciones/NPC_Gobernanza_Facciones_Controladas.md`** — herramienta de la interfaz (no mecánica de juego): ceder una Facción a un NPC que la gobierna sola mientras se juegan otras a mano, con acoplamiento cero con el motor.

## Estado general del diseño

El **núcleo de simulación** (economía, población, construcción automática, políticas, cargos, vasallaje/Facción, mantenimiento, generación de mundo Fase 0) está **cerrado** y listo para implementar. Los huecos que quedan son mayoritariamente **ajustes numéricos** (cantidades exactas, curvas de escalado, catálogos detallados de políticas) más que decisiones de diseño pendientes — ver `Consideraciones/Preguntas_Abiertas.md` para el detalle completo.

Contenido explícitamente **fuera de Fase 0** (no implementar aún): representación 3D/escenas, combate instanciado visual, exploración con niebla de guerra, identidad visual/audio, comercio marítimo, relieve de terreno (montañas/ríos/mar).

## Estado de implementación

Fase 0 (Sprints 1-6, el motor de simulación puro) está **implementada** en TypeScript (`src/`). Desde ahí el
proyecto siguió evolucionando **más allá de lo que describe esta sección** — ver
**[`Docs/Arquitectura/1_Arquitectura_Actual.md`](Docs/Arquitectura/1_Arquitectura_Actual.md)** para el estado
real hoy: este repositorio es ahora un **backend multijugador puro** (Fastify, autenticación/autorización por
comando, proyecciones por audiencia, WebSocket) — "validada jugando en el navegador" ya no aplica, el juego se
prueba contra la API HTTP/WS. El cliente de JUGADOR vive desde `2dfe9e7` en su propio repositorio
(`BronzeAgeClient`); aquí solo queda `cliente/`, la consola de administración/depuración, que todavía no puede
salir porque sigue importando el motor (ver `cliente/README.md`). El roadmap completo de esa evolución
(Fases A-E) vive en **[`Docs/Arquitectura/3_Plan_Evolucion_Roadmap.md`](Docs/Arquitectura/3_Plan_Evolucion_Roadmap.md)**.
Ver **`Consideraciones/Correcciones_Durante_Desarrollo.md`** para el registro de bugs de diseño/lógica
encontrados y corregidos durante la implementación de Fase 0 — varios de ellos resolvieron preguntas que
estaban marcadas como pendientes en esta documentación (protección temporal de asentamientos nuevos,
reemplazo de recursos agotados, calibración de Mantenimiento), y ya están reflejados en los Docs 1, 2, 4 y 5.

## Estructura del repositorio

```
Docs/                                              Documentación de diseño por sistema (QUÉ es el juego)
  0_Glosario_de_Entidades_Politicas.md
  1_Sistema_de_Mundo_y_Territorio.md
  2_Sistema_Politico_Facciones_y_Cargos.md
  3_Sistema_Economico_y_Comercio.md
  4_Sistema_de_Poblacion_Construccion_y_Mantenimiento.md
  5_Sistema_Militar_y_Combate.md
  6_Sistema_de_Tecnologia_y_Aedas.md
  Arquitectura/                                    Evolución del backend (CÓMO se sirve el juego) — empezar
                                                     por 1_Arquitectura_Actual.md y 3_Plan_Evolucion_Roadmap.md
Consideraciones/                                   Alcance, plan técnico y estado del proyecto
  Fase_0_Definicion.md
  Plan_Implementacion_Tecnica.md
  Roadmap_Escalado.md
  Checklist_Mecanicas.md
  Preguntas_Abiertas.md
  Correcciones_Durante_Desarrollo.md
src/                                                Backend: domain/constants/worldgen/world/engine (motor
                                                     puro de Fase 0) + acceso/session/server (multijugador,
                                                     Fases B-C)
cliente/                                            Cliente de depuración/administración (proyecto separado,
                                                     sigue importando el motor — ver cliente/README.md)
```
