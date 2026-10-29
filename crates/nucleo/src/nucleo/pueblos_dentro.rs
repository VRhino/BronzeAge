//! Lo que se hace dentro de un pueblo extractor o de una aldea agrícola (Economia_Construccion §14.0.1; decisiones del autor del 2026-10-29): la guarnición de jugadores, el carro
//! de la columna frente al almacén local y, en `heroes` y `pueblos`, la escolta del carro de acarreo y la mejora de la aldea dadas desde dentro.
//!
//! # Quién y con qué
//! Todo lo hace un héroe **dentro** del pueblo (`entrarEnPueblo`) y **ciudadano de la Facción de la plaza madre**; nadie reside en un pueblo, así que lo que mueve es la columna con la que
//! entró, que lo espera dentro ([`Nucleo::columna_que_lleva`]). La administración no necesita estar dentro ni ser de la Facción, pero sí que el héroe traiga columna.
//!
//! # La guarnición de jugadores
//! Además de la guarnición de serie (mineros o granjeros sin dueño), cada héroe deja escuadras de su columna (`Contenedor::Pueblo`) con un cupo **por héroe** en Liderazgo:
//! `PUEBLOS_EXTRACTORES.guarnicionJugadores.cupo` (30) en nivel 1 y `porNivel` (15) más por cada nivel de la aldea (un extractor es siempre de nivel 1). Comen como su campamento (de la plaza
//! donde reside su héroe) y defienden el pueblo en el saqueo, con o sin su héroe. Vuelven al campamento de su héroe, intactas, si el pueblo desaparece o el héroe deja de ser de la
//! Facción de la madre (la deja, o la madre cambia de dueño).
//!
//! # El almacén local
//! Se carga en el carro de la columna lo que haya, hasta donde quepa; se deja solo lo que el pueblo produce (su recurso; en la aldea, trigo y ganado), hasta su capacidad. Si al dejar se
//! llena, sale el carro de acarreo si está en casa.

