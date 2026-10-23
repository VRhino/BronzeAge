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
//! Qué lleva cada ámbito (el resto es de T3c-T3e): `HEROE` y `FACCION`, lo que el registro de cambios dice que les pertenece (`proyeccion::audiencia`): del héroe, su ficha con sus
//! loadouts, sus escuadras, las columnas en que participa, las caravanas que le tocan y las batallas en que combate; de la Facción, su ficha, sus relaciones y tecnologías, sus plazas en
//! resumen, su flota, sus pueblos y avanzadas con sus obras, sus trueques, sus órdenes de mercado y sus convocatorias; `ASENTAMIENTO`, la plaza (la ficha de hoy); `VISTA`, las columnas
//! de la Facción dentro del rectángulo; `ADMIN`, todas las columnas y todas las plazas; `CAMPAMENTO`, nada todavía.

use super::depositar;
use crate::estado::{CambioVersion, Clase, EventoNucleo};
use crate::nucleo::Nucleo;
use crate::proyeccion::ambito::Ambito;
use crate::proyeccion::audiencia::heroe_de_sesion;
use crate::proyeccion::{evento_dominio, ids, propios};
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
}

/// Un jugador (Unity o un bot) tiene héroe; el administrador y el servidor de batalla, no.
pub(super) fn es_jugador(rol: pb::TipoCliente) -> bool {
    matches!(rol, pb::TipoCliente::ClienteUnity | pb::TipoCliente::ClienteBot)
}

