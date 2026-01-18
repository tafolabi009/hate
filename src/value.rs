//! NaN-boxing value representation
//!
//! Uses the NaN space of IEEE 754 doubles to encode all Hate values in 8 bytes.
//!
//! # Layout
//! ```text
//! Doubles:     Normal IEEE 754 representation
//! Integers:    0x7FF8_0001_XXXX_XXXX (32-bit signed integer in lower bits)
//! Pointers:    0x7FFC_XXXX_XXXX_XXXX (48-bit pointer in lower bits)
//! True:        0x7FF8_0002_0000_0001
//! False:       0x7FF8_0002_0000_0000
//! Null:        0x7FF8_0003_0000_0000
//! ```

use std::fmt;
use std::ptr::NonNull;
use crate::gc::{GcObject, GcRef};

// NaN-boxing tags
const QNAN: u64 = 0x7FF8_0000_0000_0000;
const SIGN_BIT: u64 = 0x8000_0000_0000_0000;

// Type tags (in bits 48-51)
const TAG_INT: u64 = 0x0001_0000_0000;      // Integer
const TAG_BOOL: u64 = 0x0002_0000_0000;     // Boolean
const TAG_NULL: u64 = 0x0003_0000_0000;     // Null
const TAG_PTR: u64 = 0x0004_0000_0000;      // Heap pointer
const TAG_NATIVE: u64 = 0x0005_0000_0000;   // Native function (symbol index in lower bits)
const TAG_STRING: u64 = 0x0006_0000_0000;   // String (symbol index in lower bits)

// Masks
const PTR_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;  // Lower 48 bits for pointer
const INT_MASK: u64 = 0x0000_0000_FFFF_FFFF;  // Lower 32 bits for integer

/// A NaN-boxed value representing any Hate value in 8 bytes
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Value(pub u64);

impl Value {
    // ==================== Constructors ====================
    
    /// Create a null value
    #[inline(always)]
    pub const fn null() -> Self {
        Value(QNAN | TAG_NULL)
    }
    
    /// Create a boolean value
    #[inline(always)]
    pub const fn bool(b: bool) -> Self {
        Value(QNAN | TAG_BOOL | (b as u64))
    }
    
