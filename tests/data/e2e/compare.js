console.log(1 == "1", 0 == "", null == undefined, null == 0, NaN == NaN, "1" === 1);
console.log(true == 1, false == "0", undefined == 0, 1 != 2, 1 !== "1");
const o = {};
console.log(o == o, o === o, o == null, o != undefined);
console.log(typeof 1, typeof "s", typeof true, typeof undefined, typeof null, typeof {}, typeof []);
console.log(!0, !"", !!"0", !!{}, !NaN, !!-1);
