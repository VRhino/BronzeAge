# Campamentos de mercenarios

> **Reescrito el 2026-10-13** para el mundo reactivo: sin tick, la retirada de la tropa prestada ocurre al cambiar la residencia (Doc 5.15.2b: la tropa prestada vuelve sin viaje al campamento de su héroe). Los campamentos están a 600 unos de otros (Doc 1.9c). El «cómo funcionaba» está en `typescript-final:Docs/Mechanics/Campamentos_Mercenarios.md`.

## 1. Alcance y fuentes

Un campamento de mercenarios es un **enclave neutral** del mundo abierto: sin Facción, sin zona de influencia, que no crece ni desaparece y al que se puede entrar, residir, comprar, reclutar y refundar. Su mecánica está repartida en varios documentos; este es el **índice** de todas
ellas (§3) y el lugar donde se describe lo que aún no tenía documento: la **población de reclutas**, el **reclutamiento**, la **tropa prestada** y la **planta** del campamento.

- **Verificado contra:** `6a71770` (2026-10-12).
- **Fuentes de lo nuevo:** `engine/reclutamientoMercenario.ts`, `engine/layoutCampamento.ts`, `session/proyecciones/escenaCampamento.ts`, `session/comandos/mercenarios.ts` (`reclutarEnCampamento`, `pedirPrestamo`, `reponerPrestamo`, `exigirEn`), `engine/simulation.ts` (`sinPrestamosAjenos`, al cambiar la residencia del héroe).
- **Canon:** [`Docs/Game/1_Sistema_de_Mundo_y_Territorio.md`](../Game/1_Sistema_de_Mundo_y_Territorio.md) 1.9b, Doc 2.5 (residencia), Doc 5.8 (reclutar).

> **Implementado en Rust (2026-10-19, fase 4A-1: reglas puras).** Crate `crates/militar`, módulo `mercenarios`: los reclutas del campamento (§4: valor e instante, la cantidad de ahora se calcula al mirar), las tropas que ofrece (§5.1; la tecnología del campamento llega como una función, es del sistema 6), el precio (§5.2, redondeado hacia arriba **una vez sobre el total**), reclutar (§5.3, con «se une a la columna» si cabe en el Liderazgo) y la tropa prestada (§6: pedir, reponer, retirar). Quien paga (el almacén personal o el carro) lo descuenta el llamador. La planta, el mercado y el almacén personal no están en este crate.
>
> **Implementado en Rust (2026-10-20, fase 4B-1: el campamento real en el núcleo).** `crates/nucleo/src/nucleo/mercenarios.rs` sustituye el andamiaje de 3B (la columna hacía de héroe) por el héroe: reclutar, comprar, repostar, el préstamo y la residencia son órdenes de un **héroe**, que debe estar dentro del campamento (`ubicacion = mercenarios`) o con su columna a `radioPuerta` (2) de su puerta (`campamento.fuera_de_la_puerta`).

> **Implementado en Rust (2026-10-23, fase 4H T3c): el campamento visto desde dentro por la sesión.** El ámbito `CAMPAMENTO:camp-N` lleva el `CampamentoMercenarios` completo (`proyeccion::interior::campamento`): los reclutas como un ritmo (valor e instante, tope por viviendas), el stock en venta, los contadores y la próxima reposición del mercado, los residentes y los presentes (residentes y visitantes), las escuadras prestadas, las tropas que ofrece y el tope diario por bien; lo comprado hoy y el almacén personal de cada héroe van en su `Heroe`. Se marca el campamento cuando cambia el stock, los reclutas o quién vive, está o tiene tropa prestada allí, y al cambiar las tecnologías. La planta del campamento (§7) viaja desde la paridad P7 (nota de §7).

