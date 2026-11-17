# Política y tecnología (sistemas 5 y 6): decisiones del autor (2026-10-24 a 2026-10-26)

Todas las decisiones del autor sobre las preguntas y las dudas de los sistemas 5 (política y Facciones) y 6 (tecnología y Aedas) de la fase 6, por sistema y tanda. Hasta el 2026-10-26 parte de ellas solo vivía en un apunte fuera del repositorio; este documento es ahora su único registro. El detalle de cada tanda (diseño, pruebas, cifras) está en `Docs/Arquitectura/Sistema5_Contraste_y_Plan.md` y `Docs/Arquitectura/Sistema6_Contraste_y_Plan.md`; las del sistema 4 y su calibración, en `Calibracion_Sistema4_Decisiones.md`; las de las `// DUDA:` del código de antes del sistema 5, en `Dudas_Del_Codigo_Decisiones.md`.

Leyenda: **CAMBIO** = la decisión cambió el código o una cifra; «como está» = se confirma lo que había y queda escrito como regla. «Técnica» = decisión técnica del agente que el autor aceptó (por medio del orquestador) sin cambiarla. Cada punto dice dónde quedó escrito (Mechanics, canon) y, si cambió código, dónde.

## Sistema 5: política y Facciones

### Preguntas del plan (2026-10-24)
- **Q1 Puerta:** la del canon: `fijarPuerta` por grupos (neutrales, aliados, enemigos y Aedas) y `vetarJugador`; por defecto **cerrada a neutrales y enemigos**; cerrar o vetar expulsa a quien ya no puede entrar. → Aplicado en 5E: `Diplomacia_Facciones.md` §4.3, canon 2.8; `nucleo/cargos.rs`, `politica::puerta`.
- **Q2 Atacar a un aliado:** es una **traición**: con `declara_guerra` rompe la alianza en el acto, cuesta −25 de reputación (en vez de −12) y declara la guerra con su arrastre. → 5A/5D: Dip §5.4, canon 2.7; `politica::relaciones::declarar_al_atacar`, `nucleo/diplomacia.rs`.
- **Q3 Tributo:** en el paso de minuto de cada plaza vasalla, que paga su parte a la capital de la señora; sin eventos nuevos; con la economía entera, con su resto oculto. → 5D: Dip §5.6; `nucleo/construccion.rs` (tributo al final del paso), `politica::tributo`.
- **Q4 Cargos locales:** hay que residir para ser designado; el cargo se vacía al dejar de residir. → 5E: Dip §3.4, canon 2.2; `nucleo/cargos.rs`.
- **Q5 Trueques en una anexión o fusión:** siguen; los que quedan entre plazas de la misma Facción dejan de mover la reputación. → 5F: Dip §6.3 (nota de 5F), `Comercio_Caravanas.md`; `nucleo/traslado.rs`.
- **Q6 Refundación, almacén personal y caravanas a mano:** entran en el sistema 5, en su tanda (5G); devuelve al oro de los bandidos su uso del TypeScript (M6). → 5G-a y 5G-b: `Fundacion_Y_Refundacion.md`, `Movimiento_Ejercitos.md` §9.
- **Q7 Dejar la Facción dentro de un ejército:** se rechaza; primero se separa, y si es el Líder cede antes el mando. → 5A/5B (y 5A-4): Dip §3.3, canon 2.5; `nucleo/ciudadania.rs`.
- **Q8 Ciudadanos para la nobleza:** todos los residentes (fundadores y quienes se mudaron). → 5B: `Economia_Construccion.md` §9.1, Dip §4.1.
- **Decisión técnica 9** (el andamiaje de administración queda como herramienta **sobre las reglas reales**; jugadores y bots usan solo las órdenes del contrato): aceptada. → `Comandos_Parametros.md` (orden `administracion`), Dip §1.
- **Orden de trabajo:** 5A en paralelo; antes de 5B, la calibración del sistema 4 y la economía entera; después 5B → 5I. Hecho.

