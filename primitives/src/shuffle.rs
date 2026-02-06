
use protocols::protocols::rep3_ring::id::PartyID;
use protocols::protocols::rep3_ring::{
    Rep3State, network::Rep3NetworkExt,
};
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::{
    rep3_ring::Rep3RingShare,
};
use ark_ff::Zero;
use net::Network;
use rand::distributions::Standard;
use rand::prelude::Distribution;
use crate::utils::get_task_chunks;

// u32 allows to sort 4*10^9 elements. Inputs of this size require 32*4*10^9*2 bytes, i.e., 256 GB of RAM
type PermRing = u32;


pub fn shuffle_multithreads<T: IntRing2k, N: Network>(
    pi: &[Rep3RingShare<PermRing>],
    input: &[Rep3RingShare<T>],
    net: &[&N],
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

            let chunks = get_task_chunks(&shuffled_3, len, net.len())?;

            let _ = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        n.send_next_many(chunk)
                            .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e))
                    }
                }),
            );
            //net.send_next_many(&shuffled_3)?;

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

            let chunks = get_task_chunks(&shuffled_1, len, net.len())?;
            let delta = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        n.reshare_many(chunk)
                            .unwrap_or_else(|e| panic!("Shuffle failed: {:?}", e))
                    }
                }),
            );
            let delta = delta.concat();
            //let delta = net.reshare_many(&shuffled_1)?;

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

            let chunks = get_task_chunks(&rand, len, net.len())?;
            let rcv = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        n.send_and_recv_many(PartyID::ID2, chunk, PartyID::ID2)
                            .unwrap_or_else(|e| panic!("Shuffle failed: {:?}", e))
                    }
                }),
            );
            let rcv = rcv.concat();
            //let rcv: Vec<RingElement<T>> =
                //net.send_and_recv_many(PartyID::ID2, &rand, PartyID::ID2)?;

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

            let gamma = net::join_all(
                net.iter().map(| &n| {
                    move || {
                        n.recv_prev_many::<RingElement<T>>()
                            .unwrap_or_else(|e| panic!("Shuffle Multithreads : {:?}", e))
                    }
                }),
            );
            let gamma = gamma.concat();
            //let gamma: Vec<RingElement<T>> = net.recv_prev_many()?;

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
            let chunks = get_task_chunks(&rand, len, net.len())?;
            let rcv = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        n.send_and_recv_many(PartyID::ID1, chunk, PartyID::ID1)
                            .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e))
                    }
                }),
            );
            let rcv = rcv.concat();

            //let rcv: Vec<RingElement<T>> =
                //net.send_and_recv_many(PartyID::ID1, &rand, PartyID::ID1)?;

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
    net: &[&N],
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
            for a in input {
                let alpha_1_ = state.rngs.rand.random_element_rng1::<RingElement<T>>();
                alpha_1.push(alpha_1_);
                beta_1.push(a.a + a.b);
            }
            // shuffle
            let mut shuffled = Vec::with_capacity(len);
            for (pi, alpha) in pi.iter().zip(alpha_1.iter()) {
                let pi_1 = pi.a.0 as usize;
                shuffled.push(beta_1[pi_1] - alpha);
            }

            //net[0].send_and_recv_many(PartyID::ID2, &shuffled, PartyID::ID2)?
            let chunks = get_task_chunks(&shuffled, len, net.len())?;
            let results = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        n.send_and_recv_many(PartyID::ID2, chunk, PartyID::ID2)
                            .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e))
        
                    }
                }),
            );
            results.concat()
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
            // shuffle
            let mut shuffled = Vec::with_capacity(len);
            for (pi, alpha) in pi.iter().zip(alpha_1) {
                let pi_1 = pi.b.0 as usize;
                shuffled.push(beta_2[pi_1] + alpha);
            }
            let chunks = get_task_chunks(&shuffled, len, net.len())?;
            let results = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        n.send_and_recv_many(PartyID::ID2, chunk, PartyID::ID2)
                            .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e))
        
                    }
                }),
            );
            results.concat()
        }
        PartyID::ID2 => {

            let (delta,gamma) = net::join_all(
                net.iter().map(|&n| {
                    move || {
                        let delta = n.recv_many::<RingElement<T>>(PartyID::ID0)
                            .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e));
                        let gamma = n.recv_many::<RingElement<T>>(PartyID::ID1)
                            .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e));
                        (delta,gamma)
                    }
                }),
            )
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();
        
            let delta = delta.concat();
            let gamma = gamma.concat();


            // shuffle
            let mut shuffled = Vec::with_capacity(len);
            for p in pi {
                let pi_2 = p.b.0 as usize;
                let index = pi[pi_2].a.0 as usize;
                shuffled.push(gamma[index] + delta[index]);
            }
            /*
            let (send0, send1) = net::join(
                || net[0].send_many(PartyID::ID0, &shuffled),
                || net[0].send_many(PartyID::ID1, &shuffled),
            );
            */
            
            let chunks = get_task_chunks(&shuffled, len, net.len())?;
            let (_, _) = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        let delta = n.send_many::<RingElement<T>>(PartyID::ID0, &chunk)
                            .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e));
                        let gamma = n.send_many::<RingElement<T>>(PartyID::ID1, &chunk)
                            .unwrap_or_else(|e| panic!("Shuffle reveal failed: {:?}", e));
                        (delta,gamma)
                    }
                })
            ).into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();
            
            //send0?;
            //send1?;
            shuffled
        }
    };
    Ok(result)
}


