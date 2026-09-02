use std::collections::BTreeSet;

use anyhow::Result;
use docmost_local_mcp::{
    server::DocmostMcpServer,
    startup_config::WRITE_TOOL_NAMES,
    types::{AuthorityMode, StartupConfig},
};
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParam, ClientInfo, Tool},
};

const READ_TOOL_NAMES: [&str; 10] = [
    "list_spaces",
    "search_docs",
    "search_pages",
    "get_space",
    "get_page",
    "list_pages",
    "list_child_pages",
    "get_comments",
    "list_workspace_members",
    "get_current_user",
];

#[derive(Debug, Clone, Default)]
struct DummyClientHandler;

impl ClientHandler for DummyClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

fn write_config(names: &[&str]) -> StartupConfig {
    StartupConfig {
        authority_mode: AuthorityMode::Write,
        allowed_write_tools: names.iter().map(|name| (*name).to_string()).collect(),
        ..StartupConfig::default()
    }
}

#[test]
fn supported_write_allowlist_is_locked_to_exact_thirteen_names() {
    assert_eq!(
        WRITE_TOOL_NAMES,
        [
            "create_page",
            "update_page",
            "duplicate_page",
            "copy_page_to_space",
            "move_page",
            "move_page_to_space",
            "create_space",
            "update_space",
            "create_comment",
            "update_comment",
            "delete_page",
            "delete_space",
            "delete_comment",
        ]
    );
}

async fn listed_tools(config: StartupConfig) -> Result<Vec<Tool>> {
    let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
    let server_handle = tokio::spawn(async move {
        let server = DocmostMcpServer::new(config)?;
        server.serve(server_transport).await?.waiting().await?;
        anyhow::Ok(())
    });
    let client = DummyClientHandler.serve(client_transport).await?;
    let tools = client.list_tools(None).await?.tools;
    client.cancel().await?;
    server_handle.await??;
    Ok(tools)
}

fn tool_names(tools: &[Tool]) -> BTreeSet<String> {
    tools.iter().map(|tool| tool.name.to_string()).collect()
}

#[tokio::test]
async fn default_server_lists_only_exact_read_inventory() -> Result<()> {
    let tools = listed_tools(StartupConfig::default()).await?;
    let actual = tool_names(&tools);
    let expected = READ_TOOL_NAMES.map(str::to_string).into_iter().collect();
    assert_eq!(actual, expected);
    assert_eq!(
        tools.len(),
        READ_TOOL_NAMES.len(),
        "duplicate tool registered"
    );
    assert!(
        WRITE_TOOL_NAMES.iter().all(|name| !actual.contains(*name)),
        "default inventory exposed a persistent mutation: {actual:?}"
    );
    Ok(())
}

#[tokio::test]
async fn write_mode_exposes_only_the_exact_allowlisted_subset() -> Result<()> {
    let mut allowlists = WRITE_TOOL_NAMES
        .iter()
        .map(|name| vec![*name])
        .collect::<Vec<_>>();
    allowlists.push(vec!["move_page", "update_comment"]);
    allowlists.push(WRITE_TOOL_NAMES.to_vec());

    for allowlist in allowlists {
        let tools = listed_tools(write_config(&allowlist)).await?;
        let actual = tool_names(&tools);
        let mut expected = READ_TOOL_NAMES
            .map(str::to_string)
            .into_iter()
            .collect::<BTreeSet<_>>();
        expected.extend(allowlist.iter().map(|name| (*name).to_string()));
        assert_eq!(actual, expected, "allowlist {allowlist:?}");
        assert_eq!(tools.len(), expected.len(), "duplicate tool registered");
    }
    Ok(())
}

#[test]
fn server_revalidates_programmatic_authority_configuration() {
    let allowlist_in_read_only = StartupConfig {
        allowed_write_tools: BTreeSet::from(["create_page".to_string()]),
        ..StartupConfig::default()
    };
    assert!(DocmostMcpServer::new(allowlist_in_read_only).is_err());

    let empty_write_mode = StartupConfig {
        authority_mode: AuthorityMode::Write,
        ..StartupConfig::default()
    };
    assert!(DocmostMcpServer::new(empty_write_mode).is_err());

    let unknown_tool = StartupConfig {
        authority_mode: AuthorityMode::Write,
        allowed_write_tools: BTreeSet::from(["delete_everything".to_string()]),
        ..StartupConfig::default()
    };
    assert!(DocmostMcpServer::new(unknown_tool).is_err());
}

#[tokio::test]
async fn all_tools_expose_object_input_schemas() -> Result<()> {
    for tool in listed_tools(write_config(&WRITE_TOOL_NAMES)).await? {
        assert_eq!(
            tool.input_schema
                .get("type")
                .and_then(|value| value.as_str()),
            Some("object"),
            "tool {} must expose object input schema",
            tool.name
        );
    }
    Ok(())
}

