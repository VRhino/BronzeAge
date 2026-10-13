# Reconstrucción: problemas de raíz y soluciones finales

> **Estado (2026-10-12): ACORDADA, cero código.** Decisión del usuario: **no hay soluciones intermedias**; cada
> problema se resuelve con su solución final aunque implique reconstruir. Las reglas de juego están definidas
> (`Docs/Game/` y las decisiones de `Consideraciones/`), así que reconstruir no es el riesgo principal. Decisiones en §5.
>
> Sale de la revisión de distancias (`Calibracion_Distancias_Definicion.md`). El diseño del núcleo es
> `Docs/Arquitectura/13_Mundo_Reactivo_Rust.md`. Página de trabajo: https://claude.ai/artifact/9MViDfr2yzTsqaRgUCXbMU

## 1. Qué hay hoy

| Parte | Tamaño (líneas, sin tests) |
|---|---|
| Motor (`src/engine`) | 18.800 |
| Sesión (`src/session`) | 11.400 |
| Servidor HTTP/WS (`src/server`) | 7.100 |
| Bots (`src/bots`) | 3.300 |
| Mundo, worldgen, dominio y contratos | 6.000 |
| Constantes | 2.900 |
| Tests | ~32.000 |
| Canon (`Docs/Game`, 7 documentos) | 2.200 |

Servidor TypeScript con un **tick de 1 minuto de mundo** que lo avanza todo a la vez. Generador de mapas de 2.000 × 2.000
con 8 perfiles. El generador regional de Codex (32.000 × 24.000 Unity) vive aparte, sin commitear. Unity
(Conquest_prototype) trabaja aislado, adelantando sus reglas y las batallas.

## 2. Problemas de raíz y su solución final

| # | Problema de raíz | Síntomas | Solución final |
|---|---|---|---|
| P1 | **El mundo está vacío**: demasiada distancia entre las cosas | 8,5 min a la plaza vecina; nadie juega | La calibración (ligera 30, plazas a ≥ 30, zonas 10-30, ciudad −80 %) y la actividad agrupada en focos, frontera y tierra lejana (`Calibracion_Distancias_Definicion.md` §2, §4-§6) |
| P2 | **Dos generadores con dos sistemas de unidades** | perfiles de 2.000 frente al regional de Unity | **Un solo generador, el regional**, como herramienta de contenido: produce un **artefacto de mundo** inmutable en unidades estratégicas (2.667 × 2.000, factor 12), identificado por hash. El servidor lo carga; no regenera nada desde la semilla. Unity recibe un paquete derivado del mismo artefacto (×12) |
| P3 | **El tiempo es discreto**: el servidor solo existe cada minuto | órdenes que tardan hasta 60 s en empezar; detección que se salta puertas y encuentros; clientes que interpolan o extrapolan; un mensaje por tick aunque nada cambie; recuperar una caída es simular los minutos perdidos | **Mundo reactivo por eventos** (doc 13): reloj continuo, cola de eventos, **planes** de movimiento con la hora de paso por cada vértice y **ritmos** para todo lo que crece o se gasta. Sin tick, tampoco para la economía |
| P4 | **Las reglas están escritas como «cada tick»** | ~54 tasas `*PorMinuto` aplicadas por tick | Reescritas **desde el canon** como ritmos + eventos de umbral, no traducidas del código viejo |
| P5 | **El contrato viaja en progreso por tick y con fallos** | `desde` adelantado, campos borrados que llegan como `undefined` | **Contrato 05** por esquema (Protobuf): sesión en tiempo real con planes, ritmos, órdenes con respuesta causal, reloj y reanudación (§2.1) |
| P6 | **Unity y servidor sin contrato de mundo** | BA-003 dice «semilla», Codex exporta alturas y capas | **Contrato de mundo**: artefacto canónico + paquete Unity derivado, con hash y versión; factor 12; el servidor decide y Unity representa |
| P7 | **Conceptos acoplados en el código** | ciudad que crece con el número de la zona; escolta fuera de la velocidad; campamentos y alijos colocados por el motor después del mapa | El modelo sale **del canon**, con cada concepto separado; campamentos y alijos son parte del artefacto de mundo |
| P8 | **Persistencia frágil** | foto entera, mapa regenerado desde la semilla, versiones que rechazan partidas | **Registro de eventos + fotos periódicas** (doc 13 §6); el mundo es el artefacto por hash |
| P9 | **Balance atado al runner de producción** | el batch simula minuto a minuto | **Mismo núcleo en modo acelerado**, determinista con punto fijo |
| P10 | **Sin escala de juego en los clientes** | mapa web de 900 px escalado; zoom máximo 4 | Unity con el contrato de escala (factor 12, cámara ~180); el admin web dibuja el artefacto por teselas |
| P11 | **Bugs del motor actual** (escolta, ciudad, etc.) | ver P7 | Desaparecen con la reconstrucción; no se arreglan en el código viejo |
| P12 | **La experiencia del cliente se construye adivinando y refrescando** (conjunta cliente-servidor) | movimiento parcheado cuatro veces (web 0.29.1-0.29.4); cada acción vuelve a pedir la proyección entera; sondeos de 2-3 s; repintados que se llevan lo que se escribe; recargar rehace todo | Sesión en tiempo real con respuesta causal, reloj sincronizado y reanudación; Unity con almacén normalizado y UI Toolkit (§2.1, §2.2) |

