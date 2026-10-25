//! Las avanzadas en el núcleo (fase 4F; Doc 5.17, Economia_Construccion §14.1, Combate §7.4): colocarlas con el héroe en el sitio, entregar los materiales,
//! el fin exacto de la obra, mejorarlas, repararlas, cancelar esas obras y destruirlas, dónde caben, la guarnición de serie y de jugadores, el almacén, la visión, el ataque con números y por
//! Unity, las ruinas con su invulnerabilidad, la conquista y la ruina de la madre, la foto, el determinismo y el hash. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;
use asentamiento::avanzadas::{Estado as EstadoAvanzada, Etapa, TipoObra};
use asentamiento::catalogo::Tipo;
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, Rng, MS_POR_MINUTO};
use combate::ciclo::{Asignacion, EstadoBatalla, Instancia, Token};
use combate::resolucion::Lado;
use combate::resultado::{Razon, Resultado as ResultadoDeBatalla, ResultadoEscuadra, ResultadoHeroe};
use militar::catalogo::Tropa;
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::economia::Mapa;
use nucleo::{rechazos, Ajustes, Arranque, Carga, Comando, Nucleo, ObjetivoAtaque, Peticion, Relacion, Reglas, Resultado, Respuesta, SucesoCombate, SucesoMilitar};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;
const DIA: i64 = 24 * HORA;

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

/// Las reglas por defecto sin acoso ni anillo de bandidos: estos tests miden otra cosa y se calibraron sin ellos.
fn sin_bandidos() -> Reglas {
    let mut r = Reglas::default();
    let b = &mut Arc::make_mut(&mut r.economia.combate).bandidos;
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

type Hecho = (Instante, Vec<u64>, Vec<u64>, Vec<(&'static str, i64)>);

fn hechos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.ejercitos.clone(), e.asentamientos.clone(), e.datos.clone())).collect()
}

fn dato(h: &Hecho, clave: &str) -> i64 {
    h.3.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("el hecho no lleva «{clave}»: {h:?}")).1
}

fn rastro(n: &Nucleo) -> (u64, [u8; 32], u64, u64) {
    (n.version(), n.hash_estado(), n.estado().rng.estado(), n.estado().siguiente_id)
}

/// Las tiradas que gastó el generador de la partida entre dos estados suyos (hasta 8).
fn tiradas(antes: u64, despues: u64) -> usize {
    let mut r = Rng::nuevo(antes);
    for k in 0..=8 {
        if r.estado() == despues {
            return k;
        }
        r.siguiente();
    }
    panic!("más de 8 tiradas")
}

fn completa(tropa: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(tropa).unidades
}

/// Un sitio fundable, verde y con mucho bosque, a `r` del punto `c`, con `lejos` libres de la protección de un campamento de mercenarios (donde nadie combate).
fn sitio_junto_a(n: &Nucleo, c: Punto, r: i64) -> Option<Punto> {
    (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64)).find(|&p| {
        let libre = |q: Punto| !n.en_proteccion_de_campamento(q);
        libre(p) && libre(Punto::new(p.x + 12_000, p.y)) && libre(Punto::new(p.x + 9_000, p.y))
            && n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6 && !v.condenada)
    })
}

/// La plaza madre de Ana (F1), ya de nivel `nivel`, con la auto-construcción parada y la zona agrandada hasta el techo del nivel.
struct Esc {
    n: Nucleo,
    plaza: u64,
    /// Dónde se fundó la plaza.
    centro: Punto,
    cuenta: u64,
}

impl Esc {
    fn peticion(&mut self, actor: &str, c: Comando) -> Peticion {
        self.cuenta += 1;
        // Andamiaje (5E): las órdenes de plaza de una cuenta de jugador las da la administración en estos escenarios, salvo con la cuenta precedida de «!», que la da tal cual (para probar
        // quién puede darlas o el candado de batalla, que la administración no lleva). La matriz entera la prueba `tests/cargos.rs`.
        let (actor, andamiaje) = actor.strip_prefix('!').map_or((actor, true), |a| (a, false));
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        if actor == "admin" || andamiaje && comun::mandaba(&self.n, actor, &p.comando) {
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
    /// Una orden que se rechaza y no deja rastro.
    fn rechazada(&mut self, actor: &str, c: Comando) -> &'static str {
        self.asentar();
        let antes = rastro(&self.n);
        let r = self.ord(actor, c);
        assert_eq!(rastro(&self.n), antes, "el rechazo {r:?} dejó rastro");
        codigo(&r)
    }
    fn sin_cambios(&mut self, actor: &str, c: Comando) {
        self.asentar();
        let antes = rastro(&self.n);
        let r = self.ord(actor, c);
        assert_eq!(r.resultado, Resultado::SinCambios, "{r:?}");
        assert_eq!(rastro(&self.n), antes);
    }
    /// Procesa lo que ya vencía en el instante actual (una plaza recién fundada o una columna recién creada pueden tener algo a punto): así «sin rastro» mide solo la orden.
    fn asentar(&mut self) {
        let t = self.n.ahora();
        self.n.avanzar_hasta(t);
    }
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.hasta(t);
    }
    /// Avanza hasta `t` reponiendo a la plaza sus materiales a cada hora en punto: sin auto-construcción ni comercio acabaría en ruinas al día y pico, y las pruebas largas
    /// (una ciudadela son 62 horas de obra) miden otra cosa. Las horas son absolutas: trocear el avance no cambia cuándo se repone.
    fn hasta(&mut self, t: Instante) {
        loop {
            let hora = (self.n.ahora().div_euclid(HORA) + 1) * HORA;
            if hora > t {
                break;
            }
            self.n.avanzar_hasta(hora);
            let plaza = self.plaza;
            for recurso in [Madera, Piedra, Oro, Trigo] {
                self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad: 400_000 });
            }
        }
        self.n.avanzar_hasta(t);
    }
    fn almacen(&self, r: Recurso) -> i64 {
        self.n.economia(self.plaza).unwrap().almacen.cantidad(r)
    }
    /// Una columna de `faccion` en `p` (un ejército de un héroe): (columna, héroe).
    fn columna(&mut self, dueno: &str, faccion: u32, p: Punto) -> (u64, u64) {
        let t = self.n.ahora();
        let id = self.n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
        (id, self.n.ejercito(id).unwrap().columna.lider)
    }
    fn dar_escuadra(&mut self, heroe: u64, tropa: Tropa) -> u64 {
        self.ok("admin", Comando::DarEscuadra { heroe, tropa, cantidad: completa(tropa) }).creadas[0]
    }
    fn residir(&mut self, heroe: u64) {
        let plaza = self.plaza as u32;
        self.ok("admin", Comando::FijarResidencia { heroe, residencia: Some(Residencia::Asentamiento(plaza)) });
    }
    /// La puerta de antes del sistema 5 (5E): abierta también a los neutrales, que por defecto ya no entran (Q1). Para las pruebas en las que un neutral está dentro.
    fn abrir_a_los_neutrales(&mut self) {
        let plaza = self.plaza;
        self.ok("admin", Comando::FijarAcceso { asentamiento: plaza, aliados: true, neutrales: true, enemigos: false });
    }
    /// El sitio de la primera avanzada: 8 u al este de la plaza (dentro de su zona de 15).
    fn sitio(&self) -> Punto {
        Punto::new(self.centro.x + 8_000, self.centro.y)
    }
    fn avanzada(&self, id: u64) -> &asentamiento::avanzadas::Avanzada {
        self.n.avanzada(id).unwrap_or_else(|| panic!("no existe la avanzada {id}")).1
    }
    fn estado_de(&self, id: u64) -> EstadoAvanzada {
        self.avanzada(id).estado
    }
    /// Coloca una avanzada en `p` con una columna de Ana en el sitio. Devuelve (avanzada, columna, héroe).
    fn colocar_en(&mut self, p: Punto) -> (u64, u64, u64) {
        let (col, heroe) = self.columna("ana", 1, p);
        let plaza = self.plaza;
        let id = self.ok("ana", Comando::ColocarAvanzada { asentamiento: plaza, heroe, posicion: p }).creadas[0];
        (id, col, heroe)
    }
    /// Entrega con la columna `col` todo lo que le falta a la obra de la avanzada (varios viajes: el carro admite 500).
    fn entregar_todo(&mut self, av: u64, col: u64) {
        for _ in 0..40 {
            let falta = self.n.avanzada(av).unwrap().1.obra.as_ref().map(|o| o.falta()).unwrap_or_default();
            if falta.is_empty() {
                return;
            }
            for (recurso, cantidad) in falta {
                self.ord("admin", Comando::DarSuministro { ejercito: col, recurso, cantidad });
            }
            self.ok("ana", Comando::EntregarMaterialesAAvanzada { ejercito: col, avanzada: av });
        }
        panic!("la obra no acaba de recibir sus materiales");
    }
    /// Entrega todo y deja pasar la obra hasta su instante exacto.
    fn acabar_obra(&mut self, av: u64, col: u64) {
        self.entregar_todo(av, col);
        let fin = self.avanzada(av).obra.as_ref().and_then(|o| o.completa_en).expect("la obra arrancó");
        self.hasta(fin);
        assert!(self.avanzada(av).obra.is_none());
    }
    /// Coloca una atalaya y la levanta.
    fn atalaya(&mut self) -> (u64, u64, u64) {
        let p = self.sitio();
        let (av, col, heroe) = self.colocar_en(p);
        self.acabar_obra(av, col);
        (av, col, heroe)
    }
    /// Una atalaya mejorada hasta `etapa`.
    fn avanzada_de(&mut self, etapa: Etapa) -> (u64, u64, u64) {
        let (av, col, heroe) = self.atalaya();
        for destino in [Etapa::Fuerte, Etapa::Ciudadela] {
            if self.avanzada(av).etapa >= etapa {
                break;
            }
            if destino == Etapa::Ciudadela {
                self.nivel(3);
            }
            self.ok("ana", Comando::MejorarAvanzada { heroe, avanzada: av });
            self.acabar_obra(av, col);
            assert_eq!(self.avanzada(av).etapa, destino);
        }
        (av, col, heroe)
    }
    fn nivel(&mut self, nivel: u32) {
        let plaza = self.plaza;
        let r = self.ord("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        for _ in 0..14 {
            let r = self.ord("admin", Comando::AgrandarZona { asentamiento: plaza });
            assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        }
    }
    /// Levanta un Barracón en la madre (el tiempo que haga falta): con él se reclutan los lanceros de mimbre.
    fn con_barracon(&mut self) {
        let plaza = self.plaza;
        self.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Barracon });
        let activo = |e: &Esc| e.n.economia(plaza).unwrap().edificios.iter().any(|x| x.tipo == Tipo::Barracon && x.activo());
        for _ in 0..80 {
            if activo(self) {
                break;
            }
            self.avanzar(HORA);
        }
        assert!(activo(self), "el Barracón acaba por levantarse");
    }
    /// Las citas de fin de obra de avanzada que siguen vigentes, por instante.
    fn fines_de_avanzada(&self) -> Vec<Instante> {
        let e = self.n.estado();
        e.cola.volcar().iter().filter(|ev| matches!(ev.carga, Carga::Combate(SucesoCombate::FinObraAvanzada)) && e.generaciones.es_vigente(ev.clave)).map(|ev| ev.instante).collect()
    }
    /// Una columna de Beto (F2) con esas tropas, a `p`: (columna, héroe).
    fn atacante(&mut self, p: Punto, tropas: &[Tropa]) -> (u64, u64) {
        let (col, heroe) = self.columna("beto", 2, p);
        tropas.iter().for_each(|&t| {
            self.dar_escuadra(heroe, t);
        });
        (col, heroe)
    }
    fn guerra(&mut self) {
        self.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    }
    fn atacar(&mut self, heroe: u64, objetivo: ObjetivoAtaque) -> Respuesta {
        self.ord("beto", Comando::Atacar { heroe, objetivo, declara_guerra: false })
    }
}

fn montar(m: &M, semilla: u64, nivel: u32) -> Esc {
    montar_con(Nucleo::nuevo(arranque(m, semilla)), m, nivel)
}

fn montar_con(mut n: Nucleo, m: &M, nivel: u32) -> Esc {
    let mapa = Mapa::de(&m.0);
    let sitio = (0..mapa.nodos.len()).filter(|&i| mapa.nodos[i].tipo == Tipo::MinaCobre).find_map(|i| sitio_junto_a(&n, mapa.nodos[i].pos, 6_000)).expect("hay un sitio rico, lejos de los campamentos");
    let e = n.crear_ejercito(0, "ana", 1, sitio, 500, 100_000).unwrap();
    assert!(n.aplicar_en(0, Peticion::nueva("admin", "c", Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(0, Peticion::nueva("ana", "f", Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    let plaza = r.creadas[0];
    let mut esc = Esc { n, plaza, centro: sitio, cuenta: 0 };
    esc.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
    if nivel > 1 {
        esc.nivel(nivel);
    }
    esc
}

// ───────────────────────────── 0. El acceso se hereda de la plaza madre ─────────────────────────────

#[test]
fn una_avanzada_hereda_el_acceso_de_su_plaza_madre() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let p = e.sitio();
    let (av, _, _) = e.colocar_en(p);
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Alianza) });
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 4, relacion: Some(Relacion::Guerra) });
    let heroes: Vec<u64> = [("pro", 1), ("ali", 2), ("neu", 3), ("ene", 4)].into_iter().map(|(c, f)| e.columna(c, f, p).1).collect();
    let de_la_avanzada = |e: &Esc| heroes.iter().map(|&h| e.n.puede_entrar_en_avanzada(h, av)).collect::<Vec<_>>();
    let de_la_madre = |e: &Esc| heroes.iter().map(|&h| Some(e.n.puede_entrar_en_plaza(h, plaza))).collect::<Vec<_>>();
    assert_eq!(de_la_avanzada(&e), vec![Some(true), Some(true), Some(false), Some(false)], "por defecto (Q1), cerrada a neutrales y enemigos");
    assert_eq!(de_la_avanzada(&e), de_la_madre(&e));
    e.ok("admin", Comando::FijarAcceso { asentamiento: plaza, aliados: false, neutrales: true, enemigos: true });
    assert_eq!(de_la_avanzada(&e), vec![Some(true), Some(false), Some(true), Some(true)]);
    assert_eq!(de_la_avanzada(&e), de_la_madre(&e));
    assert_eq!(e.n.puede_entrar_en_avanzada(heroes[0], 9_999), None, "una avanzada que no existe no tiene puerta");
}

// ───────────────────────────── 1. Colocar ─────────────────────────────

#[test]
fn colocar_deja_una_obra_pendiente_que_ocupa_cupo_y_no_cuesta_nada_ni_se_ve_ni_entra_en_el_espacio() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (col, heroe) = e.columna("ana", 1, pos);
    let (cosas, madera, edificios, v0) = (e.n.cosas_en_el_espacio(), e.almacen(Madera), e.n.economia(plaza).unwrap().edificios.len(), e.n.version());

    let r = e.ok("ana", Comando::ColocarAvanzada { asentamiento: plaza, heroe, posicion: pos });
    let id = r.creadas[0];
    assert_eq!(e.n.version(), v0 + 1, "una orden aceptada sube la versión una vez");
    let (madre, av) = e.n.avanzada(id).unwrap();
    assert_eq!((madre, av.posicion, av.etapa, av.estado), (Some(plaza), pos, Etapa::Atalaya, EstadoAvanzada::EnConstruccion));
    let obra = av.obra.as_ref().unwrap();
    assert_eq!((obra.tipo, &obra.requerido, obra.completa_en), (TipoObra::Construir, &vec![(Madera, 200_000), (Piedra, 100_000)], None), "200 de madera y 100 de piedra, a la espera");
    assert!(obra.entregado.iter().all(|&(_, n)| n == 0) && av.almacen.capacidad(Trigo) == 0);
    // No pasa por la cola de la ciudad, no cobra al almacén, no entra en el espacio (no se ve ni se encuentra) y ocupa cupo.
    assert_eq!((e.n.cosas_en_el_espacio(), e.almacen(Madera), e.n.economia(plaza).unwrap().edificios.len()), (cosas, madera, edificios));
    assert_eq!(e.n.avanzadas_de(plaza).len(), 1);
    assert_eq!(e.n.economia(plaza).unwrap().cupo_de_avanzadas(&e.n.estado().reglas.economia.datos), 2);
    let h = &hechos(&e.n, "avanzada.obra_marcada")[0];
    assert_eq!((dato(h, "avanzada"), dato(h, "tipo"), dato(h, "x"), h.2.clone()), (id as i64, 0, pos.x, vec![plaza]));
    assert!(e.n.eventos_en_cola() > 0 && hechos(&e.n, "avanzada.obra_iniciada").is_empty(), "sin materiales no hay fin de obra");
    assert_eq!(e.n.ejercito(col).unwrap().posicion, pos);
}

#[test]
fn colocar_se_rechaza_sin_rastro_y_en_su_orden() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (_, heroe) = e.columna("ana", 1, pos);
    let (_, beto) = e.columna("beto", 2, pos);
    let colocar = |asentamiento: u64, heroe: u64, posicion: Punto| Comando::ColocarAvanzada { asentamiento, heroe, posicion };
    assert_eq!(e.rechazada("ana", colocar(9_999, heroe, pos)), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert_eq!(e.rechazada("beto", colocar(plaza, beto, pos)), rechazos::NO_AUTORIZADO, "ciudadano de otra Facción");
    assert_eq!(e.rechazada("ana", colocar(plaza, beto, pos)), rechazos::NO_AUTORIZADO, "el héroe es de otra cuenta");
    assert_eq!(e.rechazada("ana", colocar(plaza, 9_999, pos)), rechazos::HEROE_DESCONOCIDO);
    // La columna del héroe tiene que estar en el sitio (a ≤ radioPuerta), la zona manda y el asentamiento no vale.
    assert_eq!(e.rechazada("ana", colocar(plaza, heroe, Punto::new(pos.x + 5_000, pos.y))), rechazos::AVANZADA_FUERA_DEL_SITIO);
    let lejos = Punto::new(e.centro.x + 25_000, e.centro.y);
    let (_, h2) = e.columna("ana", 1, lejos);
    assert_eq!(e.rechazada("ana", colocar(plaza, h2, lejos)), rechazos::AVANZADA_FUERA_DE_LA_ZONA, "la zona de nivel 2 llega a 15 u");
    let dentro = Punto::new(e.centro.x + 1_599, e.centro.y);
    let (_, h3) = e.columna("ana", 1, dentro);
    assert_eq!(e.rechazada("ana", colocar(plaza, h3, dentro)), rechazos::AVANZADA_DENTRO_DEL_ASENTAMIENTO, "la ciudad mide una décima de la zona (1,5) y la avanzada pide su media anchura (0,1) más");
    // Con la plaza de nivel 1 no hay avanzadas.
    let mut uno = montar(m, 7, 1);
    let (p1, centro) = (uno.plaza, uno.centro);
    let dentro_de_zona = Punto::new(centro.x + 4_000, centro.y);
    let (_, h) = uno.columna("ana", 1, dentro_de_zona);
    assert_eq!(uno.rechazada("ana", Comando::ColocarAvanzada { asentamiento: p1, heroe: h, posicion: dentro_de_zona }), rechazos::AVANZADA_NIVEL_INSUFICIENTE);
    // El cupo: dos en el nivel 2, contando las pendientes.
    e.ok("ana", colocar(plaza, heroe, pos));
    let otro = Punto::new(e.centro.x - 8_000, e.centro.y);
    let (_, h4) = e.columna("ana", 1, otro);
    e.ok("ana", colocar(plaza, h4, otro));
    let tercero = Punto::new(e.centro.x, e.centro.y + 8_000);
    let (_, h5) = e.columna("ana", 1, tercero);
    assert_eq!(e.rechazada("ana", colocar(plaza, h5, tercero)), rechazos::AVANZADA_SIN_CUPO);
}

