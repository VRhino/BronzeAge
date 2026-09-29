# Tecnología por Eras I-III — plan de implementación

**Reglas:** `Consideraciones/BA-006_Revision_Tecnologia_Eras.md` (fuente hasta que el Paso 0 las baje al canon:
Doc 6 tecnología, Doc 5.7/5.8 roster, Doc 4.2.1 edificios). Este documento solo dice **cómo llegar**: modelo de
datos, archivos, pasos, pruebas y riesgos. Redactado el 2026-09-29.

**Alcance:** motor de tecnología (Eras, logros del servidor, hitos de Facción, aparición, adopción) y el contenido de
las Eras I, II y III: 25 tecnologías, 26 tropas (15 nuevas), edificios y recetas nuevas, P1/P2, §37 (población por
escalón y Palacio) y §41 (recinto de piedra). Solo la **vía del desarrollo** (logro + hito + pago).

**Fuera de alcance** (siguen en `Docs/Mecanicas a desarrollar.md`): Aedas y la venta de tecnología (§42), conquista
(D35) y comercio (D39) de tecnología, capital elegida (§13: se usa el proxy `encontrarCapital`), culturas (solo
aspecto, D18), mercenarios (§40), el efecto del equipo de asedio (las tecnologías `carpinteria_militar` y
`trabajos_asedio` existen y se adoptan, pero su equipo llega con la mecánica de asedio), Eras IV-V, D47.

## 1. Lo que hay hoy en el código

| Pieza | Dónde | Estado |
|---|---|---|
| Roster | `TROPAS_RECLUTABLES` (`src/constants.ts`) | 11 tropas, sin tecnología; `edificio` solo admite `centroUrbano`, `barracon`, `galeriaDeTiro` |
| Reclutar | `reclutarTropa` (`src/engine/tropas.ts`) | recibe `origen: 'pesants' \| 'artesanos'`; gate = edificio + nivel interno |
| Edificios | `EDIFICIO_CATALOGO` (`src/constants.ts`) | gates por `requisitoNivelAsentamiento`, `requiereEdificio`, `requiereEdificioNivel`; Barracón 3 y Armería 3 piden Palacio; Carpintería pide nivel 3; Palacio sin niveles |
| Recetas | `niveles[n].recetas` | sin gate de tecnología; ya existen `armaBronceCalidad` y `armaduraBronce` |
| Subida de nivel | `ASCENSO_ASENTAMIENTO`, `NIVEL_ASENTAMIENTO` | techo provisional en 3; el 4 pide recinto de nivel ≥ 1 |
| Tick | `avanzarSimulacion` (`src/engine/simulation.ts`) | cadena por asentamiento y luego global; emite `eventosDominio` |
| Estado | `GameSessionState` (`src/session/estado.ts`), snapshot `FORMATO_SNAPSHOT_VERSION = 18` | sin migraciones: un formato nuevo rechaza los snapshots viejos (D10: el playtest arranca de cero) |
| NPC | `src/session/npcGobernanza.ts` | recluta por `TROPAS_POR_PREFERENCIA_NPC` (escalón y poder), pasa el `origen` |
| Mundo | `src/worldgen`, `WORLDGEN_VERSION = 15` | nodos de cobre, estaño y oro; no hay hierro |
| Contrato | `src/contratos/v1/catalogoTropas.*` | catálogo de tropas para Conquest |

Tiempo: 1 tick = 1 minuto de mundo; las Eras van en semanas (Era I = 50.400 ticks).

## 2. Modelo de datos

