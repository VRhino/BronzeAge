//! Obras de un asentamiento (Economia_Construccion §6.1, §6.4, §7, §8): cola, cuadrillas, mejoras de nivel interno y las
//! órdenes manuales del Gobernador y el Maestro de Obras. El pago ocurre **al encolar**; una obra ya empezada no se cancela.

use crate::asentamiento::{Asentamiento, Evento, Exterior};
use crate::catalogo::Tipo;
use crate::datos::{Reglas, UNO};
use crate::edificio::{Edificio, Edificios, Estado, Mejora};
use crate::recurso::{Costo, Recurso};
use base::{Instante, Mil, MS_POR_MINUTO};

/// Fracción del coste (y del tiempo) de una reconstrucción tras un saqueo (`OCUPACION.fraccionCosteReconstruccion`).
const FRACCION_RECONSTRUCCION: i64 = 500_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorObra {
    TecnologiaFaltante,
    EraInsuficiente,
    /// El Centro Urbano y los puestos del Mercado nunca se añaden a mano.
    TipoNoManual,
    Unico,
    TopeAlmacenes,
    TopeViviendas,
    NivelInsuficiente,
    /// Los minerales y toda Cantera después de la primera solo se explotan con un pueblo extractor (Doc 4.2.2).
    SoloPorPueblo,
    TopeTransformacion,
    GranFundicion,
    ColaLlena,
    SinSitio,
    RecursosInsuficientes,
    NoExiste,
    NoEstaEnCola,
    NoEsMejorable,
    SinCuadrilla,
}

/// Minutos de una obra con los factores de tiempo (política «Vía Rápida», reconstrucción): `max(1, round(tiempo × factor))`.
fn minutos(tiempo: i64, factor_ppm: i64) -> i64 {
    ((tiempo as i128 * factor_ppm as i128 + UNO as i128 / 2) / UNO as i128).max(1) as i64
}

impl Asentamiento {
    /// `puedeIniciarConstruccion`: hay con qué pagar y, recurso a recurso, no se baja de la reserva. La Granja ignora la reserva de
    /// trigo y la Leñera la de madera: son la única forma de recuperarlos.
    pub fn puede_iniciar(&self, r: &Reglas, tipo: Tipo, costo: &Costo, reserva: &[Mil; 29]) -> bool {
        let propio = r.construccion.recurso_propio.get(&tipo).copied();
        self.almacen.tiene(costo) && costo.iter().all(|&(rec, n)| Some(rec) == propio || reserva[rec.indice()] <= 0 || self.almacen.cantidad(rec) - n >= reserva[rec.indice()])
    }

    /// El Almacén suma 300 a cada recurso y el Granero suma a la capacidad del trigo; se aplica al entrar en servicio
    /// (`signo` = 1) y se quita al dejar de estarlo (`signo` = −1). Una mejora de Granero suma el delta entre niveles.
    pub fn aplicar_capacidad(&mut self, r: &Reglas, tipo: Tipo, nivel: u8, signo: i64) {
        let def = r.catalogo.tipo(tipo);
        match tipo {
            Tipo::Almacen => self.almacen.ampliar_todos(signo * def.capacidad_por_recurso),
            Tipo::Granero => {
                let cap = def.nivel(nivel).map_or(0, |n| n.capacidad_trigo);
                self.almacen.ampliar(Recurso::Trigo, signo * cap);
            }
            _ => {}
        }
    }

    /// Las obras cuyo instante llegó pasan a activas. Un Mercado que se completa levanta sus puestos del nivel 1 (gratis y activos).
    pub(crate) fn completar_obras(&mut self, t: Instante, r: &Reglas, x: &mut Exterior, ev: &mut Vec<Evento>) {
        let hechos: Vec<usize> = (0..self.edificios.len()).filter(|&i| self.edificios[i].estado == Estado::EnConstruccion && self.edificios[i].completa_en.map_or(true, |c| t >= c)).collect();
        for i in hechos {
            let e = &mut self.edificios[i];
            e.estado = Estado::Activo;
            e.completa_en = None;
            e.danado = false;
            let (id, tipo, nivel) = (e.id, e.tipo, e.nivel_interno);
            self.aplicar_capacidad(r, tipo, nivel, 1);
            self.completados_pendientes += 1;
            ev.push(Evento::EdificioCompletado { id, tipo });
            if tipo == Tipo::Mercado {
                self.levantar_puestos(r, x, nivel, ev);
            }
        }
    }

