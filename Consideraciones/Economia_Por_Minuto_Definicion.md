# Economía por minuto con fines exactos (definición)

Decidido por el autor el 2026-10-22. Sustituye al plan de «economía continua» (EC0-EC6), descartado entero ese día con todos sus cambios.

## Decisión de fondo

La economía **no** se convierte en curvas continuas. Se queda el paso de un minuto por plaza (`Carga::Minuto`, decisión del 2026-10-18: no afecta a la experiencia del jugador y cuesta ~5 s de CPU por día con 1.000 plazas). Se corrige solo lo que el jugador nota. Hay que corregir el diseño que decía lo contrario: `Reconstruccion_Definicion.md` P3 («sin tick, tampoco para la economía») y la tabla de `Docs/Arquitectura/13_Mundo_Reactivo_Rust.md` §«Almacén y producción» / «Población».

## Modelo

| Qué | Cuándo | Estado |
|---|---|---|
| Gastos y entradas discretas: órdenes (pagar obra/mejora, reclutar, comprar), caravanas, entregas, botín, saqueo, acarreo | al instante de la orden o del evento | ya es así |
| Inicio de una obra encargada a mano, si hay cuadrilla libre | al instante de la orden | **nuevo** (hoy espera al siguiente paso) |
| Fin de obra, mejora, ascenso y caducidad de política | en su instante exacto, con cita propia en la cola | **nuevo** (hoy hasta 59,999 s tarde) |
| Cuadrilla liberada y siguiente obra de la cola | en el mismo instante del fin | **nuevo**; rompe paridad con el TypeScript a propósito (`ocupadas_al_empezar`) |
| Producción, extracción, recetas, consumo de comida, recaudación, mantenimiento, guarnición, pueblos | en el paso de minuto | sin cambios |
| Un cambio a mitad de minuto (edificio terminado, política, reclutamiento) | cuenta el minuto entero en el siguiente paso | sin cambios |
| Evaluación de necesidades de auto-construcción | en el paso de minuto | sin cambios |
| Población | en el paso de minuto y **siempre entera** | **nuevo** (hoy continua en milésimas, leída con suelo) |
| Contador del almacén en el cliente | escalones (valor real del servidor) + tendencia «+N/min» | decisión de presentación (4H) |

## Población entera

- Cada clase guarda habitantes enteros y un **resto oculto** de crecimiento (determinista, en milésimas). El crecimiento del minuto se suma al resto; entran personas enteras cuando el resto llega a 1. El ritmo de crecimiento es el mismo de hoy.
- Las muertes por hambre usan el mismo resto, **simétrico**: con 10 habitantes y el 5 % por minuto muere una persona cada dos minutos.
- Requisitos, mano de obra, recaudación y consumo leen la población entera.

## Hambre (D7 y D8, aprobadas por el autor el 2026-10-21; se conservan)

- **D7:** con nutrición 0 no crece ninguna clase; solo hay mortalidad.
- **D8:** al tocar nutrición 0 la plaza entra en **recuperación** y no vuelve a crecer hasta recuperar nutrición 100 (20 minutos con comida completa). Mortalidad solo mientras la nutrición esté en 0; durante la recuperación con nutrición positiva la población queda quieta. Si vuelve a 0, vuelve la mortalidad y sigue pendiente llegar a 100. Una plaza que no ha pasado hambre crece con nutrición intermedia como hoy. La marca de recuperación se guarda en el estado (foto y hash): no se deduce de la nutrición.

## Citas nuevas en la cola

- Una por obra en curso, mejora, ascenso y política activa, con la generación de la plaza (la ruina las descarta). Prioridad a fijar en `Reloj_Y_Eventos.md` al implementar; si coincide con el paso de minuto de la plaza, el fin va **antes** del paso, como hoy (`completar_obras` va primero), para que el edificio cuente ese minuto.
- Ninguna cita recurrente nueva: cada una se programa al empezar lo que espera y desaparece al cumplirse o cancelarse.

## Pendiente al implementar

- Foto nueva (población entera, restos, marca de recuperación, citas), rechazo de la anterior y huella.
- Tests ajustados a los instantes exactos; volver a medir `batch poblacion` y `batch comercio` (la cuadrilla al instante, la población entera y D7/D8 cambian el balance) cuando el autor pida medir.
- Actualizar `Economia_Construccion.md` (la nota «Límite de precisión: un minuto»), `Reloj_Y_Eventos.md`, el doc 13, la definición de la reconstrucción y la tabla de paridad.