```ts
// src/domain/types.ts
export type EraId = 'reinos_palaciales' | 'crisis_adaptacion' | 'polis_imperios';   // IV-V al cerrarse
export type TecnologiaId = /* las 25 de las Eras I-III, unión literal */;

export interface Faccion {
  // ...
  /** Tecnologías que le han aparecido (D55): visibles y adoptables. Incluye las adoptadas. */
  tecnologiasAparecidas: Record<TecnologiaId, Instante>;
  tecnologiasAdoptadas: Record<TecnologiaId, Instante>;
}

// Estado del servidor, nuevo campo de EstadoSimulacion y GameSessionState
export interface EstadoTecnologia {
  era: EraId;
  eraDesde: Instante;
  /** Contadores de los logros (D30): solo crecen. */
  contadores: Partial<Record<ContadorLogro, number>>;
  /** Logro cumplido → cuándo. Fijado para siempre (D31). Público, sin decir qué tecnología abre (D55). */
  logros: Partial<Record<TecnologiaId, Instante>>;
  /** Primera Facción que la desbloqueó (crónica; el retraso de los Aedas, §42, parte de aquí). */
  primeros: Partial<Record<TecnologiaId, { faccionId: string; en: Instante }>>;
}
```

**Catálogo** (`src/constants.ts`, junto al roster):

```ts
export const ERAS: Record<EraId, { orden: number; plazoSemanas: number }>;   // I: 5, II: 6, III: 7 (D44/D48)
export const TARIFA_ADOPCION: Record<EraId, Partial<Record<RecursoTipo, number>>>;
export const TECNOLOGIAS: Record<TecnologiaId, {
  era: EraId;
  deArranque?: true;                       // leva_comunal, hostigamiento_tribal
  logro?: { contador: ContadorLogro; umbral: number };   // umbral = X placeholder (Paso 12)
  hito: CondicionHito[];                   // todas a la vez
}>;

type CondicionHito =
  | { tipo: 'edificio'; edificio: EdificioTipo; nivelInterno?: number; donde?: 'capital' }   // "activo"
  | { tipo: 'tecnologia'; id: TecnologiaId }
  | { tipo: 'recursoEnCapital'; recurso: RecursoTipo }
  | { tipo: 'nivelAsentamiento'; nivel: number; donde?: 'capital'; conEdificio?: EdificioTipo }
  | { tipo: 'yacimientoEnTerritorio'; recurso: RecursoTipo };
```

Y en lo que ya existe, un campo `requiereTecnologia?: TecnologiaId` en: cada tropa, cada edificio (construcción),
cada nivel interno y cada receta. Una sola función `tieneTecnologia(faccion, id)` decide las cuatro puertas (D29):
la Era no se comprueba aparte, porque ninguna tecnología se adopta antes de su Era (D48).

## 3. Pasos

Cada paso deja los tests en verde y se puede fusionar por separado.

### Paso 0 — Canon
- [x] Doc 6 pasa a ser el documento de tecnología: Eras y calendario (D34, D44, D48), logro + hito (D30, D31),
  visibilidad (D55), adopción por el Rey en la capital (D32) con la tarifa por Era, catálogo de las Eras I-III con
  logros e hitos. 6.5 ("Fase 0 sin Aedas") y "Planos de X" se retiran; los Aedas remiten a §42.
- [x] Doc 5.7/5.8: roster de las Eras I-III con tecnología, edificio, equipo, escalón, población, unidades, poder y
  velocidad; clase montada (28, D8); caballo en oro (D5); Caballerizas (D28, D37).
- [x] Doc 4.2.1: P1/P2, Caballerizas, Mina de hierro, Fundición 3, Sala del Consejo, recetas nuevas; Doc 4.4: +1
  ranura del Gobernador con Sala del Consejo; Doc 4.5: el nivel 4 pide Sala del Consejo y sale el techo provisional.
- [x] Doc 1.4 (livestock = vacuno; hierro 1,5 veces más frecuente que el cobre), Doc 3.10 y 5.12.5 (solo la
  caballería alcanza las caravanas rápidas), Glosario (Era, Tecnología, Logro del servidor, Hito, Capital).
- [x] BA-006: las Eras I-III quedan marcadas como bajadas al canon; sus tablas se sustituyen por un puntero.

