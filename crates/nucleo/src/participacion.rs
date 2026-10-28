//! La participación (mecánica 54, `Docs/Mechanics/Participacion.md`): los puntos de cada héroe en cada plaza. Aquí está lo puro: las cifras del bloque `PARTICIPACION`, el
//! estado y su foto; de dónde salen los puntos y la proyección son de `nucleo::participacion`.
//!
//! Formato en la foto (desde BAN49, little endian de `codec`): `n u64 · n × (plaza u64 · m u64 · m × (héroe u64 · puntos u64))`.

use crate::codec::{ErrorFormato, Escritor, Lector};
use std::collections::BTreeMap;

pub const DOC_PARTICIPACION: &str = include_str!("../../../Docs/Mechanics/Participacion.md");

/// Los puntos de cada origen (Participacion §2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reglas {
    /// Por defender una plaza, un pueblo, una aldea o una caravana.
    pub defensa: u64,
    /// Por ganar una batalla de cualquier tipo.
    pub batalla_ganada: u64,
}

impl Reglas {
    pub fn cargar() -> Self {
        let v = asentamiento::datos::bloque_de(DOC_PARTICIPACION, "PARTICIPACION").unwrap_or_else(|e| panic!("{e}"));
        let campo = |c: &str| v.get(c).and_then(|x| x.as_u64()).unwrap_or_else(|| panic!("bloque PARTICIPACION: falta `{c}`"));
        Self { defensa: campo("defensa"), batalla_ganada: campo("batallaGanada") }
    }
}

/// Plaza → héroe → puntos. Solo hay plazas con algún punto.
pub type EstadoParticipacion = BTreeMap<u64, BTreeMap<u64, u64>>;

pub(crate) fn escribir(w: &mut Escritor, e: &EstadoParticipacion) {
    w.u64(e.len() as u64);
    for (plaza, heroes) in e {
        w.u64(*plaza);
        w.u64(heroes.len() as u64);
        for (h, p) in heroes {
            w.u64(*h);
            w.u64(*p);
        }
    }
}

pub(crate) fn leer(r: &mut Lector) -> Result<EstadoParticipacion, ErrorFormato> {
    let mut e = EstadoParticipacion::new();
    for _ in 0..r.cuenta()? {
        let plaza = r.u64()?;
        let mut heroes = BTreeMap::new();
        for _ in 0..r.cuenta()? {
            heroes.insert(r.u64()?, r.u64()?);
        }
        e.insert(plaza, heroes);
    }
    Ok(e)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn las_cifras_del_documento_y_la_foto_de_ida_y_vuelta() {
        assert_eq!(Reglas::cargar(), Reglas { defensa: 5, batalla_ganada: 5 });
        let e: EstadoParticipacion = [(3, [(7, 10), (9, 5)].into()), (4, [(7, 5)].into())].into();
        let mut w = Escritor(Vec::new());
        escribir(&mut w, &e);
        let bytes = w.0;
        let mut r = Lector::nuevo(&bytes);
        assert_eq!(leer(&mut r).unwrap(), e);
        r.fin().unwrap();
    }
}
