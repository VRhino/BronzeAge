//! El registro de cambios por clase de entidad (4H, T0): cada orden o evento que cambia una entidad la deja en `CambioVersion::tocados` con la versión que lo cerró
//! (`Nucleo::version_de`), una orden rechazada no deja marcas, lo que una orden crea sale con su clase en el contrato (`ResultadoOrden.creadas`) y el rechazo
//! `guerra.confirmar` lleva su `detalle`. Con `Ajustes::auditar_cambios` el propio núcleo comprueba, en cada versión, que ninguna entidad cambió sin quedar marcada:
//! los dos primeros escenarios corren con la auditoría puesta (el segundo, sobre el artefacto de mundo real; se omite si no está). La auditoría de toda la suite:
//! `BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo`.

mod comun;

use asentamiento::Recurso::*;
use base::{Punto, MS_POR_MINUTO};
use comun::*;
use contrato::v1 as pb;
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::sesiones::resultado_orden;
use nucleo::{rechazos, Ajustes, Arranque, Clase, Comando, Detalle, Nucleo, ObjetivoAtaque, Peticion, Relacion, Reglas, Resultado, TipoCreado};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;

fn auditado() -> Nucleo {
    Nucleo::nuevo(Arranque { ajustes: Ajustes { auditar_cambios: true, ..Ajustes::default() }, ..arranque(reticula(40, 5)) })
}

fn admin(id: &str, comando: Comando) -> Peticion {
    Peticion::nueva("admin", id, comando).de_admin()
}

/// Una columna de la cuenta en `p`: devuelve (columna, héroe).
fn columna(n: &mut Nucleo, cuenta: &str, faccion: u32, p: Punto, viveres: i64) -> (u64, u64) {
    let id = n.crear_ejercito(n.ahora(), cuenta, faccion, p, 500, viveres).unwrap();
    (id, n.ejercito(id).unwrap().columna.lider)
}

