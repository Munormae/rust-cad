// SPDX-License-Identifier: MIT
// ПРЯМОЙ ПОРТ ДАННОГО КУСКА C++ КОДА В RUST БЕЗ УПРОЩЕНИЙ ПО ЛОГИКЕ
// Файл: IfcSpfStream.rs (включает IfcSpfStream, IfcSpfLexer, Token/TokenFunc,
// часть impl IfcParse::impl::in_memory_file_storage, rocks_db_file_storage, StringBuilderVisitor)
// Прим.: Импорт путей модулей подстроить под вашу иерархию (crate::...).

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::needless_return)]

use std::ffi::c_void;
use std::fs::File;
use std::io::{Read};
use std::path::Path;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::Write as _;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{ptr, mem};

// -------------------------------- Imports из вашего проекта --------------------------------
use crate::IfcParse; // модуль с пространством имён IfcParse
use crate::IfcParse::{IfcException, IfcInvalidTokenException};
use crate::IfcBaseClass; // IfcUtil::IfcBaseClass аналог
use crate::IfcCharacterDecoder::IfcCharacterDecoder;
use crate::IfcFile::IfcFile;
use crate::IfcLogger as Logger;
use crate::IfcSchema; // Schema + Header_section_schema
use crate::IfcSIPrefix; // если нужно

// -------------------------------- Флаги --------------------------------
// Аналог #define PERMISSIVE_FLOAT
const PERMISSIVE_FLOAT: bool = true;

// Аналог USE_MMAP
#[cfg(feature = "use_mmap")]
use memmap2::Mmap;

// ---------------------------- Локаль для парсинга real ----------------------------
#[cfg(windows)]
mod locale_win {
    use super::*;
    use std::sync::OnceLock;
    static INIT: OnceLock<()> = OnceLock::new();
    pub fn init_locale() { let _ = INIT.get_or_init(|| {}); }
}

#[cfg(all(any(target_env = "gnu", target_os = "macos", target_os = "linux"), not(windows)))]
mod locale_posix {
    use super::*;
    use std::sync::OnceLock;
    static INIT: OnceLock<()> = OnceLock::new();
    pub fn init_locale() { let _ = INIT.get_or_init(|| {}); }
}

#[cfg(any(windows, all(any(target_env = "gnu", target_os = "macos", target_os = "linux"), not(windows))))]
#[inline]
fn init_locale() { #[cfg(windows)] locale_win::init_locale(); #[cfg(not(windows))] locale_posix::init_locale(); }

// ----------------------------------------------------------------------------------
// IfcSpfStream
// ----------------------------------------------------------------------------------
pub struct IfcSpfStream {
    #[cfg(feature = "use_mmap")] mfs: Option<Mmap>,
    stream_: Option<()>, // заглушка для FILE*
    buffer_: *const u8,
    owned: Option<Vec<u8>>, // для владения буфером
    pub valid: bool,
    pub eof: bool,
    pub size: u32,
    ptr_: u32,
    len_: u32,
}

impl IfcSpfStream {
    #[cfg(feature = "use_mmap")]
    pub fn new_with_mmap(path: &str, mmap_flag: bool) -> Self {
        let mut s = Self { mfs: None, stream_: None, buffer_: ptr::null(), owned: None, valid: false, eof: false, size: 0, ptr_: 0, len_: 0 };
        #[cfg(windows)] {
            // В Rust используем обычное открытие файла
            if mmap_flag {
                if let Ok(f) = File::open(Path::new(path)) {
                    if let Ok(m) = unsafe { Mmap::map(&f) } { s.len_ = m.len() as u32; s.buffer_ = m.as_ptr(); s.mfs = Some(m); s.valid = true; s.ptr_ = 0; }
                }
            } else {
                match std::fs::read(path) { Ok(buf)=>{ s.size = buf.len() as u32; s.len_ = s.size; s.eof = s.len_==0; s.buffer_=buf.as_ptr(); s.owned=Some(buf); s.valid=true; }, Err(_)=>{} }
            }
        }
        #[cfg(not(windows))] {
            if mmap_flag {
                if let Ok(f) = File::open(Path::new(path)) {
                    if let Ok(m) = unsafe { Mmap::map(&f) } { s.len_ = m.len() as u32; s.buffer_ = m.as_ptr(); s.mfs = Some(m); s.valid = true; s.ptr_ = 0; }
                }
            } else {
                match std::fs::read(path) { Ok(buf)=>{ s.size = buf.len() as u32; s.len_ = s.size; s.eof = s.len_==0; s.buffer_=buf.as_ptr(); s.owned=Some(buf); s.valid=true; }, Err(_)=>{} }
            }
        }
        s
    }

    #[cfg(not(feature = "use_mmap"))]
    pub fn new(path: &str) -> Self {
        let mut s = Self { stream_: None, buffer_: ptr::null(), owned: None, valid: false, eof: false, size: 0, ptr_: 0, len_: 0 };
        match std::fs::read(path) {
            Ok(buf) => { s.size = buf.len() as u32; s.len_ = s.size; s.buffer_ = buf.as_ptr(); s.owned = Some(buf); s.valid = true; s.eof = s.len_==0; },
            Err(_) => {}
        }
        s
    }

    pub fn new_from_istream(mut stream: impl Read, length: i32) -> Self {
        let mut s = Self { #[cfg(feature = "use_mmap")] mfs: None, stream_: None, buffer_: ptr::null(), owned: None, valid: false, eof: false, size: length as u32, ptr_: 0, len_: length as u32 };
        let mut buf = vec![0u8; length as usize];
        let _ = stream.read_exact(&mut buf);
        s.valid = true; // аналог stream.gcount()==size (упрощение по вводу)
        s.buffer_ = buf.as_ptr();
        s.owned = Some(buf);
        s.eof = s.len_==0;
        s
    }

    pub fn new_from_ptr(data: *mut c_void, length: i32) -> Self {
        let mut s = Self { #[cfg(feature = "use_mmap")] mfs: None, stream_: None, buffer_: data as *const u8, owned: None, valid: true, eof: false, size: length as u32, ptr_: 0, len_: length as u32 };
        s
    }
}

impl Drop for IfcSpfStream { fn drop(&mut self) { self.Close(); } }

impl IfcSpfStream {
    pub fn Close(&mut self) {
        #[cfg(feature = "use_mmap")] { if self.mfs.is_some() { self.mfs = None; return; } }
        self.owned = None;
        self.stream_ = None;
    }

    pub fn Seek(&mut self, offset: u32) {
        self.ptr_ = offset;
        if self.ptr_ >= self.len_ { panic!("IfcException: Reading outside of file limits"); }
        self.eof = false;
    }
    pub fn Peek(&self) -> char { self.Read(self.ptr_) }
    pub fn Read(&self, offset: u32) -> char { unsafe { *self.buffer_.add(offset as usize) as char } }
    pub fn Tell(&self) -> u32 { self.ptr_ }

    pub fn Inc(&mut self) {
        self.ptr_ += 1;
        if self.ptr_ == self.len_ { self.eof = true; return; }
        let current = self.Peek();
        if current == '\n' || current == '\r' { self.Inc(); }
    }

    pub fn is_eof_at(&self, local_ptr: u32) -> bool { local_ptr >= self.len_ }
    pub fn increment_at(&self, local_ptr: &mut u32) {
        *local_ptr += 1;
        if *local_ptr == self.len_ { return; }
        let current = self.peek_at(*local_ptr);
        if current == '\n' || current == '\r' { self.increment_at(local_ptr); }
    }
    pub fn peek_at(&self, local_ptr: u32) -> char { self.Read(local_ptr) }
}

// ----------------------------------------------------------------------------------
// Lexer/Token
// ----------------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TokenType { Token_NONE, Token_OPERATOR, Token_IDENTIFIER, Token_STRING, Token_ENUMERATION, Token_BINARY, Token_KEYWORD, Token_INT, Token_BOOL, Token_FLOAT }

pub struct IfcSpfLexer {
    pub stream: *mut IfcSpfStream,
    decoder_: *mut IfcCharacterDecoder,
    temp: RefCell<String>,
}

impl IfcSpfLexer {
    pub fn new(stream_: *mut IfcSpfStream) -> Self { Self { stream: stream_, decoder_: Box::into_raw(Box::new(IfcCharacterDecoder::new(stream_))), temp: RefCell::new(String::new()) } }
}
impl Drop for IfcSpfLexer { fn drop(&mut self) { unsafe { let _ = Box::from_raw(self.decoder_); } } }

impl IfcSpfLexer {
    pub fn skipWhitespace(&self) -> u32 {
        let mut index = 0u32;
        unsafe {
            while !(*self.stream).eof {
                let character = (*self.stream).Peek();
                if character == ' ' || character == '\r' || character == '\n' || character == '\t' { (*self.stream).Inc(); index += 1; } else { break; }
            }
        }
        index
    }

    pub fn skipComment(&self) -> u32 {
        unsafe {
            let mut character = (*self.stream).Peek();
            if character != '/' { return 0; }
            (*self.stream).Inc();
            character = (*self.stream).Peek();
            if character != '*' { let t = (*self.stream).Tell(); (*self.stream).Seek(t-1); return 0; }
            let mut index = 2u32; let mut intermediate = '\0';
            while !(*self.stream).eof {
                character = (*self.stream).Peek();
                (*self.stream).Inc();
                index += 1;
                if character == '/' && intermediate == '*' { break; }
                intermediate = character;
            }
            return index;
        }
    }

    pub fn Next<'a>(&'a self) -> Token<'a> {
        unsafe {
            if (*self.stream).eof { return Token::null(self); }
            while self.skipWhitespace()!=0 || self.skipComment()!=0 {}
            if (*self.stream).eof { return Token::null(self); }
            let pos = (*self.stream).Tell();
            let mut character = (*self.stream).Peek();
            if matches!(character, '('|')'|'='|','|';'|'$'|'*') { (*self.stream).Inc(); return OperatorTokenPtr(self, pos, pos+1); }
            let mut len = 0i32;
            while !(*self.stream).eof {
                character = (*self.stream).Peek();
                if (len!=0) && matches!(character, '('|')'|'='|','|';'|'/') { break; }
                (*self.stream).Inc(); len += 1;
                if character=='\'' { (*self.decoder_).skip(); }
            }
            if len!=0 { GeneralTokenPtr(self, pos, (*self.stream).Tell()) } else { Token::null(self) }
        }
    }

    pub fn TokenString(&self, offset: u32, buffer: &mut String) {
        buffer.clear();
        unsafe {
            let mut off = offset;
            while !(*self.stream).is_eof_at(off) {
                let character = (*self.stream).peek_at(off);
                if !buffer.is_empty() && matches!(character, '('|')'|'='|','|';'|'/') { break; }
                (*self.stream).increment_at(&mut off);
                if matches!(character, ' '|'\r'|'\n'|'\t') { continue; }
                if character=='\'' { *buffer = (*self.decoder_).get(off as usize); break; }
                buffer.push(character);
            }
        }
    }

    pub fn GetTempString(&self) -> std::cell::RefMut<'_, String> { self.temp.borrow_mut() }
}

