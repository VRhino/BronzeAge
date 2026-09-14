# 01 — Modelo de datos compartido (v1)

**Estado:** ACEPTADO como diseño de producto (qué existe, quién es dueño de qué) — **no todavía como
contrato implementable e interoperable**. `Docs/Coordinacion/propuestas/REVISION_CONTRATOS_CODEX_2026-09-11.md`
encontró transiciones sin mensaje que las dispare, huecos de completitud y afirmaciones que no coincidían
con el código real; las secciones §12/§13/§15/§17/§19 llevan las correcciones de esa revisión inline,
marcadas donde siguen abiertas (`PENDIENTE`). De lo nuevo, `Heroe` ya existe en el dominio con sus campos de
identidad (§12) y es el dueño de todo; el resto de sus campos, `Batalla` y los campos 3D todavía no. La forma
en el cable de todo ello está en `src/contratos/v1/` (§15). Todo lo demás SÍ existe hoy en `src/domain/types.ts` — se transcribe completo aquí
porque es lo que Unity necesita conocer para proyectar mundo, facción y asentamiento, no solo lo
relacionado con batalla.  
**Fecha:** 2026-09-11 (dos revisiones: se completó el inventario tras observación de que la primera versión
solo cubría identidad/héroe/batalla; luego se corrigieron 10 defectos de diseño encontrados por Codex)  
**Basado en:** BA-002, BA-003, BA-004, `REVISION_CODEX_2026-09-11.md`,
`REVISION_CONTRATOS_CODEX_2026-09-11.md` y lectura completa de `src/domain/types.ts` +
`src/server/`/`src/session/` reales.

Este documento fija el modelo de datos que persiste BronzeAge y que Conquest/Unity consume por proyección.
No sustituye `src/domain/types.ts` como fuente de verdad en tiempo de ejecución — cuando el código exista,
manda el código; este documento es el contrato que debe cumplir, y la referencia para quien no tiene el
repo de BronzeAge delante.

Convención de lectura: cada entidad se lista con TODOS sus campos tal como existen hoy en el dominio
(marcados `— existente`) más los campos NUEVOS que añade este modelo compartido (marcados `— nuevo`, y
todavía sujetos a cambio hasta que se implementen — ver revisión de Codex 2026-09-11, no son ya forma
cerrada). `?` = opcional.

**Corrección de Codex (revisión `REVISION_CONTRATOS_CODEX_2026-09-11.md`, R07) sobre `Instante`:** la
versión anterior de este documento afirmaba que `Instante` viaja como ISO 8601 — falso, no es lo que hace
el código hoy. El contrato HTTP real (`RESUMEN_PARTIDA_RESPUESTA.instante`, `server/rutas/esquemas.ts`;
`proyecciones/jugador.ts`) serializa `instante` como **número** (ms desde época), y así se mantiene aquí:
ningún campo `Instante` de este documento cambia de forma respecto al contrato ya vigente.

`isoDeInstante` (`session/estado.ts`) existe pero solo se usa en algunos campos puntuales (ej. mensajes de
auditoría), no de forma universal — no se generaliza aquí sin necesidad.

**Distinción de reloj, nueva en esta revisión:** `Instante` es tiempo de MUNDO — no avanza durante una
caída del proceso (`Docs/Arquitectura/10_Modelo_Temporal.md`: "una caída no consume tiempo de mundo"). Eso
es correcto para plazos de JUEGO (`Batalla.expiraEn`, timeout de infraestructura) pero **no vale para
vigencia de credenciales**: un token de participante (§15) debe caducar en tiempo real transcurrido aunque
el proceso esté caído, o un token robado seguiría siendo válido indefinidamente durante una caída larga. Por
eso `BattleServerAssignment.tokensParticipante[].expiraEn` usa **tiempo real UTC** (`string`, ISO 8601), un
campo distinto y explícitamente no comparable con ningún `Instante` de mundo — ver §15.

## 0. Índice

1. Cadena de identidad
2. `Faccion`
3. `Asentamiento` (con `Poblacion`, `RecursoAlmacenado`, `Edificio`, `CargosAsentamiento`, `Recinto`/`CeldaMuro`, `PoliticaActiva`)
4. Territorio derivado (`ZonaInfluencia`, `ZonaFaccion`)
5. `Ejercito`
6. `Caravana` / `CarroCaravana`
7. Comercio (`AcuerdoTrueque`, `OrdenMercado`, `CaminoComercial`)
8. `RelacionPolitica`
9. `Titulo`
10. `CampamentoBandido`
11. Mundo / worldgen (`WorldConfig`, `NodoRecurso`, `ZonaBosque`, `RioZona`, terreno/bioma/región)
12. `Heroe` (nuevo)
13. `Escuadron` (actualizado)
14. `Loadout` (nuevo)
15. `Batalla` / `BattleTicket` / `BattleResult` (nuevo)
16. Idempotencia de `BattleResult`
17. Campos 3D del asentamiento (nuevo, complementa §3)
18. Frontera cosmético/autoritativo de mundo
19. Visibilidad — qué ve Unity de todo esto
20. Invariantes generales

## 1. Cadena de identidad

```text
Usuario (acceso/)
  └── Sesion (acceso/, credenciales activas)
  └── Membresia (acceso/): usuarioId + gameId + jugadorId + rol + vigencia
        └── Heroe (domain/): jugadorId (= Membresia.jugadorId) — 1:1 dentro de gameId
              ├── Escuadron[] (heroeId) — todas sus escuadras, estén donde estén (§13)
              └── Loadout[] (heroeId)
```

**No existe una entidad `Jugador` nueva en `domain/`.** Duplicaría `Membresia` (`acceso/tipos.ts`, ya une
`usuarioId`+`gameId`+`jugadorId`+rol con vigencia) — corrección de Codex, incorporada. `Heroe.jugadorId` es
el mismo valor que `Membresia.jugadorId`, no una copia con otro significado.

Resolución de un actor autenticado: `Sesion -> Usuario -> Membresia -> jugadorId -> Heroe`. Las reglas de
juego reciben `heroeId`; auditoría y autorización conservan también `usuarioId`/`jugadorId`.
`GameSessionState.jugadores` se reemplaza por `heroes`.

**Héroes bot (decisión del usuario, 2026-09-13):** las Facciones NPC también tendrán héroes, manejados por
la IA de juego. Se llaman "héroes bot" para no confundirlos con los humanos. Un héroe bot es un `Heroe` con
`controlador: 'bot'`: no tiene `Usuario` ni `Membresia` (su `jugadorId` es `null`) y su dueño es la Facción
NPC, que ya actúa como `servicio_npc`. La unicidad "un héroe por jugador y mundo" solo aplica a los humanos.
Los crea el admin al crear una Facción NPC (`crearFaccionNpc`, doc 02 §4.2): nacen como los fundadores de su
primer asentamiento y viven en la partida como cualquier otro héroe, con `controlador: 'bot'`. Su comportamiento en el mundo de BronzeAge está pendiente (`Docs/Mecanicas a
desarrollar.md` §36); en batalla los maneja la IA de Conquest (CQ-002).

### Tabla de identidad y ámbito

| ID | Ámbito | Persistente | Regla |
|---|---|---:|---|
| `usuarioId` | cuenta global | sí | identidad autenticada, no personaje — existente |
| `gameId` | mundo/partida | sí | delimita todo estado de juego — existente |
| `jugadorId` | `Membresia` dentro de `gameId` | sí | existente; nunca se sustituye por `heroeId`, son capas distintas |
| `heroeId` | `Heroe` dentro de `gameId` | sí | como máximo un `Heroe` por `jugadorId`/`gameId`, forzado por unicidad de `Heroe.jugadorId` |
| `squadId` | `Escuadron` dentro de `gameId` | sí | único y estable con independencia del contenedor (§13) |
| `loadoutId` | `Loadout` dentro de `heroeId` | sí | referencia `squadId`, nunca copia la escuadra |
| `battleId` | `Batalla` | sí | une ticket, instancia y resultado |
| `resultId` | liquidación de una `Batalla` | sí | clave de idempotencia — §16 |
| `faccionId` | `Faccion` | sí | existente |
| `asentamientoId` | `Asentamiento` | sí | existente |
| `edificioId`, `recintoId` | dentro de `asentamientoId` | sí | existente |
| `ejercitoId`, `caravanaId` | dentro de `gameId` | sí | existente |
| `runtimeEntityId` | un World ECS de Conquest | no | nunca cruza la API ni se guarda en BronzeAge |

## 2. `Faccion`

```text
Faccion
  id                    faccionId — existente
  nombre                editable — existente
  reyId: string | null   vasallaje/políticas superiores — existente. Tras este modelo referencia heroeId
  embajadorId: string | null   designado por el Rey — existente. Referencia heroeId
  nivel                 derivado de experiencia, sube cupo de asentamientos/fundación — existente
  experiencia            MONÓTONA, nunca baja (combate/construcción/conquista/caravanas) — existente
  ciudadanosIds[]         quién tiene ciudadanía en ESTA facción, no toda la Liga — existente. Tras este
                         modelo, lista de heroeId
  reputacion              score público -100..+100, decae hacia 0 — existente
```

La "Liga" (Doc 0) no es una entidad persistida: se deriva de la red de `RelacionPolitica` activas (§8).

## 3. `Asentamiento`

