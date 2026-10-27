//! La política en el contrato (sistema 5, tanda 5H): las propuestas en `FACCION` de las dos Facciones (y en `ADMIN`), que salen con `QUITA` al contestarlas, retirarlas o podarlas; la Liga en
//! cada Facción miembro, que cambia con sus relaciones; la ciudadanía y las solicitudes propias en el `Heroe`; la forma pública de las demás Facciones en `VISTA`; los fundadores de cada plaza
//! (estado nuevo, foto `BAN36`) y los ocho hechos políticos tipados. Lo que llega por el WebSocket real lo prueba el cliente de ejemplo (`tests/cliente_de_ejemplo.rs`). Sobre el artefacto de
//! mundo real; se omiten si no está.

mod comun;

use base::{Instante, MS_POR_MINUTO};
use comun::real::{fundar, sitio_rico, M};
use contrato::v1 as pb;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::proyeccion::ambito::Ambito;
use nucleo::proyeccion::propios::{proyectar, Espectador};
use nucleo::proyeccion::vista::{Mirada, Perspectiva};
use nucleo::proyeccion::{audiencia, eventos, ids, interior, propios};
use nucleo::{Ajustes, Arranque, Clase, Comando, Estado, Nucleo, Peticion, Reglas, Respuesta};
use pb::entidad::Cuerpo;
use pb::evento_dominio::Tipado;

const MIN: Instante = MS_POR_MINUTO;
const DIA: Instante = 24 * 60 * MIN;

macro_rules! con_mundo {
    () => {
        match comun::real::mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

fn arranque(m: &M) -> Arranque {
    let mut reglas = Reglas::default();
    let b = &mut std::sync::Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    Arranque { reglas, ajustes: Ajustes::default(), semilla: 9, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

struct E {
    n: Nucleo,
    k: u64,
}

impl E {
    fn nuevo(m: &M) -> Self {
        E { n: Nucleo::nuevo(arranque(m)), k: 0 }
    }
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.k += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.k), c);
        let p = if actor == "admin" { p.de_admin() } else { p };
        let t = self.n.ahora();
        self.n.aplicar_en(t, p)
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn heroe(&mut self, cuenta: &str) -> u64 {
        self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0]
    }
    /// Una Facción nueva con su Rey (el héroe de la cuenta).
    fn faccion(&mut self, cuenta: &str, nombre: &str) -> (u32, u64) {
        let h = self.heroe(cuenta);
        (self.ok(cuenta, Comando::CrearFaccion { heroe: h, nombre: nombre.into(), sigilo: None }).creadas[0] as u32, h)
    }
    /// Lo que tocó la última versión de una clase.
    fn tocadas(&self, clase: Clase) -> Vec<u64> {
        self.n.cambios_desde(0).unwrap().last().map(|c| c.tocados.de(clase).to_vec()).unwrap_or_default()
    }
    fn existe(&self, clase: Clase, id: u64) -> Option<Cuerpo> {
        proyectar(&self.n, Espectador::Admin, clase, id).existen.into_iter().next().map(|x| x.2)
    }
    /// Los hechos tipados del registro, con su código.
    fn tipados(&self) -> Vec<(String, Tipado)> {
        let n = &self.n;
        n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter().map(move |ev| (ev.codigo.to_string(), eventos::evento_dominio(n, ev, c.version).tipado))).filter_map(|(c, t)| Some((c, t?))).collect()
    }
}

fn propuesta(c: Option<Cuerpo>) -> pb::Propuesta {
    match c {
        Some(Cuerpo::Propuesta(p)) => p,
        otro => panic!("no es una propuesta: {otro:?}"),
    }
}

// ───────────────────────────── Las propuestas ─────────────────────────────

