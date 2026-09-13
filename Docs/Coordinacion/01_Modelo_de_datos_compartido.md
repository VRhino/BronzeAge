# 01 — Modelo de datos compartido (v1)

**Estado:** ACEPTADO como diseño de producto (qué existe, quién es dueño de qué) — **no todavía como
contrato implementable e interoperable**. `Docs/Coordinacion/propuestas/REVISION_CONTRATOS_CODEX_2026-09-11.md`
encontró transiciones sin mensaje que las dispare, huecos de completitud y afirmaciones que no coincidían
con el código real; las secciones §12/§13/§15/§17/§19 llevan las correcciones de esa revisión inline,
marcadas donde siguen abiertas (`PENDIENTE`). Nada de lo nuevo (`Heroe`, `Batalla`, campos 3D) existe
todavía en `src/`. Todo lo demás SÍ existe hoy en `src/domain/types.ts` — se transcribe completo aquí
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
              ├── Escuadron[] (heroeId)
              └── Loadout[] (heroeId)
```

**No existe una entidad `Jugador` nueva en `domain/`.** Duplicaría `Membresia` (`acceso/tipos.ts`, ya une
`usuarioId`+`gameId`+`jugadorId`+rol con vigencia) — corrección de Codex, incorporada. `Heroe.jugadorId` es
el mismo valor que `Membresia.jugadorId`, no una copia con otro significado.

Resolución de un actor autenticado: `Sesion -> Usuario -> Membresia -> jugadorId -> Heroe`. Las reglas de
juego reciben `heroeId`; auditoría y autorización conservan también `usuarioId`/`jugadorId`.
`GameSessionState.jugadores` se reemplaza por `heroes`.

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
  escuadrones: Escuadron[]          guarnición — existente, dueño pasa a heroeId (§13)
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
  enTreguaHasta?: Instante     tras derrota reciente, corta perseguir Y ser perseguido — existente
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

## 12. `Heroe` — campos v1 (borrador de trabajo, NO lista cerrada)

**Corrección de Codex (R09):** llamar a esto "lista cerrada" antes de fijar los tipos abiertos de abajo era
prematuro. Sigue siendo borrador hasta que se resuelvan los puntos marcados `PENDIENTE`.

No se persisten caches ni atributos calculados — todo lo derivado (poder efectivo, liderazgo total de un
loadout) se recalcula contra el catálogo versionado vigente.

```text
Heroe
  id                    heroeId — nuevo
  jugadorId             dueño — Membresia.jugadorId, 1:1 por gameId — nuevo
  displayName           editable, nunca llave de ningún contrato — nuevo
  classDefinitionId      catálogo versionado (Conquest) — nuevo
  genero, avatar          nuevo
  nivel                   nuevo
  experienciaHaciaSiguienteNivel   nuevo. **Corrección R09:** "hacia el siguiente nivel", no acumulada
                                  desde el inicio — Conquest (`Hero.Data.cs`) ya usa esta semántica; se
                                  nombra explícito para no confundirla con `Escuadron.experiencia` (§13),
                                  que SÍ es acumulada. Nombres distintos a propósito.
  puntosDeAtributoSinGastar, puntosDePerkSinGastar   nuevo. **Corrección R09: DOS bolsas separadas**, no
                                  una — Conquest (`Hero.Data.cs:33-36`) ya las distingue; un solo
                                  `puntosSinGastar` perdería esa distinción al traer el modelo.
  atributosBase          valores base, sin equipo aplicado — nuevo
  perksDesbloqueados[]     nuevo
  tiposReclutablesDesbloqueados[]   nuevo — PENDIENTE: destino de `availableSquads` de Conquest (qué
                                  `tropaId` puede reclutar este héroe más allá del catálogo base del
                                  asentamiento). Sin decidir todavía si vive aquí o en otro sitio.
  liderazgoBase          migra desde el actual domain.Jugador.liderazgoBase — existente, cambia de dueño
  ubicacion               migra desde domain.Jugador.ubicacion (asentamiento | columna | desconectado) —
                         existente, cambia de dueño
  plazasRecordadas?       migra desde domain.Jugador.plazasRecordadas — existente, cambia de dueño
  exploracionPersonal?    migra desde domain.Jugador.exploracionPersonal — existente, cambia de dueño
  heridoHasta?           Instante — debuff temporal (antes vivía en Escuadron, ver §13) — nuevo
  loadouts: Loadout[]     nuevo
  inventario: ItemInstancia[]     por itemInstanceId — nuevo, forma PENDIENTE (ver abajo)
  equipamiento: Record<slot, itemInstanceId>   nuevo, `slot` PENDIENTE de enumerar
  monedasHeroe: { bronce, plata, oro }   nombres explícitos, economía DISTINTA del oro/recursos de
                                        BronzeAge — nuevo
