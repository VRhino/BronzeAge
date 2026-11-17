#!/bin/bash
# Arranque del contenedor (Render): el servidor y, si se piden, los bots a su lado.
#
# Los bots (BotsBrain, carpeta `bots/`) son otro proceso que entra al servidor como un jugador más, por 127.0.0.1. Solo arrancan con `BOTS_TOTAL` mayor que 0 y
# `CODIGO_REGISTRO_BOTS` puesto; sin eso, este guion hace lo mismo que lanzar el servidor a secas. Su registro (las cuentas que ya crearon) va DENTRO de la carpeta de
# datos, para que viaje con la partida en la copia remota. Las demás variables `BOTS_*` (BOTS_DIAS_LLEGADA, BOTS_SEMILLA, BOTS_HORARIO, BOTS_CAMPAMENTOS...) pasan tal cual.
# El canal de control de la consola de administración (`BOTS_TOKEN`) no se usa aquí: la plataforma solo abre un puerto.
set -u
PUERTO="${PORT:-7878}"
DATOS="${BRONZEAGE_DATOS:-/app/datos}"

/app/bronzeage-servidor &
MOTOR=$!
BOTS=""

parar() {
  # Primero los bots (Ctrl+C: se desconectan), después el servidor (SIGTERM: para, hace la última copia y sale).
  if [ -n "$BOTS" ]; then kill -INT "$BOTS" 2>/dev/null; wait "$BOTS" 2>/dev/null; BOTS=""; fi
  kill -TERM "$MOTOR" 2>/dev/null
}
trap parar TERM INT

if [ "${BOTS_TOTAL:-0}" -gt 0 ] 2>/dev/null && [ -n "${CODIGO_REGISTRO_BOTS:-}" ]; then
  # A que el servidor conteste: puede tardar si baja la partida de la copia remota o espera el relevo de otra instancia.
  while kill -0 "$MOTOR" 2>/dev/null && ! (exec 3<>"/dev/tcp/127.0.0.1/$PUERTO") 2>/dev/null; do sleep 2; done
  if kill -0 "$MOTOR" 2>/dev/null; then
    mkdir -p "$DATOS/bots"
    echo "arranque: $BOTS_TOTAL bots contra 127.0.0.1:$PUERTO"
    env -u BOTS_TOKEN BOTS_SERVIDOR="127.0.0.1:$PUERTO" BOTS_REGISTRO="$DATOS/bots/bots-registro.json" /app/bronzeage-bots &
    BOTS=$!
  fi
else
  echo "arranque: sin bots (faltan BOTS_TOTAL o CODIGO_REGISTRO_BOTS)"
fi

# `wait` vuelve también cuando llega la señal: se repite hasta que el servidor haya salido de verdad.
while kill -0 "$MOTOR" 2>/dev/null; do wait "$MOTOR"; SALIDA=$?; done
if [ -n "$BOTS" ]; then kill -INT "$BOTS" 2>/dev/null; wait "$BOTS" 2>/dev/null; fi
exit "${SALIDA:-0}"