### 5A, reglas puras (2026-10-24)
- **5A-1 Traición:** solo −25; no cobra además los −15 de «guerra tras romper». Como está. → Dip §5.4, canon 2.7.
- **5A-2 Tributo que no cabe en la capital:** se paga entero y la capital pierde el exceso, como el TypeScript. **CAMBIO.** → Dip §5.6, canon 2.4; `politica::tributo::pago` (`pagadas`/`recibidas`).
- **5A-3** (vasalla conquistada: se acaba cualquier relación activa con la conquistadora), **5A-5** (la marca de capital pasa tal cual en la fusión), **5A-6** (romper o rebelarse sobre una relación rota se rechaza, `NoActiva`): técnicas. → Dip §5.5 y §6.2.
- **5A-4 (Q7) Dejar la Facción yendo en un ejército** se rechaza **siempre**, también en uno de un solo héroe o en preparación. **CAMBIO.** → Dip §3.3, canon 2.5; `politica::faccion::Facciones::dejar(.., en_ejercito)`, `nucleo/ciudadania.rs`.
- **5A-7** `Propuesta.propuesta_por`: se decidió en 5H (el héroe que la hizo). → Dip §2.4 y nota de 5H.
- **5A-8** Quien recibe la ciudadanía de una Facción vacía pasa a ser su Rey. Como está. → Dip §3.2, canon 2.5.

### 5B, la Facción, la ciudadanía y la residencia (2026-10-24)
- **5B-1 Dejar la residencia o la Facción estando dentro:** las escuadras y el almacén personal pasan **sin viaje** a la residencia nueva; el héroe **sigue dentro como visitante** si la puerta le admite; si dejó la Facción y la puerta ya no le admite, sale expulsado al mapa solo, con su carro (`puerta_cerrada`). **CAMBIO.** → Dip §1 (correcciones de 5B), §3.3, §4.2; canon 2.5; `nucleo/ciudadania.rs`, `nucleo/dentro.rs::echar_si_ya_no_entra`.
- **5B-4 Campamento de destino** al dejar la residencia o la Facción: el más cercano **a la plaza que deja** (corrige el canon 2.5, que decía «a donde está»). **CAMBIO.** → Dip §4.2, canon 2.5.
- **5B-6 Pedir o aceptar el ingreso yendo en un ejército** se rechaza (`faccion.en_ejercito`); denegar sí se puede. **CAMBIO.** → Dip §3.2, canon 2.5; `politica::Facciones::{solicitar, responder}`.
- **5B-10 Mudarse desde una plaza en batalla** también se bloquea (`batalla.bloqueado`), además de hacia una. **CAMBIO.** → Dip §4.2, canon 2.5; `Nucleo::sin_batalla_en_su_plaza`.
- **5B-2, 5B-3, 5B-5, 5B-7, 5B-8, 5B-9, 5B-11, 5B-12 (+2,9 % del paso de minuto) y 5B-13:** técnicas. 5B-9 (`fundadores_ids`) y 5B-11 (`Heroe.faccion_id`) se hicieron en 5H.
- Las dos dudas que dejó la aplicación (dejar la Facción también se bloquea desde una plaza en batalla; la expulsión mira el lugar donde está el héroe, no solo la plaza que deja) se aceptaron con las técnicas de 5C (5C-9 y 5C-10, abajo).

### 5G-a, almacén personal y caravanas a mano (2026-10-24)
- **5G-a-1** `reclamarCaravana` sin caso de éxito hasta 5G-b (hecho en 5G-b).
- **5G-a-2** Guardar y sacar valen también **dentro** de la residencia, con el carro que el héroe trajo. **CAMBIO.** → `Movimiento_Ejercitos.md` §3.3.
- **5G-a-3** `cargarCaravana` solo de una plaza propia hasta 5D (desde 5D, también de una aliada que lo permita).
- **5G-a-4** Cada carro tiene un espacio fijo: la capacidad de carga suma solo el **hueco libre** de cada adjunta. **CAMBIO.** → canon 5.13.2, Movimiento §9; `Nucleo::capacidad_de_carga`.
- **5G-a-5** `soltarCaravana` y `aparcarCaravana` solo valen **dentro** de la plaza (`caravana.soltar_solo_dentro`); la última adjunta de un residente deshace su columna. **CAMBIO.** → Movimiento §9.

