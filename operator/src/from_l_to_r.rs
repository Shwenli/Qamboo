//! Join
//!
//! This module implements secure join operations between secret-shared tables.
//! The core protocol is FromLtoR from "Secure Statistical Analysis on Multiple Datasets: Join and Group-By"

use rand::distributions::Standard;
use rand::prelude::Distribution;
use protocols::protocols::rep3_ring::{Rep3State};
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::{arithmetic};
use protocols::protocols::rep3_ring::Rep3RingShare;
use net::Network;
use primitives::transform::{bit_decompose_many_multithreads, bit_decompose_many};
use primitives::permute;
use primitives::utils::prefix_sum_sequential;

type PermRing = u32;


/// Performs the FromLtoR join operation.
pub fn from_l_to_r_first<T: IntRing2k, N: Network>(
    k_l: &[Rep3RingShare<T>],  // Left table keys (length m)
    k_r: &[Rep3RingShare<T>],  // Right table keys (length n)
    a: &[Rep3RingShare<T>],    // Left table column values (length m)
    bitsize: usize,             // Bit size for sorting
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<PermRing>>)>
where
    Standard: Distribution<T>,
{
    let m = k_l.len();
    let n = k_r.len();
    
    if m != a.len() {
        eyre::bail!("Left table keys and values must have same length");
    }

    let total_len = m + n + m;
    
    if total_len > PermRing::MAX as usize {
        eyre::bail!("Total length exceeds PermRing capacity");
    }

    // Step 1: Construct concatenated vectors
    // keys: [k_L || k_R || k_L]
    let keys = [k_l,k_r,k_l].concat();
    let key_bits = bit_decompose_many(&keys, bitsize, net, state)?;

    // values: [A || 0^n || -A]
    let mut values = Vec::with_capacity(total_len);
    values.extend_from_slice(a);
    
    // Add n zeros for R's positions
    for _ in 0..n {
        values.push(Rep3RingShare::zero_share());
    }
    
    // Add -A for the second copy of L
    for a_val in a {
        values.push(-*a_val);
    }

    // Step 2: Genperms
    // We sort in ascending order (order = vec![false; bitsize])
    let order = true;
    let perm = permute::gen_perm(
        &key_bits,
        order,
        bitsize,
        net,
        state,
    )?;

    // Step 3: Apply inverse permutation to values
    let values_apply_perm = permute::apply_inv(&perm, &values, net, state)?;
    let opened_g = arithmetic::open_vec(&values_apply_perm, net)?;
    println!("After ApplyPerm: {:?}", opened_g);

    // Step 4: Compute prefix sum on sorted values
    let prefix_sum_values = prefix_sum_sequential(&values_apply_perm)?;
    //let opened_h = arithmetic::open_vec(&prefix_sum_values, net0)?;
    //println!("After PrefixSum: {:?}", opened_h);


    //Step 5: Apply inverse permutation to restore R's original order
    let values_apply_perm_inv = permute::apply_perm(&perm, &prefix_sum_values, net, state)?;
    //let opened_f_prime = arithmetic::open_vec(&values_apply_perm_inv, net)?;
    //println!("After ApplyInv: {:?}", opened_f_prime);

    // Step 6: The result for R is in positions [m, m+n)
    // We need to extract this portion
    // Note: After sorting, the structure should maintain the "sandwich" property
    // where L values surround R values for each matching key
    
    // Extract middle n elements (corresponding to R after sorting and prefix sum)
    let result = values_apply_perm_inv[m..m+n].to_vec();

    Ok((result,perm))
}


