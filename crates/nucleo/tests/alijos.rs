//! Los alijos de exploración (sistema 6, tanda 6F; `Generacion_Del_Mundo.md` §12): `AbrirAlijo` (174) con cada rechazo, el oro al carro de la columna en unidades enteras (lo que no cabe se
//! pierde, decisión 10 del plan del sistema 6), la conservación, «a la vista», que nada programa eventos, la foto `BAN36` y una prueba por el WebSocket real. Los casos de «ver y abrir un
//! alijo» de `typescript-final:src/engine/__tests__/alijos.test.ts` (las reglas puras y sus pruebas están en `tecnologia::alijos`). Sobre el artefacto de mundo real; se omiten si no está.

mod comun;

use asentamiento::Recurso::{self, *};
use base::{Mil, Punto};
use comun::real::{campamento, en_tierra, fundar, mundo, M};
use comun::sesion::*;
use contrato::v1::{self as pb, orden, TipoCliente};
use militar::columna::Politica;
use militar::heroe::Controlador;
use nucleo::{rechazos, Ajustes, Arranque, Comando, Dato, Estado, Nucleo, Peticion, Reglas, Resultado, Respuesta};

const U: Mil = 1_000;

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

fn nucleo(m: &M) -> Nucleo {
    let mut reglas = Reglas::default();
    // Sin bandidos que acosen ni aparezcan: no son de esta prueba.
    let b = &mut std::sync::Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) })
}

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

struct Esc {
    n: Nucleo,
    peticiones: u64,
}

impl Esc {
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.peticiones += 1;
        let p = Peticion::nueva(actor, &format!("p{}", self.peticiones), c);
        let t = self.n.ahora();
        self.n.aplicar_en(t, if actor == "admin" { p.de_admin() } else { p })
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    /// Un rechazo que no deja rastro.
    fn no(&mut self, actor: &str, c: Comando) -> &'static str {
        let antes = (self.n.version(), self.n.hash_estado());
        let r = self.ord(actor, c);
        assert_eq!((self.n.version(), self.n.hash_estado()), antes, "el rechazo {r:?} dejó rastro");
        codigo(&r)
    }
    /// Una columna de administración de `cuenta` en `p`, de la Facción `f` (que nace sin plazas): su id y el de su héroe.
    fn columna(&mut self, cuenta: &str, f: u32, p: Punto) -> (u64, u64) {
        let t = self.n.ahora();
        let col = self.n.crear_ejercito(t, cuenta, f, p, 500, 100_000).unwrap();
        (col, self.n.ejercito(col).unwrap().columna.lider)
    }
    /// Cuánto lleva el carro de la columna de ese recurso.
    fn lleva(&self, col: u64, r: Recurso) -> Mil {
        self.n.ejercito(col).unwrap().columna.carga.get(&r).copied().unwrap_or(0)
    }
    fn abrir(&mut self, cuenta: &str, heroe: u64, alijo: u32) -> Respuesta {
        self.ord(cuenta, Comando::AbrirAlijo { heroe, alijo })
    }
}

/// El alijo más cercano a la puerta del campamento `c` y su sitio.
fn alijo_junto_a(m: &M, c: usize) -> (u32, Punto) {
    let puerta = campamento(&m.0, c);
    m.0.alijos()
        .iter()
        .enumerate()
        .map(|(i, a)| (i as u32, Punto::new(a.posicion.unwrap().x, a.posicion.unwrap().y)))
        .min_by_key(|(_, p)| p.dist2(puerta))
        .expect("el artefacto trae alijos")
}

