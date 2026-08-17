# Vista de Asentamiento — Trazado urbano dinámico

Documento vivo. Recoge **solo lo acordado** en la discusión de diseño. Lo que sigue abierto va al final,
separado y marcado como tal.

## Principio rector

**Nada del trazado urbano se define al fundar el asentamiento.** Ni calles, ni manzanas, ni reservas de
espacio. Todo emerge pieza por pieza, a medida que se construyen edificios. Un plano pre-generado —aunque sea
determinista y "orgánico"— queda descartado explícitamente.

## 1. Geometría de las calles

**Las calles corren sobre las aristas de la rejilla, no sobre celdas.** Pasan entre celdas, por los vértices;
no consumen superficie construible.

```
   descartado (celda)        acordado (arista)
   ┌───┬───┬───┐             ┌───┬───┬───┐
   │ A │▓▓▓│ B │             │ A ┃   ┃ B │      ▓ = celda ocupada por calle
   ├───┼───┼───┤             ├───╄━━━╃───┤      ┃ = calle sobre arista
   │▓▓▓│▓▓▓│▓▓▓│             │   ┃   ┃   │
   └───┴───┴───┘             └───┴───┴───┘
```

Consecuencias, que se cumplen **por construcción y no por validación**:

- Un edificio nunca puede quedar encima de una calle: la calle no ocupa celdas.
- Una calle nunca puede quedar encima de otra: la red se guarda como un **conjunto de aristas unitarias**, así
  que agregar una arista existente es un no-op.

Solo ángulos de 90°. Ningún tramo diagonal, nunca.

## 2. Representación de la red

- La red es un **conjunto de aristas unitarias** de la rejilla.
- Es **derivada, no persistida**: se reconstruye recorriendo los edificios en su orden de construcción y
  aplicando la regla de crecimiento paso a paso. Como cada paso solo depende del estado anterior, el replay es
  determinista y estable — no hace falta guardar nada ni migrar partidas guardadas.
- El motor la necesita (para preferir sitios ya conectados), así que se calcula ahí. `ui/canvas.ts` solo
  dibuja lo que recibe, respetando el acoplamiento 0 de siempre.

## 3. Invariante de conexión

**Todo edificio toca una calle**: al menos una arista de su perímetro pertenece a la red.

Vale igual para un 1x1 que para el 6x6 de Granja nivel 4 — no hace falta ninguna regla de "fachada" ni
tratamiento por tamaño.

## 4. Crecimiento de la red

- **Semilla.** Al fundar, la red inicial es el **perímetro del Centro Urbano**: sus cuatro lados como aristas
  de calle. Nace ya siendo un anillo cerrado (el CU es la manzana cero) y ofrece frente de calle en las cuatro
  direcciones desde el primer tick — lo que evita el bloqueo del intento anterior, donde algunos barrios no
  tenían dónde construir al principio y el asentamiento se trababa.
- Una calle se crea **solo cuando hace falta**, al momento de colocar un edificio que la necesita.
- Un edificio que queda lejos de la red se conecta **extendiendo la calle más cercana** hasta él (no trazando
  un camino nuevo desde el Centro Urbano).
- Un tramo nunca puede atravesar un edificio, y no hace falta comprobarlo: toda arista corre por el borde
  entre dos celdas (§1).
- Entre las aristas candidatas a fachada se elige la que **prolonga una calle existente en línea recta**. Sin
  eso, muchas fachadas salen perpendiculares a la hilera y se leen como cruces entre dos casas vecinas.

## 5. Colocación de edificios

- Se prioriza la **zona/barrio** que le corresponde al tipo de edificio (cuña de dirección aleatoria por
  asentamiento, mecanismo ya existente).
- Dentro de esa zona, se **prefiere un hueco que ya toque calle existente**. Solo si no hay ninguno viable se
  coloca fuera y se extiende la red hasta ahí. Resultado buscado: ciudad compacta, poca calle redundante.

**Prioridad barrio vs. fila.** El barrio manda para elegir **dónde empezar una manzana nueva**; la fila manda
**dentro de una manzana ya empezada**. Una fila en crecimiento se completa aunque se desvíe del ángulo exacto
de su cuña. Si no, las manzanas quedarían cortadas por la mitad en un borde invisible y todo volvería a verse
disperso.

## 6. Tamaños de edificio

Cada tipo ocupa un rectángulo ancho×alto de celdas (no una sola celda). El tamaño se **deriva** de `tipo`
(+ `nivelInterno` donde aplique), nunca se persiste en el `Edificio`.

| Tipo | Tamaño | Ubicación |
|---|---|---|
| Centro Urbano | 3x3 | origen |
| Carpintería | 5x4 | barrio |
| Fundición | 2x2 | barrio |
| Armería | 2x3 | barrio |
| Barracón | 2x2 | barrio |
| Galería de tiro | 2x4 | barrio |
| Leñera / Vivienda / Almacén | 1x1 | barrio |
| Palacio | 4x4 | sin barrio, lo más cerca posible del centro |
| Mercado | zona multi-pieza (ver abajo) | barrio |
| Corral | 4x3 | **afueras** |
| Granja | 1x1 → 2x3 → 4x3 → 6x6 (niveles 1-4) | **afueras** |

