//! El ciclo de una batalla de Unity visto desde BronzeAge (Batalla_Unity_Ciclo §3-§8): la máquina de estados `convocando → asignada → en_curso →
//! aplicada`, o `cancelada` / `fallida`, con el ticket congelado, la convocatoria, el candado de escuadras, los plazos y la retención.
//!
//! **Unity decide todo lo de dentro.** Aquí solo las entradas y las salidas: cuándo se abre, quién entra, qué se bloquea, quién puede unirse, qué
//! mensajes acepta el servidor de batalla y en qué orden, y qué se aplica del resultado (`resultado.rs`). Todo es puro: el núcleo guarda las
//! `Batalla`, programa el evento de vencimiento a `expira_en` y el de fin de convocatoria a `convocatoria_hasta` (prioridad 1), reúne los ids
//! de ejércitos, caravanas y escuadras de su estado y llama a estas transiciones. El ticket lleva lo que el ciclo necesita para decidir
//! (bandos, participantes, escuadras autorizadas); los snapshots de atributos, equipo y mapa de la plaza son del codec del contrato (fase 4E).
//!
//! Los instantes de este módulo son de **mundo** (`expira_en`, `convocatoria_hasta`…); los del resultado y de los tokens, **reales** (ms).

use crate::avanzada::Etapa;
use crate::caravana::Modalidad;
use crate::datos::Reglas;
use crate::resolucion::Lado;
use base::{Instante, Punto};
use militar::heroe::Controlador;
use militar::Tropa;
use militar::{AsentamientoId, BatallaId, CampamentoId, CaravanaId, ColumnaId, EscuadraId, FaccionId, HeroeId};
use std::collections::BTreeSet;

pub use crate::resultado::{Efectos, ErrorResultado, Resultado};

/// Una caravana que se juega en una batalla de caravana. `carros`: cuántos lleva al abrir; Unity los nombra por su posición en su lista (`"<caravana>:<posición>"` en
/// `carros_perdidos` del resultado).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaravanaEnBatalla {
    pub id: CaravanaId,
    pub carros: usize,
}

/// El contexto estratégico (el «tipo» de batalla): dónde y contra qué se juega.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Contexto {
    Asedio { asentamiento: AsentamientoId },
    /// `solitarios`: persecución entre columnas personales; si no, batalla campal entre ejércitos.
    CampoAbierto { punto: Punto, solitarios: bool },
    /// Las caravanas en juego: una si un ejército asalta una, todas las adjuntas a la columna si la acosan los bandidos (quinta ronda de Entrar con todo, 2026-10-22).
    Caravana { caravanas: Vec<CaravanaEnBatalla>, punto: Punto, modalidad: Modalidad },
    CampamentoBandidos { campamento: CampamentoId, punto: Punto },
    SaqueoPueblo { pueblo: u64, punto: Punto },
    Avanzada { avanzada: u64, etapa: Etapa, punto: Punto },
}

impl Contexto {
    /// Un asedio y una avanzada (que se juega como asedio) admiten 15 héroes por bando y duran 30 minutos; el resto, 5 y 15.
    fn es_de_asedio(&self) -> bool {
        matches!(self, Contexto::Asedio { .. } | Contexto::Avanzada { .. })
    }

    pub fn capacidad(&self, r: &Reglas) -> usize {
        if self.es_de_asedio() {
            r.batalla.capacidad_asedio
        } else {
            r.batalla.capacidad_resto
        }
    }

    /// Duración máxima de la partida (ms).
    pub fn duracion(&self, r: &Reglas) -> i64 {
        if self.es_de_asedio() {
            r.batalla.duracion_asedio
        } else {
            r.batalla.duracion_resto
        }
    }

    /// Solo un asedio tiene convocatoria; los demás salen hacia Unity al abrirse. Una avanzada se juega como un asedio en capacidad y duración, pero sin
    /// convocatoria (decisión del autor, 2026-10-22).
    pub fn tiene_convocatoria(&self) -> bool {
        matches!(self, Contexto::Asedio { .. })
    }
}

