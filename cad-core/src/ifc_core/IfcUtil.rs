// src/ifc_util.rs

use std::collections::{HashSet, HashMap};
use std::sync::Arc;
use std::cmp::min;
use std::fs;

/// ====== ВНЕШНИЕ ТИПЫ/ТРЕЙТЫ (тонкие заглушки для интеграции) ===============

pub trait Declaration: Send + Sync {
    fn name(&self) -> &str;
    fn is_name(&self, name: &str) -> bool;
    fn is_decl(&self, other: &dyn Declaration) -> bool;
    fn as_entity(&self) -> Option<Arc<dyn Entity>> { None }
}

pub trait Entity: Send + Sync {
    fn all_attributes(&self) -> Vec<Arc<dyn AttributeDecl>>;
    fn all_inverse_attributes(&self) -> Vec<Arc<dyn InverseAttribute>>;
    fn derived(&self) -> Vec<bool>; // true = derived
}

pub trait AttributeDecl: Send + Sync {
    fn name(&self) -> &str;
}

pub trait InverseAttribute: Send + Sync {
    fn name(&self) -> &str;
    fn entity_reference(&self) -> Arc<dyn Entity>;
    fn attribute_reference(&self) -> Arc<dyn AttributeDecl>;
    fn attribute_index(&self, of: Arc<dyn AttributeDecl>) -> usize;
}

pub struct IfcEntityInstanceData; // твоя реальная структура

pub enum AttributeValueSetArg {
    Derived, // эквивалент C++ Derived{}
    // TODO: сюда добавь остальные варианты по необходимости
}

pub trait IfcFileLike: Send + Sync {
    fn get_inverse(&self, id: usize, ent_ref: Arc<dyn Entity>, attr_index: i32)
                   -> AggregateOfInstancePtr;
}

pub trait IfcBaseClass: Send + Sync {
    fn id(&self) -> usize;
    fn file(&self) -> Option<Arc<dyn IfcFileLike>>;
    fn declaration(&self) -> Arc<dyn Declaration>;
    fn get_attribute_value(&self, index: usize) -> AttributeValue; // см. ниже
    fn set_attribute_value(&self, index: usize, v: AttributeValueSetArg);
}

#[derive(Clone)]
pub struct AttributeValue {
    // прокси на чтение (в C++ это возвращает view)
    // TODO: заполни конкретикой
}

#[derive(Debug)]
pub struct IfcException(pub String);

impl std::fmt::Display for IfcException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}
impl std::error::Error for IfcException {}

/// ====== aggregate_of_instance =================================================

pub type AggregateOfInstancePtr = Arc<AggregateOfInstance>;

#[derive(Default)]
pub struct AggregateOfInstance {
    list_: Vec<Arc<dyn IfcBaseClass>>,
}
impl AggregateOfInstance {
    pub fn new() -> Self { Self { list_: Vec::new() } }

    pub fn push_inst(&mut self, instance: Option<Arc<dyn IfcBaseClass>>) {
        if let Some(i) = instance {
            self.list_.push(i);
        }
    }
    pub fn push_aggregate(&mut self, other: Option<AggregateOfInstancePtr>) {
        if let Some(a) = other {
            for i in &a.list_ {
                self.list_.push(i.clone());
            }
        }
    }
    pub fn size(&self) -> u32 { self.list_.len() as u32 }
    pub fn reserve(&mut self, capacity: usize) { self.list_.reserve(capacity); }
    pub fn get(&self, i: usize) -> Arc<dyn IfcBaseClass> { self.list_[i].clone() }
    pub fn contains(&self, instance: &Arc<dyn IfcBaseClass>) -> bool {
        self.list_.iter().any(|p| Arc::ptr_eq(p, instance))
    }
    pub fn remove(&mut self, instance: &Arc<dyn IfcBaseClass>) {
        self.list_.retain(|p| !Arc::ptr_eq(p, instance));
    }

    /// filtered(): вернуть те, что **не** принадлежат `entities`
    pub fn filtered(&self, entities: &HashSet<*const dyn Declaration>) -> AggregateOfInstancePtr {
        let mut out = AggregateOfInstance::new();
        for it in &self.list_ {
            let decl = it.declaration();
            let mut contained = false;
            for dptr in entities {
                // сравнение по адресу декларации (как указатели в C++)
                let d: &dyn Declaration = unsafe { &**dptr };
                if decl.is_decl(d) {
                    contained = true;
                    break;
                }
            }
            if !contained {
                out.push_inst(Some(it.clone()));
            }
        }
        Arc::new(out)
    }