**Mercado** no es un edificio: es una zona que se arma con varias piezas independientes, cada una su propio
`Edificio` con su propio tamaño, agregadas al subir de nivel.

- Nivel 1: 2 piezas de 2x2 + 1 de 3x2
- Nivel 2: + 4 piezas de 1x1 + 3 de 2x2
- Nivel 3: + 2 piezas de 3x2

No se exige que las piezas queden pegadas entre sí; usan la misma acreción por barrio que el resto.

## 7. Granja: crecimiento por niveles

Granja cambia de tamaño al subir de nivel interno. Al mejorar:

- **Se muda** a otro hueco (no se exige que quepa en el sitio actual).
- El hueco elegido es el **más cercano posible** a su posición previa.
- Siempre debe quedar **en las afueras**: en el límite exterior de la cuadrícula, nunca reabsorbida hacia el
  centro.

**La mejora tiene prioridad sobre la cercanía.** Si no hay hueco cerca, la granja se muda igual, aunque termine
en el extremo opuesto del mapa de asentamiento. La mejora nunca se bloquea ni queda en espera por falta de
espacio adyacente.

## 8. Manzanas emergentes

Las manzanas **no se dibujan ni se guardan**: son los ciclos de la red, el espacio negativo que queda
encerrado cuando las calles cierran un anillo. Salen de tres reglas:

1. **Fila con medianera.** Al buscar hueco, el edificio prefiere quedar **pegado, pared con pared**, al
   costado de otro que ya da a la misma calle, antes que empezar sitio nuevo. Sin separación entre vecinos.
   Esto es lo que convierte edificios dispersos en una cara sólida de manzana.
2. **Dos hileras de fondo por manzana.** Sale del invariante de §3: si todo edificio debe tocar calle, detrás
   de la segunda hilera ya no se puede construir sin traer otra calle. Las calles de hilera caen cada 2 filas.
3. **Transversales cada N columnas.** N se **sortea por asentamiento en el rango 4–8** con la misma semilla
   determinista que `direccionesDelAsentamiento`, más un desfase propio para que dos ciudades no tengan los
   bloques alineados a las mismas columnas. La manzana queda de N × 2 celdas.

Cuando un edificio cae sobre una de esas líneas, aporta **su trozo** de calle. La línea completa se va
formando conforme la ciudad crece hacia ahí, y **si nunca crece, esa calle no llega a existir**: no hay ningún
plano previo, solo un criterio de dónde caería la transversal el día que alguien la necesite.

> **Nota de implementación.** El criterio de "cada N columnas" es posicional a propósito. Se probaron antes
> tres versiones basadas en el historial —disparar al llegar a un largo de fila, por módulo de ese largo, y con
> un veto de separación mínima entre cruces— y las tres fallan por lo mismo: dependen del ORDEN de
> construcción, y como una hilera crece por los dos extremos y se vuelve a medir entera cada vez, las
> transversales salían amontonadas o no salían casi nunca (medido: de 2 a 6 manzanas cerradas en una ciudad de
> 50 edificios, a veces con bloques de un solo edificio de ancho).

## 9. Afueras

Granja y Corral se colocan fuera de la trama urbana (radio mínimo respecto al centro, preferencia por lo más
lejano). **Igual les llega un camino** que las conecta con la red — no quedan sueltas.

**Camino ≠ calle.** Son dos clases distintas:

- La **calle** es urbana: forma filas, admite medianeras y cierra manzanas.
- El **camino** solo conecta. Es más fino en el dibujo, y no genera manzanas ni atrae edificios a sus lados.

## 10. Bugs a no repetir

Fallos concretos observados en el enfoque de manzanas pre-generadas, que este diseño debe descartar de raíz:

1. Edificios colocados encima de calles → resuelto por §1 (calle sobre arista).
2. Calles superpuestas a otras calles → resuelto por §2 (conjunto de aristas).
3. Granjas demasiado cerca del Centro Urbano → resuelto por §8 (rama afueras explícita).

---

## 11. Dibujo

- Los edificios se dibujan con un pequeño margen hacia adentro de su celda. Como van pared con pared, sin ese
  margen la calle —que corre justo sobre el borde compartido— queda tapada y no se lee.
- El camino rural (§9) se dibuja más fino que la calle urbana.
- `ui/canvas.ts` sigue sin importar nada de `engine/*` ni de `domain/types`: recibe todo resuelto vía
  `DrawAsentamientoState`, armado en `main.ts` desde `gameStore`. Acoplamiento 0, como siempre.

## 12. Mejora de Granja

Cuatro niveles internos. Cada salto **duplica el costo en materiales** respecto al anterior, sobre la base de
su costo de construcción: madera 30 → 60 → 120 → 240. Duplica también su rinde de trigo (15 → 30 → 60 → 120):
una granja de nivel 4 ocupa 36 celdas contra la única del nivel 1, así que mantener el rinde plano convertiría
la mejora en gasto puro. Los trabajadores requeridos no cambian.

---

## Abierto

- **Mercado multi-pieza** (§6): implementado como UN edificio con huella única. Convertirlo en 3/10/12 piezas
  toca `EDIFICIOS_UNICOS`, `cupoCaravanas` y el comercio, así que se dejó fuera de la tanda del trazado.