### 5C, la experiencia, la capital, la derrota y el nombre (2026-10-24)
- **5C-1 Capital efectiva:** la plaza que lleva **más tiempo en la Facción** (desde que es suya, no desde su fundación): conquistar no cambia la capital. **CAMBIO** (`de_la_faccion_desde`, foto `BAN32`). → Dip §3.7, canon 2.2; `politica::capital::PlazaDeCapital::desde`.
- **5C-2 Escala de la distancia a la capital:** se calibra en 5I (ver P1).
- **5C-3** Todo combate con números digno da experiencia de Facción sea cual sea el objetivo (pueblo, avanzada, caravana). **CAMBIO.** → Combate §4 y §7.1, Dip §3.5, canon 2.2.1; `Nucleo::experiencia_de_asalto`.
- **5C-4** Destruir un campamento de bandidos en Unity da **solo** la experiencia de bandidos. **CAMBIO.** → `Batalla_Unity_Ciclo.md` §9.6, Combate §7.2, canon 2.2.1.
- **5C-5** La Gran Fundición es una por Facción venga de donde venga. Como está. → Dip §3.7, canon 4.2.1.
- **5C-6 Renombrar:** como mucho 32 caracteres y único entre las plazas vivas. **CAMBIO.** → `Economia_Construccion.md` §12 (bloque `NOMBRE_ASENTAMIENTO`), canon 1.2; rechazos `asentamiento.nombre_largo` y `asentamiento.nombre_duplicado`.
- **5C-7 a 5C-11:** técnicas (5C-8: un hecho de experiencia por edificio está bien; 5C-9: dejar la Facción se bloquea también con la plaza de residencia en batalla; 5C-10: la expulsión al dejar la Facción mira dónde está el héroe). **5C-12:** no se reescribe historia.

### 5G-b, la refundación y la Caravana de Fundación (2026-10-24)
- **5G-b-4** La Caravana de Fundación se suelta a mano **dentro de su origen** (campamento o plaza donde se enganchó); queda disponible y vuelve a caducar a las 48 h. **CAMBIO.** → canon 1.8 y 5.13.2, Movimiento §9, Fundacion §7; `nucleo/fundacion.rs`.
- **5G-b-5** La que vuelve sola está en el mapa: se ve (`VISTA`, rejilla de encuentros) y se intercepta (Combate §7.1). **CAMBIO.**
- **Intercepción** (decisión del mismo día): en disputa quien gana se la lleva entera; en aguante pierde el 70 % (hacia abajo) y sigue su vuelta; con servidores de batalla abre una batalla de caravana en Unity como las comerciales. **CAMBIO.** → Combate §7.1, canon 1.8, Fundacion §7; `apertura_de_fundacion` en `nucleo/batallas.rs`.
- **5G-b-7** En `CAMPAMENTO` cada uno ve solo el fondo de refundación de su Facción; `ADMIN`, todos. **CAMBIO.** → Fundacion §9.1; `proyeccion/interior.rs`.
- **5G-b-11** Una Caravana de Fundación suelta en una plaza que cae (conquista o ruina) se **destruye** con ella. **CAMBIO.** → canon 1.8 y 3.13.7, Fundacion §7.
- **5G-b-1, 2, 3, 6, 8, 9, 10:** técnicas. La 9 (`Fundar` no deshace la columna dentro) sigue como fila de paridad.

### 5D, las relaciones entre Facciones (2026-10-25)
- **5D-1** Las relaciones activas son públicas para todos en `VISTA`, sin niebla. Como está. → Dip §1 (nota de 5D), `Proyecciones_Y_Autorizacion.md`.
- **5D-3** El tributo se paga al final del paso de minuto, cuando la vasalla ya ha comido. Como está. → Dip §1 (nota de 5D) y §5.6; `nucleo/construccion.rs`.
- **5D-6** La defensa del vasallo cuenta también en pueblos y avanzadas. Como está. → Dip §1 (nota de 5D, «defender o no al vasallo con números»).
- **5D-8** Las propuestas se proyectan en 5H. Como está (hecho en 5H).
- **5D-2, 4, 5, 7, 9, 10, 11:** técnicas (5D-2: solo se proyectan las relaciones activas). **5D-12:** no se reescribe historia.

### 5E, los cargos, la matriz de autorización y la puerta (2026-10-25)
- **5E-1** Colocar una avanzada lo da el Gobernador o el Maestro de Obras de la plaza madre, titular y residente, con su columna **en el sitio**. **CAMBIO.** → Proy §5.1, `Economia_Construccion.md` §14.1, canon 4.2.2; `Autoriza::CargoEnElSitio`.
- **5E-3** Quien funda una plaza (el Líder de la columna) es su **Gobernador de oficio**; el Rey puede cambiarlo. **CAMBIO.** → Dip §3.4, canon 2.2; `nucleo/zonas.rs` (`fundar`).
- **5E-4** Comprar intel en la taberna pide estar dentro. Como está. → `Niebla_De_Guerra.md` §6.
- **5E-12** El alias `FijarAcceso` se retira del contrato (hecho en 5H; el código 117 del registro ya no se lee).
- **5E-2, 5, 6, 7, 8, 9, 10, 11, 13 y 14:** técnicas (5E-10: el andamiaje de las pruebas antiguas se revisó en 5I).

