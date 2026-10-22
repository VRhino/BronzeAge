//! Pueblos extractores y obras de mapa (Economia_Construccion §14, Doc 4.2.2). Un pueblo es el extractor de mapa de siempre (Cantera o mina)
//! plantado sobre su nodo, con un almacén local y un carro de acarreo (que vive en la flota, crate `comercio`). **No es un asentamiento**: sin
//! zona, nivel, cargos ni medidor. Es un elemento de su plaza madre, que paga su mantenimiento (cuenta como un edificio activo) y le pone los
//! trabajadores (sus pesants); por eso no está entre los `edificios` de la ciudad, que son del interior.
//!
//! # La obra de mapa
//! No pasa por la cola de construcción. Se marca sobre un nodo y queda **pendiente de materiales** (ocupa cupo desde ese momento); los
//! materiales se entregan en el sitio, y con todo entregado arranca una obra corta (la del extractor del catálogo). Al acabar, el pueblo queda
//! activo. El fin de obra se resuelve en el paso de minuto de la plaza (precisión de un minuto, sin evento propio).
//!
//! # La extracción
//! Un minuto de extracción es `produccionBase × ratioManoObra × factorTecnología` contra el almacén **local**; con el almacén lleno se para
//! (y avisa una sola vez, `PuebloLleno`). Un nodo agotado espera a regenerarse. Un pueblo no entra en el reemplazo automático ni en
//! `EXTRACCION_MAXIMOS`. El carro, el viaje y la llegada son del núcleo.

use crate::almacen::Almacen;
use crate::asentamiento::{Asentamiento, Evento};
use crate::catalogo::{costo, Tipo};
use crate::datos::{mil, por, Reglas};
use crate::produccion::{Contexto, Fuentes, Mano};
use crate::recurso::{Costo, Recurso};
use base::{Instante, Mil, MS_POR_MINUTO};
use serde_json::Value;
use std::collections::BTreeMap;

/// `PUEBLOS_EXTRACTORES`: las cifras de los pueblos y sus obras. El carro con su buey no está aquí: su coste es el de la flota
/// (`comercio::acarreo::coste_reposicion`) y lo pone quien llama.
#[derive(Clone, Debug)]
pub struct ReglasPueblos {
    /// Nivel mínimo de la plaza madre.
    pub nivel_madre_minimo: u32,
    /// Pueblos que admite la madre por nivel (2 / 3 / 4 / 5).
    pub cupo_por_nivel: BTreeMap<u32, u32>,
    /// Capacidad del almacén local, por recurso (el pueblo extrae uno solo).
    pub capacidad_almacen: Mil,
    /// Lo que la obra cuesta además del extractor y del carro con su buey.
    pub coste_adicional: Costo,
    /// Guarnición de serie: mineros sin dueño de la IA, fuera de cupo (el sistema militar los pondrá).
    pub mineros: u32,
    /// Minutos que tarda en reponerse gratis esa guarnición si cae.
    pub reposicion_mineros_min: i64,
}