### Paso 1 — Modelo y catálogo, sin efecto
- [x] Tipos del §2, `ERAS`, `TARIFA_ADOPCION`, `TECNOLOGIAS` (Eras I-III, con hitos del apéndice A).
- [x] Toda Facción nace con `leva_comunal` y `hostigamiento_tribal` adoptadas (D10, D48); el servidor, en la Era I.
- [x] Hecho de otra forma: las tecnologías de cada Facción viven en `EstadoTecnologia.porFaccion` y no en `Faccion`,
  que viaja entera a todos los jugadores (D55). Los tipos de recurso y edificio nuevos (hierro, equipo de calidad,
  carro, Caballerizas, Mina de hierro, Sala del Consejo) entran ya aquí con su ficha de catálogo y su huella, pero sin
  uso: nadie los construye hasta los Pasos 5, 8 y 9.
- [x] `FORMATO_SNAPSHOT_VERSION` 18 → 19. Proyecciones y fixtures del contrato con los campos nuevos.
- [x] Test: el catálogo es coherente (toda tecnología citada en un hito existe; toda tropa, edificio, nivel y receta
  con `requiereTecnologia` apunta a una tecnología real de una Era implementada).

### Paso 2 — Contadores, logros, Eras y aparición
- [x] Contadores (apéndice B). Las fuentes que ya emiten evento (`tropas.reclutadas`, `combate.resuelto`) se
  cuentan desde el evento; producción y extracción se suman en el tick sin evento por unidad (serían miles).
  Las batallas de Unity (`session/resultadoBatalla.ts`) cuentan igual que las del motor.
- [x] `avanzarTecnologia(estado, contadoresDelTick, instante)` al final de `avanzarSimulacion`:
  1. suma contadores; fija los logros que cruzan su umbral (evento público `tecnologia.logro`, sin nombre de
     tecnología);
  2. avance de Era (D48): todos los logros de la Era o el plazo, lo que llegue antes; evento `era.comienza`;
  3. aparición (D55): para cada Facción y cada tecnología de una Era ya abierta con el logro cumplido y el hito
     cumplido, se añade a `tecnologiasAparecidas` (evento privado `tecnologia.aparece`); el primero se anota en
     `primeros`. Los hitos se evalúan solo para tecnologías con logro cumplido y aún no aparecidas.
- [x] Tests: logro fijado para siempre; la Era no avanza antes de completar logros ni después del plazo; una
  tecnología no aparece sin logro, sin hito o con su Era cerrada; aparece a una Facción y no a otra.
- [x] Hecho así: los contadores que salen de **eventos** se suman en `exito()` (`session/comandos/tipos.ts`), por donde
  pasa todo hecho de la partida (tick, comandos, NPC); el tick solo suma lo que no deja evento (extracción y talleres,
  que `avanzarConstruccion` devuelve como `extraido`/`fabricado`). El NPC cuenta aparte lo que resuelve fuera del tick
  (reclutamiento, ataques a bandidos, bueyes de sus caravanas) y las batallas de Unity se cuentan en
  `aplicarResultado`. El batch, que no pasa por `exito`, cuenta los eventos del tick a mano. `PayloadAsedio` gana
  `enCombate`, `murallaCompleta` y `conResidentes` (proxy: algún defensor combatió en persona), y
  `PayloadCombateResuelto`, `tropaIds`. `tecnologia.aparece` va atribuido a la capital: solo lo ve su Facción.

### Paso 3 — Adopción
- [x] Comando `adoptarTecnologia` (`src/session/comandos/`): solo el Rey (`faccion.reyId`), estando en la capital
  (`encontrarCapital`, proxy de §13); la tecnología tiene que estar aparecida y no adoptada; paga
  `TARIFA_ADOPCION[era]` del almacén de la capital (D32). Códigos de error propios.
- [x] Proyección del jugador: sus tecnologías aparecidas y adoptadas, los logros públicos y la Era; nunca las
  tecnologías ocultas (D55). Administración: todo.
- [x] Tests del comando: no Rey, fuera de la capital, no aparecida, ya adoptada, sin recursos.

### Paso 4 — Las cuatro puertas
- [x] `reclutarTropa`, `anadirEdificioManualmente`, `mejorarEdificioManualmente` y la selección de recetas de
  `avanzarConstruccion` reciben las tecnologías adoptadas de la Facción dueña y comprueban `requiereTecnologia`.
  La auto-construcción no encola lo que la Facción no tiene.
