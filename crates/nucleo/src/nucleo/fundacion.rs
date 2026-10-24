//! La Caravana de Fundación de verdad y la refundación (sistema 5, tanda 5G-b; Fundacion_Y_Refundacion §4, §6, §7 y §9; Movimiento_Ejercitos §9). Todo en unidades enteras (C3b): lo que
//! se mueve se redondea hacia abajo y nada se crea ni se pierde salvo lo que la regla pierde (lo que no cabe al devolver, lo de quien dejó la Facción, la caravana sin camino a casa).
//!
//! - **Nace parada en su origen** (`Situacion::Disponible`), sin destino, con un titular: la lanza desde una plaza quien reside y está en ella (`lanzarCaravanaFundacion`) o la compra en un
//!   campamento una Facción sin plazas con su fondo (`comprarCaravanaDeRefundacion`). Suelta, caduca a las 48 h (cita `SucesoFundacion::Caduca`) y devuelve lo que costó.
//! - **La engancha su titular** a un ejército en el que va, a la puerta de donde está (`adjuntarCaravana` con su id), y funda con ella (`fundar`, que la gasta).
//! - **La de la administración** (`DarCaravanaFundacion`, sin origen ni coste) va con su columna mientras la columna exista; sin ella, se pierde.
//! - **Sin titular vuelve sola** a su origen (`Situacion::Retornando`, cita `SucesoFundacion::Llega`): lo decide [`Nucleo::vigilar_fundaciones`] al cerrar cada versión, solo para las
//!   caravanas cuya columna o cuyo titular tocó la transición, en vez de mirarlas a cada tick. Por el camino la reclama otro héroe de su Facción (`reclamarCaravana`); al llegar se desarma.
//! - **El fondo de refundación** es por campamento y por héroe (`EstadoFundacion::fondos`); el de una Facción es lo de sus ciudadanos.

