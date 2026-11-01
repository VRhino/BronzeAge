# Recompensas de misión: fija por tipo y lista libre

> **Diseño acordado el 2026-11-01 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Las cantidades son placeholders por calibrar; no hay bloque `parametros` hasta implementarlo. Entrada: `Docs/Mecanicas a desarrollar.md` 66.

## 1. Qué es

Hoy cada catálogo de misiones tiene sus recompensas cerradas. Con este diseño, una misión da dos cosas: una **recompensa fija**, marcada por su tipo, y una **lista libre** a la que se añade lo que se quiera. La lista funciona igual en las misiones de un solo uso (`Misiones.md`) y en las recurrentes (`Misiones_Recurrentes.md`).

La necesidad salió de la mesa de trabajo de misiones del admin web (`BronceAgeAdminClient/misiones.html`): se puede editar el valor de cada recompensa, pero no dar otra cosa, porque el motor no lo admite.

Casi todo existe ya y se aprovecha:

| Recompensa | Quién la da hoy | Dónde | Qué le falta |
|---|---|---|---|
| Participación | Recurrentes de plaza | `sumar_participacion` | Darla también en las cadenas de plaza |
| XP de Facción | Recurrentes y cadenas de Facción | `dar_experiencia` | Nada |
| Oro | Recurrentes individuales y cadenas de héroe | Almacén personal, con su capacidad | Pedirlo desde la lista |
| Recursos | Solo cadenas de asentamiento | Almacén de la plaza; rechaza con `mision.almacen_lleno` | Pedirlos desde la lista y darlos también al héroe y a la capital |
| Tropas | Ninguna misión | `dar_escuadra`, solo orden de administración | Darlas como recompensa |
| Objetos | Ninguna misión | No hay catálogo ni inventario real | Entrada 67 de `Mecanicas a desarrollar.md` |

Las cadenas guardan hoy un `Recompensa` por dueño (`crates/nucleo/src/misiones.rs`) y las recurrentes tres campos fijos (`crates/nucleo/src/recurrentes.rs`).

## 2. Modelo

### 2.1 Recompensa fija

La marca el tipo de misión, es obligatoria y mayor que 0.

| Tipo de misión | Recompensa fija | Motivo |
|---|---|---|
| Plaza | Participación | Solo tiene sentido en una plaza |
| Facción | XP de Facción | Es lo que mueve el nivel de la Facción |
| Héroe | Ninguna | Cada misión da lo que se ponga en su lista (decisión del autor) |

Vale igual en recurrentes y en cadenas: las cadenas de plaza pasan a dar participación.

### 2.2 Lista libre

La clave `recompensas` es una lista de elementos con su `tipo`. Puede estar vacía, salvo en una misión de héroe, que no tiene fija.

| `tipo` | Campos | Dónde vale |
|---|---|---|
| `recurso` | `recurso`, `cantidad` | En todas |
| `oro` | `cantidad` | En todas |
| `xp` | `cantidad` | En todas salvo las de Facción, donde es la fija |
| `tropa` | `tropa`, `cantidad` | Solo en misiones personales: recurrentes individuales y cadenas de héroe (decisión del autor) |

Así queda una recurrente de plaza; una cadena lleva las mismas claves:

```json
{
  "id": "PS1", "duracion": "semana", "tipo": "individual", "limiteSemanal": 10, "condiciones": { "victorias": 5 },
  "participacion": 10,
  "recompensas": [
    { "tipo": "xp", "cantidad": 10 },
    { "tipo": "recurso", "recurso": "madera", "cantidad": 300 },
    { "tipo": "tropa", "tropa": "exploradores_caballo", "cantidad": 10 }
  ],
  "titulo": "Veterano", "texto": "…", "explicacion": "…"
}
```

### 2.3 Validación

Al cargar `MISIONES` y en `fijar_catalogo_misiones`, un catálogo se rechaza entero si:

