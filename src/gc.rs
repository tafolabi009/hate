//! Generational Garbage Collector
//!
//! Two-generation copying collector with:
//! - Nursery (young generation): 1MB, minor collections
//! - Tenured (old generation): Growing heap, major collections
//! - Write barriers for old→young pointers

use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;
use std::cell::Cell;
use std::marker::PhantomData;

/// Size of the nursery (young generation)
const NURSERY_SIZE: usize = 1024 * 1024; // 1MB

/// Threshold for minor GC (bytes allocated in nursery)
const MINOR_GC_THRESHOLD: usize = 512 * 1024; // 500KB

/// Object header for GC-managed objects
#[repr(C)]
pub struct GcHeader {
    /// Mark bit and generation info
    /// Bits 0-1: Generation (0 = nursery, 1+ = tenured)
    /// Bit 2: Mark bit
    /// Bit 3: Forwarding bit (object has been copied)
    flags: Cell<u8>,
    /// Size of the object (excluding header)
    size: u32,
    /// Type tag for the object
    type_tag: u16,
    /// Padding for alignment
    _pad: u16,
}

const FLAG_MARKED: u8 = 0b0100;
const FLAG_FORWARDED: u8 = 0b1000;
const GEN_MASK: u8 = 0b0011;

impl GcHeader {
    fn new(size: u32, type_tag: u16) -> Self {
        Self {
            flags: Cell::new(0),
            size,
            type_tag,
            _pad: 0,
        }
    }
    
    fn generation(&self) -> u8 {
        self.flags.get() & GEN_MASK
    }
    
    fn set_generation(&self, gen: u8) {
        let flags = (self.flags.get() & !GEN_MASK) | (gen & GEN_MASK);
        self.flags.set(flags);
    }
    
    fn is_marked(&self) -> bool {
        self.flags.get() & FLAG_MARKED != 0
    }
    
    fn set_marked(&self, marked: bool) {
        if marked {
            self.flags.set(self.flags.get() | FLAG_MARKED);
        } else {
            self.flags.set(self.flags.get() & !FLAG_MARKED);
        }
    }
    
    fn is_forwarded(&self) -> bool {
        self.flags.get() & FLAG_FORWARDED != 0
    }
    
    fn set_forwarded(&self) {
        self.flags.set(self.flags.get() | FLAG_FORWARDED);
    }
}

/// Trait for GC-managed objects
pub trait GcObject: Sized {
    /// Type tag for this object type
    const TYPE_TAG: u16;
    
    /// Trace references to other GC objects
    fn trace(&self, tracer: &mut dyn FnMut(*mut GcHeader));
}

/// A reference to a GC-managed object
#[repr(transparent)]
pub struct GcRef<T: GcObject> {
    ptr: NonNull<T>,
    _marker: PhantomData<T>,
}

impl<T: GcObject> GcRef<T> {
    /// Create a new GcRef from a raw pointer
    /// 
    /// # Safety
    /// The pointer must be valid and point to a properly initialized T
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr),
            _marker: PhantomData,
        }
    }
    
    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }
    
    /// Get the GC header for this object
    fn header(&self) -> &GcHeader {
        unsafe {
            let header_ptr = (self.ptr.as_ptr() as *mut u8).sub(std::mem::size_of::<GcHeader>());
            &*(header_ptr as *const GcHeader)
        }
    }
}

impl<T: GcObject> Clone for GcRef<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}

impl<T: GcObject> Copy for GcRef<T> {}

impl<T: GcObject> std::ops::Deref for GcRef<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

/// Memory space for allocation
struct Space {
    start: *mut u8,
    end: *mut u8,
    current: *mut u8,
    size: usize,
}

impl Space {
    fn new(size: usize) -> Self {
        let layout = Layout::from_size_align(size, 8).unwrap();
        let start = unsafe { alloc(layout) };
        if start.is_null() {
            panic!("Failed to allocate GC space");
        }
        
        Self {
            start,
            end: unsafe { start.add(size) },
            current: start,
            size,
        }
    }
    
    fn reset(&mut self) {
        self.current = self.start;
    }
    