#[test]
fn colocar_respeta_la_ciudad_de_la_madre_y_no_pisa_otro_elemento_del_mapa() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, centro) = (e.plaza, e.centro);
    let colocar = |heroe: u64, posicion: Punto| Comando::ColocarAvanzada { asentamiento: plaza, heroe, posicion };
    // La ciudad mide una décima de la zona (la de nivel 2 son 15: 1,5) y la avanzada pide además su media anchura (0,1): justo fuera de 1,6 sí cabe.
    let primera = Punto::new(centro.x + 1_600, centro.y);
    let (_, h1) = e.columna("ana", 1, primera);
    e.ok("ana", colocar(h1, primera));
    // Otro elemento del mapa mide 0,2: a menos de eso de la primera se pisa (también con ella aún pendiente de materiales); a 0,2 justo, no.
    let pisa = Punto::new(primera.x + 199, primera.y);
    let (_, h2) = e.columna("ana", 1, pisa);
    assert_eq!(e.rechazada("ana", colocar(h2, pisa)), rechazos::AVANZADA_PISA_OTRO_ELEMENTO);
    let cabe = Punto::new(primera.x + 200, primera.y);
    let (_, h3) = e.columna("ana", 1, cabe);
    e.ok("ana", colocar(h3, cabe));
    assert_eq!(e.n.avanzadas_de(plaza).len(), 2);

    // Un pueblo extractor también es un elemento del mapa: sobre su nodo no se coloca, a 0,2 de él sí.
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let cobre = e.n.candidatos_de_pueblo(plaza).into_iter().find(|&(_, t)| t == Tipo::MinaCobre).expect("la plaza se fundó junto a una mina de cobre").0;
    let nodo = Mapa::de(&m.0).nodos[cobre as usize].pos;
    e.ok("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: cobre });
    let encima = Punto::new(nodo.x + 100, nodo.y);
    let (_, h) = e.columna("ana", 1, encima);
    assert_eq!(e.rechazada("ana", Comando::ColocarAvanzada { asentamiento: plaza, heroe: h, posicion: encima }), rechazos::AVANZADA_PISA_OTRO_ELEMENTO);
    let al_lado = Punto::new(nodo.x + 200, nodo.y);
    let (_, h) = e.columna("ana", 1, al_lado);
    e.ok("ana", Comando::ColocarAvanzada { asentamiento: plaza, heroe: h, posicion: al_lado });
}

// ───────────────────────────── 2. Entregar y el fin exacto ─────────────────────────────

#[test]
fn con_todo_entregado_arranca_la_obra_y_acaba_en_su_instante_exacto_con_cita_propia() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let pos = e.sitio();
    let (av, col, _) = e.colocar_en(pos);
    let cosas = e.n.cosas_en_el_espacio();
    // Una entrega parcial toma lo que sirve y no arranca nada.
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 150_000 });
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 7_000 });
    let en_cola = e.n.eventos_en_cola();
    e.ok("ana", Comando::EntregarMaterialesAAvanzada { ejercito: col, avanzada: av });
    let obra = e.avanzada(av).obra.clone().unwrap();
    assert_eq!((obra.falta(), obra.completa_en), (vec![(Madera, 50_000), (Piedra, 100_000)], None));
    assert_eq!(e.n.ejercito(col).unwrap().columna.carga.get(&Oro).copied(), Some(7_000), "lo que no sirve se queda en el carro");
    assert_eq!(e.n.eventos_en_cola(), en_cola, "sin todo entregado no hay cita");
    assert_eq!(dato(&hechos(&e.n, "avanzada.materiales_entregados")[0], "cantidad"), 150_000);
    // El resto, en un instante que no es de minuto.
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Piedra, cantidad: 100_000 });
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 50_000 });
    let t = e.n.ahora() + 17_345;
    e.ok_en(t, "ana", Comando::EntregarMaterialesAAvanzada { ejercito: col, avanzada: av });
    let fin = t + 2 * HORA;
    assert_eq!(e.avanzada(av).obra.as_ref().unwrap().completa_en, Some(fin));
    assert_eq!(e.n.eventos_en_cola(), en_cola + 1, "la obra tiene su cita propia en la cola, sin evento recurrente");
    assert_eq!(dato(&hechos(&e.n, "avanzada.obra_iniciada")[0], "completaEn"), fin);
    // Ni un milisegundo antes; en el exacto, operativa y en el espacio.
    e.n.avanzar_hasta(fin - 1);
    assert_eq!((e.estado_de(av), e.n.cosas_en_el_espacio()), (EstadoAvanzada::EnConstruccion, cosas));
    e.n.avanzar_hasta(fin);
    assert_eq!((e.estado_de(av), e.n.cosas_en_el_espacio()), (EstadoAvanzada::Operativa, cosas + 1));
    let h = &hechos(&e.n, "avanzada.completada")[0];
    assert_eq!((h.0, dato(h, "etapa"), dato(h, "tipo")), (fin, 0, 0));
    assert_eq!(e.n.eventos_en_cola(), en_cola, "la cita se cumplió y no queda nada recurrente por la avanzada ociosa");
    e.n.avanzar_hasta(fin + 12 * HORA);
    assert_eq!(e.n.eventos_en_cola(), en_cola, "una avanzada ociosa no programa nada");
}

#[test]
fn entregar_se_rechaza_sin_rastro() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let pos = e.sitio();
    let (av, col, _) = e.colocar_en(pos);
    let (beto, _) = e.columna("beto", 2, pos);
    let (lejana, _) = e.columna("ana", 1, Punto::new(pos.x + 5_000, pos.y));
    for c in [col, beto, lejana] {
        e.ord("admin", Comando::DarSuministro { ejercito: c, recurso: Madera, cantidad: 10_000 });
    }
    let (vacia, _) = e.columna("ana", 1, pos);
    let entregar = |ejercito: u64, avanzada: u64| Comando::EntregarMaterialesAAvanzada { ejercito, avanzada };
    assert_eq!(e.rechazada("ana", entregar(9_999, av)), rechazos::EJERCITO_DESCONOCIDO);
    assert_eq!(e.rechazada("beto", entregar(col, av)), rechazos::NO_AUTORIZADO, "la columna no es suya");
    assert_eq!(e.rechazada("ana", entregar(col, 9_999)), rechazos::AVANZADA_DESCONOCIDA);
    assert_eq!(e.rechazada("beto", entregar(beto, av)), rechazos::AVANZADA_FACCION_AJENA);
    assert_eq!(e.rechazada("ana", entregar(lejana, av)), rechazos::AVANZADA_FUERA_DE_LA_PUERTA);
    assert_eq!(e.rechazada("ana", entregar(vacia, av)), rechazos::AVANZADA_NADA_QUE_ENTREGAR);
    e.acabar_obra(av, col);
    e.ord("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 10_000 });
    assert_eq!(e.rechazada("ana", entregar(col, av)), rechazos::AVANZADA_NO_ESTA_PENDIENTE, "ya no espera materiales");
}

// ───────────────────────────── 3. Mejorar, reparar y destruir ─────────────────────────────

#[test]
fn mejorar_sigue_funcionando_acaba_en_su_instante_y_sube_de_etapa_con_su_almacen() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (av, col, heroe) = e.atalaya();
    let (_, beto) = e.columna("beto", 2, e.sitio());
    assert_eq!(e.rechazada("beto", Comando::MejorarAvanzada { heroe: beto, avanzada: av }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", Comando::MejorarAvanzada { heroe, avanzada: 9_999 }), rechazos::AVANZADA_DESCONOCIDA);
    e.ok("ana", Comando::MejorarAvanzada { heroe, avanzada: av });
    let obra = e.avanzada(av).obra.clone().unwrap();
    assert_eq!((obra.tipo, obra.requerido), (TipoObra::Mejorar, vec![(Madera, 600_000), (Oro, 50_000), (Piedra, 400_000)]));
    assert_eq!((e.estado_de(av), e.avanzada(av).etapa), (EstadoAvanzada::Operativa, Etapa::Atalaya), "mientras se mejora, sigue funcionando");
    assert_eq!(e.rechazada("ana", Comando::MejorarAvanzada { heroe, avanzada: av }), rechazos::AVANZADA_YA_HAY_OBRA);
    e.entregar_todo(av, col);
    let fin = e.avanzada(av).obra.as_ref().unwrap().completa_en.unwrap();
    assert_eq!(fin - e.n.ahora(), 12 * HORA, "atalaya → fuerte: 12 h");
    e.hasta(fin - 1);
    assert_eq!(e.avanzada(av).etapa, Etapa::Atalaya);
    e.hasta(fin);
    let a = e.avanzada(av);
    assert_eq!((a.etapa, a.operativa(), a.obra.is_none(), a.almacen.capacidad(Trigo)), (Etapa::Fuerte, true, true, 2_000_000));
    let h = &hechos(&e.n, "avanzada.completada")[1];
    assert_eq!((h.0, dato(h, "etapa"), dato(h, "tipo")), (fin, 1, 1));
    // La ciudadela pide el nivel 3 de la madre.
    assert_eq!(e.rechazada("ana", Comando::MejorarAvanzada { heroe, avanzada: av }), rechazos::AVANZADA_NIVEL_INSUFICIENTE);
    e.nivel(3);
    e.ok("ana", Comando::MejorarAvanzada { heroe, avanzada: av });
    e.acabar_obra(av, col);
    let a = e.avanzada(av);
    assert_eq!((a.etapa, a.almacen.capacidad(Madera)), (Etapa::Ciudadela, 10_000_000));
    assert_eq!(e.rechazada("ana", Comando::MejorarAvanzada { heroe, avanzada: av }), rechazos::AVANZADA_NO_MEJORABLE, "la ciudadela no tiene siguiente");
}

#[test]
fn una_avanzada_en_obra_no_se_mejora_y_la_operativa_no_se_repara() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let pos = e.sitio();
    let (av, col, heroe) = e.colocar_en(pos);
    assert_eq!(e.rechazada("ana", Comando::MejorarAvanzada { heroe, avanzada: av }), rechazos::AVANZADA_NO_MEJORABLE);
    assert_eq!(e.rechazada("ana", Comando::RepararAvanzada { heroe, avanzada: av }), rechazos::AVANZADA_NO_REPARABLE);
    e.acabar_obra(av, col);
    assert_eq!(e.rechazada("ana", Comando::RepararAvanzada { heroe, avanzada: av }), rechazos::AVANZADA_NO_REPARABLE);
    assert_eq!(e.rechazada("ana", Comando::RepararAvanzada { heroe, avanzada: 9_999 }), rechazos::AVANZADA_DESCONOCIDA);
}

#[test]
fn destruir_a_mano_libera_el_cupo_el_espacio_y_la_guarnicion_y_no_recupera_nada_y_la_ciudadela_no() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    // Cancelar la primera obra.
    let (pendiente, _, heroe) = e.colocar_en(pos);
    let madera = e.almacen(Madera);
    assert_eq!(e.rechazada("!beto", Comando::DestruirAvanzada { heroe, avanzada: pendiente }), rechazos::NO_AUTORIZADO);
    e.ok("ana", Comando::DestruirAvanzada { heroe, avanzada: pendiente });
    assert!(e.n.avanzada(pendiente).is_none() && e.n.avanzadas_de(plaza).is_empty());
    assert_eq!((e.almacen(Madera), hechos(&e.n, "avanzada.destruida").len()), (madera, 1), "no recupera nada");
    assert_eq!(dato(&hechos(&e.n, "avanzada.destruida")[0], "motivo"), 0, "a mano");
    assert_eq!(e.rechazada("ana", Comando::DestruirAvanzada { heroe, avanzada: pendiente }), rechazos::AVANZADA_DESCONOCIDA);
    // Un fuerte operativo con guarnición de jugadores: sale del espacio y la tropa vuelve intacta a su campamento.
    let (fuerte, _, h) = e.avanzada_de(Etapa::Fuerte);
    e.residir(h);
    let escuadra = e.dar_escuadra(h, Tropa::MiliciaLanceros);
    e.ok("ana", Comando::GuarnecerAvanzada { heroe: h, avanzada: fuerte, escuadras: vec![escuadra] });
    let cosas = e.n.cosas_en_el_espacio();
    e.ok("ana", Comando::DestruirAvanzada { heroe: h, avanzada: fuerte });
    assert_eq!(e.n.cosas_en_el_espacio(), cosas - 1);
    let s = e.n.heroe(h).unwrap().heroe.escuadra(escuadra).unwrap().clone();
    assert_eq!((s.contenedor, s.cantidad), (Contenedor::Campamento, completa(Tropa::MiliciaLanceros)), "vuelve sin viaje y sin bajas: final normal");
    // La ciudadela no se destruye a mano.
    let (ciudadela, _, hc) = e.avanzada_de(Etapa::Ciudadela);
    assert_eq!(e.rechazada("ana", Comando::DestruirAvanzada { heroe: hc, avanzada: ciudadela }), rechazos::AVANZADA_CIUDADELA_NO_SE_DESTRUYE);
}


#[test]
fn cancelar_una_mejora_o_una_reparacion_pierde_lo_entregado_descarta_su_cita_y_deja_la_avanzada_como_estaba() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, col, heroe) = e.colocar_en(pos);
    let cancelar = |heroe: u64, avanzada: u64| Comando::CancelarObraDeAvanzada { heroe, avanzada };
    let (_, beto) = e.columna("beto", 2, pos);
    assert_eq!(e.rechazada("ana", cancelar(heroe, 9_999)), rechazos::AVANZADA_DESCONOCIDA);
    assert_eq!(e.rechazada("!beto", cancelar(beto, av)), rechazos::NO_AUTORIZADO, "ciudadano de otra Facción");
    assert_eq!(e.rechazada("ana", cancelar(heroe, av)), rechazos::AVANZADA_NADA_QUE_CANCELAR, "la primera obra se cancela destruyendo la avanzada");
    e.acabar_obra(av, col);
    assert_eq!(e.rechazada("ana", cancelar(heroe, av)), rechazos::AVANZADA_NADA_QUE_CANCELAR, "operativa y sin obra");
    assert!(e.fines_de_avanzada().is_empty());

    // Una mejora con todo entregado y en marcha: se cancela, no hay reembolso, su cita deja de valer y la atalaya sigue operativa.
    e.ok("ana", Comando::MejorarAvanzada { heroe, avanzada: av });
    e.entregar_todo(av, col);
    let fin = e.avanzada(av).obra.as_ref().and_then(|o| o.completa_en).expect("arrancó");
    assert_eq!(e.fines_de_avanzada(), vec![fin]);
    let (madera, v0) = (e.almacen(Madera), e.n.version());
    e.ok("ana", cancelar(heroe, av));
    assert_eq!(e.n.version(), v0 + 1, "una orden aceptada sube la versión una vez");
    let a = e.avanzada(av);
    assert_eq!((a.obra.is_none(), a.etapa, a.estado), (true, Etapa::Atalaya, EstadoAvanzada::Operativa));
    assert!(e.fines_de_avanzada().is_empty(), "la cita de fin ya no es vigente");
    assert_eq!(e.almacen(Madera), madera, "lo entregado se pierde: ni vuelve a la plaza ni a la columna");
    let h = &hechos(&e.n, "avanzada.obra_cancelada")[0];
    assert_eq!((dato(h, "avanzada"), dato(h, "tipo"), h.2.clone()), (av as i64, 1, vec![plaza]));
    assert_eq!(e.rechazada("ana", cancelar(heroe, av)), rechazos::AVANZADA_NADA_QUE_CANCELAR, "ya no queda nada");
    // Llega el instante del fin cancelado: la cita sale de la cola sin cumplirse.
    let descartados = e.n.eventos_descartados();
    e.hasta(fin + MIN);
    assert_eq!((e.avanzada(av).etapa, hechos(&e.n, "avanzada.completada").len()), (Etapa::Atalaya, 1), "solo la primera obra se completó");
    assert_eq!(e.n.eventos_descartados(), descartados + 1);
    // Se puede volver a mejorar: la cita nueva sí vale y se cumple.
    e.ok("ana", Comando::MejorarAvanzada { heroe, avanzada: av });
    e.acabar_obra(av, col);
    assert_eq!(e.avanzada(av).etapa, Etapa::Fuerte);

    // Una reparación: el fuerte, derribado a ruinas por un ataque, se repara a medias y se abandona; las ruinas siguen siéndolo.
    let (_, atacante) = e.ejercito_fuerte(pos);
    e.guerra();
    assert!(e.atacar(atacante, ObjetivoAtaque::Avanzada(av)).aceptada());
    assert!(e.avanzada(av).en_ruinas());
    e.ok("ana", Comando::RepararAvanzada { heroe, avanzada: av });
    e.entregar_todo(av, col);
    let fin = e.avanzada(av).obra.as_ref().and_then(|o| o.completa_en).expect("arrancó");
    assert_eq!(e.fines_de_avanzada(), vec![fin]);
    e.ok("ana", cancelar(heroe, av));
    assert!(e.avanzada(av).en_ruinas() && e.avanzada(av).obra.is_none() && e.fines_de_avanzada().is_empty());
    assert_eq!(dato(&hechos(&e.n, "avanzada.obra_cancelada")[1], "tipo"), 2);
    e.hasta(fin + MIN);
    assert!(e.avanzada(av).en_ruinas(), "la reparación cancelada no acaba");

    // Perder la avanzada con la obra en marcha también descarta su cita: destruirla a mano, aquí.
    let otro = Punto::new(e.centro.x - 8_000, e.centro.y);
    let (segunda, col2, h2) = e.colocar_en(otro);
    e.acabar_obra(segunda, col2);
    e.ok("ana", Comando::MejorarAvanzada { heroe: h2, avanzada: segunda });
    e.entregar_todo(segunda, col2);
    assert_eq!(e.fines_de_avanzada().len(), 1);
    e.ok("ana", Comando::DestruirAvanzada { heroe: h2, avanzada: segunda });
    assert!(e.fines_de_avanzada().is_empty());
}

