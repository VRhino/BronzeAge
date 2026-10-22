//! Serialización canónica del estado militar (`EstadoMilitar`) para el hash y las fotos (formato BAN13, igual que en BAN12 y BAN11). Cada estructura se desestructura
//! **entera** (sin `..`): si el crate `militar` añade un campo, esto deja de compilar y obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `héroes{n u64 · n × héroe} · convocatorias{n u64 · n × convocatoria} · reclutas{n u64 · n ×
//! (campamento u32, población i64, en i64)} · tecnologías{n u64 · n × (facción u32, k u64, k × texto)} · relaciones{n u64 · n × (menor u32, mayor u32,
//! u8: 0 alianza, 1 vasallaje, 2 guerra)}`.
//!
//! Un héroe: `id u64 · cuenta texto · facción opt_u64 · controlador u8 (0 humano, 1 bot) · liderazgo_base i64 · presencia{ubicación ·
//! desconecta_en opt_i64 · fuera bool} · residencia{u8: 0 ninguna, 1 plaza, 2 campamento · u32} · nivel u32 · experiencia i64 · puntos de
//! atributo i64 · puntos de perk i64 · atributos{fuerza, destreza, armadura, vitalidad: 4 × i64} · perks{n u64 · n × u32} · loadouts{n u64 ·
//! n × (id u64, nombre texto, escuadras{n u64 · n × u64}, perks{n u64 · n × u32}, activo bool)} · monedas{bronce, plata, oro: 3 × i64} ·
//! herido_hasta opt_i64 · víveres i64 · ración_en opt_i64 · almacén personal{carga} · cupo{día i64 · carga} · columna opt_u64 · carro{carga} · sintético bool ·
//! informes{n u64 · n × informe} ·
//! escuadras{n u64 · n × escuadra}`.
//! Una convocatoria: `id u64 · facción u32 · lugar{u8: 0 plaza, 1 campamento · u32} · líder u64 · política u8 · integrantes{n u64 · n × (héroe u64,
//! unido_en i64, selección)} · peticiones{n u64 · n × (héroe u64, pedido_en i64, expira_en i64, selección)} · creada_en i64`, con una selección
//! `escuadras{n u64 · n × u64} · carga{carga}` (el orden de las escuadras es el de salida y de combate).
//! Una ubicación: `u8 · payload`: 0 plaza (u32), 1 columna (u64), 2 campamento (u32), 3 desconectado (punto).
//! Una escuadra: `id u64 · héroe u64 · tropa u8 (índice del catálogo) · origen u8 (0 pesants, 1 artesanos, 2 nobleza) · plaza_origen opt_u64 ·
//! cantidad i64 · nivel u8 · experiencia i64 · moral{valor i64, t0 i64, por_minuto i64, máximo opt_i64, mínimo opt_i64} · contenedor{u8 · u64:
//! 0 campamento, 1 columna, 2 escolta de una caravana, 3 fuera} · en_guarnicion bool · reserva_batalla opt_u64 · prestada opt_u64`.
//! Un informe de inspección: `objetivo{u8: 0 columna, 1 caravana, 2 plaza · u64} · visto_en i64 · expira_en i64 · donde punto · contenido{u8 · payload}`: 0
//! columna `facción opt_u64 · héroes{n u64 · n × u64} · escuadras{n u64 · n × avistada}`, 1 caravana `escoltada bool · recursos{n u64 · n × u8}`, 2 plaza
//! `facción u32 · héroes · guarnición{n u64 · n × avistada}`, con una avistada `tropa u8 · cantidad i64 · héroe u64`.
//! Una carga: `n u64 · n × (recurso u8, cantidad i64)`.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_columna::{politica_a_u8, politica_de_u8, ritmo_escribir, ritmo_leer};
use crate::codec_comercio::{carga_escribir, carga_leer};
use crate::codec_economia::{recurso_a_u8, recurso_de_u8};
use crate::estado_militar::{EstadoMilitar, HeroeNucleo, Relacion};
use comercio::mercenarios::CupoDiario;
use militar::catalogo::{Origen, Tropa, TROPAS};
use militar::convocatoria::{Convocatoria, Integrante, Lugar, Peticion, Seleccion};
use militar::escuadra::{Contenedor, Escuadra};
use militar::heroe::{Atributos, Controlador, Heroe, Loadout, Monedas, Residencia};
use militar::inspeccion::{Avistada, Contenido, Informe, Objetivo};
use militar::mercenarios::Reclutas;
use militar::presencia::{Presencia, Ubicacion};