pub fn unshuffle_multithreads<T: IntRing2k, N: Network>(
    pi: &[Rep3RingShare<PermRing>],
    input: &[Rep3RingShare<T>],
    net: &[&N],
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
            let gamma = net::join_all(
                net.iter().map(| &n| {
                    move || {
                        n.recv_many::<RingElement<T>>(PartyID::ID1)
                            .unwrap_or_else(|e| panic!("Unshuffle failed: {:?}", e))
                    }
                }),
            );
            let gamma = gamma.concat();

            //let gamma: Vec<RingElement<T>> = net.recv_many(PartyID::ID1)?;

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
            let mut result = Vec::with_capacity(len);
            let mut rand = Vec::with_capacity(len);
            for beta in beta_1_prime {
                let b = state.rngs.rand.random_element_rng2::<RingElement<T>>();
                rand.push(beta - b);
                result.push(Rep3RingShare::new_ring(RingElement::zero(), b));
            }
            let chunks = get_task_chunks(&rand, len, net.len())?;
            let rcv = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        n.send_and_recv_many(PartyID::ID1, chunk, PartyID::ID1)
                            .unwrap_or_else(|e| panic!("Unshuffle failed: {:?}", e))
        
                    }
                }),
            );
            let rcv = rcv.concat();

            //let rcv: Vec<RingElement<T>> =
            //    net.send_and_recv_many(PartyID::ID1, &rand, PartyID::ID1)?;
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

            let chunks = get_task_chunks(&shuffled_3, len, net.len())?;
            let delta = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        n.send_and_recv_many(PartyID::ID0, &chunk, PartyID::ID2)
                            .unwrap_or_else(|e| panic!("unshuffle failed: {:?}", e))
                    }
                }),
            );
            let delta = delta.concat();

            //let delta = net.send_and_recv_many(PartyID::ID0, &shuffled_3, PartyID::ID2)?;

            // second shuffle
            let mut beta_2_prime = beta_2;
            for (src, pi) in delta.into_iter().zip(pi) {
                let pi_1 = pi.b.0 as usize;
                beta_2_prime[pi_1] = src;
            }

            // Opt Reshare
            let mut result = Vec::with_capacity(len);
            let mut rand = Vec::with_capacity(len);
            for beta in beta_2_prime {
                let a = state.rngs.rand.random_element_rng1::<RingElement<T>>();
                rand.push(beta - a);
                result.push(Rep3RingShare::new_ring(a, RingElement::zero()));
            }

            let chunks = get_task_chunks(&rand, len, net.len())?;
            let rcv = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        n.send_and_recv_many(PartyID::ID0, chunk, PartyID::ID0)
                            .unwrap_or_else(|e| panic!("unshuffle failed: {:?}", e))
                    }
                })
            );
            let rcv = rcv.concat();

            //let rcv: Vec<RingElement<T>> =
                //net.send_and_recv_many(PartyID::ID0, &rand, PartyID::ID0)?;
            

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

            let chunks = get_task_chunks(&shuffled_2, len, net.len())?;
            let _ = net::join_all(
                chunks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
                    move || {
                        n.send_many(PartyID::ID1, chunk)
                            .unwrap_or_else(|e| panic!("unshuffle failed: {:?}", e))
                    }
                })
            );

            //net.send_many(PartyID::ID1, &shuffled_2)?;

            // Opt Reshare
            let mut result = Vec::with_capacity(len);
            for _ in 0..len {
                let (a, b) = state.rngs.rand.random_elements::<RingElement<T>>();
                result.push(Rep3RingShare::new_ring(a, b));
            }
            result
        }
    };
    Ok(result)
}

