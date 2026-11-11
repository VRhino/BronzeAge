# Las Eras IV y V en el motor

Diseño del autor del 2026-11-11, sin implementar. Lo pidió a las 23:05 («agrega la tecnología en la Era 5 con todo lo que necesita, agrega todas las unidades de estas Eras que están en diseño al roster real con sus respectivas tecnologías») y cerró las dudas a las 23:14. Sale de `Consideraciones/BA-006_Revision_Tecnologia_Eras.md` (Era IV, decisiones completas del 2026-09-28; Era V, en curso desde ese día), del canon (Doc 6.2 y 6.5) y de [`Cifras_De_Combate.md`](Cifras_De_Combate.md) (las tropas).

**Hoy el motor solo tiene las Eras I a III:** 26 tecnologías, 27 tropas y 24 contadores de logros. De las Eras IV y V no tiene nada.

## 1. Las Eras

| Era | Id | Nombre | Semanas | Plazo | Mínimo | Tarifa de adopción |
| --- | --- | --- | --- | ---: | ---: | --- |
| IV | `profesionalizacion_imperial` | Profesionalización | 18-26 | 8 semanas | 1 semana | 1.000 de oro y 50 lingotes de hierro |
| V | `hegemonia_macedonica` | Hegemonía macedónica | 26-52 | 26 semanas | 1 semana | 1.500 de oro y 80 lingotes de hierro |

Funcionan como las tres primeras (Doc 6.2): la siguiente empieza cuando se cumplen todos los logros de la anterior o se agota su plazo, y ninguna tecnología se adopta antes de que empiece su Era. Los Aedas siguen sin vender tecnologías de estas Eras (`AEDAS.venta.ordenEraMaximo` = 3), pero sí las cantan en épicas (§5).

## 2. Las tecnologías

Doce, seis en cada Era. **Los umbrales son provisionales** (autor, 23:14): puestos al ritmo de las Eras II y III y, como los demás, se miden con bots en la fase 7. `porPlaza` es por plaza viva del mundo (Tecnologia_Aedas §2).

### Era IV — Profesionalización

| Tecnología | Abre | Logro del servidor | Hito de la Facción |
| --- | --- | --- | --- |
| `ejercito_permanente` · Ejército permanente | +10 % de experiencia a las escuadras de los héroes de la Facción | `escuadras.veteranas`, 2 por plaza | Barracón 3 y 10 escuadras propias vivas a la vez |
| `cuerpo_oficiales` · Cuerpo de oficiales | nada propio; es una de las piezas de la Reforma macedónica | `batallas.conTresHeroes`, 1 por plaza | 5 héroes en la Facción |
| `logistica_campana` · Logística de campaña | los víveres de los ejércitos de la Facción ×1,5 | `caminos.compartidos`, 1 | una plaza de nivel 4, la Facción en nivel 5 y 3 columnas en marcha a la vez |
| `infanteria_profesional` · Infantería profesional | epílektoi e Inmortales | `batallas.ganadasEnInferioridad`, 1 por plaza | Ejército permanente y Barracón 3 en una plaza de nivel 4 |
| `hostigadores_profesionales` · Hostigadores profesionales | peltastas de Ifícrates | `escuadras.eliteDestruidas`, 2 por plaza | Pantalla de escaramuzadores y Galería de tiro 3 |
| `carga_caballeria` · Carga de caballería | caballería tesalia y caballería noble persa | `batallas.campoAbiertoConCaballeria`, 1 por plaza | Caballería organizada y Caballerizas 3 |

**Cuerpo de oficiales no da Liderazgo.** BA-006 (EIV-1) le daba +10 %, pero el autor lo quitó el 2026-11-11 a las 23:14: «el Liderazgo no se muta desde tecnologías».

**Los víveres ×1,5** (confirmado por el autor el 2026-11-11 a las 23:28): BA-006 dice «carro de suministros de los ejércitos ×1,5 (Doc 5.13)», y en Doc 5.13 el suministro son los víveres, no el carro de carga.

