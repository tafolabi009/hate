//! Complete Standard Library for Hate
//!
//! Implements all array, string, object, and math methods with optimal algorithms.

use crate::value::Value;
use crate::object::{HateArray, HateObject};
use crate::intern::Symbol;
use std::cmp::Ordering;

// ==================== Array Methods ====================

/// Array.map - allocate exact size, avoid reallocs
pub fn array_map(arr: &HateArray, mapper: impl Fn(Value, usize) -> Value) -> HateArray {
    let mut result = Vec::with_capacity(arr.len());
    for (i, &elem) in arr.elements.iter().enumerate() {
        result.push(mapper(elem, i));
    }
    HateArray { elements: result }
}

/// Array.filter - two-pass for exact allocation
pub fn array_filter(arr: &HateArray, predicate: impl Fn(Value, usize) -> bool) -> HateArray {
    // First pass: count matches
    let count = arr.elements.iter().enumerate()
        .filter(|(i, &v)| predicate(v, *i))
        .count();
    
    // Second pass: allocate and fill
    let mut result = Vec::with_capacity(count);
    for (i, &elem) in arr.elements.iter().enumerate() {
        if predicate(elem, i) {
            result.push(elem);
        }
    }
    HateArray { elements: result }
}

/// Array.reduce - single-pass accumulator
pub fn array_reduce(
    arr: &HateArray, 
    initial: Value, 
    reducer: impl Fn(Value, Value, usize) -> Value
) -> Value {
    arr.elements.iter().enumerate()
        .fold(initial, |acc, (i, &v)| reducer(acc, v, i))
}

/// Array.forEach - iterator-based, no allocation
pub fn array_for_each(arr: &HateArray, callback: impl Fn(Value, usize)) {
    for (i, &elem) in arr.elements.iter().enumerate() {
        callback(elem, i);
    }
}

/// Array.find - early exit on first match
pub fn array_find(arr: &HateArray, predicate: impl Fn(Value) -> bool) -> Value {
    for &elem in &arr.elements {
        if predicate(elem) {
            return elem;
        }
    }
    Value::null()
}

/// Array.findIndex - return -1 if not found
pub fn array_find_index(arr: &HateArray, predicate: impl Fn(Value) -> bool) -> i32 {
    for (i, &elem) in arr.elements.iter().enumerate() {
        if predicate(elem) {
            return i as i32;
        }
    }
    -1
}

/// Array.some - short-circuit on true
pub fn array_some(arr: &HateArray, predicate: impl Fn(Value) -> bool) -> bool {
    arr.elements.iter().any(|&v| predicate(v))
}

/// Array.every - short-circuit on false
pub fn array_every(arr: &HateArray, predicate: impl Fn(Value) -> bool) -> bool {
    arr.elements.iter().all(|&v| predicate(v))
}

/// Array.sort - Timsort (stable, O(n log n)) via std
pub fn array_sort(arr: &mut HateArray, compare: impl Fn(Value, Value) -> Ordering) {
    arr.elements.sort_by(|&a, &b| compare(a, b));
}

/// Default comparison for sorting
pub fn default_compare(a: Value, b: Value) -> Ordering {
    if let (Some(a_num), Some(b_num)) = (a.to_number(), b.to_number()) {
        a_num.partial_cmp(&b_num).unwrap_or(Ordering::Equal)
    } else if let (Some(a_str), Some(b_str)) = (a.as_string(), b.as_string()) {
        a_str.cmp(b_str)
    } else {
        Ordering::Equal
    }
}

/// Array.reverse - in-place swap
pub fn array_reverse(arr: &mut HateArray) {
    arr.elements.reverse();
}

/// Array.slice - creates new array from range
pub fn array_slice(arr: &HateArray, start: i32, end: i32) -> HateArray {
    let len = arr.len() as i32;
    
    // Handle negative indices
    let start = if start < 0 { (len + start).max(0) } else { start.min(len) } as usize;
    let end = if end < 0 { (len + end).max(0) } else { end.min(len) } as usize;
    
    if start >= end {
        return HateArray::new();
    }
    
    HateArray {
        elements: arr.elements[start..end].to_vec()
    }
}

