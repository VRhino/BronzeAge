//! Los Aedas itinerantes en el núcleo (sistema 6, tanda 6D; Tecnologia_Aedas §6). Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`; las pruebas se omiten si no
//! está). Portan los casos de `typescript-final:src/engine/__tests__/aedas.test.ts` que miran el mundo (cuántos nacen y dónde, el recorrido, la puerta, revelar y vender; las reglas puras de
//! cuántos y de los candidatos están en `tecnologia::itinerantes`, 6A) y la parte de la compra de `session/__tests__/comandosTecnologia.test.ts`; además, la puerta que se cierra con un Aeda
//! dentro (Q5 del autor), que ningún Aeda queda ocioso, la foto `BAN39` (hoy `BAN42`) y el determinismo.
//!
//! Dos plazas: A, de la Facción 1 («ana»), y B, de la Facción 2 («bea»). La metalurgia del cobre tiene en estas pruebas el logro `plazasEnNivel.2 ≥ 1` (lo cumple `FijarNivelAsentamiento`);
//! la descubre la Facción 1 por administración (`FijarTecnologias`), y desde entonces los Aedas la conocen a las 24 h.

mod comun;

use asentamiento::Recurso;
use base::{Instante, MS_POR_MINUTO};
use comun::real::{self, M};
use nucleo::{entidad_aeda, rechazos, Ajustes, Arranque, Carga, Comando, Dato, Estado, EventoNucleo, Nucleo, Peticion, Reglas, Resultado};
use politica::{Grupo, Grupos};
use tecnologia::datos::Logro;
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

/// Las reglas por defecto sin bandidos, con almacenes grandes y la metalurgia del cobre con el logro de nivel 2; `tec` cambia algo más del catálogo.
fn reglas_con(tec: impl FnOnce(&mut tecnologia::Reglas)) -> Reglas {
    let mut r = comun::reglas();
    let b = &mut std::sync::Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    std::sync::Arc::make_mut(&mut r.economia.datos).capacidad_inicial = 100_000_000;
    let t = std::sync::Arc::make_mut(&mut r.economia.tecnologia);
    let cobre = t.buscar("metalurgia_cobre").unwrap();
    t.catalogo[cobre as usize].logro = Some(Logro { contador: Contador::PlazasEnNivel2, umbral: 1 });
    tec(t);
    t.indexar();
    r
}

fn reglas() -> Reglas {
    reglas_con(|_| {})
}

fn arranque(m: &M, reglas: Reglas, ajustes: Ajustes) -> Arranque {
    Arranque { reglas, ajustes, semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

const SIN_AEDAS: [Grupo; 3] = [Grupo::Neutrales, Grupo::Enemigos, Grupo::Aedas];
const POR_DEFECTO: [Grupo; 2] = [Grupo::Neutrales, Grupo::Enemigos];

struct Escenario {
    n: Nucleo,
    /// La plaza de la Facción 1.
    a: u64,
    /// La plaza de la Facción 2.
    b: u64,
    k: u32,
}

impl Escenario {
    fn nuevo(m: &M, reglas: Reglas) -> Self {
        Self::en(Nucleo::nuevo(arranque(m, reglas, Ajustes::default())), m)
    }

    fn en(mut n: Nucleo, m: &M) -> Self {
        let pa = real::sitio_rico(&n, &m.0, 0, 1);
        let a = real::fundar(&mut n, "ana", 1, pa);
        let pb = real::sitio_rico(&n, &m.0, 1, 2);
        let b = real::fundar(&mut n, "bea", 2, pb);
        Escenario { n, a, b, k: 0 }
    }

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
        let (v, h) = (self.n.version(), self.n.hash_estado());
        match self.ord(actor, c).resultado {
            Resultado::Rechazada { codigo, .. } => {
                assert_eq!((self.n.version(), self.n.hash_estado()), (v, h), "un rechazo no deja rastro");
                codigo.to_string()
            }
            otro => panic!("se esperaba un rechazo: {otro:?}"),
        }
    }

    fn puerta(&mut self, plaza: u64, cerrada: &[Grupo]) {
        self.ok("admin", Comando::FijarPuerta { heroe: 0, asentamiento: plaza, cerrada_a: Grupos::de(cerrada.iter().copied()) });
    }

    fn tec(&self, id: &str) -> TecId {
        self.n.estado().reglas.economia.tecnologia.buscar(id).unwrap()
    }

    fn propias(&self, f: u32) -> tecnologia::TecnologiasFaccion {
        self.n.tecnologia().de(&self.n.estado().reglas.economia.tecnologia, f)
    }

    /// La Facción 1 cumple el logro del cobre y lo descubre ahora; devuelve el instante (los Aedas la conocen 24 h después).
    fn descubrir_cobre(&mut self) -> Instante {
        let a = self.a;
        self.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
        self.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["metalurgia_cobre".into()] });
        self.n.ahora()
    }

    /// Los Aedas parados en `plaza`.
    fn en_plaza(&self, plaza: u64) -> Vec<u32> {
        self.n.aedas().iter().filter(|a| a.en == Some(plaza as u32)).map(|a| a.id).collect()
    }

    /// El Rey de la Facción `f` (quien fundó su plaza), dentro de `plaza`.
    fn rey_dentro(&mut self, f: u32, plaza: u64) -> u64 {
        let rey = self.n.facciones().find(|x| x.id == f).and_then(|x| x.rey).unwrap();
        // Entra con su columna (se deshace dentro); si ya no va en una, la administración lo coloca.
        let entra = if self.n.heroe(rey).unwrap().columna.is_some() {
            Comando::Guarnecer { asentamiento: plaza, heroe: rey }
        } else {
            Comando::ColocarHeroe { heroe: rey, ubicacion: militar::presencia::Ubicacion::Asentamiento(plaza as u32) }
        };
        self.ok("admin", entra);
        rey
    }

    fn oro(&self, plaza: u64) -> i64 {
        self.n.economia(plaza).unwrap().almacen.cantidad(Recurso::Oro)
    }

    fn comprar(&self, heroe: u64, plaza: u64, id: &str) -> Comando {
        Comando::ComprarTecnologiaAeda { heroe, asentamiento: plaza, tecnologia: self.tec(id) }
    }
}

