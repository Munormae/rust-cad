// cad-core/ifc_core/src/parse_context.rs

use std::{fs, io::Read, path::Path};

use crate::attr::{
    IfcEntityInstanceData, InMemoryAttributeStorage, MutableAttributeValue, RocksDbAttributeStorage,
};
use crate::errors::{IfcException, IfcInvalidTokenException};
use crate::file::{file_open_status, FileHeader, IfcFile};
use crate::ifcutil::{IfcBaseClass, IfcLateBoundEntity};
use crate::lexer::{Lexer, Token, TokenFunc, TokenType};
use crate::logger::Logger;
use crate::schema::{AggregationType, Declaration, EntityDecl, NamedType, ParameterType, Schema};
use crate::types::{
    aggregate_of_aggregate_of_instance, aggregate_of_instance, reference_or_simple_type, BitVec,
    Blank, Derived, EnumerationReference, TriBool,
};

#[cfg(feature = "rocksdb")]
use rocksdb::{BlockBasedOptions, ColumnFamilyDescriptor, MergeOperands, Options, Status, DB};

#[derive(Default)]
pub struct ParseContext {
    pub tokens: Vec<PcItem>,
}

pub enum PcItem {
    Tok(Token),
    Ctx(Box<ParseContext>),
    Inst(*mut IfcBaseClass),
}

impl Drop for ParseContext {
    fn drop(&mut self) {
    }
}

impl ParseContext {
    pub fn push_ctx(&mut self) -> &mut ParseContext {
        self.tokens
            .push(PcItem::Ctx(Box::new(ParseContext::default())));
        match self.tokens.last_mut().unwrap() {
            PcItem::Ctx(b) => b.as_mut(),
            _ => unreachable!(),
        }
    }
    pub fn push_token(&mut self, t: Token) {
        self.tokens.push(PcItem::Tok(t));
    }
    pub fn push_inst(&mut self, inst: *mut IfcBaseClass) {
        self.tokens.push(PcItem::Inst(inst));
    }
}

fn dispatch_token<F: FnMut(Dispatched)>(
    instance_id: i32,
    attribute_id: i32,
    t: &Token,
    decl: Option<&Declaration>,
    mut fnc: F,
) {
    match t.tt {
        TokenType::Binary => fnc(Dispatched::Binary(TokenFunc::as_binary(t))),
        _ if TokenFunc::is_bool(t) => fnc(Dispatched::Bool(TokenFunc::as_bool(t))),
        _ if TokenFunc::is_logical(t) => fnc(Dispatched::Logical(TokenFunc::as_logical(t))),
        TokenType::Enumeration => {
            let s = TokenFunc::as_string_ref(t);
            if let Some(d) = decl.and_then(|d| d.as_enumeration_type()) {
                match d.lookup_enum_offset(&s) {
                    Ok(idx) => fnc(Dispatched::Enum(EnumerationReference::new(d, idx))),
                    Err(_) => Logger::error(format!(
                        "An enumeration literal '{}' is not valid for type '{}' at offset {}",
                        s,
                        d.name(),
                        t.start_pos
                    )),
                }
            } else {
                Logger::error(format!(
                    "An enumeration literal '{}' is not expected at attribute index '{}' at offset {}",
                    s, attribute_id, t.start_pos
                ));
            }
        }
        TokenType::Float => fnc(Dispatched::Float(TokenFunc::as_float(t))),
        TokenType::Identifier => {
            fnc(Dispatched::RefOrSimple(
                reference_or_simple_type::from_identifier(
                    TokenFunc::as_identifier(t) as usize,
                    t.start_pos as usize,
                ),
            ));
        }
        TokenType::Int => fnc(Dispatched::Int(TokenFunc::as_int(t))),
        TokenType::String => fnc(Dispatched::String(TokenFunc::as_string_ref(t))),
        TokenType::Operator if t.value_char == '*' => fnc(Dispatched::Derived),
        _ => {}
    }
}

