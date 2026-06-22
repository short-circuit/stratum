import 'package:flutter_test/flutter_test.dart';
import 'package:stratum/main.dart';

void main() {
  testWidgets('App launches', (WidgetTester tester) async {
    await tester.pumpWidget(const StratumApp());
    expect(find.text('Stratum'), findsOneWidget);
  });
}
