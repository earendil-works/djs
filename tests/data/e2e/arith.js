console.log(1 + 2 * 3, 7 / 2, 7 % 3, -7 % 3, 2 ** 10, 2 ** 0.5, 1 / 0, -1 / 0, 0 / 0);
console.log(0.1 + 0.2, 1e21, 1.5e-7, 123456789 * 987654321, -0, 0 * -1);
console.log(5 & 3, 5 | 3, 5 ^ 3, ~5, 1 << 31, -16 >> 2, -16 >>> 28, 2 ** 31 | 0);
let i = 0x7fffffff; i++; console.log(i, i | 0);
console.log(+"42", +" 12.5 ", +"0x10", +"", +"abc", -"3", +true, +null, +undefined);
