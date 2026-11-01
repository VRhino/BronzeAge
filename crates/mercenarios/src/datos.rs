//! Las cifras de los contratos. Fuente única: los bloques `MERCENARIOS_CONTRATO`, `TRABAJOS_MERCENARIOS` y `MEDALLAS_MERCENARIOS` de
//! `Docs/Mechanics/Mercenarios_Por_Contrato.md`, incrustados en el binario. El oro y las distancias pasan a milésimas; las horas, a milisegundos de mundo; los porcentajes
//! se quedan en enteros. Un id que no casa con su enum, o rangos y plazos fuera de orden, hacen fallar la carga.

use crate::tipos::{ClaseObjetivo, Medalla, TipoOrden, Trabajo};
use asentamiento::datos::{bloque_de, mil, texto_de_parametros_de};
use base::{Instante, Mil, MS_POR_MINUTO};
use militar::catalogo::Tropa;
use serde_json::Value;

pub const DOC: &str = include_str!("../../../Docs/Mechanics/Mercenarios_Por_Contrato.md");

const MS_POR_HORA: Instante = 60 * MS_POR_MINUTO;

/// Los bloques de parámetros del documento, para la identidad de las reglas.
pub fn texto_de_parametros() -> String {
    texto_de_parametros_de(DOC)
}

/// Un plazo de contrato temporal (§5): sus días, su tarifa sobre el valor del mercenario y el renombre de terminarlo sin despido (§8.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plazo {
    pub dias: u8,
    pub tarifa_pct: u32,
    pub renombre: u32,
}

/// Un rango (§8.3), en orden: el índice + 1 es su número.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rango {
    /// Su id (`a_sueldo`, `lanza`...), el que lleva el contrato 05.
    pub id: String,
    /// Renombre con el que se alcanza.
    pub renombre: u32,
    /// Escalón máximo de su licencia de reclutar (0: sin licencia).
    pub escalon: u8,
    /// Multiplicador de su tarifa.
    pub tarifa_pct: u32,
}

/// El renombre de cumplir un `atacar`, por clase de objetivo (§8.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenombreAtaque {
    pub bandidos_por_nivel: u32,
    pub ejercito: u32,
    pub caravana: u32,
    pub pueblo: u32,
    pub avanzada: u32,
    pub plaza: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReglasRenombre {
    pub atacar: RenombreAtaque,
    pub defensa_ganada: u32,
    pub escolta_llegada: u32,
    /// Por grado nuevo de una medalla: bronce, plata y oro.
    pub grado_medalla: [u32; 3],
}

/// Una orden que admite un trabajo después de la inicial (§6.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admite {
    Guardar,
    /// `guardar` del objetivo del contrato.
    GuardarObjetivo,
    IrA,
    Atacar,
    /// `atacar` de lo que esté a ≤ `radioDefensa` del objetivo del contrato.
    AtacarEnRadio,
    Unirse,
}

impl Admite {
    fn desde_id(s: &str) -> Option<Self> {
        Some(match s {
            "guardar" => Admite::Guardar,
            "guardar_objetivo" => Admite::GuardarObjetivo,
            "ir_a" => Admite::IrA,
            "atacar" => Admite::Atacar,
            "atacar_en_radio" => Admite::AtacarEnRadio,
            "unirse" => Admite::Unirse,
            _ => return None,
        })
    }
}

/// Una fila de `TRABAJOS_MERCENARIOS` (§4, §6.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilaTrabajo {
    pub trabajo: Trabajo,
    pub objetivos: Vec<ClaseObjetivo>,
    pub inicial: TipoOrden,
    pub admite: Vec<Admite>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Datos {
    /// En orden de días creciente.
    pub plazos: Vec<Plazo>,
    pub permanente_pct: u32,
    /// Oro por nivel del héroe en el valor del mercenario (milésimas).
    pub oro_por_nivel: Mil,
    /// Unidades de mapa en milésimas.
    pub radio_defensa: Mil,
    /// Plazo de `ir_a` y `atacar` (ms).
    pub plazo_orden: Instante,
    /// Tras un despido, hasta cuándo no se recontrata en la misma Facción (ms).
    pub espera_recontrato: Instante,
    /// Cuánto tiempo se puede valorar un contrato acabado (ms).
    pub valoracion: Instante,
    pub renombre: ReglasRenombre,
    pub buena_pct: u32,
    pub mala_pct: u32,
    /// En orden de renombre creciente, el primero con 0.
    pub rangos: Vec<Rango>,
    pub leva_propia: Vec<Tropa>,
    /// Una fila por trabajo, en el orden de [`Trabajo::TODOS`].
    pub trabajos: Vec<FilaTrabajo>,
    /// Umbrales de bronce, plata y oro, en el orden de [`Medalla::TODAS`].
    pub medallas: Vec<[u32; 3]>,
}

