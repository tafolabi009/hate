# Running Hate in the Browser

Hate can be compiled to WebAssembly (WASM) to run in web browsers!

## Current Status

**Not yet implemented** - Browser support is on the roadmap.

## How It Will Work

When completed, you'll be able to:

1. **Compile Hate to WASM**:
```bash
cargo build --target wasm32-unknown-unknown --release
```

2. **Use wasm-bindgen** to generate JavaScript bindings

3. **Run in any browser** with the Hate playground

## Roadmap for Browser Support

### Phase 1: Core WASM Build
- [ ] Add `wasm32-unknown-unknown` target support
- [ ] Replace `std::io` with wasm-compatible alternatives
- [ ] Create `hate-wasm` crate for browser bindings

### Phase 2: JavaScript Bridge
- [ ] Use `wasm-bindgen` for JS interop
- [ ] Implement `console.log` for `println`
- [ ] Add async/promise support for I/O

### Phase 3: Playground
- [ ] Create web-based editor with Monaco
- [ ] Syntax highlighting for `.hate` files
- [ ] Live execution with output panel

## Contributing

Want to help add browser support? See issues labeled `wasm` on GitHub.

## Example Future API

```javascript
import init, { Hate } from 'hate-wasm';

async function main() {
    await init();
    
    const hate = new Hate();
    const result = await hate.run(`
        let x = 10;
        let y = 20;
        println(x + y);
    `);
    
    console.log(result.output);  // "30\n"
}
```

## Why WASM?

- **Portable**: Runs in any modern browser
- **Fast**: Near-native performance
- **Safe**: Sandboxed execution
- **Universal**: Works on desktop, mobile, server
