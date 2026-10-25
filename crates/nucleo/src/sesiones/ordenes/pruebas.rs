//! Pruebas de la traducción de órdenes: la tabla orden ↔ comando, lo que no existe aún, la administración, lo implícito, los rechazos de forma y la exhaustividad.

use super::*;
use std::collections::BTreeSet;

/// Un mundo de mentira: la cuenta «ana» tiene el héroe 7 (en la columna 70); la plaza 3 está en (30, 40); el héroe 7 reposta en la plaza 3 y el 8 en la avanzada 6; solo el 7 está en un
/// campamento (el 4); `obra-5` es un pueblo y `obra-6` una avanzada; la batalla 9 es `uuid-9`. El héroe 99 no va en ninguna columna.
struct Mundo;

impl Consulta for Mundo {
    fn version(&self) -> u64 {
        5
    }
    fn heroe_de_cuenta(&self, cuenta: &str) -> Option<u64> {
        (cuenta == "ana").then_some(7)
    }
    fn columna_de(&self, heroe: u64) -> Option<u64> {
        (heroe != 99).then_some(heroe * 10)
    }
    fn posicion_de_plaza(&self, plaza: u64) -> Option<Punto> {
        (plaza == 3).then(|| Punto::new(30_000, 40_000))
    }
    fn lugar_de_repostar(&self, heroe: u64) -> Option<Ubicacion> {
        match heroe {
            7 => Some(Ubicacion::Asentamiento(3)),
            8 => Some(Ubicacion::Avanzada(6)),
            _ => None,
        }
    }
    fn campamento_del_heroe(&self, heroe: u64) -> Option<u32> {
        (heroe == 7).then_some(4)
    }
    fn obra(&self, id: u64) -> Option<Obra> {
        match id {
            5 => Some(Obra::Pueblo),
            6 => Some(Obra::Avanzada),
            _ => None,
        }
    }
    fn batalla(&self, uuid: &str) -> Option<u64> {
        (uuid == "uuid-9").then_some(9)
    }
    fn alijo(&self, id: &str) -> Option<u32> {
        (id == "alijo-veta-2").then_some(12)
    }
    fn tecnologia(&self, id: &str) -> Option<u8> {
        (id == "metalurgia_cobre").then_some(2)
    }
}

fn s(x: &str) -> String {
    x.into()
}
fn v(x: &[&str]) -> Vec<String> {
    x.iter().map(|&x| x.into()).collect()
}
fn carga(x: &[(&str, i64)]) -> Option<pb::Cantidades> {
    Some(pb::Cantidades { por_id: x.iter().map(|&(k, n)| (k.into(), n)).collect() })
}
fn pb_punto(x: i64, y: i64) -> Option<pb::Punto> {
    Some(pb::Punto { x, y })
}

macro_rules! orden {
    ($v:ident, $t:ident $(, $campo:ident : $valor:expr)* $(,)?) => {
        Cuerpo::$v(pb::$t { $($campo: $valor,)* ..Default::default() })
    };
}

fn en(cuerpo: Cuerpo) -> pb::Orden {
    pb::Orden { peticion_id: "p1".into(), version_esperada: Some(4), cuerpo: Some(cuerpo) }
}

fn traduce(cuerpo: Cuerpo, admin: bool) -> Result<Comando, &'static str> {
    match traducir(&Mundo, &en(cuerpo), "ana", admin) {
        Ok(p) => Ok(p.comando),
        Err(r) => match r.resultado {
            crate::comando::Resultado::Rechazada { codigo, .. } => Err(codigo),
            otro => panic!("un rechazo de la traducción es una Rechazada: {otro:?}"),
        },
    }
}

fn codigo_de(r: Result<Peticion, Respuesta>) -> &'static str {
    match r.expect_err("se esperaba un rechazo").resultado {
        crate::comando::Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("un rechazo de la traducción es una Rechazada: {otro:?}"),
    }
}

/// El nombre de la variante de un valor (lo que va antes del primer paréntesis, llave o espacio de su `Debug`).
fn nombre(d: &impl std::fmt::Debug) -> String {
    format!("{d:?}").split(|c: char| !c.is_alphanumeric()).next().unwrap_or_default().to_owned()
}

use asentamiento::Recurso::{Madera, Oro, Piedra, Trigo};

