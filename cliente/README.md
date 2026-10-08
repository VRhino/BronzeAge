# Cliente de depuración / administración — Bronze Age Collapse

> **v0.2.0 · último cambio 2026-10-06** — pasada de usabilidad del cliente admin. Sigue siendo de solo lectura
> salvo las acciones de administración (Mundo, Bots, NPC de Facción) y sigue acoplado al motor por tipos/consultas.
>
> - **Subpestañas comunes** (`src/ui/subpestanas.ts`) en Bots, Facción, Jugadores, Asentamientos y Guerra.
> - **Sigilo dibujado** (`src/sigilo/`, copia del dibujo del cliente jugador + `catalogoSigilos.json` del backend):
>   antes solo se pintaba un cuadrado bicolor. Chip de Facción con su escudo; `htmlNombreConSigilo` para chips y cabeceras.
> - **Facción**: General, Asentamientos, Diplomacia, Tecnologías (por Era, estado, logro y hitos) y Aedas/títulos.
> - **Jugadores**: General, Almacén personal, Escuadrones, Historial; punto de conexión verde / amarillo / gris
>   (`Heroe.desconectaEn` / `Heroe.fuera`). El Rey, Embajador y Gran Rey se muestran por nombre, con el id de detalle.
> - **Bots**: Facciones por nombre y sigilo (los bots sin Facción, agrupados aparte; las pizarras vacías, ocultas) y
>   «Ver memoria» como panel HTML (plan, residencia, esperas con tiempo restante, solicitud, pizarra).
> - **Mundo**: código de invitación real del servidor (`GET/PUT /v1/admin/registro/codigo`), editable; ya no se lee de
>   `VITE_CODIGO_INVITACION`.
> - **Mundo**: lista de las partidas del servidor (se refresca sola cada 5 s); un clic conecta esta consola a ella.
> - **Mundo**: «Parar y borrar esta partida» (dos clics; `DELETE /v1/admin/partidas/:gameId`): para los bots que jueguen en ella y borra
>   su registro (si el panel Bots está conectado); el servidor borra guardado, diario, historial, auditoría, conexiones y las cuentas de
>   bot; las membresías humanas quedan revocadas. Solo se conservan los **respaldos**. La consola queda **sin partida**.
> - **Sin partida** (`src/arranque.ts` → `src/sinPartida.ts`): estado válido de la consola. Lista de partidas, crear una nueva,
>   código de invitación y respaldos. Sin elección guardada se arranca como siempre (la de `VITE_GAME_ID`, que se crea si no existe);
>   una partida elegida que ya no existe no se recrea, se avisa.
> - **Respaldos** (Mundo y «sin partida»): verlos por partida (también de las borradas), respaldar la conectada ahora, restaurar
>   (devuelve la partida y las membresías humanas que cerró su borrado, y conecta la consola) y borrar.
> - **Campamentos** (pestaña nueva): cada campamento de mercenarios con General (posición, reclutas y tope, edificios, quién está
>   dentro), Residentes, Mercado y fondo de refundación, y Préstamos de tropa y bandidos de su anillo.
> - **Facción › General**: lista de miembros (conexión, cargo, residencia y dónde están ahora).
> - **Comercio › Información › Caravanas en ruta**: ahora incluye las enganchadas a un ejército, el tipo, el nombre de origen/destino y la **escolta**
>   (columna del ejército, escuadras cedidas con su poder, o «⚠ sin escolta» con la defensa base).
> - **Guerra**: pestaña con Panorama (guerras, ejércitos en campo, batallas) y Reclutamiento; estados vacíos explicados.
>
> Anterior (v0.1.1, 2026-09-10): documentación reconciliada con el backend; la niebla de guerra ya está implementada.
>
> El cambio funcional previo (2026-09-08) cableó el bloque "economía del oro" (coste de oro por soldado en
> el catálogo de reclutamiento) y de la ocupación post-conquista (Doc 5.12.9): badge "ocupada" en la lista de
> asentamientos, banner de ocupación + minutos restantes + factores en el detalle, nota en la pestaña Militar,
> recaudación/mantenimiento anotados como reducido/congelado por ocupación, marca "(dañado)" en los edificios
> de la cola y su tooltip.

Interfaz de navegador que hablaba con el backend cuando ambos vivían en el mismo repositorio. Se separa aquí
para poder inicializar con ella un repositorio propio (Fase C: este repo pasa a ser **solo servidor**, y los
clientes —jugador y administración— viven fuera).

Una sola aplicación: `index.html` → `src/main.ts`, consola de partida y de administración (crear/regenerar
mundo, avanzar tick, inspeccionar estado). El laboratorio visual del motor que vivía aquí
(`laboratorio.html` → `src/lab/`) **se eliminó el 2026-08-26**: no era un cliente —ejecutaba el motor
directamente en el navegador, sin tocar la API— y por tanto no podía aislarse por red ni acompañar a esta
carpeta a otro repositorio. Está en el historial de git por si hiciera falta rescatarlo como herramienta de
desarrollo del backend.

