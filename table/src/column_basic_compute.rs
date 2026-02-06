use core::panic;
use crate::share_column::{ShareType, ShareColumn};
use protocols::protocols::rep3_ring::id::PartyID;
use protocols::protocols::rep3_ring::arithmetic;
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::rep3_ring::Rep3RingShare;
use protocols::protocols::rep3_ring::Rep3State;
use rand::distributions::{Distribution, Standard};
use net::Network;
use primitives::{div, utils};
use crate::NetStateArgs;



impl<T: IntRing2k> std::ops::Add<ShareColumn<Rep3RingShare<T>>> for ShareColumn<Rep3RingShare<T>> {
    type Output = Self;
    fn add(self, rhs: ShareColumn<Rep3RingShare<T>>) -> Self::Output {
        assert_eq!(self.len(), rhs.len(), "Columns must have the same length");

        let data = self
            .get_data()
            .iter()
            .zip(rhs.get_data().iter())
            .map(|(a, b)| *a + *b)
            .collect();

        ShareColumn::new(data, ShareType::Arithmetic, self.get_name().to_string())
    }
}

impl<T: IntRing2k> std::ops::Add<(RingElement<T>,&PartyID)> for ShareColumn<Rep3RingShare<T>> {
    type Output = Self;
    fn add(self, rhs: (RingElement<T>, &PartyID)) -> Self::Output {
        //assert_eq!(self.len(), rhs.0.len(), "Columns must have the same length");

        let data = self
            .get_data()
            .iter()
            .map(|a| arithmetic::add_public(*a, rhs.0, *rhs.1))
            .collect::<Vec<_>>();

        ShareColumn::new(data, ShareType::Arithmetic, self.get_name().to_string())
    }
}


impl<T: IntRing2k> std::ops::AddAssign for ShareColumn<Rep3RingShare<T>> {
    fn add_assign(& mut self, rhs: ShareColumn<Rep3RingShare<T>>) {
        assert_eq!(self.len(), rhs.len(), "Columns must have the same length");
            self
            .get_data_mut()
            .iter_mut()
            .zip(rhs.get_data().iter())
            .for_each(|(a, b)| arithmetic::add_assign(a, *b));
    }
}



impl<T: IntRing2k> std::ops::Sub<ShareColumn<Rep3RingShare<T>>> for ShareColumn<Rep3RingShare<T>> {
    type Output = Self;

    fn sub(self, rhs: ShareColumn<Rep3RingShare<T>>) -> Self::Output {
        assert_eq!(self.len(), rhs.len(), "Columns must have the same length");
        let data = self
            .get_data()
            .iter()
            .zip(rhs.get_data().iter())
            .map(|(a, b)| *a - *b)
            .collect();
        ShareColumn::new(data, ShareType::Arithmetic, self.get_name().to_string())
    }
}


impl<T: IntRing2k> std::ops::Mul<RingElement<T>> for ShareColumn<Rep3RingShare<T>> {
    type Output = Self;

    fn mul(self, rhs: RingElement<T>) -> Self::Output {
        let data = self.get_data().iter().map(|a| *a * rhs).collect();
        ShareColumn::new(data, ShareType::Arithmetic, self.get_name().to_string())
    }
}


impl<T: IntRing2k, N: Network> std::ops::Mul<(ShareColumn<Rep3RingShare<T>>, (&N, &mut Rep3State), (&N, &mut Rep3State))> for ShareColumn<Rep3RingShare<T>> 
where
    Standard: Distribution<T>,{
    type Output = Self;

    fn mul(self, rhs: (ShareColumn<Rep3RingShare<T>>, (&N, &mut Rep3State), (&N, &mut Rep3State))) -> Self::Output {
        assert_eq!(self.len(), rhs.0.len(), "Columns must have the same length");

        let state0 = rhs.1.1;
        let state1 = rhs.2.1;
        let(mul1,mul2) = net::join(
        || {
            let mul1_tmp = arithmetic::local_mul_vec(&self.get_data()[..self.len()/2], &rhs.0.get_data()[..rhs.0.len()/2], state0);
            arithmetic::reshare_vec(mul1_tmp, rhs.1.0).unwrap_or_else(|e|panic!("sharecolumn mul reshare error : {}", e))
        },
        || {
            let mul2_tmp = arithmetic::local_mul_vec(&self.get_data()[self.len()/2..], &rhs.0.get_data()[rhs.0.len()/2..], state1);
            arithmetic::reshare_vec(mul2_tmp, rhs.2.0).unwrap_or_else(|e|panic!("sharecolumn mul reshare error : {}", e))
        },
    );
        let data = [mul1, mul2].concat();

        ShareColumn::new(data, ShareType::Arithmetic, self.get_name().to_string())
    }
}

