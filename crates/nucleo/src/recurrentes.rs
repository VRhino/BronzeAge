//! Las misiones recurrentes y el tablón (mecánica 55, `Docs/Mechanics/Misiones_Recurrentes.md`). Aquí está lo puro: el catálogo (que es dato y se cambia en caliente), los tablones
//! de cada plaza y cada Facción, el periodo de cada misión, aceptar, contar y cumplir, y la foto. Renovar, reclamar, elegir y de dónde sale cada avance son de `nucleo::recurrentes`.
//!
//! Formato en la foto (desde BAN50, little endian de `codec`): `catálogo (texto JSON) · origen i64 · renovación opt_i64 · n u64 · n × (dueño · tablón)`; el tablón es
//! `diarias · semanales · elegidas_diarias opt · elegidas_semanales opt · aceptadas_semana{n · (texto, u32)} · misiones{n · (definición (texto JSON), desde i64, hasta i64, progreso)}`
//! y las listas de ids, `n u64 · n × texto`.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::misiones::{escribir_dueno, leer_dueno, Dueno};
use base::{Instante, Mil, UNIDAD};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const DOC_RECURRENTES: &str = include_str!("../../../Docs/Mechanics/Misiones_Recurrentes.md");

/// Un día de mundo, en milisegundos.
pub const DIA: Instante = 24 * 60 * 60 * 1_000;
/// Una semana de mundo.
pub const SEMANA: Instante = 7 * DIA;
/// Los huecos de cada tablón: 7 diarias y 7 semanales (decisión del autor del 2026-10-28).
pub const HUECOS: usize = 7;

