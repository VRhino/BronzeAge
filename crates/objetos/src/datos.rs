//! El catálogo de los objetos del héroe. Fuente única: los bloques `<!-- parametros:NOMBRE -->` de `Docs/Mechanics/Objetos_Del_Heroe.md` (`OBJETOS`,
//! `CLASES_HEROE` y `BOTIN_OBJETOS`), incrustados en el binario. Las cifras de combate y los precios son enteros en los datos; las probabilidades,
//! decimales, pasan una sola vez a milésimas (`Mil`). Al cargar se comprueba lo que las reglas dan por hecho (`Catalogo::cargar`), con pánico y mensaje.

use crate::equipo::Equipo;
use crate::inventario::Pieza;
use base::Mil;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const OBJETOS_DEL_HEROE: &str = include_str!("../../../Docs/Mechanics/Objetos_Del_Heroe.md");

/// El id del crisol (Catalogo_De_Objetos §6b): cada forja gasta tantos como su tier.
pub(crate) const CRISOL: &str = "crisol";

fn bloque(nombre: &str) -> Value {
    let marca = format!("<!-- parametros:{nombre} -->");
    let ini = OBJETOS_DEL_HEROE.find(&marca).unwrap_or_else(|| panic!("no hay bloque «{nombre}»")) + marca.len();
    let resto = &OBJETOS_DEL_HEROE[ini..];
    let fin = resto.find("<!-- /parametros -->").unwrap_or_else(|| panic!("el bloque «{nombre}» no se cierra"));
    let cuerpo = resto[..fin].trim().strip_prefix("```json").and_then(|c| c.strip_suffix("```")).unwrap_or_else(|| panic!("el bloque «{nombre}» no es json"));
    serde_json::from_str(cuerpo).unwrap_or_else(|e| panic!("bloque «{nombre}»: {e}"))
}

fn campo<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or_else(|| panic!("falta «{k}» en {v}"))
}

fn entero(v: &Value, k: &str) -> i64 {
    campo(v, k).as_i64().unwrap_or_else(|| panic!("«{k}» no es un entero en {v}"))
}

fn texto(v: &Value, k: &str) -> String {
    campo(v, k).as_str().unwrap_or_else(|| panic!("«{k}» no es un texto en {v}")).to_owned()
}

fn lista<'a>(v: &'a Value, k: &str) -> &'a Vec<Value> {
    campo(v, k).as_array().unwrap_or_else(|| panic!("«{k}» no es una lista en {v}"))
}

/// Una probabilidad decimal (0,002 como mínimo) a milésimas, que tiene que ser exacta y estar en (0, 1].
fn probabilidad(v: &Value) -> Mil {
    let p = campo(v, "probabilidad").as_f64().unwrap_or_else(|| panic!("«probabilidad» no es un número en {v}"));
    let m = (p * 1000.0).round();
    assert!((p * 1000.0 - m).abs() < 1e-6 && m > 0.0 && m <= 1000.0, "la probabilidad {p} no es una milésima en (0, 1]");
    m as Mil
}

/// Los cuatro atributos del héroe (Clases_De_Heroe §5): los de una clase, los que da una pieza o los repartidos.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Atributos {
    pub fuerza: i64,
    pub destreza: i64,
    pub armadura: i64,
    pub vitalidad: i64,
}

impl Atributos {
    /// Los que faltan en los datos valen 0 (un arma solo nombra los que da).
    fn de(v: &Value) -> Atributos {
        let a = |k: &str| v.get(k).map_or(0, |_| entero(v, k));
        Atributos { fuerza: a("fuerza"), destreza: a("destreza"), armadura: a("armadura"), vitalidad: a("vitalidad") }
    }

    pub fn mas(self, o: Atributos) -> Atributos {
        Atributos { fuerza: self.fuerza + o.fuerza, destreza: self.destreza + o.destreza, armadura: self.armadura + o.armadura, vitalidad: self.vitalidad + o.vitalidad }
    }

