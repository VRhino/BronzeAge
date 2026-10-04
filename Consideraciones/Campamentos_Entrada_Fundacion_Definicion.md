# Campamentos como puerta de entrada y fundación por Caravana — decisiones y plan

> **Estado (2026-10-03): diseño en revisión, sin código. Consejo pasado; tres rondas de decisiones cerradas;
> calibración aceptada (§8). Quedan bots-héroe y alijos (§7).**
> Cambia cómo se entra al mundo, cómo nace una Facción nueva, de dónde saca recursos un héroe sin plaza y qué pasa
> con un jugador que no está conectado. Sustituye la «fundación a pie» (`Consideraciones/Entrada_Al_Mundo_Definicion.md`
> §6.5, `Jugador_Situado_Definicion.md` §5.9), amplía la refundación del paso 5 de mercenarios (Doc 1.9b) y rehace
> los campamentos de bandidos (Doc 1.9).
>
> Reglas de juego → irán a `Docs/Game/1` (§1.3, §1.8, §1.9, §1.9b), `Docs/Game/2` (Facción), `Docs/Game/3` §3.10 y
> §3.13 y `Docs/Game/5` (§5.13, batalla) cuando se cierre. Aquí solo decisiones, hallazgos y plan.

## 1. El enunciado del usuario (2026-10-02)

> Cuando arranca un servidor pasa todo igual, solo que al final de la creación se añade un paso: la aparición de
> campamentos de mercenarios regados por el mapa, más o menos según su tamaño. Cuando un héroe entra a la partida
> elige en cuál de los existentes aparece; se le muestra cuántos héroes eligieron ese campamento como inicial, para
> ver lo congestionado que está ese sector. Para fundar hay que: 1) ser parte de una Facción; 2) que entre sus
> miembros reúnan materiales para comprar la Caravana de Fundación en el campamento; 3) salir del campamento con
> ella, llegar a donde quieren fundar y llamar al comando `fundar` allí. La Caravana de Fundación, como las demás,
> debe poder ser atacada y escoltada por jugadores de su Facción.

## 2. Decisiones cerradas con el usuario

### 2.1 Primera ronda (2026-10-02)

| # | Decisión |
|---|---|
| D1 | **Campamentos iniciales** al final de la generación del mundo, repartidos **por separación mínima** entre ellos (valor a balancear), cantidad proporcional al área. *(Reformulada por D32.)* |
| D2 | ~~La aparición gradual se conserva.~~ **Derogada por D32.** |
| D3 | `crearHeroe` **elige campamento**: el héroe nace como residente de ese campamento. |
| D4 | La pantalla de elección muestra **cuántos héroes eligieron cada campamento como inicial**. **Solo informativo**: no hay tope. |
| D5 | Un héroe sin Facción puede **crearla** (queda como Rey sin plazas; ya es así en `crearFaccion`) **o ingresar** en una existente. |
| D6 | El ingreso es por **invitación o por solicitud**, con **visto bueno del Rey** (precisado por D31 y D46). |
| D7 | **Fundar a pie desaparece** (endurecida por D19: del todo). |
| D8 | La **expansión desde un asentamiento** (Caravana de Fundación de una plaza, Doc 1.8) **se conserva**. |
| D9 | Comprar la Caravana en un campamento: **solo una Facción SIN asentamientos**, al **75 %**, pagada desde el fondo de **esa Facción en ese campamento** (D39). |
| D10 | La caravana de campamento nace **sin destino**, parada en el campamento. **Se conduce enganchada** a la columna (Doc 5.13.2-3) y **se funda con el comando `fundar`** donde esté: **ya no es automática**. |
| D11 | **Solo el comprador** (titular) la engancha y llama `fundar`. |
| D12 | Los demás héroes de la Facción **escoltan uniéndose a la columna del titular**. |
| D13 | Si el titular muere, sale de la Facción o se desconecta, la caravana **intenta volver a su origen, el campamento** (D40), y **otro ciudadano puede reclamarla** enganchándola donde la alcance (pasa a ser titular). *(Corregida por D40: antes «se queda donde esté».)* |
| D14 | Si **nadie la engancha en un plazo** (propuesta: 48 h), **caduca y devuelve el fondo** a quienes aportaron, según el registro de D34; lo que no se pueda recibir se pierde (D43). |
| D15 | Destruida en camino: **se pierde**, sin cooldown para recomprar. |
| D16 | Punto válido para `fundar`: **fuera de zonas de influencia**, **fuera del radio de exclusión de un campamento** (§8.2), **no sobre agua**. |