/// Una fila por orden del jugador que el núcleo tiene (y una más por cada orden que da dos comandos según la obra o el lugar): la orden del contrato y el comando que debe salir.
fn ordenes_traducidas() -> Vec<(Cuerpo, Comando)> {
    use pb::orden_atacar::objetivo::Tipo as Ataca;
    use pb::orden_marchar_a::objetivo::Tipo as Marcha;
    let (h, esc) = (s("heroe-7"), v(&["esc-1", "esc-2"]));
    let madera = || vec![(Madera, 5_000)];
    let tropa = Tropa::MiliciaLanceros;
    let pueblo = || s("pueblo-4");
    vec![
        // ── Presencia y héroe ──
        (orden!(CrearHeroe, OrdenCrearHeroe, display_name: s("Aquiles"), campamento_id: s("camp-4"), class_definition_id: s("guerrero"), genero: s("masculino")), Comando::CrearHeroe { cuenta: "ana".into(), campamento: 4, controlador: Controlador::Humano }),
        (orden!(ReclutarTropa, OrdenReclutarTropa, asentamiento_id: s("as-3"), heroe_id: h.clone(), tropa_id: s(tropa.nombre()), origen: Some(s("pesants"))), Comando::ReclutarTropa { asentamiento: 3, heroe: 7, tropa }),
        (orden!(ReclutarEnCampamento, OrdenReclutarEnCampamento, tropa_id: s(tropa.nombre()), pagar_con: Some(s("carro"))), Comando::ReclutarEnCampamento { heroe: 7, tropa, pagar_con: PagarCon::Carro }),
        (orden!(PedirPrestamo, OrdenPedirPrestamo, tropa_ids: v(&[tropa.nombre(), Tropa::Lenadores.nombre()])), Comando::PedirPrestamo { heroe: 7, tropas: vec![tropa, Tropa::Lenadores] }),
        (orden!(ReponerPrestamo, OrdenReponerPrestamo), Comando::ReponerPrestamo { heroe: 7 }),
        (
            orden!(GuardarLoadout, OrdenGuardarLoadout, loadout_id: Some(s("loadout-12")), display_name: s("Plan B"), squad_ids: esc.clone(), perks_seleccionados: vec![3, 4], activo: Some(true)),
            Comando::GuardarLoadout { heroe: 7, loadout: Some(12), nombre: "Plan B".into(), escuadras: vec![1, 2], perks: vec![3, 4], activo: Some(true) },
        ),
        (orden!(BorrarLoadout, OrdenBorrarLoadout, loadout_id: s("loadout-12")), Comando::BorrarLoadout { heroe: 7, loadout: 12 }),
        (
            orden!(RepartirPuntos, OrdenRepartirPuntos, atributos: Some(pb::orden_repartir_puntos::Atributos { fuerza: Some(2), vitalidad: Some(1), ..Default::default() })),
            Comando::RepartirPuntos { heroe: 7, fuerza: 2, destreza: 0, armadura: 0, vitalidad: 1 },
        ),
        (orden!(OrdenarEscuadras, OrdenOrdenarEscuadras, escuadron_ids: v(&["esc-2", "esc-1"])), Comando::OrdenarEscuadras { heroe: 7, escuadras: vec![2, 1] }),
        (orden!(AsignarGuarnicion, OrdenAsignarGuarnicion, squad_id: s("esc-5")), Comando::AsignarGuarnicion { heroe: 7, escuadra: 5 }),
        (orden!(RetirarGuarnicion, OrdenRetirarGuarnicion, squad_id: s("esc-5")), Comando::RetirarGuarnicion { heroe: 7, escuadra: 5 }),
        (orden!(PasarAViveres, OrdenPasarAViveres, cantidad: 25_000), Comando::PasarAViveres { heroe: 7, cantidad: 25_000 }),
        (orden!(AsignarEscolta, OrdenAsignarEscolta, caravana_id: s("car-2"), heroe_id: h.clone(), escuadron_ids: v(&["esc-1"])), Comando::AsignarEscolta { caravana: 2, heroe: 7, escuadras: vec![1] }),
        (orden!(QuitarEscolta, OrdenQuitarEscolta, caravana_id: s("car-2"), heroe_id: h.clone(), escuadron_ids: v(&["esc-1"])), Comando::QuitarEscolta { caravana: 2, heroe: 7, escuadras: Some(vec![1]) }),
        (orden!(Desconectarse, OrdenDesconectarse, heroe_id: h.clone()), Comando::Desconectarse { heroe: 7 }),
        (orden!(Conectarse, OrdenConectarse, heroe_id: h.clone()), Comando::Conectarse { heroe: 7 }),
        (orden!(ComprarEnCampamento, OrdenComprarEnCampamento, recurso: s("trigo"), cantidad: 3_000, campamento_id: Some(s("camp-9"))), Comando::ComprarEnCampamento { heroe: 7, campamento: 9, recurso: Trigo, cantidad: 3_000 }),
        (orden!(RepostarEnCampamento, OrdenRepostarEnCampamento, campamento_id: Some(s("camp-9"))), Comando::RepostarEnCampamento { heroe: 7, campamento: 9 }),
        // ── Columnas y convocatorias ──
        (
            orden!(MarcharA, OrdenMarcharA, heroe_id: h.clone(), objetivo: Some(pb::orden_marchar_a::Objetivo { tipo: Some(Marcha::Punto(pb::Punto { x: 8_000, y: 4_000 })) })),
            Comando::MarcharA { ejercito: 70, destino: Punto::new(8_000, 4_000) },
        ),
        (orden!(EstacionarEjercito, OrdenEstacionarEjercito, ejercito_id: s("ej-3")), Comando::Estacionar { ejercito: 3 }),
        (orden!(ControlarReloj, OrdenControlarReloj, accion: s("fijar_factor"), factor_milesimas: Some(600_000)), Comando::Reloj(AccionReloj::FijarFactor(600_000))),
        (orden!(Fundar, OrdenFundar), Comando::Fundar { ejercito: 70 }),
        (
            orden!(SalirAlMundo, OrdenSalirAlMundo, asentamiento_id: s("as-3"), heroe_id: h.clone(), escuadron_ids: esc.clone(), carga: carga(&[("madera", 5_000)])),
            Comando::SalirAlMundo { asentamiento: 3, heroe: 7, escuadras: vec![1, 2], carga: madera() },
        ),
        (
            orden!(MovilizarEjercito, OrdenMovilizarEjercito, asentamiento_id: s("as-3"), heroe_id: h.clone(), escuadron_ids: esc.clone(), politica_de_union: Some(s("preguntar")), carga: carga(&[("madera", 5_000)])),
            Comando::MovilizarEjercito { asentamiento: 3, heroe: 7, escuadras: vec![1, 2], carga: madera(), politica: Politica::Preguntar },
        ),
        (
            orden!(SalirDelCampamento, OrdenSalirDelCampamento, campamento_id: s("camp-4"), heroe_id: h.clone(), escuadron_ids: esc.clone(), carga: carga(&[("madera", 5_000)])),
            Comando::SalirDelCampamento { campamento: 4, heroe: 7, escuadras: vec![1, 2], carga: madera(), politica: Politica::Rechazar },
        ),
        (orden!(EntrarEnAsentamiento, OrdenEntrarEnAsentamiento, asentamiento_id: s("as-3"), heroe_id: h.clone()), Comando::EntrarEnAsentamiento { asentamiento: 3, heroe: 7 }),
        (orden!(SalirDeAsentamiento, OrdenSalirDeAsentamiento, asentamiento_id: s("as-3"), heroe_id: h.clone()), Comando::SalirDeAsentamiento { asentamiento: 3, heroe: 7 }),
        (orden!(Guarnecer, OrdenGuarnecer, asentamiento_id: s("as-3"), heroe_id: h.clone()), Comando::Guarnecer { asentamiento: 3, heroe: 7 }),
        (orden!(EntrarEnCampamento, OrdenEntrarEnCampamento, campamento_id: s("camp-4"), heroe_id: h.clone()), Comando::EntrarEnCampamento { campamento: 4, heroe: 7 }),
        (
            orden!(UnirseAEjercito, OrdenUnirseAEjercito, ejercito_id: s("ej-3"), asentamiento_id: s("as-3"), heroe_id: h.clone(), escuadron_ids: esc.clone()),
            Comando::UnirseAEjercito { asentamiento: 3, heroe: 7, ejercito: 3, escuadras: vec![1, 2], carga: vec![] },
        ),
        (orden!(UnirseEnCampo, OrdenUnirseEnCampo, ejercito_id: s("ej-3"), heroe_id: h.clone()), Comando::UnirseEnCampo { heroe: 7, ejercito: 3 }),
        (
            orden!(ResponderPeticionDeUnion, OrdenResponderPeticionDeUnion, ejercito_id: s("ej-3"), heroe_id: h.clone(), solicitante_id: s("heroe-8"), aceptar: true),
            Comando::ResponderPeticionDeUnion { heroe: 7, ejercito: 3, solicitante: 8, aceptar: true },
        ),
        (orden!(SepararseDelEjercito, OrdenSepararseDelEjercito, heroe_id: h.clone()), Comando::SepararseDelEjercito { heroe: 7 }),
        (orden!(CederLiderazgo, OrdenCederLiderazgo, ejercito_id: s("ej-3"), heroe_id: h.clone(), sucesor_id: s("heroe-8")), Comando::CederLiderazgo { heroe: 7, sucesor: 8 }),
        (orden!(OrganizarEjercito, OrdenOrganizarEjercito, heroe_id: h.clone(), politica_de_union: s("aceptar")), Comando::OrganizarEjercito { heroe: 7, politica: Politica::Aceptar }),
        (orden!(CancelarFormacion, OrdenCancelarFormacion, heroe_id: h.clone()), Comando::CancelarFormacion { heroe: 7 }),
        (orden!(Repostar, OrdenRepostar, heroe_id: h.clone(), lugar_id: Some(s("as-3"))), Comando::Repostar { heroe: 7, asentamiento: 3 }),
        (orden!(Repostar, OrdenRepostar, heroe_id: h.clone(), lugar_id: Some(s("av-6"))), Comando::RepostarEnAvanzada { heroe: 7, avanzada: 6 }),
        (
            orden!(ConvocarEjercito, OrdenConvocarEjercito, heroe_id: h.clone(), politica_de_union: s("aceptar"), escuadron_ids: esc.clone(), carga: carga(&[("madera", 5_000)])),
            Comando::ConvocarEjercito { heroe: 7, escuadras: vec![1, 2], carga: madera(), politica: Politica::Aceptar },
        ),
        (
            orden!(UnirseAConvocatoria, OrdenUnirseAConvocatoria, heroe_id: h.clone(), convocatoria_id: s("conv-2"), escuadron_ids: esc.clone(), carga: carga(&[("madera", 5_000)])),
            Comando::UnirseAConvocatoria { heroe: 7, convocatoria: 2, escuadras: vec![1, 2], carga: madera() },
        ),
        (
            orden!(ResponderPeticionDeConvocatoria, OrdenResponderPeticionDeConvocatoria, heroe_id: h.clone(), convocatoria_id: s("conv-2"), solicitante_id: s("heroe-8"), aceptar: false),
            Comando::ResponderPeticionDeConvocatoria { heroe: 7, convocatoria: 2, solicitante: 8, aceptar: false },
        ),
        (
            orden!(CambiarSeleccionDeConvocatoria, OrdenCambiarSeleccionDeConvocatoria, heroe_id: h.clone(), escuadron_ids: esc.clone(), carga: carga(&[("madera", 5_000)])),
            Comando::CambiarSeleccionDeConvocatoria { heroe: 7, escuadras: vec![1, 2], carga: madera() },
        ),
        (orden!(SepararseDeConvocatoria, OrdenSepararseDeConvocatoria, heroe_id: h.clone()), Comando::SepararseDeConvocatoria { heroe: 7 }),
        (orden!(CancelarConvocatoria, OrdenCancelarConvocatoria, heroe_id: h.clone()), Comando::CancelarConvocatoria { heroe: 7 }),
        (orden!(PartirConvocatoria, OrdenPartirConvocatoria, heroe_id: h.clone()), Comando::PartirConvocatoria { heroe: 7 }),
        // ── Encuentros, combate y batalla ──
        (
            orden!(Perseguir, OrdenPerseguir, heroe_id: h.clone(), objetivo: Some(pb::orden_perseguir::Objetivo { tipo: Some(pb::orden_perseguir::objetivo::Tipo::CaravanaId(s("car-2"))) })),
            Comando::Perseguir { heroe: 7, objetivo: Objetivo::Caravana(2) },
        ),
        (orden!(DejarDePerseguir, OrdenDejarDePerseguir, heroe_id: h.clone()), Comando::DejarDePerseguir { heroe: 7 }),
        (
            orden!(Inspeccionar, OrdenInspeccionar, heroe_id: h.clone(), objetivo: Some(pb::orden_inspeccionar::Objetivo { tipo: Some(pb::orden_inspeccionar::objetivo::Tipo::AsentamientoId(s("as-3"))) })),
            Comando::Inspeccionar { heroe: 7, objetivo: Objetivo::Asentamiento(3) },
        ),
        (
            orden!(Atacar, OrdenAtacar, heroe_id: h.clone(), objetivo: Some(pb::orden_atacar::Objetivo { tipo: Some(Ataca::PuebloId(pueblo())) }), declara_guerra: Some(true)),
            Comando::Atacar { heroe: 7, objetivo: ObjetivoAtaque::Pueblo(4), declara_guerra: true },
        ),
        (orden!(UnirseABatalla, OrdenUnirseABatalla, heroe_id: h.clone(), battle_id: s("uuid-9"), lado: Some(s("defensor"))), Comando::UnirseABatalla { heroe: 7, batalla: 9, lado: Some(Lado::Defensor) }),
        (orden!(CancelarBatalla, OrdenCancelarBatalla, battle_id: s("uuid-9")), Comando::CancelarBatalla { heroe: 7, batalla: 9 }),
        // ── Avanzadas ──
        (orden!(ColocarAvanzada, OrdenColocarAvanzada, asentamiento_id: s("as-3"), heroe_id: h.clone(), posicion: pb_punto(1_000, 2_000)), Comando::ColocarAvanzada { asentamiento: 3, heroe: 7, posicion: Punto::new(1_000, 2_000) }),
        (orden!(EntregarEnObra, OrdenEntregarEnObra, heroe_id: h.clone(), obra_id: s("obra-6"), materiales: carga(&[("madera", 1)])), Comando::EntregarMaterialesAAvanzada { ejercito: 70, avanzada: 6 }),
        (orden!(EntregarEnObra, OrdenEntregarEnObra, heroe_id: h.clone(), obra_id: s("obra-5")), Comando::EntregarMateriales { ejercito: 70, pueblo: 5 }),
        (orden!(CancelarObra, OrdenCancelarObra, heroe_id: h.clone(), obra_id: s("obra-6")), Comando::CancelarObraDeAvanzada { heroe: 7, avanzada: 6 }),
        (orden!(MejorarAvanzada, OrdenMejorarAvanzada, heroe_id: h.clone(), avanzada_id: s("av-6")), Comando::MejorarAvanzada { heroe: 7, avanzada: 6 }),
        (orden!(RepararAvanzada, OrdenRepararAvanzada, heroe_id: h.clone(), avanzada_id: s("av-6")), Comando::RepararAvanzada { heroe: 7, avanzada: 6 }),
        (orden!(DestruirAvanzada, OrdenDestruirAvanzada, heroe_id: h.clone(), avanzada_id: s("av-6")), Comando::DestruirAvanzada { heroe: 7, avanzada: 6 }),
        (orden!(ElegirTropaDeAvanzada, OrdenElegirTropaDeAvanzada, heroe_id: h.clone(), avanzada_id: s("av-6"), tropa_id: s(tropa.nombre())), Comando::ElegirTropaDeAvanzada { heroe: 7, avanzada: 6, tropa }),
        (orden!(GuarnecerAvanzada, OrdenGuarnecerAvanzada, heroe_id: h.clone(), avanzada_id: s("av-6"), escuadron_ids: esc.clone()), Comando::GuarnecerAvanzada { heroe: 7, avanzada: 6, escuadras: vec![1, 2] }),
        (orden!(RetirarDeAvanzada, OrdenRetirarDeAvanzada, heroe_id: h.clone(), avanzada_id: s("av-6"), escuadron_ids: esc.clone()), Comando::RetirarDeAvanzada { heroe: 7, avanzada: 6, escuadras: vec![1, 2] }),
        (orden!(DepositarEnAvanzada, OrdenDepositarEnAvanzada, ejercito_id: s("ej-3"), avanzada_id: s("av-6"), recurso: s("oro"), cantidad: 9_000), Comando::DepositarEnAvanzada { ejercito: 3, avanzada: 6, recurso: Oro, cantidad: 9_000 }),
        (orden!(EntrarEnAvanzada, OrdenEntrarEnAvanzada, avanzada_id: s("av-6"), heroe_id: h.clone()), Comando::EntrarEnAvanzada { avanzada: 6, heroe: 7 }),
        (orden!(SalirDeAvanzada, OrdenSalirDeAvanzada, avanzada_id: s("av-6"), heroe_id: h.clone()), Comando::SalirDeAvanzada { avanzada: 6, heroe: 7 }),
        // ── Acceso y murallas ──
        (
            orden!(FijarPuerta, OrdenFijarPuerta, asentamiento_id: s("as-3"), heroe_id: h.clone(), cerrada_a: v(&["enemigos", "neutrales"])),
            Comando::FijarPuerta { heroe: 7, asentamiento: 3, cerrada_a: politica::Grupos::de([politica::Grupo::Neutrales, politica::Grupo::Enemigos]) },
        ),
        (
            orden!(VetarJugador, OrdenVetarJugador, asentamiento_id: s("as-3"), heroe_id: h.clone(), vetado_id: s("heroe-9"), vetar: true),
            Comando::VetarJugador { heroe: 7, asentamiento: 3, vetado: 9, vetar: true },
        ),
        (
            orden!(AsignarCargoLocal, OrdenAsignarCargoLocal, asentamiento_id: s("as-3"), cargo: s("tesorero"), heroe_id: s("heroe-9")),
            Comando::AsignarCargoLocal { heroe: 7, asentamiento: 3, cargo: Cargo::Tesorero, designado: 9 },
        ),
        (orden!(ComprometerRecinto, OrdenComprometerRecinto, asentamiento_id: s("as-3"), cargo: s("gobernador"), nivel: s("2")), Comando::ComprometerRecinto { asentamiento: 3, cargo: Cargo::Gobernador, nivel: 2 }),
        (orden!(MejorarRecinto, OrdenMejorarRecinto, asentamiento_id: s("as-3"), cargo: s("maestroObras"), recinto_id: s("recinto-3-1")), Comando::MejorarRecinto { asentamiento: 3, cargo: Cargo::MaestroObras, recinto: 1 }),
        (orden!(AbandonarRecinto, OrdenAbandonarRecinto, asentamiento_id: s("as-3"), recinto_id: s("recinto-3-1")), Comando::AbandonarRecinto { asentamiento: 3, recinto: 1 }),
        // ── La intel de las tabernas (4H T3d): el héroe es el de la cuenta ──
        (
            orden!(ComprarMirada, OrdenComprarMirada, origen: Some(pb::orden_comprar_mirada::Origen { tipo: s("campamento"), id: s("camp-2") }), centro: Some(pb::Punto { x: 40_000, y: 9_000 })),
            Comando::ComprarMirada { heroe: 7, origen: OrigenIntel::Campamento(2), centro: Punto::new(40_000, 9_000) },
        ),
        (
            orden!(ComprarInformePlaza, OrdenComprarInformePlaza, origen: Some(pb::orden_comprar_informe_plaza::Origen { tipo: s("asentamiento"), id: s("as-3") }), asentamiento_id: s("as-8")),
            Comando::ComprarInformePlaza { heroe: 7, origen: OrigenIntel::Plaza(3), asentamiento: 8 },
        ),
        // ── Economía y construcción ──
        (orden!(AnadirEdificioManualmente, OrdenAnadirEdificioManualmente, asentamiento_id: s("as-3"), cargo: s("gobernador"), tipo: s("granja")), Comando::AnadirEdificio { asentamiento: 3, tipo: Tipo::Granja }),
        (orden!(QuitarDeCola, OrdenQuitarDeCola, asentamiento_id: s("as-3"), cargo: s("gobernador"), edificio_id: s("ed-3-7")), Comando::QuitarDeCola { asentamiento: 3, edificio: 7 }),
        (orden!(MoverEnCola, OrdenMoverEnCola, asentamiento_id: s("as-3"), cargo: s("gobernador"), edificio_id: s("ed-3-7"), direccion: s("abajo")), Comando::MoverEnCola { asentamiento: 3, edificio: 7, arriba: false }),
        (orden!(MejorarEdificioAhora, OrdenMejorarEdificioAhora, asentamiento_id: s("as-3"), cargo: s("maestroObras"), edificio_id: s("ed-3-7")), Comando::MejorarEdificio { asentamiento: 3, edificio: 7 }),
        (orden!(AlternarAutoConstruccion, OrdenAlternarAutoConstruccion, asentamiento_id: s("as-3"), pausada: true), Comando::AlternarAutoConstruccion { asentamiento: 3, pausada: true }),
        (orden!(AlternarReceta, OrdenAlternarReceta, asentamiento_id: s("as-3"), recurso: s("madera"), pausada: true), Comando::AlternarReceta { asentamiento: 3, recurso: Madera, pausada: true }),
        (orden!(CalibrarReservaManual, OrdenCalibrarReservaManual, asentamiento_id: s("as-3"), recurso: s("trigo"), valor: 50_000), Comando::CalibrarReserva { asentamiento: 3, recurso: Trigo, valor: 50_000 }),
        (orden!(SolicitarAscenso, OrdenSolicitarAscenso, asentamiento_id: s("as-3")), Comando::SolicitarAscenso { asentamiento: 3 }),
        (orden!(ActivarPolitica, OrdenActivarPolitica, asentamiento_id: s("as-3"), cargo: s("general"), politica_id: s("leva_forzosa")), Comando::ActivarPolitica { asentamiento: 3, politica: "leva_forzosa".into(), cargo: Cargo::General }),
        // ── Comercio ──
        (
            orden!(ProponerTrueque, OrdenProponerTrueque, asentamiento_a_id: s("as-3"), lineas_a: vec![pb::orden_proponer_trueque::LineasA { recurso: s("madera"), cantidad: 5_000 }], asentamiento_b_id: s("as-4"), lineas_b: vec![pb::orden_proponer_trueque::LineasB { recurso: s("trigo"), cantidad: 2_000 }]),
            Comando::ProponerTrueque { a: 3, b: 4, lineas_a: vec![(Madera, 5_000)], lineas_b: vec![(Trigo, 2_000)] },
        ),
        (orden!(AceptarTrueque, OrdenAceptarTrueque, acuerdo_id: s("acu-8")), Comando::AceptarTrueque { acuerdo: 8 }),
        (orden!(RechazarTrueque, OrdenRechazarTrueque, acuerdo_id: s("acu-8")), Comando::RechazarTrueque { acuerdo: 8 }),
        (
            orden!(ColocarOrdenMercado, OrdenColocarOrdenMercado, asentamiento_id: s("as-3"), tipo: s("venta"), recurso: s("madera"), cantidad: 4_000, precio: Some(1_500)),
            Comando::ColocarOrdenMercado { asentamiento: 3, venta: true, recurso: Madera, cantidad: 4_000, precio: Some(1_500) },
        ),
        (orden!(CancelarOrdenMercado, OrdenCancelarOrdenMercado, orden_id: s("ord-2")), Comando::CancelarOrdenMercado { orden: 2 }),
        (orden!(ComerciarEnPlaza, OrdenComerciarEnPlaza, heroe_id: h.clone(), asentamiento_id: s("as-3"), orden_id: s("ord-2"), cantidad: 4_000), Comando::ComerciarEnPlaza { ejercito: 70, orden: 2, cantidad: 4_000 }),
        (orden!(CrearCaravana, OrdenCrearCaravana, asentamiento_id: s("as-3")), Comando::CrearCaravana { asentamiento: 3 }),
        (orden!(AgregarCarroCaravana, OrdenAgregarCarroCaravana, caravana_id: s("car-2"), tipo_carro: s("reforzado")), Comando::AnadirCarro { caravana: 2, tipo: TipoCarro::Reforzado }),
        (orden!(ComprarAnimalCaravana, OrdenComprarAnimalCaravana, caravana_id: s("car-2"), carro_indice: 1, tipo_animal: s("camello")), Comando::AnadirAnimal { caravana: 2, carro: 1, animal: Animal::Camello }),
        (orden!(MoverCarroCaravana, OrdenMoverCarroCaravana, desde_caravana_id: s("car-2"), hacia_caravana_id: s("car-3"), carro_indice: 1), Comando::MoverCarro { de: 2, a: 3, carro: 1 }),
        (orden!(ReservarCaravana, OrdenReservarCaravana, caravana_id: s("car-2"), reservada: true), Comando::ReservarCaravana { caravana: 2, reservada: true }),
        (
            orden!(PrepararCaravana, OrdenPrepararCaravana, caravana_id: s("car-2"), heroe_id: h.clone(), destino_asentamiento_id: s("as-4"), carga: carga(&[("madera", 5_000)]), salir_en: Some(9_000)),
            Comando::PrepararCaravana { caravana: 2, destino: 4, carga: madera(), salir_en: Some(9_000) },
        ),
        (orden!(CancelarCaravana, OrdenCancelarCaravana, caravana_id: s("car-2")), Comando::CancelarCaravana { caravana: 2 }),
        (
            orden!(MoverCargaCaravanaAparcada, OrdenMoverCargaCaravanaAparcada, heroe_id: h.clone(), caravana_id: s("car-2"), asentamiento_id: s("as-3"), recurso: s("madera"), cantidad: 1_000, sentido: s("descargar")),
            Comando::MoverCargaAparcada { caravana: 2, recurso: Madera, cantidad: 1_000, cargar: false },
        ),
        (orden!(EnviarCaravanaAlOrigen, OrdenEnviarCaravanaAlOrigen, heroe_id: h.clone(), caravana_id: s("car-2"), asentamiento_id: s("as-3")), Comando::EnviarCaravanaAlOrigen { caravana: 2 }),
        (orden!(EntregarDeCaravana, OrdenEntregarDeCaravana, ejercito_id: s("ej-3"), caravana_id: s("car-2"), acuerdo_id: s("acu-8")), Comando::EntregarDeCaravana { ejercito: 3, caravana: 2, acuerdo: 8 }),
        (orden!(AdjuntarCaravana, OrdenAdjuntarCaravana, ejercito_id: s("ej-3"), caravana_id: s("car-2"), heroe_id: h.clone()), Comando::AdjuntarCaravana { caravana: 2, ejercito: 3 }),
        (orden!(AparcarCaravana, OrdenAparcarCaravana, caravana_id: s("car-2"), asentamiento_id: s("as-3")), Comando::AparcarCaravana { caravana: 2, asentamiento: 3 }),
        // ── Pueblos extractores ──
        (orden!(MarcarPueblo, OrdenMarcarPueblo, asentamiento_id: s("as-3"), heroe_id: h.clone(), nodo_id: s("nodo-12")), Comando::MarcarPueblo { asentamiento: 3, nodo: 12 }),
        (orden!(DestruirPueblo, OrdenDestruirPueblo, heroe_id: h.clone(), pueblo_id: pueblo()), Comando::DestruirPueblo { pueblo: 4 }),
        (orden!(ReponerCarroDeAcarreo, OrdenReponerCarroDeAcarreo, heroe_id: h.clone(), pueblo_id: pueblo()), Comando::ReponerCarro { pueblo: 4 }),
        (orden!(EntrarEnPueblo, OrdenEntrarEnPueblo, pueblo_id: pueblo(), heroe_id: h.clone()), Comando::EntrarEnPueblo { pueblo: 4, heroe: 7 }),
        (orden!(SalirDePueblo, OrdenSalirDePueblo, pueblo_id: pueblo(), heroe_id: h.clone()), Comando::SalirDePueblo { pueblo: 4, heroe: 7 }),
        (orden!(VaciarCarro, OrdenVaciarCarro, heroe_id: h.clone()), Comando::VaciarCarro { heroe: 7 }),
        (orden!(GuardarEnAlmacenPersonal, OrdenGuardarEnAlmacenPersonal, recurso: s("oro"), cantidad: 30_000), Comando::GuardarEnAlmacenPersonal { heroe: 7, recurso: Oro, cantidad: 30_000 }),
        (orden!(SacarDelAlmacenPersonal, OrdenSacarDelAlmacenPersonal, recurso: s("madera"), cantidad: 5_000), Comando::SacarDelAlmacenPersonal { heroe: 7, recurso: Madera, cantidad: 5_000 }),
        (
            orden!(CargarCaravana, OrdenCargarCaravana, ejercito_id: s("ej-3"), caravana_id: s("car-2"), asentamiento_id: s("as-4"), recurso: s("piedra"), cantidad: 9_000),
            Comando::CargarCaravana { ejercito: 3, caravana: 2, asentamiento: 4, recurso: Piedra, cantidad: 9_000 },
        ),
        (orden!(ReclamarCaravana, OrdenReclamarCaravana, heroe_id: h.clone(), caravana_id: s("car-2")), Comando::ReclamarCaravana { heroe: 7, caravana: 2, politica: Politica::Rechazar }),
        (orden!(SoltarCaravana, OrdenSoltarCaravana, ejercito_id: s("ej-3"), caravana_id: s("car-2"), heroe_id: h.clone()), Comando::SoltarCaravana { ejercito: 3, caravana: 2, heroe: 7 }),
        // ── La Facción, la ciudadanía y la residencia (5B) ──
        (orden!(CrearFaccion, OrdenCrearFaccion, nombre: s("Micenas")), Comando::CrearFaccion { heroe: 7, nombre: "Micenas".into(), sigilo: None }),
        (
            orden!(
                CrearFaccion,
                OrdenCrearFaccion,
                nombre: s("Troya"),
                sigilo: Some(pb::orden_crear_faccion::Sigilo {
                    forma_id: s("a"),
                    campo_id: s("b"),
                    emblema_id: s("c"),
                    color_primario_id: s("d"),
                    color_secundario_id: s("e"),
                    color_emblema_id: s("f"),
                    orla_id: s("g"),
                    color_orla_id: s("h"),
                })
            ),
            Comando::CrearFaccion {
                heroe: 7,
                nombre: "Troya".into(),
                sigilo: Some(politica::sigilo::Sigilo {
                    forma: s("a"),
                    campo: s("b"),
                    emblema: s("c"),
                    color_primario: s("d"),
                    color_secundario: s("e"),
                    color_emblema: s("f"),
                    orla: s("g"),
                    color_orla: s("h"),
                }),
            },
        ),
        (orden!(SolicitarIngreso, OrdenSolicitarIngreso, faccion_id: s("f-2")), Comando::SolicitarIngreso { heroe: 7, faccion: 2 }),
        (orden!(ResponderSolicitud, OrdenResponderSolicitud, faccion_id: s("f-2"), heroe_id: s("heroe-9"), aceptar: true), Comando::ResponderSolicitud { heroe: 7, faccion: 2, solicitante: 9, aceptar: true }),
        (orden!(DejarFaccion, OrdenDejarFaccion), Comando::DejarFaccion { heroe: 7 }),
        (orden!(AsignarRey, OrdenAsignarRey, faccion_id: s("f-2"), heroe_id: s("heroe-9")), Comando::AsignarRey { heroe: 7, faccion: 2, designado: 9 }),
        (orden!(AsignarEmbajador, OrdenAsignarEmbajador, faccion_id: s("f-2"), heroe_id: s("heroe-9")), Comando::AsignarEmbajador { heroe: 7, faccion: 2, designado: 9 }),
        (orden!(CambiarResidencia, OrdenCambiarResidencia, destino_id: s("as-3"), heroe_id: h.clone()), Comando::CambiarResidencia { heroe: 7, asentamiento: 3 }),
        (orden!(ResidirEnCampamento, OrdenResidirEnCampamento, heroe_id: h.clone(), campamento_id: s("camp-4")), Comando::ResidirEnCampamento { heroe: 7, campamento: 4 }),
        (orden!(DejarResidencia, OrdenDejarResidencia, heroe_id: h), Comando::DejarResidencia { heroe: 7 }),
        (orden!(DesignarCapital, OrdenDesignarCapital, faccion_id: s("f-2"), asentamiento_id: s("as-3")), Comando::DesignarCapital { heroe: 7, faccion: 2, asentamiento: 3 }),
        (orden!(RenombrarAsentamiento, OrdenRenombrarAsentamiento, asentamiento_id: s("as-3"), nombre: s(" Tirinto ")), Comando::RenombrarAsentamiento { heroe: 7, asentamiento: 3, nombre: " Tirinto ".into() }),
        // ── Las relaciones (5D) ──
        (
            orden!(ProponerRelacion, OrdenProponerRelacion, tipo: s("vasallaje"), faccion_a_id: s("f-1"), faccion_b_id: s("f-2"), tributo_recurso: Some(s("madera")), tributo_cantidad: Some(4_000)),
            Comando::ProponerRelacion { heroe: 7, a: 1, b: 2, vasallaje: true, tributo: Some((Recurso::Madera, 4_000)) },
        ),
        (orden!(ResponderRelacion, OrdenResponderRelacion, propuesta_id: s("prop-40"), aceptar: true), Comando::ResponderRelacion { heroe: 7, propuesta: 40, aceptar: true }),
        (orden!(RetirarRelacion, OrdenRetirarRelacion, propuesta_id: s("prop-40")), Comando::RetirarRelacion { heroe: 7, propuesta: 40 }),
        (orden!(DeclararGuerra, OrdenDeclararGuerra, faccion_a_id: s("f-1"), faccion_b_id: s("f-2")), Comando::DeclararGuerra { heroe: 7, a: 1, b: 2 }),
        (orden!(ProponerPaz, OrdenProponerPaz, relacion_id: s("rel-41"), faccion_id: s("f-2")), Comando::ProponerPaz { heroe: 7, relacion: 41, faccion: 2 }),
        (orden!(RomperRelacion, OrdenRomperRelacion, relacion_id: s("rel-41"), iniciador_faccion_id: s("f-1")), Comando::RomperRelacion { heroe: 7, relacion: 41, iniciador: 1 }),
        (orden!(RebelionVasallo, OrdenRebelionVasallo, relacion_id: s("rel-41")), Comando::RebelionVasallo { heroe: 7, relacion: 41 }),
        (orden!(AdmitirOtrasFacciones, OrdenAdmitirOtrasFacciones, faccion_id: s("f-1"), admitir: true), Comando::AdmitirOtrasFacciones { heroe: 7, faccion: 1, admitir: true }),
        (orden!(AlternarReabastecerAliados, OrdenAlternarReabastecerAliados, asentamiento_id: s("as-3"), permitido: true), Comando::AlternarReabastecerAliados { asentamiento: 3, permitido: true }),
        (
            orden!(AportarARefundacion, OrdenAportarARefundacion, recurso: s("oro"), cantidad: 40_000, lado: s("carro")),
            Comando::AportarARefundacion { heroe: 7, recurso: Oro, cantidad: 40_000, lado: crate::comando::LadoDelFondo::Carro },
        ),
        (
            orden!(RetirarDeRefundacion, OrdenRetirarDeRefundacion, recurso: s("madera"), cantidad: 3_000, lado: s("almacen")),
            Comando::RetirarDeRefundacion { heroe: 7, recurso: Madera, cantidad: 3_000, lado: crate::comando::LadoDelFondo::Almacen },
        ),
        (orden!(ComprarCaravanaDeRefundacion, OrdenComprarCaravanaDeRefundacion), Comando::ComprarCaravanaDeRefundacion { heroe: 7 }),
        (orden!(LanzarCaravanaFundacion, OrdenLanzarCaravanaFundacion, origen_asentamiento_id: s("as-3")), Comando::LanzarCaravanaFundacion { heroe: 7, asentamiento: 3 }),
        (orden!(DesarmarCaravanaFundacion, OrdenDesarmarCaravanaFundacion, caravana_id: s("car-4")), Comando::DesarmarCaravanaFundacion { heroe: 7, caravana: 4 }),
        // ── La anexión y la fusión (5F) ──
        (orden!(ProponerAnexion, OrdenProponerAnexion, faccion_a_id: s("f-1"), faccion_b_id: s("f-2")), Comando::ProponerAnexion { heroe: 7, absorbente: 1, absorbida: 2 }),
        (orden!(ResponderAnexion, OrdenResponderAnexion, propuesta_id: s("prop-40"), aceptar: true), Comando::ResponderAnexion { heroe: 7, propuesta: 40, aceptar: true }),
        (orden!(RetirarAnexion, OrdenRetirarAnexion, propuesta_id: s("prop-40")), Comando::RetirarAnexion { heroe: 7, propuesta: 40 }),
        (
            orden!(ProponerFusion, OrdenProponerFusion, faccion_a_id: s("f-1"), faccion_b_id: s("f-2"), nuevo_nombre: s(" Argos "), nuevo_rey_id: s("heroe-9")),
            Comando::ProponerFusion { heroe: 7, a: 1, b: 2, nombre: " Argos ".into(), rey: 9 },
        ),
        (orden!(ResponderFusion, OrdenResponderFusion, propuesta_id: s("prop-41"), aceptar: false), Comando::ResponderFusion { heroe: 7, propuesta: 41, aceptar: false }),
        (orden!(RetirarFusion, OrdenRetirarFusion, propuesta_id: s("prop-41")), Comando::RetirarFusion { heroe: 7, propuesta: 41 }),
        // ── Los alijos (6F) ──
        (orden!(AbrirAlijo, OrdenAbrirAlijo, alijo_id: s("alijo-veta-2")), Comando::AbrirAlijo { heroe: 7, alijo: 12 }),
        // ── La adopción (6C) ──
        (orden!(AdoptarTecnologia, OrdenAdoptarTecnologia, faccion_id: s("f-1"), tecnologia_id: s("metalurgia_cobre")), Comando::AdoptarTecnologia { heroe: 7, faccion: 1, tecnologia: 2 }),
        // ── Las épicas de los Aedas residentes (6E) ──
        (orden!(EmpezarEpica, OrdenEmpezarEpica, asentamiento_id: s("as-3"), aeda_id: s("aeda-r4"), tecnologia_id: s("metalurgia_cobre")), Comando::EmpezarEpica { heroe: 7, asentamiento: 3, aeda: 4, tecnologia: 2 }),
        (orden!(AbandonarEpica, OrdenAbandonarEpica, asentamiento_id: s("as-3"), aeda_id: s("aeda-r4")), Comando::AbandonarEpica { heroe: 7, asentamiento: 3, aeda: 4 }),
    ]
}

