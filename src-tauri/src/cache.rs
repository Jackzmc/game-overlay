use std::collections::HashMap;
use std::ops::IndexMut;
use std::time::SystemTime;

pub struct Cache<T> {
    entries: HashMap<String, CacheEntry<T>>
}

pub struct CacheEntry<T> {
    insert_time: SystemTime,
    item: T
}

impl<T> Cache<T> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new()
        }
    }
    pub fn insert<S: Into<String>>(&mut self, key: S, item: T) -> Option<T> {
        self.entries.insert(key.into(), CacheEntry {
            item,
            insert_time: SystemTime::now()
        })
            .map(|e| e.item)
    }
    
}

