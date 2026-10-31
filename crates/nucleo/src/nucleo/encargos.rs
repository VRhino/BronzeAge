//! Los encargos de caravana en el núcleo (Comercio_Caravanas §5.8): rutas y envíos automáticos entre una plaza propia y otra plaza.
//!
//! En implementación: las órdenes 195-197 llegan aquí y aún no cambian nada.

use super::*;

impl Nucleo {
    /// Las órdenes de encargo (`EncargarCaravana`, `CambiarEncargo`, `CancelarEncargo`). `Ok(())` = cambió algo; rechazos y «sin cambios»
    /// salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_encargos(&mut self, _t: Instante, _p: &Peticion) -> Result<(), Respuesta> {
        Err(Respuesta::sin_cambios(self.estado.version))
    }
}
