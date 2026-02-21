//! Benchmark for comparing sequential vs parallel RNG generation
//! 
//! Run with: cargo bench -p random

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::{distributions::Standard, prelude::Distribution, SeedableRng};
use rand_chacha::ChaCha12Rng;
use random::rep3::rep3rng::{Rep3Rand, Rep3RandBitComp, Rep3CorrelatedRng};
use random::rep3::rep3rng_rayon::*;
use random::rep3::Rep3State;

// ChaCha12Rng seed size is 32 bytes
const SEED_SIZE: usize = 32;
use communication::rep3::id::PartyID;

/// Create a Rep3Rand for testing
fn create_test_rand(seed: [u8; SEED_SIZE]) -> Rep3Rand {
    Rep3Rand::new(seed, seed)
}

/// Create a Rep3RandBitComp with 2 keys for testing
fn create_test_bitcomp_2keys(seed: [u8; SEED_SIZE]) -> Rep3RandBitComp {
    Rep3RandBitComp::new_2keys(seed, seed)
}

/// Create a single Rep3State for sequential testing
fn create_single_state() -> Rep3State {
    let seed = [0u8; SEED_SIZE];
    let rand = create_test_rand(seed);
    let bitcomp1 = create_test_bitcomp_2keys(seed);
    let bitcomp2 = create_test_bitcomp_2keys(seed);
    
    Rep3State {
        id: PartyID::ID0,
        rngs: Rep3CorrelatedRng::new(rand, bitcomp1, bitcomp2),
        rng: ChaCha12Rng::from_seed(seed),
    }
}

/// Create multiple Rep3States for parallel testing
fn create_states(n: usize) -> Vec<Rep3State> {
    let base_seed = [0u8; SEED_SIZE];
    (0..n)
        .map(|i| {
            let mut seed = base_seed;
            seed[0] = i as u8;
            let rand = create_test_rand(seed);
            let bitcomp1 = create_test_bitcomp_2keys(seed);
            let bitcomp2 = create_test_bitcomp_2keys(seed);
            
            Rep3State {
                id: PartyID::ID0,
                rngs: Rep3CorrelatedRng::new(rand, bitcomp1, bitcomp2),
                rng: ChaCha12Rng::from_seed(seed),
            }
        })
        .collect()
}

/// Sequential random elements generation (truly sequential, no rayon::join)
fn seq_random_elements_vec<T>(state: &mut Rep3State, len: usize) -> (Vec<T>, Vec<T>)
where
    Standard: Distribution<T>,
{
    let a: Vec<T> = (0..len).map(|_| state.rngs.rand.random_element_rng1()).collect();
    let b: Vec<T> = (0..len).map(|_| state.rngs.rand.random_element_rng2()).collect();
    (a, b)
}

/// Sequential masking elements generation (truly sequential)
fn seq_masking_elements_vec<T>(state: &mut Rep3State, len: usize) -> Vec<T>
where
    Standard: Distribution<T>,
    T: std::ops::Sub<Output = T>,
{
    (0..len)
        .map(|_| {
            let (a, b) = state.rngs.rand.random_elements::<T>();
            a - b
        })
        .collect()
}

/// Original implementation using rayon::join (in rep3rng.rs)
fn original_random_elements_vec<T>(state: &mut Rep3State, len: usize) -> (Vec<T>, Vec<T>)
where
    Standard: Distribution<T>,
    T: Send + Sync,
{
    state.rngs.rand.random_elements_vec::<T>(len)
}

/// Parallel random elements generation using multiple states
fn par_random_elements_vec<T>(states: &mut [&mut Rep3State], len: usize) -> (Vec<T>, Vec<T>)
where
    Standard: Distribution<T>,
    T: Send + Sync,
{
    random_elements_vec_multithreads(states, len)
}

