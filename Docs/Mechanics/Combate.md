# Combate con números: encuentros, asedio y conquista

## 1. Alcance y fuentes

Cubre cómo se **resuelve** un choque cuando el servidor lo calcula con números (sin Unity): columna contra columna, columna contra caravana, columna contra campamento de bandidos, ejército contra plaza (asedio), y lo que pasa cuando una plaza **cae** (conquista, saqueo, desalojo). El movimiento previo (llegar, perseguir, alcanzar a 3) está en [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md).

- **Módulos:** `src/engine/combate.ts` (resolución), `src/engine/ejercitos.ts` (`atacarColumna`, `interceptar`, `atacarCampamento`, `asediarPlaza`, `trasDerrota`, `cargarBotin`, validaciones, `enBatalla`), `src/engine/tropa.ts` (`poderEscuadron`, `defensaDe`, `alTopeDeBatalla`), `src/engine/chatarra.ts`, `src/engine/heroe.ts` (heridos), `src/session/comandos/interaccion.ts` (comando `atacar`).
- **Reescrito el 2026-10-13** para el mundo reactivo (los choques nacen de **encuentros exactos**, no de un barrido por tick) y con la guerra logística (caravanas A/B sin inmunidad, saqueo de pueblos, avanzadas) y la calibración. El «cómo funcionaba» está en `typescript-final:Docs/Mechanics/Combate.md`.
- **Canon:** Doc 5.2 (combate y modalidades), 5.12.3–5.12.4 (encuentros y asedio), 5.12.9 (conquista), 5.15 (batalla, guarnición), 5.16 (héroe, bajas, heridos), 3.10 (combate de caravanas), 4.2.2 (saqueo de pueblos), 5.17 (avanzadas), 2.4.1 (guerra).
- **No es el combate de Unity.** Con servidores de batalla y al menos un héroe humano, `atacar` no resuelve nada aquí: abre una batalla de Unity (`session/batallas.ts`, ciclo de batalla del contrato) y el resultado se aplica cuando llega. Este documento es lo que corre **sin** ellos (bots contra bots, partidas sin servidor de batalla, y todo lo que Unity no cubre aún). Las consecuencias de ganar o perder (heridos, botín, conquista, desalojo) son las mismas por los dos caminos y están aquí.


### 1.1 Resoluciones que calcula el servidor (sin Unity)

Todas son lo que corre cuando **no** hay servidor de batalla o ningún bando tiene un humano. Cada una tiene su equivalente de Unity (apertura en `session/batallas.ts`); al llegar el resultado de Unity se aplican las **mismas consecuencias**.

| Resolución con números | Función | La dispara | Apertura equivalente en Unity |
|---|---|---|---|
| Columna contra columna | `atacarColumna` → `encuentroEntreEjercitos` | `atacar` (objetivo `ejercito`) | `aperturaContraColumna` |
| Columna contra caravana | `interceptar` → `interceptarCaravanaConEjercito` | `atacar` (objetivo `caravana`) | `aperturaContraCaravana` |
| Columna contra campamento de bandidos | `atacarCampamento` → `atacarCampamentoConColumna` | `atacar` (objetivo `campamento`) | `aperturaContraCampamento` |
| Ejército contra plaza (asedio) | `asediarPlaza` → `asediarConEjercito` | `atacar` (objetivo `asentamiento`) | `aperturaDeAsedio` |
| Ejército contra un pueblo extractor (saqueo) | `saquearPueblo` | `atacar` (objetivo `pueblo`) | apertura de saqueo (§7.3) |
| Ejército contra una avanzada | `atacarAvanzada` | `atacar` (objetivo `avanzada`) | apertura de avanzada (§7.4) |

El motor común es `resolverCombate` (§4), salvo caravana y campamento, que comparan contra una cifra fija. (El antiguo asedio de asentamiento a asentamiento, `iniciarAsedio`, se retiró el 2026-10-12: nadie lo usaba.)

> **Decisiones del autor del 2026-10-19 (sistema 4)** que ya están aplicadas en este documento: **el héroe suma 15** al poder de su bando (§3.3, `MILITAR.poderPorHeroe`), **las murallas son una fase propia** posterior al sistema militar (recinto 0 hasta entonces, §6), **la caravana aparcada en una plaza conquistada se destruye** (§8; el código de la fase 3B-2 se corrigió con su prueba), **el equipo de asedio no tiene efecto** en el combate con números y `umbralCapturaCaravana` sale del bloque `MILITAR` (§7.1). La resolución en Rust es la fase 4A-2; las cifras de los bloques ya son las vigentes.

> **Implementado en Rust (2026-10-19, fase 4A-2):** crate `crates/combate`, reglas puras sin depender del núcleo (enteros, `Rng` determinista, cifras de los bloques de este documento, de `Batalla_Unity_Ciclo`, de `OCUPACION` en `Murallas.md` y de `CAMPAMENTOS_BANDIDOS` en `Generacion_Del_Mundo.md`). Módulos: `resolucion` (§3.3-§4: poder, tirada, `resolver_combate`, bajas, chatarra), `seleccion` (§3.1), `botin` (§5), `asedio` (§6), `caravana` (§7.1), `bandidos` (§7.2 y Generacion §13), `pueblo` (§7.3), `avanzada` (§7.4), `conquista` (§8-§10) y, para el ciclo de Unity, `ciclo`, `resultado` y `union` (ver `Batalla_Unity_Ciclo.md`). **Tiradas del generador, por escrito:** 2 en `resolver_combate` (encuentro, asedio con defensores, saqueo de un pueblo y ataque a una avanzada operativa con defensores; el atacante primero), 1 en el asalto a una caravana, en el ataque a un campamento y en el acoso de los bandidos, 1 en `nivel_al_azar`, 2 por intento en `punto_del_anillo`, y 0 en las ramas sin combate (plaza protegida, sin defensores con soldados, sin atacantes, ruinas). Las cifras que el TypeScript escribía en el código están ahora en `COMBATE_NUMERICO` y `AVANZADA` (§12).
>
> // DUDA (decidido al implementar, a revisar por el autor): (1) el saqueo de un pueblo aplica la `ventajaAguante` (×1,25) **además** de la ventaja de 1,05 de todo defensor (×1,3125); (2) sin Unity, un asalto de caravana que gana el atacante quita los **últimos** carros de la lista, y un aguante sin carros que quedar elimina la caravana pero devuelve el 30 % salvado para que llegue «con los carreteros»; (3) «sin defensores» en un asedio, un saqueo o una avanzada es no tener **escuadras con soldados**: un héroe sin tropa suma 15 al poder cuando hay combate, pero no basta para abrirlo; (4) para atacar un campamento de bandidos basta un bando que combata (el TypeScript pedía soldados); (5) el punto del anillo de bandidos se sortea en el cuadrado que rodea el anillo y se descarta fuera de él (enteros, sin senos ni cosenos): es uniforme en el área y no en el radio; (6) unas ruinas de ciudadela no tienen invulnerabilidad y atacarlas no las destruye; (7) los bloques de bandidos de `Generacion_Del_Mundo.md` llevan ya los valores calibrados (`radioAtaqueCaravana` 7, anillo 30-50).

