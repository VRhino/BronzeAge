# Mundo reactivo en Rust: un mundo con reloj propio, sin tick

> **Estado (2026-10-12): diseño del núcleo de la reconstrucción, ACORDADA.** El servidor se reconstruye entero en Rust
> sobre este modelo, sin fases ni convivencia con el servidor TypeScript, sobre `main` de
> `BronzeAgeFase0`. Problemas, decisiones y orden: [`Consideraciones/Reconstruccion_Definicion.md`](../../Consideraciones/Reconstruccion_Definicion.md).
> Qué cambia para los clientes: [`Docs/Coordinacion/04_Migracion_Mundo_Reactivo.md`](../Coordinacion/04_Migracion_Mundo_Reactivo.md).
>
> Origen: conversación con el usuario (sesión «Cliente jugador», 2026-10-11) tras medir el movimiento del mapa con
> el contrato 03. Con un tick de 500 ms (valor de prueba) basta interpolar un tick por detrás; con el tick real de
> **un minuto** eso sería ver la propia columna con más de un minuto de retraso, y extrapolar falla al salir, al
> cambiar de rumbo y al cambiar de terreno. La raíz no está en el cliente: el servidor solo conoce posiciones
> discretas, una por tick.

## 1. La idea central: el mundo no se actualiza, se describe

Cada cosa guarda su estado **como una función del tiempo** y el servidor solo trabaja cuando algo cambia de verdad:
una orden, una llegada, un encuentro, un almacén que se llena, una despensa que se vacía. Entre esos momentos no hay
nada que calcular. El cliente recibe la función (el plan), no muestras sueltas, y la pinta a los fps de su pantalla.

Separación limpia:

- **Gestión del mundo** (servidor): reloj de mundo real, cola de eventos, reglas. Sin fps.
- **Presentación** (cliente): `requestAnimationFrame` al ritmo del monitor, evaluando planes y ritmos en local.

## 2. Modelo de tiempo: simulación por eventos discretos

- **Reloj de mundo** = reloj de pared × factor (×1 en producción, ×N en pruebas). No hay tick.
- **Cola de eventos** con instante absoluto (montículo o rueda de temporizadores jerárquica). Cada evento lleva la
  **generación** de su entidad: si la entidad cambia de plan, sube la generación y los eventos viejos se descartan
  al salir de la cola (invalidación perezosa, sin buscar ni borrar).
- **Orden total y determinista:** los eventos se aplican por `(instante, secuencia)`; a igual entrada, igual mundo.

### 2.1 Estado analítico

| Cosa | Se guarda | Se calcula al vuelo | Eventos que programa |
|---|---|---|---|
| Columna, caravana, Aeda | **plan**: vértices de la ruta con el instante de paso por cada uno | `posición(t)` interpolando el plan | llegada, fin de víveres, cambio de plan |
| Almacén y producción | cantidad en t₀ y **ritmo** neto por recurso | `min(tope, c₀ + ritmo·(t − t₀))` | almacén lleno, recurso a 0 |
| Obra, cola, investigación | instante de fin | progreso = (t − inicio) / duración | terminada |
| Población, moral, reputación | valor en t₀ y ritmo | ídem | umbrales |
| Mirada de taberna, propuesta, veto | instante de caducidad | — | caduca |

### 2.2 Planes de movimiento calculados de una vez

La velocidad depende del terreno, el bosque y los caminos (`engine/movimiento.ts` hoy: avance = velocidad /
(coste del punto × factor de camino)), y todo eso se conoce a lo largo de la ruta. Al dar la orden, la ruta se parte
donde cambia el coste y se calcula el instante de paso por cada vértice: **la columna sabe a qué hora pasa por cada
punto desde que sale.** El plan solo se rehace por un hecho: la orden cambia, la tropa muere (cambia la velocidad
mínima), se construye un camino en el trayecto, se engancha una caravana, empieza una batalla.

Consecuencias: «llega a las 14:32» es exacto; el cliente pinta sin estimar; una columna en marcha no cuesta nada
mientras anda (ni CPU ni red).

## 3. Lo continuo que interactúa: encuentros y visión

Es lo difícil de un mundo sin tick. Dos niveles:

- **Lo que cambia el juego** (encuentros, emboscadas, llegar a la puerta de una plaza, interceptar una caravana):
  **exacto, con eventos.** Dos planes lineales por tramos se acercan a distancia `r` en un instante que sale de una
  cuadrática por par de tramos solapados en el tiempo. Para no comparar todo con todo, cada tramo se apunta en una
  **rejilla espacio-temporal** (celda × ventana de tiempo) y solo se cruzan los candidatos que comparten celda. Al
  cambiar un plan se quitan sus apuntes, se meten los nuevos y se recalculan solo esos pares.
  - **Persecución:** el servidor conoce el plan de la presa, calcula el punto de intercepción y lo recalcula cuando
    la presa cambia de plan (un evento, no un sondeo).
- **Lo informativo** (qué ve cada Facción, la niebla explorada, la memoria de plazas): **muestreado en paralelo**,
  p. ej. cada 250 ms, sobre la misma rejilla. Unos 10.000 móviles por unas 100-300 claves de visión son milisegundos
  en un pool de hilos; no hace falta el instante exacto en que algo entra en tu vista.

## 4. Hilos: un núcleo de un solo escritor y paralelismo alrededor