### 2.2 Segunda ronda (2026-10-03, tras el consejo)

**Alcance y compatibilidad**

| # | Decisión |
|---|---|
| D17 | **Mundo nuevo.** El actual se borra al terminar la mecánica. Sin migración ni compatibilidad con snapshots anteriores. |
| D18 | **Los clientes pueden romperse.** BronzeAgeClient y Conquest se arreglan después para reflejar el motor. El contrato cambia sin transición. |
| D19 | **Fundar a pie se elimina del todo**: comando, código asociado y su uso en bots/batch. Sin flag ni modo transitorio. |
| D20 | **El plan de worldgen paralelo (`Docs/Arquitectura/11_...`) no existe para este trabajo.** Se diseña sobre el generador actual. |

**Fundación y Facción**

| # | Decisión |
|---|---|
| D30 | **Un solo mecanismo de fundación en el motor** (cierra C1): «fundar con una caravana en un punto». El comando `fundar` del titular lo llama; la caravana de una plaza que llega a su destino lo llama automáticamente. D8 y D10 son dos llamadores del mismo mecanismo. |
| D31 | **Solo el Rey aprueba** un ingreso (cierra C3). **`comprarCasa` se elimina** con todo su código asociado: no hay tercera puerta a la ciudadanía. |
| D32 | **Se elimina la aparición gradual de campamentos de mercenarios** (cierra C4). Solo existen los iniciales, colocados por separación mínima. |
| D34 | **La caravana registra quién aportó qué** al fondo gastado, para devolverlo si caduca (D14). |
| D35 | **Semilla derivada** para todo lo que se coloca al generar el mundo (campamentos, alijos): no consume el RNG compartido (P11). |
| D36 | **La caravana de campamento se puede atacar como cualquier otra.** Su **origen es el campamento** del que sale (corrige el defecto de dueño del §3.1). |

**Arranque económico (cierra P1)**

| # | Decisión |
|---|---|
| D21 | **Bandidos alrededor de los campamentos de mercenarios**, en **niveles de dificultad**: los altos exigen grupos mayores y dan más. Rendimiento por héroe y hora parecido en todos los niveles, con una prima pequeña por agruparse. Siempre sin abrir una mecánica explotable. *(Aparición precisada por D42.)* |
| D22 | **Los bandidos dan solo oro.** |
| D23 | **El mercado del campamento vende piedra y madera** además de lo que ya vende, con tope: **pila compartida del campamento + cupo diario por héroe** (D41). |
| D24 | **Ración de trigo gratis para el residente**: pequeña, pero suficiente para mover tropa a las inmediaciones y atacar bandidos. |
| D25 | **Tropa prestada por el campamento**: en el arranque, **solo a residentes** y **solo `leva_comunal`** (milicia de lanceros, leñadores, granjeros). **No come mientras vive en el campamento.** La deuda se cobra del botín. *(Naturaleza del préstamo: D45.)* |
| D25b | Si la tropa prestada muere, **se repone allí mismo con un coste bajo y asequible** (son unidades muy básicas). |
| D26 | **Rendimientos decrecientes por héroe**: el botín baja con cada campamento de bandidos destruido en las últimas 24 h. |
| D27 | **Botín de destino restringido**: el oro del botín solo puede ir al fondo de refundación o al mercado del campamento, nunca a la economía de una plaza. |
| D28 | **Reaparición de bandidos por demanda**: alrededor de un campamento, en proporción a sus residentes de Facciones sin asentamiento. |
| D29 | **Alijos de exploración**: escondites colocados al generar el mundo, ocultos por la niebla, de un solo uso y sin reaparición. **Mecánica aparte**: se define cuando esté cerrado todo lo demás, antes de la primera línea de código (D48). |
| D37 | **Bandidos unificados**: los de los asentamientos (Doc 1.9) pasan a ser **la misma entidad con niveles**, que salen **aleatorios**. |
| D38 | **Ritmo objetivo**: los **2 primeros días** son para aprender el combate del mundo. Pagar la caravana debe costar **unas 8 horas de juego a un grupo de 3 héroes**. Con esa cifra se calibra todo lo de P1 (§8). |

