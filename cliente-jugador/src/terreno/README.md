# `terreno/` — copia independiente del evaluador de terreno del servidor

Estos archivos son una copia deliberada (no un import) de una parte pequeña de `src/worldgen/` del backend:
las funciones PURAS que evalúan elevación/fertilidad/bioma en un punto dado. Cero import de `../../src` del
backend — ver el README de la carpeta padre para el porqué arquitectónico.

## Qué se copió, y qué NO

| Archivo aquí | Copiado de (backend) | Qué NO se trajo |
|---|---|---|
| `ruido.ts` | `worldgen/ruido.ts` | `generarCampoRuido` (consume RNG — genera el campo, no lo evalúa; eso lo hace el servidor una sola vez al crear el mundo) |
| `elevacion.ts` | `worldgen/elevacion.ts` | Soporte de `region` (`worldgen/regiones.ts`, ~300 líneas de guías geográficas autoradas) — ver la limitación documentada en el propio archivo |
| `fertilidad.ts` | `worldgen/fertilidad.ts` | — (completo) |
| `biomas.ts` | `worldgen/biomas.ts` | — (completo) |
| `rios.ts` | `worldgen/rios.ts` + `world/geometria.ts` | La generación de ríos (RNG); solo la consulta de distancia |
| `config.ts` | `worldgen/config.ts` | Solo los umbrales que usa la evaluación (`ELEVACION`, `ELEVACION_SUAVIZADO`, `BIOMA`) — no `RIOS`, `BOSQUE_TERRENO_PERMITIDO` ni nada de generación |
| `poligonos.ts` | `world/poligonos.ts` | Solo `formaCirculo` + el motor de fusión (`unirFormas`); no `formaPoligono`/`unirPoligonos` — esas fusionan zonas de Facción, que dependen de TODOS los asentamientos rivales (entrada privilegiada, no portable) |
| `bosques.ts` | `world/mapa.ts` (`Mapa.contornosBosques()`) | Solo la fusión — `worldgen/bosques.ts` (colocación de los círculos) consume RNG, igual que `generarCampoRuido`; los círculos ya colocados (`ZonaBosque[]`) llegan como dato público en el mapa servido |

## Disciplina de mantenimiento

Estas funciones deben coincidir **bit a bit** con las del servidor para que el terreno pintado aquí sea el
terreno real. Si `WORLDGEN_VERSION` sube en el backend (`src/worldgen/types.ts`) y el cambio toca alguna de
estas funciones o de estos umbrales, esta carpeta necesita el mismo cambio — no hay ninguna comprobación
automática que lo detecte todavía. Es el coste aceptado a conciencia de la clasificación T2a (ver el README
de la carpeta padre): la alternativa —un viaje de red para pintar cada celda de terreno— es peor.

## Limitación conocida: sin `region`

Una partida creada con `region` (`POST /admin/partidas { region: 'nilo' | ... }`) usa una guía geográfica
autorada que este módulo no reproduce — el terreno que pinta diverge del real en esa partida. `elevacion.ts`
avisa por consola (una vez) si detecta `elevacion.region` en el mapa recibido. El caso SIN región ("mundo
libre", el default) se reproduce completo, incluido el borde natural del mapa.
