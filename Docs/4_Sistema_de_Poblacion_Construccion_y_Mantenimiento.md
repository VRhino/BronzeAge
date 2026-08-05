# 4. Sistema de Población, Construcción Automática y Mantenimiento

## 4.1 Población NPC: 3 clases
Los JUGADORES son una categoría SEPARADA de estas 3 clases.

| Clase | Rol | Aparición | Crecimiento |
|---|---|---|---|
| Pesants | Trabajan recursos genéricos (granjas, canteras, bosques); tropas básicas | Desde la fundación, sin condición | Rápido, fórmula propia |
| Artesanos | Operan edificios de producción especializada (Fundición/Broncista, Curtidor-Armero, Carpintería); tropas Tier 2 | Automática, al construirse el primer edificio que los requiere | Fórmula propia independiente (pendiente detalle numérico) |
| Nobleza | Tropas de élite (vía equipo, sin grindeo) | Cantidad MÍNIMA de ciudadanos (jugadores) en el asentamiento (único requisito) | Muy lento; acelerado por el Sacerdote (no requisito) |

Cada clase tiene FÓRMULA DE CRECIMIENTO INDEPENDIENTE (no comparten los mismos pesos). Fórmula base de referencia (Pesants): comida disponible + vivienda disponible + estabilidad + felicidad.

**Reclutamiento militar (doble carril):**
- Carril COMBATE REAL (Pesants + Artesanos): deben salir a combatir (PvP o PvE) para "veteranizar" y subir de tier.
- Carril PROGRESIÓN PLANA (Nobleza): conversión INSTANTÁNEA a unidades de élite si hay equipo disponible.
- Cola de PRIORIDAD cuando la demanda excede el pool disponible (criterio exacto pendiente).

## 4.2 Auto-construcción por necesidad
- El jugador NO elige ubicación ni tipo de edificio, EXCEPTO: edificio de fundación, y edificios estratégicos (murallas, torres, puerto) que sí se colocan manualmente.
- Algoritmo de colocación: reglas por tipo de edificio (ej. granja cerca de tierra fértil, herrería cerca de mina+camino), elige la mejor casilla disponible dentro de la zona de influencia.
- Crecimiento disparado por NECESIDAD REAL: más población → necesidad de comida → granja automática; excedente de recurso → mercado/almacén.
- Excepción permanente: el COMERCIO/CARAVANAS nunca se crea automáticamente, siempre es acción manual del jugador.
- Cadenas de producción (materia prima → producto) son lógica INTERNA invisible — el jugador solo ve materiales almacenados y necesidades activas (déficits).
- Cargo MAESTRO DE OBRAS gestiona las prioridades de auto-construcción, da bonus a tiempos de construcción.
- Patrón de crecimiento: las ciudades agregan edificios desde el CENTRO hacia afuera (concéntrico).
- LAYOUT DINÁMICO según política activa: ej. política DEFENSIVA prioriza edificios defensivos y layout fácil de defender (con coste real: pierde eficiencia económica, genera "pasillos" hacia puntos de captura). Elimina la repetición visual entre asentamientos.

## 4.3 Almacenamiento
Límites de almacenaje por recurso, ampliables construyendo más capacidad. El superávit que excede el límite dispara construcción automática de más almacenamiento.

## 4.4 Políticas (mecanismo de influencia del jugador)
- Interfaz: decisiones DISCRETAS tipo menú (no sliders continuos).
- SLOTS Y POOLS POR CARGO:
  - Gobernador: 2 slots (escala hasta 5 según nivel de Facción), pool COMPLETA (todas las políticas).
  - Tesorero: 2 slots, pool de economía/comercio/mercados/precios/comisiones/caravanas/acuerdos.
  - Maestro de Obras: 1 slot, pool de auto-construcción/edificios.
  - General: 1 slot, pool de defensa/reclutamiento/fabricación de armas y armaduras.
  - Sacerdote: 1 slot, pool de felicidad/cultura/población.
  - TOTAL con todos los cargos ocupados: 7 políticas simultáneas máximo — incentiva mecánicamente la colaboración multijugador.
- Reglas: duración determinada, renovable indefinidamente, NO cancelable antes de tiempo (hay que esperar a que termine para renovar o cambiar).
- PENDIENTE: catálogo concreto de políticas dentro de cada pool; si son excluyentes entre sí dentro de un slot.

## 4.5 Mantenimiento de asentamientos (sistema unificado, incluye ex-"Coste de Gobernanza")
- MEDIDOR 0-100 por asentamiento, empieza en 100.
- COSTE PERIÓDICO en recursos + oro, que escala por (a) NIVEL del asentamiento y (b) DISTANCIA al centro de poder de la Facción (más lejos = más caro; mecanismo anti-snowball).
- Escalado del coste por nivel (se van SUMANDO materiales, no reemplazando):
  - Niveles iniciales: madera + comida.
  - Niveles medios: sube cantidad + se añade piedra.
  - Niveles avanzados: sube cantidad de nuevo + se añade oro.
  - Niveles tardíos: todos los materiales simultáneos, escalando.
- Si NO se cumple algún pago, el medidor BAJA de 100 a 0 de forma PROPORCIONAL al déficit (degradación gradual, no corte binario).
- Al llegar a 0: el asentamiento se DESTRUYE y cae en RUINAS → se limpia la zona → queda disponible para otro jugador/grupo. Esta es la MISMA ruta mecánica que el caso de abandono total (sea el asentamiento literalmente abandonado o simplemente mal gestionado mientras sigue activo).
- PENDIENTE: cantidades exactas por nivel, velocidad exacta de degradación, si hay recuperación posible antes de llegar a 0.

## 4.6 Entrada tardía y mundo lleno
El mapa es deliberadamente difícil de saturar por completo. Cuando un servidor se llena lo suficiente, se abre uno nuevo. El deterioro por mal mantenimiento/abandono libera continuamente zonas para nuevos jugadores.
