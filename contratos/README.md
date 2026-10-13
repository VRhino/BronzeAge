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
