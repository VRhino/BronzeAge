//! Los Aedas itinerantes en el núcleo (sistema 6, tanda 6D; Tecnologia_Aedas §6). Las reglas puras son las de `tecnologia::itinerantes` (6A); aquí están su vida en el mundo, lo que revelan
//! y la orden `ComprarTecnologiaAeda`.
//!
//! # Sin tick: un evento por Aeda
//! Cada Aeda tiene **un solo evento pendiente** (`Carga::Aeda`, prioridad 10, la entidad `entidad_aeda`): si va de camino, su llegada (el fin de su plan); si está parado, el fin de su
//! estancia. Al llegar se detiene `estanciaMinutos` en la plaza; al acabar la estancia parte hacia la plaza más cercana que lo acepta y no ha visitado hace poco, con un plan por la red de
//! caminos (el A\* de las caravanas, con la atracción de la red, y el coste de los tramos de camino rebajado) a `AEDAS.itinerantes.velocidad`; sin camino a ninguna, se queda otra estancia.
//! Su evento solo se invalida (sube su generación) cuando deja de valer: le cierran la puerta de la plaza en la que está, o su plaza o su destino desaparecen.
//!
//! # Cuántos, y dónde nacen
//! `max(3, ⌈vivas / 3⌉)` con alguna Facción viva; nacen los que faltan cuando cambian las vivas o una plaza pasa a aceptarlos (una puerta que se abre, una plaza conquistada, que vuelve a la puerta
//! por defecto), ya parados en las plazas que los aceptan, ordenadas por id. Nunca se eliminan.
//!
//! # La puerta (Q5 del autor)
//! Una plaza los acepta salvo que su puerta esté cerrada al grupo `aedas` (el cierre por defecto no los incluye). Cerrársela con un Aeda dentro lo hace salir al instante hacia su siguiente destino;
//! lo que ya reveló queda revelado. El que llega a una plaza que ya no lo acepta no entra: sigue hacia la siguiente.
//!
//! # Revelar (§6.3)
//! El Aeda parado revela a la Facción dueña de la plaza lo que conoce y ella no tiene aparecido ni revelado (`aedas.revela`, privado de la plaza, con la descubridora y los índices del hito que le
//! faltan). Lo que puede cambiar eso: que llegue (o nazca), que la plaza cambie de dueño y que los Aedas conozcan algo nuevo, que es justo cuando lo cantan (la cita de conocimiento, un logro que
//! se fija, una Era que empieza). Quien lo cambia marca la plaza (`revelar_pendiente`) y se revela al cerrar la versión, con lo que acaba de aparecer ya aparecido.
//!
//! # En la visión
//! Se muestrean como móviles sin Facción (`BANDERA_AEDAS`, `espacial`); no entran en la rejilla de encuentros: no se les ataca ni intercepta (decisión técnica 14).

use super::tecnologias::VistaDeFaccion;
use super::*;
use crate::estado::{entidad_aeda, Dato, PRIO_AEDAS};
use crate::estado_comercio::id_de_entidad;
use tecnologia::adopcion::ErrorVenta;
use tecnologia::itinerantes::{self, AedaItinerante};

fn rechazo_de_venta(e: ErrorVenta) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorVenta::EraNoVendible => (AEDAS_ERA_NO_VENDIBLE, "los Aedas no venden tecnologías de esa Era"),
        ErrorVenta::NoLaConoce => (AEDAS_NO_LA_CONOCE, "el Aeda aún no la conoce"),
        ErrorVenta::YaAparecida => (AEDAS_YA_APARECIDA, "a la Facción ya le ha aparecido"),
        ErrorVenta::SinOro => (AEDAS_SIN_ORO, "el almacén de la plaza no tiene el oro del precio"),
    }
}

impl Nucleo {
    /// Los Aedas itinerantes, por su número (`aeda-N` es el de la posición N − 1).
    pub fn aedas(&self) -> &[AedaItinerante] {
        &self.estado.aedas
    }

