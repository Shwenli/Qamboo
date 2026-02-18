
use rand::distributions::Standard;
use rand::prelude::Distribution;
use num_traits::{Zero, One};
use rayon::iter::IntoParallelIterator;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use communication::rep3::id::PartyID;
use communication::rep3::net_impl::Rep3NetworkImpl;
use communication::task::get_task_chunks;
use random::rep3::Rep3State;
use algebra::ring::{int_ring::IntRing2k, ring_impl::RingElement};
use protocols::protocols::rep3_ring::arithmetic;
use protocols::protocols::{rep3_ring::Rep3RingShare};
use net::Network;
use crate::shuffle::*;
use crate::mul::*;
use crate::transform::{inject_bit_multithreads, inject_bit};


// u32 allows to sort 4*10^9 elements. Inputs of this size require 32*4*10^9*2 bytes, i.e., 256 GB of RAM
type PermRing = u32;


#[expect(clippy::too_many_arguments)]
pub fn gen_perm_multithreads<T: IntRing2k, N: Network>(
    bits: &[Rep3RingShare<T>],
    order: bool,
    bitsize: usize,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>> {

    match order {
        true => {
            //ASC
            let bit_0 = inject_bit_multithreads(&bits, 0, nets, states)?;
           
            let mut perm = gen_bit_perm_multithreads(bit_0, nets, states)?; // This first permutation could be optimized by not promoting the public bits

            for i in 1..bitsize {
                let bit_i = inject_bit_multithreads(&bits, i, nets, states)?;
                
                let bit_i_aperm = apply_inv_multithreads(&perm, &bit_i, nets, states)?;
                
                let perm_i = gen_bit_perm_multithreads(bit_i_aperm, nets, states)?;
                
                perm = compose_perm_multithreads(perm, perm_i, nets, states)?;
            }

            Ok(perm)
        }
        false => {
            // DESC
            let mut bit_0 = inject_bit_multithreads(&bits, 0, nets, states)?;
           
            let party_id = states[0].id;

            for p in bit_0.iter_mut() {
                *p = arithmetic::add_public(-(*p), RingElement::one(), party_id);
            }

            let mut perm = gen_bit_perm_multithreads(bit_0, nets, states)?; // This first permutation could be optimized by not promoting the public bits

            for i in 1..bitsize {
                let bit_i = inject_bit_multithreads(&bits, i, nets, states)?;
                
                let mut bit_i_aperm = apply_inv_multithreads(&perm, &bit_i, nets, states)?;

                for p in bit_i_aperm.iter_mut() {
                    *p = arithmetic::add_public(-(*p), RingElement::one(), party_id);
                }
                
                let perm_i = gen_bit_perm_multithreads(bit_i_aperm, nets, states)?;

                perm = compose_perm_multithreads(perm, perm_i, nets, states)?;
            }

            Ok(perm)
        }
    }
    
}

#[inline]
pub fn gen_bit_perm_multithreads<N: Network>(
    bits: Vec<Rep3RingShare<PermRing>>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>> {

    let len = bits.len();

    let net_num = nets.len();
    let id = states[0].id;
    

    let mut f0 = Vec::with_capacity(len);
    let mut f1 = Vec::with_capacity(len);

    for inp in bits {
        f0.push(arithmetic::add_public(-inp, RingElement::one(), id));
        f1.push(inp);
    }

    let mut s = Rep3RingShare::zero_share();
    let mut s0 = Vec::with_capacity(len);
    let mut s1 = Vec::with_capacity(len);
    for f in f0.iter() {
        s = arithmetic::add(s, *f);
        s0.push(s);
    }
    for f in f1.iter() {
        s = arithmetic::add(s, *f);
        s1.push(s);
    }

    let (state0, state1) = states.split_at_mut(net_num / 2);
    let(mul1,mul2) = rayon::join(
        move || {
            local_mul_vec_multithreads(&f0, &s0, state0)
        },
        move || {
            local_mul_vec_multithreads(&f1, &s1, state1)
        },
    );

    let perm_a:Vec<RingElement<u32>> = mul1.into_iter().zip(mul2).map(|(a, b)| a + b).collect();

    let perm = reshare_vec_multinet(perm_a, nets)?;

    Ok(perm)
}


pub fn apply_perm_multithreads<T: IntRing2k, N: Network>(
    rho: &[Rep3RingShare<PermRing>],
    bits: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let len = rho.len();
    debug_assert_eq!(len, bits.len());

    let unshuffled = (0..len as PermRing).collect::<Vec<_>>();
    let (perm_a, perm_b) = states[0].rngs.rand.random_perm(unshuffled);

    let perm: Vec<_> = perm_a
        .into_iter()
        .zip(perm_b)
        .map(|(a, b)| Rep3RingShare::new(a, b))
        .collect();

    let opened = shuffle_reveal_multithreads::<PermRing, _>(&perm, rho, nets, states);
    
    // apply_perm 标准实现：
    // 步骤2-3: shuffle_reveal 得到 ρ = π ∘ π_rand⁻¹ (opened)
    // 步骤4: shuffle 得到 [[π_rand(v)]] (bits_shuffled)
    // 步骤5: 本地应用公开排列 ρ，得到 [[ρ(π_rand(v))]] = [[π(v)]]
    let opened = opened?;
    
    // 步骤5：本地应用公开排列 ρ
    // result[i] = bits_shuffled[ρ[i] - 1]
    let mut result_pre = vec![Rep3RingShare::zero_share(); len];
    for i in 0..len {
        result_pre[i] = bits[opened[i].0 as usize - 1];
    }
    let result = unshuffle_multithreads(&perm, &result_pre, nets,states)?;
    
    Ok(result)
}


pub fn apply_inv_multithreads<T: IntRing2k, N: Network>(
    rho: &[Rep3RingShare<PermRing>],
    bits: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let len = rho.len();
    debug_assert_eq!(len, bits.len());

    let unshuffled = (0..len as PermRing).collect::<Vec<_>>();

    let (perm_a, perm_b) = states[0].rngs.rand.random_perm(unshuffled);
    let perm: Vec<_> = perm_a
        .into_iter()
        .zip(perm_b)
        .map(|(a, b)| Rep3RingShare::new(a, b))
        .collect();

    let net_len = nets.len();
    let net0 = &nets[..net_len / 2];
    let net1 = &nets[net_len / 2..];
    let (state0, state1) = states.split_at_mut(net_len / 2);
     
    let (opened, bits_shuffled) = net::join(
        || shuffle_reveal_multithreads::<PermRing, _>(&perm, rho, net0, state0),
        || shuffle_multithreads(&perm, bits, net1, state1),
    );
    
    let mut result = vec![Rep3RingShare::zero_share(); len];
    for (p, b) in opened?.into_iter().zip(bits_shuffled?) {
        result[p.0 as usize - 1] = b;
    }

    Ok(result)
}


/// in place version to save memory
pub fn apply_inv_in_place_multithreads<T: IntRing2k, N: Network>(
    rho: &[Rep3RingShare<PermRing>],
    bits: &mut [Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<()>
where
    Standard: Distribution<T>,
{
    let len = rho.len();
    debug_assert_eq!(len, bits.len());

    let unshuffled = (0..len as PermRing).collect::<Vec<_>>();
    let (perm_a, perm_b) = states[0].rngs.rand.random_perm(unshuffled);

    let perm: Vec<_> = perm_a
        .into_iter()
        .zip(perm_b)
        .map(|(a, b)| Rep3RingShare::new(a, b))
        .collect();

    let (state0, state1) = states.split_at_mut(nets.len() / 2);

    let (opened, bits_shuffled) = net::join(
        || shuffle_reveal_multithreads::<PermRing, _>(&perm, rho, nets[..nets.len()/2].as_ref(), state0),
        || shuffle_multithreads(&perm, bits, nets[nets.len()/2..].as_ref(), state1),
    );

    let opened = opened?;
    let bits_shuffled = bits_shuffled?;

    let mut temp_result = vec![Rep3RingShare::zero_share(); len];
    for (p, b) in opened.into_iter().zip(bits_shuffled) {
        temp_result[p.0 as usize - 1] = b;
    }

    bits.copy_from_slice(&temp_result);

    Ok(())
}


pub fn compose_perm_multithreads<N: Network>(
    sigma: Vec<Rep3RingShare<PermRing>>,
    phi: Vec<Rep3RingShare<PermRing>>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>> {
    let len = sigma.len();
    debug_assert_eq!(len, phi.len());

    let unshuffled = (0..len as PermRing).collect::<Vec<_>>();

    let (perm_a, perm_b) = states[0].rngs.rand.random_perm(unshuffled);

    let perm: Vec<_> = perm_a
        .into_iter()
        .zip(perm_b)
        .map(|(a, b)| Rep3RingShare::new(a, b))
        .collect();

    let opened = shuffle_reveal_multithreads(&perm, &sigma, nets, states)?;

    let mut shuffled = Vec::with_capacity(len);
    for p in opened {
        shuffled.push(phi[p.0 as usize - 1]);
    }

    unshuffle_multithreads(&perm, &shuffled, nets, states)
}






//************ one thread version *************/

#[expect(clippy::too_many_arguments)]
pub fn gen_perm<T: IntRing2k, N: Network>(
    bits: &[Rep3RingShare<T>],
    order: bool,
    bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>> {
    match order {
        true => {
            // ASC
            let bit_0 = inject_bit(bits, 0, net, state)?;
           
            let mut perm = gen_bit_perm(bit_0, net, state)?;

            for i in 1..bitsize {
                let bit_i = inject_bit(&bits, i, net, state)?;
                
                let bit_i_aperm = apply_inv(&perm, &bit_i, net, state)?;
                
                let perm_i = gen_bit_perm(bit_i_aperm, net, state)?;
                perm = compose(perm, perm_i, net, state)?;
            }

            Ok(perm)
        }
        false => {
            // DESC
            let mut bit_0 = inject_bit(&bits, 0, net, state)?;
           
            let party_id = state.id;
            for p in bit_0.iter_mut() {
                *p = arithmetic::add_public(-(*p), RingElement::one(), party_id);
            }
            let mut perm = gen_bit_perm(bit_0, net, state)?;

            for i in 1..bitsize {
                let bit_i = inject_bit(&bits, i, net, state)?;
                
                let mut bit_i_aperm = apply_inv(&perm, &bit_i, net, state)?;
                
                let party_id = state.id;
                for p in bit_i_aperm.iter_mut() {
                    *p = arithmetic::add_public(-(*p), RingElement::one(), party_id);
                }
                
                let perm_i = gen_bit_perm(bit_i_aperm, net, state)?;
                perm = compose(perm, perm_i, net, state)?;
            }

            Ok(perm)
        }
    }
    
}



pub fn gen_bit_perm<N: Network>(
    bits: Vec<Rep3RingShare<PermRing>>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>> {

    let len = bits.len();
    let id = state.id;

    let mut f0 = Vec::with_capacity(len);
    let mut f1 = Vec::with_capacity(len);
    for inp in bits {
        f0.push(arithmetic::add_public(-inp, RingElement::one(), id));
        f1.push(inp);
    }

    let mut s = Rep3RingShare::zero_share();
    let mut s0 = Vec::with_capacity(len);
    let mut s1 = Vec::with_capacity(len);

    for f in f0.iter() {
        s = arithmetic::add(s, *f);
        s0.push(s);
    }
    for f in f1.iter() {
        s = arithmetic::add(s, *f);
        s1.push(s);
    }

    let mul1 = arithmetic::local_mul_vec(&f0, &s0, state);
    let mul2 = arithmetic::local_mul_vec(&f1, &s1, state);
    let perm_a:Vec<RingElement<u32>> = mul1.into_iter().zip(mul2).map(|(a, b)| a + b).collect();

    let perm = arithmetic::reshare_vec(perm_a, net)?;

    Ok(perm)
}


pub fn apply_perm<T: IntRing2k, N: Network>(
    rho: &[Rep3RingShare<PermRing>],
    bits: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let len = rho.len();
    debug_assert_eq!(len, bits.len());

    let unshuffled = (0..len as PermRing).collect::<Vec<_>>();
    let (perm_a, perm_b) = state.rngs.rand.random_perm(unshuffled);
    let perm: Vec<_> = perm_a
        .into_iter()
        .zip(perm_b)
        .map(|(a, b)| Rep3RingShare::new(a, b))
        .collect();

    let opened = shuffle_reveal::<PermRing, _>(&perm, rho, net, state);
    
    // apply_perm 标准实现：
    // 步骤2-3: shuffle_reveal 得到 ρ = π ∘ π_rand⁻¹ (opened)
    // 步骤4: shuffle 得到 [[π_rand(v)]] (bits_shuffled)
    // 步骤5: 本地应用公开排列 ρ，得到 [[ρ(π_rand(v))]] = [[π(v)]]
    let opened = opened?;
    
    // 步骤5：本地应用公开排列 ρ
    // result[i] = bits_shuffled[ρ[i] - 1]
    let mut result_pre = vec![Rep3RingShare::zero_share(); len];
    for i in 0..len {
        result_pre[i] = bits[opened[i].0 as usize - 1];
    }
    let result = unshuffle(&perm,&result_pre, net, state)?;
    
    Ok(result)
}


pub fn apply_inv<T: IntRing2k, N: Network>(
    rho: &[Rep3RingShare<PermRing>],
    bits: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let len = rho.len();
    debug_assert_eq!(len, bits.len());

    let unshuffled = (0..len as PermRing).collect::<Vec<_>>();
    let (perm_a, perm_b) = state.rngs.rand.random_perm(unshuffled);
    let perm: Vec<_> = perm_a
        .into_iter()
        .zip(perm_b)
        .map(|(a, b)| Rep3RingShare::new(a, b))
        .collect();

    let open_perm = shuffle_reveal::<PermRing, _>(&perm, rho, net, state)?;
    let shuffled_bits = shuffle(&perm, bits, net, state)?;
    /* 
    let (opened, bits_shuffled) = net::join(
        || shuffle_reveal::<PermRing, _>(&perm, rho, net0, state0),
        || shuffle(&perm, priv_bits, net1, state1),
    );
    */
    let mut result = vec![Rep3RingShare::zero_share(); len];
    for (p, b) in open_perm.into_iter().zip(shuffled_bits) {
        result[p.0 as usize - 1] = b;
    }

    Ok(result)
}


/// in place version to save memory
pub fn apply_inv_in_place<T: IntRing2k, N: Network>(
    rho: &[Rep3RingShare<PermRing>],
    bits: &mut [Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<()>
where
    Standard: Distribution<T>,
{
    let len = rho.len();
    debug_assert_eq!(len, bits.len());

    let unshuffled = (0..len as PermRing).collect::<Vec<_>>();
    let (perm_a, perm_b) = state.rngs.rand.random_perm(unshuffled);
    let perm: Vec<_> = perm_a
        .into_iter()
        .zip(perm_b)
        .map(|(a, b)| Rep3RingShare::new(a, b))
        .collect();

    let open_perm = shuffle_reveal::<PermRing, _>(&perm, rho, net, state)?;
    let shuffled_bits = shuffle(&perm, bits, net, state)?;

    let mut temp_result = vec![Rep3RingShare::zero_share(); len];
    for (p, b) in open_perm.into_iter().zip(shuffled_bits) {
        temp_result[p.0 as usize - 1] = b;
    }

    bits.copy_from_slice(&temp_result);

    Ok(())
}


pub fn compose<N: Network>(
    sigma: Vec<Rep3RingShare<PermRing>>,
    phi: Vec<Rep3RingShare<PermRing>>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>> {

    let len = sigma.len();
    debug_assert_eq!(len, phi.len());

    let unshuffled = (0..len as PermRing).collect::<Vec<_>>();
    let (perm_a, perm_b) = state.rngs.rand.random_perm(unshuffled);

    let perm: Vec<_> = perm_a
        .into_iter()
        .zip(perm_b)
        .map(|(a, b)| Rep3RingShare::new(a, b))
        .collect();
    let opened = shuffle_reveal(&perm, &sigma, net, state)?;

    let mut shuffled = Vec::with_capacity(len);
    for p in opened {
        shuffled.push(phi[p.0 as usize - 1]);
    }

    unshuffle(&perm, &shuffled, net, state)
}


fn shuffle<T: IntRing2k, N: Network>(
    pi: &[Rep3RingShare<PermRing>],
    input: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let len = pi.len();
    debug_assert_eq!(len, input.len());

    let result = match state.id {
        PartyID::ID0 => {
            // has p1, p3
            let mut alpha_1 = Vec::with_capacity(len);
            let mut alpha_3 = Vec::with_capacity(len);
            let mut beta_1 = Vec::with_capacity(len);

            for a in input {
                let (alpha_1_, alpha_3_) = state.rngs.rand.random_elements::<RingElement<T>>();
                alpha_1.push(alpha_1_);
                alpha_3.push(alpha_3_);
                beta_1.push(a.a + a.b);
            }

            // first shuffle
            let mut shuffled_1 = Vec::with_capacity(len);
            for (pi, alpha) in pi.iter().zip(alpha_1.iter()) {
                let pi_1 = pi.a.0 as usize;
                shuffled_1.push(beta_1[pi_1] - alpha);
            }
            // second shuffle
            let mut shuffled_3 = alpha_1;
            for (des, (pi, alpha)) in shuffled_3.iter_mut().zip(pi.iter().zip(alpha_3)) {
                let pi_3 = pi.b.0 as usize;
                *des = shuffled_1[pi_3] - alpha;
            }
            net.send_next_many(&shuffled_3)?;

            // Opt Reshare
            let mut result = Vec::with_capacity(len);
            for _ in 0..len {
                let (a, b) = state.rngs.rand.random_elements::<RingElement<T>>();
                result.push(Rep3RingShare::new_ring(a, b));
            }
            result
        }
        PartyID::ID1 => {
            // has p2, p1
            let mut alpha_1 = Vec::with_capacity(len);
            let mut beta_2 = Vec::with_capacity(len);
            for a in input {
                let alpha_1_ = state.rngs.rand.random_element_rng2::<RingElement<T>>();
                alpha_1.push(alpha_1_);
                beta_2.push(a.a);
            }

            // first shuffle
            let mut shuffled_1 = Vec::with_capacity(len);
            for (pi, alpha) in pi.iter().zip(alpha_1) {
                let pi_1 = pi.b.0 as usize;
                shuffled_1.push(beta_2[pi_1] + alpha);
            }
            let delta = net.reshare_many(&shuffled_1)?;
            // second shuffle
            let mut beta_2_prime = beta_2;
            for (des, pi) in beta_2_prime.iter_mut().zip(pi) {
                let pi_2 = pi.a.0 as usize;
                *des = delta[pi_2];
            }

            // Opt Reshare
            let mut result = Vec::with_capacity(len);
            let mut rand = Vec::with_capacity(len);
            for beta in beta_2_prime {
                let b = state.rngs.rand.random_element_rng2::<RingElement<T>>();
                rand.push(beta - b);
                result.push(Rep3RingShare::new_ring(RingElement::zero(), b));
            }
            let rcv: Vec<RingElement<T>> =
                net.send_and_recv_many(PartyID::ID2, &rand, PartyID::ID2)?;
            for (res, (r1, r2)) in result.iter_mut().zip(rcv.into_iter().zip(rand)) {
                res.a = r1 + r2;
            }
            result
        }
        PartyID::ID2 => {
            // has p3, p2
            let mut alpha_3 = Vec::with_capacity(len);
            for _ in 0..len {
                let alpha_3_ = state.rngs.rand.random_element_rng1::<RingElement<T>>();
                alpha_3.push(alpha_3_);
            }
            let gamma: Vec<RingElement<T>> = net.recv_prev_many()?;

            // first shuffle
            let mut shuffled_1 = Vec::with_capacity(len);
            for (pi, alpha) in pi.iter().zip(alpha_3.iter()) {
                let pi_3 = pi.a.0 as usize;
                shuffled_1.push(gamma[pi_3] + alpha);
            }

            // second shuffle
            let mut beta_3_prime = alpha_3;
            for (des, pi) in beta_3_prime.iter_mut().zip(pi) {
                let pi_2 = pi.b.0 as usize;
                *des = shuffled_1[pi_2];
            }

            // Opt Reshare
            let mut result = Vec::with_capacity(len);
            let mut rand = Vec::with_capacity(len);
            for beta in beta_3_prime {
                let a = state.rngs.rand.random_element_rng1::<RingElement<T>>();
                rand.push(beta - a);
                result.push(Rep3RingShare::new_ring(a, RingElement::zero()));
            }
            let rcv: Vec<RingElement<T>> = net.send_and_recv_many(PartyID::ID1, &rand, PartyID::ID1)?;

            for (res, (r1, r2)) in result.iter_mut().zip(rcv.into_iter().zip(rand)) {
                res.b = r1 + r2;
            }

            result
        }
    };
    Ok(result)
}


fn shuffle_reveal<T: IntRing2k, N: Network>(
    pi: &[Rep3RingShare<PermRing>],
    input: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<RingElement<T>>>
where
    Standard: Distribution<T>,
{
    let len = pi.len();
    debug_assert_eq!(len, input.len());
    let result = match state.id {
        PartyID::ID0 => {
            // has p1, p3
            let mut alpha_1 = Vec::with_capacity(len);
            let mut beta_1 = Vec::with_capacity(len);
            let rngs = state.rngs.rand.random_elements_rng1::<RingElement<T>>(len);
            for (a, r) in input.iter().zip(rngs) {
                alpha_1.push(r);
                beta_1.push(a.a + a.b);
            }
            
            let mut shuffled = Vec::with_capacity(len);
            for (pi, alpha) in pi.iter().zip(alpha_1.iter()) {
                let pi_1 = pi.a.0 as usize;
                shuffled.push(beta_1[pi_1] - alpha);
            }
            net.send_and_recv_many(PartyID::ID2, &shuffled, PartyID::ID2)?
        }
        PartyID::ID1 => {
            // has p2, p1
            let mut alpha_1 = Vec::with_capacity(len);
            let mut beta_2 = Vec::with_capacity(len);
            let rngs = state.rngs.rand.random_elements_rng2::<RingElement<T>>(len);
            for (a, r) in input.iter().zip(rngs) {
                alpha_1.push(r);
                beta_2.push(a.a);
            }

            let mut shuffled = Vec::with_capacity(len);
            for (pi, alpha) in pi.iter().zip(alpha_1) {
                let pi_1 = pi.b.0 as usize;
                shuffled.push(beta_2[pi_1] + alpha);
            }
            net.send_and_recv_many(PartyID::ID2, &shuffled, PartyID::ID2)?
        }
        PartyID::ID2 => {
            let delta = net.recv_many::<RingElement<T>>(PartyID::ID0)?;
            let gamma = net.recv_many::<RingElement<T>>(PartyID::ID1)?;

            // shuffle
            let mut shuffled = Vec::with_capacity(len);
            for p in pi {
                let pi_2 = p.b.0 as usize;
                let index = pi[pi_2].a.0 as usize;
                shuffled.push(gamma[index] + delta[index]);
            }

            let (send0, send1) = net::join(
                || net.send_many(PartyID::ID0, &shuffled),
                || net.send_many(PartyID::ID1, &shuffled),
            );

            send0?;
            send1?;
            shuffled
        }
    };
    Ok(result)
}


fn unshuffle<T: IntRing2k, N: Network>(
    pi: &[Rep3RingShare<PermRing>],
    input: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let len = pi.len();
    debug_assert_eq!(len, input.len());

    let result = match state.id {
        PartyID::ID0 => {
            // has p1, p3
            let mut alpha_3 = Vec::with_capacity(len);
            for _ in 0..len {
                let alpha_3_ = state.rngs.rand.random_element_rng2::<RingElement<T>>();
                alpha_3.push(alpha_3_);
            }
            let gamma: Vec<RingElement<T>> = net.recv_many(PartyID::ID1)?;

            // first shuffle
            let mut shuffled_3 = vec![RingElement::zero(); len];
            for (pi, (alpha, gamma)) in pi.iter().zip(alpha_3.iter().zip(gamma)) {
                let pi_3 = pi.b.0 as usize;
                shuffled_3[pi_3] = gamma + alpha;
            }

            // second shuffle
            let mut beta_1_prime = alpha_3;
            for (src, pi) in shuffled_3.into_iter().zip(pi) {
                let pi_1 = pi.a.0 as usize;
                beta_1_prime[pi_1] = src;
            }

            // Opt Reshare
            let rngs = state.rngs.rand.random_elements_rng2(len);
            let (rand, mut result): (Vec<_>, Vec<_>) = beta_1_prime
            .into_par_iter()
            .zip(rngs.into_par_iter())
            .map(|(beta, b)| {
                (beta - RingElement(b), Rep3RingShare::new_ring(RingElement::zero(), RingElement(b)))
            })
            .unzip();

            let rcv: Vec<RingElement<T>> = net.send_and_recv_many(PartyID::ID1, &rand, PartyID::ID1)?;

            for (res, (r1, r2)) in result.iter_mut().zip(rcv.into_iter().zip(rand)) {
                res.a = r1 + r2;
            }

            result
        }
        PartyID::ID1 => {
            // has p2, p1
            let mut alpha_2 = Vec::with_capacity(len);
            let mut beta_2 = Vec::with_capacity(len);
            for a in input {
                let alpha_2_ = state.rngs.rand.random_element_rng1::<RingElement<T>>();
                alpha_2.push(alpha_2_);
                beta_2.push(a.b);
            }

            // first shuffle
            let mut shuffled_3 = vec![RingElement::zero(); len];
            for (pi, (alpha, beta_2)) in pi.iter().zip(alpha_2.into_iter().zip(beta_2.iter())) {
                let pi_2 = pi.a.0 as usize;
                shuffled_3[pi_2] = alpha + beta_2;
            }
            let delta = net.send_and_recv_many(PartyID::ID0, &shuffled_3, PartyID::ID2)?;

            // second shuffle
            let mut beta_2_prime = beta_2;
            for (src, pi) in delta.into_iter().zip(pi) {
                let pi_1 = pi.b.0 as usize;
                beta_2_prime[pi_1] = src;
            }

            // Opt Reshare
            let rngs = state.rngs.rand.random_elements_rng1(len);
            let (rand, mut result): (Vec<_>, Vec<_>) = beta_2_prime
            .into_par_iter()
            .zip(rngs.into_par_iter())
            .map(|(beta, b)| {
                (beta - RingElement(b), Rep3RingShare::new_ring(RingElement::zero(), RingElement(b)))
            })
            .unzip();
            
            let rcv: Vec<RingElement<T>> =
                net.send_and_recv_many(PartyID::ID0, &rand, PartyID::ID0)?;

            for (res, (r1, r2)) in result.iter_mut().zip(rcv.into_iter().zip(rand)) {
                res.b = r1 + r2;
            }
            result
        }
        PartyID::ID2 => {
            // has p3, p2
            let mut alpha_3 = Vec::with_capacity(len);
            let mut alpha_2 = Vec::with_capacity(len);
            let mut beta_3 = Vec::with_capacity(len);
            for a in input {
                let (alpha_3_, alpha_2_) = state.rngs.rand.random_elements::<RingElement<T>>();
                alpha_3.push(alpha_3_);
                alpha_2.push(alpha_2_);
                beta_3.push(a.a + a.b);
            }

            // first shuffle
            let mut shuffled_3 = vec![RingElement::zero(); len];
            for (pi, (alpha, beta_3)) in pi.iter().zip(alpha_2.iter().zip(beta_3)) {
                let pi_2 = pi.b.0 as usize;
                shuffled_3[pi_2] = beta_3 - alpha;
            }

            // second shuffle
            let mut shuffled_2 = alpha_2;
            for (src, (pi, alpha)) in shuffled_3.into_iter().zip(pi.iter().zip(alpha_3)) {
                let pi_3 = pi.a.0 as usize;
                shuffled_2[pi_3] = src - alpha;
            }
            net.send_many(PartyID::ID1, &shuffled_2)?;

            // Opt Reshare
            let mut result = Vec::with_capacity(len);
            let rngs = state.rngs.rand.random_elements_vec::<RingElement<T>>(len);
            for (a,b) in rngs.0.into_iter().zip(rngs.1) {
                result.push(Rep3RingShare::new_ring(a, b));
            }
            result
        }
    };
    Ok(result)
}