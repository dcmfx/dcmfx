import { BitArray } from "../../../prelude.mjs";

export function swap_16_bit(bytes, _acc) {
  const length = Math.floor(bytes.bitSize / 16) * 2;

  const buffer = new Uint8Array(length);
  for (let i = 0; i < length; i += 2) {
    buffer[i] = bytes.byteAt(i + 1);
    buffer[i + 1] = bytes.byteAt(i);
  }

  return new BitArray(buffer);
}

export function swap_32_bit(bytes, _acc) {
  const length = Math.floor(bytes.bitSize / 32) * 4;

  const buffer = new Uint8Array(length);
  for (let i = 0; i < length; i += 4) {
    buffer[i] = bytes.byteAt(i + 3);
    buffer[i + 1] = bytes.byteAt(i + 2);
    buffer[i + 2] = bytes.byteAt(i + 1);
    buffer[i + 3] = bytes.byteAt(i);
  }

  return new BitArray(buffer);
}

export function swap_64_bit(bytes, _acc) {
  const length = Math.floor(bytes.bitSize / 64) * 8;

  const buffer = new Uint8Array(length);
  for (let i = 0; i < length; i += 8) {
    buffer[i] = bytes.byteAt(i + 7);
    buffer[i + 1] = bytes.byteAt(i + 6);
    buffer[i + 2] = bytes.byteAt(i + 5);
    buffer[i + 3] = bytes.byteAt(i + 4);
    buffer[i + 4] = bytes.byteAt(i + 3);
    buffer[i + 5] = bytes.byteAt(i + 2);
    buffer[i + 6] = bytes.byteAt(i + 1);
    buffer[i + 7] = bytes.byteAt(i);
  }

  return new BitArray(buffer);
}