### Era V — Hegemonía macedónica

| Tecnología | Abre | Logro del servidor | Hito de la Facción |
| --- | --- | --- | --- |
| `reforma_macedonica` · Reforma macedónica | agrianos | `batallas.armasCombinadas`, 1 por plaza | adoptadas Forja del hierro estandarizada, Disciplina de formación, Infantería profesional, Caballería organizada, Logística de campaña y Cuerpo de oficiales; Barracón 3, Armería 3, Caballerizas 3 y Palacio 3 |
| `falange_sarisa` · Falange de sarisas | falangitas | `batallas.contraCoalicion`, 3 | Reforma macedónica y Barracón 2 en una plaza de nivel 4 |
| `cuerpo_hipaspistas` · Cuerpo de hipaspistas | hipaspistas | `batallas.conRey`, 10 | Falange de sarisas, Barracón 3 y Palacio 3 |
| `formacion_cuna` · Formación en cuña | pródromos | `persecuciones.deCaballeria`, 2 por plaza | Reforma macedónica y Caballerizas 2 |
| `companeros_reales` · Compañeros reales | Compañeros | `batallas.campoAbiertoConCaballeriaElite`, 1 por plaza | Formación en cuña, Caballerizas 3 y Palacio 3 |
| `acero_templado` · Acero templado | forjar las armas de acero del héroe (tier V, [`Catalogo_De_Objetos.md`](Catalogo_De_Objetos.md) §2) | `fabricado.armaHierroCalidad`, 20 por plaza | Fundición 3 y Armería 3 |

- **El logro de Compañeros reales** estaba sin decidir (EV-7). Lo cerró el autor a las 23:14 con la propuesta: batallas a campo abierto ganadas con caballería de escalón 4 o 5.
- **El acero** no tenía logro ni hito. Los fijó el autor a las 23:14. **En el motor no abre nada** hasta que existan los objetos del héroe (entrada 70): entonces pide el acero para forjar las armas de tier V.

### Fuera por ahora

- **Maestría de asedio, Maestros de obras militares y la tecnología de la catapulta de torsión** (EIV-3, EIV-4 y EV-4) esperan a que el motor tenga equipo de asedio (autor, 23:14). Hoy la Carpintería solo fabrica el carro de guerra. Ninguna de las tres haría nada, y el logro de Maestros de obras («piezas de equipo de asedio fabricadas») no se podría cumplir.
- **El Batallón Sagrado** se quitó del todo (autor, 2026-11-11 20:34, `Cifras_De_Combate.md` §1).

## 2b. La subida al nivel 5

Hoy el motor no deja subir una plaza del nivel 4 al 5 (`ASCENSO_ASENTAMIENTO.nivelTechoProvisional` = 4): BA-006 (D54, EV-2 y EV-3) quería que la abriera una tecnología cívica de la Era V con un edificio neutro nuevo, y ninguno de los dos se diseñó.

**El autor la cerró el 2026-11-11 a las 23:20 con bienes, sin tecnología ni edificio nuevo:** a lo que ya cuesta la obra (5.000 de madera, 5.000 de piedra y 2.500 de oro) se suman **1.000 de vino, 1.000 de cerveza, 1.000 de aceite de oliva y 1.000 de aceite de sésamo**. Los requisitos de la plaza siguen como están (4.000 pesants, 3.200 artesanos y el Palacio 3). Se quita el techo provisional del nivel 4.