- falta la recompensa fija de su tipo o no es mayor que 0;
- una misión de héroe no lleva ninguna recompensa;
- hay un `tipo` desconocido o una cantidad que no es mayor que 0;
- la lista repite la recompensa fija de su tipo;
- una misión colectiva lleva `tropa`;
- un `recurso` o una `tropa` no existen en su catálogo, o una tropa pide más soldados de los que lleva su escuadra.

## 3. Adónde va cada recompensa

El destino lo decide quién cobra: una misión personal paga al héroe; una colectiva paga a la plaza o a la capital de la Facción.

| Recompensa | Individual o cadena de héroe | Grupal o cadena de plaza | Grupal o cadena de Facción |
|---|---|---|---|
| Recurso u oro | Almacén personal del héroe | Almacén de la plaza | Almacén de la capital efectiva |
| Tropa | Escuadra nueva del héroe | No se puede dar | No se puede dar |
| XP | Facción del héroe | Facción de la plaza | Es su recompensa fija |
| Participación (fija en plaza) | En la plaza de la misión | Grupal: a partes iguales entre quienes aportaron. Cadena: al héroe que reclama | No aplica |

## 4. Reclamar

Reclamar es **todo o nada**: se comprueba todo antes de dar nada y, si una parte no cabe, se rechaza entera y la misión sigue por reclamar. Así nunca se pierde una recompensa.

- Almacén personal: los rechazos de hoy, `heroe.sin_residencia` y `almacen_personal.lleno`.
- Almacén de la plaza o de la capital: `mision.almacen_lleno`.
- Una Facción sin plazas no puede cobrar recursos ni oro.
- Un héroe sin Facción no suma XP, y eso no impide reclamar lo demás.

## 5. Las misiones que ya hay

La traducción no cambia lo que da cada misión, salvo la participación nueva de las cadenas de plaza.

| Catálogo | Hoy | Queda |
|---|---|---|
| Cadenas de héroe (H) | `oro` | `recompensas` con su `oro` |
| Cadenas de asentamiento (A) | `recursos` | Participación fija y `recompensas` con sus recursos |
| Cadenas de Facción (F) | `xp` | `xp` fija |
| Recurrentes de plaza | `participacion`, `xp` y `oro` | Participación fija; `xp` y `oro` en `recompensas` |
| Recurrentes de Facción | `xp` y `oro` | `xp` fija; `oro` en `recompensas` |

Las cadenas de plaza A1-A11 empiezan con **10** de participación cada una (acordado con el autor, para ajustar después en el editor).

## 6. Qué cambia en el motor

- `Recompensa` deja de ser un valor por dueño: una fija más una lista, y un solo reparto al reclamar para los dos catálogos.
- Los bloques `MISIONES` y `MISIONES_RECURRENTES` adoptan la forma nueva, y `fijar_catalogo_misiones` y `GET /v1/admin/misiones/catalogo` también.
- `dar_escuadra` se separa de la orden de administración para que la use el reparto.
- Contrato, solo con añadidos: `DefinicionMisionRecurrente` lleva `recompensas`, y `mision.reclamada` cuenta lo que se dio. El campo `oro` deja de usarse.
- Cambian la huella de las reglas y la foto: hace falta partida nueva.

## 7. Lo que hacen los clientes

- **Admin web:** la ficha de `misiones.html` enseña la recompensa fija bloqueada y una lista con «Añadir recompensa», igual en las dos pestañas; valida lo mismo que §2.3 y exporta los bloques con la forma nueva.
- **Cliente de jugador:** enseña la lista en el detalle de cada misión.
- **Servicio de bots:** tiene en cuenta la lista al elegir misión.

## 8. Por calibrar y por decidir al implementar

- Las cantidades de las misiones nuevas y la participación de A1-A11.
- Si una tropa de recompensa respeta el Liderazgo del héroe: `dar_escuadra` hoy no lo mira.
- Dónde aparece esa escuadra si el héroe está en una plaza: `dar_escuadra` la pone en su columna si va a bordo y, si no, en su campamento.
