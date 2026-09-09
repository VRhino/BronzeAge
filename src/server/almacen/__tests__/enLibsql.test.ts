// Adaptador libSQL: el mismo contrato que `enDisco`, contra una base de datos SQLite en memoria. Ejercita el
// SQL de verdad: el UPSERT, el `||` del append y el `LIKE ... ESCAPE`. Cada `createClient(':memory:')` es una
// base nueva y aislada, así que no hace falta limpiar entre tests.
import { crearAlmacenEnLibsql } from '../enLibsql';
import { pruebasDeContrato } from './contrato';

pruebasDeContrato('enLibsql', () => crearAlmacenEnLibsql({ url: ':memory:' }));