    fn por_calidad(self, pct: i64) -> Atributos {
        Atributos { fuerza: por_calidad(self.fuerza, pct), destreza: por_calidad(self.destreza, pct), armadura: por_calidad(self.armadura, pct), vitalidad: por_calidad(self.vitalidad, pct) }
    }
}

/// Una cifra por tipo de daño: el daño y la penetración de un arma, la defensa de una armadura.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PorDano {
    pub perforante: i64,
    pub cortante: i64,
    pub contundente: i64,
}

impl PorDano {
    fn de(v: &Value) -> PorDano {
        PorDano { perforante: entero(v, "perforante"), cortante: entero(v, "cortante"), contundente: entero(v, "contundente") }
    }

    fn por_calidad(self, pct: i64) -> PorDano {
        PorDano { perforante: por_calidad(self.perforante, pct), cortante: por_calidad(self.cortante, pct), contundente: por_calidad(self.contundente, pct) }
    }
}

/// Una cifra por la calidad de la forja (en %), redondeada al entero más cercano (las mitades hacia arriba; Catalogo_De_Objetos §6b, «La calidad»).
fn por_calidad(x: i64, pct: i64) -> i64 {
    (x * pct + 50).div_euclid(100)
}

/// Lo que da una pieza en combate (`combate` de la ficha, Catalogo_De_Objetos §4): el motor lo publica y Conquest lo aplica.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cifras {
    Arma { atributos: Atributos, dano: PorDano, penetracion: PorDano },
    Armadura { atributos: Atributos, defensa: PorDano, salud: i64 },
}

impl Cifras {
    /// Todas las cifras multiplicadas por la calidad (%) y redondeadas: las de una pieza forjada.
    pub(crate) fn por_calidad(self, pct: i64) -> Cifras {
        match self {
            Cifras::Arma { atributos, dano, penetracion } => Cifras::Arma { atributos: atributos.por_calidad(pct), dano: dano.por_calidad(pct), penetracion: penetracion.por_calidad(pct) },
            Cifras::Armadura { atributos, defensa, salud } => Cifras::Armadura { atributos: atributos.por_calidad(pct), defensa: defensa.por_calidad(pct), salud: por_calidad(salud, pct) },
        }
    }
}

/// La receta de forja de una pieza (`forja`, Catalogo_De_Objetos §6b).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receta {
    /// El id del plano de su tier (`plano_2` … `plano_5`); se gasta uno.
    pub plano: String,
    /// Los crisoles que gasta: tantos como su tier.
    pub crisoles: u32,
    /// Los recursos del almacén personal que cobra el núcleo, en unidades enteras, por nombre (`lingoteCobre`, `cueroCurtido`…).
    pub materiales: Vec<(String, i64)>,
    /// El rango de la calidad, en %, con los dos extremos.
    pub calidad: (i64, i64),
}

/// Lo que tienen en común un arma y una armadura: la tecnología que abre su metal (ninguna, el de inicio), sus cifras y su receta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeCombate {
    /// La tecnología de `metales` que abre comprarla y forjarla (Clases_De_Heroe §4: llevarla se puede siempre).
    pub tecnologia: Option<String>,
    pub cifras: Cifras,
    /// Solo en los tiers II a V.
    pub forja: Option<Receta>,
}

/// El peso de una armadura (`tipoArmadura`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Peso {
    Ligera,
    Media,
    Pesada,
}

impl Peso {
    fn desde_nombre(n: &str) -> Peso {
        match n {
            "ligera" => Peso::Ligera,
            "media" => Peso::Media,
            "pesada" => Peso::Pesada,
            _ => panic!("peso de armadura «{n}» desconocido"),
        }
    }
}

/// Los siete huecos del equipo del héroe (Objetos_Del_Heroe §3.1, punto 3, y el de la montura de Catalogo_De_Objetos §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hueco {
    Arma,
    Casco,
    Torso,
    Guantes,
    Pantalones,
    Botas,
    Montura,
}