    fn bytes_used(&self) -> usize {
        self.current as usize - self.start as usize
    }
    
    fn bytes_free(&self) -> usize {
        self.end as usize - self.current as usize
    }
    
    fn alloc(&mut self, size: usize) -> Option<*mut u8> {
        let aligned_size = (size + 7) & !7; // 8-byte alignment
        
        if self.bytes_free() < aligned_size {
            return None;
        }
        
        let ptr = self.current;
        self.current = unsafe { self.current.add(aligned_size) };
        Some(ptr)
    }
    
    fn contains(&self, ptr: *const u8) -> bool {
        let p = ptr as usize;
        p >= self.start as usize && p < self.end as usize
    }
}

impl Drop for Space {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.size, 8).unwrap();
        unsafe { dealloc(self.start, layout) };
    }
}

/// Remember set for old→young pointers
struct RememberSet {
    entries: Vec<*mut GcHeader>,
}

impl RememberSet {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }
    
    fn add(&mut self, ptr: *mut GcHeader) {
        self.entries.push(ptr);
    }
    
    fn clear(&mut self) {
        self.entries.clear();
    }
    
    fn iter(&self) -> impl Iterator<Item = &*mut GcHeader> {
        self.entries.iter()
    }
}

/// GC statistics
#[derive(Debug, Default)]
pub struct GcStats {
    pub minor_collections: u64,
    pub major_collections: u64,
    pub total_allocated: u64,
    pub current_heap_size: usize,
    pub peak_heap_size: usize,
    pub last_pause_ns: u64,
}

/// The garbage collector
pub struct GC {
    /// Young generation (nursery)
    nursery: Space,
    /// To-space for copying (same size as nursery)
    nursery_to: Space,
    /// Old generation
    tenured: Vec<Space>,
    /// Remember set for old→young pointers
    remember_set: RememberSet,
    /// Roots (stack, globals)
    roots: Vec<*mut GcHeader>,
    /// Statistics
    stats: GcStats,
    /// Whether nursery is currently active (vs nursery_to)
    nursery_active: bool,
}

impl GC {
    /// Create a new garbage collector
    pub fn new() -> Self {
        Self {
            nursery: Space::new(NURSERY_SIZE),
            nursery_to: Space::new(NURSERY_SIZE),
            tenured: vec![Space::new(NURSERY_SIZE * 4)], // Start with 4MB tenured
            remember_set: RememberSet::new(),
            roots: Vec::new(),
            stats: GcStats::default(),
            nursery_active: true,
        }
    }
    
    /// Allocate an object
    pub fn alloc<T: GcObject>(&mut self, value: T) -> GcRef<T> {
        let total_size = std::mem::size_of::<GcHeader>() + std::mem::size_of::<T>();
        
        // Try to allocate in nursery
        let ptr = loop {
            let space = self.active_nursery_mut();
            if let Some(ptr) = space.alloc(total_size) {
                break ptr;
            }
            
            // Nursery is full, trigger minor GC
            self.minor_gc();
            
            // Try again
            let space = self.active_nursery_mut();
            if let Some(ptr) = space.alloc(total_size) {
                break ptr;
            }
            
            // Still no space, this shouldn't happen with a proper GC
            panic!("Out of memory");
        };
        
        // Initialize header
        unsafe {
            let header = ptr as *mut GcHeader;
            header.write(GcHeader::new(std::mem::size_of::<T>() as u32, T::TYPE_TAG));
            
            // Initialize object
            let obj_ptr = ptr.add(std::mem::size_of::<GcHeader>()) as *mut T;
            obj_ptr.write(value);
            
            self.stats.total_allocated += total_size as u64;
            
            GcRef::from_raw(obj_ptr)
        }
    }
    
    /// Add a root pointer
    pub fn add_root(&mut self, ptr: *mut GcHeader) {
        self.roots.push(ptr);
    }
    
    /// Remove a root pointer
    pub fn remove_root(&mut self, ptr: *mut GcHeader) {
        if let Some(pos) = self.roots.iter().position(|&p| p == ptr) {
            self.roots.swap_remove(pos);
        }
    }
    
