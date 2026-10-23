//! Serialización canónica de las batallas de Unity (`EstadoBatallas`) para el hash, las fotos y el registro de las órdenes que traen un mensaje del
//! servidor de batalla (formato BAN13; el contexto de caravana desde BAN19; la plaza congelada y los datos congelados de cada escuadra, desde BAN21). Cada estructura se desestructura **entera** (sin `..`): si `combate::ciclo` gana un campo, esto deja de
//! compilar y obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `batallas{n u64 · n × batalla}`.
//! - Batalla: `id u64 · estado u8 (0 convocando, 1 asignada, 2 en curso, 3 aplicada, 4 cancelada, 5 fallida) · iniciada_por u64 · punto · ticket ·
//!   incorporaciones{n u64 · n × (secuencia u32, lado u8, participante)} · salidas{n u64 · n × (héroe u64, motivo u8, derrotado bool, en i64)} · bloqueo ·
//!   expira_en i64 · convocatoria_hasta opt_i64 · iniciada_en opt_i64 · aplicada_en opt_i64 · cerrada_en opt_i64 · asignación opt{bool · asignación} ·
//!   resultado opt{bool · resultado}`.
//! - Ticket: `revisión u32 · contexto · duración_máxima i64 · atacante bando · defensor bando · plaza opt{bool · plaza}`. Plaza congelada (un asedio): `layout_version u32 · unidades_por_celda u32 ·
//!   edificios{n u64 · n × (id u32, tipo texto, centro punto, ancho u32, alto u32, nivel_interno u8, estado u8: 0 en cola, 1 en construcción, 2 activo, dañado bool)} ·
//!   recintos{n u64 · n × (id u32, nivel u8, celdas{n u64 · n × (col i32, fila i32, clase u8)})}`. Lado: 0 atacante, 1 defensor.
//! - Contexto: `tag u8` y, según él, `0 asedio: plaza u32 · 1 campo abierto: punto, solitarios bool · 2 caravana: caravanas{n u64 · n × (caravana u64, carros u64)}, punto, modalidad u8 (0 disputa,
//!   1 aguante) · 3 campamento de bandidos: campamento u32, punto · 4 saqueo de pueblo: pueblo u64, punto · 5 avanzada: avanzada u64, etapa u8 (0 atalaya,
//!   1 fuerte, 2 ciudadela), punto`.
//! - Bando: `facción opt_u64 · capacidad u64 · participantes{n u64 · n × participante} · sin_héroe{n u64 · n × escuadra}`.
//! - Participante: `héroe u64 · facción opt_u64 · controlador u8 (el de `codec_militar`) · casillas_libres u32 · escuadras{n u64 · n × escuadra}`.
//!   Escuadra de batalla: `id u64 · tropa u8 · dueño opt_u64 · efectivos i64 · nivel u8 · experiencia i64 · moral i64` (los tres últimos, del instante de abrir).
//! - Bloqueo: `ejércitos{n u64 · n × u64} · caravanas{n u64 · n × u64} · asentamiento opt_u64 · campamento opt_u64 · pueblo opt_u64 · avanzada opt_u64`.
//! - Asignación: `intento texto · servidor texto · revisión u32 · instancia{host texto, puerto u16, protocolo texto} · tokens{n u64 · n × (héroe u64, token texto,
//!   expira_en i64)}`.
//! - Resultado: `id texto · revisión u32 · intento texto · inicio i64 · fin i64 · ganador u8 · razón u8 (0 objetivos capturados, 1 aniquilación, 2 tiempo
//!   agotado) · objetivos{n u64 · n × (id texto, capturado_por u8: 0 atacante, 1 defensor, 2 nadie)} · por_escuadra{n u64 · n × (escuadra u64, desplegados,
//!   supervivientes, muertos, xp: i64)} · por_héroe{n u64 · n × (héroe u64, participó bool, sobrevivió bool, xp i64, derrotado u8: 0 no, 1 sí, 2 ausente, botín
//!   opt{bool · botín})} · versión_servidor texto · carros_perdidos{n u64 · n × texto}`. Botín: `objetos{n u64 · n × (definición texto, cantidad i64,
//!   instancia opt{bool · texto})} · monedas{bronce, plata, oro: i64}`.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_economia::{escribir_celdas, leer_celdas};
use crate::codec_militar::{controlador_a_u8, controlador_de_u8, tropa_a_u8, tropa_de_u8};
use crate::estado_batallas::EstadoBatallas;
use asentamiento::edificio::Estado as EstadoEdificio;
use combate::avanzada::Etapa;
use combate::caravana::Modalidad;
use combate::ciclo::{Asignacion, Batalla, BandoDeTicket, Bloqueo, CaravanaEnBatalla, Contexto, EdificioCongelado, EscuadraDeSalida, EscuadraEnBatalla, EstadoBatalla, Incorporacion, Instancia, MotivoSalida, Participante, PlazaCongelada, RecintoCongelado, Salida, SalidaBatalla, Ticket, Token};
use combate::resolucion::Lado;
use combate::resultado::{Botin, ObjetoBotin, Razon, Resultado, ResultadoEscuadra, ResultadoHeroe};
use militar::heroe::Monedas;

