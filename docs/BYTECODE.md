# Bytecode Format Freeze (v0.1)

This document freezes the `.mub` wire format implemented by `src/bytecode.rs` and consumed by `src/vm.rs`.

## Container

- Magic: 4 bytes, ASCII `MUB1`.
- Version field: none (version is encoded in magic).
- Endianness: little-endian for all fixed-width integers.
- Checksum: none.
- Export table: none.

## Integer Widths

- `u8`: opcodes, arity, capture count, opcode small operands.
- `u32`: counts/lengths/indices/entry function/jump targets.
- `i64`: immediate integer literal payload (`PUSH_INT`).

## String Encoding / Constant Pool

The only constant pool is the string table.

Layout:

1. `u32 nstrings`
2. Repeat `nstrings` times:
   - `u32 byte_len`
   - `byte_len` raw UTF-8 bytes

## Function Table

Layout:

1. `u32 nfuncs`
2. Repeat `nfuncs` times:
   - `u8 arity`
   - `u8 captures`
   - `u32 code_len`
   - `code_len` bytes of instruction stream

## Entry Function

- Final `u32 entry_fn` index into function table.

## Opcode Encoding

Instruction stream is byte-addressed. Each instruction starts with one opcode byte.

- `1  PUSH_INT`       : `i64`
- `2  PUSH_BOOL`      : `u8`
- `3  PUSH_STRING`    : `u32 string_idx`
- `4  PUSH_UNIT`      : no operands
- `5  LOAD_LOCAL`     : `u32 local_idx`
- `6  STORE_LOCAL`    : `u32 local_idx`
- `7  POP`            : no operands
- `8  JUMP`           : `u32 target_ip`
- `9  JUMP_IF_FALSE`  : `u32 target_ip`
- `10 CALL_BUILTIN`   : `u8 builtin_id, u8 argc`
- `11 RETURN`         : no operands
- `12 MK_ADT`         : `u32 tag_string_idx, u8 argc`
- `13 JUMP_IF_TAG`    : `u32 tag_string_idx, u32 target_ip`
- `14 ASSERT_CONST`   : `u32 msg_string_idx`
- `15 ASSERT_DYN`     : no operands
- `16 GET_ADT_FIELD`  : `u8 field_idx`
- `17 CALL_FN`        : `u32 fn_id, u8 argc`
- `18 MK_CLOSURE`     : `u32 fn_id, u8 capture_count`
- `19 CALL_CLOSURE`   : `u8 argc`
- `20 TRAP`           : `u32 msg_string_idx`
- `21 CONTRACT_CONST` : `u32 msg_string_idx`

## Decoder/Validator Contract

`bytecode::decode` is strict and never panics on malformed input.
It validates:

- header correctness
- truncation in all sections
- UTF-8 validity in string table
- section count/length overflow and impossible lengths
- string/function index bounds in instructions
- jump target bounds
- unknown opcodes
- unknown builtin IDs
- trailing bytes

Stable decode error codes:

- `E4101` invalid header
- `E4102` truncated stream
- `E4103` invalid UTF-8
- `E4104` invalid section length/count
- `E4105` invalid index
- `E4106` invalid jump target
- `E4107` unknown opcode
- `E4108` unknown builtin id
- `E4109` trailing bytes
