//! La población sintética del sistema «militar y combate» (fase 4G): Facciones de tres plazas fundadas a la vez sobre el mundo real, cada plaza con cuatro
//! héroes residentes que son bots mínimos y deterministas. Los bots solo dan órdenes al núcleo (`Comando`, con la cuenta de su héroe); leen el estado para
//! decidir pero no lo tocan. Sin servidores de batalla: todo combate se resuelve con números. Un planificador propio (cola de acciones por instante) avanza el
//! núcleo con `avanzar_hasta` y aplica cada orden con `aplicar_en`; los hechos se leen del registro de cambios cada media hora de mundo.
//!
//! # Los bots (hipótesis del batch, deterministas por semilla)
//! Cada plaza tiene un **Guardián**, un **Capitán**, un **Cazador** y un **Intendente** (qué cuenta hace cada papel lo sortea la semilla). Cada 15 minutos de mundo
//! cada héroe mira dónde está y qué tiene a mano:
//! - **En casa, todos** reclutan o reponen una escuadra por hora (la mejor tropa que la plaza sabe hacer y que cabe en su Liderazgo, hasta 4, 4, 2 y 1 escuadras
//!   según el papel; mientras la plaza no llega al nivel 2, una por héroe y nada que la deje por debajo de los pesants del ascenso), dejan su loadout activo con
//!   lo que tienen en el campamento (si la plaza es atacada, defienden en persona) y vacían en su almacén personal el carro que traen.
//! - **El Guardián** no sale nunca: gobierna (pide el ascenso, construye el Mercado, el Barracón y la Galería de tiro, monta la flota de dos caravanas, marca un
//!   pueblo extractor y compromete una empalizada) y pone su leva en la guarnición. En una plaza sin Guardián (una conquistada) gobierna cualquier residente.
//! - **El Capitán** convoca un ejército en la plaza (o sale directamente y los demás se le unen en la puerta) y va a por lo que tiene más cerca: un ejército
//!   enemigo a la vista (lo persigue), una plaza enemiga (tras inspeccionarla, si su poder supera con margen la defensa que le supone), un pueblo con algo que
//!   saquear, una avanzada o el campamento de bandidos de casa (cada 12 h), en un orden que sortea la semilla. Ataca con `declara_guerra` cuando hace falta. Si
//!   conquista, se queda a residir en la plaza ganada (si no deja la suya con menos de dos residentes).
//! - **El Cazador** sale con su columna personal a explorar, alrededor de casa o de la plaza enemiga más cercana; persigue columnas personales y caravanas
//!   enemigas que ve y las ataca en el instante en que las tiene a tiro, y a veces ataca el campamento de bandidos de casa. Uno de cada cuatro es **temerario**:
//!   una vez al día sale solo, sin tropa, contra ese campamento.
//! - **El Intendente** lleva a la obra los materiales del pueblo y de la atalaya (los saca del almacén de su plaza al salir) y acompaña a las caravanas: o
//!   cede una escuadra como escolta y la manda a una plaza hermana, o sale como ejército con la caravana adjunta, la aparca allí y la manda de vuelta.
//! - **Fuera**, quien manda la columna vuelve a casa con los víveres por debajo del 35 % (antes pasa a víveres el trigo del carro), tras tres objetivos o si lo
//!   hieren; en la puerta reposta o entra (`Guarnecer` / `EntrarEnAsentamiento`). Si la Facción pierde todas sus plazas, el héroe vuelve a su campamento.
//! - Solo las Facciones **belicosas** atacan a otras Facciones sin estar ya en guerra; las pacíficas atacan bandidos y responden a quien les declaró la guerra.
//!   Las Facciones van por parejas aliadas: a un aliado no se le ataca.
//!
//! # Andamiaje de administración (provisional hasta los sistemas 5 y 6)
//! La Facción de nivel 8 (`FijarNivelFaccion`), las tecnologías de la Era I (`FijarTecnologias`), las alianzas (`FijarRelacion`), y el héroe de cada papel:
//! ciudadano de su Facción (`FijarFaccionDeHeroe`), residente en su plaza (`FijarResidencia`, también al quedarse en una plaza conquistada) y dentro de ella
//! (`ColocarHeroe`). La columna de fundación entra en su plaza tras fundar. Nada más: el oro, el trigo, los materiales y la tropa salen de las plazas.
//!
//! # Lo que comprueba
//! Cada `Atacar` aceptado pasa por [`auditmil`] (bajas, experiencia, heridos, conservación, carros, botín, héroe solo, cascadas de la conquista); cada media hora,
//! los invariantes militares de [`ViolacionesMilitares`] (experiencia, hambre, heridos, batallas, reposiciones, ruinas) y cada 6 horas los del comercio y la
//! economía (`auditoria`). Con `foto`, a mitad de la corrida el núcleo se reconstruye desde su foto `BAN31` y se sigue con el reconstruido.

use crate::auditmil::{self, Antes, ViolacionesCombate};
use crate::auditoria::{auditar_cola, auditar_contenedores, Violaciones};
use crate::geo::{Cargado, Geo};
use crate::sim::{ANILLO_CAMPAMENTO, ANILLO_PLAZA, VELOCIDAD, VIVERES};
use crate::simcom::{sitio, Cuenta};
use asentamiento::catalogo::Tipo;
use asentamiento::{Cargo, Recurso};
use base::{Instante, Punto, Rng, MS_POR_MINUTO};
use comercio::caravana::{Animal, EstadoCaravana, TipoCaravana, TipoCarro};
use militar::catalogo::{Tropa, TROPAS};
use militar::columna::{Politica, TipoColumna};
use militar::convocatoria::Lugar;
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::inspeccion::{Contenido, Objetivo};
use militar::presencia::Ubicacion;
use nucleo::{Ajustes, Arranque, Carga, Comando, Nucleo, ObjetivoAtaque, Peticion, Reglas, Relacion, Resultado, SucesoMilitar};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;
const DIA: i64 = 24 * HORA;

// ── Supuestos del batch (hipótesis sintéticas, se anotan en `parametros` del informe) ──
pub const PLAZAS_POR_FACCION: u32 = 3;
pub const NIVEL_FACCION: u32 = 8;
pub const TURNO_MIN: i64 = 15;
pub const OBSERVAR_MIN: i64 = 30;
pub const BELICOSA_PCT: u64 = 60;
pub const CONVOCA_PCT: u64 = 50;
pub const SE_UNE_PCT: u64 = 50;
pub const TEMERARIO_PCT: u64 = 25;
/// Por debajo de este porcentaje de víveres la columna vuelve a casa.
pub const VOLVER_VIVERES_PCT: i64 = 35;
pub const OBJETIVOS_POR_SALIDA: u32 = 3;
/// Radio (milésimas) en el que se buscan objetivos y se explora.
pub const RADIO_CAMPANA: i64 = 160_000;
pub const RADIO_EXPLORACION: i64 = 120_000;
/// El ejército ataca una plaza si su poder es al menos este porcentaje de la defensa que le supone tras inspeccionarla (`defensa_supuesta`).
pub const MARGEN_ASEDIO_PCT: i64 = 110;
/// Los bandidos se cazan si su campamento está a esta distancia de casa como mucho, y cada héroe no vuelve a por ellos antes de estas horas.
pub const RADIO_BANDIDOS: i64 = 60_000;
pub const BANDIDOS_CADA_H: i64 = 12;
/// Tropas que se intentan reclutar en cada turno de reclutamiento, de la mejor a la peor.
pub const INTENTOS_DE_RECLUTAR: usize = 3;
/// Escuadras que recluta cada papel (Guardián, Capitán, Cazador, Intendente).
pub const ESCUADRAS_POR_PAPEL: [usize; 4] = [4, 4, 2, 1];
/// Cada cuánto se mira si la presa está a tiro, y cuánto tarda en volver a por una plaza que se descartó asediar.
pub const PASO_ALCANCE: i64 = 2_000;
pub const DESCARTE_H: i64 = 6;
/// Un pueblo se saquea si guarda al menos esto por encima de la mitad de su almacén.
pub const SAQUEO_MINIMO: i64 = 50_000;
/// Descanso en casa entre salidas, y cada cuánto sale una caravana.
pub const DESCANSO_MIN: i64 = 90;
pub const CARAVANA_CADA_H: i64 = 4;
/// Trigo que el ejército saca de la plaza en su carro para pasarlo a víveres en campaña, y carga de cada caravana.
pub const TRIGO_DE_CAMPANA: i64 = 60_000;
pub const CARGA_DE_CARAVANA: i64 = 30_000;
/// La Era I (el sistema 6 las traerá por logros): las tropas de Barracón, Galería de tiro y Caballerizas que una plaza de nivel 2 puede hacer.
pub const TECNOLOGIAS: [&str; 8] = ["metalurgia_cobre", "aleacion_bronce", "escudos_ligeros", "armamento_palacial", "arqueria_palacial", "cria_caballar", "carros_guerra", "canteria"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rol {
    Guardian,
    Capitan,
    Cazador,
    Intendente,
}

impl Rol {
    pub fn nombre(self) -> &'static str {
        match self {
            Rol::Guardian => "guardián",
            Rol::Capitan => "capitán",
            Rol::Cazador => "cazador",
            Rol::Intendente => "intendente",
        }
    }
}

/// Lo que se hace al llegar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Meta {
    Atacar(ObjetivoAtaque),
    Explorar,
    Pueblo(u64),
    Avanzada(Punto),
    Aparcar { caravana: u64, plaza: u64 },
    Casa,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tarea {
    Libre,
    Ir(Meta),
    Persiguiendo(Objetivo),
}

/// Un héroe y su bot.
#[derive(Clone, Debug)]
pub struct Bot {
    pub heroe: u64,
    pub cuenta: String,
    pub faccion: u32,
    /// La plaza donde nació (la de verdad es su residencia, que cambia si la conquistan).
    pub plaza: u64,
    pub rol: Rol,
    pub temerario: bool,
    tarea: Tarea,
    /// Invalida las citas de llegada viejas.
    cita: u32,
    objetivos: u32,
    /// La convocatoria que abrió y desde cuándo.
    convocatoria: Option<(u64, Instante)>,
    proxima_salida: Instante,
    ultimo_temerario: Instante,
    ultimo_bandido: Instante,
    proxima_caravana: Instante,
    proximo_reclutamiento: Instante,
    adjunta: bool,
    /// Las plazas que descartó asediar tras inspeccionarlas y hasta cuándo no vuelve a por ellas.
    descartadas: BTreeMap<u64, Instante>,
}

#[derive(Clone, Copy, Debug)]
enum Accion {
    Turno { bot: usize, cita: Option<u32> },
    Observar,
    Foto,
    /// Un milisegundo antes de que acabe la convocatoria de un asedio: la foto de antes del combate.
    AntesDelAsedio { batalla: u64 },
    /// Acabó la convocatoria y el asedio se resolvió con números: se audita como una orden de ataque.
    FinDelAsedio { batalla: u64 },
}

/// Un asedio abierto que espera el fin de su convocatoria: quién lo abrió, con qué columna y contra qué plaza, y la foto de antes del combate.
struct AsedioAbierto {
    bot: usize,
    columna: u64,
    plaza: u64,
    antes: Option<Antes>,
}

/// Un combate resuelto por una orden (o un acoso de bandidos): su tipo, quién ganó y los dos poderes (milésimas).
#[derive(Clone, Copy, Debug)]
pub struct Combate {
    pub tipo: &'static str,
    pub gana_atacante: bool,
    pub poder_a: i64,
    pub poder_d: i64,
    pub t: Instante,
}

pub const TIPOS: [&str; 7] = ["columna", "caravana", "campamento", "asedio", "pueblo", "avanzada", "acoso"];

/// Los invariantes que se comprueban fuera de las órdenes de ataque (todo debe quedar a 0).
#[derive(Clone, Copy, Debug, Default)]
pub struct ViolacionesMilitares {
    /// Una escuadra o un héroe con menos experiencia o nivel que en la observación anterior.
    pub experiencia: u64,
    /// Una columna sin víveres y con soldados sin su cita de deserción, una con víveres que la tiene, o una deserción con víveres.
    pub hambre: u64,
    /// Un héroe herido más allá de `heridoMinutos` desde ahora.
    pub herido: u64,
    /// Una batalla, o algo apartado por una batalla (sin servidores de batalla no debe haber ninguna).
    pub batalla: u64,
    /// La guarnición de un pueblo que se completó a deshora (un minero cada 12 min desde la primera baja) o una avanzada que no se repuso a las 6 h.
    pub reposicion: u64,
    /// Una plaza en ruinas de la que queda flota, residentes o quien esté dentro.
    pub ruina: u64,
    /// El hash tras la foto (`BAN31`) ida y vuelta no es el de antes.
    pub foto: u64,
}

