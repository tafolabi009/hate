# Hate WASM

WebAssembly build of the Hate programming language for browser execution.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                           Browser                                 │
├──────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌────────────────────────────────────────────┐ │
│  │ JS Bootstrap│  │              WebAssembly                    │ │
│  │   (~80 LOC) │  │  ┌────────────────────────────────────────┐│ │
│  │             │  │  │         Hate Runtime                   ││ │
│  │ - Load WASM │◄─│──│  • VM (register-based)                 ││ │
│  │ - DOM proxy │  │  │  • GC (generational)                   ││ │
│  │ - Promises  │──│─►│  • Compiler                            ││ │
│  │             │  │  │  • Bytecode interpreter                ││ │
│  └─────────────┘  │  └────────────────────────────────────────┘│ │
│                   └────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

**Key Principle**: 90%+ of execution happens in WASM. JavaScript is only the FFI layer.

## Building

### Prerequisites

- Rust 1.70+
- wasm-pack (`curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh`)

### Build

```bash
# Quick build
./build.sh

# Or manually
wasm-pack build --target web --out-dir www/pkg
```

### Serve Locally

```bash
cd www
python3 -m http.server 8080
# Open http://localhost:8080
```

## Usage

### Basic Usage

```html
<script type="module">
    import Hate from './hate.js';
    
    await Hate.init('./pkg/hate_wasm_bg.wasm');
    
    // Run Hate code
    const result = Hate.run(`
        let x = 42
        let y = x * 2
        print(y)
        y
    `);
    console.log('Result:', result);
</script>
```

### Inline Scripts

```html
<!-- Auto-executed when Hate is initialized -->
<script type="text/hate">
    fn factorial(n) {
        match n {
            0 => 1,
            1 => 1,
            _ => n * factorial(n - 1)
        }
    }
    
    print(factorial(10))
</script>
```

### Async Execution

```javascript
import Hate from './hate.js';

await Hate.init();

// For code with async operations
const result = await Hate.runAsync(`
    async fn getData() {
        let response = await fetch("https://api.example.com/data")
        return response
    }
    
    await getData()
`);
```

## API Reference

### Main Module (`hate.js`)

| Function | Description |
|----------|-------------|
| `init(wasmPath?)` | Initialize the Hate VM |
| `run(code)` | Execute Hate code synchronously |
| `runAsync(code)` | Execute Hate code asynchronously |
| `evaluate(expr)` | Evaluate a single expression |
| `setGlobal(name, value)` | Set a global variable |
| `getGlobal(name)` | Get a global variable |
| `loadScript(src)` | Load and execute a .hate file |

### WASM Bridges

#### ConsoleBridge
```javascript
ConsoleBridge.log(message)
ConsoleBridge.warn(message)
ConsoleBridge.error(message)
ConsoleBridge.time(label)
ConsoleBridge.timeEnd(label)
```

#### DomBridge
```javascript
const dom = new DomBridge()
dom.querySelector(selector)
dom.createElement(tag)
dom.getElementById(id)
dom.setInnerHtml(element, html)
dom.addEventListener(element, event, callback)
```

#### FetchBridge
```javascript
await FetchBridge.get(url)
await FetchBridge.post(url, body)
await FetchBridge.fetch(url, options)
```

#### TimerBridge
```javascript
TimerBridge.setTimeout(callback, delay)
TimerBridge.setInterval(callback, delay)
TimerBridge.requestAnimationFrame(callback)
```

#### StorageBridge
```javascript
StorageBridge.localGet(key)
StorageBridge.localSet(key, value)
StorageBridge.sessionGet(key)
StorageBridge.sessionSet(key, value)
```

#### CanvasBridge
```javascript
const canvas = new CanvasBridge("canvasId")
canvas.setFillStyle("red")
canvas.fillRect(0, 0, 100, 100)
canvas.fillCircle(50, 50, 25)
canvas.fillText("Hello", 10, 20)
```

## Size

The optimized WASM binary is approximately:
- **~500KB** uncompressed
- **~150KB** gzipped

## Browser Compatibility

- Chrome 89+
- Firefox 89+
- Safari 15+
- Edge 89+

Requires:
- WebAssembly
- ES Modules
- Async/Await

## Development

### Project Structure

```
crates/hate-wasm/
├── Cargo.toml       # WASM crate config
├── build.sh         # Build script
├── src/
│   ├── lib.rs       # Main HateVM bindings
│   ├── bridge.rs    # Console, Timer, Fetch, Storage bridges
│   ├── dom.rs       # DOM manipulation API
│   └── runtime.rs   # Performance, Canvas, WebSocket bridges
└── www/
    ├── index.html   # Demo page
    ├── hate.js      # Minimal JS bootstrap
    └── pkg/         # Build output
```

### Adding New Bridges

1. Add the bridge struct in the appropriate module
2. Use `#[wasm_bindgen]` to expose methods
3. Enable required `web-sys` features in `Cargo.toml`

```rust
#[wasm_bindgen]
pub struct MyBridge;

#[wasm_bindgen]
impl MyBridge {
    #[wasm_bindgen]
    pub fn my_method() -> Result<JsValue, JsValue> {
        // Implementation
    }
}
```

## License

MIT License - see the main repository for details.
