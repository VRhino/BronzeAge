//! Los Aedas residentes y sus épicas en el núcleo (sistema 6, tanda 6E; Tecnologia_Aedas §7). Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`; las pruebas se omiten si
//! no está). Portan los casos de `typescript-final:src/engine/__tests__/aedasResidentes.test.ts` que miran el núcleo (la llegada tras el plazo y el cupo, la reputación baja, Mecenas de los Aedas)
//! y la parte de épicas de `session/__tests__/comandosTecnologia.test.ts` (empezar, abandonar, quién las dirige, un hecho de la partida que cierra el último capítulo y la crónica que lo canta sin
//! nombrar la tecnología); las reglas puras están en el crate `tecnologia` (6A).
//!
//! El escenario: una plaza de nivel 2 con la auto-construcción parada, que levanta su Almacén y su Palacio por orden (como `tests/politica.rs`); la nobleza pide un solo ciudadano (el fundador)
//! para no esperar a tres residentes. La cantería tiene el logro de `plazasEnNivel.2 ≥ 1` (lo cumple `FijarNivelAsentamiento`) y, para la prueba, una épica de dos capítulos de obra.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Edificios;
use base::{Instante, MS_POR_MINUTO};
use comun::real::{self, M};
use nucleo::{entidad_residentes, entidad_tecnologia, rechazos, Ajustes, Arranque, Carga, Comando, Dato, Estado, EventoNucleo, Nucleo, Peticion, Reglas, Resultado};
use tecnologia::datos::{Capitulo, DefinicionEpica, HechoEpico, Logro};
use tecnologia::{Contador, TecId};

const MIN: Instante = MS_POR_MINUTO;
const HORA: Instante = 60 * MIN;
const DIA: Instante = 24 * HORA;

