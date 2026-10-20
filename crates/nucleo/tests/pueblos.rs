//! Los pueblos extractores y su carro de acarreo (fase 3C, Economia_Construccion §14, Doc 4.2.2; Comercio_Caravanas segunda §12): la obra de
//! mapa que se marca y se levanta con materiales entregados en el sitio, la extracción contra el almacén local, el carro que sale lleno hacia
//! la plaza madre, la ruina y la conquista de la madre, determinismo y recuperación desde foto. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

use asentamiento::catalogo::Tipo;
use asentamiento::pueblos::{EstadoPueblo, IngresoAcarreo};
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::caravana::{EstadoCaravana, TipoCaravana};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::economia::Mapa;
use nucleo::{rechazos, Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado, Respuesta};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;

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

/// Las reglas por defecto sin acoso ni anillo de bandidos: estos tests miden otra cosa y se calibraron sin ellos (los de `combate.rs` miden los bandidos).
fn sin_bandidos() -> Reglas {
    let mut r = Reglas::default();
    let b = &mut std::sync::Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    r
}

fn arranque(m: &M, semilla: u64) -> Arranque {
    Arranque { reglas: sin_bandidos(), ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

type Hecho = (Instante, Vec<u64>, Vec<(&'static str, i64)>);

fn hechos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.asentamientos.clone(), e.datos.clone())).collect()
}

fn dato(h: &Hecho, clave: &str) -> i64 {
    h.2.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("el hecho no lleva «{clave}»: {h:?}")).1
}

/// Un sitio fundable, verde y con mucho bosque, a `r` del punto `c`: una plaza que no cae en ruinas por falta de madera.
fn sitio_junto_a(n: &Nucleo, c: Punto, r: i64) -> Option<Punto> {
    (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64)).find(|&p| {
        n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6 && !v.condenada)
    })
}

/// La plaza madre de Ana (F1) a 6 u de un nodo de cobre, ya de nivel 2, con la auto-construcción parada y la zona agrandada para que el nodo
/// quede dentro. El tiempo corre solo cuando se pide.
struct Escenario {
    n: Nucleo,
    plaza: u64,
    /// Índice (en `Mapa::nodos`) del nodo de cobre junto a la plaza.
    cobre: u32,
    mapa: Arc<Mapa>,
    cuenta: u64,
}

impl Escenario {
    fn peticion(&mut self, actor: &str, c: Comando) -> Peticion {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        if actor == "admin" {
            p.de_admin()
        } else {
            p
        }
    }
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        let (t, p) = (self.n.ahora(), self.peticion(actor, c));
        self.n.aplicar_en(t, p)
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn ok_en(&mut self, t: Instante, actor: &str, c: Comando) -> Respuesta {
        let p = self.peticion(actor, c);
        let r = self.n.aplicar_en(t, p);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn rechazada(&mut self, actor: &str, c: Comando) -> &'static str {
        let r = self.ord(actor, c);
        codigo(&r)
    }
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    fn dar(&mut self, recurso: Recurso, cantidad: i64) {
        let plaza = self.plaza;
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    fn almacen(&self, r: Recurso) -> i64 {
        self.n.economia(self.plaza).unwrap().almacen.cantidad(r)
    }
    /// Una columna de `faccion` en `p`.
    fn columna(&mut self, dueno: &str, faccion: u32, p: Punto) -> u64 {
        let t = self.n.ahora();
        self.n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap()
    }
    /// Marca un pueblo de cobre en el nodo de la plaza.
    fn marcar(&mut self) -> u64 {
        let (plaza, nodo) = (self.plaza, self.cobre);
        self.ok("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo }).creadas[0]
    }
    fn pueblo(&self, id: u64) -> &asentamiento::pueblos::Pueblo {
        self.n.pueblo(id).expect("el pueblo existe").1
    }
    /// Una columna de Ana en la puerta del pueblo con todo el material de su obra (100 de madera y 12 de oro).
    fn llevar_materiales(&mut self, pueblo: u64) -> u64 {
        let sitio = self.n.posicion_de_pueblo(pueblo).unwrap();
        let col = self.columna("ana", 1, sitio);
        self.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 100_000 });
        self.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 12_000 });
        col
    }
    /// Marca un pueblo, lo levanta con todo el material y espera a que acabe la obra. Devuelve su id.
    fn pueblo_levantado(&mut self) -> u64 {
        let id = self.marcar();
        let col = self.llevar_materiales(id);
        self.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
        let EstadoPueblo::EnObra { completa_en } = self.pueblo(id).estado else { panic!("debía estar en obra") };
        self.n.avanzar_hasta(completa_en + MIN);
        assert!(self.pueblo(id).activo());
        id
    }
}

fn montar(m: &M, semilla: u64, nivel: u32) -> Escenario {
    montar_con(Nucleo::nuevo(arranque(m, semilla)), m, nivel)
}