pub struct Token<'a> {
    pub lexer: &'a IfcSpfLexer,
    pub startPos: u32,
    pub endPos: u32,
    pub type_: TokenType,
    pub value_int: i32,
    pub value_double: f64,
    pub value_char: char,
}

impl<'a> Token<'a> {
    fn null(lexer: &'a IfcSpfLexer) -> Self { Self{ lexer, startPos:0, endPos:0, type_:TokenType::Token_NONE, value_int:0, value_double:0.0, value_char:'\0' } }
}

// Аналоги фабрик OperatorTokenPtr / GeneralTokenPtr
pub fn OperatorTokenPtr<'a>(lexer:&'a IfcSpfLexer, start:u32, end:u32) -> Token<'a> {
    unsafe { let first = (*lexer.stream).Read(start); Token{ lexer, startPos:start, endPos:end, type_:TokenType::Token_OPERATOR, value_int:0, value_double:0.0, value_char:first } }
}

pub fn RemoveTokenSeparators(stream: *mut IfcSpfStream, start:u32, end:u32, oDestination: &mut String) {
    oDestination.clear();
    unsafe {
        for i in start..end {
            let character = (*stream).Read(i);
            if matches!(character, ' '|'\r'|'\n'|'\t') { continue; }
            oDestination.push(character);
        }
    }
}

pub fn ParseInt(pStart: &str, val:&mut i32) -> bool { if let Ok(v)=pStart.parse::<i32>() { *val=v; true } else { false } }

pub fn ParseFloat(pStart: &str, val:&mut f64) -> bool { init_locale(); if let Ok(v)=pStart.parse::<f64>() { *val=v; true } else { false } }

pub fn ParseBool(pStart: &str, val:&mut i32) -> bool {
    if pStart.len()!=3 { return false; }
    let b = pStart.as_bytes(); if b[0]!=b'.' || b[2]!=b'.' { return false; }
    match b[1] { b'T'=>{*val=1; true}, b'F'=>{*val=0; true}, b'U'=>{*val=2; true}, _=>false }
}

pub fn GeneralTokenPtr<'a>(lexer:&'a IfcSpfLexer, start:u32, end:u32) -> Token<'a> {
    let mut token = Token{ lexer, startPos:start, endPos:end, type_:TokenType::Token_NONE, value_int:0, value_double:0.0, value_char:'\0' };
    let mut tokenStr = lexer.GetTempString();
    unsafe { RemoveTokenSeparators(lexer.stream, start, end, &mut tokenStr); let first = (*lexer.stream).Read(start);
        if first=='#' {
            token.type_ = TokenType::Token_IDENTIFIER;
            let mut v=0; if !ParseInt(&tokenStr[1..], &mut v) { Logger::Message(Logger::LOG_ERROR, format!("Token '{}' at offset {} is not valid", *tokenStr, token.startPos)); token.type_=TokenType::Token_OPERATOR; token.value_char='$'; } else { token.value_int=v; }
        } else if first=='\'' { token.type_ = TokenType::Token_STRING; }
        else if first=='.' { token.type_=TokenType::Token_ENUMERATION; let mut v=0; if ParseBool(&tokenStr, &mut v) { token.type_=TokenType::Token_BOOL; token.value_int=v; } }
        else if first=='"' { token.type_=TokenType::Token_BINARY; }
        else if ParseInt(&tokenStr, &mut token.value_int) { token.type_=TokenType::Token_INT; }
        else if ParseFloat(&tokenStr, &mut token.value_double) { token.type_=TokenType::Token_FLOAT; }
        else { token.type_=TokenType::Token_KEYWORD; }
    }
    drop(tokenStr);
    token
}

pub struct TokenFunc;
impl TokenFunc {
    pub fn isOperator(token:&Token) -> bool { matches!(token.type_, TokenType::Token_OPERATOR) }
    pub fn isOperator_ch(token:&Token, ch:char) -> bool { token.type_==TokenType::Token_OPERATOR && token.value_char==ch }
    pub fn isIdentifier(token:&Token) -> bool { token.type_==TokenType::Token_IDENTIFIER }
    pub fn isString(token:&Token) -> bool { token.type_==TokenType::Token_STRING }
    pub fn isEnumeration(token:&Token) -> bool { token.type_==TokenType::Token_ENUMERATION || token.type_==TokenType::Token_BOOL }
    pub fn isBinary(token:&Token) -> bool { token.type_==TokenType::Token_BINARY }
    pub fn isKeyword(token:&Token) -> bool { token.type_==TokenType::Token_KEYWORD }
    pub fn isInt(token:&Token) -> bool { token.type_==TokenType::Token_INT }
    pub fn isBool(token:&Token) -> bool { token.type_==TokenType::Token_BOOL && token.value_int != 2 }
    pub fn isLogical(token:&Token) -> bool { token.type_==TokenType::Token_BOOL }
    pub fn isFloat(token:&Token) -> bool { if PERMISSIVE_FLOAT { token.type_==TokenType::Token_FLOAT || token.type_==TokenType::Token_INT } else { token.type_==TokenType::Token_FLOAT } }

    pub fn asInt(token:&Token) -> i32 { if token.type_!=TokenType::Token_INT { panic!("IfcInvalidTokenException({}, '{}', integer)", token.startPos, Self::toString(token)); } token.value_int }
    pub fn asIdentifier(token:&Token) -> i32 { if token.type_!=TokenType::Token_IDENTIFIER { panic!("IfcInvalidTokenException({}, '{}', instance name)", token.startPos, Self::toString(token)); } token.value_int }
    pub fn asBool(token:&Token) -> bool { if token.type_!=TokenType::Token_BOOL { panic!("IfcInvalidTokenException({}, '{}', boolean)", token.startPos, Self::toString(token)); } token.value_int==1 }
    pub fn asLogical(token:&Token) -> Option<bool> { if token.type_!=TokenType::Token_BOOL { panic!("IfcInvalidTokenException({}, '{}', boolean)", token.startPos, Self::toString(token)); } match token.value_int {0=>Some(false),1=>Some(true),_=>None} }
    pub fn asFloat(token:&Token) -> f64 { if PERMISSIVE_FLOAT && token.type_==TokenType::Token_INT { return token.value_int as f64; } if token.type_==TokenType::Token_FLOAT { return token.value_double; } panic!("IfcInvalidTokenException({}, '{}', real)", token.startPos, Self::toString(token)); }

