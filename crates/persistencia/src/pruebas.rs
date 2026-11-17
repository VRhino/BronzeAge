use super::*;
use crate::registro::listar_segmentos;
use std::fs::OpenOptions;
use std::time::{Duration, Instant};
use tempfile::tempdir;

const LEN: usize = 12;
const REG: u64 = (8 + 18 + LEN) as u64; // bytes por registro con datos de LEN

fn datos(i: u64) -> Vec<u8> {
    (0..LEN).map(|k| (i as usize * 31 + k) as u8).collect()
}

fn cfg() -> Config {
    Config { sincronia: Sincronia::Nunca, ..Config::default() }
}

fn escribir(a: &mut Almacen, n: u64) {
    for i in 1..=n {
        let s = a.registro().anadir(1000 + i as i64, (i % 5) as u16, &datos(i)).unwrap();
        assert_eq!(s, i);
    }
}

fn leer_todo(a: &Almacen) -> (Option<Foto>, Vec<Entrada>, Vec<Aviso>) {
    let r = a.recuperar().unwrap();
    (r.foto, r.entradas_posteriores.collect(), r.avisos)
}

fn ultimo_segmento(dir: &Path) -> PathBuf {
    listar_segmentos(dir).unwrap().pop().unwrap().ruta
}

#[test]
fn ida_y_vuelta_en_orden_y_sin_huecos() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    escribir(&mut a, 100);
    drop(a);
    let a = Almacen::abrir(d.path(), cfg()).unwrap();
    let (foto, es, avisos) = leer_todo(&a);
    assert!(foto.is_none() && avisos.is_empty());
    assert_eq!(es.len(), 100);
    for (i, e) in es.iter().enumerate() {
        let n = i as u64 + 1;
        assert_eq!(*e, Entrada { secuencia: n, instante: 1000 + n as i64, tipo: (n % 5) as u16, datos: datos(n) });
    }
}

#[test]
fn lote_y_datos_vacios() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), Config { sincronia: Sincronia::PorLote, ..cfg() }).unwrap();
    let ult = a.registro().anadir_lote([(5, 1, &b""[..]), (6, 2, &b"xy"[..])]).unwrap();
    assert_eq!(ult, 2);
    let (_, es, _) = leer_todo(&a);
    assert_eq!(es.iter().map(|e| (e.secuencia, e.instante, e.tipo, e.datos.clone())).collect::<Vec<_>>(), vec![
        (1, 5, 1, vec![]),
        (2, 6, 2, b"xy".to_vec())
    ]);
}

#[test]
fn corte_a_mitad_de_registro() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    escribir(&mut a, 10);
    drop(a);
    let ruta = ultimo_segmento(d.path());
    let f = OpenOptions::new().write(true).open(&ruta).unwrap();
    f.set_len(10 * REG - 5).unwrap(); // el décimo queda a medias
    drop(f);

    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    let (_, es, avisos) = leer_todo(&a);
    assert_eq!(es.len(), 9);
    assert!(matches!(avisos[..], [Aviso::RegistroTruncado { desplazamiento, descartados, .. }] if desplazamiento == 9 * REG && descartados == REG - 5));
    // tras reparar se puede seguir escribiendo sin huecos
    assert_eq!(a.registro().anadir(0, 0, b"nuevo").unwrap(), 10);
    let (_, es, _) = leer_todo(&a);
    assert_eq!(es.len(), 10);
    assert_eq!(es[9].datos, b"nuevo");
    assert_eq!(fs::metadata(ultimo_segmento(d.path())).unwrap().len(), 9 * REG + 8 + 18 + 5);
}

#[test]
fn corte_en_la_cabecera() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    escribir(&mut a, 3);
    drop(a);
    OpenOptions::new().write(true).open(ultimo_segmento(d.path())).unwrap().set_len(2 * REG + 3).unwrap();
    let a = Almacen::abrir(d.path(), cfg()).unwrap();
    let (_, es, avisos) = leer_todo(&a);
    assert_eq!(es.len(), 2);
    assert!(matches!(avisos[..], [Aviso::RegistroTruncado { descartados: 3, .. }]));
}