use super::*;
use crate::codec_economia::recurso_a_u8;
use crate::comando::LadoDelFondo;
use crate::estado::{Dato, SucesoFundacion, PRIO_COMERCIO};
use crate::estado_fundacion::{entidad_fundacion, CaravanaFundacion, Origen, Situacion};
use asentamiento::datos::UNO;
use asentamiento::Recurso;
use base::{entero, UNIDAD};
use comercio::{de, sumar, total, Carga as Mercancia};
use militar::columna::{Politica, TipoColumna};
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;
use std::collections::BTreeMap;

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
    // ───────────────────────────── Consultas ─────────────────────────────

    /// `costoCaravanaFundacion` (§3): los materiales iniciales, la Granja, las Viviendas iniciales y la madera de fabricar la caravana. Con las cifras de hoy, 160 de madera, 20 de piedra,
    /// 100 de trigo y 100 de oro.
    pub fn costo_caravana_fundacion(&self) -> Mercancia {
        let eco = &self.estado.reglas.economia.datos;
        let mut c = Mercancia::new();
        for &(r, n) in &eco.materiales_iniciales {
            sumar(&mut c, r, n);
        }
        for &(r, n) in &eco.catalogo.tipo(asentamiento::Tipo::Granja).costo {
            sumar(&mut c, r, n);
        }
        for &(r, n) in &eco.catalogo.tipo(asentamiento::Tipo::Vivienda).costo {
            sumar(&mut c, r, n * i64::from(eco.viviendas_iniciales));
        }
        sumar(&mut c, Recurso::Madera, eco.madera_extra_caravana);
        c
    }

    /// `costoRefundacion` (§3): cada recurso del coste normal por `porcentajeCoste`, **redondeado hacia arriba** a unidades, sin los que quedan en 0. Hoy 64, 8, 40 y 40.
    pub fn costo_refundacion(&self) -> Mercancia {
        let pct = self.estado.reglas.economia.comercio.mercenarios.porcentaje_refundacion as i128;
        let por_unidad = UNO as i128 * UNIDAD as i128;
        self.costo_caravana_fundacion()
            .into_iter()
            .map(|(r, n)| (r, ((n as i128 * pct + por_unidad - 1) / por_unidad) as Mil * UNIDAD))
            .filter(|&(_, n)| n > 0)
            .collect()
    }

    /// El cap de fundación de la Facción: el de su nivel por experiencia (`politica::nivel::cap_fundacion`, 5C), el mismo que mira `fundar`.
    fn cap_de_fundacion_de(&self, faccion: u32) -> usize {
        politica::nivel::cap_fundacion(&self.estado.reglas.economia.politica.nivel, self.nivel_de_faccion(faccion)) as usize
    }

    /// Las Caravanas de Fundación vivas de la Facción (el cupo del cap se reserva al lanzar, §4.6).
    fn fundaciones_de(&self, faccion: u32) -> usize {
        self.estado.fundacion.caravanas.values().filter(|c| c.faccion == Some(faccion)).count()
    }

    /// `fondoDeFaccion` (§9.1): lo aportado en el campamento por los héroes que **son** ciudadanos de esa Facción, por recurso.
    pub fn fondo_de_faccion(&self, campamento: u32, faccion: u32) -> Mercancia {
        let mut fondo = Mercancia::new();
        for (h, suyo) in self.estado.fundacion.fondos.get(&campamento).into_iter().flatten() {
            if self.heroe(*h).is_some_and(|x| x.faccion == Some(faccion)) {
                suyo.iter().for_each(|(&r, &n)| sumar(&mut fondo, r, n));
            }
        }
        fondo
    }

    /// Dónde está la puerta del origen de la caravana: `None` si no tiene (la de la administración) o si la plaza ya no existe.
    fn puerta_de_origen(&self, origen: Origen) -> Option<Punto> {
        match origen {
            Origen::Plaza(a) => self.estado.asentamientos.get(&a).map(|a| a.posicion),
            Origen::Campamento(c) => self.puerta_de_campamento(c),
            Origen::Ninguno => None,
        }
    }

    /// Un hecho de la Caravana de Fundación: lo oyen su Facción y su titular (y quien esté en su plaza de origen).
    fn hecho_de_fundacion(&mut self, t: Instante, codigo: &'static str, c: &CaravanaFundacion, plazas: Vec<u64>, extra: Vec<(&'static str, i64)>) {
        let mut datos = vec![("caravana", c.id as i64), ("heroe", c.titular as i64)];
        datos.extend(c.faccion.map(|f| ("faccion", i64::from(f))));
        datos.extend(extra);
        let punto = Some(c.posicion);
        self.tr.emitir(&self.estado, EventoNucleo { asentamientos: plazas, punto, ..EventoNucleo::nuevo(codigo, t, datos) });
    }

    // ───────────────────────────── Nacer, devolver, perder ─────────────────────────────

    /// Da de alta una Caravana de Fundación parada en su origen, que caduca a `caducidadCaravanaHoras`. Devuelve su id (del contador de las caravanas: en el contrato es `car-N`).
    #[allow(clippy::too_many_arguments)]
    fn nueva_fundacion(&mut self, t: Instante, faccion: u32, titular: u64, origen: Origen, posicion: Punto, contenido: Mercancia, aportes: BTreeMap<u64, Mercancia>) -> u64 {
        let id = self.estado.comercio.siguiente_caravana;
        self.estado.comercio.siguiente_caravana += 1;
        let caduca_en = t + self.estado.reglas.economia.datos.caducidad_caravana;
        self.estado.fundacion.caravanas.insert(id, CaravanaFundacion { id, faccion: Some(faccion), titular, origen, posicion, contenido, aportes, situacion: Situacion::Disponible, caduca_en: Some(caduca_en) });
        self.programar_fundacion(id, caduca_en, SucesoFundacion::Caduca);
        self.tr.tocar(Clase::Caravana, id);
        id
    }

    /// Programa el plazo de la caravana con una generación nueva: el de la situación anterior queda descartado.
    fn programar_fundacion(&mut self, id: u64, en: Instante, suceso: SucesoFundacion) {
        let entidad = entidad_fundacion(id);
        self.estado.generaciones.subir(entidad);
        let clave = self.estado.generaciones.clave(entidad);
        self.estado.cola.programar(en.max(self.estado.ahora), PRIO_COMERCIO, clave, Carga::Fundacion(suceso));
    }

    /// Saca la caravana del estado (sus plazos se descartan) y la devuelve para quien la gasta, la devuelve o la pierde.
    fn quitar_fundacion(&mut self, id: u64) -> Option<CaravanaFundacion> {
        let c = self.estado.fundacion.caravanas.remove(&id)?;
        self.estado.generaciones.subir(entidad_fundacion(id));
        self.apartar(entidad_fundacion(id));
        self.tr.tocar(Clase::Caravana, id);
        Some(c)
    }

    /// La caravana se gasta en la plaza que funda (`fundar`).
    pub(super) fn gastar_fundacion(&mut self, id: u64) {
        self.quitar_fundacion(id);
    }

    /// `devolverCaravana` (§7.1): la de una plaza devuelve lo que costó a su almacén, hasta donde quepa (lo demás se pierde); la de un campamento, al fondo de ese campamento a nombre de cada
    /// aportante que siga siendo ciudadano de su Facción (lo de los demás se pierde). Sin origen en pie, se pierde. `codigo` es el hecho (`fundacion.caravana_devuelta`, o
    /// `expansion.caravana_desarmada` si la desarma su titular).
    fn devolver_fundacion(&mut self, id: u64, t: Instante, codigo: &'static str) {
        let Some(c) = self.quitar_fundacion(id) else { return };
        match c.origen {
            Origen::Plaza(a) if self.estado.asentamientos.contains_key(&a) => {
                let almacen = &mut self.estado.asentamientos.get_mut(&a).expect("existe").economia.almacen;
                let devuelto: Mil = c.contenido.iter().map(|(&r, &n)| almacen.agregar(r, n)).sum();
                self.tr.tocar(Clase::Asentamiento, a);
                self.hecho_de_fundacion(t, codigo, &c, vec![a], vec![("devuelto", devuelto)]);
            }
            Origen::Campamento(k) => {
                let mut devuelto = 0;
                for (h, suyo) in &c.aportes {
                    if self.heroe(*h).is_none_or(|x| x.faccion != c.faccion) {
                        continue;
                    }
                    let fondo = self.estado.fundacion.fondos.entry(k).or_default().entry(*h).or_default();
                    suyo.iter().for_each(|(&r, &n)| sumar(fondo, r, n));
                    devuelto += total(suyo);
                }
                self.tr.tocar(Clase::Campamento, u64::from(k));
                self.hecho_de_fundacion(t, codigo, &c, vec![], vec![("campamento", i64::from(k)), ("devuelto", devuelto)]);
            }
            Origen::Plaza(_) | Origen::Ninguno => self.hecho_de_fundacion(t, "fundacion.caravana_perdida", &c, vec![], vec![]),
        }
    }

    /// La caravana se queda sin su titular (§7): vuelve sola a su origen desde donde está, a `CARAVANA_CATALOGO.construccion.velocidad`. Ya en su puerta, llega en el acto y se desarma;
    /// sin origen o sin camino por tierra, se pierde.
    fn volver_sola(&mut self, id: u64, t: Instante) {
        let c = &self.estado.fundacion.caravanas[&id];
        let desde = match c.situacion {
            Situacion::Adjunta(e) => self.estado.ejercitos.get(&e).map_or(c.posicion, |e| e.posicion_en(t)),
            _ => c.posicion_en(t),
        };
        let origen = c.origen;
        self.estado.fundacion.caravanas.get_mut(&id).expect("existe").posicion = desde;
        let Some(hasta) = self.puerta_de_origen(origen) else {
            let c = self.quitar_fundacion(id).expect("existe");
            return self.hecho_de_fundacion(t, "fundacion.caravana_perdida", &c, vec![], vec![]);
        };
        let camino = if desde.dist(hasta) <= self.radio_puerta() { Some(None) } else { self.camino_sin_red(desde, hasta).map(Some) };
        match camino {
            None => {
                let c = self.quitar_fundacion(id).expect("existe");
                self.hecho_de_fundacion(t, "fundacion.caravana_perdida", &c, vec![], vec![]);
            }
            Some(None) => self.devolver_fundacion(id, t, "fundacion.caravana_devuelta"),
            Some(Some(camino)) => {
                let velocidad = self.estado.reglas.economia.comercio.flota.construccion.velocidad;
                let mut plan = rutas::a_plan_por_minuto(&camino.puntos, &self.costes_efectivos(&camino.puntos, &camino.costes), self.rutas.espaciado(), t, velocidad, 0);
                let fin = plan.fin().unwrap_or(t);
                self.programar_fundacion(id, fin, SucesoFundacion::Llega);
                plan.generacion = self.estado.generaciones.actual(entidad_fundacion(id));
                let c = self.estado.fundacion.caravanas.get_mut(&id).expect("existe");
                (c.situacion, c.posicion, c.caduca_en) = (Situacion::Retornando(plan.clone()), desde, None);
                let c = c.clone();
                // En el mapa (decisión del autor 5G-b-5): en la rejilla de encuentros, como cualquier caravana que viaja.
                self.anunciar_caravana(entidad_fundacion(id), Some(plan));
                self.tr.tocar(Clase::Caravana, id);
                self.hecho_de_fundacion(t, "fundacion.caravana_vuelve", &c, vec![], vec![("llegaEn", fin)]);
            }
        }
    }

    /// Una caravana enganchada sigue con su columna mientras **la columna exista, su titular vaya en ella y siga siendo ciudadano de su Facción** (§7). Se mira al cerrar cada versión, y solo
    /// en las caravanas cuya columna o cuyo titular tocó la transición: ninguna otra cosa puede cambiar la respuesta.
    pub(super) fn vigilar_fundaciones(&mut self, t: Instante) {
        if self.estado.fundacion.caravanas.is_empty() {
            return;
        }
        let tocados = &self.tr.tocados;
        let sueltas: Vec<u64> = self
            .estado
            .fundacion
            .caravanas
            .values()
            .filter(|c| match c.situacion {
                // La de la administración (`DarCaravanaFundacion`) es andamiaje para montar escenarios: va con su columna mientras exista, sea quien sea quien la lleve.
                // DUDA: así `fundacion.sin_fundadores` sigue pudiendo darse (con la regla real, el titular va en la columna y es ciudadano: siempre hay un fundador).
                Situacion::Adjunta(e) if c.origen == Origen::Ninguno => tocados.contiene(Clase::Ejercito, e) && !self.estado.ejercitos.contains_key(&e),
                Situacion::Adjunta(e) => (tocados.contiene(Clase::Ejercito, e) || tocados.contiene(Clase::Heroe, c.titular)) && !self.la_lleva_su_titular(c, e),
                _ => false,
            })
            .map(|c| c.id)
            .collect();
        for id in sueltas {
            self.volver_sola(id, t);
        }
    }

    fn la_lleva_su_titular(&self, c: &CaravanaFundacion, ejercito: u64) -> bool {
        self.estado.ejercitos.get(&ejercito).is_some_and(|e| e.columna.participa(c.titular)) && self.heroe(c.titular).is_some_and(|h| h.faccion == c.faccion)
    }

    /// La columna `id` sale del mundo: sus Caravanas de Fundación se quedan donde estaba (desde ahí vuelven solas si nadie las recoge).
    pub(super) fn fijar_fundaciones_de(&mut self, id: u64) {
        let Some(p) = self.estado.ejercitos.get(&id).map(|e| e.posicion_en(self.estado.ahora)) else { return };
        self.estado.fundacion.caravanas.values_mut().filter(|c| c.situacion == Situacion::Adjunta(id)).for_each(|c| c.posicion = p);
    }

    /// La columna `otra` se funde en `base`: sus Caravanas de Fundación pasan a ir en `base`, con su titular.
    pub(super) fn reenganchar_fundaciones(&mut self, otra: u64, base: u64) {
        let ids: Vec<u64> = self.estado.fundacion.caravanas.values().filter(|c| c.situacion == Situacion::Adjunta(otra)).map(|c| c.id).collect();
        for id in ids {
            self.estado.fundacion.caravanas.get_mut(&id).expect("existe").situacion = Situacion::Adjunta(base);
            self.tr.tocar(Clase::Caravana, id);
        }
    }

    /// Un plazo de la cola: la suelta caduca y la que vuelve sola llega. Cada uno solo vale en la situación para la que se programó (la generación lo asegura; esto es defensivo).
    pub(super) fn suceso_de_fundacion(&mut self, id: u64, suceso: SucesoFundacion, t: Instante) {
        let Some(c) = self.estado.fundacion.caravanas.get(&id) else { return };
        let vence = match (suceso, &c.situacion) {
            (SucesoFundacion::Caduca, Situacion::Disponible) => c.caduca_en.is_some_and(|x| x <= t),
            (SucesoFundacion::Llega, Situacion::Retornando(plan)) => plan.fin().is_none_or(|x| x <= t),
            _ => false,
        };
        if vence {
            if let Situacion::Retornando(plan) = &c.situacion {
                let fin = plan.vertices.last().map(|v| v.p);
                if let Some(p) = fin {
                    self.estado.fundacion.caravanas.get_mut(&id).expect("existe").posicion = p;
                }
            }
            self.devolver_fundacion(id, t, "fundacion.caravana_devuelta");
        }
    }

    // ───────────────────────────── Engancharla y reclamarla ─────────────────────────────

    /// `adjuntarCaravana` con una Caravana de Fundación (Movimiento_Ejercitos §9): suelta en su origen, con un ejército de su Facción a la puerta en el que va **su titular**. Deja de caducar.
    pub(super) fn adjuntar_fundacion(&mut self, t: Instante, p: &Peticion, caravana: u64, ejercito: u64) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let e = self.estado.ejercitos.get(&ejercito).ok_or_else(|| Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(ejercito), v))?;
        if !self.manda_columna(p, e, false) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya"));
        }
        if !Self::es_ejercito(e) {
            return Err(rechazo(rechazos::COLUMNA_PERSONAL_SIN_CARAVANAS, "solo un ejército lleva caravanas: quien quiere llevar una sale ya como ejército"));
        }
        let c = &self.estado.fundacion.caravanas[&caravana];
        if c.situacion != Situacion::Disponible {
            return Err(rechazo(rechazos::CARAVANA_ESTADO_INVALIDO, "solo se engancha la Caravana de Fundación suelta en su origen (la que vuelve sola se reclama)"));
        }
        if e.columna.faccion != c.faccion {
            return Err(rechazo(rechazos::CARAVANA_FACCION_AJENA, "la caravana es de otra Facción"));
        }
        if !e.columna.participa(c.titular) {
            return Err(rechazo(rechazos::FUNDACION_NO_TITULAR, "la Caravana de Fundación la lleva su titular: tiene que ir en el ejército"));
        }
        if !p.admin && e.posicion_en(t).dist(c.posicion) > self.radio_puerta() {
            return Err(rechazo(rechazos::CARAVANA_FUERA_DE_LA_PUERTA, "el ejército no está a la puerta de donde está la caravana"));
        }
        self.estado.generaciones.subir(entidad_fundacion(caravana));
        let c = self.estado.fundacion.caravanas.get_mut(&caravana).expect("existe");
        (c.situacion, c.caduca_en) = (Situacion::Adjunta(ejercito), None);
        self.tr.tocar(Clase::Caravana, caravana);
        self.tr.tocar(Clase::Ejercito, ejercito);
        self.evento("ejercito.caravana_adjuntada", t, ejercito, None, vec![("caravana", caravana as i64)]);
        Ok(None)
    }

    /// `reclamarCaravana` (Movimiento_Ejercitos §9): en mundo abierto, un héroe de su Facción, a bordo de su columna y a `radioEncuentro` o menos de la Caravana de Fundación que vuelve sola,
    /// la reclama y pasa a ser su titular. Una columna personal se convierte al instante en un ejército, con él de Líder y la política de unión que elige; si ya iba en un ejército, va con él.
    pub(super) fn reclamar_fundacion(&mut self, t: Instante, heroe: u64, caravana: u64, politica: Politica) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let (columna, e) = self.columna_a_bordo(heroe)?;
        let c = &self.estado.fundacion.caravanas[&caravana];
        if !matches!(c.situacion, Situacion::Retornando(_)) {
            return Err(rechazo(rechazos::CARAVANA_NO_RECLAMABLE, "solo se reclama la Caravana de Fundación que vuelve sola a su origen"));
        }
        if self.heroe(heroe).and_then(|h| h.faccion) != c.faccion || c.faccion.is_none() {
            return Err(rechazo(rechazos::CARAVANA_FACCION_AJENA, "la caravana es de otra Facción"));
        }
        if e.columna.es_formacion() {
            return Err(rechazo(rechazos::COLUMNA_EN_FORMACION, "una formación no lleva caravanas: espera a ser un ejército"));
        }
        let (aqui, alli) = (e.posicion_en(t), c.posicion_en(t));
        if aqui.dist(alli) > self.estado.reglas.economia.militar.logistica.radio_encuentro {
            return Err(rechazo(rechazos::CARAVANA_LEJOS, "hay que estar junto a la caravana para reclamarla"));
        }
        let personal = e.columna.tipo == TipoColumna::Personal;
        self.estado.generaciones.subir(entidad_fundacion(caravana));
        self.apartar(entidad_fundacion(caravana));
        let c = self.estado.fundacion.caravanas.get_mut(&caravana).expect("existe");
        (c.titular, c.situacion, c.posicion) = (heroe, Situacion::Adjunta(columna), alli);
        if personal {
            let e = &mut self.estado.ejercitos.get_mut(&columna).expect("existe").columna;
            (e.tipo, e.politica) = (TipoColumna::Ejercito, politica);
        }
        self.tr.tocar(Clase::Caravana, caravana);
        self.tr.tocar(Clase::Ejercito, columna);
        self.evento("ejercito.caravana_adjuntada", t, columna, Some(alli), vec![("caravana", caravana as i64), ("heroe", heroe as i64), ("reclamada", 1)]);
        Ok(vec![])
    }

    /// `soltarCaravana` con la Caravana de Fundación (decisión del autor 5G-b-4): solo **dentro de su origen**, la plaza o el campamento donde se enganchó, con la columna dentro (entró con
    /// ella). Queda suelta allí y vuelve a caducar a las 48 h. En cualquier otro sitio, `caravana.soltar_solo_dentro`. Si era lo último que llevaba la columna de un residente en su residencia, la
    /// columna se deshace como al entrar sin caravanas.
    pub(super) fn soltar_fundacion(&mut self, t: Instante, ejercito: u64, caravana: u64, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let c = &self.estado.fundacion.caravanas[&caravana];
        if c.situacion != Situacion::Adjunta(ejercito) {
            return Err(Respuesta::rechazo(rechazos::CARAVANA_NO_ESTA_ADJUNTA, "la caravana no va con esta columna", v));
        }
        let hn = self.heroe(heroe).expect("existe");
        let lugar = match (c.origen, hn.heroe.presencia.ubicacion) {
            (Origen::Plaza(a), u @ Ubicacion::Asentamiento(x)) if u64::from(x) == a => Some(u),
            (Origen::Campamento(k), u @ Ubicacion::Mercenarios(x)) if x == k => Some(u),
            _ => None,
        };
        let (Some(lugar), true) = (lugar, hn.columna == Some(ejercito)) else {
            return Err(Respuesta::rechazo(rechazos::CARAVANA_SOLTAR_SOLO_DENTRO, "la Caravana de Fundación solo se suelta dentro de su origen, la plaza o el campamento donde se enganchó", v));
        };
        let puerta = self.puerta_de_origen(c.origen).expect("está dentro de su origen");
        let caduca_en = t + self.estado.reglas.economia.datos.caducidad_caravana;
        let c = self.estado.fundacion.caravanas.get_mut(&caravana).expect("existe");
        (c.situacion, c.posicion, c.caduca_en) = (Situacion::Disponible, puerta, Some(caduca_en));
        self.estado.generaciones.subir(entidad_fundacion(caravana));
        self.programar_fundacion(caravana, caduca_en, SucesoFundacion::Caduca);
        self.tr.tocar(Clase::Caravana, caravana);
        self.tr.tocar(Clase::Ejercito, ejercito);
        self.evento("ejercito.caravana_soltada", t, ejercito, None, vec![("caravana", caravana as i64), ("heroe", heroe as i64)]);
        let e = &self.estado.ejercitos[&ejercito];
        let reside = self.heroe(heroe).and_then(|h| h.heroe.residencia).is_some_and(|r| match (r, lugar) {
            (Residencia::Asentamiento(a), Ubicacion::Asentamiento(x)) => a == x,
            (Residencia::Campamento(a), Ubicacion::Mercenarios(x)) => a == x,
            _ => false,
        });
        if reside && e.caravanas_adjuntas.is_empty() && e.columna.participantes.len() == 1 && self.estado.fundacion.de_columna(ejercito).is_none() {
            self.recoger_en_residencia(t, heroe, ejercito, lugar);
        }
        Ok(vec![])
    }

    /// Una plaza cae (conquista o ruina): la Caravana de Fundación suelta en ella se destruye con la plaza y lo que costó se pierde (decisión del autor 5G-b-11, como las aparcadas, canon 3.13.7).
    pub(super) fn destruir_fundaciones_en(&mut self, plaza: u64, t: Instante) {
        let sueltas: Vec<u64> = self.estado.fundacion.caravanas.values().filter(|c| c.situacion == Situacion::Disponible && c.origen == Origen::Plaza(plaza)).map(|c| c.id).collect();
        for id in sueltas {
            let c = self.quitar_fundacion(id).expect("existe");
            self.hecho_de_fundacion(t, "fundacion.caravana_perdida", &c, vec![], vec![("plaza", plaza as i64)]);
        }
    }

    /// La Caravana de Fundación cae en manos de quien la intercepta (Combate §7.1, decisión del autor del 2026-10-24): no lleva carros, así que en una **disputa** se la llevan entera
    /// —se pierde y su carga entera es el botín— y en un **aguante** pierde el 70 % de lo que lleva, redondeado hacia abajo (también de los aportes, por héroe en orden de id, para
    /// que lo que vuelva a nombre de cada uno sume lo que queda), y sigue su vuelta con el 30 %. Devuelve lo perdido (lo carga quien llama) y si se perdió la caravana.
    pub(super) fn caer_fundacion(&mut self, t: Instante, x: u64, modalidad: combate::caravana::Modalidad) -> (Mercancia, bool) {
        use combate::caravana::Modalidad;
        let fraccion = self.estado.reglas.economia.combate.numericas.carga_perdida_aguante as i128;
        let Some(c) = self.estado.fundacion.caravanas.get_mut(&x) else { return (Mercancia::new(), false) };
        let perdido = match modalidad {
            Modalidad::Disputa => std::mem::take(&mut c.contenido),
            Modalidad::Aguante => {
                let mut perdido = Mercancia::new();
                for (&rec, n) in c.contenido.iter_mut() {
                    let l = entero((*n as i128 * fraccion / UNO as i128) as Mil);
                    if l > 0 {
                        *n -= l;
                        perdido.insert(rec, l);
                    }
                }
                c.contenido.retain(|_, n| *n > 0);
                for (&rec, &l) in &perdido {
                    let mut falta = l;
                    for suyo in c.aportes.values_mut() {
                        let toma = suyo.get(&rec).copied().unwrap_or(0).min(falta);
                        if toma > 0 {
                            quitar(suyo, rec, toma);
                            falta -= toma;
                        }
                    }
                }
                c.aportes.retain(|_, m| !m.is_empty());
                perdido
            }
        };
        self.tr.tocar(Clase::Caravana, x);
        let destruida = modalidad == Modalidad::Disputa;
        if destruida {
            let c = self.quitar_fundacion(x).expect("existe");
            self.hecho_de_fundacion(t, "fundacion.caravana_perdida", &c, vec![], vec![("interceptada", 1)]);
        }
        (perdido, destruida)
    }

    /// Una batalla de Unity aparta a la Caravana de Fundación que vuelve sola: se queda quieta donde está (un plan de un solo punto, sin llegada) y sale del espacio.
    pub(super) fn apartar_fundacion(&mut self, t: Instante, x: u64) -> bool {
        let Some(c) = self.estado.fundacion.caravanas.get(&x) else { return false };
        let Situacion::Retornando(plan) = &c.situacion else { return false };
        let p = c.posicion_en(t);
        let quieta = Plan::nuevo(plan.generacion, vec![base::Vertice { p, en: t }]);
        self.estado.generaciones.subir(entidad_fundacion(x));
        self.apartar(entidad_fundacion(x));
        let c = self.estado.fundacion.caravanas.get_mut(&x).expect("existe");
        (c.situacion, c.posicion) = (Situacion::Retornando(quieta), p);
        self.tr.tocar(Clase::Caravana, x);
        true
    }

    /// La batalla acaba y la Caravana de Fundación, si sigue, retoma su vuelta desde donde se quedó.
    pub(super) fn reponer_fundacion(&mut self, t: Instante, x: u64) -> bool {
        if !self.estado.fundacion.caravanas.get(&x).is_some_and(|c| matches!(c.situacion, Situacion::Retornando(_))) {
            return false;
        }
        self.volver_sola(x, t);
        true
    }

    /// `atacar` contra la Caravana de Fundación que vuelve sola (decisión del autor 5G-b-5; Combate §7.1, canon 1.8: «interceptable igual que las demás, tipo A o B según la zona»). Exige guerra con
    /// su Facción o la declara; se defienden los carreteros (`defensaBaseCaravana`: no lleva escolta) con la ventaja del aguante en la zona de su dueño o de un aliado. Con servidores de
    /// batalla abre una batalla de caravana en Unity, como las comerciales (decisión del autor del 2026-10-24); sin ellos se resuelve con números. Lo que pierde: [`Nucleo::caer_fundacion`].
    pub(super) fn atacar_fundacion(&mut self, t: Instante, heroe: u64, id: u64, x: u64, declara_guerra: bool) -> Result<(), Respuesta> {
        use combate::caravana::{self as asalto, Defensa, Modalidad};
        use combate::resolucion::Bando;
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let k = &self.estado.fundacion.caravanas[&x];
        match k.situacion {
            Situacion::Adjunta(_) => return Err(rechazo(rechazos::COMBATE_CARAVANA_ADJUNTA, "esa caravana va adjunta a un ejército: se ataca al ejército")),
            Situacion::Disponible => return Err(rechazo(rechazos::COMBATE_A_CUBIERTO, "esa caravana no está en el mapa")),
            Situacion::Retornando(_) => {}
        }
        let (dueno, donde) = (k.faccion, k.posicion_en(t));
        let guerra = self.comprobar_agresion(&self.estado.ejercitos[&id], t, donde, dueno, declara_guerra)?;
        let modalidad = asalto::modalidad_de(self.zona_en(donde, dueno));
        if self.estado.reglas.batallas_en_unity {
            let ap = self.apertura_de_fundacion(t, heroe, id, x, donde, modalidad);
            self.abrir_batalla_de_ataque(t, ap, guerra);
            return Ok(());
        }
        let cr = self.estado.reglas.economia.combate.clone();
        let mut atac = self.tropa_de_columna(id, t, cr.batalla.capacidad_resto);
        let columnas = self.columnas_de(&atac.escuadras);
        let participantes = Self::participantes_de(&self.estado.ejercitos[&id]);
        let punto = self.estado.ejercitos[&id].posicion_en(t);
        if let Some((a, b)) = guerra {
            self.declarar_guerra(t, a, b);
        }
        let r = asalto::resolver_asalto(&cr, Bando { escuadras: &mut atac.escuadras, heroes: &atac.heroes }, Defensa::Base, modalidad, &mut self.estado.rng).expect("la columna lleva al menos un héroe sano");
        // Como el asalto a una caravana comercial (al integrar 5D): la experiencia de un combate digno (5C-3) y la defensa del vasallo, sin señora presente (§5.9).
        self.experiencia_de_asalto(t, &atac.heroes, r.poder_atacante, r.defensa);
        self.defensa_de_vasallo(t, dueno, &[], !r.gana_atacante);
        self.volcar_escuadras(&atac.escuadras);
        let (cogido, destruida) = if r.gana_atacante {
            let (perdido, destruida) = self.caer_fundacion(t, x, modalidad);
            (self.cargar_botin_en(id, &perdido), destruida)
        } else {
            (Mercancia::new(), false)
        };
        let heridos = if r.gana_atacante { 0 } else { self.herir_heroes(&participantes, t) };
        self.recomponer_tras_bajas(&columnas, t);
        let mut datos = vec![
            ("caravana", x as i64),
            ("aguante", (modalidad == Modalidad::Aguante) as i64),
            ("poderAtacante", r.poder_atacante),
            ("defensa", r.defensa),
            ("bajasAtacante", super::combates::caidos(&r.bajas_atacante)),
            ("bajasEscolta", 0),
            ("carros", destruida as i64),
            ("botin", total(&cogido)),
            ("destruida", destruida as i64),
            ("heridos", heridos as i64),
        ];
        datos.extend(dueno.map(|f| ("faccionDefensora", i64::from(f))));
        self.hecho_de_combate(if r.gana_atacante { "combate.caravana_interceptada" } else { "combate.caravana_resiste" }, t, vec![id], vec![], punto, datos);
        Ok(())
    }

    // ───────────────────────────── Las órdenes ─────────────────────────────

    pub(super) fn ejecutar_fundacion(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        match &p.comando {
            Comando::AportarARefundacion { heroe, recurso, cantidad, lado } => self.mover_fondo(t, p, *heroe, *recurso, *cantidad, *lado, true),
            Comando::RetirarDeRefundacion { heroe, recurso, cantidad, lado } => self.mover_fondo(t, p, *heroe, *recurso, *cantidad, *lado, false),
            Comando::ComprarCaravanaDeRefundacion { heroe } => self.comprar_refundacion(t, p, *heroe),
            Comando::LanzarCaravanaFundacion { heroe, asentamiento } => self.lanzar_fundacion(t, p, *heroe, *asentamiento),
            Comando::DesarmarCaravanaFundacion { heroe, caravana } => self.desarmar_fundacion(t, p, *heroe, *caravana),
            _ => unreachable!("solo se llama con las órdenes de la refundación y la Caravana de Fundación"),
        }
    }

    /// `lanzarCaravanaFundacion` (§4), en este orden: la plaza; su titular es ciudadano de su Facción y reside y está en ella (Proyecciones §5.2.4); nivel operativo 2; el cooldown de caravanas
    /// de la plaza (compartido con las comerciales); el cupo del cap contando las caravanas vivas; y el coste en su almacén.
    fn lanzar_fundacion(&mut self, t: Instante, p: &Peticion, heroe: u64, plaza: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let a = self.plaza_o_rechazo(plaza)?;
        let (faccion, posicion) = (a.faccion, a.posicion);
        let hn = self.heroe(heroe).expect("existe");
        if hn.faccion != Some(faccion) {
            return Err(rechazo(rechazos::FUNDACION_NO_CIUDADANO, "solo un ciudadano de la Facción de la plaza lanza su Caravana de Fundación".into()));
        }
        let aqui = Residencia::Asentamiento(plaza as u32);
        if hn.heroe.residencia != Some(aqui) || hn.heroe.presencia.ubicacion != Ubicacion::Asentamiento(plaza as u32) {
            return Err(rechazo(rechazos::FUNDACION_NO_RESIDES, "la lanza quien reside en la plaza, estando dentro".into()));
        }
        if a.nivel_actual() < 2 {
            return Err(rechazo(rechazos::FUNDACION_NIVEL_INSUFICIENTE, format!("la plaza está a nivel {}: hace falta el 2", a.nivel_actual())));
        }
        let cooldown = self.estado.reglas.economia.comercio.flota.cooldown_creacion;
        if let Some(u) = self.estado.comercio.ultima_caravana.get(&plaza).filter(|&&u| u + cooldown > t) {
            return Err(rechazo(rechazos::CARAVANA_EN_COOLDOWN, format!("la plaza podrá crear otra caravana en {} s", (u + cooldown - t) / 1_000)));
        }
        let (tiene, cap) = (self.plazas_de(faccion).count() + self.fundaciones_de(faccion), self.cap_de_fundacion_de(faccion));
        if tiene >= cap {
            return Err(rechazo(rechazos::CAP_ALCANZADO, format!("la Facción tiene {tiene} entre plazas y Caravanas de Fundación y su cap es {cap}")));
        }
        let costo: Vec<(Recurso, Mil)> = self.costo_caravana_fundacion().into_iter().collect();
        let almacen = &mut self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.almacen;
        if !almacen.pagar(&costo) {
            return Err(rechazo(rechazos::CARAVANA_SIN_RECURSOS, "el almacén de la plaza no cubre el coste de la Caravana de Fundación".into()));
        }
        self.estado.comercio.ultima_caravana.insert(plaza, t);
        self.tr.tocar(Clase::Asentamiento, plaza);
        let id = self.nueva_fundacion(t, faccion, heroe, Origen::Plaza(plaza), posicion, costo.into_iter().collect(), BTreeMap::new());
        let c = self.estado.fundacion.caravanas[&id].clone();
        self.hecho_de_fundacion(t, "expansion.caravana_lanzada", &c, vec![plaza], vec![]);
        Ok(vec![id])
    }

    /// `desarmarCaravanaFundacion` (§6): solo su titular, suelta y parada en su origen (es donde está siempre la suelta). Devuelve lo que costó.
    fn desarmar_fundacion(&mut self, t: Instante, p: &Peticion, heroe: u64, caravana: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        self.autorizar_heroe(p, heroe, false)?;
        let c = self.estado.fundacion.caravanas.get(&caravana).ok_or_else(|| Respuesta::rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana), v))?;
        if c.titular != heroe {
            return Err(Respuesta::rechazo(rechazos::FUNDACION_NO_TITULAR, "solo su titular desarma la Caravana de Fundación", v));
        }
        if c.situacion != Situacion::Disponible {
            return Err(Respuesta::rechazo(rechazos::CARAVANA_NO_DISPONIBLE, "solo se desarma suelta y parada en su origen: desengánchala o déjala volver", v));
        }
        self.devolver_fundacion(caravana, t, "expansion.caravana_desarmada");
        Ok(vec![])
    }

    /// El campamento donde está el héroe (dentro, o con su columna a la puerta) y su Facción.
    fn campamento_y_faccion(&self, heroe: u64) -> Result<(u32, u32), Respuesta> {
        let v = self.estado.version;
        let campamento = self.campamento_del_heroe(heroe).ok_or_else(|| Respuesta::rechazo(rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA, "el héroe no está en un campamento ni con su columna a su puerta", v))?;
        let faccion = self.heroe(heroe).and_then(|h| h.faccion).ok_or_else(|| Respuesta::rechazo(rechazos::FACCION_NO_PERTENECE, "la refundación es de los ciudadanos de una Facción", v))?;
        Ok((campamento, faccion))
    }

    /// `aportarARefundacion` (`aporta`) y `retirarDeRefundacion` (§9.1): con el almacén personal (residiendo en ese campamento) o con el carro de la columna a la puerta (siendo su Líder).
    /// Aportar mueve `min(pedido, lo que hay)`; retirar, `min(pedido, lo aportado, el hueco)`. Solo aporta una Facción sin plazas; retirar lo propio se puede siempre que se siga siendo
    /// ciudadano.
    #[allow(clippy::too_many_arguments)]
    fn mover_fondo(&mut self, t: Instante, p: &Peticion, heroe: u64, recurso: Recurso, cantidad: Mil, lado: LadoDelFondo, aporta: bool) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let (campamento, faccion) = self.campamento_y_faccion(heroe)?;
        if aporta && self.plazas_de(faccion).next().is_some() {
            return Err(rechazo(rechazos::REFUNDACION_CON_PLAZAS, "solo una Facción sin plazas refunda desde un campamento".into()));
        }
        let pedido = entero(cantidad);
        if pedido <= 0 {
            return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "hay que mover al menos una unidad".into()));
        }
        let hn = self.heroe(heroe).expect("existe");
        // `columna`: la del carro (`None` = el almacén personal).
        let columna = match lado {
            LadoDelFondo::Almacen => {
                if hn.heroe.residencia != Some(Residencia::Campamento(campamento)) {
                    return Err(rechazo(rechazos::REFUNDACION_NO_RESIDES, "el almacén personal está en el campamento donde se reside: aquí, con el carro".into()));
                }
                None
            }
            LadoDelFondo::Carro => {
                let c = self.columna_de(heroe).ok_or_else(|| rechazo(rechazos::HEROE_SIN_COLUMNA, "hace falta tu columna a la puerta del campamento".into()))?;
                if self.estado.ejercitos[&c].columna.lider != heroe {
                    return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder de la columna mueve lo de su carro".into()));
                }
                Some(c)
            }
        };
        let carga = columna.map_or(&hn.almacen_personal, |c| &self.estado.ejercitos[&c].columna.carga);
        let aportado = self.estado.fundacion.fondos.get(&campamento).and_then(|f| f.get(&heroe)).map_or(0, |m| de(m, recurso));
        let movido = if aporta {
            let hay = de(carga, recurso);
            let movido = entero(pedido.min(hay));
            if movido <= 0 {
                return Err(match columna {
                    None => rechazo(rechazos::ALMACEN_SIN_EXISTENCIAS, format!("no hay {} en tu almacén personal", recurso.nombre())),
                    Some(_) => rechazo(rechazos::CARRO_NO_LLEVA, format!("el carro no lleva {}", recurso.nombre())),
                });
            }
            movido
        } else {
            let sitio = match columna {
                None => self.estado.reglas.economia.comercio.mercenarios.capacidad_almacen_personal - total(carga),
                Some(c) => self.capacidad_de_carga(c).unwrap_or(0) - total(carga),
            };
            let movido = entero(pedido.min(aportado).min(sitio.max(0)));
            if movido <= 0 {
                return Err(if aportado < UNIDAD {
                    rechazo(rechazos::REFUNDACION_SIN_APORTE, format!("no has aportado {} a este fondo", recurso.nombre()))
                } else if columna.is_none() {
                    rechazo(rechazos::ALMACEN_PERSONAL_LLENO, "tu almacén personal está lleno".into())
                } else {
                    rechazo(rechazos::CARRO_LLENO, "el carro está lleno".into())
                });
            }
            movido
        };
        let suyo = self.estado.fundacion.fondos.entry(campamento).or_default().entry(heroe).or_default();
        if aporta {
            sumar(suyo, recurso, movido);
        } else {
            quitar(suyo, recurso, movido);
        }
        let fondo = self.estado.fundacion.fondos.get_mut(&campamento).expect("existe");
        fondo.retain(|_, m| !m.is_empty());
        if fondo.is_empty() {
            self.estado.fundacion.fondos.remove(&campamento);
        }
        let carga = match columna {
            Some(c) => &mut self.estado.ejercitos.get_mut(&c).expect("existe").columna.carga,
            None => &mut self.estado.militar.heroes.get_mut(&heroe).expect("existe").almacen_personal,
        };
        if aporta {
            quitar(carga, recurso, movido);
        } else {
            sumar(carga, recurso, movido);
        }
        self.tr.tocar(Clase::Heroe, heroe);
        self.tr.tocar(Clase::Campamento, u64::from(campamento));
        if let Some(c) = columna {
            self.tr.tocar(Clase::Ejercito, c);
        }
        let datos = vec![("campamento", i64::from(campamento)), ("heroe", heroe as i64), ("faccion", i64::from(faccion)), ("recurso", recurso_a_u8(recurso) as i64), ("cantidad", movido)];
        let sentido = Dato::Texto(if aporta { "aporta" } else { "retira" }.into());
        self.tr.emitir(&self.estado, EventoNucleo { valores: vec![("sentido", sentido)], ..EventoNucleo::nuevo("mercenarios.fondo_refundacion", t, datos) });
        Ok(vec![])
    }

    /// `comprarCaravanaDeRefundacion` (§9.2): ciudadano de una Facción sin plazas ni otra Caravana de Fundación, con cap ≥ 1 y el fondo de su Facción en el campamento cubriendo
    /// `costoRefundacion`. Se gasta recurso a recurso de sus ciudadanos **en orden de ciudadanía**; lo de cada uno queda anotado en la caravana para devolvérselo. No toca el cooldown.
    fn comprar_refundacion(&mut self, t: Instante, p: &Peticion, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let (campamento, faccion) = self.campamento_y_faccion(heroe)?;
        if self.plazas_de(faccion).next().is_some() {
            return Err(rechazo(rechazos::REFUNDACION_CON_PLAZAS, "solo una Facción sin plazas refunda desde un campamento".into()));
        }
        if self.fundaciones_de(faccion) > 0 {
            return Err(rechazo(rechazos::REFUNDACION_YA_TIENE_CARAVANA, "la Facción ya tiene una Caravana de Fundación".into()));
        }
        if self.cap_de_fundacion_de(faccion) < 1 {
            return Err(rechazo(rechazos::CAP_ALCANZADO, "el cap de fundación de la Facción es 0".into()));
        }
        let costo = self.costo_refundacion();
        let fondo = self.fondo_de_faccion(campamento, faccion);
        let faltan: Vec<String> = costo.iter().filter(|(&r, &n)| de(&fondo, r) < n).map(|(&r, &n)| format!("{} {}", (n - de(&fondo, r)) / UNIDAD, r.nombre())).collect();
        if !faltan.is_empty() {
            return Err(rechazo(rechazos::REFUNDACION_FONDO_CORTO, format!("el fondo no cubre el coste: faltan {}", faltan.join(", "))));
        }
        let ciudadanos = self.estado.politica.facciones.get(faccion).map(|f| f.ciudadanos.clone()).unwrap_or_default();
        let fondos = self.estado.fundacion.fondos.entry(campamento).or_default();
        let mut aportes: BTreeMap<u64, Mercancia> = BTreeMap::new();
        for (&r, &necesario) in &costo {
            let mut pendiente = necesario;
            for h in &ciudadanos {
                let Some(suyo) = fondos.get_mut(h) else { continue };
                let toma = de(suyo, r).min(pendiente);
                if toma <= 0 {
                    continue;
                }
                quitar(suyo, r, toma);
                sumar(aportes.entry(*h).or_default(), r, toma);
                pendiente -= toma;
            }
            debug_assert_eq!(pendiente, 0, "el fondo cubría el coste");
        }
        fondos.retain(|_, m| !m.is_empty());
        if fondos.is_empty() {
            self.estado.fundacion.fondos.remove(&campamento);
        }
        self.tr.tocar(Clase::Campamento, u64::from(campamento));
        let puerta = self.puerta_de_campamento(campamento).expect("el campamento existe");
        let id = self.nueva_fundacion(t, faccion, heroe, Origen::Campamento(campamento), puerta, costo, aportes);
        let c = self.estado.fundacion.caravanas[&id].clone();
        self.hecho_de_fundacion(t, "mercenarios.caravana_refundacion", &c, vec![], vec![("campamento", i64::from(campamento))]);
        Ok(vec![id])
    }

    // ───────────────────────────── Administración ─────────────────────────────

    /// `DarCaravanaFundacion` (andamiaje): una Caravana de Fundación sin origen ni coste, enganchada a la columna, con su Líder de titular. `false` si ya lleva una.
    pub(super) fn dar_fundacion(&mut self, t: Instante, ejercito: u64) -> bool {
        if self.estado.fundacion.de_columna(ejercito).is_some() {
            return false;
        }
        let e = &self.estado.ejercitos[&ejercito];
        let (faccion, titular, posicion) = (e.columna.faccion, e.columna.lider, e.posicion_en(t));
        let id = self.estado.comercio.siguiente_caravana;
        self.estado.comercio.siguiente_caravana += 1;
        let c = CaravanaFundacion { id, faccion, titular, origen: Origen::Ninguno, posicion, contenido: Mercancia::new(), aportes: BTreeMap::new(), situacion: Situacion::Adjunta(ejercito), caduca_en: None };
        self.estado.fundacion.caravanas.insert(id, c);
        self.tr.tocar(Clase::Caravana, id);
        self.tr.tocar(Clase::Ejercito, ejercito);
        true
    }
}
