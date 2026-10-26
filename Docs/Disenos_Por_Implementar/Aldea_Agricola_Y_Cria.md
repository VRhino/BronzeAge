# Aldea agrícola y cría

> **Diseño acordado el 2026-10-26 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Lo que está marcado **(propuesta)** lo propuso el estudio de diseño y el autor no lo ha confirmado todavía; el resto es decisión suya. Ninguna cifra es vigente: costes, niveles internos, rendimientos y crecimiento son placeholders para el batch de población. Contexto: `Docs/Mecanicas a desarrollar.md` entradas 50 y 51. Modifica, al implementarse, [`Economia_Construccion.md`](../Mechanics/Economia_Construccion.md) §5.2, §6.2, §11.1 y §14 (pueblos extractores), `Trazado_Urbano.md` §6.3, `Generacion_Del_Mundo.md` §9 y el canon Doc 1.4, 4.2.2 y 4.5.

## 1. La aldea agrícola (idea 2)

Un tipo nuevo de **pueblo extractor**: la **aldea agrícola** (nombre provisional, para no confundirla con el edificio Granja de la ciudad). Lleva granjas y los **únicos corrales del juego**, y acarrea trigo y ganado a su plaza madre con el carro de siempre ([`Economia_Construccion.md`](../Mechanics/Economia_Construccion.md) §14, `crates/asentamiento/src/pueblos.rs`).

### 1.1 Qué cambia respecto del pueblo extractor de hoy

| Aspecto | Hoy (pueblo minero) | Aldea agrícola |
|---|---|---|
| Dónde se planta | sobre un nodo de piedra o mineral | en un **punto** de la zona de la madre (sin nodo): fertilidad mínima, ruta por tierra a la madre y separación de otros pueblos. `Pueblo` pasa de `nodo` a «sitio» (nodo o punto); `mapa.fertilidadEn` manda en sus granjas |
| Nivel mínimo de la madre | 2 (`nivelMadreMinimo`) | **1**. `nivelMadreMinimo` pasa a ser **por tipo** de pueblo |
| Cupo | 2/3/4/5 desde el nivel 2 | **cuenta como cualquier pueblo**; el cupo gana la clave del nivel 1: **1/3/4/5/6** (decidido) |
| Almacén local | un recurso, 500 | **varios recursos** (trigo y ganado); el carro carga los dos |
| Guarnición de serie | 30 mineros | **30 granjeros** (tropa `granjeros`, leva comunal, ya existe); misma reposición de uno en uno (`minutosPorMinero`, 12 min) |
| Por dentro | un extractor | **niveles fijos** de granjas y corrales (decidido): p. ej. 1: 2 granjas y 1 corral; 2: 4 y 2; 3: 6 y 3 (placeholders), que se suben como una obra de mapa (materiales llevados en el carro, obra corta) |
| Mano de obra | pesants de la madre | igual: más granjas piden más pesants y diluyen `ratioManoObra`, que es el freno natural |

- El **Corral sale de la ciudad** y solo existe en las aldeas; la Granja de la ciudad se queda (supervivencia y Granja inicial). La auto-construcción deja de proponer Corral.
- El coste (placeholder: extractor + 50 madera + carro con buey) se calibra en `batch poblacion` para que la subida 1 → 2 siga llegando hacia los 3 días (ritmo aprobado el 2026-10-13).
- La solvencia del ascenso cuenta lo acarreado por recurso (ventana de 24 h), sin cambios.
- Trazado y murallas se simplifican (el corral deja de ser «afueras» y de vetar celdas), pero cambia `LAYOUT_VERSION` y la planta de toda ciudad con corral.

### 1.2 El gate del nivel 2 (decidido)

Hoy (Doc 4.5): 200 pesants + **3 de los 7** tipos de extracción. Con los minerales solo por pueblo y el Corral fuera, en la ciudad quedan Cantera, Leñera y Granja. Nuevo gate del nivel 2: **200 pesants + 2 de {Cantera, Leñera, Granja} + una aldea agrícola activa**. Las minas y el Corral salen de la lista. Es el primer gate que mira un pueblo; `bloqueos_de_ascenso` la mira como mira edificios, y los requisitos son datos de `NIVEL_ASENTAMIENTO`.

### 1.3 Misión de asentamiento

Al fundar un asentamiento, su primera misión es levantar la aldea agrícola ([`Misiones.md`](Misiones.md)).

## 2. Cría ampliada (idea 4)

Los nodos sueltos de livestock **desaparecen del mundo** y el ganado sale solo de los **rebaños** de los corrales de las aldeas, que crecen por cría. Requiere la idea 2 (la aldea) y la idea 3 (las misiones que guían a construirla). El canon ya lo anunciaba (Doc 1.4: «cría/domesticación pospuesta»; Doc 1: «cuando exista la cría habrá las dos fuentes»).

**(propuesta)** salvo lo marcado como decidido:

- **Rebaño como ritmo.** Cada corral guarda cabezas e instante, con tope por corral y crecimiento por hora, como los reclutas del campamento: la cantidad de ahora se calcula al mirar. Los pueblos ya van en el paso de minuto de su madre, así que cabe también llevarlo allí.
- **Pie de cría y sacrificio.** Lo que pasa de un pie de cría mínimo se convierte en ganado en el almacén local y viaja en el carro a la Curtiduría.
- **Primer rebaño** con la obra del corral (o comprado con oro, como el buey).
- **Guerra.** El saqueo de una aldea se lleva parte del rebaño.
- **Especie preparada (decidido «si no es demasiado complicado»).** El rebaño lleva su **especie** y lo que da cada especie es una tabla de datos (`ESPECIES`): por especie, tope, crecimiento y productos. Solo **vacuno** al principio, con su único producto de hoy (ganado, que la Curtiduría convierte en cuero).
- **Carne y leche: fuera**, también del MVP (decidido): como recursos nuevos tocan el enum `Recurso`, almacén y granero, precios, trueque y caravanas, el `.proto`, los dorados de C# y TypeScript, `Comandos_Parametros` y la foto, y sin un uso (hoy solo se come trigo) no tendrían razón de existir en el comercio. Idea aparcada en `Docs/Ideas a diseñar.md`.
- **Generador y contrato de mundo.** Quitar el paso 4 de `Generacion_Del_Mundo.md` §9.2 (100 nodos) cambia la secuencia de tiradas: hierro y estaño extra salen en otros sitios y el artefacto de mundo cambia. Salen también `REGENERACION_NODOS.livestock`, la rejilla de ganado y los nodos de ganado de `06_Contrato_de_Mundo.md`. A decidir al implementar: aceptar mundos nuevos o consumir las tiradas sin colocar.
- **Nombre del recurso.** Dejar el id `livestock` y cambiar solo el texto que ve el jugador a «ganado» evita romper `buf breaking`, los dorados y la foto.
- **Tecnología.** El hito de `cria_caballar` («Corral activo») y el capítulo de su épica («obra de corral») pasan a mirar los corrales de las aldeas.
- **Después.** Cría de caballos (caballería) y bueyes (carros): por eso el rebaño lleva la especie desde el primer día.
