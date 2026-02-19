
use itertools::izip;
use num_traits::One;
use primitives::mul::mul_share_vec;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use rand::distributions::Standard;
use rand::prelude::Distribution;
use random::rep3::Rep3State;
use algebra::ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement};
use protocols::protocols::rep3_ring::{arithmetic,binary,Rep3RingShare};
use net::Network;
use primitives::{transform, mux, permute};
use primitives::compare::{eq_many, eq_many_multithreads, and_vec_bit_multithreads};


type PermRing = u32;


pub fn make_group_key_null_multithreads<T: IntRing2k, N: Network>(
    keys: &[Rep3RingShare<T>],
    valid: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{
    let big_one = RingElement::one()<<63;
    //let valid_a = b2a_many_multithreads(&valid, net, states)?;
    let keys_null: Vec<_> = valid.par_iter()
    .with_min_len(1024)
    .map(|v| arithmetic::mul_public(arithmetic::sub_public_by_shared(RingElement::one(), *v, states[0].id), big_one))
    .collect();
    /* 
    let valid_inv = izip!(valid).map(|v| arithmetic::sub_public_by_shared(RingElement::one(), *v, states[0].id)).collect::<Vec<_>>();
    let keys_null = izip!(valid_inv.iter()).map(|v| arithmetic::mul_public( *v,big_one)).collect::<Vec<_>>();
    */
    let keys_true = mul_share_vec(&valid, &keys, nets, states)?;
    
    let new_keys = izip!(keys_true.iter(), keys_null.iter()).map(|(a,b)| arithmetic::add(*a,*b)).collect::<Vec<_>>();

    Ok(new_keys)
}


pub fn compute_muti_keys_perm_multithreads<T: IntRing2k, N: Network>(
    keys: &Vec<&[Rep3RingShare<T>]>,
    bitsize: usize,
    order: bool,
    nets: &[&N], 
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>>
where
Standard: Distribution<T>,{

    let len =keys.len();

    let keys_binary_0 = transform::bit_decompose_many_multithreads(keys[len-1], bitsize, nets, states)?;
    let mut perm = permute::gen_perm_multithreads(&keys_binary_0, order, bitsize, nets, states)?;
    
    for i in (0..len-1).rev(){
        let new_k = permute::apply_inv_multithreads(&perm, keys[i], nets, states)?;
        let keys_binary = transform::bit_decompose_many_multithreads(&new_k, bitsize, nets, states)?;
        let perm_i = permute::gen_perm_multithreads(&keys_binary, order, bitsize, nets, states)?;//这里做完之后需要apply_inv吗
        
        perm = permute::compose_perm_multithreads(perm, perm_i, nets, states)?;
    }

    Ok(perm)
}

 
pub fn table_group_by_common_multithreads<T: IntRing2k, N: Network>(
    keys: Vec<&[Rep3RingShare<T>]>,
    vals: Vec<&[Rep3RingShare<T>]>,
    valid: &[Rep3RingShare<T>],
    order: bool,
    sort_bitsize: usize,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<(Vec<Vec<Rep3RingShare<T>>>, Vec<Vec<Rep3RingShare<T>>>, Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<Bit>>,
    Vec<Vec<Rep3RingShare<T>>>,Vec<Rep3RingShare<u32>>,Vec<Vec<Rep3RingShare<T>>>, Vec<Vec<Rep3RingShare<T>>>, Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<T>>)> 
where
Standard: Distribution<T>,{
    
    // make keys and vals null where valid is 0

    let mut new_keys_vec: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for k in keys{
        let new_k = make_group_key_null_multithreads(k, valid, nets, states)?;
        new_keys_vec.push(new_k);
    }

    let mut new_vals_vec: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for v in vals{
        let new_v = make_group_key_null_multithreads(v, valid, nets, states)?;
        new_vals_vec.push(new_v);
    }

    let perm = compute_muti_keys_perm_multithreads(&new_keys_vec.iter().map(|v| v.as_slice()).collect::<Vec<_>>(), sort_bitsize, order, nets, states)?;

    let mut k_g: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for k in new_keys_vec{
        let k_g_i = permute::apply_inv_multithreads(&perm, &k, nets, states)?;
        k_g.push(k_g_i);
    }
    
    let mut v_g: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for v in new_vals_vec{
        let v_g_i = permute::apply_inv_multithreads(&perm, &v, nets, states)?;
        v_g.push(v_g_i);
    }

    let valid = permute::apply_inv_multithreads(&perm, valid, nets, states)?;

    eprintln!("After sorting in muti_key_group_by_common:");

    let k_g_0_0 = &k_g[0][0..k_g[0].len()-1].to_vec();
    let k_g_1_0 = &k_g[0][1..].to_vec();
    let f = eq_many_multithreads(k_g_0_0, k_g_1_0, nets, states)?;

    let bit_one = RingElement(Bit::new(true));

    //get composed e
    let mut e = f;

    for i in 1..k_g.len(){
        
        let k_g_0 = &k_g[i][0..k_g[i].len()-1].to_vec();
        let k_g_1 = &k_g[i][1..].to_vec();

        let f = eq_many_multithreads(k_g_0, k_g_1, nets, states)?;

        let nxt_e = and_vec_bit_multithreads(&e, &f, nets, states)?;
        
        e = nxt_e;
    }
    
    // Finally do 1-e, because only when all keys are equal, e is 1
    e = izip!(e).map(|e_i| binary::xor_public(&e_i, &bit_one, states[0].id)).collect::<Vec<_>>();

    let e_m: Rep3RingShare<Bit> = binary::promote_to_trivial_share(states[0].id,&RingElement(Bit::new(true)));
    e.push(e_m);
   
    let e_t_res: Vec<Rep3RingShare<T>> = transform::from_bit_to_arithmetic_t_multithreads(&e, nets, states)?;
    //eprintln!("e_t_res len: {}", open_vec(e_t_res));

    //Compute the validity of the grouped table, if it's not the representative row of a group, then it's 0.
    let new_valid = mul_share_vec(&e_t_res, &valid, nets, states)?;

    let mut k_g_n: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    let k_2 = RingElement::one();
    let k_2_64 = k_2<<63;
    for i in 0..k_g.len(){
        let k_g_n_i = mux::mux_if_share_then_public_vec_multithreads(&e_t_res, &k_g[i], &k_2_64, nets, states)?;
        k_g_n.push(k_g_n_i);
    }

    //这里获取e_t之后要对e_t去取反，因为:算法是按将1排到前面，0排到后面，还要求是稳定的，不能直接对按原e获得的perm逆置。取反后，靠前的1仍然靠前，是稳定的。
    let mut e_t= transform::from_bit_to_arithmetic_t_multithreads::<u32,N>(&e, nets, states)?;
    for p in e_t.iter_mut() {
        *p = arithmetic::add_public(-(*p), RingElement::one(), states[0].id);
    }
    let perm_e = permute::gen_bit_perm_multithreads(e_t, nets, states)?;
    
    let mut k_out: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for i in 0..k_g_n.len(){
        let k_out_i= permute::apply_inv_multithreads(&perm_e, &k_g_n[i], nets, states)?;
        k_out.push(k_out_i);
    }

    let mut v_out: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for i in 0..v_g.len(){
        let v_out_i= permute::apply_inv_multithreads(&perm_e, &v_g[i], nets, states)?;
        v_out.push(v_out_i);
    }

    let valid_out = permute::apply_inv_multithreads(&perm_e, &new_valid, nets, states)?;

    Ok((k_g, v_g, e_t_res, e, k_g_n, perm_e, k_out, v_out, valid.to_vec(), valid_out))
    // [[kG]], [[vG]], [[e]], [[e_bit]], [[kGN]], [[πGNtoOUT]], [[kOUT]], [[vOUT]](目前没有使用), [[old_valid]], [[new_valid]]
    // corrected to the paper <<Secure Statistical Analysis on Multiple Datasets: Join and Group-By>> notation
    //这里的v_g_vec有没有和k_out_vec一样变成null? 这里不能变成null, 因为agg的时候需要用到原始值.    
}





//************ one thread version *************/

pub fn compute_muti_keys_perm<T: IntRing2k, N: Network>(
    keys: &Vec<&[Rep3RingShare<T>]>,
    bitsize: usize,
    order: bool,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>>
where
Standard: Distribution<T>,{

    //let keys_binary = izip!(keys).map(|k| sort::decompose_ring_vec(k, bitsize, net0, net1, state0, state1).unwrap()).collect::<Vec<_>>();
    let len =keys.len();
    let keys_binary_0 = transform::bit_decompose_many(keys[len-1], bitsize, net, state)?;
    let mut perm = permute::gen_perm(&keys_binary_0, order, bitsize, net, state)?;
    
    for i in (0..len-1).rev(){
        let new_k = permute::apply_inv(&perm, keys[i], net, state)?;
        let keys_binary = transform::bit_decompose_many(&new_k, bitsize, net, state)?;
        let perm_i = permute::gen_perm(&keys_binary, order, bitsize, net, state)?;
        
        perm = permute::compose(perm, perm_i,net, state)?;
    }

    Ok(perm)
}

/// If valid is 0, key is a big number else keep the key.
pub fn make_group_key_null<T: IntRing2k, N: Network>(
    keys: &[Rep3RingShare<T>],
    valid: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{
    let big_one = RingElement::one()<<64;
    let valid_inv = izip!(valid).map(|v| arithmetic::sub_public_by_shared(RingElement::one(), *v, state.id)).collect::<Vec<_>>();
    let keys_null = izip!(valid_inv.iter()).map(|v| arithmetic::mul_public( *v,big_one)).collect::<Vec<_>>();
    let keys_true_tmp = arithmetic::local_mul_vec(&valid, &keys, state);
    let keys_true = arithmetic::reshare_vec(keys_true_tmp, net)?;
    let new_keys = izip!(keys_true.iter(), keys_null.iter()).map(|(a,b)| arithmetic::add(*a,*b)).collect::<Vec<_>>();
    Ok(new_keys)
}


///如果是join之后的，keys需要经过make_group_key_null处理,这个还没测 
/// 得为他写个文档好好梳理一下，可以考虑之后把ei*a+(1-ei)*b写成一个函数
pub fn muti_key_group_by_common<T: IntRing2k, N: Network>(
    keys: Vec<&[Rep3RingShare<T>]>,
    vals: &[Rep3RingShare<T>],
    valid: &[Rep3RingShare<T>],
    order: bool,
    bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Vec<Rep3RingShare<T>>>, Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<T>>, Vec<Vec<Rep3RingShare<T>>>,Vec<Rep3RingShare<u32>>,Vec<Vec<Rep3RingShare<T>>>, Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<T>>)> 
where
Standard: Distribution<T>,{
    let perm = compute_muti_keys_perm(&keys, bitsize, order, net, state)?;
    let mut new_keys_vec: Vec<Vec<Rep3RingShare<T>>> = Vec::new();

    for k in keys{
        let new_k = make_group_key_null(k, valid, net, state)?;
        new_keys_vec.push(new_k);
    }

    let mut k_g: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for k in new_keys_vec{
        let k_g_i = permute::apply_inv(&perm, &k, net, state)?;
        k_g.push(k_g_i);
    }

    let v_g = permute::apply_inv(&perm, &vals, net, state)?;

    let valid = permute::apply_inv(&perm, valid, net, state)?;

    eprintln!("After sorting in muti_key_group_by_common:");

    let k_g_0_0 = &k_g[0][0..k_g[0].len()-1].to_vec();
    let k_g_1_0 = &k_g[0][1..].to_vec();
    let f = eq_many(k_g_0_0, k_g_1_0, net, state)?;
    

    let bit_one = RingElement(Bit::new(true));
    
    let mut e = f ;
    //println!("e: {:?}",binary::open_vec(&e, net0));

    //use iter to get references to each k_g[i]
    //get composed e
    for i in 1..k_g.len(){
        
        let k_g_0 = &k_g[i][0..k_g[i].len()-1].to_vec();
        let k_g_1 = &k_g[i][1..].to_vec();

        //这里可以之后增加更多线程和网络
        let f = eq_many(k_g_0, k_g_1, net, state)?;

        e = binary::and_vec_bit(&e, &f, net,state)?;
    }
    //最后做一次1-e,因为只有全部键相等时,e才为1
    e = izip!(e).map(|e_i| binary::xor_public(&e_i, &bit_one, state.id)).collect::<Vec<_>>();

    let e_m = binary::promote_to_trivial_share(state.id,&RingElement(Bit::new(true)));
    e.push(e_m);
   
    let e_t_res: Vec<Rep3RingShare<T>> = transform::from_bit_to_arithmetic_t(&e, net, state)?;

    //计算分组后的表的valid,如果不是一组的标识行，那么就是0。
    let new_valid_tmp = arithmetic::local_mul_vec(&e_t_res, &valid, state);
    let new_valid = arithmetic::reshare_vec(new_valid_tmp, net)?;
    let mut k_g_n_vec: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    //改完了，还没调试
    //这里之后换成mux_if_then_share_vec
    for i in 0..k_g.len(){

        let k_g_n_true_tmp = arithmetic::local_mul_vec(&e_t_res, &k_g[i], state);
        let k_g_n_true = arithmetic::reshare_vec(k_g_n_true_tmp, net).unwrap_or_else(|e| panic!("Reshare failed: {:?}", e));

        let k_2 = RingElement::one();
        let k_2_64 = vec![k_2<<63; e_t_res.len()];
        let e_false= izip!(e_t_res.iter()).map(|e_i| arithmetic::sub_public_by_shared(k_2, *e_i, state.id)).collect::<Vec<_>>();
        let k_g_n_false = izip!(e_false,k_2_64).map(|(e_i,k_i)| arithmetic::mul_public(e_i, k_i)).collect::<Vec<_>>();

        let k_g_n = izip!(k_g_n_true.iter(), k_g_n_false.iter()).map(|(a,b)| arithmetic::add(*a,*b)).collect::<Vec<_>>();
        k_g_n_vec.push(k_g_n);
    }

    //这里获取e_t之后要对e_t去取反，因为:算法是按将1排到前面，0排到后面，还要求是稳定的，不能直接对按原e获得的perm逆置。取反后，靠前的1仍然靠前，是稳定的。
    let mut e_t= transform::from_bit_to_arithmetic_t::<u32,N>(&e, net, state)?;
    for p in e_t.iter_mut() {
        *p = arithmetic::add_public(-(*p), RingElement::one(), state.id);
    }
    let perm_e = permute::gen_bit_perm(e_t, net, state)?;
    
    let mut k_out_vec: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for i in 0..k_g_n_vec.len(){
        let k_out= permute::apply_inv(&perm_e, &k_g_n_vec[i], net, state)?;
        k_out_vec.push(k_out);
    }

    let valid_out = permute::apply_inv(&perm_e, &new_valid, net, state)?;

    Ok((k_g, v_g, e_t_res, k_g_n_vec, perm_e, k_out_vec, valid.to_vec(), valid_out))
}



pub fn table_group_by_common<T: IntRing2k, N: Network>(
    keys: Vec<&[Rep3RingShare<T>]>,
    vals: Vec<&[Rep3RingShare<T>]>,
    valid: &[Rep3RingShare<T>],
    order: bool,
    sort_bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Vec<Rep3RingShare<T>>>, Vec<Vec<Rep3RingShare<T>>>, Vec<Rep3RingShare<T>>, Vec<Vec<Rep3RingShare<T>>>,Vec<Rep3RingShare<u32>>,Vec<Vec<Rep3RingShare<T>>>, Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<T>>)> 
where
Standard: Distribution<T>,{
    let perm = compute_muti_keys_perm(&keys, sort_bitsize, order, net, state)?;

    let mut new_keys_vec: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for k in keys{
        let new_k = make_group_key_null(k, valid, net, state)?;
        new_keys_vec.push(new_k);
    }

    let mut new_vals_vec: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for v in vals{
        let new_v = make_group_key_null(v, valid, net, state)?;
        new_vals_vec.push(new_v);
    }

    //排keys
    let mut k_g: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for k in new_keys_vec{
        let k_g_i = permute::apply_inv(&perm, &k, net, state)?;
        k_g.push(k_g_i);
    }
    
    //排vals
    let mut v_g_vec: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for v in new_vals_vec{
        let v_g = permute::apply_inv(&perm, &v, net, state)?;
        v_g_vec.push(v_g);
    }

    //排valid
    let valid = permute::apply_inv(&perm, valid, net, state)?;

    eprintln!("After sorting in muti_key_group_by_common:");

    let k_g_0_0 = &k_g[0][0..k_g[0].len()-1].to_vec();
    let k_g_1_0 = &k_g[0][1..].to_vec();
    let f = eq_many(k_g_0_0, k_g_1_0, net, state)?;

    let bit_one = RingElement(Bit::new(true));
    
    let mut e = f ;
    //println!("e: {:?}",binary::open_vec(&e, net0));

    //use iter to get references to each k_g[i]
    //get composed e
    for i in 1..k_g.len(){
        
        let k_g_0 = &k_g[i][0..k_g[i].len()-1].to_vec();
        let k_g_1 = &k_g[i][1..].to_vec();

        //这里可以之后增加更多线程和网络
        let f = eq_many(k_g_0, k_g_1, net, state)?;

        e = binary::and_vec_bit(&e, &f, net,state)?;
    }
    //最后做一次1-e,因为只有全部键相等时,e才为1
    e = izip!(e).map(|e_i| binary::xor_public(&e_i, &bit_one, state.id)).collect::<Vec<_>>();

    let e_m = binary::promote_to_trivial_share(state.id,&RingElement(Bit::new(true)));
    e.push(e_m);
   
    let e_t_res: Vec<Rep3RingShare<T>> = transform::from_bit_to_arithmetic_t(&e, net, state)?;

    //计算分组后的表的valid,如果不是一组的标识行，那么就是0。
    let new_valid_tmp = arithmetic::local_mul_vec(&e_t_res, &valid, state);
    let new_valid = arithmetic::reshare_vec(new_valid_tmp, net)?;

    let mut k_g_n_vec: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    
    //改完了，还没调试
    //这里之后换成mux_if_then_share_vec
    for i in 0..k_g.len(){
        let k_g_n_true_tmp = arithmetic::local_mul_vec(&e_t_res, &k_g[i], state);
        let k_g_n_true = arithmetic::reshare_vec(k_g_n_true_tmp, net).unwrap_or_else(|e| panic!("Reshare failed: {:?}", e));

        let k_2 = RingElement::one();
        let k_2_64 = vec![k_2<<63; e_t_res.len()];
        let e_false= izip!(e_t_res.iter()).map(|e_i| arithmetic::sub_public_by_shared(k_2, *e_i, state.id)).collect::<Vec<_>>();
        let k_g_n_false = izip!(e_false,k_2_64).map(|(e_i,k_i)| arithmetic::mul_public(e_i, k_i)).collect::<Vec<_>>();

        let k_g_n = izip!(k_g_n_true.iter(), k_g_n_false.iter()).map(|(a,b)| arithmetic::add(*a,*b)).collect::<Vec<_>>();
        k_g_n_vec.push(k_g_n);
    }

    //这里获取e_t之后要对e_t去取反，因为:算法是按将1排到前面，0排到后面，还要求是稳定的，不能直接对按原e获得的perm逆置。取反后，靠前的1仍然靠前，是稳定的。
    let mut e_t= transform::from_bit_to_arithmetic_t::<u32,N>(&e, net, state)?;
    for p in e_t.iter_mut() {
        *p = arithmetic::add_public(-(*p), RingElement::one(), state.id);
    }
    let perm_e = permute::gen_bit_perm(e_t, net, state)?;
    
    let mut k_out_vec: Vec<Vec<Rep3RingShare<T>>> = Vec::new();
    for i in 0..k_g_n_vec.len(){
        let k_out= permute::apply_inv(&perm_e, &k_g_n_vec[i], net, state)?;
        k_out_vec.push(k_out);
    }

    Ok((k_g, v_g_vec, e_t_res, k_g_n_vec, perm_e, k_out_vec, valid,new_valid))
}