fn hechos(n: &Nucleo, codigo: &str) -> Vec<EventoNucleo> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).cloned().collect()
}

fn texto(h: &EventoNucleo, clave: &str) -> Option<String> {
    h.valores.iter().find(|(k, _)| *k == clave).and_then(|(_, v)| if let Dato::Texto(s) = v { Some(s.clone()) } else { None })
}

/// Cada Aeda tiene en la cola exactamente un evento vigente, el suyo (`Carga::Aeda`), en el instante de su siguiente evento: nada más, ni nada por un Aeda ocioso.
fn un_evento_por_aeda(n: &Nucleo) {
    let e = n.estado();
    for a in n.aedas() {
        let suyos: Vec<_> = e.cola.volcar().into_iter().filter(|x| x.clave.entidad == entidad_aeda(a.id) && e.generaciones.es_vigente(x.clave)).collect();
        assert_eq!(suyos.len(), 1, "aeda-{}: {suyos:?}", a.id);
        assert_eq!((suyos[0].carga.clone(), suyos[0].instante), (Carga::Aeda, a.siguiente_evento().unwrap().max(n.ahora())), "aeda-{}", a.id);
        assert!(a.en.is_some() != a.plan.is_some() || (a.en.is_none() && a.plan.is_none() && a.hasta.is_some()), "parado, de camino o varado esperando: aeda-{} {a:?}", a.id);
    }
}

// ───────────────────────────── 1. Cuántos y dónde nacen ─────────────────────────────

#[test]
fn nacen_el_minimo_con_la_primera_plaza_parados_en_ella_y_no_nacen_mas_con_la_segunda_faccion() {
    let m = con_mundo!();
    let mut n = Nucleo::nuevo(arranque(m, reglas(), Ajustes::default()));
    assert!(n.aedas().is_empty(), "sin plazas, ninguno");
    let pa = real::sitio_rico(&n, &m.0, 0, 1);
    let a = real::fundar(&mut n, "ana", 1, pa);
    let t = n.ahora();
    let estancia = n.estado().reglas.economia.tecnologia.aedas.estancia;
    let vistos: Vec<_> = n.aedas().iter().map(|x| (x.id, x.en, x.hasta, x.recientes.clone())).collect();
    assert_eq!(vistos, (1..=3).map(|i| (i, Some(a as u32), Some(t + estancia), vec![a as u32])).collect::<Vec<_>>(), "los tres, ya llegados a la única plaza");
    un_evento_por_aeda(&n);
    let pb = real::sitio_rico(&n, &m.0, 1, 2);
    real::fundar(&mut n, "bea", 2, pb);
    assert_eq!(n.aedas().len(), 3, "con dos Facciones el objetivo sigue siendo el mínimo");
    un_evento_por_aeda(&n);
}

// ───────────────────────────── 2. El recorrido ─────────────────────────────

#[test]
fn salen_de_su_plaza_tras_la_estancia_y_llegan_a_la_otra_por_la_red_sin_quedar_nunca_ociosos() {
    let m = con_mundo!();
    let mut e = Escenario::nuevo(m, reglas());
    let (a, b) = (e.a as u32, e.b as u32);
    let estancia = e.n.estado().reglas.economia.tecnologia.aedas.estancia;
    let mut salio = false;
    for h in 1..=72 {
        e.n.avanzar_hasta(h * HORA);
        un_evento_por_aeda(&e.n);
        salio |= e.n.aedas().iter().any(|x| x.plan.is_some());
    }
    assert!(salio, "alguno sale de camino");
    for x in e.n.aedas() {
        assert!(x.recientes.contains(&a) && x.recientes.contains(&b), "aeda-{} pasó por las dos: {:?}", x.id, x.recientes);
        // Una estancia empieza al llegar y dura lo que dice el bloque.
        if let (Some(_), Some(hasta)) = (x.en, x.hasta) {
            assert!(hasta - e.n.ahora() <= estancia);
        }
    }
    // A la velocidad de los Aedas, por el plan de la red: sale cuando acaba la estancia y su plan empieza en la plaza de la que parte.
    let velocidad = e.n.estado().reglas.economia.tecnologia.aedas.velocidad;
    assert_eq!(velocidad, 20_000, "las cifras del TypeScript (Q3)");
    assert!(hechos(&e.n, "aedas.revela").is_empty(), "sin descubrimientos no revelan nada");
}

