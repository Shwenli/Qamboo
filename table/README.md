# Table Module

## Overview

The `table` module provides high-level abstractions for secure table operations in the Qamboo MPC framework. It implements a columnar secure database that allows parties to perform SQL-like queries on secretly shared data, serving as the primary interface for application developers.

## Core Abstractions

### 1. ShareTable (`share_table.rs`)

A secure table where all data is secret-shared among parties.

```rust
pub struct ShareTable<T> {
    pub key_name: Option<String>,
    pub schema: IndexMap<String, ShareColumn<T>>,
}
```

**Features:**
- Columnar storage for efficient analytics
- Named schema with IndexMap for predictable iteration
- Row count inferred from column lengths

```rust
use table::share_table::ShareTable;

// Create empty table
let mut table = ShareTable::<u64>::new();

// Insert columns
table.insert_column("user_id".to_string(), user_id_column);
table.insert_column("amount".to_string(), amount_column);

// Access columns
let ids = table.get_column_by_name("user_id");
let first_col = table.get_column_by_index(0);
```

### 2. ShareColumn (`share_column.rs`)

A column of secret-shared values.

```rust
pub struct ShareColumn<T> {
    pub name: String,
    pub data: Vec<Rep3RingShare<T>>,
}
```

### 3. NetStateArgs (`lib.rs`)

Bundle of network and state references for operator execution:

```rust
pub struct NetStateArgs<'a, N: Network> {
    pub nets: &'a [&'a N],
    pub states: &'a mut [&'a mut Rep3State],
}
```

## Table Operations

### TableOperator Trait (`table_operator.rs`)

Defines the main SQL-like operations on secure tables:

```rust
pub trait Open<T, U> {
    /// Reveal the table to all parties
    fn open<N: Network>(&mut self, netstate_args: &mut NetStateArgs<N>) 
        -> Result<ShareTable<U>>;
}

pub trait Filter<T, U> {
    /// Filter by comparison with public value
    fn filter_public<N: Network>(
        &mut self,
        filter_column: &str,
        predicate: Predicate,
        filter_value: &U,
        netstate_args: &mut NetStateArgs<N>,
    ) -> Result<()>;
    
    /// Filter by comparison with another column
    fn filter_shared<N: Network>(...);
    
    /// Filter by multiple possible values
    fn filter_in<N: Network>(...);
}

pub trait OrderBy<T> {
    /// Sort table by key column
    fn order_by<N: Network>(
        &mut self,
        key_name: &str,
        order: bool,  // true = ASC, false = DESC
        netstate_args: &mut NetStateArgs<N>,
    ) -> Result<()>;
}

pub trait Groupby<T> {
    /// Group by specified keys
    fn group_by<N: Network>(
        &mut self,
        group_keys: Vec<&str>,
        netstate_args: &mut NetStateArgs<N>,
    ) -> Result<(Vec<T>, Vec<Rep3RingShare<u32>>, Vec<Rep3RingShare<Bit>>)>;
}

pub trait AggFunc<T> {
    /// COUNT(*) aggregation
    fn agg_count<N: Network>(...);
    
    /// SUM aggregation
    fn agg_sum<N: Network>(...);
    
    /// AVG aggregation
    fn agg_avg<N: Network>(...);
}
```

### Predicate (`predicate.rs`)

Defines comparison predicates for filtering:

```rust
pub enum Predicate {
    Eq,   // ==
    Ne,   // !=
    Lt,   // <
    Le,   // <=
    Gt,   // >
    Ge,   // >=
}
```

## Usage Examples

### Creating and Sharing Data

```rust
use table::{share_table::ShareTable, NetStateArgs};
use protocols::rep3_ring::share_ring_elements;

// Each party loads their private data
let my_data: Vec<u64> = load_local_data();

// Share data among parties
let rng = &mut thread_rng();
let [shares0, shares1, shares2] = share_ring_elements(&my_data, rng);

// Create shared table
create_table.insert_column("value".to_string(), ShareColumn::new("value", shares0));
```

### Filter Operation

```rust
use table::table_operator::Filter;
use table::predicate::Predicate;

// Filter: SELECT * FROM table WHERE amount > 100
table.filter_public(
    "amount",
    Predicate::Gt,
    &100u64,
    netstate_args,
)?;
```

### Join Operation

```rust
use table::table_operator::Join;

// Join: SELECT * FROM orders JOIN customers ON orders.cust_id = customers.id
let joined_table = orders_table.join(
    &customers_table,
    "cust_id",      // left key
    "id",           // right key
    netstate_args,
)?;
```

### GroupBy with Aggregation

