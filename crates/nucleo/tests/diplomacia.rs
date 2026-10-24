//! Las relaciones entre Facciones en el núcleo (sistema 5, tanda 5D; Diplomacia_Facciones §5): proponer, aceptar, rechazar y retirar una alianza o un vasallaje (quién puede, una sola
//! pendiente, la caducidad), declarar la guerra con el arrastre por vasallaje (por la orden y al atacar), la traición al aliado (Q2), la paz, romper, la rebelión, la reputación que mueven,
//! el tributo en el paso de minuto (Q3), el desarme del señor y la vasalla conquistada, reabastecer a los aliados y admitir otras Facciones, la foto `BAN32` y el determinismo.
//! Casos de `typescript-final:src/session/__tests__/comandosDiplomacia.test.ts`, `engine/__tests__/{eventosDominioDiplomacia,reputacionVasallos,vasallaConquistada}.test.ts`.
//! Las pruebas con plazas usan el artefacto de mundo real y se omiten si no está.

mod comun;

use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::datos::PUNTO;
use comun::{c, nucleo};
use militar::catalogo::Tropa;
use nucleo::{rechazos, Comando, Detalle, Estado, Nucleo, ObjetivoAtaque, Peticion, Reglas, Relacion, Respuesta, Resultado};

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

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

/// Un núcleo con su contador de peticiones.
struct E {
    n: Nucleo,
    k: u64,
}

impl E {
    fn de(n: Nucleo) -> Self {
        E { n, k: 0 }
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
    /// Un rechazo que no deja rastro: ni versión, ni hash, ni ids gastados.
    fn no(&mut self, actor: &str, c: Comando) -> &'static str {
        let antes = (self.n.version(), self.n.hash_estado(), self.n.estado().siguiente_id);
        let r = self.ord(actor, c);
        assert_eq!((self.n.version(), self.n.hash_estado(), self.n.estado().siguiente_id), antes, "el rechazo {r:?} dejó rastro");
        codigo(&r)
    }
    fn avanzar(&mut self, ms: Instante) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    /// La columna de la cuenta (su héroe de campo es ciudadano de la Facción y, si es el primero, su Rey): (columna, héroe).
    fn columna(&mut self, cuenta: &str, faccion: u32, p: Punto) -> (u64, u64) {
        let t = self.n.ahora();
        let id = self.n.crear_ejercito(t, cuenta, faccion, p, 500, 350_000).unwrap();
        (id, self.n.ejercito(id).unwrap().columna.lider)
    }
    fn rey(&self, f: u32) -> u64 {
        self.n.faccion(f).unwrap().rey.unwrap()
    }
    fn hechos(&self, codigo: &str) -> Vec<nucleo::EventoNucleo> {
        self.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).cloned().collect()
    }
    fn alianza(&mut self, cuenta_a: &str, a: u32, cuenta_b: &str, b: u32) -> u64 {
        let (ra, rb) = (self.rey(a), self.rey(b));
        let p = self.ok(cuenta_a, Comando::ProponerRelacion { heroe: ra, a, b, vasallaje: false, tributo: None }).creadas[0];
        self.ok(cuenta_b, Comando::ResponderRelacion { heroe: rb, propuesta: p, aceptar: true });
        self.n.relacion_activa(a, b).unwrap().id
    }
}

fn dato(e: &nucleo::EventoNucleo, k: &str) -> i64 {
    e.dato(k).unwrap_or_else(|| panic!("{} no lleva «{k}»", e.codigo))
}

/// Tres Facciones con una columna cada una: ana (F1), beto (F2) y gus (F3); sus héroes de campo son sus Reyes.
fn tres() -> E {
    let mut e = E::de(nucleo(40, 5));
    for (cuenta, f, x) in [("ana", 1, 10), ("beto", 2, 20), ("gus", 3, 30)] {
        e.columna(cuenta, f, c(x, 2));
    }
    e
}

// ───────────────────────────── 1. Propuestas ─────────────────────────────