fn lado_a_u8(l: Lado) -> u8 {
    match l {
        Lado::Atacante => 0,
        Lado::Defensor => 1,
    }
}

fn lado_de_u8(i: u8) -> Result<Lado, ErrorFormato> {
    match i {
        0 => Ok(Lado::Atacante),
        1 => Ok(Lado::Defensor),
        _ => Err(ErrorFormato("lado de batalla desconocido")),
    }
}

fn estado_a_u8(e: EstadoBatalla) -> u8 {
    match e {
        EstadoBatalla::Convocando => 0,
        EstadoBatalla::Asignada => 1,
        EstadoBatalla::EnCurso => 2,
        EstadoBatalla::Aplicada => 3,
        EstadoBatalla::Cancelada => 4,
        EstadoBatalla::Fallida => 5,
    }
}

fn estado_de_u8(i: u8) -> Result<EstadoBatalla, ErrorFormato> {
    Ok(match i {
        0 => EstadoBatalla::Convocando,
        1 => EstadoBatalla::Asignada,
        2 => EstadoBatalla::EnCurso,
        3 => EstadoBatalla::Aplicada,
        4 => EstadoBatalla::Cancelada,
        5 => EstadoBatalla::Fallida,
        _ => return Err(ErrorFormato("estado de batalla desconocido")),
    })
}

fn motivo_a_u8(m: MotivoSalida) -> u8 {
    match m {
        MotivoSalida::NoConectado => 0,
        MotivoSalida::Eliminado => 1,
        MotivoSalida::Abandono => 2,
        MotivoSalida::Desconexion => 3,
    }
}

fn motivo_de_u8(i: u8) -> Result<MotivoSalida, ErrorFormato> {
    Ok(match i {
        0 => MotivoSalida::NoConectado,
        1 => MotivoSalida::Eliminado,
        2 => MotivoSalida::Abandono,
        3 => MotivoSalida::Desconexion,
        _ => return Err(ErrorFormato("motivo de salida de batalla desconocido")),
    })
}

fn lista_u64(w: &mut Escritor, v: &[u64]) {
    w.u64(v.len() as u64);
    v.iter().for_each(|&x| w.u64(x));
}

fn leer_lista_u64(r: &mut Lector) -> Result<Vec<u64>, ErrorFormato> {
    (0..r.cuenta()?).map(|_| r.u64()).collect()
}

fn u32_de(v: u64, que: &'static str) -> Result<u32, ErrorFormato> {
    u32::try_from(v).map_err(|_| ErrorFormato(que))
}

// ───────────────────────────── Ticket ─────────────────────────────

