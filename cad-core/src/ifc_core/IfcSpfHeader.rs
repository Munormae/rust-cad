// src/ifc_spf_header.rs

use std::fmt::Write as _;
use std::io::Write;
use std::sync::Arc;

/// =================== ВНЕШНИЕ ТИПЫ / ЗАГЛУШКИ ДЛЯ ИНТЕГРАЦИИ =================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType { None, Identifier, Operator, Keyword, String_ }

#[derive(Clone, Debug)]
pub struct Token {
    pub t: TokenType,
    pub value_char: Option<char>,
    pub start_pos: usize,
    pub s: String, // для идентификаторов/строк/ключевых слов
}

pub trait SpfLexer: Send + Sync {
    fn next(&mut self) -> Token;
    fn eof(&self) -> bool;
}

pub struct IfcException(pub String);
impl std::fmt::Display for IfcException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}
impl std::fmt::Debug for IfcException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}
impl std::error::Error for IfcException {}

pub struct Logger;
impl Logger {
    pub fn error<E: std::fmt::Display>(e: E) { eprintln!("[error] {}", e); }
}

#[derive(Clone)]
pub struct IfcEntityInstanceData; // твоя реальная структура

pub trait AttributeStorageFactory {
    fn make_in_memory(&self, attr_count: usize) -> IfcEntityInstanceData;
    fn make_rocks(&self) -> IfcEntityInstanceData;
}

pub trait HeaderSchema: Send + Sync {
    fn file_description_name_uc(&self) -> &'static str;
    fn file_name_name_uc(&self) -> &'static str;
    fn file_schema_name_uc(&self) -> &'static str;

    fn new_file_description(&self, data: IfcEntityInstanceData) -> Box<dyn FileDescription>;
    fn new_file_name(&self, data: IfcEntityInstanceData) -> Box<dyn FileName>;
    fn new_file_schema(&self, data: IfcEntityInstanceData) -> Box<dyn FileSchema>;

    fn file_description_attr_count(&self) -> usize;
    fn file_name_attr_count(&self) -> usize;
    fn file_schema_attr_count(&self) -> usize;
}

// минимальные интерфейсы для записи заголовка
pub trait HeaderEntity: Send + Sync {
    fn to_string_spf(&self, out: &mut dyn std::fmt::Write, with_name: bool);
    fn set_file_ptr(&mut self, _file: Option<Arc<IfcFile>>) {}
}
pub trait FileDescription: HeaderEntity {}
pub trait FileName: HeaderEntity {}
pub trait FileSchema: HeaderEntity {}

/// Варианты стораджа файла
#[derive(Clone)]
pub enum FileStorage {
    InMemory(Arc<InMemoryStorage>),
    RocksDb(Arc<RocksDbStorage>),
}
#[derive(Clone)]
pub struct InMemoryStorage {
    pub tokens: Arc<std::sync::Mutex<Box<dyn SpfLexer>>>,
    pub attr_factory: Arc<dyn AttributeStorageFactory + Send + Sync>,
    pub references_to_resolve: Arc<()>, // заглушка
}
#[derive(Clone)]
pub struct RocksDbStorage {
    pub attr_factory: Arc<dyn AttributeStorageFactory + Send + Sync>,
}

pub struct IfcFile {
    pub storage: FileStorage,
    pub header_schema: Arc<dyn HeaderSchema>,
}

/// ===== TokenFunc аналоги =====
struct TokenFunc;
impl TokenFunc {
    fn is_operator(t: &Token, c: char) -> bool {
        t.t == TokenType::Operator && t.value_char == Some(c)
    }
    fn as_string_ref(t: &Token) -> &str {
        &t.s
    }
}

/// ========================= Сам IfcSpfHeader (порт) ===========================

const ISO_10303_21: &str = "ISO-10303-21";
const HEADER: &str = "HEADER";
const ENDSEC: &str = "ENDSEC";
const DATA: &str = "DATA";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Trail { None, TrailingSemicolon }

pub struct IfcSpfHeader {
    file: Option<Arc<IfcFile>>,
    // В C++ хранится только in_memory указатель, для rocks — None.
    storage_inmem: Option<Arc<InMemoryStorage>>,

    file_description: Option<Box<dyn FileDescription>>,
    file_name: Option<Box<dyn FileName>>,
    file_schema: Option<Box<dyn FileSchema>>,
}

