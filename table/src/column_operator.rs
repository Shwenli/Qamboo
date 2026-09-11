pub mod column_basic_compute;
pub mod column_operator_impl;

use net::Network;
use algebra::ring::bit::Bit;
use protocols::rep3_ring::Rep3RingShare;
use crate::{NetStateArgs, share_column::ShareColumn};


pub trait PrefixSum<T>{

    fn prefix_sum(&self) -> T;

}

pub trait Distinct<T>{

    fn distinct<N: Network>(
        &self,
        valid: &[T],
        e: &[T],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<Vec<T>>;

}

pub trait TransformBetweenArithAndBinary<T>{

    fn from_arithmetic_to_binary<N: Network>(
        &mut self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    fn from_binary_to_arithmetic<N: Network>(
        &mut self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()>;

    fn add_new_col_from_arithmetic_to_binary<N: Network>(
        &self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn add_new_col_from_binary_to_arithmetic<N: Network>(
        &self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

}

pub trait ColumnBooleanOperator<T,U>{

    /// _binary means the input is binary shares and does not require transform from arithmetic to binary
    fn equal<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn eq_binary<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    /// Compare two columns and return a binary T share result
    fn eq_public<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn eq_public_binary<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn neq<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn neq_binary<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn neq_public<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn neq_public_binary<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn gt<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn gt_binary<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn gt_public<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn gt_public_binary<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn ge<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn ge_binary<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn ge_public<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn ge_public_binary<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn lt<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn lt_binary<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn lt_public<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn lt_public_binary<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn le<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn le_binary<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn le_public<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn le_public_binary<N: Network>(
        &self,
        pub_element: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    /// ORQ-style less-than-zero: local sign-bit (MSB) extraction on a *binary* shared column.
    /// Returns a Bit column of 1s where the plaintext, interpreted in two's complement,
    /// is negative. Requires no communication.
    /// Typically applied to a materialized difference column c = a - b to obtain a < b for free.
    fn ltz_bit(&self) -> eyre::Result<ShareColumn<Rep3RingShare<Bit>>>;
    fn ltz(&self) -> eyre::Result<ShareColumn<T>>;

    /// Compare two columns and return a binary T share result
    fn in_public_binary<N: Network>(
        &self,
        pub_elements: &Vec<U>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn and<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;

    fn or<N: Network>(
        &self,
        other_column: &ShareColumn<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<T>>;
}