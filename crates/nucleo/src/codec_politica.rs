//! Serialización canónica del estado político (`politica::Politica`) para el hash y las fotos (desde BAN28, tanda 5B). Cada estructura se desestructura **entera**
//! (sin `..`): si el crate `politica` añade un campo, esto deja de compilar y obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `facciones{n u64 · n × facción} · salidas{n u64 · n × (héroe u64, instante i64)} · relaciones{n u64 · n × relación} ·
//! propuestas{n u64 · n × propuesta} · títulos{n u64 · n × (título u8 · poseedor u32 · valor i64)} · mayor_desaparecida u32` (desde BAN33, 5F: el título es su posición en
//! `TituloId::TODOS`).
//!
//! Una facción: `id u32 · nombre texto · sigilo{forma, campo, emblema, color_primario, color_secundario, color_emblema, orla, color_orla: 8 × texto} · rey opt_u64 ·
//! embajador opt_u64 · capital_designada_en opt_i64 · experiencia i64 · ciudadanos{n u64 · n × u64} (en orden de ingreso) · solicitudes{n u64 · n × u64} ·
//! derrota{u8: 0 ninguna, 1 por una Facción (· u32), 2 colapso} · admite_otras_en_ataques bool`.
//! Una relación: `id u64 · tipo u8 (0 vasallaje, 1 alianza, 2 guerra) · a u32 · b u32 · tributo · creada_en i64 · estado u8 (0 activa, 1 rota) · rota_por opt_u64 ·
//! rota_en opt_i64 · paz_propuesta_por opt_u64`, con un tributo `hay bool · recurso u8 · cantidad_por_minuto i64`.
//! Una propuesta: `id u64 · clase{u8: 0 relación (pacto u8: 0 alianza, 1 vasallaje · a u32 · b u32 · tributo), 1 anexión (absorbente u32 · absorbida u32), 2 fusión (a u32 · b u32 ·
//! nombre texto · rey u64)} · propuesta_por u64 · creada_en i64 · expira_en i64`.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_economia::{recurso_a_u8, recurso_de_u8};
use politica::faccion::{Derrota, Faccion, Facciones};
use politica::propuestas::{Clase, Pacto, Propuesta, Propuestas};
use politica::relaciones::{EstadoRelacion, RelacionPolitica, Relaciones, TipoRelacion, Tributo};
use politica::sigilo::Sigilo;
use politica::titulos::{Titulo, TituloId};
use politica::Politica;

fn ids_escribir(w: &mut Escritor, v: &[u64]) {
    w.u64(v.len() as u64);
    v.iter().for_each(|&x| w.u64(x));
}

fn ids_leer(r: &mut Lector) -> Result<Vec<u64>, ErrorFormato> {
    (0..r.cuenta()?).map(|_| r.u64()).collect()
}

fn opt_u32_leer(r: &mut Lector) -> Result<Option<u32>, ErrorFormato> {
    r.opt_u64()?.map(|x| u32::try_from(x).map_err(|_| ErrorFormato("id de Facción fuera de rango"))).transpose()
}

pub(crate) fn sigilo_escribir(w: &mut Escritor, s: &Sigilo) {
    let Sigilo { forma, campo, emblema, color_primario, color_secundario, color_emblema, orla, color_orla } = s;
    [forma, campo, emblema, color_primario, color_secundario, color_emblema, orla, color_orla].into_iter().for_each(|x| w.texto(x));
}

pub(crate) fn sigilo_leer(r: &mut Lector) -> Result<Sigilo, ErrorFormato> {
    Ok(Sigilo {
        forma: r.texto()?,
        campo: r.texto()?,
        emblema: r.texto()?,
        color_primario: r.texto()?,
        color_secundario: r.texto()?,
        color_emblema: r.texto()?,
        orla: r.texto()?,
        color_orla: r.texto()?,
    })
}

fn faccion_escribir(w: &mut Escritor, f: &Faccion) {
    let Faccion { id, nombre, sigilo, rey, embajador, capital_designada_en, experiencia, ciudadanos, solicitudes, derrota, admite_otras_en_ataques } = f;
    w.u32(*id);
    w.texto(nombre);
    sigilo_escribir(w, sigilo);
    w.opt_u64(*rey);
    w.opt_u64(*embajador);
    w.opt_i64(*capital_designada_en);
    w.i64(*experiencia);
    ids_escribir(w, ciudadanos);
    ids_escribir(w, solicitudes);
    match derrota {
        None => w.u8(0),
        Some(Derrota::Por(g)) => {
            w.u8(1);
            w.u32(*g);
        }
        Some(Derrota::Colapso) => w.u8(2),
    }
    w.bool(*admite_otras_en_ataques);
}

