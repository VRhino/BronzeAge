# Informes de taberna por niveles

> **Diseño acordado el 2026-10-31 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Precios y enfriamientos son placeholders por calibrar; no hay bloque `parametros` hasta implementarlo. Entrada 65 de `Docs/Mecanicas a desarrollar.md`.

## 1. Qué es

El Informe de plaza se separa en varios productos que se compran por separado, cada uno con su precio y su enfriamiento. La profundidad depende del nivel de la **taberna desde la que se compra**, no de la plaza espiada: quien invierte en intel sabe más.

Hay dos familias:

- **De plaza:** datos del motor, contra cualquier Facción.
- **De bots:** datos del libro de agravios, que vive en el servicio de bots. Solo contra Facciones de bots. El motor los vende y no conoce su contenido.

Casi todo existe ya y se aprovecha:

| Pieza | Dónde | Qué aporta | Qué le falta |
|---|---|---|---|
| `comprar_informe_plaza` | `crates/nucleo/src/nucleo/taberna.rs`, Niebla_De_Guerra §6.2 | Compra, precio, enfriamiento y quién puede comprar | El tipo de informe |
| `Informe` en la memoria de la Facción | Niebla_De_Guerra §6.2 | Edificios, recintos, guarnición y héroes, con su fecha | Separarlo por tipo; residentes como cantidad; tecnologías y rutas |
| `asentamiento.informe_pedido` | `taberna.rs` | Avisa a la plaza espiada, sin decir quién | El tipo de informe |
| Niveles de la taberna | `EDIFICIO_CATALOGO.taberna` | La 2 exige plaza de nivel 2 y la 3, plaza de nivel 3 | Nada |
| Agrupación de avisos | `BronzeAgeClient/src/ui/avisos.ts` | Ya agrupa por tipo y plaza | Nada |

## 2. Catálogo

La Mirada no cambia (Niebla_De_Guerra §6.1).

**De plaza.** Objetivo: una plaza ajena que la Facción conozca.

| Informe | Revela | Taberna |
|---|---|---|
| Población | Cantidad de residentes | 1 |
| Guarnición | Tropas guarnicionadas | 2 |
| Tecnologías | Tecnologías aplicadas por su Facción | 2 |
| Trazado | Edificios y murallas | 3 |
| Rutas | Rutas comerciales activas | 3 |

**De bots.** Objetivo: una Facción de bots de la que la Facción conozca alguna plaza. Es la Facción y no una plaza porque esos datos son de toda ella.

| Informe | Revela | Taberna |
|---|---|---|
| Cuentas | Número exacto y tendencia de sus relaciones | 1 |
| Agravios | Lo que anotó en su libro y contra quién | 2 |
| Corte | Rasgos y debilidad de su Rey | 3 |
| Planes | Objetivo actual y enemigo jurado | 3 |

Basta el nivel de la taberna: los requisitos de la Taberna 2 y 3 ya cubren el nivel de la plaza donde está. La taberna de un campamento no tiene nivel y cuenta como Taberna 1.

Quién compra no cambia: el Rey, el Embajador o el Gobernador presente en la plaza, que paga de su almacén, o cualquier héroe con Facción presente en el campamento.

Todos los informes siguen siendo **exactos en el instante de la compra**, sin ruido ni datos falsos, y envejecen con su fecha.

## 3. Avisos

Se mantiene la regla actual: todo informe avisa al objetivo, sin decir quién; la Mirada no avisa a nadie. El cliente ya agrupa estos avisos.

El aviso necesita decir el **tipo** de informe, para que un bot sepa si reaccionar. Los bots solo reaccionan a los de Taberna 3:

| Informe | Reacción del bot |
|---|---|
| Trazado | Sube la alerta de su Facción |
| Rutas | Sube la alerta y escolta sus caravanas |
| Corte | Sube la alerta |
| Planes | Sube la alerta y puede cambiar de objetivo |

Esto vive en el servicio de bots. La alerta es un solo dato por Facción, «alerta hasta», que se sobrescribe y no entra en el libro: el objetivo no sabe a quién culpar y el espionaje no ocupa más memoria aunque sea constante.

## 4. Qué cambia en el motor

- **Orden `comprar_informe`** `{ origen, tipo, objetivo }`, que sustituye a `comprar_informe_plaza`. Rechazos nuevos, entre ellos taberna de nivel insuficiente y objetivo que no es una Facción de bots.
- **Catálogo en `INTEL`**: nivel requerido, precio y enfriamiento por tipo. El enfriamiento va por Facción compradora, tipo y objetivo.
- **Informes de plaza por tipo** en la memoria de la Facción, cada uno con su fecha. Son nuevos los residentes como cantidad, las tecnologías y las rutas.
- **Facción de bots**: la que tiene un Rey bot.
- **Recibo firmado** en los informes de bots: tipo, objetivo, Facción compradora e instante, firmados con un secreto compartido con el servicio de bots.
- **Aviso con el tipo de informe**, también para los dirigidos a una Facción.

## 5. Lo que hacen el cliente y el servicio de bots

- **Cliente:** elige el informe según su taberna, enseña los de plaza como hoy y presenta el recibo al servicio de bots para leer los de bots.
- **Servicio de bots:** comprueba el recibo y responde con el contenido. Si está parado, el recibo sigue valiendo cuando vuelva.
- **Bots:** compran Miradas e informes de plaza con un presupuesto por Facción; no compran informes de bots.

## 6. Por calibrar

- Precio y enfriamiento de cada tipo.
- Cuánto guarda cada Facción: hasta cinco informes por plaza conocida.
