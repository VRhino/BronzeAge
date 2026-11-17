//! El espejo de la carpeta de datos en una base de datos remota, para alojar el servidor donde el disco no dura (un contenedor que se borra en cada despliegue).
//!
//! El servidor no cambia: sigue escribiendo su registro, sus fotos, su crónica y sus cuentas en la carpeta de datos. El espejo copia esa carpeta, troceada, a una tabla remota cada
//! pocos segundos y una última vez al parar; y al arrancar con la carpeta vacía, la baja de allí antes de abrir nada. Una parada normal no pierde nada; si el proceso muere de golpe se
//! pierde, como mucho, lo escrito desde la última copia.
//!
//! - Cada fichero va en trozos de [`TROZO`] bytes con su huella. Una vuelta solo sube los trozos cuya huella cambió, así que un registro que crece sube su cola y poco más.
//! - El orden de una vuelta protege la recuperación: primero se anota qué fotos hay, luego se sube todo lo demás (el registro incluido) y al final esas fotos. Así una foto remota
//!   nunca apunta a entradas del registro que aún no han subido. Los borrados van los últimos.
//! - Si la carpeta local y el remoto tienen partidas distintas (`partida.id`), no se toca nada y el arranque falla: mejor parar que pisar una partida con otra.
//!
//! El remoto es un [`Remoto`]: [`Turso`] (libSQL por HTTP, el de producción) o [`EnMemoria`] (pruebas).

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, SystemTime},
};

use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Tamaño de un trozo. Pequeño a propósito: la cola del registro cambia en cada vuelta y se vuelve a subir entera.
pub const TROZO: usize = 64 << 10;
/// Trozos por petición al remoto (alrededor de 1 MB de datos).
const LOTE: usize = 16;

pub type Huella = [u8; 16];

fn huella(datos: &[u8]) -> Huella {
    let mut h = [0u8; 16];
    h.copy_from_slice(&Sha256::digest(datos)[..16]);
    h
}

fn hex(h: &Huella) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn de_hex(s: &str) -> Option<Huella> {
    let mut h = [0u8; 16];
    if s.len() != 32 {
        return None;
    }
    for (i, b) in h.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(h)
}

/// Un cambio en el remoto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cambio {
    /// Pone (o sustituye) un trozo.
    Poner { ruta: String, trozo: u32, huella: Huella, datos: Vec<u8> },
    /// Borra los trozos de `ruta` desde `desde` (0: el fichero entero).
    Cortar { ruta: String, desde: u32 },
}

/// Dónde vive la copia. Las rutas son relativas a la carpeta de datos, con `/`.
pub trait Remoto: Send {
    /// Todos los trozos que hay: ruta, número y huella, ordenados.
    fn indice(&mut self) -> io::Result<Vec<(String, u32, Huella)>>;
    /// Los datos de los trozos `desde..desde + cuantos` de `ruta`, en orden (los que haya).
    fn leer(&mut self, ruta: &str, desde: u32, cuantos: u32) -> io::Result<Vec<Vec<u8>>>;
    /// Aplica los cambios, todos o ninguno.
    fn aplicar(&mut self, cambios: &[Cambio]) -> io::Result<()>;
}

/// Lo que el espejo sabe de un fichero: las huellas de sus trozos en el remoto y cómo estaba en disco la última vez que lo miró (`None`: aún no lo ha mirado en este proceso).
#[derive(Clone, Debug, Default)]
struct Fichero {
    huellas: Vec<Huella>,
    visto: Option<(u64, SystemTime)>,
}

pub struct Espejo {
    dir: PathBuf,
    remoto: Box<dyn Remoto>,
    conocido: BTreeMap<String, Fichero>,
    /// Una vuelta falló a medias: lo conocido puede no ser lo que hay en el remoto, y la siguiente vuelta lo vuelve a leer de allí.
    dudoso: bool,
}

/// Qué pasó al abrir.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Apertura {
    /// Ni en disco ni en el remoto había partida.
    Vacio,
    /// La carpeta estaba vacía y se bajó la partida del remoto: tantos ficheros.
    Bajada { ficheros: usize },
    /// La carpeta ya tenía partida (la misma del remoto, o el remoto estaba vacío): manda el disco.
    Local,
}

