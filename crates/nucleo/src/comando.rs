//! Lo que entra al núcleo y lo que sale de él. Una [`Peticion`] aceptada es lo que se escribe en el registro: reproducidas
//! en orden, en sus instantes, reconstruyen el mismo mundo.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_economia::{cargo_a_u8, cargo_de_u8, recurso_a_u8, recurso_de_u8, tipo_a_u8, tipo_de_u8};
use asentamiento::{Cargo, Recurso, Tipo};
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
    /// Un edificio completado agranda la zona de su plaza (`crecimiento_por_edificio`, hasta el techo del nivel). Solo
    /// administración; lo llamará la construcción.
    AgrandarZona { asentamiento: u64 },
    /// Una Leñera pide su parcela de bosque (Doc 1.9c). Solo administración; lo llamará la construcción.
    ReclamarParcela { asentamiento: u64 },
    /// Una Leñera destruida devuelve su parcela. Solo administración.
    SoltarParcela { asentamiento: u64, parcela: u32 },
    // ── Economía y construcción de una plaza (Economia_Construccion §13). Las puede dar la administración o quien sea propietario de
    //    alguna columna de la Facción de la plaza; los cargos (Gobernador, Maestro de Obras, Tesorero) los traerá el sistema de residentes. ──
    /// Encola un edificio (paga el coste completo al encolar). `creadas` lleva el id del edificio nuevo.
    AnadirEdificio { asentamiento: u64, tipo: Tipo },
    /// Saca de la cola un edificio que aún no empezó y devuelve su coste.
    QuitarDeCola { asentamiento: u64, edificio: u32 },
    /// Sube o baja un puesto en la cola de obras.
    MoverEnCola { asentamiento: u64, edificio: u32, arriba: bool },
    /// Mejora un edificio activo al siguiente nivel interno.
    MejorarEdificio { asentamiento: u64, edificio: u32 },
    /// Pausa o reanuda la auto-construcción (solo la detección de necesidades nuevas).
    AlternarAutoConstruccion { asentamiento: u64, pausada: bool },
    /// Para o reanuda una receta por el recurso que produce, en todos los talleres.
    AlternarReceta { asentamiento: u64, recurso: Recurso, pausada: bool },
    /// Reserva manual del Tesorero para un recurso (milésimas; se recorta al máximo).
    CalibrarReserva { asentamiento: u64, recurso: Recurso, valor: Mil },
    /// Pide la subida al nivel siguiente (obra de ascenso).
    SolicitarAscenso { asentamiento: u64 },
    /// Activa una política con la duración por defecto, en el cargo indicado.
    ActivarPolitica { asentamiento: u64, politica: String, cargo: Cargo },
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
            Comando::AgrandarZona { asentamiento } => {
                w.u8(9);
                w.u64(*asentamiento);
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
            Comando::AnadirEdificio { asentamiento, tipo } => {
                w.u8(12);
                w.u64(*asentamiento);
                w.u8(tipo_a_u8(*tipo));
            }
            Comando::QuitarDeCola { asentamiento, edificio } => {
                w.u8(13);
                w.u64(*asentamiento);
                w.u32(*edificio);
            }
            Comando::MoverEnCola { asentamiento, edificio, arriba } => {
                w.u8(14);
                w.u64(*asentamiento);
                w.u32(*edificio);
                w.bool(*arriba);
            }
            Comando::MejorarEdificio { asentamiento, edificio } => {
                w.u8(15);
                w.u64(*asentamiento);
                w.u32(*edificio);
            }
            Comando::AlternarAutoConstruccion { asentamiento, pausada } => {
                w.u8(16);
                w.u64(*asentamiento);
                w.bool(*pausada);
            }
            Comando::AlternarReceta { asentamiento, recurso, pausada } => {
                w.u8(17);
                w.u64(*asentamiento);
                w.u8(recurso_a_u8(*recurso));
                w.bool(*pausada);
            }
            Comando::CalibrarReserva { asentamiento, recurso, valor } => {
                w.u8(18);
                w.u64(*asentamiento);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*valor);
            }
            Comando::SolicitarAscenso { asentamiento } => {
                w.u8(19);
                w.u64(*asentamiento);
            }
            Comando::ActivarPolitica { asentamiento, politica, cargo } => {
                w.u8(20);
                w.u64(*asentamiento);
                w.texto(politica);
                w.u8(cargo_a_u8(*cargo));
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
            9 => Comando::AgrandarZona { asentamiento: r.u64()? },
            10 => Comando::ReclamarParcela { asentamiento: r.u64()? },
            11 => Comando::SoltarParcela { asentamiento: r.u64()?, parcela: r.u32()? },
            12 => Comando::AnadirEdificio { asentamiento: r.u64()?, tipo: tipo_de_u8(r.u8()?)? },
            13 => Comando::QuitarDeCola { asentamiento: r.u64()?, edificio: r.u32()? },
            14 => Comando::MoverEnCola { asentamiento: r.u64()?, edificio: r.u32()?, arriba: r.bool()? },
            15 => Comando::MejorarEdificio { asentamiento: r.u64()?, edificio: r.u32()? },
            16 => Comando::AlternarAutoConstruccion { asentamiento: r.u64()?, pausada: r.bool()? },
            17 => Comando::AlternarReceta { asentamiento: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, pausada: r.bool()? },
            18 => Comando::CalibrarReserva { asentamiento: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, valor: r.i64()? },
            19 => Comando::SolicitarAscenso { asentamiento: r.u64()? },
            20 => Comando::ActivarPolitica { asentamiento: r.u64()?, politica: r.texto()?, cargo: cargo_de_u8(r.u8()?)? },
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
    pub const PARCELA_SIN_LIBRES: &str = "parcela.sin_libres";
    pub const PARCELA_CUPO_AGOTADO: &str = "parcela.cupo_agotado";
    pub const PARCELA_AJENA: &str = "parcela.no_es_tuya";
    // Obras (`ErrorObra` del crate `asentamiento`).
    pub const OBRA_TECNOLOGIA_FALTANTE: &str = "obra.tecnologia_faltante";
    pub const OBRA_ERA_INSUFICIENTE: &str = "obra.era_insuficiente";
    pub const OBRA_TIPO_NO_MANUAL: &str = "obra.tipo_no_manual";
    pub const OBRA_UNICO: &str = "obra.unico";
    pub const OBRA_TOPE_ALMACENES: &str = "obra.tope_almacenes";
    pub const OBRA_TOPE_VIVIENDAS: &str = "obra.tope_viviendas";
    pub const OBRA_NIVEL_INSUFICIENTE: &str = "obra.nivel_insuficiente";
    pub const OBRA_TOPE_TRANSFORMACION: &str = "obra.tope_transformacion";
    pub const OBRA_GRAN_FUNDICION: &str = "obra.gran_fundicion";
    pub const OBRA_COLA_LLENA: &str = "obra.cola_llena";
    pub const OBRA_SIN_SITIO: &str = "obra.sin_sitio";
    pub const OBRA_RECURSOS_INSUFICIENTES: &str = "obra.recursos_insuficientes";
    pub const EDIFICIO_DESCONOCIDO: &str = "edificio.desconocido";
    pub const OBRA_NO_ESTA_EN_COLA: &str = "obra.no_esta_en_cola";
    pub const OBRA_NO_MEJORABLE: &str = "obra.no_mejorable";
    pub const OBRA_SIN_CUADRILLA: &str = "obra.sin_cuadrilla";
    // Ascenso (`Bloqueo`): el rechazo lleva el código del primero y todos en el mensaje.
    pub const ASCENSO_NIVEL_MAXIMO: &str = "ascenso.nivel_maximo";
    pub const ASCENSO_EN_CURSO: &str = "ascenso.en_curso";
    pub const ASCENSO_FALTA_POBLACION: &str = "ascenso.falta_poblacion";
    pub const ASCENSO_FALTAN_EDIFICIOS: &str = "ascenso.faltan_edificios";
    pub const ASCENSO_SIN_CUPO_DE_FACCION: &str = "ascenso.sin_cupo_de_faccion";
    pub const ASCENSO_RECURSOS_INSUFICIENTES: &str = "ascenso.recursos_insuficientes";
    pub const ASCENSO_INSOLVENTE: &str = "ascenso.insolvente";
    // Políticas (`ErrorPolitica`).
    pub const POLITICA_DESCONOCIDA: &str = "politica.desconocida";
    pub const POLITICA_FUERA_DE_POOL: &str = "politica.fuera_de_pool";
    pub const POLITICA_CARGO_VACIO: &str = "politica.cargo_vacio";
    pub const POLITICA_YA_ACTIVA: &str = "politica.ya_activa";
    pub const POLITICA_SIN_SLOT: &str = "politica.sin_slot";
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

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn las_ordenes_de_economia_dan_la_vuelta_por_el_registro() {
        let comandos = vec![
            Comando::AnadirEdificio { asentamiento: 3, tipo: Tipo::Lenera },
            Comando::QuitarDeCola { asentamiento: 3, edificio: 17 },
            Comando::MoverEnCola { asentamiento: 3, edificio: 17, arriba: true },
            Comando::MejorarEdificio { asentamiento: 3, edificio: 4 },
            Comando::AlternarAutoConstruccion { asentamiento: 3, pausada: true },
            Comando::AlternarReceta { asentamiento: 3, recurso: Recurso::LingoteBronce, pausada: false },
            Comando::CalibrarReserva { asentamiento: 3, recurso: Recurso::Madera, valor: 12_345 },
            Comando::SolicitarAscenso { asentamiento: 3 },
            Comando::ActivarPolitica { asentamiento: 3, politica: "racionamiento".into(), cargo: Cargo::Sacerdote },
        ];
        for c in comandos {
            let p = Peticion { actor: "ana".into(), admin: false, peticion_id: "x".into(), version_esperada: Some(9), comando: c };
            assert_eq!(Peticion::decodificar(&p.codificar()).unwrap(), p);
        }
        // Un tipo o un recurso fuera de rango no se decodifica.
        let mut b = Peticion::nueva("a", "i", Comando::AnadirEdificio { asentamiento: 1, tipo: Tipo::Granja }).codificar();
        *b.last_mut().unwrap() = 200;
        assert!(Peticion::decodificar(&b).is_err());
    }
}
