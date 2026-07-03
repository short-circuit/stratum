# Flashcards

Stratum includes a spaced repetition (SRS) flashcard system for active recall learning.

<!-- SCREENSHOT: [flashcards-review] Flashcard review showing front of card -->

## How Flashcards Work

Create blocks with `question::` and `answer::` properties in your notes. Stratum automatically generates flashcards from these blocks.

```markdown
- What is a monad in functional programming?
  .question:: true
  .answer:: A monad is a design pattern that allows chaining operations while handling side effects.

- What is the difference between `var` and `let` in Rust?
  .question:: true
  .answer:: `var` doesn't exist in Rust. Variables are immutable by default (`let`); use `let mut` for mutability.
```

## Opening Flashcards

Click **:material-cards: Flashcards** in the sidebar or navigate to `/flashcards`.

## Review Session

1. Open the Flashcards panel
2. Cards due for review are loaded automatically
3. **Look at the front** — try to recall the answer
4. Click the card to **show the back**
5. Rate your recall:

| Rating | Meaning | Interval |
|--------|---------|----------|
| 0–2 | Forgotten | Card resets to short interval |
| 3 | Recalled with effort | Moderate interval increase |
| 4–5 | Easy recall | Large interval increase |

<!-- SCREENSHOT: [flashcards-answer] Flashcard showing answer with rating buttons -->

## Spaced Repetition Algorithm

Stratum uses a modified SM-2 algorithm:

- **Ease factor** — adjusts per card based on recall difficulty
- **Interval** — grows exponentially with successful recalls
- **Next review** — calculated automatically after each rating

## Card Properties

| Property | Description | Example |
|----------|-------------|---------|
| `question:: true` | Marks a block as a flashcard question | |
| `answer:: <text>` | The answer (can be on a child block or in properties) | |

## Session Complete

When all due cards have been reviewed, the session shows a summary:

<!-- SCREENSHOT: [flashcards-done] Session complete screen with stats -->

## Tips

- Create flashcards as you take notes — "write once, review forever"
- Use the `#flashcard` tag alongside `question::` for easy searching
- Review daily — short sessions are more effective than cramming
- Rewrite cards you consistently fail — the problem might be the card, not your memory
