
use std::ops::{Index, IndexMut};

// “[orderkey]" binary shared column
// "orderkey" arthmetic shared column

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareType {
    Arithmetic,
    Binary,
    PlainText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareColumn<T> {
    data: Vec<T>,
    datatype: ShareType,
    name: String,
}

impl<T> ShareColumn<T> {
    pub fn new(data: Vec<T>, datatype: ShareType, name: String) -> Self {
        Self {
            data,
            datatype,
            name,
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn get_data(&self) -> &[T] {
        &self.data
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
    
    pub fn get_data_mut(&mut self) -> &mut Vec<T> {
        &mut self.data
    }

    pub fn update_data(&mut self, new_data: Vec<T>) {
        self.data = new_data;
    }

    pub fn update_datatype(&mut self, new_datatype: ShareType) {
        self.datatype = new_datatype;
    }

    pub fn update_name(&mut self, new_name: String) {
        self.name = new_name;
    }

    //Keep the first n rows
    pub fn truncate_first(&mut self, n: usize) {
        self.data.truncate(n);
    }

    //Keep the last n rows
    pub fn truncate_last(&mut self, n: usize) {
        let len = self.data.len();
        self.data.drain(0..(len - n));
    }
    
    
}

impl<T> Index<usize> for ShareColumn<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T> IndexMut<usize> for ShareColumn<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}
