

use protocols::protocols::{rep3_ring::{arithmetic::{add_public, mul_assign_public, mul_public, open, open_vec}, conversion}};
use itertools::izip;
use net::{Network};
use rand::{distributions::Standard, prelude::Distribution, random};
use algebra::ring::{int_ring::IntRing2k, ring_impl::RingElement};
use protocols::protocols::rep3_ring::Rep3RingShare;
//use protocols::protocols::rep3_ring::arithmetic::{open_bit};
use protocols::protocols::rep3_ring::binary;
use protocols::protocols::rep3_ring::detail;
use protocols::protocols::rep3_ring::conversion::b2a;
use communication::rep3::id::PartyID;
use communication::rep3::net_impl::Rep3NetworkImpl;
use random::rep3::Rep3State;
use num_traits::{One, Zero};
use crate::{kogge_stone_adder::{low_depth_binary_add_const_many, low_depth_binary_add_many}, transform, utils::get_task_chunks};


/// Computes a CMUX: If `c` is `1`, returns `x_t`, otherwise returns `x_f`.
pub fn cmux_public<T: IntRing2k>(
    c: &Rep3RingShare<T>,
    x_t: &RingElement<T>,
    x_f: &RingElement<T>,
) -> eyre::Result<Rep3RingShare<T>>
where
    Standard: Distribution<T>,
{
    let xor = *x_f ^ *x_t;
    let mut and = binary::and_with_public(c, &xor);
    and ^= x_f;
    Ok(and)
}

pub fn cmux_public_many<T: IntRing2k>(
    c: &[Rep3RingShare<T>],
    x_t: &[RingElement<T>],
    x_f: &[RingElement<T>],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let xor_vec = izip!(x_f, x_t).map(|(x_f, x_t)| *x_f ^ *x_t).collect::<Vec<_>>();
    let and_vec = binary::and_with_public_many(c, &xor_vec);
    let result = izip!(and_vec, x_f).map(|(and, x_f)| and ^ *x_f).collect::<Vec<_>>();
    Ok(result)
}

