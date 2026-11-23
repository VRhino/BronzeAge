# Lo pendiente para el cliente jugador

Los cambios del motor que el cliente jugador (BronzeAgeClient) tiene que recoger se apuntan aquí, en vez de avisar a su hilo con cada uno, y se le pasan todos de golpe (decisión del autor, 2026-11-11).

**Cómo se usa.** Cada tanda del motor que toque algo que el cliente lee o muestra añade una entrada en «Pendiente», con la fecha, el commit o PR del motor, qué cambia en el contrato o en los datos y qué tendría que hacer el cliente. Cuando el autor decide pasárselo, se manda al hilo del cliente la lista entera y las entradas pasan a «Ya pasado», con la fecha. La regla de interfaz manda en todo: se muestra solo lo que el jugador puede hacer.

## Pendiente

### Tamaños y distancias del mapa, tanda 1 (2026-11-23; nucleo 0.54.0, asentamiento 0.26.0, territorio 0.5.0, batch 0.11.1, foto `BAN66`, sin cambio de contrato; mundo nuevo y partida nueva)

Lo pidió Víctor desde la escena «Distancias de plaza» de Conquest (diseño `Docs/Disenos_Por_Implementar/Tamanos_Y_Distancias_Del_Mapa.md`, bloques A, B y D). Todo viaja en los bloques de `GET /v1/datos`: el cliente que lea las cifras de ahí solo tiene que cambiar los nombres que se van.

1. **Mundo nuevo:** la semilla 42 da el artefacto `7160008d…`, con 2.267 nodos (antes 4.986): menos piedra, cobre y hierro, y 6 u entre nodos de tipo distinto. Partida nueva.
2. **`TAMANOS_MAPA` cambia de forma:** ahora es `{ "radio": { … } }`, en **radio** (antes diámetros, todos 0,2). `plaza.sinMuralla` y `plaza.conMuralla` por nivel ("1" a "5"), `aldea` y `caserioDeAldea` por nivel ("1" a "3"), `avanzada` por etapa (`atalaya`, `fuerte`, `ciudadela`), y una cifra para `pueblo`, `caserioDeAldeaGanadera`, `campamentoMercenarios` y los demás. Si el cliente pinta con estas cifras, leerlas como radio y por nivel o etapa.
3. **`ORILLA_RIO.radio`** solo trae `plaza` (15) y `bandidos` (0,8): el pueblo, la aldea y la avanzada usan ahora su radio de `TAMANOS_MAPA` (pueblo; aldea de nivel 3; ciudadela). `sitioDeFundacion.ts` lee `ORILLA_RIO`: la plaza sigue ahí, con la cifra nueva.
4. **Fundar:** `FUNDACION.separacionPlazas` pasa a **60** y `MERCENARIOS.radioExclusionFundar` a **60**; `ZONA_INFLUENCIA.radioInicial` a **24** y `radioMaximoPorNivel` a 30 / 50 / 70 / 80 / 90. Regla nueva: **a `FUNDACION.distanciaMinimaANodo` (12) o más de todo nodo de recurso**, rechazo nuevo **`fundacion.cerca_de_nodo`** (también como motivo del aviso de viabilidad). Hace falta su texto en `erroresServidor.ts`, y que la comprobación de fundar del cliente (`sitioDeFundacion.ts`, `apiCliente.ts`, con 20 por defecto si falta el dato) añada la de los nodos y no dé por defecto 20.
5. **La aldea:** `PUEBLOS_EXTRACTORES.aldea.separacionMinima` desaparece; en su lugar `separacionAPlaza` (15) y `separacionAPueblo` (7,5, a otro pueblo o aldea). El cliente lee `separacionMinima` en `pueblos.ts` (`reglasDeAldea`, el texto de ayuda y el aviso de «tierra fértil a menos de…») y en `pestanaAsentamientos.ts` (el sitio de aldea de la plaza nueva): hay que usar las dos cifras. Los anillos de las aldeas siguen los techos nuevos (hasta 30, de 30 a 50, de 50 a 70…).
6. **La avanzada:** `AVANZADAS.fraccionCiudadDeLaZona` desaparece; `AVANZADAS.distanciaALaMadre` (15) es lo más cerca que se coloca del centro de su plaza madre (`avanzada.dentro_del_asentamiento`), y pisa otro elemento si la distancia es menor que la suma de los dos radios (la avanzada cuenta como ciudadela y la aldea como de nivel 3).
7. **Mercenario que defiende una plaza:** `atacar` en su radio (`radioDefensa`, 20) se mide ahora desde el **borde de la zona** de la plaza, no desde su centro. Si el cliente filtra los objetivos que ofrece, restar el radio de la zona.
8. **Textos:** donde el cliente diga 20 entre plazas, 12 de zona inicial, 3 entre aldeas o el techo de 20 del nivel 1, cambiar a las cifras nuevas (mejor, leerlas de los bloques).

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

**Pasado al cliente jugador el 2026-11-12** (motor en main c82d6ed), por decisión del autor: las tres entradas que siguen.

### Objetos, clases, cifras, progresión y Eras IV-V (2026-11-12, PR #22 y #23, main c82d6ed)

Partida nueva: la foto pasa a `BAN63`.