impl IfcSpfHeader {
    /// C++: IfcSpfHeader(IfcFile* file)
    pub fn with_file(file: Arc<IfcFile>) -> Self {
        let storage_inmem = match &file.storage {
            FileStorage::InMemory(mem) => Some(mem.clone()),
            _ => None,
        };

        // Если rocksdb — создадим «пустые» инстансы через rocks фабрику.
        let (mut fd, mut fnm, mut fsc) = if storage_inmem.is_none() {
            let schema = &file.header_schema;
            let data_fd = match &file.storage {
                FileStorage::RocksDb(r) => r.attr_factory.make_rocks(),
                FileStorage::InMemory(m) => m.attr_factory.make_in_memory(schema.file_description_attr_count()),
            };
            let data_fn = match &file.storage {
                FileStorage::RocksDb(r) => r.attr_factory.make_rocks(),
                FileStorage::InMemory(m) => m.attr_factory.make_in_memory(schema.file_name_attr_count()),
            };
            let data_fs = match &file.storage {
                FileStorage::RocksDb(r) => r.attr_factory.make_rocks(),
                FileStorage::InMemory(m) => m.attr_factory.make_in_memory(schema.file_schema_attr_count()),
            };
            (
                Some(schema.new_file_description(data_fd)),
                Some(schema.new_file_name(data_fn)),
                Some(schema.new_file_schema(data_fs)),
            )
        } else {
            // file == null ветка из C++: создаём пустые объекты (позже перезапишем в setDefaultHeaderValues)
            let schema = &file.header_schema;
            let data_fd = match &file.storage {
                FileStorage::InMemory(m) => m.attr_factory.make_in_memory(schema.file_description_attr_count()),
                FileStorage::RocksDb(r) => r.attr_factory.make_rocks(),
            };
            let data_fn = match &file.storage {
                FileStorage::InMemory(m) => m.attr_factory.make_in_memory(schema.file_name_attr_count()),
                FileStorage::RocksDb(r) => r.attr_factory.make_rocks(),
            };
            let data_fs = match &file.storage {
                FileStorage::InMemory(m) => m.attr_factory.make_in_memory(schema.file_schema_attr_count()),
                FileStorage::RocksDb(r) => r.attr_factory.make_rocks(),
            };
            (
                Some(schema.new_file_description(data_fd)),
                Some(schema.new_file_name(data_fn)),
                Some(schema.new_file_schema(data_fs)),
            )
        };

        if let Some(ref mut x) = fd { x.set_file_ptr(Some(file.clone())); }
        if let Some(ref mut x) = fnm { x.set_file_ptr(Some(file.clone())); }
        if let Some(ref mut x) = fsc { x.set_file_ptr(Some(file.clone())); }

        Self {
            file: Some(file),
            storage_inmem,
            file_description: fd,
            file_name: fnm,
            file_schema: fsc,
        }
    }

    /// C++: IfcSpfHeader(IfcSpfLexer* lexer)
    pub fn with_lexer(lexer: Box<dyn SpfLexer>, header_schema: Arc<dyn HeaderSchema>, factory: Arc<dyn AttributeStorageFactory>) -> Self {
        let mem = Arc::new(InMemoryStorage {
            tokens: Arc::new(std::sync::Mutex::new(lexer)),
            attr_factory: factory,
            references_to_resolve: Arc::new(()),
        });

        // file_=None; создаём «дефолтные» объекты (как в C++)
        let fd = header_schema.new_file_description(mem.attr_factory.make_in_memory(header_schema.file_description_attr_count()));
        let fnm = header_schema.new_file_name(mem.attr_factory.make_in_memory(header_schema.file_name_attr_count()));
        let fsc = header_schema.new_file_schema(mem.attr_factory.make_in_memory(header_schema.file_schema_attr_count()));

        Self {
            file: None,
            storage_inmem: Some(mem),
            file_description: Some(fd),
            file_name: Some(fnm),
            file_schema: Some(fsc),
        }
    }

    fn read_semicolon(&mut self) -> Result<(), IfcException> {
        if let Some(mem) = &self.storage_inmem {
            let mut lx = mem.tokens.lock().unwrap();
            let tk = lx.next();
            if !TokenFunc::is_operator(&tk, ';') {
                return Err(IfcException("Expected ;".into()));
            }
        } else {
            // недостижимо для rocks в этом контексте
        }
        Ok(())
    }