    /// Los puestos gratuitos del Mercado para ese nivel (Economia_Construccion §6.6), activos desde que nacen.
    fn levantar_puestos(&mut self, r: &Reglas, x: &mut Exterior, nivel: u8, ev: &mut Vec<Evento>) {
        let u = self.urbano(r);
        let gratuitos = x.sitios.puestos_de_mercado(nivel, &u, &mut self.siguiente_edificio);
        self.alta_gratuitos(gratuitos, ev);
    }

    /// Arranque de obra con las cuadrillas libres **en el instante `t`**: una obra que acaba libera su cuadrilla en el mismo instante y la
    /// siguiente de la cola la toma entonces. Una reconstrucción tras saqueo no ocupa cuadrilla ni espera turno.
    pub(crate) fn arrancar_obras(&mut self, t: Instante, r: &Reglas, ev: &mut Vec<Evento>) {
        let mut cupo = r.construccion.necesidades.maximo_en_construccion_simultanea.saturating_sub(self.edificios.cuadrillas_ocupadas());
        let mut cola: Vec<usize> = (0..self.edificios.len()).filter(|&i| self.edificios[i].estado == Estado::EnCola).collect();
        cola.sort_by_key(|&i| std::cmp::Reverse(self.edificios[i].prioridad)); // estable: a igual prioridad, el orden de la lista
        let factor_tiempo = self.politicas.factor(&r.politicas, "factorTiempoConstruccion");
        for i in cola {
            let (tipo, danado) = (self.edificios[i].tipo, self.edificios[i].danado);
            if cupo == 0 && !danado {
                continue;
            }
            let def = r.catalogo.tipo(tipo);
            let mut factor = factor_tiempo;
            if danado {
                // Cobra la mitad del coste (redondeando hacia arriba) al arrancar; si no hay con qué, espera.
                let mitad: Costo = def.costo.iter().map(|&(rec, n)| (rec, (n * FRACCION_RECONSTRUCCION + UNO - 1) / UNO)).collect();
                if !self.almacen.pagar(&mitad) {
                    continue;
                }
                factor = crate::datos::por(factor, FRACCION_RECONSTRUCCION);
            }
            let e = &mut self.edificios[i];
            let completa_en = t + minutos(def.tiempo_minutos, factor) * MS_POR_MINUTO;
            e.estado = Estado::EnConstruccion;
            e.completa_en = Some(completa_en);
            ev.push(Evento::ObraIniciada { id: e.id, tipo, completa_en });
            if !danado {
                cupo -= 1;
            }
        }
    }

    /// ¿Se puede mejorar este edificio ahora? (`elegibleParaMejora`): activo, de un tipo con niveles, sin otra mejora en curso, con
    /// nivel siguiente, con el nivel operativo, la tecnología y el edificio previo que pide.
    pub fn elegible_para_mejora(&self, r: &Reglas, e: &Edificio, x: &Exterior) -> bool {
        if !e.activo() || e.mejora.is_some() || !r.construccion.con_niveles.contains(&e.tipo) {
            return false;
        }
        let Some(sig) = r.catalogo.tipo(e.tipo).nivel(e.nivel_interno + 1) else { return false };
        self.nivel_actual() >= sig.requisito_nivel_asentamiento
            && sig.tecnologia.as_deref().map_or(true, |t| (x.tiene_tecnologia)(t))
            && sig.requiere_edificio.map_or(true, |(t, n)| self.edificios.hay_activo_con_nivel(t, n))
    }

    /// Minutos de una mejora al nivel siguiente: `obraMinutos` propio o la obra base × 2 / 4 / 8, por el factor de tiempo.
    pub fn minutos_de_mejora(&self, r: &Reglas, e: &Edificio) -> i64 {
        let def = r.catalogo.tipo(e.tipo);
        let siguiente = e.nivel_interno as u32 + 1;
        let base = def.nivel(e.nivel_interno + 1).and_then(|n| n.obra_minutos).unwrap_or_else(|| def.tiempo_minutos * r.construccion.multiplicador_mejora_por_nivel.pow(siguiente - 1));
        minutos(base, self.politicas.factor(&r.politicas, "factorTiempoConstruccion"))
    }

