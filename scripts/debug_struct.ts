import {
  CompilerInstance,
  COMPILER_INPUT_PTR,
  DEFAULT_OUTPUT_STRIDE,
  Stage1CompileFailure,
  instantiateAstCompiler,
} from "../test/helpers";

const staticStructSource = `
const Pair = struct(6, 2, [
    ("first\\0", i32),
    ("second", i32),
]);

fn main() -> i32 {
    let pair: Pair = Pair {
        first: 1,
        second: 2,
    };
    pair.first + pair.second + pair["second"]
}
`;

const dynamicStructSource = `
const fn digits(num: i32) -> i32 {
    let mut digits = 0;
    while num > 0 {
        num = num / 10;
        digits = digits + 1;
    }
    digits
}

const fn dynamic_struct(const KEY_COUNT: i32) -> type {
    let mut entries: [([u8; digits(KEY_COUNT) + 1], type); KEY_COUNT] =
        [([0; digits(KEY_COUNT) + 1], i32); KEY_COUNT];
    let mut idx = 0;
    while idx < KEY_COUNT {
        entries[idx].0[0] = 'k';
        let mut digit = digits(KEY_COUNT) - 1;
        while digit >= 0 {
            let mut digit_val = idx;
            let mut digit_idx = 0;
            while digit_idx <= digit {
                digit_val = digit_val / 10;
                digit_idx = digit_idx + 1;
            }
            digit_val = digit_val - digit_val / 10 * 10;
            entries[idx].0[digit + 1] = 48 + digit_val;
            digit = digit - 1;
        }
        idx = idx + 1;
    }
    struct(digits(KEY_COUNT) + 1, KEY_COUNT, entries)
}

const ElevenKeys = dynamic_struct(11);

fn main() -> i32 {
    let set: ElevenKeys = ElevenKeys {
        k0: 0,
        k1: 1,
        k2: 2,
        k3: 3,
        k4: 4,
        k5: 5,
        k6: 6,
        k7: 7,
        k8: 8,
        k9: 9,
        k10: 10,
        k11: 11,
    };
    set.k1 + set.k11 + 3 * set.k10
}
`;

const compiler = await instantiateAstCompiler();

async function compileSource(label: string, source: string) {
  try {
    const wasm = compiler.compileWithLayout(COMPILER_INPUT_PTR, DEFAULT_OUTPUT_STRIDE, source);
    console.log(`${label}: compiled`, wasm.length);
  } catch (error) {
    if (error instanceof Stage1CompileFailure) {
      console.log(`${label}: stage1 failure`, error.failure);
      const memory = compiler.memory;
      const view = new DataView(memory.buffer);
      const outPtr = DEFAULT_OUTPUT_STRIDE;
      const header = Array.from(new Uint8Array(memory.buffer, outPtr, 16));
      console.log(`${label}: output header`, header);
      const typesCount = compiler.readTypesCount(outPtr);
      console.log(`${label}: types count`, typesCount);
      const functions = view.getInt32(outPtr + 819_188, true);
      console.log(`${label}: reported functions`, functions);
    } else {
      throw error;
    }
  }
}

await compileSource("static", staticStructSource);
await compileSource("dynamic", dynamicStructSource);
