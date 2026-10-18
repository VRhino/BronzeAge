//! Serialización canónica de la ciudad de una plaza (`trazado::Ciudad`) para el hash y las fotos. Se guarda lo único persistente
//! (semilla, parámetros, piezas en su orden de construcción, recintos); el suelo ocupado y la red de calles se **derivan** al leer
//! (`Ciudad::desde_partes`). Cada estructura se desestructura **entera** (sin `..`): si el crate añade un campo, esto deja de compilar y
//! obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `layout_version u32 · semilla u64 · parámetros{largo_fila_min i32, largo_fila_max i32,
//! radio_afueras_min_ul i32, ancho_banda_afueras_ul i32, separacion_minima_anclas i32, separacion_seguridad_anclas i32, ancho_calle i32,
//! cap_corredor_urbano u16, cap_corredor_afueras u16, fondo_manzana i32, nivel_gate_anclas u8, radio_inicial_ul i32,
//! crecimiento_por_edificio_ul i32, maximo_por_nivel_ul 5 × i32, viviendas_iniciales u32} · piezas{n u64 · n × pieza} ·
//! recintos{n u64 · n × recinto}`. Una pieza: `id u32 · tipo u16 · categoría u8 · rol u8 · rect{col i32, row i32, ancho i32, alto i32} ·
//! rotado bool · ancla_llena bool · semilla_saturada bool`. Un recinto: `muros{n u64 · n × (col i32, row i32)} · puertas{ídem} ·
//! completo bool`.
//! Categoría: Afueras 0, Genérico 1, Residencial 2, Industria 3, Militar 4, Mercado 5. Rol: Normal 0, Centro 1, Ancla primaria 2, Ancla
//! de saturación 3.
//!
//! Una foto con otra `LAYOUT_VERSION` se **rechaza**: las huellas y la red se derivan de la geometría y se reinterpretarían en silencio.

use crate::codec::{ErrorFormato, Escritor, Lector};
use asentamiento::catalogo::TIPOS;
use trazado::{Categoria, Celda, Ciudad, Parametros, Pieza, Recinto, Rect, Rol, LAYOUT_VERSION};

fn categoria_a_u8(c: Categoria) -> u8 {
    match c {
        Categoria::Afueras => 0,
        Categoria::Generico => 1,
        Categoria::Residencial => 2,
        Categoria::Industria => 3,
        Categoria::Militar => 4,
        Categoria::Mercado => 5,
    }
}

fn categoria_de_u8(i: u8) -> Result<Categoria, ErrorFormato> {
    Ok(match i {
        0 => Categoria::Afueras,
        1 => Categoria::Generico,
        2 => Categoria::Residencial,
        3 => Categoria::Industria,
        4 => Categoria::Militar,
        5 => Categoria::Mercado,
        _ => return Err(ErrorFormato("categoría de pieza desconocida")),
    })
}

fn rol_a_u8(r: Rol) -> u8 {
    match r {
        Rol::Normal => 0,
        Rol::Centro => 1,
        Rol::AnclaPrimaria => 2,
        Rol::AnclaSaturacion => 3,
    }
}

fn rol_de_u8(i: u8) -> Result<Rol, ErrorFormato> {
    Ok(match i {
        0 => Rol::Normal,
        1 => Rol::Centro,
        2 => Rol::AnclaPrimaria,
        3 => Rol::AnclaSaturacion,
        _ => return Err(ErrorFormato("rol de pieza desconocido")),
    })
}

fn celda(w: &mut Escritor, c: &Celda) {
    w.i32(c.col);
    w.i32(c.row);
}

fn celdas(w: &mut Escritor, v: &[Celda]) {
    w.u64(v.len() as u64);
    v.iter().for_each(|c| celda(w, c));
}

fn leer_celdas(r: &mut Lector) -> Result<Vec<Celda>, ErrorFormato> {
    let mut v = Vec::new();
    for _ in 0..r.cuenta()? {
        v.push(Celda::new(r.i32()?, r.i32()?));
    }
    Ok(v)
}