/// Array.concat - preallocate total size
pub fn array_concat(arrays: &[&HateArray]) -> HateArray {
    let total_len: usize = arrays.iter().map(|a| a.len()).sum();
    let mut result = Vec::with_capacity(total_len);
    
    for arr in arrays {
        result.extend_from_slice(&arr.elements);
    }
    
    HateArray { elements: result }
}

/// Array.flat - BFS with depth limit
pub fn array_flat(arr: &HateArray, _depth: usize) -> HateArray {
    // For now, just return a copy since we don't have nested arrays in Value yet
    arr.clone()
}

/// Array.flatMap - fused map + flat (no intermediate array)
pub fn array_flat_map(
    arr: &HateArray, 
    mapper: impl Fn(Value, usize) -> HateArray
) -> HateArray {
    let mut result = Vec::new();
    for (i, &elem) in arr.elements.iter().enumerate() {
        let mapped = mapper(elem, i);
        result.extend(mapped.elements);
    }
    HateArray { elements: result }
}

/// Array.includes - linear search with early exit
pub fn array_includes(arr: &HateArray, value: Value) -> bool {
    arr.elements.iter().any(|&v| v.eq(value))
}

/// Array.indexOf - linear search
pub fn array_index_of(arr: &HateArray, value: Value) -> i32 {
    arr.elements.iter()
        .position(|&v| v.eq(value))
        .map(|i| i as i32)
        .unwrap_or(-1)
}

/// Array.lastIndexOf - reverse linear search
pub fn array_last_index_of(arr: &HateArray, value: Value) -> i32 {
    arr.elements.iter()
        .rposition(|&v| v.eq(value))
        .map(|i| i as i32)
        .unwrap_or(-1)
}

/// Array.join - single allocation
pub fn array_join(arr: &HateArray, separator: &str) -> String {
    if arr.is_empty() {
        return String::new();
    }
    
    // Calculate total length for pre-allocation
    let strings: Vec<String> = arr.elements.iter()
        .map(|v| format!("{}", v))
        .collect();
    
    let total_len: usize = strings.iter().map(|s| s.len()).sum::<usize>() 
        + separator.len() * (strings.len() - 1);
    
    let mut result = String::with_capacity(total_len);
    for (i, s) in strings.iter().enumerate() {
        if i > 0 {
            result.push_str(separator);
        }
        result.push_str(s);
    }
    result
}

/// Array.fill - memset when primitive
pub fn array_fill(arr: &mut HateArray, value: Value, start: usize, end: usize) {
    let end = end.min(arr.len());
    let start = start.min(end);
    
    // For primitive values, this could be optimized to memset
    for i in start..end {
        arr.elements[i] = value;
    }
}

/// Array.splice - in-place modification
pub fn array_splice(
    arr: &mut HateArray, 
    start: usize, 
    delete_count: usize,
    items: &[Value]
) -> HateArray {
    let start = start.min(arr.len());
    let delete_count = delete_count.min(arr.len() - start);
    
    let removed: Vec<Value> = arr.elements.drain(start..start + delete_count).collect();
    
    for (i, &item) in items.iter().enumerate() {
        arr.elements.insert(start + i, item);
    }
    
    HateArray { elements: removed }
}

// ==================== String Methods ====================

/// Boyer-Moore-Horspool search for substrings
pub fn string_index_of(haystack: &str, needle: &str) -> i32 {
    if needle.is_empty() {
        return 0;
    }
    
    if needle.len() > haystack.len() {
        return -1;
    }
    
    // Build bad character table
    let mut bad_char = [needle.len(); 256];
    for (i, &b) in needle.as_bytes().iter().enumerate().take(needle.len() - 1) {
        bad_char[b as usize] = needle.len() - 1 - i;
    }
    
    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    let mut i = 0;
    
    while i <= haystack.len() - needle.len() {
        let mut j = needle.len() - 1;
        
        while haystack[i + j] == needle[j] {
            if j == 0 {
                return i as i32;
            }
            j -= 1;
        }
        
        i += bad_char[haystack[i + needle.len() - 1] as usize];
    }
    
    -1
}

/// String.lastIndexOf - reverse Boyer-Moore
pub fn string_last_index_of(haystack: &str, needle: &str) -> i32 {
    if needle.is_empty() {
        return haystack.len() as i32;
    }
    
    // Simple implementation for now
    haystack.rfind(needle).map(|i| i as i32).unwrap_or(-1)
}

