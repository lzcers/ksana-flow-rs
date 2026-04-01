use serde_json::json;
use tempfile::tempdir;

use crate::agent::memory::{
    MemoryConfig, MemoryError, MemoryStore, MemoryToolConfig, MemoryView, register_memory_tools,
};
use crate::agent::{FsMemoryStore, GenericToolExecutor, ToolCall, ToolExecutor};

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

#[tokio::test]
async fn memory_tools_register_and_execute_via_generic_executor() {
    let memory_root = tempdir().expect("memory root should exist");

    let mut executor = GenericToolExecutor::new();
    register_memory_tools(
        &mut executor,
        MemoryToolConfig::from_config(MemoryConfig::new(memory_root.path())),
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
