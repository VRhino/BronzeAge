# Estudio de evolución: backend autoritativo, múltiples frontends y conversión temporal

> **Documento de estudio (agosto de 2026).** Su plan se ejecutó en las fases A–E ([doc 3](3_Plan_Evolucion_Roadmap.md))
> y el estado vive en [1_Arquitectura_Actual.md](1_Arquitectura_Actual.md). Dos cosas de aquí ya no valen: la
> «independencia del tamaño del paso» (retirada, [doc 10](10_Modelo_Temporal.md) §1) y la entidad `Jugador` del
> punto 1, sustituida por la `Membresia` más el `Heroe` del juego (2026-09-14).

## Objetivo

Transformar el motor de simulación en un servicio de servidor autoritativo que mantenga partidas persistentes y permita conectar varios frontends simultáneamente:

- Un frontend de jugador por usuario, limitado a sus permisos y a la información que puede conocer.
- Un frontend de administración, con funciones de operación, moderación, inspección y configuración autorizada.
- Opcionalmente, clientes de observación, herramientas de balance, simulación batch o integración con Unity.

El servidor debe ser la única autoridad que acepta comandos, avanza la simulación, ejecuta NPCs, genera resultados aleatorios, conserva el estado y comunica actualizaciones a los clientes. Los frontends no deben modificar ni poseer la verdad de la partida.

La estrategia adoptada es incremental: primero se construirá el backend multijugador utilizando el motor de ticks existente; la conversión del motor a tiempo real total se hará posteriormente. El sistema de ticks será una etapa provisional para validar servidor, persistencia, red, identidad y permisos, no el modelo temporal definitivo.

## Principios de diseño

1. **Servidor autoritativo.** Un cliente solicita una intención; nunca envía un estado modificado como fuente de verdad.
2. **Partida aislada.** Cada partida tiene `gameId`, estado, configuración, reloj, cola de comandos y persistencia propios.
3. **Actor autenticado.** Toda acción se evalúa con la identidad de la sesión, no con un `jugadorId` elegido libremente por el cliente.
4. **Autorización en servidor.** El servidor comprueba rol, facción, asentamiento, cargo y visibilidad antes de ejecutar una acción.
5. **Procesamiento serial por partida.** Los comandos y ticks de un mismo `gameId` se aplican en un orden único para evitar carreras sobre recursos, población, nodos o caravanas.
6. **Estado persistente y recuperable.** Un reinicio de proceso no puede perder una partida ni alterar su secuencia aleatoria.
7. **Sincronización por proyecciones.** Cada cliente recibe solo la representación de estado adecuada a su rol y visibilidad.
8. **Contratos versionados.** API, comandos, eventos, snapshots y migraciones de datos deben tener versión.

## Arquitectura objetivo

```text
Frontend de jugador ─┐
Frontend admin      ├─ HTTP: consultas y comandos
Observador/herramienta ┘  WebSocket/SSE: actualizaciones
                         |
                         v
                 API / autenticación
                         |
                         v
             Autorización y validación de comando
                         |
                         v
       Backend dedicado de una partida (cola serial)
          |       |          |             |
          |       |          |             +-- NPCs y scheduler (ticks provisionales)
          |       |          +-- motor actual: engine/world/worldgen
          |       +-- generador de IDs y RNG de partida
          +-- eventos, snapshots y control de versión
                         |
                         v
                   Base de datos / almacenamiento
                         |
                         v
       Proyecciones por jugador, facción y administrador
                         |
                         v
                Difusión de cambios a clientes conectados
```

## Modelo operativo de una acción

```text
Cliente: comando intencional + idempotencyKey
  -> API valida formato y sesión
  -> servidor resuelve actor y gameId
  -> autorización de rol, pertenencia y visibilidad
  -> cola/transaction de la partida
  -> aplicación de una función del motor
  -> persistencia atómica de estado, eventos y versión
  -> creación de proyecciones afectadas
  -> respuesta al cliente y difusión de actualización
```

Ejemplo: el cliente puede pedir “reclutar tropa en asentamiento X”, pero el servidor determina qué usuario lo solicita, a qué `jugadorId` corresponde, si reside allí, si puede operar ese asentamiento y si la información o acción es visible para él.

## Qué se puede reutilizar

Se puede conservar, con cambios contenidos, la mayor parte de:

- `domain/types.ts` como base de los datos de juego.
- `engine/` como núcleo de reglas.
- `worldgen/` para crear mundos por semilla y versión.
- `world/` para consultas espaciales y rutas; la fachada `Mapa` debe reconstruirse desde datos persistidos.
- Las pruebas del motor como red de seguridad para la extracción.
- La UI actual como punto de partida del futuro frontend de administración, sustituyendo `GameStore` por un cliente de API.