/// Una escuadra autorizada para la batalla: `efectivos` es la **cantidad al abrir**, la fuerza reservada; el resultado cierra sobre esto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscuadraEnBatalla {
    pub id: EscuadraId,
    pub tropa: Tropa,
    /// El héroe dueño. Las escuadras sin héroe (guarnición, escolta) pueden tener dueño; las de bandidos y carreteros, no.
    pub dueno: Option<HeroeId>,
    pub efectivos: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Participante {
    pub heroe: HeroeId,
    /// `None`: sin bandera.
    pub faccion: Option<FaccionId>,
    pub controlador: Controlador,
    /// Cuánto botín cabe en su inventario (`casillasInventario − ocupadas`).
    pub casillas_libres: u32,
    pub escuadras: Vec<EscuadraEnBatalla>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BandoDeTicket {
    /// `None` cuando el bando no es de nadie: bandidos, o una columna sin bandera.
    pub faccion: Option<FaccionId>,
    pub capacidad: usize,
    pub participantes: Vec<Participante>,
    pub sin_heroe: Vec<EscuadraEnBatalla>,
}

impl BandoDeTicket {
    pub fn nuevo(faccion: Option<FaccionId>, participantes: Vec<Participante>, sin_heroe: Vec<EscuadraEnBatalla>) -> Self {
        Self { faccion, capacidad: 0, participantes, sin_heroe }
    }
}

/// El `BattleTicket`, congelado al abrir.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket {
    /// Empieza en 0 y solo sube con una salida previa al inicio.
    pub revision: u32,
    pub contexto: Contexto,
    /// Duración máxima de la partida (ms).
    pub duracion_maxima: i64,
    pub atacante: BandoDeTicket,
    pub defensor: BandoDeTicket,
}

impl Ticket {
    /// Un ticket nuevo: revisión 0, con la duración y la capacidad que corresponden al contexto. **Al abrir no se recorta** por capacidad: si la
    /// columna lleva más héroes, todos van al ticket (la cola es cosa de Unity); la capacidad solo frena a quien se une.
    pub fn nuevo(r: &Reglas, contexto: Contexto, mut atacante: BandoDeTicket, mut defensor: BandoDeTicket) -> Self {
        atacante.capacidad = contexto.capacidad(r);
        defensor.capacidad = contexto.capacidad(r);
        let duracion_maxima = contexto.duracion(r);
        Self { revision: 0, contexto, duracion_maxima, atacante, defensor }
    }

    pub fn bando(&self, lado: Lado) -> &BandoDeTicket {
        match lado {
            Lado::Atacante => &self.atacante,
            Lado::Defensor => &self.defensor,
        }
    }

    fn bando_mut(&mut self, lado: Lado) -> &mut BandoDeTicket {
        match lado {
            Lado::Atacante => &mut self.atacante,
            Lado::Defensor => &mut self.defensor,
        }
    }
}

/// Alguien que se une **después** del ticket. Solo crece.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incorporacion {
    pub secuencia: u32,
    pub lado: Lado,
    pub participante: Participante,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotivoSalida {
    /// Antes de empezar: sin castigo.
    NoConectado,
    Eliminado,
    Abandono,
    Desconexion,
}

/// Cómo acabó una escuadra de quien sale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscuadraDeSalida {
    pub escuadra: EscuadraId,
    pub supervivientes: i64,
    pub muertos: i64,
}

/// El mensaje `SalidaBatalla` de Conquest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalidaBatalla {
    pub intento: String,
    pub revision: u32,
    pub heroe: HeroeId,
    pub motivo: MotivoSalida,
    pub derrotado: bool,
    pub escuadras: Vec<EscuadraDeSalida>,
}

/// Quién ha salido, por qué y cuándo. Solo crece.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Salida {
    pub heroe: HeroeId,
    pub motivo: MotivoSalida,
    pub derrotado: bool,
    pub en: Instante,
}

/// Lo que se queda quieto mientras dura, además de sus héroes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bloqueo {
    pub ejercitos: Vec<ColumnaId>,
    pub caravanas: Vec<CaravanaId>,
    pub asentamiento: Option<AsentamientoId>,
    pub campamento: Option<CampamentoId>,
    pub pueblo: Option<u64>,
    pub avanzada: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstadoBatalla {
    Convocando,
    Asignada,
    EnCurso,
    Aplicada,
    Cancelada,
    Fallida,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instancia {
    pub host: String,
    pub puerto: u16,
    pub protocolo: String,
}

/// El token de un héroe humano, que BronzeAge custodia y nunca verifica. `expira_en` es tiempo real (ms).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub heroe: HeroeId,
    pub token: String,
    pub expira_en: i64,
}