/// Los contadores que puede usar una recurrente (Misiones_Recurrentes §7): los del héroe y los dos de la participación.
pub const CONTADORES: &[&str] = &[
    "salidas", "prestamos", "llegadas", "repostajes", "escuadrones", "infanteria", "tiro", "caballeria", "uniones", "batallas", "bandidos", "alijos", "entregas", "defensas", "victorias",
];
/// Los cargos que puede pedir una recurrente de plaza (las claves de `asentamiento::Cargo`) y una de Facción.
pub const CARGOS_DE_PLAZA: &[&str] = &["gobernador", "tesorero", "general", "maestroObras", "sacerdote"];
pub const CARGOS_DE_FACCION: &[&str] = &["rey", "embajador"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ambito {
    Asentamiento,
    Faccion,
}

impl Ambito {
    pub fn de(d: Dueno) -> Option<Ambito> {
        match d {
            Dueno::Asentamiento(_) => Some(Ambito::Asentamiento),
            Dueno::Faccion(_) => Some(Ambito::Faccion),
            Dueno::Heroe(_) => None,
        }
    }
    fn clave(self) -> &'static str {
        match self {
            Ambito::Asentamiento => "asentamiento",
            Ambito::Faccion => "faccion",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Duracion {
    Dia,
    Semana,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    Individual { limite_semanal: u32 },
    Grupal,
}

/// Una misión recurrente del catálogo, con sus textos.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recurrente {
    pub id: String,
    pub ambito: Ambito,
    pub duracion: Duracion,
    pub tipo: Tipo,
    /// El cargo que hace falta para aceptarla (clave), si pide uno.
    pub cargo: Option<String>,
    pub condiciones: Vec<(String, u64)>,
    /// Experiencia de Facción, en milésimas.
    pub xp: Mil,
    /// Participación en la plaza (Participacion §2): la del héroe en una individual, la que se reparte en una grupal.
    pub participacion: u64,
    /// Oro al almacén personal del héroe, en milésimas (solo individuales).
    pub oro: Mil,
    pub titulo: String,
    pub texto: String,
    pub explicacion: String,
}

impl Recurrente {
    pub fn es_individual(&self) -> bool {
        matches!(self.tipo, Tipo::Individual { .. })
    }

    fn desde(ambito: Ambito, m: &Value) -> Result<Recurrente, String> {
        let id = m.get("id").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or("misión sin id")?.to_owned();
        let texto = |c: &str| m.get(c).and_then(Value::as_str).map(str::to_owned).ok_or_else(|| format!("{id}: sin «{c}»"));
        let duracion = match m.get("duracion").and_then(Value::as_str) {
            Some("dia") => Duracion::Dia,
            Some("semana") => Duracion::Semana,
            _ => return Err(format!("{id}: «duracion» es «dia» o «semana»")),
        };
        let tipo = match m.get("tipo").and_then(Value::as_str) {
            Some("individual") => Tipo::Individual {
                limite_semanal: m.get("limiteSemanal").and_then(Value::as_u64).filter(|&n| n > 0 && n <= u64::from(u32::MAX)).ok_or_else(|| format!("{id}: «limiteSemanal» > 0"))? as u32,
            },
            Some("grupal") => Tipo::Grupal,
            _ => return Err(format!("{id}: «tipo» es «individual» o «grupal»")),
        };
        let cargo = match m.get("cargo") {
            None | Some(Value::Null) => None,
            Some(c) => {
                let c = c.as_str().ok_or_else(|| format!("{id}: «cargo»"))?;
                let validos = if ambito == Ambito::Asentamiento { CARGOS_DE_PLAZA } else { CARGOS_DE_FACCION };
                if !validos.contains(&c) {
                    return Err(format!("{id}: el cargo «{c}» no es de su ámbito ({})", validos.join(", ")));
                }
                Some(c.to_owned())
            }
        };
        let condiciones: Vec<(String, u64)> = m
            .get("condiciones")
            .and_then(Value::as_object)
            .filter(|o| !o.is_empty())
            .ok_or_else(|| format!("{id}: sin condiciones"))?
            .iter()
            .map(|(k, n)| {
                if !CONTADORES.contains(&k.as_str()) {
                    return Err(format!("{id}: el contador «{k}» no existe"));
                }
                n.as_u64().filter(|&n| n > 0).map(|n| (k.clone(), n)).ok_or_else(|| format!("{id}: umbral de «{k}»"))
            })
            .collect::<Result<_, _>>()?;
        let mil = |c: &str| -> Result<Mil, String> {
            match m.get(c) {
                None => Ok(0),
                Some(x) => x.as_f64().filter(|f| *f >= 0.0).map(|f| (f * UNIDAD as f64).round() as Mil).ok_or_else(|| format!("{id}: «{c}»")),
            }
        };
        let (xp, oro) = (mil("xp")?, mil("oro")?);
        let participacion = match m.get("participacion") {
            None => 0,
            Some(x) => x.as_u64().ok_or_else(|| format!("{id}: «participacion»"))?,
        };
        if ambito == Ambito::Faccion && participacion > 0 {
            return Err(format!("{id}: la participación es de una plaza: una recurrente de Facción no la da"));
        }
        if oro > 0 && !matches!(tipo, Tipo::Individual { .. }) {
            return Err(format!("{id}: el oro va al almacén de un héroe: solo lo da una individual"));
        }
        Ok(Recurrente { id: id.clone(), ambito, duracion, tipo, cargo, condiciones, xp, participacion, oro, titulo: texto("titulo")?, texto: texto("texto")?, explicacion: texto("explicacion")? })
    }

    /// La forma JSON del catálogo (la inversa de [`Recurrente::desde`]).
    pub fn a_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("id".into(), json!(self.id));
        m.insert("duracion".into(), json!(if self.duracion == Duracion::Dia { "dia" } else { "semana" }));
        match self.tipo {
            Tipo::Individual { limite_semanal } => {
                m.insert("tipo".into(), json!("individual"));
                m.insert("limiteSemanal".into(), json!(limite_semanal));
            }
            Tipo::Grupal => {
                m.insert("tipo".into(), json!("grupal"));
            }
        }
        if let Some(c) = &self.cargo {
            m.insert("cargo".into(), json!(c));
        }
        m.insert("condiciones".into(), Value::Object(self.condiciones.iter().map(|(c, n)| (c.clone(), json!(n))).collect()));
        let mil = |x: Mil| if x % UNIDAD == 0 { json!(x / UNIDAD) } else { json!(x as f64 / UNIDAD as f64) };
        if self.xp > 0 {
            m.insert("xp".into(), mil(self.xp));
        }
        if self.participacion > 0 {
            m.insert("participacion".into(), json!(self.participacion));
        }
        if self.oro > 0 {
            m.insert("oro".into(), mil(self.oro));
        }
        m.insert("titulo".into(), json!(self.titulo));
        m.insert("texto".into(), json!(self.texto));
        m.insert("explicacion".into(), json!(self.explicacion));
        Value::Object(m)
    }
}

/// El catálogo: las recurrentes de cada ámbito, en orden.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalogo {
    pub misiones: Vec<Arc<Recurrente>>,
}

