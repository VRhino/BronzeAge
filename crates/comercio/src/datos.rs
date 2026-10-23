//! Las cifras del comercio. Fuente única: los bloques `<!-- parametros:NOMBRE -->` de `Docs/Mechanics/Comercio_Caravanas.md` (y, para lo
//! que no es suyo, `Tecnologia_Aedas.md` y `Movimiento_Ejercitos.md`), incrustados en el binario. Los decimales pasan una sola vez a
//! enteros: **ppm** para tasas y factores, **milésimas** (`Mil`) para cantidades y distancias. La reputación va en **ppm de punto**
//! (0,0005 por minuto no cabe en milésimas).
//!
//! **Valores calibrados** (Comercio_Caravanas, final, y Doc 3): los bloques de TypeScript dicen 600 para `distanciaParaBonusMax` y para
//! `distanciaReferencia`; lo vigente es **120**. Las velocidades de los animales van ×1,5; `NAVEGACION.espaciadoMin` es 8 y los vados
//! cada 100. Se aplican al cargar (como `escala_distancia` 80 en `asentamiento::mantenimiento`) y las fija un test.

use asentamiento::datos::{bloque_de, mil, por, ppm, texto_de_parametros_de};
use asentamiento::Recurso;
use base::{Mil, MS_POR_MINUTO};
use serde_json::Value;
use std::collections::BTreeMap;

pub const COMERCIO: &str = include_str!("../../../Docs/Mechanics/Comercio_Caravanas.md");
pub const TECNOLOGIA: &str = include_str!("../../../Docs/Mechanics/Tecnologia_Aedas.md");
pub const MOVIMIENTO: &str = include_str!("../../../Docs/Mechanics/Movimiento_Ejercitos.md");

/// Un punto de reputación en ppm.
pub const PUNTO: i64 = 1_000_000;
/// Distancia vigente (120 unidades) para el bonus de la comisión y para la cercanía del reparto (el bloque dice 600).
pub const DISTANCIA_VIGENTE: Mil = 120_000;
/// Calibración de velocidades: ×1,5 sobre las del bloque `ANIMAL_CATALOGO`.
pub const FACTOR_VELOCIDAD_CALIBRADA: i64 = 1_500_000;

/// Los bloques de parámetros de los tres documentos, para la identidad de las reglas (hash del núcleo).
pub fn texto_de_parametros() -> String {
    [COMERCIO, TECNOLOGIA, MOVIMIENTO].iter().map(|d| texto_de_parametros_de(d)).collect()
}

fn bloque(doc: &str, nombre: &str) -> Value {
    bloque_de(doc, nombre).unwrap_or_else(|e| panic!("{e}"))
}

fn num(v: &Value, ruta: &str) -> f64 {
    ruta.split('.').try_fold(v, |v, k| v.get(k)).and_then(Value::as_f64).unwrap_or_else(|| panic!("falta el número «{ruta}»"))
}

fn costo(v: &Value) -> Vec<(Recurso, Mil)> {
    let mut c: Vec<_> = v
        .as_object()
        .expect("costo")
        .iter()
        .map(|(k, x)| (Recurso::desde_nombre(k).unwrap_or_else(|| panic!("recurso «{k}»")), mil(x.as_f64().expect("cantidad"))))
        .collect();
    c.sort();
    c
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasComision {
    pub tasa_misma_faccion: i64,
    pub tasa_externa: i64,
    pub bonus_por_distancia_max: i64,
    pub distancia_para_bonus_max: Mil,
}

impl ReglasPrecio {
    /// Precio base (ppm de oro por unidad) de `r`: el de `PRECIO_BASE` o, si es un manufacturado, el derivado de su receta. El oro no tiene.
    pub fn base_de(&self, r: Recurso) -> Option<i64> {
        self.base.get(&r).or_else(|| self.derivados.get(&r)).copied()
    }

    /// Los recursos con cotización: materias primas y manufacturados.
    pub fn cotizados(&self) -> impl Iterator<Item = Recurso> + '_ {
        self.base.keys().chain(self.derivados.keys()).copied()
    }
}

