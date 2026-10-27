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
//!    comerciales; la ruta se traza una vez al levantarse el pueblo, se guarda y entra en la red de caminos), vuelca en el almacén de la madre (y anota lo que cupo en la ventana de 24 h de la solvencia del ascenso) y vuelve vacío; si al volver
//!    el almacén está lleno otra vez, sale de nuevo.
//! 4. **Sin carro**: si el carro se pierde (batalla, destrucción del pueblo), la madre lo repone sola en cuanto puede pagarlo; mientras, el pueblo acumula hasta llenar su almacén y la extracción para
//!    (`ReponerCarro` lo compra en el instante).
//!
//! # Lo provisional
//! Marcar, destruir y reponer el carro son obras de mapa del panel de la plaza madre (Doc 4.2.2): las da el Gobernador o el Maestro de Obras de la madre, residente y dentro (5E). La
//! entrega de materiales la hace cualquier participante de la columna que sea ciudadano de la Facción; la guarnición de mineros y el saqueo son del sistema militar.

use super::*;
use crate::nucleo::mercado::id32;
use asentamiento::catalogo::Tipo;
use asentamiento::pueblos::{ErrorPueblo, Pueblo};
use asentamiento::Costo;
use super::dentro::MOTIVO_DESTRUIDO;
use comercio::acarreo;
use super::viaje::Trazado;
use comercio::viaje::{longitud_ruta, peajes_de_paradas};
use comercio::{total, Carga as Mercancia};
use militar::presencia::Ubicacion;

