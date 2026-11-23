# Lo pendiente para el cliente jugador

Los cambios del motor que el cliente jugador (BronzeAgeClient) tiene que recoger se apuntan aquí, en vez de avisar a su hilo con cada uno, y se le pasan todos de golpe (decisión del autor, 2026-11-11).

**Cómo se usa.** Cada tanda del motor que toque algo que el cliente lee o muestra añade una entrada en «Pendiente», con la fecha, el commit o PR del motor, qué cambia en el contrato o en los datos y qué tendría que hacer el cliente. Cuando el autor decide pasárselo, se manda al hilo del cliente la lista entera y las entradas pasan a «Ya pasado», con la fecha. La regla de interfaz manda en todo: se muestra solo lo que el jugador puede hacer.

## Pendiente

### La cadena «Tu primera plaza», H14 a H18 (2026-11-22; nucleo 0.53.0, sin cambio de contrato; cambia la huella: partida nueva)

Lo pidió Víctor el 2026-11-22 tras un playtest. Cinco misiones de héroe nuevas: H14 Facción, H15 destruir un campamento de bandidos, H16 aportar 20 al Fondo, H17 comprar la Caravana de Fundación, H18 fundar. Recompensas en medallas (`objeto`) y 5 de trigo. Contadores nuevos del héroe: `aportes`, `caravanas` y `fundaciones`. H16 a H18 llevan `conPlaza`: se dan por cumplidas (y se reclaman) en cuanto la Facción del héroe tiene plaza, también al entrar en una que ya la tiene. Los títulos, textos y nombres de contador van en `TEXTOS_MISIONES` de `/v1/datos`, así que el cliente TypeScript las enseña sin cambios; el de Unity, igual si lee esos datos. Si el cliente tiene una guía propia del camino (el TS, desde 0.225.0), conviene que no repita lo que ya dice la misión.

### Cancelar la solicitud de ingreso y el enfriamiento de ingreso (2026-11-19; contrato 0.31.0, nucleo 0.52.0, politica 0.8.0, sin foto nueva ni partida nueva)

Lo pidió Víctor el 2026-11-19. **El cliente TypeScript lo recoge en la misma tanda** (0.224.0, lo pidió él); esta entrada queda para el cliente de Unity.

1. **Cancelar la solicitud.** Orden nueva `cancelar_solicitud { faccion_id }` (campo 203 del oneof): el solicitante retira su solicitud de esa Facción; las demás siguen. Sin tenerla, `faccion.no_ha_pedido`. Hecho `CiudadaniaCambiada` con `como = "cancelada"` (`faccion.solicitud_cancelada`). Donde se muestra «pedido», un botón «Cancelar».
2. **El enfriamiento al salir pasa de crear a entrar.** Dejar una Facción ya no impide crear otra (también con una solicitud abierta, que se cancela sola). Durante 24 h (`CIUDADANIA.cooldownIngresoFaccionHoras`) no se pide el ingreso en ninguna, tampoco en la que se dejó: rechazo `faccion.cooldown_ingreso`, que sustituye a `faccion.cooldown_creacion`. Los textos que hablaban de los 7 días de creación cambian.
### La Armería y seis viviendas más en la planta del campamento (2026-11-19; nucleo 0.51.0, militar 0.12.0, sin cambio de contrato ni partida nueva)

1. **`CampamentoMercenarios.planta`** trae una pieza `armeria` de nivel 1 (colocada tras el edificio militar) y 16 viviendas en vez de 10; `edificios` termina en `armeria`. Las plantas de los campamentos que ya existían cambian (no se guardan). Qué haría el cliente: dibujar la Armería en la vista 3D del campamento (modelo de Armería, estilo de campamento) y comprobar que las 16 tiendas caben en la vista.
2. **El tope de reclutas** del campamento pasa de 100 a 160 (`poblacion.maximo`); el cliente ya lo lee del ritmo, no tiene que cambiar nada salvo textos que digan 100.

### La tanda del 19: al carro desde la puerta, cancelar obras de pueblo, juego de recluta, Armería de los campamentos y lo que come la plaza (2026-11-19; contrato 0.30.0, nucleo 0.49.0, asentamiento 0.25.0, objetos 0.2.0, `OBJETOS` `bronzeage-objetos-2`, sin foto nueva ni partida nueva)

Lo pidió Víctor el 2026-11-19. **El cliente TypeScript lo recoge en la misma tanda** (lo pidió él); esta entrada queda para el cliente de Unity.