    /// Write barrier - call when writing a reference from old to young
    pub fn write_barrier(&mut self, from: *mut GcHeader, to: *mut GcHeader) {
        unsafe {
            let from_ref = &*from;
            let to_ref = &*to;
            
            // If writing young pointer into old object, remember it
            if from_ref.generation() > 0 && to_ref.generation() == 0 {
                self.remember_set.add(from);
            }
        }
    }
    
    /// Perform a minor (nursery) collection
    pub fn minor_gc(&mut self) {
        let start = std::time::Instant::now();
        
        // Swap spaces
        self.nursery_active = !self.nursery_active;
        self.inactive_nursery_mut().reset();
        
        // Copy roots - clone the vec to avoid borrow issues
        let roots_copy: Vec<_> = self.roots.clone();
        for root in roots_copy {
            self.copy_object(root);
        }
        
        // Copy from remember set
        let remember_entries: Vec<_> = self.remember_set.entries.iter().cloned().collect();
        for ptr in remember_entries {
            self.copy_object(ptr);
        }
        
        // Scavenge (breadth-first copy)
        self.scavenge_nursery();
        
        self.remember_set.clear();
        self.stats.minor_collections += 1;
        self.stats.last_pause_ns = start.elapsed().as_nanos() as u64;
    }
    
    /// Perform a major (full) collection
    pub fn major_gc(&mut self) {
        let start = std::time::Instant::now();
        
        // Mark phase
        for root in &self.roots {
            self.mark(*root);
        }
        
        // Sweep phase would go here (for tenured generation)
        // For now, we just use copying for everything
        
        self.stats.major_collections += 1;
        self.stats.last_pause_ns = start.elapsed().as_nanos() as u64;
    }
    
    /// Get GC statistics
    pub fn stats(&self) -> &GcStats {
        &self.stats
    }
    
    /// Force a collection
    pub fn collect(&mut self) {
        if self.active_nursery().bytes_used() > MINOR_GC_THRESHOLD {
            self.minor_gc();
        }
    }
    
    // ==================== Internal Methods ====================
    
    fn active_nursery(&self) -> &Space {
        if self.nursery_active { &self.nursery } else { &self.nursery_to }
    }
    
    fn active_nursery_mut(&mut self) -> &mut Space {
        if self.nursery_active { &mut self.nursery } else { &mut self.nursery_to }
    }
    
    fn inactive_nursery_mut(&mut self) -> &mut Space {
        if self.nursery_active { &mut self.nursery_to } else { &mut self.nursery }
    }
    
    fn copy_object(&mut self, ptr: *mut GcHeader) {
        if ptr.is_null() {
            return;
        }
        
        unsafe {
            let header = &*ptr;
            
            // Only copy objects in nursery
            if header.generation() != 0 {
                return;
            }
            
            // Already forwarded?
            if header.is_forwarded() {
                return;
            }
            
            let size = header.size as usize + std::mem::size_of::<GcHeader>();
            
            // Copy to to-space
            let dest = self.inactive_nursery_mut().alloc(size);
            if let Some(dest) = dest {
                std::ptr::copy_nonoverlapping(ptr as *const u8, dest, size);
                
                // Mark as forwarded and store forwarding pointer
                header.set_forwarded();
                
                // Update generation (promote if survived multiple collections)
                let dest_header = &*(dest as *const GcHeader);
                dest_header.set_generation(1); // Promote to tenured
            }
        }
    }
    
    fn scavenge_nursery(&mut self) {
        // Process all objects in to-space, copying their references
        // This is a simplified implementation
    }
    
    fn mark(&self, ptr: *mut GcHeader) {
        if ptr.is_null() {
            return;
        }
        
        unsafe {
            let header = &*ptr;
            
            if header.is_marked() {
                return;
            }
            
            header.set_marked(true);
            
            // Trace children would go here
            // This requires knowing the object type and its layout
        }
    }
}