### 2.1 La experiencia en el cliente: movimiento y refresco (P12)

**El cliente final es Unity** (Conquest_prototype: campaña, asentamiento y batalla). El cliente web de jugador
(`BronzeAgeClient`) se retira; queda solo el **admin web**, como herramienta para todo lo que no es 3D (interfaces,
interacciones, reglas y contrato).

Es un problema **conjunto**: el servidor manda fotos y progreso por tick, y los clientes reconstruyen la presentación
sondeando y adivinando. En el web: CHANGELOG 0.29.x, `main.ts` (`ejecutarYRefrescar`, `refrescarDatosJuego`, sondeos de
3 s). En Unity: `BronzeAgeCampaignConnection` sondea por HTTP cada 2 s y salta de posición en posición
(`Conquest_prototype/Docs/BronzeAgeCampaignConnection.md`).

| Momento | Hoy (cliente web, el más avanzado en tiempo real) |
|---|---|
| Ver moverse una columna | Una posición por tick; el cliente interpola un tick por detrás o extrapola con correcciones |
| Dar una orden | Comando por HTTP → `GET` de la proyección entera → repintado de los paneles |
| Plaza y campamento | Sondeo cada 3 s (el campamento no tiene canal de tiempo real) |
| Escribir en un panel | El repintado lo borraría; se guarda a mano lo escrito, el selector abierto y el último error |
| Recargar la página | Vista del mapa desde `localStorage`; proyección por HTTP; resuscribir canales; el resto de cero |

**Solución final, servidor** (núcleo reactivo y contrato 05):

1. **Una sola sesión en tiempo real por jugador** que trae todo lo que ve: mapa, héroe, plaza, campamento, avisos.
   Nada sondea ni pide la proyección entera.
2. **Las órdenes van por la misma sesión**, con identificador de petición. La respuesta llega **después de los cambios
   que produjo** (entrega causal): al recibir «hecho», el cliente ya tiene el estado nuevo.
3. **Movimiento como planes** (vértices con su instante) y **economía como ritmos**.
4. **Reloj sincronizado**: la sesión publica el reloj de mundo y su ritmo; el cliente estima el desfase con la latencia.
5. **Reanudación por versión**: al reconectar, el cliente dice qué versión tiene de cada cosa y recibe solo lo que
   cambió, o una foto si es muy vieja.
6. **Lo ajeno con horizonte corto**: solo el tramo visible del plan, sin revelar el destino.
7. **Contrato por esquema en Protobuf** (D8): los tipos se generan para Rust (servidor y bots), C# (Unity) y TypeScript
   (admin web y generador). La presencia de campos de Protobuf distingue «no viene» de «se borró».

**Solución final, Unity** (el admin web aplica lo mismo con vistas reactivas):

1. **Almacén normalizado** (entidades por id) que solo cambia con los mensajes de la sesión.
2. **UI Toolkit con enlace de datos** sobre ese almacén (D6): un cambio actualiza solo lo que cambió; lo que se
   escribe, la selección y los errores no se pierden.
3. **El mapa pinta planes y ritmos** en cada fotograma: posición del plan con el reloj sincronizado, altura del terreno
   cargado.