/// Division with i64 inputs and outputs. Note each party needs to do is different.
pub fn div_rem_const_public_arithmetic_i64<N: Network>(
    numerator: &Rep3RingShare<u64>,
    denominator: RingElement<u64>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Rep3RingShare<u64>>
{
    let c = denominator;
    
    // Generate randomness r shared between P0 and P1
    // P0 uses rand_a (shared with P1), P1 uses rand_b (shared with P0)
    let (r, r_err) = match state.id {
        PartyID::ID0 => {
            let r_share:(RingElement<u64>, RingElement<u64>) = random();
            let send_r: Vec<RingElement<u64>> = vec![r_share.0, r_share.1];
            net.send_prev_many(&send_r).unwrap_or_else(|e|panic!("error: div public {}", e));
            r_share
        }
        PartyID::ID2 => {
            let r_share = net.recv_next_many().unwrap_or_else(|e|panic!("error: div public {}", e));
            (r_share[0], r_share[1])
        }
        PartyID::ID1 => {
            (RingElement(u64::zero()), RingElement(u64::zero()))
        }
    };

    // Local computation
    let (y_share, err_share) = match state.id {
        PartyID::ID0 => {
            // P0 holds x0, x2. Sum = x0 + x2.
            
            let n_a = numerator.a.0 as i64;
           
            let n_b = numerator.b.0 as i64;

            let x_sum = n_a.wrapping_add(n_b);
            let c = c.0 as i64;
            //let y_share = x_sum / c - r;
            let mut y_share = x_sum / c - r.0 as i64;
            
            let  x_sum_neg = (x_sum < 0) as i64;
            y_share -= x_sum_neg;

            let err_share = (x_sum % c) + x_sum_neg * c - r_err.0 as i64;

            let y_share = RingElement(y_share as u64);
    
            let err_share = RingElement(err_share as u64);
        
            // err(0) = (x_sum) % c + x_sum_neg * c - r;
            // If strict C++ behavior is needed, we need to check MSB here.
            (y_share, err_share)
        }
        PartyID::ID2 => {
            // P1 holds x2, x3. In this protocol, P1 acts as helper with randomness r.
            //let y_share = RingElement::zero();
            
            let y_share = r;
            let err_share = r_err;

            (y_share, err_share)
        }
        PartyID::ID1 => {
            // P1 holds x2, x1. P1 computes on x2 (which is numerator.a for P2).

            let x2 = numerator.a.0  as i64;

            let c = c.0 as i64;

            let mut y_share = x2 / c;
            
            /*
            auto x_sum_neg = x(0) < 0;
            res(0) -= x_sum_neg;
            err(0) = x(0) % c + x_sum_neg * c - c;
            */

            let x_sum_neg = (x2 < 0) as i64;
            y_share -= x_sum_neg;

            let err_share = x2 % c + x_sum_neg * c - c;

            let y_share = RingElement(y_share as u64);
            //eprintln!("y_share: {}", y_share);
            
            let err_share = RingElement(err_share as u64);
            (y_share, err_share)
        }
    };

    
    let send_data = vec![y_share, err_share];
    
    net.send_next_many(&send_data)?;
   
    let recv_data: Vec<RingElement<u64>> = net.recv_prev_many()?;
    
    let y_received = recv_data[0];
    let err_received = recv_data[1];

    let q = Rep3RingShare {
        a: y_share,
        b: y_received,
    };
    
    let err_share = Rep3RingShare {
        a: err_share,
        b: err_received,
    };

    let open_err = open(err_share, net)?;
    //eprintln!("err_share: {}", open_err);

    let open_err = open_err.0 as i64;
    let err= RingElement(!(open_err <0) as u64) ;
    
    /*
    let check_0 = lt_public(err_share, RingElement::zero(), net, state)?;
    eprintln!("check_0: {}", open_bit(check_0, net)?);

    let neq_check_0 = !check_0;
    eprintln!("neq_check_0: {}", open_bit(neq_check_0, net)?);

    //let check_0_ari = transform::from_bit_to_arithmetic_t::<u64, N>(&[check_0], net, state)?;
    //let check_0_ari = transform::from_bit_to_arithmetic_t_one(&check_0, net, state)?;
    let check_0_ari = transform::from_bit_to_t_one(&neq_check_0)?;
    //eprintln!("check_0_ari: {}", open_bit(check_0_ari, net)?);

    let err = check_0_ari;
    //eprintln!("err: {}", open_bit(err, net)?);

    let err = conversion::bit_inject(&err, net, state)?;
    eprintln!("err: {}", open(err, net)?);
    */

    //eprintln!("q_share before err: {}", open(q_share, net)?);

    let res = add_public(q, err, state.id);

    Ok(res)
}
/*
err_share: 18446744073709551611
err_share: 18446744073709551611
err_share: 18446744073709551611
err: 1
err: 1
err: 1
 */

pub fn div_rem_const_public_arithmetic_many_i64<N: Network>(
    numerator: &[Rep3RingShare<u64>],
    denominator: RingElement<u64>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<u64>>>
{
    let c = denominator;
    let len = numerator.len();
    
    // Generate randomness
    let (r_vec, r_err_vec) = match state.id {
        PartyID::ID0 => {
            let mut r_0_vec = Vec::new();
            let mut r_1_vec = Vec::new();
            /* 
            for _ in 0..len {
                let (r_0,r_1) = state.rngs.rand.random_elements::<RingElement<u64>>();
                r_0_vec.push(r_0);
                r_1_vec.push(r_1);
            }
            */
            for _ in 0..len {
                let r_share:(u32, u32) = random();
                let (r_0,r_1) = (RingElement(r_share.0 as u64), RingElement(r_share.1 as u64));
                //let (r_0,r_1) = random();//这里必须用本地随机，或者三方都要消耗随机数生成器
                r_0_vec.push(r_0);
                r_1_vec.push(r_1);
            }

            
            net.send_prev_many(&r_0_vec).unwrap_or_else(|e|panic!("error: div public {}", e));
            net.send_prev_many(&r_1_vec).unwrap_or_else(|e|panic!("error: div public {}", e));
            (r_0_vec, r_1_vec)
        }
        PartyID::ID2 => {
            
            let r_0_vec: Vec<RingElement<u64>> = net.recv_next_many().unwrap_or_else(|e|panic!("error: div public {}", e));
            let r_1_vec: Vec<RingElement<u64>> = net.recv_next_many().unwrap_or_else(|e|panic!("error: div public {}", e));
            
            (r_0_vec, r_1_vec)
        }
        PartyID::ID1 => {
            (vec![RingElement::zero(); len], vec![RingElement::zero(); len])
        }
    };

    let mut y_shares = Vec::with_capacity(len);
    let mut err_shares = Vec::with_capacity(len);

    match state.id {
        PartyID::ID0 => {
            for (num, r, r_err) in izip!(numerator, r_vec, r_err_vec) {
                let n_a= num.a.0 as i64;
                let n_b= num.b.0 as i64;
                let x_sum = n_a.wrapping_add(n_b);
                let c = c.0 as i64;

                let mut y_share = x_sum / c - (r.0 as i64);
        
                let  x_sum_neg = (x_sum < 0) as i64;
                y_share -= x_sum_neg;

                let err_share = (x_sum % c) + x_sum_neg * c - r_err.0 as i64;

                let y_share = RingElement(y_share as u64);
        
                let err_share = RingElement(err_share as u64);

                y_shares.push(y_share);
                err_shares.push(err_share);
            }
        }
        PartyID::ID2 => {
            y_shares = r_vec;
            err_shares = r_err_vec;
        }
        PartyID::ID1 => {

            for num in numerator {

                let x2 = num.a.0 as i64;
                let c = c.0 as i64;

                let mut y_share = x2 / c;
                /*
                auto x_sum_neg = x(0) < 0;
                res(0) -= x_sum_neg;
                err(0) = x(0) % c + x_sum_neg * c - c;
                */

                let x_sum_neg = (x2 < 0) as i64;
                y_share -= x_sum_neg;

                let err_share = x2 % c + x_sum_neg * c - c;

                let y_share = RingElement(y_share as u64);
                //eprintln!("y_share: {}", y_share);
                
                let err_share = RingElement(err_share as u64);

                y_shares.push(y_share);
                err_shares.push(err_share);
            }
        }
    }
    let y_shares_clone = y_shares.clone();
    let err_shares_clone= err_shares.clone();
    let send_data = vec![y_shares_clone, err_shares_clone].concat();
    net.send_next_many(&send_data)?;
    let recv_data: Vec<RingElement<u64>> = net.recv_prev_many()?;

    
    let recv_r_shares = recv_data[0..len].to_vec();
    let recv_err_shares = recv_data[len..2*len].to_vec();

    let mut q_res = Vec::with_capacity(len);
    let mut r_res = Vec::with_capacity(len);

    for (a,b) in izip!(y_shares, recv_r_shares) {
        q_res.push(Rep3RingShare {
            a,
            b,
        });
    }

    for (a,b) in izip!(err_shares, recv_err_shares) {
        r_res.push(Rep3RingShare {
            a,
            b,
        });
    }

    let open_err = open_vec(&r_res, net)?;

    let err_res = open_err.iter().map(|err| RingElement(!((err.0 as i64)<0) as u64 )).collect::<Vec<_>>();

    let res = q_res.iter().zip(err_res.iter()).map(|(q, err)| add_public(*q, *err, state.id)).collect::<Vec<_>>();

    Ok(res)
}

/// Performs non-restoring division.
/// N and D are inputs. bits is the number of bits for the quotient.
/// Assumes T is large enough (at least 2*bits).
pub fn non_restoring_division<T: IntRing2k, N: Network>(
    numerator: &Rep3RingShare<T>,
    denominator: &Rep3RingShare<T>,
    bits: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Rep3RingShare<T>>
where
    Standard: Distribution<T>,
{
    let mut r = numerator.clone();
    let d_shifted = denominator.clone() << bits;

    // Precompute -D
    // -D = !D + 1
    let one = RingElement::one();
    let neg_d = detail::low_depth_binary_add_const(&(!d_shifted), &one, net, state)?;

    let mut p = Rep3RingShare::zero_share();

    for _ in (0..bits).rev() {
        // b = MSB(R). In 2's complement with sufficient padding, MSB is at T::K - 1
        let b = r >> (T::K - 1);

        // mask = -b (arithmetic negation in binary domain) = !b + 1
        // If b=0, mask=0. If b=1, mask=-1 (all 1s).
        let mask = detail::low_depth_binary_add_const(&(!b), &one, net, state)?;

        // term = b ? D : -D
        // If b=1 (neg), we want D.
        // If b=0 (pos), we want -D.
        let term = binary::cmux(&mask, &d_shifted, &neg_d, net, state)?;

        // R = 2*R + term
        let r_shifted = r << 1;
        r = detail::low_depth_binary_add(&r_shifted, &term, net, state)?;

        // P = (P << 1) | (1-b)
        // 1-b = b ^ 1
        let bit_val = b ^ one;
        p = (p << 1) ^ bit_val;
    }

    // Q = P * 2 - (2^n - 1)
    // Q = (P << 1) - (2^n - 1)
    // Q = (P << 1) + !(2^n - 1) + 1

    let p_shifted = p << 1;
    let mask_n = (RingElement::one() << bits) - RingElement::one();
    let mask_n_share = binary::promote_to_trivial_share(state.id, &mask_n);

    // sub(a, b) = add(a, !b + 1)
    let neg_mask = detail::low_depth_binary_add_const(&(!mask_n_share), &one, net, state)?;
    let mut q = detail::low_depth_binary_add(&p_shifted, &neg_mask, net, state)?;

    // Correction: If R < 0, Q = Q - 1
    let r_is_neg = r >> (T::K - 1);
    let neg_r_is_neg = detail::low_depth_binary_add_const(&(!r_is_neg), &one, net, state)?;
    q = detail::low_depth_binary_add(&q, &neg_r_is_neg, net, state)?;

    Ok(q)
}


/*
pub fn non_restoring_division_share_by_public<T: IntRing2k, N: Network>(
    numerator: &Rep3RingShare<T>,
    denominator: &RingElement<T>,
    bits: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Rep3RingShare<T>>
where
    Standard: Distribution<T>,
{

    let mut r = numerator.clone();
    let d_shifted = denominator.clone() << bits;

    // Precompute -D
    // -D = !D + 1
    let one = RingElement::one();
    let neg_d = !d_shifted + one;

    let mut p = Rep3RingShare::zero_share();

    for _ in (0..bits).rev() {
        // b = MSB(R). In 2's complement with sufficient padding, MSB is at T::K - 1
        let b = r >> (T::K - 1);

        // mask = -b (arithmetic negation in binary domain) = !b + 1
        // If b=0, mask=0. If b=1, mask=-1 (all 1s).
        let mask = detail::low_depth_binary_add_const(&(!b), &one, net, state)?;

        // term = b ? D : -D
        // If b=1 (neg), we want D.
        // If b=0 (pos), we want -D.
        let term = cmux_public::<T>(&mask, &d_shifted, &neg_d)?;
        
        // R = 2*R + term
        let r_shifted = r << 1;
        r = detail::low_depth_binary_add(&r_shifted, &term, net, state)?;

        // P = (P << 1) | (1-b)
        // 1-b = b ^ 1
        let bit_val = b ^ one;
        p = (p << 1) ^ bit_val;
    }
    
    let p_shifted = p << 1;
    let mask_n = (RingElement::one() << bits) - RingElement::one();
    let mask_n_share = binary::promote_to_trivial_share(state.id, &mask_n);

    // sub(a, b) = add(a, !b + 1)
    let neg_mask = detail::low_depth_binary_add_const(&(!mask_n_share), &one, net, state)?;
    let mut q = detail::low_depth_binary_add(&p_shifted, &neg_mask, net, state)?;

    // Correction: If R < 0, Q = Q - 1
    let r_is_neg = r >> (T::K - 1);
    let neg_r_is_neg = detail::low_depth_binary_add_const(&(!r_is_neg), &one, net, state)?;
    q = detail::low_depth_binary_add(&q, &neg_r_is_neg, net, state)?;

    Ok(q)
}
*/

/// Batched version of non-restoring division.
pub fn non_restoring_division_many<T: IntRing2k, N: Network>(
    numerator: &[Rep3RingShare<T>],
    denominator: &[Rep3RingShare<T>],
    bits: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{ 
    let mut r = numerator.to_vec();
    let d_shifted = denominator.to_vec().iter().map(|x| x << bits).collect::<Vec<_>>();
    let one = RingElement::one();
    //let one_vec = vec![binary::promote_to_trivial_share(state.id, &RingElement::one());d_shifted.len()];
    let one_vec = vec![one; numerator.len()];

    let not_d_shifted = d_shifted.iter().map(|x| !x).collect::<Vec<_>>();
    let neg_d = low_depth_binary_add_const_many(&not_d_shifted, &one_vec, net, state)?;

    let mut p = vec![Rep3RingShare::zero_share(); numerator.len()];
    for _ in (0..bits).rev() { 
        let b = izip!(&r).map(|x| x >> (T::K - 1)).collect::<Vec<_>>();

        let not_b = izip!(&b).map(|b| !b).collect::<Vec<_>>();
        let mask = low_depth_binary_add_const_many(&not_b, &one_vec, net, state)?;

        let term = binary::cmux_many(&mask, &d_shifted, &neg_d, net, state)?;

        let r_shifted = izip!(&r).map(|x| x << 1).collect::<Vec<_>>();

        r = detail::low_depth_binary_add_many(&r_shifted, &term, net, state)?;

        let bit_val = izip!(&b, &one_vec).map(|(b, one)| b ^ one).collect::<Vec<_>>();

        p = izip!(p, bit_val).map(|(p, bit_val)| (p << 1) ^ bit_val).collect::<Vec<_>>();
    }

    let p_shifted = izip!(&p).map(|p| p << 1).collect::<Vec<_>>();
    let mask_n = (RingElement::one() << bits) - RingElement::one();
    //let mask_n_share = binary::promote_to_trivial_share(state.id, &mask_n);
    let mask_n_vec = vec![!mask_n; numerator.len()];
    
    let neg_mask = mask_n_vec.iter().map(|x| *x + one).collect::<Vec<_>>();
    //let neg_mask = detail::low_depth_binary_add_const_many(&mask_n_share_vec, &one_vec, net, state)?;
    let mut q = low_depth_binary_add_const_many(&p_shifted, &neg_mask, net, state)?;

    // Correction: If R < 0, Q = Q - 1
    let r_is_neg = izip!(&r).map(|r| r >> (T::K - 1)).collect::<Vec<_>>();
    let not_r_is_neg = izip!(&r_is_neg).map(|r| !r).collect::<Vec<_>>();
    let neg_r_is_neg = low_depth_binary_add_const_many(&not_r_is_neg, &one_vec, net, state)?;
    q = low_depth_binary_add_many(&q, &neg_r_is_neg, net, state)?;

    Ok(q)
}

/* 
pub fn non_restoring_division_share_by_public_many<T: IntRing2k, N: Network>(
    numerator: &[Rep3RingShare<T>],
    denominator: &[RingElement<T>],
    bits: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,{

    let mut r = numerator.to_vec();
    let d_vec = denominator.to_vec();
    let d_shifted = d_vec.iter().map(|x| *x << bits).collect::<Vec<_>>();
    //let one_vec = vec![binary::promote_to_trivial_share(state.id, &RingElement::one());d_shifted.len()];
    let one = RingElement::one();
    let one_vec = vec![one; numerator.len()];

    let not_d_shifted = d_shifted.iter().map(|x| !x).collect::<Vec<_>>();
    let neg_d = not_d_shifted.iter().map(|x| *x + one).collect::<Vec<_>>();

    let mut p = vec![Rep3RingShare::zero_share(); numerator.len()];

    for _ in (0..bits).rev() { 
        let b = izip!(&r).map(|x| x >> (T::K - 1)).collect::<Vec<_>>();

        let not_b = izip!(&b).map(|b| !b).collect::<Vec<_>>();
        let mask = detail::low_depth_binary_add_const_many(&not_b, &one_vec, net, state)?;

        let term = cmux_public_many(&mask, &d_shifted, &neg_d)?;

        let r_shifted = izip!(&r).map(|x| x << 1).collect::<Vec<_>>();

        r = detail::low_depth_binary_add_many(&r_shifted, &term, net, state)?;

        let bit_val = izip!(&b, &one_vec).map(|(b, one)| b ^ one).collect::<Vec<_>>();

        p = izip!(p, bit_val).map(|(p, bit_val)| (p << 1) ^ bit_val).collect::<Vec<_>>();
    }

    let p_shifted = izip!(&p).map(|p| p << 1).collect::<Vec<_>>();
    let mask_n = (RingElement::one() << bits) - RingElement::one();
    //let mask_n_share = binary::promote_to_trivial_share(state.id, &mask_n);
    let mask_n_vec = vec![!mask_n; numerator.len()];
    
    let neg_mask = mask_n_vec.iter().map(|x| *x + one).collect::<Vec<_>>();
    //let neg_mask = detail::low_depth_binary_add_const_many(&mask_n_share_vec, &one_vec, net, state)?;
    let mut q = detail::low_depth_binary_add_const_many(&p_shifted, &neg_mask, net, state)?;

    // Correction: If R < 0, Q = Q - 1
    let r_is_neg = izip!(&r).map(|r| r >> (T::K - 1)).collect::<Vec<_>>();
    let not_r_is_neg = izip!(&r_is_neg).map(|r| !r).collect::<Vec<_>>();
    let neg_r_is_neg = detail::low_depth_binary_add_const_many(&not_r_is_neg, &one_vec, net, state)?;
    q = detail::low_depth_binary_add_many(&q, &neg_r_is_neg, net, state)?;

    Ok(q)
}
*/


/// Computes division by a public, odd value inside Z_{2^k}.
/// The divisor must be invertible modulo 2^k, i.e. it has to be odd. The operation
/// is implemented by multiplying with the modular inverse of the divisor.
pub fn div_by_odd_public<T: IntRing2k>(
    shared: Rep3RingShare<T>,
    divisor: RingElement<T>,
) -> eyre::Result<Rep3RingShare<T>> {
    let inverse = invert_odd_mod_power_of_two(divisor)?;
    Ok(mul_public(shared, inverse))
}

/// Performs in-place division of a shared value by a public, odd value inside Z_{2^k}.
pub fn div_assign_by_odd_public<T: IntRing2k>(
    shared: &mut Rep3RingShare<T>,
    divisor: RingElement<T>,
) -> eyre::Result<()> {
    let inverse = invert_odd_mod_power_of_two(divisor)?;
    mul_assign_public(shared, inverse);
    Ok(())
}

fn invert_odd_mod_power_of_two<T: IntRing2k>(
    divisor: RingElement<T>,
) -> eyre::Result<RingElement<T>> {
    if divisor.is_zero() {
        eyre::bail!("Division by zero is undefined in Z_(2^k)");
    }
    if divisor.get_bit(0) != RingElement::one() {
        eyre::bail!("Only odd divisors have inverses in Z_(2^k)");
    }

    let mut inverse = RingElement::one();
    let two = RingElement::one() + RingElement::one();
    let mut covered_bits = 1usize;
    while covered_bits < T::K {
        inverse *= two - divisor * inverse;
        covered_bits <<= 1;
    }
    Ok(inverse)
}


pub fn div<T: IntRing2k, N: Network>(
    numerator: &Rep3RingShare<T>,
    denominator: &Rep3RingShare<T>,
    bits: usize,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Rep3RingShare<T>>
where
    Standard: Distribution<T>,
{
    // First convert all inputs to binary
    let numerator_binary = conversion::a2b(*numerator, net, state)?;
    let denominator_binary = conversion::a2b(*denominator, net, state)?;

    let q = non_restoring_division::<T,N>(&numerator_binary, &denominator_binary, bits, net, state)?;

    let q = b2a(&q, net, state)?;

    Ok(q)
}

/// Division.
pub fn div_multithreads<T: IntRing2k, N: Network>(
    numerator: &[Rep3RingShare<T>],
    denominator: &[Rep3RingShare<T>],
    bits: usize,
    nets: &[&N],
    states: &mut[&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    // First convert all inputs to binary
    let numerator_binary = transform::a2b_many_multithreads(numerator, nets, states)?;
    let denominator_binary = transform::a2b_many_multithreads(denominator, nets, states)?;

    let numerator_chunks = get_task_chunks(&numerator_binary,numerator.len(), nets.len())?;
    let denominator_chunks = get_task_chunks(&denominator_binary,denominator.len(), nets.len())?;

    let q = net::join_all(
        numerator_chunks.into_iter().zip(denominator_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((num_chunk, den_chunk), &n), state)| {
            move || {
                non_restoring_division_many::<T,N>(num_chunk, den_chunk, bits, n, *state).unwrap_or_else(|e| panic!("Division failed: {:?}", e))
            }
        })
    );

    let q = q.concat();

    let q = transform::b2a_many_multithreads(&q, nets, states)?;

    Ok(q)
}


/* 
/// Binary division of a share by a public element, lower than arithmetic division.
pub fn div_share_by_public_multithreads<T: IntRing2k, N: Network>(
    numerator: &[Rep3RingShare<T>],
    denominator: &RingElement<T>,
    bits: usize,
    nets: &[&N],
    states: &mut[&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let numerator_binary = transform::a2b_many_multithreads(numerator, nets, states)?;

    let numerator_chunks = get_task_chunks(&numerator_binary,numerator.len(), nets.len())?;
    let d_vec = vec![*denominator;numerator.len()];
    let d_chunks = get_task_chunks(&d_vec,d_vec.len(), nets.len())?;
    
    let q = net::join_all(
        numerator_chunks.into_iter().zip(d_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((num_chunk, den_chunk), &n), state)| {
            move || {
                non_restoring_division_share_by_public_many::<T,N>(num_chunk, den_chunk, bits, n, *state).unwrap_or_else(|e| panic!("Division failed: {:?}", e))
            }
        })
    );

    let q = q.concat();

    //let q = transform::b2a_many_multithreads(&q, nets, states)?;

    Ok(q)
}
*/

pub fn div_share_by_public<N: Network>(
    numerator: Rep3RingShare<u64>,
    denominator: &RingElement<u64>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Rep3RingShare<u64>> {

    let q = div_rem_const_public_arithmetic_i64(&numerator, *denominator, net, state)?;
    
    Ok(q)
}

/// fast version
pub fn div_share_by_public_arithmetic_multithreads<N: Network>(
    numerator: &[Rep3RingShare<u64>],
    denominator: &RingElement<u64>,
    nets: &[&N],
    states: &mut[&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<u64>>>
{
    
    let numerator_chunks = get_task_chunks(&numerator,numerator.len(), nets.len())?;
    
    let q = net::join_all(
        numerator_chunks.into_iter().zip(nets.iter()).zip(states.iter_mut()).map(|((num_chunk, &n), state)| {
            move || {
                div_rem_const_public_arithmetic_many_i64(num_chunk, *denominator, n, *state).unwrap_or_else(|e| panic!("Division failed: {:?}", e))
                
            }
        })
    );

    let q = q.concat();

    Ok(q)
}