- **Esto pide comerciar o tener más de una plaza.** En los niveles 3 y 4 una plaza hace una sola bebida (todas sus Bodegas, vino o cerveza; Doc 4.2.1) y la Prensa hace siempre un solo aceite, así que la plaza que sube trae de otra plaza o del mercado la otra bebida y el otro aceite.
- **El nivel 5 ya no depende de ninguna Era**: llega cuando una plaza junta los bienes. La vid y la cebada piden una plaza de la Facción en el nivel 3, y la oliva y el sésamo una en el 4.
- **El aceite pasa a ser dos bienes** (autor, 2026-11-11 23:24, en la tarjeta: «1.000 de cada uno, en dos bienes»). Hoy la Prensa saca el mismo `aceite` de la aceituna o del sésamo. Queda así:
  - `aceite` sigue siendo el id del aceite de oliva, que se llama «Aceite de oliva», para no tocar a los bots ni a los clientes que ya lo leen;
  - nuevo `aceiteSesamo` · «Aceite de sésamo», el recurso 42: nace en el almacén como los demás (capacidad 800 más los Almacenes) y entra en la foto, el flujo y la tendencia;
  - la receta de sésamo de la Prensa produce `aceiteSesamo` (2 de sésamo por unidad, como hoy); la de aceituna sigue con `aceite`. La Prensa sigue con una receta por plaza (`TALLERES_DE_UNA_RECETA`);
  - en `BIENES_DE_CONSUMO`, el aceite de sésamo como la cerveza: lo come la nobleza, 0,01 por cabeza y minuto, con ×1,25 (el de oliva sigue con ×1,5). Una plaza con los dos le da a la nobleza los dos modificadores, como el vino y la cerveza (§9);
  - `pausar_consumo` lo admite, como los demás bienes de consumo;
  - se vende y se compra en el mercado y viaja en caravanas como cualquier recurso, sin reglas propias.
- **Sobran** la tecnología cívica de la Era V y su edificio (EV-2), y EV-3, el contenido de nivel 5, sigue como está (Palacio 3, Maravilla).

## 3. Contadores nuevos

Doce, de 24 a 36. Los de batalla se cuentan en las mismas batallas que `batallas.libradas`: el encuentro de dos columnas con números y toda batalla de Unity, en cualquier contexto. «Campo abierto» es el encuentro con números y la batalla de Unity de campo abierto.

| Contador | Suma 1 cuando… |
| --- | --- |
| `escuadras.veteranas` | una escuadra llega al nivel 5 (una vez por escuadra, al cruzarlo) |
| `batallas.conTresHeroes` | en una batalla, algún bando lleva 3 héroes o más |
| `caminos.compartidos` | se abre el primer camino compartido: un tramo fuera de toda zona recorrido por 10 rutas o más de 3 Facciones o más (BA-006 D30; ya lo mira el motor, `camino_compartido_abierto`). Es un récord: 0 o 1 |
| `batallas.ganadasEnInferioridad` | gana el bando que empezó con menos soldados |
| `escuadras.eliteDestruidas` | por cada escuadra de escalón 4 o 5, de cualquier bando, que acaba la batalla sin soldados |
| `batallas.campoAbiertoConCaballeria` | gana a campo abierto un bando con alguna escuadra de Caballerizas |
| `batallas.armasCombinadas` | algún bando lleva a la vez escuadras de Barracón, de Galería de tiro y de Caballerizas (gane o pierda) |
| `batallas.contraCoalicion` | gana un bando de una sola Facción a uno con héroes de 2 Facciones o más |
| `batallas.conRey` | combate el Rey de alguna Facción |
| `persecuciones.deCaballeria` | una columna con caballería ataca a otra columna o a una caravana, con números o en Unity (gane o pierda) |
| `batallas.campoAbiertoConCaballeriaElite` | gana a campo abierto un bando con caballería de escalón 4 o 5 |
| `fabricado.armaHierroCalidad` | se fabrica un arma de hierro de calidad (un flujo, como los demás `fabricado.*`) |

## 4. Hitos nuevos

