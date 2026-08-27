# Índice de comandos del cliente jugador

Fuente: `src/session/comandos/registro.ts` y `src/session/comandos/esquemas.ts`.

El backend expone actualmente 32 comandos.

La interfaz actual aplica `crearFaccion` y `unirseAFaccion` desde la pestaña Facción. El resto de comandos
continúa disponible solo a través del wrapper `ejecutarComando` de `src/apiCliente.ts`.

## Checklist de implementación en la interfaz

- [ ] `fundarAsentamiento` — `faccionId`, `posicion`
- [ ] `lanzarCaravanaFundacion` — `origenAsentamientoId`, `destino`, `numJugadores`
- [ ] `desarmarCaravanaFundacion` — `caravanaId`
- [x] `crearFaccion` — `nombre`
- [x] `unirseAFaccion` — `faccionId`
- [ ] `dejarFaccion` — sin parámetros (`{}`)
- [ ] `alternarFaccionNpc` — `faccionId`, `activo`
- [ ] `asignarRey` — `faccionId`, `jugadorId`
- [ ] `asignarEmbajador` — `faccionId`, `jugadorId`
- [ ] `asignarCargoLocal` — `asentamientoId`, `cargo`, `jugadorId`
- [ ] `comprarCasa` — `asentamientoId`, `jugadorId`
- [ ] `activarPolitica` — `asentamientoId`, `cargo`, `politicaId`
- [ ] `anadirEdificioManualmente` — `asentamientoId`, `cargo`, `tipo`
- [ ] `quitarDeCola` — `asentamientoId`, `cargo`, `edificioId`
- [ ] `moverEnCola` — `asentamientoId`, `cargo`, `edificioId`, `direccion`
- [ ] `mejorarEdificioAhora` — `asentamientoId`, `cargo`, `edificioId`
- [ ] `alternarAutoConstruccion` — `asentamientoId`, `pausada`
- [ ] `calibrarReservaManual` — `asentamientoId`, `recurso`, `valor`
- [ ] `renombrarAsentamiento` — `asentamientoId`, `nombre`
- [ ] `proponerRelacion` — `tipo`, `faccionAId`, `faccionBId`; opcionales: `tributoRecurso`, `tributoCantidad`
- [ ] `romperRelacion` — `relacionId`, `iniciadorFaccionId`
- [ ] `rebelionVasallo` — `relacionId`
- [ ] `anexionar` — `faccionAId`, `faccionBId`
- [ ] `fusionar` — `faccionAId`, `faccionBId`, `nuevoNombre`, `nuevoReyId`
- [ ] `proponerTrueque` — `asentamientoAId`, `recursoA`, `cantidadA`, `asentamientoBId`, `recursoB`, `cantidadB`
- [ ] `colocarOrdenMercado` — `asentamientoId`, `tipo`, `recurso`, `cantidad`; opcional: `precio`
- [ ] `crearCaravana` — `asentamientoId`
- [ ] `reclutarTropa` — `asentamientoId`, `jugadorId`, `tropaId`, `origen`
- [ ] `iniciarAsedio` — `atacanteId`, `defensorId`, `escuadronIds`
- [ ] `combateCampoAbierto` — `asentamientoAId`, `escuadronIdsA`, `asentamientoBId`, `escuadronIdsB`
- [ ] `interceptarCaravana` — `atacanteId`, `escuadronIds`, `caravanaId`
- [ ] `atacarCampamentoBandidos` — `atacanteId`, `escuadronIds`, `campamentoId`

## Contrato HTTP común

```http
POST /v1/jugador/partidas/{gameId}/comandos
Authorization: sesion <sesionId>
Content-Type: application/json
```

```json
{
  "tipo": "nombreDelComando",
  "params": {},
  "idempotencyKey": "opcional-y-unica"
}
```

El backend valida la forma de `params` según `tipo`, comprueba autorización y reglas de dominio, y devuelve
una respuesta con `resultado`. En caso de éxito también devuelve una proyección actualizada del jugador.

Estados esperados: `400` body o tipo inválido, `401` sesión inválida, `403` no autorizado y `409` rechazo del
comando o fallo de persistencia.

## Criterio para marcar un comando

Marcar `[x]` únicamente cuando exista una interacción de la interfaz que construya sus `params`, invoque
`ejecutarComando(gameId, tipo, params)` y gestione su respuesta o error. Tener el endpoint o la función wrapper
definidos no cuenta como aplicado.
