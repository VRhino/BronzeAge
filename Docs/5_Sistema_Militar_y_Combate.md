# 5. Sistema Militar y de Combate

## 5.1 Principio general: héroe-comandante liderando tropa (heredado de Iberia)
El jugador NO combate individualmente contra multitudes — asume el rol de HÉROE/COMANDANTE que lidera una tropa de N unidades NPC. Regla de oro: Tropa > Héroe. Puede llevar más de un regimiento a una batalla pero solo despliega uno a la vez, intercambiables en puntos tácticos de reabastecimiento dentro del combate.

## 5.2 Modalidades de batalla instanciadas (heredado de Iberia)
El combate ocurre en INSTANCIAS separadas del mapa global (aunque se desencadenen en él), límites simétricos fijos, 2 bandos (Atacante/Defensor), sin empates.

1. **Asedio de asentamientos**: atacante captura banderas/áreas vitales antes de que expire el tiempo; defensor gana resistiendo. Solo defienden miembros de la Facción soberana del nodo o Facciones aliadas/vasallas confirmadas. Mortalidad severa (permadeath). Jugadores en cola desde mundo abierto rellenan la instancia dinámicamente según bajas.
2. **Mundo abierto**: choque de patrullas/ejércitos. Bandera/campamento transitorio; quien la pierde se retira, deja loot, sufre debuff temporal "Herido".
3. **Defensa/intercepción de caravanas**: combate asimétrico móvil (ver Doc 3, sección 3.10).
4. **Entrenamiento/matchmaking** (POSPUESTO a fase posterior a Fase 0/1): 15v15 puro, sin permadeath, para probar tácticas. CONFIRMADO: pospuesto de forma explícita, requiere factores no disponibles en Fase 0 (mismo motivo que Attack Timer, ver 5.6).

## 5.3 Formaciones y cohesión táctica (heredado de Iberia)
- Romper formación penaliza duro (ej. arqueros dispersos -30% precisión, escuderos aislados -20% defensa, lanceros sin formación pierden bono anti-carga). Flanquear/aislar formaciones enemigas es táctica válida.
- COHESIÓN ENTRE JUGADORES: varios jugadores anclando una línea juntos ganan Defensa Compartida, resistencia a rotura de moral, regeneración lenta de HP.
- Jugador novato: útil desde el día 1 con infantería básica de escudo barata ("carne de línea") mientras veteranos flanquean.

## 5.4 Ciclo de vida de unidades (heredado de Iberia)
- PERMADEATH individual (excepto modo entrenamiento): bajas son permanentes.
- El SQUAD (nombre, nivel veterano) persiste aunque el regimiento sea aniquilado — se puede rellenar con nuevos reclutas conservando el progreso.
- DESERCIÓN POR HAMBRE: tropas consumen raciones continuamente; sin suministro, la moral colapsa y desertan permanentemente (mismo efecto que perderlas en combate).

## 5.5 Doble carril de progresión (ver también Doc 4, sección 4.1)
- Carril COMBATE REAL (Pesants + Artesanos): tropas Tier bajo/medio, deben veteranizar combatiendo de verdad.
- Carril PROGRESIÓN PLANA (Nobleza): conversión instantánea a élite con equipo disponible.

## 5.6 Attack Timer (heredado de Iberia, pospuesto a fase posterior a Fase 0)
Asedios FORMALES en ventanas limitadas (ej. 2 veces/semana, horario fijo, ~1h de duración). Ataques logísticos (mundo abierto, caravanas) libres 24/7. CONFIRMADO: pospuesto a Fase 1+ de forma explícita — requiere factores no disponibles en Fase 0 (infraestructura de instanciado multijugador programado, sistema de colas/horarios), no implementable en el prototipo de datos puros.

## 5.7 Reclutamiento por infraestructura física (adaptado a la Edad de Bronce)
No hay árbol tecnológico abstracto — el tipo de unidad reclutable depende de la infraestructura física y NPCs residentes del asentamiento.

**Oficios/NPCs especialistas:**
- Broncista/Fundidor: funde cobre+estaño → bronce; fabrica armas.
- Curtidor-Armero: cuero y bronce laminado → armaduras, escudos, cascos.
- Carpintero: astas de lanza, ejes de carro de guerra, estructuras de escudo.
- Constructores: mantenimiento de murallas y defensas pasivas.
- Maestro de Armas: entrena y mejora estadísticas de tropas.
- Sacerdote (ya es cargo de jugador, no NPC nuevo): buff de área.

