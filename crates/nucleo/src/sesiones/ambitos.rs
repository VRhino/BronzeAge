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
//! Qué lleva cada ámbito (el resto es de T3d-T3e): `HEROE` y `FACCION`, lo que el registro de cambios dice que les pertenece (`proyeccion::audiencia`): del héroe, su ficha con sus
//! loadouts, sus escuadras, las columnas en que participa, las caravanas que le tocan y las batallas en que combate; de la Facción, su ficha, sus relaciones y tecnologías, sus plazas en
//! resumen, su flota, sus pueblos y avanzadas con sus obras, sus trueques, sus órdenes de mercado y sus convocatorias; `ASENTAMIENTO`, la plaza completa vista desde dentro
//! (`proyeccion::interior`) con los pueblos de la plaza, y `CAMPAMENTO`, el campamento de mercenarios completo (T3c); `VISTA`, el mundo que ve su Facción (o él, sin bandera) dentro del rectángulo,
//! filtrado por la niebla (T3d, abajo); `ADMIN`, todo el mundo sin niebla ni redacción (T3e, abajo).
//!
//! La plaza viaja como **cambio parcial con máscara de campos** (decisión 2): la sesión recuerda la plaza tal como el cliente la tiene ([`Sesion::plaza`]) y cada `Delta` lleva solo los campos que
//! cambiaron respecto de ella (el paso de minuto: el almacén y la población; una obra: los edificios...). Una reanudación, que no sabe qué tiene el cliente, la manda entera; el campamento
//! y todo lo demás viajan completos. Una `Delta` sin nada que contar no se manda.
//!
//! **El interior tiene su propia secuencia** (decisión del autor 2026-10-23): lo que cambia el paso de minuto dentro de la plaza no sube la versión del mundo ni entra en su registro de cambios
//! (con o sin sesiones el contador `version` y el hash son los mismos) sino que avanza `Nucleo::version_interior`, que viaja en `Asentamiento.version_interior`. La sesión compara la de la plaza que
//! recuerda con la de hoy ([`Mesa::repartir_interior`]) y manda la `Delta` del interior sin que el mundo haya avanzado.
//!
//! **La `VISTA` no sale del registro de cambios** (T3d): lo que se ve cambia sin que nada del mundo cambie (una columna que anda entra en el alcance de un ojo, a una columna ajena se le acaba el tramo
//! que se le mandó). La sesión recuerda qué entidades tiene el cliente en su vista y con qué sello ([`Mapa`]) y [`Mesa::reconciliar_vistas`] le manda lo que cambió respecto de lo que se ve ahora
//! (`proyeccion::vista::Mirada`, una por perspectiva para todas sus sesiones): tras cada reparto y en cada muestreo de 250 ms de mundo. La `NieblaProyectada` viaja con la `Foto` y con la `Delta`
//! en que cambia; una `Delta` solo de la vista va de la versión que tiene el cliente a la del mundo (puede ser la misma).
//!
//! **`ADMIN` sale del registro de cambios** (T3e), como `HEROE` y `FACCION` pero sin pertenencias: todo es suyo, así que lo que le toca de cada versión es lo que la versión tocó (de todas las clases;
//! el interior es su plaza), más las caravanas que lleva una columna tocada (se mueven con ella) y todos los hechos. No hace falta reconciliar por sellos como en `VISTA`: en `ADMIN` nada entra ni sale
//! sin que cambie el estado (no hay niebla ni rectángulo) y el registro está auditado (ninguna entidad cambia sin quedar en él), mientras que reconciliar recorrería el mundo entero en cada muestreo.
//! Cada entidad viaja con las proyecciones de siempre ([`Espectador::Admin`]: la batalla entera con su asignación sin tokens, la convocatoria con todas sus peticiones) y las plazas con su
//! interior, su máscara y su secuencia interior como en `ASENTAMIENTO` (la sesión las recuerda todas, [`Sesion::plazas`]). Nunca viaja un token de batalla. Al reanudar, la secuencia interior de cada
//! plaza la da el cliente con la clave de su ámbito (`ASENTAMIENTO:as-N`) en `Hola.versiones_interiores_conocidas`.
//!
//! DUDA: lo que una entidad enseña de otra (el `dentro_de` de una columna, los ciudadanos de una Facción) llega cuando la versión toca la entidad, como en `HEROE` y `FACCION`; y la posición de una
//! caravana en marcha la interpola el cliente con su plan (no se manda en cada muestreo).
// ponytail: la sesión de administración recuerda todas las plazas proyectadas (unos KB cada una) para calcular su máscara; con miles de plazas, recordar solo su secuencia y mandarlas enteras.

use super::depositar;
use crate::estado::{CambioVersion, Clase, EventoNucleo};
use crate::nucleo::Nucleo;
use crate::proyeccion::ambito::Ambito;
use crate::proyeccion::audiencia::{heroe_de_sesion, principal};
use crate::proyeccion::propios::Espectador;
use crate::proyeccion::vista::{cambio_de_niebla, Clave, Mirada, Perspectiva, Sello};
use crate::proyeccion::eventos::{batalla_de, evento_dominio};
use crate::proyeccion::{cambio_quita, interior, propios, vista};
use base::Punto;
use contrato::v1 as pb;
use militar::presencia::Ubicacion;
use servidor::{Difusion, SesionId};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use tokio::sync::mpsc;
use vision::Rect;

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
    /// Las plazas con su interior tal como el cliente las tiene: la de su `ASENTAMIENTO` abierto o, en `ADMIN`, todas. Contra ellas se calcula la máscara de campos de cada cambio.
    pub(super) plazas: BTreeMap<u64, pb::Asentamiento>,
    /// Lo que el cliente tiene de su `VISTA` abierta.
    pub(super) mapa: Option<Mapa>,
    /// Una sesión de administración de un moderador (`servidor::RolTecnico::Moderador`): ve `ADMIN` y no da órdenes (5E).
    pub(super) moderador: bool,
    /// Los alijos que el cliente tiene en su `HEROE` (6G: los que el héroe tiene a la vista, que cambian al andar su columna sin que nada del mundo cambie). `None`: no se sabe (reanudó por deltas).
    pub(super) alijos: Option<BTreeSet<u32>>,
    /// Los informes de inspección vigentes que el cliente tiene en su `HEROE`, por id. Al caducar uno sale con `QUITA` en la reconciliación siguiente (caducar no cambia el mundo). `None`: no se
    /// sabe (reanudó por deltas): se toma lo vigente de ahora, sin quitar nada.
    pub(super) informes: Option<BTreeSet<String>>,
    /// Lo que el cliente tiene de lo que cobra su `CAMPAMENTO` (los precios del mercado y, al residente, los de reclutar: `interior::lo_que_cobra`), que cambia sin que cambie la versión.
    /// `None`: no se sabe (reanudó por deltas): se manda el campamento.
    pub(super) cobra: Option<Cobra>,
    /// Lo que el cliente de `ADMIN` tiene de lo que no pasa por el registro de cambios (6G). `None`: no se sabe (reanudó por deltas).
    pub(super) admin_visto: Option<AdminVisto>,
}