/// Las órdenes del contrato sin comando en el núcleo, por grupo, con la variante de `Cuerpo` vacía de cada una.
fn ordenes_no_disponibles() -> Vec<Cuerpo> {
    macro_rules! vacias {
        ($($v:ident),* $(,)?) => { vec![$(Cuerpo::$v(Default::default())),*] };
    }
    vacias![
        // Aedas itinerantes (sistema 6)
        ComprarTecnologiaAeda,
    ]
}

fn n(x: i64) -> Kind {
    Kind::NumberValue(x as f64)
}
fn t(x: &str) -> Kind {
    Kind::StringValue(x.into())
}
fn estructura(campos: Vec<(&str, Kind)>) -> Struct {
    Struct { fields: campos.into_iter().map(|(k, v)| (k.into(), prost_types::Value { kind: Some(v) })).collect() }
}
fn punto_kind(x: i64, y: i64) -> Kind {
    Kind::StructValue(estructura(vec![("x", n(x)), ("y", n(y))]))
}

fn administracion(codigo: &str, campos: Vec<(&str, Kind)>) -> Cuerpo {
    Cuerpo::Administracion(pb::OrdenDeAdministracion { codigo: codigo.into(), parametros: Some(estructura(campos)) })
}

/// Una fila por orden de administración: el código, los parámetros y el comando.
fn ordenes_de_administracion() -> Vec<(Cuerpo, Comando)> {
    let tecnologias = Kind::ListValue(prost_types::ListValue { values: vec![prost_types::Value { kind: Some(t("escudos_ligeros")) }] });
    vec![
        (
            administracion("crear_ejercito", vec![("cuenta", t("ana")), ("faccion_id", t("f-1")), ("posicion", punto_kind(8_000, 9_000)), ("velocidad", n(4_000)), ("viveres", n(350_000))]),
            Comando::CrearEjercito { cuenta: "ana".into(), faccion: 1, posicion: Punto::new(8_000, 9_000), velocidad: 4_000, viveres: 350_000 },
        ),
        (administracion("fijar_nivel_faccion", vec![("faccion_id", t("f-2")), ("nivel", n(3))]), Comando::FijarNivelFaccion { faccion: 2, nivel: 3 }),
        (administracion("dar_caravana_fundacion", vec![("ejercito_id", t("ej-3"))]), Comando::DarCaravanaFundacion { ejercito: 3 }),
        (administracion("fijar_nivel_asentamiento", vec![("asentamiento_id", t("as-3")), ("nivel", n(2))]), Comando::FijarNivelAsentamiento { asentamiento: 3, nivel: 2 }),
        (administracion("agrandar_zona", vec![("asentamiento_id", t("as-3"))]), Comando::AgrandarZona { asentamiento: 3 }),
        (administracion("reclamar_parcela", vec![("asentamiento_id", t("as-3"))]), Comando::ReclamarParcela { asentamiento: 3 }),
        (administracion("soltar_parcela", vec![("asentamiento_id", t("as-3")), ("parcela", n(4))]), Comando::SoltarParcela { asentamiento: 3, parcela: 4 }),
        (administracion("dar_recursos", vec![("asentamiento_id", t("as-3")), ("recurso", t("oro")), ("cantidad", n(5_000))]), Comando::DarRecursos { asentamiento: 3, recurso: Oro, cantidad: 5_000 }),
        (administracion("dar_suministro", vec![("ejercito_id", t("ej-3")), ("recurso", t("trigo")), ("cantidad", n(7_000))]), Comando::DarSuministro { ejercito: 3, recurso: Trigo, cantidad: 7_000 }),
        (administracion("dar_almacen_personal", vec![("heroe_id", t("heroe-7")), ("recurso", t("madera")), ("cantidad", n(1_000))]), Comando::DarAlmacenPersonal { heroe: 7, recurso: Madera, cantidad: 1_000 }),
        (administracion("fijar_residencia", vec![("heroe_id", t("heroe-7")), ("residencia", t("camp-4"))]), Comando::FijarResidencia { heroe: 7, residencia: Some(Residencia::Campamento(4)) }),
        (administracion("fijar_tecnologias", vec![("faccion_id", t("f-1")), ("tecnologias", tecnologias)]), Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] }),
        (administracion("fijar_faccion_de_heroe", vec![("heroe_id", t("heroe-7")), ("faccion_id", t("f-1"))]), Comando::FijarFaccionDeHeroe { heroe: 7, faccion: Some(1) }),
        (administracion("colocar_heroe", vec![("heroe_id", t("heroe-7")), ("ubicacion", t("as-3"))]), Comando::ColocarHeroe { heroe: 7, ubicacion: Ubicacion::Asentamiento(3) }),
        (administracion("herir_heroe", vec![("heroe_id", t("heroe-7"))]), Comando::HerirHeroe { heroe: 7 }),
        (administracion("fijar_relacion", vec![("faccion_a_id", t("f-1")), ("faccion_b_id", t("f-2")), ("relacion", t("guerra"))]), Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) }),
        (administracion("colocar_bandidos", vec![("posicion", punto_kind(1_000, 2_000)), ("nivel", n(2))]), Comando::ColocarBandidos { posicion: Punto::new(1_000, 2_000), nivel: 2 }),
        (administracion("dar_escuadra", vec![("heroe_id", t("heroe-7")), ("tropa_id", t("milicia_lanceros")), ("cantidad", n(25))]), Comando::DarEscuadra { heroe: 7, tropa: Tropa::MiliciaLanceros, cantidad: 25 }),
        (administracion("fijar_controlador", vec![("heroe_id", t("heroe-7")), ("controlador", t("bot"))]), Comando::FijarControlador { heroe: 7, controlador: Controlador::Bot }),
    ]
}

