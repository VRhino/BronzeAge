//! Las cifras de las reglas: el archivo común de datos de juego que sirve el motor (`Bienvenida.mundo.url_datos_de_juego`, `GET /v1/datos`; `08_Consumo_del_Contrato.md`). Son las
//! mismas con las que juega el núcleo. Se baja una vez por versión y lo comparten todos los bots; aquí, solo lo que usan los cerebros, en milésimas.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

use serde_json::Value;

use crate::{acceso, Fallo, Puerto};

pub struct Tropa {
    pub id: String,
    /// El edificio que la recluta (`centroUrbano`, `barracon`...).
    pub edificio: String,
    pub escalon: u32,
    /// El nivel que tiene que tener su edificio (`nivelRequerido`).
    pub nivel_edificio: u32,
    /// La tecnología que la desbloquea.
    pub tecnologia: Option<String>,
    /// El equipo por soldado (`costoEquipo`) y el oro por soldado (`RECLUTAMIENTO_ORO_POR_ESCALON` más los caballos; la milicia del Centro Urbano no paga oro), en unidades.
    pub equipo: Vec<(String, i64)>,
    pub oro: i64,
    /// Los soldados de una escuadra entera (`unidadesPorDefecto`).
    pub unidades: u32,
}

/// Una condición del hito de una tecnología (`TECNOLOGIAS.<id>.hito`, Tecnologia_Aedas §3.1).
#[derive(Debug, Clone, PartialEq)]
pub enum Hito {
    /// Un edificio activo en alguna plaza de la Facción, con al menos ese nivel interno.
    Edificio(String, u32),
    /// Otra tecnología, adoptada.
    Tecnologia(String),
    /// Algo de ese recurso en el almacén de la capital.
    EnCapital(String),
    /// La capital a ese nivel y con ese edificio.
    Capital(u32, String),
    /// Un yacimiento de ese recurso en la zona de alguna plaza.
    Yacimiento(String),
}

/// Una tecnología del catálogo: su Era (`ERAS.<era>.orden`), su hito y los contadores del servidor de su logro.
pub struct Tecnologia {
    pub id: String,
    pub nombre: String,
    pub era: u32,
    pub hito: Vec<Hito>,
    /// Las partes del logro del servidor: el contador, su cifra y si es por plaza viva del mundo (`porPlaza`) o fija (`umbral`).
    pub logro: Vec<(String, i64, bool)>,
}

/// Lo que pide mejorar un edificio a un nivel interno (`niveles.<n>`): el nivel de la plaza y, si lo trae, otro edificio a cierto nivel.
pub struct Mejora {
    pub nivel_plaza: u32,
    pub edificio: Option<(String, u32)>,
}

/// Lo que pide subir a un nivel (`NIVEL_ASENTAMIENTO.requisitos`).
pub struct Requisito {
    /// Los pesants que hacen falta, en milésimas.
    pub pesants: i64,
    /// Los edificios que cuentan; con `minimo`, basta con tantos distintos de la lista (si no, todos).
    pub edificios: Vec<String>,
    pub minimo: Option<usize>,
    /// Los que tienen que estar a un nivel (`edificiosConNivel`: la Granja a 2 para subir al nivel 2).
    pub con_nivel: Vec<(String, u32)>,
}

