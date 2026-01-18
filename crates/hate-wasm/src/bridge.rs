//! JavaScript Bridge
//!
//! Minimal bridge layer between Hate WASM runtime and JavaScript APIs.
//! This provides console, timers, fetch, storage, and other web APIs.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use js_sys::{Function, Date, JSON, Reflect, Array, Object};
use web_sys::{console, Storage, Request, RequestInit, Response, Headers};

/// Console API bridge
#[wasm_bindgen]
pub struct ConsoleBridge;

#[wasm_bindgen]
impl ConsoleBridge {
    /// Log message
    #[wasm_bindgen]
    pub fn log(message: &str) {
        console::log_1(&JsValue::from_str(message));
    }
    
    /// Log multiple values
    #[wasm_bindgen(js_name = logMany)]
    pub fn log_many(values: Box<[JsValue]>) {
        let arr = Array::new();
        for v in values.iter() {
            arr.push(v);
        }
        console::log(&arr);
    }
    
    /// Warning
    #[wasm_bindgen]
    pub fn warn(message: &str) {
        console::warn_1(&JsValue::from_str(message));
    }
    
    /// Error
    #[wasm_bindgen]
    pub fn error(message: &str) {
        console::error_1(&JsValue::from_str(message));
    }
    
    /// Info
    #[wasm_bindgen]
    pub fn info(message: &str) {
        console::info_1(&JsValue::from_str(message));
    }
    
    /// Debug
    #[wasm_bindgen]
    pub fn debug(message: &str) {
        console::debug_1(&JsValue::from_str(message));
    }
    
    /// Table
    #[wasm_bindgen]
    pub fn table(data: JsValue) {
        console::table_1(&data);
    }
    
    /// Group
    #[wasm_bindgen]
    pub fn group(label: &str) {
        console::group_1(&JsValue::from_str(label));
    }
    
    /// Group end
    #[wasm_bindgen(js_name = groupEnd)]
    pub fn group_end() {
        console::group_end();
    }
    
    /// Clear
    #[wasm_bindgen]
    pub fn clear() {
        console::clear();
    }
    
    /// Time start
    #[wasm_bindgen]
    pub fn time(label: &str) {
        console::time_with_label(label);
    }
    
    /// Time end
    #[wasm_bindgen(js_name = timeEnd)]
    pub fn time_end(label: &str) {
        console::time_end_with_label(label);
    }
    
    /// Assert
    #[wasm_bindgen]
    pub fn assert(condition: bool, message: &str) {
        console::assert_with_condition_and_data_1(condition, &JsValue::from_str(message));
    }
    
    /// Count
    #[wasm_bindgen]
    pub fn count(label: &str) {
        console::count_with_label(label);
    }
    
    /// Count reset
    #[wasm_bindgen(js_name = countReset)]
    pub fn count_reset(label: &str) {
        console::count_reset_with_label(label);
    }
}

/// Timer API bridge
#[wasm_bindgen]
pub struct TimerBridge;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout_js(closure: &Function, timeout: i32) -> i32;
    
    #[wasm_bindgen(js_name = clearTimeout)]
    fn clear_timeout_js(id: i32);
    
    #[wasm_bindgen(js_name = setInterval)]
    fn set_interval_js(closure: &Function, interval: i32) -> i32;
    
    #[wasm_bindgen(js_name = clearInterval)]
    fn clear_interval_js(id: i32);
    
    #[wasm_bindgen(js_name = requestAnimationFrame)]
    fn request_animation_frame_js(closure: &Function) -> i32;
    
    #[wasm_bindgen(js_name = cancelAnimationFrame)]
    fn cancel_animation_frame_js(id: i32);
}

#[wasm_bindgen]
impl TimerBridge {
    /// Set timeout
    #[wasm_bindgen(js_name = setTimeout)]
    pub fn set_timeout(callback: &Function, delay_ms: i32) -> i32 {
        set_timeout_js(callback, delay_ms)
    }
    
    /// Clear timeout
    #[wasm_bindgen(js_name = clearTimeout)]
    pub fn clear_timeout(id: i32) {
        clear_timeout_js(id);
    }
    
