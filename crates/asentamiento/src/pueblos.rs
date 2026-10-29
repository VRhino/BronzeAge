//! Pueblos extractores y obras de mapa (Economia_Construccion §14, Doc 4.2.2). Un pueblo es el extractor de mapa de siempre (Cantera o mina)
//! plantado sobre su nodo, o una **aldea agrícola** (§14.2, mecánica 50) plantada en un punto de la zona sin nodo, con sus granjas y sus
//! corrales; los dos llevan un almacén local y un carro de acarreo (que vive en la flota, crate `comercio`). **No es un asentamiento**: sin
//! zona, nivel de asentamiento, cargos ni medidor. Es un elemento de su plaza madre, que paga su mantenimiento (cuenta como un edificio activo)
//! y le pone los trabajadores (sus pesants); por eso no está entre los `edificios` de la ciudad, que son del interior.
//!
//! # La obra de mapa
//! No pasa por la cola de construcción. Se marca sobre un nodo (o, la aldea, sobre un punto) y queda **pendiente de materiales** (ocupa cupo
//! desde ese momento); los materiales se entregan en el sitio, y con todo entregado arranca una obra corta (la del extractor del catálogo; la
//! de la aldea, la de su nivel 1). Al acabar, el pueblo queda activo. El fin de obra se resuelve en el paso de minuto de la plaza (precisión
//! de un minuto, sin evento propio). La aldea sube de nivel con otra obra de mapa igual (`mejora`) mientras sigue produciendo.
//!
//! # La extracción
//! Un minuto de extracción es `produccionBase × ratioManoObra × factorTecnología` contra el almacén **local**; con el almacén lleno se para
//! (y avisa una sola vez, `PuebloLleno`). Un nodo agotado espera a regenerarse. Un pueblo no entra en el reemplazo automático ni en
//! `EXTRACCION_MAXIMOS`. La aldea cultiva trigo (sus granjas, con la fertilidad de su punto) y cría su **rebaño** en los corrales (mecánica
//! 51): las cabezas crecen cada minuto hasta el tope de sus corrales y lo que pasa del pie de cría se sacrifica y entra como ganado. Trigo y
//! ganado van contra un almacén local **compartido**: la capacidad cuenta los dos juntos, y el carro se lleva los dos. El carro, el viaje y la
//! llegada son del núcleo.

use crate::almacen::Almacen;
use crate::asentamiento::{Asentamiento, Evento};
use crate::catalogo::{costo, Tipo};
use crate::datos::{mil, por, ppm, Reglas};
use crate::produccion::{Contexto, Fuentes, Mano};
use crate::recurso::{Costo, Recurso};
use base::{Instante, Mil, Punto, MS_POR_MINUTO};
use serde_json::Value;
use std::collections::BTreeMap;

/// Un nivel de la aldea agrícola: cuántas granjas y corrales tiene y lo que cuesta llegar a él (el 1 es la obra de la aldea).
#[derive(Clone, Debug)]
pub struct NivelAldea {
    pub granjas: u32,
    pub corrales: u32,
    /// Lo que pide la obra de ese nivel (en el 1, además del coste adicional y del carro con su buey).
    pub coste: Costo,
    pub obra_minutos: i64,
}

/// Una especie de rebaño (`ESPECIES`, §14.3): por corral, cuántas cabezas caben y cuántas se guardan para criar; cuánto crece el rebaño por hora
/// y lo que da cada cabeza sacrificada. Las cabezas van en milésimas.
#[derive(Clone, Debug)]
pub struct Especie {
    pub tope_por_corral: Mil,
    pub pie_por_corral: Mil,
    /// Crecimiento por hora, en ppm de las cabezas que hay.
    pub crecimiento_por_hora: i64,
    /// Lo que da cada cabeza sacrificada.
    pub productos: Costo,
}

/// `PUEBLOS_EXTRACTORES.aldea`: las cifras de la aldea agrícola (§14.2).
#[derive(Clone, Debug)]
pub struct ReglasAldea {
    /// Nivel mínimo de la plaza madre para levantar una aldea.
    pub nivel_madre_minimo: u32,
    /// Fertilidad mínima del terreno en el punto (ppm).
    pub fertilidad_minima: i64,
    /// Distancia mínima (milésimas de unidad) hasta cualquier otro pueblo.
    pub separacion_minima: Mil,
    /// Guarnición de serie: granjeros sin dueño de la IA, fuera de cupo (se reponen al ritmo de los mineros).
    pub granjeros: u32,
    pub niveles: BTreeMap<u32, NivelAldea>,
    /// La especie con la que nace el rebaño de sus corrales.
    pub especie_inicial: String,
    /// Parte de las cabezas que se lleva quien gana el saqueo de la aldea (ppm).
    pub saqueo_rebano: i64,
    /// `ESPECIES`, por id.
    pub especies: BTreeMap<String, Especie>,
}

impl ReglasAldea {
    pub fn nivel_maximo(&self) -> u32 {
        self.niveles.keys().next_back().copied().unwrap_or(1)
    }

    pub fn especie(&self, id: &str) -> &Especie {
        self.especies.get(id).unwrap_or_else(|| panic!("no hay especie «{id}» en ESPECIES"))
    }

    fn nivel(&self, n: u32) -> &NivelAldea {
        self.niveles.get(&n).unwrap_or_else(|| panic!("la aldea no tiene nivel {n} en PUEBLOS_EXTRACTORES"))
    }
}

/// `PUEBLOS_EXTRACTORES`: las cifras de los pueblos y sus obras. El carro con su buey no está aquí: su coste es el de la flota
/// (`comercio::acarreo::coste_reposicion`) y lo pone quien llama.
#[derive(Clone, Debug)]
pub struct ReglasPueblos {
    /// Nivel mínimo de la plaza madre para un extractor (el de la aldea está en `aldea`).
    pub nivel_madre_minimo: u32,
    /// Pueblos que admite la madre por nivel, de cualquier tipo (1 / 3 / 4 / 5 / 6).
    pub cupo_por_nivel: BTreeMap<u32, u32>,
    /// Capacidad del almacén local, por recurso en un extractor (que extrae uno solo) y en total en la aldea.
    pub capacidad_almacen: Mil,
    /// Lo que la obra cuesta además del extractor (o del nivel 1 de la aldea) y del carro con su buey.
    pub coste_adicional: Costo,
    /// Casas de la planta del pueblo (presentación, `nucleo::urbanismo::planta_de_pueblo`).
    pub casas_de_la_planta: u32,
    /// Guarnición de serie: mineros sin dueño de la IA, fuera de cupo (el sistema militar los pondrá).
    pub mineros: u32,
    /// Minutos que tarda en volver cada minero (o granjero) de esa guarnición, uno a uno, desde la primera baja.
    pub minutos_por_minero: i64,
    /// Guarnición de jugadores (§14.0.1): cupo de Liderazgo **por héroe** en un pueblo de nivel 1 (un extractor siempre lo es) y lo que suma cada nivel más de la aldea.
    pub cupo_guarnicion: i64,
    pub cupo_guarnicion_por_nivel: i64,
    pub aldea: ReglasAldea,
}