    /// Termina las mejoras cuyo instante llegó (la cita de fin de la plaza; el paso de minuto reintenta las que esperaban hueco).
    ///
    /// Una mejora que cambia la huella (la Granja) necesita un hueco de su tamaño nuevo: si no lo hay, **espera** conservando la mejora y
    /// su cuadrilla, y se reintenta en el paso de minuto siguiente.
    pub(crate) fn terminar_mejoras(&mut self, t: Instante, r: &Reglas, x: &mut Exterior, ev: &mut Vec<Evento>) {
        for i in 0..self.edificios.len() {
            let Some(m) = self.edificios[i].mejora else { continue };
            if t < m.completa_en {
                continue;
            }
            let (id, tipo, viejo) = (self.edificios[i].id, self.edificios[i].tipo, self.edificios[i].nivel_interno);
            let u = self.urbano(r);
            if !x.sitios.reubicar(id, tipo, m.nivel_objetivo, &u) {
                continue;
            }
            if tipo == Tipo::Granero {
                // El delta entre niveles.
                self.aplicar_capacidad(r, tipo, m.nivel_objetivo, 1);
                self.aplicar_capacidad(r, tipo, viejo, -1);
            }
            let e = &mut self.edificios[i];
            e.nivel_interno = m.nivel_objetivo;
            e.mejora = None;
            ev.push(Evento::MejoraCompletada { id: e.id, nivel: m.nivel_objetivo });
            if tipo == Tipo::Mercado {
                self.levantar_puestos(r, x, m.nivel_objetivo, ev);
            }
        }
    }

    /// Empieza las mejoras automáticas (siempre dejando una cuadrilla libre para construir). Como la manual, exige el hueco de la huella
    /// nueva al arrancar (Economia_Construccion §7): una Granja sin hueco no arranca y no retiene cuadrilla.
    pub(crate) fn iniciar_mejoras_automaticas(&mut self, t: Instante, r: &Reglas, x: &mut Exterior, reserva: &[Mil; 29], ev: &mut Vec<Evento>) {
        let maxc = r.construccion.necesidades.maximo_en_construccion_simultanea;
        let mut libres = maxc.saturating_sub(self.edificios.cuadrillas_ocupadas()).saturating_sub(1);
        for i in 0..self.edificios.len() {
            if libres == 0 {
                break;
            }
            if !self.elegible_para_mejora(r, &self.edificios[i], x) {
                continue;
            }
            let (tipo, nivel) = (self.edificios[i].tipo, self.edificios[i].nivel_interno);
            let costo = r.catalogo.tipo(tipo).nivel(nivel + 1).map(|n| n.costo_mejora.clone()).unwrap_or_default();
            let id = self.edificios[i].id;
            if !self.puede_iniciar(r, tipo, &costo, reserva) || !x.sitios.hueco_al_mejorar(id, tipo, nivel + 1, &self.urbano(r)) || !self.almacen.pagar(&costo) {
                continue;
            }
            let completa_en = t + self.minutos_de_mejora(r, &self.edificios[i]) * MS_POR_MINUTO;
            let e = &mut self.edificios[i];
            e.mejora = Some(Mejora { nivel_objetivo: nivel + 1, completa_en });
            ev.push(Evento::MejoraIniciada { id: e.id, nivel: nivel + 1, completa_en });
            libres -= 1;
        }
    }

    // ───────────────────────────── Órdenes manuales ─────────────────────────────

    fn tope_transformacion(&self, r: &Reglas, tipo: Tipo) -> bool {
        matches!(tipo, Tipo::Fundicion | Tipo::Curtiduria | Tipo::Armeria | Tipo::Carpinteria)
            && r.construccion.necesidades.maximo_transformacion_por_nivel.get(&self.nivel_actual()).is_some_and(|&tope| self.edificios.en_cualquier_estado(tipo) as u32 >= tope)
    }