    fn read_terminal(&mut self, term: &str, trail: Trail) -> Result<(), IfcException> {
        if let Some(mem) = &self.storage_inmem {
            let mut lx = mem.tokens.lock().unwrap();
            let tk = lx.next();
            if TokenFunc::as_string_ref(&tk) != term {
                return Err(IfcException(format!("Expected {}", term)));
            }
            drop(lx);
            if trail == Trail::TrailingSemicolon {
                self.read_semicolon()?;
            }
        } else {
            // недостижимо для rocks в этом контексте
        }
        Ok(())
    }

    /// локальная копия read_from_spf_file() — строит IfcEntityInstanceData из токенов
    fn read_from_spf_file(&mut self, s: usize) -> IfcEntityInstanceData {
        if let Some(mem) = &self.storage_inmem {
            // В C++: tokens->Next(); storage->load(-1,...); pc.construct(...)
            // Здесь просто «поглощаем» один токен и возвращаем in-memory заготовку нужного размера
            let _ = mem.tokens.lock().unwrap().next();
            return mem.attr_factory.make_in_memory(s);
        } else {
            // недостижимо для rocks при чтении SPF
            // вернем заглушку
            if let Some(file) = &self.file {
                match &file.storage {
                    FileStorage::RocksDb(r) => r.attr_factory.make_rocks(),
                    FileStorage::InMemory(m) => m.attr_factory.make_in_memory(s),
                }
            } else {
                // no file context
                IfcEntityInstanceData
            }
        }
    }

    pub fn read(&mut self) -> Result<(), IfcException> {
        self.read_terminal(ISO_10303_21, Trail::TrailingSemicolon)?;
        self.read_terminal(HEADER, Trail::TrailingSemicolon)?;

        let schema = self.header_schema();

        // file_description
        self.read_terminal(schema.file_description_name_uc(), Trail::None)?;
        self.file_description = None; // delete+new
        let data_fd = self.read_from_spf_file(schema.file_description_attr_count());
        let mut fd = schema.new_file_description(data_fd);
        fd.set_file_ptr(self.file.clone());
        self.file_description = Some(fd);
        self.read_semicolon()?;

        // file_name
        self.read_terminal(schema.file_name_name_uc(), Trail::None)?;
        self.file_name = None;
        let data_fn = self.read_from_spf_file(schema.file_name_attr_count());
        let mut fnm = schema.new_file_name(data_fn);
        fnm.set_file_ptr(self.file.clone());
        self.file_name = Some(fnm);
        self.read_semicolon()?;

        // file_schema
        self.read_terminal(schema.file_schema_name_uc(), Trail::None)?;
        self.file_schema = None;
        let data_fs = self.read_from_spf_file(schema.file_schema_attr_count());
        let mut fsc = schema.new_file_schema(data_fs);
        fsc.set_file_ptr(self.file.clone());
        self.file_schema = Some(fsc);
        self.read_semicolon()?;

        Ok(())
    }

    pub fn try_read(&mut self) -> bool {
        match self.read() {
            Ok(_) => true,
            Err(e) => {
                Logger::error(e);
                false
            }
        }
    }

    pub fn write<W: Write>(&self, mut out: W) -> std::io::Result<()> {
        writeln!(out, "{};", ISO_10303_21)?;
        writeln!(out, "{};", HEADER)?;

        {
            let mut buf = String::new();
            self.file_description().to_string_spf(&mut buf, true);
            write!(out, "{};\n", buf)?;
        }
        {
            let mut buf = String::new();
            self.file_name().to_string_spf(&mut buf, true);
            write!(out, "{};\n", buf)?;
        }
        {
            let mut buf = String::new();
            self.file_schema().to_string_spf(&mut buf, true);
            write!(out, "{};\n", buf)?;
        }

        writeln!(out, "{};", ENDSEC)?;
        writeln!(out, "{};", DATA)?;
        Ok(())
    }

    /// ===== ленивые геттеры, как в C++ (ветвление по storage) =====

