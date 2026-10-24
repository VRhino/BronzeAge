//! Lo que entra al núcleo y lo que sale de él. Una [`Peticion`] aceptada es lo que se escribe en el registro: reproducidas
//! en orden, en sus instantes, reconstruyen el mismo mundo.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::estado::Clase;
use crate::codec_economia::{cargo_a_u8, cargo_de_u8, recurso_a_u8, recurso_de_u8, tipo_a_u8, tipo_de_u8};
use crate::codec_comercio::{animal_a_u8, animal_de_u8, tipo_carro_a_u8, tipo_carro_de_u8};
use crate::codec_batallas::{asignacion_escribir, asignacion_leer, resultado_escribir, resultado_leer, salida_escribir, salida_leer, tokens_escribir, tokens_leer};
use crate::codec_columna::{politica_a_u8, politica_de_u8};
use crate::codec_politica::{sigilo_escribir, sigilo_leer};
use crate::codec_militar::{controlador_a_u8, controlador_de_u8, objetivo_escribir, objetivo_leer, relacion_a_u8, relacion_de_u8, residencia_escribir, residencia_leer, tropa_a_u8, tropa_de_u8, ubicacion_escribir, ubicacion_leer};
use crate::estado_militar::Relacion;
pub use crate::memoria::OrigenIntel;
use asentamiento::{Cargo, Recurso, Tipo};
use base::{Instante, Mil, Punto};
use combate::ciclo::{Asignacion, SalidaBatalla, Token};
use combate::resolucion::Lado;
use combate::resultado::Resultado as ResultadoDeBatalla;
use comercio::caravana::{Animal, TipoCarro};
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::heroe::{Controlador, Residencia};
use militar::inspeccion::Objetivo;
use militar::presencia::Ubicacion;
use politica::sigilo::Sigilo;

/// Lo que se puede atacar (`OrdenAtacar.Objetivo`, Combate §2): una columna, una caravana, un campamento de bandidos, una plaza, un pueblo extractor o
/// una avanzada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjetivoAtaque {
    Ejercito(u64),
    Caravana(u64),
    Campamento(u64),
    Asentamiento(u64),
    Pueblo(u64),
    Avanzada(u64),
}

fn objetivo_de_ataque_escribir(w: &mut Escritor, o: &ObjetivoAtaque) {
    let (tag, id) = match *o {
        ObjetivoAtaque::Ejercito(x) => (0, x),
        ObjetivoAtaque::Caravana(x) => (1, x),
        ObjetivoAtaque::Campamento(x) => (2, x),
        ObjetivoAtaque::Asentamiento(x) => (3, x),
        ObjetivoAtaque::Pueblo(x) => (4, x),
        ObjetivoAtaque::Avanzada(x) => (5, x),
    };
    w.u8(tag);
    w.u64(id);
}

fn objetivo_de_ataque_leer(r: &mut Lector) -> Result<ObjetivoAtaque, ErrorFormato> {
    let (tag, id) = (r.u8()?, r.u64()?);
    Ok(match tag {
        0 => ObjetivoAtaque::Ejercito(id),
        1 => ObjetivoAtaque::Caravana(id),
        2 => ObjetivoAtaque::Campamento(id),
        3 => ObjetivoAtaque::Asentamiento(id),
        4 => ObjetivoAtaque::Pueblo(id),
        5 => ObjetivoAtaque::Avanzada(id),
        _ => return Err(ErrorFormato("objetivo de ataque desconocido")),
    })
}

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
    /// Alta de una columna con su héroe de campo (de la cuenta dada, sin residencia ni loadout), quieta en `posicion`. Andamiaje de administración para
    /// escenarios y pruebas: la columna nace como ejército de un participante con la velocidad fija dada (`Ejercito::velocidad_fija`); salir de verdad es
    /// `SalirAlMundo`, `MovilizarEjercito` y `SalirDelCampamento`. Solo administración.
    CrearEjercito { cuenta: String, faccion: u32, posicion: Punto, velocidad: Mil, viveres: Mil },
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
    // ── Comercio (Comercio_Caravanas §4-§5, §8). Las puede dar la administración o el propietario de alguna columna de la Facción de la plaza
    //    (en el trueque, la del lado al que le toca); los cargos, la residencia y la presencia vendrán del sistema de héroes y residentes. ──
    /// Propone un trueque de `a` a `b`: `lineas_a` salen de A, `lineas_b` de B. `creadas` lleva el id del acuerdo.
    ProponerTrueque { a: u64, b: u64, lineas_a: Vec<(Recurso, Mil)>, lineas_b: Vec<(Recurso, Mil)> },
    /// Acepta una propuesta (la plaza B): el plazo para cumplir se cuenta desde el sí.
    AceptarTrueque { acuerdo: u64 },
    /// Rechaza una propuesta (la plaza B).
    RechazarTrueque { acuerdo: u64 },
    /// Coloca una orden de compra o de venta en el mercado de la plaza. `precio` `None` = el de referencia. `creadas` lleva el id de la orden.
    ColocarOrdenMercado { asentamiento: u64, venta: bool, recurso: Recurso, cantidad: Mil, precio: Option<i64> },
    /// Retira una orden activa antes de su plazo (no figura en el documento de mecánicas: lo pidió el autor al encargar la fase 3B-1).
    CancelarOrdenMercado { orden: u64 },
    /// El Líder de la columna toma una orden en el mostrador de la plaza, con su columna en la puerta.
    ComerciarEnPlaza { ejercito: u64, orden: u64, cantidad: Mil },
    /// Crea un casco de caravana comercial en la plaza (Mercado activo, cooldown y cupo). `creadas` lleva el id de la caravana.
    CrearCaravana { asentamiento: u64 },
    /// Fabrica un carro (sin animal) y lo monta en la caravana, pagando del almacén de su origen.
    AnadirCarro { caravana: u64, tipo: TipoCarro },
    /// Compra un animal (oro del almacén del origen) para el carro `carro` de la caravana.
    AnadirAnimal { caravana: u64, carro: u32, animal: Animal },
    /// Pasa el carro `carro` (con su animal) de la caravana `de` a la caravana `a`, del mismo origen.
    MoverCarro { de: u64, a: u64, carro: u32 },
    /// Pone o quita `reservadaManual` (fuera del reparto automático).
    ReservarCaravana { caravana: u64, reservada: bool },
    /// Lanza a mano una caravana: reserva la carga del almacén y queda `preparando` (o sale ya, con un solo carro y sin `salir_en`).
    PrepararCaravana { caravana: u64, destino: u64, carga: Vec<(Recurso, Mil)>, salir_en: Option<Instante> },
    /// Cancela una caravana `preparando`: la carga vuelve al almacén de su origen (topada por su capacidad).
    CancelarCaravana { caravana: u64 },
    /// Carga o descarga una caravana aparcada contra el almacén de la plaza que la hospeda.
    MoverCargaAparcada { caravana: u64, recurso: Recurso, cantidad: Mil, cargar: bool },
    /// Manda una caravana aparcada a su origen: vacía, al instante; con carga, viajando (vuelca lo que lleva al llegar).
    EnviarCaravanaAlOrigen { caravana: u64 },
    /// Entrega manual de una caravana adjunta a la columna, a la puerta de la plaza que recibe (§4.5).
    EntregarDeCaravana { ejercito: u64, caravana: u64, acuerdo: u64 },
    // ── Andamiaje de administración, provisional hasta que existan los sistemas que lo sustituyen. ──
    /// Añade recursos al almacén de una plaza (fuera del diseño: no es una fuente de juego). Solo administración.
    DarRecursos { asentamiento: u64, recurso: Recurso, cantidad: Mil },
    /// Añade al carro de una columna. Solo administración: es la fuente de carga de las columnas hasta que el botín del combate (fase 4D) cargue los carros;
    /// los jugadores cargan su carro al salir de un lugar.
    DarSuministro { ejercito: u64, recurso: Recurso, cantidad: Mil },
    /// Engancha una caravana `disponible` o `aparcada` a una columna de su Facción que está a la puerta de la plaza donde la caravana está
    /// (Movimiento_Ejercitos §9). La columna pasa a viajar a la velocidad de su elemento más lento y la caravana va con ella.
    AdjuntarCaravana { caravana: u64, ejercito: u64 },
    /// Deja la caravana adjunta aparcada en una plaza de su Facción, con la columna a su puerta o dentro de ella (Doc 3.13.7; Movimiento_Ejercitos §11). Es una orden
    /// explícita: entrar en un lugar no aparca las caravanas, entran con la columna.
    AparcarCaravana { caravana: u64, asentamiento: u64 },
    // ── Mercado de los campamentos de mercenarios (Comercio_Caravanas §9). `campamento` es el índice del artefacto de mundo; el héroe debe estar
    //    dentro del campamento o con su columna a la puerta. ──
    /// Compra `cantidad` (se sirven unidades enteras) de un bien. Un héroe residente paga de su almacén personal y compra con cupo diario; uno
    /// visitante paga con el oro del carro de su columna, lleva la compra al carro y no compra trigo.
    ComprarEnCampamento { heroe: u64, campamento: u32, recurso: Recurso, cantidad: Mil },
    /// Llena los víveres del héroe (los de su columna, si va en una), gratis y sin gastar el stock del mercado.
    RepostarEnCampamento { heroe: u64, campamento: u32 },
    // ── Andamiaje de administración, provisional hasta el sistema político (residencia, ciudadanía y almacén personal) y el de columnas. ──
    /// Fija (o quita, con `None`) la residencia del héroe: una plaza o un campamento. Al mudarse suelta la guarnición y se retira la tropa
    /// prestada de otro campamento. Solo administración; lo sustituirán `cambiarResidencia`, `residirEnCampamento` y la fundación.
    FijarResidencia { heroe: u64, residencia: Option<Residencia> },
    /// Añade al almacén personal de un héroe con residencia. Solo administración; lo sustituirá `guardarEnAlmacenPersonal` con las columnas.
    DarAlmacenPersonal { heroe: u64, recurso: Recurso, cantidad: Mil },
    // ── Pueblos extractores y obras de mapa (Economia_Construccion §14, Doc 4.2.2). Las puede dar la administración o el propietario de alguna
    //    columna de la Facción de la plaza madre (el Gobernador o el Maestro de Obras presentes vendrán del sistema de héroes y residentes); la
    //    entrega de materiales, el propietario de una columna de esa Facción a la puerta de la obra. ──
    /// Marca la obra de un pueblo extractor sobre un nodo de la zona de la plaza madre: queda «pendiente de materiales» y ocupa cupo. `creadas`
    /// lleva el id del pueblo.
    MarcarPueblo { asentamiento: u64, nodo: u32 },
    /// La columna entrega a la obra pendiente los materiales que lleva (su carro y sus caravanas adjuntas), con la columna a la puerta.
    EntregarMateriales { ejercito: u64, pueblo: u64 },
    /// Cancela la obra o destruye el pueblo a mano: libera el cupo y no recupera nada (ni lo entregado ni lo almacenado).
    DestruirPueblo { pueblo: u64 },
    /// Compra en la plaza madre un carro de acarreo nuevo (carro + buey) para un pueblo que se quedó sin él.
    ReponerCarro { pueblo: u64 },
    // ── Héroes y tropa (Heroe_Y_Tropa, Campamentos_Mercenarios §5-§6). Las da la cuenta del héroe o la administración; la cuenta solo manda en
    //    los suyos. Un héroe fuera del mundo solo puede conectarse. ──
    /// Crea el héroe de una cuenta dentro de un campamento de mercenarios, donde reside. `creadas` lleva su id. El controlador `Bot` solo lo
    /// pone la administración.
    CrearHeroe { cuenta: String, campamento: u32, controlador: Controlador },
    /// Recluta tropa en una plaza (las 11 condiciones de `reclutarTropa`) o repone la escuadra que ya se tiene. `creadas` lleva el id de la
    /// escuadra si es nueva.
    ReclutarTropa { asentamiento: u64, heroe: u64, tropa: Tropa },
    /// Recluta tropa en el campamento donde reside el héroe, con el oro de su almacén personal o el de su columna. `creadas` lleva el id de la
    /// escuadra si es nueva.
    ReclutarEnCampamento { heroe: u64, tropa: Tropa, pagar_con: PagarCon },
    /// Pide prestada tropa de leva comunal en el campamento donde reside el héroe (gratis). `creadas` lleva los ids de las escuadras.
    PedirPrestamo { heroe: u64, tropas: Vec<Tropa> },
    /// Repone gratis la tropa prestada que está en el campamento o en una columna a su puerta.
    ReponerPrestamo { heroe: u64 },
    /// Guarda un loadout (nuevo si `loadout` es `None`) o reescribe uno. `creadas` lleva el id del nuevo.
    GuardarLoadout { heroe: u64, loadout: Option<u64>, nombre: String, escuadras: Vec<u64>, perks: Vec<u32>, activo: Option<bool> },
    BorrarLoadout { heroe: u64, loadout: u64 },
    /// Reparte puntos de atributo (cada punto suma 1). Todo o nada.
    RepartirPuntos { heroe: u64, fuerza: i64, destreza: i64, armadura: i64, vitalidad: i64 },
    /// Pone las escuadras pedidas primero, en ese orden: el orden en que entran en combate.
    OrdenarEscuadras { heroe: u64, escuadras: Vec<u64> },
    /// Entrega una escuadra del campamento a la guarnición de la plaza donde reside el héroe, dentro de su cupo.
    AsignarGuarnicion { heroe: u64, escuadra: u64 },
    RetirarGuarnicion { heroe: u64, escuadra: u64 },
    /// Pasa trigo del carro a los víveres de la columna del héroe (nunca al revés): `min(pedido, trigo del carro, hueco)`.
    PasarAViveres { heroe: u64, cantidad: Mil },
    /// Cede escuadras del héroe (que reside en el origen) como escolta de una caravana parada en su origen.
    AsignarEscolta { caravana: u64, heroe: u64, escuadras: Vec<u64> },
    /// Retira la escolta que cedió el héroe (todas sus cedidas si `escuadras` es `None`).
    QuitarEscolta { caravana: u64, heroe: u64, escuadras: Option<Vec<u64>> },
    /// Pide salir del mundo: el evento de salida vence a `retardoDesconexionMs`. Solo dentro de una plaza o de un campamento.
    Desconectarse { heroe: u64 },
    /// Cancela la salida pendiente o vuelve al mundo.
    Conectarse { heroe: u64 },
    // ── Andamiaje de administración, provisional hasta que existan los sistemas que lo sustituyen. ──
    /// Da a la Facción esas tecnologías (sustituye a las que tenía), además de las de arranque. Solo administración; lo sustituirá el sistema 6.
    FijarTecnologias { faccion: u32, tecnologias: Vec<String> },
    /// Fija la Facción de la que es ciudadano el héroe. Solo administración; la ciudadanía es del sistema político.
    FijarFaccionDeHeroe { heroe: u64, faccion: Option<u32> },
    /// Coloca al héroe en una plaza, un campamento o una columna (que no lleve ya a otro héroe). Solo administración; lo sustituirán
    /// `salirAlMundo`, `entrarEnAsentamiento`, `entrarEnCampamento` y `salirDelCampamento` con las columnas.
    ColocarHeroe { heroe: u64, ubicacion: Ubicacion },
    /// Hiere al héroe (`heridoMinutos`). Solo administración; lo hará el combate.
    HerirHeroe { heroe: u64 },
    // ── Columnas (Movimiento_Ejercitos, Presencia_Y_Convocatoria). Las da la cuenta del héroe o la administración; las que nombran un ejército las da
    //    su Líder o, donde se dice, cualquiera de sus participantes. Sale y entra el héroe con su columna: `creadas` lleva el id de la columna que nace. ──
    /// Sale a lo suyo desde su residencia (una plaza): columna personal quieta en la puerta, con las escuadras y la carga que elige (puede salir sin tropa).
    SalirAlMundo { asentamiento: u64, heroe: u64, escuadras: Vec<u64>, carga: Vec<(Recurso, Mil)> },
    /// Sale como ejército desde su residencia (al menos una escuadra), con la política de unión que fija y no cambia.
    MovilizarEjercito { asentamiento: u64, heroe: u64, escuadras: Vec<u64>, carga: Vec<(Recurso, Mil)>, politica: Politica },
    /// Sale de un campamento de mercenarios: el residente elige tropa y carga de su almacén personal (y recibe la ración gratis) y se lleva el carro que
    /// trajo; el visitante sale con todo lo que entró (su columna). Con política distinta de `Rechazar` sale como ejército.
    SalirDelCampamento { campamento: u32, heroe: u64, escuadras: Vec<u64>, carga: Vec<(Recurso, Mil)>, politica: Politica },
    /// Entra en una plaza, a cuya puerta está, con todo lo suyo (tropa, carro, víveres y caravanas adjuntas): si la plaza le abre su puerta según su relación (la
    /// misma Facción y los residentes, siempre). En su residencia la tropa vuelve al campamento y el carro se queda con él; de visita todo queda en su columna, dentro.
    EntrarEnAsentamiento { asentamiento: u64, heroe: u64 },
    /// Sale de una plaza ajena con todo lo que entró (su columna vuelve al mapa, en la puerta).
    SalirDeAsentamiento { asentamiento: u64, heroe: u64 },
    /// Un ejército, a la puerta de una plaza que le abre su puerta, entra con todo (caravanas adjuntas incluidas, sean de donde sean) y se desarma; cada héroe entra
    /// como lo haría solo. Solo su Líder.
    Guarnecer { asentamiento: u64, heroe: u64 },
    /// Entra en un campamento de mercenarios con todo lo suyo (solo, o el ejército entero desarmándose; solo su Líder).
    EntrarEnCampamento { campamento: u32, heroe: u64 },
    /// Se suma, desde su plaza, a un ejército que pasa por su puerta, con las escuadras y la carga que saca.
    UnirseAEjercito { asentamiento: u64, heroe: u64, ejercito: u64, escuadras: Vec<u64>, carga: Vec<(Recurso, Mil)> },
    /// Se une en campo con su columna personal a un ejército o a una formación (a ≤ `radioEncuentro`); con `Preguntar` deja una petición de 10 s.
    UnirseEnCampo { heroe: u64, ejercito: u64 },
    /// El Líder acepta o rechaza la petición de unión de `solicitante`.
    ResponderPeticionDeUnion { heroe: u64, ejercito: u64, solicitante: u64, aceptar: bool },
    /// Sale del ejército con lo suyo y nace como columna personal donde estaba. `creadas` lleva la columna.
    SepararseDelEjercito { heroe: u64 },
    /// El Líder cede el mando a otro participante.
    CederLiderazgo { heroe: u64, sucesor: u64 },
    /// Una columna personal sola organiza un ejército en campo: queda quieta esperando a ser tres.
    OrganizarEjercito { heroe: u64, politica: Politica },
    /// El Líder deshace su formación.
    CancelarFormacion { heroe: u64 },
    /// El Líder llena los víveres de la columna en la puerta de una plaza que le abre su almacén.
    Repostar { heroe: u64, asentamiento: u64 },
    /// Convoca un ejército dentro del lugar donde está. `creadas` lleva la convocatoria.
    ConvocarEjercito { heroe: u64, escuadras: Vec<u64>, carga: Vec<(Recurso, Mil)>, politica: Politica },
    UnirseAConvocatoria { heroe: u64, convocatoria: u64, escuadras: Vec<u64>, carga: Vec<(Recurso, Mil)> },
    ResponderPeticionDeConvocatoria { heroe: u64, convocatoria: u64, solicitante: u64, aceptar: bool },
    CambiarSeleccionDeConvocatoria { heroe: u64, escuadras: Vec<u64>, carga: Vec<(Recurso, Mil)> },
    SepararseDeConvocatoria { heroe: u64 },
    CancelarConvocatoria { heroe: u64 },
    /// El Líder parte con los que siguen dentro: todo o nada. `creadas` lleva el ejército.
    PartirConvocatoria { heroe: u64 },
    // ── Encuentros, persecución e inspección (Movimiento_Ejercitos §7 y §12, fase 4C). ──
    /// El Líder de la columna sale en persecución de otra columna o de una caravana que **ve** (a ≤ el alcance de vista de su columna). `Objetivo::Asentamiento`
    /// no se persigue.
    Perseguir { heroe: u64, objetivo: Objetivo },
    /// Suelta la presa. La columna sigue su plan hasta donde estaba yendo y acampa.
    DejarDePerseguir { heroe: u64 },
    /// Mira de cerca (a ≤ `radioInspeccion`) una columna, una caravana o una plaza ajena: queda un informe en el héroe y el observado recibe aviso.
    Inspeccionar { heroe: u64, objetivo: Objetivo },
    /// Fija (o quita, con `None`) la relación entre dos Facciones. Solo administración; lo sustituirá el sistema político (5).
    FijarRelacion { a: u32, b: u32, relacion: Option<Relacion> },
    // ── Combate con números (Combate §2-§10, fase 4D). ──
    /// La columna del héroe (sano y a bordo) ataca lo que tiene delante, a ≤ `radioEncuentro`. Toda agresión entre Facciones exige guerra: sin ella la
    /// orden se rechaza, salvo con `declara_guerra`, que la declara (sin arrastre hasta el sistema político). Se resuelve con números: no hay batalla de Unity
    /// hasta la fase 4E. `creadas` va vacío.
    Atacar { heroe: u64, objetivo: ObjetivoAtaque, declara_guerra: bool },
    /// Coloca un campamento de bandidos de ese nivel (1 a 3) en el mapa, donde se pida. Solo administración; para escenarios y pruebas (los de verdad
    /// aparecen solos). `creadas` lleva su id.
    ColocarBandidos { posicion: Punto, nivel: u8 },
    /// Da al héroe una escuadra de `cantidad` soldados de esa tropa (nivel 1, la moral al máximo), sin pagar ni mirar el Liderazgo: en su columna si va a
    /// bordo y, si no, en su campamento. Solo administración; para escenarios y pruebas, igual que `CrearEjercito`. `creadas` lleva su id.
    DarEscuadra { heroe: u64, tropa: Tropa, cantidad: i64 },
    // ── Batallas de Unity (Batalla_Unity_Ciclo, fase 4E). No hay orden para abrirlas: las abre `Atacar` cuando el proceso tiene servidores de batalla
    //    (`Reglas::batallas_en_unity`), sea cual sea quien combata (bot contra bot también). `batalla` es el id de la partida (`Batalla::id`); el contrato lo nombra con
    //    su UUID (`uuid_de_batalla`). ──
    /// Pone a un héroe como humano o como bot. Solo administración; para escenarios y pruebas: los héroes de `CrearEjercito` nacen bot.
    FijarControlador { heroe: u64, controlador: Controlador },
    /// La columna del héroe entra en una batalla activa, en el bando que le toca (`lado` solo es obligatorio en una persecución). No lleva candado de batalla.
    UnirseABatalla { heroe: u64, batalla: u64, lado: Option<Lado> },
    /// Quien la inició se echa atrás antes de que empiece la partida: nadie pierde nada. No lleva candado de batalla.
    CancelarBatalla { heroe: u64, batalla: u64 },
    // Lo que manda el servidor de batalla (§8): no es una orden de jugador sino un mensaje de Conquest con la credencial del servidor, y el actor de la petición
    // es `batalla-servidor:<id>` (`Peticion::de_servidor_de_batalla`). Repetir uno ya aplicado no cambia nada (`SinCambios`).
    RegistrarAsignacion { batalla: u64, asignacion: Asignacion },
    ConfirmarInicio { batalla: u64, intento: String, revision: u32 },
    RegistrarTokens { batalla: u64, intento: String, tokens: Vec<Token> },
    RegistrarSalida { batalla: u64, salida: SalidaBatalla },
    AplicarResultado { batalla: u64, resultado: ResultadoDeBatalla },
    // ── Avanzadas (Doc 5.17, Economia_Construccion §14.1, fase 4F). Las da el héroe (ciudadano de la Facción de la plaza madre) o la administración; la entrega
    //    y el depósito, el propietario de una columna de esa Facción a la puerta de la avanzada. `avanzada` es su id de mundo. ──
    /// Marca la atalaya de una avanzada nueva en `posicion`, con la columna del héroe en el sitio: queda «pendiente de materiales» y ocupa cupo. `creadas` lleva
    /// su id.
    ColocarAvanzada { asentamiento: u64, heroe: u64, posicion: Punto },
    /// La columna entrega a la obra de la avanzada (primera obra, mejora o reparación) los materiales que lleva, con la columna a su puerta.
    EntregarMaterialesAAvanzada { ejercito: u64, avanzada: u64 },
    /// Marca la mejora de una avanzada operativa a la etapa siguiente.
    MejorarAvanzada { heroe: u64, avanzada: u64 },
    /// Marca la reparación de unas ruinas de fuerte o ciudadela (también una ciudadela sin dueño, que pasa a la plaza que tiene ahora su zona).
    RepararAvanzada { heroe: u64, avanzada: u64 },
    /// Destruye la avanzada a mano (o cancela su primera obra) para reubicarla: no recupera nada. La ciudadela no.
    DestruirAvanzada { heroe: u64, avanzada: u64 },
    /// El Gobernador de la madre elige la tropa de la guarnición de serie para el escalón al que pertenece (2, 3 o 4), entre las que tiene la Facción.
    ElegirTropaDeAvanzada { heroe: u64, avanzada: u64, tropa: Tropa },
    /// Deja escuadras del héroe (residente de la madre, con su columna a la puerta) en la guarnición de jugadores de la avanzada, dentro de su cupo.
    GuarnecerAvanzada { heroe: u64, avanzada: u64, escuadras: Vec<u64> },
    /// Recoge de la guarnición de la avanzada escuadras del héroe, con su columna a la puerta.
    RetirarDeAvanzada { heroe: u64, avanzada: u64, escuadras: Vec<u64> },
    /// El Líder de la columna, a la puerta de una avanzada con almacén, llena sus víveres con el trigo del almacén.
    RepostarEnAvanzada { heroe: u64, avanzada: u64 },
    /// La columna, a la puerta de la avanzada, descarga `cantidad` de un recurso de su carro (y de sus caravanas adjuntas) en el almacén, hasta donde quepa.
    DepositarEnAvanzada { ejercito: u64, avanzada: u64, recurso: Recurso, cantidad: Mil },
    /// Cancela la mejora o la reparación de la avanzada, en marcha o pendiente de materiales: sin reembolso (lo entregado se pierde) y la avanzada sigue como estaba. La
    /// primera obra no se cancela así: se destruye la avanzada.
    CancelarObraDeAvanzada { heroe: u64, avanzada: u64 },
    // ── Entrar con todo (2026-10-22). ──
    /// Quien manda en la plaza decide quién puede entrar en ella según su relación: aliados y vasallos, neutrales, enemigos. La misma Facción entra siempre; las
    /// avanzadas y los pueblos de la plaza lo heredan. Hasta el sistema político manda quien manda hoy en la plaza (`manda_en`).
    FijarAcceso { asentamiento: u64, aliados: bool, neutrales: bool, enemigos: bool },
    /// El héroe vacía el carro que trajo a su residencia en su almacén personal, hasta donde quepa; lo que no cabe se queda en el carro.
    VaciarCarro { heroe: u64 },
    // ── Entrar en un pueblo extractor o en una avanzada (2026-10-22). ──
    /// Entra en un pueblo extractor, a su puerta y activo, si el acceso de su plaza madre se lo permite, con todo lo suyo (solo, o el ejército entero desarmándose; solo
    /// su Líder). Dentro defiende con la regla de relación. Fuera del mapa, come de sus víveres.
    EntrarEnPueblo { pueblo: u64, heroe: u64 },
    /// Sale de un pueblo extractor con todo lo que entró: su columna vuelve al mapa tal cual.
    SalirDePueblo { pueblo: u64, heroe: u64 },
    /// Entra en una avanzada operativa, como en un pueblo.
    EntrarEnAvanzada { avanzada: u64, heroe: u64 },
    /// Sale de una avanzada con todo lo que entró.
    SalirDeAvanzada { avanzada: u64, heroe: u64 },
    // ── Murallas (2026-10-22, segunda tanda; Murallas.md §4-§8). Misma autorización provisional que las demás órdenes de plaza (`manda_en`); el cargo que se invoca se da por ocupado. ──
    /// Compromete un recinto de `nivel` (1 empalizada, 2 muro de piedra, 3 muralla con adarve): traza el anillo, lo congela y empieza a levantarlo celda a celda (comprometer es gratis; cada
    /// celda se paga al levantarla). Con recintos ya hechos es una **ampliación** (arrabal de 6 edificios extramuros y el anillo nuevo envuelve al exterior). `creadas` lleva el id del recinto.
    /// El cargo solo puede ser el Gobernador o el Maestro de Obras.
    ComprometerRecinto { asentamiento: u64, cargo: Cargo, nivel: u8 },
    /// Empieza a mejorar un recinto completo al nivel siguiente (el anillo en pie sigue defendiendo con su nivel de antes). Mismo cargo que comprometer.
    MejorarRecinto { asentamiento: u64, cargo: Cargo, recinto: u32 },
    /// Abandona un recinto que nunca se cerró: se borra entero, sin devolución. Murallas lo reserva al Gobernador; hasta el sistema 5 lo autoriza `manda_en`.
    AbandonarRecinto { asentamiento: u64, recinto: u32 },
    // ── La intel de las tabernas (4H T3d; Niebla_De_Guerra §6). La da la cuenta del héroe, que tiene que estar dentro del lugar de la taberna: una plaza de su Facción (paga su almacén) o un
    //    campamento de mercenarios (paga él: su almacén personal si reside allí; si no, el carro de su columna). ──
    /// Compra una Mirada sobre `centro` para la Facción del héroe: ve (no explora) `INTEL.mirada.radio` alrededor durante `duracionMinutos`; `creadas` lleva su id.
    ComprarMirada { heroe: u64, origen: OrigenIntel, centro: Punto },
    /// Compra el Informe de la plaza ajena `asentamiento`, que la Facción tiene que conocer: la foto, fechada, de sus edificios, sus recintos y su defensa, que se guarda en la memoria de la Facción.
    ComprarInformePlaza { heroe: u64, origen: OrigenIntel, asentamiento: u64 },
    // ── Política: la Facción, la ciudadanía y la residencia (sistema 5, tanda 5B; Diplomacia_Facciones §3-§4). Las da la cuenta del héroe que las nombra (el de la cuenta, si la orden
    //    del contrato no lo nombra); responder solicitudes y nombrar Embajador, además, el Rey de la Facción; nombrar Rey, uno de sus ciudadanos (con Rey, solo él). ──
    /// Crea una Facción con el héroe como primer ciudadano y Rey (§3.1); sin sigilo recibe uno libre. Su columna, si va solo, toma la Facción. `creadas` lleva su id.
    CrearFaccion { heroe: u64, nombre: String, sigilo: Option<Sigilo> },
    /// Pide el ingreso en una Facción (§3.2): entra en su lista de solicitantes. Se puede pedir a varias a la vez.
    SolicitarIngreso { heroe: u64, faccion: u32 },
    /// El Rey (`heroe`) acepta o deniega la solicitud de `solicitante` (§3.2). Aceptada, es ciudadano y sus solicitudes en las demás caen.
    ResponderSolicitud { heroe: u64, faccion: u32, solicitante: u64, aceptar: bool },
    /// Deja su Facción, con la sucesión del trono (§3.3). No se deja yendo en un ejército (Q7). Si reside en una plaza, pasa al campamento de mercenarios más cercano.
    DejarFaccion { heroe: u64 },
    /// Nombra Rey a `designado`, ciudadano de la Facción (§3.4): sin Rey, cualquier ciudadano; con Rey, es un traspaso que solo hace él.
    AsignarRey { heroe: u64, faccion: u32, designado: u64 },
    /// El Rey nombra Embajador a `designado`, ciudadano de la Facción (puede ser él mismo).
    AsignarEmbajador { heroe: u64, faccion: u32, designado: u64 },
    /// Se muda a una plaza de su Facción (§4.2): deja la casa anterior y suelta la guarnición; su campamento se traslada con él. Con cooldown.
    CambiarResidencia { heroe: u64, asentamiento: u64 },
    /// Pasa a residir en un campamento de mercenarios (cualquier héroe, de cualquier Facción). Con cooldown.
    ResidirEnCampamento { heroe: u64, campamento: u32 },
    /// Deja la plaza donde reside y pasa al campamento de mercenarios más cercano a ella (5B-4). Con cooldown.
    DejarResidencia { heroe: u64 },
    // ── Política: la capital y el nombre de la plaza (sistema 5, tanda 5C; Diplomacia_Facciones §3.7, Fundacion_Y_Refundacion §8). ──
    /// El Rey (`heroe`) designa capital de su Facción una plaza propia con Palacio activo (§3.7): con cooldown mientras la designada siga en pie. La marca pasa a esa plaza.
    DesignarCapital { heroe: u64, faccion: u32, asentamiento: u64 },
    /// Quien reside en la plaza le cambia el nombre (`renombrarAsentamiento`): se recorta; vacío, la plaza se queda sin nombre (se muestra su id).
    RenombrarAsentamiento { heroe: u64, asentamiento: u64, nombre: String },
}

