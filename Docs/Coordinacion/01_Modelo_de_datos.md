# Modelo de datos completo

> **Estado (2026-10-13): reescrito en la fase 2.** El modelo es [`proto/bronzeage/v1/entities.proto`](../../proto/bronzeage/v1/entities.proto) y
> [`battle.proto`](../../proto/bronzeage/v1/battle.proto): **todas las entidades del dominio**, no solo las de la feature del momento. Este
> documento es el mapa: qué entidad es cada mensaje, de qué documento del canon sale y qué invariantes la gobiernan. El modelo anterior
> (`typescript-final:Docs/Coordinacion/01_Modelo_de_datos_compartido.md`, JSON del servidor TypeScript) está en `typescript-final:Docs/Coordinacion/`.
> Sale del canon (`Docs/Game`) y de `Docs/Mechanics`, **no** del código viejo.
>
> **Qué proyecta hoy el servidor Rust (4H, cerrada el 2026-10-24).** Todas las entidades del dominio que el núcleo tiene salen por la sesión (contrato 05 §3), con sus ids reales. Las que aún no
> tienen dominio en el núcleo (`Propuesta`, `Titulo`, `Sigilo` de la Facción, `EstadoTecnologia`, `AedaResidente`, `AedaItinerante`, `EpicaEnCurso`, `Alijo`, el inventario y el equipo del héroe)
> son de los sistemas 5 y 6 o de Conquest y están en las filas de paridad «Contrato (4H)» de `Docs/Mecanicas a desarrollar.md`. Desde T7: `Heroe.plazas_recordadas` (`InteriorRecordado`, la foto
> del interior de cada plaza de la que el héroe salió al mundo; estado en la foto `BAN25`), `Ejercito.territorio_faccion_id` (solo en `VISTA`), `ZonaFaccion` fundida, `RedCaminos.escalones` y
> `BattleServerAssignment.servidor_id` (solo en `ADMIN`).

> **Campos de la tanda 69 (2026-11-08, contrato 0.19.0).** `Heroe.monedas` (un solo saldo de monedas, aparte del oro de las plazas; sustituye a `Monedas { bronce, plata, oro }`, que desaparece, y `Botin.monedas` es ahora un número); `Faccion.prestigio` y `Faccion.tope_alianzas` (públicos); `Edificio.buey` (el molino con su buey); `Pueblo.cultivo` (`trigo`, `vid` u `oliva`, solo en la aldea) y el almacén local de la aldea con más recursos; `TarifaMercenario.monedas` (la tarifa del tablón en monedas); nueve recursos nuevos de la comida (38 en total) y seis tipos de edificio nuevos (molino, horno, quesera, bodega, prensa y estatua). Las capas nuevas del mundo (`region`, `fertilidad_vid`, `fertilidad_oliva`) están en [`06_Contrato_de_Mundo.md`](06_Contrato_de_Mundo.md). Las fotos anteriores a `BAN59` ya no se leen.

> **Niveles del pueblo extractor (2026-11-10, contrato 0.22.0).** `Pueblo.nivel_aldea` (13) lleva el nivel también en un extractor (Cantera o mina, 1 a 3; antes ausente) y `mejorar_aldea` (158) sube también un extractor, hasta el nivel de su plaza madre; su mejora en marcha llega como la de la aldea (`ObraDeMapa` de tipo `OBRA_MEJORA_ALDEA`). Hechos nuevos `pueblo.mejora_marcada` y `pueblo.subio`. Sin formato de foto nuevo; las reglas cambian (partida nueva). Ver [`Economia_Construccion.md`](../Mechanics/Economia_Construccion.md) §14.

> **Campos de la cebada y el sésamo (2026-11-09, contrato 0.20.0).** `Edificio.receta` y `Edificio.receta_parada` (la receta que trabaja una bodega o una prensa, por su insumo, y si está parada porque la plaza bajó de nivel); `Pueblo.cultivo` admite además `cebada` y `sesamo`; tres recursos nuevos (cebada, cerveza y sésamo: 41 en total); la orden `elegir_receta` (187). Las capas nuevas del mundo (`fertilidad_cebada`, `fertilidad_sesamo`) están en [`06_Contrato_de_Mundo.md`](06_Contrato_de_Mundo.md). Las fotos anteriores a `BAN60` ya no se leen.

## 1. Reglas del modelo

