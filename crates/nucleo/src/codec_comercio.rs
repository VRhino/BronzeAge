//! Serialización canónica del comercio (`EstadoComercio`) para el hash y las fotos. Cada estructura se desestructura **entera** (sin `..`):
//! si el crate `comercio` añade un campo, esto deja de compilar y obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `siguiente_acuerdo u64 · siguiente_orden u64 · siguiente_caravana u64 ·
//! acuerdos{n u64 · n × acuerdo} · libro{órdenes{n u64 · n × orden} · historial{ídem}} · flota{n u64 · n × caravana} ·
//! reputación{n u64 · n × (facción u32, valor i64, t0 i64, alianzas u32)} · última caravana{n u64 · n × (plaza u64, instante i64)} ·
//! red{aristas{n u64 · n × (x0,y0,x1,y1 i64)} · rutas{n u64 · n × (origen u32, destino u32, facción u32, aristas{n u64 · n × arista}, último
//! lanzamiento i64)}} · reparto{n u64 · n × (plaza u64, instante i64)} · mercenarios{contadores{carga} · repone_en i64 · stock{n u64 · n ×
//! (campamento u32, carga)} · pulso opt_i64}`. El carro (`suministro`) y las caravanas adjuntas de cada columna viajan con la columna
//! (`estado`); la residencia, el almacén personal y el cupo diario de cada comprador, con su héroe (`codec_militar`).
//! Un acuerdo: `id u64 · a u32 · b u32 · líneas_a{n u64 · n × (recurso u8, total i64, entregada i64)} · líneas_b · creado_en i64 ·
//! aceptado_en opt_i64 · expira_en i64 · estado u8`. Una orden: `id u64 · plaza u32 · tipo u8 · recurso u8 · cantidad i64 · cumplida i64 ·
//! precio i64 · creado_en i64 · expira_en i64 · estado u8`. Una carga: `n u64 · n × (recurso u8, cantidad i64)`. Una caravana: `id u64 ·
//! tipo u8 · origen u32 · destino opt_u64 · contenido{carga} · carros{n u64 · n × (tipo u8, animal u8: 255 ninguno)} · estado u8 ·
//! posición · ruta{n u64 · n × punto} · costes{n u64 · n × i64} · plan{bool · generación u32 · n u32 · n × (punto, instante i64)} · peajes{n u64 · n × (ciudad u32,
//! progreso i64)} · origen_acuerdo{bool · id u64 · lado u8} · prepara_hasta opt_i64 · escolta{n u64 · n × (id u64, héroe u64, liderazgo
//! i64)} · reservada_manual bool · anfitriona opt_u64 · antes_de_batalla u8 (255 ninguno) · pausada_desde opt_i64 · de_campamento bool`.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_economia::{recurso_a_u8, recurso_de_u8};
use crate::estado_comercio::{EstadoComercio, EstadoMercenarios};
use base::{Plan, Vertice};
use comercio::caravana::{Animal, Caravana, Carro, Escuadra, EstadoCaravana, TipoCaravana, TipoCarro};
use comercio::mercenarios::{MercadoMercenario, StockCampamento};
use comercio::orden_mercado::{EstadoOrden, LibroOrdenes, OrdenMercado, TipoOrden};
use comercio::red_caminos::{Arista, RedCaminos, RutaComercial};
use comercio::reputacion::Reputacion;
use comercio::trueque::{AcuerdoTrueque, EstadoTrueque, Linea};
use comercio::viaje::Peaje;
use comercio::{Carga, Lado, Reglas};

const NINGUNO: u8 = 255;

fn estado_trueque_a_u8(e: EstadoTrueque) -> u8 {
    match e {
        EstadoTrueque::Propuesto => 0,
        EstadoTrueque::Activo => 1,
        EstadoTrueque::Cumplido => 2,
        EstadoTrueque::Expirado => 3,
        EstadoTrueque::Rechazado => 4,
    }
}

