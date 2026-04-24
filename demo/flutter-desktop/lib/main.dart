// RaTeX Flutter Desktop Demo — showcase native LaTeX rendering on macOS, Windows, Linux.
//
// Features:
//   - Sidebar with categorized preset formulas
//   - Click to render any formula in the main panel
//   - Adjustable font size
//   - Custom formula input
//   - Dark / light theme toggle
//   - Reactive window layout (sidebar collapses on narrow windows)

import 'package:flutter/material.dart';
import 'package:ratex_flutter/ratex_flutter.dart';

void main() {
  runApp(const RaTeXDesktopDemoApp());
}

// ──────────────────────────────────────────────────────────────
// Theme
// ──────────────────────────────────────────────────────────────

class AppTheme {
  static const _seedColor = Color(0xFF6750A4);

  static ThemeData light() {
    final cs = ColorScheme.fromSeed(
      seedColor: _seedColor,
      brightness: Brightness.light,
    );
    return _build(cs, Brightness.light);
  }

  static ThemeData dark() {
    final cs = ColorScheme.fromSeed(
      seedColor: _seedColor,
      brightness: Brightness.dark,
    );
    return _build(cs, Brightness.dark);
  }

  static ThemeData _build(ColorScheme cs, Brightness b) => ThemeData(
        useMaterial3: true,
        colorScheme: cs,
        brightness: b,
        cardTheme: CardThemeData(
          elevation: 0,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
            side: BorderSide(
              color: cs.outlineVariant.withValues(alpha: 0.5),
            ),
          ),
        ),
        inputDecorationTheme: InputDecorationTheme(
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(8),
          ),
          contentPadding: const EdgeInsets.symmetric(
            horizontal: 16,
            vertical: 12,
          ),
        ),
      );
}

// ──────────────────────────────────────────────────────────────
// Formula data model
// ──────────────────────────────────────────────────────────────

class FormulaGroup {
  final String label;
  final List<FormulaEntry> entries;
  const FormulaGroup({required this.label, required this.entries});
}

class FormulaEntry {
  final String name;
  final String latex;
  const FormulaEntry({required this.name, required this.latex});
}

const _formulaGroups = [
  FormulaGroup(label: 'Classics', entries: [
    FormulaEntry(
      name: 'Quadratic Formula',
      latex: r'x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}',
    ),
    FormulaEntry(
      name: "Euler's Identity",
      latex: r'e^{i\pi} + 1 = 0',
    ),
    FormulaEntry(
      name: 'Gaussian Integral',
      latex: r'\int_{-\infty}^{\infty} e^{-x^2} \, dx = \sqrt{\pi}',
    ),
    FormulaEntry(
      name: 'Basel Problem',
      latex: r'\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}',
    ),
  ]),
  FormulaGroup(label: 'Calculus', entries: [
    FormulaEntry(
      name: 'Fundamental Theorem',
      latex: r'\int_a^b f(x)\,dx = F(b) - F(a)',
    ),
    FormulaEntry(
      name: "Stokes' Theorem",
      latex: r'\int_{\partial\Omega} \omega = \int_{\Omega} d\omega',
    ),
    FormulaEntry(
      name: 'Fourier Transform',
      latex: r'\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)\, e^{-2\pi i x \xi}\, dx',
    ),
    FormulaEntry(
      name: 'Residue Theorem',
      latex: r'\oint_\gamma f(z)\,dz = 2\pi i \sum_{k} \operatorname{Res}(f, a_k)',
    ),
  ]),
  FormulaGroup(label: 'Linear Algebra', entries: [
    FormulaEntry(
      name: 'Determinant (3x3)',
      latex: r'\det\begin{pmatrix} a & b & c \\ d & e & f \\ g & h & i \end{pmatrix} = aei + bfg + cdh - ceg - bdi - afh',
    ),
    FormulaEntry(
      name: 'Matrix Multiplication',
      latex: r'(AB)_{ij} = \sum_{k=1}^{n} A_{ik} B_{kj}',
    ),
    FormulaEntry(
      name: 'Eigenvalue Equation',
      latex: r'A\mathbf{v} = \lambda \mathbf{v}',
    ),
  ]),
  FormulaGroup(label: 'Physics', entries: [
    FormulaEntry(
      name: "Maxwell's Equations",
      latex: r'\nabla \cdot \mathbf{E} = \frac{\rho}{\varepsilon_0} \\ \nabla \cdot \mathbf{B} = 0 \\ \nabla \times \mathbf{E} = -\frac{\partial\mathbf{B}}{\partial t} \\ \nabla \times \mathbf{B} = \mu_0\mathbf{J} + \mu_0\varepsilon_0\frac{\partial\mathbf{E}}{\partial t}',
    ),
    FormulaEntry(
      name: 'Schrodinger Equation',
      latex: r'i\hbar\frac{\partial}{\partial t}\Psi(\mathbf{r},t) = \hat{H}\Psi(\mathbf{r},t)',
    ),
    FormulaEntry(
      name: 'Mass-Energy Equivalence',
      latex: r'E = mc^2',
    ),
  ]),
  FormulaGroup(label: 'Chemistry', entries: [
    FormulaEntry(
      name: 'Water Formation',
      latex: r'\ce{2H2 + O2 -> 2H2O}',
    ),
    FormulaEntry(
      name: 'Combustion',
      latex: r'\ce{CH4 + 2O2 -> CO2 + 2H2O}',
    ),
    FormulaEntry(
      name: 'Haber Process',
      latex: r'\ce{N2 + 3H2 <=> 2NH3}',
    ),
    FormulaEntry(
      name: 'Physical Units',
      latex: r'\pu{6.022e23 mol-1}',
    ),
  ]),
];