    /// Set interval
    #[wasm_bindgen(js_name = setInterval)]
    pub fn set_interval(callback: &Function, interval_ms: i32) -> i32 {
        set_interval_js(callback, interval_ms)
    }
    
    /// Clear interval
    #[wasm_bindgen(js_name = clearInterval)]
    pub fn clear_interval(id: i32) {
        clear_interval_js(id);
    }
    
    /// Request animation frame
    #[wasm_bindgen(js_name = requestAnimationFrame)]
    pub fn request_animation_frame(callback: &Function) -> i32 {
        request_animation_frame_js(callback)
    }
    
    /// Cancel animation frame
    #[wasm_bindgen(js_name = cancelAnimationFrame)]
    pub fn cancel_animation_frame(id: i32) {
        cancel_animation_frame_js(id);
    }
}

/// Fetch API bridge
#[wasm_bindgen]
pub struct FetchBridge;

#[wasm_bindgen]
impl FetchBridge {
    /// Simple GET request
    #[wasm_bindgen]
    pub async fn get(url: &str) -> Result<JsValue, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let promise = window.fetch_with_str(url);
        let response = wasm_bindgen_futures::JsFuture::from(promise).await?;
        let response: Response = response.dyn_into()?;
        
        if !response.ok() {
            return Err(JsValue::from_str(&format!("HTTP error: {}", response.status())));
        }
        
        let json = wasm_bindgen_futures::JsFuture::from(response.json()?).await?;
        Ok(json)
    }
    
    /// GET request returning text
    #[wasm_bindgen(js_name = getText)]
    pub async fn get_text(url: &str) -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let promise = window.fetch_with_str(url);
        let response = wasm_bindgen_futures::JsFuture::from(promise).await?;
        let response: Response = response.dyn_into()?;
        
        if !response.ok() {
            return Err(JsValue::from_str(&format!("HTTP error: {}", response.status())));
        }
        
        let text = wasm_bindgen_futures::JsFuture::from(response.text()?).await?;
        Ok(text.as_string().unwrap_or_default())
    }
    
    /// POST request with JSON body
    #[wasm_bindgen]
    pub async fn post(url: &str, body: JsValue) -> Result<JsValue, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        
        let opts = RequestInit::new();
        opts.set_method("POST");
        let body_str: JsValue = JSON::stringify(&body)?.into();
        opts.set_body(&body_str);
        
        let headers = Headers::new()?;
        headers.set("Content-Type", "application/json")?;
        opts.set_headers(&headers.into());
        
        let request = Request::new_with_str_and_init(url, &opts)?;
        let promise = window.fetch_with_request(&request);
        let response = wasm_bindgen_futures::JsFuture::from(promise).await?;
        let response: Response = response.dyn_into()?;
        
        if !response.ok() {
            return Err(JsValue::from_str(&format!("HTTP error: {}", response.status())));
        }
        
        let json = wasm_bindgen_futures::JsFuture::from(response.json()?).await?;
        Ok(json)
    }
    
    /// Generic fetch with options
    #[wasm_bindgen]
    pub async fn fetch(url: &str, options: JsValue) -> Result<JsValue, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        
        let opts = RequestInit::new();
        
        // Extract method
        if let Ok(method) = Reflect::get(&options, &JsValue::from_str("method")) {
            if let Some(m) = method.as_string() {
                opts.set_method(&m);
            }
        }
        
        // Extract body
        if let Ok(body) = Reflect::get(&options, &JsValue::from_str("body")) {
            if !body.is_undefined() {
                opts.set_body(&body);
            }
        }
        
        // Extract headers
        if let Ok(headers_obj) = Reflect::get(&options, &JsValue::from_str("headers")) {
            if !headers_obj.is_undefined() {
                opts.set_headers(&headers_obj);
            }
        }
        
        let request = Request::new_with_str_and_init(url, &opts)?;
        let promise = window.fetch_with_request(&request);
        let response = wasm_bindgen_futures::JsFuture::from(promise).await?;
        let response: Response = response.dyn_into()?;
        
        // Build response object
        let result = Object::new();
        Reflect::set(&result, &"ok".into(), &JsValue::from_bool(response.ok()))?;
        Reflect::set(&result, &"status".into(), &JsValue::from_f64(response.status() as f64))?;
        Reflect::set(&result, &"statusText".into(), &JsValue::from_str(&response.status_text()))?;
        
        // Get body as text
        let text = wasm_bindgen_futures::JsFuture::from(response.text()?).await?;
        Reflect::set(&result, &"text".into(), &text)?;
        
        Ok(result.into())
    }
}

