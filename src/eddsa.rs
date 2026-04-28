use crate::babyjub::BabyJubPoint;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use light_poseidon::{Poseidon, PoseidonHasher};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;
use zeroize::{Zeroize, Zeroizing};

/// BN254 scalar field order (Baby Jubjub subgroup order), decimal string.
const BABYJUB_ORDER_DEC: &str =
    "21888242871839275222246405745257275088614511777268538073601725287587578984328";
/// Baby Jubjub Base8 generator (circomlibjs), decimal x/y.
const BABYJUB_BASE8_X_DEC: &str =
    "5299619240641551281634865583518297030282874472190772894086521144482721001553";
const BABYJUB_BASE8_Y_DEC: &str =
    "16950150798460657717958625567821834550301663161624707787222815936182638968203";

fn babyjub_order() -> &'static BigUint {
    static ORDER: OnceLock<BigUint> = OnceLock::new();
    ORDER.get_or_init(|| BigUint::from_str(BABYJUB_ORDER_DEC).expect("BABYJUB_ORDER_DEC"))
}

fn babyjub_base8() -> &'static BabyJubPoint {
    static BASE8: OnceLock<BabyJubPoint> = OnceLock::new();
    BASE8.get_or_init(|| {
        let x = Fr::from_str(BABYJUB_BASE8_X_DEC).expect("BABYJUB_BASE8_X_DEC");
        let y = Fr::from_str(BABYJUB_BASE8_Y_DEC).expect("BABYJUB_BASE8_Y_DEC");
        BabyJubPoint::new(x, y)
    })
}

fn parse_sign_request(input_json: &str) -> Result<SignRequest, String> {
    // Parse from a zeroizing owned copy so plaintext JSON containing private keys
    // is wiped when this helper returns.
    let input_json_owned = Zeroizing::new(input_json.to_owned());
    serde_json::from_str(&input_json_owned).map_err(|e| format!("Failed to parse input JSON: {}", e))
}

#[derive(Debug, Serialize, Deserialize)]
struct SignRequest {
    operation: String,
    data: SignInputData,
}

#[derive(Serialize, Deserialize)]
struct SignInputData {
    /// Message as raw bits, to be Poseidon-hashed inside Rust (matches circomlibjs document flow)
    /// Used by existing callers like `edDSADocument.dart`.
    #[serde(default)]
    bits: Vec<u8>,
    /// Optional pre-hashed message (Poseidon field element as decimal string).
    /// When present, we treat this as the message for EdDSA and **do not** hash bits again.
    /// This is intended for flows like PHC verifier challenge where JS/Rust already
    /// computed the Poseidon digest and we only need EdDSA over that field element.
    #[serde(rename = "msgHash")]
    #[serde(default)]
    msg_hash: Option<String>,
    #[serde(rename = "privateKeyHex")]
    private_key_hex: String,
}

