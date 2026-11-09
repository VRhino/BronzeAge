//! Lo que un ofrecido y un mercenario con marca no hacen por su cuenta, en una sola función antes del despacho (Mercenarios_Por_Contrato §2, §3, §6.1 y §11; decisiones 10 a 16 del
//! plan). Tarea T4 del plan.
//!
//! [`Nucleo::rechazo_por_contrato`] es a los contratos lo que `rechazo_por_batalla` a las batallas: un `match` exhaustivo sobre [`Comando`], sin `_`, para que toda orden nueva obligue a
//! decidir si un mercenario puede darla. Un marcado, que es un ciudadano más de la Facción que le paga, solo pierde lo que el diseño le quita: crear Facción, declarar la guerra, fundar y
//! aceptar las misiones del tablón (nunca), convocar (solo con una orden `atacar`, y a su convocatoria solo se unen mercenarios), y atacar, perseguir, unirse a un ejército, a una
//! convocatoria o a una batalla, escoltar y guarnecer una avanzada (solo con la orden que se lo permite). Lo que no es una orden, o no cabe antes del despacho, vive en su sitio: la avanzada que se guarnece sin residir en su madre
//! ([`Nucleo::guarda_la_avanzada`], que llama `avanzadas.rs`), la sucesión automática del Líder (`columnas.rs`) y la Participación (`participacion.rs`), que preguntan
//! [`Nucleo::es_marcado`], y las misiones recurrentes (`recurrentes.rs`), que miran el contrato.

use super::*;
use crate::comando::ObjetivoAtaque;
use ::mercenarios::Orden;
use combate::resolucion::Lado;
use militar::inspeccion::Objetivo as Perseguido;

/// El objetivo de un `Atacar` visto como el de una orden de mando.
fn objetivo_de_ataque(o: ObjetivoAtaque) -> Objetivo {
    match o {
        ObjetivoAtaque::Ejercito(x) => Objetivo::Ejercito(x),
        ObjetivoAtaque::Caravana(x) => Objetivo::Caravana(x),
        ObjetivoAtaque::Campamento(x) => Objetivo::Bandidos(x),
        ObjetivoAtaque::Asentamiento(x) => Objetivo::Plaza(x),
        ObjetivoAtaque::Pueblo(x) => Objetivo::Pueblo(x),
        ObjetivoAtaque::Avanzada(x) => Objetivo::Avanzada(x),
    }
}