fn estado_trueque_de_u8(i: u8) -> Result<EstadoTrueque, ErrorFormato> {
    Ok(match i {
        0 => EstadoTrueque::Propuesto,
        1 => EstadoTrueque::Activo,
        2 => EstadoTrueque::Cumplido,
        3 => EstadoTrueque::Expirado,
        4 => EstadoTrueque::Rechazado,
        _ => return Err(ErrorFormato("estado de trueque desconocido")),
    })
}

fn lado_a_u8(l: Lado) -> u8 {
    match l {
        Lado::A => 0,
        Lado::B => 1,
    }
}

fn lado_de_u8(i: u8) -> Result<Lado, ErrorFormato> {
    match i {
        0 => Ok(Lado::A),
        1 => Ok(Lado::B),
        _ => Err(ErrorFormato("lado de trueque desconocido")),
    }
}

pub(crate) fn tipo_orden_a_u8(t: TipoOrden) -> u8 {
    match t {
        TipoOrden::Compra => 0,
        TipoOrden::Venta => 1,
    }
}

fn tipo_orden_de_u8(i: u8) -> Result<TipoOrden, ErrorFormato> {
    match i {
        0 => Ok(TipoOrden::Compra),
        1 => Ok(TipoOrden::Venta),
        _ => Err(ErrorFormato("tipo de orden desconocido")),
    }
}

fn estado_orden_a_u8(e: EstadoOrden) -> u8 {
    match e {
        EstadoOrden::Activa => 0,
        EstadoOrden::Cumplida => 1,
        EstadoOrden::Expirada => 2,
    }
}

fn estado_orden_de_u8(i: u8) -> Result<EstadoOrden, ErrorFormato> {
    match i {
        0 => Ok(EstadoOrden::Activa),
        1 => Ok(EstadoOrden::Cumplida),
        2 => Ok(EstadoOrden::Expirada),
        _ => Err(ErrorFormato("estado de orden desconocido")),
    }
}

fn tipo_caravana_a_u8(t: TipoCaravana) -> u8 {
    match t {
        TipoCaravana::Comercial => 0,
        TipoCaravana::Construccion => 1,
        TipoCaravana::Militar => 2,
        TipoCaravana::Contrabando => 3,
        TipoCaravana::Acarreo => 4,
    }
}

fn tipo_caravana_de_u8(i: u8) -> Result<TipoCaravana, ErrorFormato> {
    Ok(match i {
        0 => TipoCaravana::Comercial,
        1 => TipoCaravana::Construccion,
        2 => TipoCaravana::Militar,
        3 => TipoCaravana::Contrabando,
        4 => TipoCaravana::Acarreo,
        _ => return Err(ErrorFormato("tipo de caravana desconocido")),
    })
}

pub(crate) fn tipo_carro_a_u8(t: TipoCarro) -> u8 {
    match t {
        TipoCarro::Basico => 0,
        TipoCarro::Reforzado => 1,
    }
}

pub(crate) fn tipo_carro_de_u8(i: u8) -> Result<TipoCarro, ErrorFormato> {
    match i {
        0 => Ok(TipoCarro::Basico),
        1 => Ok(TipoCarro::Reforzado),
        _ => Err(ErrorFormato("tipo de carro desconocido")),
    }
}

pub(crate) fn animal_a_u8(a: Animal) -> u8 {
    match a {
        Animal::Buey => 0,
        Animal::Caballo => 1,
        Animal::Camello => 2,
    }
}

pub(crate) fn animal_de_u8(i: u8) -> Result<Animal, ErrorFormato> {
    match i {
        0 => Ok(Animal::Buey),
        1 => Ok(Animal::Caballo),
        2 => Ok(Animal::Camello),
        _ => Err(ErrorFormato("animal desconocido")),
    }
}