impl fmt::Debug for SignInputData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignInputData")
            .field("bits", &self.bits)
            .field("msg_hash", &self.msg_hash)
            .field("private_key_hex", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignResult {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<SignatureOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DerivePublicKeyResult {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<PublicKeyOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PublicKeyOutput {
    #[serde(rename = "Ax")]
    ax: String,
    #[serde(rename = "Ay")]
    ay: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignatureOutput {
    #[serde(rename = "Ax")]
    ax: String,
    #[serde(rename = "Ay")]
    ay: String,
    #[serde(rename = "R8x")]
    r8x: String,
    #[serde(rename = "R8y")]
    r8y: String,
    #[serde(rename = "S")]
    s: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerifyRequest {
    operation: String,
    data: VerifyInputData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerifyInputData {
    #[serde(rename = "msgHash")]
    msg_hash: String,
    #[serde(rename = "publicKeyAx")]
    public_key_ax: String,
    #[serde(rename = "publicKeyAy")]
    public_key_ay: String,
    #[serde(rename = "R8x")]
    r8x: String,
    #[serde(rename = "R8y")]
    r8y: String,
    #[serde(rename = "S")]
    s: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerifyResult {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Sign data using EdDSA on Baby Jubjub curve
/// This matches the behavior of circomlibjs's EdDSA implementation
pub fn sign_eddsa(input_json: &str) -> Result<String, String> {
    // Parse input (matches Dart format: { "operation": "sign", "data": {...} })
    let request: SignRequest = parse_sign_request(input_json)?;

    if request.operation != "sign" && request.operation != "derivePublicKey" {
        return Err(format!("Unknown operation: {}", request.operation));
    }

    let mut input = request.data;

    // Validate private key
    let mut private_key_hex = Zeroizing::new(std::mem::take(&mut input.private_key_hex));
    if private_key_hex.len() != 64 {
        return Err("Private key must be 64 hex characters (32 bytes)".to_string());
    }

    // Convert private key from hex
    let private_key_bytes = Zeroizing::new(
        hex::decode(&*private_key_hex).map_err(|e| format!("Invalid hex private key: {}", e))?,
    );
    private_key_hex.zeroize();

    if private_key_bytes.len() != 32 {
        return Err("Private key must be 32 bytes".to_string());
    }

    // Public key derivation mode: return Ax/Ay only (no signing).
    if request.operation == "derivePublicKey" {
        let public_key = derive_public_key(private_key_bytes.as_slice())?;
        let output = DerivePublicKeyResult {
            success: true,
            result: Some(PublicKeyOutput {
                ax: field_to_string(public_key.x),
                ay: field_to_string(public_key.y),
            }),
            error: None,
        };
        return serde_json::to_string(&output).map_err(|e| format!("Failed to serialize output: {}", e));
    }

    // Decide how to obtain the message hash for EdDSA:
    //
    // - Legacy / document flow: we receive raw bits and hash them with Poseidon here
    //   (matching circomlibjs: Poseidon(bits) -> signPoseidon(msgHash)).
    // - PHC verifier challenge flow: JS/Rust already computed the Poseidon digest
    //   and passes it in `msgHash`, so we **must not** hash again, otherwise we'd
    //   be signing Poseidon(Poseidon(...)) instead of the intended digest.
    let msg_hash = if let Some(msg_hash_str) = input.msg_hash {
        // Parse Poseidon field element from decimal string
        Fr::from_str(&msg_hash_str)
            .map_err(|e| format!("Failed to parse msgHash as field element: {:?}", e))?
    } else {
        // Backwards-compatible behavior: hash raw bits with Poseidon
        if input.bits.len() > crate::MAX_BITS_LEN {
            return Err(format!(
                "Bits input exceeds maximum allowed length of {}",
                crate::MAX_BITS_LEN
            ));
        }
        poseidon_hash_bits(&input.bits)?
    };

    // Derive public key from private key (Baby Jubjub)
    let public_key = derive_public_key(private_key_bytes.as_slice())?;

    // Sign the message hash using EdDSA with Poseidon (matching circomlibjs.signPoseidon)
    let signature = sign_poseidon(private_key_bytes.as_slice(), &msg_hash, &public_key)?;

    // Convert to field element strings (matching circomlibjs format)
    let result = SignatureOutput {
        ax: field_to_string(public_key.x),
        ay: field_to_string(public_key.y),
        r8x: field_to_string(signature.r8.x),
        r8y: field_to_string(signature.r8.y),
        s: signature.s.to_string(),
    };

    let output = SignResult {
        success: true,
        result: Some(result),
        error: None,
    };

    serde_json::to_string(&output).map_err(|e| format!("Failed to serialize output: {}", e))
}

/// Verify EdDSA signature over a Poseidon digest.
pub fn verify_eddsa(input_json: &str) -> Result<String, String> {
    let request: VerifyRequest = serde_json::from_str(input_json)
        .map_err(|e| format!("Failed to parse input JSON: {}", e))?;

    if request.operation != "verify" {
        return Err(format!("Unknown operation: {}", request.operation));
    }

    let input = request.data;
    let msg_hash = Fr::from_str(&input.msg_hash)
        .map_err(|e| format!("Failed to parse msgHash as field element: {:?}", e))?;
    let ax = Fr::from_str(&input.public_key_ax)
        .map_err(|e| format!("Failed to parse publicKeyAx: {:?}", e))?;
    let ay = Fr::from_str(&input.public_key_ay)
        .map_err(|e| format!("Failed to parse publicKeyAy: {:?}", e))?;
    let r8x = Fr::from_str(&input.r8x).map_err(|e| format!("Failed to parse R8x: {:?}", e))?;
    let r8y = Fr::from_str(&input.r8y).map_err(|e| format!("Failed to parse R8y: {:?}", e))?;
    let s = BigUint::from_str(&input.s).map_err(|e| format!("Failed to parse S: {}", e))?;

    let is_valid = verify_poseidon(
        &Point { x: ax, y: ay },
        &Point { x: r8x, y: r8y },
        &s,
        &msg_hash,
    )?;

    let output = VerifyResult {
        success: true,
        result: Some(is_valid),
        error: None,
    };

    serde_json::to_string(&output).map_err(|e| format!("Failed to serialize output: {}", e))
}

#[derive(Debug, Clone)]
struct Point {
    x: Fr,
    y: Fr,
}

#[derive(Debug, Clone)]
struct Signature {
    r8: Point,
    s: BigUint,
}

/// Prune buffer for EdDSA (matches circomlibjs.pruneBuffer)
/// Clamps the buffer: clears bottom 3 bits of first byte, clears top bit and sets second-highest bit of last byte
fn prune_buffer(buff: &mut [u8]) {
    if buff.len() >= 32 {
        buff[0] &= 0xF8; // Clear bottom 3 bits
        buff[31] &= 0x7F; // Clear top bit
        buff[31] |= 0x40; // Set second-highest bit
    }
}

/// Blake512 hash (original BLAKE algorithm, matching circomlibjs blake-hash)
fn blake512(data: &[u8]) -> Vec<u8> {
    crate::blake::blake512(data)
}

/// Read little-endian scalar from buffer (matches Scalar.fromRprLE)
pub fn from_rpr_le(buffer: &[u8], offset: usize, length: usize) -> BigUint {
    let end = (offset + length).min(buffer.len());
    let slice = &buffer[offset..end];

    // Read little-endian
    let mut value = BigUint::from(0u32);
    let mut power = BigUint::from(1u32);

    for &byte in slice {
        value += BigUint::from(byte as u32) * &power;
        power *= 256u32;
    }

    value
}

/// Convert field element to little-endian bytes (matches F.toRprLE)
/// In JavaScript, msg is converted to Baby Jubjub field element first, then to bytes
pub fn to_rpr_le(field: &Fr) -> Vec<u8> {
    // Convert Fr to little-endian bytes
    let bytes = field.into_bigint().to_bytes_le();
    // Pad to 32 bytes if needed
    let mut result = vec![0u8; 32];
    let copy_len = bytes.len().min(32);
    result[..copy_len].copy_from_slice(&bytes[..copy_len]);
    result
}

/// Derive public key from private key using Baby Jubjub
/// Matches circomlibjs's eddsa.prv2pub()
fn derive_public_key(private_key: &[u8]) -> Result<Point, String> {
    // Step 1: Hash private key with Blake512 and prune
    let mut s_buff = Zeroizing::new(blake512(private_key));
    prune_buffer(&mut s_buff);

    // Step 2: Extract scalar s from first 32 bytes (little-endian)
    let s_biguint = from_rpr_le(&s_buff, 0, 32);

    // Step 3: s >> 3 (shift right by 3)
    let s_shifted: BigUint = &s_biguint >> 3;

    // Step 4: A = Base8 * (s >> 3) using Baby Jubjub scalar multiplication
    let public_key = babyjub_base8().mul_scalar(&s_shifted)?;

    Ok(Point {
        x: public_key.x,
        y: public_key.y,
    })
}

/// Sign a message hash using EdDSA with Poseidon (matching circomlibjs.signPoseidon)
///
/// Algorithm (from circomlibjs source):
/// 1. sBuff = pruneBuffer(blake512(prv))
/// 2. s = fromRprLE(sBuff, 0, 32)
/// 3. A = Base8 * (s >> 3)
/// 4. composeBuff = [sBuff[32:], toRprLE(msg)]
/// 5. rBuff = blake512(composeBuff)
/// 6. r = fromRprLE(rBuff, 0, 64) % subOrder
/// 7. R8 = Base8 * r
/// 8. hm = poseidon([R8[0], R8[1], A[0], A[1], msg])
/// 9. S = (r + hm * s) % subOrder
fn sign_poseidon(
    private_key: &[u8],
    msg_hash: &Fr,
    public_key: &Point,
) -> Result<Signature, String> {
    // Step 1: sBuff = pruneBuffer(blake512(prv))
    // blake512 returns 64 bytes, pruneBuffer modifies it in place
    let mut s_buff = Zeroizing::new(blake512(private_key));
    if s_buff.len() != 64 {
        return Err(format!(
            "BLAKE512 hash should be 64 bytes, got {}",
            s_buff.len()
        ));
    }
    prune_buffer(&mut s_buff);

    // Step 2: s = fromRprLE(sBuff, 0, 32)
    // Step 3: A is already computed (passed as public_key parameter)

    // Step 4: composeBuff = [sBuff[32:], toRprLE(msg)]
    // sBuff is 64 bytes, we need bytes 32-64 (second half)
    let mut compose_buff = Zeroizing::new(Vec::new());
    compose_buff.extend_from_slice(&s_buff[32..64]); // Second half of sBuff
    let msg_bytes = to_rpr_le(msg_hash);
    compose_buff.extend_from_slice(&msg_bytes);

    // Step 5: rBuff = blake512(composeBuff)
    let r_buff = Zeroizing::new(blake512(&compose_buff));

    // Step 6: r = fromRprLE(rBuff, 0, 64) % subOrder
    let r_biguint = from_rpr_le(&r_buff, 0, 64);
    let order = babyjub_order();
    let sub_order = order >> 3u32; // subOrder = order >> 3
    let r_mod: BigUint = &r_biguint % &sub_order;

    // Step 7: R8 = Base8 * r
    let r8 = babyjub_base8().mul_scalar(&r_mod)?;

    // Step 8: hm = poseidon([R8[0], R8[1], A[0], A[1], msg])
    // All coordinates are already in Fr (BN254 scalar field), so we can use them directly
    let r8_x_fr = r8.x;
    let r8_y_fr = r8.y;
    let a_x_fr = public_key.x;
    let a_y_fr = public_key.y;

    let challenge_inputs = vec![r8_x_fr, r8_y_fr, a_x_fr, a_y_fr, *msg_hash];
    let mut poseidon_challenge = Poseidon::<Fr>::new_circom(challenge_inputs.len())
        .map_err(|e| format!("Failed to create Poseidon for challenge: {:?}", e))?;
    let hm = poseidon_challenge
        .hash(&challenge_inputs)
        .map_err(|e| format!("Failed to hash challenge: {:?}", e))?;

    // Step 9: S = (r + hm * s) % subOrder
    // Convert hm (Fr) and s to BigUint for arithmetic
    let hm_biguint = BigUint::from_str(&hm.to_string())
        .map_err(|_| "Failed to convert hm to BigUint".to_string())?;
    let s_biguint = from_rpr_le(&s_buff, 0, 32);
    let hm_s = &hm_biguint * &s_biguint;
    let s_result = &r_mod + &hm_s;

    // Mod subOrder
    let s_final = &s_result % &sub_order;

    Ok(Signature {
        r8: Point { x: r8.x, y: r8.y },
        s: s_final,
    })
}

/// Verify EdDSA Poseidon signature (matches circomlibjs verifyPoseidon).
fn verify_poseidon(
    public_key: &Point,
    r8: &Point,
    s: &BigUint,
    msg_hash: &Fr,
) -> Result<bool, String> {
    // Reject out-of-range scalar.
    let order = babyjub_order();
    let sub_order = order >> 3u32;
    if s >= &sub_order || s == &BigUint::from(0u32) {
        return Ok(false);
    }

    let pub_point = BabyJubPoint::new(public_key.x, public_key.y);
    let r8_point = BabyJubPoint::new(r8.x, r8.y);
    if !pub_point.is_on_curve() || !r8_point.is_on_curve() {
        return Ok(false);
    }
    let subgroup_order = &sub_order;
    let pub_cleared = pub_point.mul_scalar(subgroup_order)?;
    let r8_cleared = r8_point.mul_scalar(subgroup_order)?;
    if pub_cleared.x != Fr::from(0u64)
        || pub_cleared.y != Fr::from(1u64)
        || r8_cleared.x != Fr::from(0u64)
        || r8_cleared.y != Fr::from(1u64)
    {
        return Ok(false);
    }

    // hm = poseidon([R8x, R8y, Ax, Ay, msg])
    let challenge_inputs = vec![r8.x, r8.y, public_key.x, public_key.y, *msg_hash];
    let mut poseidon_challenge = Poseidon::<Fr>::new_circom(challenge_inputs.len())
        .map_err(|e| format!("Failed to create Poseidon for challenge: {:?}", e))?;
    let hm = poseidon_challenge
        .hash(&challenge_inputs)
        .map_err(|e| format!("Failed to hash challenge: {:?}", e))?;

    let hm_big = BigUint::from_str(&hm.to_string())
        .map_err(|_| "Failed to convert hm to scalar".to_string())?;
    let hm_mul_8 = &hm_big * BigUint::from(8u32);

    // Canonical circomlib/Circom relation only:
    //   Base8 * S == R8 + A * (8 * hm)
    let left = babyjub_base8().mul_scalar(s)?;
    let right_8hm = r8_point.add(&pub_point.mul_scalar(&hm_mul_8)?)?;
    Ok(left.x == right_8hm.x && left.y == right_8hm.y)
}

/// Convert field element to string (matching circomlibjs format)
fn field_to_string(field: Fr) -> String {
    field_to_biguint(&field).to_string()
}

/// Convert field element to BigUint
fn field_to_biguint(field: &Fr) -> BigUint {
    let bytes = field.into_bigint().to_bytes_be();
    BigUint::from_bytes_be(&bytes)
}

// Import poseidon hash function
use crate::poseidon_hash::poseidon_hash_bits;

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;
    use serde_json::{json, Value};

    /// Generate a random 64-hex-char (32-byte) private key for each test run.
    fn random_private_key_hex() -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    #[test]
    fn verify_operation_parses_and_returns_boolean_result() {
        let private_key_hex = random_private_key_hex();
        let msg_hash = "12345";
        let signed = sign_with_msg_hash(&private_key_hex, msg_hash);
        let sig = signed.get("result").expect("signature result");
        let output = verify_from_parts(
            msg_hash,
            sig.get("Ax").and_then(|v| v.as_str()).expect("Ax"),
            sig.get("Ay").and_then(|v| v.as_str()).expect("Ay"),
            sig.get("R8x").and_then(|v| v.as_str()).expect("R8x"),
            sig.get("R8y").and_then(|v| v.as_str()).expect("R8y"),
            sig.get("S").and_then(|v| v.as_str()).expect("S"),
        );
        assert_eq!(output.get("success").and_then(|v| v.as_bool()), Some(true));
        assert!(output.get("result").is_some());
    }

    fn sign_with_msg_hash(private_key_hex: &str, msg_hash: &str) -> Value {
        let sign_input = json!({
            "operation": "sign",
            "data": {
                "msgHash": msg_hash,
                "privateKeyHex": private_key_hex
            }
        });
        let output = sign_eddsa(&sign_input.to_string()).expect("sign should succeed");
        let parsed: Value = serde_json::from_str(&output).expect("valid sign json");
        assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(true));
        parsed
    }

    fn verify_from_parts(
        msg_hash: &str,
        ax: &str,
        ay: &str,
        r8x: &str,
        r8y: &str,
        s: &str,
    ) -> Value {
        let verify_input = json!({
            "operation": "verify",
            "data": {
                "msgHash": msg_hash,
                "publicKeyAx": ax,
                "publicKeyAy": ay,
                "R8x": r8x,
                "R8y": r8y,
                "S": s
            }
        });
        let output = verify_eddsa(&verify_input.to_string()).expect("verify should return output");
        serde_json::from_str(&output).expect("valid verify json")
    }

    fn derive_pubkey(private_key_hex: &str) -> Value {
        let derive_input = json!({
            "operation": "derivePublicKey",
            "data": {
                "privateKeyHex": private_key_hex
            }
        });
        let output = sign_eddsa(&derive_input.to_string()).expect("derive should succeed");
        let parsed: Value = serde_json::from_str(&output).expect("valid derive json");
        assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(true));
        parsed
    }

    #[test]
    fn derive_public_key_returns_ax_ay_only() {
        let private_key_hex = random_private_key_hex();
        let derived = derive_pubkey(&private_key_hex);
        let result = derived.get("result").expect("derive result");

        assert!(result.get("Ax").and_then(|v| v.as_str()).is_some());
        assert!(result.get("Ay").and_then(|v| v.as_str()).is_some());
        assert!(result.get("R8x").is_none());
        assert!(result.get("R8y").is_none());
        assert!(result.get("S").is_none());
    }

    #[test]
    fn derive_public_key_matches_sign_public_key() {
        let private_key_hex = random_private_key_hex();
        let derived = derive_pubkey(&private_key_hex);
        let signed = sign_with_msg_hash(&private_key_hex, "12345");

        let derived_result = derived.get("result").expect("derive result");
        let signed_result = signed.get("result").expect("sign result");

        assert_eq!(
            derived_result.get("Ax").and_then(|v| v.as_str()),
            signed_result.get("Ax").and_then(|v| v.as_str())
        );
        assert_eq!(
            derived_result.get("Ay").and_then(|v| v.as_str()),
            signed_result.get("Ay").and_then(|v| v.as_str())
        );
    }

    #[test]
    fn sign_then_verify_valid_signature() {
        let private_key_hex = random_private_key_hex();
        let msg_hash = "12345";
        let signed = sign_with_msg_hash(&private_key_hex, msg_hash);
        let sig = signed.get("result").expect("signature result");

        let verified = verify_from_parts(
            msg_hash,
            sig.get("Ax").and_then(|v| v.as_str()).expect("Ax"),
            sig.get("Ay").and_then(|v| v.as_str()).expect("Ay"),
            sig.get("R8x").and_then(|v| v.as_str()).expect("R8x"),
            sig.get("R8y").and_then(|v| v.as_str()).expect("R8y"),
            sig.get("S").and_then(|v| v.as_str()).expect("S"),
        );

        assert_eq!(
            verified.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(verified.get("result").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn verify_fails_for_wrong_public_key() {
        let private_key_hex = random_private_key_hex();
        let msg_hash = "12345";
        let signed = sign_with_msg_hash(&private_key_hex, msg_hash);
        let sig = signed.get("result").expect("signature result");

        let verified = verify_from_parts(
            msg_hash,
            "1", // mismatched key
            "2",
            sig.get("R8x").and_then(|v| v.as_str()).expect("R8x"),
            sig.get("R8y").and_then(|v| v.as_str()).expect("R8y"),
            sig.get("S").and_then(|v| v.as_str()).expect("S"),
        );

        assert_eq!(
            verified.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            verified.get("result").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn verify_fails_for_wrong_msg_hash() {
        let private_key_hex = random_private_key_hex();
        let signed = sign_with_msg_hash(&private_key_hex, "12345");
        let sig = signed.get("result").expect("signature result");

        let verified = verify_from_parts(
            "12346", // tampered message hash
            sig.get("Ax").and_then(|v| v.as_str()).expect("Ax"),
            sig.get("Ay").and_then(|v| v.as_str()).expect("Ay"),
            sig.get("R8x").and_then(|v| v.as_str()).expect("R8x"),
            sig.get("R8y").and_then(|v| v.as_str()).expect("R8y"),
            sig.get("S").and_then(|v| v.as_str()).expect("S"),
        );

        assert_eq!(
            verified.get("success").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            verified.get("result").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn verify_returns_error_for_malformed_verify_payload() {
        let malformed = json!({
            "operation": "verify",
            "data": {
                "msgHash": "not-a-field-element",
                "publicKeyAx": "1",
                "publicKeyAy": "2",
                "R8x": "3",
                "R8y": "4",
                "S": "5"
            }
        });

        let err = verify_eddsa(&malformed.to_string()).expect_err("should fail");
        assert!(err.contains("Failed to parse msgHash as field element"));
    }

    #[test]
    fn verify_rejects_zero_scalar() {
        let private_key_hex = random_private_key_hex();
        let msg_hash = "12345";
        let signed = sign_with_msg_hash(&private_key_hex, msg_hash);
        let sig = signed.get("result").expect("signature result");

        let verified = verify_from_parts(
            msg_hash,
            sig.get("Ax").and_then(|v| v.as_str()).expect("Ax"),
            sig.get("Ay").and_then(|v| v.as_str()).expect("Ay"),
            sig.get("R8x").and_then(|v| v.as_str()).expect("R8x"),
            sig.get("R8y").and_then(|v| v.as_str()).expect("R8y"),
            "0",
        );
        assert_eq!(verified.get("success").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(verified.get("result").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn verify_rejects_off_curve_r8() {
        let private_key_hex = random_private_key_hex();
        let msg_hash = "12345";
        let signed = sign_with_msg_hash(&private_key_hex, msg_hash);
        let sig = signed.get("result").expect("signature result");

        let verified = verify_from_parts(
            msg_hash,
            sig.get("Ax").and_then(|v| v.as_str()).expect("Ax"),
            sig.get("Ay").and_then(|v| v.as_str()).expect("Ay"),
            "1", // known off-curve for BabyJub in this context
            "0",
            sig.get("S").and_then(|v| v.as_str()).expect("S"),
        );
        assert_eq!(verified.get("success").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(verified.get("result").and_then(|v| v.as_bool()), Some(false));
    }
}
