"djs-dump";
"use strict";
$djs.seed(2135587861,-1640531527);
async function main() {
  const g = counter(4);
  let r = g.next();
  while (!r.done) {
    const v = await later(r.value * 10);
    print("got", v);
    r = g.next(v);
  }
  print("total", r.value);
}
function* counter(limit) {
  let total = 0;
  for (let i = 0; i < limit; i++) {
    const got = yield i;
    total += got;
  }
  return total;
}
{const $g=$djs.k(function* (limit, total) {
let i = 1;
{
const got = yield;
total += got;
}
for (i++; i < limit; i++) {
    const got = yield i;
    total += got;
  }
return total;
},counter,"yield#1")(4,0);
$g.next();
$djs.k(async function (g, r) {
{
const v = await $djs.resume(3,"later",[10]);
print("got", v);
r = g.next(v);
}
while (!r.done) {
    const v = await later(r.value * 10);
    print("got", v);
    r = g.next(v);
  }
print("total", r.value);
},main)($g,{value:1,done:false})}
