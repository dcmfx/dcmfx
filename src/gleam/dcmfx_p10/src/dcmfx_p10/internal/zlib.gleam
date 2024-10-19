//// Functions for streaming zlib compression and decompression. On Erlang this
//// is provided by the standard library, and on JavaScript it's provided by
//// pako.js.

import dcmfx_p10/internal/zlib/flush_command.{type FlushCommand}
import dcmfx_p10/internal/zlib/inflate_result.{type InflateResult}

pub type ZlibStream

@external(erlang, "zlib", "open")
@external(javascript, "../../pako_ffi.mjs", "open")
pub fn open() -> ZlibStream

pub type Zmethod {
  Deflated
}

pub type Zstrategy {
  Default
  Filtered
  HuffmanOnly
  Rle
}

@external(erlang, "zlib", "deflateInit")
@external(javascript, "../../pako_ffi.mjs", "deflateInit")
pub fn deflate_init(
  stream: ZlibStream,
  level: Int,
  method: Zmethod,
  window_bits: Int,
  mem_level: Int,
  strategy: Zstrategy,
) -> Nil

@external(erlang, "zlib", "inflateInit")
@external(javascript, "../../pako_ffi.mjs", "inflateInit")
pub fn inflate_init(stream: ZlibStream, window_bits: Int) -> Nil

@external(erlang, "zlib", "deflate")
@external(javascript, "../../pako_ffi.mjs", "deflate")
pub fn deflate(
  stream: ZlibStream,
  data: BitArray,
  flush: FlushCommand,
) -> List(BitArray)

@external(erlang, "dcmfx_p10_ffi", "zlib_safeInflate")
@external(javascript, "../../pako_ffi.mjs", "safeInflate")
pub fn safe_inflate(
  zlib_stream: ZlibStream,
  input_bytes: BitArray,
) -> Result(InflateResult, Nil)