fn estado_caravana_a_u8(e: EstadoCaravana) -> u8 {
    match e {
        EstadoCaravana::Disponible => 0,
        EstadoCaravana::Preparando => 1,
        EstadoCaravana::EnTransito => 2,
        EstadoCaravana::Retornando => 3,
        EstadoCaravana::Aparcada => 4,
        EstadoCaravana::Adjunta => 5,
        EstadoCaravana::EnBatalla => 6,
    }
}

fn estado_caravana_de_u8(i: u8) -> Result<EstadoCaravana, ErrorFormato> {
    Ok(match i {
        0 => EstadoCaravana::Disponible,
        1 => EstadoCaravana::Preparando,
        2 => EstadoCaravana::EnTransito,
        3 => EstadoCaravana::Retornando,
        4 => EstadoCaravana::Aparcada,
        5 => EstadoCaravana::Adjunta,
        6 => EstadoCaravana::EnBatalla,
        _ => return Err(ErrorFormato("estado de caravana desconocido")),
    })
}

pub(crate) fn carga_escribir(w: &mut Escritor, c: &Carga) {
    w.u64(c.len() as u64);
    for (&r, &n) in c {
        w.u8(recurso_a_u8(r));
        w.i64(n);
    }
}

pub(crate) fn carga_leer(r: &mut Lector) -> Result<Carga, ErrorFormato> {
    let mut c = Carga::new();
    for _ in 0..r.cuenta()? {
        let (rec, n) = (recurso_de_u8(r.u8()?)?, r.i64()?);
        c.insert(rec, n);
    }
    Ok(c)
}

fn lineas_escribir(w: &mut Escritor, lineas: &[Linea]) {
    w.u64(lineas.len() as u64);
    for l in lineas {
        let Linea { recurso, cantidad_total, cantidad_entregada } = l;
        w.u8(recurso_a_u8(*recurso));
        w.i64(*cantidad_total);
        w.i64(*cantidad_entregada);
    }
}

fn lineas_leer(r: &mut Lector) -> Result<Vec<Linea>, ErrorFormato> {
    let mut v = Vec::new();
    for _ in 0..r.cuenta()? {
        v.push(Linea { recurso: recurso_de_u8(r.u8()?)?, cantidad_total: r.i64()?, cantidad_entregada: r.i64()? });
    }
    Ok(v)
}

fn acuerdo_escribir(w: &mut Escritor, a: &AcuerdoTrueque) {
    let AcuerdoTrueque { id, a: lado_a, b, lineas_a, lineas_b, creado_en, aceptado_en, expira_en, estado } = a;
    w.u64(*id);
    w.u32(*lado_a);
    w.u32(*b);
    lineas_escribir(w, lineas_a);
    lineas_escribir(w, lineas_b);
    w.i64(*creado_en);
    w.opt_i64(*aceptado_en);
    w.i64(*expira_en);
    w.u8(estado_trueque_a_u8(*estado));
}

fn acuerdo_leer(r: &mut Lector) -> Result<AcuerdoTrueque, ErrorFormato> {
    Ok(AcuerdoTrueque {
        id: r.u64()?,
        a: r.u32()?,
        b: r.u32()?,
        lineas_a: lineas_leer(r)?,
        lineas_b: lineas_leer(r)?,
        creado_en: r.i64()?,
        aceptado_en: r.opt_i64()?,
        expira_en: r.i64()?,
        estado: estado_trueque_de_u8(r.u8()?)?,
    })
}

fn orden_escribir(w: &mut Escritor, o: &OrdenMercado) {
    let OrdenMercado { id, asentamiento, tipo, recurso, cantidad, cantidad_cumplida, precio_unitario, creado_en, expira_en, estado } = o;
    w.u64(*id);
    w.u32(*asentamiento);
    w.u8(tipo_orden_a_u8(*tipo));
    w.u8(recurso_a_u8(*recurso));
    w.i64(*cantidad);
    w.i64(*cantidad_cumplida);
    w.i64(*precio_unitario);
    w.i64(*creado_en);
    w.i64(*expira_en);
    w.u8(estado_orden_a_u8(*estado));
}

