use crate::share_table::ShareTable;
use crate::share_column::{ShareColumn, ShareType};
use crate::table_operator::{GroupBySinge, Groupby, AggFunc};
use crate::NetStateArgs;
use operator::{group_by, agg_func};
use itertools::izip;
use random::rep3::Rep3State;
use protocols::rep3_ring::Rep3RingShare;
use algebra::ring::{bit::Bit, int_ring::IntRing2k};
use net::Network;
use primitives::permute::apply_inv_multithreads;
use rand::distributions::Standard;
use rand::prelude::Distribution;

impl<T: IntRing2k> GroupBySinge<Rep3RingShare<T>> for ShareTable<Rep3RingShare<T>>
where
    Standard: Distribution<T>,{
    fn group_by_single<N: Network>(
        &mut self,
        group_key_names: Vec<&str>,
        net: &N,
        state: &mut Rep3State,
    ) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<u32>>)> {
        
        let group_keys = izip!(&group_key_names).map(|name| self[*name].get_data()).collect::<Vec<_>>();
        let vals: Vec<_> = self.schema.values()
                .filter(|c| !group_key_names.contains(&c.get_name()) && c.get_name() != "valid")
                .map(|c| c.get_data())
                .collect::<Vec<_>>();

        let valid = self["valid"].get_data();

        let (_, v_g_vec, e_t_res, _, perm_e, k_out_vec, _, new_valid) =
            group_by::table_group_by_common(
                group_keys,
                vals,
                valid,
                true,
                64,
                net,
                state,
            )?;

        let k_g_iter = k_out_vec.into_iter();
        let mut v_g_iter = v_g_vec.into_iter();

        for (name, data) in group_key_names.iter().zip(k_g_iter) {
            self[*name].update_data(data);
        }

        for col in self.schema.values_mut()
            .filter(|c| !group_key_names.contains(&c.get_name()) && c.get_name() != "valid") {
            
            if let Some(data) = v_g_iter.next() {
                col.update_data(data);
            }
        }

        self["valid"].update_data(new_valid);

        Ok((e_t_res, perm_e))
    }

    fn agg_sum_single<N: Network>(
        &mut self,
        to_agg_name: &str,
        new_agg_name: &str,
        e: &[Rep3RingShare<T>],
        perm: &[Rep3RingShare<u32>],
        net: &N,
        state: &mut Rep3State,
    ) -> eyre::Result<()> {
        
        let v_g = self[to_agg_name].get_data();

        let agg_res = agg_func::table_agg_sum(
            v_g,
            e,
            perm,
            net,
            state,
        )?;

        let agg_column = ShareColumn::new(agg_res, ShareType::Arithmetic, new_agg_name.to_string());

        self.insert_column(new_agg_name.to_string(), agg_column);

        Ok(())
    }

    fn agg_count_single<N: Network>(
        &mut self,
        to_agg_name: &str,
        new_agg_name: &str,
        e: &[Rep3RingShare<T>],
        perm: &[Rep3RingShare<u32>],
        net: &N,
        state: &mut Rep3State,
    ) -> eyre::Result<()> {
        
        let v_g = self[to_agg_name].get_data();

        let agg_res = agg_func::table_agg_count(
            v_g,
            e,
            perm,
            net,
            state,
        )?;

        let agg_column = ShareColumn::new(agg_res, ShareType::Arithmetic, new_agg_name.to_string());

        self.insert_column(new_agg_name.to_string(), agg_column);

        //option : if need
        //self.delete_column(to_agg_name);

        Ok(())
    }

    
}


impl<T:IntRing2k> Groupby<Rep3RingShare<T>> for ShareTable<Rep3RingShare<T>>
where
    Standard: Distribution<T>,{

    fn group_by<N: Network>(
        &mut self,
        group_key_names: Vec<&str>,
        netstate_args: &mut NetStateArgs<N>
    ) -> eyre::Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<u32>>, Vec<Rep3RingShare<Bit>>)> {

        let (nets,  states) = netstate_args.split();

        let group_keys = izip!(&group_key_names).map(|name| self[*name].get_data()).collect::<Vec<_>>();
        let vals: Vec<_> = self.schema.values()
                .filter(|c| !group_key_names.contains(&c.get_name()) && c.get_name() != "valid")
                .map(|c| c.get_data())
                .collect::<Vec<_>>();

        let valid = self["valid"].get_data();

        let (_, _v_g, e, e_bit, _, perm_e, k_out, v_out, _old_valid, new_valid) =
            group_by::table_group_by_common_multithreads(
                group_keys,
                vals,
                valid,
                true,
                64,
                nets,
                states,
            )?;

        let k_out_iter = k_out.into_iter();
        let mut v_out_iter = v_out.into_iter();


        for (name, data) in group_key_names.iter().zip(k_out_iter) {
            self[*name].update_data(data);
        }

        for col in self.schema.values_mut()
        .filter(|c| !group_key_names.contains(&c.get_name()) && c.get_name() != "valid") {
            
            if let Some(data) = v_out_iter.next() {
                col.update_data(data);
            }
        }

        self["valid"].update_data(new_valid);

        Ok((e, perm_e, e_bit))
    }

    fn group_by_retain_valid<N: Network>(
        &mut self,
        group_key_names: Vec<&str>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<(Vec<Rep3RingShare<T>>,Vec<Rep3RingShare<u32>>, Vec<Rep3RingShare<Bit>>, Vec<Rep3RingShare<T>>)> {

        let (nets,  states) = netstate_args.split();

        let group_keys = izip!(&group_key_names).map(|name| self[*name].get_data()).collect::<Vec<_>>();
        let vals: Vec<_> = self.schema.values()
                .filter(|c| !group_key_names.contains(&c.get_name()) && c.get_name() != "valid")
                .map(|c| c.get_data())
                .collect::<Vec<_>>();

        let valid = self["valid"].get_data();

        let (_, v_g_vec, e_t_res, e_bit, _, perm_e, k_out_vec, _, old_valid, new_valid) =
            group_by::table_group_by_common_multithreads(
                group_keys,
                vals,
                valid,
                true,
                64,
                nets,
                states,
            )?;

        let k_g_iter = k_out_vec.into_iter();
        let mut v_g_iter = v_g_vec.into_iter();


        for (name, data) in group_key_names.iter().zip(k_g_iter) {
            self[*name].update_data(data);
        }

        for col in self.schema.values_mut()
        .filter(|c| !group_key_names.contains(&c.get_name()) && c.get_name() != "valid") {
            
            if let Some(data) = v_g_iter.next() {
                col.update_data(data);
            }
        }

        self["valid"].update_data(new_valid);

        Ok((e_t_res, perm_e, e_bit, old_valid))
    }

    
}