    /// Las plazas que aceptan Aedas ahora, `(id, posición)`, ordenadas por id.
    fn plazas_que_aceptan_aedas(&self) -> Vec<(u32, Punto)> {
        let r = &self.estado.reglas.economia.politica;
        self.estado.asentamientos.values().filter(|a| a.puerta.admite_aedas(r)).map(|a| (a.id as u32, a.posicion)).collect()
    }

    /// Nacen los que faltan hasta el objetivo, ya parados (con su estancia) en las plazas que los aceptan. Sin nada que hacer, no mira ninguna plaza.
    pub(super) fn nacer_aedas(&mut self) {
        let r = self.estado.reglas.economia.tecnologia.clone();
        let (vivas, existentes) = (self.espacio.plazas_por_faccion.len() as u32, self.estado.aedas.len() as u32);
        if itinerantes::objetivo(&r, vivas) <= existentes {
            return;
        }
        let aceptan = self.plazas_que_aceptan_aedas();
        for a in itinerantes::nacer(&r, existentes, vivas, &aceptan, self.estado.ahora) {
            self.revelar_pendiente.insert(u64::from(a.en.expect("nace parado")));
            self.estado.aedas.push(a);
            self.programar_aeda(self.estado.aedas.len() - 1);
        }
    }

    /// Programa el único evento del Aeda `i` (su llegada o el fin de su estancia) con su generación de ahora.
    fn programar_aeda(&mut self, i: usize) {
        let a = &self.estado.aedas[i];
        let en = a.siguiente_evento().expect("un Aeda siempre tiene su siguiente evento").max(self.estado.ahora);
        let clave = self.estado.generaciones.clave(entidad_aeda(a.id));
        self.estado.cola.programar(en, PRIO_AEDAS, clave, Carga::Aeda);
    }

    /// Vence el evento de un Aeda: si iba de camino, llega (si la plaza aún lo acepta; si no, sigue hacia otra); si estaba parado, acaba su estancia y parte.
    pub(super) fn aeda_vence(&mut self, entidad: u64, t: Instante) {
        let i = (id_de_entidad(entidad) - 1) as usize;
        let Some(destino) = self.estado.aedas.get(i).map(|a| a.destino) else { return };
        let Some(d) = destino else { return self.partir_aeda(i, t) };
        let r = &self.estado.reglas.economia.politica;
        // Regla (decisión del autor, 2026-10-25, duda 1 de 6D): el que llega a una plaza que le cerró la puerta mientras iba de camino no entra y sigue hacia la siguiente (el TypeScript solo
        // miraba la puerta al elegir destino); es la regla de Q5, «sale quien ya no puede entrar», aplicada a la entrada.
        match self.estado.asentamientos.get(&u64::from(d)).filter(|a| a.puerta.admite_aedas(r)).map(|a| a.posicion) {
            Some(posicion) => {
                let r = self.estado.reglas.economia.tecnologia.clone();
                self.estado.aedas[i].llegar(&r, d, posicion, t);
                self.revelar_pendiente.insert(u64::from(d));
                self.programar_aeda(i);
            }
            None => {
                self.estado.aedas[i].perder_destino(t);
                self.partir_aeda(i, t);
            }
        }
    }

    /// `partir`: hacia la primera candidata (fresca y cercana) a la que haya camino; sin ninguna, otra estancia donde está.
    fn partir_aeda(&mut self, i: usize, t: Instante) {
        let aceptan = self.plazas_que_aceptan_aedas();
        let (candidatas, desde, id) = {
            let a = &self.estado.aedas[i];
            (a.candidatos(&aceptan), a.posicion, a.id)
        };
        let r = self.estado.reglas.economia.tecnologia.clone();
        for c in candidatas {
            let hasta = aceptan[aceptan.binary_search_by_key(&c, |x| x.0).expect("candidata")].1;
            let Some(mut camino) = self.camino_por_la_red(desde, hasta, t) else { continue };
            if camino.puntos.len() < 2 {
                camino.puntos.push(hasta);
                camino.costes.push(0);
            }
            let costes = self.costes_efectivos(&camino.puntos, &camino.costes);
            let g = self.estado.generaciones.actual(entidad_aeda(id));
            let plan = rutas::a_plan_por_minuto(&camino.puntos, &costes, self.rutas.espaciado(), t, r.aedas.velocidad, g);
            self.estado.aedas[i].salir(c, plan);
            return self.programar_aeda(i);
        }
        self.estado.aedas[i].esperar(&r, t);
        self.programar_aeda(i);
    }