fn rechazo_de_pueblo(e: ErrorPueblo) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorPueblo::TipoNoPueblo => (PUEBLO_NODO_NO_EXTRAIBLE, "ese nodo no se explota con un pueblo extractor"),
        ErrorPueblo::TecnologiaFaltante => (PUEBLO_TECNOLOGIA_FALTANTE, "la Facción no tiene la tecnología que pide el extractor"),
        ErrorPueblo::NivelInsuficiente => (PUEBLO_NIVEL_INSUFICIENTE, "la plaza madre no tiene el nivel que pide ese pueblo"),
        ErrorPueblo::SinCupo => (PUEBLO_SIN_CUPO, "la plaza madre ya tiene todos los pueblos que su nivel permite"),
        ErrorPueblo::NoExiste => (PUEBLO_DESCONOCIDO, "no existe ese pueblo"),
        ErrorPueblo::NoEstaPendiente => (PUEBLO_NO_ESTA_PENDIENTE, "la obra no está pendiente de materiales"),
        ErrorPueblo::NadaQueEntregar => (PUEBLO_NADA_QUE_ENTREGAR, "la columna no lleva nada que la obra necesite"),
        ErrorPueblo::FertilidadInsuficiente => (PUEBLO_FERTILIDAD_INSUFICIENTE, "la tierra de ese punto no es bastante fértil para una aldea agrícola"),
        ErrorPueblo::NoEsAldea => (PUEBLO_NO_ES_ALDEA, "solo las aldeas agrícolas suben de nivel"),
        ErrorPueblo::NivelMaximo => (PUEBLO_NIVEL_MAXIMO, "la aldea ya está en su último nivel"),
        ErrorPueblo::MejoraEnCurso => (PUEBLO_MEJORA_EN_CURSO, "la aldea ya tiene una subida de nivel en marcha"),
        ErrorPueblo::NoEstaActivo => (PUEBLO_NO_ESTA_ACTIVO, "el pueblo aún no está levantado"),
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
        crate::economia::posicion_de_pueblo(&self.geografia.as_ref()?.mapa.nodos, p)
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
        let alcanzable = |t: Tipo| catalogo.tipo(t).tecnologia.as_deref().is_none_or(|tec| self.tiene_tecnologia(a.faccion, tec));
        nodos.into_iter().filter(libre).map(|n| (n, geo.mapa.nodos[n as usize].tipo)).filter(|&(_, t)| alcanzable(t)).collect()
    }

    /// Lo que `MarcarAldea` mira de un punto antes de la ruta: tierra firme, en la zona real de la plaza y a `separacionMinima` de toda plaza y todo
    /// pueblo, de cualquier Facción. Devuelve la fertilidad del punto (ppm); el mínimo lo mira la regla (`iniciar_aldea`).
    fn sitio_de_aldea_sin_ruta(&self, plaza: u64, pos: Punto) -> Result<i64, (&'static str, String)> {
        let geo = self.geografia.as_ref().ok_or((rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado".into()))?;
        let Some(fertilidad) = geo.fertilidad_en(pos).filter(|_| geo.es_tierra_firme(pos)) else { return Err((rechazos::PUEBLO_FUERA_DE_LA_ZONA, "ese punto no es tierra firme".into())) };
        if !self.en_zona(plaza, pos) {
            return Err((rechazos::PUEBLO_FUERA_DE_LA_ZONA, format!("el punto cae fuera de la zona de as-{plaza}")));
        }
        let separacion = self.estado.reglas.economia.datos.pueblos.aldea.separacion_minima;
        let nodos = &geo.mapa.nodos;
        let cerca = self.estado.asentamientos.values().any(|a| {
            a.posicion.dist(pos) < separacion || a.economia.pueblos.iter().filter_map(|x| crate::economia::posicion_de_pueblo(nodos, x)).any(|q| q.dist(pos) < separacion)
        });
        if cerca {
            return Err((rechazos::PUEBLO_DEMASIADO_CERCA, "el punto está demasiado cerca de otro pueblo o de una plaza".into()));
        }
        Ok(fertilidad)
    }

    /// El panel de la aldea agrícola: el punto más fértil de la zona de la plaza donde `MarcarAldea` se aceptaría ahora, muestreado en anillos de
    /// 0,5 u y 24 rumbos desde la separación mínima hasta el radio de la zona. `None` si no hay ninguno (o no hay mundo).
    pub fn sitio_de_aldea(&mut self, plaza: u64) -> Option<Punto> {
        let a = self.estado.asentamientos.get(&plaza)?;
        let (centro, radio) = (a.posicion, a.radio);
        let (separacion, minima) = { let r = &self.estado.reglas.economia.datos.pueblos.aldea; (r.separacion_minima, r.fertilidad_minima) };
        let mut sitios: Vec<(i64, Punto)> = (0..)
            .map(|k| separacion + 500 + k * 500)
            .take_while(|&d| d <= radio)
            .flat_map(|d| (0..24).map(move |g| (d, g)))
            .filter_map(|(d, g)| {
                let ang = ((g * 15) as f64).to_radians();
                let p = Punto::new(centro.x + (d as f64 * ang.cos()) as i64, centro.y + (d as f64 * ang.sin()) as i64);
                self.sitio_de_aldea_sin_ruta(plaza, p).ok().filter(|&f| f >= minima).map(|f| (f, p))
            })
            .collect();
        // El más fértil primero (a igualdad, el más cercano); la ruta, un A*, solo se busca para los que se prueban.
        sitios.sort_by_key(|&(f, _)| std::cmp::Reverse(f));
        sitios.into_iter().map(|(_, p)| p).find(|&p| self.camino_sin_red(p, centro).is_some())
    }

    /// La planta del pueblo (presentación pura, T / Campamentos_Mercenarios §7): una función del id del pueblo, que no se guarda.
    pub fn planta_de_pueblo(&self, id: u64) -> Option<crate::urbanismo::VistaCiudad> {
        let (_, p) = self.pueblo(id)?;
        let r = &self.estado.reglas.economia;
        let aldea = p.es_aldea().then(|| r.datos.pueblos.aldea.niveles.get(&p.nivel).map_or((1, 1), |n| (n.granjas, n.corrales)));
        Some(crate::urbanismo::planta_de_pueblo(r, self.semilla, id, p.tipo, aldea))
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
                self.plaza_o_rechazo(*asentamiento)?;
                self.autorizar_en_plaza(p, *asentamiento, None)?;
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
                // Sin ruta por tierra a la madre el carro no podría acarrear nunca: no se funda (decisión del autor, 2026-10-23).
                let madre = self.estado.asentamientos[asentamiento].posicion;
                if self.camino_sin_red(pos, madre).is_none() {
                    return Err(rechazo(rechazos::PUEBLO_SIN_RUTA, format!("el nodo {nodo} no tiene ruta por tierra hasta as-{asentamiento}")));
                }
                let carro = acarreo::coste_reposicion(&self.estado.reglas.economia.comercio.flota);
                let id = self.estado.siguiente_id;
                let mut ev = vec![];
                let faccion = self.estado.asentamientos[asentamiento].faccion;
                let (rt, adoptadas) = (self.estado.reglas.economia.tecnologia.clone(), self.estado.tecnologia.de(&self.estado.reglas.economia.tecnologia, faccion).adoptadas);
                let eco = &mut self.estado.asentamientos.get_mut(asentamiento).expect("existe").economia;
                eco.iniciar_pueblo(&datos, &|t: &str| crate::estado::adopta(&rt, adoptadas, t), id, tipo, *nodo, &carro, &mut ev).map_err(de_pueblo)?;
                self.estado.siguiente_id += 1;
                self.estado.nodos_reclamados.insert(*nodo, *asentamiento);
                self.anotar_sucesos(*asentamiento, t, &ev);
                Ok(Some(id))
            }
            Comando::MarcarAldea { asentamiento, punto } => {
                self.plaza_o_rechazo(*asentamiento)?;
                self.autorizar_en_plaza(p, *asentamiento, None)?;
                let pos = *punto;
                let fertilidad = self.sitio_de_aldea_sin_ruta(*asentamiento, pos).map_err(|(c, m)| rechazo(c, m))?;
                let madre = self.estado.asentamientos[asentamiento].posicion;
                if self.camino_sin_red(pos, madre).is_none() {
                    return Err(rechazo(rechazos::PUEBLO_SIN_RUTA, format!("el punto no tiene ruta por tierra hasta as-{asentamiento}")));
                }
                let carro = acarreo::coste_reposicion(&self.estado.reglas.economia.comercio.flota);
                let id = self.estado.siguiente_id;
                let mut ev = vec![];
                let eco = &mut self.estado.asentamientos.get_mut(asentamiento).expect("existe").economia;
                eco.iniciar_aldea(&datos, id, pos, fertilidad, &carro, &mut ev).map_err(de_pueblo)?;
                self.estado.siguiente_id += 1;
                self.anotar_sucesos(*asentamiento, t, &ev);
                Ok(Some(id))
            }
            Comando::MejorarAldea { pueblo } => {
                let (plaza, _) = self.pueblo(*pueblo).ok_or_else(|| de_pueblo(ErrorPueblo::NoExiste))?;
                self.autorizar_en_plaza(p, plaza, None)?;
                let mut ev = vec![];
                self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.mejorar_aldea(&datos, *pueblo, &mut ev).map_err(de_pueblo)?;
                self.anotar_sucesos(plaza, t, &ev);
                Ok(None)
            }
            Comando::EntregarMateriales { ejercito, pueblo } => {
                let e = self.estado.ejercitos.get(ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*ejercito)))?;
                if !self.manda_columna(p, e, false) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
                }
                let (plaza, pu) = self.pueblo(*pueblo).ok_or_else(|| de_pueblo(ErrorPueblo::NoExiste))?;
                let faccion = self.estado.asentamientos[&plaza].faccion;
                if e.columna.faccion != Some(faccion) {
                    return Err(rechazo(rechazos::PUEBLO_FACCION_AJENA, "la obra es de otra Facción".into()));
                }
                if !self.entrega_un_ciudadano(p, e, faccion) {
                    return Err(rechazo(rechazos::PUEBLO_FACCION_AJENA, "solo entregan los ciudadanos de la Facción de la obra".into()));
                }
                let sitio = self.posicion_de_pueblo(pu.id).expect("un pueblo está sobre un nodo del mapa");
                if e.posicion_en(t).dist(sitio) > self.radio_puerta() {
                    return Err(rechazo(rechazos::PUEBLO_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la obra".into()));
                }
                let disponible: Costo = self.material_de_columna(*ejercito).into_iter().collect();
                let mut ev = vec![];
                let eco = &mut self.estado.asentamientos.get_mut(&plaza).expect("existe").economia;
                let tomado = eco.entregar_a_pueblo(t, &datos, *pueblo, &disponible, &mut ev).map_err(de_pueblo)?;
                self.quitar_de_columna(*ejercito, &tomado);
                let lider = self.estado.ejercitos[ejercito].columna.lider;
                self.avance(t, Dueno::Heroe(lider), "entregas", 1);
                self.tr.tocar(Clase::Ejercito, *ejercito);
                self.anotar_sucesos(plaza, t, &ev);
                Ok(None)
            }
            Comando::DestruirPueblo { pueblo } => {
                let (plaza, _) = self.pueblo(*pueblo).ok_or_else(|| de_pueblo(ErrorPueblo::NoExiste))?;
                self.autorizar_en_plaza(p, plaza, None)?;
                let mut ev = vec![];
                // Quien estaba dentro sale al mapa con lo suyo, a la puerta del pueblo que desaparece.
                if let Some(puerta) = self.posicion_de_pueblo(*pueblo) {
                    self.vaciar_lugar(t, Ubicacion::Pueblo(*pueblo), puerta, plaza, MOTIVO_DESTRUIDO);
                }
                let destruido = self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.quitar_pueblo(*pueblo, &mut ev).expect("existe");
                if let Some(n) = destruido.nodo() {
                    self.estado.nodos_reclamados.remove(&n);
                }
                self.olvidar_ruta_de_acarreo(*pueblo);
                if let Some(carro) = destruido.carro {
                    self.perder_caravana(t, carro);
                }
                self.anotar_sucesos(plaza, t, &ev);
                Ok(None)
            }
            Comando::ReponerCarro { pueblo } => {
                let (plaza, pu) = self.pueblo(*pueblo).ok_or_else(|| de_pueblo(ErrorPueblo::NoExiste))?;
                self.autorizar_en_plaza(p, plaza, None)?;
                if !pu.activo() {
                    return Err(rechazo(rechazos::PUEBLO_NO_ESTA_ACTIVO, "el pueblo aún no está levantado".into()));
                }
                if pu.carro.is_some() {
                    return Err(rechazo(rechazos::PUEBLO_YA_TIENE_CARRO, "el pueblo ya tiene carro: solo se repone si no tiene ninguno".into()));
                }
                self.reponer_carro(plaza, *pueblo, t).map_err(|e| {
                    let (c, m) = flota::rechazo_de_caravana(e);
                    rechazo(c, m.into())
                })?;
                Ok(None)
            }
            _ => unreachable!("solo se llama con órdenes de pueblos"),
        }
    }

    /// Lo que lleva una columna y vale como material: su carro (`suministro`) y la carga de sus caravanas adjuntas, sumado por recurso.
    pub(super) fn material_de_columna(&self, ejercito: u64) -> Mercancia {
        let e = &self.estado.ejercitos[&ejercito];
        let mut todo = e.columna.carga.clone();
        for c in e.caravanas_adjuntas.iter().filter_map(|&c| self.estado.comercio.caravana(c)) {
            for (&r, &n) in &c.contenido {
                *todo.entry(r).or_insert(0) += n;
            }
        }
        todo
    }

    /// Quita de la columna lo que entregó: primero de su carro y, si no basta, de sus caravanas adjuntas por orden de id.
    pub(super) fn quitar_de_columna(&mut self, ejercito: u64, tomado: &Costo) {
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
            quitar(&mut e.columna.carga, &mut falta);
            for &c in &e.caravanas_adjuntas {
                if falta > 0 {
                    if let Some(i) = comercio.posicion_caravana(c) {
                        let antes = falta;
                        quitar(&mut comercio.flota[i].contenido, &mut falta);
                        if falta < antes {
                            self.tr.tocar(Clase::Caravana, c);
                        }
                    }
                }
            }
        }
    }

    /// El saqueo ganado de una aldea agrícola se lleva también la parte `saqueoRebano` de su rebaño (§14.3), sacrificada, en los carros del atacante y
    /// hasta donde quepa (las cabezas que no caben se quedan en la aldea). Devuelve lo llevado. Nada en un extractor.
    pub(super) fn saquear_rebano(&mut self, madre: u64, pueblo: u64, carros: &mut [combate::botin::CarroDeColumna]) -> Mercancia {
        let libre: Mil = carros.iter().map(|c| (c.capacidad - total(&c.carga)).max(0)).sum();
        let Estado { asentamientos, reglas, .. } = &mut self.estado;
        let Some(a) = asentamientos.get_mut(&madre) else { return Mercancia::new() };
        let cabe = |productos: &Costo| {
            let por_cabeza: Mil = productos.iter().map(|&(_, n)| n).sum();
            if por_cabeza > 0 { libre / por_cabeza } else { 0 }
        };
        let (cabezas, botin) = a.economia.saquear_rebano(&reglas.economia.datos, pueblo, cabe);
        if cabezas == 0 {
            return Mercancia::new();
        }
        self.tr.tocar(Clase::Pueblo, pueblo);
        let botin: Mercancia = botin.into_iter().collect();
        combate::botin::repartir_botin(carros, &botin);
        botin
    }

    // ───────────────────────────── El carro ─────────────────────────────

    /// Anota el carro `carro` en el pueblo (el pueblo lo conoce por su id de la flota).
    fn enlazar_carro(&mut self, plaza: u64, pueblo: u64, carro: u64) {
        if let Some(pu) = self.estado.asentamientos.get_mut(&plaza).and_then(|a| a.economia.pueblos.iter_mut().find(|p| p.id == pueblo)) {
            pu.carro = Some(carro);
            self.tr.tocar(Clase::Pueblo, pueblo);
        }
    }

    /// El pueblo acaba de levantarse: nace su carro (el coste ya iba en la obra), parado en el pueblo.
    pub(super) fn dar_carro_a_pueblo(&mut self, plaza: u64, pueblo: u64, t: Instante) {
        let Some(posicion) = self.posicion_de_pueblo(pueblo) else { return };
        // La ruta pueblo → madre se traza una sola vez, al levantarse, y entra en la red de caminos.
        if let Some(a) = self.estado.asentamientos.get(&plaza) {
            let (pos_madre, faccion) = (a.posicion, a.faccion);
            self.ruta_de_acarreo(plaza, pueblo, posicion, pos_madre, faccion, t);
        }
        let id = self.estado.comercio.siguiente_caravana;
        self.estado.comercio.siguiente_caravana += 1;
        self.estado.comercio.flota.push(acarreo::nuevo_carro(id, id32(plaza), posicion));
        self.tr.tocar(Clase::Caravana, id);
        self.enlazar_carro(plaza, pueblo, id);
    }

    /// Compra en la madre un carro nuevo (carro + buey) para el pueblo, que no tiene ninguno, y lo deja en él; sale en el acto si el almacén está lleno.
    fn reponer_carro(&mut self, plaza: u64, pueblo: u64, t: Instante) -> Result<(), comercio::caravana::ErrorCaravana> {
        let (id, posicion) = (self.estado.comercio.siguiente_caravana, self.posicion_de_pueblo(pueblo).expect("sobre un nodo"));
        let flota = &self.estado.reglas.economia.comercio.flota;
        let almacen = &mut self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.almacen;
        let carro = acarreo::reponer(flota, almacen, id, id32(plaza), posicion)?;
        self.estado.comercio.siguiente_caravana += 1;
        self.estado.comercio.flota.push(carro);
        self.tr.tocar(Clase::Caravana, id);
        self.tr.tocar(Clase::Asentamiento, plaza);
        self.enlazar_carro(plaza, pueblo, id);
        self.emitir_propio(t, "acarreo.carro_repuesto", vec![plaza], vec![("pueblo", pueblo as i64), ("caravana", id as i64)]);
        self.salir_acarreo(plaza, pueblo, t);
        Ok(())
    }

    /// La reposición automática (decisión del autor del 2026-10-27): en cada paso de minuto de la madre, cada pueblo activo sin carro compra uno en
    /// cuanto la madre puede pagarlo. Mientras haya una batalla sobre el pueblo o sobre la madre, espera.
    pub(super) fn reponer_carros(&mut self, plaza: u64, t: Instante) {
        let Some(a) = self.estado.asentamientos.get(&plaza) else { return };
        let sin_carro: Vec<u64> = a.economia.pueblos.iter().filter(|p| p.activo() && p.carro.is_none()).map(|p| p.id).collect();
        if sin_carro.is_empty() || self.batalla_sobre(Ubicacion::Asentamiento(id32(plaza))).is_some() {
            return;
        }
        for pueblo in sin_carro {
            if self.batalla_sobre(Ubicacion::Pueblo(pueblo)).is_none() {
                let _ = self.reponer_carro(plaza, pueblo, t);
            }
        }
    }

    /// El carro `carro` de la plaza `plaza` desapareció (destruido, perdido en una batalla...): su pueblo se queda sin él.
    pub(super) fn pueblo_sin_carro(&mut self, plaza: u64, carro: u64) {
        let Some(a) = self.estado.asentamientos.get_mut(&plaza) else { return };
        for p in a.economia.pueblos.iter_mut().filter(|p| p.carro == Some(carro)) {
            p.carro = None;
            self.tr.tocar(Clase::Pueblo, p.id);
        }
        self.tr.tocar(Clase::Asentamiento, plaza);
    }

    /// El carro del pueblo sale hacia la madre **si está en casa y el almacén local está lleno** (nada recurre por pueblo ocioso: se llama cuando
    /// el almacén se llena, cuando el carro vuelve y cuando se repone). Lleva la capacidad entera, por la ruta fija pueblo → madre (fundarlo ya exige
    /// que la haya: `pueblo.sin_ruta`).
    pub(super) fn salir_acarreo(&mut self, plaza: u64, pueblo: u64, t: Instante) {
        let Some(a) = self.estado.asentamientos.get(&plaza) else { return };
        let Some(pu) = a.economia.pueblo(pueblo) else { return };
        let Some(carro) = pu.carro else { return };
        let flota = self.estado.reglas.economia.comercio.flota.clone();
        let datos = &self.estado.reglas.economia.datos;
        let Some(c) = self.estado.comercio.caravana(carro) else { return };
        if !acarreo::debe_salir(c, &flota, pu.existencias(datos)) {
            return;
        }
        // Lo que se lleva: la capacidad entera, de su recurso o, en la aldea, del trigo y el ganado que tenga (por ese orden).
        let carga: Mercancia = pu.carga_del_carro(datos, acarreo::capacidad(&flota)).into_iter().collect();
        let (pos_madre, faccion) = (a.posicion, a.faccion);
        let Some(pos_pueblo) = self.posicion_de_pueblo(pueblo) else { return };
        let Some(tz) = self.ruta_de_acarreo(plaza, pueblo, pos_pueblo, pos_madre, faccion, t) else { return };
        let cantidad = total(&carga);
        let local = &mut self.estado.asentamientos.get_mut(&plaza).and_then(|a| a.economia.pueblos.iter_mut().find(|p| p.id == pueblo)).expect("existe").almacen;
        for (&r, &n) in &carga {
            local.quitar(r, n);
        }
        self.tr.tocar(Clase::Pueblo, pueblo);
        let i = self.estado.comercio.posicion_caravana(carro).expect("existe");
        let plan = self.plan_de(&self.estado.comercio.flota[i], &tz.puntos, &tz.costes, t);
        let peajes = peajes_de_paradas(&tz.paradas, longitud_ruta(&tz.puntos));
        let puntos = tz.puntos.clone();
        acarreo::salir(&mut self.estado.comercio.flota[i], id32(plaza), carga, tz.puntos, tz.costes, peajes, plan.clone()).expect("el carro está en casa");
        self.instalar_plan(carro, plan);
        // Cada salida renueva la ruta en la red de caminos (como el lanzamiento de una ruta comercial: no caduca mientras el carro la recorre).
        self.registrar_ruta_en_red(plaza, pueblo, &puntos, t, &[carro]);
        self.tr.tocar(Clase::Asentamiento, plaza);
        self.emitir_propio(t, "acarreo.sale", vec![plaza], vec![("pueblo", pueblo as i64), ("caravana", carro as i64), ("carga", cantidad)]);
    }

    /// La ruta del carro del pueblo hasta su madre: la guardada si sigue valiendo y, si no (no la hay, la madre cambió de Facción, una ciudad de
    /// sus peajes ya no existe o un punto dejó de ser transitable), una nueva, que se guarda y se añade a la red de caminos.
    pub(super) fn ruta_de_acarreo(&mut self, plaza: u64, pueblo: u64, pos_pueblo: Punto, pos_madre: Punto, faccion: u32, t: Instante) -> Option<Trazado> {
        if let Some(r) = self.estado.comercio.rutas_acarreo.get(&pueblo) {
            let vale = r.faccion == faccion
                && r.trazado.paradas.iter().all(|&(c, _)| self.estado.asentamientos.contains_key(&u64::from(c)))
                && self.geografia.as_ref().is_none_or(|g| r.trazado.puntos.iter().all(|&p| g.transitable(p)));
            if vale {
                return Some(r.trazado.clone());
            }
        }
        let tz = self.trazar_ruta(pueblo, plaza, pos_pueblo, pos_madre, faccion, t)?;
        self.estado.comercio.rutas_acarreo.insert(pueblo, crate::estado_comercio::RutaAcarreo { faccion, trazado: tz.clone() });
        self.registrar_ruta_en_red(plaza, pueblo, &tz.puntos, t, &[]);
        Some(tz)
    }

    /// El pueblo `pueblo` desaparece (destruido o con su madre en ruinas): su ruta guardada, también.
    pub(super) fn olvidar_ruta_de_acarreo(&mut self, pueblo: u64) {
        if self.estado.comercio.rutas_acarreo.remove(&pueblo).is_some() {
            self.invalidar_red();
        }
    }

    /// El carro llegó a la madre: vuelca lo que trae en su almacén (topado: lo que no cabe se pierde, ni cuenta para el ascenso) y vuelve vacío (su plan ya es el de la vuelta).
    pub(super) fn acarreo_llega(&mut self, t: Instante, carro: u64, madre: u64, contenido: &Mercancia) {
        let Some(plaza) = self.estado.asentamientos.get_mut(&madre) else { return drop(self.perder_caravana(t, carro)) };
        let mut ingresado: Mil = 0;
        for (&r, &n) in contenido {
            let entra = plaza.economia.almacen.agregar(r, n);
            plaza.economia.registrar_acarreo(t, r, entra);
            ingresado += entra;
        }
        self.tr.tocar(Clase::Asentamiento, madre);
        self.emitir_propio(t, "acarreo.llega", vec![madre], vec![("caravana", carro as i64), ("carga", ingresado), ("perdido", total(contenido) - ingresado)]);
    }

    /// El carro volvió al pueblo: queda listo y, si el almacén ya está lleno otra vez, sale de nuevo.
    pub(super) fn acarreo_en_casa(&mut self, t: Instante, madre: u64, carro: u64) {
        let Some(pueblo) = self.estado.asentamientos.get(&madre).and_then(|a| a.economia.pueblos.iter().find(|p| p.carro == Some(carro))).map(|p| p.id) else { return };
        self.emitir_propio(t, "acarreo.vuelve", vec![madre], vec![("pueblo", pueblo as i64), ("caravana", carro as i64)]);
        self.salir_acarreo(madre, pueblo, t);
    }
}