macro_rules! con_mundo {
    () => {
        match real::mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

/// Sin bandidos, almacenes grandes, la nobleza con un ciudadano, la cantería (logro de nivel 2, épica de dos obras: un Almacén y otra cualquiera) y el bronce de calidad militar (Era II) con el
/// mismo logro; y los cambios de `mas`.
fn reglas(mas: impl FnOnce(&mut tecnologia::Reglas)) -> Reglas {
    let mut r = comun::reglas();
    let b = &mut std::sync::Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let d = std::sync::Arc::make_mut(&mut r.economia.datos);
    d.capacidad_inicial = 100_000_000;
    d.poblacion.min_ciudadanos_nobleza = 1;
    let t = std::sync::Arc::make_mut(&mut r.economia.tecnologia);
    for id in ["canteria", "bronce_calidad_militar", "metalurgia_cobre"] {
        let x = t.buscar(id).unwrap();
        t.catalogo[x as usize].logro = Some(Logro { contador: Contador::PlazasEnNivel2, umbral: 1 });
    }
    let c = t.buscar("canteria").unwrap();
    t.catalogo[c as usize].epica = Some(DefinicionEpica {
        nombre: "Canto de la piedra".into(),
        capitulos: vec![Capitulo { hecho: HechoEpico::Obra, cantidad: 1, edificio: Some(Tipo::Almacen) }, Capitulo { hecho: HechoEpico::Obra, cantidad: 1, edificio: None }],
    });
    mas(t);
    t.indexar();
    r
}

fn arranque(m: &M, reglas: Reglas) -> Arranque {
    Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

struct Escenario {
    n: Nucleo,
    plaza: u64,
    /// El Rey de la Facción 1 (el héroe de «ana», que la fundó).
    rey: u64,
    k: u32,
    /// Lo que da `llenar` de cada recurso (milésimas).
    cantidad: i64,
}

impl Escenario {
    fn ord(&mut self, actor: &str, c: Comando) -> nucleo::Respuesta {
        self.k += 1;
        let id = format!("o{}", self.k);
        let p = if actor == "admin" { Peticion::nueva("admin", &id, c).de_admin() } else { Peticion::nueva(actor, &id, c) };
        self.n.aplicar_en(self.n.ahora(), p)
    }

    fn ok(&mut self, actor: &str, c: Comando) {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
    }

    fn rechazo(&mut self, actor: &str, c: Comando) -> String {
        // Lo que ya esperaba en este instante, antes (la orden lo procesaría primero).
        let t = self.n.ahora();
        self.n.avanzar_hasta(t);
        let (v, h) = (self.n.version(), self.n.hash_estado());
        match self.ord(actor, c).resultado {
            Resultado::Rechazada { codigo, .. } => {
                assert_eq!((self.n.version(), self.n.hash_estado()), (v, h), "un rechazo no deja rastro");
                codigo.to_string()
            }
            otro => panic!("se esperaba un rechazo: {otro:?}"),
        }
    }

    fn tec(&self, id: &str) -> TecId {
        self.n.estado().reglas.economia.tecnologia.buscar(id).unwrap()
    }

    fn avanzar(&mut self, d: Instante) {
        let t = self.n.ahora() + d;
        self.n.avanzar_hasta(t);
    }

    /// Avanza de minuto en minuto hasta que se cumple `cond` (como mucho `tope` más).
    fn hasta(&mut self, tope: Instante, que: &str, cond: impl Fn(&Nucleo) -> bool) {
        let fin = self.n.ahora() + tope;
        while !cond(&self.n) {
            assert!(self.n.ahora() < fin, "no llegó a tiempo: {que}");
            assert!(self.n.economia(self.plaza).is_some(), "la plaza cayó: {:?}", hechos(&self.n, "asentamiento.ruinas").iter().map(|h| (h.en, h.datos.clone())).collect::<Vec<_>>());
            self.avanzar(MIN);
        }
    }

    /// De todo, para semanas: la plaza con la auto-construcción parada no se sostiene sola (su mantenimiento la arruinaría).
    fn llenar(&mut self) {
        for r in asentamiento::recurso::TODOS {
            let p = self.plaza;
            let cantidad = self.cantidad;
            self.ord("admin", Comando::DarRecursos { asentamiento: p, recurso: r, cantidad });
        }
    }

    fn anadir(&mut self, tipo: Tipo) {
        self.llenar();
        let p = self.plaza;
        self.ok("admin", Comando::AnadirEdificio { asentamiento: p, tipo });
    }

    fn activos(&self, tipo: Tipo) -> usize {
        self.n.economia(self.plaza).unwrap().edificios.activos(tipo)
    }

    fn nobleza(&self) -> i64 {
        self.n.economia(self.plaza).unwrap().poblacion.nobleza
    }

    /// Las citas de llegada **vigentes** de la plaza en la cola.
    fn citas(&self) -> Vec<Instante> {
        let e = self.n.estado();
        let entidad = entidad_residentes(self.plaza);
        e.cola.volcar().into_iter().filter(|x| x.clave.entidad == entidad && e.generaciones.es_vigente(x.clave)).map(|x| x.instante).collect()
    }

    fn empezar(&self, aeda: u32, id: &str) -> Comando {
        Comando::EmpezarEpica { heroe: self.rey, asentamiento: self.plaza, aeda, tecnologia: self.tec(id) }
    }

    fn abandonar(&self, aeda: u32) -> Comando {
        Comando::AbandonarEpica { heroe: self.rey, asentamiento: self.plaza, aeda }
    }
}

/// La plaza de la Facción 1 de nivel 2 con su Palacio en pie y aún sin nobleza; el Rey dentro.
fn con_palacio(m: &M, r: Reglas) -> Escenario {
    con_palacio_y(m, r, 20_000_000)
}

/// Lo mismo, dando `cantidad` de cada recurso en cada obra.
fn con_palacio_y(m: &M, r: Reglas, cantidad: i64) -> Escenario {
    con_palacio_en(m, Nucleo::nuevo(arranque(m, r)), cantidad)
}

/// Lo mismo, sobre un núcleo dado (en memoria o persistente).
fn con_palacio_en(m: &M, mut n: Nucleo, cantidad: i64) -> Escenario {
    let p = real::sitio_rico(&n, &m.0, 0, 1);
    let plaza = real::fundar(&mut n, "ana", 1, p);
    let rey = n.heroes().find(|h| h.cuenta == "ana").unwrap().heroe.id;
    let mut e = Escenario { n, plaza, rey, k: 0, cantidad };
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 2 });
    e.anadir(Tipo::Almacen);
    e.hasta(DIA, "el Almacén", |n| n.economia(plaza).is_none_or(|x| x.edificios.activos(Tipo::Almacen) > 0));
    // Sin Palacio no hay cupo: nada espera, nada en la cola.
    assert!(e.citas().is_empty() && e.n.estado().residentes.espera_desde.is_empty() && e.nobleza() == 0, "sin Palacio, sin cupo");
    e.anadir(Tipo::Palacio);
    e.hasta(5 * DIA, "el Palacio", |n| n.economia(plaza).is_none_or(|x| x.edificios.activos(Tipo::Palacio) > 0));
    if e.n.economia(plaza).is_none() {
        let ultimos: Vec<String> = e.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).map(|h| format!("{} {}", h.en, h.codigo)).collect();
        panic!("la plaza cayó: {:?}", &ultimos[ultimos.len().saturating_sub(15)..]);
    }
    // El Rey entra en la plaza con su ejército fundador (está a la puerta).
    e.ok("admin", Comando::Guarnecer { asentamiento: plaza, heroe: rey });
    e
}

/// Y con su primer Aeda residente ya llegado (`aeda-r1`).
fn con_residente(m: &M, r: Reglas) -> Escenario {
    let mut e = con_palacio(m, r);
    e.hasta(2 * DIA, "el primer Aeda", |n| !n.residentes().aedas.is_empty());
    e
}

fn hechos(n: &Nucleo, codigo: &str) -> Vec<EventoNucleo> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).cloned().collect()
}