### 2.3 Tercera ronda (2026-10-03)

**Presencia: qué es de un jugador cuando no está** — regla **general** del mundo, no solo de la caravana.

| # | Decisión |
|---|---|
| D33 | **Desconectado** = el jugador cierra o pierde el cliente. Los jugadores **no están en el mundo mientras no están conectados**: al desconectarse, **su héroe y su tropa salen del mundo** desde donde estén y, al reconectar, **reaparecen donde quedaron**. *(Cierra C2/C5.)* |
| D33b | **En una batalla de Unity** (por tiempo), la desconexión es distinta: si reconecta antes de que la batalla caduque, **vuelve a entrar en ella**. Si la **abandona por su propia mano** (mecanismo nuevo, «abandonar batalla», **trabajo conjunto con Conquest**), se le aplica **derrota**. Si la desconexión es **involuntaria**, recibe **el resultado de su bando**: ganador si su equipo gana, perdedor si pierde. |
| D40 | **Las caravanas no son del jugador.** Cuando la gente que las conduce desaparece, **intentan volver a su origen** (Doc 3.10 se mantiene, y vale también para la de campamento: su origen es el campamento). |
| D40b | **Las unidades prestadas dejan de ser del héroe**: las cedidas a la **escolta de una caravana** y las de la **guarnición** de un asentamiento **se quedan donde están prestadas** cuando el héroe no está. |

**Economía del campamento**

| # | Decisión |
|---|---|
| D39 | **El fondo es de la Facción en un campamento** (cierra P4/M1). **Los materiales no se mueven mágicamente**: si están en otro campamento, alguien los lleva **en su carro** hasta donde está el fondo, y **al pagar la caravana se aporta desde el carro**. *(Excepción explícita a Doc 5.13, «el carro no se descarga en ruta»: solo para aportar al fondo.)* |
| D41 | **Tope del mercado = pila compartida + cupo por héroe.** Cada campamento tiene un stock de piedra y madera que se repone por hora y por el que compiten sus residentes; además, cada héroe tiene un cupo diario. La pila compartida hace que la congestión importe (un campamento lleno rinde menos por cabeza) y el cupo corta el acaparamiento. |
| D42 | **Bandidos del campamento**: aparecen **según la necesidad** (residentes de Facciones sin asentamiento), con **nivel aleatorio**, a **una distancia fija igual para todos los campamentos**, y **no demasiados a la vez**. Esa distancia es la que fija **cuánto trigo** necesita un héroe para ir a atacarlos. |
| D43 | **Devolución por caducidad**: lo que no se pueda recibir (almacén lleno, aportante muerto o que ya no es ciudadano) **se pierde**. |
| D44 | **Repostar trigo en cualquier campamento**: venden trigo por oro y, al ser neutrales, **cualquier héroe** puede repostar en ellos. La ración gratis del residente (D24) se mantiene aparte. |
| D44b | **La columna puede comer de la carga de la caravana** durante el viaje de fundación, pero **el valor de fundación queda bloqueado**: solo se come lo que exceda lo que la caravana necesita para fundar. *(Confirmada 2026-10-03.)* |
| D45 | **El préstamo no es reclutamiento.** Las unidades prestadas **nunca forman parte de la tropa del héroe** (de su «ejército» propio, no de una columna), **no ganan experiencia de tropa**, y el héroe solo las usa **para probarlas o cuando no tiene tropa propia**. **Al dejar de residir** en el campamento, el préstamo **deja de estar disponible**. Reclutar en el campamento (Doc 1.9b, paso 3) es otra mecánica, y esa sí da la tropa al héroe. |
| D46 | **Ingreso**: el Rey tiene **una lista de solicitantes** y **acepta o deniega**. Aceptado, entra en la Facción; denegado, no. |
| D47 | **Orden de los trabajos de diseño**: bots-héroe (§7) antes que los alijos (D29). |

