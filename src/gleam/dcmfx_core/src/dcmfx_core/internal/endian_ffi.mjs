import { BitArray } from "../../../prelude.mjs";

export function swap_16_bit(bytes, _acc) {
  const length = Math.floor(bytes.length / 2) * 2;
  const result = new BitArray(new Uint8Array(length));

  for (let i = 0; i < length; i += 2) {
    result.buffer[i] = bytes.buffer[i + 1];
    result.buffer[i + 1] = bytes.buffer[i];
  }

  return result;
}

export function swap_32_bit(bytes, _acc) {
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

export function swap_64_bit(bytes, _acc) {
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