Es la entidad con más estado del dominio — se detalla completa, no por encima.

```text
Asentamiento
  id                          asentamientoId — existente
  nombre?                     editable, puramente de presentación — existente
  faccionId                    existente
  jugadoresFundadoresIds[]     existente. Tras este modelo, heroesFundadoresIds — quién fundó, base de
                               ciudadanía/residencia
  posicion: Point               en el MAPA GENERAL — existente
  nivel                        NIVEL ALCANZADO — histórico, monótono, nunca baja — existente
  nivelActual                  NIVEL OPERATIVO — puede bajar por mal mantenimiento, nunca > nivel —
                               existente. Determina qué se puede construir/mejorar/reclutar AHORA
  rachaMantenimientoSano?       ticks consecutivos de mantenimiento sano, sube nivelActual al umbral —
                               existente
  fundadoEn: Instante            existente
  radioPotencial                radio de zona de influencia sin fronteras vecinas — existente
  poblacion: Poblacion            ver abajo — existente
  almacen: Record<RecursoTipo, RecursoAlmacenado>   ver abajo — existente
  edificios: Edificio[]           ver abajo — existente
  recintos?: Recinto[]            murallas, del más interior al más exterior — existente
  cargos: CargosAsentamiento       ver abajo — existente
  casasCompradas[]                 ciudadanía por compra, distinta de fundar — existente. Tras este modelo,
                                   heroeId[]
  politicaDeAcceso?               'abierto' | 'faccion_y_aliados' | 'solo_faccion' | 'cerrado'. Ausente =
                                   'faccion_y_aliados' — existente
  vetadosIds?[]                    vetados por el Gobernador, por encima de la política — existente. Tras
                                   este modelo, heroeId[]
  politicasActivas: PoliticaActiva[]   ver abajo — existente
  escuadrones: Escuadron[]          existente — DESAPARECE en el modelo de héroe: las escuadras pasan a
                                   vivir en `Heroe.escuadrones` (§13). La guarnición es un subconjunto
                                   derivado: escuadras de sus residentes con `enGuarnicion`. El cupo de
                                   guarnición de cada héroe también es derivado (edificios y políticas de
                                   este asentamiento, Doc 5.15); no se persiste.
  ocupacionHasta?: Instante          ocupación militar tras conquista — existente
  medidorMantenimiento             0-100, empieza en 100; a 0 cae en ruinas — existente
  nutricionPoblacion?              0-100, hambruna por déficit de trigo — existente
  autoConstruccionPausada?          si true, el motor deja de comprometer necesidades NUEVAS — existente
  permiteReabastecerAliados?        ejércitos aliados pueden repostar aquí — existente
  reservaManual?: Partial<Record<RecursoTipo, number>>   calibrada por el Tesorero — existente
  extractoresTicksSinCupo?: Partial<Record<EdificioTipo, number>>   desempate anti-inanición — existente
  ultimaCaravanaCreadaEn?: Instante   cooldown de creación de caravanas — existente
  visualCatalogVersion?             NUEVO — ver §17
  layoutVersion?                     NUEVO — ver §17
```

### `Poblacion`

```text
Poblacion
  pesants: number
  artesanos: number
  nobleza: number
```

3 clases de población NPC. Los héroes (antes jugadores) son una categoría aparte.

### `RecursoAlmacenado`

```text
RecursoAlmacenado
  cantidad: number
  capacidad: number
```

### `RecursoTipo` (20 valores, catálogo cerrado — existente, `constants.ts`)

```text
madera · piedra · trigo · cobre · estano · oro · livestock
lingoteCobre · lingoteEstano · lingoteBronce
cuero · cueroCurtido · cueroCalidad
armaMadera · armaCobre · armaBronce · armaBronceCalidad
armaduraBasica · armaduraIntermedia · armaduraBronce
```

### `Edificio`

```text
Edificio
  id                     edificioId — existente
  tipo: EdificioTipo       ver abajo — existente
  posicion: Point           LOCAL al espacio del asentamiento (origen en el Centro Urbano), salvo
                           extractores minerales (ambito:'mapa') que usan coordenada del MAPA GENERAL —
                           existente
  estado: 'en_cola' | 'en_construccion' | 'activo'   existente
  completaEn?: Instante     solo mientras en_construccion — existente
  ambito?: 'asentamiento' | 'mapa'   ausente = 'asentamiento' — existente
  fuenteId?                 nodo de recurso o zona de bosque que explota — existente
  nivelInterno?              solo edificios de transformación con tiers — existente
  rotado?                    orientación intercambiable ancho↔alto — existente
  semillaSaturada?            agotó sus 8 direcciones de crecimiento como semilla de anclas — existente
  anclaLlena?                 sin hueco para el próximo satélite de su categoría — existente
  prioridad?                  score de necesidad capturado al comprometerse — existente
  pausadoPorAlmacenLleno?     produjo algo que no cupo este tick — existente, solo informativo
  danado?                     ocupación post-conquista: activo bajado a en_cola dañado — existente
  visualSeed?                NUEVO — ver §17
```

### `EdificioTipo` (28 valores, catálogo cerrado — existente, `constants.ts`)

Generar este catálogo desde `EDIFICIOS_TIPO` (la fuente de verdad ya exhaustiva en tiempo de compilación) en
vez de retranscribirlo a mano evita que este documento se desincronice del código — la lista de abajo es
solo para lectura humana.

```text
centroUrbano · vivienda · granja · cantera · lenera · almacen · granero
mina · minaCobre · minaEstano · fundicion · granFundicion
corral · armeria · curtiduria · carpinteria
palacio · barracon · galeriaDeTiro
mercado · puestoMercado · maravilla
plaza · plazaDeArmas · patioDeGremios
pozo · parque
```

Footprint (ancho×alto en celdas) por tipo/nivel: `EDIFICIO_TAMANO` / `EDIFICIO_CATALOGO[tipo].niveles[n].tamano`
— fijo por tipo salvo Granja, la única cuya huella crece con `nivelInterno` (2×2→6×6). Tabla completa y escala
(una celda = ancho de calle = 3 unidades locales) en BA-005; revisión geométrica vigente en `LAYOUT_VERSION` (§17).

### `CargosAsentamiento`

```text
CargosAsentamiento
  gobernadorId: string | null
  tesoreroId: string | null
  generalId: string | null
  maestroObrasId: string | null
  sacerdoteId: string | null
```

Uno de cada, designados por el Gobernador salvo él mismo (lo designa el Rey de la facción dueña). Tras este
modelo, cada `*Id` referencia `heroeId`.

### `Recinto` / `CeldaMuro` (murallas)

```text
Recinto
  id                     recintoId — existente
  nivel                   1 empalizada · 2 muro de piedra · 3 muralla con adarve — existente
  celdas: CeldaMuro[]      anillo completo, EN ORDEN DE RECORRIDO desde la puerta principal, congelado al
                          comprometerse — existente
  avance                  índice de la última celda YA LEVANTADA — existente. **Corrección de Codex (R10):
                          `avance: -1` NO significa siempre "nada en pie"** — `engine/muralla.ts` también
                          pone `avance: -1` al INICIAR una mejora de un recinto YA COMPLETO
                          (`mejorandoA` presente). Un lector que trate `avance === -1` como "sin muro"
                          borraría visualmente (y en colisión táctica) una muralla completa en el instante
                          en que su dueño empieza a mejorarla. Regla correcta: si `mejorandoA` está
                          presente, la geometría FÍSICA vigente es la de `nivel` (el nivel ya completo, sin
                          tocar) — `avance` en ese caso mide el progreso de la MEJORA, no de la existencia.
                          Solo cuando `mejorandoA` está ausente Y `avance === -1` significa "trazo
                          comprometido, nada levantado todavía"
  mejorandoA?              nivel al que se mejora ahora, si hay mejora en curso — existente
  comprometidoEn: Instante   existente
  completadoEn?: Instante     existente

CeldaMuro
  col: number
  row: number
  clase: 'muro' | 'puerta' | 'torre'   decidida al trazar, no cambia nunca — existente
```

Una celda que nace puerta muere puerta. Identidad estable de una `CeldaMuro` para el contrato con Unity:
`recintoId` + índice de recorrido dentro de `celdas` (no tiene `id` propio hoy — corrección de Codex, ver
§17 para si se añade uno explícito).

### `PoliticaActiva`

```text
PoliticaActiva
  id
  politicaId              catálogo — existente
  cargo: CargoTipo          'gobernador'|'tesorero'|'general'|'maestroObras'|'sacerdote' — existente
  activadaEn: Instante        existente
  expiraEn: Instante          duración fija, no cancelable antes de tiempo — existente
```

## 4. Territorio derivado

```text
ZonaInfluencia
  asentamientoId
  poligono: Point[]        círculo potencial recortado contra fronteras vecinas

ZonaFaccion
  faccionId
  contornos: Point[][]      unión de las ZonaInfluencia de la facción — SOLO PARA DIBUJAR, nunca
                           consultado por ninguna regla (la pertenencia se resuelve asentamiento a
                           asentamiento)
```

## 5. `Ejercito`

