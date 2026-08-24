# Arquitectura actual del proyecto

## Propósito y alcance

El proyecto es un prototipo web de la Fase 0 de Bronze Age Collapse. Ejecuta una simulación de datos en TypeScript y ofrece una interfaz de depuración en navegador con DOM y Canvas 2D. Su objetivo actual es validar reglas de mundo, economía, población, construcción, política, comercio y combate; no es todavía un servidor multijugador.

La aplicación se construye con Vite y TypeScript. No incorpora framework de interfaz, backend, base de datos, autenticación ni capa de red.

## Vista global

```text
Navegador
  src/main.ts                 Interfaz DOM, eventos y estado visual local
  src/ui/canvas.ts            Render Canvas 2D, exclusivamente presentacional
          |
          v
  src/app/gameStore.ts        Sesión única, acciones, consultas, log e historial
          |
          v
  src/engine/                Reglas y casos de cálculo de la simulación
  src/world/                 Fachada y consultas del mapa mutable
  src/worldgen/              Generación determinista del mundo
          |
          v
  src/domain/types.ts         Tipos de datos del dominio
  src/constants.ts            Catálogos y parámetros de balance
```

La dirección principal de dependencias es correcta: la UI depende de `app`; `app` depende de motor, mundo y dominio; el motor depende de mundo, dominio y constantes. El motor no depende de la UI ni de APIs del navegador.

## Capas y responsabilidades

### Dominio: `src/domain/types.ts`

Contiene los contratos de datos centrales: facciones, asentamientos, edificios, población, escuadrones, caravanas, mercado, relaciones políticas, recursos, nodos de mapa y geometría básica.

Los datos identifican entidades mediante cadenas (`id`, `faccionId`, `asentamientoId`, `jugadorId`). Los jugadores son actualmente identidades del dominio de juego; no representan usuarios autenticados del sistema.

### Configuración: `src/constants.ts`

Contiene catálogos y parámetros de reglas: recursos, edificios, economía, población, construcción, combate, política, mundo y balance. El módulo `app/balanceConfig.ts` permite cambiar algunos valores en caliente desde la interfaz de administración actual.

Esta configuración es global para el proceso o la pestaña actual; no está versionada ni asociada a una partida concreta.

### Generación de mundo: `src/worldgen/`

Genera un `MapaGenerado` a partir de una configuración y una semilla. Incluye elevación, fertilidad, biomas, ríos, bosques, nodos de recurso, regiones y chokepoints. La generación usa un RNG propio y es reproducible si se conserva la versión del algoritmo y la semilla.

El mundo generado se considera inmutable durante la partida salvo sus recursos agotables, cuyo consumo se mantiene separado en `EstadoMapa`.

### Mundo y consultas espaciales: `src/world/`

`MapaGenerado` y `EstadoMapa` son datos serializables. La clase `Mapa` es una fachada en memoria que construye índices y ofrece consultas espaciales, stock de nodos, extracción y regeneración. `GameStore` cachea esta fachada mediante `WeakMap`; no la guarda dentro del estado exportable.

También incluye geometría, rutas y exportación de terreno para Unity. La exportación Unity es una operación de lectura del estado, no una regla de simulación.

`world/poligonos.ts` resuelve la unión de siluetas que se solapan (campo de distancia con signo más marching squares, sin dependencias externas). Sus dos consumidores son puramente de presentación: `Mapa.contornosBosques()` fusiona los discos de bosque en manchas, y `engine/zones.ts` fusiona las zonas de influencia de una misma facción en una silueta por facción. Ninguna regla de simulación consulta esos contornos: la pertenencia territorial y el alcance de recursos se siguen resolviendo contra el polígono por asentamiento y el disco por bosque.

### Motor: `src/engine/`

Agrupa reglas por subsistema: fundación, expansión, zonas, construcción, trazado urbano, población, mantenimiento, almacenamiento, mercado, comercio, caravanas, facciones, cargos, diplomacia, ligas, combate, tropas, bandidos, reputación y títulos.

La función central es `avanzarSimulacion(estado, mapa, tickActual)` en `engine/simulation.ts`. Procesa el tick de forma ordenada y devuelve el nuevo estado de los subsistemas junto con eventos. La fachada `Mapa` puede mutar `EstadoMapa` por extracción y regeneración, por lo que el tick no es completamente funcional aunque su entrada y salida están bien delimitadas.

El motor está libre de dependencias de `app`, `ui` y `main`, por lo que puede ejecutarse en un proceso Node.js sin llevar código de navegador.

Hay aleatoriedad de simulación en población, combate y bandidos mediante `Math.random()`. Los tests sustituyen esa fuente para comprobar determinismo, pero su estado no forma parte de una partida guardada.

### Aplicación y sesión local: `src/app/gameStore.ts`

`GameStore` es el dueño de la sesión actual. Mantiene un único `GameState` en memoria con mundo, estado del mapa, entidades de juego, tick, log e historial por jugador.

Sus responsabilidades actuales son:

- Inicializar y regenerar el mundo.
- Convertir intenciones de UI en llamadas al motor.
- Validar y traducir errores de dominio a entradas del log.
- Mantener un contador local para generar IDs de acciones.
- Ejecutar el tick y el NPC de gobernanza.
- Exponer consultas derivadas para la UI.
- Notificar cambios a suscriptores locales.
- Conservar una copia profunda de cada tick para la línea de tiempo.
- Importar y exportar una partida como JSON.