pub(crate) fn tropa_a_u8(t: Tropa) -> u8 {
    t as u8
}

pub(crate) fn tropa_de_u8(i: u8) -> Result<Tropa, ErrorFormato> {
    TROPAS.get(i as usize).copied().ok_or(ErrorFormato("tropa desconocida"))
}

pub(crate) fn controlador_a_u8(c: Controlador) -> u8 {
    match c {
        Controlador::Humano => 0,
        Controlador::Bot => 1,
    }
}

pub(crate) fn controlador_de_u8(i: u8) -> Result<Controlador, ErrorFormato> {
    match i {
        0 => Ok(Controlador::Humano),
        1 => Ok(Controlador::Bot),
        _ => Err(ErrorFormato("controlador desconocido")),
    }
}

pub(crate) fn ubicacion_escribir(w: &mut Escritor, u: &Ubicacion) {
    match u {
        Ubicacion::Asentamiento(a) => {
            w.u8(0);
            w.u32(*a);
        }
        Ubicacion::Columna(c) => {
            w.u8(1);
            w.u64(*c);
        }
        Ubicacion::Mercenarios(c) => {
            w.u8(2);
            w.u32(*c);
        }
        Ubicacion::Desconectado(p) => {
            w.u8(3);
            w.punto(*p);
        }
    }
}

pub(crate) fn ubicacion_leer(r: &mut Lector) -> Result<Ubicacion, ErrorFormato> {
    Ok(match r.u8()? {
        0 => Ubicacion::Asentamiento(r.u32()?),
        1 => Ubicacion::Columna(r.u64()?),
        2 => Ubicacion::Mercenarios(r.u32()?),
        3 => Ubicacion::Desconectado(r.punto()?),
        _ => return Err(ErrorFormato("ubicación desconocida")),
    })
}

pub(crate) fn residencia_escribir(w: &mut Escritor, residencia: &Option<Residencia>) {
    match residencia {
        None => {
            w.u8(0);
            w.u32(0);
        }
        Some(Residencia::Asentamiento(a)) => {
            w.u8(1);
            w.u32(*a);
        }
        Some(Residencia::Campamento(c)) => {
            w.u8(2);
            w.u32(*c);
        }
    }
}

pub(crate) fn residencia_leer(r: &mut Lector) -> Result<Option<Residencia>, ErrorFormato> {
    let (tag, id) = (r.u8()?, r.u32()?);
    Ok(match tag {
        0 => None,
        1 => Some(Residencia::Asentamiento(id)),
        2 => Some(Residencia::Campamento(id)),
        _ => return Err(ErrorFormato("residencia desconocida")),
    })
}

fn origen_a_u8(o: Origen) -> u8 {
    match o {
        Origen::Pesants => 0,
        Origen::Artesanos => 1,
        Origen::Nobleza => 2,
    }
}

fn origen_de_u8(i: u8) -> Result<Origen, ErrorFormato> {
    match i {
        0 => Ok(Origen::Pesants),
        1 => Ok(Origen::Artesanos),
        2 => Ok(Origen::Nobleza),
        _ => Err(ErrorFormato("origen de tropa desconocido")),
    }
}

fn contenedor_escribir(w: &mut Escritor, c: &Contenedor) {
    let (tag, id) = match c {
        Contenedor::Campamento => (0, 0),
        Contenedor::Ejercito(e) => (1, *e),
        Contenedor::Escolta(k) => (2, *k),
        Contenedor::Fuera => (3, 0),
        Contenedor::Avanzada(a) => (4, *a),
    };
    w.u8(tag);
    w.u64(id);
}