fn montar_con(mut n: Nucleo, m: &M, nivel: u32) -> Escenario {
    let mapa = Mapa::de(&m.0);
    let (cobre, sitio) = (0..mapa.nodos.len())
        .filter(|&i| mapa.nodos[i].tipo == Tipo::MinaCobre)
        .find_map(|i| sitio_junto_a(&n, mapa.nodos[i].pos, 6_000).map(|p| (i as u32, p)))
        .expect("hay un nodo de cobre junto a un sitio rico");
    let e = n.crear_ejercito(0, "ana", 1, sitio, 500, 100_000).unwrap();
    assert!(n.aplicar_en(0, Peticion::nueva("admin", "c", Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(0, Peticion::nueva("ana", "f", Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    let plaza = r.creadas[0];
    let mut esc = Escenario { n, plaza, cobre, mapa, cuenta: 0 };
    esc.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
    if nivel > 1 {
        esc.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel });
        // El techo de la zona sube con el nivel: se agranda hasta el del nivel 2 (15 u) para tener nodos con los que elegir.
        for _ in 0..9 {
            esc.ok("admin", Comando::AgrandarZona { asentamiento: plaza });
        }
    }
    assert!(esc.n.en_zona(plaza, esc.mapa.nodos[cobre as usize].pos) || nivel == 1, "el nodo de cobre cae en la zona");
    esc
}

#[test]
fn el_almacen_local_del_pueblo_es_del_tamano_del_carro_de_acarreo() {
    let r = Reglas::default().economia;
    assert_eq!(r.datos.pueblos.capacidad_almacen, comercio::acarreo::capacidad(&r.comercio.flota), "con un almacén menor que el carro, «sale lleno» no se dispararía nunca");
}

// ───────────────────────────── 1. La obra de mapa ─────────────────────────────

#[test]
fn una_obra_de_mapa_se_marca_ocupa_cupo_y_no_pasa_por_la_cola_ni_cuesta_nada_a_la_plaza() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    assert!(e.n.candidatos_de_pueblo(plaza).contains(&(cobre, Tipo::MinaCobre)), "el panel ofrece el nodo de cobre de la zona");
    let (madera, edificios) = (e.almacen(Madera), e.n.economia(plaza).unwrap().edificios.len());
    e.columna("beto", 2, Punto::new(1_000, 1_000));
    assert_eq!(e.rechazada("beto", Comando::MarcarPueblo { asentamiento: plaza, nodo: cobre }), rechazos::NO_AUTORIZADO);

    let id = e.marcar();
    let (madre, p) = e.n.pueblo(id).unwrap();
    assert_eq!((madre, p.tipo, p.nodo, p.carro), (plaza, Tipo::MinaCobre, cobre, None));
    let EstadoPueblo::Pendiente { requerido, entregado } = &p.estado else { panic!("pendiente de materiales") };
    assert_eq!(requerido, &vec![(Madera, 100_000), (Oro, 12_000)], "su extractor (30) + 50 + el carro (20) y el buey (12 de oro)");
    assert!(entregado.iter().all(|&(_, n)| n == 0));
    let eco = e.n.economia(plaza).unwrap();
    assert_eq!((e.almacen(Madera), eco.edificios.len(), eco.pueblos.len()), (madera, edificios, 1), "no pasa por la cola de la ciudad ni paga del almacén");
    assert!(!e.n.candidatos_de_pueblo(plaza).iter().any(|&(n, _)| n == cobre), "el nodo ya está reclamado");
    let h = hechos(&e.n, "pueblo.marcado");
    assert_eq!((h.len(), dato(&h[0], "pueblo"), dato(&h[0], "nodo")), (1, id as i64, cobre as i64));

    // Un nodo ocupado, uno que no existe, uno de ganado y uno fuera de la zona se rechazan.
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: cobre }), rechazos::PUEBLO_NODO_OCUPADO);
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: u32::MAX }), rechazos::PUEBLO_NODO_DESCONOCIDO);
    let ganado = (0..e.mapa.nodos.len()).find(|&i| e.mapa.nodos[i].tipo == Tipo::Corral).unwrap() as u32;
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: ganado }), rechazos::PUEBLO_NODO_NO_EXTRAIBLE);
    let lejos = (0..e.mapa.nodos.len()).find(|&i| e.mapa.nodos[i].tipo == Tipo::MinaEstano && !e.n.en_zona(plaza, e.mapa.nodos[i].pos)).unwrap() as u32;
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: lejos }), rechazos::PUEBLO_FUERA_DE_LA_ZONA);
}

