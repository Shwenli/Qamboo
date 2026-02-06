
use protocols::protocols::rep3_ring::{Rep3State};
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::rep3_ring::arithmetic;
use protocols::protocols::rep3_ring::Rep3RingShare;
use net::Network;
use rand::distributions::Standard;
use rand::prelude::Distribution;
use primitives::mux;
use primitives::permute::{apply_inv, apply_inv_multithreads, apply_perm, apply_perm_multithreads};
use primitives::utils::{reshare_vec_q_multithreads,prefix_sum_sequential};
use crate::from_l_to_r;

type PermRing = u32;

fn compute_jlk_multithreads<T: IntRing2k, N: Network>(
    len_m: usize,
    len_n: usize,
    perm: &[Rep3RingShare<PermRing>],
    l:  &[Rep3RingShare<T>],
    net: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
    
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let jl_k = from_l_to_r::from_l_to_r_other_multithreads(
        len_m,
        len_n,
        &perm,
        &l,
        net,
        state0,
        state1,
    )?;

    Ok(jl_k)
}

fn compute_c_multithreads<T: IntRing2k, N: Network>(
    len_m: usize,
    len_n: usize,
    perm: &[Rep3RingShare<PermRing>],
    net: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    // F_one
    let vec_one: Vec<Rep3RingShare<T>> = (0..len_m).map(|_| arithmetic::promote_to_trivial_share(state0.id, RingElement(T::one()))).collect();

    let c= from_l_to_r::from_l_to_r_other_multithreads(
        len_m,
        len_n,
        &perm,
        &vec_one,
        net,
        state0,
        state1,
    )?;
    
    Ok(c)
}

fn compute_jrj_multithreads<T: IntRing2k, N: Network>(
    c: &[Rep3RingShare<T>],
    r:  &[Rep3RingShare<T>],
    net: &[&N],
    state0: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let jrj_tmp = arithmetic::local_mul_vec(c, r, state0);

    let results = reshare_vec_q_multithreads(&jrj_tmp, net)?;

    Ok(results)
}

fn compute_vj_multithreads<T: IntRing2k, N: Network>(
    len_m: usize,
    len_n: usize,
    perm: &[Rep3RingShare<PermRing>],
    vl: &[Rep3RingShare<T>],
    vr:  &[Rep3RingShare<T>],
    net: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let v= from_l_to_r::from_l_to_r_other_multithreads(
        len_m,
        len_n,
        &perm,
        &vl,
        net,
        state0,
        state1,
    )?;

    let vj_tmp = arithmetic::local_mul_vec(&v, vr, state0);
    
    //TODO: mutithreaded reshare
    let results = reshare_vec_q_multithreads(&vj_tmp, net)?;
    
    Ok(results)
}


