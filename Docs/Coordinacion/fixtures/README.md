# Fixtures de reconstrucción de asentamiento

Ejemplos reales para que Codex/Unity pueda levantar cubos de colores sobre la huella de un asentamiento de
BronzeAge. Elegir según lo que se necesite verificar:

- **`lab-asentamiento-0-40-80-ba005/`** — **referencia geométrica vigente** (`layoutVersion` 2, BA-005:
  huellas partidas por dos y Centro Urbano 4×4). Mismo guion y misma procedencia que el fixture de laboratorio
  de abajo; incluye muralla cerrada, nivel 4 y 21 tipos. Los dos fixtures siguientes son de la geometría
  anterior: sus huellas miden el doble de lado y ya no coinciden con el motor.

- **`asentamiento-0-1108-1328/`** — cuerpo EXACTO de una respuesta HTTP real de
  `GET /v1/jugador/partidas/:gameId`, capturada contra una copia aislada de una partida real (sin murallas,
  nivel 2, 12 tipos de edificio). Úsalo para verificar el contrato de red completo: autenticación, filtro de
  visibilidad, forma exacta del wire.
- **`lab-asentamiento-0-40-80/`** — generado con el laboratorio de trazado (`lab/`, motor real sin servidor
  ni partida), moldeado a mano para incluir murallas completas y nivel 3 (21 tipos de edificio). Úsalo para
  la geometría (huellas, calles, caminos, murallas) — usa las mismas funciones del motor
  (`trazadoParaAsentamiento`) pero los campos de contexto (`gameId`, `jugadorId`, `version`) son sintéticos,
  no de una sesión real.

Ambos documentan units/coordenadas/rotación en su propio `README.md` — son las mismas convenciones en los
dos (mismo motor), el segundo README solo reconfirma con sus propios datos y explica la diferencia de
procedencia en detalle.
