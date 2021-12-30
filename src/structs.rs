use serde_derive::Deserialize;
use std::cmp::Ordering;

pub(crate) struct Value {
    pub(crate) value: String,
    pub(crate) times: i32,
    pub(crate) create_time: u64,
    pub(crate) public: bool,
}

impl Value {
    pub(crate) fn new(v: impl ToString, create_time: u64) -> Self {
        Value {
            value: v.to_string(),
            times: 1,
            create_time,
            public: true,
        }
    }
}

#[derive(Debug)]
pub(crate) struct StructInDeleteQueue {
    pub(crate) delete_time: u64,
    pub(crate) create_time: u64,
    pub(crate) key: String,
}

impl StructInDeleteQueue {
    pub(crate) fn new(delete_time: u64, create_time: u64, key: String) -> Self {
        Self {
            delete_time,
            create_time,
            key,
        }
    }
    pub(crate) fn clone(&self) -> Self {
        Self {
            delete_time: self.delete_time,
            create_time: self.create_time,
            key: self.key.clone(),
        }
    }
}

impl PartialEq<Self> for StructInDeleteQueue {
    fn eq(&self, other: &Self) -> bool {
        self.delete_time == other.delete_time
    }
}

impl Eq for StructInDeleteQueue {}
impl Ord for StructInDeleteQueue {
    fn cmp(&self, other: &Self) -> Ordering {
        other.delete_time.cmp(&self.delete_time)
        // self.delete_time.cmp(&other.delete_time)
    }
}

impl PartialOrd for StructInDeleteQueue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
#[derive(Debug, Deserialize)]
pub(crate) struct Params {
    pub(crate) times: Option<i32>,
    pub(crate) minutes: Option<u64>,
    pub(crate) private: Option<String>,
}
