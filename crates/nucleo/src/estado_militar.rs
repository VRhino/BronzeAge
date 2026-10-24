//! El estado militar dentro del núcleo (Heroe_Y_Tropa, Presencia_Y_Convocatoria, Campamentos_Mercenarios): los héroes con todas sus escuadras
//! (viven en su héroe, estén donde estén), las convocatorias de ejércitos en preparación, los reclutas de los campamentos de mercenarios que
//! alguien ha tocado, las tecnologías y las relaciones que la administración da a cada Facción hasta que existan los sistemas de tecnología y política. Entra entero en el
//! hash y en las fotos (`codec_militar`). Las reglas puras viven en el crate `militar`; aquí solo está lo que el núcleo guarda.
//!
//! Los ids de héroe, escuadra, loadout, columna y convocatoria salen del contador común de la partida (`Estado::siguiente_id`), como los de las
//! plazas: no chocan con ninguno, así que los eventos propios de un héroe (su salida del mundo) usan su id como entidad de la cola. Los de las
//! formaciones y las peticiones de unión llevan una etiqueta en los bits altos (como los del comercio, `estado_comercio`) porque comparten la
//! tabla de generaciones con la columna o el héroe de los que salen.

use crate::estado_comercio::ETIQUETA;
use asentamiento::catalogo::Tipo;
use base::Instante;
use comercio::mercenarios::CupoDiario;
use comercio::Carga as Mercancia;
use militar::convocatoria::Convocatoria;
use militar::heroe::{Heroe, Residencia};
use militar::inspeccion::Informe;
use militar::mercenarios::Reclutas;
use std::collections::{BTreeMap, BTreeSet};

const ETIQUETA_FORMACION: u64 = 6 * ETIQUETA;
const ETIQUETA_PETICION_DE_UNION: u64 = 7 * ETIQUETA;
const ETIQUETA_PETICION_DE_CONVOCATORIA: u64 = 8 * ETIQUETA;

/// Clave de entidad (de generación y de cola) de la formación de una columna: su caducidad se invalida sin tocar la generación de la columna,
/// que cambia con cada plan y cada ritmo.
pub fn entidad_formacion(columna: u64) -> u64 {
    ETIQUETA_FORMACION | columna
}
/// Clave de entidad de la petición de unión en campo de un héroe (una viva como mucho por solicitante y columna).
pub fn entidad_peticion_de_union(heroe: u64) -> u64 {
    ETIQUETA_PETICION_DE_UNION | heroe
}
/// Clave de entidad de la petición de unión a una convocatoria de un héroe.
pub fn entidad_peticion_de_convocatoria(heroe: u64) -> u64 {
    ETIQUETA_PETICION_DE_CONVOCATORIA | heroe
}

/// Un héroe del núcleo: el de las reglas puras más lo que solo existe aquí.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeroeNucleo {
    pub heroe: Heroe,
    /// La cuenta (el cliente) que lo maneja: solo ella, o la administración, le da órdenes.
    pub cuenta: String,
    /// La Facción de la que es ciudadano: copia del índice de ciudadanía de `Estado::politica` (la fuente es la Facción, con sus ciudadanos en orden de ingreso), para que quien la lee en
    /// caliente no tenga que buscarla. Solo la escribe `Nucleo::tras_cambiar_de_faccion` (`nucleo/ciudadania.rs`).
    pub faccion: Option<u32>,
    /// El almacén personal (`ALMACEN_PERSONAL`): vive donde se reside y viaja con el héroe al cambiar de base.
    pub almacen_personal: Mercancia,
    /// Lo comprado hoy en el mercado de su campamento de residencia, por bien con cupo (madera y piedra).
    pub cupo: CupoDiario,
    /// La columna en la que es participante: aquella en la que va (`ubicacion = columna`) o con la que entró **dentro** de un lugar ajeno (Entrar con todo,
    /// 2026-10-22: no está en el mapa; es lo que trajo consigo y sale con él). Una columna lista a sus participantes y este campo la nombra: se mantienen
    /// juntos (`Nucleo::meter_en_columna`).
    pub columna: Option<u64>,
    /// El carro que lleva sin columna: el de antes de salir del mundo con ella (Presencia_Y_Convocatoria §7: sus escuadras están `fuera` con él, y al
    /// conectarse nace su columna con ese carro) o el que trajo a **su residencia** (entrar en ella no lo vacía; `VaciarCarro` lo pasa al almacén personal y
    /// al salir se lo lleva). Vacío en cualquier otro momento: un visitante tiene el suyo en su columna.
    pub carro: Mercancia,
    /// Héroe de campo del andamiaje de administración (`CrearEjercito`): lleva una columna creada a mano. No cuenta como «el héroe de la cuenta»
    /// (`heroe_de_cuenta`, `crearHeroe`), de modo que una cuenta puede tener varias columnas de escenario además de su héroe.
    pub sintetico: bool,
    /// Lo que ha inspeccionado de cerca (Movimiento_Ejercitos §12): uno por objetivo, hasta `INSPECCION.informeMinutos`. Los vencidos siguen aquí
    /// hasta que otra inspección los barre; quien lee los filtra por `Informe::vigente`.
    pub informes: Vec<Informe>,
    /// La foto del interior de cada plaza de la que salió al mundo (Niebla_De_Guerra §4.5, BAN25), por plaza: la última de cada una.
    pub plazas_recordadas: BTreeMap<u32, PlazaRecordada>,
    /// Cuándo cambió de residencia por última vez por una orden suya (`cambiarResidencia`, `residirEnCampamento`, `dejarResidencia`; Diplomacia_Facciones §4.1): el cooldown cuenta desde aquí.
    /// La reubicación forzosa (ruina, conquista), fundar y la administración no lo estampan.
    pub residencia_cambiada_en: Option<Instante>,
}

