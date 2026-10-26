# Handoff de la fase 6: entrada a la fase 7

Escrito el 2026-10-21 y reescrito el 2026-10-26, al cerrar la fase 6, como **el documento de entrada a la fase 7** para quien continúe (otra sesión, la nube o Codex). Lee también `CLAUDE.md`, `Consideraciones/Reconstruccion_Definicion.md` (las fases, §4) y `Docs/Arquitectura/13_Mundo_Reactivo_Rust.md` (el diseño del núcleo).

**La fase 7 (bots en Rust y admin web) no se empieza sin una orden directa del autor.**

## Estado final de la fase 6 (2026-10-26)

- **Hecha**: las reglas de los seis sistemas del canon en Rust, cada uno con su batch de validación: mundo y territorio (1), población y construcción con el trazado urbano y las murallas (2), economía y comercio (3), militar y combate (4), política y Facciones (5) y tecnología y Aedas (6), más el contrato 05 y sus proyecciones (4H) y la crónica entera.
- **Rama**: `claude/lucid-curran-6d6ffa` (en `origin`). Último commit de código de la fase: `68d14d04` (6H, el batch tecnológico); después, solo este cierre de documentación (rama `docs-cierre-fase6`). `main` no tiene nada del Rust nuevo; el TypeScript de referencia está en la rama `typescript-final` (`src/`).
- **Foto**: `BAN40` (los Aedas residentes y las épicas, 6E). Cada formato anterior se rechaza con su motivo.
- **Pruebas**: `cargo test --workspace --no-fail-fast`: **1.582 en verde, 16 ignoradas, 106 resultados** (al cerrar 6H). Auditoría: **591 sin fallos**. Dorados: **89 pares**, verdes en Rust, C# y TypeScript.
- **Contrato**: 145 órdenes; el núcleo tiene **170 comandos** (1-160 sin el 117, retirado en 5H, y 168-178): 146 de órdenes del jugador, 19 de la orden genérica de administración y 5 del canal del servidor de batalla. **Ninguna orden da `orden.no_disponible`.** 288 códigos de evento en el catálogo (286 emitidos; los dos pendientes, los yacimientos). Rechazos: `Docs/Coordinacion/07_Catalogo_de_Rechazos.md`.

### Crates (`crates/`)

| Crate | Qué es |
|---|---|
| `base` | tipos y matemática comunes: tiempo de mundo, punto fijo, planes de movimiento, ritmos, reloj |
| `cola` | la cola de eventos del núcleo (Reloj_Y_Eventos §2) |
| `encuentros` | encuentros exactos entre planes: rejilla espacio-temporal, persecución e intercepción |
| `vision` | visión muestreada cada 250 ms por Facción, en paralelo |
| `persistencia` | registro de eventos + fotos (event sourcing) y el diario de la crónica (`Diario`) |
| `mundo` | cargador y validador del artefacto de mundo (`06_Contrato_de_Mundo.md`) |
| `rutas` | A* sobre el grafo de navegación del artefacto |
| `territorio` | reglas puras de mundo y territorio (Doc 1) |
| `asentamiento` | población, economía y construcción de una plaza (sistema 2) |
| `trazado` | trazado urbano y murallas (sistema 2) |
| `comercio` | economía y comercio (sistema 3) |
| `militar` | héroe, tropa y columna (sistema 4) |
| `combate` | combate con números y ciclo de la batalla de Unity (sistema 4) |
| `politica` | política y Facciones (sistema 5) |
| `tecnologia` | tecnología por Eras y Aedas (sistema 6) |
| `nucleo` | el núcleo de un solo hilo del mundo reactivo: estado, órdenes, eventos, proyecciones y sesiones |
| `servidor` | la sesión en tiempo real del contrato 05 sobre WebSocket |
| `contrato` | los tipos del contrato 05 generados desde `proto/` y el almacén de referencia de un cliente |
| `batch` | los batches de validación de cada sistema (bots mínimos que solo dan órdenes del núcleo o del contrato) |
| `informe` | el informe de un batch: del JSON medido a una página HTML |

## Cómo se comprueba

