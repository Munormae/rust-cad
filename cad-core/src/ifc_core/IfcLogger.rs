use chrono::{Local, Timelike};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// ===== Минимальные интерфейсы для интеграции с IfcOpenShell-типа кодом =====

/// Аналог IfcUtil::IfcBaseInterface → нужен только для строкового представления экземпляра
pub trait IfcBaseInterface: Send + Sync {
    /// Эквивалент C++: instance->as<IfcBaseClass>()->toString(oss)
    fn to_ifc_string(&self) -> String;
}

/// Аналог IfcUtil::IfcBaseClass (минимум того, что логгер использует)
pub trait IfcBaseClass: IfcBaseInterface {
    /// Нужен для SetProduct: entity->as<IfcBaseEntity>()->get("GlobalId")
    /// Верни Some(global_id) или None.
    fn global_id(&self) -> Option<String>;
}

/// ===== Логгер =====

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Severity {
    Perf = 0,
    Debug = 1,
    Notice = 2,
    Warning = 3,
    Error = 4,
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (*self as u8).partial_cmp(&(*other as u8))
    }
}
impl Ord for Severity {
    fn cmp(&self, other: &Self) -> Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Plain,
    Json,
}

static SEVERITY_STR: [&str; 5] = ["Performance", "Debug", "Notice", "Warning", "Error"];

fn now_string(with_millis: bool) -> String {
    let now = Local::now();
    let date = now.format("%F %T").to_string();
    if with_millis {
        let ms = now.nanosecond() / 1_000_000;
        format!("{date}.{ms:03}")
    } else {
        date
    }
}

#[derive(Default)]
struct LoggerState {
    // Выходы
    log1: Option<Box<dyn Write + Send>>,
    log2: Option<Box<dyn Write + Send>>,
    // внутренний буфер (как std::stringstream в C++)
    log_stream: String,

    // Настройки/состояние
    verbosity: Severity,
    max_severity: Severity,
    format: Format,

    current_product: Option<Arc<dyn IfcBaseClass>>,
    print_perf_stats_on_element: bool,

    first_timepoint: Option<Instant>,
    perf_stats: HashMap<String, f64>,        // суммарное время по имени
    perf_signal_start: HashMap<String, f64>, // последняя отметка старта
}

static LOGGER: Lazy<Mutex<LoggerState>> = Lazy::new(|| {
    let mut st = LoggerState::default();
    st.verbosity = Severity::Notice;
    st.max_severity = Severity::Notice;
    st.format = Format::Plain;
    Mutex::new(st)
});

pub struct Logger;

impl Logger {
    /// Установка/сброс текущего продукта (эквивалент SetProduct).
    /// При включённом DEBUG печатает "Begin processing {GlobalId}".
    /// При product=None и включённом `print_perf_stats_on_element` — печатает перф-статистику и очищает её.
    pub fn set_product(product: Option<Arc<dyn IfcBaseClass>>) {
        let mut lg = LOGGER.lock().unwrap();
        if let Some(ref p) = product {
            if lg.verbosity <= Severity::Debug {
                Self::message_locked(
                    &mut lg,
                    Severity::Debug,
                    "Begin processing",
                    Some(p.as_ref() as &dyn IfcBaseInterface),
                );
            }
        } else if lg.print_perf_stats_on_element {
            Self::print_perf_stats_locked(&mut lg);
            lg.perf_stats.clear();
        }
        lg.current_product = product;
    }

    /// Аналог двух перегрузок SetOutput: сюда можно передать log1 и log2.
    /// Если log2 == None — логи второй очереди будут писаться во внутренний буфер (log_stream).
    pub fn set_output(log1: Option<Box<dyn Write + Send>>, mut log2: Option<Box<dyn Write + Send>>) {
        let mut lg = LOGGER.lock().unwrap();
        lg.log1 = log1;
        if log2.is_none() {
            // как в C++: если второй не задан — пишем во внутренний буфер
            // тут оставляем None, а ниже реализация будет добавлять в lg.log_stream
        }
        lg.log2 = log2.take();
    }

