//! La medida de carga del actor (2026-11-05, visibilidad de rendimiento, pedido del autor), en dos niveles:
//! - **Lo básico**, encendido salvo `BRONZEAGE_CARGA=0` al arrancar: cuánto tiempo de pared estuvo trabajando el actor y cuánto de eso fue poner al día las `VISTA`, las órdenes que atendió
//!   con lo que tardó cada una (de sacarla de la fila a repartir su resultado: media, p50, p95, p99 y la más lenta), cuántos mensajes llegó a tener esperando en la fila, las sesiones, las que
//!   tienen `VISTA` y lo que movieron por la red (de los contadores de `servidor::MetricasRed`). Son un par de lecturas del reloj por mensaje.
//! - **El detalle**, apagado salvo que la administración lo encienda en caliente (`POST /v1/admin/carga`): el tiempo de cada fase del reparto, las órdenes por tipo (las diez más lentas), las
//!   sesiones con el canal a medio llenar y, con la carpeta de la partida, una línea JSON por ventana en `carga.jsonl`.
//!
//! Cada [`CADA_MEDIDA`] de pared se cierra una ventana: escribe su línea `carga:` en el registro, la guarda como la última (la que devuelve `GET /v1/admin/carga`) y vuelve a cero.

use serde_json::{json, Value};
use servidor::MetricasRed;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant as Pared, SystemTime, UNIX_EPOCH};

/// Cada cuánto (de pared) se cierra una ventana de la medida.
pub const CADA_MEDIDA: Duration = Duration::from_secs(10);

/// El historial de la medida con el detalle encendido, una línea JSON por ventana, junto a la carpeta `mundo/` de la partida.
pub const HISTORIAL_DE_CARGA: &str = "carga.jsonl";

/// Cuántos tipos de orden lleva el detalle de cada ventana (los más lentos por su tiempo total).
const TIPOS_EN_DETALLE: usize = 10;

/// Los contadores de red en un instante, para sacar la diferencia de una ventana.
#[derive(Clone, Copy, Default)]
struct Red {
    mensajes: u64,
    crudos: u64,
    enviados: u64,
    lentas: u64,
    tramas: u64,
    recibidos: u64,
}

impl Red {
    fn de(m: &MetricasRed) -> Self {
        let l = |a: &std::sync::atomic::AtomicU64| a.load(Ordering::Relaxed);
        Red { mensajes: l(&m.mensajes), crudos: l(&m.bytes_crudos), enviados: l(&m.bytes_enviados), lentas: l(&m.escrituras_lentas), tramas: l(&m.tramas_recibidas), recibidos: l(&m.bytes_recibidos) }
    }
}

/// Lo que se acumula en una ventana.
#[derive(Default)]
struct Ventana {
    ocupado: Duration,
    vistas: Duration,
    /// Lo que tardó cada orden, en microsegundos.
    ordenes: Vec<u32>,
    fila_maxima: usize,
    /// Sesiones que se descartaron porque su canal estaba lleno (cliente lento).
    descartadas: u32,
    // Con el detalle:
    fases: BTreeMap<&'static str, Duration>,
    /// Por tipo de orden: cuántas, tiempo total y la más lenta.
    por_tipo: BTreeMap<String, (u32, Duration, Duration)>,
    /// El máximo de sesiones con el canal a medio llenar o más en un reparto.
    canales_medio_llenos: usize,
}

/// La medida de carga del actor.
pub(super) struct Medidor {
    pub activo: bool,
    pub detalle: bool,
    desde: Pared,
    v: Ventana,
    red: Option<Arc<MetricasRed>>,
    red_antes: Red,
    /// Dónde va el historial con el detalle encendido (`<carpeta de la partida>/carga.jsonl`).
    historial: Option<PathBuf>,
    /// La última ventana cerrada, en JSON.
    ultima: Option<Value>,
}

impl Medidor {
    pub fn nuevo(red: Option<Arc<MetricasRed>>, historial: Option<PathBuf>) -> Self {
        let activo = std::env::var("BRONZEAGE_CARGA").map_or(true, |v| v.trim() != "0");
        let red_antes = red.as_deref().map(Red::de).unwrap_or_default();
        Self { activo, detalle: false, desde: Pared::now(), v: Ventana::default(), red, red_antes, historial, ultima: None }
    }

    /// El instante de empezar a medir algo (`None` con la medida apagada: no se lee el reloj).
    pub fn inicio(&self) -> Option<Pared> {
        self.activo.then(Pared::now)
    }

    /// Como [`Medidor::inicio`], solo con el detalle encendido.
    pub fn inicio_detalle(&self) -> Option<Pared> {
        (self.activo && self.detalle).then(Pared::now)
    }

    pub fn ocupado(&mut self, inicio: Option<Pared>) {
        if let Some(i) = inicio {
            self.v.ocupado += i.elapsed();
        }
    }

    pub fn vistas(&mut self, inicio: Option<Pared>) {
        if let Some(i) = inicio {
            self.v.vistas += i.elapsed();
        }
    }

