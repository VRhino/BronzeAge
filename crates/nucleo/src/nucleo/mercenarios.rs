//! Los campamentos de mercenarios dentro del núcleo (Comercio_Caravanas §9, Campamentos_Mercenarios §4-§6): comprar, repostar gratis, reclutar
//! con oro, la tropa prestada, la residencia del héroe y la reposición cada `cadaHoras` del mercado con lo comerciado entre Facciones. Las
//! cuentas son de los crates `comercio::mercenarios` (mercado) y `militar::mercenarios` (reclutas y tropa); aquí están el estado, la puerta del
//! campamento, el reloj de la reposición y el traslado a eventos.
//!
//! # La reposición sin coste por campamento ocioso
//! No hay evento recurrente mientras nada cambie: el mercado nace lleno y solo se programa la reposición (a la fase de siempre,
//! `creación + k × cadaHoras`) cuando algo la hace falta: se comerció entre Facciones (contadores > 0) o una compra dejó una pila por debajo
//! de su tope. Tras reponer, si ya no hay nada que reponer, el evento no se vuelve a programar. El stock de un campamento que nadie ha tocado
//! no se guarda (es el inicial); lo mismo vale para sus reclutas, que no se escriben nunca por minuto (valor e instante).
//!
//! # Los campamentos y los héroes
//! Quien compra, reposta o recluta es un héroe, que debe estar dentro del campamento o con su columna a la puerta. El residente (el héroe cuya
//! residencia es ese campamento) paga de su almacén personal y compra con cupo diario; el visitante, con el oro del carro de su columna. El
//! edificio militar de un campamento sale de su posición (`edificiosMilitares[huella mod 3]`, Generacion_Del_Mundo §11) y sus viviendas,
//! de `edificiosFijos`: ninguno se guarda.
//!
//! # Las tecnologías del campamento
//! Las de Tecnologia_Aedas §5 (6C): las de arranque; las demás, desde que alguien la desbloqueó (`primeros`) y pasaron 72 h (su cita de campamento) o en cuanto la tiene adoptada
//! al menos la mitad de las Facciones vivas. No se guardan: se miran al reclutar y al proyectar.

use super::*;
use crate::estado::SucesoComercio;
use crate::estado_comercio::entidad_mercenarios;
use crate::comando::PagarCon;
use asentamiento::datos::UNO;
use asentamiento::Recurso;
use comercio::evento::Evento as EventoComercio;
use comercio::mercenarios::{comprar_residente, comprar_visitante, precio_unitario, ErrorMercenarios, StockCampamento};
use comercio::precio::{precio_referencia, stock_global};
use comercio::{de, total, Carga as Mercancia};
use militar::catalogo::Tropa;
use militar::escuadra::Contenedor;
use militar::guarnicion;
use militar::heroe::Residencia;
use militar::mercenarios::{self as leva, Campamento, ErrorMercenarios as ErrorLeva, Pedido, Reclutas};
use militar::presencia::Ubicacion;
use militar::racion;

fn rechazo_de_mercenarios(e: ErrorMercenarios) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorMercenarios::CantidadInvalida => (MERCENARIOS_CANTIDAD_INVALIDA, "se compran unidades enteras (al menos una) y el oro no se vende"),
        ErrorMercenarios::SinPrecio => (MERCENARIOS_SIN_PRECIO, "ese bien no está en venta"),
        ErrorMercenarios::TrigoNoVaAlCarro => (MERCENARIOS_TRIGO_NO_VA_AL_CARRO, "el trigo no se compra para el carro: se reposta gratis a víveres"),
        ErrorMercenarios::SinStock => (MERCENARIOS_SIN_STOCK, "el campamento no tiene existencias de ese bien"),
        ErrorMercenarios::CupoAgotado => (MERCENARIOS_CUPO_AGOTADO, "se agotó el cupo diario de ese bien"),
        ErrorMercenarios::SinOro => (MERCENARIOS_SIN_ORO, "no hay oro bastante para una unidad"),
        ErrorMercenarios::SinSitio => (MERCENARIOS_SIN_SITIO, "no hay sitio para la compra"),
        ErrorMercenarios::VivereLlenos => (MERCENARIOS_VIVERES_LLENOS, "los víveres ya están llenos"),
    }
}

