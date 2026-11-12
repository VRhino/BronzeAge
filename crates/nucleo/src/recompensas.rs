//! Las recompensas de las misiones, únicas y recurrentes (mecánica 66, `Docs/Mechanics/Misiones.md` §4 y `Misiones_Recurrentes.md` §5): una **fija**, que marca el tipo de
//! misión, y una **lista libre**. Aquí está lo puro: leerlas del JSON de un catálogo, validarlas y escribirlas de vuelta. Darlas al reclamar es de `nucleo::recompensas`.

use crate::misiones::ClaseDueno;
use asentamiento::Recurso;
use base::{Mil, UNIDAD};
use militar::catalogo::Tropa;
use objetos::datos::Tipo;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

/// La recompensa fija de una misión, según su tipo: la de plaza da participación y la de Facción experiencia; la de héroe no tiene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fija {
    /// Participación en la plaza (Participacion §2), en puntos enteros.
    Participacion(u64),
    /// Experiencia de Facción, en milésimas.
    Experiencia(Mil),
}

/// Un elemento de la lista libre. Las cantidades van en milésimas, salvo los soldados de una tropa y las piezas de un objeto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Elemento {
    Recurso(Recurso, Mil),
    Oro(Mil),
    Experiencia(Mil),
    Tropa(Tropa, i64),
    /// Un objeto del catálogo (`OBJETOS`) y cuántas piezas (entrada 67): va a la mochila del héroe que cobra (D17 y D22).
    Objeto(String, u32),
}

impl Elemento {
    /// El `tipo` del JSON y, en un recurso, una tropa o un objeto, su id.
    pub fn clave(&self) -> (&'static str, &str) {
        match self {
            Elemento::Recurso(r, _) => ("recurso", r.nombre()),
            Elemento::Oro(_) => ("oro", ""),
            Elemento::Experiencia(_) => ("xp", ""),
            Elemento::Tropa(t, _) => ("tropa", t.nombre()),
            Elemento::Objeto(o, _) => ("objeto", o),
        }
    }

    fn a_json(&self) -> Value {
        match self {
            Elemento::Recurso(r, n) => json!({ "tipo": "recurso", "recurso": r.nombre(), "cantidad": mil_a_json(*n) }),
            Elemento::Oro(n) => json!({ "tipo": "oro", "cantidad": mil_a_json(*n) }),
            Elemento::Experiencia(n) => json!({ "tipo": "xp", "cantidad": mil_a_json(*n) }),
            Elemento::Tropa(t, n) => json!({ "tipo": "tropa", "tropa": t.nombre(), "cantidad": n }),
            Elemento::Objeto(o, n) => json!({ "tipo": "objeto", "objeto": o, "cantidad": n }),
        }
    }
}

/// Lo que da una misión al reclamarla.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recompensa {
    /// `None` solo en una misión de héroe.
    pub fija: Option<Fija>,
    pub lista: Vec<Elemento>,
}

