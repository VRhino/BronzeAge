//! Las cifras del sistema militar. Fuente única: los bloques `<!-- parametros:NOMBRE -->` de `Docs/Mechanics/{Heroe_Y_Tropa, Movimiento_Ejercitos,
//! Combate, Presencia_Y_Convocatoria, Campamentos_Mercenarios, Niebla_De_Guerra}.md`, incrustados en el binario. Los decimales pasan una sola vez a enteros:
//! **ppm** para tasas y factores, **milésimas** (`Mil`) para cantidades, distancias y velocidades.
//!
//! **Los bloques son los valores vigentes** (decisión del autor, 2026-10-19): a diferencia de lo que ocurría en el comercio, no hay
//! correcciones en el código. Las velocidades de tropa son las calibradas (×1,5), el radio de encuentro es 3, la puerta 2, la visión de una
//! columna 30 y 15, el catálogo trae a los Mineros y el poder de un héroe en un combate con números es 15.

use crate::catalogo::Catalogo;
use asentamiento::datos::{bloque_de, mil, ppm, texto_de_parametros_de};
use asentamiento::Tipo;
use base::{Mil, MS_POR_MINUTO};
use serde_json::Value;

pub const HEROE_Y_TROPA: &str = include_str!("../../../Docs/Mechanics/Heroe_Y_Tropa.md");
pub const MOVIMIENTO: &str = include_str!("../../../Docs/Mechanics/Movimiento_Ejercitos.md");
pub const COMBATE: &str = include_str!("../../../Docs/Mechanics/Combate.md");
pub const PRESENCIA: &str = include_str!("../../../Docs/Mechanics/Presencia_Y_Convocatoria.md");
pub const CAMPAMENTOS: &str = include_str!("../../../Docs/Mechanics/Campamentos_Mercenarios.md");
pub const NIEBLA: &str = include_str!("../../../Docs/Mechanics/Niebla_De_Guerra.md");
pub const TECNOLOGIA: &str = include_str!("../../../Docs/Mechanics/Tecnologia_Aedas.md");

/// Los bloques de parámetros de los documentos, para la identidad de las reglas (hash del núcleo).
pub fn texto_de_parametros() -> String {
    [HEROE_Y_TROPA, MOVIMIENTO, COMBATE, PRESENCIA, CAMPAMENTOS, NIEBLA, TECNOLOGIA].iter().map(|d| texto_de_parametros_de(d)).collect()
}

pub fn bloque(doc: &str, nombre: &str) -> Value {
    bloque_de(doc, nombre).unwrap_or_else(|e| panic!("{e}"))
}

pub fn num(v: &Value, ruta: &str) -> f64 {
    ruta.split('.').try_fold(v, |v, k| v.get(k)).and_then(Value::as_f64).unwrap_or_else(|| panic!("falta el número «{ruta}»"))
}

pub fn entero(v: &Value, ruta: &str) -> i64 {
    num(v, ruta) as i64
}

pub fn lista(v: &Value, ruta: &str) -> Vec<f64> {
    ruta.split('.').try_fold(v, |v, k| v.get(k)).and_then(Value::as_array).unwrap_or_else(|| panic!("falta la lista «{ruta}»")).iter().map(|x| x.as_f64().expect("número")).collect()
}