### 2.4 Cuarta ronda (2026-10-03)

| # | Decisión |
|---|---|
| D48 | **Toda caravana en uso lleva la Facción que la usa** (`faccionId`). **Solo un ciudadano de esa Facción puede reclamarla (engancharla) y redirigirla**; a cualquier otro **solo le sale la opción de atacarla**. Antes bastaba deducirla del asentamiento de origen; ahora una caravana puede salir de un campamento donde conviven héroes de varias Facciones. *(`Caravana.faccionId` ya existe y `faccionDeCaravana` la prefiere; falta que sea obligatoria y que todo el motor la use en vez del origen — §3.1.)* |
| D49 | **No se puede entrar en una Facción NPC.** Se quita del canon y de la documentación, junto con la «vida dentro de una Facción NPC» (el huésped de `Entrada_Al_Mundo` §5, descartado; borrado de `Docs/Mecanicas a desarrollar.md` §16). La lista de solicitantes (D46) solo existe en Facciones de jugadores. |
| D50 | **El sobrante de la ración gratis vuelve al campamento**, no al almacén personal (cierra la explotación de §4.4). |
| D51 | **La calibración del §8 se acepta** como punto de partida (placeholder), a ajustar con batch. |

### 2.5 Quinta ronda: bots-héroe (2026-10-04)

| # | Decisión |
|---|---|
| D52 | **Todo el funcionamiento NPC (incluida la gobernanza) sale del motor y del tick**, a otro hilo o proceso de ejecución, y **actúa desde fuera como un jugador**. Motivo: ya ocupa mucho del tick con el comportamiento actual. Arquitectura: `Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md`. |
| D53 | **No hay Facciones NPC ya asentadas al arrancar.** Todo nace por el flujo de campamento → caravana → `fundar`, bots y humanos igual. Meterlas falsearía el comportamiento real del servidor. Se derogan Doc 1.3 «el servidor arranca con Facciones NPC ya asentadas» y el comando de admin `crearFaccionNpc`. |
| D54 | **Bots-héroe y humanos conviven en los campamentos y los bots cuentan como residentes** (pila del mercado, demanda de bandidos). De momento: así, retirarlos cuando lleguen los jugadores tiene poca fricción. |
| D55 | **Los bots tienen sesiones** (presencia, D33): unas horas al día en uno o dos bloques, por semilla; fuera de ellas salen del mundo como un humano. **Lógica fuera del motor.** |
| D56 | **Llegada escalonada** de bots a lo largo de los días, como llegarían los jugadores. **Fuera del motor.** |
| D57 | **Tres perfiles**: grupo de amigos (3 que llegan juntos; uno crea la Facción, los otros solicitan), solitario, tardío. En una Facción de bots, el Rey bot acepta las solicitudes. |
| D58 | **Batch nuevo**: N bots-héroe llegando escalonados en vez de 100 Facciones fundadas en el tick 0. La línea base y los diarios de batch anteriores quedan cerrados. **Se borra la estructura que no se use en el formato nuevo** (inventario en `Docs/Arquitectura/12` §7). |
| D59 | **Una Facción de bots que pierde su última plaza no se disuelve**: sus héroes vuelven a residir en un campamento y repiten el ciclo, como un jugador. |

