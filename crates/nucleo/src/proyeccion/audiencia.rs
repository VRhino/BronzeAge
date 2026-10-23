//! A qué ámbitos de jugador pertenece cada entidad del núcleo (contrato 05 §3, 4H T3b): **quién recibe qué**. Es una función pura del estado; el núcleo la evalúa al cerrar cada versión
//! (`nucleo/pertenencias.rs`) y deja el resultado en el registro de cambios, de modo que una entidad que desaparece, o que deja de ser de un ámbito, sigue sabiendo a quién avisar con `QUITA`.
//!
//! | Clase | Ámbitos |
//! |---|---|
//! | `Heroe`, `Escuadra` | `HEROE` del héroe (si es el de su cuenta) |
//! | `Ejercito` | `HEROE` de cada participante (la `VISTA` no sale de aquí: la decide la niebla en cada muestreo, `proyeccion::vista`) |
//! | `Caravana` | `FACCION` de su plaza de origen; `HEROE` de los participantes de la columna a la que va adjunta y de quienes ceden escolta |
//! | `Batalla` | su propio ámbito `BATALLA` (al que tienen derecho los que combaten en ella, `sesiones/ambitos.rs`) y `FACCION` de las Facciones de los dos bandos y de quienes combaten (el asedio visible) |
//! | `Asentamiento`, `Avanzada`, `OrdenMercado`, `Convocatoria` | `FACCION` del dueño (de la plaza madre, de la plaza o de la Facción de la convocatoria) |
//! | `Pueblo` | `FACCION` del dueño y `ASENTAMIENTO` de su plaza madre (con su almacén local, que el paso de minuto refresca por la secuencia interior de la plaza, `Mesa::repartir_interior`, no por este registro) |
//! | `Acuerdo` | `FACCION` de las dos plazas |
//! | `Faccion` | `FACCION` de la propia |
//! | `Relacion` | `FACCION` de las dos Facciones, mientras exista |
//!
//! DUDA: la convocatoria va a la Facción entera (el canon la reparte a «su lugar y su Facción», pero solo puede unirse quien es de la Facción) y no a `HEROE`; una entidad puede estar en dos
//! ámbitos a la vez (una caravana adjunta a tu columna, en `HEROE` y en `FACCION`) y `QUITA` significa «ya no es de este ámbito».
//!
//! El héroe de una sesión es el de verdad de la cuenta y, si solo tiene héroes de campo del andamiaje de administración, el primero ([`heroe_de_sesion`]): los demás héroes de la cuenta
//! no tienen ámbito `HEROE`, pero sus columnas se ven en el del héroe de la sesión.

use super::ambito::Ambito;
use crate::estado::{Clase, Estado};
use crate::estado_militar::HeroeNucleo;
use std::collections::BTreeSet;

/// Las clases cuyas entidades viajan a un ámbito de jugador, en el orden en que se proyectan.
pub const CLASES: [Clase; 13] = [
    Clase::Ejercito,
    Clase::Asentamiento,
    Clase::Faccion,
    Clase::Heroe,
    Clase::Escuadra,
    Clase::Caravana,
    Clase::Acuerdo,
    Clase::OrdenMercado,
    Clase::Pueblo,
    Clase::Avanzada,
    Clase::Convocatoria,
    Clase::Batalla,
    Clase::Relacion,
];

/// El héroe de la sesión de una cuenta: el de verdad si lo hay y, si no, el primer héroe de campo.
pub fn heroe_de_sesion(e: &Estado, cuenta: &str) -> Option<u64> {
    e.militar.heroes.values().filter(|h| h.cuenta == cuenta).min_by_key(|h| (h.sintetico, h.heroe.id)).map(|h| h.heroe.id)
}

/// El héroe de la sesión de la cuenta de `h`. Un héroe de verdad lo es siempre; solo el de campo obliga a buscar.
pub fn principal(e: &Estado, h: &HeroeNucleo) -> Option<u64> {
    if h.sintetico {
        heroe_de_sesion(e, &h.cuenta)
    } else {
        Some(h.heroe.id)
    }
}

fn heroes_de(e: &Estado, ids: impl IntoIterator<Item = u64>) -> Vec<Ambito> {
    ids.into_iter().filter_map(|h| e.militar.heroes.get(&h)).filter_map(|h| principal(e, h)).map(Ambito::Heroe).collect()
}

fn faccion_de_plaza(e: &Estado, plaza: u64) -> Option<Ambito> {
    e.asentamientos.get(&plaza).map(|a| Ambito::Faccion(a.faccion))
}

