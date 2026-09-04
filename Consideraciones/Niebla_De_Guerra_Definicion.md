# Niebla de guerra ("último conocido") — cierre de diseño y plan de ejecución

> **Estado (2026-09-04): diseño cerrado, CERO código escrito.** Mismo criterio que
> `Murallas_Definicion.md` y `Movimiento_Ejercitos_Definicion.md`: la especificación se escribe ANTES de tocar
> el motor.
>
> **Las REGLAS DE JUEGO viven en `Docs/Game/`**, no aquí. Este documento cubre el registro de decisiones, la
> representación en el motor, el orden de ejecución y qué se congela en tests. El enunciado original es
> `Docs/Mecanicas a desarrollar.md` §12.

## 1. De dónde viene, y qué había ya

Sale del plan de arquitectura, donde era "C4 Slice 2". La infraestructura estaba hecha desde hace tiempo: la
proyección por audiencia (`proyectarParaJugador`) ya filtra el estado a lo que un jugador puede ver. Lo que
faltaba era **decidir el juego**, y en concreto un número: el radio de visión. Sin él, el filtro espacial no
se podía escribir.

**El movimiento de ejércitos desbloqueó ese número** y de paso implementó la primera mitad: desde el 2026-09-03
un jugador ve los ejércitos ajenos que entran en su zona de influencia o en el radio de visión de uno de los
suyos (`ejercitosAvistados`, Doc 5.12.7), redactados a posición, Facción y nº de participantes.

Así que esto ya no es una mecánica desde cero: es **completar** lo que quedó a medias.

| Pieza | Estado antes de esta mecánica |
|---|---|
| Proyección por audiencia | Hecha (Fase C4) |
| Ver ejércitos ajenos por espacio | **Hecho** (Doc 5.12.7) |
| Ver asentamientos ajenos por espacio | Falta — hoy no se proyecta ni uno |
| Memoria ("último conocido") | Falta — hoy lo visto aparece y desaparece |
| Conocimiento por contacto | Falta |
| Visión compartida por alianza | Falta |

## 2. Decisiones cerradas con el usuario (2026-09-04)

### 2.1 Un asentamiento ve su zona de influencia MÁS un margen

Decisión: **zona + un radio extra**. La plaza vigila algo más allá de su frontera, como una atalaya.

El margen es **60** unidades de mapa. Tres razones para ese número y no otro:

- **Menos que la vista de un ejército** (150), que es lo que mantiene el valor de explorar: una columna en
  marcha divisa una plaza mucho antes de que la plaza la divise a ella, y conserva la iniciativa.
- **Duplica el área vigilada de una plaza recién fundada**, cuya zona es exactamente 60. El margen se nota
  desde el primer minuto en vez de ser un detalle que solo importa tarde.
- Es la misma distancia a la que un ejército puede repostar en una plaza (`radioReabastecimiento`), lo que da
  una coincidencia útil de leer: **si una columna está lo bastante cerca como para repostar en tu ciudad,
  tu ciudad la ve.**

> **Consecuencia que conviene tener presente**: como la zona crece con el nivel (60 → 180), un asentamiento de
> nivel 5 vigila 240 y por tanto **ve más lejos que un ejército**. Es coherente con la ficción —una capital
> tiene atalayas, un ejército tiene ojos— y con premiar el crecimiento, pero significa que la ventaja de
> explorar se diluye alrededor de las grandes ciudades. Anotado para calibración, no para bloquear.

### 2.2 De un asentamiento rival se sabe su FICHA, no su interior

Decisión: **nombre, Facción, posición y nivel.** Nada más.

Es la misma filosofía de redacción que ya se aplica a los ejércitos ajenos: quién es y dónde está, nunca su
interior. Quedan fuera a propósito el almacén, la guarnición, las colas de construcción y los edificios — eso
es telemetría de un rival, y es justo lo que la proyección existe para impedir.

El **nivel sí entra** porque es visible desde fuera en la ficción: una ciudad grande se ve grande. No dice
cuánta tropa tiene dentro, que es lo que decidiría un ataque.

### 2.3 El conocimiento por contacto no caduca, y se refresca al volver a comerciar

Decisión: **no caduca; cada trueque nuevo actualiza la foto.**

No hace falta elegir un plazo de caducidad —que sería una constante más sin medir— porque **el dato viejo ya
se delata solo**: la foto viaja con el instante en que se tomó, y la interfaz puede decir "última información:
hace 3 horas". El jugador juzga si fiarse.

Borrar información que el jugador ya sabe se siente arbitrario; dejarla envejecer a la vista, no.

### 2.4 Lo que ya estaba decidido y no se toca

- **Se muestra el ÚLTIMO ESTADO CONOCIDO, no el actual** (2026-08-24, doc de arquitectura 6 §3). Es el patrón
  de los RTS y lo que impide que un solo trueque dé telemetría en vivo para siempre.