// ───────────────────────────── 3. La puerta ─────────────────────────────

#[test]
fn una_plaza_que_les_cerro_la_puerta_no_los_recibe_pero_el_cierre_por_defecto_no_cuenta() {
    let m = con_mundo!();
    let mut e = Escenario::nuevo(m, reglas());
    let b = e.b;
    e.puerta(b, &SIN_AEDAS);
    for h in 1..=48 {
        e.n.avanzar_hasta(h * HORA);
        assert!(e.en_plaza(b).is_empty() && e.n.aedas().iter().all(|x| x.destino != Some(b as u32)), "B les cerró la puerta: nadie va ni se para en ella");
        un_evento_por_aeda(&e.n);
    }
    // El cierre por defecto (neutrales y enemigos) no incluye a los Aedas: vuelven.
    e.puerta(b, &POR_DEFECTO);
    let mut llego = false;
    for h in 49..=72 {
        e.n.avanzar_hasta(h * HORA);
        llego |= !e.en_plaza(b).is_empty();
    }
    assert!(llego, "con la puerta por defecto, B los vuelve a recibir");
}

#[test]
fn cerrarles_la_puerta_con_un_aeda_dentro_lo_hace_salir_al_instante_y_lo_revelado_queda_revelado() {
    let m = con_mundo!();
    let mut e = Escenario::nuevo(m, reglas());
    let (a, b) = (e.a, e.b);
    // Todos a B: A les cierra la puerta (salen al instante, Q5) y no hay otra plaza.
    assert_eq!(e.en_plaza(a).len(), 3);
    e.puerta(a, &SIN_AEDAS);
    let t = e.n.ahora();
    assert!(e.en_plaza(a).is_empty(), "salen al instante");
    assert!(e.n.aedas().iter().all(|x| x.destino == Some(b as u32) && x.plan.as_ref().and_then(|p| p.inicio()) == Some(t)), "hacia su siguiente destino, desde ya");
    un_evento_por_aeda(&e.n);
    let llegan = e.n.aedas().iter().map(|x| x.siguiente_evento().unwrap()).max().unwrap();
    e.n.avanzar_hasta(llegan);
    assert_eq!(e.en_plaza(b).len(), 3, "llegan a B");
    // Con el cobre conocido, el Aeda de B se lo revela a la Facción 2.
    let desde = e.descubrir_cobre();
    e.n.avanzar_hasta(desde + DIA);
    let cobre = e.tec("metalurgia_cobre");
    assert!(e.propias(2).reveladas.contiene(cobre));
    // B les cierra la puerta: salen al instante hacia A... que también está cerrada, así que se quedan varados donde están, esperando otra estancia; lo revelado sigue revelado.
    e.puerta(b, &SIN_AEDAS);
    let t = e.n.ahora();
    assert!(e.en_plaza(b).is_empty());
    assert!(e.n.aedas().iter().all(|x| x.en.is_none() && x.destino.is_none() && x.hasta == Some(t + e.n.estado().reglas.economia.tecnologia.aedas.estancia)));
    un_evento_por_aeda(&e.n);
    assert!(e.propias(2).reveladas.contiene(cobre), "lo que ya reveló queda revelado");
    // A les abre la puerta: al acabar su espera van a A.
    e.puerta(a, &POR_DEFECTO);
    e.n.avanzar_hasta(t + DIA);
    assert_eq!(e.en_plaza(a).len(), 3);
    un_evento_por_aeda(&e.n);
}

// ───────────────────────────── 4. Revelar ─────────────────────────────

/// El escenario con los tres Aedas parados en B (A les cerró la puerta).
fn todos_en_b(m: &M, reglas: Reglas) -> Escenario {
    let mut e = Escenario::nuevo(m, reglas);
    let a = e.a;
    e.puerta(a, &SIN_AEDAS);
    let llegan = e.n.aedas().iter().map(|x| x.siguiente_evento().unwrap()).max().unwrap();
    e.n.avanzar_hasta(llegan);
    assert_eq!(e.en_plaza(e.b).len(), 3);
    e
}

#[test]
fn la_ventaja_del_primero_hasta_que_pasa_el_retraso_no_la_conocen_y_entonces_se_la_revelan_una_vez_en_plena_estancia() {
    let m = con_mundo!();
    let mut e = todos_en_b(m, reglas());
    let b = e.b;
    let desde = e.descubrir_cobre();
    let cobre = e.tec("metalurgia_cobre");
    e.n.avanzar_hasta(desde + DIA - 1);
    assert!(hechos(&e.n, "aedas.revela").is_empty(), "aún no la conocen");
    assert!(!e.en_plaza(b).is_empty(), "el Aeda sigue parado en B");
    e.n.avanzar_hasta(desde + DIA);
    let r = hechos(&e.n, "aedas.revela");
    assert_eq!(r.len(), 1, "una sola revelación aunque haya tres Aedas en la plaza");
    let h = &r[0];
    assert_eq!((h.en, h.dato("faccion"), h.dato("descubridora"), h.asentamientos.clone(), h.publico), (desde + DIA, Some(2), Some(1), vec![b], false));
    assert_eq!(texto(h, "tecnologia").as_deref(), Some("metalurgia_cobre"));
    let faltan = h.valores.iter().find(|(k, _)| *k == "faltan").map(|(_, v)| v.clone());
    assert_eq!(faltan, Some(Dato::Lista(vec![Dato::Numero(0)])), "le falta la fundición (la condición 0 del hito)");
    let t = e.propias(2);
    assert!(t.reveladas.contiene(cobre) && !t.aparecidas.contiene(cobre), "revelada, no aparecida");
    // Se revela una sola vez.
    e.n.avanzar_hasta(desde + 3 * DIA);
    assert_eq!(hechos(&e.n, "aedas.revela").len(), 1);
    un_evento_por_aeda(&e.n);
}