/// Lo que se cuenta a lo largo de la corrida.
#[derive(Clone, Debug, Default)]
pub struct Medidas {
    pub combates: Vec<Combate>,
    pub bajas_por_tropa: BTreeMap<Tropa, i64>,
    pub desertores_por_tropa: BTreeMap<Tropa, i64>,
    pub desertores_columna: i64,
    pub desertores_guarnicion: i64,
    pub heridos: u64,
    pub ms_herido: i64,
    /// Caravanas atacadas por bots: (modalidad aguante?, perdida entera?, carros perdidos), y las que resistieron.
    pub asaltos: Vec<(bool, bool, i64)>,
    pub asaltos_resistidos: [u64; 2],
    pub carros_perdidos_adjuntas: i64,
    pub saqueado_de_pueblos: i64,
    /// Oro de bandidos que cobró cada héroe que combatió (milésimas).
    pub oro_por_heroe: Vec<i64>,
    /// Conquistas: (instante, plaza, facción vieja, facción nueva, muralla dañada).
    pub conquistas: Vec<(Instante, u64, u32, u32, bool)>,
    pub ruinas: Vec<(Instante, u64)>,
    /// Plaza → instante en que una conquista le dañó la muralla, y cuándo volvió a estar completa.
    pub murallas_danadas: BTreeMap<u64, (Instante, Option<Instante>)>,
    /// Pueblo → sus reposiciones, en orden, cada una con el instante de su primera baja y los mineros perdidos en ella (la última puede seguir abierta; las demás
    /// esperan a que se lea su hecho), y cuánto tardó cada reposición completa.
    reponiendo: BTreeMap<u64, VecDeque<(Instante, i64)>>,
    pub reposiciones_pueblo: Vec<i64>,
    /// Avanzada → lo mismo que `reponiendo`, con los soldados de su guarnición de serie (también vuelven de uno en uno).
    reponiendo_avanzada: BTreeMap<u64, VecDeque<(Instante, i64)>>,
    pub reposiciones_avanzada: Vec<i64>,
    /// Lo que cada clase de orden sacó de los almacenes de las plazas, por recurso (milésimas).
    pub gasto: BTreeMap<(&'static str, Recurso), i64>,
    /// Trigo que pasó del carro a los víveres.
    pub trigo_a_viveres: i64,
    pub chatarra_generada: i64,
    pub asedios_descartados: u64,
    /// Asedios abiertos (la orden aceptada), columnas que se unieron a su convocatoria y conquistas con más de una columna.
    pub asedios_abiertos: u64,
    pub uniones_a_asedios: u64,
    pub conquistas_con_convocatoria: u64,
    /// Ataques de un héroe sin soldados a un campamento de bandidos.
    pub ataques_solos: u64,
    pub persecuciones_sin_alcance: u64,
    pub reclutados_por_tropa: BTreeMap<Tropa, i64>,
    pub hechos: BTreeMap<String, u64>,
}

/// Una foto diaria.
#[derive(Clone, Debug, Default)]
pub struct Muestra {
    pub dia: u32,
    pub soldados: i64,
    pub en_columna: i64,
    pub en_guarnicion: i64,
    pub columnas_en_el_mapa: usize,
    pub plazas: usize,
    pub facciones_vivas: usize,
    pub guerras: usize,
    pub heroes_exiliados: usize,
    pub bandidos: usize,
}

pub struct Corrida {
    pub nucleo: Nucleo,
    pub bots: Vec<Bot>,
    pub facciones: u32,
    pub belicosas: BTreeSet<u32>,
    pub plazas: Vec<u64>,
    pub ordenes: BTreeMap<&'static str, Cuenta>,
    pub medidas: Medidas,
    pub muestras: Vec<Muestra>,
    pub violaciones: Violaciones,
    pub combate: ViolacionesCombate,
    pub militares: ViolacionesMilitares,
    /// Órdenes de los bots que tenían que aceptarse y se rechazaron, por (orden, código): una anomalía del bot o del núcleo, no una regla del juego (`inv_rechazos`).
    pub inesperadas: BTreeMap<(&'static str, &'static str), u64>,
    /// Hash del núcleo antes y después de la foto ida y vuelta, si se hizo.
    pub foto: Option<([u8; 32], [u8; 32])>,
    pub pared: Duration,
}

struct Sim<'a> {
    n: Nucleo,
    c: &'a Cargado,
    geo: &'a Geo,
    semilla: u64,
    rng: Rng,
    fin: Instante,
    cola: BinaryHeap<Reverse<(Instante, usize)>>,
    acciones: Vec<Accion>,
    bots: Vec<Bot>,
    /// Plaza → los bots que nacieron en ella.
    de_plaza: BTreeMap<u64, Vec<usize>>,
    facciones: u32,
    belicosas: BTreeSet<u32>,
    visto: u64,
    ordenes: BTreeMap<&'static str, Cuenta>,
    medidas: Medidas,
    muestras: Vec<Muestra>,
    contador: u64,
    violaciones: Violaciones,
    combate: ViolacionesCombate,
    militares: ViolacionesMilitares,
    inesperadas: BTreeMap<(&'static str, &'static str), u64>,
    /// Experiencia y nivel de cada escuadra y experiencia de cada héroe en la observación anterior.
    experiencia: BTreeMap<u64, (i64, u8)>,
    experiencia_heroe: BTreeMap<u64, i64>,
    foto: Option<([u8; 32], [u8; 32])>,
    dir_foto: Option<PathBuf>,
    /// Batalla → el asedio con números que espera el fin de su convocatoria.
    asedios: BTreeMap<u64, AsedioAbierto>,
}

/// El arranque del núcleo del batch militar: las reglas de siempre (con los bandidos), sin fotos automáticas.
fn arranque(c: &Cargado, semilla: u64) -> Arranque {
    Arranque {
        reglas: Reglas::default(),
        ajustes: Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() },
        semilla,
        inicio: 0,
        rutas: c.rutas.clone(),
        mundo_hash: c.mundo.hash().into(),
        mundo: Some(c.mundo.clone()),
    }
}

/// Corre `plazas` plazas (en Facciones de `PLAZAS_POR_FACCION`) durante `dias` días de mundo. Con `foto`, a mitad de la corrida se serializa el núcleo, se
/// reconstruye desde esa foto por la persistencia y se sigue con el reconstruido.
pub fn simular(c: &Cargado, geo: &Geo, plazas: u32, dias: u32, semilla: u64, foto: bool) -> Corrida {
    let t0 = Instant::now();
    let mut s = Sim {
        n: Nucleo::nuevo(arranque(c, semilla)),
        c,
        geo,
        semilla,
        rng: Rng::derivado(semilla, "batch-militar"),
        fin: dias as i64 * DIA,
        cola: BinaryHeap::new(),
        acciones: Vec::new(),
        bots: Vec::new(),
        de_plaza: BTreeMap::new(),
        facciones: 0,
        belicosas: BTreeSet::new(),
        visto: 0,
        ordenes: BTreeMap::new(),
        medidas: Medidas::default(),
        muestras: Vec::new(),
        contador: 0,
        violaciones: Violaciones::default(),
        combate: ViolacionesCombate::default(),
        militares: ViolacionesMilitares::default(),
        inesperadas: BTreeMap::new(),
        experiencia: BTreeMap::new(),
        experiencia_heroe: BTreeMap::new(),
        foto: None,
        dir_foto: None,
        asedios: BTreeMap::new(),
    };
    s.fundar_todas(plazas);
    s.arrancar(foto);
    s.correr();
    if let Some(dir) = s.dir_foto.take() {
        let _ = s.n.apagar();
        let _ = std::fs::remove_dir_all(dir);
    }
    Corrida {
        plazas: s.de_plaza.keys().copied().collect(),
        nucleo: s.n,
        bots: s.bots,
        facciones: s.facciones,
        belicosas: s.belicosas,
        ordenes: s.ordenes,
        medidas: s.medidas,
        muestras: s.muestras,
        violaciones: s.violaciones,
        combate: s.combate,
        militares: s.militares,
        inesperadas: s.inesperadas,
        foto: s.foto,
        pared: t0.elapsed(),
    }
}

fn jugador(f: u32) -> String {
    format!("f{f}")
}

