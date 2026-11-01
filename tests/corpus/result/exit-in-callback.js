await Promise.resolve();
[1, 2, 3].forEach((x) => {
  console.log(x);
  if (x === 2) exit();
});
return "unreachable";
