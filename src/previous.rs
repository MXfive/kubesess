use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub enum Slot {
    SessionContext,
    SessionNamespace,
    GlobalContext,
    GlobalNamespace,
}

impl Slot {
    fn filename(self) -> &'static str {
        match self {
            Slot::SessionContext => "session_context",
            Slot::SessionNamespace => "session_namespace",
            Slot::GlobalContext => "global_context",
            Slot::GlobalNamespace => "global_namespace",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Slot::SessionContext => "previous session context",
            Slot::SessionNamespace => "previous session namespace",
            Slot::GlobalContext => "previous global context",
            Slot::GlobalNamespace => "previous global namespace",
        }
    }
}

fn dir(dest: &str) -> PathBuf {
    PathBuf::from(dest).join(".previous")
}

fn path(dest: &str, slot: Slot) -> PathBuf {
    dir(dest).join(slot.filename())
}

pub fn read(dest: &str, slot: Slot) -> Option<String> {
    let value = fs::read_to_string(path(dest, slot)).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn write(dest: &str, slot: Slot, value: &str) {
    let d = dir(dest);
    if fs::create_dir_all(&d).is_err() {
        return;
    }
    let _ = fs::write(path(dest, slot), value);
}
