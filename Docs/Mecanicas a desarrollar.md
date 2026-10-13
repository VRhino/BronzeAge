# Mecánicas por desarrollar

La **lista única de lo pendiente**. El estado completo de todas las mecánicas del juego —hechas, descartadas y
estas— vive en `Consideraciones/Checklist_Mecanicas.md`, que para lo pendiente solo resume y apunta aquí.
Re-contrastado contra `src/` el 2026-09-09. Lo que solo necesita calibración con números (batch, mediciones) vive en
`Docs/Mecanicas a balancear.md`. Las ideas aparcadas sin plan van a `Docs/Ideas a diseñar.md`.

Cuando una entrada se cierra (diseño **y** implementación), se **borra entera** de este archivo: lo útil que
no esté ya en el canon (`Docs/Game/`) o en una ficha de `Consideraciones/` se mueve allí primero. Mientras una
mecánica se está diseñando, sus acuerdos provisionales pueden vivir aquí como notas.

## Índice

| # | Área | Mecánica | Código hoy |
|---|---|---|---|
| 8 | CARAVANAS | Revamp de caravanas — solo los trozos diferidos (§8.1) | ◐ núcleo hecho; §8.1: solo (b) hecha |
| 20 | TECNOLOGÍA | Tecnología por Eras: lo que falta tras las Eras I-III | ◐ Eras I-III hechas |
| 38 | MILITAR | Tope de héroes en asedio: volver a 15 cuando entren jugadores | ◐ 5 mientras se prueba con NPC |
| 44 | GUERRA | Guerra logística: caravanas A/B, pueblos extractores, avanzadas | ✘ en diseño |
| 45 | LOGÍSTICA | Repostar por zona, víveres del ejército, enganchar solo dentro y reclamar | ✘ canon escrito, código pendiente |
| 46 | BOTS | Comportamiento de bots emergente: lo que los bots tienen que aprender con cada cambio | ✘ lista abierta |

**Descartado** (decisión del usuario): los **landmarks** (antes §10, 2026-10-05), la **curva de progresión inicial gradual** (antes §29, 2026-10-06), el **bloqueo de la explotación del bosque por los campamentos de bandidos** y el **catálogo ampliado de políticas** (los dos, antes en los cabos sueltos §32/§35, 2026-10-06), y también, de los cabos sueltos §33/§35 (2026-10-06): la **riqueza acumulada** por comisiones, el **bonus por distancia en el mostrador**, **cortar rutas como guerra económica**, la **cola de prioridad de reclutamiento** y las **armas de asedio** como mecánica aparte.

**Pospuesto explícitamente, fuera de esta lista:** las **Eras IV y V** de tecnología con todo lo que cuelga de ellas (decisión del
usuario, 2026-10-05; ver §20), el **ciclo de servidor de 12 meses + Maravilla + legado NPC** (antiguo §24, 2026-10-06; su texto está en `Docs/Ideas a diseñar.md`), el **Attack Timer** (Doc 5.6, decidido y aplazado a
fase posterior a Fase 0) y el **comercio marítimo / unidades navales** (fuera del alcance de Fase 0 por
diseño). Los ajustes de calibración de mecánicas ya construidas viven en cada ficha de `Consideraciones/` y en
`Preguntas_Abiertas.md`, no aquí. Lo ya cerrado (diseño + implementación) se retira de este archivo por
completo; su estado queda en el checklist.

## 8. Revamp de caravanas — trozos diferidos

El **núcleo está implementado** (2026-09-08, Pasos 1-5): la caravana `comercial` es un contenedor de carros
con animal que deriva capacidad/velocidad, casco vacío + piezas, lanzamiento manual con estado `preparando`,
y escolta sin héroe. Reglas en `Docs/Game/3` §3.13; decisiones, motor y plan en
`typescript-final:Consideraciones/Revamp_Caravanas_Definicion.md`.

### 8.1 Lo diferido — forma diseñada, implementación en un pase posterior

El diseño de 2026-09-08 recortó seis piezas (la planificación horaria se hizo el 2026-10-02: `prepararCaravana.salirEn`, Doc 3.13.3; la visibilidad por tamaño, el 2026-10-05: Doc 5.12.7) del enunciado original para no inflar el primer pase. Ninguna se
descartó: se decidió su forma y se aparcó. Aquí queda cada una con lo que falta para abordarla.

