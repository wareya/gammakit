
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, BTreeMap};

#[derive(Debug)]
struct BookkeepingInternal
{
    string_index : usize,
    string_table : HashMap<String, usize>,
    string_table_reverse : BTreeMap<usize, String>,
}

#[derive(Debug)]
pub (crate) struct Bookkeeping
{
    internal : Rc<RefCell<BookkeepingInternal>>,
}

impl Bookkeeping
{
    pub (crate) fn new() -> Bookkeeping
    {
        let mut string_table = HashMap::new();
        string_table.insert("global".to_string(), 1);
        string_table.insert("self".to_string(), 2);
        string_table.insert("other".to_string(), 3);
        string_table.insert("id".to_string(), 4);
        string_table.insert("create".to_string(), 5);
        let mut string_table_reverse = BTreeMap::new();
        string_table_reverse.insert(1, "global".to_string());
        string_table_reverse.insert(2, "self".to_string());
        string_table_reverse.insert(3, "other".to_string());
        string_table_reverse.insert(4, "id".to_string());
        string_table_reverse.insert(5, "create".to_string());
        Bookkeeping{internal : Rc::new(RefCell::new(BookkeepingInternal{string_index : 6, string_table, string_table_reverse}))}
    }
    pub (crate) fn refclone(&self) -> Bookkeeping
    {
        Bookkeeping{internal : Rc::clone(&self.internal)}
    }
    pub (crate) fn get_string_index(&self, string : &String) -> usize
    {
        let mut internal = self.internal.borrow_mut();
        if let Some(index) = internal.string_table.get(string)
        {
            *index
        }
        else
        {
            let index = internal.string_index;
            internal.string_index += 1;
            internal.string_table.insert(string.clone(), index);
            internal.string_table_reverse.insert(index, string.clone());
            index
        }
    }
    pub (crate) fn get_string(&self, index : usize) -> String
    {
        if let Some(string) = self.internal.borrow().string_table_reverse.get(&index)
        {
            string.clone()
        }
        else
        {
            format!("<index {} with no associated string>", index)
        }
    }
}