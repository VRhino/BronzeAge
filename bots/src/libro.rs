//! El libro de agravios: lo que cada Facción de bots recuerda de las demás, jugadores incluidos, y lo que saca de ello. Son reglas puras: el cerebro lee los hechos y actúa según [`Estado`].
//! El libro va en el registro para sobrevivir a los reinicios; las cifras, en [`crate::balance`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{azar::hash, balance as b};

/// Lo que se anota. Hechos iguales se juntan: una página tiene como mucho un asiento por clase, así que el libro no crece con el tiempo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Asiento {
    PlazaConquistada,
    AlianzaRota,
    GuerraDeclarada,
    PuebloSaqueado,
    CaravanaAsaltada,
    PlazaSitiada,
    TruequeIncumplido,
    TruequeCumplido,
    EnemigoComun,
    Frontera,
    Dominio,
    /// Tiene el estaño que necesita y ha decidido quitárselo.
    DisputaEstano,
}

impl Asiento {
    pub fn peso(self) -> f64 {
        match self {
            Self::PlazaConquistada => b::PESO_PLAZA_CONQUISTADA,
            Self::AlianzaRota => b::PESO_ALIANZA_ROTA,
            Self::GuerraDeclarada => b::PESO_GUERRA_DECLARADA,
            Self::PuebloSaqueado => b::PESO_PUEBLO_SAQUEADO,
            Self::CaravanaAsaltada => b::PESO_CARAVANA_ASALTADA,
            Self::PlazaSitiada => b::PESO_PLAZA_SITIADA,
            Self::TruequeIncumplido => b::PESO_TRUEQUE_INCUMPLIDO,
            Self::TruequeCumplido => b::PESO_TRUEQUE_CUMPLIDO,
            Self::EnemigoComun => b::PESO_ENEMIGO_COMUN,
            Self::DisputaEstano => b::PESO_DISPUTA_ESTANO,
            // Estos dos se anotan por hora, con su propio peso.
            Self::Frontera => b::ROCE_FRONTERA_POR_HORA,
            Self::Dominio => b::ROCE_DOMINIO_POR_HORA,
        }
    }
}

/// La escalera de antagonismo: lo que hacen sus miembros con los de la otra Facción. Es lo único que se publica.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Estado {
    Odio,
    Hostilidad,
    Recelo,
    Neutral,
    Amistad,
    Alianza,
}

impl Estado {
    pub fn de(relacion: f64) -> Self {
        match relacion {
            r if r >= b::UMBRAL_ALIANZA => Self::Alianza,
            r if r >= b::UMBRAL_AMISTAD => Self::Amistad,
            r if r > b::UMBRAL_RECELO => Self::Neutral,
            r if r > b::UMBRAL_HOSTILIDAD => Self::Recelo,
            r if r > b::UMBRAL_ODIO => Self::Hostilidad,
            _ => Self::Odio,
        }
    }
}

/// Lo que recuerda de una Facción.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pagina {
    /// Cada clase con su peso acumulado y cuándo se apuntó por última vez: el olvido se aplica al leer.
    asientos: BTreeMap<Asiento, (f64, i64)>,
    /// Le rompió una alianza: no vuelve a aliarse con ella, aunque olvide.
    #[serde(default)]
    pub traicion: bool,
    /// Lo que resistió un asedio suyo: se atreve con ella hasta entonces.
    #[serde(default)]
    pub confianza_hasta: i64,
}

impl Pagina {
    pub fn vacia(&self) -> bool {
        self.asientos.is_empty()
    }

    /// Anota `peso` en su clase, juntándolo con lo que quedaba.
    pub fn anotar(&mut self, a: Asiento, peso: f64, t: i64, vida_ms: i64) {
        let e = self.asientos.entry(a).or_insert((0.0, t));
        // Con tope: un hecho que se repite no ha de dejar un rencor que tarde meses en bajar del odio.
        *e = ((olvido(e.0, t - e.1, vida_ms) + peso).clamp(-b::TOPE_DE_ASIENTO, b::TOPE_DE_ASIENTO), t);
    }

    pub fn relacion(&self, t: i64, vida_ms: i64) -> f64 {
        self.asientos.values().map(|(p, en)| olvido(*p, t - en, vida_ms)).sum::<f64>().clamp(-100.0, 100.0)
    }

    /// Lo que pesa hoy cada clase de asiento, con el olvido aplicado: para el panel.
    pub fn vigentes(&self, t: i64, vida_ms: i64) -> Vec<(Asiento, f64)> {
        self.asientos.iter().map(|(a, (p, en))| (*a, olvido(*p, t - en, vida_ms))).collect()
    }

