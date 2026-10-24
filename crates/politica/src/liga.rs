//! La Liga y el Gran Rey (Diplomacia_Facciones §5.7, Doc 2.3): no se guardan. Una Liga es una componente conexa (≥ 2 Facciones) del grafo de relaciones
//! **activas** de cualquier clase, la guerra incluida; el Gran Rey es la señora de un vasallaje de la Liga que no es vasalla dentro de ella. Bajo demanda (decisión 13).

use crate::relaciones::{Relaciones, TipoRelacion};
use crate::tipos::FaccionId;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Liga {
    /// Por id.
    pub miembros: Vec<FaccionId>,
    pub tiene_vasallaje: bool,
    pub gran_rey: Option<FaccionId>,
}

fn raiz(padre: &mut BTreeMap<FaccionId, FaccionId>, mut x: FaccionId) -> FaccionId {
    while padre[&x] != x {
        let abuelo = padre[&padre[&x]];
        padre.insert(x, abuelo);
        x = abuelo;
    }
    x
}

/// `computeLigas`, en orden de su miembro menor. El Gran Rey es la primera señora (por id de relación) que no es vasalla en la Liga.
pub fn ligas(relaciones: &Relaciones, facciones: impl IntoIterator<Item = FaccionId>) -> Vec<Liga> {
    let mut padre: BTreeMap<FaccionId, FaccionId> = facciones.into_iter().map(|f| (f, f)).collect();
    let pares: Vec<_> = relaciones.activas().filter(|r| padre.contains_key(&r.a) && padre.contains_key(&r.b)).map(|r| (r.a, r.b)).collect();
    for (a, b) in pares {
        let (ra, rb) = (raiz(&mut padre, a), raiz(&mut padre, b));
        if ra != rb {
            padre.insert(ra.max(rb), ra.min(rb));
        }
    }
    let mut grupos: BTreeMap<FaccionId, Vec<FaccionId>> = BTreeMap::new();
    for f in padre.keys().copied().collect::<Vec<_>>() {
        grupos.entry(raiz(&mut padre, f)).or_default().push(f);
    }
    let mut vasallajes: Vec<_> = relaciones.activas().filter(|r| r.tipo == TipoRelacion::Vasallaje).collect();
    vasallajes.sort_by_key(|r| r.id);
    grupos
        .into_values()
        .filter(|m| m.len() > 1)
        .map(|miembros| {
            let dentro: Vec<_> = vasallajes.iter().filter(|r| miembros.contains(&r.a) && miembros.contains(&r.b)).collect();
            let gran_rey = dentro.iter().map(|r| r.a).find(|&s| !dentro.iter().any(|r| r.b == s));
            Liga { tiene_vasallaje: !dentro.is_empty(), gran_rey, miembros }
        })
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::relaciones::pruebas::facciones;

    #[test]
    fn componentes_conexas_con_la_guerra_incluida_y_el_gran_rey_en_la_cima() {
        let fs = facciones(6);
        let mut rs = Relaciones::default();
        // 1 señora de 2, 2 señora de 3 (antes de ser vasalla); 4 en guerra con 3. 5 y 6, sueltas.
        rs.crear(&fs, 1, TipoRelacion::Vasallaje, 2, 3, None, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Vasallaje, 1, 2, None, 0).unwrap();
        rs.crear(&fs, 3, TipoRelacion::Guerra, 4, 3, None, 0).unwrap();
        let l = ligas(&rs, fs.iter().map(|f| f.id));
        assert_eq!(l, vec![Liga { miembros: vec![1, 2, 3, 4], tiene_vasallaje: true, gran_rey: Some(1) }]);
        // Solo alianzas: sin Gran Rey.
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Alianza, 5, 6, None, 0).unwrap();
        assert_eq!(ligas(&rs, fs.iter().map(|f| f.id)), vec![Liga { miembros: vec![5, 6], tiene_vasallaje: false, gran_rey: None }]);
        assert!(ligas(&Relaciones::default(), fs.iter().map(|f| f.id)).is_empty());
    }
}