#[test]
fn cada_orden_del_jugador_se_traduce_a_su_comando_con_los_datos_de_la_peticion() {
    for (cuerpo, esperado) in ordenes_traducidas() {
        let p = traducir(&Mundo, &en(cuerpo.clone()), "ana", false).unwrap_or_else(|r| panic!("{cuerpo:?}: {r:?}"));
        assert_eq!(p.comando, esperado, "{cuerpo:?}");
        assert_eq!((p.actor.as_str(), p.admin, p.peticion_id.as_str(), p.version_esperada), ("ana", false, "p1", Some(4)));
    }
    assert!(traducir(&Mundo, &en(orden!(Fundar, OrdenFundar)), "ana", true).unwrap().admin, "la sesión de administración lo pone en la petición");
}

#[test]
fn las_ordenes_sin_comando_en_el_nucleo_se_rechazan_con_un_codigo_estable() {
    let sin_comando = ordenes_no_disponibles();
    assert_eq!(sin_comando.len(), 1);
    for cuerpo in sin_comando {
        assert_eq!(traduce(cuerpo.clone(), false), Err(rechazos::ORDEN_NO_DISPONIBLE), "{cuerpo:?}");
        assert_eq!(traduce(cuerpo.clone(), true), Err(rechazos::ORDEN_NO_DISPONIBLE), "también para el administrador: {cuerpo:?}");
    }
    // Una orden vacía tampoco se descarta.
    let vacia = pb::Orden { peticion_id: "p".into(), ..Default::default() };
    assert_eq!(codigo_de(traducir(&Mundo, &vacia, "ana", false)), rechazos::ORDEN_NO_SOPORTADA);
}