No se debe reutilizar `GameStore` como servicio tal cual: hoy contiene decisiones de sesión local, notificación de UI, historial de depuración, traducción de errores y estado singleton.

## Trabajo necesario

### 1. Definir contratos de partida, identidad y permisos

Definir entidades externas al motor:

- `Usuario`: cuenta autenticable.
- `Sesion`: credenciales, caducidad y contexto de acceso.
- `Jugador`: identidad dentro de una partida; relación con usuario y facción.
- `Rol`: jugador, administrador de partida, moderador, administrador global, observador, servicio NPC.
- `Partida`: `gameId`, estado, configuración, versión de reglas, semilla, estado del RNG, tick y estado del ciclo de servidor.
- `Membresia`: relación usuario-jugador-partida-facción con vigencia y permisos.

Documentar una matriz de autorización por comando. Los cargos del juego (rey, gobernador, tesorero, etc.) son reglas de dominio; los roles técnicos de administración no deben confundirse con ellos.

### 2. Extraer una sesión de partida de la UI

Crear una capa de aplicación de servidor, por ejemplo `GameSession` o `GameApplicationService`, que reciba una partida concreta y ejecute comandos.

Responsabilidades que se trasladan desde `GameStore`:

- Cargar/crear y guardar una partida.
- Asignar IDs de manera persistente.
- Invocar las reglas del motor.
- Ejecutar ticks y NPCs.
- Convertir errores de dominio a resultados de comando estructurados.
- Emitir eventos de aplicación.

Mantener fuera de esta capa el DOM, `subscribe()` local, `Blob`, `FileReader`, Canvas y formatos específicos de pantalla.

### 3. Hacer el estado plenamente reproducible

La partida debe almacenar, como mínimo:

- Configuración y versión de reglas/worldgen.
- Semilla y estado del generador aleatorio de simulación.
- Contador o generador de IDs.
- Tick, tiempo de próximo tick y ciclo de servidor.
- Estado del mapa mutable y todas las entidades de dominio.

Sustituir gradualmente los usos de `Math.random()` del motor por un RNG inyectado o contenido en el contexto de partida. Así una partida puede reiniciarse, auditarse y reproducirse exactamente.

También conviene hacer explícitas las mutaciones de `EstadoMapa` que hoy ocurren a través de `Mapa`, para que formen parte clara del resultado persistible de un comando o tick.

### 4. Diseñar persistencia

Como punto de partida, usar una base de datos relacional o documental que permita guardar una partida con control de versión. Hay dos estrategias compatibles:

- **Snapshots:** guardar el estado completo cada cierto número de ticks y al finalizar comandos relevantes. Es simple y adecuado para empezar.
- **Eventos/comandos más snapshots:** guardar cada comando/tick aceptado como registro auditable y crear snapshots periódicos. Facilita historial, replay, depuración, auditoría y recuperación.

Cada escritura debe incluir una versión de concurrencia. Si dos procesos intentan modificar la misma partida, solo uno puede confirmar la siguiente versión.

El historial visual no debe conservar clones ilimitados en RAM. Debe pasar a una política de snapshots, retención y/o replay desde eventos.

### 5. Añadir scheduler y concurrencia por partida

El servidor debe ejecutar el tick según una política explícita: manual para pruebas, intervalo configurable para partidas de desarrollo o calendario real para servidores persistentes.

Para cada `gameId`, procesar de forma serial:

1. Comandos aceptados en orden.
2. Tick programado.
3. Decisiones NPC asociadas al tick.
4. Persistencia y difusión.

Una primera implementación puede mantener una cola en memoria por partida. Para escalar a varios procesos será necesario un bloqueo distribuido, una cola de trabajo particionada por `gameId` o transacciones optimistas con reintentos.

### 6. Exponer API de comandos y consultas

Separar claramente dos tipos de operaciones:

- **Comandos mutables:** fundar, construir, reclutar, comerciar, nombrar cargos, combatir, avanzar tick administrativo, etc.
- **Consultas de solo lectura:** vista del mapa, detalle de asentamiento, mercado, historial visible, estado administrativo y configuración.

Los comandos deberían devolver un resultado estructurado: éxito/rechazo, código de error de dominio, versión de partida y eventos producidos. Los logs localizados son una presentación; no deben ser el único contrato entre servidor y frontend.