impl Recompensa {
    /// Lee la fija de su clase (`participacion` en una de plaza, `xp` en una de Facción) y la lista `recompensas` de la misión `m`, y lo valida todo: la fija es mayor que 0,
    /// una de héroe lleva alguna recompensa, cada elemento tiene un `tipo` conocido y una cantidad mayor que 0 (entera en los recursos y el oro), la lista no repite nada ni la fija, solo una misión
    /// `personal` (individual o cadena de héroe) da tropa u objetos, una tropa no pide más soldados de los que lleva su escuadra y un objeto existe y no es material (ni crisol ni planos,
    /// Objetos_Del_Heroe §2.1).
    pub fn leer(m: &Value, clase: ClaseDueno, personal: bool, militar: &militar::Reglas) -> Result<Recompensa, String> {
        let id = m.get("id").and_then(Value::as_str).unwrap_or("?");
        let (clave_fija, ajenas): (Option<&str>, &[&str]) = match clase {
            ClaseDueno::Heroe => (None, &["participacion", "xp"]),
            ClaseDueno::Asentamiento => (Some("participacion"), &["xp"]),
            ClaseDueno::Faccion => (Some("xp"), &["participacion"]),
        };
        // Las claves de antes (y la fija de otro tipo) no se ignoran en silencio: se perdería la recompensa.
        if let Some(k) = ajenas.iter().chain(&["oro", "recursos"]).find(|k| m.get(**k).is_some()) {
            return Err(format!("{id}: «{k}» no es la recompensa fija de su tipo: va en «recompensas»"));
        }
        let fija = match clave_fija {
            None => None,
            Some(k) => {
                let v = m.get(k).ok_or_else(|| format!("{id}: falta su recompensa fija «{k}»"))?;
                Some(match clase {
                    ClaseDueno::Asentamiento => Fija::Participacion(v.as_u64().filter(|&n| n > 0).ok_or_else(|| format!("{id}: «{k}» es un entero mayor que 0"))?),
                    _ => Fija::Experiencia(milesimas(v).ok_or_else(|| format!("{id}: «{k}» es mayor que 0"))?),
                })
            }
        };
        let lista = match m.get("recompensas") {
            None => Vec::new(),
            Some(l) => l.as_array().ok_or_else(|| format!("{id}: «recompensas» es una lista"))?.iter().map(|e| elemento(id, e, militar)).collect::<Result<Vec<_>, _>>()?,
        };
        if clase == ClaseDueno::Heroe && lista.is_empty() {
            return Err(format!("{id}: una misión de héroe no tiene recompensa fija: necesita alguna en «recompensas»"));
        }
        let mut vistas = BTreeSet::new();
        for e in &lista {
            match e {
                Elemento::Experiencia(_) if clase == ClaseDueno::Faccion => return Err(format!("{id}: la xp ya es la recompensa fija de una misión de Facción")),
                Elemento::Tropa(..) if !personal => return Err(format!("{id}: solo una misión personal (individual o de héroe) da tropa")),
                Elemento::Objeto(..) if !personal => return Err(format!("{id}: solo una misión personal (individual o de héroe) da objetos")),
                _ => {}
            }
            if !vistas.insert(e.clave()) {
                return Err(format!("{id}: «recompensas» repite {}", describir(e)));
            }
        }
        Ok(Recompensa { fija, lista })
    }

    /// Escribe las mismas claves en el JSON de la misión (la inversa de [`Recompensa::leer`]).
    pub fn escribir(&self, m: &mut Map<String, Value>) {
        match self.fija {
            Some(Fija::Participacion(n)) => {
                m.insert("participacion".into(), json!(n));
            }
            Some(Fija::Experiencia(n)) => {
                m.insert("xp".into(), mil_a_json(n));
            }
            None => {}
        }
        if !self.lista.is_empty() {
            m.insert("recompensas".into(), Value::Array(self.lista.iter().map(|e| e.a_json()).collect()));
        }
    }
}

/// Un elemento de la lista, validado.
fn elemento(id: &str, e: &Value, militar: &militar::Reglas) -> Result<Elemento, String> {
    let texto = |k: &str| e.get(k).and_then(Value::as_str).ok_or_else(|| format!("{id}: un elemento de «recompensas» sin «{k}»"));
    let cantidad = e.get("cantidad").ok_or_else(|| format!("{id}: un elemento de «recompensas» sin «cantidad»"))?;
    let mil = || milesimas(cantidad).ok_or_else(|| format!("{id}: la «cantidad» de cada recompensa es mayor que 0"));
    // Los almacenes solo guardan unidades enteras (Economia_Construccion, C3b): una fracción de recurso o de oro se perdería al darla.
    let entera = || mil().and_then(|n| if n % UNIDAD == 0 { Ok(n) } else { Err(format!("{id}: los recursos y el oro van en unidades enteras")) });
    Ok(match texto("tipo")? {
        "recurso" => {
            let nombre = texto("recurso")?;
            let r = Recurso::desde_nombre(nombre).ok_or_else(|| format!("{id}: el recurso «{nombre}» no existe"))?;
            if r == Recurso::Oro {
                return Err(format!("{id}: el oro va con «tipo»: «oro», no como recurso"));
            }
            Elemento::Recurso(r, entera()?)
        }
        "oro" => Elemento::Oro(entera()?),
        "xp" => Elemento::Experiencia(mil()?),
        "tropa" => {
            let nombre = texto("tropa")?;
            let t = Tropa::desde_nombre(nombre).ok_or_else(|| format!("{id}: la tropa «{nombre}» no existe"))?;
            let tope = militar.tropas.tropa(t).unidades;
            let n = cantidad.as_i64().filter(|&n| n > 0).ok_or_else(|| format!("{id}: los soldados de «{nombre}» son un entero mayor que 0"))?;
            if n > tope {
                return Err(format!("{id}: una escuadra de «{nombre}» lleva como mucho {tope} soldados"));
            }
            Elemento::Tropa(t, n)
        }
        "objeto" => {
            let nombre = texto("objeto")?;
            let ficha = militar.objetos.objetos.get(nombre).ok_or_else(|| format!("{id}: el objeto «{nombre}» no existe"))?;
            // Los crisoles y los planos son escasos a propósito y solo salen del botín: una misión no los da.
            if ficha.tipo == Tipo::Material {
                return Err(format!("{id}: «{nombre}» es un material de la forja: una misión no da ni crisoles ni planos"));
            }
            let n = cantidad.as_u64().filter(|&n| n > 0).and_then(|n| u32::try_from(n).ok()).ok_or_else(|| format!("{id}: las piezas de «{nombre}» son un entero mayor que 0"))?;
            Elemento::Objeto(nombre.to_owned(), n)
        }
        otro => return Err(format!("{id}: el tipo de recompensa «{otro}» no existe (recurso, oro, xp, tropa u objeto)")),
    })
}

