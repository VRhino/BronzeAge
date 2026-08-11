# Roadmap de Escalado

Desarrollo por fases, desde un prototipo de datos puros hasta la visión final del juego completo. Cada fase se apoya en la anterior; no se avanza de fase hasta validar los sistemas core de la fase actual.

## Eje 1: Fidelidad visual / representación

### Fase 0 — Prototipo de datos puro
Sin gráficos 3D, sin escenas, sin mapas visuales reales. Todo corre como simulación de datos: mapa 2D top-down con formas simples (círculos, polígonos, líneas) y paneles de texto/UI de debug. Objetivo: validar la simulación misma antes de invertir en arte o escenas. Ver `Fase_0_Definicion.md`.

#### Fase 0.1 — Evolución del generador de mundo
Sigue siendo datos puros y 2D. Añade elevación/ríos/biomas/colocación de recursos realista al generador de
mundo, construido como campos continuos + geometría vectorial (nunca tiles ni rejilla horneada) para que
sea la base de menor fricción hacia el mapa de campaña de Fase 1. Ver `Fase_0_1_Definicion.md`.

### Fase 1 — Mapa de campaña (MVP visual, Unity/C#)
- Mapa de mundo estilo Total War (vista de campaña)
- El jugador coloca su asentamiento libremente en el mapa
- Ese punto se representa con un modelo 3D en miniatura (como las ciudades en TW)
- El jugador se mueve por el mapa mundo como una miniatura/pj
- Puede interactuar con la miniatura del asentamiento y "entrar" en ella (probablemente cargando una escena separada del interior)
- Zonas de influencia dinámicas ya visibles directamente en el mapa de campaña

### Fase final — Integración total
Mundo y asentamiento como una sola entidad continua, sin transición ni carga entre "mapa mundo" y "dentro del asentamiento" (estilo WoW, donde Ventormenta no es una escena separada del resto del mapa).

## Eje 2: Presencia de mar / naval

### Fase inicial
Mundo 100% terrestre, sin mar, sin comercio marítimo, sin barcos. Más abordable para probar el resto de sistemas sin la complejidad naval encima.

### Fase completa
Mundo con mar, puertos, comercio marítimo, tecnología de barcos.

## Eje 3: (pendiente de definir más ejes)
Candidatos: número de jugadores soportados, complejidad del sistema de combate, profundidad de la diplomacia, fidelidad de la IA de NPCs viajantes/mercaderes.

## Eje 4: Ciclo de servidor y Maravilla (nuevo, a petición del usuario — inspirado en análisis comparativo con Travian)

El juego sigue siendo un SANDBOX DE GUERRA PERSISTENTE sin condición de victoria PARA EL JUGADOR (Doc 2.9 no cambia en ese sentido). Pero cada INSTANCIA DE SERVIDOR sí tiene un ciclo de vida acotado:

- **Duración del ciclo**: 12 meses por defecto (placeholder, sin calibrar).
- **Maravilla (cierre anticipado del ciclo, inspirado en Travian) — EL EDIFICIO EN SÍ, IMPLEMENTADO (a petición del usuario, esta pasada solo cubre esto)**: edificio único de coste extremo, construible SOLO en un asentamiento de NIVEL MÁXIMO (nivel 3, tope de Fase 0, ver Doc 4.5). Añadido al catálogo (`EDIFICIO_CATALOGO.maravilla`, constants.ts) y disponible vía el control manual de cola (Doc 4.2, Gobernador/Maestro de Obras) — mismo patrón que Gran Fundición/Palacio, sin auto-construcción. Coste PLACEHOLDER: 5000 madera + 5000 piedra + 500 oro + 300 cobre + 200 estaño + 200 livestock (todos los recursos EN BRUTO del catálogo actual — varias veces el coste de Palacio, el más caro hasta ahora), 200 ticks de construcción. Los materiales EXÓTICOS que pide el diseño original quedan FUERA de esta pasada — no existe todavía ningún recurso/extractor exótico en el juego; el coste actual usa solo recursos ya implementados. Verificado en el navegador: aparece en el selector con su Info (costo/tiempo/requisito), y se rechaza correctamente con "Requiere nivel de asentamiento 3" en un asentamiento nivel 1.
- **Ciclo de servidor de 12 meses + cierre + legado NPC — SIN IMPLEMENTAR (fuera de esta pasada)**: todo lo demás de este Eje (el timer de 12 meses, que completar la Maravilla resetee el servidor, y que la Facción ganadora persista como legado NPC en el mundo siguiente) sigue siendo puro diseño, sin ningún código — requiere infraestructura de servidor/multi-instancia que Fase 0 (prototipo de sesión única en el navegador) no tiene. Ver el resto de esta sección para el diseño completo, todavía vigente para cuando se aborde.
- **Legado de la Facción ganadora (sin precedente directo en Travian)**: la Facción que completó la Maravilla NO desaparece con el reset — permanece DENTRO del nuevo mundo, con los asentamientos y el nivel que tenía al cerrarse el ciclo, pero pasa a estar CONTROLADA POR NPCs: no se expande, no construye más, solo se MANTIENE, y ofrece OPCIONES DE COMERCIO a las Facciones de jugadores del nuevo ciclo.
- **Consecuencia**: con el tiempo existirán SERVIDORES DE VARIAS VUELTAS (con una o más Facciones-legado NPC de ciclos anteriores conviviendo con jugadores nuevos) junto a SERVIDORES COMPLETAMENTE NUEVOS (primera vuelta, sin ninguna Facción-legado todavía).

PENDIENTE (decisión de diseño confirmada; falta la mecánica fina, los números, y TODA la implementación del ciclo — solo el edificio está implementado):
- Catálogo de materiales exóticos nuevos (recurso + extractor); coste final de la Maravilla una vez existan.
- Que la Maravilla CAMBIE de ciclo a ciclo (cada vuelta con requisitos propios) — hoy hay un único catálogo fijo, sin concepto de "ciclo" todavía.
- Si el timer de 12 meses cierra el ciclo por sí solo cuando nadie completa la Maravilla a tiempo, o el ciclo se alarga indefinidamente hasta que alguien la complete.
- Qué pasa con el resto de Facciones/jugadores (no ganadores) al resetear: ¿se pierde todo, o hay algún tipo de legado/traslado también para ellos?
- Reglas de interacción entre jugadores nuevos y una Facción-legado NPC: ¿puede atacarse/conquistarse, o es intocable? ¿Sigue sujeta a Mantenimiento (Doc 4.5)?
- Si la posición de la Facción-legado en el nuevo mapa es la misma relativa a donde estaba, aleatoria, o elegida.
- Cómo se elige/genera la Maravilla de cada ciclo nuevo (catálogo fijo rotativo vs. generada).
- Toda la infraestructura de servidor/reset/multi-instancia en sí, que no existe en Fase 0.

## Notas de proceso
- No se pasa de fase sin haber validado los sistemas core de la fase anterior.
- Los distintos ejes pueden avanzar a ritmos distintos (ej: se puede estar en Fase 1 de representación visual y aún en fase inicial del eje naval).
