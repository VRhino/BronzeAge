//! El estado militar dentro del núcleo (Heroe_Y_Tropa, Presencia_Y_Convocatoria, Campamentos_Mercenarios): los héroes con todas sus escuadras
//! (viven en su héroe, estén donde estén), los reclutas de los campamentos de mercenarios que alguien ha tocado y las tecnologías que la
//! administración da a cada Facción hasta que exista el sistema de tecnología. Entra entero en el hash y en las fotos (`codec_militar`). Las
//! reglas puras viven en el crate `militar`; aquí solo está lo que el núcleo guarda.
//!
//! Los ids de héroe, escuadra y loadout salen del contador común de la partida (`Estado::siguiente_id`), como los de las columnas y las
//! plazas: no chocan con ninguno, así que el único evento propio de un héroe (su salida del mundo) usa su id como entidad de la cola.

use comercio::mercenarios::CupoDiario;
use comercio::Carga as Mercancia;
use militar::heroe::{Heroe, Residencia};
use militar::mercenarios::Reclutas;
use std::collections::{BTreeMap, BTreeSet};

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
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EstadoMilitar {
    /// Todos los héroes, por id ascendente.
    pub heroes: BTreeMap<u64, HeroeNucleo>,
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
        self.heroes.values().find(|h| h.cuenta == cuenta)
    }

    /// ¿Tiene la Facción adoptada esa tecnología? Las de arranque las tiene toda Facción desde que nace.
    pub fn tiene_tecnologia(&self, reglas: &militar::Reglas, faccion: u32, tecnologia: &str) -> bool {
        reglas.tecnologias.de_arranque.iter().any(|t| t == tecnologia) || self.tecnologias.get(&faccion).is_some_and(|s| s.contains(tecnologia))
    }
}
