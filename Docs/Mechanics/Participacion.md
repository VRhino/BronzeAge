# Participación

> **Implementado en Rust el 2026-10-28** (mecánica 54; diseño acordado con el autor el mismo día). Las cifras son el borrador del diseño, a calibrar con los bots. Código: `crates/nucleo/src/participacion.rs` (reglas y foto) y `crates/nucleo/src/nucleo/participacion.rs` (de dónde salen los puntos y la proyección). El origen «misión del tablón» llega con la mecánica 55 (`Docs/Disenos_Por_Implementar/Misiones_Recurrentes_Y_Tablon.md`).

## 1. Qué es

Cada héroe acumula **participación** en cada plaza: un número que dice cuánto ha hecho por esa plaza y por su Facción. Sirve para que los jugadores sepan quién ha contribuido más al crecimiento de la plaza y de la Facción. No se gasta ni da ventajas: es reconocimiento público dentro de la Facción.

## 2. Cómo se gana

La participación **se acumula para siempre**: no caduca ni se reinicia. Se suma **en origen**, al resolverse el combate (con números) o al aplicarse el resultado de Unity, como los avances de las misiones (`Misiones.md` §3).

| Origen | A qué plaza se apunta | Quién la gana | Puntos |
|---|---|---|---|
| Defender una plaza en un asedio con combate | esa plaza | cada héroe del bando defensor que combatió | `defensa` |
| Defender un pueblo o una aldea en un saqueo con combate | la plaza madre | cada héroe que lo defendía dentro y combatió | `defensa` |
| Defender una caravana comercial: de un ejército que la asalta (su escolta cedida, con números; el bando defensor, en Unity) o de los bandidos que la acosan (su escolta cedida o, si va adjunta, los héroes de la columna que la lleva) | la plaza de origen de la caravana | cada héroe que la defendió | `defensa` |
| Ganar una batalla de cualquier tipo: entre columnas, asedio, saqueo de pueblo, ataque a una avanzada, asalto a una caravana, campamento de bandidos destruido (con números o en Unity) | la plaza donde reside el héroe | cada héroe del bando ganador que combatió | `batallaGanada` |
| Cumplir una misión del tablón (mecánica 55, aún no) | esa plaza | — | — |

- Un héroe sin residencia en una plaza (residente de un campamento) no suma por la batalla ganada: no tiene a qué plaza apuntarlo.
- **Defender y ganar suman los dos** (decisión de implementación del 2026-10-28): quien defiende su plaza y gana se lleva `defensa` en esa plaza y `batallaGanada` en la suya.
- El acoso de los bandidos no es una batalla: da `defensa`, nunca `batallaGanada` (como en las misiones, donde no cuenta como batalla).
- Un asedio, un saqueo o un ataque a una avanzada **sin combate** (cae sin defensores) no da nada a nadie.
- La caravana que defienden los carreteros sola (sin escolta ni columna) no da nada: no hay héroe.

<!-- parametros:PARTICIPACION -->
```json
{
  "defensa": 5,
  "batallaGanada": 5
}
```
<!-- /parametros -->

## 3. Qué se ve

- **En la plaza** (ámbito `ASENTAMIENTO`, el interior): `Asentamiento.participacion`, la participación de cada héroe en esa plaza (por id de héroe), residente o no. El cliente la cruza con `casas_compradas` (los residentes) y `cargos` para la lista de residentes con su cargo y su participación, de mayor a menor.
- **En la Facción** (ámbito `FACCION` propio): `Faccion.miembros`, uno por ciudadano, con dónde reside (`residencia_id`, la plaza o el campamento), sus cargos (`rey`, `embajador` y los locales como `gobernador@as-3`) y la **suma** de su participación en las plazas que son de la Facción hoy. La forma pública de una Facción ajena no la lleva.

La participación en una plaza se queda con la plaza aunque el héroe se mude, deje la Facción (decisión de implementación del 2026-10-28: queda en la plaza; solo deja de sumar en la lista de la Facción) o la plaza cambie de dueña. Una plaza en ruinas se lleva la suya.

## 4. Foto

Desde **BAN49**, al final del estado: `n u64 · n × (plaza u64 · m u64 · m × (héroe u64 · puntos u64))`. Las fotos BAN40 a BAN48 se leen con la participación vacía.