#[test]
fn la_administracion_solo_se_traduce_en_una_sesion_de_administracion() {
    let filas = ordenes_de_administracion();
    assert_eq!(filas.len(), 19);
    for (cuerpo, esperado) in filas {
        assert_eq!(traduce(cuerpo.clone(), true), Ok(esperado), "{cuerpo:?}");
        assert_eq!(traduce(cuerpo.clone(), false), Err(rechazos::NO_AUTORIZADO), "un jugador no administra: {cuerpo:?}");
    }
    // Lo opcional ausente o nulo es `None`, y lo mal formado se rechaza con su código.
    let admin = |codigo, campos| traduce(administracion(codigo, campos), true);
    assert_eq!(admin("fijar_residencia", vec![("heroe_id", t("heroe-7"))]), Ok(Comando::FijarResidencia { heroe: 7, residencia: None }));
    assert_eq!(admin("fijar_faccion_de_heroe", vec![("heroe_id", t("heroe-7")), ("faccion_id", Kind::NullValue(0))]), Ok(Comando::FijarFaccionDeHeroe { heroe: 7, faccion: None }));
    assert_eq!(admin("fijar_relacion", vec![("faccion_a_id", t("f-1")), ("faccion_b_id", t("f-2"))]), Ok(Comando::FijarRelacion { a: 1, b: 2, relacion: None }));
    assert_eq!(admin("colocar_heroe", vec![("heroe_id", t("heroe-7")), ("ubicacion", t("ej-3"))]), Ok(Comando::ColocarHeroe { heroe: 7, ubicacion: Ubicacion::Columna(3) }));
    assert_eq!(admin("colocar_heroe", vec![("heroe_id", t("heroe-7")), ("ubicacion", t("camp-1"))]), Ok(Comando::ColocarHeroe { heroe: 7, ubicacion: Ubicacion::Mercenarios(1) }));
    assert_eq!(admin("fijar_residencia", vec![("heroe_id", t("heroe-7")), ("residencia", t("ej-3"))]), Err(rechazos::ORDEN_FORMA_INVALIDA), "la residencia es una plaza o un campamento");
    assert_eq!(admin("herir_heroe", vec![]), Err(rechazos::ORDEN_FORMA_INVALIDA), "falta un parámetro");
    assert_eq!(admin("herir_heroe", vec![("heroe_id", n(7))]), Err(rechazos::ORDEN_FORMA_INVALIDA), "de otro tipo");
    assert_eq!(admin("herir_heroe", vec![("heroe_id", t("pepe"))]), Err(rechazos::HEROE_DESCONOCIDO));
    assert_eq!(admin("fijar_nivel_faccion", vec![("faccion_id", t("f-1")), ("nivel", Kind::NumberValue(2.5))]), Err(rechazos::ORDEN_FORMA_INVALIDA), "un nivel no es fraccionario");
    assert_eq!(admin("fijar_nivel_faccion", vec![("faccion_id", t("f-1")), ("nivel", n(-1))]), Err(rechazos::ORDEN_FORMA_INVALIDA), "ni negativo");
    assert_eq!(admin("dar_recursos", vec![("asentamiento_id", t("as-3")), ("recurso", t("mithril")), ("cantidad", n(1))]), Err(rechazos::ORDEN_FORMA_INVALIDA));
    assert_eq!(admin("hacer_magia", vec![]), Err(rechazos::ORDEN_FORMA_INVALIDA), "un código que no existe");
    let sin_parametros = Cuerpo::Administracion(pb::OrdenDeAdministracion { codigo: "agrandar_zona".into(), parametros: None });
    assert_eq!(traduce(sin_parametros, true), Err(rechazos::ORDEN_FORMA_INVALIDA));
}

