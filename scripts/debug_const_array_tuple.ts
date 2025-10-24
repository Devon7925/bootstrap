import {
  COMPILER_INPUT_PTR,
  DEFAULT_OUTPUT_STRIDE,
  Stage1CompileFailure,
  instantiateAstCompiler,
  readAstConstantCount,
  readAstConstantEntry,
  readAstArrayTypeCount,
  readAstArrayTypeEntry,
  readAstTupleTypeCount,
  readAstTupleTypeEntry,
  readExpressionCount,
  readExpressionEntry,
  readExpressionType,
  readTypeMetadataDebugInfo,
} from "../test/helpers";

const TYPE_ID_KIND_SHIFT = 24;
const TYPE_ID_KIND_USER_COMPOSITE = 1;
const ARRAY_TYPE_CAPACITY = 256;
const TUPLE_TYPE_CAPACITY = 256;
const TYPE_ID_ARRAY_BASE = TYPE_ID_KIND_USER_COMPOSITE << TYPE_ID_KIND_SHIFT;
const TYPE_ID_TUPLE_BASE = TYPE_ID_ARRAY_BASE + ARRAY_TYPE_CAPACITY;
const TYPE_ID_TUPLE_LIMIT = TYPE_ID_TUPLE_BASE + TUPLE_TYPE_CAPACITY;
const SCRATCH_TYPE_METADATA_DEBUG_CONTEXT_OFFSET = 4_032;
const SCRATCH_TYPE_METADATA_DEBUG_SUBJECT_OFFSET = 4_036;
const SCRATCH_TYPE_METADATA_DEBUG_EXTRA_OFFSET = 4_040;
const SCRATCH_TYPE_METADATA_DEBUG_FAILURE_COUNT_OFFSET = 4_044;
const CONST_ARRAY_DEBUG_EXPR_TYPE_OFFSET = 5_000;
const CONST_ARRAY_DEBUG_ELEMENT_TYPE_OFFSET = 5_004;
const CONST_ARRAY_DEBUG_ENV_COUNT_OFFSET = 5_008;
const CONST_ARRAY_DEBUG_PARAM_COUNT_OFFSET = 5_012;
const CONST_ARRAY_DEBUG_TREAT_AS_TYPE_OFFSET = 5_016;
const TYPE_METADATA_DEBUG_LAST_CONTEXT_OFFSET = 5_020;
const TYPE_METADATA_DEBUG_LAST_SUBJECT_OFFSET = 5_024;
const TYPE_METADATA_DEBUG_LAST_EXTRA_OFFSET = 5_028;
const CONST_EVAL_DEBUG_EXPR_OFFSET = 5_032;
const CONST_EVAL_DEBUG_KIND_OFFSET = 5_036;
const CONST_EVAL_DEBUG_STEP_OFFSET = 5_040;
const CONST_EVAL_DEBUG_LOCAL_OFFSET = 5_044;
const CLONE_DEBUG_LOCAL_INDEX_OFFSET = 5_048;
const CLONE_DEBUG_INIT_TYPE_OFFSET = 5_052;
const CLONE_DEBUG_PUSHED_OFFSET = 5_056;
const BUILTIN_TYPE_NAMES = new Map<number, string>([
  [0, "i32"],
  [1, "bool"],
  [2, "i8"],
  [3, "i16"],
  [4, "i64"],
  [5, "u8"],
  [6, "u16"],
  [7, "u32"],
  [8, "u64"],
  [9, "type"],
]);

const encoder = new TextEncoder();

function describeTypeId(typeId: number): string {
  if (typeId < 0) {
    return `invalid(${typeId})`;
  }
  const builtin = BUILTIN_TYPE_NAMES.get(typeId);
  if (builtin) {
    return builtin;
  }
  if (typeId >= TYPE_ID_ARRAY_BASE && typeId < TYPE_ID_TUPLE_BASE) {
    const index = typeId - TYPE_ID_ARRAY_BASE;
    return `array#${index}`;
  }
  if (typeId >= TYPE_ID_TUPLE_BASE && typeId < TYPE_ID_TUPLE_LIMIT) {
    const index = typeId - TYPE_ID_TUPLE_BASE;
    return `tuple#${index}`;
  }
  return `unknown(${typeId})`;
}