> **Implementado en Rust (2026-10-20, fase 4B-2: el campamento con las columnas).** `SalirDelCampamento` (71) y `EntrarEnCampamento` (75) cierran el diagrama. **Salir** lo comprueba todo antes de tocar nada (`validar_salida`, `Movimiento_Ejercitos.md` §3): quien sale con otra política que `rechazar` lo hace como ejército; el **residente** elige tropa y carga de su almacén personal; **la ración gratis** (§8) llena sus víveres si pasaron `racion.cadaMinutos` (30) desde la última (`Heroe.racion_en`; no se acumula) y, si no, sale con los que lleva; el que está **de visita** sale con la columna con la que entró. **Entrar** devuelve la tropa al campamento y los víveres pasan al héroe; el carro, **desde el 2026-10-22 (entrar con todo), no se vuelca solo en el almacén personal**: se queda con el héroe (`vaciarCarro` lo pasa al almacén, hasta donde quepa) y sale con él. Entrar ya no rechaza por caravanas adjuntas (viajan con la columna). Un **ejército** entra entero si lo ordena su Líder y se desarma. **Reclutar** o **reponer el préstamo** con la columna a la puerta **ya se une a la columna** (§5.3) si cabe en el Liderazgo, y la recompone; mudarse de campamento retira de las columnas la tropa prestada. Hechos: `jugador.{sale_de_campamento, entra_en_campamento}`, `ejercito.entra_en_campamento`.
> - **El campamento** sale del artefacto de mundo (su índice y su posición). Su **edificio militar** es `edificiosMilitares[huella mod 3]` con `huella = |⌊x⌋·31 + ⌊y⌋·17 + índice|` (§3 de Generación del mundo; nada se guarda); sus viviendas, las de `edificiosFijos` (10); sus **reclutas**, `militar::mercenarios::Reclutas` (valor e instante) solo para los campamentos que alguien ha tocado: el que falta está lleno y nada los escribe por minuto. `Nucleo::{tropas_del_campamento, reclutas_de_campamento}` lo consultan.
> - **`ReclutarEnCampamento { heroe, tropa, pagar_con }`** (código 51): el campamento es donde el héroe reside (`mercenarios.no_resides`); `pagar_con` = `AlmacenPersonal` (por defecto en el TypeScript) o `Carro` (la columna del héroe a la puerta: `heroe.sin_columna` si no va en una). Rechazos de las reglas: `mercenarios.{tropa_no_disponible, ya_la_tienes_fuera, al_tope, sin_reclutas}`, `mercenarios.sin_oro`, `campamento.fuera_de_la_puerta`. El precio usa el factor de reputación de la Facción del héroe (×1,5 con la reputación en el umbral bajo) y el descuento de la Facción sin plazas; un héroe sin Facción no tiene ninguno. Hecho `mercenarios.reclutado`.
> - **`PedirPrestamo { heroe, tropas }`** (52) y **`ReponerPrestamo { heroe }`** (53): `mercenarios.{prestamo_sin_tropas, no_se_presta, prestamo_repetida, ya_prestada, nada_que_reponer}`; hechos `mercenarios.prestamo` y `mercenarios.prestamo_repuesto`. Dejar de residir en el campamento retira la prestada de donde esté (`mercenarios.prestamo_retirado`).
> - **`ComprarEnCampamento` / `RepostarEnCampamento`** toman `heroe`: el residente (cuya residencia es ese campamento) paga de su almacén personal y compra con su cupo diario; el visitante, con el carro de su columna (`heroe.sin_columna` si no va en una). Repostar llena los víveres de la columna del héroe o, sin columna, los suyos (`capacidadViveresPorHeroe`).
> - **Tecnologías del campamento (§5.1, Tecnologia_Aedas §5), provisional:** las de arranque siempre; las demás cuando las tienen adoptadas al menos `porcentajeFacciones` (0,5) de las Facciones **vivas** (las que conservan alguna plaza) según `FijarTecnologias`. El plazo de `horasTrasLaPrimera` (72 h) no existe hasta el sistema de tecnología.
> - **Provisional:** la escuadra nueva de un héroe con columna a la puerta **nace en el campamento** (no «se une a la columna»: la columna no guarda aún sus escuadras, 4B-2); `FijarResidencia` y `DarAlmacenPersonal` (administración) siguen siendo el único modo de residir y de llenar el almacén personal hasta `cambiarResidencia`/`residirEnCampamento` y `guardarEnAlmacenPersonal` *(esta, hecha en 5G-a, 2026-10-24: orden 138, desde el carro de la columna a la puerta de la residencia; Movimiento_Ejercitos §9)*; no hay ración gratis al salir del campamento (4B-2).


