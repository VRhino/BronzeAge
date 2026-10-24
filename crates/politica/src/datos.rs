//! Las cifras de la política. Fuente única: los bloques `<!-- parametros:NOMBRE -->` de `Docs/Mechanics/Diplomacia_Facciones.md`, incrustados en el
//! binario. Los días pasan a milisegundos de mundo; la experiencia a **milésimas** (`xp.combate` es 2,5); el ratio del combate digno a ppm.
//! Las cifras de reputación (umbral para proponer alianzas, ventana de la guerra tras romper) son de `comercio::datos::ReglasReputacion`.

use crate::sigilo::Catalogo;
use crate::tipos::{Grupo, Grupos};
use asentamiento::datos::{bloque_de, mil, ppm, texto_de_parametros_de};
use base::{Instante, Mil, MS_POR_MINUTO};
use serde_json::Value;

pub const DIPLOMACIA: &str = include_str!("../../../Docs/Mechanics/Diplomacia_Facciones.md");

/// Un día de mundo en milisegundos.
pub const DIA: Instante = 24 * 60 * MS_POR_MINUTO;

/// Los bloques de parámetros del documento, para la identidad de las reglas (hash del núcleo, desde 5B).
pub fn texto_de_parametros() -> String {
    texto_de_parametros_de(DIPLOMACIA)
}

fn bloque(nombre: &str) -> Value {
    bloque_de(DIPLOMACIA, nombre).unwrap_or_else(|e| panic!("{e}"))
}

fn num(v: &Value, ruta: &str) -> f64 {
    ruta.split('.').try_fold(v, |v, k| v.get(k)).and_then(Value::as_f64).unwrap_or_else(|| panic!("falta el número «{ruta}»"))
}

fn dias(v: &Value, ruta: &str) -> Instante {
    (num(v, ruta) * DIA as f64).round() as Instante
}

fn lista_u32(v: &Value) -> Vec<u32> {
    v.as_array().expect("lista").iter().map(|x| x.as_u64().expect("entero") as u32).collect()
}

