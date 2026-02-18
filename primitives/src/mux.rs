use itertools::izip;
use num_traits::{One};
use random::rep3::Rep3State;
use algebra::ring::{int_ring::IntRing2k, ring_impl::RingElement};
use protocols::protocols::rep3_ring::{arithmetic};
use protocols::protocols::rep3_ring::Rep3RingShare;
use net::{Network};
use rand::distributions::Standard;
use rand::prelude::Distribution;
use crate::utils::{get_task_chunks, reshare_vec_q_multithreads};


pub fn mux_if_then_public<T: IntRing2k>(
    cond: &Rep3RingShare<T>,
    a: &RingElement<T>,
    b: &RingElement<T>,
    state0: &mut Rep3State,
) -> eyre::Result<Rep3RingShare<T>>
{
    let one = RingElement(T::one());

    let cond_true = arithmetic::mul_public(*cond, *a);
    let cond_false = arithmetic::mul_public(arithmetic::sub_public_by_shared(one, *cond, state0.id), *b);

    let res = cond_true + cond_false;

    Ok(res)
}

/// mux_if_then_share_vec: if cond[i]==1 then a[i] else b[i]
pub fn mux_if_then_share_vec<T: IntRing2k, N: Network>(
    cond: &[Rep3RingShare<T>],
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{

    let cond_true_tmp = arithmetic::local_mul_vec(cond, a, state);
    let cond_true = arithmetic::reshare_vec(cond_true_tmp, net)?;

    let one = RingElement::one();
    let cond_false_1= izip!(cond.iter()).map(|cond_i| arithmetic::sub_public_by_shared(one, *cond_i, state.id)).collect::<Vec<_>>();
    let cond_false_tmp = arithmetic::local_mul_vec(&cond_false_1, b, state);
    let cond_false = arithmetic::reshare_vec(cond_false_tmp, net)?;

    
    let res = izip!(cond_true.iter(), cond_false.iter()).map(|(a,b)| arithmetic::add(*a,*b)).collect::<Vec<_>>();

    Ok(res)
}


pub fn mux_if_then_share_vec_multithreads<T: IntRing2k, N: Network>(
    cond: &[Rep3RingShare<T>],
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    net: &[&N],
    state0: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{

    let cond_true_tmp = arithmetic::local_mul_vec(cond, a, state0);
    let cond_true = reshare_vec_q_multithreads(&cond_true_tmp, net)?;

    let one = RingElement::one();
    let cond_false_1= izip!(cond.iter()).map(|cond_i| arithmetic::sub_public_by_shared(one, *cond_i, state0.id)).collect::<Vec<_>>();
    let cond_false_tmp = arithmetic::local_mul_vec(&cond_false_1, b, state0);
    let cond_false = reshare_vec_q_multithreads(&cond_false_tmp, net)?;

    let res = izip!(cond_true.iter(), cond_false.iter()).map(|(a,b)| arithmetic::add(*a,*b)).collect::<Vec<_>>();

    Ok(res)
}

pub fn mux_if_share_then_public_vec<T: IntRing2k, N: Network>(
    cond: &[Rep3RingShare<T>],
    a: &[Rep3RingShare<T>],
    b: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{ 

    // coumpute if cond[i]==1 then a[i]
    let cond_true_tmp = arithmetic::local_mul_vec(cond, &a, state);
    let cond_true = arithmetic::reshare_vec(cond_true_tmp, net)?;

    // compute if cond[i]==0 then b
    let c_false = izip!(cond.iter()).map(|cond_i| arithmetic::sub_public_by_shared(RingElement::one(), *cond_i, state.id)).collect::<Vec<_>>();
    let cond_false = c_false.iter().map(|cond_i| arithmetic::mul_public(*cond_i, *b)).collect::<Vec<_>>();

    // res = cond_true + cond_false to obliviously obtain the final result
    let res = izip!(cond_true.iter(), cond_false.iter()).map(|(a,b)| arithmetic::add(*a,*b)).collect::<Vec<_>>();

    Ok(res)
}

pub fn mux_if_share_then_public_vec_multithreads<T: IntRing2k, N: Network>(
    cond: &[Rep3RingShare<T>],
    a: &[Rep3RingShare<T>],
    b: &RingElement<T>,
    net: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{ 

    let task_chunks = get_task_chunks(cond, cond.len(), net.len())?;
    let a_chunks = get_task_chunks(a, a.len(), net.len())?;

    // coumpute if cond[i]==1 then a[i]
    let cond_true_chunks = net::join_all(
        task_chunks.into_iter().zip(a_chunks.into_iter()).zip(net.iter()).zip(states.iter_mut())
        .map(|(((cond_chunk, a_chunk), &n), state)| {
            move || { 
                let cond_true_tmp = arithmetic::local_mul_vec(&cond_chunk, &a_chunk, *state);
                arithmetic::reshare_vec(cond_true_tmp, n).unwrap_or_else(|e| panic!("Mux_if_share_then_public_vec_multithreads Reshare failed: {:?}", e))
            }
        })
    );

    let cond_true = cond_true_chunks.concat();

    // compute if cond[i]==0 then b
    let c_false = izip!(cond.iter()).map(|cond_i| arithmetic::sub_public_by_shared(RingElement::one(), *cond_i, states[0].id)).collect::<Vec<_>>();
    let cond_false = c_false.iter().map(|cond_i| arithmetic::mul_public(*cond_i, *b)).collect::<Vec<_>>();

    // res = cond_true + cond_false to obliviously obtain the final result
    let res = izip!(cond_true.iter(), cond_false.iter()).map(|(a,b)| arithmetic::add(*a,*b)).collect::<Vec<_>>();

    Ok(res)
}