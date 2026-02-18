
use itertools::izip;
use std::ops::Neg;
use communication::rep3::multinet_impl::{recv_prev_many_multinet, reshare_many_multinet, send_next_many_multinet};
use num_traits::{One,AsPrimitive};
use rand::{distributions::Standard, prelude::Distribution};
use random::rep3::Rep3State;
use algebra::ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement};
use communication::rep3::id::PartyID;
use protocols::protocols::{rep3_ring::Rep3RingShare};
use protocols::protocols::rep3_ring::conversion;
use random::rep3::rep3rng_rayon::*;
use net::Network;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator};
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;
use crate::kogge_stone_adder::{self, low_depth_binary_add_many_multithreads};



// u32 allows to sort 4*10^9 elements. Inputs of this size require 32*4*10^9*2 bytes, i.e., 256 GB of RAM
type PermRing = u32;

pub fn inject_bit<T: IntRing2k, N: Network>(
    inputs: &[Rep3RingShare<T>],
    bit: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>> {
    let len = inputs.len();
    let mut bits = Vec::with_capacity(len);

    for inp in inputs {
        let a = inp.a.get_bit_for_perm(bit) as PermRing;
        let b = inp.b.get_bit_for_perm(bit) as PermRing;
        bits.push(Rep3RingShare::new_ring(a.into(), b.into()));
    }
    
    conversion::bit_inject_many(&bits, net, state)
}

pub fn inject_bit_multithreads<T: IntRing2k, N: Network>(
    inputs: &[Rep3RingShare<T>],
    bit: usize,
    nets:&[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>> {
    let len = inputs.len();
    let mut bits = Vec::with_capacity(len);

    for inp in inputs {
        let a = inp.a.get_bit_for_perm(bit) as PermRing;
        let b = inp.b.get_bit_for_perm(bit) as PermRing;
        bits.push(Rep3RingShare::new_ring(a.into(), b.into()));
    }

    let results = bit_inject_many_multithreads(&bits, nets, states)?;

    Ok(results)
}

/// Translates a vector of shared bits into a vector of arithmetic sharings of the same bits. See [bit_inject] for details.
pub fn bit_inject_many_multithreads<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    assert!(x.iter().all(|a| a.a.bits() <= 1));
    assert!(x.iter().all(|a| a.b.bits() <= 1));

    let id = states[0].id;
    let len = x.len();

    // Approach: Split the value into x and y and compute an arithmetic xor.
    // The multiplication in the arithmetic xor is done in a special way according to https://eprint.iacr.org/2025/919.pdf

    let (res_a, res_b) = match id {
        PartyID::ID0 => {
            let x0 = masking_elements_vec_multithreads::<RingElement<T>>(states, len);
            
            let res_a: Vec<RingElement<T>> = x0
                .par_iter()
                .zip(x.par_iter())
                .with_min_len(1024)
                .map(|(x0, el)| {
                    let y = el.b;
                    let z0 = y * *x0;
                    *x0 + y - z0 - z0
                })
                .collect();

            // Send to P1
            send_next_many_multinet(nets, &res_a)?;

            // Receive from P2
            let res_b: Vec<RingElement<T>> = recv_prev_many_multinet(nets)?;

            (res_a, res_b)
        }
        PartyID::ID1 => {
            let x1 = masking_elements_vec_multithreads::<RingElement<T>>(states, len);

            let res_a: Vec<RingElement<T>> = x1.par_iter()
            .zip(x.par_iter())
            .with_min_len(1024)  // 至少 1024 元素才拆任务
            .map(|(x1, el)| *x1 + (el.a ^ el.b))
            .collect();

            // Send to P2
            send_next_many_multinet(nets, &res_a)?;

            // Receive from P0
            let res_b: Vec<RingElement<T>> = recv_prev_many_multinet(nets)?;

            (res_a, res_b)
        }
        PartyID::ID2 => {
            // Receive from P1
            let res_b: Vec<RingElement<T>> = recv_prev_many_multinet(nets)?;

            let x2 = masking_elements_vec_multithreads::<RingElement<T>>(states, len);

            let res_a: Vec<RingElement<T>> = x2
                .par_iter()
                .zip(x.par_iter())
                .zip(res_b.par_iter())
                .with_min_len(1024)
                .map(|((x2, el), x1)| {
                    let y = el.a;
                    let z2 = y * (*x1 + *x2);
                    *x2 - z2 - z2
                })
                .collect();

            // Send to P0
            send_next_many_multinet(nets, &res_a)?;
            
            (res_a, res_b)
        }
    };

    Ok(res_a
        .into_iter()
        .zip(res_b)
        .map(|(a, b)| Rep3RingShare::new_ring(a, b))
        .collect())
}


pub fn bit_decompose_many<T: IntRing2k, N: Network>(
    inputs: &[Rep3RingShare<T>],
    bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
    Standard: Distribution<T>,{

    let mask = (RingElement::one() << bitsize) - RingElement::one();
    let mut bits = vec![Rep3RingShare::zero_share(); inputs.len()];

    let mut res_vec = conversion::a2b_many(&inputs, net, state)?;
    for (i, mut binary) in res_vec.drain(..).enumerate() {
        binary &= &mask;
        bits[i] = binary;
    }
   
    Ok(bits)
}

pub fn bit_decompose_many_multithreads<T: IntRing2k, N: Network>(
    inputs: &[Rep3RingShare<T>],
    bitsize: usize,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
    Standard: Distribution<T>,{
        
    let mask = (RingElement::one() << bitsize) - RingElement::one();
    let mut bits = vec![Rep3RingShare::zero_share(); inputs.len()];
    
    let mut input_bit_vec = a2b_many_multithreads(inputs, nets, states)?;

    input_bit_vec.iter_mut().zip(bits.iter_mut()).for_each(|(input_bit, bit_i)| {
        *input_bit &= &mask;
        *bit_i = *input_bit;
    });
    

    Ok(bits)
}

/// transform bit to T but result is also binary share
pub fn from_bit_to_t_one<T: IntRing2k>(
    bit_share: &Rep3RingShare<Bit>,
) -> eyre::Result<Rep3RingShare<T>> 
{
    let bit_share_t = Rep3RingShare::new(
        T::from(bit_share.a.0.convert()), 
        T::from(bit_share.b.0.convert())
    );

    Ok(bit_share_t)
}

/// transform bit to T but result is also binary share
pub fn from_bit_to_t<T: IntRing2k>(
    bit_shares: &[Rep3RingShare<Bit>],
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
{
    let bit_shares_t = bit_shares
        .iter()
        .map(|bit_share| Rep3RingShare::new(
            T::from(bit_share.a.0.convert()), 
            T::from(bit_share.b.0.convert())
        ))
        .collect();

    Ok(bit_shares_t)
}

/// transform bit to T but result is also binary share. Drop bitshares after conversion
pub fn from_bit_to_t_drop<T: IntRing2k>(
    bit_shares: Vec<Rep3RingShare<Bit>>,
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
{
    let bit_shares_t = bit_shares
        .iter()
        .map(|bit_share| Rep3RingShare::new(
            T::from(bit_share.a.0.convert()), 
            T::from(bit_share.b.0.convert())
        ))
        .collect();

    Ok(bit_shares_t)
}

/// transform bit to T , result is arithmetic share
pub fn from_bit_to_arithmetic_t_one<T: IntRing2k, N: Network>(
    e: &Rep3RingShare<Bit>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Rep3RingShare<T>> 
where
Standard: Distribution<T>, {
    let e_res = from_bit_to_t_one::<T>(&e)?;
    let e_t = conversion::bit_inject(&e_res, net, state)?;
    Ok(e_t)
}

/// transform bit to T , result is arithmetic share
pub fn from_bit_to_arithmetic_t<T: IntRing2k, N: Network>(
    e: &[Rep3RingShare<Bit>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
Standard: Distribution<T>, {
    let e_res = from_bit_to_t::<T>(&e)?;
    let e_t = conversion::bit_inject_many(&e_res, net, state)?;
    Ok(e_t)
}



/// from bit to arithmetic T in multithreads, result is arithmetic share
pub fn from_bit_to_arithmetic_t_multithreads<T: IntRing2k, N: Network>(
    e: &[Rep3RingShare<Bit>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
Standard: Distribution<T>, {

    let e_res = from_bit_to_t::<T>(&e)?;

    let e_t = bit_inject_many_multithreads(&e_res, nets, states)?;
    
    Ok(e_t)
}


/// Transforms the replicated shared value x from an arithmetic sharing to a binary sharing. I.e., x = x_1 + x_2 + x_3 gets transformed into x = x'_1 xor x'_2 xor x'_3.
pub fn a2b_many_multithreads<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let id = states[0].id;
    let mut x2 = vec![Rep3RingShare::zero_share(); x.len()];

    let (mut r_vec, r2_vec) = random_elements_vec_multithreads::<RingElement<T>>(states, x.len());
    r_vec.par_iter_mut().zip(r2_vec.par_iter()).for_each(|(r, r2)| {
        *r ^= r2;
    });

    let x01_a = match id {
        PartyID::ID0 => {
            x2.iter_mut().zip(x.iter()).for_each(|(x2_i, x_i)| {
                x2_i.b = x_i.b;
            });
            r_vec
        }

        PartyID::ID1 => {
            let res = x.iter().zip(r_vec.iter()).map(|(x_i, r)| {
                let tmp = x_i.a + x_i.b;
                tmp ^ r
            }).collect::<Vec<_>>();
            res
        }
            
        PartyID::ID2 => {
            x2.iter_mut().zip(x.iter()).for_each(|(x2_i, x_i)| {
                x2_i.a = x_i.a;
            });
            r_vec
        }
    };

    // reshare x01
    let x01_b = reshare_many_multinet(nets, &x01_a)?;
    let x01 = izip!(x01_a, x01_b)
        .map(|(a, b)| Rep3RingShare::new_ring(a, b))
        .collect::<Vec<_>>();

    kogge_stone_adder::low_depth_binary_add_many_multithreads(&x01, &x2, nets, states)
}

/*
pub fn a2b_many_multithreads<T: IntRing2k, N: Network>(
    inputs: &[Rep3RingShare<T>],
    net: &[&N],
    state: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
    Standard: Distribution<T>,{ 
    let len = inputs.len();

    let inputs_tasks = get_task_chunks(&inputs, len, net.len())?;
    let results = net::join_all(
        inputs_tasks.into_iter().zip(net.iter()).zip(state.iter_mut()).map(|((chunk, &n), state)| {
            move || {
                conversion::a2b_many(chunk, n, *state).unwrap_or_else(|e| panic!("A2B failed: {:?}", e))
            }
        })
    );

    let results = results.concat();

    Ok(results)
}
*/

/// A variant of [b2a] that operates on vectors of shared values instead.
pub fn b2a_many_multithreads<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let mut res = vec![Rep3RingShare::zero_share(); x.len()];
    let id = states[0].id;

    let mut r_vec = Vec::with_capacity(x.len());
    let (r,r2) = random_elements_vec_multithreads::<RingElement<T>>(states, x.len());
    r.into_iter().zip(r2.into_iter()).for_each(|(r, r2)| {
        let r = r ^ r2;
        r_vec.push(r);
    });

    match id {
        PartyID::ID0 => {
            let k2 = random_elements2_3keys_vec_multithreads::<RingElement<T>>(states, x.len());
            res.par_iter_mut().with_min_len(1024)
            .zip(k2.0.into_par_iter()).zip(k2.1.into_par_iter()).zip(k2.2.into_par_iter()).for_each(|(((res, k2_0), k2_1), k2_2)| {
                res.b = (k2_0 + k2_1 + k2_2).neg();
            });
        }
        PartyID::ID1 => {
            let k1 = random_elements1_3keys_vec_multithreads::<RingElement<T>>(states, x.len());
            res.par_iter_mut().with_min_len(1024)
            .zip(k1.0.into_par_iter()).zip(k1.1.into_par_iter()).zip(k1.2.into_par_iter()).for_each(|(((res, k1_0), k1_1), k1_2)| {
                res.a = (k1_0 + k1_1 + k1_2).neg();
            });
        }
        PartyID::ID2 => {
            let k1 = random_elements1_3keys_vec_multithreads::<RingElement<T>>(states, x.len());
            let k2 = random_elements2_3keys_vec_multithreads::<RingElement<T>>(states, x.len());
            
            res.par_iter_mut().with_min_len(1024)
            .zip(k1.0.into_par_iter()).zip(k1.1.into_par_iter()).zip(k1.2.into_par_iter())
            .zip(k2.0.into_par_iter()).zip(k2.1.into_par_iter()).zip(k2.2.into_par_iter())
            .zip(r_vec.par_iter_mut())
            .for_each(|(((((((res, k1_0), k1_1), k1_2), k2_0), k2_1), k2_2), y)| {
                let k1_comp = k1_0 + k1_1 + k1_2;
                let k2_comp = k2_0 + k2_1 + k2_2;
                let val = k1_comp + k2_comp;
                *y ^= val;
                res.a = k2_comp.neg();
                res.b = k1_comp.neg();
            });
        }
    }

    // reshare y
    let y_a = r_vec;
    send_next_many_multinet(nets,&y_a)?;
    let local_b = recv_prev_many_multinet(nets)?;

    let y = izip!(y_a, local_b)
        .map(|(a, b)| Rep3RingShare::new_ring(a, b))
        .collect::<Vec<_>>();

    let z = low_depth_binary_add_many_multithreads(x, &y, nets, states)?;

    match id {
        PartyID::ID0 => {
            let z_b = z.iter().cloned().map(|z| z.b).collect::<Vec<_>>();
            send_next_many_multinet(nets,&z_b)?;
            let rcv: Vec<RingElement<T>> = recv_prev_many_multinet(nets)?;

            res.par_iter_mut().with_min_len(1024)
            .zip(z.into_par_iter()).zip(rcv.into_par_iter()).for_each(|((res, z), rcv)| {
                res.a = z.a ^ z.b ^ rcv;
            });
        }
        PartyID::ID1 => {
            let rcv: Vec<RingElement<T>> = recv_prev_many_multinet(nets)?;
            res.par_iter_mut().with_min_len(1024)
            .zip(z.into_par_iter()).zip(rcv.into_par_iter()).for_each(|((res, z), rcv)| {
                res.b = z.a ^ z.b ^ rcv;
            });
        }
        PartyID::ID2 => {
            let z_b = z.into_iter().map(|z| z.b).collect::<Vec<_>>();
            send_next_many_multinet(nets,&z_b)?;
        }
    }
    Ok(res)
}


/// An upcast of a Rep3RingShare from a smaller ring to a larger ring
/// Does require network interaction
pub fn upcast_a2b_many_multithreads<T, U, N>(
    shares: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut[&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<U>>>
where
    T: IntRing2k + AsPrimitive<U>,
    U: IntRing2k,
    N: Network,
    Standard: Distribution<T> + Distribution<U>,
{
    assert!(T::K < U::K);

    let binary_vec = a2b_many_multithreads(shares, nets, states)?;

    let binary_vec = binary_vec.iter().map(|binary| {
        Rep3RingShare {
            a: RingElement(binary.a.0.as_()),
            b: RingElement(binary.b.0.as_()),
        }
    }).collect::<Vec<_>>();

    let result = b2a_many_multithreads(&binary_vec, nets, states)?;

    Ok(result)
}

/*
/// Translates a vector of shared bits into a vector of arithmetic sharings of the same bits. See [bit_inject] for details.
pub fn bit_inject_many_multithreads<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    assert!(x.iter().all(|a| a.a.bits() <= 1));
    assert!(x.iter().all(|a| a.b.bits() <= 1));

    let id = states[0].id;

    let mut res_a = Vec::with_capacity(x.len());

    // Approach: Split the value into x and y and compute an arithmetic xor.
    // The multiplication in the arithmetic xor is done in a special way according to https://eprint.iacr.org/2025/919.pdf

    let res_b = match id {
        PartyID::ID0 => {
            let x0_vec = 
            for el in x.iter() {
                let x0 = states[0].rngs.rand.masking_element::<RingElement<T>>();
                let y = el.b;
                let z0 = y * x0;
                let r0 = x0 + y - z0 - z0;
                res_a.push(r0);
            }
            // Send to P1
            net.send_next_many(&res_a)?;

            // Receive from P2
            let res_b: Vec<RingElement<T>> = net.recv_prev_many()?;
            if res_b.len() != x.len() {
                eyre::bail!("Received wrong number of elements");
            }
            res_b
        }
        PartyID::ID1 => {
            for el in x.iter() {
                let x1 = state.rngs.rand.masking_element::<RingElement<T>>();
                res_a.push(x1 + (el.a ^ el.b));
            }
            // Send to P2
            net.send_next_many(&res_a)?;

            // Receive from P0
            let res_b: Vec<RingElement<T>> = net.recv_prev_many()?;
            if res_b.len() != x.len() {
                eyre::bail!("Received wrong number of elements");
            }
            res_b
        }
        PartyID::ID2 => {
            // Receive from P1
            let res_b: Vec<RingElement<T>> = net.recv_prev_many()?;
            if res_b.len() != x.len() {
                eyre::bail!("Received wrong number of elements");
            }

            for (el, x1) in izip!(x.iter(), res_b.iter()) {
                let x2 = state.rngs.rand.masking_element::<RingElement<T>>();
                let y = el.a;
                let z2 = y * (*x1 + x2);
                let r2 = x2 - z2 - z2;
                res_a.push(r2);
            }

            // Send to P0
            net.send_next_many(&res_a)?;
            res_b
        }
    };

    Ok(res_a
        .into_iter()
        .zip(res_b)
        .map(|(a, b)| Rep3RingShare::new_ring(a, b))
        .collect())
}
*/