> **Implementado en Rust (2026-10-20, fase 4C): los encuentros.** Dos columnas (de la misma clase), una columna y una caravana, dos caravanas o una columna y una plaza ajena que quedan a ≤ `radioEncuentro` dejan un **hecho** en el instante exacto del cruce (`ejercito.encuentro`, `caravana.encuentro`, `asentamiento.encuentro`), filtrado con el estado de ese instante: de Facciones distintas y no aliadas, en el mapa, sin batalla y con algún héroe sano en la columna. No abre ningún combate y no deja estado «a la espera»: atacar es una orden (fase 4D), validada por la geometría del momento en que se da. Detalle en [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) (nota de la fase 4C); las relaciones entre Facciones son por ahora el andamiaje de administración `FijarRelacion`.

> **Implementado en Rust (2026-10-20, fase 4D): el combate con números en el núcleo.** `Atacar { heroe, objetivo, declara_guerra }` (95; `ColocarBandidos` 96 y `DarEscuadra` 97 son andamiaje de administración) está en `nucleo/combates.rs`, `conquistas.rs` y `bandidos.rs`, sobre `crates/combate`. **Todo se comprueba antes de tocar nada o de gastar una tirada** (una orden rechazada no deja rastro), en este orden: el héroe (suyo, sano, a bordo de una columna fuera de batalla y que no sea una formación; cualquier participante, no solo el Líder), el objetivo (existe y está en el mapa), el alcance (`radioEncuentro`), la protección de los campamentos de mercenarios (`radioProteccion`, `<=`: ni desde allí ni contra lo que esté allí; se lee ahora de `MERCENARIOS.radioProteccion`) y la guerra. Entre Facciones distintas hace falta guerra (`FijarRelacion`); `declara_guerra` la declara (`faccion.guerra_declarada`), pero solo cuando todo lo demás ya vale, y la deja como la única relación del par, sin arrastre. Una columna sin Facción ni exige ni declara guerra, y no conquista. A los tuyos y a un aliado no se les ataca ni con declaración. Códigos de rechazo en `rechazos::COMBATE_*`.

