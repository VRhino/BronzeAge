//! Dar la recompensa de una misión al reclamarla (mecánica 66; `Docs/Mechanics/Misiones.md` §4 y `Misiones_Recurrentes.md` §5): un solo reparto para las cadenas y las
//! recurrentes. Cada orden de reclamar dice quién cobra ([`Cobro`]) y [`Nucleo::repartir`] lo da todo o nada. Lo puro (leer y validar la recompensa) es de `crate::recompensas`.

use super::*;
use crate::estado::Dato;
use crate::recompensas::{Elemento, Fija, Recompensa};
use asentamiento::Recurso;
use militar::catalogo::Tropa;

/// Adónde van los recursos, el oro y la tropa: una misión personal paga al héroe; una colectiva, a la plaza o a la capital de la Facción.
pub(super) enum Destino {
    /// El almacén personal del héroe y, la tropa, su campamento.
    Heroe(u64),
    /// El almacén de la plaza.
    Plaza(u64),
    /// El almacén de la capital efectiva de la Facción ([`Nucleo::capital_de`]).
    Capital(u32),
}

/// Quién cobra una recompensa.
pub(super) struct Cobro {
    pub destino: Destino,
    /// La Facción que suma la experiencia (la fija y la de la lista). `None`: no se suma.
    pub faccion: Option<u32>,
    /// La plaza y los héroes entre los que se reparte la participación fija, a partes iguales (división entera; el resto se pierde).
    pub participacion: Option<(u64, Vec<u64>)>,
}