#[test]
fn no_se_revela_a_quien_ya_la_tiene_aparecida_y_al_llegar_a_una_plaza_se_revela_lo_que_ya_conocen() {
    let m = con_mundo!();
    // A quien ya la tiene aparecida, no.
    let mut e = todos_en_b(m, reglas());
    let desde = e.descubrir_cobre();
    e.ok("admin", Comando::FijarTecnologias { faccion: 2, tecnologias: vec!["metalurgia_cobre".into()] });
    e.n.avanzar_hasta(desde + 2 * DIA);
    assert!(hechos(&e.n, "aedas.revela").is_empty());

    // Al llegar: los Aedas esperan en A (B cerrada) cuando la conocen, y se la revelan a la Facción 2 en cuanto llegan a B.
    let mut e = Escenario::nuevo(m, reglas());
    let b = e.b;
    e.puerta(b, &SIN_AEDAS);
    let desde = e.descubrir_cobre();
    e.n.avanzar_hasta(desde + DIA + HORA);
    assert!(hechos(&e.n, "aedas.revela").is_empty(), "en A, de la descubridora, no hay nada que revelar");
    e.puerta(b, &POR_DEFECTO);
    let abierta = e.n.ahora();
    e.n.avanzar_hasta(abierta + DIA);
    let r = hechos(&e.n, "aedas.revela");
    assert_eq!(r.len(), 1);
    let llegada = r[0].en;
    assert!(llegada > abierta && r[0].asentamientos == vec![b]);
    assert!(e.n.aedas().iter().any(|x| x.recientes.last() == Some(&(b as u32))), "llegó a B");
}

// ───────────────────────────── 5. Vender ─────────────────────────────

#[test]
fn comprarla_cuesta_el_doble_del_oro_de_la_tarifa_sale_del_almacen_de_la_plaza_y_aparece_sin_hito_sin_adoptar_y_se_adopta_despues() {
    let m = con_mundo!();
    let mut e = todos_en_b(m, reglas());
    let b = e.b;
    let rey = e.rey_dentro(2, b);
    let desde = e.descubrir_cobre();
    e.n.avanzar_hasta(desde + DIA);
    for (recurso, cantidad) in [(Recurso::Oro, 1_000_000), (Recurso::Madera, 1_000_000)] {
        e.ok("admin", Comando::DarRecursos { asentamiento: b, recurso, cantidad });
    }
    let cobre = e.tec("metalurgia_cobre");
    assert!(e.propias(2).reveladas.contiene(cobre));
    let antes = e.oro(b);
    let c = e.comprar(rey, b, "metalurgia_cobre");
    e.ok("bea", c);
    let precio = tecnologia::adopcion::precio_de_venta(&e.n.estado().reglas.economia.tecnologia, cobre);
    assert_eq!(precio, 200_000, "2 × 100 de oro de la tarifa de la Era I");
    assert_eq!(e.oro(b), antes - precio);
    let t = e.propias(2);
    assert!(t.aparecidas.contiene(cobre) && !t.adoptadas.contiene(cobre) && !t.reveladas.contiene(cobre), "aparecida sin hito, no adoptada; ya no revelada");
    let v = hechos(&e.n, "aedas.venta");
    assert_eq!(v.len(), 1);
    let aeda = e.en_plaza(b)[0];
    assert_eq!((v[0].dato("faccion"), v[0].dato("aeda"), v[0].dato("precio"), v[0].asentamientos.clone(), v[0].publico), (Some(2), Some(i64::from(aeda)), Some(precio), vec![b], false));
    assert_eq!(texto(&v[0], "tecnologia").as_deref(), Some("metalurgia_cobre"));
    assert_eq!(e.n.tecnologia().primeros[cobre as usize].map(|p| p.faccion), Some(1), "la descubridora sigue siendo la Facción 1");
    // Y se adopta como cualquier aparecida, pagando la tarifa.
    e.ok("bea", Comando::AdoptarTecnologia { heroe: rey, faccion: 2, tecnologia: cobre });
    assert!(e.n.tiene_tecnologia(2, "metalurgia_cobre"));
    un_evento_por_aeda(&e.n);
}