    pub fn fila(&mut self, esperando: usize) {
        if self.activo {
            self.v.fila_maxima = self.v.fila_maxima.max(esperando);
        }
    }

    /// Una orden atendida, con su tipo si el detalle está encendido.
    pub fn orden(&mut self, inicio: Option<Pared>, tipo: Option<String>) {
        let Some(i) = inicio else { return };
        let tardo = i.elapsed();
        self.v.ocupado += tardo;
        self.v.ordenes.push(tardo.as_micros().min(u128::from(u32::MAX)) as u32);
        if let Some(tipo) = tipo {
            let e = self.v.por_tipo.entry(tipo).or_default();
            e.0 += 1;
            e.1 += tardo;
            e.2 = e.2.max(tardo);
        }
    }

    /// El tiempo de una fase del reparto (solo con el detalle: `inicio` viene de [`Medidor::inicio_detalle`]).
    pub fn fase(&mut self, nombre: &'static str, inicio: Option<Pared>) {
        if let Some(i) = inicio {
            *self.v.fases.entry(nombre).or_default() += i.elapsed();
        }
    }

    pub fn descartada(&mut self) {
        if self.activo {
            self.v.descartadas += 1;
        }
    }

    pub fn canales_medio_llenos(&mut self, cuantos: usize) {
        self.v.canales_medio_llenos = self.v.canales_medio_llenos.max(cuantos);
    }

    /// Enciende o apaga en caliente. Al cambiar empieza una ventana nueva.
    pub fn fijar(&mut self, activo: Option<bool>, detalle: Option<bool>) {
        let (a, d) = (activo.unwrap_or(self.activo), detalle.unwrap_or(self.detalle));
        if (a, d) != (self.activo, self.detalle) {
            (self.activo, self.detalle) = (a, d);
            self.reiniciar();
        }
    }

    /// Lo que devuelve `GET /v1/admin/carga`.
    pub fn estado(&self) -> Value {
        json!({ "activo": self.activo, "detalle": self.detalle, "cada_s": CADA_MEDIDA.as_secs(), "ultima": self.ultima })
    }

    fn reiniciar(&mut self) {
        self.desde = Pared::now();
        self.v = Ventana::default();
        self.red_antes = self.red.as_deref().map(Red::de).unwrap_or_default();
    }

    /// Cierra la ventana si toca: devuelve su línea del registro (ninguna sin sesiones ni órdenes) y, con el detalle, la apunta en el historial.
    pub fn cerrar_si_toca(&mut self, sesiones: usize, vistas: usize) -> Option<String> {
        if !self.activo {
            return None;
        }
        let pared = self.desde.elapsed();
        if pared < CADA_MEDIDA {
            return None;
        }
        let red = self.red.as_deref().map(Red::de).unwrap_or_default();
        let (antes, mut v) = (self.red_antes, std::mem::take(&mut self.v));
        self.desde = Pared::now();
        self.red_antes = red;
        if sesiones == 0 && v.ordenes.is_empty() {
            return None;
        }
        let s = pared.as_secs_f64();
        let pct = |d: Duration| redondear(100.0 * d.as_secs_f64() / s, 1);
        let ms = |d: Duration| redondear(d.as_secs_f64() * 1e3, 2);
        v.ordenes.sort_unstable();
        let percentil = |p: f64| v.ordenes.get(((v.ordenes.len() as f64 * p).ceil() as usize).saturating_sub(1)).map_or(0.0, |&us| redondear(f64::from(us) / 1e3, 2));
        let media = if v.ordenes.is_empty() { 0.0 } else { redondear(v.ordenes.iter().map(|&x| f64::from(x)).sum::<f64>() / v.ordenes.len() as f64 / 1e3, 2) };
        let kb_s = |b: u64| redondear(b as f64 / 1024.0 / s, 1);
        let (crudos, enviados) = (red.crudos - antes.crudos, red.enviados - antes.enviados);
        let hasta_ms = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
        let mut ventana = json!({
            "hasta_ms": hasta_ms,
            "segundos": redondear(s, 1),
            "actor_ocupado_pct": pct(v.ocupado),
            "vistas_pct": pct(v.vistas),
            "ordenes": v.ordenes.len(),
            "ordenes_por_s": redondear(v.ordenes.len() as f64 / s, 1),
            "orden_ms": { "media": media, "p50": percentil(0.50), "p95": percentil(0.95), "p99": percentil(0.99), "max": percentil(1.0) },
            "fila_maxima": v.fila_maxima,
            "sesiones": sesiones,
            "vistas": vistas,
            "descartadas_por_canal_lleno": v.descartadas,
            "red": {
                "mensajes_por_s": redondear((red.mensajes - antes.mensajes) as f64 / s, 1),
                "kb_por_s_crudos": kb_s(crudos),
                "kb_por_s_enviados": kb_s(enviados),
                "compresion": if crudos > 0 { redondear(enviados as f64 / crudos as f64, 3) } else { 1.0 },
                "escrituras_lentas": red.lentas - antes.lentas,
                "tramas_recibidas_por_s": redondear((red.tramas - antes.tramas) as f64 / s, 1),
                "kb_por_s_recibidos": kb_s(red.recibidos - antes.recibidos),
            },
        });
        if self.detalle {
            let mut tipos: Vec<(String, (u32, Duration, Duration))> = v.por_tipo.into_iter().collect();
            tipos.sort_by_key(|(_, (_, total, _))| std::cmp::Reverse(*total));
            let tipos: Vec<Value> = tipos
                .into_iter()
                .take(TIPOS_EN_DETALLE)
                .map(|(tipo, (n, total, max))| json!({ "tipo": tipo, "n": n, "media_ms": ms(total / n.max(1)), "max_ms": ms(max), "total_ms": ms(total) }))
                .collect();
            let fases: serde_json::Map<String, Value> = v.fases.into_iter().map(|(k, d)| (k.to_owned(), json!(ms(d)))).collect();
            ventana["detalle"] = json!({ "fases_ms": fases, "ordenes_por_tipo": tipos, "canales_medio_llenos": v.canales_medio_llenos });
            if let Some(h) = &self.historial {
                let escrito = std::fs::OpenOptions::new().create(true).append(true).open(h).and_then(|mut f| writeln!(f, "{ventana}"));
                if let Err(e) = escrito {
                    eprintln!("carga: no se pudo escribir {}: {e}", h.display());
                }
            }
        }
        let linea = format!(
            "carga: actor ocupado {} % (vistas {} %); {} órdenes en {:.0} s, p50 {} ms, p95 {} ms, p99 {} ms, la más lenta {} ms; hasta {} esperando en la fila; {sesiones} sesiones, {vistas} con VISTA; red {} KB/s",
            ventana["actor_ocupado_pct"],
            ventana["vistas_pct"],
            ventana["ordenes"],
            s,
            ventana["orden_ms"]["p50"],
            ventana["orden_ms"]["p95"],
            ventana["orden_ms"]["p99"],
            ventana["orden_ms"]["max"],
            ventana["fila_maxima"],
            ventana["red"]["kb_por_s_enviados"],
        );
        self.ultima = Some(ventana);
        Some(linea)
    }
}