/// La `BattleServerAssignment` aceptada más quién la registró.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asignacion {
    pub intento: String,
    pub servidor: String,
    pub revision: u32,
    pub instancia: Instancia,
    pub tokens: Vec<Token>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorBatalla {
    /// No está activa: cerrada, o vencida su plazo (`batalla.no_existe`, 404).
    NoActiva,
    /// Otro servidor la tiene (`batalla.ya_asignada`).
    YaAsignada,
    /// Un asedio en convocatoria no se publica ni se asigna.
    EnConvocatoria,
    /// La revisión no es la vigente.
    RevisionVieja,
    /// Ni el estado de la batalla ni el motivo permiten esta operación.
    EstadoInvalido,
    /// No es el servidor de la asignación activa.
    ServidorAjeno,
    /// Un token de quien no es un humano de esta batalla.
    TokenInvalido(HeroeId),
    /// El héroe no combate en esta batalla.
    NoCombate(HeroeId),
    /// El bando se pasaría de su capacidad.
    LadoLleno,
    /// Ya salió de esta batalla y no vuelve a entrar.
    YaSalio(HeroeId),
    /// Ya combate en ella (un héroe no entra dos veces).
    YaCombate(HeroeId),
    /// Solo quien la inició puede cancelarla, y antes de que empiece.
    NoPuedeCancelar,
    /// Un escuadra de la salida que no es suya, repetida, o que no cierra sobre lo autorizado.
    SalidaInvalida,
    Resultado(ErrorResultado),
    /// «Ya se aplicó con otro resultado» (409).
    OtroResultado,
}

/// Un mensaje repetido que ya estaba aplicado responde bien sin cambiar nada: es lo que deja a Conquest reintentar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Efecto<T> {
    Aplicado(T),
    SinCambios,
}

/// Qué cambia en el mundo al salir un héroe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalidaAplicada {
    /// Las escuadras de quien sale, que se liberan; las que traen bajas (`escuadra`, `supervivientes`) fijan su cantidad.
    pub liberar: Vec<EscuadraId>,
    pub bajas: Vec<(EscuadraId, i64)>,
    /// `derrotado` (solo con la partida en curso): queda herido.
    pub herido: bool,
    /// Salió el último humano, o el atacante se quedó sin héroes antes de empezar: se canceló sin castigo (y se liberan todos los candados).
    pub cancelada: bool,
    /// Antes de empezar: sube la revisión, vuelve a `convocando` y se pierde la asignación y los tokens.
    pub reabierta: bool,
}

/// Una batalla de Unity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Batalla {
    pub id: BatallaId,
    pub estado: EstadoBatalla,
    /// Quien comprometió el combate: puede cancelarla antes de que empiece.
    pub iniciada_por: HeroeId,
    pub punto: Punto,
    pub ticket: Ticket,
    pub incorporaciones: Vec<Incorporacion>,
    pub salidas: Vec<Salida>,
    pub bloqueo: Bloqueo,
    /// Plazo del estado actual, en tiempo de mundo.
    pub expira_en: Instante,
    /// Solo un asedio: hasta cuándo se entra «como si estuviera desde el principio».
    pub convocatoria_hasta: Option<Instante>,
    pub iniciada_en: Option<Instante>,
    pub aplicada_en: Option<Instante>,
    /// Cuándo pasó a `aplicada`, `cancelada` o `fallida`: de ahí corre la retención.
    pub cerrada_en: Option<Instante>,
    pub asignacion: Option<Asignacion>,
    /// El resultado aplicado, **entero** (idempotencia).
    pub resultado: Option<Resultado>,
}

impl Batalla {
    /// Abre una batalla: `convocando`, con el plazo de asignación contado desde el fin de la convocatoria (o desde ahora si no la hay).
    pub fn abrir(r: &Reglas, id: BatallaId, iniciada_por: HeroeId, punto: Punto, ticket: Ticket, bloqueo: Bloqueo, ahora: Instante) -> Self {
        let convocatoria_hasta = ticket.contexto.tiene_convocatoria().then_some(ahora + r.batalla.convocatoria);
        Self {
            id,
            estado: EstadoBatalla::Convocando,
            iniciada_por,
            punto,
            ticket,
            incorporaciones: vec![],
            salidas: vec![],
            bloqueo,
            expira_en: convocatoria_hasta.unwrap_or(ahora) + r.batalla.plazo_asignacion,
            convocatoria_hasta,
            iniciada_en: None,
            aplicada_en: None,
            cerrada_en: None,
            asignacion: None,
            resultado: None,
        }
    }