impl Catalogo {
    /// El catálogo inicial (`MISIONES_RECURRENTES` de Misiones_Recurrentes.md).
    pub fn cargar() -> Self {
        let v = asentamiento::datos::bloque_de(DOC_RECURRENTES, "MISIONES_RECURRENTES").unwrap_or_else(|e| panic!("{e}"));
        Self::desde(&v, &[]).unwrap_or_else(|e| panic!("bloque MISIONES_RECURRENTES: {e}"))
    }

    /// Lee y valida un catálogo entero; `prohibidos` son ids que no puede usar (los de las misiones únicas).
    pub fn desde(v: &Value, prohibidos: &[&str]) -> Result<Self, String> {
        let o = v.as_object().ok_or("el catálogo es un objeto con «asentamiento» y «faccion»")?;
        if let Some(k) = o.keys().find(|k| *k != "asentamiento" && *k != "faccion") {
            return Err(format!("ámbito desconocido «{k}»"));
        }
        let mut misiones = Vec::new();
        for ambito in [Ambito::Asentamiento, Ambito::Faccion] {
            let lista = match o.get(ambito.clave()) {
                None => continue,
                Some(l) => l.as_array().ok_or_else(|| format!("«{}» es una lista", ambito.clave()))?,
            };
            for m in lista {
                misiones.push(Arc::new(Recurrente::desde(ambito, m)?));
            }
        }
        let mut ids = BTreeSet::new();
        for m in &misiones {
            if !ids.insert(m.id.as_str()) {
                return Err(format!("id repetido «{}»", m.id));
            }
            if prohibidos.contains(&m.id.as_str()) {
                return Err(format!("«{}» es el id de una misión única", m.id));
            }
        }
        Ok(Catalogo { misiones })
    }

    /// Lee el JSON en texto.
    pub fn desde_texto(s: &str, prohibidos: &[&str]) -> Result<Self, String> {
        let v: Value = serde_json::from_str(s).map_err(|e| format!("JSON mal formado: {e}"))?;
        Self::desde(&v, prohibidos)
    }

    pub fn a_json(&self) -> Value {
        let lista = |a: Ambito| Value::Array(self.misiones.iter().filter(|m| m.ambito == a).map(|m| m.a_json()).collect());
        json!({ "asentamiento": lista(Ambito::Asentamiento), "faccion": lista(Ambito::Faccion) })
    }

    pub fn buscar(&self, id: &str) -> Option<&Arc<Recurrente>> {
        self.misiones.iter().find(|m| m.id == id)
    }

    /// Las de un ámbito y una duración, en el orden del catálogo.
    pub fn de(&self, ambito: Ambito, duracion: Duracion) -> impl Iterator<Item = &Arc<Recurrente>> {
        self.misiones.iter().filter(move |m| m.ambito == ambito && m.duracion == duracion)
    }

