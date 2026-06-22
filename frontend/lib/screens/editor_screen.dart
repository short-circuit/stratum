import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/note.dart';
import '../providers/vault_provider.dart';
import '../widgets/backlinks_panel.dart';
import '../widgets/status_bar.dart';

class EditorScreen extends StatefulWidget {
  const EditorScreen({super.key});

  @override
  State<EditorScreen> createState() => _EditorScreenState();
}

class _EditorScreenState extends State<EditorScreen> {
  int? _editingLine;
  final _ctrl = TextEditingController();
  final _focus = FocusNode();
  String? _lastPath;

  // Undo
  final List<String> _undoStack = [];
  int _undoIdx = -1;

  // Raw full-source editing
  bool _rawEdit = false;
  final _rawCtrl = TextEditingController();

  @override
  void dispose() {
    _ctrl.dispose();
    _focus.dispose();
    _rawCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final vault = context.watch<VaultProvider>();
    final note = vault.currentNote;

    if (note != null && _lastPath != null && _lastPath != note.path) {
      _editingLine = null;
      _rawEdit = false;
      _undoStack.clear();
      _undoIdx = -1;
    }
    _lastPath = note?.path;

    if (note == null) return _emptyState();

    final isDark = Theme.of(context).brightness == Brightness.dark;
    final showBacklinks = note.backlinks.isNotEmpty || note.unlinkedMentions.isNotEmpty;
    final lines = note.body.split('\n');

    return Column(children: [
      _toolbar(vault, note),
      Expanded(
        child: _rawEdit
            ? _rawEditor(vault, isDark)
            : lines.isEmpty
                ? _emptyBody(vault, isDark)
                : _linesView(vault, lines, isDark),
      ),
      if (showBacklinks) _backlinksDock(note, vault),
      StatusBar(
        syncStatus: 'Up to date',
        backlinkCount: note.backlinks.length,
        wordCount: note.body.isEmpty ? 0 : note.body.split(RegExp(r'\s+')).length,
      ),
    ]);
  }

  Widget _emptyState() => Center(
    child: Column(mainAxisAlignment: MainAxisAlignment.center, children: [
      Icon(Icons.auto_stories, size: 64, color: Colors.grey[300]),
      const SizedBox(height: 16),
      Text('Select a note to edit', style: TextStyle(fontSize: 18, color: Colors.grey[500])),
    ]),
  );

  Widget _emptyBody(VaultProvider vault, bool isDark) => GestureDetector(
    onTap: _newLine,
    child: Center(child: Column(mainAxisAlignment: MainAxisAlignment.center, children: [
      Icon(Icons.edit_note, size: 48, color: Colors.grey[400]),
      const SizedBox(height: 12),
      Text('Tap to start writing', style: TextStyle(fontSize: 16, color: Colors.grey[500])),
    ])),
  );

  // ── Toolbar ──────────────────────────────────────────────────────