- **La suite**: `cargo test --workspace --no-fail-fast` (una pasada). El hook de pre-commit corre `buf lint && buf build && cargo test -q --workspace`. Compila en el `target/` de tu worktree o en `C:/Users/VRINO/.cargo-targets/`, nunca en `C:/bt-*`, y borra lo que sobre al terminar. Hace falta el artefacto de mundo (ignorado por git) en `generador/salida/mundo/mundo-*.bin`: cópialo del checkout que lo tenga. `STATUS_DLL_INIT_FAILED` (0xc0000142) es del entorno: reintenta una vez.
- **El contrato**: `buf lint`, `buf build` y `buf breaking` (en la raíz); los dorados en C# (`dotnet test` en `contratos/cs`) y en TypeScript (`npm test` en `contratos/ts`; `GOLDEN_UPDATE=1 npx vitest run` regenera los `.bin`). Guía en `contratos/README.md`.
- **La auditoría** del registro de cambios: `BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test -p nucleo -- --skip rendimiento` (591 sin fallos al cerrar la fase).
- **El catálogo**: `crates/nucleo/tests/catalogo.rs` exige que cada código de evento que emite el núcleo esté en el bloque `<!-- eventos -->` de algún documento de `Docs/Mechanics`, que los que no emite tengan su fila en `Docs/Mecanicas a desarrollar.md` y que cada rechazo esté en `07_Catalogo_de_Rechazos.md` (`BRONZEAGE_REGENERAR=1` lo regenera).
- **Las pruebas de sesión** (los 21 binarios con pruebas por el WebSocket real: `a_mano`, `admin`, `aedas`, `aedas_residentes`, `alijos`, `ambitos`, `batalla_unity`, `carga`, `cliente_de_ejemplo`, `cronica`, `eventos`, `fundacion`, `interior`, `ordenes`, `ordenes_mundo`, `propios`, `reanudacion`, `sesiones`, `tecnologias`, `vista` y `servidor/tests/sesion.rs`): **20 repeticiones seguidas solo si la tanda toca la sesión, las proyecciones, el contrato o los hechos**; si no, una pasada.
- **Los parámetros**: los bloques `<!-- parametros:NOMBRE -->` de `Docs/Mechanics` se incrustan con `include_str!` y entran en la huella de las reglas; la matriz de autorización de Proyecciones §5.1 la comprueba una prueba contra `Comando::autoriza`.
- **El andamiaje**: `BRONZEAGE_SIN_ANDAMIAJE=1` apaga el `comun::mandaba` de las pruebas antiguas y mide cuántas lo necesitan.

## Los batches y sus informes

`cargo run --release -p batch -- <sistema> [opciones]` (uso completo en la cabecera de `crates/batch/src/main.rs`). Cada uno escribe su informe (JSON y HTML) en `batch/salida/<sistema>/` (ignorado por git) y funde las conclusiones y preguntas de `crates/batch/decisiones/<sistema>.json` con `--decisiones`. Todas sus preguntas están cerradas por el autor salvo D2 y D3 de `territorio.json`. Semilla 42 por defecto; cada uno se ha corrido dos veces con la misma huella y con la semilla 7.

| Batch | Escenario | Pared medida (release) | Decisiones |
|---|---|---|---|
| `territorio` | 400 Facciones × 30 días | ≈ 186 s | `territorio.json` (D2, D3 abiertas) |
| `poblacion` | 300 plazas × 30 días (`--murallas 1`) | ≈ 47-74 s | `poblacion.json`, `Calibracion_Sistema4_Decisiones.md` |
| `comercio` | 160 plazas × 30 días | ≈ 73-104 s | `comercio.json`, ídem |
| `militar` | 60 plazas × 30 días | ≈ 480 s | `militar.json`, ídem |
| `politica` | 60 plazas × 30 días (`--nobleza 1`) | ≈ 430-440 s | `politica.json`, `Politica_Tecnologia_Decisiones.md` |
| `tecnologia` | 60 plazas × 42 días (`--escala-logros 1`) | ≈ 100 s (seis procesos a la vez) | `tecnologia.json`, ídem |

Con 0 violaciones de sus invariantes en todas las corridas. Las pruebas cortas de cada batch, en `crates/batch/tests/`.

## Rendimiento de referencia (release, máquina de 16 hilos compartida: ruido de ±20 %)

| Qué | Cifra al cerrar la fase (6G) | Cómo |
|---|---|---|
| Paso de minuto: 1.000 plazas × 3 días | 20,4-20,7 s; 3.069.110 pasos, versión 50.408 | `cargo test --release -p nucleo --test asentamientos rendimiento -- --ignored` |
| Carga: 5 min de mundo en pasos de 250 ms con 500 sesiones | 32-36 s de actor | `crates/nucleo/tests/carga.rs` (ignorada) |
| Coste por orden con el reloj parado y 500 sesiones | ≈ 21-24 ms | ídem |
| Crónica: 100.000 entradas por el núcleo | 13,3 MB; página de 200 en 0,66 ms; recuperar 0,22 s; ≈ 2,6 µs por entrada | `crates/nucleo/tests/cronica.rs`, `persistencia` |
| Batch militar | ≈ 9,7-11 s de CPU por día de mundo | `batch militar` |
| Batch tecnológico | ≈ 2,4 s de CPU por día de mundo | `batch tecnologia` |