```text
Ejercito
  id                      ejercitoId — existente
  faccionId                 existente
  origenAsentamientoId       de dónde salió y a dónde vuelve — existente
  participantes: { jugadorId: string; unidoEn: Instante }[]   existente. Tras este modelo, heroeId. Orden =
                            antigüedad, determina sucesión de liderazgo
  tipo: 'personal' | 'ejercito'   fijado al crear, nunca cambia — existente. NO se deriva de
                                 participantes.length
  liderId                    quién la formó, cede liderazgo al participante más antiguo si se
                            desconecta — existente. Tras este modelo, heroeId
  politicaDeUnion: 'rechazar' | 'aceptar' | 'preguntar'   fijada al parir la columna — existente
  peticionesDeUnion?: { jugadorId: string; pedidoEn: Instante; expiraEn: Instante }[]   solo si política
                            'preguntar' — existente. Tras este modelo, heroeId
  escuadrones: Escuadron[]    movidos aquí desde Asentamiento.escuadrones — existente
  suministro: Record<string, number>   solo trigo en Fase 0 — existente
  persiguiendo?: { tipo: 'ejercito' | 'caravana'; id: string }   objetivo móvil — existente
  (enTreguaHasta)             RETIRADO 2026-09-14: lo sustituye `Heroe.heridoHasta` (§12, Doc 5.16.4)
  caravanasAdjuntasIds[]        existente
  objetivo: { tipo: 'asentamiento'; id: string } | { tipo: 'punto'; punto: Point }   existente
  ruta: Point[]               polilínea calculada al movilizar — existente
  progreso: number             0-1 a lo largo de ruta — existente
  posicionActual: Point         existente
  estado: 'marchando' | 'estacionado' | 'regresando'   existente
```

## 6. `Caravana` / `CarroCaravana`

```text
Caravana
  id                        caravanaId — existente
  tipo: 'comercial' | 'militar' | 'construccion' | 'contrabando'   existente
  origenAsentamientoId        existente
  destinoAsentamientoId?       ausente en caravanas de fundación — existente
  contenido: Record<string, number>   existente
  posicionActual: Point         existente
  progreso: number              0-1 — existente
  ruta?: Point[]                 ausente solo en 'disponible' sin asignar — existente
  origenAcuerdoId?, ladoAcuerdo?: 'A' | 'B'   si nace de un AcuerdoTrueque — existente
  destinoPosicion?              caravana de Fundación: punto donde fundará al llegar — existente
  jugadoresFundadoresIds?        caravana de Fundación — existente. Tras este modelo, heroeId[]
  estado?: 'disponible' | 'preparando' | 'adjunta' | 'aparcada' | 'en_transito' | 'retornando'   existente
                                (revamp de caravanas, ver Docs/Game/3 §3.13)
  carros?: CarroCaravana[]        SIEMPRE presente para tipo 'comercial' desde snapshot v12 — existente
  escolta?: Escuadron[]           escuadrones EN SÍ (no ids), cedidos por viaje — existente
  reservadaManual?                fuera del reparto automático — existente
  preparaHasta?: Instante          solo en estado 'preparando' — existente

CarroCaravana
  tipoCarro: 'basico' | 'reforzado'
  animal?: 'buey' | 'caballo' | 'camello'   sin animal, el carro no viaja ni cuenta capacidad
```

## 7. Comercio

```text
AcuerdoTrueque
  id
  asentamientoAId, asentamientoBId
  recursoA, recursoB
  cantidadTotalA, cantidadTotalB, cantidadEntregadaA, cantidadEntregadaB
  creadoEn: Instante, expiraEn: Instante
  estado: 'propuesto' | 'activo' | 'rechazado' | 'cumplido' | 'expirado'

OrdenMercado
  id
  asentamientoId
  tipo: 'compra' | 'venta'
  recurso, cantidad, cantidadCumplida, precioUnitario
  creadoEn: Instante, expiraEn: Instante
  estado: 'activa' | 'cumplida' | 'expirada'

CaminoComercial
  id
  asentamientoAId, asentamientoBId
  puntos: Point[]        generado al establecer la primera relación comercial; persiste aunque el
                        AcuerdoTrueque que lo originó expire
```

## 8. `RelacionPolitica`

```text
RelacionPolitica
  id
  tipo: 'vasallaje' | 'alianza'
  faccionAId, faccionBId     en vasallaje, A es la señora y B la vasalla; en alianza es simétrica
  tributo?: { recurso: string; cantidadPorMinuto: number }   solo vasallaje
  creadoEn: Instante
  estado: 'activa' | 'rota'
```

## 9. `Titulo`

```text
Titulo
  nombre
  poseedorId          facción (o jugador según el título) que lo ostenta — DERIVADO, no se persiste como
                     estado propio, se recalcula periódicamente
  valorMetrica
```

Prestigio dinámico, sin beneficio mecánico. Solo lectura para Unity, nunca autoridad.

## 10. `CampamentoBandido`

```text
CampamentoBandido
  id
  posicion: Point
  bosqueId              bosque que ocupa
  asentamientoId          a qué asentamiento "atiende" — como mucho uno por asentamiento
  poder                  poder de combate fijo (placeholder), sin escuadrones propios
```

## 11. Mundo / worldgen

```text
WorldConfig
  ancho: number, alto: number
  seed: number
  region?: RegionId       'greciaContinental' | 'anatolia' | 'egeo' | 'nilo' | 'mesopotamia' — sesga la
                         generación al carácter de una zona real; ausente = mundo libre

NodoRecurso
  id, tipo: RecursoTipo, rareza: 'comun' | 'intermedio' | 'raro'
  posicion: Point
  cantidadInicial          NACE con esto; lo que queda de verdad es estado de partida (EstadoMapa.extraido),
                          no se persiste aquí

ZonaBosque
  id, centro: Point, radio: number
  densidad: number (0-1)   afecta rendimiento de madera

RioZona
  id, puntos: Point[]        polilínea, no celdas
  terminaEnLago: boolean      true si el descenso terminó en mínimo local
  navegable: boolean          pensado para comercio fluvial futuro, no afecta el trazo
```

`TerrenoTipo` (`agua`|`costa`|`llano`|`colina`|`montana`|`cima`) y `BiomaTipo` (`agua`|`costa`|`estepa`|
`llanuraFertil`|`colina`|`montana`|`cima`) se derivan de `CampoElevacion` por umbral — nunca se guardan por
punto, se consultan (`Mapa.terrenoEn`/`evaluarBioma`). No hace falta persistirlos punto a punto para Unity:
se recalculan igual del lado C# con la misma seed y `worldgenVersion`, o se sirven como chunks — ver §18.

## 12. `Heroe` — campos v1

**Corrección de Codex (R09):** llamar a esto "lista cerrada" antes de fijar los tipos abiertos era
prematuro. Esos tipos (forma de `ItemInstancia`, huecos de equipo, género, avatar y atributos) se cerraron el
2026-09-14 transcribiendo la estructura que Conquest ya persiste (`Hero.Data.cs`, `InventoryItem`,
`Equipment`, `AvatarParts`).

No se persisten caches ni atributos calculados — todo lo derivado (poder efectivo, liderazgo total de un
loadout) se recalcula contra el catálogo versionado vigente.

```text
Heroe
  id                    heroeId — nuevo
  jugadorId: string | null   dueño — Membresia.jugadorId, 1:1 por gameId; null en héroes bot — nuevo
  controlador: 'humano' | 'bot'   nuevo (§1)
  displayName           editable, nunca llave de ningún contrato — nuevo
  classDefinitionId      catálogo versionado (Conquest) — nuevo
  genero: 'masculino' | 'femenino'   nuevo — Conquest `Gender { Male, Female }`
  avatar: { cabezaId, peloId, barbaId, cejasId }   nuevo — ids de piezas del catálogo visual de Conquest
                                  (`AvatarParts`: headId, hairId, beardId, eyebrowId). Cosmético: BronzeAge
                                  lo guarda y lo sirve, no lo interpreta
  nivel                   nuevo
  experienciaHaciaSiguienteNivel   nuevo. **Corrección R09:** "hacia el siguiente nivel", no acumulada
                                  desde el inicio — Conquest (`Hero.Data.cs`) ya usa esta semántica; se
                                  nombra explícito para no confundirla con `Escuadron.experiencia` (§13),
                                  que SÍ es acumulada. Nombres distintos a propósito.
  puntosDeAtributoSinGastar, puntosDePerkSinGastar   nuevo. **Corrección R09: DOS bolsas separadas**, no
                                  una — Conquest (`Hero.Data.cs:33-36`) ya las distingue; un solo
                                  `puntosSinGastar` perdería esa distinción al traer el modelo.
  atributosBase: { fuerza, destreza, armadura, vitalidad }   enteros, sin equipo aplicado — nuevo
                                  (Conquest `strength`, `dexterity`, `armor`, `vitality`)
  perksDesbloqueados: number[]   ids numéricos del catálogo de perks de Conquest (`unlockedPerks:
                                  List<int>`) — nuevo
                                  (Sin campo de tipos reclutables propios — decisión del usuario
                                  2026-09-13: un héroe solo recluta los `tropaId` que le permite el
                                  asentamiento. El `availableSquads` de Conquest no se persiste en
                                  BronzeAge; el adaptador de Unity lo deriva del catálogo reclutable del
                                  asentamiento donde está.)
  liderazgoBase          migra desde el actual domain.Jugador.liderazgoBase — existente, cambia de dueño
  ubicacion               migra desde domain.Jugador.ubicacion (asentamiento | columna | desconectado) —
                         existente, cambia de dueño
  plazasRecordadas?       migra desde domain.Jugador.plazasRecordadas — existente, cambia de dueño
  exploracionPersonal?    migra desde domain.Jugador.exploracionPersonal — existente, cambia de dueño
  heridoHasta?           Instante — debuff temporal (antes vivía en Escuadron, ver §13) — nuevo. Dura 2
                         minutos de tiempo de MUNDO (decisión del usuario 2026-09-13; 2 ticks). Lo reciben
                         todos los héroes del bando perdedor de cualquier batalla; mientras dura no puede
                         ser perseguido, perseguir ni entrar en batallas. Sustituye a
                         `Ejercito.enTreguaHasta` (la Tregua desaparece, Doc 5.16.4).
  escuadrones: Escuadron[]   TODAS sus escuadras, estén donde estén (§13) — nuevo
  loadouts: Loadout[]     nuevo
  inventario: ItemInstancia[]     lo que lleva y NO tiene puesto (§12.1) — nuevo
  equipamiento: Record<SlotEquipo, ItemInstancia | null>   lo que tiene puesto (§12.1) — nuevo. Equipar SACA
                                  el objeto del inventario y lo guarda entero en su hueco; desequipar lo
                                  devuelve a una casilla libre (así lo hace Conquest, `EquipmentManagerService`)
  monedasHeroe: { bronce, plata, oro }   nombres explícitos, economía DISTINTA del oro/recursos de
                                        BronzeAge, sin relación con el oro recurso (decisión del usuario
                                        2026-09-13) — nuevo
```