    /// Стиль вывода (PLAIN / JSON)
    pub fn set_output_format(fmt: Format) {
        LOGGER.lock().unwrap().format = fmt;
    }
    pub fn output_format() -> Format {
        LOGGER.lock().unwrap().format
    }

    /// Громкость
    pub fn set_verbosity(sev: Severity) {
        LOGGER.lock().unwrap().verbosity = sev;
    }
    pub fn verbosity() -> Severity {
        LOGGER.lock().unwrap().verbosity
    }

    /// Максимальная встреченная серьёзность
    pub fn max_severity() -> Severity {
        LOGGER.lock().unwrap().max_severity
    }

    /// Включить/выключить печать перф. статистики при завершении элемента
    pub fn set_print_perf_stats_on_element(enable: bool) {
        LOGGER.lock().unwrap().print_perf_stats_on_element = enable;
    }

    /// Сообщение (строка). Эквивалент Message(type, message, instance)
    pub fn message(sev: Severity, message: &str, instance: Option<&dyn IfcBaseInterface>) {
        let mut lg = LOGGER.lock().unwrap();
        Self::message_locked(&mut lg, sev, message, instance);
    }

    /// Сообщение из исключения
    pub fn message_error<E: std::fmt::Display>(sev: Severity, err: E, instance: Option<&dyn IfcBaseInterface>) {
        Self::message(sev, &err.to_string(), instance);
    }

    /// Статусная строка (без форматирования), с \n или без него
    pub fn status(message: &str, new_line: bool) {
        let mut lg = LOGGER.lock().unwrap();
        if let Some(ref mut w1) = lg.log1 {
            let _ = write!(w1, "{message}");
            if new_line {
                let _ = writeln!(w1);
            } else {
                let _ = w1.flush();
            }
        } else {
            // wide-потоков в Rust нет; если не задан — складываем в буфер
            lg.log_stream.push_str(message);
            if new_line {
                lg.log_stream.push('\n');
            }
        }
    }

    /// Прогрессбар (как в C++: 50 символов)
    pub fn progress_bar(progress_0_50: usize) {
        let p = progress_0_50.min(50);
        let bar = format!("\r[{:#<width$}{: >rest$}]", "", "", width = p, rest = 50 - p);
        Self::status(&bar, false);
    }

    /// Внутренний буфер логов (вместо второго потока при его отсутствии)
    pub fn get_log() -> String {
        LOGGER.lock().unwrap().log_stream.clone()
    }

    /// Напечатать перф-статистику в порядке убывания времени
    pub fn print_performance_stats() {
        let mut lg = LOGGER.lock().unwrap();
        Self::print_perf_stats_locked(&mut lg);
    }

    // ======== приватные части ========

    fn ensure_perf_clock_started(lg: &mut LoggerState) -> f64 {
        if lg.first_timepoint.is_none() {
            lg.first_timepoint = Some(Instant::now());
        }
        let base = lg.first_timepoint.unwrap();
        let dt = base.elapsed();
        dt.as_secs_f64()
    }