fn contexto_escribir(w: &mut Escritor, c: &Contexto) {
    match *c {
        Contexto::Asedio { asentamiento } => {
            w.u8(0);
            w.u32(asentamiento);
        }
        Contexto::CampoAbierto { punto, solitarios } => {
            w.u8(1);
            w.punto(punto);
            w.bool(solitarios);
        }
        Contexto::Caravana { ref caravanas, punto, modalidad } => {
            w.u8(2);
            w.u64(caravanas.len() as u64);
            for k in caravanas {
                w.u64(k.id);
                w.u64(k.carros as u64);
            }
            w.punto(punto);
            w.u8(match modalidad {
                Modalidad::Disputa => 0,
                Modalidad::Aguante => 1,
            });
        }
        Contexto::CampamentoBandidos { campamento, punto } => {
            w.u8(3);
            w.u32(campamento);
            w.punto(punto);
        }
        Contexto::SaqueoPueblo { pueblo, punto } => {
            w.u8(4);
            w.u64(pueblo);
            w.punto(punto);
        }
        Contexto::Avanzada { avanzada, etapa, punto } => {
            w.u8(5);
            w.u64(avanzada);
            w.u8(match etapa {
                Etapa::Atalaya => 0,
                Etapa::Fuerte => 1,
                Etapa::Ciudadela => 2,
            });
            w.punto(punto);
        }
    }
}

fn contexto_leer(r: &mut Lector) -> Result<Contexto, ErrorFormato> {
    Ok(match r.u8()? {
        0 => Contexto::Asedio { asentamiento: r.u32()? },
        1 => Contexto::CampoAbierto { punto: r.punto()?, solitarios: r.bool()? },
        2 => {
            let caravanas = (0..r.cuenta()?).map(|_| Ok(CaravanaEnBatalla { id: r.u64()?, carros: r.u64()? as usize })).collect::<Result<_, ErrorFormato>>()?;
            let punto = r.punto()?;
            let modalidad = match r.u8()? {
                0 => Modalidad::Disputa,
                1 => Modalidad::Aguante,
                _ => return Err(ErrorFormato("modalidad de caravana desconocida")),
            };
            Contexto::Caravana { caravanas, punto, modalidad }
        }
        3 => Contexto::CampamentoBandidos { campamento: r.u32()?, punto: r.punto()? },
        4 => Contexto::SaqueoPueblo { pueblo: r.u64()?, punto: r.punto()? },
        5 => {
            let avanzada = r.u64()?;
            let etapa = match r.u8()? {
                0 => Etapa::Atalaya,
                1 => Etapa::Fuerte,
                2 => Etapa::Ciudadela,
                _ => return Err(ErrorFormato("etapa de avanzada desconocida")),
            };
            Contexto::Avanzada { avanzada, etapa, punto: r.punto()? }
        }
        _ => return Err(ErrorFormato("contexto de batalla desconocido")),
    })
}

fn escuadra_escribir(w: &mut Escritor, e: &EscuadraEnBatalla) {
    let EscuadraEnBatalla { id, tropa, dueno, efectivos, nivel, experiencia, moral } = e;
    w.u64(*id);
    w.u8(tropa_a_u8(*tropa));
    w.opt_u64(*dueno);
    w.i64(*efectivos);
    w.u8(*nivel);
    w.i64(*experiencia);
    w.i64(*moral);
}

fn escuadra_leer(r: &mut Lector) -> Result<EscuadraEnBatalla, ErrorFormato> {
    Ok(EscuadraEnBatalla { id: r.u64()?, tropa: tropa_de_u8(r.u8()?)?, dueno: r.opt_u64()?, efectivos: r.i64()?, nivel: r.u8()?, experiencia: r.i64()?, moral: r.i64()? })
}

fn participante_escribir(w: &mut Escritor, p: &Participante) {
    let Participante { heroe, faccion, controlador, casillas_libres, escuadras } = p;
    w.u64(*heroe);
    w.opt_u64(faccion.map(u64::from));
    w.u8(controlador_a_u8(*controlador));
    w.u32(*casillas_libres);
    w.u64(escuadras.len() as u64);
    escuadras.iter().for_each(|e| escuadra_escribir(w, e));
}

fn participante_leer(r: &mut Lector) -> Result<Participante, ErrorFormato> {
    let heroe = r.u64()?;
    let faccion = r.opt_u64()?.map(|f| u32_de(f, "Facción fuera de rango")).transpose()?;
    let (controlador, casillas_libres) = (controlador_de_u8(r.u8()?)?, r.u32()?);
    let escuadras = (0..r.cuenta()?).map(|_| escuadra_leer(r)).collect::<Result<_, _>>()?;
    Ok(Participante { heroe, faccion, controlador, casillas_libres, escuadras })
}

