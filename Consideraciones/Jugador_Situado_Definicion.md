# El jugador como entidad situada en el mundo — cierre de diseño y plan de ejecución

> **Estado (2026-09-06): DISEÑO CERRADO, canon escrito, sin implementar.** Treinta y seis decisiones con el usuario en once rondas, las
> derivadas resueltas y el plan escrito. Ni una línea de motor todavía — mismo criterio que
> `Murallas_Definicion.md` y `Movimiento_Ejercitos_Definicion.md`: la especificación ANTES de tocar el código.
>
> La **segunda ronda (§1.1b)** cambió el modelo de interacción: nada se dispara por proximidad, la proximidad
> abre un menú. Eso toca canon vigente (§1.1c) y saca el comercio con plaza ajena a mecánica propia (§1.1d).
> La **tercera (§1.1e)** separó al jugador solo del ejército, y la **cuarta (§1.1g)** corrigió dónde cae esa
> línea: no es el recuento de participantes, es de dónde viene la columna. Tabla en §1.1f, al canon tal cual.
> La **quinta (§1.1h)** cerró el último bloqueante: el héroe combate con el poder de una unidad de élite.
>
> **Las REGLAS DE JUEGO de esta mecánica NO viven aquí: viven en `Docs/Game/`**, que es el canon — y ya
> están escritas ahí (2026-09-06). Este documento cubre solo lo que no es regla de juego: qué se decidió y
> cuándo, cómo se representa en el motor, qué hay que tocar, en qué orden y qué se congela en tests.
>
> | Dónde | Qué se escribió |
> |---|---|
> | **Doc 0** (glosario) | *Jugador* situado y combatiente · **Columna personal** (entrada nueva) · *Ejército* como identidad y de una sola Facción · **Líder** y **Tregua** (entradas nuevas) |
> | **Doc 1** | §1.3 el spawn es literal y se funda donde se está · **§1.10 nueva**: presencia, salir, entrar, capa pública, la puerta del Gobernador, desconexión |
> | **Doc 2** | §2.5 la ciudadanía habilita, **la presencia ejerce** |
> | **Doc 3** | §3.10 una caravana sola se defiende de un jugador solo |
> | **Doc 5** | §5.1 el héroe combate · §5.2.2 el choque se declara · §5.12.1 columna vs ejército · §5.12.3 la geometría ofrece, los tres anillos, persecución y tregua · §5.12.4 llegar ya no es asediar · §5.12.6 rectificar vs cancelar · §5.12.7 el anillo de inspección · **§5.14 nueva**: unirse y separarse en campo |
>
> Cierra DOS entradas de `Docs/Mecanicas a desarrollar.md` —§13b (el jugador como entidad situada) y §14
> (movimiento libre)— porque son la misma: §14 es el verbo de §13b. Y desbloquea una tercera, el **spawn
> aleatorio de onboarding** del checklist, que es su primer minuto de juego.

## 0. El punto de partida

Lo que el enunciado pedía (§13b): el jugador es partícipe del mundo, no un ente volador superior. Tiene
ubicación. Nace en mundo abierto en un punto aleatorio, funda un asentamiento y entra en él, sale al mundo,
se mueve, entra en otros. Ve el mapa de lo conocido desde cualquier parte, pero **nunca el interior de un
asentamiento en el que no esté dentro**. El Gobernador puede prohibir la entrada a neutrales o enemigos.

Lo que había en código el 2026-09-06:

| Pieza | Estado |
|---|---|
| `Jugador` (`domain/types.ts`) | `{ id, liderazgoBase }`. Sin posición. Y el registro es **opcional**: quien no aparece en `jugadores` usa `LIDERAZGO.base` |
| `proyectarParaJugador` | Manda la Facción **entera**: todos los asentamientos propios completos (almacén, escuadrones, colas, cargos, trazado urbano) |
| `Ejercito` | Ya tiene `ruta`, `progreso`, `posicionActual`, `estado`, suministro y velocidad. `avanzarEjercitos` ya lo mueve cada tick, ya resuelve encuentros, asedios e intercepciones |
| Niebla de guerra | Ojos = plazas (`radio + 60`) y ejércitos (`150`). Memoria **por Facción** |
| `fundarAsentamiento` | Acepta una `posicion` arbitraria: hoy se funda a dedo sobre el mapa, desde ninguna parte |
| Autorización | 42 comandos, ~24 con `asentamientoId`. Exigen **residencia o cargo**, nunca **presencia** |

Tres hallazgos que condicionaron todo lo que sigue:

1. **La proyección vigente es exactamente lo que §13b prohíbe.** No falta código: sobra alcance. Se reescribe.
2. **Doc 5.12.1 ya dice "salir solo es un ejército de un participante"** — pero `participantesDe` deriva los
   participantes de los ESCUADRONES. Un jugador sin tropas cuenta 0 participantes, `velocidadDeEjercito`
   devuelve 0 (no se movería), `avanzarRacion` consume 0 (§14 pide "casi nada", no nada) y Doc 5.13.4
   disolvería la columna por vacía. El caso mínimo del doc de ejércitos **nunca se implementó de verdad**.
3. **Un jugador sin Facción no graba nada** (Doc 5.12.8, explícito: la memoria es de la Facción). El spawn
   aleatorio cae justo en ese agujero: explorarías a ciegas y sin memoria hasta fundar.

## 1. Decisiones

### 1.1 Cerradas con el usuario (2026-09-06)

1. **La regla "solo ves donde estás" es de VISIÓN y de ACCIÓN.** Estar dentro es requisito para construir,
   reclutar, comerciar, activar políticas y designar cargos ahí. **Lo ya ordenado sigue corriendo solo**
   (auto-construcción, colas, caravanas en ruta, producción): salir no congela tu ciudad, te impide darle
   órdenes nuevas. Es la lectura fuerte de "partícipe, no ente volador": con acción remota el jugador seguiría
   siendo un ojo volador con venda.

2. **En un asentamiento AJENO se ve solo la capa pública**: trazado, edificios visibles y mercado. Nunca
   almacén exacto, guarnición, colas de construcción ni cargos. Entrar es reconocimiento legítimo —ves si la
   plaza es grande, rica y está amurallada— y cerrar la puerta sigue siendo una defensa real sin que abrirla
   sea suicida.

3. **El flujo de salida, literal del usuario:** el jugador está dentro del asentamiento que tiene su base, y
   ahí tiene su roster entero de tropas. Al salir aplica la MISMA mecánica de Liderazgo para elegir con qué
   sale: una sola tropa, todas las que el Liderazgo permita, o **ninguna**. En la misma pantalla decide con
   qué materiales sale, hasta llenar el carro. Pulsa salir y aparece en el mapa de mundo **junto al
   asentamiento**. (Anotado por el usuario para futuro: el **Tesorero** podrá fijar cuánto material del
   asentamiento puede retirar un jugador — gancho, no alcance de esta pasada.)

4. **De un asentamiento que NO es el suyo se sale sin ninguna interfaz**: se sale con lo que se lleva encima,
   tropas y recursos incluidos.

5. **El movimiento es clic a destino, y por eso Fase 0 y Fase 1 comparten modelo.** Esta decisión se cerró en
   dos tiempos y conviene dejar las dos mitades, porque la segunda abarata mucho la mecánica:

   - **Primera respuesta:** "quiero que sea un avatar en tiempo real, vamos a irlo dejando marcado". Se anotó
     como frontera de Fase 1 y como proyecto aparte, entendiendo control continuo.
   - **Aclaración del usuario, el mismo día:** *"no es tipo WoW, es una miniatura que se va moviendo por un
     mapa: doy clic en un punto y la miniatura se va moviendo hasta llegar al destino"*. Eso **no es netcode**
     y no necesita otro reloj — ver §7. Es el mapa de campaña estilo Total War del `Roadmap_Escalado.md`
     (Fase 1), no la integración continua de la Fase final.

   **Consecuencia:** Fase 0 y Fase 1 no tienen dos movimientos distintos con una migración entre medias.
   Tienen el mismo, con distinta calidad de dibujo. §7 fija qué respetar para que el salto sea solo eso.

6. **Al desconectarse, el jugador DESAPARECE del mundo en el punto donde quedó, y reaparece ahí al volver.**
   No es una excepción caprichosa: es la regla que hace jugable un mundo persistente para una persona sola, y
   es además el comportamiento estándar de la capa de avatar que llegará en Fase 1.

7. **A quien le roban en campo abierto pierde carga y queda en tregua un rato.** Una ventana de 5 minutos que
   impide ordeñar al mismo viajero cada minuto mientras huye. *(Cuánto pierde y qué más implica la tregua lo
   precisa la decisión 18: **la mitad** del carro, y la tregua le impide también atacar.)*

   **Ampliada en la segunda ronda (decisión 11):** ya no es solo "a quien le roban" ni solo del viajero — es
   un debuff **de derrota**: quien pierde un choque queda en TREGUA durante esa ventana, y mientras la
   arrastre **nadie puede iniciar una persecución contra él**. Deja de ser una regla del viajero solo para
   ser la regla que evita el acoso en cadena.

   *(Lo que esta decisión decía sobre que el jugador no combate individualmente **se retiró**: ver §9, el
   usuario corrigió que un jugador solo SÍ puede combatir y eso está sin cerrar.)*

### 1.1b Segunda ronda (2026-09-06): las interacciones dejan de ser automáticas

Seis decisiones más, todas del mismo cambio de modelo: **la geometría deja de decidir y pasa a ofrecer**.

8. **Nada se dispara por proximidad: la proximidad abre un MENÚ.** Al acercarse, el cliente ofrece las
   acciones disponibles y el jugador elige. Vale para todo, bandidos incluidos — el menú de un campamento
   solo ofrece *atacar*, y solo a distancia de combate.

   | Sobre qué | Opciones |
   |---|---|
   | Ejército ajeno | *Inspeccionar* (qué tropas lo forman y de quién son) · *Perseguir* |
   | Caravana ajena o neutral | *Inspeccionar* (si lleva escolta y **qué** recursos carga, nunca cuántos) · *Interceptar* |
   | Asentamiento (en la puerta) | *Entrar* · *Asediar* · *Comerciar* · *Consultar* |
   | Campamento de bandidos | *Atacar* (solo a 15) |

   **Esto NO deshace el Paso 11 del doc de ejércitos**, que retiró `combateCampoAbierto` e
   `interceptarCaravana` como comandos. Aquellos resolvían **desde cualquier sitio contra una defensa fija**
   —atacar desde el sofá— y por eso se fueron. Lo que entra aquí es INTENCIÓN, y la resolución la sigue
   haciendo la geometría: hay que estar delante.

9. **Los tres radios, y cada uno significa una cosa distinta.** Es el corazón del rediseño:

   | Distancia | Qué habilita | Quién se entera |
   |---|---|---|
   | **150 / 80** (columna / viajero solo) | Ves que hay algo y de quién es | Nadie |
   | **40** (`radioInspeccion`, NUEVO) | Puedes inspeccionar: composición, nombres | **El inspeccionado recibe aviso** |
   | **15** (`radioEncuentro`) | Se cierra la persecución y se ofrece atacar | Los dos, obviamente |

   Los 40 son la respuesta del usuario a la reversión del canon de Doc 5.12.7 (ver §1.1c): no se revierte del
   todo, se pone **a media distancia**. Y el aviso al inspeccionado es lo que convierte el reconocimiento en
   un juego de dos: **el explorador puede huir** porque es más rápido que una columna entera y porque no ha
   tenido que meterse hasta los 15. Mirar cuesta ser visto mirando.

10. **La persecución es un estado, y termina de cuatro formas.** Perseguir fija un objetivo móvil en vez de
    un punto; la ruta se recalcula hacia donde esté. Acaba cuando:

    - se llega a **15** — la persecución cumplió, y ahí se ofrece atacar;
    - el perseguidor **cambia de destino** (un clic en otra parte la cancela);
    - el objetivo entra en tregua por haber sido derrotado (decisiones 11 y 18);
    - o no llega a empezar, porque el objetivo ya arrastraba ese debuff.

    Para **inspeccionar** no hace falta perseguir: basta con verlo, dentro de los 40.