#[test]
fn exige_madre_de_nivel_2_y_respeta_el_cupo_que_libera_destruirlo_a_mano() {
    let m = con_mundo!();
    let mut uno = montar(m, 7, 1);
    let (plaza, cobre) = (uno.plaza, uno.cobre);
    assert_eq!(uno.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: cobre }), rechazos::PUEBLO_NIVEL_INSUFICIENTE);

    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let primero = e.marcar();
    // Dos pueblos en el nivel 2: el segundo, de lo que ofrezca el panel; el tercero se rechaza aunque haya candidatos.
    let candidatos = e.n.candidatos_de_pueblo(plaza);
    assert!(candidatos.len() >= 2, "la zona de nivel 2 ofrece nodos para otros dos pueblos: {candidatos:?}");
    assert!(candidatos.iter().all(|c| c.1 != Tipo::MinaHierro), "el hierro pide una tecnología que la Facción no tiene: no es candidato");
    let segundo = e.ok("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: candidatos[0].0 }).creadas[0];
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: candidatos[1].0 }), rechazos::PUEBLO_SIN_CUPO);
    // Destruirlo a mano (aunque esté pendiente) libera el cupo y el nodo, y no devuelve nada.
    assert_eq!(e.rechazada("beto", Comando::DestruirPueblo { pueblo: primero }), rechazos::NO_AUTORIZADO);
    e.ok("ana", Comando::DestruirPueblo { pueblo: primero });
    assert!(e.n.pueblo(primero).is_none() && e.n.pueblo(segundo).is_some());
    assert_eq!(e.rechazada("ana", Comando::DestruirPueblo { pueblo: primero }), rechazos::PUEBLO_DESCONOCIDO);
    assert_eq!(hechos(&e.n, "pueblo.destruido").len(), 1);
    e.ok("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: candidatos[1].0 });
    assert_eq!(e.n.pueblos_de(plaza).len(), 2);
}

