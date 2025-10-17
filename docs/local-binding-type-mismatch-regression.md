# Local binding type mismatch regression

## Summary

The recent change that enforces equality between a local binding's recorded type
(from the resolver's stack) and the expression type recorded on each local use
causes the stage1 compiler to reject valid programs.  Numeric literals and other
expressions often carry a wider builtin type than the annotated local, so the new
check triggers even when the binding is well-typed.

## Evidence

* When resolving a `let` expression, the resolver stores the initializer's type
  into the local stack without rewriting it to the annotated local type.  For the
  declaration `let neg_i8: i8 = -1;`, the initializer literal is parsed as an
  `i32`, so the recorded type for the binding is `i32`.
* Every use of that local is parsed with the annotated type.  The parser writes
  the declared type into the expression node when it builds a local reference,
  so the AST records `neg_i8` as an `i8`.
* The new resolver guard rejects any mismatch between the recorded initializer
  type (`i32`) and the expression type (`i8`), which triggers immediately when
  another binding (such as `let neg_as_u8: u8 = neg_i8 as u8;`) references the
  local.  The same strict comparison was added to the const-specialization
  cloning path, so both normal resolution and template cloning now abort on this
  legitimate program shape.
* As a result the stage1 compiler aborts with `local binding type mismatch`
  errors and downstream tests like `test/casts.test.ts` fail even though they only
  rely on standard integer casts.

## Conclusion

The regression happens because the recorded initializer type is not guaranteed to
match the annotated local type—integer literals default to `i32`, but locals may
be narrower.  Requiring exact equality between these fields rejects valid code
paths.  A fix will need to reconcile these representations (for example by
normalising the recorded type to the annotated type or by checking for
convertibility instead of equality).
