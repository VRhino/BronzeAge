//! Las cifras del sistema 6. Fuente única: los bloques `<!-- parametros:NOMBRE -->` de `Docs/Mechanics/Tecnologia_Aedas.md` (`TECNOLOGIAS`, `ERAS`,
//! `TARIFA_ADOPCION`, `AEDAS`, `EPICAS`, `TITULO_CAPITULO` y `MERCENARIOS.tecnologia`), incrustados en el binario. Los minutos, horas y semanas pasan a
//! ms de mundo; los factores a ppm; los costes a `Costo` en unidades enteras (milésimas múltiplo de 1.000). Las cifras son las del TS (Q3 del autor:
//! no se recalibran hasta la fase 7).
//!
//! El catálogo va **en el orden del documento** (el de `Object.keys(TECNOLOGIAS)` en el TS): serde_json ordena las claves, así que el orden se saca
//! de la posición de cada clave en el bloque.

use crate::contador::Contador;
use crate::{Conjunto, TecId};
use asentamiento::datos::{bloque_de, mil, ppm};
use asentamiento::{Costo, Recurso, Tipo};
use base::{Instante, Mil, MS_POR_MINUTO};
use serde_json::Value;

pub const TECNOLOGIA_AEDAS: &str = include_str!("../../../Docs/Mechanics/Tecnologia_Aedas.md");

pub const HORA: Instante = 60 * MS_POR_MINUTO;
pub const DIA: Instante = 24 * HORA;
pub const SEMANA: Instante = 7 * DIA;

/// 1 punto de reputación en la escala de `comercio::reputacion` (`comercio::datos::PUNTO`; lo iguala una prueba).
pub const PUNTO_REPUTACION: i64 = 1_000_000;

fn bloque(nombre: &str) -> Value {
    bloque_de(TECNOLOGIA_AEDAS, nombre).unwrap_or_else(|e| panic!("{e}"))
}

fn num(v: &Value, ruta: &str) -> f64 {
    ruta.split('.').try_fold(v, |v, k| v.get(k)).and_then(Value::as_f64).unwrap_or_else(|| panic!("falta el número «{ruta}»"))
}

fn minutos(v: &Value, ruta: &str) -> Instante {
    (num(v, ruta) * MS_POR_MINUTO as f64).round() as Instante
}

fn texto(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_else(|| panic!("falta el texto «{k}»")).to_owned()
}

fn edificio(n: &str) -> Tipo {
    Tipo::desde_nombre(n).unwrap_or_else(|| panic!("edificio «{n}» desconocido"))
}

fn recurso(n: &str) -> Recurso {
    Recurso::desde_nombre(n).unwrap_or_else(|| panic!("recurso «{n}» desconocido"))
}

/// Las entradas de un bloque-objeto en el orden en que aparecen en el documento.
fn en_orden_del_documento(nombre: &str) -> Vec<(String, Value)> {
    let desde = TECNOLOGIA_AEDAS.find(&format!("<!-- parametros:{nombre} -->")).expect("bloque");
    let texto = &TECNOLOGIA_AEDAS[desde..];
    let mut v: Vec<(usize, String, Value)> = bloque(nombre)
        .as_object()
        .unwrap_or_else(|| panic!("{nombre} es un objeto"))
        .iter()
        .map(|(k, x)| (texto.find(&format!("\n  \"{k}\":")).unwrap_or_else(|| panic!("{nombre}: no encuentro «{k}»")), k.clone(), x.clone()))
        .collect();
    v.sort_by_key(|e| e.0);
    v.into_iter().map(|(_, k, x)| (k, x)).collect()
}

/// Una condición del hito (`CondicionHito`, TA §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condicion {
    /// Algún asentamiento de la Facción con ese edificio activo y `nivelInterno ≥ nivel` (1 por defecto).
    Edificio { tipo: Tipo, nivel: u8 },
    /// La tecnología está **adoptada**.
    Tecnologia(TecId),
    /// El almacén de la capital tiene más de 0 de ese recurso.
    RecursoEnCapital(Recurso),
    /// La capital tiene nivel actual ≥ `nivel` y ese edificio activo.
    CapitalEnNivel { nivel: u8, con: Tipo },
    /// Alguna zona de una plaza de la Facción contiene un nodo de ese recurso (con o sin stock).
    YacimientoEnTerritorio(Recurso),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Logro {
    pub contador: Contador,
    pub umbral: u64,
}

