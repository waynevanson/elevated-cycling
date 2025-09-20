use postcard::{from_bytes, to_allocvec};
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use std::any::type_name;
use std::fmt::Debug;

// at the table definition level.
// that way the schema already knows the data type.

// todo: find way to define the serializer/deserializer;
// really would like to define codecs

#[derive(Debug)]
pub struct Postcard<T>(pub T);

impl<T> Value for Postcard<T>
where
    for<'de> T: Serialize + Deserialize<'de> + Debug,
{
    type SelfType<'a>
        = T
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        to_allocvec(value).expect("shit")
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        from_bytes(data).expect("shit")
    }

    fn type_name() -> redb::TypeName {
        TypeName::new(&format!("Postcard<{}>", type_name::<T>()))
    }
}

impl<T> Key for Postcard<T>
where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Ord,
{
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        let lhs = Self::from_bytes(data1);
        let rhs = Self::from_bytes(data2);

        lhs.cmp(&rhs)
    }
}