/// El héroe de una cuenta: el de verdad si lo hay y, si no, el primer héroe de campo.
pub(super) fn heroe_de_cuenta(n: &Nucleo, cuenta: &str) -> Option<u64> {
    heroe_de_sesion(n.estado(), cuenta)
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

/// ¿Puede el cliente que tiene el ámbito en la versión `k` seguir con él por deltas? El registro tiene que alcanzar `k`. `HEROE`, `FACCION` y `ADMIN` se reanudan siempre así: el registro
/// dice a qué ámbito pertenece cada entidad en cada versión (también cuando el héroe no lo tenía abierto), así que los cambios de `k` a hoy, `QUITA` incluido, son exactos aunque el héroe
/// haya salido y vuelto (siempre que `k` no sea anterior a las pertenencias: `Nucleo::pertenencias_desde`). `ASENTAMIENTO` y `CAMPAMENTO` no: su contenido cambia sin que el registro lo diga (el paso de minuto de la plaza) y se ganan y se pierden con el sitio del héroe,
/// así que si el héroe cambió desde `k` el cliente no sabe qué descartó (foto). DUDA: conservador, también fuerza foto cualquier cambio del héroe que no sea de sitio; T3c lo afina.
pub(super) fn reanudable(n: &Nucleo, s: &Sesion, a: Ambito, k: u64) -> bool {
    let Some(mut registros) = n.cambios_desde(k) else { return false };
    match a {
        Ambito::Heroe(_) | Ambito::Faccion(_) => k >= n.pertenencias_desde(),
        Ambito::Admin => true,
        _ => s.heroe.is_some_and(|h| registros.all(|cv| !cv.tocados.contiene(Clase::Heroe, h))),
    }
}

/// La `Foto` de un ámbito en la versión `version`.
pub(super) fn foto(n: &Nucleo, s: &Sesion, a: Ambito, version: u64) -> pb::Foto {
    let (estado, t) = (n.estado(), n.ahora());
    let de = |clase: Clase, ids: Vec<u64>| ids.into_iter().flat_map(|id| propios::proyectar(n, s.heroe, clase, id).existen).map(|(_, _, cuerpo)| pb::Entidad { cuerpo: Some(cuerpo) }).collect::<Vec<_>>();
    let entidades = match a {
        Ambito::Heroe(_) | Ambito::Faccion(_) => n.entidades_de(a).into_iter().flat_map(|(clase, id)| de(clase, vec![id])).collect(),
        Ambito::Vista(f) => de(Clase::Ejercito, estado.ejercitos.values().filter(|e| e.columna.faccion == Some(f) && s.ve(e.posicion_en(t))).map(|e| e.id).collect()),
        Ambito::Asentamiento(id) => de(Clase::Asentamiento, vec![id]),
        Ambito::Admin => de(Clase::Ejercito, estado.ejercitos.keys().copied().collect()).into_iter().chain(de(Clase::Asentamiento, estado.asentamientos.keys().copied().collect())).collect(),
        // T3c.
        Ambito::Campamento(_) => Vec::new(),
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

/// Lo que cambió en un ámbito para una sesión: las entidades tocadas (`true` si pertenecen al ámbito, `false` si dejaron de pertenecerle; en un mismo tramo manda la última versión) y los
/// eventos, cada uno con la versión que lo produjo.
#[derive(Default)]
pub(super) struct Cubo<'r> {
    entidades: BTreeMap<(Clase, u64), bool>,
    eventos: Vec<(u64, &'r EventoNucleo)>,
}

/// La `Delta` `desde → hasta` de un ámbito con lo que cambió. Una entidad que ya no es del ámbito, o que ya no existe, sale con `QUITA`; en `VISTA`, también la columna que ya no está dentro
/// del rectángulo.
pub(super) fn delta(n: &Nucleo, s: &Sesion, a: Ambito, desde: u64, hasta: u64, cubo: Cubo<'_>) -> pb::Delta {
    let (estado, t) = (n.estado(), n.ahora());
    let cambios = cubo.entidades.iter().flat_map(|(&(clase, id), &pertenece)| {
        let a_la_vista = !matches!(a, Ambito::Vista(_)) || estado.ejercitos.get(&id).is_some_and(|e| s.ve(e.posicion_en(t)));
        propios::cambios(n, s.heroe, clase, id, pertenece && a_la_vista)
    });
    let eventos = cubo
        .eventos
        .into_iter()
        .map(|(version, ev)| {
            let heroes = ev.ejercitos.iter().filter_map(|id| estado.ejercitos.get(id)).flat_map(|e| e.columna.participantes.iter().map(|p| ids::heroe(p.heroe))).collect();
            evento_dominio(ev, version, heroes)
        })
        .collect();
    pb::Delta { ambito: Some(a.a_contrato()), desde, hasta, cambios: cambios.collect(), eventos, niebla: None }
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
        let mut cubos: BTreeMap<(SesionId, Ambito), Cubo<'r>> = BTreeMap::new();
        for cv in registros {
            let v = cv.version;
            // Los ámbitos de jugador reciben lo que el registro dice que les pertenece (`ahora`) y la baja de lo que dejó de pertenecerles (`quitan`).
            for p in &cv.pertenencias {
                for (ambitos, pertenece) in [(&p.ahora, true), (&p.quitan, false)] {
                    for &a in ambitos {
                        for (s, a) in self.suscritos(a) {
                            if self.recibe(solo, s, a, v) {
                                cubos.entry((s, a)).or_default().entidades.insert((p.clase, p.id), pertenece);
                            }
                        }
                    }
                }
            }
            // La plaza llega a quien está dentro y la administración lo ve todo: una columna o una plaza que ya no existe sale con `QUITA`.
            let admin = |clase: Clase, ids: &[u64]| ids.iter().flat_map(|&id| self.suscritos(Ambito::Admin).map(move |sa| (clase, id, sa))).collect::<Vec<_>>();
            let plazas = cv.tocados.de(Clase::Asentamiento).iter().flat_map(|&id| self.suscritos(Ambito::Asentamiento(id)).map(move |sa| (Clase::Asentamiento, id, sa)));
            for (clase, id, (s, a)) in admin(Clase::Ejercito, cv.tocados.de(Clase::Ejercito)).into_iter().chain(admin(Clase::Asentamiento, cv.tocados.de(Clase::Asentamiento))).chain(plazas) {
                if self.recibe(solo, s, a, v) {
                    cubos.entry((s, a)).or_default().entidades.insert((clase, id), true);
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
