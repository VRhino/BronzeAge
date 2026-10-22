# Economía y construcción de un asentamiento

Cómo nace un asentamiento, qué produce, cómo se construye y se mejora, cómo crece y come su gente, qué cuesta mantenerlo y cómo sube (o baja) de nivel. Es el
sistema que corre **dentro de cada asentamiento, con un paso de un minuto por plaza y los fines (obra, mejora, ascenso, política) en su instante exacto**; lo que ocurre entre asentamientos (comercio, caravanas) y la guerra están en otros documentos.

- **Reescrito el 2026-10-13** para el mundo reactivo: §1 pasa de «orden dentro del tick» a ritmos y eventos, el crecimiento de población es continuo (sin redondeo estocástico) y se añaden los pueblos extractores y la calibración (§14). Las fórmulas «por tick» de las demás secciones son **ritmos por minuto**, idénticos; donde el texto diga «cada tick» léase «continuamente, con eventos en los umbrales». El «cómo funcionaba» por tick está en `typescript-final:Docs/Mechanics/Economia_Construccion.md`.
- **Corregido el 2026-10-22** ([`Economia_Por_Minuto_Definicion.md`](../../Consideraciones/Economia_Por_Minuto_Definicion.md)): la economía **no** es continua. Sustituye al plan de «economía continua», descartado ese día. Se queda **un paso de un minuto por plaza** (producción, consumo, recaudación, mantenimiento, población y evaluación de la auto-construcción) y solo lo que el jugador nota va en su instante exacto: el inicio de una obra encargada a mano (al clic), el fin de obra, de mejora y de ascenso, y la caducidad de política. La población es **siempre entera**, con un resto oculto, y el hambre sigue D7 y D8 (§9.2). Donde el texto de abajo diga «ritmo» o «continuo» (reescritura del 2026-10-13) léase «por minuto»; §1 y §9 están al día.
- **Verificado contra:** `d406eed` (2026-10-10) y revisado el 2026-10-11 con las decisiones del usuario sobre las divergencias (piezas gratuitas, almacén lleno, Gran Fundición, reembolso de dañados, sin líneas de producción).
- **Módulos:** `src/engine/construction.ts` (≈1.870 líneas: cola de obras, auto-construcción, producción, recetas, mejoras, comandos manuales),
  `settlement.ts` (fundación y viabilidad), `almacen.ts` (primitivas de almacén), `asentamientoQuery.ts` (cupos, mano de obra, progreso de nivel),
  `population.ts` (crecimiento, comida, impuestos), `mantenimiento.ts` (coste, medidor, nivel operativo, ruinas), `ascenso.ts` (subida de nivel),
  `politicas.ts` (factores), `zones.ts` (zona de influencia), `tropas.ts` (`reservaDeTrigo`), `world/mapa.ts` (nodos, bosques, regeneración),
  `session/comandos/construccion.ts` y `ascenso.ts` (comandos).
- **Canon:** Doc 4 (4.1 población, 4.2 construcción y catálogo, 4.3 almacén, 4.4 políticas, 4.5 mantenimiento y nivel), Doc 1.2–1.3 (zona y fundación).
- **Fuera de alcance aquí:** la colocación sobre la rejilla (dónde cae cada edificio) → [`Trazado_Urbano.md`](Trazado_Urbano.md); los recintos → [`Murallas.md`](Murallas.md);
  la guarnición y las tropas (`avanzarMantenimientoTropas`, reclutar) → combate/militar; el comercio; el orden global del tick; la fertilidad y los nodos del mapa
  (los genera `worldgen`, aquí son una caja negra: `fertilidadEn`, `bosque`, `stock`).

> **Unidades.** La economía de una plaza avanza de **un minuto de mundo** en un minuto (el evento recurrente `Carga::Minuto`, con la fase de su fundación): toda tasa «por minuto» se aplica entera en cada paso, y un cambio a mitad de minuto (edificio terminado, política, reclutamiento) cuenta el minuto entero en el paso siguiente. Los tiempos de obra, mejora, ascenso y política son **instantes de fin con su propio evento** (§1).

## 1. Cómo corre un asentamiento (un paso por minuto y fines exactos)

Decidido por el autor el 2026-10-22 ([`Economia_Por_Minuto_Definicion.md`](../../Consideraciones/Economia_Por_Minuto_Definicion.md)): la economía no se convierte en curvas continuas (cuesta ~5 s de CPU por día con 1.000 plazas y no afecta a la experiencia). Se corrige solo lo que el jugador nota.

| Qué | Cuándo |
|---|---|
| Gastos y entradas discretas: órdenes (pagar obra o mejora, reclutar, comprar), caravanas, entregas, botín, saqueo, acarreo | al instante de la orden o del evento |
| **Inicio de una obra encargada a mano**, si hay cuadrilla libre | **al instante de la orden** |
| **Fin de obra, de mejora y de ascenso** | **en su instante exacto**, con una cita propia en la cola (`Carga::FinDeObras`, prioridad 1: antes del paso de minuto del mismo instante) |
| **Caducidad de política** | **en su instante exacto**, con cita propia (`Carga::CaducaPolitica`, prioridad 3: después del paso de minuto del mismo instante) |
| **Cuadrilla liberada y siguiente obra de la cola** | **en el mismo instante del fin** |
| Producción, extracción, recetas, consumo de comida, recaudación, mantenimiento, guarnición, pueblos | en el paso de minuto |
| Evaluación de necesidades de auto-construcción, mejoras automáticas, arranque de las obras comprometidas | en el paso de minuto |
| Población (crecimiento y muertes) | en el paso de minuto, **siempre entera** (§9) |

> **Implementado en Rust (2026-10-18; fines exactos y población entera, 2026-10-22).** Reglas puras en `crates/asentamiento` (población, almacén, producción, obras, auto-construcción, mantenimiento, ascenso, políticas); su estado y su reloj en `crates/nucleo`: `economia.rs` (mapa, yacimientos y sitios sobre el mundo real, compartidos con el batch), `codec_economia.rs` (serialización para el hash y las fotos), `nucleo/construccion.rs` (el paso, las citas y las órdenes) y `nucleo/zonas.rs` (la zona crece con cada edificio completado). Cada plaza avanza con un evento recurrente `Carga::Minuto` (prioridad 2, fase = su instante de fundación). **Fines exactos:** cada obra, mejora o ascenso que empieza programa una cita `Carga::FinDeObras` en su `completaEn` (la programa `anotar_sucesos` a partir de `ObraIniciada`, `MejoraIniciada` y `AscensoIniciado`, que ahora llevan `completaEn`) y cada política activada, una `Carga::CaducaPolitica` en su `expiraEn`; ambas llevan la generación de la plaza (la ruina las descarta), no son recurrentes y desaparecen al cumplirse. La cita llama a `Asentamiento::cumplir_fines` (obras, mejoras, ascenso y, si acabó una obra o una mejora, el arranque de la cola con la cuadrilla recién libre) o a `caducar_politicas`, y el núcleo agranda la zona por los edificios completados en ese mismo instante. Son idempotentes: una cita que llega con la plaza ya cambiada (una conquista descartó el ascenso, `FijarNivelAsentamiento` también) no encuentra nada que cumplir. Quien no tiene cola (pruebas y batch) usa `Asentamiento::avanzar_hasta`, que cumple los fines de `(ultimo_minuto, t]` en su instante y da el paso. Una **mejora que espera hueco** (la Granja) conserva su cuadrilla y la reintenta el paso de minuto. Población, almacén y medidor cambian el hash pero no la versión del mundo; esta solo sube si el paso o la cita dejan algo discreto (obra, mejora, ascenso, política, hambruna, déficit...) o cambia la zona. La foto es `BAN14`. Parte del `Exterior` sigue fija en el núcleo (fertilidad 1,0, un ciudadano, capital sin distancia, sin recintos); la tecnología de la Facción, la ración de la guarnición y los factores de la ocupación ya vienen de sus sistemas.