- [x] `requiereTecnologia` en el roster y las recetas actuales (apéndice C).
- [x] Tests: sin la tecnología se rechaza con un motivo claro; con ella, igual que hoy.
- [x] Hecho así: `adoptadas` es un parámetro obligatorio de `reclutarTropa`, `avanzarConstruccion`, `anadirEdificioManualmente`,
  `mejorarEdificioManualmente` y `estadoMejoraEdificio` (la Facción de la plaza). `produccionPorMinuto` lo recibe
  opcional (vista informativa). Los tests del motor que no prueban tecnología usan `TODAS_LAS_TECNOLOGIAS`, y
  `crearEstadoDeTest` adopta todo el catálogo.

### Paso 5 — Edificios de la Era I (P1, P2, D37)
- [x] Carpintería: se construye desde el nivel 2; Carpintería 2 fabrica la pieza `carroGuerra` (`carros_guerra`).
- [x] Barracón 3 y Galería 3: nivel 3 + Carpintería 2 (fuera el Palacio). Armería 2 y 3: + Fundición 2 (G3, D43).
  Fundición 3: nivel 4 (su contenido llega en el Paso 9).
- [x] **Caballerizas** (edificio nuevo, `cria_caballar`): coste y requisitos del Barracón, huella 3×2, niveles 1-3
  (el 3 = nivel 3 + Carpintería 2); cupo de guarnición derivado del catálogo (D37) en `cupoGuarnicion`.
- [x] Tipo de edificio de las tropas: `+ 'caballerizas'`.

### Paso 6 — Población por escalón y Palacio (§37)
- [x] `reclutarTropa` deja de recibir `origen`: escalones 1-2 → pesants, 3 → artesanos, 4-5 → nobleza (las
  Falangitas, escalón 4 con artesanos, llegan en la Era V). `poblacionDisponibleParaReclutar` admite nobleza.
- [x] Palacio con niveles internos (Doc 4.2.1, aprobado el 2026-09-29): 1 en asentamiento nivel 2, cupo 80, 600 madera +
  400 piedra + 100 oro, 3 días; 2 en nivel 3, cupo 240, 1.200 + 1.000 + 300, 5 días; 3 en nivel 4, cupo 400, 2.500 +
  2.500 + 800, 1 semana. El nivel 5 pide Palacio 3.
- [x] NPC y comando de reclutar sin `origen`. Borrar §37 de `Mecanicas a desarrollar`.
- [x] Hecho así: `poblacionDeTropa` (motor) decide la clase; el comando admite `origen` y lo ignora, para no romper a
  los clientes que aún lo mandan. El Palacio es un edificio con niveles (`capacidadNobles` y `obraMinutos` por
  nivel) y el requisito del nivel 5 usa `nivelInternoMinimo`. El NPC levanta el Palacio al llegar al nivel 3.

### Paso 7 — Roster de la Era I
- [x] Tropas nuevas: leñadores, granjeros (Centro Urbano, sin oro), exploradores a caballo (Caballerizas 1), carros
  de guerra (Caballerizas 2). Cifras en BA-006, "Cifras para implementar".
- [x] Cambios: milicia y lanceros de mimbre a escalón 2; hacheros armados a escalón 3 y Barracón 2; hacheros ligeros
  con `armamento_palacial`; escaramuzadores con arma de cobre + básica.
- [x] Caballería: velocidad 28; caballo a 5 de oro por animal al reclutar y al reponer (carros: 2). Comprobar que la
  persecución de caravanas va por velocidad y que solo la caballería alcanza a las de 24 (D8).
- [x] Hecho así: `caballos` por soldado y `ORO_POR_CABALLO` (5). El NPC no recluta la leva desorganizada de
  escalón 1 (leñadores, granjeros). El héroe solo (22) sigue por encima de toda tropa a pie, pero la caballería (28)
  va más rápida que él. Catálogo de tropas para Conquest, versión 2.