/// Benchmark: Sequential vs Original (rayon::join) vs Parallel (multiple states)
fn bench_random_elements_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_elements_comparison");
    
    // Test different data sizes
    for size in [6000000].iter() {
        // 1. Truly Sequential - single thread, no rayon at all
        group.bench_with_input(
            BenchmarkId::new("1_truly_sequential", size),
            size,
            |b, &size| {
                let mut state = create_single_state();
                b.iter(|| {
                    let result: (Vec<u64>, Vec<u64>) = seq_random_elements_vec(&mut state, size);
                    black_box(result);
                });
            },
        );

        // 2. Original implementation - uses rayon::join internally
        group.bench_with_input(
            BenchmarkId::new("2_original_rayon_join", size),
            size,
            |b, &size| {
                let mut state = create_single_state();
                b.iter(|| {
                    let result: (Vec<u64>, Vec<u64>) = original_random_elements_vec(&mut state, size);
                    black_box(result);
                });
            },
        );

        // 3. Parallel with 2 states
        group.bench_with_input(
            BenchmarkId::new("3_parallel_2_states", size),
            size,
            |b, &size| {
                let mut states = create_states(2);
                b.iter(|| {
                    let mut state_refs: Vec<&mut Rep3State> = states.iter_mut().collect();
                    let result: (Vec<u64>, Vec<u64>) = par_random_elements_vec(&mut state_refs, size);
                    black_box(result);
                });
            },
        );

        // 4. Parallel with 4 states (only if machine has 4+ performance cores)
        group.bench_with_input(
            BenchmarkId::new("4_parallel_4_states", size),
            size,
            |b, &size| {
                let mut states = create_states(4);
                b.iter(|| {
                    let mut state_refs: Vec<&mut Rep3State> = states.iter_mut().collect();
                    let result: (Vec<u64>, Vec<u64>) = par_random_elements_vec(&mut state_refs, size);
                    black_box(result);
                });
            },
        );

        // 5. Parallel with 8 states (for machines with many cores like r8i)
        group.bench_with_input(
            BenchmarkId::new("5_parallel_8_states", size),
            size,
            |b, &size| {
                let mut states = create_states(8);
                b.iter(|| {
                    let mut state_refs: Vec<&mut Rep3State> = states.iter_mut().collect();
                    let result: (Vec<u64>, Vec<u64>) = par_random_elements_vec(&mut state_refs, size);
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Masking elements generation
fn bench_masking_elements_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("masking_elements_comparison");
    
    for size in [1000000].iter() {
        // Truly Sequential
        group.bench_with_input(
            BenchmarkId::new("truly_sequential", size),
            size,
            |b, &size| {
                let mut state = create_single_state();
                b.iter(|| {
                    let result: Vec<u64> = seq_masking_elements_vec(&mut state, size);
                    black_box(result);
                });
            },
        );

        // Parallel with 4 states
        group.bench_with_input(
            BenchmarkId::new("parallel_4_states", size),
            size,
            |b, &size| {
                let mut states = create_states(4);
                b.iter(|| {
                    let mut state_refs: Vec<&mut Rep3State> = states.iter_mut().collect();
                    let result: Vec<u64> = masking_elements_vec_multithreads(&mut state_refs, size);
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Thread pool size scaling test
fn bench_thread_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_scaling");
    let size = 1_000_000; // 1M elements

    for num_states in [1, 2, 4, 6, 8, 12, 16, 32].iter() {
        group.bench_with_input(
            BenchmarkId::new(format!("states_{}", num_states), size),
            num_states,
            |b, &num_states| {
                let mut states = create_states(num_states);
                
                b.iter(|| {
                    let mut state_refs: Vec<&mut Rep3State> = states.iter_mut().collect();
                    let result: (Vec<u64>, Vec<u64>) = 
                        par_random_elements_vec(&mut state_refs, size);
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}



criterion_group!(
    benches,
    bench_random_elements_comparison,
    bench_masking_elements_comparison,
    bench_thread_scaling,
    //bench_small_data_overhead
);
criterion_main!(benches);