/// `bonusProduccion`: el factor (ppm) que multiplica lo que sacan los extractores de ese recurso a la Facción que la tiene adoptada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bonus {
    pub recurso: Recurso,
    pub factor: i64,
}

/// El tipo de hecho que pide un capítulo de épica (`HechoEpico`). El orden es el de `TITULO_CAPITULO`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HechoEpico {
    Defensa,
    Batalla,
    Conquista,
    Obra,
    Ascenso,
    Adopcion,
    Caravana,
}

impl HechoEpico {
    pub const TODOS: [HechoEpico; 7] = [HechoEpico::Defensa, HechoEpico::Batalla, HechoEpico::Conquista, HechoEpico::Obra, HechoEpico::Ascenso, HechoEpico::Adopcion, HechoEpico::Caravana];

    pub fn nombre(self) -> &'static str {
        ["defensa", "batalla", "conquista", "obra", "ascenso", "adopcion", "caravana"][self as usize]
    }

    pub fn desde_nombre(n: &str) -> Option<HechoEpico> {
        HechoEpico::TODOS.into_iter().find(|h| h.nombre() == n)
    }
}

/// `cap(hecho, cantidad, edificio?)`: cada capítulo pide `cantidad` hechos de un tipo (y, para `obra`, de un edificio).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capitulo {
    pub hecho: HechoEpico,
    pub cantidad: u32,
    pub edificio: Option<Tipo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinicionEpica {
    pub nombre: String,
    pub capitulos: Vec<Capitulo>,
}

