# BA-006 — Progresión histórica, tecnología y ampliación del roster

**Destino:** `BronzeAgeFase0`  
**Estado:** LISTO_PARA_REVISION  
**Prioridad:** Media — diseño posterior a Fase 0 y previo al roster ampliado de Conquest  
**Fecha:** 2026-09-13  
**Autor:** Codex · **Implementación BronzeAge:** Claude  
**Origen:** revisión conjunta de CQ-003 y decisiones del autor del proyecto (2026-09-13).

## Decisión de producto

La cronología jugable cubre aproximadamente 1300–323 a. C.: desde el apogeo anterior al colapso de la Edad
del Bronce hasta Alejandro Magno. El avance militar debe terminar haciendo accesibles las reformas
macedónicas —falangitas con sarisa, hipaspistas, caballería de Compañeros y sus tropas de apoyo— sin convertir
todo el periodo anterior en contenido obsoleto.

El autor acepta estos principios:

1. Cinco capítulos históricos globales marcan la madurez del servidor.
2. El capítulo permite descubrir tecnologías, pero nunca entrega tropas automáticamente.
3. Cada tecnología se conoce/adopta por Facción y se materializa por asentamiento mediante edificios y
   recursos.
4. Las vías de adquisición siguen siendo desarrollo propio, comercio y Aedas; conquista/mercenarios pueden
   actuar como fuentes concretas dentro de esas vías, no como una economía tecnológica paralela.
5. Las escuadras existentes conservan para siempre su `tropaId`: no evolucionan convirtiéndose en otra tropa.
6. La élite no es simplemente una mejora numérica. Falange, hostigadores, caballería y tropas flexibles se
   necesitan mutuamente.

## Alcance y relación con CQ-003

CQ-003 conserva su alcance actual: las 11 tropas ya existentes en `TROPAS_RECLUTABLES`. Esta propuesta
define la evolución futura del catálogo y del sistema tecnológico. No pide todavía a Conquest crear todas
las nuevas `SquadData`; esa solicitud se emitirá después de que BronzeAge acepte y publique IDs y contratos.

No se fijan aquí cifras finales de coste, duración, poder o umbrales mundiales. Se fijan identidad, relaciones
de desbloqueo, autoridad y secuencia de implementación. Los números se calibran con simulación una vez exista
telemetría de una campaña completa.

## Modelo de acceso: cuatro puertas

Una tropa es reclutable solo cuando se cumplen conjuntamente:

```text
capítulo permitido por el servidor
  + tecnología adoptada por la Facción
  + edificio y nivel interno en el asentamiento
  + población, equipo, animales y recursos disponibles
  = reclutamiento permitido
```

El capítulo no es un `tier` del escuadrón. Una tropa temprana puede seguir siendo veterana, barata o ideal
para una función concreta al final de la partida.

## Capítulos históricos del servidor

| ID | Capítulo | Contenido que pasa a ser investigable |
|---|---|---|
| `reinos_palaciales` | I — Reinos palaciales | levas, cobre, arcos simples y organización palacial |
| `crisis_adaptacion` | II — Crisis y adaptación | bronce avanzado, hierro temprano, carros y equitación militar |
| `polis_imperios` | III — Polis e imperios | hoplitas, ejército permanente, caballería organizada y asedio |
| `profesionalizacion_imperial` | IV — Profesionalización | tropas asalariadas, guardias reales, logística y armas combinadas |
| `reforma_macedonica` | V — Reforma macedónica | sarisa, hipaspistas, Compañeros y mando combinado macedónico |

### Cómo avanza el capítulo global

`ProgresoTecnologicoServidor` observa cuatro ejes, sin que una sola Facción controle el reloj:

- **demografía:** población mundial, ciudades y nobleza;
- **economía:** volumen de comercio, rutas, metal y equipo producidos;
- **política:** Facciones, alianzas e instituciones;
- **guerra:** batallas, asedios, combatientes movilizados y reconstrucción tras derrotas.

El paso de capítulo exige un tiempo mínimo, superar umbrales en al menos tres ejes y cumplir uno de varios
hitos narrativos. Una derrota cuenta como experiencia histórica: nunca se exige que la Facción investigadora
sea la vencedora. Un servidor pacífico puede avanzar principalmente por demografía, comercio y política.

Los umbrales se evalúan con magnitudes agregadas y diversidad de participantes, no solo con el máximo de una
Facción. Esto evita que el líder acelere por sí solo todo el servidor o que un grupo pueda bloquearlo.

## Estados y adquisición de una tecnología

