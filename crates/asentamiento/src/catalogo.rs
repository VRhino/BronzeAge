//! Catálogo de edificios (`EDIFICIO_CATALOGO`, Doc 4.2.1) y las reglas de construcción que lo acompañan (`NECESIDADES`,
//! `SCORE_BANDAS`, `EDIFICIOS_UNICOS`…). Todo sale de los bloques de Mechanics; aquí solo se convierte a tipos y enteros.

use crate::datos::{mil, ppm, UNO};
use crate::recurso::{Costo, Recurso};
use base::Mil;
use serde_json::Value;
use std::collections::BTreeMap;

macro_rules! tipos {
    ($($v:ident = $n:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Tipo { $($v),+ }
        pub const TIPOS: &[Tipo] = &[$(Tipo::$v),+];
        impl Tipo {
            /// Clave en los datos (`"lenera"`, `"galeriaDeTiro"`).
            pub fn nombre(self) -> &'static str { match self { $(Tipo::$v => $n),+ } }
            pub fn desde_nombre(s: &str) -> Option<Tipo> { match s { $($n => Some(Tipo::$v),)+ _ => None } }
        }
    };
}

tipos! {
    CentroUrbano = "centroUrbano", Vivienda = "vivienda", Granja = "granja", Cantera = "cantera", Lenera = "lenera",
    Almacen = "almacen", Granero = "granero", Mina = "mina", MinaCobre = "minaCobre", MinaEstano = "minaEstano",
    MinaHierro = "minaHierro", Corral = "corral", GranFundicion = "granFundicion", Fundicion = "fundicion",
    Curtiduria = "curtiduria", Armeria = "armeria", Carpinteria = "carpinteria", Barracon = "barracon",
    GaleriaDeTiro = "galeriaDeTiro", Caballerizas = "caballerizas", SalaConsejo = "salaConsejo", Taberna = "taberna",
    Palacio = "palacio", Mercado = "mercado", PuestoMercado = "puestoMercado", Plaza = "plaza", PlazaDeArmas = "plazaDeArmas",
    PatioDeGremios = "patioDeGremios", Pozo = "pozo", Parque = "parque", Maravilla = "maravilla",
    // Al final: la foto guarda el tipo por su posición en esta lista.
    AlbergueCaravanas = "albergueCaravanas",
    // Los de la comida y el prestigio de la tanda 69 (2026-11-08).
    Molino = "molino", Horno = "horno", Quesera = "quesera", Bodega = "bodega", Prensa = "prensa", Estatua = "estatua",
}

impl Tipo {
    /// Extractores que ligan a un nodo finito del mapa (o, la Leñera, a una parcela de bosque).
    pub fn es_extractor(self) -> bool {
        matches!(self, Tipo::Cantera | Tipo::Mina | Tipo::MinaCobre | Tipo::MinaEstano | Tipo::MinaHierro | Tipo::Corral)
    }

    /// Los extractores que solo se explotan con un pueblo extractor (Doc 4.2.2): los minerales y la Cantera (también la primera, desde
    /// el 2026-11-01: el pueblo extractor se funda desde el nivel 1). El Corral, la Leñera y la Granja nunca van por pueblo.
    pub fn es_de_pueblo(self) -> bool {
        matches!(self, Tipo::Cantera | Tipo::Mina | Tipo::MinaCobre | Tipo::MinaEstano | Tipo::MinaHierro)
    }

    /// El Corral solo existe en las aldeas agrícolas (Economia_Construccion §14.2): la ciudad no lo construye.
    pub fn solo_en_aldea(self) -> bool {
        self == Tipo::Corral
    }

    /// Edificios que piden mano de obra de pesants (`trabajadoresRequeridosTotal`, §5.1).
    pub fn usa_pesants(self) -> bool {
        self.es_extractor() || matches!(self, Tipo::Granja | Tipo::Lenera)
    }

    /// Talleres: los edificios con recetas (los cinco de la comida de la tanda 69 incluidos).
    pub fn es_taller(self) -> bool {
        matches!(
            self,
            Tipo::Fundicion | Tipo::Curtiduria | Tipo::Armeria | Tipo::Carpinteria | Tipo::GranFundicion | Tipo::Molino | Tipo::Horno | Tipo::Quesera | Tipo::Bodega | Tipo::Prensa
        )
    }

    /// Bodega y prensa (2026-11-09): trabajan una sola receta a la vez, la de su insumo activo (`TALLERES_DE_UNA_RECETA`).
    pub fn de_una_receta(self) -> bool {
        matches!(self, Tipo::Bodega | Tipo::Prensa)
    }

    /// Los talleres que trabajan los pesants: el molino y el horno, de pesants para pesants (autor, 2026-11-08). No traen artesanos.
    pub fn taller_de_pesants(self) -> bool {
        matches!(self, Tipo::Molino | Tipo::Horno)
    }

    /// Los talleres que trabajan los artesanos, y que traen a los primeros.
    pub fn pide_artesanos(self) -> bool {
        self.es_taller() && !self.taller_de_pesants()
    }
}

#[derive(Clone, Debug)]
pub struct Receta {
    pub produce: Recurso,
    /// Producción base por minuto (milésimas): el techo ideal.
    pub produccion_base: Mil,
    pub consume_por_unidad: Costo,
    pub tecnologia: Option<String>,
}

/// Un nivel interno (el 1 es el edificio recién construido) con lo que cambia al mejorar.
#[derive(Clone, Debug, Default)]
pub struct NivelInterno {
    pub trabajadores: Mil,
    pub recetas: Vec<Receta>,
    pub requisito_nivel_asentamiento: u32,
    pub costo_mejora: Costo,
    pub tecnologia: Option<String>,
    pub requiere_edificio: Option<(Tipo, u8)>,
    pub obra_minutos: Option<i64>,
    pub capacidad_nobles: Mil,
    pub capacidad_trigo: Mil,
    pub produccion_base_trigo: Mil,
    pub cupo_caravanas: u32,
    pub cupo_carros: u32,
    pub cupo_miradas: u32,
    /// Huella en celdas del trazado urbano (ancho, alto) si cambia con el nivel interno (hoy solo la Granja: 2×2, 2×3, 4×3, 6×6).
    pub tamano: Option<(i32, i32)>,
}

#[derive(Clone, Debug)]
pub struct TipoEdificio {
    pub tipo: Tipo,
    pub costo: Costo,
    pub tiempo_minutos: i64,
    /// Nivel **operativo** mínimo del asentamiento para construirlo (0 = ninguno).
    pub nivel_construccion: u32,
    pub tecnologia: Option<String>,
    pub nivel_faccion_minimo: Option<u32>,
    /// Cupos de habitantes (milésimas) que aporta cada edificio activo (Vivienda).
    pub capacidad_pesants: Mil,
    pub capacidad_artesanos: Mil,
    /// Lo que suma a la capacidad de cada recurso (Almacén).
    pub capacidad_por_recurso: Mil,
    /// Producción base por minuto (milésimas) del extractor o la granja del nivel 1.
    pub produccion: Option<(Recurso, Mil)>,
    pub trabajadores: Mil,
    /// Cuánto multiplica su producción tener buey, en ppm (`factorBuey` del molino); `UNO` en los demás.
    pub factor_buey: i64,
    /// Niveles internos; vacío si no mejora. `niveles[0]` es el nivel 1.
    pub niveles: Vec<NivelInterno>,
}

impl TipoEdificio {
    /// El nivel interno `n` (1 en adelante), o el 1 si el tipo no mejora.
    pub fn nivel(&self, n: u8) -> Option<&NivelInterno> {
        self.niveles.get((n as usize).saturating_sub(1))
    }

    pub fn max_nivel_interno(&self) -> u8 {
        self.niveles.len().max(1) as u8
    }
}

#[derive(Clone, Debug)]
pub struct Necesidades {
    pub umbral_vivienda_ocupada: i64,
    pub umbral_almacen_ampliacion: i64,
    pub maximo_almacenes_por_nivel: BTreeMap<u32, u32>,
    pub maximo_en_cola: usize,
    pub maximo_en_construccion_simultanea: usize,
    pub maximo_transformacion_por_nivel: BTreeMap<u32, u32>,
    pub nivel_gate_anclas: u32,
    /// El nivel desde el que el barrio de la industria crea su ancla (el patio de gremios): antes que los demás, para que el molino y el horno,
    /// que se hacen desde el nivel 1, vayan con los talleres (`nivelGateAnclasIndustria`, autor, 2026-11-10).
    pub nivel_gate_anclas_industria: u32,
    pub reserva_manual_maxima: Mil,
}

#[derive(Clone, Debug)]
pub struct Bandas {
    pub supervivencia: i64,
    pub crecimiento: i64,
    pub manual: i64,
    pub transformacion: i64,
}

#[derive(Clone, Debug)]
pub struct ReglasConstruccion {
    pub necesidades: Necesidades,
    pub bandas: Bandas,
    pub extraccion_maxima_por_tipo: u32,
    pub multiplicador_mejora_por_nivel: i64,
    pub unicos: Vec<Tipo>,
    pub con_niveles: Vec<Tipo>,
    /// Edificios que ignoran la reserva de su propio recurso (la Granja el trigo, la Leñera la madera).
    pub recurso_propio: BTreeMap<Tipo, Recurso>,
    pub horizonte_mantenimiento_min: i64,
    pub horizonte_comida_min: i64,
    /// `TALLERES_DE_UNA_RECETA` (2026-11-09): la bodega y la prensa, que trabajan una sola receta a la vez.
    pub talleres_de_una_receta: BTreeMap<Tipo, TallerDeUnaReceta>,
}

/// Un taller de `TALLERES_DE_UNA_RECETA` (2026-11-09): cada edificio trabaja una sola de sus recetas, la que se elige, y la plaza tiene topes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TallerDeUnaReceta {
    /// Cuántos edificios de ese tipo caben en una plaza, de cualquier estado.
    pub maximo_por_plaza: u32,
    /// Cuántas recetas distintas puede trabajar la plaza en ese tipo, por nivel operativo.
    pub recetas_distintas_por_nivel: BTreeMap<u32, u32>,
}

