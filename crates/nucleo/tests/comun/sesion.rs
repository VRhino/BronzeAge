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

    /// Conecta con el `Hola` dado y recoge lo que el servidor manda tras la `Bienvenida`: un mensaje (`Foto` o `Delta`) por cada ámbito que no reanuda sin novedad.
    pub async fn conectar(&self, hola: pb::Hola) -> (ClientePrueba, pb::Bienvenida, Vec<ms::Cuerpo>) {
        let (mut c, b) = ClientePrueba::entrar(self.srv.direccion, hola).await;
        let n = b.ambitos.iter().filter(|a| a.decision != pb::DecisionReanudacion::ReanudaSinNovedad as i32).count();
        let mut mensajes = Vec::new();
        for _ in 0..n {
            mensajes.push(c.esperar().await.cuerpo.expect("un mensaje con cuerpo"));
        }
        (c, b, mensajes)
    }

    /// Conecta a `cliente` (con el tipo dado) y recoge la `Foto` de su primer ámbito (la vacía si no tiene ninguno); las de los demás ámbitos se leen y se descartan.
    pub async fn entrar(&self, cliente: &str, tipo: TipoCliente) -> (ClientePrueba, pb::Foto) {
        let (c, _, mensajes) = self.conectar(hola(&format!("ok:{cliente}"), tipo)).await;
        let f = mensajes.into_iter().find_map(|m| if let ms::Cuerpo::Foto(f) = m { Some(f) } else { None }).unwrap_or_default();
        (c, f)
    }
}

pub fn orden_de(peticion: &str, cuerpo: orden::Cuerpo) -> mc::Cuerpo {
    mc::Cuerpo::Orden(Orden { peticion_id: peticion.into(), cuerpo: Some(cuerpo), ..Default::default() })
}

/// Da una orden y espera su resultado. Devuelve lo que llegó antes (`Delta`, `Foto` y `Revocado`, en orden), que cumple la causalidad: nada pasa de la versión del resultado.
pub async fn dar_todo(c: &mut ClientePrueba, peticion: &str, cuerpo: orden::Cuerpo) -> (Vec<ms::Cuerpo>, pb::ResultadoOrden) {
    c.enviar(orden_de(peticion, cuerpo)).await;
    let mut previos = Vec::new();
    loop {
        match c.esperar().await.cuerpo {
            Some(m @ (ms::Cuerpo::Delta(_) | ms::Cuerpo::Foto(_) | ms::Cuerpo::Revocado(_))) => previos.push(m),
            Some(ms::Cuerpo::Resultado(r)) => {
                assert_eq!(r.peticion_id, peticion);
                let version = |m: &ms::Cuerpo| match m {
                    ms::Cuerpo::Delta(d) => d.hasta,
                    ms::Cuerpo::Foto(f) => f.version,
                    _ => 0,
                };
                assert!(previos.iter().all(|m| version(m) <= r.version_tras), "un mensaje de ámbito llegó después del resultado de la orden que lo causó");
                return (previos, r);
            }
            Some(ms::Cuerpo::Aviso(_)) => {}
            o => panic!("llegó {o:?}"),
        }
    }
}

/// Da una orden y espera su resultado. Devuelve las `Delta` que llegaron antes (las `Foto` y `Revocado`, si los hubo, se descartan).
pub async fn dar(c: &mut ClientePrueba, peticion: &str, cuerpo: orden::Cuerpo) -> (Vec<pb::Delta>, pb::ResultadoOrden) {
    let (previos, r) = dar_todo(c, peticion, cuerpo).await;
    (previos.into_iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None }).collect(), r)
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

/// Las columnas (`Ejercito`) de una lista de entidades: el resto de un ámbito (el héroe, sus escuadras...) no es de las pruebas de columnas.
pub fn columnas<'a>(entidades: impl IntoIterator<Item = &'a Entidad>) -> Vec<&'a pb::Ejercito> {
    entidades.into_iter().filter_map(|e| if let Some(pb::entidad::Cuerpo::Ejercito(x)) = e.cuerpo.as_ref() { Some(x) } else { None }).collect()
}

/// Las columnas que actualiza una `Delta`.
pub fn columnas_de(d: &pb::Delta) -> Vec<&pb::Ejercito> {
    columnas(d.cambios.iter().filter_map(|c| c.datos.as_ref()))
}

/// De las `Delta` que llegaron antes de un resultado, las del ámbito del héroe.
pub fn del_heroe(deltas: &[pb::Delta]) -> Vec<&pb::Delta> {
    deltas.iter().filter(|d| d.ambito.as_ref().is_some_and(|a| a.tipo == pb::AmbitoTipo::AmbitoHeroe as i32)).collect()
}