fn texto(h: &EventoNucleo, clave: &str) -> Option<String> {
    h.valores.iter().find(|(k, _)| *k == clave).and_then(|(_, v)| if let Dato::Texto(s) = v { Some(s.clone()) } else { None })
}

// ───────────────────────────── 1. El ciclo de vida ─────────────────────────────

#[test]
fn llega_uno_cuando_se_cumple_el_plazo_con_cupo_libre_y_no_mas_que_el_cupo_y_sin_cupo_no_hay_cita() {
    let m = con_mundo!();
    let mut e = con_palacio(m, reglas(|_| {}));
    // La nobleza nace en el paso de minuto que sigue al Palacio: la plaza espera desde entonces y tiene una cita a 24 h.
    e.hasta(HORA, "la espera", |n| !n.residentes().espera_desde.is_empty());
    assert!(e.nobleza() > 0);
    let desde = *e.n.estado().residentes.espera_desde.get(&(e.plaza as u32)).expect("la plaza espera");
    assert_eq!((desde - e.n.estado().asentamientos[&e.plaza].economia.fundado_en) % MIN, 0, "en un paso de minuto de la plaza");
    let llegada = e.n.estado().reglas.economia.tecnologia.aedas.llegada;
    assert_eq!(e.citas(), vec![desde + llegada], "una sola cita, a `llegadaMinutos`");
    e.n.avanzar_hasta(desde + llegada - 1);
    assert!(e.n.residentes().aedas.is_empty(), "aún no");
    e.n.avanzar_hasta(desde + llegada);
    let llega = hechos(&e.n, "aedas.residente_llega");
    assert_eq!(llega.len(), 1);
    assert_eq!((llega[0].en, llega[0].dato("aedaResidente"), llega[0].dato("faccion"), llega[0].asentamientos.clone()), (desde + llegada, Some(1), Some(1), vec![e.plaza]));
    let a = &e.n.residentes().aedas[0];
    assert_eq!((a.id, a.nombre(), a.plaza, a.faccion, a.llegada_en), (1, "Femio", e.plaza as u32, 1, desde + llegada));
    assert_eq!(e.n.residentes_en(e.plaza), 1);
    // Con el cupo lleno (1 en el nivel 2), ninguna espera y nada en la cola, aunque pasen días.
    assert!(e.citas().is_empty() && e.n.estado().residentes_citas.is_empty() && e.n.estado().residentes.espera_desde.is_empty());
    e.avanzar(3 * DIA);
    assert_eq!(e.n.residentes().aedas.len(), 1, "no más que el cupo");
    // Nivel 3: cupo 2, la espera empieza ahora y llega el segundo a las 24 h.
    let p = e.plaza;
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: p, nivel: 3 });
    let t = e.n.ahora();
    assert_eq!(e.citas(), vec![t + llegada]);
    e.n.avanzar_hasta(t + llegada);
    assert_eq!(e.n.residentes().aedas.iter().map(|a| (a.id, a.nombre())).collect::<Vec<_>>(), vec![(1, "Femio"), (2, "Demódoco")]);
    assert!(e.citas().is_empty());
    // Mecenas de los Aedas: sin épicas cumplidas, los residentes (Doc 2.9).
    assert_eq!(e.n.medidas_de_titulos().iter().find(|x| x.faccion == 1).unwrap().mecenas, 2);
}

#[test]
fn con_la_reputacion_por_los_suelos_tardan_tres_veces_mas() {
    let m = con_mundo!();
    // El umbral de la reputación baja en 0: la Facción nueva (reputación 0) está en él.
    let mut e = con_palacio(m, reglas(|t| t.aedas.umbral_reputacion_baja = 0));
    e.hasta(HORA, "la nobleza", |n| n.residentes().espera_desde.values().next().is_some());
    let desde = e.n.ahora();
    let a = e.n.estado().reglas.economia.tecnologia.aedas.clone();
    assert_eq!(e.citas(), vec![desde + a.llegada], "se mira al comparar: primero a las 24 h");
    e.n.avanzar_hasta(desde + a.llegada);
    assert!(e.n.residentes().aedas.is_empty(), "con la reputación baja, no llega a las 24 h");
    assert_eq!(e.citas(), vec![desde + a.llegada * a.factor_llegada_reputacion_baja], "sin cruce del umbral, a las 72 h");
    e.n.avanzar_hasta(desde + a.llegada * a.factor_llegada_reputacion_baja);
    assert_eq!(e.n.residentes().aedas.len(), 1);
}

// ───────────────────────────── 2. Las épicas ─────────────────────────────

