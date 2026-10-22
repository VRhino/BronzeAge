//! Los ámbitos de las sesiones (contrato 05 §3, 4H T3a): a cuáles tiene derecho cada una, qué entidades hay en cada ámbito (la `Foto`), qué cambió en ellos (la `Delta`) y a quién
//! hay que contárselo sin recorrer todas las sesiones.
//!
//! - **Autorización** ([`derechos`]): el servidor suscribe, el cliente nunca elige (salvo el interés del mapa, `VISTA`). El héroe propio (`HEROE`) y la Facción propia (`FACCION`) los
//!   tiene quien tenga héroe y Facción; la plaza (`ASENTAMIENTO`) y el campamento (`CAMPAMENTO`), mientras el héroe está dentro y, en la plaza, es ciudadano de su Facción (dentro de
//!   una plaza ajena no se ve ningún interior); `VISTA`, quien tiene Facción y ha fijado su rectángulo; `ADMIN`, solo una sesión de administración. Un servidor de batalla no tiene
//!   ninguno. Los derechos se calculan del estado: cuando cambian (el héroe sale de la plaza, cambia de Facción...) la sesión recibe `Revocado` o la `Foto` del ámbito nuevo.
//! - **Versión por ámbito**: cada sesión guarda la última versión que el cliente tiene de cada ámbito abierto. Una `Delta` va de esa versión a la del mundo y solo existe si algo del ámbito
//!   cambió en medio; el registro de cambios por clase de T0 dice qué entidades se tocaron en cada versión.
//! - **Reparto** ([`Mesa::repartir`]): por cada entidad tocada se busca quién tiene abierto un ámbito que la contiene, con un índice de suscriptores por ámbito.
//!
//! Qué lleva cada ámbito en esta tanda (el resto es de T3b-T3e): `HEROE`, las columnas en las que participa alguna cuenta del héroe; `ASENTAMIENTO`, la plaza (la ficha de hoy);
//! `VISTA`, las columnas de la Facción dentro del rectángulo; `ADMIN`, todas las columnas y todas las plazas; `FACCION` y `CAMPAMENTO`, nada todavía.

use super::depositar;
use crate::estado::{CambioVersion, Clase, EventoNucleo};
use crate::nucleo::Nucleo;
use crate::proyeccion::ambito::Ambito;
use crate::proyeccion::{cambio_asentamiento, cambio_ejercito, cambio_quita, entidad_asentamiento, entidad_ejercito, evento_dominio, ids};
use base::Punto;
use contrato::v1 as pb;
use militar::presencia::Ubicacion;
use servidor::{Difusion, SesionId};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tokio::sync::mpsc;

/// Una sesión abierta.
pub(super) struct Sesion {
    pub(super) cliente: String,
    pub(super) rol: pb::TipoCliente,
    pub(super) tx: mpsc::Sender<Difusion>,
    /// El héroe de la sesión: el de la cuenta (`HEROE:<héroe>`). DUDA: una cuenta que solo tiene héroes de campo (el andamiaje de administración, `CrearEjercito`) usa el primero.
    pub(super) heroe: Option<u64>,
    /// Los ámbitos abiertos, con la última versión que el cliente tiene de cada uno.
    pub(super) abiertos: BTreeMap<Ambito, u64>,
    /// El interés en el mapa que fijó `CambiarVista`.
    pub(super) vista: Option<(Punto, Punto)>,
}

impl Sesion {
    pub(super) fn nueva(n: &Nucleo, cliente: String, rol: pb::TipoCliente, tx: mpsc::Sender<Difusion>) -> Self {
        let heroe = es_jugador(rol).then(|| heroe_de_cuenta(n, &cliente)).flatten();
        Sesion { cliente, rol, tx, heroe, abiertos: BTreeMap::new(), vista: None }
    }

    pub(super) fn es_admin(&self) -> bool {
        self.rol == pb::TipoCliente::ClienteAdminWeb
    }

