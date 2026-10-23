//! Quién y qué hay dentro de cada plaza y de cada campamento (4H T3c): el núcleo recuerda dónde está cada héroe (dentro de qué plaza, en cuál reside, qué escuadras tiene en su guarnición y qué
//! campamentos tocan su presencia, su residencia o sus préstamos) y cada caravana (de qué plaza es y, si está aparcada, en cuál) y, al cerrar cada versión, marca el interior (`Clase::Interior`) de las
//! plazas y el campamento (`Clase::Campamento`) donde algo de eso cambió: el de antes y el de ahora. Así ninguna orden ni regla tiene que acordarse de avisar de que alguien entró, salió, se mudó, aparcó o
//! pidió un préstamo, y las proyecciones del interior (`proyeccion::interior`) se rehacen exactamente cuando hace falta. La plaza misma (su ficha) no se toca: `FACCION` no recibe nada por ello.
//!
//! Solo se lleva si hay sesiones ([`Nucleo::seguir_pertenencias`]) o se está auditando: el batch y las pruebas del motor no pagan nada. Cuesta lo que tardan en recorrerse los héroes y las caravanas tocados
//! de la versión (unas decenas de ns por cada uno).

use super::*;
use comercio::caravana::Caravana;
use militar::escuadra::Contenedor;
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;

/// Dónde está un héroe respecto de las plazas.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Estancia {
    /// La plaza en la que está dentro ahora.
    dentro: Option<u64>,
    /// El campamento en el que está dentro ahora.
    en_campamento: Option<u32>,
    /// La plaza en la que reside.
    reside: Option<u64>,
    /// Sus escuadras con soldados en la guarnición de la plaza donde reside (`proyeccion::interior::guarnicion`).
    guarnicion: Vec<u64>,
    /// Los campamentos de mercenarios que lo cuentan: donde está dentro, donde reside y los que le prestaron tropa (ordenados, sin repetir).
    campamentos: Vec<u32>,
    /// Sus escuadras prestadas por un campamento: pedir tropa prestada cambia el campamento aunque los campamentos del héroe sigan siendo los mismos.
    prestadas: Vec<u64>,
}

fn estancia(h: &HeroeNucleo) -> Estancia {
    let reside = match h.heroe.residencia {
        Some(Residencia::Asentamiento(a)) => Some(u64::from(a)),
        _ => None,
    };
    let guarnicion = if reside.is_some() { h.heroe.escuadras.iter().filter(|s| s.contenedor == Contenedor::Campamento && s.en_guarnicion && s.con_soldados()).map(|s| s.id).collect() } else { Vec::new() };
    let en_campamento = if let Ubicacion::Mercenarios(c) = h.heroe.presencia.ubicacion { Some(c) } else { None };
    let mut campamentos: Vec<u32> = h.heroe.escuadras.iter().filter_map(|s| s.prestada).collect();
    campamentos.extend(en_campamento);
    campamentos.extend(if let Some(Residencia::Campamento(c)) = h.heroe.residencia { Some(c) } else { None });
    campamentos.sort_unstable();
    campamentos.dedup();
    let prestadas = h.heroe.escuadras.iter().filter(|s| s.prestada.is_some()).map(|s| s.id).collect();
    Estancia { dentro: if let Ubicacion::Asentamiento(a) = h.heroe.presencia.ubicacion { Some(u64::from(a)) } else { None }, en_campamento, reside, guarnicion, campamentos, prestadas }
}

/// La plaza de la que es una caravana y la que la hospeda mientras está aparcada.
fn sitio(c: &Caravana) -> (u64, Option<u64>) {
    (u64::from(c.origen), c.anfitriona.filter(|_| c.estado == EstadoCaravana::Aparcada).map(u64::from))
}

/// Lo que se sabe de dónde está cada héroe y cada caravana.
pub(super) struct Interiores {
    heroes: BTreeMap<u64, Estancia>,
    caravanas: BTreeMap<u64, (u64, Option<u64>)>,
}

impl Interiores {
    pub(super) fn de(e: &Estado) -> Self {
        Interiores { heroes: e.militar.heroes.iter().map(|(&id, h)| (id, estancia(h))).collect(), caravanas: e.comercio.flota.iter().map(|c| (c.id, sitio(c))).collect() }
    }
}

impl Nucleo {
    /// Empieza a llevar quién y qué hay dentro de las plazas (lo hace `seguir_pertenencias` y la auditoría).
    pub(super) fn seguir_interiores(&mut self) {
        self.interiores = Some(Interiores::de(&self.estado));
    }

    /// Marca el interior de las plazas donde cambió algo de lo que dicen los héroes y las caravanas tocados de esta transición. Se llama antes de cerrar la versión.
    pub(super) fn marcar_interiores(&mut self) {
        let Nucleo { estado, interiores, tr, .. } = self;
        let Some(vistos) = interiores else { return };
        let plazas = |a: &Estancia| [a.dentro, a.reside].into_iter().flatten().collect::<Vec<_>>();
        // Las escuadras tocadas pertenecen a un héroe que quizá no se tocó: se revisa también.
        let mut heroes: Vec<u64> = tr.tocados.de(Clase::Heroe).to_vec();
        for s in tr.tocados.de(Clase::Escuadra) {
            heroes.extend(estado.militar.heroes.values().find(|h| h.heroe.escuadra(*s).is_some()).map(|h| h.heroe.id));
        }
        for h in heroes {
            let ahora = estado.militar.heroes.get(&h).map(estancia);
            let antes = match &ahora {
                Some(a) => vistos.heroes.insert(h, a.clone()),
                None => vistos.heroes.remove(&h),
            };
            if antes != ahora {
                tr.tocar_todos(Clase::Interior, antes.iter().chain(&ahora).flat_map(plazas));
                tr.tocar_todos(Clase::Campamento, antes.iter().chain(&ahora).flat_map(|a| a.campamentos.iter().map(|&c| u64::from(c))));
            }
        }
        for c in tr.tocados.de(Clase::Caravana).to_vec() {
            let ahora = estado.comercio.caravana(c).map(sitio);
            let antes = match ahora {
                Some(a) => vistos.caravanas.insert(c, a),
                None => vistos.caravanas.remove(&c),
            };
            if antes != ahora {
                tr.tocar_todos(Clase::Interior, antes.iter().chain(&ahora).flat_map(|&(origen, aparcada)| [Some(origen), aparcada]).flatten());
            }
        }
    }
}
