pub mod table_filter;
pub mod table_group_by;
pub mod table_join;
pub mod table_utils;
pub mod table_order_by;
pub mod table_project;


use random::rep3::Rep3State;
use protocols::rep3_ring::Rep3RingShare;
use algebra::ring::bit::Bit;
use net::Network;
use crate::predicate::Predicate;
use crate::share_table::ShareTable;
use crate::NetStateArgs;



pub trait Open<T,U>{

    fn open<N: Network>(
        &mut self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareTable<U>>;
}

pub trait OrderBySingle<T> {

    fn order_by_single<N: Network>(
        &mut self,
        key_column: &str,
        order: bool,
        bitsize: usize,
        net: &N,
        state: &mut Rep3State,
    ) -> eyre::Result<()>;
}

pub trait OrderBy<T> {
    fn order_by<N: Network>(
        &mut self,
        key_name: &str,
        order: bool,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;
}

pub trait Filter<T,U>{

    // Filter by comparison of a column of shared values with a public value.
    fn filter_public<N: Network>(
        &mut self,
        filter_column: &str,
        predicate: Predicate,
        filter_value: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    
    // Filter by comparison of two columns of shared values. 
    fn filter_shared<N: Network>(
        &mut self,
        lhs_column: &str,
        rhs_column: &str,
        predicate: Predicate,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    
    fn filter_in<N: Network>(
        &mut self,
        filter_column_name: &str,
        filter_values: &Vec<U>,
        predicate: Predicate,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    fn filter_shared_with_any_column<N: Network>(
        &mut self,
        filter_column_name: &str,
        filter_values: &[T],
        predicate: Predicate,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    /// filter by bool values composed of multiple comparisons on column 
    fn filter_directed_by_bool<N: Network>(
        &mut self,
        composed_bool_values: &[T],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

}


pub trait GroupBySinge<T>{

    fn group_by_single<N: Network>(
        &mut self,
        group_key: Vec<&str>,
        net: &N,
        state: &mut Rep3State,
    ) -> eyre::Result<(Vec<T>,Vec<Rep3RingShare<u32>>)>;


    fn agg_sum_single<N: Network>(
        &mut self,
        to_agg_name: &str,
        new_agg_name: &str,
        e: &[T],
        perm: &[Rep3RingShare<u32>],
        net: &N,
        state: &mut Rep3State,
    ) -> eyre::Result<()>;


    fn agg_count_single<N: Network>(
        &mut self,
        to_agg_name: &str,
        new_agg_name: &str,
        e: &[T],
        perm: &[Rep3RingShare<u32>],
        net: &N,
        state: &mut Rep3State,
    ) -> eyre::Result<()>;

}


pub trait Groupby<T>{

    fn group_by<N: Network>(
        &mut self,
        group_keys: Vec<&str>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<(Vec<T>,Vec<Rep3RingShare<u32>>, Vec<Rep3RingShare<Bit>>)>;

    fn group_by_retain_valid<N: Network>(
        &mut self,
        group_keys: Vec<&str>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<(Vec<T>,Vec<Rep3RingShare<u32>>, Vec<Rep3RingShare<Bit>>, Vec<T>)>;
}

pub trait AggFunc<T>{

    fn agg_count<N: Network>(
        &mut self,
        new_agg_name: &str,
        e: &[T],
        perm: &[Rep3RingShare<u32>],
       netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    /// specifically for distinct count
    fn agg_count_by_valid<N: Network>(
        &mut self,
        new_agg_name: &str,
        e: &[T],
        perm: &[Rep3RingShare<u32>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    fn agg_sum<N: Network>(
        &mut self,
        to_agg_name: &str,
        new_agg_name: &str,
        e: &[T],
        perm: &[Rep3RingShare<u32>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    fn agg_max<N: Network>(
        &mut self,
        to_agg_name: &str,
        new_agg_name: &str,
        e: &[T],
        perm: &[Rep3RingShare<u32>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    fn agg_min<N: Network>(
        &mut self,
        to_agg_name: &str,
        new_agg_name: &str,
        e: &[T],
        e_bit: &[Rep3RingShare<Bit>],
        perm: &[Rep3RingShare<u32>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

}


pub trait Join<T>{

    fn inner_join<N: Network>(
        &self,
        k_l_name: &str,
        k_r_name: &str,
        table_r: &ShareTable<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareTable<T>>;

    fn inner_join_with_multi_keys<N: Network>(
        &self,
        k_l_name: Vec<&str>,
        k_r_name: Vec<&str>,
        table_r: &ShareTable<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareTable<T>>;

    fn semi_join<N: Network>(
        &mut self,
        k_l_name: &str,
        k_r_name: &str,
        table_r: &ShareTable<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    fn anti_join<N: Network>(
        &mut self,
        k_l_name: &str,
        k_r_name: &str,
        table_r: &ShareTable<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;
}


pub trait Project<T> {

    fn project(
        &self,
        col_names: Vec<&str>,
    ) -> eyre::Result<ShareTable<T>>;
    
} 