#[test]
fn empezar_una_epica_con_sus_rechazos_abandonarla_y_la_que_aparece_por_otra_via_se_pierde() {
    let m = con_mundo!();
    let mut e = con_residente(m, reglas(|_| {}));
    let (plaza, canteria) = (e.plaza, e.tec("canteria"));
    // Quién la dirige (dirigeAedas): el Rey dentro, sí; un héroe de otra Facción a la puerta, no.
    let (t, puerta) = (e.n.ahora(), e.n.estado().asentamientos[&plaza].posicion);
    let col = e.n.crear_ejercito(t, "beto", 2, puerta, 500, 100_000).unwrap();
    let beto = e.n.ejercito(col).unwrap().columna.lider;
    assert_eq!(e.rechazo("beto", Comando::EmpezarEpica { heroe: beto, asentamiento: plaza, aeda: 1, tecnologia: canteria }), rechazos::NO_AUTORIZADO);
    // Los rechazos, en el orden del TypeScript (Tecnologia_Aedas §7.2).
    let rey = e.rey;
    assert_eq!(e.rechazo("ana", Comando::EmpezarEpica { heroe: rey, asentamiento: 9_999, aeda: 1, tecnologia: canteria }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert_eq!(e.rechazo("ana", Comando::EmpezarEpica { heroe: rey, asentamiento: plaza, aeda: 1, tecnologia: 200 }), rechazos::TECNOLOGIA_DESCONOCIDA);
    assert_eq!(e.rechazo("ana", e.empezar(9, "canteria")), rechazos::EPICA_NO_VIVE_AQUI);
    assert_eq!(e.rechazo("ana", e.empezar(1, "leva_comunal")), rechazos::EPICA_SIN_EPICA);
    assert_eq!(e.rechazo("ana", e.empezar(1, "aleacion_bronce")), rechazos::EPICA_SIN_LOGRO);
    assert_eq!(e.rechazo("ana", e.empezar(1, "bronce_calidad_militar")), rechazos::EPICA_ERA_CERRADA);
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["metalurgia_cobre".into()] });
    assert_eq!(e.rechazo("ana", e.empezar(1, "metalurgia_cobre")), rechazos::EPICA_YA_APARECIDA);
    assert_eq!(e.rechazo("ana", e.abandonar(1)), rechazos::EPICA_NO_CANTA);
    // Empieza: sin coste, el capítulo 0 de 2, privado de la plaza.
    e.ok("ana", e.empezar(1, "canteria"));
    let h = hechos(&e.n, "aedas.epica_empieza");
    assert_eq!(h.len(), 1);
    assert_eq!(
        (h[0].dato("aedaResidente"), h[0].dato("faccion"), h[0].dato("capitulo"), h[0].dato("capitulos"), texto(&h[0], "tecnologia").as_deref(), h[0].asentamientos.clone(), h[0].publico),
        (Some(1), Some(1), Some(0), Some(2), Some("canteria"), vec![plaza], false)
    );
    assert_eq!(e.n.residentes().aedas[0].epica, Some(tecnologia::Epica::nueva(canteria)));
    assert_eq!(e.rechazo("ana", e.empezar(1, "canteria")), rechazos::EPICA_YA_CANTA);
    // Un segundo Aeda (nivel 3, cupo 2) no puede cantar la misma.
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 3 });
    e.hasta(2 * DIA, "el segundo Aeda", |n| n.residentes().aedas.len() == 2);
    assert_eq!(e.rechazo("ana", e.empezar(2, "canteria")), rechazos::EPICA_OTRO_LA_CANTA);
    // Abandonar la borra con lo avanzado (motivo `orden`).
    e.ok("ana", e.abandonar(1));
    let ab = hechos(&e.n, "aedas.epica_abandonada");
    assert_eq!((ab.len(), texto(&ab[0], "motivo").as_deref(), texto(&ab[0], "tecnologia").as_deref()), (1, Some("orden"), Some("canteria")));
    assert!(e.n.residentes().aedas[0].epica.is_none());
    // Otra vez, y la cantería le aparece a la Facción por otra vía: la épica sobra (motivo `aparecida`, decisión técnica 11).
    e.ok("ana", e.empezar(2, "canteria"));
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["metalurgia_cobre".into(), "canteria".into()] });
    let ab = hechos(&e.n, "aedas.epica_abandonada");
    assert_eq!((ab.len(), texto(&ab[1], "motivo").as_deref(), ab[1].dato("aedaResidente"), ab[1].facciones.clone()), (2, Some("aparecida"), Some(2), vec![1]));
    assert!(e.n.residentes().aedas.iter().all(|a| a.epica.is_none()));
    assert_eq!(e.rechazo("ana", e.empezar(2, "canteria")), rechazos::EPICA_YA_APARECIDA);
}

/// La versión del único hecho de ese código.
fn version_de(n: &Nucleo, codigo: &str) -> u64 {
    let v: Vec<u64> = n.cambios_desde(0).unwrap().filter(|c| c.eventos.iter().any(|x| x.codigo == codigo)).map(|c| c.version).collect();
    assert_eq!(v.len(), 1, "{codigo}: {v:?}");
    v[0]
}

