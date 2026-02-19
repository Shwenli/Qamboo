
use rand::distributions::Standard;
use rand::prelude::Distribution;
use net::Network;
use primitives::transform::*;
use primitives::mul::mul_share_vec;
use primitives::compare::{self,and_vec_multithreads};
use algebra::ring::{int_ring::IntRing2k, ring_impl::RingElement};
use protocols::protocols::rep3_ring::Rep3RingShare;
use crate::share_table::ShareTable;
use crate::table_operator::Filter;
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

        let (nets, states) = netstate_args.split();
     
        let filter_column_data = self[filter_column].get_data();

        // Transform filter_value to RingElement<T>
        let filter_value_elem = RingElement::from(*filter_value);

        let valid_data = self["valid"].get_data();
        let valid_data = a2b_many_multithreads(valid_data, nets, states)?;

        let mask_bits = predicate.apply_public(
            filter_column_data,
            &filter_value_elem,
            nets,
            states,
        )?;

        let mask_t:Vec<Rep3RingShare<T>> = from_bit_to_t(&mask_bits)?;
        let updated_valid = and_vec_multithreads(&valid_data, &mask_t, nets, states)?;

        let result_valid = b2a_many_multithreads(&updated_valid, nets, states)?;

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

        let (nets, states) = netstate_args.split();

        let lhs_data =  self[lhs_column].get_data();
        let rhs_data =  self[rhs_column].get_data();
        let lhs_len = lhs_data.len();
        let rhs_len = rhs_data.len();
        
        if lhs_len != rhs_len {
            eyre::bail!("LHS and RHS columns must have the same length");
        }

        let valid_data = self["valid"].get_data();
        let valid_data = a2b_many_multithreads(&valid_data, nets, states)?;
        
        let mask_bits = predicate.apply_shared(
            &lhs_data,
            &rhs_data,
            nets,
            states,
        )?;

        let mask_t:Vec<Rep3RingShare<T>> = from_bit_to_t(&mask_bits)?;

        let updated_valid = and_vec_multithreads(&valid_data, &mask_t, nets, states)?;

        let result_valid = b2a_many_multithreads(&updated_valid, nets, states)?;

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

        let (nets, states) = netstate_args.split();

        let filter_column_data = self[filter_column_name].get_data();
        let mut eq_vec = Vec::new();

        if predicate == Predicate::EqualBinary {
                for f_v in filter_values {
                let eq_bit = compare::eq_public_many_binary_multithreads(filter_column_data, &RingElement(*f_v), nets, states)?;
                let eq = from_bit_to_t_drop(eq_bit)?;
                eq_vec.push(eq);
            }
        }
        else if predicate == Predicate::Equal{
            for f_v in filter_values {
                let eq_bit = compare::eq_public_many_multithreads(filter_column_data, &RingElement(*f_v), nets, states)?;
                let eq = from_bit_to_t_drop(eq_bit)?;
                eq_vec.push(eq);
            }

        }

        let mut ans = eq_vec[0].clone();

        for i in 1..eq_vec.len() {
            ans = compare::or_vec_multithreads(&ans, &eq_vec[i], nets, states)?;
        }

        ans = b2a_many_multithreads(&ans, nets, states)?;

        let valid_data = self["valid"].get_data();

        let update_valid = mul_share_vec(&ans, &valid_data, nets, states)?;
        
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

        let (nets, states) = netstate_args.split();

        let filter_column_data = self[filter_column_name].get_data();
        let valid_data = self["valid"].get_data();

        let mask_bits = predicate.apply_shared(
            &filter_column_data,
            filter_values,
            nets,
            states,
        )?;

        let mask_t:Vec<Rep3RingShare<T>> = from_bit_to_t(&mask_bits)?;
        let updated_valid = and_vec_multithreads(&valid_data, &mask_t, nets, states)?;

        let result_valid = b2a_many_multithreads(&updated_valid, nets, states)?;

        self["valid"].update_data(result_valid);

        Ok(())
    }

    fn filter_directed_by_bool<N: Network>(
        &mut self,
        composed_bool_values: &[Rep3RingShare<T>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        let (nets, states) = netstate_args.split();

        let valid_data = self["valid"].get_data();
        let valid_data = a2b_many_multithreads(&valid_data, nets, states)?;

        let mut result_valid = and_vec_multithreads(&valid_data, composed_bool_values, nets, states)?;

        result_valid = b2a_many_multithreads(&result_valid, nets, states)?;

        self["valid"].update_data(result_valid);

        Ok(())
    }


}