fn redondear(x: f64, decimales: i32) -> f64 {
    let f = 10f64.powi(decimales);
    (x * f).round() / f
}

/// El tipo de una orden del contrato para el detalle (`MarcharA`, `Construir`...): el nombre de la variante de su cuerpo.
pub(super) fn tipo_de_orden(cuerpo: &Option<contrato::v1::orden::Cuerpo>) -> String {
    let Some(c) = cuerpo else { return "vacia".into() };
    let d = format!("{c:?}");
    d.split(['(', ' ', '{']).next().unwrap_or("").to_owned()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_ventana_se_cierra_con_sus_percentiles_y_con_el_detalle_va_al_historial() {
        let dir = std::env::temp_dir().join(format!("carga-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let historial = dir.join(HISTORIAL_DE_CARGA);
        let _ = std::fs::remove_file(&historial);
        let red = Arc::new(MetricasRed::default());
        let mut m = Medidor::nuevo(Some(red.clone()), Some(historial.clone()));
        m.activo = true;
        m.fijar(None, Some(true));
        let antes = Pared::now() - Duration::from_millis(5);
        for _ in 0..3 {
            m.orden(Some(antes), Some("MarcharA".into()));
        }
        m.fase("vistas", Some(antes));
        red.bytes_crudos.fetch_add(2_000, Ordering::Relaxed);
        red.bytes_enviados.fetch_add(500, Ordering::Relaxed);
        // Sin pasar la ventana, nada.
        assert!(m.cerrar_si_toca(1, 1).is_none());
        m.desde -= CADA_MEDIDA;
        let linea = m.cerrar_si_toca(1, 1).expect("la ventana se cierra");
        assert!(linea.starts_with("carga: actor ocupado"), "{linea}");
        let u = &m.estado()["ultima"];
        assert_eq!(u["ordenes"], json!(3));
        assert!(u["orden_ms"]["p99"].as_f64().unwrap() >= 5.0, "{u}");
        assert_eq!(u["red"]["compresion"], json!(0.25));
        assert_eq!(u["detalle"]["ordenes_por_tipo"][0]["tipo"], json!("MarcharA"));
        assert!(u["detalle"]["fases_ms"]["vistas"].as_f64().unwrap() >= 5.0, "{u}");
        let escrito = std::fs::read_to_string(&historial).unwrap();
        assert_eq!(escrito.lines().count(), 1);
        // Apagada, ni mide ni cierra.
        m.fijar(Some(false), None);
        assert!(m.inicio().is_none());
        m.desde -= CADA_MEDIDA;
        assert!(m.cerrar_si_toca(1, 1).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn el_tipo_de_una_orden_es_el_nombre_de_su_cuerpo() {
        use contrato::v1 as pb;
        let c = Some(pb::orden::Cuerpo::MarcharA(pb::OrdenMarcharA::default()));
        assert_eq!(tipo_de_orden(&c), "MarcharA");
        assert_eq!(tipo_de_orden(&None), "vacia");
    }
}
