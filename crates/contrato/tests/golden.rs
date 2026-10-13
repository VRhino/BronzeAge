//! Ficheros dorados (`contratos/golden`): el .bin lo genera `contratos/ts`; aquí se decodifica y se vuelve a codificar igual.
use contrato::v1::*;
use prost::Message;

fn ida_y_vuelta<M: Message + Default>(bin: &[u8]) {
    let m = M::decode(bin).unwrap();
    assert_eq!(m.encode_to_vec(), bin);
}

#[test]
fn dorados() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contratos/golden");
    let mut n = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_none_or(|x| x != "bin") {
            continue;
        }
        let bin = std::fs::read(&p).unwrap();
        let nombre = p.file_stem().unwrap().to_str().unwrap().to_string();
        match nombre.split("__").next().unwrap() {
            "MensajeDeCliente" => ida_y_vuelta::<MensajeDeCliente>(&bin),
            "MensajeDeServidor" => ida_y_vuelta::<MensajeDeServidor>(&bin),
            "ArtefactoDeMundo" => ida_y_vuelta::<ArtefactoDeMundo>(&bin),
            t => panic!("tipo dorado desconocido: {t}"),
        }
        n += 1;
    }
    assert!(n >= 10, "faltan ficheros dorados: {n}");
}