Ver también el **cliente de jugador**, que desde el commit `2dfe9e7` vive en su propio repositorio
(`BronzeAgeClient`): sí cumple el criterio de cierre de la Fase C — cero import del motor, habla solo por red.

## Esta carpeta TODAVÍA NO puede moverse a su repositorio

No es cuestión de repuntar un alias. `src/app/gameStore.ts` (25 imports), `src/ui/canvas.ts` y `src/main.ts`
importan el **motor** a través de `@motor/*` —declarado en `vite.config.ts` (`resolve.alias`) y
`tsconfig.json` (`paths`), hoy apuntando a `../src`— porque **calculan en el navegador 26 consultas derivadas
que el servidor no expone**, y porque el terreno que el servidor manda es inservible sin el código de
`worldgen/`.

La decisión tomada (2026-08-26) es que el cliente **no debe depender del motor de ninguna forma**: solo puede
hablar con el backend por red. Eso convierte `@motor/*` en un defecto a eliminar, no en una costura a
repuntar — quedan descartadas las opciones que este README recomendaba antes (copiar el motor, submódulo,
paquete npm).

Los hitos de aislamiento del backend C7–C13 están completos y la niebla de guerra, que fue el antiguo
"Slice 2" de C4, también está implementada. El detalle histórico del aislamiento está en el roadmap
(`Docs/Arquitectura/3_Plan_Evolucion_Roadmap.md`) y diagnosticado en
detalle en `Docs/Arquitectura/4_Plan_Evolucion_Tareas.md` § "Diagnóstico de aislamiento del cliente". Ver
también el repositorio `BronzeAgeClient`, que ya prueba que un cliente sin motor puede hablar con este
backend. **La
Fase C no se cierra hasta que un cliente sin importar el código del motor de este repo pueda jugar una partida
completa contra este backend** — eso NO significa cero lógica de dominio en el cliente, ver
`Docs/Arquitectura/9_Reglas_vs_Simulacion.md` para el eje real (regla de entrada propia vs. simulación /
entrada privilegiada).

## El administrador observa, no interactúa como jugador (C8, resuelto 2026-08-26)

Esta interfaz **ya no expone** los 27 comandos de rol `jugador` (fundar, reclutar, comerciar, etc.) — se
retiraron de `main.ts`: el administrador podía verlos todos igual (observar es parte de su trabajo), pero no
ejecutarlos, porque tener acceso técnico no concede autoridad dentro del juego (doc 5). La única acción de
administrador que queda en la UI es alternar si una Facción la controla el NPC de gobernanza
(`alternarFaccionNpc`), y **sí funciona**: el bug de fondo (`rolEnPartida` cortocircuitaba a
`administrador_global` en vez de mirar la `Membresia` `administrador_partida` real que se concede a quien crea
la partida) se corrigió el mismo día. Crear/regenerar mundo, avanzar tick y leer estado también son
operaciones de administración de verdad y siempre funcionaron.

## Uso

Requiere el backend corriendo aparte, con este sujeto declarado como administrador:

```bash
ADMINISTRADORES='dev:jefa' npm run server
```

```bash
npm install
npm run dev
```

`vite.config.ts` proxya `/v1` entero hacia `:3000` para evitar CORS en desarrollo. `BACKEND_URL` lo repunta a
otra instancia (`BACKEND_URL=https://…onrender.com npm run dev`). La pestaña **Mundo** de la consola muestra a
qué partida está conectada: nombre (`VITE_GAME_ID`), Local/En la nube y backend (de `BACKEND_URL`), proveedor
(`PROVEEDOR_AUTH`, por defecto `dev`). Debajo, el código de invitación del registro se lee del servidor y se cambia sin reiniciar (solo en memoria; `CODIGO_REGISTRO` es el valor de arranque).

El sujeto se cambia con `VITE_USUARIO`; el que se use debe figurar en `ADMINISTRADORES` del servidor, o el
backend responderá 403 al crear la partida. Sin `ADMINISTRADORES` no hay ningún administrador y nadie puede
crear partidas — es el default deliberado del servidor.

Se identifica con el proveedor de desarrollo del backend (`Authorization: dev <sujeto>`) y guarda el
`sesionId` en memoria — se pierde al recargar y se vuelve a pedir solo. Es un apaño de desarrollo consciente:
sustituirlo por un login real es cambiar `iniciarSesion` en `src/app/apiCliente.ts`.

> Al servir este cliente desde otro origen (ya sin el proxy de Vite), el servidor necesita CORS: configúralo
> con `ORIGENES_PERMITIDOS='https://tu-origen'` al arrancarlo — vacío por defecto, ningún origen cruzado pasa.
> El contrato completo (rutas, esquemas, qué exige sesión) está publicado en `GET /v1/openapi.json`.