```ts
type OrigenTecnologia = 'desarrollo' | 'comercio' | 'aeda' | 'conquista';

interface TecnologiaFaccion {
  tecnologiaId: string;
  estado: 'conocida' | 'adoptada';
  origen: OrigenTecnologia;
  conocidaEn: Instante;
  adoptadaEn?: Instante;
}

interface ProgresoTecnologicoServidor {
  capituloActual: CapituloHistorico;
  progresoPorEje: Record<'demografia' | 'economia' | 'politica' | 'guerra', number>;
  hitosCumplidos: string[];
}
```

- **Conocida:** la Facción tiene acceso al diseño o doctrina.
- **Adoptada:** ha completado su coste/proceso institucional y puede aplicarla.
- **Disponible localmente:** el asentamiento tiene los edificios y recursos concretos para usarla.

El primer desarrollador recibe prestigio y queda en la crónica del servidor, además de una ventaja temporal
de adopción. No obtiene monopolio permanente. Comercio y Aedas difunden tecnologías ya existentes; al
avanzar dos capítulos, el conocimiento antiguo se abarata gradualmente para permitir recuperación y entrada
de jugadores nuevos.

## Catálogo tecnológico militar inicial

### Metalurgia y equipo

| ID | Capítulo | Requisitos principales | Habilita |
|---|---:|---|---|
| `metalurgia_cobre` | I | Fundición I | `armaCobre` |
| `aleacion_bronce` | I | cobre, estaño, Fundición II | `lingoteBronce`, `armaBronce` |
| `bronce_calidad_militar` | II | `aleacion_bronce`, Armería III | `armaBronceCalidad`, `armaduraBronce` |
| `forja_hierro_temprana` | II | Fundición II y mineral de hierro | `lingoteHierro`, armas sencillas de hierro |
| `forja_hierro_estandarizada` | III | `forja_hierro_temprana`, Fundición III | producción militar estable de `armaHierro` |

Se proponen tres recursos nuevos: `hierro`, `lingoteHierro` y `armaHierro`. No se propone
`armaduraHierro` genérica: bronce, cuero y protecciones textiles mantienen funciones propias, y el hierro no
debe borrar económicamente la cadena del estaño.

### Infantería

| ID | Capítulo | Requisitos | Desbloqueo principal |
|---|---:|---|---|
| `leva_comunal` | I | inicial | milicia de lanceros |
| `escudos_ligeros` | I | Carpintería/Armería I | lanceros con escudo de mimbre |
| `armamento_palacial` | I | `metalurgia_cobre` | espadachines de cobre |
| `panoplia_bronce` | II | `bronce_calidad_militar` | guardia de bronce y lanceros micénicos |
| `disciplina_formacion` | II | Barracón II | formaciones cerradas y mejoras de cohesión |
| `ciudadania_militar` | III | asentamiento III, institución política | hoplitas ciudadanos |
| `falange_hoplita` | III | `ciudadania_militar`, `disciplina_formacion` | hoplitas veteranos |
| `infanteria_profesional` | IV | ejército permanente, Palacio | profesionales, mercenarios y guardias |
| `falange_sarisa` | V | `reforma_macedonica` | falangitas/pezhetairoi |
| `cuerpo_hipaspistas` | V | `falange_sarisa`, Palacio | hipaspistas y futura agema |

### Proyectiles y hostigamiento

| ID | Capítulo | Requisitos | Desbloqueo principal |
|---|---:|---|---|
| `hostigamiento_tribal` | I | inicial | honderos y jabalineros |
| `arqueria_palacial` | I | Galería I | arqueros |
| `arco_compuesto` | II | Carpintería II, materiales adecuados | arqueros compuestos |
| `pantalla_escaramuzadores` | III | `disciplina_formacion` | peltastas |
| `arqueria_especializada` | III | Galería III | arqueros regionales especializados |
| `hostigadores_profesionales` | IV | ejército permanente | hostigadores de élite; base para Agrianos |

`arco_compuesto` deja de representar la culminación cronológica del roster: es una tecnología oriental
antigua. La tropa puede continuar siendo cara y poderosa, pero no ocupa por sí sola el escalón macedónico.

### Caballería y carros

Se añade `caballerizas` como edificio militar de tres niveles:

| ID | Capítulo | Requisitos | Desbloqueo principal |
|---|---:|---|---|
| `equitacion_militar` | II | Caballerizas I, acceso a caballos | exploradores montados |
| `carros_guerra` | II | `equitacion_militar`, Carpintería II | carros de guerra |
| `caballeria_organizada` | III | Caballerizas II | arqueros a caballo y lanceros |
| `carga_caballeria` | IV | `caballeria_organizada`, instrucción profesional | caballería pesada |
| `formacion_cuna` | V | reforma macedónica, Caballerizas III | prodromoi y carga coordinada |
| `companeros_reales` | V | `formacion_cuna`, Palacio | caballería de Compañeros |

