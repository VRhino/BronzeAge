# Cliente de depuración / administración — Bronze Age Collapse

Interfaz de navegador que hablaba con el backend cuando ambos vivían en el mismo repositorio. Se separa aquí
para poder inicializar con ella un repositorio propio (Fase C: este repo pasa a ser **solo servidor**, y los
clientes —jugador y administración— viven fuera).

Una sola aplicación: `index.html` → `src/main.ts`, consola de partida y de administración (crear/regenerar
mundo, avanzar tick, inspeccionar estado). El laboratorio visual del motor que vivía aquí
(`laboratorio.html` → `src/lab/`) **se eliminó el 2026-08-26**: no era un cliente —ejecutaba el motor
directamente en el navegador, sin tocar la API— y por tanto no podía aislarse por red ni acompañar a esta
carpeta a otro repositorio. Está en el historial de git por si hiciera falta rescatarlo como herramienta de
desarrollo del backend.

Ver también [`cliente-jugador/`](../cliente-jugador/) (boilerplate, hito C11b): un proyecto hermano, nuevo,
que sí cumple el criterio de cierre de la Fase C — cero import del motor, habla solo por red.

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

El detalle de lo que había que resolver en el backend (hitos C7–C13, todos completos ahora — salvo C4
Slice 2, niebla de guerra, bloqueado en una decisión de balance que ningún doc de este repo fija) está en el
roadmap (`Docs/Arquitectura/3_Plan_Evolucion_Roadmap.md`) y diagnosticado en
detalle en `Docs/Arquitectura/4_Plan_Evolucion_Tareas.md` § "Diagnóstico de aislamiento del cliente". Ver
también [`cliente-jugador/`](../cliente-jugador/), un boilerplate que ya prueba que un cliente sin motor
puede hablar con este backend. **La
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

`vite.config.ts` proxya `/v1` entero hacia `:3000` para evitar CORS en desarrollo.

El sujeto se cambia con `VITE_USUARIO`; el que se use debe figurar en `ADMINISTRADORES` del servidor, o el
backend responderá 403 al crear la partida. Sin `ADMINISTRADORES` no hay ningún administrador y nadie puede
crear partidas — es el default deliberado del servidor.

Se identifica con el proveedor de desarrollo del backend (`Authorization: dev <sujeto>`) y guarda el
`sesionId` en memoria — se pierde al recargar y se vuelve a pedir solo. Es un apaño de desarrollo consciente:
sustituirlo por un login real es cambiar `iniciarSesion` en `src/app/apiCliente.ts`.

> Al servir este cliente desde otro origen (ya sin el proxy de Vite), el servidor necesita CORS: configúralo
> con `ORIGENES_PERMITIDOS='https://tu-origen'` al arrancarlo — vacío por defecto, ningún origen cruzado pasa.
> El contrato completo (rutas, esquemas, qué exige sesión) está publicado en `GET /v1/openapi.json`.