/// El objetivo de un `Perseguir` visto como el de una orden de mando.
fn objetivo_de_persecucion(o: Perseguido) -> Objetivo {
    match o {
        Perseguido::Ejercito(x) => Objetivo::Ejercito(x),
        Perseguido::Caravana(x) => Objetivo::Caravana(x),
        Perseguido::Asentamiento(x) => Objetivo::Plaza(x),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// ¿Es `heroe` un mercenario contratado, un ciudadano con marca? Su contrato y su marca nacen y acaban juntos, y el contrato es lo que se mira en cada orden: un mapa pequeño.
    pub(in crate::nucleo) fn es_marcado(&self, heroe: u64) -> bool {
        self.estado.contratos.contratos.contains_key(&heroe)
    }

    /// La orden vigente de `heroe`, si es un mercenario contratado y tiene una.
    pub(super) fn orden_de(&self, heroe: u64) -> Option<&Orden> {
        self.estado.contratos.contratos.get(&heroe)?.orden.as_ref()
    }

    /// ¿Tiene `heroe` vigente un `guardar` de esta avanzada? Es la única excepción a `avanzada.no_resides` (§6.1): quien la guarda guarnece sin residir en su plaza madre.
    pub(in crate::nucleo) fn guarda_la_avanzada(&self, heroe: u64, avanzada: u64) -> bool {
        self.orden_de(heroe).is_some_and(|o| o.tipo == TipoOrden::Guardar && o.objetivo == Objetivo::Avanzada(avanzada))
    }

    /// El bando en que está `objetivo` en la batalla `b`, si es parte de ella: lo que se ataca (una plaza, un pueblo, una avanzada, una caravana, unos bandidos) está en el bando que
    /// defiende, y una columna en el de sus héroes.
    fn lado_del_objetivo(&self, b: &Batalla, objetivo: &Objetivo) -> Option<Lado> {
        let k = &b.bloqueo;
        let atacado = match *objetivo {
            Objetivo::Plaza(x) => k.asentamiento.is_some_and(|a| u64::from(a) == x),
            Objetivo::Pueblo(x) => k.pueblo == Some(x),
            Objetivo::Avanzada(x) => k.avanzada == Some(x),
            Objetivo::Caravana(x) => k.caravanas.contains(&x),
            Objetivo::Bandidos(x) => k.campamento.is_some_and(|c| u64::from(c) == x),
            Objetivo::Ejercito(x) => return k.ejercitos.contains(&x).then(|| self.ejercito(x)).flatten().and_then(|e| e.columna.participantes.iter().find_map(|p| b.lado_de(p.heroe))),
            Objetivo::Convocatoria(_) | Objetivo::Faccion(_) | Objetivo::Punto(_) => false,
        };
        atacado.then_some(Lado::Defensor)
    }

    // ───────────────────────────── Los rechazos ─────────────────────────────

    /// Un ofrecido en el tablón no crea Facción, no pide ingreso ni funda (`faccion.mercenario_ofrecido`, §3), y un marcado tampoco, nunca (`mercenario.prohibido`).
    fn sin_faccion_propia(&self, heroe: u64) -> Option<Respuesta> {
        let v = self.estado.version;
        if self.es_marcado(heroe) {
            Some(Respuesta::rechazo(rechazos::MERCENARIO_PROHIBIDO, "un mercenario contratado no crea Facción, no pide ingreso en otra ni funda", v))
        } else if self.estado.contratos.ofertas.contains_key(&heroe) {
            Some(Respuesta::rechazo(rechazos::FACCION_MERCENARIO_OFRECIDO, "retira tu oferta del tablón antes de crear una Facción, pedir ingreso o fundar", v))
        } else {
            None
        }
    }

    /// Lo que un marcado no hace nunca, tenga la orden que tenga: `mensaje` dice qué es.
    fn prohibido_al_marcado(&self, heroe: u64, mensaje: &str) -> Option<Respuesta> {
        self.es_marcado(heroe).then(|| Respuesta::rechazo(rechazos::MERCENARIO_PROHIBIDO, mensaje, self.estado.version))
    }

    /// Lo que un marcado solo hace con una orden `tipo` sobre `objetivo` (`mercenario.sin_orden`, §6.1).
    fn pide_orden(&self, heroe: u64, tipo: TipoOrden, objetivo: Objetivo) -> Option<Respuesta> {
        let contrato = self.estado.contratos.contratos.get(&heroe)?;
        let permitido = contrato.orden.as_ref().is_some_and(|o| o.tipo == tipo && o.objetivo == objetivo);
        (!permitido).then(|| self.rechazo_sin_orden())
    }

    /// El rechazo de lo que un marcado solo hace con la orden que se lo permite.
    fn rechazo_sin_orden(&self) -> Respuesta {
        Respuesta::rechazo(rechazos::MERCENARIO_SIN_ORDEN, "un mercenario contratado solo hace eso con la orden que se lo permite", self.estado.version)
    }

    /// Un marcado solo convoca con una orden `atacar` vigente, para formar el ejército con el que cumplirla (decisión del autor del 2026-11-01): una columna personal no asedia ni
    /// ataca ejércitos ni pueblos. A su convocatoria solo se unen mercenarios ([`Nucleo::de_mercenarios`]).
    fn convoca_para_atacar(&self, heroe: u64) -> Option<Respuesta> {
        let contrato = self.estado.contratos.contratos.get(&heroe)?;
        let atacar = contrato.orden.as_ref().is_some_and(|o| o.tipo == TipoOrden::Atacar);
        (!atacar).then(|| Respuesta::rechazo(rechazos::MERCENARIO_SIN_ORDEN, "un mercenario contratado solo convoca con una orden atacar, para cumplirla", self.estado.version))
    }

    /// Un ciudadano sin marca no se une a la convocatoria de un mercenario: un bot no manda a quien no es mercenario (`mercenario.convocatoria_de_mercenarios`).
    fn de_mercenarios(&self, convocatoria: u64) -> Option<Respuesta> {
        let lider = self.convocatoria(convocatoria)?.lider;
        self.es_marcado(lider).then(|| Respuesta::rechazo(rechazos::MERCENARIO_CONVOCATORIA_DE_MERCENARIOS, "a la convocatoria de un mercenario solo se unen mercenarios con la orden de unirse a ella", self.estado.version))
    }

    /// `UnirseABatalla` de un marcado (decisión 12): con `guardar(x)` solo en el bando que defiende `x`, y con `atacar(x)` solo en el que lo ataca. El bando que le tocaría lo dice
    /// [`Nucleo::elegibilidad`]; si no le toca ninguno, o la batalla no existe, lo rechaza la orden misma.
    fn pide_orden_en_batalla(&self, t: Instante, heroe: u64, batalla: u64, pedido: Option<Lado>) -> Option<Respuesta> {
        let contrato = self.estado.contratos.contratos.get(&heroe)?;
        let Some(orden) = contrato.orden.as_ref().filter(|o| matches!(o.tipo, TipoOrden::Guardar | TipoOrden::Atacar)) else { return Some(self.rechazo_sin_orden()) };
        let b = self.batalla(batalla).filter(|b| b.activa(t))?;
        let (lado, ..) = self.elegibilidad(t, heroe, b, pedido).ok()?;
        let de_x = self.lado_del_objetivo(b, &orden.objetivo);
        let con_x = if orden.tipo == TipoOrden::Guardar { de_x } else { de_x.map(Lado::otro) };
        (con_x != Some(lado)).then(|| self.rechazo_sin_orden())
    }

    /// Se mira justo después de `rechazo_por_batalla`, antes de despachar cualquier orden. El héroe que se mira es el que nombra la orden, el Líder de la columna en las que nombran
    /// una columna y, al ceder el liderazgo, el sucesor. Se sale pronto cuando no hay contratos ni ofertas y, en cada orden, cuando ese héroe no tiene contrato ni oferta: esto se llama
    /// en todas las órdenes de la partida.
    pub(in crate::nucleo) fn rechazo_por_contrato(&self, t: Instante, p: &Peticion) -> Option<Respuesta> {
        let c = &self.estado.contratos;
        if p.admin || (c.contratos.is_empty() && c.ofertas.is_empty()) {
            return None;
        }
        match &p.comando {
            Comando::CrearFaccion { heroe, .. } | Comando::SolicitarIngreso { heroe, .. } => self.sin_faccion_propia(*heroe),
            Comando::Fundar { ejercito } => self.sin_faccion_propia(self.ejercito(*ejercito)?.columna.lider),
            Comando::DeclararGuerra { heroe, .. } => self.prohibido_al_marcado(*heroe, "un mercenario contratado no declara la guerra"),
            Comando::ConvocarEjercito { heroe, .. } => self.convoca_para_atacar(*heroe),
            Comando::AceptarMision { heroe, dueno: Dueno::Faccion(_) | Dueno::Asentamiento(_), .. } => self.prohibido_al_marcado(*heroe, "un mercenario contratado no acepta las misiones del tablón de la Facción ni de la plaza"),
            Comando::Atacar { heroe, declara_guerra: true, .. } => self.prohibido_al_marcado(*heroe, "un mercenario contratado no declara la guerra: solo ataca con la guerra ya declarada"),
            Comando::Atacar { heroe, objetivo, declara_guerra: false } => self.pide_orden(*heroe, TipoOrden::Atacar, objetivo_de_ataque(*objetivo)),
            Comando::Perseguir { heroe, objetivo } => self.pide_orden(*heroe, TipoOrden::Atacar, objetivo_de_persecucion(*objetivo)),
            Comando::UnirseAEjercito { heroe, ejercito, .. } | Comando::UnirseEnCampo { heroe, ejercito } => self.pide_orden(*heroe, TipoOrden::Unirse, Objetivo::Ejercito(*ejercito)),
            Comando::UnirseAConvocatoria { heroe, convocatoria, .. } if self.es_marcado(*heroe) => self.pide_orden(*heroe, TipoOrden::Unirse, Objetivo::Convocatoria(*convocatoria)),
            Comando::UnirseAConvocatoria { convocatoria, .. } => self.de_mercenarios(*convocatoria),
            Comando::UnirseABatalla { heroe, batalla, lado } => self.pide_orden_en_batalla(t, *heroe, *batalla, *lado),
            Comando::AdjuntarCaravana { caravana, ejercito } => self.pide_orden(self.ejercito(*ejercito)?.columna.lider, TipoOrden::Guardar, Objetivo::Caravana(*caravana)),
            Comando::AsignarEscolta { caravana, heroe, .. } => self.pide_orden(*heroe, TipoOrden::Guardar, Objetivo::Caravana(*caravana)),
            Comando::GuarnecerAvanzada { heroe, avanzada, .. } => self.pide_orden(*heroe, TipoOrden::Guardar, Objetivo::Avanzada(*avanzada)),
            Comando::CederLiderazgo { sucesor, .. } => self.es_marcado(*sucesor).then(|| Respuesta::rechazo(rechazos::MERCENARIO_MARCADO, "un mercenario contratado no recibe el mando de un ejército", self.estado.version)),
            // Con el almacén de la plaza cerrado a los ciudadanos, un marcado tampoco recluta ni repone en ella (decisión 24): la plaza decide cuánto gastan sin tener que vetarlos.
            Comando::ReclutarTropa { asentamiento, heroe, .. } => {
                let cerrado = self.estado.asentamientos.get(asentamiento)?.almacen_cerrado_a_ciudadanos;
                (cerrado && self.es_marcado(*heroe)).then(|| Respuesta::rechazo(rechazos::ALMACEN_CERRADO, "el Gobernador ha cerrado el almacén de la plaza a los ciudadanos, y un mercenario contratado lo es", self.estado.version))
            }
            // Lo demás lo hace un mercenario como cualquier ciudadano de su Facción.
            Comando::CrearEjercito { .. }
            | Comando::MarcharA { .. }
            | Comando::Estacionar { .. }
            | Comando::MarchaForzada { .. }
            | Comando::Reloj(_)
            | Comando::FijarNivelFaccion { .. }
            | Comando::DarCaravanaFundacion { .. }
            | Comando::FijarNivelAsentamiento { .. }
            | Comando::AgrandarZona { .. }
            | Comando::ReclamarParcela { .. }
            | Comando::SoltarParcela { .. }
            | Comando::AnadirEdificio { .. }
            | Comando::QuitarDeCola { .. }
            | Comando::CancelarConstruccion { .. }
            | Comando::MoverEnCola { .. }
            | Comando::MejorarEdificio { .. }
            | Comando::AsignarBuey { .. }
            | Comando::ElegirReceta { .. }
            | Comando::AlternarAutoConstruccion { .. }
            | Comando::AlternarReceta { .. }
            | Comando::CalibrarReserva { .. }
            | Comando::SolicitarAscenso { .. }
            | Comando::ActivarPolitica { .. }
            | Comando::ProponerTrueque { .. }
            | Comando::AceptarTrueque { .. }
            | Comando::RechazarTrueque { .. }
            | Comando::ColocarOrdenMercado { .. }
            | Comando::CancelarOrdenMercado { .. }
            | Comando::ComerciarEnPlaza { .. }
            | Comando::CrearCaravana { .. }
            | Comando::AnadirCarro { .. }
            | Comando::AnadirAnimal { .. }
            | Comando::MoverCarro { .. }
            | Comando::ReservarCaravana { .. }
            | Comando::RenombrarCaravana { .. }
            | Comando::PrepararCaravana { .. }
            | Comando::CancelarCaravana { .. }
            | Comando::MoverCargaAparcada { .. }
            | Comando::EnviarCaravanaAlOrigen { .. }
            | Comando::EntregarDeCaravana { .. }
            | Comando::DarRecursos { .. }
            | Comando::DarSuministro { .. }
            | Comando::AparcarCaravana { .. }
            | Comando::ComprarEnCampamento { .. }
            | Comando::RepostarEnCampamento { .. }
            | Comando::FijarResidencia { .. }
            | Comando::DarAlmacenPersonal { .. }
            | Comando::MarcarPueblo { .. }
            | Comando::EntregarMateriales { .. }
            | Comando::DestruirPueblo { .. }
            | Comando::ReponerCarro { .. }
            | Comando::MarcarAldea { .. }
            | Comando::MejorarAldea { .. }
            | Comando::CrearHeroe { .. }
            | Comando::ReclutarEnCampamento { .. }
            | Comando::PedirPrestamo { .. }
            | Comando::ReponerPrestamo { .. }
            | Comando::GuardarLoadout { .. }
            | Comando::BorrarLoadout { .. }
            | Comando::RepartirPuntos { .. }
            | Comando::OrdenarEscuadras { .. }
            | Comando::AsignarGuarnicion { .. }
            | Comando::RetirarGuarnicion { .. }
            | Comando::PasarAViveres { .. }
            | Comando::QuitarEscolta { .. }
            | Comando::Desconectarse { .. }
            | Comando::CerrarSesion { .. }
            | Comando::Conectarse { .. }
            | Comando::FijarTecnologias { .. }
            | Comando::FijarFaccionDeHeroe { .. }
            | Comando::ColocarHeroe { .. }
            | Comando::HerirHeroe { .. }
            | Comando::SalirAlMundo { .. }
            | Comando::MovilizarEjercito { .. }
            | Comando::SalirDelCampamento { .. }
            | Comando::EntrarEnAsentamiento { .. }
            | Comando::SalirDeAsentamiento { .. }
            | Comando::Guarnecer { .. }
            | Comando::EntrarEnCampamento { .. }
            | Comando::ResponderPeticionDeUnion { .. }
            | Comando::SepararseDelEjercito { .. }
            | Comando::OrganizarEjercito { .. }
            | Comando::CancelarFormacion { .. }
            | Comando::Repostar { .. }
            | Comando::ResponderPeticionDeConvocatoria { .. }
            | Comando::CambiarSeleccionDeConvocatoria { .. }
            | Comando::SepararseDeConvocatoria { .. }
            | Comando::CancelarConvocatoria { .. }
            | Comando::PartirConvocatoria { .. }
            | Comando::DejarDePerseguir { .. }
            | Comando::Inspeccionar { .. }
            | Comando::FijarRelacion { .. }
            | Comando::ColocarBandidos { .. }
            | Comando::DarEscuadra { .. }
            | Comando::FijarControlador { .. }
            | Comando::CancelarBatalla { .. }
            | Comando::RegistrarAsignacion { .. }
            | Comando::ConfirmarInicio { .. }
            | Comando::RegistrarTokens { .. }
            | Comando::RegistrarSalida { .. }
            | Comando::AplicarResultado { .. }
            | Comando::ColocarAvanzada { .. }
            | Comando::EntregarMaterialesAAvanzada { .. }
            | Comando::MejorarAvanzada { .. }
            | Comando::RepararAvanzada { .. }
            | Comando::DestruirAvanzada { .. }
            | Comando::ElegirTropaDeAvanzada { .. }
            | Comando::RetirarDeAvanzada { .. }
            | Comando::RepostarEnAvanzada { .. }
            | Comando::DepositarEnAvanzada { .. }
            | Comando::CancelarObraDeAvanzada { .. }
            | Comando::VaciarCarro { .. }
            | Comando::EntrarEnPueblo { .. }
            | Comando::SalirDePueblo { .. }
            | Comando::EntrarEnAvanzada { .. }
            | Comando::SalirDeAvanzada { .. }
            | Comando::ComprometerRecinto { .. }
            | Comando::MejorarRecinto { .. }
            | Comando::AbandonarRecinto { .. }
            | Comando::ComprarMirada { .. }
            | Comando::ComprarInformePlaza { .. }
            | Comando::ComprarInformeRegion { .. }
            | Comando::GuardarEnAlmacenPersonal { .. }
            | Comando::SacarDelAlmacenPersonal { .. }
            | Comando::CargarCaravana { .. }
            | Comando::ReclamarCaravana { .. }
            | Comando::SoltarCaravana { .. }
            | Comando::ResponderSolicitud { .. }
            | Comando::DejarFaccion { .. }
            | Comando::AsignarRey { .. }
            | Comando::AsignarEmbajador { .. }
            | Comando::CambiarResidencia { .. }
            | Comando::ResidirEnCampamento { .. }
            | Comando::DejarResidencia { .. }
            | Comando::DesignarCapital { .. }
            | Comando::RenombrarAsentamiento { .. }
            | Comando::ProponerRelacion { .. }
            | Comando::ResponderRelacion { .. }
            | Comando::RetirarRelacion { .. }
            | Comando::ProponerPaz { .. }
            | Comando::RomperRelacion { .. }
            | Comando::RebelionVasallo { .. }
            | Comando::AdmitirOtrasFacciones { .. }
            | Comando::AlternarReabastecerAliados { .. }
            | Comando::AportarARefundacion { .. }
            | Comando::RetirarDeRefundacion { .. }
            | Comando::ComprarCaravanaDeRefundacion { .. }
            | Comando::LanzarCaravanaFundacion { .. }
            | Comando::DesarmarCaravanaFundacion { .. }
            | Comando::AsignarCargoLocal { .. }
            | Comando::FijarPuerta { .. }
            | Comando::VetarJugador { .. }
            | Comando::ProponerAnexion { .. }
            | Comando::ResponderAnexion { .. }
            | Comando::RetirarAnexion { .. }
            | Comando::ProponerFusion { .. }
            | Comando::ResponderFusion { .. }
            | Comando::RetirarFusion { .. }
            | Comando::AbrirAlijo { .. }
            | Comando::ReclamarRecompensa { .. }
            | Comando::AceptarMision { dueno: Dueno::Heroe(_), .. }
            | Comando::ElegirTablon { .. }
            | Comando::FijarCatalogoMisiones { .. }
            | Comando::GuarnecerPueblo { .. }
            | Comando::RetirarDePueblo { .. }
            | Comando::CargarDePueblo { .. }
            | Comando::DejarEnPueblo { .. }
            | Comando::CargarDePlaza { .. }
            | Comando::DejarEnPlaza { .. }
            | Comando::CerrarAlmacenACiudadanos { .. }
            | Comando::EncargarCaravana { .. }
            | Comando::TomarOrdenADistancia { .. }
            | Comando::CambiarEncargo { .. }
            | Comando::CancelarEncargo { .. }
            | Comando::DetenerEncargo { .. }
            | Comando::AdoptarTecnologia { .. }
            | Comando::ComprarTecnologiaAeda { .. }
            | Comando::EmpezarEpica { .. }
            | Comando::AbandonarEpica { .. }
            | Comando::OfrecerseComoMercenario { .. }
            | Comando::RetirarOferta { .. }
            | Comando::ContratarMercenario { .. }
            | Comando::ProrrogarContrato { .. }
            | Comando::OrdenarMercenario { .. }
            | Comando::MudarMercenario { .. }
            | Comando::DespedirMercenario { .. }
            | Comando::ValorarMercenario { .. }
            | Comando::ReclamarPremioDeRango { .. } => None,
        }
    }
}