### 12.1 Inventario y equipo (forma de Conquest, `InventoryItem` / `Equipment`)

```text
ItemInstancia
  itemDefinitionId        catálogo versionado de objetos de Conquest (`itemId`)
  tipo: 'arma' | 'armadura' | 'consumible' | 'visual'   (`ItemType`, sin su valor `None`)
  cantidad                 entero ≥ 1; lo no apilable lleva 1
  itemInstanceId?          solo el equipo único (`instanceId`), único dentro de gameId; ausente = apilable
  estadisticas?: [{ nombre, valor }]   solo en equipo único (`serializedStats`). Se generan al crear la
                           instancia, así que son DATOS de ese objeto, no un cálculo derivado: dos objetos de
                           la misma definición pueden salir distintos
  precio                   fijado al crear la instancia; por unidad si es apilable (`price`)
  casillaInventario        posición en la rejilla del inventario; -1 si no ocupa casilla (`slotIndex`)
```

- **Apilables** (consumibles y similares) se identifican por `itemDefinitionId`; **equipo único**, por
  `itemInstanceId`.
- **`SlotEquipo`** — cerrado, los seis huecos de `Equipment`: `'arma' | 'casco' | 'torso' | 'guantes' |
  'pantalones' | 'botas'`. Se declara como `Record<SlotEquipo, true>` para forzar exhaustividad al compilar.
- Qué objeto va en qué hueco (tipo, categoría de arma o armadura, compatibilidad del arma con armadura
  ligera/media/pesada) lo decide el **catálogo de objetos de Conquest**. BronzeAge lo consulta en la versión
  vigente para validar `equipar` y el botín (§15); no lo redefine.
- La rejilla del inventario tiene tamaño limitado, y ese tamaño lo fija Conquest.
- Lo único derivado, y por eso no persistido, son los atributos efectivos con el equipo puesto: se recalculan
  contra el catálogo vigente (`HeroSnapshot.atributosEfectivos`, §15).

**Campamento del héroe (decisión del usuario, 2026-09-13).** Es donde guarda las escuadras que no lleva
consigo, y coincide con su residencia: no es un campo nuevo, se deriva de dónde reside (§3). Si el héroe no
reside en ningún sitio, no tiene campamento (huérfano) y sus escuadras siguen siendo suyas hasta que vuelva a
residir en algún asentamiento. Las reglas de juego completas (qué pasa al caer su asentamiento, guarnición,
escolta) están en `Docs/Game/5_Sistema_Militar_y_Combate.md` §5.15.

## 13. `Escuadron` — campos v1 (actualizado)

```text
Escuadron
  id                     squadId, único y estable dentro de gameId (§1) — existente
  heroeId                dueño (antes jugadorId) — existente, cambia de dueño
  tropaId                catálogo versionado, sin cambios — existente
  nombre                  existente
  cantidad               contador agregado, sin unidades individuales persistentes — existente
  nivel, experiencia     `experiencia` es ACUMULADA (nunca baja), reemplaza a `veterania` — nombre
                        deliberadamente distinto de `Heroe.experienciaHaciaSiguienteNivel` (§12), semántica
                        diferente — nuevo. La XP ganada en batalla la calcula Unity y llega en el
                        `BattleResult` (§15); BronzeAge la suma y aplica la curva de nivel.
  moral                   existente
  habilidadesDesbloqueadas[], formacionesDesbloqueadas[], formacionSeleccionada   nuevo — como las persiste
                        Conquest (`SquadInstanceData`): habilidades por id de texto; formaciones como índices
                        en la lista de la definición de escuadra (`permittedFormationIndexes`,
                        `selectedFormationIndex`)
  contenedor: { tipo: 'campamento' } | { tipo: 'ejercito'; ejercitoId } | { tipo: 'escolta'; caravanaId }
                        nuevo — dónde está FÍSICAMENTE. `'campamento'` = en el campamento del héroe (su
                        residencia, §12), o en ninguna parte si el héroe es huérfano.
  enGuarnicion: boolean   nuevo — solo con contenedor `'campamento'` y héroe residente: asignada a la
                        guarnición de su asentamiento. La maneja la IA de juego y el héroe no puede usarla
                        mientras siga asignada (Doc 5.15).
  reservaBatalla?: { battleId }   nuevo — candado: presente solo mientras la escuadra está en una
                        `Batalla` (§15). Una batalla nunca cambia de sitio una escuadra (el atacante la lleva
                        en su columna, el defensor la tiene donde está, la guarnición en su asentamiento, la
                        escolta en su caravana), así que no hace falta recordar a dónde volver: se quitó
                        `contenedorOrigen` (2026-09-13). Lo que pasa con las escuadras del bando que pierde
                        un asentamiento es una regla de juego aparte, Doc 5.15.
```

**Almacenamiento (cambia el 2026-09-13).** Todas las escuadras de un héroe viven en `Heroe.escuadrones`;
`Ejercito` y `Caravana` pasan a guardar solo los `squadId` que llevan. Antes se proponía mantenerlas
embebidas en su contenedor (`Asentamiento.escuadrones`...), pero en el modelo de héroe una escuadra puede no
tener asentamiento (héroe huérfano) y el campamento se traslada con el héroe. Guardarlas bajo el héroe hace
que trasladar un campamento no mueva nada y que la unicidad por `tropaId` sea una búsqueda en una sola lista.

**Sin `heridoHasta`** — se elimina de `Escuadron` (gana el modelo de BronzeAge). Una baja de escuadra
reduce `cantidad` de forma permanente, nunca hay estado intermedio de "herida". El adaptador de Conquest
debe dejar de interpretar `SquadInstanceData.unitsInjured` como resultado persistente compartido.

**Regla (decisión del usuario, 2026-09-13 — corrige la versión anterior, que permitía varias): un héroe
tiene como mucho UNA `Escuadron` por `tropaId` en toda la partida.** `tropaId` es el tipo de tropa
(`honderos`, `arqueros`...) y el par (`heroeId`, `tropaId`) es único dentro de `gameId`, esté la escuadra
en el contenedor que esté. Es la regla que el juego ya tenía (Doc 2.5, comentario de `Escuadron` en
`types.ts`); lo que fallaba era dónde se comprobaba.

Lo que señaló Codex (corregido en BronzeAge el 2026-09-13, sin esperar a `heroeId`): la unicidad solo se
comprobaba en la lista del asentamiento. Si la escuadra estaba fuera (en un `Ejercito` o de escolta en una
`Caravana`), la búsqueda no la veía y reclutar ese tipo creaba una segunda. Hoy `reclutarTropa`
(`engine/tropas.ts`) recibe el resto de la partida y rechaza si la escuadra existe en otra guarnición, ejército
o escolta; la reserva para una `Batalla` tendrá que sumarse a esa búsqueda cuando exista. Antes del arreglo, al
volver, los dos caminos de regreso hacían cosas distintas:
`devolverEscoltaAGuarnicion` (`engine/caravanas.ts`) las fusiona (suma efectivos, se queda con la mejor
veteranía y la peor moral, y una de las dos identidades desaparece) y `absorberColumna`
(`engine/ejercitos.ts`) las concatena (quedan dos, y solo la primera se puede reponer).

Solución: la unicidad se comprueba contra TODAS las escuadras del héroe, estén donde estén (con
`contenedor` es una búsqueda directa). Si ya tiene una de ese tipo, reclutar solo puede REPONERLA, y solo
donde está físicamente: en su guarnición, o en una plaza propia donde esté su columna, que es lo que ya
permite `solo_reponer`. Si está lejos, no se recluta otra. Como nunca pueden existir dos, volver nunca
obliga a fusionar: los dos caminos de regreso pasan a ser un simple movimiento, y la rama de fusión de
`devolverEscoltaAGuarnicion` desaparece en el modelo nuevo. `squadId` sigue siendo el ID opaco que
referencian tickets y resultados (el nombre es editable), aunque en la práctica (`heroeId`, `tropaId`)
funciona como clave natural.

