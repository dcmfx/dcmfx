export function float_to_shortest_string(f) {
  let a = f.toString();
  if (a.indexOf(".") === -1) {
    a += ".0";
  }

  if (a.length <= 16) {
    return a;
  }

  const b = f.toExponential();
  if (b.length < a.length) {
    return b;
  }

  return a;
}
