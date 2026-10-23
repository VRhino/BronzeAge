//! Los ids de las entidades en el contrato 05: todos `<prefijo>-<n>`, con `n` el id del núcleo (la Facción también: `f-3`). Cada entidad tiene su función y la inversa
//! (`parse_*`, `None` si el texto no es de esa entidad). Las que solo existen dentro de una plaza llevan la plaza (`ed-<plaza>-<n>`, `recinto-<plaza>-<n>`). El id de una
//! batalla no es de aquí: es un UUID (`Nucleo::uuid_de_batalla`).

macro_rules! ids {
    ($($tipo:ty, $prefijo:literal, $nombre:ident, $parse:ident;)*) => {$(
        pub fn $nombre(n: $tipo) -> String {
            format!(concat!($prefijo, "-{}"), n)
        }
        pub fn $parse(s: &str) -> Option<$tipo> {
            s.strip_prefix(concat!($prefijo, "-"))?.parse().ok()
        }
    )*};
}

ids! {
    u32, "f", faccion, parse_faccion;
    u64, "as", asentamiento, parse_asentamiento;
    u64, "heroe", heroe, parse_heroe;
    u64, "esc", escuadra, parse_escuadra;
    u64, "ej", ejercito, parse_ejercito;
    u64, "car", caravana, parse_caravana;
    u64, "conv", convocatoria, parse_convocatoria;
    u32, "camp", campamento, parse_campamento;
    u64, "band", bandidos, parse_bandidos;
    u64, "acu", acuerdo, parse_acuerdo;
    u64, "ord", orden_de_mercado, parse_orden_de_mercado;
    u64, "pueblo", pueblo, parse_pueblo;
    u64, "av", avanzada, parse_avanzada;
    u64, "obra", obra, parse_obra;
    u64, "loadout", loadout, parse_loadout;
    u32, "nodo", nodo, parse_nodo;
}

/// Una relación entre dos Facciones (`rel-<menor>-<mayor>`).
pub fn relacion(a: u32, b: u32) -> String {
    format!("rel-{}-{}", a.min(b), a.max(b))
}

/// Un informe de inspección (`insp-<héroe>-<objetivo>`, con el id del contrato del objetivo: `ej-N`, `car-N` o `as-N`): uno por héroe y objetivo.
pub fn informe(heroe: u64, objetivo: &str) -> String {
    format!("insp-{heroe}-{objetivo}")
}

pub fn edificio(plaza: u64, n: u32) -> String {
    format!("ed-{plaza}-{n}")
}

pub fn recinto(plaza: u64, n: u32) -> String {
    format!("recinto-{plaza}-{n}")
}

/// `(plaza, n)` de `ed-<plaza>-<n>`.
pub fn parse_edificio(s: &str) -> Option<(u64, u32)> {
    parse_en_plaza(s.strip_prefix("ed-")?)
}

/// `(plaza, n)` de `recinto-<plaza>-<n>`.
pub fn parse_recinto(s: &str) -> Option<(u64, u32)> {
    parse_en_plaza(s.strip_prefix("recinto-")?)
}

fn parse_en_plaza(resto: &str) -> Option<(u64, u32)> {
    let (plaza, n) = resto.split_once('-')?;
    Some((plaza.parse().ok()?, n.parse().ok()?))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn cada_id_da_la_vuelta_y_no_se_confunde_con_otro() {
        assert_eq!(faccion(3), "f-3");
        assert_eq!(parse_faccion("f-3"), Some(3));
        assert_eq!(parse_ejercito(&ejercito(7)), Some(7));
        assert_eq!(parse_heroe(&heroe(7)), Some(7));
        assert_eq!(parse_pueblo(&pueblo(9)), Some(9));
        assert_eq!(parse_avanzada(&avanzada(9)), Some(9));
        assert_eq!(parse_caravana(&caravana(2)), Some(2));
        assert_eq!(parse_campamento(&campamento(4)), Some(4));
        assert_eq!(parse_nodo(&nodo(12)), Some(12));
        assert_eq!(parse_edificio(&edificio(5, 11)), Some((5, 11)));
        assert_eq!(parse_recinto(&recinto(5, 2)), Some((5, 2)));
        assert_eq!(informe(3, &ejercito(5)), "insp-3-ej-5");
        // Un prefijo no se toma por otro (`as-` no es `av-`; `ej-` no es `esc-`) ni se acepta basura.
        assert_eq!(parse_avanzada("as-9"), None);
        assert_eq!(parse_escuadra("ej-1"), None);
        assert_eq!(parse_ejercito("ej-"), None);
        assert_eq!(parse_ejercito("ej-1x"), None);
        assert_eq!(parse_edificio("ed-5"), None);
        assert_eq!(parse_recinto(&edificio(5, 1)), None);
    }
}