    /// Lo que sale de una lista elegida (o, sin elección, las primeras del catálogo): las que siguen en el catálogo con ese ámbito y esa duración, hasta [`HUECOS`].
    pub fn resolver(&self, ambito: Ambito, duracion: Duracion, elegidas: Option<&[String]>) -> Vec<String> {
        match elegidas {
            Some(l) => l.iter().filter(|id| self.buscar(id).is_some_and(|m| m.ambito == ambito && m.duracion == duracion)).take(HUECOS).cloned().collect(),
            None => self.de(ambito, duracion).take(HUECOS).map(|m| m.id.clone()).collect(),
        }
    }
}

impl Default for Catalogo {
    fn default() -> Self {
        static C: std::sync::OnceLock<Catalogo> = std::sync::OnceLock::new();
        C.get_or_init(Catalogo::cargar).clone()
    }
}

/// El estado de una aceptación (o de una grupal).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situacion {
    Aceptada,
    Cumplida,
    Reclamada,
    Caducada,
}

/// La misión de un héroe que la aceptó.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Aceptacion {
    pub contadores: BTreeMap<String, u64>,
    pub situacion: Situacion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progreso {
    /// Héroe → su aceptación, en el orden en que aceptaron (por id).
    Individual(BTreeMap<u64, Aceptacion>),
    /// Lo de todos: el avance, lo que aportó cada héroe y su situación (`Aceptada` mientras corre).
    Grupal { contadores: BTreeMap<String, u64>, aportes: BTreeMap<u64, u64>, situacion: Situacion },
}

/// Una misión en el tablón: su definición tal como salió, su periodo y su avance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnTablon {
    pub def: Arc<Recurrente>,
    pub desde: Instante,
    pub hasta: Instante,
    pub progreso: Progreso,
}

impl EnTablon {
    pub fn nueva(def: Arc<Recurrente>, desde: Instante, hasta: Instante) -> Self {
        let progreso = if def.es_individual() { Progreso::Individual(BTreeMap::new()) } else { Progreso::Grupal { contadores: BTreeMap::new(), aportes: BTreeMap::new(), situacion: Situacion::Aceptada } };
        EnTablon { def, desde, hasta, progreso }
    }

    pub fn vigente(&self, t: Instante) -> bool {
        self.desde <= t && t < self.hasta
    }

    /// ¿Queda algo cumplido por reclamar?
    pub fn por_reclamar(&self) -> bool {
        match &self.progreso {
            Progreso::Individual(m) => m.values().any(|a| a.situacion == Situacion::Cumplida),
            Progreso::Grupal { situacion, .. } => *situacion == Situacion::Cumplida,
        }
    }

    fn cumple(&self, contadores: &BTreeMap<String, u64>) -> bool {
        self.def.condiciones.iter().all(|(c, u)| contadores.get(c).copied().unwrap_or(0) >= *u)
    }

    /// Suma lo que hizo `heroe` (si cuenta para ella); devuelve si se cumplió con ello.
    pub fn sumar(&mut self, heroe: u64, contador: &str, n: u64) -> bool {
        if n == 0 || !self.def.condiciones.iter().any(|(c, _)| c == contador) {
            return false;
        }
        let usa = |c: &BTreeMap<String, u64>| c.clone();
        match &mut self.progreso {
            Progreso::Individual(m) => {
                let Some(a) = m.get_mut(&heroe).filter(|a| a.situacion == Situacion::Aceptada) else { return false };
                *a.contadores.entry(contador.to_owned()).or_insert(0) += n;
                let contadores = usa(&a.contadores);
                if self.cumple(&contadores) {
                    if let Progreso::Individual(m) = &mut self.progreso {
                        m.get_mut(&heroe).expect("está").situacion = Situacion::Cumplida;
                    }
                    return true;
                }
                false
            }
            Progreso::Grupal { contadores, aportes, situacion } => {
                if *situacion != Situacion::Aceptada {
                    return false;
                }
                *contadores.entry(contador.to_owned()).or_insert(0) += n;
                *aportes.entry(heroe).or_insert(0) += n;
                let c = usa(contadores);
                if self.cumple(&c) {
                    if let Progreso::Grupal { situacion, .. } = &mut self.progreso {
                        *situacion = Situacion::Cumplida;
                    }
                    return true;
                }
                false
            }
        }
    }

