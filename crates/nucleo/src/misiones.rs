//! Las misiones (mecánica 49, `Docs/Mechanics/Misiones.md`): el catálogo del bloque `MISIONES` y el estado de cada dueño (héroe, plaza o Facción). Aquí está lo puro: qué misiones están
//! activas, sumar a un contador, mirar un contador de estado y cumplir; de dónde sale cada avance, reclamar y la proyección son de `nucleo::misiones`.
//!
//! Formato en la foto (desde BAN46, little endian de `codec`): `n u64 · n × (dueño · contadores{n u64 · n × (texto, u64)} · cumplidas{n u64 · n × u16} · reclamadas{n u64 · n × u16})`; el
//! dueño es una etiqueta u8 (0 héroe, 1 plaza, 2 Facción) y su id u64. Las misiones van por índice del catálogo (el orden del bloque, que fija la huella de las reglas).

use crate::codec::{ErrorFormato, Escritor, Lector};
use asentamiento::Recurso;
use base::{Mil, UNIDAD};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const DOC_MISIONES: &str = include_str!("../../../Docs/Mechanics/Misiones.md");

/// Los contadores que no se suman: se leen del estado (Misiones §3).
pub const DE_ESTADO: &[&str] = &["mercado", "barracon", "nivel", "artesanos", "nobles", "aldeas", "capital"];

/// De quién es una misión.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dueno {
    Heroe(u64),
    Asentamiento(u64),
    Faccion(u32),
}

