# Imagen del servidor (`bronzeage-servidor`) para un alojamiento de contenedores (Render). Tres etapas:
#   1. mundo:    genera el artefacto de mundo y su relieve con el generador (no están en git; unos 2 min).
#   2. compilar: compila el servidor y los bots en release.
#   3. la imagen final: los binarios (servidor y bots), el mundo y el guion de arranque, nada más.
# Con la misma semilla y el mismo código, el mundo sale con el mismo hash que en cualquier otra máquina.

FROM node:22-bookworm-slim AS mundo
WORKDIR /repo
COPY generador/package.json generador/package-lock.json generador/
RUN cd generador && npm ci
COPY proto proto
COPY buf.yaml buf.yaml
COPY scripts/generar-mundo.mjs scripts/generar-mundo.mjs
COPY generador generador
# La semilla del MUNDO (la geografía). Cambiarla es otro mundo y, por tanto, partida nueva.
ARG SEMILLA_MUNDO=42
RUN node scripts/generar-mundo.mjs ${SEMILLA_MUNDO}

FROM rust:1-slim-bookworm AS compilar
WORKDIR /repo
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY proto proto
# Los crates leen sus cifras de Docs/Mechanics al compilar (include_str!).
COPY Docs/Mechanics Docs/Mechanics
RUN cargo build --release --locked -p servicio --bin bronzeage-servidor
# Los bots (BotsBrain): proyecto aparte, fuera del workspace, que usa el crate `contrato` de aquí.
COPY bots bots
RUN cd bots && cargo build --release --locked --bin bronce-age-bots-brain

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=compilar /repo/target/release/bronzeage-servidor /app/bronzeage-servidor
COPY --from=compilar /repo/bots/target/release/bronce-age-bots-brain /app/bronzeage-bots
COPY --from=mundo /repo/generador/salida/mundo /app/mundo
COPY arranque.sh /app/arranque.sh
# El puerto lo pone la plataforma en `PORT` (el servidor escucha entonces en 0.0.0.0:PORT). La carpeta de datos es del contenedor y se pierde con él: la partida se conserva en la
# copia remota (`ALMACEN_URL`, ver DEPLOY_RENDER.md).
ENV BRONZEAGE_MUNDO=/app/mundo \
    BRONZEAGE_DATOS=/app/datos
EXPOSE 10000
# El guion lanza el servidor y, solo si se piden (`BOTS_TOTAL` y `CODIGO_REGISTRO_BOTS`), los bots a su lado.
CMD ["/app/arranque.sh"]