fn contenedor_leer(r: &mut Lector) -> Result<Contenedor, ErrorFormato> {
    let (tag, id) = (r.u8()?, r.u64()?);
    Ok(match tag {
        0 => Contenedor::Campamento,
        1 => Contenedor::Ejercito(id),
        2 => Contenedor::Escolta(id),
        3 => Contenedor::Fuera,
        4 => Contenedor::Avanzada(id),
        _ => return Err(ErrorFormato("contenedor de escuadra desconocido")),
    })
}

pub(crate) fn escuadra_escribir(w: &mut Escritor, e: &Escuadra) {
    let Escuadra { id, heroe, tropa, origen, plaza_origen, cantidad, nivel, experiencia, moral, contenedor, en_guarnicion, reserva_batalla, prestada } = e;
    w.u64(*id);
    w.u64(*heroe);
    w.u8(tropa_a_u8(*tropa));
    w.u8(origen_a_u8(*origen));
    w.opt_u64(plaza_origen.map(u64::from));
    w.i64(*cantidad);
    w.u8(*nivel);
    w.i64(*experiencia);
    ritmo_escribir(w, moral);
    contenedor_escribir(w, contenedor);
    w.bool(*en_guarnicion);
    w.opt_u64(*reserva_batalla);
    w.opt_u64(prestada.map(u64::from));
}

fn opt_u32(v: Option<u64>) -> Result<Option<u32>, ErrorFormato> {
    v.map(u32::try_from).transpose().map_err(|_| ErrorFormato("id fuera de rango"))
}

pub(crate) fn escuadra_leer(r: &mut Lector) -> Result<Escuadra, ErrorFormato> {
    let (id, heroe, tropa, origen, plaza_origen, cantidad, nivel, experiencia) = (r.u64()?, r.u64()?, tropa_de_u8(r.u8()?)?, origen_de_u8(r.u8()?)?, opt_u32(r.opt_u64()?)?, r.i64()?, r.u8()?, r.i64()?);
    let moral = ritmo_leer(r)?;
    Ok(Escuadra { id, heroe, tropa, origen, plaza_origen, cantidad, nivel, experiencia, moral, contenedor: contenedor_leer(r)?, en_guarnicion: r.bool()?, reserva_batalla: r.opt_u64()?, prestada: opt_u32(r.opt_u64()?)? })
}

fn lista_u64_escribir(w: &mut Escritor, v: &[u64]) {
    w.u64(v.len() as u64);
    v.iter().for_each(|&x| w.u64(x));
}

fn lista_u64_leer(r: &mut Lector) -> Result<Vec<u64>, ErrorFormato> {
    (0..r.cuenta()?).map(|_| r.u64()).collect()
}

fn lista_u32_escribir(w: &mut Escritor, v: &[u32]) {
    w.u64(v.len() as u64);
    v.iter().for_each(|&x| w.u32(x));
}

fn lista_u32_leer(r: &mut Lector) -> Result<Vec<u32>, ErrorFormato> {
    (0..r.cuenta()?).map(|_| r.u32()).collect()
}

fn heroe_escribir(w: &mut Escritor, h: &HeroeNucleo) {
    let HeroeNucleo { heroe, cuenta, faccion, almacen_personal, cupo, columna, carro, sintetico, informes } = h;
    let Heroe { id, controlador, liderazgo_base, presencia, residencia, nivel, experiencia, puntos_de_atributo, puntos_de_perk, atributos, perks_desbloqueados, loadouts, monedas, herido_hasta, viveres, racion_en, escuadras } = heroe;
    w.u64(*id);
    w.texto(cuenta);
    w.opt_u64(faccion.map(u64::from));
    w.u8(controlador_a_u8(*controlador));
    w.i64(*liderazgo_base);
    let Presencia { ubicacion, desconecta_en, fuera } = presencia;
    ubicacion_escribir(w, ubicacion);
    w.opt_i64(*desconecta_en);
    w.bool(*fuera);
    residencia_escribir(w, residencia);
    w.u32(*nivel);
    w.i64(*experiencia);
    w.i64(*puntos_de_atributo);
    w.i64(*puntos_de_perk);
    let Atributos { fuerza, destreza, armadura, vitalidad } = atributos;
    [fuerza, destreza, armadura, vitalidad].into_iter().for_each(|&a| w.i64(a));
    lista_u32_escribir(w, perks_desbloqueados);
    w.u64(loadouts.len() as u64);
    for Loadout { id, nombre, escuadras, perks, activo } in loadouts {
        w.u64(*id);
        w.texto(nombre);
        lista_u64_escribir(w, escuadras);
        lista_u32_escribir(w, perks);
        w.bool(*activo);
    }
    let Monedas { bronce, plata, oro } = monedas;
    [bronce, plata, oro].into_iter().for_each(|&m| w.i64(m));
    w.opt_i64(*herido_hasta);
    w.i64(*viveres);
    w.opt_i64(*racion_en);
    carga_escribir(w, almacen_personal);
    let CupoDiario { dia, comprado } = cupo;
    w.i64(*dia);
    carga_escribir(w, comprado);
    w.opt_u64(*columna);
    carga_escribir(w, carro);
    w.bool(*sintetico);
    w.u64(informes.len() as u64);
    informes.iter().for_each(|i| informe_escribir(w, i));
    w.u64(escuadras.len() as u64);
    escuadras.iter().for_each(|e| escuadra_escribir(w, e));
}