```

**`ItemInstancia` — PENDIENTE de forma (R04/R09).** Al menos: `itemInstanceId`, `itemDefinitionId`
(catálogo versionado), cantidad o estadísticas únicas si el ítem las tiene. Slots de `equipamiento` deben
enumerarse cerrado (arma/armadura/accesorio/... — igual que `RolTecnico`/`EdificioTipo`, un
`Record<Slot, true>` que fuerce exhaustividad en compilación) antes de escribir el schema JSON. No se
persisten estadísticas ya calculadas con equipo puesto — se recalculan contra la versión de catálogo
vigente (mismo principio que el resto del documento).

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
                        diferente — nuevo
  moral                   existente
  habilidadesDesbloqueadas[], formacionesDesbloqueadas[], formacionSeleccionada   nuevo
  contenedor: { tipo: 'asentamiento' | 'ejercito' | 'escolta'; id }   nuevo — dónde vive FÍSICAMENTE
                        ahora mismo. Almacenamiento: SIGUE EMBEBIDO en la lista de su contenedor
                        (`Asentamiento.escuadrones`/`Ejercito.escuadrones`/`Caravana.escolta`), como hoy —
                        no se normaliza a una colección aparte. `contenedor` es la referencia de vuelta que
                        permite resolver un `squadId` sin recorrer los tres arrays; moverlo de un
                        contenedor a otro sigue siendo una única operación atómica de dominio (ya es así
                        hoy: `salirAlMundo`/`movilizarEjercito`/etc. ya mueven escuadrones enteros en un
                        solo comando).
  reservaBatalla?: { battleId; contenedorOrigen: { tipo; id } }   nuevo — presente SOLO mientras la
                        escuadra está reservada para una `Batalla` (§15). NO es un cuarto valor de
                        `contenedor` (corrección tras R09): la escuadra sigue registrada en su contenedor
                        real; esto es un candado encima que además recuerda a dónde volver. Si el origen
                        deja de existir o cambia de dueño mientras la batalla está en curso (ej. el
                        asentamiento es conquistado), la liberación sigue la misma regla ya usada por
                        `Ejercito.origenAsentamientoId`: se reasigna al asentamiento propio más cercano del
                        mismo héroe/facción; si no queda ninguno, la escuadra queda huérfana (mismo criterio
                        que un ejército sin hogar, Doc 5.4).
```

**Sin `heridoHasta`** — se elimina de `Escuadron` (gana el modelo de BronzeAge). Una baja de escuadra
reduce `cantidad` de forma permanente, nunca hay estado intermedio de "herida". El adaptador de Conquest
debe dejar de interpretar `SquadInstanceData.unitsInjured` como resultado persistente compartido.

**Decisión (corrige la versión anterior, que lo dejaba pendiente): se permiten varias `Escuadron` del mismo
`tropaId` para el mismo héroe, sin fusión.** El invariante antiguo "un `Escuadron` por `tropaId` por
héroe/asentamiento" se ELIMINA — no se sostenía: una escuadra que sale (a un `Ejercito` o a
`reservaBatalla`) y otra reclutada después del mismo `tropaId` en el asentamiento de origen pueden volver a
coexistir, y forzar una fusión al reunirse rompería IDs y progresión individual (nivel/XP/moral de cada
una) — exactamente lo que Codex advirtió que había que evitar. La UI simplemente muestra varias entradas
del mismo `tropaId` cuando ocurre; no es un caso de error.

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
  expiraEn: Instante        fijado al crear/reasignar — timeout de infraestructura, tiempo de MUNDO
                          (ver nota de reloj en §0/§12: no confundir con expiración de credenciales)
  bandos: BatallaBando[]     capacidad independiente por bando (asimétrico permitido)
  participantes: BatallaParticipante[]   por heroeId, nunca jugadorId directo
  reservas: BatallaReserva    escuadras/suministro inmovilizados, liberables
  appliedResultId?          idempotencia — ver §16
  huellaPayloadAplicado?    hash del BattleResult ya aplicado
  instanteAplicado?, versionAplicada?
