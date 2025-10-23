<template>
  <div>
    <header class="page-header">
      <h1>Bootstrap Compiler Playground</h1>
      <p class="tagline">
        Compile Bootstrap programs straight in your browser using the stage2 WebAssembly compiler.
      </p>
    </header>

    <main class="layout">
      <section class="editor-panel">
        <label class="editor-label" for="source">Program source</label>
        <textarea
          id="source"
          class="editor"
          spellcheck="false"
          v-model="source"
          :readonly="isCompiling"
          @keydown="handleKeydown"
        ></textarea>
        <div class="actions">
          <button type="button" @click="compileAndRun" :disabled="isCompiling">Compile &amp; run</button>
          <span id="status" role="status">{{ status }}</span>
        </div>
      </section>

      <section class="results-panel">
        <h2>Results</h2>
        <div class="result" id="compile-result">
          <h3>Compilation</h3>
          <pre id="compile-output" class="output">{{ compileOutput }}</pre>
          <h4 class="result-subheading">WebAssembly Text (WAT)</h4>
          <pre id="wat-output" class="output wat-output" aria-label="WebAssembly Text">{{ watOutput }}</pre>
        </div>
        <div class="result" id="execution-result">
          <h3>Execution</h3>
          <pre id="execution-output" class="output">{{ executionOutput }}</pre>
        </div>
        <div class="result" id="download-result">
          <h3>Download</h3>
          <a
            v-if="hasDownload"
            id="download-link"
            class="download-link"
            :href="downloadHref"
            download="program.wasm"
          >
            Download compiled Wasm
          </a>
        </div>
      </section>
    </main>

    <footer class="page-footer">
      <p>
        The playground reuses the same <code>compiler.wasm</code> compiler that powers the CLI.
        Visit the repository for documentation and source code.
      </p>
    </footer>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, ref } from "vue";
import initWabt from "wabt";
import stage2ModuleUrl from "../../compiler.wasm?url";

const STAGE1_INPUT_PTR = 0;
const INSTR_OFFSET_PTR_OFFSET = 4_096;
const FUNCTIONS_COUNT_PTR_OFFSET = 851_960;
const FUNCTIONS_BASE_OFFSET = 851_968;
const FUNCTION_ENTRY_SIZE = 60;
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

const STAGE2_MODULE_URL = stage2ModuleUrl;

const encoder = new TextEncoder();

const source = ref(DEFAULT_PROGRAM);
const status = ref("");
const compileOutput = ref("");
const watOutput = ref("");
const executionOutput = ref("");
const isCompiling = ref(false);
const downloadObjectUrl = ref(null);

const hasDownload = computed(() => Boolean(downloadObjectUrl.value));
const downloadHref = computed(() => downloadObjectUrl.value ?? "");

let wabtInstancePromise = null;
const stage2ModulePromise = fetch(STAGE2_MODULE_URL)
  .then((response) => {
    if (!response.ok) {
      throw new Error(`failed to load compiler.wasm: ${response.status} ${response.statusText}`);
    }
    return response.arrayBuffer();
  })
  .then((buffer) => WebAssembly.compile(buffer));

function setStatus(message) {
  status.value = message;
}

function disableUi(disabled) {
  isCompiling.value = disabled;
}

function hideDownload() {
  if (downloadObjectUrl.value) {
    URL.revokeObjectURL(downloadObjectUrl.value);
    downloadObjectUrl.value = null;
  }
}

function publishDownload(wasmBytes) {
  hideDownload();
  const blob = new Blob([wasmBytes], { type: "application/wasm" });
  downloadObjectUrl.value = URL.createObjectURL(blob);
}

function clearOutputs() {
  compileOutput.value = "";
  watOutput.value = "";
  executionOutput.value = "";
  hideDownload();
}

function reportCompilationSuccess(wasmBytes) {
  compileOutput.value = `Produced ${wasmBytes.length} bytes of WebAssembly.`;
  watOutput.value = "Generating WAT…";
  setStatus("Compilation finished.");
  publishDownload(wasmBytes);
  void renderWat(wasmBytes);
}