// ──────────────────────────────────────────────────────────────
// App
// ──────────────────────────────────────────────────────────────

class RaTeXDesktopDemoApp extends StatelessWidget {
  const RaTeXDesktopDemoApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'RaTeX Desktop Demo',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.light(),
      darkTheme: AppTheme.dark(),
      themeMode: ThemeMode.system,
      home: const DesktopDemoPage(),
    );
  }
}

// ──────────────────────────────────────────────────────────────
// Desktop Demo Page
// ──────────────────────────────────────────────────────────────

class DesktopDemoPage extends StatefulWidget {
  const DesktopDemoPage({super.key});

  @override
  State<DesktopDemoPage> createState() => _DesktopDemoPageState();
}

class _DesktopDemoPageState extends State<DesktopDemoPage> {
  String _currentLatex = r'\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}';
  String _currentName = 'Quadratic Formula';
  double _fontSize = 32;
  final _customController = TextEditingController();

  var _displayMode = true;

  final _expandedGroups = <int>{0};

  @override
  void dispose() {
    _customController.dispose();
    super.dispose();
  }

  void _selectFormula(FormulaEntry entry) {
    setState(() {
      _currentLatex = entry.latex;
      _currentName = entry.name;
    });
  }

  void _renderCustom() {
    final text = _customController.text.trim();
    if (text.isEmpty) return;
    setState(() {
      _currentLatex = text;
      _currentName = 'Custom';
    });
  }

  // ── Build ─────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    final isWide = MediaQuery.of(context).size.width > 800;