fn dar(n: &mut Nucleo, id: &str, heroe: u64, tropa: Tropa) -> u64 {
    let cantidad = militar::Reglas::cargar().tropas.tropa(tropa).unidades;
    let r = n.aplicar_en(n.ahora(), admin(id, Comando::DarEscuadra { heroe, tropa, cantidad }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

/// ¿Está la entidad entre lo tocado en la versión `v`?
fn tocado_en(n: &Nucleo, v: u64, clase: Clase, id: u64) -> bool {
    n.cambios_desde(v - 1).unwrap().next().is_some_and(|c| c.version == v && c.tocados.contiene(clase, id))
}

// ───────────────────────────── 1. Columnas, héroes y escuadras ─────────────────────────────

#[test]
fn un_combate_con_hambre_y_heridas_no_cambia_nada_sin_dejarlo_en_el_registro() {
    let mut n = auditado();
    let ana = columna(&mut n, "ana", 1, c(10, 2), 350_000);
    let beto = columna(&mut n, "beto", 2, Punto::new(c(10, 2).x + 2_000, c(10, 2).y), 3_000);
    let gus = columna(&mut n, "gus", 3, c(30, 2), 100);
    let fuerte = dar(&mut n, "d1", ana.1, Tropa::EspadachinesBronce);
    let debil = dar(&mut n, "d2", beto.1, Tropa::MiliciaLanceros);
    dar(&mut n, "d3", gus.1, Tropa::MiliciaLanceros);
    assert!(n.aplicar_en(n.ahora(), admin("guerra", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) })).aceptada());
    assert!(tocado_en(&n, n.version(), Clase::Faccion, 1) && tocado_en(&n, n.version(), Clase::Faccion, 2), "la relación toca a las dos Facciones");

    // El combate toca al derrotado (herido, carro, escuadra), al vencedor (escuadras con bajas) y a las dos columnas.
    let r = n.aplicar_en(10 * MIN, Peticion::nueva("ana", "ataque", Comando::Atacar { heroe: ana.1, objetivo: ObjetivoAtaque::Ejercito(beto.0), declara_guerra: false }));
    assert!(r.aceptada(), "{r:?}");
    let v = n.version();
    for (clase, id) in [(Clase::Heroe, beto.1), (Clase::Heroe, ana.1), (Clase::Escuadra, debil), (Clase::Escuadra, fuerte)] {
        assert!(tocado_en(&n, v, clase, id), "{clase:?} {id} debía quedar en la versión {v}");
        assert_eq!(n.version_de(clase, id), v, "{clase:?} {id}");
    }
    assert!(!tocado_en(&n, v, Clase::Heroe, gus.1), "la columna lejana no se toca");

    // Horas después, las columnas sin víveres desertan: tocan su héroe y su escuadra sin que nadie dé ninguna orden.
    n.avanzar_hasta(6 * HORA);
    let dato = |e: &nucleo::EventoNucleo, k: &str| e.datos.iter().find(|(c, _)| *c == k).map(|(_, v)| *v as u64).unwrap();
    let mut desertores = 0;
    for c in n.cambios_desde(v).unwrap() {
        for e in c.eventos.iter().filter(|e| e.codigo == "tropas.desercion") {
            desertores += 1;
            assert!(c.tocados.contiene(Clase::Heroe, dato(e, "heroe")) && c.tocados.contiene(Clase::Escuadra, dato(e, "escuadra")), "versión {}: {:?}", c.version, c.tocados);
        }
    }
    assert!(desertores > 0, "las escuadras sin víveres desertan");
    assert!(n.version_de(Clase::Heroe, gus.1) > v);

    // Las órdenes de administración sobre el héroe lo marcan; las que no cambian nada, no.
    let antes = n.version();
    assert!(n.aplicar_en(6 * HORA, admin("hiere", Comando::HerirHeroe { heroe: gus.1 })).aceptada());
    assert!(tocado_en(&n, n.version(), Clase::Heroe, gus.1) && n.version() == antes + 1);
}

#[test]
fn una_orden_rechazada_no_deja_marcas_aunque_pasara_por_la_entidad() {
    let mut n = auditado();
    let ana = columna(&mut n, "ana", 1, c(10, 2), 350_000);
    n.avanzar_hasta(MIN);
    let v = n.version();
    // `CancelarBatalla` pide la batalla antes de comprobar nada más: la marca y rechaza.
    let r = n.aplicar_en(MIN, Peticion::nueva("ana", "no-existe", Comando::CancelarBatalla { heroe: ana.1, batalla: 999 }));
    assert!(matches!(r.resultado, Resultado::Rechazada { codigo: rechazos::BATALLA_NO_EXISTE, .. }), "{r:?}");
    assert_eq!(n.version(), v, "una orden rechazada no sube la versión");
    // Y la marca no se queda esperando a la versión siguiente.
    let otra = columna(&mut n, "beto", 2, c(20, 2), 350_000);
    assert!(n.aplicar_en(MIN, Peticion::nueva("beto", "m", Comando::MarcharA { ejercito: otra.0, destino: c(25, 2) })).aceptada());
    assert!(n.cambios_desde(v).unwrap().all(|c| !c.tocados.contiene(Clase::Batalla, 999)), "la batalla inexistente no entra en ningún registro");
}

// ───────────────────────────── 2. Plazas, comercio, convocatorias ─────────────────────────────

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

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// Un sitio verde con mucho bosque cerca del campamento `i`: una plaza que no cae en ruinas por falta de madera.
fn sitio_rico(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let c = campamento(m, i);
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

struct Mundial {
    n: Nucleo,
    cuenta: u64,
}

impl Mundial {
    fn ok(&mut self, actor: &str, c: Comando) -> nucleo::Respuesta {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c.clone());
        // Andamiaje (5E): una orden de plaza de quien mandaba en ella antes del sistema 5 se da como administración (`comun::mandaba`).
        let (t, p) = (self.n.ahora(), if actor == "admin" || comun::mandaba(&self.n, actor, &p.comando) { p.de_admin() } else { p });
        let r = self.n.aplicar_en(t, p);
        assert!(r.aceptada(), "{c:?}: {r:?}");
        r
    }
    fn fundar(&mut self, dueno: &str, faccion: u32, p: Punto) -> u64 {
        let e = self.n.crear_ejercito(self.n.ahora(), dueno, faccion, p, 500, 100_000).unwrap();
        self.ok("admin", Comando::DarCaravanaFundacion { ejercito: e });
        self.ok(dueno, Comando::Fundar { ejercito: e }).creadas[0]
    }
    /// Un héroe de la cuenta, residente y presente en la plaza, ciudadano de la Facción 1.
    fn residente(&mut self, cuenta: &str, plaza: u64) -> u64 {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
        let a = plaza as u32;
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a) });
        h
    }
}

