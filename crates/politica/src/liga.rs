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
    grupos.into_values().filter(|m| m.len() > 1).map(|miembros| con_su_gran_rey(relaciones, miembros)).collect()
}

/// La Liga de unos miembros (por id): si tiene vasallaje y su Gran Rey, la primera señora (por id de relación) que no es vasalla dentro de ella.
fn con_su_gran_rey(relaciones: &Relaciones, miembros: Vec<FaccionId>) -> Liga {
    let mut dentro: Vec<_> = relaciones.activas().filter(|r| r.tipo == TipoRelacion::Vasallaje && miembros.contains(&r.a) && miembros.contains(&r.b)).collect();
    dentro.sort_by_key(|r| r.id);
    let gran_rey = dentro.iter().map(|r| r.a).find(|&s| !dentro.iter().any(|r| r.b == s));
    Liga { tiene_vasallaje: !dentro.is_empty(), gran_rey, miembros }
}

/// La Liga de `f` (la de [`ligas`] que la contiene), sin calcular las demás: lo que proyecta su Facción (5H). `None` si no está en ninguna. `existe` dice qué Facciones cuentan (las
/// entidades, como en [`ligas`]). ponytail: recorre las relaciones activas por cada miembro (O(miembros × relaciones)); un índice por Facción si las Ligas crecen.
pub fn liga_de(relaciones: &Relaciones, f: FaccionId, existe: impl Fn(FaccionId) -> bool) -> Option<Liga> {
    if !existe(f) {
        return None;
    }
    let mut miembros = vec![f];
    let mut i = 0;
    while let Some(&x) = miembros.get(i) {
        for o in relaciones.activas().filter(|r| r.incluye(x)).map(|r| r.otra(x)) {
            if existe(o) && !miembros.contains(&o) {
                miembros.push(o);
            }
        }
        i += 1;
    }
    miembros.sort_unstable();
    (miembros.len() > 1).then(|| con_su_gran_rey(relaciones, miembros))
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

    #[test]
    fn la_liga_de_una_faccion_es_la_de_ligas_que_la_contiene() {
        let fs = facciones(6);
        let mut rs = Relaciones::default();
        rs.crear(&fs, 1, TipoRelacion::Vasallaje, 2, 3, None, 0).unwrap();
        rs.crear(&fs, 2, TipoRelacion::Vasallaje, 1, 2, None, 0).unwrap();
        rs.crear(&fs, 3, TipoRelacion::Guerra, 4, 3, None, 0).unwrap();
        rs.crear(&fs, 4, TipoRelacion::Alianza, 5, 6, None, 0).unwrap();
        let todas = ligas(&rs, fs.iter().map(|f| f.id));
        for f in 1..=6 {
            assert_eq!(liga_de(&rs, f, |x| fs.existe(x)), todas.iter().find(|l| l.miembros.contains(&f)).cloned(), "la Liga de f-{f}");
        }
        assert_eq!(liga_de(&rs, 9, |x| fs.existe(x)), None, "una Facción que no existe no está en ninguna");
        assert_eq!(liga_de(&rs, 5, |x| x != 6 && fs.existe(x)), None, "sin su única socia, sola no es Liga");
    }
}