    /// ¿Está `p` dentro del rectángulo de su vista?
    fn ve(&self, p: Punto) -> bool {
        self.vista.is_some_and(|(a, b)| (a.x..=b.x).contains(&p.x) && (a.y..=b.y).contains(&p.y))
    }

    /// ¿Participa alguna de las cuentas de la sesión en la columna?
    fn participa_en(&self, n: &Nucleo, columna: &crate::estado::Ejercito) -> bool {
        columna.columna.participantes.iter().any(|p| n.heroe(p.heroe).is_some_and(|h| h.cuenta == self.cliente))
    }
}

/// Un jugador (Unity o un bot) tiene héroe; el administrador y el servidor de batalla, no.
pub(super) fn es_jugador(rol: pb::TipoCliente) -> bool {
    matches!(rol, pb::TipoCliente::ClienteUnity | pb::TipoCliente::ClienteBot)
}

/// El héroe de una cuenta: el de verdad si lo hay y, si no, el primer héroe de campo.
pub(super) fn heroe_de_cuenta(n: &Nucleo, cuenta: &str) -> Option<u64> {
    n.heroes().filter(|h| h.cuenta == cuenta).min_by_key(|h| (h.sintetico, h.heroe.id)).map(|h| h.heroe.id)
}

/// Los ámbitos a los que tiene derecho la sesión ahora.
pub(super) fn derechos(n: &Nucleo, s: &Sesion) -> BTreeSet<Ambito> {
    if s.es_admin() {
        return BTreeSet::from([Ambito::Admin]);
    }
    let Some(h) = s.heroe.and_then(|h| n.heroe(h)) else { return BTreeSet::new() };
    let mut d = BTreeSet::from([Ambito::Heroe(h.heroe.id)]);
    if let Some(f) = h.faccion {
        d.insert(Ambito::Faccion(f));
        if s.vista.is_some() {
            d.insert(Ambito::Vista(f));
        }
    }
    match h.heroe.presencia.ubicacion {
        Ubicacion::Asentamiento(a) if n.estado().asentamientos.get(&u64::from(a)).is_some_and(|p| Some(p.faccion) == h.faccion) => {
            d.insert(Ambito::Asentamiento(u64::from(a)));
        }
        Ubicacion::Mercenarios(c) => {
            d.insert(Ambito::Campamento(c));
        }
        _ => {}
    }
    d
}

/// ¿Puede el cliente que tiene el ámbito en la versión `k` seguir con él por deltas? El registro tiene que alcanzar `k` y, para los ámbitos que se ganan y se pierden con el sitio del
/// héroe (Facción, plaza, campamento), el héroe no puede haber cambiado desde entonces (si salió y volvió, el cliente descartó o no su caché sin que el servidor lo sepa: foto).
/// DUDA: conservador, también fuerza foto cualquier cambio del héroe que no sea de sitio (reclutar, herirse...); se afina si cuesta ancho de banda.
pub(super) fn reanudable(n: &Nucleo, s: &Sesion, a: Ambito, k: u64) -> bool {
    let Some(mut registros) = n.cambios_desde(k) else { return false };
    match a {
        Ambito::Heroe(_) | Ambito::Admin => true,
        _ => s.heroe.is_some_and(|h| registros.all(|cv| !cv.tocados.contiene(Clase::Heroe, h))),
    }
}

/// La `Foto` de un ámbito en la versión `version`.
pub(super) fn foto(n: &Nucleo, s: &Sesion, a: Ambito, version: u64) -> pb::Foto {
    let (estado, t) = (n.estado(), n.ahora());
    let plaza = |id: &u64| estado.asentamientos.get(id).map(|p| entidad_asentamiento(p, &n.poligono_zona(p.id)));
    let entidades = match a {
        Ambito::Heroe(_) => estado.ejercitos.values().filter(|e| s.participa_en(n, e)).map(|e| entidad_ejercito(e, t)).collect(),
        Ambito::Vista(f) => estado.ejercitos.values().filter(|e| e.columna.faccion == Some(f) && s.ve(e.posicion_en(t))).map(|e| entidad_ejercito(e, t)).collect(),
        Ambito::Asentamiento(id) => plaza(&id).into_iter().collect(),
        Ambito::Admin => estado.ejercitos.values().map(|e| entidad_ejercito(e, t)).chain(estado.asentamientos.keys().filter_map(plaza)).collect(),
        // T3b y T3c.
        Ambito::Faccion(_) | Ambito::Campamento(_) => Vec::new(),
    };
    pb::Foto { ambito: Some(a.a_contrato()), version, entidades, niebla: None }
}