#[test]
fn la_economia_el_comercio_y_las_convocatorias_dejan_sus_entidades_en_el_registro() {
    let Some(m) = mundo() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    let ajustes = Ajustes { auditar_cambios: true, ..Ajustes::default() };
    let mut x = Mundial { n: Nucleo::nuevo(Arranque { reglas: Reglas::default(), ajustes, semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }), cuenta: 0 };
    let pa = sitio_rico(&x.n, &m.0, 0, 1);
    let a = x.fundar("ana", 1, pa);
    x.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
    let pb = sitio_rico(&x.n, &m.0, 0, 1);
    let b = x.fundar("ana", 1, pb);
    x.n.avanzar_hasta(24 * HORA);
    for plaza in [a, b] {
        for (r, q) in [(Madera, 400_000), (Piedra, 200_000), (Trigo, 300_000)] {
            // Un almacén lleno responde «sin cambios».
            x.cuenta += 1;
            x.n.aplicar_en(x.n.ahora(), admin(&format!("dar{}", x.cuenta), Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: q }));
        }
    }
    for plaza in [a, b] {
        x.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: asentamiento::catalogo::Tipo::Mercado });
    }
    x.n.avanzar_hasta(34 * HORA);
    let (h1, h2) = (x.residente("beto", a), x.residente("carla", a));
    assert!(x.n.version_de(Clase::Heroe, h2) > x.n.version_de(Clase::Heroe, h1), "cada héroe lleva la versión de su último cambio");

    // Una orden que crea algo lo marca y dice qué es.
    let r = x.ok("ana", Comando::CrearCaravana { asentamiento: a });
    let (caravana, v) = (r.creadas[0], x.n.version());
    assert_eq!(r.tipo_creadas, Some(TipoCreado::Caravana));
    assert!(tocado_en(&x.n, v, Clase::Caravana, caravana));
    assert_eq!(x.n.version_de(Clase::Caravana, caravana), v);
    // Un trueque y una orden de mercado, y lo que sigue a aceptarlos y cancelarlos.
    let tr = x.ok("ana", Comando::ProponerTrueque { a, b, lineas_a: vec![(Madera, 40_000)], lineas_b: vec![(Piedra, 20_000)] });
    assert_eq!(tr.tipo_creadas, Some(TipoCreado::Acuerdo));
    x.ok("ana", Comando::AceptarTrueque { acuerdo: tr.creadas[0] });
    assert!(tocado_en(&x.n, x.n.version(), Clase::Acuerdo, tr.creadas[0]));
    let orden = x.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 50_000, precio: None });
    assert_eq!(orden.tipo_creadas, Some(TipoCreado::OrdenMercado));
    x.ok("ana", Comando::CancelarOrdenMercado { orden: orden.creadas[0] });
    assert!(tocado_en(&x.n, x.n.version(), Clase::OrdenMercado, orden.creadas[0]));
    // El edificio encolado se anuncia con su plaza.
    let e = x.ok("ana", Comando::AnadirEdificio { asentamiento: a, tipo: asentamiento::catalogo::Tipo::Granja });
    assert_eq!(e.tipo_creadas, Some(TipoCreado::Edificio { plaza: a }));
    // Una convocatoria con dos héroes dentro de la plaza: se convoca, uno se une, parten y su ejército nace tocado.
    let conv = x.ok("beto", Comando::ConvocarEjercito { heroe: h1, escuadras: vec![], carga: vec![], politica: militar::columna::Politica::Aceptar });
    assert_eq!(conv.tipo_creadas, Some(TipoCreado::Convocatoria));
    x.ok("carla", Comando::UnirseAConvocatoria { heroe: h2, convocatoria: conv.creadas[0], escuadras: vec![], carga: vec![] });
    assert!(tocado_en(&x.n, x.n.version(), Clase::Convocatoria, conv.creadas[0]));
    let parte = x.ok("beto", Comando::PartirConvocatoria { heroe: h1, caravana_fundacion: None });
    assert_eq!(parte.tipo_creadas, Some(TipoCreado::Ejercito));
    assert!(tocado_en(&x.n, x.n.version(), Clase::Convocatoria, conv.creadas[0]), "la convocatoria que parte desaparece: queda tocada para retirarla");
    assert!(x.n.estado().militar.convocatorias.is_empty());
    // Y pasa el tiempo (preparación, paso de minuto, reposición de mercenarios...) con la auditoría puesta.
    x.n.avanzar_hasta(3 * 24 * HORA);
}