#[test]
fn los_minerales_y_la_segunda_cantera_ya_no_se_construyen_dentro_de_la_ciudad() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    e.dar(Madera, 300_000);
    for tipo in [Tipo::MinaCobre, Tipo::MinaEstano, Tipo::Mina] {
        assert_eq!(e.rechazada("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo }), rechazos::OBRA_SOLO_POR_PUEBLO, "{tipo:?}");
    }
    e.ok("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Cantera });
    assert_eq!(e.rechazada("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Cantera }), rechazos::OBRA_SOLO_POR_PUEBLO, "la primera es directa; las demás, por pueblo");
}

// ───────────────────────────── 2. Los materiales y la obra ─────────────────────────────

#[test]
fn los_materiales_se_entregan_en_el_sitio_con_una_columna_de_la_faccion_y_con_todo_entregado_arranca_la_obra() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let id = e.marcar();
    let sitio = e.n.posicion_de_pueblo(id).unwrap();
    let col = e.columna("ana", 1, sitio);
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 60_000 });
    // La puerta, la Facción y el propietario de la columna se comprueban.
    let lejos = e.columna("ana", 1, Punto::new(sitio.x + 20_000, sitio.y));
    e.ok("admin", Comando::DarSuministro { ejercito: lejos, recurso: Madera, cantidad: 10_000 });
    assert_eq!(e.rechazada("ana", Comando::EntregarMateriales { ejercito: lejos, pueblo: id }), rechazos::PUEBLO_FUERA_DE_LA_PUERTA);
    let ajena = e.columna("beto", 2, sitio);
    e.ok("admin", Comando::DarSuministro { ejercito: ajena, recurso: Madera, cantidad: 10_000 });
    assert_eq!(e.rechazada("beto", Comando::EntregarMateriales { ejercito: ajena, pueblo: id }), rechazos::PUEBLO_FACCION_AJENA);
    assert_eq!(e.rechazada("beto", Comando::EntregarMateriales { ejercito: col, pueblo: id }), rechazos::NO_AUTORIZADO);
    let vacia = e.columna("ana", 1, sitio);
    assert_eq!(e.rechazada("ana", Comando::EntregarMateriales { ejercito: vacia, pueblo: id }), rechazos::PUEBLO_NADA_QUE_ENTREGAR);

    // Una entrega parcial no arranca nada: se anota lo entregado y el carro de la columna queda vacío.
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    assert_eq!(e.pueblo(id).falta(), vec![(Madera, 40_000), (Oro, 12_000)]);
    assert!(e.n.suministro(col).is_empty());
    // Se completa con una segunda columna; lo que sobra se queda en el carro.
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 50_000 });
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 12_000 });
    let t = e.n.ahora();
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    assert_eq!(e.n.suministro(col)[&Madera], 10_000);
    assert_eq!(e.pueblo(id).estado, EstadoPueblo::EnObra { completa_en: t + 6 * HORA }, "la obra de una mina de cobre dura 6 h");
    assert_eq!(e.rechazada("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id }), rechazos::PUEBLO_NO_ESTA_PENDIENTE);
    assert_eq!((hechos(&e.n, "pueblo.materiales_entregados").len(), hechos(&e.n, "pueblo.obra_iniciada").len()), (2, 1));
    // Mientras se levanta no extrae ni tiene carro; acaba en el primer paso de minuto de la plaza que alcanza su hora (precisión de un minuto).
    e.n.avanzar_hasta(t + 6 * HORA - 1);
    assert!(!e.pueblo(id).activo());
    e.n.avanzar_hasta(t + 6 * HORA + MIN);
    let fin = hechos(&e.n, "pueblo.completado");
    assert_eq!(fin.len(), 1);
    assert!(fin[0].0 >= t + 6 * HORA && fin[0].0 < t + 6 * HORA + MIN, "acaba dentro del minuto de su hora");
    // Nace su carro de acarreo, parado en el pueblo, de la flota de la madre pero sin contar contra sus cupos.
    let carro = e.pueblo(id).carro.expect("nace con su carro");
    let c = e.n.caravana(carro).unwrap();
    assert_eq!((c.tipo, c.estado, c.origen as u64, c.posicion), (TipoCaravana::Acarreo, EstadoCaravana::Disponible, e.plaza, sitio));
    let flota = &e.n.estado().comercio.flota;
    assert_eq!((comercio::caravana::caravanas_comerciales_de(flota, e.plaza as u32), comercio::caravana::carros_en_uso(flota, e.plaza as u32)), (0, 0), "el carro de acarreo no cuenta contra los cupos del Mercado");
}

#[test]
fn una_caravana_adjunta_tambien_lleva_materiales() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    // Un Mercado para tener caravanas; la gracia de mantenimiento de un día cubre las 9 h de obra.
    e.dar(Madera, 350_000);
    e.ok("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    e.avanzar(9 * HORA);
    e.dar(Madera, 350_000);
    e.dar(Oro, 100_000);
    let car = e.ok("ana", Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
    e.ok("ana", Comando::AnadirCarro { caravana: car, tipo: comercio::caravana::TipoCarro::Basico });
    e.ok("ana", Comando::AnadirAnimal { caravana: car, carro: 0, animal: comercio::caravana::Animal::Buey });
    e.ok("ana", Comando::ReservarCaravana { caravana: car, reservada: true });
    // La columna la engancha, entra en la plaza y la deja aparcada para cargarla del almacén; luego la vuelve a enganchar.
    let pos = e.n.estado().asentamientos[&plaza].posicion;
    let col = e.columna("ana", 1, Punto::new(pos.x + 1_000, pos.y));
    e.ok("ana", Comando::AdjuntarCaravana { caravana: car, ejercito: col });
    e.ok("ana", Comando::AparcarCaravana { caravana: car, asentamiento: plaza });
    e.ok("ana", Comando::MoverCargaAparcada { caravana: car, recurso: Madera, cantidad: 100_000, cargar: true });
    e.ok("ana", Comando::MoverCargaAparcada { caravana: car, recurso: Oro, cantidad: 12_000, cargar: true });
    e.ok("ana", Comando::AdjuntarCaravana { caravana: car, ejercito: col });
    // Marca el pueblo; la columna marcha a su puerta y entrega lo que lleva en la caravana (su carro está vacío).
    let id = e.marcar();
    let sitio = e.n.posicion_de_pueblo(id).unwrap();
    e.ok("ana", Comando::MarcharA { ejercito: col, destino: sitio });
    let fin = e.n.estado().ejercitos[&col].plan.fin().expect("marcha");
    e.n.avanzar_hasta(fin);
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    assert!(matches!(e.pueblo(id).estado, EstadoPueblo::EnObra { .. }), "la caravana adjunta puso los 100 de madera y los 12 de oro");
    assert!(e.n.caravana(car).unwrap().contenido.is_empty());
}

// ───────────────────────────── 3. Extraer, llenar y acarrear ─────────────────────────────

#[test]
fn el_pueblo_extrae_contra_su_almacen_y_el_carro_sale_lleno_vuelca_en_la_madre_y_vuelve_vacio() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    let id = e.pueblo_levantado();
    let activo_desde = hechos(&e.n, "pueblo.completado")[0].0;
    let carro = e.pueblo(id).carro.unwrap();
    // Extrae contra el almacén local (no el de la plaza): 5 por minuto, también el minuto en que se completó.
    e.n.avanzar_hasta(activo_desde + 10 * MIN);
    assert_eq!(e.pueblo(id).almacen.cantidad(Cobre), 55_000);
    assert_eq!(e.almacen(Cobre), 0, "no entra en el almacén de la madre hasta que lo traiga el carro");
    // Hasta llenarse: el nodo se agota y espera a regenerarse sin que el pueblo se mude.
    let mut t = activo_desde + 10 * MIN;
    while hechos(&e.n, "acarreo.vuelve").is_empty() {
        t += 10 * MIN;
        assert!(t < activo_desde + 12 * HORA, "el carro debería haber vuelto");
        e.n.avanzar_hasta(t);
        assert!(e.pueblo(id).almacen.cantidad(Cobre) <= 500_000);
    }
    assert_eq!(e.pueblo(id).nodo, cobre);
    let sale = hechos(&e.n, "acarreo.sale");
    let (sale_en, carga) = (sale[0].0, dato(&sale[0], "carga"));
    assert_eq!(carga, 500_000, "solo sale con el carro lleno");
    assert_eq!(hechos(&e.n, "pueblo.almacen_lleno")[0].0, sale_en, "sale en el mismo minuto en que se llena");
    // El viaje es el de una caravana: plan por el mapa real a 24 u/min (el buey) hasta la plaza madre.
    let llega = hechos(&e.n, "acarreo.llega");
    let (llega_en, ingresado, perdido) = (llega[0].0, dato(&llega[0], "carga"), dato(&llega[0], "perdido"));
    let sitio = e.n.posicion_de_pueblo(id).unwrap();
    let camino = m.1.ruta(sitio, e.n.estado().asentamientos[&plaza].posicion, &rutas::Opciones::default()).unwrap();
    let esperado = (camino.coste_total() as i128 * m.1.espaciado() as i128 * 60 / 24_000) as i64;
    assert!((llega_en - sale_en - esperado).abs() <= MIN, "tarda lo que su ruta a 24 u/min: {} frente a {}", llega_en - sale_en, esperado);
    // El almacén de la madre admite 400 de cada recurso: lo que no cabe se pierde.
    assert_eq!((ingresado, perdido), (400_000, 100_000));
    // La ventana de la solvencia guarda lo que cupo, no lo que traía el carro, en la hora de la llegada.
    let eco = e.n.economia(plaza).unwrap();
    assert_eq!(eco.ingresos_acarreo, vec![IngresoAcarreo { recurso: Cobre, hora: llega_en / HORA, cantidad: 400_000 }]);
    let vuelve = hechos(&e.n, "acarreo.vuelve");
    assert_eq!((dato(&vuelve[0], "caravana"), vuelve[0].0 - llega_en), (carro as i64, llega_en - sale_en), "vuelve vacío por la misma ruta, en el mismo tiempo");
    let c = e.n.caravana(carro).unwrap();
    assert_eq!((c.tipo, c.estado, c.posicion), (TipoCaravana::Acarreo, EstadoCaravana::Disponible, sitio));
    assert!(e.n.caravana(carro).unwrap().contenido.is_empty());
    // Con el carro en casa y el almacén vacío, todo vuelve a empezar; mientras tanto, el almacén de la madre guarda lo que entró.
    assert!(e.almacen(Cobre) >= 400_000 || hechos(&e.n, "acarreo.sale").len() > 1);
}

#[test]
fn con_el_almacen_de_la_madre_lleno_el_acarreo_no_ingresa_nada_ni_cuenta_para_la_solvencia() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    e.dar(Cobre, 400_000);
    assert_eq!(e.almacen(Cobre), 400_000, "el almacén de la madre admite 400 de cada recurso y queda lleno");
    let id = e.pueblo_levantado();
    let activo_desde = hechos(&e.n, "pueblo.completado")[0].0;
    let mut t = activo_desde;
    while hechos(&e.n, "acarreo.llega").is_empty() {
        t += 10 * MIN;
        assert!(t < activo_desde + 12 * HORA, "el carro debería haber llegado");
        e.n.avanzar_hasta(t);
    }
    let llega = &hechos(&e.n, "acarreo.llega")[0];
    assert_eq!((dato(llega, "carga"), dato(llega, "perdido")), (0, 500_000), "todo se pierde contra un almacén lleno");
    assert!(e.n.economia(plaza).unwrap().ingresos_acarreo.is_empty(), "y no cuenta como ingreso: la plaza no se hace solvente con un pueblo que se pierde");
    assert!(e.pueblo(id).carro.is_some());
}