4. **Reanudación instantánea** al reabrir la aplicación o perder la conexión: el último estado se pinta desde la caché
   local y la sesión lo pone al día por versión. El paquete de mundo ya está en disco por hash.
5. **Respuesta inmediata a las órdenes**: la acción queda en «enviando» hasta la respuesta, que llega en un viaje de ida
   y vuelta con el resultado ya aplicado.

**Cómo se comprueba**: latencia de orden a pantalla (un viaje de ida y vuelta), cero peticiones de estado fuera de la
sesión, cero saltos y retrocesos en el movimiento, y tiempo de reapertura hasta ver el mundo.

### 2.2 Unity como cliente final

Estado (Conquest_prototype, 2026-10-12): Unity trabaja **aislado**, adelantando sus reglas y las batallas. El factor 12
ya está aplicado (`ScaleContract.asset`, escena `CalibrationFactor12`, héroe animado a escala 6). La campaña es
autocontenida: navegación local propia (`WorldMapLocalNavigation`, rejilla 512 × 384, A* local, ríos sin vados) y
movimiento local. La conexión de lectura por sondeo HTTP (`BronzeAgeCampaignConnection`) **se descarta**: era un paso
intermedio. Unity se conecta directamente al contrato 05 cuando exista el núcleo.

| # | Problema | Solución final |
|---|---|---|
| U1 | **Dos navegaciones** con reglas distintas: las rutas de Unity y las del servidor no coinciden | **Una sola autoridad**: la ruta y el plan los calcula el servidor; Unity solo pinta el plan. La transitabilidad sale del artefacto de mundo. La navegación local de Unity no decide rutas |
| U2 | Movimiento por instantáneas | Plan evaluado en C# con el reloj sincronizado |
| U3 | Transporte por sondeo HTTP | Sesión persistente en C# con el contrato 05 |
| U4 | **Escenas que cambian por su cuenta** sin ejecutar `entrarEnAsentamiento` ni `salirAlMundo` | **La escena sigue al estado del servidor**: entrar es una orden; la escena se carga cuando la sesión confirma la ubicación. Una sola sesión entre campaña, asentamiento y batalla |
| U5 | **Batalla** coordinada con el tick | El ciclo de batalla son eventos del núcleo (inicio, incorporaciones, resultado); lo de dentro lo decide Unity; el servidor de batalla recibe el terreno del punto desde el artefacto. El mundo sigue mientras se lucha |
| U6 | Trazado urbano en el servidor, escena en Unity | El trazado viaja por la sesión (unidades locales) y la escena se construye desde él |
| U7 | Niebla visual pendiente | El servidor filtra desde la conexión; Unity pinta la máscara con lo que recibe |
| U8 | Paquete de Unity y mundo del servidor por separado | Derivan del mismo artefacto y se identifican por hash |

## 3. Arquitectura final

```text
  Generador de mundo (TypeScript, herramienta fuera de línea)
    regional de Codex → economía del mundo (campamentos, anillos, recursos, vetas, bosque explotable)
        │       lee las constantes de juego del archivo común
        ├──► artefacto de mundo (estratégicas, hash) ──────────► servidor
        └──► paquete Unity derivado (×12, por sectores, hash) ─► Unity

  Servidor (Rust, un solo proceso): núcleo reactivo (doc 13)
    reloj continuo · cola de eventos · planes · ritmos · encuentros exactos · visión muestreada
    todas las reglas del canon · registro de eventos + fotos · modo acelerado para el batch

  Bots (Rust, servicio aparte) ◄──┐
  Unity: campaña, asentamiento ◄──┼──► sesión en tiempo real · contrato 05 (Protobuf por WebSocket)
  Admin web (herramienta)      ◄──┘     planes y ritmos · órdenes con respuesta causal · reloj · reanudación
  Servidores de batalla de Unity ◄──► eventos de batalla del núcleo + terreno del artefacto

  Constantes de juego: un archivo de datos común (servidor, bots y generador lo leen)
```

## 4. Orden de la reconstrucción