fn faccion_leer(r: &mut Lector) -> Result<Faccion, ErrorFormato> {
    Ok(Faccion {
        id: r.u32()?,
        nombre: r.texto()?,
        sigilo: sigilo_leer(r)?,
        rey: r.opt_u64()?,
        embajador: r.opt_u64()?,
        capital_designada_en: r.opt_i64()?,
        experiencia: r.i64()?,
        ciudadanos: ids_leer(r)?,
        solicitudes: ids_leer(r)?,
        derrota: match r.u8()? {
            0 => None,
            1 => Some(Derrota::Por(r.u32()?)),
            2 => Some(Derrota::Colapso),
            _ => return Err(ErrorFormato("derrota de una Facción desconocida")),
        },
        admite_otras_en_ataques: r.bool()?,
    })
}

fn tributo_escribir(w: &mut Escritor, t: &Option<Tributo>) {
    w.bool(t.is_some());
    if let Some(Tributo { recurso, cantidad_por_minuto }) = t {
        w.u8(recurso_a_u8(*recurso));
        w.i64(*cantidad_por_minuto);
    }
}

fn tributo_leer(r: &mut Lector) -> Result<Option<Tributo>, ErrorFormato> {
    if !r.bool()? {
        return Ok(None);
    }
    Ok(Some(Tributo { recurso: recurso_de_u8(r.u8()?)?, cantidad_por_minuto: r.i64()? }))
}

fn relacion_escribir(w: &mut Escritor, x: &RelacionPolitica) {
    let RelacionPolitica { id, tipo, a, b, tributo, creada_en, estado, rota_por, rota_en, paz_propuesta_por } = x;
    w.u64(*id);
    w.u8(match tipo {
        TipoRelacion::Vasallaje => 0,
        TipoRelacion::Alianza => 1,
        TipoRelacion::Guerra => 2,
    });
    w.u32(*a);
    w.u32(*b);
    tributo_escribir(w, tributo);
    w.i64(*creada_en);
    w.u8(match estado {
        EstadoRelacion::Activa => 0,
        EstadoRelacion::Rota => 1,
    });
    w.opt_u64(rota_por.map(u64::from));
    w.opt_i64(*rota_en);
    w.opt_u64(paz_propuesta_por.map(u64::from));
}

fn relacion_leer(r: &mut Lector) -> Result<RelacionPolitica, ErrorFormato> {
    Ok(RelacionPolitica {
        id: r.u64()?,
        tipo: match r.u8()? {
            0 => TipoRelacion::Vasallaje,
            1 => TipoRelacion::Alianza,
            2 => TipoRelacion::Guerra,
            _ => return Err(ErrorFormato("tipo de relación desconocido")),
        },
        a: r.u32()?,
        b: r.u32()?,
        tributo: tributo_leer(r)?,
        creada_en: r.i64()?,
        estado: match r.u8()? {
            0 => EstadoRelacion::Activa,
            1 => EstadoRelacion::Rota,
            _ => return Err(ErrorFormato("estado de relación desconocido")),
        },
        rota_por: opt_u32_leer(r)?,
        rota_en: r.opt_i64()?,
        paz_propuesta_por: opt_u32_leer(r)?,
    })
}

fn propuesta_escribir(w: &mut Escritor, p: &Propuesta) {
    let Propuesta { id, clase, propuesta_por, creada_en, expira_en } = p;
    w.u64(*id);
    match clase {
        Clase::Relacion { pacto, a, b, tributo } => {
            w.u8(0);
            w.u8(match pacto {
                Pacto::Alianza => 0,
                Pacto::Vasallaje => 1,
            });
            w.u32(*a);
            w.u32(*b);
            tributo_escribir(w, tributo);
        }
        Clase::Anexion { absorbente, absorbida } => {
            w.u8(1);
            w.u32(*absorbente);
            w.u32(*absorbida);
        }
        Clase::Fusion { a, b, nombre, rey } => {
            w.u8(2);
            w.u32(*a);
            w.u32(*b);
            w.texto(nombre);
            w.u64(*rey);
        }
    }
    w.u64(*propuesta_por);
    w.i64(*creada_en);
    w.i64(*expira_en);
}

fn propuesta_leer(r: &mut Lector) -> Result<Propuesta, ErrorFormato> {
    let id = r.u64()?;
    let clase = match r.u8()? {
        0 => {
            let pacto = match r.u8()? {
                0 => Pacto::Alianza,
                1 => Pacto::Vasallaje,
                _ => return Err(ErrorFormato("pacto de una propuesta desconocido")),
            };
            Clase::Relacion { pacto, a: r.u32()?, b: r.u32()?, tributo: tributo_leer(r)? }
        }
        1 => Clase::Anexion { absorbente: r.u32()?, absorbida: r.u32()? },
        2 => Clase::Fusion { a: r.u32()?, b: r.u32()?, nombre: r.texto()?, rey: r.u64()? },
        _ => return Err(ErrorFormato("clase de propuesta desconocida")),
    };
    Ok(Propuesta { id, clase, propuesta_por: r.u64()?, creada_en: r.i64()?, expira_en: r.i64()? })
}