1. **El aceite de sésamo y la subida al nivel 5.**
   - Hay un recurso nuevo, `aceiteSesamo` («Aceite de sésamo»; ya son 42). `aceite` sigue con su id y ahora se llama «Aceite de oliva». La receta de sésamo de la Prensa da `aceiteSesamo` (antes daba `aceite`). La nobleza lo come con ×1,25 (`BIENES_DE_CONSUMO.bienes.aceiteSesamo`, en `Economia_Construccion`); con los dos aceites se suman los dos modificadores, como el vino y la cerveza. Se puede pausar con `pausar_consumo` y se comercia como cualquier recurso, con un precio base de 3.
   - Las plazas suben ya al nivel 5 (`NIVEL_ASENTAMIENTO.nivelMaximo`). `ASCENSO_ASENTAMIENTO.nivelTechoProvisional` ya no existe. La obra del 5 cuesta, además de la madera, la piedra y el oro, 1.000 de vino, 1.000 de cerveza, 1.000 de aceite y 1.000 de aceite de sésamo (`porNivelObjetivo.5.costo`). Los requisitos no cambian.
   - Qué haría el cliente: poner el recurso nuevo en el almacén, el mercado, las caravanas y el selector de `pausar_consumo`; cambiar el nombre visible de `aceite`; ofrecer la subida al 5 con su coste, sin tope en el 4. Referencia: Economia_Construccion §2, §5.3 y §9.2, y el ascenso.
2. **Las Eras IV y V.**
   - `ERAS` trae `profesionalizacion_imperial` («Profesionalización», 8 semanas) y `hegemonia_macedonica` («Hegemonía macedónica», 26 semanas). La V es la última: sin `era_minimo_hasta_ms` ni `era_plazo_hasta_ms`. `TARIFA_ADOPCION` trae las dos Eras.
   - `TECNOLOGIAS` trae 12 tecnologías nuevas. En la IV: `ejercito_permanente`, `cuerpo_oficiales`, `logistica_campana`, `infanteria_profesional`, `hostigadores_profesionales` y `carga_caballeria`. En la V: `reforma_macedonica`, `falange_sarisa`, `cuerpo_hipaspistas`, `formacion_cuna`, `companeros_reales` y `acero_templado`. Sus hitos usan seis condiciones nuevas: `escuadrasVivas {cantidad}`, `heroesEnFaccion {cantidad}`, `plazaEnNivel {nivel}`, `nivelFaccion {nivel}`, `columnasEnMarcha {cantidad}` y `edificioEnPlazaDeNivel {edificio, nivelInterno, nivelPlaza}`. Llegan en `TecnologiaRevelada.cumplidas`, y el cliente compone la frase con el catálogo.
   - Hay 12 contadores nuevos (36 en total) en `EstadoTecnologia.contadores` y en los logros: `fabricado.armaHierroCalidad`, `caminos.compartidos`, `escuadras.veteranas`, `batallas.conTresHeroes`, `batallas.ganadasEnInferioridad`, `escuadras.eliteDestruidas`, `batallas.campoAbiertoConCaballeria`, `batallas.campoAbiertoConCaballeriaElite`, `batallas.armasCombinadas`, `batallas.contraCoalicion`, `batallas.conRey` y `persecuciones.deCaballeria`. Tres logros llevan un umbral fijo y no por plaza: `logistica_campana` (1), `falange_sarisa` (3) y `cuerpo_hipaspistas` (10).
   - `EPICAS` trae una épica para cada una de las nuevas: 6 capítulos en la IV y 7 en la V. La épica de `forja_hierro_estandarizada` pasa a llamarse «Canto de los cálibes», y «Canto del acero templado» es ahora la de `acero_templado`.
   - Efectos que el cliente puede enseñar: con `logistica_campana`, los víveres por héroe se multiplican por 1,5 (`LOGISTICA.bonusViveres`, en `Movimiento_Ejercitos`); con `ejercito_permanente`, las escuadras ganan un 10 % más de experiencia en combate (`MILITAR.bonusExperiencia`).
   - Qué haría el cliente: los textos de las dos Eras, de las 12 tecnologías, de los 12 contadores (para la barra «x / y») y de las seis condiciones de hito; no esperar un plazo en la Era V; y, si calcula por su cuenta los víveres máximos, aplicar el ×1,5. Referencia: Tecnologia_Aedas §2.1, §3.1 y §7.2.
