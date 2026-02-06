
use rand::distributions::Standard;
use rand::prelude::Distribution;
use protocols::protocols::rep3_ring::Rep3State;
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::{Rep3RingShare};
use net::Network;
use operator::sort;
use crate::share_table::ShareTable;
use crate::table_operator::{OrderBy, OrderBySingle};
use crate::NetStateArgs;


impl<T: IntRing2k> OrderBySingle<Rep3RingShare<T>> for ShareTable<Rep3RingShare<T>>
where
    Standard: Distribution<T>,
{
    fn order_by_single<N: Network>(
        &mut self,
        key_column: &str,
        order: bool,
        bitsize: usize,
        net: &N,
        state: &mut Rep3State,
    ) -> eyre::Result<()> {
        
        assert!(self.num_rows() > 0, "Table must have at least one row to sort.");

        //copy the key column data
        let key_column_data = self[key_column].get_data().to_vec();

        //prepare mutable slices for all columns
        let mut column_slices: Vec<&mut [Rep3RingShare<T>]> = Vec::with_capacity(self.num_columns());
        
        for col in self.schema.values_mut() {
            column_slices.push(col.get_data_mut());
        }

        sort::radix_sort_by_key_in_place(
            &key_column_data,
            order,
            &mut column_slices,
            bitsize,
            net,
            state,
        )?;

        Ok(())
    }
}


impl<T: IntRing2k> OrderBy<Rep3RingShare<T>> for ShareTable<Rep3RingShare<T>>
where
    Standard: Distribution<T>,{
    fn order_by<N: Network>(
        &mut self,
        key_name: &str,
        order: bool, // true is ascending, false is descending
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        assert!(self.num_rows() > 0, "Table must have at least one row to sort.");
        
        let bitsize = T::K;

        let (nets, state0, state1, states) = netstate_args.split();
        
        //copy the key column data
        let key_column_data = self[key_name].get_data().to_vec();

        //prepare mutable slices for all columns
        let mut column_slices: Vec<&mut [Rep3RingShare<T>]> = Vec::with_capacity(self.num_columns());
        
        for col in self.schema.values_mut() {
            column_slices.push(col.get_data_mut());
        }

        sort::radix_sort_by_key_in_place_multithreads(
            &key_column_data,
            order,
            &mut column_slices,
            bitsize,
            nets,
            state0,
            state1,
            states,
        )?;

        Ok(())
    }

}