1. **Entidad con identidad** = mensaje con `id` y `version` (la versión de mundo del último cambio). Es lo que el almacén del cliente guarda por id.
2. **Movimiento = `Plan`** (vértices con su instante de paso). **Cantidades que crecen o se gastan = `Ritmo`** (valor en t₀ + ritmo por minuto). Ninguna entidad guarda «progreso por tick». La excepción es la economía de una plaza (almacén y población), que cambia en escalones en el paso de minuto: su `Ritmo` lleva `en_escalones = true`, el cliente no interpola y `por_minuto` es la tendencia «+N/min».
3. **Tiempo** en ms de mundo (`*_ms`); **espacio** en milésimas de unidad estratégica; **cantidades** de juego en milésimas.
4. **Presencia de campos**: lo opcional es `optional`. Un cambio de entidad viaja **completo** (sustituye a la anterior) salvo el `Asentamiento`, que puede ir parcial con máscara de campos (`Cambio.reemplaza` y `Cambio.campos`, contrato 05 §5).
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
| **Tecnología, Aedas, intel** | `TecnologiasFaccion`, `EstadoTecnologia`, `AedaResidente`, `AedaItinerante`, `EpicaEnCurso`, `MiradaIntel`, `InformePlaza`, `InformeRegion`, `MemoriaFaccion`, `FichaConocida`, `NieblaProyectada` | Doc 6, 5.12.7-5.12.10; `Tecnologia_Aedas`, `Niebla_De_Guerra` |
| **Batalla de Unity** | `Batalla`, `BattleTicket`, `IncorporacionBatalla`, `BattleServerAssignment`, `TokensBatalla`, `InicioBatalla`, `SalidaBatalla`, `BattleResult`, y los contextos (asedio, campo abierto, caravana disputa/aguante, campamento de bandidos, saqueo de pueblo, avanzada) | Doc 5.15; `Batalla_Unity_Ciclo` |
| **Mundo** | `ArtefactoDeMundo` y sus capas (`world.proto`) | [`06_Contrato_de_Mundo.md`](06_Contrato_de_Mundo.md) |

## 3. Invariantes del modelo

- Un héroe tiene **una** ubicación: plaza, columna, campamento o fuera del mundo (solo si salió con su columna).
- Una escuadra tiene **un** contenedor. **La tropa que va con su héroe se queda con él; la prestada** (guarnición, escolta) **vuelve sin viaje al campamento de su héroe** cuando no puede seguir donde está (Doc 5.15.2b).
- Un ejército es un conjunto de héroes con sus columnas y caravanas adjuntas: **no guarda tropa por su cuenta** y **no tiene destino fijo ni vuelve solo**.
- Una caravana adjunta está en `caravanas_adjuntas_ids` de **exactamente** un ejército y su posición es la de él; una caravana aparcada en una plaza que cae **se destruye**.
- El almacén nunca baja de 0 ni supera su capacidad y cambia en escalones (paso de minuto); la población es **entera** (habitantes enteros, con un resto de crecimiento oculto que no viaja); el medidor de mantenimiento ∈ [0, 100]; la reputación ∈ [−100, 100]; `nivel` es monótono.
- Un pueblo extractor o una avanzada **cuenta como edificio activo de su plaza madre**; los pueblos pasan al conquistador con la madre y desaparecen con su ruina.
- Los `Plan` son monótonos en el tiempo (`en_ms` no decrece) y su primera posición es la de la entidad en el instante en que se emite.
- El prestigio de una Facción **solo sube** (no se gasta ni se pierde) y `Heroe.monedas` nunca es negativo; un molino pierde su buey con el molino (ruinas, obra cancelada o dañado en una conquista).
- Un identificador nunca se reutiliza dentro de una partida. Todos son `<prefijo>-<n>` (la Facción, `f-3`; ver `common.proto`).
- Las zonas de una misma Facción se solapan (no se recortan entre sí) y las de Facciones distintas no: `ZonaFaccion` es su unión y cada punto del mapa es como mucho de una Facción (`Ejercito.territorio_faccion_id`).
- Un héroe puede estar además **dentro de un pueblo extractor o de una avanzada** (`UbicacionHeroe.pueblo_id`/`avanzada_id`); la columna que entró en un lugar queda fuera del mapa (`Ejercito.dentro_de`).

## 4. Cierre de las propuestas BA

Las propuestas BA-001 a BA-006 se absorbieron en este contrato. Su cierre y la correspondencia con el `.proto` están en el
historial de git (`Docs/Coordinacion/propuestas/CIERRE_FASE_2.md`, retirado el 2026-11-13 con el modelo de cooperación).
