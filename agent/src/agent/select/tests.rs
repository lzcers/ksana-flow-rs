use std::fs;

use serde_json::json;
use tempfile::tempdir;

use crate::agent::select::{
    FileSelector, FindRequest, GrepRequest, ListDirRequest, ReadFileRequest, SelectConfig,
    SelectToolConfig, register_select_tools,
};
use crate::agent::{FsSelector, GenericToolExecutor, ToolCall, ToolExecutor};

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
async fn select_tools_register_and_execute_via_generic_executor() {
    let workspace = tempdir().expect("workspace should exist");
    fs::create_dir_all(workspace.path().join("src")).expect("src should exist");
    fs::write(
        workspace.path().join("src/lib.rs"),
        "pub fn login() {}\npub fn logout() {}\n",
    )
    .expect("lib.rs should be written");

    let mut executor = GenericToolExecutor::new();
    register_select_tools(
        &mut executor,
        SelectToolConfig::from_config(SelectConfig::new(workspace.path())),
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
}

fn tool_call(id: &str, name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        call_type: Some("function".to_string()),
        index: None,
        function: None,
        name: Some(name.to_string()),
        arguments: Some(arguments),
    }
}