#[test]
fn con_el_nodo_agotado_el_pueblo_espera_a_que_se_regenere_y_no_se_muda() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let cobre = e.cobre;
    let id = e.pueblo_levantado();
    let desde = hechos(&e.n, "pueblo.completado")[0].0;
    let inicial = e.mapa.nodos[cobre as usize].inicial;
    // Hasta agotar el nodo extrae 5 por minuto; el stock del nodo es lo que hay (101-299 unidades): se agota antes de llenar el almacén.
    let minutos_en_agotar = inicial / 5_000;
    e.n.avanzar_hasta(desde + minutos_en_agotar * MIN + MIN);
    assert_eq!(e.pueblo(id).almacen.cantidad(Cobre), inicial, "sacó todo el nodo");
    assert_eq!(e.n.estado().yacimientos[&cobre].stock, 0);
    // Espera los 10 minutos del cooldown (el nodo sigue siendo el suyo)...
    e.n.avanzar_hasta(desde + minutos_en_agotar * MIN + 5 * MIN);
    assert_eq!(e.pueblo(id).almacen.cantidad(Cobre), inicial, "agotado, espera");
    // ...y vuelve a extraer cuando se regenera.
    e.n.avanzar_hasta(desde + minutos_en_agotar * MIN + 12 * MIN);
    assert!(e.pueblo(id).almacen.cantidad(Cobre) > inicial, "regeneró y siguió");
    assert_eq!(e.pueblo(id).nodo, cobre);
}

