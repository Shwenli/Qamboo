use communication::rep3::id::PartyID;
use protocols::protocols::rep3_ring::Rep3RingShare;
use net::Network;
use table::share_table::ShareTable;
use table::share_column::{ShareColumn, ShareType};
use table::NetStateArgs;
use crate::{gen_rand_column_u64_ring, gen_valid_column_u64_ring};
use polars::prelude::*;

// Scale factor definitions for secrecy tables
// SF1: ~5M rows for SecrecyR
const SECRECY_R_SIZE: f32 = 2_000_000.0;
const SECRECY_S_SIZE: f32 = 3_000_000.0;
const SECRECY_T1_SIZE: f32 = 2_000_000.0;
const SECRECY_T2_SIZE: f32 = 1_000_000.0;
const SECRECY_T3_SIZE: f32 = 1_000_000.0;
const DIAGNOSIS_SIZE: f32 = 3_000_000.0;
const MEDICATION_SIZE: f32 = 2_000_000.0;
const STANDARD_SIZE: f32 = 5_000_000.0;

// Constants for data generation
const MAX_TIME: u64 = 10000;
const N_DISEASE: u64 = 100;
const N_DISEASE_CLASS: u64 = 16;
const MAX_COST: u64 = 10000;
const BASE_YEAR: u64 = 2020;
const NUM_YEARS: u64 = 10;
const MIN_CREDIT: u64 = 300;
const MAX_CREDIT: u64 = 850;

// Comorbidity query constants
const COHORT_MULTIPLIER: f32 = 500_000.0;
const DIAGNOSIS_MULTIPLIER: f32 = COHORT_MULTIPLIER * 10.0;
const DIAGNOSIS_TYPE_COUNT: u64 = 1000;

pub fn get_secrecy_r_size(sf: f32) -> u64 {
    (sf * SECRECY_R_SIZE) as u64
}

pub fn get_secrecy_s_size(sf: f32) -> u64 {
    (sf * SECRECY_S_SIZE) as u64
}

pub fn get_secrecy_t1_size(sf: f32) -> u64 {
    (sf * SECRECY_T1_SIZE) as u64
}

pub fn get_secrecy_t2_size(sf: f32) -> u64 {
    (sf * SECRECY_T2_SIZE) as u64
}

pub fn get_secrecy_t3_size(sf: f32) -> u64 {
    (sf * SECRECY_T3_SIZE) as u64
}

pub fn get_diagnosis_size(sf: f32) -> u64 {
    (sf * DIAGNOSIS_SIZE) as u64
}

pub fn get_medication_size(sf: f32) -> u64 {
    (sf * MEDICATION_SIZE) as u64
}

pub fn get_standard_size(sf: f32) -> u64 {
    (sf * STANDARD_SIZE) as u64
}

pub fn get_cohort_size(sf: f32) -> u64 {
    (sf * COHORT_MULTIPLIER) as u64
}

pub fn get_diagnosis_comorbidity_size(sf: f32) -> u64 {
    (sf * DIAGNOSIS_MULTIPLIER) as u64
}

