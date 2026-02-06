use net::Network;

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

}

pub trait ColumnBooleanOperator<T,U>{

    /// _binary means the input is binary shares and does not require transform from arithmetic to binary
    fn eq<N: Network>(
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