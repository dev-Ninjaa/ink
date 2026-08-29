import { a } from './nested/a';
import { missing } from './nope';

export function run() {
  return a + (missing ? 1 : 0);
}