pub struct Datos {
    /// El coste de la Caravana de Fundación desde un campamento, y desde una plaza, por recurso.
    pub costo_refundacion: Vec<(String, i64)>,
    pub costo_caravana_fundacion: Vec<(String, i64)>,
    /// El poder de un soldado de cada tropa (`poderBase`).
    pub poder: HashMap<String, u32>,
    /// El catálogo de tropas reclutables.
    pub tropas: Vec<Tropa>,
    /// Lo que suma cada héroe al poder de su bando (`Combate.MILITAR.poderPorHeroe`).
    pub poder_por_heroe: u32,
    /// La velocidad de cada tropa, en unidades por minuto.
    pub velocidad: HashMap<String, f64>,
    /// Lo que consume una columna en marcha frente a parada (`factorConsumoEnMarcha / factorConsumoEstacionado`): los víveres que se ven bajar a la puerta, por esto, en marcha.
    pub marcha_sobre_parada: f64,
    pub radio_puerta: i64,
    pub radio_encuentro: i64,
    /// Desde dónde se inspecciona.
    pub radio_inspeccion: i64,
    /// El anillo de bandidos de un campamento.
    pub anillo: (i64, i64),
    pub separacion_plazas: i64,
    pub exclusion_campamento: i64,
    /// Desde qué densidad de bosque (0-255 de la capa del artefacto) no se funda (`fundacion.bosque_denso`).
    pub bosque_denso: u8,
    /// La zona con la que nace una plaza y la que alcanza en el nivel 1.
    pub radio_inicial: i64,
    pub techo_nivel_1: i64,
    /// El nivel de plaza que pide cada edificio para construirlo (`requisitoNivelAsentamientoConstruccion`; sin él, 1).
    pub nivel_para_construir: HashMap<String, u32>,
    /// Los cultivos de aldea con el nivel que alguna plaza de la Facción ha de haber alcanzado para marcarlos (`PUEBLOS_EXTRACTORES.aldea.cultivos`).
    pub cultivos: Vec<(String, u32)>,
    /// La fertilidad mínima de una aldea, de 0 a 255 como los rásteres.
    pub fertilidad_minima: u8,
    /// La separación mínima de una aldea a otro pueblo o a una plaza, en milésimas.
    pub separacion_aldea: i64,
    /// Lo que pide subir a cada nivel.
    pub requisitos: HashMap<u32, Requisito>,
    /// Pueblos extractores: el nivel de la madre que pide y cuántos admite por nivel.
    pub pueblos_nivel_madre: u32,
    pub pueblos_por_nivel: HashMap<u32, usize>,
    /// Avanzadas: el nivel de la madre para la primera (la atalaya) y cuántas admite por nivel de la madre.
    pub avanzada_nivel_madre: u32,
    pub avanzadas_por_nivel: usize,
    /// El precio base de cada bien, en oro por unidad: las materias primas del motor (`PRECIO_BASE`) y, de los manufacturados, lo que cuesta su receta más barata a precio
    /// base, en cadena (como `comercio::precio`). El oro no cotiza.
    pub precios: BTreeMap<String, f64>,
    /// Las tecnologías, por Era; el orden de cada Era por su id; lo que cuesta adoptar una de cada Era (`TARIFA_ADOPCION`, por orden de Era, en unidades).
    pub tecnologias: Vec<Tecnologia>,
    pub eras: HashMap<String, u32>,
    pub tarifas: HashMap<u32, Vec<(String, i64)>>,
    /// La tecnología que pide levantar un edificio (`requiereTecnologia`: las Caballerizas piden la cría caballar).
    pub tecnologia_de_edificio: HashMap<String, String>,
    /// Lo que pide cada nivel interno de cada edificio.
    pub mejoras: HashMap<(String, u32), Mejora>,
    /// El Liderazgo de un héroe sin el suyo propio (`LIDERAZGO.base`) y el que compromete una escuadra por el escalón de su tropa (`costePorEscalon`).
    pub liderazgo: u32,
    pub coste_liderazgo: HashMap<u32, u32>,
    /// Las clases del héroe (`CLASES_HEROE` de Objetos_Del_Heroe), en el orden de los datos: `crear_heroe` pide una.
    pub clases: Vec<String>,
}

