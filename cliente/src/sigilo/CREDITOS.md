# Créditos de los emblemas del sigilo

Los emblemas (`emblemas.json`) son iconos de [game-icons.net](https://game-icons.net), con licencia
[Creative Commons BY 3.0](https://creativecommons.org/licenses/by/3.0/). La licencia obliga a citar a sus autores en los
créditos del juego:

- **Lorc** — https://lorcblog.blogspot.com
- **Delapouite** — https://delapouite.com
- **Caro Asercion**
- **Cathelineau**
- **Skoll**
- **Willdabeast**

Qué icono de quién va en cada emblema está en el propio `emblemas.json` (`autor`, `icono`). Los trazados salen tal cual del
repositorio https://github.com/game-icons/icons (`<autor>/<icono>.svg`, lienzo de 512×512, sin el cuadrado de fondo).

Para cambiar un icono: elige otro en game-icons.net, copia el `d` de sus `<path>` (salvo el fondo `M0 0h512v512H0z`) en la
entrada del emblema y actualiza `autor` e `icono`. Si el autor es nuevo, añádelo a esta lista. Si se añade un emblema, tiene que
existir antes en el catálogo del backend (`CATALOGO_SIGILO` en BronzeAgeFase0) y en `catalogoSigilos.json`.