    /// unique(): удалить дубликаты (по указателю/identity)
    pub fn unique(&self) -> AggregateOfInstancePtr {
        let mut seen: HashSet<usize> = HashSet::new(); // по id()
        let mut out = AggregateOfInstance::new();
        for it in &self.list_ {
            let id = it.id();
            if seen.insert(id) {
                out.push_inst(Some(it.clone()));
            }
        }
        Arc::new(out)
    }

    pub fn iter(&self) -> impl Iterator<Item=&Arc<dyn IfcBaseClass>> { self.list_.iter() }
}

/// ====== ArgumentType и строковое представление ===============================

#[derive(Copy, Clone, Debug)]
pub enum ArgumentType {
    Null,
    Derived,
    Int,
    Bool,
    Logical,
    Double,
    String_,
    Binary,
    Enumeration,
    EntityInstance,

    EmptyAggregate,
    AggregateOfInt,
    AggregateOfDouble,
    AggregateOfString,
    AggregateOfBinary,
    AggregateOfEntityInstance,

    AggregateOfEmptyAggregate,
    AggregateOfAggregateOfInt,
    AggregateOfAggregateOfDouble,
    AggregateOfAggregateOfEntityInstance,

    Unknown,
}

pub fn argument_type_to_string(t: ArgumentType) -> &'static str {
    use ArgumentType::*;
    match t {
        Null => "NULL",
        Derived => "DERIVED",
        Int => "INT",
        Bool => "BOOL",
        Logical => "LOGICAL",
        Double => "DOUBLE",
        String_ => "STRING",
        Binary => "BINARY",
        Enumeration => "ENUMERATION",
        EntityInstance => "ENTITY INSTANCE",

        EmptyAggregate => "EMPTY AGGREGATE",
        AggregateOfInt => "AGGREGATE OF INT",
        AggregateOfDouble => "AGGREGATE OF DOUBLE",
        AggregateOfString => "AGGREGATE OF STRING",
        AggregateOfBinary => "AGGREGATE OF BINARY",
        AggregateOfEntityInstance => "AGGREGATE OF ENTITY INSTANCE",

        AggregateOfEmptyAggregate => "AGGREGATE OF EMPTY AGGREGATE",
        AggregateOfAggregateOfInt => "AGGREGATE OF AGGREGATE OF INT",
        AggregateOfAggregateOfDouble => "AGGREGATE OF AGGREGATE OF DOUBLE",
        AggregateOfAggregateOfEntityInstance => "AGGREGATE OF AGGREGATE OF ENTITY INSTANCE",

        Unknown => "UNKNOWN",
    }
}

/// ====== строки/имена/escape ==================================================

pub fn valid_binary_string(s: &str) -> bool {
    s.bytes().all(|b| b == b'0' || b == b'1')
}

pub fn sanitate_material_name(s: &mut String) {
    *s = s.replace(' ', "_");
}

pub fn escape_xml(s: &mut String) {
    // порядок важен: сначала & -> &amp;
    *s = s.replace('&', "&amp;");
    *s = s.replace('"', "&quot;");
    *s = s.replace('\'', "&apos;");
    *s = s.replace('<', "&lt;");
    *s = s.replace('>', "&gt;");
}

pub fn unescape_xml(s: &mut String) {
    *s = s.replace("&quot;", "\"");
    *s = s.replace("&apos;", "'");
    *s = s.replace("&lt;", "<");
    *s = s.replace("&gt;", ">");
    *s = s.replace("&amp;", "&");
}

/// ====== IfcBaseEntity: populate_derived, get, get_inverse ====================

pub trait IfcBaseEntityExt: IfcBaseClass {
    fn populate_derived(&self) {
        if let Some(ent) = self.declaration().as_entity() {
            let derived = ent.derived();
            for (idx, is_derived) in derived.iter().enumerate() {
                if *is_derived {
                    self.set_attribute_value(idx, AttributeValueSetArg::Derived);
                }
            }
        }
    }

    fn get(&self, name: &str) -> Result<AttributeValue, IfcException> {
        let decl = self.declaration();
        let ent = decl.as_entity()
            .ok_or_else(|| IfcException(format!("{} is not an entity", decl.name())))?;
        let attrs = ent.all_attributes();
        for (idx, a) in attrs.iter().enumerate() {
            if a.name() == name {
                return Ok(self.get_attribute_value(idx));
            }
        }
        Err(IfcException(format!("{} not found on {}", name, decl.name())))
    }

