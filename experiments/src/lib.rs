pub mod tpch_database_gen;
pub mod secrecy_database_gen;
pub mod net_statistics;
pub mod timer;


use itertools::izip;
use communication::rep3::id::PartyID;
use communication::rep3::multinet_impl::{send_many_multinet, recv_many_multinet};
use protocols::protocols::rep3_ring::{self, Rep3RingShare};
use net::Network;
use table::share_column::{ShareColumn, ShareType};
use table::NetStateArgs;
use algebra::ring::ring_impl::RingElement;
use primitives::transform::a2b_many_multithreads;
use rand::{thread_rng,Rng};

/// Generate a column of 1, 2, ..., num_rows as u64 values for primary key column. 
pub fn gen_rand_column_data_pk_u64(num_rows: usize) -> Vec<u64> {
    let mut data = vec![];

    for i in 0..num_rows {
        let val = i as u64 + 1;
        data.push(val);
    }

    data
}

/// Generate a column of random u64 values in the range [min_val, max_val) 
pub fn gen_rand_column_data_u64(num_rows: usize, max_val: u64, min_val: u64) -> Vec<u64> {

    let range = max_val - min_val;
    let mut rng = thread_rng();

    let data: Vec<u64> = (0..num_rows).map(|_| rng.r#gen::<u64>()%range + min_val).collect();

    data
}

pub fn gen_valid_column_data_u64(num_rows: usize) -> Vec<u64> {

    let data = vec![1;num_rows];

    data
}

pub fn gen_valid_column_u64_ring<N: Network>(
    num_rows: usize,
    name: String,
    datatype: ShareType,
    nets: &[&N],
    partyid: PartyID,
) -> ShareColumn<Rep3RingShare<u64>>
{
    match partyid{
        PartyID::ID0=>{
            let data = gen_valid_column_data_u64(num_rows);

            let data_ring = izip!(data).map(|val| RingElement(val)).collect::<Vec<_>>();

            let mut rng = thread_rng();
            let data_ring_share = rep3_ring::share_ring_elements(&data_ring, &mut rng);
            
            let _ = send_many_multinet(nets, PartyID::ID1, &data_ring_share[1]);
            let _ = send_many_multinet(nets, PartyID::ID2, &data_ring_share[2]);

            let share_col = ShareColumn::new(data_ring_share[0].clone(), datatype, name);

            share_col
        }
        PartyID::ID1=>{
            let data_ring_share_1: Vec<Rep3RingShare<u64>> = recv_many_multinet(nets, PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let share_col = ShareColumn::new(data_ring_share_1, datatype, name);

            share_col
        }
        PartyID::ID2=>{
            let data_ring_share_2: Vec<Rep3RingShare<u64>> = recv_many_multinet(nets, PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let share_col = ShareColumn::new(data_ring_share_2, datatype, name);

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
    nets: &[&N],
    partyid: PartyID,
) -> (ShareColumn<Rep3RingShare<u64>>, Option<Vec<u64>>)
{
    match  partyid{
        PartyID::ID0=>{
            let mut rng = thread_rng();

            let data = gen_rand_column_data_u64(num_rows, max_val, min_val);
            let data_clone = data.clone();

            let data_ring = izip!(data).map(|val| RingElement(val)).collect::<Vec<_>>();

            let data_ring_share = rep3_ring::share_ring_elements(&data_ring, &mut rng);
            
            let _ = send_many_multinet(nets, PartyID::ID1, &data_ring_share[1]);
            let _ = send_many_multinet(nets, PartyID::ID2, &data_ring_share[2]);

            let share_col = ShareColumn::new(data_ring_share[0].clone(), datatype, name);

            (share_col, Some(data_clone))

        }
        PartyID::ID1=>{
            let data_ring_share_1: Vec<Rep3RingShare<u64>> = recv_many_multinet(nets, PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let share_col = ShareColumn::new(data_ring_share_1, datatype, name);
            (share_col, None)

        }
        PartyID::ID2=>{
            let data_ring_share_2: Vec<Rep3RingShare<u64>> = recv_many_multinet(nets, PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let share_col = ShareColumn::new(data_ring_share_2, datatype, name);

            (share_col, None)
        }
        
    }
}

pub fn gen_rand_column_pk_u64_ring<N: Network>(
    num_rows: usize,
    name: String,
    datatype: ShareType,
    nets: &[&N],
    partyid: PartyID
) -> (ShareColumn<Rep3RingShare<u64>>, Option<Vec<u64>>)
{
    match  partyid{
        PartyID::ID0=>{
            let data = gen_rand_column_data_pk_u64(num_rows);
            let data_clone = data.clone();

            let data_ring = izip!(data).map(|val| RingElement(val)).collect::<Vec<_>>();

            let mut rng = thread_rng();
            let data_ring_share = rep3_ring::share_ring_elements(&data_ring, &mut rng);

            let _ = send_many_multinet(nets, PartyID::ID1, &data_ring_share[1]);
            let _ = send_many_multinet(nets, PartyID::ID2, &data_ring_share[2]);

            let share_col = ShareColumn::new(data_ring_share[0].clone(), datatype, name);

            (share_col, Some(data_clone))

        }
        PartyID::ID1=>{
            let data_ring_share_1: Vec<Rep3RingShare<u64>> = recv_many_multinet(nets, PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let share_col = ShareColumn::new(data_ring_share_1, datatype, name);
            (share_col, None)

        }
        PartyID::ID2=>{
            let data_ring_share_2: Vec<Rep3RingShare<u64>> = recv_many_multinet(nets, PartyID::ID0).unwrap_or_else(|e| panic!("gen_valid_column_u64_ring: Recv failed: {:?}", e));
            let share_col = ShareColumn::new(data_ring_share_2, datatype, name);

            (share_col, None)
        }
        
    }
}

pub fn convert_binary_from_arithmetic<N: Network>(
    ori_col: &ShareColumn<Rep3RingShare<u64>>,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<ShareColumn<Rep3RingShare<u64>>> { 

    let arithematic_data = ori_col.get_data();
    let (nets, states) = netstate_args.split();
    let binary_data = a2b_many_multithreads(arithematic_data, nets, states)?;
    let new_name = format!("[{}]", ori_col.get_name());
    let share_col = ShareColumn::new(binary_data, ShareType::Binary, new_name);

    Ok(share_col)
}