//! Las cifras del sistema. Fuente única: los bloques `<!-- parametros:NOMBRE -->` de `Docs/Mechanics/Economia_Construccion.md`,
//! que se incrustan en el binario y se leen al arrancar. Cambiar una cifra es cambiar el documento; no hay copia que se desincronice.
//!
//! Los decimales de los datos (0,12, 1,5…) se convierten una sola vez, al cargar, a enteros: **ppm** (millonésimas) para tasas y
//! factores, **milésimas** (`Mil`) para cantidades. Después todo es aritmética entera.

use base::Mil;
use serde_json::Value;
use std::collections::BTreeMap;

const ECONOMIA: &str = include_str!("../../../Docs/Mechanics/Economia_Construccion.md");

/// 1,0 en ppm.
pub const UNO: i64 = 1_000_000;

/// El JSON del bloque `parametros:nombre` de un documento.
pub fn bloque_de(doc: &str, nombre: &str) -> Result<Value, String> {
    let marca = format!("<!-- parametros:{nombre} -->");
    let ini = doc.find(&marca).ok_or_else(|| format!("no hay bloque «{nombre}»"))? + marca.len();
    let resto = &doc[ini..];
    let fin = resto.find("<!-- /parametros -->").ok_or_else(|| format!("el bloque «{nombre}» no se cierra"))?;
    let cuerpo = resto[..fin].trim();
    let cuerpo = cuerpo.strip_prefix("```json").and_then(|c| c.strip_suffix("```")).ok_or_else(|| format!("el bloque «{nombre}» no es json"))?;
    serde_json::from_str(cuerpo).map_err(|e| format!("bloque «{nombre}»: {e}"))
}

/// Un bloque de Economia_Construccion. Los datos están compilados dentro: si falta o no es json es un error de programación.
pub fn bloque(nombre: &str) -> Value {
    bloque_de(ECONOMIA, nombre).unwrap_or_else(|e| panic!("{e}"))
}

/// Decimal a ppm (redondeado).
pub fn ppm(x: f64) -> i64 {
    (x * UNO as f64).round() as i64
}

/// Decimal a milésimas (redondeado).
pub fn mil(x: f64) -> Mil {
    (x * 1000.0).round() as Mil
}

fn num(v: &Value, ruta: &str) -> f64 {
    ruta.split('.').try_fold(v, |v, k| v.get(k)).and_then(Value::as_f64).unwrap_or_else(|| panic!("falta el número «{ruta}»"))
}

/// Producto de ppm: `a × b / 1,0`, redondeado al más cercano (las mitades se alejan de cero), para que los errores de
/// redondeo de una cadena de factores no se acumulen siempre hacia el mismo lado.
pub fn por(a: i64, b: i64) -> i64 {
    let p = a as i128 * b as i128;
    ((p + p.signum() * (UNO as i128 / 2)) / UNO as i128) as i64
}

/// `a / b` en ppm (`a × 1,0 / b`), redondeado al más cercano; `b > 0`.
pub fn razon(a: i64, b: i64) -> i64 {
    let n = a as i128 * UNO as i128;
    ((n + n.signum() * (b as i128 / 2)) / b as i128) as i64
}

#[derive(Clone, Debug)]
pub struct Hambre {
    /// 0..100 en milésimas.
    pub nutricion_inicial: Mil,
    pub regeneracion_por_minuto: Mil,
    pub degradacion_sin_comida: Mil,
    pub factor_crecimiento_minimo: i64,
    pub umbral_muerte: Mil,
    /// Fracción de pesants y artesanos que muere por minuto con hambre sostenida (ppm).
    pub muerte_por_minuto: i64,
    pub muerte_nobleza_por_minuto: i64,
}

#[derive(Clone, Debug)]
pub struct ReglasPoblacion {
    pub pesants_inicial: Mil,
    pub tasa_pesants: i64,
    pub tasa_artesanos: i64,
    pub tasa_nobleza: i64,
    pub min_ciudadanos_nobleza: u32,
    /// Trigo por habitante y minuto (ppm de unidad).
    pub consumo_por_habitante: i64,
    pub hambre: Hambre,
    /// Oro por habitante y minuto (ppm), por clase.
    pub impuesto_pesants: i64,
    pub impuesto_artesanos: i64,
    pub impuesto_nobleza: i64,
}