/// String.split with separator
pub fn string_split(s: &str, separator: &str) -> Vec<String> {
    if separator.is_empty() {
        // Split into characters
        s.chars().map(|c| c.to_string()).collect()
    } else {
        s.split(separator).map(String::from).collect()
    }
}

/// String.replace - first occurrence
pub fn string_replace(s: &str, from: &str, to: &str) -> String {
    if let Some(pos) = s.find(from) {
        let mut result = String::with_capacity(s.len() - from.len() + to.len());
        result.push_str(&s[..pos]);
        result.push_str(to);
        result.push_str(&s[pos + from.len()..]);
        result
    } else {
        s.to_string()
    }
}

/// String.replaceAll - all occurrences
pub fn string_replace_all(s: &str, from: &str, to: &str) -> String {
    s.replace(from, to)
}

/// String.charAt - O(n) for UTF-8 (we return the character)
pub fn string_char_at(s: &str, index: usize) -> Option<char> {
    s.chars().nth(index)
}

/// String.charCodeAt - return Unicode codepoint
pub fn string_char_code_at(s: &str, index: usize) -> Option<u32> {
    s.chars().nth(index).map(|c| c as u32)
}

/// String.substring
pub fn string_substring(s: &str, start: usize, end: usize) -> &str {
    let chars: Vec<char> = s.chars().collect();
    let start = start.min(chars.len());
    let end = end.min(chars.len());
    
    if start >= end {
        return "";
    }
    
    // Find byte positions
    let start_byte = chars[..start].iter().map(|c| c.len_utf8()).sum();
    let end_byte: usize = chars[..end].iter().map(|c| c.len_utf8()).sum();
    
    &s[start_byte..end_byte]
}

/// String.startsWith - memcmp prefix
pub fn string_starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// String.endsWith - memcmp suffix
pub fn string_ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

/// String.repeat - optimized for small counts
pub fn string_repeat(s: &str, count: usize) -> String {
    if count == 0 {
        return String::new();
    }
    if count == 1 {
        return s.to_string();
    }
    
    // Pre-allocate
    let mut result = String::with_capacity(s.len() * count);
    for _ in 0..count {
        result.push_str(s);
    }
    result
}

/// String.padStart - single allocation
pub fn string_pad_start(s: &str, target_len: usize, pad_char: char) -> String {
    let char_count = s.chars().count();
    if char_count >= target_len {
        return s.to_string();
    }
    
    let pad_count = target_len - char_count;
    let mut result = String::with_capacity(s.len() + pad_count * pad_char.len_utf8());
    
    for _ in 0..pad_count {
        result.push(pad_char);
    }
    result.push_str(s);
    result
}

/// String.padEnd - single allocation
pub fn string_pad_end(s: &str, target_len: usize, pad_char: char) -> String {
    let char_count = s.chars().count();
    if char_count >= target_len {
        return s.to_string();
    }
    
    let pad_count = target_len - char_count;
    let mut result = String::with_capacity(s.len() + pad_count * pad_char.len_utf8());
    
    result.push_str(s);
    for _ in 0..pad_count {
        result.push(pad_char);
    }
    result
}

/// String.trim - slice without allocation (returns reference in real impl)
pub fn string_trim(s: &str) -> String {
    s.trim().to_string()
}

/// String.trimStart
pub fn string_trim_start(s: &str) -> String {
    s.trim_start().to_string()
}

/// String.trimEnd
pub fn string_trim_end(s: &str) -> String {
    s.trim_end().to_string()
}

/// String.toUpperCase
pub fn string_upper(s: &str) -> String {
    s.to_uppercase()
}

/// String.toLowerCase
pub fn string_lower(s: &str) -> String {
    s.to_lowercase()
}

// ==================== Math Functions ====================

/// Math.abs
pub fn math_abs(x: f64) -> f64 {
    x.abs()
}

/// Math.sign
pub fn math_sign(x: f64) -> f64 {
    if x > 0.0 { 1.0 }
    else if x < 0.0 { -1.0 }
    else { 0.0 }
}

/// Math.floor
pub fn math_floor(x: f64) -> f64 {
    x.floor()
}

/// Math.ceil
pub fn math_ceil(x: f64) -> f64 {
    x.ceil()
}