/// Por qué se revoca un ámbito.
fn motivo(n: &Nucleo, a: Ambito) -> &'static str {
    match a {
        Ambito::Asentamiento(p) if !n.estado().asentamientos.contains_key(&p) => "ya_no_existe",
        _ => "sin_derecho",
    }
}

/// Lo que cambió en un ámbito para una sesión: las columnas y las plazas tocadas y los eventos, cada uno con la versión que lo produjo.
#[derive(Default)]
pub(super) struct Cubo<'r> {
    ejercitos: BTreeSet<u64>,
    asentamientos: BTreeSet<u64>,
    eventos: Vec<(u64, &'r EventoNucleo)>,
}

/// La `Delta` `desde → hasta` de un ámbito con lo que cambió. En `VISTA`, la columna que ya no está dentro del rectángulo sale con `QUITA`.
pub(super) fn delta(n: &Nucleo, s: &Sesion, a: Ambito, desde: u64, hasta: u64, cubo: Cubo<'_>) -> pb::Delta {
    let (estado, t) = (n.estado(), n.ahora());
    let columnas = cubo.ejercitos.iter().filter_map(|id| estado.ejercitos.get(id)).map(|e| match a {
        Ambito::Vista(_) if !s.ve(e.posicion_en(t)) => cambio_quita(pb::TipoEntidad::Ejercito, ids::ejercito(e.id)),
        _ => cambio_ejercito(e, t),
    });
    let plazas = cubo.asentamientos.iter().filter_map(|id| estado.asentamientos.get(id)).map(|p| cambio_asentamiento(p, &n.poligono_zona(p.id)));
    let eventos = cubo
        .eventos
        .into_iter()
        .map(|(version, ev)| {
            let heroes = ev.ejercitos.iter().filter_map(|id| estado.ejercitos.get(id)).flat_map(|e| e.columna.participantes.iter().map(|p| ids::heroe(p.heroe))).collect();
            evento_dominio(ev, version, heroes)
        })
        .collect();
    pb::Delta { ambito: Some(a.a_contrato()), desde, hasta, cambios: columnas.chain(plazas).collect(), eventos, niebla: None }
}

/// ¿Cambia algo de lo que decide los derechos de una sesión (el héroe, su Facción, su plaza, su campamento)?
pub(super) fn cambia_derechos(cv: &CambioVersion) -> bool {
    [Clase::Heroe, Clase::Faccion, Clase::Asentamiento, Clase::Campamento].into_iter().any(|c| !cv.tocados.de(c).is_empty())
}

/// Las sesiones abiertas y el índice de quién tiene abierto cada ámbito: ninguna se abre ni se cierra sin pasar por aquí, así que el índice siempre coincide con `abiertos`.
#[derive(Default)]
pub(super) struct Mesa {
    sesiones: HashMap<SesionId, Sesion>,
    por_cliente: HashMap<String, SesionId>,
    suscriptores: BTreeMap<Ambito, BTreeSet<SesionId>>,
}

impl Mesa {
    pub(super) fn get(&self, id: SesionId) -> Option<&Sesion> {
        self.sesiones.get(&id)
    }

    pub(super) fn get_mut(&mut self, id: SesionId) -> Option<&mut Sesion> {
        self.sesiones.get_mut(&id)
    }

    pub(super) fn de_cliente(&self, cliente: &str) -> Option<SesionId> {
        self.por_cliente.get(cliente).copied()
    }

    pub(super) fn ids(&self) -> Vec<SesionId> {
        self.sesiones.keys().copied().collect()
    }

