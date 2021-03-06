use std::collections::HashMap;
use std::collections::hash_map;
use std::default::Default;
use std::slice;
use wire_format;
use clear::Clear;
use std::hash::Hash;
use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;

#[derive(Debug)]
pub enum UnknownValue {
    Fixed32(u32),
    Fixed64(u64),
    Varint(u64),
    LengthDelimited(Vec<u8>),
}

impl UnknownValue {
    pub fn wire_type(&self) -> wire_format::WireType {
        self.get_ref().wire_type()
    }

    pub fn get_ref<'s>(&'s self) -> UnknownValueRef<'s> {
        match *self {
            UnknownValue::Fixed32(fixed32) => UnknownValueRef::Fixed32(fixed32),
            UnknownValue::Fixed64(fixed64) => UnknownValueRef::Fixed64(fixed64),
            UnknownValue::Varint(varint) => UnknownValueRef::Varint(varint),
            UnknownValue::LengthDelimited(ref bytes) => UnknownValueRef::LengthDelimited(&bytes),
        }
    }
}

pub enum UnknownValueRef<'o> {
    Fixed32(u32),
    Fixed64(u64),
    Varint(u64),
    LengthDelimited(&'o [u8]),
}

impl<'o> UnknownValueRef<'o> {
    pub fn wire_type(&self) -> wire_format::WireType {
        match *self {
            UnknownValueRef::Fixed32(_) => wire_format::WireTypeFixed32,
            UnknownValueRef::Fixed64(_) => wire_format::WireTypeFixed64,
            UnknownValueRef::Varint(_) => wire_format::WireTypeVarint,
            UnknownValueRef::LengthDelimited(_) => wire_format::WireTypeLengthDelimited,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Hash)]
pub struct UnknownValues {
    pub fixed32: Vec<u32>,
    pub fixed64: Vec<u64>,
    pub varint: Vec<u64>,
    pub length_delimited: Vec<Vec<u8>>,
}

impl UnknownValues {
    pub fn add_value(&mut self, value: UnknownValue) {
        match value {
            UnknownValue::Fixed64(fixed64) => self.fixed64.push(fixed64),
            UnknownValue::Fixed32(fixed32) => self.fixed32.push(fixed32),
            UnknownValue::Varint(varint) => self.varint.push(varint),
            UnknownValue::LengthDelimited(length_delimited) => {
                self.length_delimited.push(length_delimited)
            }
        };
    }

    pub fn iter<'s>(&'s self) -> UnknownValuesIter<'s> {
        UnknownValuesIter {
            fixed32: self.fixed32.iter(),
            fixed64: self.fixed64.iter(),
            varint: self.varint.iter(),
            length_delimited: self.length_delimited.iter(),
        }
    }
}

impl<'a> IntoIterator for &'a UnknownValues {
    type Item = UnknownValueRef<'a>;
    type IntoIter = UnknownValuesIter<'a>;

    fn into_iter(self) -> UnknownValuesIter<'a> {
        self.iter()
    }
}

pub struct UnknownValuesIter<'o> {
    fixed32: slice::Iter<'o, u32>,
    fixed64: slice::Iter<'o, u64>,
    varint: slice::Iter<'o, u64>,
    length_delimited: slice::Iter<'o, Vec<u8>>,
}

impl<'o> Iterator for UnknownValuesIter<'o> {
    type Item = UnknownValueRef<'o>;

    fn next(&mut self) -> Option<UnknownValueRef<'o>> {
        let fixed32 = self.fixed32.next();
        if fixed32.is_some() {
            return Some(UnknownValueRef::Fixed32(*fixed32.unwrap()));
        }
        let fixed64 = self.fixed64.next();
        if fixed64.is_some() {
            return Some(UnknownValueRef::Fixed64(*fixed64.unwrap()));
        }
        let varint = self.varint.next();
        if varint.is_some() {
            return Some(UnknownValueRef::Varint(*varint.unwrap()));
        }
        let length_delimited = self.length_delimited.next();
        if length_delimited.is_some() {
            return Some(UnknownValueRef::LengthDelimited(&length_delimited.unwrap()));
        }
        None
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct UnknownFields {
    // option is needed, because HashMap constructor performs allocation,
    // and very expensive
    pub fields: Option<Box<HashMap<u32, UnknownValues>>>,
}

/// Very simple hash implementation of `Hash` for `UnknownFields`.
/// Since map is unordered, we cannot put entry hashes into hasher,
/// instead we summing hashes of entries.
impl Hash for UnknownFields {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(ref map) = self.fields {
            if !map.is_empty() {
                let mut hash: u64 = 0;
                for (k, v) in &**map {
                    let mut entry_hasher = DefaultHasher::new();
                    Hash::hash(&(k, v), &mut entry_hasher);
                    hash.wrapping_add(entry_hasher.finish());
                }
                Hash::hash(&hash, state);
            }
        }
    }
}