#[test]
fn una_alianza_se_propone_y_se_acepta_por_los_reyes_y_una_sola_pendiente_por_par() {
    let mut e = tres();
    let (ra, rb) = (e.rey(1), e.rey(2));
    // Solo la autoridad diplomática de A (Rey o Embajador); una alianza no lleva tributo.
    let propone = |heroe: u64| Comando::ProponerRelacion { heroe, a: 1, b: 2, vasallaje: false, tributo: None };
    assert_eq!(e.no("beto", propone(rb)), rechazos::NO_AUTORIZADO, "el Rey de F2 no habla por F1");
    assert_eq!(e.no("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 2, vasallaje: false, tributo: Some((asentamiento::Recurso::Trigo, 1_000)) }), rechazos::DIPLOMACIA_TRIBUTO_INVALIDO);
    assert_eq!(e.no("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 1, vasallaje: false, tributo: None }), rechazos::RELACION_CONSIGO_MISMA);
    assert_eq!(e.no("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 9, vasallaje: false, tributo: None }), rechazos::FACCION_NO_EXISTE);
    let p = e.ok("ana", propone(ra));
    assert_eq!(p.tipo_creadas, Some(nucleo::TipoCreado::Propuesta));
    let propuesta = p.creadas[0];
    assert_eq!(e.n.propuesta(propuesta).unwrap().expira_en, e.n.ahora() + 3 * DIA, "RELACION_PROPUESTA.caducidadDias");
    let h = &e.hechos("diplomacia.relacion_propuesta")[0];
    assert_eq!((dato(h, "propuesta"), dato(h, "faccionA"), dato(h, "faccionB"), dato(h, "tipo"), h.facciones.clone()), (propuesta as i64, 1, 2, 0, vec![1, 2]));
    // Una sola pendiente por par, en cualquier sentido.
    assert_eq!(e.no("beto", Comando::ProponerRelacion { heroe: rb, a: 2, b: 1, vasallaje: true, tributo: None }), rechazos::DIPLOMACIA_YA_PENDIENTE);
    // Contesta el Rey de B, nadie más.
    assert_eq!(e.no("ana", Comando::ResponderRelacion { heroe: ra, propuesta, aceptar: true }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.no("beto", Comando::ResponderRelacion { heroe: rb, propuesta: 9_999, aceptar: true }), rechazos::DIPLOMACIA_PROPUESTA_DESCONOCIDA);
    e.ok("beto", Comando::ResponderRelacion { heroe: rb, propuesta, aceptar: true });
    let r = e.n.relacion_activa(2, 1).unwrap().clone();
    assert_eq!((r.a, r.b, r.tipo, r.creada_en), (1, 2, politica::relaciones::TipoRelacion::Alianza, e.n.ahora()));
    assert_eq!((e.n.relacion(1, 2), e.n.propuesta(propuesta)), (Some(Relacion::Alianza), None), "la propuesta se quita");
    assert_eq!(dato(&e.hechos("diplomacia.relacion_aceptada")[0], "relacion"), r.id as i64);
    // Las alianzas activas entran en el ritmo de la reputación de las dos.
    assert_eq!(e.n.estado().comercio.reputacion[&1].alianzas(), 1);
    // Con una relación activa ya no se propone otra.
    assert_eq!(e.no("ana", propone(ra)), rechazos::DIPLOMACIA_YA_HAY_RELACION);
}

#[test]
fn una_propuesta_se_rechaza_se_retira_y_caduca_sin_dejar_nada() {
    let mut e = tres();
    let (ra, rb, rg) = (e.rey(1), e.rey(2), e.rey(3));
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 2, vasallaje: true, tributo: Some((asentamiento::Recurso::Madera, 4_000)) }).creadas[0];
    e.ok("beto", Comando::ResponderRelacion { heroe: rb, propuesta: p, aceptar: false });
    assert_eq!((e.n.relacion(1, 2), e.hechos("diplomacia.relacion_rechazada").len()), (None, 1));
    // Retirar: la autoridad de quien propuso, mientras siga vigente.
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 3, vasallaje: false, tributo: None }).creadas[0];
    assert_eq!(e.no("gus", Comando::RetirarRelacion { heroe: rg, propuesta: p }), rechazos::NO_AUTORIZADO);
    e.ok("ana", Comando::RetirarRelacion { heroe: ra, propuesta: p });
    assert_eq!(e.no("gus", Comando::ResponderRelacion { heroe: rg, propuesta: p, aceptar: true }), rechazos::DIPLOMACIA_PROPUESTA_DESCONOCIDA);
    // Caducada: no se contesta, y no hay ningún evento esperando en la cola por ella.
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 3, vasallaje: false, tributo: None }).creadas[0];
    e.avanzar(3 * DIA);
    assert_eq!(e.no("gus", Comando::ResponderRelacion { heroe: rg, propuesta: p, aceptar: true }), rechazos::DIPLOMACIA_PROPUESTA_CADUCADA);
    assert_eq!(e.hechos("diplomacia.relacion_retirada").len(), 1);
}

// ───────────────────────────── 2. La guerra ─────────────────────────────

#[test]
fn declarar_la_guerra_arrastra_a_la_senora_y_a_las_vasallas_del_objetivo() {
    let mut e = tres();
    e.columna("dora", 4, c(35, 2));
    // F2 señora de F3 y de F4.
    for v in [3, 4] {
        e.ok("admin", Comando::FijarRelacion { a: 2, b: v, relacion: Some(Relacion::Vasallaje) });
    }
    let ra = e.rey(1);
    assert_eq!(e.no("beto", Comando::DeclararGuerra { heroe: e.rey(2), a: 1, b: 3 }), rechazos::NO_AUTORIZADO);
    e.ok("ana", Comando::DeclararGuerra { heroe: ra, a: 1, b: 3 });
    assert!([2, 3, 4].iter().all(|&f| e.n.relacion(1, f) == Some(Relacion::Guerra)), "la vasalla, su señora y la otra vasalla");
    assert_eq!(e.n.relacion_activa(1, 3).unwrap().a, 1, "A es quien declara");
    let g = &e.hechos("diplomacia.guerra_declarada")[0];
    assert_eq!((dato(g, "atacante"), dato(g, "contra"), dato(g, "traicion"), g.facciones.clone()), (1, 3, 0, vec![1, 2, 3, 4]));
    let tipado = nucleo::proyeccion::eventos::evento_dominio(&e.n, g, 1);
    let Some(contrato::v1::evento_dominio::Tipado::GuerraDeclarada(t)) = tipado.tipado else { panic!("{tipado:?}") };
    assert_eq!(t.arrastradas_ids, ["f-2", "f-4"]);
    // Con relación activa ya no se declara; contra la señora tampoco hay arrastre que repetir.
    assert_eq!(e.no("ana", Comando::DeclararGuerra { heroe: ra, a: 1, b: 2 }), rechazos::DIPLOMACIA_YA_HAY_RELACION);
}

#[test]
fn atacar_sin_guerra_confirma_con_el_arrastre_y_a_un_aliado_es_una_traicion_que_rompe_la_alianza_y_cuesta_25() {
    let mut e = tres();
    let ra = e.rey(1);
    // beto viene junto a ana; F2 es señora de F3.
    let beto_cerca = e.columna("beto", 2, Punto::new(c(10, 2).x + 2_000, c(10, 2).y));
    e.ok("admin", Comando::FijarRelacion { a: 2, b: 3, relacion: Some(Relacion::Vasallaje) });
    e.ok("admin", Comando::DarEscuadra { heroe: ra, tropa: Tropa::ArquerosCompuesto, cantidad: militar::Reglas::cargar().tropas.tropa(Tropa::ArquerosCompuesto).unidades });
    let alianza = e.alianza("ana", 1, "beto", 2);
    let atacar = |g: bool| Comando::Atacar { heroe: ra, objetivo: ObjetivoAtaque::Ejercito(beto_cerca.0), declara_guerra: g };
    let r = e.ord("ana", atacar(false));
    assert_eq!(codigo(&r), rechazos::GUERRA_CONFIRMAR);
    let Resultado::Rechazada { detalle: Some(Detalle::Guerra { objetivo, arrastre }), mensaje, .. } = &r.resultado else { panic!("{r:?}") };
    assert_eq!((*objetivo, arrastre.clone()), (2, vec![3]), "entraría su vasalla");
    assert!(mensaje.contains("traición"), "{mensaje}");
    // La traición: la alianza se rompe en el acto, −25 (no −12) y la guerra con su arrastre.
    let t = e.n.ahora();
    e.ok("ana", atacar(true));
    let rota = e.n.relacion_politica(alianza).unwrap();
    assert_eq!((rota.activa(), rota.rota_por, rota.rota_en), (false, Some(1), Some(t)));
    assert_eq!((e.n.relacion(1, 2), e.n.relacion(1, 3)), (Some(Relacion::Guerra), Some(Relacion::Guerra)));
    assert_eq!(e.n.reputacion(1), -25 * PUNTO, "solo −25 (5A-1)");
    let g = &e.hechos("diplomacia.guerra_declarada")[0];
    assert_eq!((dato(g, "traicion"), dato(&e.hechos("diplomacia.relacion_rota")[0], "faccion")), (1, 1));
    assert_eq!(e.n.estado().comercio.reputacion[&2].alianzas(), 0, "la alianza ya no suma");
}

#[test]
fn la_paz_es_mutua_y_tras_romper_una_alianza_volver_a_la_guerra_cuesta_dentro_de_la_ventana() {
    let mut e = tres();
    let (ra, rb) = (e.rey(1), e.rey(2));
    let alianza = e.alianza("ana", 1, "beto", 2);
    // Romper una alianza: cualquiera de las dos, −12 a quien rompe; una guerra no se rompe.
    assert_eq!(e.no("beto", Comando::RomperRelacion { heroe: rb, relacion: alianza, iniciador: 1 }), rechazos::NO_AUTORIZADO);
    e.ok("ana", Comando::RomperRelacion { heroe: ra, relacion: alianza, iniciador: 1 });
    assert_eq!((e.n.relacion(1, 2), e.n.reputacion(1), e.n.reputacion(2)), (None, -12 * PUNTO, 0));
    assert_eq!(e.no("ana", Comando::RomperRelacion { heroe: ra, relacion: alianza, iniciador: 1 }), rechazos::DIPLOMACIA_RELACION_ROTA, "5A-6");
    // Declararle la guerra dentro de la ventana cuesta otros −15.
    e.avanzar(MIN);
    let antes = e.n.reputacion(1);
    e.ok("ana", Comando::DeclararGuerra { heroe: ra, a: 1, b: 2 });
    assert_eq!(e.n.reputacion(1) - antes, -15 * PUNTO);
    let guerra = e.n.relacion_activa(1, 2).unwrap().id;
    assert_eq!(e.no("ana", Comando::RomperRelacion { heroe: ra, relacion: guerra, iniciador: 1 }), rechazos::DIPLOMACIA_GUERRA_NO_SE_ROMPE);
    // La paz: la ofrece una, la misma no repite, la firma la otra.
    e.ok("ana", Comando::ProponerPaz { heroe: ra, relacion: guerra, faccion: 1 });
    assert_eq!(e.no("ana", Comando::ProponerPaz { heroe: ra, relacion: guerra, faccion: 1 }), rechazos::DIPLOMACIA_YA_OFRECIO_PAZ);
    assert_eq!(e.n.relacion_activa(1, 2).unwrap().paz_propuesta_por, Some(1));
    e.ok("beto", Comando::ProponerPaz { heroe: rb, relacion: guerra, faccion: 2 });
    assert_eq!((e.n.relacion(1, 2), e.hechos("diplomacia.paz_propuesta").len(), e.hechos("diplomacia.paz_firmada").len()), (None, 1, 1));
}

#[test]
fn la_senora_libera_y_la_vasalla_se_rebela_con_la_guerra_y_su_reputacion() {
    let mut e = tres();
    let (ra, rb, rg) = (e.rey(1), e.rey(2), e.rey(3));
    // F1 señora de F2 y de F3 por propuesta aceptada.
    for (cuenta, f, rey) in [("beto", 2, rb), ("gus", 3, rg)] {
        let p = e.ok("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: f, vasallaje: true, tributo: None }).creadas[0];
        e.ok(cuenta, Comando::ResponderRelacion { heroe: rey, propuesta: p, aceptar: true });
    }
    let (v2, v3) = (e.n.relacion_activa(1, 2).unwrap().id, e.n.relacion_activa(1, 3).unwrap().id);
    // La vasalla no libera; la señora sí, y gana reputación.
    assert_eq!(e.no("beto", Comando::RomperRelacion { heroe: rb, relacion: v2, iniciador: 2 }), rechazos::DIPLOMACIA_SOLO_LA_SENORA);
    e.ok("ana", Comando::RomperRelacion { heroe: ra, relacion: v2, iniciador: 1 });
    assert_eq!(e.n.reputacion(1), 6 * PUNTO);
    // La rebelión: la autoridad de la vasalla; el vasallaje se rompe, −10 a la señora y la vasalla le declara la guerra.
    assert_eq!(e.no("ana", Comando::RebelionVasallo { heroe: ra, relacion: v3 }), rechazos::NO_AUTORIZADO);
    let antes = e.n.reputacion(1);
    e.ok("gus", Comando::RebelionVasallo { heroe: rg, relacion: v3 });
    assert_eq!((e.n.relacion(1, 3), e.n.relacion_activa(1, 3).unwrap().a), (Some(Relacion::Guerra), 3));
    assert_eq!(e.n.reputacion(1) - antes, -10 * PUNTO);
    let h = &e.hechos("diplomacia.rebelion_vasallo")[0];
    assert_eq!((dato(h, "faccionA"), dato(h, "faccionB")), (1, 3));
    assert_eq!(e.no("gus", Comando::RebelionVasallo { heroe: rg, relacion: v3 }), rechazos::DIPLOMACIA_RELACION_ROTA);
}

#[test]
fn el_rey_admite_otras_facciones_en_sus_ataques() {
    let mut e = tres();
    let (ra, rb) = (e.rey(1), e.rey(2));
    assert_eq!(e.no("beto", Comando::AdmitirOtrasFacciones { heroe: rb, faccion: 1, admitir: true }), rechazos::NO_AUTORIZADO);
    e.ok("ana", Comando::AdmitirOtrasFacciones { heroe: ra, faccion: 1, admitir: true });
    assert!(e.n.faccion(1).unwrap().admite_otras_en_ataques);
    assert_eq!(e.ord("ana", Comando::AdmitirOtrasFacciones { heroe: ra, faccion: 1, admitir: true }).resultado, Resultado::SinCambios);
    assert_eq!(dato(&e.hechos("faccion.admision_en_ataques")[0], "admite"), 1);
}

// ───────────────────────────── 3. Con plazas: el tributo, la caída y los aliados ─────────────────────────────

/// Dos Facciones con una plaza cada una cerca del campamento 0 (F1 en A, F2 en B), sin bandidos que molesten.
fn dos_plazas(m: &comun::real::M) -> (E, u64, u64) {
    let mut reglas = Reglas::default();
    let b = &mut std::sync::Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let arranque = nucleo::Arranque { reglas, ajustes: nucleo::Ajustes::default(), semilla: 5, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let mut e = E::de(Nucleo::nuevo(arranque));
    let pa = comun::real::sitio_rico(&e.n, &m.0, 0, 1);
    let a = comun::real::fundar(&mut e.n, "ana", 1, pa);
    let pb = comun::real::sitio_rico(&e.n, &m.0, 0, 2);
    let b = comun::real::fundar(&mut e.n, "beto", 2, pb);
    (e, a, b)
}

fn cantidad(e: &E, plaza: u64, r: asentamiento::Recurso) -> i64 {
    e.n.estado().asentamientos[&plaza].economia.almacen.cantidad(r)
}

#[test]
fn la_vasalla_paga_su_tributo_en_su_paso_de_minuto_en_enteros_y_lo_que_no_cabe_en_la_capital_se_pierde() {
    let m = con_mundo!();
    let (mut e, a, b) = dos_plazas(m);
    use asentamiento::Recurso::Oro;
    // F1 señora de F2 con 1,5 de oro por minuto: con una plaza, paga 1, 2, 1, 2... (resto oculto).
    let (ra, rb) = (e.rey(1), e.rey(2));
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 2, vasallaje: true, tributo: Some((Oro, 1_500)) }).creadas[0];
    e.ok("beto", Comando::ResponderRelacion { heroe: rb, propuesta: p, aceptar: true });
    e.ok("admin", Comando::DarRecursos { asentamiento: b, recurso: Oro, cantidad: 50_000 });
    let (oro_a, oro_b) = (cantidad(&e, a, Oro), cantidad(&e, b, Oro));
    e.avanzar(4 * MIN);
    let pagos: Vec<(i64, i64)> = e.hechos("diplomacia.tributo_pagado").iter().map(|h| (dato(h, "pagadas"), dato(h, "recibidas"))).collect();
    assert_eq!(pagos.iter().map(|p| p.0).sum::<i64>(), 6, "1,5 × 4 minutos: {pagos:?}");
    assert!(pagos.iter().all(|&(p, r)| p == r && (1..=2).contains(&p)), "{pagos:?}");
    let h = &e.hechos("diplomacia.tributo_pagado")[0];
    assert_eq!((dato(h, "faccionA"), dato(h, "faccionB"), dato(h, "plaza"), dato(h, "capital"), h.asentamientos.clone()), (1, 2, b as i64, a as i64, vec![b, a]));
    // La economía de cada plaza también mueve el oro (recaudación): lo que importa es lo que cuentan los hechos y que todo es entero.
    assert_eq!(cantidad(&e, b, Oro) % 1_000 + cantidad(&e, a, Oro) % 1_000, 0);
    assert!(cantidad(&e, a, Oro) >= oro_a + 6_000 - 6_000 && cantidad(&e, b, Oro) <= oro_b);
    // La capital llena: se paga igual y se pierde (5A-2).
    let cap = e.n.estado().asentamientos[&a].economia.almacen.capacidad(Oro);
    e.ok("admin", Comando::DarRecursos { asentamiento: a, recurso: Oro, cantidad: cap });
    let n0 = e.hechos("diplomacia.tributo_pagado").len();
    e.avanzar(2 * MIN);
    let tras: Vec<(i64, i64)> = e.hechos("diplomacia.tributo_pagado")[n0..].iter().map(|h| (dato(h, "pagadas"), dato(h, "recibidas"))).collect();
    assert!(!tras.is_empty() && tras.iter().all(|&(p, r)| p > 0 && r == 0), "{tras:?}");
    // Sin vasallaje, nada (y ninguna cita nueva en la cola por el tributo: va en el paso de minuto).
    let v = e.n.relacion_activa(1, 2).unwrap().id;
    e.ok("ana", Comando::RomperRelacion { heroe: ra, relacion: v, iniciador: 1 });
    let n1 = e.hechos("diplomacia.tributo_pagado").len();
    e.avanzar(3 * MIN);
    assert_eq!(e.hechos("diplomacia.tributo_pagado").len(), n1);
}

/// Una columna fuerte de F3 asedia `plaza` (sin defensores, cae sin combate al acabar la convocatoria).
fn conquistar(e: &mut E, plaza: u64, cuenta: &str, f: u32) {
    let p = e.n.estado().asentamientos[&plaza].posicion;
    e.avanzar_hasta_que_se_pueda(plaza);
    let (_, h) = e.columna(cuenta, f, Punto::new(p.x + 2_000, p.y));
    let arqueros = Tropa::ArquerosCompuesto;
    e.ok("admin", Comando::DarEscuadra { heroe: h, tropa: arqueros, cantidad: militar::Reglas::cargar().tropas.tropa(arqueros).unidades });
    e.ok(cuenta, Comando::Atacar { heroe: h, objetivo: ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    e.avanzar(MIN);
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, f, "la plaza cayó");
}

impl E {
    fn avanzar_hasta_que_se_pueda(&mut self, plaza: u64) {
        if let Some(t) = self.n.protegida_hasta(plaza).filter(|&t| t > self.n.ahora()) {
            self.n.avanzar_hasta(t);
        }
    }
}

#[test]
fn al_caer_su_ultima_plaza_la_senora_libera_a_sus_vasallas_y_la_vasalla_pasa_a_quien_la_conquista() {
    let m = con_mundo!();
    // La vasalla conquistada: F2 vasalla de F1 con tributo; F3 conquista B, la única plaza de F2 (la guerra se arrastra a F1, que no defiende y paga −3).
    let (mut e, _a, b) = dos_plazas(m);
    let tributo = Some((asentamiento::Recurso::Madera, 2_000));
    let (ra, rb) = (e.rey(1), e.rey(2));
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 2, vasallaje: true, tributo }).creadas[0];
    e.ok("beto", Comando::ResponderRelacion { heroe: rb, propuesta: p, aceptar: true });
    let vieja = e.n.relacion_activa(1, 2).unwrap().id;
    conquistar(&mut e, b, "gus", 3);
    assert_eq!(e.n.faccion(2).unwrap().derrota, Some(politica::faccion::Derrota::Por(3)));
    assert!(!e.n.relacion_politica(vieja).unwrap().activa());
    let nueva = e.n.relacion_activa(3, 2).unwrap().clone();
    assert_eq!((nueva.tipo, nueva.a, nueva.b, nueva.tributo.map(|t| t.cantidad_por_minuto)), (politica::relaciones::TipoRelacion::Vasallaje, 3, 2, Some(2_000)), "con el mismo tributo");
    assert_eq!(e.n.relacion(1, 3), Some(Relacion::Guerra), "la guerra al asediar se arrastró a la señora");
    let h = &e.hechos("diplomacia.vasallo_conquistado")[0];
    assert_eq!((dato(h, "faccionA"), dato(h, "faccionB"), dato(h, "faccionAnterior")), (3, 2, 1));
    let castigo: Vec<i64> = e.hechos("reputacion.cambio").iter().filter(|h| dato(h, "faccion") == 1).map(|h| dato(h, "delta")).collect();
    assert_eq!(castigo, [-3 * PUNTO], "la señora no estuvo y ganó el atacante (§5.9)");

    // El desarme del señor: F1 señora de F2 (con plaza); F3 conquista A, la única de F1, y F2 queda libre.
    let (mut e, a, _b) = dos_plazas(m);
    let (ra, rb) = (e.rey(1), e.rey(2));
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 2, vasallaje: true, tributo: None }).creadas[0];
    e.ok("beto", Comando::ResponderRelacion { heroe: rb, propuesta: p, aceptar: true });
    conquistar(&mut e, a, "gus", 3);
    assert_eq!(e.n.relacion(1, 2), None, "liberada");
    let h = &e.hechos("diplomacia.vasallo_liberado")[0];
    assert_eq!((dato(h, "faccionA"), dato(h, "faccionB")), (1, 2));
}

