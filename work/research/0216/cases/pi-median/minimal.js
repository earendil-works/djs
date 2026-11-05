"djs-dump";
"use strict";
$djs.seed(1975298774,1374804631,4);
$djs.k(async (tools) => {
const [a, b, c] = await Promise.all([1,$djs.resume(2,"call",["call","delay","2"],{"id":2}),3]);
return { values: [a, b, c], names: Object.keys(tools) };
},$djs.source("codemode.js","54e8cfc5dc328017","",1))($djs.host("tools"))
