//! `cargo run -p servidor --release --example rendimiento_servidor [sesiones] [ordenes_por_sesion]`
//! Mide mensajes/s del servidor de sesiones con un núcleo falso, todo en el mismo proceso (el cliente de prueba comparte CPU).

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let sesiones = args.next().and_then(|s| s.parse().ok()).unwrap_or(200);
    let ordenes = args.next().and_then(|s| s.parse().ok()).unwrap_or(500);
    let m = servidor::prueba::medir_rendimiento(sesiones, ordenes).await;
    println!(
        "{} sesiones, {} órdenes, {} mensajes en {:.2} s → {:.0} mensajes/s ({:.0} órdenes/s)",
        m.sesiones,
        m.ordenes,
        m.mensajes,
        m.segundos,
        m.mensajes_por_s(),
        m.ordenes as f64 / m.segundos
    );
}
