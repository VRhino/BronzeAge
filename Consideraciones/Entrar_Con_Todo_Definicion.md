# Entrar con todo y defensa desde dentro (definición)

Decidido por el autor el 2026-10-22. Sustituye la «columna aparcada a la puerta» que venía del TypeScript (canon Doc 1 «Entrar y salir», Doc 5 §5.14, `Docs/Mechanics/Presencia_Y_Convocatoria.md`).

## Reglas

1. **«A la puerta» y «dentro».** A la puerta es estar fuera, en el mapa, muy cerca: no defiende. Dentro es haber hecho la interacción de entrar en el elemento (plaza, campamento de mercenarios, pueblo extractor, avanzada o cualquier otro en el que se pueda entrar).
2. **Se entra con todo.** Quien entra en un lugar fuera del mapa entra con todo lo suyo: tropa, carro, víveres y **todas** sus caravanas adjuntas, sean del origen que sean. Al salir se lleva todo. No existe la columna aparcada a la puerta.
3. **Comida dentro.** La tropa de un visitante come de sus propios víveres mientras está dentro, como un ejército acampado; la plaza anfitriona no gasta trigo por sus visitas.
4. **Carro.** Nunca lleva más de su capacidad. Entrar en la residencia **no** vacía el carro: el héroe lo vacía en su almacén personal con una orden, y lo que no cabe se queda en el carro.
5. **Acceso.** El dueño del lugar decide, por relación, quién puede entrar: misma Facción, aliados y vasallos, neutrales, enemigos. Puede negar la entrada. Los pueblos extractores (y las avanzadas) heredan la decisión de su plaza madre. **Por defecto**, hasta el sistema político: entran todos menos los enemigos. La misma Facción siempre puede entrar.
6. **Defensa.** Solo defienden los de la misma Facción y los aliados o vasallos, y solo si están dentro. Neutrales y enemigos no defienden nunca, aunque estén dentro.
7. **Si el lugar cae.** Quien estaba dentro y no podía defender no pierde nada: sale al mapa con lo que tenía (tropa y carro). Quien defendió queda sujeto a lo que le pase en la defensa.

> **Implementado en Rust (2026-10-22, entrar con todo).** Foto `BAN17`, órdenes 117 (`FijarAcceso`) y 118 (`VaciarCarro`), pruebas en `crates/nucleo/tests/entrar.rs`. Detalle, dudas y filas de paridad en [`Docs/Mechanics/Presencia_Y_Convocatoria.md`](../Docs/Mechanics/Presencia_Y_Convocatoria.md) y `Docs/Mecanicas a desarrollar.md`.

## Qué cambia en el código

- Desaparece la columna aparcada a la puerta (`salidas.rs`, `columnas.rs`, `estado_militar.rs`, `convocatorias.rs`, `heroes.rs`, `espacio.rs` y sus tests). La **caravana aparcada** dentro de una plaza (Doc 3.13.7) no cambia: está dentro.
- Entrar ya no rechaza por caravanas adjuntas de otro origen.
- Orden nueva para fijar el acceso por relación (autorización provisional, `manda_en`) y orden de vaciar el carro en el almacén personal, si no existe ya.
- La defensa (4D numérica y 4E Unity) cuenta a los de dentro que sean de la misma Facción, aliados o vasallos; la caída del lugar saca a los demás con lo suyo.
