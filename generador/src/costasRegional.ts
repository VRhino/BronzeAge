import type { Point, RioZona } from './tipos';
import type { RelieveRegional } from './regional';
import { createRng, derivarSeed } from './rng';

export const COSTAS_REGIONALES = { revision: 'costas-1', nivelMar: .375, muestreo: 100,
  alcance: 260, celdaIndice: 300, alturaMinima: 32, alturaMaxima: 76,
  anchoFrenteMinimo: 30, anchoFrenteMaximo: 48, proteccionDesembocadura: 360 } as const;
export interface VerticeCostero extends Point { nx: number; ny: number; intensidad: number; altura: number; anchoFrente: number }
export interface TramoCostero { id: string; a: VerticeCostero; b: VerticeCostero; tipo: 'acantilado' | 'roca' | 'playa' }
export interface CampoCostero { revision: string; seed: number; config: typeof COSTAS_REGIONALES;
  tramos: TramoCostero[]; indice: Record<string, number[]> }
const suave=(v:number)=>{const t=Math.max(0,Math.min(1,v));return t*t*(3-2*t);};
const lerp=(a:number,b:number,t:number)=>a+(b-a)*t;
function ruido(x:number,y:number,f:number):number {
 const ix=Math.floor(x),iy=Math.floor(y),tx=suave(x-ix),ty=suave(y-iy);
 const h=(a:number,b:number)=>{const n=Math.sin(a*127.1+b*311.7+f*73.3)*43758.5453;return n-Math.floor(n);};
 return lerp(lerp(h(ix,iy),h(ix+1,iy),tx),lerp(h(ix,iy+1),h(ix+1,iy+1),tx),ty);
}
/** The continuous geographic field is constructed before forest, soils and resources.
 * Sampling callbacks must omit any previous coast field: rebuilding is idempotent. */