Usar HTTP para carga inicial, consultas y comandos normales. Usar WebSocket o Server-Sent Events para notificar nuevos ticks, resultados de comandos y cambios de estado a otros clientes conectados.

### 7. Construir proyecciones de estado por audiencia

No enviar `GameState` completo por defecto. Crear DTOs o proyecciones para:

- Jugador: sus activos, información pública y datos visibles por reglas de exploración/visibilidad.
- Facción: información compartida autorizada por pertenencia y cargo.
- Administración: estado completo, herramientas de configuración y auditoría.
- Observador: información pública o explícitamente concedida.

Aunque la niebla de guerra aún no esté implementada, esta separación debe existir desde el primer API para evitar que un cliente pueda inspeccionar datos futuros u ocultos.

### 8. Separar administración y balance

El frontend de administración puede partir de la UI actual, pero sus funciones deben protegerse con roles técnicos. Acciones como regenerar mundo, importar/exportar, cambiar balance, alternar NPC, avanzar ticks manuales o inspeccionar todo el estado no deben estar disponibles en el frontend de jugador.

El balance debe pasar de módulo mutable global a configuración versionada por partida o por temporada. Cambiarlo debe dejar auditoría, actor, fecha, versión anterior y versión nueva.

### 9. Adaptar los frontends

- Reemplazar el singleton `gameStore` del navegador por un cliente de API que gestione caché local, reconexión y suscripciones.
- Conservar el estado puramente visual en cada frontend: pestañas, selección, filtros, zoom, cachés de dibujo y formularios.
- Implementar primero el frontend administrativo, porque refleja casi toda la funcionalidad existente y facilita validar el backend.
- Crear después una interfaz de jugador que exponga solo acciones y datos autorizados.

### 10. Seguridad, operación y calidad

Incluir desde el diseño:

- Autenticación segura, expiración de sesión y protección de endpoints.
- Validación de entrada en API además de la validación de dominio.
- Limitación de frecuencia e idempotencia para comandos que puedan repetirse por reconexión.
- Logs estructurados con `gameId`, actor, comando, resultado y versión.
- Métricas de duración de tick, tamaño de cola, errores y clientes conectados.
- Backups, migraciones y pruebas de restauración de partidas.
- Pruebas de autorización, concurrencia, persistencia y reconexión, además de las pruebas actuales del motor.

## Riesgos principales y mitigaciones

| Riesgo | Consecuencia | Mitigación |
|---|---|---|
| Confiar en IDs enviados por el cliente | Acciones no autorizadas | Resolver actor y permisos exclusivamente en servidor. |
| Dos comandos simultáneos | Doble gasto o estado inconsistente | Cola serial o control de versión por partida. |
| `Math.random()` no persistido | Partidas no reproducibles tras reinicio | RNG determinista con estado guardado. |
| Enviar estado completo | Filtración de información de juego | DTOs/proyecciones por audiencia desde el servidor. |
| Historial en RAM sin límite | Consumo creciente de memoria | Snapshots y retención persistentes. |
| Balance global mutable | Partidas afectadas entre sí | Configuración versionada y ligada a partida/temporada. |
| Mezclar administración y jugador | Privilegios excesivos | Frontends y endpoints separados con roles técnicos. |

## Decisión final de orden de transformación

La conversión temporal completa se pospone hasta después de validar el backend y el multijugador. El orden definitivo es:

```text
1. Separación mínima de motor y UI
2. Backend de una partida sobre el motor de ticks actual
3. Persistencia, comunicación, identidad y permisos
4. Frontends de administración y jugador sobre ese backend
5. Conversión del motor de ticks a tiempo real total
6. Migración temporal de partidas, DTOs y balance
```

La razón es reducir el número de cambios simultáneos. Si se convierte primero el motor y se construye después el servidor, se validan demasiadas variables a la vez. Si se construye primero el servidor con ticks, la infraestructura multiusuario queda probada y la conversión posterior se concentra en el núcleo temporal.

Esta estrategia solo es segura si el tick se trata como un detalle provisional. Las nuevas capas no deben depender de él como contrato definitivo.

### Contratos que deben ser independientes de ticks desde el inicio

- El servidor debe distinguir reloj de servidor, tiempo de simulación, scheduler y unidad interna del motor.
- La API no debe exponer solo `ticksRestantes`; debe poder evolucionar a `completaEn`, duración o eventos temporales.
- La persistencia debe guardar, cuando exista, fecha de inicio, fecha de finalización, duración y versión temporal, además de cualquier contador provisional.
- El frontend no debe calcular fechas a partir de ticks.
- La API no debe definir “pasar tick” como única operación temporal definitiva.
- Los eventos y resultados deben incluir una versión temporal para permitir la migración.