impl Hueco {
    /// Los cinco de la armadura, en el orden del equipo.
    pub const ARMADURA: [Hueco; 5] = [Hueco::Casco, Hueco::Torso, Hueco::Guantes, Hueco::Pantalones, Hueco::Botas];

    fn de_armadura(n: &str) -> Hueco {
        match n {
            "casco" => Hueco::Casco,
            "torso" => Hueco::Torso,
            "guantes" => Hueco::Guantes,
            "pantalones" => Hueco::Pantalones,
            "botas" => Hueco::Botas,
            _ => panic!("hueco de armadura «{n}» desconocido"),
        }
    }
}

/// Lo que hace un consumible al usarlo (`efecto`, Catalogo_De_Objetos §3.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Consumo {
    /// Experiencia para una escuadra del héroe (`experienciaTropa`).
    ExperienciaTropa(i64),
    /// Un porcentaje de la experiencia que pide el nivel del héroe (`experienciaHeroePct`).
    ExperienciaHeroePct(i64),
    /// Una montura al azar (`monturaDe`, el cofre de monturas): una de la lista, con su probabilidad en milésimas, que suman 1.000.
    MonturaDe(Vec<(String, Mil)>),
}

/// Qué es un objeto (`tipo`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tipo {
    /// Va en el hueco del arma y define la clase.
    Arma { clase: String, combate: DeCombate },
    Armadura { hueco: Hueco, peso: Peso, combate: DeCombate },
    /// Va en el hueco de la montura. No se forja.
    Montura,
    /// Planos y crisoles, para la forja.
    Material,
    Consumible(Consumo),
}

/// La ficha de un objeto del catálogo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ficha {
    pub tipo: Tipo,
    /// Del 1 al 5; los materiales y consumibles sin tier no lo llevan (el plano lleva el suyo).
    pub tier: Option<u8>,
    /// Lo apilable va junto en una sola casilla.
    pub apilable: bool,
    /// En monedas: lo que cuesta en la tienda (si lo vende) y, por `ventaPct`, lo que da al venderlo.
    pub precio: i64,
}

impl Ficha {
    pub(crate) fn de_combate(&self) -> Option<&DeCombate> {
        match &self.tipo {
            Tipo::Arma { combate, .. } | Tipo::Armadura { combate, .. } => Some(combate),
            Tipo::Montura | Tipo::Material | Tipo::Consumible(_) => None,
        }
    }
}

/// Una clase del héroe (`CLASES_HEROE`, Clases_De_Heroe §7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clase {
    /// Los pesos de armadura que admite (Clases_De_Heroe §4).
    pub armaduras: BTreeSet<Peso>,
    pub atributos_base: Atributos,
    /// El equipo con el que nace un héroe de esta clase: su arma de inicio, el juego de inicio de `armaduraInicial` sin casco y la montura de tier I
    /// (Clases_De_Heroe §6, Catalogo_De_Objetos §3.3 y §5).
    pub inicial: Equipo,
}

/// Una entrada de una tabla de `BOTIN_OBJETOS`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entrada {
    /// `cantidad` de ese objeto con esa probabilidad (milésimas); con `solo_si_gana`, solo al que gana.
    Fija { objeto: String, cantidad: u32, probabilidad: Mil, solo_si_gana: bool },
    /// Uno de la lista, siempre (`unoDe`), con su probabilidad en milésimas, que suman 1.000.
    UnoDe(Vec<(String, Mil)>),
}

/// Una tabla de botín: cada entrada se tira por su lado (Progresion_Tropas_Y_Heroe §5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tabla(pub Vec<Entrada>);

/// Las tablas de `BOTIN_OBJETOS`, por fuente.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Botin {
    /// Por nivel del campamento de bandidos (1 a 3).
    pub bandidos: BTreeMap<u8, Tabla>,
    /// El de cada héroe en una batalla de Unity.
    pub unity: Tabla,
    /// El premio de una diaria individual del tablón (D5).
    pub mision_diaria: Tabla,
    /// Un alijo abierto.
    pub alijo: Tabla,
    /// El premio de rango de un mercenario, por rango (4 y 5).
    pub rango_mercenario: BTreeMap<u8, Tabla>,
}