fn heroe_leer(r: &mut Lector) -> Result<HeroeNucleo, ErrorFormato> {
    let (id, cuenta, faccion, controlador, liderazgo_base) = (r.u64()?, r.texto()?, opt_u32(r.opt_u64()?)?, controlador_de_u8(r.u8()?)?, r.i64()?);
    let presencia = Presencia { ubicacion: ubicacion_leer(r)?, desconecta_en: r.opt_i64()?, fuera: r.bool()? };
    let residencia = residencia_leer(r)?;
    let (nivel, experiencia, puntos_de_atributo, puntos_de_perk) = (r.u32()?, r.i64()?, r.i64()?, r.i64()?);
    let atributos = Atributos { fuerza: r.i64()?, destreza: r.i64()?, armadura: r.i64()?, vitalidad: r.i64()? };
    let perks_desbloqueados = lista_u32_leer(r)?;
    let mut loadouts = Vec::new();
    for _ in 0..r.cuenta()? {
        loadouts.push(Loadout { id: r.u64()?, nombre: r.texto()?, escuadras: lista_u64_leer(r)?, perks: lista_u32_leer(r)?, activo: r.bool()? });
    }
    let monedas = Monedas { bronce: r.i64()?, plata: r.i64()?, oro: r.i64()? };
    let (herido_hasta, viveres, racion_en, almacen_personal) = (r.opt_i64()?, r.i64()?, r.opt_i64()?, carga_leer(r)?);
    let cupo = CupoDiario { dia: r.i64()?, comprado: carga_leer(r)? };
    let (columna, carro, sintetico) = (r.opt_u64()?, carga_leer(r)?, r.bool()?);
    let informes = (0..r.cuenta()?).map(|_| informe_leer(r)).collect::<Result<_, _>>()?;
    let mut escuadras = Vec::new();
    for _ in 0..r.cuenta()? {
        escuadras.push(escuadra_leer(r)?);
    }
    let heroe = Heroe { id, controlador, liderazgo_base, presencia, residencia, nivel, experiencia, puntos_de_atributo, puntos_de_perk, atributos, perks_desbloqueados, loadouts, monedas, herido_hasta, viveres, racion_en, escuadras };
    Ok(HeroeNucleo { heroe, cuenta, faccion, almacen_personal, cupo, columna, carro, sintetico, informes })
}

pub(crate) fn relacion_a_u8(r: Relacion) -> u8 {
    match r {
        Relacion::Alianza => 0,
        Relacion::Vasallaje => 1,
        Relacion::Guerra => 2,
    }
}

pub(crate) fn relacion_de_u8(i: u8) -> Result<Relacion, ErrorFormato> {
    Ok(match i {
        0 => Relacion::Alianza,
        1 => Relacion::Vasallaje,
        2 => Relacion::Guerra,
        _ => return Err(ErrorFormato("relación desconocida")),
    })
}