impl<'a, 'b, T: IntRing2k, N: Network> std::ops::Mul<(&ShareColumn<Rep3RingShare<T>>, &'b mut NetStateArgs<'a, N>)> for ShareColumn<Rep3RingShare<T>> 
where
    Standard: Distribution<T>,{
    type Output = Self;
    fn mul(self, rhs: (&ShareColumn<Rep3RingShare<T>>, &'b mut NetStateArgs<'a, N>)) -> Self::Output {
        assert_eq!(self.len(), rhs.0.len(), "Columns must have the same length");

        let (nets, state, _state1, _) = rhs.1.split();
        let mul_data = self.get_data();
        let mul_tmp = arithmetic::local_mul_vec(mul_data, &rhs.0.get_data(), state);
        let result = utils::reshare_vec_q_multithreads(&mul_tmp, nets).unwrap_or_else(|e|panic!("sharecolumn mul reshare error : {}", e));
        
        ShareColumn::new(result, ShareType::Arithmetic, self.get_name().to_string())
    }
}

impl<'a, 'b, T: IntRing2k, N: Network> std::ops::MulAssign<(&ShareColumn<Rep3RingShare<T>>, &'b mut NetStateArgs<'a, N>)> for ShareColumn<Rep3RingShare<T>> 
where
    Standard: Distribution<T>,{

    fn mul_assign(&mut self, rhs: (&ShareColumn<Rep3RingShare<T>>, &'b mut NetStateArgs<'a, N>)){
        assert_eq!(self.len(), rhs.0.len(), "Columns must have the same length");

        let (nets, state, _, _) = rhs.1.split();
        let mul_data = self.get_data();
        let mul_tmp = arithmetic::local_mul_vec(mul_data, &rhs.0.get_data(), state);
        let result = utils::reshare_vec_q_multithreads(&mul_tmp, nets).unwrap_or_else(|e|panic!("sharecolumn mul reshare error : {}", e));
        self.update_data(result);
    }
}

impl<'a, 'b, N: Network> std::ops::Div<(&RingElement<u64>, &'b mut NetStateArgs<'a, N>)> for ShareColumn<Rep3RingShare<u64>> {
    type Output = Self;

    fn div(self, rhs: (&RingElement<u64>, &'b mut NetStateArgs<'a, N>)) -> Self::Output {
        let data = self.get_data();
        let (nets, _state0, _state1, states) = rhs.1.split();
        let res_data = div::div_share_by_public_arithmetic_multithreads(data, rhs.0, nets, states).unwrap_or_else(|e|panic!("ShareColumn: div error: {}", e));
        ShareColumn::new(res_data, ShareType::Arithmetic, self.get_name().to_string())
    }
}

impl<'a, 'b, N: Network> std::ops::DivAssign<(&RingElement<u64>, &'b mut NetStateArgs<'a, N>)> for ShareColumn<Rep3RingShare<u64>> {
    fn div_assign(&mut self, rhs: (&RingElement<u64>, &'b mut NetStateArgs<'a, N>)) {
        let data = self.get_data();
        let (nets, _state0, _state1, states) = rhs.1.split();
        let res_data = div::div_share_by_public_arithmetic_multithreads(data, rhs.0, nets, states).unwrap_or_else(|e|panic!("ShareColumn: div error: {}", e));
        self.update_data(res_data);
    }
}

impl<'a, 'b, T: IntRing2k, N: Network> std::ops::Div<(&ShareColumn<Rep3RingShare<T>>, &'b mut NetStateArgs<'a, N>)> for ShareColumn<Rep3RingShare<T>> 
where
    Standard: Distribution<T>,{
    type Output = Self;
    fn div(self, rhs: (&ShareColumn<Rep3RingShare<T>>, &'b mut NetStateArgs<'a, N>)) -> Self::Output {
        let rhs_data = rhs.0.get_data();
        let data = self.get_data();
        let bitsize = T::K / 2;
        let (nets, _state0, _state1, states) = rhs.1.split();
        let res_data = div::div_multithreads::<T,N>(data, &rhs_data, bitsize, nets, states).unwrap_or_else(|e|panic!("ShareColumn: div error: {}", e));
        ShareColumn::new(res_data, ShareType::Arithmetic, self.get_name().to_string())
    }
}

impl<'a, 'b, T: IntRing2k, N: Network> std::ops::DivAssign<(&ShareColumn<Rep3RingShare<T>>, &'b mut NetStateArgs<'a, N>)> for ShareColumn<Rep3RingShare<T>> 
where
    Standard: Distribution<T>,{
    fn div_assign(&mut self, rhs: (&ShareColumn<Rep3RingShare<T>>, &'b mut NetStateArgs<'a, N>)) {
        let rhs_data = rhs.0.get_data();
        let data = self.get_data();
        let bitsize = T::K / 2;
        let (nets, _state0, _state1, states) = rhs.1.split();
        let res_data = div::div_multithreads::<T,N>(data, &rhs_data, bitsize, nets, states).unwrap_or_else(|e|panic!("ShareColumn: div error: {}", e));
        self.update_data(res_data);
    }
}



impl<T: IntRing2k> std::ops::Neg for ShareColumn<Rep3RingShare<T>> {
    type Output = Self;
    fn neg(self)-> Self:: Output{

        let data = self.get_data().iter().map(|a| arithmetic::neg(*a)).collect();

        ShareColumn::new(data, ShareType::Arithmetic, self.get_name().to_string() + "_temp")

    }
}