1. **Del almacén al carro a la puerta.** `cargar_de_plaza` y `cargar_de_pueblo` valen con el héroe a bordo de su columna a la puerta (siendo su Líder), sin entrar; y también `dejar_en_plaza` y `dejar_en_pueblo` (el mismo 19, más tarde: Víctor quiere en la puerta una ventana «Almacén» de dos lados, que sume de 10 en 10 por clic y se confirme al final). Con el almacén de la plaza cerrado a los ciudadanos, saca también el Tesorero. En la ficha de la plaza propia (`FACCION`, con `almacen`) y en la del pueblo, «Al carro» con la columna a la puerta.
2. **Cancelar la obra de un pueblo o su mejora.** `cancelar_obra` con la obra del pueblo (`obra-N`, la de levantarlo o la de su `mejoraAldea`, que lleva el id del pueblo): el Gobernador o el Maestro de Obras de la madre, desde la plaza, dentro del pueblo o con su columna a la puerta de la obra. Lo entregado se pierde. Activo sin mejora, `pueblo.sin_obra`; hecho `pueblo.mejora_cancelada { pueblo, nivel }`. Botón en Territorio y junto a «Entregar materiales».
3. **Juego de recluta.** 15 armaduras nuevas de tier I (`{hueco}_{peso}_recluta`, metal `recluta`, con casco). Las de inicio llevan `enTienda: false`: la tienda no las vende (se le pueden vender). Filtrar la tienda por `enTienda`. Conquest necesita su aspecto.
4. **Armería de los campamentos de mercenarios.** `comprar_objeto` y `vender_objeto` dentro de cualquier campamento; sin metales con tecnología (solo `inicio`, `recluta` y monturas); sin forja (`armeria.sin_armeria`). Los hechos llevan `campamento` en vez de `plaza`.
5. **Lo que come la plaza.** `Asentamiento.comida_poblacion` (59) y `comida_guarnicion` (60): milésimas por recurso y minuto; en el interior y en la plaza propia.

### El Liderazgo cuenta solo lo que va contigo (2026-11-18; nucleo 0.48.0, militar 0.11.0, sin cambios en el contrato ni partida nueva)

Lo pidió Víctor tras un `liderazgo.superado` al salir con 14 de 21 teniendo tropa en guarniciones. El Liderazgo es la tropa bajo tu mando en la batalla: al salir, al recoger tropa a la columna (de una guarnición, una avanzada, un pueblo o una escolta), al reclutar directo a la columna y al defender tu plaza, el motor cuenta solo lo que va en tu columna más lo que se suma. Lo prestado a guarniciones o escoltas ya no gasta Liderazgo (Heroe_Y_Tropa §5). Qué cambia para el cliente:

1. **Nada que tocar en lo que ya valida:** el cliente ya cuenta solo lo que sale (en 0.220.1 se revirtió la suma de lo de fuera). Solo comprobar que ningún otro aviso o barra de Liderazgo resta lo prestado.  
2. Al **recoger** una escuadra a la columna, si el cliente avisa antes de mandar la orden, la cuenta es lo que ya va en la columna más lo que vuelve.

### El vado es un punto (2026-11-18; mundo 0.5.0, rutas 0.2.0, nucleo 0.47.0, mundo nuevo y partida nueva, sin cambios en el contrato)

Lo pidió Víctor para que una columna parada en un vado esté en el camino de quien cruce. El cauce va cerrado entero y cada vado es un nodo del grafo en su punto, enlazado con hasta 3 nodos de cada orilla: toda ruta que cruza el río pasa exactamente por ese punto (Movimiento_Ejercitos §4.3, contrato 06). Qué cambia para el cliente:

1. **Mundo nuevo:** la semilla 42 da ahora el artefacto `a98e65c7…` (versión del generador `regional-20-costas-2+estrategico-3+cauce-2`) y su paquete de relieve nuevo. Ríos, vados, nodos y campamentos siguen en el mismo sitio; cambian el grafo y la rejilla de paso alrededor de los vados. Con él hace falta partida nueva.  
2. **Las rutas** que dibuja el cliente (plan de marcha de una columna) cruzan el río por el punto del vado, en vez de por cualquier sitio de una franja de unas 16 u. Si el cliente pinta el vado como una franja ancha, conviene que lo pinte como un paso estrecho.  
3. **Parar en un vado:** se puede mandar una columna al punto de un vado (se pisa a 0,75 u de él aunque su celda sea cauce). Si el cliente valida a mano el destino con la transitabilidad del artefacto, tiene que admitir también ese radio alrededor de cada vado (`mundo::RADIO_VADO`).