    /// `anadirEdificioManualmente`: el Gobernador o el Maestro de Obras encolan cualquier edificio del catálogo salvo el Centro
    /// Urbano y los puestos del Mercado. Paga el coste completo al encolar (con la reserva dinámica, sin la manual) y, si hay una cuadrilla
    /// libre, la obra **arranca en el instante `t` de la orden**. Devuelve el id; en `ev` quedan los edificios gratuitos que creó el
    /// trazado, `Encolada` y, si arrancó, `ObraIniciada`.
    pub fn anadir_manual(&mut self, t: Instante, r: &Reglas, x: &mut Exterior, tipo: Tipo, ev: &mut Vec<Evento>) -> Result<u32, ErrorObra> {
        let def = r.catalogo.tipo(tipo);
        let c = &r.construccion;
        if tipo.es_de_pueblo() && (tipo != Tipo::Cantera || self.edificios.en_cualquier_estado(Tipo::Cantera) > 0) {
            return Err(ErrorObra::SoloPorPueblo);
        }
        if def.tecnologia.as_deref().is_some_and(|t| !(x.tiene_tecnologia)(t)) {
            return Err(ErrorObra::TecnologiaFaltante);
        }
        if def.orden_era_minimo.is_some_and(|o| x.orden_era < o) {
            return Err(ErrorObra::EraInsuficiente);
        }
        if matches!(tipo, Tipo::CentroUrbano | Tipo::PuestoMercado) {
            return Err(ErrorObra::TipoNoManual);
        }
        if c.unicos.contains(&tipo) && self.edificios.en_cualquier_estado(tipo) > 0 {
            return Err(ErrorObra::Unico);
        }
        if tipo == Tipo::Almacen && c.necesidades.maximo_almacenes_por_nivel.get(&self.nivel_actual()).is_some_and(|&m| self.edificios.en_cualquier_estado(Tipo::Almacen) as u32 >= m) {
            return Err(ErrorObra::TopeAlmacenes);
        }
        if tipo == Tipo::Vivienda && self.edificios.en_cualquier_estado(Tipo::Vivienda) as u32 >= self.maximo_viviendas(r) {
            return Err(ErrorObra::TopeViviendas);
        }
        if self.nivel_actual() < def.nivel_construccion {
            return Err(ErrorObra::NivelInsuficiente);
        }
        if self.tope_transformacion(r, tipo) {
            return Err(ErrorObra::TopeTransformacion);
        }
        if tipo == Tipo::GranFundicion && (def.nivel_faccion_minimo.is_some_and(|m| x.nivel_faccion < m) || !x.es_capital || x.faccion_tiene_gran_fundicion) {
            return Err(ErrorObra::GranFundicion);
        }
        if self.edificios.en_cola() >= c.necesidades.maximo_en_cola {
            return Err(ErrorObra::ColaLlena);
        }
        // Los recursos antes que el sitio: preguntar por sitio puede crear un ancla, y una orden rechazada no deja rastro.
        let reserva = self.reserva(r, x, false);
        if !self.puede_iniciar(r, tipo, &def.costo, &reserva) {
            return Err(ErrorObra::RecursosInsuficientes);
        }
        let Some(fuente) = self.proponer(x, tipo, &self.urbano(r), ev) else { return Err(ErrorObra::SinSitio) };
        // El sitio se toma antes de pagar: si ya no queda, no se cobra.
        let id = self.siguiente_edificio;
        self.siguiente_edificio += 1;
        let u = self.urbano(r);
        let Some(gratuitos) = x.sitios.tomar(id, tipo, fuente, &u, &mut self.siguiente_edificio) else {
            self.siguiente_edificio = id;
            return Err(ErrorObra::SinSitio);
        };
        self.almacen.pagar(&def.costo);
        self.alta_gratuitos(gratuitos, ev);
        let mut e = Edificio::nuevo(id, tipo, Estado::EnCola);
        e.prioridad = c.bandas.manual;
        e.fuente = fuente;
        self.edificios.push(e);
        ev.push(Evento::Encolada { id, tipo });
        self.arrancar_obras(t, r, ev);
        Ok(id)
    }

    /// Viviendas que sirven ahora: las que pide el nivel siguiente (ver `ReglasNivel::maximo_viviendas`).
    pub fn maximo_viviendas(&self, r: &Reglas) -> u32 {
        let v = r.catalogo.tipo(Tipo::Vivienda);
        r.nivel.maximo_viviendas(self.nivel_actual(), v.capacidad_pesants / 1000, v.capacidad_artesanos / 1000)
    }

    /// `quitarDeCola`: solo un edificio `en_cola`. Devuelve el coste completo más la mejora de cada nivel interno 2..n; si estaba
    /// dañado, la mitad de cada recurso redondeada hacia abajo. Lo que no quepa en el almacén se pierde.
    pub fn quitar_de_cola(&mut self, r: &Reglas, id: u32) -> Result<(), ErrorObra> {
        let i = self.edificios.iter().position(|e| e.id == id).ok_or(ErrorObra::NoExiste)?;
        if self.edificios[i].estado != Estado::EnCola {
            return Err(ErrorObra::NoEstaEnCola);
        }
        let (tipo, nivel, danado) = (self.edificios[i].tipo, self.edificios[i].nivel_interno, self.edificios[i].danado);
        let def = r.catalogo.tipo(tipo);
        let mut total = def.costo.clone();
        for n in 2..=nivel {
            for &(rec, c) in def.nivel(n).map_or(&[][..], |l| &l.costo_mejora[..]) {
                match total.iter_mut().find(|(x, _)| *x == rec) {
                    Some(f) => f.1 += c,
                    None => total.push((rec, c)),
                }
            }
        }
        if danado {
            total.iter_mut().for_each(|(_, n)| *n = *n * FRACCION_RECONSTRUCCION / UNO);
        }
        self.almacen.devolver(&total);
        self.edificios.remove(i);
        Ok(())
    }

