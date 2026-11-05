"djs-dump";
"use strict";
$djs.seed(67108866,0);
$djs.k(async () => {
const order = [];
await Promise.all([$djs.resume(1,"call",["call","wait","\"a\""],{"id":1}).then(() => order.push("a")), $djs.resume(2,"call",["call","wait","\"b\""],{"id":2}).then(() => order.push("b"))]);
return order;
},$djs.source("codemode.js","d2f77c9ed6fbafb9","",1))()