impl ReglasPueblos {
    pub fn de(b: &Value) -> Self {
        let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).unwrap_or_else(|| panic!("falta «{k}» en PUEBLOS_EXTRACTORES"));
        Self {
            nivel_madre_minimo: num(b, "nivelMadreMinimo") as u32,
            cupo_por_nivel: b["cupoPorNivel"].as_object().expect("cupoPorNivel").iter().map(|(k, v)| (k.parse().expect("nivel"), v.as_u64().expect("cupo") as u32)).collect(),
            capacidad_almacen: mil(num(b, "capacidadAlmacenLocal")),
            coste_adicional: costo(b.get("costeAdicional")),
            mineros: num(&b["mineros"], "escuadra") as u32,
            reposicion_mineros_min: num(&b["mineros"], "reposicionMinutos") as i64,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pueblo {
    /// Id de mundo (el mismo contador que ejércitos y plazas).
    pub id: u64,
    pub tipo: Tipo,
    /// Índice del nodo del mapa sobre el que está plantado.
    pub nodo: u32,
    pub estado: EstadoPueblo,
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

    /// El recurso que extrae.
    pub fn recurso(&self, r: &Reglas) -> Recurso {
        r.catalogo.tipo(self.tipo).produccion.expect("un extractor produce un recurso").0
    }

    /// Lo que todavía falta entregar a una obra pendiente (vacío si no lo está o ya está todo).
    pub fn falta(&self) -> Costo {
        match &self.estado {
            EstadoPueblo::Pendiente { requerido, entregado } => falta_de(requerido, entregado),
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
}

/// Suma de costes por recurso, ordenada.
fn sumar(partes: &[&Costo]) -> Costo {
    let mut total: BTreeMap<Recurso, Mil> = BTreeMap::new();
    for &(rec, n) in partes.iter().copied().flatten() {
        *total.entry(rec).or_insert(0) += n;
    }
    total.into_iter().collect()
}

impl Asentamiento {
    pub fn pueblo(&self, id: u64) -> Option<&Pueblo> {
        self.pueblos.iter().find(|p| p.id == id)
    }

    /// Cuántos pueblos admite ahora la plaza (pendientes, en obra y activos): el cupo de su nivel operativo; 0 por debajo del nivel mínimo.
    pub fn cupo_de_pueblos(&self, r: &Reglas) -> u32 {
        let nivel = self.nivel_actual();
        if nivel < r.pueblos.nivel_madre_minimo {
            return 0;
        }
        r.pueblos.cupo_por_nivel.range(..=nivel).next_back().map_or(0, |(_, &c)| c)
    }

    /// Lo que cuesta levantar un pueblo de ese tipo: su extractor, el coste adicional y el carro con su buey (`carro`, de la flota).
    pub fn costo_de_pueblo(r: &Reglas, tipo: Tipo, carro: &Costo) -> Costo {
        sumar(&[&r.catalogo.tipo(tipo).costo, &r.pueblos.coste_adicional, carro])
    }

    /// Pueblos activos: cuentan como un edificio activo más al pagar el mantenimiento de la madre.
    pub(crate) fn pueblos_activos(&self) -> usize {
        self.pueblos.iter().filter(|p| p.activo()).count()
    }

    /// Trabajadores (pesants, milésimas) que piden los pueblos activos: se suman a los de los extractores de la ciudad.
    pub(crate) fn trabajadores_de_pueblos(&self, r: &Reglas) -> Mil {
        self.pueblos.iter().filter(|p| p.activo()).map(|p| r.catalogo.tipo(p.tipo).trabajadores).sum()
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
        if self.nivel_actual() < r.pueblos.nivel_madre_minimo {
            return Err(ErrorPueblo::NivelInsuficiente);
        }
        if self.pueblos.len() as u32 >= self.cupo_de_pueblos(r) {
            return Err(ErrorPueblo::SinCupo);
        }
        let requerido = Self::costo_de_pueblo(r, tipo, carro);
        let entregado = requerido.iter().map(|&(rec, _)| (rec, 0)).collect();
        self.pueblos.push(Pueblo { id, tipo, nodo, estado: EstadoPueblo::Pendiente { requerido, entregado }, almacen: Almacen::nuevo(r.pueblos.capacidad_almacen), carro: None, lleno: false });
        ev.push(Evento::PuebloMarcado { id, tipo, nodo });
        Ok(())
    }

    /// Entrega materiales a una obra pendiente: toma de `disponible` lo que falta, recurso a recurso, y lo anota. Con todo entregado arranca
    /// la obra (la del extractor del catálogo). Devuelve lo tomado, para que quien entrega lo quite de donde lo llevaba.
    pub fn entregar_a_pueblo(&mut self, t: Instante, r: &Reglas, id: u64, disponible: &[(Recurso, Mil)], ev: &mut Vec<Evento>) -> Result<Costo, ErrorPueblo> {
        let p = self.pueblos.iter_mut().find(|p| p.id == id).ok_or(ErrorPueblo::NoExiste)?;
        let EstadoPueblo::Pendiente { requerido, entregado } = &mut p.estado else { return Err(ErrorPueblo::NoEstaPendiente) };
        let tomado = tomar_materiales(requerido, entregado, disponible);
        if tomado.is_empty() {
            return Err(ErrorPueblo::NadaQueEntregar);
        }
        ev.push(Evento::PuebloEntrega { id, cantidad: tomado.iter().map(|&(_, n)| n).sum() });
        if requerido.iter().zip(entregado.iter()).all(|(a, b)| a.1 == b.1) {
            let completa_en = t + r.catalogo.tipo(p.tipo).tiempo_minutos * MS_POR_MINUTO;
            p.estado = EstadoPueblo::EnObra { completa_en };
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

    /// Las obras de mapa cuyo instante llegó pasan a activas (paso de construcción del minuto).
    pub(crate) fn completar_pueblos(&mut self, t: Instante, ev: &mut Vec<Evento>) {
        for p in &mut self.pueblos {
            if matches!(p.estado, EstadoPueblo::EnObra { completa_en } if t >= completa_en) {
                p.estado = EstadoPueblo::Activo;
                ev.push(Evento::PuebloCompletado { id: p.id });
            }
        }
    }

    /// Un minuto de extracción de los pueblos activos contra su almacén local. Avisa (`PuebloLleno`) en el minuto en que el almacén se llena.
    pub(crate) fn extraer_pueblos(&mut self, r: &Reglas, mano: &Mano, ctx: &Contexto, fuentes: &mut dyn Fuentes, ev: &mut Vec<Evento>) {
        for p in self.pueblos.iter_mut().filter(|p| p.activo()) {
            let Some((recurso, base)) = r.catalogo.tipo(p.tipo).produccion else { continue };
            let deseado = por(por(base, mano.pesants), (ctx.factor_tecnologia)(recurso));
            let pedido = deseado.min(p.almacen.espacio(recurso));
            let salido = if pedido > 0 { fuentes.extraer(p.nodo, pedido) } else { 0 };
            p.almacen.agregar(recurso, salido);
            let lleno = p.almacen.espacio(recurso) == 0;
            if lleno && !p.lleno {
                ev.push(Evento::PuebloLleno { id: p.id });
            }
            p.lleno = lleno;
        }
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
        assert_eq!((r.nivel_madre_minimo, r.capacidad_almacen, r.mineros, r.reposicion_mineros_min), (2, 500_000, 30, 360));
        assert_eq!(r.cupo_por_nivel.values().copied().collect::<Vec<_>>(), vec![2, 3, 4, 5]);
        assert_eq!(r.coste_adicional, vec![(Recurso::Madera, 50_000)]);
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
        assert_eq!(a.cupo_de_pueblos(&r), 0);
        let mut a = madre(&r);
        assert_eq!(marcar(&mut a, &r, 1, Tipo::Corral), Err(ErrorPueblo::TipoNoPueblo));
        assert_eq!(marcar(&mut a, &r, 1, Tipo::Granja), Err(ErrorPueblo::TipoNoPueblo));
        assert_eq!(a.iniciar_pueblo(&r, &|_| false, 1, Tipo::MinaHierro, 1, &carro(), &mut vec![]), Err(ErrorPueblo::TecnologiaFaltante), "el hierro pide tecnología");
        assert_eq!((marcar(&mut a, &r, 1, Tipo::MinaCobre), marcar(&mut a, &r, 2, Tipo::Cantera)), (Ok(()), Ok(())));
        assert_eq!(marcar(&mut a, &r, 3, Tipo::Mina), Err(ErrorPueblo::SinCupo), "2 pueblos en el nivel 2, contando los pendientes");
        for (nivel, cupo) in [(3, 3), (4, 4), (5, 5)] {
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
        a.completar_pueblos(fin - 1, &mut ev);
        assert!(!a.pueblo(1).unwrap().activo());
        a.completar_pueblos(fin, &mut ev);
        assert!(a.pueblo(1).unwrap().activo() && ev.contains(&Evento::PuebloCompletado { id: 1 }));
    }

    #[test]
    fn destruirlo_libera_el_cupo_sin_recuperar_nada() {
        let r = Reglas::cargar();
        let mut a = madre(&r);
        marcar(&mut a, &r, 1, Tipo::MinaCobre).unwrap();
        marcar(&mut a, &r, 2, Tipo::Cantera).unwrap();
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
        assert_eq!(a.pueblos[0].almacen.cantidad(Recurso::Cobre), 2_500);
        a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut vec![]);
        a.extraer_pueblos(&r, &mano, &ctx(), &mut w, &mut vec![]);
        assert_eq!((a.pueblos[0].almacen.cantidad(Recurso::Cobre), a.pueblos[0].nodo), (3_000, 1), "agotado: espera, no cambia de nodo");
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
}
