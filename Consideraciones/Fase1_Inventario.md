# Fase 1 — Inventario de `Consideraciones/`

> **Estado (2026-10-12): PROPUESTA para aprobar.** Clasificación hecha leyendo la cabecera y el estado de cada documento, no
> línea a línea; la verificación de que cada regla llegó al canon es el paso 2 de cada fila marcada «verificar». Nada se borra
> hasta que apruebes la tabla. Plan: `Reconstruccion_Definicion.md` §4.1.

## Criterio

Con la reconstrucción hay tres destinos y una condición:

- **Canon**: la regla vive (o debe vivir) en `Docs/Game/` y, si hace falta reimplementarla con exactitud, en `Docs/Mechanics/`.
- **Historia**: razones y alternativas descartadas de lo ya decidido. Se conserva solo si explica un *porqué* que el canon no
  recoge; el «cómo se hizo en TypeScript» (pasos, archivos, tests, snapshots) no sirve para Rust y queda en `typescript-final`.
- **Descartado**: superado por la reconstrucción o sin valor para ella.
- **Condición**: ninguna decisión puede vivir solo aquí. Lo que hoy esté solo aquí **sube al canon** en esta fase.

## Hallazgo que condiciona la fase

`Docs/Mechanics/` (el nivel «con precisión suficiente para reimplementar») tiene solo 5 documentos: combate, movimiento,
murallas, trazado urbano y divergencias. El resto de las mecánicas implementadas (economía, comercio, población, política,
niebla, tecnología, campamentos, caravanas…) hoy tienen la regla en `Docs/Game/` pero **el detalle exacto solo está en el
código TypeScript**. Para reimplementar en Rust sin perderlo hay que extraerlo a `Docs/Mechanics/` antes de retirar el motor.
Eso añade a la fase 1 un trabajo grande que no estaba en §4.1. (La memoria dice que economía, tick, niebla, tecnología y
comercio se documentaron el 2026-10-11, pero no están en `main`: hay que localizar esa rama o sesión.)

## A. Decisiones que viven solo aquí → suben al canon (trabajo real de la fase)

| Documento | Líneas | Qué contiene | Acción |
|---|---:|---|---|
| `Calibracion_Distancias_Definicion.md` | 497 | Velocidades, zonas, ciudad, focos, anillos, estaño, bosques | Bajar §2-§5 a `Docs/Game/1` y constantes al archivo común; O1 y O2 abiertos (**tú**) |
| `Guerra_Logistica_Definicion.md` | 352 | Caravanas A/B, pueblos extractores, avanzadas, la plaza ve solo su zona | Bajar a `Docs/Game/3` y `5`; cerrar pendiente §44 |
| `Mecanicas a desarrollar.md` §45 y víveres | — | Repostar por zona, víveres del ejército, enganchar solo dentro | Decidir (**tú**) y bajar a `Docs/Game/5` |
| `BA-006_Revision_Tecnologia_Eras.md` | 961 | Eras I-III en canon; IV-V **pospuestas** por el usuario (2026-10-05) | Se conservan como catálogo de datos pospuesto; el contrato debe dejarlas abiertas (§20) |
| `Ritmo_Crecimiento_Asentamientos.md` | 806 | Crecimiento en horas vs Eras en semanas; Era I medida, decisiones pendientes | Decidir (**tú**) con la calibración nueva |
| `Murallas_Definicion.md` | 1096 | Murallas por nivel (empalizada 2 / piedra 3 / adarve 4), decidido 2026-10-09 | Verificar contra `Docs/Mechanics/Murallas.md`; bajar a `Docs/Game/4` |
| `Economia_Del_Oro_Definicion.md` | 545 | Impuestos y demanda; Paso 6 (calibración) sin hacer | Confirmar canon; la calibración se rehace con el motor nuevo |
| `Ocupacion_Post_Conquista_Definicion.md` | 656 | Ocupación; cifras `OCUPACION.*` placeholder | Confirmar canon; la calibración se rehace |
| `Vista_Asentamiento_Trazado_Urbano.md` | 1897 | Historial del trazado urbano | Lo vigente ya está en `Docs/Mechanics/Trazado_Urbano.md`; **decidir** (**tú**) si el trazado lo hace Unity o el artefacto |

## B. Implementado y con regla en el canon → historia (verificar que no quede nada solo aquí)