```rust
use table::table_operator::{Groupby, AggFunc};

// SELECT region, SUM(sales), COUNT(*) FROM table GROUP BY region
let (group_keys, perm, eq_bits) = table.group_by(
    vec!["region"],
    netstate_args,
)?;

// Apply aggregations
table.agg_sum("sales", "total_sales", &eq_bits, &perm, netstate_args)?;
table.agg_count("count", &eq_bits, &perm, netstate_args)?;
```

### Sort Operation

```rust
use table::table_operator::OrderBy;

// ORDER BY date DESC
table.order_by("date", false, netstate_args)?;
```

### Complete Query Example

```rust
// TPC-H Query 1 style operation
let mut lineitem = ShareTable::<u64>::new();
lineitem.insert_column("l_shipdate".to_string(), shipdate_col);
lineitem.insert_column("l_returnflag".to_string(), returnflag_col);
lineitem.insert_column("l_linestatus".to_string(), linestatus_col);
lineitem.insert_column("l_quantity".to_string(), quantity_col);
lineitem.insert_column("l_extendedprice".to_string(), price_col);

// 1. Filter by date
lineitem.filter_public(
    "l_shipdate",
    Predicate::Le,
    &date_threshold,
    netstate_args,
)?;

// 2. Group by returnflag and linestatus
let (_, perm, eq_bits) = lineitem.group_by(
    vec!["l_returnflag", "l_linestatus"],
    netstate_args,
)?;

// 3. Compute aggregations
lineitem.agg_sum("l_quantity", "sum_qty", &eq_bits, &perm, netstate_args)?;
lineitem.agg_sum("l_extendedprice", "sum_price", &eq_bits, &perm, netstate_args)?;
lineitem.agg_count("count_order", &eq_bits, &perm, netstate_args)?;

// 4. Order by returnflag, linestatus
lineitem.order_by("l_returnflag", true, netstate_args)?;

// 5. Open result
let result = lineitem.open(netstate_args)?;
```

## Column Operations

### ColumnOperator Trait (`column_operator.rs`)

Low-level column-wise operations:

```rust
pub trait ColumnOperator<T> {
    /// Arithmetic operations
    fn add_column(&self, other: &ShareColumn<T>) -> Result<ShareColumn<T>>;
    fn sub_column(&self, other: &ShareColumn<T>) -> Result<ShareColumn<T>>;
    fn mul_column<N: Network>(...);
    
    /// Comparison operations
    fn eq_column<N: Network>(...) -> Result<Vec<Rep3RingShare<Bit>>>;
    fn gt_column<N: Network>(...) -> Result<Vec<Rep3RingShare<Bit>>>;
}
```

## Architecture

```text
table/
├── src/
│   ├── lib.rs                   # Module exports, NetStateArgs
│   ├── share_table.rs           # ShareTable definition
│   ├── share_column.rs          # ShareColumn definition
│   ├── table_operator.rs        # High-level SQL operators (traits)
│   ├── table_filter.rs          # Filter implementation
│   ├── table_join.rs            # Join implementation
│   ├── table_group_by.rs        # GroupBy implementation
│   ├── table_order_by.rs        # OrderBy implementation
│   ├── table_project.rs         # Column projection
│   ├── table_utils.rs           # Utility functions
│   ├── predicate.rs             # Filter predicates
│   ├── column_basic_compute.rs  # Basic column operations
│   ├── column_operator.rs       # Column operator traits
│   └── column_operator_impl.rs  # Column operator implementations
```

## Type Parameters

- `T`: The underlying ring type (typically `u64`)
- `U`: The revealed type after opening (typically `u128` for precision)

## Integration with Lower Layers

```text
Application Code
       ↓
   table (SQL-like interface)
       ↓
   operator (relational operators)
       ↓
   primitives (crypto primitives)
       ↓
   protocols (MPC protocols)
       ↓
   communication + net (network layer)
```

## Performance Notes

### Memory Layout

- Columnar storage enables SIMD-friendly operations
- IndexMap provides O(1) column lookup by name
- Predictable iteration order for reproducibility

### Lazy Evaluation

Operations are executed eagerly (no query planning optimization yet).

### Multi-Threading

All operations support multi-threaded execution:

```rust
// Fork state for parallel processing
let mut states = state.fork(num_threads)?;
let state_refs: Vec<_> = states.iter_mut().collect();
let net_refs: Vec<_> = nets.iter().collect();

let mut args = NetStateArgs::new(&net_refs, &state_refs);
table.order_by("key", true, &mut args)?;
```

## Security

- All operations preserve the secret-sharing invariant
- No intermediate values are revealed during computation
- Only explicitly `open()`ed results are revealed to all parties