fn describir(e: &Elemento) -> String {
    match e.clave() {
        (tipo, "") => format!("«{tipo}»"),
        (tipo, nombre) => format!("{tipo} «{nombre}»"),
    }
}

/// Una cantidad del JSON en milésimas, si es mayor que 0.
fn milesimas(v: &Value) -> Option<Mil> {
    v.as_f64().map(|f| (f * UNIDAD as f64).round() as Mil).filter(|&n| n > 0)
}

/// Unas milésimas en el JSON: entero si es exacto.
fn mil_a_json(x: Mil) -> Value {
    if x % UNIDAD == 0 {
        json!(x / UNIDAD)
    } else {
        json!(x as f64 / UNIDAD as f64)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn reglas() -> militar::Reglas {
        militar::Reglas::cargar()
    }

    fn leer(m: Value, clase: ClaseDueno, personal: bool) -> Result<Recompensa, String> {
        Recompensa::leer(&m, clase, personal, &reglas())
    }

    #[test]
    fn la_fija_y_la_lista_se_leen_y_vuelven_igual_por_su_json() {
        let tropa = Tropa::Arqueros;
        let m = json!({
            "id": "PS1", "participacion": 10,
            "recompensas": [
                { "tipo": "xp", "cantidad": 2.5 },
                { "tipo": "recurso", "recurso": "madera", "cantidad": 300 },
                { "tipo": "oro", "cantidad": 5 },
                { "tipo": "tropa", "tropa": tropa.nombre(), "cantidad": 1 },
                { "tipo": "objeto", "objeto": "medalla_heroe_grande", "cantidad": 2 }
            ]
        });
        let r = leer(m.clone(), ClaseDueno::Asentamiento, true).unwrap();
        assert_eq!(r.fija, Some(Fija::Participacion(10)));
        assert_eq!(r.lista, vec![Elemento::Experiencia(2_500), Elemento::Recurso(Recurso::Madera, 300 * UNIDAD), Elemento::Oro(5 * UNIDAD), Elemento::Tropa(tropa, 1), Elemento::Objeto("medalla_heroe_grande".into(), 2)]);
        let mut vuelta = Map::new();
        vuelta.insert("id".into(), json!("PS1"));
        r.escribir(&mut vuelta);
        assert_eq!(Value::Object(vuelta), m);
        let f = leer(json!({ "id": "F1", "xp": 10 }), ClaseDueno::Faccion, false).unwrap();
        assert_eq!(f, Recompensa { fija: Some(Fija::Experiencia(10 * UNIDAD)), lista: vec![] });
    }

    #[test]
    fn una_recompensa_mal_hecha_no_entra_y_dice_por_que() {
        let tropa = Tropa::Arqueros;
        let tope = reglas().tropas.tropa(tropa).unidades;
        let mal = |m: Value, clase: ClaseDueno, personal: bool| leer(m, clase, personal).unwrap_err();
        let plaza = |extra: Value| {
            let mut m = json!({ "id": "X", "participacion": 10 });
            m.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            m
        };
        let con = |l: Value| plaza(json!({ "recompensas": l }));
        assert!(mal(json!({ "id": "X" }), ClaseDueno::Asentamiento, false).contains("fija «participacion»"));
        assert!(mal(json!({ "id": "X", "participacion": 0 }), ClaseDueno::Asentamiento, false).contains("mayor que 0"));
        assert!(mal(json!({ "id": "X", "xp": 0 }), ClaseDueno::Faccion, false).contains("mayor que 0"));
        assert!(mal(json!({ "id": "X" }), ClaseDueno::Heroe, true).contains("necesita alguna"));
        assert!(mal(plaza(json!({ "xp": 2 })), ClaseDueno::Asentamiento, false).contains("«xp» no es la recompensa fija"));
        assert!(mal(plaza(json!({ "oro": 2 })), ClaseDueno::Asentamiento, true).contains("«oro» no es la recompensa fija"));
        assert!(mal(json!({ "id": "X", "xp": 5, "recompensas": [{ "tipo": "xp", "cantidad": 1 }] }), ClaseDueno::Faccion, false).contains("ya es la recompensa fija"));
        assert!(mal(con(json!([{ "tipo": "joya", "cantidad": 1 }])), ClaseDueno::Asentamiento, false).contains("«joya» no existe"));
        assert!(mal(con(json!([{ "tipo": "oro", "cantidad": 0 }])), ClaseDueno::Asentamiento, false).contains("mayor que 0"));
        assert!(mal(con(json!([{ "tipo": "oro", "cantidad": 1.5 }])), ClaseDueno::Asentamiento, false).contains("unidades enteras"));
        assert!(mal(con(json!([{ "tipo": "recurso", "recurso": "madera", "cantidad": 0.5 }])), ClaseDueno::Asentamiento, false).contains("unidades enteras"));
        assert!(mal(con(json!([{ "tipo": "recurso", "recurso": "nada", "cantidad": 1 }])), ClaseDueno::Asentamiento, false).contains("«nada» no existe"));
        assert!(mal(con(json!([{ "tipo": "recurso", "recurso": "oro", "cantidad": 1 }])), ClaseDueno::Asentamiento, false).contains("«tipo»: «oro»"));
        assert!(mal(con(json!([{ "tipo": "oro", "cantidad": 1 }, { "tipo": "oro", "cantidad": 2 }])), ClaseDueno::Asentamiento, false).contains("repite «oro»"));
        let madera = json!({ "tipo": "recurso", "recurso": "madera", "cantidad": 1 });
        assert!(mal(con(json!([madera, madera])), ClaseDueno::Asentamiento, false).contains("repite recurso «madera»"));
        let soldados = |n: i64| json!([{ "tipo": "tropa", "tropa": tropa.nombre(), "cantidad": n }]);
        assert!(mal(con(soldados(1)), ClaseDueno::Asentamiento, false).contains("solo una misión personal"));
        assert!(mal(con(soldados(tope + 1)), ClaseDueno::Asentamiento, true).contains(&format!("como mucho {tope}")));
        assert!(mal(con(json!([{ "tipo": "tropa", "tropa": "dragones", "cantidad": 1 }])), ClaseDueno::Asentamiento, true).contains("«dragones» no existe"));
        assert!(leer(con(soldados(tope)), ClaseDueno::Asentamiento, true).is_ok());
        // Un objeto (entrada 67): solo en una personal, que exista, que no sea material de la forja y en piezas enteras; no se repite.
        let objeto = |o: &str, n: Value| json!([{ "tipo": "objeto", "objeto": o, "cantidad": n }]);
        assert!(mal(con(objeto("dagas_cobre", json!(1))), ClaseDueno::Asentamiento, false).contains("solo una misión personal"));
        assert!(mal(json!({ "id": "X", "xp": 5, "recompensas": objeto("dagas_cobre", json!(1)) }), ClaseDueno::Faccion, false).contains("da objetos"));
        assert!(mal(con(objeto("crisol", json!(1))), ClaseDueno::Asentamiento, true).contains("ni crisoles ni planos"));
        assert!(mal(con(objeto("plano_3", json!(1))), ClaseDueno::Asentamiento, true).contains("ni crisoles ni planos"));
        assert!(mal(con(objeto("espada_de_madera", json!(1))), ClaseDueno::Asentamiento, true).contains("«espada_de_madera» no existe"));
        assert!(mal(con(objeto("dagas_cobre", json!(0))), ClaseDueno::Asentamiento, true).contains("entero mayor que 0"));
        assert!(mal(con(objeto("dagas_cobre", json!(1.5))), ClaseDueno::Asentamiento, true).contains("entero mayor que 0"));
        let dagas = json!({ "tipo": "objeto", "objeto": "dagas_cobre", "cantidad": 1 });
        assert!(mal(con(json!([dagas, dagas])), ClaseDueno::Asentamiento, true).contains("repite objeto «dagas_cobre»"));
        assert!(leer(json!({ "id": "H", "recompensas": objeto("dagas_cobre", json!(2)) }), ClaseDueno::Heroe, true).is_ok());
    }
}