/// El catálogo entero: los objetos, las clases y el botín.
#[derive(Clone, Debug)]
pub struct Catalogo {
    /// Lo que paga la tienda por lo que le vende el héroe, en % del precio de la ficha (D18).
    pub venta_pct: i64,
    /// La tienda vende hasta este tier.
    pub tienda_hasta_tier: u8,
    /// Las casillas de la mochila y del almacén de objetos (D22).
    pub casillas_mochila: u32,
    pub casillas_almacen: u32,
    /// Las fichas, por id.
    pub objetos: BTreeMap<String, Ficha>,
    /// Las clases, por id.
    pub clases: BTreeMap<String, Clase>,
    pub botin: Botin,
}

fn uno_de(v: &Value, k: &str, objetos: &BTreeMap<String, Ficha>) -> Vec<(String, Mil)> {
    let l: Vec<(String, Mil)> = lista(v, k).iter().map(|x| (texto(x, "objeto"), probabilidad(x))).collect();
    for (o, _) in &l {
        assert!(objetos.contains_key(o), "«{k}» nombra el objeto «{o}», que no existe");
    }
    assert_eq!(l.iter().map(|e| e.1).sum::<Mil>(), 1000, "las probabilidades de «{k}» no suman 1");
    l
}

fn tabla(v: &Value, objetos: &BTreeMap<String, Ficha>) -> Tabla {
    Tabla(
        v.as_array()
            .unwrap_or_else(|| panic!("una tabla de botín es una lista: {v}"))
            .iter()
            .map(|e| {
                if e.get("unoDe").is_some() {
                    return Entrada::UnoDe(uno_de(e, "unoDe", objetos));
                }
                let objeto = texto(e, "objeto");
                assert!(objetos.contains_key(&objeto), "el botín nombra el objeto «{objeto}», que no existe");
                let solo_si_gana = campo(e, "soloSiGana").as_bool().unwrap_or_else(|| panic!("«soloSiGana» no es un sí o no en {e}"));
                Entrada::Fija { objeto, cantidad: entero(e, "cantidad") as u32, probabilidad: probabilidad(e), solo_si_gana }
            })
            .collect(),
    )
}

fn por_nivel(v: &Value, objetos: &BTreeMap<String, Ficha>) -> BTreeMap<u8, Tabla> {
    v.as_object().unwrap_or_else(|| panic!("las tablas por nivel son un objeto: {v}")).iter().map(|(n, t)| (n.parse().unwrap_or_else(|_| panic!("nivel «{n}»")), tabla(t, objetos))).collect()
}

fn de_combate(o: &Value, tier: Option<u8>, metales: &BTreeMap<String, Option<String>>, cifras: Cifras) -> DeCombate {
    let id = texto(o, "id");
    let metal = texto(o, "metal");
    let tecnologia = metales.get(&metal).unwrap_or_else(|| panic!("{id}: metal «{metal}» desconocido")).clone();
    let f = campo(o, "forja");
    let forja = (!f.is_null()).then(|| {
        assert!(matches!(tier, Some(2..=5)), "{id}: solo se forja en los tiers II a V");
        let calidad = lista(f, "calidad");
        let materiales = campo(f, "materiales").as_object().unwrap_or_else(|| panic!("{id}: los materiales son un objeto")).iter().map(|(r, c)| (r.clone(), c.as_i64().unwrap_or_else(|| panic!("{id}: «{r}» no es un entero")))).collect();
        Receta {
            plano: format!("plano_{}", entero(f, "plano")),
            crisoles: entero(f, "crisoles") as u32,
            materiales,
            calidad: (calidad[0].as_i64().expect("calidad mínima"), calidad[1].as_i64().expect("calidad máxima")),
        }
    });
    DeCombate { tecnologia, cifras, forja }
}