impl TallerDeUnaReceta {
    /// Las recetas distintas que caben en ese nivel: un nivel que no está usa el más alto por debajo y, por debajo del primero, el primero.
    pub fn recetas_distintas(&self, nivel: u32) -> u32 {
        let t = &self.recetas_distintas_por_nivel;
        t.range(..=nivel).next_back().or_else(|| t.iter().next()).map_or(1, |(_, &n)| n)
    }
}

#[derive(Clone, Debug)]
pub struct Catalogo {
    tipos: BTreeMap<Tipo, TipoEdificio>,
}

impl Catalogo {
    pub fn tipo(&self, t: Tipo) -> &TipoEdificio {
        &self.tipos[&t]
    }

    /// Todas las recetas de todos los talleres y niveles internos, en el orden del catálogo.
    pub fn recetas(&self) -> impl Iterator<Item = &Receta> + Clone {
        self.tipos.values().flat_map(|t| t.niveles.iter().flat_map(|n| n.recetas.iter()))
    }
}

fn f(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}

pub(crate) fn costo(v: Option<&Value>) -> Costo {
    v.and_then(Value::as_object)
        .map(|o| o.iter().map(|(k, x)| (Recurso::desde_nombre(k).unwrap_or_else(|| panic!("recurso «{k}»")), mil(x.as_f64().expect("cantidad")))).collect())
        .unwrap_or_default()
}

