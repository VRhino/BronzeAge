# Mercenarios por contrato

> **En construcción (entrada 48).** De este documento el motor ya lee sus bloques de parámetros y su bloque de hechos; la prosa con las reglas tal como queden en el motor llega al
> cerrar la implementación. Mientras tanto, el diseño cerrado está en [`Disenos_Por_Implementar/Mercenarios_Por_Contrato.md`](../Disenos_Por_Implementar/Mercenarios_Por_Contrato.md).
> Todas las cifras son borradores que se calibran con bots (fase 7).

## Parámetros

Las tarifas son porcentajes enteros del valor del mercenario (20 de oro por nivel del héroe más lo que costaría hoy reclutar su tropa en su campamento); `permanentePct` es caro a propósito
(decisión del autor del 2026-11-01). El radio de defensa va en unidades de mapa; los plazos, en horas de mundo. El rango `escalon` 0 quiere decir sin licencia de reclutar.

<!-- parametros:MERCENARIOS_CONTRATO -->
```json
{
  "plazos": [
    { "dias": 1, "tarifaPct": 15, "renombre": 10 },
    { "dias": 3, "tarifaPct": 35, "renombre": 25 },
    { "dias": 7, "tarifaPct": 70, "renombre": 50 }
  ],
  "permanentePct": 500,
  "oroPorNivelHeroe": 20,
  "radioDefensa": 20,
  "plazoOrdenHoras": 24,
  "esperaRecontratoHoras": 72,
  "valoracionHoras": 72,
  "renombre": {
    "atacar": { "bandidosPorNivel": 5, "ejercito": 10, "caravana": 5, "pueblo": 15, "avanzada": 15, "plaza": 30 },
    "defensaGanada": 10,
    "escoltaLlegada": 5,
    "gradoMedalla": [10, 30, 80]
  },
  "valoracion": { "buenaPct": 150, "malaPct": 0 },
  "rangos": [
    { "id": "a_sueldo", "renombre": 0, "escalon": 0, "tarifaPct": 100 },
    { "id": "lanza", "renombre": 100, "escalon": 2, "tarifaPct": 120 },
    { "id": "capitan", "renombre": 300, "escalon": 3, "tarifaPct": 150 },
    { "id": "veterano", "renombre": 700, "escalon": 4, "tarifaPct": 180 },
    { "id": "senor_de_la_guerra", "renombre": 1500, "escalon": 5, "tarifaPct": 220 }
  ],
  "levaPropia": ["milicia_lanceros", "granjeros", "mineros", "lenadores"]
}
```
<!-- /parametros -->

Cada trabajo con las clases de objetivo que admite, su orden inicial y las órdenes que admite después. `guardar_objetivo` es `guardar` del objetivo del contrato; `atacar_en_radio`,
`atacar` de lo que esté a ≤ `radioDefensa` de él. La orden inicial del permanente es `guardar` de su plaza de residencia, no de su objetivo.

<!-- parametros:TRABAJOS_MERCENARIOS -->
```json
[
  { "id": "defender", "objetivos": ["plaza", "pueblo", "avanzada"], "inicial": "guardar", "admite": ["guardar_objetivo", "ir_a", "atacar_en_radio"] },
  { "id": "escoltar", "objetivos": ["caravana"], "inicial": "guardar", "admite": ["guardar_objetivo", "ir_a"] },
  { "id": "atacar", "objetivos": ["bandidos", "ejercito", "caravana", "plaza", "pueblo", "avanzada"], "inicial": "atacar", "admite": ["atacar", "ir_a"] },
  { "id": "acompanar", "objetivos": ["convocatoria", "ejercito"], "inicial": "unirse", "admite": ["unirse", "atacar", "ir_a", "guardar"] },
  { "id": "permanente", "objetivos": ["faccion"], "inicial": "guardar", "admite": ["guardar", "ir_a", "atacar", "unirse"] }
]
```
<!-- /parametros -->

Las medallas, con los umbrales de bronce, plata y oro de su contador (cada medalla cuenta uno solo, el de su id).

<!-- parametros:MEDALLAS_MERCENARIOS -->
```json
[
  { "id": "palabra_cumplida", "umbrales": [1, 10, 30] },
  { "id": "cazador_de_bandidos", "umbrales": [3, 10, 25] },
  { "id": "muralla", "umbrales": [3, 10, 25] },
  { "id": "buen_camino", "umbrales": [5, 20, 50] },
  { "id": "asaltante", "umbrales": [1, 5, 15] },
  { "id": "campana", "umbrales": [1, 5, 15] },
  { "id": "bien_valorado", "umbrales": [5, 20, 50] },
  { "id": "leal", "umbrales": [7, 30, 90] }
]
```
<!-- /parametros -->

## Hechos

<!-- eventos -->
```json
["mercenarios.ofrecido", "mercenarios.oferta_retirada", "mercenarios.contratado", "mercenarios.prorrogado", "mercenarios.ordenado", "mercenarios.orden_cumplida",
 "mercenarios.orden_caducada", "mercenarios.mudado", "mercenarios.terminado", "mercenarios.valorado", "mercenarios.rango_subido", "mercenarios.medalla",
 "mercenarios.premio_reclamado", "mercenarios.leva_recibida"]
```
<!-- /eventos -->