pub fn only_in_l_multithreads<T: IntRing2k, N: Network>(
    k_l: Vec<Rep3RingShare<T>>,
    k_r: Vec<Rep3RingShare<T>>,
    bitsize: usize,
    net: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
    state: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,{
    
    let m = k_l.len();
    let n = k_r.len();

    let perm = from_l_to_r::from_l_to_r_gen_perm_multithreads(&k_l, &k_r, bitsize, net, state0, state1, state)?;
    
    // generate n zeros
    let vec_zero: Vec<Rep3RingShare<T>> = (0..n).map(|_| arithmetic::promote_to_trivial_share(state0.id, RingElement(T::zero()))).collect();
    
    // generate m ones and m neg ones
    let vec_one_1: Vec<Rep3RingShare<T>> = (0..m).map(|_| arithmetic::promote_to_trivial_share(state0.id, RingElement(T::one()))).collect();
    let vec_one_2: Vec<Rep3RingShare<T>> = (0..m).map(|_| arithmetic::promote_to_trivial_share(state0.id, -RingElement(T::one()))) .collect();
    
    // make f a vector of m ones, n zeros, m -ones
    let f = [vec_one_1, vec_zero, vec_one_2].concat();

    //set g to be a vector by applying perm to f
    let g = apply_inv_multithreads(&perm, &f, net, state0, state1)?;

    // compute prefix sum of g
    let h = prefix_sum_sequential(&g)?;

    // compute p 论文的算法有问题，应该是p[i] = 1-h[i+1].这里就直接对位置减1，然后序列前移，再补尾元素
    let one = RingElement(T::one());
    let mut p = h.iter().map(|h_i| arithmetic::sub_public_by_shared(one, *h_i, state1.id)).collect::<Vec<_>>();
    p.remove(0); //移除p的第一个元素
    p.push(h[2*m+n-1].clone()); //添加最后一个元素

    // compute f_res
    let f_res = apply_perm_multithreads(&perm, &p, net, state0, state1)?;
    
    Ok(f_res[..m].to_vec())
}


pub fn inner_join_table_multithreads<T: IntRing2k, N: Network>(
    k_l: Vec<Rep3RingShare<T>>,
    k_r: Vec<Rep3RingShare<T>>,
    val_l: Vec<&[Rep3RingShare<T>]>,
    val_r: Vec<&[Rep3RingShare<T>]>,
    valid_l: Vec<Rep3RingShare<T>>,
    valid_r: Vec<Rep3RingShare<T>>,
    bitsize: usize,
    nets: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Vec<Rep3RingShare<T>>>>
where
    Standard: Distribution<T>,
{
    let d = val_l.len();
    let e = val_r.len();
    let (len_m, len_n) = (k_l.len(), k_r.len());

    let perm = from_l_to_r::from_l_to_r_gen_perm_multithreads(&k_l, &k_r, bitsize, nets, state0, state1, states)?;
    
    //* j is final tabla. d is the number of value columns in left table, e is the number of  value columns in right table
    //* 2: k_r and v_j
    let mut j: Vec<Vec<Rep3RingShare<T>>> = Vec::with_capacity(d+e+2);

    //* k_r remain the same order without permutation
    j.push(k_r);

    //* compute jl_k, l_k is column form left table
    for l_k in val_l{
        let jl_k = compute_jlk_multithreads(len_m, len_n, &perm, l_k, nets, state0, state1)?;
        j.push(jl_k);
        
    }

    let c = compute_c_multithreads(len_m, len_n, &perm, nets, state0, state1)?;

    for r_j in val_r{
        let jr_j = compute_jrj_multithreads(&c, r_j, nets, state0)?;
        j.push(jr_j);
    }

    let valid_j = compute_vj_multithreads(len_m, len_n, &perm, &valid_l, &valid_r, nets, state0, state1)?;
    j.push(valid_j);

    Ok(j)
}


pub fn inner_join_table_multi_keys_multithreads<T: IntRing2k, N: Network>(
    k_l: Vec<Vec<Rep3RingShare<T>>>,
    k_r: Vec<Vec<Rep3RingShare<T>>>,
    val_l: Vec<&[Rep3RingShare<T>]>,
    val_r: Vec<&[Rep3RingShare<T>]>,
    valid_l: Vec<Rep3RingShare<T>>,
    valid_r: Vec<Rep3RingShare<T>>,
    bitsize: usize,
    nets: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
    states: &mut [&mut Rep3State],
)-> eyre::Result<Vec<Vec<Rep3RingShare<T>>>>
where
    Standard: Distribution<T>,{

    let d = val_l.len();
    let e = val_r.len();
    let k_len = k_l.len();
    //let (len_m, len_n) = (k_l.len(), k_r.len());
    let (len_m, len_n) = (valid_l.len(), valid_r.len());

    let perm = from_l_to_r::from_l_to_r_gen_multi_keys_perm_multithreads(&k_l, &k_r, bitsize, nets, state0, state1, states)?;

    /*
    let test_k = [k_l[0].clone(), k_r[0].clone(), k_l[0].clone()].concat();
    let test_k_2 = [k_l[1].clone(), k_r[1].clone(), k_l[1].clone()].concat();
    let test_k_perm = apply_inv_multithreads(&perm, &test_k, nets, state0, state1)?;
    let test_k_perm_2 = apply_inv_multithreads(&perm, &test_k_2, nets, state0, state1)?;
    let open_test_k_perm = open_vec(&test_k_perm, nets[0])?;
    let open_test_k_perm_2 = open_vec(&test_k_perm_2, nets[0])?;
    println!("open_test_k_perm: {:?}", open_test_k_perm);
    println!("open_test_k_perm_2: {:?}", open_test_k_perm_2);
    */

    //* j is final tabla. d is the number of value columns in left table, e is the number of  value columns in right table
    //* 2: k_r and v_j
    let mut j: Vec<Vec<Rep3RingShare<T>>> = Vec::with_capacity(d+e+k_len+1);

    //* k_r remain the same order without permutation
    for k in k_r{
        j.push(k);
    }

    //* compute jl_k, l_k is column form left table
    for l_k in val_l{
        let jl_k = compute_jlk_multithreads(len_m, len_n, &perm, l_k, nets, state0, state1)?;
        j.push(jl_k);
        
    }

    let c = compute_c_multithreads(len_m, len_n, &perm, nets, state0, state1)?;

    for r_j in val_r{
        let jr_j = compute_jrj_multithreads(&c, r_j, nets, state0)?;
        j.push(jr_j);
    }

    let valid_j = compute_vj_multithreads(len_m, len_n, &perm, &valid_l, &valid_r, nets, state0, state1)?;
    j.push(valid_j);

    Ok(j)
}


/// Semi-join: keep only the rows in left table that have matches in right table
pub fn semi_join_table_multithreads<T: IntRing2k, N: Network>(
    k_l: Vec<Rep3RingShare<T>>,
    k_r: Vec<Rep3RingShare<T>>,
    valid_l: Vec<Rep3RingShare<T>>,
    valid_r: Vec<Rep3RingShare<T>>,
    bitsize: usize,
    nets: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let one = RingElement(T::one());
    let bigone= one<<63;

    // if v_r ==1, then keep k_r; else set to bigone. Filter table r to avoid matching with zero values.
    let new_k_r = mux::mux_if_share_then_public_vec_multithreads(&valid_r, &k_r, &bigone, nets, state0, states)?;

    // get the flags: if k_l is not in k_r, then f_res == 1 else f_res ==0
    let f_inv = only_in_l_multithreads(
        k_l,
        new_k_r,
        bitsize,
        nets,
        state0,
        state1,
        states,
    )?;

    // invert f_res to get the final flags. If k_l is in k_r, then f_res == 1 else f_res ==0
    let f_res = f_inv.iter().map(|f_i| arithmetic::sub_public_by_shared(one, *f_i, state1.id)).collect::<Vec<_>>();
    let new_valid_tmp = arithmetic::local_mul_vec(&f_res, &valid_l, state0);

    //*  multiply f_res with v_l to get the final result
    let new_valid = reshare_vec_q_multithreads(&new_valid_tmp, nets)?;

    Ok(new_valid)
}


/// anti-join: return the rows in left table that do not match any rows in right table
pub fn anti_join_table_multithreads<T: IntRing2k, N: Network>(
    k_l: Vec<Rep3RingShare<T>>,
    k_r: Vec<Rep3RingShare<T>>,
    valid_l: Vec<Rep3RingShare<T>>,
    valid_r: Vec<Rep3RingShare<T>>,
    bitsize: usize,
    nets: &[&N],
    state0: &mut Rep3State,
    state1: &mut Rep3State,
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let one = RingElement(T::one());
    let bigone= one<<63;

    // if v_r ==1, then keep k_r; else set to bigone. Filter table r to avoid matching with zero values.
    let new_k_r = mux::mux_if_share_then_public_vec_multithreads(&valid_r, &k_r, &bigone, nets, state0, states)?;

    // get the flags: if k_l is not in k_r, then f_res == 1 else f_res ==0
    let f_inv = only_in_l_multithreads(
        k_l,
        new_k_r,
        bitsize,
        nets,
        state0,
        state1,
        states,
    )?;

    //*  update valid in table l by multiplying with f_inv
    let new_valid_tmp = arithmetic::local_mul_vec(&f_inv, &valid_l, state0);
    let new_valid = reshare_vec_q_multithreads(&new_valid_tmp, nets)?;

    Ok(new_valid)
}




//************ one thread version *************/

fn compute_jlk<T: IntRing2k, N: Network>(
    len_m: usize,
    len_n: usize,
    perm: &[Rep3RingShare<PermRing>],
    l:  &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let jl_k = from_l_to_r::from_l_to_r_other(
        len_m,
        len_n,
        &perm,
        &l,
        net,
        state,
    )?;
    
    Ok(jl_k)
}

fn compute_c<T: IntRing2k, N: Network>(
    len_m: usize,
    len_n: usize,
    perm: &[Rep3RingShare<PermRing>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    // F_one
    let vec_one: Vec<Rep3RingShare<T>> = (0..len_m).map(|_| arithmetic::promote_to_trivial_share(state.id, RingElement(T::one()))).collect();

    let c= from_l_to_r::from_l_to_r_other(
        len_m,
        len_n,
        &perm,
        &vec_one,
        net,
        state,
    )?;
    
    Ok(c)
}

fn compute_jrj<T: IntRing2k, N: Network>(
    c: &[Rep3RingShare<T>],
    r:  &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let jrj_tmp = arithmetic::local_mul_vec(c, r, state);
    
    let jrj = arithmetic::reshare_vec(jrj_tmp, net)?;
    
    Ok(jrj)
}

fn compute_vj<T: IntRing2k, N: Network>(
    len_m: usize,
    len_n: usize,
    perm: &[Rep3RingShare<PermRing>],
    vl: &[Rep3RingShare<T>],
    vr:  &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let v= from_l_to_r::from_l_to_r_other(
        len_m,
        len_n,
        &perm,
        &vl,
        net,
        state,
    )?;

    let vj_tmp = arithmetic::local_mul_vec(&v, vr, state);
    
    let vj = arithmetic::reshare_vec(vj_tmp, net)?;
    
    Ok(vj)
}


/// The bottleneck of semi-join and left-outer-join. Return 的是在l不在r中的标志，1表示满足，0表示不满足。
pub fn only_in_l<T: IntRing2k, N: Network>(
    k_l: Vec<Rep3RingShare<T>>,
    k_r: Vec<Rep3RingShare<T>>,
    bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,{
    
    let m = k_l.len();
    let n = k_r.len();

    let perm = from_l_to_r::from_l_to_r_gen_perm(&k_l, &k_r, bitsize, net, state)?;
    
    //generate n zeros
    let vec_zero: Vec<Rep3RingShare<T>> = (0..n).map(|_| arithmetic::promote_to_trivial_share(state.id, RingElement(T::zero()))).collect();
    
    //generate m ones and m neg ones
    let vec_one_1: Vec<Rep3RingShare<T>> = (0..m).map(|_| arithmetic::promote_to_trivial_share(state.id, RingElement(T::one()))).collect();
    let vec_one_2: Vec<Rep3RingShare<T>> = (0..m).map(|_| arithmetic::promote_to_trivial_share(state.id, -RingElement(T::one()))) .collect();
    
    //make f a vector of m ones, n zeros, m -ones
    let f = [vec_one_1, vec_zero, vec_one_2].concat();

    //set g to be a vector by applying perm to f
    let g = apply_inv(&perm, &f, net, state)?;

    //compute prefix sum of g
    let h = prefix_sum_sequential(&g)?;

    //compute p 论文的算法有问题，应该是p[i] = 1-h[i+1].这里就直接对位置减1，然后序列前移，再补尾元素
    let one = RingElement(T::one());
    let mut p = h.iter().map(|h_i| arithmetic::sub_public_by_shared(one, *h_i, state.id)).collect::<Vec<_>>();
    p.remove(0); //移除p的第一个元素
    p.push(h[2*m+n-1].clone()); //添加最后一个元素

    //compute f_res
    let f_res = apply_perm(&perm, &p, net, state)?;
    
    Ok(f_res[..m].to_vec())
}


pub fn inner_join<T: IntRing2k, N: Network>(
    k_l_idx: usize,
    k_r_idx: usize,
    l: Vec<Vec<Rep3RingShare<T>>>,
    r: Vec<Vec<Rep3RingShare<T>>>,
    v_l: &[Rep3RingShare<T>],
    v_r: &[Rep3RingShare<T>],
    bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Vec<Rep3RingShare<T>>>>
where
    Standard: Distribution<T>,
{
    let k_l = l[k_l_idx].clone();
    let k_r = r[k_r_idx].clone();
    let d =l.len();
    let e =r.len();
    let (len_m, len_n) = (k_l.len(), k_r.len());
    let mut j: Vec<Vec<Rep3RingShare<T>>> = Vec::with_capacity(d+e+2);
    j.push(k_r.to_vec());

    let perm = from_l_to_r::from_l_to_r_gen_perm(&k_l, &k_r, bitsize, net, state)?;

    for l_k in l.iter(){
        let jl_k = compute_jlk(len_m, len_n, &perm, &l_k, net, state)?;
        j.push(jl_k);
    }

    let c = compute_c(len_m, len_n, &perm, net, state)?;

    for r_j in r.iter(){
        let jr_j = compute_jrj(&c, &r_j, net, state)?;
        j.push(jr_j);
    }

    let vj = compute_vj(len_m, len_n, &perm, &v_l, &v_r, net, state)?;
    j.push(vj);

    Ok(j)
}


pub fn semi_join_table<T: IntRing2k, N: Network>(
    k_l: Vec<Rep3RingShare<T>>,
    k_r: Vec<Rep3RingShare<T>>,
    v_l: Vec<Rep3RingShare<T>>,
    v_r: Vec<Rep3RingShare<T>>,
    bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let one = RingElement(T::one());
    let bigone= one<<63;

    // if v_r ==1, then keep k_r; else set to bigone. Filter table r to avoid matching with zero values.
    let new_k_r = mux::mux_if_share_then_public_vec(&v_r, &k_r, &bigone, net, state)?;

    // get the flags: if k_l is not in k_r, then f_res == 1 else f_res ==0
    let f_inv = only_in_l(
        k_l,
        new_k_r,
        bitsize,
        net,
        state,
    )?;

    // invert f_res to get the final flags. If k_l is in k_r, then f_res == 1 else f_res ==0
    let f_res = f_inv.iter().map(|f_i| arithmetic::sub_public_by_shared(one, *f_i, state.id)).collect::<Vec<_>>();

    // multiply f_res with v_l to get the final result
    let new_valid_tmp = arithmetic::local_mul_vec(&f_res, &v_l, state);
    let new_valid = arithmetic::reshare_vec(new_valid_tmp, net)?;

    Ok(new_valid)
}


///inner_join primitive for ShareTable
pub fn inner_join_table<T: IntRing2k, N: Network>(
    k_l: Vec<Rep3RingShare<T>>,
    k_r: Vec<Rep3RingShare<T>>,
    val_l: Vec<&[Rep3RingShare<T>]>,
    val_r: Vec<&[Rep3RingShare<T>]>,
    v_l: Vec<Rep3RingShare<T>>,
    v_r: Vec<Rep3RingShare<T>>,
    bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Vec<Rep3RingShare<T>>>>
where
    Standard: Distribution<T>,
{
    let d = val_l.len();
    let e = val_r.len();
    let (len_m, len_n) = (k_l.len(), k_r.len());

    let perm = from_l_to_r::from_l_to_r_gen_perm(&k_l, &k_r, bitsize, net, state)?;
    

    let mut j: Vec<Vec<Rep3RingShare<T>>> = Vec::with_capacity(d+e+2);

    j.push(k_r);

    eprintln!("compute jlk...");
    for l_k in val_l{
        //eprintln!("lk.len(): {}", l_k.len());
        let jl_k = compute_jlk(len_m, len_n, &perm, l_k, net, state)?;
        j.push(jl_k);
        
    }

    eprintln!("compute c...");
    let c = compute_c(len_m, len_n, &perm, net, state)?;

    for r_j in val_r{
        let jr_j = compute_jrj(&c, r_j, net, state)?;
        j.push(jr_j);
    }

    let vj = compute_vj(len_m, len_n, &perm, &v_l, &v_r, net, state)?;
    j.push(vj);

    Ok(j)
}








