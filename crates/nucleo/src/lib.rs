//! Núcleo de un solo hilo del mundo reactivo (fase 5): aplica órdenes y eventos en `(instante, prioridad, secuencia)`, sin
//! tick. Genérico: columnas con plan y víveres como ritmo; las reglas de juego completas llegan en la fase 6.
//!
//! - [`Nucleo`]: el bucle (modo acelerado y en vivo), las órdenes con idempotencia y `version_esperada`, la persistencia
//!   por registro de peticiones + fotos, y `hash_estado()` determinista.
//! - [`Estado`]: lo que entra en el hash y en las fotos. [`CambioVersion`]: el registro de cambios por versión. Lleva las entidades tocadas por clase ([`Clase`], [`Tocados`]) y `Nucleo::version_de` da la versión en que cambió
//!   cada una; `Ajustes::auditar_cambios` (o `BRONZEAGE_AUDITAR_CAMBIOS=1` en las pruebas) comprueba que ninguna cambie sin quedar marcada.
//! - [`GeneradorDeEventos`]: el hueco para lo que se enchufa al núcleo (la visión muestreada, `espacial`).
//! - [`economia`]: cada plaza fundada lleva su economía (crate `asentamiento`) y avanza sola un minuto de mundo por evento
//!   (`Carga::Minuto`); aquí están el mapa, los yacimientos y los sitios que comparten el núcleo y el batch de población.
//! - [`urbanismo`]: la ciudad de cada plaza (crate `trazado`): dónde cae cada edificio, anclas, puestos del Mercado y reubicación de la Granja.
//! - [`EstadoComercio`]: el comercio (fases 3B-1 y 3B-2): trueques, órdenes de mercado y mostrador, flota de caravanas que viajan por el mapa
//!   (plan, peajes, reparto automático, red de caminos), mercado de mercenarios, reputación entre Facciones y sus plazos como eventos de prioridad 6 y 9 (`Carga::Comercio`, `Carga::Reputacion`); las reglas puras son el
//!   crate `comercio`. Las órdenes están en `Comando` (códigos 21-40) y el cierre por ruina o conquista, en `Nucleo`.
//! - Pueblos extractores (fase 3C): obra de mapa, extracción contra un almacén local y carro de acarreo hacia la plaza madre; las reglas
//!   puras son `asentamiento::pueblos` y `comercio::acarreo`, y las órdenes (códigos 45-48), en `Nucleo`.
//! - Héroes y tropa (fase 4B-1): [`EstadoMilitar`] guarda los héroes con sus escuadras, loadouts y almacén personal, los reclutas de los campamentos de
//!   mercenarios y las tecnologías que da la administración; reclutar en una plaza y en un campamento, la tropa prestada, la guarnición (que come en
//!   el paso de minuto de su plaza), la escolta de caravanas, conectarse y desconectarse (prioridad 1, `Carga::SalidaDelMundo`). Las reglas puras son el
//!   crate `militar` y las órdenes (códigos 41-44 y 49-68), `Nucleo`.
//! - Columnas completas (fase 4B-2): [`Ejercito`] es una [`militar::columna::Columna`] con su posición y su plan; salir y entrar en plazas y campamentos,
//!   unir, separar, ceder el mando, formar un ejército, repostar, las caravanas adjuntas, la desconexión con la columna y las convocatorias (órdenes 69-90),
//!   el hambre con su deserción (un evento por columna con hambre).
//! - Encuentros, persecución e inspección (fase 4C): el espacio del mundo (`nucleo/espacio.rs`: columnas en el mapa, caravanas y plazas en una rejilla
//!   espacio-temporal) programa cada encuentro como un evento exacto (`Carga::Encuentro`) y deja un hecho; `perseguir` rehace el plan de quien persigue cada
//!   vez que la presa cambia el suyo; `inspeccionar` guarda un informe en el héroe (órdenes 91-94).
//! - Combate con números (fase 4D): `atacar` (orden 95) con sus seis objetivos —columna, caravana (tipo A o B por la zona), campamento de bandidos, plaza,
//!   pueblo y avanzada— y sus consecuencias (heridos, mitad del carro, botín, chatarra, experiencia de escuadra); la conquista
//!   con su protección y su ocupación como eventos, el desalojo y la cascada del comercio; los bandidos como entidades fijas del espacio, con su aparición, su
//!   reaparición y su acoso a las caravanas (`PRIO_BANDIDOS`, `Carga::Combate`); y la guarnición de serie de los pueblos. El estado nuevo es [`EstadoCombate`]
//!   (foto `BAN12`, hoy `BAN19`); las reglas puras son el crate `combate`.
//! - Ciclo de la batalla de Unity (fase 4E): `atacar` abre una batalla en lugar de resolver con números cuando el proceso tiene servidores de batalla
//!   (`Reglas::batallas_en_unity`), también bot contra bot. [`EstadoBatallas`] guarda el ticket congelado, las incorporaciones, la asignación y
//!   el resultado de cada batalla (foto `BAN13`, hoy `BAN19`); mientras dura, sus columnas y caravanas quedan apartadas (no se mueven, no comen, no se las toca), sus
//!   escuadras llevan el candado `reserva_batalla` y ninguna orden puede tocar lo que combate (`rechazos::BATALLA_BLOQUEADO`). Sus plazos son eventos de
//!   prioridad 1 (`Carga::Batalla`) y los cinco mensajes del servidor de batalla, órdenes con el actor `batalla-servidor:<id>` (códigos 101-105; unirse, cancelar y
//!   el andamiaje `FijarControlador`, 98-100). Las reglas puras son `combate::{ciclo, resultado, union}`.
//! - Avanzadas (fase 4F): puestos fortificados de la plaza madre (atalaya, fuerte, ciudadela), levantados como obra de mapa con su fin exacto en una cita propia (cancelable),
//!   con visión de 12, almacén, guarnición de serie y de jugadores (`Contenedor::Avanzada`), ruinas con un día de invulnerabilidad (`SucesoCombate::FinInvulnerabilidad`) y
//!   su sitio en la rejilla de encuentros como entidad fija. Se atacan con `atacar` (con números o con una batalla de Unity, capacidad de asedio y sin convocatoria).
//!   Las órdenes son los códigos 106-116; la foto es `BAN19`. Las reglas puras son `asentamiento::avanzadas` y `combate::avanzada`.
//! - Entrar con todo (2026-10-22): quien entra en un lugar lleva consigo tropa, carro, víveres y todas sus caravanas y al salir se lo lleva (no hay «columna aparcada a la
//!   puerta»); el residente conserva su carro (`HeroeNucleo::carro`, `VaciarCarro` 118); el dueño de la plaza decide por relación quién entra (`FijarAcceso` 117,
//!   [`Acceso`]; pueblos y avanzadas lo heredan); defienden los de dentro de la misma Facción, aliados o vasallos; al caer el lugar salen al mapa con lo suyo. Foto `BAN17`.
//! - Entrar en un pueblo o en una avanzada (2026-10-22): `EntrarEnPueblo`/`SalirDePueblo` (119, 120) y `EntrarEnAvanzada`/`SalirDeAvanzada` (121, 122) con el acceso de su plaza
//!   madre; dentro se defiende con la regla de relación; cerrar la puerta (`FijarAcceso`) expulsa a quien ya no puede entrar; al atacante solo se une quien está fuera y al
//!   defensor quien está dentro; el residente que defiende y pierde pierde la mitad de su carro. Foto `BAN18`.
//! - Murallas (2026-10-22, tandas segunda y tercera): los recintos son parte de la economía de cada plaza (`asentamiento::recintos`) y se levantan celda a celda, cada una con su cita de fin
//!   de obras (`Evento::RecintoCelda`), reintentada por el paso de minuto si faltan materiales. `ComprometerRecinto` (123, también amplía), `MejorarRecinto` (124) y
//!   `AbandonarRecinto` (125), con rechazos `murallas.*`; `Nucleo::presupuesto_de_recinto` enseña el trazo y su coste antes de comprometer. La ciudad de la plaza guarda una
//!   proyección derivada de sus recintos (no los persiste). El cupo de guarnición, el requisito del nivel 4 y el mantenimiento ya los leen de ahí. Foto `BAN20`.
//!   Tercera tanda: el asedio a una plaza con números multiplica al defensor con su recinto de referencia (`Nucleo::multiplicador_de_plaza`; la columna, el pueblo, la caravana y
//!   la avanzada siguen con 1), la conquista daña el 30 % de cada recinto completo y sin mejora (por números y por Unity) y la plaza repara sola, `Nucleo::recintos_en_pie` da a
//!   Unity la geometría en pie y `VistaCiudad.murallas` la dibuja. El ticket del asedio y el informe de inspección con los recintos son de 4H (`Docs/Arquitectura/Murallas_Contraste_y_Plan.md`).
//! - [`sesiones::lanzar`]: el actor que habla con `servidor` (foto, deltas, resultado causal, reanudación). `sesiones::ordenes` traduce las 145 órdenes del contrato 05 a los 125 comandos del núcleo
//!   (99 órdenes del jugador, la de administración y 45 que se rechazan con `orden.no_disponible` hasta que existan sus sistemas; `Docs/Mechanics/Comandos_Parametros.md`).