**Edificios** (catálogo completo con costos/recetas en Doc 4.2.1, rediseño Fase 0):
- Centro Urbano: reclutamiento de la defensa mínima (Milicia de lanceros, ver roster 5.8) — a petición del usuario, decisión post Sprint 6: la tropa de entrada NO depende de Barracón (que exige nivel de asentamiento + política del General antes de existir siquiera), sino del único edificio que nace `activo` con el asentamiento desde el tick de fundación, sin cola de construcción ni gate. Así todo asentamiento puede defenderse desde el minuto uno, así sea con la unidad más débil del roster.
- Fundición: fabricación de lingotes de cobre/estaño/bronce — auto-construcción, ya no manual (ver Doc 4.2).
- Curtiduría: tratamiento de cuero (livestock → cuero → cuero curtido → cuero de calidad).
- Armería: fabricación de armas y armaduras a partir de lingotes y cuero — insumo directo del reclutamiento de Barracón/Galería de tiro.
- Carpintería: recluta armas de asedio (ariete, torre de asedio) y habilita construir/mejorar Palacio, Armería, Barracón y Galería de tiro de nivel 2+.
- Barracón: reclutamiento de tropas cuerpo a cuerpo (ver roster 5.8). Construcción vía política del General (Doc 4.4).
- Galería de tiro: reclutamiento de tropas a distancia (ver roster 5.8). Construcción vía política del General (Doc 4.4).
- Mina de cobre: extractor de cobre (Doc 1.4/4.2.1) — único extractor de cobre del juego, insumo obligatorio de Fundición para todo equipo de bronce. Extractores finitos con reemplazo automático al agotarse (mecanismo completo en Doc 4.2, incluye número fijo por tipo — NO ligado al nivel de asentamiento).
- Gran Fundición: único edificio de tier élite, sin cambios respecto a la versión ya implementada (queda para iteraciones posteriores la integración con la nueva Fundición).
- PENDIENTE: Establos (ligados a carros de guerra) no tiene equivalente en el catálogo del rediseño — las unidades de carro de guerra del roster anterior (ver 5.8) quedan sin edificio de reclutamiento definido. RESUELTO: POSPUESTO a Fase 1 de forma EXPLÍCITA e INTENCIONAL (no es un vacío accidental) — los carros de guerra no se implementan en Fase 0.

**Materiales limitantes (clave anti-"ejército meta universal"):**
- COBRE: relativamente abundante.
- ESTAÑO: raro, concentrado en pocas ubicaciones (base histórica real: la disrupción de rutas de estaño es una teoría real del colapso de la Edad de Bronce). El bronce de calidad — y por tanto las tropas de tier alto — depende del acceso a estaño.

## 5.8 Roster de tropas (rediseño Fase 0: reclutamiento por edificio + nivel interno, ver Doc 4.2.1)

**IMPLEMENTADO** (ver `constants.ts` `TROPAS_RECLUTABLES`, `engine/tropas.ts` `reclutarTropa`). Terminología (Doc 0/Glosario, igual criterio que Iberia): una **tropa** es el tipo de escuadrón que se recluta de una vez (ej. "Lanceros con escudo de mimbre"); una **unidad** es cada soldado individual dentro de una tropa. A diferencia de una versión anterior de esta sección, el número de unidades **NO lo elige el jugador** (ver "Unidades por defecto" más abajo) — cada tropa reclutada añade siempre el mismo tamaño de escuadrón fijo.

El roster ya no se organiza por Tier abstracto (inspiración Total War Troy, foco Egeo/Grecia) — cada tropa se recluta en Centro Urbano, Barracón o Galería de tiro, según el NIVEL INTERNO del edificio (1-3, ver Doc 4.2.1; Centro Urbano no tiene niveles), pagando el equipo correspondiente fabricado en Armería (ver catálogo completo de recetas en Doc 4.2.1). "Costo" en las tablas de abajo es POR SOLDADO — el costo real de reclutar es ese valor × "Unidades". Cada tropa tiene además un `poderBase` (PLACEHOLDER, ver más abajo) usado en el cálculo de combate en vez del poderBase por tier del roster anterior.

**Centro Urbano (defensa mínima, sin edificio dedicado) — carril Pesants + Artesanos:**

| Tropa | Costo (por soldado) | poderBase | Unidades |
|---|---|---|---|
| Milicia de lanceros | 2 Madera (en bruto, sin pasar por Armería) | 2 | 25 |

**Barracón (cuerpo a cuerpo) — carril Pesants + Artesanos, combate real (Doc 4.1/5.5):**