### 5F, la anexión, la fusión, el traslado y los títulos (2026-10-25)
- **5F-1** Los títulos, la Liga y el Gran Rey se recalculan cada 10 min en su plazo. Como está. → Dip §7.
- **5F-3** No se acepta una anexión ni una fusión si algún héroe de las dos combate en una batalla abierta, como bando o incorporado. **CAMBIO.** → Dip §6.1-§6.2, canon 2.6; `Nucleo::union_en_batalla` (`nucleo/traslado.rs`).
- **5F-4** Si las dos tenían Gran Fundición, la resultante conserva las dos; el tope solo impide construir otra. Como está. → Dip §6.3; `nucleo/traslado.rs`.
- **5F-9** Quien sale de un lugar que ya no le admite tras el traslado lleva el motivo `cambia_de_dueno`. **CAMBIO.** → Dip §6.3, canon 2.6; `echar_si_ya_no_entra(.., motivo)`.
- **5F-2, 5, 6, 7, 8, 10, 11, 12:** técnicas.

### 5H, el contrato de la política (2026-10-25)
- **5A-7** decidido: `propuesta_por` es el héroe. **5E-12** aplicado: `FijarAcceso` retirado.
- **Las nueve dudas de 5H siguen sin decisión del autor** salvo lo que lleva la forma pública (abajo, 2026-10-26): coste por orden con 500 vistas, propuestas caducadas, etc.; ver `Docs/Mecanicas a desarrollar.md`, fila «Política (5H)».

### Calibración del batch político, 5I (2026-10-25)
- **P1 Distancia a la capital:** un reino sostiene **media región** (~300 u) alrededor de su capital: `1 + d/300` (×2 a 300 u, ×3 a 600 u, ×4,3 a 1.000 u). **CAMBIO de cifra** (80 → 300 u). → `Economia_Construccion.md` §10.1 y §15, canon 1.0a y 4.5; `asentamiento::mantenimiento`. Cifras medidas: `crates/batch/decisiones/politica.json` (P1) y `Docs/Arquitectura/Sistema5_Contraste_y_Plan.md` («Calibración del batch político»). Desde el 2026-10-26 la cifra está en el bloque `MANTENIMIENTO` (`escalaDistancia` 300) y el código la lee de él (decisión Q1 del sistema 4: los bloques se reescriben con lo calibrado).
- **P2 Títulos:** la aspirante tiene que superar a la poseedora en un **5 %**. **CAMBIO.** → Dip §7 (`TITULOS.margenAspirante`), canon 2.9; `politica::titulos::calcular`. Tras aplicar (2026-10-25): en «Facción más grande» el margen se aplica a las plazas y la población solo desempata (como está).
- **P3** Gran Fundición inalcanzable en 30 días: se deja (fase 7, batches de más semanas).
- **P4 Cupo de nivel 2** = dos tercios de las plazas de la Facción: `maxNivel2` [1,1,2,2,3,3,3,4,4,5] (`maxNivel3` igual). **CAMBIO de cifra.** → Dip §3.5, canon 2.2.1.
- **P5** La nobleza con residentes reales: se acepta.
- **P6** Reputación que no sube: fase 7 (con EE-7 y C1b).
- **P7** La caravana adjunta cuya plaza de origen cae en ruinas **se pierde** con su plaza. **CAMBIO.** → `Comercio_Caravanas.md` §10, Movimiento §9, canon 3.13; `comercio::ruina::cerrar_dependientes`, `Nucleo::perder_caravana_de_ruina`.
- **P8** Refundación en solitario lenta: cosa de bots, fase 7.
- **P9** Los batches militar y de comercio siguen con el andamiaje de ciudadanía y residencia.
- **P10** La millonésima de la reputación al reanclar: se deja.
- **Peajes de `batch comercio`** al 22 % con el cupo nuevo de nivel 2: se deja para la fase 7 (2026-10-25, tras aplicar).
- **`escalaDistancia` pasa al bloque** (2026-10-26, por P1 y Q1 del sistema 4): el bloque `MANTENIMIENTO` de `Economia_Construccion.md` dice 300 (antes 400, del TypeScript) y `ReglasMantenimiento::de` lo lee de él en vez de fijar 300 como valor vigente. Sin cambio de cifra efectiva; cambia la huella de reglas.

