
use communication::rep3::id::PartyID;
use communication::rep3::multinet_impl::*;
use random::rep3::Rep3State;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use algebra::ring::{int_ring::IntRing2k, ring_impl::RingElement};
use protocols::protocols::{rep3_ring::Rep3RingShare};
use ark_ff::Zero;
use net::Network;
use rand::distributions::Standard;
use rand::prelude::Distribution;

// u32 allows to sort 4*10^9 elements. Inputs of this size require 32*4*10^9*2 bytes, i.e., 256 GB of RAM
type PermRing = u32;


pub fn shuffle_multithreads<T: IntRing2k, N: Network>(
    pi: &[Rep3RingShare<PermRing>],
    input: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let len = pi.len();
    debug_assert_eq!(len, input.len());
    let id = states[0].id;

    let result = match id {
        PartyID::ID0 => {
            // has p1, p3
            let mut alpha_1 = Vec::with_capacity(len);
            let mut alpha_3 = Vec::with_capacity(len);
            let mut beta_1 = Vec::with_capacity(len);
            for a in input {
                let (alpha_1_, alpha_3_) = states[0].rngs.rand.random_elements::<RingElement<T>>();
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

            let _ =send_next_many_multinet(nets, &shuffled_3);

            // Opt Reshare
            let mut result = Vec::with_capacity(len);
            for _ in 0..len {
                let (a, b) = states[0].rngs.rand.random_elements::<RingElement<T>>();
                result.push(Rep3RingShare::new_ring(a, b));
            }
            result
        }
        PartyID::ID1 => {
            // has p2, p1
            let mut alpha_1 = Vec::with_capacity(len);
            let mut beta_2 = Vec::with_capacity(len);
            for a in input {
                let alpha_1_ = states[0].rngs.rand.random_element_rng2::<RingElement<T>>();
                alpha_1.push(alpha_1_);
                beta_2.push(a.a);
            }

            // first shuffle
            let mut shuffled_1 = Vec::with_capacity(len);
            for (pi, alpha) in pi.iter().zip(alpha_1) {
                let pi_1 = pi.b.0 as usize;
                shuffled_1.push(beta_2[pi_1] + alpha);
            }

            let delta = reshare_many_multinet(nets, &shuffled_1)?;

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
                let b = states[0].rngs.rand.random_element_rng2::<RingElement<T>>();
                rand.push(beta - b);
                result.push(Rep3RingShare::new_ring(RingElement::zero(), b));
            }

            let rcv = send_and_recv_many_multinet(nets, PartyID::ID2, &rand, PartyID::ID2)?;

            for (res, (r1, r2)) in result.iter_mut().zip(rcv.into_iter().zip(rand)) {
                res.a = r1 + r2;
            }
            result
        }
        PartyID::ID2 => {
            // has p3, p2
            let mut alpha_3 = Vec::with_capacity(len);
            for _ in 0..len {
                let alpha_3_ = states[0].rngs.rand.random_element_rng1::<RingElement<T>>();
                alpha_3.push(alpha_3_);
            }

            let gamma = recv_prev_many_multinet::<RingElement<T>,N>(nets)?;

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
                let a = states[0].rngs.rand.random_element_rng1::<RingElement<T>>();
                rand.push(beta - a);
                result.push(Rep3RingShare::new_ring(a, RingElement::zero()));
            }

            let rcv = send_and_recv_many_multinet(nets, PartyID::ID1, &rand, PartyID::ID1)?;
    
            for (res, (r1, r2)) in result.iter_mut().zip(rcv.into_iter().zip(rand)) {
                res.b = r1 + r2;
            }
            result
        }
    };
    Ok(result)
}

 
pub fn shuffle_reveal_multithreads<T: IntRing2k, N: Network>(
    pi: &[Rep3RingShare<PermRing>],
    input: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<RingElement<T>>>
where
    Standard: Distribution<T>,
{
    let len = pi.len();
    let id = states[0].id;
    debug_assert_eq!(len, input.len());


    let mid = nets.len() / 2;
    let net0 = &nets[0..mid];
    let net1 = &nets[mid..];

    let result = match id {
        PartyID::ID0 => {
            // has p1, p3
            let mut alpha_1 = Vec::with_capacity(len);
            let mut beta_1 = Vec::with_capacity(len);
            let rngs = states[0].rngs.rand.random_elements_rng1::<RingElement<T>>(len);
            for (a, r) in input.iter().zip(rngs) {
                alpha_1.push(r);
                beta_1.push(a.a + a.b);
            }

            // shuffle
            let mut shuffled = Vec::with_capacity(len);
            for (pi, alpha) in pi.iter().zip(alpha_1.iter()) {
                let pi_1 = pi.a.0 as usize;
                shuffled.push(beta_1[pi_1] - alpha);
            }

            let results = send_and_recv_many_multinet(net0, PartyID::ID2, &shuffled, PartyID::ID2)?;

            results
        }
        PartyID::ID1 => {
            // has p2, p1
            let mut alpha_1 = Vec::with_capacity(len);
            let mut beta_2 = Vec::with_capacity(len);
            let rngs = states[0].rngs.rand.random_elements_rng2::<RingElement<T>>(len);
            for (a, r) in input.iter().zip(rngs) {
                alpha_1.push(r);
                beta_2.push(a.a);
            }
    
            // shuffle
            let mut shuffled = Vec::with_capacity(len);
            for (pi, alpha) in pi.iter().zip(alpha_1) {
                let pi_1 = pi.b.0 as usize;
                shuffled.push(beta_2[pi_1] + alpha);
            }

            let results = send_and_recv_many_multinet(net1, PartyID::ID2, &shuffled, PartyID::ID2)?;
            
            results
        }
        PartyID::ID2 => {

            let(delta,gamma) = net::join(
                || recv_many_multinet::<RingElement<T>,N>(net0, PartyID::ID0)
                .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e)),
                || recv_many_multinet::<RingElement<T>,N>(net1, PartyID::ID1)
                .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e)),
            );

            //let delta = recv_many_multinet::<RingElement<T>,N>(net0, PartyID::ID0)?;
            //let gamma = recv_many_multinet::<RingElement<T>,N>(net1, PartyID::ID1)?;

            // shuffle
            let mut shuffled = Vec::with_capacity(len);
            for p in pi {
                let pi_2 = p.b.0 as usize;
                let index = pi[pi_2].a.0 as usize;
                shuffled.push(gamma[index] + delta[index]);
            }

            //send_many_multinet(nets, PartyID::ID0, &shuffled)?;
            //send_many_multinet(nets, PartyID::ID1, &shuffled)?;

            net::join(
                || send_many_multinet(net0, PartyID::ID0, &shuffled)
                .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e)),
                || send_many_multinet(net1, PartyID::ID1, &shuffled)
                .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e)),
            );
            
        
            shuffled
        }
    };
    Ok(result)
}