fn ficha(o: &Value, metales: &BTreeMap<String, Option<String>>) -> Ficha {
    let tier = campo(o, "tier").as_u64().map(|t| t as u8);
    let combate = || campo(o, "combate");
    let tipo = match texto(o, "tipo").as_str() {
        "arma" => {
            let c = combate();
            Tipo::Arma { clase: texto(o, "clase"), combate: de_combate(o, tier, metales, Cifras::Arma { atributos: Atributos::de(campo(c, "atributos")), dano: PorDano::de(campo(c, "dano")), penetracion: PorDano::de(campo(c, "penetracion")) }) }
        }
        "armadura" => {
            let c = combate();
            let cifras = Cifras::Armadura { atributos: Atributos::de(campo(c, "atributos")), defensa: PorDano::de(campo(c, "defensa")), salud: entero(c, "salud") };
            Tipo::Armadura { hueco: Hueco::de_armadura(&texto(o, "hueco")), peso: Peso::desde_nombre(&texto(o, "tipoArmadura")), combate: de_combate(o, tier, metales, cifras) }
        }
        "montura" => Tipo::Montura,
        "material" => Tipo::Material,
        "consumible" => {
            let e = campo(o, "efecto");
            Tipo::Consumible(match (e.get("experienciaTropa"), e.get("experienciaHeroePct"), e.get("monturaDe")) {
                (Some(_), None, None) => Consumo::ExperienciaTropa(entero(e, "experienciaTropa")),
                (None, Some(_), None) => Consumo::ExperienciaHeroePct(entero(e, "experienciaHeroePct")),
                // Se resuelve con el catálogo entero, más abajo.
                (None, None, Some(_)) => Consumo::MonturaDe(Vec::new()),
                _ => panic!("{}: el efecto lleva una sola cosa", texto(o, "id")),
            })
        }
        t => panic!("tipo de objeto «{t}» desconocido"),
    };
    Ficha { tipo, tier, apilable: campo(o, "apilable").as_bool().expect("«apilable» es un sí o no"), precio: entero(o, "precio") }
}

