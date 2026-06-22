import 'dart:math';
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:provider/provider.dart';
import '../models/graph.dart';
import '../providers/vault_provider.dart';
import '../widgets/tag_chip.dart';

class GraphScreen extends StatefulWidget {
  const GraphScreen({super.key});

  @override
  State<GraphScreen> createState() => _GraphScreenState();
}

class _GraphScreenState extends State<GraphScreen> with SingleTickerProviderStateMixin {
  String? _selectedNode;
  String? _filterTag;
  late Ticker _ticker;
  List<ForceLayoutNode> _layoutNodes = [];
  bool _ticking = false;
  int _graphNodeCount = 0;
  final TransformationController _transformCtrl = TransformationController();
  bool _loaded = false;
  double _temperature = 1.0;
  int _iteration = 0;

  @override
  void initState() {
    super.initState();
    _ticker = createTicker(_onTick);
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadGraph());
  }

  @override
  void dispose() {
    _ticker.dispose();
    _transformCtrl.dispose();
    super.dispose();
  }

  void _onTick(Duration elapsed) {
    if (!_ticking) return;
    _simulateForces();
  }

  Future<void> _loadGraph() async {
    final vault = context.read<VaultProvider>();
    await vault.loadGraph();
    if (vault.graph != null && vault.graph!.nodes.isNotEmpty) {
      _initLayout(vault.graph!);
      _ticking = true;
      _graphNodeCount = vault.graph!.nodes.length;
      _temperature = 10.0;
      _iteration = 0;
      _ticker.start();
      _loaded = true;
      setState(() {});
    }
  }

  void _reloadIfNeeded() {
    final vault = context.read<VaultProvider>();
    final graph = vault.graph;
    if (graph != null && graph.nodes.length != _graphNodeCount) {
      _initLayout(graph);
      _ticking = true;
      _graphNodeCount = graph.nodes.length;
      _temperature = 10.0;
      _iteration = 0;
      if (!_ticker.isActive) _ticker.start();
    }
  }

  void _initLayout(GraphLayout graph) {
    final rng = Random();
    const spread = 150.0;
    _layoutNodes = graph.nodes.map((n) {
      final angle = rng.nextDouble() * 2 * pi;
      final radius = rng.nextDouble() * spread;
      return ForceLayoutNode(
        node: n,
        pos: Offset(cos(angle) * radius + 1000, sin(angle) * radius + 1000),
        velocity: Offset.zero,
      );
    }).toList();
  }

  void _simulateForces() {
    final n = _layoutNodes.length;
    if (n == 0) return;

    final area = 2000.0 * 2000.0;
    final k = sqrt(area / n) * 0.5;

    double maxDisp = 0;
    final disp = List.generate(n, (_) => Offset.zero);

    // Repulsion (Coulomb): f = k^2 / d
    for (int i = 0; i < n; i++) {
      for (int j = i + 1; j < n; j++) {
        final diff = _layoutNodes[i].pos - _layoutNodes[j].pos;
        var dist = diff.distance;
        if (dist < 1.0) dist = 1.0;
        final repForce = (k * k) / dist;
        disp[i] += diff / dist * repForce;
        disp[j] -= diff / dist * repForce;
      }
    }

    // Attraction (springs along edges): f = d^2 / k
    final vault = context.read<VaultProvider>();
    if (vault.graph != null) {
      for (final edge in vault.graph!.edges) {
        final srcIdx = _layoutNodes.indexWhere((ln) => ln.node.id == edge.source);
        final dstIdx = _layoutNodes.indexWhere((ln) => ln.node.id == edge.target);
        if (srcIdx == -1 || dstIdx == -1) continue;
        final diff = _layoutNodes[dstIdx].pos - _layoutNodes[srcIdx].pos;
        final dist = max(1.0, diff.distance);
        final attrForce = (dist * dist) / k;
        disp[srcIdx] += diff / dist * attrForce;
        disp[dstIdx] -= diff / dist * attrForce;
      }
    }

    // Gentle center gravity
    const center = Offset(1000, 1000);
    const centerStrength = 0.0005;
    for (int i = 0; i < n; i++) {
      final toCenter = center - _layoutNodes[i].pos;
      disp[i] += toCenter * centerStrength;
    }

    // Apply temperature-limited displacement
    for (int i = 0; i < n; i++) {
      final d = max(0.001, disp[i].distance);
      final limited = min(d, _temperature * 20);
      _layoutNodes[i].pos += disp[i] / d * limited;
      maxDisp = max(maxDisp, limited);
    }

    // Cool
    _temperature *= 0.95;
    _iteration++;

    if (_temperature < 0.1 || _iteration > 300) {
      _ticking = false;
      _ticker.stop();
      _autoCenter();
      return;
    }

    setState(() {});
  }

  void _autoCenter() {
    if (_layoutNodes.isEmpty) return;
    double minX = double.infinity, minY = double.infinity;
    double maxX = double.negativeInfinity, maxY = double.negativeInfinity;
    for (final ln in _layoutNodes) {
      if (ln.pos.dx < minX) minX = ln.pos.dx;
      if (ln.pos.dy < minY) minY = ln.pos.dy;
      if (ln.pos.dx > maxX) maxX = ln.pos.dx;
      if (ln.pos.dy > maxY) maxY = ln.pos.dy;
    }
    final ctr = Offset((minX + maxX) / 2, (minY + maxY) / 2);
    final canvasCenter = const Offset(1000, 1000);
    final offset = canvasCenter - ctr;
    _transformCtrl.value = Matrix4.identity()..setTranslationRaw(offset.dx, offset.dy, 0.0);
  }

  @override
  Widget build(BuildContext context) {
    final vault = context.watch<VaultProvider>();
    final graph = vault.graph;
    final isDark = Theme.of(context).brightness == Brightness.dark;

    _reloadIfNeeded();

    if (!_loaded || graph == null || graph.nodes.isEmpty) {
      return Center(child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.hub, size: 64, color: Colors.grey[300]),
          const SizedBox(height: 16),
          Text('No graph data', style: TextStyle(fontSize: 18, color: Colors.grey[500])),
          const SizedBox(height: 8),
          Text('Create notes with [[links]] to see connections', style: TextStyle(fontSize: 13, color: Colors.grey[400])),
        ],
      ));
    }

    return Column(children: [
      if (vault.tagCloud.isNotEmpty)
        Container(
          height: 40,
          padding: const EdgeInsets.symmetric(horizontal: 8),
          child: ListView(scrollDirection: Axis.horizontal, children: [
            ...vault.tagCloud.map((t) => Padding(
              padding: const EdgeInsets.only(left: 4),
              child: TagChip(
                label: t.name,
                count: t.count,
                selected: t.name == _filterTag,
                onTap: () => setState(() => _filterTag = _filterTag == t.name ? null : t.name),
              ),
            )),
          ]),
        ),
      Expanded(
        child: InteractiveViewer(
          transformationController: _transformCtrl,
          boundaryMargin: const EdgeInsets.all(2000),
          minScale: 0.1,
          maxScale: 4.0,
          child: GestureDetector(
            onTapDown: (details) {
              final pos = details.localPosition;
              String? found;
              for (final ln in _layoutNodes) {
                if ((ln.pos - pos).distance < 25) {
                  found = ln.node.id;
                  break;
                }
              }
              setState(() {
                _selectedNode = (found != null && found == _selectedNode) ? null : found;
              });
            },
            child: SizedBox(
              width: 2000,
              height: 2000,
              child: CustomPaint(
                painter: _GraphPainter(
                  graph: graph,
                  layoutNodes: _layoutNodes,
                  selectedNode: _selectedNode,
                  filterTag: _filterTag,
                  isDark: isDark,
                ),
              ),
            ),
          ),
        ),
      ),
      if (_selectedNode != null)
        _buildNodeInfo(context, graph, _selectedNode!),
    ]);
  }

  Widget _buildNodeInfo(BuildContext context, GraphLayout graph, String nodeId) {
    final node = graph.nodes.firstWhere((n) => n.id == nodeId);
    final connected = graph.connectedTo(nodeId);
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(color: isDark ? Colors.grey[850] : Colors.grey[100], border: Border(top: BorderSide(color: Theme.of(context).dividerColor))),
      child: Row(children: [
        Expanded(
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, mainAxisSize: MainAxisSize.min, children: [
            Text(node.label, style: const TextStyle(fontWeight: FontWeight.w600)),
            Text('${node.linkCount} links · ${connected.length} connections', style: TextStyle(fontSize: 12, color: Colors.grey[500])),
          ]),
        ),
        ...node.tags.map((t) => Padding(padding: const EdgeInsets.only(left: 4), child: TagChip(label: t))),
      ]),
    );
  }
}

