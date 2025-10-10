export function stubMemoryIntrinsicFunctions(source: string): string {
  const replacements: Array<{ pattern: RegExp; replacement: string }> = [
    {
      pattern: /fn load_u8\(ptr: i32\) -> i32 \{[\s\S]*?\n\}/,
      replacement: "fn load_u8(ptr: i32) -> i32 {\n        load_u8(ptr)\n    }",
    },
    {
      pattern: /fn load_u16\(ptr: i32\) -> i32 \{[\s\S]*?\n\}/,
      replacement: "fn load_u16(ptr: i32) -> i32 {\n        load_u16(ptr)\n    }",
    },
    {
      pattern: /fn load_i32\(ptr: i32\) -> i32 \{[\s\S]*?\n\}/,
      replacement: "fn load_i32(ptr: i32) -> i32 {\n        load_i32(ptr)\n    }",
    },
    {
      pattern: /fn store_u8\(ptr: i32, value: i32\) -> i32 \{[\s\S]*?\n\}/,
      replacement: "fn store_u8(ptr: i32, value: i32) -> i32 {\n        store_u8(ptr, value)\n    }",
    },
    {
      pattern: /fn store_u16\(ptr: i32, value: i32\) -> i32 \{[\s\S]*?\n\}/,
      replacement: "fn store_u16(ptr: i32, value: i32) -> i32 {\n        store_u16(ptr, value)\n    }",
    },
    {
      pattern: /fn store_i32\(ptr: i32, value: i32\) -> i32 \{[\s\S]*?\n\}/,
      replacement: "fn store_i32(ptr: i32, value: i32) -> i32 {\n        store_i32(ptr, value)\n    }",
    },
  ];

  return replacements.reduce((current, { pattern, replacement }) => current.replace(pattern, replacement), source);
}

const MEMORY_INTRINSIC_SIGNATURES = [
  "fn load_u8(ptr: i32) -> i32",
  "fn load_u16(ptr: i32) -> i32",
  "fn load_i32(ptr: i32) -> i32",
  "fn store_u8(ptr: i32, value: i32) -> i32",
  "fn store_u16(ptr: i32, value: i32) -> i32",
  "fn store_i32(ptr: i32, value: i32) -> i32",
];

export function sourceContainsInlineMemoryIntrinsics(source: string): boolean {
  return MEMORY_INTRINSIC_SIGNATURES.every((signature) => source.includes(signature));
}