enum Dispatched {
    Binary(BitVec),
    Bool(bool),
    Logical(TriBool),
    Enum(EnumerationReference),
    Float(f64),
    Int(i32),
    String(String),
    RefOrSimple(reference_or_simple_type),
    Derived,
}

// вспомогательный variant под агрегации
enum AggVariant {
    None,
    VecI(Vec<i32>),
    VecD(Vec<f64>),
    VecS(Vec<String>),
    VecB(Vec<BitVec>),
    VecRef(Vec<reference_or_simple_type>),
    VecVecI(Vec<Vec<i32>>),
    VecVecD(Vec<Vec<f64>>),
    VecVecRef(Vec<Vec<reference_or_simple_type>>),
}

impl AggVariant {
    fn push_scalar(&mut self, v: Dispatched) {
        match v {
            Dispatched::Int(x) => match self {
                AggVariant::None => *self = AggVariant::VecI(vec![x]),
                AggVariant::VecI(vv) => vv.push(x),
                AggVariant::VecD(vv) => vv.push(x as f64),
                AggVariant::VecVecD(_) | AggVariant::VecVecI(_) => {
                    Logger::error("Inconsistent aggregate valuation (scalar into nested)".into())
                }
                _ => Logger::error("Inconsistent aggregate valuation".into()),
            },
            Dispatched::Float(x) => match self {
                AggVariant::None => *self = AggVariant::VecD(vec![x]),
                AggVariant::VecD(vv) => vv.push(x),
                AggVariant::VecI(vv) => {
                    let mut nd: Vec<f64> = vv.iter().map(|&i| i as f64).collect();
                    nd.push(x);
                    *self = AggVariant::VecD(nd);
                }
                AggVariant::VecVecI(_) | AggVariant::VecVecD(_) => {
                    Logger::error("Inconsistent aggregate valuation (scalar into nested)".into())
                }
                _ => Logger::error("Inconsistent aggregate valuation".into()),
            },
            Dispatched::String(s) => match self {
                AggVariant::None => *self = AggVariant::VecS(vec![s]),
                AggVariant::VecS(vv) => vv.push(s),
                _ => Logger::error("Inconsistent aggregate valuation".into()),
            },
            Dispatched::Binary(b) => match self {
                AggVariant::None => *self = AggVariant::VecB(vec![b]),
                AggVariant::VecB(vv) => vv.push(b),
                _ => Logger::error("Inconsistent aggregate valuation".into()),
            },
            Dispatched::RefOrSimple(r) => match self {
                AggVariant::None => *self = AggVariant::VecRef(vec![r]),
                AggVariant::VecRef(vv) => vv.push(r),
                _ => Logger::error("Inconsistent aggregate valuation".into()),
            },
            Dispatched::Bool(_)
            | Dispatched::Logical(_)
            | Dispatched::Enum(_)
            | Dispatched::Derived => {
                Logger::error("Aggregates of this scalar type not supported in parser".into())
            }
        }
    }
    fn push_inner_vec_i(&mut self, v: Vec<i32>) {
        match self {
            AggVariant::None => *self = AggVariant::VecVecI(vec![v]),
            AggVariant::VecVecI(vv) => vv.push(v),
            AggVariant::VecVecD(vv) => {
                let vd: Vec<f64> = v.into_iter().map(|i| i as f64).collect();
                vv.push(vd);
            }
            _ => Logger::error("Inconsistent aggregate valuation (int[] into other)".into()),
        }
    }
    fn push_inner_vec_d(&mut self, v: Vec<f64>) {
        match self {
            AggVariant::None => *self = AggVariant::VecVecD(vec![v]),
            AggVariant::VecVecD(vv) => vv.push(v),
            AggVariant::VecVecI(vv) => {
                // upgrade int[][] -> double[][]
                let mut up: Vec<Vec<f64>> = vv
                    .iter()
                    .map(|inner| inner.iter().map(|&i| i as f64).collect())
                    .collect();
                up.push(v);
                *self = AggVariant::VecVecD(up);
            }
            _ => Logger::error("Inconsistent aggregate valuation (double[] into other)".into()),
        }
    }
    fn push_inner_vec_ref(&mut self, v: Vec<reference_or_simple_type>) {
        match self {
            AggVariant::None => *self = AggVariant::VecVecRef(vec![v]),
            AggVariant::VecVecRef(vv) => vv.push(v),
            _ => Logger::error("Inconsistent aggregate valuation (ref[] into other)".into()),
        }
    }
}