// ───────────────────────────── 4. Elegir tropa ─────────────────────────────

#[test]
fn el_gobernador_elige_la_tropa_de_cada_escalon_entre_las_que_tiene_la_faccion() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (av, _, heroe) = e.avanzada_de(Etapa::Fuerte);
    let elegir = |tropa: Tropa| Comando::ElegirTropaDeAvanzada { heroe, avanzada: av, tropa };
    assert_eq!(e.rechazada("ana", Comando::ElegirTropaDeAvanzada { heroe, avanzada: 9_999, tropa: Tropa::LancerosMimbre }), rechazos::AVANZADA_DESCONOCIDA);
    assert_eq!(e.rechazada("!beto", elegir(Tropa::LancerosMimbre)), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", elegir(Tropa::Mineros)), rechazos::AVANZADA_TROPA_NO_ELEGIBLE, "la leva (escalón 1) no defiende avanzadas");
    assert_eq!(e.rechazada("ana", elegir(Tropa::Hequetai)), rechazos::AVANZADA_TROPA_NO_ELEGIBLE, "ni la élite (escalón 5)");
    assert_eq!(e.rechazada("ana", elegir(Tropa::LancerosMimbre)), rechazos::AVANZADA_TROPA_SIN_TECNOLOGIA);
    e.sin_cambios("ana", elegir(Tropa::MiliciaLanceros));
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] });
    // Solo entre lo que la madre puede reclutar: con la tecnología pero sin el Barracón, los lanceros de mimbre no se eligen.
    assert_eq!(e.rechazada("ana", elegir(Tropa::LancerosMimbre)), rechazos::AVANZADA_TROPA_SIN_EDIFICIO);
    e.con_barracon();
    e.ok("ana", elegir(Tropa::LancerosMimbre));
    assert_eq!(e.n.tropas_elegidas_de_avanzada(av).get(&2), Some(&Tropa::LancerosMimbre));
    let h = &hechos(&e.n, "avanzada.tropa_elegida")[0];
    assert_eq!((dato(h, "escalon"), dato(h, "avanzada")), (2, av as i64));
    e.sin_cambios("ana", elegir(Tropa::LancerosMimbre));
    // La guarnición de serie nace la primera vez que se la necesita, con lo elegido para el escalón 2 (3 escuadras) y el defecto para el 3 (2).
    let (_, beto) = e.atacante(Punto::new(e.sitio().x + 1_000, e.sitio().y), &[Tropa::Lenadores]);
    e.guerra();
    assert!(e.n.guarnicion_de_serie_de_avanzada(av).is_empty(), "no hay nada hasta el primer ataque");
    e.ok("beto", Comando::Atacar { heroe: beto, objetivo: ObjetivoAtaque::Avanzada(av), declara_guerra: false });
    let tropas: Vec<Tropa> = e.n.guarnicion_de_serie_de_avanzada(av).iter().map(|s| s.tropa).collect();
    assert_eq!(tropas, vec![Tropa::LancerosMimbre, Tropa::LancerosMimbre, Tropa::LancerosMimbre, Tropa::MiliciaLanceros, Tropa::MiliciaLanceros]);
}

// ───────────────────────────── 5. La guarnición de jugadores ─────────────────────────────

#[test]
fn guarnecer_deja_escuadras_en_la_avanzada_dentro_del_cupo_comen_de_la_madre_y_se_recogen() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, col, heroe) = e.colocar_en(pos);
    e.residir(heroe);
    let a = e.dar_escuadra(heroe, Tropa::MiliciaLanceros);
    let b = e.dar_escuadra(heroe, Tropa::MiliciaLanceros);
    let guarnecer = |escuadras: Vec<u64>| Comando::GuarnecerAvanzada { heroe, avanzada: av, escuadras };
    assert_eq!(e.rechazada("ana", guarnecer(vec![a])), rechazos::AVANZADA_NO_OPERATIVA, "una obra no tiene guarnición");
    e.acabar_obra(av, col);
    assert_eq!(e.n.cupo_de_guarnicion_de_avanzada(av), Some(14), "atalaya: 14 puntos de Liderazgo por héroe residente");
    assert_eq!(e.rechazada("ana", guarnecer(vec![])), rechazos::SALIDA_SIN_ESCUADRAS);
    assert_eq!(e.rechazada("ana", guarnecer(vec![a, a])), rechazos::ESCUADRA_REPETIDA);
    assert_eq!(e.rechazada("ana", guarnecer(vec![9_999])), rechazos::ESCUADRA_AJENA);
    assert_eq!(e.rechazada("ana", guarnecer(vec![a, b])), rechazos::GUARNICION_SUPERA_EL_CUPO, "dos milicias son 28 puntos y el cupo es 14");
    let (_, h_lejos) = e.columna("ana", 1, Punto::new(pos.x + 6_000, pos.y));
    e.residir(h_lejos);
    let c = e.dar_escuadra(h_lejos, Tropa::MiliciaLanceros);
    assert_eq!(e.rechazada("ana", Comando::GuarnecerAvanzada { heroe: h_lejos, avanzada: av, escuadras: vec![c] }), rechazos::AVANZADA_FUERA_DE_LA_PUERTA);
    let (_, visitante) = e.columna("ana", 1, pos);
    let v = e.dar_escuadra(visitante, Tropa::MiliciaLanceros);
    assert_eq!(e.rechazada("ana", Comando::GuarnecerAvanzada { heroe: visitante, avanzada: av, escuadras: vec![v] }), rechazos::AVANZADA_NO_RESIDES, "solo los residentes de la madre");

    let soldados = e.n.soldados_del_campamento(plaza);
    let antes = e.n.velocidad_de_ejercito(col).unwrap();
    e.ok("ana", guarnecer(vec![a]));
    let s = e.n.heroe(heroe).unwrap().heroe.escuadra(a).unwrap().clone();
    assert_eq!(s.contenedor, Contenedor::Avanzada(av));
    assert!(!e.n.ejercito(col).unwrap().columna.escuadras.contains(&a) && e.n.ejercito(col).unwrap().columna.escuadras.contains(&b));
    assert_eq!(e.n.guarnicion_de_jugadores_de_avanzada(av).len(), 1);
    assert_eq!(e.n.soldados_del_campamento(plaza), soldados + completa(Tropa::MiliciaLanceros), "come del trigo de la madre, como su campamento");
    assert!(e.n.velocidad_de_ejercito(col).unwrap() >= antes, "la columna deja de cargar con esa escuadra");
    assert_eq!(e.rechazada("ana", guarnecer(vec![b])), rechazos::GUARNICION_SUPERA_EL_CUPO, "el cupo ya está lleno");
    assert_eq!(dato(&hechos(&e.n, "avanzada.guarnecida")[0], "liderazgo"), 14);
    e.avanzar(30 * MIN);
    assert!(e.n.heroe(heroe).unwrap().heroe.escuadra(a).unwrap().cantidad > 0, "sigue comiendo y sin desertar");

    // Recogerla: la columna a la puerta, escuadras que están allí y dentro del Liderazgo del héroe.
    assert_eq!(e.rechazada("ana", Comando::RetirarDeAvanzada { heroe, avanzada: av, escuadras: vec![b] }), rechazos::AVANZADA_ESCUADRA_NO_ESTA);
    assert_eq!(e.rechazada("ana", Comando::RetirarDeAvanzada { heroe: h_lejos, avanzada: av, escuadras: vec![c] }), rechazos::AVANZADA_FUERA_DE_LA_PUERTA);
    e.ok("ana", Comando::RetirarDeAvanzada { heroe, avanzada: av, escuadras: vec![a] });
    assert_eq!(e.n.heroe(heroe).unwrap().heroe.escuadra(a).unwrap().contenedor, Contenedor::Ejercito(col));
    assert!(e.n.ejercito(col).unwrap().columna.escuadras.contains(&a) && e.n.guarnicion_de_jugadores_de_avanzada(av).is_empty());
    assert_eq!(e.n.soldados_del_campamento(plaza), soldados);
    // Al mudarse de residencia la guarnición vuelve sola al campamento, intacta.
    e.ok("ana", guarnecer(vec![a]));
    e.ok("admin", Comando::FijarResidencia { heroe, residencia: None });
    let s = e.n.heroe(heroe).unwrap().heroe.escuadra(a).unwrap();
    assert_eq!((s.contenedor, s.cantidad), (Contenedor::Campamento, completa(Tropa::MiliciaLanceros)));
}

#[test]
fn el_cupo_crece_con_la_etapa_y_cada_heroe_tiene_el_suyo() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (av, _, heroe) = e.avanzada_de(Etapa::Fuerte);
    e.residir(heroe);
    assert_eq!(e.n.cupo_de_guarnicion_de_avanzada(av), Some(45));
    let milicias: Vec<u64> = (0..4).map(|_| e.dar_escuadra(heroe, Tropa::MiliciaLanceros)).collect();
    e.ok("ana", Comando::GuarnecerAvanzada { heroe, avanzada: av, escuadras: milicias[..3].to_vec() });
    assert_eq!(e.rechazada("ana", Comando::GuarnecerAvanzada { heroe, avanzada: av, escuadras: vec![milicias[3]] }), rechazos::GUARNICION_SUPERA_EL_CUPO, "tres milicias son 42 de 45");
    // Otro héroe residente tiene su propio cupo en la misma avanzada.
    let (_, otro) = e.columna("ana", 1, e.sitio());
    e.residir(otro);
    let suya = e.dar_escuadra(otro, Tropa::MiliciaLanceros);
    e.ok("ana", Comando::GuarnecerAvanzada { heroe: otro, avanzada: av, escuadras: vec![suya] });
    assert_eq!(e.n.guarnicion_de_jugadores_de_avanzada(av).len(), 4);
}

// ───────────────────────────── 6. El almacén ─────────────────────────────

#[test]
fn el_almacen_se_llena_con_los_carros_y_un_ejercito_reposta_en_su_puerta() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (atalaya, col, heroe) = e.atalaya();
    e.ord("admin", Comando::DarSuministro { ejercito: col, recurso: Trigo, cantidad: 100_000 });
    assert_eq!(e.rechazada("ana", Comando::DepositarEnAvanzada { ejercito: col, avanzada: atalaya, recurso: Trigo, cantidad: 100_000 }), rechazos::AVANZADA_SIN_ALMACEN, "la atalaya no tiene almacén");
    assert_eq!(e.rechazada("ana", Comando::RepostarEnAvanzada { heroe, avanzada: atalaya }), rechazos::AVANZADA_SIN_ALMACEN);
    e.ok("ana", Comando::MejorarAvanzada { heroe, avanzada: atalaya });
    e.acabar_obra(atalaya, col);
    let fuerte = atalaya;
    e.ord("admin", Comando::DarSuministro { ejercito: col, recurso: Trigo, cantidad: 400_000 });
    let deposita = |cantidad: i64| Comando::DepositarEnAvanzada { ejercito: col, avanzada: fuerte, recurso: Trigo, cantidad };
    assert_eq!(e.rechazada("ana", deposita(0)), rechazos::CANTIDAD_NO_POSITIVA);
    assert_eq!(e.rechazada("beto", deposita(10_000)), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", Comando::DepositarEnAvanzada { ejercito: col, avanzada: fuerte, recurso: Piedra, cantidad: 1 }), rechazos::AVANZADA_NADA_QUE_DEPOSITAR);
    let (lejana, _) = e.columna("ana", 1, Punto::new(e.sitio().x + 6_000, e.sitio().y));
    e.ord("admin", Comando::DarSuministro { ejercito: lejana, recurso: Trigo, cantidad: 10_000 });
    assert_eq!(e.rechazada("ana", Comando::DepositarEnAvanzada { ejercito: lejana, avanzada: fuerte, recurso: Trigo, cantidad: 10_000 }), rechazos::AVANZADA_FUERA_DE_LA_PUERTA);
    let en_carro = |e: &Esc| e.n.ejercito(col).unwrap().columna.carga.get(&Trigo).copied().unwrap_or(0);
    let antes = en_carro(&e);
    e.ok("ana", deposita(300_000));
    assert_eq!((e.avanzada(fuerte).almacen.cantidad(Trigo), en_carro(&e)), (300_000, antes - 300_000));
    assert_eq!(dato(&hechos(&e.n, "avanzada.deposito")[0], "cantidad"), 300_000);
    // El tope por recurso: caben 2.000.
    for _ in 0..4 {
        e.ord("admin", Comando::DarSuministro { ejercito: col, recurso: Trigo, cantidad: 500_000 });
        e.ord("ana", Comando::DepositarEnAvanzada { ejercito: col, avanzada: fuerte, recurso: Trigo, cantidad: 500_000 });
    }
    assert_eq!(e.avanzada(fuerte).almacen.cantidad(Trigo), 2_000_000);
    e.ord("admin", Comando::DarSuministro { ejercito: col, recurso: Trigo, cantidad: 10_000 });
    assert_eq!(e.rechazada("ana", deposita(10_000)), rechazos::AVANZADA_ALMACEN_LLENO);
    // Repostar: la columna del Líder, a la puerta, llena sus víveres con el trigo del almacén.
    let (reposta, h2) = e.columna("ana", 1, e.sitio());
    let (viveres, cap) = {
        let v = &e.n.ejercito(reposta).unwrap().columna.viveres;
        (v.valor_en(e.n.ahora()), v.maximo.unwrap())
    };
    assert_eq!(e.rechazada("ana", Comando::RepostarEnAvanzada { heroe: h2, avanzada: 9_999 }), rechazos::AVANZADA_DESCONOCIDA);
    assert_eq!(e.rechazada("beto", Comando::RepostarEnAvanzada { heroe: h2, avanzada: fuerte }), rechazos::NO_AUTORIZADO);
    let (_, ajeno) = e.columna("beto", 2, e.sitio());
    assert_eq!(e.rechazada("beto", Comando::RepostarEnAvanzada { heroe: ajeno, avanzada: fuerte }), rechazos::AVANZADA_FACCION_AJENA, "el almacén es de la Facción");
    e.ok("ana", Comando::RepostarEnAvanzada { heroe: h2, avanzada: fuerte });
    let t = e.n.ahora();
    assert_eq!(e.n.ejercito(reposta).unwrap().columna.viveres.valor_en(t), cap);
    assert_eq!(e.avanzada(fuerte).almacen.cantidad(Trigo), 2_000_000 - (cap - viveres));
    assert_eq!(dato(&hechos(&e.n, "avanzada.reabastecimiento")[0], "repuesto"), cap - viveres);
    assert_eq!(e.rechazada("ana", Comando::RepostarEnAvanzada { heroe: h2, avanzada: fuerte }), rechazos::COLUMNA_VIVERES_LLENOS);
}

// ───────────────────────────── 7. Atacar una avanzada con números ─────────────────────────────

impl Esc {
    /// Un fuerte operativo de Ana con su héroe residente en la plaza, sin ninguna columna suya en el mapa: lo deja con 300 de trigo y 150 de madera en el almacén.
    fn fuerte_guarnecido(&mut self) -> (u64, u64, u64) {
        let plaza = self.plaza;
        let (av, col, heroe) = self.avanzada_de(Etapa::Fuerte);
        self.residir(heroe);
        let escuadra = self.dar_escuadra(heroe, Tropa::MiliciaLanceros);
        self.ok("ana", Comando::GuarnecerAvanzada { heroe, avanzada: av, escuadras: vec![escuadra] });
        for (recurso, cantidad) in [(Trigo, 300_000), (Madera, 150_000)] {
            self.ord("admin", Comando::DarSuministro { ejercito: col, recurso, cantidad });
            self.ok("ana", Comando::DepositarEnAvanzada { ejercito: col, avanzada: av, recurso, cantidad });
        }
        self.ok("admin", Comando::Guarnecer { asentamiento: plaza, heroe });
        (av, heroe, escuadra)
    }
    /// Un ejército fuerte de Beto junto a la avanzada.
    fn ejercito_fuerte(&mut self, junto_a: Punto) -> (u64, u64) {
        let p = Punto::new(junto_a.x + 1_000, junto_a.y);
        self.atacante(p, &[Tropa::EspadachinesBronce, Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto, Tropa::ArquerosCompuesto, Tropa::HacherosArmados])
    }
}

#[test]
fn un_ejercito_que_gana_a_un_fuerte_se_lleva_el_almacen_que_le_cabe_lo_deja_en_ruinas_invulnerable_un_dia_y_luego_lo_destruye() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, heroe, escuadra) = e.fuerte_guarnecido();
    let (col, beto) = e.ejercito_fuerte(pos);
    e.guerra();
    let (cosas, cupo, v0) = (e.n.cosas_en_el_espacio(), e.n.avanzadas_de(plaza).len(), e.n.version());
    let almacen_antes: i64 = [Trigo, Madera].iter().map(|&r| e.avanzada(av).almacen.cantidad(r)).sum();
    let (r0, t) = (e.n.estado().rng.estado(), e.n.ahora());
    let r = e.atacar(beto, ObjetivoAtaque::Avanzada(av));
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(e.n.version(), v0 + 1);
    assert_eq!(tiradas(r0, e.n.estado().rng.estado()), 2, "un combate con defensores gasta dos tiradas");
    // Ruinas con un día de invulnerabilidad; el espacio y el cupo siguen; la guarnición de jugadores vuelve a 0 a su campamento.
    let a = e.avanzada(av);
    assert_eq!((a.etapa, a.estado), (Etapa::Fuerte, EstadoAvanzada::EnRuinas { invulnerable_hasta: Some(t + DIA) }));
    assert_eq!((e.n.cosas_en_el_espacio(), e.n.avanzadas_de(plaza).len()), (cosas, cupo));
    let s = e.n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap();
    assert_eq!((s.contenedor, s.cantidad, s.en_guarnicion), (Contenedor::Campamento, 0, false), "queda a 0 y vuelve al campamento de su héroe");
    assert!(e.n.guarnicion_de_serie_de_avanzada(av).is_empty() && e.n.guarnicion_de_jugadores_de_avanzada(av).is_empty(), "en ruinas no hay guarnición");
    // El atacante se llevó el almacén (450 caben en su carro de 500) y la chatarra de las bajas llena lo que sobra.
    let h = &hechos(&e.n, "combate.avanzada_en_ruinas")[0];
    let en_carro: i64 = e.n.ejercito(col).unwrap().columna.carga.values().sum();
    assert_eq!((dato(h, "saqueado"), en_carro), (almacen_antes, almacen_antes + dato(h, "chatarra")));
    assert_eq!([Trigo, Madera].iter().map(|&r| e.avanzada(av).almacen.cantidad(r)).sum::<i64>(), 0);
    assert_eq!((h.0, h.1.clone(), h.2.clone(), dato(h, "avanzada")), (t, vec![col], vec![plaza], av as i64));
    let h = &hechos(&e.n, "avanzada.en_ruinas")[0];
    assert_eq!((h.0, dato(h, "invulnerableHasta")), (t, t + DIA));
    // Invulnerable: otro ataque se rechaza sin rastro ni tirada.
    let antes = rastro(&e.n);
    let r = e.atacar(beto, ObjetivoAtaque::Avanzada(av));
    assert_eq!((codigo(&r), rastro(&e.n)), (rechazos::COMBATE_AVANZADA_INVULNERABLE, antes));
    // La invulnerabilidad acaba en su instante exacto, con su cita.
    e.hasta(t + DIA - 1);
    assert_eq!(e.avanzada(av).estado, EstadoAvanzada::EnRuinas { invulnerable_hasta: Some(t + DIA) });
    assert!(hechos(&e.n, "avanzada.invulnerabilidad_terminada").is_empty());
    e.hasta(t + DIA);
    assert_eq!(e.avanzada(av).estado, EstadoAvanzada::EnRuinas { invulnerable_hasta: None });
    assert_eq!(hechos(&e.n, "avanzada.invulnerabilidad_terminada")[0].0, t + DIA);
    // Pasado el día, otro ejército (el primero lleva un día sin ración) que gana la destruye: sin defensores no hay combate ni tirada.
    let (_, segundo) = e.ejercito_fuerte(pos);
    let (r0, en_el_espacio) = (e.n.estado().rng.estado(), e.n.cosas_en_el_espacio());
    let r = e.atacar(segundo, ObjetivoAtaque::Avanzada(av));
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(e.n.estado().rng.estado(), r0, "unas ruinas no tienen defensores: ni combate ni tirada");
    assert!(e.n.avanzada(av).is_none() && e.n.avanzadas_de(plaza).is_empty());
    assert_eq!(e.n.cosas_en_el_espacio(), en_el_espacio - 1, "destruida sale del espacio");
    let h = &hechos(&e.n, "avanzada.destruida")[0];
    assert_eq!(dato(h, "motivo"), 1, "por un ataque");
    assert_eq!(hechos(&e.n, "combate.avanzada_destruida").len(), 1);
    assert_eq!(hechos(&e.n, "avanzada.destruida").len(), 1);
}

