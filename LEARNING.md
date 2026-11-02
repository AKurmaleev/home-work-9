# Homework 9 Learning Notes

## Two sources, two subjects

**Lecture 19, “UNSAFE” (39 pages)** explains the responsibilities behind low-level Rust abstractions. **HW9, “Lab3 Simple arithmetic Lisp” (10 pages)** asks for an arithmetic interpreter. Their proximity does not make unsafe Rust part of the assignment.

The interpreter can use `String`, `Vec`, `HashMap`, enums, borrowing, and `Result` throughout. No raw-pointer operations, FFI, manual allocation, or uninitialized storage are justified here. Learning why unsafe code is difficult is a reason to keep this homework safe, not to transplant lecture demonstrations into it.

[ASSIGNMENT.md](ASSIGNMENT.md) separates homework requirements from project design choices. The package target is `mini_lisp`, edition `2024`, minimum Rust `1.85`, with only the standard library.

## 1. Soundness is a contract, not a successful demonstration

Undefined behavior (UB) is not an ordinary error result or a reliably catchable crash. Once it occurs, language guarantees no longer constrain the program in the usual way.

A safe abstraction containing unsafe internals is **sound** only if no safe client can trigger UB through its safe interface. One successful run, debug assertions, or a careful caller does not establish that property for every permitted safe caller.

An `unsafe` block authorizes particular operations that need additional proof. It does **not** disable the borrow checker, remove type checking, or permit UB. Ordinary ownership and reference rules still apply.

An `unsafe fn` assigns documented safety preconditions to callers. Its implementation must still justify each unsafe operation locally. In edition 2024, `unsafe_op_in_unsafe_fn` warns by default about unsafe operations without an explicit unsafe block; neither a block nor a lint is a proof of correctness.