/// Una entrada del catálogo (`DefinicionTecnologia`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definicion {
    pub id: String,
    pub nombre: String,
    /// Índice de su Era en [`Reglas::eras`].
    pub era: u8,
    pub de_arranque: bool,
    pub logro: Option<Logro>,
    pub hito: Vec<Condicion>,
    pub bonus: Option<Bonus>,
    pub epica: Option<DefinicionEpica>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Era {
    pub id: String,
    pub nombre: String,
    pub orden: u32,
    /// `plazoSemanas` en ms: el techo de la Era (TA §3.2).
    pub plazo: Instante,
    /// `minimoSemanas` en ms: el suelo (decisión del autor, 2026-10-25): con todos sus logros, la Era no pasa antes.
    pub minimo: Instante,
    /// `TARIFA_ADOPCION[era]`: lo que paga la capital por adoptar una tecnología de esta Era.
    pub tarifa: Costo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasAedas {
    /// `retrasoConocimientoMinutos` (ms): la ventaja del primero (TA §6.1).
    pub retraso_conocimiento: Instante,
    pub facciones_por_aeda: u32,
    pub minimo_itinerantes: u32,
    /// `estanciaMinutos` (ms).
    pub estancia: Instante,
    /// `velocidad` (milésimas por minuto, la escala de las columnas; la del TS, Q3).
    pub velocidad: Mil,
    pub memoria_visitas: usize,
    /// `cupoPorNivel`, por nivel actual (índice = nivel; 0 donde el bloque no da cupo).
    pub cupo_por_nivel: Vec<u32>,
    /// `llegadaMinutos` (ms).
    pub llegada: Instante,
    /// `umbralReputacionBaja` en la escala de `comercio::reputacion`.
    pub umbral_reputacion_baja: i64,
    pub factor_llegada_reputacion_baja: i64,
    /// `bonoFelicidad` y `bonoNobleza` por residente (ppm).
    pub bono_felicidad: i64,
    pub bono_nobleza: i64,
    /// `epica.enfriamientoMinutos` (ms).
    pub enfriamiento_epica: Instante,
    /// `epica.cargaMinimaCaravana` (milésimas: la carga total de una caravana comercial que llega).
    pub carga_minima_caravana: Mil,
    pub claves_recordadas: usize,
    /// `venta.factorOro`: el precio de venta es el oro de la tarifa de su Era por esto.
    pub factor_oro_venta: i64,
    pub orden_era_maximo_venta: u32,
}

/// `MERCENARIOS.tecnologia` (TA §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReglasCampamento {
    /// `porcentajeFacciones` (ppm).
    pub porcentaje_facciones: i64,
    /// `horasTrasLaPrimera` (ms).
    pub tras_la_primera: Instante,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reglas {
    /// El catálogo en el orden del documento: el índice es el [`TecId`].
    pub catalogo: Vec<Definicion>,
    /// Las Eras por su `orden` (el índice 0 es la I).
    pub eras: Vec<Era>,
    pub aedas: ReglasAedas,
    pub campamento: ReglasCampamento,
    /// `TITULO_CAPITULO`, por [`HechoEpico`].
    pub titulos_capitulo: Vec<String>,
    /// Las de arranque (`deArranque`): toda Facción las tiene aparecidas y adoptadas desde que nace.
    pub de_arranque: Conjunto,
    /// Los logros de cada contador ordenados por umbral (a igual umbral, por índice): el siguiente pendiente se mira en O(1).
    pub por_contador: Vec<Vec<(u64, TecId)>>,
    /// Las que piden `recursoEnCapital` en su hito: lo único que el paso de minuto de una capital mira (plan §5.1), sin recorrer el catálogo.
    pub piden_capital: Conjunto,
    /// Las que dan un `bonusProduccion`: el `factor_tecnologia` de una Facción que no ha adoptado ninguna es 1 sin mirar nada más.
    pub con_bonus: Conjunto,
}

impl Reglas {
    pub fn cargar() -> Self {
        let eras: Vec<(String, Value)> = {
            let mut v = en_orden_del_documento("ERAS");
            v.sort_by_key(|(_, e)| num(e, "orden") as u32);
            v
        };
        let tarifas = bloque("TARIFA_ADOPCION");
        let eras: Vec<Era> = eras
            .into_iter()
            .map(|(id, e)| Era {
                tarifa: tarifas[&id].as_object().unwrap_or_else(|| panic!("falta la tarifa de «{id}»")).iter().map(|(r, n)| (recurso(r), mil(n.as_f64().expect("número")))).collect(),
                nombre: texto(&e, "nombre"),
                orden: num(&e, "orden") as u32,
                plazo: (num(&e, "plazoSemanas") * SEMANA as f64).round() as Instante,
                minimo: (num(&e, "minimoSemanas") * SEMANA as f64).round() as Instante,
                id,
            })
            .collect();
        let entradas = en_orden_del_documento("TECNOLOGIAS");
        assert!(entradas.len() <= 64, "el Conjunto admite 64 tecnologías");
        let indice = |id: &str| entradas.iter().position(|(k, _)| k == id).unwrap_or_else(|| panic!("tecnología «{id}» desconocida")) as TecId;
        let epicas = bloque("EPICAS");
        let catalogo: Vec<Definicion> = entradas
            .iter()
            .map(|(id, t)| Definicion {
                nombre: texto(t, "nombre"),
                era: eras.iter().position(|e| e.id == texto(t, "era")).unwrap_or_else(|| panic!("{id}: Era desconocida")) as u8,
                de_arranque: t["deArranque"].as_bool() == Some(true),
                logro: t.get("logro").map(|l| Logro {
                    contador: Contador::desde_nombre(&texto(l, "contador")).unwrap_or_else(|| panic!("{id}: contador desconocido")),
                    umbral: num(l, "umbral") as u64,
                }),
                hito: t["hito"].as_array().expect("hito").iter().map(|c| condicion(c, &indice)).collect(),
                bonus: t.get("bonusProduccion").map(|b| Bonus { recurso: recurso(&texto(b, "recurso")), factor: ppm(num(b, "factor")) }),
                epica: epicas.get(id).map(|e| DefinicionEpica {
                    nombre: texto(e, "nombre"),
                    capitulos: e["capitulos"]
                        .as_array()
                        .expect("capitulos")
                        .iter()
                        .map(|c| Capitulo {
                            hecho: HechoEpico::desde_nombre(&texto(c, "hecho")).unwrap_or_else(|| panic!("{id}: hecho desconocido")),
                            cantidad: num(c, "cantidad") as u32,
                            edificio: c.get("edificio").and_then(Value::as_str).map(edificio),
                        })
                        .collect(),
                }),
                id: id.clone(),
            })
            .collect();
        let a = bloque("AEDAS");
        let cupo = &a["residentes"]["cupoPorNivel"];
        let nivel_maximo = cupo.as_object().expect("cupoPorNivel").keys().map(|k| k.parse::<usize>().expect("nivel")).max().unwrap_or(0);
        let titulos = bloque("TITULO_CAPITULO");
        let me = bloque("MERCENARIOS");
        let mut r = Reglas {
            de_arranque: Conjunto::VACIO,
            piden_capital: Conjunto::VACIO,
            con_bonus: Conjunto::VACIO,
            por_contador: Vec::new(),
            catalogo,
            eras,
            aedas: ReglasAedas {
                retraso_conocimiento: minutos(&a, "retrasoConocimientoMinutos"),
                facciones_por_aeda: num(&a, "itinerantes.faccionesPorAeda") as u32,
                minimo_itinerantes: num(&a, "itinerantes.minimo") as u32,
                estancia: minutos(&a, "itinerantes.estanciaMinutos"),
                velocidad: mil(num(&a, "itinerantes.velocidad")),
                memoria_visitas: num(&a, "itinerantes.memoriaVisitas") as usize,
                cupo_por_nivel: (0..=nivel_maximo).map(|n| cupo.get(n.to_string()).and_then(Value::as_u64).unwrap_or(0) as u32).collect(),
                llegada: minutos(&a, "residentes.llegadaMinutos"),
                umbral_reputacion_baja: (num(&a, "residentes.umbralReputacionBaja") * PUNTO_REPUTACION as f64).round() as i64,
                factor_llegada_reputacion_baja: num(&a, "residentes.factorLlegadaReputacionBaja") as i64,
                bono_felicidad: ppm(num(&a, "residentes.bonoFelicidad")),
                bono_nobleza: ppm(num(&a, "residentes.bonoNobleza")),
                enfriamiento_epica: minutos(&a, "epica.enfriamientoMinutos"),
                carga_minima_caravana: mil(num(&a, "epica.cargaMinimaCaravana")),
                claves_recordadas: num(&a, "epica.clavesRecordadas") as usize,
                factor_oro_venta: num(&a, "venta.factorOro") as i64,
                orden_era_maximo_venta: num(&a, "venta.ordenEraMaximo") as u32,
            },
            campamento: ReglasCampamento {
                porcentaje_facciones: ppm(num(&me, "tecnologia.porcentajeFacciones")),
                tras_la_primera: (num(&me, "tecnologia.horasTrasLaPrimera") * HORA as f64).round() as Instante,
            },
            titulos_capitulo: HechoEpico::TODOS.iter().map(|h| texto(&titulos, h.nombre())).collect(),
        };
        r.indexar();
        r
    }

    /// Rehace lo que se deriva del catálogo (las de arranque, las que piden la capital, las que dan bonus y los logros de cada contador): tras leerlo y en las pruebas que lo cambian.
    pub fn indexar(&mut self) {
        let de = |f: &dyn Fn(&Definicion) -> bool| self.catalogo.iter().enumerate().filter(|(_, d)| f(d)).map(|(i, _)| i as TecId).collect::<Conjunto>();
        (self.de_arranque, self.piden_capital, self.con_bonus) =
            (de(&|d| d.de_arranque), de(&|d| d.hito.iter().any(|c| matches!(c, Condicion::RecursoEnCapital(_)))), de(&|d| d.bonus.is_some()));
        let mut por_contador = vec![Vec::new(); Contador::TODOS.len()];
        for (i, d) in self.catalogo.iter().enumerate() {
            if let Some(l) = d.logro {
                por_contador[l.contador.indice()].push((l.umbral, i as TecId));
            }
        }
        por_contador.iter_mut().for_each(|v| v.sort());
        self.por_contador = por_contador;
    }

    /// Todo el catálogo como conjunto.
    pub fn todas(&self) -> Conjunto {
        Conjunto(u64::MAX >> (64 - self.catalogo.len()))
    }

    /// El índice de una tecnología por su id de cadena (el del contrato).
    pub fn buscar(&self, id: &str) -> Option<TecId> {
        self.catalogo.iter().position(|d| d.id == id).map(|i| i as TecId)
    }

    pub fn def(&self, t: TecId) -> &Definicion {
        &self.catalogo[t as usize]
    }

    /// El `orden` de la Era de la tecnología.
    pub fn orden_era(&self, t: TecId) -> u32 {
        self.eras[self.def(t).era as usize].orden
    }

    pub fn tarifa(&self, t: TecId) -> &Costo {
        &self.eras[self.def(t).era as usize].tarifa
    }
}

impl Default for Reglas {
    fn default() -> Self {
        static REGLAS: std::sync::OnceLock<Reglas> = std::sync::OnceLock::new();
        REGLAS.get_or_init(Reglas::cargar).clone()
    }
}

fn condicion(c: &Value, indice: &impl Fn(&str) -> TecId) -> Condicion {
    let nivel = |k: &str| c.get(k).and_then(Value::as_u64).unwrap_or(1) as u8;
    match texto(c, "tipo").as_str() {
        "edificio" => Condicion::Edificio { tipo: edificio(&texto(c, "edificio")), nivel: nivel("nivelInterno") },
        "tecnologia" => Condicion::Tecnologia(indice(&texto(c, "id"))),
        "recursoEnCapital" => Condicion::RecursoEnCapital(recurso(&texto(c, "recurso"))),
        "capitalEnNivel" => Condicion::CapitalEnNivel { nivel: nivel("nivel"), con: edificio(&texto(c, "conEdificio")) },
        "yacimientoEnTerritorio" => Condicion::YacimientoEnTerritorio(recurso(&texto(c, "recurso"))),
        otro => panic!("condición de hito «{otro}» desconocida"),
    }
}

/// Las tecnologías del juego por su id, para quien solo mira «¿existe?» y «¿es de arranque?» (la economía, el militar y el núcleo). Vivía en
/// `militar::datos` desde 4B-1; `militar` la reexporta. Mismo orden que entonces (alfabético) para no cambiar nada aguas abajo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasTecnologias {
    pub todas: Vec<String>,
    pub de_arranque: Vec<String>,
}