fn campo<'a>(v: &'a Value, ruta: &str) -> Result<&'a Value, String> {
    ruta.split('.').try_fold(v, |v, k| v.get(k)).ok_or_else(|| format!("falta «{ruta}»"))
}

fn entero(v: &Value, ruta: &str) -> Result<u32, String> {
    campo(v, ruta)?.as_u64().and_then(|x| u32::try_from(x).ok()).ok_or_else(|| format!("«{ruta}» no es un entero"))
}

fn texto<'a>(v: &'a Value, ruta: &str) -> Result<&'a str, String> {
    campo(v, ruta)?.as_str().ok_or_else(|| format!("«{ruta}» no es un texto"))
}

fn lista<'a>(v: &'a Value, ruta: &str) -> Result<&'a Vec<Value>, String> {
    campo(v, ruta)?.as_array().ok_or_else(|| format!("«{ruta}» no es una lista"))
}

fn tres(v: &Value, ruta: &str) -> Result<[u32; 3], String> {
    let l = lista(v, ruta)?;
    let n: Vec<u32> = l.iter().map(|x| x.as_u64().and_then(|x| u32::try_from(x).ok()).ok_or_else(|| format!("«{ruta}» lleva algo que no es un entero"))).collect::<Result<_, _>>()?;
    n.try_into().map_err(|_| format!("«{ruta}» no tiene tres cifras"))
}

fn creciente(n: &[u32]) -> bool {
    n.windows(2).all(|w| w[0] < w[1])
}

impl Datos {
    /// Las cifras del documento incrustado. Un documento mal formado es un fallo de compilación de las reglas: entra en pánico.
    pub fn cargar() -> Self {
        Self::desde(DOC).unwrap_or_else(|e| panic!("parámetros de mercenarios: {e}"))
    }

