import { compileWithAstCompiler } from "../test/helpers";

async function main() {
  try {
    await compileWithAstCompiler(`
      fn map_pair(const F: fn(i32) -> i32, lhs: i32, rhs: i32) -> (i32, i32) {
          (F(lhs), F(rhs))
      }

      fn main() -> i32 {
          let pair = map_pair(fn(x: i32) -> i32 { x + x }, 4, 7);
          pair.0 + pair.1
      }
    `);
  } catch (error) {
    console.error(error);
    if (error && typeof error === "object" && "cause" in error) {
      console.error("cause:", (error as any).cause);
    }
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
