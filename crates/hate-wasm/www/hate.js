/**
 * Hate Language - Minimal JavaScript Bridge
 * 
 * This is the minimal JS bootstrap (~80 lines) for loading and running
 * Hate code in the browser via WebAssembly. 90%+ of execution happens
 * in WASM - this is just the FFI layer.
 */

// Global Hate runtime instance
let hateVM = null;
let wasmModule = null;

/**
 * Initialize Hate runtime from WASM module
 * @param {string} wasmPath - Path to the .wasm file
 * @returns {Promise<HateVM>} - The initialized Hate VM
 */
export async function init(wasmPath = './pkg/hate_wasm_bg.wasm') {
    if (hateVM) return hateVM;
    
    // Dynamic import of wasm-bindgen generated module
    wasmModule = await import('./pkg/hate_wasm.js');
    await wasmModule.default(wasmPath);
    
    hateVM = new wasmModule.HateVM();
    return hateVM;
}

/**
 * Run Hate code and get result
 * @param {string} code - Hate source code
 * @returns {any} - Result of execution
 */
export function run(code) {
    if (!hateVM) throw new Error('Hate VM not initialized. Call init() first.');
    return hateVM.run(code);
}

/**
 * Run Hate code asynchronously
 * @param {string} code - Hate source code
 * @returns {Promise<any>} - Promise resolving to result
 */
export async function runAsync(code) {
    if (!hateVM) throw new Error('Hate VM not initialized. Call init() first.');
    return hateVM.runAsync(code);
}

/**
 * Evaluate Hate expression
 * @param {string} expr - Hate expression
 * @returns {any} - Result of evaluation
 */
export function evaluate(expr) {
    if (!hateVM) throw new Error('Hate VM not initialized. Call init() first.');
    return hateVM.eval(expr);
}

/**
 * Set global variable in Hate runtime
 * @param {string} name - Variable name
 * @param {any} value - Value to set
 */
export function setGlobal(name, value) {
    if (!hateVM) throw new Error('Hate VM not initialized. Call init() first.');
    hateVM.setGlobal(name, value);
}

/**
 * Get global variable from Hate runtime
 * @param {string} name - Variable name
 * @returns {any} - Variable value
 */
export function getGlobal(name) {
    if (!hateVM) throw new Error('Hate VM not initialized. Call init() first.');
    return hateVM.getGlobal(name);
}

/**
 * Create a script element that runs Hate code
 * @param {string} src - Path to .hate file
 */
export async function loadScript(src) {
    const response = await fetch(src);
    const code = await response.text();
    return run(code);
}

// Auto-initialize when script is loaded as module
if (typeof document !== 'undefined') {
    document.addEventListener('DOMContentLoaded', async () => {
        // Find and execute inline <script type="text/hate"> tags
        const scripts = document.querySelectorAll('script[type="text/hate"]');
        if (scripts.length > 0) {
            await init();
            for (const script of scripts) {
                if (script.src) {
                    await loadScript(script.src);
                } else {
                    run(script.textContent);
                }
            }
        }
    });
}

// Export bridges for direct access
export { wasmModule };
export default { init, run, runAsync, evaluate, setGlobal, getGlobal, loadScript };