11. **El debuff de derrota** — ver la decisión 7, ampliada arriba.

12. **Un viajero solo no puede llevar caravanas adjuntas.** Cierra de raíz al mercader unipersonal que sale
    sin tropas, engancha la flota y cruza el mundo a 22 sin nada que perder salvo carga ajena.
    *(Precisado por la decisión 20: quien no puede es la **columna personal**. Un ejército reducido a un solo
    miembro conserva las suyas — el criterio es de dónde viene la columna, no cuántos van dentro.)*

13. **Las caravanas ajenas y neutrales solo se ven dentro del radio de visión.** Fuera de él están ocultas —
    *"eso es el deber ser"*. Sin memoria, como los campamentos de bandidos y por la misma razón, solo que más
    fuerte: una caravana **se mueve**, así que una foto vieja no diría "aquí hubo una", diría una mentira
    sobre dónde está ahora.

### 1.1e Tercera ronda (2026-09-06): un participante o varios

Tres decisiones que, juntas, convierten el número de participantes en la **línea que separa dos formas de
jugar** — y no en un detalle de conteo.

14. **El destino solo lo rectifica quien va solo.** *"Es libre de decidir al estar solo; los ejércitos van
    varios jugadores juntos, así que simplificamos: destino desde el origen, no se puede modificar el destino
    de un ejército."* Doc 5.12.6 **se queda intacta** para los ejércitos: cancelar sigue siendo volver.
    *(Precisado por la decisión 20: `marcharA` es un comando de **columna personal**, no de "columna con un
    participante" — un ejército con un solo miembro sigue con su ruta fija.)*

15. **Un jugador puede unirse en campo a un ejército con el que se cruza.** Desde el menú de interacción,
    igual que inspeccionar o perseguir. Aporta lo que YA lleva encima —sus escuadrones y su carro—, no tropas
    traídas de casa.

16. **Y un jugador que va en un ejército puede separarse** y seguir por libre desde donde esté.

**Lo que sale solo de las tres juntas, y es lo más interesante:** unirse a un ejército **cuesta la libertad
de movimiento**. Entras y adoptas un destino que ya no puedes cambiar; te separas y la recuperas al instante.
Ese es el precio de marchar acompañado, y no hubo que escribir ninguna regla para tenerlo — sale de la 14.

**La otra cara, dicha en voz alta antes de que alguien la descubra como bug:** "destino inmutable" es un
**compromiso, no una cárcel**. Un grupo que quiera cambiar de rumbo puede separarse, moverse y volver a
unirse. Y está bien que se pueda: lo que hace inmutable al destino es que fue una decisión COMPARTIDA, así
que cambiarla exige que todos vuelvan a actuar. La regla no se burla, se paga.

### 1.1g Cuarta ronda (2026-09-06): ser ejército es una identidad, no un recuento

Cuatro decisiones más. La 20 corrige una lectura equivocada que tenía este documento, así que va con su
corrección explícita.

17. **Los neutrales no pueden unirse a un ejército.** ~~Solo misma Facción o **aliados**, y el aliado entra
    como **invitado**.~~ **La mitad de los aliados la ANULA la decisión 33**: solo misma Facción, y la figura
    del Invitado desaparece del diseño. Lo que sobrevive intacto es el veto a los neutrales.

    Se conserva escrita porque de ella cuelgan las decisiones 24, 31 y buena parte de §9, y borrarla dejaría
    esas referencias colgando de la nada.

18. **A quien roban pierde la MITAD del carro, y el debuff corta por los dos lados.** Tres precisiones sobre
    la decisión 7, que se queda corta:

    - Se pierde **la mitad** del contenido del carro, no todo.
    - **Carro vacío = solo el debuff**, y ya está. No hay nada que quitar.
    - **El debuff también le impide a él atacar o perseguir**, no solo protegerle. Esto no estaba dicho y es
      lo que impide usar la inmunidad como escudo: cinco minutos intocable serían cinco minutos de barra
      libre para depredar sin riesgo.

    Y una aclaración de vocabulario del usuario: se habla del **carro personal** del jugador (Doc 5.13), no
    de caravanas — que un viajero solo no puede llevar (decisión 12).

19. **Un ejército solo se origina en un asentamiento, nunca en campo abierto.** Dos viajeros que se cruzan en
    el camino no forman un ejército; lo que sí puede hacer uno es **unirse a un ejército que ya existe**
    (decisión 15). El origen de un ejército es siempre una plaza.

20. **Y una vez formado, sigue siendo un ejército aunque se quede en uno.** El ejemplo del usuario, que es la
    regla entera:

    > Un ejército de tres con caravanas. Se separa el primero: quedan dos, sigue. Se separa el segundo: queda
    > **uno solo, y sigue con su ruta preestablecida y sus caravanas, porque sigue siendo un ejército**. El
    > último intenta separarse y **el juego no se lo permite** —se perderían las caravanas—: para deshacerlo
    > tiene que **cancelar** y que el ejército vuelva.

    Lo que esto fija: **una columna nunca se queda sin participantes en campo abierto.** Separarse solo es
    posible mientras quede alguien; al último no le queda otra que replegarse. Las caravanas son la
    justificación que dio el usuario, pero la regla protege lo mismo para todo lo que va dentro —escuadrones,
    suministro, adjuntas—: nada se abandona en mitad del mapa por vaciar una lista.

### 1.1h Quinta ronda (2026-09-06): el héroe combate, y cuánto pesa

21. **El poder de un héroe solo equivale al de UNA unidad de élite.** Cierra el punto que este documento tenía
    marcado como bloqueante desde la primera corrección del usuario ("un jugador solo sí puede combatir"), que
    contradecía Doc 5.1.

    **Se DERIVA, no se escribe a mano** — mismo criterio y mismo motivo que el coste de Liderazgo
    (`Movimiento_Ejercitos_Definicion.md` §1.3): `poderBase` sigue siendo placeholder pendiente de calibrar
    (Doc 5.8), así que un 15 a mano se desincronizaría del catálogo en cuanto alguien lo toque.

    ```ts
    /** Poder de combate de un jugador POR SÍ MISMO (decisión 21): una unidad de la tropa de élite. Derivado
     * del catálogo, no escrito: hoy son 15 (arqueros con arco compuesto, escalón 5). */
    poderHeroe: () => Math.max(...TROPAS_RECLUTABLES.map((t) => t.poderBase)),
    ```

    **Qué significa ese número en la práctica**, que es lo que decide si la cifra está bien:

    | | Poder |
    |---|---|
    | Un héroe solo | **15** |
    | El peor escuadrón completo (milicia, 25 × 2) | 50 |
    | Un escuadrón de élite (arqueros compuesto, 12 × 15) | 180 |

    O sea: **un héroe vale menos de un tercio de la peor leva**. Contra una columna no es un combatiente, es
    una anécdota — y así debe ser. Donde sí decide es en los dos combates que son suyos: contra otro héroe
    solo, y contra una **caravana sin escolta**. Esto último abre un rol que el juego no tenía y conviene
    verlo venir: **el viajero solitario como salteador de caravanas**, rápido (22), barato de mantener y con
    poco que perder. Es coherente con la ficción y con la tregua de la decisión 18, pero hay que calibrarlo
    mirando, no darlo por bueno.

    **Y qué NO cambia:** al perder, el héroe no muere ni sufre bajas —pierde media carga y entra en tregua
    (decisiones 7 y 18), que es exactamente lo que gana quien le vence—. `aplicarBajas` no le toca, no queda
    herido y no gana veteranía: la veteranía es del escuadrón (Doc 5.8), y la progresión del jugador es otra
    mecánica (`Docs/Mecanicas a desarrollar.md` §11).

    **Dos consecuencias en el código**, ninguna grande:

    - `resolverCombate` **rechaza hoy un atacante con la lista de escuadrones vacía**. Deja de poder hacerlo:
      un atacante sin escuadrones pero con un participante es legítimo. La guarda pasa a ser "sin escuadrones
      **y** sin participantes".
    - `poderTotal` gana el poder de los participantes además del de los escuadrones. Con ello, un ejército de
      N jugadores suma `N × poderHeroe` — marginal frente a la tropa, y a propósito: los participantes ya
      contaban como carros y como rombos, ahora cuentan también, un poco, como espadas.

22. **Una caravana sola se defiende de un jugador solo.** Respuesta inmediata del usuario al rol de salteador
    unipersonal que abría la decisión 21. **Un jugador solo no roba caravanas**: depredar sigue exigiendo tropa.

    La defensa base de caravana **ya existe** (`MILITAR.defensaBaseCaravana`, Doc 3.10) — no se reintroduce
    nada, se **recalibra**, porque hoy vale exactamente 15, lo mismo que el héroe, y el duelo sería a cara o
    cruz. Y se **deriva**, por el mismo motivo que `poderHeroe`:

    ```ts
    defensaBaseCaravana: () => poderHeroe() * 1.7,   // hoy 25
    ```

    **Por qué 1.7.** Con la varianza de ±15% por bando (Doc 5.2.5), ganar SIEMPRE exige superar
    `1.15 / 0.85 = 1.353`. El factor deja margen sobre ese mínimo sin acercarse al escuadrón más barato:

    | | Poder efectivo peor caso | Contra caravana (25 × 1.15 = 28,75) |
    |---|---|---|
    | Héroe solo (15 × 1.15 = 17,25) | 17,25 | **pierde siempre** |
    | Milicia completa (50 × 0.85 = 42,5) | 42,5 | **gana siempre** |

    Separación limpia por los dos lados, sin zona gris que calibrar a ojo.

### 1.1i Sexta ronda (2026-09-06): el Líder

23. **El origen NO cambia al separarse.** *"Al separarse de un ejército el origen sigue siendo el mismo; el
    destino puede que cambie, pero el origen sigue siendo el mismo."* Cierra una pregunta que este documento
    tenía mal planteada —daba por hecho que una columna nacida en campo necesitaba un origen "propio"— y
    además **es lo que el motor ya hace**: `unirseAEjercito` permite hoy que un jugador de la plaza Y se sume
    a un ejército salido de X, y al replegarse sus escuadrones vuelven a X. Separarse no inventa un origen
    nuevo; hereda el que la columna ya tenía.

24. **La política de unión se fija al formar el ejército**, con tres opciones: **rechazar por defecto**,
    **aceptar por defecto** o **preguntar al Líder**. Resuelve el punto abierto de si unirse en campo necesita
    permiso: lo necesita o no según lo que decidiera quien formó la columna.

25. **El Líder** (concepto nuevo): **el jugador que dispara la creación del ejército**. Dos reglas:

    - **El Líder no puede separarse.** Para irse tiene que **elevar a otro integrante a Líder**, y entonces sí.
    - Es quien responde las peticiones de unión cuando la política es *preguntar*.

    **Y con esto la regla del último se vuelve elegante.** La decisión 20 decía "el último no puede
    separarse" como una prohibición aparte; ya no hace falta enunciarla así: **el último es, por
    construcción, el Líder** —todos los demás se fueron cediendo o sin ser Líder— y el Líder no se separa.
    La invariante "una columna nunca se queda vacía en campo abierto" deja de ser una regla y pasa a ser una
    consecuencia.

    *(La decisión 31 rompió un rato esta deducción —un invitado que quedase el último no sería Líder— y la
    decisión 33 la repara al eliminar a los invitados. El párrafo de arriba vuelve a ser cierto sin
    asteriscos.)*

    Lo que el Líder arrastraba —quién cancela, si un invitado puede ser elevado, qué pasa con una petición
    sin contestar— lo cierran las **decisiones 30-32**. *(Y qué pasa si el Líder se desconecta, la 26.)*

### 1.1j Séptima ronda (2026-09-06): desconectarse es irse con lo tuyo

26. **Si el Líder se desconecta, el liderazgo pasa al siguiente con más ANTIGÜEDAD en el ejército.** Sin
    ceremonia y sin dejar la columna huérfana. Si el sucesor también está fuera, se aplica otra vez.

    *La decisión 31 le puso un matiz —"de la Facción que formó la columna"— que la 33 retira: sin invitados,
    todos los participantes son elegibles y la sucesión no necesita filtro. Y mientras quede alguien dentro
    hay sucesor; si no queda nadie, la columna ya se disolvió sola (decisiones 27-28).*

27. **Quien se desconecta se lleva sus tropas con él**, y al volver **aparece con ellas en el último sitio
    donde estuvo** — lo que le da la oportunidad de alcanzar a su ejército y volver a unirse.

    **Mecánicamente, desconectarse es separarse y desaparecer.** Se lleva lo mismo que si se hubiera separado
    (decisión 16): sus escuadrones y hasta un carro de suministro. El ejército sigue su marcha sin él, más
    débil. Reconectarse es nacer como **columna personal** en ese punto.

    Con una diferencia que importa: **el Líder no puede separarse, pero sí desconectarse.** Por eso hace falta
    la decisión 26 — la sucesión es lo que evita que la prohibición se convierta en una columna sin mando.

    **Y esto cierra el abierto de "desconectarse con tropas" en sentido contrario a lo que este documento
    proponía.** Se había escrito que una columna con soldados debía quedarse en el mundo ("un hombre solo se
    esconde, una columna de doscientos no"). No es lo que el usuario quiere, y su versión es más simple y más
    coherente con la decisión 6: **el jugador y todo lo que lleva encima entran y salen del mundo juntos.**

    Dos consecuencias que conviene ver venir, ninguna bloqueante:

    - **Alcanzar al ejército no siempre se puede.** Una columna es tan rápida como su escuadrón más lento
      (Doc 5.12.5), así que el que vuelve alcanza a los suyos **solo si sus tropas son más rápidas que la más
      lenta de la columna**. El que llevaba la tropa pesada que frenaba a todos no vuelve a alcanzarlos
      jamás. Es coherente y hasta elegante, pero hay que decirlo o se lee como un bug.
    - **Un ejército pierde fuerza a mitad de campaña porque a alguien se le cayó internet.** Es el precio de
      la regla, y el usuario lo asume explícitamente al pedir que la desconexión inesperada no castigue al
      jugador. El castigo se lo lleva la columna, no él.

28. **Una caravana adjunta que se queda sola VUELVE A SU ORIGEN.** Es el caso de que se desconecten todos los
    integrantes de un ejército: cada uno se lleva sus tropas y su carro, y las adjuntas se quedan sin nadie.

    **Vuelven, no se evaporan**: son caravanas normales haciendo el camino de vuelta, así que son
    interceptables durante el regreso. Si su asentamiento de origen ya no existe, se pierden — mismo criterio
    que los escuadrones de un ejército sin hogar (Doc 5.4).

    **Y no contradice a Doc 5.13.2**, que dice que las adjuntas se pierden con un ejército **derrotado**. La
    diferencia es quién las dejó sin escolta: al derrotado se las quitó un enemigo —el tren de suministros es
    objetivo militar— y a estas no las venció nadie, simplemente se quedaron solas. Los carreteros se vuelven
    a casa. Son dos hechos distintos y merecen dos desenlaces distintos.

29. **La desaparición al desconectarse es DIFERIDA: 2 minutos y medio**, la mitad de la tregua. Es la
    respuesta al abierto de "desconexión como escapatoria": el jugador sigue en el mundo ese rato y solo
    entonces se lo lleva todo.

    **Por qué el número funciona.** Un perseguidor cubre entre 30 y 55 unidades en 2:30 según su tropa (12-22
    por minuto, Doc 5.12.5). O sea que **atrapa a quien se desconecta si ya lo tenía a distancia de
    inspección (40) o menos**, y no a quien estaba lejos. Justo el reparto que se busca: castiga el corte
    oportunista con el enemigo encima, no al que se le cae la línea cruzando un páramo vacío.

    **Nota de reloj:** con 1 tick = 1 minuto, un plazo de 2:30 se comprueba en frontera de tick, así que en la
    práctica vence al tercer tick. Se declara igualmente en minutos y como `Instante` (D6) y no en ticks —
    bajo el modelo analítico de §7.2 vencerá exacto sin tocar la cifra.

### 1.1k Novena ronda (2026-09-06): los tres flecos del Líder

30. **Cancelar la marcha es SOLO del Líder.** El reparo que este documento anotaba —"un Líder ausente deja a
    la columna sin poder volver"— **ya lo había resuelto la decisión 26**: si se desconecta, el mando pasa
    solo. Mientras quede conectado alguien elegible, la columna siempre tiene quien la mande volver.

    **Y nadie queda atrapado, porque cancelar no es la única salida.** Cualquier integrante puede
    **separarse** cuando quiera (decisión 20). Lo que exige el mando no es irse: es **hacer volver a todos
    los demás**. Es la distinción correcta — el rumbo lo acordaron varios, deshacerlo no puede ser cosa de
    uno cualquiera, pero seguir en la marcha nunca es obligatorio.

    Queda un caso menor y asumido: un Líder **conectado pero ausente del teclado** congela el regreso de la
    columna. No hay sucesión que lo cubra —la decisión 26 se dispara con la desconexión, no con el
    despiste— y no hace falta inventarla: quien no quiera esperar, se separa.

    *Al implementar:* `replegarEjercito` hoy no distingue quién lo pide, así que el comando gana un chequeo
    `ejercito.liderId === jugadorId` en `session/comandos/militar.ts`. Para una columna `personal` es
    trivialmente cierto —su Líder es su único participante— y ahí el comando ni siquiera aplica: una columna
    personal rectifica, no cancela (Doc 5.12.6).

31. **Un Invitado NO puede ser Líder.** ~~Nunca: ni por cesión ni por sucesión.~~ **ANULADA POR LA DECISIÓN
    33**, que elimina la figura entera. Se conserva porque el análisis que produjo es justamente lo que
    llevó a eliminarla: al escribirla apareció que la regla —no la figura— arrastraba un filtro en
    `cederLiderazgo`, un matiz en la sucesión de la decisión 26, un estado nuevo (la columna sin Líder) y
    la pérdida de la deducción elegante de la decisión 25. Ese coste, puesto delante, es lo que hizo
    preguntarse si la figura valía lo que costaba.

32. **Una petición de unión vive 10 segundos**, y al vencer se da por **rechazada**. El silencio es un no —
    el mismo criterio prudente que la política *rechazar por defecto*.

    **Es el primer plazo del diseño más corto que un tick** (60 s), y aun así **no necesita temporizador ni
    scheduler.** Nada tiene que *dispararse* en el segundo 10: lo único observable es que la petición ya no
    sirve, y eso se evalúa **al leer**. La petición lleva su `expiraEn` y lo comprueban contra `ahora` los
    dos únicos sitios que la miran: el comando con el que el Líder responde, y la proyección del que pidió.
    Caducidad perezosa, cero maquinaria — y se comporta igual bajo el modelo analítico de §7.2.

    *Al implementar:* `domain/tiempo.ts` tiene `minutos()` y `dias()` pero no `segundos()`. Se añade, en vez
    de escribir `minutos(1/6)`.

    Dos consecuencias que conviene ver:

    - **Con 10 segundos, *preguntar al Líder* solo funciona si el Líder está al teclado.** Para uno
      despistado la política degenera en *rechazar*. Es la intención —10 s es corto justamente para que nadie
      se quede esperando en mitad del mapa— pero significa que en la práctica la mayoría de columnas usará
      *aceptar* o *rechazar*, y que *preguntar* es la opción de un grupo coordinado que está hablando.
    - **La distancia se revalida al aceptar, no al pedir.** En 10 s un jugador recorre unas 3,7 unidades
      (22/min), así que con radio de unión 15 casi nunca se sale — pero el chequeo va en la aceptación de
      todos modos: el que entra tiene que estar **junto a la columna**, no donde estaba cuando pidió.

### 1.1l Décima ronda (2026-09-06): fuera los invitados

33. **Se elimina la figura del Invitado. A un ejército solo se une gente de su MISMA Facción.** Los aliados
    dejan de poder marchar dentro de una columna ajena; los neutrales seguían sin poder (decisión 17, mitad
    que sobrevive).

    **No es quitar algo: es no escribirlo.** El motor de hoy ya lo prohíbe —`engine/ejercitos.ts:285`,
    *"No se puede unir tropas a un ejército de otra Facción"*— así que los invitados eran **código nuevo que
    relajaba una regla existente**. Esta decisión deja las cosas donde ya estaban.

    **Lo que se cae con ellos**, y es la razón de la decisión:

    | Se cae | Por qué |
    |---|---|
    | La decisión 31 entera | No hay a quién excluir del liderazgo |
    | El estado **columna sin Líder** | Solo nacía de la inelegibilidad. Con todos de la misma Facción, mientras quede alguien dentro hay sucesor; y si no queda nadie, la columna ya se disolvió sola (decisiones 27-28) |
    | La excepción a la decisión 30 | Cancelar vuelve a ser del Líder sin asteriscos |
    | El matiz de la decisión 26 | "El más antiguo", punto |
    | El filtro de `cederLiderazgo` | Cualquier integrante vale |
    | Dos abiertos de §9 | El invitado que se separa en territorio anfitrión, si cuenta para el repliegue, y la mitad rara del `origenAsentamientoId` |

    **Y vuelve la deducción elegante de la decisión 25**: el último que queda es otra vez, por construcción,
    el Líder. "Un ejército nunca se queda vacío en campo abierto" deja de ser una regla que enunciar,
    escribir y probar, y vuelve a ser una consecuencia. Es la reparación de lo que la decisión 31 había roto.

    **Y `Ejercito.faccionId` vuelve a ser honesto**: la Facción de todos los que van dentro, no solo la del
    fundador. Con eso la proyección se simplifica sola —*mis columnas = las de mi Facción*—, que con
    invitados necesitaba un predicado aparte.

    **El precio, asumido: una alianza se queda sin brazo militar.** Y no es cosmético. El combate es de
    **una entidad contra una** (`ganador: 'atacante' | 'defensor'` en `engine/combate.ts`), y el propio
    módulo deja anotado que **los aliados no se cruzan**: el llamador los excluye para que dos columnas
    amigas que compartan ruta no peleen entre sí. Así que dos columnas aliadas que marchan juntas **no
    suman**: pelean por separado, y con poder aditivo y bono de cohesión concentrar siempre gana. Dos
    columnas de 10 valen bastante menos que una de 20.

    Consecuencia a la vista: **las alianzas quedan reducidas a no agresión y comercio.** Si algún día se
    quiere cooperación militar de verdad, el camino ya no son los invitados sino **combate multibando** —una
    batalla con varias entidades por bando—, que es bastante más caro que lo que aquí se retira y que hoy no
    está en ninguna parte del diseño. Se anota como el sitio por donde volvería el problema, no como deuda.

### 1.1m Undécima ronda (2026-09-06): lo que faltaba para poder implementar

Las tres salen de auditar el diseño contra el plan de §5 antes de escribir código. La 34 tapa un agujero
real; las otras dos son contrato y confirmación.

34. **Lo que decide si una columna es Ejército es EL COMANDO QUE LA PARE, no cuánta gente sale.** Doc 5.12.1
    decía *"nace cuando dos o más salen juntos de un asentamiento"* — y **ese "salir juntos" no existía**: ni
    comando, ni flujo, ni invitación. Como además a una columna `personal` no se le puede unir nadie
    (decisión 19), el camino que el motor ya recorre hoy —A moviliza, B se suma desde su plaza con
    `unirseAEjercito`— habría quedado prohibido sin querer.

    | Comando | `tipo` | Qué significa salir así |
    |---|---|---|
    | `salirAlMundo` (**sin** destino) | `personal` | Sales a lo tuyo. Rectificas el rumbo cuando quieras, nadie se te une, no llevas caravanas |
    | `movilizarEjercito` (**con** destino) | `ejercito` | Sales en campaña. Rumbo fijo desde el primer paso **aunque salgas solo**, y otros pueden sumarse |

    **Y así "salir juntos" deja de necesitar maquinaria nueva**: es lo que el motor ya hace. Uno moviliza y
    los demás se suman, en su plaza (`unirseAEjercito`, que ya existe) o en el campo (`unirseEnCampo`, del
    paso 4b). El Ejército nace en el primer comando y ahí se fijan `liderId` y `politicaDeUnion`.

    **Consecuencia que hay que ver y aceptar:** dos columnas de un solo jugador, idénticas sobre el mapa,
    pueden tener reglas distintas — una rectifica y la otra no. Es exactamente lo que §1.1f estableció ("la
    línea es de dónde vino la columna, no cuántos van dentro"), llevado hasta el final. Y hace de la salida
    una elección de verdad: *voy a lo mío* o *salgo en campaña y me pueden acompañar*.

    **Toca canon**: la fila "Cómo nace" de Doc 5.12.1 y la frase gemela de §1.1f describían un flujo que no
    se va a construir.

35. **El contrato de `salirAlMundo`** (propuesto por mí, con el permiso del usuario para elegirlo):

    ```
    salirAlMundo(asentamientoId, escuadronIds: string[], carga: Record<RecursoId, number>)
    ```

    Se valida contra cuatro cosas, todas ya existentes: **residencia** (`esResidente`), **Liderazgo**
    (`exigirLiderazgo`, decisión 3), **capacidad del carro** y **la reserva de la plaza** (`cargarCarro` ya
    protege el trigo de la guarnición). Con `escuadronIds` **vacío** no falla: es el viajero sin tropas.

    Dos cosas que el motor de hoy no soporta y hay que cambiar en el paso 3:

    - `seleccionarParaCampana` exige **al menos un escuadrón**. Deja de exigirlo para este comando.
    - `capacidadCarrosDe` deriva la capacidad **de los escuadrones**, así que un jugador sin tropas tendría
      carro de capacidad **0** — y entonces no hay "carro personal" que robar (decisión 18) ni con qué
      comerciar. Pasa a derivarse de `participantes`, que es justo lo que el paso 1 introduce.

    **Y una sub-decisión que arrastra, más barata de lo que parecía:** `Ejercito.suministro` **ya es**
    `Record<string, number>` — es `cargarCarro` quien solo mete trigo, y la capacidad quien solo lo cuenta a
    él. Así que el carro como **saco de recursos** (decisión 3: elegir materiales; decisión 18: robar "la
    mitad del contenido") **no exige cambiar el tipo**, solo la carga y el recuento: una sola capacidad, una
    sola regla de robo, y `avanzarRacion` sigue leyendo `suministro.trigo` sin enterarse. Va en el paso 3.

36. **Durante los 2:30 de desaparición diferida, la columna SIGUE MOVIÉNDOSE** como iba. Confirma lo que la
    decisión 29 daba por natural y de lo que dependía su aritmética: el perseguidor tiene que cerrar la
    distancia con su ventaja de velocidad, igual que si el otro siguiera jugando. Desconectarse no es frenar
    en seco, y si lo fuera, 2:30 sería mucho más letal de lo que la cifra sugiere.

### 1.1f CORREGIDO: la línea no es "un participante", es de dónde vienes

Este documento tenía aquí una tabla que separaba "1 participante" de "2 o más". **Estaba mal**, y la decisión
20 la corrige: un ejército reducido a un solo miembro **sigue siendo un ejército** —conserva ruta fija y
caravanas—, así que el recuento no puede ser el criterio. Lo que separa las dos formas de jugar es **qué
nació la columna**, y eso se fija al crearla y no cambia nunca:

| | **Columna personal** (`salirAlMundo`, sin destino) | **Ejército** (`movilizarEjercito`, con destino) |
|---|---|---|
| Destino | **Rectificable** en cualquier momento (decisión 14) | Fijado al salir; cancelar es volver (Doc 5.12.6) |
| Caravanas adjuntas | **No puede** (decisión 12) | Sí, y las conserva aunque baje a un miembro |
| Se le pueden unir otros | No — dos viajeros no hacen un ejército (decisión 19) | Sí, **solo de su misma Facción** (decisiones 15 y 33) |
| Separarse | No aplica: separarse ES disolverla | Sí, **mientras quede alguien** (decisión 20) |
| Desconexión | Desaparece del mundo (decisión 6) | El que se va se lleva lo suyo; la columna sigue (decisión 27) |

Doc 5.12.1 ("salir solo es un ejército de un participante — un solo concepto") sigue siendo verdad en la
REPRESENTACIÓN: una sola entidad, `Ejercito`. Ya no lo es en las reglas. **Esta tabla hay que escribirla en
Doc 5.12 al implementar**, o alguien leerá 5.12.1 y dará por hecha una simetría que dejó de existir.

### 1.1c Lo que esto le hace al canon vigente

Tres reglas de `Docs/Game/5` cambian, y conviene que sea a sabiendas:

- **Doc 5.12.3 — "los encuentros salen de la geometría, sin declararlo".** Se sustituye por "la geometría
  ofrece, el jugador declara". No contradice su espíritu: el hueco entre 15 y 150 se eligió *precisamente*
  para que el encuentro fuera una decisión ("se ve diez veces más lejos de lo que se tropieza"). Lo que
  cambia es el mecanismo, de implícito a explícito. Pero **`radioEncuentro` cambia de papel**: deja de ser
  "a qué distancia te tropiezas" y pasa a ser "a qué distancia se cierra una persecución".
- **Doc 5.12.4 — "llegar es asediar".** Deja de ser cierto: asediar es una opción del menú. La consecuencia
  es buena y hoy es imposible — se podrá **acampar junto a una ciudad enemiga sin atacarla**, o sea bloquear
  y sitiar sin asaltar.
- **Doc 5.12.7 — "del ejército avistado nunca se ve su composición ni su poder".** Se revierte **solo dentro
  de los 40**, y con aviso al observado. A distancia de vista sigue viajando redactado exactamente como hoy.
- **Doc 5.1 — "el jugador nunca combate individualmente".** Deja de ser cierto (decisión 21): combate, con el
  poder de una unidad de élite. Sigue siendo verdad el resto de la frase —es un héroe-comandante, y su peso
  real está en la tropa que lidera, no en su espada— así que la reescritura es de una línea, no del párrafo.

### 1.1d Sacado a mecánica aparte

**Comerciar con una plaza ajena desde su puerta** era la cuarta opción del menú de asentamiento y pesa más
que las otras tres juntas: hoy el comercio es intra-Facción y desde asentamiento propio, así que un
"mercader visitante" es un sistema económico nuevo (qué puede comprar y vender un extranjero, con qué
comisión, contra qué órdenes). Sale de aquí por decisión del usuario y se anota como mecánica propia en
`Docs/Mecanicas a desarrollar.md` y en el checklist.

Mientras no exista, la opción *Comerciar* sencillamente no aparece en el menú.

### 1.2 Derivadas — no se preguntaron porque las de arriba o los invariantes vigentes las obligan

- **Salir solo es el caso mínimo de movilizar, no un comando nuevo.** Lo dice ya Doc 5.12.1, y la descripción
  del usuario en (3) es punto por punto `movilizarEjercito`: escuadrones bajo Liderazgo, carro cargado del
  almacén, aparición junto al asentamiento. Lo que falta son los tres agujeros del hallazgo 2 de §0, no un
  subsistema paralelo. **Y es lo que evita reimplementar tres mecánicas ya escritas**: encuentro por
  proximidad, asedio al llegar e intercepción de caravanas ya existen para columnas; con una entidad aparte
  habría dos copias de cada una condenadas a divergir.

- **La columna se disuelve SOLO en tu residencia.** De (3) y (4) juntas sale la simetría entera:

  | Entras en… | Qué pasa con tu columna |
  |---|---|
  | **Tu residencia** | Se disuelve: escuadrones a la guarnición, carro al almacén. Es `replegarEjercito`, que ya existe |
  | **Cualquier otro asentamiento** (propio no residencia, aliado, neutral) | Se queda **aparcada a la puerta** (`estacionado`). Al salir la retomas con lo que llevabas |

  Es coherente con Doc 2.5 —el roster vive donde resides, y solo ahí se recluta— y resuelve gratis dos casos
  feos: si conquistan la plaza ajena mientras estás dentro, tu columna está fuera y la retomas; y si te
  destruyen la columna aparcada mientras estás dentro, sales a pie, que es una consecuencia legítima y no un
  estado inválido.

- **Fundar exige estar allí.** Hoy `fundarAsentamiento` acepta una `posicion` cualquiera. Con el jugador
  situado, se funda **donde se está**, y el parámetro de posición desaparece del contrato. Esto es lo que
  convierte el spawn aleatorio en juego de verdad: apareces lejos, caminas, miras, y fundas donde decides
  parar. Y da propósito de golpe a §10 (landmarks) y a la niebla.

- **`Jugador` deja de ser un registro opcional.** Una posición no tiene valor por defecto: "ausente = usa
  `LIDERAZGO.base`" no se puede extender a "ausente = está en ninguna parte". El registro pasa a crearse al
  entrar en la partida. **Migración obligatoria** de snapshots (§6), la primera de esta clase en el repo.

- **La niebla NO se toca.** Es la confusión fácil de esta mecánica y conviene dejarla escrita: 13b restringe
  **interiores**, no el mapa. Tus plazas siguen vigilando su radio + 60 para toda tu Facción aunque no estés
  dentro de ninguna (Doc 5.12.8: la memoria es de la Facción), y lo avistado y lo recordado siguen exactamente
  igual. Lo que pierdes al salir es el ALMACÉN, la COLA y la GUARNICIÓN de las plazas donde no estás, no
  saber dónde están ni qué pasa en el mundo.

- **El interior propio se RECUERDA, no se apaga.** Cuando sales de tu ciudad, lo último que viste de dentro se
  queda como foto fechada, igual que una plaza ajena avistada (`FichaConocida`, `engine/memoria.ts`). El
  mecanismo ya existe y ya está probado; sería absurdo inventar un segundo olvido. "Última información: hace
  3 horas" es exactamente lo que Doc 5.12.8 ya decidió para lo ajeno.

- **Memoria de exploración PERSONAL mientras no haya bandera.** Sin esto el onboarding es un paseo ciego: un
  jugador sin Facción no graba nada. Se le da memoria propia, que se **funde** con la de la Facción al fundar
  o al entrar en una. Cierra el hueco que Doc 5.12.8 dejaba anotado sin resolver.

- **La proyección ENCOGE.** Contraintuitivo pero medible: hoy viaja el interior completo de todos los
  asentamientos propios; después viaja el de UNO más una ficha por cada uno de los demás. Va en la misma
  dirección que el trabajo de escala de esta semana (tick de 68 ms, curva lineal), no en contra.

- **La posición es del SERVIDOR, sin discusión.** `Docs/Arquitectura/9_Reglas_vs_Simulacion.md` la clasifica
  sola: es T3 (muta y avanza cada tick) y decide el resultado de OTROS jugadores —intercepciones, encuentros,
  asedios—. Un cliente autoritativo sobre su propia posición es un teletransporte, y dos clientes discreparían
  sobre quién interceptó a quién. Esto responde la pregunta que §14 dejaba abierta ("ver si se puede mover
  esta lógica a que sea 100% cliente"): **no**. Lo que sí es del cliente: interpolar la ficha entre ticks,
  previsualizar la ruta al pasar el ratón (T2 de entrada propia: ya tiene el mapa cacheado por `mapaId`) y la
  cámara.

## 2. Representación en el motor

### 2.1 `Jugador` — gana ubicación y deja de ser opcional

```ts
export type UbicacionJugador =
  | { tipo: 'asentamiento'; asentamientoId: string }
  | { tipo: 'columna'; ejercitoId: string }
  | { tipo: 'desconectado'; punto: Point };   // decisión 6: fuera del mundo, con su sitio guardado

export interface Jugador {
  id: string;
  liderazgoBase: number;
  ubicacion: UbicacionJugador;
  // El debuff de derrota NO vive aquí: vive en la columna (`Ejercito.enTreguaHasta`, §2.2), que es la
  // entidad a la que se persigue. Un jugador dentro de un asentamiento no es perseguible por definición.
  /** Lo que ha explorado ANTES de tener Facción. Se funde con la de la Facción al fundar o unirse, y a
   * partir de ahí manda la de la Facción. Ausente para quien ya tiene bandera. */
  exploracionPersonal?: Exploracion;
}
```

`'desconectado'` guarda un `Point` y no un `ejercitoId` a propósito: al desaparecer del mundo la columna de un
solo viajero deja de existir como entidad (no consume, no ve, no la ven), y al volver se reconstruye ahí. Lo
mismo vale para quien se desconecta **con tropas**: se las lleva consigo (decisión 27).

**Dónde nace un registro, resuelto al implementar el paso 2.** No había ninguno: `GameSessionState.jugadores`
se creaba vacío y nada lo escribía nunca — el Liderazgo se las apañaba con "ausente = `LIDERAZGO.base`". Con
la ubicación eso deja de valer, así que el alta va **en el embudo de `GameSession.ejecutar`**, no en cada
comando: la primera vez que alguien ejecuta un comando **con éxito**, la partida le da registro y lo sitúa.
Repartir el alta por dos docenas de comandos garantizaba olvidarla en alguno. El sistema (tick, turno del NPC)
no es un jugador y no crea registro.

**Y el alta CREA, nunca mueve.** Un `desconectado` no puede reaparecer dentro de su ciudad gratis solo porque
ejecute un comando — eso rompería la decisión 27. La única excepción es **fundar**, que sí coloca: construir
donde estás parado no es un viaje, y sin ella el fundador que había creado su Facción un segundo antes se
quedaba marcado como fuera del mundo dentro de su propia ciudad.

### 2.2 `Ejercito` — participantes propios

```ts
export interface Ejercito {
  // …lo de hoy…
  /**
   * Quién va DENTRO, con independencia de si aporta escuadrones. Hoy se deriva de los escuadrones
   * (`participantesDe`), y por eso un jugador sin tropas no existe como participante.
   *
   * **No es una lista de ids, es una lista de entradas**: la sucesión del Líder va por ANTIGÜEDAD
   * (decisión 26) y eso no se puede leer de un array de strings sin depender del orden de inserción, que
   * las separaciones y reuniones reordenan. `unidoEn` lo hace explícito y ordenable.
   */
  participantes: { jugadorId: string; unidoEn: Instante }[];
  /**
   * Qué NACIÓ esta columna, fijado al crearla y jamás modificado (decisiones 19-20, §1.1f). No se deriva de
   * `participantes.length`: un ejército reducido a un miembro sigue siendo un ejército.
   * Lo fija **el comando que la pare** (decisión 34): `salirAlMundo` —sin destino— produce `'personal'`;
   * `movilizarEjercito` —con destino— produce `'ejercito'` aunque salga uno solo.
   */
  tipo: 'personal' | 'ejercito';
  /**
   * Quién formó el ejército (decisión 25). No puede separarse: para irse cede el liderazgo. Suya en
   * exclusiva es además la cancelación de la marcha (decisión 30). En una columna `personal` es su único
   * participante y no significa nada.
   *
   * **Puede quedar apuntando a un desconectado**: la sucesión de la decisión 26 lo reasigna al participante
   * más antiguo, sin filtros — todos son de la misma Facción (decisión 33). Nunca queda sin candidato:
   * mientras haya alguien dentro hay sucesor, y si no hay nadie la columna ya se disolvió.
   */
  liderId: string;
  /** Qué se hace con quien pide unirse en campo (decisión 24), fijada al formar la columna. */
  politicaDeUnion: 'rechazar' | 'aceptar' | 'preguntar';
  /**
   * Peticiones vivas cuando la política es `preguntar` (decisión 32). **Caducan sin temporizador**: nada se
   * dispara al vencer, lo comprueban contra `ahora` los dos sitios que las miran —el comando con el que el
   * Líder responde y la proyección del que pidió—. Una columna SIN Líder elegible las deja caducar todas.
   */
  peticionesDeUnion?: { jugadorId: string; pedidoEn: Instante; expiraEn: Instante }[];
  /** Persecución en curso (decisión 10). Un objetivo MÓVIL en vez de un punto: la ruta se recalcula hacia
   * donde esté. Ausente = marcha normal contra `objetivo`. */
  persiguiendo?: { tipo: 'ejercito' | 'caravana'; id: string };
  /**
   * Derrotado hace poco (decisiones 7, 11 y 18). Corta por los DOS lados: nadie puede perseguirle ni
   * atacarle, y él tampoco puede perseguir ni atacar — sin eso, la inmunidad sería un escudo para depredar.
   * Instante de MUNDO, no ticks (D6).
   */
  enTreguaHasta?: Instante;
}
```

`enTreguaHasta` se llamaba `noPerseguibleHasta` en la ronda anterior. Se renombra porque la decisión 18 lo
convirtió en algo distinto: no es una protección, es una **tregua** — dejas de ser objetivo y dejas de poder
elegir objetivo. El nombre viejo describía la mitad y habría invitado a implementar solo esa mitad.

No es solo para esta mecánica: es el **bug del ejército fantasma** que el propio doc de ejércitos dejó
anotado (§1.1c). Un ejército cuyos escuadrones llegan todos a 0 sigue teniendo participantes; hoy no hay forma
de decirlo.

Consecuencias directas, las tres del hallazgo 2 de §0:

- `participantesDe(ejercito)` pasa a leer `participantes`. Los rombos del mapa (Doc 5.12.2) y los carros
  (Doc 5.13) siguen contando lo mismo para una columna con tropas, y por fin cuentan 1 para un viajero solo.
- `velocidadDeEjercito` sin escuadrones deja de devolver 0: devuelve `MOVIMIENTO.velocidadJugador` (§4), más
  rápida que la tropa ligera. Es literalmente §14: "debería moverse más rápido".
- `avanzarRacion` gana consumo **por participante** además del de unidad. Con 0 escuadrones consume poco pero
  nunca 0 — misma forma que `factorConsumoEstacionado`, que ya resolvió este patrón.
- `disolverSiVacio` (Doc 5.13.4) deja de disolver una columna con participantes y sin soldados. La regla pasa
  a ser: se disuelve cuando **no queda nadie dentro**, no cuando no quedan soldados.
- `adjuntarCaravana` exige `participantes.length >= 2` (decisión 12). Es la única regla nueva que
  `participantes` habilita y que no venía de arreglar un agujero.

### 2.2b Lo que se ve de lo ajeno, por anillo

La decisión 9 convierte la redacción de la proyección en **tres niveles**, no uno. Hoy solo existe el
primero:

| Anillo | Ejército ajeno | Caravana ajena/neutral | Asentamiento ajeno |
|---|---|---|---|
| **Visión** (150 / 80) | `EjercitoAvistado` de hoy: id, Facción, posición, nº de rombos | **`CaravanaAvistada`, NUEVO**: id, Facción, posición | `AsentamientoAvistado` de hoy: nombre, Facción, posición, nivel, frontera |
| **Inspección** (40) | + escuadrones y nombre del jugador. **Avisa al observado** | + si lleva escolta y **qué** recursos carga, sin cantidades | *(nada nuevo: la ficha ya es lo público)* |
| **Puerta / encuentro** (10 / 15) | Atacar | Interceptar | Entrar · Asediar · Consultar |

Dos notas que no son obvias:

- **La caravana ajena hoy NO VIAJA EN ABSOLUTO.** `proyectarParaJugador` manda
  `caravanas: filter(esPropio(origen) || esPropio(destino))` — solo las tuyas. Sin `CaravanaAvistada` no hay
  nada sobre lo que hacer clic. Es trabajo nuevo, no un campo más.
- **La inspección es una CONSULTA, no un dato que viaje solo.** Mandar la composición de todo lo que caiga a
  40 en cada proyección sería filtrarla sin que nadie haya pagado el precio de acercarse ni avisado al
  observado. Va como comando con respuesta, y es el comando el que dispara el aviso.

### 2.3 `Asentamiento` — la puerta

```ts
  /** Quién puede entrar (decisión 2, ordenanza del Gobernador). Ausente = 'abierto'. */
  politicaDeAcceso?: 'abierto' | 'solo_faccion' | 'faccion_y_aliados' | 'cerrado';
  /** Vetos nominales, por encima de la política general. */
  vetadosIds?: string[];
```

Ambos opcionales: ausente = abierto, así que **ningún snapshot necesita migración por esto** (a diferencia de
`Jugador`, §6). Se manipula con un comando del Gobernador, no como `PoliticaActiva`: las políticas expiran a
los 150 ticks (`POLITICAS.duracionMinutosPorDefecto`) y una puerta que se abre sola a las dos horas y media no
es una puerta.

### 2.4 La superficie de comandos

| Comando | Qué hace |
|---|---|
| `salirAlMundo(asentamientoId, escuadronIds, carga)` | Desde tu **residencia**: escuadrones (0..Liderazgo, la lista **puede ir vacía**) + carga elegida hasta llenar el carro → nace una columna `personal` (decisiones 34-35), `estacionado` junto a la plaza |
| `marcharA` | Fija/rectifica destino, **solo si vas solo** (decisión 14). Con dos o más participantes se rechaza: Doc 5.12.6 sigue mandando y cancelar es volver |
| `entrarEnAsentamiento` | Exige proximidad (`MOVIMIENTO.radioPuerta`) y permiso. En residencia disuelve la columna; en cualquier otra la aparca |
| `salirDeAsentamiento` | De una plaza ajena: retomas la columna aparcada, sin interfaz (decisión 4) |
| `fijarPoliticaDeAcceso` / `vetarJugador` | Gobernador |

Y los de la segunda ronda, todos los que antes hacía sola la geometría:

| Comando | Qué hace | Rango que exige |
|---|---|---|
| `inspeccionar` | Devuelve la composición del objetivo **y avisa al observado** con un evento | 40 |
| `perseguir` | Fija objetivo móvil. Rechaza si el objetivo está en tregua, o si lo estás tú (decisión 18) | Verlo (150/80) |
| `dejarDePerseguir` | Cancela. También lo cancela cualquier `marcharA` | — |
| `atacar` | Resuelve el choque. Sustituye al disparo automático de `resolverEncuentros` | 15 |
| `interceptar` | Lo mismo contra una caravana | 15 |
| `asediar` | Ya existe (`iniciarAsedio`); lo que se quita es que la llegada lo dispare sola | 10 |
| `consultar` | Ficha pública. Ya viaja en `AsentamientoAvistado`: es solo UI | 10 |
| `unirseEnCampo` | Tu columna personal se funde en un **ejército** que tienes delante: tus escuadrones y tu carro entran, y **adoptas su destino** (decisiones 14-15). **Solo misma Facción** —aliados y neutrales se rechazan igual (decisión 33)— y un ejército no puede unirse a otro (decisión 19) | 15 |
| `separarseDelEjercito` | Sales con lo tuyo y naces como **columna personal** en esa posición, **conservando el `origenAsentamientoId`** de la columna (decisiones 16 y 23). **Se rechaza si eres el Líder** (decisión 25) — que es también por lo que la columna nunca se queda vacía: el último siempre lo es | — |
| `cederLiderazgo` | Eleva a otro integrante a Líder. Es el único camino para que el Líder pueda irse (decisión 25). Cualquier integrante vale como candidato (decisión 33) | — |
| `responderPeticionDeUnion` | El Líder acepta o rechaza a quien pidió unirse, cuando la política es *preguntar* (decisión 24). Comprueba la caducidad de 10 s y **revalida la distancia** antes de aceptar (decisión 32) | 15 |
| `replegarEjercito` (existente) | Cancelar la marcha, **solo el Líder** (decisión 30), sin excepciones. Hoy no distingue quién lo pide | — |

`movilizarEjercito` y `unirseAEjercito` **no se retiran**: sacar tropas contra un objetivo sigue siendo una
operación con sentido propio. `salirAlMundo` es su hermana sin destino.

**`unirseEnCampo` NO es el `unirseAEjercito` de hoy**, aunque se parezcan de nombre. El de hoy es
*jugador ↔ asentamiento*: exige que el ejército pase cerca de **tu plaza** (`radioReabastecimiento`) y sacas
tropas **de casa**. El nuevo es *columna ↔ columna*: exige proximidad **entre las dos**, y aportas lo que ya
llevabas encima. Los dos tienen sentido y conviven; lo que no puede pasar es que se confundan al
implementarlos, porque validan geometrías distintas.

Y `separarseDelEjercito` se lleva **hasta un carro** (`LOGISTICA.capacidadCarroPorJugador`) del suministro
común, que es exactamente lo que aportó al entrar. No hace falta inventar un reparto proporcional: el carro
ya es por jugador y de capacidad fija (Doc 5.13, decisión 12 del doc de ejércitos).

**El menú lo pinta el cliente; el rango lo comprueba el servidor.** Sin validación en cada comando, un
cliente modificado inspecciona ejércitos desde el otro lado del mapa. Esto es exactamente la regla de
`Docs/Arquitectura/9_Reglas_vs_Simulacion.md`: el menú es presentación, la distancia es un hecho del juego.

### 2.5 Dónde encaja en el tick

En ningún sitio nuevo. `avanzarEjercitos` ya es el último eslabón de la cadena y ya hace las cuatro cosas de
las que esta mecánica parte: comer, avanzar por la ruta, resolver encuentros y resolver llegadas.

Pero **dos de las cuatro se van** con la segunda ronda. El tick pasa a hacer tres cosas:

| Hoy | Después |
|---|---|
| Comer | Igual |
| Avanzar por la ruta | Igual, **más** recalcular la ruta de quien persigue |
| **Resolver encuentros** | Solo **detectar** que una persecución se cerró a 15, y ofrecerlo. El choque lo pide un comando |
| **Resolver llegadas** (asedio incluido) | Solo llegar y acampar. Asediar lo pide un comando |

O sea: el tick deja de decidir nada de combate y pasa a mantener posiciones y a **avisar**. Eso es lo que
hace que la tregua no necesite un filtro en el tick — se comprueba en el comando `perseguir`,
que es donde alguien intenta empezar algo.

Conviene saber que esas cuatro cosas **no van a seguir juntas para siempre**: comer es una tasa y se queda en
el paso de un minuto, mientras que avanzar, tropezarse y llegar son las que pasan a la forma analítica de
§7.2. Esta pasada no las separa —no hace falta todavía— pero tampoco las entrelaza más de lo que ya están.

## 3. Lo que hay que tocar

| Archivo | Qué |
|---|---|
| `domain/types.ts` | `UbicacionJugador`, `Jugador.ubicacion/exploracionPersonal`, `Ejercito.participantes/liderId/politicaDeUnion/peticionesDeUnion/tipo`, `Asentamiento.politicaDeAcceso/vetadosIds` |
| `domain/tiempo.ts` | `segundos(n)`. Hoy solo hay `minutos()` y `dias()`, y la caducidad de 10 s de la decisión 32 es el primer plazo del juego por debajo del minuto |
| `engine/ejercitos.ts` | `participantesDe`, `velocidadDeEjercito`, `disolverSiVacio`, movilización sin objetivo y con carga elegida, `adjuntarCaravana` con mínimo de 2 participantes |
| `engine/ejercitos.ts` (`resolverEncuentros`) | **Deja de disparar solo.** Pasa de resolver a *ofrecer*: los combates los piden los comandos. Lo que se queda aquí es la persecución (recalcular ruta hacia el objetivo y detectar el cierre a 15) |
| `engine/ejercitos.ts` (llegada) | La llegada a plaza ajena deja de asediar (Doc 5.12.4) |
| `engine/bandidos.ts` | El jugador ataca por menú; los bandidos siguen atacando caravanas por su cuenta (§9) |
| `engine/tropas.ts` (`avanzarRacion`) | Consumo por participante |
| `engine/pertenencia.ts` | `puedeEntrarEn(asentamiento, jugador, relaciones)` — la puerta |
| `engine/memoria.ts` | Foto del interior propio al salir; fusión de `exploracionPersonal` con la de la Facción |
| `session/comandos/autorizacion.ts` | **El cambio ancho**: ~24 comandos pasan de exigir residencia a exigir **presencia** |
| `session/comandos/*` | Los cinco comandos nuevos de §2.4; `fundarAsentamiento` pierde `posicion` |
| `session/comandos/militar.ts` (`replegarEjercito`) | Cancelar pasa a ser **solo del Líder** (decisión 30) —hoy lo acepta de cualquier participante—, sin excepciones |
| `session/proyecciones/jugador.ts` | Interior de UNO, ficha de los demás, recuerdo de los que dejaste, y **`CaravanaAvistada`** — hoy no viaja ninguna caravana ajena (§2.2b) |
| `session/npcGobernanza.ts` | Política de persecución del NPC. Sin ella, el batch deja de producir combates y las constantes calibradas dejan de significar lo mismo (§10.5) |
| `session/estado.ts` | Migración de snapshots (§6) |
| `server/rutas/*`, `openapi.ts` | Contrato |

## 4. Constantes nuevas (todas PLACEHOLDER, a calibrar por simulación)

```ts
export const MOVIMIENTO = {
  /** Velocidad de un jugador SIN tropas. Por encima de la ligera (20) — §14: "más rápido que uno que lleva
   * sus tropas". Techo natural: la caravana de contrabando (24), que por diseño escapa de todo. */
  velocidadJugador: 22,
  /** Consumo por participante y minuto, con tropas o sin ellas. §14: "casi nada de trigo". */
  consumoPorParticipante: 0.5,
  /** A qué distancia de una plaza se puede cruzar la puerta. */
  radioPuerta: 10,
  /**
   * A qué distancia se puede INSPECCIONAR, y a la que el observado se entera (decisión 9). **40**: a media
   * distancia entre ver (150) y chocar (15), que es lo que lo hace un juego de dos. Bastante lejos como para
   * que un explorador se acerque y se vaya antes de que una columna lo alcance —es más rápido—, bastante
   * cerca como para que mirar cueste ser visto mirando.
   */
  radioInspeccion: 40,
  /** Tregua tras una derrota: ni le atacan ni ataca (decisiones 7, 11 y 18). En minutos, como todo plazo
   * desde D6. */
  treguaTrasDerrotaMinutos: 5,
  /**
   * Cuánto tarda un jugador desconectado en desaparecer del mundo (decisión 29). **2,5** — la mitad de la
   * tregua. Es lo que separa el corte oportunista del corte de verdad: en ese rato un perseguidor cubre
   * 30-55 unidades, así que atrapa a quien se desconecta teniéndolo ya a distancia de inspección (40) o
   * menos, y no a quien estaba lejos.
   */
  desaparicionDiferidaMinutos: 2.5,
  /** Qué fracción del carro se lleva quien roba a un viajero (decisión 18). **La mitad**: dejarle algo es lo
   * que hace que valga la pena seguir el viaje en vez de reiniciarlo, y lo que distingue un robo de una
   * ruina. Con el carro vacío no hay botín, solo la tregua. */
  fraccionRobada: 0.5,
  /**
   * Cuánto vive una petición de unión sin contestar, antes de darse por RECHAZADA (decisión 32). Único plazo
   * del diseño **más corto que un tick**, y aun así sin temporizador: nada se dispara al vencer, se comprueba
   * al leer. En segundos porque en minutos sería `1/6`; pide un `segundos()` en `domain/tiempo.ts`, que hoy
   * solo tiene `minutos()` y `dias()`.
   */
  vidaPeticionUnionSegundos: 10,
};
```

Y en `VISION`:

```ts
  /** Lo que ve un jugador SOLO. Por debajo de los 150 de una columna: un hombre solo no despliega batidores.
   * La tensión que fija el número: bastante para viajar sin caer en emboscadas a ciegas, poco para que el
   * explorador solitario sea la mejor unidad de información del juego. */
  jugadorSolo: 80,
```

## 5. Plan de ejecución

Cada paso deja el repo verde y jugable. El orden no es negociable en los tres primeros: son los cimientos.

> **Reescrito en la undécima ronda.** La versión anterior se escribió sobre la cuarta y las decisiones 24-36
> —el Líder, la política de unión, las peticiones, la desconexión entera— no tenían paso asignado. Un plan
> que omite diez decisiones cerradas miente por omisión, así que aquí están los pasos 4c y 4d, que son
> nuevos, y los pasos 1 y 3 dicen ahora lo que las decisiones 34-35 les añadieron.

1. ~~**`Ejercito.participantes`, `tipo` y `liderId`**~~ — **HECHO (2026-09-06)**, con sus consumidores
   (`participantesDe`, `capacidadCarrosDe`, velocidad, ración, disolución) y la migración v7->v8. Arregló el
   **ejército fantasma** y dejó dos hallazgos: un escuadrón aniquilado seguía FRENANDO a la columna (persiste
   como identidad con `cantidad: 0`, y `velocidadDeEjercito` lo miraba igual), y **Doc 5.13.4 estaba escrito
   sobre la regla vieja** — reescrito: el fantasma nunca fue "sin soldados", fue "sin nadie".
2. ~~**`Jugador.ubicacion` + migración de snapshots**~~ — **HECHO (2026-09-06)**, con la migración v8->v9 y el
   alta perezosa en el embudo de `GameSession` (§2.1). Nada la lee aún, salvo fundar, que sí sitúa.
3. **`salirAlMundo` / `entrarEnAsentamiento` / `salirDeAsentamiento`**, con la simetría residencia/ajena y el
   contrato de la decisión 35 — lista de escuadrones **que puede ir vacía**, carga elegida, y el carro como
   saco de recursos. La ubicación empieza a moverse de verdad.
4. **`marcharA`** — destino rectificable, y solo yendo solo (decisión 14). Aquí ya se juega.
4b. **`unirseEnCampo` / `separarseDelEjercito`** (decisiones 15-16). Va pegado al 4 porque es lo que le da
   sentido: unirse cuesta la libertad de movimiento que el paso 4 acaba de conceder, y separarse la devuelve.
   Ojo con no confundirlo con el `unirseAEjercito` de hoy, que valida otra geometría (§2.4).
4c. **El Líder** (decisiones 24-25, 30-33): `politicaDeUnion` fijada al parir la columna, `cederLiderazgo`,
   `replegarEjercito` restringido al Líder, y `peticionesDeUnion` con su caducidad de 10 s —que se comprueba
   al leer, sin temporizador (decisión 32)— más el `segundos()` que le falta a `domain/tiempo.ts`. Va después
   del 4b porque es lo que gobierna quién entra y quién manda una vez que unirse ya funciona.
4d. **La desconexión** (decisiones 26-29 y 36): desaparición **diferida 2:30** con la columna aún moviéndose y
   aún atacable, el jugador se lleva sus tropas, sucesión del Líder por antigüedad, reaparición en el mismo
   punto, y caravanas adjuntas huérfanas de vuelta a su origen. Es el paso que una implementación ingenua se
   salta entero retirando al jugador en el `disconnect` del socket.
5. **La puerta**: `politicaDeAcceso`, vetos, comando del Gobernador.
6. **La proyección**: interior de uno, ficha de los demás, recuerdo de los dejados atrás. Es el paso que más
   contrato rompe; va después de que el estado sea correcto, no antes.
7. **Presencia en la autorización**: los ~24 comandos. **Requiere haber decidido antes qué se hace con el NPC
   de gobernanza** (§10.3): sin ubicación ni exención explícita, el batch se cae aquí.
8. **Las interacciones dejan de ser automáticas** (§1.1b). Es un paso grande y va en este orden interno:
   a) `CaravanaAvistada` en la proyección — sin ver, no hay clic;
   b) `inspeccionar` con su rango de 40 y su aviso al observado;
   c) `atacar` / `interceptar` / `asediar` explícitos, y **quitar** los disparos automáticos de
      `resolverEncuentros` y de la llegada;
   d) `perseguir` / `dejarDePerseguir` y la tregua (`enTreguaHasta`). **Antes de esto hay que medir
      `calcularRuta`** (§9): la persecución lo llama por objetivo y por tick;
   e) política de persecución del NPC, sin la cual el batch se queda sin combates.
