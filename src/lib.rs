use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::panic::{catch_unwind, AssertUnwindSafe};
use zeroize::Zeroizing;

/// Maximum raw JSON byte length accepted at every FFI entry point (guards against OOM from unbounded serde parsing).
pub(crate) const MAX_JSON_BYTES: usize = 1_048_576; // 1 MiB
/// Maximum number of field-element inputs for Poseidon hashing.
pub(crate) const MAX_POSEIDON_FIELD_INPUTS: usize = 32;
/// Maximum bit-array length for Poseidon-over-bits hashing and EdDSA signing.
pub(crate) const MAX_BITS_LEN: usize = 65_536;

pub mod babyjub;
pub mod blake;
pub mod eddsa;
pub mod poseidon_hash;

pub use eddsa::{sign_eddsa, verify_eddsa};
pub use poseidon_hash::poseidon_hash_bits;

fn write_error_json(output_json: *mut *mut c_char, message: &str) -> c_int {
    let payload = serde_json::json!({"success": false, "error": message}).to_string();
    unsafe {
        match CString::new(payload) {
            Ok(cstr) => {
                *output_json = cstr.into_raw();
                -1
            }
            Err(_) => {
                let fallback = CString::new(r#"{"success":false,"error":"Unknown error"}"#)
                    .expect("static JSON has no NUL");
                *output_json = fallback.into_raw();
                -1
            }
        }
    }
}

/// FFI function to hash field elements using Poseidon
///
/// Input JSON format:
/// {
///   "inputs": ["123", "456", ...]  // Array of field element strings (BigInt as decimal string)
/// }
///
/// Output JSON format:
/// {
///   "success": true,
///   "result": "789..."  // Field element as decimal string
/// }
///
/// On error, returns JSON with "success": false and "error": "..."
///
/// # Safety
///
/// - `input_json` must be a valid, non-null, NUL-terminated C string for the
///   duration of the call.
/// - `output_json` must be a valid, non-null, writable pointer for the
///   duration of the call.
/// - On success, `*output_json` is set to a newly allocated C string that
///   **must** be freed with [`poseidon_free_string`].
#[no_mangle]
pub unsafe extern "C" fn poseidon_hash(input_json: *const c_char, output_json: *mut *mut c_char) -> c_int {
    if input_json.is_null() || output_json.is_null() {
        return -1;
    }

    let guarded = catch_unwind(AssertUnwindSafe(|| unsafe {
        let input_str = match CStr::from_ptr(input_json).to_str() {
            Ok(s) => s,
            Err(_) => {
                return write_error_json(output_json, "Invalid UTF-8 input");
            }
        };

        if input_str.len() > MAX_JSON_BYTES {
            return write_error_json(output_json, "Input JSON exceeds maximum allowed size");
        }

        match poseidon_hash_field_elements(input_str) {
            Ok(json_str) => match CString::new(json_str) {
                Ok(cstr) => {
                    *output_json = cstr.into_raw();
                    0
                }
                Err(_) => write_error_json(output_json, "Failed to create output string"),
            },
            Err(e) => write_error_json(output_json, &e),
        }
    }));
    match guarded {
        Ok(code) => code,
        Err(_) => write_error_json(output_json, "Rust panic in poseidon_hash"),
    }
}

/// Free memory allocated by poseidon_hash
///
/// # Safety
///
/// `ptr` must be a pointer previously returned by [`poseidon_hash`] or
/// [`poseidon_hash_bits_ffi`], or null. Must not be freed more than once.
#[no_mangle]
pub unsafe extern "C" fn poseidon_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// FFI function to hash bits using Poseidon (matching circomlibjs behavior)
///
/// Input JSON format:
/// {
///   "bits": [0, 1, 0, ...]  // Array of bits (0 or 1)
/// }
///
/// Output JSON format:
/// {
///   "success": true,
///   "result": "789..."  // Field element as decimal string
/// }
///
/// On error, returns JSON with "success": false and "error": "..."
///
/// # Safety
///
/// - `input_json` must be a valid, non-null, NUL-terminated C string for the
///   duration of the call.
/// - `output_json` must be a valid, non-null, writable pointer for the
///   duration of the call.
/// - On success, `*output_json` is set to a newly allocated C string that
///   **must** be freed with [`poseidon_free_string`].
#[no_mangle]
pub unsafe extern "C" fn poseidon_hash_bits_ffi(
    input_json: *const c_char,
    output_json: *mut *mut c_char,
) -> c_int {
    if input_json.is_null() || output_json.is_null() {
        return -1;
    }

    let guarded = catch_unwind(AssertUnwindSafe(|| unsafe {
        let input_str = match CStr::from_ptr(input_json).to_str() {
            Ok(s) => s,
            Err(_) => {
                return write_error_json(output_json, "Invalid UTF-8 input");
            }
        };

        if input_str.len() > MAX_JSON_BYTES {
            return write_error_json(output_json, "Input JSON exceeds maximum allowed size");
        }

        match poseidon_hash_bits_from_json(input_str) {
            Ok(json_str) => match CString::new(json_str) {
                Ok(cstr) => {
                    *output_json = cstr.into_raw();
                    0
                }
                Err(_) => write_error_json(output_json, "Failed to create output string"),
            },
            Err(e) => write_error_json(output_json, &e),
        }
    }));
    match guarded {
        Ok(code) => code,
        Err(_) => write_error_json(output_json, "Rust panic in poseidon_hash_bits_ffi"),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoseidonHashBitsRequest {
    bits: Vec<u8>,
}

/// Hash bits using Poseidon (wrapper for poseidon_hash_bits)
fn poseidon_hash_bits_from_json(input_json: &str) -> Result<String, String> {
    let request: PoseidonHashBitsRequest = serde_json::from_str(input_json)
        .map_err(|e| format!("Failed to parse input JSON: {}", e))?;

    if request.bits.is_empty() {
        return Err("Bits input must not be empty".to_string());
    }
    if request.bits.len() > MAX_BITS_LEN {
        return Err(format!("Bits input exceeds maximum allowed length of {MAX_BITS_LEN}"));
    }

    let hash = poseidon_hash_bits(&request.bits)?;
    let hash_str = hash.into_bigint().to_string();

    let output = PoseidonHashResult {
        success: true,
        result: Some(hash_str),
        error: None,
    };

    serde_json::to_string(&output).map_err(|e| format!("Failed to serialize output: {}", e))
}

use ark_bn254::Fr;
use ark_ff::PrimeField;
use light_poseidon::{Poseidon, PoseidonHasher};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoseidonHashRequest {
    inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoseidonHashResult {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Hash an array of field elements using Poseidon
/// This matches the behavior of circomlibjs's Poseidon hash
fn poseidon_hash_field_elements(input_json: &str) -> Result<String, String> {
    let request: PoseidonHashRequest = serde_json::from_str(input_json)
        .map_err(|e| format!("Failed to parse input JSON: {}", e))?;

    if request.inputs.is_empty() {
        return Err("Input array cannot be empty".to_string());
    }
    if request.inputs.len() == 1 {
        return Err(
            "Single-input Poseidon hash is disabled to avoid [x] vs [x,0] domain collision"
                .to_string(),
        );
    }
    if request.inputs.len() > MAX_POSEIDON_FIELD_INPUTS {
        return Err(format!("Input array exceeds maximum allowed length of {MAX_POSEIDON_FIELD_INPUTS}"));
    }

    // Convert string inputs to field elements
    let mut field_elements = Vec::new();
    for (idx, input_str) in request.inputs.iter().enumerate() {
        let field_element = Fr::from_str(input_str)
            .map_err(|_| format!("Failed to parse field element at index {idx}"))?;
        field_elements.push(field_element);
    }

    // Create Poseidon instance with Circom parameters
    let num_inputs = field_elements.len();
    let mut poseidon = Poseidon::<Fr>::new_circom(num_inputs)
        .map_err(|e| format!("Failed to create Poseidon instance: {:?}", e))?;

    // Hash the field elements
    let hash = poseidon
        .hash(&field_elements)
        .map_err(|e| format!("Poseidon hash failed: {:?}", e))?;

    // Convert field element to string (decimal representation)
    let hash_str = hash.into_bigint().to_string();

    let output = PoseidonHashResult {
        success: true,
        result: Some(hash_str),
        error: None,
    };

    serde_json::to_string(&output).map_err(|e| format!("Failed to serialize output: {}", e))
}

/// FFI function to sign data using EdDSA on Baby Jubjub
///
/// Input JSON format:
/// {
///   "bits": [0, 1, 0, ...],  // Array of bits (0 or 1)
///   "privateKeyHex": "00010203..."  // 64 hex characters (32 bytes)
/// }
///
/// Output JSON format:
/// {
///   "success": true,
///   "result": {
///     "Ax": "...",
///     "Ay": "...",
///     "R8x": "...",
///     "R8y": "...",
///     "S": "..."
///   }
/// }
///
/// On error, returns JSON with "success": false and "error": "..."
///
/// # Safety
///
/// - `input_json` must be a valid, non-null, NUL-terminated C string for the
///   duration of the call.
/// - `output_json` must be a valid, non-null, writable pointer for the
///   duration of the call.
/// - On success, `*output_json` is set to a newly allocated C string that
///   **must** be freed with [`eddsa_free_string`].
#[no_mangle]
pub unsafe extern "C" fn eddsa_sign(input_json: *const c_char, output_json: *mut *mut c_char) -> c_int {
    if input_json.is_null() || output_json.is_null() {
        return -1;
    }

    let guarded = catch_unwind(AssertUnwindSafe(|| unsafe {
        let input_json_owned = match CStr::from_ptr(input_json).to_str() {
            Ok(s) => Zeroizing::new(s.to_owned()),
            Err(_) => {
                return write_error_json(output_json, "Invalid UTF-8 input");
            }
        };

        if input_json_owned.len() > MAX_JSON_BYTES {
            return write_error_json(output_json, "Input JSON exceeds maximum allowed size");
        }

        match sign_eddsa(&input_json_owned) {
            Ok(json_str) => match CString::new(json_str) {
                Ok(cstr) => {
                    *output_json = cstr.into_raw();
                    0
                }
                Err(_) => write_error_json(output_json, "Failed to create output string"),
            },
            Err(e) => write_error_json(output_json, &e),
        }
    }));
    match guarded {
        Ok(code) => code,
        Err(_) => write_error_json(output_json, "Rust panic in eddsa_sign"),
    }
}

/// FFI function to verify EdDSA signature over pre-hashed Poseidon digest.
///
/// # Safety
///
/// - `input_json` must be a valid, non-null, NUL-terminated C string for the
///   duration of the call.
/// - `output_json` must be a valid, non-null, writable pointer for the
///   duration of the call.
/// - On success, `*output_json` is set to a newly allocated C string that
///   **must** be freed with [`eddsa_free_string`].
#[no_mangle]
pub unsafe extern "C" fn eddsa_verify(input_json: *const c_char, output_json: *mut *mut c_char) -> c_int {
    if input_json.is_null() || output_json.is_null() {
        return -1;
    }

    let guarded = catch_unwind(AssertUnwindSafe(|| unsafe {
        let input_str = match CStr::from_ptr(input_json).to_str() {
            Ok(s) => s,
            Err(_) => {
                return write_error_json(output_json, "Invalid UTF-8 input");
            }
        };

        if input_str.len() > MAX_JSON_BYTES {
            return write_error_json(output_json, "Input JSON exceeds maximum allowed size");
        }

        match verify_eddsa(input_str) {
            Ok(json_str) => match CString::new(json_str) {
                Ok(cstr) => {
                    *output_json = cstr.into_raw();
                    0
                }
                Err(_) => write_error_json(output_json, "Failed to create output string"),
            },
            Err(e) => write_error_json(output_json, &e),
        }
    }));
    match guarded {
        Ok(code) => code,
        Err(_) => write_error_json(output_json, "Rust panic in eddsa_verify"),
    }
}

/// Free memory allocated by eddsa_sign
///
/// # Safety
///
/// `ptr` must be a pointer previously returned by [`eddsa_sign`] or
/// [`eddsa_verify`], or null. Must not be freed more than once.
#[no_mangle]
pub unsafe extern "C" fn eddsa_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;
    use serde_json::Value;
    use std::ptr;

    fn random_private_key_hex() -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    #[test]
    fn test_eddsa_sign() {
        let key = random_private_key_hex();
        let input = format!(r#"{{
            "operation": "sign",
            "data": {{
                "bits": [0, 1, 0, 1],
                "privateKeyHex": "{key}"
            }}
        }}"#);

        let input_cstr = CString::new(input).unwrap();
        let mut output_ptr: *mut c_char = ptr::null_mut();

        let result = unsafe { eddsa_sign(input_cstr.as_ptr(), &mut output_ptr) };

        assert_eq!(result, 0);

        if !output_ptr.is_null() {
            let output_cstr = unsafe { CStr::from_ptr(output_ptr) };
            let output_str = output_cstr.to_str().unwrap();
            println!("Output: {}", output_str);

            unsafe { eddsa_free_string(output_ptr) };
        }
    }

    #[test]
    fn test_eddsa_verify_returns_error_json_for_malformed_payload() {
        let input = r#"{
            "operation": "verify",
            "data": {
                "msgHash": "bad-field",
                "publicKeyAx": "1",
                "publicKeyAy": "2",
                "R8x": "3",
                "R8y": "4",
                "S": "5"
            }
        }"#;

        let input_cstr = CString::new(input).unwrap();
        let mut output_ptr: *mut c_char = ptr::null_mut();

        let result = unsafe { eddsa_verify(input_cstr.as_ptr(), &mut output_ptr) };
        assert_eq!(result, -1);
        assert!(!output_ptr.is_null());

        let output_cstr = unsafe { CStr::from_ptr(output_ptr) };
        let output_str = output_cstr.to_str().unwrap();
        let parsed: Value = serde_json::from_str(output_str).unwrap();
        assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(false));
        assert!(parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("Failed to parse msgHash as field element"));

        unsafe { eddsa_free_string(output_ptr) };
    }

    #[test]
    fn test_poseidon_hash_rejects_single_input() {
        let input = r#"{"inputs":["7"]}"#;
        let input_cstr = CString::new(input).unwrap();
        let mut output_ptr: *mut c_char = ptr::null_mut();

        let result = unsafe { poseidon_hash(input_cstr.as_ptr(), &mut output_ptr) };
        assert_eq!(result, -1);
        assert!(!output_ptr.is_null());

        let output_cstr = unsafe { CStr::from_ptr(output_ptr) };
        let output_str = output_cstr.to_str().unwrap();
        let parsed: Value = serde_json::from_str(output_str).unwrap();
        assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(false));
        assert!(parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("Single-input Poseidon hash is disabled"));
        unsafe { poseidon_free_string(output_ptr) };
    }

    #[test]
    fn test_poseidon_hash_bits_ffi_rejects_empty_bits() {
        // Regression test for: empty-bits FFI returns a known constant Poseidon digest.
        // An empty `bits` array must be rejected, not silently hashed as 248 zero-bits,
        // because that would return a predictable value an attacker can compute.
        let input = r#"{"bits":[]}"#;
        let input_cstr = CString::new(input).unwrap();
        let mut output_ptr: *mut c_char = ptr::null_mut();

        let result = unsafe { poseidon_hash_bits_ffi(input_cstr.as_ptr(), &mut output_ptr) };
        assert_eq!(result, -1, "empty bits must return an error code");
        assert!(!output_ptr.is_null());

        let output_cstr = unsafe { CStr::from_ptr(output_ptr) };
        let output_str = output_cstr.to_str().unwrap();
        let parsed: Value = serde_json::from_str(output_str).unwrap();
        assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(false));
        assert!(
            parsed
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .contains("must not be empty"),
            "error message should mention empty input"
        );
        unsafe { poseidon_free_string(output_ptr) };
    }

    #[test]
    fn test_poseidon_hash_bits_ffi_rejects_oversized_bits() {
        // MAX_BITS_LEN + 1 bits must be rejected before Poseidon processing.
        let bits: Vec<u8> = vec![0u8; MAX_BITS_LEN + 1];
        let bits_json: Vec<String> = bits.iter().map(|b| b.to_string()).collect();
        let input = format!("{{\"bits\":[{}]}}", bits_json.join(","));
        let input_cstr = CString::new(input).unwrap();
        let mut output_ptr: *mut c_char = ptr::null_mut();

        let result = unsafe { poseidon_hash_bits_ffi(input_cstr.as_ptr(), &mut output_ptr) };
        assert_eq!(result, -1, "oversized bits must return an error code");

        let output_str = unsafe { CStr::from_ptr(output_ptr).to_str().unwrap() };
        let parsed: Value = serde_json::from_str(output_str).unwrap();
        assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(false));
        assert!(
            parsed.get("error").and_then(|v| v.as_str()).unwrap_or_default()
                .contains("exceeds maximum allowed length"),
            "error should mention length limit"
        );
        unsafe { poseidon_free_string(output_ptr) };
    }

    #[test]
    fn test_poseidon_hash_rejects_too_many_field_inputs() {
        // MAX_POSEIDON_FIELD_INPUTS + 1 elements must be rejected.
        let inputs: Vec<String> = (0..=MAX_POSEIDON_FIELD_INPUTS).map(|i| format!("\"{}\"", i)).collect();
        let input = format!("{{\"inputs\":[{}]}}", inputs.join(","));
        let input_cstr = CString::new(input).unwrap();
        let mut output_ptr: *mut c_char = ptr::null_mut();

        let result = unsafe { poseidon_hash(input_cstr.as_ptr(), &mut output_ptr) };
        assert_eq!(result, -1, "too many inputs must return an error code");

        let output_str = unsafe { CStr::from_ptr(output_ptr).to_str().unwrap() };
        let parsed: Value = serde_json::from_str(output_str).unwrap();
        assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(false));
        assert!(
            parsed.get("error").and_then(|v| v.as_str()).unwrap_or_default()
                .contains("exceeds maximum allowed length"),
            "error should mention length limit"
        );
        unsafe { poseidon_free_string(output_ptr) };
    }

    #[test]
    fn test_eddsa_sign_rejects_oversized_bits() {
        // Signing with bits array > MAX_BITS_LEN must be rejected.
        let bits: Vec<u8> = vec![0u8; MAX_BITS_LEN + 1];
        let bits_json: Vec<String> = bits.iter().map(|b| b.to_string()).collect();
        let key = random_private_key_hex();
        let input = format!(
            "{{\"operation\":\"sign\",\"data\":{{\"bits\":[{}],\"privateKeyHex\":\"{}\"}}}}",
            bits_json.join(","),
            key
        );
        let input_cstr = CString::new(input).unwrap();
        let mut output_ptr: *mut c_char = ptr::null_mut();

        let result = unsafe { eddsa_sign(input_cstr.as_ptr(), &mut output_ptr) };
        assert_eq!(result, -1, "oversized bits must return an error code");

        let output_str = unsafe { CStr::from_ptr(output_ptr).to_str().unwrap() };
        let parsed: Value = serde_json::from_str(output_str).unwrap();
        assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(false));
        assert!(
            parsed.get("error").and_then(|v| v.as_str()).unwrap_or_default()
                .contains("exceeds maximum allowed length"),
            "error should mention length limit"
        );
        unsafe { eddsa_free_string(output_ptr) };
    }

    #[test]
    fn test_ffi_rejects_oversized_raw_json() {
        // Any FFI call whose raw JSON exceeds MAX_JSON_BYTES must be rejected before parsing.
        // Build a string that is definitely > 1 MiB but is not valid JSON, proving the check
        // fires before serde ever touches it.
        let large = "x".repeat(MAX_JSON_BYTES + 1);
        let input_cstr = CString::new(large).unwrap();
        let mut output_ptr: *mut c_char = ptr::null_mut();

        let result = unsafe { poseidon_hash_bits_ffi(input_cstr.as_ptr(), &mut output_ptr) };
        assert_eq!(result, -1, "oversized raw JSON must return an error code");

        let output_str = unsafe { CStr::from_ptr(output_ptr).to_str().unwrap() };
        let parsed: Value = serde_json::from_str(output_str).unwrap();
        assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(false));
        assert!(
            parsed.get("error").and_then(|v| v.as_str()).unwrap_or_default()
                .contains("exceeds maximum allowed size"),
            "error should mention size limit"
        );
        unsafe { poseidon_free_string(output_ptr) };
    }
}
