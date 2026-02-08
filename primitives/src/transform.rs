
use num_traits::{One,AsPrimitive};
use rand::{distributions::Standard, prelude::Distribution};
use protocols::protocols::rep3_ring::{Rep3State};
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::{rep3_ring::Rep3RingShare};
use protocols::protocols::rep3_ring::conversion;
use protocols::protocols::rep3_ring::ring::bit::Bit;
use net::Network;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use crate::utils::{get_task_chunks,get_mut_task_chunks};


// u32 allows to sort 4*10^9 elements. Inputs of this size require 32*4*10^9*2 bytes, i.e., 256 GB of RAM
type PermRing = u32;

pub fn inject_bit<T: IntRing2k, N: Network>(
    inputs: &[Rep3RingShare<T>],
    bit: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>> {
    let len = inputs.len();
    let mut bits = Vec::with_capacity(len);

    for inp in inputs {
        let a = inp.a.get_bit_for_perm(bit) as PermRing;
        let b = inp.b.get_bit_for_perm(bit) as PermRing;
        bits.push(Rep3RingShare::new_ring(a.into(), b.into()));
    }
    
    conversion::bit_inject_many(&bits, net, state)
}

pub fn inject_bit_multithreads<T: IntRing2k, N: Network>(
    inputs: &[Rep3RingShare<T>],
    bit: usize,
    net:&[&N],
    state: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<PermRing>>> {
    let len = inputs.len();
    let mut bits = Vec::with_capacity(len);

    for inp in inputs {
        let a = inp.a.get_bit_for_perm(bit) as PermRing;
        let b = inp.b.get_bit_for_perm(bit) as PermRing;
        bits.push(Rep3RingShare::new_ring(a.into(), b.into()));
    }

    let chunks = get_task_chunks(&bits, len, net.len())?;

    let results = net::join_all(
        chunks.into_iter().zip(net.iter()).zip(state.iter_mut()).map(|((chunk, &n), state)| {
            move || {
                conversion::bit_inject_many(chunk, n, *state).unwrap_or_else(|e| panic!("Bit inject failed: {:?}", e))
            }
        })
    );

    let concatenated: Vec<Rep3RingShare<PermRing>> = results.concat();

    Ok(concatenated)
}

pub fn bit_decompose_many<T: IntRing2k, N: Network>(
    inputs: &[Rep3RingShare<T>],
    bitsize: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
    Standard: Distribution<T>,{

    let mask = (RingElement::one() << bitsize) - RingElement::one();
    let mut bits = vec![Rep3RingShare::zero_share(); inputs.len()];

    let mut res_vec = conversion::a2b_many(&inputs, net, state)?;
    for (i, mut binary) in res_vec.drain(..).enumerate() {
        binary &= &mask;
        bits[i] = binary;
    }
   
    Ok(bits)
}

pub fn bit_decompose_many_multithreads<T: IntRing2k, N: Network>(
    priv_inputs: &[Rep3RingShare<T>],
    bitsize: usize,
    net: &[&N],
    state: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
    Standard: Distribution<T>,{
        
    let mask = (RingElement::one() << bitsize) - RingElement::one();
    let mut priv_bits = vec![Rep3RingShare::zero_share(); priv_inputs.len()];
    let len = priv_inputs.len();

    let priv_bit_chunks = get_mut_task_chunks(&mut priv_bits, len, net.len())?;
    let priv_input_chunks = get_task_chunks(&priv_inputs, len, net.len())?;
    //let (split1, split2) = priv_bits.split_at_mut(priv_inputs.len() / 2);

    net::join_all(
        priv_input_chunks.into_iter().zip(priv_bit_chunks.into_iter()).zip(net.iter()).zip(state.iter_mut())
        .map(|(((priv_input_chunk, priv_bit_chunk), &n), state)| {
            move || {
                let mut vec = conversion::a2b_many(priv_input_chunk, n, *state).unwrap_or_else(|e| panic!("Decompose failed: {:?}", e));
                for (i, mut binary) in vec.drain(..).enumerate() {
                    binary &= &mask;
                    priv_bit_chunk[i] = binary;
                }
            }
        })
    );
    

    Ok(priv_bits)
}

/// transform bit to T but result is also binary share
pub fn from_bit_to_t_one<T: IntRing2k>(
    bit_share: &Rep3RingShare<Bit>,
) -> eyre::Result<Rep3RingShare<T>> 
{
    let bit_share_t = Rep3RingShare::new(
        T::from(bit_share.a.0.convert()), 
        T::from(bit_share.b.0.convert())
    );

    Ok(bit_share_t)
}

/// transform bit to T but result is also binary share
pub fn from_bit_to_t<T: IntRing2k>(
    bit_shares: &[Rep3RingShare<Bit>],
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
{
    let bit_shares_t = bit_shares
        .iter()
        .map(|bit_share| Rep3RingShare::new(
            T::from(bit_share.a.0.convert()), 
            T::from(bit_share.b.0.convert())
        ))
        .collect();

    Ok(bit_shares_t)
}

/// transform bit to T but result is also binary share. Drop bitshares after conversion
pub fn from_bit_to_t_drop<T: IntRing2k>(
    bit_shares: Vec<Rep3RingShare<Bit>>,
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
{
    let bit_shares_t = bit_shares
        .iter()
        .map(|bit_share| Rep3RingShare::new(
            T::from(bit_share.a.0.convert()), 
            T::from(bit_share.b.0.convert())
        ))
        .collect();

    Ok(bit_shares_t)
}

/// transform bit to T , result is arithmetic share
pub fn from_bit_to_arithmetic_t_one<T: IntRing2k, N: Network>(
    e: &Rep3RingShare<Bit>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Rep3RingShare<T>> 
where
Standard: Distribution<T>, {
    let e_res = from_bit_to_t_one::<T>(&e)?;
    let e_t = conversion::bit_inject(&e_res, net, state)?;
    Ok(e_t)
}

/// transform bit to T , result is arithmetic share
pub fn from_bit_to_arithmetic_t<T: IntRing2k, N: Network>(
    e: &[Rep3RingShare<Bit>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
Standard: Distribution<T>, {
    let e_res = from_bit_to_t::<T>(&e)?;
    let e_t = conversion::bit_inject_many(&e_res, net, state)?;
    Ok(e_t)
}



/// from bit to arithmetic T in multithreads, result is arithmetic share
pub fn from_bit_to_arithmetic_t_multithreads<T: IntRing2k, N: Network>(
    e: &[Rep3RingShare<Bit>],
    net: &[&N],
    state: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
Standard: Distribution<T>, {

    let e_res = from_bit_to_t::<T>(&e)?;

    let e_chunks = get_task_chunks(&e_res, e.len(), net.len())?;

    let e_t = net::join_all(
        e_chunks.into_iter().zip(net.iter()).zip(state.iter_mut())
        .map(|((e_chunk, &n), state)| {
            move || {
                conversion::bit_inject_many(e_chunk, n, *state).unwrap_or_else(|e| panic!("transform:: get_e_t_multithreads {:?}", e))
            }
        })
    );

    let e_t = e_t.concat();
    
    Ok(e_t)
}

pub fn a2b_many_multithreads<T: IntRing2k, N: Network>(
    inputs: &[Rep3RingShare<T>],
    net: &[&N],
    state: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
    Standard: Distribution<T>,{ 
    let len = inputs.len();

    let inputs_tasks = get_task_chunks(&inputs, len, net.len())?;
    let results = net::join_all(
        inputs_tasks.into_iter().zip(net.iter()).zip(state.iter_mut()).map(|((chunk, &n), state)| {
            move || {
                conversion::a2b_many(chunk, n, *state).unwrap_or_else(|e| panic!("A2B failed: {:?}", e))
            }
        })
    );

    let results = results.concat();

    Ok(results)
}


pub fn b2a_many_multithreads<T: IntRing2k, N: Network>(
    binary_inputs: &[Rep3RingShare<T>],
    net: &[&N],
    state: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>> 
where
    Standard: Distribution<T>,{ 
    let len = binary_inputs.len();

    let inputs_tasks = get_task_chunks(&binary_inputs, len, net.len())?;
    let results = net::join_all(
        inputs_tasks.into_iter().zip(net.iter()).zip(state.iter_mut()).map(|((chunk, &n), state)| {
            move || {
                conversion::b2a_many(chunk, n, *state).unwrap_or_else(|e| panic!("A2B failed: {:?}", e))
            }
        })
    );

    let results = results.concat();

    Ok(results)
}



/// An upcast of a Rep3RingShare from a smaller ring to a larger ring
/// Does require network interaction
pub fn upcast_a2b_many_multithreads<T, U, N>(
    shares: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut[&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<U>>>
where
    T: IntRing2k + AsPrimitive<U>,
    U: IntRing2k,
    N: Network,
    Standard: Distribution<T> + Distribution<U>,
{
    assert!(T::K < U::K);

    let binary_vec = a2b_many_multithreads(shares, nets, states)?;

    let binary_vec = binary_vec.iter().map(|binary| {
        Rep3RingShare {
            a: RingElement(binary.a.0.as_()),
            b: RingElement(binary.b.0.as_()),
        }
    }).collect::<Vec<_>>();

    let result = b2a_many_multithreads(&binary_vec, nets, states)?;

    Ok(result)
}