    pub fn file_description(&self) -> &dyn FileDescription {
        if self.file_description.is_none() {
            let mut_this = unsafe { &mut *(self as *const _ as *mut Self) };
            let file = self.file.as_ref().expect("file required");
            let schema = &file.header_schema;
            let inst = match &file.storage {
                FileStorage::RocksDb(r) => schema.new_file_description(r.attr_factory.make_rocks()),
                FileStorage::InMemory(m) => schema.new_file_description(m.attr_factory.make_in_memory(schema.file_description_attr_count())),
            };
            mut_this.file_description = Some(inst);
            if let Some(ref mut x) = mut_this.file_description { x.set_file_ptr(self.file.clone()); }
        }
        self.file_description.as_ref().unwrap().as_ref()
    }

    pub fn file_name(&self) -> &dyn FileName {
        if self.file_name.is_none() {
            let mut_this = unsafe { &mut *(self as *const _ as *mut Self) };
            let file = self.file.as_ref().expect("file required");
            let schema = &file.header_schema;
            let inst = match &file.storage {
                FileStorage::RocksDb(r) => schema.new_file_name(r.attr_factory.make_rocks()),
                FileStorage::InMemory(m) => schema.new_file_name(m.attr_factory.make_in_memory(schema.file_name_attr_count())),
            };
            mut_this.file_name = Some(inst);
            if let Some(ref mut x) = mut_this.file_name { x.set_file_ptr(self.file.clone()); }
        }
        self.file_name.as_ref().unwrap().as_ref()
    }

    pub fn file_schema(&self) -> &dyn FileSchema {
        if self.file_schema.is_none() {
            let mut_this = unsafe { &mut *(self as *const _ as *mut Self) };
            let file = self.file.as_ref().expect("file required");
            let schema = &file.header_schema;
            let inst = match &file.storage {
                FileStorage::RocksDb(r) => schema.new_file_schema(r.attr_factory.make_rocks()),
                FileStorage::InMemory(m) => schema.new_file_schema(m.attr_factory.make_in_memory(schema.file_schema_attr_count())),
            };
            mut_this.file_schema = Some(inst);
            if let Some(ref mut x) = mut_this.file_schema { x.set_file_ptr(self.file.clone()); }
        }
        self.file_schema.as_ref().unwrap().as_ref()
    }

    pub fn file_description_mut(&mut self) -> &mut dyn FileDescription {
        if self.file_description.is_none() {
            let file = self.file.as_ref().expect("file required");
            let schema = &file.header_schema;
            let inst = match &file.storage {
                FileStorage::RocksDb(r) => schema.new_file_description(r.attr_factory.make_rocks()),
                FileStorage::InMemory(m) => schema.new_file_description(m.attr_factory.make_in_memory(schema.file_description_attr_count())),
            };
            self.file_description = Some(inst);
            if let Some(ref mut x) = self.file_description { x.set_file_ptr(self.file.clone()); }
        }
        self.file_description.as_mut().unwrap().as_mut()
    }

    pub fn file_name_mut(&mut self) -> &mut dyn FileName {
        if self.file_name.is_none() {
            let file = self.file.as_ref().expect("file required");
            let schema = &file.header_schema;
            let inst = match &file.storage {
                FileStorage::RocksDb(r) => schema.new_file_name(r.attr_factory.make_rocks()),
                FileStorage::InMemory(m) => schema.new_file_name(m.attr_factory.make_in_memory(schema.file_name_attr_count())),
            };
            self.file_name = Some(inst);
            if let Some(ref mut x) = self.file_name { x.set_file_ptr(self.file.clone()); }
        }
        self.file_name.as_mut().unwrap().as_mut()
    }

    pub fn file_schema_mut(&mut self) -> &mut dyn FileSchema {
        if self.file_schema.is_none() {
            let file = self.file.as_ref().expect("file required");
            let schema = &file.header_schema;
            let inst = match &file.storage {
                FileStorage::RocksDb(r) => schema.new_file_schema(r.attr_factory.make_rocks()),
                FileStorage::InMemory(m) => schema.new_file_schema(m.attr_factory.make_in_memory(schema.file_schema_attr_count())),
            };
            self.file_schema = Some(inst);
            if let Some(ref mut x) = self.file_schema { x.set_file_ptr(self.file.clone()); }
        }
        self.file_schema.as_mut().unwrap().as_mut()
    }

    #[inline]
    fn header_schema(&self) -> Arc<dyn HeaderSchema> {
        match &self.file {
            Some(f) => f.header_schema.clone(),
            None => panic!("Header schema required"),
        }
    }
}