| Condición | Se cumple si… |
| --- | --- |
| `escuadrasVivas` (cantidad) | los héroes de la Facción tienen a la vez esas escuadras con soldados |
| `heroesEnFaccion` (cantidad) | la Facción tiene esos héroes |
| `plazaEnNivel` (nivel) | alguna plaza de la Facción tiene ese nivel actual |
| `nivelFaccion` (nivel) | la Facción tiene ese nivel |
| `columnasEnMarcha` (cantidad) | tiene a la vez esas columnas o ejércitos moviéndose por el mapa |
| `edificioEnPlazaDeNivel` (edificio, nivel interno, nivel de la plaza) | ese edificio activo, con ese nivel, en una plaza de ese nivel |

Las tres de cantidades que cambian sin ningún disparador de la aparición (escuadras vivas, héroes y columnas en marcha), y el nivel de la Facción, se miran en el paso de minuto de la capital, como hoy `recursoEnCapital` (Tecnologia_Aedas §3), y solo si alguna candidata las pide. Las de plaza y edificio usan los disparadores de siempre.

## 5. Épicas

Una épica es la otra vía para que una tecnología aparezca a una Facción, además del hito (Tecnologia_Aedas §7.2). La empieza un Aeda residente cuando el logro del servidor ya se cumplió, la Era ha empezado y a la Facción aún no le ha aparecido; avanza con hechos de la Facción o de la plaza del Aeda (una obra, una batalla ganada, una defensa, una conquista, una subida de nivel, una adopción, una caravana comercial de 100 o más), uno cada 6 horas como mucho. Al cerrar el último capítulo, la tecnología aparece sin hito. Sirve a la Facción que no llega al hito, y no le ahorra la tarifa.

Toda tecnología con logro tiene su épica (lo exige una prueba del motor), con 3 capítulos en la Era I, 4 en la II y 5 en la III (Doc 6.3). **Propuesta, sin ver por el autor:** 6 capítulos en la Era IV y 7 en la V, con los mismos tipos de hecho. Cada capítulo pide una cantidad de hechos de un tipo; «obra» pide además un edificio. Lo mínimo son los hechos menos uno, por 6 horas; las de la Era III llevan entre 7 y 10 hechos (36 a 54 horas).

| Tecnología | Épica | Capítulos | Hechos | Mínimo |
| --- | --- | --- | ---: | ---: |
| Ejército permanente | Canto de los que nunca vuelven al arado | obra de Barracón · 2 batallas · 2 defensas · 1 adopción · 2 batallas · 1 subida de nivel | 9 | 48 h |
| Cuerpo de oficiales | Canto de los capitanes | 2 batallas · 1 adopción · obra de Barracón · 2 batallas · 1 defensa · 1 conquista | 8 | 42 h |
| Logística de campaña | Canto de los carros de víveres | 3 caravanas · obra de Mercado · 3 caravanas · 1 batalla · 1 subida de nivel · 2 caravanas | 11 | 60 h |
| Infantería profesional | Canto de Leuctra | obra de Barracón · 3 batallas · 2 defensas · 1 adopción · 2 batallas · 1 conquista | 10 | 54 h |
| Hostigadores profesionales | Canto de Lequeo | obra de Galería de tiro · 3 batallas · 1 defensa · 1 adopción · 2 batallas · 2 caravanas | 10 | 54 h |
| Carga de caballería | Canto de los jinetes de Tesalia | obra de Caballerizas · 2 batallas · 2 caravanas · 1 adopción · 3 batallas · 1 conquista | 10 | 54 h |
| Reforma macedónica | Canto de Filipo | 2 adopciones · obra de Barracón · 3 batallas · obra de Caballerizas · 1 conquista · 1 subida de nivel · 3 batallas | 12 | 66 h |
| Falange de sarisas | Canto de Queronea | obra de Barracón · 3 batallas · 2 defensas · 1 adopción · 3 batallas · 1 conquista · 2 defensas | 13 | 72 h |
| Cuerpo de hipaspistas | Canto de los portadores de escudo | obra de Palacio · 2 defensas · 3 batallas · 1 adopción · obra de Barracón · 2 batallas · 1 conquista | 11 | 60 h |
| Formación en cuña | Canto de los que corren delante | obra de Caballerizas · 3 caravanas · 2 batallas · 1 adopción · 2 caravanas · 2 batallas · 1 conquista | 12 | 66 h |
| Compañeros reales | Canto de Gaugamela | obra de Caballerizas · 3 batallas · 1 adopción · obra de Palacio · 3 batallas · 2 conquistas · 1 subida de nivel | 12 | 66 h |
| Acero templado | Canto del hierro templado | obra de Fundición · 3 caravanas · obra de Armería · 1 adopción · 2 batallas · 1 subida de nivel · 2 caravanas | 11 | 60 h |