Una orden cuesta lo que cuesta reconciliar las vistas: con ≈ 450 vistas el techo es de ≈ 65 órdenes/s (4H, T6). Lo que hay que medir con 500 bots está en «Para la fase 7» de `Docs/Mecanicas a desarrollar.md`.

## Reglas de trabajo

1. **Commits**: todo `git commit` y `git push` lo hace un subagente con `model: "haiku"`, lanzado en primer plano con el prompt completo (ficheros, mensaje exacto, rama, `push -u` si se pide); el agente principal verifica con `git status` y `git log`. Sin `--force`, sin `--no-verify`, sin reescribir historia. Mensajes en español, terminados en la línea de atribución que pida el entorno (hay que dársela explícita al subagente). No se sube nada que el autor no haya pedido. Ver `CLAUDE.md`.
2. **Tandas en worktrees**: cada tanda en su worktree y su rama, sobre la punta de `origin/claude/lucid-curran-6d6ffa`; se integra por avance rápido (`merge --ff-only`) o, si `origin` avanzó, con un merge (lo hace el Haiku) y la suite otra vez.
3. **Fuente de verdad**: si el canon (`Docs/Game/`) y `Docs/Mechanics/` se contradicen, gana Mechanics y se corrige el canon (`Docs/Mechanics/README.md`). Las reglas de juego se escriben en el canon al implementar; `Consideraciones/` lleva solo decisiones y planes; cada cambio de regla actualiza su documento de Mechanics y deja su nota «Implementado en Rust (fecha, tanda)»; no se cambian cifras de un bloque de parámetros sin decisión del autor.
4. **Paridad**: los recortes frente al TypeScript son temporales y el motor no está migrado hasta tener todas sus funciones (regla del autor del 2026-10-18). Todo recorte es una fila de `Docs/Mecanicas a desarrollar.md`, que es la lista única de pendientes: **lo cerrado se borra entero**.
5. **Dudas**: ante ambigüedad real, preguntar al autor (máximo 8 preguntas, con la opción recomendada) o elegir lo más conservador, marcar `// DUDA:` y anotarlo; al decidirse, la `// DUDA:` se reescribe como regla con su fecha. Las reglas de juego son del autor: ante una objeción técnica, buscar la solución que conserva la regla.
6. **Batches**: no se corren batches largos sin que el autor lo pida; una semana por corrida encadenando fotos si se mide por semanas.
7. **Método por sistema** (el que funcionó): contraste de canon, Mechanics y TypeScript con preguntas al autor; tandas (reglas puras, núcleo, contrato, batch), un subagente por tanda; verificación propia del agente principal; commit por Haiku.
8. **Patrón del núcleo**: reglas puras en su crate; estado en `estado*.rs` y códecs `codec_*`; comandos nuevos a continuación del último (hoy el **178**) con sus rechazos en el catálogo, su orden del contrato en `sesiones/ordenes.rs` (la prueba de exhaustividad de `sesiones/ordenes/pruebas.rs` falla hasta clasificarla) y su fila en la matriz de Proyecciones §5.1; hechos por `Transicion::emitir` y eventos de la cola con la prioridad de `Reloj_Y_Eventos.md`; sin eventos recurrentes por entidad ociosa; foto con formato nuevo si cambia el estado; determinismo normal, acelerado y a trozos; el paso de minuto sin empeorar.

## Dónde está cada cosa