#[test]
fn las_propuestas_van_a_las_dos_facciones_y_salen_al_contestarlas_retirarlas_o_podarlas() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let ((f1, r1), (f2, r2), (f3, r3)) = (e.faccion("ana", "Micenas"), e.faccion("beto", "Tebas"), e.faccion("gus", "Argos"));
    // Una alianza que F1 propone a F2: en el registro, en las dos Facciones (no en la tercera) y con quién la hizo (5A-7: el héroe).
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: r1, a: f1, b: f2, vasallaje: false, tributo: None }).creadas[0];
    assert_eq!(e.tocadas(Clase::Propuesta), vec![p]);
    assert_eq!(audiencia::de(e.n.estado(), Clase::Propuesta, p), vec![Ambito::Faccion(f1), Ambito::Faccion(f2)]);
    let x = propuesta(e.existe(Clase::Propuesta, p));
    assert_eq!((x.id.clone(), x.propuesta_por.clone(), x.expira_en_ms - x.creada_en_ms), (ids::propuesta(p), ids::heroe(r1), 3 * DIA));
    let Some(pb::propuesta::Clase::Relacion(r)) = x.clase else { panic!("de relación") };
    assert_eq!((r.tipo, r.faccion_a_id, r.faccion_b_id), (pb::TipoRelacion::RelacionAlianza as i32, ids::faccion(f1), ids::faccion(f2)));
    // Contestada, sale del almacén: la clase la marca y ya no existe (los ámbitos reciben `QUITA`).
    e.ok("beto", Comando::ResponderRelacion { heroe: r2, propuesta: p, aceptar: true });
    assert!(e.tocadas(Clase::Propuesta).contains(&p) && e.existe(Clase::Propuesta, p).is_none());
    // Anexión y fusión, con su clase; retirar la quita igual.
    let pa = e.ok("gus", Comando::ProponerAnexion { heroe: r3, absorbente: f3, absorbida: f2 }).creadas[0];
    let Some(pb::propuesta::Clase::Anexion(a)) = propuesta(e.existe(Clase::Propuesta, pa)).clase else { panic!("de anexión") };
    assert_eq!((a.absorbente_id, a.absorbida_id), (ids::faccion(f3), ids::faccion(f2)));
    e.ok("gus", Comando::RetirarAnexion { heroe: r3, propuesta: pa });
    assert!(e.tocadas(Clase::Propuesta).contains(&pa) && e.existe(Clase::Propuesta, pa).is_none());
    let pf = e.ok("ana", Comando::ProponerFusion { heroe: r1, a: f1, b: f3, nombre: "Hélade".into(), rey: r1 }).creadas[0];
    let Some(pb::propuesta::Clase::Fusion(f)) = propuesta(e.existe(Clase::Propuesta, pf)).clase else { panic!("de fusión") };
    assert_eq!((f.nuevo_nombre.as_str(), f.nuevo_rey_id), ("Hélade", ids::heroe(r1)));
    // Caducada sigue en el almacén (caducar no es un hecho: el cliente la descarta por `expira_en_ms`) hasta que otra de su clase la poda, y entonces sale.
    e.n.avanzar_hasta(e.n.ahora() + 4 * DIA);
    assert!(e.existe(Clase::Propuesta, pf).is_some(), "caducada pero en el almacén");
    let otra = e.ok("gus", Comando::ProponerFusion { heroe: r3, a: f3, b: f2, nombre: "Beocia".into(), rey: r3 }).creadas[0];
    assert_eq!(e.tocadas(Clase::Propuesta), vec![pf, otra], "la nueva y la podada");
    assert!(e.existe(Clase::Propuesta, pf).is_none());
}

// ───────────────────────────── La Liga ─────────────────────────────

