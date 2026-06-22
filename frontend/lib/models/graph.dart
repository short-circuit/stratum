
import 'package:flutter/material.dart';

class GraphNode {
  final String id;
  final String label;
  final String slug;
  final List<String> tags;
  final int linkCount;

  GraphNode({required this.id, required this.label, this.slug = '', this.tags = const [], this.linkCount = 0});
}

class GraphEdge {
  final String source;
  final String target;
  final String? label;

  GraphEdge({required this.source, required this.target, this.label});
}

class GraphLayout {
  final List<GraphNode> nodes;
  final List<GraphEdge> edges;

  GraphLayout({required this.nodes, required this.edges});

  List<GraphNode> get orphanedNodes => nodes.where((n) => edges.every((e) => e.source != n.id && e.target != n.id)).toList();

  List<GraphNode> connectedTo(String nodeId) {
    final connected = <String>{};
    for (final e in edges) {
      if (e.source == nodeId) connected.add(e.target);
      if (e.target == nodeId) connected.add(e.source);
    }
    return nodes.where((n) => connected.contains(n.id)).toList();
  }
}

class ForceNode {
  final GraphNode node;
  Offset position;
  Offset velocity;

  ForceNode(this.node, this.position) : velocity = Offset.zero;
}
