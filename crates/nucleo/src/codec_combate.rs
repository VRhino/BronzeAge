//! Serialización canónica del estado del combate (`EstadoCombate`) para el hash y las fotos (formato BAN19; de BAN13 a BAN19 es igual que en BAN12, salvo las avanzadas de BAN15). Cada estructura se desestructura
//! **entera** (sin `..`): si el estado gana un campo, esto deja de compilar y obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `bandidos{n u64 · n × campamento} · siguiente_bandidos u64 · reaparecen{n u64 · n × (plaza u64, en i64)}
//! · anillos{n u64 · n × (campamento u32, en i64)} · guarniciones{n u64 · n × (pueblo u64, escuadra, repone_en opt_i64)} · destruidos{n u64 ·
//! n × (héroe u64, k u64, k × instante i64)} · avanzadas{n u64 · n × (avanzada u64, elegidas{k u64 · k × (escalón u8, tropa u8)}, escuadras{k u64 · k × escuadra}, repone_en opt_i64)} ·
//! huérfanas{n u64 · n × avanzada}`.
//! Un campamento de bandidos: `id u64 · posición punto · nivel u8 · origen{u8 · u64: 0 plaza, 1 anillo, 2 veta, 3 administración} · bosque opt_u64`.
//! Una escuadra, como en el estado militar (`codec_militar`); una avanzada, como en la economía de la plaza (`codec_economia`).

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_economia::{escribir_avanzada, leer_avanzada};
use crate::codec_militar::{escuadra_escribir, escuadra_leer, tropa_a_u8, tropa_de_u8};
use crate::estado_combate::{CampamentoBandidos, EstadoCombate, GuarnicionDeAvanzada, GuarnicionDePueblo, OrigenBandidos};

fn origen_escribir(w: &mut Escritor, o: OrigenBandidos) {
    let (tag, valor) = match o {
        OrigenBandidos::Plaza(p) => (0, p),
        OrigenBandidos::Anillo(c) => (1, c as u64),
        OrigenBandidos::Veta(v) => (2, v as u64),
        OrigenBandidos::Admin => (3, 0),
    };
    w.u8(tag);
    w.u64(valor);
}

fn origen_leer(r: &mut Lector) -> Result<OrigenBandidos, ErrorFormato> {
    let (tag, valor) = (r.u8()?, r.u64()?);
    let como_u32 = || u32::try_from(valor).map_err(|_| ErrorFormato("índice de campamento fuera de rango"));
    Ok(match tag {
        0 => OrigenBandidos::Plaza(valor),
        1 => OrigenBandidos::Anillo(como_u32()?),
        2 => OrigenBandidos::Veta(como_u32()?),
        3 => OrigenBandidos::Admin,
        _ => return Err(ErrorFormato("origen de campamento de bandidos desconocido")),
    })
}

pub(crate) fn escribir(w: &mut Escritor, c: &EstadoCombate) {
    let EstadoCombate { bandidos, siguiente_bandidos, reaparecen, anillos, guarniciones, avanzadas, huerfanas, destruidos } = c;
    w.u64(bandidos.len() as u64);
    for b in bandidos.values() {
        let CampamentoBandidos { id, posicion, nivel, origen, bosque } = b;
        w.u64(*id);
        w.punto(*posicion);
        w.u8(*nivel);
        origen_escribir(w, *origen);
        w.opt_u64(bosque.map(u64::from));
    }
    w.u64(*siguiente_bandidos);
    w.u64(reaparecen.len() as u64);
    for (&plaza, &en) in reaparecen {
        w.u64(plaza);
        w.i64(en);
    }
    w.u64(anillos.len() as u64);
    for (&campamento, &en) in anillos {
        w.u32(campamento);
        w.i64(en);
    }
    w.u64(guarniciones.len() as u64);
    for (&pueblo, g) in guarniciones {
        let GuarnicionDePueblo { escuadra, repone_en } = g;
        w.u64(pueblo);
        escuadra_escribir(w, escuadra);
        w.opt_i64(*repone_en);
    }
    w.u64(avanzadas.len() as u64);
    for (&avanzada, g) in avanzadas {
        let GuarnicionDeAvanzada { elegidas, escuadras, repone_en } = g;
        w.u64(avanzada);
        w.u64(elegidas.len() as u64);
        for (&escalon, &tropa) in elegidas {
            w.u8(escalon);
            w.u8(tropa_a_u8(tropa));
        }
        w.u64(escuadras.len() as u64);
        escuadras.iter().for_each(|e| escuadra_escribir(w, e));
        w.opt_i64(*repone_en);
    }
    w.u64(huerfanas.len() as u64);
    huerfanas.values().for_each(|a| escribir_avanzada(w, a));
    w.u64(destruidos.len() as u64);
    for (&heroe, instantes) in destruidos {
        w.u64(heroe);
        w.u64(instantes.len() as u64);
        instantes.iter().for_each(|&t| w.i64(t));
    }
}

pub(crate) fn leer(r: &mut Lector) -> Result<EstadoCombate, ErrorFormato> {
    let mut c = EstadoCombate::default();
    for _ in 0..r.cuenta()? {
        let (id, posicion, nivel) = (r.u64()?, r.punto()?, r.u8()?);
        let origen = origen_leer(r)?;
        let bosque = r.opt_u64()?.map(|b| u32::try_from(b).map_err(|_| ErrorFormato("índice de bosque fuera de rango"))).transpose()?;
        c.bandidos.insert(id, CampamentoBandidos { id, posicion, nivel, origen, bosque });
    }
    c.siguiente_bandidos = r.u64()?;
    for _ in 0..r.cuenta()? {
        c.reaparecen.insert(r.u64()?, r.i64()?);
    }
    for _ in 0..r.cuenta()? {
        c.anillos.insert(r.u32()?, r.i64()?);
    }
    for _ in 0..r.cuenta()? {
        let pueblo = r.u64()?;
        let escuadra = escuadra_leer(r)?;
        c.guarniciones.insert(pueblo, GuarnicionDePueblo { escuadra, repone_en: r.opt_i64()? });
    }
    for _ in 0..r.cuenta()? {
        let avanzada = r.u64()?;
        let mut g = GuarnicionDeAvanzada::default();
        for _ in 0..r.cuenta()? {
            let escalon = r.u8()?;
            g.elegidas.insert(escalon, tropa_de_u8(r.u8()?)?);
        }
        for _ in 0..r.cuenta()? {
            g.escuadras.push(escuadra_leer(r)?);
        }
        g.repone_en = r.opt_i64()?;
        c.avanzadas.insert(avanzada, g);
    }
    for _ in 0..r.cuenta()? {
        let a = leer_avanzada(r)?;
        c.huerfanas.insert(a.id, a);
    }
    for _ in 0..r.cuenta()? {
        let heroe = r.u64()?;
        let mut instantes = Vec::new();
        for _ in 0..r.cuenta()? {
            instantes.push(r.i64()?);
        }
        c.destruidos.insert(heroe, instantes);
    }
    Ok(c)
}
