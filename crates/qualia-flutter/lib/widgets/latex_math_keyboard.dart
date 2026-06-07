import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Virtual LaTeX / math keyboard for precise engine and QPU prompts.
class LatexMathKeyboard extends StatelessWidget {
  final void Function(String snippet) onInsert;
  final VoidCallback? onClose;

  const LatexMathKeyboard({
    super.key,
    required this.onInsert,
    this.onClose,
  });

  static const _greek = [
    (r'\alpha', 'α'),
    (r'\beta', 'β'),
    (r'\gamma', 'γ'),
    (r'\delta', 'δ'),
    (r'\epsilon', 'ε'),
    (r'\theta', 'θ'),
    (r'\lambda', 'λ'),
    (r'\mu', 'μ'),
    (r'\pi', 'π'),
    (r'\sigma', 'σ'),
    (r'\phi', 'φ'),
    (r'\psi', 'ψ'),
    (r'\omega', 'ω'),
  ];

  static const _operators = [
    (r'\sum', '∑'),
    (r'\int', '∫'),
    (r'\partial', '∂'),
    (r'\nabla', '∇'),
    (r'\infty', '∞'),
    (r'\pm', '±'),
    (r'\times', '×'),
    (r'\leq', '≤'),
    (r'\geq', '≥'),
    (r'\neq', '≠'),
    (r'\approx', '≈'),
    (r'\sqrt{}', '√'),
  ];

  static final _structures = [
    (r'\frac{}{}', 'a/b'),
    (r'^{}', 'xⁿ'),
    (r'_{}', 'xₙ'),
    (r'\left( \right)', '( )'),
    (r'\left[ \right]', '[ ]'),
    (r'\hat{}', 'Ĥ'),
    (r'\vec{}', 'v⃗'),
    ('\$\$  \$\$', r'$$'),
  ];

  static const _quantum = [
    (r'\hat{H}', 'Ĥ'),
    (r'\Psi', 'Ψ'),
    (r'\ket{\psi}', '|ψ⟩'),
    (r'\bra{\psi}', '⟨ψ|'),
    (r'E = mc^2', 'E=mc²'),
    (r'\min_{x}', 'min'),
  ];

  static const _engine = [
    ('[qpu:qubo]', 'QUBO'),
    ('[qpu:dft]', 'DFT'),
    ('[qpu:defeasible]', 'Def'),
    ('[enable_QPU]', 'Enable'),
  ];

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Material(
      elevation: 8,
      color: cs.surfaceContainerHighest,
      child: DefaultTabController(
        length: 5,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            SizedBox(
              height: 40,
              child: Row(
                children: [
                  const Expanded(
                    child: TabBar(
                      isScrollable: true,
                      tabAlignment: TabAlignment.start,
                      tabs: [
                        Tab(text: 'αβγ'),
                        Tab(text: 'Ops'),
                        Tab(text: 'Struct'),
                        Tab(text: 'QPU'),
                        Tab(text: 'Engine'),
                      ],
                    ),
                  ),
                  if (onClose != null)
                    IconButton(
                      icon: const Icon(Icons.keyboard_hide, size: 20),
                      tooltip: 'Hide math keyboard',
                      onPressed: onClose,
                    ),
                ],
              ),
            ),
            SizedBox(
              height: 132,
              child: TabBarView(
                children: [
                  _keyGrid(_greek, cs),
                  _keyGrid(_operators, cs),
                  _keyGrid(_structures, cs),
                  _keyGrid(_quantum, cs),
                  _keyGrid(_engine, cs),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _keyGrid(List<(String, String)> keys, ColorScheme cs) {
    return Padding(
      padding: const EdgeInsets.all(6),
      child: GridView.builder(
        gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
          crossAxisCount: 6,
          mainAxisSpacing: 4,
          crossAxisSpacing: 4,
          childAspectRatio: 1.6,
        ),
        itemCount: keys.length,
        itemBuilder: (context, i) {
          final (snippet, label) = keys[i];
          return Material(
            color: cs.surface,
            borderRadius: BorderRadius.circular(6),
            child: InkWell(
              borderRadius: BorderRadius.circular(6),
              onTap: () {
                HapticFeedback.selectionClick();
                onInsert(snippet);
              },
              child: Center(
                child: Text(
                  label,
                  style: TextStyle(fontSize: 13, color: cs.onSurface),
                  textAlign: TextAlign.center,
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}

/// Insert [snippet] at the current cursor in [controller], respecting selection.
void insertAtCursor(TextEditingController controller, String snippet) {
  final text = controller.text;
  final sel = controller.selection;
  final start = sel.start >= 0 ? sel.start : text.length;
  final end = sel.end >= 0 ? sel.end : text.length;
  final newText = text.replaceRange(start, end, snippet);
  controller.value = TextEditingValue(
    text: newText,
    selection: TextSelection.collapsed(offset: start + snippet.length),
  );
}