mod codec;
mod codec_batallas;
mod codec_ciudad;
mod codec_combate;
mod codec_columna;
mod codec_comercio;
mod codec_economia;
mod codec_militar;
mod comando;
mod estado;
mod estado_batallas;
mod estado_combate;
mod estado_comercio;
mod estado_militar;
mod generadores;
mod nucleo;
pub mod economia;
pub mod espacial;
pub mod proyeccion;
pub mod sesiones;
pub mod urbanismo;

pub use codec::ErrorFormato;
pub use comando::{rechazos, AccionReloj, Comando, Detalle, ObjetivoAtaque, PagarCon, Peticion, Resultado, Respuesta, TipoCreado, ACTOR_SERVIDOR_DE_BATALLA};
pub use estado::{Asentamiento, Carga, CambioVersion, CitaReputacion, Clase, Tocados, Ejercito, Estado, EventoNucleo, PlanEnPausa, Reglas, ReglasEconomia, ReglasTerritorio, SucesoBatalla, SucesoComercio, SucesoCombate, SucesoMilitar, SIN_BANDERA, PRIO_ASENTAMIENTO, PRIO_BANDIDOS, PRIO_BATALLA, PRIO_CADUCA_POLITICA, PRIO_COMERCIO, PRIO_FIN_DE_OBRAS, PRIO_LLEGADA, PRIO_REPUTACION, PRIO_SALIDA, PRIO_UMBRAL};
pub use estado_batallas::{entidad_batalla, EstadoBatallas};
pub use estado_combate::{entidad_avanzada, entidad_bandidos, entidad_conquista, entidad_guarnicion, es_entidad_avanzada, es_entidad_bandidos, CampamentoBandidos, EstadoCombate, GuarnicionDeAvanzada, GuarnicionDePueblo, OrigenBandidos};
pub use estado_comercio::{entidad_caravana, entidad_orden, entidad_reputacion, entidad_trueque, EstadoComercio};
pub use estado_militar::{entidad_formacion, entidad_peticion_de_convocatoria, entidad_peticion_de_union, Acceso, EstadoMilitar, HeroeNucleo, Relacion, Trato};
pub use generadores::{Cx, GeneradorDeEventos};
pub use nucleo::{Ajustes, Arranque, ErrorPresupuesto, InformeViabilidad, Metricas, Nucleo, Presupuesto, RecintoEnPie, TokenDeJugador};

/// sha256 en hexadecimal minúsculas.
pub fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}
