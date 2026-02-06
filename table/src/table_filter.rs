use crate::share_table::ShareTable;
use crate::table_operator::{Filter};
use protocols::protocols::rep3_ring::arithmetic::local_mul_vec;
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::rep3_ring::{Rep3RingShare, binary};
use net::Network;
use primitives::{transform,compare};
use primitives::transform::{b2a_many_multithreads,a2b_many_multithreads};
use primitives::utils::{get_task_chunks,reshare_vec_multithreads};
use rand::distributions::Standard;
use rand::prelude::Distribution;
use crate::predicate::Predicate;
use crate::NetStateArgs;



impl<T: IntRing2k> Filter<Rep3RingShare<T>,T> for ShareTable<Rep3RingShare<T>>
where
    Standard: Distribution<T>,
{
    fn filter_public<N: Network>(
        &mut self,
        filter_column: &str,
        predicate: Predicate,
        filter_value: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        if self.num_rows() == 0 {
            return Ok(());
        }

        let (nets, _state0, _state1, states) = netstate_args.split();
     
        let filter_column_data = self[filter_column].get_data();
        let filter_len = filter_column_data.len();

        // 将 filter_value 转换为 RingElement<T>
        let filter_value_elem = RingElement::from(*filter_value);

        // 确保 nets 和 states 长度相同
        let valid_data = self["valid"].get_data();
        let valid_data = a2b_many_multithreads(valid_data, nets, states)?;
        //let valid_data_binary = a2b_many_multithreads(valid_data, nets, states)?;

        let mask_chunks = get_task_chunks(filter_column_data, filter_len, nets.len())?;
        let valid_chunks = get_task_chunks(&valid_data, filter_len, nets.len())?;

        let result_valid = net::join_all(
            //这里加上了 valid_chunks
        mask_chunks.into_iter().zip(valid_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut()).map(|(((mask_chunk, valid_chunk), &n), state)| {
                move || {
                    let mask_bits = predicate.apply_public(
                        &mask_chunk,
                        &filter_value_elem,
                        n,
                        state,
                    ).unwrap_or_else(|e|panic!("filter public: apply public error: {}",e));

                    let mask_t:Vec<Rep3RingShare<T>> = transform::from_bit_to_t(&mask_bits).unwrap_or_else(|e|panic!("filter public: trans bit to t error: {}",e));
                    let updated_valid = binary::and_vec(&valid_chunk, &mask_t, n, state).unwrap_or_else(|e|panic!("filter public: and vec error: {}",e));
                    updated_valid
                }
            }),
        );

        let result_valid = result_valid.concat();
        let result_valid = b2a_many_multithreads(&result_valid, nets, states)?;

        self["valid"].update_data(result_valid);

        Ok(())
    }

    fn filter_shared<N: Network>(
        &mut self,
        lhs_column: &str,
        rhs_column: &str,
        predicate: Predicate,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {
        
        if self.num_rows() == 0 {
            return Ok(());
        }

        let (nets, _state0, _state1, states) = netstate_args.split();

        let lhs_data =  self[lhs_column].get_data();
        let rhs_data =  self[rhs_column].get_data();
        let lhs_len = lhs_data.len();
        let rhs_len = rhs_data.len();
        
        // 确保两列长度相同
        if lhs_len != rhs_len {
            eyre::bail!("LHS and RHS columns must have the same length");
        }

        let valid_data = self["valid"].get_data();
        let valid_data = a2b_many_multithreads(&valid_data, nets, states)?;
        //let valid_column = self["valid"].get_data_mut();

        let lhs_chunks = get_task_chunks(lhs_data, lhs_len, nets.len())?;
        let rhs_chunks = get_task_chunks(rhs_data, rhs_len, nets.len())?;
        let valid_chunks = get_task_chunks(&valid_data, valid_data.len(), nets.len())?;

        let result_valid = net::join_all(
        lhs_chunks.into_iter().zip(rhs_chunks.into_iter()).zip(valid_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut())
        .map(|((((lhs_chunk, rhs_chunk), valid_chunk), &n), state)| {
                move || {
                    let mask_bits = predicate.apply_shared(
                        &lhs_chunk,
                        &rhs_chunk,
                        n,
                        state,
                    ).unwrap_or_else(|e|panic!("filter shared: apply shared error: {}",e));

                    let mask_t:Vec<Rep3RingShare<T>> = transform::from_bit_to_t(&mask_bits).unwrap_or_else(|e|panic!("filter shared: trans bit to t error: {}",e));
                    let updated_valid = binary::and_vec(&valid_chunk, &mask_t, n, state).unwrap_or_else(|e|panic!("filter shared: and vec error: {}",e));

                    updated_valid
                }
            }),
        );

        let result_valid = result_valid.concat();
        let result_valid = b2a_many_multithreads(&result_valid, nets, states)?;

        self["valid"].update_data(result_valid);

        Ok(())
    }

    fn filter_in<N: Network>(
        &mut self,
        filter_column_name: &str,
        filter_values: &Vec<T>,
        predicate: Predicate,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        assert!(predicate == Predicate::Equal || predicate == Predicate::EqualBinary);

        let (nets, _state0, _state1, states) = netstate_args.split();

        let filter_column_data = self[filter_column_name].get_data();
        let mut eq_vec = Vec::new();

        if predicate == Predicate::EqualBinary {
                for f_v in filter_values {
                let eq_bit = compare::eq_public_many_binary_multithreads(filter_column_data, &RingElement(*f_v), nets, states)?;
                let eq = transform::from_bit_to_t_drop(eq_bit)?;
                eq_vec.push(eq);
            }
        }
        else if predicate == Predicate::Equal{
            for f_v in filter_values {
                let eq_bit = compare::eq_public_many_multithreads(filter_column_data, &RingElement(*f_v), nets, states)?;
                let eq = transform::from_bit_to_t_drop(eq_bit)?;
                eq_vec.push(eq);
            }

        }

        let mut ans = eq_vec[0].clone();

        for i in 1..eq_vec.len() {
            ans = compare::or_vec_multithreads(&ans, &eq_vec[i], nets, states)?;
        }

        ans = b2a_many_multithreads(&ans, nets, states)?;

        let valid_data = self["valid"].get_data();
        let update_valid = local_mul_vec( &ans, valid_data, states[0]);
        let update_valid = reshare_vec_multithreads(update_valid, nets)?;
        
        self["valid"].update_data(update_valid);

        Ok(())
    }


    fn filter_shared_with_any_column<N: Network>(
        &mut self,
        filter_column_name: &str,
        filter_values: &[Rep3RingShare<T>],
        predicate: Predicate,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        let filter_column_data = self[filter_column_name].get_data();
        let valid_data = self["valid"].get_data();

        let (nets, _state0, _state1, states) = netstate_args.split();

        let lhs_chunks = get_task_chunks(filter_column_data, filter_column_data.len(), nets.len())?;
        let rhs_chunks = get_task_chunks(filter_values, filter_values.len(), nets.len())?;
        let valid_chunks = get_task_chunks(valid_data, valid_data.len(), nets.len())?;

        let result_valid = net::join_all(
        lhs_chunks.into_iter().zip(rhs_chunks.into_iter()).zip(valid_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut())
            .map(|((((lhs_chunk, rhs_chunk), valid_chunk), &n), state)| {
                move || {
                    let mask_bits = predicate.apply_shared(
                        &lhs_chunk, 
                        &rhs_chunk,
                        n, 
                        state,
                    ).unwrap_or_else(|e|panic!("filter shared with any column: apply shared error: {}",e));

                    let mask_t:Vec<Rep3RingShare<T>> = transform::from_bit_to_t(&mask_bits).unwrap_or_else(|e|panic!("filter shared: trans bit to t error: {}",e));
                    let updated_valid = binary::and_vec(&valid_chunk, &mask_t, n, state).unwrap_or_else(|e|panic!("filter shared: and vec error: {}",e));

                    updated_valid
                }
            })
        );
        
        let mut result_valid = result_valid.concat();
        result_valid = b2a_many_multithreads(&result_valid, nets, states)?;

        self["valid"].update_data(result_valid);

        Ok(())
    }

    fn filter_directed_by_bool<N: Network>(
        &mut self,
        composed_bool_values: &[Rep3RingShare<T>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        let (nets, _state0, _state1, states) = netstate_args.split();

        let valid_data = self["valid"].get_data();
        let valid_data = a2b_many_multithreads(&valid_data, nets, states)?;

        let mut result_valid = compare::and_vec_multithreads(&valid_data, composed_bool_values, nets, states)?;

        result_valid = b2a_many_multithreads(&result_valid, nets, states)?;

        self["valid"].update_data(result_valid);

        Ok(())
    }


}