| Documento | Líneas | Verificar |
|---|---:|---|
| `Anexion_Y_Desarme_Definicion.md` | 92 | canon `Docs/Game/2` §2.4, §2.6 |
| `Aedas_Definicion.md` | 108 | `Docs/Game/6` §6.1, 6.4, 6.7 |
| `Batalla_Este_Lado_Definicion.md` | 250 | pasa a la parte de batalla del contrato 05 |
| `Campamentos_Entrada_Fundacion_Definicion.md` | 463 | `Docs/Game/1`, `4`; 71 decisiones D*, repasar que estén |
| `Comercio_Fisico_Definicion.md` | 168 | `Docs/Game/3` |
| `Entrada_Al_Mundo_Definicion.md` | 150 | `Docs/Game/1`; el punto 6 (huésped) sigue sin fecha |
| `Identidad_Visual_Definicion.md` | 171 | `Docs/Game/2` §2.8.1; los 7 iconos por encargar son de Unity |
| `Jugador_Situado_Definicion.md` | 1443 | `Docs/Game/1`, `5`; el héroe como entidad situada |
| `Movimiento_Ejercitos_Definicion.md` | 1295 | `Docs/Mechanics/Movimiento_Ejercitos.md` (se rehace sobre planes, sin tick) |
| `Niebla_De_Guerra_Definicion.md` | 499 | `Docs/Game/1`; la visión pasa a muestreada (doc 13) |
| `Revamp_Caravanas_Definicion.md` | 308 | `Docs/Game/3` §3.13; trozos diferidos §8.1 |
| `Rutas_Caravana_Avanzadas_Definicion.md` | 190 | `Docs/Game/1` §1.6; falta el logro de la Era IV |
| `Taberna_Intel_Definicion.md` | 74 | `Docs/Game/5` §5.12.10, `4` §4.2.1 |
| `Tecnologia_Eras_I-III_Definicion.md` | 320 | plan técnico en TypeScript: **descartado**; el contenido está en BA-006 |
| `Fase_0_5_Definicion_Especializacion_y_Cupos.md` | 465 | `Docs/Game/4`; cifras placeholder |
| `Fase_0_6_Definicion_Expansion_Niveles_Asentamiento.md` | 360 | `Docs/Game/4` |
| `Checklist_Mecanicas.md` | 296 | mecánicas ya cerradas: historial |

## C. Descartado (TypeScript, planes ya cumplidos o superados)

| Documento | Líneas | Por qué |
|---|---:|---|
| `Auditoria_Tick_Eventos.md` | 518 | Auditoría del tick; el mundo reactivo la sustituye. Antes de descartarla, extraer los bugs B* que son **reglas** y no del motor |
| `Correcciones_Durante_Desarrollo.md` | 565 | Bugs del motor TypeScript |
| `Plan_Implementacion_Tecnica.md` | 63 | Stack y modelo TypeScript de la Fase 0 |
| `Roadmap_Escalado.md` | 59 | Hoja de ruta de fidelidad visual superada por la reconstrucción |
| `Fase_0_Definicion.md` | 33 | Alcance de la Fase 0 |
| `Fase_0_1_Definicion.md`, `Fase_0_3_Definicion.md`, `Fase_0_4_Definicion_Relieve_Jugable.md` | 75 / 93 / 169 | Worldgen anterior; lo sustituye el regional de Codex. Verificar que caminos y chokepoints (Doc 1 §1.5-§1.6) quedan en el canon |
| `Revision_2026-09-08_Economia_Y_Guerra.md` | 160 | Índice de sesión |

## D. Se quedan o requieren decisión

| Documento | Líneas | Destino |
|---|---:|---|
| `Reconstruccion_Definicion.md` | 188 | Se queda: plan vigente |
| `NPC_Gobernanza_Facciones_Controladas.md` | 400 | Pasa a `Docs/Arquitectura/12` (bots); cesión de Facción al NPC: **decidir** (**tú**) si sobrevive |
| `Preguntas_Abiertas.md` | 139 | Vaciar: cada punto se resuelve, se descarta o pasa a `Mecanicas a desarrollar.md` |

## Qué decides tú para cerrar el inventario

1. Aprobar o corregir las tres tablas.
2. Los 7 pendientes de `Mecanicas a desarrollar.md`: caravanas diferidas, Eras (§20), tope de asedio (§38), guerra logística (§44), repostar y enganchar (§45), bots emergentes (§46).
3. Trazado urbano: ¿lo calcula Unity o lo entrega el artefacto de mundo?
4. NPC de gobernanza: ¿se conserva la cesión de Facción?
5. Que la extracción de los detalles exactos a `Docs/Mechanics/` entre en la fase 1.