/// Precio base de cada bien con receta y sin entrada en `PRECIO_BASE` (Comercio_Caravanas §3.1): lo que consume su receta, a precio base, por
/// unidad fabricada. Un bien cuyos insumos no cotizan (las chatarras, el oro) no tiene precio; una receta se resuelve en cuanto todos sus
/// insumos tienen, y se repite hasta que nada cambia (un lingote de bronce necesita antes los de cobre y estaño).
/// Si un bien tiene recetas distintas según el taller y el nivel (cuero con 0,5, 0,33 o 0,25 de ganado), cotiza la **más barata** (Comercio_Caravanas
/// §3.1). Los manufacturados no se venden a los mercenarios (solo `PRECIO_BASE`).
fn precios_derivados(base: &BTreeMap<Recurso, i64>, catalogo: &asentamiento::Catalogo) -> BTreeMap<Recurso, i64> {
    let mut derivados: BTreeMap<Recurso, i64> = BTreeMap::new();
    loop {
        let mut cambio = false;
        for rec in catalogo.recetas().filter(|rec| !base.contains_key(&rec.produce)) {
            // Cantidad en milésimas × precio en ppm = milésimas de milésima de oro por unidad: /1000 devuelve ppm.
            let suma = rec.consume_por_unidad.iter().map(|&(i, n)| base.get(&i).or_else(|| derivados.get(&i)).map(|&p| n * p / 1_000)).sum::<Option<i64>>();
            if let Some(p) = suma.filter(|&p| p > 0 && derivados.get(&rec.produce).is_none_or(|&antes| p < antes)) {
                derivados.insert(rec.produce, p);
                cambio = true;
            }
        }
        if !cambio {
            return derivados;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasPrecio {
    /// `PRECIO_BASE`: las materias primas (ppm de oro por unidad). Solo estos bienes se venden en los campamentos de mercenarios.
    pub base: BTreeMap<Recurso, i64>,
    /// Precio base de los bienes manufacturados, derivado de sus recetas (ver [`precios_derivados`]): no está en `PRECIO_BASE`.
    pub derivados: BTreeMap<Recurso, i64>,
    pub stock_objetivo_global: Mil,
    pub factor_min: i64,
    pub factor_max: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasTrueque {
    /// Plazo (ms) para contestar una propuesta y para cumplir un trueque aceptado.
    pub plazo: i64,
    /// Retención (ms) de un trueque terminado antes de podarlo.
    pub retencion_terminados: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasAsignacion {
    pub peso_urgencia_expiracion: i64,
    pub peso_urgencia_volumen: i64,
    pub peso_cercania: i64,
    pub distancia_referencia: Mil,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefCarro {
    pub capacidad_base: Mil,
    pub costo: Vec<(Recurso, Mil)>,
    /// `true` si lo fabrica la Carpintería (el básico lo fabrica el Mercado, que ya se exige para crear).
    pub exige_carpinteria: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefAnimal {
    pub factor_carga: i64,
    /// Unidades de mapa por minuto, en milésimas, **ya calibrada** (×1,5).
    pub velocidad: Mil,
    pub costo: Vec<(Recurso, Mil)>,
}

/// Categorías de caravana sin flota propia (`CARAVANA_CATALOGO`): solo datos.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefCategoria {
    pub capacidad: Mil,
    pub velocidad: Mil,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasFlota {
    pub basico: DefCarro,
    pub reforzado: DefCarro,
    pub buey: DefAnimal,
    pub caballo: DefAnimal,
    pub camello: DefAnimal,
    pub cooldown_creacion: i64,
    pub k_preparacion_por_carro: i64,
    pub max_programacion: i64,
    /// Liderazgo de escolta por caja de Mercado (nivel interno 1, 2, 3).
    pub liderazgo_por_nivel_mercado: Vec<i64>,
    pub militar: DefCategoria,
    pub construccion: DefCategoria,
    pub contrabando: DefCategoria,
    /// `VISION.caravanaGrande.carros`: desde cuántos carros con animal una comercial se ve de lejos.
    pub carros_caravana_grande: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasRed {
    pub caducidad: i64,
    pub escalon_camino: u32,
    pub escalon_calzada: u32,
    pub logro_rutas: u32,
    pub logro_facciones: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasMercado {
    pub historial_por_plaza: usize,
    pub plazo_orden: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasReputacion {
    pub decaimiento_por_minuto: i64,
    pub bonus_trueque_cumplido: i64,
    pub penalizacion_trueque_incumplido: i64,
    pub bonus_liberar_vasallo_voluntario: i64,
    pub penalizacion_rebelion_para_senora: i64,
    pub penalizacion_romper_alianza: i64,
    pub penalizacion_atacar_aliado: i64,
    pub bonus_defender_vasallo: i64,
    pub penalizacion_no_defender_vasallo: i64,
    pub penalizacion_guerra_tras_romper_alianza: i64,
    pub ventana_guerra_tras_romper_alianza: i64,
    pub bonus_por_minuto_alianza_activa: i64,
    pub umbral_bajo_para_embajador: i64,
    pub umbral_bajo_para_comision: i64,
    pub factor_comision_por_reputacion_baja: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasMercenarios {
    pub cada: i64,
    pub proporcion_repone: i64,
    pub tope_por_bien: Mil,
    pub stock_inicial: Mil,
    pub pilas: BTreeMap<Recurso, Mil>,
    pub cupo_diario: BTreeMap<Recurso, Mil>,
    pub margen: i64,
    /// `ALMACEN_PERSONAL.capacidad`.
    pub capacidad_almacen_personal: Mil,
    /// `LOGISTICA.capacidadViveresPorHeroe`.
    pub capacidad_viveres_por_heroe: Mil,
}

/// Lo que el comercio lee de `NAVEGACION` (el grafo lo construye `crates/rutas`); con los valores vigentes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Navegacion {
    pub espaciado_min: Mil,
    pub nodos_por_lado: u32,
    pub muestreo_max: Mil,
    pub separacion_vados_min: Mil,
    pub vados_por_lado: u32,
    pub radio_paso_ciudad_min: Mil,
    pub atraccion_por_peso: i64,
    pub atraccion_min: i64,
    /// `COSTE_MOVIMIENTO.factorCamino` (ppm): sobre una arista de la red de caminos el coste del tramo se multiplica por esto.
    pub factor_camino: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reglas {
    pub comision: ReglasComision,
    pub precio: ReglasPrecio,
    pub trueque: ReglasTrueque,
    pub asignacion: ReglasAsignacion,
    pub flota: ReglasFlota,
    /// `PEAJE_PASO.tasa`.
    pub peaje: i64,
    pub red: ReglasRed,
    pub mercado: ReglasMercado,
    pub reputacion: ReglasReputacion,
    pub mercenarios: ReglasMercenarios,
    pub navegacion: Navegacion,
}

impl Reglas {
    pub fn cargar() -> Self {
        Self::de(&|n| bloque(COMERCIO, n))
    }

    /// Construye las reglas con los bloques del propio documento de comercio y, para lo ajeno, de su documento.
    fn de(b: &dyn Fn(&str) -> Value) -> Self {
        let c = b("COMISION");
        let comision = ReglasComision {
            tasa_misma_faccion: ppm(num(&c, "tasaMismaFaccion")),
            tasa_externa: ppm(num(&c, "tasaExterna")),
            bonus_por_distancia_max: ppm(num(&c, "bonusPorDistanciaMax")),
            // Vigente (Comercio_Caravanas, valores calibrados): 120, no las 600 del bloque.
            distancia_para_bonus_max: DISTANCIA_VIGENTE,
        };
        let pb = b("PRECIO_BASE");
        let base = pb
            .as_object()
            .expect("PRECIO_BASE")
            .iter()
            .map(|(k, v)| (Recurso::desde_nombre(k).unwrap_or_else(|| panic!("recurso «{k}»")), ppm(v.as_f64().expect("precio"))))
            .collect();
        let pr = b("PRECIO_REFERENCIA");
        let derivados = precios_derivados(&base, &asentamiento::Catalogo::de(&asentamiento::datos::bloque("EDIFICIO_CATALOGO")));
        let precio = ReglasPrecio { base, derivados, stock_objetivo_global: mil(num(&pr, "stockObjetivoGlobal")), factor_min: ppm(num(&pr, "factorMin")), factor_max: ppm(num(&pr, "factorMax")) };
        let t = b("TRUEQUE");
        let trueque = ReglasTrueque { plazo: num(&t, "plazoMinutosPorDefecto") as i64 * MS_POR_MINUTO, retencion_terminados: num(&t, "retencionTerminadosMinutos") as i64 * MS_POR_MINUTO };
        let a = b("ASIGNACION_CARAVANA");
        let asignacion = ReglasAsignacion {
            peso_urgencia_expiracion: ppm(num(&a, "pesoUrgenciaExpiracion")),
            peso_urgencia_volumen: ppm(num(&a, "pesoUrgenciaVolumen")),
            peso_cercania: ppm(num(&a, "pesoCercania")),
            // Vigente: 120, no las 600 del bloque.
            distancia_referencia: DISTANCIA_VIGENTE,
        };
        let (cc, ac) = (b("CARRO_CATALOGO"), b("ANIMAL_CATALOGO"));
        let carro = |k: &str| DefCarro { capacidad_base: mil(num(&cc, &format!("{k}.capacidadBase"))), costo: costo(&cc[k]["costo"]), exige_carpinteria: cc[k]["fabrica"] == "carpinteria" };
        let animal = |k: &str| DefAnimal {
            factor_carga: ppm(num(&ac, &format!("{k}.factorCarga"))),
            velocidad: por(mil(num(&ac, &format!("{k}.velocidad"))), FACTOR_VELOCIDAD_CALIBRADA),
            costo: costo(&ac[k]["costo"]),
        };
        let (cat, esc, prep, cool) = (b("CARAVANA_CATALOGO"), b("CARAVANA_ESCOLTA"), b("CARAVANA_PREPARACION"), b("CARAVANA_COOLDOWN"));
        let categoria = |k: &str| DefCategoria { capacidad: mil(num(&cat, &format!("{k}.capacidad"))), velocidad: mil(num(&cat, &format!("{k}.velocidad"))) };
        let vision = bloque(MOVIMIENTO, "VISION");
        let flota = ReglasFlota {
            basico: carro("basico"),
            reforzado: carro("reforzado"),
            buey: animal("buey"),
            caballo: animal("caballo"),
            camello: animal("camello"),
            cooldown_creacion: num(&cool, "cooldownMinutos") as i64 * MS_POR_MINUTO,
            k_preparacion_por_carro: num(&prep, "kPorCarro") as i64 * MS_POR_MINUTO,
            max_programacion: num(&prep, "maxProgramacionDias") as i64 * 24 * 60 * MS_POR_MINUTO,
            liderazgo_por_nivel_mercado: esc["liderazgoPorNivelMercado"].as_array().expect("liderazgoPorNivelMercado").iter().map(|x| x.as_i64().expect("liderazgo")).collect(),
            militar: categoria("militar"),
            construccion: categoria("construccion"),
            contrabando: categoria("contrabando"),
            carros_caravana_grande: num(&vision, "caravanaGrande.carros") as usize,
        };
        let rc = b("RED_CAMINOS");
        let red = ReglasRed {
            caducidad: num(&rc, "caducidadMinutos") as i64 * MS_POR_MINUTO,
            escalon_camino: num(&rc, "escalonCamino") as u32,
            escalon_calzada: num(&rc, "escalonCalzada") as u32,
            logro_rutas: num(&rc, "logroRutas") as u32,
            logro_facciones: num(&rc, "logroFacciones") as u32,
        };
        let m = b("MERCADO");
        let mercado = ReglasMercado { historial_por_plaza: num(&m, "historialPorPlaza") as usize, plazo_orden: num(&m, "plazoOrdenMinutos") as i64 * MS_POR_MINUTO };
        let p = |k: &str| (num(&b("REPUTACION"), k) * PUNTO as f64).round() as i64;
        let reputacion = ReglasReputacion {
            decaimiento_por_minuto: p("decaimientoPorMinuto"),
            bonus_trueque_cumplido: p("bonusTruequeCumplido"),
            penalizacion_trueque_incumplido: p("penalizacionTruequeIncumplido"),
            bonus_liberar_vasallo_voluntario: p("bonusLiberarVasalloVoluntario"),
            penalizacion_rebelion_para_senora: p("penalizacionRebelionParaSenora"),
            penalizacion_romper_alianza: p("penalizacionRomperAlianza"),
            penalizacion_atacar_aliado: p("penalizacionAtacarAliado"),
            bonus_defender_vasallo: p("bonusDefenderVasallo"),
            penalizacion_no_defender_vasallo: p("penalizacionNoDefenderVasallo"),
            penalizacion_guerra_tras_romper_alianza: p("penalizacionGuerraTrasRomperAlianza"),
            ventana_guerra_tras_romper_alianza: num(&b("REPUTACION"), "ventanaGuerraTrasRomperAlianzaDias") as i64 * 24 * 60 * MS_POR_MINUTO,
            bonus_por_minuto_alianza_activa: p("bonusPorMinutoAlianzaActiva"),
            umbral_bajo_para_embajador: p("umbralBajoParaEmbajador"),
            umbral_bajo_para_comision: p("umbralBajoParaComision"),
            factor_comision_por_reputacion_baja: ppm(num(&b("REPUTACION"), "factorComisionPorReputacionBaja")),
        };
        let mer = bloque(TECNOLOGIA, "MERCENARIOS");
        let por_recurso = |ruta: &str| -> BTreeMap<Recurso, Mil> {
            mer["mercado"][ruta].as_object().expect(ruta).iter().map(|(k, v)| (Recurso::desde_nombre(k).expect("recurso"), mil(v.as_f64().expect("cantidad")))).collect()
        };
        let mercenarios = ReglasMercenarios {
            cada: num(&mer, "mercado.cadaHoras") as i64 * 60 * MS_POR_MINUTO,
            proporcion_repone: ppm(num(&mer, "mercado.proporcionRepone")),
            tope_por_bien: mil(num(&mer, "mercado.topePorBien")),
            stock_inicial: mil(num(&mer, "mercado.stockInicial")),
            pilas: por_recurso("pilas"),
            cupo_diario: por_recurso("cupoDiario"),
            margen: ppm(num(&mer, "mercado.margen")),
            capacidad_almacen_personal: mil(num(&b("ALMACEN_PERSONAL"), "capacidad")),
            capacidad_viveres_por_heroe: mil(num(&bloque(MOVIMIENTO, "LOGISTICA"), "capacidadViveresPorHeroe")),
        };
        let nv = b("NAVEGACION");
        let navegacion = Navegacion {
            // Vigente: espaciadoMin 45 → 8 y vados cada 100.
            espaciado_min: 8_000,
            nodos_por_lado: num(&nv, "nodosPorLado") as u32,
            muestreo_max: mil(num(&nv, "muestreoMax")),
            separacion_vados_min: 100_000,
            vados_por_lado: num(&nv, "vadosPorLado") as u32,
            radio_paso_ciudad_min: mil(num(&nv, "radioPasoCiudadMin")),
            atraccion_por_peso: ppm(num(&nv, "atraccionPorPeso")),
            atraccion_min: ppm(num(&nv, "atraccionMin")),
            factor_camino: ppm(num(&b("COSTE_MOVIMIENTO"), "factorCamino")),
        };
        Reglas {
            comision,
            precio,
            trueque,
            asignacion,
            flota,
            peaje: ppm(num(&b("PEAJE_PASO"), "tasa")),
            red,
            mercado,
            reputacion,
            mercenarios,
            navegacion,
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn las_reglas_cargan_con_las_cifras_vigentes() {
        let r = Reglas::cargar();
        // Comisión: 3 %, 8 %, bonus ×1,5; el alcance del bonus es 120 (no las 600 del bloque).
        assert_eq!((r.comision.tasa_misma_faccion, r.comision.tasa_externa, r.comision.bonus_por_distancia_max), (30_000, 80_000, 1_500_000));
        assert_eq!(r.comision.distancia_para_bonus_max, 120_000);
        assert_eq!(r.asignacion.distancia_referencia, 120_000);
        assert_eq!((r.asignacion.peso_urgencia_expiracion, r.asignacion.peso_urgencia_volumen, r.asignacion.peso_cercania), (500_000, 300_000, 200_000));
        // Precios.
        assert_eq!(r.precio.base[&Recurso::Piedra], 1_200_000);
        assert!(!r.precio.base.contains_key(&Recurso::Oro), "el oro no cotiza");
        assert_eq!((r.precio.stock_objetivo_global, r.precio.factor_min, r.precio.factor_max), (500_000, 400_000, 3_000_000));
        // Trueque: un día para contestar y para cumplir; una semana de retención.
        assert_eq!((r.trueque.plazo, r.trueque.retencion_terminados), (1440 * 60_000, 10_080 * 60_000));
        // Carros y animales: velocidades ×1,5 (buey 24, caballo 36, camello 28,5).
        assert_eq!((r.flota.basico.capacidad_base, r.flota.reforzado.capacidad_base), (500_000, 800_000));
        assert!(r.flota.reforzado.exige_carpinteria && !r.flota.basico.exige_carpinteria);
        assert_eq!((r.flota.buey.velocidad, r.flota.caballo.velocidad, r.flota.camello.velocidad), (24_000, 36_000, 28_500));
        assert_eq!((r.flota.buey.factor_carga, r.flota.caballo.factor_carga, r.flota.camello.factor_carga), (1_000_000, 500_000, 750_000));
        assert_eq!(r.flota.buey.costo, vec![(Recurso::Oro, 12_000)]);
        assert_eq!(r.flota.basico.costo, vec![(Recurso::Madera, 20_000)]);
        assert_eq!((r.flota.cooldown_creacion, r.flota.k_preparacion_por_carro, r.flota.max_programacion), (10 * 60_000, 2 * 60_000, 3 * 24 * 60 * 60_000));
        assert_eq!(r.flota.liderazgo_por_nivel_mercado, vec![100, 200, 300]);
        assert_eq!(r.flota.carros_caravana_grande, 3);
        assert_eq!((r.flota.militar.velocidad, r.flota.construccion.capacidad), (12_000, 150_000));
        // Peaje, red, mercado.
        assert_eq!(r.peaje, 20_000);
        assert_eq!((r.red.caducidad, r.red.escalon_camino, r.red.escalon_calzada, r.red.logro_rutas, r.red.logro_facciones), (4320 * 60_000, 2, 6, 10, 3));
        assert_eq!((r.mercado.historial_por_plaza, r.mercado.plazo_orden), (200, 200 * 60_000));
    }

    #[test]
    fn la_reputacion_usa_las_decisiones_del_autor() {
        let r = Reglas::cargar().reputacion;
        // Decaimiento 0,002 y bonus de alianza 0,0005 por minuto (2026-10-18), en ppm de punto.
        assert_eq!((r.decaimiento_por_minuto, r.bonus_por_minuto_alianza_activa), (2_000, 500));
        assert!(r.bonus_por_minuto_alianza_activa < r.decaimiento_por_minuto);
        assert_eq!((r.bonus_trueque_cumplido, r.penalizacion_trueque_incumplido), (5 * PUNTO, -8 * PUNTO));
        assert_eq!((r.umbral_bajo_para_comision, r.umbral_bajo_para_embajador), (-40 * PUNTO, -40 * PUNTO));
        assert_eq!(r.factor_comision_por_reputacion_baja, 1_500_000);
        assert_eq!(r.ventana_guerra_tras_romper_alianza, 3 * 86_400_000);
    }

    #[test]
    fn mercenarios_y_navegacion_vigentes() {
        let r = Reglas::cargar();
        let m = &r.mercenarios;
        assert_eq!((m.cada, m.proporcion_repone, m.tope_por_bien, m.stock_inicial, m.margen), (3 * 3_600_000, 30_000, 300_000, 100_000, 1_300_000));
        assert_eq!(m.pilas[&Recurso::Piedra], 60_000);
        assert_eq!(m.cupo_diario[&Recurso::Madera], 60_000);
        assert_eq!((m.capacidad_almacen_personal, m.capacidad_viveres_por_heroe), (1_000_000, 350_000));
        let n = &r.navegacion;
        assert_eq!((n.espaciado_min, n.separacion_vados_min), (8_000, 100_000));
        assert_eq!((n.atraccion_por_peso, n.atraccion_min, n.factor_camino), (100_000, 500_000, 500_000));
    }

    #[test]
    fn la_identidad_son_los_bloques_y_no_la_prosa() {
        let t = texto_de_parametros();
        assert!(t.contains("tasaMismaFaccion") && t.contains("\"cadaHoras\"") && t.contains("capacidadViveresPorHeroe"));
        assert!(!t.contains("Reescrito el 2026-10-13"));
        assert_eq!(t.matches("<!-- parametros:").count(), t.matches("<!-- /parametros -->").count());
    }
}