    /// Solo lo que duele de verdad: lo que sigue pesando de agravios grandes, sin el roce ni los trueques.
    pub fn agravio_grave(&self, t: i64, vida_ms: i64) -> f64 {
        self.asientos.iter().filter(|(a, _)| matches!(a, Asiento::PlazaConquistada | Asiento::AlianzaRota | Asiento::PuebloSaqueado)).map(|(_, (p, en))| olvido(*p, t - en, vida_ms)).sum()
    }
}

fn olvido(peso: f64, edad_ms: i64, vida_ms: i64) -> f64 {
    peso * 0.5f64.powf(edad_ms.max(0) as f64 / vida_ms as f64)
}

/// Un objetivo de varios días de la Facción.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Objetivo {
    /// Recuperar una plaza perdida.
    Recuperar { plaza: String, faccion: String },
    /// Ir a por su enemigo jurado.
    Enemigo { faccion: String },
    /// Quitarle el estaño a quien lo tiene: la plaza es la suya más cercana al yacimiento.
    Estano { plaza: String, faccion: String },
}

impl Objetivo {
    pub fn faccion(&self) -> &str {
        match self {
            Self::Recuperar { faccion, .. } | Self::Enemigo { faccion } | Self::Estano { faccion, .. } => faccion,
        }
    }
}

/// El libro de una Facción: va entero en el registro.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Libro {
    pub paginas: BTreeMap<String, Pagina>,
    /// Cuándo vio un bot por primera vez a cada Facción: el contrato no da su fecha de creación.
    pub vistas: BTreeMap<String, i64>,
    /// Las plazas que perdió y quién se las quitó.
    pub plazas_perdidas: BTreeMap<String, (String, i64)>,
    /// Hasta cuándo escolta sus caravanas, tras un asalto.
    pub escoltas_hasta: i64,
    /// Hasta cuándo acude a sus pueblos, tras un saqueo.
    pub pueblos_hasta: i64,
    /// Hasta cuándo prefiere tropa a expansión, tras perder una guerra.
    pub cautela_hasta: i64,
    /// Le han traicionado alguna vez: pide más para aliarse.
    pub traicionada: bool,
    /// Su enemigo jurado.
    pub enemigo: Option<String>,
    pub objetivo: Option<(Objetivo, i64)>,
    /// Los yacimientos de estaño que han visto sus bots, con su posición.
    #[serde(default)]
    pub estano_visto: BTreeMap<String, (i64, i64)>,
    /// Cómo piensa conseguir el estaño, y qué yacimiento.
    #[serde(default)]
    pub estano: Option<PlanEstano>,
}

/// Cómo piensa conseguir el estaño una Facción.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanEstano {
    pub nodo: String,
    pub via: Via,
    /// Cuántas veces lo pidió por trueque sin conseguirlo: la paciencia se acaba.
    pub peticiones: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Via {
    /// Lo tiene en su zona: marca el pueblo.
    Propio,
    /// Está libre: funda al lado.
    Fundar,
    Trueque { faccion: String },
    Guerra { faccion: String },
}

/// Qué hacer con un yacimiento de estaño. Libre, fundar; de un aliado o amigo, trueque; de un enemigo, guerra. De una Facción neutral lo decide el carácter de su Rey:
/// `peticiones` son los trueques que no dieron fruto, y cada uno acerca la guerra. A un amigo nunca se le declara.
pub fn via_del_estano(dueno: Option<(&str, Estado)>, belicoso: i32, rey: [Rasgo; 2], mias: usize, suyas: usize, peticiones: u32) -> Via {
    let Some((f, estado)) = dueno else { return Via::Fundar };
    let (trueque, guerra) = (Via::Trueque { faccion: f.to_string() }, Via::Guerra { faccion: f.to_string() });
    match estado {
        Estado::Alianza | Estado::Amistad => trueque,
        Estado::Odio | Estado::Hostilidad => guerra,
        Estado::Recelo | Estado::Neutral => {
            let caracter: i32 = rey
                .iter()
                .map(|r| match r {
                    Rasgo::Orgulloso => 1,
                    Rasgo::Oportunista if mias > suyas => 1,
                    Rasgo::Codicioso | Rasgo::Cauto => -1,
                    _ => 0,
                })
                .sum();
            let fuerza = (mias as i32 - suyas as i32).signum();
            let recelo = (estado == Estado::Recelo) as i32;
            if belicoso + caracter + fuerza + recelo + peticiones as i32 > 0 {
                guerra
            } else {
                trueque
            }
        }
    }
}

impl Libro {
    pub fn pagina(&mut self, f: &str) -> &mut Pagina {
        self.paginas.entry(f.to_string()).or_default()
    }