#[derive(Clone, Debug)]
pub struct Politica {
    pub id: String,
    pub cargo: String,
    pub nombre: String,
    /// Factores multiplicativos (`factorConsumoComida`…), en ppm.
    pub factores: BTreeMap<String, i64>,
    /// Aditivos (`cupoCaravanaExtra`…).
    pub sumas: BTreeMap<String, i64>,
    pub perfil_trazado: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SlotsCargo {
    pub base: u32,
    pub maximo: u32,
}

#[derive(Clone, Debug)]
pub struct ReglasPoliticas {
    pub slots: BTreeMap<String, SlotsCargo>,
    pub nivel_faccion_por_slot_extra_gobernador: u32,
    pub slot_sala_consejo: u32,
    pub duracion_minutos: i64,
    pub catalogo: Vec<Politica>,
}

#[derive(Clone, Debug)]
pub struct Reglas {
    pub poblacion: ReglasPoblacion,
    pub politicas: ReglasPoliticas,
    /// Capacidad inicial de cada recurso en un asentamiento nuevo, en milésimas.
    pub capacidad_inicial: Mil,
    pub catalogo: crate::catalogo::Catalogo,
    pub construccion: crate::catalogo::ReglasConstruccion,
    pub mantenimiento: crate::mantenimiento::ReglasMantenimiento,
    pub nivel: crate::nivel::ReglasNivel,
}

impl Reglas {
    pub fn cargar() -> Self {
        Self::de(&|n| bloque(n))
    }