/// Lo que un héroe recuerda del interior de una plaza al salir al mundo desde ella (Niebla_De_Guerra §4.5): el almacén, la cola (lo que no está activo: lo activo es público y viaja vivo en la
/// ficha) y la guarnición, ya sin la tropa ni la carga que se lleva. No se refresca mientras está dentro, donde ve lo vivo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlazaRecordada {
    pub visto_en: Instante,
    /// Lo que había de cada recurso (sin los que estaban a 0).
    pub almacen: Mercancia,
    pub cola: Vec<EdificioRecordado>,
    /// Las escuadras de su guarnición.
    pub guarnicion: Vec<u64>,
}

/// Un edificio de la cola tal como se vio: en cola o en obra (`completa_en`), con su nivel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdificioRecordado {
    pub id: u32,
    pub tipo: Tipo,
    pub nivel: u8,
    /// Cuándo acababa la obra; `None` si estaba en cola.
    pub completa_en: Option<Instante>,
}

impl HeroeNucleo {
    /// Un héroe sin columna, sin almacén personal, sin cupo gastado y sin informes.
    pub fn nuevo(heroe: Heroe, cuenta: &str, faccion: Option<u32>) -> Self {
        Self {
            heroe,
            cuenta: cuenta.to_owned(),
            faccion,
            almacen_personal: Mercancia::new(),
            cupo: CupoDiario::default(),
            columna: None,
            carro: Mercancia::new(),
            sintetico: false,
            informes: Vec::new(),
            plazas_recordadas: BTreeMap::new(),
            residencia_cambiada_en: None,
        }
    }
}

/// La clase de la relación **activa** entre dos Facciones (la del crate `politica`, `Estado::politica.relaciones`, vista sin dirección): lo que miran los encuentros, la defensa,
/// la puerta, unirse a batallas y la visión compartida. Sin relación, son neutrales. Desde 5D la relación tiene dirección (en un vasallaje `a` es la señora; en una guerra, quien
/// declaró), estado e historia; esto es solo su clase. `FijarRelacion` (administración) la crea o la rompe con `a` como señora o declarante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relacion {
    /// Los aliados no se cruzan: ni se persiguen ni se ofrecen atacarse (Combate §5).
    Alianza,
    Vasallaje,
    /// Se puede atacar sin declarar la guerra (Combate §2, fase 4D).
    Guerra,
}

/// Cómo trata una plaza a quien llega, según la relación de su Facción con la de la plaza (Entrar con todo, 2026-10-22).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trato {
    /// La misma Facción: siempre entra, y defiende.
    Propia,
    /// Aliada o vasalla: entra si el dueño lo permite, y defiende.
    Aliada,
    /// Sin relación (o sin Facción): entra si el dueño lo permite; nunca defiende.
    Neutral,
    /// En guerra: entra si el dueño lo permite (por defecto no); nunca defiende.
    Enemiga,
}

/// Quién puede entrar en una plaza, por relación con su Facción: el dueño lo decide (`FijarAcceso`) y los pueblos extractores y las avanzadas de la plaza lo
/// heredan. La misma Facción entra siempre. Por defecto, hasta el sistema político, entran todos menos los enemigos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Acceso {
    /// Aliados y vasallos.
    pub aliados: bool,
    pub neutrales: bool,
    pub enemigos: bool,
}

impl Default for Acceso {
    fn default() -> Self {
        Self { aliados: true, neutrales: true, enemigos: false }
    }
}