#[test]
fn la_atalaya_cae_destruida_y_su_cupo_se_libera() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, _, _) = e.atalaya();
    let (_, beto) = e.ejercito_fuerte(pos);
    e.guerra();
    let cosas = e.n.cosas_en_el_espacio();
    assert!(e.atacar(beto, ObjetivoAtaque::Avanzada(av)).aceptada());
    assert!(e.n.avanzada(av).is_none() && e.n.avanzadas_de(plaza).is_empty());
    assert_eq!(e.n.cosas_en_el_espacio(), cosas - 1);
    assert_eq!(hechos(&e.n, "combate.avanzada_destruida").len(), 1);
    assert_eq!(e.n.economia(plaza).unwrap().cupo_de_avanzadas(&e.n.estado().reglas.economia.datos) as usize - e.n.avanzadas_de(plaza).len(), 2);
}

#[test]
fn si_resiste_la_guarnicion_de_serie_queda_mermada_y_se_repone_gratis_soldado_a_soldado() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, _, _) = e.avanzada_de(Etapa::Fuerte);
    let (col, beto) = e.atacante(Punto::new(pos.x + 1_000, pos.y), &[Tropa::Lenadores]);
    e.guerra();
    let t = e.n.ahora();
    let r = e.atacar(beto, ObjetivoAtaque::Avanzada(av));
    assert!(r.aceptada(), "{r:?}");
    let h = &hechos(&e.n, "combate.avanzada_resiste")[0];
    assert_eq!((h.0, h.1.clone(), h.2.clone()), (t, vec![col], vec![plaza]));
    assert!(e.n.heroe_herido(beto), "los participantes del ejército derrotado quedan heridos");
    assert_eq!(e.avanzada(av).estado, EstadoAvanzada::Operativa);
    let completa_serie = |e: &Esc| e.n.guarnicion_de_serie_de_avanzada(av).iter().all(|s| s.cantidad == completa(s.tropa));
    assert_eq!(e.n.guarnicion_de_serie_de_avanzada(av).len(), 5);
    assert!(!completa_serie(&e), "los lenadores mataron a alguien");
    let g = &e.n.estado().combate.avanzadas[&av];
    // Cinco escuadras de 25: un soldado cada 6 h / 125, desde la baja (Combate §7.4, como los mineros de un pueblo).
    let paso = 6 * HORA / 125;
    let faltan: i64 = g.escuadras.iter().map(|s| completa(s.tropa) - s.cantidad).sum();
    let soldados = |e: &Esc| e.n.guarnicion_de_serie_de_avanzada(av).iter().map(|s| s.cantidad).sum::<i64>();
    assert_eq!(g.repone_en, Some(t + paso));
    assert!(g.escuadras.iter().any(|s| s.experiencia > 0), "la guarnición también gana experiencia");
    let antes = soldados(&e);
    e.hasta(t + paso - 1);
    assert_eq!(soldados(&e), antes, "ni un milisegundo antes");
    e.hasta(t + paso);
    assert_eq!((soldados(&e), e.n.estado().combate.avanzadas[&av].repone_en), (antes + 1, Some(t + 2 * paso)), "vuelve uno y se cita el siguiente");
    e.hasta(t + faltan * paso - 1);
    assert!(!completa_serie(&e) && hechos(&e.n, "avanzada.guarnicion_repuesta").is_empty());
    e.hasta(t + faltan * paso);
    assert!(completa_serie(&e) && e.n.estado().combate.avanzadas[&av].repone_en.is_none());
    assert_eq!(hechos(&e.n, "avanzada.guarnicion_repuesta")[0].0, t + faltan * paso);
}

#[test]
fn atacar_una_avanzada_se_rechaza_sin_rastro_y_en_su_orden() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let pos = e.sitio();
    let (pendiente, col, heroe) = e.colocar_en(pos);
    let (_, beto) = e.ejercito_fuerte(pos);
    let (_, sin_tropa) = e.columna("beto", 2, Punto::new(pos.x + 1_500, pos.y));
    let (_, lejos_h) = e.atacante(Punto::new(pos.x + 20_000, pos.y), &[Tropa::MiliciaLanceros]);
    let atacar = |heroe: u64, id: u64, guerra: bool| Comando::Atacar { heroe, objetivo: ObjetivoAtaque::Avanzada(id), declara_guerra: guerra };
    assert_eq!(e.rechazada("beto", atacar(beto, 9_999, false)), rechazos::COMBATE_AVANZADA_NO_EXISTE);
    assert_eq!(e.rechazada("beto", atacar(beto, pendiente, false)), rechazos::COMBATE_AVANZADA_EN_OBRA, "una obra pendiente se ve pero no se ataca");
    e.acabar_obra(pendiente, col);
    assert_eq!(e.rechazada("ana", atacar(heroe, pendiente, false)), rechazos::COMBATE_PROPIA, "los tuyos no");
    assert_eq!(e.rechazada("beto", atacar(beto, pendiente, false)), rechazos::GUERRA_CONFIRMAR, "sin guerra se rechaza y no la declara");
    assert_eq!(e.rechazada("beto", atacar(lejos_h, pendiente, true)), rechazos::COMBATE_LEJOS);
    assert_eq!(e.rechazada("beto", atacar(sin_tropa, pendiente, true)), rechazos::COMBATE_SIN_ATACANTES, "un héroe solo no ataca una avanzada");
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Vasallaje) });
    assert_eq!(e.rechazada("beto", atacar(beto, pendiente, true)), rechazos::COMBATE_VASALLAJE, "ni con declaración se ataca a la propia señora (5D)");
}

#[test]
fn declarar_la_guerra_al_atacar_una_avanzada_solo_ocurre_si_todo_lo_demas_vale() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let pos = e.sitio();
    let (av, _, _) = e.atalaya();
    let (_, beto) = e.ejercito_fuerte(pos);
    assert!(e.n.estado().relacion(1, 2).is_none());
    e.ok("beto", Comando::Atacar { heroe: beto, objetivo: ObjetivoAtaque::Avanzada(av), declara_guerra: true });
    assert_eq!(e.n.estado().relacion(1, 2), Some(Relacion::Guerra));
    assert_eq!(hechos(&e.n, "diplomacia.guerra_declarada").len(), 1);
}

#[test]
fn una_ciudadela_nunca_se_destruye_y_el_atacante_sigue_llevandose_lo_que_queda_hasta_vaciarla() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 3);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, col, _) = e.avanzada_de(Etapa::Ciudadela);
    for _ in 0..3 {
        e.ord("admin", Comando::DarSuministro { ejercito: col, recurso: Trigo, cantidad: 400_000 });
        e.ok("ana", Comando::DepositarEnAvanzada { ejercito: col, avanzada: av, recurso: Trigo, cantidad: 400_000 });
    }
    assert_eq!(e.avanzada(av).almacen.cantidad(Trigo), 1_200_000);
    let (_, beto) = e.atacante(Punto::new(pos.x + 1_000, pos.y), &[Tropa::EspadachinesBronce, Tropa::EspadachinesBronce, Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto, Tropa::ArquerosCompuesto, Tropa::ArquerosCompuesto, Tropa::HacherosArmados, Tropa::HacherosArmados]);
    e.guerra();
    let r = e.atacar(beto, ObjetivoAtaque::Avanzada(av));
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(e.avanzada(av).estado, EstadoAvanzada::EnRuinas { invulnerable_hasta: None }, "una ciudadela no tiene invulnerabilidad");
    assert_eq!(e.n.avanzadas_de(plaza).len(), 1, "y no se destruye");
    assert_eq!(hechos(&e.n, "combate.avanzada_en_ruinas").len(), 1);
    let con_trigo = |e: &Esc| e.avanzada(av).almacen.cantidad(Trigo);
    let tras_primero = con_trigo(&e);
    assert_eq!(tras_primero, 1_200_000 - 500_000, "el carro de un héroe admite 500");
    // Atacar unas ruinas de ciudadela: sin defensores ni tirada; cada ejército se lleva lo que le cabe y la ciudadela no se destruye ni queda en otro estado.
    let r0 = e.n.estado().rng.estado();
    let (_, segundo) = e.atacante(Punto::new(pos.x + 1_000, pos.y), &[Tropa::EspadachinesBronce]);
    assert!(e.atacar(segundo, ObjetivoAtaque::Avanzada(av)).aceptada());
    assert_eq!((e.n.estado().rng.estado(), con_trigo(&e)), (r0, tras_primero - 500_000), "el segundo ejército se lleva otros 500 sin tirada");
    let (_, tercero) = e.atacante(Punto::new(pos.x + 1_000, pos.y), &[Tropa::EspadachinesBronce]);
    assert!(e.atacar(tercero, ObjetivoAtaque::Avanzada(av)).aceptada());
    assert_eq!(con_trigo(&e), 0, "y el tercero, los 200 que quedaban");
    assert_eq!(hechos(&e.n, "combate.avanzada_saqueada").len(), 2);
    assert_eq!(e.avanzada(av).estado, EstadoAvanzada::EnRuinas { invulnerable_hasta: None });
    // Vacía y en ruinas ya no hay nada que dar ni que derribar: la orden no cambia el mundo.
    let (_, cuarto) = e.atacante(Punto::new(pos.x + 1_000, pos.y), &[Tropa::EspadachinesBronce]);
    let antes = rastro(&e.n);
    let r = e.atacar(cuarto, ObjetivoAtaque::Avanzada(av));
    assert_eq!((r.resultado, rastro(&e.n)), (Resultado::SinCambios, antes));
    assert_eq!(e.n.avanzadas_de(plaza).len(), 1);
}

// ───────────────────────────── 8. Las batallas de Unity ─────────────────────────────

fn arranque_unity(m: &M, semilla: u64) -> Arranque {
    let mut a = arranque(m, semilla);
    a.reglas.batallas_en_unity = true;
    a
}

fn msg(n: &mut Nucleo, t: Instante, id: &str, comando: Comando) -> Respuesta {
    n.aplicar_en(t, Peticion::de_servidor_de_batalla("srv", id, comando))
}

fn en_curso(n: &mut Nucleo, id: u64, t: Instante, humanos: &[u64]) {
    let revision = n.batalla(id).unwrap().ticket.revision;
    let asignacion = Asignacion {
        intento: "a1".into(),
        servidor: "ignorado".into(),
        revision,
        instancia: Instancia { host: "10.0.0.1".into(), puerto: 7777, protocolo: "udp".into() },
        tokens: humanos.iter().map(|&heroe| Token { heroe, token: format!("t{heroe}"), expira_en: 9_999_999_999 }).collect(),
    };
    assert!(msg(n, t, &format!("as-{id}"), Comando::RegistrarAsignacion { batalla: id, asignacion }).aceptada());
    assert!(msg(n, t, &format!("in-{id}"), Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision }).aceptada());
    assert_eq!(n.batalla(id).unwrap().estado, EstadoBatalla::EnCurso);
}

/// Un resultado válido: de cada escuadra mueren `muertos` y el resto sobrevive; gana `ganador`; todos los héroes participan.
fn resultado(n: &Nucleo, id: u64, ganador: Lado, muertos: i64) -> ResultadoDeBatalla {
    let b = n.batalla(id).unwrap();
    ResultadoDeBatalla {
        resultado_id: format!("res-{id}"),
        revision: b.ticket.revision,
        intento: "a1".into(),
        inicio: 1_000,
        fin: 61_000,
        ganador,
        razon: Razon::ObjetivosCapturados,
        objetivos: vec![("bandera".into(), Some(ganador))],
        por_escuadra: b
            .escuadras_de()
            .map(|e| {
                let m = muertos.min(e.efectivos);
                ResultadoEscuadra { escuadra: e.id, desplegados: e.efectivos, supervivientes_al_cierre: e.efectivos - m, muertos: m, xp_ganada: 4 }
            })
            .collect(),
        por_heroe: b.heroes_que_combaten().map(|h| ResultadoHeroe { heroe: h, participo: true, sobrevivio_al_cierre: true, xp_ganada: 7, derrotado: None, botin: None }).collect(),
        version_servidor: "v1".into(),
        carros_perdidos: vec![],
    }
}

fn humano(e: &mut Esc, heroe: u64) {
    e.ok("admin", Comando::FijarControlador { heroe, controlador: Controlador::Humano });
}

