Licensed under the Apache License, version 2.0. Copyright 2018. https://www.apache.org/licenses/LICENSE-2.0

TODO:
- make keys() iterate over dict keys in insertion order
- make sure format_val does the same as the above
- make compile_ast() verify that the ast has a coherent "type" (parent, text, operator)
- turn runtime panics into internal signals instead
- give the runtime control over interpreters of its own; runtime panics will invalidate interpreters
- allow interpreters to act like generators
- add real generators, friendly with what non-programmers expect "script" to mean (e.g. an AI script, VN script, etc - changing state, returning, later coming back to where returned from)
- add bindings for input, graphics, sound