## 3. Lo que ya existe y se reutiliza (medido en el código)

- `crearFaccion` (`session/comandos/crearFaccion.ts`): sin condiciones, el creador queda Rey. Cubre D5.
- `fondoDeFaccion`, `aportarARefundacion`, `retirarDeRefundacion`, `comprarCaravanaDeRefundacion`
  (`engine/refundacion.ts`): cubren D9 salvo que hoy la caravana lleva destino y ruta y **funda sola al llegar**
  (`avanzarCaravanasFundacion`), y que hoy se aporta desde el **almacén personal**, no desde el carro (D39).
- `adjuntarCaravana` / `soltarCaravana` (`engine/ejercitos.ts`): enganche en marcha, posición = la columna,
  pérdida si la columna cae o se disuelve (Doc 5.13.2-5.13.4).
- `unirseAEjercito` / `unirseEnCampo`: un héroe se une a la columna de otro.
- `fundarAsentamiento` (`session/comandos/fundarAsentamiento.ts`): funda **donde se está** con la columna del actor
  y la deshace dentro (`cruzarLaPuerta`); `exigirPuertaDeFundacion`.
- `campamentoInicial` (`engine/mercenarios.ts`): uno en el centro. `avanzarAparicionMercenarios` se elimina (D32).
- `avanzarSpawnBandidos` / `avanzarAtaquesBandidos` (`engine/bandidos.ts`) y `atacarCampamentoConColumna`
  (`engine/combate.ts`): uno por asentamiento, poder fijo 30, botín 40 madera + 20 piedra + 15 oro, reaparición
  60 min. Base de D21/D37/D42.
- `leva_comunal` (Doc 6, de arranque): milicia de lanceros, leñadores, granjeros. Es lo que presta D25.
- `enRefugio` (`engine/zones.ts`): precedente para la protección dentro del campamento (M4).
- `MERCENARIOS.mercado` (`constants.ts`): ya hay pila por bien (`topePorBien` 300, reposición cada 3 h), margen
  ×1,3. Es la base de D41; falta el cupo por héroe.

### 3.1 Hallazgos del consejo en el código

- **La caravana de campamento no tiene dueño para el motor.** `resolverEncuentros` (`ejercitos.ts:1800`) y
  `adjuntarCaravana` (`ejercitos.ts:614`) lo deducen de `origenAsentamientoId`; con el campamento como origen no
  encuentran plaza, así que nadie puede interceptarla ni engancharla. Hay que revisar **cada sitio que deduce la
  Facción de una caravana por su origen** y pasar a `faccionDeCaravana` (hoy privada en `expansion.ts`: exportarla).
  D36 y D48 lo exigen. «Volver al origen» (D40) también tiene que saber ir a un campamento.
- **M4 no es un solo guard.** Un combate se abre por cuatro vías: `encuentroEntreEjercitos`,
  `interceptarCaravanaConEjercito`, el ataque de bandidos y `abrirBatalla`.
- **El batch no tiene bots-héroe que pasen por `crearHeroe`.** Solo lo llaman la sesión y el servidor (§7).
- **`engine/refundacion.ts` tiene cambios sin commit** (el gasto pasa a ir por ciudadanos, no por residentes).
  Cerrar antes de reabrir.

## 4. Revisión propia: contradicciones, puntos ciegos y mejoras

### 4.1 Contradicciones — todas cerradas

- **C1** → D30 (un mecanismo, dos llamadores).
- **C2** → D33 + D40: el héroe sale del mundo; la caravana vuelve a su origen, como dice Doc 3.10.
- **C3** → D31 + D46: solo el Rey, con lista de solicitantes; `comprarCasa` desaparece.
- **C4** → D32: sin aparición gradual.
- **C5** → D13 + D33: un no-titular reclama cuando el titular muere, sale de la Facción o no está en el mundo.

### 4.2 Puntos ciegos

