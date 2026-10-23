# Contratos: pruebas entre lenguajes

Los esquemas viven en `../proto/bronzeage/v1/`. Aquí se comprueba que **Rust, C# y TypeScript** los leen y escriben igual.

| Carpeta | Qué es | Cómo se ejecuta |
|---|---|---|
| `golden/` | Mensajes de ejemplo: `<Tipo>__<caso>.json` (fuente, JSON de Protobuf) y `.bin` (codificación canónica) | — |
| `ts/` | Genera TypeScript con `buf` + `protoc-gen-es` y regenera los `.bin` | `npm install && npm test` (`GOLDEN_UPDATE=1` regenera) |
| `cs/` | C# con `Grpc.Tools`: el JSON y el .bin deben dar los mismos bytes | `dotnet test` |
| `../crates/contrato` | Rust con `prost`: decodifica y recodifica cada .bin sin cambios | `cargo test -p contrato` |

Reglas para un fichero dorado nuevo: un `Struct` con **una sola clave** y mapas de **una sola entrada** (el orden de los mapas no está fijado
por Protobuf); después, `GOLDEN_UPDATE=1 npx vitest run` en `ts/`. Un cambio en un `.bin` ya commiteado es un cambio de contrato.
`buf breaking` (en la raíz) comprueba lo mismo contra el esquema.

**Qué cubren (4H, T1, 2026-10-22, y T3b):** 62 pares. Los 11 de la fase 2 (hola, ping, `atacar`, `controlar_reloj`, foto y delta de una columna, resultado rechazado, bienvenida, manifiesto de mundo) y 51 de 4H: una foto de cada entidad que
faltaba (`Heroe` también con su residencia, `Escuadron`, `Asentamiento` con economía en escalones, población entera, recinto y trazado, `Pueblo`, `Avanzada`, `ObraDeMapa`, `Caravana`, `Convocatoria`, `Batalla` con su lado para unirse y su token propio, la columna dentro de un lugar, tres
`InformeInspeccion`, la vista con niebla), la forma híbrida de un cambio (completo, parcial con máscara y `QUITA`), `Revocado`, `Aviso`, los 15 eventos tipados, la bienvenida de quien aún no tiene héroe, los resultados con `creadas` y con el detalle de `guerra.confirmar`, los
mensajes del canal de batalla en los dos sentidos (ticket de asedio con recintos, ticket de caravana con dos caravanas, incorporación, tokens, acuse, asignación sin tokens, resultado con carros perdidos) y las órdenes nuevas (145-154, `repostar` con lugar y una de cada familia).
Los `.json` se escriben a mano o con un script; los `.bin` los regenera siempre `GOLDEN_UPDATE=1 npx vitest run`. `cargo test -p contrato` además lee varios de ellos y comprueba su **significado** (que cada campo nuevo está donde el esquema lo pone).