fn construct_rec<const DEPTH: usize, F: FnOnce(AggOut)>(
    instance_id: i32,
    attribute_id: i32,
    p: &ParseContext,
    aggr: Option<&AggregationType>,
    f: F,
) {
    if p.tokens.is_empty() {
        if let Some(ag) = aggr {
            use crate::ifcutil::ArgumentType::*;
            let a = crate::ifcutil::make_aggregate(crate::ifcutil::from_parameter_type(
                ag.type_of_element(),
            ));
            let out = match a {
                AggregateOfInt => AggOut::VecI(vec![]),
                AggregateOfDouble => AggOut::VecD(vec![]),
                AggregateOfString => AggOut::VecS(vec![]),
                AggregateOfBinary => AggOut::VecB(vec![]),
                AggregateOfEntityInstance => AggOut::AggInst(aggregate_of_instance::new()),
                AggregateOfAggregateOfInt => AggOut::VecVecI(vec![]),
                AggregateOfAggregateOfDouble => AggOut::VecVecD(vec![]),
                AggregateOfAggregateOfEntityInstance => {
                    AggOut::AggAggInst(aggregate_of_aggregate_of_instance::new())
                }
                _ => AggOut::Blank,
            };
            f(out);
        }
        return;
    }

    let mut store = AggVariant::None;

    for it in &p.tokens {
        match it {
            PcItem::Tok(t) => {
                let named_decl = aggr
                    .and_then(|a| a.type_of_element().as_named_type())
                    .map(|nt| nt.declared_type());
                dispatch_token(instance_id, attribute_id, t, named_decl, |val| {
                    store.push_scalar(val)
                });
            }
            PcItem::Ctx(ctx) => {
                if DEPTH < 3 {
                    let mut inner_store = AggVariant::None;
                    construct_rec::<{ DEPTH + 1 }>(instance_id, attribute_id, ctx, None, |out| {
                        match out {
                            AggOut::VecI(v) => inner_store.push_inner_vec_i(v),
                            AggOut::VecD(v) => inner_store.push_inner_vec_d(v),
                            AggOut::VecRef(v) => inner_store.push_inner_vec_ref(v),
                            _ => Logger::error("Unsupported nested aggregate kind".into()),
                        }
                    });
                    // слить inner_store в store как элемент
                    match inner_store {
                        AggVariant::VecI(v) => {
                            if let AggVariant::None = store {
                                store = AggVariant::VecVecI(vec![]);
                            }
                            if let AggVariant::VecVecI(_) | AggVariant::VecVecD(_) = store {
                                store.push_inner_vec_i(v);
                            } else {
                                Logger::error("Nested int[] into non-nested aggregate".into());
                            }
                        }
                        AggVariant::VecD(v) => {
                            if let AggVariant::None = store {
                                store = AggVariant::VecVecD(vec![]);
                            }
                            if let AggVariant::VecVecI(_) | AggVariant::VecVecD(_) = store {
                                store.push_inner_vec_d(v);
                            } else {
                                Logger::error("Nested double[] into non-nested aggregate".into());
                            }
                        }
                        AggVariant::VecRef(v) => {
                            if let AggVariant::None = store {
                                store = AggVariant::VecVecRef(vec![]);
                            }
                            if let AggVariant::VecVecRef(_) = store {
                                store.push_inner_vec_ref(v);
                            } else {
                                Logger::error("Nested ref[] into non-nested aggregate".into());
                            }
                        }
                        _ => {}
                    }
                }
            }
            PcItem::Inst(ptr) => {
                // как в С++: упаковываем как reference_or_simple_type
                let r = reference_or_simple_type::from_instance_ptr(*ptr);
                store.push_scalar(Dispatched::RefOrSimple(r));
            }
        }
    }

    let out = match store {
        AggVariant::None => AggOut::Blank,
        AggVariant::VecI(v) => AggOut::VecI(v),
        AggVariant::VecD(v) => AggOut::VecD(v),
        AggVariant::VecS(v) => AggOut::VecS(v),
        AggVariant::VecB(v) => AggOut::VecB(v),
        AggVariant::VecRef(v) => AggOut::VecRef(v),
        AggVariant::VecVecI(v) => AggOut::VecVecI(v),
        AggVariant::VecVecD(v) => AggOut::VecVecD(v),
        AggVariant::VecVecRef(v) => AggOut::VecVecRef(v),
    };
    f(out);
}