    /// Create a true value
    #[inline(always)]
    pub const fn r#true() -> Self {
        Value(QNAN | TAG_BOOL | 1)
    }
    
    /// Create a false value
    #[inline(always)]
    pub const fn r#false() -> Self {
        Value(QNAN | TAG_BOOL)
    }
    
    /// Create an integer value (32-bit signed, unboxed)
    #[inline(always)]
    pub const fn int(i: i32) -> Self {
        Value(QNAN | TAG_INT | (i as u32 as u64))
    }
    
    /// Create a 64-bit integer value (boxed on heap if needed)
    #[inline(always)]
    pub fn int64(i: i64) -> Self {
        // If it fits in 32 bits, use unboxed representation
        if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
            Value::int(i as i32)
        } else {
            // For larger integers, we'd box them - for now, truncate
            // TODO: Implement BigInt boxing
            Value::int(i as i32)
        }
    }
    
    /// Create a float value (f64)
    #[inline(always)]
    pub fn float(f: f64) -> Self {
        // Check for NaN - we need to canonicalize it
        if f.is_nan() {
            // Use a canonical NaN that doesn't collide with our tags
            Value(0x7FF8_0000_0000_0000)
        } else {
            Value(f.to_bits())
        }
    }
    
    /// Create a pointer value to a GC-managed object
    #[inline(always)]
    pub fn ptr<T: GcObject>(ptr: GcRef<T>) -> Self {
        let addr = ptr.as_ptr() as u64;
        debug_assert!(addr & !PTR_MASK == 0, "Pointer exceeds 48 bits");
        Value(QNAN | TAG_PTR | addr)
    }
    
    /// Create a pointer value from a raw pointer
    #[inline(always)]
    pub unsafe fn from_raw_ptr(ptr: *mut u8) -> Self {
        let addr = ptr as u64;
        debug_assert!(addr & !PTR_MASK == 0, "Pointer exceeds 48 bits");
        Value(QNAN | TAG_PTR | addr)
    }
    
    /// Create a native function marker (stores the symbol index for the function name)
    #[inline(always)]
    pub fn native_fn(symbol_idx: u32) -> Self {
        Value(QNAN | TAG_NATIVE | (symbol_idx as u64))
    }
    
    /// Create a string value (stores the symbol index for the interned string)
    #[inline(always)]
    pub fn string(symbol_idx: u32) -> Self {
        Value(QNAN | TAG_STRING | (symbol_idx as u64))
    }
    
    // ==================== Type Checking ====================
    
    /// Check if this value is a float (not a tagged value)
    #[inline(always)]
    pub fn is_float(&self) -> bool {
        // A float is any value where the NaN bits don't match our quiet NaN pattern
        (self.0 & QNAN) != QNAN
    }
    
    /// Check if this value is an integer
    #[inline(always)]
    pub fn is_int(&self) -> bool {
        (self.0 & (QNAN | TAG_INT | TAG_BOOL | TAG_NULL | TAG_PTR)) == (QNAN | TAG_INT)
    }
    
    /// Check if this value is a boolean
    #[inline(always)]
    pub fn is_bool(&self) -> bool {
        (self.0 & (QNAN | 0xFFFF_FFFF_FFFE)) == (QNAN | TAG_BOOL)
    }
    
    /// Check if this value is null
    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.0 == (QNAN | TAG_NULL)
    }
    
    /// Check if this value is a pointer to a heap object
    #[inline(always)]
    pub fn is_ptr(&self) -> bool {
        (self.0 & (QNAN | TAG_PTR)) == (QNAN | TAG_PTR) && !self.is_native_fn()
    }
    
    /// Check if this value is a native function marker
    #[inline(always)]
    pub fn is_native_fn(&self) -> bool {
        (self.0 & (QNAN | 0xFFFF_0000_0000)) == (QNAN | TAG_NATIVE)
    }
    
    /// Check if this value is a string
    #[inline(always)]
    pub fn is_string(&self) -> bool {
        (self.0 & (QNAN | 0xFFFF_0000_0000)) == (QNAN | TAG_STRING)
    }
    
    /// Get the native function symbol index (if this is a native function)
    #[inline(always)]
    pub fn as_native_fn_index(&self) -> Option<u32> {
        if self.is_native_fn() {
            Some((self.0 & INT_MASK) as u32)
        } else {
            None
        }
    }
    
    /// Get the string symbol index (if this is a string)
    #[inline(always)]
    pub fn as_string_index(&self) -> Option<u32> {
        if self.is_string() {
            Some((self.0 & INT_MASK) as u32)
        } else {
            None
        }
    }
    
    /// Get the string value (if this is a string)
    #[inline(always)]
    pub fn as_string(&self) -> Option<&'static str> {
        if self.is_string() {
            let idx = (self.0 & INT_MASK) as u32;
            use crate::intern::Symbol;
            let symbol = unsafe { std::mem::transmute::<u32, Symbol>(idx) };
            Some(symbol.as_str())
        } else {
            None
        }
    }
    
    /// Check if this value is an array
    #[inline(always)]
    pub fn is_array(&self) -> bool {
        // TODO: Check GC object type when implemented
        false
    }
    
    /// Check if this value is an object
    #[inline(always)]
    pub fn is_object(&self) -> bool {
        // TODO: Check GC object type when implemented
        false
    }
    
    /// Check if this value is a function
    #[inline(always)]
    pub fn is_function(&self) -> bool {
        // TODO: Check GC object type when implemented
        false
    }
    
    /// Check if this value is a number (int or float)
    #[inline(always)]
    pub fn is_number(&self) -> bool {
        self.is_float() || self.is_int()
    }
    
    /// Check if this value is truthy
    #[inline(always)]
    pub fn is_truthy(&self) -> bool {
        match self.0 {
            x if x == (QNAN | TAG_NULL) => false,
            x if x == (QNAN | TAG_BOOL) => false, // false
            x if x == (QNAN | TAG_BOOL | 1) => true, // true
            x if (x & (QNAN | TAG_INT)) == (QNAN | TAG_INT) => {
                // Integer: truthy if non-zero
                (x & INT_MASK) != 0
            }
            _ if self.is_float() => {
                // Float: truthy if non-zero and not NaN
                let f = self.as_float_unchecked();
                f != 0.0 && !f.is_nan()
            }
            _ => true, // Objects are truthy
        }
    }
    
    // ==================== Extractors ====================
    
    /// Get the float value (unchecked)
    #[inline(always)]
    pub fn as_float_unchecked(&self) -> f64 {
        f64::from_bits(self.0)
    }
    
    /// Get the float value
    #[inline(always)]
    pub fn as_float(&self) -> Option<f64> {
        if self.is_float() {
            Some(f64::from_bits(self.0))
        } else {
            None
        }
    }
    
    /// Get the integer value (unchecked)
    #[inline(always)]
    pub fn as_int_unchecked(&self) -> i32 {
        (self.0 & INT_MASK) as i32
    }
    
    /// Get the integer value
    #[inline(always)]
    pub fn as_int(&self) -> Option<i32> {
        if self.is_int() {
            Some(self.as_int_unchecked())
        } else {
            None
        }
    }
    
    /// Get the boolean value (unchecked)
    #[inline(always)]
    pub fn as_bool_unchecked(&self) -> bool {
        (self.0 & 1) != 0
    }
    
    /// Get the boolean value
    #[inline(always)]
    pub fn as_bool(&self) -> Option<bool> {
        if self.is_bool() {
            Some(self.as_bool_unchecked())
        } else {
            None
        }
    }
    
    /// Get the pointer value (unchecked)
    #[inline(always)]
    pub fn as_ptr_unchecked<T>(&self) -> *mut T {
        (self.0 & PTR_MASK) as *mut T
    }
    
    /// Get the pointer value
    #[inline(always)]
    pub fn as_ptr<T>(&self) -> Option<NonNull<T>> {
        if self.is_ptr() {
            NonNull::new(self.as_ptr_unchecked())
        } else {
            None
        }
    }
    
    /// Convert to a number (int or float)
    #[inline(always)]
    pub fn to_number(&self) -> Option<f64> {
        if self.is_float() {
            Some(self.as_float_unchecked())
        } else if self.is_int() {
            Some(self.as_int_unchecked() as f64)
        } else {
            None
        }
    }
    
    // ==================== Arithmetic Operations ====================
    
    /// Add two values
    #[inline]
    pub fn add(self, other: Value) -> Option<Value> {
        // Fast path: both integers
        if self.is_int() && other.is_int() {
            let a = self.as_int_unchecked();
            let b = other.as_int_unchecked();
            // Check for overflow
            if let Some(result) = a.checked_add(b) {
                return Some(Value::int(result));
            }
            // Overflow: promote to float
            return Some(Value::float(a as f64 + b as f64));
        }
        
        // String concatenation
        if self.is_string() || other.is_string() {
            use crate::intern::intern;
            let a_str = if let Some(s) = self.as_string() {
                s.to_string()
            } else {
                format!("{}", self)
            };
            let b_str = if let Some(s) = other.as_string() {
                s.to_string()
            } else {
                format!("{}", other)
            };
            let result = a_str + &b_str;
            let symbol = intern(&result);
            return Some(Value::string(symbol.index()));
        }
        
        // Slow path: convert to floats
        let a = self.to_number()?;
        let b = other.to_number()?;
        Some(Value::float(a + b))
    }
    
    /// Subtract two values
    #[inline]
    pub fn sub(self, other: Value) -> Option<Value> {
        if self.is_int() && other.is_int() {
            let a = self.as_int_unchecked();
            let b = other.as_int_unchecked();
            if let Some(result) = a.checked_sub(b) {
                return Some(Value::int(result));
            }
            return Some(Value::float(a as f64 - b as f64));
        }
        
        let a = self.to_number()?;
        let b = other.to_number()?;
        Some(Value::float(a - b))
    }
    
    /// Multiply two values
    #[inline]
    pub fn mul(self, other: Value) -> Option<Value> {
        if self.is_int() && other.is_int() {
            let a = self.as_int_unchecked();
            let b = other.as_int_unchecked();
            if let Some(result) = a.checked_mul(b) {
                return Some(Value::int(result));
            }
            return Some(Value::float(a as f64 * b as f64));
        }
        
        let a = self.to_number()?;
        let b = other.to_number()?;
        Some(Value::float(a * b))
    }
    
    /// Divide two values (always returns float)
    #[inline]
    pub fn div(self, other: Value) -> Option<Value> {
        let a = self.to_number()?;
        let b = other.to_number()?;
        Some(Value::float(a / b))
    }
    
    /// Modulo two values
    #[inline]
    pub fn modulo(self, other: Value) -> Option<Value> {
        if self.is_int() && other.is_int() {
            let a = self.as_int_unchecked();
            let b = other.as_int_unchecked();
            if b != 0 {
                return Some(Value::int(a % b));
            }
            return Some(Value::float(f64::NAN));
        }
        
        let a = self.to_number()?;
        let b = other.to_number()?;
        Some(Value::float(a % b))
    }
    
    /// Negate a value
    #[inline]
    pub fn neg(self) -> Option<Value> {
        if self.is_int() {
            let a = self.as_int_unchecked();
            if let Some(result) = a.checked_neg() {
                return Some(Value::int(result));
            }
            return Some(Value::float(-(a as f64)));
        }
        
        let a = self.to_number()?;
        Some(Value::float(-a))
    }
    
    // ==================== Comparison Operations ====================
    
    /// Check equality
    #[inline]
    pub fn eq(self, other: Value) -> bool {
        // Fast path: same bits
        if self.0 == other.0 {
            // Special case: NaN != NaN
            if self.is_float() && self.as_float_unchecked().is_nan() {
                return false;
            }
            return true;
        }
        
        // Compare int and float
        if self.is_number() && other.is_number() {
            let a = self.to_number().unwrap();
            let b = other.to_number().unwrap();
            return a == b;
        }
        
        false
    }
    
    /// Less than comparison
    #[inline]
    pub fn lt(self, other: Value) -> Option<bool> {
        let a = self.to_number()?;
        let b = other.to_number()?;
        Some(a < b)
    }
    
    /// Less than or equal comparison
    #[inline]
    pub fn le(self, other: Value) -> Option<bool> {
        let a = self.to_number()?;
        let b = other.to_number()?;
        Some(a <= b)
    }
    
    /// Greater than comparison
    #[inline]
    pub fn gt(self, other: Value) -> Option<bool> {
        let a = self.to_number()?;
        let b = other.to_number()?;
        Some(a > b)
    }
    
    /// Greater than or equal comparison
    #[inline]
    pub fn ge(self, other: Value) -> Option<bool> {
        let a = self.to_number()?;
        let b = other.to_number()?;
        Some(a >= b)
    }
    
    // ==================== Bitwise Operations ====================
    
    /// Bitwise AND
    #[inline]
    pub fn bit_and(self, other: Value) -> Option<Value> {
        let a = self.as_int()?;
        let b = other.as_int()?;
        Some(Value::int(a & b))
    }
    
    /// Bitwise OR
    #[inline]
    pub fn bit_or(self, other: Value) -> Option<Value> {
        let a = self.as_int()?;
        let b = other.as_int()?;
        Some(Value::int(a | b))
    }
    
    /// Bitwise XOR
    #[inline]
    pub fn bit_xor(self, other: Value) -> Option<Value> {
        let a = self.as_int()?;
        let b = other.as_int()?;
        Some(Value::int(a ^ b))
    }
    
    /// Bitwise NOT
    #[inline]
    pub fn bit_not(self) -> Option<Value> {
        let a = self.as_int()?;
        Some(Value::int(!a))
    }
    
    /// Left shift
    #[inline]
    pub fn shl(self, other: Value) -> Option<Value> {
        let a = self.as_int()?;
        let b = other.as_int()? as u32;
        Some(Value::int(a.wrapping_shl(b)))
    }
    
    /// Right shift (arithmetic)
    #[inline]
    pub fn shr(self, other: Value) -> Option<Value> {
        let a = self.as_int()?;
        let b = other.as_int()? as u32;
        Some(Value::int(a.wrapping_shr(b)))
    }
    
    // ==================== Type Information ====================
    
    /// Get the type name of this value
    pub fn type_name(&self) -> &'static str {
        if self.is_null() {
            "null"
        } else if self.is_bool() {
            "bool"
        } else if self.is_int() {
            "i32"
        } else if self.is_float() {
            "f64"
        } else if self.is_string() {
            "string"
        } else if self.is_native_fn() {
            "native_fn"
        } else if self.is_ptr() {
            "object"
        } else {
            "unknown"
        }
    }
    
    /// Get the raw bits of this value (for debugging)
    #[inline(always)]
    pub fn bits(&self) -> u64 {
        self.0
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::null()
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "null")
        } else if self.is_bool() {
            write!(f, "{}", self.as_bool_unchecked())
        } else if self.is_int() {
            write!(f, "{}", self.as_int_unchecked())
        } else if self.is_float() {
            write!(f, "{}", self.as_float_unchecked())
        } else if self.is_ptr() {
            write!(f, "<object@{:p}>", self.as_ptr_unchecked::<u8>())
        } else {
            write!(f, "<unknown:{:#018x}>", self.0)
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "null")
        } else if self.is_bool() {
            write!(f, "{}", self.as_bool_unchecked())
        } else if self.is_int() {
            write!(f, "{}", self.as_int_unchecked())
        } else if self.is_float() {
            let val = self.as_float_unchecked();
            if val.fract() == 0.0 && val.abs() < 1e15 {
                write!(f, "{:.1}", val)
            } else {
                write!(f, "{}", val)
            }
        } else if self.is_string() {
            if let Some(s) = self.as_string() {
                write!(f, "{}", s)
            } else {
                write!(f, "<string>")
            }
        } else if self.is_native_fn() {
            write!(f, "<native fn>")
        } else if self.is_ptr() {
            write!(f, "<object>")
        } else {
            write!(f, "<unknown>")
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        Value::eq(*self, *other)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::bool(b)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::int(i)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::int64(i)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::float(f)
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_null() {
        let v = Value::null();
        assert!(v.is_null());
        assert!(!v.is_truthy());
        assert_eq!(v.type_name(), "null");
    }
    
    #[test]
    fn test_bool() {
        let t = Value::bool(true);
        let f = Value::bool(false);
        
        assert!(t.is_bool());
        assert!(f.is_bool());
        assert!(t.is_truthy());
        assert!(!f.is_truthy());
        assert_eq!(t.as_bool(), Some(true));
        assert_eq!(f.as_bool(), Some(false));
    }
    
    #[test]
    fn test_int() {
        let v = Value::int(42);
        assert!(v.is_int());
        assert!(v.is_number());
        assert!(v.is_truthy());
        assert_eq!(v.as_int(), Some(42));
        assert_eq!(v.to_number(), Some(42.0));
        
        let neg = Value::int(-100);
        assert_eq!(neg.as_int(), Some(-100));
        
        let zero = Value::int(0);
        assert!(!zero.is_truthy());
    }
    
    #[test]
    fn test_float() {
        let v = Value::float(3.14);
        assert!(v.is_float());
        assert!(v.is_number());
        assert!(v.is_truthy());
        assert!((v.as_float().unwrap() - 3.14).abs() < f64::EPSILON);
        
        let zero = Value::float(0.0);
        assert!(!zero.is_truthy());
    }
    
    #[test]
    fn test_arithmetic() {
        let a = Value::int(10);
        let b = Value::int(3);
        
        assert_eq!(a.add(b).unwrap().as_int(), Some(13));
        assert_eq!(a.sub(b).unwrap().as_int(), Some(7));
        assert_eq!(a.mul(b).unwrap().as_int(), Some(30));
        assert!((a.div(b).unwrap().as_float().unwrap() - 3.333333).abs() < 0.001);
        assert_eq!(a.modulo(b).unwrap().as_int(), Some(1));
        assert_eq!(a.neg().unwrap().as_int(), Some(-10));
    }
    
    #[test]
    fn test_overflow() {
        let max = Value::int(i32::MAX);
        let one = Value::int(1);
        
        // Overflow should promote to float
        let result = max.add(one).unwrap();
        assert!(result.is_float());
        assert!((result.as_float().unwrap() - (i32::MAX as f64 + 1.0)).abs() < 1.0);
    }
    
    #[test]
    fn test_comparison() {
        let a = Value::int(10);
        let b = Value::int(20);
        let c = Value::float(10.0);
        
        assert!(a.eq(a));
        assert!(!a.eq(b));
        assert!(a.eq(c)); // int and float with same value
        
        assert!(a.lt(b).unwrap());
        assert!(b.gt(a).unwrap());
    }
    
    #[test]
    fn test_bitwise() {
        let a = Value::int(0b1010);
        let b = Value::int(0b1100);
        
        assert_eq!(a.bit_and(b).unwrap().as_int(), Some(0b1000));
        assert_eq!(a.bit_or(b).unwrap().as_int(), Some(0b1110));
        assert_eq!(a.bit_xor(b).unwrap().as_int(), Some(0b0110));
    }
    
    #[test]
    fn test_size() {
        // Ensure Value is exactly 8 bytes
        assert_eq!(std::mem::size_of::<Value>(), 8);
    }
}
