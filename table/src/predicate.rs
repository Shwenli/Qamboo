use random::rep3::Rep3State;
use protocols::rep3_ring::{Rep3RingShare, arithmetic};
use algebra::ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement};
use net::Network;
use primitives::compare::*;
use rand::distributions::Standard;
use rand::prelude::Distribution;

/// 谓词枚举，定义支持的比较操作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Predicate {
    /// 等于 (==)
    Equal,
    /// 不等于 (!=)
    NotEqual,
    /// 大于 (>)
    GreaterThan,
    /// 大于等于 (>=)
    GreaterOrEqual,
    /// 小于 (<)
    LessThan,
    /// 小于等于 (<=)
    LessOrEqual,
    ///
    EqualBinary,
    ///
    NotEqualBinary,
    ///
    GreaterThanBinary,
    ///
    GreaterOrEqualBinary,
    ///
    LessThanBinary,
    ///
    LessOrEqualBinary,
}

impl Predicate {
    /// 应用谓词到共享值和公开值的比较
    /// 
    /// # Arguments
    /// * `shared_values` - 共享值的切片
    /// * `public_value` - 公开值
    /// * `net` - 网络连接
    /// * `state` - Rep3 状态
    /// 
    /// # Returns
    /// 返回布尔掩码的共享值，true 表示满足谓词条件
    pub fn apply_public<T: IntRing2k, N: Network>(
        &self,
        shared_values: &[Rep3RingShare<T>],
        public_value: &RingElement<T>,
        nets: &[&N],
        states: &mut[&mut Rep3State],
    ) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
    where
        Standard: Distribution<T>,
    {
        match self {
            Predicate::Equal => {
                eq_public_many_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::NotEqual => {
                neq_public_many_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::GreaterThan => {
                gt_public_many_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::GreaterOrEqual => {
                ge_public_many_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::LessThan => {
                lt_public_many_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::LessOrEqual => {
                le_public_many_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::EqualBinary => {
                eq_public_many_binary_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::NotEqualBinary => {
                neq_public_many_binary_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::GreaterThanBinary => {
                gt_public_many_binary_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::GreaterOrEqualBinary => {
                ge_public_many_binary_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::LessThanBinary => {
                lt_public_many_binary_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
            Predicate::LessOrEqualBinary => {
                le_public_many_binary_multithreads(
                    shared_values,
                    public_value,
                    nets,
                    states,
                )
            }
        }
    }

    /// 应用谓词到两列共享值的比较
    /// 
    /// # Arguments
    /// * `lhs` - 左侧共享值的切片
    /// * `rhs` - 右侧共享值的切片
    /// * `nets` - 网络连接数组
    /// * `states` - Rep3 状态数组
    /// 
    /// # Returns
    /// 返回布尔掩码的共享值，true 表示满足谓词条件
    pub fn apply_shared<T: IntRing2k, N: Network>(
        &self,
        lhs: &[Rep3RingShare<T>],
        rhs: &[Rep3RingShare<T>],
        nets: &[&N],
        states: &mut [&mut Rep3State],
    ) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
    where
        Standard: Distribution<T>,
    {
        match self {

            Predicate::Equal => {
                eq_many_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::NotEqual => {
                neq_many_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::GreaterThan => {
                gt_many_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::GreaterOrEqual => {
                ge_many_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::LessThan => {
                lt_many_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::LessOrEqual => {
                le_many_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::EqualBinary => {
                eq_many_binary_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::NotEqualBinary => {
                neq_many_binary_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::GreaterThanBinary => {
                gt_many_binary_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::GreaterOrEqualBinary => {
                ge_many_binary_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::LessThanBinary => {
                lt_many_binary_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
            Predicate::LessOrEqualBinary => {
                le_many_binary_multithreads(
                    lhs,
                    rhs,
                    nets,
                    states,
                )
            }
        }
    }


    /// 打开掩码位并转换为布尔值
    /// 
    /// # Arguments
    /// * `mask_bits` - 共享的位掩码
    /// * `net` - 网络连接
    /// 
    /// # Returns
    /// 返回打开后的布尔掩码
    pub fn open_mask<T: IntRing2k, N: Network>(
        mask_bits: &[Rep3RingShare<Bit>],
        net: &N,
    ) -> eyre::Result<Vec<bool>>
    where
        Standard: Distribution<T>,{
        let opened_mask = arithmetic::open_vec_bit(mask_bits, net)?;
        let mask: Vec<bool> = opened_mask.into_iter().map(|b| b.0.convert()).collect();
        Ok(mask)
    }
}


// 便捷的谓词构造函数
impl Predicate {
    /// 创建等于谓词
    pub fn eq() -> Self {
        Predicate::Equal
    }

    /// 创建不等于谓词
    pub fn ne() -> Self {
        Predicate::NotEqual
    }

    /// 创建大于谓词
    pub fn gt() -> Self {
        Predicate::GreaterThan
    }

    /// 创建大于等于谓词
    pub fn ge() -> Self {
        Predicate::GreaterOrEqual
    }

    /// 创建小于谓词
    pub fn lt() -> Self {
        Predicate::LessThan
    }

    /// 创建小于等于谓词
    pub fn le() -> Self {
        Predicate::LessOrEqual
    }

    /// 创建等于二进制谓词
    pub fn eq_binary() -> Self {
        Predicate::EqualBinary
    }
    /// 创建不等于二进制谓词
    pub fn ne_binary() -> Self {
        Predicate::NotEqualBinary
    }
    /// 创建大于二进制谓词
    pub fn gt_binary() -> Self {
         Predicate::GreaterThanBinary
    }
    /// 创建大于等于二进制谓词
    pub fn ge_binary() -> Self {
        Predicate::GreaterOrEqualBinary
    }
    /// 创建小于二进制谓词
    pub fn lt_binary() -> Self {
        Predicate::LessThanBinary
    }
    /// 创建小于等于二进制谓词
    pub fn le_binary() -> Self {
        Predicate::LessOrEqualBinary
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predicate_creation() {
        assert_eq!(Predicate::eq(), Predicate::Equal);
        assert_eq!(Predicate::ne(), Predicate::NotEqual);
        assert_eq!(Predicate::gt(), Predicate::GreaterThan);
        assert_eq!(Predicate::ge(), Predicate::GreaterOrEqual);
        assert_eq!(Predicate::lt(), Predicate::LessThan);
        assert_eq!(Predicate::le(), Predicate::LessOrEqual);
        assert_eq!(Predicate::eq_binary(), Predicate::EqualBinary);
        assert_eq!(Predicate::ne_binary(), Predicate::NotEqualBinary);
        assert_eq!(Predicate::gt_binary(), Predicate::GreaterThanBinary);
        assert_eq!(Predicate::ge_binary(), Predicate::GreaterOrEqualBinary);
        assert_eq!(Predicate::lt_binary(), Predicate::LessThanBinary);
        assert_eq!(Predicate::le_binary(), Predicate::LessOrEqualBinary);
       
    }
}