function reportError(error) {
  console.error(error);
  const message = error instanceof Error ? error.message : String(error);
  compileOutput.value = message;
  watOutput.value = "";
  executionOutput.value = "";
  setStatus("Compilation failed.");
  hideDownload();
}

async function compileAndRun() {
  if (!source.value.trim()) {
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

    const wasmBytes = compileWithStage2(memory, compile, source.value);
    reportCompilationSuccess(wasmBytes);
    await executeModule(wasmBytes);
  } catch (error) {
    reportError(error);
  } finally {
    disableUi(false);
  }
}

function compileWithStage2(memory, compile, currentSource) {
  const reserved = FUNCTIONS_BASE_OFFSET + STAGE1_MAX_FUNCTIONS * FUNCTION_ENTRY_SIZE;
  const sourceBytes = encoder.encode(currentSource);
  const outputPtr = sourceBytes.length;

  ensureCapacity(memory, outputPtr + reserved + 1);

  let view = new Uint8Array(memory.buffer);
  view.set(sourceBytes, STAGE1_INPUT_PTR);

  const result = compile(STAGE1_INPUT_PTR, sourceBytes.length, outputPtr) | 0;
  view = new Uint8Array(memory.buffer);

  if (result <= 0) {
    const failure = describeStage2Failure(memory, outputPtr, result);
    throw new Error(failure);
  }

  return view.slice(outputPtr, outputPtr + result);
}

function ensureCapacity(memory, required) {
  const pageSize = 65_536;
  const current = memory.buffer.byteLength;
  if (current >= required) {
    return;
  }

  const additional = required - current;
  const pagesNeeded = Math.ceil(additional / pageSize);
  memory.grow(pagesNeeded);
}

function describeStage2Failure(memory, outputPtr, statusCode) {
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

  return `stage2 compilation failed (status ${statusCode}, functions=${functions}, instr_offset=${instrOffset}, compiled_functions=${compiledFunctions})`;
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
    const exportsRecord = instance.exports;
    const main = exportsRecord.main;
    if (typeof main !== "function") {
      const exportedNames = Object.keys(exportsRecord ?? {});
      if (exportedNames.length === 0) {
        executionOutput.value = "Module instantiated successfully (no exports to run).";
      } else {
        executionOutput.value = "Module instantiated successfully (no exported 'main' function).";
      }
      return;
    }

    const result = main();
    if (typeof result === "bigint") {
      executionOutput.value = result.toString();
    } else if (result === undefined) {
      executionOutput.value = "main executed successfully (no return value).";
    } else {
      executionOutput.value = String(result);
    }
  } catch (error) {
    executionOutput.value = `Failed to execute module: ${error.message ?? error}`;
  }
}

async function renderWat(wasmBytes) {
  try {
    const wabt = await loadWabt();
    const bytes = wasmBytes instanceof Uint8Array ? wasmBytes : new Uint8Array(wasmBytes);
    let parsed = null;
    try {
      parsed = wabt.readWasm(bytes, { readDebugNames: true });
    } catch (readError) {
      throw readError instanceof Error ? readError : new Error(String(readError));
    }
    try {
      parsed.generateNames();
      parsed.applyNames();
      const wat = parsed.toText({ foldExprs: false, inlineExport: false });
      watOutput.value = wat;
    } finally {
      if (parsed) {
        parsed.destroy();
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    watOutput.value = `Failed to render WAT: ${message}`;
  }
}

async function loadWabt() {
  if (!wabtInstancePromise) {
    const promise = Promise.resolve(initWabt());
    promise.catch(() => {
      if (wabtInstancePromise === promise) {
        wabtInstancePromise = null;
      }
    });
    wabtInstancePromise = promise;
  }

  const instance = await wabtInstancePromise;
  if (!instance || typeof instance.readWasm !== "function") {
    throw new Error("failed to initialize wabt");
  }
  return instance;
}

function handleKeydown(event) {
  if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
    event.preventDefault();
    void compileAndRun();
  }
}

onBeforeUnmount(() => {
  if (downloadObjectUrl.value) {
    URL.revokeObjectURL(downloadObjectUrl.value);
  }
});
</script>
