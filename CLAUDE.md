# BronzeAge: reglas de trabajo para Claude

## Commits y push

Todo `git commit` y `git push` lo hace un **subagente con el modelo Haiku 5.5** (`model: "haiku"` en la herramienta Agent), nunca el agente principal. Decidido por el autor el 2026-10-17.

- El agente principal deja los cambios listos en el árbol de trabajo y lanza al subagente con un prompt autónomo: qué ficheros añadir, el mensaje del commit, la rama destino y la orden de `git push -u origin <rama>`.
- El mensaje de commit termina con las líneas de atribución que pida el entorno.
- Si el push falla por red, el subagente reintenta hasta 4 veces con espera creciente (2 s, 4 s, 8 s, 16 s). Si falla por otra causa, informa y no fuerza nada.
- El subagente no usa `--force`, no reescribe historia y no sube a una rama distinta de la designada.
- Al terminar, el agente principal comprueba el resultado con `git status` y `git log`, y avisa al autor si algo no quedó subido.

## Fuente de verdad de las reglas

Si el canon (`Docs/Game/`) y `Docs/Mechanics/` se contradicen, gana `Mechanics` y se corrige el canon (ver `Docs/Mechanics/README.md`).

## Después de la migración a Rust

El barrido de `Docs/Coordinacion/` y de los documentos de `Docs/Mechanics/` aún sin cruzar contra el canon se hace **cuando el motor en Rust esté terminado**, no antes (decidido el 2026-10-17). Hasta entonces el canon (`Docs/Game/`) ya está alineado con `Mechanics`.

## Skills del proyecto

`.claude/skills/session-start-hook/SKILL.md` es una copia corregida del skill de usuario del mismo nombre: la plantilla del hook es síncrona y los pasos de validación no llevan énfasis sin motivo. Está en el repo para que las sesiones nuevas la carguen, ya que `~/.claude` no se conserva entre sesiones en la nube.