impl Sim<'_> {
    fn programar(&mut self, t: Instante, a: Accion) {
        if t <= self.fin {
            self.acciones.push(a);
            self.cola.push(Reverse((t, self.acciones.len() - 1)));
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Aplica una orden y la anota en las cuentas de su tipo.
    fn orden(&mut self, t: Instante, actor: &str, etiqueta: &'static str, c: Comando) -> nucleo::Respuesta {
        self.contador += 1;
        let p = Peticion::nueva(actor, &format!("m{}", self.contador), c);
        let p = if actor == "admin" { p.de_admin() } else { p };
        let r = self.n.aplicar_en(t, p);
        let cuenta = self.ordenes.entry(etiqueta).or_default();
        match &r.resultado {
            Resultado::Aceptada => cuenta.aceptadas += 1,
            Resultado::SinCambios => cuenta.sin_cambios += 1,
            Resultado::Rechazada { codigo, .. } => *cuenta.rechazos.entry(codigo).or_insert(0) += 1,
            Resultado::Duplicada => unreachable!("los ids de petición no se repiten"),
        }
        r
    }

    fn admin(&mut self, t: Instante, etiqueta: &'static str, c: Comando) -> nucleo::Respuesta {
        self.orden(t, "admin", etiqueta, c)
    }

    /// Una orden del héroe del bot `i`.
    fn suya(&mut self, t: Instante, i: usize, etiqueta: &'static str, c: Comando) -> nucleo::Respuesta {
        let cuenta = self.bots[i].cuenta.clone();
        self.orden(t, &cuenta, etiqueta, c)
    }

    /// Una orden del héroe del bot `i` que tiene que aceptarse (el bot ya comprobó lo que exige): si se rechaza se anota en `inesperadas`, que el informe cuenta como invariante incumplido.
    fn esperada(&mut self, t: Instante, i: usize, etiqueta: &'static str, c: Comando) -> nucleo::Respuesta {
        let r = self.suya(t, i, etiqueta, c);
        if let Resultado::Rechazada { codigo, .. } = &r.resultado {
            *self.inesperadas.entry((etiqueta, *codigo)).or_insert(0) += 1;
        }
        r
    }

    /// Una orden que saca del almacén de la plaza: lo que sacó se anota como gasto de su clase.
    fn con_gasto(&mut self, t: Instante, i: usize, plaza: u64, etiqueta: &'static str, c: Comando) -> nucleo::Respuesta {
        let antes = self.almacen(plaza);
        let r = self.suya(t, i, etiqueta, c);
        if r.aceptada() {
            let despues = self.almacen(plaza);
            for (k, (&a, &d)) in antes.iter().zip(despues.iter()).enumerate() {
                if a > d {
                    *self.medidas.gasto.entry((etiqueta, asentamiento::recurso::TODOS[k])).or_insert(0) += a - d;
                }
            }
        }
        r
    }

    fn almacen(&self, plaza: u64) -> [i64; 29] {
        self.n.estado().asentamientos.get(&plaza).map_or([0; 29], |a| *a.economia.almacen.partes().0)
    }

    // ───────────────────────────── Fundación ─────────────────────────────

    /// Funda `plazas` plazas en Facciones de `PLAZAS_POR_FACCION` (la primera en el anillo de un campamento de mercenarios, las demás junto a las suyas) y en
    /// cada una sus cuatro héroes residentes. Las Facciones van por parejas aliadas; la semilla decide cuáles son belicosas y qué cuenta hace cada papel.
    fn fundar_todas(&mut self, plazas: u32) {
        self.facciones = plazas.div_ceil(PLAZAS_POR_FACCION);
        let mut propias: Vec<Vec<u64>> = vec![Vec::new(); self.facciones as usize];
        let tecnologias: Vec<String> = TECNOLOGIAS.iter().map(|t| t.to_string()).collect();
        for f in 1..=self.facciones {
            self.admin(0, "FijarNivelFaccion", Comando::FijarNivelFaccion { faccion: f, nivel: NIVEL_FACCION });
            self.admin(0, "FijarTecnologias", Comando::FijarTecnologias { faccion: f, tecnologias: tecnologias.clone() });
            if self.rng.por_debajo(100) < BELICOSA_PCT {
                self.belicosas.insert(f);
            }
        }
        let mut minuto = 0;
        for k in 0..plazas {
            let f = k % self.facciones + 1;
            let suyas = propias[f as usize - 1].clone();
            let sitio = if suyas.is_empty() {
                let centro = self.geo.campamentos[f as usize % self.geo.campamentos.len()];
                sitio(&self.n, &mut self.rng, f, centro, ANILLO_CAMPAMENTO)
            } else {
                let madre = suyas[self.rng.por_debajo(suyas.len() as u64) as usize];
                let centro = self.n.estado().asentamientos[&madre].posicion;
                sitio(&self.n, &mut self.rng, f, centro, ANILLO_PLAZA)
            };
            let Some(p) = sitio else { continue };
            minuto += 1;
            let t = minuto * MIN;
            self.n.avanzar_hasta(t);
            let Ok(columna) = self.n.crear_ejercito(t, &jugador(f), f, p, VELOCIDAD, VIVERES) else { continue };
            self.admin(t, "DarCaravanaFundacion", Comando::DarCaravanaFundacion { ejercito: columna });
            let r = self.orden(t, &jugador(f), "Fundar", Comando::Fundar { ejercito: columna });
            let Some(&id) = r.creadas.first().filter(|_| r.aceptada()) else { continue };
            // La columna de fundación entra en su plaza: su héroe de campo no es ningún bot.
            let lider = self.n.ejercito(columna).map(|e| e.columna.lider).unwrap_or(0);
            self.orden(t, &jugador(f), "Guarnecer", Comando::Guarnecer { asentamiento: id, heroe: lider });
            propias[f as usize - 1].push(id);
            self.nacen_heroes(t, f, k, id, p);
        }
        self.aliar_parejas(minuto * MIN);
    }

    /// Las parejas aliadas (5D): ya no las fija la administración, sino las reglas reales. El Rey de la impar le propone la alianza al de la par, que la acepta. El Rey de cada Facción es el
    /// héroe de campo de su primera fundación (el primer ciudadano). Una Facción sin plazas (sin Rey) se queda sin aliada.
    fn aliar_parejas(&mut self, t: Instante) {
        let rey = |n: &Nucleo, f: u32| n.faccion(f).and_then(|x| x.rey).and_then(|h| n.heroe(h).map(|x| (h, x.cuenta.clone())));
        for f in (2..=self.facciones).step_by(2) {
            let (Some((ra, ca)), Some((rb, cb))) = (rey(&self.n, f - 1), rey(&self.n, f)) else { continue };
            let r = self.orden(t, &ca, "ProponerRelacion", Comando::ProponerRelacion { heroe: ra, a: f - 1, b: f, vasallaje: false, tributo: None });
            if let Some(&propuesta) = r.creadas.first().filter(|_| r.aceptada()) {
                self.orden(t, &cb, "ResponderRelacion", Comando::ResponderRelacion { heroe: rb, propuesta, aceptar: true });
            }
        }
    }

    fn nacen_heroes(&mut self, t: Instante, f: u32, k: u32, plaza: u64, p: Punto) {
        let campamento = self.geo.campamentos.iter().enumerate().min_by_key(|(_, c)| p.dist(**c)).map_or(0, |(i, _)| i as u32);
        let mut roles = vec![Rol::Guardian, Rol::Capitan, Rol::Cazador, Rol::Intendente];
        for j in (1..roles.len()).rev() {
            roles.swap(j, self.rng.por_debajo(j as u64 + 1) as usize);
        }
        for (j, rol) in roles.into_iter().enumerate() {
            let cuenta = format!("{}-p{k}-h{j}", jugador(f));
            let r = self.orden(t, &cuenta, "CrearHeroe", Comando::CrearHeroe { cuenta: cuenta.clone(), campamento, controlador: Controlador::Humano });
            let Some(&heroe) = r.creadas.first().filter(|_| r.aceptada()) else { continue };
            self.admin(t, "FijarFaccionDeHeroe", Comando::FijarFaccionDeHeroe { heroe, faccion: Some(f) });
            self.admin(t, "FijarResidencia", Comando::FijarResidencia { heroe, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
            self.admin(t, "ColocarHeroe", Comando::ColocarHeroe { heroe, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
            let temerario = rol == Rol::Cazador && self.rng.por_debajo(100) < TEMERARIO_PCT;
            self.bots.push(Bot {
                heroe,
                cuenta,
                faccion: f,
                plaza,
                rol,
                temerario,
                tarea: Tarea::Libre,
                cita: 0,
                objetivos: 0,
                convocatoria: None,
                proxima_salida: 0,
                ultimo_temerario: -DIA,
                ultimo_bandido: -DIA,
                proxima_caravana: 0,
                proximo_reclutamiento: 0,
                adjunta: false,
                descartadas: BTreeMap::new(),
            });
            self.de_plaza.entry(plaza).or_default().push(self.bots.len() - 1);
        }
    }

    // ───────────────────────────── Bucle ─────────────────────────────

    fn arrancar(&mut self, foto: bool) {
        let t0 = self.n.ahora();
        for i in 0..self.bots.len() {
            // Los turnos se reparten por los minutos del cuarto de hora para no apilar a todos en el mismo instante.
            self.programar(t0 + (i as i64 % TURNO_MIN + 1) * MIN, Accion::Turno { bot: i, cita: None });
        }
        self.programar(t0 + OBSERVAR_MIN * MIN, Accion::Observar);
        if foto {
            self.programar(self.fin / 2, Accion::Foto);
        }
    }

    fn correr(&mut self) {
        while let Some(Reverse((t, i))) = self.cola.pop() {
            self.n.avanzar_hasta(t);
            match self.acciones[i] {
                Accion::Turno { bot, cita } => {
                    if cita.is_none() {
                        self.programar(t + TURNO_MIN * MIN, Accion::Turno { bot, cita: None });
                    }
                    if cita.is_none_or(|c| c == self.bots[bot].cita) {
                        self.turno(t, bot);
                    }
                }
                Accion::Observar => self.observar(t),
                Accion::Foto => self.ida_y_vuelta(),
                Accion::AntesDelAsedio { batalla } => {
                    if let Some(a) = self.asedios.get_mut(&batalla) {
                        a.antes = Some(Antes::de(&self.n, a.columna, ObjetivoAtaque::Asentamiento(a.plaza)));
                    }
                }
                Accion::FinDelAsedio { batalla } => self.fin_del_asedio(t, batalla),
            }
        }
        self.n.avanzar_hasta(self.fin);
        self.leer();
        self.muestrear(self.fin / DIA);
        self.comprobar(self.fin);
    }

    /// La foto `BAN31` ida y vuelta: se serializa el núcleo, se escribe como foto en un directorio de persistencia, se reconstruye desde él y se sigue con el
    /// reconstruido (que escribe su registro en ese directorio hasta el final). El hash ha de ser el mismo.
    fn ida_y_vuelta(&mut self) {
        self.leer();
        let antes = self.n.hash_estado();
        // Un directorio propio de cada foto: dos corridas a la vez en el mismo proceso (las pruebas) no se pisan.
        static FOTOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let k = FOTOS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("batch-militar-{}-{k}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let rehecho = (|| -> std::io::Result<Nucleo> {
            let almacen = persistencia::Almacen::abrir(&dir, persistencia::Config::default())?;
            almacen.fotos().guardar(&persistencia::Foto::nueva(0, self.n.ahora(), self.n.estado().serializar()))?;
            drop(almacen);
            Nucleo::con_persistencia(arranque(self.c, self.semilla), &dir, persistencia::Config::default())
        })();
        let Ok(n) = rehecho else {
            self.militares.foto += 1;
            return;
        };
        let despues = n.hash_estado();
        self.militares.foto += (antes != despues) as u64;
        self.foto = Some((antes, despues));
        self.visto = n.version();
        self.n = n;
        self.dir_foto = Some(dir);
    }

    // ───────────────────────────── Lo que se mira ─────────────────────────────

    fn heroe(&self, i: usize) -> Option<&nucleo::HeroeNucleo> {
        self.n.heroe(self.bots[i].heroe)
    }

    /// La plaza donde reside, si reside en una plaza que sigue en pie.
    fn casa(&self, i: usize) -> Option<u64> {
        match self.heroe(i)?.heroe.residencia {
            Some(Residencia::Asentamiento(p)) if self.n.estado().asentamientos.contains_key(&(p as u64)) => Some(p as u64),
            _ => None,
        }
    }

    /// ¿Es `otra` una Facción a la que la de `yo` ataca? No a sí misma ni a un aliado; las pacíficas, solo a quien ya está en guerra con ellas.
    fn enemiga(&self, yo: u32, otra: u32) -> bool {
        let rel = self.n.relacion(yo, otra);
        otra != yo && otra != 0 && rel != Some(Relacion::Alianza) && (self.belicosas.contains(&yo) || rel == Some(Relacion::Guerra))
    }

    fn faccion_de_plaza(&self, p: u64) -> Option<u32> {
        self.n.estado().asentamientos.get(&p).map(|a| a.faccion)
    }

    /// Las escuadras del héroe en su campamento, con soldados y fuera de la guarnición: las que puede sacar o con las que defiende.
    fn del_campamento(&self, i: usize) -> Vec<u64> {
        self.heroe(i).map(|h| h.heroe.escuadras.iter().filter(|s| s.contenedor == Contenedor::Campamento && !s.en_guarnicion && s.cantidad > 0).map(|s| s.id).collect()).unwrap_or_default()
    }

    /// Lo más cercano a `desde`, a `radio` como mucho.
    fn cercano<T: Copy>(desde: Punto, radio: i64, candidatos: impl Iterator<Item = (Punto, T)>) -> Option<(Punto, T)> {
        candidatos.filter(|(p, _)| desde.dist(*p) <= radio).min_by_key(|(p, _)| desde.dist(*p))
    }

    /// El campamento de bandidos más cercano a casa, a `RADIO_BANDIDOS` como mucho y fuera de la protección de los mercenarios, si al héroe le toca volver a por bandidos.
    fn bandidos_cerca(&self, t: Instante, i: usize) -> Option<(Punto, ObjetivoAtaque)> {
        let n = &self.n;
        let casa = n.estado().asentamientos.get(&self.casa(i)?)?.posicion;
        (t >= self.bots[i].ultimo_bandido + BANDIDOS_CADA_H * HORA).then_some(())?;
        Self::cercano(casa, RADIO_BANDIDOS, n.campamentos_de_bandidos().filter(|b| !n.en_proteccion_de_campamento(b.posicion)).map(|b| (b.posicion, ObjetivoAtaque::Campamento(b.id))))
    }

    /// El objetivo enemigo de esa clase más cercano: una plaza no protegida, un pueblo activo o una avanzada operativa de una Facción enemiga.
    fn enemigo_cerca(&self, yo: u32, desde: Punto, clase: usize, descartadas: &BTreeMap<u64, Instante>) -> Option<(Punto, ObjetivoAtaque)> {
        let n = &self.n;
        let descartada = |p: u64| descartadas.get(&p).is_some_and(|&hasta| n.ahora() < hasta);
        let enemigas = n.estado().asentamientos.values().filter(|a| self.enemiga(yo, a.faccion) && !descartada(a.id));
        let lejos = |p: &Punto| !n.en_proteccion_de_campamento(*p);
        match clase {
            0 => Self::cercano(desde, RADIO_CAMPANA, enemigas.filter(|a| !n.plaza_protegida(a.id)).map(|a| (a.posicion, ObjetivoAtaque::Asentamiento(a.id))).filter(|(p, _)| lejos(p))),
            1 => {
                // Solo un pueblo con algo que llevarse (lo que su almacén guarda por encima de la mitad).
                let botin = |p: &&asentamiento::pueblos::Pueblo| militar::total(&combate::pueblo::saqueable(&n.estado().reglas.economia.combate, &p.almacen)) >= SAQUEO_MINIMO;
                Self::cercano(desde, RADIO_CAMPANA, enemigas.flat_map(|a| a.economia.pueblos.iter().filter(|p| p.activo())).filter(botin).filter_map(|p| Some((n.posicion_de_pueblo(p.id)?, ObjetivoAtaque::Pueblo(p.id)))).filter(|(p, _)| lejos(p)))
            }
            _ => Self::cercano(desde, RADIO_CAMPANA, enemigas.flat_map(|a| a.economia.avanzadas.iter().filter(|x| x.operativa())).map(|x| (x.posicion, ObjetivoAtaque::Avanzada(x.id))).filter(|(p, _)| lejos(p))),
        }
    }

    /// Una presa a la vista de la columna `c`: una columna enemiga de su misma clase con algún héroe sano o, para una columna personal, una caravana enemiga en camino.
    fn presa_a_la_vista(&self, t: Instante, c: u64, yo: u32) -> Option<Objetivo> {
        let n = &self.n;
        let e = n.ejercito(c)?;
        let (aqui, vista) = (e.posicion_en(t), n.alcance_de_vista(c)?);
        let sano = |x: &nucleo::Ejercito| x.columna.participantes.iter().any(|p| n.heroe(p.heroe).is_some_and(|h| !h.heroe.esta_herido(t)));
        let columna = n
            .estado()
            .ejercitos
            .values()
            .filter(|x| x.id != c && x.columna.tipo == e.columna.tipo && n.en_el_mapa(x.id) && x.columna.faccion.is_some_and(|f| self.enemiga(yo, f)) && sano(x))
            .map(|x| (x.posicion_en(t), Objetivo::Ejercito(x.id)));
        let caravanas = n
            .estado()
            .comercio
            .flota
            .iter()
            .filter(|k| e.columna.tipo == TipoColumna::Personal && k.tipo == TipoCaravana::Comercial && matches!(k.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando))
            .filter(|k| self.faccion_de_plaza(k.origen as u64).is_some_and(|f| self.enemiga(yo, f)))
            .filter_map(|k| Some((n.posicion_de_caravana(k.id, t)?, Objetivo::Caravana(k.id))));
        Self::cercano(aqui, vista, columna.chain(caravanas)).map(|(_, o)| o)
    }

    /// Lo que cabe todavía en el carro con el que el héroe saldría: el que trajo a casa y, si conserva su columna, lo que ella lleva.
    fn libre_en_el_carro(&self, i: usize) -> i64 {
        let Some(h) = self.heroe(i) else { return 0 };
        let columna = h.columna.and_then(|c| self.n.ejercito(c)).map_or(0, |e| militar::total(&e.columna.carga));
        (self.n.estado().reglas.economia.militar.logistica.capacidad_carro - militar::total(&h.carro) - columna).max(0)
    }

    fn poder_de_columna(&self, c: u64) -> i64 {
        let cr = &self.n.estado().reglas.economia.combate;
        let escuadras: Vec<militar::escuadra::Escuadra> = self.n.escuadras_de_columna(c).into_iter().cloned().collect();
        let heroes = self.n.ejercito(c).map_or(0, |e| e.columna.participantes.len());
        combate::resolucion::poder_de_bando(cr, &escuadras, heroes)
    }

    // ───────────────────────────── El turno ─────────────────────────────

    fn turno(&mut self, t: Instante, i: usize) {
        let Some(h) = self.heroe(i) else { return };
        let (ubicacion, residencia) = (h.heroe.presencia.ubicacion, h.heroe.residencia);
        match ubicacion {
            Ubicacion::Asentamiento(p) if residencia == Some(Residencia::Asentamiento(p)) => self.en_casa(t, i, p as u64),
            Ubicacion::Asentamiento(p) => drop(self.suya(t, i, "SalirDeAsentamiento", Comando::SalirDeAsentamiento { asentamiento: p as u64, heroe: self.bots[i].heroe })),
            Ubicacion::Columna(c) => self.en_columna(t, i, c),
            Ubicacion::Pueblo(x) => drop(self.suya(t, i, "SalirDePueblo", Comando::SalirDePueblo { pueblo: x, heroe: self.bots[i].heroe })),
            Ubicacion::Avanzada(x) => drop(self.suya(t, i, "SalirDeAvanzada", Comando::SalirDeAvanzada { avanzada: x, heroe: self.bots[i].heroe })),
            Ubicacion::Mercenarios(_) | Ubicacion::Desconectado(_) => {}
        }
    }

    // ───────────────────────────── En casa ─────────────────────────────

    fn en_casa(&mut self, t: Instante, i: usize, p: u64) {
        self.bots[i].tarea = Tarea::Libre;
        if t >= self.bots[i].proximo_reclutamiento {
            self.bots[i].proximo_reclutamiento = t + HORA;
            self.reclutar(t, i, p);
            // Gobierna el Guardián; en una plaza sin Guardián (una conquistada), cualquier residente.
            let guardian = self.bots.iter().any(|b| b.rol == Rol::Guardian && self.n.heroe(b.heroe).is_some_and(|h| h.heroe.residencia == Some(Residencia::Asentamiento(p as u32))));
            if self.bots[i].rol == Rol::Guardian || !guardian {
                self.gobernar(t, i, p);
            }
        }
        self.loadout(t, i);
        // Lo que trajo en el carro (el botín) va a su almacén personal: así sale con el carro libre.
        let cabe = self.n.estado().reglas.economia.comercio.mercenarios.capacidad_almacen_personal;
        if self.heroe(i).is_some_and(|h| !h.carro.is_empty() && militar::total(&h.almacen_personal) < cabe) {
            let heroe = self.bots[i].heroe;
            self.suya(t, i, "VaciarCarro", Comando::VaciarCarro { heroe });
        }
        let (sano, libre) = (self.heroe(i).is_some_and(|h| !h.heroe.esta_herido(t)), t >= self.bots[i].proxima_salida);
        if let Some((conv, desde)) = self.bots[i].convocatoria {
            if t >= desde + 10 * MIN {
                self.partir(t, i, conv);
            }
            return;
        }
        if !sano || !libre {
            return;
        }
        match self.bots[i].rol {
            Rol::Guardian => {}
            Rol::Capitan => self.lanzar_campana(t, i, p),
            Rol::Cazador => self.salir_a_cazar(t, i, p),
            Rol::Intendente => self.intendencia(t, i, p),
        }
        if self.bots[i].rol != Rol::Guardian && self.bots[i].rol != Rol::Capitan {
            self.unirse_a_convocatoria(t, i, p);
        }
    }

    /// Recluta o repone una escuadra: la mejor tropa que la plaza sabe hacer (tecnología y edificio) y que el núcleo acepta, sin pasar de su Liderazgo con las escuadras
    /// nuevas.
    fn reclutar(&mut self, t: Instante, i: usize, p: u64) {
        let Some(h) = self.heroe(i) else { return };
        let mr = &self.n.estado().reglas.economia.militar;
        let a = &self.n.estado().asentamientos[&p];
        let usado: i64 = h.heroe.escuadras.iter().filter(|s| s.prestada.is_none()).map(|s| militar::liderazgo::coste(mr, s.tropa)).sum();
        let tope = ESCUADRAS_POR_PAPEL[self.bots[i].rol as usize];
        // Mientras la plaza no llega al nivel 2, cada héroe solo tiene una escuadra y no recluta más si se queda por debajo de los pesants que pide el ascenso.
        let requisito = self.n.estado().reglas.economia.datos.nivel.requisitos.get(&2).map_or(0, |q| q.pesants);
        let pesants = a.economia.poblacion.enteros().0;
        let crece = a.economia.nivel_actual() < 2;
        let mut tropas: Vec<&militar::catalogo::DefTropa> = TROPAS.iter().map(|&x| mr.tropas.tropa(x)).collect();
        tropas.sort_by_key(|d| (Reverse(d.escalon), Reverse(d.poder_base), d.tropa as usize));
        let elegibles: Vec<Tropa> = tropas.into_iter().filter(|d| {
            let puede = self.n.tiene_tecnologia(a.faccion, &d.tecnologia) && militar::reclutamiento::edificio_basta(d, &a.economia.edificios);
            let sin_frenar = !crece || pesants >= requisito + d.unidades;
            match h.heroe.escuadras.iter().find(|s| s.tropa == d.tropa) {
                Some(s) => puede && sin_frenar && s.contenedor == Contenedor::Campamento && s.cantidad < d.unidades,
                None => puede && (h.heroe.escuadras.is_empty() || sin_frenar && h.heroe.escuadras.len() < tope) && usado + militar::liderazgo::coste(mr, d.tropa) <= h.heroe.liderazgo_base,
            }
        })
        .map(|d| d.tropa)
        .collect();
        // De la mejor a la peor, hasta que una entre (como mucho tres intentos: el equipo o la población que falten los dice el núcleo).
        let heroe = self.bots[i].heroe;
        for tropa in elegibles.into_iter().take(INTENTOS_DE_RECLUTAR) {
            if self.con_gasto(t, i, p, "ReclutarTropa", Comando::ReclutarTropa { asentamiento: p, heroe, tropa }).aceptada() {
                break;
            }
        }
    }

    /// El loadout activo lleva lo que el héroe tiene en el campamento (fuera de la guarnición): con eso defiende en persona.
    fn loadout(&mut self, t: Instante, i: usize) {
        let Some(h) = self.heroe(i) else { return };
        let Some(l) = h.heroe.loadouts.first() else { return };
        let (id, actual) = (l.id, l.escuadras.clone());
        let quiere = self.del_campamento(i);
        if actual != quiere || !l.activo {
            let heroe = self.bots[i].heroe;
            self.suya(t, i, "GuardarLoadout", Comando::GuardarLoadout { heroe, loadout: Some(id), nombre: "Default".into(), escuadras: quiere, perks: vec![], activo: Some(true) });
        }
    }

    /// El Guardián gobierna: ascenso, edificios, flota, pueblo, muralla y guarnición.
    fn gobernar(&mut self, t: Instante, i: usize, p: u64) {
        let (nivel, madera, tiene, recintos, pueblos, mercado_activo) = {
            let a = &self.n.estado().asentamientos[&p];
            let ed = &a.economia.edificios;
            let tiene: BTreeSet<Tipo> = ed.iter().map(|e| e.tipo).collect();
            (a.economia.nivel_actual(), a.economia.almacen.cantidad(Recurso::Madera), tiene, a.economia.recintos.len(), a.economia.pueblos.len(), ed.iter().any(|e| e.tipo == Tipo::Mercado && e.activo()))
        };
        if t % (3 * HORA) < HORA {
            self.suya(t, i, "SolicitarAscenso", Comando::SolicitarAscenso { asentamiento: p });
        }
        for tipo in [Tipo::Mercado, Tipo::Barracon, Tipo::GaleriaDeTiro] {
            if !tiene.contains(&tipo) && madera >= 150_000 && (tipo == Tipo::Mercado || nivel >= 2 && t % (6 * HORA) < HORA) {
                self.suya(t, i, "AnadirEdificio", Comando::AnadirEdificio { asentamiento: p, tipo });
                break;
            }
        }
        if mercado_activo {
            self.flota(t, i, p);
        }
        let cantera = self.n.candidatos_de_pueblo(p).iter().find(|(_, tipo)| *tipo == Tipo::Cantera).map(|&(nodo, _)| nodo);
        if let Some(nodo) = cantera.filter(|_| nivel >= 2 && pueblos == 0) {
            self.suya(t, i, "MarcarPueblo", Comando::MarcarPueblo { asentamiento: p, nodo });
        }
        if nivel >= 2 && recintos == 0 {
            self.suya(t, i, "ComprometerRecinto", Comando::ComprometerRecinto { asentamiento: p, cargo: Cargo::Gobernador, nivel: 1 });
        }
        // La leva (escalón 1) a la guarnición, mientras quepa en el cupo del héroe (puntos de Liderazgo).
        let heroe = self.bots[i].heroe;
        let mr = self.n.estado().reglas.economia.militar.clone();
        let mut libre = self.n.cupo_de_guarnicion(p).unwrap_or(0) - self.heroe(i).map_or(0, |h| h.heroe.escuadras.iter().filter(|s| s.en_guarnicion).map(|s| militar::liderazgo::coste(&mr, s.tropa)).sum());
        let leva: Vec<(u64, i64)> = self.heroe(i).map(|h| h.heroe.escuadras.iter().filter(|s| s.contenedor == Contenedor::Campamento && !s.en_guarnicion && s.cantidad > 0 && mr.tropas.tropa(s.tropa).escalon == 1).map(|s| (s.id, militar::liderazgo::coste(&mr, s.tropa))).collect()).unwrap_or_default();
        for (escuadra, coste) in leva {
            if coste > libre || !self.suya(t, i, "AsignarGuarnicion", Comando::AsignarGuarnicion { heroe, escuadra }).aceptada() {
                break;
            }
            libre -= coste;
        }
    }

    /// Dos caravanas con dos carros con buey cada una, fuera del reparto automático.
    fn flota(&mut self, t: Instante, i: usize, p: u64) {
        let flota: Vec<(u64, usize, usize)> = self.n.flota_de(p).iter().filter(|c| c.tipo == TipoCaravana::Comercial).map(|c| (c.id, c.carros.len(), c.carros.iter().filter(|k| k.animal.is_some()).count())).collect();
        let Some(cupos) = self.n.cupos_de_flota(p) else { return };
        let mut carros_libres = (cupos.carros as usize).saturating_sub(flota.iter().map(|f| f.1).sum::<usize>());
        if flota.len() < 2.min(cupos.caravanas as usize) {
            let r = self.suya(t, i, "CrearCaravana", Comando::CrearCaravana { asentamiento: p });
            if let Some(&caravana) = r.creadas.first().filter(|_| r.aceptada()) {
                self.suya(t, i, "ReservarCaravana", Comando::ReservarCaravana { caravana, reservada: true });
            }
        }
        for (caravana, carros, con_animal) in flota {
            if carros < 2 && carros_libres > 0 && self.suya(t, i, "AnadirCarro", Comando::AnadirCarro { caravana, tipo: TipoCarro::Basico }).aceptada() {
                carros_libres -= 1;
            }
            for k in con_animal..carros {
                self.suya(t, i, "AnadirAnimal", Comando::AnadirAnimal { caravana, carro: k as u32, animal: Animal::Buey });
            }
        }
    }

    /// Ya está en la convocatoria: a los diez minutos parte con quien se haya unido y va a por su objetivo.
    fn partir(&mut self, t: Instante, i: usize, conv: u64) {
        let heroe = self.bots[i].heroe;
        self.bots[i].convocatoria = None;
        let r = self.suya(t, i, "PartirConvocatoria", Comando::PartirConvocatoria { heroe });
        if let Some(&c) = r.creadas.first().filter(|_| r.aceptada()) {
            self.al_salir(t, i, c);
        } else if self.n.convocatoria(conv).is_some() {
            self.suya(t, i, "CancelarConvocatoria", Comando::CancelarConvocatoria { heroe });
        }
    }

    /// El Capitán sale de campaña: convoca (y parte a los diez minutos) o sale como ejército y los suyos se le unen en la puerta.
    fn lanzar_campana(&mut self, t: Instante, i: usize, p: u64) {
        let escuadras = self.del_campamento(i);
        if escuadras.is_empty() {
            return;
        }
        let heroe = self.bots[i].heroe;
        // El trigo de campaña, hasta lo que le quepa en el carro (el que trajo a casa y no pudo vaciar va con él).
        let carga: Vec<(Recurso, i64)> = Some((Recurso::Trigo, TRIGO_DE_CAMPANA.min(self.libre_en_el_carro(i)))).filter(|x| x.1 > 0).into_iter().collect();
        if self.rng.por_debajo(100) < CONVOCA_PCT {
            let r = self.con_gasto(t, i, p, "ConvocarEjercito", Comando::ConvocarEjercito { heroe, escuadras, carga, politica: Politica::Aceptar });
            if let Some(&conv) = r.creadas.first().filter(|_| r.aceptada()) {
                self.bots[i].convocatoria = Some((conv, t));
                self.cita(t + 10 * MIN, i);
            }
            return;
        }
        let r = self.con_gasto(t, i, p, "MovilizarEjercito", Comando::MovilizarEjercito { asentamiento: p, heroe, escuadras, carga, politica: Politica::Aceptar });
        let Some(&c) = r.creadas.first().filter(|_| r.aceptada()) else { return };
        // Los compañeros libres se suman en la puerta: el Intendente desde la plaza, el Cazador saliendo y uniéndose en campo.
        for j in self.de_plaza.get(&self.bots[i].plaza).cloned().unwrap_or_default() {
            let libre = self.n.heroe(self.bots[j].heroe).is_some_and(|h| h.heroe.presencia.ubicacion == Ubicacion::Asentamiento(p as u32) && !h.heroe.esta_herido(t)) && self.bots[j].convocatoria.is_none();
            if j == i || !libre || !matches!(self.bots[j].rol, Rol::Cazador | Rol::Intendente) || self.rng.por_debajo(100) >= SE_UNE_PCT || t < self.bots[j].proxima_salida {
                continue;
            }
            let (otro, suyas) = (self.bots[j].heroe, self.del_campamento(j));
            if self.bots[j].rol == Rol::Intendente {
                self.con_gasto(t, j, p, "UnirseAEjercito", Comando::UnirseAEjercito { asentamiento: p, heroe: otro, ejercito: c, escuadras: suyas, carga: vec![] });
            } else if self.con_gasto(t, j, p, "SalirAlMundo", Comando::SalirAlMundo { asentamiento: p, heroe: otro, escuadras: suyas, carga: vec![] }).aceptada() {
                self.suya(t, j, "UnirseEnCampo", Comando::UnirseEnCampo { heroe: otro, ejercito: c });
            }
        }
        self.al_salir(t, i, c);
    }

    /// Un compañero libre se une a la convocatoria abierta en su plaza.
    fn unirse_a_convocatoria(&mut self, t: Instante, i: usize, p: u64) {
        if !self.heroe(i).is_some_and(|h| h.heroe.presencia.ubicacion == Ubicacion::Asentamiento(p as u32)) {
            return;
        }
        let f = self.bots[i].faccion;
        let Some(conv) = self.n.convocatorias_del_lugar(Lugar::Asentamiento(p as u32)).iter().find(|c| c.faccion == f && !c.integrantes.iter().any(|x| x.heroe == self.bots[i].heroe)).map(|c| c.id) else { return };
        if self.rng.por_debajo(100) < SE_UNE_PCT {
            let (heroe, escuadras) = (self.bots[i].heroe, self.del_campamento(i));
            self.suya(t, i, "UnirseAConvocatoria", Comando::UnirseAConvocatoria { heroe, convocatoria: conv, escuadras, carga: vec![] });
        }
    }

    fn salir_a_cazar(&mut self, t: Instante, i: usize, p: u64) {
        let heroe = self.bots[i].heroe;
        let solo = self.bots[i].temerario && t >= self.bots[i].ultimo_temerario + DIA;
        let escuadras = if solo { vec![] } else { self.del_campamento(i) };
        let r = self.con_gasto(t, i, p, "SalirAlMundo", Comando::SalirAlMundo { asentamiento: p, heroe, escuadras, carga: vec![] });
        let Some(&c) = r.creadas.first().filter(|_| r.aceptada()) else { return };
        if solo {
            self.bots[i].ultimo_temerario = t;
            if let Some((destino, obj)) = self.bandidos_cerca(t, i) {
                (self.bots[i].objetivos, self.bots[i].ultimo_bandido) = (OBJETIVOS_POR_SALIDA - 1, t);
                return self.ir(t, i, c, destino, Meta::Atacar(obj));
            }
        }
        self.al_salir(t, i, c);
    }

    /// El Intendente: materiales al pueblo, la avanzada, o una caravana (escoltada o adjunta).
    fn intendencia(&mut self, t: Instante, i: usize, p: u64) {
        let heroe = self.bots[i].heroe;
        let (nivel, posicion, radio) = {
            let a = &self.n.estado().asentamientos[&p];
            (a.economia.nivel_actual(), a.posicion, a.radio)
        };
        let capacidad = self.libre_en_el_carro(i);
        // 1. La obra de un pueblo que espera materiales: los saca del almacén (hasta lo que cabe en el carro).
        let obra = self.n.pueblos_de(p).iter().find(|x| !x.activo() && !x.falta().is_empty()).map(|x| (x.id, x.falta()));
        if let Some((pueblo, falta)) = obra {
            let carga = self.cabe(p, &falta, capacidad);
            if let Some(destino) = self.n.posicion_de_pueblo(pueblo).filter(|_| !carga.is_empty()) {
                let r = self.con_gasto(t, i, p, "SalirAlMundo", Comando::SalirAlMundo { asentamiento: p, heroe, escuadras: vec![], carga });
                if let Some(&c) = r.creadas.first().filter(|_| r.aceptada()) {
                    return self.ir(t, i, c, destino, Meta::Pueblo(pueblo));
                }
            }
        }
        // 2. Una atalaya, si la plaza tiene el nivel y aún no tiene ninguna.
        if nivel >= 2 && self.n.avanzadas_de(p).is_empty() {
            let coste = vec![(Recurso::Madera, 200_000), (Recurso::Piedra, 100_000)];
            let sitio = (0..8).map(|k| (k as f64 * 45.0).to_radians()).map(|a| Punto::new(posicion.x + (radio as f64 * 0.6 * a.cos()) as i64, posicion.y + (radio as f64 * 0.6 * a.sin()) as i64)).find(|q| self.n.en_zona(p, *q) && self.c.mundo.transitable_en(q.x, q.y));
            if let Some(sitio) = sitio.filter(|_| self.cabe(p, &coste, capacidad) == coste) {
                let r = self.con_gasto(t, i, p, "SalirAlMundo", Comando::SalirAlMundo { asentamiento: p, heroe, escuadras: vec![], carga: coste });
                if let Some(&c) = r.creadas.first().filter(|_| r.aceptada()) {
                    return self.ir(t, i, c, sitio, Meta::Avanzada(sitio));
                }
            }
        }
        // 3. Una caravana a una plaza hermana, por turnos: escoltada (sale sola con una escuadra cedida) o adjunta al ejército del Intendente.
        if t < self.bots[i].proxima_caravana {
            return;
        }
        let f = self.bots[i].faccion;
        let Some(hermana) = self.n.plazas_de(f).filter(|a| a.id != p).min_by_key(|a| a.posicion.dist(posicion)).map(|a| a.id) else { return };
        let Some(caravana) = self.n.flota_de(p).iter().find(|k| k.estado == EstadoCaravana::Disponible && k.carros.iter().any(|x| x.animal.is_some())).map(|k| k.id) else { return };
        self.bots[i].proxima_caravana = t + CARAVANA_CADA_H * HORA;
        if self.n.caravana(caravana).is_some_and(|k| !k.escolta.is_empty()) {
            self.suya(t, i, "QuitarEscolta", Comando::QuitarEscolta { caravana, heroe, escuadras: None });
        }
        self.bots[i].adjunta = !self.bots[i].adjunta;
        let escuadras = self.del_campamento(i);
        if self.bots[i].adjunta && !escuadras.is_empty() {
            let r = self.con_gasto(t, i, p, "MovilizarEjercito", Comando::MovilizarEjercito { asentamiento: p, heroe, escuadras, carga: vec![], politica: Politica::Rechazar });
            let Some(&c) = r.creadas.first().filter(|_| r.aceptada()) else { return };
            self.suya(t, i, "AdjuntarCaravana", Comando::AdjuntarCaravana { caravana, ejercito: c });
            let destino = self.n.estado().asentamientos[&hermana].posicion;
            return self.ir(t, i, c, destino, Meta::Aparcar { caravana, plaza: hermana });
        }
        if let Some(&escolta) = escuadras.last() {
            self.suya(t, i, "AsignarEscolta", Comando::AsignarEscolta { caravana, heroe, escuadras: vec![escolta] });
        }
        self.suya(t, i, "PrepararCaravana", Comando::PrepararCaravana { caravana, destino: hermana, carga: vec![(Recurso::Trigo, CARGA_DE_CARAVANA)], salir_en: None });
    }

    /// Lo que se puede sacar del almacén de la plaza de una lista, hasta la capacidad del carro.
    fn cabe(&self, p: u64, pedido: &[(Recurso, i64)], capacidad: i64) -> Vec<(Recurso, i64)> {
        let almacen = &self.n.estado().asentamientos[&p].economia.almacen;
        let mut libre = capacidad;
        pedido
            .iter()
            .filter_map(|&(r, q)| {
                let q = q.min(almacen.cantidad(r)).min(libre);
                libre -= q;
                (q > 0).then_some((r, q))
            })
            .collect()
    }

    // ───────────────────────────── Fuera ─────────────────────────────

    /// Acaba de salir con la columna `c`: a por lo primero que haya.
    fn al_salir(&mut self, t: Instante, i: usize, c: u64) {
        self.bots[i].objetivos = 0;
        self.bots[i].tarea = Tarea::Libre;
        self.elegir(t, i, c);
    }

    /// Una cita de llegada (o de espera) para el bot: invalida las anteriores.
    fn cita(&mut self, t: Instante, i: usize) {
        self.bots[i].cita += 1;
        let cita = self.bots[i].cita;
        self.programar(t, Accion::Turno { bot: i, cita: Some(cita) });
    }

    /// Marcha hacia `destino` para hacer `meta` al llegar; sin camino, vuelve a casa (o se queda, si ya volvía).
    fn ir(&mut self, t: Instante, i: usize, c: u64, destino: Punto, meta: Meta) {
        let r = self.suya(t, i, "MarcharA", Comando::MarcharA { ejercito: c, destino });
        let llega = match r.resultado {
            Resultado::Aceptada => self.n.ejercito(c).and_then(|e| e.plan.fin()).unwrap_or(t),
            Resultado::SinCambios => t,
            _ if meta == Meta::Casa => return,
            _ => return self.volver(t, i, c),
        };
        self.bots[i].tarea = Tarea::Ir(meta);
        self.cita(llega.max(t + 1), i);
    }

    fn volver(&mut self, t: Instante, i: usize, c: u64) {
        let destino = match self.casa(i) {
            Some(p) => self.n.estado().asentamientos[&p].posicion,
            None => match self.heroe(i).and_then(|h| h.heroe.residencia) {
                Some(Residencia::Campamento(k)) => self.n.puerta_de_campamento(k).unwrap_or_default(),
                _ => return,
            },
        };
        self.ir(t, i, c, destino, Meta::Casa);
    }

    fn en_columna(&mut self, t: Instante, i: usize, c: u64) {
        let heroe = self.bots[i].heroe;
        let Some(e) = self.n.ejercito(c) else { return };
        if e.columna.lider != heroe || e.en_batalla || e.columna.es_formacion() {
            return;
        }
        if let Some(hasta) = self.heroe(i).and_then(|h| h.heroe.herido_hasta).filter(|&x| x > t) {
            return self.cita(hasta, i);
        }
        let (viveres, maximo) = (e.columna.viveres.valor_en(t), e.columna.viveres.maximo.unwrap_or(1).max(1));
        let trigo = e.columna.carga.get(&Recurso::Trigo).copied().unwrap_or(0);
        if 100 * viveres < VOLVER_VIVERES_PCT * maximo {
            if trigo > 0 {
                let cantidad = trigo.min(maximo - viveres);
                if self.suya(t, i, "PasarAViveres", Comando::PasarAViveres { heroe, cantidad }).aceptada() {
                    self.medidas.trigo_a_viveres += cantidad;
                }
            } else if self.bots[i].tarea != Tarea::Ir(Meta::Casa) {
                return self.volver(t, i, c);
            }
        }
        match self.bots[i].tarea {
            Tarea::Persiguiendo(presa) => self.seguir_persecucion(t, i, c, presa),
            Tarea::Ir(meta) => {
                if self.n.ejercito(c).is_some_and(|e| e.marchando() && e.plan.fin().is_some_and(|f| f > t)) {
                    let llega = self.n.ejercito(c).and_then(|e| e.plan.fin()).unwrap_or(t);
                    return self.cita(llega, i);
                }
                self.llegar(t, i, c, meta);
            }
            Tarea::Libre => self.elegir(t, i, c),
        }
    }

    /// Elige qué hacer fuera, según el papel.
    fn elegir(&mut self, t: Instante, i: usize, c: u64) {
        let (rol, f) = (self.bots[i].rol, self.bots[i].faccion);
        if self.bots[i].objetivos >= OBJETIVOS_POR_SALIDA || self.casa(i).is_none() || matches!(rol, Rol::Intendente | Rol::Guardian) {
            return self.volver(t, i, c);
        }
        if let Some(presa) = self.presa_a_la_vista(t, c, f) {
            let heroe = self.bots[i].heroe;
            if self.suya(t, i, "Perseguir", Comando::Perseguir { heroe, objetivo: presa }).aceptada() {
                self.bots[i].tarea = Tarea::Persiguiendo(presa);
                return self.seguir_persecucion(t, i, c, presa);
            }
        }
        let aqui = self.n.ejercito(c).map_or_else(Punto::default, |e| e.posicion_en(t));
        let objetivo = match rol {
            // El Capitán prueba en un orden que sortea la semilla: plaza, pueblo, avanzada o bandidos.
            Rol::Capitan => {
                let primero = self.rng.por_debajo(4) as usize;
                (0..4).map(|k| (primero + k) % 4).find_map(|k| if k == 3 { self.bandidos_cerca(t, i) } else { self.enemigo_cerca(f, aqui, k, &self.bots[i].descartadas) })
            }
            // El Cazador, el 30 % de las veces; el temerario solo va a por bandidos solo, en su salida del día.
            _ => self.bandidos_cerca(t, i).filter(|_| !self.bots[i].temerario && self.rng.por_debajo(100) < 30),
        };
        if let Some((_, ObjetivoAtaque::Campamento(_))) = objetivo {
            self.bots[i].ultimo_bandido = t;
        }
        match objetivo {
            Some((destino, obj)) => self.ir(t, i, c, destino, Meta::Atacar(obj)),
            None if rol == Rol::Cazador => {
                // Explora alrededor de casa o, la mitad de las veces, alrededor de la plaza enemiga más cercana (por donde pasan sus columnas y sus caravanas).
                let casa = self.n.estado().asentamientos[&self.casa(i).expect("comprobado")].posicion;
                let enemiga = self.enemigo_cerca(f, casa, 0, &BTreeMap::new()).map(|(p, _)| p).filter(|_| self.rng.por_debajo(100) < 50);
                let centro = enemiga.unwrap_or(casa);
                self.bots[i].objetivos += 1;
                for _ in 0..8 {
                    let a = (self.rng.por_debajo(360) as f64).to_radians();
                    let r = (RADIO_EXPLORACION / 2 + self.rng.por_debajo(RADIO_EXPLORACION as u64 / 2) as i64) as f64;
                    let r = if enemiga.is_some() { r / 3.0 } else { r };
                    let destino = Punto::new(centro.x + (r * a.cos()) as i64, centro.y + (r * a.sin()) as i64);
                    if self.c.mundo.transitable_en(destino.x, destino.y) {
                        return self.ir(t, i, c, destino, Meta::Explorar);
                    }
                }
                self.volver(t, i, c)
            }
            None => self.volver(t, i, c),
        }
    }

    /// La persecución sigue mientras la columna persigue; cuando acaba, si la presa está al alcance, se ataca.
    /// Dónde está la presa en `t`, según su plan de ahora.
    fn donde_esta(&self, presa: Objetivo, t: Instante) -> Option<Punto> {
        match presa {
            Objetivo::Ejercito(x) => self.n.ejercito(x).filter(|_| self.n.en_el_mapa(x)).map(|e| e.posicion_en(t)),
            Objetivo::Caravana(x) => self.n.posicion_de_caravana(x, t),
            Objetivo::Asentamiento(_) => None,
        }
    }

    /// El primer instante desde `t` (de `PASO_ALCANCE` en `PASO_ALCANCE`, hasta el fin del plan del perseguidor) en que, según los planes de ahora, la presa queda a tiro: el
    /// bot da la orden de atacar en ese instante, como el jugador que la ve llegar.
    fn alcance(&self, t: Instante, c: u64, presa: Objetivo) -> Option<Instante> {
        let e = self.n.ejercito(c)?;
        let radio = self.n.estado().reglas.economia.militar.logistica.radio_encuentro;
        let fin = e.plan.fin().unwrap_or(t).max(t);
        (0..=(fin - t) / PASO_ALCANCE).map(|k| t + k * PASO_ALCANCE).find(|&s| self.donde_esta(presa, s).is_some_and(|d| d.dist(e.posicion_en(s)) <= radio))
    }

    /// La persecución: si la presa está a tiro, se ataca; si no, se cita el bot en el instante en que lo estará según los planes de ahora (que el núcleo rehace si la presa
    /// cambia el suyo). Si la columna ya no persigue y la presa no está a tiro, se suelta.
    fn seguir_persecucion(&mut self, t: Instante, i: usize, c: u64, presa: Objetivo) {
        let radio = self.n.estado().reglas.economia.militar.logistica.radio_encuentro;
        let aqui = self.n.ejercito(c).map_or_else(Punto::default, |e| e.posicion_en(t));
        if self.donde_esta(presa, t).is_some_and(|d| d.dist(aqui) <= radio) {
            self.bots[i].tarea = Tarea::Libre;
            self.bots[i].objetivos += 1;
            let obj = match presa {
                Objetivo::Ejercito(x) => ObjetivoAtaque::Ejercito(x),
                Objetivo::Caravana(x) => ObjetivoAtaque::Caravana(x),
                Objetivo::Asentamiento(x) => ObjetivoAtaque::Asentamiento(x),
            };
            self.atacar(t, i, c, obj);
            return self.cita(t + MIN, i);
        }
        let sigue = self.n.persiguiendo(c).is_some();
        match self.alcance(t + 1, c, presa) {
            Some(cuando) if sigue => self.cita(cuando, i),
            None if sigue => {
                let fin = self.n.ejercito(c).and_then(|e| e.plan.fin()).filter(|&f| f > t).unwrap_or(t + 2 * MIN);
                self.cita(fin, i)
            }
            _ => {
                self.bots[i].tarea = Tarea::Libre;
                self.bots[i].objetivos += 1;
                self.medidas.persecuciones_sin_alcance += 1;
                self.cita(t + MIN, i)
            }
        }
    }

    /// Ha llegado: hace lo que iba a hacer.
    fn llegar(&mut self, t: Instante, i: usize, c: u64, meta: Meta) {
        let heroe = self.bots[i].heroe;
        self.bots[i].tarea = Tarea::Libre;
        match meta {
            Meta::Atacar(obj) => {
                if let ObjetivoAtaque::Asentamiento(p) = obj {
                    if !self.vale_la_pena_asediar(t, i, c, p) {
                        self.medidas.asedios_descartados += 1;
                        self.bots[i].descartadas.insert(p, t + DESCARTE_H * HORA);
                        self.bots[i].objetivos += 1;
                        return self.elegir(t, i, c);
                    }
                }
                self.atacar(t, i, c, obj);
                self.bots[i].objetivos += 1;
                self.cita(t + MIN, i);
            }
            Meta::Explorar => self.elegir(t, i, c),
            Meta::Pueblo(pueblo) => {
                self.suya(t, i, "EntregarMateriales", Comando::EntregarMateriales { ejercito: c, pueblo });
                self.volver(t, i, c)
            }
            Meta::Avanzada(sitio) => {
                if let Some(p) = self.casa(i) {
                    let r = self.suya(t, i, "ColocarAvanzada", Comando::ColocarAvanzada { asentamiento: p, heroe, posicion: sitio });
                    if let Some(&avanzada) = r.creadas.first().filter(|_| r.aceptada()) {
                        self.suya(t, i, "EntregarMaterialesAAvanzada", Comando::EntregarMaterialesAAvanzada { ejercito: c, avanzada });
                    }
                }
                self.volver(t, i, c)
            }
            Meta::Aparcar { caravana, plaza } => {
                // Solo se aparca dentro de la plaza (decisión del autor, 2026-10-24): el ejército entra con todo en la plaza hermana, la aparca, la manda a su origen y sale con lo suyo.
                // Si la plaza ya no es de su Facción o la caravana ya no va con él (la perdió por el camino), no hay nada que aparcar.
                let de_los_suyos = self.n.estado().asentamientos.get(&plaza).is_some_and(|a| a.faccion == self.bots[i].faccion);
                let la_lleva = self.n.ejercito(c).is_some_and(|e| e.caravanas_adjuntas.contains(&caravana));
                if de_los_suyos && la_lleva && self.esperada(t, i, "GuarnecerHermana", Comando::Guarnecer { asentamiento: plaza, heroe }).aceptada() {
                    if self.esperada(t, i, "AparcarCaravana", Comando::AparcarCaravana { caravana, asentamiento: plaza }).aceptada() {
                        self.esperada(t, i, "EnviarCaravanaAlOrigen", Comando::EnviarCaravanaAlOrigen { caravana });
                    }
                    self.esperada(t, i, "SalirDeAsentamiento", Comando::SalirDeAsentamiento { asentamiento: plaza, heroe });
                }
                self.volver(t, i, c)
            }
            Meta::Casa => self.entrar(t, i, c),
        }
    }

    /// En la puerta de casa: el ejército reposta y se desarma dentro; la columna personal entra. Exiliado, entra en su campamento.
    fn entrar(&mut self, t: Instante, i: usize, c: u64) {
        let heroe = self.bots[i].heroe;
        let ejercito = self.n.ejercito(c).is_some_and(|e| e.columna.tipo == TipoColumna::Ejercito);
        self.bots[i].proxima_salida = t + DESCANSO_MIN * MIN;
        let r = match (self.casa(i), self.heroe(i).and_then(|h| h.heroe.residencia)) {
            (Some(p), _) => {
                if self.n.ejercito(c).is_some_and(|e| 100 * e.columna.viveres.valor_en(t) < 50 * e.columna.viveres.maximo.unwrap_or(1)) {
                    self.con_gasto(t, i, p, "Repostar", Comando::Repostar { heroe, asentamiento: p });
                }
                if ejercito {
                    self.suya(t, i, "Guarnecer", Comando::Guarnecer { asentamiento: p, heroe })
                } else {
                    self.suya(t, i, "EntrarEnAsentamiento", Comando::EntrarEnAsentamiento { asentamiento: p, heroe })
                }
            }
            (None, Some(Residencia::Campamento(k))) => self.suya(t, i, "EntrarEnCampamento", Comando::EntrarEnCampamento { campamento: k, heroe }),
            _ => return,
        };
        if !r.aceptada() {
            self.cita(t + TURNO_MIN * MIN, i);
            self.bots[i].tarea = Tarea::Libre;
            self.bots[i].objetivos = OBJETIVOS_POR_SALIDA;
        }
    }

    /// Antes de asediar se inspecciona: se ataca si el poder de la columna supera con margen lo que se vio (guarnición y héroes dentro).
    fn vale_la_pena_asediar(&mut self, t: Instante, i: usize, c: u64, p: u64) -> bool {
        let heroe = self.bots[i].heroe;
        if !self.suya(t, i, "Inspeccionar", Comando::Inspeccionar { heroe, objetivo: Objetivo::Asentamiento(p) }).aceptada() {
            return true;
        }
        let Some(Contenido::Plaza { heroes, .. }) = self.n.informes_de_inspeccion(heroe).iter().find(|x| x.objetivo == Objetivo::Asentamiento(p)).map(|x| x.contenido.clone()) else { return true };
        let defensa = self.defensa_supuesta(t, p, &heroes);
        let poder: i64 = std::iter::once(c).chain(self.refuerzos(t, i, p).into_iter().map(|(_, col)| col)).map(|col| self.poder_de_columna(col)).sum();
        100 * poder >= MARGEN_ASEDIO_PCT * defensa
    }

    /// Las columnas que se unirían a la convocatoria de un asedio de `i` contra la plaza `p` (M8): las que mandan otros bots de su Facción o de una aliada, en el mapa, libres,
    /// con soldados y con su héroe sano, a `radioEncuentro` de la plaza. (Bot, columna), en el orden de los bots.
    fn refuerzos(&self, t: Instante, i: usize, p: u64) -> Vec<(usize, u64)> {
        let Some(puerta) = self.n.estado().asentamientos.get(&p).map(|a| a.posicion) else { return vec![] };
        let radio = self.n.estado().reglas.economia.militar.logistica.radio_encuentro;
        let yo = self.bots[i].faccion;
        (0..self.bots.len())
            .filter(|&j| j != i && (self.bots[j].faccion == yo || self.n.relacion(yo, self.bots[j].faccion) == Some(Relacion::Alianza)))
            .filter_map(|j| {
                let col = self.n.estado().ejercito_de_heroe(self.bots[j].heroe)?;
                let e = self.n.ejercito(col)?;
                let libre = e.columna.lider == self.bots[j].heroe && !e.en_batalla && self.n.en_el_mapa(col) && e.posicion_en(t).dist(puerta) <= radio;
                (libre && self.n.soldados_de_columna(col).unwrap_or(0) > 0 && !self.n.heroe_herido(self.bots[j].heroe)).then_some((j, col))
            })
            .collect()
    }

    /// La defensa que el bot supone a la plaza: la inspección le dice quién está dentro y él sabe con qué defiende cada uno (la guarnición de los residentes y el loadout
    /// activo de los residentes sanos que están dentro; un visitante, la columna con la que entró), con la cohesión, la ventaja del defensor y el multiplicador de su muralla.
    /// Es lo que haría un jugador que conoce a sus vecinos: el batch no busca la sorpresa, sino medir los asedios que tienen sentido.
    fn defensa_supuesta(&self, t: Instante, p: u64, dentro: &[u64]) -> i64 {
        let n = &self.n;
        let cr = &n.estado().reglas.economia.combate;
        let reside = Some(Residencia::Asentamiento(p as u32));
        let mut escuadras: Vec<militar::escuadra::Escuadra> = n.heroes().filter(|h| h.heroe.residencia == reside).flat_map(|h| h.heroe.escuadras.iter().filter(|s| s.en_guarnicion)).cloned().collect();
        let mut heroes = 0;
        for h in dentro.iter().filter_map(|&h| n.heroe(h)).filter(|h| !h.heroe.esta_herido(t)) {
            heroes += 1;
            let activas = h.heroe.escuadras_del_loadout_activo();
            let suyas = h.heroe.escuadras.iter().filter(|s| if h.heroe.residencia == reside { activas.contains(&s.id) && s.contenedor == Contenedor::Campamento && !s.en_guarnicion } else { matches!(s.contenedor, Contenedor::Ejercito(_)) });
            escuadras.extend(suyas.cloned());
        }
        let poder = combate::resolucion::poder_total(cr, &escuadras, true) + heroes * cr.poder.poder_por_heroe;
        (poder as i128 * cr.poder.ventaja_defensor as i128 / 1_000_000 * n.multiplicador_de_plaza(p) as i128 / 1_000_000) as i64
    }

    // ───────────────────────────── Atacar, con sus invariantes ─────────────────────────────

    fn atacar(&mut self, t: Instante, i: usize, c: u64, obj: ObjetivoAtaque) {
        let heroe = self.bots[i].heroe;
        let (yo, contra) = (self.bots[i].faccion, self.faccion_de(obj));
        let declara_guerra = contra.is_some_and(|f| f != yo && self.n.relacion(yo, f) != Some(Relacion::Guerra));
        let antes = Antes::de(&self.n, c, obj);
        let solo = self.n.soldados_de_columna(c) == Some(0);
        let r = self.suya(t, i, "Atacar", Comando::Atacar { heroe, objetivo: obj, declara_guerra });
        if !r.aceptada() {
            return;
        }
        self.medidas.ataques_solos += (solo && matches!(obj, ObjetivoAtaque::Campamento(_))) as u64;
        if let ObjetivoAtaque::Asentamiento(p) = obj {
            return self.abrir_asedio(t, i, c, p);
        }
        let ef = auditmil::comparar(&self.n, &antes, t, &[c], obj);
        self.apuntar_combate(t, i, c, obj, &ef);
    }

    /// El asedio con números se abre con su convocatoria de 30 s (M8): se unen los refuerzos que están a tiro y se audita al acabar, como una orden de ataque.
    fn abrir_asedio(&mut self, t: Instante, i: usize, c: u64, p: u64) {
        self.medidas.asedios_abiertos += 1;
        let Some((batalla, fin)) = self.n.batalla_sobre(Ubicacion::Asentamiento(p as u32)).and_then(|b| Some((b.id, b.convocatoria_hasta?))) else { return };
        for (j, _) in self.refuerzos(t, i, p) {
            let heroe = self.bots[j].heroe;
            if self.suya(t, j, "UnirseABatalla", Comando::UnirseABatalla { heroe, batalla, lado: None }).aceptada() {
                self.medidas.uniones_a_asedios += 1;
            }
        }
        self.asedios.insert(batalla, AsedioAbierto { bot: i, columna: c, plaza: p, antes: None });
        self.programar(fin - 1, Accion::AntesDelAsedio { batalla });
        self.programar(fin, Accion::FinDelAsedio { batalla });
    }

    fn fin_del_asedio(&mut self, t: Instante, batalla: u64) {
        let Some(AsedioAbierto { bot, columna, plaza, antes: Some(antes) }) = self.asedios.remove(&batalla) else { return };
        let Some(b) = self.n.batalla(batalla) else { return };
        let columnas: Vec<u64> = std::iter::once(columna).chain(b.bloqueo.ejercitos.iter().copied().filter(|&x| x != columna && self.n.ejercito(x).is_some_and(|e| b.lado_de(e.columna.lider) == Some(combate::resolucion::Lado::Atacante)))).collect();
        let obj = ObjetivoAtaque::Asentamiento(plaza);
        let ef = auditmil::comparar(&self.n, &antes, t, &columnas, obj);
        if ef.combate().is_some_and(|h| h.codigo == "combate.asedio_conquista" && h.dato("columnas") > 1) {
            self.medidas.conquistas_con_convocatoria += 1;
        }
        self.apuntar_combate(t, bot, columna, obj, &ef);
    }

    /// Lo que se mide de un ataque ya resuelto (por la orden o, en un asedio, al acabar la convocatoria).
    fn apuntar_combate(&mut self, t: Instante, i: usize, c: u64, obj: ObjetivoAtaque, ef: &auditmil::Efecto) {
        let yo = self.bots[i].faccion;
        self.combate.sumar(&ef.v);
        let m = &mut self.medidas;
        ef.bajas_por_tropa.iter().for_each(|(&tr, &q)| *m.bajas_por_tropa.entry(tr).or_insert(0) += q);
        m.heridos += ef.heridos;
        m.ms_herido += ef.heridos as i64 * self.n.estado().reglas.economia.militar.heroe.herido;
        let Some(h) = ef.combate() else { return };
        let tipo = match obj {
            ObjetivoAtaque::Ejercito(_) => "columna",
            ObjetivoAtaque::Caravana(_) => "caravana",
            ObjetivoAtaque::Campamento(_) => "campamento",
            ObjetivoAtaque::Asentamiento(_) => "asedio",
            ObjetivoAtaque::Pueblo(_) => "pueblo",
            ObjetivoAtaque::Avanzada(_) => "avanzada",
        };
        let gana = matches!(h.codigo.as_str(), "combate.caravana_interceptada" | "combate.campamento_destruido" | "combate.asedio_conquista" | "combate.pueblo_saqueado" | "combate.avanzada_destruida" | "combate.avanzada_en_ruinas" | "combate.avanzada_saqueada") || (h.codigo == "combate.resuelto" && h.dato("ganaElAtacante") == 1);
        let defensa = [h.dato("poderDefensor"), h.dato("defensa"), h.dato("poderCampamento")].into_iter().max().unwrap_or(0);
        m.combates.push(Combate { tipo, gana_atacante: gana, poder_a: h.dato("poderAtacante"), poder_d: defensa, t });
        m.chatarra_generada += h.dato("chatarra");
        match h.codigo.as_str() {
            "combate.caravana_interceptada" => m.asaltos.push((h.dato("aguante") == 1, h.dato("destruida") == 1, h.dato("carros"))),
            "combate.caravana_resiste" => m.asaltos_resistidos[h.dato("aguante") as usize] += 1,
            "combate.campamento_destruido" => {
                let heroes = self.n.ejercito(c).map_or(1, |e| e.columna.participantes.len()).clamp(1, 5) as i64;
                m.oro_por_heroe.extend(std::iter::repeat_n(h.dato("oro") / heroes, heroes as usize));
            }
            "combate.pueblo_saqueado" => m.saqueado_de_pueblos += h.dato("saqueado"),
            "combate.asedio_conquista" => {
                if let ObjetivoAtaque::Asentamiento(p) = obj {
                    let danada = ef.hechos.iter().any(|x| x.codigo == "asentamiento.conquistada" && x.dato("murallaDanada") == 1);
                    m.conquistas.push((t, p, h.dato("faccionDefensora") as u32, yo, danada));
                    if danada {
                        m.murallas_danadas.insert(p, (t, None));
                    }
                }
            }
            _ => {}
        }
        m.carros_perdidos_adjuntas += ef.hechos.iter().filter(|x| x.codigo == "ejercito.caravanas_perdidas").map(|x| x.dato("carros")).sum::<i64>();
        // La guarnición de serie que queda mermada empieza su reposición si estaba completa (o acaba de nacer) y, si ya se reponía, la sigue sin reiniciarla.
        let llena = match obj {
            ObjetivoAtaque::Pueblo(_) => self.n.estado().reglas.economia.combate.pueblos.mineros,
            ObjetivoAtaque::Avanzada(x) => soldados_de_serie_completa(&self.n, x).0,
            _ => 0,
        };
        match (obj, ef.serie) {
            (ObjetivoAtaque::Avanzada(x), (_, None)) => drop(m.reponiendo_avanzada.remove(&x)),
            (ObjetivoAtaque::Pueblo(x), (antes, Some((d, _)))) => apuntar_reposicion(m.reponiendo.entry(x).or_default(), antes, d, llena, t),
            (ObjetivoAtaque::Avanzada(x), (antes, Some((d, _)))) => apuntar_reposicion(m.reponiendo_avanzada.entry(x).or_default(), antes, d, llena, t),
            _ => {}
        }
        if let (ObjetivoAtaque::Asentamiento(p), true) = (obj, ef.combate().is_some_and(|h| h.codigo == "combate.asedio_conquista")) {
            self.quedarse(t, i, c, p);
        }
    }

    /// Quien conquista se queda a defender (como el bot del TypeScript): la plaza ganada no tiene residentes, así que el que mandaba el asedio pasa a residir en ella (por
    /// administración, hasta el sistema 5) y vuelve a entrar, si con eso no deja su casa con menos de dos residentes.
    fn quedarse(&mut self, t: Instante, i: usize, c: u64, p: u64) {
        let Some(casa) = self.casa(i) else { return };
        if self.n.estado().militar.residentes_de(casa).count() <= 2 {
            return;
        }
        let heroe = self.bots[i].heroe;
        if self.admin(t, "FijarResidencia", Comando::FijarResidencia { heroe, residencia: Some(Residencia::Asentamiento(p as u32)) }).aceptada() {
            self.volver(t, i, c);
        }
    }

    fn faccion_de(&self, obj: ObjetivoAtaque) -> Option<u32> {
        let n = &self.n;
        match obj {
            ObjetivoAtaque::Ejercito(x) => n.ejercito(x).and_then(|e| e.columna.faccion),
            ObjetivoAtaque::Caravana(x) => n.caravana(x).and_then(|k| self.faccion_de_plaza(k.origen as u64)),
            ObjetivoAtaque::Campamento(_) => None,
            ObjetivoAtaque::Asentamiento(p) => self.faccion_de_plaza(p),
            ObjetivoAtaque::Pueblo(x) => n.pueblo(x).and_then(|(madre, _)| self.faccion_de_plaza(madre)),
            ObjetivoAtaque::Avanzada(x) => n.avanzada(x).and_then(|(madre, _)| madre).and_then(|m| self.faccion_de_plaza(m)),
        }
    }

    // ───────────────────────────── Observación ─────────────────────────────

    fn observar(&mut self, t: Instante) {
        self.programar(t + OBSERVAR_MIN * MIN, Accion::Observar);
        self.leer();
        if t % DIA < OBSERVAR_MIN * MIN {
            self.muestrear(t / DIA);
        }
        self.comprobar(t);
    }

    /// Lee los hechos nuevos del registro de cambios.
    fn leer(&mut self) {
        let Some(cambios) = self.n.cambios_desde(self.visto) else {
            self.violaciones.registro += self.n.version() - self.visto;
            self.visto = self.n.version();
            return;
        };
        let mut hechos = Vec::new();
        for c in cambios {
            hechos.extend(c.eventos.iter().map(|e| (e.codigo.to_string(), e.en, e.ejercitos.clone(), e.asentamientos.clone(), e.datos.clone())));
            self.visto = c.version;
        }
        for (codigo, t, ejercitos, plazas, datos) in hechos {
            self.hecho(&codigo, t, &ejercitos, &plazas, &datos);
        }
    }

    fn hecho(&mut self, codigo: &str, t: Instante, ejercitos: &[u64], plazas: &[u64], datos: &[(&'static str, i64)]) {
        let dato = |k: &str| datos.iter().find(|(c, _)| *c == k).map_or(0, |&(_, v)| v);
        *self.medidas.hechos.entry(codigo.to_string()).or_insert(0) += 1;
        let m = &mut self.medidas;
        match codigo {
            "tropas.desercion" => {
                let (desertores, escuadra) = (dato("desertores"), dato("escuadra") as u64);
                if let Some((_, s)) = self.n.escuadra(escuadra) {
                    *m.desertores_por_tropa.entry(s.tropa).or_insert(0) += desertores;
                }
                // Una columna deserta solo sin víveres: si su ritmo no cambió desde entonces, en ese instante no tenía.
                match ejercitos.first().and_then(|&c| self.n.ejercito(c)) {
                    Some(e) => {
                        m.desertores_columna += desertores;
                        let v = &e.columna.viveres;
                        self.militares.hambre += (v.t0 <= t && v.valor_en(t) > 0) as u64;
                    }
                    None if ejercitos.is_empty() => m.desertores_guarnicion += desertores,
                    None => m.desertores_columna += desertores,
                }
            }
            "tropas.reclutadas" => {
                if let Some((_, s)) = self.n.escuadra(dato("escuadra") as u64) {
                    *m.reclutados_por_tropa.entry(s.tropa).or_insert(0) += dato("reclutados");
                }
            }
            "bandidos.caravana_interceptada" | "bandidos.caravana_escapa" => m.combates.push(Combate { tipo: "acoso", gana_atacante: codigo == "bandidos.caravana_interceptada", poder_a: dato("poderBandido"), poder_d: dato("defensa"), t }),
            "pueblo.guarnicion_repuesta" => {
                let pueblo = dato("pueblo") as u64;
                if let Some((desde, perdidos)) = m.reponiendo.get_mut(&pueblo).and_then(|q| q.pop_front()) {
                    let cada = self.n.estado().reglas.economia.combate.pueblos.cada_minero;
                    self.militares.reposicion += (t != desde + perdidos * cada) as u64;
                    m.reposiciones_pueblo.push(t - desde);
                }
            }
            "avanzada.guarnicion_repuesta" => {
                let avanzada = dato("avanzada") as u64;
                if let Some((desde, perdidos)) = m.reponiendo_avanzada.get_mut(&avanzada).and_then(|q| q.pop_front()) {
                    let cada = soldados_de_serie_completa(&self.n, avanzada).1;
                    self.militares.reposicion += (t != desde + perdidos * cada) as u64;
                    m.reposiciones_avanzada.push(t - desde);
                }
            }
            "asentamiento.ruinas" => {
                if let Some(&p) = plazas.first() {
                    m.ruinas.push((t, p));
                }
            }
            _ => {}
        }
    }

    /// Los invariantes que no son de una orden: los del comercio y la economía cada 6 horas y al final, y los militares en cada observación.
    fn comprobar(&mut self, t: Instante) {
        if t % (6 * HORA) < OBSERVAR_MIN * MIN || t >= self.fin {
            self.violaciones.sumar(&auditar_cola(&self.n));
            self.violaciones.sumar(&auditar_contenedores(&self.n));
        }
        let n = &self.n;
        let e = n.estado();
        let v = &mut self.militares;
        // La experiencia no baja nunca.
        for h in n.heroes() {
            let antes = self.experiencia_heroe.insert(h.heroe.id, h.heroe.experiencia);
            v.experiencia += antes.is_some_and(|a| h.heroe.experiencia < a) as u64;
            for s in &h.heroe.escuadras {
                let antes = self.experiencia.insert(s.id, (s.experiencia, s.nivel));
                v.experiencia += antes.is_some_and(|(x, nv)| s.experiencia < x || s.nivel < nv) as u64;
            }
        }
        // Un herido lo está `heridoMinutos` como mucho.
        let herido = e.reglas.economia.militar.heroe.herido;
        v.herido += n.heroes().filter(|h| h.heroe.herido_hasta.is_some_and(|x| x > t + herido)).count() as u64;
        // Sin servidores de batalla solo hay asedios con números en su convocatoria (M8): ninguna otra batalla, ninguna activa pasada su convocatoria y nada apartado
        // que no esté en una de ellas.
        let activas: Vec<&combate::ciclo::Batalla> = n.batallas_activas().collect();
        v.batalla += n.batallas().filter(|b| !matches!(b.ticket.contexto, combate::ciclo::Contexto::Asedio { .. })).count() as u64;
        v.batalla += activas.iter().filter(|b| !b.en_convocatoria(t)).count() as u64;
        v.batalla += e.ejercitos.values().filter(|x| x.en_batalla && !activas.iter().any(|b| b.bloqueo.ejercitos.contains(&x.id))).count() as u64;
        // El hambre: una columna sin víveres y con soldados tiene su cita de deserción, y una con víveres no.
        let citas: BTreeSet<u64> = e.cola.volcar().iter().filter(|ev| e.generaciones.es_vigente(ev.clave) && matches!(ev.carga, Carga::Militar(SucesoMilitar::Desercion))).map(|ev| ev.clave.entidad).collect();
        for x in e.ejercitos.values().filter(|x| !x.en_batalla) {
            let soldados = n.soldados_de_columna(x.id).unwrap_or(0);
            let hambre = x.columna.viveres.valor_en(t) <= 0 && soldados > 0;
            v.hambre += (hambre != citas.contains(&x.id)) as u64;
        }
        // Una plaza en ruinas no deja nada atrás: ni flota, ni residentes, ni quien esté dentro.
        for &(_, p) in &self.medidas.ruinas {
            let (dentro, reside) = (Ubicacion::Asentamiento(p as u32), Some(Residencia::Asentamiento(p as u32)));
            v.ruina += (e.asentamientos.contains_key(&p) || e.comercio.flota.iter().any(|k| k.origen as u64 == p) || n.heroes().any(|h| h.heroe.presencia.ubicacion == dentro || h.heroe.residencia == reside)) as u64;
        }
        // Las murallas dañadas por una conquista que ya se repararon.
        for (p, (_, reparada)) in self.medidas.murallas_danadas.iter_mut().filter(|(_, (_, r))| r.is_none()) {
            if e.asentamientos.get(p).is_some_and(|a| !a.economia.recintos.iter().any(|r| r.danado())) {
                *reparada = Some(t);
            }
        }
    }

    fn muestrear(&mut self, dia: i64) {
        let n = &self.n;
        let e = n.estado();
        let escuadras = n.heroes().flat_map(|h| h.heroe.escuadras.iter());
        let (mut soldados, mut en_columna, mut en_guarnicion) = (0, 0, 0);
        for s in escuadras {
            soldados += s.cantidad;
            en_columna += s.cantidad * matches!(s.contenedor, Contenedor::Ejercito(_)) as i64;
            en_guarnicion += s.cantidad * s.en_guarnicion as i64;
        }
        let vivas: BTreeSet<u32> = e.asentamientos.values().map(|a| a.faccion).collect();
        let guerras = (1..=self.facciones).flat_map(|a| (a + 1..=self.facciones).map(move |b| (a, b))).filter(|&(a, b)| n.relacion(a, b) == Some(Relacion::Guerra)).count();
        let exiliados = self.bots.iter().filter(|b| n.heroe(b.heroe).is_some_and(|h| matches!(h.heroe.residencia, Some(Residencia::Campamento(_))))).count();
        self.muestras.push(Muestra {
            dia: dia as u32,
            soldados,
            en_columna,
            en_guarnicion,
            columnas_en_el_mapa: e.ejercitos.keys().filter(|&&c| n.en_el_mapa(c)).count(),
            plazas: e.asentamientos.len(),
            facciones_vivas: vivas.len(),
            guerras,
            heroes_exiliados: exiliados,
            bandidos: n.campamentos_de_bandidos().count(),
        });
    }
}

/// Los soldados de la guarnición de serie completa de una avanzada y cada cuánto vuelve uno (Combate §7.4).
fn soldados_de_serie_completa(n: &Nucleo, avanzada: u64) -> (i64, Instante) {
    let e = n.estado();
    let cr = &e.reglas.economia.combate;
    let escuadras = e.combate.avanzadas.get(&avanzada).map_or(&[][..], |g| &g.escuadras);
    (escuadras.iter().map(|s| cr.militar.tropas.tropa(s.tropa).unidades).sum(), combate::avanzada::reposicion_en(cr, escuadras, 0))
}

/// Apunta las bajas de un combate en las reposiciones de una guarnición de serie: si estaba completa (o acaba de nacer, `antes` vacío), abre una con el instante
/// de la primera baja; si ya se reponía, la sigue sin reiniciarla (suma los soldados perdidos).
fn apuntar_reposicion(q: &mut VecDeque<(Instante, i64)>, antes: Option<(i64, bool)>, despues: i64, llena: i64, t: Instante) {
    let (a, completa) = antes.unwrap_or((llena, true));
    if despues < a {
        match q.back_mut() {
            Some(abierta) if !completa => abierta.1 += a - despues,
            _ => q.push_back((t, a - despues)),
        }
    }
}

/// Huella del estado final del núcleo.
pub fn huella(c: &Corrida) -> [u8; 32] {
    c.nucleo.hash_estado()
}