    /// `moverEnCola`: intercambia la `prioridad` con la del vecino de la lista ordenada por prioridad descendente.
    pub fn mover_en_cola(&mut self, id: u32, arriba: bool) -> Result<(), ErrorObra> {
        let mut orden: Vec<usize> = (0..self.edificios.len()).filter(|&i| self.edificios[i].estado == Estado::EnCola).collect();
        orden.sort_by_key(|&i| std::cmp::Reverse(self.edificios[i].prioridad));
        let pos = orden.iter().position(|&i| self.edificios[i].id == id).ok_or(ErrorObra::NoEstaEnCola)?;
        let vecino = if arriba { pos.checked_sub(1) } else { (pos + 1 < orden.len()).then_some(pos + 1) };
        if let Some(v) = vecino {
            let (a, b) = (orden[pos], orden[v]);
            let p = self.edificios[a].prioridad;
            self.edificios[a].prioridad = self.edificios[b].prioridad;
            self.edificios[b].prioridad = p;
        }
        Ok(())
    }

    /// `mejorarEdificioManualmente`: exige que el edificio sea elegible y que se pueda pagar con la reserva dinámica; puede tomar la
    /// última cuadrilla.
    pub fn mejorar_ahora(&mut self, t: Instante, r: &Reglas, x: &mut Exterior, id: u32, ev: &mut Vec<Evento>) -> Result<(), ErrorObra> {
        let i = self.edificios.iter().position(|e| e.id == id).ok_or(ErrorObra::NoExiste)?;
        if !self.elegible_para_mejora(r, &self.edificios[i], x) {
            return Err(ErrorObra::NoEsMejorable);
        }
        if self.edificios.cuadrillas_ocupadas() >= r.construccion.necesidades.maximo_en_construccion_simultanea {
            return Err(ErrorObra::SinCuadrilla);
        }
        let (tipo, nivel) = (self.edificios[i].tipo, self.edificios[i].nivel_interno);
        let costo = r.catalogo.tipo(tipo).nivel(nivel + 1).map(|n| n.costo_mejora.clone()).unwrap_or_default();
        let reserva = self.reserva(r, x, false);
        if !self.puede_iniciar(r, tipo, &costo, &reserva) {
            return Err(ErrorObra::RecursosInsuficientes);
        }
        // Si la huella cambia, ya debe haber hueco para reubicar (Economia_Construccion §7).
        if !x.sitios.hueco_al_mejorar(id, tipo, nivel + 1, &self.urbano(r)) {
            return Err(ErrorObra::SinSitio);
        }
        self.almacen.pagar(&costo);
        let completa_en = t + self.minutos_de_mejora(r, &self.edificios[i]) * MS_POR_MINUTO;
        self.edificios[i].mejora = Some(Mejora { nivel_objetivo: nivel + 1, completa_en });
        ev.push(Evento::MejoraIniciada { id, nivel: nivel + 1, completa_en });
        Ok(())
    }

    /// `alternarAutoConstruccion`: pausa o reanuda solo la detección de necesidades nuevas.
    pub fn alternar_auto_construccion(&mut self, pausada: bool) -> bool {
        std::mem::replace(&mut self.auto_pausada, pausada) != pausada
    }

    /// `alternarReceta`: para o reanuda una receta por el recurso que produce, en todos los talleres.
    pub fn alternar_receta(&mut self, recurso: Recurso, pausada: bool) -> bool {
        let esta = self.recetas_pausadas.contains(&recurso);
        match (pausada, esta) {
            (true, false) => self.recetas_pausadas.push(recurso),
            (false, true) => self.recetas_pausadas.retain(|&x| x != recurso),
            _ => return false,
        }
        true
    }

    /// `calibrarReservaManual`: `clamp(round(valor), 0, reservaManualMaxima)`.
    pub fn calibrar_reserva(&mut self, r: &Reglas, recurso: Recurso, valor: Mil) {
        self.reserva_manual[recurso.indice()] = valor.clamp(0, r.construccion.necesidades.reserva_manual_maxima);
    }
}
