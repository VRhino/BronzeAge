# Lo pendiente para el cliente jugador

Los cambios del motor que el cliente jugador (BronzeAgeClient) tiene que recoger se apuntan aquí, en vez de avisar a su hilo con cada uno, y se le pasan todos de golpe (decisión del autor, 2026-11-11).

**Cómo se usa.** Cada tanda del motor que toque algo que el cliente lee o muestra añade una entrada en «Pendiente», con la fecha, el commit o PR del motor, qué cambia en el contrato o en los datos y qué tendría que hacer el cliente. Cuando el autor decide pasárselo, se manda al hilo del cliente la lista entera y las entradas pasan a «Ya pasado», con la fecha. La regla de interfaz manda en todo: se muestra solo lo que el jugador puede hacer.

## Pendiente

### Formaciones, +3 % por nivel y batallas en Unity (2026-11-11, PR #18, main e89f8d3)

Partida nueva: la foto pasa a `BAN62`.

1. **La formación de cada escuadra.**
   - `Escuadron.formacion` (campo 18, en `HEROE`): la familia elegida, por su id. Toda escuadra nace en `reposo`.
   - El catálogo es el bloque `FORMACIONES` de `Heroe_Y_Tropa` en `GET /v1/datos`. Cada familia trae `id`, `nombre` y `tecnologia` (`null` si no pide ninguna): `reposo`, `linea`, `columna` y `dispersa` sin tecnología, y `muro_escudos`, `testudo`, `cuadro` y `cuna` con `disciplina_formacion`. La de reposo es cómo forma cada tropa al seguir al héroe o pararse, y está siempre.
   - La orden `elegir_formacion` (`OrdenElegirFormacion { escuadron_id, formacion }`, 191 en `orders.proto`). La da la cuenta del héroe, esté donde esté. No emite hechos: la escuadra llega con su `formacion` nueva en `HEROE`.
   - Rechazos: `formacion.desconocida`, `formacion.sin_tecnologia`, `escuadra.ajena`, y `batalla.bloqueado` mientras la escuadra combate. La misma que ya tiene da `sin_cambios`.
   - Qué haría el cliente: en la ficha de la escuadra, la formación actual y un selector solo con las permitidas. Las permitidas son la de reposo y las que no piden tecnología, más las que pide `disciplina_formacion` si la Facción la tiene (`TecnologiasFaccion`, en `FACCION`). Un héroe sin Facción solo tiene las que no piden tecnología. Sin selector mientras la escuadra está en batalla. Referencia: Heroe_Y_Tropa §10.4.
2. **El nivel de escuadra da +3 % de poder por nivel** en el combate con números (antes +1 %). Solo si el cliente estima o enseña ese poder.
3. **Partidas con las batallas en Unity.** Una partida creada con «batallas en Unity» (`BRONZEAGE_BATALLAS_EN_UNITY=1`; en `GET /v1/datos`, `reglas.batallas_en_unity`) no resuelve `atacar` con números. Abre una `Batalla` que juega el lanzador, y con el servidor de batalla de mentira se aplica a los 30 s: gana el defensor y nadie muere. Para el cliente:
   - La batalla en curso y su resultado llegan por `BATALLA` (a quien combate) y por `FACCION` (resumida).
   - La columna queda `en_batalla` y sus órdenes dan `batalla.bloqueado` hasta que se aplica.
   - `asignacion_propia` (instancia y token) es para el cliente Unity: el cliente jugador no se conecta a la batalla, solo la sigue.
   - Referencia: Batalla_Unity_Ciclo §2-§8.
4. Las respuestas de `POST /v1/registro` y `POST /v1/sesiones` traen además `servidor` (cuenta de servidor de batalla): el cliente lo puede ignorar.

### Rescate de la rama de la revisión: fundar, plaza ajena desde dentro, escolta y recetas (2026-11-11, main ebfc4354)

Sin cambios en el contrato ni partida nueva: cambia lo que el motor manda y lo que acepta.

1. **La plaza ajena vista desde dentro.** Quien está dentro de una plaza ajena recibe en `VISTA` (`vista::plaza_desde_dentro`), además del trazado y las órdenes de mercado, **sus edificios en el suelo**: en pie, en obra y los dañados de la cola (la ruina que deja una conquista), cada uno con `completa_en_ms`, `mejora`, `danado`, `buey`, `ancho` y `alto`; y **sus recintos de muralla** (`recintos`: anillo, `avance` y mejora, sin `siguiente_celda_en_ms`). No llegan la cola sin empezar, la prioridad, la pausa, la fuente ni la receta. Qué haría el cliente: pintar esos edificios y murallas al recorrer la plaza ajena, como pinta los de la propia. Referencia: Niebla_De_Guerra §5.4, Proyecciones_Y_Autorizacion.
2. **La columna que funda entra en su plaza.** Al fundar, la columna ya no se queda fuera: una columna de un solo héroe entra como con `entrarEnAsentamiento`, y un ejército se desarma y cada héroe entra como lo haría solo. Cada entrada emite `jugador.entra_en_asentamiento`. Qué haría el cliente: tras `fundar`, llevar al jugador al interior de la plaza nueva en lugar de dejarlo en el mapa. Referencia: Fundacion_Y_Refundacion §5.
3. **Escolta del carro de acarreo.** El carro de acarreo admite **100** de escolta si la plaza madre no tiene Mercado (con Mercado, el cupo de su nivel); antes el motor daba 0. Qué haría el cliente: si calcula el cupo de escolta por su cuenta, dar 100 sin Mercado. Referencia: Comercio_Caravanas, el carro de acarreo («Batalla» y «Escolta»).
4. **Recetas con requisito de edificio.** Una receta con `requiereEdificio` (en `GET /v1/datos`, catálogo de Economia_Construccion) no se fabrica si la plaza no tiene ese edificio activo con ese nivel; hoy, el Arma de Hierro de Calidad pide una Fundición de nivel 3. Qué haría el cliente: en el taller, decir qué edificio le falta a esa receta. Referencia: Economia_Construccion §5.3.

## Ya pasado

Nada todavía.
