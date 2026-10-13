//! La población sintética: Facciones que nacen en un campamento, buscan sitio, marchan, fundan y crecen. Un planificador
//! propio (cola de acciones por instante) avanza el núcleo con `avanzar_hasta` y aplica cada acción con `aplicar_en`.

use crate::geo::{Cargado, Geo};
use base::{Instante, Punto, Rng, MS_POR_MINUTO};
use nucleo::{rechazos, Ajustes, Arranque, Comando, InformeViabilidad, Nucleo, Peticion, Reglas, Resultado};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::time::{Duration, Instant};
use territorio::Viabilidad;

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;

// ── Supuestos del batch (hipótesis sintéticas, se anotan en `parametros` del informe) ──
pub const NACEN_EN_HORAS: i64 = 48;
pub const CANDIDATOS: usize = 25;
pub const ANILLO_CAMPAMENTO: (i64, i64) = (30_000, 120_000);
pub const ANILLO_PLAZA: (i64, i64) = (30_000, 70_000);
pub const REINTENTO_MIN: i64 = 10;
pub const MAX_REINTENTOS: u32 = 6;
pub const EDIFICIO_CADA_MIN: i64 = 20;
pub const MAX_EDIFICIOS: u32 = 30;
pub const LENERA_A_MIN: [i64; 3] = [30, 90, 180];
pub const NIVEL_2_H: i64 = 48;
pub const NIVEL_3_H: i64 = 144;
pub const EXPANSION_CADA_H: i64 = 12;
pub const VELOCIDAD: i64 = 500;
pub const VIVERES: i64 = 350_000;

/// Una fundación lograda.
pub struct Fundacion {
    pub instante: Instante,
    pub primera: bool,
    /// Desde el nacimiento (la primera) o desde la orden de buscar sitio (las expansiones).
    pub tras_ms: i64,
    pub verde: bool,
}

/// Un intento de Leñera.
pub struct Lenera {
    pub anillo: usize,
    /// `ok`, `parcela.sin_libres`, `parcela.cupo_agotado` u otro código de rechazo.
    pub resultado: String,
}

#[derive(Default)]
pub struct Registros {
    /// Motivo → veces rechazado (candidatos rojos, rutas imposibles, fundaciones fallidas).
    pub rechazos: BTreeMap<String, u64>,
    pub fundaciones: Vec<Fundacion>,
    /// Búsquedas abandonadas tras agotar los reintentos: (Facción, era la primera plaza).
    pub renuncias: Vec<(u32, bool)>,
    pub lenera: Vec<Lenera>,
    pub viab_llamadas: u64,
    pub viab_tiempo: Duration,
}

/// Lo que deja una simulación.
pub struct Corrida {
    pub nucleo: Nucleo,
    pub reg: Registros,
    pub facciones: u32,
    pub pared: Duration,
}

#[derive(Clone, Copy, Debug)]
enum Accion {
    Nace(u32),
    Buscar { f: u32, reintento: u32, desde: Instante, primera: bool },
    Fundar { f: u32, reintento: u32, desde: Instante, primera: bool },
    Expansion(u32),
    Edificio { plaza: u64, n: u32 },
    Lenera { plaza: u64 },
    NivelPlaza { plaza: u64, nivel: u32 },
    NivelFaccion { f: u32, nivel: u32 },
}

struct Faccion {
    ejercito: u64,
    campamento: usize,
    /// Hay una búsqueda o una marcha en curso.
    ocupada: bool,
    plazas: Vec<u64>,
}

struct Sim<'a> {
    n: Nucleo,
    geo: &'a Geo,
    rng: Rng,
    fin: Instante,
    cola: BinaryHeap<Reverse<(Instante, usize)>>,
    acciones: Vec<Accion>,
    facciones: Vec<Faccion>,
    reg: Registros,
    contador: u64,
}

/// El núcleo recién arrancado sobre el mundo (semilla del núcleo = semilla del batch).
pub fn nucleo_nuevo(c: &Cargado, semilla: u64) -> Nucleo {
    Nucleo::nuevo(Arranque {
        reglas: Reglas::default(),
        ajustes: Ajustes::default(),
        semilla,
        inicio: 0,
        rutas: c.rutas.clone(),
        mundo_hash: c.mundo.hash().into(),
        mundo: Some(c.mundo.clone()),
    })
}

