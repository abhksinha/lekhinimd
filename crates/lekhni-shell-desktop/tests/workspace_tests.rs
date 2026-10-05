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

    // 5. Save and reload active document in welcome.md
    let welcome_idx = mgr.pages.iter().position(|p| p == "welcome.md").unwrap();
    mgr.active_page_idx = welcome_idx;
    let test_doc = b"# Saved Note\n\nAtomic write verification.\nSee [Ideas](ideas.md) and [[Nested/topic]].\n";
    mgr.save_active_content(test_doc).unwrap();
    let loaded = mgr.load_active_content();
    assert_eq!(loaded, test_doc);

    // 6. Test nested folders support
    mgr.create_page("Nested/topic.md", b"# Topic\n\nNested note content with unique keywords xyzzy.\n").unwrap();
    assert!(mgr.pages.iter().any(|p| p == "Nested/topic.md"));

    // 7. Test Trigram search index
    let hits = mgr.search_notes("xyzzy");
    assert!(!hits.is_empty());
    assert_eq!(hits[0].page_name, "Nested/topic.md");

    // 8. Test backlinks
    let bl_ideas = mgr.get_backlinks("ideas.md");
    assert!(bl_ideas.contains(&"welcome.md".to_string()));
    let bl_nested = mgr.get_backlinks("Nested/topic.md");
    assert!(bl_nested.contains(&"welcome.md".to_string()));

    // 9. Test asset saving
    let asset_path = mgr.save_asset("photo.png", b"\x89PNG\r\n\x1a\nfakeimagebytes").unwrap();
    assert_eq!(asset_path, "assets/photo.png");
    assert!(mgr.active_notebook_dir().unwrap().join("assets/photo.png").exists());

    // 10. Test renaming and moving
    let renamed = mgr.rename_page("ideas.md", "brilliant_ideas.md").unwrap();
    assert_eq!(renamed, "brilliant_ideas.md");
    assert!(mgr.pages.contains(&"brilliant_ideas.md".to_string()));

    let moved = mgr.move_page("brilliant_ideas.md", "General").unwrap();
    assert_eq!(moved, "General/brilliant_ideas.md");
    assert!(!mgr.pages.contains(&"brilliant_ideas.md".to_string()));

    // 11. Test deletion
    mgr.delete_page("Nested/topic.md").unwrap();
    assert!(!mgr.pages.iter().any(|p| p == "Nested/topic.md"));

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}
