use lekhni_shell_desktop::workspace::NotebookManager;
use std::fs;

#[test]
fn test_notebook_manager_uniqueness_and_storage() {
    let temp_dir = std::env::temp_dir().join(format!("lekhni_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&temp_dir).unwrap();

    let mut mgr = NotebookManager::new(temp_dir.clone());

    // 1. Initialized with default unique "General" notebook
    assert_eq!(mgr.notebooks.len(), 1);
    assert_eq!(mgr.notebooks[0], "General");
    assert_eq!(mgr.pages.len(), 1);
    assert_eq!(mgr.pages[0], "welcome.md");

    // 2. Test unique name generation when requesting existing name "General" (case-insensitive)
    let nb2 = mgr.create_notebook("general");
    assert_eq!(nb2, "general_1");
    assert_eq!(mgr.notebooks.len(), 2);

    let nb3 = mgr.create_notebook("General");
    assert_eq!(nb3, "General_2");
    assert_eq!(mgr.notebooks.len(), 3);

    // 3. Create distinct notebook "Personal"
    let nb4 = mgr.create_notebook("Personal");
    assert_eq!(nb4, "Personal");
    assert_eq!(mgr.notebooks.len(), 4);

    // 4. Create new pages inside active notebook
    let pg = mgr.create_page("ideas.md", b"# Ideas\n\n1. Blazing fast pure Rust editor\n").unwrap();
    assert_eq!(pg, "ideas.md");
    assert!(mgr.pages.contains(&"ideas.md".to_string()));

    // 5. Save and reload active document
    let test_doc = b"# Saved Note\n\nAtomic write verification.\n";
    mgr.save_active_content(test_doc).unwrap();
    let loaded = mgr.load_active_content();
    assert_eq!(loaded, test_doc);

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}