fn texto(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_string)
}

fn receta(v: &Value) -> Receta {
    Receta {
        produce: Recurso::desde_nombre(v["produce"].as_str().expect("produce")).expect("recurso producido"),
        produccion_base: mil(f(v, "produccionBase").expect("produccionBase")),
        consume_por_unidad: costo(v.get("consumePorUnidad")),
        tecnologia: texto(v, "requiereTecnologia"),
    }
}

fn nivel_interno(v: &Value) -> NivelInterno {
    NivelInterno {
        trabajadores: mil(f(v, "trabajadoresRequeridos").unwrap_or(0.0)),
        recetas: v.get("recetas").and_then(Value::as_array).map(|a| a.iter().map(receta).collect()).unwrap_or_default(),
        requisito_nivel_asentamiento: f(v, "requisitoNivelAsentamiento").unwrap_or(0.0) as u32,
        costo_mejora: costo(v.get("costoMejora")),
        tecnologia: texto(v, "requiereTecnologia"),
        requiere_edificio: texto(v, "requiereEdificio").map(|t| (Tipo::desde_nombre(&t).unwrap_or_else(|| panic!("tipo «{t}»")), f(v, "requiereEdificioNivel").unwrap_or(1.0) as u8)),
        obra_minutos: f(v, "obraMinutos").map(|x| x as i64),
        capacidad_nobles: mil(f(v, "capacidadNobles").unwrap_or(0.0)),
        capacidad_trigo: mil(f(v, "capacidadTrigo").unwrap_or(0.0)),
        produccion_base_trigo: mil(f(v, "produccionBaseTrigo").unwrap_or(0.0)),
        cupo_caravanas: f(v, "cupoCaravanas").unwrap_or(0.0) as u32,
        cupo_carros: f(v, "cupoCarros").unwrap_or(0.0) as u32,
        cupo_miradas: f(v, "cupoMiradas").unwrap_or(0.0) as u32,
        tamano: v.get("tamano").and_then(|t| Some((f(t, "ancho")? as i32, f(t, "alto")? as i32))),
    }
}