/// Lo que cobra un campamento a quien está dentro, tal como lo tiene el cliente (`interior::lo_que_cobra`).
pub(super) type Cobra = (BTreeMap<String, i64>, Vec<pb::TropaDeCampamento>);

/// Lo que no pasa por el registro de cambios y `ADMIN` ve igual (6G): el estado de tecnología del mundo con sus contadores (los suma el paso de minuto, que no sube la versión; sin las tecnologías de
/// cada Facción, que viajan con ella), los Aedas itinerantes (su viaje no sube la versión) y, desde la paridad del 2026-10-27, lo que no tiene clase en el registro (la memoria de la niebla de cada
/// Facción, sus Miradas, las zonas fundidas y la red de caminos: `vista::huellas_de_admin`), por su huella. Se compara en cada reparto con lo de ahora.
pub(super) struct AdminVisto {
    tecnologia: tecnologia::EstadoTecnologia,
    aedas: Vec<tecnologia::itinerantes::AedaItinerante>,
    sin_clase: BTreeMap<vista::ClaveAdmin, u64>,
}

impl AdminVisto {
    fn de(n: &Nucleo) -> Self {
        let t = &n.estado().tecnologia;
        let tecnologia = tecnologia::EstadoTecnologia {
            era: t.era,
            era_desde: t.era_desde,
            contadores: t.contadores,
            restos: [0; 24],
            logros: t.logros.clone(),
            primeros: t.primeros.clone(),
            cantadas: t.cantadas,
            por_faccion: BTreeMap::new(),
        };
        AdminVisto { tecnologia, aedas: n.estado().aedas.clone(), sin_clase: vista::huellas_de_admin(n) }
    }

    /// ¿Lo ve igual que ahora? (Sin copiar nada: es lo que se pregunta en cada reparto.)
    fn igual_tecnologia(&self, n: &Nucleo) -> bool {
        let (v, t) = (&self.tecnologia, &n.estado().tecnologia);
        v.era == t.era && v.era_desde == t.era_desde && v.contadores == t.contadores && v.logros == t.logros && v.primeros == t.primeros && v.cantadas == t.cantadas
    }
}

/// Lo que el cliente tiene de su `VISTA`: cada entidad con el sello con que se le mandó y la última niebla (la de su perspectiva, compartida: `Mesa::nieblas`).
pub(super) struct Mapa {
    tiene: BTreeMap<Clave, Sello>,
    niebla: Option<Arc<pb::NieblaProyectada>>,
}

impl Sesion {
    pub(super) fn nueva(n: &Nucleo, cliente: String, rol: pb::TipoCliente, tx: mpsc::Sender<Difusion>) -> Self {
        let heroe = es_jugador(rol).then(|| heroe_de_cuenta(n, &cliente)).flatten();
        Sesion { cliente, rol, tx, heroe, abiertos: BTreeMap::new(), vista: None, plazas: BTreeMap::new(), mapa: None, moderador: false, alijos: None, informes: None, cobra: None, admin_visto: None }
    }

    pub(super) fn es_admin(&self) -> bool {
        self.rol == pb::TipoCliente::ClienteAdminWeb
    }

    /// Quién mira en las proyecciones: la administración o el héroe de la sesión.
    fn espectador(&self) -> Espectador {
        if self.es_admin() {
            Espectador::Admin
        } else {
            Espectador::Heroe(self.heroe)
        }
    }

    /// El ámbito por el que le llega el interior de la plaza `p` que recuerda.
    fn ambito_de_plaza(&self, p: u64) -> Ambito {
        if self.es_admin() {
            Ambito::Admin
        } else {
            Ambito::Asentamiento(p)
        }
    }

