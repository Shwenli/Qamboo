use primitives::utils::open_vec_multinet;
use protocols::protocols::rep3_ring::Rep3RingShare;
use algebra::ring::{int_ring::IntRing2k};
use net::Network;
use rand::distributions::Standard;
use rand::distributions::Distribution;
use crate::share_column::{ShareColumn, ShareType};
use crate::share_table::ShareTable;
use crate::table_operator::Open;
use crate::NetStateArgs;


impl<T: IntRing2k> Open<Rep3RingShare<T>, T> for ShareTable<Rep3RingShare<T>> 
where
    Standard: Distribution<T>,{ 
    fn open<N: Network>(
        &mut self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareTable<T>> {

        let nets = netstate_args.nets;

        let mut opened_table = ShareTable::<T>::new();

        for (col_name, share_col) in self.schema.iter_mut() {
            let opened_col_data = open_vec_multinet(share_col.get_data(), nets)?;
            let opened_col_data = opened_col_data.into_iter().map(|x| x.0).collect::<Vec<_>>();
            let opened_col = ShareColumn::new(opened_col_data, ShareType::PlainText, col_name.clone());
            opened_table.insert_column(opened_col.get_name().to_string(), opened_col);
            
        }

        Ok(opened_table)
    }
}