0. **Arranque** (hecho el 2026-10-12): la rama **`typescript-final`** guarda el último TypeScript (copia de `main` en
   `eb1cdd9` el 2026-10-12; el 2026-10-13 la rama se adelantó a `main` para incluir el código TypeScript y la documentación completa de `Docs/Mechanics`, sin subir a ningún remoto). Desde ahora todo el trabajo va sobre `main`, que es la reconstrucción (D4).
   Se apaga el playtest (D5). El servidor TypeScript, sus bots y el cliente web de jugador se retiran de `main` cuando
   llegue su sustituto; hasta entonces `main` solo recibe documentación y especificación (D11).
1. **Ordenar el trabajo previo** (§4.1): el canon al día con todo lo decidido y los pendientes cerrados o descartados.
   Es lo primero, antes de crear los repos y antes de escribir código.
2. **Contratos** sobre ese canon: contrato de mundo (artefacto + paquete Unity) y contrato 05 en Protobuf (§2.1), con el
   modelo de datos completo.
   **Hecho el 2026-10-13 (diseño):** `proto/bronzeage/v1/` (common, entities, orders, events, battle, session, world; 296 mensajes, 135 órdenes, validado por carga con protobufjs), `Docs/Coordinacion/01_Modelo_de_datos.md`, `05_Contrato_de_Sesion.md`, `06_Contrato_de_Mundo.md` y `propuestas/CIERRE_FASE_2.md`. Pendiente: ficheros dorados y generación de código al crear los repos (fase 3).
3. **Tres repositorios** (D10), una vez existe el contrato: servidor (este repo), bots y admin web. Nacen desde cero
   sobre el contrato 05; el TypeScript actual queda en `typescript-final` como referencia, porque ni los bots ni el
   admin compilan sin el motor (importan `domain`, `engine`, `session` y `server`; el admin, unos 40 módulos por el
   alias `@motor/*`).
4. **Artefacto de mundo**: el generador regional consolidado, con la economía del mundo (Codex + Claude).
5. **Núcleo**: reloj, cola, planes, encuentros, visión, persistencia por eventos, modo acelerado.
6. **Reglas por sistema del canon**, cada una con su validación en batch: mundo y territorio → población y construcción
   → economía y comercio → militar y combate → política → tecnología y Aedas.
7. **Bots en Rust** y **admin web**, cada uno en su repo, sobre el contrato 05.
8. **Unity** se conecta al contrato 05 y al contrato de mundo (§2.2).
9. **Despliegue**.

No hay convivencia de los dos servidores ni compatibilidad con partidas viejas: se borran todas.

### 4.1 Ordenar el trabajo previo (fase 1)

El contrato actual (`Docs/Coordinacion/01-03` y `src/contratos/v1`) describe el servidor TypeScript y está por detrás
de lo decidido, y lo decidido vive repartido. Antes de migrar se ordena, solo con documentación:

| Parte | Qué hay hoy | Qué se hace |
|---|---|---|
| `Consideraciones/` | 36 documentos, ~15.000 líneas, de decisiones y planes | Inventario: cada documento va a **canon**, a **descartado** o a **pendiente**, con su destino |
| `Docs/Mecanicas a desarrollar.md` | 7 secciones abiertas (caravanas diferidas, tecnología por Eras, tope de asedio, guerra logística, repostar y enganchar, bots emergentes) | Cada pendiente se **decide** o se **descarta**; lo que queda abierto entra en la reconstrucción con dueño |
| `Docs/Mechanics/` | 19 documentos y ~11.000 líneas: reflejo completo del motor TypeScript (122 comandos con la forma de sus parámetros, todos los módulos de `engine/`, parámetros sincronizados con `constants.ts` por test); revisado el 2026-10-12 | **Ya no hay que extraer nada. Se reescriben** (decisión del 2026-10-13) para el mundo reactivo y la calibración: reloj continuo en vez de tick (Tick, Movimiento, Niebla, Presencia), calibración (Movimiento, Generación, Trazado, Murallas), guerra logística (Comercio, Combate, Niebla), repostar y víveres (Movimiento, Comercio). El estado «cómo funciona hoy» queda en `typescript-final` |
| `Docs/Game/` | 7 documentos, 2.200 líneas | Se **reescribe** con todo lo decidido: calibración (§2-§5 de `Calibracion_Distancias_Definicion.md`), guerra logística, víveres, murallas, Eras, héroe y batallas (BA-001) |
| `Docs/Coordinacion/` y `src/contratos/v1` | Contrato del servidor TypeScript, JSON | Se redefine como **modelo de datos completo** (todas las entidades del dominio) y como contrato 05 en Protobuf |
| `Docs/Coordinacion/propuestas/` | BA-001 a BA-006 con Codex y Unity | Se cierran o se reescriben contra el contrato de mundo y el 05 (BA-003 en particular) |
| `Docs/Arquitectura/` | 1-13, de la arquitectura TypeScript | Se conservan 12 (bots) y 13 (mundo reactivo); el resto queda como historia en `typescript-final` |