impl ReglasTecnologias {
    pub fn cargar() -> Self {
        let t = bloque("TECNOLOGIAS");
        let todas = t.as_object().expect("TECNOLOGIAS es un objeto");
        ReglasTecnologias { todas: todas.keys().cloned().collect(), de_arranque: todas.iter().filter(|(_, v)| v["deArranque"].as_bool() == Some(true)).map(|(k, _)| k.clone()).collect() }
    }
}

#[cfg(test)]
mod pruebas {
    //! El catálogo contra los invariantes de TA §12 y los casos de catálogo de `typescript-final:src/engine/__tests__/tecnologia.test.ts` y
    //! `aedasResidentes.test.ts` («catálogo de épicas»).
    use super::*;

    #[test]
    fn las_cifras_son_las_del_documento() {
        let r = Reglas::cargar();
        assert_eq!(r.catalogo.len(), 26);
        assert_eq!((r.catalogo[0].id.as_str(), r.catalogo[1].id.as_str(), r.catalogo[2].id.as_str()), ("leva_comunal", "hostigamiento_tribal", "metalurgia_cobre"), "orden del documento");
        assert_eq!(r.eras.iter().map(|e| (e.id.as_str(), e.orden, e.plazo)).collect::<Vec<_>>(), vec![("reinos_palaciales", 1, 5 * SEMANA), ("crisis_adaptacion", 2, 6 * SEMANA), ("polis_imperios", 3, 7 * SEMANA)]);
        assert!(r.eras.iter().all(|e| e.minimo == SEMANA), "cada Era dura al menos una semana");
        assert_eq!(r.eras[0].tarifa, vec![(Recurso::Madera, 200_000), (Recurso::Oro, 100_000)]);
        assert_eq!(r.eras[2].tarifa, vec![(Recurso::LingoteHierro, 30_000), (Recurso::Oro, 600_000)]);
        let a = &r.aedas;
        assert_eq!((a.retraso_conocimiento, a.estancia, a.llegada, a.enfriamiento_epica), (DIA, 6 * HORA, DIA, 6 * HORA));
        assert_eq!((a.facciones_por_aeda, a.minimo_itinerantes, a.velocidad, a.memoria_visitas), (3, 3, 20_000, 4));
        assert_eq!(a.cupo_por_nivel, vec![0, 0, 1, 2, 3, 4]);
        assert_eq!((a.umbral_reputacion_baja, a.factor_llegada_reputacion_baja, a.bono_felicidad, a.bono_nobleza), (-50 * PUNTO_REPUTACION, 3, 50_000, 250_000));
        assert_eq!((a.carga_minima_caravana, a.claves_recordadas, a.factor_oro_venta, a.orden_era_maximo_venta), (100_000, 50, 2, 3));
        assert_eq!((r.campamento.porcentaje_facciones, r.campamento.tras_la_primera), (500_000, 72 * HORA));
        assert_eq!(r.titulos_capitulo[HechoEpico::Obra as usize], "La obra de las manos");
        let canteria = r.def(r.buscar("canteria").unwrap());
        assert_eq!(canteria.bonus, Some(Bonus { recurso: Recurso::Piedra, factor: 1_500_000 }));
        assert_eq!(canteria.logro, Some(Logro { contador: Contador::ExtraidoPiedra, umbral: 11_000_000 }));
        let bronce = r.def(r.buscar("aleacion_bronce").unwrap());
        assert_eq!(bronce.hito, vec![Condicion::Edificio { tipo: Tipo::Fundicion, nivel: 2 }, Condicion::RecursoEnCapital(Recurso::Estano)]);
        let civicas = r.def(r.buscar("instituciones_civicas").unwrap());
        assert_eq!(civicas.hito, vec![Condicion::CapitalEnNivel { nivel: 3, con: Tipo::Mercado }]);
        assert_eq!(r.de_arranque, [r.buscar("leva_comunal").unwrap(), r.buscar("hostigamiento_tribal").unwrap()].into_iter().collect());
        assert_eq!(comercio::datos::PUNTO, PUNTO_REPUTACION);
    }

