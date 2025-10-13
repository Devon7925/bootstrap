const STAGE1_INPUT_PTR = 0;
const INSTR_OFFSET_PTR_OFFSET = 4_096;
const FUNCTIONS_COUNT_PTR_OFFSET = 851_960;
const FUNCTIONS_BASE_OFFSET = 851_968;
const FUNCTION_ENTRY_SIZE = 44;
const STAGE1_MAX_FUNCTIONS = 512;

const DEFAULT_PROGRAM = `// Compute the 10th Fibonacci number.
fn fib(n: i32) -> i32 {
    if n < 2 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

fn main() -> i32 {
    return fib(10);
}`;

const encoder = new TextEncoder();
let downloadUrl = null;

const elements = {
  editor: document.getElementById("source"),
  compileButton: document.getElementById("compile"),
  status: document.getElementById("status"),
  compileOutput: document.getElementById("compile-output"),
  executionOutput: document.getElementById("execution-output"),
  downloadLink: document.getElementById("download-link"),
};

elements.editor.value = DEFAULT_PROGRAM;

const stage2ModulePromise = fetch("compiler.wasm")
  .then((response) => {
    if (!response.ok) {
      throw new Error(`failed to load compiler.wasm: ${response.status} ${response.statusText}`);
    }
    return response.arrayBuffer();
  })
  .then((buffer) => WebAssembly.compile(buffer));

elements.compileButton.addEventListener("click", async () => {
  await compileAndRun();
});

elements.editor.addEventListener("keydown", (event) => {
  if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
    event.preventDefault();
    compileAndRun();
  }
});

async function compileAndRun() {
  const source = elements.editor.value;
  if (!source.trim()) {
    setStatus("Please enter some source code.");
    return;
  }

  disableUi(true);
  clearOutputs();
  setStatus("Loading compiler…");

  try {
    const module = await stage2ModulePromise;
    setStatus("Instantiating compiler…");
    const instance = await WebAssembly.instantiate(module, {});
    const { memory, compile } = instance.exports;
    if (!(memory instanceof WebAssembly.Memory) || typeof compile !== "function") {
      throw new Error("stage2 compiler does not expose the expected exports");
    }

    const wasmBytes = compileWithStage2(memory, compile, source);
    reportCompilationSuccess(wasmBytes);
    await executeModule(wasmBytes);
  } catch (error) {
    reportError(error);
  } finally {
    disableUi(false);
  }
}

function compileWithStage2(memory, compile, source) {
  const reserved = FUNCTIONS_BASE_OFFSET + STAGE1_MAX_FUNCTIONS * FUNCTION_ENTRY_SIZE;
  const buffer = memory.buffer;
  const memorySize = buffer.byteLength;
  if (memorySize <= reserved) {
    throw new Error("stage2 compiler memory layout is smaller than expected");
  }

  const outputPtr = memorySize - reserved;
  const sourceBytes = encoder.encode(source);
  if (sourceBytes.length >= outputPtr) {
    throw new Error("source is too large for the stage2 compiler memory");
  }

  new Uint8Array(buffer, STAGE1_INPUT_PTR, sourceBytes.length).set(sourceBytes);
  const result = compile(STAGE1_INPUT_PTR, sourceBytes.length, outputPtr) | 0;
  if (result <= 0) {
    const failure = describeStage2Failure(memory, outputPtr, result);
    throw new Error(failure);
  }

  const view = new Uint8Array(buffer, outputPtr, result);
  return new Uint8Array(view);
}

function describeStage2Failure(memory, outputPtr, status) {
  const view = new DataView(memory.buffer);
  const functions = safeReadI32(view, outputPtr + FUNCTIONS_COUNT_PTR_OFFSET);
  const instrOffset = safeReadI32(view, outputPtr + INSTR_OFFSET_PTR_OFFSET);
  let compiledFunctions = 0;
  if (functions > 0) {
    for (let index = 0; index < functions; index += 1) {
      const entry = outputPtr + FUNCTIONS_BASE_OFFSET + index * FUNCTION_ENTRY_SIZE;
      const codeLen = safeReadI32(view, entry + 16);
      if (codeLen > 0) {
        compiledFunctions += 1;
      } else {
        break;
      }
    }
  }

  return `stage2 compilation failed (status ${status}, functions=${functions}, instr_offset=${instrOffset}, compiled_functions=${compiledFunctions})`;
}

function safeReadI32(view, offset) {
  if (offset < 0 || offset + 4 > view.byteLength) {
    return -1;
  }
  return view.getInt32(offset, true);
}

async function executeModule(wasmBytes) {
  try {
    const { instance } = await WebAssembly.instantiate(wasmBytes, {});
    const main = instance.exports.main;
    if (typeof main !== "function") {
      elements.executionOutput.textContent = "Compiled module has no exported 'main' function.";
      return;
    }

    const result = main();
    if (typeof result === "bigint") {
      elements.executionOutput.textContent = result.toString();
    } else if (result === undefined) {
      elements.executionOutput.textContent = "main executed successfully (no return value).";
    } else {
      elements.executionOutput.textContent = String(result);
    }
  } catch (error) {
    elements.executionOutput.textContent = `Failed to execute module: ${error.message ?? error}`;
  }
}

function reportCompilationSuccess(wasmBytes) {
  elements.compileOutput.textContent = `Produced ${wasmBytes.length} bytes of WebAssembly.`;
  setStatus("Compilation finished.");
  publishDownload(wasmBytes);
}

function reportError(error) {
  console.error(error);
  const message = error instanceof Error ? error.message : String(error);
  elements.compileOutput.textContent = message;
  elements.executionOutput.textContent = "";
  setStatus("Compilation failed.");
  hideDownload();
}

function publishDownload(wasmBytes) {
  if (downloadUrl) {
    URL.revokeObjectURL(downloadUrl);
    downloadUrl = null;
  }

  const blob = new Blob([wasmBytes], { type: "application/wasm" });
  downloadUrl = URL.createObjectURL(blob);
  elements.downloadLink.href = downloadUrl;
  elements.downloadLink.hidden = false;
}

function hideDownload() {
  if (downloadUrl) {
    URL.revokeObjectURL(downloadUrl);
    downloadUrl = null;
  }
  elements.downloadLink.hidden = true;
}

function setStatus(message) {
  elements.status.textContent = message;
}

function disableUi(disabled) {
  elements.compileButton.disabled = disabled;
  elements.editor.toggleAttribute("readonly", disabled);
}

function clearOutputs() {
  elements.compileOutput.textContent = "";
  elements.executionOutput.textContent = "";
  hideDownload();
}

window.addEventListener("beforeunload", () => {
  if (downloadUrl) {
    URL.revokeObjectURL(downloadUrl);
  }
});