3. **Las tropas.**
   - Hay 10 tropas nuevas en `TROPAS_RECLUTABLES`, cada una con su tecnología:
     - del Barracón: `epilektoi` (`infanteria_profesional`), `inmortales` (`infanteria_profesional`), `falangitas` (`falange_sarisa`; se reclutan de artesanos, por su campo `poblacion`) e `hipaspistas` (`cuerpo_hipaspistas`);
     - de la Galería de tiro: `peltastas_ificrates` (`hostigadores_profesionales`) y `agrianos` (`reforma_macedonica`);
     - de las Caballerizas: `caballeria_tesalia` y `caballeria_noble_persa` (`carga_caballeria`), `prodromos` (`formacion_cuna`) y `companeros` (`companeros_reales`).
   - Toda tropa lleva ahora `tier` (del 1 al 5, la rareza de Conquest; no es el `escalon`, que sigue fijando el Liderazgo, el oro y la población) y un bloque `combate`: `vida`; `dano`, `penetracion`, `defensa` y `alivio`, cada uno con `perforante`, `cortante` y `contundente`; `bloqueo`, `cadencia` y `alcance`; y `precision` y `municion` en las de distancia. El motor no usa estas cifras: son para Conquest y los clientes.
   - Cambian los soldados por escuadra (`unidadesPorDefecto`): las de distancia tienen 20 todas (antes 12 o 18), los hequetai 18 y los espartiatas 16. `VERSION_CATALOGO_TROPAS` es 2 y va en `BattleRules.version_catalogo_tropas`.
   - Cambia la curva de experiencia de las escuadras (`MILITAR.experienciaParaSubirEscuadra`): 10, 25, 50, 100, 150, 200, 250, 275 y 315, 1.375 hasta el nivel 10 (antes eran 275). En una batalla de Unity, el motor da 4 de experiencia si gana el bando y 2 si pierde (`experienciaUnityPorVictoria` y `experienciaUnityPorDerrota`). `MILITAR.nivelesHabilidadEscuadra` (4, 7 y 10) publica los niveles que abren habilidad (D8).
   - Qué haría el cliente: las 10 tropas en el reclutamiento, cada una visible solo con su tecnología y su edificio; la ficha de combate con `tier` y `combate`; la barra de experiencia de la escuadra con la curva nueva; y, si quiere, «habilidad nueva en el nivel N». Referencia: Heroe_Y_Tropa §12 (`TROPAS_RECLUTABLES` y `VERSION_CATALOGO_TROPAS`) y Combate §3.3.
4. **Crear un héroe exige una clase (D21).**
   - `OrdenCrearHeroe.class_definition_id` tiene que ser un id de `CLASES_HEROE`: `lanza_escudo_redondo`, `espada_escudo_mediano`, `arco_lanza`, `hacha_dos_manos` o `dagas`. Vacía o desconocida, la orden se rechaza con `heroe.clase_desconocida`.
   - El héroe nace con el equipo inicial de su clase puesto: el arma de tier I de la clase, las cuatro piezas de tier I de su `armaduraInicial` (torso, guantes, pantalones y botas, sin casco) y `montura_tiro`. Nace también con la mochila y el almacén de objetos vacíos.
   - Qué haría el cliente: un selector de clase al crear el héroe, con `nombre`, `armaduras` y `atributosBase` de cada clase y su equipo inicial; sin clase, no se puede mandar la orden. Referencia: Heroe_Y_Tropa §3, Objetos_Del_Heroe §3.2.
5. **El nivel y la experiencia del héroe; adiós a los perks.**
   - El héroe sube de nivel. Subir del nivel n al n + 1 pide 100 + 20 × (n − 1) de experiencia (100, 120… 660 para pasar al 30). Desde el 30, cada nivel pide 680 (`PROGRESION_HEROE`, en `Heroe_Y_Tropa`). La experiencia sale solo del combate: con números, 1 si gana su bando y 0,5 si pierde, una vez por combate; en Unity, 4 o 2. La medalla del héroe da también experiencia (punto 7).
   - `Heroe.experiencia_hacia_siguiente_nivel` (10) es ahora **lo que le falta** para el siguiente nivel, en milésimas. Antes era la experiencia sin curva.
   - Cada nivel hasta el 30 da un punto de atributo (`puntos_de_atributo_sin_gastar`), que ya se puede gastar con `repartir_puntos`, y 3 de Liderazgo. El héroe **nace con 14 de Liderazgo** (`LIDERAZGO.base`, antes 100) y llega a 100 en el nivel 30 (`liderazgo_base`). Con 14 lleva 2 escuadras de leva o 1 de línea. Desde el 31 el nivel sigue subiendo sin dar nada.
   - El bloque `LIDERAZGO` está ahora solo en `Heroe_Y_Tropa` (con `porNivel` y `tope`). El que había en `Movimiento_Ejercitos` ya no existe.
   - Al subir de nivel llega el hecho `heroe.nivel_subio { heroe, nivel }`, con el nivel nuevo, una vez aunque cruce varios.
   - Los perks desaparecen: `Heroe.puntos_de_perk_sin_gastar` (12), `Heroe.perks_desbloqueados` (14), `Loadout.perks_seleccionados` (4) y `OrdenGuardarLoadout.perks_seleccionados` (4) quedan `reserved`, y el rechazo `loadout.perk_no_desbloqueado` ya no existe. Los abre el nivel en Conquest: `PROGRESION_HEROE.perksEnNivel` (7 y 12) y `habilidadesEnNivel` (1, 3, 5 y 10) solo se publican.
   - Qué haría el cliente: el nivel y la barra hacia el siguiente, sacada de `experiencia_hacia_siguiente_nivel` y de la curva; un aviso con `heroe.nivel_subio`; el botón de repartir puntos solo con puntos sin gastar; leer el Liderazgo de `liderazgo_base` y no del bloque; ofrecer al reclutar, al salir y en los loadouts solo lo que cabe en ese Liderazgo; y quitar los perks del loadout. Referencia: Heroe_Y_Tropa §4.3 y §5.