    pub fn desde(doc: &str) -> Result<Self, String> {
        let c = bloque_de(doc, "MERCENARIOS_CONTRATO")?;
        let plazos = lista(&c, "plazos")?
            .iter()
            .map(|p| Ok(Plazo { dias: u8::try_from(entero(p, "dias")?).map_err(|_| "plazo de más de 255 días")?, tarifa_pct: entero(p, "tarifaPct")?, renombre: entero(p, "renombre")? }))
            .collect::<Result<Vec<_>, String>>()?;
        if plazos.is_empty() || !creciente(&plazos.iter().map(|p| u32::from(p.dias)).collect::<Vec<_>>()) {
            return Err("los plazos han de ir en orden de días creciente".into());
        }
        let rangos = lista(&c, "rangos")?
            .iter()
            .map(|r| {
                let escalon = u8::try_from(entero(r, "escalon")?).map_err(|_| "escalón fuera de rango")?;
                Ok(Rango { id: texto(r, "id")?.to_owned(), renombre: entero(r, "renombre")?, escalon, tarifa_pct: entero(r, "tarifaPct")? })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if rangos.first().is_none_or(|r| r.renombre != 0) || !creciente(&rangos.iter().map(|r| r.renombre).collect::<Vec<_>>()) {
            return Err("los rangos han de ir en orden de renombre creciente, el primero con 0".into());
        }
        let leva_propia = lista(&c, "levaPropia")?
            .iter()
            .map(|t| t.as_str().and_then(Tropa::desde_nombre).ok_or_else(|| format!("levaPropia: «{t}» no es una tropa")))
            .collect::<Result<Vec<_>, String>>()?;
        let ataque = |k: &str| entero(&c, &format!("renombre.atacar.{k}"));
        let renombre = ReglasRenombre {
            atacar: RenombreAtaque {
                bandidos_por_nivel: ataque("bandidosPorNivel")?,
                ejercito: ataque("ejercito")?,
                caravana: ataque("caravana")?,
                pueblo: ataque("pueblo")?,
                avanzada: ataque("avanzada")?,
                plaza: ataque("plaza")?,
            },
            defensa_ganada: entero(&c, "renombre.defensaGanada")?,
            escolta_llegada: entero(&c, "renombre.escoltaLlegada")?,
            grado_medalla: tres(&c, "renombre.gradoMedalla")?,
        };
        let horas = |k: &str| entero(&c, k).map(|h| Instante::from(h) * MS_POR_HORA);
        Ok(Datos {
            plazos,
            permanente_pct: entero(&c, "permanentePct")?,
            oro_por_nivel: mil(f64::from(entero(&c, "oroPorNivelHeroe")?)),
            radio_defensa: mil(f64::from(entero(&c, "radioDefensa")?)),
            plazo_orden: horas("plazoOrdenHoras")?,
            espera_recontrato: horas("esperaRecontratoHoras")?,
            valoracion: horas("valoracionHoras")?,
            renombre,
            buena_pct: entero(&c, "valoracion.buenaPct")?,
            mala_pct: entero(&c, "valoracion.malaPct")?,
            rangos,
            leva_propia,
            trabajos: trabajos(&bloque_de(doc, "TRABAJOS_MERCENARIOS")?)?,
            medallas: medallas(&bloque_de(doc, "MEDALLAS_MERCENARIOS")?)?,
        })
    }

    /// La fila de un trabajo.
    pub fn trabajo(&self, t: Trabajo) -> &FilaTrabajo {
        &self.trabajos[t as usize]
    }

    /// El plazo temporal de esos días, si existe.
    pub fn plazo(&self, dias: u8) -> Option<&Plazo> {
        self.plazos.iter().find(|p| p.dias == dias)
    }

    /// Los umbrales de bronce, plata y oro de una medalla.
    pub fn umbrales(&self, m: Medalla) -> [u32; 3] {
        self.medallas[m as usize]
    }
}

/// Una fila por trabajo, en el orden del enum: un id desconocido, uno repetido o uno que falta hacen fallar la carga.
fn trabajos(v: &Value) -> Result<Vec<FilaTrabajo>, String> {
    let filas = v.as_array().ok_or("TRABAJOS_MERCENARIOS no es una lista")?;
    Trabajo::TODOS
        .iter()
        .map(|&t| {
            let mut de_t = filas.iter().filter(|f| f.get("id").and_then(Value::as_str) == Some(t.id()));
            let f = de_t.next().ok_or_else(|| format!("falta el trabajo «{}»", t.id()))?;
            if de_t.next().is_some() {
                return Err(format!("el trabajo «{}» está dos veces", t.id()));
            }
            let objetivos = lista(f, "objetivos")?
                .iter()
                .map(|o| o.as_str().and_then(ClaseObjetivo::desde_id).ok_or_else(|| format!("{}: «{o}» no es una clase de objetivo", t.id())))
                .collect::<Result<_, String>>()?;
            let inicial = TipoOrden::desde_id(texto(f, "inicial")?).ok_or_else(|| format!("{}: orden inicial desconocida", t.id()))?;
            let admite = lista(f, "admite")?
                .iter()
                .map(|a| a.as_str().and_then(Admite::desde_id).ok_or_else(|| format!("{}: «{a}» no es una orden admitida", t.id())))
                .collect::<Result<_, String>>()?;
            Ok(FilaTrabajo { trabajo: t, objetivos, inicial, admite })
        })
        .collect::<Result<Vec<_>, String>>()
        .and_then(|v| match filas.iter().filter_map(|f| f.get("id").and_then(Value::as_str)).find(|id| Trabajo::TODOS.iter().all(|t| t.id() != *id)) {
            Some(id) => Err(format!("«{id}» no es un trabajo")),
            None => Ok(v),
        })
}

/// Los umbrales de cada medalla, en el orden del enum, crecientes; las mismas reglas de ids que los trabajos.
fn medallas(v: &Value) -> Result<Vec<[u32; 3]>, String> {
    let filas = v.as_array().ok_or("MEDALLAS_MERCENARIOS no es una lista")?;
    if let Some(id) = filas.iter().filter_map(|f| f.get("id").and_then(Value::as_str)).find(|id| Medalla::TODAS.iter().all(|m| m.id() != *id)) {
        return Err(format!("«{id}» no es una medalla"));
    }
    Medalla::TODAS
        .iter()
        .map(|&m| {
            let mut de_m = filas.iter().filter(|f| f.get("id").and_then(Value::as_str) == Some(m.id()));
            let f = de_m.next().ok_or_else(|| format!("falta la medalla «{}»", m.id()))?;
            if de_m.next().is_some() {
                return Err(format!("la medalla «{}» está dos veces", m.id()));
            }
            let u = tres(f, "umbrales")?;
            if !creciente(&u) || u[0] == 0 {
                return Err(format!("los umbrales de «{}» han de ser positivos y crecientes", m.id()));
            }
            Ok(u)
        })
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn los_tres_bloques_del_documento_se_leen_con_sus_cifras() {
        let d = Datos::cargar();
        assert_eq!(d.plazos.iter().map(|p| p.dias).collect::<Vec<_>>(), [1, 3, 7]);
        assert_eq!(d.plazo(3).map(|p| (p.tarifa_pct, p.renombre)), Some((35, 25)));
        assert_eq!(d.plazo(2), None);
        assert_eq!(d.permanente_pct, 500);
        assert_eq!(d.oro_por_nivel, 20_000);
        assert_eq!(d.radio_defensa, 20_000);
        assert_eq!(d.plazo_orden, 24 * MS_POR_HORA);
        assert_eq!(d.rangos.len(), 5);
        assert_eq!((d.rangos[2].id.as_str(), d.rangos[2].renombre, d.rangos[2].escalon, d.rangos[2].tarifa_pct), ("capitan", 300, 3, 150));
        assert_eq!(d.leva_propia, [Tropa::MiliciaLanceros, Tropa::Granjeros, Tropa::Mineros, Tropa::Lenadores]);
        assert_eq!(d.renombre.atacar.plaza, 30);
        assert_eq!(d.renombre.grado_medalla, [10, 30, 80]);
        assert_eq!(d.umbrales(Medalla::Leal), [7, 30, 90]);
        let defender = d.trabajo(Trabajo::Defender);
        assert_eq!(defender.objetivos, [ClaseObjetivo::Plaza, ClaseObjetivo::Pueblo, ClaseObjetivo::Avanzada]);
        assert_eq!((defender.inicial, defender.admite.clone()), (TipoOrden::Guardar, vec![Admite::GuardarObjetivo, Admite::IrA, Admite::AtacarEnRadio]));
        assert_eq!(d.trabajo(Trabajo::Permanente).objetivos, [ClaseObjetivo::Faccion]);
        assert!(Trabajo::TODOS.iter().all(|&t| d.trabajo(t).trabajo == t));
    }

    /// El documento con una línea cambiada.
    fn con(de: &str, a: &str) -> String {
        assert!(DOC.contains(de), "el documento contiene «{de}»");
        DOC.replacen(de, a, 1)
    }

    #[test]
    fn un_id_que_no_casa_con_su_enum_hace_fallar_la_carga() {
        assert!(Datos::desde(&con("\"id\": \"escoltar\"", "\"id\": \"escoltar_lejos\"")).unwrap_err().contains("escoltar"));
        assert!(Datos::desde(&con("\"id\": \"leal\"", "\"id\": \"fiel\"")).unwrap_err().contains("fiel"));
        assert!(Datos::desde(&con("\"objetivos\": [\"caravana\"]", "\"objetivos\": [\"carro\"]")).unwrap_err().contains("carro"));
        assert!(Datos::desde(&con("\"guardar_objetivo\", \"ir_a\"]", "\"guardar_objetivo\", \"volar\"]")).unwrap_err().contains("volar"));
        assert!(Datos::desde(&con("\"lenadores\"]", "\"talladores\"]")).unwrap_err().contains("talladores"));
    }

    #[test]
    fn rangos_plazos_y_umbrales_han_de_ir_en_orden_creciente() {
        assert!(Datos::desde(&con("\"renombre\": 300", "\"renombre\": 50")).is_err());
        assert!(Datos::desde(&con("\"id\": \"a_sueldo\", \"renombre\": 0", "\"id\": \"a_sueldo\", \"renombre\": 5")).is_err());
        assert!(Datos::desde(&con("{ \"dias\": 3,", "{ \"dias\": 1,")).is_err());
        assert!(Datos::desde(&con("[7, 30, 90]", "[7, 3, 90]")).is_err());
    }
}