### Equivalencia de tropas con Conquest (catálogo puente, R04 — añadido 2026-09-13)

BronzeAge es dueño de las reglas estratégicas de cada tropa: qué edificio la recluta, coste, unidades por
escuadrón, coste de Liderazgo y escalón. Conquest es dueño de las tácticas: daño, formaciones, habilidades,
movimiento en batalla y prefab. El `id` canónico de una tropa es el `tropaId` de BronzeAge, y Conquest crea
una definición de escuadra con ese mismo `id` para cada una. Las unidades y el coste de Liderazgo los
decide BronzeAge: Conquest recibe las unidades en `SquadSnapshot.efectivosAutorizados` y no usa su propio
`leadershipCost` para nada que tenga autoridad (el Liderazgo lo valida BronzeAge).

Hoy Conquest tiene 3 definiciones (`spm01` Spearmen, `arc01` Levy Archers, `sqd01` Squires) y BronzeAge 11
tropas. La última columna es la propuesta de qué definición actual sirve de base provisional a cada una
hasta que Conquest haga la suya (pedido en CQ-003). Esos tres `id` actuales quedan como alias durante la
migración.

| `tropaId` | Nombre | Escalón | Unidades | Coste de Liderazgo | Tipo | Base provisional en Conquest |
|---|---|---:|---:|---:|---|---|
| `milicia_lanceros` | Milicia de lanceros | 1 | 25 | 7 | cuerpo a cuerpo | `spm01` Spearmen |
| `lanceros_mimbre` | Lanceros con escudo de mimbre | 1 | 25 | 7 | cuerpo a cuerpo | `spm01` Spearmen |
| `espadachines_cobre` | Espadachines de espada corta de cobre | 2 | 20 | 14 | cuerpo a cuerpo | `sqd01` Squires |
| `hacheros_ligeros` | Hacheros ligeros | 3 | 18 | 22 | cuerpo a cuerpo | `sqd01` Squires |
| `espadachines_bronce` | Espadachines con espadas y escudos de bronce | 3 | 18 | 22 | cuerpo a cuerpo | `sqd01` Squires |
| `lanceros_pesados` | Lanceros pesados micénicos | 4 | 15 | 32 | cuerpo a cuerpo | `spm01` Spearmen |
| `hacheros_armados` | Hacheros armados | 4 | 15 | 32 | cuerpo a cuerpo | `sqd01` Squires |
| `honderos` | Honderos | 2 | 20 | 14 | a distancia | `arc01` Levy Archers |
| `escaramuzadores_jabalina` | Escaramuzadores con jabalina | 3 | 18 | 22 | a distancia | `arc01` Levy Archers |
| `arqueros` | Arqueros | 3 | 18 | 22 | a distancia | `arc01` Levy Archers |
| `arqueros_compuesto` | Arqueros con arco compuesto | 5 | 12 | 45 | a distancia | `arc01` Levy Archers |

Fuente: `TROPAS_RECLUTABLES`, `UNIDADES_POR_ESCALON` y `LIDERAZGO.costePorEscalon` (`src/constants.ts`), al
2026-09-13. Publicada, generada desde esas constantes, en `src/contratos/v1/catalogoTropas.json`: su `version`
(`VERSION_CATALOGO_TROPAS`) es la que cita `BattleRules.versionCatalogoTropas`. Falta acordar con Conquest cómo corresponde el escalón (1 leva … 5 élite) con su `SquadRarity`.

## 14. `Loadout`

```text
Loadout
  id                     loadoutId — nuevo
  heroeId                  nuevo
  displayName              nuevo
  squadIds[]              ordenados, referencian Escuadron.id — nunca copian la escuadra — nuevo
  perksSeleccionados[]     nuevo
  activo: boolean          nuevo
```

**`liderazgoTotal` NO es un campo persistido de `Loadout`** — corrección de Codex (R09): la versión
anterior lo listaba dentro de la estructura persistente después de decir, en la sección de `Heroe`, que no
se persisten atributos calculados; ambas frases no pueden ser ciertas a la vez. Es un valor DERIVADO que
solo aparece en el DTO de lectura (se calcula al servir la proyección) y en la validación del servidor al
activar un loadout — nunca en `domain/types.ts`, nunca verdad aceptada desde el cliente.

Traído de Conquest tal cual: preselección de escuadrones para salir a mundo abierto o unirse a un
`Ejercito`, limitada por el liderazgo total del héroe (`Heroe.liderazgoBase` + progresión).

## 15. `Batalla` / `BattleTicket` / `BattleResult`

**Esta sección se reescribió tras R01/R03/R04/R05 de `REVISION_CONTRATOS_CODEX_2026-09-11.md`** — la
versión anterior tenía una transición sin mensaje que la dispare, no garantizaba `battleId` único
globalmente, y el snapshot/resultado tácticos no bastaban para reproducir combate ni para validar
completitud. Corregido abajo.

### `Batalla` (persistente) — ciclo de estados corregido

```text
Batalla
  id: battleId            OPACO Y GLOBALMENTE ÚNICO (UUID) — corrección R03: NO se genera con
                          `session/idGenerator.ts` (ese contador es secuencial POR PARTIDA, no sirve para
                          un id que rutas/mensajes server-a-servidor referencian sin `gameId` al lado)
  gameId
  estado: 'convocando' | 'asignada' | 'en_curso' | 'aplicada' | 'cancelada' | 'fallida'
  ticketRevision: number    empieza en 0; sube si hace falta una revisión nueva del ticket (§ABAJO)
  huellaTicket             hash del BattleTicket vigente para la ticketRevision actual
  intentoAsignacionId?      del BattleServerAssignment activo — ver abajo
  expiraEn: Instante        fijado al crear/reasignar — timeout de infraestructura (5 minutos), tiempo de MUNDO
                          (ver nota de reloj en §0/§12: no confundir con expiración de credenciales)
  iniciadaEn?: Instante      al pasar a `en_curso`
  limiteEnCurso?: Instante   iniciadaEn + duración máxima de `BattleRules` + margen (5 minutos) — tiempo de MUNDO
  bandos: BatallaBando[]     capacidad independiente por bando (asimétrico permitido)
  participantes: BatallaParticipante[]   por heroeId (humanos y bot), nunca jugadorId directo
  reservas: BatallaReserva    escuadras/suministro inmovilizados, liberables
  appliedResultId?          idempotencia — ver §16
  huellaPayloadAplicado?    hash del BattleResult ya aplicado
  instanteAplicado?, versionAplicada?
```

**Ciclo y transiciones (R01; tabla añadida el 2026-09-13):**

| Desde | Hasta | Quién y cómo | Condición | Qué se guarda |
|---|---|---|---|---|
| — | `convocando` | BronzeAge, al comprometer el combate (comando de ataque o de asedio) | participantes y escuadras válidos y sin otro candado | la `Batalla`, los candados `reservaBatalla`, el `BattleTicket` (revisión 0) y `expiraEn` = ahora + plazo de asignación |
| `convocando` | `asignada` | Conquest, `POST /v1/batallas/:battleId/asignacion` | `ticketRevision` vigente y ninguna asignación aceptada antes | `BattleServerAssignment`, `intentoAsignacionId`; `expiraEn` = ahora + plazo de inicio |
| `asignada` | `en_curso` | Conquest, `POST .../inicio` | mismo `intentoAsignacionId` y revisión vigente | `iniciadaEn` y `limiteEnCurso` |
| `en_curso` | `aplicada` | Conquest, `POST .../resultado` | checklist de doc 02 §3.3 | consecuencias, XP, liberación de candados e idempotencia (§16), todo en una sola mutación |
| `convocando` o `asignada` | `convocando` (revisión + 1) | BronzeAge, al sustituir a un participante antes del inicio | antes de `en_curso` | ticket nuevo y `huellaTicket`; la asignación y los tokens anteriores dejan de valer |
| `convocando` o `asignada` | `cancelada` | BronzeAge, comando `cancelarBatalla` | antes de `en_curso` | libera los candados y deja un evento |
| `convocando` o `asignada` | `fallida` | BronzeAge, al vencer `expiraEn` | nadie asignó o nadie inició a tiempo | libera los candados sin penalizar a nadie y deja un evento auditable |
| `en_curso` | `fallida` | BronzeAge, al vencer `limiteEnCurso` | el servidor de batalla no mandó resultado | igual que la fila anterior |

`aplicada`, `cancelada` y `fallida` son estados finales. Un `POST .../resultado` repetido sobre una batalla
`aplicada` responde con el resultado ya aplicado (§16); sobre una `cancelada` o `fallida` se rechaza y no
aplica nada.

**Cuándo se comprueban los vencimientos (1.ª revisión de Codex, punto 5):** en cada tick, como el resto de
plazos de mundo, y además justo antes de validar cualquier acción que quiera usar una escuadra con candado.
Así una batalla vencida nunca deja una escuadra bloqueada solo porque nadie haya consultado su ficha. Cada
paso a `fallida` deja un evento persistente y auditable.

