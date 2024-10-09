// Uncomment the line below if on x86
//#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::time::Instant;

// Original IFMA implementation
// 1. Inline Assembly Optimization (IFMA) - x86_64 only
#[cfg(target_arch = "x86_64")]
unsafe fn ifma_multiply_add(a: u64, b: u64, c: u64) -> u128 {
    // This function demonstrates direct use of IFMA instructions
    // It requires unsafe code due to direct hardware interaction
    let mut result_low: u64;
    let mut result_high: u64;

    std::arch::asm!(
        "vpmadd52luq {0}, {2}, {3}",
        "vpmadd52huq {1}, {2}, {3}",
        inout(xmm_reg) c => result_low,
        out(xmm_reg) result_high,
        in(xmm_reg) a,
        in(xmm_reg) b,
    );

    ((result_high as u128) << 64) | (result_low as u128)
}

// 2. Compiler Optimization (64-bit)
#[inline(always)]
fn compiler_optimized_multiply_add(a: u64, b: u64, c: u64) -> u128 {
    // This function relies on the Rust compiler's ability to optimize
    (a as u128 * b as u128) + c as u128
}

// 128-bit version
#[inline(always)]
fn compiler_optimized_multiply_add_128(a: u128, b: u128, c: u128) -> (u128, u128) {
    let low = a.wrapping_mul(b).wrapping_add(c);
    let high = ((a >> 64).wrapping_mul(b) + (a & 0xFFFFFFFFFFFFFFFF).wrapping_mul(b >> 64))
        .wrapping_add(if low < c { 1 } else { 0 });
    (low, high)
}

// 256-bit version (using a simple struct)
#[derive(Clone, Copy)]
struct U256(u128, u128);

#[inline(always)]
fn compiler_optimized_multiply_add_256(a: U256, b: U256, c: U256) -> U256 {
    let (a_low, a_high) = (a.0, a.1);
    let (b_low, b_high) = (b.0, b.1);
    let (c_low, c_high) = (c.0, c.1);

    let mul_ll = a_low.wrapping_mul(b_low);
    let mul_lh = a_low.wrapping_mul(b_high);
    let mul_hl = a_high.wrapping_mul(b_low);
    let mul_hh = a_high.wrapping_mul(b_high);

    let mid = mul_lh.wrapping_add(mul_hl);
    let (mid, carry) = mid.overflowing_add(mul_ll >> 64);
    let high = mul_hh.wrapping_add(mid >> 64).wrapping_add(if carry { 1 } else { 0 });

    let low = mul_ll.wrapping_add(mid << 64).wrapping_add(c_low);
    let high = high.wrapping_add(c_high).wrapping_add((low < c_low) as u128);

    U256(low, high)
}

// 3. High-Level Abstraction (SIMD) - Only for 64-bit
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn simd_multiply_add(a: u64, b: u64, c: u64) -> u128 {
    // This function uses SIMD instructions through Rust's std::arch module
    // It demonstrates a higher-level abstraction for optimization
    use std::arch::x86_64::*;
    let a_vec = _mm256_set1_epi64x(a as i64);
    let b_vec = _mm256_set1_epi64x(b as i64);
    let c_vec = _mm256_set1_epi64x(c as i64);
    let mul_low = _mm256_mul_epu32(a_vec, b_vec);
    let mul_high = _mm256_mul_epu32(_mm256_srli_epi64(a_vec, 32), _mm256_srli_epi64(b_vec, 32));
    let sum = _mm256_add_epi64(mul_low, c_vec);
    let result_low = _mm256_extract_epi64(sum, 0) as u128;
    let result_high = _mm256_extract_epi64(mul_high, 0) as u128;
    (result_high << 64) | result_low
}

// 4. Algorithmic Optimization (Karatsuba algorithm) - 64-bit
fn karatsuba_multiply_add(x: u64, y: u64, z: u64) -> u128 {
    // This function implements the Karatsuba algorithm for multiplication
    const MASK32: u64 = (1 << 32) - 1;
    let x0 = x & MASK32;
    let x1 = x >> 32;
    let y0 = y & MASK32;
    let y1 = y >> 32;
    let z0 = x0 as u128 * y0 as u128;
    let z2 = x1 as u128 * y1 as u128;
    let z1 = ((x0 + x1) as u128 * (y0 + y1) as u128) - z0 - z2;
    (z2 << 64) | (z1 << 32) | (z0 + z as u128)
}

