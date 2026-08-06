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

**Edificios:**
- Fundición (edificio base) y Gran Fundición (único edificio de tier élite, exclusivo de asentamientos/Facciones de mayor nivel — sin nombre "Taller Real", descartado).
- Establos: ligados a CARROS DE GUERRA, no caballería montada (coherente con la época).
- Taller de Carpintería: armas de asedio + carros de guerra.

**Materiales limitantes (clave anti-"ejército meta universal"):**
- COBRE: relativamente abundante.
- ESTAÑO: raro, concentrado en pocas ubicaciones (base histórica real: la disrupción de rutas de estaño es una teoría real del colapso de la Edad de Bronce). El bronce de calidad — y por tanto las tropas de tier alto — depende del acceso a estaño.

## 5.8 Roster de tropas (inspirado en Total War Troy, foco Egeo/Grecia)

**Tier 1 — Pesants:**
- Honderos (escaramuza)
- Lanceros con escudo de mimbre/cuero
- Espadachines con espada corta de bronce

**Tier 2 — Pesants veteranizados o Artesanos:**
- Hacheros ligeros
- Escaramuzadores con jabalina
- Arqueros con arco compuesto

**Tier 3 — Requiere Fundición + veteranía o buen equipo:**
- Lanceros pesados con escudo grande (tipo "en 8"/torre, icónico micénico)
- Hacheros armados (armadura media)
- Carros escaramuzadores (jabalina)

**Tier 4/Élite — Nobleza (progresión plana):**
- Carros de guerra reforzados (lanza)
- Guerreros de élite con armadura de bronce laminado y casco de colmillos de jabalí (ref. armadura de Dendra, cascos micénicos)
- Arqueros nobles

## 5.9 Exilio como política de soberanía (heredado de Iberia, ver también Doc 2.8)
El Gobernador puede decretar exilio de jugadores enemigos de su territorio; coste de reubicación (pérdida parcial de materiales, desplazamiento físico para recuperarlos).

## 5.10 Fuera de alcance de Fase 0
Todo lo instanciado/visual (combate real en escena, formaciones renderizadas, modo entrenamiento, attack timer con UI) pertenece a Fase 1+. En Fase 0, el combate se resuelve como CÁLCULO/LOG DE TEXTO (quién gana, bajas resultantes), sin representación gráfica.

**Mina de cobre** (confirmada durante implementación de Fase 0): edificio extractor de cobre, mismo patrón de auto-construcción/colocación/agotamiento que Cantera (piedra) y Mina de Oro. Es el único extractor de cobre del juego — sin él, el reclutamiento militar es imposible (todo tipo de tropa requiere cobre para su equipo de bronce). Se descubrió su ausencia durante el Sprint 5 al intentar reclutar la primera tropa de una partida de prueba.

**Extractores finitos y su reemplazo automático** (ver también Doc 4.2): Cantera, Mina de Oro, Mina de Cobre y Lenera explotan nodos/zonas con cantidad finita. Cuando la fuente se agota, la auto-construcción encola un extractor de reemplazo automáticamente (buscando un nuevo nodo del mismo recurso), hasta un máximo ligado al nivel del asentamiento — evita que agotar el único yacimiento condene al asentamiento a un déficit irreversible.