**Si se cae BronzeAge durante una batalla (R07):** mientras está caído, el tiempo de mundo no avanza
(`Docs/Arquitectura/10_Modelo_Temporal.md`, "una caída no consume tiempo de mundo"), así que `expiraEn` y
`limiteEnCurso` tampoco: al volver, la batalla sigue `en_curso`. La partida en Unity no depende de BronzeAge
y se sigue jugando. Conquest reintenta el `POST .../resultado`, espaciando cada vez más los intentos, hasta
recibir una respuesta definitiva (2xx, o un 409 que no sea transitorio), y BronzeAge lo acepta al volver. Los
tokens de entrada caducan en tiempo real (§0): una caída larga solo impide entrar tarde a la partida, no
afecta a quien ya está jugando. Tras una caída corta, el motor recupera de golpe los ticks pendientes; el
margen de `limiteEnCurso` existe para que esa recuperación no dé por fallida una batalla que ya terminó.

**Si se cae el servidor de batalla:** si el resultado no llega nunca, al vencer `limiteEnCurso` la batalla
pasa a `fallida`, sin penalizar a nadie.

Los plazos (de asignación, de inicio y el margen) son constantes de configuración; no se fijan aquí.

- **Se elimina `propuesta`** — el lobby/convocatoria vive FUERA de `Batalla` (`Ejercito`/`participantes`,
  sin persistencia nueva); no hay una fase "propuesta pero sin reservar" dentro de `Batalla`, porque
  `Batalla` nace ya reservada. `convocando` es el primer estado real: creada y reservada, ticket publicado,
  esperando que el orquestador de Conquest la recoja y asigne instancia.
- **`asignada -> en_curso` necesita un mensaje que hoy faltaba (R01):** Conquest confirma el INICIO real de
  la partida — no basta con haber recibido la asignación, la partida puede tardar en arrancar de verdad.
  Nuevo mensaje servidor-a-servidor `POST /v1/batallas/:battleId/inicio` (doc 02 §3.2), autenticado igual
  que asignación/resultado, con `intentoAsignacionId` para que solo el servidor que de verdad recibió esta
  asignación pueda confirmar el inicio.
- **Se elimina el estado `finalizada` como checkpoint persistido separado (adopta la recomendación de
  Codex):** `en_curso -> aplicada` es UNA sola transición atómica — el mismo comando que valida el
  `BattleResult` (checklist de §"BattleResult" abajo) aplica todas sus consecuencias (bajas, XP, herida,
  conquista, liberación de reservas) y persiste el nuevo estado `aplicada` en una única
  `aplicarYPersistir`, igual que CUALQUIER otro comando de este motor (`server/runnerDePartida.ts`): si la
  persistencia falla, se revierte al snapshot anterior — no queda un estado intermedio "aplicado a medias"
  que necesite recuperación aparte, porque el motor ya no lo permite para ningún comando. No hace falta
  inventar un mecanismo de recuperación nuevo: es la garantía que el sistema ya tiene.

### `BattleTicket` (DTO inmutable, publicado por BronzeAge) — snapshot corregido

```text
BattleTicket
  schemaVersion, battleId, gameId, ticketRevision
  contextoEstrategico        asedio { asentamientoId } | campo_abierto { punto } | caravana { caravanaId, punto }
                             | campamento_bandidos { campamentoId, punto }
  bandos: { atacante: BattleSide, defensor: BattleSide }
  mapa: BattleMapReference | SettlementBattleSnapshot   BattleMapReference = { mapaId, centro }: Unity monta el
                             terreno desde GET .../mapa/:mapaId. SettlementBattleSnapshot, en §17
  reglas: BattleRules         ver abajo

BattleSide
  faccionId                  null si el bando no es de nadie (campamento de bandidos)
  capacidadMaxima            en héroes, ver abajo
  participantes: BattleParticipantSnapshot[]
  escuadrasSinHeroe: SquadSnapshot[]
```

**Forma exacta:** `src/contratos/v1/contratos.schema.json` (JSON Schema draft-07, una entrada de `definitions`
por entidad), con su espejo en TypeScript (`dto.ts`) y un fixture de cada mensaje en `fixtures/`. Los lados
son siempre `atacante` y `defensor` (Doc 5.2: dos bandos, sin empates).

**Corrección R03 — revisión del ticket:** BA-001 permite sustituir un participante antes de empezar
(desconexión, etc.); eso sube `ticketRevision` y cambia `huellaTicket`. Una `BattleServerAssignment` o
`BattleResult` que referencie una `ticketRevision` distinta de la vigente en `Batalla` se **rechaza** — así
una asignación vieja no puede cerrar una revisión nueva.

**`BattleParticipantSnapshot`:** `heroeId`, `controlador` (`'humano'` | `'bot'`), `heroe: HeroSnapshot`,
`escuadras: SquadSnapshot[]` (solo las que lleva ese héroe, limitadas por su liderazgo). El lado lo da el
bando en el que va.

**Composición de un bando (decisión del usuario, 2026-09-13, Doc 5.15):** héroes (humanos o bot) con sus
escuadras, más `escuadrasSinHeroe` (la guarnición del asentamiento, la escolta de la caravana o las tropas de un campamento de bandidos), que maneja
la IA de juego. `capacidadMaxima` cuenta HÉROES (15 por bando en un asedio; 5 en mundo abierto, contra una caravana o contra un campamento de bandidos —
Doc 5.15.1): las escuadras sin
héroe no ocupan plaza y entran directamente. Los héroes que superan la capacidad esperan en cola y entran a
medida que caen otros.

**`HeroSnapshot` — forma mínima (R04, antes indefinida):** `displayName`, `classDefinitionId`, `nivel`,
`genero` y `avatar` (para dibujarlo en la batalla), `atributosEfectivos` (post-equipo, recalculados por
BronzeAge contra el catálogo vigente — nunca copiados de un cálculo del cliente), `perksDesbloqueados[]`,
`equipamiento` (igual forma que `Heroe.equipamiento`, §12.1: cada objeto entero, con sus estadísticas) y
`casillasInventarioLibres` (cuánto botín le cabe, ver `BattleResult`). El `heroeId` va en el participante, y
las versiones de catálogo, una sola vez por ticket en `BattleRules`.

**`SquadSnapshot` — forma mínima (R04, antes solo tenía `squadId`+cantidad+`tropaId`):** `squadId`,
`heroeId` (su dueño; `null` en las tropas sin dueño), `tropaId`, `efectivosAutorizados` (renombrado desde
"desplegados" — es la fuerza RESERVADA/autorizada, no necesariamente la que Unity ponga en juego de una vez,
ver `BattleResult` abajo), `nivel`, `experiencia`, `moral`, `habilidadesDesbloqueadas[]`,
`formacionesDesbloqueadas[]` y `formacionSeleccionada` (§13). Sin esto, dos escuadras del mismo `tropaId` con progresión distinta serían
indistinguibles para inicializar sus capacidades de combate — el catálogo del tipo de tropa por sí solo no
basta (Conquest ya conserva esta progresión en `SquadInstance.Data.cs`, no hay que reinventarla, solo
transportarla).

**`BattleRules` — forma mínima (R04, antes indefinida):**

```text
BattleRules
  duracionMaximaSegundos     1800 en un asedio, 900 en el resto (Doc 5.15.1)
  versionBalance             BALANCE_VERSION de BronzeAge
  versionCatalogoTropas      version de catalogoTropas.json (§13)
  versionCatalogoHeroe, versionCatalogoObjetos   versiones de los catálogos de Conquest (texto)
```

La única regla de victoria que fija BronzeAge es fija y no viaja en el ticket: si se agota el tiempo, gana el
defensor, en cualquier batalla; el atacante es siempre quien inicia el combate (Doc 5.15.1). El resto (capturar
objetivos, aniquilar) es táctico de Conquest. En v1 no hay tope de XP ni de monedas por batalla (hasta que Conquest publique su curva de XP,
CQ-001); cuando lo haya se añadirá como campo opcional. Las capacidades asimétricas por bando ya viven en
`bandos.*.capacidadMaxima` (BA-001), no se duplican aquí.

**Escuadras sin héroe (escolta y guarnición):** combaten sin su héroe, manejadas por la IA de juego, y van
en `escuadrasSinHeroe` del bando. El `SquadSnapshot` lleva su `heroeId` como dueño, pero ese héroe no es
participante. Son los dos únicos casos en que una escuadra combate sin su héroe (Doc 5.15).

**Tropas de un campamento de bandidos (decisión del usuario, 2026-09-14):** un héroe que ataca un campamento
combate en Unity (Doc 1.9). Las tropas del campamento van también en `escuadrasSinHeroe`, pero sin `heroeId`:
no son de nadie y no persisten entre batallas. Composición v1: una escuadra de milicia de lanceros con 15
unidades, el mismo poder (30) que el campamento tiene hoy (Doc 1.9). Igual van los **carreteros** de una
caravana sin escolta atacada por un héroe: una escuadra de milicia con 13 unidades, sin dueño (Doc 3.10).

**Quién tiene que haber en una batalla (corregido 2026-09-13).** Al menos un héroe humano en toda la
batalla. Un bando puede no tener ninguno: solo héroes bot, solo escuadras sin héroe (un asentamiento sin
defensores presentes, una caravana con escolta) o incluso nadie (un asentamiento sin guarnición ni
defensores: la batalla se juega igual, sin defensores). La versión anterior exigía un humano por bando, lo que
dejaba fuera la escolta y la guarnición. Sin ningún humano (NPC contra NPC) se sigue resolviendo con el
resolver numérico actual (`engine/combate.ts`) y nunca llega a Unity.

