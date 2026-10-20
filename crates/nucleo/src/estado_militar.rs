//! El estado militar dentro del núcleo (Heroe_Y_Tropa, Presencia_Y_Convocatoria, Campamentos_Mercenarios): los héroes con todas sus escuadras
//! (viven en su héroe, estén donde estén), las convocatorias de ejércitos en preparación, los reclutas de los campamentos de mercenarios que
//! alguien ha tocado y las tecnologías que la administración da a cada Facción hasta que exista el sistema de tecnología. Entra entero en el
//! hash y en las fotos (`codec_militar`). Las reglas puras viven en el crate `militar`; aquí solo está lo que el núcleo guarda.
//!
//! Los ids de héroe, escuadra, loadout, columna y convocatoria salen del contador común de la partida (`Estado::siguiente_id`), como los de las
//! plazas: no chocan con ninguno, así que los eventos propios de un héroe (su salida del mundo) usan su id como entidad de la cola. Los de las
//! formaciones y las peticiones de unión llevan una etiqueta en los bits altos (como los del comercio, `estado_comercio`) porque comparten la
//! tabla de generaciones con la columna o el héroe de los que salen.

use crate::estado_comercio::ETIQUETA;
use comercio::mercenarios::CupoDiario;
use comercio::Carga as Mercancia;
use militar::convocatoria::Convocatoria;
use militar::heroe::{Heroe, Residencia};
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
    /// La Facción de la que es ciudadano. Hasta el sistema político la fija la administración (`FijarFaccionDeHeroe`).
    pub faccion: Option<u32>,
    /// El almacén personal (`ALMACEN_PERSONAL`): vive donde se reside y viaja con el héroe al cambiar de base.
    pub almacen_personal: Mercancia,
    /// Lo comprado hoy en el mercado de su campamento de residencia, por bien con cupo (madera y piedra).
    pub cupo: CupoDiario,
    /// La columna en la que es participante: aquella en la que va (`ubicacion = columna`) o la que dejó aparcada a la puerta del lugar donde
    /// entró de visita. Una columna lista a sus participantes y este campo la nombra: se mantienen juntos (`Nucleo::meter_en_columna`).
    pub columna: Option<u64>,
    /// El carro con el que salió del mundo (Presencia_Y_Convocatoria §7): sus escuadras están `fuera` con él, y al conectarse nace su columna con
    /// ese carro. Vacío si no salió con una columna.
    pub carro_fuera: Mercancia,
    /// Héroe de campo del andamiaje de administración (`CrearEjercito`): lleva una columna creada a mano. No cuenta como «el héroe de la cuenta»
    /// (`heroe_de_cuenta`, `crearHeroe`), de modo que una cuenta puede tener varias columnas de escenario además de su héroe.
    pub sintetico: bool,
}

impl HeroeNucleo {
    /// Un héroe sin columna, sin almacén personal y sin cupo gastado.
    pub fn nuevo(heroe: Heroe, cuenta: &str, faccion: Option<u32>) -> Self {
        Self { heroe, cuenta: cuenta.to_owned(), faccion, almacen_personal: Mercancia::new(), cupo: CupoDiario::default(), columna: None, carro_fuera: Mercancia::new(), sintetico: false }
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

    /// ¿Tiene algún residente la plaza? (el paso de minuto de una plaza solo trabaja si lo tiene).
    pub(crate) fn tiene_residentes(&self, plaza: u64) -> bool {
        self.residentes.contains_key(&plaza)
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