#[test]
fn sin_carro_el_pueblo_acumula_hasta_llenar_el_almacen_y_para_hasta_que_se_repone() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    let id = e.pueblo_levantado();
    let carro = e.pueblo(id).carro.unwrap();
    // Un ataque destruye el carro (el hook del sistema militar) con el pueblo recién levantado.
    let t = e.n.ahora();
    assert!(e.n.aplicar_perdida_de_carros(t, carro, &[0]).unwrap().destruida);
    assert!(e.n.caravana(carro).is_none() && e.pueblo(id).carro.is_none());
    // Sin carro no sale nada: acumula, se llena una sola vez y la extracción para (el nodo no se gasta de más).
    e.avanzar(8 * HORA);
    assert_eq!(e.pueblo(id).almacen.cantidad(Cobre), 500_000);
    assert!(e.pueblo(id).lleno);
    assert_eq!((hechos(&e.n, "pueblo.almacen_lleno").len(), hechos(&e.n, "acarreo.sale").len()), (1, 0));
    let stock_nodo = e.n.estado().yacimientos.get(&cobre).map(|s| s.stock);
    e.avanzar(30 * MIN);
    assert_eq!(e.n.estado().yacimientos.get(&cobre).map(|s| s.stock), stock_nodo, "lleno, no gasta el yacimiento");

    // Reponer cuesta el carro y el buey (20 de madera y 12 de oro) de la madre, solo si el pueblo no tiene ninguno y solo a quien manda.
    assert_eq!(e.rechazada("beto", Comando::ReponerCarro { pueblo: id }), rechazos::NO_AUTORIZADO);
    let (madera, oro) = (e.almacen(Madera), e.almacen(Oro));
    e.ok("ana", Comando::ReponerCarro { pueblo: id });
    assert_eq!((madera - e.almacen(Madera), oro - e.almacen(Oro)), (20_000, 12_000));
    let nuevo = e.pueblo(id).carro.expect("tiene carro otra vez");
    assert_ne!(nuevo, carro);
    assert_eq!(e.rechazada("ana", Comando::ReponerCarro { pueblo: id }), rechazos::PUEBLO_YA_TIENE_CARRO);
    // El almacén ya estaba lleno: el carro nuevo sale al instante con él.
    assert_eq!(e.n.caravana(nuevo).unwrap().estado, EstadoCaravana::EnTransito);
    assert_eq!((e.pueblo(id).almacen.cantidad(Cobre), e.n.caravana(nuevo).unwrap().contenido[&Cobre]), (0, 500_000));
    // Un pueblo aún sin levantar no tiene carro que reponer.
    let otro = e.n.candidatos_de_pueblo(plaza).into_iter().find(|c| c.1 == Tipo::Cantera).unwrap().0;
    let pendiente = e.ok("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: otro }).creadas[0];
    assert_eq!(e.rechazada("ana", Comando::ReponerCarro { pueblo: pendiente }), rechazos::PUEBLO_NO_ESTA_ACTIVO);
}

#[test]
fn el_pueblo_paga_mantenimiento_a_la_madre_y_no_deja_eventos_recurrentes_por_pueblo_ocioso() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, datos) = (e.plaza, Reglas::default().economia.datos);
    let sin_pueblos = e.n.economia(plaza).unwrap().coste_mantenimiento(&datos, 0);
    let id = e.marcar();
    assert_eq!(e.n.economia(plaza).unwrap().coste_mantenimiento(&datos, 0), sin_pueblos, "pendiente o en obra, aún no cuenta");
    let col = e.llevar_materiales(id);
    let en_cola_base = e.n.eventos_en_cola(); // el paso de minuto de la plaza y los umbrales de víveres de la columna
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    let EstadoPueblo::EnObra { completa_en } = e.pueblo(id).estado else { panic!("en obra") };
    e.n.avanzar_hasta(completa_en + MIN);
    assert!(e.n.economia(plaza).unwrap().coste_mantenimiento(&datos, 0)[0].1 > sin_pueblos[0].1, "activo, cuenta como un edificio más de la madre");
    // Sin carro y con el almacén lleno el pueblo no programa nada: solo quedan el paso de minuto de la plaza y lo que ya había.
    let carro = e.pueblo(id).carro.unwrap();
    let t = e.n.ahora();
    e.n.aplicar_perdida_de_carros(t, carro, &[0]).unwrap();
    e.avanzar(8 * HORA);
    assert!(e.pueblo(id).lleno);
    assert_eq!(e.n.eventos_en_cola(), en_cola_base, "ni el pueblo ocioso ni el carro ausente tienen eventos propios");
}

#[test]
fn la_planta_del_pueblo_es_una_funcion_de_su_id_y_no_se_guarda() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let id = e.pueblo_levantado();
    let hash = e.n.hash_estado();
    let planta = e.n.planta_de_pueblo(id).expect("la planta del pueblo");
    assert_eq!(planta, e.n.planta_de_pueblo(id).unwrap());
    assert_eq!(planta.piezas[0].tipo, Tipo::MinaCobre, "la mina es el ancla");
    assert!(planta.piezas.iter().any(|p| p.tipo == Tipo::Almacen) && planta.piezas.iter().any(|p| p.tipo == Tipo::Corral));
    assert_eq!(e.n.hash_estado(), hash, "es presentación: no toca el estado");
    assert!(e.n.planta_de_pueblo(u64::MAX).is_none());
    // La guarnición de serie la pondrá el sistema militar; su dato (30 mineros, 6 h) está en las reglas.
    let p = Reglas::default().economia.datos.pueblos.clone();
    assert_eq!((p.mineros, p.reposicion_mineros_min), (30, 360));
}