#[test]
fn con_un_humano_el_ataque_abre_una_batalla_de_unity_sin_convocatoria_con_capacidad_de_asedio_y_bloquea_la_avanzada() {
    let m = con_mundo!();
    let mut e = montar_con(Nucleo::nuevo(arranque_unity(m, 7)), m, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, heroe, escuadra) = e.fuerte_guarnecido();
    let (col, beto) = e.ejercito_fuerte(pos);
    e.guerra();
    humano(&mut e, beto);
    let t = e.n.ahora();
    let r0 = e.n.estado().rng.estado();
    let r = e.atacar(beto, ObjetivoAtaque::Avanzada(av));
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(e.n.estado().rng.estado(), r0, "Unity decide lo de dentro: el servidor no tira");
    let id = e.n.batallas().next().expect("se abrió una batalla").id;
    let b = e.n.batalla(id).unwrap();
    assert!(matches!(b.ticket.contexto, combate::ciclo::Contexto::Avanzada { avanzada, etapa: Etapa::Fuerte, .. } if avanzada == av));
    assert_eq!((b.convocatoria_hasta, b.ticket.atacante.capacidad, b.ticket.defensor.capacidad), (None, 15, 15), "como un asedio, pero sin convocatoria");
    assert!(b.pendiente_de_asignar(t), "sale hacia Unity al abrirse");
    assert_eq!((b.bloqueo.avanzada, b.bloqueo.ejercitos.clone()), (Some(av), vec![col]));
    // Defienden la guarnición de serie (sin dueño) y la de jugadores (con su dueño), sin participantes.
    let defensor = &b.ticket.defensor;
    assert_eq!((defensor.faccion, defensor.participantes.len(), defensor.sin_heroe.len()), (Some(1), 0, 6));
    assert_eq!(defensor.sin_heroe.iter().filter(|s| s.dueno.is_none()).count(), 5);
    assert!(defensor.sin_heroe.iter().any(|s| s.id == escuadra && s.dueno == Some(heroe)));
    assert_eq!(e.n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap().reserva_batalla, Some(id), "el candado");
    assert_eq!(dato(&hechos(&e.n, "batalla.abierta")[0], "contexto"), 5);
    assert_eq!(hechos(&e.n, "batalla.abierta")[0].2, vec![plaza]);
    // El asedio se ve desde fuera: la avanzada lleva el id de la batalla que se juega contra ella.
    let ficha = |n: &Nucleo| { let (madre, v) = n.avanzada(av).unwrap(); nucleo::proyeccion::propios::avanzada(n, madre, v).batalla_id };
    assert_eq!(ficha(&e.n), Some(e.n.uuid_de_batalla(id)));
    assert_eq!(e.n.batalla_sobre(Ubicacion::Avanzada(av)).map(|b| b.id), Some(id));
    // La avanzada no se toca mientras dura: ni mejorarla ni destruirla ni guarnecerla ni atacarla otra vez.
    let (_, ana) = e.columna("ana", 1, pos);
    assert_eq!(e.rechazada("!ana", Comando::MejorarAvanzada { heroe: ana, avanzada: av }), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(e.rechazada("!ana", Comando::DestruirAvanzada { heroe: ana, avanzada: av }), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(e.rechazada("!ana", Comando::ElegirTropaDeAvanzada { heroe: ana, avanzada: av, tropa: Tropa::MiliciaLanceros }), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(e.rechazada("ana", Comando::RepostarEnAvanzada { heroe: ana, avanzada: av }), rechazos::BATALLA_BLOQUEADO);
    let (_, otro) = e.atacante(Punto::new(pos.x + 2_000, pos.y), &[Tropa::MiliciaLanceros]);
    assert_eq!(e.rechazada("beto", Comando::Atacar { heroe: otro, objetivo: ObjetivoAtaque::Avanzada(av), declara_guerra: false }), rechazos::BATALLA_BLOQUEADO);
    // Una columna amiga a su puerta, en el mapa, no defiende: hay que estar dentro (haber entrado con `EntrarEnAvanzada`).
    let r = e.ord("ana", Comando::UnirseABatalla { heroe: ana, batalla: id, lado: None });
    assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA);
    assert!(matches!(&r.resultado, Resultado::Rechazada { mensaje, .. } if mensaje.contains("hay que estar dentro")), "{r:?}");
    assert!(e.n.batalla(id).unwrap().ticket.defensor.participantes.is_empty() && e.n.batalla(id).unwrap().incorporaciones.is_empty());
}

#[test]
fn el_atacante_que_gana_por_unity_lleva_lo_mismo_que_con_numeros() {
    let m = con_mundo!();
    let mut e = montar_con(Nucleo::nuevo(arranque_unity(m, 7)), m, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, heroe, escuadra) = e.fuerte_guarnecido();
    let (col, beto) = e.ejercito_fuerte(pos);
    e.guerra();
    humano(&mut e, beto);
    assert!(e.atacar(beto, ObjetivoAtaque::Avanzada(av)).aceptada());
    let id = e.n.batallas().next().unwrap().id;
    let t = e.n.ahora() + MIN;
    en_curso(&mut e.n, id, t, &[beto]);
    let res = resultado(&e.n, id, Lado::Atacante, 2);
    let r = msg(&mut e.n, t + 5 * MIN, "res", Comando::AplicarResultado { batalla: id, resultado: res });
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(e.n.batalla(id).unwrap().estado, EstadoBatalla::Aplicada);
    // Ruinas con la invulnerabilidad de un día desde el resultado, almacén saqueado, guarnición de jugadores a 0 en su campamento.
    let ahora = t + 5 * MIN;
    assert_eq!(e.avanzada(av).estado, EstadoAvanzada::EnRuinas { invulnerable_hasta: Some(ahora + DIA) });
    let en_carro: i64 = e.n.ejercito(col).unwrap().columna.carga.values().sum();
    assert!(en_carro > 0 && e.avanzada(av).almacen.cantidad(Trigo) + e.avanzada(av).almacen.cantidad(Madera) < 450_000);
    let s = e.n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap();
    assert_eq!((s.contenedor, s.cantidad, s.reserva_batalla), (Contenedor::Campamento, 0, None));
    assert!(e.n.guarnicion_de_serie_de_avanzada(av).is_empty());
    assert_eq!(hechos(&e.n, "avanzada.en_ruinas").len(), 1);
    assert_eq!(hechos(&e.n, "batalla.aplicada")[0].2, vec![plaza]);
}

#[test]
fn si_el_defensor_gana_por_unity_la_avanzada_sigue_y_su_guarnicion_cobra_experiencia_y_queda_por_reponer() {
    let m = con_mundo!();
    let mut e = montar_con(Nucleo::nuevo(arranque_unity(m, 7)), m, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, heroe, escuadra) = e.fuerte_guarnecido();
    let (_, beto) = e.ejercito_fuerte(pos);
    e.guerra();
    humano(&mut e, beto);
    assert!(e.atacar(beto, ObjetivoAtaque::Avanzada(av)).aceptada());
    let id = e.n.batallas().next().unwrap().id;
    let t = e.n.ahora() + MIN;
    en_curso(&mut e.n, id, t, &[beto]);
    let almacen = e.avanzada(av).almacen.clone();
    let res = resultado(&e.n, id, Lado::Defensor, 3);
    let fin = t + 5 * MIN;
    assert!(msg(&mut e.n, fin, "res", Comando::AplicarResultado { batalla: id, resultado: res }).aceptada());
    assert_eq!(e.avanzada(av).estado, EstadoAvanzada::Operativa);
    assert_eq!(e.avanzada(av).almacen, almacen, "el almacén no se toca");
    let serie = e.n.guarnicion_de_serie_de_avanzada(av);
    assert!(serie.iter().all(|s| s.cantidad == completa(s.tropa) - 3 && s.experiencia > 0), "mueren 3 de cada una y ganan experiencia");
    assert_eq!(e.n.estado().combate.avanzadas[&av].repone_en, Some(fin + 6 * HORA / 125), "empieza su reposición gratuita, soldado a soldado");
    let s = e.n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap();
    assert_eq!((s.contenedor, s.cantidad, s.reserva_batalla), (Contenedor::Avanzada(av), completa(Tropa::MiliciaLanceros) - 3, None), "la de jugadores sigue en su puesto");
    assert!(e.n.heroe_herido(beto) || e.n.estado().militar.heroes[&beto].heroe.herido_hasta.is_some());
    assert_eq!(hechos(&e.n, "batalla.aplicada")[0].2, vec![plaza]);
    e.hasta(fin + 6 * HORA);
    assert!(e.n.guarnicion_de_serie_de_avanzada(av).iter().all(|s| s.cantidad == completa(s.tropa)));
}

#[test]
fn con_la_guarnicion_aniquilada_esperando_su_reposicion_el_siguiente_ataque_cae_sin_batalla() {
    let m = con_mundo!();
    let mut e = montar_con(Nucleo::nuevo(arranque_unity(m, 7)), m, 2);
    let pos = e.sitio();
    let (av, _, _) = e.avanzada_de(Etapa::Fuerte);
    let (_, primero) = e.ejercito_fuerte(pos);
    e.guerra();
    humano(&mut e, primero);
    assert!(e.atacar(primero, ObjetivoAtaque::Avanzada(av)).aceptada());
    let id = e.n.batallas().next().unwrap().id;
    let t = e.n.ahora() + MIN;
    en_curso(&mut e.n, id, t, &[primero]);
    // Unity da por muertos a todos los de la guarnición: gana la avanzada, pero se queda sin nadie en pie hasta que se reponga.
    let res = resultado(&e.n, id, Lado::Defensor, 999);
    assert!(msg(&mut e.n, t + MIN, "res", Comando::AplicarResultado { batalla: id, resultado: res }).aceptada());
    assert!(e.n.guarnicion_de_serie_de_avanzada(av).iter().all(|s| s.cantidad == 0));
    assert_eq!(e.avanzada(av).estado, EstadoAvanzada::Operativa);
    let (_, segundo) = e.ejercito_fuerte(pos);
    humano(&mut e, segundo);
    let r0 = e.n.estado().rng.estado();
    assert!(e.atacar(segundo, ObjetivoAtaque::Avanzada(av)).aceptada());
    assert_eq!((e.n.batallas().count(), e.n.estado().rng.estado()), (1, r0), "ni una batalla nueva ni una tirada");
    assert!(matches!(e.estado_de(av), EstadoAvanzada::EnRuinas { .. }));
}

#[test]
fn unas_ruinas_no_abren_batalla_de_unity_porque_no_tienen_defensores() {
    let m = con_mundo!();
    let mut e = montar_con(Nucleo::nuevo(arranque_unity(m, 7)), m, 2);
    let pos = e.sitio();
    let (av, _, _) = e.fuerte_guarnecido();
    let (_, beto) = e.ejercito_fuerte(pos);
    e.guerra();
    // La primera batalla, de un bot contra la guarnición (también va a Unity), la gana el atacante y deja el fuerte en ruinas; después, sin defensores, caen sin batalla.
    assert!(e.atacar(beto, ObjetivoAtaque::Avanzada(av)).aceptada());
    let id = e.n.batallas().next().expect("un bot contra la guarnición también abre batalla en Unity").id;
    let t = e.n.ahora() + MIN;
    en_curso(&mut e.n, id, t, &[]);
    let res = resultado(&e.n, id, Lado::Atacante, 999);
    assert!(msg(&mut e.n, t + MIN, "res", Comando::AplicarResultado { batalla: id, resultado: res }).aceptada());
    assert!(matches!(e.estado_de(av), EstadoAvanzada::EnRuinas { .. }));
    e.avanzar(DIA);
    let abiertas = e.n.batallas().count();
    // Un ejército nuevo (el primero lleva un día sin ración).
    let (_, segundo) = e.ejercito_fuerte(pos);
    assert!(e.atacar(segundo, ObjetivoAtaque::Avanzada(av)).aceptada());
    assert_eq!(e.n.batallas().count(), abiertas, "unas ruinas caen sin batalla");
    assert!(e.n.avanzada(av).is_none());
}

// ───────────────────────────── 9. Encuentros y visión ─────────────────────────────

#[test]
fn una_columna_ajena_que_se_acerca_a_una_avanzada_operativa_deja_un_encuentro_y_no_una_propia() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, col, _) = e.colocar_en(pos);
    // En obra no se ataca, así que tampoco se la «encuentra»: la columna de Beto cruza hacia ella.
    let (beto, _) = e.columna("beto", 2, Punto::new(pos.x + 9_000, pos.y));
    e.ok("beto", Comando::MarcharA { ejercito: beto, destino: Punto::new(pos.x + 200, pos.y) });
    e.avanzar(2 * MIN);
    assert!(hechos(&e.n, "avanzada.encuentro").is_empty(), "una obra pendiente no se encuentra");
    e.acabar_obra(av, col);
    // Operativa: Beto vuelve a cruzar.
    e.ok("beto", Comando::MarcharA { ejercito: beto, destino: Punto::new(pos.x + 9_000, pos.y) });
    e.avanzar(2 * MIN);
    e.ok("beto", Comando::MarcharA { ejercito: beto, destino: Punto::new(pos.x + 200, pos.y) });
    e.avanzar(2 * MIN);
    let h = hechos(&e.n, "avanzada.encuentro");
    assert_eq!(h.len(), 1, "{h:?}");
    assert_eq!((h[0].1.clone(), h[0].2.clone(), dato(&h[0], "avanzada"), dato(&h[0], "faccionA"), dato(&h[0], "faccionB")), (vec![beto], vec![plaza], av as i64, 2, 1));
    // Una columna de la misma Facción no la encuentra.
    let (propia, _) = e.columna("ana", 1, Punto::new(pos.x + 9_000, pos.y));
    e.ok("ana", Comando::MarcharA { ejercito: propia, destino: Punto::new(pos.x + 200, pos.y) });
    e.avanzar(2 * MIN);
    assert_eq!(hechos(&e.n, "avanzada.encuentro").len(), 1);
}

#[test]
fn una_avanzada_operativa_ve_a_12_en_los_limites_y_una_destruida_ya_no_ve() {
    use nucleo::espacial::GeneradorVision;
    let m = con_mundo!();
    let ajustes = Ajustes { muestreo_ms: 250, ..Ajustes::default() };
    let mut e = montar_con(Nucleo::nuevo(Arranque { ajustes, ..arranque(m, 7) }), m, 2);
    e.n.agregar_generador(Box::new(GeneradorVision::nuevo()));
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, col, heroe) = e.atalaya();
    // La columna que la levantó vuelve a la plaza y entra: a partir de aquí los únicos ojos de la Facción 1 son la avanzada y dos columnas en el centro de la plaza
    // (que ven 15), y las de las otras Facciones se ponen al otro lado de la avanzada, a más de 15 u del centro.
    let centro = e.centro;
    e.ok("ana", Comando::MarcharA { ejercito: col, destino: centro });
    e.avanzar(2 * MIN);
    e.ok("admin", Comando::Guarnecer { asentamiento: plaza, heroe });
    let ve = |e: &Esc, otra: u32| hechos(&e.n, "vision.contacto").iter().any(|h| dato(h, "faccionQueVe") == 1 && dato(h, "faccionVista") == otra as i64);
    // Fuera de las 12 u (13) no se ve; en el límite exacto (12) sí, porque ver es «distancia ≤ radio»; y dentro, también.
    e.columna("c3", 3, Punto::new(pos.x + 13_000, pos.y));
    e.columna("c4", 4, Punto::new(pos.x + 12_000, pos.y));
    e.columna("c2", 2, Punto::new(pos.x + 11_000, pos.y));
    e.avanzar(MIN);
    assert!((ve(&e, 2), ve(&e, 3), ve(&e, 4)) == (true, false, true), "{:?}", hechos(&e.n, "vision.contacto"));
    // Destruida (a mano), ya no ve: una Facción nueva a 11 u de ella (y a 19 del centro de la plaza) pasa inadvertida.
    e.ok("ana", Comando::DestruirAvanzada { heroe, avanzada: av });
    e.columna("c5", 5, Punto::new(pos.x + 11_000, pos.y + 1_000));
    e.avanzar(MIN);
    assert!(!ve(&e, 5));
}

// ───────────────────────────── 10. La madre cae ─────────────────────────────

#[test]
fn si_la_madre_es_conquistada_sus_avanzadas_pasan_con_ella_y_su_guarnicion_y_elecciones_se_reinician() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let (av, heroe, escuadra) = e.fuerte_guarnecido();
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] });
    e.con_barracon();
    e.ok("ana", Comando::ElegirTropaDeAvanzada { heroe, avanzada: av, tropa: Tropa::LancerosMimbre });
    // El ejército de Beto asedia la madre, que no tiene quien la defienda (el héroe está dentro pero sin loadout), y cae sin combate.
    let centro = e.centro;
    let (_, beto) = e.atacante(Punto::new(centro.x + 1_000, centro.y), &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    let cosas = e.n.cosas_en_el_espacio();
    e.ok("beto", Comando::Atacar { heroe: beto, objetivo: ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    tras_la_convocatoria(&mut e.n);
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2);
    // La avanzada es de la plaza: pasa con ella, con su obra, su almacén y su sitio en el espacio.
    let (madre, a) = e.n.avanzada(av).unwrap();
    assert_eq!((madre, a.etapa, a.estado), (Some(plaza), Etapa::Fuerte, EstadoAvanzada::Operativa));
    assert_eq!(e.n.cosas_en_el_espacio(), cosas + 1, "la avanzada sigue en el espacio; la columna nueva es la del héroe que se desaloja de la plaza");
    let h = &hechos(&e.n, "avanzada.traspasada")[0];
    assert_eq!((dato(h, "avanzada"), dato(h, "faccion"), h.2.clone()), (av as i64, 2, vec![plaza]));
    // La guarnición de los antiguos residentes se va a 0 al campamento y la elección del Gobernador derrotado se borra.
    let s = e.n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap();
    assert_eq!((s.contenedor, s.cantidad), (Contenedor::Campamento, 0));
    assert!(e.n.tropas_elegidas_de_avanzada(av).is_empty());
    // Ahora manda Beto (F2): puede mejorarla, y Ana ya no.
    assert_eq!(e.rechazada("!ana", Comando::MejorarAvanzada { heroe, avanzada: av }), rechazos::NO_AUTORIZADO);
    e.nivel(3);
    e.ok("beto", Comando::MejorarAvanzada { heroe: beto, avanzada: av });
    assert_eq!(e.avanzada(av).obra.as_ref().unwrap().tipo, TipoObra::Mejorar);
}

#[test]
fn si_la_madre_cae_en_ruinas_la_atalaya_y_el_fuerte_desaparecen_y_la_ciudadela_queda_sin_dueno_hasta_que_alguien_la_repara() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 3);
    let (plaza, centro) = (e.plaza, e.centro);
    // Una ciudadela con trigo, una atalaya, y un héroe residente con una escuadra en la guarnición de la ciudadela.
    let (ciudadela, col, heroe) = e.avanzada_de(Etapa::Ciudadela);
    e.ord("admin", Comando::DarSuministro { ejercito: col, recurso: Trigo, cantidad: 300_000 });
    e.ok("ana", Comando::DepositarEnAvanzada { ejercito: col, avanzada: ciudadela, recurso: Trigo, cantidad: 300_000 });
    e.residir(heroe);
    let escuadra = e.dar_escuadra(heroe, Tropa::MiliciaLanceros);
    e.ok("ana", Comando::GuarnecerAvanzada { heroe, avanzada: ciudadela, escuadras: vec![escuadra] });
    let otra = Punto::new(centro.x - 8_000, centro.y);
    let (atalaya, col2, _) = e.colocar_en(otra);
    e.acabar_obra(atalaya, col2);
    // Sin auto-construcción ni madera que repongan el mantenimiento, la plaza acaba cayendo en ruinas.
    for dia in 1..=200 {
        let t = e.n.ahora().max(dia * DIA);
        e.n.avanzar_hasta(t);
        if e.n.economia(plaza).is_none() {
            break;
        }
    }
    assert!(e.n.economia(plaza).is_none(), "la plaza cayó en ruinas");
    assert!(e.n.avanzada(atalaya).is_none(), "la atalaya desaparece con su madre");
    let (madre, ruinas) = e.n.avanzada(ciudadela).expect("la ciudadela queda como ruina sin dueño");
    assert_eq!((madre, ruinas.etapa, ruinas.estado, ruinas.almacen.cantidad(Trigo)), (None, Etapa::Ciudadela, EstadoAvanzada::EnRuinas { invulnerable_hasta: None }, 300_000));
    assert_eq!(e.n.avanzadas_huerfanas().count(), 1);
    assert_eq!(dato(&hechos(&e.n, "avanzada.destruida").iter().find(|h| dato(h, "avanzada") == atalaya as i64).unwrap(), "motivo"), 2, "por la ruina de la madre");
    assert_eq!(hechos(&e.n, "avanzada.sin_dueno").len(), 1);
    // La guarnición de jugadores vuelve intacta (final normal, no derrota) al campamento de su héroe.
    let s = e.n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap();
    assert_eq!((s.contenedor, s.cantidad), (Contenedor::Campamento, completa(Tropa::MiliciaLanceros)));
    // Unas ruinas sin dueño no reciben órdenes, no se atacan (no hay Facción con la que estar en guerra) y nadie las ve.
    assert_eq!(e.rechazada("ana", Comando::MejorarAvanzada { heroe, avanzada: ciudadela }), rechazos::AVANZADA_DESCONOCIDA);
    assert_eq!(e.rechazada("ana", Comando::GuarnecerAvanzada { heroe, avanzada: ciudadela, escuadras: vec![escuadra] }), rechazos::AVANZADA_DESCONOCIDA);
    let pos = e.n.posicion_de_avanzada(ciudadela).unwrap();
    let (col_beto, beto) = e.atacante(Punto::new(pos.x + 1_000, pos.y), &[Tropa::EspadachinesBronce]);
    e.guerra();
    assert_eq!(e.rechazada("beto", Comando::Atacar { heroe: beto, objetivo: ObjetivoAtaque::Avanzada(ciudadela), declara_guerra: false }), rechazos::COMBATE_AVANZADA_SIN_DUENO);
    // Pero se recuerdan como se vieron (Niebla_De_Guerra §4.1, decisión del autor del 2026-10-24): F2 pasa junto a ellas y su ficha dice ciudadela, de nadie, en ruinas.
    e.ok("beto", Comando::MarcharA { ejercito: col_beto, destino: Punto::new(pos.x + 3_000, pos.y) });
    let ficha = e.n.estado().memoria.fichas[&2][&nucleo::memoria::Sitio::Avanzada(ciudadela)];
    assert_eq!((ficha.datos.faccion, ficha.datos.nivel, ficha.datos.estado), (0, 3, nucleo::memoria::EstadoSitio::EnRuinas));
    // Quien tenga después esa zona la repara: una plaza nueva de Ana en el sitio de la antigua, de nivel 3, la adopta con su cupo.
    assert_eq!(e.rechazada("ana", Comando::RepararAvanzada { heroe, avanzada: ciudadela }), rechazos::AVANZADA_FUERA_DE_LA_ZONA, "todavía ninguna plaza de su Facción tiene esa zona");
    let nueva = e.fundar_otra_en(centro);
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: nueva, nivel: 3 });
    for _ in 0..14 {
        e.ord("admin", Comando::AgrandarZona { asentamiento: nueva });
    }
    assert_eq!(e.rechazada("beto", Comando::RepararAvanzada { heroe: beto, avanzada: ciudadela }), rechazos::AVANZADA_FUERA_DE_LA_ZONA, "la zona es de la Facción 1: Beto no tiene plaza ahí");
    assert!(e.n.economia(nueva).unwrap().avanzadas.is_empty());
    e.ok("ana", Comando::RepararAvanzada { heroe, avanzada: ciudadela });
    let (madre, a) = e.n.avanzada(ciudadela).unwrap();
    assert_eq!((madre, a.estado), (Some(nueva), EstadoAvanzada::EnRuinas { invulnerable_hasta: None }));
    assert_eq!((e.n.avanzadas_de(nueva).len(), e.n.avanzadas_huerfanas().count()), (1, 0));
    let obra = a.obra.as_ref().unwrap();
    assert_eq!((obra.tipo, obra.requerido.clone()), (TipoObra::Reparar, vec![(Madera, 750_000), (Oro, 150_000), (Piedra, 1_000_000)]), "la mitad de 1.500 + 300 + 2.000");
    assert_eq!(hechos(&e.n, "avanzada.adoptada").len(), 1);
    // Con los materiales entregados y los dos días de obra, vuelve a funcionar y al espacio.
    let (col3, _) = e.columna("ana", 1, pos);
    e.acabar_obra(ciudadela, col3);
    assert_eq!(e.estado_de(ciudadela), EstadoAvanzada::Operativa);
    assert_eq!(e.avanzada(ciudadela).almacen.cantidad(Trigo), 300_000, "el almacén se conserva");
}

