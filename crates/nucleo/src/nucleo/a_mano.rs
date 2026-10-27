//! El almacén personal y las caravanas a mano (5G-a, 2026-10-24; Movimiento_Ejercitos §3.3 y §9, Heroe_Y_Tropa §4.4): `guardarEnAlmacenPersonal`, `sacarDelAlmacenPersonal`,
//! `cargarCaravana`, `reclamarCaravana` y `soltarCaravana`. Todo en unidades enteras (C3b): lo que se mueve se redondea hacia abajo, y nada se crea ni se pierde (lo que sale de un sitio
//! entra en el otro).
//!
//! - **El almacén personal vive donde se reside**: se llena y se vacía con el carro, dentro de la residencia (el carro que trajo, decisión del autor 5G-a-2) o con la columna a su
//!   puerta, y entonces solo el Líder (el carro es común).
//! - **`cargarCaravana`** carga en una caravana adjunta del almacén de una plaza de su Facción a cuya puerta está la columna, sin reserva (como el reparto automático), hasta el hueco
//!   que le queda de verdad: ni la caravana ni la columna pasan de su límite (5G-a-4).
//! - **`soltarCaravana`** es aparcar en la plaza dentro de la cual está la columna (entró con todo); fuera, a la puerta o en mundo abierto, se rechaza (5G-a-5).
//! - **`reclamarCaravana`** vale solo para la Caravana de Fundación que vuelve sola (su caso de éxito está en `nucleo/fundacion.rs`, 5G-b); una comercial se rechaza con
//!   `caravana.no_reclamable`.

use super::*;
use crate::estado::Dato;
use crate::codec_economia::recurso_a_u8;
use asentamiento::Recurso;
use base::{entero, UNIDAD};
use comercio::{de, sumar, total, Carga as Mercancia};
use militar::convocatoria::Lugar;
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;
use crate::nucleo::mercado::CAPACIDAD_CARRO;

/// Quita `n` de `r` de la carga, borrando la línea que queda a cero.
fn quitar(carga: &mut Mercancia, r: Recurso, n: Mil) {
    if let Some(x) = carga.get_mut(&r) {
        *x -= n;
        if *x <= 0 {
            carga.remove(&r);
        }
    }
}

impl Nucleo {
    pub(super) fn ejecutar_a_mano(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        match &p.comando {
            Comando::GuardarEnAlmacenPersonal { heroe, recurso, cantidad } => self.mover_almacen_personal(t, p, *heroe, *recurso, *cantidad, true),
            Comando::SacarDelAlmacenPersonal { heroe, recurso, cantidad } => self.mover_almacen_personal(t, p, *heroe, *recurso, *cantidad, false),
            Comando::CargarCaravana { ejercito, caravana, asentamiento, recurso, cantidad } => self.cargar_caravana(t, p, *ejercito, *caravana, *asentamiento, *recurso, *cantidad),
            Comando::ReclamarCaravana { heroe, caravana, politica } => self.reclamar_caravana(t, p, *heroe, *caravana, *politica),
            Comando::SoltarCaravana { ejercito, caravana, heroe } => self.soltar_caravana(t, p, *ejercito, *caravana, *heroe),
            _ => unreachable!("solo se llama con las órdenes del almacén personal y de las caravanas a mano"),
        }
    }