    /// El Aeda `i` deja la plaza en la que estaba parado sin acabar su estancia (le cerraron la puerta o la plaza desapareció): su evento deja de valer y parte desde allí.
    fn echar_aeda(&mut self, i: usize) {
        let t = self.estado.ahora;
        let a = &mut self.estado.aedas[i];
        (a.en, a.hasta) = (None, None);
        self.estado.generaciones.subir(entidad_aeda(a.id));
        self.partir_aeda(i, t);
    }

    /// La puerta de `plaza` cambió (Q5): cerrada a los Aedas, los que están dentro salen al instante; abierta, pueden nacer los que esperaban una plaza que los aceptase.
    pub(super) fn puerta_de_los_aedas(&mut self, plaza: u64) {
        let r = &self.estado.reglas.economia.politica;
        if self.estado.asentamientos.get(&plaza).is_none_or(|a| a.puerta.admite_aedas(r)) {
            return self.nacer_aedas();
        }
        for i in 0..self.estado.aedas.len() {
            if self.estado.aedas[i].en == Some(plaza as u32) {
                self.echar_aeda(i);
            }
        }
    }

    /// La plaza desapareció (ya no está en el estado): el Aeda que estaba en ella parte desde allí, y el que iba hacia ella elige otro destino desde donde está.
    pub(super) fn plaza_perdida_para_los_aedas(&mut self, plaza: u64) {
        let (p, t) = (plaza as u32, self.estado.ahora);
        for i in 0..self.estado.aedas.len() {
            let a = &self.estado.aedas[i];
            if a.en == Some(p) {
                self.echar_aeda(i);
            } else if a.destino == Some(p) {
                let id = a.id;
                self.estado.aedas[i].perder_destino(t);
                self.estado.generaciones.subir(entidad_aeda(id));
                self.partir_aeda(i, t);
            }
        }
    }

    // ───────────────────────────── Revelar ─────────────────────────────

    /// Si hay un Aeda parado en `plaza`, se mira qué revela al cerrar la versión.
    pub(super) fn revelar_en(&mut self, plaza: u64) {
        if itinerantes::aeda_en(&self.estado.aedas, plaza as u32).is_some() {
            self.revelar_pendiente.insert(plaza);
        }
    }

    /// Los Aedas conocen algo nuevo: se mira en todas las plazas con un Aeda parado.
    pub(super) fn revelar_donde_hay_aedas(&mut self) {
        self.revelar_pendiente.extend(self.estado.aedas.iter().filter_map(|a| a.en).map(u64::from));
    }