| Campo | Significado |
|---|---|
| `id` | `mercenarios-{i}` (orden de aceptación al colocarlos). |
| `posicion` | Punto en tierra firme; pegado a un bosque si lo hay cerca (Generación del mundo §11). |
| `origen` | Variante de aspecto `0‥MERCENARIOS.origenes−1`; solo cambia cómo se ve. |
| `edificios` | `[taberna, vivienda ×10, mercado, un militar]` (barracón, galería de tiro o caballerizas, elegido por la posición). Fijo: no se construye. |
| `residentesIds` | Héroes que residen aquí (cualquier Facción, sin límite, aunque su Facción tenga plazas). |
| `eligieronComoInicial` | Cuántos héroes lo eligieron al nacer (solo crece). En Rust (paridad P6, 2026-10-27), `EstadoMilitar::eligieron` (foto `BAN41`; una `BAN40` se admite y empieza en 0), sumado por `CrearHeroe`; sale en `Bienvenida.campamentos_para_elegir` y en el `CampamentoMercenarios` del ámbito `CAMPAMENTO`. |
| `poblacion`, `poblacionEn` | Reclutas disponibles en ese instante (§4). |
| `mercado` | Stock en venta ([`Comercio_Caravanas.md`](Comercio_Caravanas.md) §9). |
| `fondos` | Fondo de refundación por héroe ([`Fundacion_Y_Refundacion.md`](Fundacion_Y_Refundacion.md) §9). En Rust (5G-b), `Estado::fundacion.fondos`; el contrato lo da por Facción. |
| `bandidosEn?` | Cuándo puede aparecer el siguiente campamento de bandidos de su anillo ([`Generacion_Del_Mundo.md`](Generacion_Del_Mundo.md) §13). |
| `creadoEn` | Instante de mundo en que se colocó (al crear el mundo). |

## 3. Dónde está cada cosa

| Aspecto | Documento |
|---|---|
| **Colocación** (puntos al azar, separación, pegado al bosque, ficha) | `Generacion_Del_Mundo.md` §11 |
| **Alijos** a su alrededor, abrirlos | `Generacion_Del_Mundo.md` §12 |
| **Bandidos del anillo** (cuántos, cuándo) | `Generacion_Del_Mundo.md` §13 |
| **Crear el héroe dentro**, residir, el huérfano que se acoge, ruina de una plaza | `Heroe_Y_Tropa.md` §3; [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) (residencia) |
| **Entrar, salir**, salir con todo lo que se entró, `vaciarCarro`, ración gratis al salir | [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md) §5; `Movimiento_Ejercitos.md` §3.3 |
| **Repostar** víveres gratis (cualquiera) | `Comercio_Caravanas.md` §9.2b |
| **Mercado** (qué vende, comprar, reponer), almacén personal | `Comercio_Caravanas.md` §9 |
| **Refundar** (fondo, aportar, comprar la caravana) | `Fundacion_Y_Refundacion.md` §9 |
| **Tecnología propia** del campamento (cuándo desbloquea cada una) | [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md) |
| **Aedas**, taberna e intel (Miradas, Informes) | `Tecnologia_Aedas.md`; [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md) |
| **Protección** (a `radioProteccion` nadie inicia un combate) | [`Combate.md`](Combate.md) §2 |
| **Reclutar**, **población**, **tropa prestada**, **planta** | este documento |

## 4. Población y reclutas

```
tope(c)      = (nº de viviendas en c.edificios) × MERCENARIOS.poblacionPorVivienda          ; 10 viviendas × 10 = 100
actual(c, t) = min( tope(c),  c.poblacion + horas(t − c.poblacionEn) × MERCENARIOS.poblacionPorHora )
               horas = max(0, t − poblacionEn) / 60 min
```

La población **no se escribe nunca por minuto**: lo guardado es el valor y el instante, y la cantidad de ahora se calcula al mirar. Nace llena (`poblacion = tope`). Al reclutar se guarda `poblacion = actual − cantidad` y `poblacionEn = ahora`. No come ni cuesta mantenimiento; añadir viviendas al layout subiría el tope sin tocar nada más.

## 5. Reclutar en el campamento (`reclutarEnCampamento`)