impl Esc {
    /// Funda otra plaza de Ana (F1) donde estuvo la primera, que ya no existe, y pasa a ser «la plaza» del escenario (la que se repone con `hasta`).
    fn fundar_otra_en(&mut self, p: Punto) -> u64 {
        self.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 8 });
        let (col, _) = self.columna("ana", 1, p);
        self.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
        let id = self.ok("ana", Comando::Fundar { ejercito: col }).creadas[0];
        self.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: id, pausada: true });
        self.plaza = id;
        id
    }
}

// ───────────────────────────── 11. La foto, el determinismo y el hash ─────────────────────────────

/// Un guion con avanzadas: coloca, entrega, la levanta, la mejora, guarnece y la ataca. Todas las órdenes llevan su instante: el cierre `avanzar` solo decide cómo se mueve el reloj
/// entre ellas.
fn guion(e: &mut Esc, avanzar: &dyn Fn(&mut Esc, Instante)) {
    let (plaza, pos) = (e.plaza, e.sitio());
    let (col, heroe) = e.columna("ana", 1, pos);
    e.residir(heroe);
    let escuadra = e.dar_escuadra(heroe, Tropa::MiliciaLanceros);
    let av = e.ok_en(MIN, "ana", Comando::ColocarAvanzada { asentamiento: plaza, heroe, posicion: pos }).creadas[0];
    avanzar(e, 20 * MIN);
    for (r, n) in [(Madera, 200_000), (Piedra, 100_000)] {
        e.ok_en(20 * MIN, "admin", Comando::DarSuministro { ejercito: col, recurso: r, cantidad: n });
    }
    e.ok_en(20 * MIN + 17_345, "ana", Comando::EntregarMaterialesAAvanzada { ejercito: col, avanzada: av });
    avanzar(e, 4 * HORA);
    e.ok_en(4 * HORA, "ana", Comando::MejorarAvanzada { heroe, avanzada: av });
    avanzar(e, 5 * HORA);
    e.ok_en(5 * HORA, "ana", Comando::GuarnecerAvanzada { heroe, avanzada: av, escuadras: vec![escuadra] });
    // El fuerte pide 600 de madera, 400 de piedra y 50 de oro: tres viajes del carro de 500.
    let entrega_del_fuerte = |e: &mut Esc, t: Instante| {
        for viaje in [vec![(Madera, 500_000)], vec![(Piedra, 400_000), (Oro, 50_000)], vec![(Madera, 100_000)]] {
            for (r, n) in viaje {
                e.ok_en(t, "admin", Comando::DarSuministro { ejercito: col, recurso: r, cantidad: n });
            }
            e.ok_en(t, "ana", Comando::EntregarMaterialesAAvanzada { ejercito: col, avanzada: av });
        }
    };
    // Un primer intento de mejora arranca y se abandona media hora después: su cita se descarta y la mejora se vuelve a marcar.
    entrega_del_fuerte(e, 5 * HORA);
    avanzar(e, 5 * HORA + 30 * MIN);
    e.ok_en(5 * HORA + 30 * MIN, "ana", Comando::CancelarObraDeAvanzada { heroe, avanzada: av });
    e.ok_en(5 * HORA + 30 * MIN, "ana", Comando::MejorarAvanzada { heroe, avanzada: av });
    avanzar(e, 6 * HORA);
    entrega_del_fuerte(e, 6 * HORA);
    avanzar(e, 9 * HORA);
    e.ok_en(9 * HORA, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    let beto = e.n.crear_ejercito(9 * HORA, "beto", 2, Punto::new(pos.x + 1_000, pos.y), 500, 100_000).unwrap();
    let heroe_beto = e.n.ejercito(beto).unwrap().columna.lider;
    e.ok_en(9 * HORA, "admin", Comando::DarEscuadra { heroe: heroe_beto, tropa: Tropa::Lenadores, cantidad: completa(Tropa::Lenadores) });
    e.ok_en(9 * HORA, "beto", Comando::Atacar { heroe: heroe_beto, objetivo: ObjetivoAtaque::Avanzada(av), declara_guerra: false });
    avanzar(e, 20 * HORA);
}

fn correr(m: &M, semilla: u64, avanzar: &dyn Fn(&mut Esc, Instante)) -> Nucleo {
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
    // El guion hizo de verdad lo que se espera: levantó, mejoró (tras abandonar un primer intento), guarneció y luchó.
    assert_eq!(hechos(&normal, "avanzada.obra_cancelada").len(), 1);
    assert_eq!(hechos(&normal, "avanzada.completada").len(), 2);
    assert_eq!(hechos(&normal, "avanzada.guarnecida").len(), 1);
    assert_eq!(hechos(&normal, "combate.avanzada_resiste").len(), 1);
}

#[test]
fn una_foto_a_mitad_de_obra_y_otra_con_la_guarnicion_mermada_recuperan_el_mismo_mundo_y_la_cita_sigue_exacta() {
    let m = con_mundo!();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    // Dos cortes: con la primera obra a mitad y con la guarnición de serie mermada esperando su reposición.
    for (nombre, mermada) in [("a mitad de obra", false), ("con la guarnición mermada", true)] {
        let dir = tempfile::tempdir().unwrap();
        let guion = |e: &mut Esc| {
            let pos = e.sitio();
            let (av, col, heroe) = e.colocar_en(pos);
            e.entregar_todo(av, col);
            if mermada {
                e.hasta(e.avanzada(av).obra.as_ref().and_then(|o| o.completa_en).unwrap());
                e.ok("ana", Comando::MejorarAvanzada { heroe, avanzada: av });
                e.acabar_obra(av, col);
                e.guerra();
                let (_, beto) = e.atacante(Punto::new(pos.x + 1_000, pos.y), &[Tropa::Lenadores]);
                e.ok("beto", Comando::Atacar { heroe: beto, objetivo: ObjetivoAtaque::Avanzada(av), declara_guerra: false });
            } else {
                e.avanzar(MIN);
            }
            av
        };
        // Sin parar: la referencia.
        let referencia = {
            let mut e = montar(m, 9, 2);
            guion(&mut e);
            e.avanzar(2 * DIA);
            e.n.hash_estado()
        };
        let (a_mitad, cuenta, plaza, centro, t_guion, fin) = {
            let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
            let mut e = montar_con(n, m, 2);
            let av = guion(&mut e);
            let fin = e.avanzada(av).obra.as_ref().and_then(|o| o.completa_en);
            if mermada {
                assert!(e.n.estado().combate.avanzadas[&av].repone_en.is_some(), "la foto pilla la reposición pendiente");
            } else {
                assert!(fin.is_some() && e.estado_de(av) == EstadoAvanzada::EnConstruccion, "la foto pilla la obra en mitad");
            }
            let t_guion = e.n.ahora();
            e.n.foto_ahora();
            e.avanzar(10 * MIN);
            e.n.apagar().unwrap();
            (e.n.hash_estado(), e.cuenta, e.plaza, e.centro, t_guion, fin)
        };
        let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        assert!(n.recuperado());
        assert_eq!(n.hash_estado(), a_mitad, "{nombre}: foto + registro reconstruyen el mismo mundo");
        let mut e = Esc { n, plaza, centro, cuenta };
        if let (Some(fin), false) = (fin, mermada) {
            // La cita viaja en la foto: la obra acaba en su instante exacto también tras recuperar.
            let av = e.n.avanzadas_de(plaza)[0].id;
            e.hasta(fin - 1);
            assert_eq!(e.estado_de(av), EstadoAvanzada::EnConstruccion);
            e.hasta(fin);
            assert_eq!(e.estado_de(av), EstadoAvanzada::Operativa);
        }
        e.hasta(t_guion + 2 * DIA);
        assert_eq!(e.n.hash_estado(), referencia, "{nombre}: el mundo recuperado sigue igual que el que no se interrumpió");
        e.n.apagar().unwrap();
    }
}

#[test]
fn el_estado_de_las_avanzadas_va_en_la_foto_y_cualquier_campo_cambia_el_hash_y_el_formato_anterior_se_rechaza() {
    use militar::escuadra::Escuadra;
    let reglas = Reglas::default();
    let mut base = nucleo::Estado::nuevo(reglas.clone(), "h".into(), 1, 0);
    let ruinas = asentamiento::avanzadas::Avanzada {
        id: 5,
        posicion: Punto::new(1_000, 2_000),
        etapa: Etapa::Ciudadela,
        estado: EstadoAvanzada::EnRuinas { invulnerable_hasta: None },
        obra: None,
        almacen: asentamiento::Almacen::nuevo(10_000_000),
    };
    base.combate.huerfanas.insert(5, ruinas);
    let mr = militar::Reglas::cargar();
    let mut g = nucleo::GuarnicionDeAvanzada::default();
    g.elegidas.insert(2, Tropa::LancerosMimbre);
    g.escuadras = vec![Escuadra::nueva(&mr, 70, u64::MAX, Tropa::MiliciaLanceros, 25, None, 0)];
    g.repone_en = Some(6_000);
    base.combate.avanzadas.insert(9, g);
    let bytes = base.serializar();
    assert_eq!(&bytes[..5], b"BAN33");
    let vuelta = nucleo::Estado::deserializar(&bytes, &reglas, "h").unwrap();
    assert_eq!(vuelta.combate, base.combate);
    assert_eq!(vuelta.serializar(), bytes, "serializar → deserializar → serializar es idéntico");
    let mut vistos = std::collections::HashSet::from([base.hash()]);
    type Cambio = Box<dyn Fn(&mut nucleo::EstadoCombate)>;
    let cambios: Vec<(&str, Cambio)> = vec![
        ("posición de las ruinas", Box::new(|c| c.huerfanas.get_mut(&5).unwrap().posicion = Punto::new(1_001, 2_000))),
        ("invulnerabilidad de las ruinas", Box::new(|c| c.huerfanas.get_mut(&5).unwrap().estado = EstadoAvanzada::EnRuinas { invulnerable_hasta: Some(7) })),
        ("almacén de las ruinas", Box::new(|c| {
            c.huerfanas.get_mut(&5).unwrap().almacen.agregar(Trigo, 1_000);
        })),
        ("unas ruinas menos", Box::new(|c| drop(c.huerfanas.remove(&5)))),
        ("tropa elegida", Box::new(|c| {
            c.avanzadas.get_mut(&9).unwrap().elegidas.insert(2, Tropa::MiliciaLanceros);
        })),
        ("otro escalón elegido", Box::new(|c| {
            c.avanzadas.get_mut(&9).unwrap().elegidas.insert(3, Tropa::Arqueros);
        })),
        ("guarnición de serie", Box::new(|c| c.avanzadas.get_mut(&9).unwrap().escuadras[0].cantidad -= 1)),
        ("reposición", Box::new(|c| c.avanzadas.get_mut(&9).unwrap().repone_en = None)),
        ("una guarnición menos", Box::new(|c| drop(c.avanzadas.remove(&9)))),
    ];
    for (nombre, f) in cambios {
        let mut s = nucleo::Estado::deserializar(&bytes, &reglas, "h").unwrap();
        f(&mut s.combate);
        assert!(vistos.insert(s.hash()), "el hash no cambió (o coincidió con otro cambio) al tocar {nombre}");
    }
    // Los tres plazos de la cola van y vuelven, cada uno con su instante.
    let mut con_cola = nucleo::Estado::deserializar(&bytes, &reglas, "h").unwrap();
    for (i, s) in [SucesoCombate::FinInvulnerabilidad, SucesoCombate::ReponeAvanzada, SucesoCombate::FinObraAvanzada].into_iter().enumerate() {
        con_cola.cola.programar(100 + i as i64, nucleo::PRIO_BANDIDOS, con_cola.generaciones.clave(i as u64 + 1), Carga::Combate(s));
    }
    let b2 = con_cola.serializar();
    let otra = nucleo::Estado::deserializar(&b2, &reglas, "h").unwrap();
    assert_eq!((otra.serializar(), otra.cola.len()), (b2, 3));
    // Una escuadra en la guarnición de una avanzada también va y vuelve.
    let mut s = nucleo::Estado::deserializar(&bytes, &reglas, "h").unwrap();
    s.combate.avanzadas.get_mut(&9).unwrap().escuadras[0].contenedor = Contenedor::Avanzada(5);
    assert_eq!(s.combate.avanzadas[&9].escuadras[0].contenedor, Contenedor::Avanzada(5));
    // Los formatos anteriores se rechazan con su motivo.
    let viejo = [b"BAN14".as_slice(), &bytes[5..]].concat();
    let Err(e) = nucleo::Estado::deserializar(&viejo, &reglas, "h") else { panic!("BAN14 no debe admitirse") };
    assert!(e.to_string().contains("BAN14") && e.to_string().contains("avanzadas"), "{e}");
    let viejo = [b"BAN15".as_slice(), &bytes[5..]].concat();
    let Err(e) = nucleo::Estado::deserializar(&viejo, &reglas, "h") else { panic!("BAN15 no debe admitirse") };
    assert!(e.to_string().contains("BAN15") && e.to_string().contains("cita"), "{e}");
}

#[test]
fn cada_campo_de_la_avanzada_de_una_plaza_cambia_el_hash() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (av, _, _) = e.atalaya();
    let reglas = e.n.estado().reglas.clone();
    let hash_h = e.n.mundo_hash().to_string();
    let bytes = e.n.estado().serializar();
    let base = nucleo::Estado::deserializar(&bytes, &reglas, &hash_h).unwrap();
    assert_eq!(base.serializar(), bytes);
    let plaza = e.plaza;
    type Cambio = Box<dyn Fn(&mut nucleo::Asentamiento)>;
    let cambios: Vec<(&str, Cambio)> = vec![
        ("posición", Box::new(move |a| a.economia.avanzadas.iter_mut().find(|v| v.id == av).unwrap().posicion.x += 1)),
        ("etapa", Box::new(move |a| a.economia.avanzadas.iter_mut().find(|v| v.id == av).unwrap().etapa = Etapa::Fuerte)),
        ("estado", Box::new(move |a| a.economia.avanzadas.iter_mut().find(|v| v.id == av).unwrap().estado = EstadoAvanzada::EnRuinas { invulnerable_hasta: Some(1) })),
        ("almacén", Box::new(move |a| a.economia.avanzadas.iter_mut().find(|v| v.id == av).unwrap().almacen.ampliar(Trigo, 1))),
        ("una avanzada menos", Box::new(move |a| a.economia.avanzadas.retain(|v| v.id != av))),
    ];
    let mut vistos = std::collections::HashSet::from([base.hash()]);
    for (nombre, f) in cambios {
        let mut s = nucleo::Estado::deserializar(&bytes, &reglas, &hash_h).unwrap();
        f(s.asentamientos.get_mut(&plaza).unwrap());
        assert!(vistos.insert(s.hash()), "el hash no cambió al tocar {nombre}");
    }
    // Una obra a medias (con lo entregado) viaja entera.
    let pos = Punto::new(e.centro.x - 8_000, e.centro.y);
    let (otra_av, otra_col, _) = e.colocar_en(pos);
    e.ord("admin", Comando::DarSuministro { ejercito: otra_col, recurso: Madera, cantidad: 100_000 });
    e.ok("ana", Comando::EntregarMaterialesAAvanzada { ejercito: otra_col, avanzada: otra_av });
    let bytes = e.n.estado().serializar();
    let s = nucleo::Estado::deserializar(&bytes, &reglas, &hash_h).unwrap();
    assert_eq!(s.serializar(), bytes);
    let obra = s.asentamientos[&plaza].economia.avanzada(otra_av).unwrap().obra.clone().unwrap();
    assert_eq!((obra.tipo, obra.falta()), (TipoObra::Construir, vec![(Madera, 100_000), (Piedra, 100_000)]));
}

// ───────────────────────────── 12. Entrar en una avanzada (Entrar con todo, 2026-10-22) ─────────────────────────────

/// Tropa de élite para el que defiende desde dentro.
const ELITE: [Tropa; 4] = [Tropa::Shardana, Tropa::Shardana, Tropa::Hequetai, Tropa::Hequetai];

/// Las tropas del ejército fuerte de Beto.
const FUERTE: [Tropa; 5] = [Tropa::EspadachinesBronce, Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto, Tropa::ArquerosCompuesto, Tropa::HacherosArmados];

impl Esc {
    /// Una columna de administración que nace en el instante `t`, en `p`, con esas tropas: (columna, héroe).
    fn columna_en(&mut self, t: Instante, dueno: &str, faccion: u32, p: Punto, tropas: &[Tropa]) -> (u64, u64) {
        let id = self.n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
        let h = self.n.ejercito(id).unwrap().columna.lider;
        for &tropa in tropas {
            self.ok_en(t, "admin", Comando::DarEscuadra { heroe: h, tropa, cantidad: completa(tropa) });
        }
        (id, h)
    }
    /// Un héroe de esa Facción con una columna a la puerta de la avanzada y esas tropas: (columna, héroe).
    fn visita(&mut self, cuenta: &str, faccion: u32, tropas: &[Tropa]) -> (u64, u64) {
        let pos = self.sitio();
        let (col, h) = self.columna(cuenta, faccion, pos);
        tropas.iter().for_each(|&t| {
            self.dar_escuadra(h, t);
        });
        (col, h)
    }
    fn ubicacion(&self, heroe: u64) -> Ubicacion {
        self.n.heroe(heroe).unwrap().heroe.presencia.ubicacion
    }
    fn carga_en(&self, columna: u64, r: Recurso) -> i64 {
        self.n.ejercito(columna).unwrap().columna.carga.get(&r).copied().unwrap_or(0)
    }
    fn soldados(&self, columna: u64) -> i64 {
        self.n.soldados_de_columna(columna).unwrap()
    }
}

