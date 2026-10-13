import type { Point } from './tipos';
import type { CampoElevacion } from './types';
import { evaluarElevacion, evaluarTerreno, gradienteElevacion } from './elevacion';
import { areaRegionalEn, humedadRegionalEn } from './regional';
import { evaluarForestal, type CampoForestal } from './forestacionRegional';
import type { RioRegional } from './hidrografiaRegional';
import { generarCampoRuido, evaluarRuido, type CampoRuido } from './ruido';
import { createRng, derivarSeed } from './rng';
import { esTransitable } from './costeMovimiento';
import { prepararEnclavesRegional, type EnclaveInicial } from './enclavesRegional';

export type TipoSuelo = 'aluvial' | 'humedo' | 'seco' | 'pedregoso' | 'estepa' | 'agua';
export interface CampoSuelos {
  ruido: CampoRuido; geologia: CampoRuido;
  agua: { a: Point; b: Point; alcance: number }[]; indice: Record<string, number[]>;
}
export interface RecursoRegional { id: string; tipo: 'piedra' | 'cobre' | 'estano' | 'oro' | 'ganaderia'; posicion: Point; area: number; calidad: number; categoriaEstano?: 'pequeno' | 'grande' | 'aislado' | 'cluster'; clusterEstano?: string; ofertaRelativa?: number; enclaveInicial?: EnclaveInicial }
const clamp = (x: number) => Math.max(0, Math.min(1, x));
const suave = (x: number) => { const t = clamp(x); return t * t * (3 - 2 * t); };

export function crearSuelosRegional(seed: number, rios: RioRegional[]): CampoSuelos {
  const rng = createRng(derivarSeed(seed, 'suelos-regional-1'));
  const ruido = (f: number) => generarCampoRuido(rng, { octavas: 3, frecuenciaBase: f, lacunaridad: 2.1, persistencia: 0.5 });
  const campo: CampoSuelos = { ruido: ruido(1 / 2800), geologia: ruido(1 / 6500), agua: [], indice: {} };
  for (const rio of rios) for (let i = 1; i < rio.puntos.length; i++) {
    const a = rio.puntos[i - 1]!, b = rio.puntos[i]!, alcance = 450 + rio.orden * 220, id = campo.agua.length;
    campo.agua.push({ a, b, alcance });
    for (let y = Math.floor((Math.min(a.y, b.y) - alcance) / 1000); y <= Math.floor((Math.max(a.y, b.y) + alcance) / 1000); y++)
      for (let x = Math.floor((Math.min(a.x, b.x) - alcance) / 1000); x <= Math.floor((Math.max(a.x, b.x) + alcance) / 1000); x++)
        (campo.indice[`${x},${y}`] ??= []).push(id);
  }
  return campo;
}

