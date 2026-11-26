//! El recorte del lugar que el motor mete en el ticket, en el JSON de la mesa del campo (`TicketDelLugar`, `pruebas/campo3d/generador.ts` de
//! BronzeAgeClient), para cargarlo allí con «cargar ticket» y comparar (entrada 78).
//!
//! `cargo run -p nucleo --example ticket_del_lugar -- <x u> <y u> [grande] > ticket.json`, con el mundo y su relieve en `generador/salida/mundo/`.
//! Los dos bandos van quietos (la batalla no sale de una orden): la mesa coloca los spawns con la semilla del lugar.

use combate::ciclo::{BandoDeTicket, Batalla, Contexto, Ticket};
use contrato::v1 as pb;
use mundo::{Mundo, OpcionesValidacion, RelieveFino};
use nucleo::{Ajustes, Arranque, Nucleo, Reglas};
use serde_json::{json, Value};
use std::sync::Arc;

fn u(v: i64) -> f64 {
    v as f64 / 1000.0
}

fn punto(p: &Option<pb::Punto>) -> Value {
    let p = p.clone().unwrap_or_default();
    json!({ "x": u(p.x), "y": u(p.y) })
}

fn rejilla(r: &Option<pb::RejillaDelLugar>) -> Value {
    let r = r.clone().unwrap_or_default();
    json!({ "origen": punto(&r.origen), "paso": u(i64::from(r.paso)), "columnas": r.columnas, "filas": r.filas, "valores": r.valores })
}

/// La clase de terreno de cada celda del recorte por la altura de la muestra más cercana a su centro, con los umbrales del generador
/// (`generador/src/config.ts`: agua 0,375, costa 0,42, colina 0,545, montaña 0,6, cima 0,685; como la recalcula el mapa de mundo).
fn terreno_de_las_alturas(r: &pb::RecorteDelLugar, alturas: &[u16]) -> Value {
    const UMBRALES: [f64; 5] = [0.375, 0.42, 0.545, 0.6, 0.685];
    let (b, rel) = (r.bosque.clone().unwrap(), r.relieve.clone().unwrap());
    let o = rel.origen.unwrap();
    let mut valores = vec![];
    for j in 0..b.filas as i64 {
        for i in 0..b.columnas as i64 {
            let ob = b.origen.clone().unwrap();
            let (x, y) = (ob.x + i * i64::from(b.paso) + i64::from(b.paso) / 2, ob.y + j * i64::from(b.paso) + i64::from(b.paso) / 2);
            let (mi, mj) = (((x - o.x) / i64::from(rel.paso)).clamp(0, i64::from(rel.columnas) - 1), ((y - o.y) / i64::from(rel.paso)).clamp(0, i64::from(rel.filas) - 1));
            let k = (mj * i64::from(rel.columnas) + mi) as usize;
            let agua = rel.agua[k >> 2] >> (2 * (k & 3)) & 3 != 0;
            let e = f64::from(alturas[k]) / 65535.0;
            valores.push(if agua { 1 } else { 2 + UMBRALES[1..].iter().filter(|&&u| e >= u).count() as u8 });
        }
    }
    json!({ "origen": punto(&b.origen), "paso": u(i64::from(b.paso)), "columnas": b.columnas, "filas": b.filas, "valores": valores })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (x, y): (f64, f64) = (args[0].parse().expect("x en u"), args[1].parse().expect("y en u"));
    let grande = args.get(2).is_some_and(|a| a == "grande");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo");
    let fichero = |prefijo: &str| std::fs::read_dir(&dir).unwrap().flatten().find(|e| e.file_name().to_string_lossy().starts_with(prefijo)).expect("mundo generado").path();
    let m = Mundo::cargar(&std::fs::read(fichero("mundo-")).unwrap(), &OpcionesValidacion::default()).expect("artefacto válido");
    m.poner_relieve(RelieveFino::desde(&std::fs::read(fichero("relieve-")).unwrap()).expect("relieve válido"));
    let rutas = Arc::new(rutas::Rutas::desde_mundo(&m));
    let hash = m.hash().to_owned();
    let n = Nucleo::nuevo(Arranque { reglas: Reglas::default(), ajustes: Ajustes::default(), semilla: 1, inicio: 0, rutas, mundo_hash: hash.clone(), mundo: Some(Arc::new(m)) });

    let p = base::Punto::new((x * 1000.0).round() as i64, (y * 1000.0).round() as i64);
    let cr = &n.estado().reglas.economia.combate;
    let bando = || BandoDeTicket::nuevo(None, vec![], vec![]);
    // Grande: seis héroes en el atacante (`heroes_grande`).
    let participante = |heroe| combate::ciclo::Participante { heroe, faccion: None, controlador: militar::heroe::Controlador::Humano, escuadras: vec![] };
    let atacante = BandoDeTicket::nuevo(None, (1..=if grande { 6 } else { 1 }).map(participante).collect(), vec![]);
    let ticket = Ticket::nuevo(cr, Contexto::CampoAbierto { punto: p, solitarios: !grande }, atacante, bando());
    let b = Batalla::abrir(cr, 1, 1, p, ticket, Default::default(), 0);
    let t = nucleo::proyeccion::batalla::ticket(&n, &b);
    let Some(pb::battle_ticket::Mapa::MapaReferencia(mr)) = t.mapa else { panic!("campo abierto: mapa de referencia") };
    let r = mr.recorte.expect("con relieve hay recorte");
    let rel = r.relieve.clone().unwrap();
    let alturas: Vec<u16> = rel.alturas.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let procedencia = |p: &Option<pb::ProcedenciaDelLugar>| {
        let p = p.clone().unwrap();
        json!({ "punto": punto(&p.punto), "quieto": p.quieto })
    };
    let salida = json!({
        "version": 1,
        "mapa_id": hash,
        // La mesa pide el punto del defensor: quieto, es su procedencia.
        "punto": punto(&r.procedencia_defensor.as_ref().unwrap().punto),
        "centro": punto(&mr.centro),
        "contexto": { "tipo": "campo_abierto", "solitarios": !grande },
        "recorte": {
            "lado_recorte": u(i64::from(r.lado)),
            "alturas": { "origen": punto(&rel.origen), "paso": u(i64::from(rel.paso)), "columnas": rel.columnas, "filas": rel.filas, "valores": alturas, "metros_altura": 1000 },
            "agua": (0..(rel.columnas * rel.filas) as usize).map(|k| char::from(b'0' + (rel.agua[k >> 2] >> (2 * (k & 3)) & 3))).collect::<String>(),
            // La mesa aún lee la clase de terreno del ticket: aquí sale de la altura con los umbrales del mundo, como la sacará Conquest.
            "terreno": terreno_de_las_alturas(&r, &alturas),
            "bosque": rejilla(&r.bosque),
            "fertilidad": rejilla(&r.fertilidad),
            "humedad": rejilla(&r.humedad),
            "rios": r.rios.iter().map(|rio| json!({
                "puntos": rio.puntos.iter().map(|q| punto(&Some(q.clone()))).collect::<Vec<_>>(),
                "vados": rio.vados.iter().map(|q| punto(&Some(q.clone()))).collect::<Vec<_>>(),
                "navegable": rio.navegable,
                "anchos": rio.anchos.iter().map(|&a| u(i64::from(a))).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "caminos": [],
            "procedencia_atacante": procedencia(&r.procedencia_atacante),
            "procedencia_defensor": procedencia(&r.procedencia_defensor),
        },
    });
    println!("{}", serde_json::to_string_pretty(&salida).unwrap());
    eprintln!("recorte: {} bytes en protobuf", prost::Message::encoded_len(&r));
}