impl ReglasPueblos {
    /// El cupo de la guarnición de jugadores de un pueblo de nivel `nivel`, por héroe, en puntos de Liderazgo.
    pub fn cupo_de_guarnicion(&self, nivel: u32) -> i64 {
        self.cupo_guarnicion + i64::from(nivel.saturating_sub(1)) * self.cupo_guarnicion_por_nivel
    }

    /// `b` es `PUEBLOS_EXTRACTORES`; `especies`, `ESPECIES`.
    pub fn de(b: &Value, especies: &Value) -> Self {
        let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).unwrap_or_else(|| panic!("falta «{k}» en PUEBLOS_EXTRACTORES"));
        let a = &b["aldea"];
        let aldea = ReglasAldea {
            nivel_madre_minimo: num(&b["nivelMadreMinimo"], "aldea") as u32,
            fertilidad_minima: ppm(num(a, "fertilidadMinima")),
            separacion_minima: mil(num(a, "separacionMinima")),
            granjeros: num(&a["granjeros"], "escuadra") as u32,
            niveles: a["niveles"]
                .as_object()
                .expect("aldea.niveles")
                .iter()
                .map(|(k, v)| {
                    let n = NivelAldea { granjas: num(v, "granjas") as u32, corrales: num(v, "corrales") as u32, coste: costo(v.get("coste")), obra_minutos: num(v, "obraMinutos") as i64 };
                    (k.parse().expect("nivel"), n)
                })
                .collect(),
            especie_inicial: a["especieInicial"].as_str().expect("aldea.especieInicial").to_owned(),
            saqueo_rebano: ppm(num(a, "saqueoRebano")),
            especies: especies
                .as_object()
                .expect("ESPECIES")
                .iter()
                .map(|(k, v)| {
                    let e = Especie {
                        tope_por_corral: mil(num(v, "topePorCorral")),
                        pie_por_corral: mil(num(v, "pieDeCriaPorCorral")),
                        crecimiento_por_hora: ppm(num(v, "crecimientoPorHora")),
                        productos: costo(v.get("productos")),
                    };
                    (k.clone(), e)
                })
                .collect(),
        };
        Self {
            nivel_madre_minimo: num(&b["nivelMadreMinimo"], "extractor") as u32,
            cupo_por_nivel: b["cupoPorNivel"].as_object().expect("cupoPorNivel").iter().map(|(k, v)| (k.parse().expect("nivel"), v.as_u64().expect("cupo") as u32)).collect(),
            capacidad_almacen: mil(num(b, "capacidadAlmacenLocal")),
            coste_adicional: costo(b.get("costeAdicional")),
            casas_de_la_planta: num(b, "casasDeLaPlanta") as u32,
            mineros: num(&b["mineros"], "escuadra") as u32,
            minutos_por_minero: num(&b["mineros"], "minutosPorMinero") as i64,
            cupo_guarnicion: num(&b["guarnicionJugadores"], "cupo") as i64,
            cupo_guarnicion_por_nivel: num(&b["guarnicionJugadores"], "porNivel") as i64,
            aldea,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EstadoPueblo {
    /// «Pendiente de materiales»: lo que pide la obra y lo que ya se entregó (los mismos recursos, en el mismo orden).
    Pendiente { requerido: Costo, entregado: Costo },
    /// Con todo entregado, levantándose hasta `completa_en`.
    EnObra { completa_en: Instante },
    Activo,
}

/// El rebaño de una aldea: su especie (id de `ESPECIES`) y sus cabezas, en milésimas (crece de minuto en minuto).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rebano {
    pub especie: String,
    pub cabezas: Mil,
}

/// Dónde está plantado un pueblo: un extractor, sobre su nodo; la aldea agrícola, en un punto de la zona, con la fertilidad del terreno en
/// ese punto (ppm, la que manda en sus granjas; el terreno no cambia, así que se lee una vez al marcarla).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sitio {
    Nodo(u32),
    Punto { pos: Punto, fertilidad: i64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pueblo {
    /// Id de mundo (el mismo contador que ejércitos y plazas).
    pub id: u64,
    /// El extractor del catálogo; la aldea agrícola es `Granja` (sobre un punto).
    pub tipo: Tipo,
    pub sitio: Sitio,
    /// Nivel de la aldea (1 al levantarse); un extractor es siempre 1.
    pub nivel: u32,
    pub estado: EstadoPueblo,
    /// La subida de nivel de una aldea activa (`Pendiente` o `EnObra`; nunca `Activo`): la aldea sigue produciendo con su nivel de antes.
    pub mejora: Option<EstadoPueblo>,
    /// El rebaño de los corrales de una aldea activa (nace con la aldea); `None` en un extractor o mientras la aldea es una obra.
    pub rebano: Option<Rebano>,
    /// Lo extraído, a la espera del carro. Vacío mientras no está activo.
    pub almacen: Almacen,
    /// La caravana de acarreo (id de la flota); `None` mientras no tiene carro.
    pub carro: Option<u64>,
    /// El almacén se llenó y la extracción está parada (para avisar solo al llenarse).
    pub lleno: bool,
}

impl Pueblo {
    pub fn activo(&self) -> bool {
        self.estado == EstadoPueblo::Activo
    }

    /// ¿Es una aldea agrícola?
    pub fn es_aldea(&self) -> bool {
        matches!(self.sitio, Sitio::Punto { .. })
    }

    /// El nombre del tipo de pueblo que ven los clientes: el id de catálogo del extractor, o `aldeaAgricola`.
    pub fn nombre_de_tipo(&self) -> &'static str {
        if self.es_aldea() { "aldeaAgricola" } else { self.tipo.nombre() }
    }

    /// El nodo sobre el que está (un extractor).
    pub fn nodo(&self) -> Option<u32> {
        match self.sitio {
            Sitio::Nodo(n) => Some(n),
            Sitio::Punto { .. } => None,
        }
    }

    /// El recurso principal: el que extrae un extractor; el trigo, en la aldea.
    pub fn recurso(&self, r: &Reglas) -> Recurso {
        if self.es_aldea() {
            return Recurso::Trigo;
        }
        r.catalogo.tipo(self.tipo).produccion.expect("un extractor produce un recurso").0
    }

    /// Lo que guarda el almacén local: el recurso del extractor, o trigo y ganado en la aldea.
    pub fn recursos(&self, r: &Reglas) -> Vec<Recurso> {
        if self.es_aldea() {
            vec![Recurso::Trigo, Recurso::Livestock]
        } else {
            vec![self.recurso(r)]
        }
    }

    /// Lo guardado en el almacén local (unidades enteras, milésimas), sumando sus recursos.
    pub fn existencias(&self, r: &Reglas) -> Mil {
        self.recursos(r).into_iter().map(|x| self.almacen.cantidad(x)).sum()
    }

    /// Lo que se lleva el carro de capacidad `capacidad` si sale ahora: el almacén local hasta llenarlo, recurso a recurso.
    pub fn carga_del_carro(&self, r: &Reglas, capacidad: Mil) -> Costo {
        let mut queda = capacidad;
        let mut carga = Costo::new();
        for x in self.recursos(r) {
            let n = self.almacen.cantidad(x).min(queda);
            if n > 0 {
                carga.push((x, n));
                queda -= n;
            }
        }
        carga
    }

    /// Lo que todavía falta entregar a una obra pendiente, la del pueblo o la de la subida de nivel de la aldea (vacío si no hay ninguna o ya
    /// está todo).
    pub fn falta(&self) -> Costo {
        match (&self.estado, &self.mejora) {
            (EstadoPueblo::Pendiente { requerido, entregado }, _) | (_, Some(EstadoPueblo::Pendiente { requerido, entregado })) => falta_de(requerido, entregado),
            _ => vec![],
        }
    }
}

/// Lo que falta de `requerido` por haberse entregado `entregado` (los mismos recursos, en el mismo orden): solo lo que aún pide la obra.
pub(crate) fn falta_de(requerido: &Costo, entregado: &Costo) -> Costo {
    requerido.iter().zip(entregado).map(|(&(rec, pide), &(_, hay))| (rec, pide - hay)).filter(|&(_, n)| n > 0).collect()
}

/// Toma de `disponible` lo que falta a una obra de mapa, recurso a recurso, y lo anota en `entregado`. Devuelve lo tomado, para que quien entrega lo
/// quite de donde lo llevaba. Es la entrega de materiales de un pueblo y de una avanzada.
pub(crate) fn tomar_materiales(requerido: &Costo, entregado: &mut Costo, disponible: &[(Recurso, Mil)]) -> Costo {
    let mut tomado = Costo::new();
    for (&(rec, pide), (_, hay)) in requerido.iter().zip(entregado.iter_mut()) {
        let ofrece = disponible.iter().filter(|&&(d, _)| d == rec).map(|&(_, n)| n).sum::<Mil>();
        let toma = (pide - *hay).min(ofrece);
        if toma > 0 {
            *hay += toma;
            tomado.push((rec, toma));
        }
    }
    tomado
}

/// Horas de la ventana con la que el acarreo cuenta en la solvencia del ascenso (Economia_Construccion §11).
pub const HORAS_VENTANA: i64 = 24;

/// Lo que el acarreo ingresó en el almacén de la plaza durante una hora de mundo. La ventana de 24 h son los cubos con `hora` en
/// `(ahora − 24, ahora]`: entre 23 y 24 horas de historia, la hora en curso incluida.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IngresoAcarreo {
    pub recurso: Recurso,
    /// Hora de mundo (`instante / 3 600 000`).
    pub hora: i64,
    pub cantidad: Mil,
}