const PRODUCCIONES: [(&str, Recurso); 8] = [
    ("produccionBaseTrigo", Recurso::Trigo),
    ("produccionBasePiedra", Recurso::Piedra),
    ("produccionBaseMadera", Recurso::Madera),
    ("produccionBaseOro", Recurso::Oro),
    ("produccionBaseCobre", Recurso::Cobre),
    ("produccionBaseEstano", Recurso::Estano),
    ("produccionBaseHierro", Recurso::Hierro),
    ("produccionBaseLivestock", Recurso::Livestock),
];

impl Catalogo {
    pub fn de(bloque: &Value) -> Self {
        let mut tipos = BTreeMap::new();
        for (clave, v) in bloque.as_object().expect("el catálogo es un objeto") {
            let tipo = Tipo::desde_nombre(clave).unwrap_or_else(|| panic!("tipo de edificio «{clave}»"));
            let niveles = match v.get("niveles") {
                Some(Value::Object(o)) => {
                    let mut ks: Vec<u32> = o.keys().map(|k| k.parse().expect("nivel numérico")).collect();
                    ks.sort_unstable();
                    ks.iter().map(|k| nivel_interno(&o[&k.to_string()])).collect()
                }
                _ => vec![],
            };
            let produccion = PRODUCCIONES.iter().find_map(|&(k, r)| f(v, k).map(|x| (r, mil(x))));
            tipos.insert(
                tipo,
                TipoEdificio {
                    tipo,
                    costo: costo(v.get("costo")),
                    tiempo_minutos: f(v, "tiempoConstruccionMinutos").unwrap_or(0.0) as i64,
                    nivel_construccion: f(v, "requisitoNivelAsentamientoConstruccion").unwrap_or(0.0) as u32,
                    tecnologia: texto(v, "requiereTecnologia"),
                    nivel_faccion_minimo: f(v, "nivelFaccionMinimo").map(|x| x as u32),
                    capacidad_pesants: mil(f(v, "capacidadPesants").unwrap_or(0.0)),
                    capacidad_artesanos: mil(f(v, "capacidadArtesanos").unwrap_or(0.0)),
                    capacidad_por_recurso: mil(f(v, "capacidadPorRecursoAdicional").unwrap_or(0.0)),
                    produccion,
                    trabajadores: mil(f(v, "trabajadoresRequeridos").unwrap_or(0.0)),
                    factor_buey: f(v, "factorBuey").map_or(UNO, ppm),
                    niveles,
                },
            );
        }
        Catalogo { tipos }
    }
}

fn mapa_niveles(v: &Value) -> BTreeMap<u32, u32> {
    v.as_object().expect("objeto por nivel").iter().map(|(k, x)| (k.parse().expect("nivel"), x.as_f64().expect("tope") as u32)).collect()
}

fn lista_tipos(v: &Value) -> Vec<Tipo> {
    v.as_array().expect("lista").iter().map(|x| Tipo::desde_nombre(x.as_str().expect("tipo")).expect("tipo conocido")).collect()
}

