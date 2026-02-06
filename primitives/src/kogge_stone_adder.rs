//! Implementation of the low-depth binary addition
//! This module provides functions for performing low-depth binary addition
use protocols::protocols::rep3_ring::{
    Rep3RingShare, binary,
    ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement},
};
use protocols::protocols::rep3_ring::{Rep3State, network::Rep3NetworkExt};
use itertools::izip;
use net::Network;
use num_traits::{One, Zero};
use rand::{distributions::Standard, prelude::Distribution};



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
        // TODO: Make and more communication efficient, ATM we send the full element for each level, even though they reduce in size
        // maybe just input the mask into AND?
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


#[expect(clippy::type_complexity)]
pub fn and_twice_many_iter_batch_prg<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b1: impl Iterator<Item = Rep3RingShare<T>>,
    b2: impl Iterator<Item = Rep3RingShare<T>>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<T>>)>
where
    Standard: Distribution<T>,
{
    let len = a.len();
    let mut local_a1 = Vec::with_capacity(len);
    let mut local_a2 = Vec::with_capacity(len);
    
    // 优化：一次性生成所需的所有随机对。
    // 每个元素需要 2 对随机数 (mask1, mask_b_1) 和 (mask2, mask_b_2)
    // 根据 random_elements 的实现，这通常比循环调用快得多。
    
    let random_elements: (Vec<RingElement<T>>, Vec<RingElement<T>>) = 
        state.rngs.rand.random_elements_vec::<RingElement<T>>(len*2); // 假设有这样的 API，如果没有，需用 state.rngs.rand 批量生成

    //let mut rand_iter = random_elements.into_iter();
    let mut rand_iter = random_elements.0.into_iter().zip(random_elements.1.into_iter());

    for (a, b1, b2) in izip!(a, b1, b2) {
        let (mut mask1, mask_b1) = rand_iter.next().unwrap();
        mask1 ^= mask_b1;

        let (mut mask2, mask_b2) = rand_iter.next().unwrap();
        mask2 ^= mask_b2;
        
        local_a1.push((&b1 & a) ^ mask1);
        local_a2.push((a & &b2) ^ mask2);
    }
    
    net.send_next([local_a1.to_owned(), local_a2.to_owned()])?;
    let [local_b1, local_b2] = net.recv_prev::<[Vec<RingElement<T>>; 2]>()?;

    let mut r1 = Vec::with_capacity(len);
    let mut r2 = Vec::with_capacity(len);

    for (local_a1, local_b1, local_a2, local_b2) in izip!(local_a1, local_b1, local_a2, local_b2) {
        r1.push(Rep3RingShare::new_ring(local_a1, local_b1));
        r2.push(Rep3RingShare::new_ring(local_a2, local_b2));
    }

    Ok((r1, r2))
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
        *g_item ^= *p_item & RingElement::one();//这里好好想想要不要加*
    }

    let (mut res, c) = kogge_stone_inner_with_carry_many(&p, &g, net, state)?;
    // cin=1
    for res_item in &mut res {
        *res_item = binary::xor_public(res_item, &RingElement::one(), state.id);
    }
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
    
    //之前没有加这个
    let g: Vec<_> = izip!(g, &p).map(|(g_i, p_i)| g_i ^ (*p_i & RingElement::one())).collect();

    let (res, c) = kogge_stone_inner_with_carry_many(&p, &g, net, state)?;

    let res: Vec<_> = res
        .into_iter()
        .map(|res_i| binary::xor_public(&res_i, &RingElement::one(), state.id))
        .collect(); // cin=1

    Ok((res, c))
    
}