/// El resumen de una vuelta.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vuelta {
    pub trozos: usize,
    pub bytes: usize,
    pub borrados: usize,
}

fn es_foto(ruta: &str) -> bool {
    ruta.rsplit('/').next().is_some_and(|n| n.starts_with("foto-"))
}

/// Los ficheros de `dir`, con su ruta relativa. Los temporales (`.tmp`) no cuentan: son escrituras a medias.
fn listar(dir: &Path) -> io::Result<BTreeSet<String>> {
    fn bajar(base: &Path, dir: &Path, salida: &mut BTreeSet<String>) -> io::Result<()> {
        for e in fs::read_dir(dir)? {
            let e = e?;
            let p = e.path();
            if e.file_type()?.is_dir() {
                bajar(base, &p, salida)?;
            } else if p.extension().is_none_or(|x| x != "tmp") {
                let rel = p.strip_prefix(base).map_err(io::Error::other)?;
                salida.insert(rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"));
            }
        }
        Ok(())
    }
    let mut salida = BTreeSet::new();
    if dir.is_dir() {
        bajar(dir, dir, &mut salida)?;
    }
    Ok(salida)
}

impl Espejo {
    /// Lee el índice del remoto y, si la carpeta no tiene partida y el remoto sí, la baja. Falla (sin tocar nada) si las dos partes tienen partidas distintas o si el remoto no
    /// contesta: arrancar sin él sería empezar una partida nueva y pisar la guardada.
    pub fn abrir(dir: impl AsRef<Path>, remoto: Box<dyn Remoto>) -> io::Result<(Espejo, Apertura)> {
        let dir = dir.as_ref().to_path_buf();
        let mut e = Espejo { dir, remoto, conocido: BTreeMap::new(), dudoso: false };
        e.leer_indice()?;
        let id = crate::FICHERO_ID_PARTIDA;
        let local = !listar(&e.dir)?.is_empty();
        let apertura = match (local, e.conocido.is_empty()) {
            (false, true) => Apertura::Vacio,
            (false, false) => Apertura::Bajada { ficheros: e.bajar()? },
            (true, true) => Apertura::Local,
            (true, false) => {
                let aqui = fs::read(e.dir.join(id)).ok();
                let alli = e.conocido.contains_key(id).then(|| e.remoto.leer(id, 0, 1)).transpose()?.and_then(|t| t.into_iter().next());
                if aqui.is_none() || aqui != alli {
                    return Err(io::Error::other(format!(
                        "la carpeta {} y la copia remota tienen partidas distintas; vacía una de las dos (la carpeta, para seguir la partida remota; la tabla `espejo`, para subir la local)",
                        e.dir.display()
                    )));
                }
                Apertura::Local
            }
        };
        Ok((e, apertura))
    }

    fn leer_indice(&mut self) -> io::Result<()> {
        let mut conocido: BTreeMap<String, Fichero> = BTreeMap::new();
        for (ruta, trozo, h) in self.remoto.indice()? {
            let f = conocido.entry(ruta.clone()).or_default();
            if trozo as usize != f.huellas.len() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("la copia remota de {ruta} tiene un hueco en el trozo {}", f.huellas.len())));
            }
            f.huellas.push(h);
        }
        self.conocido = conocido;
        self.dudoso = false;
        Ok(())
    }

    /// Escribe en la carpeta todo lo que hay en el remoto, comprobando cada trozo contra su huella.
    fn bajar(&mut self) -> io::Result<usize> {
        for (ruta, f) in &self.conocido {
            let destino = ruta.split('/').fold(self.dir.clone(), |p, c| p.join(c));
            if let Some(padre) = destino.parent() {
                fs::create_dir_all(padre)?;
            }
            let mut bytes = Vec::with_capacity(f.huellas.len() * TROZO);
            for desde in (0..f.huellas.len()).step_by(LOTE) {
                let cuantos = LOTE.min(f.huellas.len() - desde);
                let trozos = self.remoto.leer(ruta, desde as u32, cuantos as u32)?;
                if trozos.len() != cuantos || trozos.iter().zip(&f.huellas[desde..]).any(|(t, h)| huella(t) != *h) {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("la copia remota de {ruta} no coincide con sus huellas (trozos {desde}..{})", desde + cuantos)));
                }
                trozos.iter().for_each(|t| bytes.extend_from_slice(t));
            }
            fs::write(&destino, &bytes)?;
        }
        Ok(self.conocido.len())
    }

    /// Una vuelta: sube lo que cambió desde la anterior y borra del remoto lo que ya no está. Si falla, lo ya subido queda subido y la siguiente vuelta sigue desde ahí.
    pub fn sincronizar(&mut self) -> io::Result<Vuelta> {
        if self.dudoso {
            self.leer_indice()?;
        }
        let r = self.vuelta();
        self.dudoso = r.is_err();
        r
    }

    fn vuelta(&mut self) -> io::Result<Vuelta> {
        let mut v = Vuelta::default();
        // Las fotos se anotan antes de subir el registro y se suben después (ver la cabecera).
        let lista = listar(&self.dir)?;
        let (fotos, resto): (Vec<&String>, Vec<&String>) = lista.iter().partition(|r| es_foto(r));
        for ruta in resto.into_iter().chain(fotos) {
            self.subir(ruta, &mut v)?;
        }
        let sobran: Vec<String> = self.conocido.keys().filter(|r| !lista.contains(*r)).cloned().collect();
        for tanda in sobran.chunks(64) {
            self.remoto.aplicar(&tanda.iter().map(|r| Cambio::Cortar { ruta: r.clone(), desde: 0 }).collect::<Vec<_>>())?;
            tanda.iter().for_each(|r| {
                self.conocido.remove(r);
            });
            v.borrados += tanda.len();
        }
        Ok(v)
    }

    fn subir(&mut self, ruta: &str, v: &mut Vuelta) -> io::Result<()> {
        let camino = ruta.split('/').fold(self.dir.clone(), |p, c| p.join(c));
        // Un fichero que desaparece entre el listado y la lectura (una foto podada, un segmento compactado) se queda para la vuelta siguiente, que lo verá borrado.
        let Ok(mut f) = fs::File::open(&camino) else { return Ok(()) };
        let meta = f.metadata()?;
        let ahora = (meta.len(), meta.modified()?);
        let previo = self.conocido.get(ruta).cloned().unwrap_or_default();
        if previo.visto == Some(ahora) {
            return Ok(());
        }
        // Un `.log` (registro, crónica) solo crece mientras el proceso vive: si ya se miró y no ha encogido, basta releer desde su último trozo.
        let desde = match previo.visto {
            Some((tamano, _)) if ruta.ends_with(".log") && ahora.0 >= tamano && !previo.huellas.is_empty() => previo.huellas.len() - 1,
            _ => 0,
        };
        f.seek(SeekFrom::Start((desde * TROZO) as u64))?;
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)?;
        let mut huellas = previo.huellas[..desde].to_vec();
        let mut cambios = Vec::new();
        let mut peso = 0;
        for (i, datos) in bytes.chunks(TROZO).enumerate() {
            let (n, h) = (desde + i, huella(datos));
            if previo.huellas.get(n) != Some(&h) {
                peso += datos.len();
                cambios.push(Cambio::Poner { ruta: ruta.to_owned(), trozo: n as u32, huella: h, datos: datos.to_vec() });
            }
            huellas.push(h);
        }
        if huellas.len() < previo.huellas.len() {
            cambios.push(Cambio::Cortar { ruta: ruta.to_owned(), desde: huellas.len() as u32 });
        }
        // Por tandas, y lo conocido se pone al día tras cada una: si una falla, la vuelta se marca dudosa y se relee el índice.
        for tanda in cambios.chunks(LOTE) {
            self.remoto.aplicar(tanda)?;
        }
        v.trozos += cambios.iter().filter(|c| matches!(c, Cambio::Poner { .. })).count();
        v.bytes += peso;
        // Si el fichero creció mientras se leía, la marca vieja hace que la vuelta siguiente lo vuelva a mirar.
        self.conocido.insert(ruta.to_owned(), Fichero { huellas, visto: Some(ahora) });
        Ok(())
    }
}

