//! DOM Bridge
//!
//! Provides a minimal, efficient bridge between Hate code and browser DOM APIs.
//! This module exposes DOM operations that can be called from Hate code.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, HtmlElement, Node, NodeList, 
    Event, EventTarget, Window, MouseEvent, KeyboardEvent,
};
use js_sys::Function;

/// DOM manipulation API exposed to Hate code
#[wasm_bindgen]
pub struct DomBridge {
    document: Document,
    window: Window,
}

#[wasm_bindgen]
impl DomBridge {
    /// Create a new DOM bridge
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<DomBridge, JsValue> {
        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("No window"))?;
        let document = window.document()
            .ok_or_else(|| JsValue::from_str("No document"))?;
        
        Ok(DomBridge { document, window })
    }
    
    // ==================== Element Selection ====================
    
    /// Select element by CSS selector
    #[wasm_bindgen(js_name = querySelector)]
    pub fn query_selector(&self, selector: &str) -> Option<Element> {
        self.document.query_selector(selector).ok().flatten()
    }
    
    /// Select all elements matching CSS selector
    #[wasm_bindgen(js_name = querySelectorAll)]
    pub fn query_selector_all(&self, selector: &str) -> Result<NodeList, JsValue> {
        self.document.query_selector_all(selector)
    }
    
    /// Get element by ID
    #[wasm_bindgen(js_name = getElementById)]
    pub fn get_element_by_id(&self, id: &str) -> Option<Element> {
        self.document.get_element_by_id(id)
    }
    
    /// Get elements by class name
    #[wasm_bindgen(js_name = getElementsByClassName)]
    pub fn get_elements_by_class_name(&self, class: &str) -> web_sys::HtmlCollection {
        self.document.get_elements_by_class_name(class)
    }
    
    /// Get elements by tag name
    #[wasm_bindgen(js_name = getElementsByTagName)]
    pub fn get_elements_by_tag_name(&self, tag: &str) -> web_sys::HtmlCollection {
        self.document.get_elements_by_tag_name(tag)
    }
    
    // ==================== Element Creation ====================
    
    /// Create a new element
    #[wasm_bindgen(js_name = createElement)]
    pub fn create_element(&self, tag: &str) -> Result<Element, JsValue> {
        self.document.create_element(tag)
    }
    
    /// Create a text node
    #[wasm_bindgen(js_name = createTextNode)]
    pub fn create_text_node(&self, text: &str) -> web_sys::Text {
        self.document.create_text_node(text)
    }
    
    /// Create a document fragment
    #[wasm_bindgen(js_name = createFragment)]
    pub fn create_fragment(&self) -> Result<web_sys::DocumentFragment, JsValue> {
        Ok(self.document.create_document_fragment())
    }
    
    // ==================== Element Properties ====================
    
    /// Get inner HTML
    #[wasm_bindgen(js_name = getInnerHtml)]
    pub fn get_inner_html(&self, element: &Element) -> String {
        element.inner_html()
    }
    
    /// Set inner HTML
    #[wasm_bindgen(js_name = setInnerHtml)]
    pub fn set_inner_html(&self, element: &Element, html: &str) {
        element.set_inner_html(html);
    }
    
    /// Get text content
    #[wasm_bindgen(js_name = getTextContent)]
    pub fn get_text_content(&self, node: &Node) -> Option<String> {
        node.text_content()
    }
    
    /// Set text content
    #[wasm_bindgen(js_name = setTextContent)]
    pub fn set_text_content(&self, node: &Node, text: &str) {
        node.set_text_content(Some(text));
    }
    
    /// Get attribute
    #[wasm_bindgen(js_name = getAttribute)]
    pub fn get_attribute(&self, element: &Element, name: &str) -> Option<String> {
        element.get_attribute(name)
    }
    
    /// Set attribute
    #[wasm_bindgen(js_name = setAttribute)]
    pub fn set_attribute(&self, element: &Element, name: &str, value: &str) -> Result<(), JsValue> {
        element.set_attribute(name, value)
    }
    
    /// Remove attribute
    #[wasm_bindgen(js_name = removeAttribute)]
    pub fn remove_attribute(&self, element: &Element, name: &str) -> Result<(), JsValue> {
        element.remove_attribute(name)
    }
    
    /// Check if element has attribute
    #[wasm_bindgen(js_name = hasAttribute)]
    pub fn has_attribute(&self, element: &Element, name: &str) -> bool {
        element.has_attribute(name)
    }
    
    // ==================== Class List ====================
    
    /// Add class
    #[wasm_bindgen(js_name = addClass)]
    pub fn add_class(&self, element: &Element, class: &str) -> Result<(), JsValue> {
        element.class_list().add_1(class)
    }
    
    /// Remove class
    #[wasm_bindgen(js_name = removeClass)]
    pub fn remove_class(&self, element: &Element, class: &str) -> Result<(), JsValue> {
        element.class_list().remove_1(class)
    }
    
    /// Toggle class
    #[wasm_bindgen(js_name = toggleClass)]
    pub fn toggle_class(&self, element: &Element, class: &str) -> Result<bool, JsValue> {
        element.class_list().toggle(class)
    }
    
    /// Check if element has class
    #[wasm_bindgen(js_name = hasClass)]
    pub fn has_class(&self, element: &Element, class: &str) -> bool {
        element.class_list().contains(class)
    }
    
    // ==================== Style ====================
    
    /// Get computed style
    #[wasm_bindgen(js_name = getStyle)]
    pub fn get_style(&self, element: &Element, property: &str) -> Result<String, JsValue> {
        let html_element = element.dyn_ref::<HtmlElement>()
            .ok_or_else(|| JsValue::from_str("Not an HTML element"))?;
        
        Ok(html_element.style().get_property_value(property).unwrap_or_default())
    }
    
    /// Set style property
    #[wasm_bindgen(js_name = setStyle)]
    pub fn set_style(&self, element: &Element, property: &str, value: &str) -> Result<(), JsValue> {
        let html_element = element.dyn_ref::<HtmlElement>()
            .ok_or_else(|| JsValue::from_str("Not an HTML element"))?;
        
        html_element.style().set_property(property, value)
    }
    
    // ==================== DOM Traversal ====================
    
    /// Get parent element
    #[wasm_bindgen(js_name = getParent)]
    pub fn get_parent(&self, element: &Element) -> Option<Element> {
        element.parent_element()
    }
    
    /// Get children
    #[wasm_bindgen(js_name = getChildren)]
    pub fn get_children(&self, element: &Element) -> web_sys::HtmlCollection {
        element.children()
    }
    
    /// Get first child element
    #[wasm_bindgen(js_name = getFirstChild)]
    pub fn get_first_child(&self, element: &Element) -> Option<Element> {
        element.first_element_child()
    }
    
    /// Get last child element
    #[wasm_bindgen(js_name = getLastChild)]
    pub fn get_last_child(&self, element: &Element) -> Option<Element> {
        element.last_element_child()
    }
    
    /// Get next sibling
    #[wasm_bindgen(js_name = getNextSibling)]
    pub fn get_next_sibling(&self, element: &Element) -> Option<Element> {
        element.next_element_sibling()
    }
    
    /// Get previous sibling
    #[wasm_bindgen(js_name = getPrevSibling)]
    pub fn get_prev_sibling(&self, element: &Element) -> Option<Element> {
        element.previous_element_sibling()
    }
    
    // ==================== DOM Manipulation ====================
    
    /// Append child
    #[wasm_bindgen(js_name = appendChild)]
    pub fn append_child(&self, parent: &Node, child: &Node) -> Result<Node, JsValue> {
        parent.append_child(child)
    }
    
    /// Insert before
    #[wasm_bindgen(js_name = insertBefore)]
    pub fn insert_before(&self, parent: &Node, new_node: &Node, reference: Option<Node>) -> Result<Node, JsValue> {
        parent.insert_before(new_node, reference.as_ref())
    }
    
    /// Remove child
    #[wasm_bindgen(js_name = removeChild)]
    pub fn remove_child(&self, parent: &Node, child: &Node) -> Result<Node, JsValue> {
        parent.remove_child(child)
    }
    
    /// Replace child
    #[wasm_bindgen(js_name = replaceChild)]
    pub fn replace_child(&self, parent: &Node, new_child: &Node, old_child: &Node) -> Result<Node, JsValue> {
        parent.replace_child(new_child, old_child)
    }
    
    /// Clone node
    #[wasm_bindgen(js_name = cloneNode)]
    pub fn clone_node(&self, node: &Node, deep: bool) -> Result<Node, JsValue> {
        node.clone_node_with_deep(deep)
    }
    
    /// Remove element from DOM
    #[wasm_bindgen(js_name = remove)]
    pub fn remove(&self, element: &Element) {
        element.remove();
    }
    
    // ==================== Window/Document ====================
    
    /// Get document title
    #[wasm_bindgen(js_name = getTitle)]
    pub fn get_title(&self) -> String {
        self.document.title()
    }
    
    /// Set document title
    #[wasm_bindgen(js_name = setTitle)]
    pub fn set_title(&self, title: &str) {
        self.document.set_title(title);
    }
    
    /// Get document body
    #[wasm_bindgen(js_name = getBody)]
    pub fn get_body(&self) -> Option<HtmlElement> {
        self.document.body()
    }
    
    /// Get window inner width
    #[wasm_bindgen(js_name = getWindowWidth)]
    pub fn get_window_width(&self) -> Result<i32, JsValue> {
        self.window.inner_width().map(|v| v.as_f64().unwrap_or(0.0) as i32)
    }
    
    /// Get window inner height
    #[wasm_bindgen(js_name = getWindowHeight)]
    pub fn get_window_height(&self) -> Result<i32, JsValue> {
        self.window.inner_height().map(|v| v.as_f64().unwrap_or(0.0) as i32)
    }
    
    /// Scroll to position
    #[wasm_bindgen(js_name = scrollTo)]
    pub fn scroll_to(&self, x: f64, y: f64) {
        self.window.scroll_to_with_x_and_y(x, y);
    }
    
    /// Get scroll position X
    #[wasm_bindgen(js_name = getScrollX)]
    pub fn get_scroll_x(&self) -> Result<f64, JsValue> {
        self.window.scroll_x()
    }
    
    /// Get scroll position Y
    #[wasm_bindgen(js_name = getScrollY)]
    pub fn get_scroll_y(&self) -> Result<f64, JsValue> {
        self.window.scroll_y()
    }
}