### Paso 8 — Era II
- [x] Hierro: recursos `hierro`, `lingoteHierro`, `armaHierro`; nodos de hierro 1,5 veces más frecuentes que los de
  cobre, mismo terreno (EII-4) → `WORLDGEN_VERSION` 15 → 16; Mina de hierro (`forja_hierro_temprana`); receta del
  lingote en Fundición 2 y del arma en Armería 2.
- [x] Armería 3: `armaBronceCalidad` y `armaduraBronce` con `bronce_calidad_militar`.
- [x] Tropas: jinetes asirios, Hequetai, Shardana, guerreros filisteos (Peleset); lanceros pesados y arco compuesto
  reciben su tecnología (`panoplia_bronce`, `arco_compuesto`).
- [x] Hecho así: el hierro se genera el ÚLTIMO del pipeline, así que el resto del mundo sale bit a bit igual que en v15
  para la misma seed (comprobado en el snapshot: solo se añaden líneas de hierro). Arreglado de paso: el almacén
  inicial sembraba una lista fija de recursos y los nuevos (carro de guerra, hierro…) nacían sin capacidad; ahora
  siembra `RECURSOS_TIPO`. Lanceros pesados con armadura de bronce (Doc 5.8). Catálogo para Conquest, versión 3.

### Paso 9 — Era III
- [x] **Sala del Consejo** (`instituciones_civicas`): una por plaza, desde el nivel 3; 800 madera + 1.200 piedra +
  300 oro, 2 días, 3×3; +1 ranura de política del Gobernador (`POLITICAS.slotsPorCargo`).
- [x] Nivel 4: + Sala del Consejo; `recintoCompletoNivelMinimo` 1 → 2 (§41). Fuera `nivelTechoProvisional`.
  Borrar §41.
- [x] Fundición 3 (`forja_hierro_estandarizada`): +50 % de lingotes. Armería 3: `armaHierroCalidad` (misma
  tecnología, pide Fundición 3) y `armaduraBronceCalidad` (`bronce_laminado`).
- [x] Tropas: honderos rodios, peltastas, jinetes escitas, hoplitas ciudadanos, caballería asiria, arqueros escitas,
  Espartiatas.
- [x] Hecho así: el techo provisional no se quita, se sube a 4 — el 4 → 5 sigue esperando a la tecnología de la Era V
  (D54, EV-2). La receta del Arma de Hierro de Calidad lleva `requiereEdificio: fundicion 3`. El NPC levanta la Sala
  del Consejo con `instituciones_civicas` y mejora su empalizada a piedra en el nivel 3. Catálogo para Conquest,
  versión 4.

### Paso 10 — NPC
- [x] Adopta en cuanto le aparece una tecnología, en orden de aparición, mientras la capital guarde el doble de la
  tarifa. **Adopta como un jugador: con su Rey en la capital** (decidido el 2026-09-29, R3). Hecho así: el Rey NPC
  no sale de su capital (se le trata como a quien tiene cargo en campañas, cacerías y repartos). En el batch, el Rey
  de cada Facción es su primer fundador, como en `crearFaccionNpc`. Adelantado tras el Paso 4 para que el batch no
  se quede sin tecnología.
- [x] Construye Caballerizas (Paso 5), Mina de hierro (auto-construcción con la tecnología), Palacio (Paso 6), Sala
  del Consejo y recinto de piedra (Paso 9); la subida a 4 la pide la regla general de ascenso.
- [ ] `TROPAS_POR_PREFERENCIA_NPC` filtra por tecnología adoptada y recluta con la clase que toca.
- [ ] Actualizar `Consideraciones/NPC_Gobernanza_Facciones_Controladas.md` §4.9.

### Paso 11 — Contrato y clientes
- [x] `contratos/v1`: catálogo de tropas con las nuevas (versión 4); `ProyeccionJugador.tecnologia` (Era, logros
  públicos, aparecidas, adoptadas); comando `adoptarTecnologia` en el OpenAPI (sale de `esquemas.ts`).