#[test]
fn rechazos_de_la_compra_sin_rastro() {
    let m = con_mundo!();
    let mut e = todos_en_b(m, reglas());
    let (a, b) = (e.a, e.b);
    let rey = e.rey_dentro(2, b);
    let desde = e.descubrir_cobre();
    // Antes de que la conozca.
    let c = e.comprar(rey, b, "metalurgia_cobre");
    assert_eq!(e.rechazo("bea", c), rechazos::AEDAS_NO_LA_CONOCE);
    e.n.avanzar_hasta(desde + DIA);
    // Sin logro cumplido (la aleación del bronce): tampoco la conoce.
    let c = e.comprar(rey, b, "aleacion_bronce");
    assert_eq!(e.rechazo("bea", c), rechazos::AEDAS_NO_LA_CONOCE);
    // Una tecnología que no existe; una plaza que no existe.
    assert_eq!(e.rechazo("bea", Comando::ComprarTecnologiaAeda { heroe: rey, asentamiento: b, tecnologia: 200 }), rechazos::TECNOLOGIA_DESCONOCIDA);
    assert_eq!(e.rechazo("bea", Comando::ComprarTecnologiaAeda { heroe: rey, asentamiento: 9_999, tecnologia: 0 }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    // Sin Aeda en la plaza: en A (cerrada a los Aedas), su Rey dentro.
    let rey_a = e.rey_dentro(1, a);
    let c = e.comprar(rey_a, a, "metalurgia_cobre");
    assert_eq!(e.rechazo("ana", c), rechazos::AEDAS_NO_HAY_AEDA);
    // Solo el Rey o el Gobernador, dentro: el Rey de la Facción 1 no manda en B, y el de la 2 fuera de B tampoco compra.
    let c = e.comprar(rey_a, b, "metalurgia_cobre");
    assert_eq!(e.rechazo("ana", c), rechazos::NO_AUTORIZADO);
    e.ok("admin", Comando::ColocarHeroe { heroe: rey, ubicacion: militar::presencia::Ubicacion::Asentamiento(a as u32) });
    let c = e.comprar(rey, b, "metalurgia_cobre");
    assert_eq!(e.rechazo("bea", c), rechazos::NO_AUTORIZADO);
    e.ok("admin", Comando::ColocarHeroe { heroe: rey, ubicacion: militar::presencia::Ubicacion::Asentamiento(b as u32) });
    // Comprada, ya no se vuelve a comprar.
    let c = e.comprar(rey, b, "metalurgia_cobre");
    e.ok("bea", c.clone());
    assert_eq!(e.rechazo("bea", c), rechazos::AEDAS_YA_APARECIDA);
    // Una Era que no se vende (con el máximo bajado a 0 para la prueba: las Eras IV y V aún no existen).
    let mut e = todos_en_b(m, reglas_con(|r| r.aedas.orden_era_maximo_venta = 0));
    let b = e.b;
    let rey = e.rey_dentro(2, b);
    let desde = e.descubrir_cobre();
    e.n.avanzar_hasta(desde + DIA);
    let c = e.comprar(rey, b, "metalurgia_cobre");
    assert_eq!(e.rechazo("bea", c), rechazos::AEDAS_ERA_NO_VENDIBLE);
    // Sin el oro del precio en el almacén de la plaza (con el factor subido para la prueba: la plaza recién fundada ya tiene oro de sobra para el de verdad).
    let mut e = todos_en_b(m, reglas_con(|r| r.aedas.factor_oro_venta = 1_000));
    let b = e.b;
    let rey = e.rey_dentro(2, b);
    let desde = e.descubrir_cobre();
    e.n.avanzar_hasta(desde + DIA);
    let precio = tecnologia::adopcion::precio_de_venta(&e.n.estado().reglas.economia.tecnologia, e.tec("metalurgia_cobre"));
    assert!(e.oro(b) < precio, "{} de oro", e.oro(b));
    let c = e.comprar(rey, b, "metalurgia_cobre");
    assert_eq!(e.rechazo("bea", c), rechazos::AEDAS_SIN_ORO);
}

// ───────────────────────────── 6. La foto y el determinismo ─────────────────────────────

#[test]
fn la_foto_ban39_lleva_los_aedas_y_su_evento_y_ban38_se_rechaza() {
    let m = con_mundo!();
    let mut e = Escenario::nuevo(m, reglas());
    // Hora a hora hasta que alguno vaya de camino: cuándo sale el primero depende del mundo (con el de 2026-10-28, más tarde que a las 7 h).
    let mut t = 7 * HORA;
    while !e.n.aedas().iter().any(|x| x.plan.is_some()) && t < 3 * DIA {
        e.n.avanzar_hasta(t);
        t += HORA;
    }
    assert!(e.n.aedas().iter().any(|x| x.plan.is_some()), "alguno de camino, con su plan");
    let reglas = e.n.estado().reglas.clone();
    let mundo_hash = e.n.estado().mundo_hash.clone();
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN49");
    let vuelta = Estado::deserializar(&bytes, &reglas, &mundo_hash).unwrap();
    assert_eq!(vuelta.serializar(), bytes);
    assert_eq!(vuelta.hash(), e.n.estado().hash());
    assert_eq!(vuelta.aedas, e.n.estado().aedas);
    assert_eq!(vuelta.cola.volcar().iter().filter(|x| x.carga == Carga::Aeda && vuelta.generaciones.es_vigente(x.clave)).count(), 3, "su evento viaja en la cola");
    // Cada campo de un Aeda cambia el hash.
    let base = e.n.estado().hash();
    let cambios: Vec<Box<dyn Fn(&mut Estado)>> = vec![
        Box::new(|s| s.aedas[0].posicion.x += 1),
        Box::new(|s| s.aedas[0].en = Some(9)),
        Box::new(|s| s.aedas[0].hasta = Some(1)),
        Box::new(|s| s.aedas[0].destino = Some(9)),
        Box::new(|s| s.aedas[0].plan = None),
        Box::new(|s| s.aedas[0].recientes.push(9)),
    ];
    for c in cambios {
        let mut s = Estado::deserializar(&bytes, &reglas, &mundo_hash).unwrap();
        c(&mut s);
        assert_ne!(s.hash(), base);
    }
    let vieja = [b"BAN38".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &mundo_hash).err().expect("BAN38 se rechaza");
    assert!(error.0.contains("BAN38") && error.0.contains("Aedas"), "{}", error.0);
}

/// Dos plazas, el cobre descubierto, la puerta de B cerrada y abierta, una compra y tres días.
fn guion(m: &M, n: Nucleo, avanzar: &dyn Fn(&mut Nucleo, Instante)) -> Nucleo {
    let mut e = Escenario::en(n, m);
    let b = e.b;
    let rey = e.rey_dentro(2, b);
    e.ok("admin", Comando::DarRecursos { asentamiento: b, recurso: Recurso::Oro, cantidad: 1_000_000 });
    e.descubrir_cobre();
    avanzar(&mut e.n, 20 * HORA);
    e.puerta(b, &SIN_AEDAS);
    avanzar(&mut e.n, 30 * HORA);
    e.puerta(b, &POR_DEFECTO);
    avanzar(&mut e.n, 2 * DIA);
    if !e.en_plaza(b).is_empty() {
        let c = e.comprar(rey, b, "metalurgia_cobre");
        e.ord("bea", c);
    }
    avanzar(&mut e.n, 3 * DIA);
    assert!(e.propias(2).reveladas.contiene(e.tec("metalurgia_cobre")) || e.propias(2).aparecidas.contiene(e.tec("metalurgia_cobre")), "el guion hizo lo que se espera");
    e.n
}

#[test]
fn los_aedas_dan_el_mismo_mundo_de_una_vez_a_trozos_y_tras_una_foto() {
    let m = con_mundo!();
    let nuevo = || Nucleo::nuevo(arranque(m, reglas(), Ajustes::default()));
    let de_una = guion(m, nuevo(), &|n, t| n.avanzar_hasta(t));
    let a_trozos = guion(m, nuevo(), &|n, t| {
        let desde = n.ahora();
        for s in 1..=7 {
            n.avanzar_hasta(desde + (t - desde) * s * s / 49);
        }
    });
    assert_eq!(de_una.hash_estado(), a_trozos.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let dir = tempfile::tempdir().unwrap();
    let n = Nucleo::con_persistencia(arranque(m, reglas(), ajustes.clone()), dir.path(), Default::default()).unwrap();
    let mut cortado = guion(m, n, &|n, t| {
        let mitad = (n.ahora() + t) / 2;
        n.avanzar_hasta(mitad);
        n.foto_ahora();
        n.avanzar_hasta(mitad + HORA);
        n.apagar().unwrap();
        *n = Nucleo::con_persistencia(arranque(m, reglas(), ajustes.clone()), dir.path(), Default::default()).unwrap();
        assert!(n.recuperado());
        n.avanzar_hasta(t);
    });
    assert_eq!(cortado.hash_estado(), de_una.hash_estado());
    cortado.apagar().unwrap();
}

// ───────────────────────────── 7. Por la sesión ─────────────────────────────

#[tokio::test]
async fn por_la_sesion_el_rey_compra_al_aeda_de_su_plaza_y_su_faccion_oye_la_venta() {
    use comun::sesion::*;
    use contrato::v1::{self as pb, orden, TipoCliente};
    let m = con_mundo!();
    let mut e = todos_en_b(m, reglas());
    let (a, b) = (e.a, e.b);
    // Por la sesión se juega con un héroe de verdad: «bea» crea el suyo, entra en la Facción 2, el fundador le cede el trono y la administración lo pone dentro de B.
    let r = e.ord("bea", Comando::CrearHeroe { cuenta: "bea".into(), campamento: 0, controlador: militar::heroe::Controlador::Humano, nombre: None });
    let rey = r.creadas[0];
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: rey, faccion: Some(2) });
    let fundador = e.n.facciones().find(|f| f.id == 2).and_then(|f| f.rey).unwrap();
    e.ok("bea", Comando::AsignarRey { heroe: fundador, faccion: 2, designado: rey });
    e.ok("admin", Comando::ColocarHeroe { heroe: rey, ubicacion: militar::presencia::Ubicacion::Asentamiento(b as u32) });
    e.ok("admin", Comando::DarRecursos { asentamiento: b, recurso: Recurso::Oro, cantidad: 1_000_000 });
    let desde = e.descubrir_cobre();
    e.n.avanzar_hasta(desde + DIA);
    let w = arrancar_con(e.n).await;
    let (mut c, _) = w.entrar("bea", TipoCliente::ClienteUnity).await;
    let comprar = |plaza: u64, x: &str| orden::Cuerpo::ComprarTecnologiaAeda(pb::OrdenComprarTecnologiaAeda { asentamiento_id: format!("as-{plaza}"), tecnologia_id: x.into() });
    let (_, r) = dar(&mut c, "c1", comprar(b, "no_existe")).await;
    assert_eq!(rechazo(&r), "tecnologia.desconocida", "ya no es `orden.no_disponible`");
    let (_, r) = dar(&mut c, "c2", comprar(a, "metalurgia_cobre")).await;
    assert_eq!(rechazo(&r), "no_autorizado", "en A no manda");
    let (deltas, r) = dar(&mut c, "c3", comprar(b, "metalurgia_cobre")).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    assert!(deltas.iter().flat_map(|d| &d.eventos).any(|x| x.codigo == "aedas.venta"), "el hecho llega a la Facción");
    let (_, r) = dar(&mut c, "c4", comprar(b, "metalurgia_cobre")).await;
    assert_eq!(rechazo(&r), "aedas.ya_aparecida");
    c.cerrar().await;
    let n = w.parar().await;
    let r = &n.estado().reglas.economia.tecnologia;
    assert!(n.tecnologia().de(r, 2).aparecidas.contiene(r.buscar("metalurgia_cobre").unwrap()));
}

// ───────────────────────────── 6G: lo que ve cada ámbito ─────────────────────────────

/// Las entidades de una lista de mensajes del servidor (fotos y deltas), en orden, con la clave de su ámbito.
fn cuerpos(mensajes: &[contrato::v1::mensaje_de_servidor::Cuerpo]) -> Vec<(String, contrato::v1::entidad::Cuerpo)> {
    use contrato::v1::mensaje_de_servidor::Cuerpo as M;
    let ambito = |a: &Option<contrato::v1::Ambito>| contrato::almacen::clave_de_ambito(a);
    let mut v = Vec::new();
    for m in mensajes {
        match m {
            M::Foto(f) => v.extend(f.entidades.iter().filter_map(|e| e.cuerpo.clone()).map(|c| (ambito(&f.ambito), c))),
            M::Delta(d) => v.extend(d.cambios.iter().filter_map(|c| c.datos.as_ref()?.cuerpo.clone()).map(|c| (ambito(&d.ambito), c))),
            _ => {}
        }
    }
    v
}

/// La `CambiarVista` del mapa entero.
fn vista_de_todo() -> contrato::v1::mensaje_de_cliente::Cuerpo {
    use contrato::v1 as pb;
    pb::mensaje_de_cliente::Cuerpo::Vista(pb::CambiarVista { rectangulo: Some(pb::Rectangulo { min: Some(pb::Punto { x: 0, y: 0 }), max: Some(pb::Punto { x: 100_000_000, y: 100_000_000 }) }) })
}

/// 6G por el WebSocket real, con la metalurgia del cobre descubierta por la Facción 1 y revelada a la 2 por los tres Aedas parados en su plaza B. **HEROE**: la forma pública del estado de
/// tecnología (la Era, el logro cumplido como hecho del mundo, el descubrimiento ya cantado con su descubridora), sin contadores ni logros por tecnología. **FACCION**: las tecnologías de la
/// propia, con el detalle de la revelada (descubridora y qué condiciones del hito cumple); nunca las de la otra. **VISTA**: los Aedas que se ven, redactados (su plaza; ni hasta cuándo, ni adónde, ni
/// dónde estuvieron); quien no los ve, no los recibe. **ADMIN**: el estado entero con los contadores, y los Aedas enteros, que se ponen al día cuando cambian sin que cambie la versión.
#[tokio::test]
async fn por_la_sesion_cada_ambito_lleva_lo_suyo_de_la_tecnologia_y_de_los_aedas_y_nada_de_lo_ajeno() {
    use comun::sesion::*;
    use contrato::v1::{self as pb, entidad::Cuerpo, mensaje_de_servidor as ms, orden, TipoCliente};
    use servidor::prueba::hola;
    let m = con_mundo!();
    let mut e = todos_en_b(m, reglas());
    let b = e.b;
    let r = e.ord("bea", Comando::CrearHeroe { cuenta: "bea".into(), campamento: 0, controlador: militar::heroe::Controlador::Humano, nombre: None });
    let bea = r.creadas[0];
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: bea, faccion: Some(2) });
    e.ok("admin", Comando::ColocarHeroe { heroe: bea, ubicacion: militar::presencia::Ubicacion::Asentamiento(b as u32) });
    let desde = e.descubrir_cobre();
    e.n.avanzar_hasta(desde + DIA);
    let cobre = e.tec("metalurgia_cobre");
    assert!(e.propias(2).reveladas.contiene(cobre), "revelada a la Facción 2");
    let hito = e.n.estado().reglas.economia.tecnologia.def(cobre).hito.len();
    let e_reglas = e.n.estado().reglas.economia.tecnologia.clone();
    let w = arrancar_con(e.n).await;
    let tec_de = |x: &Cuerpo| if let Cuerpo::Tecnologias(t) = x { Some(t.clone()) } else { None };

    // bea (Facción 2): su HEROE, su FACCION y su plaza; después, su VISTA del mapa entero.
    let (mut c, _, mut recibido) = w.conectar(hola("ok:bea", TipoCliente::ClienteUnity)).await;
    c.enviar(vista_de_todo()).await;
    recibido.push(c.esperar().await.cuerpo.unwrap());
    let todo = cuerpos(&recibido);
    let publicas: Vec<(&String, &pb::EstadoTecnologia)> = todo.iter().filter_map(|(a, x)| if let Cuerpo::EstadoTecnologia(t) = x { Some((a, t)) } else { None }).collect();
    assert_eq!(publicas.len(), 1, "una vez, en HEROE");
    let (a, t) = publicas[0];
    assert_eq!(a, &format!("HEROE:heroe-{bea}"));
    assert!(t.contadores.is_empty() && t.logros.is_empty(), "ni contadores en curso ni qué tecnología abre cada logro (Q6)");
    assert_eq!(t.logros_cumplidos.iter().map(|l| (l.contador.as_str(), l.umbral)).collect::<Vec<_>>(), [("plazasEnNivel.2", 1)]);
    assert_eq!(t.cantadas, ["metalurgia_cobre"]);
    let era = &e_reglas.eras[0];
    assert_eq!((t.era_minimo_hasta_ms, t.era_plazo_hasta_ms), (Some(era.minimo), Some(era.plazo)), "los dos bordes de la Era I (desde 0): públicos");
    assert_eq!(t.primeros.iter().map(|(k, p)| (k.as_str(), p.faccion_id.as_str())).collect::<Vec<_>>(), [("metalurgia_cobre", "f-1")], "solo las descubridoras ya cantadas");
    let propias: Vec<pb::TecnologiasFaccion> = todo.iter().filter_map(|(_, x)| tec_de(x)).collect();
    assert!(!propias.is_empty() && propias.iter().all(|t| t.faccion_id == "f-2"), "las tecnologías de otra Facción no viajan nunca");
    let f2 = &propias[0];
    assert_eq!(f2.reveladas, ["metalurgia_cobre"]);
    assert_eq!((f2.detalle_reveladas.len(), f2.detalle_reveladas[0].descubridora_faccion_id.as_str(), f2.detalle_reveladas[0].cumplidas.len()), (1, "f-1", hito));
    let aedas: Vec<(&String, &pb::AedaItinerante)> = todo.iter().filter_map(|(a, x)| if let Cuerpo::AedaItinerante(x) = x { Some((a, x)) } else { None }).collect();
    assert_eq!(aedas.len(), 3, "los tres parados en su plaza");
    for (a, x) in &aedas {
        assert_eq!(a.as_str(), "VISTA:f-2");
        assert_eq!(x.en_asentamiento_id.as_deref(), Some(format!("as-{b}").as_str()));
        assert!(x.hasta_ms.is_none() && x.destino_id.is_none() && x.recientes.is_empty() && x.posicion.is_some(), "redactado: {x:?}");
    }
    assert!(!todo.iter().any(|(_, x)| matches!(x, Cuerpo::AedaResidente(_) | Cuerpo::Alijo(_))));
    c.cerrar().await;

    // ana (Facción 1), con la vista del mapa entero, no ve a los Aedas de B ni las tecnologías de la 2.
    let (mut c, _, mut recibido) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    c.enviar(vista_de_todo()).await;
    recibido.push(c.esperar().await.cuerpo.unwrap());
    let todo = cuerpos(&recibido);
    assert!(!todo.iter().any(|(_, x)| matches!(x, Cuerpo::AedaItinerante(_))), "no los ve");
    let f1: Vec<pb::TecnologiasFaccion> = todo.iter().filter_map(|(_, x)| tec_de(x)).collect();
    assert!(f1.len() == 1 && f1[0].faccion_id == "f-1" && f1[0].aparecidas.contains(&"metalurgia_cobre".to_owned()), "{f1:?}");
    c.cerrar().await;

    // ADMIN: entero, y al día cuando un Aeda cambia (la estancia acaba: sin otra plaza que los acepte, esperan otra) sin que cambie la versión.
    let (mut adm, _, recibido) = w.conectar(hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
    let todo = cuerpos(&recibido);
    let t = todo.iter().find_map(|(_, x)| if let Cuerpo::EstadoTecnologia(t) = x { Some(t) } else { None }).expect("el estado entero en ADMIN");
    assert_eq!((t.contadores.get("plazasEnNivel.2"), t.logros.contains_key("metalurgia_cobre"), t.contadores.len()), (Some(&1), true, 24));
    let enteros: Vec<&pb::AedaItinerante> = todo.iter().filter_map(|(_, x)| if let Cuerpo::AedaItinerante(x) = x { Some(x) } else { None }).collect();
    assert!(enteros.len() == 3 && enteros.iter().all(|x| x.hasta_ms.is_some() && x.recientes.last() == Some(&format!("as-{b}"))), "con su estancia y dónde estuvieron: {enteros:?}");
    let fin = enteros.iter().filter_map(|x| x.hasta_ms).min().unwrap();
    let reloj = |accion: &str, hasta_ms: Option<i64>| orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: accion.into(), hasta_ms, ..Default::default() });
    let (_, r) = dar(&mut adm, "r1", reloj("pausar", None)).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let (deltas, r) = dar(&mut adm, "r2", reloj("avanzar_hasta", Some(fin))).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let al_dia = cuerpos(&deltas.into_iter().map(ms::Cuerpo::Delta).collect::<Vec<_>>());
    let nuevo = al_dia.iter().filter_map(|(_, x)| if let Cuerpo::AedaItinerante(x) = x { Some(x) } else { None }).find(|x| x.hasta_ms > Some(fin));
    assert!(nuevo.is_some(), "el Aeda cuya estancia acabó llega a ADMIN con su nueva estancia: {al_dia:?}");
    adm.cerrar().await;
    w.parar().await;
}
