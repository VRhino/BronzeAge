//! Los ámbitos de proyección del contrato 05 (§3): lo que el servidor suscribe a cada sesión. Cada uno se nombra por su clase y el id de la entidad que lo ancla, con los ids
//! de [`ids`](super::ids): `HEROE:heroe-3`, `FACCION:f-1`, `ASENTAMIENTO:as-7`, `CAMPAMENTO:camp-2`, `VISTA:f-1` (el mundo visible a esa Facción) y `ADMIN:` (sin id). Esa misma
//! cadena `TIPO:id` es la clave de `Hola.versiones_conocidas`. El ámbito `BATALLA` es de T4 y aún no existe aquí.

use super::ids;
use contrato::v1 as pb;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ambito {
    /// El héroe propio y lo suyo.
    Heroe(u64),
    /// La Facción propia.
    Faccion(u32),
    /// La plaza donde el héroe está dentro.
    Asentamiento(u64),
    /// El campamento de mercenarios donde el héroe está dentro.
    Campamento(u32),
    /// El mundo visible a la Facción, dentro del rectángulo que fija `CambiarVista`. DUDA: el contrato 05 no le daba id; lleva el de la Facción (sin Facción no hay vista hasta la niebla personal de T3d).
    Vista(u32),
    /// Todo el estado, sin niebla (solo sesiones de administración).
    Admin,
}

impl Ambito {
    fn tipo(self) -> pb::AmbitoTipo {
        match self {
            Ambito::Heroe(_) => pb::AmbitoTipo::AmbitoHeroe,
            Ambito::Faccion(_) => pb::AmbitoTipo::AmbitoFaccion,
            Ambito::Asentamiento(_) => pb::AmbitoTipo::AmbitoAsentamiento,
            Ambito::Campamento(_) => pb::AmbitoTipo::AmbitoCampamento,
            Ambito::Vista(_) => pb::AmbitoTipo::AmbitoVista,
            Ambito::Admin => pb::AmbitoTipo::AmbitoAdmin,
        }
    }

    fn id(self) -> String {
        match self {
            Ambito::Heroe(h) => ids::heroe(h),
            Ambito::Faccion(f) | Ambito::Vista(f) => ids::faccion(f),
            Ambito::Asentamiento(a) => ids::asentamiento(a),
            Ambito::Campamento(c) => ids::campamento(c),
            Ambito::Admin => String::new(),
        }
    }

    /// El ámbito en el contrato (`Foto.ambito`, `Delta.ambito`, `Revocado.ambito`).
    pub fn a_contrato(self) -> pb::Ambito {
        pb::Ambito { tipo: self.tipo() as i32, id: self.id() }
    }

    /// La clave de `Hola.versiones_conocidas`: `TIPO:id`.
    pub fn clave(self) -> String {
        format!("{}:{}", self.tipo().as_str_name().trim_start_matches("AMBITO_"), self.id())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn cada_ambito_se_nombra_con_los_ids_del_contrato_y_su_clave_es_tipo_dos_puntos_id() {
        let casos = [
            (Ambito::Heroe(3), pb::AmbitoTipo::AmbitoHeroe, "heroe-3", "HEROE:heroe-3"),
            (Ambito::Faccion(1), pb::AmbitoTipo::AmbitoFaccion, "f-1", "FACCION:f-1"),
            (Ambito::Asentamiento(7), pb::AmbitoTipo::AmbitoAsentamiento, "as-7", "ASENTAMIENTO:as-7"),
            (Ambito::Campamento(2), pb::AmbitoTipo::AmbitoCampamento, "camp-2", "CAMPAMENTO:camp-2"),
            (Ambito::Vista(1), pb::AmbitoTipo::AmbitoVista, "f-1", "VISTA:f-1"),
            (Ambito::Admin, pb::AmbitoTipo::AmbitoAdmin, "", "ADMIN:"),
        ];
        for (ambito, tipo, id, clave) in casos {
            let c = ambito.a_contrato();
            assert_eq!((c.tipo, c.id.as_str(), ambito.clave().as_str()), (tipo as i32, id, clave));
        }
        // La clave del dorado de `Hola` es la del ámbito del héroe 3.
        assert_eq!(Ambito::Heroe(3).clave(), "HEROE:heroe-3");
    }
}