    /// Acaba su periodo: lo que no se cumplió caduca.
    pub fn vencer(&mut self) {
        match &mut self.progreso {
            Progreso::Individual(m) => m.values_mut().filter(|a| a.situacion == Situacion::Aceptada).for_each(|a| a.situacion = Situacion::Caducada),
            Progreso::Grupal { situacion, .. } => {
                if *situacion == Situacion::Aceptada {
                    *situacion = Situacion::Caducada;
                }
            }
        }
    }
}

/// El tablón de una plaza o de una Facción.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tablon {
    /// Las ids que salen esta semana.
    pub diarias: Vec<String>,
    pub semanales: Vec<String>,
    /// Lo elegido para la próxima renovación semanal (`None`: las primeras del catálogo).
    pub elegidas_diarias: Option<Vec<String>>,
    pub elegidas_semanales: Option<Vec<String>>,
    /// Aceptaciones de cada misión individual esta semana.
    pub aceptadas_semana: BTreeMap<String, u32>,
    /// Las del periodo en curso y las vencidas con algo por reclamar.
    pub misiones: Vec<EnTablon>,
}

impl Tablon {
    /// La del periodo en curso con esa id.
    pub fn vigente(&self, id: &str, t: Instante) -> Option<usize> {
        self.misiones.iter().position(|m| m.def.id == id && m.vigente(t))
    }
}

/// El estado de las recurrentes del mundo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EstadoRecurrentes {
    pub catalogo: Arc<Catalogo>,
    /// El primer instante del primer día de la partida: las semanas cuentan desde aquí.
    pub origen: Instante,
    /// Cuándo es la renovación que está en la cola (`Carga::Tablon`), si hay una.
    pub renovacion_en: Option<Instante>,
    pub tablones: BTreeMap<Dueno, Tablon>,
}

impl EstadoRecurrentes {
    pub fn nuevo(catalogo: Arc<Catalogo>, ahora: Instante) -> Self {
        EstadoRecurrentes { catalogo, origen: ahora.div_euclid(DIA) * DIA, renovacion_en: None, tablones: BTreeMap::new() }
    }

    /// Las próximas 00:00 después de `t`.
    pub fn siguiente_dia(t: Instante) -> Instante {
        (t.div_euclid(DIA) + 1) * DIA
    }

    /// La próxima renovación semanal después de `t`.
    pub fn siguiente_semana(&self, t: Instante) -> Instante {
        self.origen + ((t - self.origen).div_euclid(SEMANA) + 1) * SEMANA
    }

    /// ¿Es `t` (unas 00:00) una renovación semanal?
    pub fn es_semanal(&self, t: Instante) -> bool {
        (t - self.origen).rem_euclid(SEMANA) == 0
    }

    /// El fin del periodo de una misión que sale en `t`.
    pub fn fin_de(&self, duracion: Duracion, t: Instante) -> Instante {
        match duracion {
            Duracion::Dia => Self::siguiente_dia(t),
            Duracion::Semana => self.siguiente_semana(t),
        }
    }

