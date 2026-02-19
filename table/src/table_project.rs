
use crate::share_table::ShareTable;
use crate::table_operator::Project;
use algebra::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::{Rep3RingShare};

impl<T: IntRing2k> Project<Rep3RingShare<T>> for ShareTable<Rep3RingShare<T>>{

    fn project(&self, col_names: Vec<&str>) -> eyre::Result<ShareTable<Rep3RingShare<T>>> {

        let mut new_table = ShareTable::<Rep3RingShare<T>>::new();
        new_table.key_name = self.key_name.clone();
        
        for col_name in col_names {
            let col = &self[col_name];
            new_table.insert_column(col_name.to_string(), col.clone());
        }

        Ok(new_table)
    }

}