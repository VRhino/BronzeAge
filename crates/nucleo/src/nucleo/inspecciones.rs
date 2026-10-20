//! Inspeccionar (Movimiento_Ejercitos §12, Doc 5.12.3, fase 4C): mirar de cerca, a ≤ `radioInspeccion`, una columna ajena, una caravana o la defensa de
//! una plaza de otra Facción. Las condiciones y la forma del informe son de `militar::inspeccion`; aquí se mira el estado, se guarda el informe en el héroe
//! y se avisa al observado: obtener información cuesta ser visto.
//!
//! Lo que se ve es lo que la niebla niega de lejos y nada más: de una columna, sus héroes y cada escuadra con soldados (tropa, cuántos, de quién); de una
//! caravana, si va escoltada y **qué** recursos lleva (nunca cuántos); de una plaza, la guarnición de sus residentes y los héroes que hay dentro (nunca su
//! almacén). El informe dura `INSPECCION.informeMinutos` y se lee con [`Nucleo::informes_de_inspeccion`].

use super::*;
use militar::escuadra::Contenedor;
use militar::inspeccion::{self, Avistada, Contenido, ErrorInspeccion, Informe, Objetivo};
use militar::presencia::Ubicacion;

impl Nucleo {
    /// Los informes de inspección vigentes de un héroe, en el orden en que los hizo.
    pub fn informes_de_inspeccion(&self, heroe: u64) -> Vec<&Informe> {
        self.heroe(heroe).into_iter().flat_map(|h| h.informes.iter()).filter(|i| i.vigente(self.estado.ahora)).collect()
    }

    /// `inspeccionar`. La puede dar quien manda en el héroe, que tiene que ir a bordo de una columna que no esté en batalla. Todo se comprueba antes de
    /// guardar el informe.
    pub(super) fn inspeccionar(&mut self, t: Instante, p: &Peticion, heroe: u64, objetivo: Objetivo) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let (id, e) = self.columna_a_bordo(heroe)?;
        if e.en_batalla {
            return Err(rechazo(rechazos::COLUMNA_EN_BATALLA, "la columna está apartada por una batalla".into()));
        }
        let (aqui, faccion) = (e.posicion_en(t), e.columna.faccion);
        let a_cubierto = || rechazo(rechazos::INSPECCION_A_CUBIERTO, "no está en el mapa: no se la puede mirar".into());

        // Qué se mira: dónde está, si es de los suyos, lo que se ve y a quién se avisa.
        let (donde, propio, contenido, aviso) = match objetivo {
            Objetivo::Ejercito(x) => {
                let o = self.estado.ejercitos.get(&x).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{x}")))?;
                if !self.columna_en_el_mapa(o) {
                    return Err(a_cubierto());
                }
                let escuadras = self.escuadras_de_columna(x).into_iter().filter(|s| s.con_soldados()).map(|s| Avistada { tropa: s.tropa, cantidad: s.cantidad, heroe: s.heroe }).collect();
                let heroes = Self::participantes_de(o);
                (o.posicion_en(t), x == id, Contenido::Columna { faccion: o.columna.faccion, heroes, escuadras }, ("columna.observada", Some(x), vec![]))
            }
            Objetivo::Caravana(x) => {
                let k = self.estado.comercio.caravana(x).ok_or_else(|| rechazo(rechazos::CARAVANA_DESCONOCIDA, format!("caravana-{x}")))?;
                if !matches!(k.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando | EstadoCaravana::Adjunta) {
                    return Err(a_cubierto());
                }
                let recursos = k.contenido.iter().filter(|(_, &n)| n > 0).map(|(&r, _)| r).collect();
                let donde = self.posicion_de_caravana(x, t).unwrap_or(k.posicion);
                (donde, false, Contenido::Caravana { escoltada: k.estado == EstadoCaravana::Adjunta || !k.escolta.is_empty(), recursos }, ("caravana.observada", None, vec![k.origen as u64]))
            }
            Objetivo::Asentamiento(x) => {
                let a = self.plaza_o_rechazo(x)?;
                let dentro = self.heroes().filter(|h| h.heroe.presencia.ubicacion == Ubicacion::Asentamiento(x as u32)).map(|h| h.heroe.id).collect();
                let guarnicion = self.estado.militar.residentes_de(x).flat_map(|h| h.heroe.escuadras.iter()).filter(|s| s.contenedor == Contenedor::Campamento && s.en_guarnicion && s.con_soldados());
                let guarnicion = guarnicion.map(|s| Avistada { tropa: s.tropa, cantidad: s.cantidad, heroe: s.heroe }).collect();
                (a.posicion, faccion == Some(a.faccion), Contenido::Plaza { faccion: a.faccion, heroes: dentro, guarnicion }, ("asentamiento.observado", None, vec![x]))
            }
        };
        let mr = self.estado.reglas.economia.militar.clone();
        inspeccion::validar(&mr, objetivo, propio, aqui.dist(donde)).map_err(|er| {
            let (c, m) = match er {
                ErrorInspeccion::LaTuya => (rechazos::INSPECCION_LA_TUYA, "esa columna es la tuya"),
                ErrorInspeccion::PlazaPropia => (rechazos::INSPECCION_PLAZA_PROPIA, "esa plaza es de tu Facción"),
                ErrorInspeccion::Lejos => (rechazos::INSPECCION_LEJOS, "hay que acercarse más para inspeccionar"),
            };
            rechazo(c, m.into())
        })?;

        let informes = &mut self.heroe_mut(heroe).informes;
        inspeccion::anotar(&mr, informes, objetivo, t, donde, contenido);
        // El aviso es para el observado: la columna, o la plaza (la de origen, si es una caravana).
        let (codigo, columna, plazas) = aviso;
        let datos = vec![("observador", heroe as i64)];
        match columna {
            Some(x) => self.evento_de_columna(codigo, t, x, vec![], datos),
            None => self.emitir_propio(t, codigo, plazas, datos),
        }
        Ok(vec![])
    }
}
