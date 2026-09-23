# Flashcards

Stratum includes a spaced repetition (SRS) flashcard system for active recall learning based on the SM-2 algorithm.

<!-- SCREENSHOT: [flashcards-review] Flashcard review showing front of card -->

## How Flashcards Work

Create blocks with `question::` and `answer::` properties in your notes. Stratum automatically generates flashcards from these blocks. The **block content is the question** and `question:: true` is a boolean marker that marks the block as a flashcard; the `answer::` property holds the answer.

```markdown
- What is a monad in functional programming?
  .question:: true
  .answer:: A monad is a design pattern that allows chaining operations while handling side effects.

- What is the difference between `var` and `let` in Rust?
  .question:: true
  .answer:: `var` doesn't exist in Rust. Variables are immutable by default (`let`); use `let mut` for mutability.
```

Only blocks that have both a `question::` and an `answer::` property produce a card.

## Creating Flashcards

Flashcards are created the same way you take any other note:

1. Add a new block in the editor and type the question as the block content.
2. Add a `question:: true` property to the block to mark it as a flashcard.
3. Add an `answer::` property with the answer text.
4. The card appears in the Flashcards panel automatically — no extra step is needed.

You can also combine flashcards with the `#flashcard` tag for easier searching.

## Review Session

1. Open the Flashcards panel.
2. Cards are loaded for review — cards whose schedule is due are presented first.
3. **Look at the front** — try to recall the answer.
4. Click the card to **show the back**.
5. Rate your recall using the on-screen buttons:

| Rating | Meaning | Effect on schedule |
|--------|---------|--------------------|
| Blackout (0) | Forgotten | Resets the card — interval set to 1 day, repetitions cleared |
| Hard (2) | Recalled with difficulty | Counts as an incorrect response — reset, then rescheduled to 1 day |
| Good (3) | Recalled with effort | Successful recall — interval grows |
| Easy (5) | Effortless recall | Strongest ease-factor adjustment and interval growth |

<!-- SCREENSHOT: [flashcards-answer] Flashcard showing answer with rating buttons -->

## Spaced Repetition Algorithm

Stratum uses a modified SM-2 algorithm:

- **Ease factor** — adjusts per card based on your recall difficulty; it is never lowered below 1.3
- **Interval** — grows after successful recalls (`1 day`, then `6 days`, then multiplied by the ease factor)
- **Next review** — calculated automatically after each rating and written back to the card's properties

## Schedule Persistence

Card schedules are **not lost when you restart Stratum**. After each rating, the updated SM-2 schedule (`ease`, `interval`, `reps`, `next_review`) is written back to the block's properties and saved to the note's `.md` file on disk, then the search index is refreshed. Because schedules live in your plain-text notes, they survive app restarts, syncs, and vault re-indexing.

## Card Properties

| Property | Description | Example |
|----------|-------------|---------|
| `question:: true` | Marks a block as a flashcard question | |
| `answer:: <text>` | The answer text, stored as a property on the same block | |

## Session Complete

When all loaded cards have been reviewed, the session shows a summary with a button to start again.

<!-- SCREENSHOT: [flashcards-done] Session complete screen with stats -->

## Tips

- Create flashcards as you take notes — "write once, review forever"
- Use the `#flashcard` tag alongside `question::` for easy searching
- Review daily — short sessions are more effective than cramming
- Rewrite cards you consistently fail — the problem might be the card, not your memory
