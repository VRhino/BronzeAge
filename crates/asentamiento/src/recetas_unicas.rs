//! Bodega y prensa (`TALLERES_DE_UNA_RECETA`, 2026-11-09): cada edificio trabaja una sola de sus recetas, la que dice su insumo
//! activo (`Edificio.receta`). La plaza tiene un tope de edificios de cada tipo y otro de insumos distintos según su nivel operativo;
//! si el nivel baja y sobran recetas, paran las de los insumos que empezaron a usarse más tarde. Nada de esto se guarda: se deriva.

use crate::asentamiento::Asentamiento;
use crate::catalogo::{Catalogo, Tipo};
use crate::datos::Reglas;
use crate::edificio::{Edificio, Edificios};
use crate::recurso::Recurso;

/// Por qué no se elige una receta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorReceta {
    NoExiste,
    /// El edificio no es una bodega ni una prensa.
    NoSeElige,
    /// Ninguna receta del nivel interno del edificio consume ese insumo.
    Desconocida,
    /// Con el cambio, la plaza trabajaría más insumos distintos en ese tipo de los que caben en su nivel.
    TopePorPlaza,
    /// Ya trabaja con ese insumo.
    SinCambios,
}

/// El insumo de la primera receta del catálogo del tipo (vino←uva, aceite←aceituna): la receta de partida.
pub fn insumo_por_defecto(cat: &Catalogo, tipo: Tipo) -> Option<Recurso> {
    let primera = cat.tipo(tipo).nivel(1)?.recetas.first()?;
    primera.consume_por_unidad.first().map(|&(insumo, _)| insumo)
}

/// El insumo de la receta que trabaja el edificio: el elegido o, sin elegir, el de partida. `None` si el tipo no trabaja una sola receta.
pub fn insumo_activo(cat: &Catalogo, e: &Edificio) -> Option<Recurso> {
    if !e.tipo.de_una_receta() {
        return None;
    }
    e.receta.or_else(|| insumo_por_defecto(cat, e.tipo))
}

/// ¿Alguna receta del nivel interno del edificio consume ese insumo?
fn consume_el_insumo(cat: &Catalogo, e: &Edificio, insumo: Recurso) -> bool {
    cat.tipo(e.tipo).nivel(e.nivel_interno).is_some_and(|n| n.recetas.iter().any(|rc| rc.consume_por_unidad.iter().any(|&(i, _)| i == insumo)))
}

/// Los insumos distintos que trabajan los edificios de ese tipo (de cualquier estado), por el menor id que usa cada uno.
/// `cambio` sustituye el insumo de un edificio (el que se está eligiendo).
fn insumos_distintos(cat: &Catalogo, edificios: &[Edificio], tipo: Tipo, cambio: Option<(u32, Recurso)>) -> Vec<Recurso> {
    let mut usos: Vec<(u32, Recurso)> = edificios
        .iter()
        .filter(|e| e.tipo == tipo)
        .filter_map(|e| match cambio {
            Some((id, insumo)) if id == e.id => Some((e.id, insumo)),
            _ => insumo_activo(cat, e).map(|i| (e.id, i)),
        })
        .collect();
    usos.sort_unstable_by_key(|&(id, _)| id);
    let mut distintos: Vec<Recurso> = vec![];
    for (_, insumo) in usos {
        if !distintos.contains(&insumo) {
            distintos.push(insumo);
        }
    }
    distintos
}

/// Los ids de los edificios cuya receta está parada porque la plaza, en ese nivel operativo, no cabe en su tope de recetas distintas:
/// siguen trabajando los primeros insumos por el menor id que los usa. Orden ascendente por id.
pub fn recetas_paradas(r: &Reglas, edificios: &[Edificio], nivel: u32) -> Vec<u32> {
    let mut paradas: Vec<u32> = vec![];
    for (&tipo, taller) in &r.construccion.talleres_de_una_receta {
        let distintos = insumos_distintos(&r.catalogo, edificios, tipo, None);
        let caben = taller.recetas_distintas(nivel) as usize;
        if distintos.len() <= caben {
            continue;
        }
        let sobran = &distintos[caben..];
        let del_tipo = edificios.iter().filter(|e| e.tipo == tipo);
        paradas.extend(del_tipo.filter(|e| insumo_activo(&r.catalogo, e).is_some_and(|i| sobran.contains(&i))).map(|e| e.id));
    }
    paradas.sort_unstable();
    paradas
}

impl Asentamiento {
    /// ¿La plaza ya tiene todos los edificios de ese tipo que caben (cualquier estado)? Solo la bodega y la prensa tienen tope.
    pub(crate) fn tope_por_plaza(&self, r: &Reglas, tipo: Tipo) -> bool {
        r.construccion.talleres_de_una_receta.get(&tipo).is_some_and(|t| self.edificios.en_cualquier_estado(tipo) as u32 >= t.maximo_por_plaza)
    }

    /// La receta con la que nace un edificio nuevo de ese tipo: la que trabaja la plaza (la del menor id que no está parada) o, si no hay
    /// ninguno, la primera del catálogo.
    pub(crate) fn receta_de_partida(&self, r: &Reglas, tipo: Tipo) -> Option<Recurso> {
        if !tipo.de_una_receta() {
            return None;
        }
        let paradas = recetas_paradas(r, &self.edificios, self.nivel_actual());
        let vigente = self.edificios.iter().filter(|e| e.tipo == tipo && !paradas.contains(&e.id)).min_by_key(|e| e.id);
        vigente.and_then(|e| insumo_activo(&r.catalogo, e)).or_else(|| insumo_por_defecto(&r.catalogo, tipo))
    }

    /// Los ids de los edificios con la receta parada en el nivel operativo de ahora.
    pub fn recetas_paradas(&self, r: &Reglas) -> Vec<u32> {
        recetas_paradas(r, &self.edificios, self.nivel_actual())
    }

    /// `elegirReceta`: el edificio pasa a trabajar la receta de ese insumo. Se comprueba el tope de recetas distintas del nivel operativo
    /// al elegir (con el cambio hecho, contando los edificios de cualquier estado).
    pub fn elegir_receta(&mut self, r: &Reglas, edificio: u32, insumo: Recurso) -> Result<(), ErrorReceta> {
        let i = self.edificios.iter().position(|e| e.id == edificio).ok_or(ErrorReceta::NoExiste)?;
        let e = &self.edificios[i];
        let Some(taller) = r.construccion.talleres_de_una_receta.get(&e.tipo) else { return Err(ErrorReceta::NoSeElige) };
        if !consume_el_insumo(&r.catalogo, e, insumo) {
            return Err(ErrorReceta::Desconocida);
        }
        if insumo_activo(&r.catalogo, e) == Some(insumo) {
            return Err(ErrorReceta::SinCambios);
        }
        if insumos_distintos(&r.catalogo, &self.edificios, e.tipo, Some((edificio, insumo))).len() as u32 > taller.recetas_distintas(self.nivel_actual()) {
            return Err(ErrorReceta::TopePorPlaza);
        }
        self.edificios[i].receta = Some(insumo);
        Ok(())
    }
}