Por ello, `GameStore` mezcla sesión, casos de uso, adaptación de errores, historial de debug y parte de la administración. Es la capa que deberá dividirse o reemplazarse al introducir un servidor.

El estado se exporta como `SimulacionExportada`. El guardado conserva la versión de formato, el tick, configuración/semilla del mundo, recursos restantes, entidades y logs. No existe persistencia automática: exportar/importar se hace mediante archivos desde el navegador.

### NPC de gobernanza: `src/app/npcGobernanza.ts`

El NPC es una automatización de aplicación, separada del tick básico del motor. Se ejecuta después del tick para las facciones marcadas en `faccionesNpcIds`. Sus decisiones reutilizan funciones públicas del motor.

El hecho de que esté aislado del motor es positivo: en un servidor puede convertirse en un agente de aplicación programado, sin contaminar las reglas puras.

### Interfaz: `src/main.ts` y `src/ui/canvas.ts`

`main.ts` construye la interfaz de administración y depuración: formularios, pestañas, controles de tick, administración de facciones, asentamientos, comercio, guerra, balance, logs, generación e importación/exportación.

Conserva estado de visualización local, como pestañas activas, selección, filtros, posición de la línea de tiempo y caché visual. Se suscribe a `gameStore` para repintar.

`ui/canvas.ts` dibuja el mapa y la vista local de asentamientos. No decide reglas, rutas, trazado ni control territorial: recibe esos datos ya resueltos.

La UI actual es una consola administrativa completa, no un frontend restringido para un jugador individual.

## Estado de partida y ciclo de un tick

```text
Evento UI
  -> método de GameStore
  -> función del motor / actualización del estado
  -> registro de eventos o errores en log
  -> clon de historial y notify()
  -> render de todos los paneles y Canvas

Botón "Avanzar tick"
  -> GameStore.avanzarTick()
  -> engine.avanzarSimulacion(...)
  -> comercio automático opcional
  -> NPC de gobernanza opcional
  -> log, historial y render
```

Los ticks avanzan manualmente al pulsar un botón. No hay planificador de tiempo real ni proceso de servidor que los ejecute.

## Persistencia, historial y observabilidad

- La partida en curso vive en memoria de la pestaña del navegador.
- El guardado es un JSON descargado manualmente; no hay base de datos.
- El historial contiene una copia profunda del estado por tick dentro de `GameStore` para la línea temporal. Es útil para depuración, pero su coste de memoria aumenta con número de ticks y tamaño de partida.
- Los eventos son mensajes de texto, sin esquema de evento estable, sin auditoría de actor y sin almacenamiento externo.

## Comunicación, identidad y permisos

No existe una interfaz de comunicación entre procesos o clientes:

- No hay HTTP, WebSocket, Server-Sent Events ni RPC.
- No hay servidor ni instancias de partida identificadas por `gameId`.
- No hay cuentas, sesiones, autenticación ni autorización.
- No hay asociación entre un usuario externo y un `jugadorId` de dominio.
- No hay filtrado de datos por facción, jugador, rol administrativo o niebla de guerra.

Las acciones de `GameStore` reciben directamente IDs que provee la UI. Esto es válido para una herramienta local, pero no puede considerarse seguro en un entorno cliente-servidor: el servidor tendrá que determinar el actor autenticado y comprobar sus permisos antes de aplicar cada comando.

## Fortalezas para una futura evolución

- El motor de reglas está separado de la UI y no necesita DOM.
- El modelo de estado es explícito y mayoritariamente serializable.
- La generación de mundo tiene semilla y versión.
- Existen pruebas extensas para reglas y regresiones.
- La UI no invoca directamente el motor; ya usa una frontera de aplicación (`GameStore`).
- Las entidades usan IDs estables, adecuados para almacenamiento y mensajes de red.

## Limitaciones actuales

- Sesión única en memoria y singleton `gameStore`.
- Ausencia total de backend, transporte y persistencia servidor.
- UI administrativa con acceso al estado completo.
- Falta de identidad y autorización.
- Balance mutable globalmente.
- Aleatoriedad no persistida ni inyectada por partida.
- Historial completo en memoria sin política de retención.
- Algunos métodos de aplicación mezclan decisiones de presentación, mensajes de log y lógica operativa.

## Decisión de evolución adoptada

La conversión temporal completa no se realizará como primer paso. La primera etapa será construir el backend multijugador sobre el motor de ticks actual y validar servidor, persistencia, comunicación, identidad y permisos.

Durante esa etapa provisional:

- Cada partida será una instancia backend dedicada.
- Todos los jugadores conectados a esa instancia compartirán el mismo estado y tick.
- El servidor seguirá siendo quien avance y resuelva los ticks.
- El frontend podrá ser sustituido sin trasladar la autoridad de la partida al navegador.

La conversión posterior será de ticks discretos a tiempo real total. Esta decisión reduce el número de cambios simultáneos, pero exige no convertir el tick en un contrato permanente de infraestructura.

Desde el principio deben quedar separados:

- El reloj del servidor.
- El tiempo de simulación.
- La unidad interna provisional del motor (`tick`).
- El scheduler que decide cuándo avanzar.
- La representación temporal que recibe cada frontend.

## Verificación realizada al documentar

El comando de comprobación de tipos pasa. La suite de pruebas tiene cuatro fallos en dos ficheros de prueba ya modificados en el árbol de trabajo (`anclasSatelites.test.ts` y `lineas_produccion.test.ts`); no se ha modificado código durante este análisis.
