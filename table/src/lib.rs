pub mod share_column;
pub mod share_table;
pub mod column_basic_compute;
pub mod predicate;
pub mod table_operator;
pub mod table_join;
pub mod table_group_by;
pub mod table_filter;
pub mod table_order_by;
pub mod table_project;
pub mod table_utils;
pub mod column_operator;
pub mod column_operator_impl;



use random::rep3::Rep3State;
use net::Network;

pub struct NetStateArgs<'a, N: Network> {
    pub nets: &'a [&'a N],
    pub states: &'a mut [&'a mut Rep3State],
}

impl<'a, N: Network> NetStateArgs<'a, N> {
    pub fn new(
        nets: &'a [&'a N],
        states: &'a mut [&'a mut Rep3State],
    ) -> Self {
        Self {
            nets,
            states,
        }
    }

    pub fn split(
        &mut self,
    ) -> (
        &'a [&'a N],
        &mut [&'a mut Rep3State],
    ) {
        (self.nets, self.states)
    }

    pub fn unpack(
        self,
    ) -> (
        &'a [&'a N],
        &'a mut [&'a mut Rep3State],
    ) {
        (self.nets, self.states)
    }
}