6. **La clase, el equipo, la mochila y el almacén de objetos en `HEROE`.**
   - `class_definition_id` es la clase real: la `clase` del arma puesta. Cambia si el héroe se pone un arma de otra clase.
   - `atributos_base` es la suma de los `atributosBase` de la clase y de los puntos repartidos (D16). Lo que da el equipo no se suma, porque lo suma Conquest.
   - `equipamiento` (29) trae el equipo puesto, con el hueco nuevo `montura` (7) además de `arma`, `casco`, `torso`, `guantes`, `pantalones` y `botas`. El arma está siempre puesta.
   - `inventario` (28) es la mochila, con 72 casillas (`OBJETOS.casillasMochila`). `almacen_objetos` (44, nuevo) es el almacén de objetos de la residencia, con 144 casillas (`casillasAlmacen`). Las dos llegan solo en el héroe propio.
   - Cada pieza es un `ItemInstancia`:
     - `item_definition_id`, el id de `OBJETOS`;
     - `tipo`, uno de `arma`, `armadura`, `montura`, `material` o `consumible` (antes se documentaban `consumible` y `visual`);
     - `precio`, el de su ficha;
     - `cantidad`, porque lo apilable va junto en una sola casilla;
     - `casilla_inventario`, el número fijo de su casilla, de 0 a la capacidad menos 1, que es el que llevan las órdenes. Una pieza puesta lleva 0, que no significa nada.
   - Solo una pieza **forjada** lleva `item_instance_id` (`obj-N`) y sus cifras ya multiplicadas por la calidad en `estadisticas`: `fuerza`, `destreza`, `armadura` y `vitalidad`; `danoPerforante`, `danoCortante`, `danoContundente`, `penetracionPerforante`, `penetracionCortante` y `penetracionContundente` en un arma; `defensaPerforante`, `defensaCortante`, `defensaContundente` y `salud` en una armadura. La de tienda o de botín es la ficha al 100 % y no lleva ni id ni cifras: las cifras se leen del `combate` de su ficha.
   - **Otro héroe** (`HeroePublico`, en `VISTA`) muestra su clase y su equipo puesto. No muestra su mochila, su almacén de objetos ni sus atributos.
   - En la batalla (`BATALLA`, `ticket`), `HeroSnapshot` trae la clase, el equipo real y `atributos_efectivos` (la misma suma), y `BattleRules` trae `version_catalogo_heroe` y `version_catalogo_objetos` (las `version` de `CLASES_HEROE` y `OBJETOS`).
   - Qué haría el cliente: la pantalla de equipo (siete huecos), la mochila y el almacén por casillas, el tooltip de cada pieza con su ficha o, si es forjada, con sus `estadisticas`, y el equipo de otro héroe en su ficha. Referencia: Heroe_Y_Tropa §1 y §4.5, Objetos_Del_Heroe §3.1 y §3.2.
