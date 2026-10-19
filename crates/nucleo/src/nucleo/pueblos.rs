//! Los pueblos extractores y su carro de acarreo dentro del núcleo (Economia_Construccion §14, Doc 4.2.2; Comercio_Caravanas segunda §12). Las
//! reglas del pueblo viven en `asentamiento::pueblos` (el pueblo es un elemento de su plaza madre, no un asentamiento) y las del carro en
//! `comercio::acarreo`; aquí está lo que las une con el mapa, la flota y el viaje.
//!
//! # El ciclo
//! 1. **Obra de mapa**: `MarcarPueblo` marca un nodo de la zona de la madre y deja la obra pendiente de materiales (ocupa cupo y reclama el
//!    nodo); `EntregarMateriales` lleva a la obra, con una columna a la puerta, lo que cargan su carro y sus caravanas adjuntas. Con todo
//!    entregado arranca la obra y el paso de minuto de la plaza la termina (precisión de un minuto): al completarse nace el carro, parado en el
//!    pueblo.
//! 2. **Extracción**: dentro del paso de minuto de la plaza (ver `asentamiento::pueblos`); no hay evento por pueblo. Cuando el almacén local se
//!    llena (`pueblo.almacen_lleno`) el carro sale si está en casa.
//! 3. **Acarreo**: el carro sale lleno por su ruta fija pueblo → madre (el mismo trazado, plan, peajes y llegada de las caravanas
//!    comerciales; la red de caminos acelera sus tramos pero no registra la ruta), vuelca en el almacén de la madre y vuelve vacío; si al volver
//!    el almacén está lleno otra vez, sale de nuevo.
//! 4. **Sin carro**: si el carro se pierde (batalla, destrucción del pueblo), el pueblo acumula hasta llenar su almacén y la extracción para
//!    hasta que `ReponerCarro` compre otro en la madre.
//!
//! # Lo provisional
//! La autorización es la de las demás órdenes de plaza (`manda_en`) y la entrega de materiales exige ser el propietario de la columna; la
//! guarnición de mineros y el saqueo son del sistema militar.

use super::*;
use crate::nucleo::construccion::SIN_TECNOLOGIA;
use crate::nucleo::mercado::{id32, RADIO_PUERTA};
use asentamiento::catalogo::Tipo;
use asentamiento::pueblos::{ErrorPueblo, Pueblo};
use asentamiento::Costo;
use comercio::acarreo;
use comercio::viaje::{longitud_ruta, peajes_de_paradas};
use comercio::{total, Carga as Mercancia};