Reclutar caballería consume un caballo por soldado además de población y equipo. `ANIMAL_CATALOGO` ya tiene
caballos, pero el coste de reclutamiento actual solo acepta recursos: la futura implementación debe modelar
la transferencia/consumo de animales explícitamente, sin convertir el caballo silenciosamente en oro.

### Ingeniería y asedio

| ID | Capítulo | Requisitos | Habilita |
|---|---:|---|---|
| `carpinteria_militar` | II | Carpintería II | escalas y defensas de campaña |
| `trabajos_asedio` | III | `carpinteria_militar` | asignar cuadrillas temporales a excavar, rellenar fosos y debilitar muros |
| `ariete` | III | `trabajos_asedio` | ariete como equipo/entidad táctica |
| `maestria_asedio` | IV | Carpintería II, Armería III | asalto coordinado y máquinas avanzadas |
| `maestros_obras_militares` | IV | `maestria_asedio`, Palacio | menor coste y preparación de obras de asedio |

No existe una tropa reclutable de "zapadores" ni un cuerpo moderno de ingenieros. Los relieves y textos
atestiguan trabajos de excavación, minado, escalas y arietes, pero esas funciones se modelan como una tarea
temporal realizada por trabajadores, artesanos y combatientes asignados al asedio. `trabajos_asedio` habilita
esa acción y `maestros_obras_militares` representa conocimiento institucional, no una escuadra.

Un ariete tampoco es un `Escuadron`: es equipo o entidad táctica preparado para una batalla. Los grupos que
lo operan o que asaltan con escalas se forman para el asedio a partir de la población y las tropas presentes;
no persisten después como un tipo militar separado.

## Reforma macedónica

`reforma_macedonica` es una tecnología compuesta de Facción, no una receta de arma. Requiere:

- `forja_hierro_estandarizada`;
- `disciplina_formacion`;
- `infanteria_profesional`;
- `caballeria_organizada`;
- `logistica_campana` (rama institucional futura);
- `cuerpo_oficiales` (rama institucional futura);
- Facción y asentamiento de nivel alto;
- Palacio, Barracón III, Armería III y Caballerizas III;
- experiencia en batallas grandes usando varias armas, sin exigir victoria.

Desbloquea el sistema combinado, no una superunidad aislada:

```text
hostigadores despejan y fijan oportunidades
  -> falangitas contienen el frente
  -> hipaspistas protegen los flancos y el terreno roto
  -> Compañeros ejecutan la ruptura
  -> prodromoi exploran y persiguen
```

Los falangitas son núcleo regular final: dominan frontalmente en terreno adecuado, pero sufren al girar, en
bosque, ciudad, pendiente y flancos. Los hipaspistas y Compañeros son las unidades de élite. Los hoplitas no
desaparecen: son anteriores, más autónomos y menos dependientes de una formación profunda.

## Roster por etapas

El objetivo orientativo es 24–30 definiciones finales, no todas necesarias en la primera integración:

- **I–II:** las 11 tropas actuales, guardia de bronce, carros y exploradores montados;
- **III:** hoplitas, peltastas, arqueros especializados, caballería organizada, tropas asirias y capacidad de
  formar cuadrillas temporales de asedio;
- **IV:** mercenarios, guardias reales, caballería pesada y maestría institucional de asedio;
- **V:** falangitas, hipaspistas, Agrianos, prodromoi y Compañeros.

Las culturas aportan disponibilidad, aspecto y doctrina inicial; las tecnologías pueden difundirse. Evitar
que todas las Facciones terminen con una composición idéntica será materia de balance y de costes locales,
no de prohibiciones absolutas sin justificar.

## Papel de los Aedas

Los Aedas narran hitos reales, ofrecen acceso y conducen adopciones por capítulos. Ejemplo de épica:

```text
Canto de las lanzas largas
1. conocer una formación disciplinada
2. fabricar una reserva de armas de asta
3. sostener un periodo de instrucción
4. combatir manteniendo formación
5. coordinar infantería, hostigadores y caballería
resultado: adopción de reforma_macedonica
```

Los eventos inspiradores no deben convertirse en una lista de acciones repetibles para farmear. Cada
capítulo usa hechos relevantes y límites por batalla/periodo. La función narrativa y el registro del primer
descubridor siguen el Doc 6 vigente.