#[test]
fn la_liga_viaja_en_cada_faccion_miembro_y_cambia_con_sus_relaciones() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let ((f1, r1), (f2, r2), (f3, r3)) = (e.faccion("ana", "Micenas"), e.faccion("beto", "Tebas"), e.faccion("gus", "Argos"));
    assert!(propios::faccion(&e.n, f1).liga.is_none(), "sola, no está en ninguna");
    // F1 es señora de F2 y F2 se alía con F3: los tres en una Liga cuyo Gran Rey es F1.
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: r1, a: f1, b: f2, vasallaje: true, tributo: None }).creadas[0];
    e.ok("beto", Comando::ResponderRelacion { heroe: r2, propuesta: p, aceptar: true });
    let p = e.ok("beto", Comando::ProponerRelacion { heroe: r2, a: f2, b: f3, vasallaje: false, tributo: None }).creadas[0];
    e.ok("gus", Comando::ResponderRelacion { heroe: r3, propuesta: p, aceptar: true });
    assert!(e.tocadas(Clase::Faccion).contains(&u64::from(f1)), "la alianza entre F2 y F3 cambia también la Liga de F1");
    let liga = propios::faccion(&e.n, f1).liga.expect("en una Liga");
    assert_eq!((liga.miembros_ids, liga.tiene_vasallaje, liga.gran_rey_id), ([f1, f2, f3].map(ids::faccion).to_vec(), true, Some(ids::faccion(f1))));
    assert_eq!(propios::faccion(&e.n, f3).liga, propios::faccion(&e.n, f1).liga, "la misma en cada miembro");
    // Rota la alianza, F3 sale de la Liga y las tres Facciones se marcan.
    let rel = e.n.relacion_activa(f2, f3).unwrap().id;
    e.ok("gus", Comando::RomperRelacion { heroe: r3, relacion: rel, iniciador: f3 });
    assert!([f1, f2, f3].iter().all(|f| e.tocadas(Clase::Faccion).contains(&u64::from(*f))));
    assert!(propios::faccion(&e.n, f3).liga.is_none());
    assert_eq!(propios::faccion(&e.n, f2).liga.unwrap().miembros_ids, [f1, f2].map(ids::faccion).to_vec());
}

// ───────────────────────────── El héroe y la forma pública ─────────────────────────────

#[test]
fn el_heroe_lleva_su_faccion_y_sus_solicitudes_y_la_vista_la_forma_publica_de_las_demas() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let ((f1, r1), (f2, r2), (f3, _)) = (e.faccion("ana", "Micenas"), e.faccion("beto", "Tebas"), e.faccion("gus", "Argos"));
    let cai = e.heroe("cai");
    let heroe = |e: &E| propios::heroe(&e.n, e.n.heroe(cai).unwrap());
    assert_eq!((heroe(&e).faccion_id, heroe(&e).solicitudes_faccion_ids), (None, vec![]));
    e.ok("cai", Comando::SolicitarIngreso { heroe: cai, faccion: f1 });
    assert!(e.tocadas(Clase::Heroe).contains(&cai), "sus solicitudes viajan en su héroe");
    e.ok("cai", Comando::SolicitarIngreso { heroe: cai, faccion: f2 });
    assert_eq!(heroe(&e).solicitudes_faccion_ids, [f1, f2].map(ids::faccion).to_vec());
    // Aceptado en F1: es su ciudadano y ya no pide nada a nadie.
    e.ok("ana", Comando::ResponderSolicitud { heroe: r1, faccion: f1, solicitante: cai, aceptar: true });
    assert!(e.tocadas(Clase::Heroe).contains(&cai) && e.tocadas(Clase::Faccion).contains(&u64::from(f2)));
    assert_eq!((heroe(&e).faccion_id, heroe(&e).solicitudes_faccion_ids), (Some(ids::faccion(f1)), vec![]));
    // La vista de F1 lleva la forma pública de F2, que nombra la relación (pública) entre las dos, sin experiencia ni capital; no la suya (va entera en `FACCION`) ni la de F3, que nada de lo que
    // se ve nombra.
    e.ok("admin", Comando::FijarNivelFaccion { faccion: f2, nivel: 3 });
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: r1, a: f1, b: f2, vasallaje: false, tributo: None }).creadas[0];
    e.ok("beto", Comando::ResponderRelacion { heroe: r2, propuesta: p, aceptar: true });
    assert!(f3 > f2);
    let mirada = Mirada::de(&e.n, Perspectiva::Faccion(f1));
    let todo = vision::Rect { min: base::Punto::new(i64::MIN / 4, i64::MIN / 4), max: base::Punto::new(i64::MAX / 4, i64::MAX / 4) };
    let facciones: Vec<pb::Faccion> = mirada.contenido(Some(r1), todo, &Default::default()).iter().filter_map(|x| x.cuerpo()).filter_map(|c| if let Cuerpo::Faccion(f) = c { Some(f) } else { None }).collect();
    assert_eq!(facciones.iter().map(|f| f.id.clone()).collect::<Vec<_>>(), vec![ids::faccion(f2)]);
    let f = &facciones[0];
    assert_eq!((f.nombre.as_str(), f.nivel, f.experiencia, f.capital_id.clone(), f.rey_id.is_some()), ("Tebas", 3, 0, None, true));
    assert!(propios::faccion(&e.n, f2).experiencia > 0, "la entera sí la lleva");
}

