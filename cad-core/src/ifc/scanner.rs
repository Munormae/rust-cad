// cad-core/src/ifc/scanner.rs
use anyhow::Result;

/// Примитивный построчный «сканер» IFC P21: собираем только строки с ENTITY.
/// Возвращает вектор строк вида: "#123 = IFCSOMETHING(...);"
pub fn read_p21_entities(text: &str) -> Vec<String> {
    // IFC допускает переносы/комментарии. Упростим: склеим до ';'
    let mut out = Vec::new();
    let mut cur = String::new();
    for line in text.lines() {
        let mut l = line.trim();
        if l.starts_with("//") {
            continue;
        }
        if l.is_empty() {
            continue;
        }
        // убираем комментарии в конце строки, если // обнаружен
        if let Some(idx) = l.find("//") {
            l = &l[..idx].trim_end();
        }
        cur.push_str(l);
        if cur.ends_with(';') {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(' ');
        }
    }
    out
}

pub fn load_file(path: &str) -> Result<String> {
    Ok(std::fs::read_to_string(path)?)
}