#[test]
fn lo_que_la_orden_deja_implicito_se_resuelve_preguntando_al_mundo() {
    use pb::orden_marchar_a::objetivo::Tipo as Marcha;
    let h = || s("heroe-7");
    // `marchar_a` a un asentamiento termina en su plaza; a una plaza que no existe, no.
    let marcha = |tipo| orden!(MarcharA, OrdenMarcharA, heroe_id: h(), objetivo: Some(pb::orden_marchar_a::Objetivo { tipo: Some(tipo) }));
    assert_eq!(traduce(marcha(Marcha::AsentamientoId(s("as-3"))), false), Ok(Comando::MarcharA { ejercito: 70, destino: Punto::new(30_000, 40_000) }));
    assert_eq!(traduce(marcha(Marcha::AsentamientoId(s("as-9"))), false), Err(rechazos::ASENTAMIENTO_DESCONOCIDO));
    assert_eq!(traduce(orden!(MarcharA, OrdenMarcharA, heroe_id: h()), false), Err(rechazos::ORDEN_FORMA_INVALIDA), "sin objetivo");
    assert_eq!(traduce(orden!(MarcharA, OrdenMarcharA, heroe_id: s("heroe-99"), objetivo: Some(pb::orden_marchar_a::Objetivo { tipo: Some(Marcha::Punto(pb::Punto { x: 1, y: 2 })) })), false), Err(rechazos::HEROE_SIN_COLUMNA));
    // `repostar` sin lugar busca el más cercano a la puerta, plaza o avanzada; sin ninguno, `repostar.sin_lugar`.
    let reposta = |heroe: &str| orden!(Repostar, OrdenRepostar, heroe_id: s(heroe));
    assert_eq!(traduce(reposta("heroe-7"), false), Ok(Comando::Repostar { heroe: 7, asentamiento: 3 }));
    assert_eq!(traduce(reposta("heroe-8"), false), Ok(Comando::RepostarEnAvanzada { heroe: 8, avanzada: 6 }));
    assert_eq!(traduce(reposta("heroe-9"), false), Err(rechazos::REPOSTAR_SIN_LUGAR));
    // Una obra es un pueblo o una avanzada; la de un pueblo no se cancela con `cancelar_obra`; una que no existe, tampoco se entrega.
    assert_eq!(traduce(orden!(CancelarObra, OrdenCancelarObra, heroe_id: h(), obra_id: s("obra-5")), false), Err(rechazos::OBJETIVO_NO_SOPORTADO));
    assert_eq!(traduce(orden!(EntregarEnObra, OrdenEntregarEnObra, heroe_id: h(), obra_id: s("obra-7")), false), Err(rechazos::OBRA_DE_MAPA_DESCONOCIDA));
    assert_eq!(traduce(orden!(EntregarEnObra, OrdenEntregarEnObra, heroe_id: h(), obra_id: s("pueblo-5")), false), Err(rechazos::OBRA_DE_MAPA_DESCONOCIDA));
    // Sin campamento, el campamento donde está el héroe de la cuenta.
    assert_eq!(traduce(orden!(RepostarEnCampamento, OrdenRepostarEnCampamento), false), Ok(Comando::RepostarEnCampamento { heroe: 7, campamento: 4 }));
    assert_eq!(traduce(orden!(ComprarEnCampamento, OrdenComprarEnCampamento, recurso: s("trigo"), cantidad: 1_000), false), Ok(Comando::ComprarEnCampamento { heroe: 7, campamento: 4, recurso: Trigo, cantidad: 1_000 }));
    // Una cuenta sin héroe no tiene a quién dar las órdenes que no lo nombran.
    for cuerpo in [orden!(Fundar, OrdenFundar), orden!(ReponerPrestamo, OrdenReponerPrestamo), orden!(CancelarBatalla, OrdenCancelarBatalla, battle_id: s("uuid-9")), orden!(RepostarEnCampamento, OrdenRepostarEnCampamento)] {
        assert_eq!(codigo_de(traducir(&Mundo, &en(cuerpo), "beto", false)), rechazos::HEROE_DESCONOCIDO);
    }
    // La batalla se nombra por su UUID.
    assert_eq!(traduce(orden!(CancelarBatalla, OrdenCancelarBatalla, battle_id: s("uuid-1")), false), Err(rechazos::BATALLA_NO_EXISTE));
    // `quitar_escolta` sin escuadrones retira todas las cedidas; `fijar_puerta` sin grupos cerrados abre a todos, y un grupo repetido cuenta una vez.
    assert_eq!(traduce(orden!(QuitarEscolta, OrdenQuitarEscolta, caravana_id: s("car-2"), heroe_id: h()), false), Ok(Comando::QuitarEscolta { caravana: 2, heroe: 7, escuadras: None }));
    assert_eq!(traduce(orden!(FijarPuerta, OrdenFijarPuerta, asentamiento_id: s("as-3"), heroe_id: h()), false), Ok(Comando::FijarPuerta { heroe: 7, asentamiento: 3, cerrada_a: politica::Grupos::default() }));
    assert_eq!(
        traduce(orden!(FijarPuerta, OrdenFijarPuerta, asentamiento_id: s("as-3"), heroe_id: h(), cerrada_a: v(&["aedas", "aliados", "aedas"])), false),
        Ok(Comando::FijarPuerta { heroe: 7, asentamiento: 3, cerrada_a: politica::Grupos::de([politica::Grupo::Aliados, politica::Grupo::Aedas]) })
    );
    // El ejército y la columna personal salen sin destino: el `objetivo` se rechaza en vez de ignorarse.
    let destino = Some(pb::orden_movilizar_ejercito::Objetivo { tipo: Some(pb::orden_movilizar_ejercito::objetivo::Tipo::AsentamientoId(s("as-3"))) });
    assert_eq!(traduce(orden!(MovilizarEjercito, OrdenMovilizarEjercito, asentamiento_id: s("as-3"), heroe_id: h(), objetivo: destino), false), Err(rechazos::OBJETIVO_NO_SOPORTADO));
    let destino = Some(pb::orden_salir_del_campamento::Objetivo { tipo: Some(pb::orden_salir_del_campamento::objetivo::Tipo::Punto(pb::Punto { x: 1, y: 2 })) });
    assert_eq!(traduce(orden!(SalirDelCampamento, OrdenSalirDelCampamento, campamento_id: s("camp-1"), heroe_id: h(), objetivo: destino), false), Err(rechazos::OBJETIVO_NO_SOPORTADO));
    // La escolta se cede antes de lanzar la caravana.
    assert_eq!(traduce(orden!(PrepararCaravana, OrdenPrepararCaravana, caravana_id: s("car-2"), destino_asentamiento_id: s("as-4"), escolta_escuadron_ids: v(&["esc-1"])), false), Err(rechazos::CARAVANA_ESCOLTA_INVALIDA));
}

