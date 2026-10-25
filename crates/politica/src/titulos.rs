//! Títulos de prestigio (Diplomacia_Facciones §7, Doc 2.9): sin beneficio mecánico. Se recalculan enteros con lo que el núcleo mide de cada Facción y se
//! comparan con los anteriores para narrar (`titulo.nace`, `titulo.cambia_manos`; nada cuando uno desaparece). Desempata el orden de las Facciones (por id).
//! La cadencia (decisión 12) es del núcleo.

use crate::liga::Liga;
use crate::tipos::FaccionId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TituloId {
    FaccionMasGrande,
    MayorPoderEconomico,
    MayorEjercito,
    GranRey,
    MecenasAedas,
}

impl TituloId {
    /// Todos, en el orden en que se calculan (su posición es el id del registro de cambios del núcleo).
    pub const TODOS: [TituloId; 5] = [TituloId::FaccionMasGrande, TituloId::MayorPoderEconomico, TituloId::MayorEjercito, TituloId::GranRey, TituloId::MecenasAedas];

    /// La posición en [`TituloId::TODOS`].
    pub fn indice(self) -> u64 {
        TituloId::TODOS.iter().position(|&t| t == self).expect("está") as u64
    }

    pub fn de_indice(i: u64) -> Option<TituloId> {
        TituloId::TODOS.get(i as usize).copied()
    }

    /// Id estable del contrato (`Titulo.titulo_id`).
    pub fn clave(self) -> &'static str {
        match self {
            TituloId::FaccionMasGrande => "faccionMasGrande",
            TituloId::MayorPoderEconomico => "mayorPoderEconomico",
            TituloId::MayorEjercito => "mayorEjercito",
            TituloId::GranRey => "granRey",
            TituloId::MecenasAedas => "mecenasAedas",
        }
    }

    pub fn nombre(self) -> &'static str {
        match self {
            TituloId::FaccionMasGrande => "Facción más grande",
            TituloId::MayorPoderEconomico => "Mayor poder económico",
            TituloId::MayorEjercito => "Ejército más grande",
            TituloId::GranRey => "Gran Rey",
            TituloId::MecenasAedas => "Mecenas de los Aedas",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Titulo {
    pub id: TituloId,
    pub poseedor: FaccionId,
    pub valor: i64,
}

/// Lo que se mide de cada Facción.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Medida {
    pub faccion: FaccionId,
    pub plazas: u32,
    /// Población total de sus plazas (desempata la más grande).
    pub poblacion: i64,
    /// Oro sumando los almacenes de sus plazas.
    pub oro: i64,
    /// Soldados de todas las escuadras de **todos sus ciudadanos**, estén donde estén.
    pub soldados: i64,
    /// `épicas cumplidas × 10 + Aedas residentes` (0 hasta el sistema 6).
    pub mecenas: i64,
}

/// `calcularTitulos`. `medidas` en el orden de las Facciones; `ligas` de `liga::ligas`.
pub fn calcular(medidas: &[Medida], ligas: &[Liga]) -> Vec<Titulo> {
    // El primero con el máximo: `max_by_key` se queda con el último a igualdad, así que se recorre al revés.
    let mejor = |clave: &dyn Fn(&Medida) -> (i64, i64)| medidas.iter().rev().max_by_key(|m| clave(m)).copied();
    let Some(grande) = mejor(&|m| (m.plazas as i64, m.poblacion)) else { return vec![] };
    let rica = mejor(&|m| (m.oro, 0)).expect("hay Facciones");
    let ejercito = mejor(&|m| (m.soldados, 0)).expect("hay Facciones");
    let mecenas = mejor(&|m| (m.mecenas, 0)).expect("hay Facciones");
    let mut t = vec![
        Titulo { id: TituloId::FaccionMasGrande, poseedor: grande.faccion, valor: grande.plazas as i64 },
        Titulo { id: TituloId::MayorPoderEconomico, poseedor: rica.faccion, valor: rica.oro },
        Titulo { id: TituloId::MayorEjercito, poseedor: ejercito.faccion, valor: ejercito.soldados },
    ];
    if let Some(g) = ligas.iter().find_map(|l| l.gran_rey) {
        t.push(Titulo { id: TituloId::GranRey, poseedor: g, valor: 1 });
    }
    if mecenas.mecenas > 0 {
        t.push(Titulo { id: TituloId::MecenasAedas, poseedor: mecenas.faccion, valor: mecenas.mecenas });
    }
    t
}

/// Lo que cantan los Aedas al recalcular.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cambio {
    Nace { titulo: TituloId, faccion: FaccionId, valor: i64 },
    CambiaManos { titulo: TituloId, previo: FaccionId, actual: FaccionId },
}

/// `narrarCambiosDeTitulo`.
pub fn narrar(anteriores: &[Titulo], actuales: &[Titulo]) -> Vec<Cambio> {
    actuales
        .iter()
        .filter_map(|a| match anteriores.iter().find(|p| p.id == a.id) {
            None => Some(Cambio::Nace { titulo: a.id, faccion: a.poseedor, valor: a.valor }),
            Some(p) if p.poseedor != a.poseedor => Some(Cambio::CambiaManos { titulo: a.id, previo: p.poseedor, actual: a.poseedor }),
            Some(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/eventosDominioTitulos.test.ts`, más el cálculo.
    use super::*;

    const GRANDE: Titulo = Titulo { id: TituloId::FaccionMasGrande, poseedor: 1, valor: 3 };

    #[test]
    fn un_titulo_nuevo_nace() {
        assert_eq!(narrar(&[], &[GRANDE]), vec![Cambio::Nace { titulo: TituloId::FaccionMasGrande, faccion: 1, valor: 3 }]);
    }

    #[test]
    fn un_titulo_que_cambia_de_poseedor_cambia_de_manos() {
        let ahora = Titulo { poseedor: 2, valor: 5, ..GRANDE };
        assert_eq!(narrar(&[GRANDE], &[ahora]), vec![Cambio::CambiaManos { titulo: TituloId::FaccionMasGrande, previo: 1, actual: 2 }]);
    }

    #[test]
    fn el_mismo_poseedor_no_canta_nada_y_desaparecer_tampoco() {
        assert!(narrar(&[GRANDE], &[Titulo { valor: 4, ..GRANDE }]).is_empty());
        assert!(narrar(&[GRANDE], &[]).is_empty());
    }

    #[test]
    fn calcula_los_titulos_y_desempata_el_orden_de_las_facciones() {
        let m = [
            Medida { faccion: 1, plazas: 2, poblacion: 10, oro: 5, soldados: 7, mecenas: 0 },
            Medida { faccion: 2, plazas: 2, poblacion: 30, oro: 5, soldados: 9, mecenas: 0 },
        ];
        let liga = Liga { miembros: vec![1, 2], tiene_vasallaje: true, gran_rey: Some(2) };
        let t = calcular(&m, &[liga]);
        assert_eq!(
            t,
            vec![
                Titulo { id: TituloId::FaccionMasGrande, poseedor: 2, valor: 2 },
                Titulo { id: TituloId::MayorPoderEconomico, poseedor: 1, valor: 5 },
                Titulo { id: TituloId::MayorEjercito, poseedor: 2, valor: 9 },
                Titulo { id: TituloId::GranRey, poseedor: 2, valor: 1 },
            ],
            "a igual oro, la primera; sin épicas no hay Mecenas"
        );
        assert!(calcular(&[], &[]).is_empty());
        assert_eq!(calcular(&[Medida { faccion: 3, mecenas: 12, ..Default::default() }], &[]).last().unwrap().id, TituloId::MecenasAedas);
    }
}