/// Corre `facciones` Facciones durante `dias` días de mundo.
pub fn simular(c: &Cargado, geo: &Geo, facciones: u32, dias: u32, semilla: u64) -> Corrida {
    let t0 = Instant::now();
    let mut s = Sim {
        n: nucleo_nuevo(c, semilla),
        geo,
        rng: Rng::derivado(semilla, "batch-territorio"),
        fin: dias as i64 * 24 * HORA,
        cola: BinaryHeap::new(),
        acciones: Vec::new(),
        facciones: Vec::new(),
        reg: Registros::default(),
        contador: 0,
    };
    s.nacimientos(facciones);
    s.correr();
    Corrida { nucleo: s.n, reg: s.reg, facciones, pared: t0.elapsed() }
}

impl Sim<'_> {
    fn programar(&mut self, t: Instante, a: Accion) {
        if t <= self.fin {
            self.acciones.push(a);
            self.cola.push(Reverse((t, self.acciones.len() - 1)));
        }
    }

    /// Instantes de nacimiento repartidos al azar por las primeras 48 h y ordenados: la Facción `i` nace en el `i`-ésimo.
    fn nacimientos(&mut self, facciones: u32) {
        let mut ts: Vec<i64> = (0..facciones).map(|_| self.rng.por_debajo((NACEN_EN_HORAS * HORA) as u64) as i64).collect();
        ts.sort_unstable();
        for (i, t) in ts.into_iter().enumerate() {
            self.programar(t, Accion::Nace(i as u32 + 1));
        }
    }

    fn correr(&mut self) {
        while let Some(Reverse((t, i))) = self.cola.pop() {
            self.n.avanzar_hasta(t);
            match self.acciones[i] {
                Accion::Nace(f) => self.nace(t, f),
                Accion::Buscar { f, reintento, desde, primera } => self.buscar(t, f, reintento, desde, primera),
                Accion::Fundar { f, reintento, desde, primera } => self.fundar(t, f, reintento, desde, primera),
                Accion::Expansion(f) => self.expansion(t, f),
                Accion::Edificio { plaza, n } => self.edificio(t, plaza, n),
                Accion::Lenera { plaza } => self.lenera(t, plaza),
                Accion::NivelPlaza { plaza, nivel } => { self.admin(t, Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel }); },
                Accion::NivelFaccion { f, nivel } => { self.admin(t, Comando::FijarNivelFaccion { faccion: f, nivel }); },
            }
        }
        self.n.avanzar_hasta(self.fin);
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    fn peticion(&mut self, actor: &str, c: Comando) -> Peticion {
        self.contador += 1;
        let p = Peticion::nueva(actor, &format!("b{}", self.contador), c);
        if actor == "admin" {
            p.de_admin()
        } else {
            p
        }
    }

    fn admin(&mut self, t: Instante, c: Comando) -> nucleo::Respuesta {
        let p = self.peticion("admin", c);
        self.n.aplicar_en(t, p)
    }

    fn como_batch(&mut self, t: Instante, c: Comando) -> nucleo::Respuesta {
        let p = self.peticion("batch", c);
        self.n.aplicar_en(t, p)
    }

    fn rechazo(&mut self, motivo: &str) {
        *self.reg.rechazos.entry(motivo.to_string()).or_insert(0) += 1;
    }

    fn viab(&mut self, p: Punto, f: u32) -> InformeViabilidad {
        let t0 = Instant::now();
        let v = self.n.viabilidad(p, f).expect("el núcleo tiene mundo");
        self.reg.viab_llamadas += 1;
        self.reg.viab_tiempo += t0.elapsed();
        v
    }

    // ───────────────────────────── Acciones ─────────────────────────────

    fn nace(&mut self, t: Instante, f: u32) {
        let campamento = f as usize % self.geo.campamentos.len();
        let ejercito = self.n.crear_ejercito(t, &format!("h{f}"), "batch", f, self.geo.campamentos[campamento], VELOCIDAD, VIVERES, VIVERES).expect("alta de la columna");
        self.admin(t, Comando::DarCaravanaFundacion { ejercito });
        self.facciones.push(Faccion { ejercito, campamento, ocupada: true, plazas: Vec::new() });
        self.programar(t, Accion::Buscar { f, reintento: 0, desde: t, primera: true });
    }

    fn faccion(&self, f: u32) -> &Faccion {
        &self.facciones[f as usize - 1]
    }

    /// Un punto al azar en el anillo `[rmin, rmax]` alrededor de `c`.
    fn punto_en_anillo(&mut self, c: Punto, (rmin, rmax): (i64, i64)) -> Punto {
        let r = (rmin + self.rng.por_debajo((rmax - rmin) as u64) as i64) as f64;
        let a = (self.rng.por_debajo(360_000) as f64 / 1000.0).to_radians();
        Punto::new(c.x + (r * a.cos()) as i64, c.y + (r * a.sin()) as i64)
    }

    /// Centro y anillo de la búsqueda: el campamento la primera vez; después, una plaza propia al azar.
    fn origen_de_busqueda(&mut self, f: u32, primera: bool) -> (Punto, (i64, i64)) {
        let fa = self.faccion(f);
        if primera || fa.plazas.is_empty() {
            return (self.geo.campamentos[fa.campamento], ANILLO_CAMPAMENTO);
        }
        let propias = fa.plazas.clone();
        let id = propias[self.rng.por_debajo(propias.len() as u64) as usize];
        (self.n.estado().asentamientos[&id].posicion, ANILLO_PLAZA)
    }

    /// Hasta 25 candidatos: el primer Verde, o si no hay, los Ámbar de mejor a peor. Los Rojo solo cuentan su motivo.
    fn candidatos(&mut self, f: u32, primera: bool) -> Vec<Punto> {
        let (centro, anillo) = self.origen_de_busqueda(f, primera);
        let mut ambar: Vec<(u8, usize, Punto)> = Vec::new();
        for _ in 0..CANDIDATOS {
            let p = self.punto_en_anillo(centro, anillo);
            let v = self.viab(p, f);
            if let Some(m) = v.motivo {
                self.rechazo(m.codigo());
            } else if v.color == Viabilidad::Verde {
                return vec![p];
            } else {
                ambar.push(((v.parcelas_libres > 0) as u8 + (v.piedra > 0) as u8, v.parcelas_libres + v.piedra, p));
            }
        }
        ambar.sort_by_key(|&(a, b, _)| Reverse((a, b)));
        ambar.into_iter().map(|(_, _, p)| p).collect()
    }

    fn buscar(&mut self, t: Instante, f: u32, reintento: u32, desde: Instante, primera: bool) {
        let ejercito = self.faccion(f).ejercito;
        for sitio in self.candidatos(f, primera) {
            let r = self.como_batch(t, Comando::MarcharA { ejercito, destino: sitio });
            let llega = match &r.resultado {
                Resultado::Aceptada => self.n.estado().ejercitos[&ejercito].plan.fin().unwrap_or(t),
                Resultado::SinCambios => t,
                Resultado::Rechazada { codigo, .. } => {
                    self.rechazo(codigo);
                    continue;
                }
                Resultado::Duplicada => unreachable!("los ids de petición no se repiten"),
            };
            self.programar(llega.max(t), Accion::Fundar { f, reintento, desde, primera });
            return;
        }
        // Todo era Rojo (o sin ruta): esperar 10 minutos y volver a mirar.
        self.reintentar(t + REINTENTO_MIN * MIN, f, reintento, desde, primera);
    }

    fn reintentar(&mut self, t: Instante, f: u32, reintento: u32, desde: Instante, primera: bool) {
        if reintento >= MAX_REINTENTOS {
            self.reg.renuncias.push((f, primera));
            self.facciones[f as usize - 1].ocupada = false;
        } else {
            self.programar(t, Accion::Buscar { f, reintento: reintento + 1, desde, primera });
        }
    }

    fn fundar(&mut self, t: Instante, f: u32, reintento: u32, desde: Instante, primera: bool) {
        let ejercito = self.faccion(f).ejercito;
        let aqui = self.n.estado().ejercitos[&ejercito].posicion_en(t);
        let v = self.viab(aqui, f);
        if let Some(m) = v.motivo {
            self.rechazo(m.codigo());
            return self.reintentar(t, f, reintento, desde, primera);
        }
        let r = self.como_batch(t, Comando::Fundar { ejercito });
        let Some(&plaza) = r.creadas.first().filter(|_| r.aceptada()) else {
            if let Resultado::Rechazada { codigo, .. } = &r.resultado {
                self.rechazo(codigo);
            }
            return self.reintentar(t, f, reintento, desde, primera);
        };
        self.reg.fundaciones.push(Fundacion { instante: t, primera, tras_ms: t - desde, verde: v.color == Viabilidad::Verde });
        self.tras_fundar(t, f, plaza);
    }

    /// Todo lo que sucede con una plaza recién fundada, y con la Facción si era la primera.
    fn tras_fundar(&mut self, t: Instante, f: u32, plaza: u64) {
        let primera = self.faccion(f).plazas.is_empty();
        self.facciones[f as usize - 1].plazas.push(plaza);
        self.facciones[f as usize - 1].ocupada = false;
        if primera {
            self.programar(t + NIVEL_2_H * HORA, Accion::NivelFaccion { f, nivel: 2 });
            self.programar(t + NIVEL_3_H * HORA, Accion::NivelFaccion { f, nivel: 3 });
            self.programar(t + EXPANSION_CADA_H * HORA, Accion::Expansion(f));
        }
        self.programar(t + NIVEL_2_H * HORA, Accion::NivelPlaza { plaza, nivel: 2 });
        self.programar(t + NIVEL_3_H * HORA, Accion::NivelPlaza { plaza, nivel: 3 });
        self.programar(t + EDIFICIO_CADA_MIN * MIN, Accion::Edificio { plaza, n: 1 });
        for m in LENERA_A_MIN {
            self.programar(t + m * MIN, Accion::Lenera { plaza });
        }
    }

    fn expansion(&mut self, t: Instante, f: u32) {
        self.programar(t + EXPANSION_CADA_H * HORA, Accion::Expansion(f));
        let fa = self.faccion(f);
        if fa.ocupada {
            return;
        }
        let nivel_plaza = fa.plazas.first().map_or(0, |id| self.n.estado().asentamientos[id].nivel);
        let nivel_faccion = self.n.estado().niveles_faccion.get(&f).copied().unwrap_or(1);
        let cap = territorio::cap_de_fundacion(nivel_faccion, &self.n.estado().reglas.territorio.cap_por_nivel) as usize;
        if nivel_plaza < 2 || fa.plazas.len() >= cap {
            return;
        }
        let ejercito = fa.ejercito;
        self.admin(t, Comando::DarCaravanaFundacion { ejercito });
        self.facciones[f as usize - 1].ocupada = true;
        self.programar(t, Accion::Buscar { f, reintento: 0, desde: t, primera: false });
    }

    /// Un edificio completado agranda la zona una unidad.
    fn edificio(&mut self, t: Instante, plaza: u64, n: u32) {
        self.admin(t, Comando::AgrandarZona { asentamiento: plaza, milesimas: 1_000 });
        if n < MAX_EDIFICIOS {
            self.programar(t + EDIFICIO_CADA_MIN * MIN, Accion::Edificio { plaza, n: n + 1 });
        }
    }

    fn lenera(&mut self, t: Instante, plaza: u64) {
        let r = self.admin(t, Comando::ReclamarParcela { asentamiento: plaza });
        let resultado = match &r.resultado {
            Resultado::Aceptada => "ok".to_string(),
            Resultado::Rechazada { codigo, .. } => codigo.to_string(),
            otro => format!("{otro:?}"),
        };
        let anillo = self.geo.anillo(self.n.estado().asentamientos[&plaza].posicion).0;
        self.reg.lenera.push(Lenera { anillo, resultado });
    }
}

/// Códigos de rechazo de la Leñera que el informe distingue.
pub const SIN_LIBRES: &str = rechazos::PARCELA_SIN_LIBRES;
pub const CUPO_AGOTADO: &str = rechazos::PARCELA_CUPO_AGOTADO;