/// El prefijo del actor de los mensajes del servidor de batalla: `batalla-servidor:<id>` (§8).
pub const ACTOR_SERVIDOR_DE_BATALLA: &str = "batalla-servidor:";

/// Lo que un comando toca de las batallas: con qué hace chocar el candado (`Batalla_Unity_Ciclo` §5, `tocaLoBloqueado`).
#[derive(Default)]
pub(crate) struct Implicados {
    pub heroes: Vec<u64>,
    pub ejercitos: Vec<u64>,
    pub caravanas: Vec<u64>,
    pub asentamientos: Vec<u64>,
    pub pueblos: Vec<u64>,
    pub avanzadas: Vec<u64>,
    /// Campamentos de bandidos.
    pub campamentos: Vec<u64>,
}

/// Quién puede dar cada orden hoy (Proyecciones_Y_Autorizacion §5): la autorización provisional hasta el sistema político, que traerá los cargos, la residencia y los vetos. La
/// administración (`Peticion.admin`) puede con todas salvo las del canal del servidor de batalla. Cada orden comprueba la suya en su sitio, con el código `no_autorizado`
/// (la de administración, una sola vez en `Nucleo::ejecutar`); lo demás que exige (estar a la puerta, ser el Líder, el dinero...) son rechazos de dominio con su propio código.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Autoriza {
    /// Solo la administración: el andamiaje que sustituirán los sistemas 5 y 6, y el reloj.
    Administracion,
    /// Solo un servidor de batalla, por su credencial (`batalla-servidor:<id>`, `batalla.sin_credencial`): el canal de Unity.
    ServidorDeBatalla,
    /// La propia cuenta: crea su héroe, no el de otra ni un bot.
    SuCuenta,
    /// La cuenta del héroe que la nombra (o el de la cuenta, si la orden del contrato no lo nombra), con el héroe en el mundo (`heroe.fuera_del_mundo`).
    CuentaDelHeroe,
    /// La cuenta del héroe, aunque esté fuera del mundo.
    CuentaDelHeroeAunqueFuera,
    /// La cuenta del héroe, que además es ciudadano de la Facción de la plaza madre de la avanzada.
    CiudadanoDeLaMadre,
    /// La cuenta del héroe, que además es el Rey de la Facción que nombra la orden (`responderSolicitud` sin excepción: tampoco si la Facción no existe).
    ReyDeLaFaccion,
    /// La cuenta del héroe, que además es ciudadano de la Facción que nombra la orden y, si tiene Rey, su Rey (`asignarRey`).
    CiudadanoDeLaFaccion,
    /// La cuenta del héroe, que además reside en la plaza que nombra la orden (`renombrarAsentamiento`: basta residir).
    ResidenteDeLaPlaza,
    /// Quien manda en la plaza (`Nucleo::manda_en`): una cuenta con un héroe ciudadano de su Facción o una columna de su Facción. De una caravana, la plaza de su origen; de un trueque,
    /// la plaza A al proponerlo y la B al aceptarlo o rechazarlo; de una orden de mercado, su plaza.
    MandaEnLaPlaza,
    /// La cuenta del Líder de la columna.
    LiderDeLaColumna,
    /// La cuenta de cualquier participante de la columna.
    ParticipanteDeLaColumna,
}

impl Comando {
    /// Quién puede dar la orden ([`Autoriza`]). Cada variante se nombra a propósito (sin `_`): una orden nueva no compila hasta decidirlo.
    pub fn autoriza(&self) -> Autoriza {
        use Autoriza::*;
        match self {
            Comando::CrearEjercito { .. }
            | Comando::Reloj(_)
            | Comando::FijarNivelFaccion { .. }
            | Comando::DarCaravanaFundacion { .. }
            | Comando::FijarNivelAsentamiento { .. }
            | Comando::AgrandarZona { .. }
            | Comando::ReclamarParcela { .. }
            | Comando::SoltarParcela { .. }
            | Comando::DarRecursos { .. }
            | Comando::DarSuministro { .. }
            | Comando::FijarResidencia { .. }
            | Comando::DarAlmacenPersonal { .. }
            | Comando::FijarTecnologias { .. }
            | Comando::FijarFaccionDeHeroe { .. }
            | Comando::ColocarHeroe { .. }
            | Comando::HerirHeroe { .. }
            | Comando::FijarRelacion { .. }
            | Comando::ColocarBandidos { .. }
            | Comando::DarEscuadra { .. }
            | Comando::FijarControlador { .. } => Administracion,
            Comando::RegistrarAsignacion { .. } | Comando::ConfirmarInicio { .. } | Comando::RegistrarTokens { .. } | Comando::RegistrarSalida { .. } | Comando::AplicarResultado { .. } => ServidorDeBatalla,
            Comando::CrearHeroe { .. } => SuCuenta,
            Comando::Conectarse { .. } | Comando::CancelarConvocatoria { .. } => CuentaDelHeroeAunqueFuera,
            Comando::ReclutarTropa { .. }
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
            | Comando::AsignarEscolta { .. }
            | Comando::QuitarEscolta { .. }
            | Comando::Desconectarse { .. }
            | Comando::ComprarEnCampamento { .. }
            | Comando::RepostarEnCampamento { .. }
            | Comando::SalirAlMundo { .. }
            | Comando::MovilizarEjercito { .. }
            | Comando::SalirDelCampamento { .. }
            | Comando::EntrarEnAsentamiento { .. }
            | Comando::SalirDeAsentamiento { .. }
            | Comando::Guarnecer { .. }
            | Comando::EntrarEnCampamento { .. }
            | Comando::VaciarCarro { .. }
            | Comando::EntrarEnPueblo { .. }
            | Comando::SalirDePueblo { .. }
            | Comando::EntrarEnAvanzada { .. }
            | Comando::SalirDeAvanzada { .. }
            | Comando::UnirseAEjercito { .. }
            | Comando::UnirseEnCampo { .. }
            | Comando::ResponderPeticionDeUnion { .. }
            | Comando::SepararseDelEjercito { .. }
            | Comando::CederLiderazgo { .. }
            | Comando::OrganizarEjercito { .. }
            | Comando::CancelarFormacion { .. }
            | Comando::Repostar { .. }
            | Comando::RepostarEnAvanzada { .. }
            | Comando::ConvocarEjercito { .. }
            | Comando::UnirseAConvocatoria { .. }
            | Comando::ResponderPeticionDeConvocatoria { .. }
            | Comando::CambiarSeleccionDeConvocatoria { .. }
            | Comando::SepararseDeConvocatoria { .. }
            | Comando::PartirConvocatoria { .. }
            | Comando::Perseguir { .. }
            | Comando::DejarDePerseguir { .. }
            | Comando::Inspeccionar { .. }
            | Comando::Atacar { .. }
            | Comando::UnirseABatalla { .. }
            | Comando::CancelarBatalla { .. }
            | Comando::ComprarMirada { .. }
            | Comando::ComprarInformePlaza { .. }
            | Comando::CrearFaccion { .. }
            | Comando::SolicitarIngreso { .. }
            | Comando::DejarFaccion { .. }
            | Comando::CambiarResidencia { .. }
            | Comando::ResidirEnCampamento { .. }
            | Comando::DejarResidencia { .. } => CuentaDelHeroe,
            Comando::ResponderSolicitud { .. } | Comando::AsignarEmbajador { .. } | Comando::DesignarCapital { .. } => ReyDeLaFaccion,
            Comando::RenombrarAsentamiento { .. } => ResidenteDeLaPlaza,
            Comando::AsignarRey { .. } => CiudadanoDeLaFaccion,
            Comando::ColocarAvanzada { .. }
            | Comando::MejorarAvanzada { .. }
            | Comando::RepararAvanzada { .. }
            | Comando::DestruirAvanzada { .. }
            | Comando::CancelarObraDeAvanzada { .. }
            | Comando::ElegirTropaDeAvanzada { .. }
            | Comando::GuarnecerAvanzada { .. }
            | Comando::RetirarDeAvanzada { .. } => CiudadanoDeLaMadre,
            Comando::AnadirEdificio { .. }
            | Comando::QuitarDeCola { .. }
            | Comando::MoverEnCola { .. }
            | Comando::MejorarEdificio { .. }
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
            | Comando::CrearCaravana { .. }
            | Comando::AnadirCarro { .. }
            | Comando::AnadirAnimal { .. }
            | Comando::MoverCarro { .. }
            | Comando::ReservarCaravana { .. }
            | Comando::PrepararCaravana { .. }
            | Comando::CancelarCaravana { .. }
            | Comando::MoverCargaAparcada { .. }
            | Comando::EnviarCaravanaAlOrigen { .. }
            | Comando::MarcarPueblo { .. }
            | Comando::DestruirPueblo { .. }
            | Comando::ReponerCarro { .. }
            | Comando::FijarAcceso { .. }
            | Comando::ComprometerRecinto { .. }
            | Comando::MejorarRecinto { .. }
            | Comando::AbandonarRecinto { .. } => MandaEnLaPlaza,
            Comando::MarcharA { .. } | Comando::Estacionar { .. } | Comando::Fundar { .. } | Comando::ComerciarEnPlaza { .. } => LiderDeLaColumna,
            Comando::EntregarDeCaravana { .. }
            | Comando::AdjuntarCaravana { .. }
            | Comando::AparcarCaravana { .. }
            | Comando::EntregarMateriales { .. }
            | Comando::EntregarMaterialesAAvanzada { .. }
            | Comando::DepositarEnAvanzada { .. } => ParticipanteDeLaColumna,
        }
    }