```

**Ciclo corregido (R01):** `convocando -> asignada -> en_curso -> aplicada`, con salidas a `cancelada`
(desde `convocando`/`asignada`, antes de `en_curso`) y `fallida` (timeout o fallo de infraestructura antes
de `en_curso`).

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
  vigencia (emitidoEn, expiraEn)
  contextoEstrategico        qué se está disputando (asentamiento/campo abierto/caravana)
  bandos: [{ ladoId, capacidadMinima, capacidadMaxima, participantes: BattleParticipantSnapshot[] }]
  mapa: BattleMapReference | SettlementBattleSnapshot   ver §17 para la forma de SettlementBattleSnapshot
  reglas: BattleRules         ver abajo
  autorizacion                limitada a esta battleId + ticketRevision, ver doc 02 §3
```

**Corrección R03 — revisión del ticket:** BA-001 permite sustituir un participante antes de empezar
(desconexión, etc.); eso sube `ticketRevision` y cambia `huellaTicket`. Una `BattleServerAssignment` o
`BattleResult` que referencie una `ticketRevision` distinta de la vigente en `Batalla` se **rechaza** — así
una asignación vieja no puede cerrar una revisión nueva.

**`BattleParticipantSnapshot`:** `heroeId`, `ladoId`, `HeroSnapshot`, `SquadSnapshot[]`.

**`HeroSnapshot` — forma mínima (R04, antes indefinida):** `heroeId`, `displayName`, `classDefinitionId`,
`nivel`, `atributosEfectivos` (post-equipo, recalculados por BronzeAge contra el catálogo vigente — nunca
copiados de un cálculo del cliente), `perksDesbloqueados[]`, `equipamiento` (igual forma que
`Heroe.equipamiento`, §12), `versionCatalogoHeroe`.

**`SquadSnapshot` — forma mínima (R04, antes solo tenía `squadId`+cantidad+`tropaId`):** `squadId`,
`tropaId`, `efectivosAutorizados` (renombrado desde "desplegados" — es la fuerza RESERVADA/autorizada, no
necesariamente la que Unity ponga en juego de una vez, ver `BattleResult` abajo), `nivel`, `experiencia`,
`moral`, `habilidadesDesbloqueadas[]`, `formacionesDesbloqueadas[]`, `formacionSeleccionada`,
`versionCatalogoTropas`. Sin esto, dos escuadras del mismo `tropaId` con progresión distinta serían
indistinguibles para inicializar sus capacidades de combate — el catálogo del tipo de tropa por sí solo no
basta (Conquest ya conserva esta progresión en `SquadInstance.Data.cs`, no hay que reinventarla, solo
transportarla).

**`BattleRules` — forma mínima (R04, antes indefinida):** `schemaVersionBalance`, duración máxima de
partida, condiciones de victoria permitidas por este contexto, `versionCatalogoTropas`,
`versionCatalogoHeroe`. Las capacidades asimétricas por bando ya viven en `bandos[].capacidadMinima/Maxima`
(BA-001), no se duplican aquí.

**Escoltas sin dueño presente (R04):** una escuadra de `Caravana.escolta` puede entrar en las reservas de
un bando como propiedad administrativa de su `heroeId` sin que ese héroe sea un `BattleParticipant` físico
(no está presente en la partida real) — el `SquadSnapshot` lleva su `heroeId` de referencia igual, pero no
exige una entrada en `participantes` para él. Distinto de que el héroe combata en persona.

**Batallas sin humanos (NPC vs NPC) — fuera de alcance v1 (R04):** el ciclo persistente de `Batalla` es
para encuentros con al menos un `BattleParticipant` humano por bando. El combate íbamos NPC-contra-NPC del
mundo abierto sigue resolviéndose con el resolver numérico actual (`engine/combate.ts`), sin tocar, hasta
que se acuerde una representación táctica para ese caso — no se inventa un héroe humano de relleno para
encajarlo en este contrato.