/// Los hechos `alijo.abierto`: sus datos enteros y el id del alijo.
fn abiertos(n: &Nucleo) -> Vec<(Vec<(&'static str, i64)>, String)> {
    n.cambios_desde(0)
        .unwrap()
        .flat_map(|c| c.eventos.iter())
        .filter(|e| e.codigo == "alijo.abierto")
        .map(|e| (e.datos.clone(), e.valores.iter().find_map(|(k, v)| if let (&"alijo", Dato::Texto(s)) = (k, v) { Some(s.clone()) } else { None }).expect("el id del alijo")))
        .collect()
}

#[test]
fn se_abre_en_el_sitio_una_vez_por_heroe_y_lo_que_trae_sube_al_carro() {
    let m = con_mundo!();
    let (alijo, sitio) = alijo_junto_a(m, 0);
    let oro = m.0.alijos()[alijo as usize].cantidad;
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    // Trae una sola cosa (oro, madera, piedra o trigo): la que diga el artefacto.
    let r = e.n.alijo(alijo).unwrap().recurso;
    assert_eq!(r.nombre(), m.0.alijos()[alijo as usize].recurso);
    let (col, ana) = e.columna("ana", 1, sitio);
    // Se ve mientras no lo abre y nada lo ha programado.
    assert!(e.n.puede_buscar_alijos(ana));
    assert!(e.n.alijos_a_la_vista(ana, e.n.ahora()).contains(&alijo));
    let cola = e.n.estado().cola.len();
    e.ok("ana", Comando::AbrirAlijo { heroe: ana, alijo });
    assert_eq!(e.n.estado().cola.len(), cola, "abrir un alijo no programa nada: los alijos no tienen tick");
    assert_eq!(e.lleva(col, r), base::entero(oro), "lo que trae, en unidades enteras, al carro de la columna");
    assert!(e.n.heroe(ana).unwrap().alijos_abiertos.contains(&alijo));
    assert!(!e.n.alijos_a_la_vista(ana, e.n.ahora()).contains(&alijo), "abierto ya no se ve");
    let hechos = abiertos(&e.n);
    assert_eq!(hechos.len(), 1);
    assert_eq!(hechos[0].0, vec![("heroe", ana as i64), ("cantidad", base::entero(oro)), ("cargado", base::entero(oro))]);
    assert_eq!(hechos[0].1, m.0.alijos()[alijo as usize].id);
    // Solo lo oye quien lo abre, no los demás de su columna (decisión del autor).
    let hecho = e.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).find(|x| x.codigo == "alijo.abierto").unwrap().clone();
    assert_eq!((hecho.heroes, hecho.ejercitos), (vec![ana], vec![]));
    // Una vez por héroe; otro héroe en el mismo sitio abre el suyo (el alijo no desaparece).
    assert_eq!(e.no("ana", Comando::AbrirAlijo { heroe: ana, alijo }), rechazos::ALIJO_YA_ABIERTO);
    let (col_b, beto) = e.columna("beto", 2, sitio);
    assert!(e.abrir("beto", beto, alijo).aceptada());
    assert_eq!(e.lleva(col_b, r), base::entero(oro));
}

#[test]
fn cada_rechazo_con_su_codigo_y_sin_rastro() {
    let m = con_mundo!();
    let (alijo, sitio) = alijo_junto_a(m, 0);
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    let (_, ana) = e.columna("ana", 1, sitio);
    assert_eq!(e.no("ana", Comando::AbrirAlijo { heroe: ana, alijo: 99_999 }), rechazos::ALIJO_DESCONOCIDO);
    assert_eq!(e.no("beto", Comando::AbrirAlijo { heroe: ana, alijo }), rechazos::NO_AUTORIZADO, "el héroe es de otra cuenta");
    // Lejos: a 20 u del alijo (más que `radioPuerta`).
    let (_, lejos) = e.columna("cris", 3, en_tierra(&m.0, sitio, 20 * U, 0));
    assert!(e.n.ejercito(e.n.columna_del_heroe(lejos).unwrap().id).unwrap().posicion_en(e.n.ahora()).dist(sitio) > 10 * U);
    assert_eq!(e.no("cris", Comando::AbrirAlijo { heroe: lejos, alijo }), rechazos::ALIJO_LEJOS);
    // Con una plaza en su Facción ya no se buscan alijos: ni se ven ni se abren.
    let otro = comun::real::sitio_rico(&e.n, &m.0, 3, 1);
    fundar(&mut e.n, "dora", 1, otro);
    let t = e.n.ahora();
    e.n.avanzar_hasta(t); // lo que la fundación dejó para este instante
    assert!(!e.n.puede_buscar_alijos(ana));
    assert!(e.n.alijos_a_la_vista(ana, e.n.ahora()).is_empty());
    assert_eq!(e.no("ana", Comando::AbrirAlijo { heroe: ana, alijo }), rechazos::ALIJO_NO_PUEDES_BUSCAR);
}

