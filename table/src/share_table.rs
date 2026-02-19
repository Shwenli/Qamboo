use crate::share_column::ShareColumn;
use std::ops::Index;
use std::ops::IndexMut;
use indexmap::IndexMap;
use communication::rep3::id::PartyID;
use random::rep3::Rep3State;



#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareTable<T> {
    pub key_name: Option<String>,
    pub schema: IndexMap<String, ShareColumn<T>>,
}

impl<T> ShareTable<T> {

    pub fn new() -> Self {
        let schema = IndexMap::new();
        
        Self {
            key_name: None,
            schema,
        }
    }

    // Get the index of a column by its name (panic if not found)
    pub fn get_column_index(&self, name: &str) -> usize {
        self.schema.get_index_of(name).unwrap_or_else(||panic!("Column '{}' not found", name))
    }

    // Get a column by its index (panic if index out of bounds)
    pub fn get_column_by_index(&self, index: usize) -> &ShareColumn<T> {
        self.schema
            .get_index(index)
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("Column index {} out of bounds", index))
    }

    // Optional version: Get a column by its index (return Option)
    pub fn get_column_by_index_opt(&self, index: usize) -> Option<&ShareColumn<T>> {
        self.schema.get_index(index).map(|(_, v)| v)
    }

    // Get a mutable reference to a column by its index (panic if index out of bounds)
    pub fn get_column_by_index_mut(&mut self, index: usize) -> &mut ShareColumn<T> {
        self.schema
            .get_index_mut(index)
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("Column index {} out of bounds", index))
    }

    // Get a mutable reference to a column by its index (return Option)
    pub fn get_column_by_index_mut_opt(&mut self, index: usize) -> Option<&mut ShareColumn<T>> {
        self.schema.get_index_mut(index).map(|(_, v)| v)
    }

    // Get a column by its name (panic if not found)
    pub fn get_column_by_name(&self, name: &str) -> &ShareColumn<T> {
        self.schema.get(name).unwrap_or_else(|| panic!("Column '{}' not found", name))
    }
    

    // Get a mutable reference to a column by its name (panic if not found)
    pub fn get_column_by_name_mut(&mut self, name: &str) -> &mut ShareColumn<T> {
        self.schema.get_mut(name).unwrap_or_else(|| panic!("Column '{}' not found", name))
    }

    /// Get the number of columns in the table
    pub fn num_columns(&self) -> usize {
        self.schema.len()
    }

    // Get the number of rows in the table 
    //(assuming all columns have the same number of rows, return 0 if no columns)
    pub fn num_rows(&self) -> usize {
        self.schema.first().map(|(_, c)| c.len()).unwrap_or(0)
    }

    pub fn delete_column(&mut self, name: &str) -> Option<ShareColumn<T>> {
        self.schema.shift_remove(name)
    }

    pub fn insert_column(&mut self, name: String, column: ShareColumn<T>) {
        self.schema.insert(name, column);
    }

    pub fn update_column_name(&mut self, old_name: &str, new_name: &str) {
        if let Some(column) = self.schema.shift_remove(old_name) {
            
            self.schema.insert(new_name.to_string(), column);
            self[new_name].update_name(new_name.to_string());

        } else {
            panic!("Column '{}' not found", old_name);
        }
    }

    pub fn get_key_column(&self) -> &ShareColumn<T> {
        self.key_name.as_ref().map(|name| self.get_column_by_name(name)).expect("Key column not set")
    }

    pub fn print_schema(&self) {
        println!("Table Schema:");
        for (name, column) in &self.schema {
            println!("Column Name: {}, Length: {}", name, column.len());
        }
    }

    // Keep the first n rows
    pub fn head(& mut self, n: usize) {
        for (_, column) in &mut self.schema {
            column.truncate_first(n);
        }
    }

    // Keep the last n rows
    pub fn tail(& mut self, n: usize) {
        for (_, column) in &mut self.schema {
            column.truncate_last(n);
        }
    }
    
}

impl<T: std::fmt::Debug> ShareTable<T> {

    // Print the first n rows of the table. 
    // The first line is the column names, and each subsequent line is a tuple of values.
    pub fn print_first_rows(&self, n: usize, state: &mut Rep3State) {

        if state.id == PartyID:: ID0{

            let num_rows = self.num_rows();
            let display_rows = if n > num_rows { num_rows } else { n };
            
            let headers: Vec<&str> = self.schema.keys().map(|k| k.as_str()).collect();
            println!("{:?}", headers);

            for row_idx in 0..display_rows {
                print!("(");
                for (col_idx, column) in self.schema.values().enumerate() {
                    if col_idx > 0 {
                        print!(", ");
                    }
                    print!("{:?}", column[row_idx]);
                }
                println!(")");

            }
        }
    }
}

// Implement indexing: table["column_name"] and table[0]
impl<T> Index<&str> for ShareTable<T> {
    type Output = ShareColumn<T>;

    fn index(&self, name: &str) -> &Self::Output {
        self.get_column_by_name(name)
    }
}

impl<T> Index<usize> for ShareTable<T> {
    type Output = ShareColumn<T>;

    fn index(&self, index: usize) -> &Self::Output {
        self.get_column_by_index(index)
    }
}

impl<T> IndexMut<&str> for ShareTable<T> {
    fn index_mut(&mut self, name: &str) -> &mut Self::Output {
        self.get_column_by_name_mut(name)
    }
}

impl<T> IndexMut<usize> for ShareTable<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_column_by_index_mut(index)
    }
}




