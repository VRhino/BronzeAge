//! Lo común de las pruebas por la sesión: el servidor real (WebSocket) con el actor del núcleo, clientes de prueba y los mensajes que reciben.

use contrato::v1::{self as pb, mensaje_de_cliente as mc, mensaje_de_servidor as ms, orden, Entidad, Orden, TipoCliente};
use nucleo::{sesiones, Nucleo};
use servidor::{
    prueba::{hola, AutenticadorDePrueba, ClientePrueba},
    servir, ComandoSesion, Config, RelojCompartido, Servidor,
};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// El servidor y el actor del núcleo que lo atiende.
pub struct Servicio {
    pub srv: Servidor,
    pub tx: mpsc::Sender<ComandoSesion>,
    actor: JoinHandle<Nucleo>,
}

pub async fn arrancar_con(n: Nucleo) -> Servicio {
    let reloj = RelojCompartido::nuevo(n.ahora());
    let (tx, rx) = mpsc::channel(1024);
    let actor = sesiones::lanzar(n, reloj.clone(), rx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let srv = servir(listener, tx.clone(), AutenticadorDePrueba, reloj, Config::default()).await;
    Servicio { srv, tx, actor }
}

impl Servicio {
    pub async fn parar(self) -> Nucleo {
        self.srv.detener().await;
        drop(self.tx);
        self.actor.await.unwrap()
    }

    /// Conecta a `cliente` (con el tipo dado) y recoge su `Foto` inicial.
    pub async fn entrar(&self, cliente: &str, tipo: TipoCliente) -> (ClientePrueba, pb::Foto) {
        let (mut c, _) = ClientePrueba::entrar(self.srv.direccion, hola(&format!("ok:{cliente}"), tipo)).await;
        let f = foto(&mut c).await;
        (c, f)
    }
}

pub fn orden_de(peticion: &str, cuerpo: orden::Cuerpo) -> mc::Cuerpo {
    mc::Cuerpo::Orden(Orden { peticion_id: peticion.into(), cuerpo: Some(cuerpo), ..Default::default() })
}

/// Da una orden y espera su resultado. Devuelve las `Delta` que llegaron antes, que cumplen la causalidad: ninguna pasa de la versión del resultado.
pub async fn dar(c: &mut ClientePrueba, peticion: &str, cuerpo: orden::Cuerpo) -> (Vec<pb::Delta>, pb::ResultadoOrden) {
    c.enviar(orden_de(peticion, cuerpo)).await;
    let mut deltas = Vec::new();
    loop {
        match c.esperar().await.cuerpo {
            Some(ms::Cuerpo::Delta(d)) => deltas.push(d),
            Some(ms::Cuerpo::Resultado(r)) => {
                assert_eq!(r.peticion_id, peticion);
                assert!(deltas.iter().all(|d| d.hasta <= r.version_tras), "una Delta llegó después del resultado de la orden que la causó");
                return (deltas, r);
            }
            Some(ms::Cuerpo::Aviso(_)) => {}
            o => panic!("llegó {o:?}"),
        }
    }
}

/// El código de rechazo de un resultado (falla si se aceptó).
pub fn rechazo(r: &pb::ResultadoOrden) -> &str {
    assert_eq!(r.resultado, pb::Resultado::OrdenRechazada as i32, "se esperaba un rechazo: {r:?}");
    r.rechazo.as_ref().expect("un rechazo trae su código").codigo.as_str()
}

pub fn ejercito(e: &Entidad) -> &pb::Ejercito {
    match e.cuerpo.as_ref() {
        Some(pb::entidad::Cuerpo::Ejercito(x)) => x,
        _ => panic!("se esperaba un Ejercito"),
    }
}

pub async fn foto(c: &mut ClientePrueba) -> pb::Foto {
    match c.esperar().await.cuerpo {
        Some(ms::Cuerpo::Foto(f)) => f,
        o => panic!("se esperaba Foto, llegó {o:?}"),
    }
}

pub async fn delta(c: &mut ClientePrueba) -> pb::Delta {
    match c.esperar().await.cuerpo {
        Some(ms::Cuerpo::Delta(d)) => d,
        o => panic!("se esperaba Delta, llegó {o:?}"),
    }
}

pub async fn resultado(c: &mut ClientePrueba) -> pb::ResultadoOrden {
    match c.esperar().await.cuerpo {
        Some(ms::Cuerpo::Resultado(r)) => r,
        o => panic!("se esperaba Resultado, llegó {o:?}"),
    }
}

pub async fn aviso(c: &mut ClientePrueba) -> pb::Aviso {
    match c.esperar().await.cuerpo {
        Some(ms::Cuerpo::Aviso(a)) => a,
        o => panic!("se esperaba un Aviso, llegó {o:?}"),
    }
}

pub async fn silencio(c: &mut ClientePrueba) {
    assert!(tokio::time::timeout(Duration::from_millis(300), c.recibir()).await.is_err(), "no debía llegar nada");
}

/// Espera, leyendo sus mensajes, la `Delta` que trae el hecho `codigo`: lo que el reloj de mundo hace por su cuenta (un encuentro entre dos columnas, por ejemplo) llega cuando el mundo
/// llega a su instante, antes o después de la siguiente orden según el reloj, y se junta o no con ella en una sola `Delta`. Quien cuente las `Delta` de una orden tiene que esperar
/// antes lo que el reloj ya tiene que decirle.
pub async fn esperar_hecho(c: &mut ClientePrueba, codigo: &str) -> pb::Delta {
    loop {
        let d = delta(c).await;
        if d.eventos.iter().any(|e| e.codigo == codigo) {
            return d;
        }
    }
}
