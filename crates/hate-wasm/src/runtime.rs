//! WASM Runtime
//!
//! Provides the runtime support for executing Hate code in WebAssembly.
//! This includes built-in functions that bridge to browser APIs.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use js_sys::Function;
use web_sys::console;

/// Performance measurement utilities
#[wasm_bindgen]
pub struct PerfBridge;

#[wasm_bindgen]
impl PerfBridge {
    /// Get performance.now() in milliseconds
    #[wasm_bindgen]
    pub fn now() -> f64 {
        if let Some(window) = web_sys::window() {
            if let Some(perf) = window.performance() {
                return perf.now();
            }
        }
        0.0
    }
    
    /// Measure execution time of a callback
    #[wasm_bindgen]
    pub fn measure(label: &str, callback: &Function) -> Result<f64, JsValue> {
        let start = Self::now();
        callback.call0(&JsValue::NULL)?;
        let end = Self::now();
        let duration = end - start;
        console::log_1(&format!("{}: {:.2}ms", label, duration).into());
        Ok(duration)
    }
    
    /// Create a performance mark
    #[wasm_bindgen]
    pub fn mark(name: &str) -> Result<(), JsValue> {
        if let Some(window) = web_sys::window() {
            if let Some(perf) = window.performance() {
                perf.mark(name)?;
            }
        }
        Ok(())
    }
    
    /// Clear performance marks
    #[wasm_bindgen(js_name = clearMarks)]
    pub fn clear_marks() -> Result<(), JsValue> {
        if let Some(window) = web_sys::window() {
            if let Some(perf) = window.performance() {
                perf.clear_marks();
            }
        }
        Ok(())
    }
}

/// Clipboard API
#[wasm_bindgen]
pub struct ClipboardBridge;

#[wasm_bindgen]
impl ClipboardBridge {
    /// Write text to clipboard
    #[wasm_bindgen(js_name = writeText)]
    pub async fn write_text(text: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let navigator = window.navigator();
        let clipboard = navigator.clipboard();
        wasm_bindgen_futures::JsFuture::from(clipboard.write_text(text)).await?;
        Ok(())
    }
    
    /// Read text from clipboard
    #[wasm_bindgen(js_name = readText)]
    pub async fn read_text() -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let navigator = window.navigator();
        let clipboard = navigator.clipboard();
        let result = wasm_bindgen_futures::JsFuture::from(clipboard.read_text()).await?;
        Ok(result.as_string().unwrap_or_default())
    }
}

/// Random number generation (crypto-secure)
#[wasm_bindgen]
pub struct CryptoBridge;

#[wasm_bindgen]
impl CryptoBridge {
    /// Get crypto-secure random bytes
    #[wasm_bindgen(js_name = getRandomBytes)]
    pub fn get_random_bytes(length: usize) -> Result<Vec<u8>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let crypto = window.crypto()?;
        let mut buffer = vec![0u8; length];
        crypto.get_random_values_with_u8_array(&mut buffer)?;
        Ok(buffer)
    }
    
    /// Get random UUID
    #[wasm_bindgen(js_name = randomUuid)]
    pub fn random_uuid() -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let crypto = window.crypto()?;
        Ok(crypto.random_uuid())
    }
}

/// Alert/Prompt/Confirm dialogs
#[wasm_bindgen]
pub struct DialogBridge;

#[wasm_bindgen]
impl DialogBridge {
    /// Show alert dialog
    #[wasm_bindgen]
    pub fn alert(message: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        window.alert_with_message(message)
    }
    
    /// Show confirm dialog
    #[wasm_bindgen]
    pub fn confirm(message: &str) -> Result<bool, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        window.confirm_with_message(message)
    }
    
    /// Show prompt dialog
    #[wasm_bindgen]
    pub fn prompt(message: &str, default: Option<String>) -> Result<Option<String>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        match default {
            Some(d) => window.prompt_with_message_and_default(message, &d),
            None => window.prompt_with_message(message),
        }
    }
}

