

pub fn get_task_chunks<T>(
    data: &[T],
    data_len: usize,
    net_num: usize,
) -> eyre::Result<Vec<&[T]>> {

    let mut chunks = Vec::with_capacity(net_num);
    let base_chunk_size = data_len / net_num;
    let remainder = data_len % net_num;
    let mut start = 0;

    for i in 0..net_num {
        let length = base_chunk_size + if i < remainder { 1 } else { 0 };
        let end = start + length;
        chunks.push(&data[start..end]);
        start = end;
    }

    Ok(chunks)
}

pub fn get_mut_task_chunks<T>(
    task: &mut [T],
    data_len: usize,
    net_num: usize,
) -> eyre::Result<Vec<&mut [T]>> {

    let mut chunks = Vec::with_capacity(net_num);
    let base_chunk_size = data_len / net_num;
    let remainder = data_len % net_num;
    
    let mut remaining_slice = task;

    for i in 0..net_num {
        let length = base_chunk_size + if i < remainder { 1 } else { 0 };
        let (chunk, rest) = remaining_slice.split_at_mut(length);
        chunks.push(chunk);
        remaining_slice = rest;
    }

    Ok(chunks)
}




