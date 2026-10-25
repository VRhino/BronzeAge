//! A qué ámbitos de jugador pertenece cada entidad del núcleo (contrato 05 §3, 4H T3b): **quién recibe qué**. Es una función pura del estado; el núcleo la evalúa al cerrar cada versión
//! (`nucleo/pertenencias.rs`) y deja el resultado en el registro de cambios, de modo que una entidad que desaparece, o que deja de ser de un ámbito, sigue sabiendo a quién avisar con `QUITA`.
//!
//! | Clase | Ámbitos |
//! |---|---|
//! | `Heroe`, `Escuadra` | `HEROE` del héroe (si es el de su cuenta) |
//! | `Ejercito` | `HEROE` de cada participante (la `VISTA` no sale de aquí: la decide la niebla en cada muestreo, `proyeccion::vista`) |
//! | `Caravana` | `FACCION` de su plaza de origen; `HEROE` de los participantes de la columna a la que va adjunta y de quienes ceden escolta. La de Fundación: `FACCION` de su Facción y `HEROE` de su titular y de quienes van en su columna |
//! | `Batalla` | su propio ámbito `BATALLA` (al que tienen derecho los que combaten en ella, `sesiones/ambitos.rs`) y `FACCION` de las Facciones de los dos bandos y de quienes combaten (el asedio visible) |
//! | `Asentamiento`, `Avanzada`, `OrdenMercado`, `Convocatoria` | `FACCION` del dueño (de la plaza madre, de la plaza o de la Facción de la convocatoria) |
//! | `Pueblo` | `FACCION` del dueño y `ASENTAMIENTO` de su plaza madre (con su almacén local, que el paso de minuto refresca por la secuencia interior de la plaza, `Mesa::repartir_interior`, no por este registro) |
//! | `Acuerdo` | `FACCION` de las dos plazas |
//! | `Faccion` | `FACCION` de la propia |
//! | `Relacion` | `FACCION` de las dos Facciones, mientras exista |
//! | `Propuesta` | `FACCION` de las dos Facciones de su par, mientras esté en el almacén (5H) |
//!
//! La convocatoria va a la Facción entera y no a `HEROE` (solo puede unirse quien es de la Facción: Presencia_Y_Convocatoria §9.6). Una entidad puede estar en dos ámbitos a la vez (una caravana
//! adjunta a tu columna, en `HEROE` y en `FACCION`) y `QUITA` significa «ya no es de este ámbito».
//!
//! El héroe de una sesión es el de verdad de la cuenta y, si solo tiene héroes de campo del andamiaje de administración, el primero ([`heroe_de_sesion`]): los demás héroes de la cuenta
//! no tienen ámbito `HEROE`, pero sus columnas se ven en el del héroe de la sesión.

use super::ambito::Ambito;
use crate::estado::{Clase, Estado};
use crate::estado_militar::HeroeNucleo;
use std::collections::BTreeSet;

