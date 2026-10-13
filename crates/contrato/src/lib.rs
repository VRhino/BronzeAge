//! Contrato 05 (`bronzeage.v1`): tipos generados desde `proto/`.
pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/bronzeage.v1.rs"));
}

#[cfg(test)]
mod tests {
    use super::v1::*;
    use prost::Message;

    #[test]
    fn mensaje_de_sesion_ida_y_vuelta() {
        let m = MensajeDeCliente {
            cuerpo: Some(mensaje_de_cliente::Cuerpo::Ping(Ping { cliente_envio_ms: 42 })),
        };
        let bytes = m.encode_to_vec();
        assert_eq!(MensajeDeCliente::decode(bytes.as_slice()).unwrap(), m);
    }
}