**a) Cría de animales de arrastre.** Hoy los animales solo se compran con oro. El enunciado quiere obtenerlos
también por cría. El Corral (Doc 1.4/4.2.1) produce *livestock*, que es un recurso distinto — la cría de
bueyes/caballos/camellos necesitaría su propio edificio o una receta que consuma livestock + trigo y tarde
ticks. Falta: decidir si es un edificio nuevo o una función del Corral, el coste y el ritmo, y si cada tipo
de animal exige condiciones (el camello, un bioma; el caballo, quizá un nivel de asentamiento).

**c) Inmunidad del camello al desierto — POSPUESTO a fase posterior a Fase 0** (decisión del usuario,
2026-09-09). El camello "no se muere en los desiertos"; buey y caballo sí. Pero no existe un bioma `desierto`
de primera clase (el tipo es `agua|costa|estepa|llanuraFertil|colina|montana|cima`; la aridez del Nilo es
`estepa` de fertilidad baja). Necesitaría o un `BiomaTipo` nuevo, o anclar el "desierto" a `estepa` bajo un
umbral de fertilidad + una regla de *attrition* por tick sobre buey/caballo al cruzarlo. Es un cambio de
worldgen que no aporta a Fase 0; se retoma cuando el mapa tenga terreno árido real. Mientras tanto el camello
es una "opción media" a secas, aceptado.

**d) Catálogo ampliado de carros.** El primer pase trae solo dos carros: el básico (Mercado) y uno
"reforzado" (Carpintería) que solo da más capacidad. El enunciado habla de "varios tipos" fabricables en la
Carpintería. Falta: los ejes que diferencian un carro de otro más allá de la capacidad — resistencia a la
captura (un carro que sobrevive a una derrota), penalización de velocidad (un carro que no frena tanto al
animal rápido), coste en recursos más caros. Se abre cuando la Carpintería tenga niveles internos que lo
justifiquen.

**e) Unificación con el carro de columna.** `Ejercito.suministro` (Doc 5.13) y los carros de una caravana son
el mismo concepto físico: un vehículo con capacidad tirado para llevar carga por el mapa. Doc 5.13.3 ya dejó
anotado que se unifican "cuando se diseñe el revamp". El revamp los deja **separados a propósito** en este
pase —una caravana adjunta a un ejército sigue siendo su propia entidad— porque unificar el modelo físico es
un refactor sin premio de juego inmediato. Falta: un tipo `Carro` compartido y que tanto `Ejercito` como
`Caravana` lo compongan.


## 20. Tecnología por Eras: lo que falta tras las Eras I-III

**Estado: Eras I-III en el canon (Doc 6) y en el código (2026-09-29, vía del desarrollo), `código: ◐`.** Plan y
bitácora: `Consideraciones/Tecnologia_Eras_I-III_Definicion.md`. Falta:

- **Eras IV y V — POSPUESTAS** (decisión del usuario, 2026-10-05): el alcance se queda en las Eras I-III. Catálogo, roster y
  logros siguen en la revisión (`Consideraciones/BA-006_Revision_Tecnologia_Eras.md`, EV-1 a EV-7). La subida 4 → 5 la tiene
  que desbloquear una tecnología de la Era V (D54): hasta entonces el código mantiene el techo provisional en el nivel 4
  (`ASCENSO_ASENTAMIENTO.nivelTechoProvisional`), que es lo esperado mientras la Era V esté fuera.
- **Otras vías** (Doc 6.1): conquista (aparece al conquistar una plaza que reclutaba con ella) y comercio (pago a
  otra Facción; ¿la vendedora acepta y fija el precio?). La de los Aedas (venta de tecnología) ya está en el juego, Doc 6.7.
- **Logro de `logistica_campana` (Era IV) — POSPUESTO** con la Era IV: la condición ya existe como consulta pura,
  `caminoCompartidoAbierto` (`engine/redCaminos.ts`); se engancha al catálogo al escribir la Era IV.