impl Default for DomBridge {
    fn default() -> Self {
        Self::new().expect("Failed to create DomBridge")
    }
}

/// Event handling utilities
#[wasm_bindgen]
pub struct EventBridge;

#[wasm_bindgen]
impl EventBridge {
    /// Add event listener
    #[wasm_bindgen(js_name = addEventListener)]
    pub fn add_event_listener(
        target: &EventTarget,
        event_type: &str,
        callback: &Function,
    ) -> Result<(), JsValue> {
        target.add_event_listener_with_callback(event_type, callback)
    }
    
    /// Remove event listener
    #[wasm_bindgen(js_name = removeEventListener)]
    pub fn remove_event_listener(
        target: &EventTarget,
        event_type: &str,
        callback: &Function,
    ) -> Result<(), JsValue> {
        target.remove_event_listener_with_callback(event_type, callback)
    }
    
    /// Prevent default
    #[wasm_bindgen(js_name = preventDefault)]
    pub fn prevent_default(event: &Event) {
        event.prevent_default();
    }
    
    /// Stop propagation
    #[wasm_bindgen(js_name = stopPropagation)]
    pub fn stop_propagation(event: &Event) {
        event.stop_propagation();
    }
    
    /// Get event target
    #[wasm_bindgen(js_name = getTarget)]
    pub fn get_target(event: &Event) -> Option<EventTarget> {
        event.target()
    }
    