enum AggOut {
    Blank,
    VecI(Vec<i32>),
    VecD(Vec<f64>),
    VecS(Vec<String>),
    VecB(Vec<BitVec>),
    VecRef(Vec<reference_or_simple_type>),
    VecVecI(Vec<Vec<i32>>),
    VecVecD(Vec<Vec<f64>>),
    VecVecRef(Vec<Vec<reference_or_simple_type>>),
}

impl ParseContext {
    #[allow(clippy::too_many_arguments)]
    pub fn construct(
        &self,
        name: i32,
        references_to_resolve: &mut Vec<(MutableAttributeValue, Vec<reference_or_simple_type>)>,
        decl: Option<&Declaration>,
        expected_size: Option<usize>,
        resolve_reference_index: i32,
        coerce_attribute_count: bool,
    ) -> IfcEntityInstanceData {
        let mut param_types: Vec<&ParameterType> = vec![];
        let mut transient_named_type: Option<NamedType> = None;

        if let Some(d) = decl {
            if let Some(td) = d.as_type_declaration() {
                param_types = vec![td.declared_type()];
            } else if d.as_enumeration_type().is_some() {
                transient_named_type = Some(NamedType::transient(d.clone()));
                param_types = vec![transient_named_type.as_ref().unwrap()];
            } else if let Some(e) = d.as_entity() {
                param_types = e
                    .all_attributes()
                    .into_iter()
                    .map(|a| a.type_of_attribute())
                    .collect();
            }
        }

        if (decl.is_some() && self.tokens.len() != param_types.len())
            || expected_size
                .map(|e| e != self.tokens.len())
                .unwrap_or(false)
        {
            let expected = expected_size.unwrap_or(param_types.len());
            Logger::warning(format!(
                "Expected {} attribute values, found {} for instance #{}",
                expected,
                self.tokens.len(),
                if name > 0 { name } else { 0 }
            ));
        }

        if self.tokens.is_empty() {
            return IfcEntityInstanceData::from_memory(InMemoryAttributeStorage::new(0));
        }

        let capacity = if coerce_attribute_count {
            if let Some(d) = decl {
                std::cmp::min(param_types.len(), self.tokens.len())
            } else {
                self.tokens.len()
            }
        } else {
            self.tokens.len()
        };

        let mut storage = InMemoryAttributeStorage::new(capacity);

        let mut kt = param_types.iter();
        for (idx, token) in self.tokens.iter().enumerate() {
            let index = idx as u8;
            let param_type = if decl.is_some() {
                *kt.next().unwrap_or(&ParameterType::UNIT)
            } else {
                ParameterType::UNIT
            };

            match token {
                PcItem::Tok(t) => {
                    let named_decl = param_type
                        .as_named_type()
                        .map(|nt| nt.declared_type().clone());
                    dispatch_token(
                        name,
                        index as i32,
                        t,
                        named_decl.as_ref(),
                        |val| match val {
                            Dispatched::RefOrSimple(r) => {
                                if name > 0 {
                                    let idx_for = if resolve_reference_index == -1 {
                                        index
                                    } else {
                                        resolve_reference_index as u8
                                    };
                                    references_to_resolve.push((
                                        MutableAttributeValue {
                                            instance_name: name as usize,
                                            index: idx_for,
                                        },
                                        vec![r],
                                    ));
                                }
                            }
                            Dispatched::Int(v) => storage.set_int(index as usize, v),
                            Dispatched::Float(v) => storage.set_double(index as usize, v),
                            Dispatched::String(v) => storage.set_string(index as usize, v),
                            Dispatched::Binary(v) => storage.set_bitset(index as usize, v),
                            Dispatched::Bool(v) => storage.set_bool(index as usize, v),
                            Dispatched::Logical(v) => storage.set_tribool(index as usize, v),
                            Dispatched::Enum(v) => storage.set_enumref(index as usize, v),
                            Dispatched::Derived => {
                                storage.set(index as usize, crate::attr::AnyValue::Derived(Derived))
                            }
                        },
                    );
                }
                PcItem::Ctx(ctx) => {
                    // развернуть агрегацию
                    let mut pt = &param_type;
                    while let Some(nt) = pt.as_named_type() {
                        if let Some(td) = nt.declared_type().as_type_declaration() {
                            pt = td.declared_type();
                        } else {
                            break;
                        }
                    }
                    let aggr = pt.as_aggregation_type();

                    construct_rec::<0>(name, index as i32, ctx, aggr, |out| match out {
                        AggOut::VecRef(v) => {
                            if name > 0 {
                                let idx_for = if resolve_reference_index == -1 {
                                    index
                                } else {
                                    resolve_reference_index as u8
                                };
                                references_to_resolve.push((
                                    MutableAttributeValue {
                                        instance_name: name as usize,
                                        index: idx_for,
                                    },
                                    v,
                                ));
                            }
                        }
                        AggOut::VecVecRef(v) => {
                            if name > 0 {
                                let idx_for = if resolve_reference_index == -1 {
                                    index
                                } else {
                                    resolve_reference_index as u8
                                };
                                // расплющим как в оригинале — один атрибут со списком ссылок
                                let flat: Vec<reference_or_simple_type> =
                                    v.into_iter().flatten().collect();
                                references_to_resolve.push((
                                    MutableAttributeValue {
                                        instance_name: name as usize,
                                        index: idx_for,
                                    },
                                    flat,
                                ));
                            }
                        }
                        AggOut::VecI(v) => storage.set_vec_int(index as usize, v),
                        AggOut::VecD(v) => storage.set_vec_double(index as usize, v),
                        AggOut::VecS(v) => storage.set_vec_string(index as usize, v),
                        AggOut::VecB(v) => storage.set_vec_bitset(index as usize, v),
                        AggOut::VecVecI(v) => storage.set_vec_vec_int(index as usize, v),
                        AggOut::VecVecD(v) => storage.set_vec_vec_double(index as usize, v),
                        AggOut::Blank => storage.set_blank(index as usize),
                    });
                }
                PcItem::Inst(ptr) => {
                    let r = reference_or_simple_type::from_instance_ptr(*ptr);
                    if name > 0 {
                        let idx_for = if resolve_reference_index == -1 {
                            index
                        } else {
                            resolve_reference_index as u8
                        };
                        references_to_resolve.push((
                            MutableAttributeValue {
                                instance_name: name as usize,
                                index: idx_for,
                            },
                            vec![r],
                        ));
                    }
                }
            }
        }

        IfcEntityInstanceData::from_memory(storage)
    }
}