    /// Un tablón nuevo para `d` en `t`: lo de por defecto, desde ya hasta el fin del día o de la semana en curso.
    pub fn tablon_nuevo(&self, d: Dueno, t: Instante) -> Option<Tablon> {
        let ambito = Ambito::de(d)?;
        let c = &self.catalogo;
        let mut tablon = Tablon { diarias: c.resolver(ambito, Duracion::Dia, None), semanales: c.resolver(ambito, Duracion::Semana, None), ..Default::default() };
        for id in tablon.diarias.iter().chain(&tablon.semanales) {
            let def = c.buscar(id).expect("resuelta").clone();
            let hasta = self.fin_de(def.duracion, t);
            tablon.misiones.push(EnTablon::nueva(def, t, hasta));
        }
        Some(tablon)
    }

    /// La renovación de las 00:00 `t` en un tablón (Misiones_Recurrentes §2): vence lo que acaba (y se va si no queda nada por reclamar); en la semanal cambia lo que sale y vuelve a 0 el
    /// tope; y saca lo que empieza ahora con la definición vigente del catálogo.
    pub fn renovar(&self, ambito: Ambito, tablon: &mut Tablon, t: Instante) {
        let semanal = self.es_semanal(t);
        for m in &mut tablon.misiones {
            if m.hasta <= t {
                m.vencer();
            }
        }
        tablon.misiones.retain(|m| m.hasta > t || m.por_reclamar());
        let c = &self.catalogo;
        if semanal {
            tablon.diarias = c.resolver(ambito, Duracion::Dia, tablon.elegidas_diarias.as_deref());
            tablon.semanales = c.resolver(ambito, Duracion::Semana, tablon.elegidas_semanales.as_deref());
            tablon.aceptadas_semana.clear();
        }
        let salen: Vec<String> = if semanal { tablon.diarias.iter().chain(&tablon.semanales).cloned().collect() } else { tablon.diarias.clone() };
        for id in salen {
            // Una que ya no está en el catálogo (lo cambió la administración) no sale hasta que se elija otra.
            let Some(def) = c.buscar(&id).filter(|d| d.ambito == ambito).cloned() else { continue };
            if tablon.vigente(&id, t).is_some() {
                continue;
            }
            let hasta = self.fin_de(def.duracion, t);
            tablon.misiones.push(EnTablon::nueva(def, t, hasta));
        }
    }
}

// ───────────────────────────── Foto ─────────────────────────────

fn escribir_ids(w: &mut Escritor, l: &[String]) {
    w.u64(l.len() as u64);
    l.iter().for_each(|s| w.texto(s));
}

fn leer_ids(r: &mut Lector) -> Result<Vec<String>, ErrorFormato> {
    (0..r.cuenta()?).map(|_| r.texto()).collect()
}

fn escribir_contadores(w: &mut Escritor, c: &BTreeMap<String, u64>) {
    w.u64(c.len() as u64);
    for (k, n) in c {
        w.texto(k);
        w.u64(*n);
    }
}

fn leer_contadores(r: &mut Lector) -> Result<BTreeMap<String, u64>, ErrorFormato> {
    let mut c = BTreeMap::new();
    for _ in 0..r.cuenta()? {
        c.insert(r.texto()?, r.u64()?);
    }
    Ok(c)
}

fn situacion_a_u8(s: Situacion) -> u8 {
    match s {
        Situacion::Aceptada => 0,
        Situacion::Cumplida => 1,
        Situacion::Reclamada => 2,
        Situacion::Caducada => 3,
    }
}

fn situacion_de_u8(n: u8) -> Result<Situacion, ErrorFormato> {
    Ok(match n {
        0 => Situacion::Aceptada,
        1 => Situacion::Cumplida,
        2 => Situacion::Reclamada,
        3 => Situacion::Caducada,
        _ => return Err(ErrorFormato("situación de misión recurrente desconocida")),
    })
}

