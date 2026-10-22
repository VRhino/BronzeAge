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

## Segunda ronda de decisiones del autor (2026-10-22)

Cuatro reglas que completan las anteriores. Las dudas de la primera ronda que resuelven («el dueño cierra la puerta y los de dentro siguen», «un enemigo o un neutral dentro puede unirse al ataque», «el residente derrotado sale con el carro entero») quedan cerradas.

1. **Se puede entrar en un pueblo extractor y en una avanzada, como en una plaza.** Hay órdenes de entrar y de salir de cada uno (`EntrarEnPueblo` 119, `SalirDePueblo` 120, `EntrarEnAvanzada` 121, `SalirDeAvanzada` 122, a continuación de la 118). El acceso se hereda de la plaza madre (`puede_entrar_en_pueblo`, `puede_entrar_en_avanzada`) y rige la regla de entrar con todo: la columna entera queda dentro, fuera del mapa, y sale con él. Dentro se cuenta para la defensa con la regla de relación (misma Facción, aliados y vasallos; neutrales y enemigos nunca), además de la guarnición. Si el lugar desaparece o cambia de dueño (una avanzada destruida o en ruinas, la madre en ruinas o conquistada, un pueblo destruido a mano) quien estaba dentro sale al mapa con lo suyo y sin perder nada; quien defendió queda sujeto a lo que le pase en la defensa.
2. **Cerrar la puerta expulsa.** Si el dueño (`FijarAcceso`) cierra a una relación, quienes ya están dentro con esa relación salen al mapa al instante con todo lo suyo: en la plaza y en sus pueblos y avanzadas, que heredan su acceso. La misma Facción y los residentes no se expulsan nunca.
3. **Al ataque solo se une quien está fuera (en el mapa); a la defensa, solo quien está dentro.** Vale para el asedio, el saqueo de un pueblo y el ataque a una avanzada, en `combate::union` (`lado_para_unirse`), en `unirse_a_batalla`, en la apertura de las batallas de Unity y en la resolución con números. Un héroe de la Facción atacante que está dentro (entró como neutral, por ejemplo) no puede unirse al ataque desde dentro. Quien está dentro de otro lugar no se une a ninguno de los dos bandos.
4. **El residente que defiende y pierde.** Si la derrota implica **conquista**, se aplican las consecuencias del canon (Doc 5 §5.4 y §5.15.5, `Docs/Mechanics/Combate.md` §9): cada héroe defensor queda fuera con las escuadras que usó y sobrevivieron; el resto de las escuadras de los residentes, guarnición incluida, quedan a 0 y se van con su héroe a la plaza más cercana de su Facción, donde pasa a residir (sin ninguna, al campamento de mercenarios más cercano); pierden la residencia y los cargos locales; no hay captura de guarnición. El canon no fija qué pasa con el carro del residente que defendió y perdió, así que se aplica la regla del derrotado: **pierde la mitad, que va al botín del vencedor**. Si la derrota **no implica conquista** (el saqueo de un pueblo, el ataque a una avanzada que cae o queda en ruinas), el residente defensor pierde igualmente la mitad del carro.

> **Implementado en Rust (2026-10-22, segunda ronda).** Foto `BAN18` (ubicaciones `Pueblo` y `Avanzada` del héroe), órdenes 119-122, hecho `heroe.expulsado`, módulo `crates/nucleo/src/nucleo/dentro.rs`; pruebas en `crates/nucleo/tests/{entrar,pueblos,avanzadas,combate,batallas}.rs`. Detalle, dudas y filas de paridad en [`Docs/Mechanics/Presencia_Y_Convocatoria.md`](../Docs/Mechanics/Presencia_Y_Convocatoria.md), [`Docs/Mechanics/Combate.md`](../Docs/Mechanics/Combate.md) y `Docs/Mecanicas a desarrollar.md`.
>
> Dudas que quedan (marcadas `// DUDA:` en el código): el visitante amigo que defendió y perdió **conserva** su carro (el canon le deja salir con la columna con la que entró, sin perder nada); «avanzada que resiste» se ha leído como «avanzada cuyo ataque gana el atacante» (si el defensor gana no hay derrota que castigar); y al conquistar la madre salen del mapa todos los que estaban dentro de sus pueblos y avanzadas, sea cual sea su relación con el conquistador.