/// Los precios de los manufacturados a partir de los de `base` y de las `recetas` (`produce`, lo que consume por unidad): la más barata, resuelta en cadena (un lingote de
/// bronce necesita antes los de cobre y estaño).
pub fn precios_con_derivados(base: &BTreeMap<String, f64>, recetas: &[(String, Vec<(String, f64)>)]) -> BTreeMap<String, f64> {
    let mut precios = base.clone();
    loop {
        let mut cambio = false;
        for (produce, insumos) in recetas.iter().filter(|(p, _)| !base.contains_key(p)) {
            let suma: Option<f64> = insumos.iter().map(|(i, n)| precios.get(i).map(|p| p * n)).sum();
            if let Some(p) = suma.filter(|&p| p > 0.0 && precios.get(produce).is_none_or(|&antes| p < antes - 1e-9)) {
                precios.insert(produce.clone(), p);
                cambio = true;
            }
        }
        if !cambio {
            return precios;
        }
    }
}

impl Datos {
    /// El precio base de `bien`, en oro por unidad (0 si no cotiza).
    pub fn precio(&self, bien: &str) -> f64 {
        self.precios.get(bien).copied().unwrap_or(0.0)
    }

    pub fn desde_json(v: &Value) -> Result<Self, Fallo> {
        let falta = |que: &str| Fallo::Registro(format!("datos de juego sin {que}"));
        let d = &v["documentos"];
        // Los documentos van en unidades del juego; las distancias, a milésimas.
        let mil = |x: &Value, que: &str| x.as_f64().map(|f| (f * 1000.0).round() as i64).ok_or_else(|| falta(que));
        let costo_refundacion = v["derivados"]["costo_refundacion"].as_object().ok_or_else(|| falta("costo_refundacion"))?.iter().filter_map(|(r, n)| Some((r.clone(), n.as_i64()?))).collect();
        let costo_caravana_fundacion =
            v["derivados"]["costo_caravana_fundacion"].as_object().ok_or_else(|| falta("costo_caravana_fundacion"))?.iter().filter_map(|(r, n)| Some((r.clone(), n.as_i64()?))).collect();
        let catalogo = d["Heroe_Y_Tropa"]["TROPAS_RECLUTABLES"].as_array().ok_or_else(|| falta("TROPAS_RECLUTABLES"))?;
        let poder = catalogo.iter().filter_map(|t| Some((t["id"].as_str()?.to_string(), t["poderBase"].as_u64()? as u32))).collect();
        let velocidad = catalogo.iter().filter_map(|t| Some((t["id"].as_str()?.to_string(), t["velocidad"].as_f64()?))).collect();
        let logistica = &d["Movimiento_Ejercitos"]["LOGISTICA"];
        let marcha_sobre_parada = logistica["factorConsumoEnMarcha"].as_f64().unwrap_or(1.0) / logistica["factorConsumoEstacionado"].as_f64().filter(|f| *f > 0.0).unwrap_or(1.0);
        let ht = &d["Heroe_Y_Tropa"];
        let por_caballo = ht["ORO_POR_CABALLO"].as_i64().unwrap_or(0);
        let tropas = catalogo
            .iter()
            .filter_map(|t| {
                let escalon = t["escalon"].as_u64()? as u32;
                let edificio: String = t["edificio"].as_str()?.into();
                let oro = if edificio == "centroUrbano" { 0 } else { ht["RECLUTAMIENTO_ORO_POR_ESCALON"][escalon.to_string()].as_i64().unwrap_or(0) + t["caballos"].as_i64().unwrap_or(0) * por_caballo };
                Some(Tropa {
                    id: t["id"].as_str()?.into(),
                    edificio,
                    escalon,
                    nivel_edificio: t["nivelRequerido"].as_u64().unwrap_or(1) as u32,
                    tecnologia: t["tecnologia"].as_str().map(String::from),
                    equipo: t["costoEquipo"].as_object().into_iter().flatten().filter_map(|(r, n)| Some((r.clone(), n.as_i64()?))).collect(),
                    oro,
                    unidades: t["unidadesPorDefecto"].as_u64().unwrap_or(0) as u32,
                })
            })
            .collect();
        let eco = &d["Economia_Construccion"];
        let nivel_para_construir = eco["EDIFICIO_CATALOGO"]
            .as_object()
            .ok_or_else(|| falta("EDIFICIO_CATALOGO"))?
            .iter()
            .map(|(t, e)| (t.clone(), e["requisitoNivelAsentamientoConstruccion"].as_u64().unwrap_or(1) as u32))
            .collect();
        let requisitos = eco["NIVEL_ASENTAMIENTO"]["requisitos"]
            .as_object()
            .ok_or_else(|| falta("NIVEL_ASENTAMIENTO"))?
            .iter()
            .filter_map(|(n, r)| {
                let edificios = r["edificios"].as_array()?.iter().filter_map(|e| e.as_str().map(String::from)).collect();
                let con_nivel = r["edificiosConNivel"].as_object().into_iter().flatten().filter_map(|(t, n)| Some((t.clone(), n.as_u64()? as u32))).collect();
                let requisito = Requisito { pesants: r["pesants"].as_i64()? * 1000, edificios, minimo: r["edificiosMinimo"].as_u64().map(|m| m as usize), con_nivel };
                Some((n.parse().ok()?, requisito))
            })
            .collect();
        let pueblos = &eco["PUEBLOS_EXTRACTORES"];
        let pueblos_por_nivel = pueblos["cupoPorNivel"].as_object().into_iter().flatten().filter_map(|(n, c)| Some((n.parse().ok()?, c.as_u64()? as usize))).collect();
        let aldea = &pueblos["aldea"];
        let mut cultivos: Vec<(String, u32)> = aldea["cultivos"].as_object().into_iter().flatten().filter_map(|(c, x)| Some((c.clone(), x["nivelMadre"].as_u64()? as u32))).collect();
        cultivos.sort_by_key(|(c, n)| (*n, c.clone()));
        let fertilidad_minima = (aldea["fertilidadMinima"].as_f64().unwrap_or(0.3) * 255.0).ceil() as u8;
        let separacion_aldea = (aldea["separacionMinima"].as_f64().unwrap_or(3.0) * 1000.0) as i64;
        let avanzadas = &eco["AVANZADAS"];
        let base: BTreeMap<String, f64> = d["Comercio_Caravanas"]["PRECIO_BASE"].as_object().into_iter().flatten().filter_map(|(k, v)| Some((k.clone(), v.as_f64()?))).collect();
        let recetas: Vec<(String, Vec<(String, f64)>)> = eco["EDIFICIO_CATALOGO"]
            .as_object()
            .into_iter()
            .flatten()
            .flat_map(|(_, e)| e["niveles"].as_object().into_iter().flatten().flat_map(|(_, n)| n["recetas"].as_array().into_iter().flatten()))
            .filter_map(|r| Some((r["produce"].as_str()?.to_string(), r["consumePorUnidad"].as_object()?.iter().filter_map(|(k, v)| Some((k.clone(), v.as_f64()?))).collect())))
            .collect();
        let precios = precios_con_derivados(&base, &recetas);
        let ta = &d["Tecnologia_Aedas"];
        let eras: HashMap<String, u32> = ta["ERAS"].as_object().into_iter().flatten().filter_map(|(e, x)| Some((e.clone(), x["orden"].as_u64()? as u32))).collect();
        let tarifas = ta["TARIFA_ADOPCION"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(e, x)| Some((*eras.get(e)?, x.as_object()?.iter().filter_map(|(r, n)| Some((r.clone(), n.as_i64()?))).collect())))
            .collect();
        let mut tecnologias: Vec<Tecnologia> = ta["TECNOLOGIAS"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(id, x)| {
                let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
                let hito = x["hito"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|h| {
                        Some(match h["tipo"].as_str()? {
                            "edificio" => Hito::Edificio(s(&h["edificio"]), h["nivelInterno"].as_u64().unwrap_or(1) as u32),
                            "tecnologia" => Hito::Tecnologia(s(&h["id"])),
                            "recursoEnCapital" => Hito::EnCapital(s(&h["recurso"])),
                            "capitalEnNivel" => Hito::Capital(h["nivel"].as_u64().unwrap_or(1) as u32, s(&h["conEdificio"])),
                            "yacimientoEnTerritorio" => Hito::Yacimiento(s(&h["recurso"])),
                            _ => return None,
                        })
                    })
                    .collect();
                let l = &x["logro"];
                let logro = [l, &l["ademas"]]
                    .into_iter()
                    .filter_map(|p| Some((p["contador"].as_str()?.to_string(), p["porPlaza"].as_i64().or(p["umbral"].as_i64())?, p["porPlaza"].is_i64())))
                    .collect();
                Tecnologia { id: id.clone(), nombre: s(&x["nombre"]), era: eras.get(&s(&x["era"])).copied().unwrap_or(1), hito, logro }
            })
            .collect();
        tecnologias.sort_by_key(|t| (t.era, t.id.clone()));
        let catalogo_edificios = eco["EDIFICIO_CATALOGO"].as_object().ok_or_else(|| falta("EDIFICIO_CATALOGO"))?;
        let tecnologia_de_edificio = catalogo_edificios.iter().filter_map(|(t, e)| Some((t.clone(), e["requiereTecnologia"].as_str()?.to_string()))).collect();
        let mejoras = catalogo_edificios
            .iter()
            .flat_map(|(t, e)| {
                e["niveles"].as_object().into_iter().flatten().filter_map(move |(n, x)| {
                    let edificio = x["requiereEdificio"].as_str().map(|r| (r.to_string(), x["requiereEdificioNivel"].as_u64().unwrap_or(1) as u32));
                    Some(((t.clone(), n.parse().ok()?), Mejora { nivel_plaza: x["requisitoNivelAsentamiento"].as_u64().unwrap_or(1) as u32, edificio }))
                })
            })
            .collect();
        let anillo = &d["Generacion_Del_Mundo"]["CAMPAMENTOS_BANDIDOS"]["anillo"];
        let territorio = &v["reglas"]["territorio"];
        Ok(Self {
            costo_refundacion,
            costo_caravana_fundacion,
            poder,
            tropas,
            poder_por_heroe: d["Combate"]["MILITAR"]["poderPorHeroe"].as_u64().unwrap_or(0) as u32,
            velocidad,
            marcha_sobre_parada,
            radio_puerta: mil(&d["Movimiento_Ejercitos"]["MOVIMIENTO"]["radioPuerta"], "radioPuerta")?,
            radio_encuentro: mil(&d["Movimiento_Ejercitos"]["LOGISTICA"]["radioEncuentro"], "radioEncuentro")?,
            radio_inspeccion: mil(&d["Movimiento_Ejercitos"]["MOVIMIENTO"]["radioInspeccion"], "radioInspeccion")?,
            anillo: (mil(&anillo["radioMin"], "anillo")?, mil(&anillo["radioMax"], "anillo")?),
            separacion_plazas: territorio["separacion_plazas"].as_i64().ok_or_else(|| falta("separacion_plazas"))?,
            exclusion_campamento: territorio["exclusion_campamento"].as_i64().ok_or_else(|| falta("exclusion_campamento"))?,
            bosque_denso: (eco["FUNDACION"]["densidadBosqueMaximaParaFundar"].as_f64().unwrap_or(1.0) * 255.0).ceil().min(255.0) as u8,
            radio_inicial: territorio["radio_inicial"].as_i64().ok_or_else(|| falta("radio_inicial"))?,
            techo_nivel_1: territorio["techos"][0].as_i64().ok_or_else(|| falta("techos"))?,
            nivel_para_construir,
            cultivos,
            fertilidad_minima,
            separacion_aldea,
            requisitos,
            pueblos_nivel_madre: pueblos["nivelMadreMinimo"]["extractor"].as_u64().unwrap_or(1) as u32,
            pueblos_por_nivel,
            avanzada_nivel_madre: avanzadas["nivelMadre"]["atalaya"].as_u64().unwrap_or(2) as u32,
            avanzadas_por_nivel: avanzadas["cupoPorNivelDeLaMadre"].as_u64().unwrap_or(1) as usize,
            precios,
            tecnologias,
            eras,
            tarifas,
            tecnologia_de_edificio,
            mejoras,
            liderazgo: ht["LIDERAZGO"]["base"].as_u64().unwrap_or(100) as u32,
            coste_liderazgo: ht["LIDERAZGO"]["costePorEscalon"].as_object().into_iter().flatten().filter_map(|(e, c)| Some((e.parse().ok()?, c.as_u64()? as u32))).collect(),
            clases: d["Objetos_Del_Heroe"]["CLASES_HEROE"]["clases"].as_array().into_iter().flatten().filter_map(|c| Some(c["id"].as_str()?.to_string())).collect(),
        })
    }