## Alcance de la etapa provisional de backend

Cada partida será una instancia backend dedicada. Todos los jugadores y el administrador de esa partida se conectarán al mismo proceso o despliegue, que tendrá:

- Un estado único.
- Una única secuencia de ticks.
- Una `GameSession` autoritativa.
- Una cola serial de comandos.
- Persistencia propia.
- RNG y contador de IDs propios.
- NPCs y scheduler propios.

El servidor podrá conservar un avance manual de tick para pruebas, pero no debe ser la autoridad normal de producción. La infraestructura ya debe soportar reinicio, recuperación, reconexión, comandos idempotentes y difusión por WebSocket/SSE.

## Conversión posterior a tiempo real total

El motor actual resuelve:

```text
estado en tick N -> estado en tick N + 1
```

El motor definitivo resolverá:

```text
estado + ahora + tiempo transcurrido -> nuevo estado + eventos temporales
```

No será suficiente con cambiar el scheduler. Será necesario modificar el modelo temporal de las reglas:

- Contadores como `ticksRestantes` pasarán a fechas de finalización o duraciones.
- `fundadoEnTick`, `heridoHastaTick`, `regeneraEnTick` y cooldowns pasarán a tiempo de simulación.
- Producción, consumo, población, hambre, mantenimiento y moral se expresarán como tasas o acumuladores.
- Construcción, políticas, caravanas, bandidos, NPC y regeneración utilizarán eventos temporales.
- El movimiento de caravanas se derivará de salida, velocidad y tiempo transcurrido.
- El motor será independiente del tamaño del paso: 60 segundos de una vez equivaldrán a seis intervalos de 10 segundos.
- La aleatoriedad se asociará a eventos estables y no a la frecuencia del scheduler.
- Se definirá la semántica de comandos inmediatos, programados y condicionales.
- Se persistirá `ultimoProcesadoEn` y se procesarán eventos vencidos después de un reinicio.

Los sistemas con mayor impacto serán producción/consumo, población, hambre, mantenimiento, caravanas y bandidos. También cambiarán los DTOs y los temporizadores de los frontends.

## Fases finales corregidas

### Fase A: núcleo reutilizable

1. Estabilizar las pruebas existentes.
2. Extraer la ejecución del motor de la UI.
3. Introducir contexto de partida para RNG e IDs.
4. Separar eventos estructurados de mensajes de presentación.

### Fase B: backend provisional con ticks

1. Crear `GameSession` para una partida.
2. Crear proceso backend Node.js y repositorio de partida.
3. Persistir estado, tick, RNG, IDs, configuración y eventos.
4. Exponer API administrativa.
5. Conectar la interfaz actual como cliente remoto.

### Fase C: multijugador sobre ticks

1. Añadir usuarios, membresías y roles.
2. Validar comandos según actor, facción, asentamiento y cargo.
3. Añadir WebSocket/SSE, reconexión e idempotencia.
4. Servir proyecciones por audiencia (nunca el estado completo a un jugador). El frontend de jugador en sí
   queda fuera del alcance de este repo — se construye en un repo de interfaz aparte.

### Fase D: conversión temporal total

1. Introducir reloj de simulación y fechas sin eliminar aún la infraestructura de servidor.
2. Migrar construcción, políticas y cooldowns.
3. Migrar producción, consumo, población, hambre y mantenimiento.
4. Migrar caravanas, comercio, bandidos, NPC y combate.
5. Crear migración explícita de partidas de ticks a tiempo real.
6. Cambiar DTOs y frontends de contadores a fechas, duraciones y eventos.
7. Recalibrar el balance con simulaciones de referencia.

### Fase E: operación persistente

1. Scheduler temporal definitivo y recuperación de eventos vencidos.
2. Auditoría, snapshots, backups y restauración.
3. Métricas y herramientas de moderación.
4. Ciclos de servidor, Maravilla, legado NPC y temporadas.

## Criterios de éxito adicionales

Antes de la conversión temporal debe ser posible ejecutar una partida persistente en un backend dedicado, conectar varios jugadores y un administrador, procesar comandos de forma serial y recuperar el servidor sin alterar la secuencia de ticks.

La transición completa solo estará terminada cuando esa misma infraestructura opere con tiempo real total, sin que el motor dependa de un paso global fijo y sin que API, persistencia o frontends tengan que seguir tratando el tick como unidad temporal principal.