Solo **donde se reside**, y estando en él: dentro, o con la columna a la puerta (`exigirEn`: `ubicacion = mercenarios` en ese campamento, o la columna a `≤ radioPuerta`; si no, `campamento.lejos`). Comando `{ tropaId, pagarCon? }` con `pagarCon ∈ almacenPersonal (por defecto) | carro`.

### 5.1 Qué se puede reclutar (`tropasDelCampamento`)

Una tropa del catálogo `TROPAS_RECLUTABLES` está disponible si **todas** se cumplen:

1. Su edificio existe en el campamento: `barracon`, `galeriaDeTiro` o `caballerizas` (el Centro Urbano no existe aquí, así que **la leva de arranque no se recluta**) y el campamento lo tiene en `edificios`.
2. `tropa.nivelRequerido ≤ MERCENARIOS.nivelEdificios` (3: todas sus tropas, limitadas por la tecnología).
3. Su `tecnologia` está en las **tecnologías del campamento** (`tecnologiasDelCampamento`): las de arranque siempre; las demás, cuando la tiene al menos `MERCENARIOS.tecnologia.porcentajeFacciones` de las Facciones humanas **vivas** (las que conservan algún asentamiento; los bots cuentan igual) **o** han pasado `horasTrasLaPrimera` desde que la primera la desbloqueó, lo que llegue antes.
   Una tecnología que nadie ha desbloqueado (por hito, épica o compra) no está; sin Facciones vivas manda solo el plazo. Detalle en [`Tecnologia_Aedas.md`](Tecnologia_Aedas.md).

### 5.2 El precio (`precioPorSoldado`)

```
base     = RECLUTAMIENTO_ORO_POR_ESCALON[escalón] + (caballos ?? 0) × ORO_POR_CABALLO + (Σ unidades de equipo de tropa.costoEquipo) × MERCENARIOS.valorEquipoEnOro
conRecargo = base × MERCENARIOS.recargo × factorComisionPorReputacion(Facción)       ; la mala reputación (≤ −40) lo multiplica por 1,5
precio   = conRecargo                                    si la Facción tiene asentamientos (o el héroe no tiene Facción)
         = conRecargo × MERCENARIOS.descuentoSinAsentamientos   si la Facción no tiene ninguno
oro total = ceil(precio × cantidad)
```

El campamento **cobra todo en oro** (no recibe equipo fabricado): cada unidad de equipo vale `valorEquipoEnOro`. Para el héroe sin Facción no hay recargo por reputación ni descuento.

### 5.3 Condiciones y efecto, en orden

Todas lanzan `MercenariosInvalidoError`:

1. Reside en un campamento («solo se recluta en el campamento donde se reside») y el héroe existe.
2. La tropa está disponible (§5.1).
3. **Una escuadra por tropa**: si ya tiene la suya (no prestada) y no es reponible aquí (campamento, o columna a la puerta del campamento; nunca escolta ni fuera), rechaza.
4. `cantidad = unidadesPorDefecto − (existente?.cantidad ?? 0)`; si ≤ 0, ya está al tope.
5. `actual(campamento) ≥ cantidad` («no tiene reclutas suficientes»).
6. **Pago**: con `almacenPersonal`, `heroe.almacenPersonal.oro ≥ oro`; con `carro`, debe ser el Líder de una columna a `≤ radioPuerta` del campamento y llevar el oro en el carro. El oro se descuenta (la clave se borra si queda en 0).

Efecto: si la escuadra existe, `cantidad +=` (moral, nivel y experiencia no cambian); si no, nace `escuadron-{campamento.id}-{n}` en `contenedor: campamento` con la progresión inicial y `moral 100`.
**Se une a la columna**: si es nueva y su Líder tiene una columna a la puerta y la escuadra **cabe en su Liderazgo** contando lo que ya lleva fuera del campamento (`puedeLlevar(héroe, [fuera + nueva])`), nace directamente en esa columna (`contenedor: ejercito`, y se añade a `escuadronIds`) para no obligarle a entrar y volver a salir. Una escuadra que se repone no se mueve.
`poblacion = actual − cantidad`, `poblacionEn = ahora`. Evento `mercenarios.reclutado { campamentoId, heroeId, tropaId, cantidad, oro, seUne }`. Resultado `{ cantidad, oro, seUne }`.
A diferencia de reclutar en una plaza, **no cuenta contadores de tecnología** y **no exige reserva de trigo ni población de plaza**.

