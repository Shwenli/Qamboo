
use crate::rep3::Rep3State;
use rand::{distributions::Standard, prelude::Distribution};
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;
use rayon::iter::IndexedParallelIterator;

struct SendPtr<T>(*mut T);
impl<T> Copy for SendPtr<T> {}
impl<T> Clone for SendPtr<T> {
    fn clone(&self) -> Self { *self }
}
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}


pub fn random_elements_vec_multithreads<T>(
    states: &mut [&mut Rep3State],
    len: usize,
) -> (Vec<T>, Vec<T>)
where
    Standard: Distribution<T>,
    T: Send + Sync,
{
    let num_states = states.len();
    let base_chunk = len / num_states;
    let remainder = len % num_states;

    let (mask, mask_b) = states.par_iter_mut()
        .enumerate()
        .with_min_len(1024)
        .map(|(i, state)| {
            let chunk_len = base_chunk + if i < remainder { 1 } else { 0 };
            state.rngs.rand.random_elements_vec::<T>(chunk_len)
        })
        .reduce(
            || (Vec::new(), Vec::new()),
            |(mut a1, mut b1), (mut a2, mut b2)| {
                a1.append(&mut a2);
                b1.append(&mut b2);
                (a1, b1)
            }
        );

    (mask, mask_b)
}

pub fn masking_elements_vec_multithreads<T>(
    states: &mut [&mut Rep3State],
    len: usize,
) -> Vec<T>
where
    Standard: Distribution<T>,
    T: Send + Sync + std::ops::Sub<Output = T>,
{
    let num_states = states.len();
    let base_chunk = len / num_states;
    let remainder = len % num_states;

    let mask = states.par_iter_mut()
        .enumerate()
        .with_min_len(1024)  
        .map(|(i, state)| {
            let chunk_len = base_chunk + if i < remainder { 1 } else { 0 };
            state.rngs.rand.masking_elements_vec::<T>(chunk_len)
        })
        .reduce(
            || Vec::new(),           // identity
            |mut acc, mut vec| {     // reduce_op
                acc.append(&mut vec); 
                acc
            }
        );
    mask

}

pub fn random_elements1_3keys_vec_multithreads<T>(
    states: &mut [&mut Rep3State],
    len: usize,
) -> (Vec<T>, Vec<T>, Vec<T>)
where
    Standard: Distribution<T>,
    T: Send + Sync,
{
    let num_states = states.len();
    let base_chunk = len / num_states;
    let remainder = len % num_states;

    let (a, b, c) = states.par_iter_mut()
        .enumerate()
        .with_min_len(1024)
        .map(|(i, state)| {
            let chunk_len = base_chunk + if i < remainder { 1 } else { 0 };
            state.rngs.bitcomp1.random_elements_3keys_vec::<T>(chunk_len)
        })
        .reduce(
            || (Vec::new(), Vec::new(), Vec::new()),
            |(a1, b1, c1), (mut a2, mut b2, mut c2)| {
                let mut a = a1;
                a.append(&mut a2);
                let mut b = b1;
                b.append(&mut b2);
                let mut c = c1;
                c.append(&mut c2);
                (a, b, c)
            }
        );

    (a, b, c)
}

pub fn random_elements2_3keys_vec_multithreads<T>(
    states: &mut [&mut Rep3State],
    len: usize,
) -> (Vec<T>, Vec<T>, Vec<T>)
where
    Standard: Distribution<T>,
    T: Send + Sync,
{
    let num_states = states.len();
    let base_chunk = len / num_states;
    let remainder = len % num_states;

    let (a, b, c) = states.par_iter_mut()
        .enumerate()
        .with_min_len(1024)
        .map(|(i, state)| {
            let chunk_len = base_chunk + if i < remainder { 1 } else { 0 };
            state.rngs.bitcomp2.random_elements_3keys_vec::<T>(chunk_len)
        })
        .reduce(
            || (Vec::new(), Vec::new(), Vec::new()),
            |(a1, b1, c1), (mut a2, mut b2, mut c2)| {
                let mut a = a1;
                a.append(&mut a2);
                let mut b = b1;
                b.append(&mut b2);
                let mut c = c1;
                c.append(&mut c2);
                (a, b, c)
            }
        );

    (a, b, c)
}


pub fn random_elements_rng2_vec_multithreads<T>(
    states: &mut [&mut Rep3State],
    len: usize,
) -> Vec<T>
where
    Standard: Distribution<T>,
    T: Send + Sync,
{
    let num_states = states.len();
    let base_chunk = len / num_states;
    let remainder = len % num_states;

    let mut result: Vec<T> = Vec::with_capacity(len);
    let send_ptr = SendPtr(result.as_mut_ptr());

    states
        .par_iter_mut()
        .enumerate()
        .with_min_len(1024)
        .for_each(|(i, state)| {
            let send_ptr = send_ptr; 
            let start = i * base_chunk + i.min(remainder);
            let chunk_len = base_chunk + if i < remainder { 1 } else { 0 };

            unsafe {
                let dst = send_ptr.0.add(start);
                for j in 0..chunk_len {
                    dst.add(j).write(state.rngs.rand.random_element_rng2());
                }
            }
        });

    // Safety: All len elements have been written by the threads (non-overlapping regions covering [0, len))
    unsafe { result.set_len(len); }

    result
}

pub fn random_elements_rng1_vec_multithreads<T>(
    states: &mut [&mut Rep3State],
    len: usize,
) -> Vec<T>
where
    Standard: Distribution<T>,
    T: Send + Sync,
{
    let num_states = states.len();
    let base_chunk = len / num_states;
    let remainder = len % num_states;
    
    let mut result: Vec<T> = Vec::with_capacity(len);
    let send_ptr = SendPtr(result.as_mut_ptr());

    states
        .par_iter_mut()
        .enumerate()
        .with_min_len(1024)
        .for_each(|(i, state)| {
            let send_ptr = send_ptr; 
            let start = i * base_chunk + i.min(remainder);
            let chunk_len = base_chunk + if i < remainder { 1 } else { 0 };

            unsafe {
                let dst = send_ptr.0.add(start);
                for j in 0..chunk_len {
                    dst.add(j).write(state.rngs.rand.random_element_rng1());
                }
            }
        });

    unsafe { result.set_len(len); }

    result
}



