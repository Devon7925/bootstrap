import { compileWithAstCompiler } from "../test/helpers";

const source = `
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

const wasm = await compileWithAstCompiler(source);
console.log("wasm bytes", wasm.length);
await Bun.write("/tmp/struct.wasm", wasm);

function readLeb(bytes: Uint8Array, index: number): [number, number] {
    let result = 0;
    let shift = 0;
    let offset = index;
    while (offset < bytes.length) {
        const byte = bytes[offset];
        result |= (byte & 0x7f) << shift;
        offset += 1;
        if ((byte & 0x80) === 0) {
            break;
        }
        shift += 7;
    }
    return [result, offset - index];
}

let ptr = 8; // skip magic and version
while (ptr < wasm.length) {
    const id = wasm[ptr++];
    const [sectionSize, sizeLen] = readLeb(wasm, ptr);
    ptr += sizeLen;
    if (id === 10) {
        // code section
        console.log("code section size", sectionSize);
        const start = ptr;
        const [funcCount, funcLen] = readLeb(wasm, ptr);
        ptr += funcLen;
        console.log("function count", funcCount);
        for (let i = 0; i < funcCount; i++) {
            const [bodySize, bodyLen] = readLeb(wasm, ptr);
            ptr += bodyLen;
            console.log(`body ${i} declared size`, bodySize);
            ptr += bodySize;
        }
        console.log("actual code section bytes", ptr - start);
        break;
    } else {
        ptr += sectionSize;
    }
}