#[test]
fn byte_corrupto_en_el_centro_corta_ahi_y_no_lee_basura() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    escribir(&mut a, 10);
    drop(a);
    let ruta = ultimo_segmento(d.path());
    let mut b = fs::read(&ruta).unwrap();
    b[(4 * REG + 8 + 18 + 3) as usize] ^= 0xFF; // un byte de los datos del registro 5
    fs::write(&ruta, &b).unwrap();

    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    let (_, es, avisos) = leer_todo(&a);
    assert_eq!(es.iter().map(|e| e.secuencia).collect::<Vec<_>>(), vec![1, 2, 3, 4]);
    assert!(matches!(avisos[..], [Aviso::RegistroCorrupto { desplazamiento, .. }] if desplazamiento == 4 * REG));
    assert_eq!(a.registro().anadir(0, 0, b"z").unwrap(), 5);
    assert_eq!(leer_todo(&a).1.len(), 5);
}

#[test]
fn longitud_corrupta_no_provoca_lecturas_enormes() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    escribir(&mut a, 3);
    drop(a);
    let ruta = ultimo_segmento(d.path());
    let mut b = fs::read(&ruta).unwrap();
    b[REG as usize..REG as usize + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    fs::write(&ruta, &b).unwrap();
    let a = Almacen::abrir(d.path(), cfg()).unwrap();
    let (_, es, avisos) = leer_todo(&a);
    assert_eq!(es.len(), 1);
    assert!(matches!(avisos[..], [Aviso::RegistroCorrupto { .. }]));
}

#[test]
fn rotacion_de_segmentos_y_dano_en_uno_intermedio() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), Config { max_segmento: 5 * REG, ..cfg() }).unwrap();
    escribir(&mut a, 23);
    drop(a);
    let segs = listar_segmentos(d.path()).unwrap();
    assert_eq!(segs.iter().map(|s| s.inicio).collect::<Vec<_>>(), vec![1, 6, 11, 16, 21]);

    // todo se recupera en orden a través de los segmentos
    let a = Almacen::abrir(d.path(), Config { max_segmento: 5 * REG, ..cfg() }).unwrap();
    let (_, es, avisos) = leer_todo(&a);
    assert!(avisos.is_empty());
    assert_eq!(es.iter().map(|e| e.secuencia).collect::<Vec<_>>(), (1..=23).collect::<Vec<_>>());
    drop(a);

    // daño en el segmento 2: lo posterior se aparta y se sigue desde la secuencia 9
    let mut b = fs::read(&segs[1].ruta).unwrap();
    b[(2 * REG + 20) as usize] ^= 1; // registro con secuencia 8
    fs::write(&segs[1].ruta, &b).unwrap();
    let mut a = Almacen::abrir(d.path(), Config { max_segmento: 5 * REG, ..cfg() }).unwrap();
    let (_, es, avisos) = leer_todo(&a);
    assert_eq!(es.len(), 7);
    assert_eq!(avisos.iter().filter(|a| matches!(a, Aviso::SegmentoDescartado { .. })).count(), 3);
    assert_eq!(a.registro().anadir(0, 0, b"").unwrap(), 8);
    assert_eq!(leer_todo(&a).1.len(), 8);
}

#[test]
fn foto_mas_entradas_posteriores() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    escribir(&mut a, 10);
    a.fotos().guardar(&Foto::nueva(6, 1006, b"estado hasta 6".to_vec())).unwrap();
    let (foto, es, avisos) = leer_todo(&a);
    let f = foto.unwrap();
    assert_eq!((f.secuencia, f.instante, f.datos.as_slice()), (6, 1006, &b"estado hasta 6"[..]));
    assert_eq!(es.iter().map(|e| e.secuencia).collect::<Vec<_>>(), vec![7, 8, 9, 10]);
    assert!(avisos.is_empty());
}

#[test]
fn foto_corrupta_cae_a_la_anterior() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    escribir(&mut a, 10);
    let fotos = a.fotos();
    fotos.guardar(&Foto::nueva(4, 1004, vec![4; 5000])).unwrap();
    fotos.guardar(&Foto::nueva(8, 1008, vec![8; 5000])).unwrap();
    let ruta = foto::listar(d.path()).unwrap().pop().unwrap().1;

    for estropicio in [|b: &mut Vec<u8>| b[60] ^= 0x55, |b: &mut Vec<u8>| b.truncate(30), |b: &mut Vec<u8>| b[10] ^= 1] {
        let original = fs::read(&ruta).unwrap();
        let mut b = original.clone();
        estropicio(&mut b);
        fs::write(&ruta, &b).unwrap();
        let (foto, es, avisos) = leer_todo(&a);
        assert_eq!(foto.unwrap().secuencia, 4);
        assert_eq!(es.iter().map(|e| e.secuencia).collect::<Vec<_>>(), (5..=10).collect::<Vec<_>>());
        assert!(matches!(avisos[..], [Aviso::FotoCorrupta { .. }]));
        fs::write(&ruta, original).unwrap();
    }
    assert_eq!(leer_todo(&a).0.unwrap().secuencia, 8);
}