    /// Get current target
    #[wasm_bindgen(js_name = getCurrentTarget)]
    pub fn get_current_target(event: &Event) -> Option<EventTarget> {
        event.current_target()
    }
    
    /// Get mouse X position
    #[wasm_bindgen(js_name = getMouseX)]
    pub fn get_mouse_x(event: &MouseEvent) -> i32 {
        event.client_x()
    }
    
    /// Get mouse Y position
    #[wasm_bindgen(js_name = getMouseY)]
    pub fn get_mouse_y(event: &MouseEvent) -> i32 {
        event.client_y()
    }
    
    /// Get key from keyboard event
    #[wasm_bindgen(js_name = getKey)]
    pub fn get_key(event: &KeyboardEvent) -> String {
        event.key()
    }
    
    /// Get key code
    #[wasm_bindgen(js_name = getKeyCode)]
    pub fn get_key_code(event: &KeyboardEvent) -> u32 {
        event.key_code()
    }
    
    /// Check if ctrl key is pressed
    #[wasm_bindgen(js_name = isCtrlPressed)]
    pub fn is_ctrl_pressed(event: &KeyboardEvent) -> bool {
        event.ctrl_key()
    }
    
    /// Check if shift key is pressed
    #[wasm_bindgen(js_name = isShiftPressed)]
    pub fn is_shift_pressed(event: &KeyboardEvent) -> bool {
        event.shift_key()
    }
    
    /// Check if alt key is pressed
    #[wasm_bindgen(js_name = isAltPressed)]
    pub fn is_alt_pressed(event: &KeyboardEvent) -> bool {
        event.alt_key()
    }
}