fn rechazo_de_pueblo(e: ErrorPueblo) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorPueblo::TipoNoPueblo => (PUEBLO_NODO_NO_EXTRAIBLE, "ese nodo no se explota con un pueblo extractor"),
        ErrorPueblo::TecnologiaFaltante => (PUEBLO_TECNOLOGIA_FALTANTE, "la Facción no tiene la tecnología que pide el extractor"),
        ErrorPueblo::NivelInsuficiente => (PUEBLO_NIVEL_INSUFICIENTE, "la plaza madre debe ser de nivel 2 o más"),
        ErrorPueblo::SinCupo => (PUEBLO_SIN_CUPO, "la plaza madre ya tiene todos los pueblos que su nivel permite"),
        ErrorPueblo::NoExiste => (PUEBLO_DESCONOCIDO, "no existe ese pueblo"),
        ErrorPueblo::NoEstaPendiente => (PUEBLO_NO_ESTA_PENDIENTE, "la obra no está pendiente de materiales"),
        ErrorPueblo::NadaQueEntregar => (PUEBLO_NADA_QUE_ENTREGAR, "la columna no lleva nada que la obra necesite"),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Los pueblos extractores de una plaza (en cualquier estado), por id.
    pub fn pueblos_de(&self, plaza: u64) -> &[Pueblo] {
        self.estado.asentamientos.get(&plaza).map_or(&[], |a| &a.economia.pueblos)
    }

    /// Un pueblo por id y la plaza madre a la que pertenece. Recorre las plazas: es para órdenes y consultas, no para el camino caliente.
    pub fn pueblo(&self, id: u64) -> Option<(u64, &Pueblo)> {
        self.estado.asentamientos.values().find_map(|a| a.economia.pueblo(id).map(|p| (a.id, p)))
    }

    /// Dónde está un pueblo (la posición de su nodo en el mapa).
    pub fn posicion_de_pueblo(&self, id: u64) -> Option<Punto> {
        let (_, p) = self.pueblo(id)?;
        self.geografia.as_ref()?.mapa.nodos.get(p.nodo as usize).map(|n| n.pos)
    }

    /// Los nodos que el panel de la plaza madre ofrece para un pueblo (§14): los de piedra y minerales dentro de su zona real, sin pueblo ni
    /// extractor encima, con el extractor que les toca y cuya tecnología tenga la Facción. Vacío sin mundo o sin plaza.
    pub fn candidatos_de_pueblo(&self, plaza: u64) -> Vec<(u32, Tipo)> {
        let (Some(geo), Some(a)) = (self.geografia.as_ref(), self.estado.asentamientos.get(&plaza)) else { return vec![] };
        let catalogo = &self.estado.reglas.economia.datos.catalogo;
        let mut nodos = geo.mapa.piedra_en(a.posicion, a.radio);
        nodos.extend(geo.mapa.minerales_en(a.posicion, a.radio));
        nodos.sort_unstable();
        let libre = |n: &u32| !self.estado.nodos_reclamados.contains_key(n) && self.en_zona(plaza, geo.mapa.nodos[*n as usize].pos);
        let alcanzable = |t: Tipo| catalogo.tipo(t).tecnologia.as_deref().is_none_or(SIN_TECNOLOGIA);
        nodos.into_iter().filter(libre).map(|n| (n, geo.mapa.nodos[n as usize].tipo)).filter(|&(_, t)| alcanzable(t)).collect()
    }

    /// La planta del pueblo (presentación pura, T / Campamentos_Mercenarios §7): una función del id del pueblo, que no se guarda.
    pub fn planta_de_pueblo(&self, id: u64) -> Option<crate::urbanismo::VistaCiudad> {
        let (_, p) = self.pueblo(id)?;
        Some(crate::urbanismo::planta_de_pueblo(&self.estado.reglas.economia, self.semilla, id, p.tipo))
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Las órdenes de los pueblos. `Ok(Some(n))` = cambió algo y creó el pueblo `n`; `Ok(None)` = cambió algo; rechazos y «sin cambios» salen
    /// como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_pueblos(&mut self, t: Instante, p: &Peticion) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let de_pueblo = |e: ErrorPueblo| {
            let (c, m) = rechazo_de_pueblo(e);
            rechazo(c, m.into())
        };
        let datos = self.estado.reglas.economia.datos.clone();
        match &p.comando {
            Comando::MarcarPueblo { asentamiento, nodo } => {
                let plaza = self.plaza_o_rechazo(*asentamiento)?;
                if !self.manda_en(p, plaza.faccion) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, format!("no mandas en as-{asentamiento}")));
                }
                let geo = self.geografia.as_ref().ok_or_else(|| rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado".into()))?;
                let n = geo.mapa.nodos.get(*nodo as usize).ok_or_else(|| rechazo(rechazos::PUEBLO_NODO_DESCONOCIDO, format!("nodo-{nodo}")))?;
                let (tipo, pos) = (n.tipo, n.pos);
                if !tipo.es_de_pueblo() {
                    return Err(de_pueblo(ErrorPueblo::TipoNoPueblo));
                }
                if self.estado.nodos_reclamados.contains_key(nodo) {
                    return Err(rechazo(rechazos::PUEBLO_NODO_OCUPADO, format!("el nodo {nodo} ya está explotado")));
                }
                if !self.en_zona(*asentamiento, pos) {
                    return Err(rechazo(rechazos::PUEBLO_FUERA_DE_LA_ZONA, format!("el nodo {nodo} cae fuera de la zona de as-{asentamiento}")));
                }
                let carro = acarreo::coste_reposicion(&self.estado.reglas.economia.comercio.flota);
                let id = self.estado.siguiente_id;
                let mut ev = vec![];
                let eco = &mut self.estado.asentamientos.get_mut(asentamiento).expect("existe").economia;
                eco.iniciar_pueblo(&datos, &SIN_TECNOLOGIA, id, tipo, *nodo, &carro, &mut ev).map_err(de_pueblo)?;
                self.estado.siguiente_id += 1;
                self.estado.nodos_reclamados.insert(*nodo, *asentamiento);
                self.anotar_sucesos(*asentamiento, t, &ev);
                Ok(Some(id))
            }
            Comando::EntregarMateriales { ejercito, pueblo } => {
                let e = self.estado.ejercitos.get(ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}")))?;
                if !(p.admin || e.propietario == p.actor) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
                }
                let (plaza, pu) = self.pueblo(*pueblo).ok_or_else(|| de_pueblo(ErrorPueblo::NoExiste))?;
                if e.faccion != self.estado.asentamientos[&plaza].faccion {
                    return Err(rechazo(rechazos::PUEBLO_FACCION_AJENA, "la obra es de otra Facción".into()));
                }
                let sitio = self.posicion_de_pueblo(pu.id).expect("un pueblo está sobre un nodo del mapa");
                if e.posicion_en(t).dist(sitio) > RADIO_PUERTA {
                    return Err(rechazo(rechazos::PUEBLO_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la obra".into()));
                }
                let disponible: Costo = self.material_de_columna(*ejercito).into_iter().collect();
                let mut ev = vec![];
                let eco = &mut self.estado.asentamientos.get_mut(&plaza).expect("existe").economia;
                let tomado = eco.entregar_a_pueblo(t, &datos, *pueblo, &disponible, &mut ev).map_err(de_pueblo)?;
                self.quitar_de_columna(*ejercito, &tomado);
                self.tr.tocados.push(*ejercito);
                self.anotar_sucesos(plaza, t, &ev);
                Ok(None)
            }
            Comando::DestruirPueblo { pueblo } => {
                let (plaza, _) = self.pueblo(*pueblo).ok_or_else(|| de_pueblo(ErrorPueblo::NoExiste))?;
                if !self.manda_en(p, self.estado.asentamientos[&plaza].faccion) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, format!("no mandas en as-{plaza}")));
                }
                let mut ev = vec![];
                let destruido = self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.quitar_pueblo(*pueblo, &mut ev).expect("existe");
                self.estado.nodos_reclamados.remove(&destruido.nodo);
                if let Some(carro) = destruido.carro {
                    self.perder_caravana(t, carro);
                }
                self.anotar_sucesos(plaza, t, &ev);
                Ok(None)
            }
            Comando::ReponerCarro { pueblo } => {
                let (plaza, pu) = self.pueblo(*pueblo).ok_or_else(|| de_pueblo(ErrorPueblo::NoExiste))?;
                if !self.manda_en(p, self.estado.asentamientos[&plaza].faccion) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, format!("no mandas en as-{plaza}")));
                }
                if !pu.activo() {
                    return Err(rechazo(rechazos::PUEBLO_NO_ESTA_ACTIVO, "el pueblo aún no está levantado".into()));
                }
                if pu.carro.is_some() {
                    return Err(rechazo(rechazos::PUEBLO_YA_TIENE_CARRO, "el pueblo ya tiene carro: solo se repone si no tiene ninguno".into()));
                }
                let (id, posicion) = (self.estado.comercio.siguiente_caravana, self.posicion_de_pueblo(*pueblo).expect("sobre un nodo"));
                let flota = &self.estado.reglas.economia.comercio.flota;
                let almacen = &mut self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.almacen;
                let carro = acarreo::reponer(flota, almacen, id, id32(plaza), posicion).map_err(|e| {
                    let (c, m) = flota::rechazo_de_caravana(e);
                    rechazo(c, m.into())
                })?;
                self.estado.comercio.siguiente_caravana += 1;
                self.estado.comercio.flota.push(carro);
                self.tr.asentamientos.push(plaza);
                self.enlazar_carro(plaza, *pueblo, id);
                self.emitir_propio(t, "acarreo.carro_repuesto", vec![plaza], vec![("pueblo", *pueblo as i64), ("caravana", id as i64)]);
                self.salir_acarreo(plaza, *pueblo, t);
                Ok(None)
            }
            _ => unreachable!("solo se llama con órdenes de pueblos"),
        }
    }

    /// Lo que lleva una columna y vale como material: su carro (`suministro`) y la carga de sus caravanas adjuntas, sumado por recurso.
    fn material_de_columna(&self, ejercito: u64) -> Mercancia {
        let e = &self.estado.ejercitos[&ejercito];
        let mut todo = e.suministro.clone();
        for c in e.caravanas_adjuntas.iter().filter_map(|&c| self.estado.comercio.caravana(c)) {
            for (&r, &n) in &c.contenido {
                *todo.entry(r).or_insert(0) += n;
            }
        }
        todo
    }

    /// Quita de la columna lo que entregó: primero de su carro y, si no basta, de sus caravanas adjuntas por orden de id.
    fn quitar_de_columna(&mut self, ejercito: u64, tomado: &Costo) {
        let Estado { ejercitos, comercio, .. } = &mut self.estado;
        let e = ejercitos.get_mut(&ejercito).expect("existe");
        for &(r, n) in tomado {
            let mut falta = n;
            let quitar = |contenido: &mut Mercancia, falta: &mut Mil| {
                let de_aqui = (*falta).min(contenido.get(&r).copied().unwrap_or(0));
                if de_aqui > 0 {
                    *falta -= de_aqui;
                    let queda = contenido[&r] - de_aqui;
                    if queda > 0 {
                        contenido.insert(r, queda);
                    } else {
                        contenido.remove(&r);
                    }
                }
            };
            quitar(&mut e.suministro, &mut falta);
            for &c in &e.caravanas_adjuntas {
                if falta > 0 {
                    if let Some(i) = comercio.posicion_caravana(c) {
                        quitar(&mut comercio.flota[i].contenido, &mut falta);
                    }
                }
            }
        }
    }

    // ───────────────────────────── El carro ─────────────────────────────

    /// Anota el carro `carro` en el pueblo (el pueblo lo conoce por su id de la flota).
    fn enlazar_carro(&mut self, plaza: u64, pueblo: u64, carro: u64) {
        if let Some(pu) = self.estado.asentamientos.get_mut(&plaza).and_then(|a| a.economia.pueblos.iter_mut().find(|p| p.id == pueblo)) {
            pu.carro = Some(carro);
        }
    }

    /// El pueblo acaba de levantarse: nace su carro (el coste ya iba en la obra), parado en el pueblo.
    pub(super) fn dar_carro_a_pueblo(&mut self, plaza: u64, pueblo: u64) {
        let Some(posicion) = self.posicion_de_pueblo(pueblo) else { return };
        let id = self.estado.comercio.siguiente_caravana;
        self.estado.comercio.siguiente_caravana += 1;
        self.estado.comercio.flota.push(acarreo::nuevo_carro(id, id32(plaza), posicion));
        self.enlazar_carro(plaza, pueblo, id);
    }

    /// El carro `carro` de la plaza `plaza` desapareció (destruido, perdido en una batalla...): su pueblo se queda sin él.
    pub(super) fn pueblo_sin_carro(&mut self, plaza: u64, carro: u64) {
        let Some(a) = self.estado.asentamientos.get_mut(&plaza) else { return };
        a.economia.pueblos.iter_mut().filter(|p| p.carro == Some(carro)).for_each(|p| p.carro = None);
        self.tr.asentamientos.push(plaza);
    }

    /// El carro del pueblo sale hacia la madre **si está en casa y el almacén local está lleno** (nada recurre por pueblo ocioso: se llama cuando
    /// el almacén se llena, cuando el carro vuelve y cuando se repone). Lleva la capacidad entera, por la ruta fija pueblo → madre. Sin ruta
    /// por tierra no sale.
    // DUDA: un pueblo sin ruta por tierra a su madre queda parado (el almacén lleno avisa una sola vez y nada reintenta); el relieve de la
    // zona de una plaza casi siempre la permite.
    pub(super) fn salir_acarreo(&mut self, plaza: u64, pueblo: u64, t: Instante) {
        let Some(a) = self.estado.asentamientos.get(&plaza) else { return };
        let Some(pu) = a.economia.pueblo(pueblo) else { return };
        let Some(carro) = pu.carro else { return };
        let flota = self.estado.reglas.economia.comercio.flota.clone();
        let recurso = pu.recurso(&self.estado.reglas.economia.datos);
        let Some(c) = self.estado.comercio.caravana(carro) else { return };
        if !acarreo::debe_salir(c, &flota, pu.almacen.cantidad(recurso)) {
            return;
        }
        let (pos_madre, faccion) = (a.posicion, a.faccion);
        let Some(pos_pueblo) = self.posicion_de_pueblo(pueblo) else { return };
        let Some(tz) = self.trazar_ruta(pueblo, plaza, pos_pueblo, pos_madre, faccion, t) else { return };
        let cantidad = acarreo::capacidad(&flota);
        self.estado.asentamientos.get_mut(&plaza).and_then(|a| a.economia.pueblos.iter_mut().find(|p| p.id == pueblo)).expect("existe").almacen.quitar(recurso, cantidad);
        let i = self.estado.comercio.posicion_caravana(carro).expect("existe");
        let plan = self.plan_de(&self.estado.comercio.flota[i], &tz.puntos, &tz.costes, t);
        let peajes = peajes_de_paradas(&tz.paradas, longitud_ruta(&tz.puntos));
        let carga: Mercancia = [(recurso, cantidad)].into_iter().collect();
        acarreo::salir(&mut self.estado.comercio.flota[i], id32(plaza), carga, tz.puntos, tz.costes, peajes, plan.clone()).expect("el carro está en casa");
        self.instalar_plan(carro, plan);
        self.tr.asentamientos.push(plaza);
        self.emitir_propio(t, "acarreo.sale", vec![plaza], vec![("pueblo", pueblo as i64), ("caravana", carro as i64), ("carga", cantidad)]);
    }

    /// El carro llegó a la madre: vuelca lo que trae en su almacén (topado: lo que no cabe se pierde) y vuelve vacío (su plan ya es el de la vuelta).
    pub(super) fn acarreo_llega(&mut self, t: Instante, carro: u64, madre: u64, contenido: &Mercancia) {
        let Some(plaza) = self.estado.asentamientos.get_mut(&madre) else { return self.perder_caravana(t, carro) };
        let ingresado: Mil = contenido.iter().map(|(&r, &n)| plaza.economia.almacen.agregar(r, n)).sum();
        self.tr.asentamientos.push(madre);
        self.emitir_propio(t, "acarreo.llega", vec![madre], vec![("caravana", carro as i64), ("carga", ingresado), ("perdido", total(contenido) - ingresado)]);
    }

    /// El carro volvió al pueblo: queda listo y, si el almacén ya está lleno otra vez, sale de nuevo.
    pub(super) fn acarreo_en_casa(&mut self, t: Instante, madre: u64, carro: u64) {
        let Some(pueblo) = self.estado.asentamientos.get(&madre).and_then(|a| a.economia.pueblos.iter().find(|p| p.carro == Some(carro))).map(|p| p.id) else { return };
        self.emitir_propio(t, "acarreo.vuelve", vec![madre], vec![("pueblo", pueblo as i64), ("caravana", carro as i64)]);
        self.salir_acarreo(madre, pueblo, t);
    }
}