    fn get_inverse(&self, name: &str) -> Result<AggregateOfInstancePtr, IfcException> {
        let file = self.file().ok_or_else(|| IfcException("Instance not added to file".into()))?;
        let decl = self.declaration();
        let ent = decl.as_entity()
            .ok_or_else(|| IfcException(format!("{} is not an entity", decl.name())))?;
        let invs = ent.all_inverse_attributes();
        for inv in invs {
            if inv.name() == name {
                let idx = inv.attribute_index(inv.attribute_reference()) as i32;
                return Ok(file.get_inverse(self.id(), inv.entity_reference(), idx));
            }
        }
        Err(IfcException(format!("{} not found on {}", name, decl.name())))
    }
}

// blanket impl для всех IfcBaseClass
impl<T: IfcBaseClass + ?Sized> IfcBaseEntityExt for T {}

/// ====== make_aggregate / from_parameter_type =================================

pub fn make_aggregate(elem: ArgumentType) -> ArgumentType {
    use ArgumentType::*;
    match elem {
        Int => AggregateOfInt,
        Double => AggregateOfDouble,
        String_ => AggregateOfString,
        Binary => AggregateOfBinary,
        EntityInstance => AggregateOfEntityInstance,
        AggregateOfInt => AggregateOfAggregateOfInt,
        AggregateOfDouble => AggregateOfAggregateOfDouble,
        AggregateOfEntityInstance => AggregateOfAggregateOfEntityInstance,
        EmptyAggregate => AggregateOfEmptyAggregate,
        _ => Unknown,
    }
}

// минимальные трейты для анализа типов (соответствуют IfcParse::*)
pub trait ParameterType {
    fn as_aggregation_type(&self) -> Option<&dyn AggregationType> { None }
    fn as_named_type(&self) -> Option<&dyn NamedTypeP> { None }
    fn as_simple_type(&self) -> Option<&dyn SimpleType> { None }
}
pub trait AggregationType {
    fn type_of_element(&self) -> &dyn ParameterType;
}
pub trait NamedTypeP {
    fn declared_type(&self) -> &dyn Declaration;
}
#[derive(Copy, Clone, Debug)]
pub enum SimpleTypeKind { Binary, Boolean, Integer, Logical, Number, Real, String_ }
pub trait SimpleType {
    fn declared_type(&self) -> SimpleTypeKind;
}

pub fn from_parameter_type(pt: &dyn ParameterType) -> ArgumentType {
    if let Some(at) = pt.as_aggregation_type() {
        return make_aggregate(from_parameter_type(at.type_of_element()));
    }
    if let Some(nt) = pt.as_named_type() {
        let dt = nt.declared_type();
        if dt.as_entity().is_some() {
            return ArgumentType::EntityInstance;
        }
        // перечисление/селект – трактуются как entity/enum. Здесь оставляем enum как ENUMERATION:
        // для select обычно mапится на entity instance в рантайме
        // TODO: если нужно — добавь as_enumeration_type/as_select_type в Declaration
        if dt.is_name("ENUM") { // условный маркер, замени на реальную проверку
            return ArgumentType::Enumeration;
        }
        // тип-объявление: спускаемся внутрь — вызывающая сторона должна передать real pt
        // Здесь уступка: без доступа к вложенному pt (как в C++) вернуть Unknown нельзя —
        // но в реальном коде твой NamedTypeP должен давать путь внутрь
        // Оставим Unknown, если добраться нельзя.
        return ArgumentType::Unknown;
    }
    if let Some(st) = pt.as_simple_type() {
        return match st.declared_type() {
            SimpleTypeKind::Binary  => ArgumentType::Binary,
            SimpleTypeKind::Boolean => ArgumentType::Bool,
            SimpleTypeKind::Integer => ArgumentType::Int,
            SimpleTypeKind::Logical => ArgumentType::Logical,
            SimpleTypeKind::Number  => ArgumentType::Double,
            SimpleTypeKind::Real    => ArgumentType::Double,
            SimpleTypeKind::String_ => ArgumentType::String_,
        };
    }
    ArgumentType::Unknown
}

/// ====== path utils (rename/delete) ===========================================

pub mod path {
    use super::*;

    pub fn rename_file(old_filename: &str, new_filename: &str) -> bool {
        // повторяем семантику: сначала удалить цель, затем переименовать
        let _ = delete_file(new_filename);
        fs::rename(old_filename, new_filename).is_ok()
    }

    pub fn delete_file(filename: &str) -> bool {
        match fs::remove_file(filename) {
            Ok(_) => true,
            Err(e) => {
                // на некоторых платформах возвращает NotFound — совместимо с C++
                e.kind() != std::io::ErrorKind::NotFound
            }
        }
    }
}
