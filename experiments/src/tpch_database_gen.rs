use itertools::izip;
use protocols::protocols::rep3_ring::id::PartyID;
use protocols::protocols::rep3_ring::network::Rep3NetworkExt;
use protocols::protocols::rep3_ring::{self, Rep3RingShare};
use net::Network;
use table::share_table::ShareTable;
use table::share_column::{ShareColumn, ShareType};
use table::NetStateArgs;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use primitives::transform::a2b_many_multithreads;
use rand::{thread_rng,Rng};
use polars::prelude::*;


pub fn get_lineitem_table_size(sf: f32) -> u64 {
    (sf * 6000000.0) as u64
}

pub fn get_orders_table_size(sf: f32) -> u64 {
    (sf * 1500000.0) as u64
}

pub fn get_customer_table_size(sf: f32) -> u64 {
    (sf * 150000.0) as u64
}

pub fn get_part_table_size(sf: f32) -> u64 {
    (sf * 200000.0) as u64
}

pub fn get_supplier_table_size(sf: f32) -> u64 {
    (sf * 10000.0) as u64
}

pub fn get_partsupp_table_size(sf: f32) -> u64 {
    (sf * 800000.0) as u64
}

pub fn get_nation_table_size() -> u64 {
    25
}

pub fn get_region_table_size() -> u64 {
    5
}



fn gen_rand_column_data_pk_u64(num_rows: usize) -> Vec<u64> {
    //生成的值为1-numrows的序列
    let mut data = vec![];

    for i in 0..num_rows {
        let val = i as u64 + 1;
        data.push(val);
    }

    data
}