const filters = process.argv.slice(2);

function shouldRunCase(label: string): boolean {
  if (filters.length === 0) {
    return true;
  }
  return filters.some((filter) => label.includes(filter));
}

const cases = [
  {
    label: "tuple-index",
    source: `
const KEY_NAME_CAP: i32 = 4;

const TUP: ([i32; KEY_NAME_CAP], type) = ([42; KEY_NAME_CAP], i32);

fn main() -> i32 {
    TUP.0[0] as i32
}
`,
  },
  {
    label: "working",
    source: `
const KEY_COUNT: i32 = 12;
const KEY_NAME_CAP: i32 = 4;

const fn foo(const COUNT: i32) -> [([i32; KEY_NAME_CAP], type); COUNT] {
    let entries: [([i32; KEY_NAME_CAP], type); COUNT] =
        [([42; KEY_NAME_CAP], i32); COUNT];
    let idx = 0;
    entries
}

const BAR: [([i32; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

fn main() -> i32 {
    42
}
`,
  },
  {
    label: "value-no-mut",
    source: `
const KEY_COUNT: i32 = 12;
const KEY_NAME_CAP: i32 = 4;

const fn foo(const COUNT: i32) -> [([i32; KEY_NAME_CAP], type); COUNT] {
    let entries: [([i32; KEY_NAME_CAP], type); COUNT] =
        [([42; KEY_NAME_CAP], i32); COUNT];
    let idx = 0;
    entries
}

const BAR: [([i32; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

fn main() -> i32 {
    BAR[0].0[0] as i32
}
`,
  },
  {
    label: "direct-array",
    source: `
const KEY_COUNT: i32 = 12;
const KEY_NAME_CAP: i32 = 4;

const BAR: [([i32; KEY_NAME_CAP], type); KEY_COUNT] = [([42; KEY_NAME_CAP], i32); KEY_COUNT];

fn main() -> i32 {
    BAR[0].0[0] as i32
}
`,
  },
  {
    label: "call-direct",
    source: `
const KEY_COUNT: i32 = 12;
const KEY_NAME_CAP: i32 = 4;

const fn foo(const COUNT: i32) -> [([i32; KEY_NAME_CAP], type); COUNT] {
    [([42; KEY_NAME_CAP], i32); COUNT]
}

const BAR: [([i32; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

fn main() -> i32 {
    BAR[0].0[0] as i32
}
`,
  },
  {
    label: "call-with-local",
    source: `
const KEY_COUNT: i32 = 12;
const KEY_NAME_CAP: i32 = 4;

const fn foo(const COUNT: i32) -> [([i32; KEY_NAME_CAP], type); COUNT] {
    let entries: [([i32; KEY_NAME_CAP], type); COUNT] =
        [([42; KEY_NAME_CAP], i32); COUNT];
    entries
}

const BAR: [([i32; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

fn main() -> i32 {
    BAR[0].0[0] as i32
}
`,
  },
  {
    label: "call-with-idx",
    source: `
const KEY_COUNT: i32 = 12;
const KEY_NAME_CAP: i32 = 4;

const fn foo(const COUNT: i32) -> [([i32; KEY_NAME_CAP], type); COUNT] {
    let entries: [([i32; KEY_NAME_CAP], type); COUNT] =
        [([42; KEY_NAME_CAP], i32); COUNT];
    let idx = 0;
    entries
}

const BAR: [([i32; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

fn main() -> i32 {
    BAR[0].0[0] as i32
}
`,
  },
  {
    label: "failing",
    source: `
const KEY_COUNT: i32 = 12;
const KEY_NAME_CAP: i32 = 4;

const fn foo(const COUNT: i32) -> [([i32; KEY_NAME_CAP], type); COUNT] {
    let mut entries: [([i32; KEY_NAME_CAP], type); COUNT] =
        [([42; KEY_NAME_CAP], i32); COUNT];
    let mut idx = 0;
    entries
}

const BAR: [([i32; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

fn main() -> i32 {
    BAR[0].0[0] as i32
}
`,
  },
];