// -------- guess_file_type --------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    IfcSpf,
    RocksDb,
    Unknown,
}

pub fn guess_file_type(path: &str) -> FileType {
    let p = Path::new(path);
    if !p.exists() {
        return FileType::IfcSpf; // как в оригинале
    }
    return if p.is_dir() {
        let current = p.join("CURRENT");
        if !current.exists() || !current.is_file() {
            return FileType::Unknown;
        }
        let mut s = String::new();
        if let Ok(mut f) = fs::File::open(&current) {
            if f.read_to_string(&mut s).is_ok() {
                if s.trim_start().starts_with("MANIFEST-") {
                    return FileType::RocksDb;
                }
            }
        }
        FileType::Unknown
    } else {
        FileType::IfcSpf
    };
}

// -------- InstanceStreamer::read_instance (упрощённо, под наш лексер/стейт) --------

pub struct InstanceStreamer<'a> {
    pub good: file_open_status,
    pub header: Option<FileHeader>,
    pub yielded_header_instances: u8,
    pub lexer: &'a mut Lexer,
    pub schema: &'a Schema,
    pub storage: crate::parser_storage::LoaderBackend<'a>,
    pub token_stream: [Token; 3],
    pub progress: usize,
    pub references_to_resolve: Vec<(MutableAttributeValue, Vec<reference_or_simple_type>)>,
    pub coerce_attribute_count: bool,
}