7. **Nueve órdenes nuevas (192 a 200, en `orders.proto`).** Las ocho primeras las da el héroe de la cuenta; `configurar_forja` (200), el Tesorero o el Gobernador de una plaza (punto 11). Ninguna cambia nada si se rechaza.
   - `equipar_objeto { casilla }` (192) se pone la pieza de esa casilla de la mochila. Lo que estaba en el hueco vuelve a la mochila. Si es un arma de otra clase, vuelven también las armaduras cuyo peso no admite la clase nueva.
   - `desequipar_objeto { hueco }` (193) se quita la pieza de un hueco (`casco`, `torso`, `guantes`, `pantalones`, `botas` o `montura`) y la pasa a la mochila. El arma no se quita (`equipo.arma_fija`). Un hueco desconocido es `orden.forma_invalida`.
   - Dónde se equipa y se quita (D2): dentro de una plaza (también ajena), de un pueblo o de un campamento. En una columna, en una avanzada o fuera del mundo, el rechazo es `equipo.fuera_de_lugar`; con alguna escuadra suya con `reserva_batalla`, `equipo.en_batalla`. Las dos órdenes emiten `heroe.equipo_cambiado { heroe }`, y el equipo nuevo llega en `HEROE`.
   - Rechazos de equipar y quitar: `equipo.no_equipable` (un material o un consumible), `equipo.incompatible` (la clase del arma no admite ese peso: `CLASES_HEROE.armaduras` frente a `tipoArmadura`), `equipo.hueco_vacio`, `mochila.llena`, `objeto.desconocido`, `inventario.casilla_vacia`, `inventario.poca_cantidad` e `inventario.cantidad_invalida`.
   - `guardar_objeto { casilla, cantidad }` (194) pasa piezas de la mochila al almacén de objetos, y `sacar_objeto { casilla, cantidad }` (195) del almacén a la mochila. Las dos se dan solo dentro de la residencia, donde está el almacén (D22): fuera, `almacen_objetos.fuera_de_tu_residencia`; sin residencia, `heroe.sin_residencia`. Otros rechazos: `almacen_objetos.lleno`, `mochila.llena` y los de las casillas de arriba. No emiten hecho; el cambio llega en `HEROE`.
   - `comprar_objeto { objeto }` (196) compra una pieza al 100 %, con las monedas del héroe, y la mete en la mochila. `vender_objeto { casilla, cantidad }` (197) vende cualquier objeto de la mochila al 50 % del precio de su ficha (`ventaPct`), redondeando hacia abajo; una pieza forjada vale lo mismo (D18).
   - Dónde se compra y se vende (D1): dentro de cualquier plaza con una Armería activa (`armeria.sin_armeria`). Solo se compran armas, armaduras y monturas hasta el tier `tiendaHastaTier` (2) (`armeria.no_se_vende`), y solo si la Facción **de la plaza** tiene la tecnología del metal (`OBJETOS.metales`; `inicio` y las monturas no piden ninguna; si no, `armeria.sin_tecnologia`). Otros rechazos: `heroe.sin_monedas` y `mochila.llena`. Hechos: `heroe.objeto_comprado { heroe, plaza, precio, objeto }` y `heroe.objeto_vendido { heroe, plaza, cantidad, cobro, objeto }`.
   - `forjar_objeto { objeto }` (198) forja un arma o una armadura de los tiers II a V dentro de **cualquier plaza** con la Armería activa, si su forja está abierta al grupo del héroe (la de su Facción, siempre; punto 11). Fuera de una plaza con la Armería activa, el rechazo es `armeria.sin_armeria`; con la forja cerrada a su grupo, `forja.cerrada`. Gasta de la mochila un `plano_{tier}` y los crisoles de la receta, y los `materiales` de la receta, en unidades: en la plaza donde reside, primero de su almacén personal (`HEROE.almacen_personal`) y lo que falte del carro; en cualquier otra, del carro de la columna con la que entró. Paga además en monedas el impuesto de la forja (punto 11), que va al fondo de la plaza. Los tiers III a V piden que el mundo esté en su Era (`forja.era_cerrada`). La calidad sale entre el 90 y el 110 %, y la pieza nace con su `obj-N`. Rechazos, en este orden: `armeria.sin_armeria`, `forja.cerrada`, `objeto.desconocido`, `forja.sin_receta`, `armeria.sin_tecnologia`, `forja.era_cerrada`, `forja.sin_plano`, `forja.sin_crisoles`, `mochila.llena`, `forja.sin_materiales` y `heroe.sin_monedas`. El rechazo `forja.fuera_de_tu_residencia` ya no existe. Hecho: `heroe.objeto_forjado { heroe, plaza, pieza (el N de obj-N), calidad (%), impuesto (monedas), objeto }`.
   - `usar_objeto { casilla, escuadron_id? }` (199) usa un consumible de la mochila, donde se equipa (D2), con sus mismos rechazos. Lo que no es consumible da `objeto.no_se_usa`. Cada consumible tiene su efecto:
     - una medalla de tropa da su `experienciaTropa` a `escuadron_id`, que tiene que ser una escuadra suya (`escuadra.ajena`), no prestada (`escuadra.prestada`) y por debajo del nivel 10 (`escuadra.nivel_maximo`), sin el bonus de Ejército permanente;
     - una medalla del héroe da su `experienciaHeroePct` de lo que pide su nivel actual para subir, y puede emitir `heroe.nivel_subio`;
     - el cofre de monturas da una montura de tier III, IV o V, que llega como el botín (punto 8).
   - Hecho de `usar_objeto`: `heroe.objeto_usado { heroe, objeto }`, más `escuadra` si es una medalla de tropa, y `montura` y `destino` (`mochila`, `almacen` o `perdido`) si es el cofre.
   - Qué haría el cliente (la regla de interfaz manda):
     - **Equipar, quitar y usar**: solo en uno de esos tres sitios (`ubicacion`) y sin ninguna escuadra con `reserva_batalla`; «Equipar» solo en armas, armaduras de un peso que admite la clase y monturas; nunca «Quitar» en el arma.
     - **Guardar y sacar**: solo con `ubicacion` igual a `residencia_id`.
     - **La tienda**: solo dentro de una plaza con la Armería activa, con las piezas que se pueden comprar allí y las monedas que hacen falta.
     - **La forja**: solo dentro de una plaza con la Armería activa y su forja abierta a su grupo, con las recetas de los metales de `metales_armeria` cuyo tier ya abrió la Era del mundo y cuyo plano, crisoles, materiales (los que tiene a mano allí) e impuesto tiene el héroe; cada receta con su coste, su impuesto y su rango de calidad (punto 11).
     - **La medalla de tropa**: un selector con sus escuadras que la pueden recibir.
     - **Mochila llena**: si no queda casilla libre en la mochila, no ofrecer comprar, forjar, sacar, quitar ni equipar algo que mande una pieza a la mochila.
   - Referencia: Heroe_Y_Tropa §4.4 a §4.6, Proyecciones_Y_Autorizacion (la tabla de órdenes).
