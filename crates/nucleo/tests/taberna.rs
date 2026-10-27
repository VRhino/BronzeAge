//! La intel de las tabernas (4H T3d; Niebla_De_Guerra §6): comprar una Mirada (ve sin explorar, con cupo por taberna, enfriamiento por zona y precio por distancia) y el Informe de una plaza ajena que
//! la Facción conoce (con su enfriamiento, su precio por nivel y el aviso sin firma a la plaza espiada), en un campamento de mercenarios (paga el héroe) y en una plaza con Taberna (paga su almacén); lo
//! que la vista enseña de todo ello y lo que viaja en la foto. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;
use asentamiento::catalogo::Tipo;
use asentamiento::Recurso::{self, *};
use base::{Instante, Mil, Punto, MS_POR_MINUTO};
use contrato::v1::{entidad::Cuerpo, TipoEntidad};
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::memoria::OrigenIntel;
use nucleo::proyeccion::ids;
use nucleo::proyeccion::vista::{Mirada, Perspectiva};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Estado, Nucleo, Peticion, Reglas, Resultado, Respuesta};
use rutas::Rutas;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};
use vision::{Mascara, Rect};

const MIN: Instante = MS_POR_MINUTO;
const HORA: Instante = 60 * MIN;
const U: Mil = 1_000;

type M = (Arc<Mundo>, Arc<Rutas>);

fn mundo() -> Option<&'static M> {
    static M: OnceLock<Option<M>> = OnceLock::new();
    M.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo");
        let f = std::fs::read_dir(dir).ok()?.flatten().find(|e| e.file_name().to_string_lossy().starts_with("mundo-"))?;
        let m = Mundo::cargar(&std::fs::read(f.path()).ok()?, &OpcionesValidacion::default()).expect("artefacto válido");
        let r = Arc::new(Rutas::desde_mundo(&m));
        Some((Arc::new(m), r))
    })
    .as_ref()
}

macro_rules! con_mundo {
    () => {
        match mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

fn sitio_rico(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let c = m.campamentos()[i].posicion.unwrap();
    for r in (0..8).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6) {
                return p;
            }
        }
    }
    panic!("sin sitio rico cerca del campamento {i}");
}

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

/// `eva` (F1) reside y está dentro del campamento 0 con 500 de oro en su almacén personal; la plaza de F2 (`gus`) está cerca; la plaza de F1 (`fundador_a`), en otro sitio.
struct Esc {
    n: Nucleo,
    eva: u64,
    ajena: u64,
    propia: u64,
    peticiones: u64,
}

impl Esc {
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.peticiones += 1;
        let p = Peticion::nueva(actor, &format!("p{}", self.peticiones), c);
        let t = self.n.ahora();
        // 5I (5E-10): sin el andamiaje de `comun::mandaba`; cada cuenta da sus órdenes de plaza con la matriz real.
        self.n.aplicar_en(t, if actor == "admin" { p.de_admin() } else { p })
    }

    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        r
    }

    /// Un rechazo que no deja rastro.
    fn rechazada(&mut self, actor: &str, c: Comando) -> &'static str {
        let antes = (self.n.version(), self.n.hash_estado());
        let r = self.ord(actor, c);
        assert_eq!((self.n.version(), self.n.hash_estado()), antes, "el rechazo {r:?} dejó rastro");
        codigo(&r)
    }

    fn plaza(&mut self, cuenta: &str, faccion: u32, p: Punto) -> u64 {
        let t = self.n.ahora();
        let col = self.n.crear_ejercito(t, cuenta, faccion, p, 500, 100_000).unwrap();
        self.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
        let plaza = self.ok(cuenta, Comando::Fundar { ejercito: col }).creadas[0];
        self.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        plaza
    }

    fn oro_de_eva(&self) -> Mil {
        self.n.heroe(self.eva).unwrap().almacen_personal.get(&Oro).copied().unwrap_or(0)
    }

    fn mirada(&mut self, centro: Punto) -> Respuesta {
        let eva = self.eva;
        self.ord("eva", Comando::ComprarMirada { heroe: eva, origen: OrigenIntel::Campamento(0), centro })
    }
}

