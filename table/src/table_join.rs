
use crate::share_table::ShareTable;
use crate::share_column::{ShareColumn, ShareType};
use crate::table_operator::Join;
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::Rep3RingShare;
use net::Network;
use operator::join::{anti_join_table_multithreads, inner_join_table_multi_keys_multithreads, inner_join_table_multithreads,semi_join_table_multithreads};
use rand::distributions::Standard;
use rand::prelude::Distribution;
use crate::NetStateArgs;


impl<T: IntRing2k> Join<Rep3RingShare<T>> for ShareTable<Rep3RingShare<T>>
where
    Standard: Distribution<T>,{
    
    fn inner_join<N: Network>(
        &self,
        k_l_name: &str,
        k_r_name: &str,
        r: &ShareTable<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareTable<Rep3RingShare<T>>>{

        let sort_bitsize = T::K;

        let k_l = self[k_l_name].get_data().to_vec();
        let k_r = r[k_r_name].get_data().to_vec();

        let valid_l = self["valid"].get_data().to_vec();
        let valid_r = r["valid"].get_data().to_vec();
        
        let valid_name = "valid";

        let (nets, state0, state1, states) = netstate_args.split();

        //Include three parts, right table join key, left table value columns, 
        //right table value columns, and finally the valid column
        let mut result_table = ShareTable::<Rep3RingShare<T>>::new();

        let key_column = ShareColumn::<Rep3RingShare<T>>::new(
            Vec::new(),
            ShareType::Arithmetic,
            k_r_name.to_string(),
        );
        result_table.insert_column(key_column.get_name().to_string(), key_column);

        let mut l_vec = Vec::new();
        let mut r_vec = Vec::new();
        
        for col in self.schema.values() {

            if col.get_name() == k_l_name || col.get_name() == valid_name {
                continue;
            }

            l_vec.push(col.get_data());

            let new_col = ShareColumn::<Rep3RingShare<T>>::new(
                Vec::new(),
                ShareType::Arithmetic,
                col.get_name().to_string(),
            );

            result_table.insert_column(new_col.get_name().to_string(), new_col);
        }

        for col in r.schema.values() {

            if col.get_name() == k_r_name || col.get_name() == valid_name {
                continue;
            }
            r_vec.push(col.get_data());

            let new_col = ShareColumn::<Rep3RingShare<T>>::new(
                Vec::new(),
                ShareType::Arithmetic,
                col.get_name().to_string(),
            );

            result_table.insert_column(new_col.get_name().to_string(), new_col);
        }

        let valid_column = ShareColumn::<Rep3RingShare<T>>::new(
            Vec::new(),
            ShareType::Arithmetic,
            "valid".to_string(),
        );
        result_table.insert_column(valid_column.get_name().to_string(), valid_column);

        let result_vec = inner_join_table_multithreads(
            k_l,
            k_r,
            l_vec,
            r_vec,
            valid_l,
            valid_r,
            sort_bitsize,
            nets,
            state0,
            state1,
            states,
        )?;

        for (i, data_i) in result_vec.into_iter().enumerate() {
            let col = result_table.get_column_by_index_mut(i);
            col.update_data(data_i);
        }

        // Result table does not keep left table join key, only right table join key is kept
        Ok(result_table)
    }


    fn inner_join_multi_keys<N: Network>(
        &self,
        k_l_name: Vec<&str>,
        k_r_name: Vec<&str>,
        table_r: &ShareTable<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareTable<Rep3RingShare<T>>> {

        let sort_bitsize = T::K;

        let mut k_l = Vec::new();
        let mut k_r = Vec::new();

        for key in k_l_name.iter() {
            k_l.push(self[*key].get_data().to_vec());
        }

        for key in k_r_name.iter() {
            k_r.push(table_r[*key].get_data().to_vec());
        }

        let valid_l = self["valid"].get_data().to_vec();
        let valid_r = table_r["valid"].get_data().to_vec();

        let (nets, state0, state1, states) = netstate_args.split();


        let mut result_table = ShareTable::<Rep3RingShare<T>>::new();

        for key in k_r_name.iter() {
            let key_column = ShareColumn::<Rep3RingShare<T>>::new(
                Vec::new(),
                ShareType::Arithmetic,
                key.to_string(),
            );
            result_table.insert_column(key_column.get_name().to_string(), key_column);
        }

        let mut l_vec = Vec::new();
        let mut r_vec = Vec::new();
        //fixed:: 应该为l_vec的schema
        for col in self.schema.values() {

            if k_l_name.contains(&col.get_name()) || col.get_name() == "valid" {
                continue;
            }

            l_vec.push(col.get_data());

            let new_col = ShareColumn::<Rep3RingShare<T>>::new(
                Vec::new(),
                ShareType::Arithmetic,
                col.get_name().to_string(),
            );

            result_table.insert_column(new_col.get_name().to_string(), new_col);
        }

        for col in table_r.schema.values() {
            //eprintln!("r:{}", col.get_name());

            if k_r_name.contains(&col.get_name()) || col.get_name() == "valid" {
                continue;
            }
            r_vec.push(col.get_data());

            let new_col = ShareColumn::<Rep3RingShare<T>>::new(
                Vec::new(),
                ShareType::Arithmetic,
                col.get_name().to_string(),
            );

            result_table.insert_column(new_col.get_name().to_string(), new_col);
        }

        let valid_column = ShareColumn::<Rep3RingShare<T>>::new(
            Vec::new(),
            ShareType::Arithmetic,
            "valid".to_string(),
        );
        result_table.insert_column(valid_column.get_name().to_string(), valid_column);

        
        let result_vec = inner_join_table_multi_keys_multithreads(
            k_l,
            k_r,
            l_vec,
            r_vec,
            valid_l,
            valid_r,
            sort_bitsize,
            nets,
            state0,
            state1,
            states,
        )?;

        for (i, data_i) in result_vec.into_iter().enumerate() {
            let col = result_table.get_column_by_index_mut(i);
            // 将整列数据追加（移动 data_i 的元素）
            col.update_data(data_i);
        }

        Ok(result_table)
        
    }

    fn semi_join<N: Network>(
        &mut self,
        k_l_name: &str,
        k_r_name: &str,
        table_r: &ShareTable<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {
        
        let sort_bitsize = T::K;
        
        let k_l = self[k_l_name].get_data().to_vec();
        let k_r = table_r[k_r_name].get_data().to_vec();

        let valid_l = self["valid"].get_data().to_vec();
        let valid_r = table_r["valid"].get_data().to_vec();

        let (nets, state0, state1, states) = netstate_args.split();
        
        let new_valid = semi_join_table_multithreads(
            k_l,
            k_r,
            valid_l,
            valid_r,
            sort_bitsize,
            nets,
            state0,
            state1,
            states,
        )?;

        self["valid"].update_data(new_valid);

        Ok(())
            
    }

    fn anti_join<N: Network>(
        &mut self,
        k_l_name: &str,
        k_r_name: &str,
        table_r: &ShareTable<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        let sort_bitsize = T::K;

        let k_l = self[k_l_name].get_data().to_vec();
        let k_r = table_r[k_r_name].get_data().to_vec();

        let valid_l = self["valid"].get_data().to_vec();
        let valid_r = table_r["valid"].get_data().to_vec();

        let (nets, state0, state1, states) = netstate_args.split();
        
        let new_valid = anti_join_table_multithreads(
            k_l,
            k_r,
            valid_l,
            valid_r,
            sort_bitsize,
            nets,
            state0,
            state1,
            states,
        )?;

        self["valid"].update_data(new_valid);

        Ok(())

    }
    
}