    #[test]
    fn solo_las_de_arranque_van_sin_logro_y_las_demas_con_umbral_positivo() {
        let r = Reglas::cargar();
        for d in &r.catalogo {
            assert_eq!(d.de_arranque, d.logro.is_none(), "{}", d.id);
            assert!(d.logro.is_none_or(|l| l.umbral > 0), "{}", d.id);
        }
        let mut arranque: Vec<&str> = r.de_arranque.iter().map(|t| r.def(t).id.as_str()).collect();
        arranque.sort();
        assert_eq!(arranque, ["hostigamiento_tribal", "leva_comunal"]);
    }

    #[test]
    fn un_hito_solo_pide_tecnologias_de_su_era_o_anteriores_y_cada_era_tiene_tarifa() {
        let r = Reglas::cargar();
        for (i, d) in r.catalogo.iter().enumerate() {
            for c in &d.hito {
                if let Condicion::Tecnologia(o) = *c {
                    assert!(r.orden_era(o) <= r.orden_era(i as TecId), "{} pide {}", d.id, r.def(o).id);
                }
            }
        }
        assert!(r.eras.iter().all(|e| e.tarifa.iter().any(|&(rec, n)| rec == Recurso::Oro && n > 0)));
    }

    #[test]
    fn toda_tecnologia_con_logro_tiene_epica_de_3_4_o_5_capitulos_segun_su_era() {
        let r = Reglas::cargar();
        for d in &r.catalogo {
            if d.de_arranque {
                assert!(d.epica.is_none(), "{}", d.id);
                continue;
            }
            let e = d.epica.as_ref().unwrap_or_else(|| panic!("{} sin épica", d.id));
            assert_eq!(e.capitulos.len(), 3 + d.era as usize, "{}", d.id);
            for c in &e.capitulos {
                assert!(c.cantidad > 0);
                assert_eq!(c.hecho == HechoEpico::Obra, c.edificio.is_some(), "{}: solo las obras llevan edificio", d.id);
            }
        }
    }

    #[test]
    fn los_logros_de_cada_contador_van_por_umbral() {
        let r = Reglas::cargar();
        assert_eq!(r.por_contador.iter().map(Vec::len).sum::<usize>(), 24, "un logro por tecnología sin arranque, uno por contador");
        assert!(r.por_contador.iter().all(|v| v.len() == 1));
        assert_eq!(r.por_contador[Contador::ExtraidoCobre.indice()], vec![(41_000, r.buscar("metalurgia_cobre").unwrap())]);
        assert!(r.por_contador.iter().all(|v| v.windows(2).all(|w| w[0] <= w[1])));
    }

    #[test]
    fn reglas_tecnologias_de_siempre() {
        let t = ReglasTecnologias::cargar();
        assert_eq!(t.de_arranque, vec!["hostigamiento_tribal".to_owned(), "leva_comunal".to_owned()]);
        assert_eq!(t.todas.len(), 26);
    }
}
