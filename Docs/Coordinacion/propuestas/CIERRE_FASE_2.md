# Cierre de las propuestas BA-001 a BA-006 (fase 2, 2026-10-13)

La reconstrucción redefine el contrato entre BronzeAge, Unity y los demás clientes (D8): Protobuf por esquema, sesión en tiempo real,
mundo por hash. Las propuestas `BA-*` se **absorben** en ese contrato: lo que decidieron sigue vigente, pero su forma (JSON, HTTP,
semilla, tick) se sustituye. **Pendiente de confirmar con Codex** (esta carpeta es suya); el usuario decide cuándo y cómo se le avisa.

| Propuesta | Qué decidió | Dónde vive ahora |
|---|---|---|
| **BA-001** Ciclo de batalla | El servidor decide que la batalla ocurra, dónde, quién entra y sale; Unity decide lo de dentro; ticket congelado, asignación, tokens, inicio, incorporaciones, salida, resultado con idempotencia; candado de escuadras | `battle.proto` (mismos nombres v1) + sesión de servidor de batalla en `session.proto` + [`Mechanics/Batalla_Unity_Ciclo.md`](../../Mechanics/Batalla_Unity_Ciclo.md). Ampliado con asalto de caravana `disputa`/`aguante`, saqueo de pueblo y ataque a avanzada |
| **BA-002** Identidades compartidas | Ids estables de héroe, escuadra, loadout, ítem, Facción, asentamiento, ejército... compartidos entre servidor y Unity | los `id` de `entities.proto` (cadenas estables, nunca reutilizadas en una partida) |
| **BA-003** Worldgen y asentamientos 3D | Frontera híbrida: el generador regional produce el mundo, el servidor lo valida y manda sobre las reglas | [`06_Contrato_de_Mundo.md`](../06_Contrato_de_Mundo.md) + `world.proto`: artefacto inmutable por hash, paquete de Unity ×12 derivado. Sin semilla en el contrato |
| **BA-004** Modelo compartido de entidades | Héroe v1, escuadra, loadout, equipamiento, catálogos; schemas y fixtures canónicos | `entities.proto` y `battle.proto`; los catálogos (sigilos, tropas) como datos versionados del archivo común de datos de juego. Los ficheros dorados sustituyen a los fixtures JSON |
| **BA-005** Huellas y escala del asentamiento | Dimensiones lógicas ÷ 2, celda lógica y escala física, `layoutVersion` 2, origen de grilla | El **servidor calcula el trazado** (algoritmo en `Mechanics/Trazado_Urbano.md`) y lo entrega en el ámbito del asentamiento; Unity solo dibuja. La ciudad ocupa un 80 % menos del mapa (200 unidades locales por unidad de mapa) |
| **BA-006** Progresión histórica y roster | Cuatro puertas de acceso, Eras, catálogo tecnológico, roster por etapas, Aedas | Eras I-III en el canon (Doc 6) y en `Mechanics/Tecnologia_Aedas.md`; **Eras IV y V pospuestas** para después, con el motor nuevo |

## Qué necesita Codex para encajar

1. Generar `proto/bronzeage/v1/*.proto` a C# (namespace `BronzeAge.Contract.V1`) y sustituir los DTO JSON por los mensajes.
2. Sustituir `BronzeAgeCampaignConnection` (sondeo HTTP) por la sesión (`session.proto`): reloj sincronizado, planes y ritmos evaluados en cada fotograma, almacén normalizado, reanudación por versión.
3. Que el generador regional produzca el artefacto (`world.proto`) con la economía del mundo del canon 1.9c.
4. Pintar los planes que manda el servidor: la navegación local de Unity deja de decidir rutas.