impl<'a> InstanceStreamer<'a> {
    pub fn read_instance(&mut self) -> Option<(usize, &'a Declaration, IfcEntityInstanceData)> {
        if let Some(h) = &mut self.header {
            if self.yielded_header_instances < 3 {
                let res = match self.yielded_header_instances {
                    0 => (
                        0usize,
                        h.file_description().declaration(),
                        h.file_description().take_data(),
                    ),
                    1 => (
                        0usize,
                        h.file_name().declaration(),
                        h.file_name().take_data(),
                    ),
                    _ => (
                        0usize,
                        h.file_schema().declaration(),
                        h.file_schema().take_data(),
                    ),
                };
                self.yielded_header_instances += 1;
                return Some(res);
            }
        }

        let mut current_id: u32 = 0;
        while self.good.is_ok() && !self.lexer.stream.eof && current_id == 0 {
            if self.token_stream[0].tt == TokenType::Identifier
                && self.token_stream[1].tt == TokenType::Operator
                && self.token_stream[1].value_char == '='
                && self.token_stream[2].tt == TokenType::Keyword
            {
                current_id = TokenFunc::as_identifier(&self.token_stream[0]) as u32;
                let ent_name = TokenFunc::as_string_ref(&self.token_stream[2]);
                let entity_type = match self.schema.declaration_by_name(&ent_name) {
                    Ok(d) => d,
                    Err(e) => {
                        Logger::error(format!(
                            "{} at offset {}",
                            e, self.token_stream[2].start_pos
                        ));
                        self.advance_token_stream();
                        continue;
                    }
                };
                if entity_type.as_entity().is_none() {
                    Logger::error(format!(
                        "Non entity type {} at offset {}",
                        entity_type.name(),
                        self.token_stream[2].start_pos
                    ));
                    self.advance_token_stream();
                    continue;
                }

                let mut ps = ParseContext::default();
                self.lexer.next(); // сдвинуть после KEYWORD
                if let Err(e) = self.storage.load(
                    current_id as i32,
                    entity_type.as_entity().unwrap(),
                    &mut ps,
                    -1,
                ) {
                    self.good = file_open_status::InvalidSyntax;
                    Logger::error(e.to_string());
                    break;
                }

                self.progress += 1;
                if self.progress % 1000 == 0 {
                    Logger::status(format!("\r#{}", current_id), false);
                }

                let data = ps.construct(
                    current_id as i32,
                    &mut self.references_to_resolve,
                    Some(entity_type),
                    None,
                    -1,
                    self.coerce_attribute_count,
                );
                return Some((current_id as usize, entity_type, data));
            }

            let next = match self.lexer.next_safe() {
                Ok(t) => t,
                Err(e) => {
                    Logger::error(format!("{}. Parsing terminated", e));
                    self.good = file_open_status::InvalidSyntax;
                    break;
                }
            };
            if !self.lexer.stream.eof && next.tt == TokenType::None {
                self.good = file_open_status::InvalidSyntax;
                break;
            }
            self.token_stream.rotate_left(1);
            self.token_stream[2] = next;
        }
        None
    }

    fn advance_token_stream(&mut self) {
        let _ = self.lexer.next_safe().map(|t| {
            self.token_stream.rotate_left(1);
            self.token_stream[2] = t;
        });
    }
}

// -------- RocksDB-backed file storage glue --------

#[cfg(feature = "rocksdb")]
pub struct RocksDbFileStorage {
    pub file: *mut IfcFile,
    pub db: *mut DB,
    pub wopts: rocksdb::WriteOptions,