fn orden_leer(r: &mut Lector) -> Result<OrdenMercado, ErrorFormato> {
    Ok(OrdenMercado {
        id: r.u64()?,
        asentamiento: r.u32()?,
        tipo: tipo_orden_de_u8(r.u8()?)?,
        recurso: recurso_de_u8(r.u8()?)?,
        cantidad: r.i64()?,
        cantidad_cumplida: r.i64()?,
        precio_unitario: r.i64()?,
        creado_en: r.i64()?,
        expira_en: r.i64()?,
        estado: estado_orden_de_u8(r.u8()?)?,
    })
}

fn caravana_escribir(w: &mut Escritor, c: &Caravana) {
    let Caravana {
        id,
        tipo,
        origen,
        destino,
        contenido,
        carros,
        estado,
        posicion,
        ruta,
        costes,
        plan,
        peajes,
        origen_acuerdo,
        prepara_hasta,
        escolta,
        reservada_manual,
        anfitriona,
        antes_de_batalla,
        pausada_desde,
        de_campamento,
    } = c;
    w.u64(*id);
    w.u8(tipo_caravana_a_u8(*tipo));
    w.u32(*origen);
    w.opt_u64(destino.map(u64::from));
    carga_escribir(w, contenido);
    w.u64(carros.len() as u64);
    for Carro { tipo, animal } in carros {
        w.u8(tipo_carro_a_u8(*tipo));
        w.u8(animal.map_or(NINGUNO, animal_a_u8));
    }
    w.u8(estado_caravana_a_u8(*estado));
    w.punto(*posicion);
    w.u64(ruta.len() as u64);
    ruta.iter().for_each(|&p| w.punto(p));
    w.u64(costes.len() as u64);
    costes.iter().for_each(|&c| w.i64(c));
    match plan {
        Some(Plan { generacion, vertices }) => {
            w.bool(true);
            w.u32(*generacion);
            w.u32(vertices.len() as u32);
            for v in vertices {
                w.punto(v.p);
                w.i64(v.en);
            }
        }
        None => w.bool(false),
    }
    w.u64(peajes.len() as u64);
    for Peaje { ciudad, progreso } in peajes {
        w.u32(*ciudad);
        w.i64(*progreso);
    }
    match origen_acuerdo {
        Some((a, lado)) => {
            w.bool(true);
            w.u64(*a);
            w.u8(lado_a_u8(*lado));
        }
        None => w.bool(false),
    }
    w.opt_i64(*prepara_hasta);
    w.u64(escolta.len() as u64);
    for Escuadra { id, heroe, liderazgo } in escolta {
        w.u64(*id);
        w.u64(*heroe);
        w.i64(*liderazgo);
    }
    w.bool(*reservada_manual);
    w.opt_u64(anfitriona.map(u64::from));
    w.u8(antes_de_batalla.map_or(NINGUNO, estado_caravana_a_u8));
    w.opt_i64(*pausada_desde);
    w.bool(*de_campamento);
}

fn id32(v: u64) -> Result<u32, ErrorFormato> {
    u32::try_from(v).map_err(|_| ErrorFormato("id de asentamiento fuera de rango"))
}