/// Lo que se dio, para el hecho `mision.reclamada`.
pub(super) struct Dado {
    pub datos: Vec<(&'static str, i64)>,
    pub valores: Vec<(&'static str, Dato)>,
}

impl Nucleo {
    /// Reclamar es todo o nada: comprueba que todo cabe y solo entonces lo da todo. Primero el destino de los recursos, el oro y la tropa (lo único que puede rechazar) y después la
    /// participación, los objetos y la experiencia, que no fallan. Devuelve lo que se dio de verdad.
    pub(super) fn repartir(&mut self, t: Instante, r: &Recompensa, cobro: Cobro) -> Result<Dado, Respuesta> {
        let mut entregas = Vec::new();
        let mut tropas = Vec::new();
        let mut objetos = Vec::new();
        let mut xp = match r.fija {
            Some(Fija::Experiencia(x)) => x,
            Some(Fija::Participacion(_)) | None => 0,
        };
        for e in &r.lista {
            match e {
                &Elemento::Recurso(rec, n) => entregas.push((rec, n)),
                &Elemento::Oro(n) => entregas.push((Recurso::Oro, n)),
                &Elemento::Experiencia(n) => xp += n,
                &Elemento::Tropa(tropa, n) => tropas.push((tropa, n)),
                Elemento::Objeto(o, n) => objetos.push((o.clone(), *n)),
            }
        }
        let mut dado = Dado { datos: Vec::new(), valores: Vec::new() };
        let mut escuadras = Vec::new();
        // Si la lista no lleva nada para el destino, no se mira.
        if !entregas.is_empty() || !tropas.is_empty() {
            match cobro.destino {
                Destino::Heroe(h) => escuadras = self.dar_al_heroe(t, h, &entregas, &tropas)?,
                Destino::Plaza(a) => self.dar_a_la_plaza(a, &entregas)?,
                Destino::Capital(f) => {
                    let capital = self.capital_de(f).ok_or_else(|| Respuesta::rechazo(rechazos::MISION_SIN_CAPITAL, format!("{} no tiene plaza donde dejar la recompensa", ids::faccion(f)), self.estado.version))?;
                    self.dar_a_la_plaza(capital, &entregas)?;
                }
            }
        }
        if let (Some(Fija::Participacion(puntos)), Some((plaza, mut heroes))) = (r.fija, cobro.participacion) {
            // Un mercenario contratado (marcado) no cobra Participación ni cuenta en el reparto: su parte es de los demás.
            heroes.retain(|&h| !self.es_marcado(h));
            let parte = puntos.checked_div(heroes.len() as u64).unwrap_or(0);
            if parte > 0 {
                self.sumar_participacion(plaza, &heroes, parte);
                dado.datos.push(("participacion", parte as i64));
            }
        }
        // Los objetos (solo en una misión personal, que paga al héroe): a su mochila y, si no caben, a su almacén de objetos; si tampoco, se pierden. No impiden reclamar
        // y los cuenta su hecho `heroe.botin` (D17 y D22).
        if let Destino::Heroe(h) = cobro.destino {
            self.entregar_objetos(t, h, "mision", objetos);
        }
        if let (Some(f), true) = (cobro.faccion, xp > 0) {
            self.dar_experiencia(t, f, xp, super::experiencia::motivo::MISION);
            dado.datos.push(("xp", xp));
        }
        if let Some(&(_, oro)) = entregas.iter().find(|(rec, _)| *rec == Recurso::Oro) {
            dado.datos.push(("oro", oro));
        }
        let recursos: Vec<Dato> = entregas.iter().filter(|(rec, _)| *rec != Recurso::Oro).map(|&(rec, n)| Dato::Lista(vec![Dato::Texto(rec.nombre().to_owned()), Dato::Numero(n)])).collect();
        if !recursos.is_empty() {
            dado.valores.push(("recursos", Dato::Lista(recursos)));
        }
        let escuadras: Vec<Dato> = escuadras.into_iter().filter(|&(_, sumados, _)| sumados > 0).map(|(tropa, sumados, id)| Dato::Lista(vec![Dato::Texto(tropa.nombre().to_owned()), Dato::Numero(sumados), Dato::Texto(ids::escuadra(id))])).collect();
        if !escuadras.is_empty() {
            dado.valores.push(("escuadras", Dato::Lista(escuadras)));
        }
        Ok(dado)
    }

    /// Los recursos y el oro de `entregas` al almacén personal de `heroe` y cada tropa a su escuadra, en su campamento ([`Nucleo::sumar_soldados`]; sin Liderazgo, ni rechazo si ya
    /// la tiene llena: lo que sobra se pierde). Rechaza sin cambiar nada si no existe, no reside en ningún sitio (la tropa nace en el campamento donde reside) o el almacén no admite todo
    /// junto. Devuelve, por tropa, los soldados que se sumaron de verdad y la escuadra.
    fn dar_al_heroe(&mut self, t: Instante, heroe: u64, entregas: &[(Recurso, Mil)], tropas: &[(Tropa, i64)]) -> Result<Vec<(Tropa, i64, u64)>, Respuesta> {
        let v = self.estado.version;
        let hn = self.heroe(heroe).ok_or_else(|| Respuesta::rechazo(rechazos::HEROE_DESCONOCIDO, ids::heroe(heroe), v))?;
        if hn.heroe.residencia.is_none() {
            return Err(Respuesta::rechazo(rechazos::HEROE_SIN_RESIDENCIA, "sin residencia no hay almacén personal ni campamento donde dejar la recompensa", v));
        }
        let lleva: Mil = hn.almacen_personal.values().sum();
        if lleva + entregas.iter().map(|&(_, n)| n).sum::<Mil>() > self.estado.reglas.economia.comercio.mercenarios.capacidad_almacen_personal {
            return Err(Respuesta::rechazo(rechazos::ALMACEN_PERSONAL_LLENO, "la recompensa no cabe en tu almacén personal", v));
        }
        for &(rec, n) in entregas {
            *self.heroe_mut(heroe).almacen_personal.entry(rec).or_insert(0) += n;
        }
        Ok(tropas
            .iter()
            .map(|&(tropa, soldados)| {
                let (id, _, sumados) = self.sumar_soldados(t, heroe, tropa, soldados);
                (tropa, sumados, id)
            })
            .collect())
    }

    /// Los recursos y el oro de `entregas` al almacén de la plaza `a`. Rechaza sin cambiar nada si no existe o alguno no cabe.
    fn dar_a_la_plaza(&mut self, a: u64, entregas: &[(Recurso, Mil)]) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let plaza = self.estado.asentamientos.get_mut(&a).ok_or_else(|| Respuesta::rechazo(rechazos::ASENTAMIENTO_DESCONOCIDO, ids::asentamiento(a), v))?;
        if entregas.iter().any(|&(rec, n)| plaza.economia.almacen.espacio(rec) < n) {
            return Err(Respuesta::rechazo(rechazos::MISION_ALMACEN_LLENO, "la recompensa no cabe en el almacén de la plaza", v));
        }
        for &(rec, n) in entregas {
            plaza.economia.almacen.agregar(rec, n);
        }
        self.tr.tocar(Clase::Asentamiento, a);
        Ok(())
    }
}