    /// El Liderazgo que compromete una escuadra de la tropa (el de su escalón, sin mirar cuántos soldados le quedan; 0 si no está en el catálogo).
    pub fn coste_liderazgo_de(&self, tropa: &str) -> u32 {
        self.tropas.iter().find(|t| t.id == tropa).and_then(|t| self.coste_liderazgo.get(&t.escalon)).copied().unwrap_or(0)
    }

    /// El poder de una escuadra: sus soldados por el poder de su tropa.
    /// La clase del `n`-ésimo héroe: se reparten por turno (vacía si los datos no traen clases; el servidor la rechaza).
    pub fn clase(&self, n: usize) -> String {
        self.clases.get(n % self.clases.len().max(1)).cloned().unwrap_or_default()
    }

    pub fn poder_de(&self, e: &contrato::v1::Escuadron) -> u32 {
        e.cantidad * self.poder.get(&e.tropa_id).copied().unwrap_or(0)
    }
}

/// Los datos de juego del servidor de `puerto`: se bajan la primera vez y se guardan por versión.
pub async fn de(puerto: &Puerto) -> Result<Arc<Datos>, Fallo> {
    static DATOS: Mutex<Option<HashMap<String, Arc<Datos>>>> = Mutex::new(None);
    let info = puerto.bienvenida.mundo.clone().unwrap_or_default();
    if let Some(d) = DATOS.lock().unwrap().get_or_insert_with(HashMap::new).get(&info.version_datos_de_juego) {
        return Ok(d.clone());
    }
    let ruta = if info.url_datos_de_juego.is_empty() { "/v1/datos".to_string() } else { info.url_datos_de_juego };
    let v: Value = serde_json::from_slice(&acceso::get(puerto.servidor(), &ruta).await?).map_err(|e| Fallo::Registro(format!("datos de juego: {e}")))?;
    let d = Arc::new(Datos::desde_json(&v)?);
    DATOS.lock().unwrap().get_or_insert_with(HashMap::new).insert(info.version_datos_de_juego, d.clone());
    Ok(d)
}

#[cfg(test)]
mod precios {
    use super::*;

    #[test]
    fn el_bronce_cotiza_por_su_receta_en_cadena_y_la_mas_barata_gana() {
        let base: BTreeMap<String, f64> = [("cobre", 3.0), ("estano", 6.0), ("madera", 1.0)].into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        let r = |p: &str, i: &[(&str, f64)]| (p.to_string(), i.iter().map(|(k, n)| (k.to_string(), *n)).collect::<Vec<_>>());
        let recetas = vec![
            r("lingoteBronce", &[("lingoteCobre", 1.6), ("lingoteEstano", 0.4)]),
            r("lingoteCobre", &[("cobre", 2.0)]),
            r("lingoteEstano", &[("estano", 5.0)]),
            r("armaMadera", &[("madera", 3.0)]),
            r("armaMadera", &[("madera", 2.0)]),
        ];
        let p = precios_con_derivados(&base, &recetas);
        assert_eq!((p["lingoteCobre"], p["lingoteEstano"], p["armaMadera"]), (6.0, 30.0, 2.0));
        assert!((p["lingoteBronce"] - 21.6).abs() < 1e-9, "como el motor: 1,6 × 6 + 0,4 × 30");
    }
}