use super::*;
use asentamiento::Recurso;
use comercio::total;
use militar::escuadra::{Contenedor, Escuadra};
use militar::liderazgo;
use militar::presencia::Ubicacion;

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Las escuadras de jugadores que guarnecen el pueblo, por héroe y en el orden de cada uno.
    pub fn guarnicion_de_jugadores_de_pueblo(&self, id: u64) -> Vec<&Escuadra> {
        self.heroes().flat_map(|h| h.heroe.escuadras.iter()).filter(|e| e.contenedor == Contenedor::Pueblo(id)).collect()
    }

    /// El cupo de la guarnición de jugadores del pueblo por héroe, en puntos de Liderazgo. `None` si el pueblo no existe.
    pub fn cupo_de_guarnicion_de_pueblo(&self, id: u64) -> Option<i64> {
        let (_, p) = self.pueblo(id)?;
        Some(self.estado.reglas.economia.datos.pueblos.cupo_de_guarnicion(p.nivel))
    }

    // ───────────────────────────── Las órdenes ─────────────────────────────

    /// `guarnecerPueblo`, `retirarDePueblo`, `cargarDePueblo` y `dejarEnPueblo`.
    pub(super) fn ejecutar_dentro_de_pueblo(&mut self, t: Instante, p: &Peticion) -> Result<(), Respuesta> {
        match &p.comando {
            Comando::GuarnecerPueblo { heroe, pueblo, escuadras } => self.guarnecer_pueblo(t, p, *heroe, *pueblo, escuadras),
            Comando::RetirarDePueblo { heroe, pueblo, escuadras } => self.retirar_de_pueblo(t, p, *heroe, *pueblo, escuadras),
            Comando::CargarDePueblo { heroe, pueblo, recurso, cantidad } => self.cargar_de_pueblo(t, p, *heroe, *pueblo, *recurso, *cantidad),
            Comando::DejarEnPueblo { heroe, pueblo, recurso, cantidad } => self.dejar_en_pueblo(t, p, *heroe, *pueblo, *recurso, *cantidad),
            _ => unreachable!("solo se llama con órdenes de dentro de un pueblo"),
        }
    }

    /// Comprueba que `heroe` (de la cuenta) está dentro del pueblo activo `pueblo`, es ciudadano de la Facción de su madre y trae columna. Devuelve la madre y la columna.
    pub(super) fn dentro_del_pueblo(&self, p: &Peticion, heroe: u64, pueblo: u64) -> Result<(u64, u64), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let (madre, pu) = self.pueblo(pueblo).ok_or_else(|| rechazo(rechazos::PUEBLO_DESCONOCIDO, ids::pueblo(pueblo)))?;
        if !pu.activo() {
            return Err(rechazo(rechazos::PUEBLO_NO_ESTA_ACTIVO, "el pueblo aún es una obra".into()));
        }
        self.autorizar_heroe(p, heroe, false)?;
        let h = self.heroe(heroe).expect("existe");
        if !p.admin {
            if h.heroe.presencia.ubicacion != Ubicacion::Pueblo(pueblo) {
                return Err(rechazo(rechazos::HEROE_NO_ESTA_EN_EL_PUEBLO, format!("el héroe no está dentro de {}", ids::pueblo(pueblo))));
            }
            if h.faccion != Some(self.estado.asentamientos[&madre].faccion) {
                return Err(rechazo(rechazos::PUEBLO_FACCION_AJENA, "solo los ciudadanos de la Facción de la plaza madre".into()));
            }
        }
        let columna = self.columna_que_lleva(heroe).ok_or_else(|| rechazo(rechazos::HEROE_SIN_COLUMNA, "el héroe no trajo columna".into()))?;
        Ok((madre, columna))
    }

    /// Deja escuadras de la columna en la guarnición del pueblo, dentro del cupo del héroe.
    fn guarnecer_pueblo(&mut self, t: Instante, p: &Peticion, heroe: u64, pueblo: u64, escuadras: &[u64]) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let (madre, cid) = self.dentro_del_pueblo(p, heroe, pueblo)?;
        if escuadras.is_empty() {
            return Err(rechazo(rechazos::SALIDA_SIN_ESCUADRAS, "hay que dejar al menos una escuadra".into()));
        }
        let h = &self.heroe(heroe).expect("existe").heroe;
        let mut vistas = std::collections::BTreeSet::new();
        for &id in escuadras {
            let s = h.escuadra(id).ok_or_else(|| rechazo(rechazos::ESCUADRA_AJENA, format!("la escuadra {id} no es tuya")))?;
            if !vistas.insert(id) {
                return Err(rechazo(rechazos::ESCUADRA_REPETIDA, format!("la escuadra {id} se repite")));
            }
            if s.contenedor != Contenedor::Ejercito(cid) {
                return Err(rechazo(rechazos::PUEBLO_ESCUADRA_FUERA_DE_TU_COLUMNA, format!("la escuadra {id} no va en tu columna")));
            }
            if s.cantidad <= 0 {
                return Err(rechazo(rechazos::ESCUADRA_ANIQUILADA, format!("la escuadra {id} está aniquilada")));
            }
            if s.reserva_batalla.is_some() {
                return Err(rechazo(rechazos::ESCUADRA_EN_BATALLA, format!("la escuadra {id} está reservada para una batalla")));
            }
        }
        let mr = self.estado.reglas.economia.militar.clone();
        let cupo = self.cupo_de_guarnicion_de_pueblo(pueblo).expect("existe");
        let ocupado = liderazgo::comprometido(&mr, h.escuadras.iter().filter(|s| s.contenedor == Contenedor::Pueblo(pueblo) || vistas.contains(&s.id)));
        if ocupado > cupo {
            return Err(rechazo(rechazos::GUARNICION_SUPERA_EL_CUPO, format!("la guarnición pasaría a {ocupado} de un cupo de {cupo}")));
        }
        let antes = self.velocidad_de_ejercito(cid).unwrap_or(0);
        self.estado.ejercitos.get_mut(&cid).expect("existe").columna.escuadras.retain(|s| !vistas.contains(s));
        let ids: Vec<u64> = vistas.into_iter().collect();
        self.poner_escuadras_en(&[heroe], &ids, Contenedor::Pueblo(pueblo), t);
        self.recomponer_columna(cid, t, antes);
        self.tr.tocar(Clase::Ejercito, cid);
        self.tr.tocar(Clase::Pueblo, pueblo);
        self.emitir_propio(t, "pueblo.guarnecido", vec![madre], vec![("pueblo", pueblo as i64), ("heroe", heroe as i64), ("escuadras", ids.len() as i64), ("liderazgo", ocupado)]);
        Ok(())
    }

    /// Recoge escuadras suyas de la guarnición del pueblo a la columna con la que entró, sin pasar de su Liderazgo.
    fn retirar_de_pueblo(&mut self, t: Instante, p: &Peticion, heroe: u64, pueblo: u64, escuadras: &[u64]) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let (madre, cid) = self.dentro_del_pueblo(p, heroe, pueblo)?;
        if escuadras.is_empty() {
            return Err(rechazo(rechazos::SALIDA_SIN_ESCUADRAS, "hay que recoger al menos una escuadra".into()));
        }
        let mr = self.estado.reglas.economia.militar.clone();
        let h = &self.heroe(heroe).expect("existe").heroe;
        let mut vistas = std::collections::BTreeSet::new();
        for &id in escuadras {
            let s = h.escuadra(id).ok_or_else(|| rechazo(rechazos::ESCUADRA_AJENA, format!("la escuadra {id} no es tuya")))?;
            if !vistas.insert(id) {
                return Err(rechazo(rechazos::ESCUADRA_REPETIDA, format!("la escuadra {id} se repite")));
            }
            if s.contenedor != Contenedor::Pueblo(pueblo) {
                return Err(rechazo(rechazos::PUEBLO_ESCUADRA_NO_ESTA, format!("la escuadra {id} no está en ese pueblo")));
            }
            if s.reserva_batalla.is_some() {
                return Err(rechazo(rechazos::ESCUADRA_EN_BATALLA, format!("la escuadra {id} está reservada para una batalla")));
            }
        }
        let fuera = h.escuadras.iter().filter(|s| s.contenedor != Contenedor::Campamento && s.contenedor != Contenedor::Pueblo(pueblo) || vistas.contains(&s.id));
        if !liderazgo::puede_llevar(&mr, h.liderazgo_base, fuera) {
            return Err(rechazo(rechazos::LIDERAZGO_SUPERADO, "las escuadras pasan del Liderazgo del héroe".into()));
        }
        let antes = self.velocidad_de_ejercito(cid).unwrap_or(0);
        let ids: Vec<u64> = vistas.into_iter().collect();
        self.estado.ejercitos.get_mut(&cid).expect("existe").columna.escuadras.extend(&ids);
        self.poner_escuadras_en(&[heroe], &ids, Contenedor::Ejercito(cid), t);
        self.sincronizar_hambre(cid, t);
        self.recomponer_columna(cid, t, antes);
        self.tr.tocar(Clase::Ejercito, cid);
        self.tr.tocar(Clase::Pueblo, pueblo);
        self.emitir_propio(t, "pueblo.guarnicion_retirada", vec![madre], vec![("pueblo", pueblo as i64), ("heroe", heroe as i64), ("escuadras", ids.len() as i64)]);
        Ok(())
    }

    /// Carga en el carro de la columna `cantidad` de `recurso` del almacén local, hasta donde quepa.
    fn cargar_de_pueblo(&mut self, t: Instante, p: &Peticion, heroe: u64, pueblo: u64, recurso: Recurso, cantidad: Mil) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let (madre, cid) = self.dentro_del_pueblo(p, heroe, pueblo)?;
        if cantidad <= 0 {
            return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "hay que cargar una cantidad positiva".into()));
        }
        let hay = self.pueblo(pueblo).expect("existe").1.almacen.cantidad(recurso);
        if hay <= 0 {
            return Err(rechazo(rechazos::PUEBLO_NADA_QUE_CARGAR, format!("el almacén del pueblo no tiene {}", recurso.nombre())));
        }
        let libre = (self.capacidad_de_carga(cid).unwrap_or(0) - total(&self.estado.ejercitos[&cid].columna.carga)).max(0);
        if libre == 0 {
            return Err(rechazo(rechazos::CARRO_LLENO, "el carro de tu columna está lleno".into()));
        }
        let n = cantidad.min(hay).min(libre);
        self.almacen_de_pueblo(madre, pueblo).quitar(recurso, n);
        *self.estado.ejercitos.get_mut(&cid).expect("existe").columna.carga.entry(recurso).or_insert(0) += n;
        self.tr.tocar(Clase::Ejercito, cid);
        self.tr.tocar(Clase::Pueblo, pueblo);
        self.tr.tocar(Clase::Asentamiento, madre);
        self.emitir_propio(t, "pueblo.cargado", vec![madre], vec![("pueblo", pueblo as i64), ("heroe", heroe as i64), ("recurso", crate::codec_economia::recurso_a_u8(recurso) as i64), ("cantidad", n)]);
        Ok(())
    }

    /// Deja en el almacén local `cantidad` de `recurso` (uno de los que produce el pueblo) del carro de la columna, hasta donde quepa. Si se llena, sale el carro de acarreo.
    fn dejar_en_pueblo(&mut self, t: Instante, p: &Peticion, heroe: u64, pueblo: u64, recurso: Recurso, cantidad: Mil) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let (madre, cid) = self.dentro_del_pueblo(p, heroe, pueblo)?;
        if cantidad <= 0 {
            return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "hay que dejar una cantidad positiva".into()));
        }
        let datos = &self.estado.reglas.economia.datos;
        let pu = self.pueblo(pueblo).expect("existe").1;
        if !pu.recursos(datos).contains(&recurso) {
            return Err(rechazo(rechazos::PUEBLO_RECURSO_AJENO, format!("este pueblo no guarda {}: solo lo que produce", recurso.nombre())));
        }
        let lleva = self.estado.ejercitos[&cid].columna.carga.get(&recurso).copied().unwrap_or(0);
        if lleva <= 0 {
            return Err(rechazo(rechazos::PUEBLO_NADA_QUE_DEJAR, format!("tu columna no lleva {}", recurso.nombre())));
        }
        // La capacidad es la del almacén entero: la aldea la reparte entre el trigo y el ganado.
        let libre = (datos.pueblos.capacidad_almacen - pu.existencias(datos)).min(pu.almacen.espacio(recurso)).max(0);
        if libre == 0 {
            return Err(rechazo(rechazos::PUEBLO_ALMACEN_SIN_SITIO, "el almacén del pueblo está lleno".into()));
        }
        let n = cantidad.min(lleva).min(libre);
        self.almacen_de_pueblo(madre, pueblo).agregar(recurso, n);
        let carga = &mut self.estado.ejercitos.get_mut(&cid).expect("existe").columna.carga;
        *carga.entry(recurso).or_insert(0) -= n;
        carga.retain(|_, x| *x > 0);
        self.tr.tocar(Clase::Ejercito, cid);
        self.tr.tocar(Clase::Pueblo, pueblo);
        self.tr.tocar(Clase::Asentamiento, madre);
        self.emitir_propio(t, "pueblo.dejado", vec![madre], vec![("pueblo", pueblo as i64), ("heroe", heroe as i64), ("recurso", crate::codec_economia::recurso_a_u8(recurso) as i64), ("cantidad", n)]);
        self.salir_acarreo(madre, pueblo, t);
        Ok(())
    }

    /// El almacén local del pueblo `pueblo` de la plaza `madre` (que existen).
    fn almacen_de_pueblo(&mut self, madre: u64, pueblo: u64) -> &mut asentamiento::almacen::Almacen {
        &mut self.estado.asentamientos.get_mut(&madre).and_then(|a| a.economia.pueblos.iter_mut().find(|p| p.id == pueblo)).expect("existe").almacen
    }

    // ───────────────────────────── Cuando la guarnición se va ─────────────────────────────

    /// Las escuadras de jugadores del pueblo de los héroes que cumplen `suelta` vuelven, intactas, al campamento de su héroe. Devuelve cuántas.
    pub(super) fn liberar_guarnicion_de_pueblo(&mut self, pueblo: u64, suelta: impl Fn(&crate::estado_militar::HeroeNucleo) -> bool) -> usize {
        let mut liberadas = 0;
        let heroes: Vec<u64> = self.heroes().filter(|h| suelta(h) && h.heroe.escuadras.iter().any(|e| e.contenedor == Contenedor::Pueblo(pueblo))).map(|h| h.heroe.id).collect();
        for h in heroes {
            for e in self.heroe_mut(h).heroe.escuadras.iter_mut().filter(|e| e.contenedor == Contenedor::Pueblo(pueblo)) {
                e.contenedor = Contenedor::Campamento;
                liberadas += 1;
            }
        }
        if liberadas > 0 {
            self.tr.tocar(Clase::Pueblo, pueblo);
        }
        liberadas
    }

    /// `heroe` acaba de cambiar de Facción: sus escuadras en pueblos de plazas que ya no son de su Facción vuelven a su campamento.
    pub(super) fn guarnicion_de_pueblos_tras_cambiar_de_faccion(&mut self, heroe: u64) {
        let Some(h) = self.heroe(heroe) else { return };
        let faccion = h.faccion;
        let pueblos: std::collections::BTreeSet<u64> = h.heroe.escuadras.iter().filter_map(|e| if let Contenedor::Pueblo(x) = e.contenedor { Some(x) } else { None }).collect();
        for x in pueblos {
            let de_su_faccion = self.pueblo(x).is_some_and(|(madre, _)| Some(self.estado.asentamientos[&madre].faccion) == faccion);
            if !de_su_faccion {
                self.liberar_guarnicion_de_pueblo(x, |h| h.heroe.id == heroe);
            }
        }
    }

    /// La plaza `plaza` acaba de cambiar de dueño: en sus pueblos, la guarnición de los héroes que no son de la Facción nueva vuelve a su campamento.
    pub(super) fn guarnicion_de_pueblos_tras_conquista(&mut self, plaza: u64) {
        let Some(a) = self.estado.asentamientos.get(&plaza) else { return };
        let faccion = a.faccion;
        let pueblos: Vec<u64> = a.economia.pueblos.iter().map(|p| p.id).collect();
        for x in pueblos {
            self.liberar_guarnicion_de_pueblo(x, |h| h.faccion != Some(faccion));
        }
    }
}