// ───────────────────────────── Los fundadores (BAN36) ─────────────────────────────

#[test]
fn los_fundadores_viajan_en_la_plaza_dejan_de_serlo_al_mudarse_y_van_en_la_foto_ban35() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    // Una columna de F1 con su héroe de campo (Líder) y ana, ciudadana: las dos fundan.
    let p = sitio_rico(&e.n, &m.0, 0, 1);
    let col = e.n.crear_ejercito(0, "gus", 1, p, 500, 100_000).unwrap();
    let gus = e.n.ejercito(col).unwrap().columna.lider;
    let ana = e.heroe("ana");
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: Some(1) });
    e.ok("admin", Comando::ColocarHeroe { heroe: ana, ubicacion: Ubicacion::Columna(col) });
    e.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
    let plaza = e.ok("gus", Comando::Fundar { ejercito: col }).creadas[0];
    assert_eq!(e.n.estado().asentamientos[&plaza].fundadores, vec![gus, ana], "en su orden, el Líder primero");
    assert!(e.n.estado().asentamientos[&plaza].cargos.es_titular(asentamiento::Cargo::Gobernador, gus), "el Líder es su Gobernador de oficio (5E-3)");
    // En el interior, en la ficha propia y no en la plaza ajena vista.
    assert_eq!(interior::asentamiento(&e.n, plaza).unwrap().fundadores_ids, [gus, ana].map(ids::heroe).to_vec());
    let Some(Cuerpo::Asentamiento(ficha)) = e.existe(Clase::Asentamiento, plaza) else { panic!("la ficha") };
    assert_eq!(ficha.fundadores_ids.len(), 2);
    assert!(nucleo::proyeccion::vista::plaza_avistada(&e.n, plaza).fundadores_ids.is_empty(), "quién la fundó no se ve desde fuera");
    // La foto (BAN35, hoy BAN36) los lleva y mueven el hash; BAN34 se rechaza con su motivo.
    let (reglas, hash) = (Reglas::default(), m.0.hash().to_string());
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN47");
    let s = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
    assert_eq!((s.asentamientos[&plaza].fundadores.clone(), s.hash()), (vec![gus, ana], e.n.hash_estado()));
    let mut otro = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
    otro.asentamientos.get_mut(&plaza).unwrap().fundadores.pop();
    assert_ne!(otro.hash(), s.hash());
    let vieja = [b"BAN34".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &hash).err().expect("BAN34 se rechaza");
    assert!(error.0.contains("BAN34") && error.0.contains("quién las fundó"), "{}", error.0);
    // Ana se muda a un campamento: deja de ser fundadora (como `heroesFundadoresIds` en el TypeScript).
    e.ok("admin", Comando::FijarResidencia { heroe: ana, residencia: Some(Residencia::Campamento(0)) });
    assert_eq!(e.n.estado().asentamientos[&plaza].fundadores, vec![gus]);
}

// ───────────────────────────── Los hechos tipados ─────────────────────────────