// ───────────────────────────── 4. Destruirlo, la ruina y la conquista de la madre ─────────────────────────────

/// Avanza de minuto en minuto hasta que el carro del pueblo esté en camino a la madre.
fn hasta_que_el_carro_vaya_cargado(e: &mut Escenario, pueblo: u64) -> u64 {
    let carro = e.pueblo(pueblo).carro.unwrap();
    let limite = e.n.ahora() + 12 * HORA;
    while e.n.caravana(carro).unwrap().estado != EstadoCaravana::EnTransito {
        e.avanzar(MIN);
        assert!(e.n.ahora() < limite, "el carro debería haber salido");
    }
    carro
}

#[test]
fn destruir_a_mano_un_pueblo_con_el_carro_en_camino_pierde_el_carro_su_carga_y_libera_el_nodo() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    let id = e.pueblo_levantado();
    let carro = hasta_que_el_carro_vaya_cargado(&mut e, id);
    let cobre_madre = e.almacen(Cobre);
    e.ok("ana", Comando::DestruirPueblo { pueblo: id });
    assert!(e.n.pueblo(id).is_none() && e.n.caravana(carro).is_none(), "se pierde el pueblo y su carro con lo que llevaba");
    assert!(!e.n.estado().nodos_reclamados.contains_key(&cobre), "el nodo vuelve a estar libre");
    e.avanzar(3 * HORA);
    assert_eq!(e.almacen(Cobre), cobre_madre, "lo que iba en el carro no llegó nunca");
    assert!(hechos(&e.n, "acarreo.llega").is_empty());
    assert!(e.n.candidatos_de_pueblo(plaza).iter().any(|&(n, _)| n == cobre), "y vuelve a ser candidato");
}

#[test]
fn la_ruina_de_la_madre_se_lleva_sus_pueblos_y_sus_carros_y_libera_los_nodos() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    let id = e.pueblo_levantado();
    let carro = e.pueblo(id).carro.unwrap();
    // Sin auto-construcción ni madera que repongan el mantenimiento, la plaza acaba cayendo en ruinas.
    for dia in 1..=150 {
        let t = e.n.ahora().max(dia * 24 * HORA);
        e.n.avanzar_hasta(t);
        if e.n.economia(plaza).is_none() {
            break;
        }
    }
    assert!(e.n.economia(plaza).is_none(), "la plaza cayó en ruinas");
    assert!(e.n.pueblo(id).is_none() && e.n.pueblos_de(plaza).is_empty());
    assert!(e.n.caravana(carro).is_none(), "su carro, con la madre por origen, se pierde en la cascada de comercio");
    assert!(hechos(&e.n, "comercio.caravana_perdida").iter().any(|h| dato(h, "caravana") == carro as i64));
    assert!(!e.n.estado().nodos_reclamados.contains_key(&cobre), "el nodo queda libre");
    assert!(e.n.estado().comercio.flota.iter().all(|c| c.tipo != TipoCaravana::Acarreo));
}

#[test]
fn la_conquista_de_la_madre_deja_los_pueblos_con_la_plaza_y_el_carro_en_camino_sigue() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let id = e.pueblo_levantado();
    let carro = hasta_que_el_carro_vaya_cargado(&mut e, id);
    // La conquista de verdad: el ejército de Beto (F2) asedia la madre, que no tiene quien la defienda, y cae sin combate.
    let p = e.n.estado().asentamientos[&plaza].posicion;
    let beto = e.columna("beto", 2, Punto::new(p.x + 2_000, p.y));
    let heroe = e.n.ejercito(beto).unwrap().columna.lider;
    let arqueros = militar::catalogo::Tropa::ArquerosCompuesto;
    e.ok("admin", Comando::DarEscuadra { heroe, tropa: arqueros, cantidad: militar::Reglas::cargar().tropas.tropa(arqueros).unidades });
    e.ok("beto", Comando::Atacar { heroe, objetivo: nucleo::ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2);
    // Los pueblos son de la plaza: pasan con ella (y su zona) al conquistador, y la flota traspasada, también el carro de acarreo.
    assert!(e.n.pueblo(id).is_some_and(|(madre, p)| madre == plaza && p.carro == Some(carro)));
    assert_eq!(e.n.caravana(carro).unwrap().estado, EstadoCaravana::EnTransito);
    assert_eq!(hechos(&e.n, "comercio.flota_traspasada").len(), 1);
    e.avanzar(HORA);
    assert!(!hechos(&e.n, "acarreo.llega").is_empty(), "el carro entrega en la plaza ya conquistada");
}