9. **Onboarding**: spawn aleatorio, memoria personal, fundar donde se está. Cierra la entrada del checklist.
10. **Calibración** por batch — ahora nueve constantes, y con el combate ya intencional, que es lo que hace la
    medición representativa.

El paso 8 podría partirse en dos mecánicas si se hace largo: (a)-(c) son "las interacciones son
intencionales" y (d)-(e) son "la persecución". Se dejan juntos porque sin persecución, quitar el disparo
automático deja el juego **sin ninguna forma de forzar un combate** — el enemigo se limita a no pulsar nada.

## 6. La migración de snapshots

Es la primera migración obligatoria del repo y conviene decirlo en voz alta: hasta ahora **toda** mecánica
nueva se diseñó para que un snapshot viejo siguiera cargando sin tocarlo (la niebla lo consiguió con "Facción
ausente = no ha visto nada"; el Liderazgo con "jugador ausente = `LIDERAZGO.base`"). Con la posición no hay
default posible, así que hay que **censar**.

**Hecha (paso 2), en dos saltos y no en uno:**

- **v7 -> v8**: `Ejercito` gana `participantes`, `tipo` y `liderId`. Sin pérdida — `participantes` se deduce
  de los escuadrones, que es exactamente lo que el motor calculaba antes; `tipo` es siempre `'ejercito'`
  porque hasta v8 la única forma de parir una columna era movilizar; `liderId` es el primer participante,
  arbitrario y sin nada mejor disponible, porque quién formó la columna no se guardaba en ninguna parte.
- **v8 -> v9**: `Jugador` gana `ubicacion`. El censo es la unión de los cinco sitios donde el motor dejaba
  escritos a los jugadores antes de que existieran como entidad — `jugadores`, `jugadoresFundadoresIds`,
  `casasCompradas`, `cargos` y `Escuadron.jugadorId`, dentro y fuera de campaña. Se ordena por id para que
  el resultado sea determinista.

**La ubicación la deduce la MISMA función que el alta en marcha** (`ubicacionDeducida`), y no dos parecidas:
si difirieran, cargar una partida vieja colocaría a la gente en un sitio distinto del que la coloca el juego.
El orden de la deducción es el del canon y no es obvio: **la columna gana a la residencia** — un jugador de
campaña sigue residiendo en su ciudad, pero está en el camino (Doc 2.5).

## 7. La continuidad con Fase 1 — el movimiento es el MISMO, y no hace falta netcode

Esta sección se reescribió entera el 2026-09-06 tras la aclaración de la decisión 5. La versión anterior daba
el avatar por "otro proyecto, otro reloj" porque asumía control continuo; con clic a destino **eso deja de ser
cierto**, y la mecánica sale bastante más barata.

### 7.1 Por qué clic a destino no es netcode

"Netcode" —el caro, el de predicción y reconciliación— hace falta cuando el servidor recibe **entrada
impredecible y continua**: WASD, apuntar, saltar. El cliente no puede saber dónde estará dentro de 200 ms
porque depende de lo que se pulse en ese momento, así que hay que transmitir posiciones sin parar y
reconciliar la divergencia.

Con clic a destino no hay nada de eso, porque **el futuro es conocido**:

| | Clic a destino (lo nuestro) | Control continuo (Fase final del roadmap) |
|---|---|---|
| Qué manda el cliente | **Un mensaje por decisión**: "voy a (x,y)" | Un flujo de entrada, 30-60 veces por segundo |
| Qué sabe el servidor | La ruta entera y cuándo llega | Solo lo que se acaba de pulsar |
| Divergencia cliente/servidor | **Ninguna**: los dos evalúan la misma función | Constante, hay que reconciliarla |
| Posiciones por la red | **No se mandan** | Se mandan sin parar |

El servidor valida el destino una vez, calcula la ruta y la parametriza por tiempo. A partir de ahí los dos
lados tienen la misma función y el cliente pinta a 144 fps sin preguntar nada — no está simulando, está
leyendo. Lo mismo vale para las miniaturas ajenas: se recibe su ruta y su instante de salida una vez, y se
mueven localmente.

Todo esto cabe en la arquitectura de comandos y eventos que ya existe: un comando (`marcharA`) y eventos por
WebSocket. **Ninguna pieza de red nueva.**

### 7.2 Lo que separa Fase 0 de Fase 1, y son dos piezas concretas

Hoy [`avanzarPosicionEnRuta`](../src/engine/movimiento.ts) **integra por Euler**: cada tick muestrea
`costeEnPunto` en la posición actual y avanza. Por eso la posición solo existe en fronteras de tick — el
límite real, y no el reloj de mundo, que ya es tiempo real desde la Fase D.

La ruta es una polilínea fija y el terreno un campo estático, así que se puede **parametrizar la ruta por
tiempo una sola vez al lanzarla** (integrar el coste a lo largo del trazado y guardar la tabla acumulada).
Con eso, `posición(t) = f(ruta, velocidad, t₀, t)`: una consulta, no una simulación. La resolución del
movimiento pasa a ser infinita y deja de depender de ningún reloj.

Las dos piezas del salto, ninguna exclusiva de esta mecánica:

1. **Parametrización temporal de la ruta** (`world/rutas.ts` + `engine/movimiento.ts`).
2. **Scheduler de eventos** — llegadas, encuentros y expiraciones dejan de comprobarse cada tick y pasan a
   agendarse en su instante. Ya estaba priorizado por su cuenta en
   [`Docs/Arquitectura/10_Modelo_Temporal.md`](../Docs/Arquitectura/10_Modelo_Temporal.md) §8 ("sube de
   prioridad: el scheduler — es lo que el backlog de mecánicas necesita"). Es también lo que resuelve la
   inmunidad de 5 minutos de la decisión 7.

Y tres cosas pequeñas que Fase 1 tendrá que resolver, todas acotadas: acordar el "ahora" entre cliente y
servidor (la proyección ya lleva `instante`, falta el offset estimado al conectar), avisar por evento cuando
un trayecto se corta antes de tiempo, y recalcular desde la posición actual al redirigir a media ruta.

> **El coste honesto de la migración:** integrar finamente la ruta al lanzarla da tiempos de llegada
> **ligeramente distintos** a los de hoy — más exactos, pero distintos. Eso mueve balance ya calibrado
> (velocidades de tropa y de caravana, ventanas de intercepción) y hay que remedirlo con el batch.

**La persecución es la excepción, y se resuelve recalculando** (decisión del usuario, segunda ronda). Una
persecución persigue algo que se mueve, así que no hay polilínea fija que parametrizar: la forma cerrada se
rompe para el perseguidor. En vez de resolver analíticamente el punto de intercepción —que se puede, dadas
las dos velocidades, pero es frágil en cuanto el perseguido gira—, **se recalcula la ruta hacia la posición
actual del objetivo** y cada tramo corto sí es analítico. El perseguido conserva su forma cerrada intacta;
solo el perseguidor paga el recálculo, que es justo el reparto correcto del coste.

### 7.3 Qué respetar en Fase 0 para que el salto sea solo eso

Cuatro reglas, todas comprobables:

1. **`posicionActual` es la verdad; `ruta`/`progreso` son cómo llegó ahí.** Nunca derivar la posición del
   progreso: al pasar a la forma analítica, la posición se calculará de otro modo y el resto del motor no
   debe enterarse.
2. **Ninguna velocidad ni plazo se expresa en ticks.** Todo en minutos e `Instante`, como manda D6 — la capa
   analítica trabaja en tiempo continuo y las mismas cifras siguen valiendo. Es la más fácil de romper por
   descuido (ver §10.4).
3. **Las interacciones son DISTANCIAS, no coincidencias de tick.** `radioEncuentro`, `radioPuerta` y
   `radioReabastecimiento` ya lo son; que ninguna regla nueva diga "en el mismo tick". Es lo que permite
   convertir un encuentro en un tiempo de colisión resuelto por adelantado.
4. **Entrar y salir son comandos, no efectos de tick.** Se resuelven al instante contra la distancia, así que
   funcionan igual bajo cualquier reloj.

### 7.4 Lo que sigue fuera de alcance

El control continuo de la **Fase final** del `Roadmap_Escalado.md` ("mundo y asentamiento como una sola
entidad continua, estilo WoW") es lo único que necesitaría netcode de verdad: entrada del cliente a alta
frecuencia, autoridad con reconciliación y gestión de interés. Ni esta mecánica ni la Fase 1 lo requieren, y
—lo importante— tampoco exigiría que la economía corriera más rápido.

> **Nota:** el análisis completo de por qué bajar `SIMULACION.duracionTickMs` es la palanca equivocada
> (coste de CPU medido, persistencia por operación, el laboratorio de balance, y la revisión de la retirada
> de P3 en el doc 10) está hecho pero **todavía sin escribir en `Docs/Arquitectura/10_Modelo_Temporal.md`**,
> que es donde le toca vivir. Pendiente.

## 8. Invariantes a congelar en tests

- Un jugador está **en exactamente un sitio**: asentamiento, columna o desconectado. Nunca dos, nunca ninguno.
- La proyección **nunca** lleva el interior (almacén, cola, guarnición, cargos, trazado) de un asentamiento en
  el que el jugador no está. Ni siquiera propio. Ni siquiera si es el Gobernador.
- Salir de la residencia y volver a entrar sin hacer nada **deja el asentamiento idéntico** (ida y vuelta de
  escuadrones y carga sin pérdidas ni duplicados).
- Una columna con participantes y **cero soldados no se disuelve**; una sin participantes sí.
- Entrar en una plaza con `politicaDeAcceso: 'cerrado'` se rechaza aunque seas de su Facción y aunque seas el
  Rey; el Gobernador es quien manda en su puerta.
- La niebla **no cambia** al salir de una plaza: las mismas celdas exploradas y los mismos avistados antes y
  después.
- Cargar un snapshot anterior a la mecánica coloca a todo jugador conocido y no pierde ninguno.

Y los de la segunda ronda:

- **Ningún combate ocurre sin un comando detrás.** Dos columnas enemigas pueden pasarse la una junto a la
  otra a distancia 1 y no pasa nada. Es el invariante que más tests viejos rompe, y el que hay que congelar
  primero.
- **Llegar a una plaza enemiga no la asedia.** Se acampa delante.
- Una `CaravanaAvistada` **nunca lleva cantidades**, solo nombres de recurso; y no viaja ninguna fuera del
  radio de visión.
- `inspeccionar` a más de 40 se rechaza, y **siempre** emite el aviso al observado — no hay inspección
  silenciosa.
- No se puede iniciar `perseguir` contra alguien en tregua, y sí un minuto después. Y quien está en tregua tampoco puede perseguir ni atacar.
- Un ejército de **un solo participante no puede adjuntar caravanas**.

Y los de la tercera ronda, todos alrededor de la misma línea:

- **`marcharA` se acepta en una columna `personal` y se rechaza en un `ejercito`** — incluso en uno reducido
  a un solo miembro. Es el invariante que hace cumplir las decisiones 14 y 20, y el sitio donde una
  implementación descuidada derivaría el permiso de `participantes.length` en vez de leer `tipo`.
- **Un ejército con un solo miembro conserva ruta fija y caravanas.** Es el caso del ejemplo de la decisión
  20 y el que delata si alguien confundió identidad con recuento.
- **El Líder no puede separarse**, y por eso una columna **nunca se queda sin participantes en campo
  abierto**. El test se escribe sobre el Líder, no sobre el recuento: es la condición real.
- **Ceder el liderazgo y separarse a continuación sí funciona** — la única salida del Líder.
- **La política de unión se respeta**: `rechazar` no deja entrar a nadie, `aceptar` no pregunta, `preguntar`
  no mete a nadie hasta que el Líder responda.
- El que se separa **conserva el `origenAsentamientoId`** de la columna, no el de su residencia.
- **Desconectarse se lleva las tropas del jugador y deja el ejército en pie**, con un participante menos y su
  suministro reducido en un carro. Reconectar las devuelve al mismo punto.
- **Un ejército nunca se queda sin Líder**: si el Líder se desconecta, lo es el de más antigüedad; si ese
  también está fuera, el siguiente.
- La antigüedad se lee de `participantes[].unidoEn`, **nunca del orden del array** — separarse y volver a
  unirse lo reordena.
- **Desconectarse NO retira al jugador del mundo en el acto**: sigue ahí 2:30, moviéndose como iba, y es
  atacable durante ese rato. Es el invariante que hace real la decisión 29 y el que una implementación
  ingenua se salta (retirar en el `disconnect` del socket).
- **Una caravana adjunta que se queda sin columna vuelve a su origen**; una cuya columna fue DERROTADA se
  pierde. Los dos caminos se prueban por separado — es la distinción que la decisión 28 introduce.
- **Unirse en campo y separarse conservan todo**: los mismos escuadrones y el mismo suministro total antes y
  después de la operación, sin pérdidas ni duplicados. Es el mismo tipo de test de ida y vuelta que el de
  salir de la residencia y volver a entrar.
- Quien se separa se lleva **como mucho un carro** de suministro, nunca más.
- **Todo participante de una columna es de su `faccionId`.** Es lo que hace honesto el campo (decisión 33).
- **Dos columnas personales no pueden fusionarse** en ninguna circunstancia (decisión 19).
- Quien está **en tregua** no puede ser perseguido ni atacado, **y tampoco puede perseguir ni atacar**. Las
  dos mitades se comprueban por separado: la segunda es la que se olvida.
- Robar a un carro **vacío** no falla ni da botín: aplica la tregua y nada más.
- **Un atacante sin escuadrones pero con un participante combate**; uno sin ninguna de las dos cosas se
  rechaza. Es el cambio exacto de la guarda de `resolverCombate` (decisión 21).
- **El héroe derrotado no sufre bajas, no queda herido y no gana veteranía.** Solo pierde media carga y entra
  en tregua.
- `poderHeroe` **sale del catálogo**, no de una constante escrita: subir el `poderBase` de la tropa de élite
  sube el del héroe sin tocar nada más.

- **Solo el Líder cancela la marcha** (decisión 30). Un participante cualquiera que lo pida se rechaza — hoy
  `replegarEjercito` lo aceptaría, así que este test empieza en rojo.
- **Un ejército nunca se queda vacío en campo abierto**, y se prueba por su causa y no por el síntoma: el
  Líder no puede separarse, y el último que queda siempre lo es (decisiones 25 y 33).
- **Un aliado no puede unirse a un ejército** (decisión 33), ni en campo ni al pasar por su plaza. Se prueba
  junto al neutral y con el mismo rechazo — el motor ya lo hace en `unirseAEjercito` y no debe dejar de
  hacerlo al añadir `unirseEnCampo`.
- **Una petición de unión caduca a los 10 s y cuenta como rechazada**, y caduca **sin que corra ningún
  temporizador**: el test avanza el reloj sin ejecutar ticks y la petición ya está muerta al leerla.
- **Aceptar revalida la distancia**: el que pidió desde 14 y se alejó a 20 mientras el Líder pensaba, se
  rechaza al aceptar.
## 9. Abierto a propósito

- ~~BLOQUEANTE: el jugador solo sí combate~~ — **CERRADO (decisión 21)**, y el canon ya está reescrito:
  `Docs/Game/5` §5.1 dice ahora que el héroe combate con el poder de una unidad de élite, conservando la
  regla de oro Tropa > Héroe.

- **NUEVO — el jugador dentro de una plaza que cambia de manos.** Se resolvió qué pasa con su COLUMNA (está
  aparcada fuera y la retoma, §1.2), pero no qué pasa con ÉL. Está dentro de una ciudad que acaba de pasar a
  manos enemigas. ¿Se le expulsa a la puerta, queda atrapado, se le aplica la política de acceso del nuevo
  dueño? Lo natural es lo primero —sale a la puerta y retoma su columna— pero no está decidido, y es un caso
  que va a ocurrir.

- **NUEVO — cuánto cuesta recalcular rutas, y hay que medirlo antes de comprometerse.** Dos cosas nuevas
  piden pathfinding con frecuencia: rectificar destino (`marcharA`, tantas veces como el jugador quiera
  hacer clic) y la persecución, que recalcula **cada tick** hacia donde esté el objetivo. `calcularRuta` es
  A* sobre coste de terreno; hoy solo corre al movilizar, un puñado de veces por partida. Con N persecuciones
  activas pasa a correr N veces por tick, dentro de un tick que hoy cuesta 62 ms a 100 asentamientos. No es
  una objeción de diseño —la decisión de recalcular es correcta— es una medición que falta y que puede
  cambiar el CÓMO: recalcular cada tick, cada k ticks, o solo cuando el objetivo se desvía lo suficiente.
- **Bandidos: asimetría aceptada, falta escribirla.** La decisión 8 dice "todas las interacciones son
  intencionales", y lo son *del lado del jugador*: atacar un campamento es una opción de menú a 15. Pero un
  campamento no tiene a nadie que pulse, así que **los bandidos siguen atacando caravanas por su cuenta**
  (`avanzarAtaquesBandidos`). Es coherente —la intención de un NPC es su política— pero es una excepción a una
  regla enunciada como universal y tiene que constar en el canon.
- ~~Desconectarse CON tropas~~ — **CERRADO (decisión 27)**, y en sentido contrario a lo que este documento
  proponía: el jugador se lleva sus tropas consigo y vuelve con ellas al mismo punto.
- ~~La desconexión como escapatoria~~ — **CERRADO (decisión 29)**: desaparición diferida 2:30.
- ~~Caravanas adjuntas huérfanas si se desconectan todos~~ — **CERRADO (decisión 28)**: vuelven a su origen.
- ~~¿La columna sigue moviéndose durante los 2:30?~~ — **CERRADO (decisión 36)**: sí, y de eso dependía la
  aritmética de la decisión 29.
- ~~No había flujo para "salir juntos"~~ — **CERRADO (decisión 34)**: lo que decide el `tipo` es el comando
  que pare la columna, y "salir juntos" es movilizar y sumarse, que el motor ya hace.
- ~~`salirAlMundo` no tenía contrato~~ — **CERRADO (decisión 35)**, con su sub-decisión del carro como saco
  de recursos pendiente de ejecutarse en el paso 3.
- **Qué es exactamente la "capa pública"** de una ciudad ajena (decisión 2): qué edificios se distinguen desde
  dentro y si el mercado deja operar o solo mirar.
- **El Gobernador ausente.** Con la decisión 1, un Gobernador de campaña no puede gobernar. Es el precio
  buscado —los cargos y la delegación pasan a importar— pero hay que ver si la Fase 0 aguanta sin un mecanismo
  de delegación temporal.
- **El tope del Tesorero** sobre cuánto material puede retirar un jugador (anotado por el usuario en la
  decisión 3): gancho previsto, alcance de otra pasada.
- ~~Unirse en campo, ¿con permiso o sin él?~~ — **CERRADO (decisiones 24-25)**: lo decide la política de unión
  que fijó el Líder al formar la columna. Y **los cuatro flecos que abrió alrededor del Líder están cerrados**
  (decisiones 26 y 30-32):
  - ~~¿Quién puede cancelar la marcha?~~ **Solo el Líder** (30). El reparo del Líder ausente lo cubría ya la
    sucesión de la 26, y nadie queda atrapado porque separarse sigue siendo libre.
  - ~~¿Qué pasa si el Líder se desconecta?~~ **Sucede el más antiguo** (26).
  - ~~¿Puede un invitado ser elevado a Líder?~~ **Ya no hay invitados** (33).
  - ~~¿Qué pasa con una petición que nadie contesta?~~ **Caduca a los 10 s y cuenta como rechazada** (32).

  **Y no queda nada vivo de este bloque.** La columna sin Líder —que había que representar y probar— no puede
  existir desde la decisión 33: sin invitados, todo participante es elegible.
- ~~Unirse a un ejército aliado rompe `Ejercito.faccionId`~~ — **CERRADO de raíz (decisión 33)**: no se puede
  unir. El campo vuelve a ser la Facción de todos los que van dentro, y los dos detalles que la figura del
  invitado dejaba pendientes —separarse en territorio anfitrión, contar o no para el repliegue— se van con
  ella. Lo que hereda el diseño en su lugar es el precio: **una alianza sin brazo militar**, porque el
  combate es de una entidad contra una y dos columnas amigas no suman. Anotado en la decisión 33.
- ~~De dónde sale el `origenAsentamientoId` de una columna que nace al separarse~~ — **CERRADO (decisión
  23)**: no cambia, se hereda. Queda un caso estrecho, y solo hay que elegir al implementar: el de quien se
  unió **en campo** viniendo de su propia columna. Al fundirse adoptó el origen del ejército, así que al
  separarse se replegaría a la plaza de la que salió ESE ejército y no a la suya. **La decisión 33 lo deja
  mucho más pequeño**: ahora siempre es un compatriota replegando a una plaza de su propia Facción, donde
  antes podía ser un aliado replegando a territorio ajeno. La alternativa sería un origen por participante
  en vez de uno por columna, que es más máquina de la que el caso merece.
- **Las velocidades se quedan como están** (decisión del usuario, segunda ronda). Se anota porque con
  persecución explícita la velocidad decide quién puede forzar un combate y quién escapa, así que pasa a ser
  mucho más determinante que hoy. No se toca ahora; se mide en el paso 10.
- **La capital (§13) y el Liderazgo (§11)** siguen siendo mecánicas aparte. Esta las roza pero no las cierra.

## 10. Riesgos

1. **El paso 7 (presencia en la autorización) toca ~24 comandos de golpe.** Es el que puede romper cosas en
   sitios que nadie relacione con esta mecánica. Mitigación: va después del paso 6, con la proyección ya
   correcta, y cada comando migrado lleva su test de rechazo por ausencia.
2. **La migración de snapshots no tiene marcha atrás.** Un snapshot migrado no vuelve a cargar en una versión
   anterior. Mitigación: respaldo antes de la primera carga migrada (`server/respaldos.ts` ya existe).
3. **El NPC de gobernanza no sabe estar en ningún sitio.** `npcGobernanza.ts` funda, construye y recluta como
   si estuviera en todas partes. O se le da ubicación, o se le exime explícitamente — y si se le exime, el
   batch deja de medir el juego que se está diseñando.
4. **La continuidad con Fase 1 se puede erosionar sin querer.** Las cuatro reglas de §7.3 no las comprueba
   ningún test hoy. Vale la pena un guardián de arquitectura para la 2 (ningún plazo ni velocidad en ticks),
   que es la más fácil de romper por descuido — y la que más cuesta deshacer después, porque queda repartida
   por constantes y campos persistidos.
5. **El batch se queda sin combates y nadie se entera.** Es el riesgo más silencioso de la segunda ronda: al
   dejar de dispararse por geometría, el combate solo ocurre si alguien lo pide, y en el laboratorio no hay
   nadie pidiendo. Las constantes militares ya calibradas —poder, varianza, bajas, veteranía— se seguirían
   midiendo sobre un mundo en paz sin que ninguna prueba fallara. Mitigación: la política de persecución del
   NPC entra en el mismo paso 8, no después, y el batch reporta número de combates por corrida como métrica
   de primera clase.
6. **Quitar el disparo automático rompe muchos tests a la vez.** `resolverEncuentros` y la llegada-asedio
   están cubiertos por tests que asumen que el choque ocurre solo. No es riesgo de diseño sino de ejecución:
   conviene congelar antes el invariante nuevo ("ningún combate sin comando") y migrar los viejos contra él,
   en vez de ir arreglándolos de uno en uno.
