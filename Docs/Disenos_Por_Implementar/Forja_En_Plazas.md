# La forja de la Armería en cualquier plaza

> **Diseño acordado el 2026-11-12 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Cambia D1 de la tanda de objetos tal como está hoy en la rama (`Docs/Mechanics/Heroe_Y_Tropa.md` §4.6: se forja solo en la plaza donde se reside, con los materiales del almacén personal). Entrada 77 de `Docs/Mecanicas a desarrollar.md`.

## 1. Qué cambia

| | Hoy (rama de la tanda de objetos) | Con este diseño |
|---|---|---|
| Dónde se forja | En la Armería de la plaza donde reside el héroe, estando dentro | En cualquier plaza con la Armería activa, si su forja está abierta a su grupo (§2) |
| Materiales | Plano y crisoles de la mochila; lingotes, cuero y madera del almacén personal | Plano y crisoles de la mochila; el resto, del almacén personal y del carro en su residencia, y del carro en las demás (§3) |
| Qué tier | El de la tecnología del metal, de la Facción dueña de la plaza | Además, los tiers III a V piden su Era (§4) |
| Coste | Plano, crisoles y materiales | Además, un impuesto en monedas que va a un fondo de la plaza (§5 y §6) |
| Lo que se ve | Dentro, la Armería en la lista de edificios | Dentro, si la forja está abierta para él, cuánto pagaría y qué metales hay; desde fuera, si la plaza tiene Armería (§7) |

La tienda de la Armería no cambia: compra y vende a todo el que está dentro de la plaza, con los metales de la Facción dueña.

## 2. Quién forja en una plaza

Los grupos son los de la puerta (`politica::puerta::grupo_de_puerta`):

- **La misma Facción:** siempre puede forjar.
- **Aliados:** alianza o vasallaje activos, en cualquier sentido.
- **Neutrales:** sin relación, y también quien no tiene Facción. Un mercenario contratado por la Facción de la plaza es neutral.
- **Enemigos:** en guerra.

Para cada uno de los tres grupos de fuera, la plaza guarda si su forja está **abierta** (sí o no) y su **recargo** sobre el impuesto: sin recargo, +100 %, +200 % o +500 %. De partida los tres están cerrados y con +100 %.

- Lo fijan el **Tesorero o el Gobernador** de la plaza, residentes y dentro, con una orden de la plaza.
- Con la forja cerrada a su grupo, forjar se rechaza. Para estar dentro hace falta además que la puerta lo deje entrar (`FijarAcceso`); a los enemigos, por defecto, no los deja.
- **En una conquista** la apertura y los recargos vuelven a los de partida, como las demás decisiones del gobierno derrotado (`Docs/Mechanics/Combate.md`, conquista).

## 3. De dónde salen los materiales

- **El plano y los crisoles:** de la mochila, como hoy.
- **Lingotes, cuero y madera:**
  - En la plaza donde reside: primero de su almacén personal, y lo que falte, del carro (el que trajo o el de la columna que conserva).
  - En cualquier otra plaza: del carro de la columna con la que entró. En el mundo el héroe va siempre en una columna y entra en una plaza ajena con su columna personal; un ejército entra desarmándose, cada héroe con lo suyo (`Guarnecer`).
- Todo o nada, como hoy: si falta algo no se gasta nada.

## 4. Tecnología y Era

Para forjar una pieza hacen falta dos cosas:

- **La tecnología de su metal,** adoptada por la Facción dueña de la plaza (no la del héroe), como hoy.
- **Su Era, para los tiers III a V:** la Era vigente del mundo, que es la misma para todas las Facciones, tiene que ser al menos la del tier. El tier III pide la Era III, el IV la IV y el V la V. El tier II no pide Era.

Una tecnología no aparece antes que su Era, así que el acero ya pide la Era V por sí solo. Con la regla, cada Era abre su tier:

| Era | Armas | Armaduras |
|---|---|---|
| I, Reinos palaciales | II de cobre | II de cobre |
| II, Crisis y adaptación | II de hierro temprano | II de hierro temprano |
| III, Polis e imperios | III de bronce | III de hierro |
| IV, Profesionalización | IV de hierro | IV de bronce |
| V, Hegemonía macedónica | V de acero | V de bronce laminado |

Sin la regla, la armadura IV (bronce) se forjaría desde la Era I, antes que la III (hierro, Era III), y la armadura V (bronce laminado, Era III) dos Eras antes que el arma V.

- **Llevar puesta** una pieza se puede siempre (Objetos_Del_Heroe §2.1).
- **El botín** no da piezas hechas, solo planos, crisoles, medallas y el cofre de monturas: un plano de tier V puede salir en la Era I y guardarse hasta la V.
- **Las monturas** del cofre (tiers III, IV y V) no tienen metal y no piden Era.

## 5. El impuesto de la forja

- Se paga en **monedas** del héroe por cada forja aceptada; si la forja se rechaza no se paga nada. Sin monedas suficientes, la forja se rechaza.
- **Base** por tier de la pieza: II 100, III 175, IV 250 y V 350 monedas (en la proporción de los precios del catálogo).
- **La misma Facción paga la base,** también los residentes de la plaza.
- **Cada grupo de fuera** paga la base más su recargo: sin recargo, ×2 (+100 %), ×3 (+200 %) o ×6 (+500 %).

## 6. El fondo de la plaza

- Cada plaza tiene un **fondo de monedas** donde entra el impuesto de su forja.
- **Se gasta en contratos de mercenarios** firmados desde la Taberna de la plaza, por quien hoy ya puede pagar con su almacén (`pagar_con = almacenPlaza`, Mercenarios_Por_Contrato §5.1): el Rey, el Embajador o el Gobernador, dentro. Paga la tarifa en monedas, y las monedas pasan a las del mercenario, como con `pagar_con = monedas`. Como el almacén de la plaza, no paga en el campamento ni al prorrogar.
- Los firmantes no cambian: el Tesorero fija la apertura y los recargos de la forja, pero no firma contratos.
- **Una plaza sin Taberna** acumula el fondo, pero no lo puede gastar hasta que la tenga.
- **En una conquista** el fondo pasa a la Facción conquistadora con la plaza.

## 7. Lo que se ve

- **Dentro de la plaza:** si su forja está abierta para él, cuánto pagaría por tier (base y recargo de su grupo) y los metales que puede forjar allí (tecnología de la Facción dueña y Era).
- **Desde fuera:** la ficha de la plaza dice si tiene una Armería activa, como ya dice si tiene Mercado (cambia la decisión del 2026-11-07, que solo dejaba ver el Mercado y la muralla).
- **El fondo** lo ve quien está dentro de la plaza.

## 8. Lo que toca

- **Motor:**
  - en la plaza, la configuración de la forja por grupo y el fondo (estado y foto);
  - una orden para fijar la configuración (Tesorero o Gobernador);
  - `forjar_objeto` en cualquier plaza con su grupo, la Era, los materiales y el impuesto; el rechazo `forja.fuera_de_tu_residencia` desaparece;
  - el pago `fondoPlaza` en los contratos;
  - la conquista reinicia la configuración y pasa el fondo;
  - los campos nuevos de la plaza (dentro y desde fuera).
- **Datos:** la base por tier y los recargos posibles en el bloque `OBJETOS`.
- **Cliente jugador:** el panel de la forja para el Tesorero y el Gobernador, lo que ve el héroe dentro y fuera, y el pago con el fondo (`Docs/Coordinacion/Pendientes_Cliente_Jugador.md`).
- **BotsBrain:** sus héroes pagan el impuesto y forjan fuera con su carro; sus Tesoreros y Gobernadores pueden configurar la forja.
