import {describe,it,expect} from 'vitest';
import {crearCostasRegional,aplicarCostaRegional} from '../costasRegional';
import {crearRelieveRegional,evaluarRelieveRegional} from '../regional';
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
 it('protects a river mouth',()=>{
  const region=crearRelieveRegional(42,bounds),base=(p:{x:number;y:number})=>evaluarRelieveRegional(region,p);
  const field=crearCostasRegional(42,region,base),v=field.tramos.find(s=>s.tipo==='acantilado')!.a;
  const protectedField=crearCostasRegional(42,region,base,[{puntos:[{x:v.x,y:v.y}]} as any]);
  const same=protectedField.tramos.find(s=>s.a.x===v.x&&s.a.y===v.y)!;expect(same.a.intensidad).toBe(0);
 });
});
