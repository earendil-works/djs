// Serializing the value fails: the script fails with that TypeError.
const o = {};
o.o = o;
console.log("printed before the failure");
return o;
