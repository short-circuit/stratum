//! Headless tests for the template service commands:
//! `list_templates`, `save_template`, `apply_template`,
//! `install_starter_templates`, `install_starter_sample`.

mod common;

use common::create_test_vault;

#[tokio::test]
async fn save_template_then_list_and_apply() {
    let vault = create_test_vault();

    vault
        .service
        .save_template("meeting".into(), "# {{title}}\n\nDate: {{date}}\n".into())
        .await
        .unwrap();

    let templates = vault.service.list_templates().await.unwrap();
    assert!(
        templates.iter().any(|t| t.name == "meeting"),
        "saved template must be listed: {templates:?}"
    );

    let rendered = vault
        .service
        .apply_template("meeting".into(), "pages/meeting.md".into(), vec![])
        .await
        .unwrap();

    assert!(rendered.contains("# pages/meeting.md"));
    assert!(!rendered.contains("{{date}}"));
    assert!(vault.exists("pages/meeting.md"));
}

#[tokio::test]
async fn apply_template_substitutes_custom_variables() {
    let vault = create_test_vault();
    vault
        .service
        .save_template("note".into(), "Owner: {{owner}}\n".into())
        .await
        .unwrap();

    let rendered = vault
        .service
        .apply_template(
            "note".into(),
            "pages/owned.md".into(),
            vec![("owner".into(), "Alice".into())],
        )
        .await
        .unwrap();

    assert_eq!(rendered.trim(), "Owner: Alice");
}

#[tokio::test]
async fn apply_template_rejects_missing_template() {
    let vault = create_test_vault();

    let err = vault
        .service
        .apply_template("nope".into(), "pages/out.md".into(), vec![])
        .await
        .unwrap_err();

    assert!(err.contains("Template not found"), "got: {err}");
}

#[tokio::test]
async fn install_starter_templates_writes_bundled_templates() {
    let vault = create_test_vault();

    let installed = vault.service.install_starter_templates(None).await.unwrap();

    assert!(!installed.is_empty(), "bundled templates must be installed");
    for name in &installed {
        assert!(
            vault.exists(&format!("templates/{name}.md")),
            "template {name} must exist on disk"
        );
    }
}

#[tokio::test]
async fn install_starter_sample_seeds_an_empty_vault() {
    let vault = create_test_vault();

    let created = vault.service.install_starter_sample().await.unwrap();

    assert!(!created.is_empty(), "sample notes must be created");
    for path in &created {
        assert!(vault.exists(path), "sample note {path} must exist");
    }
}

#[tokio::test]
async fn install_starter_sample_is_a_noop_on_a_populated_vault() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/existing.md".into(), None)
        .await
        .unwrap();

    let created = vault.service.install_starter_sample().await.unwrap();

    assert!(created.is_empty(), "populated vault must not be reseeded");
}
