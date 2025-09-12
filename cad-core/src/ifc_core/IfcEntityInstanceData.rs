// attribute.rs

use bitvec::vec::BitVec;
use rocksdb::{DB};
use std::sync::Arc;

// =======================
// Базовые типы (из .h)
// =======================

#[derive(Debug, Clone)]
pub struct Blank;

#[derive(Debug, Clone)]
pub struct Derived;

#[derive(Debug, Clone)]
pub struct EmptyAggregate;

#[derive(Debug, Clone)]
pub struct EmptyAggregateOfAggregate;

/// Замена boost::logic::tribool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriBool {
    True,
    False,
    Indeterminate,
}

#[derive(Debug, Clone)]
pub struct EnumerationReference {
    name: String,
    index: usize,
}

impl EnumerationReference {
    pub fn new(name: String, index: usize) -> Self {
        Self { name, index }
    }

    pub fn value(&self) -> String {
        self.name.clone()
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

// =======================
// IfcBaseClass (заглушка)
// =======================
pub mod ifcutil {
    #[derive(Debug, Clone)]
    pub struct IfcBaseClass {
        pub id: usize,
        pub identity: usize,
        pub decl_name: String,
        pub is_entity: bool,
    }

    impl IfcBaseClass {
        pub fn new(id: usize, identity: usize, decl_name: &str, is_entity: bool) -> Self {
            Self {
                id,
                identity,
                decl_name: decl_name.to_string(),
                is_entity,
            }
        }

        pub fn declaration(&self) -> &Self {
            self
        }

        pub fn as_entity(&self) -> bool {
            self.is_entity
        }

        pub fn name(&self) -> &str {
            &self.decl_name
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum ArgumentType {
        Blank,
        Int,
        Bool,
        Double,
        String,
        Tribool,
        Bitset,
        Enum,
        Instance,
        Aggregate,
        AggregateOfAggregate,
        Derived,
        EmptyAggregate,
        EmptyAggregateOfAggregate,
    }
}

// =======================
// Агрегаты (замена shared_ptr)
// =======================
#[derive(Debug, Clone)]
pub struct AggregateOfInstance {
    items: Vec<Arc<ifcutil::IfcBaseClass>>,
}

impl AggregateOfInstance {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, item: Arc<ifcutil::IfcBaseClass>) {
        self.items.push(item);
    }

    pub fn size(&self) -> usize {
        self.items.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<ifcutil::IfcBaseClass>> {
        self.items.iter()
    }
}

#[derive(Debug, Clone)]
pub struct AggregateOfAggregateOfInstance {
    items: Vec<Vec<Arc<ifcutil::IfcBaseClass>>>,
}

impl AggregateOfAggregateOfInstance {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, inner: Vec<Arc<ifcutil::IfcBaseClass>>) {
        self.items.push(inner);
    }

    pub fn size(&self) -> usize {
        self.items.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Vec<Arc<ifcutil::IfcBaseClass>>> {
        self.items.iter()
    }
}

// =======================
// Аналог SizeVisitor
// =======================
pub trait SizeVisitor {
    fn size(&self) -> i32;
}

impl SizeVisitor for Blank {
    fn size(&self) -> i32 {
        -1
    }
}
impl SizeVisitor for Derived {
    fn size(&self) -> i32 {
        -1
    }
}
impl SizeVisitor for i32 {
    fn size(&self) -> i32 {
        -1
    }
}
impl SizeVisitor for bool {
    fn size(&self) -> i32 {
        -1
    }
}
impl SizeVisitor for TriBool {
    fn size(&self) -> i32 {
        -1
    }
}
impl SizeVisitor for f64 {
    fn size(&self) -> i32 {
        -1
    }
}
impl SizeVisitor for String {
    fn size(&self) -> i32 {
        -1
    }
}
impl SizeVisitor for BitVec {
    fn size(&self) -> i32 {
        -1
    }
}
impl SizeVisitor for EmptyAggregate {
    fn size(&self) -> i32 {
        0
    }
}
impl SizeVisitor for EmptyAggregateOfAggregate {
    fn size(&self) -> i32 {
        0
    }
}
impl SizeVisitor for Vec<i32> {
    fn size(&self) -> i32 {
        self.len() as i32
    }
}
impl SizeVisitor for Vec<f64> {
    fn size(&self) -> i32 {
        self.len() as i32
    }
}
impl SizeVisitor for Vec<Vec<i32>> {
    fn size(&self) -> i32 {
        self.len() as i32
    }
}
impl SizeVisitor for Vec<Vec<f64>> {
    fn size(&self) -> i32 {
        self.len() as i32
    }
}
impl SizeVisitor for Vec<String> {
    fn size(&self) -> i32 {
        self.len() as i32
    }
}
impl SizeVisitor for Vec<BitVec> {
    fn size(&self) -> i32 {
        self.len() as i32
    }
}
impl SizeVisitor for EnumerationReference {
    fn size(&self) -> i32 {
        -1
    }
}
impl SizeVisitor for Arc<ifcutil::IfcBaseClass> {
    fn size(&self) -> i32 {
        -1
    }
}
impl SizeVisitor for AggregateOfInstance {
    fn size(&self) -> i32 {
        self.size() as i32
    }
}
impl SizeVisitor for AggregateOfAggregateOfInstance {
    fn size(&self) -> i32 {
        self.size() as i32
    }
}


use rocksdb::{WriteOptions};
use std::convert::TryFrom;
use std::fmt::Debug;


// =============== IfcParse::declaration / schema / file ===============

#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    pub is_header: bool,
}
impl Schema {
    pub fn new(name: &str, is_header: bool) -> Self {
        Self {
            name: name.to_string(),
            is_header,
        }
    }
}
pub struct HeaderSectionSchema;
impl HeaderSectionSchema {
    pub fn get_schema() -> Schema {
        Schema::new("header", true)
    }
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub schema: Schema,
    pub is_entity: bool,
    pub is_enumeration: bool,
}
impl Declaration {
    pub fn new_entity(name: &str, schema: Schema) -> Self {
        Self {
            name: name.to_string(),
            schema,
            is_entity: true,
            is_enumeration: false,
        }
    }
    pub fn new_type(name: &str, schema: Schema, is_enum: bool) -> Self {
        Self {
            name: name.to_string(),
            schema,
            is_entity: false,
            is_enumeration: is_enum,
        }
    }
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
    pub fn as_entity(&self) -> bool {
        self.is_entity
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn as_enumeration_type(&self) -> &Self {
        self
    }
    pub fn lookup_enum_value(&self, idx: usize) -> String {
        // В реальном IfcOpenShell тут маппинг. Для переноса — делаем символическое имя:
        format!("{}#{}", self.name, idx)
    }
}

#[derive(Debug, Clone)]
pub struct SchemaContainer {
    pub decls: Vec<Declaration>,
}
impl SchemaContainer {
    pub fn new() -> Self {
        Self { decls: Vec::new() }
    }
    pub fn declarations(&self) -> &Vec<Declaration> {
        &self.decls
    }
}

#[derive(Debug, Clone)]
pub struct IfcFileShim {
    pub schema: Arc<SchemaContainer>,
}
impl IfcFileShim {
    pub fn new(schema: Arc<SchemaContainer>) -> Self {
        Self { schema }
    }
    pub fn schema(&self) -> &SchemaContainer {
        &self.schema
    }
}

// ================= TypeEncoder (A..Z кодировки) =================

pub struct TypeEncoder;
impl TypeEncoder {
    pub fn encode_type_blank() -> u8 {
        b'A'
    } // 0
    pub fn encode_type_int() -> u8 {
        b'B'
    } // 1
    pub fn encode_type_bool() -> u8 {
        b'C'
    } // 2
    pub fn encode_type_tribool() -> u8 {
        b'D'
    } // 3
    pub fn encode_type_double() -> u8 {
        b'E'
    } // 4
    pub fn encode_type_string() -> u8 {
        b'F'
    } // 5
    pub fn encode_type_bitset() -> u8 {
        b'G'
    } // 6
    pub fn encode_type_enumref() -> u8 {
        b'H'
    } // 7
    pub fn encode_type_ifcptr() -> u8 {
        b'I'
    } // 8
    pub fn encode_type_vec_int() -> u8 {
        b'J'
    } // 9
    pub fn encode_type_vec_double() -> u8 {
        b'K'
    } // 10
    pub fn encode_type_vec_vec_int() -> u8 {
        b'L'
    } // 11
    pub fn encode_type_vec_vec_double() -> u8 {
        b'M'
    } // 12
    pub fn encode_type_vec_string() -> u8 {
        b'N'
    } // 13
    pub fn encode_type_vec_bitset() -> u8 {
        b'O'
    } // 14
    pub fn encode_type_agg_inst() -> u8 {
        b'P'
    } // 15
    pub fn encode_type_agg_agg_inst() -> u8 {
        b'Q'
    } // 16
    pub fn encode_type_derived() -> u8 {
        b'R'
    } // 17
    pub fn encode_type_empty_agg() -> u8 {
        b'S'
    } // 18
    pub fn encode_type_empty_agg_agg() -> u8 {
        b'T'
    } // 19

    pub fn to_index(c: u8) -> usize {
        (c - b'A') as usize
    }
}

// ================= In-memory storage (storage_ptr) =================

#[derive(Clone)]
pub enum AnyValue {
    Blank(Blank),
    Derived(Derived),
    Int(i32),
    Bool(bool),
    Tri(TriBool),
    Double(f64),
    Str(String),
    Bitset(BitVec),
    EmptyAgg(EmptyAggregate),
    EmptyAggAgg(EmptyAggregateOfAggregate),
    VecInt(Vec<i32>),
    VecDouble(Vec<f64>),
    VecVecInt(Vec<Vec<i32>>),
    VecVecDouble(Vec<Vec<f64>>),
    VecStr(Vec<String>),
    VecBitset(Vec<BitVec>),
    EnumRef(EnumerationReference),
    IfcPtr(Arc<ifcutil::IfcBaseClass>),
    AggInst(AggregateOfInstance),
    AggAggInst(AggregateOfAggregateOfInstance),
}

#[derive(Clone)]
pub struct MemoryAttributeStorage {
    pub slots: Vec<AnyValue>, // индекс = index_
}
impl MemoryAttributeStorage {
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }
    pub fn set(&mut self, idx: usize, v: AnyValue) {
        if idx >= self.slots.len() {
            self.slots.resize(idx + 1, AnyValue::Blank(Blank));
        }
        self.slots[idx] = v;
    }
    pub fn index(&self, idx: usize) -> usize {
        use AnyValue::*;
        match self.slots.get(idx) {
            None => TypeEncoder::to_index(TypeEncoder::encode_type_blank()),
            Some(Blank(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_blank()),
            Some(Derived(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_derived()),
            Some(Int(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_int()),
            Some(Bool(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_bool()),
            Some(Tri(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_tribool()),
            Some(Double(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_double()),
            Some(Str(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_string()),
            Some(Bitset(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_bitset()),
            Some(EmptyAgg(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_empty_agg()),
            Some(EmptyAggAgg(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_empty_agg_agg()),
            Some(VecInt(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_vec_int()),
            Some(VecDouble(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_vec_double()),
            Some(VecVecInt(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_vec_vec_int()),
            Some(VecVecDouble(_)) => {
                TypeEncoder::to_index(TypeEncoder::encode_type_vec_vec_double())
            }
            Some(VecStr(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_vec_string()),
            Some(VecBitset(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_vec_bitset()),
            Some(EnumRef(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_enumref()),
            Some(IfcPtr(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_ifcptr()),
            Some(AggInst(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_agg_inst()),
            Some(AggAggInst(_)) => TypeEncoder::to_index(TypeEncoder::encode_type_agg_agg_inst()),
        }
    }
    pub fn has<T>(&self, idx: usize) -> bool {
        use AnyValue::*;
        if let Some(v) = self.slots.get(idx) {
            let ok = match v {
                Blank(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<Blank>(),
                Derived(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<Derived>(),
                Int(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<i32>(),
                Bool(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<bool>(),
                Tri(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<TriBool>(),
                Double(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<f64>(),
                Str(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<String>(),
                Bitset(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<BitVec>(),
                EmptyAgg(_) => {
                    std::any::TypeId::of::<T>() == std::any::TypeId::of::<EmptyAggregate>()
                }
                EmptyAggAgg(_) => {
                    std::any::TypeId::of::<T>()
                        == std::any::TypeId::of::<EmptyAggregateOfAggregate>()
                }
                VecInt(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<Vec<i32>>(),
                VecDouble(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<Vec<f64>>(),
                VecVecInt(_) => {
                    std::any::TypeId::of::<T>() == std::any::TypeId::of::<Vec<Vec<i32>>>()
                }
                VecVecDouble(_) => {
                    std::any::TypeId::of::<T>() == std::any::TypeId::of::<Vec<Vec<f64>>>()
                }
                VecStr(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<Vec<String>>(),
                VecBitset(_) => {
                    std::any::TypeId::of::<T>() == std::any::TypeId::of::<Vec<BitVec>>()
                }
                EnumRef(_) => {
                    std::any::TypeId::of::<T>() == std::any::TypeId::of::<EnumerationReference>()
                }
                IfcPtr(_) => {
                    std::any::TypeId::of::<T>()
                        == std::any::TypeId::of::<Arc<ifcutil::IfcBaseClass>>()
                }
                AggInst(_) => {
                    std::any::TypeId::of::<T>() == std::any::TypeId::of::<AggregateOfInstance>()
                }
                AggAggInst(_) => {
                    std::any::TypeId::of::<T>()
                        == std::any::TypeId::of::<AggregateOfAggregateOfInstance>()
                }
            };
            return ok;
        }
        false
    }
    pub fn get<T: TryFrom<AnyValue>>(&self, idx: usize) -> T {
        let v = self
            .slots
            .get(idx)
            .cloned()
            .unwrap_or(AnyValue::Blank(Blank));
        T::try_from(v).ok().expect("bad type")
    }
    pub fn apply_visitor<V: super::SizeVisitor>(&self, idx: usize, visit: &V) -> i32 {
        use AnyValue::*;
        match self.slots.get(idx) {
            None => (-1),
            Some(Blank(b)) => visit.size(),
            Some(Derived(d)) => visit.size(),
            Some(Int(_)) => visit.size(),
            Some(Bool(_)) => visit.size(),
            Some(Tri(_)) => visit.size(),
            Some(Double(_)) => visit.size(),
            Some(Str(_)) => visit.size(),
            Some(Bitset(_)) => visit.size(),
            Some(EmptyAgg(e)) => e.size(),
            Some(EmptyAggAgg(e)) => e.size(),
            Some(VecInt(v)) => v.size(),
            Some(VecDouble(v)) => v.size(),
            Some(VecVecInt(v)) => v.size(),
            Some(VecVecDouble(v)) => v.size(),
            Some(VecStr(v)) => v.size(),
            Some(VecBitset(v)) => v.size(),
            Some(EnumRef(_)) => visit.size(),
            Some(IfcPtr(_)) => visit.size(),
            Some(AggInst(a)) => a.size() as i32,
            Some(AggAggInst(a)) => a.size() as i32,
        }
    }
}

// =============== RocksDB storage shim (db_ptr) =================

pub mod implrocks {
    use super::*;
    #[derive(Clone)]
    pub struct RocksDbFileStorage {
        pub db: Arc<DB>,
        pub wopts: WriteOptions,
        pub file: Arc<IfcFileShim>,
    }
    impl RocksDbFileStorage {
        pub fn assert_existance(&self, id: usize, _kind: i32) -> Arc<ifcutil::IfcBaseClass> {
            // В реале — lookup. Здесь создаём заглушку:
            Arc::new(ifcutil::IfcBaseClass::new(id, id, "IFCGENERIC", true))
        }
        pub const ENTITYINSTANCE_REF: i32 = 1;
        pub const TYPEDECL_REF: i32 = 2;
    }

    // serialize/deserialize набор (аналог impl::serialize/deserialize)
    pub mod ser {
        use super::*;
        pub fn serialize_ifcptr(val: &Arc<ifcutil::IfcBaseClass>) -> Vec<u8> {
            // [tag, kind, id(usize)]
            let mut out = Vec::with_capacity(1 + 1 + std::mem::size_of::<usize>());
            out.push(super::super::TypeEncoder::encode_type_ifcptr());
            out.push(if val.declaration().as_entity() {
                b'i'
            } else {
                b't'
            });
            let id = if val.id != 0 { val.id } else { val.identity };
            out.extend_from_slice(&id.to_ne_bytes());
            out
        }
        pub fn serialize_enumref(v: &EnumerationReference, schema: &SchemaContainer) -> Vec<u8> {
            // [tag, decl_index(usize), enum_index(usize)]
            let mut out = Vec::with_capacity(1 + std::mem::size_of::<usize>() * 2);
            out.push(super::super::TypeEncoder::encode_type_enumref());
            // В реальности: index_in_schema(). Здесь найдём по имени.
            let decl_idx = schema
                .declarations()
                .iter()
                .position(|d| d.name == v.value())
                .unwrap_or(0);
            out.extend_from_slice(&decl_idx.to_ne_bytes());
            out.extend_from_slice(&v.index().to_ne_bytes());
            out
        }
        pub fn serialize_blank() -> Vec<u8> {
            vec![super::super::TypeEncoder::encode_type_blank()]
        }
        pub fn serialize_derived() -> Vec<u8> {
            vec![super::super::TypeEncoder::encode_type_derived()]
        }
        pub fn serialize_empty_agg() -> Vec<u8> {
            vec![super::super::TypeEncoder::encode_type_empty_agg()]
        }
        pub fn serialize_empty_agg_agg() -> Vec<u8> {
            vec![super::super::TypeEncoder::encode_type_empty_agg_agg()]
        }
        pub fn serialize_tribool(t: TriBool) -> Vec<u8> {
            let v = match t {
                TriBool::False => 0u8,
                TriBool::True => 1u8,
                TriBool::Indeterminate => 2u8,
            };
            vec![super::super::TypeEncoder::encode_type_tribool(), v]
        }
        pub fn serialize_bitset(b: &BitVec) -> Vec<u8> {
            let mut out = Vec::with_capacity(1 + b.len());
            out.push(super::super::TypeEncoder::encode_type_bitset());
            let s: String = b.iter().map(|bit| if *bit { '1' } else { '0' }).collect();
            out.extend_from_slice(s.as_bytes());
            out
        }
        pub fn serialize_agg_inst(a: &AggregateOfInstance) -> Vec<u8> {
            // [tag] then sequence of [kind(1), usize]
            let mut out = Vec::with_capacity(1 + a.size() * (1 + std::mem::size_of::<usize>()));
            out.push(super::super::TypeEncoder::encode_type_agg_inst());
            for it in a.iter() {
                out.push(if it.as_ref().as_entity() { b'i' } else { b't' });
                let id = if it.id != 0 { it.id } else { it.identity };
                out.extend_from_slice(&id.to_ne_bytes());
            }
            out
        }
        pub fn serialize_agg_agg_inst(a: &AggregateOfAggregateOfInstance) -> Vec<u8> {
            // [tag] then for each inner: [size(usize)] then repeated [kind(1), usize]
            let mut out = Vec::new();
            out.push(super::super::TypeEncoder::encode_type_agg_agg_inst());
            for inner in a.iter() {
                let sz = inner.len();
                out.extend_from_slice(&sz.to_ne_bytes());
                for it in inner.iter() {
                    out.push(if it.as_ref().as_entity() { b'i' } else { b't' });
                    let id = if it.id != 0 { it.id } else { it.identity };
                    out.extend_from_slice(&id.to_ne_bytes());
                }
            }
            out
        }

        pub fn deserialize_tribool(buf: &[u8]) -> Option<TriBool> {
            if buf.first().copied()? != super::super::TypeEncoder::encode_type_tribool() {
                return None;
            }
            match buf.get(1).copied().unwrap_or(255) {
                0 => Some(TriBool::False),
                1 => Some(TriBool::True),
                2 => Some(TriBool::Indeterminate),
                _ => None,
            }
        }
        pub fn deserialize_bitset(buf: &[u8]) -> Option<BitVec> {
            if buf.first().copied()? != super::super::TypeEncoder::encode_type_bitset() {
                return None;
            }
            let s = std::str::from_utf8(&buf[1..]).ok()?;
            let mut bv = BitVec::new();
            for ch in s.chars() {
                bv.push(ch == '1');
            }
            Some(bv)
        }
        pub fn deserialize_enumref(
            storage: &implrocks::RocksDbFileStorage,
            buf: &[u8],
        ) -> Option<EnumerationReference> {
            if buf.first().copied()? != super::super::TypeEncoder::encode_type_enumref() {
                return None;
            }
            let mut off = 1usize;
            let mut read_usize = |b: &[u8], off: &mut usize| {
                let mut arr = [0u8; std::mem::size_of::<usize>()];
                arr.copy_from_slice(&b[*off..*off + arr.len()]);
                *off += arr.len();
                usize::from_ne_bytes(arr)
            };
            let decl_idx = read_usize(buf, &mut off);
            let enum_idx = read_usize(buf, &mut off);
            let decl = storage.file.schema().declarations().get(decl_idx)?;
            Some(EnumerationReference::new(decl.name.clone(), enum_idx))
        }
        pub fn deserialize_agg_inst(
            storage: &implrocks::RocksDbFileStorage,
            buf: &[u8],
        ) -> Option<AggregateOfInstance> {
            if buf.first().copied()? != super::super::TypeEncoder::encode_type_agg_inst() {
                return None;
            }
            let mut out = AggregateOfInstance::new();
            let mut off = 1usize;
            while off < buf.len() {
                let kind = buf[off];
                off += 1;
                let mut arr = [0u8; std::mem::size_of::<usize>()];
                arr.copy_from_slice(&buf[off..off + arr.len()]);
                off += arr.len();
                let id = usize::from_ne_bytes(arr);
                let ptr = if kind == b'i' {
                    storage.assert_existance(id, implrocks::RocksDbFileStorage::ENTITYINSTANCE_REF)
                } else {
                    storage.assert_existance(id, implrocks::RocksDbFileStorage::TYPEDECL_REF)
                };
                out.push(ptr);
            }
            Some(out)
        }
        pub fn deserialize_agg_agg_inst(
            storage: &implrocks::RocksDbFileStorage,
            buf: &[u8],
        ) -> Option<AggregateOfAggregateOfInstance> {
            if buf.first().copied()? != super::super::TypeEncoder::encode_type_agg_agg_inst() {
                return None;
            }
            let mut out = AggregateOfAggregateOfInstance::new();
            let mut off = 1usize;
            while off < buf.len() {
                let mut arr_sz = [0u8; std::mem::size_of::<usize>()];
                arr_sz.copy_from_slice(&buf[off..off + arr_sz.len()]);
                off += arr_sz.len();
                let inner_sz = usize::from_ne_bytes(arr_sz);
                let mut inner: Vec<Arc<ifcutil::IfcBaseClass>> = Vec::with_capacity(inner_sz);
                for _ in 0..inner_sz {
                    let kind = buf[off];
                    off += 1;
                    let mut arr = [0u8; std::mem::size_of::<usize>()];
                    arr.copy_from_slice(&buf[off..off + arr.len()]);
                    off += arr.len();
                    let id = usize::from_ne_bytes(arr);
                    let ptr = if kind == b'i' {
                        storage
                            .assert_existance(id, implrocks::RocksDbFileStorage::ENTITYINSTANCE_REF)
                    } else {
                        storage.assert_existance(id, implrocks::RocksDbFileStorage::TYPEDECL_REF)
                    };
                    inner.push(ptr);
                }
                out.push(inner);
            }
            Some(out)
        }
    }
}

// ================= AttributeArray pointer union =================

#[derive(Clone)]
pub struct DbPtr {
    pub db: Arc<DB>,
    pub wopts: WriteOptions,
    pub file: Arc<IfcFileShim>,
}
#[derive(Clone)]
pub struct StoragePtr {
    pub storage: Arc<MemoryAttributeStorage>,
}
#[derive(Clone)]
pub struct AttributeArrayPtr {
    pub storage_ptr: StoragePtr,
    pub db_ptr: Option<implrocks::RocksDbFileStorage>,
}
impl AttributeArrayPtr {
    pub fn from_memory(storage: Arc<MemoryAttributeStorage>) -> Self {
        Self {
            storage_ptr: StoragePtr { storage },
            db_ptr: None,
        }
    }
    pub fn with_db(mut self, db: Arc<DB>, file: Arc<IfcFileShim>) -> Self {
        let mut wopts = WriteOptions::default();
        wopts.set_sync(false);
        self.db_ptr = Some(implrocks::RocksDbFileStorage { db, wopts, file });
        self
    }
}

// ================= AttributeValue =================

#[derive(Clone)]
pub struct AttributeValue {
    pub array_: AttributeArrayPtr,
    pub storage_model_: u8, // 0=in-memory, 1=rocksdb
    pub instance_name_: usize,
    pub entity_or_type_: Declaration,
    pub index_: u8,
}

impl AttributeValue {
    pub fn new_mem(
        array: AttributeArrayPtr,
        instance_name: usize,
        decl: Declaration,
        index: u8,
    ) -> Self {
        Self {
            array_: array,
            storage_model_: 0,
            instance_name_: instance_name,
            entity_or_type_: decl,
            index_: index,
        }
    }
    pub fn new_db(
        array: AttributeArrayPtr,
        instance_name: usize,
        decl: Declaration,
        index: u8,
    ) -> Self {
        Self {
            array_: array,
            storage_model_: 1,
            instance_name_: instance_name,
            entity_or_type_: decl,
            index_: index,
        }
    }
}

// ======== dispatch helpers (порт C++ dispatch_get_/has_/index_) ========

fn make_key(is_header: bool, is_entity: bool, name_or_id: String, index: u8) -> String {
    let prefix = if is_header {
        "h|"
    } else if is_entity {
        "i|"
    } else {
        "t|"
    };
    format!("{}{}|{}", prefix, name_or_id, index)
}

fn db_get_raw(
    array: &AttributeArrayPtr,
    is_header: bool,
    is_entity: bool,
    name_or_id: String,
    index: u8,
) -> Option<Vec<u8>> {
    let dbs = array.db_ptr.as_ref()?;
    let key = make_key(is_header, is_entity, name_or_id, index);
    let v = dbs.db.get(key.as_bytes()).ok()??;
    Some(v.to_vec())
}
fn db_put_raw(
    array: &AttributeArrayPtr,
    is_header: bool,
    is_entity: bool,
    name_or_id: String,
    index: u8,
    val: &[u8],
) {
    if let Some(dbs) = &array.db_ptr {
        let key = make_key(is_header, is_entity, name_or_id, index);
        let _ = dbs.db.put_opt(key.as_bytes(), val, &dbs.wopts);
    }
}

fn dispatch_index_(
    array_: &AttributeArrayPtr,
    storage_model_: u8,
    instance_name_: usize,
    entity_or_type: &Declaration,
    index_: u8,
) -> usize {
    return if storage_model_ == 0 {
        array_.storage_ptr.storage.index(index_ as usize)
    } else {
        let is_header = entity_or_type.schema().is_header;
        let name_or_id = if is_header {
            entity_or_type.name().to_string()
        } else {
            instance_name_.to_string()
        };
        if let Some(v) = db_get_raw(
            array_,
            is_header,
            entity_or_type.as_entity(),
            name_or_id,
            index_,
        ) {
            return TypeEncoder::to_index(*v.first().unwrap_or(&TypeEncoder::encode_type_blank()));
        }
        TypeEncoder::to_index(TypeEncoder::encode_type_blank())
    };
}

fn dispatch_has_<T>(
    _marker: std::marker::PhantomData<T>,
    array_: &AttributeArrayPtr,
    storage_model_: u8,
    instance_name_: usize,
    entity_or_type: &Declaration,
    index_: u8,
) -> bool {
    if storage_model_ == 0 {
        return array_.storage_ptr.storage.has::<T>(index_ as usize);
    } else {
        let is_header = entity_or_type.schema().is_header;
        let name_or_id = if is_header {
            entity_or_type.name().to_string()
        } else {
            instance_name_.to_string()
        };
        if let Some(v) = db_get_raw(
            array_,
            is_header,
            entity_or_type.as_entity(),
            name_or_id,
            index_,
        ) {
            if std::any::TypeId::of::<Blank>() == std::any::TypeId::of::<T>() {
                return v.is_empty(); // как в C++: пустое = Blank
            }
            // compare type tag
            if let Some(tag) = v.first().copied() {
                // Жёсткая проверка по TypeEncoder
                let target = if std::any::TypeId::of::<i32>() == std::any::TypeId::of::<T>() {
                    TypeEncoder::encode_type_int()
                } else if std::any::TypeId::of::<bool>() == std::any::TypeId::of::<T>() {
                    TypeEncoder::encode_type_bool()
                } else if std::any::TypeId::of::<TriBool>() == std::any::TypeId::of::<T>() {
                    TypeEncoder::encode_type_tribool()
                } else if std::any::TypeId::of::<f64>() == std::any::TypeId::of::<T>() {
                    TypeEncoder::encode_type_double()
                } else if std::any::TypeId::of::<String>() == std::any::TypeId::of::<T>() {
                    TypeEncoder::encode_type_string()
                } else if std::any::TypeId::of::<BitVec>() == std::any::TypeId::of::<T>() {
                    TypeEncoder::encode_type_bitset()
                } else if std::any::TypeId::of::<EnumerationReference>()
                    == std::any::TypeId::of::<T>()
                {
                    TypeEncoder::encode_type_enumref()
                } else if std::any::TypeId::of::<Arc<ifcutil::IfcBaseClass>>()
                    == std::any::TypeId::of::<T>()
                {
                    TypeEncoder::encode_type_ifcptr()
                } else if std::any::TypeId::of::<AggregateOfInstance>()
                    == std::any::TypeId::of::<T>()
                {
                    TypeEncoder::encode_type_agg_inst()
                } else if std::any::TypeId::of::<AggregateOfAggregateOfInstance>()
                    == std::any::TypeId::of::<T>()
                {
                    TypeEncoder::encode_type_agg_agg_inst()
                } else if std::any::TypeId::of::<Blank>() == std::any::TypeId::of::<T>() {
                    TypeEncoder::encode_type_blank()
                } else {
                    TypeEncoder::encode_type_blank()
                };
                return tag == target;
            }
        }
        false
    }
}

fn dispatch_get_string_from_enumref(
    array_: &AttributeArrayPtr,
    storage_model_: u8,
    instance_name_: usize,
    entity_or_type: &Declaration,
    index_: u8,
) -> Option<String> {
    if storage_model_ == 0 {
        // in-memory EnumRef хранится как AnyValue::EnumRef
        let v: EnumerationReference = array_.storage_ptr.storage.get(index_ as usize);
        return Some(v.value());
    } else {
        let is_header = entity_or_type.schema().is_header;
        let name_or_id = if is_header {
            entity_or_type.name().to_string()
        } else {
            instance_name_.to_string()
        };
        if let Some(buf) = db_get_raw(
            array_,
            is_header,
            entity_or_type.as_entity(),
            name_or_id,
            index_,
        ) {
            if let Some(er) =
                implrocks::ser::deserialize_enumref(array_.db_ptr.as_ref().unwrap(), &buf)
            {
                return Some(er.value());
            }
        }
    }
    None
}

fn dispatch_get_<T: TryFrom<AnyValue>>(
    array_: &AttributeArrayPtr,
    storage_model_: u8,
    instance_name_: usize,
    entity_or_type: &Declaration,
    index_: u8,
) -> T {
    if storage_model_ == 0 {
        return array_.storage_ptr.storage.get(index_ as usize);
    } else {
        // Для типов, которые требуют специальной десериализации, обрабатываем отдельно
        let is_header = entity_or_type.schema().is_header;
        let name_or_id = if is_header {
            entity_or_type.name().to_string()
        } else {
            instance_name_.to_string()
        };
        let buf = db_get_raw(
            array_,
            is_header,
            entity_or_type.as_entity(),
            name_or_id,
            index_,
        )
        .unwrap_or_default();
        // Спец-типы:
        if std::any::TypeId::of::<EnumerationReference>() == std::any::TypeId::of::<T>() {
            if let Some(er) =
                implrocks::ser::deserialize_enumref(array_.db_ptr.as_ref().unwrap(), &buf)
            {
                let any = AnyValue::EnumRef(er);
                return T::try_from(any).ok().unwrap();
            }
        }
        if std::any::TypeId::of::<Arc<ifcutil::IfcBaseClass>>() == std::any::TypeId::of::<T>() {
            // buf: [tag, kind, usize]
            if buf.first().copied().unwrap_or(0) == TypeEncoder::encode_type_ifcptr() {
                let kind = buf.get(1).copied().unwrap_or(b'i');
                let mut arr = [0u8; std::mem::size_of::<usize>()];
                arr.copy_from_slice(&buf[2..2 + arr.len()]);
                let id = usize::from_ne_bytes(arr);
                let dbs = array_.db_ptr.as_ref().unwrap();
                let ptr = if kind == b'i' {
                    dbs.assert_existance(id, implrocks::RocksDbFileStorage::ENTITYINSTANCE_REF)
                } else {
                    dbs.assert_existance(id, implrocks::RocksDbFileStorage::TYPEDECL_REF)
                };
                let any = AnyValue::IfcPtr(ptr);
                return T::try_from(any).ok().unwrap();
            }
        }
        if std::any::TypeId::of::<AggregateOfInstance>() == std::any::TypeId::of::<T>() {
            if let Some(a) =
                implrocks::ser::deserialize_agg_inst(array_.db_ptr.as_ref().unwrap(), &buf)
            {
                let any = AnyValue::AggInst(a);
                return T::try_from(any).ok().unwrap();
            }
        }
        if std::any::TypeId::of::<AggregateOfAggregateOfInstance>() == std::any::TypeId::of::<T>() {
            if let Some(a) =
                implrocks::ser::deserialize_agg_agg_inst(array_.db_ptr.as_ref().unwrap(), &buf)
            {
                let any = AnyValue::AggAggInst(a);
                return T::try_from(any).ok().unwrap();
            }
        }
        if std::any::TypeId::of::<TriBool>() == std::any::TypeId::of::<T>() {
            if let Some(tb) = implrocks::ser::deserialize_tribool(&buf) {
                let any = AnyValue::Tri(tb);
                return T::try_from(any).ok().unwrap();
            }
        }
        if std::any::TypeId::of::<BitVec>() == std::any::TypeId::of::<T>() {
            if let Some(bv) = implrocks::ser::deserialize_bitset(&buf) {
                let any = AnyValue::Bitset(bv);
                return T::try_from(any).ok().unwrap();
            }
        }
        // по умолчанию — Unsupported в этом блоке
        panic!("Unsupported DB get for requested T");
    }
}

// ======= TryFrom AnyValue для извлечений =======
macro_rules! impl_tryfrom_any {
    ($t:ty, $p:pat => $e:expr) => {
        impl TryFrom<AnyValue> for $t {
            type Error = ();
            fn try_from(v: AnyValue) -> Result<Self, Self::Error> {
                match v {
                    $p => Ok($e),
                    _ => Err(()),
                }
            }
        }
    };
}
impl_tryfrom_any!(i32, AnyValue::Int(x) => x);
impl_tryfrom_any!(bool, AnyValue::Bool(x) => x);
impl_tryfrom_any!(f64, AnyValue::Double(x) => x);
impl_tryfrom_any!(TriBool, AnyValue::Tri(x) => x);
impl_tryfrom_any!(String, AnyValue::Str(x) => x);
impl_tryfrom_any!(BitVec, AnyValue::Bitset(x) => x);
impl_tryfrom_any!(EnumerationReference, AnyValue::EnumRef(x) => x);
impl_tryfrom_any!(Arc<ifcutil::IfcBaseClass>, AnyValue::IfcPtr(x) => x);
impl_tryfrom_any!(Vec<i32>, AnyValue::VecInt(x) => x);
impl_tryfrom_any!(Vec<f64>, AnyValue::VecDouble(x) => x);
impl_tryfrom_any!(Vec<Vec<i32>>, AnyValue::VecVecInt(x) => x);
impl_tryfrom_any!(Vec<Vec<f64>>, AnyValue::VecVecDouble(x) => x);
impl_tryfrom_any!(Vec<String>, AnyValue::VecStr(x) => x);
impl_tryfrom_any!(Vec<BitVec>, AnyValue::VecBitset(x) => x);
impl_tryfrom_any!(AggregateOfInstance, AnyValue::AggInst(x) => x);
impl_tryfrom_any!(AggregateOfAggregateOfInstance, AnyValue::AggAggInst(x) => x);

// =================== «Операторы» AttributeValue ===================

impl AttributeValue {
    pub fn as_int(&self) -> i32 {
        dispatch_get_::<i32>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_bool(&self) -> bool {
        dispatch_get_::<bool>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_double(&self) -> f64 {
        dispatch_get_::<f64>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_tribool(&self) -> TriBool {
        if dispatch_has_::<bool>(
            std::marker::PhantomData,
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        ) {
            return if self.as_bool() {
                TriBool::True
            } else {
                TriBool::False
            };
        }
        dispatch_get_::<TriBool>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_string(&self) -> String {
        if dispatch_has_::<EnumerationReference>(
            std::marker::PhantomData,
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        ) {
            if let Some(s) = dispatch_get_string_from_enumref(
                &self.array_,
                self.storage_model_,
                self.instance_name_,
                &self.entity_or_type_,
                self.index_,
            ) {
                return s;
            }
        }
        dispatch_get_::<String>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_enumref(&self) -> EnumerationReference {
        dispatch_get_::<EnumerationReference>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_bitset(&self) -> BitVec {
        dispatch_get_::<BitVec>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_ifcptr(&self) -> Arc<ifcutil::IfcBaseClass> {
        dispatch_get_::<Arc<ifcutil::IfcBaseClass>>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_vec_int(&self) -> Vec<i32> {
        dispatch_get_::<Vec<i32>>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_vec_double(&self) -> Vec<f64> {
        dispatch_get_::<Vec<f64>>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_vec_vec_int(&self) -> Vec<Vec<i32>> {
        dispatch_get_::<Vec<Vec<i32>>>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_vec_vec_double(&self) -> Vec<Vec<f64>> {
        dispatch_get_::<Vec<Vec<f64>>>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_vec_string(&self) -> Vec<String> {
        dispatch_get_::<Vec<String>>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_vec_bitset(&self) -> Vec<BitVec> {
        dispatch_get_::<Vec<BitVec>>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_agg_inst(&self) -> AggregateOfInstance {
        dispatch_get_::<AggregateOfInstance>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn as_agg_agg_inst(&self) -> AggregateOfAggregateOfInstance {
        dispatch_get_::<AggregateOfAggregateOfInstance>(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }

    pub fn is_null(&self) -> bool {
        dispatch_has_::<Blank>(
            std::marker::PhantomData,
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
    pub fn size(&self) -> u32 {
        // как в C++: array_.storage_ptr->apply_visitor(SizeVisitor{}, index_)
        // у нас visitor реализован в Storage
        self.array_
            .storage_ptr
            .storage
            .apply_visitor(self.index_ as usize, &()) as u32
    }
    pub fn arg_type_index(&self) -> usize {
        dispatch_index_(
            &self.array_,
            self.storage_model_,
            self.instance_name_,
            &self.entity_or_type_,
            self.index_,
        )
    }
}

// ============== RocksDB high-level set/has API (порт шаблонов) ==============

pub struct RocksDbAttributeStorage;
impl RocksDbAttributeStorage {
    pub fn has_blank(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<Blank>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn has_int(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<i32>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn set_blank(array: &AttributeArrayPtr, decl: &Declaration, identity: usize, index: u8) {
        let buf = implrocks::ser::serialize_blank();
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_derived(array: &AttributeArrayPtr, decl: &Declaration, identity: usize, index: u8) {
        let buf = implrocks::ser::serialize_derived();
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_empty_agg(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) {
        let buf = implrocks::ser::serialize_empty_agg();
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_empty_agg_agg(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) {
        let buf = implrocks::ser::serialize_empty_agg_agg();
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_tribool(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        t: TriBool,
    ) {
        let buf = implrocks::ser::serialize_tribool(t);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_bitset(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        b: &BitVec,
    ) {
        let buf = implrocks::ser::serialize_bitset(b);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_ifcptr(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        p: &Arc<ifcutil::IfcBaseClass>,
    ) {
        let buf = implrocks::ser::serialize_ifcptr(p);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_enumref(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        e: &EnumerationReference,
    ) {
        let dbs = array.db_ptr.as_ref().expect("db");
        let buf = implrocks::ser::serialize_enumref(e, dbs.file.schema());
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_agg_inst(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        a: &AggregateOfInstance,
    ) {
        let buf = implrocks::ser::serialize_agg_inst(a);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_agg_agg_inst(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        a: &AggregateOfAggregateOfInstance,
    ) {
        let buf = implrocks::ser::serialize_agg_agg_inst(a);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
}

// ================= In-memory helpers: set for simple & vector types =================

impl MemoryAttributeStorage {
    pub fn set_int(&mut self, idx: usize, v: i32) {
        self.set(idx, AnyValue::Int(v));
    }
    pub fn set_bool(&mut self, idx: usize, v: bool) {
        self.set(idx, AnyValue::Bool(v));
    }
    pub fn set_double(&mut self, idx: usize, v: f64) {
        self.set(idx, AnyValue::Double(v));
    }
    pub fn set_string<S: Into<String>>(&mut self, idx: usize, v: S) {
        self.set(idx, AnyValue::Str(v.into()));
    }
    pub fn set_tribool(&mut self, idx: usize, v: TriBool) {
        self.set(idx, AnyValue::Tri(v));
    }
    pub fn set_bitset(&mut self, idx: usize, v: BitVec) {
        self.set(idx, AnyValue::Bitset(v));
    }
    pub fn set_enumref(&mut self, idx: usize, v: EnumerationReference) {
        self.set(idx, AnyValue::EnumRef(v));
    }
    pub fn set_ifcptr(&mut self, idx: usize, v: Arc<ifcutil::IfcBaseClass>) {
        self.set(idx, AnyValue::IfcPtr(v));
    }
    pub fn set_vec_int(&mut self, idx: usize, v: Vec<i32>) {
        self.set(idx, AnyValue::VecInt(v));
    }
    pub fn set_vec_double(&mut self, idx: usize, v: Vec<f64>) {
        self.set(idx, AnyValue::VecDouble(v));
    }
    pub fn set_vec_vec_int(&mut self, idx: usize, v: Vec<Vec<i32>>) {
        self.set(idx, AnyValue::VecVecInt(v));
    }
    pub fn set_vec_vec_double(&mut self, idx: usize, v: Vec<Vec<f64>>) {
        self.set(idx, AnyValue::VecVecDouble(v));
    }
    pub fn set_vec_string(&mut self, idx: usize, v: Vec<String>) {
        self.set(idx, AnyValue::VecStr(v));
    }
    pub fn set_vec_bitset(&mut self, idx: usize, v: Vec<BitVec>) {
        self.set(idx, AnyValue::VecBitset(v));
    }
    pub fn set_agg_inst(&mut self, idx: usize, v: AggregateOfInstance) {
        self.set(idx, AnyValue::AggInst(v));
    }
    pub fn set_agg_agg_inst(&mut self, idx: usize, v: AggregateOfAggregateOfInstance) {
        self.set(idx, AnyValue::AggAggInst(v));
    }
}

// ================= RocksDB: serialize helpers for simple & vector =================

mod rdb_simple_ser {
    use super::*;

    pub fn ser_i32(v: i32) -> Vec<u8> {
        let mut out = vec![TypeEncoder::encode_type_int()];
        out.extend_from_slice(&v.to_ne_bytes());
        out
    }
    pub fn ser_bool(v: bool) -> Vec<u8> {
        vec![TypeEncoder::encode_type_bool(), if v { 1 } else { 0 }]
    }
    pub fn ser_f64(v: f64) -> Vec<u8> {
        let mut out = vec![TypeEncoder::encode_type_double()];
        out.extend_from_slice(&v.to_ne_bytes());
        out
    }
    pub fn ser_string(s: &str) -> Vec<u8> {
        let mut out = vec![TypeEncoder::encode_type_string()];
        out.extend_from_slice(s.as_bytes());
        out
    }

    pub fn ser_vec_i32(v: &[i32]) -> Vec<u8> {
        let mut out = vec![TypeEncoder::encode_type_vec_int()];
        let len = v.len();
        out.extend_from_slice(&len.to_ne_bytes());
        for x in v {
            out.extend_from_slice(&x.to_ne_bytes());
        }
        out
    }
    pub fn ser_vec_f64(v: &[f64]) -> Vec<u8> {
        let mut out = vec![TypeEncoder::encode_type_vec_double()];
        let len = v.len();
        out.extend_from_slice(&len.to_ne_bytes());
        for x in v {
            out.extend_from_slice(&x.to_ne_bytes());
        }
        out
    }
    pub fn ser_vec_string(v: &[String]) -> Vec<u8> {
        let mut out = vec![TypeEncoder::encode_type_vec_string()];
        let len = v.len();
        out.extend_from_slice(&len.to_ne_bytes());
        for s in v {
            let slen = s.as_bytes().len();
            out.extend_from_slice(&slen.to_ne_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        out
    }
    pub fn ser_vec_vec_i32(v: &[Vec<i32>]) -> Vec<u8> {
        let mut out = vec![TypeEncoder::encode_type_vec_vec_int()];
        let outer = v.len();
        out.extend_from_slice(&outer.to_ne_bytes());
        for inner in v {
            let il = inner.len();
            out.extend_from_slice(&il.to_ne_bytes());
            for x in inner {
                out.extend_from_slice(&x.to_ne_bytes());
            }
        }
        out
    }
    pub fn ser_vec_vec_f64(v: &[Vec<f64>]) -> Vec<u8> {
        let mut out = vec![TypeEncoder::encode_type_vec_vec_double()];
        let outer = v.len();
        out.extend_from_slice(&outer.to_ne_bytes());
        for inner in v {
            let il = inner.len();
            out.extend_from_slice(&il.to_ne_bytes());
            for x in inner {
                out.extend_from_slice(&x.to_ne_bytes());
            }
        }
        out
    }
    pub fn ser_vec_bitset(v: &[BitVec]) -> Vec<u8> {
        let mut out = vec![TypeEncoder::encode_type_vec_bitset()];
        let outer = v.len();
        out.extend_from_slice(&outer.to_ne_bytes());
        for bv in v {
            let s: String = bv.iter().map(|b| if *b { '1' } else { '0' }).collect();
            let sl = s.as_bytes().len();
            out.extend_from_slice(&sl.to_ne_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        out
    }

    pub fn de_i32(buf: &[u8]) -> Option<i32> {
        if buf.first().copied()? != TypeEncoder::encode_type_int() {
            return None;
        }
        let mut a = [0u8; 4];
        a.copy_from_slice(&buf[1..5]);
        Some(i32::from_ne_bytes(a))
    }
    pub fn de_bool(buf: &[u8]) -> Option<bool> {
        if buf.first().copied()? != TypeEncoder::encode_type_bool() {
            return None;
        }
        Some(buf.get(1).copied()? != 0)
    }
    pub fn de_f64(buf: &[u8]) -> Option<f64> {
        if buf.first().copied()? != TypeEncoder::encode_type_double() {
            return None;
        }
        let mut a = [0u8; 8];
        a.copy_from_slice(&buf[1..9]);
        Some(f64::from_ne_bytes(a))
    }
    pub fn de_string(buf: &[u8]) -> Option<String> {
        if buf.first().copied()? != TypeEncoder::encode_type_string() {
            return None;
        }
        Some(String::from_utf8(buf[1..].to_vec()).ok()?)
    }

    fn read_usize(buf: &[u8], off: &mut usize) -> usize {
        let mut a = [0u8; std::mem::size_of::<usize>()];
        a.copy_from_slice(&buf[*off..*off + a.len()]);
        *off += a.len();
        usize::from_ne_bytes(a)
    }
    pub fn de_vec_i32(buf: &[u8]) -> Option<Vec<i32>> {
        if buf.first().copied()? != TypeEncoder::encode_type_vec_int() {
            return None;
        }
        let mut off = 1;
        let n = read_usize(buf, &mut off);
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let mut a = [0u8; 4];
            a.copy_from_slice(&buf[off..off + 4]);
            off += 4;
            out.push(i32::from_ne_bytes(a));
        }
        Some(out)
    }
    pub fn de_vec_f64(buf: &[u8]) -> Option<Vec<f64>> {
        if buf.first().copied()? != TypeEncoder::encode_type_vec_double() {
            return None;
        }
        let mut off = 1;
        let n = read_usize(buf, &mut off);
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let mut a = [0u8; 8];
            a.copy_from_slice(&buf[off..off + 8]);
            off += 8;
            out.push(f64::from_ne_bytes(a));
        }
        Some(out)
    }
    pub fn de_vec_string(buf: &[u8]) -> Option<Vec<String>> {
        if buf.first().copied()? != TypeEncoder::encode_type_vec_string() {
            return None;
        }
        let mut off = 1;
        let n = read_usize(buf, &mut off);
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let sl = read_usize(buf, &mut off);
            let s = std::str::from_utf8(&buf[off..off + sl]).ok()?.to_string();
            off += sl;
            out.push(s);
        }
        Some(out)
    }
    pub fn de_vec_vec_i32(buf: &[u8]) -> Option<Vec<Vec<i32>>> {
        if buf.first().copied()? != TypeEncoder::encode_type_vec_vec_int() {
            return None;
        }
        let mut off = 1;
        let outer = read_usize(buf, &mut off);
        let mut out = Vec::with_capacity(outer);
        for _ in 0..outer {
            let il = read_usize(buf, &mut off);
            let mut inner = Vec::with_capacity(il);
            for _ in 0..il {
                let mut a = [0u8; 4];
                a.copy_from_slice(&buf[off..off + 4]);
                off += 4;
                inner.push(i32::from_ne_bytes(a));
            }
            out.push(inner);
        }
        Some(out)
    }
    pub fn de_vec_vec_f64(buf: &[u8]) -> Option<Vec<Vec<f64>>> {
        if buf.first().copied()? != TypeEncoder::encode_type_vec_vec_double() {
            return None;
        }
        let mut off = 1;
        let outer = read_usize(buf, &mut off);
        let mut out = Vec::with_capacity(outer);
        for _ in 0..outer {
            let il = read_usize(buf, &mut off);
            let mut inner = Vec::with_capacity(il);
            for _ in 0..il {
                let mut a = [0u8; 8];
                a.copy_from_slice(&buf[off..off + 8]);
                off += 8;
                inner.push(f64::from_ne_bytes(a));
            }
            out.push(inner);
        }
        Some(out)
    }
    pub fn de_vec_bitset(buf: &[u8]) -> Option<Vec<BitVec>> {
        if buf.first().copied()? != TypeEncoder::encode_type_vec_bitset() {
            return None;
        }
        let mut off = 1;
        let outer = read_usize(buf, &mut off);
        let mut out = Vec::with_capacity(outer);
        for _ in 0..outer {
            let sl = read_usize(buf, &mut off);
            let s = std::str::from_utf8(&buf[off..off + sl]).ok()?;
            off += sl;
            let mut bv = BitVec::new();
            for ch in s.chars() {
                bv.push(ch == '1');
            }
            out.push(bv);
        }
        Some(out)
    }
}

// ============== RocksDB Attribute Storage: set/has for simple & vector ==============

impl RocksDbAttributeStorage {
    pub fn set_int(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        v: i32,
    ) {
        let buf = rdb_simple_ser::ser_i32(v);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_bool(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        v: bool,
    ) {
        let buf = rdb_simple_ser::ser_bool(v);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_double(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        v: f64,
    ) {
        let buf = rdb_simple_ser::ser_f64(v);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_string(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        s: &str,
    ) {
        let buf = rdb_simple_ser::ser_string(s);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }

    pub fn set_vec_int(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        v: &[i32],
    ) {
        let buf = rdb_simple_ser::ser_vec_i32(v);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_vec_double(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        v: &[f64],
    ) {
        let buf = rdb_simple_ser::ser_vec_f64(v);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_vec_string(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        v: &[String],
    ) {
        let buf = rdb_simple_ser::ser_vec_string(v);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_vec_vec_int(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        v: &[Vec<i32>],
    ) {
        let buf = rdb_simple_ser::ser_vec_vec_i32(v);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_vec_vec_double(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        v: &[Vec<f64>],
    ) {
        let buf = rdb_simple_ser::ser_vec_vec_f64(v);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }
    pub fn set_vec_bitset(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
        v: &[BitVec],
    ) {
        let buf = rdb_simple_ser::ser_vec_bitset(v);
        let is_header = decl.schema().is_header;
        let name_or_id = if is_header {
            decl.name().to_string()
        } else {
            identity.to_string()
        };
        db_put_raw(array, is_header, decl.as_entity(), name_or_id, index, &buf);
    }

    pub fn has_int(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<i32>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn has_bool(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<bool>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn has_double(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<f64>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn has_string(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<String>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn has_vec_int(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<Vec<i32>>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn has_vec_double(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<Vec<f64>>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn has_vec_string(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<Vec<String>>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn has_vec_vec_int(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<Vec<Vec<i32>>>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn has_vec_vec_double(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<Vec<Vec<f64>>>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
    pub fn has_vec_bitset(
        array: &AttributeArrayPtr,
        decl: &Declaration,
        identity: usize,
        index: u8,
    ) -> bool {
        dispatch_has_::<Vec<BitVec>>(std::marker::PhantomData, array, 1, identity, decl, index)
    }
}

// ============== AttributeValue удобные конструкторы ==============

impl AttributeValue {
    pub fn mem(
        storage: Arc<MemoryAttributeStorage>,
        instance_name: usize,
        decl: Declaration,
        index: u8,
    ) -> Self {
        Self::new_mem(
            AttributeArrayPtr::from_memory(storage),
            instance_name,
            decl,
            index,
        )
    }
    pub fn with_db(self, db: Arc<DB>, file: Arc<IfcFileShim>) -> Self {
        let array = self.array_.with_db(db, file);
        Self {
            array_: array,
            ..self
        }
    }
}

// ======================= Self-check (по желанию) =======================
// #[cfg(test)]
// mod tests {
//     use super::*;
//     #[test]
//     fn roundtrip_mem() {
//         let mut mem = MemoryAttributeStorage::new();
//         mem.set_int(0, 42);
//         mem.set_string(1, "hello");
//         let storage = Arc::new(mem);
//         let decl = Declaration::new_entity("IFCFOO", Schema::new("schema", false));
//         let av0 = AttributeValue::mem(storage.clone(), 100, decl.clone(), 0);
//         let av1 = AttributeValue::mem(storage.clone(), 100, decl.clone(), 1);
//         assert_eq!(av0.as_int(), 42);
//         assert_eq!(av1.as_string(), "hello");
//     }
// }
