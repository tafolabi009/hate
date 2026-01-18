//! Hate Runtime and Standard Library
//!
//! Built-in functions and types available globally

use crate::value::Value;
use crate::gc::GC;
use crate::error::{HateError, HateResult, Span};
use std::time::{SystemTime, UNIX_EPOCH, Instant};

/// Native function type
pub type NativeFn = fn(&mut Runtime, &[Value]) -> HateResult<Value>;

/// A native function record
pub struct NativeFunction {
    pub name: &'static str,
    pub arity: u8,  // 255 = variadic
    pub func: NativeFn,
}

/// Runtime environment with standard library
pub struct Runtime {
    /// Garbage collector (shared with VM)
    pub gc: GC,
    /// Program start time for performance measurements
    start_time: Instant,
    /// Random number generator state (xorshift64)
    rng_state: u64,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            gc: GC::new(),
            start_time: Instant::now(),
            rng_state: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }
    
    /// Get all built-in functions
    pub fn builtins() -> Vec<NativeFunction> {
        vec![
            // I/O
            NativeFunction { name: "print", arity: 255, func: native_print },
            NativeFunction { name: "println", arity: 255, func: native_println },
            NativeFunction { name: "input", arity: 1, func: native_input },
            
            // Type conversion
            NativeFunction { name: "int", arity: 1, func: native_int },
            NativeFunction { name: "float", arity: 1, func: native_float },
            NativeFunction { name: "str", arity: 1, func: native_str },
            NativeFunction { name: "bool", arity: 1, func: native_bool },
            
            // Type checking
            NativeFunction { name: "type", arity: 1, func: native_type },
            NativeFunction { name: "isInt", arity: 1, func: native_is_int },
            NativeFunction { name: "isFloat", arity: 1, func: native_is_float },
            NativeFunction { name: "isStr", arity: 1, func: native_is_str },
            NativeFunction { name: "isBool", arity: 1, func: native_is_bool },
            NativeFunction { name: "isNull", arity: 1, func: native_is_null },
            NativeFunction { name: "isArray", arity: 1, func: native_is_array },
            NativeFunction { name: "isObject", arity: 1, func: native_is_object },
            NativeFunction { name: "isFunction", arity: 1, func: native_is_function },
            
            // Math
            NativeFunction { name: "abs", arity: 1, func: native_abs },
            NativeFunction { name: "floor", arity: 1, func: native_floor },
            NativeFunction { name: "ceil", arity: 1, func: native_ceil },
            NativeFunction { name: "round", arity: 1, func: native_round },
            NativeFunction { name: "sqrt", arity: 1, func: native_sqrt },
            NativeFunction { name: "pow", arity: 2, func: native_pow },
            NativeFunction { name: "sin", arity: 1, func: native_sin },
            NativeFunction { name: "cos", arity: 1, func: native_cos },
            NativeFunction { name: "tan", arity: 1, func: native_tan },
            NativeFunction { name: "log", arity: 1, func: native_log },
            NativeFunction { name: "log10", arity: 1, func: native_log10 },
            NativeFunction { name: "exp", arity: 1, func: native_exp },
            NativeFunction { name: "min", arity: 255, func: native_min },
            NativeFunction { name: "max", arity: 255, func: native_max },
            NativeFunction { name: "random", arity: 0, func: native_random },
            NativeFunction { name: "randomInt", arity: 2, func: native_random_int },
            
            // String functions
            NativeFunction { name: "len", arity: 1, func: native_len },
            NativeFunction { name: "charAt", arity: 2, func: native_char_at },
            NativeFunction { name: "substr", arity: 3, func: native_substr },
            NativeFunction { name: "indexOf", arity: 2, func: native_index_of },
            NativeFunction { name: "split", arity: 2, func: native_split },
            NativeFunction { name: "trim", arity: 1, func: native_trim },
            NativeFunction { name: "upper", arity: 1, func: native_upper },
            NativeFunction { name: "lower", arity: 1, func: native_lower },
            NativeFunction { name: "startsWith", arity: 2, func: native_starts_with },
            NativeFunction { name: "endsWith", arity: 2, func: native_ends_with },
            NativeFunction { name: "replace", arity: 3, func: native_replace },
            
            // Array functions
            NativeFunction { name: "push", arity: 2, func: native_push },
            NativeFunction { name: "pop", arity: 1, func: native_pop },
            NativeFunction { name: "shift", arity: 1, func: native_shift },
            NativeFunction { name: "unshift", arity: 2, func: native_unshift },
            NativeFunction { name: "slice", arity: 3, func: native_slice },
            NativeFunction { name: "concat", arity: 2, func: native_concat },
            NativeFunction { name: "reverse", arity: 1, func: native_reverse },
            NativeFunction { name: "sort", arity: 1, func: native_sort },
            NativeFunction { name: "join", arity: 2, func: native_join },
            NativeFunction { name: "range", arity: 3, func: native_range },
            
            // Object functions
            NativeFunction { name: "keys", arity: 1, func: native_keys },
            NativeFunction { name: "values", arity: 1, func: native_values },
            NativeFunction { name: "entries", arity: 1, func: native_entries },
            NativeFunction { name: "hasKey", arity: 2, func: native_has_key },
            
            // Time
            NativeFunction { name: "time", arity: 0, func: native_time },
            NativeFunction { name: "timeMs", arity: 0, func: native_time_ms },
            NativeFunction { name: "sleep", arity: 1, func: native_sleep },
            
            // Assertions
            NativeFunction { name: "assert", arity: 1, func: native_assert },
            NativeFunction { name: "assertEq", arity: 2, func: native_assert_eq },
            
            // Debug
            NativeFunction { name: "debug", arity: 1, func: native_debug },
            NativeFunction { name: "panic", arity: 1, func: native_panic },
        ]
    }
    
    /// XorShift64 random number generator
    pub fn random(&mut self) -> f64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as f64) / (u64::MAX as f64)
    }
    
    /// Random integer in range [min, max)
    pub fn random_int(&mut self, min: i32, max: i32) -> i32 {
        let r = self.random();
        min + ((r * (max - min) as f64) as i32)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== I/O Functions ====================

fn native_print(rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let _ = rt; // Unused
    for (i, arg) in args.iter().enumerate() {
        if i > 0 { print!(" "); }
        print!("{}", arg);
    }
    std::io::Write::flush(&mut std::io::stdout()).ok();
    Ok(Value::null())
}

fn native_println(rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    native_print(rt, args)?;
    println!();
    Ok(Value::null())
}

fn native_input(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    if !args.is_empty() {
        print!("{}", args[0]);
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok();
    // TODO: Return as GC-managed string
    Ok(Value::null())
}

// ==================== Type Conversion ====================

fn native_int(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    if value.is_int() {
        Ok(value)
    } else if value.is_float() {
        Ok(Value::int(value.as_float_unchecked() as i32))
    } else if value.is_bool() {
        Ok(Value::int(if value.as_bool_unchecked() { 1 } else { 0 }))
    } else {
        Ok(Value::int(0))
    }
}

fn native_float(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    if value.is_float() {
        Ok(value)
    } else if value.is_int() {
        Ok(Value::float(value.as_int_unchecked() as f64))
    } else if value.is_bool() {
        Ok(Value::float(if value.as_bool_unchecked() { 1.0 } else { 0.0 }))
    } else {
        Ok(Value::float(0.0))
    }
}

fn native_str(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    // TODO: Create GC-managed string
    let _ = format!("{}", value);
    Ok(Value::null())
}

fn native_bool(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    Ok(Value::bool(value.is_truthy()))
}

// ==================== Type Checking ====================

fn native_type(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    // TODO: Return type string via GC
    let _ = value.type_name();
    Ok(Value::null())
}

fn native_is_int(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    Ok(Value::bool(value.is_int()))
}

fn native_is_float(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    Ok(Value::bool(value.is_float()))
}

fn native_is_str(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    Ok(Value::bool(value.is_string()))
}

fn native_is_bool(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    Ok(Value::bool(value.is_bool()))
}

fn native_is_null(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    Ok(Value::bool(value.is_null()))
}

fn native_is_array(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    Ok(Value::bool(value.is_array()))
}

fn native_is_object(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    Ok(Value::bool(value.is_object()))
}

fn native_is_function(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    Ok(Value::bool(value.is_function()))
}

// ==================== Math Functions ====================

fn native_abs(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    if value.is_int() {
        Ok(Value::int(value.as_int_unchecked().abs()))
    } else if value.is_float() {
        Ok(Value::float(value.as_float_unchecked().abs()))
    } else {
        Err(HateError::InvalidOperator {
            op: "abs".to_string(),
            left: value.type_name().to_string(),
            right: String::new(),
            span: Span::empty(),
        })
    }
}

fn native_floor(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    if value.is_float() {
        Ok(Value::float(value.as_float_unchecked().floor()))
    } else if value.is_int() {
        Ok(value)
    } else {
        Ok(Value::float(0.0))
    }
}

fn native_ceil(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    if value.is_float() {
        Ok(Value::float(value.as_float_unchecked().ceil()))
    } else if value.is_int() {
        Ok(value)
    } else {
        Ok(Value::float(0.0))
    }
}

fn native_round(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    if value.is_float() {
        Ok(Value::float(value.as_float_unchecked().round()))
    } else if value.is_int() {
        Ok(value)
    } else {
        Ok(Value::float(0.0))
    }
}

fn native_sqrt(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    let n = if value.is_float() {
        value.as_float_unchecked()
    } else if value.is_int() {
        value.as_int_unchecked() as f64
    } else {
        return Ok(Value::float(f64::NAN));
    };
    Ok(Value::float(n.sqrt()))
}

fn native_pow(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let base = args.get(0).copied().unwrap_or(Value::null());
    let exp = args.get(1).copied().unwrap_or(Value::null());
    
    let b = if base.is_float() {
        base.as_float_unchecked()
    } else if base.is_int() {
        base.as_int_unchecked() as f64
    } else {
        return Ok(Value::float(f64::NAN));
    };
    
    let e = if exp.is_float() {
        exp.as_float_unchecked()
    } else if exp.is_int() {
        exp.as_int_unchecked() as f64
    } else {
        return Ok(Value::float(f64::NAN));
    };
    
    Ok(Value::float(b.powf(e)))
}

fn native_sin(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    let n = if value.is_float() {
        value.as_float_unchecked()
    } else if value.is_int() {
        value.as_int_unchecked() as f64
    } else {
        return Ok(Value::float(f64::NAN));
    };
    Ok(Value::float(n.sin()))
}

fn native_cos(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    let n = if value.is_float() {
        value.as_float_unchecked()
    } else if value.is_int() {
        value.as_int_unchecked() as f64
    } else {
        return Ok(Value::float(f64::NAN));
    };
    Ok(Value::float(n.cos()))
}

fn native_tan(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    let n = if value.is_float() {
        value.as_float_unchecked()
    } else if value.is_int() {
        value.as_int_unchecked() as f64
    } else {
        return Ok(Value::float(f64::NAN));
    };
    Ok(Value::float(n.tan()))
}

fn native_log(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    let n = if value.is_float() {
        value.as_float_unchecked()
    } else if value.is_int() {
        value.as_int_unchecked() as f64
    } else {
        return Ok(Value::float(f64::NAN));
    };
    Ok(Value::float(n.ln()))
}

fn native_log10(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    let n = if value.is_float() {
        value.as_float_unchecked()
    } else if value.is_int() {
        value.as_int_unchecked() as f64
    } else {
        return Ok(Value::float(f64::NAN));
    };
    Ok(Value::float(n.log10()))
}

fn native_exp(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    let n = if value.is_float() {
        value.as_float_unchecked()
    } else if value.is_int() {
        value.as_int_unchecked() as f64
    } else {
        return Ok(Value::float(f64::NAN));
    };
    Ok(Value::float(n.exp()))
}

fn native_min(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    if args.is_empty() {
        return Ok(Value::null());
    }
    let mut min = args[0];
    for &arg in &args[1..] {
        if arg.lt(min).unwrap_or(false) {
            min = arg;
        }
    }
    Ok(min)
}

fn native_max(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    if args.is_empty() {
        return Ok(Value::null());
    }
    let mut max = args[0];
    for &arg in &args[1..] {
        if arg.gt(max).unwrap_or(false) {
            max = arg;
        }
    }
    Ok(max)
}

fn native_random(rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    Ok(Value::float(rt.random()))
}

fn native_random_int(rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let min = args.get(0).and_then(|v| v.as_int()).unwrap_or(0);
    let max = args.get(1).and_then(|v| v.as_int()).unwrap_or(100);
    Ok(Value::int(rt.random_int(min, max)))
}

// ==================== String Functions ====================

fn native_len(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    // TODO: Get actual string/array length from GC
    let _ = value;
    Ok(Value::int(0))
}

fn native_char_at(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_substr(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_index_of(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::int(-1))
}

fn native_split(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_trim(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_upper(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_lower(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_starts_with(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::bool(false))
}

fn native_ends_with(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::bool(false))
}

fn native_replace(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

// ==================== Array Functions ====================

fn native_push(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_pop(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_shift(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_unshift(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_slice(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_concat(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_reverse(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_sort(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_join(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_range(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

// ==================== Object Functions ====================

fn native_keys(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_values(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_entries(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::null())
}

fn native_has_key(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    // TODO: Implement
    Ok(Value::bool(false))
}

// ==================== Time Functions ====================

fn native_time(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    Ok(Value::float(secs as f64))
}

fn native_time_ms(_rt: &mut Runtime, _args: &[Value]) -> HateResult<Value> {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    Ok(Value::float(ms as f64))
}

fn native_sleep(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let ms = args.get(0).and_then(|v| v.as_int()).unwrap_or(0) as u64;
    std::thread::sleep(std::time::Duration::from_millis(ms));
    Ok(Value::null())
}

// ==================== Assertions ====================

fn native_assert(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    if !value.is_truthy() {
        return Err(HateError::Internal("Assertion failed".to_string()));
    }
    Ok(Value::null())
}

fn native_assert_eq(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let a = args.get(0).copied().unwrap_or(Value::null());
    let b = args.get(1).copied().unwrap_or(Value::null());
    if !a.eq(b) {
        return Err(HateError::Internal(format!(
            "Assertion failed: {} != {}", a, b
        )));
    }
    Ok(Value::null())
}

// ==================== Debug ====================

fn native_debug(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let value = args.get(0).copied().unwrap_or(Value::null());
    eprintln!("[DEBUG] {:?} = {}", value.type_name(), value);
    Ok(value)
}

fn native_panic(_rt: &mut Runtime, args: &[Value]) -> HateResult<Value> {
    let msg = args.get(0).copied().unwrap_or(Value::null());
    Err(HateError::Internal(format!("panic: {}", msg)))
}

// ==================== Math Constants Module ====================

/// Math module with constants
pub struct MathModule;

impl MathModule {
    pub const PI: f64 = std::f64::consts::PI;
    pub const E: f64 = std::f64::consts::E;
    pub const TAU: f64 = std::f64::consts::TAU;
    pub const SQRT2: f64 = std::f64::consts::SQRT_2;
    pub const LN2: f64 = std::f64::consts::LN_2;
    pub const LN10: f64 = std::f64::consts::LN_10;
    pub const INFINITY: f64 = f64::INFINITY;
    pub const NEG_INFINITY: f64 = f64::NEG_INFINITY;
    pub const NAN: f64 = f64::NAN;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_random() {
        let mut rt = Runtime::new();
        let r1 = rt.random();
        let r2 = rt.random();
        assert!(r1 >= 0.0 && r1 < 1.0);
        assert!(r2 >= 0.0 && r2 < 1.0);
        assert_ne!(r1, r2);
    }
    
    #[test]
    fn test_random_int() {
        let mut rt = Runtime::new();
        for _ in 0..100 {
            let r = rt.random_int(10, 20);
            assert!(r >= 10 && r < 20);
        }
    }
    
    #[test]
    fn test_abs() {
        let mut rt = Runtime::new();
        let args = [Value::int(-42)];
        let result = native_abs(&mut rt, &args).unwrap();
        assert_eq!(result.as_int(), Some(42));
        
        let args = [Value::float(-3.14)];
        let result = native_abs(&mut rt, &args).unwrap();
        assert!((result.as_float().unwrap() - 3.14).abs() < 0.0001);
    }
    
    #[test]
    fn test_min_max() {
        let mut rt = Runtime::new();
        
        let args = [Value::int(3), Value::int(1), Value::int(4), Value::int(1), Value::int(5)];
        let min = native_min(&mut rt, &args).unwrap();
        let max = native_max(&mut rt, &args).unwrap();
        
        assert_eq!(min.as_int(), Some(1));
        assert_eq!(max.as_int(), Some(5));
    }
    
    #[test]
    fn test_trig() {
        let mut rt = Runtime::new();
        
        let args = [Value::float(0.0)];
        let sin = native_sin(&mut rt, &args).unwrap();
        let cos = native_cos(&mut rt, &args).unwrap();
        
        assert!((sin.as_float().unwrap() - 0.0).abs() < 0.0001);
        assert!((cos.as_float().unwrap() - 1.0).abs() < 0.0001);
    }
}
