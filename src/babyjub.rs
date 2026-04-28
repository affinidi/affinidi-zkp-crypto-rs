// Baby Jubjub curve implementation matching circomlibjs
// Curve parameters: A = 168700, D = 168696
// Note: Use ark_bn254::Fr (not Fq) - it has the correct modulus for Baby Jubjub
use ark_bn254::Fr;
use ark_ff::Field;
use num_bigint::BigUint;
use std::str::FromStr;

pub struct BabyJubPoint {
    pub x: Fr,
    pub y: Fr,
}

// Baby Jubjub curve parameters
const A: &str = "168700";
const D: &str = "168696";

impl BabyJubPoint {
    pub fn identity() -> Self {
        BabyJubPoint {
            x: Fr::from(0u64),
            y: Fr::from(1u64),
        }
    }

    pub fn new(x: Fr, y: Fr) -> Self {
        BabyJubPoint { x, y }
    }

    pub fn is_on_curve(&self) -> bool {
        let a = Fr::from_str(A).expect("A");
        let d = Fr::from_str(D).expect("D");
        let x2 = self.x * self.x;
        let y2 = self.y * self.y;
        (a * x2) + y2 == Fr::from(1u64) + (d * x2 * y2)
    }

    /// Add two points on Baby Jubjub curve (matching circomlibjs.addPoint)
    /// Formula from circomlibjs:
    /// res[0] = (beta + gamma) / (1 + d*tau)
    /// res[1] = (delta + A*beta - gamma) / (1 - d*tau)
    /// where:
    ///   beta = a[0]*b[1]
    ///   gamma = a[1]*b[0]
    ///   delta = (a[1] - A*a[0]) * (b[0] + b[1])
    ///   tau = beta * gamma
    ///   dtau = D * tau
    pub fn add(&self, other: &BabyJubPoint) -> Result<BabyJubPoint, String> {
        let a = Fr::from_str(A).expect("A");
        let d = Fr::from_str(D).expect("D");

        let beta = self.x * other.y;
        let gamma = self.y * other.x;
        let delta = (self.y - a * self.x) * (other.x + other.y);
        let tau = beta * gamma;
        let dtau = d * tau;

        let one = Fr::from(1u64);
        let denom_x = one + dtau;
        let denom_y = one - dtau;

        // Compute inverses
        let inv_x = denom_x
            .inverse()
            .ok_or("BabyJub addition failed: denominator x has no inverse")?;
        let inv_y = denom_y
            .inverse()
            .ok_or("BabyJub addition failed: denominator y has no inverse")?;

        let x = (beta + gamma) * inv_x;
        let y = (delta + a * beta - gamma) * inv_y;

        Ok(BabyJubPoint::new(x, y))
    }

    fn conditional_select(a: &BabyJubPoint, b: &BabyJubPoint, bit: u8) -> BabyJubPoint {
        let bit_fr = Fr::from(bit as u64);
        let inv_bit_fr = Fr::from(1u64) - bit_fr;

        BabyJubPoint {
            x: (a.x * inv_bit_fr) + (b.x * bit_fr),
            y: (a.y * inv_bit_fr) + (b.y * bit_fr),
        }
    }

    /// Scalar multiplication using a fixed-length ladder with branchless selection.
    ///
    /// This avoids data-dependent branching on scalar bits and always runs a
    /// constant number of iterations for reduced timing side-channel leakage.
    pub fn mul_scalar(&self, scalar: &BigUint) -> Result<BabyJubPoint, String> {
        let curve_order = BigUint::from_str(
            "21888242871839275222246405745257275088614511777268538073601725287587578984328",
        )
        .unwrap();
        let scalar_mod = scalar % &curve_order;

        // Use a fixed-width 256-bit representation to keep bit access uniform.
        let mut scalar_bytes_le = scalar_mod.to_bytes_le();
        scalar_bytes_le.resize(32, 0u8);

        // Montgomery ladder style state:
        // r0 = k*P for processed prefix, r1 = (k+1)*P.
        let mut r0 = BabyJubPoint::identity();
        let mut r1 = BabyJubPoint::new(self.x, self.y);

        for bit_index in (0..256).rev() {
            let byte = scalar_bytes_le[bit_index / 8];
            let bit = (byte >> (bit_index % 8)) & 1;

            let r0_plus_r1 = r0.add(&r1)?;
            let r0_double = r0.add(&r0)?;
            let r1_double = r1.add(&r1)?;

            // If bit == 0: (r0, r1) = (2*r0, r0+r1)
            // If bit == 1: (r0, r1) = (r0+r1, 2*r1)
            r0 = BabyJubPoint::conditional_select(&r0_double, &r0_plus_r1, bit);
            r1 = BabyJubPoint::conditional_select(&r0_plus_r1, &r1_double, bit);
        }

        Ok(r0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base8_doubling() {
        let base8_x = Fr::from_str(
            "5299619240641551281634865583518297030282874472190772894086521144482721001553",
        )
        .unwrap();
        let base8_y = Fr::from_str(
            "16950150798460657717958625567821834550301663161624707787222815936182638968203",
        )
        .unwrap();

        let base8 = BabyJubPoint::new(base8_x, base8_y);
        let base8_2 = base8.add(&base8).expect("base8 doubling");

        // Expected from JavaScript
        let expected_x = Fr::from_str(
            "10031262171927540148667355526369034398030886437092045105752248699557385197826",
        )
        .unwrap();
        let expected_y = Fr::from_str(
            "633281375905621697187330766174974863687049529291089048651929454608812697683",
        )
        .unwrap();

        assert_eq!(base8_2.x, expected_x);
        assert_eq!(base8_2.y, expected_y);
    }
}