    pub fn cerrada(&self) -> bool {
        matches!(self.estado, EstadoBatalla::Aplicada | EstadoBatalla::Cancelada | EstadoBatalla::Fallida)
    }

    /// Activa: `convocando`, `asignada` o `en_curso` **y** dentro de plazo. Una vencida deja de bloquear ya, aunque la cierre su evento.
    pub fn activa(&self, ahora: Instante) -> bool {
        !self.cerrada() && ahora < self.expira_en
    }

    /// Un asedio en sus 30 s de convocatoria: quien se une entra en el ticket y no se publica ni se asigna.
    pub fn en_convocatoria(&self, ahora: Instante) -> bool {
        self.convocatoria_hasta.is_some_and(|h| ahora < h)
    }

    /// Lo que lista `GET /batallas/pendientes`: `convocando`, activas y fuera de convocatoria.
    pub fn pendiente_de_asignar(&self, ahora: Instante) -> bool {
        self.estado == EstadoBatalla::Convocando && self.activa(ahora) && !self.en_convocatoria(ahora)
    }

    /// Una cerrada se retira del estado `conservarCerradasMinutos` después de cerrarse; sin `cerradaEn` (guardada antes de existir) cuenta
    /// `aplicadaEn` y, en su defecto, `expiraEn`.
    pub fn retirable(&self, r: &Reglas, ahora: Instante) -> bool {
        self.cerrada() && ahora >= self.cerrada_en.or(self.aplicada_en).unwrap_or(self.expira_en) + r.batalla.conservar_cerradas
    }

    pub fn salio(&self, heroe: HeroeId) -> bool {
        self.salidas.iter().any(|s| s.heroe == heroe)
    }

    /// Los que combatirían ahora, con su bando: los del ticket y las incorporaciones que no han salido.
    fn participantes(&self) -> impl Iterator<Item = (Lado, &Participante)> {
        let t = &self.ticket;
        t.atacante
            .participantes
            .iter()
            .map(|p| (Lado::Atacante, p))
            .chain(t.defensor.participantes.iter().map(|p| (Lado::Defensor, p)))
            .chain(self.incorporaciones.iter().map(|i| (i.lado, &i.participante)))
            .filter(|(_, p)| !self.salio(p.heroe))
    }

    pub fn participante(&self, heroe: HeroeId) -> Option<&Participante> {
        self.participantes().find(|(_, p)| p.heroe == heroe).map(|(_, p)| p)
    }

    pub fn lado_de(&self, heroe: HeroeId) -> Option<Lado> {
        self.participantes().find(|(_, p)| p.heroe == heroe).map(|(l, _)| l)
    }