    /// Qué entidad crea la orden, si crea alguna: es la clase de los ids de `Respuesta::creadas`.
    pub(crate) fn tipo_creado(&self) -> Option<TipoCreado> {
        use TipoCreado::*;
        Some(match self {
            Comando::CrearEjercito { .. } | Comando::SalirAlMundo { .. } | Comando::MovilizarEjercito { .. } | Comando::SalirDelCampamento { .. } | Comando::SepararseDelEjercito { .. } | Comando::PartirConvocatoria { .. } | Comando::Conectarse { .. } => Ejercito,
            Comando::Fundar { .. } => Asentamiento,
            Comando::ReclamarParcela { .. } => Parcela,
            Comando::AnadirEdificio { asentamiento, .. } => Edificio { plaza: *asentamiento },
            Comando::ComprometerRecinto { asentamiento, .. } => Recinto { plaza: *asentamiento },
            Comando::ProponerTrueque { .. } => Acuerdo,
            Comando::ColocarOrdenMercado { .. } => OrdenMercado,
            Comando::CrearCaravana { .. } => Caravana,
            Comando::MarcarPueblo { .. } => Pueblo,
            Comando::ColocarAvanzada { .. } => Avanzada,
            Comando::CrearHeroe { .. } => Heroe,
            Comando::ReclutarTropa { .. } | Comando::ReclutarEnCampamento { .. } | Comando::PedirPrestamo { .. } | Comando::DarEscuadra { .. } => Escuadra,
            Comando::GuardarLoadout { .. } => Loadout,
            Comando::UnirseABatalla { batalla, .. } => Incorporacion { batalla: *batalla },
            Comando::ConvocarEjercito { .. } => Convocatoria,
            Comando::ColocarBandidos { .. } => CampamentoBandidos,
            Comando::ComprarMirada { .. } => Mirada,
            Comando::CrearFaccion { .. } => Faccion,
            _ => return None,
        })
    }

    /// ¿Lleva el candado de batalla? Todos los comandos salvo los que actúan sobre la propia batalla y conectarse y desconectarse (cerrar el cliente en plena
    /// batalla tiene que anotarse, y volver a conectarse anular esa petición, §5.1), y los mensajes del servidor de batalla, que no pasan por el registro de
    /// comandos de jugador.
    pub(crate) fn lleva_candado(&self) -> bool {
        !matches!(
            self,
            Comando::UnirseABatalla { .. }
                | Comando::CancelarBatalla { .. }
                | Comando::Conectarse { .. }
                | Comando::Desconectarse { .. }
                | Comando::RegistrarAsignacion { .. }
                | Comando::ConfirmarInicio { .. }
                | Comando::RegistrarTokens { .. }
                | Comando::RegistrarSalida { .. }
                | Comando::AplicarResultado { .. }
        )
    }

    /// ¿Es una orden de salir de un lugar al mapa? Es lo único que puede dar, durante un asedio, un saqueo o un ataque, quien está dentro y no se ha unido a la defensa
    /// (Entrar con todo, cuarta ronda): `SalirDeAsentamiento`, `SalirDePueblo`, `SalirDeAvanzada` y, para el residente de su propia plaza, que no sale con `SalirDeAsentamiento` sino
    /// eligiendo tropa y carga, `SalirAlMundo`. El candado no mira entonces el lugar ni que se esté dentro; sí que se combata.
    // `SalirAlMundo` del residente cuenta como salir del lugar (decisión del autor, quinta ronda, 2026-10-22): es su única salida. Lo que reservó la batalla (la guarnición con dueño) sigue
    // sin poder llevarse.
    pub(crate) fn es_salida_de_lugar(&self) -> bool {
        matches!(self, Comando::SalirDeAsentamiento { .. } | Comando::SalirDePueblo { .. } | Comando::SalirDeAvanzada { .. } | Comando::SalirAlMundo { .. })
    }

    /// Los héroes, columnas, caravanas, plazas, pueblos, avanzadas y campamentos de bandidos que nombra el comando. Cada variante se nombra a propósito (sin `_`): si se
    /// añade una orden, esto deja de compilar y obliga a decidir qué toca. `Perseguir` e `Inspeccionar` apuntan a una columna o a una caravana; las plazas
    /// que se inspeccionan o se atacan no se cuentan (la batalla de una plaza la rechaza `atacar`, y marchar hacia una asediada se puede, §5).
    pub(crate) fn implicados(&self) -> Implicados {
        let mut i = Implicados::default();
        match self {
            Comando::Perseguir { heroe, objetivo } | Comando::Inspeccionar { heroe, objetivo } => {
                i.heroes.push(*heroe);
                match *objetivo {
                    Objetivo::Ejercito(x) => i.ejercitos.push(x),
                    Objetivo::Caravana(x) => i.caravanas.push(x),
                    Objetivo::Asentamiento(_) => {}
                }
            }
            Comando::Atacar { heroe, objetivo, declara_guerra: _ } => {
                i.heroes.push(*heroe);
                match *objetivo {
                    ObjetivoAtaque::Ejercito(x) => i.ejercitos.push(x),
                    ObjetivoAtaque::Caravana(x) => i.caravanas.push(x),
                    ObjetivoAtaque::Campamento(x) => i.campamentos.push(x),
                    ObjetivoAtaque::Pueblo(x) => i.pueblos.push(x),
                    ObjetivoAtaque::Avanzada(x) => i.avanzadas.push(x),
                    ObjetivoAtaque::Asentamiento(_) => {}
                }
            }
            Comando::MarcharA { ejercito, .. } | Comando::Estacionar { ejercito } | Comando::DarCaravanaFundacion { ejercito } | Comando::Fundar { ejercito } | Comando::DarSuministro { ejercito, .. } => i.ejercitos.push(*ejercito),
            Comando::ComerciarEnPlaza { ejercito, .. } => i.ejercitos.push(*ejercito),
            Comando::EntregarDeCaravana { ejercito, caravana, .. } | Comando::AdjuntarCaravana { caravana, ejercito } => {
                i.ejercitos.push(*ejercito);
                i.caravanas.push(*caravana);
            }
            Comando::EntregarMateriales { ejercito, pueblo } => {
                i.ejercitos.push(*ejercito);
                i.pueblos.push(*pueblo);
            }
            Comando::DestruirPueblo { pueblo } | Comando::ReponerCarro { pueblo } => i.pueblos.push(*pueblo),
            Comando::EntrarEnPueblo { pueblo, heroe } | Comando::SalirDePueblo { pueblo, heroe } => {
                i.heroes.push(*heroe);
                i.pueblos.push(*pueblo);
            }
            Comando::EntregarMaterialesAAvanzada { ejercito, avanzada } | Comando::DepositarEnAvanzada { ejercito, avanzada, .. } => {
                i.ejercitos.push(*ejercito);
                i.avanzadas.push(*avanzada);
            }
            Comando::ColocarAvanzada { asentamiento, heroe, .. } => {
                i.heroes.push(*heroe);
                i.asentamientos.push(*asentamiento);
            }
            Comando::MejorarAvanzada { heroe, avanzada }
            | Comando::RepararAvanzada { heroe, avanzada }
            | Comando::DestruirAvanzada { heroe, avanzada }
            | Comando::CancelarObraDeAvanzada { heroe, avanzada }
            | Comando::ElegirTropaDeAvanzada { heroe, avanzada, .. }
            | Comando::GuarnecerAvanzada { heroe, avanzada, .. }
            | Comando::RetirarDeAvanzada { heroe, avanzada, .. }
            | Comando::RepostarEnAvanzada { heroe, avanzada }
            | Comando::EntrarEnAvanzada { avanzada, heroe }
            | Comando::SalirDeAvanzada { avanzada, heroe } => {
                i.heroes.push(*heroe);
                i.avanzadas.push(*avanzada);
            }
            Comando::AparcarCaravana { caravana, asentamiento } => {
                i.caravanas.push(*caravana);
                i.asentamientos.push(*asentamiento);
            }
            Comando::ComprarMirada { heroe, origen, .. } | Comando::ComprarInformePlaza { heroe, origen, .. } => {
                i.heroes.push(*heroe);
                if let OrigenIntel::Plaza(a) = origen {
                    i.asentamientos.push(*a);
                }
            }
            // Aceptar cambia la Facción del solicitante, y con ella la de su columna: no mientras combate.
            Comando::ResponderSolicitud { heroe, solicitante, .. } => i.heroes.extend([*heroe, *solicitante]),
            Comando::CambiarResidencia { heroe, asentamiento } | Comando::DesignarCapital { heroe, asentamiento, .. } | Comando::RenombrarAsentamiento { heroe, asentamiento, .. } => {
                i.heroes.push(*heroe);
                i.asentamientos.push(*asentamiento);
            }
            Comando::AnadirCarro { caravana, .. }
            | Comando::AnadirAnimal { caravana, .. }
            | Comando::ReservarCaravana { caravana, .. }
            | Comando::CancelarCaravana { caravana }
            | Comando::MoverCargaAparcada { caravana, .. }
            | Comando::EnviarCaravanaAlOrigen { caravana } => i.caravanas.push(*caravana),
            Comando::PrepararCaravana { caravana, destino, .. } => {
                i.caravanas.push(*caravana);
                i.asentamientos.push(*destino);
            }
            Comando::MoverCarro { de, a, .. } => i.caravanas.extend([*de, *a]),
            Comando::ProponerTrueque { a, b, .. } => i.asentamientos.extend([*a, *b]),
            Comando::FijarNivelAsentamiento { asentamiento, .. }
            | Comando::AgrandarZona { asentamiento }
            | Comando::ReclamarParcela { asentamiento }
            | Comando::SoltarParcela { asentamiento, .. }
            | Comando::AnadirEdificio { asentamiento, .. }
            | Comando::QuitarDeCola { asentamiento, .. }
            | Comando::MoverEnCola { asentamiento, .. }
            | Comando::MejorarEdificio { asentamiento, .. }
            | Comando::AlternarAutoConstruccion { asentamiento, .. }
            | Comando::AlternarReceta { asentamiento, .. }
            | Comando::CalibrarReserva { asentamiento, .. }
            | Comando::SolicitarAscenso { asentamiento }
            | Comando::FijarAcceso { asentamiento, .. }
            | Comando::ActivarPolitica { asentamiento, .. }
            | Comando::ComprometerRecinto { asentamiento, .. }
            | Comando::MejorarRecinto { asentamiento, .. }
            | Comando::AbandonarRecinto { asentamiento, .. }
            | Comando::ColocarOrdenMercado { asentamiento, .. }
            | Comando::CrearCaravana { asentamiento }
            | Comando::DarRecursos { asentamiento, .. }
            | Comando::MarcarPueblo { asentamiento, .. } => i.asentamientos.push(*asentamiento),
            Comando::ReclutarTropa { asentamiento, heroe, .. }
            | Comando::EntrarEnAsentamiento { asentamiento, heroe }
            | Comando::SalirDeAsentamiento { asentamiento, heroe }
            | Comando::Guarnecer { asentamiento, heroe }
            | Comando::Repostar { heroe, asentamiento }
            | Comando::SalirAlMundo { asentamiento, heroe, .. }
            | Comando::MovilizarEjercito { asentamiento, heroe, .. } => {
                i.heroes.push(*heroe);
                i.asentamientos.push(*asentamiento);
            }
            Comando::UnirseAEjercito { asentamiento, heroe, ejercito, .. } => {
                i.heroes.push(*heroe);
                i.asentamientos.push(*asentamiento);
                i.ejercitos.push(*ejercito);
            }
            Comando::UnirseEnCampo { heroe, ejercito } | Comando::ResponderPeticionDeUnion { heroe, ejercito, .. } => {
                i.heroes.push(*heroe);
                i.ejercitos.push(*ejercito);
            }
            Comando::AsignarEscolta { caravana, heroe, .. } | Comando::QuitarEscolta { caravana, heroe, .. } => {
                i.heroes.push(*heroe);
                i.caravanas.push(*caravana);
            }
            Comando::ComprarEnCampamento { heroe, .. }
            | Comando::RepostarEnCampamento { heroe, .. }
            | Comando::FijarResidencia { heroe, .. }
            | Comando::DarAlmacenPersonal { heroe, .. }
            | Comando::ReclutarEnCampamento { heroe, .. }
            | Comando::PedirPrestamo { heroe, .. }
            | Comando::ReponerPrestamo { heroe }
            | Comando::GuardarLoadout { heroe, .. }
            | Comando::BorrarLoadout { heroe, .. }
            | Comando::RepartirPuntos { heroe, .. }
            | Comando::OrdenarEscuadras { heroe, .. }
            | Comando::AsignarGuarnicion { heroe, .. }
            | Comando::RetirarGuarnicion { heroe, .. }
            | Comando::PasarAViveres { heroe, .. }
            | Comando::Desconectarse { heroe }
            | Comando::Conectarse { heroe }
            | Comando::VaciarCarro { heroe }
            | Comando::FijarFaccionDeHeroe { heroe, .. }
            | Comando::ColocarHeroe { heroe, .. }
            | Comando::HerirHeroe { heroe }
            | Comando::SalirDelCampamento { heroe, .. }
            | Comando::EntrarEnCampamento { heroe, .. }
            | Comando::SepararseDelEjercito { heroe }
            | Comando::CederLiderazgo { heroe, .. }
            | Comando::OrganizarEjercito { heroe, .. }
            | Comando::CancelarFormacion { heroe }
            | Comando::ConvocarEjercito { heroe, .. }
            | Comando::UnirseAConvocatoria { heroe, .. }
            | Comando::ResponderPeticionDeConvocatoria { heroe, .. }
            | Comando::CambiarSeleccionDeConvocatoria { heroe, .. }
            | Comando::SepararseDeConvocatoria { heroe }
            | Comando::CancelarConvocatoria { heroe }
            | Comando::PartirConvocatoria { heroe }
            | Comando::DejarDePerseguir { heroe }
            | Comando::DarEscuadra { heroe, .. }
            | Comando::FijarControlador { heroe, .. }
            | Comando::UnirseABatalla { heroe, .. }
            | Comando::CancelarBatalla { heroe, .. }
            | Comando::CrearFaccion { heroe, .. }
            | Comando::SolicitarIngreso { heroe, .. }
            | Comando::DejarFaccion { heroe }
            | Comando::AsignarRey { heroe, .. }
            | Comando::AsignarEmbajador { heroe, .. }
            | Comando::ResidirEnCampamento { heroe, .. }
            | Comando::DejarResidencia { heroe } => i.heroes.push(*heroe),
            Comando::CrearEjercito { .. }
            | Comando::Reloj(_)
            | Comando::FijarNivelFaccion { .. }
            | Comando::AceptarTrueque { .. }
            | Comando::RechazarTrueque { .. }
            | Comando::CancelarOrdenMercado { .. }
            | Comando::CrearHeroe { .. }
            | Comando::FijarTecnologias { .. }
            | Comando::FijarRelacion { .. }
            | Comando::ColocarBandidos { .. }
            | Comando::RegistrarAsignacion { .. }
            | Comando::ConfirmarInicio { .. }
            | Comando::RegistrarTokens { .. }
            | Comando::RegistrarSalida { .. }
            | Comando::AplicarResultado { .. } => {}
        }
        i
    }
}

impl Comando {
    /// Las entidades que la orden nombra y su ejecución puede cambiar, para el registro de cambios: las de [`Comando::implicados`] (héroes, caravanas, pueblos y avanzadas;
    /// las columnas y las plazas las anota cada regla) y las que el candado de batalla no mira: trueques, órdenes de mercado y campamentos de mercenarios.
    pub(crate) fn nombradas(&self) -> Vec<(Clase, u64)> {
        let i = self.implicados();
        let mut v: Vec<(Clase, u64)> = [(Clase::Heroe, i.heroes), (Clase::Caravana, i.caravanas), (Clase::Pueblo, i.pueblos), (Clase::Avanzada, i.avanzadas)]
            .into_iter()
            .flat_map(|(clase, ids)| ids.into_iter().map(move |id| (clase, id)))
            .collect();
        match self {
            Comando::AceptarTrueque { acuerdo } | Comando::RechazarTrueque { acuerdo } | Comando::EntregarDeCaravana { acuerdo, .. } => v.push((Clase::Acuerdo, *acuerdo)),
            Comando::CancelarOrdenMercado { orden } | Comando::ComerciarEnPlaza { orden, .. } => v.push((Clase::OrdenMercado, *orden)),
            Comando::ComprarEnCampamento { campamento, .. }
            | Comando::RepostarEnCampamento { campamento, .. }
            | Comando::EntrarEnCampamento { campamento, .. }
            | Comando::SalirDelCampamento { campamento, .. }
            | Comando::CrearHeroe { campamento, .. }
            | Comando::FijarResidencia { residencia: Some(Residencia::Campamento(campamento)), .. } => v.push((Clase::Campamento, u64::from(*campamento))),
            Comando::FijarNivelFaccion { faccion, .. } | Comando::FijarTecnologias { faccion, .. } | Comando::CrearEjercito { faccion, .. } => v.push((Clase::Faccion, u64::from(*faccion))),
            Comando::FijarRelacion { a, b, .. } => v.extend([(Clase::Faccion, u64::from(*a)), (Clase::Faccion, u64::from(*b)), (Clase::Relacion, Clase::par(*a, *b))]),
            Comando::FijarFaccionDeHeroe { faccion: Some(faccion), .. }
            | Comando::SolicitarIngreso { faccion, .. }
            | Comando::ResponderSolicitud { faccion, .. }
            | Comando::AsignarRey { faccion, .. }
            | Comando::AsignarEmbajador { faccion, .. }
            | Comando::DesignarCapital { faccion, .. } => v.push((Clase::Faccion, u64::from(*faccion))),
            Comando::ResidirEnCampamento { campamento, .. } => v.push((Clase::Campamento, u64::from(*campamento))),
            _ => {}
        }
        v
    }
}

/// De dónde sale el oro al reclutar en un campamento de mercenarios.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PagarCon {
    AlmacenPersonal,
    /// Del carro de la columna del héroe, a la puerta del campamento.
    Carro,
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

/// Una lista de `(recurso, cantidad)`: `n u64 · n × (recurso u8, cantidad i64)`.
fn lineas(w: &mut Escritor, v: &[(Recurso, Mil)]) {
    w.u64(v.len() as u64);
    for &(r, n) in v {
        w.u8(recurso_a_u8(r));
        w.i64(n);
    }
}

fn leer_lineas(r: &mut Lector) -> Result<Vec<(Recurso, Mil)>, ErrorFormato> {
    let mut v = Vec::new();
    for _ in 0..r.cuenta()? {
        v.push((recurso_de_u8(r.u8()?)?, r.i64()?));
    }
    Ok(v)
}

/// La taberna de una compra de intel: `0 · plaza u64` o `1 · campamento u32`.
fn origen_escribir(w: &mut Escritor, o: OrigenIntel) {
    match o {
        OrigenIntel::Plaza(a) => {
            w.u8(0);
            w.u64(a);
        }
        OrigenIntel::Campamento(c) => {
            w.u8(1);
            w.u32(c);
        }
    }
}

fn origen_leer(r: &mut Lector) -> Result<OrigenIntel, ErrorFormato> {
    Ok(match r.u8()? {
        0 => OrigenIntel::Plaza(r.u64()?),
        1 => OrigenIntel::Campamento(r.u32()?),
        _ => return Err(ErrorFormato("taberna de una compra de intel desconocida")),
    })
}

fn lista_u64(w: &mut Escritor, v: &[u64]) {
    w.u64(v.len() as u64);
    v.iter().for_each(|&x| w.u64(x));
}

fn leer_lista_u64(r: &mut Lector) -> Result<Vec<u64>, ErrorFormato> {
    (0..r.cuenta()?).map(|_| r.u64()).collect()
}

impl Peticion {
    pub fn nueva(actor: &str, peticion_id: &str, comando: Comando) -> Self {
        Self { actor: actor.into(), admin: false, peticion_id: peticion_id.into(), version_esperada: None, comando }
    }
    pub fn de_admin(mut self) -> Self {
        self.admin = true;
        self
    }
    /// Un mensaje del servidor de batalla `servidor` (su id en `SERVIDORES_BATALLA`): el actor es `batalla-servidor:<id>`, nunca la cuenta de un jugador.
    pub fn de_servidor_de_batalla(servidor: &str, peticion_id: &str, comando: Comando) -> Self {
        Self::nueva(&format!("{ACTOR_SERVIDOR_DE_BATALLA}{servidor}"), peticion_id, comando)
    }

