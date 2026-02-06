
use protocols::protocols::rep3_ring::{Rep3State};
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::{rep3_ring::Rep3RingShare};
use net::Network;
use rand::distributions::Standard;
use rand::prelude::Distribution;
use primitives::permute;
use primitives::transform::{bit_decompose_many_multithreads, bit_decompose_many};


pub fn radix_sort_by_key_multithreads<T: IntRing2k, N: Network>(
    inputs: Vec<Rep3RingShare<T>>,
    order: bool,
    bitsize: usize,
    net: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
    Standard: Distribution<T>,{
    
    let key_bits = bit_decompose_many_multithreads(&inputs, bitsize, net, states)?;

    let perm = permute::gen_perm_multithreads(
        &key_bits,
        order,
        bitsize,
        net,
        state0,
        state1,
        states,
    )?;

    let result = permute::apply_inv_multithreads(&perm, &inputs, net, state0, state1)?;

    Ok(result)
}

///In place version Table sort 
pub fn radix_sort_by_key_in_place_multithreads<T: IntRing2k, N: Network>(
    key: &[Rep3RingShare<T>],
    order: bool,
    inputs: &mut [&mut [Rep3RingShare<T>]],
    bitsize: usize,
    nets: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
    states: &mut [&mut Rep3State],
) -> eyre::Result<()>
where
    Standard: Distribution<T>,{
    
    let key_bits = bit_decompose_many_multithreads(&key, bitsize, nets, states)?;

    let perm = permute::gen_perm_multithreads(
        &key_bits, order, bitsize, nets, state0, state1, states,
    )?;

    for inp in inputs {
        permute::apply_inv_in_place_multithreads(&perm, inp, nets, state0, state1)?;
    }

    Ok(())
}




//************ one thread version *************/

pub fn radix_sort_by_key<T: IntRing2k, N: Network>(
    key: &[Rep3RingShare<T>],
    order: bool,
    inputs: Vec<&[Rep3RingShare<T>]>,
    bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Vec<Rep3RingShare<T>>>> 
where
    Standard: Distribution<T>,{
    
    let mut results = Vec::with_capacity(inputs.len());

    let key_bits = bit_decompose_many(&key, bitsize, net, state)?;

    let perm = permute::gen_perm(
        &key_bits, order, bitsize, net, state,
    )?;

    for inp in inputs {
        results.push(permute::apply_inv(&perm, inp, net, state)?)
    }

    Ok(results)
}

///In place version Table sort 
pub fn radix_sort_by_key_in_place<T: IntRing2k, N: Network>(
    key: &[Rep3RingShare<T>],
    order: bool,
    inputs: &mut [&mut [Rep3RingShare<T>]],
    bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<()>
where
    Standard: Distribution<T>,{
    
    let key_bits = bit_decompose_many(&key, bitsize, net, state)?;

    let perm = permute::gen_perm(
        &key_bits, order, bitsize, net, state,
    )?;

    for inp in inputs {
        permute::apply_inv_in_place(&perm, inp, net, state)?;
    }

    Ok(())
}