8. **El botín de objetos.**
   - Lo que llega sin pedirlo va a la mochila. Si no cabe, va al almacén de objetos, aunque el héroe esté fuera de su residencia; si tampoco cabe, se pierde (D17 y D22).
   - Cada entrega emite `heroe.botin { heroe, origen, mochila, almacen, perdido }`, que oye solo el héroe. Las tres listas son de `[objeto, cantidad]`, con la forma de `recursos` en `mision.reclamada`. Los `origen` son:
     - `unity`: cada héroe que participó en una batalla de Unity contra otros héroes, con la tabla `unity`; lo de `soloSiGana` llega solo si ganó su bando. Contra bandidos no llega: solo `bandidos`;
     - `bandidos`: quien cobra el botín de un campamento destruido, con la tabla de su nivel, dentro de las 8 primeras destrucciones completas de su ventana de 24 h;
     - `diaria`: quien reclama una diaria **individual** del tablón, además de su recompensa;
     - `alijo`: quien abre un alijo;
     - `mercenario`: el mercenario que llega al rango 4 o al 5;
     - `mision`: el objeto de una recompensa de misión personal.
   - `RecompensaMision.tipo` puede ser `objeto`: `clave` es el id de `OBJETOS` y `cantidad`, piezas (no milésimas). Solo puede darlo una misión personal y nunca es un material. Hoy ninguna misión de los datos lo usa. Ese objeto no va en `mision.reclamada` sino en el `heroe.botin` que sale justo después.
   - Qué haría el cliente: un aviso con `heroe.botin` que diga qué fue a la mochila, qué al almacén y qué se perdió. En la diaria individual, además, la tabla `misionDiaria` como premio (hoy un crisol, una medalla de tropa pequeña y una medalla del héroe muy pequeña). Y pintar el tipo `objeto` en la lista de recompensas de una misión. Referencia: Objetos_Del_Heroe §3.7, Misiones §4.1, Misiones_Recurrentes §5.
9. **Los datos de los objetos en `GET /v1/datos`**, en el documento nuevo `Objetos_Del_Heroe`.
   - **`OBJETOS`**: `version`, `ventaPct` (50), `tiendaHastaTier` (2), `casillasMochila` (72), `casillasAlmacen` (144), `forja` (la forja en las plazas, punto 11: `impuestoPorTier`, `recargosPct`, `recargoInicialPct` y `eraPorTier`), `rarezas` (el nombre de rareza de cada tier, de `comun` a `legendaria`), `metales` (la tecnología que abre cada metal) y `objetos`: 136 fichas (30 armas, 87 armaduras, 5 monturas, 5 materiales y 9 consumibles).
   - Cada ficha lleva `id`, `nombre`, `tipo`, `tier` (`null` en los consumibles y en el crisol), `apilable` y `precio` en monedas. Las armas llevan además `clase`; las armaduras, `hueco` y `tipoArmadura` (`ligera`, `media` o `pesada`); las dos, `metal`, `combate` (las cifras del tooltip) y `forja` (`plano`, `crisoles`, `materiales` y `calidad`; `null` en el tier I). Los consumibles llevan `efecto`: `experienciaTropa`, `experienciaHeroePct` o `monturaDe`.
   - **`CLASES_HEROE`**: `version` y las cinco clases, con `nombre`, `armaduras`, `armaduraInicial` y `atributosBase`.
   - **`BOTIN_OBJETOS`**: las tablas `bandidos` (por nivel, del 1 al 3), `unity`, `misionDiaria`, `alijo` y `rangoMercenario` (rangos 4 y 5). Cada entrada es fija (`objeto`, `cantidad`, `probabilidad`, `soloSiGana`) o `unoDe`.
   - Los precios bajos de lo que no es arma ni armadura (D19) son los que paga la tienda al venderle, la mitad del `precio` de la ficha:
     - medallas de tropa: 2, 5, 10 y 20;
     - medallas del héroe: 5, 10, 20 y 40;
     - cofre de monturas: 50;
     - crisol: 5;
     - plano: 10 por su tier.
     Ninguno se compra: la tienda solo vende armas, armaduras y monturas.
   - Qué haría el cliente: los tooltips, la tienda, la forja y el selector de clase con estos tres bloques, sin copiar ninguna cifra. Referencia: Objetos_Del_Heroe §2 y §4.
10. **Lo que sale del contrato y de los datos.** Por si el cliente lo leía:
    - En `HEROE`: `Heroe` 12 y 14, `Loadout` 4 y `OrdenGuardarLoadout` 4, los de los perks (punto 5).
    - En la batalla (`BATALLA`, `ticket` y `resultado`): `HeroSnapshot.perks_desbloqueados` (6) y `casillas_inventario_libres` (9); `ResultadoEscuadra.xp_ganada` (5); `ResultadoHeroe.xp_ganada` (4) y `botin` (6). Los mensajes `Botin` y `ObjetoBotin` ya no existen: la experiencia, las monedas y los objetos de una batalla de Unity los pone el motor (D6 y D7), y llegan por `HEROE` y `heroe.botin`.
    - `SquadSnapshot` gana `estandarte` (13) y `aspecto` (14), que llegan vacíos (D20): el cliente los puede ignorar.
    - En `GET /v1/datos`: desaparecen `BATALLA.casillasInventario`, `BATALLA.versionCatalogoHeroe` y `BATALLA.versionCatalogoObjetos` (en `Batalla_Unity_Ciclo`), `LIDERAZGO` de `Movimiento_Ejercitos` y `ASCENSO_ASENTAMIENTO.nivelTechoProvisional`.
    - El rechazo `forja.fuera_de_tu_residencia`, que no llegó a salir: se forja en cualquier plaza (punto 11).