#[test]
fn las_obras_de_la_plaza_avanzan_la_epica_y_el_ultimo_capitulo_la_hace_aparecer_y_la_cronica_lo_canta_sin_nombrarla() {
    let m = con_mundo!();
    let mut e = con_residente(m, reglas(|_| {}));
    let canteria = e.tec("canteria");
    e.ok("ana", e.empezar(1, "canteria"));
    // Una obra que no es la del capítulo (pide un Almacén) no cuenta.
    let granjas = e.activos(Tipo::Granja);
    e.anadir(Tipo::Granja);
    let plaza = e.plaza;
    e.hasta(DIA, "la Granja", |n| n.economia(plaza).unwrap().edificios.activos(Tipo::Granja) > granjas);
    assert_eq!(e.n.residentes().aedas[0].epica.as_ref().map(|x| (x.capitulo, x.hechos)), Some((0, 0)), "la Granja no es la obra del capítulo");
    // El Almacén cierra el primer capítulo (`aedas.epica_capitulo`).
    let antes = e.activos(Tipo::Almacen);
    e.anadir(Tipo::Almacen);
    e.hasta(DIA, "el Almacén", |n| n.economia(plaza).unwrap().edificios.activos(Tipo::Almacen) > antes);
    let cap = hechos(&e.n, "aedas.epica_capitulo");
    assert_eq!(cap.len(), 1);
    assert_eq!((cap[0].dato("capitulo"), cap[0].dato("capitulos"), texto(&cap[0], "tecnologia").as_deref()), (Some(1), Some(2), Some("canteria")));
    let ep = e.n.residentes().aedas[0].epica.clone().unwrap();
    assert_eq!((ep.capitulo, ep.hechos, ep.claves.len()), (1, 0, 1));
    ida_y_vuelta(&e);
    // Tras el enfriamiento (6 h), otra obra cierra la épica.
    let enfriamiento = e.n.estado().reglas.economia.tecnologia.aedas.enfriamiento_epica;
    e.n.avanzar_hasta(ep.ultimo_hecho_en.unwrap() + enfriamiento);
    let r = e.n.estado().reglas.economia.tecnologia.clone();
    assert!(!e.n.tecnologia().de(&r, 1).aparecidas.contiene(canteria));
    e.anadir(Tipo::Granja);
    e.hasta(DIA, "la épica", |n| n.residentes().cumplidas_de(1) == 1);
    let f = e.n.tecnologia().de(&r, 1);
    assert!(f.aparecidas.contiene(canteria) && !f.adoptadas.contiene(canteria), "aparece sin hito, sin adoptar");
    assert!(e.n.residentes().aedas[0].epica.is_none());
    assert_eq!(e.n.tecnologia().primeros[canteria as usize].expect("la descubridora").faccion, 1, "un descubrimiento lo es sea cual sea la vía");
    let citas = e.n.estado().cola.volcar().into_iter().filter(|x| x.clave.entidad == entidad_tecnologia(canteria)).count();
    assert_eq!(citas, 2, "las dos citas del descubrimiento");
    let tec = hechos(&e.n, "aedas.epica_tecnologia");
    assert_eq!((tec.len(), texto(&tec[0], "tecnologia").as_deref(), tec[0].publico, tec[0].asentamientos.clone()), (1, Some("canteria"), false, vec![plaza]));
    let canto = hechos(&e.n, "aedas.epica_cumplida");
    assert_eq!(canto.len(), 1);
    assert!(canto[0].publico && texto(&canto[0], "tecnologia").is_none() && canto[0].asentamientos.is_empty(), "pública y sin nombrar la tecnología");
    let completada = e.n.cambios_desde(0).unwrap().filter(|c| c.eventos.iter().any(|h| h.codigo == "construccion.edificio_completado")).map(|c| c.version).max().unwrap();
    assert_eq!(version_de(&e.n, "aedas.epica_cumplida"), completada, "en la versión de la obra que la cierra");
    let cronica: Vec<EventoNucleo> = hechos(&e.n, "cronica.entrada").into_iter().filter(|h| texto(h, "tipo").as_deref() == Some("aedas.epica_cumplida")).collect();
    assert_eq!(cronica.len(), 1, "y entra en la crónica");
    assert!(texto(&cronica[0], "tecnologia").is_none());
    // Mecenas de los Aedas: una épica cumplida (× 10) y un residente.
    assert_eq!(e.n.medidas_de_titulos().iter().find(|x| x.faccion == 1).unwrap().mecenas, 11);
    // La anexión (decisión técnica 13): la Facción 2 absorbe a la 1; su residente pasa con la plaza y su épica cumplida se suma a la de la que recoge.
    let (t, p) = (e.n.ahora(), e.n.estado().asentamientos[&plaza].posicion);
    let col = e.n.crear_ejercito(t, "c2", 2, base::Punto::new(p.x + 40_000, p.y), 500, 350_000).unwrap();
    let r2 = e.n.ejercito(col).unwrap().columna.lider;
    let propuesta = e.ord("c2", Comando::ProponerAnexion { heroe: r2, absorbente: 2, absorbida: 1 });
    assert!(propuesta.aceptada(), "{propuesta:?}");
    let rey = e.rey;
    e.ok("ana", Comando::ResponderAnexion { heroe: rey, propuesta: propuesta.creadas[0], aceptar: true });
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2);
    assert!(e.n.residentes().aedas.iter().all(|a| a.faccion == 2));
    assert_eq!((e.n.residentes().cumplidas_de(1), e.n.residentes().cumplidas_de(2)), (0, 1));
    assert_eq!(e.n.medidas_de_titulos().iter().find(|x| x.faccion == 2).unwrap().mecenas, 11);
}