/// El espejo andando en su hilo: una vuelta cada `cada`, y la última al parar.
pub struct EspejoVivo {
    parar: mpsc::Sender<()>,
    hilo: JoinHandle<io::Result<Vuelta>>,
}

impl EspejoVivo {
    pub fn lanzar(mut espejo: Espejo, cada: Duration) -> EspejoVivo {
        let (parar, rx) = mpsc::channel();
        let hilo = std::thread::Builder::new()
            .name("espejo".into())
            .spawn(move || {
                let mut fallando = false;
                loop {
                    let fin = !matches!(rx.recv_timeout(cada), Err(mpsc::RecvTimeoutError::Timeout));
                    let r = espejo.sincronizar();
                    match &r {
                        Err(e) if !fallando => eprintln!("espejo: no se pudo copiar al remoto ({e}); se reintenta cada {} s", cada.as_secs()),
                        Ok(_) if fallando => println!("espejo: la copia remota vuelve a estar al día"),
                        _ => {}
                    }
                    fallando = r.is_err();
                    if fin {
                        // La última copia es la que importa: si falla, un par de intentos más antes de rendirse.
                        let mut r = r;
                        for _ in 0..3 {
                            if r.is_ok() {
                                break;
                            }
                            std::thread::sleep(Duration::from_secs(2));
                            r = espejo.sincronizar();
                        }
                        return r;
                    }
                }
            })
            .expect("se puede lanzar el hilo del espejo");
        EspejoVivo { parar, hilo }
    }