Reference: [undefined behavior and soundness](https://doc.rust-lang.org/reference/behavior-considered-undefined.html), [unsafe blocks](https://doc.rust-lang.org/reference/expressions/block-expr.html#unsafe-blocks), [edition 2024 unsafe-operation lint](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html).

## 2. Raw pointers carry obligations, not just addresses

Possessing or copying a raw pointer does not prove that dereferencing it is valid. A review must establish:

- **Allocation and provenance:** the pointer has the required relationship to a live allocation; matching numeric addresses are not enough.
- **Range and alignment:** the full accessed range is valid, size calculations do not overflow, and alignment meets the operation's contract.
- **Initialization and validity:** the bytes represent a valid value of the accessed type.
- **Lifetime:** the allocation and its contents remain available for the whole access or resulting borrow.
- **Aliasing:** accesses respect shared-reference restrictions and mutable-reference exclusivity, including valid reborrowing.
- **Concurrency:** conflicting accesses have appropriate synchronization; data races remain UB.

A pointer cast does not establish any of these facts. Creating a reference can itself impose validity requirements before any later explicit dereference.

Pointer `offset` is not arbitrary address arithmetic: for non-zero-sized pointees, its in-bounds contract constrains the traversed range to the originating allocation, allowing a one-past endpoint, and requires the byte offset to fit `isize`. A one-past pointer is not an element to read. A wrapping pointer calculation does not make a later access valid.

Reference: [pointer module and provenance](https://doc.rust-lang.org/std/ptr/index.html), [pointer offset](https://doc.rust-lang.org/std/primitive.pointer.html#method.offset).

### Ownership trap: `ptr::read`

`ptr::read` makes a bitwise copy without clearing the original storage. It is not `Clone`. For a non-`Copy` owner such as `String`, treating both copies as independent owners can cause use-after-free or double destruction.

**Non-runnable ownership illustration:** one allocation → original owner plus copied owner bits → two attempted frees. The missing step is a correct ownership-transfer and cleanup protocol, not an extra pointer cast. Prefer normal moves, borrowing, or an intentional safe clone.

Reference: [`ptr::read` safety and ownership](https://doc.rust-lang.org/std/ptr/fn.read.html).

## 3. Miri is evidence for executions, not universal proof

Miri can detect many forms of UB in supported executions. A clean run establishes only that it did not detect a violation in the cases and configuration exercised; untested paths, inputs, targets, and unsupported behavior remain outside that evidence.

Stacked Borrows and Tree Borrows are evolving operational models used to investigate aliasing. They are not the normative, fully settled Rust memory model. Do not turn a model-specific observation into an unconditional language guarantee.

No Miri run or benchmark result is claimed in these notes.

Reference: [Miri documentation and limitations](https://github.com/rust-lang/miri#readme), [Reference caveats about aliasing](https://doc.rust-lang.org/reference/behavior-considered-undefined.html).

## 4. Initialization, validity, and representation differ

### `MaybeUninit` and partial initialization

`MaybeUninit<T>` can hold storage before a valid `T` exists. It does not track initialization or validate the contents. Extracting a `T` requires proof that its required fields/elements are initialized and valid; padding need not be initialized.

For partial construction, track exactly which values exist. If later construction fails or unwinds, cleanup must drop only initialized values, exactly once, when cleanup is required. Dropping the `MaybeUninit` wrapper does not drop its inner `T`. A leak is generally not UB, but that does not make a leaking constructor correct.

**Non-runnable construction illustration:** initialize slots 0 and 1; slot 2 fails → clean up slots 0 and 1, never interpret the whole buffer as a completed array. For this homework, `Vec::push` supplies ordinary safe ownership and cleanup instead.

Reference: [`MaybeUninit` initialization and cleanup](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html).

### `transmute`: equal size is only one condition

`transmute` reinterprets a value's representation; it does not parse or validate it. Equal size does not establish valid discriminants, reference lifetimes, pointee alignment, ownership, or a suitable ABI. For example, not every byte is a valid `bool`, and zero bytes do not create a valid reference.

Prefer explicit conversions. The safe expression `u32::from_le_bytes([0x78, 0x56, 0x34, 0x12])` yields `0x12345678` with an explicit byte order. Lisp numeric tokens instead need text parsing followed by domain validation, not reinterpretation of their bytes.

Reference: [`transmute` requirements and safe alternatives](https://doc.rust-lang.org/std/mem/fn.transmute.html).

### `repr(C)` is not a file format

`repr(C)` defines specified layout rules for interoperability; it does not supply portable endianness, remove all padding, validate arbitrary bytes, or make every field FFI-safe. It is not a serialization or deserialization mechanism. An enum value still needs a valid discriminant.

Reference: [type layout and C representation](https://doc.rust-lang.org/reference/type-layout.html#the-c-representation).

## 5. Interior mutability and marker types are not escape hatches

`UnsafeCell<T>` permits mutation of its contents through shared access under a correct abstraction. It does not relax mutable-reference exclusivity, manage lifetimes, or prevent data races. It is not `Sync`. Use safe abstractions such as `Cell`, `RefCell`, or synchronization types only when their semantics fit; this single-threaded interpreter needs an ordinary mutable environment borrow.

`PhantomData<T>` is a zero-sized marker that influences properties such as variance, auto traits, and drop checking. It does not allocate a `T`, free a resource, extend a real allocation's lifetime, or automatically grant `Send`/`Sync` to a wrapper. Marker choice must describe actual ownership or borrowing, not hide missing invariants.

Reference: [`UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html), [`PhantomData`](https://doc.rust-lang.org/std/marker/struct.PhantomData.html).

### A conceptual `Vec` is not a production collection

A pointer/length/capacity diagram explains only part of a vector's responsibilities. A real implementation must handle allocation layout, initialized elements, growth failure, overflow, zero-sized types, ownership, destruction, and unwind safety. It must not assume a stable field order for `Vec` itself.

Use the standard `Vec<Expr>` for lists. Understanding its invariants does not justify reimplementing its unsafe internals.

Reference: [`Vec` guarantees](https://doc.rust-lang.org/std/vec/struct.Vec.html#guarantees).

## 6. FFI needs both a memory contract and an unwind contract

**Non-runnable API critique:** a safe `extern "C"` function accepting an arbitrary raw pointer and length, then constructing a slice with `from_raw_parts`, is unsound unless its safe inputs actually establish the slice's requirements. `extern "C"` selects an ABI; it does not make the function unsafe to call from safe Rust.

A null check and a plausible length are insufficient. Slice construction requires a non-null, aligned pointer even for an empty slice, a valid initialized range within one allocation, a bounded byte size, appropriate aliasing, and a justified lifetime. A foreign caller's promise cannot silently become a precondition on an unrestricted safe Rust interface. A real FFI design must expose the unsafe contract or prove validity through controlled safe inputs and ownership.

Unwinding is a separate concern. Rust unwinding that reaches a non-unwinding `extern "C"` boundary aborts rather than providing ordinary recovery. Foreign exceptions crossing an incompatible non-unwinding boundary can cause UB. A suitable unwind ABI, such as `"C-unwind"`, permits the crossing it specifies; it does not repair memory violations.

`catch_unwind` catches unwinding Rust panics, not UB, aborting panics, or arbitrary native faults. Its behavior for foreign exceptions is limited and not a general recovery mechanism. It neither rolls back mutations nor guarantees destructor execution after process abort. Use `Result` for expected interpreter errors.

Reference: [`from_raw_parts` safety](https://doc.rust-lang.org/std/slice/fn.from_raw_parts.html), [FFI unwinding](https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-unwinding), [`catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html).

## 7. Applying safe Rust to the mini Lisp

### From text to a result

```text
source line → lexer → tokens → parser → Expr AST → evaluator + Env → number or error
```

1. **Lexer:** separate parentheses and atom text. Parentheses are delimiters; whitespace separates adjacent atoms. Do not mistake the operator `-` for a negative numeric literal.
2. **Parser:** turn tokens into `Number(f64)`, `Symbol(String)`, or `List(Vec<Expr>)`. Match opening and closing parentheses and consume one complete expression; reject missing closers and trailing expressions.
3. **Evaluator:** match AST variants. Numbers evaluate to values; variable symbols require lookup; lists select supported arithmetic or the `define` special form.
4. **Environment:** retain numeric bindings in `HashMap<String, f64>` across lines. The environment lives outside the loop's per-line parsing state.
5. **REPL:** read input, recognize exit commands, call the stages, display values or readable errors, and continue after expression errors. Input/output failures also need explicit handling.

These are interpreter stages, not a request to execute source code in the host language. Parsing `(+ 1 2)` creates data representing a Lisp expression; evaluation implements only the supported operations.

### Worked example

```text
(* (+ 2 3) 4)
List([Symbol("*"), List([Symbol("+"), Number(2), Number(3)]), Number(4)])
inner addition → 5; outer multiplication → 20
```

The middle line is a conceptual AST display, not literal Rust source. Traversing this tree recursively does not add user-defined recursive functions to the language.

### Ownership and mutable borrowing

Enums and exhaustive `match` keep syntax cases explicit. `String` and `Vec<Expr>` own their contents, so AST data need not outlive a borrowed input buffer accidentally. Evaluation can borrow the AST instead of cloning subtrees.

An `Env` containing numbers can return a copied `f64` from lookup. Do not retain a map-entry borrow while recursively evaluating an expression that may mutate the same map. Use sequential, short borrows of the mutable environment; no raw pointers or interior-mutability workaround is needed.

`define` is special because its target symbol is a name, not an expression to look up. Validate its structure, evaluate the right-hand side, then insert the resulting value. Redefinition is allowed. A failed right-hand side does not itself replace the target, but successful nested definitions already performed are not rolled back.

### Numeric and error contracts

`+` and `*` are variadic with empty identities `0` and `1`; `-` and `/` require exactly two operands. Evaluate operands left to right. Preserve that order rather than regrouping floating-point arithmetic or side effects.

Finite-only arithmetic is a language choice: Rust `f64` itself permits NaN and infinities, and floating-point division by zero is not UB. Reject zero divisors explicitly, including negative zero, and validate numeric inputs and arithmetic results. Binary floating-point still has rounding error; finite does not mean exact decimal arithmetic.

Use `Result` to propagate parse/evaluation failures to the REPL's reporting boundary. Avoid `unwrap`, panic-based validation, and `catch_unwind` as error handling. Expected malformed-input errors should not terminate the session.

Input length, nesting depth, and environment size need bounded handling, including draining an overlong physical line before reading the next expression. Implemented limits are 16 KiB of UTF-8 payload per line, 64 nested lists, and 1,024 variable names created by evaluation. [ASSIGNMENT.md](ASSIGNMENT.md) defines exact counting and recovery rules. Draining bounds retained memory, not how long a blocking stream may take to finish a line.

## 8. What testing can establish

Test tokens, AST structure, operator arities, nested results, variable persistence, redefinition, and malformed input independently. Then test REPL recovery and termination as a complete workflow. Include zero divisors, non-finite results, confirmed resource boundaries, and left-to-right nested mutations that survive a later error.

Tests demonstrate the cases exercised, not universal correctness or measured performance. [REPORT.md](REPORT.md) records executed checks and their scope. No Miri or performance result is claimed.