pub(crate) fn escribir(w: &mut Escritor, c: &Ciudad) {
    w.u32(LAYOUT_VERSION);
    w.u64(c.semilla());
    let Parametros {
        largo_fila_min,
        largo_fila_max,
        radio_afueras_min_ul,
        ancho_banda_afueras_ul,
        separacion_minima_anclas,
        separacion_seguridad_anclas,
        ancho_calle,
        cap_corredor_urbano,
        cap_corredor_afueras,
        fondo_manzana,
        nivel_gate_anclas,
        radio_inicial_ul,
        crecimiento_por_edificio_ul,
        maximo_por_nivel_ul,
        viviendas_iniciales,
    } = c.params();
    [largo_fila_min, largo_fila_max, radio_afueras_min_ul, ancho_banda_afueras_ul, separacion_minima_anclas, separacion_seguridad_anclas, ancho_calle].iter().for_each(|&&x| w.i32(x));
    w.u16(*cap_corredor_urbano);
    w.u16(*cap_corredor_afueras);
    w.i32(*fondo_manzana);
    w.u8(*nivel_gate_anclas);
    w.i32(*radio_inicial_ul);
    w.i32(*crecimiento_por_edificio_ul);
    maximo_por_nivel_ul.iter().for_each(|&x| w.i32(x));
    w.u32(*viviendas_iniciales);

    w.u64(c.piezas().len() as u64);
    for p in c.piezas() {
        let Pieza { id, tipo, categoria, rol, rect, rotado, ancla_llena, semilla_saturada } = p;
        w.u32(*id);
        w.u16(*tipo);
        w.u8(categoria_a_u8(*categoria));
        w.u8(rol_a_u8(*rol));
        let Rect { col, row, ancho, alto } = rect;
        [col, row, ancho, alto].iter().for_each(|&&x| w.i32(x));
        w.bool(*rotado);
        w.bool(*ancla_llena);
        w.bool(*semilla_saturada);
    }
    w.u64(c.recintos().len() as u64);
    for rec in c.recintos() {
        let Recinto { muros, puertas, completo } = rec;
        celdas(w, muros);
        celdas(w, puertas);
        w.bool(*completo);
    }
}

pub(crate) fn leer(r: &mut Lector) -> Result<Ciudad, ErrorFormato> {
    let version = r.u32()?;
    if version != LAYOUT_VERSION {
        return Err(ErrorFormato("la foto guarda una ciudad con otra revisión de la geometría (LAYOUT_VERSION)"));
    }
    let semilla = r.u64()?;
    let params = Parametros {
        largo_fila_min: r.i32()?,
        largo_fila_max: r.i32()?,
        radio_afueras_min_ul: r.i32()?,
        ancho_banda_afueras_ul: r.i32()?,
        separacion_minima_anclas: r.i32()?,
        separacion_seguridad_anclas: r.i32()?,
        ancho_calle: r.i32()?,
        cap_corredor_urbano: r.u16()?,
        cap_corredor_afueras: r.u16()?,
        fondo_manzana: r.i32()?,
        nivel_gate_anclas: r.u8()?,
        radio_inicial_ul: r.i32()?,
        crecimiento_por_edificio_ul: r.i32()?,
        maximo_por_nivel_ul: [r.i32()?, r.i32()?, r.i32()?, r.i32()?, r.i32()?],
        viviendas_iniciales: r.u32()?,
    };
    // Valores con los que la geometría dividiría por cero o no avanzaría: una foto así está corrupta.
    if params.largo_fila_min < 1 || params.largo_fila_max < params.largo_fila_min || params.ancho_calle < 1 || params.fondo_manzana < 1 || params.separacion_minima_anclas < 1 {
        return Err(ErrorFormato("parámetros de la ciudad inválidos"));
    }
    let mut piezas = Vec::new();
    for _ in 0..r.cuenta()? {
        let (id, tipo) = (r.u32()?, r.u16()?);
        if tipo as usize >= TIPOS.len() {
            return Err(ErrorFormato("tipo de pieza desconocido"));
        }
        let (categoria, rol) = (categoria_de_u8(r.u8()?)?, rol_de_u8(r.u8()?)?);
        let rect = Rect::new(r.i32()?, r.i32()?, r.i32()?, r.i32()?);
        if rect.ancho < 1 || rect.alto < 1 {
            return Err(ErrorFormato("pieza sin superficie"));
        }
        piezas.push(Pieza { id, tipo, categoria, rol, rect, rotado: r.bool()?, ancla_llena: r.bool()?, semilla_saturada: r.bool()? });
    }
    let mut recintos = Vec::new();
    for _ in 0..r.cuenta()? {
        recintos.push(Recinto { muros: leer_celdas(r)?, puertas: leer_celdas(r)?, completo: r.bool()? });
    }
    Ciudad::desde_partes(semilla, params, version, piezas, recintos).map_err(|_| ErrorFormato("la ciudad guardada es incoherente (piezas repetidas, superpuestas o fuera de la ventana)"))
}