#[test]
fn solo_se_conservan_n_fotos_y_no_quedan_temporales() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), Config { fotos_conservadas: 2, ..cfg() }).unwrap();
    escribir(&mut a, 5);
    for s in 1..=5 {
        a.fotos().guardar(&Foto::nueva(s, 0, vec![s as u8; 100])).unwrap();
    }
    assert_eq!(foto::listar(d.path()).unwrap().iter().map(|f| f.0).collect::<Vec<_>>(), vec![4, 5]);
    // un temporal huérfano (corte durante la escritura de una foto) se limpia al abrir y no estorba
    fs::write(d.path().join("foto-00000000000000000009.tmp"), b"a medias").unwrap();
    drop(a);
    let a = Almacen::abrir(d.path(), cfg()).unwrap();
    assert!(!d.path().join("foto-00000000000000000009.tmp").exists());
    assert_eq!(leer_todo(&a).0.unwrap().secuencia, 5);
}

#[test]
fn compactar_borra_segmentos_anteriores_a_la_foto_mas_antigua() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), Config { max_segmento: 5 * REG, fotos_conservadas: 2, ..cfg() }).unwrap();
    escribir(&mut a, 23); // segmentos desde 1, 6, 11, 16, 21
    assert_eq!(a.compactar().unwrap(), 0, "sin foto no se borra nada");
    let fotos = a.fotos();
    fotos.guardar(&Foto::nueva(12, 0, b"f12".to_vec())).unwrap();
    fotos.guardar(&Foto::nueva(18, 0, b"f18".to_vec())).unwrap();
    // la más antigua conservada es la 12: sirve el segmento que empieza en 11 (cubre 11..15), caen 1 y 6
    assert_eq!(a.compactar().unwrap(), 2);
    assert_eq!(listar_segmentos(d.path()).unwrap().iter().map(|s| s.inicio).collect::<Vec<_>>(), vec![11, 16, 21]);
    assert_eq!(a.compactar().unwrap(), 0);
    // cualquiera de las fotos conservadas sigue pudiendo recuperar
    let (f, es, _) = leer_todo(&a);
    assert_eq!((f.unwrap().secuencia, es.len()), (18, 5));
    fs::remove_file(foto::listar(d.path()).unwrap().pop().unwrap().1).unwrap();
    let (f, es, avisos) = leer_todo(&a);
    assert_eq!((f.unwrap().secuencia, es.len()), (12, 11));
    assert!(avisos.is_empty());
}

#[test]
fn recuperar_es_idempotente_y_no_modifica_disco() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    escribir(&mut a, 20);
    a.fotos().guardar(&Foto::nueva(10, 7, b"x".to_vec())).unwrap();
    drop(a);
    let ruta = ultimo_segmento(d.path());
    let tam = fs::metadata(&ruta).unwrap().len();
    OpenOptions::new().write(true).open(&ruta).unwrap().set_len(tam - 3).unwrap(); // cola dañada

    let a = Almacen::abrir(d.path(), cfg()).unwrap();
    let tam_tras_abrir = fs::metadata(&ruta).unwrap().len();
    let (f1, e1, a1) = leer_todo(&a);
    let (f2, e2, a2) = leer_todo(&a);
    assert_eq!((&f1, &e1, &a1), (&f2, &e2, &a2));
    assert_eq!(a1.len(), 1);
    assert_eq!(fs::metadata(&ruta).unwrap().len(), tam_tras_abrir);
    // y reabrir otra vez da el mismo mundo (los avisos de la reparación ya no se repiten porque ya está reparado)
    drop(a);
    let a = Almacen::abrir(d.path(), cfg()).unwrap();
    let (f3, e3, a3) = leer_todo(&a);
    assert_eq!((f1, e1), (f3, e3));
    assert!(a3.is_empty());
}

#[test]
fn la_secuencia_continua_tras_reabrir_y_nunca_se_reutiliza_la_de_una_foto() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    escribir(&mut a, 5);
    drop(a);
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    assert_eq!(a.registro().anadir(0, 0, b"").unwrap(), 6);

    // la foto 20 existe aunque el registro se quedó en 6 (cola perdida por un corte de corriente)
    a.fotos().guardar(&Foto::nueva(20, 0, b"estado".to_vec())).unwrap();
    drop(a);
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    assert_eq!(a.registro().anadir(0, 0, b"").unwrap(), 21);
    assert_eq!(a.registro().anadir(0, 0, b"").unwrap(), 22);
    drop(a);
    let a = Almacen::abrir(d.path(), cfg()).unwrap();
    let (f, es, avisos) = leer_todo(&a);
    assert_eq!(f.unwrap().secuencia, 20);
    assert_eq!(es.iter().map(|e| e.secuencia).collect::<Vec<_>>(), vec![21, 22]);
    assert!(avisos.is_empty());
}

