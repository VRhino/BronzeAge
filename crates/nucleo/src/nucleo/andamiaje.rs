//! Órdenes de administración solo para pruebas (2026-11-16, pedidas para probar en Conquest lo que una partida nueva no tiene): dar monedas a un héroe, llenar el fondo de monedas de una
//! plaza y colocar un Aeda (un itinerante parado en una plaza o un residente). No son fuentes de juego: como `DarRecursos` o `DarEscuadra`, son andamiaje de escenarios.

use super::*;
use crate::estado::entidad_aeda;
use tecnologia::itinerantes::AedaItinerante;

impl Nucleo {
    /// `DarMonedas`, `DarFondoMonedas` y `ColocarAeda`. `Ok(ids)` = cambió algo y creó esos ids (el número del Aeda colocado); los rechazos salen como `Err` sin haber tocado nada.
    pub(super) fn ejecutar_pruebas(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let positiva = |cantidad: i64| if cantidad > 0 { Ok(()) } else { Err(Respuesta::rechazo(rechazos::CANTIDAD_NO_POSITIVA, "la cantidad tiene que ser positiva", v)) };
        match p.comando {
            Comando::DarMonedas { heroe, cantidad } => {
                positiva(cantidad)?;
                self.heroe(heroe).ok_or_else(|| Respuesta::rechazo(rechazos::HEROE_DESCONOCIDO, ids::heroe(heroe), v))?;
                let h = &mut self.heroe_mut(heroe).heroe;
                h.monedas = h.monedas.saturating_add(cantidad);
                self.tr.tocar(Clase::Heroe, heroe);
                Ok(vec![])
            }
            Comando::DarFondoMonedas { asentamiento, cantidad } => {
                positiva(cantidad)?;
                self.plaza_o_rechazo(asentamiento)?;
                let a = self.estado.asentamientos.get_mut(&asentamiento).expect("existe");
                a.forja.fondo = a.forja.fondo.saturating_add(cantidad);
                self.tr.tocar(Clase::Asentamiento, asentamiento);
                Ok(vec![])
            }
            Comando::ColocarAeda { asentamiento, residente } => {
                let (faccion, posicion) = {
                    let a = self.plaza_o_rechazo(asentamiento)?;
                    (a.faccion, a.posicion)
                };
                if residente {
                    let aeda = self.estado.residentes.llega(asentamiento as u32, faccion, t);
                    self.contar_residente(asentamiento, 1);
                    self.tocar_residente(aeda);
                    self.tocar_residentes(faccion, asentamiento);
                    self.emitir_propio(t, "aedas.residente_llega", vec![asentamiento], vec![("aedaResidente", i64::from(aeda)), ("faccion", i64::from(faccion))]);
                    return Ok(vec![u64::from(aeda)]);
                }
                Ok(vec![u64::from(self.colocar_itinerante(asentamiento, posicion, t))])
            }
            _ => unreachable!("solo las órdenes de administración para pruebas"),
        }
    }

    /// Para en `plaza` al Aeda itinerante que ya está en ella o, si no, al más cercano (a igual distancia, el de menor número), con su estancia entera, como si acabara de llegar; si no hay
    /// ninguno, nace uno allí. Su evento pendiente deja de valer y se programa el fin de la estancia; lo que conoce se revela al cerrar la versión. Devuelve su número.
    fn colocar_itinerante(&mut self, plaza: u64, posicion: Punto, t: Instante) -> u32 {
        let r = self.estado.reglas.economia.tecnologia.clone();
        let ya = self.estado.aedas.iter().position(|a| a.en == Some(plaza as u32));
        let cercano = || self.estado.aedas.iter().enumerate().min_by_key(|(_, a)| (a.plan.as_ref().and_then(|p| p.posicion_en(t)).unwrap_or(a.posicion).dist2(posicion), a.id)).map(|(i, _)| i);
        let i = match ya.or_else(cercano) {
            Some(i) => i,
            None => {
                let id = self.estado.aedas.len() as u32 + 1;
                self.estado.aedas.push(AedaItinerante { id, posicion, en: None, hasta: None, destino: None, plan: None, recientes: Vec::new() });
                self.estado.aedas.len() - 1
            }
        };
        let id = self.estado.aedas[i].id;
        self.estado.aedas[i].llegar(&r, plaza as u32, posicion, t);
        self.estado.generaciones.subir(entidad_aeda(id));
        self.programar_aeda(i);
        self.revelar_pendiente.insert(plaza);
        id
    }
}