fn hora_de(t: Instante) -> i64 {
    t.div_euclid(60 * MS_POR_MINUTO)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorPueblo {
    /// Solo van por pueblo los minerales y la Cantera.
    TipoNoPueblo,
    TecnologiaFaltante,
    NivelInsuficiente,
    SinCupo,
    NoExiste,
    NoEstaPendiente,
    /// Lo que se ofrece no sirve a lo que falta.
    NadaQueEntregar,
    /// El punto de la aldea no llega a la fertilidad mínima.
    FertilidadInsuficiente,
    /// La subida de nivel es de las aldeas.
    NoEsAldea,
    /// La aldea ya está en su último nivel.
    NivelMaximo,
    /// La aldea ya tiene una subida de nivel marcada o en obra.
    MejoraEnCurso,
    NoEstaActivo,
}

/// Un minuto de cría (§14.3): el rebaño crece `crecimientoPorHora / 60` de sus cabezas (por la mano de obra y el factor de tecnología del ganado)
/// hasta el tope de sus `corrales`, y las cabezas enteras que pasan del pie de cría se sacrifican y entran en `almacen` mientras quepan en `hueco`.
/// Con el almacén lleno no se sacrifica nada y el rebaño sigue creciendo hasta el tope.
fn criar(reb: &mut Rebano, esp: &Especie, corrales: i64, mano: i64, factor: i64, almacen: &mut Almacen, hueco: Mil) {
    let (tope, pie) = (esp.tope_por_corral * corrales, esp.pie_por_corral * corrales);
    let crece = por(por(por(reb.cabezas, esp.crecimiento_por_hora), mano), factor) / 60;
    reb.cabezas = (reb.cabezas + crece).min(tope);
    let por_cabeza: Mil = esp.productos.iter().map(|&(_, n)| n).sum();
    let sobran = (reb.cabezas - pie).max(0) / 1000;
    let caben = if por_cabeza > 0 { hueco / por_cabeza } else { sobran };
    let k = sobran.min(caben);
    if k > 0 {
        reb.cabezas -= k * 1000;
        for &(x, n) in &esp.productos {
            almacen.producir(x, n * k);
        }
    }
}

/// Suma de costes por recurso, ordenada.
fn sumar(partes: &[&Costo]) -> Costo {
    let mut total: BTreeMap<Recurso, Mil> = BTreeMap::new();
    for &(rec, n) in partes.iter().copied().flatten() {
        *total.entry(rec).or_insert(0) += n;
    }
    total.into_iter().collect()
}

/// Una obra pendiente de materiales por `requerido`, sin nada entregado.
fn pendiente(requerido: Costo) -> EstadoPueblo {
    let entregado = requerido.iter().map(|&(rec, _)| (rec, 0)).collect();
    EstadoPueblo::Pendiente { requerido, entregado }
}

impl Asentamiento {
    pub fn pueblo(&self, id: u64) -> Option<&Pueblo> {
        self.pueblos.iter().find(|p| p.id == id)
    }

    /// Cuántos pueblos admite ahora la plaza (pendientes, en obra y activos, de cualquier tipo): el cupo de su nivel operativo.
    pub fn cupo_de_pueblos(&self, r: &Reglas) -> u32 {
        r.pueblos.cupo_por_nivel.range(..=self.nivel_actual()).next_back().map_or(0, |(_, &c)| c)
    }

    /// Aldeas agrícolas activas de la plaza (el gate del nivel 2 pide una; la misión A1 las cuenta).
    pub fn aldeas_activas(&self) -> u32 {
        self.pueblos.iter().filter(|p| p.es_aldea() && p.activo()).count() as u32
    }

    /// Lo que cuesta levantar un pueblo de ese tipo: su extractor, el coste adicional y el carro con su buey (`carro`, de la flota).
    pub fn costo_de_pueblo(r: &Reglas, tipo: Tipo, carro: &Costo) -> Costo {
        sumar(&[&r.catalogo.tipo(tipo).costo, &r.pueblos.coste_adicional, carro])
    }

    /// Lo que cuesta levantar una aldea agrícola: su nivel 1, el coste adicional y el carro con su buey.
    pub fn costo_de_aldea(r: &Reglas, carro: &Costo) -> Costo {
        sumar(&[&r.pueblos.aldea.nivel(1).coste, &r.pueblos.coste_adicional, carro])
    }

    /// Pueblos activos: cuentan como un edificio activo más al pagar el mantenimiento de la madre.
    pub(crate) fn pueblos_activos(&self) -> usize {
        self.pueblos.iter().filter(|p| p.activo()).count()
    }

    /// Trabajadores (pesants, milésimas) que piden los pueblos activos: se suman a los de los extractores de la ciudad. Una aldea pide los de
    /// sus granjas y sus corrales.
    pub(crate) fn trabajadores_de_pueblos(&self, r: &Reglas) -> Mil {
        let de = |t: Tipo| r.catalogo.tipo(t).trabajadores;
        self.pueblos
            .iter()
            .filter(|p| p.activo())
            .map(|p| match p.es_aldea() {
                true => {
                    let n = r.pueblos.aldea.nivel(p.nivel);
                    i64::from(n.granjas) * de(Tipo::Granja) + i64::from(n.corrales) * de(Tipo::Corral)
                }
                false => de(p.tipo),
            })
            .sum()
    }

    fn admite_otro_pueblo(&self, r: &Reglas, nivel_minimo: u32) -> Result<(), ErrorPueblo> {
        if self.nivel_actual() < nivel_minimo {
            return Err(ErrorPueblo::NivelInsuficiente);
        }
        if self.pueblos.len() as u32 >= self.cupo_de_pueblos(r) {
            return Err(ErrorPueblo::SinCupo);
        }
        Ok(())
    }

    /// Marca la obra de un pueblo sobre un nodo (que el núcleo ya comprobó libre, de ese tipo y dentro de la zona): queda pendiente de
    /// materiales y ocupa cupo. `carro` es el coste del carro con su buey.
    #[allow(clippy::too_many_arguments)]
    pub fn iniciar_pueblo(&mut self, r: &Reglas, tiene_tecnologia: &dyn Fn(&str) -> bool, id: u64, tipo: Tipo, nodo: u32, carro: &Costo, ev: &mut Vec<Evento>) -> Result<(), ErrorPueblo> {
        if !tipo.es_de_pueblo() {
            return Err(ErrorPueblo::TipoNoPueblo);
        }
        if r.catalogo.tipo(tipo).tecnologia.as_deref().is_some_and(|t| !tiene_tecnologia(t)) {
            return Err(ErrorPueblo::TecnologiaFaltante);
        }
        self.admite_otro_pueblo(r, r.pueblos.nivel_madre_minimo)?;
        let estado = pendiente(Self::costo_de_pueblo(r, tipo, carro));
        self.pueblos.push(Pueblo { id, tipo, sitio: Sitio::Nodo(nodo), nivel: 1, estado, mejora: None, rebano: None, almacen: Almacen::nuevo(r.pueblos.capacidad_almacen), carro: None, lleno: false });
        ev.push(Evento::PuebloMarcado { id, tipo, nodo });
        Ok(())
    }

    /// Marca la obra de una aldea agrícola en `pos` (que el núcleo ya comprobó dentro de la zona, con ruta y separada de los demás pueblos), con
    /// la `fertilidad` del terreno en ese punto: queda pendiente de materiales y ocupa cupo.
    pub fn iniciar_aldea(&mut self, r: &Reglas, id: u64, pos: Punto, fertilidad: i64, carro: &Costo, ev: &mut Vec<Evento>) -> Result<(), ErrorPueblo> {
        self.admite_otro_pueblo(r, r.pueblos.aldea.nivel_madre_minimo)?;
        if fertilidad < r.pueblos.aldea.fertilidad_minima {
            return Err(ErrorPueblo::FertilidadInsuficiente);
        }
        let estado = pendiente(Self::costo_de_aldea(r, carro));
        let sitio = Sitio::Punto { pos, fertilidad };
        self.pueblos.push(Pueblo { id, tipo: Tipo::Granja, sitio, nivel: 1, estado, mejora: None, rebano: None, almacen: Almacen::nuevo(r.pueblos.capacidad_almacen), carro: None, lleno: false });
        ev.push(Evento::AldeaMarcada { id, pos });
        Ok(())
    }

    /// Marca la subida de nivel de una aldea activa: otra obra de mapa pendiente de materiales (los del nivel siguiente). Mientras, la aldea
    /// sigue produciendo con su nivel de ahora.
    pub fn mejorar_aldea(&mut self, r: &Reglas, id: u64, ev: &mut Vec<Evento>) -> Result<(), ErrorPueblo> {
        let p = self.pueblos.iter_mut().find(|p| p.id == id).ok_or(ErrorPueblo::NoExiste)?;
        if !p.es_aldea() {
            return Err(ErrorPueblo::NoEsAldea);
        }
        if !p.activo() {
            return Err(ErrorPueblo::NoEstaActivo);
        }
        if p.mejora.is_some() {
            return Err(ErrorPueblo::MejoraEnCurso);
        }
        if p.nivel >= r.pueblos.aldea.nivel_maximo() {
            return Err(ErrorPueblo::NivelMaximo);
        }
        let nivel = p.nivel + 1;
        p.mejora = Some(pendiente(r.pueblos.aldea.nivel(nivel).coste.clone()));
        ev.push(Evento::AldeaMejoraMarcada { id, nivel });
        Ok(())
    }

    /// Entrega materiales a una obra pendiente (la del pueblo o la de la subida de nivel de la aldea): toma de `disponible` lo que falta, recurso
    /// a recurso, y lo anota. Con todo entregado arranca la obra (la del extractor del catálogo, o la del nivel de la aldea). Devuelve lo tomado,
    /// para que quien entrega lo quite de donde lo llevaba.
    pub fn entregar_a_pueblo(&mut self, t: Instante, r: &Reglas, id: u64, disponible: &[(Recurso, Mil)], ev: &mut Vec<Evento>) -> Result<Costo, ErrorPueblo> {
        let p = self.pueblos.iter_mut().find(|p| p.id == id).ok_or(ErrorPueblo::NoExiste)?;
        let minutos = match (p.es_aldea(), p.activo()) {
            (false, _) => r.catalogo.tipo(p.tipo).tiempo_minutos,
            (true, false) => r.pueblos.aldea.nivel(1).obra_minutos,
            (true, true) => r.pueblos.aldea.niveles.get(&(p.nivel + 1)).map_or(0, |n| n.obra_minutos),
        };
        let obra = match (&mut p.estado, &mut p.mejora) {
            (EstadoPueblo::Pendiente { .. }, _) => &mut p.estado,
            (EstadoPueblo::Activo, Some(m @ EstadoPueblo::Pendiente { .. })) => m,
            _ => return Err(ErrorPueblo::NoEstaPendiente),
        };
        let EstadoPueblo::Pendiente { requerido, entregado } = obra else { unreachable!("pendiente") };
        let tomado = tomar_materiales(requerido, entregado, disponible);
        if tomado.is_empty() {
            return Err(ErrorPueblo::NadaQueEntregar);
        }
        ev.push(Evento::PuebloEntrega { id, cantidad: tomado.iter().map(|&(_, n)| n).sum() });
        if requerido.iter().zip(entregado.iter()).all(|(a, b)| a.1 == b.1) {
            let completa_en = t + minutos * MS_POR_MINUTO;
            *obra = EstadoPueblo::EnObra { completa_en };
            ev.push(Evento::PuebloObraIniciada { id, completa_en });
        }
        Ok(tomado)
    }

    /// Cancela la obra o destruye el pueblo a mano: libera el cupo y no recupera nada. Devuelve el pueblo para que el núcleo libere su nodo
    /// y su carro.
    pub fn quitar_pueblo(&mut self, id: u64, ev: &mut Vec<Evento>) -> Option<Pueblo> {
        let i = self.pueblos.iter().position(|p| p.id == id)?;
        ev.push(Evento::PuebloDestruido { id });
        Some(self.pueblos.remove(i))
    }

    /// Las obras de mapa cuyo instante llegó pasan a activas, y las subidas de nivel de las aldeas suben (paso de construcción del minuto).
    pub(crate) fn completar_pueblos(&mut self, t: Instante, r: &Reglas, ev: &mut Vec<Evento>) {
        for p in &mut self.pueblos {
            if matches!(p.estado, EstadoPueblo::EnObra { completa_en } if t >= completa_en) {
                p.estado = EstadoPueblo::Activo;
                // El primer rebaño llega con la obra: el pie de cría de sus corrales.
                if p.es_aldea() {
                    let a = &r.pueblos.aldea;
                    let cabezas = a.especie(&a.especie_inicial).pie_por_corral * i64::from(a.nivel(p.nivel).corrales);
                    p.rebano = Some(Rebano { especie: a.especie_inicial.clone(), cabezas });
                }
                ev.push(Evento::PuebloCompletado { id: p.id });
            }
            if matches!(p.mejora, Some(EstadoPueblo::EnObra { completa_en }) if t >= completa_en) {
                p.mejora = None;
                p.nivel += 1;
                ev.push(Evento::AldeaSubio { id: p.id, nivel: p.nivel });
            }
        }
    }

    /// Un minuto de extracción de los pueblos activos contra su almacén local. Avisa (`PuebloLleno`) en el minuto en que el almacén se llena.
    pub(crate) fn extraer_pueblos(&mut self, r: &Reglas, mano: &Mano, ctx: &Contexto, fuentes: &mut dyn Fuentes, ev: &mut Vec<Evento>) {
        for p in self.pueblos.iter_mut().filter(|p| p.activo()) {
            let lleno = match p.sitio {
                Sitio::Nodo(nodo) => {
                    let Some((recurso, base)) = r.catalogo.tipo(p.tipo).produccion else { continue };
                    let deseado = por(por(base, mano.pesants), (ctx.factor_tecnologia)(recurso));
                    let pedido = deseado.min(p.almacen.hueco(recurso));
                    let salido = if pedido > 0 { fuentes.extraer(nodo, pedido) } else { 0 };
                    p.almacen.producir(recurso, salido);
                    p.almacen.hueco(recurso) == 0
                }
                Sitio::Punto { fertilidad, .. } => {
                    let n = r.pueblos.aldea.nivel(p.nivel);
                    let granja = r.catalogo.tipo(Tipo::Granja).nivel(1).map_or(0, |x| x.produccion_base_trigo);
                    let trigo = por(por(por(granja * i64::from(n.granjas), fertilidad), mano.pesants), ctx.factor_trigo);
                    // El almacén local de la aldea es uno para los dos: lo que ya hay de uno quita sitio al otro.
                    let hueco = |a: &Almacen| (r.pueblos.capacidad_almacen - a.disponible(Recurso::Trigo) - a.disponible(Recurso::Livestock)).max(0);
                    let entra = trigo.min(hueco(&p.almacen));
                    if entra > 0 {
                        p.almacen.producir(Recurso::Trigo, entra);
                    }
                    let libre = hueco(&p.almacen);
                    if let Some(reb) = &mut p.rebano {
                        criar(reb, r.pueblos.aldea.especie(&reb.especie), i64::from(n.corrales), mano.pesants, (ctx.factor_tecnologia)(Recurso::Livestock), &mut p.almacen, libre);
                    }
                    hueco(&p.almacen) == 0
                }
            };
            if lleno && !p.lleno {
                ev.push(Evento::PuebloLleno { id: p.id });
            }
            p.lleno = lleno;
        }
    }

    /// El saqueo de una aldea se lleva la parte `saqueoRebano` de sus cabezas enteras, hasta lo que quepa en `cabe` (cabezas); devuelve las cabezas que se lleva y
    /// lo que valen sacrificadas (lo que entra en los carros del atacante). Nada en un extractor o sin rebaño.
    pub fn saquear_rebano(&mut self, r: &Reglas, id: u64, cabe: impl Fn(&Costo) -> i64) -> (i64, Costo) {
        let Some(reb) = self.pueblos.iter_mut().find(|p| p.id == id).and_then(|p| p.rebano.as_mut()) else { return (0, vec![]) };
        let esp = r.pueblos.aldea.especie(&reb.especie);
        let quiere = por(reb.cabezas, r.pueblos.aldea.saqueo_rebano) / 1000;
        let cabezas = quiere.min(cabe(&esp.productos)).max(0);
        reb.cabezas -= cabezas * 1000;
        (cabezas, esp.productos.iter().map(|&(x, n)| (x, n * cabezas)).collect())
    }

    /// Anota lo que un carro de acarreo acaba de ingresar de verdad en el almacén de la plaza (lo que cupo, no lo que traía). Suma en la hora
    /// de `t` y descarta lo que ya salió de la ventana, así el vector no pasa de 24 horas por recurso.
    pub fn registrar_acarreo(&mut self, t: Instante, recurso: Recurso, ingresado: Mil) {
        if ingresado <= 0 {
            return;
        }
        let hora = hora_de(t);
        self.ingresos_acarreo.retain(|i| i.hora > hora - HORAS_VENTANA);
        match self.ingresos_acarreo.iter_mut().find(|i| i.recurso == recurso && i.hora == hora) {
            Some(i) => i.cantidad += ingresado,
            None => self.ingresos_acarreo.push(IngresoAcarreo { recurso, hora, cantidad: ingresado }),
        }
    }

    /// Lo que el acarreo ingresa por minuto en el almacén de la plaza, de media en la ventana de las últimas 24 h (`t` es la hora de la
    /// pregunta): es lo que cuenta como acarreo en la solvencia del ascenso (Doc 4.5). Con el almacén lleno no ingresa nada y no cuenta nada.
    pub(crate) fn acarreo_por_minuto(&self, t: Instante, recurso: Recurso) -> Mil {
        let hora = hora_de(t);
        let total: Mil = self.ingresos_acarreo.iter().filter(|i| i.recurso == recurso && i.hora > hora - HORAS_VENTANA && i.hora <= hora).map(|i| i.cantidad).sum();
        total / (HORAS_VENTANA * 60)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::UNO;
    use crate::produccion::mano_de_obra;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Mundo(HashMap<u32, Mil>);

    impl Fuentes for Mundo {
        fn densidad_parcela(&self, _: u32) -> i64 {
            0
        }
        fn extraer(&mut self, n: u32, pedido: Mil) -> Mil {
            let s = self.0.entry(n).or_insert(1_000_000);
            let sale = pedido.min(*s);
            *s -= sale;
            sale
        }
    }

    fn ctx<'a>() -> Contexto<'a> {
        Contexto { fertilidad: UNO, factor_trigo: UNO, factor_talleres: UNO, factor_tecnologia: &|_| UNO, tiene_tecnologia: &|_| true, recetas_pausadas: &[] }
    }

    fn carro() -> Costo {
        vec![(Recurso::Madera, 20_000), (Recurso::Oro, 12_000)]
    }

    /// Una plaza de nivel 2 con pesants de sobra.
    fn madre(r: &Reglas) -> Asentamiento {
        let mut a = Asentamiento::fundar(r, 0, &[], 3);
        a.nivel = 2;
        a.medidor.nivel_actual = 2;
        a
    }

    fn marcar(a: &mut Asentamiento, r: &Reglas, id: u64, tipo: Tipo) -> Result<(), ErrorPueblo> {
        a.iniciar_pueblo(r, &|_| true, id, tipo, id as u32, &carro(), &mut vec![])
    }

    #[test]
    fn las_cifras_salen_del_documento() {
        let r = Reglas::cargar().pueblos;
        assert_eq!((r.nivel_madre_minimo, r.capacidad_almacen, r.mineros, r.minutos_por_minero, r.casas_de_la_planta), (2, 500_000, 30, 12, 6));
        assert_eq!(r.cupo_por_nivel.values().copied().collect::<Vec<_>>(), vec![1, 3, 4, 5, 6]);
        assert_eq!(r.coste_adicional, vec![(Recurso::Madera, 50_000)]);
        let a = &r.aldea;
        assert_eq!((a.nivel_madre_minimo, a.fertilidad_minima, a.separacion_minima, a.granjeros, a.nivel_maximo()), (1, 300_000, 3_000, 30, 3));
        assert_eq!(a.niveles.values().map(|n| (n.granjas, n.corrales)).collect::<Vec<_>>(), vec![(2, 1), (4, 2), (6, 3)]);
    }

    #[test]
    fn un_pueblo_de_cobre_cuesta_100_de_madera_y_12_de_oro() {
        let r = Reglas::cargar();
        // 30 de la mina de cobre + 50 + 20 del carro; 12 de oro del buey (el ejemplo del documento).
        assert_eq!(Asentamiento::costo_de_pueblo(&r, Tipo::MinaCobre, &carro()), vec![(Recurso::Madera, 100_000), (Recurso::Oro, 12_000)]);
        assert_eq!(Asentamiento::costo_de_pueblo(&r, Tipo::Cantera, &carro()), vec![(Recurso::Madera, 90_000), (Recurso::Oro, 12_000)]);
    }

    #[test]
    fn exige_madre_de_nivel_2_cupo_y_un_extractor_de_pueblo() {
        let r = Reglas::cargar();
        let mut a = Asentamiento::fundar(&r, 0, &[], 3);
        assert_eq!(marcar(&mut a, &r, 1, Tipo::MinaCobre), Err(ErrorPueblo::NivelInsuficiente));
        assert_eq!(a.cupo_de_pueblos(&r), 1, "en el nivel 1 cabe un pueblo: la aldea agrícola");
        let mut a = madre(&r);
        assert_eq!(marcar(&mut a, &r, 1, Tipo::Corral), Err(ErrorPueblo::TipoNoPueblo));
        assert_eq!(marcar(&mut a, &r, 1, Tipo::Granja), Err(ErrorPueblo::TipoNoPueblo));
        assert_eq!(a.iniciar_pueblo(&r, &|_| false, 1, Tipo::MinaHierro, 1, &carro(), &mut vec![]), Err(ErrorPueblo::TecnologiaFaltante), "el hierro pide tecnología");
        assert_eq!((marcar(&mut a, &r, 1, Tipo::MinaCobre), marcar(&mut a, &r, 2, Tipo::Cantera), marcar(&mut a, &r, 3, Tipo::Mina)), (Ok(()), Ok(()), Ok(())));
        assert_eq!(marcar(&mut a, &r, 4, Tipo::Mina), Err(ErrorPueblo::SinCupo), "3 pueblos en el nivel 2, contando los pendientes");
        for (nivel, cupo) in [(3, 4), (4, 5), (5, 6)] {
            (a.nivel, a.medidor.nivel_actual) = (nivel, nivel);
            assert_eq!(a.cupo_de_pueblos(&r), cupo);
        }
    }

    #[test]
    fn la_obra_arranca_con_todo_entregado_y_acaba_en_el_paso_de_minuto_siguiente() {
        let r = Reglas::cargar();
        let mut a = madre(&r);
        marcar(&mut a, &r, 1, Tipo::MinaCobre).unwrap();
        assert_eq!(a.pueblo(1).unwrap().falta(), vec![(Recurso::Madera, 100_000), (Recurso::Oro, 12_000)]);
        let mut ev = vec![];
        // Una entrega parcial no arranca nada; sobra lo que no hace falta.
        let t = a.entregar_a_pueblo(1_000, &r, 1, &[(Recurso::Madera, 60_000), (Recurso::Piedra, 9_000)], &mut ev).unwrap();
        assert_eq!(t, vec![(Recurso::Madera, 60_000)]);
        assert_eq!(a.pueblo(1).unwrap().falta(), vec![(Recurso::Madera, 40_000), (Recurso::Oro, 12_000)]);
        assert_eq!(a.entregar_a_pueblo(1_000, &r, 1, &[(Recurso::Piedra, 5)], &mut ev), Err(ErrorPueblo::NadaQueEntregar));
        let t = a.entregar_a_pueblo(2_000, &r, 1, &[(Recurso::Madera, 500_000), (Recurso::Oro, 12_000)], &mut ev).unwrap();
        assert_eq!(t, vec![(Recurso::Madera, 40_000), (Recurso::Oro, 12_000)]);
        let fin = 2_000 + 360 * MS_POR_MINUTO;
        assert_eq!(a.pueblo(1).unwrap().estado, EstadoPueblo::EnObra { completa_en: fin }, "6 h de obra de mina de cobre");
        assert!(ev.contains(&Evento::PuebloObraIniciada { id: 1, completa_en: fin }));
        assert_eq!(a.entregar_a_pueblo(3_000, &r, 1, &[(Recurso::Madera, 1)], &mut ev), Err(ErrorPueblo::NoEstaPendiente));
        // No acaba antes de su hora; en el paso que la alcanza, queda activo.
        a.completar_pueblos(fin - 1, &r, &mut ev);
        assert!(!a.pueblo(1).unwrap().activo());
        a.completar_pueblos(fin, &r, &mut ev);
        assert!(a.pueblo(1).unwrap().activo() && ev.contains(&Evento::PuebloCompletado { id: 1 }));
    }

    #[test]
    fn destruirlo_libera_el_cupo_sin_recuperar_nada() {
        let r = Reglas::cargar();
        let mut a = madre(&r);
        marcar(&mut a, &r, 1, Tipo::MinaCobre).unwrap();
        marcar(&mut a, &r, 2, Tipo::Cantera).unwrap();
        marcar(&mut a, &r, 5, Tipo::Cantera).unwrap();
        a.entregar_a_pueblo(0, &r, 1, &[(Recurso::Madera, 50_000)], &mut vec![]).unwrap();
        let almacen = a.almacen.clone();
        assert!(a.quitar_pueblo(1, &mut vec![]).is_some() && a.quitar_pueblo(1, &mut vec![]).is_none());
        assert_eq!(a.almacen, almacen, "lo entregado no vuelve");
        assert_eq!(marcar(&mut a, &r, 3, Tipo::Mina), Ok(()), "el cupo quedó libre");
    }

    /// Un pueblo activo de cobre en la plaza, con pesants de sobra.
    fn con_pueblo_activo(r: &Reglas) -> Asentamiento {
        let mut a = madre(r);
        marcar(&mut a, r, 1, Tipo::MinaCobre).unwrap();
        a.pueblos[0].estado = EstadoPueblo::Activo;
        a
    }

    #[test]
    fn extrae_contra_su_almacen_se_llena_una_vez_y_para() {
        let r = Reglas::cargar();
        let mut a = con_pueblo_activo(&r);
        let (mut w, mut ev) = (Mundo::default(), vec![]);
        let mano = mano_de_obra(&r.catalogo, &a.edificios, 20, 0, a.trabajadores_de_pueblos(&r));
        assert_eq!(mano.requeridos_pesants, 12_000, "la Granja de la fundación (4) y los 8 del pueblo");
        a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut ev);
        assert_eq!(a.pueblos[0].almacen.cantidad(Recurso::Cobre), 5_000, "5 por minuto con la mano de obra completa");
        assert_eq!(a.almacen.cantidad(Recurso::Cobre), 0, "lo extraído se queda en el pueblo, no en la plaza");
        for _ in 0..200 {
            a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut ev);
        }
        assert_eq!((a.pueblos[0].almacen.cantidad(Recurso::Cobre), w.0[&1]), (500_000, 500_000), "para al llenarse y no gasta el yacimiento");
        assert_eq!(ev, vec![Evento::PuebloLleno { id: 1 }], "avisa una sola vez");
        // Al vaciarse el almacén vuelve a extraer y a poder avisar.
        a.pueblos[0].almacen.quitar(Recurso::Cobre, 500_000);
        a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut ev);
        assert!(!a.pueblos[0].lleno && a.pueblos[0].almacen.cantidad(Recurso::Cobre) == 5_000);
    }

    #[test]
    fn el_nodo_agotado_espera_sin_mudarse_y_faltando_pesants_se_extrae_proporcionalmente_menos() {
        let r = Reglas::cargar();
        let mut a = con_pueblo_activo(&r);
        let mut w = Mundo::default();
        w.0.insert(1, 3_000);
        let mano = mano_de_obra(&r.catalogo, &a.edificios, 6, 0, a.trabajadores_de_pueblos(&r));
        assert_eq!(mano.pesants, 500_000, "6 pesants para 12 que piden la Granja y el pueblo");
        a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut vec![]);
        assert_eq!((a.pueblos[0].almacen.cantidad(Recurso::Cobre), a.pueblos[0].almacen.disponible(Recurso::Cobre)), (2_000, 2_500));
        a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut vec![]);
        a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut vec![]);
        assert_eq!((a.pueblos[0].almacen.cantidad(Recurso::Cobre), a.pueblos[0].nodo()), (3_000, Some(1)), "agotado: espera, no cambia de nodo");
    }

    #[test]
    fn el_acarreo_cuenta_en_la_solvencia_lo_que_ingresa_de_media_en_24_horas() {
        let r = Reglas::cargar();
        let mut a = con_pueblo_activo(&r);
        const H: Instante = 60 * MS_POR_MINUTO;
        assert_eq!(a.acarreo_por_minuto(0, Recurso::Cobre), 0, "sin llegadas no cuenta nada, tenga o no carro");
        a.pueblos[0].carro = Some(9);
        assert_eq!(a.acarreo_por_minuto(0, Recurso::Cobre), 0, "la producción nominal ya no cuenta");
        // 144 u en 3 llegadas (dos en la misma hora): 144 / 1440 min = 0,1 por minuto; otro recurso, aparte; una llegada sin sitio no suma.
        a.registrar_acarreo(2 * H, Recurso::Cobre, 48_000);
        a.registrar_acarreo(2 * H + 30 * MS_POR_MINUTO, Recurso::Cobre, 48_000);
        a.registrar_acarreo(10 * H, Recurso::Cobre, 48_000);
        a.registrar_acarreo(10 * H, Recurso::Piedra, 1_440_000);
        a.registrar_acarreo(10 * H, Recurso::Cobre, 0);
        assert_eq!(a.ingresos_acarreo.len(), 3, "dos cubos de cobre (horas 2 y 10) y uno de piedra");
        assert_eq!((a.acarreo_por_minuto(10 * H, Recurso::Cobre), a.acarreo_por_minuto(10 * H, Recurso::Piedra)), (100, 1_000));
        // La ventana: en la hora 25 sigue el cubo de la hora 2 (23 horas atrás); en la 26 ya salió.
        assert_eq!(a.acarreo_por_minuto(25 * H, Recurso::Cobre), 100);
        assert_eq!(a.acarreo_por_minuto(26 * H, Recurso::Cobre), 33, "queda el cubo de la hora 10: 48 u / 1440 min");
        assert_eq!(a.acarreo_por_minuto(34 * H, Recurso::Cobre), 0, "y a las 24 horas de la última llegada, nada");
    }

    #[test]
    fn registrar_poda_lo_que_salio_de_la_ventana_y_el_vector_no_crece() {
        let r = Reglas::cargar();
        let mut a = con_pueblo_activo(&r);
        for h in 0..100 {
            a.registrar_acarreo(h * 60 * MS_POR_MINUTO, Recurso::Cobre, 1_000);
        }
        assert_eq!(a.ingresos_acarreo.len(), HORAS_VENTANA as usize, "24 cubos horarios como mucho");
        assert_eq!(a.ingresos_acarreo.first().map(|i| i.hora), Some(76));
    }

    /// Una plaza de nivel 1 con una aldea marcada en un punto de fertilidad 0,5.
    fn con_aldea(r: &Reglas) -> Asentamiento {
        let mut a = Asentamiento::fundar(r, 0, &[], 3);
        a.iniciar_aldea(r, 1, Punto::new(5_000, 0), 500_000, &carro(), &mut vec![]).unwrap();
        a
    }

    #[test]
    fn la_aldea_se_marca_desde_el_nivel_1_en_tierra_fertil_y_ocupa_el_cupo() {
        let r = Reglas::cargar();
        let mut a = Asentamiento::fundar(&r, 0, &[], 3);
        let mut ev = vec![];
        assert_eq!(a.iniciar_aldea(&r, 1, Punto::new(5_000, 0), 299_999, &carro(), &mut ev), Err(ErrorPueblo::FertilidadInsuficiente));
        a.iniciar_aldea(&r, 1, Punto::new(5_000, 0), 300_000, &carro(), &mut ev).unwrap();
        assert_eq!(ev, vec![Evento::AldeaMarcada { id: 1, pos: Punto::new(5_000, 0) }]);
        let p = a.pueblo(1).unwrap();
        assert!(p.es_aldea() && p.nodo().is_none() && p.nivel == 1 && p.mejora.is_none());
        // Nivel 1 (30) + 50 + el carro (20 de madera y 12 de oro): lo mismo que una mina de cobre.
        assert_eq!(p.falta(), vec![(Recurso::Madera, 100_000), (Recurso::Oro, 12_000)]);
        assert_eq!(a.iniciar_aldea(&r, 2, Punto::new(9_000, 0), UNO, &carro(), &mut ev), Err(ErrorPueblo::SinCupo), "en el nivel 1, un solo pueblo");
        assert_eq!(a.aldeas_activas(), 0, "pendiente no cuenta");
    }

    #[test]
    fn la_aldea_cultiva_trigo_con_la_fertilidad_de_su_punto_y_saca_ganado_contra_un_almacen_compartido() {
        let r = Reglas::cargar();
        let mut a = con_aldea(&r);
        a.pueblos[0].estado = EstadoPueblo::Activo;
        a.pueblos[0].rebano = Some(Rebano { especie: "vacuno".into(), cabezas: 20_000 });
        assert_eq!(a.aldeas_activas(), 1);
        let (mut w, mut ev) = (Mundo::default(), vec![]);
        assert_eq!(a.trabajadores_de_pueblos(&r), 12_000, "2 granjas y 1 corral, 4 cada uno");
        let mano = mano_de_obra(&r.catalogo, &a.edificios, 40, 0, a.trabajadores_de_pueblos(&r));
        a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut ev);
        let p = &a.pueblos[0];
        assert_eq!((p.almacen.cantidad(Recurso::Trigo), p.almacen.cantidad(Recurso::Livestock)), (120_000, 0), "2 × 120 × 0,5 de trigo; el pie de cría no se sacrifica");
        assert_eq!(p.rebano.as_ref().unwrap().cabezas, 20_050, "20 cabezas × 15 % por hora = 0,05 por minuto");
        assert!(w.0.is_empty(), "no toca ningún nodo");
        // A los 20 minutos hay una cabeza entera por encima del pie: se sacrifica y da 60 de ganado.
        let mut minutos = 1;
        while a.pueblos[0].almacen.cantidad(Recurso::Livestock) == 0 {
            a.pueblos[0].almacen = Almacen::nuevo(r.pueblos.capacidad_almacen);
            a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut ev);
            minutos += 1;
        }
        assert_eq!((minutos, a.pueblos[0].almacen.cantidad(Recurso::Livestock), a.pueblos[0].rebano.as_ref().unwrap().cabezas), (20, 60_000, 20_016), "el rebaño crece sobre lo que ya creció");
        for _ in 0..10 {
            a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut ev);
        }
        let p = &a.pueblos[0];
        assert_eq!(p.existencias(&r), 500_000, "el almacén local es uno para los dos");
        assert_eq!(ev, vec![Evento::PuebloLleno { id: 1 }]);
        assert_eq!(p.carga_del_carro(&r, 500_000).iter().map(|x| x.1).sum::<Mil>(), 500_000, "el carro se lo lleva todo");
        assert_eq!(p.carga_del_carro(&r, 500_000)[0].0, Recurso::Trigo);
    }

    #[test]
    fn la_aldea_sube_de_nivel_con_otra_obra_de_mapa_sin_dejar_de_producir() {
        let r = Reglas::cargar();
        let mut a = con_aldea(&r);
        let mut ev = vec![];
        assert_eq!(a.mejorar_aldea(&r, 1, &mut ev), Err(ErrorPueblo::NoEstaActivo));
        a.entregar_a_pueblo(0, &r, 1, &[(Recurso::Madera, 100_000), (Recurso::Oro, 12_000)], &mut ev).unwrap();
        assert_eq!(a.pueblo(1).unwrap().estado, EstadoPueblo::EnObra { completa_en: 240 * MS_POR_MINUTO }, "4 h la del nivel 1");
        a.completar_pueblos(240 * MS_POR_MINUTO, &r, &mut ev);
        assert!(a.pueblo(1).unwrap().activo());
        ev.clear();
        a.mejorar_aldea(&r, 1, &mut ev).unwrap();
        assert_eq!(ev, vec![Evento::AldeaMejoraMarcada { id: 1, nivel: 2 }]);
        assert_eq!(a.mejorar_aldea(&r, 1, &mut ev), Err(ErrorPueblo::MejoraEnCurso));
        assert_eq!(a.pueblo(1).unwrap().falta(), vec![(Recurso::Madera, 60_000), (Recurso::Piedra, 20_000)]);
        assert!(a.pueblo(1).unwrap().activo(), "sigue activa mientras espera los materiales");
        let t = 1_000 * MS_POR_MINUTO;
        a.entregar_a_pueblo(t, &r, 1, &[(Recurso::Madera, 60_000), (Recurso::Piedra, 20_000)], &mut ev).unwrap();
        assert_eq!(a.pueblo(1).unwrap().mejora, Some(EstadoPueblo::EnObra { completa_en: t + 120 * MS_POR_MINUTO }));
        a.completar_pueblos(t + 119 * MS_POR_MINUTO, &r, &mut ev);
        assert_eq!(a.pueblo(1).unwrap().nivel, 1);
        a.completar_pueblos(t + 120 * MS_POR_MINUTO, &r, &mut ev);
        let p = a.pueblo(1).unwrap();
        assert_eq!((p.nivel, p.mejora.clone()), (2, None));
        assert!(ev.contains(&Evento::AldeaSubio { id: 1, nivel: 2 }));
        assert_eq!(a.trabajadores_de_pueblos(&r), 24_000, "4 granjas y 2 corrales");
        a.pueblos[0].nivel = 3;
        assert_eq!(a.mejorar_aldea(&r, 1, &mut ev), Err(ErrorPueblo::NivelMaximo));
        // Un extractor no sube de nivel.
        let mut m = madre(&r);
        marcar(&mut m, &r, 2, Tipo::MinaCobre).unwrap();
        m.pueblos[0].estado = EstadoPueblo::Activo;
        assert_eq!(m.mejorar_aldea(&r, 2, &mut ev), Err(ErrorPueblo::NoEsAldea));
    }
    #[test]
    fn la_aldea_nace_con_su_pie_de_cria_y_el_saqueo_se_lleva_la_mitad_de_las_cabezas() {
        let r = Reglas::cargar();
        let mut a = con_aldea(&r);
        let mut ev = vec![];
        a.entregar_a_pueblo(0, &r, 1, &[(Recurso::Madera, 100_000), (Recurso::Oro, 12_000)], &mut ev).unwrap();
        assert!(a.pueblo(1).unwrap().rebano.is_none(), "en obra aún no hay rebaño");
        a.completar_pueblos(240 * MS_POR_MINUTO, &r, &mut ev);
        assert_eq!(a.pueblo(1).unwrap().rebano, Some(Rebano { especie: "vacuno".into(), cabezas: 20_000 }), "20 por corral, 1 corral");
        // Caben 6 cabezas en los carros: quiere 10 (la mitad de 20) y se lleva 6, con su ganado.
        assert_eq!(a.saquear_rebano(&r, 1, |_| 6), (6, vec![(Recurso::Livestock, 360_000)]));
        assert_eq!(a.pueblo(1).unwrap().rebano.as_ref().unwrap().cabezas, 14_000);
        assert_eq!(a.saquear_rebano(&r, 1, |_| 100).0, 7, "la mitad de 14");
        assert_eq!(a.saquear_rebano(&r, 99, |_| 100), (0, vec![]), "sin rebaño no hay nada que llevarse");
    }
}
