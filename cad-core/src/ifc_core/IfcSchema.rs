// ifc_core/src/schema.rs

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use once_cell::sync::Lazy;

// ==== ВНЕШНИЕ ТИПЫ/ТРЕЙТЫ, которые уже есть у тебя в порте ==================

// Представление данных экземпляра (как в C++)
pub struct IfcEntityInstanceData; // твоя реальная структура

// Базовый класс сущностей (как в C++)
pub trait IfcBaseClass: Send + Sync {}

// Позднесвязанная сущность, если фабрика не задана
pub struct IfcLateBoundEntity {
    pub decl: Arc<dyn Declaration>,
    pub data: IfcEntityInstanceData,
}
impl IfcBaseClass for IfcLateBoundEntity {}

// Исключения
#[derive(Debug)]
pub struct IfcException(pub String);

impl std::fmt::Display for IfcException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for IfcException {}

// ==== МОДЕЛЬ ДЕКЛАРАЦИЙ, как в IfcSchema.h ===================================

pub trait Entity: Send + Sync {
    /// Прямой суперкласс (если есть)
    fn supertype(&self) -> Option<Arc<dyn Declaration>>;
    /// Для удобства: число атрибутов и т.д., если нужно — добавишь
}

pub trait TypeDeclaration: Send + Sync {
    fn declared_type(&self) -> Arc<dyn ParameterType>;
}

pub trait SelectType: Send + Sync {}
pub trait EnumerationType: Send + Sync {}

/// parameter_type из C++ — объединяет named/aggregation и т.п.
/// Для этого файла нам нужна только named_type.
pub trait ParameterType: Send + Sync {
    fn as_named_type(&self) -> Option<Arc<dyn NamedType>> {
        None
    }
}

pub trait NamedType: ParameterType {
    fn declared_type(&self) -> Arc<dyn Declaration>;

    /// C++: bool named_type::is(name) { return declared_type()->is(name); }
    fn is_name(&self, name: &str) -> bool {
        self.declared_type().is_name(name)
    }
    /// C++: bool named_type::is(decl) { return declared_type()->is(decl); }
    fn is_decl(&self, decl: &dyn Declaration) -> bool {
        self.declared_type().is_decl(decl)
    }
}

/// Главный интерфейс декларации
pub trait Declaration: Send + Sync {
    fn name(&self) -> &str;
    fn name_upper(&self) -> &str;
    fn schema(&self) -> Arc<SchemaDefinition>;

    fn as_entity(&self) -> Option<Arc<dyn Entity>> {
        None
    }
    fn as_type_declaration(&self) -> Option<Arc<dyn TypeDeclaration>> {
        None
    }
    fn as_select_type(&self) -> Option<Arc<dyn SelectType>> {
        None
    }
    fn as_enumeration_type(&self) -> Option<Arc<dyn EnumerationType>> {
        None
    }

    /// C++: declaration::is(name)
    fn is_name(&self, name: &str) -> bool {
        // В C++ приводится к upper-case, если встречается нижний регистр
        let has_lower = name.chars().any(|c| c.is_ascii_lowercase());
        let cand = if has_lower {
            let mut s = name.to_string();
            s.make_ascii_uppercase();
            s
        } else {
            name.to_string()
        };
        if self.name_upper() == cand {
            return true;
        }
        // поднимаемся по суперклассам
        if let Some(ent) = self.as_entity() {
            if let Some(super_decl) = ent.supertype() {
                if super_decl.is_name(&cand) {
                    return true;
                }
            }
        }
        // следуем через типы
        if let Some(tdecl) = self.as_type_declaration() {
            if let Some(named) = tdecl.declared_type().as_named_type() {
                if named.is_name(&cand) {
                    return true;
                }
            }
        }
        false
    }

    /// C++: declaration::is(decl)
    fn is_decl(&self, decl: &dyn Declaration) -> bool {
        // быстрый путь — указатели совпадают (в Rust — сравнение по имени/схеме)
        if std::ptr::eq(self as *const _, decl as *const _) {
            return true;
        }
        if let Some(ent) = self.as_entity() {
            if let Some(super_decl) = ent.supertype() {
                if super_decl.is_decl(decl) {
                    return true;
                }
            }
        }
        if let Some(tdecl) = self.as_type_declaration() {
            if let Some(named) = tdecl.declared_type().as_named_type() {
                if named.is_decl(decl) {
                    return true;
                }
            }
        }
        false
    }
}