- **Equipo de asedio**: `carpinteria_militar` y `trabajos_asedio` se adoptan, pero su equipo no tiene efecto en combate.
- **Clientes**: el panel de tecnología del cliente de administración está hecho (2026-10-02). Faltan, en BronzeAgeClient,
  el mismo panel, y en Conquest las definiciones de escuadra de las 15 tropas nuevas y los modelos de los edificios nuevos (CQ-006).

## 38. Tope de héroes en asedio: volver a 15 cuando entren jugadores

Mientras se prueba con NPC, el asedio admite 5 héroes por bando como el resto de batallas (decisión del usuario,
2026-09-27; `BATALLA.capacidad.asedio`, Doc 5.15.1). Cuando entren jugadores vuelve a **15 contra 15**: cambiar la
constante y el canon, y avisar a Conquest, que abre la instancia con esa capacidad (`BattleSide.capacidadMaxima`).

## 44. Guerra logística: caravanas A/B, pueblos extractores y avanzadas

En diseño desde el 2026-10-11, cero código y canon sin tocar (se reescribe al implementar). Todo vive en
`Consideraciones/Guerra_Logistica_Definicion.md`:
- **Cerrado**: se acaba la inmunidad de caravanas; batalla A (disputa de carros) fuera de la zona del dueño o de sus
  aliados y B (aguante, pierde el 70 %) dentro; atacar una caravana siempre exige guerra o la declara. Pueblos
  extractores con almacén propio, guarnición de mineros y carro de acarreo que sale lleno; saqueo por encima del 50 %.
  Obras de mapa: se marcan desde la plaza y se levantan llevando los materiales al sitio.
- **Cerrado también**: avanzadas (atalaya → fuerte → ciudadela), y la plaza pasa a ver solo su zona (lo de fuera lo
  ven las avanzadas, radio 60). Diseño completo; falta detallar el plan e implementar.
- La calibración de distancias y velocidades va en una sesión aparte.

## 45. Repostar por zona, víveres del ejército, enganchar solo dentro y reclamar

**Canon reescrito el 2026-10-11** (Doc 5.13, 5.13.2, 5.13.3, 5.14.1, 5.14.2, 5.14.5; Doc 1.8 y 1.9b; Doc 3.13.7).
El código todavía hace lo de antes; `Docs/Mechanics/Movimiento_Ejercitos.md` describe el código y se actualiza al
implementarlo.

**Qué cambia:**
- **Se acaba `LOGISTICA.radioReabastecimiento`** (60):
  - la columna personal reposta sola **dentro de la zona de influencia** de una plaza donde puede;
  - el ejército no reposta solo: usa el botón **Repostar** en la puerta (`radioPuerta`);
  - recoger tropas, cargar una caravana adjunta y entregar un trueque se hacen también **en la puerta**.
- **Víveres del ejército**: un montón propio con la capacidad sumada; al separarse o desarmarse, parte igual.
  `pasarAViveres` en un ejército llena ese montón.
- **Aliado incluye el vasallaje** al repostar (hoy `estanAliadas` solo cuenta alianzas).
- **Campamento de mercenarios**: Repostar gratis para todos, distinto de comprar trigo. La ración del residente cada
  30 min (`MERCENARIOS.racion`) queda absorbida.
- **Enganchar solo dentro** del lugar donde está la caravana (plaza o campamento), nunca en mundo abierto. **Reclamar**
  una caravana suelta de tu Facción, junto a ella, es una acción aparte.

**Impacto en el código (2026-10-11):**
- **Motor**:
  - `engine/viveres.ts` (montón del ejército, reparto igual);
  - `engine/ejercitos.ts`:
    - `repostarSiPuede`: solo columnas personales y por zona;
    - `puedeRepostarEn`: vasallaje;
    - los 4 usos del radio (recoger tropas, `adjuntarCaravana`, `cargarCaravanaAdjunta`, comer en el tick);
  - `engine/expansion.ts` (reclamar la de Fundación en ruta);
  - `engine/pertenencia.ts` (predicado de aliado con vasallaje);
  - `engine/mercadoMercenario.ts`.
- **Sesión**:
  - `session/comandos/ejercitos.ts` (`adjuntarCaravana`, `soltarCaravana`, `entregarDeCaravana`, comando nuevo
    `repostar`, comando nuevo `reclamarCaravana`);
  - `presencia.ts` y `ayudas.ts` (`conViveresLlenos`, salir de un campamento);
  - `autorizacion.ts`, `esquemas.ts`, `registro.ts`.