impl Catalogo {
    pub fn cargar() -> Self {
        let (ob, cl, bo) = (bloque("OBJETOS"), bloque("CLASES_HEROE"), bloque("BOTIN_OBJETOS"));
        let metales: BTreeMap<String, Option<String>> =
            campo(&ob, "metales").as_object().expect("«metales» es un objeto").iter().map(|(m, t)| (m.clone(), t.as_str().map(str::to_owned))).collect();
        let mut objetos = BTreeMap::new();
        for o in lista(&ob, "objetos") {
            let id = texto(o, "id");
            assert!(objetos.insert(id.clone(), ficha(o, &metales)).is_none(), "el objeto «{id}» está dos veces");
        }
        // El efecto del cofre nombra monturas: se lee con todas las fichas ya cargadas.
        for o in lista(&ob, "objetos") {
            if let Some(e) = o.get("efecto").filter(|e| e.get("monturaDe").is_some()) {
                let monturas = uno_de(e, "monturaDe", &objetos);
                objetos.get_mut(&texto(o, "id")).expect("recién cargada").tipo = Tipo::Consumible(Consumo::MonturaDe(monturas));
            }
        }
        for (id, f) in &objetos {
            if let Some(r) = f.de_combate().and_then(|c| c.forja.as_ref()) {
                for m in [&r.plano, &CRISOL.to_owned()] {
                    assert!(objetos.get(m).is_some_and(|p| p.apilable && p.tipo == Tipo::Material), "{id}: la forja pide «{m}», que no es un material apilable del catálogo");
                }
            }
        }
        let de_tier_1 = |pred: &dyn Fn(&Tipo) -> bool| -> Vec<&String> { objetos.iter().filter(|(_, f)| f.tier == Some(1) && pred(&f.tipo)).map(|(id, _)| id).collect() };
        let monturas = de_tier_1(&|t| *t == Tipo::Montura);
        assert_eq!(monturas.len(), 1, "hay una sola montura de tier I, la de inicio");
        let montura = Pieza::del_catalogo(monturas[0]);
        let mut clases = BTreeMap::new();
        for c in lista(&cl, "clases") {
            let id = texto(c, "id");
            let armas = de_tier_1(&|t| matches!(t, Tipo::Arma { clase, .. } if *clase == id));
            assert_eq!(armas.len(), 1, "la clase «{id}» tiene una sola arma de inicio (tier I)");
            let peso = Peso::desde_nombre(&texto(c, "armaduraInicial"));
            let pieza = |h: Hueco| {
                let p = de_tier_1(&|t| matches!(t, Tipo::Armadura { hueco, peso: q, .. } if *hueco == h && *q == peso));
                assert_eq!(p.len(), 1, "la clase «{id}» tiene una sola pieza de inicio de {h:?} {peso:?}");
                Some(Pieza::del_catalogo(p[0]))
            };
            let inicial = Equipo { arma: Pieza::del_catalogo(armas[0]), casco: None, torso: pieza(Hueco::Torso), guantes: pieza(Hueco::Guantes), pantalones: pieza(Hueco::Pantalones), botas: pieza(Hueco::Botas), montura: Some(montura.clone()) };
            let armaduras = lista(c, "armaduras").iter().map(|p| Peso::desde_nombre(p.as_str().expect("un peso es un texto"))).collect();
            let clase = Clase { armaduras, atributos_base: Atributos::de(campo(c, "atributosBase")), inicial };
            assert!(clases.insert(id.clone(), clase).is_none(), "la clase «{id}» está dos veces");
        }
        for (id, f) in &objetos {
            if let Tipo::Arma { clase, .. } = &f.tipo {
                assert!(clases.contains_key(clase), "el arma «{id}» nombra la clase «{clase}», que no existe");
            }
        }
        let botin = Botin {
            bandidos: por_nivel(campo(&bo, "bandidos"), &objetos),
            unity: tabla(campo(&bo, "unity"), &objetos),
            mision_diaria: tabla(campo(&bo, "misionDiaria"), &objetos),
            alijo: tabla(campo(&bo, "alijo"), &objetos),
            rango_mercenario: por_nivel(campo(&bo, "rangoMercenario"), &objetos),
        };
        Catalogo {
            venta_pct: entero(&ob, "ventaPct"),
            tienda_hasta_tier: entero(&ob, "tiendaHastaTier") as u8,
            casillas_mochila: entero(&ob, "casillasMochila") as u32,
            casillas_almacen: entero(&ob, "casillasAlmacen") as u32,
            objetos,
            clases,
            botin,
        }
    }
}

