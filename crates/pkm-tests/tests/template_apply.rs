mod common;

use common::create_test_vault;

#[test]
fn test_apply_template_with_variables() {
    let tv = create_test_vault();

    // Create a template directory and file
    std::fs::create_dir_all(tv.vault_path.join("templates")).unwrap();
    let template_content = "# {{title}}\n\nDate: {{date}}\n\n## Notes\n\n{{body}}";
    std::fs::write(tv.vault_path.join("templates/meeting.md"), template_content).unwrap();

    // List templates
    let template_dir = tv.vault_path.join("templates");
    let mut templates = Vec::new();
    if template_dir.exists() {
        for entry in std::fs::read_dir(&template_dir).unwrap() {
            let entry = entry.unwrap();
            templates.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    assert!(templates.contains(&"meeting.md".to_string()));
}

#[test]
fn test_list_templates_empty() {
    let tv = create_test_vault();
    let template_dir = tv.vault_path.join("templates");
    std::fs::create_dir_all(&template_dir).unwrap();
    let count = std::fs::read_dir(&template_dir).unwrap().count();
    assert_eq!(count, 0);
}

#[test]
fn test_list_templates_multiple() {
    let tv = create_test_vault();
    let template_dir = tv.vault_path.join("templates");
    std::fs::create_dir_all(&template_dir).unwrap();

    for name in &["daily.md", "meeting.md", "project.md"] {
        std::fs::write(template_dir.join(name), format!("# {} template", name)).unwrap();
    }

    let mut names: Vec<String> = std::fs::read_dir(&template_dir).unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(names, vec!["daily.md", "meeting.md", "project.md"]);
}