fn escenario(m: &M) -> Esc {
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let mut x = Esc { n, eva: 0, ajena: 0, propia: 0, peticiones: 0 };
    let (sa, sb) = (sitio_rico(&x.n, &m.0, 3, 1), sitio_rico(&x.n, &m.0, 0, 2));
    x.propia = x.plaza("fundador_a", 1, sa);
    x.ajena = x.plaza("gus", 2, sb);
    x.eva = x.ok("eva", Comando::CrearHeroe { cuenta: "eva".into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
    let eva = x.eva;
    x.ok("admin", Comando::FijarFaccionDeHeroe { heroe: eva, faccion: Some(1) });
    x.ok("admin", Comando::DarAlmacenPersonal { heroe: eva, recurso: Oro, cantidad: 500_000 });
    let h = x.n.heroe(eva).unwrap();
    assert_eq!((h.heroe.residencia, h.heroe.presencia.ubicacion), (Some(Residencia::Campamento(0)), Ubicacion::Mercenarios(0)), "nace residente y dentro del campamento");
    x
}

fn todo() -> Rect {
    Rect { min: Punto::new(0, 0), max: Punto::new(2_667_000, 2_000_000) }
}

/// `d` unidades hacia el centro del mapa desde `p` (para no salirse de él).
fn hacia_dentro(p: Punto, d: Mil) -> Punto {
    Punto::new(if p.x > 1_333_000 { p.x - d } else { p.x + d }, p.y)
}

/// Lo que la vista de F1 enseña: `(clase, id) → cuerpo`, y la niebla.
fn vista_f1(n: &Nucleo) -> (BTreeMap<(TipoEntidad, String), Cuerpo>, Mascara, Mascara) {
    let m = Mirada::de(n, Perspectiva::Faccion(1));
    let v = m.contenido(None, todo(), &BTreeMap::new()).into_iter().filter_map(|x| Some(((TipoEntidad::try_from(x.clave.0).ok()?, x.clave.1.clone()), x.cuerpo()?))).collect();
    let f = m.niebla().cloned().unwrap();
    let mascara = |b: &[u8]| Mascara::desde_bytes(f.columnas as usize, f.filas as usize, i64::from(f.tamano_celda), b);
    (v, mascara(&f.celdas), mascara(&f.visibles))
}

#[test]
fn la_mirada_ve_sin_explorar_cuesta_segun_lo_lejos_tiene_cupo_y_enfria_su_zona() {
    let m = con_mundo!();
    let mut e = escenario(m);
    let sitio = e.n.estado().asentamientos[&e.ajena].posicion;
    let puerta = e.n.puerta_de_campamento(0).unwrap();
    let as_ajena = ids::asentamiento(e.ajena);
    assert!(!vista_f1(&e.n).0.contains_key(&(TipoEntidad::Asentamiento, as_ajena.clone())), "sin Mirada, F1 no ve la plaza de F2");

    // Lo que no se puede: fuera del mapa, sin estar en el campamento, sin Facción, sin oro.
    assert_eq!(codigo(&e.mirada(Punto::new(-1, 0))), rechazos::INTEL_FUERA_DEL_MAPA);
    let otro = e.ok("leo", Comando::CrearHeroe { cuenta: "leo".into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
    assert_eq!(e.rechazada("leo", Comando::ComprarMirada { heroe: otro, origen: OrigenIntel::Campamento(0), centro: sitio }), rechazos::INTEL_SIN_FACCION);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: otro, faccion: Some(1) });
    assert_eq!(e.rechazada("leo", Comando::ComprarMirada { heroe: otro, origen: OrigenIntel::Campamento(1), centro: sitio }), rechazos::INTEL_NO_PRESENTE);
    assert_eq!(e.rechazada("leo", Comando::ComprarMirada { heroe: otro, origen: OrigenIntel::Campamento(0), centro: sitio }), rechazos::INTEL_SIN_ORO, "leo no tiene oro");

    // Eva compra una Mirada sobre la plaza de F2: 40 de oro más 0,5 por unidad de distancia a su ojo más cercano (la taberna del campamento), redondeado arriba.
    let antes = e.oro_de_eva();
    let r = e.mirada(sitio);
    assert!(r.aceptada(), "{r:?}");
    let id = r.creadas[0];
    let precio = ((40_000 + (puerta.dist(sitio) * 500 + 999) / 1_000) + 999) / 1_000 * 1_000;
    assert_eq!(antes - e.oro_de_eva(), precio, "precio por distancia: {} u", puerta.dist(sitio) / 1_000);
    let (v, celdas, visibles) = vista_f1(&e.n);
    assert!(matches!(v.get(&(TipoEntidad::Asentamiento, as_ajena.clone())), Some(Cuerpo::Asentamiento(a)) if a.almacen.is_empty()), "con la Mirada se ve la plaza, redactada");
    assert!(matches!(v.get(&(TipoEntidad::MiradaIntel, ids::mirada(id))), Some(Cuerpo::Mirada(x)) if x.origen_id == "camp-0" && x.expira_en_ms == e.n.ahora() + 120 * MIN && x.libre_en_ms == e.n.ahora() + 240 * MIN));
    assert!(visibles.explorado_en(sitio) && !celdas.explorado_en(sitio), "ve, pero no explora");
    assert!(e.n.estado().memoria.exploracion.get(&1).is_none_or(|m| !m.explorado_en(sitio)) && !e.n.estado().memoria.fichas.contains_key(&1), "ni deja memoria");

    // El campamento admite una Mirada abierta por Facción; y una zona mirada se enfría.
    assert_eq!(codigo(&e.mirada(hacia_dentro(sitio, 300 * U))), rechazos::INTEL_CUPO_LLENO);
    e.n.avanzar_hasta(e.n.ahora() + 121 * MIN);
    let (v, _, visibles) = vista_f1(&e.n);
    assert!(!v.contains_key(&(TipoEntidad::Asentamiento, as_ajena.clone())) && !visibles.explorado_en(sitio), "cerrada la Mirada, no se ve");
    assert!(v.contains_key(&(TipoEntidad::MiradaIntel, ids::mirada(id))), "pero sigue enfriando su zona");
    assert_eq!(codigo(&e.mirada(hacia_dentro(sitio, 10 * U))), rechazos::INTEL_ZONA_RECIENTE);
    assert!(e.mirada(hacia_dentro(sitio, 40 * U)).aceptada(), "fuera de su radio, otra zona");
    e.n.avanzar_hasta(e.n.ahora() + 241 * MIN);
    assert!(!vista_f1(&e.n).0.contains_key(&(TipoEntidad::MiradaIntel, ids::mirada(id))), "enfriada, la Mirada deja de viajar");
    assert!(e.mirada(sitio).aceptada(), "y la zona se puede volver a mirar");
}

#[test]
fn el_informe_de_una_plaza_ajena_exige_conocerla_cuesta_por_nivel_avisa_sin_firma_y_se_recuerda_con_su_fecha() {
    let m = con_mundo!();
    let mut e = escenario(m);
    let (eva, ajena, propia) = (e.eva, e.ajena, e.propia);
    let sitio = e.n.estado().asentamientos[&ajena].posicion;
    let informe = |e: &mut Esc, asentamiento: u64| e.ord("eva", Comando::ComprarInformePlaza { heroe: eva, origen: OrigenIntel::Campamento(0), asentamiento });
    assert_eq!(e.rechazada("eva", Comando::ComprarInformePlaza { heroe: eva, origen: OrigenIntel::Campamento(0), asentamiento: ajena }), rechazos::INTEL_PLAZA_DESCONOCIDA, "no la conoce");
    assert_eq!(codigo(&informe(&mut e, propia)), rechazos::INTEL_PLAZA_PROPIA);

    // Con una Mirada abierta sobre ella, la conoce: el Informe cuesta 60 de oro por nivel.
    assert!(e.mirada(sitio).aceptada());
    let nivel = e.n.estado().asentamientos[&ajena].nivel();
    let antes = e.oro_de_eva();
    let v0 = e.n.version();
    assert!(informe(&mut e, ajena).aceptada());
    assert_eq!(antes - e.oro_de_eva(), 60_000 * i64::from(nivel));
    let aviso = e.n.cambios_desde(v0).unwrap().flat_map(|c| c.eventos.iter()).find(|ev| ev.codigo == "asentamiento.informe_pedido").expect("la plaza espiada se entera");
    assert_eq!((aviso.asentamientos.as_slice(), aviso.datos.is_empty()), (&[ajena][..], true), "sin decir quién");
    let guardado = e.n.estado().memoria.informes[&1][&ajena].clone();
    assert_eq!((guardado.conocido_en, guardado.nivel, guardado.posicion), (e.n.ahora(), nivel, sitio));
    let (v, _, _) = vista_f1(&e.n);
    let Some(Cuerpo::Informe(i)) = v.get(&(TipoEntidad::InformePlaza, ids::asentamiento(ajena))) else { panic!("la vista enseña el informe") };
    assert_eq!((i.faccion_id.as_str(), i.conocido_en_ms, i.edificios.is_empty()), ("f-2", e.n.ahora(), false));
    let (activo, en_obra) = (contrato::v1::EstadoEdificio::EdificioActivo as i32, contrato::v1::EstadoEdificio::EdificioEnConstruccion as i32);
    assert!(i.edificios.iter().all(|b| b.estado == activo || b.estado == en_obra), "solo lo levantado y lo que está en obra, nunca la cola");

    // Otro de la misma plaza tiene que esperar su enfriamiento (6 horas); cuando se puede, sustituye al anterior.
    assert_eq!(codigo(&informe(&mut e, ajena)), rechazos::INTEL_INFORME_RECIENTE);
    e.n.avanzar_hasta(e.n.ahora() + 6 * HORA + MIN);
    e.n.avanzar_hasta(e.n.ahora() + 4 * HORA);
    assert!(e.mirada(sitio).aceptada(), "sin Mirada abierta ni ficha no la conocería: otra Mirada");
    assert!(informe(&mut e, ajena).aceptada());
    assert!(e.n.estado().memoria.informes[&1][&ajena].conocido_en > guardado.conocido_en);

    // La intel viaja en la foto.
    let bytes = e.n.estado().serializar();
    let vuelta = Estado::deserializar(&bytes, &e.n.estado().reglas.clone(), m.0.hash()).unwrap();
    assert_eq!((vuelta.memoria.miradas.len(), vuelta.memoria.informes[&1].len()), (e.n.estado().memoria.miradas.len(), 1));
    assert_eq!(vuelta.serializar(), bytes);
}

#[test]
fn en_una_plaza_hace_falta_su_taberna_estar_dentro_y_paga_su_almacen() {
    let m = con_mundo!();
    let mut e = escenario(m);
    let (propia, eva) = (e.propia, e.eva);
    let centro = e.n.estado().asentamientos[&propia].posicion;
    let comprar = |e: &mut Esc| e.ord("eva", Comando::ComprarMirada { heroe: eva, origen: OrigenIntel::Plaza(propia), centro: hacia_dentro(centro, 50 * U) });
    // En una plaza compran el Rey, el Embajador o el Gobernador, dentro de ella (`compraIntel`, 5E): eva es la Embajadora de F1.
    let rey = e.n.faccion(1).and_then(|f| f.rey).expect("F1 tiene Rey");
    e.ok("admin", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: eva });
    assert_eq!(codigo(&comprar(&mut e)), rechazos::NO_AUTORIZADO, "eva está en el campamento: fuera de la plaza no compra");
    e.ok("admin", Comando::ColocarHeroe { heroe: eva, ubicacion: Ubicacion::Asentamiento(propia as u32) });
    assert_eq!(codigo(&comprar(&mut e)), rechazos::INTEL_SIN_TABERNA);
    assert_eq!(codigo(&e.ord("eva", Comando::ComprarMirada { heroe: eva, origen: OrigenIntel::Plaza(e.ajena), centro })), rechazos::INTEL_PLAZA_AJENA);

    // Una Taberna levantada (nivel 2) admite una Mirada; la paga el almacén de la plaza.
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: propia, nivel: 2 });
    for r in [Madera, Piedra, Oro, Trigo] {
        e.ok("admin", Comando::DarRecursos { asentamiento: propia, recurso: r, cantidad: 400_000 });
    }
    e.ok("admin", Comando::AnadirEdificio { asentamiento: propia, tipo: Tipo::Taberna });
    let activa = |e: &Esc| e.n.economia(propia).unwrap().edificios.iter().any(|b| b.tipo == Tipo::Taberna && b.activo());
    for _ in 0..40 {
        if activa(&e) {
            break;
        }
        e.n.avanzar_hasta(e.n.ahora() + HORA);
        for r in [Madera, Piedra, Trigo] {
            e.ok("admin", Comando::DarRecursos { asentamiento: propia, recurso: r, cantidad: 100_000 });
        }
    }
    assert!(activa(&e), "la Taberna acaba por levantarse");
    let oro = |e: &Esc| e.n.economia(propia).unwrap().almacen.cantidad(Recurso::Oro);
    let (antes, personal) = (oro(&e), e.oro_de_eva());
    assert!(comprar(&mut e).aceptada());
    assert!(antes - oro(&e) >= 40_000 && e.oro_de_eva() == personal, "paga la plaza, no el héroe");
    assert_eq!(codigo(&e.ord("eva", Comando::ComprarMirada { heroe: eva, origen: OrigenIntel::Plaza(propia), centro: hacia_dentro(centro, 300 * U) })), rechazos::INTEL_CUPO_LLENO, "nivel 1: una a la vez");
    assert!(e.n.estado().memoria.miradas.iter().any(|x| x.origen == OrigenIntel::Plaza(propia)));
}