    /// Al cerrar la versión: en cada plaza marcada que siga teniendo un Aeda parado, la Facción dueña recibe lo que el Aeda conoce y ella no tiene aparecido ni revelado (`aedas.revela { faccion,
    /// descubridora; tecnologia, faltan }`, privado de la plaza; `faltan`, los índices de las condiciones del hito que aún no cumple).
    pub(super) fn revelaciones(&mut self, t: Instante) {
        if self.revelar_pendiente.is_empty() {
            return;
        }
        let r = self.estado.reglas.economia.tecnologia.clone();
        for plaza in std::mem::take(&mut self.revelar_pendiente) {
            if itinerantes::aeda_en(&self.estado.aedas, plaza as u32).is_none() {
                continue;
            }
            let Some((f, punto)) = self.estado.asentamientos.get(&plaza).map(|a| (a.faccion, a.posicion)) else { continue };
            let nuevas = self.estado.tecnologia.por_revelar(&r, f, &VistaDeFaccion::de(self, f), t);
            if nuevas.is_empty() {
                continue;
            }
            let propias = self.estado.tecnologia.de_mut(&r, f);
            nuevas.iter().for_each(|x| propias.reveladas.poner(x.tec));
            for x in nuevas {
                let datos = vec![("faccion", i64::from(f)), ("descubridora", i64::from(x.descubridora))];
                let faltan = Dato::Lista(x.faltan.iter().map(|&i| Dato::Numero(i64::from(i))).collect());
                let valores = vec![("tecnologia", Dato::Texto(r.def(x.tec).id.clone())), ("faltan", faltan)];
                self.tr.emitir(&self.estado, EventoNucleo { asentamientos: vec![plaza], punto: Some(punto), valores, ..EventoNucleo::nuevo("aedas.revela", t, datos) });
            }
            self.tr.tocar(Clase::Faccion, u64::from(f));
        }
    }

    // ───────────────────────────── Vender ─────────────────────────────

    /// `ComprarTecnologiaAeda` (TA §6.4): el Rey o el Gobernador presentes en la plaza compran al Aeda parado en ella; paga el almacén de la plaza el doble del oro de la tarifa de su Era
    /// (unidades enteras) y la tecnología aparece a la Facción dueña sin su hito (`aedas.venta { faccion, aeda, precio; tecnologia }`, privado de la plaza). Los rechazos no tocan nada.
    pub(super) fn ejecutar_compra_a_aeda(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let Comando::ComprarTecnologiaAeda { heroe, asentamiento, tecnologia: x } = p.comando else { unreachable!("solo comprar a un Aeda") };
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let Some((f, punto)) = self.estado.asentamientos.get(&asentamiento).map(|a| (a.faccion, a.posicion)) else {
            return Err(rechazo(rechazos::ASENTAMIENTO_DESCONOCIDO, "esa plaza no existe"));
        };
        self.autorizar_en_plaza(p, asentamiento, Some(heroe))?;
        let Some(aeda) = itinerantes::aeda_en(&self.estado.aedas, asentamiento as u32).map(|a| a.id) else {
            return Err(rechazo(rechazos::AEDAS_NO_HAY_AEDA, "no hay ningún Aeda en esta plaza"));
        };
        let r = self.estado.reglas.economia.tecnologia.clone();
        if usize::from(x) >= r.catalogo.len() {
            return Err(rechazo(rechazos::TECNOLOGIA_DESCONOCIDA, "esa tecnología no existe"));
        }
        let Estado { tecnologia, asentamientos, .. } = &mut self.estado;
        let almacen = &mut asentamientos.get_mut(&asentamiento).expect("existe").economia.almacen;
        let precio = tecnologia.vender(&r, f, x, almacen, t).map_err(|e| {
            let (c, m) = rechazo_de_venta(e);
            rechazo(c, m)
        })?;
        let datos = vec![("faccion", i64::from(f)), ("aeda", i64::from(aeda)), ("precio", precio)];
        let valores = vec![("tecnologia", Dato::Texto(r.def(x).id.clone()))];
        self.tr.emitir(&self.estado, EventoNucleo { asentamientos: vec![asentamiento], punto: Some(punto), valores, ..EventoNucleo::nuevo("aedas.venta", t, datos) });
        self.tr.tocar(Clase::Asentamiento, asentamiento);
        self.tr.tocar(Clase::Faccion, u64::from(f));
        // Aparecer no cambia ningún hito (la condición `tecnologia` mira las adoptadas) ni los campamentos (miran las adoptadas y la descubridora, que ya existía). La épica de esta
        // tecnología que cantara un Aeda residente de la Facción se pierde (6E, `motivo: aparecida`).
        self.epicas_sobrantes(t, f, x);
        Ok(vec![])
    }
}