/// Generates the permutation for FromLtoR join operation.
pub fn from_l_to_r_gen_perm<T: IntRing2k, N: Network>(
    k_l: &[Rep3RingShare<T>],  // Left table keys (length m)
    k_r: &[Rep3RingShare<T>],  // Right table keys (length n)
    bitsize: usize,             // Bit size for sorting
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>>
where
    Standard: Distribution<T>,
{
    let m = k_l.len();
    let n = k_r.len();

    let total_len = m + n + m;
    
    if total_len > PermRing::MAX as usize {
        eyre::bail!("Total length exceeds PermRing capacity");
    }

    // Step 1: Construct concatenated vectors
    // keys: [k_L || k_R || k_L]
    let keys = [k_l,k_r,k_l].concat();
    let key_bits = bit_decompose_many(&keys, bitsize, net, state)?;

    // Step 2: Genperms
    // We sort in ascending order (order = vec![false; bitsize])
    let order = true;
    let perm = permute::gen_perm(
        &key_bits,
        order,
        bitsize,
        net,
        state,
    )?;

    Ok(perm)
}


pub fn from_l_to_r_other<T: IntRing2k, N: Network>(
    len_m: usize, //length of L
    len_n: usize, //length of R
    perm: &[Rep3RingShare<PermRing>],  // Permutation from sorting
    a: &[Rep3RingShare<T>],    // Left table column values (length m)
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let total_len = len_m + len_n + len_m;

    //values: [A || 0^n || -A]
    let mut values = Vec::with_capacity(total_len);
    values.extend_from_slice(a);
    
    //Add n zeros for R's positions
    for _ in 0..len_n {
        values.push(Rep3RingShare::zero_share());
    }
    
    //Add -A for the second copy of L
    for a_val in a {
        values.push(-*a_val);
    }

    //Apply inverse permutation to values
    let values_apply_perm = permute::apply_inv(&perm, &values, net, state)?;

    //Compute prefix sum on sorted values
    let prefix_sum_values = prefix_sum_sequential(&values_apply_perm)?;

    //Apply inverse permutation to restore R's original order
    let values_apply_perm_inv = permute::apply_perm(&perm, &prefix_sum_values, net, state)?;

    let result = values_apply_perm_inv[len_m..len_m+len_n].to_vec();

    Ok(result)
}


pub fn from_l_to_r_gen_perm_multithreads<T: IntRing2k, N: Network>(
    k_l: &[Rep3RingShare<T>],  // Left table keys (length m)
    k_r: &[Rep3RingShare<T>],  // Right table keys (length n)
    bitsize: usize,             // Bit size for sorting
    nets: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>>
where
    Standard: Distribution<T>,
{
    let m = k_l.len();
    let n = k_r.len();

    let total_len = m + n + m;
    
    if total_len > PermRing::MAX as usize {
        eyre::bail!("Total length exceeds PermRing capacity");
    }

    // Step 1: Construct concatenated vectors
    // keys: [k_L || k_R || k_L]
    let keys = [k_l,k_r,k_l].concat();
    let key_bits = bit_decompose_many_multithreads(&keys, bitsize, nets, states)?;

    // Step 2: Genperms
    // We sort in ascending order (order = vec![false; bitsize])
    let order = true;
    let perm = permute::gen_perm_multithreads(
        &key_bits,
        order,
        bitsize,
        nets,
        state0,
        state1,
        states,
    )?;
    
    // Note: The permutation is generated in ascending order, which is suitable for the FromLtoR join operation.
    Ok(perm)
}

pub fn from_l_to_r_gen_multi_keys_perm_multithreads<T: IntRing2k, N: Network>(
    k_l: &Vec<Vec<Rep3RingShare<T>>>,  // Left table keys (length m)
    k_r: &Vec<Vec<Rep3RingShare<T>>>,  // Right table keys (length n)
    bitsize: usize,             // Bit size for sorting
    nets: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>>
where
    Standard: Distribution<T>,
{
    let m = k_l[0].len();
    let n = k_r[0].len();

    let total_len = m + n + m;
    
    if total_len > PermRing::MAX as usize {
        eyre::bail!("Total length exceeds PermRing capacity");
    }

    let kl_0 = k_l[0].as_slice();
    let kr_0 = k_r[0].as_slice();
    let key0 = [kl_0, kr_0, kl_0].concat();
    let key_bits0 = bit_decompose_many_multithreads(&key0, bitsize, nets, states)?;

    let mut perm = permute::gen_perm_multithreads(
        &key_bits0,
        true,
        bitsize,
        nets,
        state0,
        state1,
        states,
    )?;


    for i in 1..k_l.len() {
        let kl_i = k_l[i].as_slice();
        let kr_i = k_r[i].as_slice();
        let key_i  = [kl_i, kr_i, kl_i].concat();
        let key_i_after_perm = permute::apply_inv_multithreads(&perm, &key_i, nets, state0, state1)?;
        let key_i_bits = bit_decompose_many_multithreads(&key_i_after_perm, bitsize, nets, states)?;

        let perm_i = permute::gen_perm_multithreads(
            &key_i_bits,
            true,
            bitsize,
            nets,
            state0,
            state1,
            states,
        )?;

        perm = permute::compose_perm_multithreads(perm, perm_i, nets, state0)?;
    }

    // Note: The permutation is generated in ascending order, which is suitable for the FromLtoR join operation.
    Ok(perm)
}


pub fn from_l_to_r_other_multithreads<T: IntRing2k, N: Network>(
    len_m: usize, //length of L
    len_n: usize, //length of R
    perm: &[Rep3RingShare<PermRing>],  // Permutation from sorting
    a: &[Rep3RingShare<T>],    // Left table column values (length m)
    net: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    
    let total_len = len_m + len_n + len_m;
    // values: [A || 0^n || -A]
    let mut values = Vec::with_capacity(total_len);
    values.extend_from_slice(a);
    
    // Add n zeros for R's positions
    for _ in 0..len_n {
        values.push(Rep3RingShare::zero_share());
    }
    
    // Add -A for the second copy of L
    for a_val in a {
        values.push(-*a_val);
    }
    //这里有问题，负数会变成大整数

    // Step 3: Apply inverse permutation to values
    let values_apply_perm = permute::apply_inv_multithreads(&perm, &values, net, state0, state1)?;

    // Step 4: Compute prefix sum on sorted values
    let prefix_sum_values = prefix_sum_sequential(&values_apply_perm)?;

    //Step 5: Apply inverse permutation to restore R's original order
    let values_apply_perm_inv = permute::apply_perm_multithreads(&perm, &prefix_sum_values, net, state0, state1)?;

    let result = values_apply_perm_inv[len_m..len_m+len_n].to_vec();

    Ok(result)
}