11. **La forja en cualquier plaza y el fondo de la plaza** (decisión del autor del 2026-11-12; Heroe_Y_Tropa §4.6, Objetos_Del_Heroe §2.1 y §3.4, Mercenarios_Por_Contrato §5.1).
   - **Quién forja dónde.** El grupo del héroe respecto de la plaza es el de la puerta: la misma Facción (su `Heroe.faccion_id` igual al `Asentamiento.faccion_id`; un mercenario contratado es ciudadano de la Facción que lo contrata y cuenta como ella), aliados (una `RelacionPolitica` activa de alianza o de vasallaje entre las dos Facciones, en cualquier sentido), enemigos (una de guerra) y neutrales (el resto, también sin Facción). El cliente lo saca de su `faccion_id`, del de la plaza y de las relaciones activas de su Facción, que le llegan en `FACCION` (y todas, públicas, en la `VISTA`). La misma Facción forja siempre; cada grupo de fuera, solo si la plaza le abrió su forja.
   - **El impuesto.** Se paga en monedas del héroe por cada forja aceptada: `OBJETOS.forja.impuestoPorTier` del tier de la pieza (II 100, III 175, IV 250 y V 350) por `(100 + recargo) / 100`, redondeado hacia abajo, con el recargo de su grupo (la misma Facción, 0: paga la base). Si no le llegan las monedas, `heroe.sin_monedas`.
   - **La Era.** `OBJETOS.forja.eraPorTier` da el `orden` de la Era que pide cada tier (III la 3, IV la 4, V la 5; el II no pide). La Era del mundo es `EstadoTecnologia.era` (en `HEROE`), y su `orden`, el de `ERAS` en `/v1/datos` (Tecnologia_Aedas).
   - **Lo que ve el héroe.** Desde fuera, toda ficha de plaza lleva `Asentamiento.armeria_activa` (55), también la avistada: basta para decir «aquí hay Armería». Dentro de la plaza (el interior en `ASENTAMIENTO` si es de su Facción; la plaza ajena vista desde dentro en la `VISTA` si no) llegan además `forja` (56, `ForjaPlaza`: `aliados`, `neutrales` y `enemigos`, cada uno `AperturaForja { abierta, recargo_pct }`, y `fondo`, las monedas del fondo) y `metales_armeria` (57, los metales cuya tecnología tiene la Facción de la plaza, `inicio` incluido; vacío sin Armería activa). El motor no calcula nada por espectador: con eso, su grupo, `OBJETOS.forja` y la Era, el cliente sabe si puede forjar, qué tiers y cuánto pagaría. Una adopción de tecnología de la Facción de la plaza le vuelve a mandar la plaza con los metales nuevos.
   - **`configurar_forja { asentamiento_id, grupo, abierta, recargo_pct }` (200).** El Tesorero o el Gobernador de la plaza, residente y dentro (si no, `no_autorizado`), abre o cierra la forja a un grupo de fuera (`grupo`: `aliados`, `neutrales` o `enemigos`; otro valor, también `aedas`, es `orden.forma_invalida`) y le fija el recargo, uno de `OBJETOS.forja.recargosPct` (0, 100, 200 o 500; otro, `forja.recargo_invalido`). La misma configuración que ya tiene da `sin_cambios`. De partida la forja está cerrada a los tres grupos con el recargo `recargoInicialPct` (100), y vuelve a eso cuando conquistan la plaza (el fondo se queda con ella). Hecho: `asentamiento.forja_configurada { plaza, grupo (texto), abierta (0 o 1), recargo (%) }`, que oyen su Facción y quien está dentro de la plaza. Abrir la forja no abre la puerta: para entrar, el grupo tiene que poder pasar la puerta (`fijar_puerta`).
   - **Pagar contratos con el fondo.** `contratar_mercenario` admite `pagar_con = "fondoPlaza"`: desde dentro de una plaza de su Facción con la Taberna activa, como `almacenPlaza` (el campamento del mercenario en la región de la plaza; firma el Rey, el Embajador o el Gobernador, y otro cargo, también el Tesorero, da `no_autorizado`), paga la **tarifa en monedas** del tablón con el fondo de la plaza (`forja.fondo`), y las monedas pasan a las del mercenario. Si el fondo no llega, `mercenarios.sin_fondo`. En el campamento y en `prorrogar_contrato`, `orden.forma_invalida`. El hecho `mercenarios.contratado` lleva las monedas en `monedas` y `oro` 0.
   - Qué haría el cliente (la regla de interfaz manda):
     - en el mapa, el icono de Armería en las plazas con `armeria_activa`;
     - dentro, el panel de la forja con las recetas que puede forjar allí y su impuesto; nada de forjar si la forja está cerrada a su grupo, ni recetas de un metal que no está en `metales_armeria`, ni de un tier cuya Era no ha llegado; y si el héroe no reside en la plaza, solo con los materiales de su carro;
     - el panel de configuración de la forja (los tres grupos, abierta o cerrada y el recargo de la lista) solo al Tesorero o al Gobernador de la plaza, residentes y dentro, con el fondo a la vista;
     - en la Taberna, «pagar con el fondo de la plaza» solo al Rey, al Embajador o al Gobernador, dentro, y solo si el fondo llega a la tarifa en monedas.

