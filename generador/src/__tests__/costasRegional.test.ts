import {describe,it,expect} from 'vitest';
import {crearCostasRegional,aplicarCostaRegional,COSTAS_REGIONALES} from '../costasRegional';
import {crearRelieveRegional,evaluarRelieveRegional} from '../regional';
import {FACTOR} from '../campamentos';
const bounds={ancho:2667*FACTOR,alto:2000*FACTOR};
const region=crearRelieveRegional(42,bounds),base=(p:{x:number;y:number})=>evaluarRelieveRegional(region,p);
const field=crearCostasRegional(42,region,base);
describe('costas (revisión 2: acantilado, roca y playa por todo el mapa, con calas, farallones y estuarios)',()=>{
 it('es serializable y determinista por semilla',()=>{
  const copy=JSON.parse(JSON.stringify(field));
  expect(crearCostasRegional(42,region,base)).toEqual(field);
  expect(crearCostasRegional(7,region,base)).not.toEqual(field);
  for(let y=0;y<bounds.alto;y+=997)for(let x=0;x<bounds.ancho;x+=1013){const p={x,y};expect(aplicarCostaRegional(field,p,base(p))).toBe(aplicarCostaRegional(copy,p,base(p)));}
 });
 it('reparte la costa entre los tres tipos y pone calas y farallones',()=>{
  const largo:Record<string,number>={acantilado:0,roca:0,playa:0};
  for(const t of field.tramos)largo[t.tipo]!+=Math.hypot(t.a.x-t.b.x,t.a.y-t.b.y);
  const total=Object.values(largo).reduce((a,b)=>a+b,0);
  // Los umbrales salen de COSTAS_REGIONALES.reparto por longitud; el tipo de un tramo es el que más pesa en él.
  expect(largo.acantilado!/total).toBeGreaterThan(COSTAS_REGIONALES.reparto.acantilado-.06);
  expect(largo.acantilado!/total).toBeLessThan(COSTAS_REGIONALES.reparto.acantilado+.06);
  expect(largo.playa!/total).toBeGreaterThan(.35);
  expect(field.calas.length).toBeGreaterThan(20);
  expect(field.farallones.length).toBeGreaterThan(20);
  for(const v of field.tramos.flatMap(t=>[t.a,t.b]))expect(v.pesos.reduce((a,b)=>a+b,0)).toBeCloseTo(1,6);
 });
 it('no toca nada lejos de la costa y solo mueve la orilla cerca de ella',()=>{
  let cambia=0,cruza=0,lejos=0;
  for(let y=0;y<bounds.alto;y+=113)for(let x=0;x<bounds.ancho;x+=127){
   const p={x,y},h=base(p),v=aplicarCostaRegional(field,p,h);
   if(Math.abs(v-h)>1e-9)cambia++;
   if((v>=COSTAS_REGIONALES.nivelMar)!==(h>=COSTAS_REGIONALES.nivelMar)){cruza++;
    // Solo cambia de tierra a mar (o al revés) cerca de la línea base: el contorno, las calas y los estuarios.
    const d=Math.min(...field.tramos.map(t=>{const dx=t.b.x-t.a.x,dy=t.b.y-t.a.y,k=Math.max(0,Math.min(1,((x-t.a.x)*dx+(y-t.a.y)*dy)/(dx*dx+dy*dy)));return Math.hypot(x-t.a.x-k*dx,y-t.a.y-k*dy);}));
    if(d>(COSTAS_REGIONALES.estuario.largo+5)*FACTOR)lejos++;}
  }
  expect(cambia).toBeGreaterThan(1000);
  expect(cruza).toBeGreaterThan(20);
  expect(lejos).toBe(0);
  const p=region.areas[2]!.centro;expect(aplicarCostaRegional(field,p,base(p))).toBe(base(p));
 });
 it('es continua al cruzar el borde de su alcance',()=>{
  // A lo largo de rectas que salen de la costa, ningún salto entre muestras a 0,25 u.
  let peor=0;
  for(const t of field.tramos.filter((_,i)=>i%37===0)){
   for(const sentido of [1,-1])for(let d=(COSTAS_REGIONALES.alcance-4)*FACTOR;d<(COSTAS_REGIONALES.alcance+4)*FACTOR;d+=3){
    const a={x:t.a.x+sentido*t.a.nx*d,y:t.a.y+sentido*t.a.ny*d},b={x:a.x+sentido*t.a.nx*3,y:a.y+sentido*t.a.ny*3};
    const ha=aplicarCostaRegional(field,a,base(a)),hb=aplicarCostaRegional(field,b,base(b));
    // Solo donde la costa más cercana está en el borde del alcance (la recta puede acercarse a otra costa).
    const dm=Math.min(...field.tramos.map(o=>{const dx=o.b.x-o.a.x,dy=o.b.y-o.a.y,k=Math.max(0,Math.min(1,((a.x-o.a.x)*dx+(a.y-o.a.y)*dy)/(dx*dx+dy*dy)));return Math.hypot(a.x-o.a.x-k*dx,a.y-o.a.y-k*dy);}))/FACTOR;
    if(Math.abs(dm-COSTAS_REGIONALES.alcance)<2)peor=Math.max(peor,Math.abs(hb-ha));
   }
  }
  expect(peor).toBeLessThan(.002);
 });
 it('una desembocadura queda en playa y, en tierra baja, abre un estuario',()=>{
  const rioEn=(v:{x:number;y:number;nx:number;ny:number})=>{const n={x:-v.nx,y:-v.ny};
   return {n,rio:{id:'r',fin:'agua',terminaEnLago:false,navegable:false,puntos:[{x:v.x-n.x*60*FACTOR,y:v.y-n.y*60*FACTOR},{x:v.x-n.x*20*FACTOR,y:v.y-n.y*20*FACTOR},{x:v.x+n.x*2*FACTOR,y:v.y+n.y*2*FACTOR}]} as any};};
  const v=field.tramos.find(s=>s.tipo==='acantilado')!.a;
  const conRio=crearCostasRegional(42,region,base,[rioEn(v).rio]);
  expect(conRio.tramos.find(s=>s.a.x===v.x&&s.a.y===v.y)!.a.pesos[2]).toBeCloseTo(1,6);
  expect(conRio.estuarios.length).toBeGreaterThan(0);
  // El estuario, donde la tierra de detrás es más baja: a 10 u río arriba, sobre el cauce, hay agua de mar.
  const adentro=(a:{x:number;y:number;nx:number;ny:number})=>({x:a.x+a.nx*10*FACTOR,y:a.y+a.ny*10*FACTOR});
  const w=field.tramos.map(s=>s.a).reduce((m,a)=>base(adentro(a))<base(adentro(m))?a:m);
  const {rio}=rioEn(w),conEstuario=crearCostasRegional(42,region,base,[rio]),q=adentro(w);
  expect(aplicarCostaRegional(conEstuario,q,base(q))).toBeLessThan(COSTAS_REGIONALES.nivelMar);
 });
});
