# Plan de Implementación Técnica — Fase 0

## Stack
- **Lenguaje motor de simulación:** TypeScript (rápido de iterar; Fase 1 se hará en Unity/C#, por lo que el código de Fase 0 no se porta 1:1, pero sirve como prototipo de validación de reglas ya depurado, más fácil de re-implementar en C# después que diseñar desde cero).
- **Interfaz de debug:** aplicación web ligera (ej. Vite + TypeScript, sin framework pesado) con mapa 2D top-down (canvas o SVG simple) y paneles de texto/log.
- **Persistencia:** en memoria o archivo JSON local es suficiente para Fase 0 (no hace falta base de datos real todavía).

## Modelo de datos (entidades núcleo)

```typescript
interface Jugador { id: string; faccionId: string; nombre: string }

interface Faccion {
  id: string; nombre: string; reyId: string; embajadorId: string;
  nivel: number; scoreReputacion: number; capFundacionActual: number;
}

interface Asentamiento {
  id: string; faccionId: string; posicion: { x: number; y: number };
  nivel: number; medidorMantenimiento: number; // 0-100
  poblacion: { pesants: number; artesanos: number; nobleza: number };
  almacen: Record<string, number>; // recurso -> cantidad
  cargos: { gobernador: string; tesorero: string; general: string; maestroObras: string; sacerdote: string };
  politicasActivas: Politica[];
  edificios: Edificio[];
}

interface ZonaInfluencia { asentamientoId: string; poligono: Punto[] }
interface NodoRecurso { tipo: string; posicion: Punto; cantidad: number }
interface Caravana {
  origen: string; destino: string; contenido: Record<string, number>;
  posicionActual: Punto; tipo: 'comercial' | 'militar' | 'construccion' | 'contrabando';
}
interface AcuerdoTrueque {
  faccionA: string; faccionB: string; recursoA: string; recursoB: string;
  cantidadTotal: number; cantidadEntregada: number;
}
interface RelacionPolitica { faccionA: string; faccionB: string; tipo: 'vasallo' | 'aliado'; fechaInicio: Date }
interface Titulo { nombre: string; faccionActual: string; valorMetrica: number }
```

## Orden de construcción (por dependencias reales)

### Sprint 1 — Mundo y fundación
Generación de mapa (1000x1000 parametrizable, recursos dispersos con reglas de rareza y espaciado mínimo) → fundación libre → zona de influencia automática → colisión de fronteras (límite duro)

### Sprint 2 — Población y construcción automática
Las 3 clases (Pesants/Artesanos/Nobleza) con sus fórmulas de crecimiento independientes → auto-construcción por necesidad (reglas de colocación simplificadas a "mejor casilla disponible según tipo de edificio") → almacenamiento con límites

### Sprint 3 — Economía
Oro (origen: minas como nodo de recurso más) → trueque manual con caravanas físicas moviéndose por el mapa → órdenes de mercado con precios dinámicos (referencia por defecto + override manual) → comisiones de comercio → transporte individual espontáneo → categorías de caravana

### Sprint 4 — Estructura política
Cargos (asignación/elección, slots y pools de políticas) → Facción → Ciudadanía → Políticas (layout dinámico como flag simple, no visual real aún) → Vasallaje/Alianza → Formación de Liga → Cap de fundación escalando con nivel de Facción → Fusión/anexión

### Sprint 5 — Guerra simplificada (solo cálculo, sin instancias visuales)
Reclutamiento por infraestructura (Broncista/Curtidor-Armero/Carpintero, limitado por cobre/estaño) → combate como resolución numérica (asedio/caravana/campo abierto usando el mismo motor) → mantenimiento de tropas (deserción por hambre) → doble carril de progresión (combate real vs. progresión plana)

### Sprint 6 — Capas de sistema (cierre)
Mantenimiento de asentamientos (medidor 0-100, coste escalonado, degradación proporcional) → Score de reputación de Facción (-100 a +100, decaimiento, los 4 usos) → Sandbox sin condición de victoria: títulos dinámicos + narración de Aedas como log de texto

## Notas de alcance
Todo lo listado es simulable como datos puros, sin necesidad de arte, escenas ni modelos 3D — coherente con `Fase_0_Definicion.md`. La interfaz de debug (mapa 2D + paneles) es el único elemento "visual", y es deliberadamente mínima.