export function evaluarSuelo(campo: CampoSuelos, elevacion: CampoElevacion, p: Point, bosque = 0) {
  if (evaluarTerreno(elevacion, p) === 'agua') return { fertilidad: 0, pastos: 0, aluvial: 0, tipo: 'agua' as TipoSuelo };
  const g = gradienteElevacion(elevacion, p, 2), pendiente = Math.hypot(g.dx, g.dy) * 1000;
  const humedad = humedadRegionalEn(elevacion.regional!, p), h = evaluarElevacion(elevacion, p);
  let agua = 0;
  for (const id of campo.indice[`${Math.floor(p.x / 1000)},${Math.floor(p.y / 1000)}`] ?? []) {
    const { a, b, alcance } = campo.agua[id]!, dx = b.x - a.x, dy = b.y - a.y;
    const t = clamp(((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy || 1));
    agua = Math.max(agua, 1 - suave(Math.hypot(p.x - a.x - dx * t, p.y - a.y - dy * t) / alcance));
  }
  for (const lago of elevacion.lagos ?? []) {
    let distancia = Infinity;
    for (const q of lago.contorno ?? [lago.centro]) distancia = Math.min(distancia, Math.hypot(q.x - p.x, q.y - p.y));
    agua = Math.max(agua, (1 - suave(distancia / 450)) * 0.65);
  }
  const llano = 1 - suave((pendiente - 0.025) / 0.24), aluvial = agua * llano;
  const suelo = evaluarRuido(campo.ruido, p);
  const fertilidad = clamp((0.12 + humedad * 0.55 + (suelo - 0.5) * 0.4 + aluvial * 0.35) * llano * (1 - suave((h - 0.6) / 0.2)));
  const pastos = clamp((0.2 + humedad * 0.55 + suelo * 0.2) * (1 - suave(pendiente / 0.45)) * (1 - bosque * 0.85));
  const tipo: TipoSuelo = pendiente > 0.18 || h > 0.63 ? 'pedregoso' : aluvial > 0.4 ? 'aluvial' : humedad > 0.6 ? 'humedo' : humedad < 0.3 ? 'estepa' : 'seco';
  return { fertilidad, pastos, aluvial, tipo };
}

/** Provincias amplias y cuotas provisionales: no son reservas económicas ni
 * alteran los nodos de partidas. Los candidatos evitan agua y pendientes fuertes. */
export function generarRecursosRegional(seed: number, campo: CampoSuelos, elevacion: CampoElevacion, forestacion: CampoForestal, densidad = 4, accesoContinental?: (p: Point) => boolean) {
  if (![1, 2, 4, 8].includes(densidad)) throw new Error('Densidad de recursos no admitida');
  const rng = createRng(derivarSeed(seed, 'recursos-regional-1'));
  const { ancho, alto } = elevacion.regional!.limites;
  const candidatos: { posicion: Point; area: number; altura: number; geologia: number; pastos: number; azar: number }[] = [];
  for (let y = 250; y < alto; y += 500) for (let x = 250; x < ancho; x += 500) {
    const posicion = { x: x + (rng() - 0.5) * 250, y: y + (rng() - 0.5) * 250 };
    if (evaluarTerreno(elevacion, posicion) === 'agua') continue;
    const g = gradienteElevacion(elevacion, posicion, 2);
    if (Math.hypot(g.dx, g.dy) * 1000 > 0.35) continue;
    const bosque = evaluarForestal(forestacion, elevacion, posicion).densidad;
    candidatos.push({ posicion, area: areaRegionalEn(elevacion.regional!, posicion).id,
      altura: evaluarElevacion(elevacion, posicion), geologia: evaluarRuido(campo.geologia, posicion),
      pastos: evaluarSuelo(campo, elevacion, posicion, bosque).pastos, azar: rng() });
  }
  const recursos: RecursoRegional[] = [], usados = new Set<Point>();
  const cuotas: [RecursoRegional['tipo'], number][] = [['estano', 20], ['oro', 12], ['cobre', 60], ['piedra', 140], ['ganaderia', 100]];
  for (const [tipo, cuota] of cuotas) {
    const calidad = (c: typeof candidatos[number]) => tipo === 'ganaderia' ? c.pastos
      : tipo === 'piedra' ? clamp(0.3 + (c.altura - 0.4) * 2)
      : tipo === 'cobre' ? suave((c.geologia - 0.43) / 0.18) * suave((c.altura - 0.46) / 0.08)
      : tipo === 'estano' ? (1 - suave((c.geologia - 0.37) / 0.12)) * suave((c.altura - 0.48) / 0.08)
      : suave((c.geologia - 0.54) / 0.14) * suave((c.altura - 0.50) / 0.08);
    const orden = candidatos.filter(c => !usados.has(c.posicion) && calidad(c) > 0.2)
      .sort((a, b) => calidad(b) * (0.4 + b.azar * 0.6) - calidad(a) * (0.4 + a.azar * 0.6));
    let n = 0;
    const objetivo = Math.round(cuota * ancho * alto / (32000 * 24000));
    const porRegion = new Map<number, number>();
    const tope = Math.max(1, Math.ceil(objetivo * (tipo === 'piedra' ? 0.20 : tipo === 'ganaderia' ? 0.25 : 0.35)));
    while (n < objetivo) {
      let mejor = -1, valor = -Infinity;
      for (let i = 0; i < orden.length; i++) {
        const c = orden[i]!, ocupacion = porRegion.get(c.area) ?? 0;
        if (usados.has(c.posicion) || ocupacion >= tope) continue;
        const puntuacion = calidad(c) * (0.4 + c.azar * 0.6) / (1 + ocupacion * 0.18);
        if (puntuacion > valor) { mejor = i; valor = puntuacion; }
      }
      if (mejor < 0) break;
      const c = orden.splice(mejor, 1)[0]!;
      if (recursos.some(r => Math.hypot(r.posicion.x - c.posicion.x, r.posicion.y - c.posicion.y) < (r.tipo === tipo ? 700 : 220))) continue;
      recursos.push({ id: `${tipo}-${n++}`, tipo, posicion: c.posicion, area: c.area, calidad: calidad(c) });
      usados.add(c.posicion); porRegion.set(c.area, (porRegion.get(c.area) ?? 0) + 1);
    }
  }
  // Referencia histórica para elegir las cuatro provincias estratégicas.
  // Estas fuentes se sustituyen al final por aislados y concentraciones ordinarias.
  const originales = recursos.filter(r => r.tipo === 'estano').sort((a, b) => b.calidad - a.calidad);
  const resto = recursos.filter(r => r.tipo !== 'estano');
  const grandes: RecursoRegional[] = [];
  for (const r of originales) {
    if (grandes.some(g => g.area === r.area)) continue;
    grandes.push({ ...r, id: `estano-grande-${grandes.length}`, categoriaEstano: 'grande', ofertaRelativa: 8 });
    if (grandes.length === 4) break;
  }
  if (grandes.length < 4) for (const r of originales) {
    if (grandes.some(g => g.posicion === r.posicion)) continue;
    grandes.push({ ...r, id: `estano-grande-${grandes.length}`, categoriaEstano: 'grande', ofertaRelativa: 8 });
    if (grandes.length === 4) break;
  }
  const pequenos: RecursoRegional[] = [];
  for (const area of elevacion.regional!.areas) {
    const sitios = candidatos.filter(c => c.area === area.id && ![...resto, ...grandes].some(r => Math.hypot(r.posicion.x - c.posicion.x, r.posicion.y - c.posicion.y) < 220));
    sitios.sort((a, b) => {
      const valor = (c: typeof candidatos[number]) => Math.hypot(c.posicion.x - area.centro.x, c.posicion.y - area.centro.y) / 1000 + c.geologia * 0.35;
      return valor(a) - valor(b);
    });
    const c = sitios[0];
    if (c) pequenos.push({ id: `estano-pequeno-${area.id}`, tipo: 'estano', posicion: c.posicion,
      area: area.id, calidad: 0.1 + (1 - c.geologia) * 0.15, categoriaEstano: 'pequeno', ofertaRelativa: 1 });
  }
  const base = [...resto, ...pequenos, ...grandes];
  // Prospección fina independiente: añadir recursos generales antes de reconstruir
  // cobre y estaño. La densidad no altera relieve, bosques ni hidrografía.
  const extraRng = createRng(derivarSeed(seed, 'recursos-generales-regional-2'));
  const finos: { posicion: Point; area: number; altura: number; geologia: number; bosque: number; pastos: number; azar: number }[] = [];
  for (let y = 75; y < alto; y += 150) for (let x = 75; x < ancho; x += 150) {
    const posicion = { x: x + (extraRng() - 0.5) * 75, y: y + (extraRng() - 0.5) * 75 };
    const terreno = evaluarTerreno(elevacion, posicion);
    if (terreno === 'agua' || terreno === 'cima') continue;
    const g = gradienteElevacion(elevacion, posicion, 2);
    if (Math.hypot(g.dx, g.dy) * 1000 > 0.35) continue;
    const bosque = evaluarForestal(forestacion, elevacion, posicion).densidad;
    finos.push({ posicion, area: areaRegionalEn(elevacion.regional!, posicion).id,
      altura: evaluarElevacion(elevacion, posicion), geologia: evaluarRuido(campo.geologia, posicion), bosque,
      pastos: evaluarSuelo(campo, elevacion, posicion, bosque).pastos, azar: extraRng() });
  }
  const salida = [...base], indice = new Map<string, RecursoRegional[]>();
  const registrar = (r: RecursoRegional) => { const key = `${Math.floor(r.posicion.x / 250)},${Math.floor(r.posicion.y / 250)}`;
    const lista = indice.get(key) ?? []; lista.push(r); indice.set(key, lista); };
  salida.forEach(registrar);
  for (const [tipo, cuota] of [['piedra', 140], ['cobre', 60], ['oro', 12], ['ganaderia', 100]] as const) {
    const objetivo = Math.round(cuota * densidad * ancho * alto / (32000 * 24000));
    const calidad = (c: typeof finos[number]) => tipo === 'piedra' ? clamp(0.4 + (c.altura - 0.4) * 1.2)
      : tipo === 'ganaderia' ? c.pastos
      : tipo === 'cobre' ? suave((c.geologia - 0.43) / 0.18) * suave((c.altura - 0.46) / 0.08)
      : suave((c.geologia - 0.54) / 0.14) * suave((c.altura - 0.50) / 0.08);
    const orden = finos.filter(c => calidad(c) > 0.2).sort((a, b) =>
      calidad(b) * (0.4 + b.azar * 0.6) * (tipo === 'piedra' && b.bosque >= 0.15 ? 2 : 1)
      - calidad(a) * (0.4 + a.azar * 0.6) * (tipo === 'piedra' && a.bosque >= 0.15 ? 2 : 1));
    const existentes = salida.filter(r => r.tipo === tipo), porArea = new Map<number, number>();
    existentes.forEach(r => porArea.set(r.area, (porArea.get(r.area) ?? 0) + 1));
    let cantidad = existentes.length;
    const limiteArea = Math.max(1, Math.ceil(objetivo * (tipo === 'piedra' || tipo === 'ganaderia' ? 0.12 : 0.20)));
    for (const c of orden) {
      if (cantidad >= objetivo) break;
      if ((porArea.get(c.area) ?? 0) >= limiteArea) continue;
      const cx = Math.floor(c.posicion.x / 250), cy = Math.floor(c.posicion.y / 250);
      let ocupado = false;
      for (let y = cy - 1; y <= cy + 1 && !ocupado; y++) for (let x = cx - 1; x <= cx + 1 && !ocupado; x++)
        ocupado = (indice.get(`${x},${y}`) ?? []).some(r => Math.hypot(r.posicion.x - c.posicion.x, r.posicion.y - c.posicion.y) <
          (r.tipo === tipo ? tipo === 'piedra' ? 220 : 250 : 45));
      if (ocupado) continue;
      const r: RecursoRegional = { id: `general-${tipo}-${cantidad}`, tipo, posicion: c.posicion, area: c.area, calidad: calidad(c) };
      salida.push(r); registrar(r); porArea.set(c.area, (porArea.get(c.area) ?? 0) + 1); cantidad++;
    }
  }
  // Cobre distribuido: cuota regional según superficie candidata y selección por
  // distancia a las fuentes ya elegidas. La geología pondera riqueza, no excluye regiones.
  const disponibles = finos.filter(c => esTransitable(elevacion, c.posicion));
  const finales = salida.filter(r => r.tipo !== 'cobre' && r.tipo !== 'estano');
  const libres = disponibles.filter(c => !finales.some(r => Math.hypot(r.posicion.x-c.posicion.x,r.posicion.y-c.posicion.y)<45));
  const objetivoCobre = Math.round(60 * densidad * ancho * alto / (32000 * 24000));
  const cobre: RecursoRegional[] = [];
  const areas = elevacion.regional!.areas;
  const sitiosPorArea = areas.map(area => libres.filter(c => c.area === area.id));
  const pesos = sitiosPorArea.map(s => Math.sqrt(s.length));
  const sumaPesos = pesos.reduce((s, p) => s+p, 0);
  const cuotasCobre = pesos.map(p => p ? Math.max(1,Math.floor(objetivoCobre*p/sumaPesos)) : 0);
  while(cuotasCobre.reduce((s,n)=>s+n,0)<objetivoCobre){let k=0;for(let i=1;i<pesos.length;i++)if(pesos[i]!/(cuotasCobre[i]!+1)>pesos[k]!/(cuotasCobre[k]!+1))k=i;cuotasCobre[k]!++;}
  areas.forEach((area,i)=>{
    const sitios = sitiosPorArea[i]!, locales: RecursoRegional[] = [];
    const distancias = sitios.map(c=>Math.hypot(c.posicion.x-area.centro.x,c.posicion.y-area.centro.y));
    for(let n=0;n<cuotasCobre[i]!;n++){
      let mejor=-1,valor=-Infinity;
      sitios.forEach((c,j)=>{const d=distancias[j]!;if(locales.length&&d<250)return;
        const v=locales.length ? Math.min(d,4000)*(0.8+c.geologia*0.2) : -d+c.azar*100;
        if(v>valor){valor=v;mejor=j;}});
      if(mejor<0)break;
      const c=sitios[mejor]!,r:RecursoRegional={id:`cobre-disperso-${area.id}-${n}`,tipo:'cobre',posicion:c.posicion,area:area.id,calidad:clamp(0.25+c.geologia*0.65)};
      locales.push(r);cobre.push(r);sitios.forEach((s,j)=>{distancias[j]=Math.min(distancias[j]!,Math.hypot(s.posicion.x-c.posicion.x,s.posicion.y-c.posicion.y));});
    }
  });
  finales.push(...cobre);
  const tin: RecursoRegional[] = [];
  const distancia = (a:Point,b:Point)=>Math.hypot(a.x-b.x,a.y-b.y);
  const aptosTin=disponibles.filter(c=>!finales.some(r=>distancia(r.posicion,c.posicion)<45));
  // A locally gentle point can still be trapped on a ledge. Match the route
  // diagnostic's 250 m grid and 30 m segment checks before reserving a tin node.
  const rutasTin = new Map<Point, boolean>();
  const rutaLocalTin = (p: Point): boolean => {
    if(accesoContinental && !accesoContinental(p))return false;
    const cached = rutasTin.get(p); if (cached !== undefined) return cached;
    const nx = Math.ceil(ancho / 250), ny = Math.ceil(alto / 250);
    const dx = ancho / nx, dy = alto / ny;
    const segmentoLibre = (a: Point, b: Point) => {
      const pasos = Math.max(1, Math.ceil(distancia(a,b)/30));
      for(let i=0;i<=pasos;i++) if(!esTransitable(elevacion,{
        x:a.x+(b.x-a.x)*i/pasos,y:a.y+(b.y-a.y)*i/pasos})) return false;
      return true;
    };
    const cx = Math.floor(p.x/dx), cy = Math.floor(p.y/dy);
    for(let y=Math.max(0,cy-2);y<=Math.min(ny-1,cy+2);y++)
      for(let x=Math.max(0,cx-2);x<=Math.min(nx-1,cx+2);x++){
        const q={x:(x+.5)*dx,y:(y+.5)*dy};
        if(!segmentoLibre(p,q))continue;
        for(const [ox,oy] of [[-1,0],[1,0],[0,-1],[0,1]]){
          const xx=x+ox!,yy=y+oy!;if(xx<0||yy<0||xx>=nx||yy>=ny)continue;
          if(segmentoLibre(q,{x:(xx+.5)*dx,y:(yy+.5)*dy})){
            rutasTin.set(p,true);return true;
          }
        }
      }
    rutasTin.set(p,false);return false;
  };
  // Dos fuentes aisladas por región, antes de elegir las concentraciones.
  for(const area of areas){
    const sitios=aptosTin.filter(c=>c.area===area.id);
    for(let n=0;n<2;n++){
      const orden=[...sitios].sort((a,b)=>n===0 ? distancia(a.posicion,area.centro)-distancia(b.posicion,area.centro)
        : Math.min(...tin.filter(t=>t.area===area.id).map(t=>distancia(t.posicion,b.posicion)))-Math.min(...tin.filter(t=>t.area===area.id).map(t=>distancia(t.posicion,a.posicion))));
      const c=orden.find(c=>tin.every(t=>distancia(t.posicion,c.posicion)>=2000)&&rutaLocalTin(c.posicion))
        ?? orden.find(c=>tin.every(t=>distancia(t.posicion,c.posicion)>=1100)&&rutaLocalTin(c.posicion));
      if(!c)throw new Error(`Área ${area.id+1}: no hay espacio para dos fuentes de estaño separadas`);
      tin.push({id:`estano-aislado-${area.id}-${n}`,tipo:'estano',posicion:c.posicion,area:area.id,calidad:.2,categoriaEstano:'aislado',ofertaRelativa:1});
    }
  }
  // Las antiguas cuatro zonas estratégicas se enriquecen con fuentes ordinarias,
  // separadas más que el diámetro máximo propuesto (1080 m).
  for(const [i,ancla] of grandes.entries()){
    const sitios=aptosTin.filter(c=>c.area===ancla.area).sort((a,b)=>distancia(a.posicion,ancla.posicion)-distancia(b.posicion,ancla.posicion));
    for(let n=0;n<6;n++){
      const c=sitios.find(c=>tin.every(t=>distancia(t.posicion,c.posicion)>=1100)&&rutaLocalTin(c.posicion));
      if(!c)throw new Error(`Concentración ${i+1}: falta espacio para seis fuentes de estaño`);
      tin.push({id:`estano-cluster-${i}-${n}`,tipo:'estano',posicion:c.posicion,area:c.area,calidad:.2,categoriaEstano:'cluster',clusterEstano:`cluster-${i}`,ofertaRelativa:1});
    }
  }
  return prepararEnclavesRegional([...finales,...tin],elevacion,forestacion);
}