/// `CargarCaravana` mira lo mismo (`Nucleo::abre_su_almacen_a`, la duda 3 de 5G-a cerrada en 5D).
#[test]
fn una_plaza_aliada_abre_su_almacen_para_repostar_si_lo_permite() {
    let m = con_mundo!();
    let (mut e, _a, b) = dos_plazas(m);
    e.alianza("ana", 1, "beto", 2);
    e.ok("admin", Comando::DarRecursos { asentamiento: b, recurso: asentamiento::Recurso::Trigo, cantidad: 2_000_000 });
    let pb = e.n.estado().asentamientos[&b].posicion;
    let t = e.n.ahora();
    let seca = e.n.crear_ejercito(t, "carla", 1, pb, 500, 50_000).unwrap();
    let lider = e.n.ejercito(seca).unwrap().columna.lider;
    let repostar = Comando::Repostar { heroe: lider, asentamiento: b };
    assert_eq!(codigo(&e.ord("carla", repostar.clone())), rechazos::REPOSTAR_PLAZA_CERRADA, "por defecto, solo la propia");
    // Solo quien manda en la plaza la abre; sin cambio, nada.
    assert_eq!(e.no("carla", Comando::AlternarReabastecerAliados { asentamiento: b, permitido: true }), rechazos::NO_AUTORIZADO);
    e.ok("beto", Comando::AlternarReabastecerAliados { asentamiento: b, permitido: true });
    assert_eq!(e.ord("beto", Comando::AlternarReabastecerAliados { asentamiento: b, permitido: true }).resultado, Resultado::SinCambios);
    assert_eq!(e.hechos("logistica.reabastecer_aliados").len(), 1);
    e.ok("carla", repostar.clone());
    assert_eq!(e.n.ejercito(seca).unwrap().columna.viveres.valor_en(e.n.ahora()), 350_000);
    // Una Facción neutral no reposta aunque esté abierta a los aliados.
    let t = e.n.ahora();
    let otra = e.n.crear_ejercito(t, "gus", 3, pb, 500, 50_000).unwrap();
    let lider_g = e.n.ejercito(otra).unwrap().columna.lider;
    assert_eq!(codigo(&e.ord("gus", Comando::Repostar { heroe: lider_g, asentamiento: b })), rechazos::REPOSTAR_PLAZA_CERRADA);
}