> **Implementado en Rust (2026-10-21, fase 4E).** El combate con números sigue siendo el camino cuando **no hay servidores de batalla** (`Reglas::batallas_en_unity` falsa) o cuando es bot contra bot ([`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §2): `atacar` comprueba lo mismo y, en lugar de resolver, abre la batalla de Unity si hay un humano en los dos bandos. Las consecuencias comunes se comparten: `caer_caravana` (los carros que se pierden —los últimos con números, los que dice Unity— con su animal, la escolta a 0 y de vuelta al campamento de su héroe, la carga salvada del aguante), `conquistar_plaza`, `perder_la_mitad_del_carro`, el botín de los bandidos y el saqueo del pueblo. Un asedio con números sigue sin convocatoria: una sola columna (la convocatoria de 30 s y los refuerzos son de la batalla de Unity).
>
> - **Columna** (§5): de su misma clase, en el mapa, con algún héroe sano; dos tiradas; gana el poder. El derrotado pierde la mitad de su carro (y una personal suelta su presa), **todos sus participantes** quedan heridos (`heridoMinutos` 2; si ninguno queda sano, ni persigue ni lo persiguen: `columna.presa_a_cubierto`) y el ganador carga lo robado más la chatarra de las bajas de los dos bandos hasta su capacidad de carga (`capacidad_de_carga`). Hecho: `combate.resuelto`.
> - **Caravana** (§7.1): la suelta (`en_transito` o `retornando`; la adjunta se ataca al ejército, `combate.caravana_adjunta`; la parada no está en el mapa, `combate.a_cubierto`). El tipo sale de la zona **en el instante del ataque** (`Nucleo::zona_de_caravana`, con `en_zona`): B (aguante) en la zona de una plaza del dueño o de su alianza o vasallaje, A (disputa) en cualquier otra o fuera de todas. Una tirada. Gana el atacante: `aplicar_al_dueno` (la mitad de los carros, hacia arriba, y en B el 70 % de la carga), el atacante carga la carga perdida, la **escolta cedida queda a 0 y vuelve al campamento de su héroe** y, si no queda ningún carro, la caravana se pierde (`comercio.caravana_perdida`). Gana la caravana: el atacante pierde el 25 %, la escolta el 5 % y todos los participantes de la columna quedan heridos. Hechos: `combate.caravana_interceptada` / `combate.caravana_resiste`. // DUDA: una escolta sin soldados en pie cuenta como no tenerla (defienden los carreteros, 26); un aguante que se queda sin carros salva igual el 30 % y se entrega **al almacén de la plaza de origen** (el documento dice «llega con los carreteros» sin decir a dónde).
> - **Campamento de bandidos** (§7.2): sin guerra; basta una columna con un héroe sano (no hace falta tropa). Una tirada. Cae: el oro del nivel para **cada héroe que combatió** (hasta 5; `botin_de_bandidos` con la ventana de 24 h por héroe, en `EstadoCombate.destruidos`) va al carro y se agenda su reaparición; aguanta: todos los participantes heridos y la mitad del carro perdida. Hechos: `combate.campamento_destruido` / `combate.ataque_campamento_fallido`.
> - **Plaza** (§6): solo un ejército con soldados, de otra Facción, a ≤ `radioEncuentro`. Una plaza protegida **se rechaza** (`combate.plaza_protegida`, sin rastro) en vez de «rebotar» con un hecho. Defienden (`defensa_de_plaza`) la guarnición de los residentes (siempre), los residentes dentro, sanos y conectados con su loadout activo (hasta 15) y, en el cupo que dejan, los visitantes de la propia Facción o de una aliada o vasalla que están dentro, con las escuadras de su columna aparcada (una columna en el mapa, a la puerta, no defiende: 2026-10-22). Sin defensores con soldados cae **sin combate ni tirada**. Heridos: sus defensores en persona si cae, todos los participantes del ejército si resiste; la chatarra va al carro del ejército si cae y al almacén de la plaza si resiste. Hechos: `combate.asedio_conquista` / `combate.asedio_resistido`. El ejército sigue fuera; murallas con recinto 0 (multiplicador 1) y equipo de asedio sin efecto.
> - **Pueblo extractor** (§7.3): un ejército de otra Facción que la de la madre; el pueblo ha de estar activo. La guarnición de serie nace completa la primera vez que se la necesita (`EstadoCombate.guarniciones`) y se repone a las 6 horas con un evento (`SucesoCombate::ReponeGuarnicion`, prioridad 3, ligado a `entidad_guarnicion`). Si el atacante gana se lleva `saquear`: lo que el almacén guarda por encima de la mitad, hasta lo que cabe en el carro; si no, quedan heridos todos sus participantes. // DUDA: el reloj de 6 h empieza en el primer combate que deja la guarnición por debajo de 30 (no solo si «cae») y no se reinicia mientras corre; con la guarnición a 0 esperando su reposición, el pueblo se saquea sin combate ni tirada; la chatarra, si el pueblo resiste, va al almacén de la madre. Hechos: `combate.pueblo_saqueado` / `combate.pueblo_resiste`.
> - **Avanzada**: ver la nota de la fase 4F, más abajo.
>
> Las tiradas son las del documento: 2 en columna, plaza con defensores y pueblo con guarnición; 1 en caravana, campamento y acoso; 0 en las ramas sin combate. La experiencia de escuadra (`aplicar_bajas`) y las bajas se vuelcan al estado, y cada columna que combatió **re-anota su consumo y su velocidad** (`recomponer_columna`, también reprograma su deserción). Sin Unity (la fase 4E la añade) todo se resuelve con números.

> **Implementado en Rust (2026-10-22, fase 4F): las avanzadas.** `nucleo/avanzadas.rs` (`atacar_avanzada`, `cae_avanzada`) sobre `combate::avanzada` y `asentamiento::avanzadas`; foto `BAN15` (hoy `BAN16`). La obra de mapa (colocar, entregar, mejorar, reparar, destruir a mano) está en [`Economia_Construccion.md`](Economia_Construccion.md) §14.1.
>
> - **Se valida antes de tocar nada ni gastar una tirada**, en este orden: la avanzada existe (`combate.avanzada_no_existe`) y tiene dueño (`combate.avanzada_sin_dueno`: unas ruinas de ciudadela sin dueño no tienen Facción con la que estar en guerra); el atacante es un ejército (`combate.solo_un_ejercito`) con Facción; la avanzada no es una obra pendiente (`combate.avanzada_en_obra`: se ve pero no se ataca) ni un fuerte en su día de invulnerabilidad (`combate.avanzada_invulnerable`); el alcance (`radioEncuentro`), la protección de los campamentos de mercenarios y la guerra de siempre (`comprobar_agresion`: a los tuyos y a un aliado no se les ataca ni con `declara_guerra`; la guerra solo se declara cuando todo lo demás vale); que no esté en una batalla de Unity; y que el ejército lleve soldados. Unas ruinas de ciudadela que ya no tienen almacén son «sin cambios».
> - **Defienden** la guarnición de serie y la de jugadores (escuadras de héroes residentes de la madre dejadas a su puerta, [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md) §7): sin héroes, porque nadie está dentro, así que no suma el `poderPorHeroe` del defensor. **Solo defiende lo que está dentro** ([`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §11.1, decisión del autor del 2026-10-22): una columna amiga a su puerta, en el mapa, no defiende, ni con números ni uniéndose a la batalla. Se resuelve como un asedio (`resolver_ataque`): dos tiradas con defensores en pie; **unas ruinas no tienen defensores** y caen sin combate ni tirada; una operativa con toda su guarnición aniquilada esperando la reposición cae igual, sin combate. El atacante es la columna del héroe (tope de 15 héroes, como un asedio).
> - **La guarnición de serie** nace completa la primera vez que se la necesita: las escuadras de la tabla de `AVANZADA.guarnicionDeSerie` con la tropa que eligió el Gobernador para cada escalón (`ElegirTropaDeAvanzada`, 111, solo entre las que la madre puede reclutar) o `tropaPorDefecto` (la milicia de lanceros del canon, en los tres escalones). Mermada, se repone gratis a las 6 horas (`reposicionMinutos`) del primer combate que la deja incompleta (un evento, `SucesoCombate::ReponeAvanzada`, prioridad 3; no se reinicia mientras corre); la reposición rehace la composición si desde entonces se eligió otra tropa o la avanzada subió de etapa. Elegir tropa o subir de etapa con la guarnición completa la rehace al instante; con la guarnición mermada, llega con la reposición (cambiar de tropa no cura a nadie).
> - **Si gana el atacante** (`cae_avanzada`, la misma por los dos caminos): se lleva **del almacén lo que le quepa en los carros** de sus columnas (por rondas, `combate::avanzada::saquear`) y, con lo que sobre, la chatarra de las bajas; la **atalaya** queda destruida (sale del cupo, del espacio y del estado); el **fuerte** queda en ruinas con `invulnerable_hasta = ahora + 1 día` (con su cita `SucesoCombate::FinInvulnerabilidad`, prioridad 2) y, si ya estaba en ruinas, pasado el día, queda destruido; la **ciudadela** queda en ruinas sin invulnerabilidad y no se destruye nunca (atacarla otra vez solo saquea lo que quede). Al perderse el puesto: la guarnición de jugadores queda a 0 y vuelve al campamento de su héroe (derrota, Doc 5.15.2b), la obra de mejora o reparación en marcha se pierde y la guarnición de serie se borra (renace completa al repararla; las elecciones del Gobernador se conservan). Si **resiste**: heridos todos los participantes del ejército, la chatarra al almacén de la madre y la guarnición de serie mermada, con su experiencia.
> - **Hechos:** `combate.avanzada_destruida`, `combate.avanzada_en_ruinas`, `combate.avanzada_saqueada` (unas ruinas de ciudadela que siguen siéndolo) y `combate.avanzada_resiste` (datos: avanzada, etapa, facciones, poderes y bajas, heridos, chatarra, saqueado), más `avanzada.destruida` (`motivo` 1) y `avanzada.en_ruinas` (`invulnerableHasta`), `avanzada.invulnerabilidad_terminada` y `avanzada.guarnicion_repuesta`.
> - **Con Unity** (`batallas_en_unity` y un humano en la columna atacante): se abre una batalla de contexto `avanzada` (5) con capacidad de asedio (15 por bando), 30 minutos y **sin convocatoria** (decisión del autor, 2026-10-22); ver [`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md). Unas ruinas no abren batalla.
>
> **Decisiones del autor del 2026-10-22 sobre la 4F**, ya aplicadas: solo defiende lo que está dentro (una columna amiga a su puerta no cuenta); la reposición de la guarnición de serie son 6 h; la tropa por defecto es la del escalón 2 del canon en los tres escalones y el dueño elige otra por escalón solo entre las que la madre puede reclutar; el atacante que gana se lleva todo lo que le quepa del almacén (el pueblo protege el 50 %, la avanzada no); y el almacén es por recurso.
>
> // DUDA (decidido al implementar, a revisar por el autor): la chatarra de una avanzada que resiste va al almacén de la madre, como la del pueblo.

> **Implementado en Rust (2026-10-22, decisiones de 4F).** Foto `BAN16`. (1) **Solo defiende lo que está dentro** ([`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §11.1): `defensa_de_plaza`, `atacar_pueblo` y `atacar_avanzada` ya contaban solo lo de dentro; el cambio está en el ciclo de Unity, donde `unirse_a_batalla` pide estar dentro para defender un asedio (`presencia.ubicacion` en esa plaza), un pueblo o una avanzada (nadie puede estar dentro de estos dos: se rechaza siempre) con `batalla.invalida` («para defender hay que estar dentro…»); la pureza está en `combate::union` (`Reparto::dentro`, `ErrorUnion::HayQueEstarDentro`). Una caravana no se entra y no pide estar dentro. (2) **`ElegirTropaDeAvanzada`** exige, además del escalón y la tecnología de la Facción, el edificio activo de la madre al nivel que pide la tropa (`militar::reclutamiento::edificio_basta`, el mismo que usa reclutar): `avanzada.tropa_sin_edificio`. La elección no se revisa después. (3) Reposición de la guarnición de serie (6 h), almacén por recurso, botín (todo lo que quepa) y repostar (columna personal o ejército) quedan como estaban, sin duda. (4) **`CancelarObraDeAvanzada`** (116): ver [`Economia_Construccion.md`](Economia_Construccion.md) §14.1.

Un comando, un objetivo: `ejercito` · `caravana` · `campamento` (de bandidos) · `asentamiento` · `pueblo` · `avanzada`. **Los choques nacen de un encuentro exacto** (dos planes que se acercan a ≤ `radioEncuentro`, [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §2): el servidor ofrece atacar en ese instante y el jugador decide; llegar nunca ataca.

1. El héroe debe estar **sano** (`exigirSano`) y en una columna.
2. **Toda agresión entre Facciones exige guerra o la declara** (Doc 2.4.1). Si no hay guerra con el dueño del objetivo, `atacar` sin `declaraGuerra: true` se rechaza con un `detalleError` que enumera las consecuencias (contra quién queda la Facción en guerra y quién entra por arrastre); con la confirmación, **declara la guerra** (cualquier héroe puede) y ataca. Excepción: unirse a un asedio que abrió otro, siendo neutral y admitido, no la declara. Los bandidos y los campamentos no la requieren.
3. **Protección de campamento de mercenarios** (`radioProteccion` = 10): nadie inicia un combate a menos de ese radio de un campamento, ni desde allí ni contra lo que esté allí.
4. Si hay servidores de batalla (`ctx.batallasEnUnity`) y hay humano en alguno de los bandos → se abre batalla de Unity y termina aquí.
5. Si no, se resuelve según el objetivo (§4–§7), y el comando aplica las consecuencias.

Llegar a un sitio **no** ataca; atacar siempre se ordena (humano o bot).

## 3. Quién combate

### 3.1 Un ejército que ataca (`enBatalla`)
De la columna entran **las escuadras de los héroes sanos**, y de ellos solo los `tope` mejores (`BATALLA.capacidad.resto` para encuentros/caravanas/bandidos = 5, `.asedio` para asedios = **15** desde el 2026-10-13: 15 contra 15): orden por **nivel de héroe descendente, luego poder de sus escuadras descendente, luego id**. El resto de escuadras se **aparta**: no combaten ni sufren bajas, y se devuelven intactas a su columna (`conApartadas`). Las de un héroe herido no entran (Doc 5.16.4).

### 3.2 Quién defiende una plaza (`defensaDe`)
- **La guarnición** (escuadras con `enGuarnicion` de los campamentos de los residentes) — sin héroe, siempre defiende, incluso si su dueño está herido.
- Los **residentes que defienden en persona** (`heroesQueDefienden`): residentes **dentro** de la plaza, **sanos** y **conectados** (`!fuera`); con tope `BATALLA.capacidad.asedio` (mismo orden que §3.1). Aportan las escuadras de su **loadout activo** que tengan en el campamento. El resto del campamento no defiende.
- Los **visitantes amigos** (`visitantesQueEntranADefender`, decidido el 2026-10-12): héroes que no residen pero están **dentro** de la plaza en ese momento, sanos y conectados, de la **propia Facción** de la plaza o de una **aliada** (`grupoDePuerta` = `propia` o `aliados`: alianza o vasallaje en cualquier sentido). Ocupan el cupo que dejan los residentes (`BATALLA.capacidad.asedio − residentes que entran`), por nivel y poder, y aportan las escuadras de la **columna que dejaron aparcada al entrar** (`contenedor: ejercito`; ya no está en el mapa: es lo que trajeron y cuenta como dentro). **Solo defiende lo que está dentro** (decisión del autor, 2026-10-22, [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §11.1): una columna amiga en el mapa, a la puerta, no defiende, ni con números ni uniéndose a la batalla. `defensoresDeLaPlaza` junta todo: héroes (los que quedan heridos si la plaza cae) y escuadras. En Unity no hace falta: se unen a la batalla.
- Una escuadra aniquilada (`cantidad ≤ 0`) no cuenta.

### 3.3 Poder de una escuadra y de un héroe
```
poderEscuadron = poderBase(tropa) × cantidad × (1 + nivel × bonusPoderPorNivelEscuadra)
poderBando     = Σ poderEscuadron  +  MILITAR.poderPorHeroe × (héroes sanos del bando que combaten)
```
`poderBase` sale del catálogo `TROPAS_RECLUTABLES`. El nivel de escuadra (1‥`nivelMaximoEscuadra`) sube con experiencia (`experienciaParaSubirEscuadra`); la tropa **prestada** por un campamento de mercenarios no gana experiencia.
**El héroe suma 15** (`MILITAR.poderPorHeroe`, decidido el 2026-10-19, Doc 5.1): aditivo, por cada héroe sano que combate en ese bando, además de sus escuadras. Por eso un bando con héroes y sin una sola escuadra tiene poder, y una columna sin tropa puede combatir con números. // DUDA: el poder de los héroes entra en la suma antes de la cohesión, la varianza y las ventajas (§4), y la cohesión sigue contando solo escuadras.

## 4. La resolución (`resolverCombate`)

Es **un solo motor** para encuentro, asedio, campamento y caravana (con variantes en §5, §6, §7).

```
jitterA = 1 + (rng()·2 − 1) · varianzaCombate        // dos tiradas, A primero
jitterD = 1 + (rng()·2 − 1) · varianzaCombate
poderA  = poderBando(atacantes)                        × jitterA
poderD  = poderBando(defensores) × (1 + bonusCohesionPorEscuadronExtra·(nDef − 1)) × jitterD × multiplicadorDefensor × ventajaDefensor   // nDef = escuadras del defensor
ganador = poderA > poderD ? atacante : defensor       // empate exacto: gana el defensor
ratio   = min(poderA, poderD) / max(poderA, poderD, 1)
bajasGanador  = 0,05 + 0,15 · ratio
bajasPerdedor = 0,30 + 0,40 · (1 − ratio)
```
- La **cohesión** (`bonusCohesionPorEscuadronExtra`) solo la lleva el **defensor** y solo si hay más de una escuadra; el atacante nunca.
- `ventajaDefensor` (1,05) y el `multiplicadorDefensor` (muralla, §6) multiplican solo al defensor.
- Con un bando **vacío** (sin escuadras con soldados y sin héroes sanos) lanza `CombateInvalidoError`.
- **Bajas permanentes** (`aplicarBajas`): cada escuadra pierde `round(cantidad × fracción)` (mínimo 0 de cantidad). La escuadra **persiste como identidad** con `cantidad: 0`. Además cada escuadra gana experiencia: `experienciaGanadaPorVictoria` si su bando ganó, `experienciaGanadaPorDerrota` si no.
- **Consume exactamente dos `rng()`** por combate, en ese orden. Las ramas sin combate (plaza sin defensores, protegida) **no consumen aleatoriedad**: una partida sin asedios hace las mismas llamadas al RNG que antes de existir los asedios.
- **Combate digno** (`esCombateDigno`): `min(poderA, poderD) ≥ NIVEL_FACCION.ratioCombateDigno × max(poderA, poderD)`. Solo un combate digno da experiencia de Facción.
- **Chatarra** (Doc 4.2.1): por cada soldado muerto, de cada bando, el metal de su equipo × `CHATARRA.fraccion`; se suma y la recoge quien gana.
- Evento `combate.resuelto` con poderes, tropas e informe de bajas por escuadra de cada bando.

### 4.1 Experiencia de Facción por combatir
`xpDeCombate = NIVEL_FACCION.xp.combate × (héroes DISTINTOS de las escuadras que participaron en ese bando)`, solo si el combate fue digno; se aplica a la Facción de cada bando. (Una caravana sin escolta no suma por su lado: su defensa es una cifra fija.)

## 5. Encuentro entre columnas (`atacarColumna` / `encuentroEntreEjercitos`)

Validaciones (`validarAtaqueAColumna`, compartidas con la apertura de batalla Unity): no la propia columna; **no a los tuyos ni a un aliado**; **misma clase** (personal contra personal, ejército contra ejército); atacante a ≤ `LOGISTICA.radioEncuentro` (**3** calibrado; 15 en el bloque TypeScript) con héroe sano; el defensor con algún héroe sano.

1. Cada bando pasa por `enBatalla`. El atacante es el «atacante» del motor y el otro el «defensor»; **sin muralla** (multiplicador 1) y, por tanto, el defensor en campo abierto conserva la cohesión y la ventaja de 1,05 (el motor no distingue).
2. **Gana quien dice la resolución por poder** (§4), igual que en un asedio (decisión del autor, 2026-10-10; antes el atacante solo ganaba si dejaba al defensor sin un soldado en pie y casi siempre perdía). Las bajas del 30–70 % al bando que pierde ya vienen aplicadas por `resolverCombate`. `encuentroEntreEjercitos` devuelve `ganador: 'a' | 'b'` (a = atacante).
3. **Secuela de la derrota** (`trasDerrota`): el bando que pierde pierde `MOVIMIENTO.fraccionRobada` (la mitad) de **cada recurso de su carro**; los héroes del perdedor (`vencidos` = **todos** los participantes del bando que pierde, hayan combatido o no) quedan **heridos**. Si era personal, además suelta su presa.
4. **Botín**: lo robado + la chatarra de las bajas va al carro del ganador (`cargarBotin`) hasta `capacidadCargaDe(ganador, caravanas)`; **lo que no cabe se pierde**.
5. Eventos: `combate.resuelto`, `combate.encuentro`; se narran a los **dos** hogares.

## 6. Asedio de una plaza (`asediarPlaza` → `asediarConEjercito`)

Validaciones (`validarAsedio`): atacante **ejército** (una columna personal no abre un asedio); plaza de **otra Facción**; a ≤ 3 con héroe sano; plaza **no protegida** tras su conquista.

Orden (`asediarConEjercito`):
1. **Plaza protegida** (`estaProtegida`: `ahora < protegidaHasta`): rebota, `combate.asedio_resistido`, sin combate ni RNG.
2. **Sin defensores** (ninguna escuadra con soldados en `defensaDe`) → **cae sin combate**, sin RNG: `combate.asedio_conquista` y XP de conquista para el atacante. (Sin atacantes con soldados → «sin nadie con quien combatir», no cae.)
3. En otro caso, `resolverCombate(atacantes, defensores, rng, multiplicadorMuralla)` con
   `multiplicadorMuralla = 1 + (bonoDefensaPorNivel[nivel] − 1) × integridad / nº puertas`, usando solo el **último recinto** de la plaza (el más exterior) — ver [`Murallas.md`](Murallas.md) §9. `integridad` ∈ [0,1]: un anillo a medio cerrar casi no defiende. **Las murallas son una fase propia posterior al sistema militar** (decidido el 2026-10-19): hasta entonces el recinto es 0, el multiplicador vale 1 y no hay cupo de muralla en la guarnición.
4. **Reputación**: si atacante y defensor están **aliados** (se atacó sin romper la alianza), la Facción atacante recibe `REPUTACION.penalizacionAtacarAliado` (la más severa).
5. **XP de Facción**: combate (a ambos, si digno) y, si conquista, `NIVEL_FACCION.xp.conquista` al atacante.
6. Resultado: `conquistado = (ganador == atacante)`. El ejército **sigue fuera y acampa**; conquistar no lo convierte en guarnición (Doc 5.15.5).

Consecuencias que aplica `asediarPlaza`:
- **Heridos**: si hubo combate real (`tropaDefensora` no vacía), quedan heridos los **defensores en persona** si la plaza cae, o **todos los participantes del ejército** si resiste. Sin combate, nadie.
- **Chatarra**: al carro del ejército si conquista (limitada por su capacidad de carga); al almacén de la plaza si resiste.
- **Si cae**: `aplicarConquista` (§8) + `desalojarResidentes` (§9) + `registrarDerrota` (§10).
- Eventos: se atribuyen siempre al hogar del atacante, y a la plaza **solo si resistió** (si cae es del atacante y contárselo a ella lo enseñaría dos veces). Hueco conocido: al vencido no le llega su derrota, porque pierde la plaza por la que la vería.

## 7. Otros objetivos

### 7.1 Interceptar una caravana (`interceptar` → `interceptarCaravanaConEjercito`)
- Rechaza una caravana **adjunta** (se ataca al ejército). **Ya no hay refugio ni inmunidad**: ninguna zona protege del todo a una caravana (Doc 3.10); lo que cambia es el **tipo**.
- **El servidor decide el tipo** por la posición de la caravana **en el instante del ataque** contra los polígonos de zona: **B — Aguante** si está en zona del dueño o de un aliado del dueño (alianza o vasallaje); **A — Disputa de carros** en zona de un enemigo, de un aliado de un enemigo, de un neutral o fuera de toda zona.
- Atacar una caravana exige guerra con su dueño en cualquier sitio, o la declara (§2).
- Defensa = `poderTotal(escolta, cohesión)` si tiene **escolta sin héroe**; si no, `MILITAR.defensaBaseCaravana`. Con escolta por ejército, el ejército.
- **Con números (bot contra bot)**: `gana = poder(atacante) × jitter > defensa × ventajaDeTipo` (**una** tirada de `rng`), con `ventajaDeTipo` = ×1,25 en **B** (placeholder) y ×1 en **A**.
  - **A, si gana el atacante**: pierde la caravana **la mitad de los carros, redondeando hacia arriba**, con su animal; el atacante carga la carga de esos carros en los suyos hasta donde quepa (el resto se destruye). Si pierde todos, la caravana se elimina (Comercio §libera cupo y aplica el cooldown).
  - **B, si gana el atacante**: el dueño pierde el **70 %** de la carga, recurso a recurso, y **el 30 % se salva siempre** (llega en los carros que queden o con los carreteros); también pierde la mitad de los carros (placeholder). El atacante carga lo que le quepa del 70 %.
  - Si gana el defensor: no se pierde nada; bajas del atacante 25 %, escolta 5 % y los héroes del atacante quedan heridos.
- **Con Unity**: el servidor manda el tipo (`modalidad: 'disputa' | 'aguante'`, con los carros identificados) y Unity responde quién ganó y los `carrosPerdidos`. Los carros **no cambian de manos en el mundo: solo la carga**.
- Desaparece el «captura exactamente el 50 %» (`umbralCapturaCaravana` se retiró del bloque `MILITAR` el 2026-10-19).
- Interceptar no da experiencia de Facción. Sí da la defensa de una caravana: **experiencia de Facción por héroe participante** del bando que defiende (Doc 2.2).

### 7.2 Atacar un campamento de bandidos (`atacarCampamento` → `atacarCampamentoConColumna`)
- **Los bandidos no actúan dentro de ninguna zona de influencia** y solo atacan caravanas que pasan junto a su campamento (un **encuentro exacto** con el radio de ataque, 7). Es PvE y no exige guerra.
- La columna debe llevar soldados; compara `poder × jitter` con el `poder` fijo del campamento (**una** tirada). Gana → bajas 5 %, el campamento cae (su botín de oro por héroe, `botinDeBandidos`, lo carga el comando en el carro de la columna: lo que no cabe se pierde); pierde → bajas 25 %, héroes heridos y se pierde la mitad del carro (nadie se la queda). Eventos: `combate.campamento_destruido` si gana y `combate.ataque_campamento_fallido` si pierde; el comando les añade al payload el oro de cada héroe (si cae) o el carro perdido (si aguanta).
- XP de Facción: `NIVEL_FACCION.xp.bandidos / nivel de Facción` por campamento destruido (0 desde `nivelSinXpBandidos`), **sin exigir combate digno**.

### 7.3 Saqueo de un pueblo extractor (`saquearPueblo`)
- Atacante **ejército** de otra Facción (exige guerra o la declara, §2), a ≤ 3 del pueblo. Es una **batalla propia** de saqueo, **nunca de conquista**: el pueblo no cambia de dueño ni se destruye.
- Defensa: la **guarnición de serie** (30 mineros sin dueño, IA, fuera de cupo) y quien lo defienda **desde dentro** (hoy nadie puede entrar en un pueblo: solo los mineros); se resuelve con `resolverCombate` y la ventaja del defensor de B.
- **El pueblo protege el 50 % de su capacidad**: si gana el atacante, se lleva solo lo que esté **por encima de la mitad**, recurso a recurso, hasta lo que le quepa en los carros; si gana el defensor, nada. Un almacén de 500: con 250 o menos, nada; con 275, 25; con 500, 250.
- La guarnición de serie, si cae, se repone gratis en 6 horas.

### 7.4 Ataque a una avanzada (`atacarAvanzada`)
- Solo un **ejército** la abre; dura 30 minutos como un asedio. Exige guerra o la declara.
- Defienden la **guarnición de serie** (por etapa: atalaya 1 escuadra del escalón 2; fuerte 3 + 2 del escalón 3; ciudadela 5 del escalón 3 + 3 del escalón 4) y la de jugadores (14 / 45 / 90 puntos de Liderazgo por héroe residente de la madre). Solo defiende lo que está dentro: una columna amiga a su puerta, en el mapa, no (hoy nadie puede entrar en una avanzada).
- Si gana el atacante: **atalaya destruida**; **fuerte en ruinas** con 1 día de invulnerabilidad (si sigue en ruinas pasado ese día y lo atacan y gana el atacante, queda destruido); **ciudadela en ruinas, nunca destruida**. Del almacén, el atacante se lleva lo que le quepa en los carros. En ruinas no funciona nada (visión, guarnición, reabastecimiento) y la guarnición de jugadores queda a 0 y vuelve al campamento de su héroe. Reparar cuesta el 50 % de construir la etapa.

## 8. Conquista: lo que le pasa a la plaza (`aplicarConquista`)

Pura y determinista (no usa RNG). La ciudad cambia de dueño entera:
- **Cargos**: todos los cargos locales quedan vacíos. **Fundadores y casas compradas**: vacíos.
- **Población**: pesants, artesanos y nobleza pierden `OCUPACION.fraccionSaqueoPoblacion` (`floor` de lo que queda).
- **Edificios**: de los `activo`, ordenados por id, exentos el Centro Urbano y la **primera Granja y la primera Leñera** activas, los primeros `ceil(fraccionEdificiosDanados × n)` pasan a `en_cola` marcados `danado` (se reconstruyen por `fraccionCosteReconstruccion`). Un Almacén o Granero dañado deja de aportar capacidad y lo guardado que ya no cabe se pierde.
- **Recintos**: cada recinto completo pierde `floor(fraccionDanoMuralla × celdas)` de `avance` (mínimo −1): no cae, deja de dar el multiplicador pleno hasta repararse por obra normal.
- **La obra de ascenso en curso se pierde** sin devolución.
- **Las decisiones del gobierno derrotado caen**: políticas activas, reserva manual del Tesorero, pausa de auto-construcción y recetas paradas.
- `medidorMantenimiento = 100`; `protegidaHasta = ahora + proteccionMinutos` (inmune a otro asedio un día); `ocupacionHasta = ahora + duracionMinutos` (recaudación y crecimiento al `factorRecaudacion` / `factorCrecimiento`, mantenimiento congelado).
- **Queda sin guarnición**: nadie la guarnece por haberla ganado; si los conquistadores quieren defenderla, tienen que pasar a residir en ella.
- **Sus pueblos extractores y sus avanzadas pasan al conquistador** con la zona (en una **ruina** de la madre, los pueblos y las atalayas/fuertes desaparecen y la ciudadela queda como ruina sin dueño). Las **caravanas aparcadas** en la plaza se destruyen (Doc 3.13.7).

## 9. Desalojo (`desalojarResidentes`)

Nadie se queda dentro de una plaza enemiga:
- Quien estaba **dentro**: si tiene una columna aparcada a la puerta, vuelve a ella; si no, nace una **columna personal** estacionada en la plaza (`${plaza}-salida-${héroe}-${instante}`), con el carro vacío y las escuadras con las que **defendió y sobrevivieron**; su `faccionId` es la de la plaza si residía, `''` si era visitante. Un residente herido no defendió, así que sale solo.
- **Todos los residentes**: el resto de su campamento (guarnición incluida: tropa **prestada** que ya no puede seguir donde está, Doc 5.15.2b) queda a 0 y pasa con ellos al **asentamiento más cercano de su Facción** (empate por id), donde pasan a residir (se añaden a sus `casasCompradas`). **Sin ninguno**, pasan al **campamento de mercenarios más cercano** a la plaza perdida (`acogerEnCampamentoMasCercano`), con sus escuadras a 0 pero suyas, con su nivel y experiencia. Lo que llevaban fuera (columna, escolta) no se toca.
- El traslado **no mira el cupo de viviendas** del destino (`ponytail:` anotado; si hace falta tope, ahí).

## 10. Derrota de Facción (`registrarDerrota`)
Si tras caer la plaza la Facción perdedora **no conserva ningún asentamiento**, queda anotada `derrotadaPor = ganadora` (o `null` si la última colapsó sin que nadie la derrotara). Qué hace después con sus héroes lo decide quien los gobierna.

## 11. Heridos

`herir(heroes, ids, ahora)`: `heridoHasta = ahora + HEROE.heridoMinutos`; a quien ya lo está no se le alarga. `heridosEn(heroes, ahora)` da el conjunto vigente. Un herido **no persigue, no se le persigue, no entra en batallas** y, si todos los héroes de una columna lo están, esa columna es intocable. Sustituye a la antigua Tregua.

## 12. Parámetros

<!-- parametros:MILITAR -->
```json
{
  "racionPorSoldadoPorMinuto": 0.15,
  "regeneracionMoralPorMinuto": 5,
  "degradacionMoralSinRacion": 20,
  "desercionFraccionPorMinutoSinMoral": 0.05,
  "bonusPoderPorNivelEscuadra": 0.01,
  "nivelMaximoEscuadra": 10,
  "experienciaParaSubirEscuadra": [
    5,
    10,
    20,
    40,
    40,
    40,
    40,
    40,
    40
  ],
  "experienciaGanadaPorVictoria": 1,
  "experienciaGanadaPorDerrota": 0.5,
  "ventajaDefensor": 1.05,
  "bonusCohesionPorEscuadronExtra": 0.1,
  "varianzaCombate": 0.15,
  "defensaBaseCaravana": 26,
  "poderPorHeroe": 15
}
```
<!-- /parametros -->

Los bloques de este documento son los vigentes (2026-10-19). `radioEncuentro` (3) está en `LOGISTICA` ([`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md)), `radioProteccion` (10) en `MERCENARIOS`, `BATALLA.capacidad.asedio` (15) en [`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) y el radio de ataque de los bandidos (`radioAtaqueCaravana`) en `CAMPAMENTOS_BANDIDOS` de [`Generacion_Del_Mundo.md`](Generacion_Del_Mundo.md). La `ventajaDefensor` del aguante (B) es `COMBATE_NUMERICO.ventajaAguante` (×1,25, placeholder).

<!-- parametros:COMBATE_NUMERICO -->
```json
{
  "bajasGanadorBase": 0.05,
  "bajasGanadorPorRatio": 0.15,
  "bajasPerdedorBase": 0.3,
  "bajasPerdedorPorRatio": 0.4,
  "bajasFijasGanador": 0.05,
  "bajasFijasPerdedor": 0.25,
  "ventajaAguante": 1.25,
  "fraccionCargaPerdidaAguante": 0.7,
  "fraccionCarrosPerdidos": 0.5,
  "fraccionProtegidaPueblo": 0.5
}
```
<!-- /parametros -->

`COMBATE_NUMERICO` reúne las cifras que el motor TypeScript tenía escritas en el código (añadido el 2026-10-19 al implementarlo en Rust, fase 4A-2): las bajas de `resolverCombate` (§4: `bajasGanador = base + porRatio · ratio`, `bajasPerdedor = base + porRatio · (1 − ratio)`), las **fijas** de un combate contra una cifra (caravana y campamento: `bajasFijasGanador` al bando que gana y `bajasFijasPerdedor` al que pierde, escolta incluida), la ventaja del aguante (§7.1), el 70 % de carga que pierde un aguante, la fracción de carros que pierde un asalto de caravana (la mitad, hacia arriba) y la parte del almacén de un pueblo que no se puede saquear (§7.3).

<!-- parametros:AVANZADA -->
```json
{
  "invulnerabilidadMinutos": 1440,
  "cupoGuarnicion": {
    "atalaya": 14,
    "fuerte": 45,
    "ciudadela": 90
  },
  "guarnicionDeSerie": {
    "atalaya": {
      "2": 1
    },
    "fuerte": {
      "2": 3,
      "3": 2
    },
    "ciudadela": {
      "3": 5,
      "4": 3
    }
  },
  "tropaPorDefecto": "milicia_lanceros",
  "reposicionMinutos": 360
}
```
<!-- /parametros -->

`AVANZADA.invulnerabilidadMinutos` es el día de invulnerabilidad de un fuerte en ruinas (§7.4, Doc 5.17). `cupoGuarnicion` es el cupo de la guarnición de jugadores **por héroe residente de la madre y por avanzada**, en puntos de Liderazgo (14 / 45 / 90). `guarnicionDeSerie` dice cuántas escuadras de IA, fuera de cupo, defienden cada etapa en cada escalón (Doc 5.17). `tropaPorDefecto` es la tropa de cada escalón mientras el Gobernador no elija otra (la milicia de lanceros del canon, la misma en los tres escalones; el Gobernador elige por escalón solo entre las que la madre puede reclutar: la tecnología de la Facción y el edificio activo al nivel que pide), y `reposicionMinutos` lo que tarda en reponerse gratis esa guarnición (**6 h**, decisión del autor del 2026-10-22; las mismas que los mineros de un pueblo). Las cifras de la obra, el cupo de avanzadas y el almacén están en `AVANZADAS` de [`Economia_Construccion.md`](Economia_Construccion.md) §14.

<!-- parametros:HEROE -->
```json
{
  "bronceInicial": 500,
  "topeAtributo": 100,
  "heridoMinutos": 2
}
```
<!-- /parametros -->

<!-- parametros:CHATARRA -->
```json
{
  "fraccion": 0.5
}
```
<!-- /parametros -->

<!-- parametros:NIVEL_FACCION -->
```json
```
<!-- /parametros -->

## 13. Invariantes

- Un combate con números consume **exactamente** las tiradas indicadas (2 en `resolverCombate`, 1 en caravana y campamento, 0 en las ramas sin combate), en el orden de la cola de eventos: lo vigila la prueba de reproducción.
- Las bajas son permanentes y una escuadra nunca desaparece: queda a `cantidad: 0` hasta que se rellene.
- Una columna con todos sus héroes heridos no se puede atacar ni perseguir.
- Una plaza conquistada es inatacable durante `proteccionMinutos`.
- Con números defienden una plaza sus residentes y, en el cupo que dejan, los héroes de la propia Facción o aliados que estén dentro; no neutrales ni enemigos (`defensaConAliados.test.ts`, `batallas.test.ts`).
- Tras `desalojarResidentes` ningún héroe queda con `ubicacion` dentro de la plaza conquistada.

## 14. Casos límite y decisiones que parecen raras

- **Todos los participantes del perdedor quedan heridos**, hayan entrado en la batalla o no (el tope de 5 héroes por bando aparta a los demás, pero la derrota es de la columna).
- **El carro se pierde por mitades aunque el héroe que lo llevaba estuviera herido y no combatiera**: el carro es de la columna.
- **Un encuentro en campo no da ventaja de defensor «de verdad»**: el motor da cohesión y 1,05 al segundo bando; por eso el orden atacante/defensor importa y el comando lo fija (quien ordena es el atacante).
- **El multiplicador de muralla usa solo el último recinto** de la lista, no el mejor: los recintos interiores son «historia en piedra» (ver `Murallas.md`).
- **Defender al vasallo también cuenta con números** (2026-10-12): tras resolver con números un asedio, un choque entre columnas o una intercepción cuyo defensor es de una Facción vasalla, se aplica `ajustesPorDefensaDeVasallo` (Diplomacia §5.9) con los héroes que de verdad defendieron (los de la plaza que entran a defender, los de la columna atacada; una caravana suelta no lleva ninguno). Una señora cuenta como presente si tiene un héroe dentro de la plaza de su vasalla cuando la atacan (§3.2); si no, y la vasalla pierde, paga la penalización.
- **El mismo asedio puede pasar por Unity o por números**; las consecuencias deben ser idénticas por los dos caminos (`aplicarConquista`, `desalojarResidentes`, `registrarDerrota` son compartidos).

## 15. Divergencias

Ninguna abierta.