## Invariantes

- El progreso del servidor es monotónico; una ciudad destruida no devuelve el mundo a otro capítulo.
- Conocer una tecnología no crea recursos, edificios ni escuadras.
- La adopción es por Facción; la capacidad de producir/reclutar se valida por asentamiento.
- Una escuadra nunca cambia de `tropaId` por experiencia ni por avance de capítulo.
- Las tropas antiguas siguen reclutables mientras sus edificios y recursos existan.
- Los catálogos publicados llevan versión y los tickets congelan esa versión.
- BronzeAge conserva autoridad sobre reclutamiento, cantidad y Liderazgo; Conquest sobre comportamiento
  táctico, formaciones, daño, movimiento y habilidades.
- Ningún `tropaId` nuevo entra en un `BattleTicket` hasta que Conquest publique una definición compatible.

## Secuencia de implementación propuesta

1. Aceptar o corregir este diseño; resolver solamente las decisiones estructurales, no balance fino.
2. Definir schemas de `Tecnologia`, `TecnologiaFaccion`, `ProgresoTecnologicoServidor` y catálogo versionado.
3. Implementar capítulos y conocimiento/adopción sin alterar todavía `TROPAS_RECLUTABLES`.
4. Integrar las tres vías existentes: desarrollo, comercio y Aedas.
5. Introducir hierro y sus tres recursos con pruebas económicas y simulación batch.
6. Añadir Caballerizas y coste explícito de caballos.
7. Publicar por lotes los nuevos `tropaId`, abriendo una propuesta CQ por lote antes de permitirlos en tickets.
8. Añadir asedio y máquinas como contrato separado de `Escuadron`.
9. Implementar el capítulo macedónico cuando Conquest soporte formaciones, caballería y armas combinadas.

## Criterios de aceptación del diseño

- Una tecnología distingue capítulo permitido, conocimiento, adopción y disponibilidad local.
- Dos Facciones del mismo servidor pueden tener tecnologías distintas sin estar en capítulos distintos.
- Un asentamiento sin edificio/recurso no recluta una tropa aunque su Facción conozca la tecnología.
- Un descubridor obtiene reconocimiento sin crear un monopolio permanente ni bloquear a jugadores nuevos.
- El servidor puede avanzar por una combinación pacífica o bélica y ninguna Facción controla por sí sola el
  reloj.
- Hierro no invalida el bronce y el arco compuesto no se trata como invención macedónica.
- Falangitas, hipaspistas y Compañeros tienen roles distintos y se necesitan tácticamente.
- Las escuadras anteriores no se transforman, eliminan ni degradan al cambiar de capítulo.

## Base histórica consultada

- Metropolitan Museum of Art, [*Warfare in Ancient Greece*](https://www.metmuseum.org/de/essays/warfare-in-ancient-greece)
  y [la Macedonia de Filipo](https://www.metmuseum.org/fr/perspectives/mediterranean-game-of-thrones).
- British Museum, [síntesis del ejército neoasirio](https://www.britishmuseum.org/blog/introducing-assyrians),
  y Metropolitan Museum of Art, [relieve de asedio](https://www.metmuseum.org/art/collection/search/322622).
- Encyclopaedia Iranica: [ejército aqueménida](https://www.iranicaonline.org/articles/army-i/),
  [Inmortales](https://www.iranicaonline.org/articles/immortals/),
  [carros](https://www.iranicaonline.org/articles/chariot-av/) y
  [Gaugamela](https://www.iranicaonline.org/articles/gaugamela/).
- Cambridge University Press, [guerra homérica](https://www.cambridge.org/core/books/world-of-homer/war/F2A1443EB89216D99B59CE261200B4E4)
  y [pueblos de Anatolia en la Edad del Hierro](https://doi.org/10.1017/CHOL9780521086912.019).

Estas fuentes sostienen las familias y la cronología general. Los nombres de tecnología, gates y efectos son
diseño de juego, no categorías históricas literales.

## Decisiones pendientes para BronzeAge

1. Forma exacta de los umbrales de capítulo y su versión de balance.
2. Si `conquista` queda como origen explícito o se modela como adquisición mediante especialista/archivo
   capturado dentro de comercio/Aedas.
3. Producción y distribución de mineral de hierro en worldgen.
4. Costes, huella y requisitos de `caballerizas`.
5. Primer lote de tropas posterior a CQ-003; se recomienda II (guardia de bronce, carro y jinete explorador)
   antes de saltar a hoplitas o Macedonia.