// 128-bit version
fn karatsuba_multiply_add_128(x: u128, y: u128, z: u128) -> (u128, u128) {
    const MASK64: u128 = (1u128 << 64) - 1;
    let x0 = x & MASK64;
    let x1 = x >> 64;
    let y0 = y & MASK64;
    let y1 = y >> 64;

    let z0 = x0 * y0;
    let z2 = x1 * y1;
    let z1 = ((x0 + x1) * (y0 + y1)) - z0 - z2;

    let low = z0.wrapping_add(z1 << 64).wrapping_add(z);
    let high = z2.wrapping_add(z1 >> 64).wrapping_add((low < z0) as u128);

    (low, high)
}

// 256-bit version
fn karatsuba_multiply_add_256(x: U256, y: U256, z: U256) -> U256 {
    let (x0, x1) = (x.0, x.1);
    let (y0, y1) = (y.0, y.1);

    let (z0_low, z0_high) = karatsuba_multiply_add_128(x0, y0, z.0);
    let (z2_low, z2_high) = karatsuba_multiply_add_128(x1, y1, 0);
    let (z1_low, z1_high) = karatsuba_multiply_add_128(x0.wrapping_add(x1), y0.wrapping_add(y1), 0);

    let z1_low = z1_low.wrapping_sub(z0_low).wrapping_sub(z2_low);
    let z1_high = z1_high.wrapping_sub(z0_high).wrapping_sub(z2_high);

    let low = z0_low.wrapping_add(z1_low);
    let high = z0_high.wrapping_add(z1_high).wrapping_add(z2_low).wrapping_add(z.1);

    U256(low, high)
}

fn benchmark<T, U>(f: impl Fn(T, T, T) -> U, name: &str, a: T, b: T, c: T, iterations: u32)
where
    T: Copy,
    U: Copy,
{
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = f(a, b, c);
    }
    let duration = start.elapsed();
    println!("{} time: {:?}", name, duration);
}

fn main() {
    let iterations = 1_000_000;
    println!("Running benchmarks ({} iterations each):", iterations);

    // 64-bit benchmarks
    let a_64 = 0x123456789abcdef0;
    let b_64 = 0x0fedcba987654321;
    let c_64 = 0x1111111111111111;

    #[cfg(target_arch = "x86_64")]
    if std::is_x86_feature_detected!("avx512ifma") {
        benchmark(|a, b, c| unsafe { ifma_multiply_add(a, b, c) }, "IFMA (64-bit)", a_64, b_64, c_64, iterations);
    } else {
        println!("IFMA not supported on this CPU.");
    }

    benchmark(compiler_optimized_multiply_add, "Compiler Optimized (64-bit)", a_64, b_64, c_64, iterations);

    #[cfg(target_arch = "x86_64")]
    if std::is_x86_feature_detected!("avx2") {
        benchmark(|a, b, c| unsafe { simd_multiply_add(a, b, c) }, "SIMD (AVX2) (64-bit)", a_64, b_64, c_64, iterations);
    } else {
        println!("AVX2 not supported on this CPU.");
    }

    benchmark(karatsuba_multiply_add, "Karatsuba (64-bit)", a_64, b_64, c_64, iterations);

    // 128-bit benchmarks
    let a_128 = (a_64 as u128) << 64 | b_64 as u128;
    let b_128 = (b_64 as u128) << 64 | a_64 as u128;
    let c_128 = (c_64 as u128) << 64 | c_64 as u128;

    benchmark(compiler_optimized_multiply_add_128, "Compiler Optimized (128-bit)", a_128, b_128, c_128, iterations / 10);
    benchmark(karatsuba_multiply_add_128, "Karatsuba (128-bit)", a_128, b_128, c_128, iterations / 10);

    // 256-bit benchmarks
    let a_256 = U256(a_128, b_128);
    let b_256 = U256(b_128, a_128);
    let c_256 = U256(c_128, c_128);

    benchmark(compiler_optimized_multiply_add_256, "Compiler Optimized (256-bit)", a_256, b_256, c_256, iterations / 100);
    benchmark(karatsuba_multiply_add_256, "Karatsuba (256-bit)", a_256, b_256, c_256, iterations / 100);

    // 5. External Library (ed25519-dalek)
    use ed25519_dalek::Signer;
    use ed25519_dalek::Keypair;
    use rand::rngs::OsRng;
    let mut csprng = OsRng{};
    let keypair: Keypair = Keypair::generate(&mut csprng);
    let message: &[u8] = b"Optimize without unsafe.";
    let signature = keypair.sign(message);
    println!("Ed25519 Signature: {:?}", signature.to_bytes());

    println!("Benchmarks completed.");
}