## 6. La tropa prestada

Gratis, para **quien reside** en el campamento, para aprender a usar tropa antes de tener la propia.

- **`pedirPrestamo { tropaIds }`** (estando en su campamento): al menos una; solo **leva comunal** (`tropa.tecnologia = leva_comunal`: milicia de lanceros, leñadores, granjeros; una, dos o las tres, sin repetir ids); rechaza una tropa que ya tiene prestada. Crea por cada una `prestada-{héroe}-{tropaId}` completa (`unidadesPorDefecto`), `moral 100`, en `contenedor: campamento`, con `prestada: { campamentoId }`. Evento `mercenarios.prestamo`.
- **No es tropa del héroe**: no gana experiencia (`conExperiencia` la deja igual), no cuenta para «una escuadra por tropa», no come mientras vive en el campamento (la ración es de la guarnición/campamento de plazas).
- **`reponerPrestamo`**: repone **gratis** hasta su tamaño completo (`tamanoPrestada` = `unidadesPorDefecto` de su tropa) las prestadas que estén en el campamento o en la columna a su puerta; rechaza si no hay nada que reponer. Evento `mercenarios.prestamo_repuesto { heroeId, repuestas }`.
- **Retirada** (`sinPrestamosAjenos`, paso 23 del tick): toda escuadra con `prestada` cuyo campamento ya no tiene al héroe en `residentesIds` se **retira** de donde esté —del héroe, de su columna (`escuadronIds`) y de una escolta (`escoltaIds`)—, narrada con `mercenarios.prestamo_retirado { heroeId, campamentoId, escuadras[{ escuadronId, tropaId, cantidad }] }` (asentamiento `''` = de nadie; solo lo recibe el héroe del payload). Sin ella, al jugador le desaparecían las escuadras sin explicación.

## 7. La planta del campamento (`layoutCampamento`)

> **Implementado en Rust (2026-10-27, paridad P7).** `nucleo::urbanismo::planta_de_campamento` (y `Nucleo::planta_de_campamento`), con el trazado de Rust y la receta de abajo: la taberna con la huella del Centro Urbano en el origen, el Mercado con sus 5 puestos, el militar y las diez viviendas, cada uno con su ancla de saturación si la necesita y sin gate de nivel, en un radio de 60 u. l.; la empalizada a una celda de margen con la puerta de dos celdas al sur. La semilla es la de la partida y el índice del campamento (`trazado::hash(semilla, "campamento", [i])`), no el id del TypeScript, así que la planta no coincide pieza a pieza con la suya. No se guarda. Viaja en el `CampamentoMercenarios` del ámbito `CAMPAMENTO` (`planta`, `trazado`, `empalizada`, `empalizada_puerta`).

Presentación pura: ningún número cambia lo que hace el campamento. Reutiliza el motor de trazado de asentamientos ([`Trazado_Urbano.md`](Trazado_Urbano.md)) **una sola vez**: el campamento no crece, no se mantiene, no se degrada ni sube de nivel. Función pura de `{ id, edificios }`: no se guarda y la misma entidad siempre da lo mismo.

```
perfil = resolverPerfil(id)                 ← el id del campamento es la semilla de su planta (cada uno tiene la suya)
si hay taberna: se coloca un «centroUrbano» en (0, 0)         ← la taberna es el ancla principal
para tipo en [mercado, barracón, galería de tiro, caballerizas, vivienda]:       ← en este orden
    para cada edificio de ese tipo del campamento:
        asegurarAncla(tipo): igual que la construcción pero sin cupo de nivel; si hace falta, crea un ancla nueva (plaza de armas, plaza, pozo, parque…)
        sitioParaTipo({ radioPotencial: MERCENARIOS.layout.radioLocal }, …)  → se coloca en el sitio que da el motor
        si es el mercado: coloca sus puestos de nivel MERCENARIOS.layout.nivelMercado (MERCADO_PUESTOS_POR_NIVEL[1] = [1,1,2,2,3] → 5 puestos)
trazado = trazadoParaAsentamiento(sin recintos) → huellas y calles
empalizada = un anillo de una celda a una celda de margen alrededor de todas las huellas y calles, con la puerta de dos celdas al sur
```

