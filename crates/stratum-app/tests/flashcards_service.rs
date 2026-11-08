//! Headless tests for the flashcard service commands:
//! `generate_flashcards`, `generate_cards_from_page`, `review_card`.

mod common;

use common::{block_dto, create_test_vault};

#[tokio::test]
async fn generate_flashcards_reads_question_answer_properties() {
    let vault = create_test_vault();
    let mut block = block_dto("What language is memory safe?");
    block.properties = vec![
        ("question".into(), "true".into()),
        ("answer".into(), "Rust".into()),
    ];
    vault
        .service
        .save_blocks("pages/cards.md".into(), vec![block], None)
        .await
        .unwrap();

    let cards = vault.service.generate_flashcards().await.unwrap();

    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].front, "What language is memory safe?");
    assert_eq!(cards[0].back, "Rust");
    assert_eq!(cards[0].page_path, "pages/cards.md");
}

#[tokio::test]
async fn generate_cards_from_page_parses_q_and_a_markdown() {
    let vault = create_test_vault();
    vault.write_file(
        "pages/qa.md",
        "---\ntitle: QA\n---\n\nQ: What is 2+2?\nA: 4\n\n---\n\nQ: Capital of France?\nA: Paris\n",
    );

    let cards = vault
        .service
        .generate_cards_from_page("pages/qa.md".into())
        .await
        .unwrap();

    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0].front, "What is 2+2?");
    assert_eq!(cards[0].back, "4");
    assert_eq!(cards[1].back, "Paris");
}

#[tokio::test]
async fn review_card_updates_sm2_schedule() {
    let vault = create_test_vault();
    let mut block = block_dto("Question");
    block.properties = vec![("answer".into(), "Answer".into())];
    let block_id = block.id.clone();
    vault
        .service
        .save_blocks("pages/review.md".into(), vec![block], None)
        .await
        .unwrap();

    let card = vault
        .service
        .review_card(block_id, 5, "pages/review.md".into())
        .await
        .unwrap();

    assert_eq!(card.repetitions, 1);
    assert_eq!(card.interval_days, 1);
    assert!(!card.next_review.is_empty());
}