for (const { label, source } of cases) {
  if (!shouldRunCase(label)) {
    continue;
  }
  const compiler = await instantiateAstCompiler();
  const inputPtr = COMPILER_INPUT_PTR;
  const outputPtr = DEFAULT_OUTPUT_STRIDE;
  const inputLen = encoder.encode(source).length;
  console.log(`\n=== ${label} case ===`);
  try {
    const wasm = compiler.compileWithLayout(inputPtr, outputPtr, source);
    console.log("compiled successfully", wasm.length);
    const constantCount = readAstConstantCount(compiler.memory, outputPtr, inputLen);
    console.log("success constant count", constantCount);
    const arrayCount = readAstArrayTypeCount(compiler.memory, outputPtr, inputLen);
    console.log("success array type count", arrayCount);
    for (let index = 0; index < arrayCount; index += 1) {
      const entry = readAstArrayTypeEntry(compiler.memory, outputPtr, inputLen, index);
      console.log(
        `array[${index}] element=${entry.elementType} (${describeTypeId(entry.elementType)}) length=${entry.length} cache=${entry.cachedTypeId}`,
      );
    }
    const tupleCount = readAstTupleTypeCount(compiler.memory, outputPtr, inputLen);
    console.log("success tuple type count", tupleCount);
    for (let index = 0; index < tupleCount; index += 1) {
      const entry = readAstTupleTypeEntry(compiler.memory, outputPtr, inputLen, index);
      console.log(
        `tuple[${index}] elements=${entry.elements.map(describeTypeId).join(", ")} cache=${entry.cachedTypeId}`,
      );
    }
    const exprCount = readExpressionCount(compiler.memory, outputPtr, inputLen);
    console.log("success expr count", exprCount);
    const maxExpr = Math.min(exprCount, 16);
    for (let index = 0; index < maxExpr; index += 1) {
      const entry = readExpressionEntry(compiler.memory, outputPtr, inputLen, index);
      const exprType = readExpressionType(compiler.memory, outputPtr, inputLen, index);
      console.log(
        `expr[${index}] kind=${entry.kind} data0=${entry.data0} data1=${entry.data1} data2=${entry.data2} type=${exprType} (${describeTypeId(exprType)})`,
      );
    }
    const view = new DataView(compiler.memory.buffer);
    console.log(
      "success clone let debug",
      view.getInt32(CLONE_DEBUG_LOCAL_INDEX_OFFSET, true),
      view.getInt32(CLONE_DEBUG_INIT_TYPE_OFFSET, true),
      view.getInt32(CLONE_DEBUG_PUSHED_OFFSET, true),
    );
  } catch (error) {
    if (!(error instanceof Stage1CompileFailure)) {
      throw error;
    }
    console.log("stage1 failure", error.failure);
    const scratchCount = compiler.readScratchTypesCount(outputPtr);
    console.log("scratch type count", scratchCount);
    for (let index = 0; index < scratchCount; index += 1) {
      const entry = compiler.readScratchTypeEntry(outputPtr, index);
      if (entry.typeId < 0) {
        continue;
      }
      console.log(
        `scratch[${index}] type=${entry.typeId} (${describeTypeId(entry.typeId)}) extra=${entry.extra}`,
      );
    }
    const arrayCount = readAstArrayTypeCount(compiler.memory, outputPtr, inputLen);
    console.log("array type count", arrayCount);
    for (let index = 0; index < arrayCount; index += 1) {
      const entry = readAstArrayTypeEntry(compiler.memory, outputPtr, inputLen, index);
      console.log(
        `array[${index}] element=${entry.elementType} (${describeTypeId(entry.elementType)}) length=${entry.length} cache=${entry.cachedTypeId}`,
      );
    }
    const tupleCount = readAstTupleTypeCount(compiler.memory, outputPtr, inputLen);
    console.log("tuple type count", tupleCount);
    for (let index = 0; index < tupleCount; index += 1) {
      const entry = readAstTupleTypeEntry(compiler.memory, outputPtr, inputLen, index);
      console.log(
        `tuple[${index}] elements=${entry.elements.map(describeTypeId).join(", ")} cache=${entry.cachedTypeId}`,
      );
    }
    const constantCount = readAstConstantCount(compiler.memory, outputPtr, inputLen);
    console.log("constant count", constantCount);
    for (let index = 0; index < constantCount; index += 1) {
      const entry = readAstConstantEntry(compiler.memory, outputPtr, inputLen, index);
      console.log(
        `const[${index}] name_offset=${entry.nameStart} len=${entry.nameLength} type=${entry.type} (${describeTypeId(entry.type)}) value=${entry.value} expr=${entry.exprIndex}`,
      );
    }
    const exprCount = readExpressionCount(compiler.memory, outputPtr, inputLen);
    console.log("expr count", exprCount);
    const maxExpr = Math.min(exprCount, 32);
    for (let index = 0; index < maxExpr; index += 1) {
      const entry = readExpressionEntry(compiler.memory, outputPtr, inputLen, index);
      const exprType = readExpressionType(compiler.memory, outputPtr, inputLen, index);
      console.log(
        `expr[${index}] kind=${entry.kind} data0=${entry.data0} data1=${entry.data1} data2=${entry.data2} type=${exprType} (${describeTypeId(exprType)})`,
      );
    }
    const debugInfo = readTypeMetadataDebugInfo(compiler.memory, outputPtr);
    console.log("type metadata debug", debugInfo);
    const view = new DataView(compiler.memory.buffer);
    console.log(
      "type metadata raw",
      view.getInt32(outputPtr + 4_032, true),
      view.getInt32(outputPtr + 4_036, true),
      view.getInt32(outputPtr + 4_040, true),
    );
    console.log(
      "type metadata fallback",
      view.getInt32(SCRATCH_TYPE_METADATA_DEBUG_CONTEXT_OFFSET, true),
      view.getInt32(SCRATCH_TYPE_METADATA_DEBUG_SUBJECT_OFFSET, true),
      view.getInt32(SCRATCH_TYPE_METADATA_DEBUG_EXTRA_OFFSET, true),
    );
    console.log(
      "type metadata failure count",
      view.getInt32(outputPtr + SCRATCH_TYPE_METADATA_DEBUG_FAILURE_COUNT_OFFSET, true),
      view.getInt32(SCRATCH_TYPE_METADATA_DEBUG_FAILURE_COUNT_OFFSET, true),
    );
    console.log(
      "type metadata last",
      view.getInt32(outputPtr + TYPE_METADATA_DEBUG_LAST_CONTEXT_OFFSET, true),
      view.getInt32(outputPtr + TYPE_METADATA_DEBUG_LAST_SUBJECT_OFFSET, true),
      view.getInt32(outputPtr + TYPE_METADATA_DEBUG_LAST_EXTRA_OFFSET, true),
    );
    console.log(
      "const eval last",
      view.getInt32(CONST_EVAL_DEBUG_EXPR_OFFSET, true),
      view.getInt32(CONST_EVAL_DEBUG_KIND_OFFSET, true),
      view.getInt32(CONST_EVAL_DEBUG_STEP_OFFSET, true),
      view.getInt32(CONST_EVAL_DEBUG_LOCAL_OFFSET, true),
    );
    console.log(
      "clone let debug",
      view.getInt32(CLONE_DEBUG_LOCAL_INDEX_OFFSET, true),
      view.getInt32(CLONE_DEBUG_INIT_TYPE_OFFSET, true),
      view.getInt32(CLONE_DEBUG_PUSHED_OFFSET, true),
    );
    console.log(
      "array repeat debug",
      view.getInt32(CONST_ARRAY_DEBUG_EXPR_TYPE_OFFSET, true),
      describeTypeId(view.getInt32(CONST_ARRAY_DEBUG_EXPR_TYPE_OFFSET, true)),
      view.getInt32(CONST_ARRAY_DEBUG_ELEMENT_TYPE_OFFSET, true),
      describeTypeId(view.getInt32(CONST_ARRAY_DEBUG_ELEMENT_TYPE_OFFSET, true)),
      view.getInt32(CONST_ARRAY_DEBUG_ENV_COUNT_OFFSET, true),
      view.getInt32(CONST_ARRAY_DEBUG_PARAM_COUNT_OFFSET, true),
      view.getInt32(CONST_ARRAY_DEBUG_TREAT_AS_TYPE_OFFSET, true),
    );
  }
}