impl Dueno {
    pub fn clase(self) -> ClaseDueno {
        match self {
            Dueno::Heroe(_) => ClaseDueno::Heroe,
            Dueno::Asentamiento(_) => ClaseDueno::Asentamiento,
            Dueno::Faccion(_) => ClaseDueno::Faccion,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClaseDueno {
    Heroe,
    Asentamiento,
    Faccion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recompensa {
    /// Oro al almacén personal del héroe, en milésimas.
    Oro(Mil),
    /// Recursos al almacén de la plaza, en milésimas.
    Recursos(Vec<(Recurso, Mil)>),
    /// Experiencia de Facción, en milésimas.
    Experiencia(Mil),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mision {
    pub id: String,
    pub clase: ClaseDueno,
    /// La misión que hay que cumplir antes (índice del catálogo).
    pub previa: Option<u16>,
    /// `false`: está en el catálogo pero no se activa nunca.
    pub habilitada: bool,
    /// Contador y umbral; se cumplen todos a la vez.
    pub condiciones: Vec<(String, u64)>,
    pub recompensa: Recompensa,
}

/// El catálogo, en el orden del bloque: héroe, plaza y Facción.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reglas {
    pub misiones: Vec<Mision>,
}

impl Reglas {
    pub fn cargar() -> Self {
        let v = asentamiento::datos::bloque_de(DOC_MISIONES, "MISIONES").unwrap_or_else(|e| panic!("{e}"));
        Self::desde(&v).unwrap_or_else(|e| panic!("bloque MISIONES: {e}"))
    }

    pub fn desde(v: &Value) -> Result<Self, String> {
        let mut misiones = Vec::new();
        let mut previas = Vec::new();
        for (clave, clase) in [("heroe", ClaseDueno::Heroe), ("asentamiento", ClaseDueno::Asentamiento), ("faccion", ClaseDueno::Faccion)] {
            for m in v.get(clave).and_then(Value::as_array).ok_or_else(|| format!("falta la lista «{clave}»"))? {
                let id = m.get("id").and_then(Value::as_str).ok_or("misión sin id")?.to_owned();
                let condiciones: Vec<(String, u64)> = m
                    .get("condiciones")
                    .and_then(Value::as_object)
                    .ok_or_else(|| format!("{id}: sin condiciones"))?
                    .iter()
                    .map(|(k, n)| n.as_u64().filter(|&n| n > 0).map(|n| (k.clone(), n)).ok_or_else(|| format!("{id}: umbral de «{k}»")))
                    .collect::<Result<_, _>>()?;
                let mil = |x: &Value| x.as_f64().map(|f| (f * UNIDAD as f64).round() as Mil).filter(|&n| n > 0);
                let recompensa = match clase {
                    ClaseDueno::Heroe => Recompensa::Oro(m.get("oro").and_then(mil).ok_or_else(|| format!("{id}: sin oro"))?),
                    ClaseDueno::Asentamiento => Recompensa::Recursos(
                        m.get("recursos")
                            .and_then(Value::as_object)
                            .ok_or_else(|| format!("{id}: sin recursos"))?
                            .iter()
                            .map(|(r, n)| Ok((Recurso::desde_nombre(r).ok_or_else(|| format!("{id}: recurso «{r}»"))?, mil(n).ok_or_else(|| format!("{id}: cantidad de «{r}»"))?)))
                            .collect::<Result<_, String>>()?,
                    ),
                    ClaseDueno::Faccion => Recompensa::Experiencia(m.get("xp").and_then(mil).ok_or_else(|| format!("{id}: sin xp"))?),
                };
                previas.push(m.get("previa").and_then(Value::as_str).map(str::to_owned));
                let habilitada = m.get("habilitada").and_then(Value::as_bool).unwrap_or(true);
                misiones.push(Mision { id, clase, previa: None, habilitada, condiciones, recompensa });
            }
        }
        for (i, previa) in previas.into_iter().enumerate() {
            let Some(p) = previa else { continue };
            let j = misiones.iter().position(|m| m.id == p).ok_or_else(|| format!("{}: previa «{p}» desconocida", misiones[i].id))?;
            if misiones[j].clase != misiones[i].clase || j >= i {
                return Err(format!("{}: la previa «{p}» debe ser del mismo dueño y estar antes", misiones[i].id));
            }
            misiones[i].previa = Some(j as u16);
        }
        let ids: BTreeSet<&str> = misiones.iter().map(|m| m.id.as_str()).collect();
        if ids.len() != misiones.len() {
            return Err("ids repetidos".into());
        }
        Ok(Reglas { misiones })
    }

    /// El índice de la misión `id` de esa clase de dueño.
    pub fn buscar(&self, clase: ClaseDueno, id: &str) -> Option<u16> {
        self.misiones.iter().position(|m| m.clase == clase && m.id == id).map(|i| i as u16)
    }

    /// Las misiones de esa clase de dueño, con su índice.
    pub fn de(&self, clase: ClaseDueno) -> impl Iterator<Item = (u16, &Mision)> {
        self.misiones.iter().enumerate().filter(move |(_, m)| m.clase == clase).map(|(i, m)| (i as u16, m))
    }
}

impl Default for Reglas {
    fn default() -> Self {
        static R: std::sync::OnceLock<Reglas> = std::sync::OnceLock::new();
        R.get_or_init(Reglas::cargar).clone()
    }
}

/// Lo de un dueño. Sin entrada, todo a 0 y nada cumplido.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeDueno {
    pub contadores: BTreeMap<String, u64>,
    pub cumplidas: BTreeSet<u16>,
    pub reclamadas: BTreeSet<u16>,
}

impl DeDueno {
    /// ¿Está activa la misión `i`? Habilitada, sin cumplir y con su previa cumplida.
    pub fn activa(&self, r: &Reglas, i: u16) -> bool {
        let m = &r.misiones[i as usize];
        m.habilitada && !self.cumplidas.contains(&i) && m.previa.is_none_or(|p| self.cumplidas.contains(&p))
    }

    /// El avance de un contador (0 si no se cuenta).
    pub fn contador(&self, nombre: &str) -> u64 {
        self.contadores.get(nombre).copied().unwrap_or(0)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EstadoMisiones {
    pub duenos: BTreeMap<Dueno, DeDueno>,
}

impl EstadoMisiones {
    pub fn de(&self, d: Dueno) -> Option<&DeDueno> {
        self.duenos.get(&d)
    }

    /// ¿Usa alguna misión activa de `d` el contador?
    pub fn cuenta(&self, r: &Reglas, d: Dueno, contador: &str) -> bool {
        let vacio = DeDueno::default();
        let x = self.duenos.get(&d).unwrap_or(&vacio);
        r.de(d.clase()).any(|(i, m)| x.activa(r, i) && m.condiciones.iter().any(|(c, _)| c == contador))
    }

    /// Suma `n` al contador de `d` si alguna misión activa lo usa; devuelve las misiones que se cumplen (en orden, con las que se activan y ya cumplían).
    pub fn sumar(&mut self, r: &Reglas, d: Dueno, contador: &str, n: u64) -> Vec<u16> {
        if n == 0 || !self.cuenta(r, d, contador) {
            return Vec::new();
        }
        *self.duenos.entry(d).or_default().contadores.entry(contador.to_owned()).or_insert(0) += n;
        self.cumplir(r, d)
    }

    /// Un contador de estado: se queda con el máximo visto, si alguna misión activa lo usa.
    pub fn record(&mut self, r: &Reglas, d: Dueno, contador: &str, valor: u64) -> Vec<u16> {
        if !self.cuenta(r, d, contador) || self.duenos.get(&d).is_some_and(|x| x.contador(contador) >= valor) {
            return Vec::new();
        }
        self.duenos.entry(d).or_default().contadores.insert(contador.to_owned(), valor);
        self.cumplir(r, d)
    }

    /// Fija las misiones activas de `d` que ya cumplen todas sus condiciones (y las que se activan con ellas y también cumplen), y borra los contadores que ya no usa ninguna misión sin
    /// cumplir.
    pub fn cumplir(&mut self, r: &Reglas, d: Dueno) -> Vec<u16> {
        let Some(x) = self.duenos.get_mut(&d) else { return Vec::new() };
        let mut nuevas = Vec::new();
        loop {
            let ya: Vec<u16> = r.de(d.clase()).filter(|(i, m)| x.activa(r, *i) && m.condiciones.iter().all(|(c, u)| x.contador(c) >= *u)).map(|(i, _)| i).collect();
            if ya.is_empty() {
                break;
            }
            x.cumplidas.extend(&ya);
            nuevas.extend(ya);
        }
        if !nuevas.is_empty() {
            let usados: BTreeSet<&str> = r.de(d.clase()).filter(|(i, m)| m.habilitada && !x.cumplidas.contains(i)).flat_map(|(_, m)| m.condiciones.iter().map(|(c, _)| c.as_str())).collect();
            x.contadores.retain(|c, _| usados.contains(c.as_str()));
        }
        nuevas
    }

    /// Los contadores de estado que usa alguna misión activa de `d`.
    pub fn de_estado_activos(&self, r: &Reglas, d: Dueno) -> Vec<&'static str> {
        DE_ESTADO.iter().copied().filter(|c| self.cuenta(r, d, c)).collect()
    }
}

pub(crate) fn escribir_dueno(w: &mut Escritor, d: Dueno) {
    match d {
        Dueno::Heroe(h) => {
            w.u8(0);
            w.u64(h);
        }
        Dueno::Asentamiento(a) => {
            w.u8(1);
            w.u64(a);
        }
        Dueno::Faccion(f) => {
            w.u8(2);
            w.u64(u64::from(f));
        }
    }
}

pub(crate) fn leer_dueno(r: &mut Lector) -> Result<Dueno, ErrorFormato> {
    Ok(match r.u8()? {
        0 => Dueno::Heroe(r.u64()?),
        1 => Dueno::Asentamiento(r.u64()?),
        2 => Dueno::Faccion(u32::try_from(r.u64()?).map_err(|_| ErrorFormato("Facción de misión fuera de rango"))?),
        _ => return Err(ErrorFormato("dueño de misión desconocido")),
    })
}

pub(crate) fn escribir(w: &mut Escritor, e: &EstadoMisiones) {
    let EstadoMisiones { duenos } = e;
    w.u64(duenos.len() as u64);
    for (d, x) in duenos {
        let DeDueno { contadores, cumplidas, reclamadas } = x;
        escribir_dueno(w, *d);
        w.u64(contadores.len() as u64);
        for (c, n) in contadores {
            w.texto(c);
            w.u64(*n);
        }
        for conjunto in [cumplidas, reclamadas] {
            w.u64(conjunto.len() as u64);
            conjunto.iter().for_each(|&i| w.u16(i));
        }
    }
}

pub(crate) fn leer(r: &mut Lector, reglas: &Reglas) -> Result<EstadoMisiones, ErrorFormato> {
    let mut duenos = BTreeMap::new();
    for _ in 0..r.cuenta()? {
        let d = leer_dueno(r)?;
        let mut x = DeDueno::default();
        for _ in 0..r.cuenta()? {
            x.contadores.insert(r.texto()?, r.u64()?);
        }
        for conjunto in [&mut x.cumplidas, &mut x.reclamadas] {
            for _ in 0..r.cuenta()? {
                let i = r.u16()?;
                if i as usize >= reglas.misiones.len() {
                    return Err(ErrorFormato("misión fuera del catálogo"));
                }
                conjunto.insert(i);
            }
        }
        duenos.insert(d, x);
    }
    Ok(EstadoMisiones { duenos })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn el_catalogo_del_documento_se_lee_entero_con_sus_cadenas() {
        let r = Reglas::cargar();
        assert_eq!(r.de(ClaseDueno::Heroe).count(), 13);
        assert_eq!(r.de(ClaseDueno::Asentamiento).count(), 11);
        assert_eq!(r.de(ClaseDueno::Faccion).count(), 12);
        let f4 = r.buscar(ClaseDueno::Faccion, "F4").unwrap();
        assert_eq!(r.misiones[f4 as usize].previa, r.buscar(ClaseDueno::Faccion, "F3"));
        assert!(r.misiones[r.buscar(ClaseDueno::Asentamiento, "A1").unwrap() as usize].habilitada, "la A1 se activa con la aldea agrícola");
        assert_eq!(r.misiones[r.buscar(ClaseDueno::Heroe, "H1").unwrap() as usize].recompensa, Recompensa::Oro(10 * UNIDAD));
    }

    #[test]
    fn una_cadena_comparte_su_contador_y_lo_que_no_esta_activo_no_cuenta() {
        let r = Reglas::cargar();
        let mut e = EstadoMisiones::default();
        let f = Dueno::Faccion(1);
        let id = |s| r.buscar(ClaseDueno::Faccion, s).unwrap();
        // F3 pide F1: sin capital, fundar no cuenta.
        assert!(e.sumar(&r, f, "fundaciones", 1).is_empty());
        assert_eq!(e.de(f), None);
        assert_eq!(e.record(&r, f, "capital", 1), vec![id("F1")]);
        assert!(e.sumar(&r, f, "fundaciones", 1).is_empty());
        assert_eq!(e.sumar(&r, f, "fundaciones", 1), vec![id("F3")]);
        assert_eq!(e.de(f).unwrap().contador("fundaciones"), 2, "F4 sigue contando desde donde lo dejó F3");
        assert_eq!(e.sumar(&r, f, "fundaciones", 3), vec![id("F4")]);
        assert_eq!(e.de(f).unwrap().contador("fundaciones"), 0, "sin misión que lo use, el contador se borra");
        let mut w = Escritor::default();
        escribir(&mut w, &e);
        let bytes = w.0;
        assert_eq!(leer(&mut Lector::nuevo(&bytes), &r).unwrap(), e);
    }

    /// El catálogo de textos (`Misiones_Textos.md`, fuera de la huella) está entero: cada misión con su título, su texto y su explicación, cada contador con su nombre, y nada que no exista.
    #[test]
    fn cada_mision_y_cada_contador_tiene_su_texto() {
        let doc = include_str!("../../../Docs/Mechanics/Misiones_Textos.md");
        let v = asentamiento::datos::bloque_de(doc, "TEXTOS_MISIONES").unwrap_or_else(|e| panic!("{e}"));
        let r = Reglas::cargar();
        let misiones = v["misiones"].as_object().expect("misiones");
        let contadores = v["contadores"].as_object().expect("contadores");
        for m in &r.misiones {
            let t = &misiones.get(&m.id).unwrap_or_else(|| panic!("{} sin texto", m.id));
            for campo in ["titulo", "texto", "explicacion"] {
                assert!(t[campo].as_str().is_some_and(|x| !x.is_empty()), "{}: {campo}", m.id);
            }
            for (c, _) in &m.condiciones {
                assert!(contadores.get(c).and_then(Value::as_str).is_some_and(|x| !x.is_empty()), "contador {c} sin nombre");
            }
        }
        assert_eq!(misiones.len(), r.misiones.len(), "textos de misiones que no existen");
        let usados: BTreeSet<&str> = r.misiones.iter().flat_map(|m| m.condiciones.iter().map(|(c, _)| c.as_str())).collect();
        assert!(contadores.keys().all(|c| usados.contains(c.as_str())), "nombres de contadores que nadie usa");
    }
}