// ───────────────────────────── 5. Determinismo y recuperación ─────────────────────────────

/// Un guion con pueblos: marca, entrega, se levanta, pierde el carro (el primero de la partida), lo repone y marca otro pueblo. Las órdenes
/// llevan su instante: `avanzar` solo decide cómo se mueve el reloj entre ellas.
fn guion(e: &mut Escenario, avanzar: &dyn Fn(&mut Escenario, Instante)) {
    let plaza = e.plaza;
    let id = e.marcar();
    let otro = e.n.candidatos_de_pueblo(plaza)[0].0;
    let col = e.llevar_materiales(id);
    avanzar(e, 20 * MIN);
    e.ok_en(20 * MIN, "ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    avanzar(e, 8 * HORA);
    e.n.aplicar_perdida_de_carros(8 * HORA, 1, &[0]).expect("el carro del pueblo");
    avanzar(e, 12 * HORA);
    e.ok_en(12 * HORA, "ana", Comando::ReponerCarro { pueblo: id });
    e.ok_en(12 * HORA, "ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: otro });
    avanzar(e, 20 * HORA);
}

fn correr(m: &M, semilla: u64, avanzar: &dyn Fn(&mut Escenario, Instante)) -> Nucleo {
    let mut e = montar(m, semilla, 2);
    guion(&mut e, avanzar);
    e.n.avanzar_hasta(20 * HORA);
    e.n
}

#[test]
fn mismas_ordenes_dan_el_mismo_hash_normal_acelerado_y_a_trozos() {
    let m = con_mundo!();
    let normal = correr(m, 3, &|e, t| e.n.avanzar_hasta(t));
    assert_eq!(normal.hash_estado(), correr(m, 3, &|e, t| e.n.avanzar_hasta(t)).hash_estado(), "repetible");
    assert_eq!(normal.hash_estado(), correr(m, 3, &|_, _| {}).hash_estado(), "acelerado de un golpe");
    let a_trozos = correr(m, 3, &|e, t| {
        let ahora = e.n.ahora();
        for s in 1..=5 {
            e.n.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 25);
        }
    });
    assert_eq!(normal.hash_estado(), a_trozos.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    assert_ne!(normal.hash_estado(), correr(m, 4, &|e, t| e.n.avanzar_hasta(t)).hash_estado(), "otra semilla, otro mundo");
    // El guion hizo de verdad lo que se espera.
    assert!(!hechos(&normal, "acarreo.llega").is_empty() && hechos(&normal, "pueblo.completado").len() == 1 && hechos(&normal, "acarreo.carro_repuesto").len() == 1);
}

#[test]
fn una_foto_a_mitad_de_obra_y_otra_con_el_carro_en_camino_recuperan_el_mismo_mundo() {
    let m = con_mundo!();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    // Dos cortes: con la obra a medias (3 h de 6) y con el carro cargado y en camino.
    for (nombre, carro_en_camino) in [("a mitad de obra", false), ("con el carro en camino", true)] {
        let dir = tempfile::tempdir().unwrap();
        // Sin parar: la referencia.
        let referencia = {
            let mut e = montar(m, 9, 2);
            let id = e.marcar();
            let col = e.llevar_materiales(id);
            e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
            e.avanzar(10 * HORA);
            e.avanzar(10 * HORA);
            (e.n.hash_estado(), e.n.estado().comercio.clone())
        };
        let (a_mitad, cuenta, plaza, id, ahora) = {
            let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
            let mut e = montar_con(n, m, 2);
            let id = e.marcar();
            let col = e.llevar_materiales(id);
            e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
            if carro_en_camino {
                e.avanzar(6 * HORA + MIN);
                let carro = hasta_que_el_carro_vaya_cargado(&mut e, id);
                assert_eq!(e.n.caravana(carro).unwrap().estado, EstadoCaravana::EnTransito, "la foto pilla el viaje en mitad");
            } else {
                e.avanzar(3 * HORA);
                assert!(matches!(e.pueblo(id).estado, EstadoPueblo::EnObra { .. }), "la foto pilla la obra en mitad");
            }
            e.n.foto_ahora();
            e.avanzar(10 * MIN);
            e.n.apagar().unwrap();
            (e.n.hash_estado(), e.cuenta, e.plaza, id, e.n.ahora())
        };
        let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        assert!(n.recuperado());
        assert_eq!(n.hash_estado(), a_mitad, "{nombre}: foto + registro reconstruyen el mismo mundo");
        let cobre = n.pueblo(id).unwrap().1.nodo;
        let mut e = Escenario { n, plaza, cobre, mapa: Mapa::de(&m.0), cuenta };
        e.avanzar(20 * HORA - ahora);
        assert_eq!(e.n.hash_estado(), referencia.0, "{nombre}: el mundo recuperado sigue igual que el que no se interrumpió");
        assert_eq!(e.n.estado().comercio, referencia.1);
        e.n.apagar().unwrap();
    }
}
