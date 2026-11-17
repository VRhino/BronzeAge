# BronceAgeBotsBrain

Bots de BronzeAge en Rust: servicio aparte que usa el contrato 05 como un jugador, sin privilegios.

Contrato: `proto/bronzeage/v1/` en BronzeAgeFase0 (D8). Plan: `Consideraciones/Reconstruccion_Definicion.md` de ese repo.

## El puerto (7C)

`Puerto` (`src/puerto.rs`) es lo único que un cerebro conoce del mundo: `observar()` (sus ámbitos con niebla, en el `Almacen` del contrato) y `actuar(orden)` (respuesta causal). Un solo adaptador: el WebSocket del contrato 05 como `CLIENTE_BOT`, con alta y login por `src/acceso.rs`. Ante un corte, un hueco de secuencia o un token caducado reconecta con lo que conserva y reenvía la orden con el mismo `peticion_id`.

Prueba contra el servidor real (se omite si falta el binario o el artefacto de mundo):

```sh
cargo build --release -p servicio --manifest-path ../BronzeAgeFase0/Cargo.toml --target-dir target/motor
cargo test
```

## El runner (7D)

`Proceso` (`src/runner.rs`) maneja a todos los bots de una partida, cada uno con su conexión: las altas según el plan de llegadas (`src/llegadas.rs`, D56-D57: grupos de cinco amigos al mismo campamento, solitarios y tardíos), el horario de conexión (D55, `conectarse`/`desconectarse`), el ritmo (cada `cada_ms` de mundo, o antes si un hecho nombra a su héroe), la pizarra por Facción y la memoria desechable, que define cada `Cerebro`. Guarda un registro JSON (cuentas, héroes, perfiles y llegadas hechas) para volver con los mismos héroes; si el servidor tiene otro mundo, se niega a arrancar.

## Los cerebros (7E)

`src/cerebro.rs`: hoy el bot **sin plaza**. Forma su Facción según su perfil (el Rey acepta toda solicitud), pide la leva, explora el anillo, caza en grupo lo que pueden vencer los que están listos, junta el fondo, compra la Caravana de Fundación, funda en un sitio que el mapa da por bueno y los suyos se mudan a la plaza nueva. Las cifras salen de `GET /v1/datos` del motor (`src/datos.rs`) y el terreno del artefacto de mundo (`src/mapa.rs`); ninguna va copiada. Prueba a mano: `BOTS_FACTOR=360 BOTS_TRAZA=1 cargo test --test cerebro -- --ignored --nocapture` (seis horas de mundo).

## El batch «partida» (7F)

`cargo run --release --bin partida`: arranca el servidor con una partida nueva, lo acelera con la cuenta de administración (solo eso: los bots juegan como jugadores, sin andamiaje) y deja jugar a `BOTS_TOTAL` bots `PARTIDA_DIAS` días de mundo a ×`PARTIDA_FACTOR`. Escribe `partidas/partida-<semilla>.md` y `.json`: una fila por día con lo que ve `ADMIN` (Facciones, plazas y niveles, héroes con plaza, escuadras, batallas, caravanas), las órdenes de los bots por resultado y los hechos.
