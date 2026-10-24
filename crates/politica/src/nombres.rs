//! El nombre de una ciudad nueva (Fundacion_Y_Refundacion §8, `nombreDeCiudadLibre`): ciudades reales de Grecia y el Egeo, Chipre, Anatolia, Mesopotamia, el Levante y Egipto, de la
//! Edad del Bronce a Alejandro. Cada partida saca de aquí sin repetir (comparando sin tildes ni mayúsculas); agotadas, la plaza se queda sin nombre. Es presentación: no cambia lo que hace
//! una plaza y no gasta el azar de la partida.

use crate::sigilo::hash32;

/// `NOMBRES_DE_CIUDADES` (`typescript-final:src/engine/nombresDeCiudades.ts`), en su orden.
pub const NOMBRES_DE_CIUDADES: &[&str] = &[
    // Grecia y el Egeo
    "Micenas", "Tirinto", "Pilos", "Orcómeno", "Gla", "Argos", "Esparta", "Atenas", "Corinto", "Eleusis",
    "Lerna", "Asine", "Midea", "Yolcos", "Dimini", "Lefkandi", "Calcis", "Eretria", "Delfos", "Olimpia",
    "Mégara", "Egina", "Naxos", "Delos", "Akrotiri", "Filakopi", "Platea", "Queronea", "Pela", "Olinto",
    "Potidea", "Dodona", "Cnosos", "Festos", "Malia", "Zakros", "Gurnia", "Cidonia",
    // Chipre
    "Enkomi", "Kition", "Salamina", "Pafos",
    // Anatolia
    "Troya", "Hattusa", "Alacahöyük", "Kanesh", "Purushanda", "Tarso", "Mileto", "Éfeso", "Colofón", "Focea",
    "Esmirna", "Halicarnaso", "Cnido", "Sardes", "Gordio", "Pérgamo", "Sinope", "Lámpsaco", "Cícico", "Aspendos",
    "Side", "Janto", "Pátara", "Telmeso", "Milasa", "Ancira", "Pesinunte",
    // Mesopotamia
    "Ur", "Uruk", "Lagash", "Girsu", "Umma", "Nippur", "Kish", "Eridú", "Larsa", "Isin",
    "Babilonia", "Borsippa", "Sippar", "Acad", "Assur", "Nínive", "Calah", "Mari", "Terqa", "Eshnunna",
    "Nagar", "Harrán", "Susa", "Dur-Kurigalzu", "Opis", "Shuruppak", "Adab", "Carquemish",
    // Levante
    "Ugarit", "Biblos", "Tiro", "Sidón", "Arvad", "Sarepta", "Beirut", "Alalakh", "Ebla", "Qatna",
    "Cades", "Damasco", "Hamat", "Alepo", "Hazor", "Meguido", "Jericó", "Jerusalén", "Siquem", "Laquis",
    "Gezer", "Ascalón", "Gaza", "Asdod", "Jaffa", "Samaria", "Betseán", "Dor", "Ecrón", "Aco", "Tadmor",
    // Egipto y Nubia
    "Menfis", "Tebas", "Heliópolis", "Hermópolis", "Abidos", "Ajetatón", "Pi-Ramsés", "Avaris", "Tanis", "Bubastis",
    "Sais", "Mendes", "Busiris", "Buto", "Elefantina", "Edfu", "Nejeb", "Hieracómpolis", "Coptos", "Licópolis",
    "Heracleópolis", "Crocodilópolis", "Naucratis", "Pelusio", "Dendera", "Esna", "Kom Ombo", "Giza", "Buhen", "Kerma", "Napata",
];

/// El nombre como se compara: sin tildes (las vocales acentuadas, la diéresis y la eñe pasan a su letra base, como `normalize('NFD')` sin las marcas) y en minúsculas.
pub fn normalizar(nombre: &str) -> String {
    nombre
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'Á' | 'À' | 'Â' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ñ' | 'Ñ' => 'n',
            'ç' | 'Ç' => 'c',
            c => c,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

/// `nombreDeCiudadLibre`: un nombre de la lista que `usados` (los de las plazas de la partida) no lleve ya, el `hash32(semilla) mod libres` de los libres. El mismo id sobre las mismas
/// plazas da el mismo nombre. `None` si ya se usaron todos.
pub fn ciudad_libre<'a>(usados: impl IntoIterator<Item = &'a str>, semilla: &str) -> Option<&'static str> {
    let ocupados: std::collections::BTreeSet<String> = usados.into_iter().filter(|n| !n.is_empty()).map(normalizar).collect();
    let libres: Vec<&'static str> = NOMBRES_DE_CIUDADES.iter().copied().filter(|n| !ocupados.contains(&normalizar(n))).collect();
    (!libres.is_empty()).then(|| libres[hash32(semilla) as usize % libres.len()])
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/nombresDeCiudades.test.ts`.
    use super::*;

    #[test]
    fn son_al_menos_cien_sin_repetidos_ni_vacios_tampoco_sin_tildes() {
        assert_eq!(NOMBRES_DE_CIUDADES.len(), 159);
        assert!(NOMBRES_DE_CIUDADES.iter().all(|n| n.trim() == *n && !n.is_empty()));
        let unicos: std::collections::BTreeSet<String> = NOMBRES_DE_CIUDADES.iter().map(|n| normalizar(n)).collect();
        assert_eq!(unicos.len(), NOMBRES_DE_CIUDADES.len());
        assert!(unicos.iter().all(|n| n.chars().all(|c| c.is_ascii())), "la normalización quita todas las tildes de la lista");
    }

    #[test]
    fn es_de_la_lista_y_el_mismo_id_sobre_las_mismas_plazas_da_el_mismo_nombre() {
        let n = ciudad_libre([], "as-1").unwrap();
        assert!(NOMBRES_DE_CIUDADES.contains(&n));
        assert_eq!(ciudad_libre([], "as-1"), Some(n));
    }

    #[test]
    fn nunca_da_uno_que_la_partida_ya_use_con_o_sin_tildes_o_mayusculas() {
        let mut usados: Vec<String> = Vec::new();
        for i in 0..NOMBRES_DE_CIUDADES.len() {
            let n = ciudad_libre(usados.iter().map(String::as_str), &format!("as-{i}")).expect("quedan libres");
            assert!(!usados.iter().any(|u| normalizar(u) == normalizar(n)));
            usados.push(n.to_owned());
        }
        assert_eq!(ciudad_libre(usados.iter().map(String::as_str), "as-x"), None, "agotados, sin nombre");
        let primero = NOMBRES_DE_CIUDADES[0].to_uppercase();
        assert_ne!(ciudad_libre([primero.as_str(), ""], "a"), Some(NOMBRES_DE_CIUDADES[0]));
        assert_ne!(ciudad_libre(["efeso"], "a"), Some("Éfeso"));
    }

    #[test]
    fn ids_distintos_reparten_por_toda_la_lista() {
        let distintos: std::collections::BTreeSet<&str> = (0..60).filter_map(|i| ciudad_libre([], &format!("as-{i}"))).collect();
        assert!(distintos.len() > 30);
    }
}