class ForceLayoutNode {
  final GraphNode node;
  Offset pos;
  Offset velocity;

  ForceLayoutNode({required this.node, required this.pos, required this.velocity});
}

class _GraphPainter extends CustomPainter {
  final GraphLayout graph;
  final List<ForceLayoutNode> layoutNodes;
  final String? selectedNode;
  final String? filterTag;
  final bool isDark;

  _GraphPainter({
    required this.graph,
    required this.layoutNodes,
    this.selectedNode,
    this.filterTag,
    required this.isDark,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint();
    const nodeRadius = 8.0;
    const selectedRadius = 16.0;

    // Edges
    for (final edge in graph.edges) {
      final src = layoutNodes.where((n) => n.node.id == edge.source).firstOrNull;
      final dst = layoutNodes.where((n) => n.node.id == edge.target).firstOrNull;
      if (src == null || dst == null) continue;

      final isSelectedEdge = selectedNode != null && (edge.source == selectedNode || edge.target == selectedNode);
      paint
        ..color = isSelectedEdge
            ? (isDark ? Colors.blue[300]! : Colors.blue[400]!)
            : (isDark ? const Color(0xFF444444) : const Color(0xFFBBBBBB))
        ..strokeWidth = isSelectedEdge ? 2.0 : 1.0;
      canvas.drawLine(src.pos, dst.pos, paint);
    }

    // Nodes
    for (final ln in layoutNodes) {
      final isSelected = ln.node.id == selectedNode;
      final isFiltered = filterTag != null && !ln.node.tags.contains(filterTag);
      if (isFiltered) continue;

      final radius = isSelected ? selectedRadius : nodeRadius;

      // Fill
      paint
        ..color = isSelected
            ? (isDark ? Colors.blue[400]! : Colors.blue[500]!)
            : (isDark ? const Color(0xFF607D8B) : const Color(0xFF78909C))
        ..style = PaintingStyle.fill;
      canvas.drawCircle(ln.pos, radius, paint);

      // Border
      if (isSelected) {
        paint
          ..color = isDark ? Colors.blue[200]! : Colors.blue[600]!
          ..style = PaintingStyle.stroke
          ..strokeWidth = 2;
        canvas.drawCircle(ln.pos, radius + 2, paint);
      }

      // Label
      final labelStyle = TextStyle(
        color: isDark ? Colors.white70 : Colors.black87,
        fontSize: isSelected ? 12 : 10,
        fontWeight: isSelected ? FontWeight.w700 : FontWeight.w500,
      );
      final tp = TextPainter(
        text: TextSpan(text: ln.node.label, style: labelStyle),
        textDirection: TextDirection.ltr,
      );
      tp.layout(maxWidth: 120);
      tp.paint(canvas, Offset(ln.pos.dx - tp.width / 2, ln.pos.dy + radius + 2));
    }
  }

  @override
  bool shouldRepaint(covariant _GraphPainter old) => true;
}