- **Ocho de las doce piden una conquista**, como cinco de las nueve de la Era III: una Facción que no conquista nunca solo llega a ellas por hito. Las que no la piden son Ejército permanente, Logística de campaña, Hostigadores profesionales y Acero templado.
- **El nombre de la de Acero templado se parece al de Forja de hierro estandarizada** (Era III, «Canto del acero templado»); se puede cambiar si confunde.

## 6. Las tropas

Diez, con los datos que el autor cerró en la Armería (vista «Tropas y héroe», `Cifras_De_Combate.md` §1, §4 y §7): tier, escalón, soldados y equipo por soldado. En `TROPAS_RECLUTABLES` faltaban dos cifras:
- **la velocidad**, la propuesta del 2026-11-11 que el autor aceptó (23:14);
- **el poder para el combate con números**, el de las tropas de hoy con su mismo tier y escalón. Por ejemplo, las de tier V y escalón 5 llevan 17, como los hequetai y los arqueros escitas.

| Tropa | Id | Era | Tecnología | Edificio | Tier | Escalón | Población | Soldados | Equipo por soldado | Poder | Velocidad |
| --- | --- | --- | --- | --- | --- | ---: | --- | ---: | --- | ---: | ---: |
| Epílektoi | `epilektoi` | IV | Infantería profesional | Barracón 2 | IV | 3 | artesanos | 18 | arma de hierro de calidad, armadura de bronce | 10 | 24 |
| Inmortales | `inmortales` | IV | Infantería profesional | Barracón 3 | IV | 5 | nobleza | 15 | arma de hierro de calidad, arma de bronce (arco), armadura intermedia | 15 | 24 |
| Peltastas de Ifícrates | `peltastas_ificrates` | IV | Hostigadores profesionales | Galería de tiro 3 | IV | 4 | nobleza | 20 | 2 armas de hierro de calidad, armadura básica | 14 | 30 |
| Caballería tesalia | `caballeria_tesalia` | IV | Carga de caballería | Caballerizas 3 | IV | 4 | nobleza | 15 | arma de hierro de calidad, armadura intermedia, 1 caballo | 14 | 84 |
| Caballería noble persa | `caballeria_noble_persa` | IV | Carga de caballería | Caballerizas 3 | V | 4 | nobleza | 16 | 2 armas de hierro, armadura de bronce, 1 caballo | 17 | 84 |
| Pródromos | `prodromos` | V | Formación en cuña | Caballerizas 2 | III | 3 | artesanos | 18 | arma de hierro, 1 caballo | 9 | 84 |
| Falangitas | `falangitas` | V | Falange de sarisas | Barracón 2 | IV | 4 | **artesanos** | 32 | arma de hierro de calidad (sarisa), armadura intermedia | 14 | 18 |
| Hipaspistas | `hipaspistas` | V | Cuerpo de hipaspistas | Barracón 3 | V | 5 | nobleza | 20 | arma de hierro de calidad, armadura de bronce de calidad | 17 | 24 |
| Compañeros | `companeros` | V | Compañeros reales | Caballerizas 3 | V | 5 | nobleza | 16 | arma de hierro de calidad, armadura de bronce de calidad, 1 caballo | 17 | 84 |
| Agrianos | `agrianos` | V | Reforma macedónica | Galería de tiro 3 | V | 5 | nobleza | 20 | 2 armas de hierro de calidad, armadura básica | 17 | 30 |

