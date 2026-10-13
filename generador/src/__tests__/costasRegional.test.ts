import {describe,it,expect} from 'vitest';
import {crearCostasRegional,aplicarCostaRegional} from '../costasRegional';
import {crearRelieveRegional,evaluarRelieveRegional} from '../regional';
import {crearCostasUnity,definirBloquesUnity,muestrearBloqueUnity} from '../../scripts/unity-regional.datos';
import {generarCampoElevacion,evaluarElevacion} from '../elevacion';
import {createRng} from '../rng';
const bounds={ancho:8000,alto:6000};
describe('canonical coastal morphology',()=>{
 it('is serializable and seed deterministic, varies types and preserves inland and water classification',()=>{
  const region=crearRelieveRegional(42,bounds),base=(p:{x:number;y:number})=>evaluarRelieveRegional(region,p);
  const field=crearCostasRegional(42,region,base),copy=JSON.parse(JSON.stringify(field));
  expect(crearCostasRegional(42,region,base)).toEqual(field);
  expect(crearCostasRegional(7,region,base)).not.toEqual(field);
  expect(new Set(field.tramos.map(s=>s.tipo)).size).toBe(3);
  let changed=0;for(let y=0;y<6000;y+=71)for(let x=0;x<8000;x+=79){const p={x,y},h=base(p),v=aplicarCostaRegional(field,p,h);
   expect(v).toBe(aplicarCostaRegional(copy,p,h));expect(v>=.375).toBe(h>=.375);if(Math.abs(v-h)>1e-6)changed++;
  }
  expect(changed).toBeGreaterThan(20);
  const p=region.areas[2]!.centro;expect(aplicarCostaRegional(field,p,base(p))).toBe(base(p));
 });
 it('protects a river mouth and shares exact terrain samples at sector borders',()=>{
  const region=crearRelieveRegional(42,bounds),base=(p:{x:number;y:number})=>evaluarRelieveRegional(region,p);
  const field=crearCostasRegional(42,region,base),v=field.tramos.find(s=>s.tipo==='acantilado')!.a;
  const protectedField=crearCostasRegional(42,region,base,[{puntos:[{x:v.x,y:v.y}]} as any]);
  const same=protectedField.tramos.find(s=>s.a.x===v.x&&s.a.y===v.y)!;expect(same.a.intensidad).toBe(0);
  const height=(x:number,y:number)=>aplicarCostaRegional(field,{x,y},base({x,y}))*1000;
  const blocks=definirBloquesUnity(8000,8000,4000,33),a=muestrearBloqueUnity(blocks[0]!,height),b=muestrearBloqueUnity(blocks[1]!,height);
  for(let z=0;z<33;z++)expect(a.raw.readUInt16LE((z*33+32)*2)).toBe(b.raw.readUInt16LE(z*33*2));
 });
 it('exports faces from the same canonical elevation evaluator',()=>{
  const e=generarCampoElevacion(createRng(1),bounds);e.regional=crearRelieveRegional(42,bounds);
  e.costas=crearCostasRegional(42,e.regional,p=>evaluarElevacion(e,p));
  const exported=crearCostasUnity(e)!;expect(exported.segmentos.length).toBeGreaterThan(0);
  for(const s of exported.segmentos)for(const p of s.puntos)expect(p.alturaY).toBeCloseTo(evaluarElevacion(e,{x:p.x,y:p.z})*1000+.8,8);
 });
});
