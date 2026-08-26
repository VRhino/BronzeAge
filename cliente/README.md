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

Lo que falta en el backend para que sea posible está desglosado y numerado en el roadmap
(`Docs/Arquitectura/3_Plan_Evolucion_Roadmap.md`, hitos **C8–C13**) y diagnosticado en detalle en
`Docs/Arquitectura/4_Plan_Evolucion_Tareas.md` § "Diagnóstico de aislamiento del cliente". **La Fase C no se
cierra hasta que un cliente sin una sola línea del motor pueda jugar una partida completa contra este
backend.**

## Defecto conocido: los comandos de juego responden 403

`apiCliente.ejecutarComando` manda a `/v1/admin/partidas/:gameId/comandos`, donde el actor entra como
`administrador_global` — rol que no figura en `rolesPermitidos` de **ninguno** de los 30 comandos de la matriz
(`session/comandos/autorizacion.ts`). Los ~30 botones de acción de la interfaz mueren en
`403 rol_insuficiente`. Crear/regenerar mundo, avanzar tick y leer estado sí funcionan: son operaciones de
administración de verdad.

No es una regresión que arreglar aquí: es la consecuencia correcta de la regla del doc 5 ("tener acceso
técnico no concede autoridad dentro del juego") aplicada a un cliente que nunca decidió qué superficie habla.
Ver hito **C8** del roadmap.

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
