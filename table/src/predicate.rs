use protocols::protocols::rep3_ring::Rep3State;
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::rep3_ring::{Rep3RingShare, arithmetic};
use protocols::protocols::rep3_ring::ring::bit::Bit;
use net::Network;
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
        net: &N,
        state: &mut Rep3State,
    ) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
    where
        Standard: Distribution<T>,
    {
        match self {
            Predicate::Equal => {
                primitives::compare::eq_public_many(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::NotEqual => {
                primitives::compare::neq_public_many(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::GreaterThan => {
                primitives::compare::gt_public_many(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::GreaterOrEqual => {
                primitives::compare::ge_public_many(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::LessThan => {
                primitives::compare::lt_public_many(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::LessOrEqual => {
                primitives::compare::le_public_many(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::EqualBinary => {
                primitives::compare::eq_public_many_binary(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::NotEqualBinary => {
                primitives::compare::neq_public_many_binary(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::GreaterThanBinary => {
                primitives::compare::gt_public_many_binary(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::GreaterOrEqualBinary => {
                primitives::compare::ge_public_many_binary(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::LessThanBinary => {
                primitives::compare::lt_public_many_binary(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
            Predicate::LessOrEqualBinary => {
                primitives::compare::le_public_many_binary(
                    shared_values,
                    public_value,
                    net,
                    state,
                )
            }
        }
    }

    /// 应用谓词到两列共享值的比较
    /// 
    /// # Arguments
    /// * `lhs` - 左侧共享值的切片
    /// * `rhs` - 右侧共享值的切片
    /// * `net` - 网络连接
    /// * `state` - Rep3 状态
    /// 
    /// # Returns
    /// 返回布尔掩码的共享值，true 表示满足谓词条件
    pub fn apply_shared<T: IntRing2k, N: Network>(
        &self,
        lhs: &[Rep3RingShare<T>],
        rhs: &[Rep3RingShare<T>],
        net: &N,
        state: &mut Rep3State,
    ) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
    where
        Standard: Distribution<T>,
    {
        match self {

            Predicate::Equal => {
                primitives::compare::eq_many(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::NotEqual => {
                primitives::compare::neq_many(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::GreaterThan => {
                primitives::compare::gt_many(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::GreaterOrEqual => {
                primitives::compare::ge_many(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::LessThan => {
                primitives::compare::lt_many(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::LessOrEqual => {
                primitives::compare::le_many(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::EqualBinary => {
                primitives::compare::eq_many_binary(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::NotEqualBinary => {
                primitives::compare::neq_many_binary(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::GreaterThanBinary => {
                primitives::compare::gt_many_binary(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::GreaterOrEqualBinary => {
                primitives::compare::ge_many_binary(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::LessThanBinary => {
                primitives::compare::lt_many_binary(
                    lhs,
                    rhs,
                    net,
                    state,
                )
            }
            Predicate::LessOrEqualBinary => {
                primitives::compare::le_many_binary(
                    lhs,
                    rhs,
                    net,
                    state,
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
