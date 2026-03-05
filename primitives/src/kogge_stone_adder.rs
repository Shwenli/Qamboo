//! Implementation of the low-depth binary addition
//! This module provides functions for performing low-depth binary addition
use protocols::protocols::rep3_ring::{Rep3RingShare, binary};
use algebra::ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement};
use random::rep3::Rep3State;
use itertools::izip;
use net::Network;
use num_traits::{One, Zero};
//use random::rep3::rep3rng_rayon::random_elements_vec_multithreads;
use communication::rep3::net_impl::Rep3NetworkImpl;
//use communication::rep3::multinet_impl::{send_next_many_multinet, recv_prev_many_multinet};
use rayon::slice::ParallelSliceMut;
use crate::compare::and_vec_multithreads;
use rand::{distributions::Standard, prelude::Distribution};
use rayon::iter::*;



/// Adds a vector of shared values x1 and a vector of public constants x2
pub fn low_depth_binary_add_const_many<T: IntRing2k, N: Network>(
    x1: &[Rep3RingShare<T>],
    x2: &[RingElement<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    // 1. P = x1 ^ x2
    let mut p = izip!(x1, x2)
        .map(|(s, c)| binary::xor_public(s, c, state.id))
        .collect::<Vec<_>>();

    // 2. G = x1 & x2 (Local AND)
    let mut g = izip!(x1, x2)
        .map(|(s, c)| s & c)
        .collect::<Vec<_>>();

    // 3. Kogge-Stone Inner
    kogge_stone_inner_many(&mut p, &mut g, net, state)?;
    
    Ok(g)
}

/// low-depth binary addition many
pub fn low_depth_binary_add_many<T: IntRing2k, N: Network>(
    x1: &[Rep3RingShare<T>],
    x2: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let mut p = izip!(x1, x2).map(|(x1, x2)| x1 ^ x2).collect::<Vec<_>>();
    let mut g = binary::and_vec(x1, x2, net, state)?;
    kogge_stone_inner_many(&mut p, &mut g, net, state)?;
    Ok(g)
}

// !Error
pub fn low_depth_binary_add_many_multithreads<T: IntRing2k, N: Network>(
    x1: &[Rep3RingShare<T>],
    x2: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let mut p = x1.par_iter()
        .zip_eq(x2.par_iter())
        .with_min_len(1024)
        .map(|(x1_i, x2_i)| x1_i ^ x2_i)
        .collect::<Vec<_>>();

    //let mut g = binary::and_vec(x1, x2, nets[0], states[0])?;
    let mut g = and_vec_multithreads(x1, x2, nets, states)?;
    kogge_stone_inner_many_multithreads(&mut p, &mut g, nets, states)?;

    Ok(g)
}


fn kogge_stone_inner_many<T: IntRing2k, N: Network>(
    p: &mut [Rep3RingShare<T>],
    g: &mut [Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<()>
where
    Standard: Distribution<T>,
{
    let bitlen = T::K;
    let d: u32 = bitlen.ilog2(); // T is a ring with 2^k elements
    debug_assert!(bitlen.is_power_of_two());

    let s_ = p.to_owned();

    for i in 0..d {
        let shift = 1 << i;
        let p_ = p.iter().map(|el| el << shift);
        let g_ = g.iter().map(|el| el << shift);
    
        let (r1, r2) = and_twice_many_iter(p, g_, p_, net, state)?;
        for (p, r2) in izip!(p.iter_mut(), r2) {
            *p = r2;
        }
        for (g, r1) in izip!(g.iter_mut(), r1) {
            *g ^= r1;
        }
    }

    for (g, s_) in izip!(g.iter_mut(), s_) {
        *g <<= 1;
        *g ^= s_;
    }
    Ok(())
}

pub fn kogge_stone_inner_many_multithreads<T: IntRing2k, N: Network>(
    p: &mut [Rep3RingShare<T>],
    g: &mut [Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<()>
where
    Standard: Distribution<T>,
{
    let bitlen = T::K;
    let d: u32 = bitlen.ilog2(); // T is a ring with 2^k elements
    debug_assert!(bitlen.is_power_of_two());

    let s_ = p.to_owned();

    let num_threads = nets.len();
    let chunk_size = (p.len() + num_threads - 1) / num_threads; 

    p.par_chunks_mut(chunk_size)
    .zip_eq(g.par_chunks_mut(chunk_size))
    .zip(nets.par_iter())
    .zip(states.par_iter_mut())
    .for_each(|(((p_chunk, g_chunk), net), state)| {
        for i in 0..d {
            let shift = 1 << i;
            let p_: Vec<_> = p_chunk.iter().map(|el| el << shift).collect();
            let g_: Vec<_> = g_chunk.iter().map(|el| el << shift).collect();

            let (r1, r2) = and_twice_many_iter(&p_chunk, g_.into_iter(), p_.into_iter(), *net, state)
            .unwrap_or_else(|e|panic!("Error in and_twice_many_iter: {:?}", e));
            p_chunk.clone_from_slice(&r2);
            for (g, r1) in g_chunk.iter_mut().zip(r1) {
                *g ^= r1;
            }
        }
    });

    g.par_iter_mut()
    .zip_eq(s_.into_par_iter())
    .with_min_len(1024)
    .for_each(|(g, s_)| {
        *g <<= 1;
        *g ^= s_;
    });

    Ok(())
}



fn kogge_stone_inner_with_carry_many<T: IntRing2k, N: Network>(
    p: &[Rep3RingShare<T>],
    g: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<Bit>>)>
where
    Standard: Distribution<T>,
{
    let mut g = kogge_stone_loop_many(p.to_vec(), g.to_vec(), net, state)?;
    let mut carries = Vec::with_capacity(g.len());
    for g_item in &mut g {
        carries.push(g_item.get_bit(T::K - 1));
        *g_item <<= 1;
        *g_item ^= p[carries.len() - 1];  // 注意：这里假设 p 和 g 的长度相同，且索引对应
    }
    Ok((g, carries))
}

fn kogge_stone_inner_with_carry_many_multithreads<T: IntRing2k, N: Network>(
    p: &[Rep3RingShare<T>],
    g: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<Bit>>)>
where
    Standard: Distribution<T>,
{
    let mut g = kogge_stone_loop_many_multithreads(p.to_vec(), g.to_vec(), nets, states)?;
    let carries: Vec<_> = g
    .par_iter_mut()
    .with_min_len(1024)
    .enumerate()
    .map(|(i, g_item)| {
        let carry = g_item.get_bit(T::K - 1);
        *g_item <<= 1;
        *g_item ^= p[i];
        carry
    })
    .collect();
    
    Ok((g, carries))
}

fn kogge_stone_loop_many<T: IntRing2k, N: Network>(
    mut p: Vec<Rep3RingShare<T>>,
    mut g: Vec<Rep3RingShare<T>>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let bitlen = T::K;
    let d: u32 = bitlen.ilog2(); // T is a ring with 2^k elements
    debug_assert!(bitlen.is_power_of_two());

    for i in 0..d {
        let shift = 1 << i;
        let p_: Vec<_> = p.iter().map(|el| el << shift).collect();
        let g_: Vec<_> = g.iter().map(|el| el << shift).collect();
        //let p_ = p.iter().map(|el| el << shift);
        //let g_ = g.iter().map(|el| el << shift);
        // TODO: Make and more communication efficient, ATM we send the full element for each level, even though they reduce in size
        // maybe just input the mask into AND?
        let (r1, r2) = and_twice_many_iter(&p, g_.into_iter(), p_.into_iter(), net, state)?;
        p = r2;
        for (g, r1) in g.iter_mut().zip(r1) {
            *g ^= r1;
        }
    }
    Ok(g)
}

fn kogge_stone_loop_many_multithreads<T: IntRing2k, N: Network>(
    p: Vec<Rep3RingShare<T>>,
    g: Vec<Rep3RingShare<T>>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let bitlen = T::K;

    // T is a ring with 2^k elements
    let d: u32 = bitlen.ilog2(); 
    debug_assert!(bitlen.is_power_of_two());

    let mut p = p;
    let mut g = g;

    // Adjust chunk size based on number of threads and data size
    let num_threads = nets.len();
    let chunk_size = (p.len() + num_threads - 1) / num_threads; 

    p.par_chunks_mut(chunk_size)
    .zip_eq(g.par_chunks_mut(chunk_size))
    .zip(nets.par_iter())
    .zip(states.par_iter_mut())
    .for_each(|(((p_chunk, g_chunk), net), state)| {
        for i in 0..d {
            let shift = 1 << i;
            let p_: Vec<_> = p_chunk.iter().map(|el| el << shift).collect();
            let g_: Vec<_> = g_chunk.iter().map(|el| el << shift).collect();

            let (r1, r2) = and_twice_many_iter(&p_chunk, g_.into_iter(), p_.into_iter(), *net, state)
            .unwrap_or_else(|e|panic!("Error in and_twice_many_iter: {:?}", e));
            p_chunk.clone_from_slice(&r2);
            for (g, r1) in g_chunk.iter_mut().zip(r1) {
                *g ^= r1;
            }
        }
    });
    
    Ok(g)
}



#[expect(clippy::type_complexity)]
fn and_twice_many_iter<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b1: impl Iterator<Item = Rep3RingShare<T>>,
    b2: impl Iterator<Item = Rep3RingShare<T>>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<T>>)>
where
    Standard: Distribution<T>,
{
    let mut local_a1 = Vec::with_capacity(a.len());
    let mut local_a2 = Vec::with_capacity(a.len());
    for (a, b1, b2) in izip!(a, b1, b2) {
        let (mut mask1, mask_b) = state.rngs.rand.random_elements::<RingElement<T>>();
        mask1 ^= mask_b;

        let (mut mask2, mask_b) = state.rngs.rand.random_elements::<RingElement<T>>();
        mask2 ^= mask_b;
        local_a1.push((&b1 & a) ^ mask1);
        local_a2.push((a & &b2) ^ mask2);
    }

    net.send_next([local_a1.to_owned(), local_a2.to_owned()])?;
    let [local_b1, local_b2] = net.recv_prev::<[Vec<RingElement<T>>; 2]>()?;

    let mut r1 = Vec::with_capacity(a.len());
    let mut r2 = Vec::with_capacity(a.len());

    for (local_a1, local_b1, local_a2, local_b2) in izip!(local_a1, local_b1, local_a2, local_b2) {
        r1.push(Rep3RingShare::new_ring(local_a1, local_b1));
        r2.push(Rep3RingShare::new_ring(local_a2, local_b2));
    }

    Ok((r1, r2))
}

/* 
#[expect(clippy::type_complexity)]
fn and_twice_many_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b1: Vec<Rep3RingShare<T>>,
    b2: Vec<Rep3RingShare<T>>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<T>>)>
where
    Standard: Distribution<T>,
{
    //let mut local_a1 = Vec::with_capacity(a.len());
    //let mut local_a2 = Vec::with_capacity(a.len());

    let (mut mask1, mask_b) = random_elements_vec_multithreads::<RingElement<T>>(states, a.len() );
    mask1.par_iter_mut()
    .zip_eq(mask_b.par_iter())
    .with_min_len(1024)
    .for_each(|(m1, mb)| {
        *m1 ^= *mb;
    });

    let (mut mask2, mask_b) = random_elements_vec_multithreads::<RingElement<T>>(states, a.len());
    mask2.par_iter_mut()
    .zip_eq(mask_b.par_iter())
    .with_min_len(1024)
    .for_each(|(m2, mb)| {
        *m2 ^= *mb;
    });

    let (local_a1, local_a2):(Vec<RingElement<T>>, Vec<RingElement<T>>) = a.par_iter()
        .zip_eq(b1.par_iter())
        .zip_eq(b2.par_iter())
        .zip_eq(mask1.par_iter())
        .zip_eq(mask2.par_iter())
        .with_min_len(1024)
        .map(|((((a, b1), b2), mask1i), mask2i)| {
            let local_a1 = (b1 & a) ^ mask1i;
            let local_a2 = (a & b2) ^ mask2i;
            (local_a1, local_a2)
        }).unzip();

    send_next_many_multinet(nets, &[local_a1.to_owned(), local_a2.to_owned()].concat())?;
    let result = recv_prev_many_multinet::<RingElement<T>, N>(nets)?;
    let (local_b1, local_b2) = result.split_at(a.len());

    let (r1, r2): (Vec<_>, Vec<_>) = local_a1.par_iter()
    .zip_eq(local_b1.par_iter())
    .zip_eq(local_a2.par_iter())
    .zip_eq(local_b2.par_iter())
    .with_min_len(1024)
    .map(|(((a1, b1), a2), b2)| (Rep3RingShare::new_ring(*a1, *b1), Rep3RingShare::new_ring(*a2, *b2)))
    .unzip();

    Ok((r1, r2))
}
*/

pub fn low_depth_binary_sub_with_carry_many<T: IntRing2k, N: Network>(
    x1: &[Rep3RingShare<T>],
    x2: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<Bit>>)>
where
    Standard: Distribution<T>,
{
    // bitnot of x2
    let x2 = x2.iter().map(|x| !x).collect::<Vec<_>>();
    // Now start the Kogge-Stone adder
    let p = izip!(x1, &x2).map(|(x1, x2)| x1 ^ x2).collect::<Vec<_>>();
    let mut g = binary::and_vec(x1, &x2, net, state)?;
    // Since carry_in = 1, we need to XOR the LSB of x1 and x2 to g (i.e., xor the LSB of p)
    for (g_item, p_item) in izip!(g.iter_mut(), p.iter()) {
        *g_item ^= *p_item & RingElement::one();
    }

    let (mut res, c) = kogge_stone_inner_with_carry_many(&p, &g, net, state)?;
    // cin=1
    for res_item in &mut res {
        *res_item = binary::xor_public(res_item, &RingElement::one(), state.id);
    }
    Ok((res, c))
}


pub fn low_depth_binary_sub_with_carry_many_multithreads<T: IntRing2k, N: Network>(
    x1: &[Rep3RingShare<T>],
    x2: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<Bit>>)>
where
    Standard: Distribution<T>,
{
    let (x2,p):(Vec<_>,Vec<_>) = x1.par_iter()
    .zip_eq(x2.par_iter())
    .with_min_len(1024)
    .map(|(x1_i, x2_i)| {
        let x2_i = !x2_i;
        let p_i = *x1_i ^ x2_i;
        (x2_i, p_i)
    }).unzip();
    
    let mut g = and_vec_multithreads(x1, &x2, nets, states)?;
    
    // Since carry_in = 1, we need to XOR the LSB of x1 and x2 to g (i.e., xor the LSB of p)
    g.par_iter_mut()
    .zip_eq(p.par_iter())
    .with_min_len(1024)
    .for_each(|(g_item, p_item)| {
        *g_item ^= *p_item & RingElement::one();
    });

    let (mut res, c) = kogge_stone_inner_with_carry_many_multithreads(&p, &g, nets, states)?;
    // cin=1
    res.par_iter_mut()
    .with_min_len(1024)
    .for_each(|res_item| {
        *res_item = binary::xor_public(res_item, &RingElement::one(), states[0].id);
    });
    Ok((res, c))
}

/// Calculates 2^k + x1 - x2 for vectors
pub fn low_depth_binary_sub_by_const_with_carry_many<T: IntRing2k, N: Network>(
    x1: &[Rep3RingShare<T>],
    x2: &[RingElement<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<Bit>>)>
where
    Standard: Distribution<T>,
{
    let x2_: Vec<_> = x2.iter().map(|val| !val + RingElement::one()).collect();

    // Add x1 + x2_ via a packed Kogge-Stone adder
    let p: Vec<_> = izip!(x1, &x2_)
        .map(|(x1_i, x2_i)| binary::xor_public(x1_i, x2_i, state.id))
        .collect();

    let g: Vec<_> = izip!(x1, &x2_).map(|(x1_i, x2_i)| x1_i & x2_i).collect();

    let (res, mut carries) = kogge_stone_inner_with_carry_many(&p, &g, net, state)?;

    // Correct the carry for cases where x2[i] was zero
    for (c, x2_i) in izip!(carries.iter_mut(), x2.iter()) {
        if x2_i.is_zero() {
            // We cut off the carry in the two's complement, so we have to xor in the end
            *c = !*c;
        }
    }

    Ok((res, carries))  
}

pub fn low_depth_binary_sub_by_const_with_carry_many_multithreads<T: IntRing2k, N: Network>(
    x1: &[Rep3RingShare<T>],
    x2: &[RingElement<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<Bit>>)>
where
    Standard: Distribution<T>,
{
    let id = states[0].id;

    let (_x2_, (p, g)): (Vec<_>, (Vec<_>, Vec<_>)) = x1.par_iter()
    .zip_eq(x2.par_iter())
    .with_min_len(1024)
    .map(|(x1_i, x2_i)| {
        let x2_i = !x2_i + RingElement::one();
        let p_i = binary::xor_public(x1_i, &x2_i, id);
        let g_i = *x1_i & x2_i;
        (x2_i, (p_i, g_i))
    }).unzip();

    let (res, mut carries) = kogge_stone_inner_with_carry_many_multithreads(&p, &g, nets, states)?;

    // Correct the carry for cases where x2[i] was zero
    carries.par_iter_mut()
    .zip_eq(x2.par_iter())
    .with_min_len(1024)
    .for_each(|(c, x2_i)| {
        if x2_i.is_zero() {
            *c = !*c;
        }
    });

    Ok((res, carries))  
}


/// Calculates 2^k + x1 - x2 for vectors
pub fn low_depth_binary_sub_from_const_with_carry_many<T: IntRing2k, N: Network>(
    x1: &[RingElement<T>],
    x2: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<Bit>>)>
where
    Standard: Distribution<T>,
{
    let x2_: Vec<_> = x2.iter().map(|val| !val).collect();

    // Add x1 + x2_ via a packed Kogge-Stone adder, where carry_in = 1
    let p: Vec<_> = izip!(x1, &x2_)
        .map(|(x1_i, x2_i)| binary::xor_public(x2_i, x1_i, state.id))
        .collect();

    let g: Vec<_> = izip!(x1, &x2_).map(|(x1_i, x2_i)| x2_i & x1_i).collect();
    
    let g: Vec<_> = izip!(g, &p).map(|(g_i, p_i)| g_i ^ (*p_i & RingElement::one())).collect();

    let (res, c) = kogge_stone_inner_with_carry_many(&p, &g, net, state)?;

    let res: Vec<_> = res
        .into_iter()
        .map(|res_i| binary::xor_public(&res_i, &RingElement::one(), state.id))
        .collect(); // cin=1

    Ok((res, c))
}

pub fn low_depth_binary_sub_from_const_with_carry_many_multithreads<T: IntRing2k, N: Network>(
    x1: &[RingElement<T>],
    x2: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<Bit>>)>
where
    Standard: Distribution<T>,
{
    let id= states[0].id;

    let (_x2_, (p, g)): (Vec<_>, (Vec<_>, Vec<_>)) = x1.par_iter()
    .zip_eq(x2.par_iter())
    .with_min_len(1024)
    .map(|(x1_i, x2_i)| {
        let x2_i = !x2_i;
        let p_i = binary::xor_public(&x2_i, &x1_i, id);
        let g_i = x2_i & *x1_i;
        let g_i = g_i ^ (p_i & RingElement::one()); 
        (x2_i, (p_i, g_i))
    }).unzip();

    let (res, c) = kogge_stone_inner_with_carry_many_multithreads(&p, &g, nets, states)?;

    let res: Vec<_> = res
        .into_par_iter()
        .with_min_len(1024)
        .map(|res_i| binary::xor_public(&res_i, &RingElement::one(), id))
        .collect(); // cin=1

    Ok((res, c))
}
