use rand::distributions::Standard;
use rand::prelude::Distribution;

use protocols::protocols::rep3_ring::Rep3RingShare;
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use net::Network;
use operator::distinct::distinct_after_groupby_multithreads;
use primitives::compare;
use primitives::transform;
use primitives::utils::prefix_sum_sequential;
use primitives::utils::get_task_chunks;
use crate::share_column::ShareType;
use crate::share_column::ShareColumn;
use crate::column_operator::{ColumnBooleanOperator, Distinct, PrefixSum, TransformBetweenArithAndBinary};
use crate::NetStateArgs;

/// Prefix sum of a column and return the last element as the total sum
impl <T: IntRing2k> PrefixSum<Rep3RingShare<T>> for ShareColumn<Rep3RingShare<T>> 
where
Standard: Distribution<T>,{
    fn prefix_sum(&self) -> Rep3RingShare<T> {
        let data = self.get_data();
        let prefix_sum_vec = prefix_sum_sequential(data).unwrap_or_else(|e| panic!("Prefix sum error: {}", e));

        prefix_sum_vec[data.len()-1]
    }

}

impl<T: IntRing2k> TransformBetweenArithAndBinary<Rep3RingShare<T>> for ShareColumn<Rep3RingShare<T>>
where
Standard: Distribution<T>,{
    fn from_arithmetic_to_binary<N: Network>(
        &mut self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        let (nets, _state0, _state1, states) = netstate_args.split();

        let data = self.get_data();
        let new_data = transform::a2b_many_multithreads(&data, nets, states)?;
        
        self.update_data(new_data);
        self.update_datatype(ShareType::Binary);

        Ok(())
    }

    fn from_binary_to_arithmetic<N: Network>(
        &mut self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        let (nets, _state0, _state1, states) = netstate_args.split();

        let data = self.get_data();
        let new_data = transform::b2a_many_multithreads(&data, nets, states)?;

        self.update_data(new_data);
        self.update_datatype(ShareType::Arithmetic);

        Ok(())
    }
}