    pub(crate) fn codificar(&self) -> Vec<u8> {
        let mut w = Escritor::default();
        w.texto(&self.actor);
        w.bool(self.admin);
        w.texto(&self.peticion_id);
        w.opt_u64(self.version_esperada);
        match &self.comando {
            Comando::CrearEjercito { cuenta, faccion, posicion, velocidad, viveres } => {
                w.u8(1);
                w.texto(cuenta);
                w.u32(*faccion);
                w.punto(*posicion);
                w.i64(*velocidad);
                w.i64(*viveres);
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
            Comando::ProponerTrueque { a, b, lineas_a, lineas_b } => {
                w.u8(21);
                w.u64(*a);
                w.u64(*b);
                lineas(&mut w, lineas_a);
                lineas(&mut w, lineas_b);
            }
            Comando::AceptarTrueque { acuerdo } => {
                w.u8(22);
                w.u64(*acuerdo);
            }
            Comando::RechazarTrueque { acuerdo } => {
                w.u8(23);
                w.u64(*acuerdo);
            }
            Comando::ColocarOrdenMercado { asentamiento, venta, recurso, cantidad, precio } => {
                w.u8(24);
                w.u64(*asentamiento);
                w.bool(*venta);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
                w.opt_i64(*precio);
            }
            Comando::CancelarOrdenMercado { orden } => {
                w.u8(25);
                w.u64(*orden);
            }
            Comando::ComerciarEnPlaza { ejercito, orden, cantidad } => {
                w.u8(26);
                w.u64(*ejercito);
                w.u64(*orden);
                w.i64(*cantidad);
            }
            Comando::CrearCaravana { asentamiento } => {
                w.u8(27);
                w.u64(*asentamiento);
            }
            Comando::AnadirCarro { caravana, tipo } => {
                w.u8(28);
                w.u64(*caravana);
                w.u8(tipo_carro_a_u8(*tipo));
            }
            Comando::AnadirAnimal { caravana, carro, animal } => {
                w.u8(29);
                w.u64(*caravana);
                w.u32(*carro);
                w.u8(animal_a_u8(*animal));
            }
            Comando::MoverCarro { de, a, carro } => {
                w.u8(30);
                w.u64(*de);
                w.u64(*a);
                w.u32(*carro);
            }
            Comando::ReservarCaravana { caravana, reservada } => {
                w.u8(31);
                w.u64(*caravana);
                w.bool(*reservada);
            }
            Comando::PrepararCaravana { caravana, destino, carga, salir_en } => {
                w.u8(32);
                w.u64(*caravana);
                w.u64(*destino);
                lineas(&mut w, carga);
                w.opt_i64(*salir_en);
            }
            Comando::CancelarCaravana { caravana } => {
                w.u8(33);
                w.u64(*caravana);
            }
            Comando::MoverCargaAparcada { caravana, recurso, cantidad, cargar } => {
                w.u8(34);
                w.u64(*caravana);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
                w.bool(*cargar);
            }
            Comando::EnviarCaravanaAlOrigen { caravana } => {
                w.u8(35);
                w.u64(*caravana);
            }
            Comando::EntregarDeCaravana { ejercito, caravana, acuerdo } => {
                w.u8(36);
                w.u64(*ejercito);
                w.u64(*caravana);
                w.u64(*acuerdo);
            }
            Comando::DarRecursos { asentamiento, recurso, cantidad } => {
                w.u8(37);
                w.u64(*asentamiento);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
            }
            Comando::DarSuministro { ejercito, recurso, cantidad } => {
                w.u8(38);
                w.u64(*ejercito);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
            }
            Comando::AdjuntarCaravana { caravana, ejercito } => {
                w.u8(39);
                w.u64(*caravana);
                w.u64(*ejercito);
            }
            Comando::AparcarCaravana { caravana, asentamiento } => {
                w.u8(40);
                w.u64(*caravana);
                w.u64(*asentamiento);
            }
            Comando::ComprarEnCampamento { heroe, campamento, recurso, cantidad } => {
                w.u8(41);
                w.u64(*heroe);
                w.u32(*campamento);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
            }
            Comando::RepostarEnCampamento { heroe, campamento } => {
                w.u8(42);
                w.u64(*heroe);
                w.u32(*campamento);
            }
            Comando::FijarResidencia { heroe, residencia } => {
                w.u8(43);
                w.u64(*heroe);
                residencia_escribir(&mut w, residencia);
            }
            Comando::DarAlmacenPersonal { heroe, recurso, cantidad } => {
                w.u8(44);
                w.u64(*heroe);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
            }
            Comando::MarcarPueblo { asentamiento, nodo } => {
                w.u8(45);
                w.u64(*asentamiento);
                w.u32(*nodo);
            }
            Comando::EntregarMateriales { ejercito, pueblo } => {
                w.u8(46);
                w.u64(*ejercito);
                w.u64(*pueblo);
            }
            Comando::DestruirPueblo { pueblo } => {
                w.u8(47);
                w.u64(*pueblo);
            }
            Comando::ReponerCarro { pueblo } => {
                w.u8(48);
                w.u64(*pueblo);
            }
            Comando::CrearHeroe { cuenta, campamento, controlador } => {
                w.u8(49);
                w.texto(cuenta);
                w.u32(*campamento);
                w.u8(controlador_a_u8(*controlador));
            }
            Comando::ReclutarTropa { asentamiento, heroe, tropa } => {
                w.u8(50);
                w.u64(*asentamiento);
                w.u64(*heroe);
                w.u8(tropa_a_u8(*tropa));
            }
            Comando::ReclutarEnCampamento { heroe, tropa, pagar_con } => {
                w.u8(51);
                w.u64(*heroe);
                w.u8(tropa_a_u8(*tropa));
                w.u8(*pagar_con as u8);
            }
            Comando::PedirPrestamo { heroe, tropas } => {
                w.u8(52);
                w.u64(*heroe);
                w.u64(tropas.len() as u64);
                tropas.iter().for_each(|&t| w.u8(tropa_a_u8(t)));
            }
            Comando::ReponerPrestamo { heroe } => {
                w.u8(53);
                w.u64(*heroe);
            }
            Comando::GuardarLoadout { heroe, loadout, nombre, escuadras, perks, activo } => {
                w.u8(54);
                w.u64(*heroe);
                w.opt_u64(*loadout);
                w.texto(nombre);
                lista_u64(&mut w, escuadras);
                w.u64(perks.len() as u64);
                perks.iter().for_each(|&k| w.u32(k));
                w.u8(activo.map_or(2, u8::from));
            }
            Comando::BorrarLoadout { heroe, loadout } => {
                w.u8(55);
                w.u64(*heroe);
                w.u64(*loadout);
            }
            Comando::RepartirPuntos { heroe, fuerza, destreza, armadura, vitalidad } => {
                w.u8(56);
                w.u64(*heroe);
                [fuerza, destreza, armadura, vitalidad].into_iter().for_each(|&p| w.i64(p));
            }
            Comando::OrdenarEscuadras { heroe, escuadras } => {
                w.u8(57);
                w.u64(*heroe);
                lista_u64(&mut w, escuadras);
            }
            Comando::AsignarGuarnicion { heroe, escuadra } => {
                w.u8(58);
                w.u64(*heroe);
                w.u64(*escuadra);
            }
            Comando::RetirarGuarnicion { heroe, escuadra } => {
                w.u8(59);
                w.u64(*heroe);
                w.u64(*escuadra);
            }
            Comando::PasarAViveres { heroe, cantidad } => {
                w.u8(60);
                w.u64(*heroe);
                w.i64(*cantidad);
            }
            Comando::AsignarEscolta { caravana, heroe, escuadras } => {
                w.u8(61);
                w.u64(*caravana);
                w.u64(*heroe);
                lista_u64(&mut w, escuadras);
            }
            Comando::QuitarEscolta { caravana, heroe, escuadras } => {
                w.u8(62);
                w.u64(*caravana);
                w.u64(*heroe);
                w.bool(escuadras.is_some());
                lista_u64(&mut w, escuadras.as_deref().unwrap_or_default());
            }
            Comando::Desconectarse { heroe } => {
                w.u8(63);
                w.u64(*heroe);
            }
            Comando::Conectarse { heroe } => {
                w.u8(64);
                w.u64(*heroe);
            }
            Comando::FijarTecnologias { faccion, tecnologias } => {
                w.u8(65);
                w.u32(*faccion);
                w.u64(tecnologias.len() as u64);
                tecnologias.iter().for_each(|t| w.texto(t));
            }
            Comando::FijarFaccionDeHeroe { heroe, faccion } => {
                w.u8(66);
                w.u64(*heroe);
                w.opt_u64(faccion.map(u64::from));
            }
            Comando::ColocarHeroe { heroe, ubicacion } => {
                w.u8(67);
                w.u64(*heroe);
                ubicacion_escribir(&mut w, ubicacion);
            }
            Comando::HerirHeroe { heroe } => {
                w.u8(68);
                w.u64(*heroe);
            }
            Comando::SalirAlMundo { asentamiento, heroe, escuadras, carga } => {
                w.u8(69);
                w.u64(*asentamiento);
                w.u64(*heroe);
                lista_u64(&mut w, escuadras);
                lineas(&mut w, carga);
            }
            Comando::MovilizarEjercito { asentamiento, heroe, escuadras, carga, politica } => {
                w.u8(70);
                w.u64(*asentamiento);
                w.u64(*heroe);
                lista_u64(&mut w, escuadras);
                lineas(&mut w, carga);
                w.u8(politica_a_u8(*politica));
            }
            Comando::SalirDelCampamento { campamento, heroe, escuadras, carga, politica } => {
                w.u8(71);
                w.u32(*campamento);
                w.u64(*heroe);
                lista_u64(&mut w, escuadras);
                lineas(&mut w, carga);
                w.u8(politica_a_u8(*politica));
            }
            Comando::EntrarEnAsentamiento { asentamiento, heroe } => {
                w.u8(72);
                w.u64(*asentamiento);
                w.u64(*heroe);
            }
            Comando::SalirDeAsentamiento { asentamiento, heroe } => {
                w.u8(73);
                w.u64(*asentamiento);
                w.u64(*heroe);
            }
            Comando::Guarnecer { asentamiento, heroe } => {
                w.u8(74);
                w.u64(*asentamiento);
                w.u64(*heroe);
            }
            Comando::EntrarEnCampamento { campamento, heroe } => {
                w.u8(75);
                w.u32(*campamento);
                w.u64(*heroe);
            }
            Comando::UnirseAEjercito { asentamiento, heroe, ejercito, escuadras, carga } => {
                w.u8(76);
                w.u64(*asentamiento);
                w.u64(*heroe);
                w.u64(*ejercito);
                lista_u64(&mut w, escuadras);
                lineas(&mut w, carga);
            }
            Comando::UnirseEnCampo { heroe, ejercito } => {
                w.u8(77);
                w.u64(*heroe);
                w.u64(*ejercito);
            }
            Comando::ResponderPeticionDeUnion { heroe, ejercito, solicitante, aceptar } => {
                w.u8(78);
                w.u64(*heroe);
                w.u64(*ejercito);
                w.u64(*solicitante);
                w.bool(*aceptar);
            }
            Comando::SepararseDelEjercito { heroe } => {
                w.u8(79);
                w.u64(*heroe);
            }
            Comando::CederLiderazgo { heroe, sucesor } => {
                w.u8(80);
                w.u64(*heroe);
                w.u64(*sucesor);
            }
            Comando::OrganizarEjercito { heroe, politica } => {
                w.u8(81);
                w.u64(*heroe);
                w.u8(politica_a_u8(*politica));
            }
            Comando::CancelarFormacion { heroe } => {
                w.u8(82);
                w.u64(*heroe);
            }
            Comando::Repostar { heroe, asentamiento } => {
                w.u8(83);
                w.u64(*heroe);
                w.u64(*asentamiento);
            }
            Comando::ConvocarEjercito { heroe, escuadras, carga, politica } => {
                w.u8(84);
                w.u64(*heroe);
                lista_u64(&mut w, escuadras);
                lineas(&mut w, carga);
                w.u8(politica_a_u8(*politica));
            }
            Comando::UnirseAConvocatoria { heroe, convocatoria, escuadras, carga } => {
                w.u8(85);
                w.u64(*heroe);
                w.u64(*convocatoria);
                lista_u64(&mut w, escuadras);
                lineas(&mut w, carga);
            }
            Comando::ResponderPeticionDeConvocatoria { heroe, convocatoria, solicitante, aceptar } => {
                w.u8(86);
                w.u64(*heroe);
                w.u64(*convocatoria);
                w.u64(*solicitante);
                w.bool(*aceptar);
            }
            Comando::CambiarSeleccionDeConvocatoria { heroe, escuadras, carga } => {
                w.u8(87);
                w.u64(*heroe);
                lista_u64(&mut w, escuadras);
                lineas(&mut w, carga);
            }
            Comando::SepararseDeConvocatoria { heroe } => {
                w.u8(88);
                w.u64(*heroe);
            }
            Comando::CancelarConvocatoria { heroe } => {
                w.u8(89);
                w.u64(*heroe);
            }
            Comando::PartirConvocatoria { heroe } => {
                w.u8(90);
                w.u64(*heroe);
            }
            Comando::Perseguir { heroe, objetivo } => {
                w.u8(91);
                w.u64(*heroe);
                objetivo_escribir(&mut w, objetivo);
            }
            Comando::DejarDePerseguir { heroe } => {
                w.u8(92);
                w.u64(*heroe);
            }
            Comando::Inspeccionar { heroe, objetivo } => {
                w.u8(93);
                w.u64(*heroe);
                objetivo_escribir(&mut w, objetivo);
            }
            Comando::FijarRelacion { a, b, relacion } => {
                w.u8(94);
                w.u32(*a);
                w.u32(*b);
                w.bool(relacion.is_some());
                w.u8(relacion.map_or(0, relacion_a_u8));
            }
            Comando::Atacar { heroe, objetivo, declara_guerra } => {
                w.u8(95);
                w.u64(*heroe);
                objetivo_de_ataque_escribir(&mut w, objetivo);
                w.bool(*declara_guerra);
            }
            Comando::ColocarBandidos { posicion, nivel } => {
                w.u8(96);
                w.punto(*posicion);
                w.u8(*nivel);
            }
            Comando::DarEscuadra { heroe, tropa, cantidad } => {
                w.u8(97);
                w.u64(*heroe);
                w.u8(tropa_a_u8(*tropa));
                w.i64(*cantidad);
            }
            Comando::FijarControlador { heroe, controlador } => {
                w.u8(98);
                w.u64(*heroe);
                w.u8(controlador_a_u8(*controlador));
            }
            Comando::UnirseABatalla { heroe, batalla, lado } => {
                w.u8(99);
                w.u64(*heroe);
                w.u64(*batalla);
                w.u8(match lado {
                    None => 0,
                    Some(Lado::Atacante) => 1,
                    Some(Lado::Defensor) => 2,
                });
            }
            Comando::CancelarBatalla { heroe, batalla } => {
                w.u8(100);
                w.u64(*heroe);
                w.u64(*batalla);
            }
            Comando::RegistrarAsignacion { batalla, asignacion } => {
                w.u8(101);
                w.u64(*batalla);
                asignacion_escribir(&mut w, asignacion);
            }
            Comando::ConfirmarInicio { batalla, intento, revision } => {
                w.u8(102);
                w.u64(*batalla);
                w.texto(intento);
                w.u32(*revision);
            }
            Comando::RegistrarTokens { batalla, intento, tokens } => {
                w.u8(103);
                w.u64(*batalla);
                w.texto(intento);
                tokens_escribir(&mut w, tokens);
            }
            Comando::RegistrarSalida { batalla, salida } => {
                w.u8(104);
                w.u64(*batalla);
                salida_escribir(&mut w, salida);
            }
            Comando::AplicarResultado { batalla, resultado } => {
                w.u8(105);
                w.u64(*batalla);
                resultado_escribir(&mut w, resultado);
            }
            Comando::ColocarAvanzada { asentamiento, heroe, posicion } => {
                w.u8(106);
                w.u64(*asentamiento);
                w.u64(*heroe);
                w.punto(*posicion);
            }
            Comando::EntregarMaterialesAAvanzada { ejercito, avanzada } => {
                w.u8(107);
                w.u64(*ejercito);
                w.u64(*avanzada);
            }
            Comando::MejorarAvanzada { heroe, avanzada } => {
                w.u8(108);
                w.u64(*heroe);
                w.u64(*avanzada);
            }
            Comando::RepararAvanzada { heroe, avanzada } => {
                w.u8(109);
                w.u64(*heroe);
                w.u64(*avanzada);
            }
            Comando::DestruirAvanzada { heroe, avanzada } => {
                w.u8(110);
                w.u64(*heroe);
                w.u64(*avanzada);
            }
            Comando::ElegirTropaDeAvanzada { heroe, avanzada, tropa } => {
                w.u8(111);
                w.u64(*heroe);
                w.u64(*avanzada);
                w.u8(tropa_a_u8(*tropa));
            }
            Comando::GuarnecerAvanzada { heroe, avanzada, escuadras } => {
                w.u8(112);
                w.u64(*heroe);
                w.u64(*avanzada);
                lista_u64(&mut w, escuadras);
            }
            Comando::RetirarDeAvanzada { heroe, avanzada, escuadras } => {
                w.u8(113);
                w.u64(*heroe);
                w.u64(*avanzada);
                lista_u64(&mut w, escuadras);
            }
            Comando::RepostarEnAvanzada { heroe, avanzada } => {
                w.u8(114);
                w.u64(*heroe);
                w.u64(*avanzada);
            }
            Comando::DepositarEnAvanzada { ejercito, avanzada, recurso, cantidad } => {
                w.u8(115);
                w.u64(*ejercito);
                w.u64(*avanzada);
                w.u8(recurso_a_u8(*recurso));
                w.i64(*cantidad);
            }
            Comando::CancelarObraDeAvanzada { heroe, avanzada } => {
                w.u8(116);
                w.u64(*heroe);
                w.u64(*avanzada);
            }
            Comando::EntrarEnPueblo { pueblo, heroe } => {
                w.u8(119);
                w.u64(*pueblo);
                w.u64(*heroe);
            }
            Comando::SalirDePueblo { pueblo, heroe } => {
                w.u8(120);
                w.u64(*pueblo);
                w.u64(*heroe);
            }
            Comando::EntrarEnAvanzada { avanzada, heroe } => {
                w.u8(121);
                w.u64(*avanzada);
                w.u64(*heroe);
            }
            Comando::SalirDeAvanzada { avanzada, heroe } => {
                w.u8(122);
                w.u64(*avanzada);
                w.u64(*heroe);
            }
            Comando::FijarAcceso { asentamiento, aliados, neutrales, enemigos } => {
                w.u8(117);
                w.u64(*asentamiento);
                w.bool(*aliados);
                w.bool(*neutrales);
                w.bool(*enemigos);
            }
            Comando::VaciarCarro { heroe } => {
                w.u8(118);
                w.u64(*heroe);
            }
            Comando::ComprometerRecinto { asentamiento, cargo, nivel } => {
                w.u8(123);
                w.u64(*asentamiento);
                w.u8(cargo_a_u8(*cargo));
                w.u8(*nivel);
            }
            Comando::MejorarRecinto { asentamiento, cargo, recinto } => {
                w.u8(124);
                w.u64(*asentamiento);
                w.u8(cargo_a_u8(*cargo));
                w.u32(*recinto);
            }
            Comando::AbandonarRecinto { asentamiento, recinto } => {
                w.u8(125);
                w.u64(*asentamiento);
                w.u32(*recinto);
            }
            Comando::ComprarMirada { heroe, origen, centro } => {
                w.u8(126);
                w.u64(*heroe);
                origen_escribir(&mut w, *origen);
                w.punto(*centro);
            }
            Comando::ComprarInformePlaza { heroe, origen, asentamiento } => {
                w.u8(127);
                w.u64(*heroe);
                origen_escribir(&mut w, *origen);
                w.u64(*asentamiento);
            }
            Comando::CrearFaccion { heroe, nombre, sigilo } => {
                w.u8(128);
                w.u64(*heroe);
                w.texto(nombre);
                w.bool(sigilo.is_some());
                if let Some(s) = sigilo {
                    sigilo_escribir(&mut w, s);
                }
            }
            Comando::SolicitarIngreso { heroe, faccion } => {
                w.u8(129);
                w.u64(*heroe);
                w.u32(*faccion);
            }
            Comando::ResponderSolicitud { heroe, faccion, solicitante, aceptar } => {
                w.u8(130);
                w.u64(*heroe);
                w.u32(*faccion);
                w.u64(*solicitante);
                w.bool(*aceptar);
            }
            Comando::DejarFaccion { heroe } => {
                w.u8(131);
                w.u64(*heroe);
            }
            Comando::AsignarRey { heroe, faccion, designado } => {
                w.u8(132);
                w.u64(*heroe);
                w.u32(*faccion);
                w.u64(*designado);
            }
            Comando::AsignarEmbajador { heroe, faccion, designado } => {
                w.u8(133);
                w.u64(*heroe);
                w.u32(*faccion);
                w.u64(*designado);
            }
            Comando::CambiarResidencia { heroe, asentamiento } => {
                w.u8(134);
                w.u64(*heroe);
                w.u64(*asentamiento);
            }
            Comando::ResidirEnCampamento { heroe, campamento } => {
                w.u8(135);
                w.u64(*heroe);
                w.u32(*campamento);
            }
            Comando::DejarResidencia { heroe } => {
                w.u8(136);
                w.u64(*heroe);
            }
            Comando::DesignarCapital { heroe, faccion, asentamiento } => {
                w.u8(137);
                w.u64(*heroe);
                w.u32(*faccion);
                w.u64(*asentamiento);
            }
            Comando::RenombrarAsentamiento { heroe, asentamiento, nombre } => {
                w.u8(138);
                w.u64(*heroe);
                w.u64(*asentamiento);
                w.texto(nombre);
            }
        }
        w.0
    }

    pub(crate) fn decodificar(bytes: &[u8]) -> Result<Peticion, ErrorFormato> {
        let mut r = Lector::nuevo(bytes);
        let (actor, admin, peticion_id, version_esperada) = (r.texto()?, r.bool()?, r.texto()?, r.opt_u64()?);
        let comando = match r.u8()? {
            1 => Comando::CrearEjercito { cuenta: r.texto()?, faccion: r.u32()?, posicion: r.punto()?, velocidad: r.i64()?, viveres: r.i64()? },
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
            21 => Comando::ProponerTrueque { a: r.u64()?, b: r.u64()?, lineas_a: leer_lineas(&mut r)?, lineas_b: leer_lineas(&mut r)? },
            22 => Comando::AceptarTrueque { acuerdo: r.u64()? },
            23 => Comando::RechazarTrueque { acuerdo: r.u64()? },
            24 => Comando::ColocarOrdenMercado { asentamiento: r.u64()?, venta: r.bool()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()?, precio: r.opt_i64()? },
            25 => Comando::CancelarOrdenMercado { orden: r.u64()? },
            26 => Comando::ComerciarEnPlaza { ejercito: r.u64()?, orden: r.u64()?, cantidad: r.i64()? },
            27 => Comando::CrearCaravana { asentamiento: r.u64()? },
            28 => Comando::AnadirCarro { caravana: r.u64()?, tipo: tipo_carro_de_u8(r.u8()?)? },
            29 => Comando::AnadirAnimal { caravana: r.u64()?, carro: r.u32()?, animal: animal_de_u8(r.u8()?)? },
            30 => Comando::MoverCarro { de: r.u64()?, a: r.u64()?, carro: r.u32()? },
            31 => Comando::ReservarCaravana { caravana: r.u64()?, reservada: r.bool()? },
            32 => Comando::PrepararCaravana { caravana: r.u64()?, destino: r.u64()?, carga: leer_lineas(&mut r)?, salir_en: r.opt_i64()? },
            33 => Comando::CancelarCaravana { caravana: r.u64()? },
            34 => Comando::MoverCargaAparcada { caravana: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()?, cargar: r.bool()? },
            35 => Comando::EnviarCaravanaAlOrigen { caravana: r.u64()? },
            36 => Comando::EntregarDeCaravana { ejercito: r.u64()?, caravana: r.u64()?, acuerdo: r.u64()? },
            37 => Comando::DarRecursos { asentamiento: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()? },
            38 => Comando::DarSuministro { ejercito: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()? },
            39 => Comando::AdjuntarCaravana { caravana: r.u64()?, ejercito: r.u64()? },
            40 => Comando::AparcarCaravana { caravana: r.u64()?, asentamiento: r.u64()? },
            41 => Comando::ComprarEnCampamento { heroe: r.u64()?, campamento: r.u32()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()? },
            42 => Comando::RepostarEnCampamento { heroe: r.u64()?, campamento: r.u32()? },
            43 => Comando::FijarResidencia { heroe: r.u64()?, residencia: residencia_leer(&mut r)? },
            44 => Comando::DarAlmacenPersonal { heroe: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()? },
            45 => Comando::MarcarPueblo { asentamiento: r.u64()?, nodo: r.u32()? },
            46 => Comando::EntregarMateriales { ejercito: r.u64()?, pueblo: r.u64()? },
            47 => Comando::DestruirPueblo { pueblo: r.u64()? },
            48 => Comando::ReponerCarro { pueblo: r.u64()? },
            49 => Comando::CrearHeroe { cuenta: r.texto()?, campamento: r.u32()?, controlador: controlador_de_u8(r.u8()?)? },
            50 => Comando::ReclutarTropa { asentamiento: r.u64()?, heroe: r.u64()?, tropa: tropa_de_u8(r.u8()?)? },
            51 => Comando::ReclutarEnCampamento {
                heroe: r.u64()?,
                tropa: tropa_de_u8(r.u8()?)?,
                pagar_con: match r.u8()? {
                    0 => PagarCon::AlmacenPersonal,
                    1 => PagarCon::Carro,
                    _ => return Err(ErrorFormato("forma de pago desconocida")),
                },
            },
            52 => Comando::PedirPrestamo { heroe: r.u64()?, tropas: (0..r.cuenta()?).map(|_| tropa_de_u8(r.u8()?)).collect::<Result<_, _>>()? },
            53 => Comando::ReponerPrestamo { heroe: r.u64()? },
            54 => Comando::GuardarLoadout {
                heroe: r.u64()?,
                loadout: r.opt_u64()?,
                nombre: r.texto()?,
                escuadras: leer_lista_u64(&mut r)?,
                perks: (0..r.cuenta()?).map(|_| r.u32()).collect::<Result<_, _>>()?,
                activo: match r.u8()? {
                    0 => Some(false),
                    1 => Some(true),
                    2 => None,
                    _ => return Err(ErrorFormato("valor de «activo» desconocido")),
                },
            },
            55 => Comando::BorrarLoadout { heroe: r.u64()?, loadout: r.u64()? },
            56 => Comando::RepartirPuntos { heroe: r.u64()?, fuerza: r.i64()?, destreza: r.i64()?, armadura: r.i64()?, vitalidad: r.i64()? },
            57 => Comando::OrdenarEscuadras { heroe: r.u64()?, escuadras: leer_lista_u64(&mut r)? },
            58 => Comando::AsignarGuarnicion { heroe: r.u64()?, escuadra: r.u64()? },
            59 => Comando::RetirarGuarnicion { heroe: r.u64()?, escuadra: r.u64()? },
            60 => Comando::PasarAViveres { heroe: r.u64()?, cantidad: r.i64()? },
            61 => Comando::AsignarEscolta { caravana: r.u64()?, heroe: r.u64()?, escuadras: leer_lista_u64(&mut r)? },
            62 => {
                let (caravana, heroe, con_ids) = (r.u64()?, r.u64()?, r.bool()?);
                let ids = leer_lista_u64(&mut r)?;
                Comando::QuitarEscolta { caravana, heroe, escuadras: con_ids.then_some(ids) }
            }
            63 => Comando::Desconectarse { heroe: r.u64()? },
            64 => Comando::Conectarse { heroe: r.u64()? },
            65 => Comando::FijarTecnologias { faccion: r.u32()?, tecnologias: (0..r.cuenta()?).map(|_| r.texto()).collect::<Result<_, _>>()? },
            66 => Comando::FijarFaccionDeHeroe {
                heroe: r.u64()?,
                faccion: r.opt_u64()?.map(u32::try_from).transpose().map_err(|_| ErrorFormato("Facción fuera de rango"))?,
            },
            67 => Comando::ColocarHeroe { heroe: r.u64()?, ubicacion: ubicacion_leer(&mut r)? },
            68 => Comando::HerirHeroe { heroe: r.u64()? },
            69 => Comando::SalirAlMundo { asentamiento: r.u64()?, heroe: r.u64()?, escuadras: leer_lista_u64(&mut r)?, carga: leer_lineas(&mut r)? },
            70 => Comando::MovilizarEjercito { asentamiento: r.u64()?, heroe: r.u64()?, escuadras: leer_lista_u64(&mut r)?, carga: leer_lineas(&mut r)?, politica: politica_de_u8(r.u8()?)? },
            71 => Comando::SalirDelCampamento { campamento: r.u32()?, heroe: r.u64()?, escuadras: leer_lista_u64(&mut r)?, carga: leer_lineas(&mut r)?, politica: politica_de_u8(r.u8()?)? },
            72 => Comando::EntrarEnAsentamiento { asentamiento: r.u64()?, heroe: r.u64()? },
            73 => Comando::SalirDeAsentamiento { asentamiento: r.u64()?, heroe: r.u64()? },
            74 => Comando::Guarnecer { asentamiento: r.u64()?, heroe: r.u64()? },
            75 => Comando::EntrarEnCampamento { campamento: r.u32()?, heroe: r.u64()? },
            76 => Comando::UnirseAEjercito { asentamiento: r.u64()?, heroe: r.u64()?, ejercito: r.u64()?, escuadras: leer_lista_u64(&mut r)?, carga: leer_lineas(&mut r)? },
            77 => Comando::UnirseEnCampo { heroe: r.u64()?, ejercito: r.u64()? },
            78 => Comando::ResponderPeticionDeUnion { heroe: r.u64()?, ejercito: r.u64()?, solicitante: r.u64()?, aceptar: r.bool()? },
            79 => Comando::SepararseDelEjercito { heroe: r.u64()? },
            80 => Comando::CederLiderazgo { heroe: r.u64()?, sucesor: r.u64()? },
            81 => Comando::OrganizarEjercito { heroe: r.u64()?, politica: politica_de_u8(r.u8()?)? },
            82 => Comando::CancelarFormacion { heroe: r.u64()? },
            83 => Comando::Repostar { heroe: r.u64()?, asentamiento: r.u64()? },
            84 => Comando::ConvocarEjercito { heroe: r.u64()?, escuadras: leer_lista_u64(&mut r)?, carga: leer_lineas(&mut r)?, politica: politica_de_u8(r.u8()?)? },
            85 => Comando::UnirseAConvocatoria { heroe: r.u64()?, convocatoria: r.u64()?, escuadras: leer_lista_u64(&mut r)?, carga: leer_lineas(&mut r)? },
            86 => Comando::ResponderPeticionDeConvocatoria { heroe: r.u64()?, convocatoria: r.u64()?, solicitante: r.u64()?, aceptar: r.bool()? },
            87 => Comando::CambiarSeleccionDeConvocatoria { heroe: r.u64()?, escuadras: leer_lista_u64(&mut r)?, carga: leer_lineas(&mut r)? },
            88 => Comando::SepararseDeConvocatoria { heroe: r.u64()? },
            89 => Comando::CancelarConvocatoria { heroe: r.u64()? },
            90 => Comando::PartirConvocatoria { heroe: r.u64()? },
            91 => Comando::Perseguir { heroe: r.u64()?, objetivo: objetivo_leer(&mut r)? },
            92 => Comando::DejarDePerseguir { heroe: r.u64()? },
            93 => Comando::Inspeccionar { heroe: r.u64()?, objetivo: objetivo_leer(&mut r)? },
            94 => {
                let (a, b, hay, relacion) = (r.u32()?, r.u32()?, r.bool()?, relacion_de_u8(r.u8()?)?);
                Comando::FijarRelacion { a, b, relacion: hay.then_some(relacion) }
            }
            95 => Comando::Atacar { heroe: r.u64()?, objetivo: objetivo_de_ataque_leer(&mut r)?, declara_guerra: r.bool()? },
            96 => Comando::ColocarBandidos { posicion: r.punto()?, nivel: r.u8()? },
            97 => Comando::DarEscuadra { heroe: r.u64()?, tropa: tropa_de_u8(r.u8()?)?, cantidad: r.i64()? },
            98 => Comando::FijarControlador { heroe: r.u64()?, controlador: controlador_de_u8(r.u8()?)? },
            99 => Comando::UnirseABatalla {
                heroe: r.u64()?,
                batalla: r.u64()?,
                lado: match r.u8()? {
                    0 => None,
                    1 => Some(Lado::Atacante),
                    2 => Some(Lado::Defensor),
                    _ => return Err(ErrorFormato("lado de batalla desconocido")),
                },
            },
            100 => Comando::CancelarBatalla { heroe: r.u64()?, batalla: r.u64()? },
            101 => Comando::RegistrarAsignacion { batalla: r.u64()?, asignacion: asignacion_leer(&mut r)? },
            102 => Comando::ConfirmarInicio { batalla: r.u64()?, intento: r.texto()?, revision: r.u32()? },
            103 => Comando::RegistrarTokens { batalla: r.u64()?, intento: r.texto()?, tokens: tokens_leer(&mut r)? },
            104 => Comando::RegistrarSalida { batalla: r.u64()?, salida: salida_leer(&mut r)? },
            105 => Comando::AplicarResultado { batalla: r.u64()?, resultado: resultado_leer(&mut r)? },
            106 => Comando::ColocarAvanzada { asentamiento: r.u64()?, heroe: r.u64()?, posicion: r.punto()? },
            107 => Comando::EntregarMaterialesAAvanzada { ejercito: r.u64()?, avanzada: r.u64()? },
            108 => Comando::MejorarAvanzada { heroe: r.u64()?, avanzada: r.u64()? },
            109 => Comando::RepararAvanzada { heroe: r.u64()?, avanzada: r.u64()? },
            110 => Comando::DestruirAvanzada { heroe: r.u64()?, avanzada: r.u64()? },
            111 => Comando::ElegirTropaDeAvanzada { heroe: r.u64()?, avanzada: r.u64()?, tropa: tropa_de_u8(r.u8()?)? },
            112 => Comando::GuarnecerAvanzada { heroe: r.u64()?, avanzada: r.u64()?, escuadras: leer_lista_u64(&mut r)? },
            113 => Comando::RetirarDeAvanzada { heroe: r.u64()?, avanzada: r.u64()?, escuadras: leer_lista_u64(&mut r)? },
            114 => Comando::RepostarEnAvanzada { heroe: r.u64()?, avanzada: r.u64()? },
            115 => Comando::DepositarEnAvanzada { ejercito: r.u64()?, avanzada: r.u64()?, recurso: recurso_de_u8(r.u8()?)?, cantidad: r.i64()? },
            116 => Comando::CancelarObraDeAvanzada { heroe: r.u64()?, avanzada: r.u64()? },
            117 => Comando::FijarAcceso { asentamiento: r.u64()?, aliados: r.bool()?, neutrales: r.bool()?, enemigos: r.bool()? },
            118 => Comando::VaciarCarro { heroe: r.u64()? },
            119 => Comando::EntrarEnPueblo { pueblo: r.u64()?, heroe: r.u64()? },
            120 => Comando::SalirDePueblo { pueblo: r.u64()?, heroe: r.u64()? },
            121 => Comando::EntrarEnAvanzada { avanzada: r.u64()?, heroe: r.u64()? },
            122 => Comando::SalirDeAvanzada { avanzada: r.u64()?, heroe: r.u64()? },
            123 => Comando::ComprometerRecinto { asentamiento: r.u64()?, cargo: cargo_de_u8(r.u8()?)?, nivel: r.u8()? },
            124 => Comando::MejorarRecinto { asentamiento: r.u64()?, cargo: cargo_de_u8(r.u8()?)?, recinto: r.u32()? },
            125 => Comando::AbandonarRecinto { asentamiento: r.u64()?, recinto: r.u32()? },
            126 => Comando::ComprarMirada { heroe: r.u64()?, origen: origen_leer(&mut r)?, centro: r.punto()? },
            127 => Comando::ComprarInformePlaza { heroe: r.u64()?, origen: origen_leer(&mut r)?, asentamiento: r.u64()? },
            128 => {
                let (heroe, nombre) = (r.u64()?, r.texto()?);
                Comando::CrearFaccion { heroe, nombre, sigilo: if r.bool()? { Some(sigilo_leer(&mut r)?) } else { None } }
            }
            129 => Comando::SolicitarIngreso { heroe: r.u64()?, faccion: r.u32()? },
            130 => Comando::ResponderSolicitud { heroe: r.u64()?, faccion: r.u32()?, solicitante: r.u64()?, aceptar: r.bool()? },
            131 => Comando::DejarFaccion { heroe: r.u64()? },
            132 => Comando::AsignarRey { heroe: r.u64()?, faccion: r.u32()?, designado: r.u64()? },
            133 => Comando::AsignarEmbajador { heroe: r.u64()?, faccion: r.u32()?, designado: r.u64()? },
            134 => Comando::CambiarResidencia { heroe: r.u64()?, asentamiento: r.u64()? },
            135 => Comando::ResidirEnCampamento { heroe: r.u64()?, campamento: r.u32()? },
            136 => Comando::DejarResidencia { heroe: r.u64()? },
            137 => Comando::DesignarCapital { heroe: r.u64()?, faccion: r.u32()?, asentamiento: r.u64()? },
            138 => Comando::RenombrarAsentamiento { heroe: r.u64()?, asentamiento: r.u64()?, nombre: r.texto()? },
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
    /// Una orden vacía (sin cuerpo).
    pub const ORDEN_NO_SOPORTADA: &str = "orden.no_soportada";
    /// Una orden del contrato que el núcleo aún no tiene (política, tecnología, Aedas, intel, refundación, caravanas a mano y almacén personal): se rechaza con este código estable en vez de descartarse.
    pub const ORDEN_NO_DISPONIBLE: &str = "orden.no_disponible";
    /// Un valor de la orden fuera de su catálogo cerrado, o un id que no tiene la forma de ninguna entidad (`orden.forma_invalida`).
    pub const ORDEN_FORMA_INVALIDA: &str = "orden.forma_invalida";
    /// `obra-N` no es ningún pueblo ni ninguna avanzada.
    pub const OBRA_DE_MAPA_DESCONOCIDA: &str = "obra_de_mapa.desconocida";
    /// `repostar` sin `lugar_id` y sin ninguna plaza ni avanzada operativa a la puerta de la columna.
    pub const REPOSTAR_SIN_LUGAR: &str = "repostar.sin_lugar";
    pub const OBJETIVO_NO_SOPORTADO: &str = "objetivo.no_soportado";
    pub const RELOJ_INVALIDO: &str = "reloj.invalido";
    pub const SIN_MUNDO: &str = "mundo.no_cargado";
    pub const ASENTAMIENTO_DESCONOCIDO: &str = "asentamiento.desconocido";
    pub const SIN_CARAVANA: &str = "fundacion.sin_caravana";
    pub const FUNDACION_SIN_FACCION: &str = "fundacion.sin_faccion";
    /// Ningún héroe de la columna es ciudadano de la Facción de la caravana: no hay fundadores (`minFundadoresParaFaccionNueva`, 1).
    pub const FUNDACION_SIN_FUNDADORES: &str = "fundacion.sin_fundadores";

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
    pub const OBRA_SOLO_POR_PUEBLO: &str = "obra.solo_por_pueblo";
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
    // Trueque (`ErrorTrueque`).
    pub const TRUEQUE_DESCONOCIDO: &str = "trueque.desconocido";
    pub const TRUEQUE_MISMO_ASENTAMIENTO: &str = "trueque.mismo_asentamiento";
    pub const TRUEQUE_LADO_VACIO: &str = "trueque.lado_vacio";
    pub const TRUEQUE_CANTIDAD_NO_POSITIVA: &str = "trueque.cantidad_no_positiva";
    pub const TRUEQUE_RECURSO_REPETIDO: &str = "trueque.recurso_repetido";
    pub const TRUEQUE_NO_ESTA_PROPUESTO: &str = "trueque.no_esta_propuesto";
    pub const TRUEQUE_PROPUESTA_VENCIDA: &str = "trueque.propuesta_vencida";
    pub const TRUEQUE_NO_ACTIVO: &str = "trueque.no_activo";
    // Mercado de la plaza (`ErrorOrden`) y mostrador (`ErrorMostrador`).
    pub const MERCADO_SIN_MERCADO: &str = "mercado.sin_mercado";
    pub const ORDEN_DESCONOCIDA: &str = "orden.desconocida";
    pub const ORDEN_NO_ACTIVA: &str = "orden.no_activa";
    pub const ORDEN_CANTIDAD_NO_POSITIVA: &str = "orden.cantidad_no_positiva";
    pub const ORDEN_SIN_STOCK: &str = "orden.sin_stock";
    pub const MOSTRADOR_NO_ES_LIDER: &str = "mostrador.no_es_lider";
    pub const MOSTRADOR_ORDEN_AJENA: &str = "mostrador.orden_ajena";
    pub const MOSTRADOR_ORDEN_CADUCADA: &str = "mostrador.orden_caducada";
    pub const MOSTRADOR_FUERA_DE_LA_PUERTA: &str = "mostrador.fuera_de_la_puerta";
    pub const MOSTRADOR_CANTIDAD_NO_POSITIVA: &str = "mostrador.cantidad_no_positiva";
    pub const MOSTRADOR_PRECIO_NO_POSITIVO: &str = "mostrador.precio_no_positivo";
    pub const MOSTRADOR_RECURSO_ORO: &str = "mostrador.recurso_oro";
    pub const MOSTRADOR_NADA_QUE_SERVIR: &str = "mostrador.nada_que_servir";
    // Caravanas (`ErrorCaravana`).
    pub const CARAVANA_DESCONOCIDA: &str = "caravana.desconocida";
    pub const CARAVANA_NO_ES_COMERCIAL: &str = "caravana.no_es_comercial";
    pub const CARAVANA_NO_DISPONIBLE: &str = "caravana.no_disponible";
    pub const CARAVANA_NO_PREPARANDO: &str = "caravana.no_preparando";
    pub const CARAVANA_NO_APARCADA: &str = "caravana.no_aparcada";
    pub const CARAVANA_ESTADO_INVALIDO: &str = "caravana.estado_invalido";
    pub const CARAVANA_SIN_MERCADO: &str = "caravana.sin_mercado";
    pub const CARAVANA_EN_COOLDOWN: &str = "caravana.en_cooldown";
    pub const CARAVANA_CUPO_DE_CARAVANAS: &str = "caravana.cupo_de_caravanas";
    pub const CARAVANA_CUPO_DE_CARROS: &str = "caravana.cupo_de_carros";
    pub const CARAVANA_SIN_CARPINTERIA: &str = "caravana.sin_carpinteria";
    pub const CARAVANA_SIN_RECURSOS: &str = "caravana.sin_recursos";
    pub const CARAVANA_CARRO_INEXISTENTE: &str = "caravana.carro_inexistente";
    pub const CARAVANA_CARRO_YA_TIENE_ANIMAL: &str = "caravana.carro_ya_tiene_animal";
    pub const CARAVANA_ORIGEN_DISTINTO: &str = "caravana.origen_distinto";
    pub const CARAVANA_SIN_CAPACIDAD: &str = "caravana.sin_capacidad";
    pub const CARAVANA_CARGA_INVALIDA: &str = "caravana.carga_invalida";
    pub const CARAVANA_SIN_STOCK: &str = "caravana.sin_stock";
    pub const CARAVANA_CUPO_DE_ESCOLTA: &str = "caravana.cupo_de_escolta";
    pub const CARAVANA_ESCOLTA_INVALIDA: &str = "caravana.escolta_invalida";
    pub const CARAVANA_SIN_RUTA: &str = "caravana.sin_ruta";
    pub const CARAVANA_SALIDA_INVALIDA: &str = "caravana.salida_invalida";
    pub const CARAVANA_NO_HA_LLEGADO: &str = "caravana.no_ha_llegado";
    pub const CARAVANA_SIN_PLAN: &str = "caravana.sin_plan";
    pub const CARAVANA_NADA_QUE_MOVER: &str = "caravana.nada_que_mover";
    /// Adjuntar o aparcar con la columna lejos de la puerta de la plaza donde está la caravana.
    pub const CARAVANA_FUERA_DE_LA_PUERTA: &str = "caravana.fuera_de_la_puerta";
    /// Adjuntar a una columna de otra Facción.
    pub const CARAVANA_FACCION_AJENA: &str = "caravana.faccion_ajena";
    pub const CARAVANA_PLAZA_AJENA: &str = "caravana.plaza_ajena";
    pub const CARAVANA_NO_ESTA_ADJUNTA: &str = "caravana.no_esta_adjunta";
    // Entrega manual (§4.5).
    pub const ENTREGA_INVALIDA: &str = "entrega.invalida";
    pub const ENTREGA_NO_DEBE: &str = "entrega.no_debe";
    pub const ENTREGA_FUERA_DE_LA_PUERTA: &str = "entrega.fuera_de_la_puerta";
    pub const CANTIDAD_NO_POSITIVA: &str = "cantidad.no_positiva";
    // Pueblos extractores y obras de mapa (`ErrorPueblo` y lo que el núcleo comprueba sobre el mapa).
    pub const PUEBLO_DESCONOCIDO: &str = "pueblo.desconocido";
    pub const PUEBLO_NODO_DESCONOCIDO: &str = "pueblo.nodo_desconocido";
    /// El nodo es de ganado (el Corral es de la ciudad) o el tipo no va por pueblo.
    pub const PUEBLO_NODO_NO_EXTRAIBLE: &str = "pueblo.nodo_no_extraible";
    pub const PUEBLO_NODO_OCUPADO: &str = "pueblo.nodo_ocupado";
    pub const PUEBLO_FUERA_DE_LA_ZONA: &str = "pueblo.fuera_de_la_zona";
    /// El nodo no tiene ruta por tierra hasta la plaza madre: su carro no podría acarrear.
    pub const PUEBLO_SIN_RUTA: &str = "pueblo.sin_ruta";
    pub const PUEBLO_TECNOLOGIA_FALTANTE: &str = "pueblo.tecnologia_faltante";
    pub const PUEBLO_NIVEL_INSUFICIENTE: &str = "pueblo.nivel_insuficiente";
    pub const PUEBLO_SIN_CUPO: &str = "pueblo.sin_cupo";
    pub const PUEBLO_NO_ESTA_PENDIENTE: &str = "pueblo.no_esta_pendiente";
    pub const PUEBLO_NADA_QUE_ENTREGAR: &str = "pueblo.nada_que_entregar";
    pub const PUEBLO_FACCION_AJENA: &str = "pueblo.faccion_ajena";
    pub const PUEBLO_FUERA_DE_LA_PUERTA: &str = "pueblo.fuera_de_la_puerta";
    pub const PUEBLO_NO_ESTA_ACTIVO: &str = "pueblo.no_esta_activo";
    pub const PUEBLO_YA_TIENE_CARRO: &str = "pueblo.ya_tiene_carro";
    // Avanzadas (`ErrorAvanzada` y lo que el núcleo comprueba sobre el mapa, la columna y la guarnición; fase 4F).
    pub const AVANZADA_DESCONOCIDA: &str = "avanzada.desconocida";
    pub const AVANZADA_NIVEL_INSUFICIENTE: &str = "avanzada.nivel_insuficiente";
    pub const AVANZADA_SIN_CUPO: &str = "avanzada.sin_cupo";
    pub const AVANZADA_FUERA_DE_LA_ZONA: &str = "avanzada.fuera_de_la_zona";
    /// El sitio cae sobre la ciudad de la plaza madre (su radio es una décima del de su zona).
    pub const AVANZADA_DENTRO_DEL_ASENTAMIENTO: &str = "avanzada.dentro_del_asentamiento";
    /// El sitio pisa otra avanzada, un pueblo o un campamento (cada elemento del mapa mide 0,2).
    pub const AVANZADA_PISA_OTRO_ELEMENTO: &str = "avanzada.pisa_otro_elemento";
    /// No hay una mejora ni una reparación que cancelar.
    pub const AVANZADA_NADA_QUE_CANCELAR: &str = "avanzada.nada_que_cancelar";
    /// La columna del héroe no está en el sitio donde se pide colocarla.
    pub const AVANZADA_FUERA_DEL_SITIO: &str = "avanzada.fuera_del_sitio";
    pub const AVANZADA_FACCION_AJENA: &str = "avanzada.faccion_ajena";
    pub const AVANZADA_FUERA_DE_LA_PUERTA: &str = "avanzada.fuera_de_la_puerta";
    pub const AVANZADA_NO_ESTA_PENDIENTE: &str = "avanzada.no_esta_pendiente";
    pub const AVANZADA_NADA_QUE_ENTREGAR: &str = "avanzada.nada_que_entregar";
    pub const AVANZADA_NO_MEJORABLE: &str = "avanzada.no_mejorable";
    pub const AVANZADA_NO_REPARABLE: &str = "avanzada.no_reparable";
    pub const AVANZADA_YA_HAY_OBRA: &str = "avanzada.ya_hay_obra";
    pub const AVANZADA_CIUDADELA_NO_SE_DESTRUYE: &str = "avanzada.ciudadela_no_se_destruye";
    /// Solo funciona una avanzada operativa (no una obra ni unas ruinas).
    pub const AVANZADA_NO_OPERATIVA: &str = "avanzada.no_operativa";
    pub const AVANZADA_SIN_ALMACEN: &str = "avanzada.sin_almacen";
    pub const AVANZADA_ALMACEN_LLENO: &str = "avanzada.almacen_lleno";
    pub const AVANZADA_NADA_QUE_DEPOSITAR: &str = "avanzada.nada_que_depositar";
    pub const AVANZADA_NO_RESIDES: &str = "avanzada.no_resides";
    pub const AVANZADA_ESCUADRA_FUERA_DE_TU_COLUMNA: &str = "avanzada.escuadra_fuera_de_tu_columna";
    pub const AVANZADA_ESCUADRA_NO_ESTA: &str = "avanzada.escuadra_no_esta";
    pub const AVANZADA_TROPA_NO_ELEGIBLE: &str = "avanzada.tropa_no_elegible";
    pub const AVANZADA_TROPA_SIN_TECNOLOGIA: &str = "avanzada.tropa_sin_tecnologia";
    /// La plaza madre no tiene el edificio activo, al nivel que pide la tropa, para reclutarla.
    pub const AVANZADA_TROPA_SIN_EDIFICIO: &str = "avanzada.tropa_sin_edificio";
    // Murallas (`ErrorRecinto` del crate `asentamiento` y `ErrorMuralla` del trazo), en el orden en que se comprueban.
    /// El cargo que se invoca no es el Gobernador ni el Maestro de Obras.
    pub const MURALLAS_SIN_CARGO: &str = "murallas.sin_cargo";
    pub const MURALLAS_NIVEL_INEXISTENTE: &str = "murallas.nivel_inexistente";
    /// La plaza no tiene el nivel operativo que pide el nivel de recinto (empalizada 2, piedra 3, adarve 4).
    pub const MURALLAS_NIVEL_DE_PLAZA: &str = "murallas.nivel_de_plaza";
    /// Hay un recinto sin cerrar (en obra, o dañado por una conquista).
    pub const MURALLAS_EN_OBRA: &str = "murallas.en_obra";
    /// Ampliar exige 6 edificios fuera del recinto exterior.
    pub const MURALLAS_ARRABAL: &str = "murallas.arrabal";
    pub const MURALLAS_SIN_CENTRO_URBANO: &str = "murallas.sin_centro_urbano";
    pub const MURALLAS_NO_SE_PUEDE_CERRAR: &str = "murallas.no_se_puede_cerrar";
    pub const MURALLAS_FUERA_DE_LA_VENTANA: &str = "murallas.fuera_de_la_ventana";
    /// El anillo nuevo no envuelve estrictamente al anterior.
    pub const MURALLAS_NO_ENVUELVE: &str = "murallas.no_envuelve";
    pub const MURALLAS_RECINTO_DESCONOCIDO: &str = "murallas.recinto_desconocido";
    /// Mejorar un recinto sin cerrar.
    pub const MURALLAS_INCOMPLETO: &str = "murallas.incompleto";
    pub const MURALLAS_YA_MEJORANDO: &str = "murallas.ya_mejorando";
    pub const MURALLAS_EN_EL_MAXIMO: &str = "murallas.en_el_maximo";
    /// Abandonar un recinto que alguna vez estuvo completo.
    pub const MURALLAS_YA_CERRADO: &str = "murallas.ya_cerrado";
    // Mercado de los campamentos de mercenarios (`ErrorMercenarios`).
    pub const CAMPAMENTO_DESCONOCIDO: &str = "campamento.desconocido";
    pub const CAMPAMENTO_FUERA_DE_LA_PUERTA: &str = "campamento.fuera_de_la_puerta";
    pub const MERCENARIOS_CANTIDAD_INVALIDA: &str = "mercenarios.cantidad_invalida";
    pub const MERCENARIOS_SIN_PRECIO: &str = "mercenarios.sin_precio";
    pub const MERCENARIOS_TRIGO_NO_VA_AL_CARRO: &str = "mercenarios.trigo_no_va_al_carro";
    pub const MERCENARIOS_SIN_STOCK: &str = "mercenarios.sin_stock";
    pub const MERCENARIOS_CUPO_AGOTADO: &str = "mercenarios.cupo_agotado";
    pub const MERCENARIOS_SIN_ORO: &str = "mercenarios.sin_oro";
    pub const MERCENARIOS_SIN_SITIO: &str = "mercenarios.sin_sitio";
    pub const MERCENARIOS_VIVERES_LLENOS: &str = "mercenarios.viveres_llenos";
    // Reclutar en un campamento (`ErrorMercenarios`) y la tropa prestada.
    pub const MERCENARIOS_NO_RESIDES: &str = "mercenarios.no_resides";
    pub const MERCENARIOS_TROPA_NO_DISPONIBLE: &str = "mercenarios.tropa_no_disponible";
    pub const MERCENARIOS_YA_LA_TIENES_FUERA: &str = "mercenarios.ya_la_tienes_fuera";
    pub const MERCENARIOS_AL_TOPE: &str = "mercenarios.al_tope";
    pub const MERCENARIOS_SIN_RECLUTAS: &str = "mercenarios.sin_reclutas";
    pub const MERCENARIOS_PRESTAMO_SIN_TROPAS: &str = "mercenarios.prestamo_sin_tropas";
    pub const MERCENARIOS_NO_SE_PRESTA: &str = "mercenarios.no_se_presta";
    pub const MERCENARIOS_PRESTAMO_REPETIDA: &str = "mercenarios.prestamo_repetida";
    pub const MERCENARIOS_YA_PRESTADA: &str = "mercenarios.ya_prestada";
    pub const MERCENARIOS_NADA_QUE_REPONER: &str = "mercenarios.nada_que_reponer";
    // Reclutar en una plaza (`ErrorReclutamiento`), en el orden en que se comprueban.
    pub const RECLUTAMIENTO_NO_PUEDES: &str = "reclutamiento.no_puedes";
    pub const RECLUTAMIENTO_SIN_TECNOLOGIA: &str = "reclutamiento.sin_tecnologia";
    pub const RECLUTAMIENTO_RESERVADA: &str = "reclutamiento.reservada";
    pub const RECLUTAMIENTO_YA_LA_TIENES_FUERA: &str = "reclutamiento.ya_la_tienes_fuera";
    pub const RECLUTAMIENTO_SOLO_REPONER: &str = "reclutamiento.solo_reponer";
    pub const RECLUTAMIENTO_AL_TOPE: &str = "reclutamiento.al_tope";
    pub const RECLUTAMIENTO_SIN_POBLACION: &str = "reclutamiento.sin_poblacion";
    pub const RECLUTAMIENTO_SIN_EDIFICIO: &str = "reclutamiento.sin_edificio";
    pub const RECLUTAMIENTO_SIN_RECURSOS: &str = "reclutamiento.sin_recursos";
    pub const RECLUTAMIENTO_SIN_RESERVA_DE_TRIGO: &str = "reclutamiento.sin_reserva_de_trigo";
    // El héroe, sus puntos, loadouts, escuadras y guarnición (`ErrorHeroe`).
    pub const HEROE_YA_EXISTE: &str = "heroe.ya_existe";
    pub const HEROE_CAMPAMENTO_DESCONOCIDO: &str = "heroe.campamento_desconocido";
    pub const HEROE_PUNTOS_NEGATIVOS: &str = "heroe.puntos_negativos";
    pub const HEROE_NINGUN_PUNTO: &str = "heroe.ningun_punto";
    pub const HEROE_PUNTOS_INSUFICIENTES: &str = "heroe.puntos_insuficientes";
    pub const HEROE_TOPE_DE_ATRIBUTO: &str = "heroe.tope_de_atributo";
    pub const LOADOUT_SIN_NOMBRE: &str = "loadout.sin_nombre";
    pub const LOADOUT_DESCONOCIDO: &str = "loadout.desconocido";
    pub const LOADOUT_PERK_NO_DESBLOQUEADO: &str = "loadout.perk_no_desbloqueado";
    pub const ESCUADRA_REPETIDA: &str = "escuadra.repetida";
    pub const ESCUADRA_AJENA: &str = "escuadra.ajena";
    pub const ESCUADRA_EN_BATALLA: &str = "escuadra.en_batalla";
    pub const LIDERAZGO_SUPERADO: &str = "liderazgo.superado";
    pub const GUARNICION_SIN_RESIDENCIA: &str = "guarnicion.sin_residencia";
    pub const GUARNICION_NO_EN_CAMPAMENTO: &str = "guarnicion.no_en_campamento";
    pub const GUARNICION_YA_ASIGNADA: &str = "guarnicion.ya_asignada";
    pub const GUARNICION_SUPERA_EL_CUPO: &str = "guarnicion.supera_el_cupo";
    pub const GUARNICION_NO_ASIGNADA: &str = "guarnicion.no_asignada";
    // Presencia: dónde está el héroe y si está en el mundo.
    pub const HEROE_FUERA_DEL_MUNDO: &str = "heroe.fuera_del_mundo";
    pub const HEROE_DESCONECTANDOSE: &str = "heroe.desconectandose";
    pub const HEROE_SIN_COLUMNA: &str = "heroe.sin_columna";
    pub const HEROE_EN_COLUMNA: &str = "heroe.en_columna";
    pub const HEROE_NO_ESTA_EN_LA_PLAZA: &str = "heroe.no_esta_en_la_plaza";
    pub const HEROE_SIN_RESIDENCIA: &str = "heroe.sin_residencia";
    pub const HEROE_UBICACION_INVALIDA: &str = "heroe.ubicacion_invalida";
    // Tecnologías (andamiaje de administración).
    pub const TECNOLOGIA_DESCONOCIDA: &str = "tecnologia.desconocida";
    // Columnas (`ErrorColumna`) y convocatorias (`ErrorConvocatoria`).
    pub const COLUMNA_YA_VAS_EN_ELLA: &str = "columna.ya_vas_en_ella";
    pub const COLUMNA_NO_ADMITE: &str = "columna.no_admite";
    pub const COLUMNA_NO_ES_EJERCITO: &str = "columna.no_es_ejercito";
    pub const COLUMNA_SOLO_UNA_PERSONAL: &str = "columna.solo_una_personal";
    pub const COLUMNA_OTRA_FACCION: &str = "columna.otra_faccion";
    pub const COLUMNA_LEJOS: &str = "columna.lejos";
    pub const COLUMNA_NO_PREGUNTA: &str = "columna.no_pregunta";
    pub const COLUMNA_NO_VA_DENTRO: &str = "columna.no_va_dentro";
    pub const COLUMNA_DE_UNA_PERSONAL_NO_TE_SEPARAS: &str = "columna.de_una_personal_no_te_separas";
    pub const COLUMNA_EL_LIDER_NO_PUEDE: &str = "columna.el_lider_no_puede";
    pub const COLUMNA_ES_EL_ULTIMO: &str = "columna.es_el_ultimo";
    pub const COLUMNA_SOLO_EL_LIDER: &str = "columna.solo_el_lider";
    pub const COLUMNA_YA_ERES_EL_LIDER: &str = "columna.ya_eres_el_lider";
    pub const COLUMNA_SUCESOR_FUERA: &str = "columna.sucesor_fuera";
    pub const COLUMNA_NO_HAY_PETICION: &str = "columna.no_hay_peticion";
    pub const COLUMNA_PETICION_CADUCADA: &str = "columna.peticion_caducada";
    pub const COLUMNA_YA_ES_EJERCITO: &str = "columna.ya_es_ejercito";
    pub const COLUMNA_YA_ORGANIZANDO: &str = "columna.ya_organizando";
    pub const COLUMNA_NO_ESTA_SOLO: &str = "columna.no_esta_solo";
    pub const COLUMNA_POLITICA_INVALIDA: &str = "columna.politica_invalida";
    pub const COLUMNA_NO_ES_UNA_FORMACION: &str = "columna.no_es_una_formacion";
    pub const CONVOCATORIA_DESCONOCIDA: &str = "convocatoria.desconocida";
    pub const CONVOCATORIA_FUERA_DE_UN_LUGAR: &str = "convocatoria.fuera_de_un_lugar";
    pub const CONVOCATORIA_SIN_FACCION: &str = "convocatoria.sin_faccion";
    pub const CONVOCATORIA_YA_EN_ELLA: &str = "convocatoria.ya_en_ella";
    pub const CONVOCATORIA_POLITICA_INVALIDA: &str = "convocatoria.politica_invalida";
    pub const CONVOCATORIA_OTRO_LUGAR: &str = "convocatoria.otro_lugar";
    pub const CONVOCATORIA_OTRA_FACCION: &str = "convocatoria.otra_faccion";
    pub const CONVOCATORIA_NO_ESTAS: &str = "convocatoria.no_estas";
    pub const CONVOCATORIA_EL_LIDER_CANCELA: &str = "convocatoria.el_lider_cancela";
    pub const CONVOCATORIA_SOLO_EL_LIDER: &str = "convocatoria.solo_el_lider";
    pub const CONVOCATORIA_NO_HAY_PETICION: &str = "convocatoria.no_hay_peticion";
    pub const CONVOCATORIA_PETICION_CADUCADA: &str = "convocatoria.peticion_caducada";
    pub const CONVOCATORIA_LIDER_FUERA: &str = "convocatoria.lider_fuera";
    pub const CONVOCATORIA_NO_PUEDE_SALIR: &str = "convocatoria.no_puede_salir";
    // Salir y entrar en un lugar, y repostar.
    pub const HEROE_YA_EN_COLUMNA: &str = "heroe.ya_en_columna";
    pub const HEROE_EN_CONVOCATORIA: &str = "heroe.en_convocatoria";
    pub const HEROE_NO_ESTA_EN_EL_CAMPAMENTO: &str = "heroe.no_esta_en_el_campamento";
    pub const HEROE_DENTRO_DE_UN_LUGAR: &str = "heroe.dentro_de_un_lugar";
    pub const SALIDA_SOLO_DE_TU_RESIDENCIA: &str = "salida.solo_de_tu_residencia";
    pub const SALIDA_DESDE_TU_RESIDENCIA: &str = "salida.desde_tu_residencia";
    pub const SALIDA_SIN_ESCUADRAS: &str = "salida.sin_escuadras";
    pub const ESCUADRA_ANIQUILADA: &str = "escuadra.aniquilada";
    pub const ESCUADRA_FUERA_DEL_CAMPAMENTO: &str = "escuadra.fuera_del_campamento";
    pub const ESCUADRA_EN_LA_GUARNICION: &str = "escuadra.en_la_guarnicion";
    pub const CARGA_NEGATIVA: &str = "carga.negativa";
    pub const CARRO_LLENO: &str = "carro.no_cabe";
    pub const ALMACEN_SIN_EXISTENCIAS: &str = "almacen.sin_existencias";
    pub const COLUMNA_ES_UN_EJERCITO: &str = "columna.es_un_ejercito";
    pub const COLUMNA_EN_FORMACION: &str = "columna.en_formacion";
    pub const COLUMNA_PERSONAL_SIN_CARAVANAS: &str = "columna.personal_sin_caravanas";
    pub const COLUMNA_VIVERES_LLENOS: &str = "columna.viveres_llenos";
    pub const ASENTAMIENTO_FUERA_DE_LA_PUERTA: &str = "asentamiento.fuera_de_la_puerta";
    pub const PUERTA_CERRADA: &str = "puerta.cerrada";
    pub const HEROE_NO_ESTA_EN_EL_PUEBLO: &str = "heroe.no_esta_en_el_pueblo";
    pub const HEROE_NO_ESTA_EN_LA_AVANZADA: &str = "heroe.no_esta_en_la_avanzada";
    pub const VACIAR_SOLO_EN_TU_RESIDENCIA: &str = "carro.vaciar_solo_en_tu_residencia";
    pub const ALMACEN_PERSONAL_LLENO: &str = "almacen_personal.lleno";
    pub const REPOSTAR_PLAZA_CERRADA: &str = "repostar.plaza_cerrada";
    pub const REPOSTAR_SIN_EXISTENCIAS: &str = "repostar.sin_existencias";
    // Persecución (`ErrorPersecucion`), inspección (`ErrorInspeccion`) y batalla.
    pub const PERSECUCION_SIN_HEROE_SANO: &str = "persecucion.sin_heroe_sano";
    pub const PERSECUCION_PRESA_SIN_HEROE_SANO: &str = "persecucion.presa_sin_heroe_sano";
    pub const PERSECUCION_CLASE_DISTINTA: &str = "persecucion.clase_distinta";
    pub const PERSECUCION_PROPIA_O_ALIADA: &str = "persecucion.propia_o_aliada";
    pub const PERSECUCION_CARAVANA_ADJUNTA: &str = "persecucion.caravana_adjunta";
    pub const PERSECUCION_EJERCITO_SIN_CARAVANAS: &str = "persecucion.ejercito_sin_caravanas";
    pub const PERSECUCION_A_SI_MISMA: &str = "persecucion.a_si_misma";
    pub const PERSECUCION_A_CUBIERTO: &str = "persecucion.a_cubierto";
    pub const PERSECUCION_FUERA_DE_VISTA: &str = "persecucion.fuera_de_vista";
    pub const INSPECCION_LA_TUYA: &str = "inspeccion.la_tuya";
    pub const INSPECCION_PLAZA_PROPIA: &str = "inspeccion.plaza_propia";
    pub const INSPECCION_LEJOS: &str = "inspeccion.lejos";
    pub const INSPECCION_A_CUBIERTO: &str = "inspeccion.a_cubierto";
    // La intel de las tabernas (4H T3d; Niebla_De_Guerra §6).
    /// Quien compra intel tiene que ser ciudadano de una Facción (es para ella).
    pub const INTEL_SIN_FACCION: &str = "intel.sin_faccion";
    /// La taberna es de una plaza de otra Facción.
    pub const INTEL_PLAZA_AJENA: &str = "intel.plaza_ajena";
    /// Hay que estar dentro del lugar de la taberna.
    pub const INTEL_NO_PRESENTE: &str = "intel.no_presente";
    /// La plaza no tiene una Taberna levantada.
    pub const INTEL_SIN_TABERNA: &str = "intel.sin_taberna";
    pub const INTEL_FUERA_DEL_MAPA: &str = "intel.fuera_del_mapa";
    /// La taberna no admite Miradas (cupo 0).
    pub const INTEL_SIN_CUPO: &str = "intel.sin_cupo";
    /// La taberna ya tiene abiertas todas las Miradas que admite.
    pub const INTEL_CUPO_LLENO: &str = "intel.cupo_lleno";
    /// La Facción miró esa zona hace poco: hay que esperar a que se enfríe.
    pub const INTEL_ZONA_RECIENTE: &str = "intel.zona_reciente";
    pub const INTEL_PLAZA_PROPIA: &str = "intel.plaza_propia";
    /// La Facción no conoce esa plaza: ni tiene su ficha ni cae en una Mirada abierta.
    pub const INTEL_PLAZA_DESCONOCIDA: &str = "intel.plaza_desconocida";
    /// Ya tiene un Informe reciente de esa plaza.
    pub const INTEL_INFORME_RECIENTE: &str = "intel.informe_reciente";
    /// No hay oro para pagarla (en el almacén de la plaza, en el personal o en el carro).
    pub const INTEL_SIN_ORO: &str = "intel.sin_oro";
    /// La columna está apartada por una batalla: no se mueve ni se le dan órdenes de movimiento, de persecución ni de inspección.
    pub const COLUMNA_EN_BATALLA: &str = "columna.en_batalla";
    pub const RELACION_CONSIGO_MISMA: &str = "relacion.consigo_misma";
    // Combate con números (fase 4D).
    pub const HEROE_HERIDO: &str = "heroe.herido";
    pub const COMBATE_PROTECCION_DE_CAMPAMENTO: &str = "combate.proteccion_de_campamento";
    pub const COMBATE_LEJOS: &str = "combate.lejos";
    pub const COMBATE_PROPIA: &str = "combate.propia";
    pub const COMBATE_ALIADA: &str = "combate.aliada";
    /// `atacar` contra una Facción con la que no se está en guerra y sin `declara_guerra`: el cliente tiene que confirmarla (`Detalle::Guerra` dice contra quién).
    pub const GUERRA_CONFIRMAR: &str = "guerra.confirmar";
    pub const COMBATE_A_CUBIERTO: &str = "combate.a_cubierto";
    pub const COMBATE_CLASE_DISTINTA: &str = "combate.clase_distinta";
    pub const COMBATE_SIN_HEROE_SANO: &str = "combate.sin_heroe_sano";
    pub const ESCUADRA_CANTIDAD_INVALIDA: &str = "escuadra.cantidad_invalida";
    pub const COMBATE_SIN_FACCION: &str = "combate.sin_faccion";
    pub const COMBATE_SIN_ATACANTES: &str = "combate.sin_atacantes";
    pub const COMBATE_SOLO_UN_EJERCITO: &str = "combate.solo_un_ejercito";
    pub const COMBATE_CARAVANA_ADJUNTA: &str = "combate.caravana_adjunta";
    pub const COMBATE_PLAZA_PROTEGIDA: &str = "combate.plaza_protegida";
    pub const COMBATE_PLAZA_PROPIA: &str = "combate.plaza_propia";
    pub const COMBATE_PUEBLO_INACTIVO: &str = "combate.pueblo_inactivo";
    pub const COMBATE_AVANZADA_NO_EXISTE: &str = "combate.avanzada_no_existe";
    /// La avanzada aún es una obra pendiente de materiales o en marcha: se ve en el mapa, pero no se puede atacar (Doc 4.2.2).
    pub const COMBATE_AVANZADA_EN_OBRA: &str = "combate.avanzada_en_obra";
    /// Un fuerte en ruinas es invulnerable un día (Doc 5.17).
    pub const COMBATE_AVANZADA_INVULNERABLE: &str = "combate.avanzada_invulnerable";
    /// Unas ruinas de ciudadela sin dueño: no hay Facción con la que estar en guerra ni nada que defender.
    pub const COMBATE_AVANZADA_SIN_DUENO: &str = "combate.avanzada_sin_dueno";
    pub const CAMPAMENTO_DE_BANDIDOS_DESCONOCIDO: &str = "bandidos.desconocido";
    pub const BANDIDOS_NIVEL_INVALIDO: &str = "bandidos.nivel_invalido";
    pub const BANDIDOS_SOBRE_UNA_ZONA: &str = "bandidos.sobre_una_zona";
    // Batallas de Unity (fase 4E; Batalla_Unity_Ciclo §5-§9). El contrato responde `404` a `batalla.no_existe` y `409` al resto.
    /// Lo que toca la orden está en una batalla activa (§5): ni el héroe, ni su columna, ni la plaza asediada, ni el objetivo reciben órdenes.
    pub const BATALLA_BLOQUEADO: &str = "batalla.bloqueado";
    /// No existe, está cerrada o su plazo venció (404).
    pub const BATALLA_NO_EXISTE: &str = "batalla.no_existe";
    /// No puede unirse: está herido, ya combate, su columna ya combate, hay que estar a ≤ `radioEncuentro` del punto o el bando está lleno.
    pub const BATALLA_INVALIDA: &str = "batalla.invalida";
    pub const BATALLA_SIN_COLUMNA: &str = "batalla.sin_columna";
    pub const BATALLA_LEJOS: &str = "batalla.lejos";
    pub const BATALLA_LADO_LLENO: &str = "batalla.lado_lleno";
    pub const BATALLA_YA_COMBATE: &str = "batalla.ya_combate";
    pub const BATALLA_YA_SALIO: &str = "batalla.ya_salio";
    pub const BATALLA_NADIE_DE_FUERA: &str = "batalla.nadie_de_fuera";
    pub const BATALLA_SOLO_PERSONALES: &str = "batalla.solo_personales";
    pub const BATALLA_LADO_OBLIGATORIO: &str = "batalla.lado_obligatorio";
    pub const BATALLA_NADIE_AYUDA_A_BANDIDOS: &str = "batalla.nadie_ayuda_a_bandidos";
    pub const BATALLA_SIN_FACCION: &str = "batalla.sin_faccion";
    pub const BATALLA_NO_COMBATE_AQUI: &str = "batalla.no_combate_aqui";
    pub const BATALLA_LADO_DISTINTO: &str = "batalla.lado_distinto";
    pub const BATALLA_NO_PUEDE_CANCELAR: &str = "batalla.no_puede_cancelar";
    /// Otro servidor la tiene: gana la primera asignación.
    pub const BATALLA_YA_ASIGNADA: &str = "batalla.ya_asignada";
    pub const BATALLA_EN_CONVOCATORIA: &str = "batalla.en_convocatoria";
    pub const BATALLA_REVISION_VIEJA: &str = "batalla.revision_vieja";
    pub const BATALLA_ESTADO_INVALIDO: &str = "batalla.estado_invalido";
    /// No es el servidor de la asignación activa.
    pub const BATALLA_SERVIDOR_AJENO: &str = "batalla.servidor_ajeno";
    /// El mensaje no viene con la credencial de un servidor de batalla (`batalla-servidor:<id>`).
    pub const BATALLA_SIN_CREDENCIAL: &str = "batalla.sin_credencial";
    pub const BATALLA_TOKEN_INVALIDO: &str = "batalla.token_invalido";
    pub const BATALLA_NO_COMBATE: &str = "batalla.no_combate";
    pub const BATALLA_SALIDA_INVALIDA: &str = "batalla.salida_invalida";
    /// La salida de una partida en curso omite alguna de las escuadras autorizadas del héroe (Batalla_Unity_Ciclo §8.4).
    pub const BATALLA_SALIDA_INCOMPLETA: &str = "batalla.salida_incompleta";
    /// El checklist de §8.5 rechaza el resultado entero (el mensaje dice cuál de sus puntos).
    pub const BATALLA_RESULTADO_INVALIDO: &str = "batalla.resultado_invalido";
    /// «Ya se aplicó con otro resultado» (409).
    pub const BATALLA_OTRO_RESULTADO: &str = "batalla.otro_resultado";
    // La Facción, la ciudadanía y la residencia (sistema 5, tanda 5B; `ErrorFaccion` y `ErrorResidencia` del crate `politica`).
    pub const FACCION_NO_EXISTE: &str = "faccion.no_existe";
    /// El nombre queda vacío tras recortarlo.
    pub const FACCION_NOMBRE_VACIO: &str = "faccion.nombre_vacio";
    /// Otra Facción ya se llama así (sin mirar mayúsculas).
    pub const FACCION_NOMBRE_DUPLICADO: &str = "faccion.nombre_duplicado";
    /// Alguna pieza del sigilo no es del catálogo, o los dos colores del fondo son el mismo.
    pub const FACCION_SIGILO_INVALIDO: &str = "faccion.sigilo_invalido";
    /// Otra Facción lleva un sigilo idéntico.
    pub const FACCION_SIGILO_DUPLICADO: &str = "faccion.sigilo_duplicado";
    /// Ya es ciudadano de una Facción: 1 héroe, 1 Facción.
    pub const FACCION_YA_PERTENECE: &str = "faccion.ya_pertenece";
    /// Dejó una Facción hace menos de `CIUDADANIA.cooldownCreacionFaccionDias`.
    pub const FACCION_COOLDOWN_CREACION: &str = "faccion.cooldown_creacion";
    /// Responder a quien no ha pedido entrar.
    pub const FACCION_NO_HA_PEDIDO: &str = "faccion.no_ha_pedido";
    /// No es ciudadano de ninguna Facción.
    pub const FACCION_NO_PERTENECE: &str = "faccion.no_pertenece";
    /// Dejar la Facción yendo en un ejército (aunque sea solo) o en uno en preparación (Q7): primero se separa.
    pub const FACCION_EN_EJERCITO: &str = "faccion.en_ejercito";
    /// El designado para Rey o Embajador no es ciudadano de la Facción.
    pub const FACCION_NO_CIUDADANO: &str = "faccion.no_ciudadano";
    /// Nombrar Embajador en una Facción sin Rey.
    pub const FACCION_SIN_REY: &str = "faccion.sin_rey";
    /// Cambió de residencia hace menos de `CIUDADANIA.cooldownCambioResidenciaDias`.
    pub const RESIDENCIA_COOLDOWN: &str = "residencia.cooldown";
    /// Solo se reside en una plaza de la propia Facción.
    pub const RESIDENCIA_PLAZA_AJENA: &str = "residencia.plaza_ajena";
    pub const RESIDENCIA_YA_RESIDE: &str = "residencia.ya_reside";
    /// La plaza lo ha vetado (los vetos llegan en 5E).
    pub const RESIDENCIA_VETADO: &str = "residencia.vetado";
    /// Dejar la residencia sin residir en ninguna plaza (en un campamento de mercenarios se cambia de campamento o se muda a una plaza).
    pub const RESIDENCIA_SIN_PLAZA: &str = "residencia.sin_plaza";
    // La capital de la Facción (sistema 5, tanda 5C; `ErrorCapital` del crate `politica`).
    /// Designar capital una plaza que no es de la Facción.
    pub const CAPITAL_PLAZA_AJENA: &str = "capital.plaza_ajena";
    /// La plaza no tiene un Palacio activo.
    pub const CAPITAL_SIN_PALACIO: &str = "capital.sin_palacio";
    /// La plaza ya es la capital (efectiva) de la Facción.
    pub const CAPITAL_YA_ES_LA_CAPITAL: &str = "capital.ya_es_la_capital";
    /// La capital designada sigue en pie y no han pasado `CAPITAL.cooldownDias` desde que se designó.
    pub const CAPITAL_COOLDOWN: &str = "capital.cooldown";
}

/// Lo que un rechazo cuenta además de su código, para que el cliente decida con ello (`RechazoOrden.detalle`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Detalle {
    /// `guerra.confirmar`: la Facción contra la que quedaría en guerra quien ataca y las que entrarían por arrastre (aliados y vasallos; vacía hasta el sistema 5).
    Guerra { objetivo: u32, arrastre: Vec<u32> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resultado {
    Aceptada,
    Rechazada { codigo: &'static str, mensaje: String, detalle: Option<Detalle> },
    /// Válida pero no cambió nada: no sube la versión.
    SinCambios,
    /// Misma `peticion_id` ya aceptada: no se ejecuta otra vez.
    Duplicada,
}

/// Qué es lo que lleva `Respuesta::creadas` (cada orden crea, como mucho, una clase de entidad). Los ids son los del núcleo; el contrato los nombra con `proyeccion::ids`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoCreado {
    Ejercito,
    Asentamiento,
    Heroe,
    Escuadra,
    Caravana,
    Convocatoria,
    CampamentoBandidos,
    Acuerdo,
    OrdenMercado,
    /// Un pueblo extractor recién marcado: mientras está «pendiente de materiales» el contrato lo anuncia como `ObraDeMapa` (`proyeccion::referencia_creada`).
    Pueblo,
    /// Una avanzada recién marcada, igual que el pueblo.
    Avanzada,
    Loadout,
    /// Una parcela de bosque (andamiaje de administración): el contrato no la tiene como entidad, no sale en `ResultadoOrden.creadas`.
    Parcela,
    /// El edificio encolado, con la plaza a la que pertenece (los ids de edificio son de cada plaza).
    Edificio { plaza: u64 },
    /// El recinto comprometido, con su plaza.
    Recinto { plaza: u64 },
    /// Las incorporaciones (por su secuencia) de quien se unió a esa batalla.
    Incorporacion { batalla: u64 },
    /// Una Mirada de la taberna: vive en la memoria de la Facción (la `VISTA` la enseña; no es una entidad del registro de cambios).
    Mirada,
    /// Una Facción nueva (`crearFaccion`).
    Faccion,
}

impl TipoCreado {
    /// Las entidades que la orden toca al crear `ids` (el núcleo las anota en el registro de cambios).
    pub(crate) fn tocadas(self, ids: &[u64]) -> Vec<(Clase, u64)> {
        let de = |c: Clase| ids.iter().map(|&i| (c, i)).collect();
        match self {
            TipoCreado::Ejercito => de(Clase::Ejercito),
            TipoCreado::Asentamiento => de(Clase::Asentamiento),
            TipoCreado::Heroe => de(Clase::Heroe),
            TipoCreado::Escuadra => de(Clase::Escuadra),
            TipoCreado::Caravana => de(Clase::Caravana),
            TipoCreado::Convocatoria => de(Clase::Convocatoria),
            TipoCreado::CampamentoBandidos => de(Clase::Bandidos),
            TipoCreado::Acuerdo => de(Clase::Acuerdo),
            TipoCreado::OrdenMercado => de(Clase::OrdenMercado),
            TipoCreado::Pueblo => de(Clase::Pueblo),
            TipoCreado::Avanzada => de(Clase::Avanzada),
            TipoCreado::Faccion => de(Clase::Faccion),

            TipoCreado::Incorporacion { batalla } => vec![(Clase::Batalla, batalla)],
            TipoCreado::Edificio { plaza } | TipoCreado::Recinto { plaza } => vec![(Clase::Asentamiento, plaza)],
            // El loadout vive en el héroe que la orden nombra y la parcela, en la plaza que la orden nombra: ya van en `Comando::implicados`. La Mirada es de la memoria de la Facción.
            TipoCreado::Loadout | TipoCreado::Parcela | TipoCreado::Mirada => vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Respuesta {
    pub resultado: Resultado,
    /// Versión de mundo con los efectos ya aplicados.
    pub version_tras: u64,
    /// Los ids de lo que creó la orden, de la clase `tipo_creadas`.
    pub creadas: Vec<u64>,
    /// La clase de `creadas`; `None` si no creó nada.
    pub tipo_creadas: Option<TipoCreado>,
}

impl Respuesta {
    pub(crate) fn rechazo(codigo: &'static str, mensaje: impl Into<String>, version: u64) -> Self {
        Self::rechazo_con(codigo, mensaje, version, None)
    }
    pub(crate) fn rechazo_con(codigo: &'static str, mensaje: impl Into<String>, version: u64, detalle: Option<Detalle>) -> Self {
        Self { resultado: Resultado::Rechazada { codigo, mensaje: mensaje.into(), detalle }, version_tras: version, creadas: vec![], tipo_creadas: None }
    }
    pub(crate) fn sin_cambios(version: u64) -> Self {
        Self { resultado: Resultado::SinCambios, version_tras: version, creadas: vec![], tipo_creadas: None }
    }
    pub(crate) fn duplicada(version: u64) -> Self {
        Self { resultado: Resultado::Duplicada, version_tras: version, creadas: vec![], tipo_creadas: None }
    }
    pub(crate) fn aceptada_con(version: u64, creadas: Vec<u64>, tipo: Option<TipoCreado>) -> Self {
        let tipo_creadas = tipo.filter(|_| !creadas.is_empty());
        Self { resultado: Resultado::Aceptada, version_tras: version, creadas, tipo_creadas }
    }
    pub fn aceptada(&self) -> bool {
        self.resultado == Resultado::Aceptada
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use combate::ciclo::{EscuadraDeSalida, Instancia, MotivoSalida};
    use combate::resultado::{Botin, ObjetoBotin, Razon, ResultadoEscuadra, ResultadoHeroe};

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

    #[test]
    fn las_ordenes_de_comercio_dan_la_vuelta_por_el_registro() {
        let comandos = vec![
            Comando::ProponerTrueque { a: 3, b: 4, lineas_a: vec![(Recurso::Madera, 100_000), (Recurso::Oro, 5)], lineas_b: vec![(Recurso::Cobre, 30_000)] },
            Comando::AceptarTrueque { acuerdo: 7 },
            Comando::RechazarTrueque { acuerdo: 7 },
            Comando::ColocarOrdenMercado { asentamiento: 3, venta: true, recurso: Recurso::Piedra, cantidad: 9, precio: Some(-4) },
            Comando::ColocarOrdenMercado { asentamiento: 3, venta: false, recurso: Recurso::Piedra, cantidad: 9, precio: None },
            Comando::CancelarOrdenMercado { orden: 2 },
            Comando::ComerciarEnPlaza { ejercito: 1, orden: 2, cantidad: 3_000 },
            Comando::CrearCaravana { asentamiento: 3 },
            Comando::AnadirCarro { caravana: 1, tipo: TipoCarro::Reforzado },
            Comando::AnadirAnimal { caravana: 1, carro: 0, animal: Animal::Camello },
            Comando::MoverCarro { de: 1, a: 2, carro: 1 },
            Comando::ReservarCaravana { caravana: 1, reservada: true },
            Comando::PrepararCaravana { caravana: 1, destino: 4, carga: vec![(Recurso::Madera, 1_000)], salir_en: Some(77) },
            Comando::PrepararCaravana { caravana: 1, destino: 4, carga: vec![], salir_en: None },
            Comando::CancelarCaravana { caravana: 1 },
            Comando::MoverCargaAparcada { caravana: 1, recurso: Recurso::Trigo, cantidad: 5, cargar: false },
            Comando::EnviarCaravanaAlOrigen { caravana: 1 },
            Comando::EntregarDeCaravana { ejercito: 2, caravana: 1, acuerdo: 3 },
            Comando::DarRecursos { asentamiento: 3, recurso: Recurso::Oro, cantidad: 1_000 },
            Comando::DarSuministro { ejercito: 2, recurso: Recurso::Oro, cantidad: 1_000 },
            Comando::AdjuntarCaravana { caravana: 1, ejercito: 2 },
            Comando::AparcarCaravana { caravana: 1, asentamiento: 3 },
            Comando::ComprarEnCampamento { heroe: 2, campamento: 7, recurso: Recurso::Cobre, cantidad: 5_000 },
            Comando::RepostarEnCampamento { heroe: 2, campamento: 7 },
            Comando::MarcarPueblo { asentamiento: 3, nodo: 70_001 },
            Comando::EntregarMateriales { ejercito: 2, pueblo: 9 },
            Comando::DestruirPueblo { pueblo: 9 },
            Comando::ReponerCarro { pueblo: 9 },
        ];
        for c in comandos {
            let p = Peticion { actor: "ana".into(), admin: true, peticion_id: "y".into(), version_esperada: None, comando: c };
            assert_eq!(Peticion::decodificar(&p.codificar()).unwrap(), p);
        }
        // Un tipo de carro fuera de rango no se decodifica.
        let mut b = Peticion::nueva("a", "i", Comando::AnadirCarro { caravana: 1, tipo: TipoCarro::Basico }).codificar();
        *b.last_mut().unwrap() = 9;
        assert!(Peticion::decodificar(&b).is_err());
    }

    #[test]
    fn las_ordenes_de_la_faccion_y_la_residencia_dan_la_vuelta_por_el_registro() {
        let sigilo = politica::Reglas::default().sigilo.sigilo_libre("f-1", [].iter());
        let comandos = vec![
            Comando::CrearFaccion { heroe: 2, nombre: "Micenas".into(), sigilo: None },
            Comando::CrearFaccion { heroe: 2, nombre: "Troya".into(), sigilo: Some(sigilo) },
            Comando::SolicitarIngreso { heroe: 2, faccion: 3 },
            Comando::ResponderSolicitud { heroe: 2, faccion: 3, solicitante: 9, aceptar: true },
            Comando::ResponderSolicitud { heroe: 2, faccion: 3, solicitante: 9, aceptar: false },
            Comando::DejarFaccion { heroe: 2 },
            Comando::AsignarRey { heroe: 2, faccion: 3, designado: 9 },
            Comando::AsignarEmbajador { heroe: 2, faccion: 3, designado: 9 },
            Comando::CambiarResidencia { heroe: 2, asentamiento: 5 },
            Comando::ResidirEnCampamento { heroe: 2, campamento: 4 },
            Comando::DejarResidencia { heroe: 2 },
        ];
        for c in comandos {
            let p = Peticion { actor: "ana".into(), admin: false, peticion_id: "pol".into(), version_esperada: None, comando: c };
            assert_eq!(Peticion::decodificar(&p.codificar()).unwrap(), p);
        }
        // Aceptar una solicitud no se da mientras el solicitante combate: el candado lo nombra.
        assert_eq!(Comando::ResponderSolicitud { heroe: 2, faccion: 3, solicitante: 9, aceptar: true }.implicados().heroes, vec![2, 9]);
        assert_eq!(Comando::CrearFaccion { heroe: 2, nombre: "x".into(), sigilo: None }.tipo_creado(), Some(TipoCreado::Faccion));
    }

    #[test]
    fn las_ordenes_de_avanzadas_dan_la_vuelta_por_el_registro() {
        let comandos = vec![
            Comando::ColocarAvanzada { asentamiento: 3, heroe: 2, posicion: Punto::new(-5, 77_000) },
            Comando::EntregarMaterialesAAvanzada { ejercito: 9, avanzada: 12 },
            Comando::MejorarAvanzada { heroe: 2, avanzada: 12 },
            Comando::RepararAvanzada { heroe: 2, avanzada: 12 },
            Comando::DestruirAvanzada { heroe: 2, avanzada: 12 },
            Comando::ElegirTropaDeAvanzada { heroe: 2, avanzada: 12, tropa: Tropa::LancerosMimbre },
            Comando::GuarnecerAvanzada { heroe: 2, avanzada: 12, escuadras: vec![5, 6] },
            Comando::RetirarDeAvanzada { heroe: 2, avanzada: 12, escuadras: vec![6] },
            Comando::RepostarEnAvanzada { heroe: 2, avanzada: 12 },
            Comando::DepositarEnAvanzada { ejercito: 9, avanzada: 12, recurso: Recurso::Trigo, cantidad: 40_000 },
            Comando::CancelarObraDeAvanzada { heroe: 2, avanzada: 12 },
        ];
        for c in comandos {
            let p = Peticion { actor: "ana".into(), admin: false, peticion_id: "av".into(), version_esperada: Some(4), comando: c };
            assert_eq!(Peticion::decodificar(&p.codificar()).unwrap(), p);
        }
        // Una tropa fuera del catálogo no se decodifica, y cada orden nombra la avanzada que toca para el candado de las batallas.
        let mut b = Peticion::nueva("a", "i", Comando::ElegirTropaDeAvanzada { heroe: 1, avanzada: 1, tropa: Tropa::Arqueros }).codificar();
        *b.last_mut().unwrap() = 200;
        assert!(Peticion::decodificar(&b).is_err());
        for c in [
            Comando::EntregarMaterialesAAvanzada { ejercito: 9, avanzada: 12 },
            Comando::MejorarAvanzada { heroe: 2, avanzada: 12 },
            Comando::CancelarObraDeAvanzada { heroe: 2, avanzada: 12 },
            Comando::GuarnecerAvanzada { heroe: 2, avanzada: 12, escuadras: vec![] },
            Comando::DepositarEnAvanzada { ejercito: 9, avanzada: 12, recurso: Recurso::Oro, cantidad: 1 },
            Comando::Atacar { heroe: 2, objetivo: ObjetivoAtaque::Avanzada(12), declara_guerra: false },
        ] {
            assert_eq!(c.implicados().avanzadas, vec![12], "{c:?}");
        }
        assert!(Comando::ColocarAvanzada { asentamiento: 3, heroe: 2, posicion: Punto::new(0, 0) }.implicados().avanzadas.is_empty());
    }

    #[test]
    fn las_ordenes_de_heroes_dan_la_vuelta_por_el_registro() {
        let comandos = vec![
            Comando::FijarResidencia { heroe: 2, residencia: Some(Residencia::Campamento(7)) },
            Comando::FijarResidencia { heroe: 2, residencia: Some(Residencia::Asentamiento(3)) },
            Comando::FijarResidencia { heroe: 2, residencia: None },
            Comando::DarAlmacenPersonal { heroe: 2, recurso: Recurso::Oro, cantidad: 9 },
            Comando::CrearHeroe { cuenta: "ana".into(), campamento: 4, controlador: Controlador::Bot },
            Comando::ReclutarTropa { asentamiento: 3, heroe: 2, tropa: Tropa::Espartiatas },
            Comando::ReclutarEnCampamento { heroe: 2, tropa: Tropa::LancerosMimbre, pagar_con: PagarCon::Carro },
            Comando::ReclutarEnCampamento { heroe: 2, tropa: Tropa::MiliciaLanceros, pagar_con: PagarCon::AlmacenPersonal },
            Comando::PedirPrestamo { heroe: 2, tropas: vec![Tropa::Granjeros, Tropa::Mineros] },
            Comando::ReponerPrestamo { heroe: 2 },
            Comando::GuardarLoadout { heroe: 2, loadout: None, nombre: "Asalto".into(), escuadras: vec![5, 6], perks: vec![1, 9], activo: None },
            Comando::GuardarLoadout { heroe: 2, loadout: Some(8), nombre: "Defensa".into(), escuadras: vec![], perks: vec![], activo: Some(true) },
            Comando::GuardarLoadout { heroe: 2, loadout: Some(8), nombre: "Defensa".into(), escuadras: vec![5], perks: vec![], activo: Some(false) },
            Comando::BorrarLoadout { heroe: 2, loadout: 8 },
            Comando::RepartirPuntos { heroe: 2, fuerza: 3, destreza: 0, armadura: 1, vitalidad: -2 },
            Comando::OrdenarEscuadras { heroe: 2, escuadras: vec![6, 5, 4] },
            Comando::AsignarGuarnicion { heroe: 2, escuadra: 5 },
            Comando::RetirarGuarnicion { heroe: 2, escuadra: 5 },
            Comando::PasarAViveres { heroe: 2, cantidad: 30_000 },
            Comando::AsignarEscolta { caravana: 1, heroe: 2, escuadras: vec![5, 6] },
            Comando::QuitarEscolta { caravana: 1, heroe: 2, escuadras: None },
            Comando::QuitarEscolta { caravana: 1, heroe: 2, escuadras: Some(vec![]) },
            Comando::QuitarEscolta { caravana: 1, heroe: 2, escuadras: Some(vec![5]) },
            Comando::Desconectarse { heroe: 2 },
            Comando::Conectarse { heroe: 2 },
            Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["arco_compuesto".into(), "leva_comunal".into()] },
            Comando::FijarFaccionDeHeroe { heroe: 2, faccion: Some(3) },
            Comando::FijarFaccionDeHeroe { heroe: 2, faccion: None },
            Comando::ColocarHeroe { heroe: 2, ubicacion: Ubicacion::Asentamiento(3) },
            Comando::ColocarHeroe { heroe: 2, ubicacion: Ubicacion::Columna(9) },
            Comando::ColocarHeroe { heroe: 2, ubicacion: Ubicacion::Mercenarios(1) },
            Comando::ColocarHeroe { heroe: 2, ubicacion: Ubicacion::Desconectado(Punto::new(5, -7)) },
            Comando::ColocarHeroe { heroe: 2, ubicacion: Ubicacion::Pueblo(6) },
            Comando::ColocarHeroe { heroe: 2, ubicacion: Ubicacion::Avanzada(7) },
            Comando::HerirHeroe { heroe: 2 },
            Comando::SalirAlMundo { asentamiento: 3, heroe: 2, escuadras: vec![5, 6], carga: vec![(Recurso::Madera, 1_000)] },
            Comando::MovilizarEjercito { asentamiento: 3, heroe: 2, escuadras: vec![5], carga: vec![], politica: Politica::Preguntar },
            Comando::SalirDelCampamento { campamento: 4, heroe: 2, escuadras: vec![], carga: vec![(Recurso::Oro, 9)], politica: Politica::Aceptar },
            Comando::EntrarEnAsentamiento { asentamiento: 3, heroe: 2 },
            Comando::SalirDeAsentamiento { asentamiento: 3, heroe: 2 },
            Comando::Guarnecer { asentamiento: 3, heroe: 2 },
            Comando::EntrarEnCampamento { campamento: 4, heroe: 2 },
            Comando::UnirseAEjercito { asentamiento: 3, heroe: 2, ejercito: 9, escuadras: vec![5], carga: vec![(Recurso::Trigo, 7)] },
            Comando::UnirseEnCampo { heroe: 2, ejercito: 9 },
            Comando::ResponderPeticionDeUnion { heroe: 2, ejercito: 9, solicitante: 4, aceptar: true },
            Comando::ResponderPeticionDeUnion { heroe: 2, ejercito: 9, solicitante: 4, aceptar: false },
            Comando::SepararseDelEjercito { heroe: 2 },
            Comando::CederLiderazgo { heroe: 2, sucesor: 3 },
            Comando::OrganizarEjercito { heroe: 2, politica: Politica::Aceptar },
            Comando::CancelarFormacion { heroe: 2 },
            Comando::Repostar { heroe: 2, asentamiento: 3 },
            Comando::ConvocarEjercito { heroe: 2, escuadras: vec![6, 5], carga: vec![], politica: Politica::Preguntar },
            Comando::UnirseAConvocatoria { heroe: 2, convocatoria: 8, escuadras: vec![5], carga: vec![(Recurso::Oro, 1)] },
            Comando::ResponderPeticionDeConvocatoria { heroe: 2, convocatoria: 8, solicitante: 4, aceptar: false },
            Comando::CambiarSeleccionDeConvocatoria { heroe: 2, escuadras: vec![], carga: vec![] },
            Comando::SepararseDeConvocatoria { heroe: 2 },
            Comando::CancelarConvocatoria { heroe: 2 },
            Comando::PartirConvocatoria { heroe: 2 },
            Comando::Perseguir { heroe: 2, objetivo: Objetivo::Ejercito(9) },
            Comando::Perseguir { heroe: 2, objetivo: Objetivo::Caravana(4) },
            Comando::DejarDePerseguir { heroe: 2 },
            Comando::Inspeccionar { heroe: 2, objetivo: Objetivo::Asentamiento(5) },
            Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Vasallaje) },
            Comando::FijarRelacion { a: 3, b: 2, relacion: None },
            Comando::FijarAcceso { asentamiento: 3, aliados: true, neutrales: false, enemigos: true },
            Comando::VaciarCarro { heroe: 2 },
            Comando::EntrarEnPueblo { pueblo: 6, heroe: 2 },
            Comando::SalirDePueblo { pueblo: 6, heroe: 2 },
            Comando::EntrarEnAvanzada { avanzada: 7, heroe: 2 },
            Comando::SalirDeAvanzada { avanzada: 7, heroe: 2 },
            Comando::ComprometerRecinto { asentamiento: 3, cargo: Cargo::MaestroObras, nivel: 2 },
            Comando::MejorarRecinto { asentamiento: 3, cargo: Cargo::Gobernador, recinto: 1 },
            Comando::AbandonarRecinto { asentamiento: 3, recinto: 2 },
            Comando::ComprarMirada { heroe: 2, origen: OrigenIntel::Plaza(3), centro: Punto::new(40_000, -7) },
            Comando::ComprarMirada { heroe: 2, origen: OrigenIntel::Campamento(1), centro: Punto::new(0, 9) },
            Comando::ComprarInformePlaza { heroe: 2, origen: OrigenIntel::Campamento(4), asentamiento: 5 },
            Comando::ComprarInformePlaza { heroe: 2, origen: OrigenIntel::Plaza(3), asentamiento: 6 },
            Comando::Atacar { heroe: 2, objetivo: ObjetivoAtaque::Ejercito(9), declara_guerra: false },
            Comando::Atacar { heroe: 2, objetivo: ObjetivoAtaque::Caravana(4), declara_guerra: true },
            Comando::Atacar { heroe: 2, objetivo: ObjetivoAtaque::Campamento(1), declara_guerra: false },
            Comando::Atacar { heroe: 2, objetivo: ObjetivoAtaque::Asentamiento(5), declara_guerra: true },
            Comando::Atacar { heroe: 2, objetivo: ObjetivoAtaque::Pueblo(6), declara_guerra: false },
            Comando::Atacar { heroe: 2, objetivo: ObjetivoAtaque::Avanzada(7), declara_guerra: false },
            Comando::ColocarBandidos { posicion: Punto::new(-5, 9_000), nivel: 3 },
            Comando::DarEscuadra { heroe: 4, tropa: Tropa::Arqueros, cantidad: 12 },
            Comando::FijarControlador { heroe: 4, controlador: Controlador::Humano },
            Comando::UnirseABatalla { heroe: 4, batalla: 77, lado: None },
            Comando::UnirseABatalla { heroe: 4, batalla: 77, lado: Some(Lado::Atacante) },
            Comando::UnirseABatalla { heroe: 4, batalla: 77, lado: Some(Lado::Defensor) },
            Comando::CancelarBatalla { heroe: 4, batalla: 77 },
            Comando::RegistrarAsignacion {
                batalla: 77,
                asignacion: Asignacion { intento: "a1".into(), servidor: "srv".into(), revision: 2, instancia: Instancia { host: "10.0.0.1".into(), puerto: 7777, protocolo: "udp".into() }, tokens: vec![Token { heroe: 4, token: "t".into(), expira_en: 99 }] },
            },
            Comando::ConfirmarInicio { batalla: 77, intento: "a1".into(), revision: 2 },
            Comando::RegistrarTokens { batalla: 77, intento: "a1".into(), tokens: vec![Token { heroe: 5, token: "u".into(), expira_en: 100 }] },
            Comando::RegistrarSalida {
                batalla: 77,
                salida: SalidaBatalla { intento: "a1".into(), revision: 2, heroe: 4, motivo: MotivoSalida::Eliminado, derrotado: true, escuadras: vec![EscuadraDeSalida { escuadra: 8, supervivientes: 3, muertos: 7 }] },
            },
            Comando::AplicarResultado {
                batalla: 77,
                resultado: ResultadoDeBatalla {
                    resultado_id: "r1".into(),
                    revision: 2,
                    intento: "a1".into(),
                    inicio: 10,
                    fin: 90,
                    ganador: Lado::Defensor,
                    razon: Razon::TiempoAgotado,
                    objetivos: vec![("bandera".into(), None), ("torre".into(), Some(Lado::Atacante))],
                    por_escuadra: vec![ResultadoEscuadra { escuadra: 8, desplegados: 10, supervivientes_al_cierre: 3, muertos: 7, xp_ganada: 4 }],
                    por_heroe: vec![
                        ResultadoHeroe { heroe: 4, participo: true, sobrevivio_al_cierre: false, xp_ganada: 9, derrotado: Some(true), botin: None },
                        ResultadoHeroe {
                            heroe: 5,
                            participo: true,
                            sobrevivio_al_cierre: true,
                            xp_ganada: 1,
                            derrotado: None,
                            botin: Some(Botin { objetos: vec![ObjetoBotin { definicion: "espada".into(), cantidad: 1, instancia: Some("i-1".into()) }, ObjetoBotin { definicion: "oro".into(), cantidad: 5, instancia: None }], monedas: militar::heroe::Monedas { bronce: 1, plata: 2, oro: 3 } }),
                        },
                        ResultadoHeroe { heroe: 6, participo: false, sobrevivio_al_cierre: true, xp_ganada: 0, derrotado: Some(false), botin: None },
                    ],
                    version_servidor: "v1".into(),
                    carros_perdidos: vec!["0".into(), "2".into()],
                },
            },
        ];
        for c in comandos {
            let p = Peticion { actor: "ana".into(), admin: false, peticion_id: "z".into(), version_esperada: Some(3), comando: c };
            assert_eq!(Peticion::decodificar(&p.codificar()).unwrap(), p);
        }
        // Una tropa fuera del catálogo, una forma de pago o un controlador desconocidos no se decodifican.
        let mut b = Peticion::nueva("a", "i", Comando::ReclutarTropa { asentamiento: 1, heroe: 1, tropa: Tropa::Arqueros }).codificar();
        *b.last_mut().unwrap() = 200;
        assert!(Peticion::decodificar(&b).is_err());
        let mut b = Peticion::nueva("a", "i", Comando::ReclutarEnCampamento { heroe: 1, tropa: Tropa::Arqueros, pagar_con: PagarCon::Carro }).codificar();
        *b.last_mut().unwrap() = 7;
        assert!(Peticion::decodificar(&b).is_err());
        let mut b = Peticion::nueva("a", "i", Comando::CrearHeroe { cuenta: "a".into(), campamento: 0, controlador: Controlador::Humano }).codificar();
        *b.last_mut().unwrap() = 7;
        assert!(Peticion::decodificar(&b).is_err());
        // Un lado de batalla desconocido tampoco.
        let mut b = Peticion::nueva("a", "i", Comando::UnirseABatalla { heroe: 1, batalla: 1, lado: None }).codificar();
        *b.last_mut().unwrap() = 9;
        assert!(Peticion::decodificar(&b).is_err());
        // Un objetivo de ataque desconocido tampoco.
        let mut b = Peticion::nueva("a", "i", Comando::Atacar { heroe: 1, objetivo: ObjetivoAtaque::Pueblo(3), declara_guerra: false }).codificar();
        let n = b.len();
        b[n - 10] = 9;
        assert!(Peticion::decodificar(&b).is_err());
    }
}
