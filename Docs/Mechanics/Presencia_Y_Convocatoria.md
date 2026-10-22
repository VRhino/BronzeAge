# Presencia y convocatoria

## 1. Alcance y fuentes

Cubre **dónde está un héroe** (los cuatro sitios posibles y cómo pasa de uno a otro), los comandos de **presencia** —salir al mundo, entrar y salir de una plaza o de un campamento, guarnecer, marchar, conectarse y desconectarse, fijar la puerta y vetar— y la **convocatoria**: un ejército en preparación *dentro* de una plaza o un campamento, antes de salir.

- **Reescrito el 2026-10-13** para el mundo reactivo (planes en vez de ruta por tick, desconexión como evento programado) y la calibración (`radioPuerta` = 2). El «cómo funcionaba» está en `typescript-final:Docs/Mechanics/Presencia_Y_Convocatoria.md`.
- **Fuentes:** `engine/ubicacion.ts`, `engine/presencia.ts`, `engine/convocatoria.ts`, `session/comandos/presencia.ts`, `session/comandos/convocatoria.ts`, `session/comandos/desarmar.ts`, `session/comandos/autorizacion.ts` (quién puede cada uno), `session/proyecciones/jugador.ts` (`convocatoriasDelLugar`), y del motor de ejércitos (`engine/ejercitos.ts`): `entrarEnCampamento`, `salirDelCampamento`, `marcharA`, `exigirPuedeGuarnecer`, `caravanasAlEntrar`.
- **Canon:** [`Docs/Game/1_Sistema_de_Mundo_y_Territorio.md`](../Game/1_Sistema_de_Mundo_y_Territorio.md) 1.10 (el héroe está situado), [`Docs/Game/5_Sistema_Militar_y_Combate.md`](../Game/5_Sistema_Militar_y_Combate.md) 5.12.1 y 5.14.5.
- **No está aquí** (está en su documento):
  - Qué hace por dentro `salirAlMundo` / `salirDelCampamento` / `movilizarEjercito` (elegir tropa, Liderazgo, cargar el carro, ruta) → [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §3; terminar la campaña, `caravanasAlEntrar`, inspeccionar → §11–§12; juntar y separar columnas, formar un ejército en campo → §10.
  - La **desconexión que aplica el evento programado** (`salirDelMundo`: cuándo, aplazamiento, qué pasa con la columna) → [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §7. Aquí solo los comandos que la piden y la cancelan.
  - La regla de la **puerta** (grupos, capas, exilio) → [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md); aquí, los dos comandos que la fijan.
  - La **niebla** que se graba al salir y la foto del interior → [`Niebla_De_Guerra.md`](Niebla_De_Guerra.md).
  - Dónde defiende un héroe que está dentro de una plaza cuando la atacan → [`Combate.md`](Combate.md) §3.2.

> **Implementado en Rust (2026-10-19, fase 4A-1: reglas puras).** Crate `crates/militar`: `presencia` (§2 y §7: los cuatro sitios, `desconectarse` con su retardo de 2 min 30 s, `conectarse` y qué hace el evento de salida: sale, espera al fin de la batalla o se aplaza hasta `topeAplazamiento` si le persiguen) y `convocatoria` (§9: convocar, unirse con `Aceptar` o `Preguntar`, peticiones de 10 s, cambiar la selección, separarse, cancelar y **partir todo o nada**). «Poder salir con lo elegido» (§9.2) lo simula el núcleo con la salida real y llega a `partir` como una función; el resto de §3-§6 y §8 (las transiciones concretas, la puerta y los vetos) es de la fase 4B y del sistema político.
>
> **Implementado en Rust (2026-10-20, fase 4B-1: presencia mínima).** Del diagrama de §2.1 solo está lo que no mueve columnas: el héroe **nace** en un campamento (`crearHeroe`, [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md) §3) y **se desconecta y se conecta** (§7). `Desconectarse { heroe }` (63) anota la salida a `ahora + retardoDesconexionMs` (2 min 30 s) y programa **un evento** (`Carga::SalidaDelMundo`, entidad = el héroe, prioridad 1, `PRIO_SALIDA`); `Conectarse { heroe }` (64) lo cancela subiendo la generación del héroe (sin eventos) o, si ya salió, lo devuelve al mundo (`heroe.vuelve_al_mundo`). Al vencer, `militar::presencia` decide (hoy ni se combate ni se persigue: sale; esperar al fin de una batalla y el aplazamiento de 5 min por persecución están en la regla y los dispararán 4C y 4D) y emite `heroe.sale_del_mundo`. Fuera del mundo solo se puede conectar (`heroe.fuera_del_mundo`).
> - **Provisional (resuelto en la 4B-2, nota siguiente):** las transiciones, `marcharA` del héroe y la convocatoria (§3-§6, §9). La administración colocaba al héroe con `ColocarHeroe` (67) mientras tanto y se queda como andamiaje (plaza, campamento o una columna, a la que se suma como participante). Falta la puerta y los vetos (§8, sistema político).
> - Una desconexión pendiente impide colocar al héroe en una columna (`heroe.desconectandose`).

> **Implementado en Rust (2026-10-20, fase 4B-2: presencia completa y convocatorias).** `crates/nucleo/src/nucleo/{salidas,uniones,convocatorias,columnas}.rs`; formato de foto **`BAN10`**. Resuelve lo «provisional» de la nota anterior, salvo la puerta y los vetos (sistema 5).
> - **Transiciones (§3-§5).** `SalirAlMundo` (69, solo desde tu residencia), `SalirDelCampamento` (71), `EntrarEnAsentamiento` (72), `SalirDeAsentamiento` (73), `Guarnecer` (74) y `EntrarEnCampamento` (75) son las órdenes del diagrama de §2.1; el detalle de la salida (los dos tiempos `validar_salida`/`aplicar_salida`), de la entrada y de lo que pasa con el carro, los víveres y la columna está en [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md). Entra en una plaza quien **reside** en ella o es **ciudadano de su Facción**; los aliados, los vetos y la puerta (`fijarPuerta`, §8) son del sistema político y no están (fila de paridad). `SalirDeAsentamiento` solo retoma la columna aparcada de una plaza ajena y se rechaza desde tu residencia (`salida.desde_tu_residencia`, como en el TypeScript); además exige estar **dentro de esa plaza** (`heroe.no_esta_en_la_plaza`, `columna.no_aparcada`), que el TypeScript no miraba (fila de paridad).
> - **Quien va en una convocatoria no sale por su cuenta (§9.5):** `heroe.en_convocatoria` (en el TypeScript, `movilizacion.invalida`).
> - **`MarcharA` y `Estacionar` (§6)** los da el Líder de la columna (o la administración); la orden de un participante que no manda es `no_autorizado`.
> - **Desconectarse con la columna en el mapa (§7).** Ya se admite. Al vencer el evento (`Carga::SalidaDelMundo`, prioridad 1) el héroe sale con sus escuadras (`Contenedor::Fuera`), su parte del carro (`desgajar`, a prorrata) y de los víveres, que guarda (`carro_fuera`, `Heroe::viveres`). Si era el Líder, el mando pasa al participante **conectado** más antiguo; si era el último, la columna se disuelve (sus caravanas vuelven solas). `Conectarse` hace nacer su columna personal nueva en el punto donde salió, con sus escuadras y su carro (`heroe.vuelve_al_mundo`).
> - **La convocatoria (§9)** vive en `EstadoMilitar.convocatorias` (ids del contador común, ausente = ninguna) y se guarda en la foto. Las seis órdenes son `ConvocarEjercito` (84), `UnirseAConvocatoria` (85), `ResponderPeticionDeConvocatoria` (86), `CambiarSeleccionDeConvocatoria` (87), `SepararseDeConvocatoria` (88), `CancelarConvocatoria` (89) y `PartirConvocatoria` (90); las reglas son de `militar::convocatoria` y el núcleo pone la comprobación «saldría con eso» (§9.2): **simula la salida real** de cada integrante (`validar_salida`) con lo que eligió, así que «saldría» y «sale» no pueden discrepar. Convocar, unirse, aceptar y cambiar la selección la rechazan si esa simulación falla; la convocatoria **no tiene caducidad** ni evento.
> - **Partir es todo o nada (§9.4).** Simula a todos los integrantes que siguen dentro del lugar, en orden, acumulando lo que gastan en la misma plaza (`Reservas`: el almacén lo comparten, lo que saca el primero no lo tiene el segundo). Si alguien no puede, no sale nadie (`convocatoria.no_puede_salir` con su motivo); si el Líder ya no está dentro, `convocatoria.lider_fuera`. Si todos pueden, el Líder sale como un ejército quieto en la puerta, con la política de la convocatoria, y los demás se suman a él con lo que eligieron. Un integrante **de visita** se suma con la columna que dejó aparcada, que ha de poder fundirse con la del Líder (personal, de la misma Facción, sin formación; si no, `columna.solo_una_personal`/`columna.otra_faccion`). Quien se fue del lugar (o del mundo) mientras tanto se queda fuera sin más.
> - **Único plazo:** la petición de unión de un integrante potencial con la política `Preguntar` (10 s, el silencio vale como «no»): un evento `CaducaPeticionDeConvocatoria` (prioridad 8, entidad = el solicitante) por petición, que se descarta al contestarla. Hechos: `convocatoria.{convocada, unido, union_pedida, union_rechazada, seleccion_cambiada, separado, cancelada, partio, peticion_caducada}`, `jugador.{sale_al_mundo, sale_de_campamento, sale_de_asentamiento, entra_en_asentamiento, entra_en_campamento}`, `ejercito.{movilizado, guarnece, entra_en_campamento}`.
> - **DUDA para el autor:** la convocatoria no tiene el «+30 s» de espera que tiene una batalla de Unity al convocar (§9.4 no lo pide). Resuelta en la 4E (2026-10-21): el «+30 s» es de la convocatoria del **asedio** ([`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §4) y la convocatoria de ejército sigue sin esperar nada.


```
UbicacionHeroe = asentamiento { asentamientoId }      dentro de una plaza
               | columna      { ejercitoId }          en el mapa, en su columna (personal o ejército)
               | mercenarios  { campamentoId }        dentro de un campamento de mercenarios
               | desconectado { punto }               fuera del mundo (solo si salió con su columna, Reloj_Y_Eventos §7)
```

Residir es otra cosa (ciudadanía, Doc 2.5); un héroe de campaña sigue residiendo en su plaza aunque esté en el camino. **Solo los comandos de esta sección escriben la ubicación** (más `fundar`, que deja a la columna dentro de la plaza recién levantada).

**Ubicación deducida** (`ubicacionDeducida`) cuando no hay registro (un héroe que nunca actuó, o una partida anterior a la mecánica): *la columna gana a la residencia* —si va en alguna columna, `columna`—; si no, `asentamiento` de la plaza donde reside; si no, `desconectado` en `(0, 0)` (un punto de relleno).
> **Implementado en Rust (2026-10-22, entrar con todo).** Decisión del autor ([`Consideraciones/Entrar_Con_Todo_Definicion.md`](../../Consideraciones/Entrar_Con_Todo_Definicion.md)): **desaparece la «columna aparcada a la puerta»** del TypeScript. A la puerta se está **fuera**, en el mapa, muy cerca, y no se defiende; **dentro** es haber hecho la interacción de entrar. Quien entra en un lugar fuera del mapa entra **con todo lo suyo** —tropa, carro, víveres y **todas** sus caravanas adjuntas, sean del origen que sean— y al salir se lo lleva. Foto **`BAN17`** (las plazas guardan su acceso). `crates/nucleo/src/nucleo/salidas.rs`; pruebas en `crates/nucleo/tests/entrar.rs`.
> - **De visita** (otra plaza; un campamento que no es su residencia) todo queda en su **columna**, que sigue existiendo pero ya no está en el mapa (no se la ve, ni se la persigue, ni se la ataca: sale de la rejilla de `encuentros`). La tropa de un visitante **come de sus víveres** como un ejército acampado y la plaza anfitriona no gasta trigo por sus visitas (solo da de comer a la guarnición de sus residentes). Salir (`SalirDeAsentamiento`, `SalirDelCampamento`) devuelve la columna al mapa tal cual; quien no trajo columna (lo colocó la administración) sale solo con sus víveres. `columna.no_aparcada` desaparece. A quien no trajo nada no se le deja sacar tropa ni carga de un lugar donde no reside (`salida.solo_de_tu_residencia`): su campamento y su almacén personal están en su residencia.
> - **En su residencia** la tropa vuelve al campamento y los víveres al héroe, pero el **carro no se vacía**: se queda con él (`HeroeNucleo::carro`, el campo de antes `carro_fuera`, que ya estaba en la foto) y al salir se lo lleva; capacidad de 500 u. del carro (`CAPACIDAD_CARRO`), contando lo que ya trae al volver a salir. `VaciarCarro { heroe }` (**118**) lo pasa al **almacén personal** hasta `ALMACEN_PERSONAL.capacidad`, recurso a recurso y por orden; lo que no cabe se queda en el carro (`almacen_personal.lleno` si no cabe nada; sin cambios si no hay carro; `carro.vaciar_solo_en_tu_residencia` fuera de ella). Ya no se vuelca solo ni en un campamento ni en una plaza. Hecho `heroe.vacia_el_carro`.
> - **Caravanas adjuntas**: entrar ya no rechaza por caravanas de otro origen (`caravana.ajena_al_lugar` desaparece) ni las aparca: viajan con la columna. Un ejército que entra (`guarnecer`, `entrarEnCampamento`) se desarma como antes y **su Líder se queda con la columna y las caravanas** (sigue siendo un ejército mientras las lleve); si es residente, la columna sigue dentro, sin tropa, solo para llevarlas, y el carro se queda en ella. Aparcar una caravana en su plaza (Doc 3.13.7) es una orden explícita (`AparcarCaravana`) que no cambia.
> - **Quién entra** (`FijarAcceso { asentamiento, aliados, neutrales, enemigos }`, **117**): el dueño de la plaza decide por **relación** quién puede entrar. La misma Facción y los residentes entran **siempre**. Aliados y vasallos, neutrales (también quien no tiene Facción) y enemigos (en guerra) según lo que fije; **por defecto, hasta el sistema político, entran todos menos los enemigos**. Los pueblos extractores y las avanzadas **heredan** el acceso de su plaza madre (`puede_entrar_en_pueblo`, `puede_entrar_en_avanzada`) y se entra en ellos con sus propias órdenes (nota siguiente). Autorización provisional: `manda_en` (administración o ciudadano de la Facción). **Cerrar la puerta echa a quien ya está dentro** y ya no puede entrar (nota siguiente). `EntrarEnAsentamiento` rechaza con `puerta.cerrada`; `Guarnecer` mira a **cada héroe** del ejército. El campamento de mercenarios no es de nadie y está abierto a todos. Hecho `asentamiento.acceso`.
> - **Defensa** ([`Combate.md`](Combate.md) §3.2, [`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §7.1): defienden los de dentro de la misma Facción, aliados o vasallos (con números, de oficio; por Unity solo entra de oficio la guarnición y ellos se unen con `UnirseABatalla`, tercera ronda); **neutrales y enemigos no defienden nunca**, aunque estén dentro, ni quien está a la puerta. **Si el lugar cae** (conquista o ruina) quien estaba dentro sin poder defender **sale al mapa con lo suyo sin perder nada** (con la columna con la que entró; el residente sin columna, con una nueva con su carro y sus víveres) y los defensores quedan sujetos al resultado. En una **ruina** salen todos los de dentro al mapa (antes quedaban en el campamento más cercano); el hecho `heroe.reubicado` lleva `alMapa`.
> - **Convocatorias**: quien parte lo hace con todo lo que trajo; el residente, con el carro que no vació; el de visita, con su columna entera, que se funde con la del Líder (`columna.solo_una_personal` si lleva caravanas o es un ejército).

> **Implementado en Rust (2026-10-22, entrar con todo, segunda ronda).** Foto **`BAN18`**: el héroe puede estar `Pueblo(id)` o `Avanzada(id)` (etiquetas 4 y 5 de la ubicación en `codec_militar`; BAN17 se rechaza con su motivo). `crates/nucleo/src/nucleo/dentro.rs`; pruebas en `crates/nucleo/tests/{entrar,pueblos,avanzadas}.rs`.
> - **Entrar y salir de un pueblo o de una avanzada**: `EntrarEnPueblo { pueblo, heroe }` (**119**), `SalirDePueblo` (**120**), `EntrarEnAvanzada { avanzada, heroe }` (**121**) y `SalirDeAvanzada` (**122**). Se entra **de visita con todo**, como en una plaza ajena (nadie reside en un pueblo ni en una avanzada, así que no hay rama de residente): la columna entera queda dentro, fuera del mapa, y come de sus víveres; al salir vuelve al mapa tal cual, a la puerta. Entra solo quien va en una columna personal; un ejército (o una personal con compañía) entra por orden de su Líder, se desarma y cada héroe entra como lo haría solo (como `entrarEnCampamento`). Rechazos, en este orden: `pueblo.desconocido` / `avanzada.desconocida` (una ruina sin dueño también), `pueblo.no_esta_activo` / `avanzada.no_operativa` (una obra o unas ruinas no tienen puerta), `no_autorizado`, `heroe.sin_columna`, `columna.en_formacion` / `columna.solo_el_lider`, `pueblo.fuera_de_la_puerta` / `avanzada.fuera_de_la_puerta` (a más de `radioPuerta`; la administración no lo mira) y `puerta.cerrada` (la plaza madre no abre su puerta a algún héroe del ejército: mismo acceso que la plaza, `puede_entrar_en_plaza`). Salir rechaza con `heroe.no_esta_en_el_pueblo` / `heroe.no_esta_en_la_avanzada`. Hechos `jugador.entra_en_pueblo`, `ejercito.entra_en_pueblo`, `jugador.sale_de_pueblo` y los de avanzada (`…_avanzada`), todos atribuidos a la plaza madre. Una batalla sobre el lugar los bloquea (`batalla.bloqueado`), también a quien está dentro (salvo para salir, y solo si no se unió a la defensa: cuarta ronda, nota al final).
> - **Cerrar la puerta expulsa**: al cambiar `FijarAcceso`, quien está dentro de la plaza, de sus pueblos o de sus avanzadas y ya no puede entrar según el acceso nuevo **sale al mapa al instante y con todo lo suyo** (su columna tal cual, a la puerta del sitio; si no trajo ninguna, nace una columna personal con su carro y sus víveres). La misma Facción y los residentes no se expulsan (`puede_entrar_en_plaza` siempre les abre). Una sola mutación; hecho `heroe.expulsado` por cada uno, con `lugar` (0 plaza, 1 pueblo, 2 avanzada), `id`, `ejercito` y `motivo` (0 puerta cerrada, 1 el lugar desaparece, 2 el lugar cambia de dueño). Abrir la puerta no saca a nadie. Mientras un pueblo o una avanzada de la plaza está en una batalla, `FijarAcceso` de un jugador se rechaza con `batalla.bloqueado` (la administración no lleva candado). Cierra la DUDA «cerrar la puerta no echa a quien ya está dentro».
> - **El lugar desaparece o cambia de dueño**: un pueblo destruido a mano, una avanzada destruida (a mano o por un ataque) o en ruinas, la plaza madre en ruinas (sus pueblos y avanzadas) vacían el lugar: todos los de dentro salen al mapa con lo suyo y sin perder nada (`heroe.expulsado`, motivo 1). La plaza madre conquistada (sus pueblos y avanzadas pasan al conquistador) es distinta desde la tercera ronda (nota siguiente): salen solo los que el acceso heredado ya no deja entrar.
> - **Convocatorias**: `lugar_de` no reconoce un pueblo ni una avanzada como lugar: no se convoca dentro de ellos.

> **Implementado en Rust (2026-10-22, entrar con todo, tercera ronda).** Foto `BAN18` (sin cambios de formato). `crates/nucleo/src/nucleo/dentro.rs` (`vaciar_tras_conquista`) y `salidas.rs` (`acceso_deja_pasar`); pruebas en `crates/nucleo/tests/{pueblos,avanzadas}.rs`.
> - **Conquista de la plaza madre**: sus pueblos y avanzadas pasan al conquistador con el **acceso heredado de la plaza** (en la tercera ronda era el que fijó el dueño anterior; desde la cuarta ronda el acceso de la plaza conquistada vuelve al de por defecto, ver la nota siguiente). De los que estaban dentro de ellos, **salen al mapa con todo lo suyo solo quienes ya no podrían entrar** con el dueño nuevo (sus enemigos y los que el acceso por defecto o el heredado cierran: se mira la relación de su Facción con la del conquistador, **sin** la excepción de la residencia, porque quien residía en la madre la pierde con la conquista); los demás —un aliado, un neutral si el acceso los admite— **se quedan dentro**. Hecho `heroe.expulsado` (`motivo` 2) solo por los que salen. Si la madre cae en ruinas salen todos (`motivo` 1). Cierra la `// DUDA` de la segunda ronda («salen todos»).
> - **Todo defensor derrotado pierde la mitad del carro**, residente o visitante amigo (ver [`Combate.md`](Combate.md) §9); en la conquista, el visitante que defendió ya no «sale con su columna sin perder el carro». Quien no defendió sigue saliendo sin perder nada.

> **Implementado en Rust (2026-10-22, entrar con todo, cuarta ronda).** Foto `BAN18` (sin estado nuevo). Pruebas en `crates/nucleo/tests/{batallas,pueblos,avanzadas}.rs`.
> - **Al conquistar una plaza su acceso vuelve al de por defecto** (todos menos enemigos, respecto al nuevo dueño: `Acceso::default()`, en `conquistar_plaza`); el nuevo dueño lo cambia luego con `FijarAcceso`. Sus pueblos y avanzadas lo heredan, así que la salida de quienes ya no podrían entrar (`vaciar_tras_conquista`, `acceso_deja_pasar`) se evalúa con ese acceso reiniciado: salen los enemigos del conquistador; el neutral que el dueño viejo habría echado se queda. Cierra la `// DUDA` de `conquistas.rs`.
> - **Durante un asedio, un saqueo o un ataque a una avanzada, quien está dentro y no se ha unido a la defensa solo puede salir**: `SalirDeAsentamiento`, `SalirDePueblo`, `SalirDeAvanzada` (y `SalirAlMundo` del residente) se admiten pese al candado de batalla (`Comando::es_salida_de_lugar`, `rechazo_por_batalla`); cualquier otra orden sigue con `batalla.bloqueado` hasta que acaba. Al salir queda fuera, en el mapa, con todo lo suyo; si es neutral o enemigo del dueño puede unirse al ataque desde fuera ([`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §7.1). Quien se unió a la defensa combate y sigue bloqueado. `SalirAlMundo` cuenta como salida (decisión del autor, quinta ronda, 2026-10-22). Detalle en [`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §5.1.
`estaEnAsentamiento(heroes, id, plaza, …)` = `(registrada ?? deducida)` es `asentamiento` de esa plaza.

### 2.1 El diagrama de transiciones

| De → a | Comando | Qué ocurre con la columna |
|---|---|---|
| (nada) → `mercenarios` | `crearHeroe` | No tiene columna: nace dentro, como residente. |
| `asentamiento` (su residencia) → `columna` | `salirAlMundo` (o `movilizarEjercito`) | Nace una columna con lo que elige. |
| `columna` → `asentamiento` (su residencia) | `entrarEnAsentamiento` | La columna **se deshace** dentro: la tropa al campamento, los víveres al héroe; el **carro se queda con él** (`vaciarCarro` lo pasa al almacén personal). |
| `columna` → `asentamiento` (otra) | `entrarEnAsentamiento` | Entra **con todo** (tropa, carro, víveres, caravanas): la columna queda **dentro**, fuera del mapa, si la puerta le abre. |
| `asentamiento` (otra) → `columna` | `salirDeAsentamiento` | Sale con todo lo que entró: la columna vuelve al mapa. |
| `columna` → `mercenarios` | `entrarEnCampamento` | En el propio, se deshace (el carro con él); en otro, la columna entra entera. |
| `mercenarios` → `columna` | `salirDelCampamento` | Residente: elige tropa y carga y se lleva el carro; visitante: sale con todo lo que entró. |
| `columna` (ejército) → `asentamiento` | `guarnecer` | El ejército se desarma en la puerta y cada héroe entra. |
| `columna` → `asentamiento` | `fundar` | La columna se deshace en la plaza que funda. |
| cualquiera → `desconectado` / de vuelta | `desconectarse` + tick / `conectarse` | Ver §7. |

No hay una salida de la residencia que no sea `salirAlMundo` (o `movilizarEjercito`, o `salirDelCampamento`): ahí se tiene el roster entero y el almacén, y hay algo que elegir.

## 3. Salir de la residencia (`salirAlMundo`)

Comando `{ asentamientoId, heroeId, escuadronIds, carga }`. Elegir tropa, validar Liderazgo y cargar el carro es la función de `Movimiento_Ejercitos.md` §3.2. Lo que añade el comando:

1. Crea la columna con id `ejercito-{ids.siguiente()}` y la añade; la plaza queda con la tropa fuera y la carga descontada.
2. **Llena los víveres** del héroe (`conViveresLlenos`), **después** de elegir la carga y con la tropa ya fuera (Movimiento §6.1).
3. **Toma la foto del interior** que deja atrás (`conFotoTomadaPor`): del asentamiento **ya sin** la tropa ni la carga que se lleva, guarda `{ vistoEn, almacen, cola (edificios no activos), guarnición }`. Lo `activo` no entra: es público y viaja vivo. Es lo que verá de esa plaza al volver a mirar de lejos.
4. `ubicacion = columna`. Evento `jugador.sale_al_mundo { ejercitoId, asentamientoId, heroeId, escuadronIds, liderazgoUsado, cargaTotal }` a la plaza.

Está envuelto en `sinConvocatoria` (§9): quien está en un ejército en preparación no sale por su cuenta.

## 4. Entrar y salir de una plaza

### 4.1 `entrarEnAsentamiento { asentamientoId, heroeId }` (`cruzarLaPuerta`)

Exige tener columna (`jugador.sin_columna`). Cuatro condiciones, en este orden (`MovilizacionInvalidaError`):

1. **La columna es personal**: «Vas en un ejército: hay que separarse antes de entrar en una plaza». (Un ejército entra por `guarnecer`, §4.3.)
2. **En la puerta**: `distancia(columna, plaza) ≤ MOVIMIENTO.radioPuerta` (**2** calibrado; 10 en el bloque TypeScript). Entrar es un acto, no un roce.
3. **Con permiso**: `puedeEntrarEn(plaza, héroe, columna.faccionId, relaciones)`: un residente entra siempre; la propia Facción nunca se bloquea; el resto, según lo que el dueño haya fijado para su relación (aliados y vasallos, neutrales, enemigos; por defecto todos menos los enemigos, `fijarAcceso`); el veto individual y los grupos (`fijarPuerta`) son del sistema político. Detalle en [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md) (`PUERTA.cerradaAPorDefecto`).
4. **Residencia o no**:
   - **Su residencia**: la columna **se deshace** (`absorberColumna`): las escuadras vuelven al campamento (`contenedor: campamento`), los víveres al héroe y el **carro se queda con él** (decidido el 2026-10-22: antes se volcaba entero en el almacén de la plaza, y lo que no cabía se perdía); la columna desaparece salvo que lleve caravanas adjuntas.
   - **Cualquier otra**: entra **con todo**: la columna queda **dentro**, fuera del mapa, intacta (tropa, carro, víveres y caravanas).

Después `ubicacion = asentamiento`. Evento `jugador.entra_en_asentamiento`. De la segunda rama sale gratis un caso: si conquistan la plaza (o la arruinan) mientras estás dentro, sales al mapa con tu columna y sin perder nada.

### 4.2 `salirDeAsentamiento { asentamientoId, heroeId }`

Sale de una plaza **ajena** con todo lo que entró (su columna vuelve al mapa), sin pantalla de equipamiento. Rechaza **a propósito** desde la propia residencia (manda a `salirAlMundo`). Exige tener columna. Pone `ubicacion = columna` y emite `jugador.sale_de_asentamiento`. No comprueba la geometría ni que el héroe estuviera dentro de *esa* plaza: lo único que mira de la plaza es si reside en ella. Envuelto en `sinConvocatoria`.

### 4.3 `guarnecer { asentamientoId, heroeId }` (un ejército vuelve a una plaza propia)

1. `exigirPuedeGuarnecer`: es un **ejército** (`tipo = ejercito`) a `≤ radioPuerta` de una plaza que le abre su puerta (a cada uno de sus héroes: la misma Facción siempre, el resto según el acceso); si no, error. (Antes solo guarnecía su propia Facción.)
2. `caravanasAlEntrar`: entra con **todas** sus caravanas adjuntas, sean de donde sean; **no se aparcan** (decidido el 2026-10-22): se quedan con la columna del Líder (Movimiento §11). (Antes solo entraba si todas eran de esa plaza y pasaban a `aparcada`.)
3. `desarmarAlEntrar`: **solo su Líder** hace entrar al ejército; se deshacen las formaciones: cada héroe queda en una columna personal (el Líder con el resto del carro, los demás con su parte a prorrata).
4. Cada héroe de la columna **entra como lo haría solo** (§4.1): el residente deja su tropa y su carro; el que no reside entra de visita con su columna a la puerta.
5. Eventos: uno `jugador.entra_en_asentamiento` por héroe y `ejercito.guarnece { asentamientoId, ejercitoId, escuadrones, caravanas }`.

## 5. Entrar y salir de un campamento de mercenarios

### 5.1 `entrarEnCampamento { campamentoId, heroeId }`

Exige columna. Si la columna es **personal con un solo participante**, entra él solo (`entrarEnCampamento` del motor); si no —un ejército, o una personal con compañía— entra el ejército entero, ordenado por su Líder:

- **Solo (motor)**: error si no es el Líder de una columna de un participante; error si no está a `≤ radioPuerta` (2) del campamento. `ubicacion = mercenarios`.
  - **No residente** (visitante): entra **con todo**; su columna queda dentro, fuera del mapa, intacta, sin tocar nada.
  - **Residente**: las escuadras vuelven al campamento (`alCampamento`) y el **carro se queda con él**: ya no se vuelca solo en el almacén personal (decidido el 2026-10-22; lo hace `vaciarCarro`, hasta `ALMACEN_PERSONAL.capacidad`, y lo que no cabe se queda en el carro).
- **Ejército**: debe estar a la puerta; `desarmarAlEntrar` (como `guarnecer`: solo el Líder, cada héroe en su columna personal, con **todas** las caravanas, que se quedan con la columna del Líder) y cada héroe entra como lo haría solo. Evento `ejercito.entra_en_campamento`.
Evento por héroe: `jugador.entra_en_campamento`.

### 5.2 `salirDelCampamento`

Ver [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §3.3 (el residente elige tropa y carga del almacén personal y recibe la ración gratis; el visitante retoma su columna). El comando ajusta la ubicación, el almacén personal, `racionEn` y los víveres del héroe y emite `jugador.sale_de_campamento`. Envuelto en `sinConvocatoria`.

## 6. Marchar (`marcharA`)

Fija o rectifica el destino de la columna **del que va en ella**, tantas veces como quiera:

- Un **ejército** lo dirige solo su Líder.
- Una formación (ejército aún por formar) no se mueve.
- El héroe debe tener `ubicacion = columna` (dentro de una plaza hay que salir antes).
- Destino `ObjetivoEjercito` (un asentamiento o un punto) → `calcularRuta` con los asentamientos como pasos de río; sin ruta por tierra, error. El agua no se cruza.
- Efecto: `estado = marchando`, `objetivo`, un **plan nuevo calculado desde la posición real en ese instante** ([`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §4.2: vértices con su instante de paso) y se **abandona la persecución** (marchar a un punto y perseguir a la vez no existe). El plan anterior queda invalidado (sube la generación).
- `rectifica = (la columna ya estaba marchando)`. Evento `jugador.marcha_fijada { ejercitoId, heroeId, objetivo, rectifica }`.

## 7. Conectarse y desconectarse

- **`desconectarse { heroeId }`**: si ya está `fuera` o ya tiene `desconectaEn`, no hace nada. Si no: `desconectaEn = ahora + PRESENCIA.retardoDesconexionMs` (150 000 ms = 2 min 30 s) y **se programa el evento de salida a ese instante**. Evento `jugador.se_desconecta`. El héroe sigue en el mundo esos dos minutos y medio, moviéndose como iba (su plan sigue).
- **`conectarse { heroeId }`**:
  - Si **no estaba `fuera`**: si tenía `desconectaEn`, lo **cancela** (sin eventos); si no, nada.
  - Si estaba `fuera`: `volverAlMundo`. Dentro de una plaza o un campamento solo recupera el control. Si salió con su columna, **reaparece en su punto** con una columna personal nueva (`ejercito-{ids.siguiente()}`): `tipo personal`, `estacionado`, `politicaDeUnion rechazar`, `liderId` él, `suministro` = su carro de `fuera`, `escuadronIds` = sus escuadras `contenedor: fuera` (que vuelven a `ejercito`), `faccionId` la de su ciudadanía (o `''`), `origenAsentamientoId` su residencia (o `''`). Evento `jugador.vuelve_al_mundo` atribuido a su residencia.
- **En plena batalla de Unity** los dos comandos **no llevan el candado** de batalla: `desconectarse` se anota (`desconectaEn`) pero el evento de salida no se aplica mientras el héroe combate —sale en cuanto acaba la batalla—, y `conectarse` la anula ([`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §5.1).
- **Fuera del mundo no se da ninguna orden**: la autorización rechaza todo menos `conectarse` con el motivo `fuera_del_mundo`.
- Quién lo aplica y cuándo: el evento programado ([`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §7). Lo pide el cliente al cerrarse, o el runner de bots al acabar su sesión.

## 8. Fijar la puerta y vetar

- **`fijarPuerta { asentamientoId, heroeId, cerradaA[] }`**: quien puede es el **Gobernador residente** de la plaza o el **Rey** de su Facción (no hace falta que esté presente); es **un único estado** (el último manda). Guarda `puertaCerradaA = grupos únicos`. No expira. Evento `asentamiento.puerta_fijada`.
- **`vetarJugador { asentamientoId, heroeId, vetadoId, vetar }`**: solo el **Gobernador residente**. Añade o quita a `vetadosIds` (ordenado). **A un residente no se le veta** (`PuertaInvalidaError`): eso es el exilio. Evento `asentamiento.veto`.
- La regla que decide quién entra (residente > veto > grupo > propia Facción) está en [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md).

## 9. La convocatoria (ejército en preparación)

Un héroe **dentro** de una plaza o un campamento convoca un ejército; otros de su Facción **que están dentro del mismo lugar** se le unen, cada uno con la tropa y la carga con las que saldría. **Nadie ha salido todavía**: las escuadras siguen en su campamento y el almacén intacto, así que cancelar o separarse no devuelve nada. Espera **sin caducidad**.

### 9.1 Modelo (`estado.convocatorias?`, ausente = `[]`)

```
Convocatoria {
  id: "convocatoria-{n}",  faccionId,  lugar: { tipo: asentamiento|campamento, id },
  liderId,  politicaDeUnion: aceptar | preguntar,                          (nunca «rechazar»)
  integrantes: [ { heroeId, unidoEn, escuadronIds[], carga{} } ],          (el Líder primero)
  peticiones?: [ { heroeId, pedidoEn, expiraEn, escuadronIds[], carga{} } ],
  creadaEn
}
```

`lugarDe(héroe)`: `asentamiento` si `ubicacion = asentamiento`, `campamento` si `mercenarios`, nada en una columna. `integrantesDentro(c, héroes)` = los integrantes que **siguen en el mismo lugar**: quien se fue o se desconectó no sale con los demás. Un héroe va como mucho en una convocatoria (`convocatoriaDe`).

### 9.2 La comprobación de «saldría con eso»

Convocar, unirse, aceptar una petición y cambiar la selección **simulan la salida real** del héroe sobre un estado que se descarta (`exigirQuePuedeSalir` → `salirDelLugar`), por el mismo camino que usará al partir:

| Lugar | Reside en él | No reside |
|---|---|---|
| Plaza | `salirAlMundo` con su selección | `salirDeAsentamiento` (su selección **se ignora**: sale con todo lo que entró, su columna) |
| Campamento | `salirDelCampamento` con su selección | `salirDelCampamento` (sale con su columna; la selección se ignora) |

Si no puede, falla con el motivo concreto: «{héroe} no puede salir con lo elegido: …». Reutilizar la salida garantiza que tropa, Liderazgo, carga y víveres se validen igual que al salir solo.

### 9.3 Los comandos

- **`convocarEjercito { heroeId, escuadronIds, carga, politicaDeUnion }`**: debe estar dentro de un lugar (si no, «se convoca desde dentro de una plaza o de un campamento»), tener Facción, no estar ya en una convocatoria y poder salir con lo elegido. Crea la convocatoria con él de Líder e integrante único. Evento `convocatoria.convocada`.
- **`unirseAConvocatoria { heroeId, convocatoriaId, … }`**: la convocatoria existe; el héroe está en el **mismo lugar** y es de la **misma Facción** (no vacía); no está ya en otra; y puede salir con lo elegido.
  - Política `aceptar`: se une ya (`convocatoria.unido`; quita su petición pendiente si la había). Devuelve `{ unido: true }`.
  - Política `preguntar`: deja una **petición** al Líder que vive `MOVIMIENTO.vidaPeticionUnionSegundos` (10 s); al anotar una nueva se barren las caducadas y la propia anterior. Evento `convocatoria.union_pedida { expiraEn }`; `{ unido: false }`.
- **`responderPeticionDeConvocatoria { heroeId, convocatoriaId, solicitanteId, aceptar }`**: solo el **Líder**; la petición debe existir y **no haber caducado** («el silencio cuenta como un no»). Rechazar la quita y avisa (`convocatoria.union_rechazada`). Aceptar **revalida todo** con la selección de la petición (sigue dentro, misma Facción, puede salir) y lo une (`convocatoria.unido`).
- **`cambiarSeleccionDeConvocatoria { heroeId, escuadronIds, carga }`**: cualquier integrante, también el Líder, cambia **solo la suya**; se valida como al unirse. El orden de `escuadronIds` es el orden con el que su columna sale y combate. Evento `convocatoria.seleccion_cambiada`.
- **`separarseDeConvocatoria`**: un integrante se va; sigue dentro del lugar. El **Líder no se separa: cancela**. Evento `convocatoria.separado`.
- **`cancelarConvocatoria`**: solo el Líder; la convocatoria desaparece y todos siguen dentro con todo en su sitio. Avisa también a quienes tenían una petición. Evento `convocatoria.cancelada`.
- **`partirConvocatoria`**: ver §9.4.

Todos exigen estar en una convocatoria (salvo convocar y unirse). Los eventos llevan `heroesIds` (Líder, protagonista, todos los integrantes y los avisados extra) para que lleguen al canal `heroe/<id>` de cada uno; se atribuyen a la plaza si el lugar es una plaza, a ninguna si es un campamento.

### 9.4 Partir (`partirConvocatoria`)

Solo el **Líder**. `dentro = integrantesDentro`; el **Líder debe ser el primero** de los que siguen dentro («El Líder ya no está dentro del lugar» si no).

```
para cada integrante de `dentro`, en orden:
    salirDelLugar(estado, …, su selección)        ← la salida REAL; si falla, falla TODO (ver abajo)
    anota (heroeId, ejercitoId de su columna)
base = la columna del Líder  →  se convierte en el ejército:
        tipo = ejercito, politicaDeUnion = la de la convocatoria,
        (sin origen: un ejército no vuelve solo a ningún sitio)
para cada otra columna:  unirseEnCampo(ejército, columna con la posición del ejército)   ← todas salen por la misma puerta
        se quita su columna; el héroe pasa a ubicacion = columna del ejército
la convocatoria se elimina
```

**Todo o nada**: si alguien ya no puede salir con lo que eligió, el comando entero se aborta (transacción) y nadie sale; el mensaje dice quién y por qué. Resultado `{ ejercitoId }`, evento `convocatoria.partio { ejercitoId }` más los eventos de cada salida. El ejército nace **quieto en la puerta** y lo dirige su Líder con `marcharA`. Los integrantes que ya no estaban dentro **quedan fuera** y no salen.

### 9.5 Quién no puede salir por su cuenta

`sinConvocatoria` envuelve `salirAlMundo`, `salirDeAsentamiento`, `salirDelCampamento`, `movilizarEjercito` y `unirseAEjercito`: si el héroe va en una convocatoria, rechazan con «Estás en un ejército en preparación: sepárate o cancélalo antes de salir por tu cuenta» (`movilizacion.invalida`). `partirConvocatoria` usa los manejadores **sin** envolver. Los demás comandos (`entrarEn*`, `desconectarse`, `fundar`, `guarnecer`) no se envuelven.

### 9.6 Lo que ve cada uno (proyección)

`convocatoriasDelLugar`: las convocatorias **de la propia Facción del héroe en el mismo lugar**, más la suya aunque ya no esté. Cada una trae sus integrantes dentro (con tropa nombrada, nivel, moral y cantidad, **en el orden elegido**) y las **peticiones vivas** solo para el Líder y para quien las hizo. `soyLider`, `soyIntegrante` y `miConvocatoriaId` se derivan por héroe. Nada sale del lugar.

## 10. Autorización

Todos son `rolesPermitidos: ['jugador']` y exigen `heroeId === actor` (nadie actúa a nombre de otro), salvo `crearHeroe` (el único de un actor sin héroe) y estos:

| Comando | Condición añadida |
|---|---|
| `salirAlMundo` | reside en la plaza y las escuadras elegidas son suyas |
| `fijarPuerta` | Gobernador residente de la plaza **o** Rey de su Facción |
| `vetarJugador` | Gobernador residente de la plaza |
| los de convocatoria | `heroeId === actor` (el resto lo valida el motor) |

Un héroe `fuera` del mundo: solo `conectarse` (§7).

## 11. Parámetros

<!-- parametros:PRESENCIA -->
```json
{
  "retardoDesconexionMs": 150000,
  "topeAplazamientoMs": 300000
}
```
<!-- /parametros -->

(`MOVIMIENTO.radioPuerta` y `vidaPeticionUnionSegundos` están en el bloque `MOVIMIENTO` de [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §14 (con su valor calibrado); `PUERTA` en [`Diplomacia_Facciones.md`](Diplomacia_Facciones.md).)

## 12. Invariantes

| Invariante | Test que lo vigila |
|---|---|
| `salirAlMundo`: saca con las tropas y la carga que elige, sin tropas también, rechaza carga que no cabe (tope sobre el total), no deja a la plaza sin la comida de su guarnición, no se sale dos veces | `session/__tests__/comandosPresencia.test.ts` |
| `entrarEnAsentamiento`: volver a la residencia deshace la columna (el carro se queda con el héroe); en otra entra con todo; no desde lejos; no en un ejército; quien no está en el mundo no tiene puerta | `comandosPresencia.test.ts`; en Rust, `columnas.rs` y `entrar.rs` |
| Entrar en un pueblo o una avanzada (119-122: rechazos, columna dentro fuera del mapa, víveres, ejército por su Líder, salir con todo), cerrar la puerta que expulsa de la plaza, el pueblo y la avanzada, destruir/ruina/conquista que vacían el lugar, foto `BAN18` y determinismo normal/acelerado/a trozos | `crates/nucleo/tests/{entrar,pueblos,avanzadas}.rs` |
| Entrar con todo (Rust): el visitante entra con su columna entera y come de sus víveres; las caravanas de otro origen entran y salen con el ejército; el residente no vacía el carro al entrar y `vaciarCarro` deja lo que no cabe; el acceso por relación y su herencia a pueblos y avanzadas; defensa de los de dentro (aliado sí, neutral y enemigo no, aliado a la puerta no); salida sin pérdidas al caer el lugar; foto y determinismo | `crates/nucleo/tests/{entrar,combate,batallas,pueblos,avanzadas,heroes}.rs` |
| `salirDeAsentamiento`: devuelve a su columna con lo que llevaba; se rechaza desde la propia residencia | `comandosPresencia.test.ts` |
| `guarnecer` vuelca la tropa, consume el ejército y deja al jugador dentro (con sus caravanas); una columna personal no guarnece | `comandosPresencia.test.ts` |
| `marcharA`: pone en marcha, se rectifica, lo dirige el Líder, sin destino sale quieto, no desde dentro de una plaza, no por mar | `comandosPresencia.test.ts` |
| La puerta la controlan Gobernador y Rey; por defecto deja entrar a los suyos; el veto pesa más que el grupo y se levanta con el mismo comando; un residente entra aunque esté cerrada y no se le veta | `comandosPresencia.test.ts`, `engine/__tests__/puerta.test.ts` |
| Campamentos: sale con tropa y carga, al volver se deshace (el carro se queda con el héroe y `vaciarCarro` lo pasa al almacén); otro campamento: entra con todo; lejos no se entra | `session/__tests__/entrarSalirCampamento.test.ts` |
| Desconectarse: dentro de la plaza no se mueve nada; en campo se lleva tropa y carro; se aplaza si le persiguen hasta el tope; el Líder cede el mando al conectado más antiguo; volver antes de 2:30 cancela | `session/__tests__/presenciaEnElMundo.test.ts` |
| Convocatoria en plaza: convocar no saca a nadie; parten como un ejército quieto; el Líder lo dirige; cancelar no mueve nada; separarse; política `preguntar`; no se sale por su cuenta ni se une a dos; no se convoca con carga que no podría salir; quien salió queda fuera al partir | `session/__tests__/convocatoria.test.ts` |
| Convocatoria en campamento; cambiar la selección (cada uno la suya, el orden es el de salida); la proyección; los avisos nombran a todos | `convocatoria.test.ts` |
| Ubicación: la grabada de exploración personal solo a quien está en columna sin bandera | `engine/__tests__/ubicacion.test.ts` |

**Sin test propio:** `ubicacionDeducida` y `estaEnAsentamiento` fuera de las pruebas de presencia, la caducidad de una petición de convocatoria, el reintento de `partirConvocatoria` cuando un integrante se queda sin Liderazgo, y la foto del interior al salir.

## 13. Casos límite y decisiones que parecen raras

- **Una convocatoria no caduca ni se limpia sola**: si su Líder se desconecta, desaparece de la plaza o es expulsado, sigue ahí hasta que él la cancele. Cancelar no mira el lugar: el Líder puede hacerlo desde donde esté, siempre que no esté `fuera` del mundo (entonces nadie puede, y la convocatoria espera a que vuelva).
- **Partir exige que el Líder siga dentro**; los demás pueden haberse ido (se quedan fuera) pero el Líder no.
- **La selección de un visitante se ignora al partir**: un no residente sale con su columna, la que trajo, elija lo que elija; si no trajo ninguna y elige tropa o carga, `salida.solo_de_tu_residencia` (en Rust; ya no hay `columna.no_aparcada`).
- ~~Entrar a tu residencia con carga que no cabe pierde lo que desborda la capacidad del almacén de la plaza~~ (TypeScript): **corregido el 2026-10-22**, el carro entra con el héroe y no se vuelca solo; no se pierde nada al entrar.
- **El almacén personal se llena en orden de recurso del carro** (`vaciarCarro`), no por prioridad: el último que no cabe es el que se queda en el carro.
- **`salirDeAsentamiento` no comprueba el lugar**: basta tener columna y no residir en esa plaza; es inocuo (la ubicación pasa a su columna) pero no valida que estuviera dentro.
- **El ejército entra por `guarnecer` a toda plaza que le abra la puerta** (antes solo a plazas propias); se desarma y cada héroe entra como lo haría solo. Una personal entra por `entrarEnAsentamiento`.
- **Desconectado no es ausente**: dentro de una plaza se queda donde está, defiende la guarnición y no él en persona (Combate §3.2); en el mapa, sale con su columna y no se le puede cazar.
- **`vetarJugador` solo lo hace el Gobernador, `fijarPuerta` también el Rey**: el Rey puede cerrar grupos pero no vetar a un jugador concreto.

## 14. Divergencias

Ninguna abierta.