- **Decisiones del autor**: `Consideraciones/Dudas_Del_Codigo_Decisiones.md` (las `// DUDA:` de antes del sistema 5 y el laboratorio del trazado), `Consideraciones/Calibracion_Sistema4_Decisiones.md` (los batches del sistema 4, la economía entera y las calibraciones 1 y 2) y `Consideraciones/Politica_Tecnologia_Decisiones.md` (los sistemas 5 y 6, hasta las de 6G y 6H del 2026-10-26).
- **Planes de cada sistema** (contraste, tandas, avance con cifras y dudas): `Docs/Arquitectura/Sistema4_Contraste_y_Plan.md`, `4H_Contraste_y_Plan.md` (contrato y proyecciones), `Murallas_Contraste_y_Plan.md`, `Economia_Entera_Plan.md`, `Sistema5_Contraste_y_Plan.md` y `Sistema6_Contraste_y_Plan.md`; los de los sistemas 1-3, en `Consideraciones/` (`Economia_Por_Minuto_Definicion.md`, `Entrar_Con_Todo_Definicion.md`) y en las notas de Mechanics.
- **Pendientes**: `Docs/Mecanicas a desarrollar.md` (paridad, dudas abiertas y «Para la fase 7»); calibración, `Docs/Mecanicas a balancear.md`.
- **Contratos**: `Docs/Coordinacion/05_Contrato_de_Sesion.md`, `06_Contrato_de_Mundo.md`, `07_Catalogo_de_Rechazos.md`, `01_Modelo_de_datos.md`, `proto/bronzeage/v1/` y `contratos/README.md`; la forma de cada comando y su traducción, `Docs/Mechanics/Comandos_Parametros.md`.
- **Bots** (fase 7): `Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md`; repos hermanos locales `BronceAgeBotsBrain` (bots, Rust) y `BronceAgeAdminClient` (admin web).
- **Plan vivo (artefacto)**: https://claude.ai/artifact/Ng7sdxYGmSNDphMDzRGYfw (fases 0-9 y el mapa de la fase 6).

## Historia de la fase 6, por sistema

| Sistema | Qué se hizo | Detalle |
|---|---|---|
| 1. Mundo y territorio | fundar, zonas que crecen solo por edificio completado y chocan, parcelas de bosque, `batch territorio` | `Docs/Mechanics/Fundacion_Y_Refundacion.md`, `Generacion_Del_Mundo.md` |
| 2. Población y construcción | economía por minuto con fines exactos, trazado urbano (`LAYOUT_VERSION` 6), murallas (empalizada, piedra, adarve), `batch poblacion` | `Consideraciones/Economia_Por_Minuto_Definicion.md`, `Murallas_Contraste_y_Plan.md`, `Dudas_Del_Codigo_Decisiones.md` |
| 3. Economía y comercio | trueques, mercado, caravanas, red de caminos, pueblos extractores y acarreo, economía entera, `batch comercio` | `Comercio_Caravanas.md`, `Economia_Entera_Plan.md` |
| 4. Militar y combate | héroe, tropa, columnas, convocatorias, combate con números, bandidos, avanzadas, la batalla de Unity, «entrar con todo», `batch militar` | `Sistema4_Contraste_y_Plan.md`, `Entrar_Con_Todo_Definicion.md`, `Calibracion_Sistema4_Decisiones.md` |
| 4H. Contrato y proyecciones | ámbitos `HEROE`, `FACCION`, `ASENTAMIENTO`, `CAMPAMENTO`, `VISTA` con niebla, `ADMIN` y `BATALLA`; hechos de dominio, reanudación y carga | `4H_Contraste_y_Plan.md` |
| 5. Política | Facciones, ciudadanía y residencia, experiencia y capital, relaciones, cargos y puerta, anexión y fusión, títulos, refundación, contrato, `batch politica` (fotos `BAN28`-`BAN35`) | `Sistema5_Contraste_y_Plan.md`, `Politica_Tecnologia_Decisiones.md` |
| 6. Tecnología y Aedas | reglas puras, alijos, crónica entera, logros y Eras, aparición y adopción, Aedas itinerantes y residentes con sus épicas, contrato, `batch tecnologia` (fotos `BAN36`-`BAN40`) | `Sistema6_Contraste_y_Plan.md`, `Politica_Tecnologia_Decisiones.md` |

## Pendientes menores

- Bloques de parámetros repetidos entre documentos de Mechanics (`LIDERAZGO`, `HEROE`, `CHATARRA`, `VISION`, `PRESENCIA`, `MERCENARIOS`): hoy los iguala una prueba; conviene dejar uno solo.
- `MANTENIMIENTO.escalaDistancia`: el bloque del TypeScript dice 400 y el código lleva 300 como valor vigente (P1); decidir si pasa al bloque (duda abierta).
- El commit `2a7d447` está firmado «Haiku» en vez de «Sonnet»; no se corrige para no reescribir historia subida.
- Las sesiones de nube no avisan al terminar y no ven los ficheros locales: todo lo que necesiten debe estar en el repositorio (por eso existe este documento). La nube parte de lo que esté en `origin`.