fn bando_escribir(w: &mut Escritor, b: &BandoDeTicket) {
    let BandoDeTicket { faccion, capacidad, participantes, sin_heroe } = b;
    w.opt_u64(faccion.map(u64::from));
    w.u64(*capacidad as u64);
    w.u64(participantes.len() as u64);
    participantes.iter().for_each(|p| participante_escribir(w, p));
    w.u64(sin_heroe.len() as u64);
    sin_heroe.iter().for_each(|e| escuadra_escribir(w, e));
}

fn bando_leer(r: &mut Lector) -> Result<BandoDeTicket, ErrorFormato> {
    let faccion = r.opt_u64()?.map(|f| u32_de(f, "Facción fuera de rango")).transpose()?;
    let capacidad = r.u64()? as usize;
    let participantes = (0..r.cuenta()?).map(|_| participante_leer(r)).collect::<Result<_, _>>()?;
    let sin_heroe = (0..r.cuenta()?).map(|_| escuadra_leer(r)).collect::<Result<_, _>>()?;
    Ok(BandoDeTicket { faccion, capacidad, participantes, sin_heroe })
}

fn plaza_escribir(w: &mut Escritor, p: &PlazaCongelada) {
    let PlazaCongelada { layout_version, unidades_por_celda, edificios, recintos } = p;
    w.u32(*layout_version);
    w.u32(*unidades_por_celda);
    w.u64(edificios.len() as u64);
    for EdificioCongelado { id, tipo, centro, ancho, alto, nivel_interno, estado, danado } in edificios {
        w.u32(*id);
        w.texto(tipo);
        w.punto(*centro);
        w.u32(*ancho);
        w.u32(*alto);
        w.u8(*nivel_interno);
        w.u8(match estado {
            EstadoEdificio::EnCola => 0,
            EstadoEdificio::EnConstruccion => 1,
            EstadoEdificio::Activo => 2,
        });
        w.bool(*danado);
    }
    w.u64(recintos.len() as u64);
    for RecintoCongelado { id, nivel, celdas } in recintos {
        w.u32(*id);
        w.u8(*nivel);
        escribir_celdas(w, celdas);
    }
}

fn plaza_leer(r: &mut Lector) -> Result<PlazaCongelada, ErrorFormato> {
    let (layout_version, unidades_por_celda) = (r.u32()?, r.u32()?);
    let edificios = (0..r.cuenta()?)
        .map(|_| {
            let (id, tipo, centro, ancho, alto, nivel_interno) = (r.u32()?, r.texto()?, r.punto()?, r.u32()?, r.u32()?, r.u8()?);
            let estado = match r.u8()? {
                0 => EstadoEdificio::EnCola,
                1 => EstadoEdificio::EnConstruccion,
                2 => EstadoEdificio::Activo,
                _ => return Err(ErrorFormato("estado de edificio congelado desconocido")),
            };
            Ok(EdificioCongelado { id, tipo, centro, ancho, alto, nivel_interno, estado, danado: r.bool()? })
        })
        .collect::<Result<_, _>>()?;
    let recintos = (0..r.cuenta()?).map(|_| Ok(RecintoCongelado { id: r.u32()?, nivel: r.u8()?, celdas: leer_celdas(r)? })).collect::<Result<_, ErrorFormato>>()?;
    Ok(PlazaCongelada { layout_version, unidades_por_celda, edificios, recintos })
}

fn ticket_escribir(w: &mut Escritor, t: &Ticket) {
    let Ticket { revision, contexto, duracion_maxima, atacante, defensor, plaza } = t;
    w.u32(*revision);
    contexto_escribir(w, contexto);
    w.i64(*duracion_maxima);
    bando_escribir(w, atacante);
    bando_escribir(w, defensor);
    match plaza {
        Some(p) => {
            w.bool(true);
            plaza_escribir(w, p);
        }
        None => w.bool(false),
    }
}

fn ticket_leer(r: &mut Lector) -> Result<Ticket, ErrorFormato> {
    let (revision, contexto, duracion_maxima, atacante, defensor) = (r.u32()?, contexto_leer(r)?, r.i64()?, bando_leer(r)?, bando_leer(r)?);
    let plaza = if r.bool()? { Some(plaza_leer(r)?) } else { None };
    Ok(Ticket { revision, contexto, duracion_maxima, atacante, defensor, plaza })
}