/// Un objetivo de inspección o persecución: `u8` (0 columna, 1 caravana, 2 plaza) y la id.
pub(crate) fn objetivo_escribir(w: &mut Escritor, o: &Objetivo) {
    let (tag, id) = match o {
        Objetivo::Ejercito(id) => (0, id),
        Objetivo::Caravana(id) => (1, id),
        Objetivo::Asentamiento(id) => (2, id),
    };
    w.u8(tag);
    w.u64(*id);
}

pub(crate) fn objetivo_leer(r: &mut Lector) -> Result<Objetivo, ErrorFormato> {
    let (tag, id) = (r.u8()?, r.u64()?);
    Ok(match tag {
        0 => Objetivo::Ejercito(id),
        1 => Objetivo::Caravana(id),
        2 => Objetivo::Asentamiento(id),
        _ => return Err(ErrorFormato("objetivo desconocido")),
    })
}

fn avistadas_escribir(w: &mut Escritor, v: &[Avistada]) {
    w.u64(v.len() as u64);
    for Avistada { tropa, cantidad, heroe } in v {
        w.u8(tropa_a_u8(*tropa));
        w.i64(*cantidad);
        w.u64(*heroe);
    }
}

fn avistadas_leer(r: &mut Lector) -> Result<Vec<Avistada>, ErrorFormato> {
    (0..r.cuenta()?).map(|_| Ok(Avistada { tropa: tropa_de_u8(r.u8()?)?, cantidad: r.i64()?, heroe: r.u64()? })).collect()
}

fn informe_escribir(w: &mut Escritor, i: &Informe) {
    let Informe { objetivo, visto_en, expira_en, donde, contenido } = i;
    objetivo_escribir(w, objetivo);
    w.i64(*visto_en);
    w.i64(*expira_en);
    w.punto(*donde);
    match contenido {
        Contenido::Columna { faccion, heroes, escuadras } => {
            w.u8(0);
            w.opt_u64(faccion.map(u64::from));
            lista_u64_escribir(w, heroes);
            avistadas_escribir(w, escuadras);
        }
        Contenido::Caravana { escoltada, recursos } => {
            w.u8(1);
            w.bool(*escoltada);
            w.u64(recursos.len() as u64);
            recursos.iter().for_each(|&r| w.u8(recurso_a_u8(r)));
        }
        Contenido::Plaza { faccion, heroes, guarnicion } => {
            w.u8(2);
            w.u32(*faccion);
            lista_u64_escribir(w, heroes);
            avistadas_escribir(w, guarnicion);
        }
    }
}

fn informe_leer(r: &mut Lector) -> Result<Informe, ErrorFormato> {
    let objetivo = objetivo_leer(r)?;
    let (visto_en, expira_en, donde) = (r.i64()?, r.i64()?, r.punto()?);
    let contenido = match r.u8()? {
        0 => Contenido::Columna { faccion: opt_u32(r.opt_u64()?)?, heroes: lista_u64_leer(r)?, escuadras: avistadas_leer(r)? },
        1 => Contenido::Caravana { escoltada: r.bool()?, recursos: (0..r.cuenta()?).map(|_| recurso_de_u8(r.u8()?)).collect::<Result<_, _>>()? },
        2 => Contenido::Plaza { faccion: r.u32()?, heroes: lista_u64_leer(r)?, guarnicion: avistadas_leer(r)? },
        _ => return Err(ErrorFormato("contenido de informe desconocido")),
    };
    Ok(Informe { objetivo, visto_en, expira_en, donde, contenido })
}

fn seleccion_escribir(w: &mut Escritor, s: &Seleccion) {
    let Seleccion { escuadras, carga } = s;
    lista_u64_escribir(w, escuadras);
    carga_escribir(w, carga);
}

fn seleccion_leer(r: &mut Lector) -> Result<Seleccion, ErrorFormato> {
    Ok(Seleccion { escuadras: lista_u64_leer(r)?, carga: carga_leer(r)? })
}

