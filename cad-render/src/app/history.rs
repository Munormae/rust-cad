use cad_core::Document;

/// Простой стек undo/redo со снапшотами всего документа.
/// Для drag-переноса есть временный backup, чтобы вся перетаскивалка была одним действием.
#[derive(Default, Clone)]
pub struct History {
    undo: Vec<Document>,
    redo: Vec<Document>,
    drag_backup: Option<Document>,
}

impl History {
    /// Обычная запись состояния (для add/delete/finish curve etc.)
    pub fn record(&mut self, doc: &Document) {
        self.undo.push(doc.clone());
        self.redo.clear();
    }

    /// Если ещё не зафиксирован backup для текущего drag — запомнить состояние ДО начала.
    pub fn ensure_drag_backup(&mut self, doc: &Document) {
        if self.drag_backup.is_none() {
            self.drag_backup = Some(doc.clone());
        }
    }

    pub fn has_drag_backup(&self) -> bool {
        self.drag_backup.is_some()
    }

    /// Завершить drag: положить backup в undo и очистить backup.
    pub fn commit_drag(&mut self, _doc_after: &mut Document) {
        if let Some(before) = self.drag_backup.take() {
            self.undo.push(before);
            self.redo.clear();
        }
    }

    pub fn undo(&mut self) -> Option<Document> {
        if let Some(prev) = self.undo.pop() {
            // текущий doc кладём в redo снаружи? Проще: вернём prev, а ответственность за текущее берёт вызывающий:
            // Мы не знаем текущий doc тут, поэтому не пушим его. Вызов: let prev = undo(); if prev { swap(doc, prev) }
            Some(prev)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<Document> {
        self.redo.pop()
    }
}