/// Generate SecrecyR table: (id, ak)
pub fn gen_secrecy_r_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * SECRECY_R_SIZE) as usize;
    let sqrt_s = (num_rows as f64).sqrt() as u64;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    table.key_name = Some("id".to_string());
    let mut columns: Vec<Column> = Vec::new();

    // id column: random in range [0, sqrt(S))
    let (id_col, plain_id) = gen_rand_column_u64_ring(
        num_rows,
        "id".to_string(),
        sqrt_s,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("id".to_string(), id_col);
    if let Some(data) = plain_id {
        columns.push(Column::new("id".into(), data));
    }

    // ak column: random in range [0, sqrt(S))
    let (ak_col, plain_ak) = gen_rand_column_u64_ring(
        num_rows,
        "ak".to_string(),
        sqrt_s,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("ak".to_string(), ak_col);
    if let Some(data) = plain_ak {
        columns.push(Column::new("ak".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate Cohort table for comorbidity query: (pid)
/// pid is sequential: 0, 1, 2, ..., num_rows-1
pub fn gen_cohort_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * COHORT_MULTIPLIER) as usize;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    table.key_name = Some("pid".to_string());
    let mut columns: Vec<Column> = Vec::new();

    // pid column: sequential 0, 1, 2, ..., num_rows-1
    let pid_data: Vec<u64> = (0..num_rows as u64).collect();
    let pid_data_clone = pid_data.clone();
    
    let pid_col = match partyid {
        PartyID::ID0 => {
            use rand::thread_rng;
            use protocols::protocols::rep3_ring;
            use algebra::ring::ring_impl::RingElement;
            
            let data_ring: Vec<_> = pid_data.into_iter().map(RingElement).collect();
            let data_ring_share = rep3_ring::share_ring_elements(&data_ring, &mut thread_rng());
            
            use communication::rep3::multinet_impl::send_many_multinet;
            let _ = send_many_multinet(nets, PartyID::ID1, &data_ring_share[1]);
            let _ = send_many_multinet(nets, PartyID::ID2, &data_ring_share[2]);
            
            ShareColumn::new(data_ring_share[0].clone(), ShareType::Arithmetic, "pid".to_string())
        }
        PartyID::ID1 | PartyID::ID2 => {
            use communication::rep3::multinet_impl::recv_many_multinet;
            let data_ring_share: Vec<Rep3RingShare<u64>> = recv_many_multinet(nets, PartyID::ID0)
                .unwrap_or_else(|e| panic!("gen_cohort_table: Recv failed: {:?}", e));
            ShareColumn::new(data_ring_share, ShareType::Arithmetic, "pid".to_string())
        }
    };
    
    table.insert_column("pid".to_string(), pid_col);
    if partyid == PartyID::ID0 {
        columns.push(Column::new("pid".into(), pid_data_clone));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate Diagnosis table for comorbidity query: (pid, diag)
/// pid: random in [0, 2 * cohort_size) - to allow for pids not in Cohort
/// diag: random in [0, DIAGNOSIS_TYPE_COUNT)
pub fn gen_diagnosis_comorbidity_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * DIAGNOSIS_MULTIPLIER) as usize;
    let max_pid = 2 * (sf * COHORT_MULTIPLIER) as u64; // 2 * cohort_size

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    // pid column: [0, 2 * cohort_size)
    let (pid_col, plain_pid) = gen_rand_column_u64_ring(
        num_rows,
        "pid".to_string(),
        max_pid,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("pid".to_string(), pid_col);
    if let Some(data) = plain_pid {
        columns.push(Column::new("pid".into(), data));
    }

    // diag column: [0, DIAGNOSIS_TYPE_COUNT)
    let (diag_col, plain_diag) = gen_rand_column_u64_ring(
        num_rows,
        "diag".to_string(),
        DIAGNOSIS_TYPE_COUNT,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("diag".to_string(), diag_col);
    if let Some(data) = plain_diag {
        columns.push(Column::new("diag".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate SecrecyS table: (id)
pub fn gen_secrecy_s_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * SECRECY_S_SIZE) as usize;
    let sqrt_s = (num_rows as f64).sqrt() as u64;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    table.key_name = Some("id".to_string());
    let mut columns: Vec<Column> = Vec::new();

    // id column
    let (id_col, plain_id) = gen_rand_column_u64_ring(
        num_rows,
        "id".to_string(),
        sqrt_s,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("id".to_string(), id_col);
    if let Some(data) = plain_id {
        columns.push(Column::new("id".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate SecrecyT1 table: (person, coinsurance, state)
pub fn gen_secrecy_t1_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * SECRECY_T1_SIZE) as usize;
    let sqrt_t1 = (num_rows as f64).sqrt() as u64;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    // person column
    let (person_col, plain_person) = gen_rand_column_u64_ring(
        num_rows,
        "person".to_string(),
        sqrt_t1,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("person".to_string(), person_col);
    if let Some(data) = plain_person {
        columns.push(Column::new("person".into(), data));
    }

    // coinsurance column: [0, 100]
    let (coinsurance_col, plain_coinsurance) = gen_rand_column_u64_ring(
        num_rows,
        "coinsurance".to_string(),
        100,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("coinsurance".to_string(), coinsurance_col);
    if let Some(data) = plain_coinsurance {
        columns.push(Column::new("coinsurance".into(), data));
    }

    // state column: [0, 50]
    let (state_col, plain_state) = gen_rand_column_u64_ring(
        num_rows,
        "state".to_string(),
        50,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("state".to_string(), state_col);
    if let Some(data) = plain_state {
        columns.push(Column::new("state".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate SecrecyT2 table: (person, disease, cost)
pub fn gen_secrecy_t2_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * SECRECY_T2_SIZE) as usize;
    let sqrt_t2 = (num_rows as f64).sqrt() as u64;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    // person column
    let (person_col, plain_person) = gen_rand_column_u64_ring(
        num_rows,
        "person".to_string(),
        sqrt_t2,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("person".to_string(), person_col);
    if let Some(data) = plain_person {
        columns.push(Column::new("person".into(), data));
    }

    // disease column: [0, N_DISEASE)
    let (disease_col, plain_disease) = gen_rand_column_u64_ring(
        num_rows,
        "disease".to_string(),
        N_DISEASE,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("disease".to_string(), disease_col);
    if let Some(data) = plain_disease {
        columns.push(Column::new("disease".into(), data));
    }

    // cost column: [0, MAX_COST)
    let (cost_col, plain_cost) = gen_rand_column_u64_ring(
        num_rows,
        "cost".to_string(),
        MAX_COST,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("cost".to_string(), cost_col);
    if let Some(data) = plain_cost {
        columns.push(Column::new("cost".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate SecrecyT3 table: (disease, class_)
pub fn gen_secrecy_t3_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * SECRECY_T3_SIZE) as usize;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    // disease column: [0, N_DISEASE)
    let (disease_col, plain_disease) = gen_rand_column_u64_ring(
        num_rows,
        "disease".to_string(),
        N_DISEASE,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("disease".to_string(), disease_col);
    if let Some(data) = plain_disease {
        columns.push(Column::new("disease".into(), data));
    }

    // class_ column: [0, N_DISEASE_CLASS)
    let (class_col, plain_class) = gen_rand_column_u64_ring(
        num_rows,
        "class_".to_string(),
        N_DISEASE_CLASS,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("class_".to_string(), class_col);
    if let Some(data) = plain_class {
        columns.push(Column::new("class_".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate Diagnosis table: (pid, time, diagnosis)
pub fn gen_diagnosis_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * DIAGNOSIS_SIZE) as usize;
    let num_patients = num_rows / 2;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    // pid column: [0, num_patients)
    let (pid_col, plain_pid) = gen_rand_column_u64_ring(
        num_rows,
        "pid".to_string(),
        num_patients as u64,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("pid".to_string(), pid_col);
    if let Some(data) = plain_pid {
        columns.push(Column::new("pid".into(), data));
    }

    // time column: [0, MAX_TIME)
    let (time_col, plain_time) = gen_rand_column_u64_ring(
        num_rows,
        "time".to_string(),
        MAX_TIME,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("time".to_string(), time_col);
    if let Some(data) = plain_time {
        columns.push(Column::new("time".into(), data));
    }

    // diagnosis column: [0, 10)
    let (diag_col, plain_diag) = gen_rand_column_u64_ring(
        num_rows,
        "diag".to_string(),
        10,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("diag".to_string(), diag_col);
    if let Some(data) = plain_diag {
        columns.push(Column::new("diag".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate Medication table: (pid, time, med)
pub fn gen_medication_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * MEDICATION_SIZE) as usize;
    let num_patients = num_rows / 2;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    // pid column: [0, num_patients)
    let (pid_col, plain_pid) = gen_rand_column_u64_ring(
        num_rows,
        "pid".to_string(),
        num_patients as u64,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("pid".to_string(), pid_col);
    if let Some(data) = plain_pid {
        columns.push(Column::new("pid".into(), data));
    }

    // time column: [0, MAX_TIME)
    let (time_col, plain_time) = gen_rand_column_u64_ring(
        num_rows,
        "time".to_string(),
        MAX_TIME,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("time".to_string(), time_col);
    if let Some(data) = plain_time {
        columns.push(Column::new("time".into(), data));
    }

    // med column: [0, 10)
    let (med_col, plain_med) = gen_rand_column_u64_ring(
        num_rows,
        "med".to_string(),
        10,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("med".to_string(), med_col);
    if let Some(data) = plain_med {
        columns.push(Column::new("med".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate Password table: (id, pwd)
pub fn gen_password_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * STANDARD_SIZE) as usize;
    let num_users = num_rows / 10;
    let num_passwd = num_rows / 50;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    table.key_name = Some("id".to_string());
    let mut columns: Vec<Column> = Vec::new();

    // id column: [0, num_users)
    let (id_col, plain_id) = gen_rand_column_u64_ring(
        num_rows,
        "id".to_string(),
        num_users as u64,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("id".to_string(), id_col);
    if let Some(data) = plain_id {
        columns.push(Column::new("id".into(), data));
    }

    // pwd column: [0, num_passwd)
    let (pwd_col, plain_pwd) = gen_rand_column_u64_ring(
        num_rows,
        "pwd".to_string(),
        num_passwd as u64,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("pwd".to_string(), pwd_col);
    if let Some(data) = plain_pwd {
        columns.push(Column::new("pwd".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate Taxi table: (company, fare, type)
pub fn gen_taxi_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * STANDARD_SIZE) as usize;
    let num_companies: u64 = 1000;
    let max_fare: u64 = 100;
    let types: u64 = 20;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    // company column: biased distribution (company^2 % num_companies)
    // For simplicity, we generate random and apply bias in plaintext for ID0
    let (company_col, plain_company) = gen_rand_column_u64_ring(
        num_rows,
        "company".to_string(),
        num_companies,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("company".to_string(), company_col);
    if let Some(mut data) = plain_company {
        // Apply bias: (company * company) % num_companies
        for val in data.iter_mut() {
            *val = (*val * *val) % num_companies;
        }
        columns.push(Column::new("company".into(), data));
    }

    // fare column: [0, max_fare)
    let (fare_col, plain_fare) = gen_rand_column_u64_ring(
        num_rows,
        "fare".to_string(),
        max_fare,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("fare".to_string(), fare_col);
    if let Some(data) = plain_fare {
        columns.push(Column::new("fare".into(), data));
    }

    // type column: [0, types) - ~5% airport types (0-1)
    let (type_col, plain_type) = gen_rand_column_u64_ring(
        num_rows,
        "type".to_string(),
        types,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("type".to_string(), type_col);
    if let Some(data) = plain_type {
        columns.push(Column::new("type".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}

/// Generate CreditScore table: (uid, agency, year, cs)
pub fn gen_credit_score_table<N: Network>(
    sf: f32,
    netstate_args: &mut NetStateArgs<N>,
) -> eyre::Result<(ShareTable<Rep3RingShare<u64>>, Option<DataFrame>)> {
    let num_rows = (sf * STANDARD_SIZE) as usize;
    let num_agencies: u64 = 10;
    // Not everyone has a score from every agency
    let num_users = 2 * num_rows as u64 / num_agencies;

    let (nets, states) = netstate_args.split();
    let partyid = states[0].id;

    let mut table = ShareTable::<Rep3RingShare<u64>>::new();
    let mut columns: Vec<Column> = Vec::new();

    // uid column: [0, num_users)
    let (uid_col, plain_uid) = gen_rand_column_u64_ring(
        num_rows,
        "uid".to_string(),
        num_users,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("uid".to_string(), uid_col);
    if let Some(data) = plain_uid {
        columns.push(Column::new("uid".into(), data));
    }

    // agency column: [0, num_agencies)
    let (agency_col, plain_agency) = gen_rand_column_u64_ring(
        num_rows,
        "agency".to_string(),
        num_agencies,
        0,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("agency".to_string(), agency_col);
    if let Some(data) = plain_agency {
        columns.push(Column::new("agency".into(), data));
    }

    // year column: [BASE_YEAR, BASE_YEAR + NUM_YEARS)
    let (year_col, plain_year) = gen_rand_column_u64_ring(
        num_rows,
        "year".to_string(),
        BASE_YEAR + NUM_YEARS,
        BASE_YEAR,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("year".to_string(), year_col);
    if let Some(data) = plain_year {
        columns.push(Column::new("year".into(), data));
    }

    // cs (credit score) column: [MIN_CREDIT, MAX_CREDIT)
    let (cs_col, plain_cs) = gen_rand_column_u64_ring(
        num_rows,
        "cs".to_string(),
        MAX_CREDIT,
        MIN_CREDIT,
        ShareType::Arithmetic,
        nets,
        partyid,
    );
    table.insert_column("cs".to_string(), cs_col);
    if let Some(data) = plain_cs {
        columns.push(Column::new("cs".into(), data));
    }

    let valid = gen_valid_column_u64_ring(num_rows, "valid".to_string(), ShareType::Arithmetic, nets, partyid);
    table.insert_column("valid".to_string(), valid);

    if partyid == PartyID::ID0 {
        let df = DataFrame::new(columns).expect("Failed to create DataFrame");
        Ok((table, Some(df)))
    } else {
        Ok((table, None))
    }
}
