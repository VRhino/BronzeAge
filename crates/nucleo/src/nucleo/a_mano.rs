//! El almacén personal y las caravanas a mano (5G-a, 2026-10-24; Movimiento_Ejercitos §3.3 y §9, Heroe_Y_Tropa §4.4): `guardarEnAlmacenPersonal`, `sacarDelAlmacenPersonal`,
//! `cargarCaravana`, `reclamarCaravana` y `soltarCaravana`. Todo en unidades enteras (C3b): lo que se mueve se redondea hacia abajo, y nada se crea ni se pierde (lo que sale de un sitio
//! entra en el otro).
//!
//! - **El almacén personal vive donde se reside**: se llena y se vacía con el carro de la columna, con la columna a la puerta de la propia residencia, y solo el Líder (el carro es común).
//! - **`cargarCaravana`** carga en una caravana adjunta del almacén de una plaza de su Facción a cuya puerta está la columna, sin reserva (como el reparto automático).
//! - **`soltarCaravana`** es aparcar en la plaza de su Facción donde está la columna (a su puerta o dentro): no hay caravanas sueltas por el mapa (2026-10-19).
//! - **`reclamarCaravana`** vale solo para la Caravana de Fundación que vuelve sola; mientras la Caravana de Fundación sea una marca de la columna (hasta 5G-b), no hay ninguna que
//!   reclamar y la orden se rechaza con `caravana.no_reclamable`.