- **P1 — cerrado** por D21-D29, D37, D38, D41, D42, D44, D45. Oro de bandidos (y alijos); piedra y madera del
  mercado; trigo de la ración y del mercado; arranque con tropa prestada.
- **P2 — M4**, alcance en §3.1, radio en §8.2.
- **P3 — M3**, contador doble (§8.3).
- **P4 — cerrado** por D39.
- **P5 — cerrado** por D34 + D43.
- **P6 — M2**, cofundadores (§8.3).
- **P7 — cerrado** por D36.
- **P8 — se resuelve calibrando** con D38 (§8).
- **P9 — radio en §8.2.** La comprobación de D16 va en el motor y vale para los dos llamadores (D30).
- **P10 — cerrado** por D17-D18.
- **P11 — cerrado** por D35.

### 4.3 Mejoras

- **M1.** ~~Fondo por Facción~~ — descartada por D39.
- **M2.** Cofundadores = héroes de la columna del titular (§8.3).
- **M3.** Contador doble (§8.3).
- **M4.** Protección del residente dentro del radio del campamento: nadie inicia combate ahí (§8.2).
- **M5.** Registrar el reparto del fondo gastado — aceptada como D34.

### 4.4 El coste de movimiento (hallazgo del usuario, 2026-10-03)

Una columna en marcha come de su carro: **0,15 trigo por soldado y minuto** (`MILITAR.racionPorSoldadoPorMinuto`),
una décima parte acampada (Doc 5.13). Consecuencias:

- **La distancia de los bandidos fija el trigo** (D42). Agruparse escala el alcance solo: 5 héroes llevan 5 carros.
- **Se reposta en cualquier campamento** comprando trigo (D44), además de la ración gratis del residente (D24).
- **Explotación del sobrante**: el carro devuelve lo que sobra al almacén de origen. Si ese almacén es el personal,
  la ración gratis se acumula hasta pagar el trigo de la caravana. **El sobrante de la ración gratis debe volver al
  campamento**, no al almacén personal (D50).
- **La tropa prestada come en marcha** (D25 solo la exime en el campamento).
- **El viaje de fundación también come**, y puede comer de la caravana por encima de su valor de fundación (D44b).
- **El trigo comprado puede hacer que cazar bandidos dé pérdidas** — ver §8.1: es la restricción que más pesa en la
  calibración.

## 5. Plan (provisional, se reordena al cerrar §7)

**Diseño pendiente antes del código:** §7 y §8 cerrados → bots-héroe (§7) → alijos (D29).

0. Cerrar los cambios sin commit de `engine/refundacion.ts`.
1. **Campamentos iniciales** por separación mínima, con semilla derivada; eliminar la aparición gradual.
2. **`crearHeroe` con `campamentoId`**, contador doble y proyección para la pantalla de elección; **protección
   dentro del campamento** (M4, las cuatro vías) en el mismo paso.
3. **Economía del residente**: ración de trigo + reposte comprando en cualquier campamento; préstamo de
   `leva_comunal` (D45) con reposición barata y deuda; piedra y madera en el mercado con pila + cupo (D41).
4. **Bandidos unificados por niveles** (D21, D22, D26-D28, D37, D42) y **alijos** (D29).
5. **Ingreso con lista de solicitantes del Rey** (sustituye `unirseAFaccion`); **eliminar `comprarCasa`**.
6. **Fundación única** (D30): caravana de campamento sin destino, origen = campamento (atacable, D36; vuelve a él,
   D40); fondo por Facción y campamento, aportado desde el carro (D39); enganche y reclamo; caducidad con registro
   (D34, D43); la caravana de plaza llama al mismo mecanismo; cofundadores (M2); D16 en el motor; **eliminar fundar
   a pie** (D19).
7. **Presencia** (D33, D40b): salir del mundo al desconectar y reaparecer al reconectar; las unidades prestadas se
   quedan. En batalla (D33b): reentrada, «abandonar batalla» y resultado del bando, **con Conquest** (protocolo
   BA/CQ).