### `BattleResult` (DTO, producido por el servidor de batalla Unity) — completitud corregida

```text
BattleResult
  schemaVersion, battleId, resultId, ticketRevision
  inicio, fin, ganador, razon
  objetivos: ObjectiveResult[]
  porEscuadra: [{ squadId, desplegados, supervivientesAlCierre, muertos }]
  porHeroe: [{ heroeId, participo, sobrevivioAlCierre, herido?: boolean }]
  versionServidor, autenticidad
```

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
reporta aquí; BronzeAge solo necesita saber si terminó vivo. `herido` se deriva de las reglas del ticket
(`BattleRules`) aplicadas sobre los hechos — la fórmula exacta queda **pendiente de decidir junto con XP**
(siguiente punto), no inventada aquí.

**Hechos que alimentan XP/nivel en v1 — declarado explícito (R05):** `participo`, `ganador` (¿su bando
ganó?) y duración de la partida. Capturas u otras bajas atribuidas individualmente NO entran en v1 —
si se necesitan más adelante, es una versión de contrato nueva, no una ampliación silenciosa de esta.

**Daño de asedio — fuera de alcance v1, declarado explícito (R05):** este `BattleResult` NO lleva daño de
edificios/murallas. Un asedio que destruye algo persistente es una decisión de dominio que todavía no se ha
tomado (ver §17); hasta que se tome, un asedio dentro de este contrato no persiste destrucción física.

El resultado contiene hechos tácticos, nunca deltas de progresión ya calculados. BronzeAge sigue calculando
nivel/XP de escuadra, herida de héroe, conquista, ocupación y liberación de reservas a partir de estos
hechos, contra los catálogos vigentes.

### `BattleServerAssignment`

```text
BattleServerAssignment
  battleId, ticketRevision      debe coincidir con la revisión vigente — corrección R03
  intentoAsignacionId            identifica esta asignación concreta; solo el productor que la recibió
                                puede luego confirmar inicio/resultado con este mismo id — corrección R03
  instancia (host, puerto, protocolo)
  credencialServidorAServidor    registra la asignación, confirma inicio y firma el BattleResult — NUNCA
                                visible a un cliente
  tokensParticipante: [{ heroeId, token, expiraEn: string (ISO 8601, tiempo REAL, no Instante — ver §0) }]
                                alcance limitado a esta battleId+ticketRevision
```

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
  settlementId, ticketRevision
  sistemaCoordenadas          origen, ejes, escala — el factor de conversión de arriba, explícito aquí
  edificios: [{ edificioId, tipo, posicion, footprint, nivelInterno, estado, visualSeed }]
                              geometría EFECTIVA en el instante de abrir — copia congelada, no una
                              referencia viva al Asentamiento (que puede seguir cambiando mientras la
                              batalla está en curso)
  recintos: [{ recintoId, nivel, celdas, nivelEfectivo }]
                              `nivelEfectivo` resuelve la ambigüedad de §3 (avance=-1 durante una mejora
                              de un recinto ya completo): congela el nivel FÍSICO real de cada recinto en
                              ese instante, sin que el lector tenga que reinterpretar `avance`/`mejorandoA`
  puertasYObstaculos            derivado de `celdas` (clase 'puerta'), más cualquier obstáculo táctico
                                añadido para el escenario (spawns, límites navegables — no altera edificio/
                                muralla de origen)
  layoutVersion, visualCatalogVersion   versiones usadas al congelar, para reproducibilidad TS/C#
```

**Política explícita (R10):** cambios estratégicos en el `Asentamiento` real mientras la `Batalla` está en
curso (nueva construcción, mejora de muralla) NO tocan este snapshot ya congelado — se aplican al
`Asentamiento` real después, cuando la batalla se resuelve (`aplicada`/`cancelada`/`fallida`). Dos
reconstrucciones de la misma ciudad (vista normal vigente vs. escenario de asedio congelado) pueden diferir
mientras la batalla dura; eso es esperado, no un bug de sincronización.

**Daño de asedio:** sigue sin decidirse si un asedio puede destruir algo persistente — §15 ya declara que
`BattleResult` v1 no lleva esa información. Este snapshot no necesita un campo para ello todavía.

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
pendiente de resolver junto con `src/contratos/v1/`.

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
