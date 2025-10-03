use std::fs;
use std::sync::OnceLock;

use bootstrap::compile;

const STAGE1_SOURCE_PATH: &str = "compiler/stage1.bp";

static STAGE1_SOURCE: OnceLock<String> = OnceLock::new();
static STAGE1_WASM: OnceLock<Vec<u8>> = OnceLock::new();

pub fn stage1_source() -> &'static str {
    STAGE1_SOURCE
        .get_or_init(|| {
            fs::read_to_string(STAGE1_SOURCE_PATH)
                .unwrap_or_else(|err| panic!("failed to load stage1 source: {err}"))
        })
        .as_str()
}

pub fn stage1_wasm() -> &'static [u8] {
    STAGE1_WASM
        .get_or_init(|| {
            compile(stage1_source())
                .and_then(|compilation| compilation.into_wasm())
                .unwrap_or_else(|err| panic!("failed to compile stage1 source: {err}"))
        })
        .as_slice()
}