### `// DUDA:` del sistema 5 decididas al cerrar la fase 6 (2026-10-26)
- **Forma pública de una Facción ajena** (5H, duda 2): **solo el nivel**, sin la experiencia exacta y sin la capital. Como está. → `proyeccion/propios.rs::faccion_publica` (la DUDA pasa a regla).
- **Almacén personal de quien reside en una plaza** (Entrar con todo): «el almacén personal es siempre el mismo, solo se traslada»: una única capacidad (`ALMACEN_PERSONAL.capacidad`) viva donde viva, y viaja con la residencia. Como está. → canon 2.5 (ya lo decía), `Comercio_Caravanas.md` §9 (nota bajo el bloque `ALMACEN_PERSONAL`); `nucleo/salidas.rs` (la DUDA pasa a regla).
- **Líder no ciudadano que funda** (5E-3, 2026-10-26): para el autor el caso no puede darse («las columnas de ejércitos no pueden ser multifacción, y un ciudadano de la Facción A no puede cargar una caravana de fundación de la Facción B»). Al comprobarlo salió una vía real con órdenes del contrato: los fundadores se tomaban por orden de llegada y el recorte a `maxJugadoresFundacionGrupal` podía dejar fuera al Líder tras `CederLiderazgo`. **Arreglado** como ya decía Mechanics (Fundación §5.2, «el Líder primero»): los fundadores son el Líder, el titular y los demás, en ese orden (`nucleo/zonas.rs`, `fundar`), así que el Líder siempre es fundador y Gobernador de oficio. Solo la herramienta de administración `FijarFaccionDeHeroe` puede dejar un Líder de otra Facción; entonces la plaza nace sin Gobernador.

## Sistema 6: tecnología y Aedas

### Preguntas del plan (2026-10-24)
- **Q1 Orden:** todo el sistema 6 después del sistema 5 (tras 5I), sin intercalar. Hecho (6A-6H).
- **Q2 Crónica:** se guarda **todo el historial** de entradas públicas, en un almacén aparte de la foto, y quien llega tarde lo puede leer. **CAMBIO.** → TA §8 (nota de la infraestructura de 6C), contrato 05 §8 (`PedirCronica`/`Cronica`), canon 6.7 («Lore y crónica»); `persistencia::Diario`, `nucleo/cronica.rs`.
- **Q3 Umbrales de los logros (X) y velocidad de los Aedas:** se portan tal cual del TypeScript y **no se vuelven a medir hasta la fase 7** (bots reales). → TA §11 (bloques sin cambio), canon 6.3 y 6.6, `Docs/Mecanicas a desarrollar.md` («Para la fase 7»); `tecnologia::datos`.
- **Q4 Reclutas que cuentan para `reclutados.*`:** los de una plaza (reponer incluido) **y los de un campamento de mercenarios**. **CAMBIO** (divergencia aprobada del TypeScript). → TA §2.1 y §14, canon 6.3; `tecnologia::contador::reclutamiento`, llamado desde `nucleo/heroes.rs` y `nucleo/mercenarios.rs`.
- **Q5 Aeda itinerante en una plaza que le cierra la puerta:** sale al instante hacia su siguiente destino; lo revelado queda revelado. → TA §6.2, canon 6.7; `nucleo/aedas.rs` (`puerta_de_los_aedas`, `echar_aeda`).
- **Q6 Qué ve todo el mundo del avance:** los logros cumplidos y la Era, como el TypeScript; los contadores en curso, solo la administración. → TA §10 y nota de 6G, Proy §6.3, canon 6.4; `proyeccion` (`EstadoTecnologia` público en `HEROE`).
- Las 17 decisiones técnicas de la §7 del plan: aceptadas.

### 6A, reglas puras (2026-10-25)
- Sus tres dudas (comprar a un Aeda nunca crea `primeros`; la adopción no rechaza «sin Rey» en las reglas puras; la clave de una obra lleva la plaza) **no tienen decisión expresa del autor**: siguen como las dejó el agente (la primera, precisada en TA §13 en 6D) y están en la lista de dudas abiertas de `Docs/Mecanicas a desarrollar.md`.