pub(crate) fn escribir(w: &mut Escritor, e: &EstadoRecurrentes) {
    w.texto(&e.catalogo.a_json().to_string());
    w.i64(e.origen);
    w.opt_i64(e.renovacion_en);
    w.u64(e.tablones.len() as u64);
    for (d, t) in &e.tablones {
        escribir_dueno(w, *d);
        escribir_ids(w, &t.diarias);
        escribir_ids(w, &t.semanales);
        for elegidas in [&t.elegidas_diarias, &t.elegidas_semanales] {
            w.bool(elegidas.is_some());
            if let Some(l) = elegidas {
                escribir_ids(w, l);
            }
        }
        w.u64(t.aceptadas_semana.len() as u64);
        for (id, n) in &t.aceptadas_semana {
            w.texto(id);
            w.u32(*n);
        }
        w.u64(t.misiones.len() as u64);
        for m in &t.misiones {
            w.texto(&json!({ "ambito": m.def.ambito.clave(), "def": m.def.a_json() }).to_string());
            w.i64(m.desde);
            w.i64(m.hasta);
            match &m.progreso {
                Progreso::Individual(a) => {
                    w.u8(0);
                    w.u64(a.len() as u64);
                    for (h, x) in a {
                        w.u64(*h);
                        w.u8(situacion_a_u8(x.situacion));
                        escribir_contadores(w, &x.contadores);
                    }
                }
                Progreso::Grupal { contadores, aportes, situacion } => {
                    w.u8(1);
                    w.u8(situacion_a_u8(*situacion));
                    escribir_contadores(w, contadores);
                    w.u64(aportes.len() as u64);
                    for (h, n) in aportes {
                        w.u64(*h);
                        w.u64(*n);
                    }
                }
            }
        }
    }
}

