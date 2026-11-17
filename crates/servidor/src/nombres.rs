//! Los nombres de los héroes que viajan con cada mensaje (`MensajeDeServidor.nombres_heroes`, 2026-10-27, pedido por el cliente de jugador).
//!
//! El contrato nombra a los héroes por id (`heroe-N`) en muchos sitios (la Facción, la convocatoria, las columnas, los cargos, los hechos...). En
//! vez de añadir un nombre junto a cada id, la sesión mira los bytes que va a enviar, busca los `heroe-N` que aparecen y añade al final del
//! mensaje el nombre de los que lo tienen. Lo hace la tarea de cada sesión, fuera del hilo del núcleo; el núcleo solo apunta cada nombre nuevo en
//! el directorio compartido, una vez por héroe (los nombres no cambian ni se borran).

use contrato::v1::MensajeDeServidor;
use prost::Message;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

/// El directorio compartido de nombres de héroe, por id numérico. Vacío: ningún mensaje lleva nombres.
#[derive(Clone, Debug, Default)]
pub struct NombresHeroes(Arc<RwLock<HashMap<u64, String>>>);

impl NombresHeroes {
    /// Apunta el nombre del héroe `id`.
    pub fn poner(&self, id: u64, nombre: &str) {
        self.0.write().unwrap_or_else(|e| e.into_inner()).insert(id, nombre.to_owned());
    }

    /// Los nombres de los héroes con nombre cuyo id (`heroe-N`) aparece en `bytes`, por id del contrato.
    pub fn de(&self, bytes: &[u8]) -> BTreeMap<String, String> {
        let ids = ids_de_heroe(bytes);
        if ids.is_empty() {
            return BTreeMap::new();
        }
        let d = self.0.read().unwrap_or_else(|e| e.into_inner());
        ids.into_iter().filter_map(|id| Some((format!("heroe-{id}"), d.get(&id)?.clone()))).collect()
    }

    /// Añade a un `MensajeDeServidor` ya codificado los nombres de los héroes que nombra. Un campo repetido (el mapa) al final de los bytes
    /// es Protobuf válido: el que decodifica lo junta con lo demás.
    pub fn anadir(&self, bytes: &mut Vec<u8>) {
        let nombres = self.de(bytes);
        if !nombres.is_empty() {
            MensajeDeServidor { nombres_heroes: nombres.into_iter().collect(), ..Default::default() }.encode(bytes).expect("un Vec crece lo que haga falta");
        }
    }
}

/// Los `N` de cada `heroe-N` que aparece en `bytes` (sin repetir).
fn ids_de_heroe(bytes: &[u8]) -> std::collections::BTreeSet<u64> {
    const PREFIJO: &[u8] = b"heroe-";
    let mut ids = std::collections::BTreeSet::new();
    let mut i = 0;
    while let Some(p) = bytes[i..].windows(PREFIJO.len()).position(|w| w == PREFIJO) {
        let inicio = i + p + PREFIJO.len();
        let fin = inicio + bytes[inicio..].iter().take_while(|b| b.is_ascii_digit()).count();
        if fin > inicio {
            if let Some(id) = std::str::from_utf8(&bytes[inicio..fin]).ok().and_then(|s| s.parse().ok()) {
                ids.insert(id);
            }
        }
        i = fin.max(inicio);
    }
    ids
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn los_nombres_de_los_heroes_nombrados_van_al_final_y_se_decodifican_con_el_mensaje() {
        let d = NombresHeroes::default();
        d.poner(7, "Arkanthos");
        d.poner(12, "Vrhino");
        assert_eq!(ids_de_heroe(b"\x07heroe-7\x08heroe-12\x07heroe-7heroe-x heroe-"), [7, 12].into());
        let m = MensajeDeServidor { secuencia: 3, ..Default::default() };
        let mut bytes = m.encode_to_vec();
        // Sin héroes nombrados no se añade nada.
        let antes = bytes.clone();
        d.anadir(&mut bytes);
        assert_eq!(bytes, antes);
        // Con un id dentro (aquí, en un aviso cualquiera), su nombre; el héroe sin nombre (el 9) no va.
        let m = MensajeDeServidor {
            secuencia: 4,
            cuerpo: Some(contrato::v1::mensaje_de_servidor::Cuerpo::Aviso(contrato::v1::Aviso { codigo: "heroe-12 y heroe-9".into(), ..Default::default() })),
            ..Default::default()
        };
        let mut bytes = m.encode_to_vec();
        d.anadir(&mut bytes);
        let vuelta = MensajeDeServidor::decode(bytes.as_slice()).unwrap();
        assert_eq!(vuelta.cuerpo, m.cuerpo);
        assert_eq!(vuelta.nombres_heroes, [("heroe-12".to_owned(), "Vrhino".to_owned())].into());
    }
}