| Nivel | Tropa | Costo (por soldado) | poderBase | Unidades |
|---|---|---|---|---|
| 1 | Lanceros con escudo de mimbre | 1 Arma de Madera | 3 | 20 |
| 1 | Espadachines de espada corta de cobre | 1 Arma de Cobre + 1 Armadura Básica | 4 | 20 |
| 2 | Hacheros ligeros | 1 Arma de Bronce + 1 Armadura Básica | 7 | 18 |
| 2 | Espadachines con espadas y escudos de bronce | 2 Arma de Bronce + 1 Armadura Intermedia | 9 | 18 |
| 3 | Lanceros pesados micénicos (escudos grandes) | 2 Arma de Bronce + 2 Armadura Intermedia | 14 | 15 |
| 3 | Hacheros armados (armadura media) | 1 Arma de Bronce + 1 Armadura Intermedia | 12 | 15 |

**Galería de tiro (a distancia) — carril Pesants + Artesanos, combate real:**

| Nivel | Tropa | Costo (por soldado) | poderBase | Unidades |
|---|---|---|---|---|
| 1 | Honderos (escaramuzadores) | 1 Arma de Madera | 5 | 25 |
| 2 | Escaramuzadores con jabalina | 1 Arma de Bronce + 1 Armadura Básica | 8 | 20 |
| 2 | Arqueros | 1 Arma de Bronce + 1 Armadura Intermedia | 9 | 25 |
| 3 | Arqueros con arco compuesto | 3 Arma de Bronce + 2 Armadura Intermedia | 15 | 20 |

`poderBase` es PLACEHOLDER: no estaba en el diseño original (solo equipo/nivel), interpolado a partir de la progresión ya existente del roster anterior (3 → 6 → 12 → 25 en 4 tiers) repartida en estas 10 tropas a lo largo de 3 niveles — pendiente de calibración por simulación.

**Unidades por defecto** (`unidadesPorDefecto`, a petición del usuario — corrige una contradicción real con la propia definición de "tropa" de arriba, que ya decía "se recluta de una vez" mientras el motor aceptaba una `cantidad` libre): cada tropa forma/amplía su escuadrón en bloques de tamaño FIJO al reclutarse, el jugador ya no elige cuántos soldados reclutar. `costoEquipo` sigue siendo por soldado, así que el costo real pagado de una vez es "Costo (por soldado)" × "Unidades" de la tabla — ej. Milicia de lanceros cuesta 2 Madera/soldado × 25 = 50 Madera por reclutamiento. Cifras PLACEHOLDER sin calibrar por simulación todavía. En la UI (pestaña Guerra), el segmento "Info:" bajo el selector de tropa muestra ambos desgloses (por soldado y total del escuadrón) antes de confirmar.