pub(crate) fn leer(r: &mut Lector) -> Result<EstadoRecurrentes, ErrorFormato> {
    let catalogo = Arc::new(Catalogo::desde_texto(&r.texto()?, &[]).map_err(|_| ErrorFormato("catálogo de misiones recurrentes mal formado"))?);
    let (origen, renovacion_en) = (r.i64()?, r.opt_i64()?);
    let mut tablones = BTreeMap::new();
    for _ in 0..r.cuenta()? {
        let d = leer_dueno(r)?;
        let mut t = Tablon { diarias: leer_ids(r)?, semanales: leer_ids(r)?, ..Default::default() };
        t.elegidas_diarias = if r.bool()? { Some(leer_ids(r)?) } else { None };
        t.elegidas_semanales = if r.bool()? { Some(leer_ids(r)?) } else { None };
        for _ in 0..r.cuenta()? {
            t.aceptadas_semana.insert(r.texto()?, r.u32()?);
        }
        for _ in 0..r.cuenta()? {
            let v: Value = serde_json::from_str(&r.texto()?).map_err(|_| ErrorFormato("misión recurrente mal formada"))?;
            let ambito = if v.get("ambito").and_then(Value::as_str) == Some("faccion") { Ambito::Faccion } else { Ambito::Asentamiento };
            let def = Arc::new(Recurrente::desde(ambito, &v["def"]).map_err(|_| ErrorFormato("misión recurrente mal formada"))?);
            let (desde, hasta) = (r.i64()?, r.i64()?);
            let progreso = match r.u8()? {
                0 => {
                    let mut a = BTreeMap::new();
                    for _ in 0..r.cuenta()? {
                        let h = r.u64()?;
                        let situacion = situacion_de_u8(r.u8()?)?;
                        a.insert(h, Aceptacion { situacion, contadores: leer_contadores(r)? });
                    }
                    Progreso::Individual(a)
                }
                1 => {
                    let situacion = situacion_de_u8(r.u8()?)?;
                    let contadores = leer_contadores(r)?;
                    let mut aportes = BTreeMap::new();
                    for _ in 0..r.cuenta()? {
                        aportes.insert(r.u64()?, r.u64()?);
                    }
                    Progreso::Grupal { contadores, aportes, situacion }
                }
                _ => return Err(ErrorFormato("progreso de misión recurrente desconocido")),
            };
            t.misiones.push(EnTablon { def, desde, hasta, progreso });
        }
        tablones.insert(d, t);
    }
    Ok(EstadoRecurrentes { catalogo, origen, renovacion_en, tablones })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn el_catalogo_inicial_se_lee_y_vuelve_igual_por_su_json() {
        let c = Catalogo::cargar();
        assert_eq!(c.de(Ambito::Asentamiento, Duracion::Dia).count(), 10);
        assert_eq!(c.de(Ambito::Asentamiento, Duracion::Semana).count(), 10);
        assert_eq!(c.misiones.iter().filter(|m| m.ambito == Ambito::Faccion).count(), 8);
        assert_eq!(Catalogo::desde(&c.a_json(), &[]).unwrap(), c);
        // No choca con las únicas.
        let unicas = crate::misiones::Reglas::cargar();
        let ids: Vec<&str> = unicas.misiones.iter().map(|m| m.id.as_str()).collect();
        Catalogo::desde(&c.a_json(), &ids).unwrap();
    }

    #[test]
    fn un_catalogo_mal_hecho_no_entra_y_dice_por_que() {
        let mal = |s: &str| Catalogo::desde_texto(s, &["A1"]).unwrap_err();
        let base = |extra: &str| format!(r#"{{"asentamiento":[{{"id":"X","duracion":"dia","tipo":"individual","limiteSemanal":3,"condiciones":{{"batallas":1}},"titulo":"t","texto":"t","explicacion":"e"{extra}}}]}}"#);
        assert!(Catalogo::desde_texto(&base(""), &[]).is_ok());
        assert!(mal(&base(r#","cargo":"rey""#)).contains("cargo"));
        assert!(mal(&base(r#","condiciones":{"volar":1}"#)).contains("volar"));
        assert!(mal(&base(r#","limiteSemanal":0"#)).contains("limiteSemanal"));
        assert!(mal(&base(r#","id":"A1""#)).contains("única"));
        assert!(mal("{").contains("JSON"));
    }

    #[test]
    fn las_renovaciones_caen_a_las_00_00_y_la_semanal_cada_7_dias_desde_el_primer_dia() {
        let e = EstadoRecurrentes::nuevo(Arc::new(Catalogo::cargar()), 3 * DIA + 5_000);
        assert_eq!(e.origen, 3 * DIA);
        assert_eq!(EstadoRecurrentes::siguiente_dia(3 * DIA + 5_000), 4 * DIA);
        assert_eq!(EstadoRecurrentes::siguiente_dia(4 * DIA), 5 * DIA);
        assert_eq!(e.siguiente_semana(3 * DIA + 5_000), 10 * DIA);
        assert!(e.es_semanal(10 * DIA) && !e.es_semanal(9 * DIA));
    }

    #[test]
    fn la_foto_de_ida_y_vuelta() {
        let mut e = EstadoRecurrentes::nuevo(Arc::new(Catalogo::cargar()), 0);
        let mut t = e.tablon_nuevo(Dueno::Asentamiento(3), 1_000).unwrap();
        t.elegidas_semanales = Some(vec!["PS2".into()]);
        t.aceptadas_semana.insert("PD1".into(), 2);
        if let Progreso::Individual(a) = &mut t.misiones[0].progreso {
            a.insert(7, Aceptacion { contadores: [("batallas".to_owned(), 1)].into(), situacion: Situacion::Cumplida });
        }
        t.misiones[4].sumar(9, "batallas", 2);
        e.tablones.insert(Dueno::Asentamiento(3), t);
        e.tablones.insert(Dueno::Faccion(2), e.tablon_nuevo(Dueno::Faccion(2), 1_000).unwrap());
        e.renovacion_en = Some(DIA);
        let mut w = Escritor(Vec::new());
        escribir(&mut w, &e);
        let bytes = w.0;
        let mut r = Lector::nuevo(&bytes);
        assert_eq!(leer(&mut r).unwrap(), e);
        r.fin().unwrap();
    }
}
