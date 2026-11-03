//! Ayudas de las pruebas de los mercenarios por contrato (entrada 48): dar órdenes con un id de petición propio, bots en el tablón de un campamento, firmantes con su cargo y su oro en el carro,
//! y leer lo que queda en el estado. Sobre el artefacto de mundo real (`super::real`); solo usan la API pública del núcleo.

use super::real::M;
use asentamiento::{Cargo, Recurso};
use base::Mil;
use mercenarios::{Contrato, Objetivo, Trabajo};
use militar::columna::Politica;
use militar::heroe::{Controlador, Residencia};
use nucleo::{Ajustes, Arranque, Comando, Estado, EventoNucleo, Nucleo, Peticion, Reglas, Respuesta, Resultado};
use persistencia::{Almacen, Config, Foto, Sincronia};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

/// El oro con el que sale un firmante de su campamento si la prueba no dice otra cosa: 500 de oro.
pub const ORO: Mil = 500_000;

/// El núcleo de las pruebas de mercenarios sobre el mundo real: sin anillo de bandidos en los campamentos (ni caravanas atacadas) para que nadie estorbe en la puerta, y sin leva propia
/// (`levaPropia` vacío) para que un bot recién ofrecido no tenga tropa y su tarifa sea la de su nivel solo. La leva propia, con [`nucleo_con_leva`].
pub fn nucleo(m: &M) -> Nucleo {
    nucleo_con(m, |_| {})
}

/// Como [`nucleo`], con las reglas que `ajustar` cambie: un catálogo de misiones recurrentes propio, las batallas en Unity...
pub fn nucleo_con(m: &M, ajustar: impl FnOnce(&mut Reglas)) -> Nucleo {
    let mut reglas = reglas_sin_estorbos();
    Arc::make_mut(&mut reglas.economia.contratos).leva_propia.clear();
    ajustar(&mut reglas);
    Nucleo::nuevo(arranque(m, reglas))
}

/// Como [`nucleo`], con la leva propia de las reglas: un bot que se ofrece por primera vez recibe sus cuatro escuadras (y su tarifa las cuenta).
pub fn nucleo_con_leva(m: &M) -> Nucleo {
    Nucleo::nuevo(arranque(m, reglas_sin_estorbos()))
}

fn reglas_sin_estorbos() -> Reglas {
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    reglas
}