### La crónica, infraestructura de 6C (2026-10-25)
- **Duda 4, fallo al escribir la crónica:** «es preferible como está»: el hilo de la persistencia deja de hacer fotos (aviso por `stderr`) en vez de abortar; al reiniciar se vuelve a la foto anterior y la repetición rehace lo que falte. → TA §8 (nota), `Docs/Mecanicas a desarrollar.md`; `crates/nucleo/src/nucleo.rs` (`hilo_de_persistencia`).
- **Duda 5, el registro de órdenes entero:** «lo más eficiente para el servidor»: `compactar` sigue borrando los segmentos anteriores a la foto más antigua; la historia anterior solo vive en el diario de la crónica. → TA §8; `persistencia`.
- Dudas 1-3 y 6-9: técnicas (el directorio va dentro de la partida; se guarda el `EventoDominio` codificado; escribe el hilo de la persistencia; una página puede pasar del límite; sin persistencia no se guarda; página y `Delta` se solapan).

### 6F, los alijos (2026-10-25)
- **1** Se abren solo desde el mapa y junto al alijo (`alijo.lejos` también sin columna en el mapa). Como está. → `Generacion_Del_Mundo.md` §12, `Comandos_Parametros.md`.
- **2** El hecho `alijo.abierto` solo lo oye quien lo abre. **CAMBIO** (antes, también los participantes de su columna). → Generacion §12; `nucleo/alijos.rs` (`heroes: vec![heroe]`).
- **3** Un id de alijo desconocido se rechaza en la traducción del contrato (`alijo.desconocido`, sin rastro). Como está.

### 6B, el estado, los contadores, los logros y las Eras (2026-10-25)
- **6B-1 Duración de las Eras:** cada Era dura un **mínimo de 1 semana** (`ERAS[..].minimoSemanas`) además del techo `plazoSemanas`; con todos sus logros pasa al cumplir el mínimo, nunca antes. **CAMBIO.** → TA §3 y §3.2 (bloque `ERAS`), canon 6.2; `tecnologia::datos::Era::minimo`, `EstadoTecnologia::proxima_era_en`.
- **6B-2** `primera_en_llegar` de la crónica sale de un índice derivado de plazas por **nivel alcanzado** (misma regla que el TypeScript, sin foto; D16 cerrada). **CAMBIO.** → `Espacio::plazas_por_alcanzado` (vigilado por la auditoría).
- **6B-3** La Caravana de Fundación capturada cuenta en `caravanas.destruidasOCapturadas`. Como está. → TA §2.1; `nucleo/fundacion.rs`.
- **6B-4** La fabricación con un taller real dentro del núcleo, a 6H (hecho: `crates/nucleo/tests/fabricacion.rs`).
- **6B-5** técnica (`tecnologia.logro` y `era.comienza` en la crónica, de 6C).

### 6C, la aparición, la adopción y los campamentos (2026-10-25)
- **Las seis dudas, como se recomendó:** (1) la aparición se evalúa al cerrar la versión, con todo lo que cambió la transición; (2) los Aedas cantan también al fijarse un logro o empezar una Era; (3) «el Rey en la capital» es estar **dentro**; (4) `recursoEnCapital` espera al siguiente paso de minuto de la capital; (5) `FijarTecnologias` programa las dos citas al registrar la descubridora; (6) la cita de campamento marca todos los campamentos. → TA §3 (nota de 6C), §4, §5; `nucleo/tecnologias.rs`.

### 6D, los Aedas itinerantes (2026-10-25)
- **Las ocho dudas, como se recomendó:** (1) el que llega a una plaza que le cerró la puerta mientras iba de camino no entra y sigue a la siguiente; (2) expulsado sin otra plaza que lo acepte queda varado y vuelve a buscar al cabo de una estancia; (3) su evento no arma el plazo de los títulos ni la cita de la Era; (4) el plan del viaje se fija al salir y no hay `pasosRio` en las plazas (fila de paridad, a medir con bots en la fase 7); (5) la revelación se hace al cerrar la versión; (6) con varios Aedas parados se compra al primero por número y revelan una vez; (7) los Aedas no son una clase del registro de cambios; (8) `precio` de `aedas.venta` en milésimas (convención del contrato: toda cantidad viaja ×1.000). → TA §6.2-§6.4 y nota de 6D, canon 6.7; `nucleo/aedas.rs`.

