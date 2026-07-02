# Math Equations

Stratum supports LaTeX math equations rendered with KaTeX — both inline and display mode.

<!-- SCREENSHOT: [math-editor] Math equation editor modal with preview -->

## Inline Math

Wrap LaTeX in `$...$` for inline equations:

```markdown
The quadratic formula is $x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$.
```

Renders as: The quadratic formula is $x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$.

## Display Math

Use `$$...$$` for centered display equations:

```markdown
$$
E = mc^2
$$
```

Or larger expressions:

```markdown
$$
\sum_{i=1}^{n} i = \frac{n(n+1)}{2}
$$
```

## Math Editor Modal

For complex equations, use the math editor:

1. Type inline math `$...$` or display math `$$...$$`
2. Click the math to open the editor
3. Edit the LaTeX in the text area
4. **Live preview** updates as you type
5. Press `Ctrl+Enter` to save or `Escape` to cancel

<!-- SCREENSHOT: [math-editor-modal] The modal equation editor with LaTeX input and rendered preview -->

## Symbol Palette

The math editor includes a **Symbol Palette** for common LaTeX symbols, organized by category:

| Category | Examples |
|----------|---------|
| Greek | `\alpha`, `\beta`, `\gamma`, `\pi` |
| Operators | `\sum`, `\prod`, `\int`, `\lim` |
| Relations | `\leq`, `\geq`, `\neq`, `\approx` |
| Arrows | `\to`, `\Rightarrow`, `\mapsto` |
| Brackets | `\{`, `\}`, `\langle`, `\rangle` |
| Calculus | `\partial`, `\nabla`, `\infty` |

Click a symbol to insert it at the cursor position in the LaTeX editor.

## Examples

### Linear algebra

```markdown
$$
A = \begin{pmatrix}
a_{11} & a_{12} \\
a_{21} & a_{22}
\end{pmatrix}
$$
```

### Calculus

```markdown
$$
\frac{d}{dx} \int_{a}^{x} f(t) dt = f(x)
$$
```

### Set theory

```markdown
$$
A \cap B = \{x : x \in A \wedge x \in B\}
$$
```

## Tips

- Use `$...$` for inline math within paragraphs
- Use `$$...$$` for standalone equations on their own line
- The symbol palette saves time — explore it when writing complex expressions
- Ctrl+Click on rendered math to edit directly