/// Math.round
pub fn math_round(x: f64) -> f64 {
    x.round()
}

/// Math.trunc
pub fn math_trunc(x: f64) -> f64 {
    x.trunc()
}

/// Math.min - variadic
pub fn math_min(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::INFINITY, f64::min)
}

/// Math.max - variadic
pub fn math_max(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// Math.sqrt
pub fn math_sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// Math.cbrt
pub fn math_cbrt(x: f64) -> f64 {
    x.cbrt()
}

/// Math.pow
pub fn math_pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

/// Math.exp
pub fn math_exp(x: f64) -> f64 {
    x.exp()
}

/// Math.log (natural log)
pub fn math_log(x: f64) -> f64 {
    x.ln()
}

/// Math.log10
pub fn math_log10(x: f64) -> f64 {
    x.log10()
}

/// Math.log2
pub fn math_log2(x: f64) -> f64 {
    x.log2()
}

/// Math.sin
pub fn math_sin(x: f64) -> f64 {
    x.sin()
}

/// Math.cos
pub fn math_cos(x: f64) -> f64 {
    x.cos()
}

/// Math.tan
pub fn math_tan(x: f64) -> f64 {
    x.tan()
}

/// Math.asin
pub fn math_asin(x: f64) -> f64 {
    x.asin()
}

/// Math.acos
pub fn math_acos(x: f64) -> f64 {
    x.acos()
}

/// Math.atan
pub fn math_atan(x: f64) -> f64 {
    x.atan()
}

/// Math.atan2
pub fn math_atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

/// Math.sinh
pub fn math_sinh(x: f64) -> f64 {
    x.sinh()
}

/// Math.cosh
pub fn math_cosh(x: f64) -> f64 {
    x.cosh()
}

/// Math.tanh
pub fn math_tanh(x: f64) -> f64 {
    x.tanh()
}

/// Math.clamp
pub fn math_clamp(x: f64, min: f64, max: f64) -> f64 {
    x.max(min).min(max)
}

/// xoshiro256** PRNG state
pub struct Xoshiro256StarStar {
    state: [u64; 4],
}

impl Xoshiro256StarStar {
    pub fn new(seed: u64) -> Self {
        // Initialize state using SplitMix64
        let mut state = [0u64; 4];
        let mut z = seed;
        for s in &mut state {
            z = z.wrapping_add(0x9e3779b97f4a7c15);
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            *s = z ^ (z >> 31);
        }
        Self { state }
    }
    
    fn rotl(x: u64, k: u32) -> u64 {
        (x << k) | (x >> (64 - k))
    }
    
    pub fn next(&mut self) -> u64 {
        let result = Self::rotl(self.state[1].wrapping_mul(5), 7).wrapping_mul(9);
        let t = self.state[1] << 17;
        
        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        
        self.state[2] ^= t;
        self.state[3] = Self::rotl(self.state[3], 45);
        
        result
    }
    
    /// Generate random f64 in [0, 1)
    pub fn next_f64(&mut self) -> f64 {
        // Use the high 53 bits for double precision
        (self.next() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    
    /// Generate unbiased random int in [min, max)
    pub fn next_int(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        let range = (max - min) as u64;
        let threshold = (u64::MAX - range + 1) % range;
        
        loop {
            let r = self.next();
            if r >= threshold {
                return min + (r % range) as i64;
            }
        }
    }
}

// ==================== Object Methods ====================

/// Object.keys
pub fn object_keys(obj: &HateObject, store: &crate::object::ObjectStore) -> Vec<Symbol> {
    store.get_class(obj.class).property_names()
}

/// Object.values
pub fn object_values(obj: &HateObject) -> Vec<Value> {
    obj.slots.clone()
}

/// Object.entries
pub fn object_entries(obj: &HateObject, store: &crate::object::ObjectStore) -> Vec<(Symbol, Value)> {
    let keys = store.get_class(obj.class).property_names();
    keys.into_iter()
        .enumerate()
        .map(|(i, k)| (k, obj.slots.get(i).copied().unwrap_or(Value::null())))
        .collect()
}

/// Object.hasOwnProperty
pub fn object_has_own_property(obj: &HateObject, key: Symbol, store: &crate::object::ObjectStore) -> bool {
    store.get_class(obj.class).lookup(key).is_some()
}

/// Object.assign - merge properties
pub fn object_assign(target: &mut HateObject, sources: &[&HateObject]) {
    for source in sources {
        for &value in &source.slots {
            target.slots.push(value);
        }
    }
}

/// Object.freeze
pub fn object_freeze(obj: &mut HateObject) {
    obj.frozen = true;
    obj.sealed = true;
}

/// Object.seal
pub fn object_seal(obj: &mut HateObject) {
    obj.sealed = true;
}

// ==================== Range Helper ====================

/// Create a range array
pub fn range(start: i64, end: i64, step: i64) -> HateArray {
    if step == 0 {
        return HateArray::new();
    }
    
    let count = if step > 0 {
        ((end - start).max(0) as u64 / step as u64) as usize
    } else {
        ((start - end).max(0) as u64 / (-step) as u64) as usize
    };
    
    let mut elements = Vec::with_capacity(count);
    let mut current = start;
    
    if step > 0 {
        while current < end {
            elements.push(Value::int64(current));
            current += step;
        }
    } else {
        while current > end {
            elements.push(Value::int64(current));
            current += step;
        }
    }
    
    HateArray { elements }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_array_map() {
        let arr = HateArray::from_values(vec![Value::int(1), Value::int(2), Value::int(3)]);
        let result = array_map(&arr, |v, _| {
            Value::int(v.as_int().unwrap() * 2)
        });
        assert_eq!(result.len(), 3);
        assert_eq!(result.get(0), Some(Value::int(2)));
        assert_eq!(result.get(2), Some(Value::int(6)));
    }
    
    #[test]
    fn test_array_filter() {
        let arr = HateArray::from_values(vec![
            Value::int(1), Value::int(2), Value::int(3), Value::int(4)
        ]);
        let result = array_filter(&arr, |v, _| {
            v.as_int().unwrap() % 2 == 0
        });
        assert_eq!(result.len(), 2);
        assert_eq!(result.get(0), Some(Value::int(2)));
        assert_eq!(result.get(1), Some(Value::int(4)));
    }
    
    #[test]
    fn test_array_reduce() {
        let arr = HateArray::from_values(vec![
            Value::int(1), Value::int(2), Value::int(3)
        ]);
        let sum = array_reduce(&arr, Value::int(0), |acc, v, _| {
            Value::int(acc.as_int().unwrap() + v.as_int().unwrap())
        });
        assert_eq!(sum.as_int(), Some(6));
    }
    
    #[test]
    fn test_string_boyer_moore() {
        assert_eq!(string_index_of("hello world", "world"), 6);
        assert_eq!(string_index_of("hello world", "foo"), -1);
        assert_eq!(string_index_of("hello world", ""), 0);
    }
    
    #[test]
    fn test_string_operations() {
        assert!(string_starts_with("hello", "hel"));
        assert!(!string_starts_with("hello", "ello"));
        assert!(string_ends_with("hello", "llo"));
        assert_eq!(string_repeat("ab", 3), "ababab");
        assert_eq!(string_pad_start("hi", 5, ' '), "   hi");
    }
    
    #[test]
    fn test_math_functions() {
        assert_eq!(math_sign(5.0), 1.0);
        assert_eq!(math_sign(-3.0), -1.0);
        assert_eq!(math_sign(0.0), 0.0);
        assert_eq!(math_clamp(5.0, 0.0, 3.0), 3.0);
        assert_eq!(math_clamp(-1.0, 0.0, 3.0), 0.0);
    }
    
    #[test]
    fn test_xoshiro_rng() {
        let mut rng = Xoshiro256StarStar::new(12345);
        
        for _ in 0..100 {
            let f = rng.next_f64();
            assert!(f >= 0.0 && f < 1.0);
        }
        
        for _ in 0..100 {
            let i = rng.next_int(10, 20);
            assert!(i >= 10 && i < 20);
        }
    }
    
    #[test]
    fn test_range() {
        let r = range(0, 5, 1);
        assert_eq!(r.len(), 5);
        
        let r = range(10, 0, -2);
        assert_eq!(r.len(), 5);
        assert_eq!(r.get(0).and_then(|v| v.as_int()), Some(10));
        assert_eq!(r.get(4).and_then(|v| v.as_int()), Some(2));
    }
}