### 6E, los Aedas residentes y las épicas (2026-10-25)
- **1, 2, 3, 5, 6, 7 y 8, como están:** la espera siguiente empieza en el instante de la llegada; la ruina se lleva a los residentes sin hecho; la épica perdida por conquista la oye la Facción de antes; una Facción sin reputación guardada la tiene en 0; un `aeda_id` sin la forma `aeda-rN` sale `epica.no_vive_aqui`; las épicas se procesan antes que la aparición por hito; empezar o abandonar no lleva el candado de la plaza en batalla. → TA §7.1-§7.2 (notas de 6E).
- **4, dos épicas de la misma tecnología tras una anexión o fusión se unifican:** sigue la más avanzada (capítulos, luego hechos, luego la del Aeda que llegó antes); la otra se pierde con `motivo: unificada`. **CAMBIO.** → TA §7.2, canon 6.7 (épica); `tecnologia::residentes::Residentes::unificar`, llamada desde `nucleo/residentes.rs` (`trasladar_residentes`).

### 6G, el contrato y las proyecciones (decidido el 2026-10-26)
- **Las once dudas, como están** («todo ok»): (1) el Aeda itinerante viaja en `VISTA` con el tramo visible de su plan; (2) lo que no pasa por el registro (contadores e itinerantes en `ADMIN`, alijos en `HEROE`) va entero en la `Delta` de una reanudación con novedad y se da por sabido en una sin novedad; (3) los alijos a la vista se renuevan con el muestreo de la vista y tras cada reparto; (4) un héroe que deja de poder buscar alijos estando desconectado conserva los que tenía hasta su siguiente `Foto` de `HEROE`; (5) `cumplidas[i]` de una condición `recursoEnCapital` puede ir atrasada hasta el siguiente cambio de la Facción; (6) no viajan `eraDe` ni `capitalId`; (7) `Alijo.oro` es el del artefacto, en milésimas; (8) al fijarse un logro o empezar una Era se marcan todas las Facciones; (9) las `claves` de una épica solo van a `ADMIN`; (10) `aeda-N` y `aeda-rN` comparten la clase `AEDA`; (11) la forma pública lleva los dos bordes de la Era. → TA nota de 6G, Niebla §5.4 y nota de 6G, Proy §6.3; `proyeccion/`, `sesiones/ambitos.rs`, `nucleo/tecnologias.rs`.

### 6H, el batch tecnológico (decidido el 2026-10-26)
- **Las ocho dudas, como están** («todo ok»). **T1** (ninguna épica se cumple: el hito llega antes), **T2** (no se extrae estaño en este mundo con estos bots) y **T3** (alijos de ≈ 4 u de oro frente a 300 u del recluta de campamento más barato) quedan **para la fase 7**. **T4** (el Rey solo adopta dentro de la capital) se mantiene. Las otras cuatro: el Rey del batch de comercio pasa la corona al Gobernador de oficio de la capital para adoptar; `conHoplitas` y `conResidentes` no viajan en los hechos (el invariante de la doble cuenta solo los acota); el mínimo de una semana y el paso por todos los logros no se ejercitan en el batch (los cubren las pruebas del núcleo); `--escala-logros` es un experimento del batch. → `crates/batch/decisiones/tecnologia.json` (T1-T4 cerradas), `Docs/Mecanicas a desarrollar.md` («Para la fase 7»).

## Batch de territorio

### D3, el modo de estrés de fronteras (decidido el 2026-10-26)
- **Modo de estrés opcional**, fuera del escenario por defecto: `batch territorio --estres-fronteras 1` fija el nivel 3 de cada plaza con la orden de administración `FijarNivelAsentamiento` nada más fundarla; el informe lo marca en el título y en «Modo». Medido con estrés (400 Facciones, 30 días, semilla 42): 495 plazas, todas de nivel 3; 2,21 contactos de frontera por plaza (máximo 5), área perdida por recortes 7,5 % de media (máxima 26,5 %, verde bajo el 25 % de media), 0 plazas en zona ajena y separación mínima 30,012 u; huella 8b8fff82… igual en dos corridas. El escenario por defecto no cambia (huella 17903d5a…, la misma que antes del cambio). Ninguna invariante de frontera se viola. → `crates/batch/decisiones/territorio.json` (D3 cerrada), `crates/batch/src/sim.rs`.