    fn de(b: &dyn Fn(&str) -> Value) -> Self {
        let (p, i, h) = (b("POBLACION"), b("IMPUESTOS"), b("POBLACION"));
        let hambre = Hambre {
            nutricion_inicial: mil(num(&h, "hambre.nutricionInicial")),
            regeneracion_por_minuto: mil(num(&h, "hambre.regeneracionPorMinuto")),
            degradacion_sin_comida: mil(num(&h, "hambre.degradacionSinComida")),
            factor_crecimiento_minimo: ppm(num(&h, "hambre.factorCrecimientoMinimo")),
            umbral_muerte: mil(num(&h, "hambre.umbralMuertePorHambre")),
            muerte_por_minuto: ppm(num(&h, "hambre.fraccionMuertePorMinutoHambre")),
            muerte_nobleza_por_minuto: ppm(num(&h, "hambre.fraccionMuertePorMinutoHambreNobleza")),
        };
        let poblacion = ReglasPoblacion {
            pesants_inicial: mil(num(&p, "pesants.inicial")),
            tasa_pesants: ppm(num(&p, "pesants.tasaCrecimientoBase")),
            tasa_artesanos: ppm(num(&p, "artesanos.tasaCrecimientoBase")),
            tasa_nobleza: ppm(num(&p, "nobleza.tasaCrecimientoBase")),
            min_ciudadanos_nobleza: num(&p, "nobleza.minCiudadanos") as u32,
            consumo_por_habitante: ppm(num(&p, "consumoComidaPorHabitante")),
            hambre,
            impuesto_pesants: ppm(num(&i, "tasaPesants")),
            impuesto_artesanos: ppm(num(&i, "tasaArtesanos")),
            impuesto_nobleza: ppm(num(&i, "tasaNobleza")),
        };

        let pol = b("POLITICAS");
        let slots = pol["slotsPorCargo"]
            .as_object()
            .expect("slotsPorCargo")
            .iter()
            .map(|(k, v)| (k.clone(), SlotsCargo { base: num(v, "base") as u32, maximo: num(v, "maximo") as u32 }))
            .collect();
        let catalogo = b("POLITICA_CATALOGO")
            .as_array()
            .expect("POLITICA_CATALOGO es una lista")
            .iter()
            .map(|d| {
                let (mut factores, mut sumas) = (BTreeMap::new(), BTreeMap::new());
                for (k, v) in d.as_object().expect("política") {
                    match (k.as_str(), v.as_f64()) {
                        (c, Some(x)) if c.starts_with("factor") => drop(factores.insert(c.to_string(), ppm(x))),
                        (c, Some(x)) if c.starts_with("cupo") => drop(sumas.insert(c.to_string(), x as i64)),
                        _ => {}
                    }
                }
                let texto = |k: &str| d.get(k).and_then(Value::as_str).map(str::to_string);
                Politica { id: texto("id").expect("id"), cargo: texto("cargo").expect("cargo"), nombre: texto("nombre").expect("nombre"), factores, sumas, perfil_trazado: texto("perfilTrazado") }
            })
            .collect();
        let politicas = ReglasPoliticas {
            slots,
            nivel_faccion_por_slot_extra_gobernador: num(&pol, "nivelFaccionPorSlotExtraGobernador") as u32,
            slot_sala_consejo: num(&pol, "slotSalaConsejo") as u32,
            duracion_minutos: num(&pol, "duracionMinutosPorDefecto") as i64,
            catalogo,
        };
        Reglas {
            poblacion,
            politicas,
            capacidad_inicial: mil(num(&b("ALMACEN"), "capacidadInicialPorRecurso")),
            catalogo: crate::catalogo::Catalogo::de(&b("EDIFICIO_CATALOGO")),
            construccion: crate::catalogo::ReglasConstruccion::de(b),
            mantenimiento: crate::mantenimiento::ReglasMantenimiento::de(&b("MANTENIMIENTO")),
            nivel: crate::nivel::ReglasNivel::de(&b("NIVEL_ASENTAMIENTO"), &b("ASCENSO_ASENTAMIENTO"), &b("CUPO_NIVEL_ASENTAMIENTO")),
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn los_bloques_se_leen_del_documento() {
        let doc = "x\n<!-- parametros:A -->\n```json\n{\"n\": 2.5}\n```\n<!-- /parametros -->\ny";
        assert_eq!(bloque_de(doc, "A").unwrap()["n"], 2.5);
        assert!(bloque_de(doc, "B").is_err());
        assert!(bloque_de("<!-- parametros:A -->\n```json\n{", "A").is_err());
    }

    #[test]
    fn las_reglas_cargan_con_las_cifras_de_mechanics() {
        let r = Reglas::cargar();
        let p = &r.poblacion;
        assert_eq!((p.pesants_inicial, p.tasa_pesants, p.tasa_artesanos, p.tasa_nobleza), (20_000, 120_000, 150_000, 10_000));
        assert_eq!((p.consumo_por_habitante, p.min_ciudadanos_nobleza), (100_000, 3));
        assert_eq!((p.hambre.nutricion_inicial, p.hambre.regeneracion_por_minuto, p.hambre.degradacion_sin_comida), (100_000, 5_000, 20_000));
        assert_eq!((p.hambre.factor_crecimiento_minimo, p.hambre.muerte_por_minuto, p.hambre.muerte_nobleza_por_minuto), (200_000, 50_000, 10_000));
        assert_eq!((p.impuesto_pesants, p.impuesto_artesanos, p.impuesto_nobleza), (4_000, 15_000, 60_000));
        assert_eq!(r.capacidad_inicial, 400_000);
        assert_eq!(r.politicas.catalogo.len(), 16);
        assert_eq!(r.politicas.slots["gobernador"].maximo, 5);
        let racion = r.politicas.catalogo.iter().find(|p| p.id == "racionamiento").unwrap();
        assert_eq!(racion.factores["factorConsumoComida"], 800_000);
        let flota = r.politicas.catalogo.iter().find(|p| p.id == "cupo_caravana_extra").unwrap();
        assert_eq!(flota.sumas["cupoCaravanaExtra"], 1);
    }

    #[test]
    fn producto_de_ppm() {
        assert_eq!(por(UNO, UNO), UNO);
        assert_eq!(por(800_000, 1_500_000), 1_200_000);
        assert_eq!(por(3, 500_000), 2, "1,5 redondea a 2");
        assert_eq!(por(-3, 500_000), -2);
        assert_eq!(razon(1, 3), 333_333);
        assert_eq!(razon(2, 3), 666_667);
    }
}
