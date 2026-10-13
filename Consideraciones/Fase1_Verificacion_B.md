# Fase 1 — Verificación de la tabla B

> **Estado (2026-10-13): en curso; ocho documentos verificados y retirados.** Por cada documento ya implementado se comprueba que ninguna decisión viva solo en él: lo
> vigente tiene que estar en `Docs/Game/` o en `Docs/Mechanics/`; lo abierto, decidido o en `Docs/Ideas a diseñar.md`. Cuando
> un documento queda verificado se **borra** de `Consideraciones/` (la historia queda en git y en `typescript-final`, decisión
> del autor del 2026-10-09 y del 2026-10-13). Nada se borra sin tu visto bueno.

| Documento | Resultado | Pendiente de decidir |
|---|---|---|
| `Jugador_Situado_Definicion.md` | Sin nada vivo en el documento: las 36 decisiones están en el código y en `Mechanics` (Presencia, Movimiento, Heroe_Y_Tropa); los abiertos están cerrados. El caso «el jugador dentro de una plaza que cambia de manos» lo resuelve el desalojo (ningún héroe queda dentro tras la conquista, `Combate.md` §9). Los bandidos que atacan caravanas ya constan en el canon (1.9, 3.10). | V4 (tope del Tesorero) |
| `Movimiento_Ejercitos_Definicion.md` | Reglas y constantes en `Mechanics/Movimiento_Ejercitos.md`, `Heroe_Y_Tropa.md` y canon 5.12-5.14. Quedan tres ideas que no están en ningún sitio. | V1, V2, V3 |
| `Campamentos_Entrada_Fundacion_Definicion.md` | Cerrado entero (71 decisiones, sin puntos abiertos): canon 1.9b y `Mechanics` (Campamentos_Mercenarios, Fundacion_Y_Refundacion, Presencia). | — |
| `Murallas_Definicion.md` | Vigente en `Mechanics/Murallas.md` y canon 4.2.1; las cifras son placeholder. Quedan dos ideas aplazadas. | V5 |

## Segunda tanda (2026-10-13)

| Documento | Resultado | Pendiente de decidir |
|---|---|---|
| `Anexion_Y_Desarme_Definicion.md` | Sin nada vivo: reglas en canon 2.4 y 2.6 y en `Mechanics/Diplomacia_Facciones.md` (caducidad a 3 días, una pendiente por par, anexión y fusión a la vez). Lo «fuera de esta pasada» eran pantallas del cliente web, que se retira. | — |
| `Batalla_Este_Lado_Definicion.md` | Sin nada vivo: las 10 decisiones y el reparto con Unity están en `Mechanics/Batalla_Unity_Ciclo.md` y canon 5.15; el canal `batalla/<id>` y CQ-011 pasan al contrato 05. | — |
| `Comercio_Fisico_Definicion.md` | Reglas en `Mechanics/Comercio_Caravanas.md` (§8.3 el mostrador). Dos palancas sin diseñar. | W1 |
| `Revamp_Caravanas_Definicion.md` | Reglas en canon 3.13 y `Mechanics/Comercio_Caravanas.md`; los trozos diferidos ya están en Ideas §47. | W2, W3 |

- **W1. Comercio del forastero**: ¿puede publicar sus propias órdenes en un mercado ajeno, o solo tomar las publicadas?, y ¿puede el
  Gobernador cerrar el mostrador sin cerrar la puerta (dejar pasar pero no vender, o vender solo a aliados)? *Recomiendo `Docs/Ideas a diseñar.md`.*
- **W2. Experiencia de Facción por defender una caravana** (`NIVEL_FACCION.xp.defensaCaravana`): hoy no se multiplica por los
  jugadores que participan porque la escolta no tenía héroes reales. Con escolta real, ¿se multiplica como el resto del combate?
  No está en `Mechanics`. *Recomiendo que sí, igual que el resto del combate,* y escribirlo en el canon (5.16).
- **W3. El camello sin su inmunidad al desierto** sigue en el catálogo como «opción media». *Recomiendo dejarlo y revisarlo al calibrar con el motor nuevo.*

### Decidido de la segunda tanda
W1 al `Docs/Ideas a diseñar.md` §47; W2 escrito en el canon (2.2: la defensa de una caravana da experiencia de Facción por héroe participante); W3 se queda. Retirados `Anexion_Y_Desarme`, `Batalla_Este_Lado`, `Comercio_Fisico` y `Revamp_Caravanas`.

## Tercera tanda (2026-10-13)