impl Default for Catalogo {
    fn default() -> Self {
        static CATALOGO: std::sync::OnceLock<Catalogo> = std::sync::OnceLock::new();
        CATALOGO.get_or_init(Catalogo::cargar).clone()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn el_catalogo_tiene_30_armas_87_armaduras_y_5_monturas() {
        let c = Catalogo::cargar();
        let cuenta = |p: &dyn Fn(&Tipo) -> bool| c.objetos.values().filter(|f| p(&f.tipo)).count();
        assert_eq!(cuenta(&|t| matches!(t, Tipo::Arma { .. })), 30);
        assert_eq!(cuenta(&|t| matches!(t, Tipo::Armadura { .. })), 87);
        assert_eq!(cuenta(&|t| *t == Tipo::Montura), 5);
        assert_eq!(cuenta(&|t| *t == Tipo::Material), 5, "el crisol y los planos II a V");
        assert_eq!(cuenta(&|t| matches!(t, Tipo::Consumible(_))), 9, "ocho medallas y el cofre");
        assert_eq!(c.clases.len(), 5);
        assert_eq!((c.venta_pct, c.tienda_hasta_tier, c.casillas_mochila, c.casillas_almacen), (50, 2, 72, 144));
    }

    #[test]
    fn la_ficha_lleva_la_tecnologia_de_su_metal_sus_cifras_y_su_receta() {
        let c = Catalogo::cargar();
        let coraza = &c.objetos["torso_pesada_bronce"];
        assert_eq!((coraza.tier, coraza.precio, coraza.apilable), (Some(4), 2250, false));
        let Tipo::Armadura { hueco: Hueco::Torso, peso: Peso::Pesada, combate } = &coraza.tipo else { panic!("es un torso pesado") };
        assert_eq!(combate.tecnologia.as_deref(), Some("aleacion_bronce"));
        assert_eq!(combate.cifras, Cifras::Armadura { atributos: Atributos { fuerza: 1, destreza: 0, armadura: 8, vitalidad: 6 }, defensa: PorDano { perforante: 14, cortante: 14, contundente: 8 }, salud: 948 });
        let r = combate.forja.as_ref().expect("se forja");
        assert_eq!((r.plano.as_str(), r.crisoles, r.calidad), ("plano_4", 4, (90, 110)));
        assert_eq!(r.materiales, vec![("cueroCurtido".to_owned(), 4), ("lingoteBronce".to_owned(), 9)]);
        let lanza = &c.objetos["lanza_escudo_redondo_inicio"];
        let Tipo::Arma { clase, combate } = &lanza.tipo else { panic!("es un arma") };
        assert_eq!((clase.as_str(), &combate.tecnologia, &combate.forja), ("lanza_escudo_redondo", &None, &None), "la de inicio no pide tecnología ni se forja");
        assert_eq!(combate.cifras, Cifras::Arma { atributos: Atributos { armadura: 2, ..Atributos::default() }, dano: PorDano { perforante: 71, cortante: 0, contundente: 20 }, penetracion: PorDano { perforante: 2, cortante: 0, contundente: 2 } });
    }

    #[test]
    fn las_probabilidades_pasan_a_milesimas() {
        let c = Catalogo::cargar();
        assert_eq!(c.botin.unity.0[5], Entrada::Fija { objeto: "plano_5".into(), cantidad: 1, probabilidad: 2, solo_si_gana: false }, "0,002 es la más pequeña");
        assert_eq!(c.botin.alijo.0[1], Entrada::UnoDe(vec![("plano_3".into(), 700), ("plano_4".into(), 300)]));
        assert_eq!(c.objetos["cofre_monturas"].tipo, Tipo::Consumible(Consumo::MonturaDe(vec![("montura_escita".into(), 700), ("montura_tesalia".into(), 250), ("montura_nisea".into(), 50)])));
        assert_eq!((c.botin.bandidos.keys().copied().collect::<Vec<_>>(), c.botin.rango_mercenario.keys().copied().collect::<Vec<_>>()), (vec![1, 2, 3], vec![4, 5]));
    }

    #[test]
    fn cada_clase_lleva_su_base_sus_armaduras_y_su_equipo_inicial() {
        let c = Catalogo::cargar();
        let dagas = &c.clases["dagas"];
        assert_eq!(dagas.atributos_base, Atributos { fuerza: 3, destreza: 5, armadura: 0, vitalidad: 2 });
        assert_eq!(dagas.armaduras, [Peso::Ligera].into());
        assert_eq!(c.clases["espada_escudo_mediano"].armaduras, [Peso::Media, Peso::Pesada].into());
        for (id, clase) in &c.clases {
            let b = clase.atributos_base;
            assert_eq!(b.fuerza + b.destreza + b.armadura + b.vitalidad, 10, "{id}: todas suman 10 (Clases_De_Heroe §3.1)");
        }
    }

    #[test]
    fn la_calidad_redondea_al_mas_cercano() {
        assert_eq!((por_calidad(71, 90), por_calidad(71, 110), por_calidad(5, 90), por_calidad(5, 110), por_calidad(0, 110)), (64, 78, 5, 6, 0));
    }
}
