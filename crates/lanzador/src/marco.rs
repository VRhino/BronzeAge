//! Los mensajes entre el lanzador y un proceso de batalla, por su socket local: cada uno es un mensaje de Protobuf con su longitud delante en un varint (lo que escriben
//! `WriteDelimitedTo` y leen `ParseDelimitedFrom` en C#, y `encode_length_delimited` en Rust).

use std::io;

use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// El mayor mensaje que se admite: un ticket de asedio con su plaza cabe de sobra.
pub const MAXIMO: usize = 16 << 20;

/// El siguiente mensaje, o `None` si el otro lado cerró limpio (antes de empezar uno).
pub async fn leer<M: Message + Default, R: AsyncRead + Unpin>(r: &mut R) -> io::Result<Option<M>> {
    let (mut largo, mut desplazamiento) = (0u64, 0u32);
    loop {
        let mut b = [0u8; 1];
        if r.read(&mut b).await? == 0 {
            return if desplazamiento == 0 { Ok(None) } else { Err(io::ErrorKind::UnexpectedEof.into()) };
        }
        largo |= u64::from(b[0] & 0x7f) << desplazamiento;
        if b[0] & 0x80 == 0 {
            break;
        }
        desplazamiento += 7;
        if desplazamiento > 63 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "longitud mal formada"));
        }
    }
    let largo = usize::try_from(largo).ok().filter(|&l| l <= MAXIMO).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "mensaje demasiado grande"))?;
    let mut bytes = vec![0; largo];
    r.read_exact(&mut bytes).await?;
    M::decode(bytes.as_slice()).map(Some).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Escribe un mensaje con su longitud delante.
pub async fn escribir<M: Message, W: AsyncWrite + Unpin>(w: &mut W, m: &M) -> io::Result<()> {
    w.write_all(&m.encode_length_delimited_to_vec()).await?;
    w.flush().await
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use contrato::v1::{mensaje_para_proceso_de_batalla as al_proceso, BattleTicket, MensajeParaProcesoDeBatalla};

    #[tokio::test]
    async fn ida_y_vuelta_con_varios_seguidos_y_el_cierre_limpio_o_a_medias() {
        let (mut a, mut b) = tokio::io::duplex(64);
        let largo = MensajeParaProcesoDeBatalla { cuerpo: Some(al_proceso::Cuerpo::Ticket(BattleTicket { battle_id: "x".repeat(300), ..Default::default() })) };
        let corto = MensajeParaProcesoDeBatalla { cuerpo: Some(al_proceso::Cuerpo::Ticket(BattleTicket { ticket_revision: 2, ..Default::default() })) };
        let (l, c) = (largo.clone(), corto.clone());
        let escritor = tokio::spawn(async move {
            escribir(&mut a, &l).await.unwrap();
            escribir(&mut a, &c).await.unwrap();
        });
        assert_eq!(leer::<MensajeParaProcesoDeBatalla, _>(&mut b).await.unwrap(), Some(largo), "300 bytes: la longitud va en dos bytes");
        assert_eq!(leer::<MensajeParaProcesoDeBatalla, _>(&mut b).await.unwrap(), Some(corto));
        escritor.await.unwrap();
        assert_eq!(leer::<MensajeParaProcesoDeBatalla, _>(&mut b).await.unwrap(), None, "cerrado entre dos mensajes: limpio");
        let (mut a, mut b) = tokio::io::duplex(64);
        a.write_all(&[0x05, 0x01]).await.unwrap();
        drop(a);
        assert!(leer::<MensajeParaProcesoDeBatalla, _>(&mut b).await.is_err(), "cortado a medias: error");
    }
}