impl UnknownFields {
    pub fn new() -> UnknownFields {
        Default::default()
    }

    fn init_map(&mut self) {
        if self.fields.is_none() {
            self.fields = Some(Default::default());
        }
    }

    fn find_field<'a>(&'a mut self, number: &'a u32) -> &'a mut UnknownValues {
        self.init_map();

        match self.fields.as_mut().unwrap().entry(*number) {
            hash_map::Entry::Occupied(e) => e.into_mut(),
            hash_map::Entry::Vacant(e) => e.insert(Default::default()),
        }
    }

    pub fn add_fixed32(&mut self, number: u32, fixed32: u32) {
        self.find_field(&number).fixed32.push(fixed32);
    }

    pub fn add_fixed64(&mut self, number: u32, fixed64: u64) {
        self.find_field(&number).fixed64.push(fixed64);
    }

    pub fn add_varint(&mut self, number: u32, varint: u64) {
        self.find_field(&number).varint.push(varint);
    }

    pub fn add_length_delimited(&mut self, number: u32, length_delimited: Vec<u8>) {
        self.find_field(&number)
            .length_delimited
            .push(length_delimited);
    }

    pub fn add_value(&mut self, number: u32, value: UnknownValue) {
        self.find_field(&number).add_value(value);
    }

    pub fn iter<'s>(&'s self) -> UnknownFieldsIter<'s> {
        UnknownFieldsIter { entries: self.fields.as_ref().map(|m| m.iter()) }
    }

    pub fn get(&self, field_number: u32) -> Option<&UnknownValues> {
        match self.fields {
            Some(ref map) => map.get(&field_number),
            None => None,
        }
    }
}

impl Clear for UnknownFields {
    fn clear(&mut self) {
        if let Some(ref mut fields) = self.fields {
            fields.clear();
        }
    }
}

impl<'a> IntoIterator for &'a UnknownFields {
    type Item = (u32, &'a UnknownValues);
    type IntoIter = UnknownFieldsIter<'a>;

    fn into_iter(self) -> UnknownFieldsIter<'a> {
        self.iter()
    }
}

pub struct UnknownFieldsIter<'s> {
    entries: Option<hash_map::Iter<'s, u32, UnknownValues>>,
}

impl<'s> Iterator for UnknownFieldsIter<'s> {
    type Item = (u32, &'s UnknownValues);

    fn next(&mut self) -> Option<(u32, &'s UnknownValues)> {
        match self.entries {
            Some(ref mut entries) => entries.next().map(|(&number, values)| (number, values)),
            None => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::UnknownFields;
    use std::hash::Hash;
    use std::hash::Hasher;
    use std::collections::hash_map::DefaultHasher;

    #[test]
    fn unknown_fields_hash() {
        let mut unknown_fields_1 = UnknownFields::new();
        let mut unknown_fields_2 = UnknownFields::new();

        // Check field order is not important

        unknown_fields_1.add_fixed32(10, 222);
        unknown_fields_1.add_fixed32(10, 223);
        unknown_fields_1.add_fixed64(14, 224);

        unknown_fields_2.add_fixed32(10, 222);
        unknown_fields_2.add_fixed64(14, 224);
        unknown_fields_2.add_fixed32(10, 223);

        fn hash(unknown_fields: &UnknownFields) -> u64 {
            let mut hasher = DefaultHasher::new();
            Hash::hash(unknown_fields, &mut hasher);
            hasher.finish()
        }

        assert_eq!(hash(&unknown_fields_1), hash(&unknown_fields_2));
    }
}
