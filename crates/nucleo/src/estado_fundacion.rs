//! La Caravana de Fundación como entidad y el fondo de refundación de los campamentos de mercenarios (Fundacion_Y_Refundacion §2, §7 y §9; sistema 5, tanda 5G-b). Entra entero
//! en el hash y en las fotos. Las reglas están en `nucleo/fundacion.rs`.
//!
//! La caravana toma su id del contador de las caravanas comerciales (`EstadoComercio::siguiente_caravana`): en el contrato es una `Caravana` más (`car-N`, tipo `CARAVANA_DE_FUNDACION`).
//! Sus plazos (la caducidad de la suelta y la llegada de la que vuelve sola) son eventos de la cola con la clave [`entidad_fundacion`].

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_columna::{plan_escribir, plan_leer};
use crate::codec_comercio::{carga_escribir, carga_leer};
use crate::estado_comercio::ETIQUETA;
use base::{Instante, Plan, Punto};
use comercio::Carga as Mercancia;
use std::collections::BTreeMap;

const ETIQUETA_FUNDACION: u64 = 16 * ETIQUETA;

/// Clave de entidad (de generación, de cola y de la rejilla de encuentros) de una Caravana de Fundación.
pub fn entidad_fundacion(id: u64) -> u64 {
    ETIQUETA_FUNDACION | id
}

/// ¿Es esta clave de entidad la de una Caravana de Fundación? (la rejilla de encuentros la mezcla con columnas, caravanas y plazas).
pub fn es_entidad_fundacion(entidad: u64) -> bool {
    entidad & !(ETIQUETA - 1) == ETIQUETA_FUNDACION
}

/// De dónde salió: lo que costó vuelve allí.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origen {
    /// Lanzada desde una plaza (`lanzarCaravanaFundacion`): vuelve a su almacén.
    Plaza(u64),
    /// Comprada en un campamento de mercenarios (`comprarCaravanaDeRefundacion`): vuelve al fondo de ese campamento, a nombre de cada aportante.
    Campamento(u32),
    /// La dio la administración (`DarCaravanaFundacion`): no costó nada y, si vuelve sola, se pierde.
    Ninguno,
}

/// Dónde está.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Situacion {
    /// Suelta y parada en su origen (`disponible`): caduca en `caduca_en`.
    Disponible,
    /// Enganchada a la columna (`adjunta`): va donde va ella.
    Adjunta(u64),
    /// Vuelve sola a su origen (`retornando`) con este plan: se puede reclamar por el camino; al llegar se desarma.
    Retornando(Plan),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaravanaFundacion {
    pub id: u64,
    /// Su Facción (D48). `None` solo en la que da la administración a una columna sin bandera.
    pub faccion: Option<u32>,
    /// Quien la lleva y funda con ella.
    pub titular: u64,
    pub origen: Origen,
    /// Dónde está si no va en una columna ni vuelve con un plan (parada en su origen) y dónde la dejó la última columna que la llevó.
    pub posicion: Punto,
    /// Lo que costó: lo que se devuelve.
    pub contenido: Mercancia,
    /// Solo la de un campamento: lo que puso cada aportante, para devolvérselo a su nombre.
    pub aportes: BTreeMap<u64, Mercancia>,
    pub situacion: Situacion,
    /// Solo suelta: cuándo caduca.
    pub caduca_en: Option<Instante>,
}

impl CaravanaFundacion {
    /// Dónde está en `t` (la adjunta, donde la dejó su columna la última vez que se miró: quien la proyecta usa la columna).
    pub fn posicion_en(&self, t: Instante) -> Punto {
        match &self.situacion {
            Situacion::Retornando(plan) => plan.posicion_en(t).unwrap_or(self.posicion),
            _ => self.posicion,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EstadoFundacion {
    pub caravanas: BTreeMap<u64, CaravanaFundacion>,
    /// El fondo de refundación (`CampamentoMercenarios.fondos`): por campamento, lo que aportó cada héroe.
    pub fondos: BTreeMap<u32, BTreeMap<u64, Mercancia>>,
}

impl EstadoFundacion {
    /// La primera Caravana de Fundación enganchada a la columna, si lleva alguna.
    pub fn de_columna(&self, ejercito: u64) -> Option<u64> {
        self.caravanas.values().find(|c| c.situacion == Situacion::Adjunta(ejercito)).map(|c| c.id)
    }
}

fn mercancias_escribir(w: &mut Escritor, m: &BTreeMap<u64, Mercancia>) {
    w.u64(m.len() as u64);
    for (&h, c) in m {
        w.u64(h);
        carga_escribir(w, c);
    }
}

fn mercancias_leer(r: &mut Lector) -> Result<BTreeMap<u64, Mercancia>, ErrorFormato> {
    (0..r.cuenta()?).map(|_| Ok((r.u64()?, carga_leer(r)?))).collect()
}

pub(crate) fn escribir(w: &mut Escritor, f: &EstadoFundacion) {
    w.u64(f.caravanas.len() as u64);
    for c in f.caravanas.values() {
        w.u64(c.id);
        w.opt_u64(c.faccion.map(u64::from));
        w.u64(c.titular);
        match c.origen {
            Origen::Plaza(a) => {
                w.u8(0);
                w.u64(a);
            }
            Origen::Campamento(k) => {
                w.u8(1);
                w.u32(k);
            }
            Origen::Ninguno => w.u8(2),
        }
        w.punto(c.posicion);
        carga_escribir(w, &c.contenido);
        mercancias_escribir(w, &c.aportes);
        match &c.situacion {
            Situacion::Disponible => w.u8(0),
            Situacion::Adjunta(e) => {
                w.u8(1);
                w.u64(*e);
            }
            Situacion::Retornando(plan) => {
                w.u8(2);
                plan_escribir(w, plan);
            }
        }
        w.opt_i64(c.caduca_en);
    }
    w.u64(f.fondos.len() as u64);
    for (&k, m) in &f.fondos {
        w.u32(k);
        mercancias_escribir(w, m);
    }
}

pub(crate) fn leer(r: &mut Lector) -> Result<EstadoFundacion, ErrorFormato> {
    let mut f = EstadoFundacion::default();
    for _ in 0..r.cuenta()? {
        let id = r.u64()?;
        let faccion = r.opt_u64()?.map(u32::try_from).transpose().map_err(|_| ErrorFormato("Facción fuera de rango"))?;
        let titular = r.u64()?;
        let origen = match r.u8()? {
            0 => Origen::Plaza(r.u64()?),
            1 => Origen::Campamento(r.u32()?),
            2 => Origen::Ninguno,
            _ => return Err(ErrorFormato("origen de Caravana de Fundación desconocido")),
        };
        let (posicion, contenido, aportes) = (r.punto()?, carga_leer(r)?, mercancias_leer(r)?);
        let situacion = match r.u8()? {
            0 => Situacion::Disponible,
            1 => Situacion::Adjunta(r.u64()?),
            2 => Situacion::Retornando(plan_leer(r)?),
            _ => return Err(ErrorFormato("situación de Caravana de Fundación desconocida")),
        };
        let caduca_en = r.opt_i64()?;
        f.caravanas.insert(id, CaravanaFundacion { id, faccion, titular, origen, posicion, contenido, aportes, situacion, caduca_en });
    }
    for _ in 0..r.cuenta()? {
        let k = r.u32()?;
        f.fondos.insert(k, mercancias_leer(r)?);
    }
    Ok(f)
}
