// mode: body
console.log(this === globalThis, typeof this);
var local = 1;
console.log(typeof globalThis.local);