fn rechazo_de_leva(e: ErrorLeva) -> (&'static str, String) {
    use rechazos::*;
    match e {
        ErrorLeva::NoResides => (MERCENARIOS_NO_RESIDES, "solo se recluta en el campamento donde se reside".into()),
        ErrorLeva::Lejos => (CAMPAMENTO_FUERA_DE_LA_PUERTA, "hay que estar dentro del campamento o con la columna a su puerta".into()),
        ErrorLeva::NoDisponible => (MERCENARIOS_TROPA_NO_DISPONIBLE, "el campamento no ofrece esa tropa".into()),
        ErrorLeva::YaLaTienesFuera(c) => (MERCENARIOS_YA_LA_TIENES_FUERA, format!("ya tienes esa escuadra y no está aquí ({c:?}): solo se repone donde está")),
        ErrorLeva::AlTope => (MERCENARIOS_AL_TOPE, "la escuadra ya está completa".into()),
        ErrorLeva::SinReclutas { hacen_falta, hay } => (MERCENARIOS_SIN_RECLUTAS, format!("hacen falta {hacen_falta} reclutas y el campamento tiene {hay}")),
        ErrorLeva::SinOro => (MERCENARIOS_SIN_ORO, "no hay oro bastante".into()),
        ErrorLeva::SinTropas => (MERCENARIOS_PRESTAMO_SIN_TROPAS, "un préstamo pide al menos una tropa".into()),
        ErrorLeva::NoSePresta => (MERCENARIOS_NO_SE_PRESTA, "solo se presta la tropa de leva comunal".into()),
        ErrorLeva::Repetida => (MERCENARIOS_PRESTAMO_REPETIDA, "una tropa no puede repetirse en el préstamo".into()),
        ErrorLeva::YaPrestada => (MERCENARIOS_YA_PRESTADA, "esa tropa ya está prestada".into()),
        ErrorLeva::NadaQueReponer => (MERCENARIOS_NADA_QUE_REPONER, "no hay tropa prestada que reponer".into()),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// El stock en venta de un campamento del artefacto (el inicial si nadie lo ha tocado). `None` si no existe.
    pub fn stock_de_campamento(&self, campamento: u32) -> Option<StockCampamento> {
        let n = self.geografia.as_ref()?.mapa.campamentos.len();
        ((campamento as usize) < n).then(|| self.estado.comercio.mercenarios.stock.get(&campamento).cloned().unwrap_or_else(|| StockCampamento::nuevo(&self.estado.reglas.economia.comercio)))
    }

    /// El estado del mercado de mercenarios (contadores, stock y cita de reposición).
    pub fn mercenarios(&self) -> &crate::estado_comercio::EstadoMercenarios {
        &self.estado.comercio.mercenarios
    }

    /// Los reclutas que tiene el campamento ahora, en milésimas. `None` si no existe.
    pub fn reclutas_de_campamento(&self, campamento: u32) -> Option<Mil> {
        self.puerta_de_campamento(campamento)?;
        Some(self.reclutas_de(campamento).actual(&self.estado.reglas.economia.militar, self.estado.reglas.economia.militar.mercenarios.viviendas, self.estado.ahora))
    }

    /// Las tropas que ofrece el campamento ahora (su edificio militar, su nivel y la tecnología desbloqueada). `None` si no existe.
    pub fn tropas_del_campamento(&self, campamento: u32) -> Option<Vec<Tropa>> {
        let edificio = self.edificio_militar(campamento)?;
        Some(leva::tropas_del_campamento(&self.estado.reglas.economia.militar, &[edificio], &self.tecnologia_del_campamento()))
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    /// Dónde está la puerta del campamento `i` del artefacto.
    pub fn puerta_de_campamento(&self, campamento: u32) -> Option<Punto> {
        self.geografia.as_ref()?.mapa.campamentos.get(campamento as usize).copied()
    }

    /// La puerta de cada campamento del artefacto, con su índice (sin contar quién reside en él, que es lo caro de [`Nucleo::campamentos_para_elegir`]).
    pub fn puertas_de_campamentos(&self) -> impl Iterator<Item = (u32, Punto)> + '_ {
        self.geografia.iter().flat_map(|g| (0u32..).zip(g.mapa.campamentos.iter().copied()))
    }

    /// Los campamentos donde un jugador sin héroe puede nacer (`Bienvenida.campamentos_para_elegir`): de cada uno del artefacto, su índice, su puerta, su origen y cuántos héroes residen
    /// en él. DUDA: `eligieronComoInicial` no se guarda todavía (fila de paridad «Militar y combate (4B-1)»); la sesión lo manda a 0.
    pub fn campamentos_para_elegir(&self) -> Vec<(u32, Punto, u32, usize)> {
        let Some(g) = self.geografia.as_ref() else { return Vec::new() };
        let residentes = |c: u32| self.heroes().filter(|h| h.heroe.residencia == Some(Residencia::Campamento(c))).count();
        g.campamentos_del_artefacto().into_iter().zip(0u32..).map(|((puerta, origen), c)| (c, puerta, origen, residentes(c))).collect()
    }

    /// El origen cultural del campamento `c` del artefacto (el de su dueño por defecto: `Bienvenida.campamentos_para_elegir`).
    pub fn origen_de_campamento(&self, campamento: u32) -> Option<u32> {
        self.geografia.as_ref()?.campamentos_del_artefacto().get(campamento as usize).map(|&(_, origen)| origen)
    }

    /// El edificio militar del campamento, elegido por su posición: `huella = |⌊x⌋·31 + ⌊y⌋·17 + índice|`.
    pub fn edificio_militar(&self, campamento: u32) -> Option<asentamiento::Tipo> {
        let p = self.puerta_de_campamento(campamento)?;
        let lista = &self.estado.reglas.economia.militar.mercenarios.edificios_militares;
        let huella = (p.x.div_euclid(1_000) * 31 + p.y.div_euclid(1_000) * 17 + campamento as i64).unsigned_abs();
        Some(lista[huella as usize % lista.len()])
    }

    /// Los reclutas del campamento: los guardados, o el campamento lleno si nadie lo ha tocado.
    pub fn reclutas_de(&self, campamento: u32) -> Reclutas {
        let mr = &self.estado.reglas.economia.militar;
        self.estado.militar.reclutas.get(&campamento).copied().unwrap_or_else(|| Reclutas::llenos(mr, mr.mercenarios.viviendas, 0))
    }

    /// ¿Desbloquea el campamento esa tecnología (Tecnologia_Aedas §5, 6C)? Las de arranque siempre; las demás, si alguien la desbloqueó y pasaron `horasTrasLaPrimera` o si la tienen
    /// adoptada al menos `porcentajeFacciones` de las Facciones vivas (las que conservan alguna plaza).
    fn tecnologia_del_campamento(&self) -> impl Fn(&str) -> bool + '_ {
        let libres = self.tecnologias_de_los_campamentos();
        move |tecnologia| crate::estado::adopta(&self.estado.reglas.economia.tecnologia, libres, tecnologia)
    }

    /// ¿Está el héroe en el campamento? Dentro, o con su columna a la puerta.
    pub(super) fn esta_en_el_campamento(&self, heroe: u64, campamento: u32, t: Instante) -> bool {
        let Some(puerta) = self.puerta_de_campamento(campamento) else { return false };
        match self.heroe(heroe).map(|h| h.heroe.presencia.ubicacion) {
            Some(Ubicacion::Mercenarios(c)) => c == campamento,
            Some(Ubicacion::Columna(e)) => self.estado.ejercitos.get(&e).is_some_and(|e| e.posicion_en(t).dist(puerta) <= self.radio_puerta()),
            _ => false,
        }
    }

    /// El campamento donde está el héroe cuando la orden no lo dice (`comprar_en_campamento` y `repostar_en_campamento` sin `campamento_id`): aquel en el que está dentro
    /// o, si va en su columna, el más cercano a cuya puerta está. `None` si no está en ninguno.
    pub fn campamento_del_heroe(&self, heroe: u64) -> Option<u32> {
        if let Ubicacion::Mercenarios(c) = self.heroe(heroe)?.heroe.presencia.ubicacion {
            return Some(c);
        }
        let aqui = self.columna_del_heroe(heroe)?.posicion_en(self.estado.ahora);
        let todos = 0..u32::try_from(self.geografia.as_ref()?.mapa.campamentos.len()).ok()?;
        todos.filter_map(|c| Some((aqui.dist(self.puerta_de_campamento(c)?), c))).filter(|&(d, _)| d <= self.radio_puerta()).min().map(|(_, c)| c)
    }

    /// El campamento donde reside el héroe, si reside en uno.
    fn campamento_de_residencia(&self, heroe: u64) -> Option<u32> {
        match self.heroe(heroe)?.heroe.residencia {
            Some(Residencia::Campamento(c)) => Some(c),
            _ => None,
        }
    }

    /// Cambia la residencia del héroe: suelta su guarnición y retira la tropa prestada por un campamento donde ya no reside, de donde esté (también
    /// de la escolta de una caravana). La reubicación forzosa de una ruina y `FijarResidencia` pasan por aquí.
    pub(super) fn mudar_de_residencia(&mut self, heroe: u64, nueva: Option<Residencia>) {
        let t = self.estado.ahora;
        let antes = self.heroe(heroe).and_then(|h| h.heroe.residencia);
        for r in [antes, nueva].into_iter().flatten() {
            match r {
                Residencia::Campamento(c) => self.tr.tocar(Clase::Campamento, u64::from(c)),
                Residencia::Asentamiento(a) => self.tr.tocar(Clase::Asentamiento, u64::from(a)),
            }
        }
        self.tr.tocar(Clase::Heroe, heroe);
        self.estado.militar.mudar(heroe, nueva);
        // Q4: quien deja de residir en una plaza deja sus cargos en ella.
        if let Some(Residencia::Asentamiento(a)) = antes.filter(|&r| Some(r) != nueva) {
            self.vaciar_cargos_de(heroe, u64::from(a));
            // Y deja de ser uno de sus fundadores (5H, 5B-9; `faccion.ts` lo quitaba de `heroesFundadoresIds`).
            if let Some(plaza) = self.estado.asentamientos.get_mut(&u64::from(a)) {
                plaza.fundadores.retain(|&h| h != heroe);
            }
        }
        // Llega o se va un residente de un campamento: cambia la demanda de bandidos de su anillo.
        for c in [antes, nueva].into_iter().flatten().filter_map(|r| if let Residencia::Campamento(c) = r { Some(c) } else { None }) {
            self.reprogramar_anillo(c, t);
        }
        let velocidades: std::collections::BTreeMap<u64, Mil> = self.heroe(heroe).map_or_else(Default::default, |h| h.heroe.escuadras.iter().filter_map(|e| if let Contenedor::Ejercito(c) = e.contenedor { Some((c, self.velocidad_de_ejercito(c)?)) } else { None }).collect());
        let h = &mut self.estado.militar.heroes.get_mut(&heroe).expect("existe").heroe;
        guarnicion::soltar(h);
        // La guarnición de una avanzada es también de la plaza donde se reside: al mudarse, esas escuadras vuelven al campamento, intactas (final normal).
        h.escuadras.iter_mut().filter(|e| matches!(e.contenedor, Contenedor::Avanzada(_))).for_each(|e| e.contenedor = Contenedor::Campamento);
        let retiradas = leva::retirar_prestamos_ajenos(h, &|c| nueva == Some(Residencia::Campamento(c)));
        self.tr.tocar_todos(Clase::Escuadra, retiradas.iter().map(|e| e.id));
        let mut columnas = Vec::new();
        for e in &retiradas {
            match e.contenedor {
                Contenedor::Escolta(c) => {
                    if let Some(i) = self.estado.comercio.posicion_caravana(c) {
                        self.estado.comercio.flota[i].escolta.retain(|x| x.id != e.id);
                        self.tr.tocar(Clase::Caravana, c);
                    }
                }
                Contenedor::Ejercito(c) => {
                    if let Some(col) = self.estado.ejercitos.get_mut(&c) {
                        col.columna.escuadras.retain(|x| *x != e.id);
                        self.tr.tocar(Clase::Ejercito, c);
                        columnas.push(c);
                    }
                }
                Contenedor::Campamento | Contenedor::Fuera | Contenedor::Avanzada(_) => {}
            }
        }
        columnas.sort_unstable();
        columnas.dedup();
        // La tropa que se va de una columna cambia lo que come y a qué velocidad marcha.
        for c in columnas {
            self.recomponer_columna(c, t, velocidades.get(&c).copied().unwrap_or(0));
        }
        if !retiradas.is_empty() {
            self.emitir_propio(t, "mercenarios.prestamo_retirado", vec![], vec![("heroe", heroe as i64), ("escuadras", retiradas.len() as i64)]);
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Las órdenes de los campamentos y la residencia. `Ok(ids)` = cambió algo y creó esos ids; los rechazos y «sin cambios» salen como `Err` sin
    /// haber tocado el estado.
    pub(super) fn ejecutar_mercenarios(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta::sin_cambios(v);
        let de_mercenarios = |e: ErrorMercenarios| {
            let (c, m) = rechazo_de_mercenarios(e);
            rechazo(c, m.into())
        };
        let de_leva = |e: ErrorLeva| {
            let (c, m) = rechazo_de_leva(e);
            rechazo(c, m)
        };
        let cr = self.estado.reglas.economia.comercio.clone();
        let mr = self.estado.reglas.economia.militar.clone();
        match &p.comando {
            Comando::ComprarEnCampamento { heroe, campamento, recurso, cantidad } => {
                self.autorizar_heroe(p, *heroe, false)?;
                self.puerta_de_campamento(*campamento).ok_or_else(|| rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, ids::campamento(*campamento)))?;
                if !self.esta_en_el_campamento(*heroe, *campamento, t) {
                    return Err(rechazo(rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA, "el héroe no está en el campamento ni con la columna a su puerta".into()));
                }
                let referencia = precio_referencia(&cr.precio, *recurso, stock_global(*recurso, self.estado.asentamientos.values().map(|a| &a.economia.almacen)));
                let precio = precio_unitario(&cr.mercenarios, referencia);
                let stock = self.stock_de_campamento(*campamento).expect("el campamento existe").de(*recurso);
                let residente = self.campamento_de_residencia(*heroe) == Some(*campamento);
                let h = self.heroe(*heroe).expect("existe");
                let columna = self.columna_de(*heroe);
                let compra = if residente {
                    let cupo = h.cupo.restante(&cr.mercenarios, *recurso, t);
                    comprar_residente(*recurso, precio, *cantidad, stock, cupo, de(&h.almacen_personal, Recurso::Oro), cr.mercenarios.capacidad_almacen_personal - total(&h.almacen_personal)).map_err(de_mercenarios)?
                } else {
                    let columna = columna.ok_or_else(|| rechazo(rechazos::HEROE_SIN_COLUMNA, "el visitante paga con el carro de su columna: el héroe no va en ninguna".into()))?;
                    let carro = &self.estado.ejercitos[&columna].columna.carga;
                    let libre = self.capacidad_de_carga(columna).unwrap_or(0) - total(carro);
                    comprar_visitante(*recurso, precio, *cantidad, stock, de(carro, Recurso::Oro), libre).map_err(de_mercenarios)?
                };
                // El oro cobrado se destruye (sumidero); la compra va al almacén personal (residente) o al carro (visitante).
                let carga = if residente {
                    &mut self.estado.militar.heroes.get_mut(heroe).expect("existe").almacen_personal
                } else {
                    let columna = columna.expect("el visitante va en una columna");
                    self.tr.tocar(Clase::Ejercito, columna);
                    &mut self.estado.ejercitos.get_mut(&columna).expect("existe").columna.carga
                };
                *carga.entry(Recurso::Oro).or_insert(0) -= compra.oro_cobrado;
                *carga.entry(*recurso).or_insert(0) += compra.cantidad;
                carga.retain(|_, n| *n > 0);
                if residente && cr.mercenarios.cupo_diario.contains_key(recurso) {
                    self.heroe_mut(*heroe).cupo.anotar(*recurso, compra.cantidad, t);
                }
                let m = &mut self.estado.comercio.mercenarios;
                let mut s = m.stock.get(campamento).cloned().unwrap_or_else(|| StockCampamento::nuevo(&cr));
                *s.stock.entry(*recurso).or_insert(0) -= compra.cantidad;
                m.stock.insert(*campamento, s);
                let ev = EventoComercio::MercenariosComprado { campamento: *campamento, heroe: *heroe, recurso: *recurso, cantidad: compra.cantidad };
                self.emitir_comercio(t, ev, vec![], &[("oro", compra.oro_cobrado), ("residente", residente as i64)]);
                self.armar_reposicion(t);
                Ok(vec![])
            }
            Comando::RepostarEnCampamento { heroe, campamento } => {
                self.autorizar_heroe(p, *heroe, false)?;
                self.puerta_de_campamento(*campamento).ok_or_else(|| rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, ids::campamento(*campamento)))?;
                if !self.esta_en_el_campamento(*heroe, *campamento, t) {
                    return Err(rechazo(rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA, "el héroe no está en el campamento ni con la columna a su puerta".into()));
                }
                let falta = match self.columna_de(*heroe) {
                    Some(c) => {
                        let e = &self.estado.ejercitos[&c];
                        e.columna.viveres.maximo.unwrap_or(0) - e.columna.viveres.valor_en(t)
                    }
                    None => mr.logistica.capacidad_viveres - self.heroe(*heroe).expect("existe").heroe.viveres,
                };
                if falta <= 0 {
                    return Err(de_mercenarios(ErrorMercenarios::VivereLlenos));
                }
                let datos = vec![("campamento", *campamento as i64), ("heroe", *heroe as i64), ("cantidad", falta)];
                match self.columna_de(*heroe) {
                    Some(c) => {
                        let e = self.estado.ejercitos.get_mut(&c).expect("existe");
                        racion::llenar(&mut e.columna.viveres, t, falta);
                        self.reprogramar_columna(c, t);
                        self.evento("mercenarios.repostado", t, c, None, datos);
                    }
                    None => {
                        self.heroe_mut(*heroe).heroe.viveres = mr.logistica.capacidad_viveres;
                        self.emitir_propio(t, "mercenarios.repostado", vec![], datos);
                    }
                }
                Ok(vec![])
            }
            Comando::FijarResidencia { heroe, residencia } => {
                self.autorizar_heroe(p, *heroe, true)?;
                match residencia {
                    Some(Residencia::Asentamiento(a)) => {
                        self.plaza_o_rechazo(*a as u64)?;
                    }
                    Some(Residencia::Campamento(c)) => {
                        self.puerta_de_campamento(*c).ok_or_else(|| rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, ids::campamento(*c)))?;
                    }
                    None => {}
                }
                if self.heroe(*heroe).expect("existe").heroe.residencia == *residencia {
                    return Err(sin_cambios());
                }
                self.mudar_de_residencia(*heroe, *residencia);
                Ok(vec![])
            }
            Comando::DarAlmacenPersonal { heroe, recurso, cantidad } => {
                self.autorizar_heroe(p, *heroe, true)?;
                if *cantidad <= 0 {
                    return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "la cantidad debe ser mayor que cero".into()));
                }
                let h = self.heroe_mut(*heroe);
                if h.heroe.residencia.is_none() {
                    return Err(rechazo(rechazos::HEROE_SIN_RESIDENCIA, "el almacén personal vive donde se reside".into()));
                }
                let n = base::entero((*cantidad).min((cr.mercenarios.capacidad_almacen_personal - total(&h.almacen_personal)).max(0))); // unidades enteras (C3b)
                if n == 0 {
                    return Err(sin_cambios());
                }
                *h.almacen_personal.entry(*recurso).or_insert(0) += n;
                Ok(vec![])
            }
            Comando::ReclutarEnCampamento { heroe, tropa, pagar_con } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let campamento = self.campamento_de_residencia(*heroe).ok_or_else(|| de_leva(ErrorLeva::NoResides))?;
                let esta = self.esta_en_el_campamento(*heroe, campamento, t);
                let columna = self.columna_de(*heroe);
                let velocidad_antes = columna.and_then(|c| self.velocidad_de_ejercito(c)).unwrap_or(0);
                let lider_de = columna.filter(|c| self.estado.ejercitos.get(c).is_some_and(|e| e.columna.lider == *heroe));
                let (oro_disponible, de_la_columna) = match (pagar_con, columna) {
                    (PagarCon::AlmacenPersonal, _) => (de(&self.heroe(*heroe).expect("existe").almacen_personal, Recurso::Oro), None),
                    (PagarCon::Carro, Some(c)) => (de(&self.estado.ejercitos[&c].columna.carga, Recurso::Oro), Some(c)),
                    (PagarCon::Carro, None) => return Err(rechazo(rechazos::HEROE_SIN_COLUMNA, "para pagar con el carro el héroe debe ir en una columna".into())),
                };
                let faccion = self.heroe(*heroe).expect("existe").faccion;
                let factor_reputacion = faccion.map_or(UNO, |f| if self.estado.comercio.reputacion_en(f, t) <= cr.reputacion.umbral_bajo_para_comision { cr.reputacion.factor_comision_por_reputacion_baja } else { UNO });
                let sin_asentamientos = faccion.is_some_and(|f| self.plazas_de(f).next().is_none());
                let ofrece = self.tropas_del_campamento(campamento).expect("el campamento existe");
                let mut reclutas = self.reclutas_de(campamento);
                let puerta = self.puerta_de_campamento(campamento).expect("existe");
                let Estado { militar, ejercitos, siguiente_id, .. } = &mut self.estado;
                let h = &mut militar.heroes.get_mut(heroe).expect("existe").heroe;
                let antes = h.escuadras.len();
                let distancia = |c: u64| ejercitos.get(&c).map(|e| e.posicion_en(t).dist(puerta));
                let hecho = leva::reclutar(
                    &mr,
                    Campamento { id: campamento, viviendas: mr.mercenarios.viviendas, reclutas: &mut reclutas, ofrece: &ofrece },
                    Pedido {
                        heroe: &mut *h,
                        tropa: *tropa,
                        reside: true,
                        esta_en_el_campamento: esta,
                        distancia_de_columna: &distancia,
                        oro_disponible,
                        factor_reputacion,
                        sin_asentamientos,
                        columna_del_lider: lider_de,
                    },
                    || {
                        *siguiente_id += 1;
                        *siguiente_id - 1
                    },
                    t,
                )
                .map_err(de_leva)?;
                let nueva = h.escuadras.len() > antes;
                // Los logros `reclutados.*` también cuentan el recluta de un campamento (Q4 del autor).
                let reclutado = tecnologia::contador::reclutamiento(tropa.nombre(), mr.tropas.tropa(*tropa).edificio, hecho.cantidad.max(0) as u64, nueva);
                let contenedor = h.escuadra(hecho.escuadra).map(|e| e.contenedor);
                if let (Some(c), Some(Contenedor::Ejercito(_))) = (hecho.se_une, contenedor) {
                    ejercitos.get_mut(&c).expect("existe").columna.escuadras.push(hecho.escuadra);
                }
                militar.reclutas.insert(campamento, reclutas);
                self.tr.tocar(Clase::Campamento, u64::from(campamento));
                let carga = match de_la_columna {
                    Some(c) => &mut ejercitos.get_mut(&c).expect("existe").columna.carga,
                    None => &mut militar.heroes.get_mut(heroe).expect("existe").almacen_personal,
                };
                *carga.entry(Recurso::Oro).or_insert(0) -= hecho.oro;
                carga.retain(|_, n| *n > 0);
                if let Some(c) = de_la_columna {
                    self.tr.tocar(Clase::Ejercito, c);
                }
                // La escuadra nueva o repuesta que va en la columna cambia lo que come y a qué velocidad marcha.
                if let Some(Contenedor::Ejercito(c)) = contenedor {
                    self.recomponer_columna(c, t, velocidad_antes);
                }
                self.emitir_propio(
                    t,
                    "mercenarios.reclutado",
                    vec![],
                    vec![("campamento", campamento as i64), ("heroe", *heroe as i64), ("tropa", crate::codec_militar::tropa_a_u8(*tropa) as i64), ("cantidad", hecho.cantidad), ("oro", hecho.oro), ("escuadra", hecho.escuadra as i64), ("nueva", nueva as i64)],
                );
                self.contar(t, reclutado);
                Ok(if nueva { vec![hecho.escuadra] } else { vec![] })
            }
            Comando::PedirPrestamo { heroe, tropas } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let campamento = self.campamento_de_residencia(*heroe).ok_or_else(|| de_leva(ErrorLeva::NoResides))?;
                if !self.esta_en_el_campamento(*heroe, campamento, t) {
                    return Err(de_leva(ErrorLeva::Lejos));
                }
                let Estado { militar, siguiente_id, .. } = &mut self.estado;
                let h = &mut militar.heroes.get_mut(heroe).expect("existe").heroe;
                let creadas = leva::pedir_prestamo(
                    &mr,
                    h,
                    campamento,
                    tropas,
                    &mut || {
                        *siguiente_id += 1;
                        *siguiente_id - 1
                    },
                    t,
                )
                .map_err(de_leva)?;
                self.emitir_propio(t, "mercenarios.prestamo", vec![], vec![("campamento", campamento as i64), ("heroe", *heroe as i64), ("escuadras", creadas.len() as i64)]);
                Ok(creadas)
            }
            Comando::ReponerPrestamo { heroe } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let campamento = self.campamento_de_residencia(*heroe).ok_or_else(|| de_leva(ErrorLeva::NoResides))?;
                if !self.esta_en_el_campamento(*heroe, campamento, t) {
                    return Err(de_leva(ErrorLeva::Lejos));
                }
                let columna = self.columna_de(*heroe);
                let velocidad_antes = columna.and_then(|c| self.velocidad_de_ejercito(c)).unwrap_or(0);
                let puerta = self.puerta_de_campamento(campamento).expect("existe");
                let Estado { militar, ejercitos, .. } = &mut self.estado;
                let distancia = |c: u64| ejercitos.get(&c).map(|e| e.posicion_en(t).dist(puerta));
                let repuestas = leva::reponer_prestamo(&mr, &mut militar.heroes.get_mut(heroe).expect("existe").heroe, campamento, &distancia).map_err(de_leva)?;
                // La tropa repuesta que va en la columna a su puerta come más: se vuelve a anotar el consumo.
                if let Some(c) = columna {
                    self.recomponer_columna(c, t, velocidad_antes);
                }
                self.emitir_propio(t, "mercenarios.prestamo_repuesto", vec![], vec![("heroe", *heroe as i64), ("repuestas", repuestas as i64)]);
                Ok(vec![])
            }
            _ => unreachable!("solo se llama con órdenes de los campamentos"),
        }
    }

    /// Cambió algo de la columna que mueve sus eventos (los víveres, la tropa): anota el hambre de cada escuadra, sube su generación y reprograma llegada, umbrales, deserción y encuentros.
    pub(super) fn reprogramar_columna(&mut self, id: u64, t: Instante) {
        self.sincronizar_hambre(id, t);
        let g = self.estado.generaciones.subir(id);
        let e = self.estado.ejercitos.get_mut(&id).expect("existe");
        let marchando = e.marchando();
        if marchando {
            e.plan.generacion = g;
        }
        self.tr.tocar(Clase::Ejercito, id);
        self.programar_ejercito(id, t);
        if marchando {
            self.avisar_plan(id, true);
        }
        self.anunciar_columna(id, Motivo::Ritmo);
    }

    // ───────────────────────────── Reposición ─────────────────────────────

    /// El mercado de mercenarios es uno solo (sus contadores y su próxima reposición): lo que lo cambia toca a todos los campamentos.
    pub(super) fn tocar_todos_los_campamentos(&mut self) {
        let n = self.geografia.as_ref().map_or(0, |g| g.mapa.campamentos.len() as u64);
        self.tr.tocar_todos(Clase::Campamento, 0..n);
    }

    /// Anota lo comerciado entre Facciones en los contadores de reposición (§9.3) y, si cuenta, arma la reposición.
    pub(super) fn registrar_comercio_de_mercenarios(&mut self, entre_facciones: bool, carga: &Mercancia) {
        let rm = self.estado.reglas.economia.comercio.mercenarios.clone();
        self.estado.comercio.mercenarios.mercado.registrar_comercio(&rm, entre_facciones, carga);
        self.tocar_todos_los_campamentos();
        if entre_facciones && !carga.is_empty() {
            let t = self.estado.ahora;
            self.armar_reposicion(t);
        }
    }

    /// ¿Hay algo que reponer? Contadores de comercio sin aplicar, o una pila por debajo de su tope.
    fn hay_que_reponer(&self) -> bool {
        let m = &self.estado.comercio.mercenarios;
        let pilas = &self.estado.reglas.economia.comercio.mercenarios.pilas;
        !m.mercado.contadores.is_empty() || m.stock.values().any(|s| pilas.iter().any(|(&b, &tope)| s.de(b) < tope))
    }

    /// Programa la reposición en la próxima fase de `cadaHoras` (la de siempre: `creación + k × cada`) si hace falta y no hay ya una programada.
    pub(super) fn armar_reposicion(&mut self, t: Instante) {
        if self.estado.comercio.mercenarios.pulso.is_some() || !self.hay_que_reponer() {
            return;
        }
        let cada = self.estado.reglas.economia.comercio.mercenarios.cada;
        let m = &mut self.estado.comercio.mercenarios;
        // Las citas en las que no había nada que reponer no hicieron nada: se salta a la primera fase que no ha pasado.
        if m.mercado.repone_en < t {
            m.mercado.repone_en += (t - m.mercado.repone_en + cada - 1) / cada * cada;
        }
        let due = m.mercado.repone_en;
        m.pulso = Some(due);
        self.tocar_todos_los_campamentos();
        self.programar_comercio(due, entidad_mercenarios(), SucesoComercio::ReponeMercenarios);
    }

    /// Vence la reposición (`reponerMercados`, §9.3).
    pub(super) fn repone_mercenarios(&mut self, t: Instante) {
        self.estado.comercio.mercenarios.pulso = None;
        let cr = self.estado.reglas.economia.comercio.clone();
        let n = self.geografia.as_ref().map_or(0, |g| g.mapa.campamentos.len() as u32);
        let m = &mut self.estado.comercio.mercenarios;
        let mut campamentos: Vec<StockCampamento> = (0..n).map(|i| m.stock.get(&i).cloned().unwrap_or_else(|| StockCampamento::nuevo(&cr))).collect();
        if m.mercado.reponer(&cr.mercenarios, t, &mut campamentos) {
            let inicial = StockCampamento::nuevo(&cr);
            for (i, s) in campamentos.into_iter().enumerate() {
                if s == inicial {
                    m.stock.remove(&(i as u32));
                } else {
                    m.stock.insert(i as u32, s);
                }
            }
            self.tocar_todos_los_campamentos();
            // Es público: los mercados de todos los campamentos se reponen a la vez, no dice nada que oculte la niebla.
            self.tr.emitir(&self.estado, EventoNucleo { publico: true, ..EventoNucleo::nuevo("mercenarios.repuesto", t, vec![("campamentos", n as i64)]) });
        }
        self.armar_reposicion(t);
    }
}
