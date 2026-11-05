use std::collections::HashSet;

pub struct Resource<T> {
    pub owner_id: usize,                // ID главного владельца (полные права)
    pub readers: HashSet<usize>,        // Set ID модулей, у которых есть доступ на чтение
    pub data: T,                        // Сам объект (File, TcpStream и т.д.)
}

impl<T> Resource<T> {
    pub fn new(owner_id: usize, data: T) -> Self {
        Self {
            owner_id,
            readers: HashSet::new(),
            data,
        }
    }

    // Проверка: имеет ли модуль право хотя бы читать?
    pub fn can_read(&self, module_id: usize) -> bool {
        self.owner_id == module_id || self.readers.contains(&module_id)
    }

    // Проверка: является ли модуль полноправным владельцем?
    pub fn is_owner(&self, module_id: usize) -> bool {
        self.owner_id == module_id
    }

    // --- Управление правами ---

    /// Добавить нового читателя (если он не владелец)
    pub fn add_reader(&mut self, module_id: usize) -> bool {
        if self.owner_id == module_id {
            return false;
        }
        self.readers.insert(module_id)
    }

    /// Отозвать доступ у конкретного читателя
    pub fn remove_reader(&mut self, module_id: usize) -> bool {
        self.readers.remove(&module_id)
    }

    /// Сменить владельца. Новый владелец удаляется из читателей (если там был),
    /// а старый владелец может по желанию остаться читателем или уйти.
    pub fn transfer_ownership(&mut self, new_owner_id: usize, keep_old_as_reader: bool) {
        let old_owner = self.owner_id;

        // Убираем нового владельца из читателей, чтобы не было пересечений
        self.readers.remove(&new_owner_id);

        // Назначаем нового хозяина
        self.owner_id = new_owner_id;

        // Если старый владелец должен сохранить доступ на чтение — кидаем его в readers
        if keep_old_as_reader && old_owner != new_owner_id {
            self.readers.insert(old_owner);
        }
    }

    /// Очистить всех читателей разом
    pub fn clear_readers(&mut self) {
        self.readers.clear();
    }
}