impl Default for GC {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== GC-Managed Object Types ====================

/// GC-managed string
#[repr(C)]
pub struct GcString {
    len: u32,
    hash: u32,
    // Followed by UTF-8 bytes
}

impl GcString {
    pub fn len(&self) -> usize {
        self.len as usize
    }
    
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    pub fn as_str(&self) -> &str {
        unsafe {
            let bytes_ptr = (self as *const Self).add(1) as *const u8;
            let bytes = std::slice::from_raw_parts(bytes_ptr, self.len as usize);
            std::str::from_utf8_unchecked(bytes)
        }
    }
    
    pub fn hash(&self) -> u32 {
        self.hash
    }
}

impl GcObject for GcString {
    const TYPE_TAG: u16 = 1;
    
    fn trace(&self, _tracer: &mut dyn FnMut(*mut GcHeader)) {
        // Strings have no references
    }
}

/// GC-managed array
#[repr(C)]
pub struct GcArray {
    len: u32,
    capacity: u32,
    // Followed by Value elements
}

impl GcArray {
    pub fn len(&self) -> usize {
        self.len as usize
    }
    
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    pub fn capacity(&self) -> usize {
        self.capacity as usize
    }
}

impl GcObject for GcArray {
    const TYPE_TAG: u16 = 2;
    
    fn trace(&self, tracer: &mut dyn FnMut(*mut GcHeader)) {
        // Trace each element
        // Elements are Values which may contain pointers
    }
}

/// GC-managed object (hash map)
#[repr(C)]
pub struct GcObject_ {
    /// Pointer to hidden class (shape/map)
    map: *mut GcHeader,
    /// Number of properties
    count: u32,
    /// Capacity
    capacity: u32,
    // Followed by property values
}

impl GcObject for GcObject_ {
    const TYPE_TAG: u16 = 3;
    
    fn trace(&self, tracer: &mut dyn FnMut(*mut GcHeader)) {
        if !self.map.is_null() {
            tracer(self.map);
        }
        // Trace property values
    }
}

/// GC-managed closure
#[repr(C)]
pub struct GcClosure {
    /// Pointer to function
    function: *mut GcHeader,
    /// Number of upvalues
    upvalue_count: u8,
    // Followed by upvalue pointers
}

impl GcObject for GcClosure {
    const TYPE_TAG: u16 = 4;
    
    fn trace(&self, tracer: &mut dyn FnMut(*mut GcHeader)) {
        if !self.function.is_null() {
            tracer(self.function);
        }
        // Trace upvalues
    }
}

/// GC-managed upvalue
#[repr(C)]
pub struct GcUpvalue {
    /// Pointer to the value (on stack or closed)
    location: *mut crate::value::Value,
    /// Closed value (when upvalue is closed)
    closed: crate::value::Value,
    /// Next upvalue in the list
    next: *mut GcUpvalue,
}

impl GcObject for GcUpvalue {
    const TYPE_TAG: u16 = 5;
    
    fn trace(&self, tracer: &mut dyn FnMut(*mut GcHeader)) {
        // The closed value may contain a pointer
        // Next upvalue
        if !self.next.is_null() {
            let header = unsafe {
                (self.next as *mut u8).sub(std::mem::size_of::<GcHeader>()) as *mut GcHeader
            };
            tracer(header);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gc_creation() {
        let gc = GC::new();
        assert_eq!(gc.stats.minor_collections, 0);
    }
    
    #[test]
    fn test_space_allocation() {
        let mut space = Space::new(1024);
        let ptr = space.alloc(64);
        assert!(ptr.is_some());
        assert_eq!(space.bytes_used(), 64);
    }
    
    #[test]
    fn test_space_exhaustion() {
        let mut space = Space::new(64);
        let _ = space.alloc(32);
        let ptr = space.alloc(64); // Too big
        assert!(ptr.is_none());
    }
    
    #[test]
    fn test_header_flags() {
        let header = GcHeader::new(100, 1);
        assert_eq!(header.generation(), 0);
        assert!(!header.is_marked());
        
        header.set_generation(1);
        assert_eq!(header.generation(), 1);
        
        header.set_marked(true);
        assert!(header.is_marked());
    }
}
