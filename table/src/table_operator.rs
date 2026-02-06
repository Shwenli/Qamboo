use protocols::protocols::rep3_ring::Rep3State;
use protocols::protocols::rep3_ring::Rep3RingShare;
use protocols::protocols::rep3_ring::ring::bit::Bit;
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

/// Trait for tables that support order by operations
pub trait OrderBySingle<T> {
    /// 根据指定的列进行排序
    /// 
    /// # Arguments
    /// * `key_column` - 用作排序键的列的引用
    /// * `order` - 排序顺序，每个位对应一个比特位的排序方向
    /// * `bitsize` - 考虑的比特大小
    /// * `net0` - 第一个网络连接
    /// * `net1` - 第二个网络连接
    /// * `state0` - 第一个 Rep3 状态
    /// * `state1` - 第二个 Rep3 状态
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

    /// 根据指定的列和谓词进行过滤（与公开值比较）
    /// 
    /// # Arguments
    /// * `filter_column` - 用作过滤的列引用
    /// * `predicate` - 谓词（比较操作类型）
    /// * `filter_value` - 过滤值
    /// * `valid_column` - 有效性列的名称，用于标记哪些行是有效的
    /// * `nets` - 网络连接切片,支持多线程并行执行
    /// * `states` - Rep3 状态切片,与网络连接一一对应
    /// 
    fn filter_public<N: Network>(
        &mut self,
        filter_column: &str,
        predicate: Predicate,
        filter_value: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    /// 根据两列共享值的比较进行过滤
    /// 
    /// # Arguments
    /// * `lhs_column` - 左侧列引用
    /// * `predicate` - 谓词(比较操作类型)
    /// * `rhs_column` - 右侧列引用
    /// * `valid_column` - 有效性列的名称,用于标记哪些行是有效的
    /// * `nets` - 网络连接切片,支持多线程并行执行
    /// * `states` - Rep3 状态切片,与网络连接一一对应
    /// 
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

    /// 根据指定的列进行分组聚合
    /// 
    /// # Arguments
    /// * `group_column` - 用作分组的列引用
    /// * `agg_column` - 用作聚合的列引用
    /// * `agg_func` - 聚合函数类型（如求和、计数等）
    /// * `nets` - 网络连接切片,支持多线程并行执行
    /// * `states` - Rep3 状态切片,与网络连接一一对应
    /// 
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

    fn inner_join_multi_keys<N: Network>(
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