    /// Hace la última copia (con el servidor ya parado) y devuelve su resultado.
    pub fn parar(self) -> io::Result<Vuelta> {
        let _ = self.parar.send(());
        self.hilo.join().map_err(|_| io::Error::other("el hilo del espejo murió"))?
    }
}

/// Un remoto en memoria, para las pruebas. Los clones comparten los datos, y `fallar` hace que las siguientes escrituras fallen.
#[derive(Clone, Default)]
pub struct EnMemoria {
    pub trozos: Arc<Mutex<BTreeMap<(String, u32), (Huella, Vec<u8>)>>>,
    pub fallar: Arc<Mutex<bool>>,
}

impl Remoto for EnMemoria {
    fn indice(&mut self) -> io::Result<Vec<(String, u32, Huella)>> {
        Ok(self.trozos.lock().unwrap().iter().map(|((r, t), (h, _))| (r.clone(), *t, *h)).collect())
    }

    fn leer(&mut self, ruta: &str, desde: u32, cuantos: u32) -> io::Result<Vec<Vec<u8>>> {
        Ok(self.trozos.lock().unwrap().range((ruta.to_owned(), desde)..(ruta.to_owned(), desde + cuantos)).map(|(_, (_, d))| d.clone()).collect())
    }

    fn aplicar(&mut self, cambios: &[Cambio]) -> io::Result<()> {
        if *self.fallar.lock().unwrap() {
            return Err(io::Error::other("remoto caído"));
        }
        let mut t = self.trozos.lock().unwrap();
        for c in cambios {
            match c {
                Cambio::Poner { ruta, trozo, huella, datos } => {
                    t.insert((ruta.clone(), *trozo), (*huella, datos.clone()));
                }
                Cambio::Cortar { ruta, desde } => t.retain(|(r, n), _| r != ruta || n < desde),
            }
        }
        Ok(())
    }
}

/// libSQL por HTTP (Turso): el protocolo Hrana, `POST <base>/v2/pipeline`. La tabla es `espejo (ruta, trozo, huella, datos)`.
pub struct Turso {
    agente: ureq::Agent,
    url: String,
    token: Option<String>,
}

const B64: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::GeneralPurposeConfig::new().with_encode_padding(false).with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