/// Storage API bridge (localStorage / sessionStorage)
#[wasm_bindgen]
pub struct StorageBridge;

#[wasm_bindgen]
impl StorageBridge {
    /// Get local storage
    fn local_storage() -> Result<Storage, JsValue> {
        web_sys::window()
            .ok_or("No window")?
            .local_storage()?
            .ok_or_else(|| "No local storage".into())
    }
    
    /// Get session storage
    fn session_storage() -> Result<Storage, JsValue> {
        web_sys::window()
            .ok_or("No window")?
            .session_storage()?
            .ok_or_else(|| "No session storage".into())
    }
    
    /// Get item from local storage
    #[wasm_bindgen(js_name = localGet)]
    pub fn local_get(key: &str) -> Result<Option<String>, JsValue> {
        Self::local_storage()?.get_item(key)
    }
    
    /// Set item in local storage
    #[wasm_bindgen(js_name = localSet)]
    pub fn local_set(key: &str, value: &str) -> Result<(), JsValue> {
        Self::local_storage()?.set_item(key, value)
    }
    
    /// Remove item from local storage
    #[wasm_bindgen(js_name = localRemove)]
    pub fn local_remove(key: &str) -> Result<(), JsValue> {
        Self::local_storage()?.remove_item(key)
    }
    
    /// Clear local storage
    #[wasm_bindgen(js_name = localClear)]
    pub fn local_clear() -> Result<(), JsValue> {
        Self::local_storage()?.clear()
    }
    
    /// Get local storage length
    #[wasm_bindgen(js_name = localLength)]
    pub fn local_length() -> Result<u32, JsValue> {
        Self::local_storage()?.length()
    }
    
    /// Get key at index from local storage
    #[wasm_bindgen(js_name = localKey)]
    pub fn local_key(index: u32) -> Result<Option<String>, JsValue> {
        Self::local_storage()?.key(index)
    }
    
    /// Get item from session storage
    #[wasm_bindgen(js_name = sessionGet)]
    pub fn session_get(key: &str) -> Result<Option<String>, JsValue> {
        Self::session_storage()?.get_item(key)
    }
    
    /// Set item in session storage
    #[wasm_bindgen(js_name = sessionSet)]
    pub fn session_set(key: &str, value: &str) -> Result<(), JsValue> {
        Self::session_storage()?.set_item(key, value)
    }
    
    /// Remove item from session storage
    #[wasm_bindgen(js_name = sessionRemove)]
    pub fn session_remove(key: &str) -> Result<(), JsValue> {
        Self::session_storage()?.remove_item(key)
    }
    
    /// Clear session storage
    #[wasm_bindgen(js_name = sessionClear)]
    pub fn session_clear() -> Result<(), JsValue> {
        Self::session_storage()?.clear()
    }
}

/// JSON utilities
#[wasm_bindgen]
pub struct JsonBridge;

#[wasm_bindgen]
impl JsonBridge {
    /// Parse JSON string
    #[wasm_bindgen]
    pub fn parse(json: &str) -> Result<JsValue, JsValue> {
        JSON::parse(json)
    }
    
    /// Stringify value to JSON
    #[wasm_bindgen]
    pub fn stringify(value: &JsValue) -> Result<String, JsValue> {
        JSON::stringify(value).map(|s| s.into())
    }
    