use super::*;
use crate::estado::Dato;
use crate::codec_economia::recurso_a_u8;
use asentamiento::Recurso;
use base::{entero, UNIDAD};
use comercio::{de, sumar, total, Carga as Mercancia};
use militar::convocatoria::Lugar;
use militar::heroe::Residencia;

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
            Comando::ReclamarCaravana { heroe, caravana, politica: _ } => self.reclamar_caravana(p, *heroe, *caravana),
            Comando::SoltarCaravana { ejercito, caravana, heroe } => self.soltar_caravana(t, p, *ejercito, *caravana, *heroe),
            _ => unreachable!("solo se llama con las órdenes del almacén personal y de las caravanas a mano"),
        }
    }

    /// `guardarEnAlmacenPersonal` (`guarda`) y `sacarDelAlmacenPersonal`: del carro de la columna al almacén personal hasta su capacidad, o del almacén al carro hasta lo que cabe en él
    /// (`capacidad_de_carga`, con las caravanas adjuntas). El Líder, con la columna a la puerta de su residencia (la administración salta la puerta).
    fn mover_almacen_personal(&mut self, t: Instante, p: &Peticion, heroe: u64, recurso: Recurso, cantidad: Mil, guarda: bool) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let (columna, e) = self.columna_a_bordo(heroe)?;
        if e.columna.lider != heroe {
            return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder de la columna mueve lo de su carro".into()));
        }
        let aqui = e.posicion_en(t);
        let hn = self.heroe(heroe).expect("existe");
        let lugar = match hn.heroe.residencia {
            Some(Residencia::Asentamiento(a)) => Lugar::Asentamiento(a),
            Some(Residencia::Campamento(c)) => Lugar::Campamento(c),
            None => return Err(rechazo(rechazos::HEROE_SIN_RESIDENCIA, "sin residencia no hay almacén personal".into())),
        };
        if !p.admin && self.puerta_de(lugar).is_none_or(|puerta| aqui.dist(puerta) > self.radio_puerta()) {
            return Err(rechazo(rechazos::ALMACEN_PERSONAL_FUERA_DE_TU_RESIDENCIA, "el almacén personal está en tu residencia: solo se toca con tu columna a su puerta".into()));
        }
        let pedido = entero(cantidad);
        if pedido <= 0 {
            return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "hay que mover al menos una unidad".into()));
        }
        let carga = &self.estado.ejercitos[&columna].columna.carga;
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
            let hueco = self.capacidad_de_carga(columna).unwrap_or(0) - total(carga);
            let movido = entero(pedido.min(guardado).min(hueco));
            if movido <= 0 {
                return Err(if guardado < UNIDAD {
                    rechazo(rechazos::ALMACEN_SIN_EXISTENCIAS, format!("no hay {} en tu almacén personal", recurso.nombre()))
                } else {
                    rechazo(rechazos::CARRO_LLENO, "el carro está lleno".into())
                });
            }
            movido
        };
        let carro = &mut self.estado.ejercitos.get_mut(&columna).expect("existe").columna.carga;
        let personal = &mut self.estado.militar.heroes.get_mut(&heroe).expect("existe").almacen_personal;
        let (de_aqui, a_alli) = if guarda { (carro, personal) } else { (personal, carro) };
        quitar(de_aqui, recurso, movido);
        sumar(a_alli, recurso, movido);
        self.tr.tocar(Clase::Heroe, heroe);
        self.tr.tocar(Clase::Ejercito, columna);
        let datos = vec![("heroe", heroe as i64), ("recurso", recurso_a_u8(recurso) as i64), ("cantidad", movido)];
        let sentido = Dato::Texto(if guarda { "guarda" } else { "saca" }.into());
        self.tr.emitir(&self.estado, EventoNucleo { valores: vec![("sentido", sentido)], ..EventoNucleo::nuevo("heroe.almacen_personal", t, datos) });
        Ok(vec![])
    }

    /// `cargarCaravana` (Movimiento_Ejercitos §9): `min(pedido, espacio de la caravana, existencias de la plaza)`, sin reserva de trigo (es mercancía, como la del reparto automático).
    /// El espacio es su capacidad efectiva (con «Carga Ampliada» de su origen) menos lo que ya lleva.
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
        // DUDA: «donde pueda repostar»; hasta el sistema político solo abre su almacén la plaza propia (`permiteReabastecerAliados` es de 5D), como `repostar`.
        if faccion != Some(a.faccion) {
            return Err(rechazo(rechazos::CARAVANA_PLAZA_AJENA, "esa plaza no abre su almacén a esta columna".into()));
        }
        let existencias = a.economia.almacen.cantidad(recurso);
        let c = &self.estado.comercio.flota[i];
        let espacio = self.capacidad_de_caravana(c) - c.carga_total();
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

    /// `reclamarCaravana` (Movimiento_Ejercitos §9): solo la Caravana de Fundación que vuelve sola a su origen, en mundo abierto. Hoy la Caravana de Fundación es una marca de la columna, no una
    /// caravana, así que ninguna se puede reclamar: se comprueba lo que la orden exige del héroe y de la caravana y se rechaza con `caravana.no_reclamable` (la Caravana de Fundación de verdad
    /// es de 5G-b).
    fn reclamar_caravana(&mut self, p: &Peticion, heroe: u64, caravana: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        self.autorizar_heroe(p, heroe, false)?;
        if self.estado.comercio.posicion_caravana(caravana).is_none() {
            return Err(Respuesta::rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana), v));
        }
        self.columna_a_bordo(heroe)?;
        Err(Respuesta::rechazo(rechazos::CARAVANA_NO_RECLAMABLE, "solo se reclama la Caravana de Fundación que vuelve sola a su origen", v))
    }

    /// `soltarCaravana` (Movimiento_Ejercitos §9): la adjunta queda aparcada en la plaza de la Facción de la caravana a cuya puerta (o dentro de la cual) está la columna; la más cercana y,
    /// si empatan, la de menor id. En mundo abierto se rechaza: no hay caravanas sueltas por el mapa.
    fn soltar_caravana(&mut self, t: Instante, p: &Peticion, ejercito: u64, caravana: u64, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let e = self.estado.ejercitos.get(&ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(ejercito)))?;
        if !self.manda_columna(p, e, false) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
        }
        self.autorizar_heroe(p, heroe, false)?;
        if !e.columna.participantes.iter().any(|x| x.heroe == heroe) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "el héroe no va en esa columna".into()));
        }
        let i = self.estado.comercio.posicion_caravana(caravana).ok_or_else(|| rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana)))?;
        if !e.caravanas_adjuntas.contains(&caravana) {
            return Err(rechazo(rechazos::CARAVANA_NO_ESTA_ADJUNTA, "la caravana no va con esta columna".into()));
        }
        let (aqui, radio) = (e.posicion_en(t), self.radio_puerta());
        let faccion = self.estado.asentamientos.get(&(self.estado.comercio.flota[i].origen as u64)).map(|o| o.faccion);
        let plaza = self.estado.asentamientos.values().filter(|a| Some(a.faccion) == faccion).map(|a| (aqui.dist(a.posicion), a.id)).filter(|&(d, _)| d <= radio).min().map(|(_, id)| id);
        let plaza = plaza.ok_or_else(|| rechazo(rechazos::CARAVANA_FUERA_DE_LA_PUERTA, "una caravana solo se suelta dentro de una plaza de su Facción, o a su puerta".into()))?;
        self.aparcar_caravana(t, p, caravana, plaza, "ejercito.caravana_soltada", &[("heroe", heroe as i64)])?;
        Ok(vec![])
    }
}
