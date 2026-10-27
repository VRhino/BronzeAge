# Textos de las misiones

> **Implementado el 2026-10-27** (decisión del autor: los textos los tiene el motor, no en la foto sino en un catálogo aparte que se envía en los datos de juego). Las reglas de las misiones están en [`Misiones.md`](Misiones.md).

## 1. Qué es

El catálogo de textos que el cliente enseña de cada misión: su **título**, una frase de **qué hacer** y, para las barras de avance, el **nombre de cada contador**. Va en `GET /v1/datos`, en `documentos.Misiones_Textos.TEXTOS_MISIONES`.

- **No es regla.** No entra en la huella de las reglas (`ReglasEconomia::huella` solo lee `Misiones.md`) ni en la foto: cambiar un texto no rompe ninguna partida, solo cambia la versión de los datos de juego.
- **Está entero.** Cada misión del catálogo `MISIONES` tiene su título y su texto, y cada contador que usa alguna condición tiene su nombre (lo vigila `nucleo::misiones::pruebas::cada_mision_y_cada_contador_tiene_su_texto`).
- **Sorpresa.** Que el catálogo viaje entero no quiere decir que el cliente lo enseñe: una misión no se muestra hasta que se activa (decisión del autor del 2026-10-27).

## 2. Catálogo

`misiones`: por id, `titulo` y `texto`. `contadores`: por contador, el nombre que acompaña a la barra («3 / 5 trueques»).

<!-- parametros:TEXTOS_MISIONES -->
```json
{
  "misiones": {
    "H1": { "titulo": "Primeros pasos", "texto": "Sal al mundo desde tu plaza o tu campamento" },
    "H2": { "titulo": "Tropa de préstamo", "texto": "Pide tropa prestada en un campamento de mercenarios" },
    "H3": { "titulo": "Hasta el final del camino", "texto": "Marcha y llega a tu destino" },
    "H4": { "titulo": "Repostar", "texto": "Rellena los víveres de tu columna en una plaza, avanzada o campamento" },
    "H5": { "titulo": "Ciudadano", "texto": "Entra en una Facción" },
    "H6": { "titulo": "Tu primera escuadra", "texto": "Recluta una escuadra nueva" },
    "H7": { "titulo": "Un ejército de verdad", "texto": "Recluta 50 de infantería, 25 de tiro y 10 de caballería" },
    "H8": { "titulo": "Juntos", "texto": "Únete a un ejército o a una convocatoria" },
    "H9": { "titulo": "Bautismo de fuego", "texto": "Combate en una batalla" },
    "H10": { "titulo": "Limpieza de caminos", "texto": "Destruye un campamento de bandidos" },
    "H11": { "titulo": "Tesoro escondido", "texto": "Abre un alijo" },
    "H12": { "titulo": "Manos a la obra", "texto": "Entrega materiales a la obra de un pueblo o una avanzada" },
    "H13": { "titulo": "Forjar al héroe", "texto": "Reparte tus puntos de atributo" },
    "A1": { "titulo": "La aldea", "texto": "Funda una aldea agrícola" },
    "A2": { "titulo": "Abrir el mercado", "texto": "Ten un Mercado activo" },
    "A3": { "titulo": "Primer barracón", "texto": "Ten un Barracón activo" },
    "A4": { "titulo": "Crecer", "texto": "Sube la plaza a nivel 2" },
    "A5": { "titulo": "El primer pueblo", "texto": "Completa un pueblo" },
    "A6": { "titulo": "Oficios", "texto": "Ten algún artesano" },
    "A7": { "titulo": "Murallas", "texto": "Completa un recinto de la muralla" },
    "A8": { "titulo": "Avanzada", "texto": "Completa una avanzada" },
    "A9": { "titulo": "Ciudad", "texto": "Sube la plaza a nivel 3" },
    "A10": { "titulo": "Nobleza", "texto": "Ten algún noble" },
    "A11": { "titulo": "Resistir", "texto": "Resiste un asedio" },
    "F1": { "titulo": "Una capital", "texto": "Ten capital" },
    "F2": { "titulo": "Repartir cargos", "texto": "Nombra un Embajador o un cargo local" },
    "F3": { "titulo": "Expansión", "texto": "Funda 2 plazas" },
    "F4": { "titulo": "Un reino", "texto": "Funda 5 plazas" },
    "F5": { "titulo": "Progreso", "texto": "Adopta una tecnología" },
    "F6": { "titulo": "Gobernar", "texto": "Activa una política" },
    "F7": { "titulo": "Comercio", "texto": "Cumple un trueque con otra Facción" },
    "F8": { "titulo": "Rutas comerciales", "texto": "Cumple 5 trueques" },
    "F9": { "titulo": "Aliados", "texto": "Acepta una alianza" },
    "F10": { "titulo": "Paz en la región", "texto": "Destruid 5 campamentos de bandidos" },
    "F11": { "titulo": "Conquista", "texto": "Conquista una plaza" },
    "F12": { "titulo": "Cantares", "texto": "Empieza una épica" }
  },
  "contadores": {
    "salidas": "salidas al mundo",
    "prestamos": "préstamos",
    "llegadas": "llegadas",
    "repostajes": "repostajes",
    "ciudadania": "ciudadanía",
    "escuadrones": "escuadras nuevas",
    "infanteria": "infantería",
    "tiro": "tiro",
    "caballeria": "caballería",
    "uniones": "uniones",
    "batallas": "batallas",
    "bandidos": "campamentos de bandidos",
    "alijos": "alijos",
    "entregas": "entregas",
    "atributos": "atributos repartidos",
    "mercado": "Mercado",
    "barracon": "Barracón",
    "nivel": "nivel",
    "pueblos": "pueblos",
    "artesanos": "artesanos",
    "nobles": "nobles",
    "recintos": "recintos",
    "avanzadas": "avanzadas",
    "asedios": "asedios resistidos",
    "aldeas": "aldeas",
    "capital": "capital",
    "cargos": "cargos",
    "fundaciones": "plazas fundadas",
    "tecnologias": "tecnologías",
    "politicas": "políticas",
    "trueques": "trueques",
    "aliados": "alianzas",
    "conquistas": "conquistas",
    "epicas": "épicas"
  }
}
```
<!-- /parametros -->
