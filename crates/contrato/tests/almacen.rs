//! El almacén de referencia guarda todas las clases del `oneof` de `Entidad` (fase 7: faltaban `NodoRecurso` y `MemoriaFaccion`, y un cliente se caía al recibirlas).

use contrato::almacen::{clave, Almacen};
use contrato::v1::{self as pb, entidad::Cuerpo, mensaje_de_servidor as ms, AmbitoTipo, TipoEntidad};

#[test]
fn guarda_el_nodo_de_recurso_y_la_memoria_de_la_faccion() {
    let nodo = Cuerpo::Nodo(pb::NodoRecurso { id: "nodo-7".into(), ..Default::default() });
    let memoria = Cuerpo::Memoria(pb::MemoriaFaccion { faccion_id: "f-2".into(), version: 3, ..Default::default() });
    assert_eq!(clave(&nodo), (TipoEntidad::NodoRecurso as i32, "nodo-7".into()));
    assert_eq!(clave(&memoria), (TipoEntidad::MemoriaFaccion as i32, "f-2".into()));

    let ambito = Some(pb::Ambito { tipo: AmbitoTipo::AmbitoFaccion as i32, id: "f-2".into() });
    let entidades = [nodo.clone(), memoria.clone()].map(|c| pb::Entidad { cuerpo: Some(c) }).to_vec();
    let mut a = Almacen::default();
    a.aplicar(&ms::Cuerpo::Foto(pb::Foto { ambito, version: 5, entidades, ..Default::default() }));
    let f = &a.ambitos["FACCION:f-2"];
    assert_eq!(f.entidades.get(&clave(&nodo)), Some(&nodo));
    assert_eq!(f.entidades.get(&clave(&memoria)), Some(&memoria));
}