    fn message_locked(
        lg: &mut LoggerState,
        sev: Severity,
        message: &str,
        instance: Option<&dyn IfcBaseInterface>,
    ) {
        if sev < lg.verbosity {
            return;
        }

        // PERF учёт (как в C++)
        if sev == Severity::Perf {
            let t0 = Self::ensure_perf_clock_started(lg);
            if let Some(orig) = message.strip_prefix("done ") {
                if let Some(start) = lg.perf_signal_start.get(orig).copied() {
                    *lg.perf_stats.entry(orig.to_string()).or_insert(0.0) += t0 - start;
                }
            } else {
                lg.perf_signal_start.insert(message.to_string(), t0);
            }
        }

        if sev > lg.max_severity {
            lg.max_severity = sev;
        }

        // выбор вывода: log2 → если нет, в буфер log_stream
        match lg.format {
            Format::Plain => {
                let mut buf = String::new();
                // "[Severity] [YYYY-MM-DD HH:MM:SS(.mmm)] {GlobalId} message"
                let _ = write!(&mut buf, "[{}] ", SEVERITY_STR[sev as usize]);
                let _ = write!(
                    &mut buf,
                    "[{}] ",
                    now_string(sev <= Severity::Perf /* PERF/DEBUG → с мс */)
                );

                if let Some(ref prod) = lg.current_product {
                    if let Some(guid) = prod.global_id() {
                        let _ = write!(&mut buf, "{{{}}} ", guid);
                    }
                }
                let _ = write!(&mut buf, "{}", message);

                // Пишем строку
                if let Some(ref mut w2) = lg.log2 {
                    let _ = writeln!(w2, "{}", buf);
                    if let Some(inst) = instance {
                        let mut inst_s = inst.to_ifc_string();
                        if inst_s.len() > 259 {
                            inst_s.truncate(256);
                            inst_s.push_str("...");
                        }
                        let _ = writeln!(w2, "{}", inst_s);
                    }
                } else {
                    lg.log_stream.push_str(&buf);
                    lg.log_stream.push('\n');
                    if let Some(inst) = instance {
                        let mut inst_s = inst.to_ifc_string();
                        if inst_s.len() > 259 {
                            inst_s.truncate(256);
                            inst_s.push_str("...");
                        }
                        lg.log_stream.push_str(&inst_s);
                        lg.log_stream.push('\n');
                    }
                }
            }
            Format::Json => {
                #[derive(Serialize)]
                struct JsonMsg<'a> {
                    time: &'a str,
                    level: &'a str,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    product: Option<String>,
                    message: &'a str,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    instance: Option<String>,
                }

                let time_s = now_string(false);
                let product = lg
                    .current_product
                    .as_ref()
                    .and_then(|p| Some(p.to_ifc_string()));
                let instance_s = instance.map(|i| i.to_ifc_string());

                let obj = JsonMsg {
                    time: &time_s,
                    level: SEVERITY_STR[sev as usize],
                    product,
                    message,
                    instance: instance_s,
                };

                let line = serde_json::to_string(&obj).unwrap_or_else(|_| {
                    // на всякий случай fallback
                    format!(
                        r#"{{"time":"{}","level":"{}","message":"{}"}}"#,
                        time_s, SEVERITY_STR[sev as usize], escape_json(message)
                    )
                });

                if let Some(ref mut w2) = lg.log2 {
                    let _ = writeln!(w2, "{}", line);
                } else {
                    lg.log_stream.push_str(&line);
                    lg.log_stream.push('\n');
                }
            }
        }
    }

    fn print_perf_stats_locked(lg: &mut LoggerState) {
        let mut items: Vec<(f64, String)> = lg
            .perf_stats
            .iter()
            .map(|(k, v)| (*v, k.clone()))
            .collect();

        items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

        let max_len = items.iter().map(|(_, s)| s.len()).max().unwrap_or(0);
        for (t, name) in items {
            let mut msg = String::new();
            let _ = write!(
                &mut msg,
                "{}{}: {}",
                name,
                " ".repeat(max_len.saturating_sub(name.len())),
                t
            );
            Self::message_locked(lg, Severity::Perf, &msg, None);
        }
    }
}

/// Простая JSON-экранизация (на случай fallback)
fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(&mut out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

/// ==== Пример «заглушечной» реализации интерфейсов (удали в продакшене) ====
///
/// struct Dummy implements IfcBaseClass

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy(String);
    impl IfcBaseInterface for Dummy {
        fn to_ifc_string(&self) -> String {
            format!("Dummy({})", self.0)
        }
    }
    impl IfcBaseClass for Dummy {
        fn global_id(&self) -> Option<String> {
            Some(self.0.clone())
        }
    }

    #[test]
    fn smoke() {
        // log2 → в буфер
        Logger::set_output(None, None);
        Logger::set_output_format(Format::Plain);
        Logger::set_verbosity(Severity::Debug);

        let prod = Arc::new(Dummy("ABCDEF".into()));
        Logger::set_product(Some(prod));

        Logger::message(Severity::Notice, "Hello", None);
        Logger::message(Severity::Perf, "phase1", None);
        std::thread::sleep(Duration::from_millis(5));
        Logger::message(Severity::Perf, "done phase1", None);

        Logger::set_product(None);
        let s = Logger::get_log();
        assert!(s.contains("Hello"));
        assert!(s.contains("Performance"));
    }
}