    pub instance_cache: std::collections::HashMap<usize, *mut IfcBaseClass>,
    pub type_instance_cache: std::collections::HashMap<usize, *mut IfcBaseClass>,
}

#[cfg(feature = "rocksdb")]
impl RocksDbFileStorage {
    pub fn new(path: &str, file: *mut IfcFile, readonly: bool) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        // ZSTD если доступен, иначе NoCompression — как в C++
        if rocksdb::supported_compressions().contains(&rocksdb::DBCompressionType::Zstd) {
            opts.set_compression_type(rocksdb::DBCompressionType::Zstd);
        }

        let db = if readonly {
            DB::open_for_read_only(&opts, path, false).ok()
        } else {
            DB::open(&opts, path).ok()
        }
        .expect("failed to open RocksDB");

        let mut wopts = rocksdb::WriteOptions::default();
        wopts.disable_wal(true);

        Self {
            file,
            db: Box::into_raw(Box::new(db)),
            wopts,
            instance_cache: Default::default(),
            type_instance_cache: Default::default(),
        }
    }

    pub unsafe fn drop_db(&mut self) {
        if !self.db.is_null() {
            let db = Box::from_raw(self.db);
            // flush + compact
            let _ = db.flush();
            let _ = db.compact_range::<&[u8], &[u8]>(None, None);
            self.db = std::ptr::null_mut();
        }
    }

    pub fn instance_by_id(&mut self, id: i32) -> *mut IfcBaseClass {
        self.assert_existance(id as usize, InstanceRef::EntityInstanceRef)
    }

    pub fn assert_existance(&mut self, number: usize, r: InstanceRef) -> *mut IfcBaseClass {
        if r == InstanceRef::EntityInstanceRef {
            if let Some(p) = self.instance_cache.get(&number) {
                return *p;
            }
        } else if let Some(p) = self.type_instance_cache.get(&number) {
            return *p;
        }

        let key = format!(
            "{}|{}|_",
            if r == InstanceRef::EntityInstanceRef {
                "i"
            } else {
                "t"
            },
            number
        );
        let mut val = Vec::<u8>::new();
        unsafe {
            let db = &*self.db;
            if let Ok(Some(v)) = db.get(key.as_bytes()) {
                val = v.to_vec();
            } else {
                panic!("Instance #{} not found", number);
            }
        }

        let mut s_bytes = [0u8; std::mem::size_of::<usize>()];
        s_bytes.copy_from_slice(&val[..s_bytes.len()]);
        let decl_idx = usize::from_ne_bytes(s_bytes);

        let schema = unsafe { &*(*self.file).schema };
        if decl_idx >= schema.declarations().len() {
            panic!("declaration index OOB");
        }
        let decl = schema.declarations()[decl_idx];
        let is_entity = decl.as_entity().is_some();
        if is_entity != (r == InstanceRef::EntityInstanceRef) {
            panic!("Incorrect reference kind");
        }

        let data = IfcEntityInstanceData::from_rocksdb(RocksDbAttributeStorage::default());
        let inst: *mut IfcBaseClass = unsafe {
            if (*self.file).instantiate_typed_instances {
                schema.instantiate(decl, data)
            } else {
                Box::into_raw(Box::new(IfcLateBoundEntity::new(decl, data)))
            }
        };
        unsafe {
            (*inst).id_ = number as i32;
            (*inst).file_ = self.file;
        }
        if r == InstanceRef::EntityInstanceRef {
            self.instance_cache.insert(number, inst);
        } else {
            self.type_instance_cache.insert(number, inst);
        }
        inst
    }

    pub fn process_deletion_inverse(&mut self, inst: *mut IfcBaseClass) {
        unsafe {
            let id = (*inst).id_;
            let prefix = format!("v|{}|", id);
            let db = &*self.db;

            // Удаляем диапазон v|id|*
            let mut it = db.raw_iterator();
            it.seek(prefix.as_bytes());
            let mut end_key = Vec::<u8>::new();
            while it.valid() {
                if !it
                    .key()
                    .map(|k| k.starts_with(prefix.as_bytes()))
                    .unwrap_or(false)
                {
                    if let Some(k) = it.key() {
                        end_key = k.to_vec();
                    }
                    break;
                }
                it.next();
            }
            let mut batch = rocksdb::WriteBatch::default();
            if !end_key.is_empty() {
                batch.delete_range(prefix.as_bytes(), &end_key);
            } else {
                // до конца
                batch.delete_range(prefix.as_bytes(), &[]);
            }
            let _ = db.write_opt(&self.wopts, &batch);

            // Обновление инверсий как в С++
            let attrs = crate::traverse::traverse(inst, 1);
            for entity_attr in attrs.iter() {
                let name = unsafe { (**entity_attr).id_ };
                if name == id || name == 0 {
                    continue;
                }

                // v|name|* — пройти и выкинуть current id
                let prefix2 = format!("v|{}|", name);
                let mut it2 = db.raw_iterator();
                it2.seek(prefix2.as_bytes());
                while it2.valid() {
                    if !it2
                        .key()
                        .map(|k| k.starts_with(prefix2.as_bytes()))
                        .unwrap_or(false)
                    {
                        break;
                    }
                    let k = it2.key().unwrap().to_vec();
                    let mut s = it2.value().unwrap().to_vec();
                    let cnt = s.len() / std::mem::size_of::<usize>();
                    let mut vals = vec![0usize; cnt];
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            s.as_ptr(),
                            vals.as_mut_ptr() as *mut u8,
                            s.len(),
                        );
                    }
                    if let Some(pos) = vals.iter().position(|&x| x == id as usize) {
                        vals.remove(pos);
                        let mut ns = vec![0u8; vals.len() * std::mem::size_of::<usize>()];
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                vals.as_ptr() as *const u8,
                                ns.as_mut_ptr(),
                                ns.len(),
                            );
                        }
                        let _ = db.put_opt(&self.wopts, &k, ns);
                    }
                    it2.next();
                }
            }
        }
    }
}

