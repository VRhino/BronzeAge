use std::path::PathBuf;

fn main() {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../proto");
    let ficheros = ["common", "entities", "orders", "events", "battle", "session", "world"]
        .map(|f| raiz.join(format!("bronzeage/v1/{f}.proto")));
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());
    prost_build::compile_protos(&ficheros, &[raiz]).unwrap();
    println!("cargo:rerun-if-changed=../../proto");
}
