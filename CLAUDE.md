# BronzeAge: reglas de trabajo para Claude

## Cómo se trabaja el código

Modo **ponytail** siempre encendido, en nivel full, en la nube y en el PC (decidido por el autor el 2026-11-27): en todo trabajo de código se siguen las reglas de `.claude/skills/ponytail/SKILL.md` aunque no se invoque. El nivel ultra no se usa. Antes de abrir un PR se pasa `/ponytail-review`. Mientras el autor no decida otra cosa, no se ponen comentarios `shortcut:` en el código.

- **Cambios quirúrgicos.** Cada línea cambiada sale de lo pedido. No se retoca código, comentarios ni formato de al lado. Lo muerto ajeno se señala, no se borra; lo que deja huérfano el cambio propio, sí.
- **Criterio de éxito comprobable.** Antes de empezar, cada tarea dice cómo se sabrá que está hecha: «arregla X» pasa a «una prueba que reproduce X, y que pase».
- **El cambio más pequeño no recorta las reglas de la casa.** La ronda final de docs y la subida de versión del crate forman parte del cambio.

## Versiones de los crates

Cada crate lleva su propia versión en su `Cargo.toml` y la sube cuando cambia (decidido por el autor el 2026-10-31; hasta entonces todos estaban en 0.1.0). Mientras sean 0.x: cambio de reglas, de contrato o de comportamiento → sube el segundo número (0.2.0 → 0.3.0); arreglo sin cambio de comportamiento → el tercero (0.2.0 → 0.2.1). Solo pruebas o comentarios no suben nada. Un cambio del `proto/` sube `contrato`. Se sube en el mismo commit que el cambio, una vez por tanda aunque haya varios commits. El servidor dice al arrancar su versión y la del núcleo.

## Fuente de verdad de las reglas

Si el canon (`Docs/Game/`) y `Docs/Mechanics/` se contradicen, gana `Mechanics` y se corrige el canon (ver `Docs/Mechanics/README.md`).

## La suite completa

Cuando haga falta pasar la suite completa del motor, se lanza con `node scripts/suite.mjs`, nunca con `cargo test --workspace` a secas (decidido por el autor el 2026-11-04). Compila con el perfil `suite` y lanza los binarios en paralelo: unos 4 min en vez de 22. La salida de cada binario queda en `target/suite-registros/` y el resumen termina en una línea `FIN:`. Para una prueba suelta o un crate, `cargo test -p <crate> ...` como siempre.

Desde el 2026-11-22 la suite no arranca sin el artefacto de mundo (`generador/salida/mundo/mundo-*.bin`): sin él las pruebas sobre el mundo real se omiten en silencio y cuentan como bien. En la nube se genera con `cd generador && npm ci && npm run generar && npx tsx scripts/generar-mundo.ts 42 salida/mundo` (unos 3 min). Con `cargo test -p` a secas siguen omitiéndose sin avisar.

Desde el 2026-11-26 ha de ser además el **mundo acordado** (`generador/mundo-acordado.json`): la suite y las pruebas del núcleo se niegan con otro, y `generar-mundo.ts` avisa si con la semilla acordada sale otro hash. El generador no usa `Math.pow` ni `**` (cambian entre versiones de Node; va `potencia()` de `src/potencia.ts`) y lo vigila `src/__tests__/determinismo.test.ts`. Si un cambio del generador cambia el mundo a propósito, el hash nuevo va a `mundo-acordado.json` en el mismo commit.