#[test]
fn se_entra_en_una_avanzada_operativa_con_el_acceso_de_la_madre_y_se_sale_con_todo() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    e.abrir_a_los_neutrales();
    let (plaza, pos) = (e.plaza, e.sitio());
    // Una obra no tiene puerta: ni se ve, ni se entra.
    let (av, col, ana) = e.colocar_en(pos);
    assert_eq!(e.rechazada("ana", Comando::EntrarEnAvanzada { avanzada: av, heroe: ana }), rechazos::AVANZADA_NO_OPERATIVA);
    e.acabar_obra(av, col);
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 4, relacion: Some(Relacion::Guerra) });

    // Los rechazos, en su orden y sin dejar rastro.
    let (_, enemigo) = e.visita("eno", 4, &[]);
    let lejos = e.columna("lia", 1, Punto::new(pos.x + 5_000, pos.y));
    let lia = e.n.ejercito(lejos.0).unwrap().columna.lider;
    assert_eq!(e.rechazada("ana", Comando::EntrarEnAvanzada { avanzada: 9_999, heroe: ana }), rechazos::AVANZADA_DESCONOCIDA);
    assert_eq!(e.rechazada("beto", Comando::EntrarEnAvanzada { avanzada: av, heroe: ana }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("lia", Comando::EntrarEnAvanzada { avanzada: av, heroe: lia }), rechazos::AVANZADA_FUERA_DE_LA_PUERTA);
    assert_eq!(e.rechazada("eno", Comando::EntrarEnAvanzada { avanzada: av, heroe: enemigo }), rechazos::PUERTA_CERRADA, "la madre no abre su puerta a los enemigos");
    assert_eq!(e.rechazada("ana", Comando::SalirDeAvanzada { avanzada: av, heroe: ana }), rechazos::HEROE_NO_ESTA_EN_LA_AVANZADA);
    assert_eq!(e.rechazada("ana", Comando::SalirDeAvanzada { avanzada: 9_999, heroe: ana }), rechazos::AVANZADA_DESCONOCIDA);

    // Entra con todo y sale con todo; dentro no está en el mapa y come de sus víveres.
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 30_000 });
    let escuadra = e.dar_escuadra(ana, Tropa::MiliciaLanceros);
    let (carga0, viveres0, t) = (e.carga_en(col, Madera), e.n.ejercito(col).unwrap().columna.viveres.valor_en(e.n.ahora()), e.n.ahora());
    e.ok("ana", Comando::EntrarEnAvanzada { avanzada: av, heroe: ana });
    assert_eq!(e.ubicacion(ana), Ubicacion::Avanzada(av));
    assert!(!e.n.en_el_mapa(col));
    assert_eq!((e.carga_en(col, Madera), e.n.ejercito(col).unwrap().columna.escuadras.clone()), (carga0, vec![escuadra]));
    let h = &hechos(&e.n, "jugador.entra_en_avanzada")[0];
    assert_eq!((h.0, h.2.clone(), dato(h, "heroe"), dato(h, "avanzada"), dato(h, "ejercito")), (t, vec![plaza], ana as i64, av as i64, col as i64));
    e.avanzar(HORA);
    assert!(e.n.ejercito(col).unwrap().columna.viveres.valor_en(e.n.ahora()) < viveres0, "come de sus propios víveres");
    e.ok("ana", Comando::SalirDeAvanzada { avanzada: av, heroe: ana });
    assert_eq!(e.ubicacion(ana), Ubicacion::Columna(col));
    assert!(e.n.en_el_mapa(col));
    assert_eq!((e.carga_en(col, Madera), e.n.ejercito(col).unwrap().posicion), (carga0, pos), "sale con todo, a la puerta");
    assert_eq!(dato(&hechos(&e.n, "jugador.sale_de_avanzada")[0], "avanzada"), av as i64);

    // El aliado y quien no tiene relación entran por defecto; el mismo ejército entra por orden de su Líder.
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Alianza) });
    let (_, aliado) = e.visita("alia", 2, &[]);
    let (_, neutral) = e.visita("neu", 3, &[]);
    e.ok("alia", Comando::EntrarEnAvanzada { avanzada: av, heroe: aliado });
    e.ok("neu", Comando::EntrarEnAvanzada { avanzada: av, heroe: neutral });
    assert_eq!((e.ubicacion(aliado), e.ubicacion(neutral)), (Ubicacion::Avanzada(av), Ubicacion::Avanzada(av)));

    // Cerrar la puerta a los neutrales saca al mapa a «neu», con su columna, y a nadie más.
    let colneu = e.n.heroe(neutral).unwrap().columna.unwrap();
    e.ok("ana", Comando::FijarAcceso { asentamiento: plaza, aliados: true, neutrales: false, enemigos: false });
    assert_eq!((e.ubicacion(neutral), e.n.en_el_mapa(colneu), e.ubicacion(aliado)), (Ubicacion::Columna(colneu), true, Ubicacion::Avanzada(av)));
    let echado = &hechos(&e.n, "heroe.expulsado")[0];
    assert_eq!((dato(echado, "heroe"), dato(echado, "lugar"), dato(echado, "sitio"), dato(echado, "motivo"), echado.2.clone()), (neutral as i64, 2, av as i64, 0, vec![plaza]));
}

#[test]
fn en_una_avanzada_defienden_los_de_dentro_de_la_faccion_de_la_madre_y_los_aliados_y_no_los_neutrales_ni_los_que_estan_a_la_puerta() {
    let m = con_mundo!();
    // (cuenta, Facción, relación con F1, ¿entra?, ¿defiende?): un neutral dentro no defiende; un aliado a la puerta, tampoco; un vasallo y un aliado dentro, sí.
    for (cuenta, faccion, relacion, entra, defiende) in [
        ("alia", 3, Some(Relacion::Alianza), true, true),
        ("vasa", 3, Some(Relacion::Vasallaje), true, true),
        ("ana", 1, None, true, true),
        ("neu", 3, None, true, false),
        ("puerta", 3, Some(Relacion::Alianza), false, false),
    ] {
        let mut e = montar(m, 7, 2);
        e.abrir_a_los_neutrales();
        let (plaza, pos) = (e.plaza, e.sitio());
        let (av, col, ana) = e.avanzada_de(Etapa::Fuerte);
        if let Some(r) = relacion {
            e.ok("admin", Comando::FijarRelacion { a: 1, b: faccion, relacion: Some(r) });
        }
        // La de «ana» es la columna con la que se levantó; las demás, una nueva. Todas con la élite.
        let (c, h) = if cuenta == "ana" {
            ELITE.iter().for_each(|&t| {
                e.dar_escuadra(ana, t);
            });
            (col, ana)
        } else {
            e.visita(cuenta, faccion, &ELITE)
        };
        if entra {
            e.ok(cuenta, Comando::EntrarEnAvanzada { avanzada: av, heroe: h });
            assert!(!e.n.en_el_mapa(c));
        }
        let (_, beto) = e.ejercito_fuerte(pos);
        e.guerra();
        let soldados0 = e.soldados(c);
        let (cosas, v0) = (e.n.cosas_en_el_espacio(), e.n.version());
        assert!(e.atacar(beto, ObjetivoAtaque::Avanzada(av)).aceptada());
        assert_eq!(e.n.version(), v0 + 1);
        let (resiste, ruinas) = (hechos(&e.n, "combate.avanzada_resiste").len(), hechos(&e.n, "combate.avanzada_en_ruinas").len());
        assert_eq!((resiste, ruinas), (defiende as usize, !defiende as usize), "{cuenta}: ¿defiende?");
        let _ = (plaza, cosas);
        assert_eq!(e.n.heroe_herido(beto), defiende, "{cuenta}: el atacante queda herido si pierde");
        assert!(!e.n.heroe_herido(h), "{cuenta}: quien defiende y gana no queda herido y el que no defiende no pierde nada");
        if defiende {
            assert_eq!(e.estado_de(av), EstadoAvanzada::Operativa);
            assert_eq!(e.ubicacion(h), Ubicacion::Avanzada(av), "la avanzada resiste: el defensor sigue dentro");
            assert!(e.soldados(c) < soldados0, "{cuenta}: sufrió bajas defendiendo");
        } else {
            // Cae en ruinas: quien estaba dentro sale al mapa con su columna intacta (nadie queda dentro de unas ruinas).
            assert!(matches!(e.estado_de(av), EstadoAvanzada::EnRuinas { .. }));
            assert_eq!((e.ubicacion(h), e.n.en_el_mapa(c), e.soldados(c)), (Ubicacion::Columna(c), true, soldados0), "{cuenta}: sale al mapa sin perder nada");
            assert_eq!(hechos(&e.n, "heroe.expulsado").len(), entra as usize);
        }
    }
}

#[test]
fn el_residente_y_el_visitante_amigo_que_defienden_una_avanzada_y_la_pierden_quedan_heridos_pierden_la_mitad_del_carro_y_salen_al_mapa() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, col, ana) = e.avanzada_de(Etapa::Fuerte);
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 3, relacion: Some(Relacion::Alianza) });
    // Dentro: la residente «ana» (tropa floja y 31 000 de madera) y un aliado visitante (tropa floja y 31 000 de piedra).
    e.residir(ana);
    e.dar_escuadra(ana, Tropa::Lenadores);
    let antes = e.carga_en(col, Madera);
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 31_000 });
    let vis = e.visita("alia", 3, &[Tropa::Lenadores]);
    e.ok("admin", Comando::DarSuministro { ejercito: vis.0, recurso: Piedra, cantidad: 31_000 });
    e.ok("ana", Comando::EntrarEnAvanzada { avanzada: av, heroe: ana });
    e.ok("alia", Comando::EntrarEnAvanzada { avanzada: av, heroe: vis.1 });
    let con_madera = e.carga_en(col, Madera);
    assert_eq!(con_madera, antes + 31_000);
    let (cb, beto) = e.ejercito_fuerte(pos);
    e.guerra();
    let t = e.n.ahora();
    assert!(e.atacar(beto, ObjetivoAtaque::Avanzada(av)).aceptada());
    // Gana el atacante: el fuerte queda en ruinas (invulnerable un día), los dos defensores en persona quedan heridos y salen al mapa con su columna.
    let h = &hechos(&e.n, "combate.avanzada_en_ruinas")[0];
    assert_eq!((h.0, dato(h, "heridos")), (t, 2));
    assert!(matches!(e.estado_de(av), EstadoAvanzada::EnRuinas { invulnerable_hasta: Some(_) }));
    assert!(e.n.heroe_herido(ana) && e.n.heroe_herido(vis.1) && !e.n.heroe_herido(beto));
    assert_eq!((e.ubicacion(ana), e.ubicacion(vis.1)), (Ubicacion::Columna(col), Ubicacion::Columna(vis.0)));
    assert!(e.n.en_el_mapa(col) && e.n.en_el_mapa(vis.0));
    assert_eq!(e.carga_en(col, Madera), con_madera - con_madera / 2 / 1_000 * 1_000, "la residente pierde la mitad de lo que llevaba su carro, hacia abajo y en unidades enteras, recurso a recurso");
    assert_eq!(e.carga_en(vis.0, Piedra), 31_000 - 15_000, "el visitante amigo que defendió también pierde la mitad: misma regla para todos");
    // La mitad perdida es del vencedor, hasta donde le quepa (el almacén de la avanzada y la chatarra ya llenan parte de su carro).
    assert!(dato(h, "carroPerdido") > 0 && e.carga_en(cb, Madera) > 0);
    assert_eq!(hechos(&e.n, "heroe.expulsado").iter().map(|x| (dato(x, "heroe"), dato(x, "lugar"), dato(x, "motivo"), x.2.clone())).collect::<Vec<_>>(), vec![(ana as i64, 2, 1, vec![plaza]), (vis.1 as i64, 2, 1, vec![plaza])]);
}

#[test]
fn destruir_a_mano_una_avanzada_saca_al_mapa_a_quien_esta_dentro_con_lo_suyo() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    e.abrir_a_los_neutrales();
    let (av, _, ana) = e.atalaya();
    let (cn, neutral) = e.visita("neu", 3, &[Tropa::MiliciaLanceros]);
    e.ok("neu", Comando::EntrarEnAvanzada { avanzada: av, heroe: neutral });
    assert!(!e.n.en_el_mapa(cn));
    let pos = e.sitio();
    e.ok("ana", Comando::DestruirAvanzada { heroe: ana, avanzada: av });
    assert!(e.n.avanzada(av).is_none());
    assert_eq!((e.ubicacion(neutral), e.n.en_el_mapa(cn), e.n.ejercito(cn).unwrap().posicion), (Ubicacion::Columna(cn), true, pos));
    assert_eq!(dato(&hechos(&e.n, "heroe.expulsado")[0], "motivo"), 3, "destruida a mano");
}

