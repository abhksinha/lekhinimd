use std::collections::HashMap;
use std::sync::Mutex;
use lekhni_shell::Fs;
use lekhni_store::atomic_save::save_atomic;
use lekhni_store::external_edit::{check_external_edit, ConflictResolution, FileGeneration};
use lekhni_store::notebook::NotebookModel;
use lekhni_store::recovery::RecoveryJournal;

struct MockFs {
    files: Mutex<HashMap<String, Vec<u8>>>,
}

impl MockFs {
    fn new() -> Self {
        Self {
            files: Mutex::new(HashMap::new()),
        }
    }
}

impl Fs for MockFs {
    type Error = &'static str;

    fn read_bytes(&self, path: &str) -> Result<Vec<u8>, Self::Error> {
        let lock = self.files.lock().unwrap();
        lock.get(path).cloned().ok_or("File not found")
    }

    fn write_atomic(&self, path: &str, data: &[u8]) -> Result<(), Self::Error> {
        let mut lock = self.files.lock().unwrap();
        lock.insert(path.to_string(), data.to_vec());
        Ok(())
    }

    fn stat(&self, path: &str) -> Result<(u64, u64), Self::Error> {
        let lock = self.files.lock().unwrap();
        if let Some(bytes) = lock.get(path) {
            Ok((bytes.len() as u64, 1000))
        } else {
            Err("File not found")
        }
    }

    fn remove(&self, path: &str) -> Result<(), Self::Error> {
        let mut lock = self.files.lock().unwrap();
        lock.remove(path);
        Ok(())
    }

    fn rename(&self, from: &str, to: &str) -> Result<(), Self::Error> {
        let mut lock = self.files.lock().unwrap();
        if let Some(data) = lock.remove(from) {
            lock.insert(to.to_string(), data);
            Ok(())
        } else {
            Err("Source file not found")
        }
    }
}

#[test]
fn test_atomic_save_and_recovery() {
    let fs = MockFs::new();
    let content = b"# Important Notes\nContent saved.";

    save_atomic(&fs, "notebook/Note.md", content).expect("atomic save succeeds");

    let read_back = fs.read_bytes("notebook/Note.md").expect("file exists");
    assert_eq!(read_back, content);

    // Test recovery snapshot
    RecoveryJournal::write_snapshot(&fs, "notebook", "Note.md", b"# Unsaved edits").unwrap();
    let journal_path = RecoveryJournal::journal_path("notebook", "Note.md");
    assert!(fs.files.lock().unwrap().contains_key(&journal_path));

    RecoveryJournal::discard_snapshot(&fs, "notebook", "Note.md").unwrap();
    assert!(!fs.files.lock().unwrap().contains_key(&journal_path));
}

#[test]
fn test_external_edit_detection() {
    let saved = FileGeneration::new(1000, 50, 0x1234);

    // No change
    assert_eq!(
        check_external_edit(&saved, 1000, 50, false),
        ConflictResolution::UpToDate
    );

    // External change when clean buffer -> reload silently
    assert_eq!(
        check_external_edit(&saved, 1200, 60, false),
        ConflictResolution::ReloadSilently
    );

    // External change when dirty buffer -> conflict prompt
    assert_eq!(
        check_external_edit(&saved, 1200, 60, true),
        ConflictResolution::PromptConflict
    );
}

#[test]
fn test_notebook_model() {
    let mut nb = NotebookModel::new("/home/user/Notes".into());
    nb.add_page("page1.md".into(), "Page 1".into());
    nb.add_page("sub/page2.md".into(), "Page 2".into());

    assert_eq!(nb.pages.len(), 2);
    assert_eq!(nb.active_page().unwrap().title, "Page 1");

    assert!(nb.select_page("sub/page2.md"));
    assert_eq!(nb.active_page().unwrap().title, "Page 2");
}