#[test]
fn con_el_carro_casi_lleno_cabe_lo_que_cabe_y_el_resto_se_pierde() {
    let m = con_mundo!();
    let (alijo, sitio) = alijo_junto_a(m, 0);
    let oro = base::entero(m.0.alijos()[alijo as usize].cantidad);
    assert!(oro > U, "el alijo trae más de una unidad");
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    let r = e.n.alijo(alijo).unwrap().recurso;
    // El relleno, de algo que el alijo no trae, para que lo cargado se vea solo.
    let (relleno, relleno_b) = if r == Madera { (Piedra, Trigo) } else if r == Piedra { (Madera, Trigo) } else { (Madera, Piedra) };
    let (col, ana) = e.columna("ana", 1, sitio);
    let capacidad = e.n.capacidad_de_carga(col).unwrap();
    let lleva: Mil = e.n.ejercito(col).unwrap().columna.carga.values().sum();
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: relleno, cantidad: capacidad - lleva - U });
    let antes = (e.lleva(col, r), e.n.ejercito(col).unwrap().columna.carga.values().sum::<Mil>());
    assert_eq!(antes.1, capacidad - U, "queda una unidad libre");
    e.ok("ana", Comando::AbrirAlijo { heroe: ana, alijo });
    // Conservación: entra en el carro exactamente lo cargado; lo demás no va a ninguna parte.
    assert_eq!(e.lleva(col, r), antes.0 + U);
    let (datos, _) = abiertos(&e.n).pop().unwrap();
    assert_eq!(datos, vec![("heroe", ana as i64), ("cantidad", oro), ("cargado", U)]);
    // Con el carro lleno se abre igual (cargado 0) y queda abierto.
    let (col_b, beto) = e.columna("beto", 2, sitio);
    let lleva: Mil = e.n.ejercito(col_b).unwrap().columna.carga.values().sum();
    e.ok("admin", Comando::DarSuministro { ejercito: col_b, recurso: relleno_b, cantidad: e.n.capacidad_de_carga(col_b).unwrap() - lleva });
    e.ok("beto", Comando::AbrirAlijo { heroe: beto, alijo });
    assert_eq!((e.lleva(col_b, r), abiertos(&e.n).pop().unwrap().0[2]), (0, ("cargado", 0)));
    assert!(e.n.heroe(beto).unwrap().alijos_abiertos.contains(&alijo));
}

#[test]
fn los_alijos_abiertos_viajan_en_la_foto_ban36_y_ban35_se_rechaza() {
    let m = con_mundo!();
    let (alijo, sitio) = alijo_junto_a(m, 1);
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    let (_, ana) = e.columna("ana", 1, sitio);
    e.ok("ana", Comando::AbrirAlijo { heroe: ana, alijo });
    let (reglas, hash) = (e.n.estado().reglas.clone(), m.0.hash().to_string());
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN47");
    let s = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
    assert_eq!((s.militar.heroes[&ana].alijos_abiertos.iter().copied().collect::<Vec<_>>(), s.hash()), (vec![alijo], e.n.hash_estado()));
    let mut otro = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
    otro.militar.heroes.get_mut(&ana).unwrap().alijos_abiertos.clear();
    assert_ne!(otro.hash(), s.hash(), "los alijos abiertos entran en el hash");
    let vieja = [b"BAN35".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &hash).err().expect("BAN35 se rechaza");
    assert!(error.0.contains("BAN35") && error.0.contains("alijos"), "{}", error.0);
}

fn heroes(entidades: impl IntoIterator<Item = Option<pb::Entidad>>) -> Vec<pb::Heroe> {
    entidades.into_iter().flatten().filter_map(|e| if let Some(pb::entidad::Cuerpo::Heroe(h)) = e.cuerpo { Some(h) } else { None }).collect()
}