// ───────────────────────────── 3. Lo que ve el contrato ─────────────────────────────

fn sin_batallas(_: u64) -> String {
    unreachable!("este caso no nombra batallas")
}

#[test]
fn lo_que_crea_una_orden_sale_con_su_clase_y_su_id_del_contrato() {
    let mut n = nucleo(10, 10);
    let r = n.aplicar_en(0, admin("e", Comando::CrearEjercito { cuenta: "ana".into(), faccion: 1, posicion: c(2, 2), velocidad: 500, viveres: 350_000 }));
    let (ejercito, heroe) = (r.creadas[0], n.ejercito(r.creadas[0]).unwrap().columna.lider);
    let creadas = resultado_orden("p1", &r, &sin_batallas).creadas;
    assert_eq!(creadas, vec![pb::ReferenciaCreada { entidad: pb::TipoEntidad::Ejercito as i32, id: format!("ej-{ejercito}") }]);
    let r = n.aplicar_en(0, admin("d", Comando::DarEscuadra { heroe, tropa: Tropa::MiliciaLanceros, cantidad: 10 }));
    let esc = r.creadas[0];
    assert_eq!(resultado_orden("p2", &r, &sin_batallas).creadas, vec![pb::ReferenciaCreada { entidad: pb::TipoEntidad::Escuadron as i32, id: format!("esc-{esc}") }]);
    // Una orden que no crea nada no lleva clase ni referencias, y una duplicada tampoco repite lo creado.
    let r = n.aplicar_en(0, Peticion::nueva("ana", "m", Comando::MarcharA { ejercito, destino: c(5, 5) }));
    assert!(r.tipo_creadas.is_none() && resultado_orden("p3", &r, &sin_batallas).creadas.is_empty());
    let duplicada = n.aplicar_en(0, Peticion::nueva("ana", "m", Comando::MarcharA { ejercito, destino: c(5, 5) }));
    assert!(duplicada.creadas.is_empty() && duplicada.tipo_creadas.is_none());
}

#[test]
fn atacar_sin_guerra_pide_confirmar_con_el_detalle_de_contra_quien() {
    let mut n = nucleo(10, 10);
    let ana = columna(&mut n, "ana", 1, c(5, 5), 350_000);
    let beto = columna(&mut n, "beto", 3, Punto::new(c(5, 5).x + 1_000, c(5, 5).y), 350_000);
    let ataque = |guerra| Peticion::nueva("ana", if guerra { "g" } else { "s" }, Comando::Atacar { heroe: ana.1, objetivo: ObjetivoAtaque::Ejercito(beto.0), declara_guerra: guerra });
    let r = n.aplicar_en(0, ataque(false));
    assert!(matches!(&r.resultado, Resultado::Rechazada { codigo: rechazos::GUERRA_CONFIRMAR, detalle: Some(Detalle::Guerra { objetivo: 3, arrastre }), .. } if arrastre.is_empty()), "{r:?}");
    let rechazo = resultado_orden("p", &r, &sin_batallas).rechazo.expect("rechazada");
    assert_eq!(rechazo.codigo, "guerra.confirmar");
    let detalle = rechazo.detalle.expect("lleva detalle");
    let campo = |k: &str| detalle.fields[k].kind.clone().unwrap();
    assert_eq!(campo("objetivo"), prost_types::value::Kind::StringValue("f-3".into()));
    assert!(matches!(campo("facciones"), prost_types::value::Kind::ListValue(l) if l.values.is_empty()), "el arrastre queda vacío hasta el sistema político");
    // Otros rechazos no llevan detalle.
    let r = n.aplicar_en(0, Peticion::nueva("beto", "x", Comando::MarcharA { ejercito: ana.0, destino: c(6, 5) }));
    assert!(resultado_orden("q", &r, &sin_batallas).rechazo.expect("no es suya").detalle.is_none());
    // Con la declaración la orden pasa.
    assert!(n.aplicar_en(0, ataque(true)).aceptada());
}