// ───────────────────────────── 3. La foto y el determinismo ─────────────────────────────

/// La foto `BAN40` lleva los residentes, sus épicas con sus claves y las citas de llegada, ida y vuelta con el mismo hash; `BAN38` se rechaza con su motivo.
fn ida_y_vuelta(e: &Escenario) {
    let (reglas, mundo_hash) = (e.n.estado().reglas.clone(), e.n.estado().mundo_hash.clone());
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN50");
    let vuelta = Estado::deserializar(&bytes, &reglas, &mundo_hash).unwrap();
    assert_eq!(vuelta.serializar(), bytes);
    assert_eq!(vuelta.hash(), e.n.estado().hash());
    assert_eq!((&vuelta.residentes, &vuelta.residentes_citas), (&e.n.estado().residentes, &e.n.estado().residentes_citas));
    let residentes = |x: &Estado| x.cola.volcar().into_iter().filter(|c| matches!(c.carga, Carga::Residentes)).count();
    assert_eq!(residentes(&vuelta), residentes(e.n.estado()));
    let vieja = [b"BAN38".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &mundo_hash).err().expect("BAN38 se rechaza");
    assert!(error.0.contains("BAN38") && error.0.contains("Aedas residentes"), "{}", error.0);
}

#[test]
fn la_foto_ban40_lleva_los_residentes_y_la_cita_de_llegada() {
    let m = con_mundo!();
    let mut e = con_residente(m, reglas(|_| {}));
    let plaza = e.plaza;
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 3 });
    assert_eq!(e.citas().len(), 1, "una plaza que espera");
    ida_y_vuelta(&e);
}

/// Funda, levanta el Palacio, espera al primer Aeda, empieza su épica, encarga un Almacén (el primer capítulo) y avanza 3 días: cubre la espera, la cita de llegada, la épica con su clave y
/// el capítulo.
fn guion(m: &M, n: Nucleo, avanzar: &dyn Fn(&mut Nucleo, Instante)) -> Nucleo {
    let mut e = con_palacio_en(m, n, 20_000_000);
    e.hasta(2 * DIA, "el primer Aeda", |n| !n.residentes().aedas.is_empty());
    e.ok("ana", e.empezar(1, "canteria"));
    e.anadir(Tipo::Almacen);
    let t = e.n.ahora() + 3 * DIA;
    avanzar(&mut e.n, t);
    assert_eq!(e.n.residentes().aedas[0].epica.as_ref().map(|x| x.capitulo), Some(1), "el guion cierra el primer capítulo");
    e.n
}

