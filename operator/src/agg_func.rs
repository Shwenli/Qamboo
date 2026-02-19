
use itertools::izip;
use rand::distributions::Standard;
use rand::prelude::Distribution;
use random::rep3::Rep3State;
use algebra::ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement};
use protocols::protocols::rep3_ring::{arithmetic,binary};
use protocols::protocols::rep3_ring::Rep3RingShare;
use net::Network;
use primitives::utils;
use primitives::transform;
use primitives::mux;
use primitives::permute;
use crate::group_by;


/// Aggregation count for muti-key group by. Should after group_by.
pub fn table_agg_count_multithreads<T: IntRing2k, N: Network>(
    e: &[Rep3RingShare<T>],
    perm_e: &[Rep3RingShare<u32>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{

    //Get a sequence of 1..V_g.len()
    let t_one = RingElement(T::one());

    let len = e.len();

    let mut vec_x = vec![t_one; len];
    for i in 1..len{
        vec_x[i] = vec_x[i-1] + t_one;
    }

    let mut x = Vec::new();
    let e_m = vec_x[len-1]; 
    for i in 0..len{
        let x_i = mux::mux_if_then_public(&e[i], &vec_x[i], &e_m, states[0])?;
        x.push(x_i);
    }
    
    let y = permute::apply_inv_multithreads(&perm_e, &x, nets, states)?;

    let mut s = y.clone();
    for i in 1..s.len(){
        s[i] = arithmetic::sub(y[i], y[i-1]);
    }

    Ok(s)
}

/// Using valid to count. specifically for distinct count. old valid should be before group_by?
pub fn table_agg_count_by_valid_multithreads<T: IntRing2k, N: Network>(
    old_valid: &[Rep3RingShare<T>],
    perm_e: &[Rep3RingShare<u32>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{ 

    let x = utils::prefix_sum_sequential(old_valid)?;
    
    let y = permute::apply_inv_multithreads(&perm_e, &x, nets, states)?;

    let mut s = y.clone();

    for i in 1..s.len(){
        s[i] = arithmetic::sub(y[i], y[i-1]);
    }

    Ok(s)
}

/// aggregation sum for muti-key group by. after group_by
pub fn table_agg_sum_multithreads<T: IntRing2k, N: Network>(
    v_g: &[Rep3RingShare<T>],
    e: &[Rep3RingShare<T>],
    perm_e: &[Rep3RingShare<u32>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{

    //let (_, v_g, e, _, perm_e, k_out_vec,_) = muti_key_group_by_common(keys, vals, valid, true, 64, net0, net1, state0, state1)?;
    println!("After group_by in agg_sum:");

    let w = utils::prefix_sum_sequential(&v_g)?;
    //println!("w: {:?}",arithmetic::open_vec(&w, net0));

    let w_m = vec![w[w.len()-1];w.len()];
    //println!("w_m: {:?}", arithmetic::open_vec(&w_m, net0)?);

    let x = mux::mux_if_then_share_vec_multithreads(&e, &w, &w_m, nets, states)?;
    //println!("x: {:?}",arithmetic::open_vec(&x, net0));

    let y = permute::apply_inv_multithreads(&perm_e, &x, nets, states)?;
    //println!("y: {:?}",arithmetic::open_vec(&y, net0));

    let mut s = y.clone();
    for i in 1..s.len(){
        s[i] = arithmetic::sub(y[i], y[i-1]);
    }

    Ok(s)
}


/// aggregation min for muti-key group by. after group_by
pub fn table_agg_min_multithreads<T: IntRing2k, N: Network>(
    v_g: &[Rep3RingShare<T>],
    e: &[Rep3RingShare<T>],
    e_bit: &[Rep3RingShare<Bit>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{

    let id = states[0].id;

    // e_now = [1, e[0], e[1], ..., e[len-2]]
    let mut e_now = Vec::new();
    let mut e_bit_now = Vec::new();

    let one =  RingElement(T::one());
    let one_share = arithmetic::promote_to_trivial_share(id, one);
    e_now.push(one_share);
    e_now.extend(e[0..e.len()-1].iter());

    let bit_one = RingElement(Bit::new(true));
    let bit_one_share = binary::promote_to_trivial_share(id, &bit_one);
    e_bit_now.push(bit_one_share);
    e_bit_now.extend(e_bit[0..e_bit.len()-1].iter());


    //[[xi]] ← Ifthen( [[ei−1]] : [[vG [i]]], [[0]])
    let zero = RingElement(T::zero());
    let x = mux::mux_if_share_then_public_vec_multithreads(&e_now, &v_g, &zero, nets, states)?;

    //[[gi]] := 1 − [[ei−1]]
    let g = izip!(e_bit_now).map(|e_i| binary::xor_public(&e_i, &bit_one, id)).collect::<Vec<_>>();
    let g_t = transform::from_bit_to_arithmetic_t_multithreads::<u32,N>(&g, nets, states)?;
    let perm_g = permute::gen_bit_perm_multithreads(g_t, nets, states)?;

    let y = permute::apply_inv_multithreads(&perm_g, &x, nets, states)?;

    Ok(y)
}


/// aggregation max for muti-key group by. after group_by. Grouup by key need to add values.
pub fn table_agg_max_multithreads<T: IntRing2k, N: Network>(
    v_g: &[Rep3RingShare<T>],
    e: &[Rep3RingShare<T>],
    perm_e: &[Rep3RingShare<u32>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{

    // [[xi ]] ← Ifthen( [[ei ]] : [[vG [i]]], [[0]])
    let zero = RingElement(T::zero());
    let x = mux::mux_if_share_then_public_vec_multithreads(&e, &v_g, &zero, nets, states)?;

    let y = permute::apply_inv_multithreads(&perm_e, &x, nets, states)?;

    Ok(y)
}





//************ one thread version *************/
/// aggregation sum for muti-key group by. after group_by
pub fn table_agg_sum<T: IntRing2k, N: Network>(
    v_g: &[Rep3RingShare<T>],
    e: &[Rep3RingShare<T>],
    perm_e: &[Rep3RingShare<u32>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{

    //let (_, v_g, e, _, perm_e, k_out_vec,_) = muti_key_group_by_common(keys, vals, valid, true, 64, net0, net1, state0, state1)?;
    println!("After group_by in agg_sum:");

    //println!("v_g: {:?}",arithmetic::open_vec(&v_g, net0));

    let w = utils::prefix_sum_sequential(&v_g)?;
    //println!("w: {:?}",arithmetic::open_vec(&w, net0));

    let w_m = vec![w[w.len()-1];w.len()];
    //println!("w_m: {:?}", arithmetic::open_vec(&w_m, net0)?);

    let x = mux::mux_if_then_share_vec(&e, &w, &w_m, net, state)?;
    //println!("x: {:?}",arithmetic::open_vec(&x, net0));

    let y = permute::apply_inv(&perm_e, &x, net, state)?;
    //println!("y: {:?}",arithmetic::open_vec(&y, net0));

    //以上测试均没有问题

    let mut s = y.clone();
    for i in 1..s.len(){
        s[i] = arithmetic::sub(y[i], y[i-1]);
    }

    Ok(s)
}

/// Aggregation count for muti-key group by. After group_by.
pub fn table_agg_count<T: IntRing2k, N: Network>(
    v_g: &[Rep3RingShare<T>],
    e: &[Rep3RingShare<T>],
    perm_e: &[Rep3RingShare<u32>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{

    //Get a sequence of 1..V_g.len()
    let t_one = RingElement(T::one());
    let len = v_g.len();
    let mut vec_x = vec![t_one; len];
    for i in 1..len{
        vec_x[i] = vec_x[i-1] + t_one;
    }

    let mut x = Vec::new();
    let e_m = vec_x[len-1]; 
    for i in 0..len{
        let x_i = mux::mux_if_then_public(&e[i], &vec_x[i], &e_m, state)?;
        x.push(x_i);
    }
    
    eprintln!("x: {:?}",arithmetic::open_vec(&x, net));

    let y = permute::apply_inv(&perm_e, &x, net, state)?;

    let mut s = y.clone();
    for i in 1..s.len(){
        s[i] = arithmetic::sub(y[i], y[i-1]);
    }

    Ok(s)

}


/// This algorithm is to replace ccs23 group_by_count by using valid.
pub fn table_agg_count_by_valid<T: IntRing2k, N: Network>(
    old_valid: &[Rep3RingShare<T>],
    perm_e: &[Rep3RingShare<u32>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{ 

    let x = utils::prefix_sum_sequential(old_valid)?;
    
    let y = permute::apply_inv(&perm_e, &x, net, state)?;

    let mut s = y.clone();

    for i in 1..s.len(){
        s[i] = arithmetic::sub(y[i], y[i-1]);
    }

    Ok(s)
}


/// aggregation sum for muti-key group by
pub fn agg_sum<T: IntRing2k, N: Network>(
    keys: Vec<&[Rep3RingShare<T>]>,
    vals: &[Rep3RingShare<T>],
    valid: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Rep3RingShare<T>>,Vec<Vec<Rep3RingShare<T>>>)>
where
Standard: Distribution<T>,{

    let (_, v_g, e, _, perm_e, k_out_vec,_,_) = group_by::muti_key_group_by_common(keys, vals, valid, true, 64, net, state)?;
    println!("After group_by in agg_sum:");
    println!("v_g: {:?}",arithmetic::open_vec(&v_g, net));

    let w = utils::prefix_sum_sequential(&v_g)?;
    println!("w: {:?}",arithmetic::open_vec(&w, net));

    let w_m = vec![w[w.len()-1];w.len()];
    println!("w_m: {:?}", arithmetic::open_vec(&w_m, net)?);

    let x = mux::mux_if_then_share_vec(&e, &w, &w_m, net, state)?;
    println!("x: {:?}",arithmetic::open_vec(&x, net));

    let y = permute::apply_inv(&perm_e, &x, net, state)?;
    println!("y: {:?}",arithmetic::open_vec(&y, net));
    //以上测试均没有问题

    let mut s = y.clone();
    for i in 1..s.len(){
        s[i] = arithmetic::sub(y[i], y[i-1]);
    }

    Ok((s, k_out_vec))
}

pub fn agg_count<T: IntRing2k, N: Network>(
    keys: Vec<&[Rep3RingShare<T>]>,
    vals: &[Rep3RingShare<T>],
    valid: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Rep3RingShare<T>>,Vec<Vec<Rep3RingShare<T>>>)>
where
Standard: Distribution<T>,{

    let (_, v_g, e, _, perm_e, k_out_vec,_,_) = group_by::muti_key_group_by_common(keys, vals, valid, true, 64, net, state)?;
    
    let s = table_agg_count(&v_g, &e, &perm_e, net, state)?;

    Ok((s, k_out_vec))
}

pub fn agg_count_by_valid<T: IntRing2k, N: Network>(
    keys: Vec<&[Rep3RingShare<T>]>,
    vals: &[Rep3RingShare<T>],
    valid: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Rep3RingShare<T>>,Vec<Vec<Rep3RingShare<T>>>)>
where
Standard: Distribution<T>,{

    let (_, _, _, _, perm_e, k_out_vec,old_valid,_) = group_by::muti_key_group_by_common(keys, vals, valid, true, 64, net, state)?;
    
    let s = table_agg_count_by_valid(&old_valid, &perm_e, net, state)?;
    Ok((s, k_out_vec))
}