- [x] Modelo de datos compartido (doc 01): recursos, edificios, tabla de tropas y §21 Tecnología. Respuesta a BA-006
  con lo que necesita Conquest (15 escuadras nuevas, tipo montado, modelos de Caballerizas y Sala del Consejo): el
  ticket CQ lo abre Codex desde ahí.
- [ ] Cliente de administración (`cliente/`): panel de tecnología y Eras. **Aplazado**: el cliente y `lab/` no
  compilan desde antes de este plan (`capacidadCasas`, `bandidosProximoSpawnEn`); primero hay que ponerlos al día.
  BronzeAgeClient: su propio trabajo sobre `ProyeccionJugador.tecnologia` y el comando.

### Paso 12 — Calibrar los logros con batch (lo corre la sesión de balance)
- [ ] `scripts/batch`: medidor de contadores por semana y del día en que cada Facción adopta cada tecnología.
- [ ] Fijar cada `umbral` en lo que marque su contador en la semana objetivo (BA-006, tabla de logros).
- [ ] Comprobar D49 con las Eras I-III: niveles de asentamiento y tropas reclutadas por Era.

### Paso 13 — Limpieza
- [ ] Código muerto (techo provisional, `origen` de reclutamiento, comentarios que citan el Palacio en Barracón 3).
- [ ] `Mecanicas a desarrollar`: §20 se reduce a lo que falte (Eras IV-V, vías de conquista y comercio); §37 y §41
  fuera.

## 4. Riesgos

- **R1 — Checkpoints de batch.** El Paso 8 sube `WORLDGEN_VERSION` y todo el plan sube el formato del snapshot: los
  checkpoints de `simulaciones-batch/checkpoints` dejan de valer. Avisar a la sesión de balance antes de fusionar.
  Otra sesión trabaja en el relieve del mundo (`Docs/Arquitectura/11_Plan_Worldgen_Relieve_Estrategico.md`): los
  nodos de hierro deben entrar coordinados con ella.
- **R2 — Conflictos en `constants.ts` y `npcGobernanza.ts`**, que la sesión de balance edita a diario. Pasos
  pequeños y fusionados rápido.
- **R3 — El Rey en la capital para el NPC.** Decidido (2026-09-29): el NPC cumple la misma regla. Los reyes bot salen
  de campaña, así que el NPC tiene que volver con el Rey a casa para adoptar; medir con batch que no retrase las
  adopciones más allá de la semana objetivo de cada logro.
- **R4 — Ritmo.** Hoy los NPC reclutan todo lo que su edificio permite; con las puertas, la guerra de las primeras
  semanas cambia (medida de §11.8 de `Ritmo_Crecimiento_Asentamientos.md`). Medir con batch tras los Pasos 4-7.
- **R5 — Conquest** no tiene las tropas nuevas: una batalla de Unity con una de ellas necesita su definición
  (Paso 11) o un respaldo (resolver con números).

## Apéndice A — Hitos de las Eras I-III como condiciones

| Tecnología | Condiciones |
|---|---|
| `metalurgia_cobre` | edificio fundicion |
| `aleacion_bronce` | edificio fundicion nivel 2 · recursoEnCapital estano |
| `escudos_ligeros` | edificio barracon |
| `armamento_palacial` | edificio armeria · tecnologia metalurgia_cobre |
| `arqueria_palacial` | edificio galeriaDeTiro |
| `cria_caballar` | edificio corral |
| `carros_guerra` | tecnologia cria_caballar · edificio caballerizas nivel 2 · edificio carpinteria nivel 2 |
| `bronce_calidad_militar` | edificio armeria nivel 3 |
| `forja_hierro_temprana` | edificio fundicion nivel 2 · yacimientoEnTerritorio hierro |
| `panoplia_bronce` | tecnologia bronce_calidad_militar · edificio barracon nivel 3 |
| `disciplina_formacion` | edificio barracon nivel 2 |
| `arco_compuesto` | edificio carpinteria nivel 2 · edificio galeriaDeTiro nivel 3 |
| `equitacion_militar` | edificio caballerizas |
| `carpinteria_militar` | edificio carpinteria nivel 2 |
| `instituciones_civicas` | nivelAsentamiento 3 en la capital con mercado |
| `ciudadania_militar` | tecnologia instituciones_civicas · edificio barracon nivel 2 |
| `falange_hoplita` | tecnologia ciudadania_militar · edificio barracon nivel 3 |
| `pantalla_escaramuzadores` | edificio galeriaDeTiro nivel 2 · edificio armeria nivel 2 |
| `arqueria_especializada` | edificio galeriaDeTiro nivel 3 |
| `forja_hierro_estandarizada` | edificio fundicion nivel 2 · edificio minaHierro |
| `bronce_laminado` | edificio armeria nivel 3 |
| `caballeria_organizada` | edificio caballerizas nivel 2 |
| `trabajos_asedio` | edificio carpinteria nivel 2 |

