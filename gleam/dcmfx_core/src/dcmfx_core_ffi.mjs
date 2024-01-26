import { BitArray } from "../prelude.mjs";

export function decimal_string__float_to_shortest_string(f) {
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

export function endian__swap_16_bit(bytes, _acc) {
  const length = Math.floor(bytes.length / 2) * 2;
  const result = new BitArray(new Uint8Array(length));

  for (let i = 0; i < length; i += 2) {
    result.buffer[i] = bytes.buffer[i + 1];
    result.buffer[i + 1] = bytes.buffer[i];
  }

  return result;
}

export function endian__swap_32_bit(bytes, _acc) {
  const length = Math.floor(bytes.length / 4) * 4;
  const result = new BitArray(new Uint8Array(length));

  for (let i = 0; i < length; i += 4) {
    result.buffer[i] = bytes.buffer[i + 3];
    result.buffer[i + 1] = bytes.buffer[i + 2];
    result.buffer[i + 2] = bytes.buffer[i + 1];
    result.buffer[i + 3] = bytes.buffer[i];
  }

  return result;
}

export function endian__swap_64_bit(bytes, _acc) {
  const length = Math.floor(bytes.length / 8) * 8;
  const result = new BitArray(new Uint8Array(length));

  for (let i = 0; i < length; i += 8) {
    result.buffer[i] = bytes.buffer[i + 7];
    result.buffer[i + 1] = bytes.buffer[i + 6];
    result.buffer[i + 2] = bytes.buffer[i + 5];
    result.buffer[i + 3] = bytes.buffer[i + 4];
    result.buffer[i + 4] = bytes.buffer[i + 3];
    result.buffer[i + 5] = bytes.buffer[i + 2];
    result.buffer[i + 6] = bytes.buffer[i + 1];
    result.buffer[i + 7] = bytes.buffer[i];
  }

  return result;
}

export function utils__pad_start(string, desired_length, pad_string) {
  return string.padStart(desired_length, pad_string);
}

export function utils__spaces(n) {
  if (n <= 0) {
    return "";
  }

  return " ".repeat(n);
}

export function utils__string_fast_length(string) {
  return string.length;
}