    /// Los héroes que combaten (`participacionesDe`): bloqueados, sin órdenes y sin salir del mundo.
    pub fn heroes_que_combaten(&self) -> impl Iterator<Item = HeroeId> + '_ {
        self.participantes().map(|(_, p)| p.heroe)
    }

    /// `escuadrasDe`: las de los participantes presentes, las incorporadas y las sin héroe. Las de quien salió ya se cerraron con su salida.
    pub fn escuadras_de(&self) -> impl Iterator<Item = &EscuadraEnBatalla> + '_ {
        let t = &self.ticket;
        self.participantes().flat_map(|(_, p)| p.escuadras.iter()).chain(t.atacante.sin_heroe.iter()).chain(t.defensor.sin_heroe.iter())
    }

    /// `escuadrasConDueno`: las que llevan el candado `reservaBatalla` mientras dura. Con él no se reponen ni entran o salen de la guarnición.
    pub fn escuadras_con_candado(&self) -> Vec<EscuadraId> {
        self.escuadras_de().filter(|e| e.dueno.is_some()).map(|e| e.id).collect()
    }

    fn humanos(&self) -> usize {
        self.participantes().filter(|(_, p)| p.controlador == Controlador::Humano).count()
    }

    fn en_el_lado(&self, lado: Lado) -> usize {
        self.participantes().filter(|(l, _)| *l == lado).count()
    }

    /// Un héroe se une con su columna entera (`entran`: sus héroes sanos, con lo que llevan en ella); quien está dentro de un lugar y no tiene columna (un residente) se une sin ella (`columna: None`). Durante la convocatoria entran **en el
    /// ticket** como uno de los del principio; después, cada uno es una **incorporación** y el ticket no cambia ni sube de revisión. Si el
    /// bando está lleno, no entra: `yaDentro + entran > capacidad`. Devuelve las secuencias asignadas (vacío en la convocatoria).
    ///
    /// El lado (`lado_para_unirse`) y las condiciones del mundo (herido, ya en otra batalla, distancia) las comprueba quien llama.
    pub fn unirse(&mut self, ahora: Instante, lado: Lado, entran: Vec<Participante>, columna: Option<ColumnaId>) -> Result<Vec<u32>, ErrorBatalla> {
        if !self.activa(ahora) {
            return Err(ErrorBatalla::NoActiva);
        }
        if let Some(p) = entran.iter().find(|p| self.salio(p.heroe)) {
            return Err(ErrorBatalla::YaSalio(p.heroe));
        }
        if let Some(p) = entran.iter().enumerate().find(|(i, p)| self.participante(p.heroe).is_some() || entran[..*i].iter().any(|x| x.heroe == p.heroe)) {
            return Err(ErrorBatalla::YaCombate(p.1.heroe));
        }
        if self.en_el_lado(lado) + entran.len() > self.ticket.bando(lado).capacidad {
            return Err(ErrorBatalla::LadoLleno);
        }
        if let Some(c) = columna.filter(|c| !self.bloqueo.ejercitos.contains(c)) {
            self.bloqueo.ejercitos.push(c);
        }
        if self.en_convocatoria(ahora) {
            self.ticket.bando_mut(lado).participantes.extend(entran);
            return Ok(vec![]);
        }
        let base = self.incorporaciones.len() as u32;
        let secuencias = (1..=entran.len() as u32).map(|i| base + i).collect();
        self.incorporaciones.extend(entran.into_iter().enumerate().map(|(i, participante)| Incorporacion { secuencia: base + i as u32 + 1, lado, participante }));
        Ok(secuencias)
    }

    /// Quien la inició la cancela antes de que empiece la partida, sin que nadie pierda nada. Devuelve los candados que se sueltan.
    pub fn cancelar(&mut self, ahora: Instante, heroe: HeroeId) -> Result<Vec<EscuadraId>, ErrorBatalla> {
        if !self.activa(ahora) {
            return Err(ErrorBatalla::NoActiva);
        }
        if heroe != self.iniciada_por || !matches!(self.estado, EstadoBatalla::Convocando | EstadoBatalla::Asignada) {
            return Err(ErrorBatalla::NoPuedeCancelar);
        }
        let liberar = self.escuadras_con_candado();
        self.cerrar(EstadoBatalla::Cancelada, ahora);
        Ok(liberar)
    }

    /// `vencerBatallas`, a `expira_en`: toda activa cuyo plazo llegó pasa a `fallida` sin castigo (vale desde `convocando`, `asignada` y
    /// `en_curso`). Devuelve los candados que se sueltan; `None` si no tocaba.
    pub fn vencer(&mut self, ahora: Instante) -> Option<Vec<EscuadraId>> {
        if self.cerrada() || ahora < self.expira_en {
            return None;
        }
        let liberar = self.escuadras_con_candado();
        self.cerrar(EstadoBatalla::Fallida, ahora);
        Some(liberar)
    }

    fn cerrar(&mut self, estado: EstadoBatalla, ahora: Instante) {
        self.estado = estado;
        self.cerrada_en = Some(ahora);
    }

    /// Solo el servidor de la asignación activa, con la revisión vigente.
    fn comprobar_asignacion(&self, intento: &str, servidor: &str, revision: u32) -> Result<(), ErrorBatalla> {
        match &self.asignacion {
            Some(a) if a.intento == intento && a.servidor == servidor => {}
            _ => return Err(ErrorBatalla::ServidorAjeno),
        }
        if revision != self.ticket.revision {
            return Err(ErrorBatalla::RevisionVieja);
        }
        Ok(())
    }

    fn token_valido(&self, t: &Token) -> bool {
        self.participante(t.heroe).is_some_and(|p| p.controlador == Controlador::Humano)
    }

    /// `convocando → asignada`. La **misma** asignación (mismo intento y mismo servidor) repetida no cambia nada. Si no, tiene que estar `convocando`
    /// (si no, `YaAsignada`: gana la primera), fuera de convocatoria, con la revisión vigente y con tokens solo de **humanos** de esta batalla.
    pub fn registrar_asignacion(&mut self, r: &Reglas, ahora: Instante, a: Asignacion) -> Result<Efecto<()>, ErrorBatalla> {
        if !self.activa(ahora) {
            return Err(ErrorBatalla::NoActiva);
        }
        if self.asignacion.as_ref().is_some_and(|x| x.intento == a.intento && x.servidor == a.servidor) {
            return Ok(Efecto::SinCambios);
        }
        if self.estado != EstadoBatalla::Convocando {
            return Err(ErrorBatalla::YaAsignada);
        }
        if self.en_convocatoria(ahora) {
            return Err(ErrorBatalla::EnConvocatoria);
        }
        if a.revision != self.ticket.revision {
            return Err(ErrorBatalla::RevisionVieja);
        }
        if let Some(t) = a.tokens.iter().find(|t| !self.token_valido(t)) {
            return Err(ErrorBatalla::TokenInvalido(t.heroe));
        }
        self.asignacion = Some(a);
        self.estado = EstadoBatalla::Asignada;
        self.expira_en = ahora + r.batalla.plazo_inicio;
        Ok(Efecto::Aplicado(()))
    }

    /// `asignada → en_curso`: la partida empezó de verdad. Ya `en_curso`: sin cambios.
    pub fn confirmar_inicio(&mut self, r: &Reglas, ahora: Instante, intento: &str, servidor: &str, revision: u32) -> Result<Efecto<()>, ErrorBatalla> {
        if !self.activa(ahora) {
            return Err(ErrorBatalla::NoActiva);
        }
        self.comprobar_asignacion(intento, servidor, revision)?;
        match self.estado {
            EstadoBatalla::EnCurso => Ok(Efecto::SinCambios),
            EstadoBatalla::Asignada => {
                self.estado = EstadoBatalla::EnCurso;
                self.iniciada_en = Some(ahora);
                self.expira_en = ahora + self.ticket.duracion_maxima + r.batalla.margen;
                Ok(Efecto::Aplicado(()))
            }
            _ => Err(ErrorBatalla::EstadoInvalido),
        }
    }

    /// Los tokens de los humanos que se unieron después de la asignación. Un token nuevo del mismo héroe sustituye al anterior. Sin evento.
    pub fn registrar_tokens(&mut self, ahora: Instante, intento: &str, servidor: &str, tokens: Vec<Token>) -> Result<(), ErrorBatalla> {
        if !self.activa(ahora) {
            return Err(ErrorBatalla::NoActiva);
        }
        self.comprobar_asignacion(intento, servidor, self.ticket.revision)?;
        if !matches!(self.estado, EstadoBatalla::Asignada | EstadoBatalla::EnCurso) {
            return Err(ErrorBatalla::EstadoInvalido);
        }
        if let Some(t) = tokens.iter().find(|t| !self.token_valido(t)) {
            return Err(ErrorBatalla::TokenInvalido(t.heroe));
        }
        let a = self.asignacion.as_mut().expect("comprobar_asignacion lo garantiza");
        for t in tokens {
            a.tokens.retain(|x| x.heroe != t.heroe);
            a.tokens.push(t);
        }
        Ok(())
    }

    /// Un héroe sale de la batalla (CQ-011). Repetir la salida de quien ya salió no cambia nada (se mira **antes** de comprobar la asignación:
    /// tras una salida previa al inicio la revisión y la asignación ya no son las del mensaje).
    ///
    /// - `NoConectado`, solo con la batalla `asignada`: sin castigo, escuadras intactas y libres; sube la revisión, vuelve a `convocando`
    ///   (la asignación y los tokens dejan de valer) y su participación se quita del ticket sin que entre nadie en su lugar. Se cancela sin
    ///   castigo si ya no queda ningún humano o el atacante se queda sin héroes.
    /// - `Eliminado`, `Abandono`, `Desconexion`, solo `en_curso`: se aplican las bajas de sus escuadras y, si `derrotado`, queda herido. El
    ///   ticket no cambia.
    ///
    /// En los dos casos se liberan sus escuadras. Si salió quien la inició, `iniciada_por` pasa al primer atacante que queda.
    // DUDA: el documento pide que `supervivientes + muertos` cierre sobre lo autorizado, que cada escuadra sea suya y que no se repitan, pero no
    // que la salida cubra todas las suyas. Aquí no se exige: las que no vienen conservan su cantidad (y se liberan igualmente).
    pub fn registrar_salida(&mut self, r: &Reglas, ahora: Instante, servidor: &str, s: SalidaBatalla) -> Result<Efecto<SalidaAplicada>, ErrorBatalla> {
        if !self.activa(ahora) {
            return Err(ErrorBatalla::NoActiva);
        }
        if self.salio(s.heroe) {
            return Ok(Efecto::SinCambios);
        }
        self.comprobar_asignacion(&s.intento, servidor, s.revision)?;
        let suyas: Vec<EscuadraEnBatalla> = self.participante(s.heroe).ok_or(ErrorBatalla::NoCombate(s.heroe))?.escuadras.clone();
        let antes_de_empezar = s.motivo == MotivoSalida::NoConectado;
        if antes_de_empezar != (self.estado == EstadoBatalla::Asignada) || (!antes_de_empezar && self.estado != EstadoBatalla::EnCurso) {
            return Err(ErrorBatalla::EstadoInvalido);
        }
        let mut bajas = vec![];
        if !antes_de_empezar {
            for (i, e) in s.escuadras.iter().enumerate() {
                let autorizada = suyas.iter().find(|x| x.id == e.escuadra).ok_or(ErrorBatalla::SalidaInvalida)?;
                if s.escuadras[..i].iter().any(|x| x.escuadra == e.escuadra) || e.supervivientes < 0 || e.muertos < 0 || e.supervivientes + e.muertos != autorizada.efectivos {
                    return Err(ErrorBatalla::SalidaInvalida);
                }
                bajas.push((e.escuadra, e.supervivientes));
            }
        }
        let liberar: Vec<EscuadraId> = suyas.iter().map(|e| e.id).collect();
        let atacaba = self.lado_de(s.heroe) == Some(Lado::Atacante);
        self.salidas.push(Salida { heroe: s.heroe, motivo: s.motivo, derrotado: s.derrotado, en: ahora });
        if self.iniciada_por == s.heroe {
            let primero = self.participantes().find(|(l, _)| *l == Lado::Atacante).map(|(_, p)| p.heroe);
            if let Some(h) = primero {
                self.iniciada_por = h;
            }
        }
        let mut aplicada = SalidaAplicada { liberar, bajas, herido: !antes_de_empezar && s.derrotado, cancelada: false, reabierta: false };
        if antes_de_empezar {
            self.ticket.atacante.participantes.retain(|p| p.heroe != s.heroe);
            self.ticket.defensor.participantes.retain(|p| p.heroe != s.heroe);
            self.incorporaciones.retain(|i| i.participante.heroe != s.heroe);
            if self.humanos() == 0 || (atacaba && self.en_el_lado(Lado::Atacante) == 0) {
                aplicada.cancelada = true;
                aplicada.liberar = self.escuadras_con_candado();
                aplicada.liberar.extend(suyas.iter().map(|e| e.id));
                self.cerrar(EstadoBatalla::Cancelada, ahora);
            } else {
                aplicada.reabierta = true;
                self.ticket.revision += 1;
                self.estado = EstadoBatalla::Convocando;
                self.asignacion = None;
                self.expira_en = ahora + r.batalla.plazo_asignacion;
            }
        }
        Ok(Efecto::Aplicado(aplicada))
    }

    /// `aplicarResultado` (`en_curso → aplicada`): valida el checklist **entero** o no cambia nada; aplicarlo y pasar a `aplicada` es una sola
    /// mutación. Repetir el mismo resultado sobre una `aplicada` responde bien sin cambiar nada; otro distinto se rechaza. Devuelve lo que el núcleo
    /// tiene que aplicar (heridos, escuadras, experiencia, botín, candados): ver [`Efectos`].
    pub fn aplicar_resultado(&mut self, ahora: Instante, servidor: &str, res: Resultado, objeto_ya_existe: &dyn Fn(&str) -> bool) -> Result<Efecto<Efectos>, ErrorBatalla> {
        if self.estado == EstadoBatalla::Aplicada {
            return if self.resultado.as_ref() == Some(&res) { Ok(Efecto::SinCambios) } else { Err(ErrorBatalla::OtroResultado) };
        }
        if self.estado != EstadoBatalla::EnCurso {
            return Err(ErrorBatalla::EstadoInvalido);
        }
        if !self.activa(ahora) {
            return Err(ErrorBatalla::NoActiva);
        }
        self.validar_resultado(servidor, &res, objeto_ya_existe).map_err(ErrorBatalla::Resultado)?;
        let efectos = self.efectos_del_resultado(&res);
        self.resultado = Some(res);
        self.aplicada_en = Some(ahora);
        self.cerrar(EstadoBatalla::Aplicada, ahora);
        Ok(Efecto::Aplicado(efectos))
    }
}