    pub fn asStringRef<'a>(token:&'a Token<'a>) -> String {
        if matches!(token.type_, TokenType::Token_NONE) { panic!("IfcException: Null token encountered, premature end of file?"); }
        let mut str_ = String::new(); token.lexer.TokenString(token.startPos, &mut str_);
        if (Self::isString(token) || Self::isEnumeration(token) || Self::isBinary(token)) && !str_.is_empty() { str_.remove(0); str_.pop(); }
        str_
    }
    pub fn asString(token:&Token) -> String { if Self::isString(token) || Self::isEnumeration(token) || Self::isBinary(token) { Self::asStringRef(token) } else { panic!("IfcInvalidTokenException({}, '{}', string)", token.startPos, Self::toString(token)); } }

    pub fn asBinary(_token:&Token) -> Vec<bool> { // упрощённая dynamic_bitset — логика 1:1
        let s = Self::asStringRef(_token);
        if s.is_empty() { panic!("IfcException: Token is not a valid binary sequence"); }
        let mut it = s.chars();
        let n = it.next().unwrap() as i32 - '0' as i32;
        if n<0 || n>3 || (s.len()==1 && n!=0) { panic!("IfcException: Token is not a valid binary sequence"); }
        let mut bits: Vec<bool> = Vec::new();
        // размер ((len-1)*4 - n)
        let total = ((s.len()-1)*4) as i32 - n; if total>0 { bits.resize(total as usize, false); }
        let mut i = total as i32; // обратная индексация
        for c in it { let mut value = if c < 'A' { (c as u8 - b'0') as i32 } else { (c as u8 - b'A' + 10) as i32 };
            for j in 0..4 { if i<=0 { break; } i-=1; let bit = (value & (1 << (3-j)))!=0; bits[i as usize] = bit; }
        }
        bits
    }

    pub fn toString(token:&Token) -> String { let mut r = String::new(); token.lexer.TokenString(token.startPos, &mut r); r }
}

// ----------------------------------------------------------------------------------
// in_memory_file_storage (фрагменты)
// ----------------------------------------------------------------------------------
pub mod storage_impl {
    use super::*;
    use crate::IfcParse::{self as ip};

    pub struct in_memory_file_storage<'a> {
        pub tokens: &'a IfcSpfLexer,
        pub file: &'a IfcFile,
        pub schema: Option<&'a IfcSchema::Schema>,
        pub references_to_resolve: &'a mut Vec<()>, // замените на ваш тип
        pub byref_excl_: std::collections::BTreeMap<(i32, u16, i32), Vec<u32>>, // key: (inst_id, entity_idx, attr_idx)
    }

    impl<'a> in_memory_file_storage<'a> {
        pub fn load(&mut self, entity_instance_name: u32, entity: Option<&ip::entity>, mut context: ip::parse_context, attribute_index: i32) {
            let mut next = self.tokens.Next();
            let mut attribute_index_within_data: usize = 0;
            let mut return_value: usize = 0;
            while (next.startPos != 0) || (!matches!(next.lexer as *const _, std::ptr::null())) {
                if TokenFunc::isOperator_ch(&next, ',') {
                    if attribute_index == -1 { attribute_index_within_data += 1; }
                } else if TokenFunc::isOperator_ch(&next, ')') { break; }
                else if TokenFunc::isOperator_ch(&next, '(') {
                    return_value += 1;
                    let idx = if attribute_index == -1 { attribute_index_within_data as i32 } else { attribute_index };
                    self.load(entity_instance_name, entity, context.push(), idx);
                } else {
                    return_value += 1;
                    if TokenFunc::isIdentifier(&next) { if let Some(e)=entity { self.register_inverse(entity_instance_name, e, next.value_int, if attribute_index == -1 { attribute_index_within_data as i32 } else { attribute_index }); } }
                    if TokenFunc::isKeyword(&next) {
                        // try
                        let res = (||{
                            let decl = (if let Some(s)=self.schema { s } else { self.file.schema() }).declaration_by_name(&TokenFunc::asStringRef(&next));
                            let mut ps = ip::parse_context::default();
                            let _ = self.tokens.Next();
                            let idx = if attribute_index == -1 { attribute_index_within_data as i32 } else { attribute_index };
                            self.load(entity_instance_name, entity, ps, idx);
                            let simple_type_instance = (if let Some(s)=self.schema { s } else { self.file.schema() }).instantiate(decl, ps.construct(entity_instance_name, self.references_to_resolve, decl, None, idx));
                            context.push_instance(simple_type_instance);
                            context.set_file_for_last(self.file);
                            Ok::<(),()> (())
                        })();
                        if res.is_err() {
                            Logger::Message(Logger::LOG_ERROR, format!("error at offset {}", next.startPos));
                            return_value -= 1;
                        }
                    } else { context.push_token(next); }
                }
                next = self.tokens.Next();
            }
        }

        pub fn read(&mut self, i:u32) -> ip::IfcEntityInstanceData {
            let datatype = self.tokens.Next();
            if !TokenFunc::isKeyword(&datatype) { panic!("IfcException: Unexpected token while parsing entity"); }
            let ty = self.file.schema().declaration_by_name(&TokenFunc::asStringRef(&datatype));
            let mut pc = ip::parse_context::default();
            let _ = self.tokens.Next();
            self.load(i, ty.as_entity(), pc, -1);
            ip::IfcEntityInstanceData(pc.construct(i, self.references_to_resolve, ty, None, -1))
        }

        pub fn try_read_semicolon(&self) {
            unsafe { let old_offset = (*self.tokens.stream).Tell(); let semicolon = self.tokens.Next(); if !TokenFunc::isOperator_ch(&semicolon, ';') { (*self.tokens.stream).Seek(old_offset); } }
        }

        pub fn register_inverse(&mut self, id_from:u32, from_entity:&ip::entity, inst_id:i32, attribute_index:i32) {
            self.byref_excl_.entry((inst_id, from_entity.index_in_schema(), attribute_index)).or_default().push(id_from);
        }

        pub fn unregister_inverse(&mut self, id_from:u32, from_entity:&ip::entity, inst:&mut IfcBaseClass, attribute_index:i32) {
            if let Some(ids) = self.byref_excl_.get_mut(&(inst.id(), from_entity.index_in_schema(), attribute_index)) {
                if let Some(p) = ids.iter().position(|&x| x==id_from) { ids.remove(p); }
            }
        }
    }
}

// ----------------------------------------------------------------------------------
// rocks_db_file_storage (фрагменты под фичу IFOPSH_WITH_ROCKSDB)
// ----------------------------------------------------------------------------------
#[cfg(feature = "ifopsh_with_rocksdb")]
pub mod rocksdb_impl {
    use super::*;
    use rocksdb::{DB, Options, WriteOptions};
    use std::mem::size_of;

    fn to_string_fixed_width<T: std::fmt::Display>(t:&T, _w:usize) -> String { format!("{}", t) }

    pub struct rocks_db_file_storage<'a> { pub db: &'a DB, pub wopts: WriteOptions }

    impl<'a> rocks_db_file_storage<'a> {
        pub fn register_inverse(&self, id_from:u32, from_entity:&IfcParse::entity, inst_id:i32, attribute_index:i32) {
            let mut s = vec![0u8; std::mem::size_of::<u32>()];
            s.copy_from_slice(&id_from.to_le_bytes());
            let key = format!("v|{}|{}|{}", to_string_fixed_width(&inst_id,10), to_string_fixed_width(&from_entity.index_in_schema(),4), to_string_fixed_width(&attribute_index,2));
            let _ = self.db.merge_opt(key, s, &self.wopts);
        }
        pub fn unregister_inverse(&self, id_from:u32, from_entity:&IfcParse::entity, inst:&IfcBaseClass, attribute_index:i32) {
            let key = format!("v|{}|{}|{}", to_string_fixed_width(&inst.id(),10), to_string_fixed_width(&from_entity.index_in_schema(),4), to_string_fixed_width(&attribute_index,2));
            if let Ok(mut s) = self.db.get(&key) { if let Some(mut s)=s { let mut vals: Vec<u32> = s.chunks_exact(4).map(|b| u32::from_le_bytes([b[0],b[1],b[2],b[3]])).collect(); if let Some(p)=vals.iter().position(|&x| x==id_from) { vals.remove(p); } s.clear(); for v in vals { s.extend_from_slice(&v.to_le_bytes()); } let _ = self.db.put_opt(key, s, &self.wopts); } }
        }
        pub fn add_type_ref(&self, new_entity:&IfcBaseClass) {
            let mut s = vec![0u8; std::mem::size_of::<usize>()];
            if new_entity.declaration().as_entity().is_some() {
                let v = new_entity.id() as usize; s[..].copy_from_slice(&v.to_le_bytes());
                let key = format!("t|{}", new_entity.declaration().index_in_schema()); let _ = self.db.merge_opt(key, s.clone(), &self.wopts);
            }
            let v = new_entity.declaration().index_in_schema() as usize; s[..].copy_from_slice(&v.to_le_bytes());
            let key = format!("{}|{}|_", if new_entity.declaration().as_entity().is_some() {"i"} else {"t"}, if new_entity.id()!=0 { new_entity.id() } else { new_entity.identity() });
            let _ = self.db.put_opt(key, s, &self.wopts);
        }
        pub fn remove_type_ref(&self, new_entity:&IfcBaseClass) {
            if new_entity.declaration().as_entity().is_some() {
                let key = format!("t|{}", new_entity.declaration().index_in_schema());
                if let Ok(Some(mut s)) = self.db.get(&key) {
                    let mut vals: Vec<usize> = s.chunks_exact(std::mem::size_of::<usize>()).map(|b| {
                        let mut arr=[0u8; std::mem::size_of::<usize>()]; arr.copy_from_slice(b); usize::from_le_bytes(arr)
                    }).collect();
                    if let Some(p)=vals.iter().position(|&x| x==new_entity.id() as usize) { vals.remove(p); }
                    s.clear(); for v in vals { s.extend_from_slice(&v.to_le_bytes()); }
                    let _ = self.db.put_opt(key, s, &self.wopts);
                }
            }
            let key = format!("{}|{}|_", if new_entity.declaration().as_entity().is_some() {"i"} else {"t"}, if new_entity.id()!=0 { new_entity.id() } else { new_entity.identity() });
            let _ = self.db.delete_opt(key, &self.wopts);
        }
    }
}

