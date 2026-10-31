# BronzeAge: reglas de trabajo para Claude

## Commits y push

Todo `git commit` y `git push` lo hace un **subagente con el modelo Haiku 5.5** (`model: "haiku"` en la herramienta Agent), nunca el agente principal. Decidido por el autor el 2026-10-17.

- El agente principal deja los cambios listos en el árbol de trabajo y lanza al subagente con un prompt autónomo: qué ficheros añadir, el mensaje del commit, la rama destino y la orden de `git push -u origin <rama>`.
- El mensaje de commit termina con las líneas de atribución que pida el entorno.
- Si el push falla por red, el subagente reintenta hasta 4 veces con espera creciente (2 s, 4 s, 8 s, 16 s). Si falla por otra causa, informa y no fuerza nada.
- El subagente no usa `--force`, no reescribe historia y no sube a una rama distinta de la designada.
- Al terminar, el agente principal comprueba el resultado con `git status` y `git log`, y avisa al autor si algo no quedó subido.

## Versiones de los crates

Cada crate lleva su propia versión en su `Cargo.toml` y la sube cuando cambia (decidido por el autor el 2026-10-31; hasta entonces todos estaban en 0.1.0). Mientras sean 0.x: cambio de reglas, de contrato o de comportamiento → sube el segundo número (0.2.0 → 0.3.0); arreglo sin cambio de comportamiento → el tercero (0.2.0 → 0.2.1). Solo pruebas o comentarios no suben nada. Un cambio del `proto/` sube `contrato`. Se sube en el mismo commit que el cambio, una vez por tanda aunque haya varios commits. El servidor dice al arrancar su versión y la del núcleo.

## Fuente de verdad de las reglas

Si el canon (`Docs/Game/`) y `Docs/Mechanics/` se contradicen, gana `Mechanics` y se corrige el canon (ver `Docs/Mechanics/README.md`).