fn caravana_leer(r: &mut Lector) -> Result<Caravana, ErrorFormato> {
    let (id, tipo, origen) = (r.u64()?, tipo_caravana_de_u8(r.u8()?)?, r.u32()?);
    let destino = r.opt_u64()?.map(id32).transpose()?;
    let contenido = carga_leer(r)?;
    let mut carros = Vec::new();
    for _ in 0..r.cuenta()? {
        let tipo = tipo_carro_de_u8(r.u8()?)?;
        let animal = match r.u8()? {
            NINGUNO => None,
            i => Some(animal_de_u8(i)?),
        };
        carros.push(Carro { tipo, animal });
    }
    let estado = estado_caravana_de_u8(r.u8()?)?;
    let posicion = r.punto()?;
    let mut ruta = Vec::new();
    for _ in 0..r.cuenta()? {
        ruta.push(r.punto()?);
    }
    let mut costes = Vec::new();
    for _ in 0..r.cuenta()? {
        costes.push(r.i64()?);
    }
    let plan = if r.bool()? {
        let generacion = r.u32()?;
        let mut vertices = Vec::new();
        for _ in 0..r.u32()? {
            vertices.push(Vertice { p: r.punto()?, en: r.i64()? });
        }
        Some(Plan { generacion, vertices })
    } else {
        None
    };
    let mut peajes = Vec::new();
    for _ in 0..r.cuenta()? {
        peajes.push(Peaje { ciudad: r.u32()?, progreso: r.i64()? });
    }
    let origen_acuerdo = if r.bool()? { Some((r.u64()?, lado_de_u8(r.u8()?)?)) } else { None };
    let prepara_hasta = r.opt_i64()?;
    let mut escolta = Vec::new();
    for _ in 0..r.cuenta()? {
        escolta.push(Escuadra { id: r.u64()?, heroe: r.u64()?, liderazgo: r.i64()? });
    }
    let reservada_manual = r.bool()?;
    let anfitriona = r.opt_u64()?.map(id32).transpose()?;
    let antes_de_batalla = match r.u8()? {
        NINGUNO => None,
        i => Some(estado_caravana_de_u8(i)?),
    };
    Ok(Caravana {
        id,
        tipo,
        origen,
        destino,
        contenido,
        carros,
        estado,
        posicion,
        ruta,
        costes,
        plan,
        peajes,
        origen_acuerdo,
        prepara_hasta,
        escolta,
        reservada_manual,
        anfitriona,
        antes_de_batalla,
        pausada_desde: r.opt_i64()?,
        de_campamento: r.bool()?,
    })
}

fn arista_escribir(w: &mut Escritor, a: &Arista) {
    let ((x0, y0), (x1, y1)) = a;
    w.i64(*x0);
    w.i64(*y0);
    w.i64(*x1);
    w.i64(*y1);
}

fn arista_leer(r: &mut Lector) -> Result<Arista, ErrorFormato> {
    Ok(((r.i64()?, r.i64()?), (r.i64()?, r.i64()?)))
}

fn red_escribir(w: &mut Escritor, red: &RedCaminos) {
    let aristas: Vec<&Arista> = red.aristas().collect();
    w.u64(aristas.len() as u64);
    aristas.into_iter().for_each(|a| arista_escribir(w, a));
    let rutas: Vec<_> = red.rutas().collect();
    w.u64(rutas.len() as u64);
    for RutaComercial { origen, destino, faccion, aristas, ultimo_lanzamiento } in rutas {
        w.u32(*origen);
        w.u32(*destino);
        w.u32(*faccion);
        w.u64(aristas.len() as u64);
        aristas.iter().for_each(|a| arista_escribir(w, a));
        w.i64(*ultimo_lanzamiento);
    }
}

fn red_leer(r: &mut Lector) -> Result<RedCaminos, ErrorFormato> {
    let mut aristas = Vec::new();
    for _ in 0..r.cuenta()? {
        aristas.push(arista_leer(r)?);
    }
    let mut rutas = Vec::new();
    for _ in 0..r.cuenta()? {
        let (origen, destino, faccion) = (r.u32()?, r.u32()?, r.u32()?);
        let mut propias = Vec::new();
        for _ in 0..r.cuenta()? {
            propias.push(arista_leer(r)?);
        }
        rutas.push(RutaComercial { origen, destino, faccion, aristas: propias, ultimo_lanzamiento: r.i64()? });
    }
    Ok(RedCaminos::desde_partes(aristas, rutas))
}