fn gen_rand_column_data_u64(num_rows: usize, max_val: u64, min_val: u64) -> Vec<u64> {

    let range = max_val - min_val;
    let mut rng = thread_rng();

    let data: Vec<u64> = (0..num_rows).map(|_| rng.r#gen::<u64>()%range + min_val).collect();

    data
}

fn gen_valid_column_data_u64(num_rows: usize) -> Vec<u64> {

    let data = vec![1;num_rows];

    data
}

fn gen_valid_column_u64_ring<N: Network>(
    num_rows: usize,
    name: String,
    datatype: ShareType,
    net0: &N,
    net1: &N,
    partyid: PartyID,
) -> ShareColumn<Rep3RingShare<u64>>
{
    
    match partyid{
        PartyID::ID0=>{
            let data = gen_valid_column_data_u64(num_rows);
            let len = data.len();

            let data_ring = izip!(data).map(|val| RingElement(val)).collect::<Vec<_>>();

            let mut rng = thread_rng();
            let data_ring_share = rep3_ring::share_ring_elements(&data_ring, &mut rng);
            let _ = net0.send_many(PartyID::ID1, &data_ring_share[1][..len/2]);
            let _ = net1.send_many(PartyID::ID1, &data_ring_share[1][len/2..]);
            let _ = net0.send_many(PartyID::ID2, &data_ring_share[2][..len/2]);
            let _ = net1.send_many(PartyID::ID2, &data_ring_share[2][len/2..]);

            let share_col = ShareColumn::new(data_ring_share[0].clone(), datatype, name);

            share_col

        }
        PartyID::ID1=>{
            let data_ring_share_1: Vec<Rep3RingShare<u64>> = net0.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share_2: Vec<Rep3RingShare<u64>> = net1.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share= [data_ring_share_1,data_ring_share_2].concat();
            let share_col = ShareColumn::new(data_ring_share, datatype, name);

            share_col
        }
        PartyID::ID2=>{
            let data_ring_share_1: Vec<Rep3RingShare<u64>> = net0.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share_2: Vec<Rep3RingShare<u64>> = net1.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share= [data_ring_share_1,data_ring_share_2].concat();
            let share_col = ShareColumn::new(data_ring_share, datatype, name);

            share_col
        }
        
    }
}

pub fn gen_rand_column_u64_ring<N: Network>(
    num_rows: usize,
    name: String,
    max_val: u64,
    min_val: u64,
    datatype: ShareType,
    net0: &N,
    net1: &N,
    partyid: PartyID,
) -> (ShareColumn<Rep3RingShare<u64>>, Option<Vec<u64>>)
{
    
    match  partyid{
        PartyID::ID0=>{
            let mut rng = thread_rng();

            let data = gen_rand_column_data_u64(num_rows, max_val, min_val);
            let len = data.len();
            let data_clone = data.clone();

            let data_ring = izip!(data).map(|val| RingElement(val)).collect::<Vec<_>>();

            let data_ring_share = rep3_ring::share_ring_elements(&data_ring, &mut rng);
            
            let _ = net0.send_many(PartyID::ID1, &data_ring_share[1][..len/2]);
            let _ = net1.send_many(PartyID::ID1, &data_ring_share[1][len/2..]);
            let _ = net0.send_many(PartyID::ID2, &data_ring_share[2][..len/2]);
            let _ = net1.send_many(PartyID::ID2, &data_ring_share[2][len/2..]);

            let share_col = ShareColumn::new(data_ring_share[0].clone(), datatype, name);

            (share_col, Some(data_clone))

        }
        PartyID::ID1=>{
            let data_ring_share_1: Vec<Rep3RingShare<u64>> = net0.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share_2: Vec<Rep3RingShare<u64>> = net1.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share= [data_ring_share_1,data_ring_share_2].concat();
            let share_col = ShareColumn::new(data_ring_share, datatype, name);
            (share_col, None)

        }
        PartyID::ID2=>{
            let data_ring_share_1: Vec<Rep3RingShare<u64>> = net0.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share_2: Vec<Rep3RingShare<u64>> = net1.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share= [data_ring_share_1,data_ring_share_2].concat();
            let share_col = ShareColumn::new(data_ring_share, datatype, name);

            (share_col, None)
        }
        
    }
}

fn get_rand_column_pk_u64_ring<N: Network>(
    num_rows: usize,
    name: String,
    datatype: ShareType,
    net0: &N,
    net1: &N,
    partyid: PartyID
) -> (ShareColumn<Rep3RingShare<u64>>, Option<Vec<u64>>)
{
    
    match  partyid{
        PartyID::ID0=>{
            let data = gen_rand_column_data_pk_u64(num_rows);
            let len = data.len();
            let data_clone = data.clone();

            let data_ring = izip!(data).map(|val| RingElement(val)).collect::<Vec<_>>();

            let mut rng = thread_rng();
            let data_ring_share = rep3_ring::share_ring_elements(&data_ring, &mut rng);

            let _ = net0.send_many(PartyID::ID1, &data_ring_share[1][..len/2]);
            let _ = net1.send_many(PartyID::ID1, &data_ring_share[1][len/2..]);
            let _ = net0.send_many(PartyID::ID2, &data_ring_share[2][..len/2]);
            let _ = net1.send_many(PartyID::ID2, &data_ring_share[2][len/2..]);

            let share_col = ShareColumn::new(data_ring_share[0].clone(), datatype, name);

            (share_col, Some(data_clone))

        }
        PartyID::ID1=>{
            let data_ring_share_1: Vec<Rep3RingShare<u64>> = net0.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share_2: Vec<Rep3RingShare<u64>> = net1.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share= [data_ring_share_1,data_ring_share_2].concat();
            let share_col = ShareColumn::new(data_ring_share, datatype, name);
            (share_col, None)

        }
        PartyID::ID2=>{
            let data_ring_share_1: Vec<Rep3RingShare<u64>> = net0.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share_2: Vec<Rep3RingShare<u64>> = net1.recv_many(PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let data_ring_share= [data_ring_share_1,data_ring_share_2].concat();
            let share_col = ShareColumn::new(data_ring_share, datatype, name);

            (share_col, None)
        }
        
    }
}

pub fn convert_binary_from_arithmetic<N: Network>(
    ori_col: &ShareColumn<Rep3RingShare<u64>>,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<ShareColumn<Rep3RingShare<u64>>> { 

    let arithematic_data = ori_col.get_data();
    let (nets, _state0, _state1, states) = netstate_args.split();
    let binary_data = a2b_many_multithreads(arithematic_data, nets, states)?;
    let new_name = format!("[{}]", ori_col.get_name());
    let share_col = ShareColumn::new(binary_data, ShareType::Binary, new_name);

    Ok(share_col)
}

//* TPCH lineitem table
//* 
pub fn gen_lineitem_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {

    let num_rows = (sf * 6000000.0) as usize; 

    /*
    let schema = vec![
        ("l_orderkey".to_string(), ShareType::Arithmetic),
        ("l_partkey".to_string(), ShareType::Arithmetic),
        ("l_suppkey".to_string(), ShareType::Arithmetic),
        ("l_linenumber".to_string(), ShareType::Arithmetic),
        ("l_quantity".to_string(), ShareType::Arithmetic),
        ("l_extendedprice".to_string(), ShareType::Arithmetic),
        ("l_discount".to_string(), ShareType::Arithmetic),
        ("l_tax".to_string(), ShareType::Arithmetic),
        ("l_returnflag".to_string(), ShareType::Arithmetic),
        ("l_linestatus".to_string(), ShareType::Arithmetic),
        ("l_shipdate".to_string(), ShareType::Arithmetic),
        ("l_commitdate".to_string(), ShareType::Arithmetic),
        ("l_receiptdate".to_string(), ShareType::Arithmetic),
        ("l_shipinstruct".to_string(), ShareType::Arithmetic),
        ("l_shipmode".to_string(), ShareType::Arithmetic),
        ("l_comment".to_string(), ShareType::Arithmetic),
    ];
    */
    let (nets, state0, state1, _states) = netstate_args.split();
    let (net0, net1) = (nets[0], nets[1]);

    let partyid = state0.id;

    let mut lineitem_table = ShareTable::<Rep3RingShare<u64>>::new();

    lineitem_table.key_name = Some("no".to_string());
    
    // For Polars LazyFrame construction on ID0
    let mut columns: Vec<Column> = Vec::new();


    let (orderkey, plain_orderkey) = gen_rand_column_u64_ring(num_rows, "l_orderkey".to_string(),get_orders_table_size(sf), 1, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_orderkey".to_string(), orderkey);
    if let Some(data) = plain_orderkey {
        columns.push(Column::new("l_orderkey".into(), data));
    }

    let (partkey, plain_partkey) = gen_rand_column_u64_ring(num_rows, "l_partkey".to_string(),get_part_table_size(sf), 1, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_partkey".to_string(), partkey);
    if let Some(data) = plain_partkey {
        columns.push(Column::new("l_partkey".into(), data));
    }

    let (suppkey, plain_suppkey) = gen_rand_column_u64_ring(num_rows, "l_suppkey".to_string(),get_supplier_table_size(sf), 1, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_suppkey".to_string(), suppkey);
    if let Some(data) = plain_suppkey {
        columns.push(Column::new("l_suppkey".into(), data));
    }
    
    /* Missing l_linenumber in original code? Assuming it was intentional to skip or use default behavior? 
       The original code didn't insert l_linenumber either.
    */

    let (quantity, plain_quantity) = gen_rand_column_u64_ring(num_rows, "l_quantity".to_string(), 51, 1, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_quantity".to_string(), quantity);
    // Keep plain_quantity for extendedprice calculation
    let plain_quantity_clone = plain_quantity.clone();
    if let Some(data) = plain_quantity {
        columns.push(Column::new("l_quantity".into(), data));
    }

    let (extendedprice, plain_extendedprice) = gen_rand_column_u64_ring(num_rows, "l_extendedprice".to_string(), 111, 90, ShareType::Arithmetic, net0, net1, partyid);
    let extendedprice = extendedprice * (lineitem_table["l_quantity"].clone(), (net0, state0), (net1, state1));
    lineitem_table.insert_column("l_extendedprice".to_string(), extendedprice);
    
    if let (Some(mut ep), Some(qt)) = (plain_extendedprice, plain_quantity_clone) {
        // extendedprice *= quantity
        for (e, q) in ep.iter_mut().zip(qt.iter()) {
            *e *= *q;
        }
        columns.push(Column::new("l_extendedprice".into(), ep));
    }


    let (discount, plain_discount) = gen_rand_column_u64_ring(num_rows, "l_discount".to_string(), 10, 0, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_discount".to_string(), discount);
    if let Some(data) = plain_discount {
        columns.push(Column::new("l_discount".into(), data));
    }

    let (tax, plain_tax) = gen_rand_column_u64_ring(num_rows, "l_tax".to_string(), 8, 0, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_tax".to_string(), tax);
    if let Some(data) = plain_tax {
        columns.push(Column::new("l_tax".into(), data));
    }

    let (returnflag, plain_returnflag) = gen_rand_column_u64_ring(num_rows, "l_returnflag".to_string(), 3, 0, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_returnflag".to_string(), returnflag);
    if let Some(data) = plain_returnflag {
        columns.push(Column::new("l_returnflag".into(), data));
    }

    let (linestatus, plain_linestatus) = gen_rand_column_u64_ring(num_rows, "l_linestatus".to_string(), 2, 0, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_linestatus".to_string(), linestatus);
    if let Some(data) = plain_linestatus {
        columns.push(Column::new("l_linestatus".into(), data));
    }

    let (shipdate, plain_shipdate) = gen_rand_column_u64_ring(num_rows, "l_shipdate".to_string(), 121, 1, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_shipdate".to_string(), shipdate);
    let plain_shipdate_clone = plain_shipdate.clone();
    if let Some(data) = plain_shipdate {
        columns.push(Column::new("l_shipdate".into(), data));
    }

    let (commitdate, plain_commitdate) = gen_rand_column_u64_ring(num_rows, "l_commitdate".to_string(), 90, 30, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_commitdate".to_string(), commitdate);
    if let Some(data) = plain_commitdate {
        columns.push(Column::new("l_commitdate".into(), data));
    }

    let (receiptdate_rand, plain_receiptdate_rand) = gen_rand_column_u64_ring(num_rows, "l_receiptdate".to_string(), 30, 1, ShareType::Arithmetic, net0, net1, partyid);
    let receiptdate_tmp = lineitem_table["l_shipdate"].clone() + receiptdate_rand;
    let receiptdate = ShareColumn::new(receiptdate_tmp.get_data().to_vec(), ShareType::Arithmetic, "l_receiptdate".to_string());
    lineitem_table.insert_column("l_receiptdate".to_string(), receiptdate);
    
    if let (Some(mut rd), Some(sd)) = (plain_receiptdate_rand, plain_shipdate_clone) {
        // receiptdate = shipdate + rand
        for (r, s) in rd.iter_mut().zip(sd.iter()) {
            *r += *s;
        }
        columns.push(Column::new("l_receiptdate".into(), rd));
    }


    let (shipinstruct, plain_shipinstruct) = gen_rand_column_u64_ring(num_rows, "l_shipinstruct".to_string(), 4, 0, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_shipinstruct".to_string(), shipinstruct);
    if let Some(data) = plain_shipinstruct {
        columns.push(Column::new("l_shipinstruct".into(), data));
    }

    let (shipmode, plain_shipmode) = gen_rand_column_u64_ring(num_rows, "l_shipmode".to_string(), 7, 0, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_shipmode".to_string(), shipmode);
    if let Some(data) = plain_shipmode {
        columns.push(Column::new("l_shipmode".into(), data));
    }

    let (comment, plain_comment) = gen_rand_column_u64_ring(num_rows, "l_comment".to_string(), 100, 0, ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("l_comment".to_string(), comment);
    if let Some(data) = plain_comment {
        columns.push(Column::new("l_comment".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    lineitem_table.insert_column("valid".to_string(), valid);
    // Note: 'valid' column is usually metadata not necessary for Polars unless needed for query logic.
    // If it's part of the standard TPCH table in this implementation, include it.
    

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((lineitem_table, Some(df)))
    } else {
        Ok((lineitem_table, None))
    }
    //let l_orderkey = gen_rand_column_u64_ring(num_rows, "l_orderkey".to_string(), 1000000000, 1, ShareType::Arithmetic, net, state);
}


pub fn gen_orders_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,    
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> { 

    let num_rows = (sf * 1500000.0) as usize;
    /*
    let schema = vec![
        ("o_orderkey".to_string(), ShareType::Arithmetic),
        ("o_custkey".to_string(), ShareType::Arithmetic),
        ("o_orderstatus".to_string(), ShareType::Arithmetic),
        ("o_totalprice".to_string(), ShareType::Arithmetic),
        ("o_orderdate".to_string(), ShareType::Arithmetic),
        ("o_orderpriority".to_string(), ShareType::Arithmetic),
        ("o_clerk".to_string(), ShareType::Arithmetic),
        ("o_shippriority".to_string(), ShareType::Arithmetic),
        ("o_comment".to_string(), ShareType::Arithmetic),
    ];
    */

    let (nets, state0,_,_) = netstate_args.split();
    let (net0, net1) = (nets[0], nets[1]);

    let partyid = state0.id;
    
    let mut orders_table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    orders_table.key_name = Some("o_orderkey".to_string());
    let (col, plain_col) = get_rand_column_pk_u64_ring(num_rows, "o_orderkey".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    orders_table.insert_column("o_orderkey".to_string(), col);
    if let Some(data) = plain_col {
        columns.push(Column::new("o_orderkey".into(), data));
    }

    let (custkey, plain_custkey) = gen_rand_column_u64_ring(num_rows, "o_custkey".to_string(), get_customer_table_size(sf), 1, ShareType::Arithmetic, net0, net1, partyid);
    orders_table.insert_column("o_custkey".to_string(), custkey);
    if let Some(data) = plain_custkey {
        columns.push(Column::new("o_custkey".into(), data));
    }

    let (orderstatus, plain_orderstatus) = gen_rand_column_u64_ring(num_rows, "o_orderstatus".to_string(), 3, 0, ShareType::Arithmetic, net0, net1, partyid);
    orders_table.insert_column("o_orderstatus".to_string(), orderstatus);
    if let Some(data) = plain_orderstatus {
        columns.push(Column::new("o_orderstatus".into(), data));
    }

    let (totalprice, plain_totalprice) = gen_rand_column_u64_ring(num_rows, "o_totalprice".to_string(), 111, 90, ShareType::Arithmetic, net0, net1, partyid);
    orders_table.insert_column("o_totalprice".to_string(), totalprice);
    if let Some(data) = plain_totalprice {
        columns.push(Column::new("o_totalprice".into(), data));
    }

    let (orderdate, plain_orderdate) = gen_rand_column_u64_ring(num_rows, "o_orderdate".to_string(), 121, 1, ShareType::Arithmetic, net0, net1, partyid);
    orders_table.insert_column("o_orderdate".to_string(), orderdate);
    if let Some(data) = plain_orderdate {
        columns.push(Column::new("o_orderdate".into(), data));
    }

    let (orderpriority, plain_orderpriority) = gen_rand_column_u64_ring(num_rows, "o_orderpriority".to_string(), 6, 1, ShareType::Arithmetic, net0, net1, partyid);
    orders_table.insert_column("o_orderpriority".to_string(), orderpriority);
    if let Some(data) = plain_orderpriority {
        columns.push(Column::new("o_orderpriority".into(), data));
    }

    let (clerk, plain_clerk) = gen_rand_column_u64_ring(num_rows, "o_clerk".to_string(), 1000000, 1, ShareType::Arithmetic, net0, net1, partyid);
    orders_table.insert_column("o_clerk".to_string(), clerk);
    if let Some(data) = plain_clerk {
        columns.push(Column::new("o_clerk".into(), data));
    }

    let (shippriority, plain_shippriority) = gen_rand_column_u64_ring(num_rows, "o_shippriority".to_string(), 1, 0, ShareType::Arithmetic, net0, net1, partyid);
    orders_table.insert_column("o_shippriority".to_string(), shippriority);
    if let Some(data) = plain_shippriority {
        columns.push(Column::new("o_shippriority".into(), data));
    }

    let (comment, plain_comment) = gen_rand_column_u64_ring(num_rows, "o_comment".to_string(), 1000000, 1, ShareType::Arithmetic, net0, net1, partyid);
    orders_table.insert_column("o_comment".to_string(), comment);
    if let Some(data) = plain_comment {
        columns.push(Column::new("o_comment".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    orders_table.insert_column("valid".to_string(), valid);
    

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((orders_table, Some(df)))
    } else {
        Ok((orders_table, None))
    }
}


pub fn gen_customer_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {

    let num_rows = (sf * 150000.0) as usize;

    /* 
    let schema = vec![
        ("c_custkey".to_string(), ShareType::Arithmetic),
        ("c_name".to_string(), ShareType::Arithmetic),
        ("c_address".to_string(), ShareType::Arithmetic),
        ("c_nationkey".to_string(), ShareType::Arithmetic),
        ("c_phone".to_string(), ShareType::Arithmetic),
        ("c_cntrycode".to_string(), ShareType::Arithmetic),
        ("c_acctbal".to_string(), ShareType::Arithmetic),
        ("c_mktsegment".to_string(), ShareType::Arithmetic),
        ("c_comment".to_string(), ShareType::Arithmetic),
    ];
    */

    let (nets, state0,_,_) = netstate_args.split();
    let (net0, net1) = (nets[0], nets[1]);
    let partyid = state0.id;

    let customer_size = get_customer_table_size(sf);

    let mut customer_table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    customer_table.key_name = Some("c_custkey".to_string());

    let (col, plain_custkey) = get_rand_column_pk_u64_ring(num_rows, "c_custkey".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    customer_table.insert_column("c_custkey".to_string(), col);
    if let Some(data) = plain_custkey {
        columns.push(Column::new("c_custkey".into(), data.clone())); // Clone needed because we use it for c_name plain as well
        columns.push(Column::new("c_name".into(), data)); // c_name is same as c_custkey
    }

    let mut name = customer_table["c_custkey"].clone();
    name.update_name("c_name".to_string());
    let c_name = name;
    customer_table.insert_column("c_name".to_string(), c_name);

    let (address, plain_address) = gen_rand_column_u64_ring(num_rows, "c_address".to_string(), customer_size, 1, ShareType::Arithmetic, net0, net1, partyid);
    customer_table.insert_column("c_address".to_string(), address);
    if let Some(data) = plain_address {
        columns.push(Column::new("c_address".into(), data));
    }

    let (nationkey, plain_nationkey) = gen_rand_column_u64_ring(num_rows, "c_nationkey".to_string(), 25, 1, ShareType::Arithmetic, net0, net1, partyid);
    customer_table.insert_column("c_nationkey".to_string(), nationkey);
    if let Some(data) = plain_nationkey {
        columns.push(Column::new("c_nationkey".into(), data));
    }

    let (acctbal, plain_acctbal) = gen_rand_column_u64_ring(num_rows, "c_acctbal".to_string(), 99999, 0, ShareType::Arithmetic, net0, net1, partyid);
    customer_table.insert_column("c_acctbal".to_string(), acctbal);
    if let Some(data) = plain_acctbal {
        columns.push(Column::new("c_acctbal".into(), data));
    }

    let (phone, plain_phone) = gen_rand_column_u64_ring(num_rows, "c_phone".to_string(), 9999999, 1000000, ShareType::Arithmetic, net0, net1, partyid);
    let plain_phone_clone = plain_phone.clone();
    customer_table.insert_column("c_phone".to_string(), phone);
    if let Some(data) = plain_phone {
        columns.push(Column::new("c_phone".into(), data));
    }

    let mut cntrycode = customer_table["c_phone"].clone() / (&RingElement(100000u64), netstate_args);
    cntrycode.update_name("c_cntrycode".to_string());
    customer_table.insert_column("c_cntrycode".to_string(), cntrycode);
    if let Some(mut cp) = plain_phone_clone {
        // Divide by 10000 to get country code
        for val in cp.iter_mut() {
            *val /= 100000;
        }
        columns.push(Column::new("c_cntrycode".into(), cp));
    }

    let (mktsegment, plain_mktsegment) = gen_rand_column_u64_ring(num_rows, "c_mktsegment".to_string(), 6, 1, ShareType::Arithmetic, net0, net1, partyid);
    customer_table.insert_column("c_mktsegment".to_string(), mktsegment);
    if let Some(data) = plain_mktsegment {
        columns.push(Column::new("c_mktsegment".into(), data));
    }

    let (comment, plain_comment) = gen_rand_column_u64_ring(num_rows, "c_comment".to_string(), 1000000, 1, ShareType::Arithmetic, net0, net1, partyid);
    customer_table.insert_column("c_comment".to_string(), comment);
    if let Some(data) = plain_comment {
        columns.push(Column::new("c_comment".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    customer_table.insert_column("valid".to_string(), valid);
    

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((customer_table, Some(df)))
    } else {
        Ok((customer_table, None))
    }
        
}

pub fn gen_part_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {

    let num_rows = (sf * 200000.0) as usize;

    /* 
    let schema = vec![
        ("p_partkey".to_string(), ShareType::Arithmetic),
        ("p_name".to_string(), ShareType::Arithmetic),
        ("p_mfgr".to_string(), ShareType::Arithmetic),
        ("p_brand".to_string(), ShareType::Arithmetic),
        ("p_type".to_string(), ShareType::Arithmetic),
        ("p_size".to_string(), ShareType::Arithmetic),
        ("p_container".to_string(), ShareType::Arithmetic),
        ("p_retailprice".to_string(), ShareType::Arithmetic),
        ("p_comment".to_string(), ShareType::Arithmetic),
    ];
    */


    let (nets, state0, _, _) = netstate_args.split();
    let (net0, net1) = (nets[0], nets[1]);
    let partyid = state0.id;

    let mut part_table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    part_table.key_name = Some("p_partkey".to_string());
    let (col, plain_partkey) = get_rand_column_pk_u64_ring(num_rows, "p_partkey".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    part_table.insert_column("p_partkey".to_string(), col);
    if let Some(data) = plain_partkey {
        columns.push(Column::new("p_partkey".into(), data));
    }

    let (name, plain_name) = gen_rand_column_u64_ring(num_rows, "p_name".to_string(), 21, 1, ShareType::Arithmetic, net0, net1, partyid);
    part_table.insert_column("p_name".to_string(), name);
    if let Some(data) = plain_name {
        columns.push(Column::new("p_name".into(), data));
    }

    let (mfgr, plain_mfgr) = gen_rand_column_u64_ring(num_rows, "p_mfgr".to_string(), 25, 1, ShareType::Arithmetic, net0, net1, partyid);
    part_table.insert_column("p_mfgr".to_string(), mfgr);
    if let Some(data) = plain_mfgr {
        columns.push(Column::new("p_mfgr".into(), data));
    }

    let (brand, plain_brand) = gen_rand_column_u64_ring(num_rows, "p_brand".to_string(), 26, 1, ShareType::Arithmetic, net0, net1, partyid);
    part_table.insert_column("p_brand".to_string(), brand);
    if let Some(data) = plain_brand {
        columns.push(Column::new("p_brand".into(), data));
    }

    let (type_, plain_type) = gen_rand_column_u64_ring(num_rows, "p_type".to_string(), 11, 1, ShareType::Arithmetic, net0, net1, partyid);
    part_table.insert_column("p_type".to_string(), type_);
    if let Some(data) = plain_type {
        columns.push(Column::new("p_type".into(), data));
    }

    let (size, plain_size) = gen_rand_column_u64_ring(num_rows, "p_size".to_string(), 51, 1, ShareType::Arithmetic, net0, net1, partyid);
    part_table.insert_column("p_size".to_string(), size);
    if let Some(data) = plain_size {
        columns.push(Column::new("p_size".into(), data));
    }

    let (container, plain_container) = gen_rand_column_u64_ring(num_rows, "p_container".to_string(), 41, 1, ShareType::Arithmetic, net0, net1, partyid);
    part_table.insert_column("p_container".to_string(), container);
    if let Some(data) = plain_container {
        columns.push(Column::new("p_container".into(), data));
    }

    let (retailprice, plain_retailprice) = gen_rand_column_u64_ring(num_rows, "p_retailprice".to_string(), 9999999, 1, ShareType::Arithmetic, net0, net1, partyid);
    part_table.insert_column("p_retailprice".to_string(), retailprice);
    if let Some(data) = plain_retailprice {
        columns.push(Column::new("p_retailprice".into(), data));
    }

    let (comment, plain_comment) = gen_rand_column_u64_ring(num_rows, "p_comment".to_string(), 1000000, 1, ShareType::Arithmetic, net0, net1, partyid);
    part_table.insert_column("p_comment".to_string(), comment);
    if let Some(data) = plain_comment {
        columns.push(Column::new("p_comment".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    part_table.insert_column("valid".to_string(), valid);
    

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((part_table, Some(df)))
    } else {
        Ok((part_table, None))
    }
}

pub fn gen_supplier_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> { 

    let num_rows = (sf * 10000.0) as usize;

    /* 
    let schema = vec![
        ("s_suppkey".to_string(), ShareType::Arithmetic),
        ("s_name".to_string(), ShareType::Arithmetic),
        ("s_address".to_string(), ShareType::Arithmetic),
        ("s_nationkey".to_string(), ShareType::Arithmetic),
        ("s_phone".to_string(), ShareType::Arithmetic),
        ("s_acctbal".to_string(), ShareType::Arithmetic),
        ("s_comment".to_string(), ShareType::Arithmetic),
    ];
    */

    /*
        auto suppkey = counterColumn(S);

        // Foreign key to Nation
        auto nationkey = randomColumn(S, 0, nationSize());

        // -1000..10000
        auto acctbal = randomColumn(S, -1000, 10000 + 1);

        // Integer representation for name
        // Value calculation is arbitrary, it's solely to obtain unique values
        // that differ from the primary key
        auto name = suppkey * 2 + 1;

        // 4-bit comment
        auto comment = randomColumn(S, 0, (1 << COMMENT_BITS));

        // Integer representation for address in 0..10000
        auto address = randomColumn(S, 0, 100000 + 1);

        // Integer representation for phone number in 1B..2B
        auto phone = randomColumn(S, 1000000000, 2000000000 + 1);
    */

    let (nets, state0, _, _) = netstate_args.split();
    let (net0, net1) = (nets[0], nets[1]);

    let partyid = state0.id;

    let mut supplier_table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    supplier_table.key_name = Some("s_suppkey".to_string());

    let (suppkey, plain_suppkey) = get_rand_column_pk_u64_ring(num_rows, "s_suppkey".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    supplier_table.insert_column("s_suppkey".to_string(), suppkey);
    let plain_suppkey_clone = plain_suppkey.clone();
    if let Some(data) = plain_suppkey {
        columns.push(Column::new("s_suppkey".into(), data));
    }

    let mut name = supplier_table["s_suppkey"].clone() * RingElement(2u64) + (RingElement(1u64), &partyid);
    name.update_name("s_name".to_string());
    let s_name = name;
    supplier_table.insert_column("s_name".to_string(), s_name);

    if let Some(mut sk) = plain_suppkey_clone {
        // s_name = s_suppkey * 2 + 1
        for val in sk.iter_mut() {
            *val = *val * 2 + 1;
        }
        columns.push(Column::new("s_name".into(), sk));
    }

    let (address, plain_address) = gen_rand_column_u64_ring(num_rows, "s_address".to_string(), 100000, 1, ShareType::Arithmetic, net0, net1, partyid);
    supplier_table.insert_column("s_address".to_string(), address);
    if let Some(data) = plain_address {
        columns.push(Column::new("s_address".into(), data));
    }

    let (phone, plain_phone) = gen_rand_column_u64_ring(num_rows, "s_phone".to_string(), 2000000000, 1000000000, ShareType::Arithmetic, net0, net1, partyid);
    supplier_table.insert_column("s_phone".to_string(), phone);
    if let Some(data) = plain_phone {
        columns.push(Column::new("s_phone".into(), data));
    }

    let (nationkey, plain_nationkey) = gen_rand_column_u64_ring(num_rows, "s_nationkey".to_string(), 25, 1, ShareType::Arithmetic, net0, net1, partyid);
    supplier_table.insert_column("s_nationkey".to_string(), nationkey);
    if let Some(data) = plain_nationkey {
        columns.push(Column::new("s_nationkey".into(), data));
    }

    let (acctbal, plain_acctbal) = gen_rand_column_u64_ring(num_rows, "s_acctbal".to_string(), 10000, 0, ShareType::Arithmetic, net0, net1, partyid);
    supplier_table.insert_column("s_acctbal".to_string(), acctbal);
    if let Some(data) = plain_acctbal {
        columns.push(Column::new("s_acctbal".into(), data));
    }

    let (comment, plain_comment) = gen_rand_column_u64_ring(num_rows, "s_comment".to_string(), 1000000, 1, ShareType::Arithmetic, net0, net1, partyid);
    supplier_table.insert_column("s_comment".to_string(), comment);
    if let Some(data) = plain_comment {
        columns.push(Column::new("s_comment".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    supplier_table.insert_column("valid".to_string(), valid);
    

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((supplier_table, Some(df)))
    } else {
        Ok((supplier_table, None))
    }
}

pub fn gen_partsupp_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {

    let num_rows = (sf * 800000.0) as usize;

    let schema = vec![
        ("ps_partkey".to_string(), ShareType::Arithmetic),   
        ("ps_suppkey".to_string(), ShareType::Arithmetic),
        ("ps_availqty".to_string(), ShareType::Arithmetic),
        ("ps_supplycost".to_string(), ShareType::Arithmetic),
        ("ps_comment".to_string(), ShareType::Arithmetic),
    ];

    let (nets, state0, _, _) = netstate_args.split();
    let (net0, net1) = (nets[0], nets[1]);

    let partyid = state0.id;

    //let partsupp_size = get_partsupp_table_size(sf);

    let mut partsupp_table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    partsupp_table.key_name = Some("ps_partkey".to_string());
    let (col, plain_partkey) = get_rand_column_pk_u64_ring(num_rows, "ps_partkey".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    partsupp_table.insert_column("ps_partkey".to_string(), col);
    if let Some(data) = plain_partkey {
        columns.push(Column::new("ps_partkey".into(), data));
    }

    let (col, plain_suppkey) = get_rand_column_pk_u64_ring(num_rows, "ps_suppkey".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    partsupp_table.insert_column("ps_suppkey".to_string(), col);
    if let Some(data) = plain_suppkey {
        columns.push(Column::new("ps_suppkey".into(), data));
    }

    for i in 2..schema.len() {
        let (name, datatype) = &schema[i];
        let (col, plain_col) = gen_rand_column_u64_ring(num_rows, name.clone(), 1000000, 1, datatype.clone(), net0, net1, partyid);
        partsupp_table.insert_column(name.clone(), col);
        if let Some(data) = plain_col {
            columns.push(Column::new(name.into(), data));
        }
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    partsupp_table.insert_column("valid".to_string(), valid);

    
    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((partsupp_table, Some(df)))
    } else {
        Ok((partsupp_table, None))
    }
}

pub fn gen_nation_table<N: Network>(
    netstate_args: &mut NetStateArgs<N>
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {

    let num_rows = 25 as usize;

    /* 
    let schema = vec![
        ("n_nationkey".to_string(), ShareType::Arithmetic),
        ("n_name".to_string(), ShareType::Arithmetic),
        ("n_regionkey".to_string(), ShareType::Arithmetic),
        ("n_comment".to_string(), ShareType::Arithmetic),
    ];
    */

    let (nets, state0, _, _) = netstate_args.split();
    let (net0, net1) = (nets[0], nets[1]);

    let partyid = state0.id;

    let mut nation_table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    nation_table.key_name = Some("n_nationkey".to_string());

    let (col, plain_nationkey) = get_rand_column_pk_u64_ring(num_rows, "n_nationkey".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    nation_table.insert_column("n_nationkey".to_string(), col);
    if let Some(data) = plain_nationkey {
        columns.push(Column::new("n_nationkey".into(), data.clone()));
        
        let mut n_name = data;
        for val in n_name.iter_mut() {
            *val = *val * 2 + 1;
        }
        columns.push(Column::new("n_name".into(), n_name));
    }

    let mut name = nation_table["n_nationkey"].clone() * RingElement(2u64) + (RingElement(1u64), &partyid);
    name.update_name("n_name".to_string());
    let n_name = name;
    nation_table.insert_column("n_name".to_string(), n_name);

    let (n_regionkey, plain_regionkey) = gen_rand_column_u64_ring(num_rows, "n_regionkey".to_string(), get_region_table_size(), 1, ShareType::Arithmetic, net0, net1, partyid);
    nation_table.insert_column("n_regionkey".to_string(), n_regionkey);
    if let Some(data) = plain_regionkey {
        columns.push(Column::new("n_regionkey".into(), data));
    }

    let (n_comment, plain_comment) = gen_rand_column_u64_ring(num_rows, "n_comment".to_string(), 1000000, 1, ShareType::Arithmetic, net0, net1, partyid);
    nation_table.insert_column("n_comment".to_string(), n_comment);
    if let Some(data) = plain_comment {
        columns.push(Column::new("n_comment".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    nation_table.insert_column("valid".to_string(), valid);
    

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((nation_table, Some(df)))
    } else {
        Ok((nation_table, None))
    }
}

pub fn gen_region_table<N: Network>(
    netstate_args: &mut NetStateArgs<N>
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {

    let num_rows = 5 as usize;

    /* 
    let schema = vec![
        ("r_regionkey".to_string(), ShareType::Arithmetic),
        ("r_name".to_string(), ShareType::Arithmetic),
        ("r_comment".to_string(), ShareType::Arithmetic),
    ];
    */

    let (nets, state0, _, _) = netstate_args.split();
    let (net0, net1) = (nets[0], nets[1]);

    let partyid = state0.id;

    let mut region_table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    region_table.key_name = Some("r_regionkey".to_string());

    let (col, plain_regionkey) = get_rand_column_pk_u64_ring(num_rows, "r_regionkey".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    region_table.insert_column("r_regionkey".to_string(), col);
    if let Some(data) = plain_regionkey {
        columns.push(Column::new("r_regionkey".into(), data.clone()));
        
        let mut r_name = data;
        for val in r_name.iter_mut() {
            *val = *val * 2 + 1;
        }
        columns.push(Column::new("r_name".into(), r_name));
    }

    let mut name = region_table["r_regionkey"].clone() * RingElement(2u64) + (RingElement(1u64), &partyid);
    name.update_name("r_name".to_string());
    let r_name = name;
    region_table.insert_column("r_name".to_string(), r_name);

    let (comment, plain_comment) = gen_rand_column_u64_ring(num_rows, "r_comment".to_string(), 1000000, 1, ShareType::Arithmetic, net0, net1, partyid);
    region_table.insert_column("r_comment".to_string(), comment);
    if let Some(data) = plain_comment {
        columns.push(Column::new("r_comment".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, net0, net1, partyid);
    region_table.insert_column("valid".to_string(), valid);
    

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((region_table, Some(df)))
    } else {
        Ok((region_table, None))
    }
}