impl<T:IntRing2k> AggFunc<Rep3RingShare<T>> for ShareTable<Rep3RingShare<T>>
where
    Standard: Distribution<T>,{
    
    /// ccs23 agg_count version
    fn agg_count<N: Network>(
        &mut self,
        new_agg_name: &str,
        e: &[Rep3RingShare<T>],
        perm: &[Rep3RingShare<u32>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        let (nets, states) = netstate_args.split();

        let agg_res = agg_func::table_agg_count_multithreads(
            e,
            perm,
            nets,
            states,
        )?;

        let agg_column = ShareColumn::new(agg_res, ShareType::Arithmetic, new_agg_name.to_string());

        self.insert_column(new_agg_name.to_string(), agg_column);

        //option : if need
        //self.delete_column(to_agg_name);

        Ok(())
    }

    fn agg_count_by_valid<N: Network>(
        &mut self,
        new_agg_name: &str,
        old_valid: &[Rep3RingShare<T>],
        perm: &[Rep3RingShare<u32>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {
        
        let (nets, states) = netstate_args.split();

        let agg_res = agg_func::table_agg_count_by_valid_multithreads(
            old_valid,
            perm,
            nets,
            states,
        )?;

        let agg_column = ShareColumn::new(agg_res, ShareType::Arithmetic, new_agg_name.to_string());

        self.insert_column(new_agg_name.to_string(), agg_column);

        Ok(())
        
    }
    
    fn agg_sum<N: Network>(
        &mut self,
        to_agg_name: &str,
        new_agg_name: &str,
        e: &[Rep3RingShare<T>],
        perm: &[Rep3RingShare<u32>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        let (nets, states) = netstate_args.split();
        
        //* get v_g from v_out by applying inverse permutation. 
        let v_out = self[to_agg_name].get_data();
            let v_g = apply_inv_multithreads(perm, v_out, nets, states)?;

        let agg_res = agg_func::table_agg_sum_multithreads(
            &v_g,
            e,
            perm,
            nets,
            states,
        )?;

        let agg_column = ShareColumn::new(agg_res, ShareType::Arithmetic, new_agg_name.to_string());

        self.insert_column(new_agg_name.to_string(), agg_column);

        Ok(())
    }

    fn agg_max<N: Network>(
            &mut self,
            to_agg_name: &str,
            new_agg_name: &str,
            e: &[Rep3RingShare<T>],
            perm: &[Rep3RingShare<u32>],
            netstate_args: &mut NetStateArgs<N>,
        ) -> eyre::Result<()> {

        let (nets, states) = netstate_args.split();

        let v_out = self[to_agg_name].get_data();
        let v_g = apply_inv_multithreads(perm, v_out, nets, states)?;

        let agg_res = agg_func::table_agg_max_multithreads(
            &v_g,
            e,
            perm,
            nets,
            states,
        )?;

        let agg_column = ShareColumn::new(agg_res, ShareType::Arithmetic, new_agg_name.to_string());
        
        self.insert_column(new_agg_name.to_string(), agg_column);
        
        Ok(())
    }

    fn agg_min<N: Network>(
        &mut self,
        to_agg_name: &str,
        new_agg_name: &str,
        e: &[Rep3RingShare<T>],
        e_bit: &[Rep3RingShare<Bit>],
        perm: &[Rep3RingShare<u32>],
        netstate_args: &mut NetStateArgs<N>,
    )-> eyre::Result<()>{
        
        let (nets, states) = netstate_args.split();

        let v_out = self[to_agg_name].get_data();
        let v_g = apply_inv_multithreads(perm, v_out, nets, states)?;

        let agg_res = agg_func::table_agg_min_multithreads(
            &v_g,
            e,
            e_bit,
            nets,
            states,
        )?;

        let agg_column = ShareColumn::new(agg_res, ShareType::Arithmetic, new_agg_name.to_string());

        self.insert_column(new_agg_name.to_string(), agg_column);

        Ok(())
    }
}