
use rand::distributions::Standard;
use rand::prelude::Distribution;
use protocols::rep3_ring::Rep3RingShare;
use algebra::ring::{int_ring::IntRing2k, ring_impl::RingElement};
use net::Network;
use operator::distinct::distinct_after_groupby_multithreads;
use primitives::compare::*;
use primitives::transform;
use primitives::utils::prefix_sum_sequential;
use crate::share_column::ShareType;
use crate::share_column::ShareColumn;
use crate::column_operator::{ColumnBooleanOperator, Distinct, PrefixSum, TransformBetweenArithAndBinary};
use crate::NetStateArgs;

/// Prefix sum of a column and return the last element as the total sum
impl <T: IntRing2k> PrefixSum<Rep3RingShare<T>> for ShareColumn<Rep3RingShare<T>> 
where
Standard: Distribution<T>,{
    fn prefix_sum(&self) -> Rep3RingShare<T> {
        let data = self.get_data();
        let prefix_sum_vec = prefix_sum_sequential(data).unwrap_or_else(|e| panic!("Prefix sum error: {}", e));

        prefix_sum_vec[data.len()-1]
    }

}

impl<T: IntRing2k> TransformBetweenArithAndBinary<Rep3RingShare<T>> for ShareColumn<Rep3RingShare<T>>
where
Standard: Distribution<T>,{
    fn from_arithmetic_to_binary<N: Network>(
        &mut self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        let (nets, states) = netstate_args.split();

        let data = self.get_data();
        let new_data = transform::a2b_many_multithreads(&data, nets, states)?;
        
        self.update_data(new_data);
        self.update_datatype(ShareType::Binary);

        Ok(())
    }

    fn from_binary_to_arithmetic<N: Network>(
        &mut self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<()> {

        let (nets,  states) = netstate_args.split();

        let data = self.get_data();
        let new_data = transform::b2a_many_multithreads(&data, nets, states)?;

        self.update_data(new_data);
        self.update_datatype(ShareType::Arithmetic);

        Ok(())
    }

    fn add_new_col_from_arithmetic_to_binary<N: Network>(
        &self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {
        let (nets, states) = netstate_args.split();

        let data = self.get_data();
        let new_data = transform::a2b_many_multithreads(&data, nets, states)?;

        let new_col_name = format!("[{}]", self.get_name());

        let new_col = ShareColumn::new(new_data, ShareType::Binary, new_col_name);

        Ok(new_col)
    }

    fn add_new_col_from_binary_to_arithmetic<N: Network>(
        &self,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();

        let data = self.get_data();
        let new_data = transform::b2a_many_multithreads(&data, nets, states)?;

        let name = self.get_name();
        let new_col_name = name.strip_prefix('[').and_then(|s| s.strip_suffix(']')).unwrap_or(name).to_string();

        let new_col = ShareColumn::new(new_data, ShareType::Arithmetic, new_col_name);

        Ok(new_col)
    }
}

impl<T:IntRing2k> Distinct<Rep3RingShare<T>> for ShareColumn<Rep3RingShare<T>>
where
Standard: Distribution<T>,{
    fn distinct<N: Network>(
        &self,
        valid: &[Rep3RingShare<T>],
        e: &[Rep3RingShare<T>],
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<Vec<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();

        let val_data = self.get_data();
        
        let result = distinct_after_groupby_multithreads(val_data, e, valid, nets, states)?;

        Ok(result)
    }
}


impl<T: IntRing2k> ColumnBooleanOperator<Rep3RingShare<T>, T> for ShareColumn<Rep3RingShare<T>>
where
Standard: Distribution<T>,{ 

    fn equal<N: Network>(
        &self, 
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> 
    where
    Standard: Distribution<T>,{ 

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();
        
        let result_bit = eq_many_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn eq_binary<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();
        
        let result_bit = eq_many_binary_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn eq_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();
    
        let data = self.get_data();

        let result_bit = eq_public_many_multithreads(data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn eq_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();

        let data = self.get_data();

        let result_bit = eq_public_many_binary_multithreads(data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn neq<N: Network>(
        &self, 
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> 
    where
    Standard: Distribution<T>,{ 

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let result_bit = neq_many_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn neq_binary<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let result_bit = neq_many_binary_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }
    
    fn neq_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();
    
        let data = self.get_data();

        let result_bit = neq_public_many_multithreads(data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn neq_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();
        
        let data = self.get_data();

        let result_bit = neq_public_many_binary_multithreads(data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn gt<N: Network>(
        &self, 
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> 
    where
    Standard: Distribution<T>,{ 

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();
        
        let result_bit = gt_many_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)

    }

    fn gt_binary<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {
        
        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let result_bit = gt_many_binary_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
        
    }

    fn gt_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();
    
        let a_data = self.get_data();

        let result_bit = gt_public_many_multithreads(a_data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn gt_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();
        
        let a_data = self.get_data();

        let result_bit = gt_public_many_binary_multithreads(a_data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn ge<N: Network>(
        &self, 
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> 
    where
    Standard: Distribution<T>,{ 

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let result_bit = ge_many_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn ge_binary<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {
        
        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let result_bit = ge_many_binary_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn ge_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();
    
        let a_data = self.get_data();

        let result_bit = ge_public_many_multithreads(a_data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn ge_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();
        
        let a_data = self.get_data();

        let result_bit = ge_public_many_binary_multithreads(a_data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn lt<N: Network>(
        &self, 
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> 
    where
    Standard: Distribution<T>,{ 

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let result_bit = lt_many_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn lt_binary<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let result_bit = lt_many_binary_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
        
    }

    fn lt_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();

        let result_bit = lt_public_many_multithreads(a_data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
        
    }

    fn lt_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();

        let result_bit = lt_public_many_binary_multithreads(a_data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn le<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let result_bit = le_many_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn le_binary<N: Network>(
        &self,
        b: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), b.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();
        let b_data = b.get_data();

        let result_bit = le_many_binary_multithreads(a_data, b_data, nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn le_public<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();

        let result_bit = le_public_many_multithreads(a_data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn le_public_binary<N: Network>(
        &self,
        pub_element: &T,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> { 

        let (nets, states) = netstate_args.split();

        let a_data = self.get_data();

        let result_bit = le_public_many_binary_multithreads(a_data, &RingElement(*pub_element), nets, states)?;

        let result = transform::from_bit_to_t(&result_bit)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn in_public_binary<N: net::Network>(
        &self,
        pub_elements: &Vec<T>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert!(!pub_elements.is_empty(), "Public elements cannot be empty");

        let (nets, states) = netstate_args.split();

        let c_data = self.get_data();
        let mut eq_vec = Vec::new();
        
        for pub_element in pub_elements {
            let eq_bit = eq_public_many_binary_multithreads(c_data, &RingElement(*pub_element), nets, states)?;
            let eq = transform::from_bit_to_t_drop(eq_bit)?;
            eq_vec.push(eq);
        }

        let mut res = eq_vec[0].clone();
        
        for i in 1..pub_elements.len() {
            res = or_vec_multithreads(&res, &eq_vec[i], nets, states)?;
        }

        res = transform::b2a_many_multithreads(&res, nets, states)?;

        let result_column = ShareColumn::new(res, ShareType::Binary, self.get_name().to_string());
        
        Ok(result_column)
    }

    fn and<N: Network>(
        &self,
        other_column: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), other_column.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let result = and_vec_multithreads(self.get_data(), other_column.get_data(), nets, states)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

    fn or<N: Network>(
        &self,
        other_column: &ShareColumn<Rep3RingShare<T>>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> eyre::Result<ShareColumn<Rep3RingShare<T>>> {

        assert_eq!(self.len(), other_column.len(), "Columns must have the same length");

        let (nets, states) = netstate_args.split();

        let result = or_vec_multithreads(self.get_data(), other_column.get_data(), nets, states)?;

        let result_column = ShareColumn::new(result, ShareType::Binary, self.get_name().to_string());

        Ok(result_column)
    }

}