8. **Bots-héroe** que arranquen como los jugadores y calibración en batch con D38.
9. Canon (`Docs/Game`), contrato, clientes. En el canon, además: quitar `comprarCasa` y el ingreso en Facciones NPC
   (Doc 2, «Cambiar de residencia»: «usa comprar casa o unirse a una Facción»; `Docs/Arquitectura/5` fila de
   `unirseAFaccion`).

## 6. Revisión por consejo (2026-10-03)

Cinco consejeros (contrarian, primeros principios, expansionista, outsider, ejecutor), revisión cruzada anónima y
síntesis. Veredicto: **no estaba listo**; todos coincidieron en que P1 bloqueaba el ciclo entero (el jugador
elegía campamento y se quedaba sin salida).

- **Coincidencias**: P1 era el núcleo y faltaba en el plan; la protección del campamento iba tarde (paso 6 tras el
  spawn del paso 2); «desconectado» no existía en el motor; M1 y M2 con apoyo unánime.
- **Choques**: cómo resolver P1 (kit inicial frente a contratos con NPC); C1 (asimetría frente a unificar) — se
  impuso «un mecanismo, dos llamadores», adoptado como D30; cómo romper el contrato — irrelevante tras D18.
- **Lo que salió en la revisión cruzada**: el mundo vivo a migrar (resuelto por D17); con 5 jugadores repartidos en
  4+ campamentos nadie junta grupo (D1 contra D9) — sigue siendo un riesgo del playtest (§8.2); los hallazgos de
  código del §3.1.

## 7. Puntos abiertos

1. **Arquitectura de NPC fuera del motor (D52)**: `Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md`.
2. **Alijos (D29)**: contenido, cantidad y ubicación. Después de los bots-héroe.

## 8. Calibración (2026-10-03, aceptada como punto de partida — D51)

Todo PLACEHOLDER, derivado de constantes que ya existen. Mapa 2000×2000; velocidad ligera 20 u/min; ración
0,15 trigo por soldado y minuto (0,015 acampado); carro 500.

### 8.1 Cuánto cuesta la caravana y de dónde sale el oro

`costoCaravanaFundacion()` = 160 madera + 20 piedra + 100 trigo + 100 oro. Al 75 %: **120 madera, 15 piedra,
75 trigo, 75 oro.** Comprado todo en el mercado del campamento (precio base × 1,3):

| Recurso | Cantidad | Oro |
|---|---|---|
| Madera | 120 × 1,3 | 156 |
| Piedra | 15 × 1,56 | 23 |
| Trigo | 75 × 1,95 | 146 |
| Oro | 75 | 75 |
| **Total** | | **≈ 400 oro** |

D38 (8 h × 3 héroes = 24 héroe-hora) → **≈ 17 oro netos por héroe y hora**, con los gastos ya descontados (trigo de
las salidas, reposición de la tropa prestada, deuda).

**La restricción que más pesa: el trigo de cada salida.** Con un escuadrón prestado de 15 y bandidos a 200 de
distancia, una salida son 20 min de marcha (15 × 0,15 × 20 = **45 trigo**) más el combate acampado (poco). Si ese
trigo se compra a 1,95, cada salida cuesta **≈ 88 oro**: ningún botín razonable lo cubre y cazar bandidos da
pérdidas. Por eso **la ración gratis (D24) tiene que cubrir las salidas de nivel 1** y el trigo comprado (D44)
queda para lo que la exceda: niveles altos, más gente, viaje de fundación.

Propuesta:

- **Ración gratis**: **60 trigo** por residente, que se rellenan **cada 30 min** en su campamento (una salida de
  nivel 1 con margen). Lo que sobra vuelve al campamento (D50). No acumulable.
- **Ciclo de una salida de nivel 1**: 10 min de ida + combate (≤ 15 min en Unity, inmediato con números) + 10 min
  de vuelta ≈ **30 min → 2 salidas por hora**.