impl ReglasConstruccion {
    pub fn de(b: &dyn Fn(&str) -> Value) -> Self {
        let (n, s, r) = (b("NECESIDADES"), b("SCORE_BANDAS"), b("RESERVA_CONSTRUCCION"));
        let g = |v: &Value, k: &str| f(v, k).unwrap_or_else(|| panic!("falta «{k}»"));
        ReglasConstruccion {
            necesidades: Necesidades {
                umbral_vivienda_ocupada: ppm(g(&n, "umbralViviendaOcupada")),
                umbral_almacen_ampliacion: ppm(g(&n, "umbralAlmacenAmpliacion")),
                maximo_almacenes_por_nivel: mapa_niveles(&n["maximoAlmacenesPorNivel"]),
                maximo_en_cola: g(&n, "maximoEnCola") as usize,
                maximo_en_construccion_simultanea: g(&n, "maximoEnConstruccionSimultanea") as usize,
                maximo_transformacion_por_nivel: mapa_niveles(&n["maximoTransformacionPorNivel"]),
                nivel_gate_anclas: g(&n, "nivelGateAnclas") as u32,
                nivel_gate_anclas_industria: g(&n, "nivelGateAnclasIndustria") as u32,
                reserva_manual_maxima: mil(g(&n, "reservaManualMaxima")),
            },
            bandas: Bandas {
                supervivencia: g(&s, "supervivencia") as i64,
                crecimiento: g(&s, "crecimiento") as i64,
                manual: g(&s, "manual") as i64,
                transformacion: g(&s, "transformacion") as i64,
            },
            extraccion_maxima_por_tipo: g(&b("EXTRACCION_MAXIMOS"), "porTipo") as u32,
            multiplicador_mejora_por_nivel: g(&b("MEJORA_EDIFICIO"), "multiplicadorPorNivel") as i64,
            unicos: lista_tipos(&b("EDIFICIOS_UNICOS")),
            con_niveles: lista_tipos(&b("EDIFICIOS_CON_NIVELES")),
            recurso_propio: b("RECURSO_PROPIO_DE_EDIFICIO")
                .as_object()
                .expect("objeto")
                .iter()
                .map(|(k, v)| (Tipo::desde_nombre(k).expect("tipo"), Recurso::desde_nombre(v.as_str().expect("recurso")).expect("recurso")))
                .collect(),
            horizonte_mantenimiento_min: g(&r, "horizonteMinutosMantenimiento") as i64,
            horizonte_comida_min: g(&r, "horizonteMinutosComida") as i64,
            talleres_de_una_receta: b("TALLERES_DE_UNA_RECETA")
                .as_object()
                .expect("TALLERES_DE_UNA_RECETA")
                .iter()
                .map(|(k, v)| {
                    let tipo = Tipo::desde_nombre(k).unwrap_or_else(|| panic!("tipo «{k}» en TALLERES_DE_UNA_RECETA"));
                    (tipo, TallerDeUnaReceta { maximo_por_plaza: g(v, "maximoPorPlaza") as u32, recetas_distintas_por_nivel: mapa_niveles(&v["recetasDistintasPorNivel"]) })
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;

    #[test]
    fn los_38_tipos_se_leen_y_sus_nombres_son_coherentes() {
        let r = Reglas::cargar();
        assert_eq!(TIPOS.len(), 38);
        for &t in TIPOS {
            assert_eq!(Tipo::desde_nombre(t.nombre()), Some(t));
            assert_eq!(r.catalogo.tipo(t).tipo, t);
        }
    }

    #[test]
    fn cifras_del_canon_en_el_catalogo() {
        let c = Reglas::cargar().catalogo;
        let vivienda = c.tipo(Tipo::Vivienda);
        assert_eq!((vivienda.capacidad_pesants, vivienda.capacidad_artesanos, vivienda.tiempo_minutos), (15_000, 10_000, 10));
        assert_eq!(vivienda.costo, vec![(Recurso::Madera, 10_000)]);
        let granja = c.tipo(Tipo::Granja);
        // El arranque rápido (2026-10-28): Vivienda 10, Granja 5 y Leñera 2 minutos a nivel 1.
        assert_eq!((granja.trabajadores, granja.tiempo_minutos, c.tipo(Tipo::Lenera).tiempo_minutos), (4_000, 5, 2));
        // La Granja no sube de nivel y da 80 de trigo por minuto, 2/3 de los 120 de antes (2026-11-11, decisión del autor).
        let trigo: Vec<Mil> = granja.niveles.iter().map(|n| n.produccion_base_trigo).collect();
        assert_eq!(trigo, vec![80_000], "un solo nivel, 80 por minuto");
        assert_eq!(c.tipo(Tipo::Almacen).capacidad_por_recurso, 600_000, "almacenes al doble desde el 2026-11-05");
        assert_eq!(c.tipo(Tipo::Palacio).nivel(3).unwrap().capacidad_nobles, 400_000);
        assert_eq!(c.tipo(Tipo::Granero).nivel(4).unwrap().capacidad_trigo, 12_000_000);
        let fundicion = c.tipo(Tipo::Fundicion);
        assert_eq!(fundicion.nivel(2).unwrap().trabajadores, 8_000);
        let bronce = fundicion.nivel(2).unwrap().recetas.iter().find(|r| r.produce == Recurso::LingoteBronce).unwrap();
        assert_eq!(bronce.consume_por_unidad, vec![(Recurso::LingoteCobre, 1_600), (Recurso::LingoteEstano, 400)]);
        assert_eq!(bronce.tecnologia.as_deref(), Some("aleacion_bronce"));
        assert_eq!(fundicion.nivel(3).unwrap().requiere_edificio, None);
        assert_eq!(c.tipo(Tipo::Armeria).nivel(2).unwrap().requiere_edificio, Some((Tipo::Fundicion, 2)));
        assert_eq!(c.tipo(Tipo::Cantera).produccion, Some((Recurso::Piedra, 10_000)));
        assert_eq!(c.tipo(Tipo::Lenera).produccion, Some((Recurso::Madera, 8_000)));
        assert_eq!(c.tipo(Tipo::Mercado).nivel(3).unwrap().cupo_carros, 18);
        assert!(c.tipo(Tipo::CentroUrbano).costo.is_empty());
        assert_eq!(c.tipo(Tipo::Molino).factor_buey, 2_000_000, "`factorBuey` 2 en el molino");
        assert!(TIPOS.iter().filter(|&&t| t != Tipo::Molino).all(|&t| c.tipo(t).factor_buey == UNO), "los demás no tienen buey");
    }

    #[test]
    fn reglas_de_construccion() {
        let r = Reglas::cargar().construccion;
        assert_eq!((r.necesidades.maximo_en_cola, r.necesidades.maximo_en_construccion_simultanea), (4, 2));
        assert_eq!(r.necesidades.maximo_almacenes_por_nivel[&3], 16);
        assert_eq!(r.necesidades.maximo_transformacion_por_nivel[&2], 3);
        assert_eq!((r.bandas.supervivencia, r.bandas.transformacion), (10_000, 500));
        assert_eq!(r.necesidades.umbral_vivienda_ocupada, 850_000);
        assert!(r.unicos.contains(&Tipo::Palacio) && !r.unicos.contains(&Tipo::Vivienda));
        assert_eq!(r.recurso_propio[&Tipo::Granja], Recurso::Trigo);
        assert_eq!((r.horizonte_mantenimiento_min, r.horizonte_comida_min), (8, 8));
    }

    #[test]
    fn la_bodega_y_la_prensa_trabajan_una_receta_con_topes_por_plaza() {
        let r = Reglas::cargar();
        let t = &r.construccion.talleres_de_una_receta;
        assert_eq!(t.keys().copied().collect::<Vec<_>>(), vec![Tipo::Bodega, Tipo::Prensa]);
        assert!(TIPOS.iter().all(|tipo| tipo.de_una_receta() == t.contains_key(tipo)), "`Tipo::de_una_receta` dice lo mismo que el bloque");
        let (bodega, prensa) = (&t[&Tipo::Bodega], &t[&Tipo::Prensa]);
        assert_eq!((bodega.maximo_por_plaza, prensa.maximo_por_plaza), (3, 3));
        // Bebidas: una en los niveles 3 y 4, las dos en el 5; un nivel por debajo del primero usa el primero.
        assert_eq!([1, 2, 3, 4, 5].map(|n| bodega.recetas_distintas(n)), [1, 1, 1, 1, 2]);
        assert_eq!([1, 4, 5].map(|n| prensa.recetas_distintas(n)), [1, 1, 1], "el aceite, de una sola fuente");
        // Las recetas: vino de uva y cerveza de cebada; aceite de aceituna y aceite de sésamo.
        let recetas = |tipo: Tipo| r.catalogo.tipo(tipo).nivel(1).unwrap().recetas.iter().map(|rc| (rc.produce, rc.consume_por_unidad.clone())).collect::<Vec<_>>();
        assert_eq!(recetas(Tipo::Bodega), vec![(Recurso::Vino, vec![(Recurso::Uva, 2_000)]), (Recurso::Cerveza, vec![(Recurso::Cebada, 2_000)])]);
        assert_eq!(recetas(Tipo::Prensa), vec![(Recurso::Aceite, vec![(Recurso::Aceituna, 2_000)]), (Recurso::Aceite, vec![(Recurso::Sesamo, 2_000)])]);
    }
}
