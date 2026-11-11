# Objetos del héroe: catálogo, mochila, equipo, tienda y forja

Diseño cerrado con el autor el 2026-11-11 (hilo «Evaluar objetos y armaduras del motor»), sin implementar. Entrada 70 de `Docs/Mecanicas a desarrollar.md`. Referencia: Conquest (`Conquest_prototype`, `ItemDataSO`, `EquipmentManagerService`, `ItemPricingService`) y su CQ-004, aceptada pero sin hacer.

## 1. De dónde se parte

- **Contrato.** Ya existen `ItemInstancia`, `Equipamiento` (arma, casco, torso, guantes, pantalones y botas), `Heroe.inventario`/`equipamiento`, `HeroSnapshot.equipamiento`/`casillas_inventario_libres` y `Botin.objetos`, y la versión `versionCatalogoObjetos` del ticket (hoy la cadena fija `conquest-items-1`).
- **Motor.** No guarda ni mochila ni equipo. Los objetos del botín de Unity se descartan (solo se suman las monedas), el ticket manda el equipo vacío y `atributosEfectivos = atributosBase`, y no hay orden para equipar. El héroe no tiene clase (se proyecta `default`). `BATALLA.casillasInventario` = 72.
- **Cliente jugador.** Pinta los seis huecos y la mochila, solo para mirar.
- **Conquest.** 12 armaduras (ligera, media y pesada de torso, guantes, pantalones y botas; ningún casco), 4 armas (una por clase) y 2 consumibles, todo Común. Un solo generador de stats al azar para todas las armaduras y otro para todas las armas, así que las tres armaduras dan lo mismo. En combate el equipo solo suma fuerza, destreza, armadura y vitalidad: las defensas, el daño, la penetración y la salud que tira el generador solo salen en el tooltip. Compatibilidad: arco con ligera; lanza y mandoble con ligera o media; espada y escudo con cualquiera. Precio por rareza y tipo, venta al 50 %, tienda en bronce. El proceso de batalla en red (2026-11-11) solo lee del héroe su id, su bando y si es bot, y no manda botín.

## 2. Decisiones (autor, 2026-11-11)

Regla de reparto: el motor guarda el estado persistente y las reglas fuera de la batalla; el servidor de batallas (Conquest) guarda lo táctico. Un solo dueño por número.

> **Cambiado por el autor el 2026-11-11, 19:30** (hilo del catálogo, [`Catalogo_De_Objetos.md`](Catalogo_De_Objetos.md)): **datos en el motor, reglas en Conquest.** Las cifras de combate de los objetos y de las tropas (atributos, daño, penetración, defensa, alivio y vida) son datos del motor, que las publica con su versión en `GET /v1/datos`. Conquest se queda con cómo se calculan (las fases del daño, los coeficientes de cada clase, los rasgos del golpe, las animaciones y el aspecto) y lee las cifras al arrancar el servidor de batalla. Motivos: el cliente jugador solo habla con el motor y las necesita para los tooltips; la forja tira los valores en el motor; y el equilibrio del héroe contra las tropas pide tener las dos cifras en el mismo sitio. Cambia los puntos 1, 2, 3 y 9.

1. **Catálogo partido con el mismo id.** El motor tiene la ficha de cada objeto: id, hueco, categoría de arma o tipo de armadura, rareza, si se apila y precio (y si da Liderazgo, si lo decide la curva de Liderazgo). Va versionado y se sirve en `GET /v1/datos`. Conquest tiene su aspecto y las reglas de combate, y rechaza un ticket con un id o una versión que no tenga. *(Desde las 19:30, las cifras de combate también son de la ficha del motor.)*
2. **Stats fijos por pieza** en el catálogo del motor (en Conquest hasta las 19:30). Las piezas **forjadas** llevan valores tirados por el motor y guardados en su instancia (`item_instance_id`, `estadisticas`).
3. **Qué da cada pieza en combate:** la cifra es del motor y la regla que la aplica, de Conquest (hasta las 19:30, todo de Conquest).
4. **Fuentes:** botín de Unity, recompensa de misión, tienda de monedas y forja.
5. **Moneda:** el saldo único del héroe (`monedas`). El bronce, la plata y el oro de Conquest no existen en modo conectado.
6. **Órdenes del motor:** equipar, desequipar, vender, comprar y forjar, con la regla de huecos y la de compatibilidad. El equipo llega a la batalla en el ticket, nunca desde el cliente.
7. **Equipo inicial** según la clase, por ids.
8. **El equipo no cuenta en las batallas con números.**
9. **Los atributos efectivos los suma el servidor de batallas.** El motor manda los base y el equipo. El cliente jugador enseña los stats leyendo `OBJETOS` del motor, solo para mirar (antes, un fichero que exportaba Conquest).

## 3. Lo que falta

### 3.1 Motor