**Desempate del mismo instante** (prioridad de tipo, [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §2): el fin de una obra, mejora o ascenso va **antes** del paso de minuto de la plaza (el edificio terminado cuenta ese minuto, como siempre: `completar_obras` iba primero); la caducidad de una política va **después** (una política de `d` minutos se aplica en `d` pasos, incluido el que cae en su instante de caducidad); la población come antes que las tropas; la recaudación usa la población ya crecida; el oro recaudado paga el mantenimiento del mismo instante; una ruina deja de existir para el comercio y los ejércitos de ese instante.

**Auto-construcción:** `evaluarNecesidades` (§6.2) corre en el paso de minuto, con el estado al empezar el paso, y compromete proyectos (pagados, en cola) que arrancan en el paso siguiente si hay cuadrilla. Una obra encargada **a mano** no espera: arranca en el instante de la orden.

**Consecuencias sobre los pasos de §6:**
- Una obra encargada a mano arranca **en el instante de la orden** si hay cuadrilla libre; una comprometida por la auto-construcción en el paso *t* arranca en *t+1 min* (o antes, si una cuadrilla se libera a mitad del minuto); termina en `completaEn = inicio + duración`, en ese instante.
- La medida de cuadrillas (§6.1) usa el estado **en el instante del evento**; una obra o una mejora que termina libera su cuadrilla en ese mismo instante y la **siguiente obra de la cola** (por prioridad) arranca entonces. **Ruptura de paridad con el TypeScript, decidida por el autor:** allí el Paso 2 medía las cuadrillas «al empezar el tick» y una obra que acababa no liberaba hueco hasta el tick siguiente (`ocupadasAlEmpezar`).
- La experiencia de Facción por edificio completado y por ascenso se aplica **al ocurrir** el evento.
- Los recintos ven su obra (Murallas §5) como cualquier otra obra de fin programado.

## 2. Modelo de datos

```
RecursoAlmacenado = { cantidad, capacidad }          Asentamiento.almacen: Record<RecursoTipo, RecursoAlmacenado>
```

29 tipos de recurso (`RECURSOS_TIPO`): madera, piedra, trigo, cobre, estano, oro, livestock, hierro; intermedios (lingoteCobre/Estano/Bronce/Hierro, cuero, cueroCurtido,
cueroCalidad); equipo (armaMadera/Cobre/Bronce/BronceCalidad/Hierro/HierroCalidad, armaduraBasica/Intermedia/Bronce/BronceCalidad, carroGuerra); chatarra (Cobre/Bronce/Hierro).
**Todos** nacen en el almacén (cantidad 0 o inicial, capacidad 400) para que la capacidad exista desde el principio.

```
Edificio = {
  id, tipo, posicion, ambito: 'asentamiento'|'mapa' (por defecto 'asentamiento'),
  estado: 'en_cola' | 'en_construccion' | 'activo',
  completaEn?       solo en_construccion: instante en que pasa a activo (fecha absoluta)
  prioridad?        score con que se comprometió; ordena el arranque de obra
  fuenteId?         nodo (cantera/minas/corral) o bosque (leñera) que explota
  nivelInterno?     ausente = 1
  mejora?           { nivelObjetivo, completaEn } mientras se mejora (sigue 'activo' y produciendo)
  danado?           un saqueo lo dejó en cola para reconstruir (§8)
  pausadoPorAlmacenLleno?   solo informativo (§5.5)
  rotado?, anclaLlena?, semillaSaturada?   geometría (Trazado_Urbano.md)
}
```

Campos de `Asentamiento` que usa este documento:

| Campo | Significado |
|---|---|
| `nivel` | **Nivel alcanzado**: monótono, solo sube al terminar una obra de ascenso. Fija el techo de población y el radio máximo de zona. |
| `nivelActual` | **Nivel operativo**: sube y baja con la salud del mantenimiento, nunca por encima de `nivel`. Decide qué se puede construir/mejorar y qué materiales cobra el mantenimiento. |
| `rachaMantenimientoSano` | Ticks seguidos con el mantenimiento pagado entero (ausente = 0). |
| `medidorMantenimiento` | 0–100, nace en 100. |
| `nutricionPoblacion` | 0–100, ausente = 100. |
| `poblacion` | `{ pesants, artesanos, nobleza }`. |
| `radioPotencial` | Radio de la zona de influencia. |
| `ascenso?` | `{ nivelObjetivo, iniciadoEn, completaEn }` — obra de ascenso en curso. |
| `autoConstruccionPausada?`, `recetasPausadas?` (recursos), `reservaManual?` (por recurso, 0–`reservaManualMaxima`) | Controles del jugador. |
| `extractoresTicksSinCupo?` | Contador anti-inanición por tipo de extractor (§6.2). |
| `ocupacionHasta?`, `protegidaHasta?` | Ventanas tras una conquista o una fundación. |
| `cargos` | `gobernadorId, tesoreroId, generalId, maestroObrasId, sacerdoteId`. |

## 3. Fundación (`fundarAsentamiento`)

Validaciones, en este orden (cada una lanza `FundacionInvalidaError`):

1. Entre 1 y `FUNDACION.maxJugadoresFundacionGrupal` (5) héroes fundadores.
2. La posición está dentro del mapa.
3. `posicionLibreParaFundar`: el punto **no cae dentro de la zona real (recortada) de ningún asentamiento existente** (de cualquier Facción, también la propia) y está **a `separacionPlazas` (30) o más de cualquier plaza** (Doc 1.0a: con zonas pequeñas la separación manda).
4. El terreno no es `cima`, ni `agua`, ni una celda **no transitable** del artefacto (río sin vado, pendiente: la plaza quedaría incomunicada). En Rust las tres comprobaciones de terreno van antes que las de zona y distancia.
5. La Facción existe y no ha llegado al tope de fundación `calcularCapFundacion(nivel de Facción)` (cuenta los asentamientos que ya tiene).

`exigirPuertaDeFundacion` (mínimo de fundadores `FUNDACION.minFundadoresParaFaccionNueva`, ciudadanía previa `exigeCiudadaniaPrevia`) es una función aparte para fundar una **Facción
nueva**; hoy está abierta (1 fundador, sin ciudadanía previa).

Estado inicial:

```
id            = 'asentamiento-<nº de asentamientos existentes>-<round x>-<round y>-t<floor(fundadoEn / 60000)>'
                (el minuto de mundo lo hace único si se refunda donde cayó otro; el id es además la semilla del trazado urbano)
nivel = nivelActual = 1;  rachaMantenimientoSano = 0;  medidorMantenimiento = 100
radioPotencial = ZONA_INFLUENCIA.radioInicial   (§3.1)
poblacion      = { pesants: POBLACION.pesants.inicial, artesanos: 0, nobleza: 0 }
almacen[tipo]  = { cantidad: FUNDACION.materialesIniciales[tipo] ?? 0, capacidad: ALMACEN.capacidadInicialPorRecurso }   (los 29 tipos)
edificios      = centroUrbano (0,0) + FUNDACION.viviendasIniciales viviendas + 1 granja, todos 'activo' y de ámbito 'asentamiento',
                 colocados con sitioEnBarrio sobre una ciudad que solo tiene el centro (ids: 'edificio-<id>-centro-urbano', '-vivienda-inicial-<i>', '-granja-inicial')
cargos         = todos null;  casasCompradas = fundadores;  politicasActivas = []
protegidaHasta = fundadoEn + OCUPACION.proteccionMinutos         (nace protegida un día, como una plaza recién conquistada)
```

A la Facción se le quita `derrotadaPor` (quien vuelve a tener un asentamiento ya no está derrotado), cada fundador recibe ciudadanía, se borran sus solicitudes de ingreso y, **si la Facción ya tenía
algún asentamiento**, gana `NIVEL_FACCION.xp.fundacion` de experiencia (el primero no: es nacer, no crecer). La fundación no cobra nada al almacén (el gasto es el de la caravana de fundación, fuera de aquí).

**Viabilidad** (`evaluarViabilidadFundacion`, solo lectura, **no impide** fundar): `fundable = dentroDelMapa ∧ posicionLibre ∧ terreno ∉ {cima, agua}`; `recomendable = fundable ∧ hay un bosque con hueco libre de
Leñera en la zona real con que nacería ∧ hay ≥ 1 nodo de piedra sin explotar en esa zona`. «Zona real» = círculo del radio inicial recortado contra los asentamientos de **otras** Facciones
(`zonaInicialDeFundacion`). Hay bosque libre si algún bosque cuyo borde toca la zona tiene menos Leñeras que su capacidad (`capacidadLeneras`, §5.2).

### 3.1 Zona de influencia

> **Implementado en Rust (2026-10-13)**: `crates/territorio/src/zona.rs` (reglas puras) y `crates/nucleo/src/nucleo/zonas.rs` (estado, órdenes y eventos).

La zona es un **disco** (centro = la plaza). Nace con `radioInicial` (6) y **solo crece por obra, nunca por tiempo**: cada edificio completado la agranda `crecimientoPorEdificioCompletado` (1), sin pasar del **techo de su nivel** (`radioMaximoPorNivel`: 10 / 15 / 20 / 25 / 30). Subir de nivel solo levanta el techo: el radio no salta. La construcción llama a `AgrandarZona` al completar cada edificio (hasta entonces la administración); el ascenso, a `FijarNivelAsentamiento`.

**Contacto.** Como el radio solo cambia en esos instantes, no hay eventos programados: al fundar y al agrandar una zona se comprueba qué plazas de **otra Facción** quedan tocándola (`distancia ≤ suma de radios`) y se anuncia una sola vez por pareja (`zona.frontera`, `Estado.contactos`). Las zonas nunca se encogen, así que un contacto no se deshace.

**Fronteras duras.** Desde el contacto la zona de una plaza queda recortada por un semiplano contra cada vecina de otra Facción que la toca: la frontera pasa a `d · r_propio / (r_propio + r_rival)` del centro propio, perpendicular a la recta que une los centros, y **se mueve** cuando crece un radio. Los de la **misma** Facción no se recortan (se solapan). Pertenecer a la zona es una consulta exacta en enteros (`Nucleo::en_zona`: `(p − c)·(o − c)·(r + r') ≤ d²·r`, sin raíces); el polígono de 48 lados (`Nucleo::poligono_zona`, Sutherland-Hodgman) es solo presentación (`Asentamiento.zona`) y no entra en ninguna decisión ni en el hash.

## 4. Almacén

- `agregarRecurso(a, r, n)`: `cantidad = min(capacidad, cantidad + n)`. `agregarRecursoConSobrante` además devuelve lo que no cupo.
- `descontarRecursos`: resta sin comprobar; un resultado entre −1e-9 y 0 se redondea a 0 (ruido de coma flotante); uno más negativo se deja negativo (lo vigila el test de invariantes).
- `tieneRecursos`: todos los recursos del coste con `cantidad ≥ coste`.
- `ampliarCapacidad(a, r, extra)`: `capacidad = max(0, capacidad + extra)` y `cantidad = min(cantidad, capacidad)` (lo que no cabe se pierde).
- **Capacidad por edificio** (`aplicarCapacidadDeEdificio`, `signo` ±1): el Almacén suma `signo × capacidadPorRecursoAdicional` (300) a **cada recurso que exista**; el Granero suma `signo × capacidadTrigo[nivel]` al trigo.
  Se suma al **entrar en servicio** (obra o reconstrucción terminada) y se resta al dejar de estarlo (daño de un saqueo). Una mejora de Granero suma el **delta** entre niveles.
- Tope de Almacenes por nivel operativo: `NECESIDADES.maximoAlmacenesPorNivel[nivelActual]`, contando los de **cualquier estado**. Tope de Viviendas: §6.5.

## 5. Producción

Cada edificio `activo` produce cada tick, **en el orden del array `edificios`** (importa si el almacén se llena a mitad del recorrido). Todas las cantidades se añaden con tope de capacidad y, si no caben, la producción **se pausa** (§5.5).

### 5.1 Mano de obra

No hay asignación por edificio: hay una demanda agregada.

```
trabajadoresRequeridosTotal   = Σ EDIFICIO_CATALOGO[tipo].trabajadoresRequeridos   de los edificios ACTIVOS de tipo granja, cantera, lenera, mina, minaCobre, minaEstano, minaHierro, corral
                                (el valor del tipo, no el del nivel interno; un extractor con el nodo agotado sigue contando)
ratioManoObra                 = trabajadoresRequeridosTotal ≤ 0 ? 1 : min(1, pesants / trabajadoresRequeridosTotal)

trabajadoresTransformacion    = Σ niveles[nivelInterno].trabajadoresRequeridos   de los ACTIVOS fundicion, curtiduria, armeria, carpinteria, granFundicion
ratioManoObraArtesanos        = trabajadoresTransformacion ≤ 0 ? 1 : min(1, artesanos / trabajadoresTransformacion)
```

La Gran Fundición cuenta como los demás talleres (sus 12 artesanos). Los pesants que sobran (`pesants − trabajadoresRequeridosTotal`) son el pool de reclutamiento (`poblacionDisponibleParaReclutar`):
pesants → el excedente; artesanos → `artesanos − trabajadoresTransformacion` (≥ 0); nobleza → toda.

### 5.2 Extractores y granjas

| Edificio | Producción por tick | Notas |
|---|---|---|
| Granja | `produccionTrigoDeGranja(nivelInterno) × fertilidadZona × ratioManoObra × factorProduccionTrigo` | `produccionTrigoDeGranja` = `niveles[nivel].produccionBaseTrigo` (60/90/120/180) |
| Leñera | `produccionBaseMadera × bosque.densidad × ratioManoObra` | no usa fertilidad ni tecnología; si el bosque no existe, no produce |
| Cantera, Mina (oro), Mina de cobre / estaño / hierro, Corral | `mapa.extraer(fuenteId, produccionBase × ratioManoObra × factorTecnologia)` | rinde piedra, oro, cobre, estaño, hierro, livestock. Solo si el nodo tiene stock |

- **`fertilidadZona`** (`mejorFertilidadEnZona`): la mayor `mapa.fertilidadEn` de una lista de candidatos = la posición del asentamiento + `SITIO.muestrasFertilidad` (40) puntos `i = 0..39` en
  `(pos.x + cos(2π·i/40)·r, pos.y + sin(2π·i/40)·r)` con `r = ((i mod 5)/5) · radioPotencial`, quedándose con los que caen dentro del polígono de zona (o con todos si el polígono está vacío). Todas las Granjas
  del asentamiento comparten este único valor.

<!-- parametros:SITIO -->
```json
{
  "muestrasFertilidad": 40
}
```
<!-- /parametros -->

- **`factorTecnologia`** = producto de `bonusProduccion.factor` de las tecnologías adoptadas cuyo `bonusProduccion.recurso` coincide con el recurso del extractor (p. ej. `canteria` ×1,5 la piedra).
- **Extracción**: el extractor pide `min(produccionBase × …, espacio libre del almacén para ese recurso)` y `mapa.extraer` devuelve `min(pedido, stock)`: **solo se gasta del yacimiento lo que cabe**. El stock es `cantidadInicial − extraído`. Si tras extraer el stock llega a 0, se emite
  `construccion.yacimiento_agotado`.
- **Regeneración** (`mapa.avanzarRegeneracion`, global del tick): un nodo con stock 0 que aún no tiene calendario agenda `instante + cooldown` (`REGENERACION_NODOS`: `livestock` 3 min; **todo lo demás** 10 min); cuando
  `instante ≥ calendario` se borra lo extraído y el nodo vuelve a su cantidad inicial. Evento `mapa.yacimiento_regenerado`.
- **Bosques** (en la reconstrucción: parcelas, ver nota en `Generacion_Del_Mundo.md` §10.3): un bosque admite 1, 2 o 3 Leñeras según su radio (`LENERA_POR_BOSQUE`: ≥ `umbral3` → 3, ≥ `umbral2` → 2, si no 1). La Leñera se coloca en el espacio de la ciudad, pero su
  `fuenteId` es el bosque.
- **Nodos reclamados**: un yacimiento lo explota un solo extractor de **todo el mundo** (`reclamosDeFuentes`: conjunto de `fuenteId` de todos los asentamientos; las Leñeras se cuentan aparte por bosque).

### 5.3 Recetas (`avanzarRecetas`)

Para cada edificio `activo` con `niveles` (fundicion, curtiduria, armeria, carpinteria, granFundicion), en el orden del array; para cada receta del **nivel interno actual** en el orden del catálogo
(una receta puede consumir lo que otra produjo este mismo tick):

```
saltar si  recetasPausadas contiene receta.produce   ó  falta la tecnología  ó  receta.requiereEdificio no está (algún edificio activo de ese tipo con nivelInterno ≥ nivel)
cantidad = produccionBase × ratioManoObraArtesanos × factorProduccionTalleres
para cada insumo:  disponible = max(0, almacen[insumo].cantidad − reserva[insumo]);  cantidad = min(cantidad, disponible / porUnidad)
espacio = capacidad − cantidad de receta.produce;  si cantidad > espacio: cantidad = espacio y el edificio queda marcado pausadoPorAlmacenLleno
si cantidad ≤ 0: nada
descontar  porUnidad × cantidad  de cada insumo;  añadir cantidad de receta.produce
```

- Con el producto sin sitio la receta **para**: solo se fabrica (y solo se consumen los insumos de) lo que cabe.
- `reserva` es la misma de la auto-construcción (§6.3): un taller nunca baja un recurso por debajo de lo que el mantenimiento y la comida van a cobrar, más la reserva manual del Tesorero.
- **No hay penalización por distancia** entre el taller y sus fuentes (se descartó el 2026-10-11): la posición de un edificio dentro de la ciudad no cambia lo que produce.
- `factorProduccionTalleres` es el producto de las políticas activas (Barrios Gremiales ×1,1).
- Las recetas devuelven además la cantidad fabricada por recurso, que alimenta los logros de Tecnología.

### 5.4 Recetas pausadas

`alternarReceta` añade o quita un recurso de `recetasPausadas`; la pausa vale para **todos** los talleres de la plaza que fabriquen ese recurso (incluida la Gran Fundición).

### 5.5 Almacén lleno

`pausadoPorAlmacenLleno` se pone a `true` en un edificio cuando su producción de este tick quedó limitada por el espacio del almacén y a `false` cuando no. Un **extractor** o una **receta** pausados no gastan
yacimiento ni insumos por lo que no cabe (§5.2–5.3); la **Granja** y la **Leñera** (que no gastan nada) simplemente no suman lo que no cabe.

## 6. Construcción

### 6.1 Ciclo de una obra y orden dentro de `avanzarConstruccion`

Estados: `en_cola` (pagado, esperando cuadrilla) → `en_construccion` (con `completaEn`) → `activo`. **El pago ocurre al encolar**, no al empezar.

```
Paso 1 — para cada edificio, en orden:
    en_construccion:  si completaEn existe e instante < completaEn → sigue.
                      si no (incl. completaEn ausente) → completa: evento construccion.edificio_completado; edificiosCompletados += 1;
                      capacidad de Almacén/Granero (+, nivel 1 si es nuevo; el que conserve si es reconstrucción);
                      si es 'mercado': nacen los puestos del nivel 1 (§6.6);   resultado: activo, completaEn y danado borrados.
    en_cola:          se trata en el Paso 2.
    activo:           produce (§5.2).
Paso 2 — arranque de obra:
    cupo = max(0, maximoEnConstruccionSimultanea − cuadrillasOcupadas(edificios AL EMPEZAR EL TICK))
    para cada en_cola ordenado por prioridad desc (estable):
        si cupo ≤ 0 y no es 'danado' → sigue en cola
        si es 'danado': cobra ceil(costo × fraccionCosteReconstruccion) por recurso; si no hay → espera; factorDanado = fraccionCosteReconstruccion
        evento construccion.iniciada;   estado = en_construccion;   completaEn = instante + max(1, round(tiempoConstruccionMinutos × factorTiempoConstruccion × factorDanado)) min
        si no es 'danado': cupo −= 1
Paso 3 — reserva = reservaDinamicaConstruccion(edificios ya actualizados, almacén actual, ración de guarnición) + reservaManual   (§6.3). Se calcula UNA vez y no se rehace.
Paso 4 — recetas (§5.3), con esa reserva.
Paso 5 — mejoras de nivel interno (§7), con esa reserva.
Paso 6 — radioPotencial = min( radioMaximoPorNivel[nivel alcanzado], radioPotencial + crecimientoPorEdificioCompletado × obras terminadas en el Paso 1 )
Paso 7 — si no autoConstruccionPausada: evaluarNecesidades (§6.2) → nuevos proyectos pagados y encolados; evento construccion.necesidad_detectada por cada uno.
Paso 8 — avanzarObraDeRecintos (Murallas.md §5), con la misma reserva.
Resultado: edificios = [edificiosBase, ...nuevosProyectos]   (el orden del array es el historial de crecimiento: NO se reordena, ver Trazado_Urbano.md)
```

> **En Rust (2026-10-22) los pasos 1 y 2 ya no son un bloque del paso de minuto**, y el bloque de arriba es el del TypeScript. Completar las obras (Paso 1) y terminar mejoras (Paso 5, primera mitad) y ascenso ocurren en su instante exacto con su cita (§1); el Paso 2 se ejecuta al fin de una obra o mejora (con la cuadrilla que libera), al encargar una obra a mano y en el paso de minuto (las comprometidas por la auto-construcción y las reconstrucciones), y **mide las cuadrillas en ese instante**, no «al empezar el tick» (`cupo = max(0, maximoEnConstruccionSimultanea − cuadrillasOcupadas ahora)`). Los pasos 3 a 8 siguen en el paso de minuto en ese orden.

Consecuencias de orden que conviene saber (TypeScript):

- Un proyecto comprometido en el tick *t* arranca como pronto en *t+1*; una obra de *m* minutos que arranca en *t+1* se completa en el primer tick con `instante ≥ completaEn` (= *t+1+m*).
- El Paso 2 mide las cuadrillas con el estado **de principio de tick**: una obra que se completa en el Paso 1 no libera hueco hasta el tick siguiente. En cambio las mejoras (Paso 5) ven el estado ya actualizado. *(En Rust no: el hueco se libera en el instante del fin, §1.)*
- `cuadrillasOcupadas` = edificios `en_construccion` **no `danado`** + edificios con `mejora` en curso.
- El radio crece también por **reconstrucciones** terminadas, y estas también dan experiencia de Facción: `edificiosCompletados` no distingue.
- Las **mejoras** que terminan no suman al radio ni a la experiencia.

### 6.2 Auto-construcción (`evaluarNecesidades`)

Cada candidato recibe `score = base de su banda + urgencia(0–100, recortada) + bonus` y se compromete en orden de score descendente mientras haya cupo y fondos. Las bandas no se solapan
(`SCORE_BANDAS`): supervivencia 10000 > extractorBase 5000 > crecimiento 1000 > manual 900 > transformación 500.

**Reglas generales de elegibilidad de cualquier candidato:**
- `puedePagar(tipo)`: la Facción tiene la tecnología que pide el tipo (`requiereTecnologia`) **y** `puedeIniciarConstruccion` sobre el almacén **entero** del tick (se pregunta *antes* de buscar sitio porque la colocación es lo más caro del tick; es conservador: solo descarta lo que el commit iba a rechazar igualmente).
- `proyectoEnCurso(tipo)`: ya hay un edificio de ese tipo que no es `activo`, o ya se propuso uno en esta pasada (evita que dos ramas elijan el mismo bosque o nodo).
- `puedeIniciarConstruccion(almacén, coste, tipo, reserva)`: `tieneRecursos` y, para cada recurso del coste con `reserva > 0`: `disponible − coste ≥ reserva`. **Exentos**: la Granja ignora la reserva de trigo y la Leñera la de madera
  (`RECURSO_PROPIO`), porque son la única forma de recuperarlos.
- Los ids nuevos son `edificio-<id asentamiento>-<contador>`, con el contador empezando en `edificios.length` y saltando los ya usados; **cada candidato propuesto consume un número**, se comprometa o no.

**Candidatos, en el orden en que se proponen** (el orden de propuesta desempata scores iguales: el sort es estable):

| # | Candidato | Condición | Score |
|---|---|---|---|
| 1 | **Granja** | `(granjas activas = 0 ∨ enDeficitProyectado)` ∧ ¬`hayMejoraGranjaDisponible` ∧ `granjasPendientes < límite` ∧ `puedePagar` ∧ hay sitio | supervivencia + (`granjas activas = 0` ? 100 : `deficitRatio × 100`) |
| 2 | **Leñera** | leñeras **activas** `< EXTRACCION_MAXIMOS.porTipo` ∧ ¬`proyectoEnCurso` ∧ `puedePagar` ∧ hay bosque con hueco en la zona (`bosqueParaLenera`) ∧ hay sitio local | supervivencia + (0 activas → 100; `madera < reserva.madera` → 90; si no 30) |
| 3 | **Extractores**: cantera, corral, minaCobre, mina, minaEstano, minaHierro (en este orden) | `necesitaNuevoExtractor` ∧ ¬`proyectoEnCurso` ∧ `puedePagar` ∧ hay nodo libre y, si es interno (corral), sitio local | extractorBase + (alguna fuente viva ? 40 : 100) + `min(ticksSinCupo × bonusPorMinutoStarved, bonusMaximo)` |
| 4 | **Vivienda** | (`capacidad de pesants = 0` ∨ `ocupaciónMáx ≥ umbralViviendaOcupada`) ∧ ¬pendiente ∧ ¬`alcanzoTopeDeViviendas` ∧ `puedePagar` | crecimiento + `ocupaciónMáx × 100` |
| 5 | **Granero** | trigo `cantidad/capacidad ≥ umbralAlmacenAmpliacion` ∧ no hay granero activo ∧ ¬pendiente ∧ `puedePagar` | crecimiento + `ocupaciónTrigo × 100` |
| 6 | **Almacén** | mayor ocupación de cualquier recurso con capacidad > 0 `≥ umbralAlmacenAmpliacion` ∧ ¬pendiente ∧ ¬`alcanzoTopeDeAlmacenes` ∧ `puedePagar` | crecimiento + `ocupaciónMáx × 100` |
| 7 | **Curtiduría / Armería / Fundición** (a lo sumo **una** en vuelo de las tres, y solo **una** propuesta por pasada: el primero de la lista que valga) | `nivelActual ≥ requisitoNivelBase('fundicion')` (= 2) ∧ ninguna de las tres pendiente; para cada tipo en ese orden: ¬`alcanzoTopeDeTransformacion` ∧ `tieneInsumoDeArranque` ∧ `puedePagar` ∧ hay sitio | transformación (sin urgencia) |
| 8 | **Carpintería** | `nivelActual ≥ requisitoNivelAsentamientoConstruccion` (2) ∧ ¬tope ∧ ¬pendiente ∧ `puedePagar` ∧ hay sitio | transformación |

Definiciones auxiliares:

- **Déficit de trigo**: `produccionActual = Σ por granja activa de produccionTrigoDeGranja(nivel) × fertilidadZona × ratioManoObra × factorProduccionTrigo`;
  `consumoActual = consumoComidaPoblacion + ración de la guarnición`; `enDeficitTrigo = produccionActual < consumoActual`.
  El **objetivo proyectado** usa una población `max(real, capacidad de Vivienda)` por clase: `enDeficitProyectado = produccionActual < consumoObjetivo`; `deficitRatio = (consumoObjetivo − produccionActual)/consumoObjetivo`.
- **`granjasPendientes`** = granjas con estado ≠ activo; **`límite`** = `maximoGranjasPendientesEnDeficit` (3) si `enDeficitTrigo`, si no 1.
- **`hayMejoraGranjaDisponible`**: queda al menos una cuadrilla libre *tras* reservar la de construir (`cuadrillasOcupadas < maximoEnConstruccionSimultanea − 1`) ∧ alguna granja activa, sin mejora en curso, tiene siguiente nivel y su `costoMejora` es pagable (con reserva).
  Si es verdadera, **se prefiere mejorar a construir otra** (mejorar no diluye `ratioManoObra`).
- **`necesitaNuevoExtractor`**: de los edificios de ese tipo (en cualquier estado) cuenta los que tienen fuente viva (`stock > 0`): si ninguno → sí; si no → `vivos < EXTRACCION_MAXIMOS.porTipo`.
- **Sitio del extractor mineral/cantera/corral**: el nodo de ese tipo **más cercano a la posición del asentamiento** (empates por orden de generación del mundo) con stock, dentro del polígono de zona, no reclamado y cuyo terreno no sea `cima`. Cantera y minas se plantan en la posición del nodo (ámbito `mapa`); el Corral conserva el `fuenteId` pero se coloca en la ciudad.
- **`ocupaciónMáx` de Vivienda**: `max(pesants/capacidadPesants, artesanos/capacidadArtesanos)`; una capacidad ≤ 0 cuenta como ocupación 1.
- **`tieneInsumoDeArranque(tipo)`**: alguna receta del **nivel 1** del tipo consume un insumo con `cantidad > 0` en el almacén (sin recetas → verdadero). La Armería casi siempre pasa (Arma de Madera pide solo madera).
- **Topes de transformación** (`MAXIMO_TRANSFORMACION_POR_NIVEL[nivelActual]`: 2→3, 3→5, 4→5, 5→5; nivel 1 sin tope): cuentan los edificios de ese tipo en cualquier estado.
- **Anclas**: para los tipos con categoría de ancla (vivienda y transformación), antes de pedir sitio se llama a `asegurarAnclaPara`, que puede **crear un ancla nueva** (un edificio gratis y `activo`: plaza, pozo, parque, plaza de armas, patio de gremios; ver
  [`Trazado_Urbano.md`](Trazado_Urbano.md)) si `nivelActual ≥ 2` (`NIVEL_GATE_ANCLAS`) y solo si luego cabe algún satélite del tipo pedido. **El ancla se conserva aunque el candidato no llegue a comprometerse.**

**Commit único:**

```
cupo = max(0, maximoEnCola − nº de edificios en_cola)
para cada candidato por score desc (estable):
    si cupo ≤ 0 → fin
    si es de transformación y alcanzoTopeDeTransformacion(contando los ya comprometidos aquí) → saltar
    si !puedeIniciarConstruccion(almacén actual, coste, tipo, reserva) → saltar
    si pisaCalleComprometida (revalidar el sitio contra la ciudad tal como queda realmente ordenada) → saltar
    pagar (descontar el coste COMPLETO del catálogo);  nuevo edificio en_cola con prioridad = score;  registrarReclamo(fuente);  cupo −= 1
extractoresTicksSinCupo[tipo] = (comprometido ? 0 : +1)   solo para los extractores PROPUESTOS este tick
```

Un candidato que no se compromete **no deja rastro** (no hay placeholders atascados): se reevalúa el tick siguiente. La fuente de un extractor solo queda tomada cuando se paga.

### 6.3 Reserva

`reservaDinamicaConstruccion(asentamiento, capital, ración de guarnición)`:

```
para cada recurso r del coste de mantenimiento actual:   reserva[r] = costeMantenimiento[r] × RESERVA_CONSTRUCCION.horizonteMinutosMantenimiento
reserva.trigo = (consumoComidaPoblacion + ración de guarnición [+ extra de reclutamiento]) × RESERVA_CONSTRUCCION.horizonteMinutosComida
(+ reservaManual[r] del Tesorero, SOLO en el camino automático: recetas, mejoras automáticas, evaluarNecesidades, obra de recintos)
```

La construcción **manual** y la mejora manual usan la reserva dinámica **sin** la manual: son decisiones informadas y están exentas.

### 6.4 Construcción manual (`anadirEdificioManualmente`)

La pide un residente presente con el cargo `gobernador` o `maestroObras` (el motor solo exige que **alguien** tenga ese cargo; es el comando el que comprueba que el actor es ese titular). Cualquier tipo del catálogo menos el Centro Urbano y los puestos
de Mercado. Validaciones, en orden:

1. Hay alguien en ese cargo. 2. La Facción tiene la tecnología del tipo. 3. Si el tipo pide una Era (`ordenEraMinimo`, hoy solo la Maravilla: la V), el servidor la ha alcanzado (`ERAS[era].orden`). 3b. No es `centroUrbano` ni `puestoMercado`.
4. Tipos **únicos** (`EDIFICIOS_UNICOS`, §14.2): ya hay uno (de cualquier estado).
5. Tope de Almacenes; tope de Viviendas. 6. `nivelActual ≥ requisitoNivelAsentamientoConstruccion` del tipo. 7. Tope de transformación.
8. **Gran Fundición**: nivel de Facción ≥ `nivelFaccionMinimo` (6), el asentamiento es la **capital**, y la Facción no tiene ya otra (ninguno de sus asentamientos).
9. `en_cola < maximoEnCola`. 10. `asegurarAnclaPara` y búsqueda de sitio (si no hay: «No hay sitio disponible dentro de la zona de influencia»).
11. `puedeIniciarConstruccion` con la reserva dinámica (sin la manual).

Paga el coste completo, crea el edificio `en_cola` con id `edificio-<id>-manual-<n>` (n = id de contexto del comando) y `prioridad = SCORE_BANDAS.manual` (900). No comprueba `tieneInsumoDeArranque`, ni `necesitaNuevoExtractor`, ni
`autoConstruccionPausada`, ni `hayProyectoPendiente` de los tipos no únicos (se pueden encolar varias Viviendas hasta el tope de cola).

`quitarDeCola(cargo, edificioId)`: solo un edificio `en_cola`; **devuelve el coste completo del catálogo** más el `costoMejora` de cada nivel interno 2..`nivelInterno`; **si el edificio está `danado` devuelve solo `OCUPACION.fraccionCosteReconstruccion` (la mitad) de cada recurso, redondeando hacia abajo**. Se añade con `agregarRecurso` (el tope de capacidad
puede perder parte del reembolso). `moverEnCola(arriba|abajo)`: intercambia la `prioridad` con la del vecino de la lista ordenada por prioridad desc; en el extremo no hace nada.

### 6.5 Topes por nivel

- **Almacenes**: `NECESIDADES.maximoAlmacenesPorNivel[nivelActual]`.
- **Viviendas** (`maximoViviendasPorNivel(n)`): si existe el requisito del nivel `n+1`: `max( ceil(pesants_{n+1}/capacidadPesants), ceil(artesanos_{n+1}/capacidadArtesanos) ) + MARGEN_VIVIENDAS (2)`; en el nivel máximo, `ceil(techoPoblacion[n]/capacidadPesants)`. **El margen de 2 se decidió el 2026-10-18**: con el cupo justo (40 Viviendas × 10 = 400 artesanos para un requisito de 400) la población logística se acerca al cupo sin tocarlo y el requisito no se cumpliría nunca; con el margen el cupo es 420.
  Cuenta Viviendas de cualquier estado.

### 6.6 Puestos del Mercado

El Mercado es una **zona**: al completarse crea sus puestos del nivel 1 y al mejorar a cada nivel los de ese nivel, **gratis, ya `activo`**, sin pasar por la cola. Para cada forma de `MERCADO_PUESTOS_POR_NIVEL[nivel]` (en ese orden) busca sitio con
`sitioEnTrazado(..., 'puestoMercado', forma)`; si no hay, **se salta en silencio**. El puesto lleva en `nivelInterno` su forma (no es progresión) y `PUESTO_MERCADO_FORMA[forma]` es su huella. Los puestos son edificios `activo`
a todos los efectos (cuentan en el mantenimiento, §10.1).

## 7. Mejoras de nivel interno

Tipos que mejoran: `EDIFICIOS_CON_NIVELES` (§14.2).

**Elegibilidad** (`elegibleParaMejora`): el edificio está `activo`, es de esos tipos, no tiene ya una `mejora`, existe el nivel siguiente y se cumple: `nivelActual del asentamiento ≥ siguiente.requisitoNivelAsentamiento`;
la Facción tiene `siguiente.requiereTecnologia`; si hay `requiereEdificio`, **algún** edificio activo de ese tipo tiene `nivelInterno ≥ requiereEdificioNivel` (por defecto 1).

**Duración** (`minutosDeMejora`): `max(1, round( (niveles[siguiente].obraMinutos ?? tiempoConstruccionMinutos × MEJORA_EDIFICIO.multiplicadorPorNivel^(siguiente − 1)) × factorTiempoConstruccion ))`.
O sea, la obra base ×2 para el nivel 2, ×4 para el 3, ×8 para el 4, salvo los `obraMinutos` propios (Taberna, Palacio).

**Cada tick** (`avanzarMejoras`), dos pasos *(en Rust, el primero ocurre en el instante de fin de la mejora y el segundo en el paso de minuto)*:

1. **Terminar**: para cada edificio con `mejora` y `instante ≥ completaEn` → `aplicarMejoraTerminada`: si la huella cambia de tamaño (hoy solo la Granja: 2×2, 2×3, 4×3, 6×6) se muda a un hueco con `reubicarPorTamano`;
   **si no hay hueco, la mejora espera** (conserva `mejora` y su cuadrilla) y se reintenta el tick siguiente *(en Rust: el paso de minuto siguiente)*. Al terminar sube `nivelInterno`, borra `mejora`, y: Mercado → crea los puestos del nivel nuevo; Granero → `ampliarCapacidad(trigo, capacidadTrigo[nuevo] − capacidadTrigo[viejo])`.
   Evento `construccion.mejora_completada`.
2. **Empezar las automáticas**: `cuadrillasLibres = maximoEnConstruccionSimultanea − cuadrillasOcupadas − 1` (**siempre deja una libre para construir**). Recorre los edificios en orden del array mientras `cuadrillasLibres > 0`:
   si es elegible y `puedeIniciarConstruccion(costoMejora)` (con la reserva), **paga**, fija `mejora = {nivelObjetivo, completaEn: instante + minutos}`, `cuadrillasLibres −= 1`, evento `construccion.mejora_iniciada`. Se mejora todo lo que se pueda pagar, de cualquier tipo.

**Mejora manual** (`mejorarEdificioManualmente`, Gobernador o Maestro de Obras): el edificio está activo; tecnología; `estadoMejoraEdificio` (elegible + pagable con la reserva dinámica); **debe quedar una cuadrilla** (`cuadrillasOcupadas < maximoEnConstruccionSimultanea`, puede tomar la última);
si la huella cambia, ya debe haber hueco para reubicar. Paga y arranca la mejora; el nivel sube al terminar.

Mientras se mejora el edificio sigue activo y produciendo con su nivel actual; la mejora ocupa cuadrilla.

## 8. Reconstrucción tras un saqueo

La conquista (Combate.md) baja un porcentaje de edificios a `estado: 'en_cola'` con `danado: true` (conservando `nivelInterno`) y les quita su capacidad de almacén. En el Paso 2 (§6.1) un `danado`: **no ocupa cuadrilla ni espera turno**, cobra
`ceil(coste de catálogo × fraccionCosteReconstruccion)` **al arrancar** (si no hay con qué, espera), y tarda `tiempoConstruccionMinutos × factorTiempoConstruccion × fraccionCosteReconstruccion`. Al terminar vuelve a `activo`, recupera su capacidad (con el nivel que conservó) y `danado` se borra.
Quitar de la cola un `danado` devuelve la mitad del coste y de las mejoras (§6.4).

## 9. Población

### 9.1 Crecimiento (`crecerPoblacion`)

Las tres clases usan la misma forma: `esperado = actual × tasa`, con `tasa = tasaBase × comidaFactor × espacioFactor × estabilidad(1) × felicidad`, y **población siempre entera** (decisión del 2026-10-22, que sustituye a la «población continua» del 2026-10-13): cada clase guarda **habitantes enteros y un resto oculto** en milésimas de habitante; **no hay tirada**. En cada paso de un minuto el incremento `actual × tasa` (con `actual` = habitantes **más su resto**, que es la población continua de siempre, así que el ritmo de crecimiento es el mismo) se suma al resto, y **entra una persona entera cada vez que el resto llega a 1**. Requisitos de nivel, mano de obra, recaudación y consumo leen los habitantes enteros (el resto no lo lee nadie).

```
comidaFactor = factorCrecimientoMinimo + (1 − factorCrecimientoMinimo) × nutricion/100
felicidad    = factorCrecimientoPoblacion(políticas) × (ocupado ? OCUPACION.factorCrecimiento : 1) × (1 + AEDAS.residentes.bonoFelicidad × aedasResidentes)

Pesants:   espacio = capacidadPesants ≤ 0 ? 0 : clamp(1 − pesants/capacidadPesants, 0, 1)      (capacidad = nº de Viviendas activas × 15)
           tasa = pesants.tasaCrecimientoBase × comida × espacio × felicidad
Artesanos: solo si hay ALGÚN edificio de transformación activo (fundicion, curtiduria, armeria, carpinteria) y espacio > 0 (capacidad = Viviendas activas × 10):
           si artesanos = 0 → +1 sin tirar el dado (evento poblacion.primeros_artesanos);  si no, tasa = artesanos.tasaCrecimientoBase × comida × espacio × felicidad
Nobleza:   solo si casasCompradas.length ≥ nobleza.minCiudadanos ∧ hay Palacio activo ∧ espacio > 0 (capacidad = capacidadNobles del nivel del Palacio, 80/240/400);
           si nobleza = 0 → +1 sin dado (evento poblacion.primeros_nobles);  si no, tasa = nobleza.tasaCrecimientoBase × comida × espacio × felicidad × factorCrecimientoNobleza × (1 + AEDAS.residentes.bonoNobleza × aedasResidentes)
Techo:     techo = techoPoblacion[nivel ALCANZADO];  espacioBajoTecho = max(0, techo − población total)
           crecimientoBruto = suma de las tres;  factorTecho = (bruto > espacioBajoTecho ∧ bruto > 0) ? espacioBajoTecho/bruto : 1
           resto de cada clase += incremento × factorTecho;  habitantes += resto entero;  resto = lo que sobra (0 ≤ resto < 1)       (nunca purga población ya asentada)
```
Crecen las tres clases **solo** si la plaza puede crecer (§9.2: nutrición mayor que 0 y fuera de la recuperación del hambre). Los «+1 sin dado» de artesanos y nobleza son una persona entera.

**El crecimiento no consume el generador aleatorio.** Los eventos «primeros artesanos» y «primeros nobles» (+1 sin dado) se disparan al cumplirse la condición.

### 9.2 Comida y hambruna (`avanzarNutricionPoblacion`)

```
consumo = total × consumoComidaPorHabitante × factorConsumoComida(políticas)          (por tick; la clase no importa)
factorSuministro = consumo > 0 ? min(1, trigo/consumo) : 1
trigo = max(0, trigo − consumo)                                                       (se consume lo que haya)
nutricion = factorSuministro ≥ 1 ? min(100, n + regeneracionPorMinuto) : max(0, n − degradacionSinComida × (1 − factorSuministro))
si nutricion ≤ umbralMuertePorHambre (0):
    afectados = pesants + artesanos
    comunes   = afectados > 0 ? min(afectados, ceil(afectados × fraccionMuertePorMinutoHambre)) : 0
    muertesPesants = round(comunes × pesants/afectados);  muertesArtesanos = comunes − muertesPesants
    muertesNobleza = nobleza > 0 ? ceil(nobleza × fraccionMuertePorMinutoHambreNobleza) : 0
    evento poblacion.hambruna_muerte { muertes, muertesPesants, muertesArtesanos, muertesNobleza } (si muertes > 0)
```

La nutrición **no** degrada el medidor de mantenimiento: son dos medidores independientes.

**Mortalidad entera y simétrica (2026-10-22).** El bloque anterior es el del TypeScript. En Rust, mientras `nutricion = 0`, cada minuto muere la fracción `fraccionMuertePorMinutoHambre` de los pesants y de los artesanos enteros (5 %) y `fraccionMuertePorMinutoHambreNobleza` de la nobleza entera (1 %), **sin redondear hacia arriba**: la fracción se suma al resto de la clase con el signo cambiado y **muere una persona entera cuando el resto llega a −1**. Con 10 habitantes y el 5 % por minuto muere una persona cada dos minutos. Un resto del signo contrario (de antes de que cambiara el régimen) se descarta. El suceso `poblacion.hambruna` lleva las **personas** muertas ese minuto por clase (`pesants`, `artesanos`, `nobleza`) y solo se emite si murió alguien. La cobertura de un minuto con déficit parcial es `stock > 0 ? 1 : min(1, producción/consumo)`.

**Hambre: D7 y D8 (aprobadas por el autor el 2026-10-21).**
- **D7.** Con nutrición 0 **no crece ninguna clase** (ni aparecen los primeros artesanos o nobles): solo hay mortalidad. El factor de crecimiento `factorCrecimientoMinimo` (0,2) solo se aplica con nutrición positiva (mayor que 0 y fuera de la recuperación).
- **D8.** Al tocar nutrición 0 la plaza entra en **recuperación** y **no vuelve a crecer hasta recuperar la nutrición 100** (20 minutos con comida completa: `regeneracionPorMinuto` = 5). La mortalidad es solo mientras la nutrición esté en 0; **durante la recuperación con nutrición positiva la población queda quieta**. Si la nutrición vuelve a 0, vuelve la mortalidad y sigue pendiente llegar a 100. Una plaza que **no ha pasado hambre** crece con la nutrición intermedia como siempre (`comidaFactor` entre 0,2 y 1). La marca de recuperación (`recuperando`) se guarda en el estado (foto y hash); no se deduce de la nutrición.

### 9.3 Recaudación de oro

`recaudacionOro = (pesants × tasaPesants + artesanos × tasaArtesanos + nobleza × tasaNobleza) × factorRecaudacion(políticas) × (ocupado ? OCUPACION.factorRecaudacion : 1)`. No depende de la distancia a la capital ni del nivel. Se suma al almacén con tope de capacidad
(§1, paso 7).

## 10. Mantenimiento y nivel operativo

### 10.1 Coste (`calcularCostoMantenimiento`)

```
nivel     = nivelActual
edificios = Σ sobre los edificios con estado 'activo' de  pesoVivienda (0,5) si es vivienda · pesoPiezaGratuita (0,5) si es de `piezasGratuitas` (puestoMercado, plaza, plazaDeArmas, patioDeGremios, pozo, parque) · 1 el resto, incluido el centroUrbano
escala    = (1 + edificios/edificiosReferencia) × (1 + distancia a la capital/escalaDistancia)           (distancia 0 si no hay capital)
coste.madera = costoBase.madera × escala
si nivel ≥ nivelParaPiedra: coste.piedra = piedraBase × escala
si nivel ≥ nivelParaOro:    coste.oro    = oroBase × escala
+ upkeepDeRecintos(recintos)   (Murallas.md §9.4: las celdas en pie de cada anillo a su nivel, sumado sin escala)
```

El **trigo no forma parte** del coste (se descuenta una sola vez, donde se consume). La **capital** (`encontrarCapital`) es la designada (`capitalDeFaccionId == faccionId` y con un Palacio activo, `capitalDesignada`) y, si no hay,
el asentamiento vivo más antiguo (`fundadoEn` menor) de la Facción.

### 10.2 Pago y medidor (`avanzarMantenimiento`)

```
si transcurrido(fundadoEn) < graciaMinutos  ó  estaOcupado → nada (ni cobra ni regenera)
costo = calcularCostoMantenimiento;  cubierta = media aritmética sobre los recursos del coste de min(1, almacén/coste)     (coste vacío → 1)
si cubierta ≥ 1: descontar el coste entero         si no: de cada recurso se descuenta min(disponible, coste)  (se paga todo lo que hay)

cubierta ≥ 1 (sano):
    racha += 1;  medidor = min(100, medidor + regeneracionSiPagoCompleto)
    si racha ≥ minutosSanosParaRecuperarNivel ∧ nivelActual < nivel:  nivelActual += 1, racha = 0, evento mantenimiento.recuperado
cubierta < 1 (déficit):
    racha = 0;  medidor = max(0, medidor − degradacionPorDeficitTotal × (1 − cubierta))
    medidor ≤ 0:
        nivelActual > 1 → nivelActual −= 1, medidor = medidorInicial, evento mantenimiento.colapsado
        nivelActual = 1 → RUINAS: evento asentamiento.ruinas { faccionId, nombre, razon, faltantes[], fundadoEn, duro }, destruido = true
    si no: evento mantenimiento.deficit (si el medidor bajó)
```

`racha` sigue creciendo sin tope mientras `nivelActual == nivel`. Al recuperar un nivel el medidor también regenera ese tick (la regeneración se aplica antes de subir el nivel) y la racha vuelve a 0.

### 10.3 Reserva y lo que evita

La reserva de §6.3 existe para que la obra, los talleres y las mejoras nunca dejen sin pagar el mantenimiento ni la comida de los próximos `horizonteMinutos*` ticks.

### 10.4 Ruinas

Cuando `avanzarMantenimiento` devuelve `destruido`, el asentamiento desaparece del estado y el tick resuelve todo lo que colgaba de él: sus residentes pasan a la plaza más cercana de su Facción (o al campamento de mercenarios más cercano), su campamento de
bandidos se dispersa, sus trueques se cancelan sin penalizar y las caravanas que iban o venían vuelven o se pierden (`engine/ruina.ts`, ver [`Comercio_Caravanas.md`](Comercio_Caravanas.md) §10). Si era la última plaza de la Facción, queda derrotada sin ganador.

## 11. Nivel de asentamiento

### 11.1 Gates (`evaluarGatesDeNivel(a, nivelObjetivo)`)

```
poblacion = pesants ≥ req.pesants ∧ artesanos ≥ req.artesanos           (la nobleza no cuenta; la población es entera desde el 2026-10-22, así que ya no hay que redondear: antes se leía redondeada al entero más cercano)
construidos = nº de tipos de req.edificios con algún edificio ACTIVO cuyo nivelInterno ≥ req.nivelInternoMinimo[tipo] (por defecto 1)
cumpleRecinto = req.recintoCompletoNivelMinimo indefinido ∨ algún recinto con nivel ≥ ese mínimo e integridad ≥ 1      ; una mejora o una ampliación en obra no lo quita: cuenta el anillo en pie con su nivel de antes (Murallas.md §9.3)
edificios = construidos ≥ (req.edificiosMinimo ?? req.edificios.length) ∧ cumpleRecinto
```

Devuelve `null` si el nivel no tiene requisito. `calcularNivelAsentamiento` encadena los gates desde `nivel` y dice hasta dónde se cumplirían (la usan los bots para decidir si piden la subida; **no sube nada**).

### 11.2 Pedir la subida (`evaluarAscenso` / `iniciarAscenso`)

Solo el **Gobernador** (residente y presente). Bloqueos posibles, **todos a la vez**: `nivel_maximo` (también si `nivelObjetivo > ASCENSO_ASENTAMIENTO.nivelTechoProvisional`), `ascenso_en_curso`, `falta_poblacion`, `faltan_edificios`, `sin_cupo_de_faccion`,
`recursos_insuficientes` (el coste de `porNivelObjetivo[objetivo]` no está entero en el almacén), `insolvente`.

- **Cupo de la Facción** (`cupoLibreParaNivel`): solo existe para los niveles objetivo 2 y 3. Ocupan plaza los asentamientos de la Facción con `nivelActual == objetivo` **o** con una obra de ascenso hacia ese nivel; hay cupo si
  `ocupadas < calcularCupoNivel(nivel de Facción, objetivo)` (curva `CUPO_NIVEL_ASENTAMIENTO.maxNivel2/3`, índice = `clamp(nivel de Facción, 1, 10) − 1`). Los niveles 4 y 5 no tienen cupo (siempre libre).
- **Solvencia**: para cada recurso del coste de mantenimiento **en el nivel objetivo** (`calcularCostoMantenimiento` con `nivel = nivelActual = objetivo`), el ingreso bruto por minuto —`Σ produccionPorMinuto` de sus edificios (con topes por stock del nodo y tecnologías)
  más, para el oro, `recaudacionOro`— debe ser ≥ ese coste. Es bruto a propósito: no descuenta lo que gasten recetas o tropas. Lo que llega por **acarreo** (§14, pueblos extractores) cuenta como la **media por minuto de lo que ingresó de verdad
  en el almacén de la plaza en las últimas 24 h** (decidido por el autor el 2026-10-19; antes, la producción nominal de los pueblos con carro): con el almacén lleno un pueblo no hace solvente a nadie.
- Si todo va bien: cobra el coste entero del almacén y fija `ascenso = { nivelObjetivo, iniciadoEn, completaEn: instante + obraMinutos }` (evento `asentamiento.ascenso_iniciado`).

### 11.3 Durante la obra

El asentamiento funciona con normalidad. Si lo conquistan, la obra se pierde sin devolución (Combate.md).

### 11.4 Terminar (`avanzarAscenso`)

Si `instante ≥ completaEn`: `nivel = nivelObjetivo` y, **solo si `nivelActual == nivel` antes de subir**, también `nivelActual = nivelObjetivo` (un asentamiento degradado no se «cura» de golpe); borra `ascenso`; evento `asentamiento.nivel_subio`. No vuelve a mirar gates, cupo ni solvencia.

## 12. Políticas que tocan la economía

`activarPolitica(cargo, politicaId)`: la política existe; si el cargo no es el Gobernador, la política es de su pool (`def.cargo == cargo`); el cargo está ocupado; no está ya activa para ese cargo; hay slot libre
(`slotsDisponibles`: Gobernador `min(maximo, base + floor(nivelFaccion / nivelFaccionPorSlotExtraGobernador)) + (Sala del Consejo activa ? slotSalaConsejo : 0)`; el resto, `base`). Dura `duracionMinutosPorDefecto`, no se cancela,
y `avanzarPoliticas` la borra cuando `instante ≥ expiraEn` (evento `politica.expirada`).

Los factores se **multiplican** entre las políticas activas del asentamiento (`productoFactor`; 1 si no hay ninguna) salvo `cupoCaravanaExtra` y `cupoGuarnicionExtra`, que se **suman**. Los que lee este documento: `factorConsumoComida` (Racionamiento ×0,8),
`factorCrecimientoNobleza`, `factorTiempoConstruccion` (Vía Rápida ×0,75; afecta a obras **y** mejoras al arrancar), `factorProduccionTrigo` (Edicto de Cosecha ×1,5), `factorRecaudacion` y `factorCrecimientoPoblacion` (Presión Fiscal ×1,6 / ×0,8),
`factorTiempoMuralla`, `factorProduccionTalleres`. Los demás (comisiones, caravanas, reclutamiento, guarnición, perfil de trazado) pertenecen a sus sistemas.

## 13. Comandos

| Comando | Quién (residente y presente) | Qué |
|---|---|---|
| `anadirEdificioManualmente { asentamientoId, cargo, tipo }` | `gobernador` o `maestroObras` | §6.4. Evento `construccion.encolada` |
| `quitarDeCola { asentamientoId, cargo, edificioId }` | ídem | §6.4. `construccion.desencolada` |
| `moverEnCola { …, direccion }` | ídem | §6.4. `construccion.cola_reordenada` |
| `mejorarEdificioAhora { …, edificioId }` | ídem | §7. `construccion.mejora_forzada` |
| `alternarAutoConstruccion { asentamientoId, pausada }` | gobernador o maestroObras | Idempotente. `construccion.auto_alternada` |
| `alternarReceta { asentamientoId, recurso, pausada }` | gobernador o maestroObras | Idempotente. `construccion.receta_alternada` |
| `calibrarReservaManual { asentamientoId, recurso, valor }` | `tesorero` (y debe haber uno) | `reservaManual[recurso] = clamp(round(valor), 0, NECESIDADES.reservaManualMaxima)`. Sin evento |
| `renombrarAsentamiento { asentamientoId, nombre }` | cualquier residente | Nombre vacío = volver a mostrar el id. `asentamiento.renombrado` |
| `solicitarAscenso { asentamientoId }` | `gobernador` | §11.2. Error `ascenso.invalido` con todos los bloqueos |

La pausa de la auto-construcción detiene solo la **detección de necesidades nuevas** (§6.2); lo ya pagado, las mejoras automáticas, las recetas y la obra de recintos siguen.

## 14. Parámetros

### 14.1 Catálogo de edificios

Cada entrada: `costo` (recursos que se pagan al encolar), `tiempoConstruccionMinutos`, `requisitoNivelAsentamientoConstruccion` (nivel **operativo** mínimo para construirlo, ausente = 0), `requiereTecnologia`, las cifras propias del tipo
(`capacidadPesants/Artesanos`, `produccionBase*`, `trabajadoresRequeridos`, `capacidadPorRecursoAdicional`, `nivelFaccionMinimo`) y, si mejora, `niveles` con `costoMejora`, `requisitoNivelAsentamiento`, `requiereEdificio(+Nivel)`, `requiereTecnologia`,
`obraMinutos`, `recetas`, `capacidadNobles`, `cupoCaravanas`, `cupoCarros`, `cupoMiradas`, `capacidadTrigo`, `produccionBaseTrigo`, `tamano`. Los edificios sin coste (`centroUrbano`, `puestoMercado`, `plaza`, `plazaDeArmas`, `patioDeGremios`, `pozo`, `parque`) nunca pasan por la cola: nacen gratis
(fundación, mejora del Mercado, ancla).

<!-- parametros:EDIFICIO_CATALOGO -->
```json
{
  "centroUrbano": {
    "costo": {},
    "tiempoConstruccionMinutos": 0
  },
  "vivienda": {
    "costo": {
      "madera": 10
    },
    "tiempoConstruccionMinutos": 240,
    "capacidadPesants": 15,
    "capacidadArtesanos": 10
  },
  "granja": {
    "costo": {
      "madera": 30
    },
    "tiempoConstruccionMinutos": 120,
    "produccionBaseTrigo": 60,
    "trabajadoresRequeridos": 4,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 4,
        "recetas": [],
        "produccionBaseTrigo": 60,
        "tamano": {
          "ancho": 2,
          "alto": 2
        }
      },
      "2": {
        "trabajadoresRequeridos": 4,
        "recetas": [],
        "produccionBaseTrigo": 90,
        "tamano": {
          "ancho": 2,
          "alto": 3
        },
        "costoMejora": {
          "madera": 60,
          "piedra": 20
        }
      },
      "3": {
        "trabajadoresRequeridos": 4,
        "recetas": [],
        "produccionBaseTrigo": 120,
        "tamano": {
          "ancho": 4,
          "alto": 3
        },
        "costoMejora": {
          "madera": 120,
          "piedra": 40
        }
      },
      "4": {
        "trabajadoresRequeridos": 4,
        "recetas": [],
        "produccionBaseTrigo": 180,
        "tamano": {
          "ancho": 6,
          "alto": 6
        },
        "costoMejora": {
          "madera": 240,
          "piedra": 80
        }
      }
    }
  },
  "cantera": {
    "costo": {
      "madera": 20
    },
    "tiempoConstruccionMinutos": 240,
    "produccionBasePiedra": 10,
    "trabajadoresRequeridos": 4
  },
  "lenera": {
    "costo": {
      "madera": 10
    },
    "tiempoConstruccionMinutos": 60,
    "produccionBaseMadera": 5,
    "trabajadoresRequeridos": 4
  },
  "almacen": {
    "costo": {
      "madera": 50
    },
    "tiempoConstruccionMinutos": 360,
    "capacidadPorRecursoAdicional": 300
  },
  "granero": {
    "costo": {
      "madera": 50
    },
    "tiempoConstruccionMinutos": 360,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "capacidadTrigo": 2000
      },
      "2": {
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "capacidadTrigo": 3000,
        "requisitoNivelAsentamiento": 2,
        "costoMejora": {
          "madera": 100,
          "piedra": 30
        }
      },
      "3": {
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "capacidadTrigo": 4000,
        "requisitoNivelAsentamiento": 2,
        "costoMejora": {
          "madera": 200,
          "piedra": 60
        }
      },
      "4": {
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "capacidadTrigo": 6000,
        "requisitoNivelAsentamiento": 3,
        "costoMejora": {
          "madera": 400,
          "piedra": 120
        }
      }
    }
  },
  "mina": {
    "costo": {
      "madera": 40
    },
    "tiempoConstruccionMinutos": 480,
    "produccionBaseOro": 4,
    "trabajadoresRequeridos": 6
  },
  "minaCobre": {
    "costo": {
      "madera": 30
    },
    "tiempoConstruccionMinutos": 360,
    "produccionBaseCobre": 5,
    "trabajadoresRequeridos": 8
  },
  "minaEstano": {
    "costo": {
      "madera": 50
    },
    "tiempoConstruccionMinutos": 480,
    "produccionBaseEstano": 3,
    "trabajadoresRequeridos": 8
  },
  "minaHierro": {
    "costo": {
      "madera": 30
    },
    "tiempoConstruccionMinutos": 360,
    "produccionBaseHierro": 5,
    "trabajadoresRequeridos": 8,
    "requiereTecnologia": "forja_hierro_temprana"
  },
  "corral": {
    "costo": {
      "madera": 30
    },
    "tiempoConstruccionMinutos": 240,
    "produccionBaseLivestock": 3,
    "trabajadoresRequeridos": 4
  },
  "granFundicion": {
    "costo": {
      "madera": 150,
      "piedra": 100,
      "oro": 50
    },
    "tiempoConstruccionMinutos": 2880,
    "nivelFaccionMinimo": 6,
    "requisitoNivelAsentamientoConstruccion": 4,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 12,
        "recetas": [
          {
            "produce": "lingoteCobre",
            "produccionBase": 4,
            "consumePorUnidad": {
              "chatarraCobre": 1
            }
          },
          {
            "produce": "lingoteBronce",
            "produccionBase": 4,
            "consumePorUnidad": {
              "chatarraBronce": 1
            }
          },
          {
            "produce": "lingoteHierro",
            "produccionBase": 4,
            "consumePorUnidad": {
              "chatarraHierro": 1
            }
          }
        ]
      }
    }
  },
  "fundicion": {
    "costo": {
      "madera": 80,
      "piedra": 40
    },
    "tiempoConstruccionMinutos": 720,
    "requisitoNivelAsentamientoConstruccion": 2,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 4,
        "recetas": [
          {
            "produce": "lingoteCobre",
            "produccionBase": 5,
            "consumePorUnidad": {
              "cobre": 2
            }
          }
        ]
      },
      "2": {
        "requisitoNivelAsentamiento": 2,
        "costoMejora": {
          "madera": 150,
          "piedra": 100
        },
        "trabajadoresRequeridos": 8,
        "recetas": [
          {
            "produce": "lingoteCobre",
            "produccionBase": 5,
            "consumePorUnidad": {
              "cobre": 2
            }
          },
          {
            "produce": "lingoteEstano",
            "produccionBase": 3,
            "consumePorUnidad": {
              "estano": 5
            },
            "requiereTecnologia": "aleacion_bronce"
          },
          {
            "produce": "lingoteBronce",
            "produccionBase": 1,
            "consumePorUnidad": {
              "lingoteCobre": 1.6,
              "lingoteEstano": 0.4
            },
            "requiereTecnologia": "aleacion_bronce"
          },
          {
            "produce": "lingoteHierro",
            "produccionBase": 5,
            "consumePorUnidad": {
              "hierro": 2,
              "madera": 1
            },
            "requiereTecnologia": "forja_hierro_temprana"
          }
        ]
      },
      "3": {
        "requisitoNivelAsentamiento": 4,
        "requiereTecnologia": "forja_hierro_estandarizada",
        "costoMejora": {
          "madera": 300,
          "piedra": 200
        },
        "trabajadoresRequeridos": 12,
        "recetas": [
          {
            "produce": "lingoteCobre",
            "produccionBase": 7.5,
            "consumePorUnidad": {
              "cobre": 2
            }
          },
          {
            "produce": "lingoteEstano",
            "produccionBase": 4.5,
            "consumePorUnidad": {
              "estano": 5
            },
            "requiereTecnologia": "aleacion_bronce"
          },
          {
            "produce": "lingoteBronce",
            "produccionBase": 1.5,
            "consumePorUnidad": {
              "lingoteCobre": 1.6,
              "lingoteEstano": 0.4
            },
            "requiereTecnologia": "aleacion_bronce"
          },
          {
            "produce": "lingoteHierro",
            "produccionBase": 7.5,
            "consumePorUnidad": {
              "hierro": 2,
              "madera": 1
            },
            "requiereTecnologia": "forja_hierro_temprana"
          }
        ]
      }
    }
  },
  "curtiduria": {
    "costo": {
      "madera": 80,
      "piedra": 30
    },
    "tiempoConstruccionMinutos": 720,
    "requisitoNivelAsentamientoConstruccion": 2,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 4,
        "recetas": [
          {
            "produce": "cuero",
            "produccionBase": 8,
            "consumePorUnidad": {
              "livestock": 0.5
            }
          }
        ]
      },
      "2": {
        "requisitoNivelAsentamiento": 2,
        "costoMejora": {
          "madera": 150,
          "piedra": 100
        },
        "trabajadoresRequeridos": 6,
        "recetas": [
          {
            "produce": "cuero",
            "produccionBase": 12,
            "consumePorUnidad": {
              "livestock": 0.3333333333333333
            }
          },
          {
            "produce": "cueroCurtido",
            "produccionBase": 2,
            "consumePorUnidad": {
              "cuero": 3
            }
          }
        ]
      },
      "3": {
        "requisitoNivelAsentamiento": 3,
        "costoMejora": {
          "madera": 450,
          "piedra": 200
        },
        "trabajadoresRequeridos": 8,
        "recetas": [
          {
            "produce": "cuero",
            "produccionBase": 6,
            "consumePorUnidad": {
              "livestock": 0.25
            }
          },
          {
            "produce": "cueroCurtido",
            "produccionBase": 3,
            "consumePorUnidad": {
              "cuero": 3
            }
          },
          {
            "produce": "cueroCalidad",
            "produccionBase": 1,
            "consumePorUnidad": {
              "cueroCurtido": 2,
              "cuero": 1
            }
          }
        ]
      }
    }
  },
  "armeria": {
    "costo": {
      "madera": 80,
      "piedra": 30
    },
    "tiempoConstruccionMinutos": 720,
    "requisitoNivelAsentamientoConstruccion": 2,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 4,
        "recetas": [
          {
            "produce": "armaCobre",
            "produccionBase": 3,
            "consumePorUnidad": {
              "lingoteCobre": 1,
              "madera": 1
            },
            "requiereTecnologia": "metalurgia_cobre"
          },
          {
            "produce": "armaduraBasica",
            "produccionBase": 3,
            "consumePorUnidad": {
              "cuero": 5
            }
          },
          {
            "produce": "armaMadera",
            "produccionBase": 2,
            "consumePorUnidad": {
              "madera": 2
            }
          }
        ]
      },
      "2": {
        "requisitoNivelAsentamiento": 2,
        "requiereEdificio": "fundicion",
        "requiereEdificioNivel": 2,
        "costoMejora": {
          "madera": 150,
          "piedra": 100
        },
        "trabajadoresRequeridos": 8,
        "recetas": [
          {
            "produce": "armaCobre",
            "produccionBase": 3,
            "consumePorUnidad": {
              "lingoteCobre": 1,
              "madera": 1
            },
            "requiereTecnologia": "metalurgia_cobre"
          },
          {
            "produce": "armaduraBasica",
            "produccionBase": 3,
            "consumePorUnidad": {
              "cuero": 5
            }
          },
          {
            "produce": "armaBronce",
            "produccionBase": 2,
            "consumePorUnidad": {
              "lingoteBronce": 1,
              "madera": 2
            },
            "requiereTecnologia": "aleacion_bronce"
          },
          {
            "produce": "armaHierro",
            "produccionBase": 2,
            "consumePorUnidad": {
              "lingoteHierro": 1,
              "madera": 2
            },
            "requiereTecnologia": "forja_hierro_temprana"
          },
          {
            "produce": "armaduraIntermedia",
            "produccionBase": 2,
            "consumePorUnidad": {
              "lingoteCobre": 1,
              "cueroCurtido": 5
            }
          },
          {
            "produce": "armaMadera",
            "produccionBase": 2,
            "consumePorUnidad": {
              "madera": 2
            }
          }
        ]
      },
      "3": {
        "requisitoNivelAsentamiento": 3,
        "requiereEdificio": "fundicion",
        "requiereEdificioNivel": 2,
        "costoMejora": {
          "madera": 450,
          "piedra": 200
        },
        "trabajadoresRequeridos": 20,
        "recetas": [
          {
            "produce": "armaCobre",
            "produccionBase": 3,
            "consumePorUnidad": {
              "lingoteCobre": 1,
              "madera": 1
            },
            "requiereTecnologia": "metalurgia_cobre"
          },
          {
            "produce": "armaduraBasica",
            "produccionBase": 3,
            "consumePorUnidad": {
              "cuero": 5
            }
          },
          {
            "produce": "armaBronce",
            "produccionBase": 2,
            "consumePorUnidad": {
              "lingoteBronce": 1,
              "madera": 2
            },
            "requiereTecnologia": "aleacion_bronce"
          },
          {
            "produce": "armaHierro",
            "produccionBase": 2,
            "consumePorUnidad": {
              "lingoteHierro": 1,
              "madera": 2
            },
            "requiereTecnologia": "forja_hierro_temprana"
          },
          {
            "produce": "armaduraIntermedia",
            "produccionBase": 2,
            "consumePorUnidad": {
              "lingoteCobre": 1,
              "cueroCurtido": 5
            }
          },
          {
            "produce": "armaBronceCalidad",
            "produccionBase": 1,
            "consumePorUnidad": {
              "lingoteBronce": 5,
              "madera": 5
            },
            "requiereTecnologia": "bronce_calidad_militar"
          },
          {
            "produce": "armaduraBronce",
            "produccionBase": 1,
            "consumePorUnidad": {
              "lingoteBronce": 1,
              "cueroCalidad": 5
            },
            "requiereTecnologia": "bronce_calidad_militar"
          },
          {
            "produce": "armaHierroCalidad",
            "produccionBase": 1,
            "consumePorUnidad": {
              "lingoteHierro": 2,
              "madera": 3
            },
            "requiereTecnologia": "forja_hierro_estandarizada",
            "requiereEdificio": {
              "tipo": "fundicion",
              "nivel": 3
            }
          },
          {
            "produce": "armaduraBronceCalidad",
            "produccionBase": 1,
            "consumePorUnidad": {
              "lingoteBronce": 2,
              "cueroCalidad": 5
            },
            "requiereTecnologia": "bronce_laminado"
          },
          {
            "produce": "armaMadera",
            "produccionBase": 2,
            "consumePorUnidad": {
              "madera": 2
            }
          }
        ]
      }
    }
  },
  "carpinteria": {
    "costo": {
      "madera": 60,
      "piedra": 20
    },
    "tiempoConstruccionMinutos": 720,
    "requisitoNivelAsentamientoConstruccion": 2,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 0,
        "recetas": []
      },
      "2": {
        "requisitoNivelAsentamiento": 3,
        "costoMejora": {
          "madera": 120,
          "piedra": 60
        },
        "trabajadoresRequeridos": 4,
        "recetas": [
          {
            "produce": "carroGuerra",
            "produccionBase": 1,
            "consumePorUnidad": {
              "madera": 20,
              "cueroCurtido": 2,
              "lingoteBronce": 1
            },
            "requiereTecnologia": "carros_guerra"
          }
        ]
      }
    }
  },
  "barracon": {
    "costo": {
      "madera": 30
    },
    "tiempoConstruccionMinutos": 720,
    "requisitoNivelAsentamientoConstruccion": 2,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 0,
        "recetas": []
      },
      "2": {
        "requisitoNivelAsentamiento": 2,
        "requiereEdificio": "carpinteria",
        "costoMejora": {
          "madera": 100,
          "piedra": 60
        },
        "trabajadoresRequeridos": 0,
        "recetas": []
      },
      "3": {
        "requisitoNivelAsentamiento": 3,
        "requiereEdificio": "carpinteria",
        "requiereEdificioNivel": 2,
        "costoMejora": {
          "madera": 300,
          "piedra": 200
        },
        "trabajadoresRequeridos": 0,
        "recetas": []
      }
    }
  },
  "galeriaDeTiro": {
    "costo": {
      "madera": 50
    },
    "tiempoConstruccionMinutos": 720,
    "requisitoNivelAsentamientoConstruccion": 2,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 0,
        "recetas": []
      },
      "2": {
        "requisitoNivelAsentamiento": 2,
        "requiereEdificio": "carpinteria",
        "costoMejora": {
          "madera": 140,
          "piedra": 20
        },
        "trabajadoresRequeridos": 0,
        "recetas": []
      },
      "3": {
        "requisitoNivelAsentamiento": 3,
        "requiereEdificio": "carpinteria",
        "requiereEdificioNivel": 2,
        "costoMejora": {
          "madera": 400,
          "piedra": 100
        },
        "trabajadoresRequeridos": 0,
        "recetas": []
      }
    }
  },
  "caballerizas": {
    "costo": {
      "madera": 30
    },
    "tiempoConstruccionMinutos": 720,
    "requiereTecnologia": "cria_caballar",
    "requisitoNivelAsentamientoConstruccion": 2,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 0,
        "recetas": []
      },
      "2": {
        "requisitoNivelAsentamiento": 2,
        "requiereEdificio": "carpinteria",
        "costoMejora": {
          "madera": 100,
          "piedra": 60
        },
        "trabajadoresRequeridos": 0,
        "recetas": []
      },
      "3": {
        "requisitoNivelAsentamiento": 3,
        "requiereEdificio": "carpinteria",
        "requiereEdificioNivel": 2,
        "costoMejora": {
          "madera": 300,
          "piedra": 200
        },
        "trabajadoresRequeridos": 0,
        "recetas": []
      }
    }
  },
  "salaConsejo": {
    "costo": {
      "madera": 800,
      "piedra": 1200,
      "oro": 300
    },
    "requiereTecnologia": "instituciones_civicas",
    "tiempoConstruccionMinutos": 2880,
    "requisitoNivelAsentamientoConstruccion": 3
  },
  "taberna": {
    "costo": {
      "madera": 150,
      "piedra": 60
    },
    "tiempoConstruccionMinutos": 360,
    "requisitoNivelAsentamientoConstruccion": 2,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "cupoMiradas": 1
      },
      "2": {
        "requisitoNivelAsentamiento": 2,
        "costoMejora": {
          "madera": 250,
          "piedra": 120
        },
        "obraMinutos": 720,
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "cupoMiradas": 2
      },
      "3": {
        "requisitoNivelAsentamiento": 3,
        "costoMejora": {
          "madera": 500,
          "piedra": 250
        },
        "obraMinutos": 1440,
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "cupoMiradas": 3
      }
    }
  },
  "palacio": {
    "costo": {
      "madera": 600,
      "piedra": 400,
      "oro": 100
    },
    "tiempoConstruccionMinutos": 4320,
    "requisitoNivelAsentamientoConstruccion": 2,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "capacidadNobles": 80
      },
      "2": {
        "requisitoNivelAsentamiento": 3,
        "costoMejora": {
          "madera": 1200,
          "piedra": 1000,
          "oro": 300
        },
        "obraMinutos": 7200,
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "capacidadNobles": 240
      },
      "3": {
        "requisitoNivelAsentamiento": 4,
        "costoMejora": {
          "madera": 2500,
          "piedra": 2500,
          "oro": 800
        },
        "obraMinutos": 10080,
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "capacidadNobles": 400
      }
    }
  },
  "mercado": {
    "costo": {
      "madera": 100
    },
    "tiempoConstruccionMinutos": 480,
    "niveles": {
      "1": {
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "cupoCaravanas": 2,
        "cupoCarros": 3
      },
      "2": {
        "requisitoNivelAsentamiento": 2,
        "costoMejora": {
          "madera": 150,
          "piedra": 100
        },
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "cupoCaravanas": 4,
        "cupoCarros": 9
      },
      "3": {
        "requisitoNivelAsentamiento": 3,
        "costoMejora": {
          "madera": 450,
          "piedra": 200
        },
        "trabajadoresRequeridos": 0,
        "recetas": [],
        "cupoCaravanas": 6,
        "cupoCarros": 18
      }
    }
  },
  "puestoMercado": {
    "costo": {},
    "tiempoConstruccionMinutos": 0
  },
  "plaza": {
    "costo": {},
    "tiempoConstruccionMinutos": 0
  },
  "plazaDeArmas": {
    "costo": {},
    "tiempoConstruccionMinutos": 0
  },
  "patioDeGremios": {
    "costo": {},
    "tiempoConstruccionMinutos": 0
  },
  "pozo": {
    "costo": {},
    "tiempoConstruccionMinutos": 0
  },
  "parque": {
    "costo": {},
    "tiempoConstruccionMinutos": 0
  },
  "maravilla": {
    "costo": {
      "madera": 5000,
      "piedra": 5000,
      "oro": 500,
      "cobre": 300,
      "estano": 200,
      "livestock": 200
    },
    "tiempoConstruccionMinutos": 40320,
    "requisitoNivelAsentamientoConstruccion": 5,
    "ordenEraMinimo": 5
  }
}
```
<!-- /parametros -->

### 14.2 Construcción y producción

<!-- parametros:NECESIDADES -->
```json
{
  "umbralViviendaOcupada": 0.85,
  "maximoGranjasPendientesEnDeficit": 3,
  "umbralAlmacenAmpliacion": 0.9,
  "maximoAlmacenesPorNivel": {
    "1": 4,
    "2": 8,
    "3": 16,
    "4": 24,
    "5": 32
  },
  "maximoEnCola": 4,
  "maximoEnConstruccionSimultanea": 2,
  "maximoTransformacionPorNivel": {
    "2": 3,
    "3": 5,
    "4": 5,
    "5": 5
  },
  "nivelGateAnclas": 2,
  "reservaManualMaxima": 999
}
```
<!-- /parametros -->

<!-- parametros:SCORE_BANDAS -->
```json
{
  "supervivencia": 10000,
  "extractorBase": 5000,
  "crecimiento": 1000,
  "manual": 900,
  "transformacion": 500
}
```
<!-- /parametros -->

<!-- parametros:EXTRACTOR_DESEMPATE -->
```json
{
  "bonusPorMinutoStarved": 1,
  "bonusMaximo": 400
}
```
<!-- /parametros -->

<!-- parametros:EXTRACCION_MAXIMOS -->
```json
{
  "porTipo": 10
}
```
<!-- /parametros -->

<!-- parametros:MEJORA_EDIFICIO -->
```json
{
  "multiplicadorPorNivel": 2
}
```
<!-- /parametros -->

<!-- parametros:ALMACEN -->
```json
{
  "capacidadInicialPorRecurso": 400
}
```
<!-- /parametros -->

<!-- parametros:MERCADO_PUESTOS_POR_NIVEL -->
```json
{
  "1": [
    1,
    1,
    2,
    2,
    3
  ],
  "2": [
    1,
    1,
    2,
    2,
    3,
    3,
    3
  ],
  "3": [
    1,
    2,
    2,
    3
  ]
}
```
<!-- /parametros -->

<!-- parametros:PUESTO_MERCADO_FORMA -->
```json
{
  "1": {
    "ancho": 1,
    "alto": 2
  },
  "2": {
    "ancho": 1,
    "alto": 3
  },
  "3": {
    "ancho": 1,
    "alto": 1
  }
}
```
<!-- /parametros -->

<!-- parametros:LENERA_POR_BOSQUE -->
```json
{
  "umbral2": 45,
  "umbral3": 65
}
```
<!-- /parametros -->

<!-- parametros:REGENERACION_NODOS -->
```json
{
  "metales": {
    "cooldownMinutos": 10
  },
  "livestock": {
    "cooldownMinutos": 3
  }
}
```
<!-- /parametros -->

Listas del catálogo (también en `constants.ts`): `EDIFICIOS_UNICOS`, `EDIFICIOS_CON_NIVELES` y `RECURSO_PROPIO_DE_EDIFICIO` (el recurso que cada edificio de supervivencia no protege con la reserva).

<!-- parametros:EDIFICIOS_UNICOS -->
```json
[
  "granero",
  "barracon",
  "galeriaDeTiro",
  "caballerizas",
  "palacio",
  "salaConsejo",
  "taberna",
  "mercado",
  "granFundicion",
  "maravilla"
]
```
<!-- /parametros -->

<!-- parametros:EDIFICIOS_CON_NIVELES -->
```json
[
  "fundicion",
  "curtiduria",
  "armeria",
  "carpinteria",
  "barracon",
  "galeriaDeTiro",
  "caballerizas",
  "palacio",
  "mercado",
  "granja",
  "granero",
  "taberna"
]
```
<!-- /parametros -->

<!-- parametros:RECURSO_PROPIO_DE_EDIFICIO -->
```json
{
  "granja": "trigo",
  "lenera": "madera"
}
```
<!-- /parametros -->

`NECESIDADES.maximoTransformacionPorNivel` es el tope de cada tipo de transformación por nivel operativo (§6.2), `nivelGateAnclas` el nivel mínimo para crear un ancla nueva y `reservaManualMaxima` el recorte de la reserva manual (§13).

### 14.3 Fundación y zona

<!-- parametros:FUNDACION -->
```json
{
  "maxJugadoresFundacionGrupal": 5,
  "minFundadoresParaFaccionNueva": 1,
  "exigeCiudadaniaPrevia": false,
  "distanciaMinimaAparicion": 200,
  "intentosDeAparicion": 200,
  "materialesIniciales": {
    "madera": 50,
    "piedra": 20,
    "trigo": 100,
    "oro": 100
  },
  "viviendasIniciales": 3,
  "costoMaderaExtraCaravana": 50,
  "caducidadCaravanaHoras": 48
}
```
<!-- /parametros -->

<!-- parametros:ZONA_INFLUENCIA -->
```json
{
  "radioInicial": 30,
  "crecimientoPorEdificioCompletado": 5,
  "radioMaximoPorNivel": {
    "1": 60,
    "2": 90,
    "3": 120,
    "4": 150,
    "5": 180
  },
  "segmentosPoligono": 48,
  "pasoFusionContorno": 5
}
```
<!-- /parametros -->

### 14.4 Población

<!-- parametros:POBLACION -->
```json
{
  "pesants": {
    "inicial": 20,
    "tasaCrecimientoBase": 0.12
  },
  "artesanos": {
    "tasaCrecimientoBase": 0.15
  },
  "nobleza": {
    "minCiudadanos": 3,
    "tasaCrecimientoBase": 0.01
  },
  "consumoComidaPorHabitante": 0.1,
  "hambre": {
    "nutricionInicial": 100,
    "regeneracionPorMinuto": 5,
    "degradacionSinComida": 20,
    "factorCrecimientoMinimo": 0.2,
    "umbralMuertePorHambre": 0,
    "fraccionMuertePorMinutoHambre": 0.05,
    "fraccionMuertePorMinutoHambreNobleza": 0.01
  }
}
```
<!-- /parametros -->

<!-- parametros:IMPUESTOS -->
```json
{
  "tasaPesants": 0.004,
  "tasaArtesanos": 0.015,
  "tasaNobleza": 0.06
}
```
<!-- /parametros -->

### 14.5 Mantenimiento y nivel

<!-- parametros:MANTENIMIENTO -->
```json
{
  "medidorInicial": 100,
  "costoBase": {
    "madera": 1.5
  },
  "nivelParaPiedra": 2,
  "piedraBase": 3,
  "nivelParaOro": 3,
  "oroBase": 2,
  "edificiosReferencia": 50,
  "pesoVivienda": 0.5,
  "pesoPiezaGratuita": 0.5,
  "piezasGratuitas": [
    "puestoMercado",
    "plaza",
    "plazaDeArmas",
    "patioDeGremios",
    "pozo",
    "parque"
  ],
  "escalaDistancia": 400,
  "degradacionPorDeficitTotal": 10,
  "regeneracionSiPagoCompleto": 5,
  "graciaMinutos": 1440,
  "minutosSanosParaRecuperarNivel": 30
}
```
<!-- /parametros -->

<!-- parametros:RESERVA_CONSTRUCCION -->
```json
{
  "horizonteMinutosMantenimiento": 8,
  "horizonteMinutosComida": 8
}
```
<!-- /parametros -->

<!-- parametros:NIVEL_ASENTAMIENTO -->
```json
{
  "nivelMaximo": 5,
  "requisitos": {
    "2": {
      "pesants": 200,
      "artesanos": 0,
      "edificios": [
        "cantera",
        "lenera",
        "granja",
        "mina",
        "minaCobre",
        "minaEstano",
        "corral"
      ],
      "edificiosMinimo": 3
    },
    "3": {
      "pesants": 500,
      "artesanos": 400,
      "edificios": [
        "armeria",
        "curtiduria",
        "fundicion",
        "barracon",
        "galeriaDeTiro"
      ]
    },
    "4": {
      "pesants": 1000,
      "artesanos": 800,
      "edificios": [
        "salaConsejo"
      ],
      "recintoCompletoNivelMinimo": 2
    },
    "5": {
      "pesants": 2000,
      "artesanos": 1600,
      "edificios": [
        "palacio"
      ],
      "nivelInternoMinimo": {
        "palacio": 3
      }
    }
  },
  "techoPoblacion": {
    "1": 300,
    "2": 1500,
    "3": 6000,
    "4": 12000,
    "5": 20000
  }
}
```
<!-- /parametros -->

<!-- parametros:ASCENSO_ASENTAMIENTO -->
```json
{
  "porNivelObjetivo": {
    "2": {
      "costo": {
        "madera": 600,
        "piedra": 400,
        "oro": 150
      },
      "obraMinutos": 4320
    },
    "3": {
      "costo": {
        "madera": 1200,
        "piedra": 1000,
        "oro": 400,
        "lingoteBronce": 100
      },
      "obraMinutos": 10080
    },
    "4": {
      "costo": {
        "madera": 2500,
        "piedra": 2500,
        "oro": 1000,
        "lingoteBronce": 150
      },
      "obraMinutos": 20160
    },
    "5": {
      "costo": {
        "madera": 5000,
        "piedra": 5000,
        "oro": 2500
      },
      "obraMinutos": 30240
    }
  },
  "nivelTechoProvisional": 4
}
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

### 14.6 Políticas

<!-- parametros:POLITICAS -->
```json
{
  "slotsPorCargo": {
    "gobernador": {
      "base": 2,
      "maximo": 5
    },
    "tesorero": {
      "base": 2,
      "maximo": 2
    },
    "general": {
      "base": 1,
      "maximo": 1
    },
    "maestroObras": {
      "base": 1,
      "maximo": 1
    },
    "sacerdote": {
      "base": 1,
      "maximo": 1
    }
  },
  "nivelFaccionPorSlotExtraGobernador": 3,
  "slotSalaConsejo": 1,
  "duracionMinutosPorDefecto": 150
}
```
<!-- /parametros -->

<!-- parametros:POLITICA_CATALOGO -->
```json
[
  {
    "id": "racionamiento",
    "cargo": "sacerdote",
    "nombre": "Racionamiento",
    "factorConsumoComida": 0.8
  },
  {
    "id": "culto_fertilidad",
    "cargo": "sacerdote",
    "nombre": "Culto a la Fertilidad",
    "factorCrecimientoNobleza": 1.5
  },
  {
    "id": "via_rapida",
    "cargo": "maestroObras",
    "nombre": "Vía Rápida de Construcción",
    "factorTiempoConstruccion": 0.75
  },
  {
    "id": "postura_defensiva",
    "cargo": "maestroObras",
    "nombre": "Postura Defensiva",
    "perfilTrazado": "compacta",
    "factorTiempoMuralla": 0.75
  },
  {
    "id": "arterias_comerciales",
    "cargo": "maestroObras",
    "nombre": "Arterias Comerciales",
    "perfilTrazado": "caminera",
    "factorComisionExterna": 0.8
  },
  {
    "id": "barrios_gremiales",
    "cargo": "maestroObras",
    "nombre": "Barrios Gremiales",
    "perfilTrazado": "gremial",
    "factorProduccionTalleres": 1.1
  },
  {
    "id": "plazas_mayores",
    "cargo": "maestroObras",
    "nombre": "Plazas Mayores",
    "perfilTrazado": "nucleos",
    "factorCrecimientoPoblacion": 1.25
  },
  {
    "id": "comercio_abierto",
    "cargo": "tesorero",
    "nombre": "Comercio Abierto",
    "factorComisionExterna": 0.6
  },
  {
    "id": "aranceles",
    "cargo": "tesorero",
    "nombre": "Aranceles Proteccionistas",
    "factorComisionExterna": 1.5
  },
  {
    "id": "leva_forzosa",
    "cargo": "general",
    "nombre": "Leva Forzosa",
    "factorCostoReclutamiento": 0.7
  },
  {
    "id": "levas_guarnicion",
    "cargo": "general",
    "nombre": "Levas de guarnición",
    "cupoGuarnicionExtra": 14
  },
  {
    "id": "cupo_caravana_extra",
    "cargo": "tesorero",
    "nombre": "Ampliación de Flota",
    "cupoCaravanaExtra": 1
  },
  {
    "id": "carga_ampliada",
    "cargo": "tesorero",
    "nombre": "Carga Ampliada",
    "factorCapacidadCaravana": 1.5
  },
  {
    "id": "rutas_rapidas",
    "cargo": "tesorero",
    "nombre": "Rutas Rápidas",
    "factorVelocidadCaravana": 1.5
  },
  {
    "id": "edicto_cosecha",
    "cargo": "gobernador",
    "nombre": "Edicto de Cosecha",
    "factorProduccionTrigo": 1.5
  },
  {
    "id": "presion_fiscal",
    "cargo": "tesorero",
    "nombre": "Presión Fiscal",
    "factorRecaudacion": 1.6,
    "factorCrecimientoPoblacion": 0.8
  }
]
```
<!-- /parametros -->

Todas las cifras de este documento son **placeholder sin calibrar** salvo que el código diga lo contrario; el efecto conjunto se mide con batch (`Docs/Mecanicas a balancear.md`).

## 15. Invariantes

| Invariante | Dónde se vigila |
|---|---|
| El almacén nunca es negativo, ni NaN, ni supera su capacidad | `invariantes_simulacion_larga.test.ts` |
| La población nunca es negativa ni NaN | ídem |
| `medidorMantenimiento` siempre en [0, 100] | ídem |
| `nivel` nunca baja; `nivelActual ≤ nivel` | ídem |
| Un recurso nunca baja de la reserva por una obra o taller automáticos | `recetasTalleres.test.ts`, `regresiones_historicas.test.ts` |
| El coste de mantenimiento escala con edificios activos (la Vivienda y las piezas gratuitas cuentan la mitad), con el nivel efectivo y con la distancia sin tope | `costoMantenimiento.test.ts` |
| El colapso del medidor baja `nivelActual` un escalón; en nivel 1 es ruina | `eventosDominioMantenimiento.test.ts` |
| La nutrición cae en `100/degradacionSinComida` ticks sin trigo y cuesta población solo desde 0 | `hambruna.test.ts` |
| El cupo de Facción por nivel no se puede superar ni reservando | `cupo_nivel_invariante.test.ts`, `ascenso.test.ts` |
| La capacidad de Almacén/Granero sigue a los edificios que funcionan (no se infla tras reconstruir) | `capacidadAlmacenaje.test.ts` |
| Ningún edificio queda bajo una calle comprometida | `edificiosSobreCalle.test.ts` (Trazado_Urbano.md) |
| La mejora manual exige cuadrilla libre y respeta gates | `mejoraManual.test.ts` |

## 16. Casos límite y decisiones que parecen raras

- **El pago es al encolar y se devuelve entero al quitar** de la cola; una vez `en_construccion` no hay vuelta atrás ni reembolso.
- **El orden del array `edificios` no se toca** al ordenar la cola: lo que se reordena es `prioridad`. Permutar el array movería las calles (Trazado_Urbano.md).
- **Un almacén lleno pausa**: ni el taller consume insumos ni el extractor gasta el yacimiento por lo que no cabe (§5.5).
- **Los nodos se regeneran rápido** (3 / 10 min), así que «agotar» un yacimiento es un bache corto; mientras tanto el extractor sigue pidiendo trabajadores y cuenta en el mantenimiento.
- **Todo edificio `activo` paga mantenimiento**: los normales y el Centro Urbano, 1; las Viviendas y las piezas gratuitas (puestos del Mercado, anclas), 0,5.
- **`requiereEdificio`**, tanto de una mejora como de una receta, se cumple con **cualquier** edificio activo de ese tipo que tenga el nivel.
- **La zona crece solo construyendo**: sin obras terminadas el radio no crece, aunque suba el nivel (solo sube su techo).
- **`nivel` vs `nivelActual`**: el techo de población y el radio máximo usan el nivel **alcanzado**; construir, mejorar, el cupo de Almacenes/Viviendas/transformación y los materiales del mantenimiento usan el **operativo**.
- **Gracia de mantenimiento** de un día desde `fundadoEn`; durante ella el medidor ni baja ni sube. La ocupación tras una conquista suspende el mantenimiento por completo (no cobra, no regenera).
- **Las anclas pueden crearse sin llegar a construir nada** (§6.2): son suelo que ocupa un edificio gratuito.
- **Cada candidato propuesto gasta un número de id** aunque no se comprometa.
- **Las políticas expiran después de la construcción** del mismo tick (§1): el último tick de una política todavía la aplica.

## 17. Divergencias

Ninguna abierta (las encontradas al documentar se resolvieron el 2026-10-11; ver [`Divergencias.md`](Divergencias.md) para las demás áreas).

## 14. Pueblos extractores y obras de mapa (guerra logística, Doc 4.2.2)

> **Implementado en Rust (2026-10-19, fase 3C).** Reglas puras en `crates/asentamiento/src/pueblos.rs` (el pueblo y su obra, el cupo, los costes y la extracción contra el almacén local) y el bloque `PUEBLOS_EXTRACTORES` de más abajo (**entra en la huella de las reglas de las fotos**); integración en `crates/nucleo/src/nucleo/pueblos.rs` (órdenes, carro de acarreo, candidatos), `economia.rs` (el mapa indexa ahora también los nodos de mineral) y `urbanismo.rs` (la planta). Formato de foto **`BAN7`** con los pueblos y **`BAN8`** (2026-10-19, ventana del acarreo; `BAN7` se rechaza): cada plaza guarda sus pueblos y la ventana en `codec_economia`. Pruebas: `crates/nucleo/tests/pueblos.rs` y los módulos `pueblos` y `acarreo`.
> - **Un pueblo no es un edificio de la ciudad.** Vive en `Asentamiento.pueblos` de su madre (por id ascendente; su id es de mundo, del mismo contador que ejércitos y plazas), no en `edificios`: no pasa por la cola, no ocupa cuadrilla, no tiene pieza en la ciudad y no agranda ni la ciudad ni la zona. Activo, cuenta como **un edificio activo más** en el mantenimiento de la madre y sus trabajadores (los del extractor del catálogo) entran en la demanda de pesants de `ratioManoObra`; mientras está pendiente o en obra no cuenta para nada. No cuenta en los gates de nivel (ninguno pide un mineral a partir del nivel 3).
> - **Restricción de extracción.** `AnadirEdificio` rechaza con `obra.solo_por_pueblo` los minerales y toda Cantera después de la primera (la primera, en cualquier estado, sigue siendo directa). La auto-construcción ya solo proponía la primera Cantera y el Corral, y `Mapa::proponer` devuelve «sin sitio» a las minas: no cambia el ritmo de la ciudad, y el informe de `batch poblacion` sale igual. Los pueblos no entran en `EXTRACCION_MAXIMOS` ni en el reemplazo automático.
> - **Obra de mapa** (órdenes 45-48, con la autorización provisional de las demás órdenes de plaza: administración o propietario de una columna de la Facción de la madre). `MarcarPueblo { asentamiento, nodo }` exige un nodo de piedra o mineral **libre, dentro de la zona real** de la madre, madre de nivel operativo ≥ 2, cupo (2 / 3 / 4 / 5 por nivel, contando pendientes y en obra) y la tecnología del extractor; deja la obra «pendiente de materiales» (`pueblo.marcado`), reclama el nodo y **no cobra nada al almacén**. `EntregarMateriales { ejercito, pueblo }` lleva a la obra, con la columna **a `radioPuerta` (2 u)** de su nodo y de la misma Facción, lo que falta de lo que cargan su carro (`suministro`) y sus caravanas adjuntas (primero el carro, luego las caravanas por id), recurso a recurso; con todo entregado arranca la obra (`pueblo.obra_iniciada`, la duración del extractor del catálogo: Cantera 4 h, cobre y hierro 6 h, oro y estaño 8 h) y el primer paso de minuto de la plaza con `t >= completaEn` la termina (precisión de un minuto, sin evento propio). `DestruirPueblo { pueblo }` cancela la obra o destruye el pueblo: libera cupo y nodo y **no devuelve nada** (ni lo entregado ni lo almacenado); el carro se pierde con su carga. `ReponerCarro { pueblo }` compra en la madre un carro nuevo (carro + buey) solo si el pueblo activo no tiene ninguno. `Nucleo::candidatos_de_pueblo(plaza)` es el panel: los nodos de la zona sin pueblo cuyo extractor puede levantar la Facción.
> - **Coste.** `PUEBLOS_EXTRACTORES.costeAdicional` (50 madera) + el coste del extractor del catálogo + el carro con su buey de la flota (`comercio::acarreo::coste_reposicion`: 20 madera y 12 oro): una mina de cobre son **100 madera y 12 oro**, como dice el ejemplo de arriba.
> - **Extracción.** Un minuto de extracción del pueblo activo es `produccionBase × ratioManoObra × factorTecnología` contra el almacén local (500 por recurso: `capacidadAlmacenLocal`, que debe ser al menos la capacidad del carro). Con el almacén lleno para y avisa una sola vez (`pueblo.almacen_lleno`); con el nodo agotado espera a la regeneración (10 min) sin mudarse. Va dentro del paso de minuto de la plaza: no hay evento por pueblo, tampoco por el ocioso.
> - **Solvencia del ascenso (2026-10-19, decidido por el autor).** Lo que llega por acarreo cuenta como la **media por minuto de lo que ingresó en el almacén de la madre en las últimas 24 h**, no como la producción nominal. `Asentamiento.ingresos_acarreo` guarda la ventana como cubos de una hora por recurso (`IngresoAcarreo { recurso, hora, cantidad }`, a lo sumo 24 por recurso): el carro, al llegar (`acarreo_llega`), anota en su hora **lo que cupo** (lo que se pierde por almacén lleno no cuenta) y descarta los cubos que salieron de la ventana. Es una acumulación en la llegada: ningún evento por pueblo ocioso y el paso de minuto no cambia. La solvencia suma los cubos con `hora ∈ (ahora − 24, ahora]` (la hora en curso y las 23 anteriores: entre 23 y 24 horas de historia) y divide entre 1.440 minutos, en enteros con división truncada (siempre por debajo: conservador). Un pueblo recién levantado no pesa del todo hasta que lleva un día entregando. `bloqueos_de_ascenso` recibe ahora el instante de la pregunta. Sin bloque de parámetros nuevo: **la huella de las reglas no cambia**; sí cambia el formato de foto (`BAN8`).
> - **Ruina y conquista de la madre.** Los pueblos son de la plaza: en la ruina desaparecen con ella (sus nodos quedan libres y sus carros, que la tienen por origen, se pierden en la cascada de comercio); en la conquista pasan con la plaza (y su carro, en la flota traspasada) a quien la tenga.
> - **Planta del pueblo.** `Nucleo::planta_de_pueblo(id)` / `urbanismo::planta_de_pueblo`: función pura del id (`hash(semilla, "pueblo", [id])`), no se guarda; el trazado de asentamientos una sola vez con el extractor como ancla en el origen, 4 casas, el almacén y el corral del buey.
> - **Hechos nuevos** (datos entre paréntesis): `pueblo.marcado` (pueblo, tipo, nodo), `pueblo.materiales_entregados` (pueblo, cantidad), `pueblo.obra_iniciada` (pueblo, completaEn), `pueblo.completado`, `pueblo.almacen_lleno`, `pueblo.destruido` (pueblo), `acarreo.sale` (pueblo, caravana, carga), `acarreo.llega` (caravana, carga, perdido), `acarreo.vuelve` (pueblo, caravana) y `acarreo.carro_repuesto` (pueblo, caravana). Rechazos nuevos: `obra.solo_por_pueblo` y `pueblo.*` (desconocido, nodo_desconocido, nodo_no_extraible, nodo_ocupado, fuera_de_la_zona, tecnologia_faltante, nivel_insuficiente, sin_cupo, no_esta_pendiente, nada_que_entregar, faccion_ajena, fuera_de_la_puerta, no_esta_activo, ya_tiene_carro).
> - **Provisional (DUDA en el código y filas de paridad en `Docs/Mecanicas a desarrollar.md`).** Los cargos y los ciudadanos de la Facción no existen: manda `manda_en` (administración, propietario de una columna o cuenta de un héroe ciudadano de la Facción) y entrega el propietario de una columna. Desde la fase 4B-1 las tecnologías de la Facción (requisitos de edificios, recetas y pueblos) las da la administración (`FijarTecnologias`) y la ración de la guarnición de los residentes entra en el `Exterior` de la plaza. La guarnición de 30 mineros (el dato está en el bloque) y el saqueo son del sistema militar. Las avanzadas (§5.17) son la fase 4F (§14.1). Un pueblo completado no agranda la zona de la madre. La planta no garantiza el tope de 0,2 de mapa.

**La extracción mineral sale de la ciudad.** Los minerales (cobre, estaño, hierro, oro) y toda Cantera **después de la primera** solo se explotan con un **pueblo extractor**; Granja, Leñera y Corral siguen dentro. La **primera Cantera** sigue siendo directa (sin piedra no hay obra de ascenso al nivel 2). En nivel 1 ya no hay minas; el oro de esa etapa sale de la recaudación por población.

- **Qué es**: el extractor de mapa (`ambito: 'mapa'`) sobre su nodo, **más un almacén local (500, placeholder), un carro con buey (Comercio §12) y una guarnición de serie de 30 mineros** (IA, sin dueño, fuera de cupo, se repone gratis en 6 h). No es un asentamiento: sin zona, nivel, cargos ni medidor; cuenta como **edificio activo de su plaza madre**, que paga su mantenimiento.
- **Requisitos**: madre de **nivel 2 o más** y el nodo dentro de su zona. **Cupo** por nivel de la madre: 2 / 3 / 4 / 5 pueblos.
- **Trabajadores**: pesants de la madre; el pueblo no tiene población propia ni crece.
- **Extracción**: **ritmo** `produccionBase × ratioManoObra × factorTecnología` contra el almacén local; con el almacén lleno, la extracción para (evento «almacén lleno»). Con el nodo agotado espera a la regeneración (`REGENERACION_NODOS`) sin mudarse; los pueblos **no** entran en el reemplazo automático ni en `EXTRACCION_MAXIMOS`.
- **Destruirlo a mano** libera el cupo y no recupera nada. Tamaño: como mucho **0,2 de mapa**.
- **Conquista de la madre**: los pueblos pasan al conquistador; **ruina** de la madre: desaparecen.
- **Solvencia** para subir de nivel (§11): lo que llega por acarreo cuenta como la media de lo que **ingresa en el almacén de la madre** en una ventana de 24 h (decidido el 2026-10-19; ya no la producción nominal del pueblo).

**Obras de mapa** (pueblos y avanzadas): no pasan por la cola de construcción. (1) Panel en la plaza madre (Gobernador o Maestro de Obras presentes): candidatos = nodos de la zona sin pueblo; una avanzada no tiene candidatos (la coloca el héroe en el sitio, dentro de la zona). (2) La obra aparece en el mapa **«pendiente de materiales»** y ocupa cupo. (3) Los materiales se **llevan físicamente** en los carros de las columnas o en caravanas adjuntas a un ejército, y se entregan con la columna a `radioPuerta`. (4) Con todo entregado arranca la obra (corta; evento de fin). (5) Cancelar o destruir no recupera lo entregado. Pendiente, se ve pero no se puede atacar. Costes (placeholder): pueblo = su extractor + 50 madera + carro (20 madera) + buey (12 oro), obra de 4-8 h.

**Mineros**: tropa nueva del Centro Urbano (leva, escalón 1, 30 unidades, velocidad 30, 1 de piedra por soldado, sin oro); entran en la tropa prestada del campamento.

<!-- parametros:PUEBLOS_EXTRACTORES -->
```json
{
  "nivelMadreMinimo": 2,
  "cupoPorNivel": {
    "2": 2,
    "3": 3,
    "4": 4,
    "5": 5
  },
  "capacidadAlmacenLocal": 500,
  "costeAdicional": {
    "madera": 50
  },
  "mineros": {
    "escuadra": 30,
    "reposicionMinutos": 360
  }
}
```
<!-- /parametros -->

### 14.1 Avanzadas (Doc 5.17, fase 4F)

Una avanzada es la otra obra de mapa (§14, «Obras de mapa»). Reglas puras en `crates/asentamiento/src/avanzadas.rs` (la avanzada vive en `Asentamiento.avanzadas` de su plaza madre, como los pueblos); el ataque, las ruinas y la guarnición son de [`Combate.md`](Combate.md) §7.4 (bloque `AVANZADA`) y el núcleo las une en `crates/nucleo/src/nucleo/avanzadas.rs`.

<!-- parametros:AVANZADAS -->
```json
{
  "nivelMadre": {
    "atalaya": 2,
    "fuerte": 2,
    "ciudadela": 3
  },
  "cupoPorNivelDeLaMadre": 1,
  "capacidadAlmacen": {
    "atalaya": 0,
    "fuerte": 2000,
    "ciudadela": 10000
  },
  "tamano": 0.2,
  "fraccionCiudadDeLaZona": 0.1,
  "fraccionReparacion": 0.5,
  "obras": {
    "atalaya": {
      "costo": {
        "madera": 200,
        "piedra": 100
      },
      "minutos": 120
    },
    "fuerte": {
      "costo": {
        "madera": 600,
        "piedra": 400,
        "oro": 50
      },
      "minutos": 720
    },
    "ciudadela": {
      "costo": {
        "madera": 1500,
        "piedra": 2000,
        "oro": 300
      },
      "minutos": 2880
    }
  }
}
```
<!-- /parametros -->

Las etapas son las tres de Doc 5.17 (atalaya → fuerte → ciudadela) y `obras` es el coste de **cada paso** (construir la atalaya, mejorarla a fuerte, mejorar el fuerte a ciudadela), con su duración en minutos de mundo: los «viajes de carro de 500» del documento son unidades de mapa, a **placeholder**. `nivelMadre` es el nivel operativo mínimo de la plaza madre para tener esa etapa y `cupoPorNivelDeLaMadre`, cuántas avanzadas admite por nivel (2 en el nivel 2 … 5 en el 5, contando las pendientes y las que están en ruinas). `capacidadAlmacen` es la cabida **por recurso** de cada etapa (la atalaya no tiene almacén). `tamano` es lo que mide un elemento nuevo del mapa (Doc 4.2.2 y 5.17: «como mucho 0,2 de mapa»): un disco de ese diámetro, de modo que dos elementos no se pisan si sus centros distan `tamano` o más. `fraccionCiudadDeLaZona` es lo que mide la ciudad de la plaza en el mapa como fracción del radio de su zona (una décima: «la ciudad nunca pasa de una décima de su provincia», Calibración de distancias §4.2): una avanzada no se coloca a menos de ese radio más la mitad de su tamaño del centro de la madre. `fraccionReparacion` es el 50 % del coste de la etapa que cuesta reparar unas ruinas.

> **Implementado en Rust (2026-10-22, fase 4F).** Foto `BAN15` (hoy `BAN16`).
> - **Colocar** (`ColocarAvanzada { asentamiento, heroe, posicion }`, 106): el héroe, ciudadano de la Facción de la madre (o la administración), con su columna **en el sitio** (a ≤ `radioPuerta` de `posicion`), la marca dentro de la zona real de la madre, sin pisar su ciudad (`avanzada.dentro_del_asentamiento`: a menos de `radio de la zona × fraccionCiudadDeLaZona + tamano / 2` de su centro) ni otro elemento del mapa (`avanzada.pisa_otro_elemento`: otra avanzada en cualquier estado, un pueblo sobre su nodo o un campamento de mercenarios o de bandidos a menos de `tamano`). La madre ha de tener nivel operativo ≥ 2 y cupo. La obra queda **«pendiente de materiales»** (no se ve como puesto, no se ataca, no ve, ocupa cupo), no cobra nada al almacén y no pasa por la cola de construcción.
> - **Entregar** (`EntregarMaterialesAAvanzada { ejercito, avanzada }`, 107): con la columna a la puerta de la obra y de la misma Facción, lo que lleva su carro y sus caravanas adjuntas, recurso a recurso, como en un pueblo. Con todo entregado arranca la obra y su **fin tiene cita propia** (`SucesoCombate::FinObraAvanzada`, ligada a la avanzada —`entidad_obra_de_avanzada`—, prioridad 1, en su instante exacto: no espera al paso de minuto).
> - **Mejorar** (108) y **reparar** (109) son la misma obra con otro coste: pasar a la etapa siguiente (a nivel de la madre suficiente) o reparar unas ruinas de fuerte o ciudadela (la mitad del coste de la etapa, la duración entera). Mientras se mejora la avanzada sigue funcionando; mientras se repara sigue en ruinas. Un ataque que la derriba pierde la obra. **Cancelar** (`CancelarObraDeAvanzada { heroe, avanzada }`, 116): la mejora o la reparación, pendiente de materiales o ya en marcha, se abandona **sin reembolso** (lo entregado se pierde) y la avanzada sigue como estaba (operativa en su etapa, o en ruinas). Rechazos: `avanzada.nada_que_cancelar` (sin obra, o la primera, que se cancela destruyendo la avanzada), `avanzada.desconocida`, `no_autorizado`. Su cita de fin deja de ser vigente (sube la generación de la entidad de su obra: lo mismo que pasa al destruir la avanzada, derribarla un ataque o caer su madre) y el hecho es `avanzada.obra_cancelada` (avanzada, tipo).
> - **Destruir a mano** (110): atalaya y fuerte, también si solo es una obra pendiente; la ciudadela no. No recupera nada.
> - **Mantenimiento**: una avanzada operativa cuenta como un edificio activo de la madre (como un pueblo activo). Pendiente, en obra o en ruinas, no cuenta.
> - **Almacén y guarnición**: el almacén es **por recurso** (decisión del autor, 2026-10-22: 2.000 por cada recurso en un fuerte, 10.000 en una ciudadela). `DepositarEnAvanzada` (115) y `RepostarEnAvanzada` (114) llenan y vacían el almacén ([`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §8); `GuarnecerAvanzada` (112), `RetirarDeAvanzada` (113) y `ElegirTropaDeAvanzada` (111) manejan la guarnición ([`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md), [`Combate.md`](Combate.md) §7.4).
> - **Ruinas y la madre**: una avanzada en ruinas no funciona (ni visión, ni guarnición, ni almacén, ni mantenimiento) y sigue ocupando cupo. Si la madre es **conquistada**, las avanzadas pasan con ella (su guarnición de jugadores se va con el desalojo y se reinician la de serie y las elecciones del Gobernador). Si la madre **cae en ruinas**, la atalaya y el fuerte desaparecen (la guarnición de jugadores vuelve intacta a su campamento) y la ciudadela queda como **ruina sin dueño** (`EstadoCombate::huerfanas`, con su almacén), que repara (`RepararAvanzada`) un héroe ciudadano de la Facción que tenga ahora su zona: la ruina pasa a esa plaza si tiene el nivel (3) y el cupo.
> - **Hechos nuevos** (datos entre paréntesis): `avanzada.obra_marcada` (avanzada, tipo 0 construir / 1 mejorar / 2 reparar, x, y), `avanzada.materiales_entregados` (avanzada, cantidad), `avanzada.obra_iniciada` (avanzada, tipo, completaEn), `avanzada.obra_cancelada` (avanzada, tipo), `avanzada.completada` (avanzada, tipo, etapa), `avanzada.destruida` (avanzada, motivo: 0 a mano, 1 un ataque, 2 la ruina de la madre), `avanzada.sin_dueno`, `avanzada.adoptada`, `avanzada.tropa_elegida`, `avanzada.guarnecida` / `avanzada.guarnicion_retirada`, `avanzada.deposito`, `avanzada.reabastecimiento`, `avanzada.traspasada` (avanzada, faccion), `avanzada.encuentro` (una columna ajena junto a una avanzada que se puede atacar). Rechazos nuevos: `avanzada.*` (desconocida, nivel_insuficiente, sin_cupo, fuera_de_la_zona, dentro_del_asentamiento, fuera_del_sitio, faccion_ajena, fuera_de_la_puerta, no_esta_pendiente, nada_que_entregar, no_mejorable, no_reparable, ya_hay_obra, ciudadela_no_se_destruye, no_operativa, sin_almacen, almacen_lleno, nada_que_depositar, no_resides, escuadra_fuera_de_tu_columna, escuadra_no_esta, tropa_no_elegible, tropa_sin_tecnologia, tropa_sin_edificio, pisa_otro_elemento, nada_que_cancelar) y `combate.avanzada_*`.
> - **Provisional** (filas de paridad): manda un héroe ciudadano de la Facción de la madre, sin Gobernador ni Maestro de Obras presentes ni panel; no hay planta de avanzada (el tamaño de 0,2 solo sirve para no pisar otros elementos del mapa).

> **Implementado en Rust (2026-10-22, decisiones de 4F).** Foto `BAN16` (la cita de fin de la obra de una avanzada es `SucesoCombate::FinObraAvanzada`, de la entidad `entidad_obra_de_avanzada`, y las cifras de `AVANZADAS` cambian: `tamano` y `fraccionCiudadDeLaZona` sustituyen a `distanciaMinimaAlAsentamiento`). (1) **Colocación según el canon**: ni sobre la ciudad de la madre ni sobre otro elemento del mapa (`asentamiento::avanzadas::ReglasAvanzadas::comprobar_sitio`; el núcleo junta los centros con `elementos_de_mapa`). El «2 u» de la versión anterior era el radio de la puerta y no una regla de colocación. (2) **Cancelar una mejora o una reparación** (116), con la cita de fin descartada por generación. (3) **Elegir tropa** (`ElegirTropaDeAvanzada`, 111) solo entre lo que la madre puede reclutar: escalón elegible, tecnología de la Facción y edificio activo al nivel que pide la tropa (`avanzada.tropa_sin_edificio`); por defecto la milicia de lanceros del canon en los tres escalones. (4) **Sin duda**: almacén por recurso, botín de todo lo que quepa, reposición de 6 h y repostar con columna personal o ejército. (5) **Defender exige estar dentro** ([`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §11.1).

Los bloques de parámetros de este documento son los del motor TypeScript. Valores vigentes para la reconstrucción (Calibracion_Distancias_Definicion §4.2):

| Parámetro | Bloque (TypeScript) | Vigente |
|---|---|---|
| `ZONA_INFLUENCIA.radioInicial` | 30 | **6** |
| `ZONA_INFLUENCIA.radioMaximoPorNivel` | 60 / 90 / 120 / 150 / 180 | **10 / 15 / 20 / 25 / 30** |
| `ZONA_INFLUENCIA.crecimientoPorEdificioCompletado` | 5 | **1** |
| `ZONA_INFLUENCIA.separacionPlazas` | — | **30** (Doc 1.0a) |
| `MERCENARIOS.radioExclusionFundar` | 100 | **20** |
| `ESCALA` / `radioUrbanoDe` | existían | **se borran**: el espacio local y el del mapa son independientes (Doc 1.0a). La ciudad usa su propio `radioCiudad` (30, +5 por edificio, máximos 60-180; `Trazado_Urbano.md`) |
| `MANTENIMIENTO.escalaDistancia` | 400 | **80** |
| `LINEAS_PRODUCCION` | 25 / 150 / 90 | **se elimina** (decisión previa del autor) |
| `RECURSO_RAREZA.espacioMinimo` | 20 / 40 / 120 | **4 / 7 / 20** |
| `LIVESTOCK.espacioMinimo` | 30 | **5** |
| Ritmo de niveles 2-3 | — | aprobado (Doc 4.5); niveles 4-5 con las Eras IV-V, pospuestos |