#[tokio::test]
async fn every_persistent_mutation_has_expected_annotations() -> Result<()> {
    let tools = listed_tools(write_config(&WRITE_TOOL_NAMES)).await?;
    let expectations = [
        ("create_page", false),
        ("update_page", true),
        ("duplicate_page", false),
        ("copy_page_to_space", false),
        ("move_page", true),
        ("move_page_to_space", true),
        ("create_space", false),
        ("update_space", true),
        ("create_comment", false),
        ("update_comment", true),
        ("delete_page", true),
        ("delete_space", true),
        ("delete_comment", true),
    ];

    assert_eq!(expectations.len(), WRITE_TOOL_NAMES.len());
    for (name, destructive) in expectations {
        let tool = tools
            .iter()
            .find(|tool| tool.name == name)
            .unwrap_or_else(|| panic!("missing write tool {name}"));
        let annotations = tool
            .annotations
            .as_ref()
            .unwrap_or_else(|| panic!("{name} must have annotations"));
        assert_eq!(annotations.read_only_hint, Some(false), "{name}");
        assert_eq!(annotations.destructive_hint, Some(destructive), "{name}");
        assert_eq!(annotations.idempotent_hint, Some(false), "{name}");
    }
    Ok(())
}

#[tokio::test]
async fn delete_metadata_names_stable_targets_and_consequences() -> Result<()> {
    let tools = listed_tools(write_config(&WRITE_TOOL_NAMES)).await?;
    for (name, field, consequences) in [
        ("delete_page", "page_id", ["descendant", "trash"]),
        ("delete_space", "space_id", ["cascade", "attachment"]),
        ("delete_comment", "comment_id", ["repl", "cascade"]),
    ] {
        let tool = tools
            .iter()
            .find(|tool| tool.name == name)
            .unwrap_or_else(|| panic!("missing delete tool {name}"));
        let description = tool.description.as_deref().unwrap_or_default();
        assert!(description.contains("stable") && description.contains("UUID"));
        assert!(
            consequences
                .iter()
                .all(|needle| description.to_ascii_lowercase().contains(needle)),
            "{name} must disclose its cascade consequence: {description}"
        );
        let properties = tool.input_schema["properties"]
            .as_object()
            .expect("delete input properties");
        let field_schema = properties
            .get(field)
            .unwrap_or_else(|| panic!("{name} missing {field}"));
        let field_description = field_schema["description"].as_str().unwrap_or_default();
        assert!(field_description.contains("Stable") && field_description.contains("UUID"));
        assert!(
            tool.input_schema["required"]
                .as_array()
                .is_some_and(|required| required.iter().any(|value| value == field))
        );
    }
    Ok(())
}

#[tokio::test]
async fn server_get_page_requires_slug_id_schema() -> Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
    let server_handle = tokio::spawn(async move {
        let server = DocmostMcpServer::new(StartupConfig::default())?;
        server.serve(server_transport).await?.waiting().await?;
        anyhow::Ok(())
    });
    let client = DummyClientHandler.serve(client_transport).await?;
    let tools = client.list_tools(None).await?;
    let get_page = tools
        .tools
        .into_iter()
        .find(|tool| tool.name == "get_page")
        .expect("get_page tool should exist");
    let properties = get_page
        .input_schema
        .get("properties")
        .and_then(|value| value.as_object())
        .expect("get_page tool should expose properties");
    assert!(properties.contains_key("slug_id"));

    let error = client
        .call_tool(CallToolRequestParam {
            name: "get_page".into(),
            arguments: Some(serde_json::Map::new()),
        })
        .await
        .expect_err("missing slug_id should be rejected");
    assert!(error.to_string().contains("slug_id"));
    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

#[tokio::test]
async fn server_required_input_fields_are_present() -> Result<()> {
    let tools = listed_tools(write_config(&WRITE_TOOL_NAMES)).await?;
    for (tool_name, property_name) in [
        ("get_page", "slug_id"),
        ("get_space", "space_id"),
        ("list_pages", "space_id"),
        ("list_child_pages", "page_id"),
        ("get_comments", "page_id"),
        ("search_docs", "query"),
        ("search_pages", "query"),
        ("create_page", "space_id"),
        ("create_page", "title"),
        ("update_page", "page_id"),
        ("duplicate_page", "page_id"),
        ("copy_page_to_space", "page_id"),
        ("copy_page_to_space", "space_id"),
        ("move_page", "page_id"),
        ("move_page_to_space", "page_id"),
        ("move_page_to_space", "space_id"),
        ("create_space", "name"),
        ("create_space", "slug"),
        ("update_space", "space_id"),
        ("create_comment", "page_id"),
        ("create_comment", "markdown"),
        ("update_comment", "comment_id"),
        ("update_comment", "markdown"),
        ("delete_page", "page_id"),
        ("delete_space", "space_id"),
        ("delete_comment", "comment_id"),
    ] {
        let tool = tools
            .iter()
            .find(|tool| tool.name == tool_name)
            .unwrap_or_else(|| panic!("{tool_name} tool should exist"));
        let properties = tool
            .input_schema
            .get("properties")
            .and_then(|value| value.as_object())
            .unwrap_or_else(|| panic!("{tool_name} should expose properties"));
        assert!(properties.contains_key(property_name));
    }
    Ok(())
}
