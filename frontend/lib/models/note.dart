
class Note {
  final String path;
  final String title;
  final Frontmatter frontmatter;
  final String body;
  final List<Link> links;
  final List<Tag> tags;
  final List<Backlink> backlinks;
  final List<String> unlinkedMentions;
  final bool isFavorite;
  final DateTime modifiedAt;

  Note({
    required this.path,
    required this.title,
    required this.frontmatter,
    required this.body,
    this.links = const [],
    this.tags = const [],
    this.backlinks = const [],
    this.unlinkedMentions = const [],
    this.isFavorite = false,
    DateTime? modifiedAt,
  }) : modifiedAt = modifiedAt ?? DateTime.now();

  String get slug => path.split('/').last.replaceAll('.md', '');

  bool get hasFrontmatter => frontmatter.title != null || frontmatter.tags.isNotEmpty;
}

class Frontmatter {
  final String? title;
  final String? created;
  final String? modified;
  final List<String> tags;
  final List<String> aliases;

  Frontmatter({
    this.title,
    this.created,
    this.modified,
    this.tags = const [],
    this.aliases = const [],
  });

  Map<String, dynamic> toJson() => {
    'title': title,
    'created': created,
    'modified': modified,
    'tags': tags,
    'aliases': aliases,
  };

  factory Frontmatter.fromJson(Map<String, dynamic> json) => Frontmatter(
    title: json['title'] as String?,
    created: json['created'] as String?,
    modified: json['modified'] as String?,
    tags: List<String>.from(json['tags'] ?? []),
    aliases: List<String>.from(json['aliases'] ?? []),
  );
}

class Link {
  final String target;
  final String? displayText;
  final bool resolved;
  final int line;

  Link({required this.target, this.displayText, this.resolved = false, this.line = 0});
}

class Tag {
  final String name;
  final String source; // "frontmatter" or "inline"

  Tag({required this.name, this.source = 'inline'});
}

class Backlink {
  final String sourcePath;
  final String sourceTitle;
  final String contextSnippet;
  final int line;

  Backlink({required this.sourcePath, required this.sourceTitle, this.contextSnippet = '', this.line = 0});
}