    pub(super) fn sesiones(&self) -> impl Iterator<Item = (SesionId, &Sesion)> {
        self.sesiones.iter().map(|(&id, s)| (id, s))
    }

    pub(super) fn insertar(&mut self, id: SesionId, s: Sesion) {
        for &a in s.abiertos.keys() {
            self.suscriptores.entry(a).or_default().insert(id);
        }
        self.por_cliente.insert(s.cliente.clone(), id);
        self.sesiones.insert(id, s);
    }

    /// Quita la sesión y la baja de todos los ámbitos. Devuelve si existía.
    pub(super) fn quitar(&mut self, id: SesionId) -> bool {
        let Some(s) = self.sesiones.remove(&id) else { return false };
        if self.por_cliente.get(&s.cliente) == Some(&id) {
            self.por_cliente.remove(&s.cliente);
        }
        for a in s.abiertos.keys() {
            self.baja(*a, id);
        }
        true
    }

    fn baja(&mut self, a: Ambito, id: SesionId) {
        if let Some(ids) = self.suscriptores.get_mut(&a) {
            ids.remove(&id);
            if ids.is_empty() {
                self.suscriptores.remove(&a);
            }
        }
    }

    fn abrir(&mut self, id: SesionId, a: Ambito, version: u64) {
        if let Some(s) = self.sesiones.get_mut(&id) {
            s.abiertos.insert(a, version);
            self.suscriptores.entry(a).or_default().insert(id);
        }
    }

    pub(super) fn cerrar(&mut self, id: SesionId, a: Ambito) {
        if self.sesiones.get_mut(&id).is_some_and(|s| s.abiertos.remove(&a).is_some()) {
            self.baja(a, id);
        }
    }

    /// Anota la versión que el cliente tiene ahora de un ámbito abierto.
    pub(super) fn avanzar(&mut self, id: SesionId, a: Ambito, version: u64) {
        if let Some(v) = self.sesiones.get_mut(&id).and_then(|s| s.abiertos.get_mut(&a)) {
            *v = version;
        }
    }