pub fn unshuffle_multithreads<T: IntRing2k, N: Network>(
    pi: &[Rep3RingShare<PermRing>],
    input: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let len = pi.len();
    let id = states[0].id;
    debug_assert_eq!(len, input.len());
    let result = match id {
        PartyID::ID0 => {
            // has p1, p3
            
            let alpha_3 = states[0].rngs.rand.random_elements_rng2::<RingElement<T>>(len);
            
            let gamma = recv_many_multinet::<RingElement<T>,N>(nets, PartyID::ID1)?;

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
            let rngs = states[0].rngs.rand.random_elements_rng2(len);
            let (rand, mut result): (Vec<_>, Vec<_>) = beta_1_prime
            .into_par_iter().with_min_len(1024)
            .zip(rngs.into_par_iter())
            .map(|(beta, b)| {
                (beta - RingElement(b), Rep3RingShare::new_ring(RingElement::zero(), RingElement(b)))
            })
            .unzip();

            let rcv = send_and_recv_many_multinet(nets, PartyID::ID1, &rand, PartyID::ID1)?;

            for (res, (r1, r2)) in result.iter_mut().zip(rcv.into_iter().zip(rand)) {
                res.a = r1 + r2;
            }
            result
        }
        PartyID::ID1 => {
            // has p2, p1
            let mut beta_2 = Vec::with_capacity(len);
            let alpha_2 = states[0].rngs.rand.random_elements_rng1::<RingElement<T>>(len);
            for a in input {
                beta_2.push(a.b);
            }

            // first shuffle
            let mut shuffled_3 = vec![RingElement::zero(); len];
            for (pi, (alpha, beta_2)) in pi.iter().zip(alpha_2.into_iter().zip(beta_2.iter())) {
                let pi_2 = pi.a.0 as usize;
                shuffled_3[pi_2] = alpha + beta_2;
            }

            let delta = send_and_recv_many_multinet(nets, PartyID::ID0, &shuffled_3, PartyID::ID2)?;

            // second shuffle
            let mut beta_2_prime = beta_2;
            for (src, pi) in delta.into_iter().zip(pi) {
                let pi_1 = pi.b.0 as usize;
                beta_2_prime[pi_1] = src;
            }

            // Opt Reshare
            let rngs = states[0].rngs.rand.random_elements_rng1(len);
            let (rand, mut result): (Vec<_>, Vec<_>) = beta_2_prime
            .into_par_iter()
            .zip(rngs.into_par_iter())
            .map(|(beta, b)| {
                (beta - RingElement(b), Rep3RingShare::new_ring(RingElement(b),RingElement::zero() ))
            })
            .unzip();

            let rcv = send_and_recv_many_multinet(nets, PartyID::ID0, &rand, PartyID::ID0)?;

            for (res, (r1, r2)) in result.iter_mut().zip(rcv.into_iter().zip(rand)) {
                res.b = r1 + r2;
            }
            result
        }
        PartyID::ID2 => {
            // has p3, p2
            let mut beta_3 = Vec::with_capacity(len);
            let (alpha_3, alpha_2) = states[0].rngs.rand.random_elements_vec::<RingElement<T>>(len);
            for a in input {
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

            send_many_multinet(nets, PartyID::ID1, &shuffled_2)?;

            // Opt Reshare
            let mut result = Vec::with_capacity(len);
            let rngs = states[0].rngs.rand.random_elements_vec::<RingElement<T>>(len);
            for (a,b) in rngs.0.into_iter().zip(rngs.1) {
                result.push(Rep3RingShare::new_ring(a, b));
            }
            result
        }
    };
    Ok(result)
}