- **Tipos y contrato**: `domain/types.ts` (`Ejercito.viveres`); `src/contratos/v1/dto.ts` y su schema;
  `session/proyecciones/jugador.ts`.
- **Bots**:
  - `bots/cerebro/fundar.ts`, que engancha justo al salir y re-engancha en ruta;
  - `bots/cerebro/sinPlaza.ts`, que engancha tras `salirDelCampamento`;
  - `bots/cerebro/militar.ts`, que calcula el alcance con los víveres.
- **Clientes**:
  - BronzeAgeClient: `apiCliente.ts`, `main.ts`, `tiposDominio.ts`, `ui/caravanasAdjuntas.ts`, `ui/panelAliados.ts`,
    `ui/unirseDesdePlaza.ts`;
  - cliente admin: `cliente/src/app/gameStore.ts`, `cliente/src/main.ts`.
- **Tests**: unos 15 archivos. Los que más pesan son `avanzarEjercitos.test.ts`, `comandosEjercitos.test.ts`,
  `caravanaFundacion.test.ts`, `fundacionCampamento.test.ts`, `fundacionDesdePlaza.test.ts`, `guarnecer.test.ts`,
  `viveres.test.ts` y `entrarSalirCampamento.test.ts`.
- **Docs**: `Docs/Mechanics/Movimiento_Ejercitos.md` §6-§9, `Docs/Coordinacion/02` (comandos) y
  `Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md` (bots).

## 46. Comportamiento de bots emergente

**Lista abierta y acumulativa**: aquí se anota, según se diseñan las demás mecánicas, lo que los bots (`src/bots/`,
Doc Arquitectura 12) tendrán que hacer distinto o aprender. No se implementa con cada mecánica: se aborda en su
momento, con todo junto. Cada punto dice de qué mecánica sale.

**De §45 (repostar y caravanas):**
- `bots/cerebro/fundar.ts` y `sinPlaza.ts`: hoy salen y **luego** enganchan la Caravana de Fundación. Tienen que
  engancharla **antes de salir**, dentro de la plaza o del campamento, y salir ya como ejército.
- `sinPlaza.ts`: sale del campamento «admitiendo compañía» con un destino fijado; el ejército sale quieto y se
  dirige después con `marcharA`.
- Reclamar una caravana suelta (la de Fundación que vuelve sola) crea un ejército: decidir si el bot lo usa.
- `militar.ts`: el alcance de ida y vuelta se calcula con los víveres del héroe; pasa a ser los del ejército.
- Repostar: una columna personal lo hace sola dentro de la zona; un ejército de bots tiene que pulsar **Repostar** en
  la puerta antes de salir y al volver. En un campamento de mercenarios, repostar es gratis para todos.
- El vasallaje cuenta como aliado al repostar.

**De repostar gratis en el campamento (2026-10-11, `repostarEnCampamento`, `Docs/Mechanics/Comercio_Caravanas.md` §9.2b):**
- `bots/cerebro/sinPlaza.ts` se apoya solo en la ración gratis al salir (`racionAlSalir`: víveres llenos). Con `repostarEnCampamento` puede rellenarlos a la puerta del campamento, con la columna fuera, sin entrar y volver a salir: decidir si le compensa y cuándo (p. ej. al volver de un campamento de bandidos, antes de lanzarse a otro).
- `debeVolver` calcula el trigo del viaje de vuelta con los víveres de la columna: si el bot repostara a la puerta, la vuelta ya no es «hasta que se acabe el trigo».

**De §44 (guerra logística):**
- Marcar obras de mapa (pueblos, avanzadas) y llevar los materiales al sitio.
- Escoltar el carro de acarreo; saquear pueblos y atacar caravanas.
- **Aceptar o no el riesgo de declarar la guerra** al atacar (toda agresión la declara, salvo unirse al asedio de otro).
- Guarnecer avanzadas y repostar en ellas.
- Reclutar mineros en el Centro Urbano.
- La plaza ve solo su zona: los bots deciden con lo que ven, así que verán menos hasta tener avanzadas.
