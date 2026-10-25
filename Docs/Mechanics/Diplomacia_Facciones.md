# Diplomacia y Facciones

## 1. Alcance y fuentes

Cubre la **Facción como entidad** (crearla, entrar, salir, cargos, nivel, reputación, sigilo, capital), la **residencia** del héroe, la **puerta** de una plaza (quién entra), las **relaciones
entre Facciones** (alianza, vasallaje y su tributo, guerra y paz, rebelión, la Liga y el Gran Rey), la **anexión** y la **fusión** (y el traslado de todo lo que colgaba de una Facción que
desaparece), la **derrota** y los **títulos** de prestigio.

- **Reescrito el 2026-10-13** para el mundo reactivo: lo que corría «cada tick» (reputación, tributo, títulos, liberación de vasallos) pasa a ritmos y a reevaluaciones por dependencia ([`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md)), y la guerra incorpora la regla nueva de que **toda agresión entre Facciones exige guerra o la declara** (§5.4). El «cómo funcionaba» está en `typescript-final:Docs/Mechanics/Diplomacia_Facciones.md`.
- **Verificado contra:** `71d5758` (2026-10-11).
- **Implementado en Rust (2026-10-24, 5A): las reglas puras** en `crates/politica` (sin núcleo todavía: las órdenes siguen dando `orden.no_disponible` hasta 5B-5F). Módulos: `faccion` (§3.1-§3.4), `nivel` (§3.5), `cargos` y `residencia` (§3.4, §4.1-§4.2), `capital` (§3.7), `sigilo` (§3.8), `puerta` (§4.3), `relaciones` (§5.2-§5.9), `propuestas` (§2.4, §5.3, §6.1-§6.2), `tributo` (§5.6), `liga` (§5.7), `titulos` (§7), `traslado` (§6.3) y `autorizacion` (los predicados de `Proyecciones_Y_Autorizacion.md` §4.3 y §5.2). Aplican las decisiones del autor del 2026-10-24 (Q1-Q8, `Docs/Arquitectura/Sistema5_Contraste_y_Plan.md`) y la economía entera (C3b): el tributo paga unidades enteras con resto oculto. La reputación sigue en `comercio::reputacion`: `politica` devuelve sus `Suceso`s.
- **Correcciones de 5B (decisiones del autor del 2026-10-24, en Rust):** **5B-1** quien deja su residencia (por cualquiera de las tres órdenes) o su Facción estando dentro de la plaza que deja **sigue dentro como visitante** si la puerta le admite, con su carro y sus víveres; sus escuadras (su campamento) y su almacén personal pasan **sin viaje** a la residencia nueva, porque viven con ella. Si dejó la Facción y la puerta ya no le admite (de la plaza, o del pueblo o la avanzada donde esté), sale expulsado al mapa **solo** —sin tropa— con su carro (`heroe.expulsado`, motivo `puerta_cerrada`). **5B-4** al dejar la residencia y al dejar la Facción, la residencia nueva es el campamento de mercenarios más cercano **a la plaza que deja** (no a donde está). **5B-6** pedir el ingreso, o que lo acepten, yendo en un ejército (aunque sea solo, o en uno en preparación) se rechaza como dejarla (`faccion.en_ejercito`); denegarlo, no. **5B-10** nadie se muda **desde** una plaza en batalla (las tres órdenes de residencia y dejar la Facción dan `batalla.bloqueado`), igual que nadie se muda a una. Código en `crates/nucleo/src/nucleo/ciudadania.rs` y `dentro.rs` (`echar_si_ya_no_entra`); pruebas en `crates/nucleo/tests/politica.rs` y `batallas.rs` (`nadie_se_muda_de_una_plaza_en_batalla_aunque_ya_este_fuera`).
- **Implementado en Rust (2026-10-24, 5C): la experiencia y el nivel, la capital, la derrota y el nombre de las plazas** (`crates/nucleo/src/nucleo/experiencia.rs`, foto `BAN29`). **§3.5:** la experiencia se da en el instante de cada hecho (`faccion.experiencia` con `motivo` —1 trueque, 2 combate, 3 batalla de Unity, 4 edificio, 5 conquista, 6 ascenso, 7 fundación, 8 bandidos—, `cantidad` y el total; `faccion.nivel_subio` si cruza un umbral): combate con números **digno** en columna contra columna y asedio (los del TypeScript), a la Facción de cada héroe que combatió; batalla de Unity, por héroe que participó y sin filtro; conquista; campamento de bandidos destruido, por campamento; edificio completado; ascenso (30 × el nivel); fundación salvo la primera; trueque entre Facciones distintas. El nivel ya no se guarda: `Nucleo::nivel_de_faccion` lo deriva y lo leen el cap de fundación y el cupo de nivel (`politica::nivel::{cap_fundacion, cupo_nivel}`, un solo bloque cada uno, en este documento), los huecos de políticas y la Gran Fundición; `FijarNivelFaccion` (administración) fija la experiencia al umbral y crea la Facción vacía si no existe. **§3.7:** `designar_capital` (137, el Rey) con sus rechazos `capital.*`; la marca vive en la plaza (`Asentamiento::capital_de`) y sobrevive a la conquista sin valer; la capital efectiva se calcula en el paso de minuto de cada plaza entre las de su Facción (índice `plazas_por_faccion`, derivado) y da `es_capital`, la distancia a la capital (que sube el mantenimiento: `Economia_Construccion.md` §10.1) y la Gran Fundición única. **§8:** `derrotadaPor` al caer la última plaza (conquista, por números o por Unity: la Facción conquistadora; ruina: nadie) y se borra al fundar. El nombre de las plazas, en `Fundacion_Y_Refundacion.md` §8.
- **Implementado en Rust (2026-10-24, 5D): las relaciones entre Facciones** (`crates/nucleo/src/nucleo/diplomacia.rs`, foto `BAN32`). Las relaciones dejan el mapa de pares del andamiaje y viven en `Estado::politica` (§2.3: dirección, estado, fechas, tributo, quién la rompió y quién ofreció la paz; las rotas se conservan), con ids del contador común (`rel-N`; las propuestas, `prop-N`). Órdenes 144-152, mapeadas al contrato: `proponer_relacion`, `responder_relacion`, `retirar_relacion` (§5.3, con caducidad al mirarla y sin eventos en la cola), `declarar_guerra` (§5.4, con el arrastre por vasallaje), `proponer_paz`, `romper_relacion`, `rebelion_vasallo` (§5.5, la rebelión expira sin penalizar los trueques activos entre plazas de las dos), `admitir_otras_facciones` (lo lee el reparto de las batallas: `rey_admite_otras`) y `alternar_reabastecer_aliados` (`Asentamiento::permite_reabastecer_aliados`: repostar y `cargarCaravana` en una plaza aliada o vasalla que lo permite, Movimiento_Ejercitos §8). `atacar` con `declaraGuerra` declara con la misma regla: con arrastre, la traición contra una aliada (Q2) y el rechazo `combate.vasallaje` contra la propia señora o vasalla; sin `declaraGuerra`, `guerra.confirmar` lleva el arrastre en su detalle. El **tributo** se paga en el paso de minuto de cada plaza vasalla, tras comer su guarnición (Q3, 5A-2; resto oculto en `Asentamiento::resto_tributo`; un `diplomacia.tributo_pagado` por plaza que paga). El **desarme del señor** y la **vasalla conquistada** ocurren en el instante en que una Facción pierde su última plaza (conquista o ruina), y también al aceptar una vasalla sin plazas otra señora (§12). La **reputación**: los `Suceso`s de `politica` (romper una alianza, liberar a la vasalla, la rebelión, la traición, la guerra tras romper, defender o no al vasallo con números —columna, caravana, asedio, pueblo, avanzada— y en Unity) en el instante del hecho, y las alianzas activas de cada Facción en el ritmo de su reputación al cambiar. `FijarRelacion` (administración, decisión 9) rompe la activa del par y crea la pedida, con `a` señora o declarante, sin propuesta, arrastre ni reputación. Proyección: `RelacionPolitica` entera (activa) en `FACCION` de las dos, en `VISTA` de todos (son públicas) y en `ADMIN`; al romperse sale (`QUITA`). Hechos: los doce `diplomacia.*` de las relaciones, `faccion.admision_en_ataques` y `logistica.reabastecer_aliados`; `diplomacia.guerra_declarada` sustituye a `faccion.guerra_declarada` (con `arrastradas` y `traicion`) y `faccion.relacion` se retira. Pruebas: `crates/nucleo/tests/diplomacia.rs`. Las propuestas no se proyectan todavía (5H: viajan en `creadas` y en sus hechos).
- **Implementado en Rust (2026-10-25, 5F): la anexión, la fusión, el traslado, la Liga, el Gran Rey y los títulos** (`crates/nucleo/src/nucleo/traslado.rs` y `titulos.rs`, foto `BAN33`; `BAN34` al integrar 5E). Órdenes 168-173 (5E tomó del 158 al 160), mapeadas al contrato: `proponer_anexion`, `responder_anexion`, `retirar_anexion` (§6.1: la autoridad diplomática de la absorbente propone y retira, el Rey de la absorbida contesta) y `proponer_fusion`, `responder_fusion`, `retirar_fusion` (§6.2: el Rey de A propone y retira, el de B contesta), con la caducidad al mirarlas, la revalidación al aceptar y `diplomacia.batalla_abierta` si alguna de las dos ataca o defiende una batalla activa **o algún héroe de las dos combate en una, como bando o incorporado** (5F-3, decisión del autor del 2026-10-25). **El traslado** (§6.3) aplica en la misma transición el plan de `politica::traslado` y todo lo demás: las plazas con todo lo que cuelga de ellas (cargos locales, guarnición, residentes, flota, pueblos, avanzadas), sin la marca de capital del origen ni el resto de su tributo; en la anexión, las plazas son del destino **desde ese instante** (5C-1: la capital del destino no cambia) y en la fusión conservan su antigüedad (Doc 2.6: la capital de C sin designar es la plaza más antigua); después la ciudadanía (cada ciudadano del origen, en su orden, al final de los del destino; su copia, su columna si va solo y la demanda de bandidos de su campamento): primero las plazas, para que quien reside en una conserve sus cargos locales (5E: los pierde quien deja de ser de la Facción dueña de su plaza, y aquí la plaza se va con él); las columnas, las convocatorias, las Caravanas de Fundación y las rutas de la red; la memoria de la niebla (se funde al cerrar la versión, tras grabar lo que vieron los ojos del origen: lo explorado por unión, las fichas y los Informes con los del destino delante y sin los de sitios ya propios, las Miradas al destino); la reputación y las tecnologías del andamiaje del origen se pierden (en la fusión C nace con la mayor reputación y la unión de las tecnologías); una Facción derrotada que recibe plazas deja de estarlo (§2.1); quien queda dentro de un lugar que ya no le admite sale al mapa con lo suyo (`heroe.expulsado` con el motivo `cambia_de_dueno` de la conquista: 5F-9, decisión del autor del 2026-10-25). **Q5:** los trueques en curso siguen (van entre plazas) y los que quedan dentro de la misma Facción no mueven la reputación al cumplirse o vencer. **Los títulos** (§7) se recalculan en un plazo global de la cola cada `TITULOS.cadenciaMinutos` (bloque nuevo) que arma lo que pasa en el mundo, y se narran (`titulo.nace`, `titulo.cambia_manos`, públicos); viajan a la `VISTA` de todos y a `ADMIN` (`Clase::Titulo`). La Liga y el Gran Rey se derivan bajo demanda (`Nucleo::ligas`). La crónica canta la anexión, la fusión y los títulos. Pruebas: `crates/nucleo/tests/anexion_fusion.rs`.
- **Implementado en Rust (2026-10-24, 5B): la Facción como entidad, la ciudadanía y la residencia en el núcleo** (`crates/nucleo/src/nucleo/ciudadania.rs`, estado `Estado::politica`, foto `BAN28`). Órdenes 128-136: `CrearFaccion`, `SolicitarIngreso`, `ResponderSolicitud`, `DejarFaccion`, `AsignarRey`, `AsignarEmbajador` (§3.1-§3.4), `CambiarResidencia`, `ResidirEnCampamento` y `DejarResidencia` con su cooldown (§4.1-§4.2), mapeadas al contrato. La Facción guarda sus ciudadanos en orden de ingreso y `HeroeNucleo::faccion` es su copia; ganar o perder la ciudadanía da la Facción a la columna de quien va solo y funde lo explorado sin bandera (§4.3). Dejar la Facción yendo en un ejército se rechaza siempre (Q7, 5A-4; también en un ejército en preparación); quien deja su plaza por dejar la Facción o por `dejarResidencia` pasa al campamento de mercenarios más cercano **a la plaza que deja** (decisión 10 y 5B-4). Quien deja su casa estando dentro de ella sigue dentro de visita si la puerta le admite, y su tropa y su almacén personal pasan sin viaje a la casa nueva (5B-1; ver la nota de las correcciones de 5B, abajo). Al fundar, los ciudadanos de la Facción de la columna (hasta `FUNDACION.maxJugadoresFundacionGrupal`) pasan a residir en la plaza, sin estampar el cooldown (`fundacion.sin_fundadores` si no hay ninguno). La administración opera sobre las mismas reglas (decisión 9): `FijarFaccionDeHeroe` quita o da la ciudadanía con la sucesión (la Facción que nombra y no existe nace vacía, «Facción N», y quien entra es su Rey), `CrearEjercito` hace ciudadano a su héroe de campo, `FijarResidencia` es la reubicación forzosa. Los cargos locales, los vetos y la puerta por grupos son de 5E; el nivel por experiencia, la capital y la derrota, de 5C; las relaciones reales, de 5D (el estado ya guarda sus almacenes, vacíos).
- **Fuentes:** `engine/faccion.ts`, `engine/cargos.ts`, `engine/pertenencia.ts`, `engine/capital.ts`, `engine/sigilo.ts`, `engine/reputacion.ts`, `engine/diplomacia.ts`, `engine/liga.ts`, `engine/anexion.ts`,
  `engine/fusion.ts`, `engine/trasladoDeFaccion.ts`, `engine/titulos.ts`, `engine/mercenarios.ts` (residir en un campamento), `engine/ubicacion.ts` (cruzar la puerta), `session/comandos/{crearFaccion,
  ingresoEnFaccion,dejarFaccion,cargos,diplomacia,anexion,fusion,presencia}.ts`, `session/comandos/autorizacion.ts`.
- **Canon:** [`Docs/Game/2_Sistema_Politico_Facciones_y_Cargos.md`](../Game/2_Sistema_Politico_Facciones_y_Cargos.md) (y Doc 0 para el glosario, Doc 1.10.5 para la puerta).
- **No está aquí:** las consecuencias militares de una guerra o de una alianza (aliados no se atacan, repostar, refugio: → [`Combate.md`](Combate.md) y [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md)); la visión
  compartida de aliados y vasallos (→ [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md)); la experiencia que sube el nivel de Facción (los eventos que la dan están en cada sistema; la curva y las cifras, en
  el bloque `NIVEL_FACCION` de §3.5); el desalojo de residentes al conquistar una plaza (→ `Combate.md`); el Rey y la Aeda/épica (→ [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md)); el generador aleatorio (**nada de esto lo consume**).

## 2. Modelo de datos

### 2.1 `Faccion` (`estado.facciones`)

| Campo | Significado |
|---|---|
| `id`, `nombre` | `faccion-custom-{n}` al crearla un jugador; `faccion-fusion-{instante}-{A}-{B}` si nace de una fusión. El nombre no se repite (sin mirar mayúsculas) al **crearla**. |
| `sigilo` | Identificador heráldico (§3.8). Se elige al crear y **no cambia nunca**. |
| `reyId`, `embajadorId` | Cargos de Facción. `reyId = null` solo si no queda ningún ciudadano. |
| `capitalDesignadaEn?` | Cuándo designó el Rey su capital (cooldown, §3.7). |
| `experiencia`, `nivel` | Experiencia monótona y nivel derivado (§3.5). |
| `ciudadanosIds` | Ciudadanos, en **orden de ingreso** (el fundador primero): de ahí sale la sucesión del trono. |
| `solicitudesIds?` | Héroes que han pedido entrar. |
| `reputacion` | −100..+100. |
| `derrotadaPor?` | Presente solo mientras no tiene ningún asentamiento (§8). `null` = nadie (su última plaza colapsó). |
| `admiteOtrasEnAtaques?` | Ajuste permanente del Rey (→ `Combate.md`). |

### 2.2 Lo que cuelga de otras entidades

- `Asentamiento.heroesFundadoresIds` + `casasCompradas` — residentes de esa plaza (las dos vías son equivalentes: `esResidente`). `cargos` (`gobernadorId`, `tesoreroId`, `generalId`, `maestroObrasId`,
  `sacerdoteId`), `vetadosIds?`, `puertaCerradaA?`, `capitalDeFaccionId?` (la marca de la capital designada).
- `CampamentoMercenarios.residentesIds` — residentes de un campamento (de cualquier Facción).
- Estado: `relaciones`, `propuestasRelacion`, `propuestasAnexion`, `propuestasFusion`, `salidasFaccionPorHeroe[heroeId]` (cuándo dejó su última Facción), `cambiosResidenciaPorHeroe[heroeId]` (último
  cambio de residencia), `titulos`.
- **Se reside en un solo sitio:** una plaza **o** un campamento, nunca los dos.

### 2.3 `RelacionPolitica` (`estado.relaciones`)

`{id, tipo: 'vasallaje'|'alianza'|'guerra', faccionAId, faccionBId, tributo?, creadoEn, estado: 'activa'|'rota', pazPropuestaPor?}`. En un vasallaje **A es la señora y B la vasalla**; en una guerra, **A
es quien la declaró**; la alianza es simétrica. Una relación rota no se borra. Ids: `vasallaje-{A}-{B}-{n}`, `alianza-{A}-{B}-{n}`, `guerra-{atacante}-{objetivo}-{n}`.

### 2.4 Propuestas

Todas llevan `creadaEn`, `expiraEn` y `propuestaPor` (el héroe que la hizo):
- `PropuestaRelacion {id, tipo: 'alianza'|'vasallaje', faccionAId, faccionBId, tributo?}` — A ofrece, el Rey de B contesta.
- `PropuestaAnexion {id, absorbenteId, absorbidaId}` — la absorbente es quien propone.
- `PropuestaFusion {id, faccionAId, faccionBId, nuevoNombre, nuevoReyId}` — fija ya el nombre y el Rey de C.

Una propuesta **vigente** es la que cumple `ahora < expiraEn`. Entre dos Facciones hay **una sola pendiente de cada clase, en cualquier sentido**.

## 3. La Facción

### 3.1 Crear (`crearFaccion`)

Validaciones, en este orden: nombre no vacío (tras recortar) · nombre no usado por otra (comparación sin mayúsculas) · si trae sigilo, del catálogo y no idéntico a otro (§3.8) · el actor **no es ciudadano de
ninguna** (1 héroe, 1 Facción) · si salió de una hace menos de `CIUDADANIA.cooldownCreacionFaccionDias` días, no (`salidasFaccionPorHeroe`; solo lo escribe `dejarFaccion`). Resultado: la Facción nace con el
actor como **primer ciudadano y primer Rey** (`nivel 1`, `experiencia 0`, `reputacion 0`, sin Embajador), sin sus solicitudes en otras; lo que anduvo sin bandera (`exploracionPersonal`) pasa a su memoria, y
su columna (si va solo) toma la Facción. Sin sigilo dado se le asigna `sigiloLibre(faccionId, ocupados)`.

### 3.2 Entrar (`solicitarIngreso`, `responderSolicitud`)

- **Pedir:** el actor no es ciudadano de ninguna ni va en un ejército (aunque sea solo, o en uno en preparación: 5B-6, 2026-10-24); entra en `solicitudesIds` de esa Facción (pedir dos veces es pedir lo mismo). Puede pedir a **varias** a la vez.
- **Responder:** solo el **Rey** de esa Facción. **Aceptar** (el héroe no es ya ciudadano de otra ni va en un ejército: 5B-6; denegar sí se puede): lo añade al final de `ciudadanosIds` y lo quita de **todas** las listas de solicitantes (si la Facción se había quedado sin nadie, **pasa a ser su Rey**: 5A-8, 2026-10-24; vale para cualquier vía de dar la ciudadanía, también la de la administración); su exploración personal
  se funde en la memoria de la Facción y su columna la toma. **Denegar:** solo lo quita de esa lista. Aceptar no da residencia: para vivir en una plaza se muda (§4.2).
- **Las solicitudes no caducan**; al ganar la ciudadanía en una Facción (crearla o ser aceptado) se borran de todas las listas (`sinSolicitudesDe`; `sinSolicitudesCaducadas` era la red de seguridad del tick).

### 3.3 Salir (`dejarFaccion`)

Sin parámetros: solo la propia. `quitarCiudadania`: lo quita de `ciudadanosIds`; si era el **Rey**, el trono pasa al **primero de `ciudadanosIds`** (`null` si no queda nadie); el Embajador se vacía si era el que
se iba **o** si es el nuevo Rey. Además, si reside en una plaza, **deja la residencia** y sus cargos locales y la guarnición (§4.2); si reside en un campamento, sigue allí. Estampa `salidasFaccionPorHeroe`. Su columna
(si va solo) queda sin Facción. No cuenta para el cooldown de residencia. Quien deja la residencia al irse pasa al **campamento de mercenarios más cercano a la plaza que deja** (§4.2, Doc 0, 5B-4), con sus escuadras y su almacén personal, sin viaje; si estaba dentro sigue de visita, y si la puerta ya no le admite sin la Facción sale expulsado al mapa solo, con su carro (5B-1) (el TypeScript lo dejaba sin residencia: divergencia, decisión 10 del plan del sistema 5). **Q7 (decisión del autor, 2026-10-24):** dejar la Facción yendo en un ejército **se rechaza siempre**, también si el ejército es de un solo héroe (5A-4, 2026-10-24), y en una convocatoria con otros héroes: primero se separa o lo deshace entrando en una plaza o campamento, y si es el Líder, cede antes el mando (un ejército es siempre de una sola Facción).

### 3.4 Cargos

- **Rey** (`asignarRey`): el designado ha de ser ciudadano. La primera asignación (sin Rey) la puede hacer cualquier ciudadano; con Rey, solo él traspasa el cargo. No hay votación.
- **Embajador** (`asignarEmbajador`): lo designa el Rey, que debe existir; el designado es ciudadano (puede ser el propio Rey).
- **Cargos locales** (`asignarCargoLocal`): el designado es ciudadano de la Facción dueña. El **Gobernador** lo designa **el Rey** (no exige que esté presente). Los otros cuatro (`tesorero`, `general`,
  `maestroObras`, `sacerdote`) los designa el **Gobernador vigente**, residente y **presente** en la plaza, y solo si ya hay Gobernador. Un héroe puede ocupar varios cargos. **El designado ha de residir en la plaza** (decisión **Q4** del autor, 2026-10-24: un cargo que no puede ejercerse ni se vacía
  nunca sería un hueco; el TypeScript no lo miraba). El cargo se vacía al dejar de residir.
- **`admitirOtrasFacciones`:** el Rey fija `admiteOtrasEnAtaques`.
- Un cargo local se vacía al dejar el héroe su residencia en esa plaza (§4.2); un cargo de Facción, al irse el héroe de la Facción.

> **Implementado en Rust (2026-10-25, 5E):** `AsignarCargoLocal { heroe, asentamiento, cargo, designado }` (158, `asignar_cargo_local`; `nucleo/cargos.rs`) con `politica::cargos::asignar_cargo_local`:
> el Gobernador lo designa el Rey de la Facción dueña sin estar presente; los otros cuatro, el Gobernador residente y dentro, y solo con Gobernador (`cargo.sin_gobernador`); el designado es ciudadano de la
> Facción dueña (`cargo.no_ciudadano`) y reside en la plaza (`cargo.no_reside`, Q4); ya titular, `sin_cambios`; sustituye al anterior. Hecho `cargo.local_asignado` (`heroe`, `asentamiento`, `faccion` y el
> `cargo` como texto). Los cargos viven en la plaza (`Asentamiento::cargos`, foto **`BAN33`**) y se vacían al dejar de residir (toda mudanza pasa por `mudar_de_residencia`: las tres órdenes de residencia, dejar
> la Facción, la reubicación por ruina, el desalojo y `FijarResidencia`), al dejar de ser de la Facción dueña aunque no se mude (`FijarFaccionDeHeroe`) y en la conquista (todos). **Quien funda una
> plaza (el Líder de la columna) es su Gobernador de oficio** (5E-3, decisión del autor del 2026-10-25; `fundar` emite `cargo.local_asignado`), si es uno de los fundadores (el cargo pide residir: un
> Líder que no es ciudadano de la Facción de la caravana no reside y la plaza nace sin Gobernador); el Rey puede cambiarlo después con `asignar_cargo_local`. Viajan en el interior de la plaza (`ASENTAMIENTO`, `Asentamiento.cargos`) y en la ficha de las plazas propias (`FACCION`) y en `ADMIN`.

### 3.5 Nivel de Facción y cupos

```
calcularNivelFaccion = 1 + (cuántos umbrales de NIVEL_FACCION.xpParaNivel han alcanzado ya `experiencia`, en orden), con tope nivelMaximo
```

`experiencia` solo sube (`aplicarAjustesExperiencia` ignora lo ≤ 0). Quien da la experiencia (`aplicarExperiencia`) sube el nivel **en el mismo momento** (evento `faccion.nivel_subio`); el resto se recalcula al ocurrir el hecho
(construcción, ascensos). El nivel de una Facción que nace de una fusión se calcula al nacer. Qué hecho da experiencia lo cuenta cada sistema; la cuantía es el bloque `NIVEL_FACCION` de abajo (cifras de
`typescript-final:src/constants.ts`, `NIVEL_FACCION`, las del canon Doc 2.2.1): `combate` por héroe participante y solo si el combate es digno (`ratioCombateDigno`; en Unity sin filtro), **sea cual sea el objetivo** con números (5C-3: también saquear un pueblo, atacar una avanzada y asaltar una caravana; una batalla de Unity contra bandidos no da la de combate, solo la de bandidos, 5C-4),
`edificioCompletado` por edificio, `conquista` por plaza conquistada, `ascensoPorNivel` × el nivel alcanzado, `fundacion` por plaza fundada que no sea la primera de la Facción, `truequeCumplido` a cada
lado de un trueque entre Facciones distintas, y `bandidos` por campamento destruido (no por héroe) dividido por el nivel de la Facción y 0 desde `nivelSinXpBandidos`.
- **Cap de fundación** (`calcularCapFundacion(nivel)`) = `CAP_FUNDACION_POR_NIVEL[min(10, nivel) − 1]`.
- **Cupo de asentamientos en nivel 2 o 3** (`calcularCupoNivel(nivel, objetivo)`) = `CUPO_NIVEL_ASENTAMIENTO.maxNivel2|maxNivel3[min(10, nivel) − 1]`.

> **Implementado en Rust (2026-10-24, 5A, reglas puras):** `crates/politica::nivel`. La experiencia se guarda en **milésimas** (2,5 no cabe en un entero): `xp_de(Hecho)` da la de cada
> hecho (bandidos: `1000 / nivel`, redondeado hacia abajo), `nivel(experiencia)` cuenta los umbrales alcanzados con tope `nivelMaximo` y `Facciones::aplicar_experiencia` ignora lo ≤ 0 y devuelve el
> nivel nuevo si sube (el núcleo emite `faccion.nivel_subio`). `cap_fundacion` y `cupo_nivel` leen los bloques de este documento.

<!-- parametros:NIVEL_FACCION -->
```json
{
  "nivelMaximo": 10,
  "xpParaNivel": [100, 180, 300, 480, 760, 1200, 1900, 3000, 4800],
  "xp": {
    "combate": 2.5,
    "edificioCompletado": 1,
    "conquista": 10,
    "ascensoPorNivel": 30,
    "fundacion": 20,
    "truequeCumplido": 2,
    "bandidos": 1
  },
  "nivelSinXpBandidos": 5,
  "ratioCombateDigno": 0.5
}
```
<!-- /parametros -->

### 3.6 Reputación

Cada Facción lleva un número −100..+100 (`ajustarReputacion` lo recorta). La reputación es un **ritmo** (no se barre cada minuto):
```
si rep > 0 : rep = max(0, rep − decaimientoPorMinuto) ; si rep < 0 : rep = min(0, rep + decaimientoPorMinuto)
rep += (nº de alianzas activas en las que está) × bonusPorMinutoAlianzaActiva ; recortado a [−100, 100]
```
Qué la mueve (cifras en `REPUTACION`, bloque en `Comercio_Caravanas.md` §13): trueque cumplido (+) y vencido (−) entre Facciones distintas · romper una alianza (− a quien rompe) · liberar a un vasallo voluntariamente (+ a la
señora) · rebelión de un vasallo (− a la señora) · atacar a un aliado (−, en `Combate.md`) · **defender a un vasallo** (§5.9) · **declarar la guerra tras romper una alianza** (§5.4). Qué cambia según ella: proponer una alianza exige `reputacion ≥ umbralBajoParaEmbajador`; el recargo de comisión
(`Comercio_Caravanas.md` §3.3) y del reclutamiento en campamentos (`REPUTACION.factorComisionPorReputacionBaja`, con `reputacion ≤ umbralBajoParaComision`); la llegada de Aedas residentes (`Tecnologia_Aedas.md`).

### 3.7 Capital (`designarCapital`)

El Rey designa una plaza **propia con Palacio activo** que no sea ya la capital. Si había una capital **designada vigente** (es de la Facción y conserva un Palacio activo), hay que esperar `CAPITAL.cooldownDias` días
desde `capitalDesignadaEn`; si se perdió, no hay cooldown. Efecto: `capitalDesignadaEn = ahora`; solo esa plaza lleva `capitalDeFaccionId = faccionId` (se quita de las demás). **La capital efectiva**
(`encontrarCapital`): la designada vigente, o si no, el asentamiento vivo que lleva **más tiempo en la Facción** (5C-1, decisión del autor del 2026-10-24: desde que es suyo, no desde su fundación; a igual instante, el id menor). Conquistar una plaza más antigua que las propias **no cambia la capital**: es del conquistador desde la conquista.

> **Correcciones del autor a 5C (2026-10-24, en Rust):** cada plaza guarda `de_la_faccion_desde` (al fundarla, su fundación; al conquistarla, el instante de la conquista; foto `BAN32`), y es lo que mira `politica::capital::efectiva` (`PlazaDeCapital::desde`). La escala de la distancia a la capital (`escalaDistancia` = 80 u) se calibra en 5I (5C-2, `Docs/Mecanicas a balancear.md`). Prueba: `tests/caravanas.rs::la_conquista_pasa_la_flota_hace_volver_a_las_que_iban_y_destruye_la_aparcada` (la capital de quien conquista sigue siendo la suya).

### 3.8 Sigilo

Ocho campos: `formaId`, `campoId`, `emblemaId`, `orlaId`, `colorPrimarioId`, `colorSecundarioId`, `colorEmblemaId`, `colorOrlaId`, todos de `CATALOGO_SIGILO` (el catálogo de formas, campos, emblemas, orlas y
colores está en `constants.ts`, `version` incluida). **Vale** si es del catálogo y `colorPrimarioId ≠ colorSecundarioId`. **Duplicado** = igual a otro en todos los campos (el color de la orla solo cuenta si hay orla);
los parecidos se permiten. `sigiloLibre(semilla, ocupados)`: toma `hash32(semilla)` (FNV-1a de 32 bits sobre los códigos del texto) como índice en el espacio de combinaciones (formas × campos × emblemas × orlas ×
k·(k−1)·k·k colores) y prueba `+1` hasta dar con uno libre; desmezcla el número por restos sucesivos en el orden forma, campo, emblema, orla, color primario, secundario (saltando el primario), color del emblema,
color de la orla.

<!-- parametros:CATALOGO_SIGILO -->
```json
{
  "version": 4,
  "formas": [
    "clasico",
    "aspis",
    "ocho",
    "rombo",
    "ovalo",
    "torre"
  ],
  "campos": [
    "liso",
    "partido",
    "cortado",
    "cuartelado",
    "bandado",
    "palado",
    "jefe",
    "cruz",
    "aspa",
    "chevron",
    "bordura",
    "losanjado",
    "bandas",
    "zigzag",
    "ondas",
    "greca",
    "rombos",
    "radiante"
  ],
  "orlas": [
    "ninguna",
    "lisa",
    "greca",
    "cuerda",
    "puntos",
    "dentada"
  ],
  "emblemas": [
    "toro",
    "minotauro",
    "leon",
    "grifo",
    "caballo",
    "jabali",
    "cabra",
    "serpiente",
    "pulpo",
    "delfin",
    "espiral",
    "doble_hacha",
    "cuernos_consagracion",
    "carro",
    "aguila",
    "ciervo",
    "rayo",
    "sol",
    "estrella_ocho",
    "roseta",
    "creciente",
    "disco_alado",
    "escorpion",
    "carnero",
    "galera",
    "murex",
    "cedro",
    "palma",
    "ancora",
    "anfora",
    "halcon",
    "uraeus",
    "escarabajo",
    "anj",
    "ojo_udyat",
    "loto",
    "papiro",
    "esfinge",
    "corona_doble",
    "obelisco",
    "piramide",
    "columna",
    "templo",
    "trirreme",
    "casco_corintio",
    "aspis",
    "lechuza",
    "maza",
    "lambda",
    "olivo",
    "corona_laurel",
    "trigo",
    "lanza",
    "espada",
    "arco",
    "tridente"
  ],
  "colores": [
    {
      "id": "rojo",
      "nombre": "Rojo",
      "hex": "#b3261e"
    },
    {
      "id": "azul",
      "nombre": "Azul lapislázuli",
      "hex": "#1f4e9c"
    },
    {
      "id": "ocre",
      "nombre": "Ocre",
      "hex": "#d9a21b"
    },
    {
      "id": "verde",
      "nombre": "Verde",
      "hex": "#2e7d32"
    },
    {
      "id": "blanco",
      "nombre": "Blanco",
      "hex": "#f4efe4"
    },
    {
      "id": "negro",
      "nombre": "Negro",
      "hex": "#1c1b1a"
    },
    {
      "id": "purpura",
      "nombre": "Púrpura de Tiro",
      "hex": "#6a1b6e"
    },
    {
      "id": "naranja",
      "nombre": "Naranja",
      "hex": "#e0661a"
    },
    {
      "id": "turquesa",
      "nombre": "Turquesa",
      "hex": "#1b9aa6"
    },
    {
      "id": "marron",
      "nombre": "Marrón",
      "hex": "#6d4426"
    },
    {
      "id": "gris",
      "nombre": "Gris",
      "hex": "#7a7a78"
    },
    {
      "id": "oro",
      "nombre": "Oro",
      "hex": "#c9a227"
    }
  ],
  "reservados": {
    "neutro": {
      "formaId": "clasico",
      "campoId": "liso",
      "emblemaId": "sin_bandera",
      "colorPrimarioId": "gris",
      "colorSecundarioId": "blanco",
      "colorEmblemaId": "blanco",
      "orlaId": "ninguna",
      "colorOrlaId": "gris"
    },
    "bandidos": {
      "formaId": "clasico",
      "campoId": "liso",
      "emblemaId": "bandidos",
      "colorPrimarioId": "negro",
      "colorSecundarioId": "rojo",
      "colorEmblemaId": "blanco",
      "orlaId": "ninguna",
      "colorOrlaId": "rojo"
    },
    "mercenarios": {
      "formaId": "clasico",
      "campoId": "liso",
      "emblemaId": "mercenarios",
      "colorPrimarioId": "marron",
      "colorSecundarioId": "ocre",
      "colorEmblemaId": "blanco",
      "orlaId": "ninguna",
      "colorOrlaId": "ocre"
    }
  }
}
```
<!-- /parametros -->

## 4. Residencia, campamento y puerta

### 4.1 Reglas generales
- **Cooldown de residencia:** `cambiarResidencia`, `dejarResidencia` y `residirEnCampamento` exigen que hayan pasado `CIUDADANIA.cooldownCambioResidenciaDias` días desde el último (`cambiosResidenciaPorHeroe`); el primero
  es libre y la reubicación forzosa (conquista, ruina) **no cuenta**.
- Cada uno actúa solo sobre sí mismo (`heroeId` = el actor).
- El almacén personal **viaja con el héroe** al cambiar de base (→ `Movimiento_Ejercitos.md` §3).

### 4.2 Las cuatro vías
- **`cambiarResidencia(destino)`:** solo a una plaza de **su propia Facción** (ciudadano), a la que aún no reside y que no lo ha vetado. Atómico: deja la plaza anterior (si había una) con sus cargos locales,
  toma la nueva (`casasCompradas`), deja el campamento donde viviera, suelta la guarnición y estampa el cooldown. Su campamento (escuadras) se traslada sin más, porque viven en el héroe. Es la única puerta a
  vivir en una plaza que no sea fundarla.
- **`dejarResidencia`:** libera la vivienda y los cargos locales; el héroe pasa **en el acto** al campamento de mercenarios **más cercano a la plaza que deja** (5B-4, 2026-10-24; el TypeScript miraba dónde estaba el héroe; empate: id menor). Estampa cooldown.
- **Quien se muda estando dentro de la casa que deja** (5B-1, 2026-10-24): su campamento y su almacén personal pasan sin viaje a la casa nueva y él **sigue dentro de visita** si la puerta le admite; solo quien deja además la Facción puede quedarse fuera de la puerta, y entonces sale expulsado al mapa solo, con su carro. **No se muda nadie de una plaza en batalla** (5B-10): ni hacia ella ni desde ella.
- **`residirEnCampamento(id)`:** cualquier héroe, de cualquier Facción y aunque la suya tenga plazas; deja la plaza (con sus cargos) o el campamento anterior. Estampa cooldown. Sin límite de residentes.
- **Reubicación forzosa:** al arruinarse una plaza (`reubicarResidentesDeRuina`): a la plaza de su Facción más cercana a la ruina si queda alguna (pasan a residir ahí) y, si no, al campamento más cercano. Al
  conquistarla, → `Combate.md` (desalojo).

### 4.3 La puerta (`puedeEntrarEn`, `grupoDePuerta`)

`grupoDePuerta(relaciones, faccionDelJugador, faccionDelAsentamiento)`: la misma → `propia`; **alianza o vasallaje activos, en cualquier sentido** → `aliados`; **guerra activa** → `enemigos`; el resto → `neutrales`.
`puedeEntrarEn(plaza, heroeId, faccionDelHeroe, relaciones)`, por capas:
1. **Un residente entra siempre.** 2. Un **vetado** no entra. 3. Si no, entra si el grupo no está en `puertaCerradaA` (por defecto, `PUERTA.cerradaAPorDefecto`: neutrales y enemigos); la propia Facción nunca se bloquea.
`fijarPuerta(plaza, cerradaA[])` sustituye el conjunto de grupos cerrados (`neutrales`, `aliados`, `enemigos`, `aedas`; sin repetidos) y no caduca; lo hace el **Gobernador** residente y presente **o el Rey de la
Facción dueña** (no tiene que estar presente), y manda el último. `vetarJugador(plaza, vetado, vetar)`: añade o quita de `vetadosIds` (ordenado); solo el Gobernador residente y presente; **a un residente no se le
veta**. El grupo `aedas` lo usa `Tecnologia_Aedas.md`.

`cruzarLaPuerta` (entrar): solo en columna personal, a ≤ `MOVIMIENTO.radioPuerta` y con `puedeEntrarEn`; en la propia residencia la columna se disuelve (tropa al campamento; el carro se queda con el héroe, desde el
2026-10-22), en otra entra con todo y queda dentro. Detalle en `Movimiento_Ejercitos.md`.

> **Implementado en Rust (2026-10-22, entrar con todo): la puerta provisional.** Hasta el sistema político (5) no existen `fijarPuerta`, los vetos ni los grupos `aedas`. En su lugar `FijarAcceso { asentamiento, aliados, neutrales, enemigos }` (117) deja que quien manda en la plaza (`manda_en`: la administración o un ciudadano de su Facción) decida por **relación** (las de `FijarRelacion`: alianza o vasallaje → `aliados`; guerra → `enemigos`; el resto, y quien no tiene Facción, → `neutrales`) quién puede entrar; la misma Facción y los residentes pasan siempre. **Por defecto entran todos menos los enemigos** (la decisión del autor del 2026-10-22; el `PUERTA.cerradaAPorDefecto` del bloque, neutrales y enemigos, es del sistema político y no se aplica todavía). Los pueblos extractores y las avanzadas heredan el acceso de su plaza madre (`puede_entrar_en_pueblo`, `puede_entrar_en_avanzada`; se entra en ellos con `EntrarEnPueblo` y `EntrarEnAvanzada`, 2026-10-22). **Cerrar la puerta echa al mapa, al instante y con todo lo suyo, a quien ya está dentro y ya no puede entrar** (de la plaza y de sus pueblos y avanzadas; la misma Facción y los residentes no se expulsan nunca). Ver [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md).

> **Decisión Q1 del autor (2026-10-24) e implementado en Rust (5A, reglas puras):** la puerta es la de esta sección: por grupos con `aedas`, vetos del Gobernador, **por defecto cerrada a neutrales y
> enemigos** (`PUERTA.cerradaAPorDefecto`) y **cerrar o vetar expulsa** a quien ya no puede entrar. `crates/politica::puerta` (`Puerta::puede_entrar` por capas, `fijar`, `vetar`, `expulsados`,
> `admite_aedas`). El núcleo sigue con `FijarAcceso` hasta 5E, que lo sustituye por `FijarPuerta` y `VetarJugador`.

> **Implementado en Rust (2026-10-25, 5E):** la puerta de esta sección en el núcleo (`Asentamiento::puerta`, `politica::puerta::Puerta`, foto **`BAN33`**; `nucleo/cargos.rs`). `FijarPuerta { heroe,
> asentamiento, cerrada_a }` (159, `fijar_puerta`) la da el Gobernador residente y dentro o el Rey de la Facción dueña sin estar presente; `VetarJugador { heroe, asentamiento, vetado, vetar }` (160,
> `vetar_jugador`), solo el Gobernador (`puerta.residente` a un residente); sin cambio, `sin_cambios`. Cerrar o vetar saca al mapa, al instante y con todo lo suyo, a quien está dentro de la plaza, de sus
> pueblos o de sus avanzadas y ya no puede entrar; con un pueblo o una avanzada de la plaza en batalla, un jugador no la toca (`batalla.bloqueado`). La conquista la devuelve a la de por defecto (y
> borra los vetos). Hechos `asentamiento.puerta_fijada` (`cerrada_a`, la lista de grupos) y `asentamiento.veto` (`heroe`, `vetado`). `FijarAcceso` (117) queda en el núcleo como alias (cierra los
> grupos que van a `false`; los Aedas, como estaban). Un vetado de la Facción dueña tampoco repone en la plaza (`puedeReclutarEn`). El grupo `aedas` se guarda y se proyecta; quien lo mira es el
> sistema 6.

<!-- parametros:CIUDADANIA -->
```json
{
  "cooldownCreacionFaccionDias": 7,
  "cooldownCambioResidenciaDias": 3
}
```
<!-- /parametros -->

<!-- parametros:CAPITAL -->
```json
{
  "cooldownDias": 14
}
```
<!-- /parametros -->

<!-- parametros:CAP_FUNDACION_POR_NIVEL -->
```json
[
  1,
  2,
  3,
  3,
  4,
  5,
  5,
  6,
  6,
  7
]
```
<!-- /parametros -->

<!-- parametros:CUPO_NIVEL_ASENTAMIENTO -->
```json
{
  "maxNivel2": [
    1,
    1,
    1,
    1,
    2,
    2,
    2,
    3,
    3,
    3
  ],
  "maxNivel3": [
    0,
    0,
    1,
    1,
    1,
    2,
    2,
    2,
    2,
    3
  ]
}
```
<!-- /parametros -->

<!-- parametros:PUERTA -->
```json
{
  "cerradaAPorDefecto": [
    "neutrales",
    "enemigos"
  ]
}
```
<!-- /parametros -->

## 5. Relaciones entre Facciones

### 5.1 Quién puede hacer qué (autorización)
«Autoridad diplomática» = ciudadano de esa Facción y **Rey o Embajador** suyo.
| Comando | Quién |
|---|---|
| `proponerRelacion`, `declararGuerra`, `proponerAnexion` | autoridad diplomática de `faccionAId` (quien propone o declara) |
| `responderRelacion` | **Rey** de `faccionBId` de la propuesta |
| `retirarRelacion`, `retirarAnexion` | autoridad diplomática de la Facción que propuso |
| `romperRelacion` | autoridad diplomática de `iniciadorFaccionId` |
| `proponerPaz` | autoridad diplomática de `faccionId` (la que ofrece) |
| `rebelionVasallo` | autoridad diplomática de la **vasalla** (`faccionBId` de la relación) |
| `responderAnexion` | Rey de la absorbida |
| `proponerFusion`, `retirarFusion` | Rey de A; `responderFusion`: Rey de B |
| `asignarRey` | cualquier ciudadano si no hay Rey; si lo hay, solo él |
| `asignarEmbajador`, `designarCapital`, `admitirOtrasFacciones`, `asignarCargoLocal('gobernador')` | Rey |
| `asignarCargoLocal` (resto) | Gobernador residente y presente |
| `responderSolicitud` | Rey |
| `crearFaccion`, `solicitarIngreso`, `dejarFaccion` | cualquier héroe (las reglas las comprueba el comando) |

### 5.2 Validar un par (`validarPar`)
Distintas, las dos existen y **no hay entre ellas ninguna relación activa de ninguna clase** (la guerra cuenta). Por eso para declarar guerra a una aliada, a una señora o a una vasalla hay que romper antes la relación.

### 5.3 Alianza y vasallaje: propuesta y respuesta
- **Proponer** (`proponerRelacionPendiente`): `exigirRelacionPosible` + no hay otra propuesta vigente del par. Condiciones de `exigirRelacionPosible`: par válido; las dos tienen **Rey**; **vasallaje:** ninguna de las
  dos es ya vasalla de una tercera (basta con que lo sea cualquiera de las dos); **alianza:** `puedeProponerAlianza(A)` (reputación de A ≥ `REPUTACION.umbralBajoParaEmbajador`; la de B no se mira). Crea la propuesta
  (`expiraEn = ahora + RELACION_PROPUESTA.caducidadDias` días); en vasallaje lleva `tributo {recurso (por defecto trigo), cantidadPorMinuto (por defecto 0)}`; sin validar la cantidad.
  La lista se **poda de caducadas al añadir**: no hay barrido en el tick.
- **Responder:** debe estar vigente (si no existe o caducó, se rechaza con su código). **Rechazar:** la quita. **Aceptar:** vuelve a mirar `exigirRelacionPosible` y que **ninguna de las dos tenga una batalla
  abierta**; entonces nace la relación (`activa`, `creadoEn = ahora`) y se quita la propuesta.
- **Retirar:** la autoridad de la que ofreció, mientras nadie contestó.

### 5.4 Guerra y paz (`declararGuerra`, `proponerPaz`)
**Toda agresión entre Facciones exige guerra o la declara** (decidido el 2026-10-11, Doc 2.4.1): atacar una caravana, saquear un pueblo, atacar una avanzada, atacar una columna o asediar una plaza. Si no hay guerra con el dueño del objetivo, `atacar` sin la confirmación `declaraGuerra: true` se rechaza con un `detalleError` que enumera las consecuencias; con ella, **declara la guerra** (cualquier héroe puede, no solo el Rey o el Embajador) y la guerra se arrastra por vasallaje como siempre. Excepción: unirse como atacante a un asedio que abrió otro, siendo neutral y admitido por el Rey de la Facción atacante, no la declara. El comando `declararGuerra` (Rey o Embajador) sigue existiendo.
**Atacar a un aliado es una traición** (decisión **Q2** del autor, 2026-10-24): `atacar` con `declaraGuerra: true` contra una Facción con la que hay una **alianza** activa rompe la alianza en el acto (`rotaPor` = el atacante), le cuesta `penalizacionAtacarAliado` (−25, en lugar de los −12 de romperla, y **solo eso**: no suma los −15 de la guerra tras romper por la alianza que rompe en ese instante, 5A-1) y declara la guerra con su arrastre. Deja `rotaPor`/`rotaEn`, así que volver a declararle la guerra dentro de la ventana, tras una paz, sí cobra los −15. Con un vasallaje entre las dos se sigue rechazando (la vasalla se rebela, la señora libera). `declararGuerra` contra una aliada sigue exigiendo romper antes.
Al declarar, si `atacante` rompió una alianza con `objetivo` (`rotaPor`) hace menos de `ventanaGuerraTrasRomperAlianzaDias` días (`rotaEn`), pierde `penalizacionGuerraTrasRomperAlianza` de reputación
(`ajustesPorGuerraTrasRomper`, que aplica el comando). `romperRelacion` anota `rotaPor`/`rotaEn` solo en las alianzas; las que acaban por anexión o fusión no.

```
declarar(atacante, objetivo):  validarPar(atacante, objetivo)
    señor  = (el señor activo del objetivo, si es vasallo)  si no  el objetivo
    bando  = { señor, objetivo } ∪ { vasallos activos del señor }
    para cada id de bando, distinto del atacante :  nace una guerra atacante→id  salvo que ya haya una relación activa
                                                      del atacante con id  (el objetivo directo se incluye siempre: si la tiene, falló validarPar)
```
Queda **una `RelacionPolitica` por Facción alcanzada**, todas con el mismo contador. No hay condición previa (ni frontera, ni casus belli, ni batalla). 
**Paz:** por cada relación-guerra; la primera Facción en ofrecerla deja `pazPropuestaPor`; cuando la ofrece la **otra**, la guerra pasa a `rota` (`firmada`). La misma Facción no puede ofrecerla dos veces. Una guerra
**no** se rompe con `romperRelacion`.

### 5.5 Romper
- **`romperRelacion(relacionId, iniciador)`:** una guerra, no. **Alianza:** la rompe cualquiera de las dos; `penalizacionRomperAlianza` a quien rompe. **Vasallaje:** solo la señora (`bonusLiberarVasalloVoluntario` a ella);
  la vasalla solo puede rebelarse. La relación pasa a `rota`. Romper o rebelarse sobre una relación **ya rota** se rechaza (5A-6, 2026-10-24; el TypeScript no lo miraba y volvía a cobrar la reputación).
- **`rebelionVasallo(relacionId)`:** la vasalla; el vasallaje pasa a `rota`; los trueques `activo` **entre plazas de las dos Facciones** pasan a `expirado` (sin penalización); `penalizacionRebelionParaSenora` a la señora;
  y la vasalla **declara guerra a la señora** (con el arrastre de §5.4: a sus demás vasallos también). Evento `diplomacia.rebelion_vasallo`.
- **Señor desarmado** (`liberarVasallosDeSenoresDesarmados`, **al perder la señora su última plaza**, antes del tributo): todo vasallaje activo cuya señora **no tiene ningún asentamiento** pasa a `rota`, sin reputación ni guerra; evento
  `diplomacia.vasallo_liberado`. Ocurre en el instante de la caída de la última plaza.
- **Vasalla conquistada por un tercero** (`pasarVasallasConquistadas`, **al quedarse la vasalla sin plazas**, justo después del desarme del señor y antes del tributo; decidido el 2026-10-12): para todo vasallaje activo
  cuya vasalla **no tiene ninguna plaza** y cuyo `derrotadaPor` es otra Facción existente que **no es la señora**: el vasallaje pasa a `rota`, nace `vasallaje-{conquistador}-{vasalla}-c{instante}`
  con el **mismo tributo**, y **cualquier relación activa** entre vasalla y conquistador (la guerra, o la que hubiera) pasa a `rota`, para que nunca queden dos activas en el par (5A-3, 2026-10-24; el TypeScript solo acababa las guerras). Sin reputación. Evento `diplomacia.vasallo_conquistado`. Ocurre en el instante en que cae su
  última plaza, justo después del desarme del señor. Con alguna plaza, por ruina (`derrotadaPor = null`) o si la conquistó la propia señora, no cambia nada.

### 5.6 Tributo (`avanzarTributos`, como flujo continuo)
**Se paga en el paso de minuto de cada plaza vasalla** (decisión **Q3** del autor, 2026-10-24): sin eventos nuevos en la cola, y en **enteros con resto oculto** (C3b): cada plaza acumula su parte en un resto oculto y paga las unidades enteras, sin pasar de su stock; lo que no paga en ese minuto se pierde (no es deuda). **Lo que no cabe en el almacén de la capital se paga igual y se pierde** (5A-2, 2026-10-24, como el TypeScript). `cantidadPorMinuto` va en milésimas (contrato `Tributo`). Lo que sigue describe la regla de reparto y de tope.
Por cada vasallaje activo con tributo (decidido el 2026-10-11): el pactado `cantidadPorMinuto` se **reparte a partes iguales entre todos los asentamientos de la vasalla**; cada uno paga `min(su stock del recurso, su parte)` (si es > 0) y todo se suma al almacén de la **capital de la señora** (`encontrarCapital`, §3.7; topado por capacidad: el exceso se pierde). Un evento `diplomacia.tributo_pagado` por plaza que paga (`señoraId` = la capital). Sin capital o sin plazas de la vasalla, no hay pago.

### 5.7 Liga y Gran Rey (`computeLigas`)
No se guarda. Componentes conexas del grafo de relaciones **activas** (cualquier tipo, la guerra incluida) con ≥ 2 Facciones. `tieneVasallaje`: hay un vasallaje activo dentro. **Gran Rey:** la Facción señora de la
Liga que **nunca es vasalla** dentro de esa misma Liga (la primera que cumple).

### 5.8 Predicados de relación (en `pertenencia.ts`)
`estanAliadas` (alianza activa), `estanEnGuerra` (guerra activa), `compartenVision` (alianza **o** vasallaje, cualquier sentido), `grupoDePuerta` (§4.3).

<!-- parametros:RELACION_PROPUESTA -->
```json
{
  "caducidadDias": 3
}
```
<!-- /parametros -->

### 5.9 Defender al vasallo (`conReputacionDeVasallos`, al aplicar el resultado de una batalla)
```
vasalla = bando defensor.faccionId   (si no hay Facción, nada)
para cada señora con un vasallaje activo sobre vasalla:
    presente = algún héroe de la señora en el bando defensor, con participo = true en el resultado
    presente  y gana el defensor : reputación(señora) += bonusDefenderVasallo
    !presente y gana el atacante : reputación(señora) += penalizacionNoDefenderVasallo
    en cualquier otro caso       : nada
```
Vale para cualquier batalla cuyo defensor sea de la vasalla (plaza, caravana, y lo que venga), por las dos vías: la batalla de Unity (`conReputacionDeVasallos`, con los héroes que participaron) y el combate con números (`atacar` contra una plaza, una columna o una caravana: `conDefensaDeVasallo` en `session/comandos/interaccion.ts`, con los héroes que defendieron). Las dos usan `ajustesPorDefensaDeVasallo` (`engine/reputacion.ts`). Con números, defienden una plaza sus residentes y, en el cupo que dejan, los héroes de la propia Facción o de una aliada que estén dentro (`Combate.md` §3.2): la señora está presente si tiene un héroe dentro de la plaza de su vasalla; si no, aparece ausente y paga la penalización cuando la plaza cae.

## 6. Anexión y fusión

### 6.1 Anexión (A absorbe a B)
- **Proponer** (`proponerAnexion`): Rey o Embajador de A. `exigirAnexionPosible`: distintas; existen; **B tiene Rey**; B **no es vasalla de una tercera** (si su señor es A, vale: el vasallaje termina con la anexión). Una
  sola pendiente del par en cualquier sentido. `expiraEn = ahora + ANEXION.caducidadDias`; `propuestaPor` = el actor.
- **Responder** (solo el Rey de B): rechazar la quita; aceptar vuelve a mirar `exigirAnexionPosible` y que **ninguna de las dos tenga batalla abierta** (que ataque o defienda una, o que algún héroe suyo combata en una, como bando o incorporado: 5F-3), y ejecuta `anexionar` → `trasladarFaccion(B → A)` (§6.3).
  No se vuelve a comprobar que quien propuso aún tenga autoridad.
- **A conserva** nombre, Rey, Embajador, sigilo, reputación, experiencia y tecnología; de B se pierden sus cargos de Facción, sigilo, reputación, experiencia y tecnología. Evento público `diplomacia.anexion`.

### 6.2 Fusión (A y B se disuelven en C)
- **Proponer** (`proponerFusion`): el Rey de A, que fija `nuevoNombre` y `nuevoReyId`. `exigirFusionPosible`: distintas; existen; las dos tienen Rey; `nuevoReyId` es el Rey de A o el de B; nombre no vacío tras recortar;
  ninguna es vasalla de una tercera. **El nombre se revisa como en `crearFaccion`:** no vacío y no usado por otra Facción (sin mirar mayúsculas); las dos que se disuelven no cuentan, así que C puede quedarse con el nombre de una. `propuestaPor = a.reyId`.
- **Responder** (Rey de B): aceptar vuelve a mirar todo lo anterior, que `a.reyId` siga siendo `propuestaPor` y que no haya batalla abierta (como en la anexión, también un héroe de las dos incorporado: 5F-3).
- **Nace C:** `id = faccion-fusion-{instante}-{A}-{B}`, `nombre`, `sigilo` de A, `reyId = nuevoReyId`, `embajadorId = null`, `experiencia = max`, `reputacion = max` de las dos, `nivel = calcularNivelFaccion`,
  `ciudadanosIds` vacío (luego se llenan), tecnología = **unión** (aparecidas y adoptadas; reveladas menos las ya aparecidas). **Capitales:** la marca de capital de cada plaza sale; solo la que tenía designada el Rey
  electo pasa a `C` (`capitalDeFaccionId = C`), tal cual, sin mirar si sigue vigente (5A-5: `capitalDesignada` ya ignora una marca sin Palacio activo); `capitalDesignadaEn` de C queda sin definir. Después `trasladarFaccion(A → C)` y `trasladarFaccion(B → C)`. El Rey que no lo es de C queda como ciudadano.

### 6.3 `trasladarFaccion(origen → destino)` (ruta única)
Todo lo que lleva el id de `origen` pasa a `destino`, en este orden de efectos:
- **Relaciones activas del origen:** un **vasallaje en el que el origen es señor** pasa a `destino` (misma vasalla, mismo tributo) **salvo** que la vasalla sea el propio destino o `destino` ya tenga una relación
  activa con ella (entonces queda `rota`). Todas las demás relaciones activas del origen (alianzas, guerras, vasallajes en que es vasalla) pasan a `rota`, **sin reputación**.
- **Memoria:** la de `destino` y la del origen se funden (exploración por unión de bits; fichas e informes: gana la de `destino` si chocan, y se descartan las de plazas ya propias). La del origen se borra.
- **Tecnología:** se borra la del origen; `primeros[tec].faccionId` apunta a `destino` si era el origen.
- **Facciones:** el origen desaparece; `derrotadaPor` que apuntaba al origen apunta a `destino`; `destino.ciudadanosIds` = unión (sin repetir, el destino primero).
- **Asentamientos, ejércitos, caravanas, rutas de la red, Aedas residentes, Miradas:** su `faccionId` pasa a `destino`. Las épicas cumplidas del origen se **suman** a las de `destino`.
- **Propuestas** (anexión, fusión, relación) que nombraban al origen se descartan; las que otros hacen a `destino` siguen.
- **No se tocan:** `solicitudesIds` del origen (desaparecen con él), `salidasFaccionPorHeroe`. La marca `capitalDeFaccionId` del origen **se borra** de sus plazas al pasar a `destino`.
- **Trueques en curso (Q5, decisión del autor del 2026-10-24):** siguen como están (van entre plazas, y las plazas cambian de Facción); los que quedan entre plazas de la misma Facción ya no mueven la reputación al cumplirse o vencer.
- **Antigüedad de las plazas (5F):** en la anexión, las del origen son del destino desde el instante de la anexión (la capital efectiva del destino no cambia, como en una conquista: 5C-1); en la fusión conservan la suya, y la capital de C sin designar es la plaza más antigua de las dos (Doc 2.6).
- **La puerta (5F):** tras el traslado, quien está dentro de una plaza, un pueblo o una avanzada que ya no le admite (un aliado del origen dentro de una plaza suya, o un ciudadano del origen dentro de la de una enemiga del destino) sale al mapa con lo suyo, como al cerrar la puerta, con el motivo `cambia_de_dueno` de la conquista (`heroe.expulsado`; 5F-9, decisión del autor del 2026-10-25).
- **Cargos locales (5E, al integrar con 5F):** van con la plaza. Quien reside en una plaza del origen pasa con ella al destino y conserva sus cargos; en Rust las plazas cambian de Facción **antes** que los ciudadanos, porque quien deja de ser de la Facción dueña de su plaza pierde sus cargos en ella (§3.4).
- **Gran Fundición (5F, `// DUDA:`):** si las dos tenían una, la Facción que queda tiene dos; ninguna regla dice que se pierda una (la regla «una por Facción» es de construir), y se quedan las dos.

<!-- parametros:ANEXION -->
```json
{
  "caducidadDias": 3
}
```
<!-- /parametros -->

<!-- parametros:FUSION -->
```json
{
  "caducidadDias": 3
}
```
<!-- /parametros -->

## 7. Títulos de prestigio (`calcularTitulos`, por reevaluación)

Se recalculan enteros cuando cambia lo que miran (Facciones, plazas, relaciones, héroes) y como muestreo de baja frecuencia, tras la reputación, y no dan ningún beneficio mecánico. Con ≥ 1 Facción:
- **Facción más grande:** más asentamientos; empate: mayor población total.
- **Mayor poder económico:** más oro sumando los almacenes de sus plazas.
- **Ejército más grande:** más soldados sumando los escuadrones de **todos sus ciudadanos**, estén donde estén.
- **Gran Rey:** la Facción cima de una Liga por vasallaje (§5.7), si la hay (la primera Liga que tenga).
- **Mecenas de los Aedas:** `valor = épicas cumplidas × 10 + Aedas residentes`; gana el mayor y solo existe si es > 0.
En cada caso desempata el **orden de la lista de Facciones** (el sort es estable). `narrarCambiosDeTitulo` compara con los de la evaluación anterior y emite `titulo.cambia_manos` (el poseedor cambió) o `titulo.nace`
(no existía un título de ese nombre antes), públicos, cantados por los Aedas; no emite nada cuando un título desaparece.

**Cadencia (5F, decisión 12 del plan del sistema 5; C15):** un solo plazo **global** en la cola (prioridad 10, `Reloj_Y_Eventos` §2), cada `TITULOS.cadenciaMinutos` de mundo, que se arma solo
cuando pasa algo en el mundo (una orden aceptada o un evento de la cola que no sea el propio plazo) y no hay otro pendiente: un mundo sin nada que se mueva no deja nada en la cola. Al vencer se
recalculan todos con lo que hay en ese instante y se narran los cambios contra los guardados (estado de la foto). La cifra es un placeholder.

<!-- parametros:TITULOS -->
```json
{
  "cadenciaMinutos": 10
}
```
<!-- /parametros -->

## 8. Derrota (`registrarDerrota`)
Se llama al caer una plaza (conquista por asedio, batalla o ruina). Si la Facción **no conserva ninguna**, `derrotadaPor = ganadora` (o `null` si colapsó sola). Una Facción sin plazas **no desaparece**: sus héroes
siguen, residen en campamentos y puede volver a fundar (al fundar, `derrotadaPor` se borra). Un señor sin plazas libera a sus vasallos (§5.5).

## 9. Eventos y consumidores

| Evento | Quién lo emite |
|---|---|
| `faccion.creada`, `faccion.solicitud_ingreso`, `faccion.ciudadania_union`, `faccion.solicitud_denegada`, `faccion.abandonada`, `faccion.nivel_subio`, `faccion.capital_designada`, `faccion.admision_en_ataques` | comandos y tick |
| `cargo.rey_asignado`, `cargo.embajador_asignado`, `cargo.local_asignado` | comandos |
| `ciudadania.residencia_cambiada`, `…_dejada`, `…_en_campamento` | comandos |
| `diplomacia.relacion_propuesta`, `_aceptada`, `_rechazada`, `_retirada`, `relacion_rota`, `guerra_declarada`, `paz_propuesta`, `paz_firmada`, `rebelion_vasallo`, `vasallo_liberado`, `vasallo_conquistado`, `tributo_pagado` | comandos y tick |
| `diplomacia.anexion_propuesta/_rechazada/_retirada`, `diplomacia.anexion`, `diplomacia.fusion_propuesta/_rechazada/_retirada`, `diplomacia.fusion` | comandos |
| `asentamiento.puerta_fijada`, `asentamiento.veto` | comandos |
| `titulo.cambia_manos`, `titulo.nace` | tick |
La crónica de los Aedas canta la anexión, la fusión, la liberación de vasallos y los títulos (→ `Tecnologia_Aedas.md`).

## 10. Orden de los eventos del mismo instante
(Prioridades en [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §2, prioridad 9.) Liberar vasallos de señores desarmados → tributo → experiencia de construcción y niveles → `avanzarReputacion` y `sinSolicitudesCaducadas` → reposición de mercados → títulos.

## 11. Invariantes

| Invariante | Test que lo vigila |
|---|---|
| Quien crea una Facción es su primer ciudadano y Rey; un héroe, una Facción; cooldown de creación | `comandosMembresiaFaccion.test.ts` |
| Pedir entra una vez en la lista; aceptar da ciudadanía y quita la solicitud; denegar solo la quita | `faccion.test.ts`, `comandosMembresiaFaccion.test.ts` |
| `cambiarResidencia`: deja la vieja, toma la nueva, traslada el campamento, solo en la propia Facción; cooldown | `faccion.test.ts`, `comandosResidencia.test.ts` |
| `dejarResidencia` y `dejarFaccion` liberan la casa y los cargos locales; el héroe sigue ciudadano o no | `comandosResidencia.test.ts` |
| Residir en un campamento y reubicar de una ruina | `residenciaMercenaria.test.ts` |
| Rey/Embajador/cargos locales: quién puede ser designado y en qué orden | `cargos.test.ts` |
| Capital: Palacio activo, marca única, cooldown, sin cooldown si se perdió | `capital.test.ts` |
| Sigilo: catálogo, duplicado exacto, `sigiloLibre` determinista | `sigilo.test.ts` |
| Puerta: grupos, capas (residente > veto > grupo), propia Facción nunca bloqueada | `puerta.test.ts` |
| Proponer y responder alianza/vasallaje, una sola pendiente, caducidad, revalidación | `comandosDiplomacia.test.ts` |
| Eventos de diplomacia (guerra, paz, rebelión, tributo, liberación) | `eventosDominioDiplomacia.test.ts` |
| Anexión: propuesta, respuesta, traslado, relaciones, desarme del señor | `comandosAnexion.test.ts` |
| Fusión: propuesta, respuesta, qué hereda C, capital, relaciones | `comandosFusion.test.ts` |
| Derrota: solo se anota si no queda plaza | `derrotas.test.ts` |
| Vasalla conquistada por un tercero (traspaso, tributo, guerras, casos sin efecto) | `vasallaConquistada.test.ts` |
| Defender al vasallo (bonus, penalización, sin efecto; con números y en Unity) y guerra tras romper una alianza (ventana y quién paga) | `reputacionVasallos.test.ts`, `batallas.test.ts` |
| Títulos y experiencia | `eventosDominioTitulos.test.ts`, `experienciaFaccion.test.ts` |

## 12. Casos límite y decisiones que parecen raras
- **`derrotadaPor` no caduca mientras la Facción no tenga plaza:** si una vasalla ya conquistada, sin plazas, se hace vasalla de otra Facción distinta del conquistador, en la reevaluación siguiente pasa a ser vasalla del conquistador.
- **El señor «debe defender» solo como reputación** (decidido el 2026-10-12): no hay obligación en el motor; no defender cuesta reputación y es todo el castigo.
- **Una propuesta caducada no desaparece sola:** queda en la lista hasta que se añade otra del mismo tipo (que poda las caducadas) o se contesta; ya no se puede contestar ni cuenta como «pendiente».
- **Declarar guerra no arrastra a los aliados del bando contrario con los que ya hay relación:** solo se salta quien tiene una relación activa con el atacante; el objetivo directo, si la tiene, hace fallar todo.
- **El tributo se reparte entre todas las plazas de la vasalla:** una plaza sin stock no paga y las demás no cubren su parte; con `cantidadPorMinuto = 0` no pasa nada.
- **La alianza da reputación mínima:** el bonus por alianza (`bonusPorMinutoAlianzaActiva`) es menor que el decaimiento, así que por sí sola no sube una reputación por encima de ~0,05 (suman varias alianzas).
- **El Embajador designado puede ser el propio Rey**, y al irse el Rey el Embajador se vacía si el heredero era él mismo.
- **`asignarRey` con Rey ya presente** es un traspaso: el antiguo sigue siendo ciudadano; el Embajador no se toca.
- **Aceptar una anexión no mira si quien propuso conserva autoridad** (la fusión sí mira que el Rey de A siga siendo el mismo).
- **La puerta cerrada a `aliados` cierra también a vasallos y señores** (el grupo los incluye).
- **Un veto no alcanza a un residente:** se rechaza al vetar, y si alguien vetado acaba residiendo (p. ej. reubicado desde una ruina, que no mira los vetos) entra siempre.

## 13. Divergencias
Anotadas en [`Divergencias.md`](Divergencias.md).

## Catálogo de eventos

Los códigos de los hechos de dominio de este documento (4H T5, 2026-10-23; contrato 05 §8). La prueba `crates/nucleo/tests/catalogo.rs` lee el bloque de cada documento de `Docs/Mechanics` y exige que todo código que emite el núcleo esté en alguno, y que los que el núcleo aún no emite figuren como paridad pendiente en `Docs/Mecanicas a desarrollar.md`. De estos, aún no se emiten 2 (`asentamiento.veto` y `cargo.local_asignado`, de 5E); los ocho de la anexión y la fusión y los dos de los títulos se emiten desde 5F (2026-10-25); los diez de 5B se emiten desde el 2026-10-24, `faccion.nivel_subio` y `faccion.capital_designada` desde 5C y los doce de las relaciones (`diplomacia.relacion_propuesta`, `_aceptada`, `_rechazada`, `_retirada`, `diplomacia.relacion_rota`, `diplomacia.guerra_declarada` —que sustituye a `faccion.guerra_declarada`—, `diplomacia.paz_propuesta`, `diplomacia.paz_firmada`, `diplomacia.rebelion_vasallo`, `diplomacia.vasallo_liberado`, `diplomacia.vasallo_conquistado`, `diplomacia.tributo_pagado`) y `faccion.admision_en_ataques` desde 5D (el mismo día).

<!-- eventos -->
```json
[
  "asentamiento.veto", "cargo.embajador_asignado", "cargo.local_asignado", "cargo.rey_asignado", "ciudadania.residencia_cambiada", "ciudadania.residencia_dejada",
  "ciudadania.residencia_en_campamento", "diplomacia.anexion", "diplomacia.anexion_propuesta", "diplomacia.anexion_rechazada", "diplomacia.anexion_retirada",
  "diplomacia.fusion", "diplomacia.fusion_propuesta", "diplomacia.fusion_rechazada", "diplomacia.fusion_retirada", "diplomacia.guerra_declarada", "diplomacia.paz_firmada",
  "diplomacia.paz_propuesta", "diplomacia.rebelion_vasallo", "diplomacia.relacion_aceptada", "diplomacia.relacion_propuesta", "diplomacia.relacion_rechazada",
  "diplomacia.relacion_retirada", "diplomacia.relacion_rota", "diplomacia.tributo_pagado", "diplomacia.vasallo_conquistado", "diplomacia.vasallo_liberado",
  "faccion.abandonada", "faccion.admision_en_ataques", "faccion.capital_designada", "faccion.ciudadania_union", "faccion.creada", "faccion.nivel_subio",
  "faccion.solicitud_denegada", "faccion.solicitud_ingreso", "titulo.cambia_manos", "titulo.nace"
]
```
<!-- /eventos -->
