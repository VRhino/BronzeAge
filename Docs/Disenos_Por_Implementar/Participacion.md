# Participación

> **Diseño acordado el 2026-10-28 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Las cifras de puntos son un borrador a calibrar con los bots. Entrada 54 de `Docs/Mecanicas a desarrollar.md`.

## 1. Qué es

Cada héroe acumula **participación** en cada plaza: un número que dice cuánto ha hecho por esa plaza y por su Facción. Sirve para que los jugadores sepan quién ha contribuido más al crecimiento de la plaza y de la Facción. No se gasta ni da ventajas: es reconocimiento público dentro de la Facción.

Sustituye a la idea de «puntos de asentamiento» que se habló primero.

## 2. Cómo se gana

La participación **se acumula para siempre**: no caduca ni se reinicia.

| Origen | A qué plaza se apunta | Quién la gana |
|---|---|---|
| Cumplir una misión del tablón de una plaza ([`Misiones_Recurrentes_Y_Tablon.md`](Misiones_Recurrentes_Y_Tablon.md)) | esa plaza | el héroe que la cumplió; en una grupal, a partes iguales entre quienes aportaron algo |
| Defender una caravana (escolta que combate) | la plaza de origen de la caravana | cada héroe que combatió en su defensa |
| Defender un pueblo o una aldea | la plaza madre | cada héroe que combatió en su defensa |
| Defender una plaza (asedio) | esa plaza | cada héroe que combatió en su defensa |
| Combatir en una batalla de cualquier tipo y ganarla | la plaza donde reside el héroe | cada héroe del bando ganador que combatió |

Un héroe sin residencia en una plaza (residente de un campamento) no suma por el último origen: no tiene a qué plaza apuntarlo.

## 3. Qué se ve

- **En la plaza** (cliente, ámbito `ASENTAMIENTO`): la lista de sus residentes con su cargo y su participación en esa plaza, de mayor a menor.
- **En la Facción** (cliente, ámbito `FACCION` propio): la lista de sus miembros con su cargo, dónde residen y la **suma** de su participación en todas las plazas de la Facción.

La participación en una plaza se queda con la plaza aunque el héroe se mude o la plaza cambie de dueña; la suma de la Facción solo cuenta las plazas que son de la Facción hoy.

## 4. Por decidir al implementar

- Cuántos puntos da cada origen (borrador: misión individual 10, grupal 20 a repartir, defensa 5, batalla ganada 5).
- Si un héroe que se va de la Facción conserva su participación en las plazas (propuesta: sí, queda en la plaza; solo se deja de sumar en la lista de la Facción).