/// Lo que se queda quieto por las batallas activas (`bloqueosDe`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bloqueos {
    pub heroes: BTreeSet<HeroeId>,
    pub ejercitos: BTreeSet<ColumnaId>,
    pub caravanas: BTreeSet<CaravanaId>,
    pub asentamientos: BTreeSet<AsentamientoId>,
    pub campamentos: BTreeSet<CampamentoId>,
    pub pueblos: BTreeSet<u64>,
    pub avanzadas: BTreeSet<u64>,
}

impl Bloqueos {
    /// `heroeBloqueado`: combate, o está **dentro** de la plaza asediada (`ubicacion` es esa plaza): tampoco da órdenes.
    pub fn heroe_bloqueado(&self, heroe: HeroeId, plaza_donde_esta: Option<AsentamientoId>) -> bool {
        self.heroes.contains(&heroe) || plaza_donde_esta.is_some_and(|p| self.asentamientos.contains(&p))
    }
}

/// `bloqueosDe`: de las batallas **activas** (vencidas no cuentan), los héroes que combaten y lo que se queda quieto.
pub fn bloqueos_de<'a>(batallas: impl IntoIterator<Item = &'a Batalla>, ahora: Instante) -> Bloqueos {
    let mut b = Bloqueos::default();
    for x in batallas.into_iter().filter(|x| x.activa(ahora)) {
        b.heroes.extend(x.heroes_que_combaten());
        b.ejercitos.extend(&x.bloqueo.ejercitos);
        b.caravanas.extend(&x.bloqueo.caravanas);
        b.asentamientos.extend(x.bloqueo.asentamiento);
        b.campamentos.extend(x.bloqueo.campamento);
        b.pueblos.extend(x.bloqueo.pueblo);
        b.avanzadas.extend(x.bloqueo.avanzada);
    }
    b
}