Resultado: `elementos` (edificios y, de relleno, `puestoMercado`, `plazaDeArmas`, `plaza`, `pozo`, `parque`), `calles` y `empalizada { muro, puerta }`. La empalizada es **decorativa**: dentro no hay combate.
Con las cifras de hoy, un campamento típico (taberna, mercado, barracón y diez viviendas) da 19 elementos —1 taberna, 1 mercado, 5 puestos, 1 plaza de armas, 1 barracón y 10 viviendas—, 23 tramos de calle y una puerta de 6×3 unidades.
**Para Conquest** (`escenaDeCampamento`): el mismo trazado en celdas (`unidadesPorCelda = REJILLA_ASENTAMIENTO.tamanoCelda`): `{ campamentoId, origen, layoutVersion: LAYOUT_VERSION, unidadesPorCelda, edificios[{ edificioId, tipo, posicion (centro), ancho, alto }], calles[], empalizada[celdas muro/puerta] }`.

## 8. Quién puede qué dentro

| Acción | Quién | Dónde |
|---|---|---|
| Residir | cualquier héroe, sin límite, cuesta el cooldown de residencia | `residirEnCampamento`, no exige estar allí |
| Entrar / salir | cualquiera con columna a la puerta (propio o ajeno) | [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md) §5 |
| Reclutar, tropa prestada, comprar con el almacén personal | **solo el residente** | dentro o con la columna a la puerta |
| Comprar de visita | cualquiera, con el oro del carro (sin trigo) | `Comercio_Caravanas.md` §9.2 |
| Repostar víveres | cualquiera, gratis | `Comercio_Caravanas.md` §9.2b |
| Aportar al fondo de refundación | ciudadano de una Facción **sin plazas**, dentro o con la columna a la puerta: del almacén personal residiendo allí, o del carro siendo su Líder (**implementado en Rust, 2026-10-24, 5G-b**: `aportarARefundacion` 153, `retirarDeRefundacion` 154, `comprarCaravanaDeRefundacion` 155; el fondo va por Facción en `CAMPAMENTO`) | `Fundacion_Y_Refundacion.md` §9 |
| Intel de la taberna | cualquier héroe **de una Facción** | `Niebla_De_Guerra.md` |
| Iniciar un combate a `radioProteccion` (10) | **nadie** (ni jugadores ni bandidos) | `Combate.md` §2 |
| Abrir un alijo | héroe de una Facción **sin asentamientos** o sin Facción | `Generacion_Del_Mundo.md` §12 |

## 9. Parámetros

<!-- parametros:MERCENARIOS -->
```json
{
  "separacion": 900,
  "margenBorde": 150,
  "intentosColocacion": 2000,
  "salSemilla": 1835364963,
  "racion": {
    "cadaMinutos": 30
  },
  "radioExclusionFundar": 100,
  "radioProteccion": 10,
  "origenes": 3,
  "edificiosFijos": [
    "taberna",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "vivienda",
    "mercado"
  ],
  "edificiosMilitares": [
    "barracon",
    "galeriaDeTiro",
    "caballerizas"
  ],
  "layout": {
    "radioLocal": 60,
    "nivelMercado": 1
  },
  "margenBosque": 100,
  "pegadoAlBorde": 10,
  "poblacionPorVivienda": 10,
  "poblacionPorHora": 10,
  "recargo": 1.5,
  "descuentoSinAsentamientos": 0.6,
  "nivelEdificios": 3,
  "valorEquipoEnOro": 8,
  "tecnologia": {
    "porcentajeFacciones": 0.5,
    "horasTrasLaPrimera": 72
  },
  "mercado": {
    "cadaHoras": 3,
    "proporcionRepone": 0.03,
    "topePorBien": 300,
    "stockInicial": 100,
    "pilas": {
      "madera": 300,
      "piedra": 60,
      "trigo": 300
    },
    "cupoDiario": {
      "madera": 60,
      "piedra": 10
    },
    "margen": 1.3
  },
  "refundacion": {
    "porcentajeCoste": 0.4
  }
}
```
<!-- /parametros -->