// ───────────────────────────── Asignación, salida y resultado (también en las órdenes del servidor de batalla) ─────────────────────────────

pub(crate) fn token_escribir(w: &mut Escritor, t: &Token) {
    let Token { heroe, token, expira_en } = t;
    w.u64(*heroe);
    w.texto(token);
    w.i64(*expira_en);
}

pub(crate) fn token_leer(r: &mut Lector) -> Result<Token, ErrorFormato> {
    Ok(Token { heroe: r.u64()?, token: r.texto()?, expira_en: r.i64()? })
}

pub(crate) fn tokens_escribir(w: &mut Escritor, v: &[Token]) {
    w.u64(v.len() as u64);
    v.iter().for_each(|t| token_escribir(w, t));
}

pub(crate) fn tokens_leer(r: &mut Lector) -> Result<Vec<Token>, ErrorFormato> {
    (0..r.cuenta()?).map(|_| token_leer(r)).collect()
}

pub(crate) fn asignacion_escribir(w: &mut Escritor, a: &Asignacion) {
    let Asignacion { intento, servidor, revision, instancia, tokens } = a;
    let Instancia { host, puerto, protocolo } = instancia;
    w.texto(intento);
    w.texto(servidor);
    w.u32(*revision);
    w.texto(host);
    w.u16(*puerto);
    w.texto(protocolo);
    tokens_escribir(w, tokens);
}

pub(crate) fn asignacion_leer(r: &mut Lector) -> Result<Asignacion, ErrorFormato> {
    let (intento, servidor, revision) = (r.texto()?, r.texto()?, r.u32()?);
    let instancia = Instancia { host: r.texto()?, puerto: r.u16()?, protocolo: r.texto()? };
    Ok(Asignacion { intento, servidor, revision, instancia, tokens: tokens_leer(r)? })
}

pub(crate) fn salida_escribir(w: &mut Escritor, s: &SalidaBatalla) {
    let SalidaBatalla { intento, revision, heroe, motivo, derrotado, escuadras } = s;
    w.texto(intento);
    w.u32(*revision);
    w.u64(*heroe);
    w.u8(motivo_a_u8(*motivo));
    w.bool(*derrotado);
    w.u64(escuadras.len() as u64);
    for EscuadraDeSalida { escuadra, supervivientes, muertos } in escuadras {
        w.u64(*escuadra);
        w.i64(*supervivientes);
        w.i64(*muertos);
    }
}

pub(crate) fn salida_leer(r: &mut Lector) -> Result<SalidaBatalla, ErrorFormato> {
    let (intento, revision, heroe, motivo, derrotado) = (r.texto()?, r.u32()?, r.u64()?, motivo_de_u8(r.u8()?)?, r.bool()?);
    let escuadras = (0..r.cuenta()?).map(|_| Ok(EscuadraDeSalida { escuadra: r.u64()?, supervivientes: r.i64()?, muertos: r.i64()? })).collect::<Result<_, ErrorFormato>>()?;
    Ok(SalidaBatalla { intento, revision, heroe, motivo, derrotado, escuadras })
}

fn botin_escribir(w: &mut Escritor, b: &Botin) {
    let Botin { objetos, monedas } = b;
    w.u64(objetos.len() as u64);
    for ObjetoBotin { definicion, cantidad, instancia } in objetos {
        w.texto(definicion);
        w.i64(*cantidad);
        w.bool(instancia.is_some());
        w.texto(instancia.as_deref().unwrap_or(""));
    }
    let Monedas { bronce, plata, oro } = monedas;
    [bronce, plata, oro].into_iter().for_each(|&m| w.i64(m));
}

fn botin_leer(r: &mut Lector) -> Result<Botin, ErrorFormato> {
    let mut objetos = Vec::new();
    for _ in 0..r.cuenta()? {
        let (definicion, cantidad, hay, texto) = (r.texto()?, r.i64()?, r.bool()?, r.texto()?);
        objetos.push(ObjetoBotin { definicion, cantidad, instancia: hay.then_some(texto) });
    }
    Ok(Botin { objetos, monedas: Monedas { bronce: r.i64()?, plata: r.i64()?, oro: r.i64()? } })
}