    /// `guardarEnAlmacenPersonal` (`guarda`) y `sacarDelAlmacenPersonal`: del carro al almacén personal hasta su capacidad, o del almacén al carro hasta lo que cabe en él. **Dentro de su
    /// residencia** el carro es el que trajo (en la columna que conserva para sus caravanas, si la conserva; decisión del autor 5G-a-2); **fuera**, el de su columna, con la columna a la
    /// puerta de su residencia y siendo su Líder (la administración salta la puerta).
    fn mover_almacen_personal(&mut self, t: Instante, p: &Peticion, heroe: u64, recurso: Recurso, cantidad: Mil, guarda: bool) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let hn = self.heroe(heroe).expect("existe");
        let (lugar, dentro) = match (hn.heroe.residencia, hn.heroe.presencia.ubicacion) {
            (Some(Residencia::Asentamiento(a)), u) => (Lugar::Asentamiento(a), u == Ubicacion::Asentamiento(a)),
            (Some(Residencia::Campamento(c)), u) => (Lugar::Campamento(c), u == Ubicacion::Mercenarios(c)),
            (None, _) => return Err(rechazo(rechazos::HEROE_SIN_RESIDENCIA, "sin residencia no hay almacén personal".into())),
        };
        // `columna`: dónde va el carro (`None` = con el héroe, dentro); `capacidad`: lo que admite.
        let (columna, capacidad) = if dentro {
            match hn.columna.filter(|c| self.estado.ejercitos.contains_key(c)) {
                Some(c) => (Some(c), self.capacidad_de_carga(c).unwrap_or(0)),
                None => (None, CAPACIDAD_CARRO),
            }
        } else {
            let (c, e) = self.columna_a_bordo(heroe)?;
            if e.columna.lider != heroe {
                return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder de la columna mueve lo de su carro".into()));
            }
            if !p.admin && self.puerta_de(lugar).is_none_or(|puerta| e.posicion_en(t).dist(puerta) > self.radio_puerta()) {
                return Err(rechazo(rechazos::ALMACEN_PERSONAL_FUERA_DE_TU_RESIDENCIA, "el almacén personal está en tu residencia: dentro, o con tu columna a su puerta".into()));
            }
            (Some(c), self.capacidad_de_carga(c).unwrap_or(0))
        };
        let pedido = entero(cantidad);
        if pedido <= 0 {
            return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "hay que mover al menos una unidad".into()));
        }
        let carga = columna.map_or(&hn.carro, |c| &self.estado.ejercitos[&c].columna.carga);
        let movido = if guarda {
            let lleva = de(carga, recurso);
            let sitio = self.estado.reglas.economia.comercio.mercenarios.capacidad_almacen_personal - total(&hn.almacen_personal);
            let movido = entero(pedido.min(lleva).min(sitio));
            if movido <= 0 {
                return Err(if lleva < UNIDAD {
                    rechazo(rechazos::CARRO_NO_LLEVA, format!("el carro no lleva {}", recurso.nombre()))
                } else {
                    rechazo(rechazos::ALMACEN_PERSONAL_LLENO, "tu almacén personal está lleno".into())
                });
            }
            movido
        } else {
            let guardado = de(&hn.almacen_personal, recurso);
            let movido = entero(pedido.min(guardado).min(capacidad - total(carga)));
            if movido <= 0 {
                return Err(if guardado < UNIDAD {
                    rechazo(rechazos::ALMACEN_SIN_EXISTENCIAS, format!("no hay {} en tu almacén personal", recurso.nombre()))
                } else {
                    rechazo(rechazos::CARRO_LLENO, "el carro está lleno".into())
                });
            }
            movido
        };
        let hn = self.estado.militar.heroes.get_mut(&heroe).expect("existe");
        let carro = match columna {
            Some(c) => &mut self.estado.ejercitos.get_mut(&c).expect("existe").columna.carga,
            None => &mut hn.carro,
        };
        let (de_aqui, a_alli) = if guarda { (carro, &mut hn.almacen_personal) } else { (&mut hn.almacen_personal, carro) };
        quitar(de_aqui, recurso, movido);
        sumar(a_alli, recurso, movido);
        self.tr.tocar(Clase::Heroe, heroe);
        if let Some(c) = columna {
            self.tr.tocar(Clase::Ejercito, c);
        }
        let datos = vec![("heroe", heroe as i64), ("recurso", recurso_a_u8(recurso) as i64), ("cantidad", movido)];
        let sentido = Dato::Texto(if guarda { "guarda" } else { "saca" }.into());
        self.tr.emitir(&self.estado, EventoNucleo { valores: vec![("sentido", sentido)], ..EventoNucleo::nuevo("heroe.almacen_personal", t, datos) });
        Ok(vec![])
    }

    /// `cargarCaravana` (Movimiento_Ejercitos §9): `min(pedido, espacio, existencias de la plaza)`, sin reserva de trigo (es mercancía, como la del reparto automático). El espacio es
    /// su capacidad efectiva (con «Carga Ampliada» de su origen) menos lo que ya lleva, sin pasar del hueco que le queda a la columna (5G-a-4).
    #[allow(clippy::too_many_arguments)]
    fn cargar_caravana(&mut self, t: Instante, p: &Peticion, ejercito: u64, caravana: u64, plaza: u64, recurso: Recurso, cantidad: Mil) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let e = self.estado.ejercitos.get(&ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(ejercito)))?;
        if !self.manda_columna(p, e, false) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
        }
        let i = self.estado.comercio.posicion_caravana(caravana).ok_or_else(|| rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana)))?;
        if !e.caravanas_adjuntas.contains(&caravana) {
            return Err(rechazo(rechazos::CARAVANA_NO_ESTA_ADJUNTA, "la caravana no va con esta columna".into()));
        }
        let pedido = entero(cantidad);
        if pedido <= 0 {
            return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "hay que cargar al menos una unidad".into()));
        }
        let (aqui, faccion) = (e.posicion_en(t), e.columna.faccion);
        let a = self.plaza_o_rechazo(plaza)?;
        if !p.admin && aqui.dist(a.posicion) > self.radio_puerta() {
            return Err(rechazo(rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la plaza".into()));
        }
        // «Donde pueda repostar» (duda 3 de 5G-a, cerrada en 5D): la plaza propia, o la de una aliada o vasalla que abre su almacén a los aliados (`permiteReabastecerAliados`), como `repostar`.
        if faccion != Some(a.faccion) && !self.abre_su_almacen_a(plaza, faccion) {
            return Err(rechazo(rechazos::CARAVANA_PLAZA_AJENA, "esa plaza no abre su almacén a esta columna".into()));
        }
        let existencias = a.economia.almacen.cantidad(recurso);
        // El hueco de verdad (5G-a-4): el de la caravana, sin pasar del que le queda a la columna, cuyo carro común puede estar usando ya parte del de sus adjuntas.
        let libre_columna = self.capacidad_de_carga(ejercito).unwrap_or(0) - total(&self.estado.ejercitos[&ejercito].columna.carga);
        let c = &self.estado.comercio.flota[i];
        let espacio = (self.capacidad_de_caravana(c) - c.carga_total()).min(libre_columna);
        if espacio < UNIDAD {
            return Err(rechazo(rechazos::CARAVANA_SIN_CAPACIDAD, "la caravana ya va llena".into()));
        }
        let cargado = entero(pedido.min(espacio).min(existencias));
        if cargado <= 0 {
            return Err(rechazo(rechazos::ALMACEN_SIN_EXISTENCIAS, format!("no hay {} en el almacén de la plaza", recurso.nombre())));
        }
        let sacado = self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.almacen.quitar(recurso, cargado);
        debug_assert_eq!(sacado, cargado);
        sumar(&mut self.estado.comercio.flota[i].contenido, recurso, cargado);
        self.tr.tocar(Clase::Asentamiento, plaza);
        self.tr.tocar(Clase::Caravana, caravana);
        self.evento_de_columna("ejercito.caravana_cargada", t, ejercito, vec![plaza], vec![("caravana", caravana as i64), ("plaza", plaza as i64), ("recurso", recurso_a_u8(recurso) as i64), ("cargado", cargado)]);
        Ok(vec![])
    }

    /// `reclamarCaravana` (Movimiento_Ejercitos §9): solo la Caravana de Fundación que vuelve sola a su origen, en mundo abierto (`Nucleo::reclamar_fundacion`, 5G-b). Una caravana comercial nunca
    /// se reclama (`caravana.no_reclamable`).
    fn reclamar_caravana(&mut self, t: Instante, p: &Peticion, heroe: u64, caravana: u64, politica: militar::columna::Politica) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        self.autorizar_heroe(p, heroe, false)?;
        if self.estado.fundacion.caravanas.contains_key(&caravana) {
            return self.reclamar_fundacion(t, heroe, caravana, politica);
        }
        if self.estado.comercio.posicion_caravana(caravana).is_none() {
            return Err(Respuesta::rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana), v));
        }
        self.columna_a_bordo(heroe)?;
        Err(Respuesta::rechazo(rechazos::CARAVANA_NO_RECLAMABLE, "solo se reclama la Caravana de Fundación que vuelve sola a su origen", v))
    }

    /// `soltarCaravana` (Movimiento_Ejercitos §9, canon 3.13.7): solo **dentro** de una plaza, la plaza dentro de la cual está la columna (entró con todo y el héroe que la orden nombra está
    /// dentro con ella). Fuera, a la puerta o en mundo abierto, se rechaza (decisión del autor 5G-a-5). La caravana queda aparcada allí como con `aparcarCaravana` (en una plaza de su
    /// Facción; la carga sigue a los carros) y, si era la última y la columna es la que un residente conserva en su residencia para llevarla, la columna se deshace como al entrar sin
    /// caravanas.
    fn soltar_caravana(&mut self, t: Instante, p: &Peticion, ejercito: u64, caravana: u64, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let e = self.estado.ejercitos.get(&ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(ejercito)))?;
        if !self.manda_columna(p, e, false) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
        }
        if !self.manda_columna(p, e, true) {
            return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder engancha y aparca las caravanas".into()));
        }
        self.autorizar_heroe(p, heroe, false)?;
        if !e.columna.participantes.iter().any(|x| x.heroe == heroe) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "el héroe no va en esa columna".into()));
        }
        if self.estado.fundacion.caravanas.contains_key(&caravana) {
            return self.soltar_fundacion(t, ejercito, caravana, heroe);
        }
        if self.estado.comercio.posicion_caravana(caravana).is_none() {
            return Err(rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana)));
        }
        if !e.caravanas_adjuntas.contains(&caravana) {
            return Err(rechazo(rechazos::CARAVANA_NO_ESTA_ADJUNTA, "la caravana no va con esta columna".into()));
        }
        let hn = self.heroe(heroe).expect("existe");
        let plaza = match hn.heroe.presencia.ubicacion {
            Ubicacion::Asentamiento(a) if hn.columna == Some(ejercito) => u64::from(a),
            _ => return Err(rechazo(rechazos::CARAVANA_SOLTAR_SOLO_DENTRO, "una caravana solo se suelta dentro de una plaza: hay que entrar antes".into())),
        };
        self.aparcar_caravana(t, p, caravana, plaza, "ejercito.caravana_soltada", &[("heroe", heroe as i64)])?;
        Ok(vec![])
    }
}