fn mercenarios_escribir(w: &mut Escritor, m: &EstadoMercenarios) {
    let EstadoMercenarios { mercado, stock, pulso } = m;
    let MercadoMercenario { contadores, repone_en } = mercado;
    carga_escribir(w, contadores);
    w.i64(*repone_en);
    w.u64(stock.len() as u64);
    for (&campamento, s) in stock {
        w.u32(campamento);
        carga_escribir(w, &s.stock);
    }
    w.opt_i64(*pulso);
}

fn mercenarios_leer(r: &mut Lector) -> Result<EstadoMercenarios, ErrorFormato> {
    let mut m = EstadoMercenarios { mercado: MercadoMercenario { contadores: carga_leer(r)?, repone_en: r.i64()? }, ..EstadoMercenarios::default() };
    for _ in 0..r.cuenta()? {
        let campamento = r.u32()?;
        m.stock.insert(campamento, StockCampamento { stock: carga_leer(r)? });
    }
    m.pulso = r.opt_i64()?;
    Ok(m)
}

pub(crate) fn escribir(w: &mut Escritor, c: &EstadoComercio) {
    let EstadoComercio { acuerdos, libro, flota, reputacion, ultima_caravana, red, reparto, mercenarios, siguiente_acuerdo, siguiente_orden, siguiente_caravana } = c;
    w.u64(*siguiente_acuerdo);
    w.u64(*siguiente_orden);
    w.u64(*siguiente_caravana);
    w.u64(acuerdos.len() as u64);
    acuerdos.iter().for_each(|a| acuerdo_escribir(w, a));
    let LibroOrdenes { ordenes, historial } = libro;
    w.u64(ordenes.len() as u64);
    ordenes.iter().for_each(|o| orden_escribir(w, o));
    w.u64(historial.len() as u64);
    historial.iter().for_each(|o| orden_escribir(w, o));
    w.u64(flota.len() as u64);
    flota.iter().for_each(|c| caravana_escribir(w, c));
    w.u64(reputacion.len() as u64);
    for (&faccion, rep) in reputacion {
        let (valor, t0) = rep.ancla();
        w.u32(faccion);
        w.i64(valor);
        w.i64(t0);
        w.u32(rep.alianzas());
    }
    w.u64(ultima_caravana.len() as u64);
    for (&plaza, &t) in ultima_caravana {
        w.u64(plaza);
        w.i64(t);
    }
    red_escribir(w, red);
    w.u64(reparto.len() as u64);
    for (&plaza, &t) in reparto {
        w.u64(plaza);
        w.i64(t);
    }
    mercenarios_escribir(w, mercenarios);
}

pub(crate) fn leer(r: &mut Lector, reglas: &Reglas) -> Result<EstadoComercio, ErrorFormato> {
    let mut c = EstadoComercio { siguiente_acuerdo: r.u64()?, siguiente_orden: r.u64()?, siguiente_caravana: r.u64()?, ..EstadoComercio::default() };
    for _ in 0..r.cuenta()? {
        c.acuerdos.push(acuerdo_leer(r)?);
    }
    for _ in 0..r.cuenta()? {
        c.libro.ordenes.push(orden_leer(r)?);
    }
    for _ in 0..r.cuenta()? {
        c.libro.historial.push(orden_leer(r)?);
    }
    for _ in 0..r.cuenta()? {
        c.flota.push(caravana_leer(r)?);
    }
    for _ in 0..r.cuenta()? {
        let (faccion, valor, t0, alianzas) = (r.u32()?, r.i64()?, r.i64()?, r.u32()?);
        c.reputacion.insert(faccion, Reputacion::desde(&reglas.reputacion, valor, t0, alianzas));
    }
    for _ in 0..r.cuenta()? {
        c.ultima_caravana.insert(r.u64()?, r.i64()?);
    }
    c.red = red_leer(r)?;
    for _ in 0..r.cuenta()? {
        c.reparto.insert(r.u64()?, r.i64()?);
    }
    c.mercenarios = mercenarios_leer(r)?;
    Ok(c)
}