### Nada encima del río (2026-11-18; nucleo 0.46.0, territorio 0.4.0, asentamiento 0.24.0, sin cambios en el contrato ni partida nueva)

Lo pidió Víctor tras ver en Render una aldea de un bot encima del río. El motor ya no deja levantar nada donde su maqueta entraría en el cauce dibujado (`ORILLA_RIO`, Economia_Construccion §14.3). Qué cambia para el cliente:

1. **Rechazos nuevos:** `fundacion.junto_al_rio` (fundar), `pueblo.junto_al_rio` (marcar pueblo o aldea) y `avanzada.junto_al_rio` (colocar avanzada). Hacen falta sus textos.  
2. **El aviso de viabilidad** puede traer el motivo `fundacion.junto_al_rio`: el cliente lo enseña como los demás (rojo, «demasiado cerca del río»).  
3. **Los candidatos de pueblo** ya no traen los nodos junto al río, y el sitio de aldea que propone el panel queda fuera del cauce con sus campos. Si el cliente deja elegir a mano el punto de una aldea o una avanzada, conviene que no ofrezca los que el motor rechazará (las distancias están en el bloque `ORILLA_RIO` de `GET /v1/datos`, catálogo de Economia_Construccion).  
4. Lo levantado antes de la regla (la aldea de as-187 en Render) no se toca.

### Cambiar las escuadras elegidas en una batalla en que ya se combate (2026-11-16; contrato 0.29.0, nucleo 0.45.0, combate 0.12.0, sin partida nueva)

**`OrdenElegirEscuadrasDeBatalla { battle_id, escuadron_ids }`** (campo 202 del oneof): quien ya combate (el residente que defiende su plaza entra de oficio con su loadout) cambia sus elegidas antes de que empiece la partida, con las mismas reglas que `OrdenUnirseABatalla.escuadron_ids`. Con la partida en curso, `batalla.ya_empezada`. Qué haría el cliente: en la preparación de una batalla en que ya combate, mandar esta orden con lo elegido (Batalla_Unity_Ciclo §7.3).

### Batallas: elegir escuadras, bajarse, el final de quien sale y el hecho de cierre (2026-11-16; contrato 0.28.0, nucleo 0.44.0, combate 0.11.0, sin partida nueva)

Lo pidió el hilo de la migración a Unity. Qué cambia y qué tendría que hacer el cliente (Batalla_Unity_Ciclo §7, §7.2 y §10):

1. **`OrdenUnirseABatalla.escuadron_ids`** (campo 4): las elegidas en la preparación de la batalla. En su base (defender su plaza), un subconjunto de su campamento que quepa en su Liderazgo; fuera de ella, todas las que lleva (otra lista se rechaza con `batalla.sin_elegir_escuadras`). Vacío, como antes.
2. **`OrdenBajarseDeBatalla { battle_id }`** (campo 201 del oneof): bajarse antes de que empiece la partida; la batalla sigue para los demás. Con la partida en curso, `batalla.ya_empezada`. El cliente puede ofrecer «Bajarse» mientras la batalla esté convocando o asignada.
3. **`Batalla.salida_propia { motivo, derrotado, en_ms }`** (campo 22, motivo nuevo `SALIDA_BAJADA = 5`): quien salió sigue viendo la batalla entera hasta que se retira; el cliente puede enseñarle cómo acabó.
4. **El hecho `heroe.cierre_batalla`**, privado del héroe, al aplicarse el resultado: monedas, experiencia, herido, botín y pérdida del carro, objetos y cada escuadra con supervivientes, muertos y experiencia. Sirve para el informe de después de la batalla.
5. **`escuadras_que_lleva` de la vista** (columna ajena y ficha de mercenario) trae ahora el id de la tropa (`milicia_lanceros`) y no su nombre: el cliente tiene que traducirlo con su catálogo.

### El estilo de época de cada plaza (2026-11-16; contrato 0.27.0, nucleo 0.43.0, tecnologia 0.4.0, foto BAN65, partida nueva)

Lo pidió el hilo del cliente para la ciudad 3D. Un mensaje nuevo, `EstiloEpoca { era, grado }` (era 1 a 5, el orden de la Era; grado 1 a 3; la Era I llega siempre como (1, 3)), en tres sitios:

1. **`Asentamiento.estilo_epoca`** (campo 58): a quien ve la plaza por dentro (interior, la plaza propia también en `FACCION`, la ajena vista desde dentro en `VISTA` y `ADMIN`); ausente en la avistada desde fuera. Entra en la máscara del cambio parcial (`contrato::plaza`).
2. **`Pueblo.estilo_epoca_madre`** (campo 24): el de su plaza madre, solo con la planta, como `nivel_madre`.
3. **`Avanzada.estilo_epoca_madre`** (campo 16): el de su plaza madre, a su Facción y a quien está dentro de una ajena; ausente en la avistada y en una ruina sin dueño.

Es el máximo al que llegó la plaza y nunca baja (Tecnologia_Aedas §4.2). Qué haría el cliente: dibujar los edificios con ese estilo en vez de sacarlo de las adoptadas de la Facción (así vale también en la plaza ajena por dentro y en la conquistada).

### El sigilo en el ticket de batalla (2026-11-15; contrato 0.26.0, nucleo 0.42.0, politica 0.7.0)

Sin partida nueva: dos campos nuevos en el ticket, que el cliente ve en `Batalla.ticket`.

1. **`BattleSide.sigilo` y `BattleParticipantSnapshot.sigilo`** (el `Sigilo` de `common.proto`, con sus ocho ids). Siempre vienen: el de la Facción del bando o del participante; para los bandidos de un campamento, el reservado `bandidos` de `CATALOGO_SIGILO`; sin Facción, el `neutro`. Qué haría el cliente: si enseña los bandos de una batalla (la ficha de espera, el informe), dibujar el sigilo de cada uno con lo que ya tiene en `src/sigilo/`, sin buscar la Facción; para los bandidos, el emblema `bandidos` tendría que tener dibujo (hoy `catalogoSigilos.json` lo trae, ver si `emblemas.ts` lo pinta). Referencia: Batalla_Unity_Ciclo §3.2.

### La Vía Rápida en toda obra (2026-11-15; asentamiento 0.23.0)

Sin cambios en el contrato ni partida nueva: cambia lo que tardan dos obras.

1. **Subida de nivel de la plaza.** Su duración es ahora `obraMinutos × factorTiempoConstruccion` (mínimo 1 minuto), como las demás obras: la Vía Rápida la acorta. El fin sigue llegando en el `completa_en` del ascenso. Qué haría el cliente: si muestra la duración antes de pedir la subida, aplicarle el factor de las políticas activas de la plaza.
2. **Muralla.** Cada celda tarda `minutosPorCelda × factorTiempoMuralla × factorTiempoConstruccion`. Qué haría el cliente: lo mismo, si muestra lo que tarda cada celda o el recinto entero. Referencia: Economia_Construccion §11 y Murallas §5.

### Héroe hasta el nivel 50 y su aspecto guardado (2026-11-13, directo a main; nucleo 0.41.0, militar 0.10.0, contrato 0.25.0, foto BAN64, partida nueva)

1. **Curva del héroe** (`PROGRESION_HEROE`, en `Heroe_Y_Tropa`): `experienciaPorNivel` desaparece y llega `incrementos` (`[{desdeNivel, suma}]`). Pasar al 2 pide 3 y cada nivel 3 más hasta el 10 (27), 10 más hasta el 20 (127) y 20 más hasta el 50 (727); desde el 50, 747 cada uno. `nivelDeTope` es 50. Lo que le falta para subir sigue en `Heroe.experiencia_hacia_siguiente_nivel` (milésimas). Si el cliente muestra la curva o el total, que la calcule de los datos, no de la fórmula vieja.
2. **Liderazgo** (`LIDERAZGO`): `porNivel` desaparece y llega `nivelDelTope` (25): de 14 a 100 en el 25, 3 o 4 por nivel. El valor del héroe sigue en `Heroe.liderazgo_base`.
3. **Puntos de atributo:** uno por nivel hasta el 50. **Habilidades** en los niveles 1, 4, 8 y 16 y **perks** en el 11 y el 20 (`habilidadesEnNivel`, `perksEnNivel`), si se muestran.
4. **Aspecto:** `crearHeroe` guarda `genero` (masculino, femenino o vacío = masculino; otro texto, `orden.forma_invalida`) y las cuatro piezas de `avatar` (vacías o con hasta 32 letras ASCII, cifras, `_` o `-`; si no, `heroe.aspecto_invalido`, rechazo nuevo). Salen en `Heroe.femenino` y `Heroe.avatar` del héroe propio y del público.

## Ya pasado

Lo ya pasado al cliente se borra de aquí y queda en git (limpieza del 2026-11-23; lo último pasado, el 2026-11-12, motor en main c82d6ed).