- **Las falangitas salen de los artesanos** aunque son de escalón 4: es la excepción de BA-006. `TROPAS_RECLUTABLES` gana un campo `poblacion` para la tropa que no sigue a su escalón.
- **Los soldados** son los de `Cifras_De_Combate.md` §4, así que no siguen la regla «cuanto más de élite, menos cuerpos». Se quita la prueba `el_tamano_de_la_escuadra_baja_con_el_escalon`, como ya decidió el autor el 2026-11-11 a las 20:15.
- **Las 27 tropas de hoy pasan también a sus soldados de `Cifras_De_Combate.md` §4 en la misma tanda** (autor, 23:28), para que las de élite nuevas no queden con más cuerpos que las viejas (los hequetai pasan de 12 a 18).
- **Sus cifras de combate** (vida, daño, defensa…) son las de `Cifras_De_Combate.md` §7 y llegan con la entrada 72.

## 7. Lo que toca en el motor

- **Datos** (`Docs/Mechanics`, al implementar): `ERAS` y `TARIFA_ADOPCION` con las dos Eras; las 12 tecnologías en `TECNOLOGIAS` y sus épicas en `EPICAS`; las 10 tropas en `TROPAS_RECLUTABLES` y los soldados de Cifras §4 en las 27 de hoy (§6); los bienes de la obra del nivel 5 en `ASCENSO_ASENTAMIENTO` y fuera `nivelTechoProvisional`; la receta de sésamo de la Prensa con `aceiteSesamo` y su entrada en `BIENES_DE_CONSUMO` (§2b).
- **`asentamiento`:** el recurso `aceiteSesamo` (de 41 a 42 recursos en `recurso.rs`), que cambia la foto (un BAN nuevo), y las pruebas de recetas y de consumo.
- **`militar`:** las 10 tropas en el enum, el campo `poblacion` y quitar la prueba del tamaño por escalón.
- **`tecnologia`:** los 12 contadores (los arrays de 24 pasan a 36 en `tecnologia`, `nucleo` y `batch`), las 6 condiciones de hito con su lectura, y un resumen de la batalla con lo que miran los contadores de §3.
- **`nucleo`:** el resumen en el encuentro con números, el asalto a una caravana y al aplicar una batalla de Unity; `escuadras.veteranas` al dar experiencia; `caminos.compartidos` cuando el camino se abre; la vista de la Facción con las condiciones nuevas, y el paso de minuto de la capital para las que cambian sin disparador; los efectos de Ejército permanente (+10 % de experiencia) y Logística de campaña (víveres ×1,5).
- **`batch`:** el simulador tecnológico (`simmil/tec.rs`) conoce las condiciones nuevas.
- **Contrato:** no cambia, porque las tecnologías, los contadores, las tropas y los recursos viajan como texto; solo el comentario de `pausarConsumo` en `orders.proto` nombra el aceite de sésamo.
- **Cliente jugador** (a `Docs/Coordinacion/Pendientes_Cliente_Jugador.md`): los iconos y los nombres de las 10 tropas y de las 12 tecnologías, las dos Eras en el árbol de tecnologías, el texto de los 6 hitos nuevos en la ficha, el aceite de sésamo (icono, nombre, y «Aceite de oliva» para `aceite`) y los bienes de la subida al nivel 5.
- **Partida nueva**, y suben `asentamiento`, `tecnologia`, `militar`, `nucleo` y `batch`.

## 8. Lo que toca en Conquest

Las 10 tropas como escuadras, con los modelos y las cifras de combate de la entrada 72.

## 9. Pendiente

- **Del autor:**
  - el modificador del aceite de sésamo (§2b): propuesto ×1,25, como la cerveza junto al vino; la otra opción es ×1,5, igual que el de oliva;
  - las épicas de 6 y 7 capítulos y sus nombres (§5).
- **En la fase 7:** medir los umbrales con bots.