export function crearCostasRegional(seed:number,regional:RelieveRegional,alturaBase:(p:Point)=>number,rios:readonly RioZona[]=[]):CampoCostero {
 const config=COSTAS_REGIONALES,rng=createRng(derivarSeed(seed,'costas-regional-1')),fase=rng()*1000;
 const campo:CampoCostero={revision:config.revision,seed,config,tramos:[],indice:{}};
 const {ancho,alto}=regional.limites,nx=Math.ceil(ancho/config.muestreo),ny=Math.ceil(alto/config.muestreo);
 const heights=new Float64Array((nx+1)*(ny+1));
 for(let y=0;y<=ny;y++)for(let x=0;x<=nx;x++)heights[y*(nx+1)+x]=alturaBase({x:Math.min(ancho,x*config.muestreo),y:Math.min(alto,y*config.muestreo)});
 const mouths=rios.flatMap(r=>r.puntos.slice(-6)).filter(p=>alturaBase(p)<.42);
 const vertices=new Map<string,VerticeCostero>();
 function vertex(a:Point,b:Point,ha:number):VerticeCostero {
  const key=[a.x,a.y,b.x,b.y].join(',');const cached=vertices.get(key);if(cached)return cached;
  let low=0,high=1;for(let i=0;i<16;i++){const t=(low+high)/2,h=alturaBase({x:lerp(a.x,b.x,t),y:lerp(a.y,b.y,t)});if((h>=config.nivelMar)===(ha>=config.nivelMar))low=t;else high=t;}
  const t=(low+high)/2,p={x:lerp(a.x,b.x,t),y:lerp(a.y,b.y,t)};
  const gx=alturaBase({x:p.x+15,y:p.y})-alturaBase({x:p.x-15,y:p.y});
  const gy=alturaBase({x:p.x,y:p.y+15})-alturaBase({x:p.x,y:p.y-15}),g=Math.hypot(gx,gy)||1;
  const x=p.x/ancho,y=p.y/alto;
  const roca=Math.max((1-suave((x-.35)/.06))*(1-suave((y-.52)/.06)),suave((x-.35)/.06)*(1-suave((y-.29)/.06)));
  const variation=ruido(p.x/720,p.y/720,fase),detail=ruido(p.x/170,p.y/170,fase+17);
  let intensidad=roca*suave((variation-.25)/.38);
  for(const mouth of mouths)intensidad*=suave((Math.hypot(p.x-mouth.x,p.y-mouth.y)-100)/(config.proteccionDesembocadura-100));
  const v={...p,nx:gx/g,ny:gy/g,intensidad,altura:lerp(config.alturaMinima,config.alturaMaxima,variation*.65+detail*.35),anchoFrente:lerp(config.anchoFrenteMinimo,config.anchoFrenteMaximo,detail)};
  vertices.set(key,v);return v;
 }
 function segment(a:VerticeCostero,b:VerticeCostero){
  if(Math.hypot(a.x-b.x,a.y-b.y)<.001)return;
  const intensity=(a.intensidad+b.intensidad)/2,index=campo.tramos.length;
  campo.tramos.push({id:`costa-${index}`,a,b,tipo:intensity>.55?'acantilado':intensity>.12?'roca':'playa'});
  for(let y=Math.floor((Math.min(a.y,b.y)-config.alcance)/config.celdaIndice);y<=Math.floor((Math.max(a.y,b.y)+config.alcance)/config.celdaIndice);y++)
   for(let x=Math.floor((Math.min(a.x,b.x)-config.alcance)/config.celdaIndice);x<=Math.floor((Math.max(a.x,b.x)+config.alcance)/config.celdaIndice);x++)(campo.indice[`${x},${y}`]??=[]).push(index);
 }
 for(let y=0;y<ny;y++)for(let x=0;x<nx;x++){
  const x0=x*config.muestreo,x1=Math.min(ancho,(x+1)*config.muestreo),y0=y*config.muestreo,y1=Math.min(alto,(y+1)*config.muestreo);
  const ps=[{x:x0,y:y0},{x:x1,y:y0},{x:x1,y:y1},{x:x0,y:y1}];
  const hs=[heights[y*(nx+1)+x]!,heights[y*(nx+1)+x+1]!,heights[(y+1)*(nx+1)+x+1]!,heights[(y+1)*(nx+1)+x]!];
  const edges=[[0,1],[1,2],[3,2],[0,3]],cross:VerticeCostero[]=[];
  for(const [a,b] of edges as [number,number][])if((hs[a]!>=config.nivelMar)!==(hs[b]!>=config.nivelMar))cross.push(vertex(ps[a]!,ps[b]!,hs[a]!));
  if(cross.length===2)segment(cross[0]!,cross[1]!);
  else if(cross.length===4){const center=alturaBase({x:(x0+x1)/2,y:(y0+y1)/2})>=config.nivelMar;
   if(center===(hs[0]!>=config.nivelMar)){segment(cross[0]!,cross[1]!);segment(cross[2]!,cross[3]!);}else{segment(cross[0]!,cross[3]!);segment(cross[1]!,cross[2]!);}}
 }
 return campo;
}
/** Same globally indexed field for terrain, slope, forest and exported geometry. */
export function aplicarCostaRegional(campo:CampoCostero,p:Point,base:number):number {
 const c=campo.config;let nearest=Infinity,best:TramoCostero|undefined,tBest=0;
 for(const id of campo.indice[`${Math.floor(p.x/c.celdaIndice)},${Math.floor(p.y/c.celdaIndice)}`]??[]){
  const s=campo.tramos[id]!,dx=s.b.x-s.a.x,dy=s.b.y-s.a.y;
  const t=Math.max(0,Math.min(1,((p.x-s.a.x)*dx+(p.y-s.a.y)*dy)/(dx*dx+dy*dy)));
  const d=Math.hypot(p.x-lerp(s.a.x,s.b.x,t),p.y-lerp(s.a.y,s.b.y,t));if(d<nearest){nearest=d;best=s;tBest=t;}
 }
 if(!best||nearest>=c.alcance)return base;
 const intensity=lerp(best.a.intensidad,best.b.intensidad,tBest);if(intensity<=0)return base;
 const land=base>=c.nivelMar,dist=nearest*(land?1:-1),width=lerp(best.a.anchoFrente,best.b.anchoFrente,tBest);
 const h=lerp(best.a.altura,best.b.altura,tBest)/1000;
 const target=land?c.nivelMar+h*suave(dist/width):c.nivelMar+Math.max(-.055,dist*.0013);
 const fade=land?1-suave((dist-width)/(c.alcance-width)):1-suave(-dist/80);
 return base+(target-base)*intensity*fade;
}