// ───────────────────────────── 4. Foto y determinismo ─────────────────────────────

fn guion(e: &mut E, paso: &dyn Fn(&mut E, Instante)) {
    let (ra, rb, rg) = (e.rey(1), e.rey(2), e.rey(3));
    let p = e.ok("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 2, vasallaje: true, tributo: Some((asentamiento::Recurso::Oro, 700)) }).creadas[0];
    paso(e, 3 * MIN + 17);
    e.ok("beto", Comando::ResponderRelacion { heroe: rb, propuesta: p, aceptar: true });
    paso(e, 5 * MIN);
    e.ok("gus", Comando::ProponerRelacion { heroe: rg, a: 3, b: 1, vasallaje: false, tributo: None });
    paso(e, 7 * MIN + 3);
    e.ok("gus", Comando::DeclararGuerra { heroe: rg, a: 3, b: 2 });
    paso(e, 11 * MIN);
}

#[test]
fn la_diplomacia_es_determinista_viaja_en_la_foto_ban32_y_nada_espera_en_la_cola() {
    let m = con_mundo!();
    let montar = || {
        let (mut e, _, b) = dos_plazas(m);
        let pc = comun::real::sitio_rico(&e.n, &m.0, 1, 3);
        comun::real::fundar(&mut e.n, "gus", 3, pc);
        e.ok("admin", Comando::DarRecursos { asentamiento: b, recurso: asentamiento::Recurso::Oro, cantidad: 30_000 });
        e
    };
    let mut normal = montar();
    guion(&mut normal, &|e, ms| e.avanzar(ms));
    let mut trozos = montar();
    guion(&mut trozos, &|e, ms| {
        for _ in 0..4 {
            e.avanzar(ms / 4);
        }
        e.avanzar(ms % 4);
    });
    assert_eq!((normal.n.hash_estado(), normal.n.version()), (trozos.n.hash_estado(), trozos.n.version()));
    assert!(!normal.hechos("diplomacia.tributo_pagado").is_empty() && normal.n.relacion(1, 2) == Some(Relacion::Vasallaje));
    assert_eq!((normal.n.relacion(3, 2), normal.n.relacion(3, 1)), (Some(Relacion::Guerra), Some(Relacion::Guerra)), "la guerra a la vasalla arrastra a su señora");
    // La foto lleva las relaciones con su historia, la propuesta pendiente, el resto del tributo y la apertura a los aliados, y cada pieza mueve el hash.
    let (reglas, hash) = (normal.n.estado().reglas.clone(), m.0.hash().to_string());
    let bytes = normal.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN32");
    let s = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
    assert_eq!((s.politica.clone(), s.hash()), (normal.n.estado().politica.clone(), normal.n.hash_estado()));
    assert!(s.politica.relaciones.iter().count() >= 3 && s.politica.propuestas.iter().count() == 1);
    let plaza = *s.asentamientos.keys().nth(1).unwrap();
    for cambio in [0, 1] {
        let mut otro = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
        let a = otro.asentamientos.get_mut(&plaza).unwrap();
        if cambio == 0 {
            a.resto_tributo += 1;
        } else {
            a.permite_reabastecer_aliados = !a.permite_reabastecer_aliados;
        }
        assert_ne!(otro.hash(), s.hash(), "{cambio}");
    }
    // Ninguna relación ni propuesta deja citas en la cola: la cola es la de la economía y la reputación.
    let citas: Vec<_> = normal.n.estado().cola.volcar().into_iter().filter(|ev| normal.n.estado().generaciones.es_vigente(ev.clave)).map(|ev| format!("{:?}", ev.carga)).collect();
    assert!(citas.iter().all(|c| !c.contains("Relacion") && !c.contains("Propuesta")), "{citas:?}");
}