**Milicia de lanceros y Arma de Madera** (post Sprint 6, decisión real del usuario tras diagnóstico por simulación — ver `Correcciones_Durante_Desarrollo.md` #32): antes, las 3 tropas de nivel 1 exigían la cadena metalúrgica o del cuero COMPLETA (Mina de Cobre/Corral → Fundición/Curtiduría → Armería), y menos del 6% de los asentamientos nace con un nodo de cobre o livestock dentro de su zona inicial — la primera tropa tardaba una mediana de ~196 ticks y solo la conseguía el 5.7% de los asentamientos en un batch de 200 runs × 900 ticks. "Milicia de lanceros" se paga con madera en bruto (sin pasar por Armería) y es deliberadamente la más débil del roster (poderBase 2) — existe para que el bucle de juego arranque pronto, no para ganar batallas. "Lanceros con escudo de mimbre" y "Honderos" se recostearon de Arma de Cobre/Armadura Básica a Arma de Madera (corrige además una incoherencia temática: un escudo de mimbre pagado con tecnología de cobre, y una honda pagada con armadura de cuero). Con el cambio shipeado, el mismo diagnóstico sube a 44.3% de asentamientos con al menos una tropa, mediana tick 21.

**Milicia de lanceros pasa de Barracón a Centro Urbano** (corrección posterior, a petición del usuario — la defensa mínima seguía dependiendo de un edificio con su propio gate: nivel de asentamiento indirecto + política "Construir Barracón" del General + cola de construcción, ver `Correcciones_Durante_Desarrollo.md` #36): "Milicia de lanceros" ahora se recluta vía Centro Urbano, el único edificio que nace `activo` con el asentamiento desde el tick de fundación (Doc 1.3), sin cola ni política. El único requisito que queda es tener un General asignado (igual que cualquier reclutamiento, Doc 2.2) y los 25 soldados de población + 50 madera del escuadrón — verificado en el navegador: reclutable en el tick 11 (en cuanto la población alcanza 25 pesants desde los 20 iniciales), muy por delante de cuándo Barracón podría siquiera empezar a construirse.

**Nobleza (progresión plana) — YA NO recluta tropas** (decisión real del usuario, ver `Correcciones_Durante_Desarrollo.md` #30 — corrige el texto anterior de esta sección, que seguía describiendo el reclutamiento vía Gran Fundición como vigente): Nobleza sigue existiendo sin cambios como clase de población (crecimiento, requisito de Palacio, ciudadanos mínimos), pero se retiró por completo la posibilidad de convertirla en tropa. El único carril de reclutamiento militar en Fase 0 es el de equipo (Centro Urbano/Barracón/Galería de tiro), abierto a Pesants y Artesanos.

**Relación entre tropas ya reclutadas y el edificio que las produjo**: CONFIRMADO — NO existe ninguna relación posterior al reclutamiento. Una vez una tropa está reclutada y en el mundo, es independiente del edificio (Barracón/Galería de tiro) que la originó. Si el edificio sube de nivel después, los escuadrones ya existentes NO se ven afectados de ninguna forma — ni mejoran ni empeoran. "Mejorar" solo significa poder reclutar tropas nuevas de mayor nivel a partir de ese momento (ver párrafo de RESUELTO más abajo).

**Regla confirmada (a petición del usuario, cierra una ambigüedad real detectada auditando el código)**: una tropa reclutada JAMÁS cambia de identidad/tipo al ganar experiencia. "Milicia de lanceros" que sube de veteranía se queda siendo "Milicia de lanceros" con más poder — nunca pasa a ser "Hacheros" ni ningún otro `tropaId`. El pool de origen (Pesants/Artesanos) tampoco cambia. La veteranía da un bonus de poder continuo al MISMO escuadrón (`poderBase * (1 + veterania * bonusVeteraniaPorPunto)`, fórmula sin cambios, ver `poderEscuadron` en `engine/combate.ts`) — "mejorar" de tropa solo ocurre reclutando una tropa DISTINTA y mejor cuando Barracón/Galería de tiro suba de nivel interno; eso crea un escuadrón nuevo, no transforma el existente.

**Auditado contra el código real, código muerto RETIRADO**: el texto anterior de esta sección decía que el ascenso automático de tier por veteranía (`ascenderTierSiCorresponde`, `TROPA_CATALOGO`, `ASCENSO_TROPA`) "convivía" con el sistema de tropas de equipo, aplicándose a Artesanos/Nobleza. Eso no era cierto — Nobleza no recluta tropas en absoluto (ver arriba), y Artesanos recluta por el mismo carril `reclutarTropa` que Pesants, que SIEMPRE asigna `tropaId`. Como `ascenderTierSiCorresponde` se desactivaba explícitamente en cuanto `tropaId` estaba presente, ningún escuadrón real pasaba por esa rama — era código muerto. Se retiró por completo: `TROPA_CATALOGO`, `ASCENSO_TROPA`, `ascenderTierSiCorresponde` y el campo `Escuadron.tier` (siempre valía 1, nunca cambiaba) ya no existen en el código. `Escuadron.tropaId` pasó de opcional a OBLIGATORIO (único origen real de escuadrones). La UI (tabla de escuadrones en Asentamientos, panel militar en Guerra) mostraba "Tier 1" de forma engañosa para toda tropa sin excepción — ahora muestra el `nivelRequerido` real de la tropa reclutada (Doc 5.8, catálogo `TROPAS_RECLUTABLES`). Verificado en el navegador reclutando Milicia de lanceros: se muestra "Nivel 1" correctamente en ambas vistas, sin errores de consola.

PENDIENTE:
- Establos / unidades de carro de guerra (Carros escaramuzadores, Carros de guerra reforzados del roster anterior): sin edificio de reclutamiento definido en el rediseño — Carpintería solo cubre armas de asedio (ariete, torre de asedio), no carros. Queda sin resolver si se retiran de Fase 0 o necesitan su propio edificio.

## 5.9 Exilio como política de soberanía (heredado de Iberia, ver también Doc 2.8)
El Gobernador puede decretar exilio de jugadores enemigos de su territorio; coste de reubicación (pérdida parcial de materiales, desplazamiento físico para recuperarlos).

## 5.10 Fuera de alcance de Fase 0
Todo lo instanciado/visual (combate real en escena, formaciones renderizadas, modo entrenamiento, attack timer con UI) pertenece a Fase 1+. En Fase 0, el combate se resuelve como CÁLCULO/LOG DE TEXTO (quién gana, bajas resultantes), sin representación gráfica.