/// Canvas drawing utilities
#[wasm_bindgen]
pub struct CanvasBridge {
    context: web_sys::CanvasRenderingContext2d,
}

#[wasm_bindgen]
impl CanvasBridge {
    /// Create canvas bridge from element ID
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Result<CanvasBridge, JsValue> {
        let document = web_sys::window()
            .ok_or("No window")?
            .document()
            .ok_or("No document")?;
        
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?
            .dyn_into::<web_sys::HtmlCanvasElement>()?;
        
        let context = canvas
            .get_context("2d")?
            .ok_or("No 2D context")?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
        
        Ok(CanvasBridge { context })
    }
    
    /// Clear canvas
    #[wasm_bindgen]
    pub fn clear(&self, width: f64, height: f64) {
        self.context.clear_rect(0.0, 0.0, width, height);
    }
    
    /// Set fill style
    #[wasm_bindgen(js_name = setFillStyle)]
    pub fn set_fill_style(&self, color: &str) {
        self.context.set_fill_style_str(color);
    }
    
    /// Set stroke style
    #[wasm_bindgen(js_name = setStrokeStyle)]
    pub fn set_stroke_style(&self, color: &str) {
        self.context.set_stroke_style_str(color);
    }
    
    /// Set line width
    #[wasm_bindgen(js_name = setLineWidth)]
    pub fn set_line_width(&self, width: f64) {
        self.context.set_line_width(width);
    }
    
    /// Fill rectangle
    #[wasm_bindgen(js_name = fillRect)]
    pub fn fill_rect(&self, x: f64, y: f64, width: f64, height: f64) {
        self.context.fill_rect(x, y, width, height);
    }
    
    /// Stroke rectangle
    #[wasm_bindgen(js_name = strokeRect)]
    pub fn stroke_rect(&self, x: f64, y: f64, width: f64, height: f64) {
        self.context.stroke_rect(x, y, width, height);
    }
    
    /// Begin path
    #[wasm_bindgen(js_name = beginPath)]
    pub fn begin_path(&self) {
        self.context.begin_path();
    }
    
    /// Close path
    #[wasm_bindgen(js_name = closePath)]
    pub fn close_path(&self) {
        self.context.close_path();
    }
    
    /// Move to
    #[wasm_bindgen(js_name = moveTo)]
    pub fn move_to(&self, x: f64, y: f64) {
        self.context.move_to(x, y);
    }
    
    /// Line to
    #[wasm_bindgen(js_name = lineTo)]
    pub fn line_to(&self, x: f64, y: f64) {
        self.context.line_to(x, y);
    }
    
    /// Draw arc
    #[wasm_bindgen]
    pub fn arc(&self, x: f64, y: f64, radius: f64, start_angle: f64, end_angle: f64) -> Result<(), JsValue> {
        self.context.arc(x, y, radius, start_angle, end_angle)
    }
    
    /// Fill circle
    #[wasm_bindgen(js_name = fillCircle)]
    pub fn fill_circle(&self, x: f64, y: f64, radius: f64) -> Result<(), JsValue> {
        self.context.begin_path();
        self.context.arc(x, y, radius, 0.0, std::f64::consts::PI * 2.0)?;
        self.context.fill();
        Ok(())
    }
    
    /// Stroke
    #[wasm_bindgen]
    pub fn stroke(&self) {
        self.context.stroke();
    }
    
    /// Fill
    #[wasm_bindgen]
    pub fn fill(&self) {
        self.context.fill();
    }
    
    /// Fill text
    #[wasm_bindgen(js_name = fillText)]
    pub fn fill_text(&self, text: &str, x: f64, y: f64) -> Result<(), JsValue> {
        self.context.fill_text(text, x, y)
    }
    
    /// Set font
    #[wasm_bindgen(js_name = setFont)]
    pub fn set_font(&self, font: &str) {
        self.context.set_font(font);
    }
    
