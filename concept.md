New programming language:

* Works for both cpu and gpu on the web
* Primary targets are wasm and wgsl
* Inspired by rust
    * Syntax
    * Lifetimes
    * Traits
    * Unsafe
* Types can be treated as constant values
    * Can be passed into/out of const functions to replace generic types
    * Can be passed into functions for generic functions
    * Memory is represented as a constructed type
        * Gets compiled down to either a wasm memory section or a wgsl readwrite storage array
* Types
    * Struct
    * Enum
    * Tuple
    * Unit(0 length tuple)
    * Array
    * Slice
    * Numbers (depending on target)
        * u8, u16, u32, u64
        * i8, i16, i32, i64
        * f16, f32, f64
        * Integer literals currently default to `i32`. Use type annotations, parameters, or explicit coercions to work with other widths, and note that the type checker does not perform implicit promotions between widths or signedness.
    * Borrow/mutable borrow
    * Raw pointers(unsafe only)
* Operator overloading can be done via traits

Most types will live as locals, reading/writing from memory will require passing in the memory to use.
String constants will be translated to local `u8` arrays, but can be written to/read from a memory section.
Similar to rust can use newtype pattern using 1 element tuple.