impl Acceso {
    /// ¿Deja pasar a quien se le trata así? La misma Facción pasa siempre.
    pub fn permite(&self, trato: Trato) -> bool {
        match trato {
            Trato::Propia => true,
            Trato::Aliada => self.aliados,
            Trato::Neutral => self.neutrales,
            Trato::Enemiga => self.enemigos,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EstadoMilitar {
    /// Todos los héroes, por id ascendente.
    pub heroes: BTreeMap<u64, HeroeNucleo>,
    /// Los ejércitos en preparación dentro de una plaza o un campamento (Presencia_Y_Convocatoria §9), por id ascendente. Sin caducidad: duran
    /// hasta que su Líder parte o cancela.
    pub convocatorias: BTreeMap<u64, Convocatoria>,
    /// Los reclutas de los campamentos de mercenarios (índice del artefacto) que alguien ha tocado: el que falta está lleno. Se guardan como
    /// valor e instante (`militar::mercenarios::Reclutas`); la cantidad de ahora se calcula al mirar y nada la escribe por minuto.
    pub reclutas: BTreeMap<u32, Reclutas>,
    /// Tecnologías que la administración ha dado a cada Facción (`FijarTecnologias`), además de las de arranque, que tiene toda Facción.
    pub tecnologias: BTreeMap<u32, BTreeSet<String>>,
    /// Plaza → héroes que residen en ella. Es un índice de `Heroe::residencia`: se reconstruye al recuperar y solo lo toca [`EstadoMilitar::mudar`].
    /// Con él, el paso de un minuto de una plaza sin residentes cuesta una consulta.
    pub(crate) residentes: BTreeMap<u64, BTreeSet<u64>>,
}

impl EstadoMilitar {
    /// Da de alta un héroe (con su residencia, si ya la tiene).
    pub fn insertar(&mut self, h: HeroeNucleo) {
        if let Some(Residencia::Asentamiento(plaza)) = h.heroe.residencia {
            self.residentes.entry(plaza as u64).or_default().insert(h.heroe.id);
        }
        self.heroes.insert(h.heroe.id, h);
    }

    /// Cambia la residencia de un héroe y mantiene el índice. No suelta la guarnición ni retira la tropa prestada: eso es de quien la cambia.
    pub fn mudar(&mut self, id: u64, nueva: Option<Residencia>) {
        let Some(h) = self.heroes.get_mut(&id) else { return };
        if let Some(Residencia::Asentamiento(plaza)) = h.heroe.residencia {
            if let Some(set) = self.residentes.get_mut(&(plaza as u64)) {
                set.remove(&id);
                if set.is_empty() {
                    self.residentes.remove(&(plaza as u64));
                }
            }
        }
        h.heroe.residencia = nueva;
        if let Some(Residencia::Asentamiento(plaza)) = nueva {
            self.residentes.entry(plaza as u64).or_default().insert(id);
        }
    }

    /// Los héroes que residen en la plaza, por id.
    pub fn residentes_de(&self, plaza: u64) -> impl Iterator<Item = &HeroeNucleo> + '_ {
        self.residentes.get(&plaza).into_iter().flatten().filter_map(|id| self.heroes.get(id))
    }

    /// El héroe de una cuenta, si lo hay (uno por cuenta y partida).
    pub fn heroe_de_cuenta(&self, cuenta: &str) -> Option<&HeroeNucleo> {
        self.heroes.values().find(|h| !h.sintetico && h.cuenta == cuenta)
    }

    /// ¿Tiene la Facción adoptada esa tecnología? Las de arranque las tiene toda Facción desde que nace.
    pub fn tiene_tecnologia(&self, reglas: &militar::Reglas, faccion: u32, tecnologia: &str) -> bool {
        reglas.tecnologias.de_arranque.iter().any(|t| t == tecnologia) || self.tecnologias.get(&faccion).is_some_and(|s| s.contains(tecnologia))
    }

    /// Soldados en pie de las escuadras `ids` que llevan los participantes `participantes` (lo que come una columna y lo que la hace ver más lejos).
    pub(crate) fn soldados_de(&self, participantes: &[u64], ids: &[u64]) -> i64 {
        participantes.iter().filter_map(|h| self.heroes.get(h)).flat_map(|h| h.heroe.escuadras.iter()).filter(|e| ids.contains(&e.id)).map(|e| e.cantidad.max(0)).sum()
    }

    /// La convocatoria en la que va el héroe, si va en alguna.
    pub fn convocatoria_de(&self, heroe: u64) -> Option<&Convocatoria> {
        self.convocatorias.values().find(|c| c.integrantes.iter().any(|i| i.heroe == heroe))
    }
}
