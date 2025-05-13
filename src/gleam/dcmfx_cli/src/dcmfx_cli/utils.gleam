@external(erlang, "erlang", "halt")
@external(javascript, "node:process", "exit")
pub fn exit_with_status(status: Int) -> Nil

/// Converts a `Bool` to an `Int`, either `1` or `0`.
///
pub fn bool_to_int(b: Bool) -> Int {
  case b {
    True -> 1
    False -> 0
  }
}