1. **Catálogo `OBJETOS`** en Mechanics (bloque `parametros`): las fichas, la tabla de compatibilidad arma/armadura, el precio de venta y la versión, que sustituye a `conquest-items-1`. Servido en `GET /v1/datos`, como `FORMACIONES`.
2. **La clase del héroe.** Hoy `crear_heroe` la ignora. Hace falta guardarla (y su catálogo, con las cuatro de Conquest) para el equipo inicial y para el ticket.
3. **Mochila y equipo guardados en el héroe**: 72 casillas, apilables juntos, seis huecos. Persistencia y foto nueva (partida nueva).
4. **Proyección.** `HEROE` con la mochila y el equipo reales; la ficha pública con lo que lleva puesto (canon 5.16: público), la mochila privada.
5. **Órdenes** `equipar`, `desequipar`, `vender`, `comprar` y `forjar`, con sus rechazos en el catálogo 07 (objeto desconocido, hueco equivocado, incompatible con el arma, mochila llena, sin monedas, sin materiales, en batalla). Equipar quita lo incompatible si cabe en la mochila.
6. **Equipo inicial** al crear el héroe, según su clase.
7. **Ticket.** El equipo real, la clase, `casillas_inventario_libres` real y la versión del catálogo. `atributos_efectivos` pasa a llevar los base (o se deja de usar).
8. **Resultado de Unity.** Aplicar los objetos del botín: id del catálogo de la versión del ticket, apilar, caber. Las salidas anticipadas aún no traen botín (G4 de Conquest).
9. **Misiones.** El tipo `objeto` en `recompensas` (entrada 67).
10. **Mercenarios.** Los objetos de los rangos 4 y 5 (entrada 68).
11. **Tienda.** Dónde está, qué vende y a qué precio.
12. **Forja.** Recetas (pieza base, materiales, rangos), dónde se forja, quién paga (almacén personal) y la tirada del azar.
13. **Docs:** Heroe_Y_Tropa, Batalla_Unity_Ciclo §3.2 y §9, Comandos_Parametros, Proyecciones, 07, canon 5.16, y una entrada en `Pendientes_Cliente_Jugador.md` por cada tanda. Versiones de `militar`, `combate`, `nucleo` y `contrato`.

### 3.2 Contrato

- Las órdenes nuevas en `orders.proto` y el catálogo en los datos.
- Regenerar el TypeScript del cliente jugador y la DLL `BronzeAge.Contrato` de Conquest con cada cambio.

### 3.3 Cliente jugador (BronzeAgeClient)

- La ventana de equipo deja de ser solo para mirar: equipar, quitar y vender, con aviso de incompatibilidad.
- Tienda y forja (con los rangos antes de forjar).
- Tooltips con stats, de `OBJETOS` del motor.
- El equipo de otros héroes en su ficha, y los objetos del botín en los avisos.

### 3.4 Conquest como servidor de batallas

- Los ids de `ItemDataSO` iguales a los del catálogo del motor, y los cascos que faltan.
- Leer los stats fijos de cada pieza del bloque `OBJETOS` del motor (por versión) en lugar del generador, con ligera, media y pesada distintas.
- Rechazar un ticket con un objeto o una versión de catálogo que no tenga, como ya hace con `tropaId`.
- Montar el héroe desde el `HeroSnapshot`: clase, atributos base y equipo (hoy solo id, bando y si es bot). Depende de pasar el juego a red (punto 1 de «Lo que falta» de su `22_Servidor_De_Batalla_Red`).
- Usar los valores de las piezas forjadas que vengan en el ticket.
- Que las defensas, el daño, la penetración y la vida del equipo lleguen al combate como modificadores de la pila (`StatId`; su pendiente P1), no solo los atributos.
- El botín en el resultado.
- Leer también del motor las cifras de combate de las tropas en lugar de las de sus `SquadData`.
- Actualizar CQ-004 con este reparto.

### 3.5 Conquest como cliente del jugador (Unity)

- La ventana de inventario manda órdenes al motor en vez de usar los servicios locales (`EquipmentManagerService`, `StoreTransactionService`, `SaveSystem`).
- Pintar el equipo de los demás héroes desde el ticket.

### 3.6 Bots y administración

- **BotsBrain:** que los bots se equipen y compren, o irán sin equipo en las batallas de Unity.
- **AdminClient:** ver la mochila de un héroe y darle objetos para probar.

## 4. Por decidir

1. ~~El catálogo inicial~~: cerrado el 2026-11-11 en `Catalogo_De_Objetos.md` (30 armas y 87 armaduras en cinco tiers).
2. ~~A cuánto se vende~~: al 50 %, como en Conquest (`Catalogo_De_Objetos.md` §6).
3. Dónde están la tienda y la forja (propuesto: la Armería de la plaza). Qué vende la tienda: solo los tiers I y II (autor, 2026-11-11 21:31; `Catalogo_De_Objetos.md` §6).
4. ~~Las recetas de forja~~: plano del tier, crisoles y materiales (`Catalogo_De_Objetos.md` §6b).
5. Dónde puede equiparse el héroe: en cualquier sitio o solo donde reside.
6. ~~Si la clase limita el arma~~: el arma define la clase (`Clases_De_Heroe.md` §4).
7. ~~Quién decide el botín de Unity~~: las tablas las tiene el motor (`Catalogo_De_Objetos.md` §6b).
8. Si una armadura da Liderazgo (lo decide la curva de Liderazgo).
9. Si se pierde equipo al perder una batalla (propuesto: no; Conquest no tiene durabilidad).
10. ~~Consumibles en batalla~~: fuera (autor, 2026-11-11 22:32).

## 5. Orden propuesto

1. Catálogo, clase, mochila y equipar, en el motor y en el cliente jugador.
2. Botín, misiones y tienda.
3. Forja.
