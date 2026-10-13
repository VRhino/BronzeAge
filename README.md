# BronzeAge (servidor y contrato)

Juego de estrategia/MMO de la Edad del Bronce. Este repositorio es la **reconstrucción**: servidor en Rust sobre un mundo reactivo sin
tick, Unity como cliente final. El servidor TypeScript anterior (motor, bots y cliente web) está en la rama `typescript-final`.

Plan y estado: [`Consideraciones/Reconstruccion_Definicion.md`](Consideraciones/Reconstruccion_Definicion.md).

| Carpeta | Qué hay |
|---|---|
| `Docs/Game/` | Canon de las reglas del juego (numerado) |
| `Docs/Mechanics/` | Cómo funciona cada sistema (reloj, movimiento, niebla, economía...) |
| `Docs/Coordinacion/` | Contratos con Unity y los demás clientes; propuestas de Codex |
| `proto/bronzeage/v1/` | El contrato 05 en Protobuf (`buf.yaml`: `buf lint`, `buf build`, `buf breaking`) |
| `crates/contrato/` | Tipos Rust del contrato (`cargo test -p contrato`) |
| `contratos/` | Ficheros dorados y pruebas entre Rust, C# y TypeScript |
| `generador/` | Generador de mundo fuera de línea (TypeScript); produce el artefacto de mundo |
| `Consideraciones/` | Decisiones y planes vigentes |

Repos hermanos (locales, sin remoto): `BronceAgeBotsBrain` (bots, Rust) y `BronceAgeAdminClient` (admin web).
