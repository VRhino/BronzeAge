//! Los títulos de prestigio, la Liga y el Gran Rey (sistema 5, tanda 5F; Diplomacia_Facciones §5.7 y §7, Doc 2.3 y 2.9). Sin beneficio mecánico: los cantan los Aedas.
//!
//! # Por eventos, sin tick
//! Un solo plazo **global** en la cola (`Carga::Titulos`, entidad `entidad_titulos`, prioridad 10: después de todo lo del mismo instante), a `TITULOS.cadenciaMinutos` del momento en que se arma.
//! Se arma cuando pasa algo en el mundo —una orden aceptada o un evento de la cola que no sea él mismo— y no hay otro pendiente (`Estado::titulos_en`); al vencer no se rearma: lo hará lo siguiente
//! que pase. Un mundo quieto (o sin Facciones) no deja nada en la cola; uno con plazas, cuyo paso de minuto cambia el oro de todas, recalcula cada `cadenciaMinutos` (C15, decisión 12 del plan).
//!
//! # Lo que se mide
//! De cada Facción como entidad, en el orden de sus ids (desempata el primero, como el orden de la lista del TypeScript): sus plazas y la población total de ellas, el oro de sus almacenes en
//! unidades, los soldados de todas las escuadras de todos sus ciudadanos (estén donde estén) y, hasta el sistema 6, ningún mecenazgo (el título «Mecenas de los Aedas» no nace). La Liga y el Gran
//! Rey salen de las relaciones activas (`politica::liga`), bajo demanda: no se guardan. Se guardan los títulos de la última evaluación (`Politica::titulos`, foto), con los que se narra la siguiente
//! (`titulo.nace`, `titulo.cambia_manos`, hechos públicos que la crónica canta; nada cuando un título desaparece). Un título que cambia de poseedor o de valor, nace o desaparece se toca en el
//! registro (`Clase::Titulo`, id = su posición en `TituloId::TODOS`): viaja a la `VISTA` de todos y a `ADMIN`.

use super::*;
use crate::estado::{entidad_titulos, Dato, PRIO_TITULOS};
use politica::titulos::{Cambio as CambioDeTitulo, Medida, Titulo, TituloId};

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Los títulos de la última evaluación.
    pub fn titulos(&self) -> &[Titulo] {
        &self.estado.politica.titulos
    }

    /// Las Ligas de ahora (§5.7): componentes conexas de las relaciones activas entre Facciones que existen, con su Gran Rey.
    pub fn ligas(&self) -> Vec<politica::liga::Liga> {
        politica::liga::ligas(&self.estado.politica.relaciones, self.estado.politica.facciones.iter().map(|f| f.id))
    }

    /// Lo que se mide de cada Facción como entidad para los títulos, en el orden de sus ids.
    pub fn medidas_de_titulos(&self) -> Vec<Medida> {
        let e = &self.estado;
        e.politica
            .facciones
            .iter()
            .map(|f| {
                let plazas: Vec<&crate::estado::Asentamiento> = self.plazas_de(f.id).collect();
                let soldados = f.ciudadanos.iter().filter_map(|h| e.militar.heroes.get(h)).flat_map(|h| &h.heroe.escuadras).map(|s| s.cantidad.max(0)).sum();
                Medida {
                    faccion: f.id,
                    plazas: plazas.len() as u32,
                    poblacion: plazas.iter().map(|a| a.economia.poblacion.total()).sum(),
                    oro: plazas.iter().map(|a| a.economia.almacen.cantidad(asentamiento::Recurso::Oro) / base::UNIDAD).sum(),
                    soldados,
                    // DUDA: épicas cumplidas × 10 + Aedas residentes, cuando existan (sistema 6).
                    mecenas: 0,
                }
            })
            .collect()
    }

    // ───────────────────────────── El plazo ─────────────────────────────

    /// Arma el plazo de los títulos si no hay uno pendiente y hay alguna Facción: lo llama todo lo que pasa en el mundo salvo el propio plazo.
    pub(super) fn armar_titulos(&mut self, t: Instante) {
        if self.estado.titulos_en.is_some() || self.estado.politica.facciones.iter().next().is_none() {
            return;
        }
        let en = t + self.estado.reglas.economia.politica.cadencia_titulos;
        let clave = self.estado.generaciones.clave(entidad_titulos());
        self.estado.cola.programar(en, PRIO_TITULOS, clave, Carga::Titulos);
        self.estado.titulos_en = Some(en);
    }

    /// Vence el plazo: se recalculan los títulos con lo de ahora y se narran los cambios.
    pub(super) fn recalcular_titulos(&mut self, t: Instante) {
        self.estado.titulos_en = None;
        let nuevos = politica::titulos::calcular(&self.medidas_de_titulos(), &self.ligas());
        let anteriores = std::mem::replace(&mut self.estado.politica.titulos, nuevos);
        let nuevos = &self.estado.politica.titulos;
        for id in TituloId::TODOS {
            let (antes, ahora) = (anteriores.iter().find(|x| x.id == id), nuevos.iter().find(|x| x.id == id));
            if antes != ahora {
                self.tr.tocar(Clase::Titulo, id.indice());
            }
        }
        for c in politica::titulos::narrar(&anteriores, &self.estado.politica.titulos.clone()) {
            let (codigo, titulo, datos) = match c {
                CambioDeTitulo::Nace { titulo, faccion, valor } => ("titulo.nace", titulo, vec![("faccion", i64::from(faccion)), ("valor", valor)]),
                CambioDeTitulo::CambiaManos { titulo, previo, actual } => ("titulo.cambia_manos", titulo, vec![("faccion", i64::from(actual)), ("faccionAnterior", i64::from(previo))]),
            };
            let valores = vec![("titulo", Dato::Texto(titulo.clave().into())), ("nombre", Dato::Texto(titulo.nombre().into()))];
            self.tr.emitir(&self.estado, EventoNucleo { valores, publico: true, ..EventoNucleo::nuevo(codigo, t, datos) });
        }
    }
}