#[test]
fn los_hechos_politicos_llegan_tipados() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let ((f1, r1), (f2, r2), (f3, r3)) = (e.faccion("ana", "Micenas"), e.faccion("beto", "Tebas"), e.faccion("gus", "Argos"));
    let cai = e.heroe("cai");
    e.ok("cai", Comando::SolicitarIngreso { heroe: cai, faccion: f1 });
    e.ok("ana", Comando::ResponderSolicitud { heroe: r1, faccion: f1, solicitante: cai, aceptar: true });
    e.ok("ana", Comando::AsignarEmbajador { heroe: r1, faccion: f1, designado: cai });
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: r1, a: f1, b: f2, vasallaje: true, tributo: None }).creadas[0];
    e.ok("beto", Comando::ResponderRelacion { heroe: r2, propuesta: p, aceptar: true });
    let rel = e.n.relacion_activa(f1, f2).unwrap().id;
    e.ok("beto", Comando::RebelionVasallo { heroe: r2, relacion: rel });
    let guerra = e.n.relacion_activa(f1, f2).unwrap().id;
    e.ok("ana", Comando::ProponerPaz { heroe: r1, relacion: guerra, faccion: f1 });
    e.ok("beto", Comando::ProponerPaz { heroe: r2, relacion: guerra, faccion: f2 });
    let pa = e.ok("gus", Comando::ProponerAnexion { heroe: r3, absorbente: f3, absorbida: f2 }).creadas[0];
    e.ok("beto", Comando::ResponderAnexion { heroe: r2, propuesta: pa, aceptar: true });
    let t = e.tipados();
    let de = |codigo: &str| t.iter().filter(|(c, _)| c == codigo).map(|(_, x)| x.clone()).collect::<Vec<_>>();
    assert!(matches!(&de("faccion.creada")[0], Tipado::CiudadaniaCambiada(x) if x.como == "creada" && x.faccion_id == ids::faccion(f1)));
    assert!(matches!(&de("faccion.solicitud_ingreso")[0], Tipado::SolicitudDeIngreso(x) if x.heroe_id == ids::heroe(cai) && x.faccion_id == ids::faccion(f1)));
    assert!(matches!(&de("faccion.ciudadania_union")[0], Tipado::CiudadaniaCambiada(x) if x.como == "ingreso"));
    assert!(matches!(&de("cargo.embajador_asignado")[0], Tipado::CargoAsignado(x) if x.cargo == "embajador" && x.heroe_id == ids::heroe(cai) && x.asentamiento_id.is_none()));
    assert!(matches!(&de("diplomacia.relacion_propuesta")[0], Tipado::PropuestaDiplomatica(x) if x.clase == "relacion" && x.tipo == Some(pb::TipoRelacion::RelacionVasallaje as i32) && x.propuesta_por == ids::heroe(r1)));
    assert!(matches!(&de("diplomacia.relacion_aceptada")[0], Tipado::PropuestaResuelta(x) if x.resultado == "aceptada" && x.propuesta_id == Some(ids::propuesta(p)) && x.relacion_id == Some(ids::relacion(rel))));
    assert!(matches!(&de("diplomacia.rebelion_vasallo")[0], Tipado::RelacionTerminada(x) if x.como == "rebelion" && x.faccion_a_id == ids::faccion(f1)));
    assert!(matches!(&de("diplomacia.guerra_declarada")[0], Tipado::GuerraDeclarada(_)), "la rebelión declara la guerra");
    assert!(matches!(&de("diplomacia.paz_firmada")[0], Tipado::RelacionTerminada(x) if x.como == "paz" && x.por_faccion_id == Some(ids::faccion(f2))));
    assert!(matches!(&de("diplomacia.anexion_propuesta")[0], Tipado::PropuestaDiplomatica(x) if x.clase == "anexion" && x.faccion_a_id == ids::faccion(f3)));
    assert!(matches!(&de("diplomacia.anexion")[0], Tipado::FaccionesUnidas(x) if x.clase == "anexion" && x.resultante_id == ids::faccion(f3) && x.faccion_b_id == ids::faccion(f2)));
    // Con una plaza: el Gobernador de oficio al fundar y, a los diez minutos, los títulos.
    let p = sitio_rico(&e.n, &m.0, 0, f1);
    let plaza = fundar(&mut e.n, "dora", f1, p);
    e.n.avanzar_hasta(e.n.ahora() + 11 * MIN);
    let t = e.tipados();
    let de = |codigo: &str| t.iter().filter(|(c, _)| c == codigo).map(|(_, x)| x.clone()).collect::<Vec<_>>();
    assert!(matches!(de("cargo.local_asignado").last().unwrap(), Tipado::CargoAsignado(x) if x.cargo == "gobernador" && x.asentamiento_id == Some(ids::asentamiento(plaza))));
    assert!(de("titulo.nace").iter().any(|x| matches!(x, Tipado::TituloCambiado(t) if t.faccion_id == ids::faccion(f1) && !t.titulo_id.is_empty())));
}