#[test]
fn si_la_madre_es_conquistada_solo_sale_de_su_avanzada_quien_el_acceso_heredado_ya_no_deja_entrar_y_si_cae_en_ruinas_salen_todos_con_lo_suyo() {
    let m = con_mundo!();
    // Conquista de la madre (sin defensores: cae sin combate). La avanzada pasa al conquistador (F2) con la puerta heredada, que vuelve a la de por defecto (Q1: cerrada a neutrales y
    // enemigos): dentro hay un neutral (F3, al que la madre abrió la puerta), un aliado del conquistador (F5) y un enemigo suyo (F4, en guerra con F2). Salen los que ya no podrían entrar:
    // el neutral y el enemigo.
    let mut e = montar(m, 7, 2);
    e.abrir_a_los_neutrales();
    let (plaza, centro) = (e.plaza, e.centro);
    let (av, _, _) = e.avanzada_de(Etapa::Fuerte);
    e.ok("admin", Comando::FijarRelacion { a: 2, b: 5, relacion: Some(Relacion::Alianza) });
    e.ok("admin", Comando::FijarRelacion { a: 2, b: 4, relacion: Some(Relacion::Guerra) });
    let mut visitas = vec![];
    for (cuenta, faccion) in [("neu", 3), ("alia", 5), ("ene", 4)] {
        let (c, h) = e.visita(cuenta, faccion, &[Tropa::MiliciaLanceros]);
        e.ok(cuenta, Comando::EntrarEnAvanzada { avanzada: av, heroe: h });
        visitas.push((c, h));
    }
    let [(cn_neu, neutral), (cn_alia, aliado), (cn_ene, enemigo)] = visitas[..] else { unreachable!() };
    let (_, beto) = e.atacante(Punto::new(centro.x + 1_000, centro.y), &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    if let Some(fin) = e.n.protegida_hasta(plaza) {
        e.hasta(fin.max(e.n.ahora()));
    }
    e.ok("beto", Comando::Atacar { heroe: beto, objetivo: ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    tras_la_convocatoria(&mut e.n);
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2);
    assert_eq!(e.ubicacion(aliado), Ubicacion::Avanzada(av), "el aliado del conquistador se queda dentro");
    assert!(!e.n.en_el_mapa(cn_alia));
    assert_eq!((e.ubicacion(enemigo), e.n.en_el_mapa(cn_ene)), (Ubicacion::Columna(cn_ene), true), "el enemigo del conquistador sale al mapa con la columna con la que entró");
    assert_eq!((e.ubicacion(neutral), e.n.en_el_mapa(cn_neu)), (Ubicacion::Columna(cn_neu), true), "y el neutral, con la puerta de por defecto");
    let echados = hechos(&e.n, "heroe.expulsado");
    assert_eq!(echados.len(), 2);
    for x in &echados {
        assert_eq!((dato(x, "motivo"), dato(x, "lugar"), x.2.clone()), (2, 2, vec![plaza]));
    }
    let mut fuera: Vec<i64> = echados.iter().map(|x| dato(x, "heroe")).collect();
    fuera.sort_unstable();
    assert_eq!(fuera, { let mut v = vec![neutral as i64, enemigo as i64]; v.sort_unstable(); v });

    // Ruina de la madre: una ciudadela queda como ruina sin dueño y la atalaya desaparece; los dos sitios se vacían.
    let mut e = montar(m, 7, 3);
    e.abrir_a_los_neutrales();
    let plaza = e.plaza;
    let (ciudadela, _, _) = e.avanzada_de(Etapa::Ciudadela);
    let otra = Punto::new(e.centro.x - 8_000, e.centro.y);
    let (atalaya, col2, _) = e.colocar_en(otra);
    e.acabar_obra(atalaya, col2);
    let (c1, h1) = e.visita("uno", 3, &[]);
    let (c2, h2) = e.columna("dos", 3, otra);
    let h2 = e.n.ejercito(c2).unwrap().columna.lider.max(h2);
    e.ok("uno", Comando::EntrarEnAvanzada { avanzada: ciudadela, heroe: h1 });
    e.ok("dos", Comando::EntrarEnAvanzada { avanzada: atalaya, heroe: h2 });
    for dia in 1..=200 {
        let t = e.n.ahora().max(dia * DIA);
        e.n.avanzar_hasta(t);
        if e.n.economia(plaza).is_none() {
            break;
        }
    }
    assert!(e.n.economia(plaza).is_none(), "la plaza cayó en ruinas");
    assert_eq!((e.ubicacion(h1), e.n.en_el_mapa(c1), e.ubicacion(h2), e.n.en_el_mapa(c2)), (Ubicacion::Columna(c1), true, Ubicacion::Columna(c2), true));
    assert!(hechos(&e.n, "heroe.expulsado").iter().all(|x| dato(x, "motivo") == 1 && dato(x, "lugar") == 2 && x.2.is_empty()), "el hecho no nombra a la plaza que ya no está");
    assert_eq!(hechos(&e.n, "heroe.expulsado").len(), 2);
}

// ───────────────────────────── 13. Unity: defender una avanzada desde dentro ─────────────────────────────

#[test]
fn por_unity_de_oficio_solo_defiende_la_guarnicion_los_amigos_de_dentro_se_unen_y_al_ataque_solo_se_une_quien_esta_fuera() {
    let m = con_mundo!();
    let mut e = montar_con(Nucleo::nuevo(arranque_unity(m, 7)), m, 2);
    e.abrir_a_los_neutrales();
    let (plaza, pos) = (e.plaza, e.sitio());
    let (av, col, ana) = e.avanzada_de(Etapa::Fuerte);
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 3, relacion: Some(Relacion::Alianza) });
    // Dentro: la residente «ana» con su élite y 30 000 de madera; un aliado (F3) y un neutral (F4) de visita; y un héroe de la Facción 2 (la atacante) que entró como neutral.
    e.residir(ana);
    ELITE.iter().for_each(|&t| {
        e.dar_escuadra(ana, t);
    });
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 30_000 });
    let madera0 = e.carga_en(col, Madera);
    let aliado = e.visita("alia", 3, &[Tropa::MiliciaLanceros]);
    let neutral = e.visita("neu", 4, &[Tropa::MiliciaLanceros]);
    let intruso = e.visita("intr", 2, &[Tropa::MiliciaLanceros]);
    for (cuenta, h) in [("ana", ana), ("alia", aliado.1), ("neu", neutral.1), ("intr", intruso.1)] {
        e.ok(cuenta, Comando::EntrarEnAvanzada { avanzada: av, heroe: h });
    }
    let (cb, beto) = e.ejercito_fuerte(pos);
    let (_, refuerzo) = e.atacante(Punto::new(pos.x + 1_500, pos.y), &[Tropa::MiliciaLanceros]);
    e.guerra();
    humano(&mut e, beto);
    humano(&mut e, ana);
    let r = e.atacar(beto, ObjetivoAtaque::Avanzada(av));
    assert!(r.aceptada(), "{r:?}");
    let id = e.n.batallas().next().expect("se abrió una batalla").id;
    let b = e.n.batalla(id).unwrap();
    // De oficio solo defiende la guarnición de serie: ni la residente que está dentro ni el aliado ni el neutral están en el ticket. Se abre en Unity aunque el atacante sea un bot (toda batalla va a Unity con servidores).
    let defensor = &b.ticket.defensor;
    assert!(defensor.participantes.is_empty());
    assert_eq!(defensor.sin_heroe.iter().filter(|s| s.dueno.is_none()).count(), 5);
    // Quien está dentro de la avanzada atacada y no se ha unido a la defensa solo puede salir (cuarta ronda): cualquier otra orden sigue bloqueada hasta que acabe.
    assert_eq!(e.rechazada("alia", Comando::VaciarCarro { heroe: aliado.1 }), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(e.rechazada("neu", Comando::VaciarCarro { heroe: neutral.1 }), rechazos::BATALLA_BLOQUEADO);
    // La residente y el aliado, dentro, se unen a la defensa (la residente con las escuadras de su columna); el neutral no es amigo del dueño y desde dentro no ataca; el de la Facción atacante
    // que está dentro no se une al ataque desde dentro.
    let r = e.ord("ana", Comando::UnirseABatalla { heroe: ana, batalla: id, lado: None });
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(e.n.batalla(id).unwrap().participante(ana).unwrap().escuadras.len(), ELITE.len());
    let r = e.ord("alia", Comando::UnirseABatalla { heroe: aliado.1, batalla: id, lado: None });
    assert!(r.aceptada(), "{r:?}");
    let r = e.ord("neu", Comando::UnirseABatalla { heroe: neutral.1, batalla: id, lado: None });
    assert!(matches!(&r.resultado, Resultado::Rechazada { mensaje, .. } if mensaje.contains("hay que estar fuera")), "{r:?}");
    let r = e.ord("intr", Comando::UnirseABatalla { heroe: intruso.1, batalla: id, lado: None });
    assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA);
    assert!(matches!(&r.resultado, Resultado::Rechazada { mensaje, .. } if mensaje.contains("hay que estar fuera")), "{r:?}");
    let r = e.ord("intr", Comando::UnirseABatalla { heroe: intruso.1, batalla: id, lado: Some(Lado::Atacante) });
    assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA, "pedir el bando atacante tampoco lo salva");
    // Quien está fuera, en el mapa, de la Facción atacante sí se une al ataque.
    let r = e.ord("beto", Comando::UnirseABatalla { heroe: refuerzo, batalla: id, lado: None });
    assert!(r.aceptada(), "{r:?}");
    let b = e.n.batalla(id).unwrap();
    assert_eq!((b.lado_de(ana), b.lado_de(aliado.1), b.lado_de(refuerzo), b.lado_de(intruso.1), b.lado_de(neutral.1)), (Some(Lado::Defensor), Some(Lado::Defensor), Some(Lado::Atacante), None, None));

    // Gana el atacante: el fuerte queda en ruinas, todos los defensores, la residente y el aliado que se unió, pierden la mitad del carro (es del vencedor), y todos salen al mapa con lo suyo.
    let t = e.n.ahora() + MIN;
    en_curso(&mut e.n, id, t, &[beto, ana]);
    let res = resultado(&e.n, id, Lado::Atacante, 2);
    let r = msg(&mut e.n, t + 5 * MIN, "res", Comando::AplicarResultado { batalla: id, resultado: res });
    assert!(r.aceptada(), "{r:?}");
    assert!(matches!(e.estado_de(av), EstadoAvanzada::EnRuinas { .. }));
    assert_eq!(e.carga_en(col, Madera), madera0 - madera0 / 2, "la residente pierde la mitad de su carro");
    assert!(e.carga_en(cb, Madera) > 0, "y es del vencedor");
    assert_eq!(e.carga_en(aliado.0, Madera), 0);
    for (nombre, c, h) in [("ana", col, ana), ("alia", aliado.0, aliado.1), ("neu", neutral.0, neutral.1), ("intr", intruso.0, intruso.1)] {
        assert_eq!((e.ubicacion(h), e.n.en_el_mapa(c)), (Ubicacion::Columna(c), true), "{nombre} sale al mapa con su columna");
    }
    assert_eq!(hechos(&e.n, "heroe.expulsado").len(), 4);
    assert_eq!(hechos(&e.n, "batalla.aplicada")[0].2, vec![plaza]);
    let _ = pos;
}

/// Lo mismo en una avanzada atacada: el aliado que se une a defender no sale hasta que acaba; el neutral, dentro y sin unirse, solo puede salir (cualquier otra orden se rechaza) y,
/// fuera, se une al ataque.
#[test]
fn durante_el_ataque_a_una_avanzada_quien_esta_dentro_y_no_se_unio_solo_puede_salir_y_el_defensor_unido_no_sale() {
    let m = con_mundo!();
    let mut e = montar_con(Nucleo::nuevo(arranque_unity(m, 7)), m, 2);
    e.abrir_a_los_neutrales();
    let pos = e.sitio();
    let (av, _, _) = e.avanzada_de(Etapa::Fuerte);
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 3, relacion: Some(Relacion::Alianza) });
    let aliado = e.visita("alia", 3, &[Tropa::MiliciaLanceros]);
    let neutral = e.visita("neu", 4, &[Tropa::MiliciaLanceros]);
    for (cuenta, h) in [("alia", aliado.1), ("neu", neutral.1)] {
        e.ok(cuenta, Comando::EntrarEnAvanzada { avanzada: av, heroe: h });
    }
    let (_, beto) = e.ejercito_fuerte(pos);
    e.guerra();
    let r = e.atacar(beto, ObjetivoAtaque::Avanzada(av));
    assert!(r.aceptada(), "{r:?}");
    let id = e.n.batallas().next().expect("se abrió una batalla").id;
    let r = e.ord("alia", Comando::UnirseABatalla { heroe: aliado.1, batalla: id, lado: None });
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(e.rechazada("alia", Comando::SalirDeAvanzada { avanzada: av, heroe: aliado.1 }), rechazos::BATALLA_BLOQUEADO, "el defensor unido no sale");
    assert_eq!(e.rechazada("neu", Comando::VaciarCarro { heroe: neutral.1 }), rechazos::BATALLA_BLOQUEADO, "dentro, sin unirse, solo se puede salir");
    e.ok("neu", Comando::SalirDeAvanzada { avanzada: av, heroe: neutral.1 });
    assert_eq!((e.ubicacion(neutral.1), e.n.en_el_mapa(neutral.0)), (Ubicacion::Columna(neutral.0), true));
    let r = e.ord("neu", Comando::UnirseABatalla { heroe: neutral.1, batalla: id, lado: None });
    assert!(r.aceptada(), "fuera, el neutral se une al ataque: {r:?}");
    assert_eq!(e.n.batalla(id).unwrap().lado_de(neutral.1), Some(Lado::Atacante));
}

// ───────────────────────────── 14. Un guion con gente dentro: determinismo y foto ─────────────────────────────

/// Una atalaya con gente dentro: se levanta, un aliado con su élite y un neutral entran, se cierra la puerta a los neutrales (sale uno) y un ejército fuerte la ataca (resiste
/// el aliado desde dentro); el aliado sale, entra la residente con su carro y otro ejército la derriba (ella pierde la mitad del carro y sale al mapa). Cada orden lleva su
/// instante: `avanzar` solo decide cómo se mueve el reloj entre ellas.
fn guion_con_gente(e: &mut Esc, avanzar: &dyn Fn(&mut Esc, Instante)) {
e.abrir_a_los_neutrales();
    let (plaza, pos) = (e.plaza, e.sitio());
    // Cada columna nace en el instante en que se usa, sea cual sea el modo de avanzar (una columna de administración, sin ración, se muere de hambre en minutos).
    let (col, ana) = e.columna_en(0, "ana", 1, pos, &[Tropa::Lenadores]);
    e.residir(ana);
    let av = e.ok_en(MIN, "ana", Comando::ColocarAvanzada { asentamiento: plaza, heroe: ana, posicion: pos }).creadas[0];
    e.ok_en(MIN, "admin", Comando::FijarRelacion { a: 1, b: 3, relacion: Some(Relacion::Alianza) });
    avanzar(e, 20 * MIN);
    for (r, n) in [(Madera, 200_000), (Piedra, 100_000)] {
        e.ok_en(20 * MIN, "admin", Comando::DarSuministro { ejercito: col, recurso: r, cantidad: n });
    }
    e.ok_en(20 * MIN + 17_345, "ana", Comando::EntregarMaterialesAAvanzada { ejercito: col, avanzada: av });
    e.ok_en(20 * MIN + 17_345, "admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 31_000 });
    avanzar(e, 4 * HORA);
    let (_, aliado) = e.columna_en(4 * HORA, "alia", 3, pos, &ELITE);
    let (_, neutral) = e.columna_en(4 * HORA, "neu", 4, pos, &[Tropa::MiliciaLanceros]);
    e.ok_en(4 * HORA, "alia", Comando::EntrarEnAvanzada { avanzada: av, heroe: aliado });
    e.ok_en(4 * HORA, "neu", Comando::EntrarEnAvanzada { avanzada: av, heroe: neutral });
    avanzar(e, 5 * HORA);
    e.ok_en(5 * HORA, "ana", Comando::FijarAcceso { asentamiento: plaza, aliados: true, neutrales: false, enemigos: false });
    avanzar(e, 9 * HORA);
    let (_, primero) = e.columna_en(9 * HORA, "beto", 2, Punto::new(pos.x + 1_000, pos.y), &FUERTE);
    e.ok_en(9 * HORA, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    e.ok_en(9 * HORA, "beto", Comando::Atacar { heroe: primero, objetivo: ObjetivoAtaque::Avanzada(av), declara_guerra: false });
    avanzar(e, 10 * HORA);
    e.ok_en(10 * HORA, "alia", Comando::SalirDeAvanzada { avanzada: av, heroe: aliado });
    e.ok_en(10 * HORA, "ana", Comando::EntrarEnAvanzada { avanzada: av, heroe: ana });
    avanzar(e, 12 * HORA);
    let (_, segundo) = e.columna_en(12 * HORA, "beto", 2, Punto::new(pos.x + 1_000, pos.y), &FUERTE);
    e.ok_en(12 * HORA, "beto", Comando::Atacar { heroe: segundo, objetivo: ObjetivoAtaque::Avanzada(av), declara_guerra: false });
    avanzar(e, 20 * HORA);
}

fn correr_con_gente(m: &M, semilla: u64, avanzar: &dyn Fn(&mut Esc, Instante)) -> Nucleo {
    let mut e = montar(m, semilla, 2);
    guion_con_gente(&mut e, avanzar);
    e.n.avanzar_hasta(20 * HORA);
    e.n
}

#[test]
fn entrar_defender_y_ser_echado_de_una_avanzada_da_el_mismo_hash_normal_acelerado_y_a_trozos() {
    let m = con_mundo!();
    let normal = correr_con_gente(m, 3, &|e, t| e.n.avanzar_hasta(t));
    assert_eq!(normal.hash_estado(), correr_con_gente(m, 3, &|e, t| e.n.avanzar_hasta(t)).hash_estado(), "repetible");
    assert_eq!(normal.hash_estado(), correr_con_gente(m, 3, &|_, _| {}).hash_estado(), "acelerado de un golpe");
    let a_trozos = correr_con_gente(m, 3, &|e, t| {
        let ahora = e.n.ahora();
        for s in 1..=5 {
            e.n.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 25);
        }
    });
    assert_eq!(normal.hash_estado(), a_trozos.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    assert_ne!(normal.hash_estado(), correr_con_gente(m, 4, &|e, t| e.n.avanzar_hasta(t)).hash_estado(), "otra semilla, otro mundo");
    // El guion hizo de verdad lo que se espera.
    assert_eq!(hechos(&normal, "jugador.entra_en_avanzada").len(), 3);
    assert_eq!(hechos(&normal, "jugador.sale_de_avanzada").len(), 1);
    assert_eq!(hechos(&normal, "combate.avanzada_resiste").len(), 1, "el aliado defiende desde dentro");
    assert_eq!(hechos(&normal, "combate.avanzada_destruida").len(), 1);
    assert_eq!(hechos(&normal, "heroe.expulsado").iter().map(|h| (dato(h, "lugar"), dato(h, "motivo"))).collect::<Vec<_>>(), vec![(2, 0), (2, 3)], "el neutral al cerrar la puerta y la residente al destruirse");
    // Nada recurre por una entidad ociosa: como mucho un evento de deserción vigente por columna, esté dentro de una avanzada o en el mapa.
    let vigentes = normal.estado().cola.volcar().iter().filter(|ev| normal.estado().generaciones.es_vigente(ev.clave) && matches!(ev.carga, Carga::Militar(SucesoMilitar::Desercion))).count();
    assert!(vigentes <= normal.estado().ejercitos.len(), "{vigentes} deserciones vigentes");
}

#[test]
fn una_foto_con_un_heroe_dentro_de_una_avanzada_se_recupera_y_sigue_igual_que_el_mundo_sin_interrumpir() {
    let m = con_mundo!();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    let guion = |e: &mut Esc| {
        e.abrir_a_los_neutrales();
        let (av, _, _) = e.atalaya();
        let (col, h) = e.visita("neu", 3, &[Tropa::MiliciaLanceros]);
        e.ok("neu", Comando::EntrarEnAvanzada { avanzada: av, heroe: h });
        (av, col, h)
    };
    // Sin parar: la referencia.
    let referencia = {
        let mut e = montar(m, 9, 2);
        let (av, _, h) = guion(&mut e);
        e.avanzar(HORA);
        e.ok("neu", Comando::SalirDeAvanzada { avanzada: av, heroe: h });
        e.avanzar(HORA);
        e.n.hash_estado()
    };
    let dir = tempfile::tempdir().unwrap();
    let (a_mitad, plaza, centro, cuenta, av, col, h) = {
        let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let mut e = montar_con(n, m, 2);
        let (av, col, h) = guion(&mut e);
        let bytes = e.n.estado().serializar();
        assert_eq!(&bytes[..5], b"BAN33");
        let vuelta = nucleo::Estado::deserializar(&bytes, &Reglas::default(), &e.n.estado().mundo_hash).unwrap();
        assert_eq!(vuelta.militar.heroes[&h].heroe.presencia.ubicacion, Ubicacion::Avanzada(av));
        assert_eq!((vuelta.serializar(), vuelta.hash()), (bytes, e.n.estado().hash()));
        e.n.foto_ahora();
        e.avanzar(HORA);
        e.n.apagar().unwrap();
        (e.n.hash_estado(), e.plaza, e.centro, e.cuenta, av, col, h)
    };
    let n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo, con el héroe dentro de la avanzada");
    let mut e = Esc { n, plaza, centro, cuenta };
    assert!(!e.n.en_el_mapa(col));
    e.ok("neu", Comando::SalirDeAvanzada { avanzada: av, heroe: h });
    e.avanzar(HORA);
    assert_eq!(e.n.hash_estado(), referencia, "el mundo recuperado sigue igual que el que no se interrumpió");
    e.n.apagar().unwrap();
}

// ───────────────────────────── La niebla: la avanzada es un ojo de 12 (4H T3d) ─────────────────────────────

#[test]
fn la_avanzada_operativa_ve_12_explora_su_disco_y_deja_de_ver_al_destruirla() {
    use contrato::v1::TipoEntidad;
    use nucleo::proyeccion::vista::{Mirada, Perspectiva};
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (av, col, heroe) = e.atalaya();
    let pos = e.avanzada(av).posicion;
    assert!(e.n.estado().memoria.exploracion[&1].explorado_en(Punto::new(pos.x + 11_500, pos.y)), "la avanzada explora su disco al quedar operativa");
    // La columna de Ana se va lejos: solo la plaza (15 alrededor de su centro, a 8 u al oeste) y la avanzada (12) ven por F1.
    e.ok("ana", Comando::MarcharA { ejercito: col, destino: Punto::new(e.centro.x - 80_000, e.centro.y) });
    let fin = e.n.ejercito(col).unwrap().plan.fin().unwrap();
    e.hasta(fin + MIN);
    let (cerca, lejos) = (e.columna("beto", 2, Punto::new(pos.x + 11_500, pos.y)).0, e.columna("gus", 3, Punto::new(pos.x + 13_000, pos.y)).0);
    let ve = |e: &Esc| {
        let rect = vision::Rect { min: Punto::new(0, 0), max: Punto::new(2_667_000, 2_000_000) };
        let m = Mirada::de(&e.n, Perspectiva::Faccion(1));
        let ids: Vec<String> = m.contenido(None, rect, &Default::default()).into_iter().filter(|v| v.clave.0 == TipoEntidad::Ejercito as i32).map(|v| v.clave.1).collect();
        (ids.contains(&nucleo::proyeccion::ids::ejercito(cerca)), ids.contains(&nucleo::proyeccion::ids::ejercito(lejos)))
    };
    assert_eq!(ve(&e), (true, false), "a 11,5 de la avanzada se ve; a 13, no");
    e.ok("ana", Comando::DestruirAvanzada { heroe, avanzada: av });
    assert_eq!(ve(&e), (false, false), "destruida, no ve");
}

/// Un asedio con números se resuelve al acabar su convocatoria de 30 s (calibración del sistema 4, M8): deja pasar las que haya abiertas.
fn tras_la_convocatoria(n: &mut Nucleo) {
    let fin = n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max();
    if let Some(h) = fin {
        n.avanzar_hasta(h);
    }
}