    /// Los héroes que cambiaron pueden ser el de una sesión: el héroe de verdad de una cuenta gana al de campo y no se vuelve a buscar.
    pub(super) fn reasignar_heroes<'r>(&mut self, n: &Nucleo, registros: impl Iterator<Item = &'r CambioVersion>) {
        let cuentas: BTreeSet<&str> = registros.flat_map(|cv| cv.tocados.de(Clase::Heroe)).filter_map(|&h| n.heroe(h)).map(|h| h.cuenta.as_str()).collect();
        for cuenta in cuentas {
            let Some(s) = self.por_cliente.get(cuenta).and_then(|id| self.sesiones.get_mut(id)) else { continue };
            if es_jugador(s.rol) && s.heroe.is_none_or(|h| n.heroe(h).is_none_or(|h| h.sintetico)) {
                s.heroe = heroe_de_cuenta(n, cuenta);
            }
        }
    }

    /// DUDA: `distribuir` lo llama para todas las sesiones en cada versión que toca un héroe, una Facción, una plaza o un campamento (barato por sesión, O(sesiones) por versión: T6 lo mide con 500).
    /// Pone los ámbitos abiertos de la sesión al día con sus derechos: `Revocado` de los que perdió y `Foto` de los que ganó (de todos con `refoto`).
    pub(super) fn sincronizar(&mut self, n: &Nucleo, id: SesionId, refoto: bool, envios: &mut Vec<(SesionId, Difusion)>) {
        let Some(s) = self.sesiones.get(&id) else { return };
        let ahora = derechos(n, s);
        let perdidos: Vec<Ambito> = s.abiertos.keys().filter(|a| !ahora.contains(a)).copied().collect();
        let nuevos: Vec<Ambito> = ahora.iter().filter(|a| refoto || !s.abiertos.contains_key(a)).copied().collect();
        let version = n.version();
        for a in perdidos {
            self.cerrar(id, a);
            envios.push((id, Difusion::Revocado(pb::Revocado { ambito: Some(a.a_contrato()), motivo: motivo(n, a).into() })));
        }
        for a in nuevos {
            let Some(s) = self.sesiones.get(&id) else { return };
            envios.push((id, Difusion::Foto(foto(n, s, a, version))));
            self.abrir(id, a, version);
        }
    }

    /// ¿Debe `(sesión, ámbito)` enterarse de lo que cambió en la versión `v`? Si el cliente ya tiene esa versión del ámbito, no.
    fn recibe(&self, solo: Option<SesionId>, id: SesionId, a: Ambito, v: u64) -> bool {
        solo.is_none_or(|s| s == id) && self.sesiones.get(&id).and_then(|s| s.abiertos.get(&a)).is_some_and(|&k| v > k)
    }

    fn suscritos(&self, a: Ambito) -> impl Iterator<Item = (SesionId, Ambito)> + '_ {
        self.suscriptores.get(&a).into_iter().flatten().map(move |&s| (s, a))
    }

    /// Los ámbitos `HEROE` de las sesiones de las cuentas que participan en la columna.
    fn heroes_de_columna(&self, n: &Nucleo, columna: u64) -> Vec<(SesionId, Ambito)> {
        let Some(e) = n.estado().ejercitos.get(&columna) else { return Vec::new() };
        let cuentas: BTreeSet<&str> = e.columna.participantes.iter().filter_map(|p| n.heroe(p.heroe)).map(|h| h.cuenta.as_str()).collect();
        cuentas.into_iter().filter_map(|c| self.por_cliente.get(c)).filter_map(|&id| Some((id, Ambito::Heroe(self.sesiones.get(&id)?.heroe?)))).filter(|(id, a)| self.suscriptores.get(a).is_some_and(|s| s.contains(id))).collect()
    }

    /// Agrupa por `(sesión, ámbito)` lo que cambió en los registros y le toca al cliente (`solo` = una sola sesión). Una pasada por los registros, no una por sesión.
    pub(super) fn repartir<'r>(&self, n: &Nucleo, registros: impl Iterator<Item = &'r CambioVersion>, solo: Option<SesionId>) -> BTreeMap<(SesionId, Ambito), Cubo<'r>> {
        let estado = n.estado();
        let mut cubos: BTreeMap<(SesionId, Ambito), Cubo<'r>> = BTreeMap::new();
        for cv in registros {
            let v = cv.version;
            for &id in cv.tocados.de(Clase::Ejercito) {
                let vista = estado.ejercitos.get(&id).and_then(|e| e.columna.faccion).into_iter().flat_map(|f| self.suscritos(Ambito::Vista(f)));
                for (s, a) in self.heroes_de_columna(n, id).into_iter().chain(vista).chain(self.suscritos(Ambito::Admin)) {
                    if self.recibe(solo, s, a, v) {
                        cubos.entry((s, a)).or_default().ejercitos.insert(id);
                    }
                }
            }
            for &id in cv.tocados.de(Clase::Asentamiento) {
                for (s, a) in self.suscritos(Ambito::Asentamiento(id)).chain(self.suscritos(Ambito::Admin)) {
                    if self.recibe(solo, s, a, v) {
                        cubos.entry((s, a)).or_default().asentamientos.insert(id);
                    }
                }
            }
            for ev in &cv.eventos {
                let oyen: BTreeSet<(SesionId, Ambito)> = ev.ejercitos.iter().flat_map(|&id| self.heroes_de_columna(n, id)).collect();
                for (s, a) in oyen {
                    if self.recibe(solo, s, a, v) {
                        cubos.entry((s, a)).or_default().eventos.push((v, ev));
                    }
                }
            }
        }
        cubos
    }

    /// Deposita los envíos en el orden dado; las sesiones lentas o cerradas se descartan.
    pub(super) fn entregar(&mut self, envios: Vec<(SesionId, Difusion)>) {
        for (id, d) in envios {
            if self.sesiones.get(&id).is_some_and(|s| !depositar(&s.tx, d)) {
                self.quitar(id);
            }
        }
    }
}
