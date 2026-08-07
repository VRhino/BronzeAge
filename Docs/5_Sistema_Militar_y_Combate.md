# 5. Sistema Militar y de Combate

## 5.1 Principio general: héroe-comandante liderando tropa (heredado de Iberia)
El jugador NO combate individualmente contra multitudes — asume el rol de HÉROE/COMANDANTE que lidera una tropa de N unidades NPC. Regla de oro: Tropa > Héroe. Puede llevar más de un regimiento a una batalla pero solo despliega uno a la vez, intercambiables en puntos tácticos de reabastecimiento dentro del combate.

## 5.2 Modalidades de batalla instanciadas (heredado de Iberia)
El combate ocurre en INSTANCIAS separadas del mapa global (aunque se desencadenen en él), límites simétricos fijos, 2 bandos (Atacante/Defensor), sin empates.

1. **Asedio de asentamientos**: atacante captura banderas/áreas vitales antes de que expire el tiempo; defensor gana resistiendo. Solo defienden miembros de la Facción soberana del nodo o Facciones aliadas/vasallas confirmadas. Mortalidad severa (permadeath). Jugadores en cola desde mundo abierto rellenan la instancia dinámicamente según bajas.
2. **Mundo abierto**: choque de patrullas/ejércitos. Bandera/campamento transitorio; quien la pierde se retira, deja loot, sufre debuff temporal "Herido".
3. **Defensa/intercepción de caravanas**: combate asimétrico móvil (ver Doc 3, sección 3.10).
4. **Entrenamiento/matchmaking** (POSPUESTO a fase posterior a Fase 0/1): 15v15 puro, sin permadeath, para probar tácticas.

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
Asedios FORMALES en ventanas limitadas (ej. 2 veces/semana, horario fijo, ~1h de duración). Ataques logísticos (mundo abierto, caravanas) libres 24/7.

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
- Fundición: fabricación de lingotes de cobre/estaño/bronce — auto-construcción, ya no manual (ver Doc 4.2).
- Curtiduría: tratamiento de cuero (livestock → cuero → cuero curtido → cuero de calidad).
- Armería: fabricación de armas y armaduras a partir de lingotes y cuero — insumo directo del reclutamiento de Barracón/Galería de tiro.
- Carpintería: recluta armas de asedio (ariete, torre de asedio) y habilita construir/mejorar Palacio, Armería, Barracón y Galería de tiro de nivel 2+.
- Barracón: reclutamiento de tropas cuerpo a cuerpo (ver roster 5.8). Construcción vía política del General (Doc 4.4).
- Galería de tiro: reclutamiento de tropas a distancia (ver roster 5.8). Construcción vía política del General (Doc 4.4).
- Gran Fundición: único edificio de tier élite, sin cambios respecto a la versión ya implementada (queda para iteraciones posteriores la integración con la nueva Fundición).
- PENDIENTE: Establos (ligados a carros de guerra) no tiene equivalente en el catálogo del rediseño — las unidades de carro de guerra del roster anterior (ver 5.8) quedan sin edificio de reclutamiento definido.

**Materiales limitantes (clave anti-"ejército meta universal"):**
- COBRE: relativamente abundante.
- ESTAÑO: raro, concentrado en pocas ubicaciones (base histórica real: la disrupción de rutas de estaño es una teoría real del colapso de la Edad de Bronce). El bronce de calidad — y por tanto las tropas de tier alto — depende del acceso a estaño.

## 5.8 Roster de tropas (rediseño Fase 0: reclutamiento por edificio + nivel interno, ver Doc 4.2.1)

El roster ya no se organiza por Tier abstracto (inspiración Total War Troy, foco Egeo/Grecia) — cada unidad se recluta en Barracón o Galería de tiro, según el NIVEL INTERNO del edificio (1-3, ver Doc 4.2.1), pagando el equipo correspondiente fabricado en Armería: AC = Arma de Cobre, AB = Arma de Bronce, ABC = Arma de Bronce de Calidad, AmB = Armadura Básica, AmI = Armadura Intermedia, AaBr = Armadura de Bronce.

**Barracón (cuerpo a cuerpo) — carril Pesants, combate real (Doc 4.1/5.5):**

| Nivel | Unidad | Costo |
|---|---|---|
| 1 | Lanceros con escudo de mimbre | 1 AC |
| 1 | Espadachines de espada corta de cobre | 1 AC + 1 AmB |
| 2 | Hacheros ligeros | 1 AB + 1 AmB |
| 2 | Espadachines con espadas y escudos de bronce | 2 AB + 1 AmI |
| 3 | Lanceros pesados micénicos (escudos grandes) | 2 AB + 2 AmI |
| 3 | Hacheros armados (armadura media) | 1 AB + 1 AmI |

**Galería de tiro (a distancia) — carril Pesants, combate real:**

| Nivel | Unidad | Costo |
|---|---|---|
| 1 | Honderos (escaramuzadores) | 1 AmB |
| 2 | Escaramuzadores con jabalina | 1 AB + 1 AmB |
| 2 | Arqueros | 1 AB + 1 AmI |
| 3 | Arqueros con arco compuesto | 3 AB + 2 AmI |

**Nobleza (progresión plana) — sin cambios respecto a la versión ya implementada**: se sigue reclutando exclusivamente vía Gran Fundición, con su costo actual (cobre+estaño+oro) y conversión instantánea a élite — el rediseño de Barracón/Galería de tiro no la afecta.

PENDIENTE:
- Mapeo exacto entre el nivel interno del edificio (1-3, desbloquea qué unidades se pueden reclutar) y el mecanismo de veteranía por combate real ya existente (Doc 5.5/ASCENSO_TROPA) — si conviven ambos ejes (nivel de edificio desbloquea la receta/unidad, veteranía sigue dando bonus de poder al squad) o si uno reemplaza al otro.
- Establos / unidades de carro de guerra (Carros escaramuzadores, Carros de guerra reforzados del roster anterior): sin edificio de reclutamiento definido en el rediseño — Carpintería solo cubre armas de asedio (ariete, torre de asedio), no carros. Queda sin resolver si se retiran de Fase 0 o necesitan su propio edificio.

## 5.9 Exilio como política de soberanía (heredado de Iberia, ver también Doc 2.8)
El Gobernador puede decretar exilio de jugadores enemigos de su territorio; coste de reubicación (pérdida parcial de materiales, desplazamiento físico para recuperarlos).

## 5.10 Fuera de alcance de Fase 0
Todo lo instanciado/visual (combate real en escena, formaciones renderizadas, modo entrenamiento, attack timer con UI) pertenece a Fase 1+. En Fase 0, el combate se resuelve como CÁLCULO/LOG DE TEXTO (quién gana, bajas resultantes), sin representación gráfica.

**Mina de cobre** (confirmada durante implementación de Fase 0): edificio extractor de cobre, mismo patrón de auto-construcción/colocación/agotamiento que Cantera (piedra) y Mina de Oro. Es el único extractor de cobre del juego — sin él, el reclutamiento militar es imposible (todo tipo de tropa requiere cobre para su equipo de bronce). Se descubrió su ausencia durante el Sprint 5 al intentar reclutar la primera tropa de una partida de prueba.

**Extractores finitos y su reemplazo automático** (ver también Doc 4.2): Cantera, Mina de Oro, Mina de Cobre y Lenera explotan nodos/zonas con cantidad finita. Cuando la fuente se agota, la auto-construcción encola un extractor de reemplazo automáticamente (buscando un nuevo nodo del mismo recurso), hasta un máximo ligado al nivel del asentamiento — evita que agotar el único yacimiento condene al asentamiento a un déficit irreversible.