#[test]
fn hueco_se_reporta_si_la_foto_valida_no_llega_al_registro() {
    let d = tempdir().unwrap();
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    escribir(&mut a, 4);
    a.fotos().guardar(&Foto::nueva(2, 0, b"f2".to_vec())).unwrap();
    a.fotos().guardar(&Foto::nueva(20, 0, b"f20".to_vec())).unwrap();
    drop(a);
    let mut a = Almacen::abrir(d.path(), cfg()).unwrap();
    a.registro().anadir(0, 0, b"").unwrap(); // secuencia 21
    // si la foto 20 se estropea, desde la 2 faltan las secuencias 5..=20
    let ruta = foto::listar(d.path()).unwrap().pop().unwrap().1;
    fs::write(ruta, b"basura").unwrap();
    let (f, es, avisos) = leer_todo(&a);
    assert_eq!(f.unwrap().secuencia, 2);
    assert_eq!(es.iter().map(|e| e.secuencia).collect::<Vec<_>>(), vec![3, 4]);
    assert!(avisos.contains(&Aviso::Hueco { esperada: 5, hallada: 21 }));
}

#[test]
fn politicas_de_sincronia_no_cambian_el_resultado() {
    for s in [Sincronia::Nunca, Sincronia::PorLote, Sincronia::Intervalo(Duration::ZERO), Sincronia::Intervalo(Duration::from_secs(3600))] {
        let d = tempdir().unwrap();
        let mut a = Almacen::abrir(d.path(), Config { sincronia: s, max_segmento: 10 * REG, ..cfg() }).unwrap();
        escribir(&mut a, 25);
        a.registro().sincronizar().unwrap();
        assert_eq!(leer_todo(&a).1.len(), 25);
    }
}

/// `cargo test -p persistencia --release -- --ignored --nocapture medir`
#[test]
#[ignore]
fn medir_rendimiento() {
    let carga = [7u8; 100];
    let medir = |nombre: &str, s: Sincronia, n: u64, lote: usize| {
        let d = tempdir().unwrap();
        let mut a = Almacen::abrir(d.path(), Config { sincronia: s, ..Config::default() }).unwrap();
        let t = Instant::now();
        for _ in 0..n / lote as u64 {
            a.registro().anadir_lote((0..lote).map(|i| (i as i64, 1u16, &carga[..]))).unwrap();
        }
        a.registro().sincronizar().unwrap();
        let e = t.elapsed();
        println!("escritura {nombre:<28} lote {lote:>4}: {:>10.0} registros/s", n as f64 / e.as_secs_f64());
        let t = Instant::now();
        let r = a.recuperar().unwrap();
        let total = r.entradas_posteriores.count() as u64;
        println!("lectura   {nombre:<28} {:>10.0} registros/s ({total} leídos)", total as f64 / t.elapsed().as_secs_f64());
    };
    medir("sin fsync", Sincronia::Nunca, 1_000_000, 1);
    medir("sin fsync", Sincronia::Nunca, 1_000_000, 100);
    medir("fsync por lote (lote 1)", Sincronia::PorLote, 2_000, 1);
    medir("fsync por lote (lote 10)", Sincronia::PorLote, 10_000, 10);
    medir("fsync por lote (lote 100)", Sincronia::PorLote, 100_000, 100);
    medir("fsync cada 50 ms (lote 1)", Sincronia::Intervalo(Duration::from_millis(50)), 200_000, 1);

    let estado: Vec<u8> = (0..50_000_000u64).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8 % 16).collect();
    let d = tempdir().unwrap();
    let a = Almacen::abrir(d.path(), Config::default()).unwrap();
    let t = Instant::now();
    a.fotos().guardar(&Foto::nueva(1, 0, estado)).unwrap();
    println!("foto de 50 MB: guardar {:?}", t.elapsed());
    let t = Instant::now();
    let r = a.recuperar().unwrap();
    println!("foto de 50 MB: cargar y validar {:?} ({} bytes)", t.elapsed(), r.foto.unwrap().datos.len());
}