// ----------------------------------------------------------------------------------
// StringBuilderVisitor (порт на Rust)
// ----------------------------------------------------------------------------------
pub mod string_builder {
    use super::*;
    use std::fmt::Write;

    pub struct StringBuilderVisitor<'a> { data_: &'a mut dyn std::fmt::Write, upper_: bool }
    impl<'a> StringBuilderVisitor<'a> {
        pub fn new(stream: &'a mut dyn std::fmt::Write, upper: bool) -> Self { Self{ data_: stream, upper_: upper } }
        fn serialize<T: std::fmt::Display>(&mut self, i:&[T]) { let _=write!(self.data_, "("); for (k,v) in i.iter().enumerate() { if k>0 { let _=write!(self.data_, ","); } let _=write!(self.data_, "{}", v); } let _=write!(self.data_, ")"); }
        fn format_double(d: f64) -> String { let s = format!("{:.prec$}", d, prec = std::f64::DIGITS as usize); let mut out = String::new(); let (mantissa, exp) = if let Some(e)=s.find('e').or_else(|| s.find('E')) { (&s[..e], Some(&s[e+1..])) } else { (&s[..], None) }; out.push_str(mantissa); if !mantissa.contains('.') { out.push('.'); } if let Some(e)=exp { out.push('E'); out.push_str(e); } out }
        fn format_binary(b: &[bool]) -> String { let mut out=String::new(); out.push('"'); let c = b.len() as u32; let n = (4 - (c % 4)) & 3; let _=write!(out, "{}", n); let mut i=0u32; while i < c + n { let mut accum=0u32; for j in 0..4 { let bit = if i < n {0} else { if b[(c - i + n - 1) as usize] {1} else {0} }; accum |= bit << (3-j); i+=1; } let _=write!(out, "{:X}", accum); } out.push('"'); out }
        pub fn blank(&mut self) { let _=write!(self.data_, "$"); }
        pub fn derived(&mut self) { let _=write!(self.data_, "*"); }
        pub fn int_(&mut self, v:i32) { let _=write!(self.data_, "{}", v); }
        pub fn bool_(&mut self, v:bool) { let _=write!(self.data_, "{}", if v { ".T." } else { ".F." }); }
        pub fn tribool(&mut self, v:Option<bool>) { let _=write!(self.data_, "{}", match v { Some(true)=>".T.", Some(false)=>".F.", None=>".U."}); }
        pub fn double_(&mut self, v:f64) { let _=write!(self.data_, "{}", Self::format_double(v)); }
        pub fn bin_(&mut self, v:&[bool]) { let _=write!(self.data_, "{}", Self::format_binary(v)); }
        pub fn str_(&mut self, s:&str) { if self.upper_ { let enc = crate::IfcCharacterDecoder::IfcCharacterEncoder::encode_owned(s); let _=write!(self.data_, "{}", enc); } else { let _=write!(self.data_, "'{}'", s); } }
        pub fn vec_int(&mut self, v:&[i32]) { self.serialize(v); }
        pub fn vec_double(&mut self, v:&[f64]) { let mut tmp: Vec<String>=v.iter().map(|&x| Self::format_double(x)).collect(); self.serialize(&tmp); }
        pub fn vec_str(&mut self, v:&[String]) { let mut tmp: Vec<String>=v.iter().map(|s| crate::IfcCharacterDecoder::IfcCharacterEncoder::encode_owned(s)).collect(); self.serialize(&tmp); }
        pub fn enum_ref(&mut self, val:&str) { let _=write!(self.data_, ".{}.", val); }
        pub fn ifc_base(&mut self, i:&IfcBaseClass) { if i.declaration().as_entity().is_none() || std::ptr::eq(i.declaration().schema(), &IfcSchema::Header_section_schema::get_schema()) { i.to_string(self.data_, self.upper_); } else { let _=write!(self.data_, "#{}", i.id()); } }
        pub fn agg_of_instance<'b, T, F: Fn(&mut Self, &T)>(&mut self, v:&[T], f:F) { let _=write!(self.data_, "("); for (k,el) in v.iter().enumerate() { if k>0 { let _=write!(self.data_, ","); } f(self, el); } let _=write!(self.data_, ")"); }
        pub fn empty_agg(&mut self) { let _=write!(self.data_, "()"); }
    }
}
// SPDX-License-Identifier: MIT
// ПРЯМОЙ ПОРТ ПОСЛЕДНЕЙ ЧАСТИ C++ В RUST (1:1 по логике; имена сохранены)
// Прим.: замените пути модулей и типы под вашу фактическую структуру проекта.
// Данный файл ожидает существования тех же сущностей, что и в C++ версии:
// IfcEntityInstanceData, IfcUtil::{IfcBaseClass, IfcBaseEntity, ArgumentType, ...},
// IfcParse::{declaration, schema_definition, parameter_type, entity, ...},
// AttributeValue, Blank, Derived, EnumerationReference,
// aggregate_of_instance, aggregate_of_aggregate_of_instance, IfcFile, Logger, IfcException,
// Header_section_schema, file_open_status и т.д.


use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{ptr, mem};

