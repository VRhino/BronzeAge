//! Lo que entra al núcleo y lo que sale de él. Una [`Peticion`] aceptada es lo que se escribe en el registro: reproducidas
//! en orden, en sus instantes, reconstruyen el mismo mundo.

use crate::codec::{ErrorFormato, Escritor, Lector};
use base::{Instante, Mil, Punto};

/// Acción sobre el reloj de mundo (`OrdenControlarReloj`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccionReloj {
    Pausar,
    Reanudar,
    /// Segundos de mundo por segundo real × 1000.
    FijarFactor(u32),
    /// Adelanta el mundo hasta ese instante sin esperar (modo acelerado).
    AvanzarHasta(Instante),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Comando {
    /// Alta de una columna (la hará `salir_al_mundo` en la fase 6). Solo administración.
    CrearEjercito { lider: String, propietario: String, faccion: u32, posicion: Punto, velocidad: Mil, viveres: Mil, capacidad_viveres: Mil },
    /// Marcha desde la posición real al instante de la orden.
    MarcharA { ejercito: u64, destino: Punto },
    /// Cancela la marcha: la columna se queda donde está.
    Estacionar { ejercito: u64 },
    /// Solo administración.
    Reloj(AccionReloj),
    /// Nivel de una Facción (fija su cap de fundación). Solo administración; lo sustituirá la experiencia de Facción.
    FijarNivelFaccion { faccion: u32, nivel: u32 },
    /// Engancha una Caravana de Fundación a la columna. Solo administración; lo sustituirán la compra en un campamento y el
    /// lanzamiento desde una plaza.
    DarCaravanaFundacion { ejercito: u64 },
    /// Funda una plaza donde está la columna, gastando su caravana (Doc 1.8).
    Fundar { ejercito: u64 },
    /// Sube el nivel de una plaza (y con él el techo de su zona). Solo administración; lo sustituirá el ascenso.
    FijarNivelAsentamiento { asentamiento: u64, nivel: u32 },
    /// Agranda la zona de una plaza (un edificio completado: 1 unidad). Solo administración; lo llamará la construcción.
    AgrandarZona { asentamiento: u64, milesimas: Mil },
    /// Una Leñera pide su parcela de bosque (Doc 1.9c). Solo administración; lo llamará la construcción.
    ReclamarParcela { asentamiento: u64 },
    /// Una Leñera destruida devuelve su parcela. Solo administración.
    SoltarParcela { asentamiento: u64, parcela: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peticion {
    /// Cliente que la manda; clave de la idempotencia junto con `peticion_id`.
    pub actor: String,
    pub admin: bool,
    pub peticion_id: String,
    pub version_esperada: Option<u64>,
    pub comando: Comando,
}

impl Peticion {
    pub fn nueva(actor: &str, peticion_id: &str, comando: Comando) -> Self {
        Self { actor: actor.into(), admin: false, peticion_id: peticion_id.into(), version_esperada: None, comando }
    }
    pub fn de_admin(mut self) -> Self {
        self.admin = true;
        self
    }

    pub(crate) fn codificar(&self) -> Vec<u8> {
        let mut w = Escritor::default();
        w.texto(&self.actor);
        w.bool(self.admin);
        w.texto(&self.peticion_id);
        w.opt_u64(self.version_esperada);
        match &self.comando {
            Comando::CrearEjercito { lider, propietario, faccion, posicion, velocidad, viveres, capacidad_viveres } => {
                w.u8(1);
                w.texto(lider);
                w.texto(propietario);
                w.u32(*faccion);
                w.punto(*posicion);
                w.i64(*velocidad);
                w.i64(*viveres);
                w.i64(*capacidad_viveres);
            }
            Comando::MarcharA { ejercito, destino } => {
                w.u8(2);
                w.u64(*ejercito);
                w.punto(*destino);
            }
            Comando::Estacionar { ejercito } => {
                w.u8(3);
                w.u64(*ejercito);
            }
            Comando::Reloj(a) => {
                w.u8(4);
                match a {
                    AccionReloj::Pausar => w.u8(0),
                    AccionReloj::Reanudar => w.u8(1),
                    AccionReloj::FijarFactor(f) => {
                        w.u8(2);
                        w.u32(*f);
                    }
                    AccionReloj::AvanzarHasta(t) => {
                        w.u8(3);
                        w.i64(*t);
                    }
                }
            }
            Comando::FijarNivelFaccion { faccion, nivel } => {
                w.u8(5);
                w.u32(*faccion);
                w.u32(*nivel);
            }
            Comando::DarCaravanaFundacion { ejercito } => {
                w.u8(6);
                w.u64(*ejercito);
            }
            Comando::Fundar { ejercito } => {
                w.u8(7);
                w.u64(*ejercito);
            }
            Comando::FijarNivelAsentamiento { asentamiento, nivel } => {
                w.u8(8);
                w.u64(*asentamiento);
                w.u32(*nivel);
            }
            Comando::AgrandarZona { asentamiento, milesimas } => {
                w.u8(9);
                w.u64(*asentamiento);
                w.i64(*milesimas);
            }
            Comando::ReclamarParcela { asentamiento } => {
                w.u8(10);
                w.u64(*asentamiento);
            }
            Comando::SoltarParcela { asentamiento, parcela } => {
                w.u8(11);
                w.u64(*asentamiento);
                w.u32(*parcela);
            }
        }
        w.0
    }

    pub(crate) fn decodificar(bytes: &[u8]) -> Result<Peticion, ErrorFormato> {
        let mut r = Lector::nuevo(bytes);
        let (actor, admin, peticion_id, version_esperada) = (r.texto()?, r.bool()?, r.texto()?, r.opt_u64()?);
        let comando = match r.u8()? {
            1 => Comando::CrearEjercito {
                lider: r.texto()?,
                propietario: r.texto()?,
                faccion: r.u32()?,
                posicion: r.punto()?,
                velocidad: r.i64()?,
                viveres: r.i64()?,
                capacidad_viveres: r.i64()?,
            },
            2 => Comando::MarcharA { ejercito: r.u64()?, destino: r.punto()? },
            3 => Comando::Estacionar { ejercito: r.u64()? },
            4 => Comando::Reloj(match r.u8()? {
                0 => AccionReloj::Pausar,
                1 => AccionReloj::Reanudar,
                2 => AccionReloj::FijarFactor(r.u32()?),
                3 => AccionReloj::AvanzarHasta(r.i64()?),
                _ => return Err(ErrorFormato("acción de reloj desconocida")),
            }),
            5 => Comando::FijarNivelFaccion { faccion: r.u32()?, nivel: r.u32()? },
            6 => Comando::DarCaravanaFundacion { ejercito: r.u64()? },
            7 => Comando::Fundar { ejercito: r.u64()? },
            8 => Comando::FijarNivelAsentamiento { asentamiento: r.u64()?, nivel: r.u32()? },
            9 => Comando::AgrandarZona { asentamiento: r.u64()?, milesimas: r.i64()? },
            10 => Comando::ReclamarParcela { asentamiento: r.u64()? },
            11 => Comando::SoltarParcela { asentamiento: r.u64()?, parcela: r.u32()? },
            _ => return Err(ErrorFormato("comando desconocido")),
        };
        r.fin()?;
        Ok(Peticion { actor, admin, peticion_id, version_esperada, comando })
    }
}

/// Códigos de rechazo estables (`RechazoOrden.codigo`).
pub mod rechazos {
    pub const NO_AUTORIZADO: &str = "no_autorizado";
    pub const VERSION_OBSOLETA: &str = "version.obsoleta";
    pub const EJERCITO_DESCONOCIDO: &str = "ejercito.desconocido";
    pub const HEROE_DESCONOCIDO: &str = "heroe.desconocido";
    pub const RUTA_IMPOSIBLE: &str = "ruta.imposible";
    pub const ORDEN_NO_SOPORTADA: &str = "orden.no_soportada";
    pub const OBJETIVO_NO_SOPORTADO: &str = "objetivo.no_soportado";
    pub const RELOJ_INVALIDO: &str = "reloj.invalido";
    pub const LIDER_REPETIDO: &str = "heroe.ya_tiene_columna";
    pub const SIN_MUNDO: &str = "mundo.no_cargado";
    pub const ASENTAMIENTO_DESCONOCIDO: &str = "asentamiento.desconocido";
    pub const SIN_CARAVANA: &str = "fundacion.sin_caravana";
    pub const CAP_ALCANZADO: &str = "fundacion.cap_alcanzado";
    pub const NIVEL_INVALIDO: &str = "nivel.invalido";
    pub const ZONA_INVALIDA: &str = "zona.invalida";
    pub const PARCELA_SIN_LIBRES: &str = "parcela.sin_libres";
    pub const PARCELA_CUPO_AGOTADO: &str = "parcela.cupo_agotado";
    pub const PARCELA_AJENA: &str = "parcela.no_es_tuya";
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resultado {
    Aceptada,
    Rechazada { codigo: &'static str, mensaje: String },
    /// Válida pero no cambió nada: no sube la versión.
    SinCambios,
    /// Misma `peticion_id` ya aceptada: no se ejecuta otra vez.
    Duplicada,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Respuesta {
    pub resultado: Resultado,
    /// Versión de mundo con los efectos ya aplicados.
    pub version_tras: u64,
    /// Ejércitos o plazas creados por la orden.
    pub creadas: Vec<u64>,
}

impl Respuesta {
    pub(crate) fn rechazo(codigo: &'static str, mensaje: impl Into<String>, version: u64) -> Self {
        Self { resultado: Resultado::Rechazada { codigo, mensaje: mensaje.into() }, version_tras: version, creadas: vec![] }
    }
    pub fn aceptada(&self) -> bool {
        self.resultado == Resultado::Aceptada
    }
}