#[tokio::test]
async fn por_la_sesion_se_abre_un_alijo_y_el_heroe_lo_lleva_en_heroe_y_en_admin() {
    let m = con_mundo!();
    let (alijo, sitio) = alijo_junto_a(m, 0);
    let id = m.0.alijos()[alijo as usize].id.clone();
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    // Un héroe sin Facción que sale del campamento 0 y marcha hasta el alijo.
    let ana = e.ok("ana", Comando::CrearHeroe { cuenta: "ana".into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
    let col = e.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: ana, escuadras: vec![], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    e.ok("ana", Comando::MarcharA { ejercito: col, destino: sitio });
    let fin = e.n.ejercito(col).unwrap().plan.fin().unwrap();
    e.n.avanzar_hasta(fin);
    assert!(e.n.alijos_a_la_vista(ana, e.n.ahora()).contains(&alijo));
    let w = arrancar_con(e.n).await;
    let (mut c, f) = w.entrar("ana", TipoCliente::ClienteUnity).await;
    // 6G: el alijo a la vista viaja en su `HEROE`, con su sitio y su oro.
    let a_la_vista: Vec<pb::Alijo> = f.entidades.iter().filter_map(|e| if let Some(pb::entidad::Cuerpo::Alijo(a)) = &e.cuerpo { Some(a.clone()) } else { None }).collect();
    assert!(a_la_vista.iter().any(|a| a.id == id && a.cantidad == m.0.alijos()[alijo as usize].cantidad && a.recurso == m.0.alijos()[alijo as usize].recurso && a.posicion.as_ref().map(|p| (p.x, p.y)) == Some((sitio.x, sitio.y))), "{a_la_vista:?}");
    assert!(heroes(f.entidades.into_iter().map(Some))[0].alijos_abiertos.is_empty());

    let abrir = |x: &str| orden::Cuerpo::AbrirAlijo(pb::OrdenAbrirAlijo { alijo_id: x.into() });
    let (deltas, r) = dar(&mut c, "a1", abrir(&id)).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    let d = del_heroe(&deltas);
    let h = heroes(d.iter().flat_map(|d| d.cambios.iter().map(|x| x.datos.clone())));
    assert_eq!(h.last().unwrap().alijos_abiertos, vec![id.clone()], "el héroe propio con el alijo abierto, por su id del artefacto");
    let hecho = d.iter().flat_map(|d| &d.eventos).find(|x| x.codigo == "alijo.abierto").expect("el hecho llega al héroe");
    let Some(pb::evento_dominio::Tipado::AlijoAbierto(t)) = &hecho.tipado else { panic!("tipado: {hecho:?}") };
    assert_eq!((t.heroe_id.clone(), t.alijo_id.as_str(), t.cantidad > 0, t.cargado), (format!("heroe-{ana}"), id.as_str(), true, t.cantidad));
    // 6G: abierto, deja de estar a la vista: sale de su `HEROE` (en el reparto de la orden, antes del resultado).
    assert!(d.iter().flat_map(|d| &d.cambios).any(|x| x.tipo == pb::TipoCambio::CambioQuita as i32 && x.entidad == pb::TipoEntidad::Alijo as i32 && x.id == id), "{d:?}");
    let (_, r) = dar(&mut c, "a2", abrir(&id)).await;
    assert_eq!(rechazo(&r), "alijo.ya_abierto");
    let (_, r) = dar(&mut c, "a3", abrir("alijo-de-nadie")).await;
    assert_eq!(rechazo(&r), "alijo.desconocido", "ya no es `orden.no_disponible`");
    c.cerrar().await;
    // 6G: quien es de una Facción con plazas no busca alijos: bea, con la suya, no recibe ninguno aunque esté en el mismo sitio.
    let mut n = w.parar().await;
    fundar(&mut n, "otro", 7, en_tierra(&m.0, sitio, 60_000, 0));
    let t = n.ahora();
    n.crear_ejercito(t, "bea", 7, sitio, 500, 100_000).unwrap();
    let w = arrancar_con(n).await;
    let (c, f) = w.entrar("bea", TipoCliente::ClienteUnity).await;
    assert!(!f.entidades.iter().any(|e| matches!(e.cuerpo, Some(pb::entidad::Cuerpo::Alijo(_)))), "los alijos solo los ve quien no tiene Facción o es de una sin plazas");
    c.cerrar().await;
    // La administración ve los alijos abiertos de cada héroe.
    let (admin, f) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let h = heroes(f.entidades.into_iter().map(Some));
    assert_eq!(h.iter().find(|x| x.id == format!("heroe-{ana}")).unwrap().alijos_abiertos, vec![id.clone()]);
    admin.cerrar().await;
    let n = w.parar().await;
    let r = n.alijo(n.alijo_por_id(&id).unwrap()).unwrap().recurso;
    assert!(n.ejercito(col).unwrap().columna.carga.get(&r).copied().unwrap_or(0) > 0, "lo que trae el alijo, en el carro");
}
