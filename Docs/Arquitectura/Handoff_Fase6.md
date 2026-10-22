# Handoff de la fase 6 (reglas por sistema en Rust)

Escrito el 2026-10-21 para quien continúe el trabajo (otra sesión, la nube o Codex). Lee también `CLAUDE.md`, `Docs/Arquitectura/13_Mundo_Reactivo_Rust.md` y `Docs/Arquitectura/Sistema4_Contraste_y_Plan.md`.

## Dónde está todo

- **Rama de trabajo:** `claude/lucid-curran-6d6ffa` (en `origin`). `main` no tiene nada del Rust nuevo. Referencia del TypeScript antiguo: rama `typescript-final` (`src/`).
- **Plan vivo (artefacto):** https://claude.ai/artifact/Ng7sdxYGmSNDphMDzRGYfw (fases 0-9 y mapa de implementación de la fase 6, ≈65 %).
- **Deuda de paridad:** `Docs/Mecanicas a desarrollar.md`, sección «Paridad con TypeScript». Regla permanente del autor (2026-10-18): los recortes son temporales y el motor no está migrado hasta tener todas las funciones del TypeScript. Todo recorte nuevo es una fila ahí.
- **Calibración pendiente de decidir por el autor:** `Docs/Mecanicas a balancear.md`.

## Reglas de trabajo

1. Todo `git commit` y `git push` lo hace un subagente con `model: "haiku"`; el agente principal verifica con `git status` y `git log`. Sin `--force`. No se sube nada que el autor no haya pedido. Mensajes de commit en español y con la línea `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` (hay que pedirla explícita al subagente).
2. Si el canon (`Docs/Game/`) y `Docs/Mechanics/` se contradicen, gana Mechanics y se corrige el canon. Los bloques `<!-- parametros:NOMBRE -->` de Mechanics se incrustan con `include_str!`; en el sistema 4 se reescribieron con lo calibrado y son la fuente de verdad.
3. Ante ambigüedad real: preguntar al autor, o elegir lo más conservador, marcar `// DUDA:` y anotarlo. Responder siempre en español.
4. Cada fase termina con: `cargo test --workspace --no-fail-fast -j 1` en verde (sin `-j 1` falla el enlazado de los ejemplos `medir` y `rendimiento`, fallo preexistente), notas «Implementado en Rust (fecha, fase)» en los Mechanics afectados, filas de paridad, foto con formato nuevo (`BAN16` hoy), determinismo normal/acelerado/a trozos, sin eventos recurrentes por entidad ociosa, y paso de minuto sin empeorar (`cargo test --release -p nucleo --test asentamientos rendimiento -- --ignored`).
5. Patrón de integración en `crates/nucleo`: reglas puras en un crate propio (`asentamiento`, `comercio`, `militar`, `combate`), estado en `estado*.rs`, codecs `codec_*`, comandos nuevos a continuación del último (hoy el 115) con rechazos documentados, hechos de registro y eventos de la cola con la prioridad de `Docs/Mechanics/Reloj_Y_Eventos.md`.
6. La verificación del batch se deja para después cuando el autor lo pide; no corras batches largos sin que lo pida.

## Estado (≈65 % de la fase 6)

Hecho y commiteado: sistema 1 (mundo y territorio), 2 (población y construcción), 3 (economía y comercio, con decisiones C1-C6 aplicadas) y, del 4 (militar y combate), 4A-1, 4A-2, 4B-1, 4B-2, 4C, 4D y 4E; la 4F (avanzadas, 2026-10-22) está hecha en el árbol de trabajo sin commitear, con las decisiones del autor del mismo día aplicadas (solo defiende lo que está dentro, colocación por tamaño, tropa elegible entre lo que la madre recluta, cancelar obras; foto `BAN16`). 1.014 tests en verde (64 binarios, 5 ignorados).

## Siguiente, en este orden

1. **Murallas** como fase propia (hoy recinto 0): empalizada nv2, piedra nv3, adarve nv4. Ver `Docs/Mechanics` de murallas y `Combate.md` §6.
2. **4G batch militar** (`batch militar`, patrón de `batch comercio` en `crates/batch`) y volver a medir comercio tras C2-C4 (fila en «Paridad»).
3. **4H contrato y proyecciones**: mapear al Protobuf (`proto/bronzeage/v1/`, contrato 05) las órdenes del núcleo; hoy solo `MarcharA`, `Estacionar` y `Reloj` están mapeadas en `sesiones::traducir`. Incluye snapshots del ticket de batalla y envío a Unity.
4. **Sistema 5 (política)**: cargos, residencia completa, relaciones y guerra, XP de Facción, derrota y traslado. Sustituye los stubs de administración (`FijarFaccionDeHeroe`, `FijarRelacion`, `FijarResidencia`, `DarAlmacenPersonal`, `ColocarHeroe`, `CrearEjercito`, `DarSuministro`). Para cada uno haz contraste canon/Mechanics antes y lanza las preguntas al autor.
5. **Sistema 6 (tecnología y Aedas)**: sustituye `FijarTecnologias` y el plazo de 72 h de las tecnologías del campamento (hace falta `primeros`). Eras IV-V pospuestas.
6. Después: fase 7 (bots y admin web), 8 (Unity), 9 (despliegue y llevar la rama a `main` por PR, decisión del autor).

Método que ha funcionado en cada sistema: contraste de canon y Mechanics con preguntas al autor (máximo 8, con opción recomendada), descomposición en fases (reglas puras, luego núcleo, luego batch), un subagente por fase, verificación propia del agente principal y commit por Haiku.

## Decisiones abiertas del autor

- Entrar en un pueblo extractor o en una avanzada: la regla «solo defiende lo que está dentro» (`Movimiento_Ejercitos.md` §11.1) los deja con solo su guarnición porque no existe la interacción de entrar; si se quiere que defiendan héroes, hay que diseñarla.
- Repostar automático por zona: falta decidir el ritmo de rellenado (la plaza gasta trigo por encima de su reserva); la zona es un disco recortado por semiplanos, convexa, así que la entrada y salida exactas son viables.
- Calibración de reputación, ventana de acarreo, cifras de mercenarios y caravanas del Mercado: dejadas como están hasta medir con bots (fase 7).
- Ganado (P5 del sistema 1): dejado y por medir con comercio.
- Las sesiones de nube no avisan al terminar y no ven los ficheros locales: todo lo que necesiten debe estar en el repo (por eso existe este documento). La nube parte de lo que esté en `origin`.

## Pendientes menores

- `crates/batch/src/medidas.rs:27`: variable `t` sin usar.
- Directorios de compilación sobrantes en `C:/` (`bt-*`), borrables.
- Comentario obsoleto en `crates/nucleo/src/nucleo/mercado.rs` («el bloque dice 10; el vigente es 2»).
- Bloques de parámetros repetidos entre documentos de Mechanics (`LIDERAZGO`, `HEROE`, `CHATARRA`, `VISION`, `PRESENCIA`, `MERCENARIOS`): hoy los iguala un test; conviene dejar uno solo.
- El commit `2a7d447` está firmado «Haiku» en vez de «Sonnet»; no se corrige para no reescribir historia subida.