/// Los ámbitos a los que pertenece la entidad `(clase, id)` ahora, ordenados y sin repetir. Vacío si ya no existe. `Escuadra` no se pregunta por su id (no se sabe de quién es sin
/// buscarla): su ámbito es el de su dueño, `de(Heroe, dueño)`.
pub fn de(e: &Estado, clase: Clase, id: u64) -> Vec<Ambito> {
    let mut v: Vec<Ambito> = Vec::new();
    match clase {
        Clase::Ejercito => {
            if let Some(x) = e.ejercitos.get(&id) {
                v = heroes_de(e, x.columna.participantes.iter().map(|p| p.heroe));
            }
        }
        Clase::Heroe => v.extend(e.militar.heroes.get(&id).filter(|h| principal(e, h) == Some(id)).map(|_| Ambito::Heroe(id))),
        Clase::Escuadra => unreachable!("el ámbito de una escuadra es el de su dueño"),
        Clase::Caravana => {
            if let Some(c) = e.comercio.caravana(id) {
                let en_columna = e.ejercitos.values().find(|x| x.caravanas_adjuntas.contains(&id)).into_iter().flat_map(|x| x.columna.participantes.iter().map(|p| p.heroe));
                v = heroes_de(e, en_columna.chain(c.escolta.iter().map(|s| s.heroe)));
                v.extend(faccion_de_plaza(e, u64::from(c.origen)));
            }
        }
        Clase::Batalla => {
            if let Some(b) = e.batallas.batallas.get(&id) {
                let combatientes = b.heroes_que_combaten().filter_map(|h| e.militar.heroes.get(&h)).filter_map(|h| h.faccion);
                v.push(Ambito::Batalla(id));
                v.extend(b.ticket.atacante.faccion.into_iter().chain(b.ticket.defensor.faccion).chain(combatientes).map(Ambito::Faccion));
            }
        }
        Clase::Asentamiento => v.extend(faccion_de_plaza(e, id)),
        Clase::Pueblo => {
            if let Some(a) = e.asentamientos.values().find(|a| a.economia.pueblo(id).is_some()) {
                v.extend([Ambito::Faccion(a.faccion), Ambito::Asentamiento(a.id)]);
            }
        }
        Clase::Avanzada => v.extend(e.asentamientos.values().find(|a| a.economia.avanzada(id).is_some()).map(|a| Ambito::Faccion(a.faccion))),
        Clase::OrdenMercado => v.extend(e.comercio.libro.ordenes.iter().chain(&e.comercio.libro.historial).find(|o| o.id == id).and_then(|o| faccion_de_plaza(e, u64::from(o.asentamiento)))),
        Clase::Convocatoria => v.extend(e.militar.convocatorias.get(&id).map(|c| Ambito::Faccion(c.faccion))),
        Clase::Acuerdo => v.extend(e.comercio.acuerdo(id).into_iter().flat_map(|a| [a.a, a.b]).filter_map(|p| faccion_de_plaza(e, u64::from(p)))),
        Clase::Faccion => v.push(Ambito::Faccion(id as u32)),
        Clase::Relacion => {
            let (a, b) = Clase::facciones_del_par(id);
            if e.militar.relacion(a, b).is_some() {
                v.extend([Ambito::Faccion(a), Ambito::Faccion(b)]);
            }
        }
        Clase::Campamento | Clase::Bandidos | Clase::Interior => {}
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// Las Facciones que existen: las que tienen nivel, reputación, tecnologías, relaciones, ciudadanos o plazas.
pub fn facciones(e: &Estado) -> BTreeSet<u32> {
    e.niveles_faccion
        .keys()
        .chain(e.comercio.reputacion.keys())
        .chain(e.militar.tecnologias.keys())
        .chain(e.militar.relaciones.keys().flat_map(|(a, b)| [a, b]))
        .copied()
        .chain(e.militar.heroes.values().filter_map(|h| h.faccion))
        .chain(e.asentamientos.values().map(|a| a.faccion))
        .collect()
}

/// Los ids de todas las entidades de una clase (menos `Escuadra`, que se recorre desde sus héroes).
pub fn todas(e: &Estado, clase: Clase) -> Vec<u64> {
    match clase {
        Clase::Ejercito => e.ejercitos.keys().copied().collect(),
        Clase::Asentamiento => e.asentamientos.keys().copied().collect(),
        Clase::Faccion => facciones(e).into_iter().map(u64::from).collect(),
        Clase::Heroe => e.militar.heroes.keys().copied().collect(),
        Clase::Escuadra => unreachable!("las escuadras se recorren desde sus héroes"),
        Clase::Caravana => e.comercio.flota.iter().map(|c| c.id).collect(),
        Clase::Acuerdo => e.comercio.acuerdos.iter().map(|a| a.id).collect(),
        Clase::OrdenMercado => e.comercio.libro.ordenes.iter().chain(&e.comercio.libro.historial).map(|o| o.id).collect(),
        Clase::Pueblo => e.asentamientos.values().flat_map(|a| a.economia.pueblos.iter().map(|p| p.id)).collect(),
        Clase::Avanzada => e.asentamientos.values().flat_map(|a| a.economia.avanzadas.iter().map(|v| v.id)).collect(),
        Clase::Convocatoria => e.militar.convocatorias.keys().copied().collect(),
        Clase::Batalla => e.batallas.batallas.keys().copied().collect(),
        Clase::Relacion => e.militar.relaciones.keys().map(|&(a, b)| Clase::par(a, b)).collect(),
        Clase::Campamento | Clase::Bandidos | Clase::Interior => Vec::new(),
    }
}