pub(crate) fn resultado_escribir(w: &mut Escritor, res: &Resultado) {
    let Resultado { resultado_id, revision, intento, inicio, fin, ganador, razon, objetivos, por_escuadra, por_heroe, version_servidor, carros_perdidos } = res;
    w.texto(resultado_id);
    w.u32(*revision);
    w.texto(intento);
    w.i64(*inicio);
    w.i64(*fin);
    w.u8(lado_a_u8(*ganador));
    w.u8(match razon {
        Razon::ObjetivosCapturados => 0,
        Razon::Aniquilacion => 1,
        Razon::TiempoAgotado => 2,
    });
    w.u64(objetivos.len() as u64);
    for (id, por) in objetivos {
        w.texto(id);
        w.u8(por.map_or(2, lado_a_u8));
    }
    w.u64(por_escuadra.len() as u64);
    for ResultadoEscuadra { escuadra, desplegados, supervivientes_al_cierre, muertos, xp_ganada } in por_escuadra {
        w.u64(*escuadra);
        [desplegados, supervivientes_al_cierre, muertos, xp_ganada].into_iter().for_each(|&n| w.i64(n));
    }
    w.u64(por_heroe.len() as u64);
    for ResultadoHeroe { heroe, participo, sobrevivio_al_cierre, xp_ganada, derrotado, botin } in por_heroe {
        w.u64(*heroe);
        w.bool(*participo);
        w.bool(*sobrevivio_al_cierre);
        w.i64(*xp_ganada);
        w.u8(derrotado.map_or(2, u8::from));
        w.bool(botin.is_some());
        if let Some(b) = botin {
            botin_escribir(w, b);
        }
    }
    w.texto(version_servidor);
    w.u64(carros_perdidos.len() as u64);
    carros_perdidos.iter().for_each(|c| w.texto(c));
}

pub(crate) fn resultado_leer(r: &mut Lector) -> Result<Resultado, ErrorFormato> {
    let (resultado_id, revision, intento, inicio, fin) = (r.texto()?, r.u32()?, r.texto()?, r.i64()?, r.i64()?);
    let ganador = lado_de_u8(r.u8()?)?;
    let razon = match r.u8()? {
        0 => Razon::ObjetivosCapturados,
        1 => Razon::Aniquilacion,
        2 => Razon::TiempoAgotado,
        _ => return Err(ErrorFormato("razón de victoria desconocida")),
    };
    let mut objetivos = Vec::new();
    for _ in 0..r.cuenta()? {
        let id = r.texto()?;
        objetivos.push((id, match r.u8()? {
            2 => None,
            i => Some(lado_de_u8(i)?),
        }));
    }
    let por_escuadra = (0..r.cuenta()?)
        .map(|_| Ok(ResultadoEscuadra { escuadra: r.u64()?, desplegados: r.i64()?, supervivientes_al_cierre: r.i64()?, muertos: r.i64()?, xp_ganada: r.i64()? }))
        .collect::<Result<_, ErrorFormato>>()?;
    let mut por_heroe = Vec::new();
    for _ in 0..r.cuenta()? {
        let (heroe, participo, sobrevivio_al_cierre, xp_ganada) = (r.u64()?, r.bool()?, r.bool()?, r.i64()?);
        let derrotado = match r.u8()? {
            0 => Some(false),
            1 => Some(true),
            2 => None,
            _ => return Err(ErrorFormato("«derrotado» desconocido")),
        };
        let botin = if r.bool()? { Some(botin_leer(r)?) } else { None };
        por_heroe.push(ResultadoHeroe { heroe, participo, sobrevivio_al_cierre, xp_ganada, derrotado, botin });
    }
    let version_servidor = r.texto()?;
    let carros_perdidos = (0..r.cuenta()?).map(|_| r.texto()).collect::<Result<_, _>>()?;
    Ok(Resultado { resultado_id, revision, intento, inicio, fin, ganador, razon, objetivos, por_escuadra, por_heroe, version_servidor, carros_perdidos })
}

// ───────────────────────────── Batalla ─────────────────────────────