### `BattleResult` (DTO, producido por el servidor de batalla Unity) — completitud corregida

```text
BattleResult
  schemaVersion, battleId, resultId, ticketRevision
  intentoAsignacionId        el de la asignación activa (doc 02 §3.3, punto 10)
  inicio, fin                tiempo REAL UTC, ISO 8601 (§0): el servidor de batalla no conoce el reloj de mundo
  ganador                    'atacante' | 'defensor' — sin empates (Doc 5.2)
  razon                      'objetivos_capturados' | 'aniquilacion' | 'tiempo_agotado'
  objetivos: [{ objetivoId, capturadoPor: lado | null }]   informativo: BronzeAge no aplica nada a partir de
                             ellos, y sus ids son del mapa táctico de Conquest
  porEscuadra: [{ squadId, desplegados, supervivientesAlCierre, muertos, xpGanada }]
  porHeroe: [{ heroeId, participo, sobrevivioAlCierre, xpGanada, botin? }]
  versionServidor            build del servidor de batalla
```

Sin campo `autenticidad`: el resultado lo autentica la credencial de la cabecera más `intentoAsignacionId`
(doc 02 §3.3).

**Corrección R05 — completitud obligatoria:** `porEscuadra` debe traer una entrada por CADA `squadId` que
figure en las reservas del ticket, incluso si nunca se desplegó (entrada con `desplegados: 0`,
`supervivientesAlCierre` y `muertos` en 0). Omitir una escuadra reservada, o reportarla con ceros sin haber
jugado, deja de ser una liquidación válida — la validación semántica de doc 02 §3.2 lo exige explícito.

**Terminología corregida:** `desplegados` es informativo (lo que Unity de verdad puso en juego, puede
variar a lo largo de la partida por redespliegues) — NO autoritativo y NO el mismo número que
`efectivosAutorizados` del ticket (que es la fuerza RESERVADA antes de jugar). El invariante de
conservación se cierra sobre lo autorizado, no sobre lo desplegado:

```text
supervivientesAlCierre + muertos == efectivosAutorizados     (del SquadSnapshot correspondiente)
```

Esto cubre retirar una escuadra viva y redesplegarla sin duplicar sus bajas ni sus supervivientes — se
cuenta una sola vez, al CIERRE de la batalla, nunca por evento de despliegue intermedio.

**`sobrevivioAlCierre` de un héroe (R05):** describe solo el estado FINAL al cerrar la partida — si el
servidor de batalla permite respawn del héroe durante la partida, eso es asunto táctico de Unity y no se
reporta aquí; BronzeAge solo necesita saber si terminó vivo.

**Herido del héroe (decidido 2026-09-13, Doc 5.16.4):** no lo manda Unity. BronzeAge lo aplica al aplicar
el resultado: todos los héroes del bando perdedor quedan heridos 2 minutos de mundo (`Heroe.heridoHasta`).
Mientras dura no pueden ser perseguidos, perseguir ni entrar en batallas: los comandos que lo intenten se
rechazan, y un héroe herido no puede figurar en un `BattleTicket` nuevo. Por eso `porHeroe` ya no lleva el
campo `herido`. Sustituye a la Tregua de `Ejercito`.

