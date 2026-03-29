use std::fs;

use serde_json::json;
use tempfile::tempdir;

use crate::agents::memory::{
    FileSelector, FindRequest, FsMemoryStore, FsSelector, GrepRequest, ListDirRequest,
    MemoryConfig, MemoryError, MemoryStore, MemoryToolConfig, MemoryView, ReadFileRequest,
    SelectConfig, register_memory_tools,
};
use crate::agents::{GenericToolExecutor, ToolCall, ToolExecutor};

#[test]
fn fs_memory_store_supports_crud_and_guards() {
    let root = tempdir().expect("temp dir should exist");
    let mut store =
        FsMemoryStore::new(MemoryConfig::new(root.path())).expect("memory store should be created");

    store
        .create(
            "/memories/notes/analysis.md",
            "# JWT\n\n- Entry point: login()\n- Entry point: login()\n",
        )
        .expect("create should work");

    let listing = store.view("/memories", None).expect("listing should work");
    assert!(matches!(listing, MemoryView::Directory(_)));

    let duplicate = store.replace_text(
        "/memories/notes/analysis.md",
        "Entry point: login()",
        "Entry point: authenticate()",
    );
    assert!(matches!(
        duplicate,
        Err(MemoryError::MultipleOccurrences { .. })
    ));

    store
        .create(
            "/memories/notes/unique.md",
            "# JWT\n\n- Entry point: login()\n- Dependencies: utils.py\n",
        )
        .expect("unique file should be created");
    store
        .replace_text(
            "/memories/notes/unique.md",
            "Entry point: login()",
            "Entry point: authenticate()",
        )
        .expect("replace should work");
    store
        .insert("/memories/notes/unique.md", 3, "- Security: JWT tokens")
        .expect("insert should work");
    store
        .rename("/memories/notes/unique.md", "/memories/notes/jwt.md")
        .expect("rename should work");

    let snapshot = store
        .view("/memories/notes/jwt.md", None)
        .expect("snapshot should work");
    match snapshot {
        MemoryView::File(snapshot) => {
            let rendered = snapshot.to_string();
            assert!(rendered.contains("authenticate"));
            assert!(rendered.contains("Security: JWT tokens"));
        }
        MemoryView::Directory(_) => panic!("expected file snapshot"),
    }

    let invalid = store.create("/tmp/outside.md", "nope");
    assert!(matches!(invalid, Err(MemoryError::InvalidPath(_))));

    store
        .delete("/memories/notes/jwt.md")
        .expect("delete should work");
    store.clear_all().expect("clear_all should work");

    let cleared = store
        .view("/memories", None)
        .expect("root should still exist");
    match cleared {
        MemoryView::Directory(listing) => assert_eq!(listing.entries.len(), 1),
        MemoryView::File(_) => panic!("expected directory listing"),
    }
}

#[test]
fn fs_selector_supports_list_find_grep_and_read() {
    let workspace = tempdir().expect("temp dir should exist");
    fs::create_dir_all(workspace.path().join("src")).expect("src should exist");
    fs::write(
        workspace.path().join("src/auth.py"),
        "def login(user):\n    return user\n\n\ndef logout(user):\n    return None\n",
    )
    .expect("auth.py should be written");
    fs::write(
        workspace.path().join("src/utils.py"),
        "def helper():\n    return 'ok'\n",
    )
    .expect("utils.py should be written");

    let selector = FsSelector::new(SelectConfig::new(workspace.path()));

    let mut list_request = ListDirRequest::new(".");
    list_request.max_depth = Some(1);
    let entries = selector
        .list_dir(&list_request)
        .expect("list_dir should work");
    assert!(
        entries
            .iter()
            .any(|entry| entry.path == "src" && entry.is_dir)
    );

    let mut find_request = FindRequest::new(".");
    find_request.name_pattern = Some("*.py".into());
    find_request.only_files = true;
    find_request.max_depth = Some(2);
    let files = selector.find(&find_request).expect("find should work");
    assert!(files.contains(&"src/auth.py".to_string()));
    assert!(files.contains(&"src/utils.py".to_string()));

    let mut grep_request = GrepRequest::new("def ", ".");
    grep_request.file_pattern = Some("*.py".into());
    grep_request.max_results = Some(5);
    let matches = selector.grep(&grep_request).expect("grep should work");
    assert_eq!(matches[0].file, "src/auth.py");

    let mut read_request = ReadFileRequest::new("src/auth.py");
    read_request.start_line = Some(1);
    read_request.end_line = Some(3);
    let content = selector
        .read_file(&read_request)
        .expect("read_file should work");
    assert_eq!(content.start_line, 1);
    assert_eq!(content.end_line, 3);
    assert_eq!(content.total_lines, 6);
    assert!(content.content.contains("def login(user):"));
}

#[tokio::test]
async fn memory_tools_register_and_execute_via_generic_executor() {
    let memory_root = tempdir().expect("memory root should exist");
    let workspace = tempdir().expect("workspace should exist");
    fs::create_dir_all(workspace.path().join("src")).expect("src should exist");
    fs::write(
        workspace.path().join("src/lib.rs"),
        "pub fn login() {}\npub fn logout() {}\n",
    )
    .expect("lib.rs should be written");

    let mut executor = GenericToolExecutor::new();
    register_memory_tools(
        &mut executor,
        MemoryToolConfig::from_configs(
            MemoryConfig::new(memory_root.path()),
            SelectConfig::new(workspace.path()),
        ),
    )
    .expect("tools should register");

    let write_result = executor
        .execute(&tool_call(
            "call_write",
            "memory_write",
            json!({
                "path": "/memories/analysis.md",
                "content": "# Analysis\n\n- target: auth module"
            }),
        ))
        .await
        .expect("memory_write should succeed");
    assert_eq!(write_result.output["ok"], json!(true));

    let read_result = executor
        .execute(&tool_call(
            "call_read",
            "memory_read",
            json!({
                "path": "/memories/analysis.md"
            }),
        ))
        .await
        .expect("memory_read should succeed");
    assert_eq!(read_result.output["kind"], json!("file"));
    assert!(
        read_result.output["lines"]
            .to_string()
            .contains("target: auth module")
    );

    let search_result = executor
        .execute(&tool_call(
            "call_search",
            "file_search",
            json!({
                "path": ".",
                "pattern": "login",
                "file_pattern": "*.rs"
            }),
        ))
        .await
        .expect("file_search should succeed");
    assert_eq!(
        search_result.output["matches"][0]["file"],
        json!("src/lib.rs")
    );

    let read_file_result = executor
        .execute(&tool_call(
            "call_file_read",
            "file_read",
            json!({
                "path": "src/lib.rs",
                "start_line": 1,
                "end_line": 1
            }),
        ))
        .await
        .expect("file_read should succeed");
    assert!(
        read_file_result.output["content"]
            .as_str()
            .unwrap_or_default()
            .contains("login")
    );

    let bad_call = executor
        .execute(&tool_call(
            "call_bad",
            "memory_write",
            json!({
                "path": "/tmp/outside.md",
                "content": "nope"
            }),
        ))
        .await;
    assert!(bad_call.is_err());
}

fn tool_call(id: &str, name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        call_type: None,
        index: None,
        function: None,
        name: Some(name.to_string()),
        arguments: Some(arguments),
    }
}