### Formaciones, +3 % por nivel y batallas en Unity (2026-11-11, PR #18, main e89f8d3)

Partida nueva: la foto pasa a `BAN62`.

1. **La formación de cada escuadra.**
   - `Escuadron.formacion` (campo 18, en `HEROE`): la familia elegida, por su id. Toda escuadra nace en `reposo`.
   - El catálogo es el bloque `FORMACIONES` de `Heroe_Y_Tropa` en `GET /v1/datos`. Cada familia trae `id`, `nombre` y `tecnologia` (`null` si no pide ninguna): `reposo`, `linea`, `columna` y `dispersa` sin tecnología, y `muro_escudos`, `testudo`, `cuadro` y `cuna` con `disciplina_formacion`. La de reposo es cómo forma cada tropa al seguir al héroe o pararse, y está siempre.
   - La orden `elegir_formacion` (`OrdenElegirFormacion { escuadron_id, formacion }`, 191 en `orders.proto`). La da la cuenta del héroe, esté donde esté. No emite hechos: la escuadra llega con su `formacion` nueva en `HEROE`.
   - Rechazos: `formacion.desconocida`, `formacion.sin_tecnologia`, `escuadra.ajena`, y `batalla.bloqueado` mientras la escuadra combate. La misma que ya tiene da `sin_cambios`.
   - Qué haría el cliente: en la ficha de la escuadra, la formación actual y un selector solo con las permitidas. Las permitidas son la de reposo y las que no piden tecnología, más las que pide `disciplina_formacion` si la Facción la tiene (`TecnologiasFaccion`, en `FACCION`). Un héroe sin Facción solo tiene las que no piden tecnología. Sin selector mientras la escuadra está en batalla. Referencia: Heroe_Y_Tropa §10.4.
2. **El nivel de escuadra da +3 % de poder por nivel** en el combate con números (antes +1 %). Solo si el cliente estima o enseña ese poder.
3. **Partidas con las batallas en Unity.** Una partida creada con «batallas en Unity» (`BRONZEAGE_BATALLAS_EN_UNITY=1`; en `GET /v1/datos`, `reglas.batallas_en_unity`) no resuelve `atacar` con números. Abre una `Batalla` que juega el lanzador, y con el servidor de batalla de mentira se aplica a los 30 s: gana el defensor y nadie muere. Para el cliente:
   - La batalla en curso y su resultado llegan por `BATALLA` (a quien combate) y por `FACCION` (resumida).
   - La columna queda `en_batalla` y sus órdenes dan `batalla.bloqueado` hasta que se aplica.
   - `asignacion_propia` (instancia y token) es para el cliente Unity: el cliente jugador no se conecta a la batalla, solo la sigue.
   - Referencia: Batalla_Unity_Ciclo §2-§8.
4. Las respuestas de `POST /v1/registro` y `POST /v1/sesiones` traen además `servidor` (cuenta de servidor de batalla): el cliente lo puede ignorar.

### Rescate de la rama de la revisión: fundar, plaza ajena desde dentro, escolta y recetas (2026-11-11, main ebfc4354)

Sin cambios en el contrato ni partida nueva: cambia lo que el motor manda y lo que acepta.

1. **La plaza ajena vista desde dentro.** Quien está dentro de una plaza ajena recibe en `VISTA` (`vista::plaza_desde_dentro`), además del trazado y las órdenes de mercado, **sus edificios en el suelo**: en pie, en obra y los dañados de la cola (la ruina que deja una conquista), cada uno con `completa_en_ms`, `mejora`, `danado`, `buey`, `ancho` y `alto`; y **sus recintos de muralla** (`recintos`: anillo, `avance` y mejora, sin `siguiente_celda_en_ms`). No llegan la cola sin empezar, la prioridad, la pausa, la fuente ni la receta. Qué haría el cliente: pintar esos edificios y murallas al recorrer la plaza ajena, como pinta los de la propia. Referencia: Niebla_De_Guerra §5.4, Proyecciones_Y_Autorizacion.
2. **La columna que funda entra en su plaza.** Al fundar, la columna ya no se queda fuera: una columna de un solo héroe entra como con `entrarEnAsentamiento`, y un ejército se desarma y cada héroe entra como lo haría solo. Cada entrada emite `jugador.entra_en_asentamiento`. Qué haría el cliente: tras `fundar`, llevar al jugador al interior de la plaza nueva en lugar de dejarlo en el mapa. Referencia: Fundacion_Y_Refundacion §5.
3. **Escolta del carro de acarreo.** El carro de acarreo admite **100** de escolta si la plaza madre no tiene Mercado (con Mercado, el cupo de su nivel); antes el motor daba 0. Qué haría el cliente: si calcula el cupo de escolta por su cuenta, dar 100 sin Mercado. Referencia: Comercio_Caravanas, el carro de acarreo («Batalla» y «Escolta»).
4. **Recetas con requisito de edificio.** Una receta con `requiereEdificio` (en `GET /v1/datos`, catálogo de Economia_Construccion) no se fabrica si la plaza no tiene ese edificio activo con ese nivel; hoy, el Arma de Hierro de Calidad pide una Fundición de nivel 3. Qué haría el cliente: en el taller, decir qué edificio le falta a esa receta. Referencia: Economia_Construccion §5.3.