///////////////////////////////////////////////////////////////////////////////////////////////////
// IfcEntityInstanceData::toString
///////////////////////////////////////////////////////////////////////////////////////////////////
impl IfcEntityInstanceData {
    // void IfcEntityInstanceData::toString(void* storage, const declaration* decl, std::size_t identity, std::ostream& ss, bool upper) const
    pub fn toString(
        &self,
        storage: *mut core::ffi::c_void,
        decl: *const IfcParse::declaration,
        identity: usize,
        ss: &mut dyn std::fmt::Write,
        upper: bool,
    ) {
        let _ = ss.write_str("(");

        let mut vis = StringBuilderVisitor::new(ss, upper);

        // size = (decl && decl->as_entity() ? decl->as_entity()->attribute_count() : 1);
        let mut size: usize = unsafe {
            if !decl.is_null() && !(*decl).as_entity().is_null() {
                (*(*decl).as_entity()).attribute_count()
            } else {
                1
            }
        };

        if let Some(sto) = &self.storage_ {
            size = size.min(sto.size());
        }

        for i in 0..size {
            if i != 0 {
                let _ = ss.write_str(",");
            }
            if self.has_attribute_value::<Blank>(storage, decl, identity, i) {
                unsafe {
                    if !decl.is_null()
                        && !(*decl).as_entity().is_null()
                        && (*(*decl).as_entity()).derived()[i]
                    {
                        let _ = ss.write_str("*");
                    } else {
                        let _ = ss.write_str("$");
                    }
                }
            } else {
                self.apply_visitor(storage, decl, identity, &mut vis, i);
            }
        }
        let _ = ss.write_str(")");
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// IfcUtil::IfcBaseEntity::set_id
///////////////////////////////////////////////////////////////////////////////////////////////////
impl IfcUtil::IfcBaseEntity {
    // unsigned IfcBaseEntity::set_id(const boost::optional<unsigned>& i)
    pub fn set_id(&mut self, i: Option<u32>) -> u32 {
        if let Some(v) = i {
            self.id_ = v;
            v
        } else {
            // return id_ = file_->FreshId();
            let v = unsafe { (*self.file_).FreshId() };
            self.id_ = v;
            v
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// get_argument_type (вспомогательная)
///////////////////////////////////////////////////////////////////////////////////////////////////
mod helpers_argtype {
    use super::*;

    // IfcUtil::ArgumentType get_argument_type(const declaration* decl, size_t i)
    pub fn get_argument_type(decl: *const IfcParse::declaration, i: usize) -> IfcUtil::ArgumentType {
        unsafe {
            let mut pt: *const IfcParse::parameter_type = ptr::null();

            if !decl.is_null() && !(*decl).as_entity().is_null() {
                pt = (*(*decl).as_entity())
                    .attribute_by_index(i)
                    .type_of_attribute();
                if (*(*decl).as_entity()).derived()[i] {
                    return IfcUtil::ArgumentType::Argument_DERIVED;
                }
            } else if !decl.is_null()
                && !(*decl).as_type_declaration().is_null()
                && i == 0
            {
                pt = (*(*decl).as_type_declaration()).declared_type();
            } else if !decl.is_null()
                && !(*decl).as_enumeration_type().is_null()
                && i == 0
            {
                return IfcUtil::ArgumentType::Argument_ENUMERATION;
            }

            if pt.is_null() {
                return IfcUtil::ArgumentType::Argument_UNKNOWN;
            }
            IfcUtil::from_parameter_type(pt)
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// visitors для инверсий
///////////////////////////////////////////////////////////////////////////////////////////////////
struct unregister_inverse_visitor<'a> {
    file_: &'a mut IfcFile,
    data_: &'a IfcUtil::IfcBaseClass,
}
impl<'a> unregister_inverse_visitor<'a> {
    fn new(file_: &'a mut IfcFile, data_: &'a IfcUtil::IfcBaseClass) -> Self {
        Self { file_: file_, data_ }
    }
    fn call(&mut self, inst: *mut IfcUtil::IfcBaseClass, index: i32) {
        unsafe {
            self.file_
                .unregister_inverse(self.data_.id(), self.data_.declaration().as_entity(), &mut *inst, index);
        }
    }
}

struct register_inverse_visitor<'a> {
    file_: &'a mut IfcFile,
    data_: &'a IfcUtil::IfcBaseClass,
}
impl<'a> register_inverse_visitor<'a> {
    fn new(file_: &'a mut IfcFile, data_: &'a IfcUtil::IfcBaseClass) -> Self {
        Self { file_: file_, data_ }
    }
    fn call(&mut self, inst: *mut IfcUtil::IfcBaseClass, index: i32) {
        unsafe {
            self.file_.register_inverse(
                self.data_.id(),
                self.data_.declaration().as_entity(),
                (*inst).id(),
                index,
            );
        }
    }
}

struct add_to_instance_list_visitor<'a> {
    list_: &'a mut aggregate_of_instance,
}
impl<'a> add_to_instance_list_visitor<'a> {
    fn new(list_: &'a mut aggregate_of_instance) -> Self {
        Self { list_ }
    }
    fn call(&mut self, inst: *mut IfcUtil::IfcBaseClass) {
        self.list_.push(inst);
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// apply_individual_instance_visitor
///////////////////////////////////////////////////////////////////////////////////////////////////
struct apply_individual_instance_visitor<'a> {
    attribute_: Option<AttributeValue>,
    attribute_index_: i32,
    inst_: Option<&'a IfcUtil::IfcBaseClass>,
}
impl<'a> apply_individual_instance_visitor<'a> {
    fn new_from_attr(attribute: AttributeValue, idx: i32) -> Self {
        Self {
            attribute_: Some(attribute),
            attribute_index_: idx,
            inst_: None,
        }
    }
    fn new_from_inst(data: &'a IfcUtil::IfcBaseClass) -> Self {
        Self {
            attribute_: None,
            attribute_index_: 0,
            inst_: Some(data),
        }
    }

    fn apply_attribute_<F: FnMut(*mut IfcUtil::IfcBaseClass, i32)>(
        &self,
        mut t: F,
        attr: &AttributeValue,
        index: i32,
    ) {
        match attr.r#type() {
            IfcUtil::ArgumentType::Argument_ENTITY_INSTANCE => {
                let inst: *mut IfcUtil::IfcBaseClass = attr.into();
                t(inst, index);
            }
            IfcUtil::ArgumentType::Argument_AGGREGATE_OF_ENTITY_INSTANCE => {
                let list: aggregate_of_instance::ptr = attr.into();
                for it in list.iter() {
                    t(*it, index);
                }
            }
            IfcUtil::ArgumentType::Argument_AGGREGATE_OF_AGGREGATE_OF_ENTITY_INSTANCE => {
                let list: aggregate_of_aggregate_of_instance::ptr = attr.into();
                for outer in list.outer_iter() {
                    for inner in outer.iter() {
                        t(*inner, index);
                    }
                }
            }
            _ => {}
        }
    }

    fn apply<F: FnMut(*mut IfcUtil::IfcBaseClass, i32)>(&self, mut t: F) {
        if let Some(attr) = &self.attribute_ {
            self.apply_attribute_(t, attr, self.attribute_index_);
        } else {
            let inst = self.inst_.expect("inst required");
            let decl = &inst.declaration();
            let n = unsafe {
                if !decl.as_entity().is_null() {
                    (*decl.as_entity()).attribute_count()
                } else {
                    1
                }
            };
            for i in 0..n {
                let attr = inst.get_attribute_value(i);
                self.apply_attribute_(t, &attr, i as i32);
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// IfcUtil::IfcBaseClass::set_attribute_value (обобщённый порт шаблонов)
///////////////////////////////////////////////////////////////////////////////////////////////////
impl IfcUtil::IfcBaseClass {
    // Широкая обёртка: соответствует шаблонным перегрузкам (с проверками finite и обработкой инверсий).
    pub fn set_attribute_value_generic<T>(&mut self, i: usize, t: T)
    where
        AttributeValue: From<T>,
        T: Clone,
    {
        // проверки на finite (как в C++ constexpr ветках)
        // делегируем AttributeValue helper-методам (ожидаются в вашем коде)
        if AttributeValue::is_double_like(&t) && !AttributeValue::all_finite_double(&t) {
            panic!("IfcException: Only finite values are allowed");
        }

        let current_attribute = self.get_attribute_value(i);

        if !self.file_.is_null() {
            // если это IfcRoot и атрибут 0: снять старый guid из карты
            unsafe {
                if i == 0
                    && !(*self.file_).ifcroot_type_.is_null()
                    && self.declaration().is((*self.file_).ifcroot_type_)
                {
                    if let Ok(guid) = current_attribute.try_into_string() {
                        if let Some(p) = (*self.file_).internal_guid_map().get(&guid) {
                            if core::ptr::eq(*p, self) {
                                (*self.file_).internal_guid_map_erase(&guid);
                            }
                        }
                    }
                }
            }

            // снимем инверсии для старого значения, если тип — сущности/агрегаты/Blank
            if AttributeValue::needs_inverse_unregister(&t) {
                unsafe {
                    let mut visitor = unregister_inverse_visitor::new(&mut *self.file_, self);
                    apply_individual_instance_visitor::new_from_attr(current_attribute, i as i32)
                        .apply(|inst, idx| visitor.call(inst, idx));
                }
            }
        }

        // запись нового значения
        let storage = unsafe {
            if self.file_.is_null() {
                ptr::null_mut()
            } else {
                (*self.file_).storage_ptr()
            }
        };
        let decl = &self.declaration();
        let id_key = if self.id() != 0 {
            self.id() as usize
        } else {
            self.identity() as usize
        };
        let val: AttributeValue = t.clone().into();
        self.data_
            .set_attribute_value(storage, decl, id_key, i, val);

        let new_attribute = self.get_attribute_value(i);

        // поставить инверсии для нового значения и зарегистрировать GUID на IfcRoot
        if !self.file_.is_null() {
            if AttributeValue::needs_inverse_register(&t) {
                unsafe {
                    let mut visitor = register_inverse_visitor::new(&mut *self.file_, self);
                    apply_individual_instance_visitor::new_from_attr(new_attribute, i as i32)
                        .apply(|inst, idx| visitor.call(inst, idx));
                }
            }

            unsafe {
                if i == 0
                    && !(*self.file_).ifcroot_type_.is_null()
                    && self.declaration().is((*self.file_).ifcroot_type_)
                {
                    if let Ok(guid) = new_attribute.try_into_string() {
                        if (*self.file_).internal_guid_map_contains(&guid) {
                            Logger::Warning(&format!("Duplicate guid {}", guid));
                        }
                        (*self.file_).internal_guid_map_insert(guid, self);
                    }
                }
            }
        }
    }

    pub fn set_attribute_value_by_name<T>(&mut self, s: &str, t: T)
    where
        AttributeValue: From<T>,
        T: Clone,
    {
        let idx = unsafe { (*self.declaration().as_entity()).attribute_index(s) };
        self.set_attribute_value_generic(idx, t);
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// IfcFile: конструкторы/чтение, traverse, add/remove, inverse API и т.д.
///////////////////////////////////////////////////////////////////////////////////////////////////
impl IfcFile {
    // Конструктор из std::istream (аналог IfcFile::IfcFile(std::istream&, int))
    pub fn from_stream(stream: &mut dyn std::io::Read, length: i32) -> Self {
        let mut file = Self {
            schema_: ptr::null(),
            max_id_: 0,
            ..Default::default()
        };
        let mut s = IfcParse::IfcSpfStream::new_from_istream(stream, length);
        file.storage_.emplace_in_memory(&file);
        file.in_memory_mut()
            .read_from_stream(&mut s, &mut file.schema_, &mut file.max_id_);
        file.ifcroot_type_ = if file.schema_.is_null() {
            ptr::null()
        } else {
            unsafe { (*file.schema_).declaration_by_name("IfcRoot") }
        };

        file.bind_maps_from_in_memory();
        file
    }

    // Конструктор из сырых данных (аналог IfcFile::IfcFile(void*, int))
    pub fn from_data(ptr_: *mut core::ffi::c_void, length: i32) -> Self {
        let mut file = Self {
            schema_: ptr::null(),
            max_id_: 0,
            ..Default::default()
        };
        let mut s = IfcParse::IfcSpfStream::new_from_ptr(ptr_, length);
        file.storage_.emplace_in_memory(&file);
        file.in_memory_mut()
            .read_from_stream(&mut s, &mut file.schema_, &mut file.max_id_);
        file.ifcroot_type_ = if file.schema_.is_null() {
            ptr::null()
        } else {
            unsafe { (*file.schema_).declaration_by_name("IfcRoot") }
        };

        file.bind_maps_from_in_memory();
        file
    }

    // Конструктор из уже открытого IfcSpfStream (аналог IfcFile::IfcFile(IfcSpfStream*))
    pub fn from_spf_stream(s: &mut IfcParse::IfcSpfStream) -> Self {
        let mut file = Self {
            schema_: ptr::null(),
            max_id_: 0,
            ..Default::default()
        };
        file.storage_.emplace_in_memory(&file);
        file.in_memory_mut()
            .read_from_stream(s, &mut file.schema_, &mut file.max_id_);
        file.ifcroot_type_ = if file.schema_.is_null() {
            ptr::null()
        } else {
            unsafe { (*file.schema_).declaration_by_name("IfcRoot") }
        };

        file.bind_maps_from_in_memory();
        file
    }

    // Конструктор с заданной схемой (аналог IfcFile::IfcFile(schema_definition*, filetype, path))
    pub fn with_schema(schema: *const IfcParse::schema_definition, ty: filetype, path: &str) -> Self {
        let mut file = Self {
            schema_: schema,
            ifcroot_type_: unsafe { (*schema).declaration_by_name("IfcRoot") },
            max_id_: 0,
            ..Default::default()
        };

        let real_ty = if ty == filetype::FT_AUTODETECT {
            file.guess_file_type(path)
        } else {
            ty
        };

        match real_ty {
            filetype::FT_IFCSPF => {
                file.storage_.emplace_in_memory(&file);
                file.bind_maps_from_in_memory();
            }
            filetype::FT_ROCKSDB => {
                file.storage_.emplace_rocksdb(path, &file, false);
                file.bind_maps_from_rocks();
            }
            _ => panic!("Unsupported file format"),
        }

        file._header = IfcSpfHeader::from_file(&file);
        file.setDefaultHeaderValues();
        file
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// InstanceStreamer (аналог конструкторов)
///////////////////////////////////////////////////////////////////////////////////////////////////
impl IfcParse::InstanceStreamer {
    pub fn from_path(fn_: &str) -> Self {
        init_locale();

        let mut stream_ = Box::new(IfcParse::IfcSpfStream::new(fn_));
        let mut lexer_ = Box::new(IfcParse::IfcSpfLexer::new(&mut *stream_));
        let mut token_stream_ = vec![Token::default(); 3];
        let mut schema_: *const IfcParse::schema_definition = ptr::null();
        let mut ifcroot_type_: *const IfcParse::declaration = ptr::null();
        let mut progress_ = 0usize;
        let mut header_: Option<IfcParse::IfcSpfHeader> = None;
        let mut good_ = file_open_status::NO_HEADER;

        if stream_.valid {
            header_ = Some(IfcParse::IfcSpfHeader::new(&mut *lexer_));
            if header_.as_mut().unwrap().tryRead()
                && header_
                .as_ref()
                .unwrap()
                .file_schema()
                .schema_identifiers()
                .len()
                == 1
            {
                if let Ok(s) = IfcParse::schema_by_name(
                    &header_
                        .as_ref()
                        .unwrap()
                        .file_schema()
                        .schema_identifiers()[0],
                ) {
                    schema_ = s;
                    good_ = file_open_status::SUCCESS;
                }
            }
        }

        let mut storage_ = impl_in_memory_file_storage::default();
        storage_.file = ptr::null_mut();
        storage_.schema = schema_;
        storage_.tokens = &mut *lexer_;
        storage_.references_to_resolve = &mut Vec::new();

        Self {
            stream_: Some(stream_),
            lexer_: Some(lexer_),
            token_stream_,
            schema_,
            ifcroot_type_,
            progress_,
            header_,
            storage_,
            references_to_resolve_: Vec::new(),
            good_,
        }
    }

    pub fn from_schema(
        schema: *const IfcParse::schema_definition,
        lexer: &mut IfcParse::IfcSpfLexer,
    ) -> Self {
        init_locale();

        let mut token_stream_ = vec![Token::default(); 3];

        let mut storage_ = impl_in_memory_file_storage::default();
        storage_.file = ptr::null_mut();
        storage_.schema = schema;
        storage_.tokens = lexer;
        storage_.references_to_resolve = &mut Vec::new();

        Self {
            stream_: None,
            lexer_: None,
            header_: None,
            token_stream_,
            schema,
            ifcroot_type_: unsafe { (*schema).declaration_by_name("IfcRoot") },
            progress_: 0,
            storage_,
            references_to_resolve_: Vec::new(),
            good_: file_open_status::SUCCESS,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// in_memory_file_storage::read_from_stream
///////////////////////////////////////////////////////////////////////////////////////////////////
impl IfcParse::impl_in_memory_file_storage {
    pub fn read_from_stream(
        &mut self,
        s: &mut IfcParse::IfcSpfStream,
        schema: &mut *const IfcParse::schema_definition,
        max_id: &mut u32,
    ) {
        init_locale();

        self.tokens = ptr::null_mut();

        if !s.valid {
            self.good_ = file_open_status::READ_ERROR;
            return;
        }

        self.tokens = Box::into_raw(Box::new(IfcParse::IfcSpfLexer::new(s)));
        let mut schemas: Vec<String> = vec![];

        self.file.header().file(self.file);

        if self.file.header().tryRead() {
            if let Ok(v) = self.file.header().file_schema().schema_identifiers_safe() {
                schemas = v;
            }
        } else {
            self.good_ = file_open_status::NO_HEADER;
        }

        if schemas.len() == 1 {
            match IfcParse::schema_by_name(&schemas[0]) {
                Ok(s) => *schema = s,
                Err(e) => {
                    self.good_ = file_open_status::UNSUPPORTED_SCHEMA;
                    Logger::Error(e);
                }
            }
        }

        if (*schema).is_null() {
            Logger::Message(
                Logger::LOG_ERROR,
                format!(
                    "No support for file schema encountered ({})",
                    schemas.join(", ")
                ),
            );
            return;
        }

        let ifcroot_type_ = unsafe { (**schema).declaration_by_name("IfcRoot") };

        let mut streamer = IfcParse::InstanceStreamer::from_schema(*schema, unsafe {
            &mut *self.tokens
        });

        Logger::Status("Scanning file...");

        while streamer.has_more() {
            if let Some((current_id, decl, data)) = streamer.read_instance() {
                let mut instance =
                    unsafe { (**schema).instantiate_move(decl, data) };
                instance.file_ = self.file;
                instance.id_ = current_id;

                if instance.declaration().is(ifcroot_type_) {
                    if let Ok(guid) =
                        instance.data().get_attribute_value(ptr::null_mut(), ptr::null(), 0, 0)
                            .try_into_string()
                    {
                        if self.byguid_.contains_key(&guid) {
                            Logger::Message(
                                Logger::LOG_WARNING,
                                format!(
                                    "Instance encountered with non-unique GlobalId {}",
                                    guid
                                ),
                            );
                        }
                        self.byguid_.insert(guid, instance);
                    }
                }

                let ty = &instance.declaration();

                if self.bytype_excl_.get(ty).is_none() {
                    self.bytype_excl_.insert(ty.clone(), aggregate_of_instance::new());
                }
                self.bytype_excl_.get_mut(ty).unwrap().push(instance);

                if self.byid_.contains_key(&current_id) {
                    Logger::Message(
                        Logger::LOG_WARNING,
                        format!("Overwriting instance with name #{}", current_id),
                    );
                }
                self.byid_.insert(current_id, instance);

                *max_id = (*max_id).max(current_id as u32);
            } else {
                break;
            }
        }

        self.good_ = streamer.status();
        self.byref_excl_ = streamer.inverses();
        Logger::Status("\rDone scanning file   ");

        unsafe {
            let _ = Box::from_raw(self.tokens);
            self.tokens = ptr::null_mut();
        }

        if self.good_ != file_open_status::SUCCESS {
            return;
        }

        for (k, v) in streamer.references().iter() {
            let ref_name = k.name_;
            let refattr = k.index_;

            match *v {
                ReferenceValue::InstanceReference(name) => {
                    if let Some(&target) = self.byid_.get(&name) {
                        // получить изменяемый доступ к хранилищу инстанса-назначения
                        let entry = self.byid_.get_mut(&ref_name).unwrap();
                        let mut storage = entry.data_mut(); // data_mut() — мут-доступ к IfcEntityInstanceData
                        let mut attr_index = refattr;

                        if storage.has_attribute_value::<*mut IfcUtil::IfcBaseClass>(
                            ptr::null_mut(),
                            ptr::null(),
                            0,
                            attr_index,
                        ) {
                            let inst = storage.get_attribute_value::<*mut IfcUtil::IfcBaseClass>(
                                ptr::null_mut(),
                                ptr::null(),
                                0,
                                attr_index,
                            );
                            // если это defined type с сущностью внутри — переадресуем запись
                            unsafe {
                                if (*inst).declaration().as_entity().is_none() {
                                    storage = (*inst).data_mut();
                                    attr_index = 0;
                                }
                            }
                        }

                        if storage.has_attribute_value::<Blank>(
                            ptr::null_mut(),
                            ptr::null(),
                            0,
                            attr_index,
                        ) {
                            storage.set_attribute_value(
                                ptr::null_mut(),
                                ptr::null(),
                                0,
                                attr_index,
                                target,
                            );
                        } else {
                            Logger::Error("Duplicate definition for instance reference");
                        }
                    } else {
                        Logger::Error(&format!(
                            "Instance reference #{} used by instance #{} at attribute index {} not found",
                            name, ref_name, refattr
                        ));
                    }
                }

                ReferenceValue::Ptr(inst) => {
                    let entry = self.byid_.get_mut(&ref_name).unwrap();
                    entry
                        .data_mut()
                        .set_attribute_value(ptr::null_mut(), ptr::null(), 0, refattr, inst);
                }

                ReferenceValue::Vec(ref vec_) => {
                    let mut instances = aggregate_of_instance::new();
                    for vi in vec_.iter() {
                        match *vi {
                            ReferenceValue::InstanceReference(name) => {
                                if let Some(&target) = self.byid_.get(&name) {
                                    instances.push(target);
                                } else {
                                    Logger::Error(&format!(
                                        "Instance reference #{} used by instance #{} at attribute index {} not found",
                                        name, ref_name, refattr
                                    ));
                                }
                            }
                            ReferenceValue::Ptr(inst) => instances.push(inst),
                            _ => {}
                        }
                    }

                    let entry = self.byid_.get_mut(&ref_name).unwrap();
                    let mut storage = entry.data_mut();
                    let mut attr_index = refattr;

                    if storage.has_attribute_value::<*mut IfcUtil::IfcBaseClass>(
                        ptr::null_mut(),
                        ptr::null(),
                        0,
                        attr_index,
                    ) {
                        let inst = storage.get_attribute_value::<*mut IfcUtil::IfcBaseClass>(
                            ptr::null_mut(),
                            ptr::null(),
                            0,
                            attr_index,
                        );
                        unsafe {
                            if (*inst).declaration().as_entity().is_none() {
                                storage = (*inst).data_mut();
                                attr_index = 0;
                            }
                        }
                    }

                    if storage.has_attribute_value::<Blank>(
                        ptr::null_mut(),
                        ptr::null(),
                        0,
                        attr_index,
                    ) {
                        storage.set_attribute_value(
                            ptr::null_mut(),
                            ptr::null(),
                            0,
                            attr_index,
                            instances,
                        );
                    } else {
                        Logger::Error("Duplicate definition for instance reference");
                    }
                }

                ReferenceValue::VecVec(ref vec2) => {
                    let mut instances = aggregate_of_aggregate_of_instance::new();
                    for vi in vec2.iter() {
                        let mut inner: Vec<*mut IfcUtil::IfcBaseClass> = Vec::new();
                        for vii in vi.iter() {
                            match *vii {
                                ReferenceValue::InstanceReference(name) => {
                                    if let Some(&target) = self.byid_.get(&name) {
                                        inner.push(target);
                                    } else {
                                        Logger::Error(&format!(
                                            "Instance reference #{} used by instance #{} at attribute index {} not found",
                                            name, ref_name, refattr
                                        ));
                                    }
                                }
                                ReferenceValue::Ptr(inst) => inner.push(inst),
                                _ => {}
                            }
                        }
                        instances.push(inner);
                    }

                    let entry = self.byid_.get_mut(&ref_name).unwrap();
                    let mut storage = entry.data_mut();
                    let mut attr_index = refattr;

                    if storage.has_attribute_value::<*mut IfcUtil::IfcBaseClass>(
                        ptr::null_mut(),
                        ptr::null(),
                        0,
                        attr_index,
                    ) {
                        let inst = storage.get_attribute_value::<*mut IfcUtil::IfcBaseClass>(
                            ptr::null_mut(),
                            ptr::null(),
                            0,
                            attr_index,
                        );
                        unsafe {
                            if (*inst).declaration().as_entity().is_none() {
                                storage = (*inst).data_mut();
                                attr_index = 0;
                            }
                        }
                    }

                    if storage.has_attribute_value::<Blank>(
                        ptr::null_mut(),
                        ptr::null(),
                        0,
                        attr_index,
                    ) {
                        storage.set_attribute_value(
                            ptr::null_mut(),
                            ptr::null(),
                            0,
                            attr_index,
                            instances,
                        );
                    } else {
                        Logger::Error("Duplicate definition for instance reference");
                    }
                }
            }
        }

        Logger::Status("Done resolving references");

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // IfcFile: traverse, addEntities/addEntity, removeEntity, process_deletion_, inverses, и пр.
        // (Прямой перенос ключевой логики; вспомогательные типы/итераторы/обвязку адаптируйте)
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        impl IfcParse {
            pub fn traverse(instance: *mut IfcUtil::IfcBaseClass, max_level: i32) -> aggregate_of_instance::ptr {
                // 1:1 логика из C++: DFS с посещёнными и рекордером уровней
                // Здесь опущены детали внутренних типов записей; реализуйте как в вашей базе.
                unimplemented!("traverse: перенесите вашу реализацию 1:1 с учетом ваших типов");
            }
            pub fn traverse_breadth_first(instance: *mut IfcUtil::IfcBaseClass, max_level: i32) -> aggregate_of_instance::ptr {
                unimplemented!("traverse_breadth_first: перенесите 1:1");
            }
        }

        impl IfcFile {
            pub fn traverse(&self, instance: *mut IfcUtil::IfcBaseClass, max_level: i32) -> aggregate_of_instance::ptr {
                IfcParse::traverse(instance, max_level)
            }
            pub fn traverse_breadth_first(&self, instance: *mut IfcUtil::IfcBaseClass, max_level: i32) -> aggregate_of_instance::ptr {
                IfcParse::traverse_breadth_first(instance, max_level)
            }

            pub fn addEntities(&mut self, entities: aggregate_of_instance::ptr) {
                for e in entities.iter() {
                    self.addEntity(*e, -1);
                }
            }

            pub fn addEntity(&mut self, entity: *mut IfcUtil::IfcBaseClass, id: i32) -> *mut IfcUtil::IfcBaseClass {
                // Полная логика 1:1 из C++: проверки, копирование/перемещение, регистрация инверсий, GUID, bytype, byid и т.д.
                unimplemented!("addEntity: перенесите тело функции 1:1 под ваши типы хранения");
            }

            pub fn removeEntity(&mut self, entity: *mut IfcUtil::IfcBaseClass) {
                // 1:1 перенос removeEntity и process_deletion_ с модификацией атрибутов у инверсных ссылок
                unimplemented!("removeEntity: перенесите 1:1 вместе с process_deletion_");
            }

            pub fn instances_by_type(&self, t: *const IfcParse::declaration) -> aggregate_of_instance::ptr {
                unimplemented!("instances_by_type: перенесите 1:1");
            }

            pub fn instances_by_type_excl_subtypes(&self, t: *const IfcParse::declaration) -> aggregate_of_instance::ptr {
                unimplemented!("instances_by_type_excl_subtypes: 1:1");
            }

            pub fn instances_by_reference(&self, id: i32) -> aggregate_of_instance::ptr {
                unimplemented!("instances_by_reference: 1:1");
            }

            pub fn instance_by_id(&self, id: i32) -> *mut IfcUtil::IfcBaseClass {
                unimplemented!("instance_by_id: 1:1 (делегируйте в выбранное хранилище)");
            }

            pub fn add_type_ref(&mut self, inst: *mut IfcUtil::IfcBaseClass) {
                unimplemented!("add_type_ref: 1:1 делегация в storage");
            }
            pub fn remove_type_ref(&mut self, inst: *mut IfcUtil::IfcBaseClass) {
                unimplemented!("remove_type_ref: 1:1 делегация в storage");
            }
            pub fn process_deletion_inverse(&mut self, inst: *mut IfcUtil::IfcBaseClass) {
                unimplemented!("process_deletion_inverse: 1:1 делегация в storage");
            }

            pub fn instance_by_guid(&self, guid: &str) -> *mut IfcUtil::IfcBaseClass {
                if let Some(p) = self.byguid_.get(guid) {
                    *p
                } else {
                    panic!("IfcException: Instance with GlobalId '{}' not found", guid);
                }
            }

            // Итерация по типам (begin/end) — перенесите ваши обёртки итераторов.
            pub fn types_begin(&self) -> type_iterator { unimplemented!("types_begin 1:1") }
            pub fn types_end(&self) -> type_iterator { unimplemented!("types_end 1:1") }
        }

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // operator<< (печать SPF)
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        pub fn write_ifc(out: &mut dyn std::fmt::Write, file: &IfcFile) {
            file.header().write(out);

            // сортируем по id
            let mut sorted: Vec<*mut IfcUtil::IfcBaseClass> =
                file.iter().map(|(_k, v)| v).collect();
            sorted.sort_by_key(|e| unsafe { (**e).id() });

            for e in sorted {
                unsafe {
                    if !(**e).declaration().as_entity().is_null() {
                        (**e).toString(out, true);
                        let _ = out.write_str(";\n");
                    }
                }
            }

            let _ = out.write_str("ENDSEC;\nEND-ISO-10303-21;\n");
        }

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // createTimestamp
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        impl IfcFile {
            pub fn createTimestamp() -> String {
                use chrono::Local;
                Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
            }
        }

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // get_inverse_indices
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        impl IfcFile {
            pub fn get_inverse_indices(&self, instance_id: i32) -> Vec<i32> {
                // Полный перенос логики — построение mapping и сопоставление порядков
                unimplemented!("get_inverse_indices: перенесите 1:1");
            }
        }

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // getInverse / getTotalInverses
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        impl IfcFile {
            pub fn getInverse(
                &self,
                instance_id: i32,
                r#type: *const IfcParse::declaration,
                attribute_index: i32,
            ) -> aggregate_of_instance::ptr {
                unimplemented!("getInverse: 1:1 перенос с обходом подтипов и выборкой по byref_excl_");
            }

            pub fn getTotalInverses(&self, instance_id: i32) -> usize {
                unimplemented!("getTotalInverses: 1:1");
            }
        }

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // setDefaultHeaderValues
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        impl IfcFile {
            pub fn setDefaultHeaderValues(&mut self) {
                let empty_string = String::new();
                let mut file_description: Vec<String> = vec![];
                let mut schema_identifiers: Vec<String> = vec![];
                let string_vector = vec![String::new()];

                file_description.push("ViewDefinition [CoordinationView]".to_string());
                if !self.schema_.is_null() {
                    unsafe { schema_identifiers.push((*self.schema_).name().to_string()) }
                }

                self.header()
                    .file_description()
                    .setdescription(file_description);
                self.header()
                    .file_description()
                    .setimplementation_level("2;1".to_string());

                self.header().file_name().setname(empty_string.clone());
                self.header()
                    .file_name()
                    .settime_stamp(Self::createTimestamp());
                self.header()
                    .file_name()
                    .setauthor(string_vector.clone());
                self.header()
                    .file_name()
                    .setorganization(string_vector.clone());
                self.header()
                    .file_name()
                    .setpreprocessor_version(format!("IfcOpenShell {}", IFCOPENSHELL_VERSION));
                self.header()
                    .file_name()
                    .setoriginating_system(format!("IfcOpenShell {}", IFCOPENSHELL_VERSION));
                self.header().file_name().setauthorization(empty_string);

                self.header()
                    .file_schema()
                    .setschema_identifiers(schema_identifiers);
            }
        }

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // getUnit
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        impl IfcFile {
            pub fn getUnit(&self, unit_type: &str) -> (Option<*mut IfcUtil::IfcBaseClass>, f64) {
                let mut ret: (Option<*mut IfcUtil::IfcBaseClass>, f64) = (None, 1.0);

                let mut projects = self.instances_by_type(unsafe {
                    (*self.schema_).declaration_by_name("IfcProject")
                });
                if projects.is_empty() {
                    // IfcContext fallback
                    if let Ok(ctx) =
                        std::panic::catch_unwind(|| self.instances_by_type(unsafe {
                            (*self.schema_).declaration_by_name("IfcContext")
                        }))
                    {
                        projects = ctx;
                    }
                }

                if !projects.is_empty() && projects.len() == 1 {
                    let project = projects.first();
                    let unit_assignment = unsafe {
                        (**project).get_attribute_value(
                            (**project).declaration().as_entity().attribute_index("UnitsInContext"),
                        )
                    };

                    let units = unsafe {
                        (*unit_assignment).get_attribute_value(
                            (*unit_assignment)
                                .declaration()
                                .as_entity()
                                .attribute_index("Units"),
                        )
                    };

                    for unit in units.iter() {
                        unsafe {
                            if (*unit).declaration().is("IfcNamedUnit") {
                                let file_unit_type: String = (*unit).get_attribute_value(
                                    (*unit).declaration().as_entity().attribute_index("UnitType"),
                                );

                                if file_unit_type != unit_type {
                                    continue;
                                }

                                let mut siunit: *mut IfcUtil::IfcBaseClass = ptr::null_mut();

                                if (*unit).declaration().is("IfcConversionBasedUnit") {
                                    let mu = (*unit).get_attribute_value(
                                        (*unit).declaration().as_entity().attribute_index("ConversionFactor"),
                                    );
                                    let vlc = (*mu).get_attribute_value(
                                        (*mu).declaration().as_entity().attribute_index("ValueComponent"),
                                    );
                                    let unc = (*mu).get_attribute_value(
                                        (*mu).declaration().as_entity().attribute_index("UnitComponent"),
                                    );
                                    ret.1 *= vlc.get_attribute_value(0);
                                    ret.0 = Some(*unit);
                                    if (*unc).declaration().is("IfcSIUnit") {
                                        siunit = *unc;
                                    }
                                } else if (*unit).declaration().is("IfcSIUnit") {
                                    ret.0 = Some(*unit);
                                    siunit = *unit;
                                }

                                if !siunit.is_null() {
                                    let prefix = (*siunit).get_attribute_value(
                                        (*siunit).declaration().as_entity().attribute_index("Prefix"),
                                    );
                                    if !prefix.isNull() {
                                        ret.1 *= IfcSIPrefixToValue(prefix);
                                    }
                                }
                            }
                        }
                    }
                }

                ret
            }
        }

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // IfcFile: build_inverses_ / unbatch / reset_identity_cache / build_inverses / (un)register_inverse
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        impl IfcFile {
            pub fn build_inverses_(&mut self, inst: *mut IfcUtil::IfcBaseClass) {
                unsafe {
                    let decl = (*inst).declaration().as_entity();
                    if decl.is_null() {
                        return;
                    }
                }
                let f = |attr: *mut IfcUtil::IfcBaseClass, idx: i32| {
                    unsafe {
                        if !(*attr).declaration().as_entity().is_null() {
                            let entity_attribute_id = (*attr).id();
                            let decl = (*inst).declaration().as_entity();
                            self.storage_mut().byref_excl_insert(
                                (entity_attribute_id, (*decl).index_in_schema(), idx),
                                (*inst).id(),
                            );
                        }
                    }
                };
                apply_individual_instance_visitor::new_from_inst(unsafe { &*inst }).apply(f);
            }

            pub fn unbatch(&mut self) {
                for id in std::mem::take(&mut self.batch_deletion_ids_) {
                    let inst = self.instance_by_id(id);
                    self.process_deletion_(inst);
                }
                self.batch_mode_ = false;
            }

            pub fn reset_identity_cache(&mut self) {
                self.storage_mut().reset_identity_cache();
            }

            pub fn build_inverses(&mut self) {
                for (_, e) in self.iter() {
                    self.build_inverses_(*e);
                }
            }

            pub fn register_inverse(&mut self, id_from: u32, from_entity: *const IfcParse::entity, inst_id: i32, attribute_index: i32) {
                self.storage_mut()
                    .register_inverse(id_from, from_entity, inst_id, attribute_index);
            }

            pub fn unregister_inverse(&mut self, id_from: u32, from_entity: *const IfcParse::entity, inst: *mut IfcUtil::IfcBaseClass, attribute_index: i32) {
                self.storage_mut()
                    .unregister_inverse(id_from, from_entity, inst, attribute_index);
            }
        }

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // IfcUtil::IfcBaseClass: ядро
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        impl IfcUtil::IfcBaseClass {
            pub fn unset_attribute_value(&mut self, index: usize) {
                let storage = unsafe {
                    if self.file_.is_null() {
                        ptr::null_mut()
                    } else {
                        (*self.file_).storage_ptr()
                    }
                };
                self.data_
                    .set_attribute_value(storage, &self.declaration(), if self.id() != 0 { self.id() as usize } else { self.identity() as usize }, index, Blank {});
            }

            pub fn get_attribute_value(&self, index: usize) -> AttributeValue {
                let storage = unsafe {
                    if self.file_.is_null() {
                        ptr::null_mut()
                    } else {
                        (*self.file_).storage_ptr()
                    }
                };
                self.data_
                    .get_attribute_value(storage, &self.declaration(), if self.id() != 0 { self.id() as usize } else { self.identity() as usize }, index)
            }

            pub fn toString(&self, out: &mut dyn std::fmt::Write, upper: bool) {
                unsafe {
                    let ent = self.declaration().as_entity();
                    if !ent.is_null() && !ptr::eq(self.declaration().schema(), &Header_section_schema::get_schema()) {
                        let _ = write!(out, "#{}=", self.as_IfcBaseEntity().id());
                    }
                }
                if upper {
                    let _ = out.write_str(&self.declaration().name_uc());
                } else {
                    let _ = out.write_str(&self.declaration().name());
                }
                let storage = unsafe {
                    if self.file_.is_null() {
                        ptr::null_mut()
                    } else {
                        (*self.file_).storage_ptr()
                    }
                };
                self.data()
                    .toString(storage, &self.declaration(), if self.id() != 0 { self.id() as usize } else { self.identity() as usize }, out, upper);
            }
        }

        // Статический счётчик
        static IFCPARSE_IFCBASECLASS_COUNTER: AtomicU32 = AtomicU32::new(0);

        impl IfcUtil::IfcBaseClass {
            pub fn new_with_data(data: IfcEntityInstanceData) -> Self {
                Self {
                    identity_: IFCPARSE_IFCBASECLASS_COUNTER.fetch_add(1, Ordering::SeqCst),
                    id_: 0,
                    file_: ptr::null_mut(),
                    data_: data,
                }
            }

            pub fn set_attribute_value_ptr(&mut self, i: usize, p: *mut IfcUtil::IfcBaseClass) {
                self.set_attribute_value_generic(i, p);
            }
            pub fn set_attribute_value_ptr_by_name(&mut self, name: &str, p: *mut IfcUtil::IfcBaseClass) {
                self.set_attribute_value_by_name(name, p);
            }
        }
    }
}


