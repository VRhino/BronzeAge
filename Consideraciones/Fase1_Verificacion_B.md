# Fase 1 — Verificación de la tabla B

> **Estado (2026-10-13): en curso.** Por cada documento ya implementado se comprueba que ninguna decisión viva solo en él: lo
> vigente tiene que estar en `Docs/Game/` o en `Docs/Mechanics/`; lo abierto, decidido o en `Docs/Ideas a diseñar.md`. Cuando
> un documento queda verificado se **borra** de `Consideraciones/` (la historia queda en git y en `typescript-final`, decisión
> del autor del 2026-10-09 y del 2026-10-13). Nada se borra sin tu visto bueno.

| Documento | Resultado | Pendiente de decidir |
|---|---|---|
| `Jugador_Situado_Definicion.md` | Sin nada vivo en el documento: las 36 decisiones están en el código y en `Mechanics` (Presencia, Movimiento, Heroe_Y_Tropa); los abiertos están cerrados. El caso «el jugador dentro de una plaza que cambia de manos» lo resuelve el desalojo (ningún héroe queda dentro tras la conquista, `Combate.md` §9). Los bandidos que atacan caravanas ya constan en el canon (1.9, 3.10). | V4 (tope del Tesorero) |
| `Movimiento_Ejercitos_Definicion.md` | Reglas y constantes en `Mechanics/Movimiento_Ejercitos.md`, `Heroe_Y_Tropa.md` y canon 5.12-5.14. Quedan tres ideas que no están en ningún sitio. | V1, V2, V3 |
| `Campamentos_Entrada_Fundacion_Definicion.md` | Cerrado entero (71 decisiones, sin puntos abiertos): canon 1.9b y `Mechanics` (Campamentos_Mercenarios, Fundacion_Y_Refundacion, Presencia). | — |
| `Murallas_Definicion.md` | Vigente en `Mechanics/Murallas.md` y canon 4.2.1; las cifras son placeholder. Quedan dos ideas aplazadas. | V5 |

## Lo que hay que decidir

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

## Quedan por verificar (16)

`Anexion_Y_Desarme`, `Batalla_Este_Lado`, `Comercio_Fisico`, `Entrada_Al_Mundo`, `Identidad_Visual`, `Revamp_Caravanas`,
`Rutas_Caravana_Avanzadas`, `Taberna_Intel`, `Fase_0_5_Especializacion_y_Cupos`, `Fase_0_6_Expansion_Niveles_Asentamiento`,
`Checklist_Mecanicas`, `Economia_Del_Oro`, `Ocupacion_Post_Conquista`, `Niebla`/`Aedas`/`Trazado` (ya borrados) y
`Docs/Mecanicas a desarrollar.md` §8.1, §20, §38, §44-§46 (decididos el 2026-10-13, falta aplicar).