    return Scaffold(
      body: Row(
        children: [
          if (isWide) _buildSidebar(context),
          Expanded(
            child: Column(
              children: [
                _buildToolbar(context, isWide: isWide),
                const Divider(height: 1),
                Expanded(child: _buildRenderArea(context)),
              ],
            ),
          ),
        ],
      ),
    );
  }

  // ── Sidebar ───────────────────────────────────────────────

  Widget _buildSidebar(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return Container(
      width: 280,
      decoration: BoxDecoration(
        color: cs.surfaceContainerLow,
        border: Border(right: BorderSide(color: cs.outlineVariant)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 20, 16, 8),
            child: Row(
              children: [
                Icon(Icons.functions, color: cs.primary, size: 20),
                const SizedBox(width: 8),
                Text(
                  'Formulas',
                  style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        color: cs.onSurfaceVariant,
                      ),
                ),
              ],
            ),
          ),
          const Divider(height: 1),
          Expanded(
            child: ListView(
              padding: const EdgeInsets.symmetric(vertical: 8),
              children: [
                for (var i = 0; i < _formulaGroups.length; i++)
                  _buildGroup(i, _formulaGroups[i], cs),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildGroup(int index, FormulaGroup group, ColorScheme cs) {
    final isExpanded = _expandedGroups.contains(index);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        InkWell(
          onTap: () {
            setState(() {
              if (isExpanded) {
                _expandedGroups.remove(index);
              } else {
                _expandedGroups.add(index);
              }
            });
          },
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Row(
              children: [
                Icon(
                  isExpanded ? Icons.expand_less : Icons.expand_more,
                  size: 16,
                  color: cs.onSurfaceVariant,
                ),
                const SizedBox(width: 4),
                Text(
                  group.label,
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w600,
                    color: cs.onSurfaceVariant,
                    letterSpacing: 0.5,
                  ),
                ),
              ],
            ),
          ),
        ),
        if (isExpanded)
          ...group.entries.map(
            (entry) => _buildEntry(entry, cs),
          ),
      ],
    );
  }

  Widget _buildEntry(FormulaEntry entry, ColorScheme cs) {
    final isSelected = entry.name == _currentName;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      child: InkWell(
        onTap: () => _selectFormula(entry),
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          decoration: BoxDecoration(
            color: isSelected ? cs.secondaryContainer : null,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text(
            entry.name,
            style: TextStyle(
              fontSize: 13,
              fontWeight: isSelected ? FontWeight.w600 : FontWeight.w400,
              color: isSelected ? cs.onSecondaryContainer : cs.onSurface,
            ),
          ),
        ),
      ),
    );
  }

  // ── Toolbar ───────────────────────────────────────────────

  Widget _buildToolbar(BuildContext context, {required bool isWide}) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              if (!isWide)
                Builder(
                  builder: (ctx) => IconButton(
                    icon: const Icon(Icons.menu),
                    onPressed: () => Scaffold.of(ctx).openDrawer(),
                    visualDensity: VisualDensity.compact,
                  ),
                ),
              Expanded(
                child: Text(
                  _currentName,
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 8),
              IconButton(
                icon: const Icon(Icons.text_decrease, size: 20),
                onPressed: _fontSize > 8
                    ? () => setState(() => _fontSize -= 2)
                    : null,
                visualDensity: VisualDensity.compact,
                tooltip: 'Decrease size',
              ),
              SizedBox(
                width: 40,
                child: Text(
                  '${_fontSize.round()}',
                  textAlign: TextAlign.center,
                  style: const TextStyle(fontSize: 13),
                ),
              ),
              IconButton(
                icon: const Icon(Icons.text_increase, size: 20),
                onPressed: _fontSize < 96
                    ? () => setState(() => _fontSize += 2)
                    : null,
                visualDensity: VisualDensity.compact,
                tooltip: 'Increase size',
              ),
              const SizedBox(width: 16),
              SegmentedButton<bool>(
                segments: const [
                  ButtonSegment(value: true, label: Text('Display')),
                  ButtonSegment(value: false, label: Text('Inline')),
                ],
                selected: {_displayMode},
                onSelectionChanged: (v) =>
                    setState(() => _displayMode = v.first),
                style: ButtonStyle(
                  visualDensity: VisualDensity.compact,
                  textStyle: WidgetStateProperty.all(
                    const TextStyle(fontSize: 12),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _customController,
                  decoration: const InputDecoration(
                    hintText: 'Enter custom LaTeX, e.g. \\frac{a}{b}',
                    isDense: true,
                  ),
                  style:
                      const TextStyle(fontFamily: 'monospace', fontSize: 13),
                  onSubmitted: (_) => _renderCustom(),
                ),
              ),
              const SizedBox(width: 8),
              FilledButton.tonalIcon(
                onPressed: _renderCustom,
                icon: const Icon(Icons.play_arrow, size: 18),
                label: const Text('Render'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  // ── Render area ───────────────────────────────────────────

  Widget _buildRenderArea(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return LayoutBuilder(
      builder: (context, constraints) {
        return Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            scrollDirection: Axis.vertical,
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Container(
                constraints: BoxConstraints(
                  minWidth: constraints.maxWidth - 48,
                  minHeight: constraints.maxHeight - 48,
                ),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 32,
                          vertical: 36,
                        ),
                        child: RaTeXWidget(
                          latex: _currentLatex,
                          fontSize: _fontSize,
                          displayMode: _displayMode,
                          color: cs.onSurface,
                          onError: (e) => _buildErrorWidget(cs, e),
                          loading: const Padding(
                            padding: EdgeInsets.all(24),
                            child: CircularProgressIndicator(strokeWidth: 2),
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(height: 16),
                    Container(
                      constraints: const BoxConstraints(maxWidth: 600),
                      child: Card(
                        color: cs.surfaceContainerHighest,
                        child: Padding(
                          padding: const EdgeInsets.all(16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                'LaTeX Source',
                                style: TextStyle(
                                  fontSize: 11,
                                  fontWeight: FontWeight.w600,
                                  color: cs.onSurfaceVariant,
                                  letterSpacing: 0.5,
                                ),
                              ),
                              const SizedBox(height: 8),
                              SelectableText(
                                _currentLatex,
                                style: TextStyle(
                                  fontFamily: 'monospace',
                                  fontSize: 13,
                                  color: cs.onSurface,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _buildErrorWidget(ColorScheme cs, RaTeXException e) {
    return Card(
      color: cs.errorContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.error_outline, color: cs.onErrorContainer, size: 20),
            const SizedBox(width: 8),
            Flexible(
              child: Text(
                e.message,
                style: TextStyle(color: cs.onErrorContainer, fontSize: 13),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