fn texto(v: &str) -> Value {
    json!({ "type": "text", "value": v })
}

fn entero(v: u32) -> Value {
    json!({ "type": "integer", "value": v.to_string() })
}

impl Turso {
    /// `url`: la de la base (`libsql://<base>.turso.io`, o `https://` o `http://`). Crea la tabla si falta.
    pub fn abrir(url: &str, token: Option<String>) -> io::Result<Turso> {
        let url = url.trim().trim_end_matches('/');
        let base = match url.split_once("://") {
            Some(("libsql" | "https" | "wss", resto)) => format!("https://{resto}"),
            Some(("http" | "ws", resto)) => format!("http://{resto}"),
            _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "la URL de la base no es libsql://, https:// ni http://")),
        };
        let agente = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(10)).timeout(Duration::from_secs(60)).build();
        let mut t = Turso { agente, url: format!("{base}/v2/pipeline"), token };
        t.ejecutar(vec![json!({
            "sql": "CREATE TABLE IF NOT EXISTS espejo (ruta TEXT NOT NULL, trozo INTEGER NOT NULL, huella TEXT NOT NULL, datos BLOB NOT NULL, PRIMARY KEY (ruta, trozo))"
        })])?;
        Ok(t)
    }

    /// Ejecuta las sentencias en una sola petición; con más de una, dentro de una transacción. Devuelve las filas de cada una.
    fn ejecutar(&mut self, sentencias: Vec<Value>) -> io::Result<Vec<Vec<Vec<Value>>>> {
        let varias = sentencias.len() > 1;
        let mut peticiones = Vec::new();
        if varias {
            peticiones.push(json!({ "type": "execute", "stmt": { "sql": "BEGIN" } }));
        }
        peticiones.extend(sentencias.into_iter().map(|s| json!({ "type": "execute", "stmt": s })));
        if varias {
            peticiones.push(json!({ "type": "execute", "stmt": { "sql": "COMMIT" } }));
        }
        peticiones.push(json!({ "type": "close" }));
        let mut p = self.agente.post(&self.url).set("Content-Type", "application/json");
        if let Some(t) = &self.token {
            p = p.set("Authorization", &format!("Bearer {t}"));
        }
        let cuerpo = json!({ "requests": peticiones }).to_string();
        let r = match p.send_string(&cuerpo) {
            Ok(r) => r,
            Err(ureq::Error::Status(codigo, r)) => {
                let detalle: String = r.into_string().unwrap_or_default().chars().take(300).collect();
                return Err(io::Error::other(format!("la base contestó {codigo}: {detalle}")));
            }
            Err(e) => return Err(io::Error::other(format!("no se llega a la base: {e}"))),
        };
        let mut bytes = Vec::new();
        r.into_reader().read_to_end(&mut bytes)?;
        let v: Value = serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("respuesta de la base: {e}")))?;
        let mal = |q: &str| io::Error::new(io::ErrorKind::InvalidData, format!("respuesta de la base: {q}"));
        let resultados = v["results"].as_array().ok_or_else(|| mal("sin resultados"))?;
        let mut filas = Vec::new();
        for r in resultados {
            match r["type"].as_str() {
                Some("ok") => {
                    if r["response"]["type"] == "execute" {
                        let f = r["response"]["result"]["rows"].as_array().map(|f| f.iter().map(|c| c.as_array().cloned().unwrap_or_default()).collect()).unwrap_or_default();
                        filas.push(f);
                    }
                }
                _ => return Err(io::Error::other(format!("la base rechazó una sentencia: {}", r["error"]["message"].as_str().unwrap_or("sin detalle")))),
            }
        }
        if varias {
            // Fuera el BEGIN y el COMMIT.
            if filas.len() < 2 {
                return Err(mal("faltan resultados de la transacción"));
            }
            filas.pop();
            filas.remove(0);
        }
        Ok(filas)
    }
}