    /// Set text align
    #[wasm_bindgen(js_name = setTextAlign)]
    pub fn set_text_align(&self, align: &str) {
        self.context.set_text_align(align);
    }
    
    /// Save context state
    #[wasm_bindgen]
    pub fn save(&self) {
        self.context.save();
    }
    
    /// Restore context state
    #[wasm_bindgen]
    pub fn restore(&self) {
        self.context.restore();
    }
    
    /// Translate
    #[wasm_bindgen]
    pub fn translate(&self, x: f64, y: f64) -> Result<(), JsValue> {
        self.context.translate(x, y)
    }
    
    /// Rotate
    #[wasm_bindgen]
    pub fn rotate(&self, angle: f64) -> Result<(), JsValue> {
        self.context.rotate(angle)
    }
    
    /// Scale
    #[wasm_bindgen]
    pub fn scale(&self, x: f64, y: f64) -> Result<(), JsValue> {
        self.context.scale(x, y)
    }
}

/// WebSocket bridge
#[wasm_bindgen]
pub struct WebSocketBridge {
    socket: web_sys::WebSocket,
}

#[wasm_bindgen]
impl WebSocketBridge {
    /// Create new WebSocket connection
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str) -> Result<WebSocketBridge, JsValue> {
        let socket = web_sys::WebSocket::new(url)?;
        Ok(WebSocketBridge { socket })
    }
    
    /// Send message
    #[wasm_bindgen]
    pub fn send(&self, message: &str) -> Result<(), JsValue> {
        self.socket.send_with_str(message)
    }
    
    /// Close connection
    #[wasm_bindgen]
    pub fn close(&self) -> Result<(), JsValue> {
        self.socket.close()
    }
    
    /// Get ready state
    #[wasm_bindgen(js_name = getReadyState)]
    pub fn get_ready_state(&self) -> u16 {
        self.socket.ready_state()
    }
    
    /// Set onmessage handler
    #[wasm_bindgen(js_name = setOnMessage)]
    pub fn set_on_message(&self, callback: &Function) {
        self.socket.set_onmessage(Some(callback.unchecked_ref()));
    }
    
    /// Set onopen handler
    #[wasm_bindgen(js_name = setOnOpen)]
    pub fn set_on_open(&self, callback: &Function) {
        self.socket.set_onopen(Some(callback.unchecked_ref()));
    }
    
    /// Set onclose handler
    #[wasm_bindgen(js_name = setOnClose)]
    pub fn set_on_close(&self, callback: &Function) {
        self.socket.set_onclose(Some(callback.unchecked_ref()));
    }
    
    /// Set onerror handler
    #[wasm_bindgen(js_name = setOnError)]
    pub fn set_on_error(&self, callback: &Function) {
        self.socket.set_onerror(Some(callback.unchecked_ref()));
    }
}

/// User agent information
#[wasm_bindgen]
pub struct NavigatorBridge;

#[wasm_bindgen]
impl NavigatorBridge {
    /// Get user agent
    #[wasm_bindgen(js_name = getUserAgent)]
    pub fn get_user_agent() -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        Ok(window.navigator().user_agent()?)
    }
    
    /// Get language
    #[wasm_bindgen(js_name = getLanguage)]
    pub fn get_language() -> Result<Option<String>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        Ok(window.navigator().language())
    }
    
    /// Check if online
    #[wasm_bindgen(js_name = isOnline)]
    pub fn is_online() -> bool {
        if let Some(window) = web_sys::window() {
            return window.navigator().on_line();
        }
        true
    }
    
    /// Get platform
    #[wasm_bindgen(js_name = getPlatform)]
    pub fn get_platform() -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        window.navigator().platform()
    }
    
    /// Get hardware concurrency (CPU cores)
    #[wasm_bindgen(js_name = getCpuCores)]
    pub fn get_cpu_cores() -> u64 {
        if let Some(window) = web_sys::window() {
            return window.navigator().hardware_concurrency() as u64;
        }
        1
    }
}