    pub fn relacion(&self, f: &str, t: i64, vida_ms: i64) -> f64 {
        self.paginas.get(f).map_or(0.0, |p| p.relacion(t, vida_ms))
    }

    pub fn estado(&self, f: &str, t: i64, vida_ms: i64) -> Estado {
        Estado::de(self.relacion(f, t, vida_ms))
    }

    /// Nueva para esta Facción: nivel bajo, poco tiempo y nada anotado (una Facción antigua que nunca se cruzó con ella también tiene la página vacía).
    /// El roce de vecindad no cuenta como anotado: agravios y favores, sí.
    pub fn nueva(&self, f: &str, nivel: u32, t: i64) -> bool {
        let sin_nada = self.paginas.get(f).is_none_or(|p| p.asientos.keys().all(|a| matches!(a, Asiento::Frontera | Asiento::Dominio)));
        nivel <= b::NUEVA_NIVEL_MAX && t - self.vistas.get(f).copied().unwrap_or(t) <= b::NUEVA_TIEMPO_MS && sin_nada
    }

    /// El umbral para aliarse con otra: más alto si ya la traicionaron.
    pub fn umbral_de_alianza(&self) -> f64 {
        b::UMBRAL_ALIANZA + if self.traicionada { b::RECARGO_TRAS_TRAICION } else { 0.0 }
    }
}

/// El carácter del Rey: da personalidad a su Facción y se puede aprovechar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rasgo {
    Rencoroso,
    Oportunista,
    Leal,
    Orgulloso,
    Codicioso,
    Cauto,
}

const RASGOS: [Rasgo; 6] = [Rasgo::Rencoroso, Rasgo::Oportunista, Rasgo::Leal, Rasgo::Orgulloso, Rasgo::Codicioso, Rasgo::Cauto];

/// Los dos rasgos y la debilidad de un Rey, fijos por su héroe: la debilidad es el segundo rasgo, el que se puede aprovechar. Leal y Oportunista no van juntos.
pub fn rasgos(rey: &str) -> [Rasgo; 2] {
    let h = hash(rey) as usize;
    let a = RASGOS[h % 6];
    let mut d = RASGOS[(h >> 8) % 6];
    if d == a || matches!((a, d), (Rasgo::Leal, Rasgo::Oportunista) | (Rasgo::Oportunista, Rasgo::Leal)) {
        d = RASGOS[(RASGOS.iter().position(|x| *x == a).unwrap() + 3) % 6];
    }
    [a, d]
}

/// La vida media del olvido según el Rey: el rencoroso tarda el doble.
pub fn vida_media(rey: Option<&str>) -> i64 {
    if rey.is_some_and(|r| rasgos(r).contains(&Rasgo::Rencoroso)) {
        2 * b::VIDA_MEDIA_MS
    } else {
        b::VIDA_MEDIA_MS
    }
}

/// Los nombres por cultura: cada Facción de amigos sale de una lista, para que sus miembros suenen del mismo pueblo.
const CULTURAS: [&[&str]; 6] = [
    &["Tudhaliya", "Suppiluliuma", "Mursili", "Hattusili", "Muwatalli", "Arnuwanda", "Puduhepa", "Telipinu", "Labarna", "Zidanta"],
    &["Ammurapi", "Niqmaddu", "Ibiranu", "Yaqarum", "Niqmepa", "Sharelli", "Ahatmilku", "Ilimilku", "Thariyelli", "Arhalbu"],
    &["Agamemnon", "Nestor", "Atreo", "Idomeneo", "Minos", "Teseo", "Pelops", "Ariadna", "Clitemnestra", "Diomedes"],
    &["Ramses", "Tutmosis", "Amenofis", "Seti", "Horemheb", "Merenptah", "Hatshepsut", "Nefertari", "Ahmose", "Tiye"],
    &["Hammurabi", "Shamshi", "Burnaburiash", "Kurigalzu", "Nazimaruttash", "Adad", "Enlil", "Ishme", "Naram", "Sargon"],
    &["Kirta", "Danel", "Aqhat", "Hiram", "Ahiram", "Sidqia", "Shipitbaal", "Yehimilk", "Elibaal", "Abibaal"],
];