| Documento | Resultado | Pendiente de decidir |
|---|---|---|
| `Entrada_Al_Mundo_Definicion.md` | Cerrado: lo que quedaba (la vida de un huésped) se descartó el 2026-10-03; las reglas están en canon 1.3 y 1.9b y `Mechanics/Campamentos_Mercenarios.md`. | — |
| `Identidad_Visual_Definicion.md` | Cerrado (sigilo, estandarte e insignias de títulos en canon 2.8.1). Quedan dos cosas externas: siete emblemas sin icono (escudo en ocho, Puerta de los Leones, águila bicéfala hitita, toro alado, árbol sagrado, casco de colmillos, sol de Vergina) y las insignias de títulos de jugador-poseedor cuando exista «general con más victorias». | X1 |
| `Rutas_Caravana_Avanzadas_Definicion.md` | Reglas en canon 1.6 y `Mechanics/Comercio_Caravanas.md`. Los puntos abiertos (vados, separación del grafo, bosques, escala del regional) quedan superados por la calibración (`NAVEGACION`, mundo de 2.667) y por el artefacto de mundo; el logro `logistica_campana` va con la Era IV, pospuesta. | — |
| `Taberna_Intel_Definicion.md` | Reglas en canon 5.12.10 y 4.2.1. Un héroe sin Facción no compra: ya consta en el canon (5.12.10). Quedan sin diseñar los rumores de las tabernas y el «Estado de Facción». | X1 |

- **X1.** Pasar a `Docs/Ideas a diseñar.md` los siete emblemas por encargar (tarea del lado de Unity), las insignias de títulos de
  jugador-poseedor, los rumores de las tabernas y el «Estado de Facción». *Recomiendo hacerlo.*

## Decidido el 2026-10-13

- **V1** varianza ±15 %: se deja como está (constante del archivo común). **V2** escuadra mermada: regla escrita en el canon (5.11.1). **V3, V4** y la demolición de recintos: a `Docs/Ideas a diseñar.md` §47. **Jerarquía de puertas: descartada.**
- Retirados de `Consideraciones/` (siguen en git y en `typescript-final`): `Jugador_Situado`, `Movimiento_Ejercitos`, `Campamentos_Entrada_Fundacion`, `Murallas`. Las referencias de los `.md` apuntan ahora a `typescript-final:Consideraciones/...`.

## Propuesta original

- **V1. Varianza de combate** (±15 %, `MILITAR.varianzaCombate`, canon 5.2.5). El documento de movimiento la dejó abierta con
  tres opciones (dejarla, bajarla a 0,03-0,05, quitarla entre Facciones). Hoy solo afecta a las batallas que se resuelven con
  números (bots y PvE): las humanas las decide Unity. *Recomiendo dejarla como está* y reservarla como constante del archivo de
  datos común; el núcleo reactivo la usa con una semilla propia por batalla.
- **V2. Una escuadra mermada paga el Liderazgo completo** (3/20 cuesta lo mismo que 20/20). Es lo que impide el exploit de la
  élite parcial. *Recomiendo dejarlo y anotarlo como regla* en el canon.
- **V3. Órdenes condicionales** (reglas de enfrentamiento y retirada automática por umbral de bajas) para quien vuelve tras
  horas sin conectar. No está en ningún sitio. *Recomiendo `Docs/Ideas a diseñar.md`.*
- **V4. Tope del Tesorero** a cuánto material puede retirar un jugador (gancho previsto en la decisión 3 de Jugador_Situado).
  *Recomiendo `Docs/Ideas a diseñar.md`.*
- **V5. Jerarquía de puertas** (postigo / puerta / casa-puerta) y **demolición de un recinto completo**. *Recomiendo `Docs/Ideas a diseñar.md`.*

## Quedan por verificar (4)

`Anexion_Y_Desarme`, `Batalla_Este_Lado`, `Comercio_Fisico`, `Entrada_Al_Mundo`, `Identidad_Visual`, `Revamp_Caravanas`,
`Rutas_Caravana_Avanzadas`, `Taberna_Intel`, `Fase_0_5_Especializacion_y_Cupos`, `Fase_0_6_Expansion_Niveles_Asentamiento`,
`Checklist_Mecanicas`, `Economia_Del_Oro`, `Ocupacion_Post_Conquista`, `Niebla`/`Aedas`/`Trazado` (ya borrados) y
`Docs/Mecanicas a desarrollar.md` §8.1, §20, §38, §44-§46 (decididos el 2026-10-13, falta aplicar).
