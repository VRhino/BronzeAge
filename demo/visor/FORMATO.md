# Visor de la demo: uso y formato de `demo.json`

`index.html` es una sola página sin dependencias. Anima un `demo.json` **evaluando los planes en cada fotograma**
(interpolación lineal por tramos entre vértices con instante de paso), como hará el cliente real
([13 §5](../../Docs/Arquitectura/13_Mundo_Reactivo_Rust.md)): no hay posiciones por tick.

## Uso

```
cd demo/visor
python -m http.server 8765        # o cualquier servidor estático
# abrir http://localhost:8765/index.html            (carga demo.json y, si no existe, ejemplo.json)
# http://localhost:8765/index.html?f=otro.json      (carga ese archivo)
```

Desde `file://` el navegador no deja `fetch`: usa el botón **Abrir demo.json** o arrastra el archivo a la página.
(El 404 de `demo.json` en la consola es el intento previo al respaldo `ejemplo.json`.)

Controles: arrastrar = desplazar; rueda = zoom; clic en un punto = seleccionar (muestra su plan: pasado a trazos,
futuro continuo, vértices y radio de visión); doble clic en la lista = seguir; clic en un evento = saltar a su instante
y centrar el mapa; espacio = play/pausa; barra inferior = saltar en el tiempo; x1/x10/x60/x600 = velocidad.
Con más de 200 entidades no se dibuja el plan entero de cada una, y con más de 2000 tampoco la estela (solo la del
seleccionado). Con más de 600 los puntos se pintan con `fillRect` agrupados por color.

## Ejemplos

```
node generar-ejemplo.mjs                              # -> ejemplo.json (50 entidades, semilla 42; está commiteado)
node generar-ejemplo.mjs 10000 ejemplo-10000.json     # prueba de carga; NO se commitea (ver .gitignore, ~5,6 MB)
node generar-ejemplo.mjs N salida.json SEMILLA
```

## Formato

Posiciones y radios en **milésimas de unidad estratégica** (x crece a la derecha, y hacia abajo), tiempos en **ms de mundo**.

```json
{ "titulo": "...", "semilla": 42, "duracion_ms": 3600000,
  "mundo": { "ancho": 2667000, "alto": 2000000,
             "terreno": { "columnas": 667, "filas": 500, "base64": "<u8 por celda, fila a fila>" } },
  "campamentos": [ { "id": "campamento-0", "x": 0, "y": 0 } ],
  "entidades": [ { "id": "ej-1", "nombre": "Ejército 1", "faccion": "f1", "color": "#3b82f6",
                   "plan": [ { "x": 0, "y": 0, "t": 0 } ], "radio_vision": 30000 } ],
  "eventos": [ { "t": 12345, "tipo": "encuentro", "texto": "...", "x": 0, "y": 0, "entidades": ["ej-1", "ej-2"] } ] }
```

- `terreno.base64`: `columnas * filas` bytes, fila a fila desde arriba-izquierda; cada celda cubre `ancho/columnas` x `alto/filas`.
  Valores: 1 agua, 2 costa, 3 llano, 4 colina, 5 montaña, 6 cima (0 = sin dato, transparente). Opcional.
- `plan`: vértices con `t` no decreciente. Antes del primer `t` la entidad no se ve; después del último se queda quieta
  en el último vértice. Dos vértices consecutivos con la misma posición son una parada. Un cambio de plan en el servidor
  sería un `plan` nuevo cuyo primer vértice es la posición actual (aquí cada entidad trae uno solo para toda la demo).
- `eventos`: se ordenan por `t` al cargar; `x`,`y` y `entidades` son opcionales; el anillo del mapa dura unos segundos reales.
- `campamentos`, `radio_vision` y `color` son opcionales.

## Cómo evalúa el visor (rendimiento)

Planes aplanados en `Float64Array` (`T`, `X`, `Y`) con rango por entidad y un **cursor de tramo cacheado**: en reproducción
normal el tramo es el mismo o el siguiente (O(1)); si hay salto de tiempo, búsqueda binaria. El indicador del mapa muestra
fotogramas, evaluaciones/s y ms de evaluar y de dibujar por fotograma (también en `window.__visor.M`).
Medido en el navegador integrado con `ejemplo-10000.json` a x600: 60 fps, evaluar ~0,13 ms y dibujar ~1,3 ms por fotograma
(600.000 evaluaciones/s).