#[test]
fn el_mismo_mundo_de_una_vez_a_trozos_y_tras_una_foto() {
    let m = con_mundo!();
    let r = || reglas(|_| {});
    let de_una = guion(m, Nucleo::nuevo(arranque(m, r())), &|n, t| n.avanzar_hasta(t));
    let a_trozos = guion(m, Nucleo::nuevo(arranque(m, r())), &|n, t| {
        let desde = n.ahora();
        for s in 1..=7 {
            n.avanzar_hasta(desde + (t - desde) * s * s / 49);
        }
    });
    assert_eq!(de_una.hash_estado(), a_trozos.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    // Con una foto con el capítulo en curso y una cita pendiente, recuperado sigue igual que el que no se interrumpió.
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |a: Ajustes| Arranque { ajustes: a, ..arranque(m, r()) };
    let dir = tempfile::tempdir().unwrap();
    let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
    let mut cortado = guion(m, n, &|n, t| {
        let desde = n.ahora();
        n.avanzar_hasta(desde + (t - desde) / 2);
        n.foto_ahora();
        n.avanzar_hasta(desde + (t - desde) / 2 + HORA);
        n.apagar().unwrap();
        *n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        assert!(n.recuperado());
        n.avanzar_hasta(t);
    });
    assert_eq!(cortado.hash_estado(), de_una.hash_estado());
    cortado.apagar().unwrap();
}

// ───────────────────────────── 4. Por la sesión ─────────────────────────────

#[tokio::test]
async fn por_la_sesion_el_rey_empieza_y_abandona_una_epica_y_su_faccion_oye_el_hecho() {
    use comun::sesion::*;
    use contrato::v1::{self as pb, orden, TipoCliente};
    let m = con_mundo!();
    let mut e = con_residente(m, reglas(|_| {}));
    let plaza = e.plaza;
    // Por la sesión se juega con un héroe de verdad: «ana» crea el suyo, entra en la Facción, el fundador le cede el trono y la administración lo pone dentro.
    let r = e.ord("ana", Comando::CrearHeroe { cuenta: "ana".into(), campamento: 0, controlador: militar::heroe::Controlador::Humano, nombre: None });
    let rey = r.creadas[0];
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: rey, faccion: Some(1) });
    let fundador = e.rey;
    e.ok("ana", Comando::AsignarRey { heroe: fundador, faccion: 1, designado: rey });
    e.ok("admin", Comando::ColocarHeroe { heroe: rey, ubicacion: militar::presencia::Ubicacion::Asentamiento(plaza as u32) });
    let r = e.ord("eva", Comando::CrearHeroe { cuenta: "eva".into(), campamento: 0, controlador: militar::heroe::Controlador::Humano, nombre: None });
    assert!(r.aceptada());
    let w = arrancar_con(e.n).await;
    use contrato::v1::entidad::Cuerpo;
    let cuerpos = |ms: &[contrato::v1::mensaje_de_servidor::Cuerpo]| -> Vec<(String, Cuerpo)> {
        use contrato::v1::mensaje_de_servidor::Cuerpo as M;
        let a = |x: &Option<pb::Ambito>| contrato::almacen::clave_de_ambito(x);
        ms.iter()
            .flat_map(|m| match m {
                M::Foto(f) => f.entidades.iter().filter_map(|e| e.cuerpo.clone()).map(|c| (a(&f.ambito), c)).collect::<Vec<_>>(),
                M::Delta(d) => d.cambios.iter().filter_map(|c| c.datos.as_ref()?.cuerpo.clone()).map(|c| (a(&d.ambito), c)).collect(),
                _ => Vec::new(),
            })
            .collect()
    };
    let residente = |todo: &[(String, Cuerpo)]| todo.iter().rev().find_map(|(a, x)| if let Cuerpo::AedaResidente(r) = x { Some((a.clone(), r.clone())) } else { None });
    let posibles = |todo: &[(String, Cuerpo)]| todo.iter().rev().find_map(|(_, x)| if let Cuerpo::Tecnologias(t) = x { Some(t.epicas_posibles.clone()) } else { None });
    // 6G: quien no es de la Facción (eva, sin ella) no ve a sus residentes.
    let (otra, _, recibido) = w.conectar(servidor::prueba::hola("ok:eva", TipoCliente::ClienteUnity)).await;
    assert!(residente(&cuerpos(&recibido)).is_none(), "los rivales no ven los residentes ajenos");
    otra.cerrar().await;
    let (mut c, _, recibido) = w.conectar(servidor::prueba::hola("ok:ana", TipoCliente::ClienteUnity)).await;
    // 6G: el residente en FACCION, aún sin épica, y la épica que podría empezar.
    let todo = cuerpos(&recibido);
    let (ambito, r1) = residente(&todo).expect("su residente en FACCION");
    assert_eq!((ambito.as_str(), r1.id.as_str(), r1.asentamiento_id.clone(), r1.faccion_id.as_str(), r1.epica.is_none()), ("FACCION:f-1", "aeda-r1", format!("as-{plaza}"), "f-1", true));
    assert!(posibles(&todo).expect("sus tecnologías").contains(&"canteria".to_owned()));
    let empezar = |aeda: &str, x: &str| orden::Cuerpo::EmpezarEpica(pb::OrdenEmpezarEpica { asentamiento_id: format!("as-{plaza}"), aeda_id: aeda.into(), tecnologia_id: x.into() });
    let (_, r) = dar(&mut c, "e1", empezar("aeda-9", "canteria")).await;
    assert_eq!(rechazo(&r), "epica.no_vive_aqui", "ya no es `orden.no_disponible`");
    let (_, r) = dar(&mut c, "e2", empezar("aeda-r1", "no_existe")).await;
    assert_eq!(rechazo(&r), "tecnologia.desconocida");
    let (deltas, r) = dar(&mut c, "e3", empezar("aeda-r1", "canteria")).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    let empieza = deltas.iter().flat_map(|d| &d.eventos).find(|x| x.codigo == "aedas.epica_empieza").expect("el hecho llega a la Facción");
    assert_eq!(empieza.asentamiento_id.as_deref(), Some(format!("as-{plaza}").as_str()));
    let Some(pb::evento_dominio::Tipado::EpicaCambia(t)) = &empieza.tipado else { panic!("tipado: {empieza:?}") };
    assert_eq!((t.aeda_id.as_str(), t.como.as_str(), t.tecnologia_id.as_deref(), t.capitulo, t.capitulos), ("aeda-r1", "empieza", Some("canteria"), 0, 2));
    // 6G: el residente con su épica (sin la memoria de claves, que es de la administración) y la épica ya no es posible.
    let todo = cuerpos(&deltas.iter().cloned().map(contrato::v1::mensaje_de_servidor::Cuerpo::Delta).collect::<Vec<_>>());
    let (_, r1) = residente(&todo).expect("el residente en la Delta de FACCION");
    let x = r1.epica.expect("canta");
    assert_eq!((x.tecnologia_id.as_str(), x.capitulo, x.capitulos, x.hechos, x.hechos_necesarios, x.claves.len()), ("canteria", 0, 2, 0, 1, 0));
    assert!(!posibles(&todo).expect("sus tecnologías").contains(&"canteria".to_owned()), "ya la canta");
    let (_, r) = dar(&mut c, "e4", empezar("aeda-r1", "canteria")).await;
    assert_eq!(rechazo(&r), "epica.ya_canta");
    let abandonar = orden::Cuerpo::AbandonarEpica(pb::OrdenAbandonarEpica { asentamiento_id: format!("as-{plaza}"), aeda_id: "aeda-r1".into() });
    let (deltas, r) = dar(&mut c, "e5", abandonar).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    assert!(deltas.iter().flat_map(|d| &d.eventos).any(|x| x.codigo == "aedas.epica_abandonada"));
    c.cerrar().await;
    // 6G: la administración ve al residente.
    let (adm, _, recibido) = w.conectar(servidor::prueba::hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
    assert_eq!(residente(&cuerpos(&recibido)).map(|(a, r)| (a, r.id)), Some(("ADMIN:".to_owned(), "aeda-r1".to_owned())));
    adm.cerrar().await;
    let n = w.parar().await;
    assert!(n.residentes().aedas[0].epica.is_none());
}