impl<T:IntRing2k> Distinct<Rep3RingShare<T>> for ShareColumn<Rep3RingShare<T>>
where
Standard: Distribution<T>,{
    fn distinct<N: Network>(
        &self,
        valid: &[Rep3RingShare<T>],
        e: &[Rep3RingShare<T>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<Vec<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();

        let val_data = self.get_data();
        
        let result = distinct_after_groupby_multithreads(val_data, e, valid, nets, states)?;

        Ok(result)
    }
}


impl<T: IntRing2k> ColumnBooleanOperator<Rep3RingShare<T>, T> for ShareColumn<Rep3RingShare<T>>
where
Standard: Distribution<T>,{ 

    fn eq<N: Network>(
        &self, 
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> 
    where
    Standard: Distribution<T>,{ 

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();
        
        let reusult_bit = compare::eq_many_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&reusult_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn eq_binary<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();
        
        let reusult_bit = compare::eq_many_binary_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&reusult_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn eq_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (net, _state0, _state1, states) = netstate_args.split();
    
        let data = self.get_data();

        let result_bit = compare::eq_public_many_multithreads(data, &RingElement(*pub_element), net, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn eq_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();

        let data = self.get_data();

        let result_bit = compare::eq_public_many_binary_multithreads(data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn neq<N: Network>(
        &self, 
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> 
    where
    Standard: Distribution<T>,{ 

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let result_bit = compare::neq_many_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn neq_binary<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;
        let b_chunks = get_task_chunks(&b_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(b_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((a_chunk, b_chunk), &net), state)| {
                move || {
                    let bits = compare::neq_many_binary(a_chunk, b_chunk, net, state).unwrap_or_else(|e|panic!("Column geq error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }
    
    fn neq_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();
    
        let data = self.get_data();

        let result_bit = compare::neq_public_many_multithreads(data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn neq_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();
        
        let a_data = self.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(nets.iter()).zip(states.iter_mut()).map(|((a_chunk, &net), state)| {
                move || {
                    let bits = compare::neq_public_many_binary(a_chunk, &RingElement(*pub_element), net, state).unwrap_or_else(|e|panic!("Column geq error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn gt<N: Network>(
        &self, 
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> 
    where
    Standard: Distribution<T>,{ 

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;
        let b_chunks = get_task_chunks(&b_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(b_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((a_chunk, b_chunk), &net), state)| {
                move || {
                    let bits = compare::gt_many_binary(a_chunk, b_chunk, net, state).unwrap_or_else(|e|panic!("Column geq error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)

    }

    fn gt_binary<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {
        
        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;
        let b_chunks = get_task_chunks(&b_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(b_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((a_chunk, b_chunk), &net), state)| {
                move || {
                    let bits = compare::gt_many(a_chunk, b_chunk, net, state).unwrap_or_else(|e|panic!("Column geq error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
        
    }

    fn gt_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();
    
        let a_data = self.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(nets.iter()).zip(states.iter_mut()).map(|((a_chunk, &net), state)| {
                move || {
                    let bits = compare::gt_public_many(a_chunk, &RingElement(*pub_element), net, state).unwrap_or_else(|e|panic!("Column geq error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn gt_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();
        
        let a_data = self.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(nets.iter()).zip(states.iter_mut()).map(|((a_chunk, &net), state)| {
                move || {
                    let bits = compare::gt_public_many_binary(a_chunk, &RingElement(*pub_element), net, state).unwrap_or_else(|e|panic!("Column geq error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn ge<N: Network>(
        &self, 
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> 
    where
    Standard: Distribution<T>,{ 

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;
        let b_chunks = get_task_chunks(&b_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(b_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((a_chunk, b_chunk), &net), state)| {
                move || {
                    let bits = compare::ge_many(a_chunk, b_chunk, net, state).unwrap_or_else(|e|panic!("Column ge error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn ge_binary<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {
        
        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;
        let b_chunks = get_task_chunks(&b_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(b_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((a_chunk, b_chunk), &net), state)| {
                move || {
                    let bits = compare::ge_many_binary(a_chunk, b_chunk, net, state).unwrap_or_else(|e|panic!("Column ge error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn ge_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();
    
        let a_data = self.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(nets.iter()).zip(states.iter_mut()).map(|((a_chunk, &net), state)| {
                move || {
                    let bits = compare::ge_public_many(a_chunk, &RingElement(*pub_element), net, state).unwrap_or_else(|e|panic!("Column ge public error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn ge_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();    
        
        let a_data = self.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(nets.iter()).zip(states.iter_mut()).map(|((a_chunk, &net), state)| {
                move || {
                    let bits = compare::ge_public_many_binary(a_chunk, &RingElement(*pub_element), net, state).unwrap_or_else(|e|panic!("Column ge public error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn lt<N: Network>(
        &self, 
        other_column: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> 
    where
    Standard: Distribution<T>,{ 

        assert_eq!(self.len(), other_column.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let other_data = other_column.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;
        let other_chunks = get_task_chunks(&other_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(other_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((a_chunk, other_chunk), &net), state)| {
                move || {
                    let bits = compare::lt_many(a_chunk, other_chunk, net, state).unwrap_or_else(|e|panic!("Column lt error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn lt_binary<N: Network>(
        &self,
        other_column: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), other_column.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let other_data = other_column.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;
        let other_chunks = get_task_chunks(&other_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(other_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((a_chunk, b_chunk), &net), state)| {
                move || {
                    let bits = compare::lt_many_binary(a_chunk, b_chunk, net, state).unwrap_or_else(|e|panic!("Column lt error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
        
    }

    fn lt_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(nets.iter()).zip(states.iter_mut()).map(|((a_chunk, &net), state)| {
                move || {
                    let bits = compare::lt_public_many(a_chunk, &RingElement(*pub_element), net, state).unwrap_or_else(|e|panic!("Column lt public error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
        
    }

    fn lt_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(nets.iter()).zip(states.iter_mut()).map(|((a_chunk, &net), state)| {
                move || {
                    let bits = compare::lt_public_many_binary(a_chunk, &RingElement(*pub_element), net, state).unwrap_or_else(|e|panic!("Column lt public error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn le<N: Network>(
        &self,
        other_column: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), other_column.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = other_column.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;
        let b_chunks = get_task_chunks(&b_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(b_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((a_chunk, b_chunk), &net), state)| {
                move || {
                    let bits = compare::le_many(a_chunk, b_chunk, net, state).unwrap_or_else(|e|panic!("Column le error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn le_binary<N: Network>(
        &self,
        other_column: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), other_column.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = other_column.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;
        let b_chunks = get_task_chunks(&b_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(b_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((a_chunk, b_chunk), &net), state)| {
                move || {
                    let bits = compare::le_many_binary(a_chunk, b_chunk, net, state).unwrap_or_else(|e|panic!("Column le error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn le_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(nets.iter()).zip(states.iter_mut()).map(|((a_chunk, &net), state)| {
                move || {
                    let bits = compare::le_public_many(a_chunk, &RingElement(*pub_element), net, state).unwrap_or_else(|e|panic!("Column le public error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn le_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> { 

        let (nets, _state0, _state1, states) = netstate_args.split();

        let a_data = self.get_data();

        let a_chunks = get_task_chunks(&a_data, self.len(), nets.len())?;

        let result_bit = net::join_all(
        a_chunks.into_iter().zip(nets.iter()).zip(states.iter_mut()).map(|((a_chunk, &net), state)| {
                move || {
                    let bits = compare::le_public_many_binary(a_chunk, &RingElement(*pub_element), net, state).unwrap_or_else(|e|panic!("Column le public error: {}",e));
                    bits
                }
            })
        );

        let result_bit = result_bit.concat();

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn in_public_binary<N: net::Network>(
        &self,
        pub_elements: &Vec<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert!(!pub_elements.is_empty(), "Public elements cannot be empty");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let c_data = self.get_data();
        let mut eq_vec = Vec::new();
        
        for pub_element in pub_elements {
            let eq_bit = compare::eq_public_many_binary_multithreads(c_data, &RingElement(*pub_element), nets, states)?;
            let eq = transform::from_bit_to_t_drop(eq_bit)?;
            eq_vec.push(eq);
        }

        let mut res = eq_vec[0].clone();
        
        for i in 1..pub_elements.len() {
            res = compare::or_vec_multithreads(&res, &eq_vec[i], nets, states)?;
        }

        res = transform::b2a_many_multithreads(&res, nets, states)?;

        let result_column = ShareColumn::new(res, ShareType::Binary, self.get_name().to_string());
        
        Ok(result_column)
    }

    fn and<N: Network>(
        &self,
        other_column: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), other_column.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let result = compare::and_vec_multithreads(self.get_data(), other_column.get_data(), nets, states)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn or<N: Network>(
        &self,
        other_column: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), other_column.len(), "Columns must have the same length");

        let (nets, _state0, _state1, states) = netstate_args.split();

        let result = compare::or_vec_multithreads(self.get_data(), other_column.get_data(), nets, states)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

}