fn arranque(m: &M, reglas: Reglas) -> Arranque {
    Arranque { reglas, ajustes: Ajustes::default(), semilla: 48, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

/// Un núcleo con el estado de `n` tras `retocar`, que cambia lo que ningún comando da todavía (una ficha con rango y premios, una escuadra herida). Pasa por la foto, como un reinicio del
/// servidor, y escribe en un directorio temporal que vive lo que viva el `TempDir` devuelto.
pub fn retocado(m: &M, n: &Nucleo, retocar: impl FnOnce(&mut Estado)) -> (Nucleo, TempDir) {
    let reglas = n.estado().reglas.clone();
    let mut estado = Estado::deserializar(&n.estado().serializar(), &reglas, n.mundo_hash()).expect("la foto del estado se lee");
    retocar(&mut estado);
    let dir = tempfile::tempdir().expect("directorio temporal");
    let config = Config { sincronia: Sincronia::Nunca, ..Config::default() };
    Almacen::abrir(dir.path(), config).expect("almacén nuevo").fotos().guardar(&Foto::nueva(0, n.ahora(), estado.serializar())).expect("foto escrita");
    (Nucleo::con_persistencia(arranque(m, reglas), dir.path(), config).expect("el núcleo se recupera de la foto"), dir)
}

/// Da la orden `c` como `actor` (la cuenta del héroe, o `"admin"` para la administración) en el instante de ahora, con un id de petición que no se repite en todo el binario de pruebas.
pub fn dar(n: &mut Nucleo, actor: &str, c: Comando) -> Respuesta {
    static PETICIONES: AtomicU64 = AtomicU64::new(0);
    let id = format!("mc-{}", PETICIONES.fetch_add(1, Ordering::Relaxed));
    let p = Peticion::nueva(actor, &id, c);
    let t = n.ahora();
    n.aplicar_en(t, if actor == "admin" { p.de_admin() } else { p })
}

/// Da la orden y exige que se acepte: devuelve su respuesta (con las entidades que `creadas`).
pub fn ok(n: &mut Nucleo, actor: &str, c: Comando) -> Respuesta {
    let r = dar(n, actor, c);
    assert!(r.aceptada(), "{r:?}");
    r
}

/// Da la orden y exige que se rechace sin dejar rastro (ni versión nueva ni cambio en el estado): devuelve el código del rechazo. Antes deja que se cumpla lo que tocaba ya (la salida de una
/// columna, por ejemplo, cita sus hechos para este mismo instante): la versión que sube por eso no es rastro del rechazo.
pub fn rechazo(n: &mut Nucleo, actor: &str, c: Comando) -> &'static str {
    n.avanzar_hasta(n.ahora());
    let antes = (n.version(), n.hash_estado());
    let r = dar(n, actor, c);
    assert_eq!((n.version(), n.hash_estado()), antes, "el rechazo {r:?} dejó rastro");
    match r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

/// Da la orden y exige que no cambie nada (`SinCambios`, sin versión nueva).
pub fn sin_cambios(n: &mut Nucleo, actor: &str, c: Comando) {
    n.avanzar_hasta(n.ahora());
    let antes = (n.version(), n.hash_estado());
    let r = dar(n, actor, c);
    assert_eq!(r.resultado, Resultado::SinCambios, "{r:?}");
    assert_eq!((n.version(), n.hash_estado()), antes);
}

/// Un héroe nuevo de la cuenta `cuenta` en el campamento `campamento`, residente y dentro. El bot lo crea la administración; el humano, su cuenta.
pub fn heroe_en(n: &mut Nucleo, cuenta: &str, campamento: u32, controlador: Controlador) -> u64 {
    let actor = if controlador == Controlador::Bot { "admin" } else { cuenta };
    ok(n, actor, Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador, nombre: None }).creadas[0]
}

/// Un bot de la cuenta `cuenta` que reside en el campamento `campamento`, sin Facción, dentro de él y ya ofrecido en su tablón. Devuelve su id.
pub fn bot_ofrecido(n: &mut Nucleo, cuenta: &str, campamento: u32) -> u64 {
    let bot = heroe_en(n, cuenta, campamento, Controlador::Bot);
    ok(n, cuenta, Comando::OfrecerseComoMercenario { heroe: bot });
    bot
}

/// Un héroe de la cuenta `cuenta` y la Facción `faccion` que sale del campamento `campamento` con `oro` en el carro de su columna, que queda a la puerta: puede firmar allí.
/// Reside en el campamento. Si la Facción no tenía ciudadanos, es su Rey (el primer ciudadano de una Facción vacía lo es); si no, un ciudadano sin cargo.
pub fn firmante(n: &mut Nucleo, cuenta: &str, faccion: u32, campamento: u32, oro: Mil, controlador: Controlador) -> u64 {
    let heroe = heroe_en(n, cuenta, campamento, controlador);
    ok(n, "admin", Comando::FijarFaccionDeHeroe { heroe, faccion: Some(faccion) });
    ok(n, "admin", Comando::DarAlmacenPersonal { heroe, recurso: Recurso::Oro, cantidad: oro });
    salir_con_oro(n, cuenta, heroe, campamento, oro);
    heroe
}

/// El héroe, residente de `campamento` y dentro, sale con `oro` del almacén personal en el carro: su columna queda a la puerta.
pub fn salir_con_oro(n: &mut Nucleo, cuenta: &str, heroe: u64, campamento: u32, oro: Mil) {
    ok(n, cuenta, Comando::SalirDelCampamento { campamento, heroe, escuadras: vec![], carga: vec![(Recurso::Oro, oro)], politica: Politica::Rechazar, caravana_fundacion: None });
}

/// Hace a `designado` titular de `cargo` en `plaza`: la administración lo muda antes a ella (hay que residir, Q4) y `quien`, con la cuenta `cuenta`, lo designa. Al Gobernador lo designa el Rey de
/// la Facción dueña; a los demás, el Gobernador dentro de su plaza.
pub fn con_cargo(n: &mut Nucleo, cuenta: &str, quien: u64, plaza: u64, cargo: Cargo, designado: u64) {
    ok(n, "admin", Comando::FijarResidencia { heroe: designado, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    ok(n, cuenta, Comando::AsignarCargoLocal { heroe: quien, asentamiento: plaza, cargo, designado });
}

/// La orden de contratar de `heroe` (que la da su cuenta): devuelve la respuesta tal cual, para comprobar el código o la aceptación.
pub fn contratar(n: &mut Nucleo, heroe: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: Option<u8>, residencia: Option<u64>) -> Respuesta {
    let cuenta = n.heroe(heroe).expect("el firmante existe").cuenta.clone();
    dar(n, &cuenta, Comando::ContratarMercenario { heroe, mercenario, trabajo, objetivo, plazo, residencia, pagar_con: None })
}

/// Como [`contratar`], pero exige que se rechace sin dejar rastro y devuelve el código.
pub fn contratar_rechazado(n: &mut Nucleo, heroe: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: Option<u8>, residencia: Option<u64>) -> &'static str {
    let cuenta = n.heroe(heroe).expect("el firmante existe").cuenta.clone();
    rechazo(n, &cuenta, Comando::ContratarMercenario { heroe, mercenario, trabajo, objetivo, plazo, residencia, pagar_con: None })
}

/// El contrato vigente del mercenario, si lo tiene.
pub fn contrato(n: &Nucleo, mercenario: u64) -> Option<Contrato> {
    n.estado().contratos.contratos.get(&mercenario).cloned()
}

/// ¿Está el héroe en un tablón?
pub fn ofrecido(n: &Nucleo, heroe: u64) -> bool {
    n.estado().contratos.ofertas.contains_key(&heroe)
}

/// El oro que lleva el carro de la columna del héroe (en milésimas).
pub fn oro_del_carro(n: &Nucleo, heroe: u64) -> Mil {
    let columna = n.estado().ejercito_de_heroe(heroe).expect("el héroe va en una columna");
    n.ejercito(columna).expect("la columna existe").columna.carga.get(&Recurso::Oro).copied().unwrap_or(0)
}

/// El oro del almacén personal del héroe (en milésimas).
pub fn oro_personal(n: &Nucleo, heroe: u64) -> Mil {
    n.heroe(heroe).expect("el héroe existe").almacen_personal.get(&Recurso::Oro).copied().unwrap_or(0)
}

/// Los hechos de ese código que ha dado el núcleo desde el principio (en uno recuperado de una foto, [`retocado`], desde la foto), en orden.
pub fn hechos(n: &Nucleo, codigo: &str) -> Vec<EventoNucleo> {
    let desde = (0..=n.version()).find(|&v| n.cambios_desde(v).is_some()).expect("el registro de cambios llega a la versión de ahora");
    n.cambios_desde(desde).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).cloned().collect()
}