#[test]
fn un_id_sin_la_forma_de_su_entidad_o_un_valor_fuera_de_su_catalogo_se_rechazan_con_su_codigo() {
    use pb::orden_atacar::objetivo::Tipo as Ataca;
    let h = || s("heroe-7");
    let casos: Vec<(Cuerpo, &str)> = vec![
        (orden!(EstacionarEjercito, OrdenEstacionarEjercito, ejercito_id: s("heroe-3")), rechazos::EJERCITO_DESCONOCIDO),
        (orden!(Desconectarse, OrdenDesconectarse, heroe_id: s("ana")), rechazos::HEROE_DESCONOCIDO),
        (orden!(AceptarTrueque, OrdenAceptarTrueque, acuerdo_id: s("ord-1")), rechazos::TRUEQUE_DESCONOCIDO),
        (orden!(CancelarOrdenMercado, OrdenCancelarOrdenMercado, orden_id: s("acu-1")), rechazos::ORDEN_DESCONOCIDA),
        (orden!(CrearCaravana, OrdenCrearCaravana, asentamiento_id: s("av-1")), rechazos::ASENTAMIENTO_DESCONOCIDO),
        (orden!(MarcarPueblo, OrdenMarcarPueblo, asentamiento_id: s("as-3"), heroe_id: h(), nodo_id: s("12")), rechazos::PUEBLO_NODO_DESCONOCIDO),
        (orden!(QuitarDeCola, OrdenQuitarDeCola, asentamiento_id: s("as-3"), edificio_id: s("ed-4-7")), rechazos::EDIFICIO_DESCONOCIDO),
        (orden!(AbandonarRecinto, OrdenAbandonarRecinto, asentamiento_id: s("as-3"), recinto_id: s("recinto-4-1")), rechazos::MURALLAS_RECINTO_DESCONOCIDO),
        (orden!(AbrirAlijo, OrdenAbrirAlijo, alijo_id: s("alijo-99")), rechazos::ALIJO_DESCONOCIDO),
        (orden!(AdoptarTecnologia, OrdenAdoptarTecnologia, faccion_id: s("f-1"), tecnologia_id: s("no_existe")), rechazos::TECNOLOGIA_DESCONOCIDA),
        (orden!(EmpezarEpica, OrdenEmpezarEpica, asentamiento_id: s("as-3"), aeda_id: s("aeda-4"), tecnologia_id: s("metalurgia_cobre")), rechazos::EPICA_NO_VIVE_AQUI),
        (orden!(EmpezarEpica, OrdenEmpezarEpica, asentamiento_id: s("as-3"), aeda_id: s("aeda-r4"), tecnologia_id: s("no_existe")), rechazos::TECNOLOGIA_DESCONOCIDA),
        (orden!(AbandonarEpica, OrdenAbandonarEpica, asentamiento_id: s("as-3"), aeda_id: s("r4")), rechazos::EPICA_NO_VIVE_AQUI),
        (orden!(Atacar, OrdenAtacar, heroe_id: h(), objetivo: Some(pb::orden_atacar::Objetivo { tipo: Some(Ataca::CampamentoId(s("camp-1"))) })), rechazos::CAMPAMENTO_DE_BANDIDOS_DESCONOCIDO),
        (orden!(Atacar, OrdenAtacar, heroe_id: h()), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(OrdenarEscuadras, OrdenOrdenarEscuadras, escuadron_ids: v(&["esc-1", "ej-2"])), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(AlternarReceta, OrdenAlternarReceta, asentamiento_id: s("as-3"), recurso: s("mithril")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ReclutarTropa, OrdenReclutarTropa, asentamiento_id: s("as-3"), heroe_id: h(), tropa_id: s("dragones")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ActivarPolitica, OrdenActivarPolitica, asentamiento_id: s("as-3"), cargo: s("rey"), politica_id: s("x")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ComprometerRecinto, OrdenComprometerRecinto, asentamiento_id: s("as-3"), cargo: s("gobernador"), nivel: s("alto")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ConvocarEjercito, OrdenConvocarEjercito, heroe_id: h(), politica_de_union: s("siempre")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ColocarOrdenMercado, OrdenColocarOrdenMercado, asentamiento_id: s("as-3"), tipo: s("regalo"), recurso: s("madera")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(AgregarCarroCaravana, OrdenAgregarCarroCaravana, caravana_id: s("car-2"), tipo_carro: s("dorado")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ComprarAnimalCaravana, OrdenComprarAnimalCaravana, caravana_id: s("car-2"), carro_indice: -1, tipo_animal: s("buey")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ComprarAnimalCaravana, OrdenComprarAnimalCaravana, caravana_id: s("car-2"), carro_indice: 0, tipo_animal: s("dragon")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(MoverEnCola, OrdenMoverEnCola, asentamiento_id: s("as-3"), edificio_id: s("ed-3-1"), direccion: s("al lado")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(MoverCargaCaravanaAparcada, OrdenMoverCargaCaravanaAparcada, caravana_id: s("car-2"), recurso: s("madera"), sentido: s("tirar")), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(FijarPuerta, OrdenFijarPuerta, asentamiento_id: s("as-3"), heroe_id: h(), cerrada_a: v(&["vecinos"])), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(AsignarCargoLocal, OrdenAsignarCargoLocal, asentamiento_id: s("as-3"), cargo: s("rey"), heroe_id: h()), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(UnirseABatalla, OrdenUnirseABatalla, heroe_id: h(), battle_id: s("uuid-9"), lado: Some(s("espectador"))), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ReclutarEnCampamento, OrdenReclutarEnCampamento, tropa_id: s("milicia_lanceros"), pagar_con: Some(s("honor"))), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(SalirAlMundo, OrdenSalirAlMundo, asentamiento_id: s("as-3"), heroe_id: h(), carga: carga(&[("mithril", 1)])), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(GuardarLoadout, OrdenGuardarLoadout, display_name: s("x"), perks_seleccionados: vec![-1]), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ColocarAvanzada, OrdenColocarAvanzada, asentamiento_id: s("as-3"), heroe_id: h()), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ComprarMirada, OrdenComprarMirada, centro: Some(pb::Punto { x: 1, y: 2 })), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ComprarMirada, OrdenComprarMirada, origen: Some(pb::orden_comprar_mirada::Origen { tipo: s("muelle"), id: s("as-3") }), centro: Some(pb::Punto { x: 1, y: 2 })), rechazos::ORDEN_FORMA_INVALIDA),
        (orden!(ComprarInformePlaza, OrdenComprarInformePlaza, origen: Some(pb::orden_comprar_informe_plaza::Origen { tipo: s("campamento"), id: s("as-3") }), asentamiento_id: s("as-8")), rechazos::CAMPAMENTO_DESCONOCIDO),
    ];
    for (cuerpo, codigo) in casos {
        assert_eq!(traduce(cuerpo.clone(), false), Err(codigo), "{cuerpo:?}");
    }
    // Todo rechazo de la traducción lleva la versión del mundo.
    let r = traducir(&Mundo, &en(orden!(EstacionarEjercito, OrdenEstacionarEjercito, ejercito_id: s("x"))), "ana", false).unwrap_err();
    assert_eq!(r.version_tras, 5);
}

