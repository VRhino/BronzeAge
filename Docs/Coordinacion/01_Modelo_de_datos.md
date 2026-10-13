# Modelo de datos completo

> **Estado (2026-10-13): reescrito en la fase 2.** El modelo es [`proto/bronzeage/v1/entities.proto`](../../proto/bronzeage/v1/entities.proto) y
> [`battle.proto`](../../proto/bronzeage/v1/battle.proto): **todas las entidades del dominio**, no solo las de la feature del momento. Este
> documento es el mapa: qué entidad es cada mensaje, de qué documento del canon sale y qué invariantes la gobiernan. El modelo anterior
> (`typescript-final:Docs/Coordinacion/01_Modelo_de_datos_compartido.md`, JSON del servidor TypeScript) está en `typescript-final:Docs/Coordinacion/`.
> Sale del canon (`Docs/Game`) y de `Docs/Mechanics`, **no** del código viejo.

## 1. Reglas del modelo

1. **Entidad con identidad** = mensaje con `id` y `version` (la versión de mundo del último cambio). Es lo que el almacén del cliente guarda por id.
2. **Movimiento = `Plan`** (vértices con su instante de paso). **Cantidades que crecen o se gastan = `Ritmo`** (valor en t₀ + ritmo por minuto). Ninguna entidad guarda «progreso por tick».
3. **Tiempo** en ms de mundo (`*_ms`); **espacio** en milésimas de unidad estratégica; **cantidades** de juego en milésimas.
4. **Presencia de campos**: lo opcional es `optional`; borrar es explícito (`campos_borrados`).
5. **Catálogos** (recursos, edificios, tropas, tecnologías, políticas): ids de cadena del archivo común de datos de juego, no enumeraciones.
6. **Lo derivado** (zona de un asentamiento, cupo de guarnición, liderazgo total de un loadout) viaja marcado como derivado y no se escribe.
7. **Nada de la antigua cadena por tick**: no existen `ruta`/`progreso`, `origenAsentamientoId`, el estado `regresando`, `extractoresTicksSinCupo` ni las caravanas `militar` y `contrabando`.

## 2. Entidades por dominio

| Dominio | Mensajes (entities.proto) | Canon / Mechanics |
|---|---|---|
| **Facción y política** | `Faccion`, `RelacionPolitica`, `Propuesta` (relación, anexión, fusión), `Titulo`, `Sigilo` | Doc 2; `Diplomacia_Facciones` |
| **Asentamiento** | `Asentamiento` (población y mantenimiento como `Ritmo`), `Edificio`, `Recinto`, `CeldaMuro`, `Cargos`, `PoliticaActiva`, `AscensoEnCurso`, `Existencia` | Doc 1.2-1.3, 4; `Economia_Construccion`, `Murallas`, `Trazado_Urbano` |
| **Guerra logística** | `Pueblo`, `Avanzada`, `ObraDeMapa` | Doc 4.2.2, 5.17; `Economia_Construccion` §14 |
| **Héroe y tropa** | `Heroe`, `Escuadron`, `Loadout`, `Equipamiento`, `ItemInstancia`, `InteriorRecordado`, `ContenedorEscuadron` | Doc 5.11, 5.15-5.16; `Heroe_Y_Tropa`, `Presencia_Y_Convocatoria` |
| **Columnas y caravanas** | `Ejercito` (con `Plan`, víveres propios, sin origen), `Caravana` (plan, estado, carros), `Convocatoria` | Doc 5.12-5.14, 3.13; `Movimiento_Ejercitos`, `Comercio_Caravanas` |
| **Campamentos y mundo vivo** | `CampamentoMercenarios`, `CampamentoBandidos`, `Alijo`, `NodoRecurso`, `ZonaBosque`, `ZonaFaccion` | Doc 1.4, 1.9-1.9c; `Campamentos_Mercenarios`, `Generacion_Del_Mundo` |
| **Comercio** | `AcuerdoTrueque`, `LineaTrueque`, `OrdenMercado`, `RutaComercial`, `RedCaminos` | Doc 3; `Comercio_Caravanas` |
| **Tecnología, Aedas, intel** | `TecnologiasFaccion`, `EstadoTecnologia`, `AedaResidente`, `AedaItinerante`, `EpicaEnCurso`, `MiradaIntel`, `InformePlaza`, `MemoriaFaccion`, `FichaConocida`, `NieblaProyectada` | Doc 6, 5.12.7-5.12.10; `Tecnologia_Aedas`, `Niebla_De_Guerra` |
| **Batalla de Unity** | `Batalla`, `BattleTicket`, `IncorporacionBatalla`, `BattleServerAssignment`, `TokensBatalla`, `InicioBatalla`, `SalidaBatalla`, `BattleResult`, `Botin`, y los contextos (asedio, campo abierto, caravana disputa/aguante, campamento de bandidos, saqueo de pueblo, avanzada) | Doc 5.15; `Batalla_Unity_Ciclo` |
| **Mundo** | `ArtefactoDeMundo` y sus capas (`world.proto`) | [`06_Contrato_de_Mundo.md`](06_Contrato_de_Mundo.md) |

## 3. Invariantes del modelo

- Un héroe tiene **una** ubicación: plaza, columna, campamento o fuera del mundo (solo si salió con su columna).
- Una escuadra tiene **un** contenedor. **La tropa que va con su héroe se queda con él; la prestada** (guarnición, escolta) **vuelve sin viaje al campamento de su héroe** cuando no puede seguir donde está (Doc 5.15.2b).
- Un ejército es un conjunto de héroes con sus columnas y caravanas adjuntas: **no guarda tropa por su cuenta** y **no tiene destino fijo ni vuelve solo**.
- Una caravana adjunta está en `caravanas_adjuntas_ids` de **exactamente** un ejército y su posición es la de él; una caravana aparcada en una plaza que cae **se destruye**.
- El almacén nunca baja de 0 ni supera su capacidad; la población es continua y se lee entera; el medidor de mantenimiento ∈ [0, 100]; la reputación ∈ [−100, 100]; `nivel` es monótono.
- Un pueblo extractor o una avanzada **cuenta como edificio activo de su plaza madre**; los pueblos pasan al conquistador con la madre y desaparecen con su ruina.
- Los `Plan` son monótonos en el tiempo (`en_ms` no decrece) y su primera posición es la de la entidad en el instante en que se emite.
- Un identificador nunca se reutiliza dentro de una partida.

## 4. Cierre de las propuestas BA

Las propuestas de `propuestas/` se absorben en este contrato; su cierre y la correspondencia con el `.proto` están en
[`propuestas/CIERRE_FASE_2.md`](propuestas/CIERRE_FASE_2.md).