  Widget _toolbar(VaultProvider vault, Note note) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        border: Border(bottom: BorderSide(color: Theme.of(context).dividerColor)),
      ),
      child: Row(children: [
        Expanded(child: Text(note.title, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15))),
        _tbBtn(Icons.undo, 'Undo', _undoIdx > 0 ? () => _undo(vault) : null, null),
        _tbBtn(Icons.redo, 'Redo', _undoIdx < _undoStack.length - 1 ? () => _redo(vault) : null, null),
        const SizedBox(width: 4),
        _tbBtn(Icons.edit, 'New line', _newLine, null),
        _tbBtn(
          _rawEdit ? Icons.view_agenda : Icons.edit_document,
          _rawEdit ? 'Preview' : 'Raw edit',
          () => _toggleRaw(vault),
          _rawEdit ? Theme.of(context).colorScheme.primary : null,
        ),
        _tbBtn(Icons.save, 'Save', () => vault.saveCurrentNote(), null),
      ]),
    );
  }

  Widget _tbBtn(IconData icon, String tt, VoidCallback? onTap, Color? c) {
    return SizedBox(
      width: 32, height: 32,
      child: IconButton(
        icon: Icon(icon, size: 16),
        onPressed: onTap,
        tooltip: tt,
        padding: EdgeInsets.zero,
        constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
        splashRadius: 14,
        color: onTap == null ? Colors.grey[600] : (c ?? Colors.grey[400]),
      ),
    );
  }

  // ── Line view ────────────────────────────────────────────────────

  Widget _linesView(VaultProvider vault, List<String> lines, bool isDark) {
    return ListView.builder(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 20),
      itemCount: lines.length + 1, // +1 for empty trailing line to add
      itemBuilder: (_, i) {
        if (i < lines.length) {
          return _editingLine == i
              ? _lineEditor(vault, i, lines, isDark)
              : _linePreview(vault, i, lines[i], isDark);
        }
        // Trailing empty spot to append a new line
        if (_editingLine == -1) return _lineEditor(vault, -1, lines, isDark);
        return GestureDetector(
          onTap: () {
            if (_editingLine != null) {
              final note = vault.currentNote;
              if (note != null) {
                _commitLine(_ctrl.text, _editingLine!, lines, vault);
              }
            }
            if (_editingLine == -1) return;
            _newLine();
          },
          behavior: HitTestBehavior.opaque,
          child: const SizedBox(height: 28),
        );
      },
    );
  }

  Widget _linePreview(VaultProvider vault, int idx, String line, bool isDark) {
    final base = TextStyle(fontSize: 14, height: 1.6, color: isDark ? Colors.grey[300]! : Colors.grey[800]!);
    return GestureDetector(
      onTap: () {
        if (_editingLine != null) {
          // Commit current edit before switching
          final note = vault.currentNote;
          if (note != null) {
            _commitLine(_ctrl.text, _editingLine!, note.body.split('\n'), vault);
          }
        }
        if (_editingLine == idx) return; // already editing this line
        setState(() {
          _editingLine = idx;
          _ctrl.text = line;
        });
      },
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 1),
        child: line.isEmpty
            ? const SizedBox(height: 14)
            : RichText(text: _parseInline(line, base)),
      ),
    );
  }

  Widget _lineEditor(VaultProvider vault, int idx, List<String> lines, bool isDark) {
    if (_ctrl.text.isEmpty) _ctrl.text = idx >= 0 ? lines[idx] : '';
    return TextField(
      controller: _ctrl,
      focusNode: _focus,
      autofocus: true,
      maxLines: 1,
      textInputAction: TextInputAction.done,
      style: TextStyle(fontFamily: 'monospace', fontSize: 14, color: isDark ? Colors.grey[200] : Colors.grey[800]),
      decoration: InputDecoration(
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(4),
          borderSide: BorderSide(color: Theme.of(context).colorScheme.primary.withValues(alpha: 0.3)),
        ),
        contentPadding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        isDense: true,
      ),
      onSubmitted: (val) {
        _commitLine(val, idx, lines, vault);
        _focus.unfocus();
      },
      onEditingComplete: () => _commitLine(_ctrl.text, idx, lines, vault),
    );
  }

  void _commitLine(String newText, int idx, List<String> lines, VaultProvider vault) {
    if (vault.currentNote == null) return;
    final newLines = List<String>.from(lines);
    if (idx == -1) {
      // Append
      if (newText.trim().isEmpty) { setState(() => _editingLine = null); return; }
      newLines.add(newText);
    } else {
      newLines[idx] = newText;
    }
    _applyBody(newLines.join('\n'), vault);
    setState(() => _editingLine = null);
  }

  void _newLine() {
    setState(() {
      _editingLine = -1;
      _ctrl.text = '';
    });
  }

  void _applyBody(String body, VaultProvider vault) {
    _pushUndo(body);
    vault.updateCurrentBody(body);
    vault.saveCurrentNote();
  }

  // ── Raw editor ───────────────────────────────────────────────────

  void _toggleRaw(VaultProvider vault) {
    if (_rawEdit) {
      final b = _rawCtrl.text;
      if (b != vault.currentNote?.body) _applyBody(b, vault);
    } else {
      _rawCtrl.text = vault.currentNote?.body ?? '';
    }
    setState(() {
      _rawEdit = !_rawEdit;
      _editingLine = null;
    });
  }

  Widget _rawEditor(VaultProvider vault, bool isDark) {
    if (_rawCtrl.text.isEmpty) _rawCtrl.text = vault.currentNote?.body ?? '';
    return TextField(
      controller: _rawCtrl,
      maxLines: null,
      expands: true,
      autofocus: true,
      style: TextStyle(fontFamily: 'monospace', fontSize: 14, height: 1.6, color: isDark ? Colors.grey[200] : Colors.grey[800]),
      decoration: const InputDecoration(border: InputBorder.none, contentPadding: EdgeInsets.all(20)),
    );
  }

  // ── Undo/redo ────────────────────────────────────────────────────

  void _pushUndo(String body) {
    if (_undoIdx < _undoStack.length - 1) _undoStack.removeRange(_undoIdx + 1, _undoStack.length);
    _undoStack.add(body);
    _undoIdx = _undoStack.length - 1;
  }

  void _undo(VaultProvider vault) {
    if (_undoIdx <= 0 || vault.currentNote == null) return;
    _undoIdx--;
    vault.updateCurrentBody(_undoStack[_undoIdx]);
    vault.saveCurrentNote();
    setState(() { _editingLine = null; _rawEdit = false; });
  }

  void _redo(VaultProvider vault) {
    if (_undoIdx >= _undoStack.length - 1 || vault.currentNote == null) return;
    _undoIdx++;
    vault.updateCurrentBody(_undoStack[_undoIdx]);
    vault.saveCurrentNote();
    setState(() { _editingLine = null; _rawEdit = false; });
  }

  // ── Backlinks ────────────────────────────────────────────────────

  Widget _backlinksDock(Note note, VaultProvider vault) {
    return SizedBox(
      height: 120,
      child: BacklinksPanel(
        backlinks: note.backlinks,
        unlinkedMentions: note.unlinkedMentions,
        onNavigate: vault.openNote,
      ),
    );
  }

  // ── Inline parser (line-level) ───────────────────────────────────

  TextSpan _parseInline(String text, TextStyle base) {
    return TextSpan(children: _inlineSpans(text, base));
  }

  List<InlineSpan> _inlineSpans(String text, TextStyle base) {
    final spans = <InlineSpan>[];
    final re = RegExp(
      r'(\*\*|__)(.+?)\1'                     // bold
      r'|(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)'  // italic *
      r'|(?<!_)_(?!_)(.+?)(?<!_)_(?!_)'        // italic _
      r'|~~(.+?)~~'                              // strikethrough
      r'|`(.+?)`'                                // code
      r'|\[\[([^\]]+)\]\]'                      // wiki link
      r'|\[([^\]]+)\]\(([^)]+)\)'              // md link [t](u)
      r'|#([a-zA-Z][\w-]*)'                    // tag
      r'|!\[([^\]]*)\]\(([^)]+)\)'             // image
      r'|- \[([ x])\] (.+)'                    // checkbox
    );

    int last = 0;
    for (final m in re.allMatches(text)) {
      if (m.start > last) spans.add(TextSpan(text: text.substring(last, m.start), style: base));

      if (m.group(1) != null) {
        // bold
        spans.add(TextSpan(text: m.group(2), style: base.copyWith(fontWeight: FontWeight.w700)));
      } else if (m.group(3) != null || m.group(4) != null) {
        // italic
        spans.add(TextSpan(text: m.group(3) ?? m.group(4), style: base.copyWith(fontStyle: FontStyle.italic)));
      } else if (m.group(5) != null) {
        // strikethrough
        spans.add(TextSpan(text: m.group(5), style: base.copyWith(decoration: TextDecoration.lineThrough)));
      } else if (m.group(6) != null) {
        // code
        spans.add(TextSpan(text: m.group(6), style: base.copyWith(fontFamily: 'monospace', fontSize: (base.fontSize ?? 14) * 0.92, backgroundColor: Colors.grey.withValues(alpha: 0.2))));
      } else if (m.group(7) != null) {
        // wiki link
        spans.add(TextSpan(text: m.group(7), style: base.copyWith(color: Colors.blue[400], decoration: TextDecoration.underline)));
      } else if (m.group(8) != null && m.group(9) != null) {
        // md link
        spans.add(TextSpan(text: m.group(8), style: base.copyWith(color: Colors.blue[400], decoration: TextDecoration.underline)));
      } else if (m.group(10) != null) {
        // tag
        spans.add(TextSpan(text: '#${m.group(10)}', style: base.copyWith(color: Colors.blue[300], fontWeight: FontWeight.w500)));
      } else if (m.group(11) != null && m.group(12) != null) {
        spans.add(TextSpan(text: '[img:${m.group(11)}]', style: base.copyWith(color: Colors.grey[500], fontStyle: FontStyle.italic)));
      } else if (m.group(13) != null && m.group(14) != null) {
        // checkbox
        final checked = m.group(13) == 'x';
        spans.add(WidgetSpan(child: Icon(
          checked ? Icons.check_box : Icons.check_box_outline_blank,
          size: 16,
          color: checked ? Colors.green[600] : Colors.grey[500],
        )));
        spans.add(WidgetSpan(child: const SizedBox(width: 4)));
        final deco = checked ? TextDecoration.lineThrough : null;
        spans.add(TextSpan(text: m.group(14), style: base.copyWith(decoration: deco, color: checked ? Colors.grey[500] : null)));
      }

      last = m.end;
    }

    if (last < text.length) spans.add(TextSpan(text: text.substring(last), style: base));
    if (spans.isEmpty) spans.add(TextSpan(text: text, style: base));
    return spans;
  }
}
