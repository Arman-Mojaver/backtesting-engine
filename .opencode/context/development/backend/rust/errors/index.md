<!-- Context: development/backend/rust/errors/index | Priority: high | Version: 1.0 | Updated: 2026-05-24 -->

# Rust Compiler Error Index

Quick lookup: error code → category file. Load the category file for full explanation + fix examples.

**Source**: https://doc.rust-lang.org/error_codes/error-index.html

---

## How to Use

1. Find your error code in the table below
2. Note the **Category** column
3. Load the corresponding file: `.opencode/context/development/backend/rust/errors/{category}.md`
4. Find the error section for the exact fix

---

## Error Code → Category Map

| Code | Summary | Category File |
|------|---------|---------------|
| E0004 | Non-exhaustive match patterns | `patterns.md` |
| E0005 | Pattern binds by reference and value | `patterns.md` |
| E0007 | Binding moved value in pattern | `patterns.md` |
| E0009 | Mixing by-ref and by-move in pattern | `patterns.md` |
| E0023 | Wrong number of fields in pattern | `patterns.md` |
| E0025 | Field bound multiple times in pattern | `patterns.md` |
| E0026 | Struct pattern has extra fields | `patterns.md` |
| E0027 | Struct pattern missing fields | `patterns.md` |
| E0029 | Non-scalar type in range pattern | `patterns.md` |
| E0030 | Range pattern lower > upper bound | `patterns.md` |
| E0033 | Trait object cannot be dereferenced | `types.md` |
| E0034 | Multiple applicable methods (ambiguous) | `traits.md` |
| E0038 | Trait not object-safe | `traits.md` |
| E0040 | Explicit destructor call | `types.md` |
| E0044 | Foreign items cannot have type/const params | `generics.md` |
| E0045 | Variadic function must have C calling convention | `types.md` |
| E0046 | Missing trait item in impl | `traits.md` |
| E0049 | Wrong number of type params in trait impl | `traits.md` |
| E0050 | Wrong number of function params in trait impl | `traits.md` |
| E0053 | Method signature mismatch in trait impl | `traits.md` |
| E0054 | Invalid cast | `types.md` |
| E0055 | Infinite auto-deref | `types.md` |
| E0057 | Wrong number of args to closure | `types.md` |
| E0059 | Invalid use of `...` in function args | `types.md` |
| E0060 | Wrong number of args to variadic function | `types.md` |
| E0061 | Wrong number of args to function | `types.md` |
| E0062 | Field specified more than once in struct literal | `types.md` |
| E0063 | Missing fields in struct literal | `types.md` |
| E0067 | Invalid left-hand side of assignment | `types.md` |
| E0069 | Return with value in `!` function | `types.md` |
| E0070 | Invalid left-hand side of assignment | `types.md` |
| E0071 | Expected struct/enum, found other | `types.md` |
| E0072 | Recursive type has infinite size | `types.md` |
| E0075 | Newtype struct must have exactly one field | `types.md` |
| E0080 | Constant evaluation error | `types.md` |
| E0081 | Discriminant value already used | `types.md` |
| E0084 | Non-integer enum discriminant | `types.md` |
| E0106 | Missing lifetime specifier | `lifetimes.md` |
| E0107 | Wrong number of generic args | `generics.md` |
| E0109 | Type args provided for non-generic type | `generics.md` |
| E0110 | Lifetime args provided for non-generic type | `lifetimes.md` |
| E0116 | Cannot define inherent impl for foreign type | `traits.md` |
| E0117 | Orphan rule violation (foreign trait + foreign type) | `traits.md` |
| E0118 | Inherent impl on non-nominal type | `traits.md` |
| E0119 | Conflicting trait implementations | `traits.md` |
| E0120 | Drop impl on non-struct/enum | `traits.md` |
| E0121 | `_` type placeholder in signatures | `types.md` |
| E0124 | Field declared twice in struct | `types.md` |
| E0128 | Type param with default used in other default | `generics.md` |
| E0131 | `main` cannot have type params | `types.md` |
| E0133 | Unsafe code outside unsafe block | `types.md` |
| E0152 | Duplicate lang item | `types.md` |
| E0161 | Cannot move unsized value | `ownership-borrow.md` |
| E0162 | Irrefutable `if let` pattern | `patterns.md` |
| E0164 | Non-struct/variant in tuple struct pattern | `patterns.md` |
| E0165 | Irrefutable `while let` pattern | `patterns.md` |
| E0178 | Invalid `+` in type position | `types.md` |
| E0183 | Manual implementation of `Fn*` trait | `traits.md` |
| E0184 | Cannot implement both `Copy` and `Drop` | `traits.md` |
| E0185 | Associated function in trait impl has `self` | `traits.md` |
| E0186 | Method in trait impl missing `self` | `traits.md` |
| E0191 | Trait object missing associated type binding | `traits.md` |
| E0192 | Negative impl on non-auto trait | `traits.md` |
| E0195 | Lifetime params don't match trait declaration | `lifetimes.md` |
| E0197 | Inherent impl cannot be unsafe | `types.md` |
| E0198 | Negative impl cannot be unsafe | `traits.md` |
| E0199 | Implementing unsafe trait without `unsafe` | `traits.md` |
| E0200 | Implementing safe trait with `unsafe` | `traits.md` |
| E0201 | Duplicate definitions in impl | `traits.md` |
| E0204 | Cannot derive `Copy` (contains non-Copy field) | `traits.md` |
| E0206 | `Copy` impl on non-ADT | `traits.md` |
| E0207 | Unconstrained type/lifetime param in impl | `generics.md` |
| E0210 | Orphan rule: type param must be local | `traits.md` |
| E0211 | Wrong number of type args to function/method | `generics.md` |
| E0214 | Type provided where lifetime expected | `lifetimes.md` |
| E0220 | Unknown associated type in trait bound | `generics.md` |
| E0221 | Ambiguous associated type | `generics.md` |
| E0222 | Ambiguous associated type in bounds | `generics.md` |
| E0223 | Ambiguous associated type path | `generics.md` |
| E0224 | Trait bound required for associated type | `generics.md` |
| E0225 | Only auto-traits allowed after first trait bound | `traits.md` |
| E0226 | More than one explicit lifetime bound | `lifetimes.md` |
| E0227 | Ambiguous lifetime bound | `lifetimes.md` |
| E0228 | Lifetime bound required | `lifetimes.md` |
| E0229 | Associated type binding in non-trait path | `generics.md` |
| E0230 | Trait has no associated type | `generics.md` |
| E0231 | Associated type binding missing | `generics.md` |
| E0232 | Associated type binding has no value | `generics.md` |
| E0243 | Wrong number of type args | `generics.md` |
| E0244 | Wrong number of type args | `generics.md` |
| E0251 | Two params with same name | `generics.md` |
| E0252 | Import name conflict | `modules.md` |
| E0253 | Cannot import non-public item | `modules.md` |
| E0254 | Import conflicts with existing definition | `modules.md` |
| E0255 | Import conflicts with existing definition | `modules.md` |
| E0256 | Import conflicts with type param | `modules.md` |
| E0259 | Extern crate name conflict | `modules.md` |
| E0260 | Name conflict with extern crate | `modules.md` |
| E0261 | Undeclared lifetime | `lifetimes.md` |
| E0262 | Invalid lifetime name | `lifetimes.md` |
| E0263 | Lifetime declared twice in same scope | `lifetimes.md` |
| E0264 | Unknown external lang item | `types.md` |
| E0267 | `break`/`continue` inside closure | `types.md` |
| E0268 | `break`/`continue` outside loop | `types.md` |
| E0271 | Type mismatch in trait associated type | `traits.md` |
| E0275 | Overflow evaluating trait requirement | `traits.md` |
| E0276 | Requirement in impl stricter than in trait | `traits.md` |
| E0277 | Trait bound not satisfied | `traits.md` |
| E0281 | Closure type mismatch | `traits.md` |
| E0282 | Type annotations needed | `types.md` |
| E0283 | Ambiguous trait implementation | `traits.md` |
| E0284 | Cannot resolve associated type | `traits.md` |
| E0297 | Irrefutable pattern in `for` loop | `patterns.md` |
| E0301 | Mutable borrow in pattern guard | `ownership-borrow.md` |
| E0302 | Assignment in pattern guard | `ownership-borrow.md` |
| E0303 | Pattern binding in expression position | `patterns.md` |
| E0307 | Invalid `self` type in method | `types.md` |
| E0308 | Type mismatch | `types.md` |
| E0309 | Type must outlive lifetime | `lifetimes.md` |
| E0310 | Type must outlive `'static` | `lifetimes.md` |
| E0311 | Lifetime may not outlive required scope | `lifetimes.md` |
| E0312 | Lifetime of reference outlives lifetime of borrowed content | `lifetimes.md` |
| E0316 | `where` clause lifetime not in scope | `lifetimes.md` |
| E0317 | `if` without `else` returns non-unit | `types.md` |
| E0320 | Recursive `Drop` impl | `traits.md` |
| E0321 | Cross-crate opt-out trait impl | `traits.md` |
| E0322 | Cannot implement `Sized` explicitly | `traits.md` |
| E0323 | Associated const mismatch in impl | `traits.md` |
| E0324 | Associated method mismatch in impl | `traits.md` |
| E0325 | Associated type mismatch in impl | `traits.md` |
| E0326 | Associated const type mismatch | `traits.md` |
| E0328 | Cannot implement `Unsize` explicitly | `traits.md` |
| E0364 | Private item re-exported | `modules.md` |
| E0365 | Private item re-exported | `modules.md` |
| E0366 | `derive` on enum with `repr(C)` | `types.md` |
| E0367 | `derive` on union | `types.md` |
| E0368 | Binary op not implemented for type | `traits.md` |
| E0369 | Binary op not implemented for type | `traits.md` |
| E0370 | Enum discriminant overflow | `types.md` |
| E0371 | Implementing a supertrait's method | `traits.md` |
| E0373 | Closure outlives captured variable | `lifetimes.md` |
| E0374 | `CoerceUnsized` impl missing field | `traits.md` |
| E0375 | `CoerceUnsized` impl has multiple fields | `traits.md` |
| E0376 | `CoerceUnsized` impl on non-struct | `traits.md` |
| E0377 | `CoerceUnsized` field not coercible | `traits.md` |
| E0378 | `DispatchFromDyn` impl invalid | `traits.md` |
| E0379 | Trait method cannot be `const` | `traits.md` |
| E0380 | Auto trait cannot have methods/assoc items | `traits.md` |
| E0381 | Use of possibly uninitialized variable | `ownership-borrow.md` |
| E0382 | Use after move | `ownership-borrow.md` |
| E0383 | Use of partially moved value | `ownership-borrow.md` |
| E0384 | Reassignment of immutable variable | `ownership-borrow.md` |
| E0386 | Mutation of immutable field in `&mut` | `ownership-borrow.md` |
| E0387 | Mutation of immutable variable in closure | `ownership-borrow.md` |
| E0388 | Mutation of immutable static | `ownership-borrow.md` |
| E0389 | Mutation through `&` reference | `ownership-borrow.md` |
| E0390 | Inherent impl on primitive type | `traits.md` |
| E0391 | Type dependency cycle | `types.md` |
| E0392 | Type param never used | `generics.md` |
| E0393 | Type param required for impl | `generics.md` |
| E0401 | Use of type param from outer function | `generics.md` |
| E0403 | Two type params with same name | `generics.md` |
| E0404 | Expected trait, found type | `traits.md` |
| E0405 | Trait not in scope | `traits.md` |
| E0407 | Method not a member of trait | `traits.md` |
| E0408 | Variable not bound in all patterns | `patterns.md` |
| E0409 | Variable bound inconsistently across patterns | `patterns.md` |
| E0411 | `Self` used outside impl/trait | `types.md` |
| E0412 | Type not in scope | `types.md` |
| E0415 | Identifier bound more than once in params | `types.md` |
| E0416 | Identifier bound more than once in pattern | `patterns.md` |
| E0422 | Does not name a struct | `types.md` |
| E0423 | Expected function, found struct | `types.md` |
| E0424 | `self` not available in static method | `types.md` |
| E0425 | Unresolved name | `modules.md` |
| E0426 | Undeclared label | `types.md` |
| E0428 | Name defined multiple times | `modules.md` |
| E0429 | `self` import not at end of path | `modules.md` |
| E0430 | `self` import appears more than once | `modules.md` |
| E0431 | `self` import only allowed in `use` list | `modules.md` |
| E0432 | Unresolved import | `modules.md` |
| E0433 | Undeclared crate/module/type | `modules.md` |
| E0434 | `const` cannot refer to `let` binding | `types.md` |
| E0435 | Non-constant value in constant expression | `types.md` |
| E0436 | Functional record update on non-struct | `types.md` |
| E0437 | Unknown associated type in trait impl | `traits.md` |
| E0438 | Unknown associated const in trait impl | `traits.md` |
| E0439 | Invalid `simd_shuffle` index | `types.md` |
| E0445 | Private trait in public interface | `modules.md` |
| E0446 | Private type in public interface | `modules.md` |
| E0447 | Visibility qualifier on trait impl item | `modules.md` |
| E0448 | Visibility qualifier on inherent impl item | `modules.md` |
| E0449 | Unnecessary visibility qualifier | `modules.md` |
| E0451 | Struct fields not visible | `modules.md` |
| E0452 | Malformed lint attribute | `types.md` |
| E0453 | Overriding forbidden lint | `types.md` |
| E0454 | Link name must not be empty | `types.md` |
| E0455 | Linking kind not supported on target | `types.md` |
| E0457 | Plugin not found | `types.md` |
| E0458 | Unknown kind in link attribute | `types.md` |
| E0459 | Link attribute missing name | `types.md` |
| E0460 | Found possibly newer version of crate | `modules.md` |
| E0461 | Couldn't find crate with expected target | `modules.md` |
| E0462 | Found staticlib, expected rlib | `modules.md` |
| E0463 | Can't find crate | `modules.md` |
| E0464 | Multiple matching crates | `modules.md` |
| E0466 | Malformed `#[macro_export]` | `modules.md` |
| E0468 | `extern crate` in non-root module | `modules.md` |
| E0469 | Imported macro not found | `modules.md` |
| E0472 | `asm!` unsupported on target | `types.md` |
| E0476 | Coercion target lifetime not longer than source | `lifetimes.md` |
| E0477 | Lifetime does not outlive required scope | `lifetimes.md` |
| E0478 | Lifetime bound not satisfied | `lifetimes.md` |
| E0482 | Lifetime of return value not constrained | `lifetimes.md` |
| E0491 | Reference has longer lifetime than referent | `lifetimes.md` |
| E0492 | Borrow of static with non-static lifetime | `lifetimes.md` |
| E0493 | Destructor of const/static may not run | `types.md` |
| E0495 | Cannot infer lifetime due to conflicting requirements | `lifetimes.md` |
| E0496 | Lifetime name shadows outer lifetime | `lifetimes.md` |
| E0497 | Lifetime bound on type param outlives type | `lifetimes.md` |
| E0499 | Mutable borrow more than once | `ownership-borrow.md` |
| E0500 | Closure borrows variable already borrowed | `ownership-borrow.md` |
| E0501 | Closure borrows variable used by parent | `ownership-borrow.md` |
| E0502 | Borrow conflicts (mutable + immutable) | `ownership-borrow.md` |
| E0503 | Cannot use value because it is borrowed | `ownership-borrow.md` |
| E0504 | Cannot move value into closure while borrowed | `ownership-borrow.md` |
| E0505 | Move while borrowed | `ownership-borrow.md` |
| E0506 | Assign to borrowed value | `ownership-borrow.md` |
| E0507 | Move out of borrowed content | `ownership-borrow.md` |
| E0508 | Move out of array | `ownership-borrow.md` |
| E0509 | Move out of type implementing `Drop` | `ownership-borrow.md` |
| E0510 | Borrow of moved value | `ownership-borrow.md` |
| E0511 | Invalid monomorphization of intrinsic | `types.md` |
| E0512 | Transmute with different sizes | `types.md` |
| E0514 | Incompatible crate versions | `modules.md` |
| E0515 | Return reference to local variable | `lifetimes.md` |
| E0516 | `typeof` not implemented | `types.md` |
| E0517 | `repr` attribute on wrong type | `types.md` |
| E0518 | `inline` attribute on non-function | `types.md` |
| E0519 | Crate-level attribute not in crate root | `modules.md` |
| E0520 | Cannot specialize default impl | `traits.md` |
| E0521 | Borrowed data escapes outside closure | `lifetimes.md` |
| E0522 | Attribute not allowed on `extern` block | `types.md` |
| E0524 | Two closures require unique access | `ownership-borrow.md` |
| E0525 | Closure implements `FnOnce` but expected `Fn`/`FnMut` | `traits.md` |
| E0527 | Wrong number of elements in array pattern | `patterns.md` |
| E0528 | Wrong number of elements in slice pattern | `patterns.md` |
| E0529 | Array/slice pattern on non-array/slice | `patterns.md` |
| E0530 | Pattern binding shadows something else | `patterns.md` |
| E0531 | Unknown tuple struct/variant in pattern | `patterns.md` |
| E0532 | Expected pattern, found something else | `patterns.md` |
| E0533 | Not a unit struct/variant | `patterns.md` |
| E0534 | `inline` attribute requires argument | `types.md` |
| E0535 | Unknown argument to `inline` | `types.md` |
| E0536 | `not(...)` takes exactly one argument | `types.md` |
| E0537 | Unknown predicate in `cfg` | `types.md` |
| E0538 | Attribute argument used multiple times | `types.md` |
| E0541 | Unknown meta item | `types.md` |
| E0542 | `stable` attribute missing `since` | `types.md` |
| E0543 | `stable` attribute missing `feature` | `types.md` |
| E0544 | Multiple `stable` attributes | `types.md` |
| E0545 | `unstable` attribute missing `issue` | `types.md` |
| E0546 | `unstable` attribute missing `feature` | `types.md` |
| E0547 | Multiple `unstable` attributes | `types.md` |
| E0549 | Deprecated attribute on unsafe impl | `types.md` |
| E0550 | Multiple `deprecated` attributes | `types.md` |
| E0551 | Incorrect meta item | `types.md` |
| E0552 | Unrecognized representation hint | `types.md` |
| E0554 | `#![feature]` in stable release | `types.md` |
| E0556 | Malformed `feature` attribute | `types.md` |
| E0557 | Feature not present in edition | `types.md` |
| E0559 | Unknown field in struct variant | `types.md` |
| E0560 | Unknown field in struct literal | `types.md` |
| E0561 | Patterns not allowed in function pointer type | `patterns.md` |
| E0562 | `impl Trait` in non-return position | `traits.md` |
| E0565 | Literal in `cfg` predicate | `types.md` |
| E0566 | Conflicting representation hints | `types.md` |
| E0567 | Generic params on auto trait | `traits.md` |
| E0568 | Supertrait on auto trait | `traits.md` |
| E0569 | `repr(packed)` on non-ADT | `types.md` |
| E0570 | ABI not supported on target | `types.md` |
| E0571 | `break` with value outside `loop` | `types.md` |
| E0572 | Return outside function | `types.md` |
| E0573 | Expected type, found something else | `types.md` |
| E0574 | Expected struct/enum/union, found other | `types.md` |
| E0575 | Expected associated type, found other | `generics.md` |
| E0576 | Associated item not found in trait | `traits.md` |
| E0577 | Expected module, found other | `modules.md` |
| E0578 | Module not found in scope | `modules.md` |
| E0579 | Lower range bound > upper range bound | `patterns.md` |
| E0580 | `main` must return `()` or `Result` | `types.md` |
| E0581 | Lifetime in function pointer not in argument | `lifetimes.md` |
| E0582 | Lifetime bound not used in type | `lifetimes.md` |
| E0583 | File not found for module | `modules.md` |
| E0584 | Expected item, found doc comment | `types.md` |
| E0585 | Attribute not followed by item | `types.md` |
| E0586 | `..` in tuple struct pattern | `patterns.md` |
| E0587 | `repr(packed)` and `repr(align)` conflict | `types.md` |
| E0588 | `repr(packed)` on type with `repr(align)` field | `types.md` |
| E0589 | Invalid `repr(align)` value | `types.md` |
| E0590 | `break` or `continue` with no label | `types.md` |
| E0591 | Function pointer type mismatch | `types.md` |
| E0592 | Method or function already defined | `types.md` |
| E0593 | Closure argument count mismatch | `traits.md` |
| E0594 | Cannot assign to immutable | `ownership-borrow.md` |
| E0595 | Closure cannot move out of captured variable | `ownership-borrow.md` |
| E0596 | Cannot borrow as mutable (not declared `mut`) | `ownership-borrow.md` |
| E0597 | Value dropped while borrowed | `lifetimes.md` |
| E0599 | Method not found | `types.md` |
| E0600 | Unary op not implemented | `types.md` |
| E0601 | `main` function not found | `types.md` |
| E0602 | Unknown lint | `types.md` |
| E0603 | Private item used | `modules.md` |
| E0604 | Cast to/from non-primitive | `types.md` |
| E0605 | Non-primitive cast | `types.md` |
| E0606 | Casting between incompatible pointer types | `types.md` |
| E0607 | Cast to thin pointer from fat pointer | `types.md` |
| E0608 | Cannot index into value | `types.md` |
| E0609 | No field on type | `types.md` |
| E0610 | Primitive type has no field | `types.md` |
| E0614 | Cannot deref non-pointer | `types.md` |
| E0615 | Attempted to take value of method | `types.md` |
| E0616 | Field is private | `modules.md` |
| E0617 | `va_list` type mismatch | `types.md` |
| E0618 | Expected function, found non-function | `types.md` |
| E0619 | Type annotation needed (deprecated) | `types.md` |
| E0620 | Invalid cast | `types.md` |
| E0621 | Explicit lifetime required | `lifetimes.md` |
| E0622 | Intrinsic must be a function | `types.md` |
| E0623 | Lifetime mismatch in function argument | `lifetimes.md` |
| E0624 | Method not visible | `modules.md` |
| E0625 | Static in pattern | `patterns.md` |
| E0626 | Borrow may still be in use at yield | `async.md` |
| E0627 | Yield outside generator | `async.md` |
| E0628 | Generator has too many parameters | `async.md` |
| E0631 | Closure type mismatch | `traits.md` |
| E0632 | Explicit generic arg after `..` | `generics.md` |
| E0633 | `unwind` attribute invalid | `types.md` |
| E0634 | `rustc_paren_sugar` on non-trait | `types.md` |
| E0635 | Unknown feature | `types.md` |
| E0636 | Feature declared twice | `types.md` |
| E0637 | `'_` not allowed here | `lifetimes.md` |
| E0638 | `..` in struct without `..` support | `types.md` |
| E0639 | Cannot create non-exhaustive struct/enum | `types.md` |
| E0640 | Infer type in const/static | `types.md` |
| E0641 | Cannot cast to/from pointer with unknown size | `types.md` |
| E0642 | Patterns not allowed in closure args | `patterns.md` |
| E0643 | Lifetime in `impl Trait` bound | `lifetimes.md` |
| E0644 | Closure/generator in `extern` fn | `types.md` |
| E0646 | `main` cannot be `unsafe` | `types.md` |
| E0647 | `start` cannot be `unsafe` | `types.md` |
| E0648 | `export_name` cannot start with `_ZN` | `types.md` |
| E0657 | `impl Trait` lifetime not in scope | `lifetimes.md` |
| E0658 | Feature not available in this edition | `types.md` |
| E0659 | Ambiguous name | `modules.md` |
| E0660 | Invalid `asm!` register | `types.md` |
| E0661 | Invalid `asm!` register class | `types.md` |
| E0662 | Invalid `asm!` operand modifier | `types.md` |
| E0663 | Invalid `asm!` operand modifier | `types.md` |
| E0664 | `asm!` clobber_abi not supported | `types.md` |
| E0665 | `Default` derive on enum without `#[default]` | `traits.md` |
| E0666 | Nested `impl Trait` | `traits.md` |
| E0667 | `impl Trait` in projections | `traits.md` |
| E0668 | Malformed `asm!` | `types.md` |
| E0669 | `asm!` output operand constraint | `types.md` |
| E0670 | `async fn` in Rust 2015 | `async.md` |
| E0671 | Const params cannot depend on type params | `generics.md` |
| E0687 | Lifetime in pattern | `lifetimes.md` |
| E0688 | `impl Trait` in `extern` fn | `traits.md` |
| E0689 | Cannot call method on ambiguous numeric type | `types.md` |
| E0690 | Transparent enum has wrong number of fields | `types.md` |
| E0691 | Zero-sized field in transparent struct | `types.md` |
| E0692 | `repr(transparent)` with other repr | `types.md` |
| E0693 | Incorrect `repr(align)` | `types.md` |
| E0695 | `break` without label in labeled block | `types.md` |
| E0696 | `continue` pointing to labeled block | `types.md` |
| E0697 | Static closures not supported | `types.md` |
| E0698 | Type param on `async fn` in trait | `async.md` |
| E0699 | Method called on type with late-bound lifetime | `lifetimes.md` |
| E0700 | Unstable feature used | `types.md` |
| E0701 | Enum variant not found | `types.md` |
| E0703 | Invalid ABI | `types.md` |
| E0704 | Incorrect visibility on trait impl item | `modules.md` |
| E0705 | Feature declared stable before it was unstable | `types.md` |
| E0706 | `async fn` in trait (old error, now supported) | `async.md` |
| E0708 | `async` block in non-async context | `async.md` |
| E0710 | Unknown `#[target_feature]` | `types.md` |
| E0711 | `#[target_feature]` on safe function | `types.md` |
| E0712 | Thread-local with non-static lifetime | `lifetimes.md` |
| E0713 | Borrow outlives drop | `lifetimes.md` |
| E0714 | Marker trait with items | `traits.md` |
| E0715 | Closed-over value does not implement `Sync` | `traits.md` |
| E0716 | Temporary dropped while borrowed | `lifetimes.md` |
| E0717 | `rustc_transparent_union` on non-union | `types.md` |
| E0718 | `lang` attribute on wrong type | `types.md` |
| E0719 | Duplicate `lang` item | `types.md` |
| E0720 | Opaque type expands to recursive type | `types.md` |
| E0722 | Malformed `#[optimize]` | `types.md` |
| E0724 | `#[ffi_returns_twice]` on non-foreign fn | `types.md` |
| E0725 | Feature not allowed in `#[target_feature]` | `types.md` |
| E0726 | Implicit elided lifetime not allowed | `lifetimes.md` |
| E0727 | `static` in `async` generator | `async.md` |
| E0728 | `await` outside `async` | `async.md` |
| E0729 | Pattern not allowed in `async` fn | `async.md` |
| E0730 | Non-constant length in array pattern | `patterns.md` |
| E0731 | Enum with discriminant and `derive(Default)` | `traits.md` |
| E0732 | `#[repr(inttype)]` required for enum with discriminant | `types.md` |
| E0733 | Recursive `async fn` without indirection | `async.md` |
| E0734 | Stability attribute not allowed here | `types.md` |
| E0735 | Stability attribute not allowed here | `types.md` |
| E0736 | `#[track_caller]` on `#[naked]` fn | `types.md` |
| E0737 | `#[track_caller]` requires Rust ABI | `types.md` |
| E0739 | `#[track_caller]` on trait impl method | `types.md` |
| E0740 | Union field with `Drop` | `types.md` |
| E0741 | Invalid lifetime param name | `lifetimes.md` |
| E0742 | Visibility not allowed here | `modules.md` |
| E0743 | C-variadic fn with wrong ABI | `types.md` |
| E0744 | `continue` in `if let` chain | `types.md` |
| E0745 | `extern` block item with body | `types.md` |
| E0746 | Return `impl Trait` with non-`Sized` bound | `traits.md` |
| E0747 | Generic arg order wrong | `generics.md` |
| E0748 | Const param type not allowed | `generics.md` |
| E0749 | Negative impl on non-auto trait | `traits.md` |
| E0750 | Negative impl cannot be default | `traits.md` |
| E0751 | Conflicting impls for same type | `traits.md` |
| E0752 | `main` cannot be `async` | `async.md` |
| E0753 | Doc comment not followed by item | `types.md` |
| E0754 | Malformed `macro_export` | `modules.md` |
| E0755 | `ffi_pure` on non-foreign fn | `types.md` |
| E0756 | `ffi_const` on non-foreign fn | `types.md` |
| E0757 | `ffi_const` and `ffi_pure` together | `types.md` |
| E0758 | Malformed doc comment | `types.md` |
| E0759 | Return type requires `'static` | `lifetimes.md` |
| E0760 | `return` in `async` generator | `async.md` |
| E0761 | Multiple `link_name` attributes | `types.md` |
| E0762 | `link_name` on `#[link_section]` fn | `types.md` |
| E0763 | `link_name` on `#[export_name]` fn | `types.md` |
| E0764 | Mutable ref in `const` | `types.md` |
| E0765 | `"` in `asm!` template | `types.md` |
| E0766 | `asm!` with no output operands | `types.md` |
| E0767 | Static in thread-local initializer | `types.md` |
| E0768 | `#[default]` on non-unit variant | `traits.md` |
| E0769 | Struct pattern on tuple variant | `patterns.md` |
| E0770 | Recursive type alias | `types.md` |
| E0771 | Lifetime in `impl Trait` in `dyn Trait` | `lifetimes.md` |
| E0772 | Expected lifetime, found something else | `lifetimes.md` |
| E0773 | Attempted to define built-in macro | `types.md` |
| E0774 | `#[used]` on non-static | `types.md` |
| E0775 | `#[cmse_nonsecure_entry]` on non-extern fn | `types.md` |
| E0776 | `#[cmse_nonsecure_entry]` on variadic fn | `types.md` |
| E0777 | `#[cmse_nonsecure_entry]` on fn with too many params | `types.md` |
| E0778 | `#[cmse_nonsecure_entry]` on fn with too large return | `types.md` |
| E0779 | `#[cmse_nonsecure_entry]` on fn with non-copy params | `types.md` |
| E0780 | `#[cmse_nonsecure_entry]` on fn with non-copy return | `types.md` |
| E0781 | `#[cmse_nonsecure_entry]` on fn with non-FFI-safe params | `types.md` |
| E0782 | Trait objects must include `dyn` | `traits.md` |
| E0783 | Trait objects without `dyn` (edition) | `traits.md` |
| E0784 | Multiple `#[default]` variants | `traits.md` |
| E0785 | Inherent impl on `dyn` or `impl` type | `traits.md` |
| E0786 | Derive macro produced invalid syntax | `types.md` |
| E0787 | `naked` fn with incompatible attribute | `types.md` |
| E0788 | `naked` fn must use `asm!` | `types.md` |
| E0789 | `naked` fn `asm!` must have `noreturn` | `types.md` |
| E0790 | Cannot call method on trait object | `traits.md` |
| E0791 | `#[rustc_must_implement_one_of]` on non-trait | `traits.md` |
| E0792 | Type alias `impl Trait` with wrong generic params | `generics.md` |
| E0793 | Reference to packed field is unaligned | `types.md` |
| E0794 | Cannot drop `extern` static | `types.md` |
| E0795 | Invalid `#[rustc_legacy_const_generics]` index | `generics.md` |
| E0796 | `static_mut_refs` lint | `ownership-borrow.md` |
| E0797 | `'static` bound on associated type | `lifetimes.md` |
| E0798 | `#[rustc_coinductive]` on non-trait | `traits.md` |
| E0799 | Mutable ref to mutable static | `ownership-borrow.md` |
| E0800 | Mutable ref to mutable static | `ownership-borrow.md` |
| E0801 | Conflicting `abi` in `extern` block | `types.md` |
| E0802 | `feature(precise_capturing)` required | `lifetimes.md` |
| E0803 | `use<>` bound on non-`impl Trait` | `lifetimes.md` |
| E0804 | `use<>` bound with invalid param | `lifetimes.md` |
| E0805 | `use<>` bound with duplicate param | `lifetimes.md` |
| E0806 | `use<>` bound with late-bound param | `lifetimes.md` |

---

## Category Files

| File | What It Covers |
|------|---------------|
| `ownership-borrow.md` | Move, borrow, mutability errors (E0381, E0382, E0384, E0499–E0510, E0594–E0596) |
| `lifetimes.md` | Lifetime annotations, elision, dangling refs (E0106, E0309–E0312, E0491–E0497, E0515, E0597) |
| `traits.md` | Trait bounds, impl conflicts, object safety (E0034, E0038, E0046, E0117, E0119, E0277) |
| `types.md` | Type mismatches, casts, inference (E0308, E0054, E0282, E0391, E0599) |
| `patterns.md` | Match exhaustiveness, binding errors (E0004, E0023–E0030, E0408, E0416) |
| `modules.md` | Imports, visibility, crate resolution (E0432, E0433, E0446, E0603) |
| `generics.md` | Generic params, associated types, const generics (E0107, E0207, E0220–E0232) |
| `async.md` | Async/await, generators, futures (E0626–E0628, E0670, E0706, E0728, E0733, E0752) |