#[cfg(feature = "rocksdb")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InstanceRef {
    EntityInstanceRef,
    TypeDeclRef,
}

// -------- in-memory file storage glue --------

pub struct InMemoryFileStorage<'a> {
    pub byid: std::collections::HashMap<i32, *mut IfcBaseClass>,
    pub file: &'a mut IfcFile,
}

impl<'a> InMemoryFileStorage<'a> {
    pub fn instance_by_id(&self, id: i32) -> *mut IfcBaseClass {
        self.byid.get(&id).copied().unwrap_or_else(|| {
            panic!("Instance #{} not found", id);
        })
    }
}

// -------- IfcFile create/DTors glue (частично) --------

impl Drop for IfcFile {
    fn drop(&mut self) {
        // как в C++: освобождаем byid_
        for (_, ptr) in self.byid_.drain() {
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    }
}

impl IfcFile {
    pub fn create_in_rocksdb(&mut self, decl: &Declaration) -> *mut IfcBaseClass {
        if decl.as_entity().is_some() || decl.as_type_declaration().is_some() {
            let inst = self.schema.instantiate(
                decl,
                IfcEntityInstanceData::from_rocksdb(RocksDbAttributeStorage::default()),
            );
            unsafe {
                (*inst).file_ = self as *mut _;
            }
            self.add_entity(inst)
        } else {
            panic!("Requires an entity or type declaration");
        }
    }

    pub fn create_in_memory(&mut self, decl: &Declaration) -> *mut IfcBaseClass {
        let inst = if let Some(ent) = decl.as_entity() {
            self.schema.instantiate(
                decl,
                IfcEntityInstanceData::from_memory(InMemoryAttributeStorage::new(
                    ent.attribute_count(),
                )),
            )
        } else if decl.as_type_declaration().is_some() {
            self.schema.instantiate(
                decl,
                IfcEntityInstanceData::from_memory(InMemoryAttributeStorage::new(1)),
            )
        } else {
            panic!("Requires an entity or type declaration");
        };
        unsafe {
            (*inst).file_ = self as *mut _;
        }
        self.add_entity(inst)
    }
}