pub(crate) fn escribir(w: &mut Escritor, p: &Politica) {
    let Politica { facciones, relaciones, propuestas, titulos, mayor_desaparecida } = p;
    w.u64(facciones.iter().count() as u64);
    facciones.iter().for_each(|f| faccion_escribir(w, f));
    w.u64(facciones.salidas().len() as u64);
    for (&h, &t) in facciones.salidas() {
        w.u64(h);
        w.i64(t);
    }
    w.u64(relaciones.iter().count() as u64);
    relaciones.iter().for_each(|x| relacion_escribir(w, x));
    w.u64(propuestas.iter().count() as u64);
    propuestas.iter().for_each(|x| propuesta_escribir(w, x));
    w.u64(titulos.len() as u64);
    for &Titulo { id, poseedor, valor } in titulos {
        w.u8(id.indice() as u8);
        w.u32(poseedor);
        w.i64(valor);
    }
    w.u32(*mayor_desaparecida);
}

pub(crate) fn leer(r: &mut Lector) -> Result<Politica, ErrorFormato> {
    let facciones: Vec<Faccion> = (0..r.cuenta()?).map(|_| faccion_leer(r)).collect::<Result<_, _>>()?;
    let salidas = (0..r.cuenta()?).map(|_| Ok((r.u64()?, r.i64()?))).collect::<Result<_, ErrorFormato>>()?;
    let relaciones: Vec<RelacionPolitica> = (0..r.cuenta()?).map(|_| relacion_leer(r)).collect::<Result<_, _>>()?;
    let propuestas: Vec<Propuesta> = (0..r.cuenta()?).map(|_| propuesta_leer(r)).collect::<Result<_, _>>()?;
    let titulos = (0..r.cuenta()?)
        .map(|_| Ok(Titulo { id: TituloId::de_indice(u64::from(r.u8()?)).ok_or(ErrorFormato("título desconocido"))?, poseedor: r.u32()?, valor: r.i64()? }))
        .collect::<Result<_, ErrorFormato>>()?;
    Ok(Politica { facciones: Facciones::desde(facciones, salidas), relaciones: Relaciones::desde(relaciones), propuestas: Propuestas::desde(propuestas), titulos, mayor_desaparecida: r.u32()? })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use asentamiento::Recurso;

    #[test]
    fn la_politica_da_la_vuelta_entera() {
        let reglas = politica::Reglas::default();
        let mut fs = Facciones::default();
        fs.crear(&reglas, 1, 10, "Micenas", None, 5).unwrap();
        fs.otorgar(1, 11).unwrap();
        fs.asignar_embajador(1, 11).unwrap();
        fs.crear(&reglas, 2, 20, "Troya", None, 5).unwrap();
        fs.solicitar(2, 30, false).unwrap();
        fs.registrar_derrota(2, Some(1));
        fs.crear_vacia(&reglas, 3);
        fs.registrar_derrota(3, None);
        fs.otorgar(3, 40).unwrap();
        fs.dejar(40, 77, false).unwrap();
        let tributo = Some(Tributo { recurso: Recurso::Trigo, cantidad_por_minuto: 2_500 });
        let relaciones = Relaciones::desde([
            RelacionPolitica { id: 50, tipo: TipoRelacion::Vasallaje, a: 1, b: 2, tributo, creada_en: 9, estado: EstadoRelacion::Activa, rota_por: None, rota_en: None, paz_propuesta_por: None },
            RelacionPolitica { id: 51, tipo: TipoRelacion::Alianza, a: 1, b: 3, tributo: None, creada_en: 9, estado: EstadoRelacion::Rota, rota_por: Some(3), rota_en: Some(12), paz_propuesta_por: Some(1) },
        ]);
        let propuestas = Propuestas::desde([
            Propuesta { id: 60, clase: Clase::Relacion { pacto: Pacto::Alianza, a: 2, b: 3, tributo }, propuesta_por: 20, creada_en: 1, expira_en: 2 },
            Propuesta { id: 61, clase: Clase::Anexion { absorbente: 1, absorbida: 2 }, propuesta_por: 10, creada_en: 1, expira_en: 2 },
            Propuesta { id: 62, clase: Clase::Fusion { a: 1, b: 3, nombre: "Argos".into(), rey: 10 }, propuesta_por: 10, creada_en: 1, expira_en: 2 },
        ]);
        let titulos = vec![Titulo { id: TituloId::FaccionMasGrande, poseedor: 1, valor: 3 }, Titulo { id: TituloId::GranRey, poseedor: 2, valor: 1 }];
        let p = Politica { facciones: fs, relaciones, propuestas, titulos, mayor_desaparecida: 7 };
        let mut w = Escritor::default();
        escribir(&mut w, &p);
        let mut r = Lector::nuevo(&w.0);
        assert_eq!(leer(&mut r).unwrap(), p);
        r.fin().unwrap();
    }
}