impl Remoto for Turso {
    fn indice(&mut self) -> io::Result<Vec<(String, u32, Huella)>> {
        let mal = || io::Error::new(io::ErrorKind::InvalidData, "índice de la copia remota ilegible");
        let filas = self.ejecutar(vec![json!({ "sql": "SELECT ruta, trozo, huella FROM espejo ORDER BY ruta, trozo" })])?;
        filas
            .into_iter()
            .flatten()
            .map(|f| {
                let ruta = f.first().and_then(|c| c["value"].as_str()).ok_or_else(mal)?;
                let trozo = f.get(1).and_then(|c| c["value"].as_str()).and_then(|s| s.parse().ok()).ok_or_else(mal)?;
                let h = f.get(2).and_then(|c| c["value"].as_str()).and_then(de_hex).ok_or_else(mal)?;
                Ok((ruta.to_owned(), trozo, h))
            })
            .collect()
    }

    fn leer(&mut self, ruta: &str, desde: u32, cuantos: u32) -> io::Result<Vec<Vec<u8>>> {
        let filas = self.ejecutar(vec![json!({
            "sql": "SELECT datos FROM espejo WHERE ruta = ? AND trozo >= ? AND trozo < ? ORDER BY trozo",
            "args": [texto(ruta), entero(desde), entero(desde + cuantos)],
        })])?;
        filas
            .into_iter()
            .flatten()
            .map(|f| {
                f.first()
                    .and_then(|c| c["base64"].as_str())
                    .and_then(|b| B64.decode(b).ok())
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "trozo de la copia remota ilegible"))
            })
            .collect()
    }

    fn aplicar(&mut self, cambios: &[Cambio]) -> io::Result<()> {
        if cambios.is_empty() {
            return Ok(());
        }
        let sentencias = cambios
            .iter()
            .map(|c| match c {
                Cambio::Poner { ruta, trozo, huella, datos } => json!({
                    "sql": "INSERT OR REPLACE INTO espejo (ruta, trozo, huella, datos) VALUES (?, ?, ?, ?)",
                    "args": [texto(ruta), entero(*trozo), texto(&hex(huella)), { "type": "blob", "base64": B64.encode(datos) }],
                }),
                Cambio::Cortar { ruta, desde } => json!({ "sql": "DELETE FROM espejo WHERE ruta = ? AND trozo >= ?", "args": [texto(ruta), entero(*desde)] }),
            })
            .collect();
        self.ejecutar(sentencias).map(|_| ())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn escribir(dir: &Path, ruta: &str, datos: &[u8]) {
        let p = dir.join(ruta);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, datos).unwrap();
    }

    fn ruido(n: usize, semilla: u8) -> Vec<u8> {
        (0..n).map(|i| (i as u32).wrapping_mul(2654435761).to_le_bytes()[1] ^ semilla).collect()
    }

    /// Lo que hay en la carpeta, fichero a fichero.
    fn contenido(dir: &Path) -> BTreeMap<String, Vec<u8>> {
        listar(dir).unwrap().into_iter().map(|r| (r.clone(), fs::read(dir.join(&r)).unwrap())).collect()
    }

    #[test]
    fn una_carpeta_vacia_recibe_la_partida_del_remoto_tal_cual() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let remoto = EnMemoria::default();
        escribir(a.path(), "partida.id", b"p-1");
        escribir(a.path(), "mundo/registro-00000000000000000001.log", &ruido(3 * TROZO + 17, 1));
        escribir(a.path(), "mundo/foto-00000000000000000009.zst", &ruido(40 * TROZO, 2));
        escribir(a.path(), "acceso/cuentas.jsonl", b"{}\n");
        escribir(a.path(), "mundo/foto-00000000000000000010.tmp", b"a medias");
        let (mut e, apertura) = Espejo::abrir(a.path(), Box::new(remoto.clone())).unwrap();
        assert_eq!(apertura, Apertura::Local);
        let v = e.sincronizar().unwrap();
        assert_eq!((v.trozos, v.borrados), (1 + 4 + 40 + 1, 0));
        assert_eq!(e.sincronizar().unwrap(), Vuelta::default(), "sin cambios, la segunda vuelta no sube nada");

        let (_, apertura) = Espejo::abrir(b.path(), Box::new(remoto)).unwrap();
        assert_eq!(apertura, Apertura::Bajada { ficheros: 4 });
        let mut esperado = contenido(a.path());
        esperado.remove("mundo/foto-00000000000000000010.tmp");
        assert_eq!(contenido(b.path()), esperado);
    }

    #[test]
    fn un_registro_que_crece_solo_sube_su_cola_y_lo_borrado_se_borra() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let remoto = EnMemoria::default();
        let registro = "mundo/registro-00000000000000000001.log";
        let mut datos = ruido(5 * TROZO + 100, 3);
        escribir(a.path(), "partida.id", b"p-1");
        escribir(a.path(), registro, &datos);
        escribir(a.path(), "mundo/foto-00000000000000000003.zst", &ruido(1000, 4));
        let (mut e, _) = Espejo::abrir(a.path(), Box::new(remoto.clone())).unwrap();
        e.sincronizar().unwrap();

        datos.extend(ruido(TROZO, 5));
        escribir(a.path(), registro, &datos);
        fs::remove_file(a.path().join("mundo/foto-00000000000000000003.zst")).unwrap();
        escribir(a.path(), "mundo/foto-00000000000000000007.zst", &ruido(2000, 6));
        let v = e.sincronizar().unwrap();
        assert_eq!((v.trozos, v.borrados), (2 + 1, 1), "el último trozo del registro, el nuevo y la foto nueva; fuera la foto vieja");

        // Un fichero que se reescribe más corto (las cuentas, al compactar) pierde los trozos que sobran.
        escribir(a.path(), registro, &datos[..TROZO + 5]);
        e.sincronizar().unwrap();
        Espejo::abrir(b.path(), Box::new(remoto)).unwrap();
        assert_eq!(contenido(b.path()), contenido(a.path()));
    }

    #[test]
    fn dos_partidas_distintas_no_se_pisan() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let remoto = EnMemoria::default();
        escribir(a.path(), "partida.id", b"p-1");
        Espejo::abrir(a.path(), Box::new(remoto.clone())).unwrap().0.sincronizar().unwrap();
        escribir(b.path(), "partida.id", b"p-2");
        assert!(Espejo::abrir(b.path(), Box::new(remoto.clone())).is_err());
        assert_eq!(remoto.trozos.lock().unwrap().len(), 1, "el remoto sigue como estaba");
        // La misma partida sí abre, y manda el disco.
        assert_eq!(Espejo::abrir(a.path(), Box::new(remoto)).unwrap().1, Apertura::Local);
    }

    #[test]
    fn tras_un_fallo_del_remoto_la_vuelta_siguiente_lo_pone_al_dia() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let remoto = EnMemoria::default();
        escribir(a.path(), "partida.id", b"p-1");
        escribir(a.path(), "mundo/registro-00000000000000000001.log", &ruido(40 * TROZO, 7));
        let (mut e, _) = Espejo::abrir(a.path(), Box::new(remoto.clone())).unwrap();
        *remoto.fallar.lock().unwrap() = true;
        assert!(e.sincronizar().is_err());
        *remoto.fallar.lock().unwrap() = false;
        e.sincronizar().unwrap();
        Espejo::abrir(b.path(), Box::new(remoto)).unwrap();
        assert_eq!(contenido(b.path()), contenido(a.path()));
    }

    #[test]
    fn el_espejo_vivo_copia_solo_y_hace_la_ultima_copia_al_parar() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let remoto = EnMemoria::default();
        escribir(a.path(), "partida.id", b"p-1");
        let (e, _) = Espejo::abrir(a.path(), Box::new(remoto.clone())).unwrap();
        let vivo = EspejoVivo::lanzar(e, Duration::from_secs(3600));
        escribir(a.path(), "acceso/cuentas.jsonl", b"{\"nick\":\"ana\"}\n");
        vivo.parar().unwrap();
        Espejo::abrir(b.path(), Box::new(remoto)).unwrap();
        assert_eq!(contenido(b.path()), contenido(a.path()));
    }
}