// ───────────────────────────── 5. La conquista, la ruina y el traslado ─────────────────────────────

#[test]
fn la_conquista_pasa_los_residentes_a_la_conquistadora_y_pierden_la_epica() {
    let m = con_mundo!();
    let mut e = con_residente(m, reglas(|_| {}));
    let plaza = e.plaza;
    e.ok("ana", e.empezar(1, "canteria"));
    // Un ejército de la Facción 2 asedia la plaza (con números: sin servidores de batalla).
    let (t, p) = (e.n.ahora(), e.n.estado().asentamientos[&plaza].posicion);
    let col = e.n.crear_ejercito(t, "beto", 2, base::Punto::new(p.x + 2_000, p.y), 500, 350_000).unwrap();
    let h = e.n.ejercito(col).unwrap().columna.lider;
    for tropa in [militar::catalogo::Tropa::EspadachinesBronce, militar::catalogo::Tropa::ArquerosCompuesto] {
        let cantidad = militar::Reglas::cargar().tropas.tropa(tropa).unidades;
        e.ok("admin", Comando::DarEscuadra { heroe: h, tropa, cantidad });
    }
    e.ok("beto", Comando::Atacar { heroe: h, objetivo: nucleo::ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    let fin = e.n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max().expect("la convocatoria del asedio");
    e.n.avanzar_hasta(fin);
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2, "conquistada");
    // Los residentes que quedan son de la conquistadora y sin épica; la de antes avisa a la Facción vieja.
    assert!(e.n.residentes().aedas.iter().all(|a| a.faccion == 2 && a.epica.is_none()));
    let ab = hechos(&e.n, "aedas.epica_abandonada");
    assert_eq!((ab.len(), texto(&ab[0], "motivo").as_deref(), ab[0].facciones.clone(), ab[0].dato("plaza")), (1, Some("conquista"), vec![1], Some(plaza as i64)));
    // La plaza se revisó tras el saqueo: tiene a lo sumo su cupo y, si espera, una sola cita.
    assert!(e.citas().len() <= 1);
    assert_eq!(e.n.residentes_en(plaza) as usize, e.n.residentes().aedas.iter().filter(|a| a.plaza == plaza as u32).count());
}

#[test]
fn cuando_el_cupo_baja_el_aeda_se_va_y_la_ruina_no_deja_nada_en_la_cola() {
    let m = con_mundo!();
    // Con pocos recursos: sin auto-construcción, el mantenimiento colapsa y la plaza acaba en ruinas.
    let mut e = con_palacio_y(m, reglas(|_| {}), 8_000_000);
    let plaza = e.plaza;
    e.hasta(2 * DIA, "el primer Aeda", |n| !n.residentes().aedas.is_empty());
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 3 });
    let fin = e.n.ahora() + 30 * DIA;
    while e.n.economia(plaza).is_some() {
        assert!(e.n.ahora() < fin, "la plaza no cae");
        e.avanzar(HORA);
    }
    // El mantenimiento colapsa antes de la ruina: sin nobleza o sin nivel, el cupo cae a 0 y el Aeda se va (antes de que llegue el segundo).
    let se_van = hechos(&e.n, "aedas.residente_se_va");
    assert_eq!(se_van.iter().map(|h| (h.dato("aedaResidente"), h.dato("cupo"), h.asentamientos.clone())).collect::<Vec<_>>(), vec![(Some(1), Some(0), vec![plaza])]);
    assert!(e.n.residentes().aedas.is_empty() && e.n.estado().residentes.espera_desde.is_empty() && e.n.estado().residentes_citas.is_empty(), "la ruina se los lleva");
    assert!(e.citas().is_empty() && e.n.residentes_en(plaza) == 0, "nada en la cola");
}