/// Las variantes de `Comando`, leídas de su definición: una variante nueva aparece aquí sola.
fn variantes_de_comando() -> BTreeSet<String> {
    include_str!("../../comando.rs")
        .lines()
        .skip_while(|l| *l != "pub enum Comando {")
        .skip(1)
        .take_while(|l| *l != "}")
        .filter_map(|l| l.strip_prefix("    "))
        .filter(|l| l.starts_with(|c: char| c.is_ascii_uppercase()))
        .map(|l| l.split(|c: char| !c.is_alphanumeric()).next().unwrap_or_default().to_owned())
        .collect()
}

#[test]
fn todo_comando_esta_clasificado_y_toda_orden_del_contrato_esta_mapeada_o_es_no_disponible() {
    // Cada variante de `Comando` tiene un único origen: una orden del jugador, la administración o el canal del servidor de batalla (§8, códigos 101-105). El alias `FijarAcceso` se retiró en
    // 5H (5E-12).
    let del_jugador: BTreeSet<String> = ordenes_traducidas().iter().map(|(_, c)| nombre(c)).collect();
    let de_administracion: BTreeSet<String> = ordenes_de_administracion().iter().map(|(_, c)| nombre(c)).collect();
    let del_servidor_de_batalla: BTreeSet<String> = ["RegistrarAsignacion", "ConfirmarInicio", "RegistrarTokens", "RegistrarSalida", "AplicarResultado"].map(String::from).into();
    let todos = variantes_de_comando();
    assert_eq!((todos.len(), del_jugador.len(), de_administracion.len(), del_servidor_de_batalla.len()), (169, 145, 19, 5));
    let clasificados: Vec<&String> = del_jugador.iter().chain(&de_administracion).chain(&del_servidor_de_batalla).collect();
    assert_eq!(clasificados.len(), todos.len(), "una variante en dos orígenes");
    let sin_clasificar: Vec<&String> = todos.iter().filter(|v| !clasificados.contains(v)).collect();
    let inexistentes: Vec<&&String> = clasificados.iter().filter(|v| !todos.contains(**v)).collect();
    assert!(sin_clasificar.is_empty() && inexistentes.is_empty(), "comandos sin clasificar: {sin_clasificar:?}; clasificados que no existen: {inexistentes:?}");

    // Toda orden del contrato (`Orden.cuerpo`) está en una de las tres listas: mapeada, no disponible o la de administración.
    let tabla = ordenes_traducidas().into_iter().map(|(c, _)| c).chain(ordenes_no_disponibles()).chain(ordenes_de_administracion().into_iter().take(1).map(|(c, _)| c));
    let tabla: BTreeSet<String> = tabla.map(|c| nombre(&c)).collect();
    let proto = include_str!("../../../../../proto/bronzeage/v1/orders.proto");
    let en_el_contrato = proto.lines().skip_while(|l| !l.contains("oneof cuerpo {")).skip(1).take_while(|l| l.trim() != "}").map(str::trim).filter(|l| !l.starts_with("//") && l.ends_with(';')).count();
    assert_eq!((tabla.len(), en_el_contrato), (145, 145), "una orden del contrato sin mapear ni clasificar como no disponible");
}

/// El nombre de una orden en el contrato (el campo del `oneof`): la variante de `Cuerpo` en serpiente (`MarcharA` → `marchar_a`).
fn en_el_contrato(variante: &str) -> String {
    let mut s = String::new();
    for (i, c) in variante.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            s.push('_');
        }
        s.push(c.to_ascii_lowercase());
    }
    s
}

/// Cómo dice el documento quién da una orden.
fn quien(a: crate::comando::Autoriza) -> &'static str {
    use crate::comando::Autoriza::*;
    match a {
        Administracion => "administración",
        ServidorDeBatalla => "servidor de batalla",
        SuCuenta => "su cuenta",
        CuentaDelHeroe => "la cuenta del héroe",
        CuentaDelHeroeAunqueFuera => "la cuenta del héroe, aunque esté fuera",
        ReyDeLaFaccion => "la cuenta del héroe, Rey de la Facción",
        CiudadanoDeLaFaccion => "la cuenta del héroe, ciudadano de la Facción (con Rey, solo él)",
        ResidenteDeLaPlaza => "la cuenta del héroe, residente de la plaza",
        AutoridadDiplomatica => "la cuenta del héroe, Rey o Embajador de la Facción",
        Cargo([asentamiento::Cargo::Gobernador]) => "el Gobernador de la plaza, residente y dentro",
        Cargo([asentamiento::Cargo::Tesorero]) => "el Tesorero de la plaza, residente y dentro",
        Cargo([asentamiento::Cargo::Gobernador, asentamiento::Cargo::MaestroObras]) => "el Gobernador o el Maestro de Obras de la plaza, residente y dentro",
        Cargo([asentamiento::Cargo::Gobernador, asentamiento::Cargo::Tesorero]) => "el Gobernador o el Tesorero de la plaza, residente y dentro",
        Cargo(otros) => unreachable!("sin texto para los cargos {otros:?}"),
        CargoDeLaOrden => "el titular del cargo de la orden, residente y dentro",
        ResidenteDentro => "un residente de la plaza, dentro",
        OperaAparcada => "un residente del origen, dentro de la plaza donde está aparcada",
        CargoEnElSitio([asentamiento::Cargo::Gobernador, asentamiento::Cargo::MaestroObras]) => "el Gobernador o el Maestro de Obras de la madre, residente, con su columna en el sitio",
        CargoEnElSitio(otros) => unreachable!("sin texto para los cargos {otros:?}"),
        AsignaCargoLocal => "el Rey (Gobernador) o el Gobernador, residente y dentro (los demás)",
        FijaPuerta => "el Gobernador, residente y dentro, o el Rey de la Facción",
        CompraIntel => "la cuenta del héroe; en una plaza, dentro y Rey, Embajador o Gobernador",
        DirigeAedas => "el Rey de la dueña, el Gobernador o el Sacerdote de la plaza, dentro",
        LiderDeLaColumna => "el Líder de la columna",
        ParticipanteDeLaColumna => "un participante de la columna",
    }
}

/// La matriz de autorización de `Docs/Mechanics/Proyecciones_Y_Autorizacion.md` (§5, el bloque `<!-- autorizacion -->`) es la del código: cada orden del contrato con su comando y quién la da
/// (`Comando::autoriza`), las de administración (`administracion:<codigo>`), los mensajes del canal del servidor de batalla (`servidor:<mensaje>`) y las que aún no existen (`no disponible`).
/// Falla si falta una fila, sobra una o no coincide.
#[test]
fn la_matriz_de_autorizacion_del_documento_es_la_del_codigo() {
    let doc = include_str!("../../../../../Docs/Mechanics/Proyecciones_Y_Autorizacion.md");
    let bloque = doc.split("<!-- autorizacion -->").nth(1).and_then(|b| b.split("<!-- /autorizacion -->").next()).expect("el documento tiene el bloque de la matriz");
    let sin = |s: &str| s.trim().trim_matches('`').to_owned();
    let documento: BTreeSet<(String, String, String)> = bloque
        .lines()
        .filter(|l| l.starts_with("| `"))
        .map(|l| {
            let c: Vec<&str> = l.split('|').collect();
            (sin(c[1]), sin(c[2]), c[3].trim().to_owned())
        })
        .collect();
    let mut codigo: BTreeSet<(String, String, String)> = ordenes_traducidas().iter().map(|(o, c)| (en_el_contrato(&nombre(o)), nombre(c), quien(c.autoriza()).to_owned())).collect();
    for (o, c) in ordenes_de_administracion() {
        let Cuerpo::Administracion(a) = o else { unreachable!() };
        codigo.insert((format!("administracion:{}", a.codigo), nombre(&c), quien(c.autoriza()).to_owned()));
    }
    // El canal del servidor de batalla (`MensajeDeServidorDeBatalla`): sus cinco comandos son `Autoriza::ServidorDeBatalla` en `Comando::autoriza`.
    for (m, c) in [("asignacion", "RegistrarAsignacion"), ("inicio", "ConfirmarInicio"), ("tokens", "RegistrarTokens"), ("salida", "RegistrarSalida"), ("resultado", "AplicarResultado")] {
        codigo.insert((format!("servidor:{m}"), c.to_owned(), "servidor de batalla".to_owned()));
    }
    codigo.extend(ordenes_no_disponibles().iter().map(|o| (en_el_contrato(&nombre(o)), "—".to_owned(), "no disponible".to_owned())));
    let fila = |(o, c, q): &(String, String, String)| format!("| `{o}` | {} | {q} |", if c == "—" { c.clone() } else { format!("`{c}`") });
    let faltan: Vec<String> = codigo.difference(&documento).map(fila).collect();
    let sobran: Vec<String> = documento.difference(&codigo).map(fila).collect();
    assert!(faltan.is_empty() && sobran.is_empty(), "la matriz del documento no es la del código.\nFaltan o no coinciden (como las dice el código):\n{}\nSobran (como las dice el documento):\n{}", faltan.join("\n"), sobran.join("\n"));
    assert_eq!(codigo.len(), 145 + 19 + 5 + 1, "cada comando de una orden del jugador (dos órdenes dan dos), la administración, el servidor de batalla y las no disponibles");
}