// ==== ФАБРИКА ИНСТАНЦИЙ ======================================================

pub trait InstanceFactory: Send + Sync {
    fn instantiate(&self, decl: Arc<dyn Declaration>, data: IfcEntityInstanceData)
                   -> Box<dyn IfcBaseClass>;
}

// ==== ОПРЕДЕЛЕНИЕ СХЕМЫ ======================================================

pub struct SchemaDefinition {
    name: String,
    declarations: Vec<Arc<dyn Declaration>>,
    factory: Option<Arc<dyn InstanceFactory>>,

    // кэши подмножества типов, как в C++
    type_declarations: Vec<Arc<dyn TypeDeclaration>>,
    select_types: Vec<Arc<dyn SelectType>>,
    enumeration_types: Vec<Arc<dyn EnumerationType>>,
    entities: Vec<Arc<dyn Entity>>,
}

impl SchemaDefinition {
    pub fn new(
        name: impl Into<String>,
        mut decls: Vec<Arc<dyn Declaration>>,
        factory: Option<Arc<dyn InstanceFactory>>,
    ) -> Arc<Self> {
        // в C++ сортировка по индексу; здесь сортируем по имени — если у тебя есть index(),
        // можно заменить ключ сортировки.
        decls.sort_by(|a, b| a.name_upper().cmp(b.name_upper()));

        let mut type_decls = Vec::new();
        let mut selects = Vec::new();
        let mut enums = Vec::new();
        let mut ents = Vec::new();

        for d in &decls {
            // это аналог присвоения (**it).schema_ = this; — у нас schema() должен возвращать Arc<Self>,
            // так что схему «привяжет» фабрика/построитель деклараций (или предоставь setter, если нужно).
            if let Some(td) = d.as_type_declaration() {
                type_decls.push(td);
            }
            if let Some(st) = d.as_select_type() {
                selects.push(st);
            }
            if let Some(et) = d.as_enumeration_type() {
                enums.push(et);
            }
            if let Some(en) = d.as_entity() {
                ents.push(en);
            }
        }

        let sd = Arc::new(Self {
            name: name.into(),
            declarations: decls,
            factory,
            type_declarations: type_decls,
            select_types: selects,
            enumeration_types: enums,
            entities: ents,
        });

        // регистрируем (как в конструкторе C++)
        register_schema(sd.clone());
        sd
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn declarations(&self) -> &Vec<Arc<dyn Declaration>> {
        &self.declarations
    }

    pub fn instantiate(
        self: &Arc<Self>,
        decl: Arc<dyn Declaration>,
        data: IfcEntityInstanceData,
    ) -> Box<dyn IfcBaseClass> {
        if let Some(f) = &self.factory {
            f.instantiate(decl, data)
        } else {
            Box::new(IfcLateBoundEntity { decl, data })
        }
    }
}

// ==== ГЛОБАЛЬНЫЙ РЕЕСТР СХЕМ =================================================

static SCHEMAS: Lazy<RwLock<HashMap<String, Arc<SchemaDefinition>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn register_schema(schema: Arc<SchemaDefinition>) {
    let key = schema.name().to_ascii_uppercase();
    let mut map = SCHEMAS.write().unwrap();
    map.insert(key, schema);
}

pub fn schema_by_name(name: &str) -> Result<Arc<SchemaDefinition>, IfcException> {
    // лениво ensure-м горимпортим схемы (как #ifdef + get_schema())
    ensure_schemas_loaded();

    let key = name.to_ascii_uppercase();
    let map = SCHEMAS.read().unwrap();
    map.get(&key)
        .cloned()
        .ok_or_else(|| IfcException(format!("No schema named {}", name)))
}

pub fn schema_names() -> Vec<String> {
    // аналог C++: пытаемся прогрузить хотя бы IFC2X3 для побочных регистраций
    let _ = schema_by_name("IFC2X3");
    let map = SCHEMAS.read().unwrap();
    map.keys().cloned().collect()
}

pub fn clear_schemas() {
    // дергаем clear для модулей (как в C++)
    clear_schema_modules();

    // чистим реестр
    let mut map = SCHEMAS.write().unwrap();
    map.clear();
}

// ==== ЛЕНИВАЯ ИНИЦИАЛИЗАЦИЯ МОДУЛЕЙ СХЕМ ====================================

fn ensure_schemas_loaded() {
    #[cfg(feature = "schema_2x3")]
    ifc2x3::get_schema();
    #[cfg(feature = "schema_4")]
    ifc4::get_schema();
    #[cfg(feature = "schema_4x1")]
    ifc4x1::get_schema();
    #[cfg(feature = "schema_4x2")]
    ifc4x2::get_schema();
    #[cfg(feature = "schema_4x3_rc1")]
    ifc4x3_rc1::get_schema();
    #[cfg(feature = "schema_4x3_rc2")]
    ifc4x3_rc2::get_schema();
    #[cfg(feature = "schema_4x3_rc3")]
    ifc4x3_rc3::get_schema();
    #[cfg(feature = "schema_4x3_rc4")]
    ifc4x3_rc4::get_schema();
    #[cfg(feature = "schema_4x3")]
    ifc4x3::get_schema();
    #[cfg(feature = "schema_4x3_tc1")]
    ifc4x3_tc1::get_schema();
    #[cfg(feature = "schema_4x3_add1")]
    ifc4x3_add1::get_schema();
    #[cfg(feature = "schema_4x3_add2")]
    ifc4x3_add2::get_schema();
}

fn clear_schema_modules() {
    #[cfg(feature = "schema_2x3")]
    ifc2x3::clear_schema();
    #[cfg(feature = "schema_4")]
    ifc4::clear_schema();
    #[cfg(feature = "schema_4x1")]
    ifc4x1::clear_schema();
    #[cfg(feature = "schema_4x2")]
    ifc4x2::clear_schema();
    #[cfg(feature = "schema_4x3_rc1")]
    ifc4x3_rc1::clear_schema();
    #[cfg(feature = "schema_4x3_rc2")]
    ifc4x3_rc2::clear_schema();
    #[cfg(feature = "schema_4x3_rc3")]
    ifc4x3_rc3::clear_schema();
    #[cfg(feature = "schema_4x3_rc4")]
    ifc4x3_rc4::clear_schema();
    #[cfg(feature = "schema_4x3")]
    ifc4x3::clear_schema();
    #[cfg(feature = "schema_4x3_tc1")]
    ifc4x3_tc1::clear_schema();
    #[cfg(feature = "schema_4x3_add1")]
    ifc4x3_add1::clear_schema();
    #[cfg(feature = "schema_4x3_add2")]
    ifc4x3_add2::clear_schema();
}

// ==== ИНТЕРФЕЙСЫ МОДУЛЕЙ СХЕМ (как твои Ifc2x3::get_schema и т.п.) ==========
// Эти модули должны быть предоставлены соответствующими крейтами/модулями.
// Сигнатуры ровно под вызовы выше. Внутри они создают SchemaDefinition::new(...)
// и регистрируют декларации через register_schema() — один-в-один с твоим C++.

#[cfg(feature = "schema_2x3")]
pub mod ifc2x3 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4")]
pub mod ifc4 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4x1")]
pub mod ifc4x1 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4x2")]
pub mod ifc4x2 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4x3_rc1")]
pub mod ifc4x3_rc1 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4x3_rc2")]
pub mod ifc4x3_rc2 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4x3_rc3")]
pub mod ifc4x3_rc3 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4x3_rc4")]
pub mod ifc4x3_rc4 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4x3")]
pub mod ifc4x3 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4x3_tc1")]
pub mod ifc4x3_tc1 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4x3_add1")]
pub mod ifc4x3_add1 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
#[cfg(feature = "schema_4x3_add2")]
pub mod ifc4x3_add2 {
    pub fn get_schema() {}
    pub fn clear_schema() {}
}