    /// El rectángulo de su vista.
    fn rect(&self) -> Option<Rect> {
        self.vista.map(|(min, max)| Rect { min, max })
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
    d.extend(h.faccion.map(Ambito::Faccion));
    // La vista es de su Facción o, sin bandera, la suya (su niebla personal).
    if s.vista.is_some() {
        d.insert(Ambito::Vista(h.faccion.map_or(Perspectiva::Heroe(h.heroe.id), Perspectiva::Faccion)));
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
    // Las batallas en que combate (el suyo o el héroe de campo de su cuenta), también las cerradas mientras el núcleo las conserve: así ve el resultado.
    let e = n.estado();
    d.extend(n.batallas().filter(|b| b.heroes_que_combaten().any(|c| e.militar.heroes.get(&c).is_some_and(|x| principal(e, x) == Some(h.heroe.id)))).map(|b| Ambito::Batalla(b.id)));
    d
}

/// ¿Puede el cliente que tiene el ámbito en la versión `k` seguir con él por deltas? El registro tiene que alcanzar `k`. Todos los ámbitos que se abren al conectar se reanudan así: el registro dice qué
/// entidad cambió en cada versión y a qué ámbito pertenecía (también cuando el héroe no lo tenía abierto), así que los cambios de `k` a hoy, `QUITA` incluido, son exactos aunque el héroe haya salido y
/// vuelto (siempre que `k` no sea anterior a las pertenencias: `Nucleo::pertenencias_desde`). Desde T3c eso vale también para `ASENTAMIENTO` y `CAMPAMENTO`: quién entra y sale marca el interior en el registro.
/// El interior de la plaza que cambia el paso de minuto no está en el registro del mundo sino en su propia secuencia (`Nucleo::version_interior`): `ki` es la que el cliente conserva (0 si no trae
/// ninguna) y no puede ser mayor que la de hoy (un cliente «del futuro», p. ej. tras recuperar un registro más corto que el proceso vivo, recibe la foto). Que sea menor no cuesta la reanudación por deltas:
/// la plaza se manda entera, sin hacer falta un registro del interior. `VISTA` no se reanuda: se abre con `CambiarVista`.
pub(super) fn reanudable(n: &Nucleo, a: Ambito, k: u64, ki: u64) -> bool {
    n.cambios_desde(k).is_some()
        && match a {
            Ambito::Asentamiento(p) => k >= n.pertenencias_desde() && n.version_interior(p).is_some_and(|actual| ki <= actual),
            Ambito::Heroe(_) | Ambito::Faccion(_) | Ambito::Batalla(_) | Ambito::Campamento(_) => k >= n.pertenencias_desde(),
            Ambito::Admin => true,
            Ambito::Vista(_) => false,
        }
}

/// Las plazas que el cliente tiene con su interior cuando el ámbito está al día: la de `ASENTAMIENTO` o, en `ADMIN`, todas.
pub(super) fn plazas_de(n: &Nucleo, a: Ambito) -> BTreeMap<u64, pb::Asentamiento> {
    let ids: Vec<u64> = match a {
        Ambito::Asentamiento(p) => vec![p],
        Ambito::Admin => n.estado().asentamientos.keys().copied().collect(),
        _ => Vec::new(),
    };
    ids.into_iter().filter_map(|p| Some((p, interior::asentamiento(n, p)?))).collect()
}

/// La `Foto` de un ámbito en la versión `version`.
pub(super) fn foto(n: &Nucleo, s: &Sesion, a: Ambito, version: u64) -> pb::Foto {
    let de = |(clase, id): (Clase, u64)| propios::proyectar(n, s.espectador(), clase, id).existen.into_iter().map(|(_, _, cuerpo)| pb::Entidad { cuerpo: Some(cuerpo) });
    let plazas = || plazas_de(n, a).into_values().map(|p| pb::Entidad { cuerpo: Some(pb::entidad::Cuerpo::Asentamiento(p)) });
    let entidad = |c: pb::entidad::Cuerpo| pb::Entidad { cuerpo: Some(c) };
    let entidades = match a {
        // El héroe lleva además la forma pública del estado de tecnología (es de todos) y los alijos que tiene a la vista (6G).
        Ambito::Heroe(h) => n
            .entidades_de(a)
            .into_iter()
            .chain([(Clase::Tecnologia, 0)])
            .flat_map(de)
            .chain(alijos_a_la_vista(n, h).into_iter().filter_map(|i| propios::alijo(n, i)).map(|x| entidad(pb::entidad::Cuerpo::Alijo(x))))
            .collect(),
        Ambito::Faccion(_) | Ambito::Batalla(_) => n.entidades_de(a).into_iter().flat_map(de).collect(),
        Ambito::Vista(p) => return foto_de_vista(n, s, p, version).0,
        Ambito::Asentamiento(_) => plazas().chain(n.entidades_de(a).into_iter().flat_map(de)).collect(),
        // Y lo que no es una clase del registro de cambios: los Aedas itinerantes enteros (6G), la memoria de la niebla de cada Facción, sus Miradas, las zonas y la red (paridad, 2026-10-27).
        Ambito::Admin => {
            let aedas = n.estado().aedas.iter().map(|x| entidad(pb::entidad::Cuerpo::AedaItinerante(propios::aeda_itinerante(x))));
            let sin_clase = vista::huellas_de_admin(n).into_keys().filter_map(|k| vista::de_admin(n, &k)).map(entidad);
            plazas().chain(propios::todo_menos_plazas(n).into_iter().flat_map(de)).chain(aedas).chain(sin_clase).collect()
        }
        Ambito::Campamento(c) => de((Clase::Campamento, u64::from(c))).collect(),
    };
    pb::Foto { ambito: Some(a.a_contrato(&|b| n.uuid_de_batalla(b))), version, entidades, niebla: None }
}

/// Los alijos que el héroe `h` tiene a la vista ahora (6G; ninguno si no puede buscarlos).
fn alijos_a_la_vista(n: &Nucleo, h: u64) -> Vec<u32> {
    n.alijos_a_la_vista(h, n.ahora())
}

/// La `Foto` de la `VISTA` desde la perspectiva `p` (con su `NieblaProyectada`) y lo que el cliente tendrá con ella.
fn foto_de_vista(n: &Nucleo, s: &Sesion, p: Perspectiva, version: u64) -> (pb::Foto, Mapa) {
    let m = Mirada::de(n, p);
    let rect = s.rect().expect("la vista se abre con su rectángulo");
    let (mut entidades, mut tiene) = (Vec::new(), BTreeMap::new());
    for x in m.contenido(s.heroe, rect, &BTreeMap::new()) {
        if let Some(c) = x.cuerpo() {
            entidades.push(pb::Entidad { cuerpo: Some(c) });
            tiene.insert(x.clave, x.sello);
        }
    }
    let niebla = m.niebla().cloned();
    (pb::Foto { ambito: Some(Ambito::Vista(p).a_contrato(&|b| n.uuid_de_batalla(b))), version, entidades, niebla: niebla.clone() }, Mapa { tiene, niebla: niebla.map(Arc::new) })
}

/// Por qué se revoca un ámbito.
fn motivo(n: &Nucleo, a: Ambito) -> &'static str {
    match a {
        Ambito::Asentamiento(p) if !n.estado().asentamientos.contains_key(&p) => "ya_no_existe",
        Ambito::Batalla(b) if n.batalla(b).is_none() => "ya_no_existe",
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

impl Cubo<'_> {
    /// El interior de la plaza `id` avanzó (pasos de minuto, que no suben la versión del mundo): la plaza y sus pueblos activos (su almacén local es parte del paso) entran en lo que hay que contar.
    ///
    /// DUDA: los pueblos van completos en cada delta del interior aunque su almacén no haya cambiado (la sesión no recuerda cómo los tiene el cliente, solo la plaza): unas decenas de bytes por pueblo y
    /// minuto. Si el tráfico de T6 lo pide, la sesión puede recordar también el último pueblo mandado y comparar. Y una delta solo del interior lleva como `hasta` la versión del mundo de ese momento
    /// (que puede ser igual a `desde`): el contrato 05 §7 lo dice, pero ningún cliente real lo ha leído aún.
    pub(super) fn interior_de(&mut self, n: &Nucleo, id: u64) {
        self.entidades.insert((Clase::Asentamiento, id), true);
        self.entidades.extend(n.pueblos_de(id).iter().filter(|p| p.activo()).map(|p| ((Clase::Pueblo, p.id), true)));
    }
}

/// Lo que se manda a una sesión de un ámbito: la `Delta` si hay algo que contar, y cómo quedan las plazas que el cliente tiene con su interior (`None`: ya no la tiene), aunque no haya `Delta`: la plaza
/// que se recuerda avanza su secuencia interior para no volver a compararla hasta el paso siguiente.
pub(super) struct Entrega {
    pub(super) delta: Option<pb::Delta>,
    pub(super) plazas: Vec<(u64, Option<pb::Asentamiento>)>,
}

/// La `Delta` `desde → hasta` de un ámbito con lo que cambió (`hasta` es la versión del mundo: un tramo solo del interior, que no la sube, va de la que tiene el cliente a la misma o a una posterior sin que haya
/// cambiado nada más suyo), y cómo quedan las plazas que el cliente tiene con su interior. Una entidad que ya no es del ámbito, o que ya no existe, sale con
/// `QUITA`. La plaza de `ASENTAMIENTO` y las de `ADMIN` viajan como cambio parcial con máscara contra la que el cliente ya tiene ([`Sesion::plazas`]; si no
/// se sabe, entera). Sin nada que contar no hay `Delta`: un cambio que no mueve nada de lo que ve el cliente (la ficha de la plaza sin cambios) no cuesta un mensaje.
pub(super) fn delta(n: &Nucleo, s: &Sesion, a: Ambito, desde: u64, hasta: u64, cubo: Cubo<'_>) -> Entrega {
    let (mut plazas, mut cambios) = (Vec::new(), Vec::new());
    for (&(clase, id), &pertenece) in &cubo.entidades {
        if clase == Clase::Asentamiento && matches!(a, Ambito::Asentamiento(_) | Ambito::Admin) {
            let (cambio, despues) = interior::cambio_de_plaza(n, id, s.plazas.get(&id));
            cambios.extend(cambio);
            plazas.push((id, despues));
            continue;
        }
        cambios.extend(propios::cambios(n, s.espectador(), clase, id, pertenece));
    }
    let eventos: Vec<pb::EventoDominio> = cubo.eventos.into_iter().map(|(version, ev)| evento_dominio(n, ev, version)).collect();
    let delta = (!cambios.is_empty() || !eventos.is_empty()).then(|| pb::Delta { ambito: Some(a.a_contrato(&|b| n.uuid_de_batalla(b))), desde, hasta, cambios, eventos, niebla: None });
    Entrega { delta, plazas }
}

/// La sesión que reanuda y hasta qué versión oyó ya los hechos: la mayor que declara en `Hola.versiones_conocidas`. Un cliente que tuvo un ámbito al día hasta la versión `k` estaba conectado cuando se
/// repartió `k` (o reanudó entonces), así que recibió los hechos que le tocaban hasta `k` por el ámbito que los oía, aunque ese ámbito ya no lo tenga (la plaza que dejó) y otro que también los oye vaya
/// por detrás (su `FACCION`, que no cambió desde entonces). Sin estado nuevo: lo dice el `Hola` (duda 8 de T5, 4H T6).
///
/// Regla (decisión del autor del 2026-10-27): un cliente que el servidor descarta por lento a mitad de un reparto pudo aplicar la `Delta` de un ámbito y no la de otro con un hecho de esa misma versión: al reanudar declara la mayor y ese
/// hecho no le vuelve (el estado sí: lo que le faltó del otro ámbito llega en su `Delta` de reanudación).
#[derive(Clone, Copy)]
pub(super) struct Reanuda {
    pub(super) sesion: SesionId,
    pub(super) oido: u64,
}

/// ¿Cambia algo de lo que decide los derechos de una sesión (el héroe, su Facción, su plaza, su campamento)?
pub(super) fn cambia_derechos(cv: &CambioVersion) -> bool {
    [Clase::Heroe, Clase::Faccion, Clase::Asentamiento, Clase::Campamento, Clase::Batalla].into_iter().any(|c| !cv.tocados.de(c).is_empty())
}

/// Las sesiones abiertas y el índice de quién tiene abierto cada ámbito: ninguna se abre ni se cierra sin pasar por aquí, así que el índice siempre coincide con `abiertos`.
#[derive(Default)]
pub(super) struct Mesa {
    sesiones: HashMap<SesionId, Sesion>,
    por_cliente: HashMap<String, SesionId>,
    suscriptores: BTreeMap<Ambito, BTreeSet<SesionId>>,
    /// La última niebla de cada perspectiva con alguna `VISTA` abierta, la que tienen sus sesiones al día (4H T6).
    nieblas: BTreeMap<Perspectiva, Arc<pb::NieblaProyectada>>,
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

    /// Las sesiones de servidores de batalla (`CLIENTE_SERVIDOR_BATALLA`): reciben los tickets y no tienen ningún ámbito.
    pub(super) fn servidores(&self) -> impl Iterator<Item = (SesionId, &Sesion)> {
        self.sesiones().filter(|(_, s)| s.rol == pb::TipoCliente::ClienteServidorBatalla)
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

    /// Abre el ámbito `a` en la versión `version`; el de una plaza (y `ADMIN`) recuerda cómo las tiene el cliente (la `Foto` que acaba de recibir).
    fn abrir(&mut self, n: &Nucleo, id: SesionId, a: Ambito, version: u64) {
        if let Some(s) = self.sesiones.get_mut(&id) {
            s.abiertos.insert(a, version);
            s.plazas.extend(plazas_de(n, a));
            self.suscriptores.entry(a).or_default().insert(id);
        }
    }

    pub(super) fn cerrar(&mut self, id: SesionId, a: Ambito) {
        if self.sesiones.get_mut(&id).is_some_and(|s| s.abiertos.remove(&a).is_some()) {
            let s = self.sesiones.get_mut(&id).expect("existe");
            match a {
                Ambito::Asentamiento(p) => drop(s.plazas.remove(&p)),
                Ambito::Admin => s.plazas.clear(),
                Ambito::Vista(_) => s.mapa = None,
                _ => {}
            }
            self.baja(a, id);
        }
    }

    /// Anota la versión que el cliente tiene ahora de un ámbito abierto y cómo quedan las plazas que tiene con su interior.
    pub(super) fn avanzar(&mut self, id: SesionId, a: Ambito, version: u64, plazas: Vec<(u64, Option<pb::Asentamiento>)>) {
        let Some(s) = self.sesiones.get_mut(&id) else { return };
        let Some(v) = s.abiertos.get_mut(&a) else { return };
        *v = version;
        for (p, x) in plazas {
            match x {
                Some(x) => drop(s.plazas.insert(p, x)),
                None => drop(s.plazas.remove(&p)),
            }
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
            envios.push((id, Difusion::Revocado(pb::Revocado { ambito: Some(a.a_contrato(&|b| n.uuid_de_batalla(b))), motivo: motivo(n, a).into() })));
        }
        for a in nuevos {
            let Some(s) = self.sesiones.get(&id) else { return };
            let (f, mapa) = match a {
                Ambito::Vista(p) => {
                    let (f, m) = foto_de_vista(n, s, p, version);
                    (f, Some(m))
                }
                _ => (foto(n, s, a, version), None),
            };
            envios.push((id, Difusion::Foto(f)));
            self.abrir(n, id, a, version);
            self.recordar_foto(n, id, a);
            if let (Some(m), Some(s)) = (mapa, self.sesiones.get_mut(&id)) {
                s.mapa = Some(m);
            }
        }
    }

    /// ¿Debe `(sesión, ámbito)` enterarse de lo que cambió en la versión `v`? Si el cliente ya tiene esa versión del ámbito (o es otra la sesión que reanuda), no.
    fn recibe(&self, solo: Option<Reanuda>, id: SesionId, a: Ambito, v: u64) -> bool {
        solo.is_none_or(|r| r.sesion == id) && self.sesiones.get(&id).and_then(|s| s.abiertos.get(&a)).is_some_and(|&k| v > k)
    }

    fn suscritos(&self, a: Ambito) -> impl Iterator<Item = (SesionId, Ambito)> + '_ {
        self.suscriptores.get(&a).into_iter().flatten().map(move |&s| (s, a))
    }

    /// A quién se cuenta un hecho de la versión `v` (contrato 05 §8, 4H T5) y por qué ámbito: a cada sesión **una sola vez**, por el primero de sus ámbitos abiertos que lo oye y aún no tiene la versión
    /// `v`, en este orden: la plaza a la que se atribuye (`ASENTAMIENTO`, quien está dentro), la batalla del hecho (`BATALLA`, quien combate), los héroes nombrados (`HEROE`), las Facciones que lo oyen
    /// (`FACCION`) y, para un hecho de batalla, la `VISTA` de quien tiene esa batalla en su mapa (la ve bajo la niebla: el hecho no le dice nada que no vea). Un ámbito que se acaba de abrir en este mismo
    /// reparto ya tiene la versión (su `Foto`) pero no el hecho (una `Foto` no lleva hechos): el hecho va por el siguiente que lo oye (4H T6; antes se perdía: el de entrar en la plaza no llegaba a quien
    /// entraba). Un hecho público llega a todo jugador con héroe, por su `HEROE`. Nada más: un hecho sin plaza que nombra a alguien no es de todos (no puede revelar lo que oculta la niebla). La
    /// administración, que lo oye todo, no pasa por aquí.
    fn oyentes(&self, n: &Nucleo, ev: &EventoNucleo, v: u64, solo: Option<Reanuda>) -> BTreeMap<SesionId, Ambito> {
        let e = n.estado();
        let mut oyen = BTreeMap::new();
        let mut oir = |(s, a): (SesionId, Ambito)| {
            if self.recibe(solo, s, a, v) {
                oyen.entry(s).or_insert(a);
            }
        };
        let batalla = batalla_de(ev);
        let heroes = ev.heroes.iter().filter_map(|h| e.militar.heroes.get(h)).filter_map(|h| principal(e, h)).map(Ambito::Heroe);
        let ambitos = ev.asentamientos.iter().map(|&p| Ambito::Asentamiento(p)).chain(batalla.map(Ambito::Batalla)).chain(heroes).chain(ev.facciones.iter().map(|&f| Ambito::Faccion(f)));
        for a in ambitos {
            self.suscritos(a).for_each(&mut oir);
        }
        if let Some(b) = batalla {
            let clave = (pb::TipoEntidad::Batalla as i32, n.uuid_de_batalla(b));
            let vistas = self.suscriptores.iter().filter(|(a, _)| matches!(a, Ambito::Vista(_))).flat_map(|(&a, ss)| ss.iter().map(move |&s| (s, a)));
            vistas.filter(|(s, _)| self.sesiones.get(s).and_then(|x| x.mapa.as_ref()).is_some_and(|m| m.tiene.contains_key(&clave))).for_each(&mut oir);
        }
        // Quien está dentro de una plaza del hecho lo oye por su `HEROE`, sea de su Facción o visitante, aliado o neutral (decisión del autor, 2026-10-24).
        if !ev.asentamientos.is_empty() {
            let dentro = |h: u64| e.militar.heroes.get(&h).is_some_and(|x| matches!(x.heroe.presencia.ubicacion, Ubicacion::Asentamiento(p) if ev.asentamientos.contains(&u64::from(p))));
            self.suscriptores.iter().filter(|(a, _)| matches!(a, Ambito::Heroe(h) if dentro(*h))).flat_map(|(&a, ss)| ss.iter().map(move |&s| (s, a))).for_each(&mut oir);
        }
        if ev.publico {
            self.suscriptores.iter().filter(|(a, _)| matches!(a, Ambito::Heroe(_))).flat_map(|(&a, ss)| ss.iter().map(move |&s| (s, a))).for_each(&mut oir);
        }
        oyen
    }

    /// Agrupa por `(sesión, ámbito)` lo que cambió en los registros y le toca al cliente (`solo`: la sesión que reanuda, y nadie más). Una pasada por los registros, no una por sesión.
    pub(super) fn repartir<'r>(&self, n: &Nucleo, registros: impl Iterator<Item = &'r CambioVersion>, solo: Option<Reanuda>) -> BTreeMap<(SesionId, Ambito), Cubo<'r>> {
        let mut cubos: BTreeMap<(SesionId, Ambito), Cubo<'r>> = BTreeMap::new();
        // Sin ningún ámbito abierto no hay a quién contar nada (ni siquiera hace falta buscar los oyentes de los hechos).
        if self.suscriptores.is_empty() {
            return cubos;
        }
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
            // La plaza y el campamento llegan a quien está dentro (la plaza, cuando cambia la ficha o quién entra y sale; el paso de minuto no está aquí: lo reparte `repartir_interior`).
            let plazas = cv.tocados.de(Clase::Asentamiento).iter().chain(cv.tocados.de(Clase::Interior)).flat_map(|&id| self.suscritos(Ambito::Asentamiento(id)).map(move |sa| (Clase::Asentamiento, id, sa)));
            let campamentos = cv.tocados.de(Clase::Campamento).iter().flat_map(|&c| self.suscritos(Ambito::Campamento(c as u32)).map(move |sa| (Clase::Campamento, c, sa)));
            for (clase, id, (s, a)) in plazas.chain(campamentos) {
                if self.recibe(solo, s, a, v) {
                    cubos.entry((s, a)).or_default().entidades.insert((clase, id), true);
                }
            }
            // La forma pública del estado de tecnología es de todos (6G): cuando la versión la toca (un logro, una Era, un canto; pocas veces en una partida), a cada `HEROE`.
            if !cv.tocados.de(Clase::Tecnologia).is_empty() {
                let heroes = self.suscriptores.iter().filter(|(a, _)| matches!(a, Ambito::Heroe(_))).flat_map(|(&a, ss)| ss.iter().map(move |&s| (s, a)));
                for (s, a) in heroes.filter(|&(s, a)| self.recibe(solo, s, a, v)).collect::<Vec<_>>() {
                    cubos.entry((s, a)).or_default().entidades.insert((Clase::Tecnologia, 0), true);
                }
            }
            // La administración lo ve todo: lo que la versión tocó, de todas las clases (el interior es su plaza; lo que ya no existe sale con `QUITA`), las caravanas que lleva una columna tocada y todos
            // los hechos.
            let admins: Vec<SesionId> = self.suscritos(Ambito::Admin).map(|(s, _)| s).filter(|&s| self.recibe(solo, s, Ambito::Admin, v)).collect();
            if !admins.is_empty() {
                let fundacion = &n.estado().fundacion;
                let adjuntas = cv
                    .tocados
                    .de(Clase::Ejercito)
                    .iter()
                    .filter_map(|id| n.estado().ejercitos.get(id))
                    .flat_map(|x| x.caravanas_adjuntas.iter().copied().chain(fundacion.de_columna(x.id)).map(|c| (Clase::Caravana, c)));
                // El estado de tecnología no: `ADMIN` lo ve entero, con los contadores que no marcan nada, y lo pone al día `admin_al_dia` en cada reparto.
                let tocadas: Vec<(Clase, u64)> = Clase::TODAS
                    .into_iter()
                    .filter(|&c| c != Clase::Tecnologia)
                    .flat_map(|c| cv.tocados.de(c).iter().map(move |&id| (if c == Clase::Interior { Clase::Asentamiento } else { c }, id)))
                    .chain(adjuntas)
                    .collect();
                for s in admins {
                    let cubo = cubos.entry((s, Ambito::Admin)).or_default();
                    cubo.entidades.extend(tocadas.iter().map(|&k| (k, true)));
                    cubo.eventos.extend(cv.eventos.iter().map(|ev| (v, ev)));
                }
            }
            // Los hechos que el cliente que reanuda ya oyó no vuelven (`Reanuda::oido`).
            if solo.is_some_and(|r| v <= r.oido) {
                continue;
            }
            for ev in &cv.eventos {
                for (s, a) in self.oyentes(n, ev, v, solo) {
                    cubos.entry((s, a)).or_default().eventos.push((v, ev));
                }
            }
        }
        cubos
    }

    /// Suma a los cubos el interior de las plazas que avanzaron su secuencia (pasos de minuto: no suben la versión del mundo, así que no están en ningún registro de cambios) desde la que cada cliente tiene,
    /// que es la de la plaza que se recuerda ([`Sesion::plazas`]: la de su `ASENTAMIENTO` o, en `ADMIN`, todas). Una consulta por plaza recordada; la proyección, solo si hay algo nuevo.
    pub(super) fn repartir_interior<'r>(&self, n: &Nucleo, cubos: &mut BTreeMap<(SesionId, Ambito), Cubo<'r>>) {
        for (&s, x) in &self.sesiones {
            for (&plaza, p) in &x.plazas {
                if n.version_interior(plaza).is_some_and(|actual| p.version_interior < actual) {
                    cubos.entry((s, x.ambito_de_plaza(plaza))).or_default().interior_de(n, plaza);
                }
            }
        }
    }

    /// Pone al día la `VISTA` de cada sesión que la tiene abierta con lo que se ve ahora: `ACTUALIZA` de lo que entra o cambió (su sello), `QUITA` de lo que ya no se ve o salió del rectángulo y la niebla
    /// si cambió, en una sola `Delta` de la versión que tiene el cliente a la de ahora. Lo que se ve se calcula una vez por perspectiva (`Mirada`) para todas sus sesiones, y la niebla también: todas
    /// comparten la última de su perspectiva (`Mesa::nieblas`), así que compararla es comparar punteros, y a quien tiene la anterior le llega solo lo que cambió (`vista::cambio_de_niebla`, calculado una
    /// vez por perspectiva).
    // ponytail: cada perspectiva recorre todas las entidades del mundo y cada sesión las de su rectángulo en cada muestreo (O(entidades) por sesión); una rejilla espacial si el mundo crece mucho más.
    pub(super) fn reconciliar_vistas(&mut self, n: &Nucleo, solo: Option<SesionId>, envios: &mut Vec<(SesionId, Difusion)>) {
        let abiertas: Vec<(SesionId, Perspectiva)> = self
            .suscriptores
            .iter()
            .filter_map(|(a, ss)| if let Ambito::Vista(p) = a { Some(ss.iter().filter(|s| solo.is_none_or(|x| x == **s)).map(|&s| (s, *p)).collect::<Vec<_>>()) } else { None })
            .flatten()
            .collect();
        let mut miradas: BTreeMap<Perspectiva, (Mirada<'_>, Option<Arc<pb::NieblaProyectada>>)> = BTreeMap::new();
        // El cambio de la niebla de cada perspectiva para las sesiones que tienen la anterior de la perspectiva (casi todas): se calcula una vez.
        let mut cambios_de_niebla: BTreeMap<Perspectiva, pb::NieblaProyectada> = BTreeMap::new();
        let v = n.version();
        for (id, p) in abiertas {
            let (m, niebla) = miradas.entry(p).or_insert_with(|| {
                let m = Mirada::de(n, p);
                let niebla = m.niebla().map(|nueva| self.nieblas.get(&p).filter(|vieja| ***vieja == *nueva).cloned().unwrap_or_else(|| Arc::new(nueva.clone())));
                (m, niebla)
            });
            let Some(s) = self.sesiones.get_mut(&id) else { continue };
            let (Some(rect), Some(mapa)) = (s.rect(), s.mapa.as_ref()) else { continue };
            let a = Ambito::Vista(p);
            let (mut cambios, mut tiene) = (Vec::new(), BTreeMap::new());
            for x in m.contenido(s.heroe, rect, &mapa.tiene) {
                if mapa.tiene.get(&x.clave) != Some(&x.sello) {
                    let Some(c) = x.cuerpo() else { continue };
                    cambios.push(pb::Cambio { tipo: pb::TipoCambio::CambioActualiza as i32, entidad: x.clave.0, id: x.clave.1.clone(), datos: Some(pb::Entidad { cuerpo: Some(c) }), reemplaza: true, ..Default::default() });
                }
                tiene.insert(x.clave, x.sello);
            }
            cambios.extend(mapa.tiene.keys().filter(|c| !tiene.contains_key(*c)).map(|(tipo, eid)| cambio_quita(pb::TipoEntidad::try_from(*tipo).unwrap_or_default(), eid.clone())));
            let nueva_niebla = match (niebla.as_ref(), mapa.niebla.as_ref()) {
                (Some(ahora), Some(antes)) => !Arc::ptr_eq(ahora, antes) && **ahora != **antes,
                (ahora, _) => ahora.is_some(),
            };
            if cambios.is_empty() && !nueva_niebla {
                // Lo mismo con otro puntero (tras una `Foto`): en adelante, el de la perspectiva.
                s.mapa = Some(Mapa { tiene, niebla: niebla.clone() });
                continue;
            }
            let envio = niebla.as_ref().filter(|_| nueva_niebla).map(|ahora| match &mapa.niebla {
                Some(antes) if self.nieblas.get(&p).is_some_and(|x| Arc::ptr_eq(x, antes)) => cambios_de_niebla.entry(p).or_insert_with(|| cambio_de_niebla(antes, ahora)).clone(),
                Some(antes) => cambio_de_niebla(antes, ahora),
                None => (**ahora).clone(),
            });
            let desde = s.abiertos[&a];
            envios.push((id, Difusion::Delta(pb::Delta { ambito: Some(a.a_contrato(&|b| n.uuid_de_batalla(b))), desde, hasta: v, cambios, eventos: Vec::new(), niebla: envio })));
            s.abiertos.insert(a, v);
            s.mapa = Some(Mapa { tiene, niebla: niebla.clone() });
        }
        // La niebla de cada perspectiva que se miró queda como la última entregada; con todas las vistas, las de perspectivas que ya nadie mira se olvidan.
        if solo.is_none() {
            self.nieblas.clear();
        }
        for (p, (_, niebla)) in miradas {
            match niebla {
                Some(x) => drop(self.nieblas.insert(p, x)),
                None => drop(self.nieblas.remove(&p)),
            }
        }
    }

    /// Tras mandar la `Foto` de un ámbito: lo que lleva de lo que no pasa por el registro de cambios es lo que el cliente tiene (los alijos de `HEROE`; en `ADMIN`, el estado de tecnología y los
    /// Aedas itinerantes).
    pub(super) fn recordar_foto(&mut self, n: &Nucleo, id: SesionId, a: Ambito) {
        let Some(s) = self.sesiones.get_mut(&id) else { return };
        match a {
            Ambito::Heroe(h) => {
                s.alijos = Some(alijos_a_la_vista(n, h).into_iter().collect());
                s.informes = Some(informes_vigentes(n, h));
            }
            Ambito::Admin => s.admin_visto = Some(AdminVisto::de(n)),
            Ambito::Campamento(c) => s.cobra = Some(interior::lo_que_cobra(n, c, s.heroe)),
            _ => {}
        }
    }

    /// Lo que no pasa por el registro de cambios y le falta al cliente de un ámbito (6G), dejando anotado que ya lo tiene: los alijos de su `HEROE` o, en `ADMIN`, la tecnología con sus contadores
    /// y los Aedas itinerantes. Lo usa la reanudación por deltas, que lo manda en su misma `Delta`.
    pub(super) fn fuera_del_registro(&mut self, n: &Nucleo, id: SesionId, a: Ambito) -> Vec<pb::Cambio> {
        let Some(s) = self.sesiones.get_mut(&id) else { return Vec::new() };
        match a {
            Ambito::Heroe(h) => cambios_de_heroe(n, s, h),
            Ambito::Admin => cambios_de_admin(n, s),
            Ambito::Campamento(c) => cambios_de_campamento(n, s, c),
            _ => Vec::new(),
        }
    }

    /// Pone al día lo que cobra el `CAMPAMENTO` de cada sesión (de `solo`, si se da): si los precios del mercado o, del residente, los de reclutar cambiaron respecto de lo que tiene el
    /// cliente, el campamento entero en una `Delta` del ámbito (2026-10-27). En cada reparto, como `ADMIN`: dentro de un campamento hay pocas sesiones y comparar no copia el campamento.
    pub(super) fn campamentos_al_dia(&mut self, n: &Nucleo, solo: Option<SesionId>, envios: &mut Vec<(SesionId, Difusion)>) {
        let dentro: Vec<(SesionId, u32)> = self
            .suscriptores
            .iter()
            .filter_map(|(a, ss)| if let Ambito::Campamento(c) = a { Some(ss.iter().filter(|s| solo.is_none_or(|x| x == **s)).map(|&s| (s, *c)).collect::<Vec<_>>()) } else { None })
            .flatten()
            .collect();
        for (id, c) in dentro {
            let Some(s) = self.sesiones.get_mut(&id) else { continue };
            let cambios = cambios_de_campamento(n, s, c);
            entregar_fuera_del_registro(n, id, s, Ambito::Campamento(c), cambios, envios);
        }
    }

    /// Pone al día los alijos que tiene a la vista el `HEROE` de cada sesión (6G; de `solo`, si se da), en una `Delta` del ámbito de la versión que tiene el cliente a la de ahora. Va con el muestreo
    /// de la vista (y tras cada reparto, como ella): la columna anda sin que nada del mundo cambie.
    pub(super) fn reconciliar_alijos(&mut self, n: &Nucleo, solo: Option<SesionId>, envios: &mut Vec<(SesionId, Difusion)>) {
        let heroes: Vec<(SesionId, u64)> = self
            .suscriptores
            .iter()
            .filter_map(|(a, ss)| if let Ambito::Heroe(h) = a { Some(ss.iter().filter(|s| solo.is_none_or(|x| x == **s)).map(|&s| (s, *h)).collect::<Vec<_>>()) } else { None })
            .flatten()
            .collect();
        for (id, h) in heroes {
            let Some(s) = self.sesiones.get_mut(&id) else { continue };
            let cambios = cambios_de_heroe(n, s, h);
            entregar_fuera_del_registro(n, id, s, Ambito::Heroe(h), cambios, envios);
        }
    }

    /// Pone al día en cada `ADMIN` (de `solo`, si se da) lo que no pasa por el registro de cambios (6G): el estado de tecnología con sus contadores, si cambió, y los Aedas itinerantes que se movieron
    /// de plaza o de plan. En cada reparto: las sesiones de administración son pocas y la comparación no copia nada.
    pub(super) fn admin_al_dia(&mut self, n: &Nucleo, solo: Option<SesionId>, envios: &mut Vec<(SesionId, Difusion)>) {
        let admins: Vec<SesionId> = self.suscritos(Ambito::Admin).map(|(s, _)| s).filter(|&s| solo.is_none_or(|x| x == s)).collect();
        for id in admins {
            let Some(s) = self.sesiones.get_mut(&id) else { continue };
            let cambios = cambios_de_admin(n, s);
            entregar_fuera_del_registro(n, id, s, Ambito::Admin, cambios, envios);
        }
    }

    /// ¿Tiene alguna sesión su `VISTA` abierta? (Si no, no hay nada que muestrear.)
    pub(super) fn hay_vistas(&self) -> bool {
        self.suscriptores.keys().any(|a| matches!(a, Ambito::Vista(_)))
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

/// Un `ACTUALIZA` completo de una entidad.
fn actualiza(tipo: pb::TipoEntidad, id: String, cuerpo: pb::entidad::Cuerpo) -> pb::Cambio {
    pb::Cambio { tipo: pb::TipoCambio::CambioActualiza as i32, entidad: tipo as i32, id, datos: Some(pb::Entidad { cuerpo: Some(cuerpo) }), reemplaza: true, ..Default::default() }
}

/// Lo que el `HEROE` de `h` ve cambiar sin que cambie el mundo: los alijos a la vista y los informes de inspección que caducaron.
fn cambios_de_heroe(n: &Nucleo, s: &mut Sesion, h: u64) -> Vec<pb::Cambio> {
    let mut cambios = cambios_de_alijos(n, s, h);
    cambios.extend(cambios_de_informes(n, s, h));
    cambios
}

/// Los ids de los informes de inspección vigentes del héroe de la sesión `h` y de los de campo de su cuenta, que viajan en su `HEROE`.
fn informes_vigentes(n: &Nucleo, h: u64) -> BTreeSet<String> {
    let Some(cuenta) = n.heroe(h).map(|x| x.cuenta.as_str()) else { return BTreeSet::new() };
    let de_la_cuenta = n.estado().militar.heroes.values().filter(|x| x.cuenta == cuenta).map(|x| x.heroe.id);
    de_la_cuenta.flat_map(|x| n.informes_de_inspeccion(x).into_iter().map(move |i| propios::id_de_informe(x, i))).collect()
}

/// Los informes de inspección que el cliente tenía y ya no están vigentes (caducaron): `QUITA` (paridad, 2026-10-27; antes los
/// descartaba el cliente por su `expira_en_ms`). Los nuevos llegan por el registro de cambios, con el héroe.
fn cambios_de_informes(n: &Nucleo, s: &mut Sesion, h: u64) -> Vec<pb::Cambio> {
    let ahora = informes_vigentes(n, h);
    let quitar: Vec<String> = s.informes.as_ref().map(|antes| antes.difference(&ahora).cloned().collect()).unwrap_or_default();
    s.informes = Some(ahora);
    quitar.into_iter().map(|id| cambio_quita(pb::TipoEntidad::InformeInspeccion, id)).collect()
}

/// Los alijos que entran a la vista del héroe `h` (`ACTUALIZA`) y los que salen (`QUITA`: se alejó, los abrió o ya no puede buscarlos) respecto de lo que el cliente tiene, que queda al día. Si no se
/// sabe qué tiene (reanudó por deltas) y el héroe puede buscarlos, se le quitan todos los demás alijos del mundo; si no puede buscar, no se le manda nada.
// Decidido por el autor (2026-10-26, duda 4 de 6G): un héroe que deja de poder buscar alijos estando desconectado y reanuda por deltas conserva en su cliente los que tenía hasta su
// siguiente `Foto` de `HEROE` (arreglarlo pediría quitar todos los alijos del mundo a cada héroe que reconecta).
fn cambios_de_alijos(n: &Nucleo, s: &mut Sesion, h: u64) -> Vec<pb::Cambio> {
    // Lo barato primero: quien no puede buscar y no tiene ninguno (casi todos) no cuesta nada más.
    let busca = n.puede_buscar_alijos(h);
    if !busca && s.alijos.as_ref().is_none_or(BTreeSet::is_empty) {
        s.alijos = Some(BTreeSet::new());
        return Vec::new();
    }
    let ahora: BTreeSet<u32> = if busca { alijos_a_la_vista(n, h).into_iter().collect() } else { BTreeSet::new() };
    let quitar: Vec<u32> = match &s.alijos {
        Some(antes) => antes.difference(&ahora).copied().collect(),
        None => (0..n.alijos_del_artefacto()).filter(|i| !ahora.contains(i)).collect(),
    };
    let entran: Vec<u32> = ahora.iter().copied().filter(|i| s.alijos.as_ref().is_none_or(|x| !x.contains(i))).collect();
    s.alijos = Some(ahora);
    let mut cambios: Vec<pb::Cambio> = entran.into_iter().filter_map(|i| propios::alijo(n, i)).map(|x| actualiza(pb::TipoEntidad::Alijo, x.id.clone(), pb::entidad::Cuerpo::Alijo(x))).collect();
    cambios.extend(quitar.into_iter().filter_map(|i| n.alijo(i)).map(|x| cambio_quita(pb::TipoEntidad::Alijo, x.id.clone())));
    cambios
}

/// El campamento entero si lo que cobra cambió respecto de lo que tiene el cliente (todo, si no se sabe), dejándolo anotado.
fn cambios_de_campamento(n: &Nucleo, s: &mut Sesion, c: u32) -> Vec<pb::Cambio> {
    let ahora = interior::lo_que_cobra(n, c, s.heroe);
    if s.cobra.as_ref() == Some(&ahora) {
        return Vec::new();
    }
    s.cobra = Some(ahora);
    interior::campamento_por_dentro(n, c, s.espectador()).map(|x| actualiza(pb::TipoEntidad::CampamentoMercenarios, x.id.clone(), pb::entidad::Cuerpo::CampamentoMercenarios(x))).into_iter().collect()
}

/// Lo que `ADMIN` ve y no pasa por el registro, si cambió respecto de lo que el cliente tiene (todo, si no se sabe): el estado de tecnología entero, los Aedas itinerantes que se movieron y lo que no
/// tiene clase cuya huella cambió (`ACTUALIZA`) o que ya no está (`QUITA`: una Mirada que dejó de enfriar, la zona de una Facción que se quedó sin plazas). Al reanudar no se sabe qué tiene el
/// cliente: se manda todo lo de ahora y nada se quita.
fn cambios_de_admin(n: &Nucleo, s: &mut Sesion) -> Vec<pb::Cambio> {
    let visto = s.admin_visto.as_ref();
    let tecnologia = visto.is_none_or(|x| !x.igual_tecnologia(n));
    let movidos: Vec<&tecnologia::itinerantes::AedaItinerante> = n.estado().aedas.iter().enumerate().filter(|(i, x)| visto.and_then(|v| v.aedas.get(*i)) != Some(*x)).map(|(_, x)| x).collect();
    let huellas = vista::huellas_de_admin(n);
    let cambiadas: Vec<&vista::ClaveAdmin> = huellas.iter().filter(|(k, h)| visto.and_then(|v| v.sin_clase.get(*k)) != Some(*h)).map(|(k, _)| k).collect();
    let quitadas: Vec<&vista::ClaveAdmin> = visto.map(|v| v.sin_clase.keys().filter(|k| !huellas.contains_key(*k)).collect()).unwrap_or_default();
    if !tecnologia && movidos.is_empty() && cambiadas.is_empty() && quitadas.is_empty() {
        return Vec::new();
    }
    let mut cambios = Vec::new();
    if tecnologia {
        cambios.push(actualiza(pb::TipoEntidad::EstadoTecnologia, propios::ID_ESTADO_TECNOLOGIA.into(), pb::entidad::Cuerpo::EstadoTecnologia(propios::estado_tecnologia(n, Espectador::Admin))));
    }
    cambios.extend(movidos.into_iter().map(|x| actualiza(pb::TipoEntidad::Aeda, crate::proyeccion::ids::aeda(x.id), pb::entidad::Cuerpo::AedaItinerante(propios::aeda_itinerante(x)))));
    cambios.extend(cambiadas.into_iter().filter_map(|k| Some(actualiza(k.0, k.1.clone(), vista::de_admin(n, k)?))));
    cambios.extend(quitadas.into_iter().map(|k| cambio_quita(k.0, k.1.clone())));
    s.admin_visto = Some(AdminVisto::de(n));
    cambios
}

/// Manda lo que no pasa por el registro en una `Delta` del ámbito, de la versión que tiene el cliente a la de ahora (puede ser la misma), y el ámbito queda en la de ahora.
fn entregar_fuera_del_registro(n: &Nucleo, id: SesionId, s: &mut Sesion, a: Ambito, cambios: Vec<pb::Cambio>, envios: &mut Vec<(SesionId, Difusion)>) {
    if cambios.is_empty() {
        return;
    }
    let (Some(&desde), v) = (s.abiertos.get(&a), n.version()) else { return };
    envios.push((id, Difusion::Delta(pb::Delta { ambito: Some(a.a_contrato(&|b| n.uuid_de_batalla(b))), desde, hasta: v, cambios, eventos: Vec::new(), niebla: None })));
    s.abiertos.insert(a, v);
}