- **Un aliado ve lo que ves tú, EN VIVO** — no "último conocido". La alianza es cooperación explícita.

## 3. El modelo que sale de esas decisiones

Tres fuentes de visibilidad, y cada una entrega un tipo de dato distinto:

| Fuente | Qué habilita | Datos | Frescura |
|---|---|---|---|
| **Espacial** | Lo que cae en tu zona + margen, o en la vista de un ejército tuyo | Ficha (2.2) | **En vivo** |
| **Contacto** | Un asentamiento con el que has comerciado | Ficha (2.2) | **Congelada**, con su instante |
| **Alianza** | Todo lo que ven tus aliados | Lo que el aliado vea | **En vivo** |

La distinción clave, y la que evita el error fácil: **lo que ves AHORA es en vivo; lo que RECUERDAS es una
foto.** Un asentamiento rival dentro de tu zona se proyecta en vivo porque lo estás mirando; uno con el que
comerciaste hace tres horas y está al otro lado del mapa, congelado.

Cuando las dos fuentes coinciden, gana la de en vivo: estar mirándolo es mejor información que recordarlo.

## 4. Representación en el motor

### `ConocimientoJugador` — la única entidad nueva

```
ConocimientoJugador {
  jugadorId: string
  asentamientos: Record<asentamientoId, FichaConocida>
}

FichaConocida {
  asentamientoId, nombre, faccionId, posicion, nivel
  conocidoEn: Instante        // cuándo se tomó la foto
}
```

Vive en `GameSessionState`, como `jugadores`. Un jugador sin registro no conoce nada — igual que un jugador
sin registro usa el Liderazgo base, así que **las partidas guardadas no necesitan migración**.

**Solo guarda lo de CONTACTO.** Lo espacial no se guarda: se deriva en cada proyección de dónde están tus
asentamientos y tus ejércitos, exactamente como ya se hace con `ejercitosAvistados`. Guardar lo que se puede
derivar es la clase de error que este proyecto lleva evitando desde el principio.

### En la proyección

`ProyeccionJugador` gana un campo, hermano del que ya existe para ejércitos:

```
asentamientosAvistados: FichaAsentamiento[]   // en vivo, por espacio o alianza
asentamientosConocidos: FichaConocida[]       // congelados, por contacto
```

Dos arrays y no uno, por el mismo motivo que se separaron `ejercitos` y `ejercitosAvistados`: **la diferencia
entre "lo estoy viendo" y "lo recuerdo" es de tipo, no de un campo opcional que el cliente pueda olvidarse de
mirar.** Y solo el segundo lleva `conocidoEn`, porque solo el segundo puede estar viejo.

## 5. Plan de ejecución

- [ ] **Paso 1 — Visión espacial de asentamientos ajenos.** El margen (`radioVisionAsentamiento`) y el filtro
      en `proyectarParaJugador`, reutilizando `seVeAhora` que ya existe para ejércitos. Sin memoria todavía:
      esto solo añade "veo lo que tengo delante". Es el paso que más juego añade por menos código.
- [ ] **Paso 2 — `ConocimientoJugador` en el estado**, con su migración de snapshot (aunque sea vacía) y el
      registro al cerrarse un trueque. Todavía no se proyecta: solo se acumula.
- [ ] **Paso 3 — Proyectar lo conocido**, con `conocidoEn`, y la regla de que lo visto en vivo gana sobre lo
      recordado.
- [ ] **Paso 4 — Visión compartida por alianza.** En vivo, y solo mientras la alianza esté activa: al
      romperse, lo que se veía por ella pasa a ser recuerdo (con la fecha de la ruptura) o desaparece —
      **decidir al llegar**, no antes.
- [ ] **Paso 5 — Render**, en los dos clientes. Espejo de lo que ya se hizo con los ejércitos: el de
      administración lo ve todo, el de jugador solo lo suyo y lo avistado, y lo recordado se pinta distinto de
      lo visto.
- [ ] **Paso 6 — Calibración** del margen (2.1) contra la vista de ejército, con la nota del nivel 5.

## 6. Invariantes a congelar en tests

1. Un asentamiento rival **fuera** de zona+margen y de la vista de todo ejército propio **no aparece en
   absoluto** en la proyección — ni redactado.
2. Lo proyectado de un rival **nunca** incluye almacén, escuadrones, edificios ni colas. Ni en vivo ni
   recordado.
3. Un asentamiento visto en vivo y además conocido por contacto aparece **una sola vez**, y como visto.
4. La foto congelada **no cambia** cuando cambia el asentamiento real; solo cuando hay un contacto nuevo.
5. Un jugador sin registro de conocimiento proyecta listas vacías, nunca `undefined`.