/// `NIVEL_FACCION.xp`: experiencia por hecho, en milésimas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XpPorHecho {
    pub combate: Mil,
    pub edificio_completado: Mil,
    pub conquista: Mil,
    pub ascenso_por_nivel: Mil,
    pub fundacion: Mil,
    pub trueque_cumplido: Mil,
    pub bandidos: Mil,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasNivel {
    pub nivel_maximo: u32,
    /// Umbrales acumulados de experiencia (milésimas) para pasar del nivel `i + 1` al `i + 2`.
    pub umbrales: Vec<Mil>,
    pub xp: XpPorHecho,
    /// Desde este nivel de Facción los bandidos ya no dan experiencia.
    pub nivel_sin_xp_bandidos: u32,
    /// `ratioCombateDigno` en ppm: el bando débil tiene al menos esta parte del poder del fuerte (el `Informe::digno` de `combate`).
    pub ratio_combate_digno: i64,
    /// `CAP_FUNDACION_POR_NIVEL`: plazas que la Facción puede fundar en total, por nivel 1..=10.
    pub cap_fundacion: Vec<u32>,
    /// `CUPO_NIVEL_ASENTAMIENTO.maxNivel2|maxNivel3`, por nivel de Facción 1..=10.
    pub cupo_nivel2: Vec<u32>,
    pub cupo_nivel3: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reglas {
    /// `CIUDADANIA.cooldownCreacionFaccionDias` (ms): tras dejar una Facción no se crea otra hasta que pase.
    pub cooldown_creacion: Instante,
    /// `CIUDADANIA.cooldownCambioResidenciaDias` (ms).
    pub cooldown_residencia: Instante,
    /// `CAPITAL.cooldownDias` (ms).
    pub cooldown_capital: Instante,
    /// `PUERTA.cerradaAPorDefecto`.
    pub puerta_por_defecto: Grupos,
    /// Caducidad (ms) de las propuestas de relación, anexión y fusión.
    pub caducidad_relacion: Instante,
    pub caducidad_anexion: Instante,
    pub caducidad_fusion: Instante,
    pub nivel: ReglasNivel,
    pub sigilo: Catalogo,
}

impl Reglas {
    pub fn cargar() -> Self {
        let ciudadania = bloque("CIUDADANIA");
        let nivel = bloque("NIVEL_FACCION");
        let cupo = bloque("CUPO_NIVEL_ASENTAMIENTO");
        let xp = |k: &str| mil(num(&nivel, &format!("xp.{k}")));
        let puerta = bloque("PUERTA")["cerradaAPorDefecto"]
            .as_array()
            .expect("cerradaAPorDefecto")
            .iter()
            .map(|g| Grupo::desde_nombre(g.as_str().expect("grupo")).unwrap_or_else(|| panic!("grupo «{g}»")))
            .collect::<Vec<_>>();
        Self {
            cooldown_creacion: dias(&ciudadania, "cooldownCreacionFaccionDias"),
            cooldown_residencia: dias(&ciudadania, "cooldownCambioResidenciaDias"),
            cooldown_capital: dias(&bloque("CAPITAL"), "cooldownDias"),
            puerta_por_defecto: Grupos::de(puerta),
            caducidad_relacion: dias(&bloque("RELACION_PROPUESTA"), "caducidadDias"),
            caducidad_anexion: dias(&bloque("ANEXION"), "caducidadDias"),
            caducidad_fusion: dias(&bloque("FUSION"), "caducidadDias"),
            nivel: ReglasNivel {
                nivel_maximo: num(&nivel, "nivelMaximo") as u32,
                umbrales: nivel["xpParaNivel"].as_array().expect("xpParaNivel").iter().map(|x| mil(x.as_f64().expect("umbral"))).collect(),
                xp: XpPorHecho {
                    combate: xp("combate"),
                    edificio_completado: xp("edificioCompletado"),
                    conquista: xp("conquista"),
                    ascenso_por_nivel: xp("ascensoPorNivel"),
                    fundacion: xp("fundacion"),
                    trueque_cumplido: xp("truequeCumplido"),
                    bandidos: xp("bandidos"),
                },
                nivel_sin_xp_bandidos: num(&nivel, "nivelSinXpBandidos") as u32,
                ratio_combate_digno: ppm(num(&nivel, "ratioCombateDigno")),
                cap_fundacion: lista_u32(&bloque("CAP_FUNDACION_POR_NIVEL")),
                cupo_nivel2: lista_u32(&cupo["maxNivel2"]),
                cupo_nivel3: lista_u32(&cupo["maxNivel3"]),
            },
            sigilo: Catalogo::de(&bloque("CATALOGO_SIGILO")),
        }
    }
}

impl Default for Reglas {
    fn default() -> Self {
        static REGLAS: std::sync::OnceLock<Reglas> = std::sync::OnceLock::new();
        REGLAS.get_or_init(Reglas::cargar).clone()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn las_cifras_son_las_del_documento() {
        let r = Reglas::cargar();
        assert_eq!((r.cooldown_creacion, r.cooldown_residencia, r.cooldown_capital), (7 * DIA, 3 * DIA, 14 * DIA));
        assert_eq!((r.caducidad_relacion, r.caducidad_anexion, r.caducidad_fusion), (3 * DIA, 3 * DIA, 3 * DIA));
        assert_eq!(r.puerta_por_defecto, Grupos::de([Grupo::Neutrales, Grupo::Enemigos]), "Q1: cerrada a neutrales y enemigos");
        let n = &r.nivel;
        assert_eq!(n.nivel_maximo, 10);
        assert_eq!(n.umbrales, [100, 180, 300, 480, 760, 1200, 1900, 3000, 4800].map(|x| x * 1000));
        assert_eq!(
            (n.xp.combate, n.xp.edificio_completado, n.xp.conquista, n.xp.ascenso_por_nivel, n.xp.fundacion, n.xp.trueque_cumplido, n.xp.bandidos),
            (2_500, 1_000, 10_000, 30_000, 20_000, 2_000, 1_000)
        );
        assert_eq!((n.nivel_sin_xp_bandidos, n.ratio_combate_digno), (5, 500_000));
        assert_eq!(n.cap_fundacion, vec![1, 2, 3, 3, 4, 5, 5, 6, 6, 7]);
    }

    #[test]
    fn el_cupo_es_el_mismo_que_lee_la_economia() {
        // El bloque está repetido en Economia_Construccion: los dos tienen que decir lo mismo.
        let r = Reglas::cargar();
        let e = asentamiento::Reglas::cargar();
        for nivel in 1..=10 {
            assert_eq!(Some(r.nivel.cupo_nivel2[nivel - 1]), e.nivel.cupo_faccion(2, nivel as u32));
            assert_eq!(Some(r.nivel.cupo_nivel3[nivel - 1]), e.nivel.cupo_faccion(3, nivel as u32));
        }
    }

    #[test]
    fn la_identidad_son_los_bloques_del_documento() {
        let t = texto_de_parametros();
        for nombre in ["CIUDADANIA", "CAPITAL", "PUERTA", "RELACION_PROPUESTA", "ANEXION", "FUSION", "CATALOGO_SIGILO", "NIVEL_FACCION"] {
            assert!(t.contains(&format!("<!-- parametros:{nombre} -->")), "falta {nombre}");
        }
    }
}