```text
          ┌─────────────── tokio (N hilos de E/S) ───────────────┐
clientes ─┤ WS/HTTP · auth · parseo y validación sin estado       │
          └───────────────┬──────────────────────▲───────────────┘
                 órdenes  │ (canal MPSC)          │ mensajes serializados (Bytes compartidos)
                          ▼                       │
   ┌──────────── NÚCLEO (1 hilo, determinista) ────────────────┐
   │ cola de eventos · aplica órdenes y eventos en (t, seq)     │
   │ arenas de entidades (slotmap) · sin locks                  │
   └───┬─────────────────┬──────────────────┬───────────────────┘
       │ peticiones      │ registro de      │ lote de cambios por intervalo
       ▼                 ▼ cambios          ▼
  rayon: rutas A*,   hilo de             rayon: visión por clave de Facción,
  combate,           persistencia        diffs por canal, serialización
  intercepciones     (log + fotos)       una vez por grupo → tokio
```

- **Por qué un solo hilo en el núcleo:** cada evento cuesta microsegundos; 1.000 jugadores dan del orden de cientos
  a pocos miles de eventos por segundo y un hilo aguanta del orden del millón. A cambio: sin locks, sin carreras,
  reproducible y depurable.
- **Lo caro, fuera y en paralelo:** rutas (A* sobre el grafo de navegación), visión, diffs y serialización por
  grupo, resolución de combate, persistencia. Lo que vuelve de un worker (p. ej. una ruta) entra al núcleo como un
  evento más **y se escribe en el log**, para que una reproducción no dependa de qué hilo terminó antes.
- **Partir el núcleo por regiones** (simulación paralela de eventos con sincronización conservadora en los bordes)
  solo si una medición lo exige. Las entidades se diseñan con afinidad a una región para poder hacerlo sin rehacer.
- **Números:** posiciones y tiempos en **punto fijo** (`i64`), no `f64`, para que las reproducciones sean idénticas
  en cualquier máquina.
- **Crates orientativos:** `tokio`, `axum` o `tokio-tungstenite`, `rayon`, `crossbeam-channel`, `slotmap`, `bytes`,
  `prost` (Protobuf, el formato del contrato 05, común con C# y TypeScript) + `zstd`, `tracing`, `mimalloc`.

## 5. Protocolo con los clientes: planes y ritmos en vez de posiciones

- **Lo propio:** el plan completo una vez (vértices con su instante). El cliente evalúa `posición(t)` en cada
  fotograma, sin tráfico mientras la columna anda. Un cambio de plan llega como plan nuevo que **empieza donde está
  la columna**: ni parada ni salto.
- **Lo ajeno:** el plan entero revelaría el destino (intención, hoy protegida en 02 §4.1). Se manda **solo el tramo
  dentro de tu vista y con horizonte corto** (p. ej. los próximos 30 s), renovado: fluido y sin filtrar intención.
- **Economía, obras, colas:** «cantidad en t₀ + ritmo» y «termina en t». El cliente anima barras y contadores; el
  servidor no envía nada hasta que un ritmo cambia.
- **Se conserva del contrato 03:** fotos + deltas por canal, agrupación por clave de visión, serialización única por
  grupo, `resync`/`revocado`. Lo que cambia es lo que viaja dentro: planes y ritmos en vez de progreso por tick.

## 6. Persistencia y recuperación

- **Event sourcing:** log de solo añadir (órdenes + resultados de workers) y una **foto periódica** del estado, hecha
  sin parar el núcleo (estructuras persistentes o clon rápido serializado por otro hilo).
- **Arranque:** última foto + repetición del log. Como el estado es analítico, «ponerse al día» tras una caída es
  procesar los eventos pendientes, no simular los minutos perdidos.

## 7. Escala orientativa (1.000 jugadores)

- 10-20.000 móviles con los bots, miles de plazas con ritmos: decenas de MB de estado; núcleo casi siempre ocioso.
- El cuello de botella es el envío a los clientes, no la simulación: con planes, una columna cuesta un mensaje por
  orden, no uno por tick; visión y serialización en paralelo y por grupo.

## 8. Riesgos y decisiones abiertas

1. **Reglas que hoy dependen del tick** (consumo de víveres, deserción por hambre, mantenimiento, producción por
   tick, bandidos, catch-up): hay que reescribirlas como ritmos + eventos de umbral. Es el grueso del trabajo y es
   **trabajo de reglas**: cada una se valida contra `Docs/Game/` y las simulaciones batch antes de darla por buena.
2. **Órdenes a mitad de camino:** en un mundo continuo se aplican desde la posición real del instante de la orden
   (hoy, desde la del inicio del tick). Cambia sutilmente el resultado de algunas jugadas; decisión de diseño.
3. **Combate de Unity:** su ciclo (`BA-001`) ya es por eventos; encaja, pero el contrato de inicio/fin de batalla se
   revisa al migrar.
4. **Bots** (`12_NPC_Fuera_Del_Motor.md`): siguen siendo clientes externos, sin privilegios, pero se reescriben en Rust
   como servicio aparte que usa el contrato 05 igual que un jugador.
5. **Herramientas de batch y balance** (`scripts/run-batch-sim.ts`, mediciones): necesitan un modo «acelerado» del
   reloj de mundo (procesar la cola sin esperar al reloj de pared).

## 9. Relación con lo que existe

- `typescript-final:Docs/Arquitectura/10_Modelo_Temporal.md` (tick = minuto de mundo) describe el servidor actual; este documento lo sustituye.
- `typescript-final:Docs/Arquitectura/6_Sincronizacion_Visibilidad_y_Escala.md` y el contrato 03 aportan el envoltorio de la entrega (canales, grupos,
  backpressure), que el contrato 05 conserva.