    /// Stringify with indentation
    #[wasm_bindgen(js_name = stringifyPretty)]
    pub fn stringify_pretty(value: &JsValue, indent: u32) -> Result<String, JsValue> {
        JSON::stringify_with_replacer_and_space(value, &JsValue::NULL, &JsValue::from_f64(indent as f64))
            .map(|s| s.into())
    }
}

/// Date/Time utilities
#[wasm_bindgen]
pub struct DateBridge;

#[wasm_bindgen]
impl DateBridge {
    /// Get current timestamp (milliseconds)
    #[wasm_bindgen]
    pub fn now() -> f64 {
        Date::now()
    }
    
    /// Create date from timestamp
    #[wasm_bindgen(js_name = fromTimestamp)]
    pub fn from_timestamp(ms: f64) -> Date {
        Date::new(&JsValue::from_f64(ms))
    }
    
    /// Get ISO string
    #[wasm_bindgen(js_name = toIsoString)]
    pub fn to_iso_string(date: &Date) -> String {
        date.to_iso_string().into()
    }
    
    /// Get locale string
    #[wasm_bindgen(js_name = toLocaleString)]
    pub fn to_locale_string(date: &Date) -> String {
        date.to_locale_string("en-US", &Object::new()).into()
    }
    
    /// Get year
    #[wasm_bindgen(js_name = getYear)]
    pub fn get_year(date: &Date) -> u32 {
        date.get_full_year()
    }
    
    /// Get month (0-11)
    #[wasm_bindgen(js_name = getMonth)]
    pub fn get_month(date: &Date) -> u32 {
        date.get_month()
    }
    
    /// Get day of month (1-31)
    #[wasm_bindgen(js_name = getDay)]
    pub fn get_day(date: &Date) -> u32 {
        date.get_date()
    }
    
    /// Get hours (0-23)
    #[wasm_bindgen(js_name = getHours)]
    pub fn get_hours(date: &Date) -> u32 {
        date.get_hours()
    }
    
    /// Get minutes (0-59)
    #[wasm_bindgen(js_name = getMinutes)]
    pub fn get_minutes(date: &Date) -> u32 {
        date.get_minutes()
    }
    
    /// Get seconds (0-59)
    #[wasm_bindgen(js_name = getSeconds)]
    pub fn get_seconds(date: &Date) -> u32 {
        date.get_seconds()
    }
}

/// URL utilities
#[wasm_bindgen]
pub struct UrlBridge;

#[wasm_bindgen]
impl UrlBridge {
    /// Get current URL
    #[wasm_bindgen(js_name = getCurrentUrl)]
    pub fn get_current_url() -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let location = window.location();
        Ok(location.href()?)
    }
    
    /// Get pathname
    #[wasm_bindgen(js_name = getPathname)]
    pub fn get_pathname() -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let location = window.location();
        Ok(location.pathname()?)
    }
    
    /// Get query string
    #[wasm_bindgen(js_name = getSearch)]
    pub fn get_search() -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let location = window.location();
        Ok(location.search()?)
    }
    
    /// Get hash
    #[wasm_bindgen(js_name = getHash)]
    pub fn get_hash() -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let location = window.location();
        Ok(location.hash()?)
    }
    
    /// Set hash
    #[wasm_bindgen(js_name = setHash)]
    pub fn set_hash(hash: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let location = window.location();
        location.set_hash(hash)
    }
    
    /// Navigate to URL
    #[wasm_bindgen(js_name = navigate)]
    pub fn navigate(url: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let location = window.location();
        location.set_href(url)
    }
    
    /// Reload page
    #[wasm_bindgen]
    pub fn reload() -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let location = window.location();
        location.reload()
    }
    
    /// Push history state
    #[wasm_bindgen(js_name = pushState)]
    pub fn push_state(url: &str, title: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let history = window.history()?;
        history.push_state_with_url(&JsValue::NULL, title, Some(url))
    }
    
    /// Go back
    #[wasm_bindgen]
    pub fn back() -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let history = window.history()?;
        history.back()
    }
    
    /// Go forward
    #[wasm_bindgen]
    pub fn forward() -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let history = window.history()?;
        history.forward()
    }
}