/// Cinco valores, uno por escalón (las claves `"1"`…`"5"`).
fn por_escalon(v: &Value) -> [f64; 5] {
    std::array::from_fn(|i| num(v, &(i + 1).to_string()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasLiderazgo {
    /// Liderazgo de un héroe que no lo ha subido (`liderazgoBase`).
    pub base: i64,
    /// Coste de una escuadra por escalón (1 a 5).
    pub coste_por_escalon: [i64; 5],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasHeroe {
    /// Bronce con el que nace (`HEROE.bronceInicial`).
    pub bronce_inicial: i64,
    pub tope_atributo: i64,
    /// Cuánto dura estar herido (ms de mundo).
    pub herido: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasGuarnicion {
    /// Cupo de Barracón y de Galería de tiro por nivel interno (1, 2, 3).
    pub por_nivel_edificio: Vec<i64>,
    pub por_nivel_caballerizas: Vec<i64>,
    /// Cupo del mejor recinto completo por nivel (empalizada, piedra, adarve).
    pub recinto_completo_por_nivel: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasReclutamiento {
    /// Oro por soldado y escalón (1 a 5), en milésimas.
    pub oro_por_escalon: [Mil; 5],
    pub oro_por_caballo: Mil,
    /// Minutos de ración que debe cubrir el trigo del almacén al reclutar (`RESERVA_CONSTRUCCION.horizonteMinutosComida`).
    pub horizonte_comida: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasLogistica {
    /// El carro de cada héroe, en milésimas de unidad de carga.
    pub capacidad_carro: Mil,
    /// Los víveres que lleva cada héroe.
    pub capacidad_viveres: Mil,
    /// Factor de ración de una columna en marcha y acampada (ppm); acampada es el de marcha por el de acampar.
    pub factor_marcha: i64,
    pub factor_estacionado: i64,
    /// Distancia de encuentro y de unión en campo.
    pub radio_encuentro: Mil,
    /// Solo se narra repostar si los víveres iban por debajo de esta parte de la capacidad (ppm).
    pub umbral_narrar_reposte: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasMovimiento {
    /// Velocidad de un héroe solo, sin tropa ni caravana.
    pub velocidad_jugador: Mil,
    /// Trigo por participante y minuto, a ración entera (milésimas).
    pub consumo_por_participante: Mil,
    /// A cuánto de una puerta se entra, se recluta o se repone una columna.
    pub radio_puerta: Mil,
    /// Cuánto vive una petición de unión (ms).
    pub vida_peticion_union: i64,
    /// A cuánto se inspecciona una columna, una caravana o una plaza ajena (`MOVIMIENTO.radioInspeccion`).
    pub radio_inspeccion: Mil,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasFormacion {
    /// Héroes que hacen falta para que una formación sea un ejército.
    pub minimo: usize,
    /// Cuánto espera una formación (ms).
    pub plazo: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasVision {
    /// Alcance de vista de una columna con soldados y de un héroe solo.
    pub ejercito: Mil,
    pub jugador_solo: Mil,
    /// Lado de la celda de la rejilla de lo explorado (`EXPLORACION.tamanoCelda`, Niebla_De_Guerra §7).
    pub tamano_celda: Mil,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasMilitar {
    /// Trigo por soldado y minuto a ración entera (milésimas).
    pub racion_por_soldado: Mil,
    /// Moral que sube cada minuto con ración y que baja sin ella (milésimas de punto de moral).
    pub regeneracion_moral: Mil,
    pub degradacion_moral: Mil,
    /// Fracción de la escuadra que deserta cada minuto con la moral a cero (ppm).
    pub desercion: i64,
    /// Poder extra por nivel de escuadra (ppm).
    pub bonus_poder_por_nivel: i64,
    pub nivel_maximo: u8,
    /// Experiencia que pide cada nivel para subir al siguiente (milésimas de punto).
    pub experiencia_para_subir: Vec<Mil>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasInspeccion {
    /// Cuánto se guarda lo que se vio al inspeccionar (ms): `INSPECCION.informeMinutos`.
    pub informe: i64,
}

/// La intel de las tabernas (`INTEL`, Niebla_De_Guerra §6-§7), en enteros: distancias y oro en milésimas, plazos en ms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasIntel {
    /// Alcance de una Mirada (y radio de su enfriamiento por zona).
    pub radio_mirada: Mil,
    /// Cuánto dura una Mirada abierta, y cuánto se enfría su zona después.
    pub duracion_mirada: i64,
    pub enfriamiento_mirada: i64,
    /// Precio de una Mirada: `oroBase` más `oroPorUnidad` por unidad de distancia a los ojos propios (milésimas de oro por unidad).
    pub oro_base: Mil,
    pub oro_por_unidad: Mil,
    /// Precio de un Informe de plaza por nivel de la plaza, y cuánto hay que esperar para pedir otro de la misma plaza.
    pub oro_por_nivel: Mil,
    pub enfriamiento_informe: i64,
    /// Miradas abiertas a la vez que admite la taberna de un campamento, por Facción.
    pub cupo_campamento: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasPresencia {
    /// Cuánto tarda en salir del mundo quien se desconecta (ms).
    pub retardo_desconexion: i64,
    /// Hasta cuándo se puede aplazar esa salida si le persiguen (ms desde que lo pidió).
    pub tope_aplazamiento: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasMercenarios {
    /// Multiplicador del precio de un campamento (ppm).
    pub recargo: i64,
    /// Multiplicador de la Facción sin asentamientos (ppm).
    pub descuento_sin_asentamientos: i64,
    /// Nivel interno de los edificios militares de un campamento.
    pub nivel_edificios: u8,
    /// Oro de cada unidad de equipo (milésimas), porque un campamento cobra todo en oro.
    pub valor_equipo_en_oro: Mil,
    pub poblacion_por_vivienda: i64,
    pub poblacion_por_hora: i64,
    /// Edificios militares entre los que la posición elige el de cada campamento (`edificiosMilitares`, en ese orden).
    pub edificios_militares: Vec<Tipo>,
    /// Viviendas de un campamento (las de `edificiosFijos`).
    pub viviendas: usize,
    /// A cuánto de un campamento nadie inicia un combate, ni desde allí ni contra lo que esté allí (`radioProteccion`, Combate §2).
    pub radio_proteccion: Mil,
    /// Parte de las Facciones vivas que debe tener adoptada una tecnología para que el campamento la desbloquee (ppm).
    pub porcentaje_facciones: i64,
    /// Cada cuánto da el campamento la ración gratis a un residente que sale de él (ms): llena sus víveres si pasó ese plazo desde la última.
    pub racion_cada: i64,
}

/// Las tecnologías del juego (el bloque `TECNOLOGIAS`): hasta el sistema 6 solo interesa cuáles existen y cuáles tiene toda Facción desde que
/// nace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasTecnologias {
    pub todas: Vec<String>,
    pub de_arranque: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reglas {
    pub tropas: Catalogo,
    pub liderazgo: ReglasLiderazgo,
    pub heroe: ReglasHeroe,
    pub guarnicion: ReglasGuarnicion,
    pub reclutamiento: ReglasReclutamiento,
    pub logistica: ReglasLogistica,
    pub movimiento: ReglasMovimiento,
    pub formacion: ReglasFormacion,
    pub vision: ReglasVision,
    pub militar: ReglasMilitar,
    pub presencia: ReglasPresencia,
    pub inspeccion: ReglasInspeccion,
    pub intel: ReglasIntel,
    pub mercenarios: ReglasMercenarios,
    pub tecnologias: ReglasTecnologias,
}

impl Reglas {
    pub fn cargar() -> Self {
        let (l, h, g) = (bloque(HEROE_Y_TROPA, "LIDERAZGO"), bloque(HEROE_Y_TROPA, "HEROE"), bloque(HEROE_Y_TROPA, "GUARNICION"));
        let (oro, lg, mv) = (bloque(HEROE_Y_TROPA, "RECLUTAMIENTO_ORO_POR_ESCALON"), bloque(MOVIMIENTO, "LOGISTICA"), bloque(MOVIMIENTO, "MOVIMIENTO"));
        let (fm, vs, mi) = (bloque(MOVIMIENTO, "FORMACION_EJERCITO"), bloque(MOVIMIENTO, "VISION"), bloque(COMBATE, "MILITAR"));
        let (pr, me) = (bloque(PRESENCIA, "PRESENCIA"), bloque(CAMPAMENTOS, "MERCENARIOS"));
        let enteros = |v: Vec<f64>| v.into_iter().map(|x| x as i64).collect::<Vec<_>>();
        Reglas {
            tropas: Catalogo::de(&bloque(HEROE_Y_TROPA, "TROPAS_RECLUTABLES")),
            liderazgo: ReglasLiderazgo { base: entero(&l, "base"), coste_por_escalon: por_escalon(&l["costePorEscalon"]).map(|x| x as i64) },
            heroe: ReglasHeroe { bronce_inicial: entero(&h, "bronceInicial"), tope_atributo: entero(&h, "topeAtributo"), herido: entero(&h, "heridoMinutos") * MS_POR_MINUTO },
            guarnicion: ReglasGuarnicion {
                por_nivel_edificio: enteros(lista(&g, "cupoPorNivelEdificio")),
                por_nivel_caballerizas: enteros(lista(&g, "cupoPorNivelCaballerizas")),
                recinto_completo_por_nivel: enteros(lista(&g, "recintoCompletoPorNivel")),
            },
            reclutamiento: ReglasReclutamiento {
                oro_por_escalon: por_escalon(&oro).map(mil),
                oro_por_caballo: mil(bloque(HEROE_Y_TROPA, "ORO_POR_CABALLO").as_f64().expect("ORO_POR_CABALLO")),
                horizonte_comida: entero(&bloque(HEROE_Y_TROPA, "RESERVA_CONSTRUCCION"), "horizonteMinutosComida"),
            },
            logistica: ReglasLogistica {
                capacidad_carro: mil(num(&lg, "capacidadCarroPorJugador")),
                capacidad_viveres: mil(num(&lg, "capacidadViveresPorHeroe")),
                factor_marcha: ppm(num(&lg, "factorConsumoEnMarcha")),
                factor_estacionado: ppm(num(&lg, "factorConsumoEstacionado")),
                radio_encuentro: mil(num(&lg, "radioEncuentro")),
                umbral_narrar_reposte: ppm(num(&lg, "umbralNarrarReposte")),
            },
            movimiento: ReglasMovimiento {
                velocidad_jugador: mil(num(&mv, "velocidadJugador")),
                consumo_por_participante: mil(num(&mv, "consumoPorParticipante")),
                radio_puerta: mil(num(&mv, "radioPuerta")),
                vida_peticion_union: entero(&mv, "vidaPeticionUnionSegundos") * 1000,
                radio_inspeccion: mil(num(&mv, "radioInspeccion")),
            },
            formacion: ReglasFormacion { minimo: entero(&fm, "minimo") as usize, plazo: entero(&fm, "plazoMinutos") * MS_POR_MINUTO },
            vision: ReglasVision { ejercito: mil(num(&vs, "ejercito")), jugador_solo: mil(num(&vs, "jugadorSolo")), tamano_celda: mil(num(&bloque(NIEBLA, "EXPLORACION"), "tamanoCelda")) },
            militar: ReglasMilitar {
                racion_por_soldado: mil(num(&mi, "racionPorSoldadoPorMinuto")),
                regeneracion_moral: mil(num(&mi, "regeneracionMoralPorMinuto")),
                degradacion_moral: mil(num(&mi, "degradacionMoralSinRacion")),
                desercion: ppm(num(&mi, "desercionFraccionPorMinutoSinMoral")),
                bonus_poder_por_nivel: ppm(num(&mi, "bonusPoderPorNivelEscuadra")),
                nivel_maximo: entero(&mi, "nivelMaximoEscuadra") as u8,
                experiencia_para_subir: lista(&mi, "experienciaParaSubirEscuadra").into_iter().map(mil).collect(),
            },
            presencia: ReglasPresencia { retardo_desconexion: entero(&pr, "retardoDesconexionMs"), tope_aplazamiento: entero(&pr, "topeAplazamientoMs") },
            inspeccion: ReglasInspeccion { informe: entero(&bloque(NIEBLA, "INSPECCION"), "informeMinutos") * MS_POR_MINUTO },
            intel: {
                let i = bloque(NIEBLA, "INTEL");
                ReglasIntel {
                    radio_mirada: mil(num(&i, "mirada.radio")),
                    duracion_mirada: entero(&i, "mirada.duracionMinutos") * MS_POR_MINUTO,
                    enfriamiento_mirada: entero(&i, "mirada.cooldownMinutos") * MS_POR_MINUTO,
                    oro_base: mil(num(&i, "mirada.oroBase")),
                    oro_por_unidad: mil(num(&i, "mirada.oroPorUnidad")),
                    oro_por_nivel: mil(num(&i, "informe.oroPorNivel")),
                    enfriamiento_informe: entero(&i, "informe.cooldownMinutos") * MS_POR_MINUTO,
                    cupo_campamento: entero(&i, "campamento.cupoMiradas") as u32,
                }
            },
            mercenarios: ReglasMercenarios {
                recargo: ppm(num(&me, "recargo")),
                descuento_sin_asentamientos: ppm(num(&me, "descuentoSinAsentamientos")),
                nivel_edificios: entero(&me, "nivelEdificios") as u8,
                valor_equipo_en_oro: mil(num(&me, "valorEquipoEnOro")),
                poblacion_por_vivienda: entero(&me, "poblacionPorVivienda"),
                poblacion_por_hora: entero(&me, "poblacionPorHora"),
                edificios_militares: nombres(&me, "edificiosMilitares").iter().map(|n| Tipo::desde_nombre(n).unwrap_or_else(|| panic!("edificio «{n}» desconocido"))).collect(),
                viviendas: nombres(&me, "edificiosFijos").iter().filter(|n| *n == "vivienda").count(),
                radio_proteccion: mil(num(&me, "radioProteccion")),
                porcentaje_facciones: ppm(num(&me, "tecnologia.porcentajeFacciones")),
                racion_cada: entero(&me, "racion.cadaMinutos") * MS_POR_MINUTO,
            },
            tecnologias: {
                let t = bloque(TECNOLOGIA, "TECNOLOGIAS");
                let todas = t.as_object().expect("TECNOLOGIAS es un objeto");
                ReglasTecnologias { todas: todas.keys().cloned().collect(), de_arranque: todas.iter().filter(|(_, v)| v["deArranque"].as_bool() == Some(true)).map(|(k, _)| k.clone()).collect() }
            },
        }
    }
}

/// Una lista de textos del bloque.
fn nombres(v: &Value, ruta: &str) -> Vec<String> {
    v.get(ruta).and_then(Value::as_array).unwrap_or_else(|| panic!("falta la lista «{ruta}»")).iter().map(|x| x.as_str().expect("texto").to_owned()).collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::collections::BTreeMap;

    /// Los bloques con el mismo nombre en dos documentos (`LIDERAZGO` está en Heroe_Y_Tropa y en Movimiento_Ejercitos…) deben decir lo mismo.
    fn bloques_de(doc: &str) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        let mut resto = doc;
        while let Some(i) = resto.find("<!-- parametros:") {
            let nombre = resto[i + 16..].split(" -->").next().unwrap().to_owned();
            if let Ok(b) = bloque_de(doc, &nombre) {
                m.insert(nombre, b);
            }
            resto = &resto[i + 16..];
        }
        m
    }

    #[test]
    fn los_bloques_repetidos_entre_documentos_coinciden() {
        const BATALLA: &str = include_str!("../../../Docs/Mechanics/Batalla_Unity_Ciclo.md");
        let docs = [HEROE_Y_TROPA, MOVIMIENTO, COMBATE, PRESENCIA, CAMPAMENTOS, NIEBLA, TECNOLOGIA, BATALLA, include_str!("../../../Docs/Mechanics/Reloj_Y_Eventos.md")];
        let todos: Vec<_> = docs.iter().map(|d| bloques_de(d)).collect();
        let mut repetidos = 0;
        for (i, a) in todos.iter().enumerate() {
            for b in &todos[i + 1..] {
                for (nombre, va) in a {
                    if let Some(vb) = b.get(nombre) {
                        repetidos += 1;
                        assert_eq!(va, vb, "el bloque «{nombre}» dice cosas distintas en dos documentos");
                    }
                }
            }
        }
        assert!(repetidos >= 6, "LIDERAZGO, HEROE, CHATARRA, VISION, PRESENCIA y MERCENARIOS están en dos documentos ({repetidos})");
    }

    #[test]
    fn las_cifras_vigentes_son_las_de_los_bloques_sin_correcciones() {
        let r = Reglas::cargar();
        assert_eq!((r.liderazgo.base, r.liderazgo.coste_por_escalon), (100, [7, 14, 22, 32, 45]));
        assert_eq!((r.heroe.bronce_inicial, r.heroe.tope_atributo, r.heroe.herido), (500, 100, 2 * MS_POR_MINUTO));
        assert_eq!(r.guarnicion.por_nivel_edificio, vec![7, 14, 22]);
        assert_eq!(r.guarnicion.por_nivel_caballerizas, vec![0, 14, 32]);
        assert_eq!(r.guarnicion.recinto_completo_por_nivel, vec![10, 15, 20]);
        assert_eq!(r.reclutamiento.oro_por_escalon, [1_000, 2_000, 4_000, 7_000, 11_000]);
        assert_eq!((r.reclutamiento.oro_por_caballo, r.reclutamiento.horizonte_comida), (5_000, 8));
        // Calibración de distancias (Movimiento_Ejercitos §14): los bloques ya traen lo vigente.
        assert_eq!((r.logistica.capacidad_carro, r.logistica.capacidad_viveres, r.logistica.radio_encuentro), (500_000, 350_000, 3_000));
        assert_eq!(r.logistica.umbral_narrar_reposte, 500_000);
        assert_eq!((r.logistica.factor_marcha, r.logistica.factor_estacionado), (250_000, 100_000));
        assert_eq!((r.movimiento.velocidad_jugador, r.movimiento.consumo_por_participante, r.movimiento.radio_puerta), (33_000, 500, 2_000));
        assert_eq!((r.movimiento.vida_peticion_union, r.movimiento.radio_inspeccion, r.inspeccion.informe), (10_000, 8_000, 10 * MS_POR_MINUTO));
        assert_eq!((r.formacion.minimo, r.formacion.plazo), (3, 10 * MS_POR_MINUTO));
        assert_eq!((r.vision.ejercito, r.vision.jugador_solo), (30_000, 15_000));
        assert_eq!((r.presencia.retardo_desconexion, r.presencia.tope_aplazamiento), (150_000, 300_000));
    }

    #[test]
    fn la_ficha_militar_de_las_escuadras() {
        let m = Reglas::cargar().militar;
        assert_eq!((m.racion_por_soldado, m.regeneracion_moral, m.degradacion_moral), (150, 5_000, 20_000));
        assert_eq!((m.desercion, m.bonus_poder_por_nivel, m.nivel_maximo), (50_000, 10_000, 10));
        assert_eq!(m.experiencia_para_subir, vec![5_000, 10_000, 20_000, 40_000, 40_000, 40_000, 40_000, 40_000, 40_000]);
    }

    #[test]
    fn los_mercenarios_y_el_campamento() {
        let m = Reglas::cargar().mercenarios;
        assert_eq!((m.recargo, m.descuento_sin_asentamientos, m.nivel_edificios), (1_500_000, 600_000, 3));
        assert_eq!((m.valor_equipo_en_oro, m.poblacion_por_vivienda, m.poblacion_por_hora), (8_000, 10, 10));
        assert_eq!((m.edificios_militares, m.viviendas, m.porcentaje_facciones), (vec![Tipo::Barracon, Tipo::GaleriaDeTiro, Tipo::Caballerizas], 10, 500_000));
        assert_eq!(m.racion_cada, 30 * MS_POR_MINUTO, "la ración gratis del campamento, cada 30 minutos");
        assert_eq!(m.radio_proteccion, 10_000, "nadie inicia un combate a 10 unidades de un campamento");
    }

    #[test]
    fn las_tecnologias_son_las_del_bloque_y_las_de_arranque_las_dos_del_inicio() {
        let t = Reglas::cargar().tecnologias;
        let mut arranque = t.de_arranque.clone();
        arranque.sort();
        assert_eq!(arranque, vec!["hostigamiento_tribal".to_owned(), "leva_comunal".to_owned()]);
        assert!(t.todas.len() > 10 && t.todas.contains(&"leva_comunal".to_owned()) && t.todas.contains(&"arco_compuesto".to_owned()));
        let r = Reglas::cargar();
        assert!(r.tropas.iter().all(|d| t.todas.contains(&d.tecnologia)), "toda tropa pide una tecnología que existe");
    }

    #[test]
    fn la_identidad_son_los_bloques_y_no_la_prosa() {
        let t = texto_de_parametros();
        assert!(t.contains("\"mineros\"") && t.contains("capacidadViveresPorHeroe") && t.contains("poderPorHeroe") && t.contains("retardoDesconexionMs"));
        assert!(!t.contains("umbralRecalculo") && !t.contains("umbralCapturaCaravana") && !t.contains("radioReabastecimiento"));
        assert_eq!(t.matches("<!-- parametros:").count(), t.matches("<!-- /parametros -->").count());
    }
}
