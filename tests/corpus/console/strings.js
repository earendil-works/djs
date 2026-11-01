// Strings are printed raw at the top level and JSON-quoted inside objects and arrays.
console.log('quotes " and \' and `');
console.log("multi\nline\tand tab");
console.log(["quotes \" and '", "back\\slash", "nl\n", "tab\t", "nul\u0000", "\u001f\u007f"]);
console.log({ "key with space": 1, "quote\"key": 2, "": 3 });
console.log("unicode: é ü ß 漢字 😀", ["é", "😀", "\u2028\u2029"]);
console.log(["lone \ud800 surrogate", "\udc00"]);
console.log("x".repeat(300));