- **Botín** (oro, D22), por héroe y salida, con D21 (prima pequeña por agruparse):

  | Nivel | Héroes | Poder | Oro por héroe |
  |---|---|---|---|
  | 1 | 1 | 20 | 9 |
  | 2 | 2-3 | 60 | 10 |
  | 3 | 4-5 | 120 | 11 |

  2 salidas/h × 9 = 18 oro/h, menos deuda y reposiciones ≈ **17 netos**: cuadra con D38.
- **Rendimientos decrecientes (D26)**: botín completo en las **8 primeras** destrucciones de las últimas 24 h; luego
  −15 % por cada una; **a partir de la 14ª, solo experiencia.** Las 8 h de D38 repartidas en 2 días (≈ 4 h/día,
  8 salidas) quedan dentro del tramo completo; el que juega 12 h seguidas no va 3 veces más rápido.
- **Tropa prestada (D25, D45)**: **15 milicias de lanceros** (poder ≈ 30) contra un nivel 1 de 20: gana casi
  siempre, con bajas. **Reposición: 1 oro por unidad** (lo mismo que un escalón 1 de `RECLUTAMIENTO_ORO_POR_ESCALON`).
  **Deuda**: el préstamo en sí no cuesta; solo las reposiciones, descontadas del botín. Tope de deuda **30 oro**:
  por encima, el campamento no repone hasta que se salde.
- **Mercado (D41)**: pila compartida de **300 madera y 60 piedra** por campamento (reposición cada 3 h, como ya
  hace `MERCENARIOS.mercado`). Cupo por héroe y día: **60 madera y 10 piedra**. Un grupo de 3 junta en un día
  180 madera y 30 piedra: más de lo que pide la caravana, pero no puede vaciar la pila de un campamento lleno.

### 8.2 Radios y separación

- **Radio del campamento (M4, protección)**: **60**. Dentro, nadie inicia combate. Cubre el layout del campamento y
  su entorno inmediato; es menor que la vista de un héroe solo (80): se ve venir a quien espera fuera.
- **Radio de exclusión para fundar (D16)**: **100** = 60 de protección + 30 de `ZONA_INFLUENCIA.radioInicial` +
  10 de margen. La zona de una plaza nueva no puede nacer pisando el campamento.
- **Distancia de los bandidos (D42)**: **anillo de 150 a 250** alrededor del campamento, igual para todos. Fuera de
  la protección, a 10 min de marcha a velocidad ligera (con el trigo de §8.1).
- **Cuántos a la vez (D42)**: **1 por cada 2 residentes de Facciones sin asentamiento**, mínimo 1, máximo 6 por
  campamento. Reaparición por demanda (D28): cuando hay menos de los que tocan, aparece uno cada **10 min**.
- **Separación entre campamentos (D1)**: **≥ 600** = dos anillos de bandidos (2 × 250) + 100 de margen, para que
  los anillos no se pisen. En 2000×2000 salen **≈ 6-9 campamentos.**
  **Riesgo del playtest (consejo)**: con 5 jugadores en 6-9 campamentos, cada uno queda solo y nadie junta grupo.
  Para el playtest, **separación 900 → ≈ 4 campamentos**, y subir hacia 600 cuando haya población.

### 8.3 Contador doble y cofundadores

- **M3**: la pantalla de elección muestra **dos números** por campamento: **eligieron como inicial** (D4, crece
  siempre) y **residentes ahora** (cuántos viven allí en este momento). El primero dice la historia; el segundo, la
  congestión real. Con D41 (la pila compartida), el segundo es el que importa para decidir.
- **M2**: los **héroes que estén en la columna del titular al llamar `fundar`** son los cofundadores, hasta
  `FUNDACION.maxJugadoresFundacionGrupal` (5). Unirse a la columna ya es el consentimiento (D12).

## 9. NPC fuera del motor (D52)

Movido a su propio documento: `Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md` (2026-10-04). Es un cambio de
infraestructura que va más allá de los campamentos; los bots-héroe (D53-D59) se construyen ya sobre él.