(`RECLUTAMIENTO_ORO_POR_ESCALON` y `ORO_POR_CABALLO` en [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md) §12; `REPUTACION.umbralBajoParaComision` (−40) y `factorComisionPorReputacionBaja` (1,5) en [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md); `MERCADO_PUESTOS_POR_NIVEL` en `constants.ts`.)

## 10. Invariantes

| Invariante | Test que lo vigila |
|---|---|
| Colocación: separación mínima, lejos del borde, misma seed ⇒ mismos, nunca sobre agua, pegados a un bosque | `engine/__tests__/mercenarios.test.ts` |
| Población: el tope son sus viviendas por lo que da cada una y recupera por hora; más viviendas, más tope; lo gastado se recupera sin ningún evento | `engine/__tests__/reclutamientoMercenario.test.ts` |
| Qué se recluta: edificio y tecnología; la tecnología del campamento por porcentaje de Facciones o por plazo; sin Facciones manda el plazo | `reclutamientoMercenario.test.ts` |
| Precio: oro del escalón, caballos y equipo × recargo; la mala reputación lo encarece; sin asentamientos lo abarata | `reclutamientoMercenario.test.ts` |
| Reclutar: crea la escuadra, cobra del almacén personal o del carro y gasta población; repone solo lo que falta; se une a la columna si cabe en el Liderazgo (contando lo que ya lleva fuera); rechaza no residente, tropa que no ofrece, sin oro, sin reclutas, al tope | `reclutamientoMercenario.test.ts` |
| Tropa prestada: se pide gratis una, dos o las tres, no gana experiencia; se repone gratis; al dejar de residir se retira, también de la columna, con evento | `session/__tests__/entrarSalirCampamento.test.ts`, `reclutamientoMercenario.test.ts` |
| Planta: todo colocado, con calles, sin solapes y dentro de la empalizada; determinista y distinta por campamento | `engine/__tests__/layoutCampamento.test.ts`, `contratos/v1/__tests__/contratos.test.ts` |
| Residir y reubicar de una ruina | `session/__tests__/residenciaMercenaria.test.ts` |
| Mercado de campamento | `engine/__tests__/mercadoMercenario.test.ts` |

## 11. Casos límite y decisiones que parecen raras

- **La población gastada no se paga con tiempo de tick**: se recupera al mirar, así que un campamento sin visitas durante días está lleno sin que haya corrido nada.
- **Reclutar aquí no pide reserva de trigo ni población de plaza**: el campamento cobra todo en oro y gasta solo sus reclutas; por eso es la vía para recomponerse de quien no alcanza una tropa por edificio o materiales.
- **Una Facción sin plazas paga menos** (`descuentoSinAsentamientos`) **y una de mala reputación paga más** (×1,5): los dos factores se multiplican.
- **La escuadra reclutada puede nacer ya en la columna** si cabe en el Liderazgo; si no cabe, nace en el campamento aunque el héroe tenga la columna a la puerta.
- **«Una escuadra por tropa»** cuenta también la que esté en la columna a la puerta (se repone allí); la prestada nunca cuenta.
- **El préstamo dura lo que dura la residencia**: mudarse (o quedarse sin casa) retira la tropa prestada en el mismo instante, aunque esté en una columna lejos.
- **El Centro Urbano no existe en el campamento**: la tropa de leva de arranque que sale de él no se recluta aquí (solo se **presta**).
- **La empalizada no defiende**: es decorado; la protección real es el radio `radioProteccion`, donde nadie inicia combates.

## 12. Divergencias

Ninguna abierta.

## Catálogo de eventos

Los códigos de los hechos de dominio de este documento (4H T5, 2026-10-23; contrato 05 §8). La prueba `crates/nucleo/tests/catalogo.rs` lee el bloque de cada documento de `Docs/Mechanics` y exige que todo código que emite el núcleo esté en alguno, y que los que el núcleo aún no emite figuren como paridad pendiente en `Docs/Mecanicas a desarrollar.md`. El núcleo los emite todos.

<!-- eventos -->
```json
[
  "mercenarios.prestamo", "mercenarios.prestamo_repuesto", "mercenarios.prestamo_retirado", "mercenarios.reclutado", "mercenarios.repostado"
]
```
<!-- /eventos -->