/// `sinCandadosVencidos`: antes de ejecutar un comando con candado, las escuadras de las batallas **vencidas** (activas por estado pero fuera de
/// plazo) se sueltan, sin cerrarlas ni narrar nada: eso es del evento de vencimiento. Así reponer o guarnecer no dependen de que ese evento haya
/// pasado.
pub fn candados_vencidos(batallas: &[Batalla], ahora: Instante) -> Vec<EscuadraId> {
    batallas.iter().filter(|b| !b.cerrada() && ahora >= b.expira_en).flat_map(|b| b.escuadras_con_candado()).collect()
}

/// El `battleId` del contrato: un UUID con la forma `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, **determinista** y sin aleatoriedad global. Sale del
/// `gameId` y del contador de ids de la partida (`n`):
///
/// ```text
/// resumen(semilla) = FNV-1a de 32 bits sobre las unidades UTF-16 del gameId (primo 0x01000193, empezando por `semilla`), en hex de 8 cifras
/// a = resumen(0x811c9dc5)     b = resumen(0x01000193)     c = n en hex de 16 cifras
/// battleId = a '-' b[0..4) '-' b[4..8) '-' c[0..4) '-' c[4..16)
/// ```
pub fn uuid_de_batalla(game_id: &str, n: u64) -> String {
    let resumen = |semilla: u32| {
        let h = game_id.encode_utf16().fold(semilla, |h, c| (h ^ c as u32).wrapping_mul(0x0100_0193));
        format!("{h:08x}")
    };
    let (a, b, c) = (resumen(0x811c_9dc5), resumen(0x0100_0193), format!("{n:016x}"));
    format!("{a}-{}-{}-{}-{}", &b[0..4], &b[4..8], &c[0..4], &c[4..16])
}

#[cfg(test)]
mod pruebas;