**Estado de la fase 1 (2026-10-13).** Hecho: inventario y verificación de `Consideraciones/` (de 37 documentos quedan los 5 de la reconstrucción: plan, calibración, guerra logística, ritmo de crecimiento y Eras IV-V pospuestas; el resto, en `typescript-final`); canon al día con la calibración, la guerra logística, el ritmo de niveles 2-3, la tropa del héroe y prestada y la ausencia de vuelta a casa de los ejércitos; `Docs/Mecanicas a desarrollar.md` reducido a las Eras; `Docs/Arquitectura/` reducido a 12 y 13; ideas aparcadas en `Docs/Ideas a diseñar.md` §47; reglas de los bots en `Arquitectura/12` §16. `Docs/Mechanics` reescrito para el mundo reactivo (2026-10-13): `Tick.md` sustituido por `Reloj_Y_Eventos.md`, `Movimiento_Ejercitos`, `Niebla_De_Guerra`, `Presencia_Y_Convocatoria` y `Combate` con el diseño nuevo; el resto con nota de «Reescrito», parches de lo que cambia y las cifras vigentes de la calibración (los pasos «por tick» de Economía, Tecnología y Diplomacia se leen como ritmos y eventos; su fórmula por minuto es la misma). Pendiente: afinar esos tres línea a línea al implementar cada sistema (fase 6); BA-001 a BA-005 se cierran o reescriben dentro de la fase 2, junto al contrato de mundo y el 05, porque Codex edita esa carpeta y su contenido es el contrato con Unity; BA-006 se pospone con las Eras. Registro detallado: `Fase1_Inventario.md` y `Fase1_Verificacion_B.md`.

Salida de la fase: un canon sin contradicciones, ninguna decisión que viva solo en `Consideraciones/`, y un modelo de
datos del que se generan los contratos.

## 5. Decisiones (2026-10-12)

| # | Decisión |
|---|---|
| D1 | **La reconstrucción sustituye** al plan anterior de cerrar primero el servidor TypeScript y migrar después por fases. Se retiran sus referencias (`typescript-final:Docs/Coordinacion/04_Migracion_Mundo_Reactivo.md`, `Docs/Arquitectura/13_Mundo_Reactivo_Rust.md`, índice de Coordinación) |
| D2 | **Servidor de un solo núcleo, todo en Rust**, en un proceso. Sin reparto TypeScript/Rust |
| D3 | **El generador sigue en TypeScript** como herramienta fuera de línea: no corre durante la partida, es el código de Codex, y lo que comparte con el servidor son las constantes de juego en un archivo de datos común |
| D4 | **Dentro de `BronzeAgeFase0`, sobre `main`** (decisión del 2026-10-12, sustituye a la rama `reconstruccion`). La rama **`typescript-final`**, copia de `main` en `eb1cdd9`, guarda el último TypeScript |
| D5 | **El playtest se apaga por completo** |
| D6 | **Unity usa UI Toolkit con enlace de datos** para campaña y asentamiento |
| D7 | **Del web queda solo el admin**, como herramienta para lo que no es 3D (interfaces, interacciones, reglas y contrato). El cliente web de jugador se retira |
| D8 | **Protobuf** para el contrato 05: generación de código para Rust, C# y TypeScript, mensajes compactos, presencia de campos |
| D9 | **Bots en Rust**, como servicio aparte que usa el mismo contrato que un jugador, sin privilegios |
| D10 | **Tres repositorios separados**: servidor (`BronzeAgeFase0`), bots y admin web. Se crean **después** del contrato y nacen desde cero sobre el contrato 05 (decisión del 2026-10-12) |
| D11 | **Se ordena antes de migrar** (§4.1): mientras dure, `main` solo recibe documentación y especificación |
