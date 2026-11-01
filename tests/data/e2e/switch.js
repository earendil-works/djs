for (let v = 0; v < 6; v++) {
  switch (v) {
    case 0: console.log("zero");
    case 1: console.log("one"); break;
    case "2": console.log("string two"); break;
    default: console.log("default", v); break;
    case 4: console.log("four");
  }
}