fn batalla_escribir(w: &mut Escritor, b: &Batalla) {
    let Batalla { id, estado, iniciada_por, punto, ticket, incorporaciones, salidas, bloqueo, expira_en, convocatoria_hasta, iniciada_en, aplicada_en, cerrada_en, asignacion, resultado } = b;
    w.u64(*id);
    w.u8(estado_a_u8(*estado));
    w.u64(*iniciada_por);
    w.punto(*punto);
    ticket_escribir(w, ticket);
    w.u64(incorporaciones.len() as u64);
    for Incorporacion { secuencia, lado, participante } in incorporaciones {
        w.u32(*secuencia);
        w.u8(lado_a_u8(*lado));
        participante_escribir(w, participante);
    }
    w.u64(salidas.len() as u64);
    for Salida { heroe, motivo, derrotado, en } in salidas {
        w.u64(*heroe);
        w.u8(motivo_a_u8(*motivo));
        w.bool(*derrotado);
        w.i64(*en);
    }
    let Bloqueo { ejercitos, caravanas, asentamiento, campamento, pueblo, avanzada } = bloqueo;
    lista_u64(w, ejercitos);
    lista_u64(w, caravanas);
    w.opt_u64(asentamiento.map(u64::from));
    w.opt_u64(campamento.map(u64::from));
    w.opt_u64(*pueblo);
    w.opt_u64(*avanzada);
    w.i64(*expira_en);
    [convocatoria_hasta, iniciada_en, aplicada_en, cerrada_en].into_iter().for_each(|&o| w.opt_i64(o));
    w.bool(asignacion.is_some());
    if let Some(a) = asignacion {
        asignacion_escribir(w, a);
    }
    w.bool(resultado.is_some());
    if let Some(res) = resultado {
        resultado_escribir(w, res);
    }
}

fn batalla_leer(r: &mut Lector) -> Result<Batalla, ErrorFormato> {
    let (id, estado, iniciada_por, punto, ticket) = (r.u64()?, estado_de_u8(r.u8()?)?, r.u64()?, r.punto()?, ticket_leer(r)?);
    let incorporaciones = (0..r.cuenta()?).map(|_| Ok(Incorporacion { secuencia: r.u32()?, lado: lado_de_u8(r.u8()?)?, participante: participante_leer(r)? })).collect::<Result<_, ErrorFormato>>()?;
    let salidas = (0..r.cuenta()?).map(|_| Ok(Salida { heroe: r.u64()?, motivo: motivo_de_u8(r.u8()?)?, derrotado: r.bool()?, en: r.i64()? })).collect::<Result<_, ErrorFormato>>()?;
    let (ejercitos, caravanas) = (leer_lista_u64(r)?, leer_lista_u64(r)?);
    let asentamiento = r.opt_u64()?.map(|a| u32_de(a, "plaza fuera de rango")).transpose()?;
    let campamento = r.opt_u64()?.map(|c| u32_de(c, "campamento fuera de rango")).transpose()?;
    let bloqueo = Bloqueo { ejercitos, caravanas, asentamiento, campamento, pueblo: r.opt_u64()?, avanzada: r.opt_u64()? };
    let (expira_en, convocatoria_hasta, iniciada_en, aplicada_en, cerrada_en) = (r.i64()?, r.opt_i64()?, r.opt_i64()?, r.opt_i64()?, r.opt_i64()?);
    let asignacion = if r.bool()? { Some(asignacion_leer(r)?) } else { None };
    let resultado = if r.bool()? { Some(resultado_leer(r)?) } else { None };
    Ok(Batalla { id, estado, iniciada_por, punto, ticket, incorporaciones, salidas, bloqueo, expira_en, convocatoria_hasta, iniciada_en, aplicada_en, cerrada_en, asignacion, resultado })
}

pub(crate) fn escribir(w: &mut Escritor, b: &EstadoBatallas) {
    let EstadoBatallas { batallas } = b;
    w.u64(batallas.len() as u64);
    batallas.values().for_each(|x| batalla_escribir(w, x));
}

pub(crate) fn leer(r: &mut Lector) -> Result<EstadoBatallas, ErrorFormato> {
    let mut b = EstadoBatallas::default();
    for _ in 0..r.cuenta()? {
        let x = batalla_leer(r)?;
        b.batallas.insert(x.id, x);
    }
    Ok(b)
}
