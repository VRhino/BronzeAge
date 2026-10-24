# Calibración del sistema 4 y de los batches: decisiones del autor (2026-10-24)

Respuestas a las preguntas de `crates/batch/decisiones/{militar,poblacion,comercio}.json`. Pendiente de aplicar en código, canon y Mechanics.

## Batch militar
- **M1 Saqueo de pueblos:** se deja tal cual. Defender los pueblos es parte de la jugabilidad; cambiará cuando haya bots.
- **M2 Atalayas:** su guarnición interna se repone de forma gradual en 6 h, como los mineros del pueblo (hoy se repone entera a las 6 h). Defenderla es trabajo de los héroes. CAMBIO.
- **M3 Choque en campo abierto:** se mantiene la cohesión y el ×1,05 del defensor.
- **M4 Bandidos:** la reaparición pasa de 60 min a **6 h**. CAMBIO de cifra (`CAMPAMENTOS_BANDIDOS`).
- **M5 Protección al fundar:** un día de protección al fundar (`protegida_hasta = fundado_en + proteccionMinutos`), reutilizando el plazo de la conquista. CAMBIO.
- **M6 Botín personal:** el uso del oro de bandidos vuelve con la paridad del TypeScript: refundación (fondo, aportar, retirar, comprar la Caravana de Refundación), Caravana de Fundación, almacén personal (guardar y sacar) y caravanas a mano entran en el **sistema 5** (tanda 5G). Sin orden nueva de «depositar».
- **M8 Asedio con números:** ventana de convocatoria de 30 s para que se unan otras columnas cercanas, como con Unity. CAMBIO.
- **M7 Equipo de Barracón y Galería:** se deja para la fase 7 (bots reales).
- **M9 Reclutar gasta población:** decisión del jugador (como hoy).
- **M10 Carga en los carros de una caravana adjunta:** la carga sigue a los carros: aparcar deja en la caravana lo que ya no cabe en la columna; perderla pierde lo que iba en sus carros. Sustituye al rechazo `carro.no_cabe` de `AparcarCaravana`. CAMBIO.

## Batch de población
- **Murallas M1 Empalizada temprana:** la red recalculada con el muro conserva la puerta y su tramo de calle o camino a cada lado, sembrados como red conectada (no islas). `LAYOUT_VERSION` 5 y volver a medir. CAMBIO.
- **Murallas M2 Cifras de la empalizada:** se mantienen.
- **C1 Nivel 3 en el batch:** se deja para la fase 7.
- **P5 Ganado:** se deja.

## Batch de comercio
- **C7 Comisión que no cabe:** se deja (se pierde).
- **C3b → regla nueva de fondo: TODO lo almacenado va en ENTEROS.** Almacenes de plaza, pueblo y avanzada, carros, caravanas, almacén personal y existencias de los campamentos guardan solo unidades enteras: no existen decimales en los almacenes. La producción, la extracción, las recetas, el consumo, el mantenimiento y la recaudación por minuto acumulan un **resto oculto** determinista (como la población entera) y entran o salen unidades enteras. El comercio, las entregas, las comisiones, la acuñación, el botín, el saqueo, el reparto de carga entre carros y cualquier fracción **se redondean hacia abajo** (nunca se crea de más). Cambia la foto. CAMBIO GRANDE: tanda propia («economía entera») antes de las tandas de núcleo del sistema 5.
- **C1b Reputación concentrada:** esperar a la fase 7.
- **C2b Acarreo en la solvencia:** se da por implementada; se medirá con un ascenso al nivel 3.