/// Las clases cuyas entidades viajan a un ámbito de jugador, en el orden en que se proyectan.
pub const CLASES: [Clase; 14] = [
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
    Clase::Propuesta,
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
            } else if let Some(c) = e.fundacion.caravanas.get(&id) {
                // La Caravana de Fundación (5G-b): su Facción, su titular y quienes van en la columna que la lleva.
                let en_columna = match c.situacion {
                    crate::estado_fundacion::Situacion::Adjunta(x) => e.ejercitos.get(&x).map(|x| x.columna.participantes.iter().map(|p| p.heroe).collect()).unwrap_or_default(),
                    _ => Vec::new(),
                };
                v = heroes_de(e, en_columna.into_iter().chain([c.titular]));
                v.extend(c.faccion.map(Ambito::Faccion));
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
        Clase::Faccion => v.extend(existe_faccion(e, id as u32).then_some(Ambito::Faccion(id as u32))),
        Clase::Relacion => v.extend(e.politica.relaciones.get(id).into_iter().flat_map(|r| [Ambito::Faccion(r.a), Ambito::Faccion(r.b)])),
        Clase::Propuesta => v.extend(e.politica.propuestas.get(id).into_iter().flat_map(|p| {
            let (a, b) = p.clase.par();
            [Ambito::Faccion(a), Ambito::Faccion(b)]
        })),
        // Los títulos son públicos: van a la `VISTA` de todos (la decide `proyeccion::vista`) y a `ADMIN`, no a un ámbito de jugador.
        Clase::Campamento | Clase::Bandidos | Clase::Interior | Clase::Titulo => {}
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// Las Facciones que se nombran en el mundo: las de la entidad (5B: aunque no les quede nadie ni nada) y las que la administración nombra sin crearlas, por su reputación, tecnologías,
/// relaciones (también las rotas: el id de una Facción no se vuelve a dar, ni el de la que desapareció en una anexión o una fusión, 5F), ciudadanos o plazas. Si una existe todavía lo dice
/// [`existe_faccion`].
pub fn facciones(e: &Estado) -> BTreeSet<u32> {
    e.politica
        .facciones
        .iter()
        .map(|f| &f.id)
        .chain(e.comercio.reputacion.keys())
        .chain(e.militar.tecnologias.keys())
        .copied()
        .chain(e.politica.relaciones.iter().flat_map(|r| [r.a, r.b]))
        .chain(e.militar.heroes.values().filter_map(|h| h.faccion))
        .chain(e.asentamientos.values().map(|a| a.faccion))
        .collect()
}

/// ¿Existe la Facción `f`? Como entidad, o porque la nombra algo (lo de [`facciones`], sin recorrerlo todo si es una entidad). La que desaparece en una anexión o una fusión ya no existe: sale
/// del registro con `QUITA`.
pub fn existe_faccion(e: &Estado, f: u32) -> bool {
    e.politica.facciones.existe(f)
        || e.comercio.reputacion.contains_key(&f)
        || e.militar.tecnologias.contains_key(&f)
        || e.politica.relaciones.activas().any(|r| r.incluye(f))
        || e.militar.heroes.values().any(|h| h.faccion == Some(f))
        || e.asentamientos.values().any(|a| a.faccion == f)
}

/// Los ids de todas las entidades de una clase (menos `Escuadra`, que se recorre desde sus héroes).
pub fn todas(e: &Estado, clase: Clase) -> Vec<u64> {
    match clase {
        Clase::Ejercito => e.ejercitos.keys().copied().collect(),
        Clase::Asentamiento => e.asentamientos.keys().copied().collect(),
        Clase::Faccion => facciones(e).into_iter().map(u64::from).collect(),
        Clase::Heroe => e.militar.heroes.keys().copied().collect(),
        Clase::Escuadra => unreachable!("las escuadras se recorren desde sus héroes"),
        Clase::Caravana => e.comercio.flota.iter().map(|c| c.id).chain(e.fundacion.caravanas.keys().copied()).collect(),
        Clase::Acuerdo => e.comercio.acuerdos.iter().map(|a| a.id).collect(),
        Clase::OrdenMercado => e.comercio.libro.ordenes.iter().chain(&e.comercio.libro.historial).map(|o| o.id).collect(),
        Clase::Pueblo => e.asentamientos.values().flat_map(|a| a.economia.pueblos.iter().map(|p| p.id)).collect(),
        Clase::Avanzada => e.asentamientos.values().flat_map(|a| a.economia.avanzadas.iter().map(|v| v.id)).collect(),
        Clase::Convocatoria => e.militar.convocatorias.keys().copied().collect(),
        Clase::Batalla => e.batallas.batallas.keys().copied().collect(),
        Clase::Relacion => e.politica.relaciones.activas().map(|r| r.id).collect(),
        Clase::Propuesta => e.politica.propuestas.iter().map(|p| p.id).collect(),
        Clase::Campamento | Clase::Bandidos | Clase::Interior | Clase::Titulo => Vec::new(),
    }
}
