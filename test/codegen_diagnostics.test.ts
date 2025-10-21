import { expect, test } from "bun:test";

import { expectCompileFailure } from "./helpers";

function buildLocalHeavyProgram(localCount: number): string {
  const declarations = Array.from({ length: localCount }, (_, index) => {
    return `        let v${index}: i32 = ${index};`;
  }).join("\n");

  return `
    fn main() -> i32 {
${declarations}\n        0
    }
  `;
}

test("code generation failures populate diagnostics", async () => {
  const source = buildLocalHeavyProgram(600);
  const failure = await expectCompileFailure(source);
  expect(failure.failure.detail).toBe("code generation failed");
});