/// El nombre del `n`-ésimo bot del grupo `grupo`: de 1 a 24 caracteres, como pide `crear_heroe`. Los que se repiten llevan ordinal.
pub fn nombre(grupo: u32, n: usize) -> String {
    let lista = CULTURAS[grupo as usize % CULTURAS.len()];
    let base = lista[n % lista.len()];
    match n / lista.len() {
        0 => base.to_string(),
        v => format!("{base} {}", v + 1),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const DIA: i64 = 86_400_000;

    #[test]
    fn un_agravio_se_olvida_a_la_mitad_en_su_vida_media() {
        let mut p = Pagina::default();
        p.anotar(Asiento::PlazaConquistada, -60.0, 0, 7 * DIA);
        assert_eq!(p.relacion(0, 7 * DIA), -60.0);
        assert!((p.relacion(7 * DIA, 7 * DIA) + 30.0).abs() < 1e-9);
        // Hechos iguales se juntan: la página no crece.
        p.anotar(Asiento::PlazaConquistada, -60.0, 7 * DIA, 7 * DIA);
        assert!((p.relacion(7 * DIA, 7 * DIA) + 90.0).abs() < 1e-9);
        assert_eq!(p.asientos.len(), 1);
        for _ in 0..20 {
            p.anotar(Asiento::PlazaConquistada, -60.0, 7 * DIA, 7 * DIA);
        }
        assert_eq!(p.relacion(7 * DIA, 7 * DIA), -100.0, "con tope");
    }

    #[test]
    fn la_escalera_cae_en_sus_umbrales() {
        assert_eq!(Estado::de(50.0), Estado::Alianza);
        assert_eq!(Estado::de(20.0), Estado::Amistad);
        assert_eq!(Estado::de(0.0), Estado::Neutral);
        assert_eq!(Estado::de(-20.0), Estado::Recelo);
        assert_eq!(Estado::de(-50.0), Estado::Hostilidad);
        assert_eq!(Estado::de(-75.0), Estado::Odio);
    }

    #[test]
    fn nueva_pide_las_tres_condiciones() {
        let mut l = Libro::default();
        l.vistas.insert("f-2".into(), 0);
        assert!(l.nueva("f-2", 1, DIA));
        assert!(!l.nueva("f-2", 2, DIA), "nivel");
        assert!(!l.nueva("f-2", 1, 8 * DIA), "tiempo");
        l.pagina("f-2").anotar(Asiento::Frontera, -1.0, 0, 7 * DIA);
        assert!(l.nueva("f-2", 1, DIA), "ser vecina no es haber hecho nada");
        l.pagina("f-2").anotar(Asiento::CaravanaAsaltada, -15.0, 0, 7 * DIA);
        assert!(!l.nueva("f-2", 1, DIA), "si agrede pierde el respeto");
    }

    #[test]
    fn el_estano_se_consigue_segun_quien_lo_tiene_y_el_caracter_del_rey() {
        let tranquilo = [Rasgo::Leal, Rasgo::Rencoroso];
        let g = Via::Guerra { faccion: "f".into() };
        let c = Via::Trueque { faccion: "f".into() };
        assert_eq!(via_del_estano(None, 2, tranquilo, 1, 1, 0), Via::Fundar);
        assert_eq!(via_del_estano(Some(("f", Estado::Alianza)), 2, tranquilo, 5, 1, 9), c, "a un aliado se le pide");
        assert_eq!(via_del_estano(Some(("f", Estado::Hostilidad)), -2, tranquilo, 1, 5, 0), g, "a un enemigo se le quita");
        assert_eq!(via_del_estano(Some(("f", Estado::Neutral)), 2, tranquilo, 1, 1, 0), g, "un militarista prefiere la guerra");
        assert_eq!(via_del_estano(Some(("f", Estado::Neutral)), -2, tranquilo, 1, 1, 0), c, "un mercantil, el comercio");
        assert_eq!(via_del_estano(Some(("f", Estado::Neutral)), 0, tranquilo, 1, 1, 0), c, "sin motivo, comercia");
        assert_eq!(via_del_estano(Some(("f", Estado::Neutral)), 0, [Rasgo::Orgulloso, Rasgo::Leal], 1, 1, 0), g);
        assert_eq!(via_del_estano(Some(("f", Estado::Neutral)), 0, [Rasgo::Cauto, Rasgo::Leal], 1, 1, 2), g, "la paciencia se acaba");
    }

    #[test]
    fn un_rey_tiene_dos_rasgos_compatibles_y_fijos() {
        for r in (0..200).map(|i| format!("heroe-{i}")) {
            let [a, d] = rasgos(&r);
            assert_ne!(a, d);
            assert!(!matches!((a, d), (Rasgo::Leal, Rasgo::Oportunista) | (Rasgo::Oportunista, Rasgo::Leal)), "{r}");
            assert_eq!(rasgos(&r), [a, d]);
        }
    }

    #[test]
    fn los_nombres_caben_y_no_se_repiten_en_un_grupo() {
        let v: Vec<_> = (0..40).map(|n| nombre(0, n)).collect();
        assert!(v.iter().all(|n| n.chars().count() <= 24));
        assert_eq!(v.iter().collect::<std::collections::BTreeSet<_>>().len(), 40);
    }
}