fn convocatoria_escribir(w: &mut Escritor, c: &Convocatoria) {
    let Convocatoria { id, faccion, lugar, lider, politica, integrantes, peticiones, creada_en } = c;
    w.u64(*id);
    w.u32(*faccion);
    let (tag, lugar) = match lugar {
        Lugar::Asentamiento(a) => (0, *a),
        Lugar::Campamento(k) => (1, *k),
    };
    w.u8(tag);
    w.u32(lugar);
    w.u64(*lider);
    w.u8(politica_a_u8(*politica));
    w.u64(integrantes.len() as u64);
    for Integrante { heroe, unido_en, seleccion } in integrantes {
        w.u64(*heroe);
        w.i64(*unido_en);
        seleccion_escribir(w, seleccion);
    }
    w.u64(peticiones.len() as u64);
    for Peticion { heroe, pedido_en, expira_en, seleccion } in peticiones {
        w.u64(*heroe);
        w.i64(*pedido_en);
        w.i64(*expira_en);
        seleccion_escribir(w, seleccion);
    }
    w.i64(*creada_en);
}

fn convocatoria_leer(r: &mut Lector) -> Result<Convocatoria, ErrorFormato> {
    let (id, faccion) = (r.u64()?, r.u32()?);
    let (tag, lugar) = (r.u8()?, r.u32()?);
    let lugar = match tag {
        0 => Lugar::Asentamiento(lugar),
        1 => Lugar::Campamento(lugar),
        _ => return Err(ErrorFormato("lugar de convocatoria desconocido")),
    };
    let (lider, politica) = (r.u64()?, politica_de_u8(r.u8()?)?);
    let mut integrantes = Vec::new();
    for _ in 0..r.cuenta()? {
        integrantes.push(Integrante { heroe: r.u64()?, unido_en: r.i64()?, seleccion: seleccion_leer(r)? });
    }
    let mut peticiones = Vec::new();
    for _ in 0..r.cuenta()? {
        peticiones.push(Peticion { heroe: r.u64()?, pedido_en: r.i64()?, expira_en: r.i64()?, seleccion: seleccion_leer(r)? });
    }
    Ok(Convocatoria { id, faccion, lugar, lider, politica, integrantes, peticiones, creada_en: r.i64()? })
}

pub(crate) fn escribir(w: &mut Escritor, m: &EstadoMilitar) {
    let EstadoMilitar { heroes, convocatorias, reclutas, tecnologias, relaciones, residentes: _ } = m;
    w.u64(heroes.len() as u64);
    heroes.values().for_each(|h| heroe_escribir(w, h));
    w.u64(convocatorias.len() as u64);
    convocatorias.values().for_each(|c| convocatoria_escribir(w, c));
    w.u64(reclutas.len() as u64);
    for (&campamento, Reclutas { poblacion, en }) in reclutas {
        w.u32(campamento);
        w.i64(*poblacion);
        w.i64(*en);
    }
    w.u64(tecnologias.len() as u64);
    for (&faccion, set) in tecnologias {
        w.u32(faccion);
        w.u64(set.len() as u64);
        set.iter().for_each(|t| w.texto(t));
    }
    w.u64(relaciones.len() as u64);
    for (&(a, b), relacion) in relaciones {
        w.u32(a);
        w.u32(b);
        w.u8(relacion_a_u8(*relacion));
    }
}

pub(crate) fn leer(r: &mut Lector) -> Result<EstadoMilitar, ErrorFormato> {
    let mut m = EstadoMilitar::default();
    for _ in 0..r.cuenta()? {
        m.insertar(heroe_leer(r)?);
    }
    for _ in 0..r.cuenta()? {
        let c = convocatoria_leer(r)?;
        m.convocatorias.insert(c.id, c);
    }
    for _ in 0..r.cuenta()? {
        let campamento = r.u32()?;
        m.reclutas.insert(campamento, Reclutas { poblacion: r.i64()?, en: r.i64()? });
    }
    for _ in 0..r.cuenta()? {
        let faccion = r.u32()?;
        let mut set = std::collections::BTreeSet::new();
        for _ in 0..r.cuenta()? {
            set.insert(r.texto()?);
        }
        m.tecnologias.insert(faccion, set);
    }
    for _ in 0..r.cuenta()? {
        let (a, b) = (r.u32()?, r.u32()?);
        let relacion = relacion_de_u8(r.u8()?)?;
        m.relaciones.insert((a, b), relacion);
    }
    Ok(m)
}