**XP — la calcula Unity (decisión del usuario 2026-09-13; sustituye a la regla anterior de "BronzeAge
calcula la XP a partir de participó/ganó/duración").** La XP depende del desempeño en batalla: unidades y
héroes abatidos, capturas de bandera, daño hecho y recibido, MVP, puesto en la tabla de su bando, etc.
Todo eso solo lo tiene el servidor de batalla, así que `xpGanada` llega ya calculada por héroe y por
escuadra. Cómo se aplica:

- Es un DELTA de la batalla, nunca un total: BronzeAge lo suma a la experiencia persistida y aplica la curva
  de nivel del catálogo versionado (`versionCatalogoHeroe`/`versionCatalogoTropas`). Al ser delta,
  reintentar el mismo `resultId` no duplica nada (§16) y no pisa progreso ganado en otro sitio.
- Viene del servidor de batalla autenticado, nunca de un cliente Unity individual — el mismo nivel de
  confianza que ya tiene para declarar muertos y ganador (`02_Arquitectura_objetivo.md`).
- BronzeAge la valida igual que el resto del resultado: entero, ≥ 0 y, si `BattleRules` define un tope de
  XP por batalla (valor de balance, no fijado aquí), por debajo de ese tope.
- Los hechos que usa Unity para calcularla (bajas atribuidas, capturas, daño...) no tienen que viajar a
  BronzeAge: solo el resultado.

**Botín — lo decide Unity, igual que la XP (decisión del usuario, 2026-09-14).** Unity sabe qué ganó cada
héroe en la partida; BronzeAge lo guarda, porque el inventario y las monedas del héroe viven en él (§12).

```text
botin
  objetos: [{ itemDefinitionId, cantidad, itemInstanceId?, estadisticas? }]   forma de ItemInstancia (§12.1)
                                        sin tipo, precio ni casilla: los completa BronzeAge desde el catálogo
                                        y al colocarlo en el inventario
  monedas: { bronce, plata, oro }       suma a `monedasHeroe`
```

- Es un DELTA, como la XP: se suma a lo que el héroe ya tiene, y reintentar el mismo `resultId` no lo
  duplica (§16).
- Solo lo reciben héroes con `participo: true`.
- Unity no da más objetos de los que caben (`HeroSnapshot.casillasInventarioLibres`). Un botín que no cabe
  invalida el resultado entero, igual que cualquier otra cifra por encima de lo autorizado.
- Cada `itemDefinitionId` existe en `versionCatalogoObjetos` del ticket, y cada `itemInstanceId` es nuevo en
  la partida. Las estadísticas de un objeto nuevo no se validan: vienen del servidor de batalla autenticado,
  con la misma confianza que la XP.
- Qué se gana y cuánto es balance de Conquest. Si en la partida se gastan consumibles o se pierde equipo, v1
  no lo recoge: está preguntado en CQ-004.

**Daño de asedio — lo decide BronzeAge y no viaja en el resultado (R05; decisión del usuario 2026-09-14).**
Si el atacante conquista, BronzeAge aplica el saqueo determinista que ya existe en el motor (Doc 5.12.9,
`OCUPACION`): nada se destruye; una parte de los edificios queda `danado`, a reparar pagando de nuevo una
fracción de su coste, y cada recinto de muralla pierde parte de su `avance`. Si el asedio resiste, no hay
daño. Unity no reporta daño de edificios ni de murallas.

El resultado contiene hechos tácticos más la XP ganada y el botín (las dos excepciones al "solo hechos",
decididas por el usuario el 2026-09-13 y el 2026-09-14). BronzeAge aplica esa XP con la curva de nivel y
guarda el botín, y sigue decidiendo por su cuenta la
herida de los héroes del bando perdedor, conquista, ocupación y liberación de reservas a partir de los hechos, contra
los catálogos vigentes. Qué les pasa a los héroes, escuadras y guarnición del bando que pierde un
asentamiento está en Doc 5.15. Si la batalla fue en mundo abierto, el héroe derrotado pierde además la mitad
de su carro (Doc 5.16.6).

### `BattleServerAssignment`

```text
BattleServerAssignment
  battleId, ticketRevision      debe coincidir con la revisión vigente — corrección R03
  intentoAsignacionId            identifica esta asignación concreta; solo el productor que la recibió
                                puede luego confirmar inicio/resultado con este mismo id — corrección R03
  instancia (host, puerto, protocolo)
  tokensParticipante: [{ heroeId, token, expiraEn: string (ISO 8601, tiempo REAL, no Instante — ver §0) }]
                                solo héroes humanos (los bot no se conectan); alcance limitado a esta
                                battleId+ticketRevision
```

La credencial servidor-a-servidor no viaja en el cuerpo: el servidor de batalla la presenta en la cabecera
(`Authorization: batalla-servidor <token>`, doc 02 §3.3). BronzeAge anota qué servidor registró la asignación,
y solo ese puede confirmar el inicio y mandar el resultado. `POST .../inicio` lleva `battleId`,
`ticketRevision` e `intentoAsignacionId` (`InicioBatalla` en el schema).

Dos clases de autorización, nunca la misma credencial para las dos — ver doc 02 §3. Si `ticketRevision`
sube, toda asignación/token de la revisión anterior queda invalidado automáticamente — no hace falta un
mensaje de revocación aparte.

## 16. Idempotencia de `BattleResult`

`Batalla.appliedResultId` + `huellaPayloadAplicado` (hash) + `instanteAplicado` + `versionAplicada` viven en
la propia `Batalla`, persistidos — **no** en la caché en memoria de `RunnerDePartida.idempotencia` (máx.
500 entradas, se pierde al reiniciar). Mismo `resultId`+hash → éxito previo sin mutar, respondiendo el
resultado ya aplicado. Mismo `resultId` con hash distinto → se rechaza y se audita. Aplicar consecuencias y
pasar a `aplicada` quedan en la MISMA mutación persistida (§15, ciclo corregido) — no existe un estado
`finalizada` intermedio que recuperar por separado.

## 17. Campos 3D del asentamiento (nuevo, complementa §3)

El contrato de grilla/edificios/murallas para reconstrucción 3D existe casi completo en §3. Campos NUEVOS
que faltan añadir (opcionales, no rompen nada):

- `Edificio.visualSeed` (§3), `Asentamiento.visualCatalogVersion` (§3) — elección determinística de
  variante 3D por `edificioId`. Deben viajar en el snapshot que consume Unity, no quedarse solo internos.
- Identidad de `CeldaMuro` para Unity: `recintoId` + índice de recorrido dentro de `celdas` (§3); se añade
  un `id` explícito por celda solo si el consumo desde Unity lo simplifica.
- `Asentamiento.layoutVersion` (§3) + seed/perfil de trazado — calles/trazado son datos DERIVADOS, no
  persistidos punto a punto; sin esto Unity no puede reconstruirlas idénticas con una versión distinta del
  algoritmo. **Parcial (BA-005):** existe como constante global `LAYOUT_VERSION` (`constants.ts`), no por
  asentamiento — hay una sola geometría por build. Se publica en `GET /v1/balance` (`geometriaUrbana`), viaja
  en el export del laboratorio y el snapshot de partida la guarda: `cargarPartida` rechaza una distinta.
- Factor de conversión celda→unidad Unity — **definido (BA-005):** una celda = `REJILLA_ASENTAMIENTO.tamanoCelda`
  = 3 unidades locales; Unity aplica su propia escala `S` (3.5 hoy) y `(X,Z) = (x·S, −y·S)`. BronzeAge no
  conoce `S`.

**Invariante:** una variante 3D nunca cambia huella, acceso, producción ni colisión estratégica — solo
aspecto.

### `SettlementBattleSnapshot` — forma mínima (R10, antes indefinida)

Se congela al abrir la `Batalla` (§15), a partir del `Asentamiento` real en ESE instante, y viaja dentro
del `BattleTicket.mapa`:

```text
SettlementBattleSnapshot
  settlementId
  layoutVersion               revisión geométrica usada al congelar (LAYOUT_VERSION)
  unidadesPorCelda            3 (REJILLA_ASENTAMIENTO.tamanoCelda). Coordenadas locales: origen en el centro
                              del Centro Urbano, `y` hacia abajo, las mismas del fixture de BA-005
  edificios: [{ edificioId, tipo, posicion, ancho, alto, nivelInterno?, estado, danado? }]
                              solo los internos. `posicion` es el centro de la huella, en unidades; `ancho` y
                              `alto`, la huella en celdas con la rotación ya aplicada. Geometría EFECTIVA en
                              el instante de abrir — copia congelada, no una referencia viva al Asentamiento
                              (que puede seguir cambiando mientras la batalla está en curso)
  recintos: [{ recintoId, nivel, celdas }]
                              `nivel` es el nivel FÍSICO en pie y `celdas` solo las que están en pie, en su
                              orden de recorrido (el índice coincide con el del `Recinto`). Resuelve la
                              ambigüedad de §3: si hay `mejorandoA`, en pie está el anillo entero al nivel
                              anterior; si no, las celdas hasta `avance`. El lector no reinterpreta nada. La
                              celda (col, row) ocupa de (col, row) a (col + 1, row + 1), por unidadesPorCelda
```

Las puertas se leen de `celdas` (clase `'puerta'`). Queda fuera de v1, y se añadirá cuando exista: los
obstáculos tácticos del escenario (spawns, límites navegables), `visualSeed`/`visualCatalogVersion` (todavía
no están en el dominio) y las calles, que hoy Unity toma de `trazadoPorAsentamiento` de la proyección.

**Política explícita (R10):** cambios estratégicos en el `Asentamiento` real mientras la `Batalla` está en
curso (nueva construcción, mejora de muralla) NO tocan este snapshot ya congelado — se aplican al
`Asentamiento` real después, cuando la batalla se resuelve (`aplicada`/`cancelada`/`fallida`). Dos
reconstrucciones de la misma ciudad (vista normal vigente vs. escenario de asedio congelado) pueden diferir
mientras la batalla dura; eso es esperado, no un bug de sincronización.

**Daño de asedio:** lo aplica BronzeAge al conquistar, no Unity (§15): edificios dañados a reparar y murallas
con menos avance, nada destruido. Este snapshot no necesita ningún campo para ello.

## 18. Frontera cosmético/autoritativo de mundo

`WorldConfig` + `worldgenVersion` viajan a Unity; BronzeAge sigue siendo la fuente autoritativa de topología
estratégica, regiones, ríos, caminos, propiedad y zonas de influencia (`src/worldgen/` + `src/world/mapa.ts`,
sin cambios).

**Regla de frontera (no negociable):** cualquier relieve que cambie navegación, visibilidad o accesibilidad
deja de ser cosmético y debe provenir de datos autoritativos de BronzeAge. Unity solo genera microdetalle
(piedras sueltas, hierba, texturas) cuando de verdad no altera ninguna regla.

**Verificabilidad, no solo declaración (R10):** decir "misma seed y `worldgenVersion`" no prueba por sí
solo que la generación C# reproduce la TypeScript — hay que elegir entre servir datos canónicos/chunks
(BronzeAge genera y Unity solo consume, sin reimplementar el algoritmo) o exigir fixtures de PARIDAD
explícitos (mismo seed+worldgenVersion, mismo resultado exacto de terreno/ríos/biomas comparado byte a
byte entre ambos lados) antes de aceptar la generación determinista C# como válida. Sin uno de los dos, el
worldgen híbrido sigue aceptado como decisión de producto, pero no es todavía un contrato verificable —
pendiente: `src/contratos/v1/` todavía no lo cubre.

## 19. Visibilidad — qué ve Unity de todo esto

Ninguna entidad de arriba se sirve completa a cualquier cliente sin filtrar — es la misma regla de niebla de
guerra/ciudadanía que ya aplica hoy en `session/proyecciones/`, no algo nuevo que inventar para Unity.
**Corrección de Codex (R08):** la versión anterior describía esto solo como serialización/niebla de guerra;
es también un asunto de PERMISO, y hay que ser preciso con la diferencia:

- **Público sin restricción:** posición/nombre de `Asentamiento` y `Ejercito` propios visibles, `Faccion`
  (nombre/nivel/reputación), `ZonaFaccion` (fronteras dibujadas), `Titulo`.
- **Interior de un `Asentamiento` — es un PERMISO, no solo niebla de guerra.** La proyección real
  (`session/proyecciones/jugador.ts`) busca el interior detallado dentro de `asentamientosPropios` —
  residencia/ciudadanía del héroe, no "cualquier asentamiento donde el héroe esté físicamente parado en
  este instante". Estar de pie dentro de un asentamiento AJENO (uno donde no tiene residencia/ciudadanía) no
  concede por sí solo ver su almacén/cola con detalle — eso lo decide la política de acceso del
  asentamiento (`Asentamiento.politicaDeAcceso`, §3) y la ciudadanía, no la mera presencia. Este modelo NO
  amplía ese permiso: quien tenga acceso hoy en la proyección de Vite tiene el mismo acceso en el DTO de
  Unity, ni más ni menos.
- **Filtrado por niebla de guerra/memoria (dentro de los asentamientos PROPIOS):** si el héroe no está
  presente ahora en uno de sus propios asentamientos, se sirve la última foto memorizada
  (`Heroe.plazasRecordadas`, antes `Jugador.plazasRecordadas`) en vez del estado vivo.
- **Héroes ajenos (decisión del usuario, 2026-09-13; canon Doc 5.16.7):** de un héroe que puedes ver, ves
  nombre, clase, nivel, si está herido y hasta cuándo, los escuadrones que lleva consigo (tipo, unidades y
  nivel) y el equipo que lleva puesto. Todo lo demás del héroe es privado de su jugador. Dónde está sigue
  sujeto a la niebla de guerra. Forma del dato en doc 02 §4.1 (`HeroePublico`). El nombre de un compañero de
  Facción lo ven todos sus ciudadanos aunque no lo vean (decisión del usuario, 2026-09-14; `nombresDeCompaneros`).
- **Nunca sale al cliente:** hashes/secretos de `Usuario`, credenciales de `BattleServerAssignment`,
  `runtimeEntityId` de Conquest, cualquier campo interno de cálculo (ver `Titulo.valorMetrica` como derivado,
  no autoritativo).
- La proyección exacta por rol (jugador/observador/administrador) queda para cuando se diseñen los DTO de
  lectura general (`Docs/Arquitectura/5_Contratos_Identidad_Permisos.md`, punto 7; ver también doc 02 §"Extensión
  de lectura estratégica") — este documento fija QUÉ existe, no todavía el DTO de lectura filtrado campo por
  campo.

## 20. Invariantes generales (resumen)

- Un ID es opaco, estable y distinto del nombre visible — ningún contrato usa `displayName`/`nombre` como
  llave.
- Las cantidades persistentes nunca se deducen contando entidades ECS vivas en un cliente.
- Unity no envía autoridad persistente: solicita acciones y reporta resultados desde un servidor de batalla
  autenticado.
- Todo DTO lleva `schemaVersion`; las reglas relevantes llevan además su versión de balance.
- Cambios incompatibles crean una versión nueva — nunca se cambia en silencio el significado de un campo.
- Los modelos ECS son internos de Conquest y nunca forman parte del contrato de red.
- Sin migración de partidas existentes: si un snapshot viejo no carga con este modelo, se descarta y se crea
  uno nuevo desde cero.
