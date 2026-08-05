# Roadmap de Escalado

Desarrollo por fases, desde un prototipo de datos puros hasta la visión final del juego completo. Cada fase se apoya en la anterior; no se avanza de fase hasta validar los sistemas core de la fase actual.

## Eje 1: Fidelidad visual / representación

### Fase 0 — Prototipo de datos puro
Sin gráficos 3D, sin escenas, sin mapas visuales reales. Todo corre como simulación de datos: mapa 2D top-down con formas simples (círculos, polígonos, líneas) y paneles de texto/UI de debug. Objetivo: validar la simulación misma antes de invertir en arte o escenas. Ver `Fase_0_Definicion.md`.

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

## Notas de proceso
- No se pasa de fase sin haber validado los sistemas core de la fase anterior.
- Los distintos ejes pueden avanzar a ritmos distintos (ej: se puede estar en Fase 1 de representación visual y aún en fase inicial del eje naval).