"Edificio" = activo en cualquier plaza de la Facción, salvo que diga capital.

## Apéndice B — Contadores de los logros

| Contador | Fuente | Logros |
|---|---|---|
| `extraido.cobre`, `extraido.hierro` | extracción en `avanzarConstruccion` | `metalurgia_cobre`, `forja_hierro_estandarizada` |
| `fabricado.equipoBronce`, `fabricado.armaduraBronce` | producción de la Armería | `bronce_calidad_militar`, `bronce_laminado` |
| `caravanas.llegadasConEstano` | llegada de caravana con estaño en la carga | `aleacion_bronce` |
| `caravanas.destruidasOCapturadas` | combate de caravanas | `forja_hierro_temprana` |
| `bandidos.campamentosDestruidos` | `engine/bandidos.ts` / combate | `escudos_ligeros` |
| `animales.comprados` | mercado + compra de animales de caravana | `cria_caballar` |
| `batallas.libradas`, `batallas.campoAbierto` | `combate.resuelto` + Unity | `arqueria_palacial`, `carros_guerra` |
| `asedios.resistidosEnCombate`, `asedios.resistidosConResidentes`, `asedios.contraMurallaCompleta` | combate en plaza (sin rebotes por ocupación) | `disciplina_formacion`, `ciudadania_militar`, `trabajos_asedio` |
| `batallas.conHoplitas` | participantes de la batalla | `falange_hoplita` |
| `conquistas.conMurallaCompleta` | conquista | `carpinteria_militar` (umbral 1) |
| `reclutados.escuadrones`, `reclutados.<tropaId>` (soldados) | `tropas.reclutadas` | `panoplia_bronce`, `arco_compuesto`, `equitacion_militar`, `pantalla_escaramuzadores`, `arqueria_especializada`, `caballeria_organizada` |
| `plazasEnNivel.2`, `plazasEnNivel.3` | recuento al final del tick (máximo alcanzado) | `armamento_palacial`, `instituciones_civicas` |

## Apéndice C — `requiereTecnologia` sobre lo que ya existe

| Qué | Tecnología |
|---|---|
| Milicia de lanceros; leñadores; granjeros | `leva_comunal` |
| Honderos; escaramuzadores con jabalina | `hostigamiento_tribal` |
| Receta `armaCobre` | `metalurgia_cobre` |
| Recetas `lingoteEstano`, `lingoteBronce`, `armaBronce`; espadachines de bronce; hacheros armados | `aleacion_bronce` |
| Lanceros con escudo de mimbre | `escudos_ligeros` |
| Espadachines de cobre; hacheros ligeros | `armamento_palacial` |
| Arqueros | `arqueria_palacial` |
| Recetas `armaBronceCalidad`, `armaduraBronce` | `bronce_calidad_militar` |
| Lanceros pesados micénicos | `panoplia_bronce` |
| Arqueros con arco compuesto | `arco_compuesto` |

Ojo: `lingoteBronce` también lo pide la subida 2 → 3 (D54). Sin `aleacion_bronce` no hay nivel 3, que es lo que
se busca.
