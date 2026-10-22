//! Sesión en tiempo real del contrato 05 sobre WebSocket: hola, foto, delta, órdenes con respuesta causal, reloj y reanudación (fase 5).
//!
//! # Forma
//! - Una trama binaria WebSocket = un mensaje Protobuf (`MensajeDeCliente` / `MensajeDeServidor`).
//! - El servidor **no conoce el dominio**: habla con el núcleo por canales `tokio::sync::mpsc` ([`nucleo`]). El núcleo es un
//!   actor que recibe [`ComandoSesion`] y devuelve [`Difusion`] ya construidas; aquí solo se numeran, comprimen y envían.
//!   Se eligió canal (y no una trait `Nucleo`) porque el núcleo es un actor de un solo hilo lógico (mundo reactivo sin tick):
//!   un canal lo desacopla sin `async_trait`, ordena los comandos por llegada y sirve igual a un núcleo real y al falso.
//! - La autenticación sí es una trait ([`Autenticador`]), porque llama a un servicio externo.
//!
//! # Invariantes que garantiza este crate
//! 1. **Secuencia**: cada `MensajeDeServidor` de una sesión lleva `secuencia` 1, 2, 3… sin huecos. Hay una sola tarea por sesión que
//!    numera y escribe, y la numeración se hace al codificar, no al encolar en otro sitio.
//! 2. **Causalidad**: el servidor reenvía las [`Difusion`] **en el orden exacto en que el núcleo las deposita en el canal de la
//!    sesión** y nunca reordena. Por tanto, si el núcleo deposita toda `Delta` con `hasta <= version_tras` *antes* del
//!    `ResultadoOrden` que la produjo (obligación del núcleo, con un único productor por sesión), el cliente las recibe en ese
//!    orden. Los `Pong` y errores se intercalan, pero nunca adelantan ni retrasan Delta/Resultado entre sí.
//! 3. **Una sola sesión por cliente** (`Identidad::cliente_id`): una sesión nueva *sustituye* a la vieja (que recibe
//!    `Aviso{sesion.reemplazada}` y se cierra); así una reconexión tras un corte no espera a que caduque el socket viejo.
//!    El núcleo ve `Desconecta(vieja)` antes que `Conecta(nueva)`.
//! 4. **Límite de órdenes pendientes** por sesión ([`Config::max_ordenes_pendientes`], anunciado en la `Bienvenida`): una orden
//!    de más no llega al núcleo y se contesta `ErrorSesion{demasiadas_ordenes, fatal: false}`. Una orden deja de estar pendiente
//!    cuando su `ResultadoOrden` sale hacia el cliente.
//!
//! # Obligaciones del núcleo
//! - Escribir en el canal de sesión con `try_send` y tratar `Full` como cliente lento: descartar la sesión (soltar su `Sender`
//!   hace que la sesión cierre con `interno`). Un `send().await` del núcleo contra una sesión lenta podría bloquear el mundo
//!   entero, y además puede interbloquearse con el `send` de la sesión hacia el núcleo.
//! - Contestar el `oneshot` de `Conecta` con la `Bienvenida` (el servidor la envía la primera, con `secuencia` 1) y, si procede,
//!   depositar a continuación la `Foto`/`Delta` de reanudación de cada ámbito en el canal de la sesión (uno por ámbito que no reanuda sin novedad).
//! - Avisar con `Revocado` cuando una sesión pierde el derecho a un ámbito (el cliente descarta sus entidades).
//!
//! # Compresión
//! `zstd` se negocia con `Hola.compresiones`. Como el `.proto` no tiene campo de respuesta, cada trama **se autodescribe**: una
//! trama zstd empieza por la firma `28 B5 2F FD`, y ningún mensaje Protobuf de sesión puede empezar por `0x28` (sería el campo 5
//! en varint; el campo 5 es de longitud delimitada o no existe). Quien recibe descomprime si ve la firma. El servidor solo
//! comprime si el cliente la ofreció, nunca la `Bienvenida`, y solo tramas de [`Config::umbral_compresion`] bytes o más.
//!
//! # Códigos de error
//! Los del contrato (`token_invalido`, `version_protocolo`, `demasiadas_ordenes`, `partida_no_disponible`, `rol_no_permitido`,
//! `interno`…) más, aditivos y documentados en [`codigos`]: `hola_esperado`, `hola_repetido`, `mensaje_invalido`.
//!
//! # Reanudación
//! Reanudar es **reconectar**: un nuevo `Hola` con `versiones_conocidas`, y el núcleo decide por ámbito (sin novedad, deltas o
//! foto) en la `Bienvenida`. No hay reanudación a mitad de sesión: un segundo `Hola` recibe `ErrorSesion{hola_repetido}` (no fatal) y
//! se ignora (decidido el 2026-10-13, contrato 05 §7).

pub mod nucleo;
pub mod prueba;
mod sesion;

pub use nucleo::{
    codigos, Autenticador, ComandoSesion, Config, Difusion, ErrorAcceso, Identidad, RelojCompartido, SesionId,
};
pub use sesion::{servir, Servidor};
